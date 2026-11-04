//! Legacy `knox_record_batch` adapter.
//!
//! The batch tool keeps its original request and response shape for existing
//! clients, but every write now goes through the atomic `knox_record` path so
//! validation, local refs, dedupe and idempotency behave identically.

use serde_json::{Map, Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

/// Translates one legacy batch item into a `knox_record` entry.
fn translate_item(item: &Value, default_link: Option<bool>) -> KnoxResult<Value> {
    let object = item
        .as_object()
        .ok_or_else(|| KnoxError::InvalidInput("batch actions must be objects".to_string()))?;
    let mut entry = Map::new();

    if let Some(action_type) = object.get("action_type") {
        entry.insert("kind".to_string(), action_type.clone());
    }
    for key in [
        "summary",
        "files",
        "tags",
        "confidence",
        "supersedes",
        "symbols",
        "rationale",
        "rejected_alternatives",
        "context_summary",
        "significance",
        "is_resolved",
        "resolution_note",
        "parent_action_id",
        "caused_by",
        "ref",
    ] {
        if let Some(value) = object.get(key) {
            entry.insert(key.to_string(), value.clone());
        }
    }

    if let Some(relations) = object.get("relations").and_then(Value::as_array) {
        let translated: Vec<Value> = relations
            .iter()
            .map(|relation| {
                let mut relation_object = relation.as_object().cloned().unwrap_or_default();
                if !relation_object.contains_key("to") {
                    if let Some(target) = relation_object.remove("target_id") {
                        relation_object.insert("to".to_string(), target);
                    }
                }
                Value::Object(relation_object)
            })
            .collect();
        entry.insert("relations".to_string(), Value::Array(translated));
    }

    let link = object
        .get("link_to_previous")
        .and_then(Value::as_bool)
        .or(default_link);
    if let Some(link) = link {
        entry.insert("link_to_previous".to_string(), Value::Bool(link));
    }

    Ok(Value::Object(entry))
}

/// Handles the `knox_record_batch` tool invocation.
pub fn handle_record_batch(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let object = params.as_object().ok_or_else(|| {
        KnoxError::InvalidInput("record_batch params must be an object".to_string())
    })?;

    let actions = object
        .get("actions")
        .and_then(Value::as_array)
        .ok_or_else(|| KnoxError::InvalidInput("record_batch requires 'actions'".to_string()))?;

    let default_link = object.get("link_to_previous").and_then(Value::as_bool);
    let entries: Vec<Value> = actions
        .iter()
        .map(|item| translate_item(item, default_link))
        .collect::<KnoxResult<_>>()?;

    let mut record = Map::new();
    if let Some(actor) = object.get("actor") {
        record.insert("actor".to_string(), actor.clone());
    }
    for key in ["project", "session_id", "idem", "dry_run"] {
        if let Some(value) = object.get(key) {
            record.insert(key.to_string(), value.clone());
        }
    }
    record.insert("entries".to_string(), Value::Array(entries));

    let ack = super::handle_record(db, Value::Object(record))?;

    // Preserve the legacy response keys while surfacing the new diagnostics.
    let ids = ack.get("ids").cloned().unwrap_or_else(|| json!([]));
    let mut response = json!({
        "status": ack.get("status").cloned().unwrap_or_else(|| json!("recorded")),
        "count": ack.get("count").cloned().unwrap_or_else(|| json!(0)),
        "recorded_count": ack.get("recorded_count").cloned().unwrap_or_else(|| json!(0)),
        "ids": ids.clone(),
        "action_ids": ids,
        "session_id": ack.get("session_id").cloned().unwrap_or(Value::Null),
        "session_implicit": ack.get("session_implicit").cloned().unwrap_or(json!(false)),
        "deduped": ack.get("deduped").cloned().unwrap_or(json!([])),
        "warnings": ack.get("warnings").cloned().unwrap_or(json!([])),
        "deprecated": "knox_record_batch is kept for compatibility; prefer knox_record"
    });
    if let Some(refs) = ack.get("refs") {
        response["refs"] = refs.clone();
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_record_batch_with_link_to_previous() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "actor": "gemini",
            "actions": [
                {
                    "action_type": "decide",
                    "summary": "Architecture decision 1",
                    "symbols": ["Server"]
                },
                {
                    "action_type": "write",
                    "summary": "Implementation 1",
                    "files": ["src/main.rs"],
                    "link_to_previous": true
                }
            ]
        });

        let res = handle_record_batch(&db, params).unwrap();
        assert_eq!(res.get("count").unwrap(), 2);
        let ids = res.get("ids").unwrap().as_array().unwrap();
        assert_eq!(ids.len(), 2);

        let second_id = crate::core::ids::ActionId::from(ids[1].as_str().unwrap());
        let second_act = crate::storage::nodes::get_action(&db, &second_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            second_act.parent_action_id.unwrap().as_str(),
            ids[0].as_str().unwrap()
        );
    }

    #[test]
    fn legacy_batch_is_atomic() {
        let db = KnoxDb::open_memory().unwrap();
        let result = handle_record_batch(
            &db,
            json!({
                "actor": "gemini",
                "actions": [
                    {"action_type": "plan", "summary": "Valid first"},
                    {"action_type": "write", "summary": "x".repeat(400)}
                ]
            }),
        );
        assert!(result.is_err());
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
