//! Edge operations and relationship linking for actions in Knox.

use rusqlite::params;

use super::db::KnoxDb;
use crate::core::error::KnoxResult;
use crate::core::ids::ActionId;

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
        conn.prepare("SELECT id FROM actions WHERE parent_action_id = ?1 ORDER BY timestamp ASC")?;
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
        };

        insert_action(&db, &parent).unwrap();
        insert_action(&db, &child).unwrap();

        link_parent(&db, &child.id, &parent.id).unwrap();

        let parent_id = get_parent_action_id(&db, &child.id).unwrap();
        assert_eq!(parent_id, Some(parent.id.clone()));

        let children = get_child_action_ids(&db, &parent.id).unwrap();
        assert_eq!(children, vec![child.id]);
    }
}
