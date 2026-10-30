//! JSON-RPC 2.0 and Model Context Protocol (MCP) data structures.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standard JSON-RPC 2.0 parse error code.
pub const PARSE_ERROR: i64 = -32700;

/// Standard JSON-RPC 2.0 invalid request code.
pub const INVALID_REQUEST: i64 = -32600;

/// Standard JSON-RPC 2.0 method not found code.
pub const METHOD_NOT_FOUND: i64 = -32601;

/// Standard JSON-RPC 2.0 invalid params code.
pub const INVALID_PARAMS: i64 = -32602;

/// Standard JSON-RPC 2.0 internal error code.
pub const INTERNAL_ERROR: i64 = -32603;

/// An incoming JSON-RPC 2.0 / MCP request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version string, typically `"2.0"`.
    #[serde(default = "default_jsonrpc")]
    pub jsonrpc: String,

    /// Request identifier (null or omitted for notifications).
    #[serde(default)]
    pub id: Option<Value>,

    /// The method name to invoke.
    pub method: String,

    /// Optional structured parameters for the method.
    #[serde(default)]
    pub params: Option<Value>,
}

fn default_jsonrpc() -> String {
    "2.0".to_string()
}

impl McpRequest {
    /// Creates a new MCP request with the given method and optional identifier.
    pub fn new(method: impl Into<String>, id: Option<Value>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: default_jsonrpc(),
            id,
            method: method.into(),
            params,
        }
    }

    /// Returns `true` if this request is a notification (lacks an id).
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// An outgoing JSON-RPC 2.0 / MCP response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC version string, always `"2.0"`.
    pub jsonrpc: String,

    /// Request identifier matching the incoming request, or null.
    pub id: Option<Value>,

    /// Result payload if the request succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error payload if the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

impl McpResponse {
    /// Creates a successful MCP response.
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: default_jsonrpc(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Creates an error MCP response.
    pub fn error(id: Option<Value>, error: McpError) -> Self {
        Self {
            jsonrpc: default_jsonrpc(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC 2.0 error representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpError {
    /// Error code (standard JSON-RPC or application-specific).
    pub code: i64,

    /// Concise human-readable error description.
    pub message: String,

    /// Optional additional error context or diagnostic data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl McpError {
    /// Constructs a custom error with code, message, and optional extra data.
    pub fn custom(code: i64, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }

    /// Constructs a `-32700 Parse error` response.
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::custom(PARSE_ERROR, message, None)
    }

    /// Constructs a `-32600 Invalid Request` response.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::custom(INVALID_REQUEST, message, None)
    }

    /// Constructs a `-32601 Method not found` response.
    pub fn method_not_found(message: impl Into<String>) -> Self {
        Self::custom(METHOD_NOT_FOUND, message, None)
    }

    /// Constructs a `-32602 Invalid params` response.
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::custom(INVALID_PARAMS, message, None)
    }

    /// Constructs a `-32603 Internal error` response.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::custom(INTERNAL_ERROR, message, None)
    }
}

/// Metadata and input schema for a single tool exposed by Knox MCP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescription {
    /// Unique name of the tool (e.g. `"knox_record_action"`).
    pub name: String,

    /// Human-readable explanation of the tool's function.
    pub description: String,

    /// JSON Schema Draft-07 describing the tool's acceptable arguments.
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

impl ToolDescription {
    /// Constructs a new tool description.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// Standard parameters received when `tools/call` is invoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallToolParams {
    /// Target tool name.
    pub name: String,

    /// Arguments object supplied to the tool.
    #[serde(default)]
    pub arguments: Option<Value>,
}

/// Individual content block in an MCP tool call output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    /// Textual content response.
    #[serde(rename = "text")]
    Text {
        /// Text body.
        text: String,
    },
}

/// Return envelope for an MCP tool execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// Content items produced by the tool.
    pub content: Vec<ToolContent>,

    /// Whether the tool execution resulted in an error.
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolCallResult {
    /// Constructs a success tool result with a single text block.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text {
                text: content.into(),
            }],
            is_error: None,
        }
    }

    /// Constructs an error tool result with an error text block.
    pub fn error(error_message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text {
                text: error_message.into(),
            }],
            is_error: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_request_serialization_and_deserialization() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let req: McpRequest = serde_json::from_str(raw).expect("parse request");
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id, Some(json!(1)));
        assert_eq!(req.params, None);
        assert!(!req.is_notification());

        let serialized = serde_json::to_string(&req).expect("serialize request");
        assert!(serialized.contains("\"method\":\"tools/list\""));
    }

    #[test]
    fn test_mcp_notification_detection() {
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/cancelled"}"#;
        let req: McpRequest = serde_json::from_str(raw).expect("parse notification");
        assert!(req.is_notification());
        assert_eq!(req.id, None);
    }

    #[test]
    fn test_mcp_response_success_round_trip() {
        let res = McpResponse::success(Some(json!(42)), json!({"status": "ok"}));
        let serialized = serde_json::to_string(&res).expect("serialize");
        assert!(serialized.contains("\"result\":{\"status\":\"ok\"}"));
        assert!(!serialized.contains("\"error\""));

        let deserialized: McpResponse = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized.id, Some(json!(42)));
        assert!(deserialized.error.is_none());
    }

    #[test]
    fn test_mcp_response_error_round_trip() {
        let err = McpError::method_not_found("No such method 'foo'");
        let res = McpResponse::error(Some(json!("req-1")), err);
        let serialized = serde_json::to_string(&res).expect("serialize");
        assert!(serialized.contains("\"code\":-32601"));
        assert!(!serialized.contains("\"result\""));

        let deserialized: McpResponse = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized.error.unwrap().code, METHOD_NOT_FOUND);
    }

    #[test]
    fn test_tool_description_schema_serialization() {
        let schema = json!({
            "type": "object",
            "properties": {
                "actor": {"type": "string"}
            },
            "required": ["actor"]
        });
        let tool = ToolDescription::new("knox_start_session", "Starts session", schema);
        let serialized = serde_json::to_string(&tool).expect("serialize tool");
        assert!(serialized.contains("\"inputSchema\":{"));
        assert!(serialized.contains("\"name\":\"knox_start_session\""));
    }

    #[test]
    fn test_tool_call_result_constructors() {
        let success = ToolCallResult::text("all good");
        assert_eq!(success.is_error, None);
        assert_eq!(success.content.len(), 1);

        let err = ToolCallResult::error("failed operation");
        assert_eq!(err.is_error, Some(true));
    }
}
