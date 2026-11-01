//! Heuristic summary generators for Knox transcript actions.
//! Generates concise summaries without requiring an LLM.

use serde_json::Value;

/// Representation of a file modification detected during transcript analysis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileModification {
    /// Relative path of the modified file.
    pub path: String,
    /// Number of lines added.
    pub lines_added: usize,
    /// Number of lines removed.
    pub lines_removed: usize,
    /// Whether this file was created as a new file.
    pub is_new: bool,
}

/// Representation of a debug command execution detected during transcript analysis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugExecution {
    /// Command line string that was executed.
    pub command: String,
    /// Process exit code, if known.
    pub exit_code: Option<i32>,
}

/// Classification of a human conversation input during transcript ingestion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HumanInputKind {
    /// An inquiry, question, doubt, or uncertainty.
    Doubt,
    /// A directive, instruction, plan, or task.
    Plan,
    /// General dialogue, clarification, or conversational exchange.
    Dialogue,
}

/// Classifies a user input string into a Doubt, Plan, or Dialogue.
pub fn classify_human_input(content: &str) -> HumanInputKind {
    let clean = extract_user_request_text(content).trim();
    if clean.is_empty() {
        return HumanInputKind::Plan;
    }

    let lower = clean.to_ascii_lowercase();

    // Check for explicit commands or slash commands
    if lower.contains("/goal")
        || lower.contains("/plan")
        || lower.contains("/fix")
        || lower.contains("/build")
        || lower.starts_with("implement")
        || lower.starts_with("build")
        || lower.starts_with("create")
        || lower.starts_with("add ")
        || lower.starts_with("refactor")
        || lower.starts_with("fix ")
        || lower.starts_with("delete ")
        || lower.starts_with("remove ")
    {
        return HumanInputKind::Plan;
    }

    // Check polite instruction prefixes: "can you ...", "could you ...", "please ..."
    let words: Vec<&str> = lower
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();

    if words.len() >= 2 {
        let first_two = (words[0], words[1]);
        if first_two == ("can", "you")
            || first_two == ("could", "you")
            || first_two == ("would", "you")
            || first_two == ("will", "you")
            || words[0] == "please"
        {
            return HumanInputKind::Plan;
        }
    }

    // Check question starters: "can i ...", "what ...", "why ...", "how ...", "is it ...", "should we ..."
    let question_starters = [
        "what", "why", "how", "where", "when", "which", "who", "whom", "should", "would", "is",
        "are", "was", "were",
    ];
    if let Some(&first) = words.first() {
        if question_starters.contains(&first) {
            return HumanInputKind::Doubt;
        }
        if (first == "can" || first == "could")
            && words.get(1).is_some_and(|w| matches!(*w, "i" | "we"))
        {
            return HumanInputKind::Doubt;
        }
        if (first == "do" || first == "does" || first == "did")
            && words
                .get(1)
                .is_some_and(|w| matches!(*w, "we" | "i" | "you" | "they" | "it"))
        {
            return HumanInputKind::Doubt;
        }
    }

    // Check if ends with '?' or contains question / doubt phrases
    let trimmed_end = clean.trim_end_matches(&[' ', '\r', '\n', '`', '*', '"', '\'', '.'][..]);
    if trimmed_end.ends_with('?') || clean.contains('?') {
        return HumanInputKind::Doubt;
    }

    let doubt_phrases = [
        "i wonder",
        "not sure",
        "doubt",
        "wondering",
        "confused",
        "is it possible",
        "should we",
        "can we",
        "what if",
        "what happens",
    ];
    if doubt_phrases.iter().any(|&phrase| lower.contains(phrase)) {
        return HumanInputKind::Doubt;
    }

    // Conversational acknowledgments, greetings, or short confirmations
    let conversational_words = [
        "ok",
        "okay",
        "yes",
        "no",
        "sure",
        "thanks",
        "thank",
        "sounds",
        "apply",
        "proceed",
        "continue",
        "done",
        "hello",
        "hi",
        "hey",
        "yep",
        "nope",
        "acknowledged",
        "cool",
        "great",
        "nice",
    ];
    if words.len() <= 3 && words.iter().any(|w| conversational_words.contains(w)) {
        return HumanInputKind::Dialogue;
    }

    // Default to Plan for descriptive tasks and directives
    HumanInputKind::Plan
}

/// Generates a summary for a user plan action, taking at most 200 characters.
pub fn summarize_plan(content: &str) -> String {
    let clean = extract_user_request_text(content);
    truncate_chars(clean.trim(), 200)
}

/// Generates a summary for a write action from file modifications.
/// Example format: `"src/parser.rs (+42 -15), src/grammar.rs (new)"`.
pub fn summarize_write(mods: &[FileModification]) -> String {
    if mods.is_empty() {
        return "Files modified".to_string();
    }

    let parts: Vec<String> = mods
        .iter()
        .map(|m| {
            if m.is_new {
                format!("{} (new)", m.path)
            } else if m.lines_added > 0 && m.lines_removed > 0 {
                format!("{} (+{} -{})", m.path, m.lines_added, m.lines_removed)
            } else if m.lines_added > 0 {
                format!("{} (+{})", m.path, m.lines_added)
            } else if m.lines_removed > 0 {
                format!("{} (-{})", m.path, m.lines_removed)
            } else {
                m.path.clone()
            }
        })
        .collect();

    parts.join(", ")
}

/// Generates a summary for a debug action from command executions.
/// Example format: `"cargo test -- --nocapture (exit: 0)"`.
pub fn summarize_debug(executions: &[DebugExecution]) -> String {
    if executions.is_empty() {
        return "Command executed".to_string();
    }

    let parts: Vec<String> = executions
        .iter()
        .map(|exec| {
            let code = exec.exit_code.unwrap_or(0);
            format!("{} (exit: {})", exec.command, code)
        })
        .collect();

    parts.join(", ")
}

/// Generates a summary for an agent decision action, taking at most 200 characters.
pub fn summarize_decide(content: &str) -> String {
    truncate_chars(content.trim(), 200)
}

/// Normalizes file paths to clean relative paths with forward slashes.
pub fn normalize_file_path(path: &str) -> String {
    let replaced = path.replace('\\', "/");
    let without_drive = match replaced.find(':') {
        Some(idx) if idx <= 2 => &replaced[idx + 1..],
        _ => &replaced[..],
    };

    let segments: Vec<&str> = without_drive
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();

    // If 'knox' appears as a folder component, take relative path after it
    if let Some(pos) = segments.iter().position(|s| s.eq_ignore_ascii_case("knox")) {
        if pos + 1 < segments.len() {
            return segments[pos + 1..].join("/");
        }
    }

    segments.join("/")
}

/// Extracts a string argument from a JSON value checking multiple key candidates.
pub fn extract_tool_args_string(args: &Value, keys: &[&str]) -> Option<String> {
    match args {
        Value::Object(map) => {
            for key in keys {
                for (k, v) in map {
                    if k.eq_ignore_ascii_case(key) {
                        return match v {
                            Value::String(s) => Some(clean_enclosed_quotes(s)),
                            _ => Some(v.to_string()),
                        };
                    }
                }
            }
            None
        }
        Value::String(raw_json) => {
            if let Ok(parsed) = serde_json::from_str::<Value>(raw_json) {
                extract_tool_args_string(&parsed, keys)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parses a file modification from tool name and arguments.
pub fn parse_file_modification(tool_name: &str, args: &Value) -> Option<FileModification> {
    match tool_name {
        "edit_file" | "edit" => {
            let path_raw = extract_tool_args_string(args, &["Path", "path", "file", "file_path"])?;
            let path = normalize_file_path(&path_raw);
            let target = extract_tool_args_string(args, &["Target", "target", "old_string"])
                .unwrap_or_default();
            let replacement =
                extract_tool_args_string(args, &["Replacement", "replacement", "new_string"])
                    .unwrap_or_default();

            let lines_removed = if target.is_empty() {
                0
            } else {
                target.lines().count()
            };
            let lines_added = if replacement.is_empty() {
                0
            } else {
                replacement.lines().count()
            };

            Some(FileModification {
                path,
                lines_added,
                lines_removed,
                is_new: false,
            })
        }
        "write_file" | "create_file" | "write" => {
            let path_raw = extract_tool_args_string(args, &["Path", "path", "file", "file_path"])?;
            let path = normalize_file_path(&path_raw);
            let content =
                extract_tool_args_string(args, &["Content", "content"]).unwrap_or_default();
            let lines_added = if content.is_empty() {
                0
            } else {
                content.lines().count()
            };

            Some(FileModification {
                path,
                lines_added,
                lines_removed: 0,
                is_new: true,
            })
        }
        _ => None,
    }
}

/// Parses a debug execution from tool name and arguments.
pub fn parse_debug_execution(
    tool_name: &str,
    args: &Value,
    exit_code: Option<i32>,
) -> Option<DebugExecution> {
    match tool_name {
        "run_command" | "exec_command" | "execute_command" | "command" | "terminal" | "bash" => {
            let cmd_raw =
                extract_tool_args_string(args, &["CommandLine", "command_line", "command", "cmd"])?;
            let command = clean_enclosed_quotes(&cmd_raw);
            Some(DebugExecution { command, exit_code })
        }
        _ => None,
    }
}

/// Truncates string to at most max_len Unicode characters safely.
fn truncate_chars(s: &str, max_len: usize) -> String {
    s.chars().take(max_len).collect()
}

/// Strips outer quotes if the string was doubly quoted.
fn clean_enclosed_quotes(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Extracts primary user request text, ignoring metadata wrappers if present.
fn extract_user_request_text(content: &str) -> &str {
    if let Some(start) = content.find("<USER_REQUEST>") {
        let after_start = &content[start + "<USER_REQUEST>".len()..];
        if let Some(end) = after_start.find("</USER_REQUEST>") {
            return &after_start[..end];
        }
    }

    if let Some(meta_start) = content.find("<ADDITIONAL_METADATA>") {
        return &content[..meta_start];
    }

    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_summarize_plan_with_user_request_wrapper() {
        let input = "<USER_REQUEST>\nImplement token streaming in parser\n</USER_REQUEST>\n<ADDITIONAL_METADATA>\ninfo\n</ADDITIONAL_METADATA>";
        let summary = summarize_plan(input);
        assert_eq!(summary, "Implement token streaming in parser");
    }

    #[test]
    fn test_summarize_plan_truncation() {
        let long_input = "a".repeat(300);
        let summary = summarize_plan(&long_input);
        assert_eq!(summary.chars().count(), 200);
    }

    #[test]
    fn test_summarize_write_formatting() {
        let mods = vec![
            FileModification {
                path: "src/parser.rs".to_string(),
                lines_added: 42,
                lines_removed: 15,
                is_new: false,
            },
            FileModification {
                path: "src/grammar.rs".to_string(),
                lines_added: 30,
                lines_removed: 0,
                is_new: true,
            },
        ];

        let summary = summarize_write(&mods);
        assert_eq!(summary, "src/parser.rs (+42 -15), src/grammar.rs (new)");
    }

    #[test]
    fn test_summarize_debug_formatting() {
        let executions = vec![DebugExecution {
            command: "cargo test -- --nocapture".to_string(),
            exit_code: Some(0),
        }];

        let summary = summarize_debug(&executions);
        assert_eq!(summary, "cargo test -- --nocapture (exit: 0)");
    }

    #[test]
    fn test_parse_tool_arguments() {
        let args = json!({
            "Path": "\"d:\\\\Knox\\\\src\\\\parser.rs\"",
            "Target": "fn parse()",
            "Replacement": "fn parse()\nfn parse_stream()"
        });

        let file_mod = parse_file_modification("edit_file", &args).unwrap();
        assert_eq!(file_mod.path, "src/parser.rs");
        assert_eq!(file_mod.lines_removed, 1);
        assert_eq!(file_mod.lines_added, 2);
        assert!(!file_mod.is_new);
    }

    #[test]
    fn test_classify_human_input_doubts() {
        assert_eq!(
            classify_human_input("what happens if i delete this"),
            HumanInputKind::Doubt
        );
        assert_eq!(
            classify_human_input("can i delete this one the text is double and overlapping"),
            HumanInputKind::Doubt
        );
        assert_eq!(
            classify_human_input("Should we use SQLite or RocksDB?"),
            HumanInputKind::Doubt
        );
        assert_eq!(
            classify_human_input("Is sqlite locking safe with multi-process?"),
            HumanInputKind::Doubt
        );
        assert_eq!(
            classify_human_input("Which architecture is better: monolithic or layered?"),
            HumanInputKind::Doubt
        );
        assert_eq!(
            classify_human_input("I wonder if we need a connection pool"),
            HumanInputKind::Doubt
        );
    }

    #[test]
    fn test_classify_human_input_plans() {
        assert_eq!(
            classify_human_input("Refactor parser module to support streaming tokens"),
            HumanInputKind::Plan
        );
        assert_eq!(
            classify_human_input("can you fix the login button?"),
            HumanInputKind::Plan
        );
        assert_eq!(
            classify_human_input("please implement token streaming in parser"),
            HumanInputKind::Plan
        );
        assert_eq!(
            classify_human_input("Implement session caching in rust"),
            HumanInputKind::Plan
        );
        assert_eq!(
            classify_human_input(
                "<USER_REQUEST>\nFix connection timeout in db.rs\n</USER_REQUEST>"
            ),
            HumanInputKind::Plan
        );
    }

    #[test]
    fn test_classify_human_input_dialogue() {
        assert_eq!(classify_human_input("ok"), HumanInputKind::Dialogue);
        assert_eq!(classify_human_input("apply"), HumanInputKind::Dialogue);
        assert_eq!(
            classify_human_input("thanks a lot"),
            HumanInputKind::Dialogue
        );
    }
}
