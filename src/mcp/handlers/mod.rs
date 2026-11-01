//! Handlers and dispatcher for Model Context Protocol (MCP) tools in Knox.

pub mod check_guardrails;
pub mod end_session;
pub mod get_context;
pub mod get_history;
pub mod get_impact;
pub mod get_lineage;
pub mod get_path;
pub mod ingest;
pub mod list_stale;
pub mod prompts;
pub mod record_action;
pub mod record_batch;
pub mod record_decision;
pub mod record_doubt;
pub mod resolve_doubt;
pub mod resources;
pub mod scan_staged;
pub mod search;
pub mod start_session;
pub mod update_action;

pub use check_guardrails::handle_check_guardrails;
pub use end_session::handle_end_session;
pub use get_context::handle_get_context;
pub use get_history::handle_get_history;
pub use get_impact::handle_get_impact;
pub use get_lineage::handle_get_lineage;
pub use get_path::handle_get_path;
pub use ingest::handle_ingest;
pub use list_stale::handle_list_stale;
pub use prompts::{handle_prompts_get, handle_prompts_list};
pub use record_action::handle_record_action;
pub use record_batch::handle_record_batch;
pub use record_decision::handle_record_decision;
pub use record_doubt::handle_record_doubt;
pub use resolve_doubt::handle_resolve_doubt;
pub use resources::{handle_resources_list, handle_resources_read};
pub use scan_staged::handle_scan_staged;
pub use search::handle_search;
pub use start_session::handle_start_session;
pub use update_action::handle_update_action;

use serde_json::{Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::mcp::protocol::{CallToolParams, McpError, McpRequest, McpResponse};
use crate::mcp::tools::all_tools;
use crate::storage::db::KnoxDb;

/// Dispatches an MCP request by method name and parameters against the database.
pub fn dispatch(method: &str, params: Value, db: &KnoxDb) -> McpResponse {
    dispatch_with_id(None, method, params, db)
}

/// Dispatches a structured `McpRequest`, preserving its request identifier.
pub fn dispatch_request(req: &McpRequest, db: &KnoxDb) -> McpResponse {
    dispatch_with_id(
        req.id.clone(),
        &req.method,
        req.params.clone().unwrap_or(Value::Null),
        db,
    )
}

/// Core dispatcher routing incoming methods to tool handlers or protocol endpoints.
pub fn dispatch_with_id(
    id: Option<Value>,
    method: &str,
    params: Value,
    db: &KnoxDb,
) -> McpResponse {
    match method {
        "initialize" => handle_initialize(id),
        "ping" => McpResponse::success(id, json!({})),
        "tools/list" => McpResponse::success(id, json!({ "tools": all_tools() })),
        "tools/call" => handle_tools_call(id, params, db),
        "resources/list" => McpResponse::success(id, handle_resources_list()),
        "resources/read" => match handle_resources_read(db, params) {
            Ok(res) => McpResponse::success(id, res),
            Err(err) => map_error(id, err),
        },
        "prompts/list" => McpResponse::success(id, handle_prompts_list()),
        "prompts/get" => match handle_prompts_get(db, params) {
            Ok(res) => McpResponse::success(id, res),
            Err(err) => map_error(id, err),
        },
        tool if tool.starts_with("knox_") => match execute_tool(tool, params, db) {
            Ok(res) => McpResponse::success(id, res),
            Err(err) => map_error(id, err),
        },
        unknown => McpResponse::error(
            id,
            McpError::method_not_found(format!("Method not found: {unknown}")),
        ),
    }
}

/// Executes an individual Knox tool by its canonical tool name.
pub fn execute_tool(name: &str, params: Value, db: &KnoxDb) -> KnoxResult<Value> {
    match name {
        "knox_start_session" => handle_start_session(db, params),
        "knox_end_session" => handle_end_session(db, params),
        "knox_record_action" => handle_record_action(db, params),
        "knox_record_decision" => handle_record_decision(db, params),
        "knox_record_doubt" => handle_record_doubt(db, params),
        "knox_resolve_doubt" => handle_resolve_doubt(db, params),
        "knox_record_batch" => handle_record_batch(db, params),
        "knox_update_action" => handle_update_action(db, params),
        "knox_get_context" => handle_get_context(db, params),
        "knox_get_history" => handle_get_history(db, params),
        "knox_get_lineage" => handle_get_lineage(db, params),
        "knox_get_impact" => handle_get_impact(db, params),
        "knox_get_path" => handle_get_path(db, params),
        "knox_check_guardrails" => handle_check_guardrails(db, params),
        "knox_search" => handle_search(db, params),
        "knox_list_stale" => handle_list_stale(db, params),
        "knox_scan_staged" => handle_scan_staged(db, params),
        "knox_ingest" => handle_ingest(db, params),
        _ => Err(KnoxError::InvalidInput(format!("Unknown tool: {name}"))),
    }
}

fn handle_initialize(id: Option<Value>) -> McpResponse {
    McpResponse::success(
        id,
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "serverInfo": {
                "name": "knox",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn handle_tools_call(id: Option<Value>, params: Value, db: &KnoxDb) -> McpResponse {
    let call_params: Result<CallToolParams, _> = serde_json::from_value(params);
    let call = match call_params {
        Ok(c) => c,
        Err(e) => {
            return McpResponse::error(
                id,
                McpError::invalid_params(format!("Invalid tools/call arguments: {e}")),
            );
        }
    };

    let args = call.arguments.unwrap_or(json!({}));
    match execute_tool(&call.name, args, db) {
        Ok(data) => {
            let text = if let Some(md) = data.get("markdown").and_then(|v| v.as_str()) {
                md.to_string()
            } else {
                serde_json::to_string_pretty(&data).unwrap_or_default()
            };
            McpResponse::success(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": text
                    }],
                    "structuredContent": data
                }),
            )
        }
        Err(err) => McpResponse::success(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": format!("Error: {err}")
                }],
                "isError": true
            }),
        ),
    }
}

fn map_error(id: Option<Value>, err: KnoxError) -> McpResponse {
    match err {
        KnoxError::InvalidInput(msg)
        | KnoxError::SessionNotFound(msg)
        | KnoxError::ActionNotFound(msg)
        | KnoxError::SessionClosed(msg) => McpResponse::error(id, McpError::invalid_params(msg)),
        KnoxError::Json(e) => McpResponse::error(id, McpError::parse_error(e.to_string())),
        other => McpResponse::error(id, McpError::internal_error(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_initialize() {
        let db = KnoxDb::open_memory().unwrap();
        let res = dispatch("initialize", json!({}), &db);
        assert!(res.error.is_none());
        let result = res.result.unwrap();
        assert_eq!(
            result.get("serverInfo").unwrap().get("name").unwrap(),
            "knox"
        );
    }

    #[test]
    fn test_dispatch_tools_list() {
        let db = KnoxDb::open_memory().unwrap();
        let res = dispatch("tools/list", json!({}), &db);
        assert!(res.error.is_none());
        let result = res.result.unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 18);
    }

    #[test]
    fn test_dispatch_resources_and_prompts() {
        let db = KnoxDb::open_memory().unwrap();
        let res_list = dispatch("resources/list", json!({}), &db);
        assert!(res_list.error.is_none());
        let resources = res_list.result.unwrap().get("resources").unwrap().as_array().unwrap().len();
        assert!(resources >= 2);

        let res_read = dispatch("resources/read", json!({"uri": "knox://project/overview"}), &db);
        assert!(res_read.error.is_none());

        let p_list = dispatch("prompts/list", json!({}), &db);
        assert!(p_list.error.is_none());

        let p_get = dispatch("prompts/get", json!({"name": "architectural-audit"}), &db);
        assert!(p_get.error.is_none());
    }

    #[test]
    fn test_dispatch_tools_call_start_session() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "name": "knox_start_session",
            "arguments": {
                "actor": "gemini"
            }
        });

        let res = dispatch("tools/call", params, &db);
        assert!(res.error.is_none());
        let result = res.result.unwrap();
        let content = result.get("content").unwrap().as_array().unwrap();
        assert!(!content.is_empty());
        assert!(result.get("structuredContent").is_some());
    }

    #[test]
    fn test_dispatch_direct_tool_call() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "actor": "cursor"
        });

        let res = dispatch("knox_start_session", params, &db);
        assert!(res.error.is_none());
        let result = res.result.unwrap();
        assert!(result.get("session_id").is_some());
    }

    #[test]
    fn test_dispatch_unknown_method() {
        let db = KnoxDb::open_memory().unwrap();
        let res = dispatch("unknown_rpc", json!({}), &db);
        assert!(res.error.is_some());
        assert_eq!(
            res.error.unwrap().code,
            crate::mcp::protocol::METHOD_NOT_FOUND
        );
    }
}
