//! IPC command handler for model configuration testing.

use knox::core::config::KnoxConfig;
use knox::models::router::ModelRouter;
use knox::models::traits::AskRequest;
use serde::{Deserialize, Serialize};

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

/// Tests model connectivity and embedding generation for in-flight settings.
#[tauri::command]
pub async fn test_model_connection(draft: ModelSettingsDto) -> Result<ModelTestResultDto, String> {
    tokio::task::spawn_blocking(move || {
        let cfg = KnoxConfig {
            model_profile: draft.profile,
            model_endpoint: draft.endpoint.filter(|s| !s.trim().is_empty()),
            model_api_key: draft.api_key.filter(|s| !s.trim().is_empty()),
            embedding_model: draft.embedding_model.filter(|s| !s.trim().is_empty()),
            chat_model: draft.chat_model.filter(|s| !s.trim().is_empty()),
            allow_cloud_egress: draft.allow_cloud_egress,
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
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_model_connection_ipc() {
        let draft = ModelSettingsDto {
            profile: "mock".to_string(),
            endpoint: None,
            api_key: None,
            embedding_model: None,
            chat_model: None,
            allow_cloud_egress: false,
        };
        let res = test_model_connection(draft).await.unwrap();
        assert!(res.success);
        assert!(res.embedding_ok);
        assert_eq!(res.embedding_dim, 64);
        assert!(res.answer_preview.is_some());
    }
}

