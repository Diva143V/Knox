//! High-level record adapter over the atomic batch writer.

use serde_json::Value;

use crate::core::error::{KnoxError, KnoxResult};
use crate::storage::db::KnoxDb;

pub fn handle_record(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let object = params
        .as_object()
        .ok_or_else(|| KnoxError::InvalidInput("record params must be an object".to_string()))?;
    let entries = object
        .get("entries")
        .cloned()
        .ok_or_else(|| KnoxError::InvalidInput("record requires 'entries'".to_string()))?;
    let mut batch = object.clone();
    batch.remove("entries");
    batch.insert("actions".to_string(), entries);
    super::handle_record_batch(db, Value::Object(batch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn requires_entries() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(handle_record(&db, json!({"actor": "agent"})).is_err());
    }
}
