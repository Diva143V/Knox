//! SQLite persistence and storage layer for Knox.

pub mod db;
pub mod edges;
pub mod embeddings;
pub mod migrations;
pub mod nodes;
pub mod queries;
pub mod registry;
pub mod sessions;
pub mod telemetry;

pub use db::KnoxDb;
pub use embeddings::{
    blob_to_vector, cosine_similarity, count_embeddings, find_similar_actions, get_embedding,
    get_embeddings_batch, insert_embedding, insert_embedding_conn, vector_to_blob,
};
pub use edges::{
    ensure_ref, get_all_edges_for_actions, get_child_action_ids, get_edge_rejections,
    get_edges_for_action, get_parent_action_id, get_ref, insert_edge, insert_ref, link_follows,
    link_parent, record_edge_rejection,
};
pub use migrations::run_migrations;
pub use nodes::{
    fetch_action_delta_summary, find_action_by_commit, find_causal_action_by_file_and_commit,
    get_action, get_deltas_by_action, get_deltas_by_file, get_project, get_project_by_path,
    get_project_by_remote, insert_action, insert_action_with_deltas, insert_deltas, insert_project,
    list_projects, mark_superseded, parse_hunk_line_range, resolve_doubt, update_action,
};
pub use queries::{
    decode_cursor, encode_cursor, get_actions_by_ids, get_status_report, query_actions,
    search_actions,
};
pub use registry::{
    RegistryEntry, find_registered_project, list_registered_projects, register_project,
    registry_path,
};
pub use sessions::{
    end_session, find_open_session, get_session, is_project_egress_allowed, list_recent_sessions,
    resolve_or_create_project, resolve_project_identifier, set_project_egress, start_session,
    timeout_stale_sessions,
};
pub use telemetry::{
    QueryTelemetryStats, determine_empty_reason, get_action_duplicate_stats, get_query_latencies,
    get_query_log_stats, get_successful_query_token_stats, get_top_rejected_edge_types,
    get_zombie_session_count, prune_query_log, record_query_log, record_query_log_best_effort,
};
