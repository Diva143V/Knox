//! Model layer status and diagnostic commands for Knox CLI (Workstream H).

use crate::core::config::KnoxConfig;
use crate::core::error::KnoxResult;
use crate::models::profile::ModelProfile;
use crate::models::router::ModelRouter;
use crate::models::traits::AskRequest;

/// Displays the active model configuration, profile, and endpoint health.
pub fn cmd_model_status(config: &KnoxConfig) -> KnoxResult<()> {
    let profile = ModelProfile::from_config(config);
    println!("=== Knox Model Layer Status ===");
    println!();
    println!("Active Profile    : {}", profile.kind.as_str());
    println!("Clean-Machine     : {}", if !profile.kind.is_enabled() { "ENABLED (models off, zero external dependencies)" } else { "DISABLED (external/mock models active)" });
    println!("Endpoint          : {}", if profile.endpoint.is_empty() { "(none)" } else { &profile.endpoint });
    println!("Embedding Model   : {}", profile.embedding_model);
    println!("Chat / Ask Model  : {}", profile.chat_model);
    println!("API Key Configured: {}", if profile.api_key.is_some() { "yes" } else { "no" });
    println!();

    if !profile.kind.is_enabled() {
        println!("Note: Knox runs out-of-the-box with models 'off'.");
        println!("To activate local models (Ollama): export KNOX_MODEL_PROFILE=local");
        println!("To activate mock models in tests : export KNOX_MODEL_PROFILE=mock");
        println!("To activate cloud models (OpenAI): export KNOX_MODEL_PROFILE=cloud KNOX_MODEL_API_KEY=sk-...");
    }

    Ok(())
}

/// Tests model connectivity, embedding generation, and answering against the active profile.
pub fn cmd_model_test(config: &KnoxConfig, test_query: &str) -> KnoxResult<()> {
    let router = ModelRouter::from_config(config);
    println!("=== Testing Knox Model Layer ({}) ===", router.profile.kind.as_str());
    println!();

    // 1. Test embedding
    print!("1. Testing vector embedding generation... ");
    match router.embed("Architectural decision record: SQLite WAL mode") {
        Ok(vec) => {
            println!("OK (dimensions: {}, first 3: [{:.3}, {:.3}, {:.3}])", vec.len(), vec.first().unwrap_or(&0.0), vec.get(1).unwrap_or(&0.0), vec.get(2).unwrap_or(&0.0));
        }
        Err(e) => {
            println!("FALLBACK ({e})");
        }
    }

    // 2. Test answering
    print!("2. Testing grounded answering for: \"{test_query}\"... ");
    let req = AskRequest {
        question: test_query.to_string(),
        file_path: None,
        project: None,
        strict_citations: true,
    };

    match router.answer(&req, &[]) {
        Ok(resp) => {
            println!("OK (model: {})", resp.model);
            println!();
            println!("Response:");
            println!("{}", resp.answer);
        }
        Err(e) => {
            println!("FAILED ({e})");
        }
    }

    Ok(())
}

