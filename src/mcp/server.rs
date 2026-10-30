//! Model Context Protocol (MCP) stdio server implementation.
//!
//! Provides the stdio event loop that reads line-delimited JSON-RPC requests,
//! dispatches them to tool handlers, writes responses, and manages background timeouts.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use crate::core::error::KnoxResult;
use crate::mcp::handlers::dispatch_request;
use crate::mcp::protocol::{McpError, McpRequest, McpResponse};
use crate::mcp::session_manager::SessionManager;
use crate::storage::db::KnoxDb;

/// Default background interval for checking stale sessions (60 seconds).
pub const DEFAULT_TIMEOUT_CHECK_INTERVAL: Duration = Duration::from_secs(60);

/// Default session idle expiration limit (30 minutes).
pub const DEFAULT_SESSION_MAX_IDLE: Duration = Duration::from_secs(1800);

/// Starts the MCP server on standard input/output with the given database handle.
pub async fn run_mcp_server(db: KnoxDb) -> KnoxResult<()> {
    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let stdout = tokio::io::stdout();
    run_mcp_server_loop(
        db,
        stdin,
        stdout,
        DEFAULT_TIMEOUT_CHECK_INTERVAL,
        DEFAULT_SESSION_MAX_IDLE,
    )
    .await
}

/// Generic MCP server event loop operating over arbitrary asynchronous reader and writer streams.
pub async fn run_mcp_server_loop<R, W>(
    db: KnoxDb,
    reader: R,
    mut writer: W,
    check_interval: Duration,
    max_idle: Duration,
) -> KnoxResult<()>
where
    R: AsyncBufRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let shared_db = Arc::new(Mutex::new(db));
    let session_manager = Arc::new(SessionManager::new());

    let timeout_handle = SessionManager::spawn_timeout_task(
        session_manager.clone(),
        shared_db.clone(),
        check_interval,
        max_idle,
    );

    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(resp) = handle_line(trimmed, &shared_db) {
            send_response(&mut writer, &resp).await?;
        }
    }

    timeout_handle.abort();
    Ok(())
}

/// Parses a line into an `McpRequest` and produces an `McpResponse` if applicable.
fn handle_line(line: &str, db: &Arc<Mutex<KnoxDb>>) -> Option<McpResponse> {
    let req_result: Result<McpRequest, _> = serde_json::from_str(line);

    match req_result {
        Ok(req) => {
            // Notifications starting with "notifications/" do not return responses
            if req.is_notification() && req.method.starts_with("notifications/") {
                tracing::debug!(method = %req.method, "Received MCP client notification");
                return None;
            }

            tracing::debug!(method = %req.method, id = ?req.id, "Processing MCP request");
            let db_guard = match db.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            Some(dispatch_request(&req, &db_guard))
        }
        Err(err) => Some(McpResponse::error(
            None,
            McpError::parse_error(format!("Malformed JSON-RPC request: {err}")),
        )),
    }
}

/// Formats and transmits an `McpResponse` over the asynchronous output stream.
async fn send_response<W>(writer: &mut W, response: &McpResponse) -> KnoxResult<()>
where
    W: AsyncWrite + Unpin,
{
    let mut payload = serde_json::to_string(response)?;
    payload.push('\n');
    writer.write_all(payload.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::duplex;

    #[tokio::test]
    async fn test_run_mcp_server_loop_initialization_and_ping() {
        let db = KnoxDb::open_memory().unwrap();
        let (client_read, server_write) = duplex(4096);
        let (server_read_pipe, mut client_write) = duplex(4096);

        let server_task = tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(server_read_pipe);
            run_mcp_server_loop(
                db,
                reader,
                server_write,
                Duration::from_millis(50),
                Duration::from_secs(1800),
            )
            .await
        });

        let mut client_lines = tokio::io::BufReader::new(client_read).lines();

        // Send initialize
        let init_req = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}});
        client_write
            .write_all(format!("{init_req}\n").as_bytes())
            .await
            .unwrap();
        client_write.flush().await.unwrap();
        let resp: serde_json::Value =
            serde_json::from_str(&client_lines.next_line().await.unwrap().unwrap()).unwrap();
        assert_eq!(resp.get("id").unwrap(), 1);

        // Send ping
        let ping_req = json!({"jsonrpc": "2.0", "id": 2, "method": "ping"});
        client_write
            .write_all(format!("{ping_req}\n").as_bytes())
            .await
            .unwrap();
        client_write.flush().await.unwrap();
        let ping_resp: serde_json::Value =
            serde_json::from_str(&client_lines.next_line().await.unwrap().unwrap()).unwrap();
        assert_eq!(ping_resp.get("id").unwrap(), 2);

        drop(client_write);
        assert!(server_task.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_run_mcp_server_loop_malformed_json_returns_parse_error() {
        let db = KnoxDb::open_memory().unwrap();
        let (client_read, server_write) = duplex(4096);
        let (server_read_pipe, mut client_write) = duplex(4096);

        let server_task = tokio::spawn(async move {
            let reader = tokio::io::BufReader::new(server_read_pipe);
            run_mcp_server_loop(
                db,
                reader,
                server_write,
                Duration::from_millis(50),
                Duration::from_secs(1800),
            )
            .await
        });

        let mut client_lines = tokio::io::BufReader::new(client_read).lines();

        // Send malformed line
        client_write
            .write_all(b"not a json string\n")
            .await
            .unwrap();
        client_write.flush().await.unwrap();

        let line = client_lines.next_line().await.unwrap().unwrap();
        let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
        let err = resp.get("error").unwrap();
        assert_eq!(err.get("code").unwrap(), -32700);

        drop(client_write);
        assert!(server_task.await.unwrap().is_ok());
    }
}
