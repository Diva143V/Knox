//! Transcript ingestion command for Knox.
//! Parses conversational transcripts and persists them into the knowledge graph.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::Path;

use rusqlite::params;

use crate::core::error::{KnoxError, KnoxResult};
use crate::core::types::{IngestPreview, IngestReport, KnoxAction};
use crate::ingest::{ingest_bytes_with_tool, ingest_file_with_tool, rules};
use crate::storage::db::KnoxDb;
use crate::storage::edges::link_follows;
use crate::storage::nodes::insert_action;
use crate::storage::sessions::{end_session, start_session};

/// Explanation of a single ingestion classification decision.
#[derive(Clone, Debug)]
pub struct IngestExplanation {
    /// Source offset of the transcript step.
    pub source_offset: usize,
    /// Rule ID that matched (e.g. "R01").
    pub rule_id: String,
    /// Confidence score (0.0–1.0).
    pub confidence: f64,
    /// Human-readable reason for the classification.
    pub reason: String,
    /// Action type produced, if any.
    pub action_type: Option<String>,
    /// Whether the entry was excluded from ingestion.
    pub excluded: bool,
}

/// Imports a conversation transcript into the Knox knowledge graph.
///
/// 1. Reads and maps the transcript into `KnoxAction` records via `ingest_file_with_tool`.
/// 2. Records an ingestion session in the database (associating with inferred project).
/// 3. Classifies each action against the R01–R10 rule table.
/// 4. Checks for duplicates and inserts each unique action into the database.
/// 5. Links consecutive actions within the session.
/// 6. Closes the session and returns an `IngestReport`.
///
/// When `explain` is true, per-action classification explanations are printed
/// to stderr.
pub fn cmd_ingest(
    transcript_path: &Path,
    db: &KnoxDb,
    explain: bool,
) -> KnoxResult<IngestReport> {
    let (tool_detected, actions) = ingest_file_with_tool(transcript_path)?;
    if actions.is_empty() {
        return Ok(IngestReport {
            tool_detected,
            ..Default::default()
        });
    }

    let existing_projects = crate::storage::list_projects(db).unwrap_or_default();
    let inferred_project = infer_project_path(transcript_path, &actions, &existing_projects);

    let report = persist_actions_with_session(
        actions,
        tool_detected,
        inferred_project.as_deref(),
        db,
    )?;

    if explain {
        eprintln!("{}", rules::rule_table_summary());
    }

    Ok(report)
}

/// Imports raw transcript content directly into the Knox knowledge graph.
pub fn cmd_ingest_raw(
    content: &str,
    db: &KnoxDb,
    actor_override: Option<&str>,
    explain: bool,
) -> KnoxResult<IngestReport> {
    let (mut tool_detected, mut actions) = ingest_bytes_with_tool(content.as_bytes())?;
    if actions.is_empty() {
        return Ok(IngestReport {
            tool_detected,
            ..Default::default()
        });
    }

    if let Some(actor) = actor_override {
        let trimmed = actor.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("auto") {
            tool_detected = trimmed.to_string();
            for a in &mut actions {
                a.actor = trimmed.to_string();
            }
        }
    }

    let report = persist_actions_with_session(actions, tool_detected, None, db)?;

    if explain {
        eprintln!("{}", rules::rule_table_summary());
    }

    Ok(report)
}

/// Ingests commit history from a Git repository into the Knox knowledge graph.
pub fn cmd_ingest_git(
    project_path: &Path,
    max_commits: usize,
    db: &KnoxDb,
) -> KnoxResult<IngestReport> {
    let actions = crate::ingest::git_log::run_git_log(project_path, max_commits)?;
    if actions.is_empty() {
        return Ok(IngestReport {
            tool_detected: "git".to_string(),
            ..Default::default()
        });
    }

    let project_str = project_path.to_string_lossy().to_string();
    persist_actions_with_session(actions, "git".to_string(), Some(&project_str), db)
}

/// Generates a dry-run preview of a transcript before committing to the database.
pub fn cmd_preview(
    path: Option<&Path>,
    content: Option<&str>,
    db: &KnoxDb,
) -> KnoxResult<IngestPreview> {
    let (tool_detected, actions) = if let Some(p) = path {
        ingest_file_with_tool(p)?
    } else if let Some(c) = content {
        ingest_bytes_with_tool(c.as_bytes())?
    } else {
        return Err(KnoxError::ParseError(
            "Neither path nor content provided for preview".to_string(),
        ));
    };

    let conn = db.conn();
    let mut plans_count = 0;
    let mut writes_count = 0;
    let mut debugs_count = 0;
    let mut decisions_count = 0;
    let mut files_set = HashSet::new();
    let mut duplicates_count = 0;

    for a in &actions {
        match a.action_type.as_str() {
            "plan" => plans_count += 1,
            "write" => writes_count += 1,
            "debug" => debugs_count += 1,
            "decide" => decisions_count += 1,
            _ => {}
        }

        for f in &a.files {
            files_set.insert(f.clone());
        }

        let is_dup = conn
            .query_row(
                "SELECT 1 FROM actions WHERE actor = ?1 AND action_type = ?2 AND summary = ?3 LIMIT 1",
                params![a.actor.as_str(), a.action_type.as_str(), a.summary.as_str()],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if is_dup {
            duplicates_count += 1;
        }
    }

    let sample_actions = actions.iter().take(6).cloned().collect();
    let mut files_touched: Vec<String> = files_set.into_iter().collect();
    files_touched.sort();

    Ok(IngestPreview {
        tool_detected,
        total_actions: actions.len(),
        plans_count,
        writes_count,
        debugs_count,
        decisions_count,
        files_touched,
        duplicates_count,
        sample_actions,
    })
}

/// Helper to compute deterministic hash for ingest logging.
pub fn ingest_source_hash(
    tool_detected: &str,
    project_scope: Option<&str>,
    offset: usize,
    summary: &str,
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    tool_detected.hash(&mut hasher);
    project_scope.unwrap_or_default().hash(&mut hasher);
    offset.hash(&mut hasher);
    summary.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Ensures the `ingest_offsets` table exists.
pub fn ensure_ingest_offsets_table(db: &KnoxDb) -> KnoxResult<()> {
    db.conn().execute(
        "CREATE TABLE IF NOT EXISTS ingest_offsets (
            transcript_path TEXT PRIMARY KEY,
            last_line_offset INTEGER NOT NULL DEFAULT 0,
            last_byte_offset INTEGER NOT NULL DEFAULT 0,
            last_ingested_at TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

/// Retrieves the last recorded (line_offset, byte_offset) for a transcript path.
pub fn get_ingest_offset(db: &KnoxDb, path_key: &str) -> KnoxResult<Option<(usize, usize)>> {
    ensure_ingest_offsets_table(db)?;
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT last_line_offset, last_byte_offset FROM ingest_offsets WHERE transcript_path = ?1",
    )?;
    let mut rows = stmt.query(params![path_key])?;
    if let Some(row) = rows.next()? {
        let line: i64 = row.get(0)?;
        let byte: i64 = row.get(1)?;
        Ok(Some((line as usize, byte as usize)))
    } else {
        Ok(None)
    }
}

/// Sets the recorded (line_offset, byte_offset) for a transcript path.
pub fn set_ingest_offset(
    db: &KnoxDb,
    path_key: &str,
    line_offset: usize,
    byte_offset: usize,
) -> KnoxResult<()> {
    ensure_ingest_offsets_table(db)?;
    let now = chrono::Utc::now().to_rfc3339();
    db.conn().execute(
        "INSERT INTO ingest_offsets (transcript_path, last_line_offset, last_byte_offset, last_ingested_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(transcript_path) DO UPDATE SET
            last_line_offset = excluded.last_line_offset,
            last_byte_offset = excluded.last_byte_offset,
            last_ingested_at = excluded.last_ingested_at",
        params![path_key, line_offset as i64, byte_offset as i64, now],
    )?;
    Ok(())
}

/// Prunes ingest log entries older than `max_age_days` days to prevent unbounded database growth.
pub fn prune_ingest_log(db: &KnoxDb, max_age_days: i64) -> KnoxResult<usize> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days);
    let cutoff_str = cutoff.to_rfc3339();
    let deleted = db.conn().execute(
        "DELETE FROM ingest_log
         WHERE action_id IS NULL
            OR action_id IN (SELECT id FROM actions WHERE timestamp < ?1)
            OR action_id NOT IN (SELECT id FROM actions)",
        params![cutoff_str],
    )?;
    Ok(deleted)
}

/// Core helper to persist parsed actions, link follows edges, and record session metrics.
/// If `active_session_id` is provided, actions are associated with it; otherwise an ingestion
/// session is created and ended.
pub fn persist_actions_to_db(
    mut actions: Vec<KnoxAction>,
    tool_detected: String,
    project_scope: Option<&str>,
    active_session_id: Option<&crate::core::ids::SessionId>,
    db: &KnoxDb,
) -> KnoxResult<IngestReport> {
    let (session_id, was_created) = match active_session_id {
        Some(id) => (id.clone(), false),
        None => {
            let session = start_session(db, &tool_detected, project_scope)?;
            (session.id, true)
        }
    };

    let mut files_touched = HashSet::new();
    let mut previous_action_id = None;
    let mut actions_imported = 0;
    let mut duplicates_skipped = 0;

    let conn = db.conn();

    // Resolve project ID if provided
    let project_id = if let Some(scope) = project_scope {
        crate::storage::sessions::resolve_project_identifier(db, scope)?
            .map(crate::core::ids::ProjectId::from)
    } else {
        None
    };

    for (offset, action) in actions.iter_mut().enumerate() {
        let source_hash = ingest_source_hash(
            &tool_detected,
            project_scope,
            offset,
            action.summary.as_str(),
        );

        // Smart deduplication check
        let existing_match = conn
            .query_row(
                "SELECT id, seen_count FROM actions WHERE actor = ?1 AND action_type = ?2 AND summary = ?3 LIMIT 1",
                params![action.actor.as_str(), action.action_type.as_str(), action.summary.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .ok();

        // Classify the action against the R01–R10 rule table.
        let classification = classify_action(action, offset);
        let (rule_id, reason) = classification.as_ref().map(|c| (c.rule_id.clone(), c.reason.as_str())).unzip();

        if let Some((existing_id, seen_count)) = existing_match {
            duplicates_skipped += 1;
            conn.execute(
                "UPDATE actions SET seen_count = ?1 WHERE id = ?2",
                params![seen_count + 1, existing_id],
            )?;
            conn.execute(
                "INSERT OR REPLACE INTO ingest_log
                 (source_hash, source_offset, outcome, rule_id, reason, action_id)
                 VALUES (?1, ?2, 'deduped', ?3, ?4, NULL)",
                params![
                    source_hash,
                    offset as i64,
                    rule_id,
                    reason,
                ],
            )?;
            continue;
        }

        if action.session_id.is_none() {
            action.session_id = Some(session_id.clone());
        }

        if action.project_id.is_none() && project_id.is_some() {
            action.project_id = project_id.clone();
        }

        for file in &action.files {
            files_touched.insert(file.clone());
        }

        let current_id = insert_action(db, action)?;
        actions_imported += 1;
        conn.execute(
            "INSERT OR REPLACE INTO ingest_log
             (source_hash, source_offset, outcome, rule_id, reason, action_id)
             VALUES (?1, ?2, 'imported', ?3, ?4, ?5)",
            params![
                source_hash,
                offset as i64,
                rule_id,
                reason,
                current_id.as_str()
            ],
        )?;

        if let Some(ref prev_id) = previous_action_id {
            link_follows(db, prev_id, &current_id)?;
        }
        previous_action_id = Some(current_id);
    }

    if was_created {
        end_session(db, &session_id)?;
    }

    Ok(IngestReport {
        actions_imported,
        files_touched: files_touched.len(),
        sessions_created: if was_created && actions_imported > 0 { 1 } else { 0 },
        duplicates_skipped,
        tool_detected,
    })
}

/// Ingests transcript lines since the last offset on begin, when a path is configured.
///
/// Ensures a session with zero `record` calls still produces memory.
pub fn cmd_ingest_incremental(
    transcript_path: &Path,
    db: &KnoxDb,
    active_session_id: Option<&crate::core::ids::SessionId>,
    project_scope: Option<&str>,
) -> KnoxResult<IngestReport> {
    if !transcript_path.exists() {
        return Ok(IngestReport::default());
    }

    let path_key = transcript_path
        .canonicalize()
        .unwrap_or_else(|_| transcript_path.to_path_buf())
        .to_string_lossy()
        .to_string();

    let full_bytes = std::fs::read(transcript_path)?;
    if full_bytes.is_empty() {
        return Ok(IngestReport::default());
    }

    let (mut last_line_offset, _) = get_ingest_offset(db, &path_key)?.unwrap_or((0, 0));

    let full_text = String::from_utf8_lossy(&full_bytes);
    let all_lines: Vec<&str> = full_text.lines().collect();
    let total_lines = all_lines.len();

    // Reset offset if file was truncated or rotated
    if total_lines < last_line_offset {
        last_line_offset = 0;
    }

    if total_lines <= last_line_offset {
        return Ok(IngestReport::default());
    }

    let new_lines_slice = &all_lines[last_line_offset..];
    let new_lines_text = new_lines_slice.join("\n");
    let new_lines_bytes = new_lines_text.as_bytes();

    // Map new lines into KnoxAction records
    let (tool_detected, actions) = match ingest_bytes_with_tool(new_lines_bytes) {
        Ok(res) => res,
        Err(_) => {
            // If new lines alone lack tool headers (e.g. middle of transcript), detect tool from full file
            if let Ok((detected, full_actions)) = ingest_bytes_with_tool(&full_bytes) {
                let mut found_actions = Vec::new();
                for mapper in crate::ingest::all_mappers() {
                    if mapper.tool_name().eq_ignore_ascii_case(&detected) {
                        if let Ok(mapped) = mapper.map(new_lines_bytes) {
                            found_actions = mapped;
                            break;
                        }
                    }
                }
                if found_actions.is_empty() && last_line_offset > 0 {
                    // For structured JSON exports (e.g. Cursor, OpenAI), map full document
                    // and keep only actions not yet recorded in SQLite
                    found_actions = full_actions
                        .into_iter()
                        .filter(|action| {
                            let is_dup = db
                                .conn()
                                .query_row(
                                    "SELECT 1 FROM actions WHERE actor = ?1 AND action_type = ?2 AND summary = ?3 LIMIT 1",
                                    params![action.actor.as_str(), action.action_type.as_str(), action.summary.as_str()],
                                    |_| Ok(true),
                                )
                                .unwrap_or(false);
                            !is_dup
                        })
                        .collect();
                }
                (detected, found_actions)
            } else {
                ("generic".to_string(), Vec::new())
            }
        }
    };

    if actions.is_empty() {
        set_ingest_offset(db, &path_key, total_lines, full_bytes.len())?;
        return Ok(IngestReport {
            tool_detected,
            ..Default::default()
        });
    }

    // Determine project scope
    let existing_projects = crate::storage::list_projects(db).unwrap_or_default();
    let inferred = infer_project_path(transcript_path, &actions, &existing_projects);
    let resolved_scope = project_scope.or(inferred.as_deref());

    let report = persist_actions_to_db(
        actions,
        tool_detected,
        resolved_scope,
        active_session_id,
        db,
    )?;

    set_ingest_offset(db, &path_key, total_lines, full_bytes.len())?;

    Ok(report)
}

/// Common helper to persist actions, link follows edges, and record session metrics.
fn persist_actions_with_session(
    actions: Vec<KnoxAction>,
    tool_detected: String,
    project_scope: Option<&str>,
    db: &KnoxDb,
) -> KnoxResult<IngestReport> {
    persist_actions_to_db(actions, tool_detected, project_scope, None, db)
}

/// Classifies a persisted action against the R01–R10 rule table based on its
/// properties, returning the matching rule and reason.
fn classify_action(action: &KnoxAction, _offset: usize) -> Option<rules::RuleMatch> {
    let action_type = action.action_type.as_str();
    let has_files = !action.files.is_empty();
    let _confidence = action.confidence.unwrap_or(0.0);

    match action_type {
        "write" => Some(rules::RuleMatch {
            rule_id: "R01".to_string(),
            confidence: 0.90,
            reason: "tool call writes or edits a file".to_string(),
            action_type: Some("write".to_string()),
            excluded: false,
        }),
        "debug" => Some(rules::RuleMatch {
            rule_id: "R10".to_string(),
            confidence: 0.60,
            reason: "error output with exit code or stack trace".to_string(),
            action_type: Some("debug".to_string()),
            excluded: false,
        }),
        "decide" => {
            let confidence = if has_files { 0.80 } else { 0.50 };
            Some(rules::RuleMatch {
                rule_id: "R05".to_string(),
                confidence,
                reason: format!(
                    "explicit decision phrasing (file refs: {})",
                    has_files
                ),
                action_type: Some("decide".to_string()),
                excluded: false,
            })
        }
        "plan" => Some(rules::RuleMatch {
            rule_id: "R04".to_string(),
            confidence: 0.60,
            reason: "user imperative with task verb and object".to_string(),
            action_type: Some("plan".to_string()),
            excluded: false,
        }),
        "doubt" => Some(rules::RuleMatch {
            rule_id: "R06".to_string(),
            confidence: 0.70,
            reason: "question classified as doubt".to_string(),
            action_type: Some("doubt".to_string()),
            excluded: false,
        }),
        "dialogue" => Some(rules::RuleMatch {
            rule_id: "R07".to_string(),
            confidence: 0.0,
            reason: "acknowledgement or greeting; excluded as chatter".to_string(),
            action_type: None,
            excluded: true,
        }),
        _ => None,
    }
}

/// Attempts to infer the project root directory from transcript path or action file paths,
/// matching against existing tracked projects if possible.
fn infer_project_path(
    transcript_path: &Path,
    actions: &[crate::core::types::KnoxAction],
    existing_projects: &[crate::core::types::Project],
) -> Option<String> {
    let trans_str = transcript_path.to_string_lossy().to_string();

    // 1. Check if transcript path or any action files explicitly match an existing project
    for proj in existing_projects {
        let norm_proj_path = proj.path.replace('\\', "/").to_lowercase();
        let norm_trans = trans_str.replace('\\', "/").to_lowercase();
        if !norm_proj_path.is_empty() && norm_trans.contains(&norm_proj_path) {
            return Some(proj.path.clone());
        }
        for action in actions {
            for file in &action.files {
                let norm_f = file.replace('\\', "/").to_lowercase();
                if !norm_proj_path.is_empty() && norm_f.contains(&norm_proj_path) {
                    return Some(proj.path.clone());
                }
            }
        }
    }

    // 2. If exactly one project already exists, attach to it by default
    if existing_projects.len() == 1 {
        return Some(existing_projects[0].path.clone());
    }

    // 3. Try to find common absolute path from action files
    for action in actions {
        for file in &action.files {
            let p = Path::new(file);
            if p.is_absolute() {
                if let Some(parent) = p.parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    let clean = parent_str
                        .strip_prefix(r"\\?\")
                        .unwrap_or(&parent_str)
                        .to_string();
                    if !clean.is_empty() {
                        return Some(clean);
                    }
                }
            }
        }
    }

    // 4. Derive from transcript_path parent, skipping internal subdirectories
    let mut curr = transcript_path.parent();
    while let Some(dir) = curr {
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.is_empty()
            && !name.starts_with('.')
            && name != "logs"
            && name != "fixtures"
            && name != "brain"
            && name != "target"
        {
            let path_str = dir.to_string_lossy().to_string();
            let clean = path_str
                .strip_prefix(r"\\?\")
                .unwrap_or(&path_str)
                .to_string();
            if !clean.is_empty() {
                return Some(clean);
            }
        }
        curr = dir.parent();
    }

    // Fallback: direct parent of transcript_path
    transcript_path.parent().map(|p| {
        let s = p.to_string_lossy().to_string();
        s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
    })
}

/// Formats an `IngestReport` for friendly CLI output.
pub fn format_ingest_report(report: &IngestReport) -> String {
    let mut msg = format!(
        "Ingested {} actions across {} touched files ({} session created, format: {})",
        report.actions_imported,
        report.files_touched,
        report.sessions_created,
        report.tool_detected
    );
    if report.duplicates_skipped > 0 {
        msg.push_str(&format!(
            ", skipped {} duplicate(s)",
            report.duplicates_skipped
        ));
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cmd_ingest_with_antigravity_fixture() {
        let db = KnoxDb::open_memory().unwrap();
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("transcript_antigravity.jsonl");

        let report = cmd_ingest(&fixture_path, &db, false).unwrap();
        assert_eq!(report.actions_imported, 6);
        assert_eq!(report.files_touched, 2);
        assert_eq!(report.sessions_created, 1);
        assert_eq!(report.tool_detected, "antigravity");
        let imported_logs: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM ingest_log WHERE outcome = 'imported'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(imported_logs, 6);

        // Re-ingest the same fixture -> duplicates should be skipped!
        let second_report = cmd_ingest(&fixture_path, &db, false).unwrap();
        assert_eq!(second_report.actions_imported, 0);
        assert_eq!(second_report.duplicates_skipped, 6);
        let deduped_logs: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM ingest_log WHERE outcome = 'deduped'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(deduped_logs, 6);
    }

    #[test]
    fn test_cmd_preview_and_ingest_raw() {
        let db = KnoxDb::open_memory().unwrap();
        let raw_md = "## User\nBuild universal ingest\n\n## Assistant\nCreated universal mappers in src/ingest/";

        let preview = cmd_preview(None, Some(raw_md), &db).unwrap();
        assert_eq!(preview.total_actions, 2);
        assert_eq!(preview.duplicates_count, 0);

        let report = cmd_ingest_raw(raw_md, &db, Some("custom-agent"), false).unwrap();
        assert_eq!(report.actions_imported, 2);
        assert_eq!(report.tool_detected, "custom-agent");
    }
}
