//! Model abstraction and routing layer for Knox (Workstream H).
//!
//! Provides the five domain model slots:
//! - `EmbeddingModel`: vector generation.
//! - `RerankModel`: candidate reranking.
//! - `AnsweringModel`: grounded question answering for `knox_ask`.
//! - `ExtractionModel`: transcript proposal extraction.
//! - `ContradictionModel`: architectural contradiction detection.
//!
//! Enforces the clean-machine promise via `ModelProfileKind::Off` by default.

pub mod adapters;
pub mod mock;
pub mod profile;
pub mod router;
pub mod traits;

pub use adapters::{CloudAdapter, LocalAdapter};
pub use mock::MockModelBackend;
pub use profile::{ModelProfile, ModelProfileKind};
pub use router::ModelRouter;
pub use traits::{
    AnsweringModel, AskRequest, AskResponse, Citation, ContradictionEvaluation, ContradictionModel,
    EmbeddingModel, ExtractedActionProposal, ExtractionModel, RerankModel, ScoredAction,
};

