//! Tests for Knox configuration and global path resolution.

use knox::core::config::{default_global_db_path, KnoxConfig};
use std::path::Path;

#[test]
fn test_resolve_db_path_always_global() {
    let mut cfg = KnoxConfig::default();
    assert_eq!(cfg.resolve_db_path(), default_global_db_path());

    // When global_mode is explicitly set to false, it must still resolve to the global database
    cfg.global_mode = false;
    assert_eq!(cfg.resolve_db_path(), default_global_db_path());

    // resolve_db_path_for must also resolve to global database regardless of project path
    let project_dir = Path::new("/some/arbitrary/project");
    assert_eq!(cfg.resolve_db_path_for(project_dir), default_global_db_path());
}

