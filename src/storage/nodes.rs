//! Node CRUD operations for Knox actions and entities in SQLite.

use chrono::{DateTime, Utc};
use rusqlite::params;

use super::db::KnoxDb;
use crate::core::error::KnoxResult;
use crate::core::ids::{ActionId, ProjectId, SessionId};
use crate::core::types::{KnoxAction, Project};

pub use super::queries::query_actions;

/// Inserts a new `KnoxAction` into the database, populating files, tags,
/// and handling supersede relationships.
pub fn insert_action(db: &KnoxDb, action: &KnoxAction) -> KnoxResult<ActionId> {
    let conn = db.conn();

    conn.execute(
        "INSERT INTO actions (
            id, actor, action_type, summary, confidence, timestamp,
            session_id, parent_action_id, superseded_by, is_current
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            action.id.as_str(),
            action.actor.as_str(),
            action.action_type.as_str(),
            action.summary.as_str(),
            action.confidence,
            action.timestamp.to_rfc3339(),
            action.session_id.as_ref().map(|s| s.as_str()),
            action.parent_action_id.as_ref().map(|p| p.as_str()),
            None::<&str>,
            if action.is_current { 1 } else { 0 },
        ],
    )?;

    if let Some(ref old_id) = action.supersedes {
        mark_superseded(db, old_id, &action.id)?;
    }

    insert_action_files_and_tags(conn, &action.id, &action.files, &action.tags)?;

    tracing::debug!(
        id = %action.id,
        actor = %action.actor,
        action_type = %action.action_type,
        "Inserted action into knowledge graph"
    );

    Ok(action.id.clone())
}

/// Inserts associated files and tags for a given action.
fn insert_action_files_and_tags(
    conn: &rusqlite::Connection,
    id: &ActionId,
    files: &[String],
    tags: &[String],
) -> KnoxResult<()> {
    let mut stmt_files = conn.prepare_cached(
        "INSERT OR IGNORE INTO action_files (action_id, file_path) VALUES (?1, ?2)",
    )?;
    for file in files {
        stmt_files.execute(params![id.as_str(), file.as_str()])?;
    }

    let mut stmt_tags =
        conn.prepare_cached("INSERT OR IGNORE INTO action_tags (action_id, tag) VALUES (?1, ?2)")?;
    for tag in tags {
        stmt_tags.execute(params![id.as_str(), tag.as_str()])?;
    }
    Ok(())
}

/// Fetches associated files and tags for an action ID.
fn fetch_action_files_and_tags(
    conn: &rusqlite::Connection,
    id: &ActionId,
) -> KnoxResult<(Vec<String>, Vec<String>)> {
    let mut file_stmt = conn.prepare_cached(
        "SELECT file_path FROM action_files WHERE action_id = ?1 ORDER BY file_path",
    )?;
    let files = file_stmt
        .query_map(params![id.as_str()], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<String>, _>>()?;

    let mut tag_stmt =
        conn.prepare_cached("SELECT tag FROM action_tags WHERE action_id = ?1 ORDER BY tag")?;
    let tags = tag_stmt
        .query_map(params![id.as_str()], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<String>, _>>()?;

    Ok((files, tags))
}

/// Retrieves an action by its unique identifier, including associated files and tags.
pub fn get_action(db: &KnoxDb, id: &ActionId) -> KnoxResult<Option<KnoxAction>> {
    let conn = db.conn();

    let mut stmt = conn.prepare(
        "SELECT id, actor, action_type, summary, confidence, timestamp,
                session_id, parent_action_id, superseded_by, is_current
         FROM actions WHERE id = ?1",
    )?;

    let mut rows = stmt.query(params![id.as_str()])?;
    if let Some(row) = rows.next()? {
        let action_id = ActionId::from(row.get::<_, String>(0)?);
        let actor: String = row.get(1)?;
        let action_type: String = row.get(2)?;
        let summary: String = row.get(3)?;
        let confidence: Option<f64> = row.get(4)?;
        let timestamp_str: String = row.get(5)?;
        let session_id: Option<String> = row.get(6)?;
        let parent_action_id: Option<String> = row.get(7)?;
        let superseded_by: Option<String> = row.get(8)?;
        let is_current_int: i32 = row.get(9)?;

        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let (files, tags) = fetch_action_files_and_tags(conn, id)?;

        Ok(Some(KnoxAction {
            id: action_id,
            actor,
            action_type,
            summary,
            files,
            tags,
            confidence,
            timestamp,
            session_id: session_id.map(SessionId::from),
            parent_action_id: parent_action_id.map(ActionId::from),
            supersedes: None,
            is_current: is_current_int == 1 && superseded_by.is_none(),
        }))
    } else {
        Ok(None)
    }
}

/// Marks an existing action as superseded by a newer action.
pub fn mark_superseded(db: &KnoxDb, old_id: &ActionId, new_id: &ActionId) -> KnoxResult<()> {
    db.conn().execute(
        "UPDATE actions SET superseded_by = ?1, is_current = 0 WHERE id = ?2",
        params![new_id.as_str(), old_id.as_str()],
    )?;
    Ok(())
}

/// Inserts a project record into the database.
pub fn insert_project(db: &KnoxDb, project: &Project) -> KnoxResult<ProjectId> {
    let clean_path = project
        .path
        .strip_prefix(r"\\?\")
        .unwrap_or(&project.path)
        .trim_end_matches(['\\', '/']);

    db.conn().execute(
        "INSERT INTO projects (id, name, path, created_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(path) DO UPDATE SET name = excluded.name",
        params![
            project.id.as_str(),
            project.name.as_str(),
            clean_path,
            project.created_at.to_rfc3339()
        ],
    )?;
    Ok(project.id.clone())
}

/// Retrieves a project record by its unique identifier.
pub fn get_project(db: &KnoxDb, id: &ProjectId) -> KnoxResult<Option<Project>> {
    let conn = db.conn();
    let mut stmt = conn.prepare("SELECT id, name, path, created_at FROM projects WHERE id = ?1")?;
    let mut rows = stmt.query(params![id.as_str()])?;

    if let Some(row) = rows.next()? {
        let proj_id = ProjectId::from(row.get::<_, String>(0)?);
        let name: String = row.get(1)?;
        let path: String = row.get(2)?;
        let created_at_str: String = row.get(3)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Some(Project {
            id: proj_id,
            name,
            path,
            created_at,
        }))
    } else {
        Ok(None)
    }
}

/// Retrieves a project record by its canonical root path.
pub fn get_project_by_path(db: &KnoxDb, path: &str) -> KnoxResult<Option<Project>> {
    let clean = path
        .strip_prefix(r"\\?\")
        .unwrap_or(path)
        .trim_end_matches(['\\', '/']);
    let verbatim = format!(r"\\?\{clean}");

    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, name, path, created_at FROM projects WHERE path = ?1 OR path = ?2",
    )?;
    let mut rows = stmt.query(params![clean, verbatim])?;

    if let Some(row) = rows.next()? {
        let proj_id = ProjectId::from(row.get::<_, String>(0)?);
        let name: String = row.get(1)?;
        let mut p: String = row.get(2)?;
        if let Some(stripped) = p.strip_prefix(r"\\?\") {
            p = stripped.to_string();
        }
        let created_at_str: String = row.get(3)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Some(Project {
            id: proj_id,
            name,
            path: p,
            created_at,
        }))
    } else {
        Ok(None)
    }
}

/// Retrieves all projects ordered by creation time.
/// Normalizes paths by stripping Windows verbatim prefix `\\?\` if present.
pub fn list_projects(db: &KnoxDb) -> KnoxResult<Vec<Project>> {
    let conn = db.conn();
    let mut stmt = conn.prepare("SELECT id, name, path, created_at FROM projects ORDER BY created_at ASC")?;
    let mut rows = stmt.query([])?;
    let mut projects = Vec::new();

    while let Some(row) = rows.next()? {
        let proj_id = ProjectId::from(row.get::<_, String>(0)?);
        let name: String = row.get(1)?;
        let mut path: String = row.get(2)?;
        if let Some(stripped) = path.strip_prefix(r"\\?\") {
            path = stripped.to_string();
        }
        let created_at_str: String = row.get(3)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        projects.push(Project {
            id: proj_id,
            name,
            path,
            created_at,
        });
    }

    Ok(projects)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get_action() {
        let db = KnoxDb::open_memory().unwrap();
        let action = KnoxAction {
            id: ActionId::new(),
            actor: "cursor".to_string(),
            action_type: "write".to_string(),
            summary: "Implemented node CRUD".to_string(),
            files: vec!["src/storage/nodes.rs".to_string()],
            tags: vec!["storage".to_string(), "crud".to_string()],
            confidence: Some(0.9),
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
        };

        insert_action(&db, &action).unwrap();
        let fetched = get_action(&db, &action.id)
            .unwrap()
            .expect("action should exist");

        assert_eq!(fetched.id, action.id);
        assert_eq!(fetched.summary, action.summary);
        assert_eq!(fetched.files, action.files);
        let mut expected_tags = action.tags.clone();
        expected_tags.sort();
        assert_eq!(fetched.tags, expected_tags);
        assert!(fetched.is_current);
    }

    #[test]
    fn test_supersede_action() {
        let db = KnoxDb::open_memory().unwrap();
        let action1 = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "plan".to_string(),
            summary: "Old architecture plan".to_string(),
            files: vec!["plan.md".to_string()],
            tags: vec!["plan".to_string()],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
        };
        insert_action(&db, &action1).unwrap();

        let action2 = KnoxAction {
            id: ActionId::new(),
            actor: "antigravity".to_string(),
            action_type: "decide".to_string(),
            summary: "New architecture plan".to_string(),
            files: vec!["plan.md".to_string()],
            tags: vec!["plan".to_string()],
            confidence: Some(1.0),
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: Some(action1.id.clone()),
            is_current: true,
        };
        insert_action(&db, &action2).unwrap();

        let fetched1 = get_action(&db, &action1.id).unwrap().unwrap();
        let fetched2 = get_action(&db, &action2.id).unwrap().unwrap();

        assert!(!fetched1.is_current);
        assert!(fetched2.is_current);
    }

    #[test]
    fn test_list_projects() {
        let db = KnoxDb::open_memory().unwrap();
        let proj1 = Project {
            id: ProjectId::new(),
            name: "Knox".to_string(),
            path: r"\\?\D:\Knox".to_string(),
            created_at: Utc::now(),
        };
        let proj2 = Project {
            id: ProjectId::new(),
            name: "hongli".to_string(),
            path: r"D:\hongli".to_string(),
            created_at: Utc::now(),
        };

        insert_project(&db, &proj1).unwrap();
        insert_project(&db, &proj2).unwrap();

        let list = list_projects(&db).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Knox");
        assert_eq!(list[0].path, r"D:\Knox"); // verbatim prefix stripped
        assert_eq!(list[1].name, "hongli");
        assert_eq!(list[1].path, r"D:\hongli");
    }
}
