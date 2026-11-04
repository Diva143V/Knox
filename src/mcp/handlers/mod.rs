//! Handlers and dispatcher for Model Context Protocol (MCP) tools in Knox.
//!
//! Only the six high-level workflow handlers are exposed.

pub mod begin;
pub mod check;
pub mod check_guardrails;
pub mod get_context;
pub mod get_lineage;
pub mod handoff;
pub mod record;
pub mod trace;
pub mod prompts;
pub mod resources;

pub use begin::handle_begin;
pub use check::handle_check;
pub use get_context::handle_context;
pub use handoff::handle_handoff;
pub use record::handle_record;
pub use trace::handle_trace;
pub use prompts::{handle_prompts_get, handle_prompts_list};
pub use resources::{handle_resources_list, handle_resources_read};

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
        "initialize" => handle_initialize(id, params),
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
/// Only the six advertised tools are accepted; legacy names are no longer routed.
pub fn execute_tool(name: &str, params: Value, db: &KnoxDb) -> KnoxResult<Value> {
    let result = match name {
        "knox_context" => handle_context(db, params),
        "knox_begin" => handle_begin(db, params),
        "knox_handoff" => handle_handoff(db, params),
        "knox_check" => handle_check(db, params),
        "knox_trace" => handle_trace(db, params),
        "knox_record" => handle_record(db, params),
        _ => Err(KnoxError::InvalidInput(format!("Unknown tool: {name}"))),
    }?;
    Ok(result)
}

/// Protocol revisions this server understands, newest first.
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];
/// Version used when the client requests nothing or an unsupported revision.
const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";

/// Compact protocol summary returned in the `initialize` response so clients
/// that never surface `knox.md` still give the model the operating contract.
const SERVER_INSTRUCTIONS: &str = "\
Knox is an external long-term memory graph for AI coding agents. Use it instead of re-deriving project context.

Mandatory protocol:
1. Start every turn with knox_begin to resume the session and receive budgeted context.
2. Pre-flight: call knox_check with the files you are about to touch (and any symbols) to read active decisions, open doubts and superseded anti-patterns before editing.
3. During work: call knox_record once per turn with every decision, doubt and code change. Include files, symbols, why, and relations; give each entry a ref when another entry in the same call points at it.
4. Navigation: use knox_trace to trace why code exists before refactoring.
5. End the turn with knox_handoff to close the session and produce the digest the next session resumes from.

Never delete history; append and supersede. Identify yourself in actor (for example gemini, cursor, claude, codex, human). Use only the six tools above.";

/// Negotiates the protocol revision, echoing a supported client request.
fn negotiate_protocol_version(params: &Value) -> String {
    params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .filter(|requested| SUPPORTED_PROTOCOL_VERSIONS.contains(requested))
        .unwrap_or(DEFAULT_PROTOCOL_VERSION)
        .to_string()
}

fn handle_initialize(id: Option<Value>, params: Value) -> McpResponse {
    McpResponse::success(
        id,
        json!({
            "protocolVersion": negotiate_protocol_version(&params),
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            },
            "serverInfo": {
                "name": "knox",
                "version": env!("CARGO_PKG_VERSION")
            },
            "instructions": SERVER_INSTRUCTIONS
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
            // D9: never return the same fact in two shapes — content carries the
            // rendered text; structuredContent is dropped to avoid duplication.
            McpResponse::success(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": text
                    }],
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
        | KnoxError::SessionClosed(msg)
        | KnoxError::IdemConflict(msg)
        | KnoxError::OversizeInput(msg)
        | KnoxError::InvalidRef(msg)
        | KnoxError::CrossProject(msg)
        | KnoxError::Cycle(msg) => McpResponse::error(id, McpError::invalid_params(msg)),
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
    fn test_initialize_negotiates_protocol_and_returns_instructions() {
        let db = KnoxDb::open_memory().unwrap();

        let supported = dispatch(
            "initialize",
            json!({ "protocolVersion": "2024-11-05" }),
            &db,
        );
        let result = supported.result.unwrap();
        assert_eq!(result.get("protocolVersion").unwrap(), "2024-11-05");
        assert!(
            result
                .get("instructions")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("knox_check")
        );

        let unknown = dispatch(
            "initialize",
            json!({ "protocolVersion": "1999-01-01" }),
            &db,
        );
        let result = unknown.result.unwrap();
        assert_eq!(
            result.get("protocolVersion").unwrap(),
            DEFAULT_PROTOCOL_VERSION
        );

        let missing = dispatch("initialize", json!({}), &db);
        let result = missing.result.unwrap();
        assert_eq!(
            result.get("protocolVersion").unwrap(),
            DEFAULT_PROTOCOL_VERSION
        );
    }

    #[test]
    fn test_dispatch_tools_list() {
        let db = KnoxDb::open_memory().unwrap();
        let res = dispatch("tools/list", json!({}), &db);
        assert!(res.error.is_none());
        let result = res.result.unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), crate::mcp::tools::TOOL_COUNT);
    }

    #[test]
    fn test_dispatch_resources_and_prompts() {
        let db = KnoxDb::open_memory().unwrap();
        let res_list = dispatch("resources/list", json!({}), &db);
        assert!(res_list.error.is_none());
        let resources = res_list
            .result
            .unwrap()
            .get("resources")
            .unwrap()
            .as_array()
            .unwrap()
            .len();
        assert!(resources >= 2);

        let res_read = dispatch(
            "resources/read",
            json!({"uri": "knox://project/overview"}),
            &db,
        );
        assert!(res_read.error.is_none());

        let p_list = dispatch("prompts/list", json!({}), &db);
        assert!(p_list.error.is_none());

        let p_get = dispatch("prompts/get", json!({"name": "architectural-audit"}), &db);
        assert!(p_get.error.is_none());
    }

    #[test]
    fn test_dispatch_tools_call_begin() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "name": "knox_begin",
            "arguments": {
                "actor": "gemini"
            }
        });

        let res = dispatch("tools/call", params, &db);
        assert!(res.error.is_none());
        let result = res.result.unwrap();
        let content = result.get("content").unwrap().as_array().unwrap();
        assert!(!content.is_empty());
        // D9: structuredContent is not returned — content is the single shape.
        assert!(result.get("structuredContent").is_none());
    }

    #[test]
    fn test_dispatch_direct_tool_call() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "actor": "cursor",
            "workspace": "demo"
        });

        let res = dispatch("knox_begin", params, &db);
        assert!(res.error.is_none());
        let result = res.result.unwrap();
        assert!(
            result.get("scope").and_then(|s| s.get("session_id")).is_some()
        );
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
