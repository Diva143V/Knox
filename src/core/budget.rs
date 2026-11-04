//! Output budget primitives shared by context-producing features.

/// Default token budget for compact context responses.
pub const DEFAULT_CONTEXT_BUDGET: usize = 1_200;
/// Default token budget for guardrail pre-flight checks.
pub const DEFAULT_CHECK_BUDGET: usize = 600;
/// Default token budget for causal trace responses.
pub const DEFAULT_TRACE_BUDGET: usize = 800;
/// Hard ceiling for caller-provided context budgets.
pub const MAX_CONTEXT_BUDGET: usize = 4_000;

/// Conservative token estimate for UTF-8 text.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// Truncates text at a line boundary without exceeding the requested budget.
pub fn truncate_to_budget(text: &str, budget: usize) -> (String, usize, bool) {
    if estimate_tokens(text) <= budget {
        return (text.to_string(), estimate_tokens(text), false);
    }

    let mut output = String::new();
    for line in text.lines() {
        let candidate = if output.is_empty() {
            line.to_string()
        } else {
            format!("{output}\n{line}")
        };
        if estimate_tokens(&candidate) > budget {
            break;
        }
        output = candidate;
    }

    let used = estimate_tokens(&output);
    (output, used, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_and_truncates_at_line_boundaries() {
        let text = "one two three four\nfive six seven eight\nnine ten eleven twelve";
        let (trimmed, used, truncated) = truncate_to_budget(text, 5);
        assert_eq!(trimmed, "one two three four");
        assert_eq!(used, 5);
        assert!(truncated);
    }

    #[test]
    fn preserves_text_within_budget() {
        let text = "short";
        assert_eq!(truncate_to_budget(text, 10), (text.to_string(), 2, false));
    }
}
