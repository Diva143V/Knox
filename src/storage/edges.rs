//! Edge operations and relationship linking for actions in Knox.

use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};

use super::db::KnoxDb;
use crate::core::error::KnoxResult;
use crate::core::ids::{ActionId, ProjectId};
use crate::core::types::{Edge, EdgeRejection, Ref};

/// Inserts or updates an edge in the `edges` table.
pub fn insert_edge(db: &KnoxDb, edge: &Edge) -> KnoxResult<()> {
    db.conn().execute(
        "INSERT INTO edges (id, from_action_id, to_id, edge_type, created_at, label, provenance, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET
             edge_type = excluded.edge_type,
             label = excluded.label,
             provenance = excluded.provenance,
             note = excluded.note",
        params![
            edge.id,
            edge.from_action_id.as_str(),
            edge.to_id,
            edge.edge_type,
            edge.created_at.to_rfc3339(),
            edge.label,
            edge.provenance,
            edge.note,
        ],
    )?;
    Ok(())
}

/// Retrieves all edges directly connected to an action (either as origin or target).
pub fn get_edges_for_action(db: &KnoxDb, action_id: &ActionId) -> KnoxResult<Vec<Edge>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, from_action_id, to_id, edge_type, created_at, label, provenance, note
         FROM edges
         WHERE from_action_id = ?1 OR to_id = ?1
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![action_id.as_str()], map_edge_row)?;
    let mut edges = Vec::new();
    for row in rows {
        edges.push(row?);
    }
    Ok(edges)
}

/// Retrieves all outgoing edges originating from a set of actions.
pub fn get_all_edges_for_actions(db: &KnoxDb, action_ids: &[ActionId]) -> KnoxResult<Vec<Edge>> {
    if action_ids.is_empty() {
        return Ok(Vec::new());
    }
    let conn = db.conn();
    let mut edges = Vec::new();

    for chunk in action_ids.chunks(500) {
        let placeholders = vec!["?"; chunk.len()].join(", ");
        let sql = format!(
            "SELECT id, from_action_id, to_id, edge_type, created_at, label, provenance, note
             FROM edges
             WHERE from_action_id IN ({placeholders})
             ORDER BY created_at ASC"
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut params_vec: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(chunk.len());
        for id in chunk {
            params_vec.push(&id.0);
        }
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec.iter()), map_edge_row)?;
        for row in rows {
            edges.push(row?);
        }
    }
    Ok(edges)
}

fn map_edge_row(row: &rusqlite::Row) -> rusqlite::Result<Edge> {
    let created_at_str: String = row.get(4)?;
    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    Ok(Edge {
        id: row.get(0)?,
        from_action_id: ActionId::from(row.get::<_, String>(1)?),
        to_id: row.get(2)?,
        edge_type: row.get(3)?,
        created_at,
        label: row.get(5)?,
        provenance: row.get(6)?,
        note: row.get(7)?,
    })
}

/// Inserts or updates an external reference in the `refs` table.
pub fn insert_ref(db: &KnoxDb, ref_entry: &Ref) -> KnoxResult<()> {
    db.conn().execute(
        "INSERT INTO refs (id, kind, value, project_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
             kind = excluded.kind,
             value = excluded.value,
             project_id = excluded.project_id",
        params![
            ref_entry.id,
            ref_entry.kind,
            ref_entry.value,
            ref_entry.project_id.as_ref().map(|p| p.as_str()),
            ref_entry.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Looks up an external reference in the `refs` table by ID.
pub fn get_ref(db: &KnoxDb, id: &str) -> KnoxResult<Option<Ref>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, kind, value, project_id, created_at FROM refs WHERE id = ?1",
    )?;
    let ref_opt = stmt
        .query_row(params![id], |row| {
            let created_at_str: String = row.get(4)?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let project_str: Option<String> = row.get(3)?;
            Ok(Ref {
                id: row.get(0)?,
                kind: row.get(1)?,
                value: row.get(2)?,
                project_id: project_str.map(ProjectId::from),
                created_at,
            })
        })
        .optional()?;
    Ok(ref_opt)
}

/// Ensures an external reference exists in the `refs` table, creating it if missing.
pub fn ensure_ref(
    db: &KnoxDb,
    id: &str,
    kind: &str,
    value: &str,
    project_id: Option<&ProjectId>,
) -> KnoxResult<()> {
    let now = Utc::now().to_rfc3339();
    db.conn().execute(
        "INSERT OR IGNORE INTO refs (id, kind, value, project_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            kind,
            value,
            project_id.map(|p| p.as_str()),
            now,
        ],
    )?;
    Ok(())
}

/// Records a rejected relation attempt in the `edge_rejections` table.
pub fn record_edge_rejection(
    db: &KnoxDb,
    project_id: Option<&str>,
    from_id: Option<&str>,
    to_id: Option<&str>,
    attempted_type: &str,
    reason: &str,
) -> KnoxResult<()> {
    let id = format!("rej_{}", uuid::Uuid::new_v4());
    let ts = Utc::now().to_rfc3339();
    db.conn().execute(
        "INSERT INTO edge_rejections (id, ts, project_id, from_id, to_id, attempted_type, reason)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, ts, project_id, from_id, to_id, attempted_type, reason],
    )?;
    Ok(())
}

/// Retrieves the most recent edge rejections.
pub fn get_edge_rejections(db: &KnoxDb, limit: usize) -> KnoxResult<Vec<EdgeRejection>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT id, ts, project_id, from_id, to_id, attempted_type, reason
         FROM edge_rejections
         ORDER BY rowid DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(EdgeRejection {
            id: row.get(0)?,
            ts: row.get(1)?,
            project_id: row.get(2)?,
            from_id: row.get(3)?,
            to_id: row.get(4)?,
            attempted_type: row.get(5)?,
            reason: row.get(6)?,
        })
    })?;
    let mut rejections = Vec::new();
    for r in rows {
        rejections.push(r?);
    }
    Ok(rejections)
}

/// Links an action to its causal parent action (`parent_action_id`).
pub fn link_parent(db: &KnoxDb, child_id: &ActionId, parent_id: &ActionId) -> KnoxResult<()> {
    db.conn().execute(
        "UPDATE actions SET parent_action_id = ?1 WHERE id = ?2",
        params![parent_id.as_str(), child_id.as_str()],
    )?;
    Ok(())
}

/// Links an action as temporally following a predecessor action.
/// Sets the `parent_action_id` to the predecessor if not already linked.
pub fn link_follows(db: &KnoxDb, prev_id: &ActionId, next_id: &ActionId) -> KnoxResult<()> {
    db.conn().execute(
        "UPDATE actions SET parent_action_id = ?1 WHERE id = ?2 AND parent_action_id IS NULL",
        params![prev_id.as_str(), next_id.as_str()],
    )?;
    Ok(())
}

/// Retrieves the parent action identifier for a given action, if one exists.
pub fn get_parent_action_id(db: &KnoxDb, id: &ActionId) -> KnoxResult<Option<ActionId>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT parent_action_id FROM actions WHERE id = ?1 AND parent_action_id IS NOT NULL",
    )?;
    let mut rows = stmt.query(params![id.as_str()])?;

    if let Some(row) = rows.next()? {
        let parent: String = row.get(0)?;
        Ok(Some(ActionId::from(parent)))
    } else {
        Ok(None)
    }
}

/// Retrieves all direct children (actions that declare this action as their parent).
pub fn get_child_action_ids(db: &KnoxDb, id: &ActionId) -> KnoxResult<Vec<ActionId>> {
    let conn = db.conn();
    let mut stmt =
        conn.prepare("SELECT id FROM actions WHERE parent_action_id = ?1 ORDER BY COALESCE(timestamp_ms, 0) ASC")?;
    let rows = stmt.query_map(params![id.as_str()], |r| {
        r.get::<_, String>(0).map(ActionId::from)
    })?;

    let mut children = Vec::new();
    for child in rows {
        children.push(child?);
    }
    Ok(children)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::KnoxAction;
    use crate::storage::nodes::insert_action;
    use chrono::Utc;

    #[test]
    fn test_link_parent_and_get_children() {
        let db = KnoxDb::open_memory().unwrap();
        let parent = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "plan".to_string(),
            summary: "Root task".to_string(),
            files: vec![],
            tags: vec![],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        };
        let child = KnoxAction {
            id: ActionId::new(),
            actor: "gemini".to_string(),
            action_type: "write".to_string(),
            summary: "Subtask".to_string(),
            files: vec![],
            tags: vec![],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        };

        insert_action(&db, &parent).unwrap();
        insert_action(&db, &child).unwrap();

        link_parent(&db, &child.id, &parent.id).unwrap();

        let parent_id = get_parent_action_id(&db, &child.id).unwrap();
        assert_eq!(parent_id, Some(parent.id.clone()));

        let children = get_child_action_ids(&db, &parent.id).unwrap();
        assert_eq!(children, vec![child.id]);
    }

    #[test]
    fn test_edge_and_ref_helpers() {
        let db = KnoxDb::open_memory().unwrap();
        let action = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type: "plan".to_string(),
            summary: "Root task".to_string(),
            files: vec![],
            tags: vec![],
            confidence: None,
            timestamp: Utc::now(),
            session_id: None,
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            ..Default::default()
        };
        insert_action(&db, &action).unwrap();

        // 1. Insert and retrieve ref
        let ref_entry = Ref {
            id: "file:src/db.rs".to_string(),
            kind: "file".to_string(),
            value: "src/db.rs".to_string(),
            project_id: None,
            created_at: Utc::now(),
        };
        insert_ref(&db, &ref_entry).unwrap();
        let fetched_ref = get_ref(&db, "file:src/db.rs").unwrap().unwrap();
        assert_eq!(fetched_ref.id, "file:src/db.rs");
        assert_eq!(fetched_ref.value, "src/db.rs");

        // 2. Insert edge pointing to the ref
        let edge = Edge {
            id: format!("{}:implements:file:src/db.rs", action.id),
            from_action_id: action.id.clone(),
            to_id: "file:src/db.rs".to_string(),
            edge_type: "implements".to_string(),
            created_at: Utc::now(),
            label: Some("database implementation".to_string()),
            provenance: Some("test".to_string()),
            note: Some("resolves schema task".to_string()),
        };
        insert_edge(&db, &edge).unwrap();

        let edges = get_edges_for_action(&db, &action.id).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to_id, "file:src/db.rs");
        assert_eq!(edges[0].edge_type, "implements");
        assert_eq!(edges[0].label.as_deref(), Some("database implementation"));

        // 3. Record and retrieve edge rejection
        record_edge_rejection(
            &db,
            None,
            Some(action.id.as_str()),
            Some("file:src/db.rs"),
            "invalid_type",
            "Testing edge rejection logging",
        )
        .unwrap();

        let rejections = get_edge_rejections(&db, 10).unwrap();
        assert_eq!(rejections.len(), 1);
        assert_eq!(rejections[0].attempted_type, "invalid_type");
        assert_eq!(rejections[0].reason, "Testing edge rejection logging");
    }
}
