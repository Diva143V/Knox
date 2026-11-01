//! Action group clustering state machine for transcript ingestion.
//!
//! Groups transcript events into intent-aligned action clusters bounded by
//! `USER_INPUT` entries, emitting 1-3 `KnoxAction` records per cluster.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::summary::{
    DebugExecution, FileModification, HumanInputKind, classify_human_input, parse_debug_execution,
    parse_file_modification, summarize_debug, summarize_decide, summarize_plan, summarize_write,
};
use crate::core::error::{KnoxError, KnoxResult};
use crate::core::ids::{ActionId, SessionId};
use crate::core::types::KnoxAction;

/// Operational state of the action grouping state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrouperState {
    /// Waiting for the first user input boundary.
    Idle,
    /// Currently collecting response events for the active group.
    Collecting,
}

/// A parsed line or step from an IDE conversation transcript.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TranscriptStep {
    /// Monotonically increasing step index.
    pub step_index: Option<i64>,
    /// Source origin (e.g. "USER_EXPLICIT", "MODEL", "SYSTEM").
    pub source: Option<String>,
    /// Step type (e.g. "USER_INPUT", "PLANNER_RESPONSE", "GENERIC").
    #[serde(rename = "type")]
    pub step_type: Option<String>,
    /// Execution status.
    pub status: Option<String>,
    /// RFC 3339 timestamp when the step occurred.
    pub created_at: Option<String>,
    /// Text content of the step, if any.
    pub content: Option<String>,
    /// Model internal reasoning, if any.
    pub thinking: Option<String>,
    /// Tool calls invoked during this step.
    pub tool_calls: Option<Vec<TranscriptToolCall>>,
}

/// A single tool call invocation inside a transcript step.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TranscriptToolCall {
    /// Name of the tool invoked (e.g. "edit_file", "run_command").
    pub name: String,
    /// Arguments payload supplied to the tool.
    pub args: Option<Value>,
}

/// Internal data container for the in-flight action group.
#[derive(Clone, Debug, Default)]
struct CollectedGroup {
    human_plan: Option<KnoxAction>,
    write_mods: Vec<FileModification>,
    write_files: Vec<String>,
    first_write_timestamp: Option<DateTime<Utc>>,
    debug_execs: Vec<DebugExecution>,
    first_debug_timestamp: Option<DateTime<Utc>>,
    decision_content: Option<String>,
    decision_timestamp: Option<DateTime<Utc>>,
    default_timestamp: DateTime<Utc>,
}

/// State machine that clusters transcript steps into consolidated Knox actions.
pub struct ActionGrouper {
    state: GrouperState,
    agent_actor: String,
    session_id: Option<SessionId>,
    current_group: Option<CollectedGroup>,
}

impl ActionGrouper {
    /// Creates a new action grouper for the specified agent actor name.
    pub fn new(agent_actor: impl Into<String>) -> Self {
        Self {
            state: GrouperState::Idle,
            agent_actor: agent_actor.into(),
            session_id: None,
            current_group: None,
        }
    }

    /// Sets the optional session identifier for actions emitted by this grouper.
    pub fn with_session_id(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Returns the current state of the grouper state machine.
    pub fn state(&self) -> GrouperState {
        self.state
    }

    /// Ingests a single parsed transcript step, returning any actions flushed on boundary transition.
    pub fn process_step(&mut self, step: &TranscriptStep) -> Vec<KnoxAction> {
        let step_type = step.step_type.as_deref().unwrap_or_default();
        let timestamp = parse_step_timestamp(step.created_at.as_deref());

        match step_type {
            "USER_INPUT" => self.handle_user_input(step, timestamp),
            "PLANNER_RESPONSE" => {
                self.handle_planner_response(step, timestamp);
                Vec::new()
            }
            "GENERIC" => {
                self.handle_generic_output(step);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// Parses and processes a single raw JSON transcript line.
    pub fn process_line(&mut self, line: &str) -> KnoxResult<Vec<KnoxAction>> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let step: TranscriptStep = serde_json::from_str(trimmed).map_err(|err| {
            KnoxError::ParseError(format!("Failed to parse transcript line: {err}"))
        })?;

        Ok(self.process_step(&step))
    }

    /// Finalizes the stream and flushes any pending actions from the current group.
    pub fn finish(&mut self) -> Vec<KnoxAction> {
        self.state = GrouperState::Idle;
        self.flush_current_group()
    }

    /// Handles boundary transition when a USER_INPUT step arrives.
    fn handle_user_input(
        &mut self,
        step: &TranscriptStep,
        timestamp: DateTime<Utc>,
    ) -> Vec<KnoxAction> {
        let emitted = self.flush_current_group();

        let content_str = step.content.as_deref().unwrap_or_default();
        let plan_summary = summarize_plan(content_str);
        let kind = classify_human_input(content_str);

        let (action_type, is_resolved, tags) = match kind {
            HumanInputKind::Doubt => (
                "doubt".to_string(),
                Some(false),
                vec!["user-doubt".to_string(), "open-question".to_string()],
            ),
            HumanInputKind::Dialogue => (
                "dialogue".to_string(),
                Some(true),
                vec!["conversation".to_string()],
            ),
            HumanInputKind::Plan => ("plan".to_string(), Some(true), vec!["plan".to_string()]),
        };

        let human_plan = KnoxAction {
            id: ActionId::new(),
            actor: "human".to_string(),
            action_type,
            summary: plan_summary,
            files: Vec::new(),
            tags,
            confidence: None,
            timestamp,
            session_id: self.session_id.clone(),
            parent_action_id: None,
            supersedes: None,
            is_current: true,
            is_resolved,
            ..Default::default()
        };

        self.current_group = Some(CollectedGroup {
            human_plan: Some(human_plan),
            default_timestamp: timestamp,
            ..Default::default()
        });
        self.state = GrouperState::Collecting;

        emitted
    }

    /// Collects actions from a PLANNER_RESPONSE step.
    fn handle_planner_response(&mut self, step: &TranscriptStep, timestamp: DateTime<Utc>) {
        if self.current_group.is_none() {
            self.current_group = Some(CollectedGroup {
                default_timestamp: timestamp,
                ..Default::default()
            });
            self.state = GrouperState::Collecting;
        }

        let group = match self.current_group.as_mut() {
            Some(g) => g,
            None => return,
        };

        if let Some(tool_calls) = &step.tool_calls {
            for call in tool_calls {
                let args = call.args.as_ref().unwrap_or(&Value::Null);

                if let Some(file_mod) = parse_file_modification(&call.name, args) {
                    if !group.write_files.contains(&file_mod.path) {
                        group.write_files.push(file_mod.path.clone());
                    }
                    if group.first_write_timestamp.is_none() {
                        group.first_write_timestamp = Some(timestamp);
                    }
                    group.write_mods.push(file_mod);
                } else if let Some(debug_exec) = parse_debug_execution(&call.name, args, None) {
                    if group.first_debug_timestamp.is_none() {
                        group.first_debug_timestamp = Some(timestamp);
                    }
                    group.debug_execs.push(debug_exec);
                }
            }
        }

        // If no tool calls and content is present, record decision text
        let has_tool_calls = step.tool_calls.as_ref().is_some_and(|tc| !tc.is_empty());
        if !has_tool_calls {
            if let Some(content) = &step.content {
                if !content.trim().is_empty() && group.decision_content.is_none() {
                    group.decision_content = Some(content.clone());
                    group.decision_timestamp = Some(timestamp);
                }
            }
        }
    }

    /// Handles generic command output to extract process exit codes if present.
    fn handle_generic_output(&mut self, step: &TranscriptStep) {
        if let Some(group) = self.current_group.as_mut() {
            if let Some(content) = &step.content {
                if let Some(exit_code) = extract_exit_code(content) {
                    if let Some(last_exec) = group.debug_execs.last_mut() {
                        if last_exec.exit_code.is_none() {
                            last_exec.exit_code = Some(exit_code);
                        }
                    }
                }
            }
        }
    }

    /// Flushes the current in-flight group into 1-3 Knox actions.
    fn flush_current_group(&mut self) -> Vec<KnoxAction> {
        let Some(mut group) = self.current_group.take() else {
            return Vec::new();
        };

        let mut actions = Vec::new();
        let plan_id = group.human_plan.take().map(|plan| {
            let id = plan.id.clone();
            actions.push(plan);
            id
        });

        let has_writes = !group.write_mods.is_empty();
        let has_debugs = !group.debug_execs.is_empty();

        if has_writes {
            actions.push(self.build_write_action(&plan_id, &group));
        }

        if has_debugs {
            actions.push(self.build_debug_action(&plan_id, &group));
        }

        if !has_writes && !has_debugs {
            if let Some(decide_action) = self.build_decide_action(&plan_id, &group) {
                actions.push(decide_action);
            }
        }

        actions
    }

    /// Constructs a write action from the collected file modifications in a group.
    fn build_write_action(&self, plan_id: &Option<ActionId>, group: &CollectedGroup) -> KnoxAction {
        let summary = summarize_write(&group.write_mods);
        let timestamp = group
            .first_write_timestamp
            .unwrap_or(group.default_timestamp);
        KnoxAction {
            id: ActionId::new(),
            actor: self.agent_actor.clone(),
            action_type: "write".to_string(),
            summary,
            files: group.write_files.clone(),
            tags: Vec::new(),
            confidence: Some(0.9),
            timestamp,
            session_id: self.session_id.clone(),
            parent_action_id: plan_id.clone(),
            supersedes: None,
            is_current: true,
            ..Default::default()
        }
    }

    /// Constructs a debug action from the collected command executions in a group.
    fn build_debug_action(&self, plan_id: &Option<ActionId>, group: &CollectedGroup) -> KnoxAction {
        let summary = summarize_debug(&group.debug_execs);
        let timestamp = group
            .first_debug_timestamp
            .unwrap_or(group.default_timestamp);
        KnoxAction {
            id: ActionId::new(),
            actor: self.agent_actor.clone(),
            action_type: "debug".to_string(),
            summary,
            files: Vec::new(),
            tags: Vec::new(),
            confidence: None,
            timestamp,
            session_id: self.session_id.clone(),
            parent_action_id: plan_id.clone(),
            supersedes: None,
            is_current: true,
            ..Default::default()
        }
    }

    /// Constructs an optional decide action from reasoning text if present.
    fn build_decide_action(
        &self,
        plan_id: &Option<ActionId>,
        group: &CollectedGroup,
    ) -> Option<KnoxAction> {
        let content = group.decision_content.as_ref()?;
        let summary = summarize_decide(content);
        if summary.trim().is_empty() {
            return None;
        }

        let timestamp = group.decision_timestamp.unwrap_or(group.default_timestamp);
        Some(KnoxAction {
            id: ActionId::new(),
            actor: self.agent_actor.clone(),
            action_type: "decide".to_string(),
            summary,
            files: Vec::new(),
            tags: Vec::new(),
            confidence: None,
            timestamp,
            session_id: self.session_id.clone(),
            parent_action_id: plan_id.clone(),
            supersedes: None,
            is_current: true,
            ..Default::default()
        })
    }
}

/// Parses an RFC 3339 timestamp string or defaults to Utc::now().
fn parse_step_timestamp(raw: Option<&str>) -> DateTime<Utc> {
    match raw {
        Some(s) => DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        None => Utc::now(),
    }
}

/// Attempts to extract an integer exit code from tool output text.
fn extract_exit_code(content: &str) -> Option<i32> {
    // Pattern: "exited with code 0" or "exit code: 0"
    for phrase in &["exited with code", "exit code:", "exit:"] {
        if let Some(idx) = content.to_lowercase().find(phrase) {
            let after = &content[idx + phrase.len()..];
            let number_str: String = after
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '-')
                .collect();
            if let Ok(code) = number_str.parse::<i32>() {
                return Some(code);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool_step(index: i64, name: &str, args: Value, time: &str) -> TranscriptStep {
        TranscriptStep {
            step_index: Some(index),
            source: Some("MODEL".to_string()),
            step_type: Some("PLANNER_RESPONSE".to_string()),
            tool_calls: Some(vec![TranscriptToolCall {
                name: name.to_string(),
                args: Some(args),
            }]),
            created_at: Some(time.to_string()),
            ..Default::default()
        }
    }

    fn user_step(text: &str) -> TranscriptStep {
        TranscriptStep {
            step_index: Some(0),
            source: Some("USER_EXPLICIT".to_string()),
            step_type: Some("USER_INPUT".to_string()),
            content: Some(text.to_string()),
            created_at: Some("2026-10-29T12:00:00Z".to_string()),
            ..Default::default()
        }
    }

    fn generic_step(text: &str) -> TranscriptStep {
        TranscriptStep {
            step_index: Some(3),
            source: Some("MODEL".to_string()),
            step_type: Some("GENERIC".to_string()),
            content: Some(text.to_string()),
            created_at: Some("2026-10-29T12:00:15Z".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_action_grouper_write_and_debug_flow() {
        let mut grouper = ActionGrouper::new("antigravity");

        let step1 = user_step("Refactor parser module");
        assert!(grouper.process_step(&step1).is_empty());
        assert_eq!(grouper.state(), GrouperState::Collecting);

        let step2 = tool_step(
            1,
            "edit_file",
            json!({"Path": "src/parser.rs", "Target": "old", "Replacement": "new"}),
            "2026-10-29T12:00:05Z",
        );
        grouper.process_step(&step2);

        let step3 = tool_step(
            2,
            "run_command",
            json!({"CommandLine": "cargo test"}),
            "2026-10-29T12:00:10Z",
        );
        grouper.process_step(&step3);

        let step4 = generic_step("Command exited with code 0.\ntest ok");
        grouper.process_step(&step4);

        let actions = grouper.finish();
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0].action_type, "plan");
        let plan_id = actions[0].id.clone();

        assert_eq!(actions[1].action_type, "write");
        assert_eq!(actions[1].parent_action_id, Some(plan_id.clone()));

        assert_eq!(actions[2].action_type, "debug");
        assert_eq!(actions[2].parent_action_id, Some(plan_id));
    }

    #[test]
    fn test_action_grouper_decide_flow() {
        let mut grouper = ActionGrouper::new("antigravity");

        grouper.process_step(&TranscriptStep {
            step_index: Some(0),
            source: Some("USER_EXPLICIT".to_string()),
            step_type: Some("USER_INPUT".to_string()),
            content: Some("Which architecture is better: monolithic or layered?".to_string()),
            ..Default::default()
        });

        grouper.process_step(&TranscriptStep {
            step_index: Some(1),
            source: Some("MODEL".to_string()),
            step_type: Some("PLANNER_RESPONSE".to_string()),
            content: Some(
                "Layered architecture is preferred for strict dependency boundaries.".to_string(),
            ),
            ..Default::default()
        });

        let actions = grouper.finish();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action_type, "doubt");
        assert_eq!(actions[0].is_resolved, Some(false));
        assert_eq!(actions[1].action_type, "decide");
        assert_eq!(
            actions[1].summary,
            "Layered architecture is preferred for strict dependency boundaries."
        );
        assert_eq!(actions[1].parent_action_id, Some(actions[0].id.clone()));
    }

    #[test]
    fn test_action_grouper_dialogue_classification() {
        let mut grouper = ActionGrouper::new("antigravity");

        grouper.process_step(&TranscriptStep {
            step_index: Some(0),
            source: Some("USER_EXPLICIT".to_string()),
            step_type: Some("USER_INPUT".to_string()),
            content: Some("ok sounds good".to_string()),
            ..Default::default()
        });

        let actions = grouper.finish();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, "dialogue");
        assert_eq!(actions[0].is_resolved, Some(true));
    }
}
