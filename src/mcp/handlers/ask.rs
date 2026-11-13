//! MCP tool handler for `knox_ask` (Workstream H3).
//!
//! Synthesizes grounded answers to technical questions across project memory
//! with verified citations, prompt injection defense, and clean fallbacks.

use serde::{Deserialize, Serialize};
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

/// DTO for a verified citation within an answer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AskCitationDto {
    pub action_id: String,
    pub summary: String,
    pub file: Option<String>,
}

/// Structured response DTO for grounded Q&A and brainstorming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskResultDto {
    pub question: String,
    pub answer: String,
    pub citations: Vec<AskCitationDto>,
    pub citations_count: usize,
    pub model: String,
    pub latency_ms: u64,
    pub candidates_evaluated: usize,
    pub grounded: bool,
    pub confidence: f64,
}

/// Core logic for executing grounded ask query, reusable by both MCP and Tauri commands.
pub fn execute_ask(
    db: &KnoxDb,
    config: &KnoxConfig,
    question: &str,
    project_input: Option<&str>,
    file_path: Option<&str>,
    strict_citations: bool,
) -> KnoxResult<AskResultDto> {
    let start_time = std::time::Instant::now();
    let trimmed_question = question.trim();
    if trimmed_question.is_empty() {
        return Err(KnoxError::InvalidInput(
            "'question' parameter cannot be empty".to_string(),
        ));
    }

    // Resolve project scope if provided, or fallback to most recent project
    let project_id = match project_input {
        Some(p) if !p.trim().is_empty() => resolve_project_identifier(db, p)?,
        _ => crate::storage::list_projects(db)
            .ok()
            .and_then(|list| list.into_iter().next().map(|p| p.id.as_str().to_string())),
    };

    // 1. Retrieve candidate actions from project memory
    let filter = crate::core::types::QueryFilter {
        project: project_id.clone(),
        file_path: file_path.map(str::to_string),
        limit: 30,
        ..Default::default()
    };

    let mut candidates = query_actions(db, &filter)?.results;

    // 2. Prompt injection defense: sanitize question
    let sanitized_question = sanitize_prompt(trimmed_question);

    let ask_req = AskRequest {
        question: sanitized_question.clone(),
        file_path: file_path.map(str::to_string),
        project: project_id.clone(),
        strict_citations,
    };

    // 3. ModelRouter resolution & embedding-based search if active
    let router = ModelRouter::for_project(config, Some(db), project_id.as_deref());
    router.check_egress()?;

    if router.is_enabled() {
        if let Ok(q_vec) = router.embed(&sanitized_question) {
            if let Ok(similar) = crate::storage::embeddings::find_similar_actions(
                db,
                project_id.as_deref(),
                &q_vec,
                10,
                0.4,
            ) {
                let sim_ids: Vec<ActionId> = similar.into_iter().map(|(id, _)| id).collect();
                if let Ok(extra_actions) = crate::storage::queries::get_actions_by_ids(db, &sim_ids) {
                    for act in extra_actions {
                        if (project_id.is_none() || act.project_id.as_ref().map(|p| p.as_str()) == project_id.as_deref())
                            && !candidates.iter().any(|c| c.id == act.id)
                        {
                            candidates.push(act);
                        }
                    }
                }
            }
        }
    }

    let candidates_count = candidates.len();

    // 4. Rank candidates by relevance to the question
    let ranked = rank_actions_hybrid(
        candidates,
        file_path,
        &[],
        Some(&sanitized_question),
        Some(&router),
        Some(db),
    );

    // 5. Answer generation via ModelRouter
    let mut resp = router.answer(&ask_req, &ranked)?;

    // 6. Citation verification
    let valid_ids: std::collections::HashSet<ActionId> =
        ranked.iter().map(|a| a.id.clone()).collect();

    if strict_citations {
        resp.citations.retain(|c| valid_ids.contains(&c.action_id));
        resp.answer = filter_hallucinated_citations(&resp.answer, &valid_ids);
    }

    let latency_ms = start_time.elapsed().as_millis() as u64;
    crate::storage::telemetry::record_query_log_best_effort(
        db,
        &crate::core::types::QueryLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            tool: "knox_ask".to_string(),
            project_id: project_id.clone(),
            rows: ranked.len(),
            tokens: resp.answer.len() / 4,
            latency_ms,
            reason: if ranked.is_empty() {
                Some("no_records".to_string())
            } else {
                None
            },
        },
    );

    let citations_dto: Vec<AskCitationDto> = resp
        .citations
        .into_iter()
        .map(|c| AskCitationDto {
            action_id: c.action_id.as_str().to_string(),
            summary: c.summary,
            file: c.file,
        })
        .collect();

    let citations_count = citations_dto.len();
    let grounded = resp.grounded && citations_count > 0;

    Ok(AskResultDto {
        question: trimmed_question.to_string(),
        answer: resp.answer,
        citations: citations_dto,
        citations_count,
        model: resp.model,
        latency_ms,
        candidates_evaluated: candidates_count,
        grounded,
        confidence: resp.confidence,
    })
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

    // 1. Enforce project scope (Workstream C for MCP agents)
    let project_str = input.project.as_deref().unwrap_or("").trim();
    if project_str.is_empty() {
        return Err(KnoxError::NoScope(
            "no project scope provided — pass `project` to scope the call".to_string(),
        ));
    }

    let config = KnoxConfig::from_env();
    let res = execute_ask(
        db,
        &config,
        &input.question,
        Some(project_str),
        input.file_path.as_deref(),
        input.strict_citations,
    )?;

    Ok(json!({
        "question": res.question,
        "answer": res.answer,
        "citations": res.citations,
        "citations_count": res.citations_count,
        "grounded": res.grounded,
        "model": res.model,
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

