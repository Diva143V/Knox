//! MCP tool handler for `knox_record`.
//!
//! Thin adapter: parses params, delegates to `cmd_record`, renders response.

use serde_json::Value;

use crate::core::error::KnoxResult;
use crate::storage::db::KnoxDb;

pub fn handle_record(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    crate::commands::record::cmd_record(db, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::KnoxError;
    use serde_json::json;

    fn record(db: &KnoxDb, entries: Value) -> KnoxResult<Value> {
        handle_record(db, json!({ "actor": "gemini", "project": "demo", "entries": entries }))
    }

    #[test]
    fn requires_entries() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(handle_record(&db, json!({"actor": "agent", "project": "demo"})).is_err());
    }

    #[test]
    fn dedupe_exact_increments_seen_count() {
        let db = KnoxDb::open_memory().unwrap();
        let entry = json!([{
            "kind": "action",
            "summary": "Implement parser",
            "files": ["src/parser.rs"],
            "symbols": ["parse"]
        }]);
        let first = record(&db, entry.clone()).unwrap();
        let second = record(&db, entry).unwrap();
        assert_eq!(first["ids"][0], second["ids"][0]);
        assert_eq!(second["deduped"].as_array().unwrap().len(), 1);

        let id = crate::core::ids::ActionId::from(second["ids"][0].as_str().unwrap());
        let action = crate::storage::nodes::get_action(&db, &id)
            .unwrap()
            .unwrap();
        assert_eq!(action.seen_count, 2);
    }

    #[test]
    fn dedupe_near_respects_window() {
        let db = KnoxDb::open_memory().unwrap();
        let first = record(
            &db,
            json!([{
                "kind": "action",
                "summary": "implement the token parser for imports",
                "files": ["src/parser.rs"]
            }]),
        )
        .unwrap();
        let id = crate::core::ids::ActionId::from(first["ids"][0].as_str().unwrap());

        // Move the original outside the 24h window.
        db.conn()
            .execute(
                "UPDATE actions SET timestamp_ms = ?1 WHERE id = ?2",
                rusqlite::params![
                    (chrono::Utc::now() - chrono::Duration::hours(48)).timestamp_millis(),
                    id.as_str()
                ],
            )
            .unwrap();

        let second = record(
            &db,
            json!([{
                "kind": "action",
                "summary": "implement the token parser for imports",
                "files": ["src/parser.rs"]
            }]),
        )
        .unwrap();
        assert_ne!(first["ids"][0], second["ids"][0]);
        assert!(second["deduped"].as_array().unwrap().is_empty());
    }

    #[test]
    fn local_refs_resolve_to_committed_ids() {
        let db = KnoxDb::open_memory().unwrap();
        let result = record(
            &db,
            json!([
                {"ref": "e1", "kind": "plan", "summary": "Plan the parser"},
                {
                    "ref": "e2",
                    "kind": "action",
                    "summary": "Write the parser",
                    "files": ["src/parser.rs"],
                    "caused_by": "e1",
                    "relations": [{"to": "e1", "relation_type": "implements"}]
                }
            ]),
        )
        .unwrap();

        let e2 = crate::core::ids::ActionId::from(result["refs"]["e2"].as_str().unwrap());
        let action = crate::storage::nodes::get_action(&db, &e2)
            .unwrap()
            .unwrap();
        assert_eq!(
            action.parent_action_id.unwrap().as_str(),
            result["refs"]["e1"].as_str().unwrap()
        );
        assert_eq!(
            action.relations[0].target_id.as_str(),
            result["refs"]["e1"].as_str().unwrap()
        );
    }

    #[test]
    fn idempotency_replays_and_rejects_different_payload() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "actor": "gemini",
            "project": "demo",
            "idem": "turn-1",
            "entries": [{"kind": "plan", "summary": "Plan once"}]
        });
        let first = handle_record(&db, params.clone()).unwrap();
        let second = handle_record(&db, params).unwrap();
        assert_eq!(first["ids"], second["ids"]);

        let conflicting = json!({
            "actor": "gemini",
            "project": "demo",
            "idem": "turn-1",
            "entries": [{"kind": "plan", "summary": "Plan twice"}]
        });
        assert!(matches!(
            handle_record(&db, conflicting),
            Err(KnoxError::IdemConflict(_))
        ));
    }

    #[test]
    fn oversize_and_invalid_refs_are_errors() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(matches!(
            record(
                &db,
                json!([{"kind": "plan", "summary": "x".repeat(301)}])
            ),
            Err(KnoxError::OversizeInput(_))
        ));
        assert!(matches!(
            record(
                &db,
                json!([{
                    "kind": "action",
                    "summary": "Write code",
                    "relations": [{"to": "missing-ref"}]
                }])
            ),
            Err(KnoxError::InvalidRef(_))
        ));
    }

    #[test]
    fn batch_failure_leaves_no_rows() {
        let db = KnoxDb::open_memory().unwrap();
        let before: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM actions", [], |r| r.get(0))
            .unwrap();
        let result = handle_record(
            &db,
            json!([
                {"kind": "plan", "summary": "Valid first"},
                {"kind": "action", "summary": "Valid second"},
                {"kind": "action", "summary": "x".repeat(301)}
            ]),
        );
        assert!(result.is_err());
        let after: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM actions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn low_value_and_no_guardrail_warnings() {
        let db = KnoxDb::open_memory().unwrap();
        let result = record(
            &db,
            json!([
                {"kind": "action", "summary": "Tweak something"},
                {"kind": "decision", "summary": "Choose a database"}
            ]),
        )
        .unwrap();
        let warnings = result["warnings"].as_array().unwrap();
        assert!(warnings.iter().any(|w| w["code"] == "low_value"));
        assert!(warnings.iter().any(|w| w["code"] == "no_guardrail"));
    }

    #[test]
    fn local_ref_to_deduped_entry_points_at_existing_row() {
        let db = KnoxDb::open_memory().unwrap();
        let first = record(
            &db,
            json!([{
                "ref": "plan",
                "kind": "plan",
                "summary": "Plan the parser",
                "files": ["src/parser.rs"]
            }]),
        )
        .unwrap();
        let existing = first["ids"][0].as_str().unwrap().to_string();

        let second = record(
            &db,
            json!([
                {
                    "ref": "same-plan",
                    "kind": "plan",
                    "summary": "Plan the parser",
                    "files": ["src/parser.rs"]
                },
                {
                    "ref": "work",
                    "kind": "action",
                    "summary": "Implement the lexer",
                    "files": ["src/lexer.rs"],
                    "caused_by": "same-plan"
                }
            ]),
        )
        .unwrap();

        assert_eq!(second["refs"]["same-plan"], existing);
        let work = crate::core::ids::ActionId::from(second["refs"]["work"].as_str().unwrap());
        let action = crate::storage::nodes::get_action(&db, &work)
            .unwrap()
            .unwrap();
        assert_eq!(action.parent_action_id.unwrap().as_str(), existing);
    }

    #[test]
    fn dry_run_reports_but_writes_nothing() {
        let db = KnoxDb::open_memory().unwrap();
        let result = handle_record(
            &db,
            json!({
                "actor": "gemini",
                "project": "demo",
                "dry_run": true,
                "entries": [{"kind": "plan", "summary": "Plan only"}]
            }),
        )
        .unwrap();
        assert_eq!(result["status"], "dry_run");
        assert_eq!(result["count"], 1);
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn self_reference_closes_a_cycle() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(matches!(
            record(
                &db,
                json!([{
                    "ref": "e1",
                    "kind": "action",
                    "summary": "Circular work",
                    "files": ["src/cycle.rs"],
                    "caused_by": "e1"
                }])
            ),
            Err(KnoxError::Cycle(_))
        ));
    }

    #[test]
    fn relation_to_another_project_is_rejected() {
        use crate::core::ids::ProjectId;
        use crate::core::types::Project;

        let db = KnoxDb::open_memory().unwrap();
        for (name, path) in [("alpha", "/work/alpha"), ("beta", "/work/beta")] {
            crate::storage::nodes::insert_project(
                &db,
                &Project {
                    id: ProjectId::new(),
                    name: name.to_string(),
                    path: path.to_string(),
                    remote: None,
                    created_at: chrono::Utc::now(),
                },
            )
            .unwrap();
        }

        let beta_session =
            crate::storage::sessions::start_session(&db, "gemini", Some("beta")).unwrap();
        let beta = handle_record(
            &db,
            json!({
                "actor": "gemini",
                "project": "beta",
                "session_id": beta_session.id.as_str(),
                "entries": [{"kind": "plan", "summary": "Beta plan"}]
            }),
        )
        .unwrap();
        let beta_id = beta["ids"][0].as_str().unwrap().to_string();

        let alpha_session =
            crate::storage::sessions::start_session(&db, "claude", Some("alpha")).unwrap();
        let result = handle_record(
            &db,
            json!({
                "actor": "claude",
                "project": "alpha",
                "session_id": alpha_session.id.as_str(),
                "entries": [{
                    "kind": "action",
                    "summary": "Alpha write",
                    "files": ["src/alpha.rs"],
                    "relations": [{"to": beta_id}]
                }]
            }),
        );
        assert!(matches!(result, Err(KnoxError::CrossProject(_))));
    }
}
