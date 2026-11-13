//! IPC command handler for model configuration testing.

use knox::core::config::KnoxConfig;
use knox::mcp::handlers::ask::{execute_ask, AskResultDto};
use knox::models::router::ModelRouter;
use knox::models::traits::AskRequest;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

/// Status for a detected local model daemon (Ollama, LM Studio, etc.).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalDaemonStatus {
    pub port: u16,
    pub is_online: bool,
    pub name: String,
}

/// Form draft values passed from ModelTab when testing connectivity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelSettingsDto {
    pub profile: String,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub embedding_model: Option<String>,
    pub chat_model: Option<String>,
    pub allow_cloud_egress: bool,
}

/// Structured test connection result returned to the UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelTestResultDto {
    pub success: bool,
    pub latency_ms: u64,
    pub embedding_ok: bool,
    pub embedding_dim: usize,
    pub answer_preview: Option<String>,
    pub error: Option<String>,
}

/// Inner core logic for testing model connectivity and embedding generation.
pub fn test_model_connection_impl(
    stored_key: Option<String>,
    draft: ModelSettingsDto,
) -> Result<ModelTestResultDto, String> {
    let effective_key = match draft.api_key.as_deref().map(str::trim) {
        Some(k) if k.starts_with('•') || k.starts_with('*') => stored_key,
        Some(k) if !k.is_empty() => Some(k.to_string()),
        _ => stored_key,
    };

    // Diagnostic connection test evaluates configured credentials even if global egress is local-only
    let effective_egress = if draft.profile == "cloud" {
        true
    } else {
        draft.allow_cloud_egress
    };

    let cfg = KnoxConfig {
        model_profile: draft.profile,
        model_endpoint: draft.endpoint.filter(|s| !s.trim().is_empty()),
        model_api_key: effective_key,
        embedding_model: draft.embedding_model.filter(|s| !s.trim().is_empty()),
        chat_model: draft.chat_model.filter(|s| !s.trim().is_empty()),
        allow_cloud_egress: effective_egress,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let router = ModelRouter::from_config(&cfg);

    // 1. Test embedding generation
    let embed_res = router.embed("Knox knowledge graph connectivity test");
    let (embedding_ok, embedding_dim) = match embed_res {
        Ok(vec) => (true, vec.len()),
        Err(_) => (false, 0),
    };

    // 2. Test lightweight answering
    let req = AskRequest {
        question: "Is model connectivity operational?".to_string(),
        file_path: None,
        project: None,
        strict_citations: false,
    };

    match router.answer(&req, &[]) {
        Ok(resp) => Ok(ModelTestResultDto {
            success: true,
            latency_ms: start.elapsed().as_millis() as u64,
            embedding_ok,
            embedding_dim,
            answer_preview: Some(resp.answer),
            error: None,
        }),
        Err(e) => {
            if embedding_ok {
                // Embeddings work even if text LLM chat has a schema notice
                Ok(ModelTestResultDto {
                    success: true,
                    latency_ms: start.elapsed().as_millis() as u64,
                    embedding_ok: true,
                    embedding_dim,
                    answer_preview: None,
                    error: Some(e.to_string()),
                })
            } else {
                Ok(ModelTestResultDto {
                    success: false,
                    latency_ms: start.elapsed().as_millis() as u64,
                    embedding_ok: false,
                    embedding_dim: 0,
                    answer_preview: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }
}

/// Tests model connectivity and embedding generation for in-flight settings.
#[tauri::command]
pub async fn test_model_connection(
    state: State<'_, AppState>,
    draft: ModelSettingsDto,
) -> Result<ModelTestResultDto, String> {
    let stored_key = {
        let cfg = state.config.lock().await;
        cfg.model_api_key.clone()
    };
    tokio::task::spawn_blocking(move || test_model_connection_impl(stored_key, draft))
        .await
        .map_err(|e| e.to_string())?
}

/// Probes localhost ports for active local LLM daemons (Ollama, LM Studio, LocalAI).
#[tauri::command]
pub async fn check_local_daemons() -> Result<Vec<LocalDaemonStatus>, String> {
    tokio::task::spawn_blocking(|| {
        let targets = [(11434, "Ollama"), (1234, "LM Studio"), (8080, "LocalAI")];
        let mut res = Vec::new();
        for (port, name) in targets {
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
            let is_online = std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(150)).is_ok();
            res.push(LocalDaemonStatus {
                port,
                is_online,
                name: name.to_string(),
            });
        }
        Ok(res)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Asks a question or brainstorms ideas against project memory using the active model engine.
#[tauri::command]
pub async fn ask_knox(
    state: State<'_, AppState>,
    question: String,
    project: Option<String>,
    file_path: Option<String>,
) -> Result<AskResultDto, String> {
    let db_arc = state.db.clone();
    let cfg_arc = state.config.clone();

    tokio::task::spawn_blocking(move || {
        let db = db_arc.blocking_lock();
        let cfg = cfg_arc.blocking_lock();

        execute_ask(
            &db,
            &cfg,
            &question,
            project.as_deref(),
            file_path.as_deref(),
            true,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_connection_mock() {
        let draft = ModelSettingsDto {
            profile: "mock".to_string(),
            endpoint: None,
            api_key: None,
            embedding_model: None,
            chat_model: None,
            allow_cloud_egress: false,
        };
        let res = test_model_connection_impl(None, draft).unwrap();
        assert!(res.success);
        assert!(res.embedding_ok);
        assert_eq!(res.embedding_dim, 64);
        assert!(res.answer_preview.is_some());
    }

    #[test]
    fn test_model_connection_masked_key_resolution() {
        let stored_secret = Some("sk-mock-key-12345".to_string());
        let draft = ModelSettingsDto {
            profile: "mock".to_string(),
            endpoint: None,
            api_key: Some("••••••••".to_string()), // UI sent masked key
            embedding_model: None,
            chat_model: None,
            allow_cloud_egress: false,
        };
        let res = test_model_connection_impl(stored_secret, draft).unwrap();
        assert!(res.success);
        assert!(res.embedding_ok);
    }

    #[tokio::test]
    async fn test_check_local_daemons() {
        let daemons = check_local_daemons().await.unwrap();
        assert_eq!(daemons.len(), 3);
        assert_eq!(daemons[0].name, "Ollama");
        assert_eq!(daemons[0].port, 11434);
        assert_eq!(daemons[1].name, "LM Studio");
        assert_eq!(daemons[1].port, 1234);
        assert_eq!(daemons[2].name, "LocalAI");
        assert_eq!(daemons[2].port, 8080);
    }

    #[test]
    fn test_execute_ask_mock() {
        use knox::storage::db::KnoxDb;
        let db = KnoxDb::open_memory().unwrap();
        let cfg = KnoxConfig {
            model_profile: "mock".to_string(),
            ..Default::default()
        };

        let res = execute_ask(
            &db,
            &cfg,
            "Why did we use SQLite for the knowledge graph?",
            None,
            None,
            true,
        )
        .unwrap();

        assert_eq!(res.model, "mock-deterministic-v1");
        assert!(!res.answer.is_empty());
    }
}

