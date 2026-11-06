//! Deterministic in-memory mock model implementations for testing and evaluation (Workstream H).
//!
//! Provides GPU-free, network-free implementations of all five model traits:
//! - Deterministic pseudo-embeddings with unit normalization.
//! - Overlap-based candidate reranking.
//! - Strictly-grounded extractive answering citing `[ref:ACT-ID]`.
//! - Pattern-based proposal extraction from transcripts.
//! - Rule-based contradiction detection.

use std::collections::HashSet;

use super::traits::{
    AnsweringModel, AskRequest, AskResponse, Citation, ContradictionEvaluation, ContradictionModel,
    EmbeddingModel, ExtractedActionProposal, ExtractionModel, RerankModel, ScoredAction,
};
use crate::core::error::KnoxResult;
use crate::core::types::KnoxAction;

const MOCK_DIMENSION: usize = 64;

/// Deterministic mock backend implementing all 5 traits.
#[derive(Debug, Default, Clone)]
pub struct MockModelBackend {
    pub dimension: usize,
    pub model_name: String,
}

impl MockModelBackend {
    pub fn new() -> Self {
        Self {
            dimension: MOCK_DIMENSION,
            model_name: "mock-deterministic-v1".to_string(),
        }
    }
}

impl EmbeddingModel for MockModelBackend {
    fn embed(&self, text: &str) -> KnoxResult<Vec<f32>> {
        let dim = if self.dimension == 0 { MOCK_DIMENSION } else { self.dimension };
        let mut vector = vec![0.0f32; dim];

        let tokens: Vec<&str> = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();

        if tokens.is_empty() {
            return Ok(vector);
        }

        for token in tokens {
            let lower = token.to_ascii_lowercase();
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for byte in lower.as_bytes() {
                h ^= u64::from(*byte);
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
            let idx = (h as usize) % dim;
            let sign = if (h >> 32) & 1 == 0 { 1.0f32 } else { -1.0f32 };
            vector[idx] += sign;
        }

        // Normalize vector to unit length
        let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        }

        Ok(vector)
    }

    fn dimension(&self) -> usize {
        if self.dimension == 0 { MOCK_DIMENSION } else { self.dimension }
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

impl RerankModel for MockModelBackend {
    fn rerank(&self, query: &str, candidates: &[KnoxAction]) -> KnoxResult<Vec<ScoredAction>> {
        let q_tokens: HashSet<String> = query
            .to_ascii_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .map(str::to_string)
            .collect();

        let mut scored: Vec<ScoredAction> = candidates
            .iter()
            .map(|action| {
                let mut act_tokens = HashSet::new();
                for t in action.summary.to_ascii_lowercase().split(|c: char| !c.is_alphanumeric()) {
                    if t.len() > 2 {
                        act_tokens.insert(t.to_string());
                    }
                }
                for f in &action.files {
                    for t in f.to_ascii_lowercase().split(|c: char| !c.is_alphanumeric()) {
                        if t.len() > 2 {
                            act_tokens.insert(t.to_string());
                        }
                    }
                }
                for s in &action.symbols {
                    for t in s.to_ascii_lowercase().split(|c: char| !c.is_alphanumeric()) {
                        if t.len() > 2 {
                            act_tokens.insert(t.to_string());
                        }
                    }
                }

                let intersection = q_tokens.intersection(&act_tokens).count();
                let union = q_tokens.union(&act_tokens).count();
                let score = if union == 0 {
                    0.0
                } else {
                    intersection as f64 / union as f64
                };

                ScoredAction {
                    action: action.clone(),
                    score,
                    reason: Some(format!("matched {intersection} terms")),
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| b.action.timestamp.cmp(&a.action.timestamp))
        });

        Ok(scored)
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

impl AnsweringModel for MockModelBackend {
    fn answer(&self, request: &AskRequest, context: &[KnoxAction]) -> KnoxResult<AskResponse> {
        if context.is_empty() {
            return Ok(AskResponse {
                answer: format!(
                    "No relevant architectural context or engineering decisions found in Knox for: \"{}\"",
                    request.question
                ),
                citations: Vec::new(),
                confidence: 0.0,
                grounded: false,
                model: self.model_name.clone(),
            });
        }

        let mut citations = Vec::new();
        let mut answer_lines = Vec::new();

        answer_lines.push(format!("Based on Knox project memory for query \"{}\":\n", request.question));

        for action in context.iter().take(5) {
            let file_hint = action.files.first().cloned();
            let citation = Citation {
                action_id: action.id.clone(),
                summary: action.summary.clone(),
                file: file_hint,
            };
            citations.push(citation);

            answer_lines.push(format!(
                "- **{}** (actor: {}, type: {}): {} [ref:{}]",
                action.timestamp.format("%Y-%m-%d"),
                action.actor,
                action.action_type,
                action.summary,
                action.id
            ));
            if let Some(ref r) = action.rationale {
                answer_lines.push(format!("  *Rationale*: {r}"));
            }
        }

        Ok(AskResponse {
            answer: answer_lines.join("\n"),
            citations,
            confidence: 1.0,
            grounded: true,
            model: self.model_name.clone(),
        })
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

impl ExtractionModel for MockModelBackend {
    fn extract_proposals(&self, raw_transcript: &str) -> KnoxResult<Vec<ExtractedActionProposal>> {
        let mut proposals = Vec::new();

        for line in raw_transcript.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let lower = trimmed.to_ascii_lowercase();
            if lower.contains("decided to") || lower.contains("decision:") || lower.contains("chosen to") {
                proposals.push(ExtractedActionProposal {
                    action_type: "decide".to_string(),
                    summary: trimmed.to_string(),
                    rationale: Some("Derived from transcript decision statement".to_string()),
                    files: Vec::new(),
                    symbols: Vec::new(),
                    confidence: 0.9,
                });
            } else if lower.contains("implemented") || lower.contains("created file") || lower.contains("modified") {
                proposals.push(ExtractedActionProposal {
                    action_type: "write".to_string(),
                    summary: trimmed.to_string(),
                    rationale: None,
                    files: Vec::new(),
                    symbols: Vec::new(),
                    confidence: 0.85,
                });
            }
        }

        Ok(proposals)
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

impl ContradictionModel for MockModelBackend {
    fn check_contradiction(
        &self,
        proposed_change: &str,
        established_action: &KnoxAction,
    ) -> KnoxResult<ContradictionEvaluation> {
        let prop_lower = proposed_change.to_ascii_lowercase();
        let est_lower = established_action.summary.to_ascii_lowercase();

        let negation_words = ["not", "instead", "deprecate", "replace", "revert", "abandon", "remove"];
        let has_negation = negation_words.iter().any(|w| prop_lower.contains(w));

        let est_words: Vec<&str> = est_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .collect();

        let overlaps = est_words.iter().filter(|w| prop_lower.contains(*w)).count();

        if has_negation && overlaps >= 1 {
            Ok(ContradictionEvaluation {
                contradicts: true,
                established_id: established_action.id.clone(),
                explanation: format!(
                    "Proposed change appears to alter established decision '{}' (overlapping keywords with negation detected)",
                    established_action.summary
                ),
                confidence: 0.85,
            })
        } else {
            Ok(ContradictionEvaluation {
                contradicts: false,
                established_id: established_action.id.clone(),
                explanation: "No architectural contradiction identified.".to_string(),
                confidence: 0.9,
            })
        }
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

