//! MCP tool handler for `knox_ask` (Workstream H3).
//!
//! Synthesizes grounded answers to technical questions across project memory
//! with verified citations, prompt injection defense, and clean fallbacks.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::core::config::KnoxConfig;
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::ActionId;
use crate::models::router::ModelRouter;
use crate::models::traits::AskRequest;
use crate::retrieval::rank::rank_actions_hybrid;
use crate::storage::db::KnoxDb;
use crate::storage::queries::query_actions;
use crate::storage::sessions::resolve_project_identifier;

#[derive(Debug, Deserialize)]
pub struct AskParams {
    pub question: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default = "default_strict")]
    pub strict_citations: bool,
}

fn default_strict() -> bool {
    true
}

/// Executes `knox_ask`.
pub fn handle_ask(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let input: AskParams = serde_json::from_value(params)
        .map_err(|e| KnoxError::InvalidInput(format!("Failed to parse knox_ask parameters: {e}")))?;

    if input.question.trim().is_empty() {
        return Err(KnoxError::InvalidInput(
            "'question' parameter cannot be empty".to_string(),
        ));
    }

    // 1. Enforce project scope (Workstream C)
    let project_id = match input.project.as_deref() {
        Some(p) => resolve_project_identifier(db, p)?,
        None => {
            return Err(KnoxError::NoScope(
                "no project scope provided — pass `project` to scope the call".to_string(),
            ));
        }
    };

    // 2. Retrieve candidate actions from project memory
    let filter = crate::core::types::QueryFilter {
        project: project_id.clone(),
        file_path: input.file_path.clone(),
        limit: 30,
        ..Default::default()
    };

    let mut candidates = query_actions(db, &filter)?.results;

    // 3. Prompt injection defense: sanitize question
    let sanitized_question = sanitize_prompt(&input.question);

    let ask_req = AskRequest {
        question: sanitized_question.clone(),
        file_path: input.file_path.clone(),
        project: project_id.clone(),
        strict_citations: input.strict_citations,
    };

    // 4. ModelRouter resolution & embedding-based search if active (fails closed on cloud without egress)
    let config = KnoxConfig::from_env();
    let router = ModelRouter::for_project(&config, Some(db), project_id.as_deref());
    router.check_egress()?;

    if router.is_enabled() {
        if let Ok(q_vec) = router.embed(&sanitized_question) {
            if let Ok(similar) = crate::storage::embeddings::find_similar_actions(db, &q_vec, 10, 0.4) {
                let sim_ids: Vec<ActionId> = similar.into_iter().map(|(id, _)| id).collect();
                if let Ok(extra_actions) = crate::storage::queries::get_actions_by_ids(db, &sim_ids) {
                    for act in extra_actions {
                        if !candidates.iter().any(|c| c.id == act.id) {
                            candidates.push(act);
                        }
                    }
                }
            }
        }
    }

    // 5. Rank candidates by relevance to the question
    let ranked = rank_actions_hybrid(
        candidates,
        input.file_path.as_deref(),
        &[],
        Some(&sanitized_question),
        Some(&router),
        Some(db),
    );

    // 6. Answer generation via ModelRouter
    let mut resp = router.answer(&ask_req, &ranked)?;

    // 7. Citation verification
    let valid_ids: std::collections::HashSet<ActionId> =
        ranked.iter().map(|a| a.id.clone()).collect();

    if input.strict_citations {
        resp.citations.retain(|c| valid_ids.contains(&c.action_id));
        resp.answer = filter_hallucinated_citations(&resp.answer, &valid_ids);
    }

    Ok(json!({
        "question": input.question,
        "answer": resp.answer,
        "citations": resp.citations,
        "citations_count": resp.citations.len(),
        "grounded": resp.grounded && !resp.citations.is_empty(),
        "model": resp.model,
    }))
}

/// Sanitizes input to prevent prompt injection attacks from altering agent instructions.
pub fn sanitize_prompt(text: &str) -> String {
    let mut sanitized = text.to_string();
    let escape_patterns = [
        ("<system>", ""),
        ("</system>", ""),
        ("<instructions>", ""),
        ("</instructions>", ""),
        ("ignore all previous instructions", "[neutralized]"),
        ("ignore previous instructions", "[neutralized]"),
        ("disregard all previous instructions", "[neutralized]"),
    ];

    // Multi-pass loop until fixed-point to neutralize repeated and nested injection markers
    for _ in 0..10 {
        let mut changed = false;
        for (pattern, replacement) in escape_patterns {
            while let Some(pos) = sanitized.to_ascii_lowercase().find(pattern) {
                sanitized.replace_range(pos..pos + pattern.len(), replacement);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    sanitized
}

/// Removes `[ref:UUID]` tokens if the UUID is not present in the verified candidate set.
pub fn filter_hallucinated_citations(
    text: &str,
    valid_ids: &std::collections::HashSet<ActionId>,
) -> String {
    let re = regex::Regex::new(r"\[ref:([a-zA-Z0-9_\-]+)\]").unwrap();
    re.replace_all(text, |caps: &regex::Captures| {
        let id_str = &caps[1];
        let id = ActionId::from(id_str);
        if valid_ids.contains(&id) {
            caps[0].to_string()
        } else {
            String::new()
        }
    })
    .to_string()
}

