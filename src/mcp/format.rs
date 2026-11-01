//! Token-efficient Markdown formatting and context synthesis for Knox MCP tools.

use crate::core::types::KnoxAction;

/// Formats a list of `KnoxAction` records into a token-dense, agent-readable Markdown digest.
///
/// Reduces token usage by ~70-80% compared to verbose pretty-printed JSON while highlighting
/// architectural decisions, constraints, and causal relationships.
pub fn format_markdown_digest(title: &str, actions: &[KnoxAction], synthesis: bool) -> String {
    if actions.is_empty() {
        return format!("### {title}\n*No matching actions recorded in Knox.*");
    }

    if synthesis {
        format_synthesized_digest(title, actions)
    } else {
        format_linear_digest(title, actions)
    }
}

/// Formats actions as a structured chronological list with callouts for decisions.
fn format_linear_digest(title: &str, actions: &[KnoxAction]) -> String {
    let mut out = String::with_capacity(1024);
    out.push_str(&format!("### {title} ({} entries)\n\n", actions.len()));

    for act in actions {
        let ts = act.timestamp.format("%Y-%m-%d %H:%M").to_string();
        if act.action_type == "decide" {
            let callout_type = if act.significance.as_deref() == Some("minor") {
                "[!NOTE]\n> **Minor Decision"
            } else {
                "[!IMPORTANT]\n> **Decision"
            };
            out.push_str(&format!(
                "> {callout_type} [{}]** by `@{}` ({}):\n> {}\n",
                act.id, act.actor, ts, act.summary
            ));
            if let Some(ref r) = act.rationale {
                out.push_str(&format!("> - **Rationale**: {}\n", r));
            }
            if !act.rejected_alternatives.is_empty() {
                out.push_str(&format!(
                    "> - **Rejected**: {}\n",
                    act.rejected_alternatives.join(", ")
                ));
            }
            if !act.files.is_empty() {
                out.push_str(&format!("> - **Files**: {}\n", act.files.join(", ")));
            }
            if !act.symbols.is_empty() {
                out.push_str(&format!("> - **Symbols**: {}\n", act.symbols.join(", ")));
            }
            out.push('\n');
        } else if act.action_type == "doubt" || act.action_type == "question" {
            let status = if act.is_resolved == Some(true) {
                "[RESOLVED]"
            } else {
                "[OPEN DOUBT]"
            };
            out.push_str(&format!(
                "> [!WARNING]\n> **Technical Doubt {} [{}]** by `@{}` ({}):\n> {}\n",
                status, act.id, act.actor, ts, act.summary
            ));
            if let Some(ref note) = act.resolution_note {
                out.push_str(&format!("> - **Resolution**: {}\n", note));
            }
            if let Some(ref ctx) = act.context_summary {
                out.push_str(&format!("> - **Context**: {}\n", ctx));
            }
            if !act.files.is_empty() {
                out.push_str(&format!("> - **Files**: {}\n", act.files.join(", ")));
            }
            out.push('\n');
        } else {
            let files_str = if act.files.is_empty() {
                String::new()
            } else {
                format!(" (`{}`)", act.files.join("`, `"))
            };
            out.push_str(&format!(
                "- **{}** [`{}`] by `@{}` ({}){}\n  {}\n",
                act.action_type.to_uppercase(),
                act.id,
                act.actor,
                ts,
                files_str,
                act.summary
            ));
            if !act.symbols.is_empty() {
                out.push_str(&format!("  *Symbols*: {}\n", act.symbols.join(", ")));
            }
            if let Some(ref ctx) = act.context_summary {
                out.push_str(&format!("  *Context*: {}\n", ctx));
            }
            if !act.relations.is_empty() {
                let rels: Vec<String> = act
                    .relations
                    .iter()
                    .map(|r| format!("{} `{}`", r.relation_type, r.target_id))
                    .collect();
                out.push_str(&format!("  *Relations*: {}\n", rels.join(", ")));
            }
        }
    }

    out
}

/// Formats actions grouped into semantic categories: Active Decisions, Touched Symbols, and Recent Actions.
fn format_synthesized_digest(title: &str, actions: &[KnoxAction]) -> String {
    let mut out = String::with_capacity(1500);
    out.push_str(&format!("### {title} — Synthesized Context\n\n"));

    let open_doubts: Vec<&KnoxAction> = actions
        .iter()
        .filter(|a| {
            (a.action_type == "doubt" || a.action_type == "question") && a.is_resolved != Some(true)
        })
        .collect();
    let decisions: Vec<&KnoxAction> = actions
        .iter()
        .filter(|a| a.action_type == "decide")
        .collect();
    let writes: Vec<&KnoxAction> = actions
        .iter()
        .filter(|a| a.action_type == "write")
        .collect();
    let other: Vec<&KnoxAction> = actions
        .iter()
        .filter(|a| {
            a.action_type != "decide"
                && a.action_type != "write"
                && !(a.action_type == "doubt" || a.action_type == "question")
        })
        .collect();

    // 0. Open Doubts & Technical Inquiries
    if !open_doubts.is_empty() {
        out.push_str("#### ❓ Open Doubts & Unresolved Questions\n");
        for d in &open_doubts {
            let ts = d.timestamp.format("%Y-%m-%d").to_string();
            out.push_str(&format!(
                "> [!WARNING]\n> **Open Doubt [{}]** ({}): {}\n",
                d.id, ts, d.summary
            ));
            if let Some(ref ctx) = d.context_summary {
                out.push_str(&format!("> - **Context**: {}\n", ctx));
            }
            if !d.files.is_empty() {
                out.push_str(&format!(
                    "> - **Target Files**: `{}`\n",
                    d.files.join("`, `")
                ));
            }
            out.push('\n');
        }
    }

    // 1. Active Decisions
    if !decisions.is_empty() {
        out.push_str("#### 1. Architectural Decisions & Constraints\n");
        for d in decisions {
            let ts = d.timestamp.format("%Y-%m-%d").to_string();
            let callout_type = if d.significance.as_deref() == Some("minor") {
                "[!NOTE]\n> **Minor Decision"
            } else {
                "[!IMPORTANT]\n> **Decision"
            };
            out.push_str(&format!(
                "> {callout_type} [{}]** ({}): {}\n",
                d.id, ts, d.summary
            ));
            if let Some(ref r) = d.rationale {
                out.push_str(&format!("> - **Rationale**: {}\n", r));
            }
            if !d.rejected_alternatives.is_empty() {
                out.push_str(&format!(
                    "> - **Rejected Alternatives**: {}\n",
                    d.rejected_alternatives.join("; ")
                ));
            }
            out.push('\n');
        }
    }

    // 2. Touched Symbols
    let mut all_symbols: Vec<String> = actions
        .iter()
        .flat_map(|a| a.symbols.iter().cloned())
        .collect();
    all_symbols.sort();
    all_symbols.dedup();
    if !all_symbols.is_empty() {
        out.push_str("#### 2. Key Code Symbols Referenced\n");
        out.push_str(&format!("`{}`\n\n", all_symbols.join("`, `")));
    }

    // 3. Recent Implementation Writes
    if !writes.is_empty() {
        out.push_str("#### 3. Recent Code Modifications\n");
        for w in writes.iter().take(5) {
            let files = if w.files.is_empty() {
                "".to_string()
            } else {
                format!(" on `{}`", w.files.join(", "))
            };
            out.push_str(&format!("- **[{}]** {}{}\n", w.id, w.summary, files));
        }
        out.push('\n');
    }

    // 4. Other actions summary
    if !other.is_empty() {
        out.push_str(&format!(
            "*+ {} other actions (debug/plan/read)*\n",
            other.len()
        ));
    }

    out
}

/// Formats a causal chain into an indented tree showing root plan down to leaf execution.
pub fn format_lineage_markdown(target_label: &str, chain: &[KnoxAction]) -> String {
    if chain.is_empty() {
        return format!("### Causal Lineage for `{target_label}`\n*No causal parent chain found.*");
    }

    let mut out = String::with_capacity(1024);
    out.push_str(&format!(
        "### Causal Lineage for `{target_label}` ({} hops)\n\n",
        chain.len()
    ));

    // The chain is returned starting with the queried action and walking back to root ancestor.
    // We reverse it to display chronologically: Root Plan -> Architecture -> Implementation.
    let chronological: Vec<&KnoxAction> = chain.iter().rev().collect();
    for (i, act) in chronological.iter().enumerate() {
        let indent = "    ".repeat(i);
        let prefix = if i == 0 { "── [Root]" } else { "└──" };
        let ts = act.timestamp.format("%Y-%m-%d %H:%M").to_string();
        out.push_str(&format!(
            "{}{} **{}** [`{}`] by `@{}` ({}):\n{}   {}\n",
            indent,
            prefix,
            act.action_type.to_uppercase(),
            act.id,
            act.actor,
            ts,
            indent,
            act.summary
        ));
        if let Some(ref r) = act.rationale {
            out.push_str(&format!("{}   *Rationale*: {}\n", indent, r));
        }
    }

    out
}

/// Formats guardrails (active decisions, constraint violations, open doubts, and rejected patterns) for target files before editing.
pub fn format_guardrails_markdown(
    files: &[String],
    active_decisions: &[KnoxAction],
    stale_decisions: &[KnoxAction],
    violations: &[String],
    open_doubts: &[KnoxAction],
) -> String {
    let mut out = String::with_capacity(1024);
    out.push_str(&format!(
        "### Knox Pre-Flight Guardrails ({} target files)\n\n",
        files.len()
    ));

    if !files.is_empty() {
        out.push_str(&format!("**Files**: `{}`\n\n", files.join("`, `")));
    }

    if !violations.is_empty() {
        out.push_str("#### 🚨 Architectural Violations Detected\n");
        for v in violations {
            out.push_str(&format!("> [!CAUTION]\n> **Conflict**: {}\n", v));
        }
        out.push('\n');
    }

    if !open_doubts.is_empty() {
        out.push_str("#### ❓ Open Doubts & Technical Inquiries\n");
        for d in open_doubts {
            out.push_str(&format!(
                "> [!WARNING]\n> **Open Doubt [{}]** ({}): {}\n",
                d.id,
                d.timestamp.format("%Y-%m-%d"),
                d.summary
            ));
            if let Some(ref ctx) = d.context_summary {
                out.push_str(&format!("> - **Context**: {}\n", ctx));
            }
            out.push('\n');
        }
    }

    if active_decisions.is_empty()
        && stale_decisions.is_empty()
        && violations.is_empty()
        && open_doubts.is_empty()
    {
        out.push_str("✅ *No restrictive architectural constraints or warnings found for these files. Proceed with standard implementation.*\n");
        return out;
    }

    if !active_decisions.is_empty() {
        out.push_str("#### ⚠️ Active Architectural Constraints\n");
        for d in active_decisions {
            out.push_str(&format!(
                "> [!IMPORTANT]\n> **Decision [{}]** ({}): {}\n",
                d.id,
                d.timestamp.format("%Y-%m-%d"),
                d.summary
            ));
            if let Some(ref r) = d.rationale {
                out.push_str(&format!("> - **Mandatory Rationale**: {}\n", r));
            }
            if !d.rejected_alternatives.is_empty() {
                out.push_str(&format!(
                    "> - **Prohibited Patterns**: {}\n",
                    d.rejected_alternatives.join("; ")
                ));
            }
            out.push('\n');
        }
    }

    if !stale_decisions.is_empty() {
        out.push_str("#### ❌ Previously Rejected or Superseded Anti-Patterns\n");
        for s in stale_decisions {
            out.push_str(&format!(
                "- **Superseded [{}]**: {} *(Do not re-propose this approach)*\n",
                s.id, s.summary
            ));
        }
        out.push('\n');
    }

    out
}
