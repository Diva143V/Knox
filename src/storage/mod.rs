//! SQLite persistence and storage layer for Knox.

pub mod db;
pub mod edges;
pub mod migrations;
pub mod nodes;
pub mod queries;
pub mod sessions;

pub use db::KnoxDb;
pub use edges::{get_child_action_ids, get_parent_action_id, link_follows, link_parent};
pub use migrations::run_migrations;
pub use nodes::{
    fetch_action_delta_summary, find_action_by_commit, find_causal_action_by_file_and_commit,
    get_action, get_deltas_by_action, get_deltas_by_file, get_project, get_project_by_path,
    insert_action, insert_action_with_deltas, insert_deltas, insert_project, list_projects,
    mark_superseded, parse_hunk_line_range, resolve_doubt, update_action,
};
pub use queries::{get_actions_by_ids, get_status_report, query_actions, search_actions};
pub use sessions::{
    end_session, find_open_session, get_session, list_recent_sessions, resolve_project_identifier, start_session,
    timeout_stale_sessions,
};
