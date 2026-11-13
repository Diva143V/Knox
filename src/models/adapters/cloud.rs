//! Remote cloud model adapter (OpenAI-compatible chat and embedding API).

use serde_json::{Value, json};
use std::time::Duration;

use super::super::traits::{
    AnsweringModel, AskRequest, AskResponse, Citation, ContradictionEvaluation, ContradictionModel,
    EmbeddingModel, ExtractedActionProposal, ExtractionModel, RerankModel, ScoredAction,
};
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::KnoxAction;

/// Adapter for standard OpenAI-compatible cloud endpoints.
#[derive(Debug, Clone)]
pub struct CloudAdapter {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub embedding_model: String,
    pub chat_model: String,
    pub timeout: Duration,
}

impl CloudAdapter {
    pub fn new(endpoint: &str, api_key: Option<String>, embedding_model: &str, chat_model: &str) -> Self {
        let clean_endpoint = endpoint.trim_end_matches('/').to_string();
        Self {
            endpoint: clean_endpoint,
            api_key,
            embedding_model: embedding_model.to_string(),
            chat_model: chat_model.to_string(),
            timeout: Duration::from_secs(30),
        }
    }

    fn post_json(&self, path: &str, body: Value) -> KnoxResult<Value> {
        let url = format!("{}{path}", self.endpoint);
        let mut req = ureq::post(&url).timeout(self.timeout).set("Content-Type", "application/json");

        if let Some(ref key) = self.api_key {
            req = req.set("Authorization", &format!("Bearer {key}"));
        }

        let resp = req
            .send_json(body)
            .map_err(|e| KnoxError::Model(format!("Cloud API request to {url} failed: {e}")))?;

        let val: Value = resp
            .into_json()
            .map_err(|e| KnoxError::Model(format!("Invalid JSON response from {url}: {e}")))?;
        Ok(val)
    }
}

impl EmbeddingModel for CloudAdapter {
    fn embed(&self, text: &str) -> KnoxResult<Vec<f32>> {
        let payload = json!({
            "model": self.embedding_model,
            "input": text,
        });

        let res = self.post_json("/embeddings", payload)?;
        if let Some(arr) = res.pointer("/data/0/embedding").and_then(|v| v.as_array()) {
            let floats: Vec<f32> = arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect();
            return Ok(floats);
        }

        Err(KnoxError::Model(format!(
            "Failed to retrieve vector embedding from cloud model '{}'",
            self.embedding_model
        )))
    }

    fn embed_batch(&self, texts: &[String]) -> KnoxResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let payload = json!({
            "model": self.embedding_model,
            "input": texts,
        });

        let res = self.post_json("/embeddings", payload)?;
        if let Some(arr) = res.get("data").and_then(|v| v.as_array()) {
            let mut results = vec![Vec::new(); texts.len()];
            for item in arr {
                let idx = item.get("index").and_then(|i| i.as_u64()).map(|i| i as usize);
                let emb = item.get("embedding").and_then(|e| e.as_array()).map(|a| {
                    a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect::<Vec<f32>>()
                });
                if let (Some(idx), Some(emb)) = (idx, emb) {
                    if idx < results.len() {
                        results[idx] = emb;
                    }
                }
            }
            if results.iter().all(|r| !r.is_empty()) {
                return Ok(results);
            }
        }

        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimension(&self) -> usize {
        1536 // Standard for text-embedding-3-small
    }

    fn model_name(&self) -> &str {
        &self.embedding_model
    }
}

impl RerankModel for CloudAdapter {
    fn rerank(&self, query: &str, candidates: &[KnoxAction]) -> KnoxResult<Vec<ScoredAction>> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let q_vec = self.embed(query)?;

        let summaries: Vec<String> = candidates.iter().map(|c| c.summary.clone()).collect();
        let cand_vecs = self.embed_batch(&summaries)?;

        let mut scored = Vec::with_capacity(candidates.len());
        for (candidate, cand_vec) in candidates.iter().zip(cand_vecs.into_iter()) {
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

impl AnsweringModel for CloudAdapter {
    fn answer(&self, request: &AskRequest, context: &[KnoxAction]) -> KnoxResult<AskResponse> {
        let mut context_summary = String::new();
        let mut citations = Vec::new();

        for action in context {
            context_summary.push_str(&format!(
                "- Action ID: {}\n  Actor: {}\n  Type: {}\n  Summary: {}\n  Rationale: {:?}\n\n",
                action.id, action.actor, action.action_type, action.summary, action.rationale
            ));
            citations.push(Citation {
                action_id: action.id.clone(),
                summary: action.summary.clone(),
                file: action.files.first().cloned(),
            });
        }

        let system_prompt = "You are an expert software architect with access to project memory.\n\
             Answer the user's question using ONLY the provided Knox knowledge graph actions.\n\
             Cite every claim using the format [ref:ACTION_ID].";

        let user_prompt = format!("Context:\n{context_summary}\n\nQuestion: {}", request.question);

        let payload = json!({
            "model": self.chat_model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ],
            "temperature": 0.2,
        });

        let res = self.post_json("/chat/completions", payload)?;
        let answer = res
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .unwrap_or("No response generated.")
            .to_string();

        Ok(AskResponse {
            answer,
            citations,
            confidence: 0.95,
            grounded: true,
            model: self.chat_model.clone(),
        })
    }

    fn model_name(&self) -> &str {
        &self.chat_model
    }
}

impl ExtractionModel for CloudAdapter {
    fn extract_proposals(&self, raw_transcript: &str) -> KnoxResult<Vec<ExtractedActionProposal>> {
        let system_prompt = "Extract key architectural decisions and code implementation changes from this transcript as a JSON array.\n\
             Each element must have: action_type ('decide' or 'write'), summary, rationale, and confidence (0.0 to 1.0).";

        let payload = json!({
            "model": self.chat_model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": raw_transcript }
            ],
            "response_format": { "type": "json_object" }
        });

        let res = self.post_json("/chat/completions", payload)?;
        let content = res
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .unwrap_or("[]");

        let val: Value = serde_json::from_str(content).unwrap_or_default();
        let list = val.get("proposals").or(val.get("actions")).unwrap_or(&val);
        let proposals: Vec<ExtractedActionProposal> =
            serde_json::from_value(list.clone()).unwrap_or_default();
        Ok(proposals)
    }

    fn model_name(&self) -> &str {
        &self.chat_model
    }
}

impl ContradictionModel for CloudAdapter {
    fn check_contradiction(
        &self,
        proposed_change: &str,
        established_action: &KnoxAction,
    ) -> KnoxResult<ContradictionEvaluation> {
        let system_prompt = "Evaluate whether the proposed change contradicts the established architectural decision.\n\
             Return a JSON object with fields: contradicts (bool), explanation (string), confidence (float).";

        let user_prompt = format!(
            "Established Decision: {}\nProposed Change: {}",
            established_action.summary, proposed_change
        );

        let payload = json!({
            "model": self.chat_model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ],
            "response_format": { "type": "json_object" }
        });

        let res = self.post_json("/chat/completions", payload)?;
        let content = res
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");

        let val: Value = serde_json::from_str(content).unwrap_or_default();
        let contradicts = val.get("contradicts").and_then(|v| v.as_bool()).unwrap_or(false);
        let explanation = val
            .get("explanation")
            .and_then(|v| v.as_str())
            .unwrap_or("Evaluated by cloud model")
            .to_string();
        let confidence = val.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.9);

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
