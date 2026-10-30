<!-- knox:config
project_id: "eb624457-04f1-41cc-98fd-08d84e9ded65"
kg_path: "C:/Users/Diwa/.knox/knox.db"
mcp_server: "knox serve --stdio"
mcp_transport: "stdio"
tools_registered:
- knox_get_context
- knox_get_history
- knox_record_action
- knox_record_decision
- knox_search
- knox_list_stale
created_at: "2026-10-29T18:25:18.457451100+00:00"
knox_version: "0.1.0"
-->

# Knox — Active Knowledge Graph

This project is tracked by **Knox**, a persistent knowledge graph that stores
decisions, plans, and actions from every AI-assisted coding session.

## For AI Agents — MANDATORY PROTOCOL

1. **BEFORE acting on any task**: Call `knox_get_context` with the target file
path(s) to retrieve current decisions, prior approaches, and recent actions.
Do NOT re-derive context that already exists in the KG.

2. **AFTER completing significant work**: Call `knox_record_action` with:
- `actor`: Your software name (e.g., `gemini`, `cursor`, `chatgpt`)
- `action_type`: One of `plan`, `write`, `change`, `decide`, `debug`, `read`, `review`
- `files`: List of affected file paths (relative to project root)
- `summary`: Concise description of what was done and why
- `supersedes` (optional): ID of a prior decision this replaces

3. **When making a design decision or choosing an approach**: Call
`knox_record_decision` — these are the highest-value entries in the KG.
Always include the rationale and alternatives considered.

4. **Edge naming convention**: Prefix every action with your software name:
`gemini_write`, `cursor_plan`, `chatgpt_decide`, `copilot_debug`, etc.

5. **Never skip the read step**. Even if you think you know the context,
the KG may contain decisions from other tools or sessions you're unaware of.

## For Humans

Run `knox status` to see what's tracked. Run `knox query <file>` to inspect
the decision history for any file. The KG is append-only — nothing is ever
deleted, only superseded.

## Connection

- **MCP Server**: `knox serve --stdio`
- **Direct CLI**: `knox query`, `knox status`
- **KG Location**: See config block above
