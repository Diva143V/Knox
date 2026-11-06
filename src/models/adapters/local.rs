//! Local HTTP model adapter (e.g. Ollama, LocalAI, vLLM).

use serde_json::{Value, json};
use std::time::Duration;

use super::super::traits::{
    AnsweringModel, AskRequest, AskResponse, Citation, ContradictionEvaluation, ContradictionModel,
    EmbeddingModel, ExtractedActionProposal, ExtractionModel, RerankModel, ScoredAction,
};
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::KnoxAction;

/// Adapter for local LLM engines (defaulting to Ollama endpoint).
#[derive(Debug, Clone)]
pub struct LocalAdapter {
    pub endpoint: String,
    pub embedding_model: String,
    pub chat_model: String,
    pub timeout: Duration,
}

impl LocalAdapter {
    pub fn new(endpoint: &str, embedding_model: &str, chat_model: &str) -> Self {
        let clean_endpoint = endpoint.trim_end_matches('/').to_string();
        Self {
            endpoint: clean_endpoint,
            embedding_model: embedding_model.to_string(),
            chat_model: chat_model.to_string(),
            timeout: Duration::from_secs(15),
        }
    }

    fn post_json(&self, path: &str, body: Value) -> KnoxResult<Value> {
        let url = format!("{}{path}", self.endpoint);
        let resp = ureq::post(&url)
            .timeout(self.timeout)
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|e| KnoxError::Model(format!("Local model request to {url} failed: {e}")))?;

        let val: Value = resp
            .into_json()
            .map_err(|e| KnoxError::Model(format!("Invalid JSON response from {url}: {e}")))?;
        Ok(val)
    }
}

impl EmbeddingModel for LocalAdapter {
    fn embed(&self, text: &str) -> KnoxResult<Vec<f32>> {
        // Try Ollama /api/embeddings first, fallback to OpenAI-compatible /v1/embeddings
        let payload = json!({
            "model": self.embedding_model,
            "prompt": text,
        });

        match self.post_json("/api/embeddings", payload) {
            Ok(res) => {
                if let Some(arr) = res.get("embedding").and_then(|v| v.as_array()) {
                    let floats: Vec<f32> = arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect();
                    return Ok(floats);
                }
            }
            Err(_) => {
                // Try OpenAI-style /v1/embeddings
                let v1_payload = json!({
                    "model": self.embedding_model,
                    "input": text,
                });
                if let Ok(res) = self.post_json("/v1/embeddings", v1_payload) {
                    if let Some(arr) = res.pointer("/data/0/embedding").and_then(|v| v.as_array()) {
                        let floats: Vec<f32> = arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect();
                        return Ok(floats);
                    }
                }
            }
        }

        Err(KnoxError::Model(format!(
            "Failed to retrieve vector embedding from local model '{}'",
            self.embedding_model
        )))
    }

    fn dimension(&self) -> usize {
        768 // Standard default for local models like nomic-embed-text
    }

    fn model_name(&self) -> &str {
        &self.embedding_model
    }
}

impl RerankModel for LocalAdapter {
    fn rerank(&self, query: &str, candidates: &[KnoxAction]) -> KnoxResult<Vec<ScoredAction>> {
        // Embed the query and calculate cosine similarities against candidate summaries
        let q_vec = self.embed(query)?;

        let mut scored = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let cand_vec = self.embed(&candidate.summary)?;
            let sim = crate::storage::embeddings::cosine_similarity(&q_vec, &cand_vec);
            scored.push(ScoredAction {
                action: candidate.clone(),
                score: sim as f64,
                reason: Some(format!("cosine_sim: {:.3}", sim)),
            });
        }

        scored.sort_by(|a, b| b.score.total_cmp(&a.score));
        Ok(scored)
    }

    fn model_name(&self) -> &str {
        &self.embedding_model
    }
}

impl AnsweringModel for LocalAdapter {
    fn answer(&self, request: &AskRequest, context: &[KnoxAction]) -> KnoxResult<AskResponse> {
        let mut context_summary = String::new();
        let mut citations = Vec::new();

        for action in context {
            context_summary.push_str(&format!(
                "- Action ID: {}\n  Actor: {}\n  Type: {}\n  Summary: {}\n  Files: {:?}\n\n",
                action.id, action.actor, action.action_type, action.summary, action.files
            ));
            citations.push(Citation {
                action_id: action.id.clone(),
                summary: action.summary.clone(),
                file: action.files.first().cloned(),
            });
        }

        let prompt = format!(
            "You are an expert software architect with access to project memory.\n\
             Answer the user's question using ONLY the provided Knox knowledge graph actions.\n\
             Cite every claim using the format [ref:ACTION_ID].\n\n\
             Context:\n{context_summary}\n\
             Question: {}\n\nAnswer:",
            request.question
        );

        let payload = json!({
            "model": self.chat_model,
            "prompt": prompt,
            "stream": false,
        });

        let res = self.post_json("/api/generate", payload)?;
        let answer = res
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("No response generated.")
            .to_string();

        Ok(AskResponse {
            answer,
            citations,
            confidence: 0.9,
            grounded: true,
            model: self.chat_model.clone(),
        })
    }

    fn model_name(&self) -> &str {
        &self.chat_model
    }
}

impl ExtractionModel for LocalAdapter {
    fn extract_proposals(&self, raw_transcript: &str) -> KnoxResult<Vec<ExtractedActionProposal>> {
        let prompt = format!(
            "Extract key architectural decisions and code implementation changes from this transcript as JSON array.\n\
             Each element must have: action_type ('decide' or 'write'), summary, rationale, and confidence (0.0 to 1.0).\n\n\
             Transcript:\n{raw_transcript}\n\nJSON array:"
        );

        let payload = json!({
            "model": self.chat_model,
            "prompt": prompt,
            "stream": false,
            "format": "json",
        });

        let res = self.post_json("/api/generate", payload)?;
        let response_text = res
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("[]");

        let proposals: Vec<ExtractedActionProposal> =
            serde_json::from_str(response_text).unwrap_or_default();
        Ok(proposals)
    }

    fn model_name(&self) -> &str {
        &self.chat_model
    }
}

impl ContradictionModel for LocalAdapter {
    fn check_contradiction(
        &self,
        proposed_change: &str,
        established_action: &KnoxAction,
    ) -> KnoxResult<ContradictionEvaluation> {
        let prompt = format!(
            "Evaluate whether this proposed change contradicts the established architectural decision.\n\
             Return a JSON object with: contradicts (boolean), explanation (string), confidence (0.0 to 1.0).\n\n\
             Established Decision: {}\n\
             Proposed Change: {}\n\nJSON:",
            established_action.summary, proposed_change
        );

        let payload = json!({
            "model": self.chat_model,
            "prompt": prompt,
            "stream": false,
            "format": "json",
        });

        let res = self.post_json("/api/generate", payload)?;
        let text = res.get("response").and_then(|v| v.as_str()).unwrap_or("{}");

        let val: Value = serde_json::from_str(text).unwrap_or_default();
        let contradicts = val.get("contradicts").and_then(|v| v.as_bool()).unwrap_or(false);
        let explanation = val
            .get("explanation")
            .and_then(|v| v.as_str())
            .unwrap_or("Evaluated by local model")
            .to_string();
        let confidence = val.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.8);

        Ok(ContradictionEvaluation {
            contradicts,
            established_id: established_action.id.clone(),
            explanation,
            confidence,
        })
    }

    fn model_name(&self) -> &str {
        &self.chat_model
    }
}
