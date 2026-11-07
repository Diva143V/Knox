//! Atomic record command.
//!
//! Business logic for `knox_record`: one transaction per call, local refs
//! between entries, idempotency replay/conflict, exact and near dedupe,
//! oversize caps, cycle detection, cross-project rejection, dedupe-id
//! remapping for later local refs.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::{ActionId, ProjectId};
use crate::core::types::{ActionRelation, KnoxAction, RelationType};
use crate::ingest::summary::normalize_file_path_with_root;
use crate::models::ModelRouter;
use crate::storage::db::KnoxDb;

const MAX_SUMMARY_CHARS: usize = 300;
const MAX_WHY_CHARS: usize = 1_000;
const DEDUPE_WINDOW: Duration = Duration::hours(24);
const NEAR_DUPLICATE_JACCARD: f64 = 0.85;
const MAX_WARNINGS: usize = 5;

/// Input parameters for `knox_record`.
#[derive(Debug, Deserialize)]
pub struct RecordParams {
    pub actor: String,
    /// Project scope (required — cross-project writes are rejected).
    #[serde(default, alias = "workspace")]
    pub project: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub idem: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
    pub entries: Vec<RecordEntry>,
}

/// A single action entry within a `knox_record` request.
#[derive(Debug, Deserialize)]
pub struct RecordEntry {
    #[serde(default, alias = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub action_type: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default, rename = "ref")]
    pub local_ref: Option<String>,
    #[serde(default)]
    pub caused_by: Option<String>,
    #[serde(default)]
    pub parent_action_id: Option<String>,
    #[serde(default)]
    pub supersedes: Option<String>,
    #[serde(default)]
    pub relations: Vec<RecordRelation>,
    #[serde(default, alias = "alternatives")]
    pub rejected_alternatives: Vec<String>,
    #[serde(default)]
    pub context_summary: Option<String>,
    #[serde(default)]
    pub significance: Option<String>,
    #[serde(default)]
    pub is_resolved: Option<bool>,
    #[serde(default)]
    pub resolution_note: Option<String>,
    #[serde(default)]
    pub link_to_previous: Option<bool>,
}

/// A semantic relation to another action within a `knox_record` entry.
#[derive(Debug, Deserialize)]
pub struct RecordRelation {
    #[serde(default, alias = "target")]
    pub to: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default, alias = "type")]
    pub relation_type: Option<String>,
}

/// A prepared entry awaiting commit, with assigned IDs and dedupe key.
#[derive(Debug, Clone)]
pub struct PreparedEntry {
    pub action: KnoxAction,
    pub local_ref: Option<String>,
    pub dedupe_key: String,
    pub supersedes_hint: Option<String>,
}

/// A recorded entry result, with dedupe tracking.
#[derive(Debug, Clone)]
struct RecordedEntry {
    id: ActionId,
    local_ref: Option<String>,
    deduped: bool,
}

/// Deterministic FNV-1a hash, stable across processes and releases.
fn stable_hash(input: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn canonical_hash(value: &Value) -> KnoxResult<String> {
    Ok(stable_hash(&serde_json::to_string(value)?))
}

fn map_kind(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "decision" | "decide" => "decide".to_string(),
        "doubt" | "question" => "doubt".to_string(),
        "plan" => "plan".to_string(),
        "action" | "write" | "change" | "edit" => "write".to_string(),
        "outcome" | "verify" | "test" => "verify".to_string(),
        "debug" | "error" => "debug".to_string(),
        "review" | "read" => "review".to_string(),
        other => normalize_action_type(other),
    }
}

/// Normalises an unknown action-type string into a canonical vocabulary.
fn normalize_action_type(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "decision" => "decide".to_string(),
        "question" => "doubt".to_string(),
        other => other.to_string(),
    }
}

/// Resolves the session for a record call, creating one implicitly if needed.
fn resolve_session(
    db: &KnoxDb,
    actor: &str,
    project: Option<&str>,
    provided_id: Option<&str>,
) -> KnoxResult<(crate::core::ids::SessionId, bool)> {
    if let Some(sid_str) = provided_id {
        let trimmed = sid_str.trim();
        if !trimmed.is_empty() {
            let sid = crate::core::ids::SessionId::from(trimmed);
            let session = crate::storage::sessions::get_session(db, &sid)?
                .ok_or_else(|| KnoxError::SessionNotFound(sid.to_string()))?;
            if !session.is_open {
                return Err(KnoxError::SessionClosed(sid.to_string()));
            }
            return Ok((sid, false)); // explicit session
        }
    }

    let project_id = match project {
        Some(p) => crate::storage::sessions::resolve_project_identifier(db, p)?,
        None => None,
    };

    let recent = crate::storage::sessions::list_recent_sessions(db, 20)?;
    for s in recent {
        if s.is_open && s.actor == actor {
            let same_project = match &project_id {
                Some(pid) => s.project_id.as_ref().map(|p| p.as_str()) == Some(pid.as_str()),
                None => s.project_id.is_none(),
            };
            if same_project {
                return Ok((s.id, true));
            }
        }
    }

    let session = crate::storage::sessions::start_session(db, actor, project)?;
    Ok((session.id, false))
}

/// Picks a default significance instead of labelling every action "major".
fn default_significance(action_type: &str) -> &'static str {
    match action_type {
        "decide" | "plan" | "doubt" => "major",
        _ => "minor",
    }
}

fn dedupe_tokens(action: &KnoxAction) -> HashSet<String> {
    let mut tokens = HashSet::new();
    for token in action
        .summary
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
    {
        tokens.insert(token.to_string());
    }
    for file in &action.files {
        tokens.insert(file.to_lowercase());
    }
    for symbol in &action.symbols {
        tokens.insert(symbol.to_lowercase());
    }
    tokens
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// Finds an equivalent action written within the merge window.
fn find_dedupe_candidate(
    conn: &rusqlite::Connection,
    action: &KnoxAction,
    dedupe_key: &str,
    router: Option<&crate::models::router::ModelRouter>,
) -> KnoxResult<Option<ActionId>> {
    let cutoff = (Utc::now() - DEDUPE_WINDOW).timestamp_millis();

    let exact: Option<String> = conn
        .query_row(
            "SELECT id FROM actions
             WHERE dedupe_key = ?1 AND COALESCE(timestamp_ms, 0) >= ?2
             ORDER BY timestamp_ms DESC LIMIT 1",
            params![dedupe_key, cutoff],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = exact {
        return Ok(Some(ActionId::from(id)));
    }

    // Near-duplicate: same actor/project/kind and Jaccard >= 0.85.
    let mut stmt = conn.prepare(
        "SELECT a.id, a.summary, COALESCE(a.dedupe_key, ''),
                COALESCE(a.rejected_alternatives, '[]')
         FROM actions a
         WHERE a.actor = ?1
           AND a.action_type = ?2
           AND ((?3 IS NULL AND a.project_id IS NULL) OR a.project_id = ?3)
           AND COALESCE(a.timestamp_ms, 0) >= ?4
         ORDER BY a.timestamp_ms DESC LIMIT 50",
    )?;
    let candidates = stmt
        .query_map(
            params![
                action.actor,
                action.action_type,
                action.project_id.as_ref().map(ProjectId::as_str),
                cutoff
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;

    let candidate_tokens = dedupe_tokens(action);
    let candidate_vec = router.and_then(|r| if r.is_enabled() { r.embed(&action.summary).ok() } else { None });

    for (id, summary, existing_key, existing_alternatives) in candidates {
        // Decisions are only duplicates when they reject the same alternatives;
        // otherwise the new entry may be adding a guardrail rule.
        if action.action_type == "decide" {
            let alternatives: Vec<String> =
                serde_json::from_str(&existing_alternatives).unwrap_or_default();
            if alternatives != action.rejected_alternatives {
                continue;
            }
        }

        // Exact summaries are always duplicates; otherwise require file/symbol
        // overlap as well so unrelated short entries do not collapse.
        let mut other = action.clone();
        other.summary = summary;
        other.dedupe_key = if existing_key.is_empty() {
            None
        } else {
            Some(existing_key)
        };
        if jaccard(&candidate_tokens, &dedupe_tokens(&other)) >= NEAR_DUPLICATE_JACCARD {
            return Ok(Some(ActionId::from(id)));
        }

        // Semantic embedding similarity check when models are enabled
        if let Some(ref cand_vec) = candidate_vec {
            let mut emb_stmt = conn.prepare("SELECT vector FROM embeddings WHERE action_id = ?1")?;
            let mut emb_rows = emb_stmt.query(params![id.as_str()])?;
            if let Some(row) = emb_rows.next()? {
                let blob: Vec<u8> = row.get(0)?;
                let existing_vec = crate::storage::embeddings::blob_to_vector(&blob);
                let sim = crate::storage::embeddings::cosine_similarity(cand_vec, &existing_vec);
                if sim >= 0.88 {
                    return Ok(Some(ActionId::from(id)));
                }
            }
        }
    }
    Ok(None)
}

fn record_ref_key(value: &str) -> &str {
    value.trim().strip_prefix("ref:").unwrap_or(value.trim())
}

fn resolve_reference(value: &str, local_ids: &HashMap<String, ActionId>) -> Option<ActionId> {
    let key = record_ref_key(value);
    local_ids.get(key).cloned()
}

fn would_cycle(
    start: &ActionId,
    parent: &ActionId,
    pending_parents: &HashMap<ActionId, ActionId>,
) -> bool {
    let mut current = Some(parent.clone());
    let mut seen = HashSet::new();
    while let Some(id) = current {
        if &id == start {
            return true;
        }
        if !seen.insert(id.clone()) {
            return true;
        }
        current = pending_parents.get(&id).cloned();
    }
    false
}

fn resolve_target_project(
    conn: &rusqlite::Connection,
    target: &ActionId,
    current_project: Option<&ProjectId>,
) -> KnoxResult<Option<Option<String>>> {
    let project: Option<Option<String>> = conn
        .query_row(
            "SELECT project_id FROM actions WHERE id = ?1",
            params![target.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(p) = project {
        return Ok(Some(p));
    }

    // Check refs table
    let ref_project: Option<Option<String>> = conn
        .query_row(
            "SELECT project_id FROM refs WHERE id = ?1",
            params![target.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(p) = ref_project {
        return Ok(Some(p));
    }

    // If target has a ref prefix (e.g. "file:src/db.rs"), auto-register in refs
    let target_str = target.as_str();
    if let Some((kind, val)) = target_str.split_once(':') {
        if matches!(
            kind,
            "file" | "symbol" | "commit" | "doc" | "url" | "ref" | "requirement" | "defect" | "issue"
        ) && !val.is_empty()
        {
            let now = Utc::now().to_rfc3339();
            let proj_str = current_project.map(|p| p.as_str());
            conn.execute(
                "INSERT OR IGNORE INTO refs (id, kind, value, project_id, created_at)
                 VALUES (?1, ?2, ?3, (SELECT id FROM projects WHERE id = ?4), ?5)",
                params![target_str, kind, val, proj_str, now],
            )?;
            return Ok(Some(proj_str.map(|s| s.to_string())));
        }
    }

    Ok(None)
}

/// Rewrites parent, supersession and relation targets through a dedupe map.
fn remap_action_refs(action: &mut KnoxAction, remap: &HashMap<ActionId, ActionId>) {
    if let Some(parent) = action.parent_action_id.as_ref() {
        if let Some(replacement) = remap.get(parent) {
            action.parent_action_id = Some(replacement.clone());
        }
    }
    if let Some(supersedes) = action.supersedes.as_ref() {
        if let Some(replacement) = remap.get(supersedes) {
            action.supersedes = Some(replacement.clone());
        }
    }
    for relation in &mut action.relations {
        if let Some(replacement) = remap.get(&relation.target_id) {
            relation.target_id = replacement.clone();
        }
    }
}

/// Runs the atomic record workflow for `knox_record`.
///
/// One transaction per call. Validates entries, assigns IDs, resolves local
/// refs, dedupes against existing actions, inserts new actions, and builds
/// the acknowledgment response. Supports idempotency keys, dry-run mode,
/// oversize caps, cycle detection, and cross-project rejection.
/// Parsed and prepared batch ready for transaction execution.
pub struct PreparedRecordBatch {
    pub session_id: crate::core::ids::SessionId,
    pub is_implicit: bool,
    pub prepared: Vec<PreparedEntry>,
    pub warnings: Vec<Value>,
    pub dry_run: bool,
    pub idem: Option<String>,
}

/// 1. Validates the input payload, checks idempotency replay, and verifies mandatory fields.
pub fn validate_record_input(
    db: &KnoxDb,
    params: &Value,
) -> KnoxResult<(RecordParams, String, Option<Value>)> {
    let object = params
        .as_object()
        .ok_or_else(|| KnoxError::InvalidInput("record params must be an object".to_string()))?
        .clone();

    // Hash the payload before deserialising so replay detection covers every
    // field the caller sent (except the key itself and dry_run).
    let mut hash_object = object.clone();
    hash_object.remove("idem");
    hash_object.remove("dry_run");
    let payload_hash = canonical_hash(&Value::Object(hash_object))?;

    let input: RecordParams = serde_json::from_value(Value::Object(object))
        .map_err(|e| KnoxError::InvalidInput(format!("Invalid record params: {e}")))?;

    if input.actor.trim().is_empty() {
        return Err(KnoxError::InvalidInput(
            "actor must not be empty".to_string(),
        ));
    }
    if input.project.is_none() {
        return Err(KnoxError::NoScope(
            "project is required — cross-project writes are rejected".to_string(),
        ));
    }
    if input.entries.is_empty() {
        return Err(KnoxError::InvalidInput(
            "record requires at least one entry".to_string(),
        ));
    }

    if let Some(key) = input.idem.as_deref() {
        if let Some((stored_hash, response)) = load_idempotent_response(db, key)? {
            if stored_hash == payload_hash {
                return Ok((input, payload_hash, Some(response)));
            }
            return Err(KnoxError::IdemConflict(key.to_string()));
        }
    }

    Ok((input, payload_hash, None))
}

/// 2. Resolves session, validates individual entries, checks cycles & directions, and prepares records.
pub fn prepare_record_entries(
    db: &KnoxDb,
    input: &RecordParams,
) -> KnoxResult<PreparedRecordBatch> {
    let project = input.project.as_deref();
    let (session_id, is_implicit) = resolve_session(
        db,
        input.actor.trim(),
        project,
        input.session_id.as_deref(),
    )?;
    let session = crate::storage::sessions::get_session(db, &session_id)?
        .ok_or_else(|| KnoxError::SessionNotFound(session_id.to_string()))?;
    let project_id = session.project_id.clone();
    let project_root = project_id
        .as_ref()
        .and_then(|pid| crate::storage::nodes::get_project(db, pid).ok().flatten())
        .map(|p| p.path);

    let actor = input.actor.trim().to_string();
    let now = Utc::now();

    // First pass: validate, assign ids, and collect local refs.
    let mut local_ids: HashMap<String, ActionId> = HashMap::new();
    let mut pending_ids: HashSet<ActionId> = HashSet::new();
    let mut prepared: Vec<PreparedEntry> = Vec::with_capacity(input.entries.len());
    let mut warnings: Vec<Value> = Vec::new();
    let mut pending_parents: HashMap<ActionId, ActionId> = HashMap::new();
    let mut previous_id: Option<ActionId> = None;

    for entry in &input.entries {
        let summary = entry.summary.trim();
        if summary.is_empty() {
            return Err(KnoxError::InvalidInput(
                "summary must not be empty on every entry".to_string(),
            ));
        }
        if summary.chars().count() > MAX_SUMMARY_CHARS {
            return Err(KnoxError::OversizeInput(format!(
                "summary exceeds {MAX_SUMMARY_CHARS} characters"
            )));
        }
        let why = entry.why.clone().or_else(|| entry.rationale.clone());
        if let Some(ref why) = why {
            if why.chars().count() > MAX_WHY_CHARS {
                return Err(KnoxError::OversizeInput(format!(
                    "why exceeds {MAX_WHY_CHARS} characters"
                )));
            }
        }

        let local_ref = entry
            .local_ref
            .as_deref()
            .map(record_ref_key)
            .filter(|r| !r.is_empty())
            .map(str::to_string);
        if let Some(ref local_ref) = local_ref {
            if local_ids.contains_key(local_ref) {
                return Err(KnoxError::InvalidRef(format!(
                    "duplicate local ref '{local_ref}'"
                )));
            }
        }

        let action_type = entry
            .kind
            .as_deref()
            .or(entry.action_type.as_deref())
            .map(map_kind)
            .ok_or_else(|| {
                KnoxError::InvalidInput("every entry requires 'kind' or 'action_type'".to_string())
            })?;

        let is_doubt = action_type == "doubt";
        let is_resolved = entry.is_resolved.or(Some(!is_doubt));
        let significance = entry.significance.clone().unwrap_or_else(|| {
            default_significance(&action_type).to_string()
        });

        let norm_files: Vec<String> = entry
            .files
            .iter()
            .map(|file| normalize_file_path_with_root(file, project_root.as_deref()))
            .collect();
        let symbols = entry.symbols.clone();

        let id = ActionId::new();
        pending_ids.insert(id.clone());
        if let Some(ref local_ref) = local_ref {
            local_ids.insert(local_ref.clone(), id.clone());
        }

        let parent_hint = entry
            .caused_by
            .as_deref()
            .or(entry.parent_action_id.as_deref())
            .map(str::to_string);
        let should_link = entry.link_to_previous.unwrap_or(false);
        let parent_id = match parent_hint {
            Some(ref hint) => Some(
                resolve_reference(hint, &local_ids)
                    .unwrap_or_else(|| ActionId::from(record_ref_key(hint))),
            ),
            None if should_link => previous_id.clone(),
            None => None,
        };

        if let Some(ref parent) = parent_id {
            if !pending_ids.contains(parent) {
                // A parent outside this batch must already exist and belong to
                // the same project; cross-project edges are rejected.
                match crate::storage::nodes::get_action(db, parent)? {
                    Some(existing) => {
                        if let (Some(current), Some(existing_project)) =
                            (project_id.as_ref(), existing.project_id.as_ref())
                        {
                            if current.as_str() != existing_project.as_str() {
                                return Err(KnoxError::CrossProject(format!(
                                    "parent action {parent} belongs to another project"
                                )));
                            }
                        }
                    }
                    None => {
                        return Err(KnoxError::InvalidRef(format!(
                            "unknown parent action or ref '{}'",
                            entry.caused_by.as_deref().unwrap_or("")
                        )));
                    }
                }
            }
            pending_parents.insert(id.clone(), parent.clone());
            if would_cycle(&id, parent, &pending_parents) {
                return Err(KnoxError::Cycle(format!(
                    "caused_by chain for entry '{}' closes a cycle",
                    local_ref.as_deref().unwrap_or(id.as_str())
                )));
            }
        }

        let dedupe_key = stable_hash(&format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            project_id.as_ref().map(ProjectId::as_str).unwrap_or(""),
            actor,
            action_type,
            summary.to_lowercase(),
            norm_files.join(","),
            symbols.join(","),
            why.as_deref().unwrap_or(""),
            entry.rejected_alternatives.join(",")
        ));

        let action = KnoxAction {
            id: id.clone(),
            actor: actor.clone(),
            action_type,
            summary: summary.to_string(),
            files: norm_files,
            tags: entry.tags.clone(),
            confidence: entry.confidence,
            timestamp: now,
            session_id: Some(session_id.clone()),
            project_id: project_id.clone(),
            parent_action_id: parent_id,
            supersedes: None,      // resolved in the second pass
            relations: Vec::new(), // resolved in the second pass
            symbols,
            rationale: why,
            rejected_alternatives: entry.rejected_alternatives.clone(),
            context_summary: entry.context_summary.clone(),
            significance: Some(significance),
            is_current: true,
            is_resolved,
            resolution_note: entry.resolution_note.clone(),
            commit_sha: None,
            delta_summary: None,
            dedupe_key: Some(dedupe_key.clone()),
            seen_count: 1,
        };

        if action.files.is_empty() && action.symbols.is_empty() {
            warnings.push(json!({
                "code": "low_value",
                "ref": local_ref,
                "message": "entry has no files and no symbols"
            }));
        }
        if action.action_type == "decide" && action.rejected_alternatives.is_empty() {
            warnings.push(json!({
                "code": "no_guardrail",
                "ref": local_ref,
                "message": "decision has no rejected_alternatives, so it creates no guardrail rule"
            }));
        }

        prepared.push(PreparedEntry {
            action,
            local_ref,
            dedupe_key,
            supersedes_hint: entry.supersedes.clone(),
        });
        previous_id = Some(id);
    }

    // Precompute local action types for direction validation without re-borrowing `prepared`.
    let local_types: HashMap<ActionId, String> = prepared
        .iter()
        .map(|p| (p.action.id.clone(), p.action.action_type.clone()))
        .collect();

    // Second pass: resolve semantic relations and supersession now that every
    // local ref exists.
    for (entry_index, prepared_entry) in prepared.iter_mut().enumerate() {
        if let Some(ref hint) = prepared_entry.supersedes_hint {
            let target = resolve_reference(hint, &local_ids)
                .unwrap_or_else(|| ActionId::from(record_ref_key(hint)));
            prepared_entry.action.supersedes = Some(target);
        }

        let relations_input = &input.entries[entry_index].relations;
        let mut relations = Vec::with_capacity(relations_input.len());
        for relation in relations_input {
            let raw_target = relation
                .to
                .as_deref()
                .or(relation.target_id.as_deref())
                .ok_or_else(|| {
                    KnoxError::InvalidRef("relation requires 'to' or 'target_id'".to_string())
                })?;
            let target = resolve_reference(raw_target, &local_ids)
                .unwrap_or_else(|| ActionId::from(record_ref_key(raw_target)));
            let is_local = local_ids.values().any(|candidate| candidate == &target);
            if !is_local {
                match resolve_target_project(db.conn(), &target, project_id.as_ref())? {
                    Some(target_project) => {
                        if let (Some(current_project), Some(target_project)) =
                            (project_id.as_ref(), target_project.as_ref())
                        {
                            if current_project.as_str() != target_project.as_str() {
                                return Err(KnoxError::CrossProject(format!(
                                    "relation target {target} belongs to another project"
                                )));
                            }
                        }
                    }
                    None => {
                        return Err(KnoxError::InvalidRef(format!(
                            "unknown relation target '{raw_target}'"
                        )));
                    }
                }
            }

            let raw_type = relation.relation_type.as_deref().unwrap_or("relates_to");
            let relation_type = match raw_type.parse::<RelationType>() {
                Ok(rt) => rt,
                Err(_) => {
                    let reason = format!(
                        "Unknown relation type '{raw_type}'. Allowed types: 'caused_by', 'implements', 'resolves', 'verifies', 'depends_on', 'relates_to', 'contradicts', 'supersedes', 'reverts'."
                    );
                    let _ = crate::storage::edges::record_edge_rejection(
                        db,
                        project_id.as_ref().map(|p| p.as_str()),
                        Some(prepared_entry.action.id.as_str()),
                        Some(target.as_str()),
                        raw_type,
                        &reason,
                    );
                    return Err(KnoxError::InvalidRelation(reason));
                }
            };

            // Direction validator per type (Task B6)
            let source_type = prepared_entry.action.action_type.as_str();
            let target_type_opt: Option<String> = if is_local {
                local_types.get(&target).cloned()
            } else {
                db.conn()
                    .query_row(
                        "SELECT action_type FROM actions WHERE id = ?1",
                        params![target.as_str()],
                        |r| r.get(0),
                    )
                    .optional()?
            };

            match relation_type {
                RelationType::Implements => {
                    if matches!(source_type, "decide" | "plan") {
                        let reason = format!(
                            "Invalid relation direction for 'implements': a {source_type} cannot implement another action. Code actions ('write', 'debug') implement decisions ('decide'). Reverse the relation or use 'caused_by'."
                        );
                        let _ = crate::storage::edges::record_edge_rejection(
                            db,
                            project_id.as_ref().map(|p| p.as_str()),
                            Some(prepared_entry.action.id.as_str()),
                            Some(target.as_str()),
                            "implements",
                            &reason,
                        );
                        return Err(KnoxError::InvalidRelation(reason));
                    }
                }
                RelationType::Resolves => {
                    if source_type == "doubt" {
                        let reason = "Invalid relation direction for 'resolves': a doubt cannot resolve an action. Actions ('write', 'decide') resolve doubts ('doubt'). Reverse the relation.".to_string();
                        let _ = crate::storage::edges::record_edge_rejection(
                            db,
                            project_id.as_ref().map(|p| p.as_str()),
                            Some(prepared_entry.action.id.as_str()),
                            Some(target.as_str()),
                            "resolves",
                            &reason,
                        );
                        return Err(KnoxError::InvalidRelation(reason));
                    }
                    if let Some(ref target_type) = target_type_opt {
                        if target_type != "doubt" {
                            let reason = format!(
                                "Invalid relation target for 'resolves': 'resolves' must target a doubt, but target '{target}' has kind '{target_type}'."
                            );
                            let _ = crate::storage::edges::record_edge_rejection(
                                db,
                                project_id.as_ref().map(|p| p.as_str()),
                                Some(prepared_entry.action.id.as_str()),
                                Some(target.as_str()),
                                "resolves",
                                &reason,
                            );
                            return Err(KnoxError::InvalidRelation(reason));
                        }
                    }
                }
                RelationType::Verifies => {
                    if matches!(source_type, "decide" | "plan") {
                        let reason = format!(
                            "Invalid relation direction for 'verifies': a {source_type} cannot verify an implementation action. Testing or verification actions verify plans or decisions."
                        );
                        let _ = crate::storage::edges::record_edge_rejection(
                            db,
                            project_id.as_ref().map(|p| p.as_str()),
                            Some(prepared_entry.action.id.as_str()),
                            Some(target.as_str()),
                            "verifies",
                            &reason,
                        );
                        return Err(KnoxError::InvalidRelation(reason));
                    }
                }
                _ => {}
            }

            relations.push(ActionRelation {
                target_id: target,
                relation_type,
            });
        }
        prepared_entry.action.relations = relations;
    }

    warnings.truncate(MAX_WARNINGS);

    Ok(PreparedRecordBatch {
        session_id,
        is_implicit,
        prepared,
        warnings,
        dry_run: input.dry_run,
        idem: input.idem.clone(),
    })
}

/// Evaluates potential architectural contradictions against established decisions and persists model edges.
fn check_and_persist_contradictions(
    conn: &rusqlite::Connection,
    router: &ModelRouter,
    action: &KnoxAction,
    warnings: &mut Vec<Value>,
) -> KnoxResult<()> {
    if !router.is_enabled() || router.contradiction().is_none() {
        return Ok(());
    }

    let mut stmt = conn.prepare(
        "SELECT id, actor, action_type, summary, rationale, project_id, timestamp
         FROM actions
         WHERE is_current = 1 AND action_type = 'decide' AND id != ?1 AND (project_id = ?2 OR project_id IS NULL)
         LIMIT 20",
    )?;

    let proj_str = action.project_id.as_ref().map(|p| p.as_str());
    let rows = stmt.query_map(params![action.id.as_str(), proj_str], |row| {
        let ts_str: String = row.get(6)?;
        let ts = DateTime::parse_from_rfc3339(&ts_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(KnoxAction {
            id: ActionId::from(row.get::<_, String>(0)?),
            actor: row.get(1)?,
            action_type: row.get(2)?,
            summary: row.get(3)?,
            rationale: row.get(4)?,
            project_id: row.get::<_, Option<String>>(5)?.map(crate::core::ids::ProjectId),
            timestamp: ts,
            is_current: true,
            ..Default::default()
        })
    })?;

    let proposed_text = match &action.rationale {
        Some(r) => format!("{}\nRationale: {}", action.summary, r),
        None => action.summary.clone(),
    };

    let mut stmt_edge = conn.prepare_cached(
        "INSERT OR REPLACE INTO edges (id, from_action_id, to_id, edge_type, created_at, provenance, note)
         VALUES (?1, ?2, ?3, 'contradicts', ?4, 'model', ?5)",
    )?;
    let mut stmt_rel = conn.prepare_cached(
        "INSERT OR IGNORE INTO action_relations (from_action_id, to_action_id, relation_type)
         VALUES (?1, ?2, 'contradicts')",
    )?;

    for est in rows {
        let est_act = est?;
        if let Ok(eval) = router.check_contradiction(&proposed_text, &est_act) {
            if eval.contradicts {
                let edge_id = format!("{}:{}:contradicts", action.id, est_act.id);
                let now_str = Utc::now().to_rfc3339();
                stmt_edge.execute(params![
                    edge_id,
                    action.id.as_str(),
                    est_act.id.as_str(),
                    now_str,
                    eval.explanation,
                ])?;
                stmt_rel.execute(params![action.id.as_str(), est_act.id.as_str()])?;
                warnings.push(json!({
                    "code": "contradiction_detected",
                    "action_id": action.id.as_str(),
                    "established_id": est_act.id.as_str(),
                    "explanation": eval.explanation,
                }));
            }
        }
    }

    Ok(())
}

/// 3. Commits the prepared batch into the database inside a transaction.
pub fn commit_record_batch(
    db: &KnoxDb,
    batch: &PreparedRecordBatch,
    payload_hash: &str,
) -> KnoxResult<Value> {
    let dry_run = batch.dry_run;
    let config = crate::core::config::KnoxConfig::from_env();
    let project_id = batch.prepared.first().and_then(|e| e.action.project_id.as_ref().map(|p| p.as_str()));
    let router = crate::models::router::ModelRouter::for_project(&config, Some(db), project_id);

    db.with_tx(|conn| {
        let mut warnings = batch.warnings.clone();
        let mut recorded = Vec::with_capacity(batch.prepared.len());
        // A local ref that dedupes into an existing action must be remapped so
        // later entries in the same batch point at the committed row, not at
        // the generated id that was never inserted.
        let mut id_remap: HashMap<ActionId, ActionId> = HashMap::new();
        for entry in &batch.prepared {
            let mut action = entry.action.clone();
            remap_action_refs(&mut action, &id_remap);

            if let Some(existing) = find_dedupe_candidate(conn, &action, &entry.dedupe_key, Some(&router))? {
                if !dry_run {
                    conn.execute(
                        "UPDATE actions
                         SET seen_count = seen_count + 1,
                             dedupe_key = COALESCE(dedupe_key, ?1)
                         WHERE id = ?2",
                        params![entry.dedupe_key, existing.as_str()],
                    )?;
                }
                id_remap.insert(entry.action.id.clone(), existing.clone());
                recorded.push(RecordedEntry {
                    id: existing,
                    local_ref: entry.local_ref.clone(),
                    deduped: true,
                });
                continue;
            }

            if !dry_run {
                crate::storage::nodes::insert_action_conn(conn, &action)?;
                if router.is_enabled() {
                    if let Ok(vec) = router.embed(&action.summary) {
                        let _ = crate::storage::embeddings::insert_embedding_conn(
                            conn,
                            &action.id,
                            &vec,
                            &router.profile.embedding_model,
                        );
                    }
                    check_and_persist_contradictions(conn, &router, &action, &mut warnings)?;
                }
            }
            recorded.push(RecordedEntry {
                id: entry.action.id.clone(),
                local_ref: entry.local_ref.clone(),
                deduped: false,
            });
        }

        let response = build_ack(&batch.session_id, &recorded, &warnings, dry_run, batch.is_implicit);
        if let Some(key) = batch.idem.as_deref() {
            if !dry_run {
                conn.execute(
                    "INSERT INTO idempotency_keys (key, payload_hash, response_json, created_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        key,
                        payload_hash,
                        serde_json::to_string(&response)?,
                        Utc::now().to_rfc3339()
                    ],
                )?;
            }
        }
        Ok(response)
    })
}

/// Records a batch of actions against an open or implicit session.
/// Validates entries, resolves parent and supersedes references, dedupes against existing actions,
/// inserts new actions, and builds the acknowledgment response.
pub fn cmd_record(db: &KnoxDb, params: Value) -> KnoxResult<Value> {
    let (input, payload_hash, cached) = validate_record_input(db, &params)?;
    if let Some(resp) = cached {
        return Ok(resp);
    }
    let prepared_batch = prepare_record_entries(db, &input)?;
    commit_record_batch(db, &prepared_batch, &payload_hash)
}

fn build_ack(
    session_id: &crate::core::ids::SessionId,
    recorded: &[RecordedEntry],
    warnings: &[Value],
    dry_run: bool,
    session_implicit: bool,
) -> Value {
    let ids: Vec<String> = recorded.iter().map(|r| r.id.to_string()).collect();
    let deduped: Vec<String> = recorded
        .iter()
        .filter(|r| r.deduped)
        .map(|r| r.id.to_string())
        .collect();
    let refs: Map<String, Value> = recorded
        .iter()
        .filter_map(|r| {
            r.local_ref
                .as_ref()
                .map(|local_ref| (local_ref.clone(), Value::String(r.id.to_string())))
        })
        .collect();

    json!({
        "status": if dry_run { "dry_run" } else { "recorded" },
        "count": recorded.len(),
        "recorded_count": recorded.len() - deduped.len(),
        "ids": ids,
        "refs": Value::Object(refs),
        "deduped": deduped,
        "warnings": warnings,
        "session_id": session_id.to_string(),
        "session_implicit": session_implicit,
        "dry_run": dry_run
    })
}

fn load_idempotent_response(db: &KnoxDb, key: &str) -> KnoxResult<Option<(String, Value)>> {
    let row: Option<(String, String)> = db
        .conn()
        .query_row(
            "SELECT payload_hash, response_json FROM idempotency_keys WHERE key = ?1",
            params![key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    row.map(|(hash, response)| {
        serde_json::from_str(&response)
            .map(|value| (hash, value))
            .map_err(KnoxError::Json)
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record(db: &KnoxDb, entries: Value) -> KnoxResult<Value> {
        cmd_record(db, json!({ "actor": "gemini", "project": "demo", "entries": entries }))
    }

    #[test]
    fn requires_entries() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(cmd_record(&db, json!({"actor": "agent", "project": "demo"})).is_err());
    }

    #[test]
    fn dedupe_exact_increments_seen_count() {
        let db = KnoxDb::open_memory().unwrap();
        let entry = json!([{
            "kind": "action",
            "summary": "Implement parser",
            "files": ["src/parser.rs"],
            "symbols": ["parse"]
        }]);
        let first = record(&db, entry.clone()).unwrap();
        let second = record(&db, entry).unwrap();
        assert_eq!(first["ids"][0], second["ids"][0]);
        assert_eq!(second["deduped"].as_array().unwrap().len(), 1);

        let id = ActionId::from(second["ids"][0].as_str().unwrap());
        let action = crate::storage::nodes::get_action(&db, &id)
            .unwrap()
            .unwrap();
        assert_eq!(action.seen_count, 2);
    }

    #[test]
    fn dedupe_near_respects_window() {
        let db = KnoxDb::open_memory().unwrap();
        let first = record(
            &db,
            json!([{
                "kind": "action",
                "summary": "implement the token parser for imports",
                "files": ["src/parser.rs"]
            }]),
        )
        .unwrap();
        let id = ActionId::from(first["ids"][0].as_str().unwrap());

        // Move the original outside the 24h window.
        db.conn()
            .execute(
                "UPDATE actions SET timestamp_ms = ?1 WHERE id = ?2",
                params![
                    (Utc::now() - Duration::hours(48)).timestamp_millis(),
                    id.as_str()
                ],
            )
            .unwrap();

        let second = record(
            &db,
            json!([{
                "kind": "action",
                "summary": "implement the token parser for imports",
                "files": ["src/parser.rs"]
            }]),
        )
        .unwrap();
        assert_ne!(first["ids"][0], second["ids"][0]);
        assert!(second["deduped"].as_array().unwrap().is_empty());
    }

    #[test]
    fn local_refs_resolve_to_committed_ids() {
        let db = KnoxDb::open_memory().unwrap();
        let result = record(
            &db,
            json!([
                {"ref": "e1", "kind": "plan", "summary": "Plan the parser"},
                {
                    "ref": "e2",
                    "kind": "action",
                    "summary": "Write the parser",
                    "files": ["src/parser.rs"],
                    "caused_by": "e1",
                    "relations": [{"to": "e1", "relation_type": "implements"}]
                }
            ]),
        )
        .unwrap();

        let e2 = ActionId::from(result["refs"]["e2"].as_str().unwrap());
        let action = crate::storage::nodes::get_action(&db, &e2)
            .unwrap()
            .unwrap();
        assert_eq!(
            action.parent_action_id.unwrap().as_str(),
            result["refs"]["e1"].as_str().unwrap()
        );
        assert_eq!(
            action.relations[0].target_id.as_str(),
            result["refs"]["e1"].as_str().unwrap()
        );
    }

    #[test]
    fn idempotency_replays_and_rejects_different_payload() {
        let db = KnoxDb::open_memory().unwrap();
        let params = json!({
            "actor": "gemini",
            "project": "demo",
            "idem": "turn-1",
            "entries": [{"kind": "plan", "summary": "Plan once"}]
        });
        let first = cmd_record(&db, params.clone()).unwrap();
        let second = cmd_record(&db, params).unwrap();
        assert_eq!(first["ids"], second["ids"]);

        let conflicting = json!({
            "actor": "gemini",
            "project": "demo",
            "idem": "turn-1",
            "entries": [{"kind": "plan", "summary": "Plan twice"}]
        });
        assert!(matches!(
            cmd_record(&db, conflicting),
            Err(KnoxError::IdemConflict(_))
        ));
    }

    #[test]
    fn oversize_and_invalid_refs_are_errors() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(matches!(
            record(
                &db,
                json!([{"kind": "plan", "summary": "x".repeat(MAX_SUMMARY_CHARS + 1)}])
            ),
            Err(KnoxError::OversizeInput(_))
        ));
        assert!(matches!(
            record(
                &db,
                json!([{
                    "kind": "action",
                    "summary": "Write code",
                    "relations": [{"to": "missing-ref"}]
                }])
            ),
            Err(KnoxError::InvalidRef(_))
        ));
    }

    #[test]
    fn batch_failure_leaves_no_rows() {
        let db = KnoxDb::open_memory().unwrap();
        let before: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM actions", [], |r| r.get(0))
            .unwrap();
        let result = record(
            &db,
            json!([
                {"kind": "plan", "summary": "Valid first"},
                {"kind": "action", "summary": "Valid second"},
                {"kind": "action", "summary": "x".repeat(MAX_SUMMARY_CHARS + 1)}
            ]),
        );
        assert!(result.is_err());
        let after: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM actions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn low_value_and_no_guardrail_warnings() {
        let db = KnoxDb::open_memory().unwrap();
        let result = record(
            &db,
            json!([
                {"kind": "action", "summary": "Tweak something"},
                {"kind": "decision", "summary": "Choose a database"}
            ]),
        )
        .unwrap();
        let warnings = result["warnings"].as_array().unwrap();
        assert!(warnings.iter().any(|w| w["code"] == "low_value"));
        assert!(warnings.iter().any(|w| w["code"] == "no_guardrail"));
    }

    #[test]
    fn local_ref_to_deduped_entry_points_at_existing_row() {
        let db = KnoxDb::open_memory().unwrap();
        let first = record(
            &db,
            json!([{
                "ref": "plan",
                "kind": "plan",
                "summary": "Plan the parser",
                "files": ["src/parser.rs"]
            }]),
        )
        .unwrap();
        let existing = first["ids"][0].as_str().unwrap().to_string();

        let second = record(
            &db,
            json!([
                {
                    "ref": "same-plan",
                    "kind": "plan",
                    "summary": "Plan the parser",
                    "files": ["src/parser.rs"]
                },
                {
                    "ref": "work",
                    "kind": "action",
                    "summary": "Implement the lexer",
                    "files": ["src/lexer.rs"],
                    "caused_by": "same-plan"
                }
            ]),
        )
        .unwrap();

        assert_eq!(second["refs"]["same-plan"], existing);
        let work = ActionId::from(second["refs"]["work"].as_str().unwrap());
        let action = crate::storage::nodes::get_action(&db, &work)
            .unwrap()
            .unwrap();
        assert_eq!(action.parent_action_id.unwrap().as_str(), existing);
    }

    #[test]
    fn dry_run_reports_but_writes_nothing() {
        let db = KnoxDb::open_memory().unwrap();
        let result = cmd_record(
            &db,
            json!({
                "actor": "gemini",
                "project": "demo",
                "dry_run": true,
                "entries": [{"kind": "plan", "summary": "Plan only"}]
            }),
        )
        .unwrap();
        assert_eq!(result["status"], "dry_run");
        assert_eq!(result["count"], 1);
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn self_reference_closes_a_cycle() {
        let db = KnoxDb::open_memory().unwrap();
        assert!(matches!(
            record(
                &db,
                json!([{
                    "ref": "e1",
                    "kind": "action",
                    "summary": "Circular work",
                    "files": ["src/cycle.rs"],
                    "caused_by": "e1"
                }])
            ),
            Err(KnoxError::Cycle(_))
        ));
    }

    #[test]
    fn relation_to_another_project_is_rejected() {
        use crate::core::ids::ProjectId;
        use crate::core::types::Project;

        let db = KnoxDb::open_memory().unwrap();
        for (name, path) in [("alpha", "/work/alpha"), ("beta", "/work/beta")] {
            crate::storage::nodes::insert_project(
                &db,
                &Project {
                    id: ProjectId::new(),
                    name: name.to_string(),
                    path: path.to_string(),
                    remote: None,
                    created_at: Utc::now(),
                },
            )
            .unwrap();
        }

        let beta_session =
            crate::storage::sessions::start_session(&db, "gemini", Some("beta")).unwrap();
        let beta = cmd_record(
            &db,
            json!({
                "actor": "gemini",
                "project": "beta",
                "session_id": beta_session.id.as_str(),
                "entries": [{"kind": "plan", "summary": "Beta plan"}]
            }),
        )
        .unwrap();
        let beta_id = beta["ids"][0].as_str().unwrap().to_string();

        let alpha_session =
            crate::storage::sessions::start_session(&db, "claude", Some("alpha")).unwrap();
        let result = cmd_record(
            &db,
            json!({
                "actor": "claude",
                "project": "alpha",
                "session_id": alpha_session.id.as_str(),
                "entries": [{
                    "kind": "action",
                    "summary": "Alpha write",
                    "files": ["src/alpha.rs"],
                    "relations": [{"to": beta_id}]
                }]
            }),
        );
        assert!(matches!(result, Err(KnoxError::CrossProject(_))));
    }

    #[test]
    fn test_reversed_implements_is_rejected_and_logged() {
        let db = KnoxDb::open_memory().unwrap();
        let res = record(
            &db,
            json!([
                {"ref": "code", "kind": "write", "summary": "Wrote backend database module"},
                {
                    "ref": "dec",
                    "kind": "decide",
                    "summary": "Decided on SQLite",
                    "relations": [{"to": "code", "relation_type": "implements"}]
                }
            ]),
        );
        assert!(matches!(res, Err(KnoxError::InvalidRelation(_))));
        if let Err(KnoxError::InvalidRelation(msg)) = res {
            assert!(msg.contains("cannot implement another action"));
        }

        let rejections = crate::storage::edges::get_edge_rejections(&db, 10).unwrap();
        assert_eq!(rejections.len(), 1);
        assert_eq!(rejections[0].attempted_type, "implements");
        assert!(rejections[0].reason.contains("cannot implement another action"));
    }

    #[test]
    fn test_reversed_resolves_is_rejected_and_logged() {
        let db = KnoxDb::open_memory().unwrap();
        let res = record(
            &db,
            json!([
                {"ref": "code", "kind": "write", "summary": "Wrote backend database module"},
                {
                    "ref": "d",
                    "kind": "doubt",
                    "summary": "Is SQLite fast enough?",
                    "relations": [{"to": "code", "relation_type": "resolves"}]
                }
            ]),
        );
        assert!(matches!(res, Err(KnoxError::InvalidRelation(_))));
        if let Err(KnoxError::InvalidRelation(msg)) = res {
            assert!(msg.contains("a doubt cannot resolve an action"));
        }

        let rejections = crate::storage::edges::get_edge_rejections(&db, 10).unwrap();
        assert_eq!(rejections.len(), 1);
        assert_eq!(rejections[0].attempted_type, "resolves");
    }

    #[test]
    fn test_unknown_relation_type_is_rejected_and_logged() {
        let db = KnoxDb::open_memory().unwrap();
        let res = record(
            &db,
            json!([
                {"ref": "a1", "kind": "write", "summary": "Wrote first module"},
                {
                    "ref": "a2",
                    "kind": "write",
                    "summary": "Wrote second module",
                    "relations": [{"to": "a1", "relation_type": "bogus_link"}]
                }
            ]),
        );
        assert!(matches!(res, Err(KnoxError::InvalidRelation(_))));
        if let Err(KnoxError::InvalidRelation(msg)) = res {
            assert!(msg.contains("Unknown relation type 'bogus_link'"));
        }

        let rejections = crate::storage::edges::get_edge_rejections(&db, 10).unwrap();
        assert_eq!(rejections.len(), 1);
        assert_eq!(rejections[0].attempted_type, "bogus_link");
    }

    #[test]
    fn test_edge_to_file_ref_resolves_and_persists() {
        let db = KnoxDb::open_memory().unwrap();
        let res = record(
            &db,
            json!([
                {
                    "ref": "w",
                    "kind": "write",
                    "summary": "Implemented SQLite backend",
                    "relations": [{"to": "file:src/db.rs", "relation_type": "implements"}]
                }
            ]),
        )
        .unwrap();

        let action_id = res["ids"][0].as_str().unwrap();
        let ref_entry = crate::storage::edges::get_ref(&db, "file:src/db.rs").unwrap().unwrap();
        assert_eq!(ref_entry.id, "file:src/db.rs");
        assert_eq!(ref_entry.kind, "file");
        assert_eq!(ref_entry.value, "src/db.rs");

        let edges = crate::storage::edges::get_edges_for_action(
            &db,
            &ActionId::from(action_id.to_string()),
        )
        .unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to_id, "file:src/db.rs");
        assert_eq!(edges[0].edge_type, "implements");
    }
}
