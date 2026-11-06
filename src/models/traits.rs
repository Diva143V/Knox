//! Domain traits for Knox model layer integration (Workstream H).
//!
//! Defines the five core model slots:
//! 1. `EmbeddingModel`: vector embedding generation for semantic search.
//! 2. `RerankModel`: semantic candidate reranking.
//! 3. `AnsweringModel`: grounded synthesis with verified citations for `knox_ask`.
//! 4. `ExtractionModel`: automated proposal extraction from raw transcripts.
//! 5. `ContradictionModel`: architectural contradiction and drift detection.

use serde::{Deserialize, Serialize};

use crate::core::error::KnoxResult;
use crate::core::ids::ActionId;
use crate::core::types::KnoxAction;

/// Output of a candidate reranker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredAction {
    pub action: KnoxAction,
    pub score: f64,
    pub reason: Option<String>,
}

/// Request parameters for grounded answering (`knox_ask`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskRequest {
    pub question: String,
    pub file_path: Option<String>,
    pub project: Option<String>,
    pub strict_citations: bool,
}

/// A verified citation within an answer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Citation {
    pub action_id: ActionId,
    pub summary: String,
    pub file: Option<String>,
}

/// Response returned by grounded answering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskResponse {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub confidence: f64,
    pub grounded: bool,
    pub model: String,
}

/// Proposal extracted from unstructured transcript text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedActionProposal {
    pub action_type: String,
    pub summary: String,
    pub rationale: Option<String>,
    pub files: Vec<String>,
    pub symbols: Vec<String>,
    pub confidence: f64,
}

/// Architectural contradiction evaluation against established decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionEvaluation {
    pub contradicts: bool,
    pub established_id: ActionId,
    pub explanation: String,
    pub confidence: f64,
}

/// Trait 1: Vector embedding generation.
pub trait EmbeddingModel: Send + Sync {
    /// Generates a dense float vector for the provided text.
    fn embed(&self, text: &str) -> KnoxResult<Vec<f32>>;

    /// Generates dense float vectors for a batch of texts.
    fn embed_batch(&self, texts: &[String]) -> KnoxResult<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Dimensionality of the produced vectors (e.g. 64, 384, 1536).
    fn dimension(&self) -> usize;

    /// Identifier of the active embedding model.
    fn model_name(&self) -> &str;
}

/// Trait 2: Semantic candidate reranking.
pub trait RerankModel: Send + Sync {
    /// Re-orders candidate actions by semantic relevance to the query.
    fn rerank(&self, query: &str, candidates: &[KnoxAction]) -> KnoxResult<Vec<ScoredAction>>;

    /// Identifier of the active rerank model.
    fn model_name(&self) -> &str;
}

/// Trait 3: Grounded question answering with verified citations for `knox_ask`.
pub trait AnsweringModel: Send + Sync {
    /// Generates a grounded response to a query using retrieved KnoxActions as evidence.
    fn answer(&self, request: &AskRequest, context: &[KnoxAction]) -> KnoxResult<AskResponse>;

    /// Identifier of the active answering model.
    fn model_name(&self) -> &str;
}

/// Trait 4: Transcript action proposal extraction.
pub trait ExtractionModel: Send + Sync {
    /// Extracts structured decision/write action proposals from raw conversation text.
    fn extract_proposals(&self, raw_transcript: &str) -> KnoxResult<Vec<ExtractedActionProposal>>;

    /// Identifier of the active extraction model.
    fn model_name(&self) -> &str;
}

/// Trait 5: Decision contradiction and drift evaluation.
pub trait ContradictionModel: Send + Sync {
    /// Checks whether a proposed action or code change contradicts an established architectural decision.
    fn check_contradiction(
        &self,
        proposed_change: &str,
        established_action: &KnoxAction,
    ) -> KnoxResult<ContradictionEvaluation>;

    /// Identifier of the active contradiction model.
    fn model_name(&self) -> &str;
}

