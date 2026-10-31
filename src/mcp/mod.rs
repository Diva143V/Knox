//! Model Context Protocol (MCP) server implementation for Knox.

pub mod format;
pub mod handlers;
pub mod protocol;
pub mod server;
pub mod session_manager;
pub mod tools;

pub use handlers::{dispatch, dispatch_request, execute_tool};
pub use protocol::{
    CallToolParams, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, McpError,
    McpRequest, McpResponse, PARSE_ERROR, ToolCallResult, ToolContent, ToolDescription,
};
pub use server::{run_mcp_server, run_mcp_server_loop};
pub use session_manager::{SessionManager, SessionState};
pub use tools::{TOOL_NAMES, all_tools, find_tool};
