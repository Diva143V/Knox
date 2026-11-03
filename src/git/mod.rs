//! Git integration module for diff capture, blame inspection, and drift verification.

pub mod diff;

pub use diff::{
    capture_untracked_file, capture_working_tree_diff, get_git_blame_commit,
    parse_unified_diff, verify_repository_drift,
};
