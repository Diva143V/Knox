//! Dynamic model router with fallback and clean-machine guarantees (Workstream H).
//!
//! When `profile` is `Off`, the router requires zero external dependencies and returns
//! deterministic fallbacks, guaranteeing 100% byte-identical behavior to Knox v2.1.

use std::sync::Arc;

use super::adapters::{CloudAdapter, LocalAdapter};
use super::mock::MockModelBackend;
use super::profile::{ModelProfile, ModelProfileKind};
use super::traits::{
    AnsweringModel, AskRequest, AskResponse, ContradictionEvaluation, ContradictionModel,
    EmbeddingModel, ExtractedActionProposal, ExtractionModel, RerankModel, ScoredAction,
};
use crate::core::config::KnoxConfig;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::KnoxAction;

/// Central router dispatching model tasks to the configured profile backend.
#[derive(Clone)]
pub struct ModelRouter {
    pub profile: ModelProfile,
    embedding: Option<Arc<dyn EmbeddingModel>>,
    rerank: Option<Arc<dyn RerankModel>>,
    answering: Option<Arc<dyn AnsweringModel>>,
    extraction: Option<Arc<dyn ExtractionModel>>,
    contradiction: Option<Arc<dyn ContradictionModel>>,
}

impl ModelRouter {
    /// Creates a router from KnoxConfig.
    pub fn from_config(config: &KnoxConfig) -> Self {
        Self::for_project(config, None, None)
    }

    /// Creates a router from KnoxConfig, resolving project-level cloud egress in SQLite if needed.
    pub fn for_project(
        config: &KnoxConfig,
        db: Option<&crate::storage::KnoxDb>,
        project_id: Option<&str>,
    ) -> Self {
        let mut resolved_config = config.clone();
        if !resolved_config.allow_cloud_egress {
            if let (Some(db), Some(pid)) = (db, project_id) {
                if let Ok(allowed) = crate::storage::is_project_egress_allowed(db, pid) {
                    if allowed {
                        resolved_config.allow_cloud_egress = true;
                    }
                }
            }
        }
        let profile = ModelProfile::from_config(&resolved_config);
        Self::from_profile(profile)
    }

    /// Verifies that egress is permitted; returns KnoxError::Model if cloud profile lacks authorization.
    pub fn check_egress(&self) -> KnoxResult<()> {
        if self.profile.kind == ModelProfileKind::Cloud && !self.profile.allow_cloud_egress {
            Err(KnoxError::Model(
                "Cloud model requests blocked: project has not enabled egress and KNOX_ALLOW_CLOUD_EGRESS is not set".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Creates a router from a resolved ModelProfile.
    pub fn from_profile(profile: ModelProfile) -> Self {
        match profile.kind {
            ModelProfileKind::Off => Self {
                profile,
                embedding: None,
                rerank: None,
                answering: None,
                extraction: None,
                contradiction: None,
            },
            ModelProfileKind::Mock => {
                let backend = Arc::new(MockModelBackend::new());
                Self {
                    profile,
                    embedding: Some(backend.clone()),
                    rerank: Some(backend.clone()),
                    answering: Some(backend.clone()),
                    extraction: Some(backend.clone()),
                    contradiction: Some(backend),
                }
            }
            ModelProfileKind::Local => {
                let adapter = Arc::new(LocalAdapter::new(
                    &profile.endpoint,
                    &profile.embedding_model,
                    &profile.chat_model,
                ));
                Self {
                    profile,
                    embedding: Some(adapter.clone()),
                    rerank: Some(adapter.clone()),
                    answering: Some(adapter.clone()),
                    extraction: Some(adapter.clone()),
                    contradiction: Some(adapter),
                }
            }
            ModelProfileKind::Cloud => {
                if !profile.allow_cloud_egress {
                    tracing::warn!(
                        endpoint = %profile.endpoint,
                        "Cloud model profile requires explicit egress opt-in (KNOX_ALLOW_CLOUD_EGRESS=true); refusing to load outbound adapters"
                    );
                    Self {
                        profile,
                        embedding: None,
                        rerank: None,
                        answering: None,
                        extraction: None,
                        contradiction: None,
                    }
                } else {
                    let adapter = Arc::new(CloudAdapter::new(
                        &profile.endpoint,
                        profile.api_key.clone(),
                        &profile.embedding_model,
                        &profile.chat_model,
                    ));
                    Self {
                        profile,
                        embedding: Some(adapter.clone()),
                        rerank: Some(adapter.clone()),
                        answering: Some(adapter.clone()),
                        extraction: Some(adapter.clone()),
                        contradiction: Some(adapter),
                    }
                }
            }
        }
    }

    /// True if an active model backend is enabled and authorized.
    pub fn is_enabled(&self) -> bool {
        self.profile.kind.is_enabled() && self.embedding.is_some()
    }

    /// Returns the active embedding model, if enabled.
    pub fn embedding(&self) -> Option<Arc<dyn EmbeddingModel>> {
        self.embedding.clone()
    }

    /// Returns the active rerank model, if enabled.
    pub fn rerank_model(&self) -> Option<Arc<dyn RerankModel>> {
        self.rerank.clone()
    }

    /// Returns the active answering model, if enabled.
    pub fn answering(&self) -> Option<Arc<dyn AnsweringModel>> {
        self.answering.clone()
    }

    /// Returns the active extraction model, if enabled.
    pub fn extraction(&self) -> Option<Arc<dyn ExtractionModel>> {
        self.extraction.clone()
    }

    /// Returns the active contradiction model, if enabled.
    pub fn contradiction(&self) -> Option<Arc<dyn ContradictionModel>> {
        self.contradiction.clone()
    }

    /// Embeds text with fallback: fails gracefully if models are disabled or egress is blocked.
    pub fn embed(&self, text: &str) -> KnoxResult<Vec<f32>> {
        match &self.embedding {
            Some(m) => m.embed(text),
            None => {
                if self.profile.kind == ModelProfileKind::Cloud && !self.profile.allow_cloud_egress {
                    Err(KnoxError::Model(
                        "Cloud model requests blocked: project has not enabled egress and KNOX_ALLOW_CLOUD_EGRESS is not set".to_string(),
                    ))
                } else {
                    Err(KnoxError::Model(
                        "Model profile is 'off'. Enable models via KNOX_MODEL_PROFILE or config to use embeddings.".to_string(),
                    ))
                }
            }
        }
    }

    /// Re-ranks candidates; falls back to unchanged order if models are disabled or model call fails.
    pub fn rerank(&self, query: &str, candidates: &[KnoxAction]) -> KnoxResult<Vec<ScoredAction>> {
        match &self.rerank {
            Some(m) => match m.rerank(query, candidates) {
                Ok(scored) => Ok(scored),
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "Rerank model failed; falling back to heuristic ordering"
                    );
                    Ok(candidates
                        .iter()
                        .enumerate()
                        .map(|(idx, act)| ScoredAction {
                            action: act.clone(),
                            score: 1.0 / (idx + 1) as f64,
                            reason: Some(format!("unranked (model error: {err})")),
                        })
                        .collect())
                }
            },
            None => {
                // Heuristic fallback: return candidates in incoming order with dummy score
                Ok(candidates
                    .iter()
                    .enumerate()
                    .map(|(idx, act)| ScoredAction {
                        action: act.clone(),
                        score: 1.0 / (idx + 1) as f64,
                        reason: Some("unranked (models off)".to_string()),
                    })
                    .collect())
            }
        }
    }

    /// Answers a query; falls back to deterministic extractive summary if models are disabled or model call fails.
    pub fn answer(&self, request: &AskRequest, context: &[KnoxAction]) -> KnoxResult<AskResponse> {
        match &self.answering {
            Some(m) => match m.answer(request, context) {
                Ok(resp) => Ok(resp),
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "Active model answering failed; falling back to deterministic extractive answering"
                    );
                    let mock = MockModelBackend::new();
                    let mut resp = mock.answer(request, context)?;
                    resp.model = format!("extractive-fallback (model error: {err})");
                    Ok(resp)
                }
            },
            None => {
                // Extractive fallback when models are disabled
                let mock = MockModelBackend::new();
                let mut resp = mock.answer(request, context)?;
                resp.model = "extractive-fallback (models off)".to_string();
                Ok(resp)
            }
        }
    }

    /// Extracts action proposals from transcript; returns empty vec if models are disabled.
    pub fn extract_proposals(&self, raw_transcript: &str) -> KnoxResult<Vec<ExtractedActionProposal>> {
        match &self.extraction {
            Some(m) => m.extract_proposals(raw_transcript),
            None => Ok(Vec::new()),
        }
    }

    /// Evaluates architectural contradiction; returns non-contradicting if models are disabled.
    pub fn check_contradiction(
        &self,
        proposed_change: &str,
        established_action: &KnoxAction,
    ) -> KnoxResult<ContradictionEvaluation> {
        match &self.contradiction {
            Some(m) => m.check_contradiction(proposed_change, established_action),
            None => Ok(ContradictionEvaluation {
                contradicts: false,
                established_id: established_action.id.clone(),
                explanation: "Models are disabled; contradiction evaluation skipped.".to_string(),
                confidence: 0.0,
            }),
        }
    }
}
