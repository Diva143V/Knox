//! Session lifecycle management and persistence for Knox.

use chrono::{DateTime, Utc};
use rusqlite::params;
use std::time::Duration;

use super::db::KnoxDb;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::{ProjectId, SessionId};
use crate::core::types::{Project, Session};

/// Resolves an existing project ID or creates a new project entry.
fn resolve_project_id(
    conn: &rusqlite::Connection,
    proj_identifier: &str,
    started_at: DateTime<Utc>,
) -> KnoxResult<String> {
    let clean_path = proj_identifier
        .strip_prefix(r"\\?\")
        .unwrap_or(proj_identifier)
        .trim_end_matches(['\\', '/']);
    let verbatim_path = format!(r"\\?\{clean_path}");

    let mut stmt = conn
        .prepare("SELECT id FROM projects WHERE id = ?1 OR name = ?1 OR path = ?1 OR path = ?2")?;
    let mut rows = stmt.query(params![clean_path, verbatim_path])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        let name = std::path::Path::new(clean_path)
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(clean_path)
            .to_string();

        let new_proj = Project {
            id: ProjectId::new(),
            name,
            path: clean_path.to_string(),
            created_at: started_at,
        };
        conn.execute(
            "INSERT INTO projects (id, name, path, created_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET name = excluded.name",
            params![
                new_proj.id.as_str(),
                new_proj.name.as_str(),
                new_proj.path.as_str(),
                new_proj.created_at.to_rfc3339()
            ],
        )?;
        Ok(new_proj.id.to_string())
    }
}

/// Starts a new work session in Knox, optionally associated with a project.
pub fn start_session(db: &KnoxDb, actor: &str, project: Option<&str>) -> KnoxResult<Session> {
    let conn = db.conn();
    let session_id = SessionId::new();
    let started_at = Utc::now();

    let resolved_project_id = if let Some(proj_identifier) = project {
        Some(resolve_project_id(conn, proj_identifier, started_at)?)
    } else {
        None
    };

    conn.execute(
        "INSERT INTO sessions (id, actor, project_id, started_at, ended_at, is_open)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            session_id.as_str(),
            actor,
            resolved_project_id.as_deref(),
            started_at.to_rfc3339(),
            None::<&str>,
            1
        ],
    )?;

    tracing::info!(session_id = %session_id, actor = %actor, "Started Knox session");

    Ok(Session {
        id: session_id,
        actor: actor.to_string(),
        project: project.map(|s| s.to_string()),
        started_at,
        ended_at: None,
        is_open: true,
    })
}

/// Concludes an active session, marking it closed and recording the end timestamp.
pub fn end_session(db: &KnoxDb, session_id: &SessionId) -> KnoxResult<Session> {
    let current = get_session(db, session_id)?
        .ok_or_else(|| KnoxError::SessionNotFound(session_id.to_string()))?;

    if !current.is_open {
        return Err(KnoxError::SessionClosed(session_id.to_string()));
    }

    let ended_at = Utc::now();
    db.conn().execute(
        "UPDATE sessions SET is_open = 0, ended_at = ?1 WHERE id = ?2",
        params![ended_at.to_rfc3339(), session_id.as_str()],
    )?;

    tracing::info!(session_id = %session_id, "Ended Knox session");

    Ok(Session {
        ended_at: Some(ended_at),
        is_open: false,
        ..current
    })
}

/// Computes the timestamp of the last activity within a session.
fn get_session_last_activity(
    conn: &rusqlite::Connection,
    s_id: &str,
    default_time: DateTime<Utc>,
) -> DateTime<Utc> {
    let act_stmt = conn
        .prepare_cached("SELECT MAX(timestamp) FROM actions WHERE session_id = ?1")
        .ok();
    if let Some(mut stmt) = act_stmt {
        let latest: Option<String> = stmt.query_row(params![s_id], |r| r.get(0)).unwrap_or(None);
        if let Some(s) = latest {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
                return dt.with_timezone(&Utc);
            }
        }
    }
    default_time
}

/// Identifies and closes open sessions that have had no activity for longer than `max_idle`.
pub fn timeout_stale_sessions(db: &KnoxDb, max_idle: Duration) -> KnoxResult<Vec<SessionId>> {
    let conn = db.conn();
    let mut stmt =
        conn.prepare("SELECT id, actor, project_id, started_at FROM sessions WHERE is_open = 1")?;

    let sessions = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let now = Utc::now();
    let max_idle_chrono =
        chrono::Duration::from_std(max_idle).unwrap_or_else(|_| chrono::Duration::minutes(30));
    let mut timed_out = Vec::new();

    for (s_id, _, _, started_str) in sessions {
        let started_at = DateTime::parse_from_rfc3339(&started_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(now);

        let last_activity = get_session_last_activity(conn, &s_id, started_at);

        if now - last_activity > max_idle_chrono {
            let session_id = SessionId::from(s_id);
            conn.execute(
                "UPDATE sessions SET is_open = 0, ended_at = ?1 WHERE id = ?2",
                params![now.to_rfc3339(), session_id.as_str()],
            )?;
            timed_out.push(session_id);
        }
    }

    Ok(timed_out)
}

/// Retrieves a session record by its identifier.
pub fn get_session(db: &KnoxDb, session_id: &SessionId) -> KnoxResult<Option<Session>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT s.id, s.actor, s.started_at, s.ended_at, s.is_open, p.name
         FROM sessions s
         LEFT JOIN projects p ON s.project_id = p.id
         WHERE s.id = ?1",
    )?;

    let mut rows = stmt.query(params![session_id.as_str()])?;
    if let Some(row) = rows.next()? {
        let id = SessionId::from(row.get::<_, String>(0)?);
        let actor: String = row.get(1)?;
        let started_str: String = row.get(2)?;
        let ended_str: Option<String> = row.get(3)?;
        let is_open_int: i32 = row.get(4)?;
        let project_name: Option<String> = row.get(5)?;

        let started_at = DateTime::parse_from_rfc3339(&started_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let ended_at = ended_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        Ok(Some(Session {
            id,
            actor,
            project: project_name,
            started_at,
            ended_at,
            is_open: is_open_int == 1,
        }))
    } else {
        Ok(None)
    }
}

/// Lists recent sessions ordered by start time descending.
pub fn list_recent_sessions(db: &KnoxDb, limit: usize) -> KnoxResult<Vec<Session>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT s.id, s.actor, s.started_at, s.ended_at, s.is_open, p.name
         FROM sessions s
         LEFT JOIN projects p ON s.project_id = p.id
         ORDER BY s.started_at DESC
         LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        let id = SessionId::from(row.get::<_, String>(0)?);
        let actor: String = row.get(1)?;
        let started_str: String = row.get(2)?;
        let ended_str: Option<String> = row.get(3)?;
        let is_open_int: i32 = row.get(4)?;
        let project_name: Option<String> = row.get(5)?;

        let started_at = DateTime::parse_from_rfc3339(&started_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let ended_at = ended_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        Ok(Session {
            id,
            actor,
            project: project_name,
            started_at,
            ended_at,
            is_open: is_open_int == 1,
        })
    })?;

    let mut sessions = Vec::new();
    for r in rows {
        sessions.push(r?);
    }
    Ok(sessions)
}

/// Counts the number of actions recorded under a given session identifier.
pub fn count_session_actions(db: &KnoxDb, session_id: &SessionId) -> KnoxResult<usize> {
    let conn = db.conn();
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM actions WHERE session_id = ?1")?;
    let count: i64 = stmt.query_row(params![session_id.as_str()], |r| r.get(0))?;
    Ok(count as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_and_end_session() {
        let db = KnoxDb::open_memory().unwrap();
        let session = start_session(&db, "test_agent", Some("knox")).unwrap();
        assert!(session.is_open);
        assert!(session.ended_at.is_none());

        let ended = end_session(&db, &session.id).unwrap();
        assert!(!ended.is_open);
        assert!(ended.ended_at.is_some());

        // Ending already ended session should return error
        let err = end_session(&db, &session.id).unwrap_err();
        assert!(matches!(err, KnoxError::SessionClosed(_)));
    }

    #[test]
    fn test_timeout_stale_sessions() {
        let db = KnoxDb::open_memory().unwrap();
        let session = start_session(&db, "idle_agent", None).unwrap();

        // 0 duration timeout should immediately close it
        let timed_out = timeout_stale_sessions(&db, Duration::from_secs(0)).unwrap();
        assert_eq!(timed_out, vec![session.id.clone()]);

        let fetched = get_session(&db, &session.id).unwrap().unwrap();
        assert!(!fetched.is_open);
    }

    #[test]
    fn test_list_recent_sessions() {
        let db = KnoxDb::open_memory().unwrap();
        let s1 = start_session(&db, "agent_1", Some("proj1")).unwrap();
        let s2 = start_session(&db, "agent_2", None).unwrap();

        let list = list_recent_sessions(&db, 10).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, s2.id); // most recent first
        assert_eq!(list[1].id, s1.id);
    }
}
