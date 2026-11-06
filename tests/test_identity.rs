//! Workstream E acceptance tests: Identity, Git remote normalization, and cross-clone memory resolution.

use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::Utc;
use serde_json::json;
use tempfile::tempdir;

use knox::commands::check::cmd_check;
use knox::commands::context::cmd_context;
use knox::commands::init::cmd_init;
use knox::commands::record::cmd_record;
use knox::core::config::KnoxConfig;
use knox::core::ids::ProjectId;
use knox::core::types::Project;
use knox::git::normalize_remote_url;
use knox::storage::db::KnoxDb;
use knox::storage::nodes::insert_project;
use knox::storage::sessions::{resolve_project_identifier, start_session};

/// Helper to write a mock `.git/config` with the given remote URL.
fn setup_git_repo_with_remote(repo_path: &Path, remote_url: &str) {
    let git_dir = repo_path.join(".git");
    fs::create_dir_all(&git_dir).unwrap();
    let config_content = format!(
        "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n[remote \"origin\"]\n\turl = {}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n",
        remote_url
    );
    fs::write(git_dir.join("config"), config_content).unwrap();
}

#[test]
fn test_remote_url_normalization_exhaustive() {
    let canonical = "github.com/org/repo";

    // 1. SSH variations
    assert_eq!(
        normalize_remote_url("git@github.com:org/repo.git").as_deref(),
        Some(canonical)
    );
    assert_eq!(
        normalize_remote_url("git@github.com:org/repo").as_deref(),
        Some(canonical)
    );
    assert_eq!(
        normalize_remote_url("ssh://git@github.com/org/repo.git").as_deref(),
        Some(canonical)
    );
    assert_eq!(
        normalize_remote_url("ssh://git@github.com:22/org/repo.git").as_deref(),
        Some(canonical)
    );

    // 2. HTTPS / HTTP variations
    assert_eq!(
        normalize_remote_url("https://github.com/org/repo.git").as_deref(),
        Some(canonical)
    );
    assert_eq!(
        normalize_remote_url("https://github.com/org/repo/").as_deref(),
        Some(canonical)
    );
    assert_eq!(
        normalize_remote_url("https://github.com/org/repo").as_deref(),
        Some(canonical)
    );
    assert_eq!(
        normalize_remote_url("http://github.com/org/repo.git").as_deref(),
        Some(canonical)
    );

    // 3. Case insensitivity
    assert_eq!(
        normalize_remote_url("HTTPS://GitHub.COM/Org/Repo.git").as_deref(),
        Some(canonical)
    );
    assert_eq!(
        normalize_remote_url("git@GITHUB.COM:ORG/REPO.git").as_deref(),
        Some(canonical)
    );

    // 4. Git protocol
    assert_eq!(
        normalize_remote_url("git://github.com/org/repo.git").as_deref(),
        Some(canonical)
    );

    // 5. Authentication tokens embedded in URL
    assert_eq!(
        normalize_remote_url("https://oauth2:ghp_secrettoken@github.com/org/repo.git").as_deref(),
        Some(canonical)
    );

    // 6. GitLab nested subgroups
    assert_eq!(
        normalize_remote_url("git@gitlab.com:enterprise/platform/core.git").as_deref(),
        Some("gitlab.com/enterprise/platform/core")
    );

    // 7. Non-remote paths rejected
    assert_eq!(normalize_remote_url("C:\\Users\\dev\\project"), None);
    assert_eq!(normalize_remote_url("/home/dev/project"), None);
    assert_eq!(normalize_remote_url("./relative/path"), None);
    assert_eq!(normalize_remote_url(""), None);
}

#[test]
fn test_e1_migrations_backfill_remote_from_git_config() {
    let repo_dir = tempdir().unwrap();
    setup_git_repo_with_remote(repo_dir.path(), "git@github.com:acme/billing-service.git");

    let db_path = repo_dir.path().join("migration_test.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();

    // 1. Initialize schema v8 manually without remote
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );
        INSERT OR REPLACE INTO schema_version (version) VALUES (8);",
    )
    .unwrap();

    let proj_id = ProjectId::new();
    let repo_path_str = repo_dir.path().to_string_lossy().to_string();
    conn.execute(
        "INSERT INTO projects (id, name, path, created_at) VALUES (?1, 'billing-service', ?2, ?3)",
        rusqlite::params![proj_id.as_str(), repo_path_str, Utc::now().to_rfc3339()],
    )
    .unwrap();

    // 2. Run migrations (which applies v9 and backfills remote)
    knox::storage::migrations::run_migrations(&conn).unwrap();

    // 3. Verify projects.remote was backfilled from .git/config
    let updated_remote: Option<String> = conn
        .query_row(
            "SELECT remote FROM projects WHERE id = ?1",
            rusqlite::params![proj_id.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        updated_remote.as_deref(),
        Some("github.com/acme/billing-service")
    );
}

#[test]
fn test_e2_resolution_order_id_remote_path_name() {
    let clone1_dir = tempdir().unwrap();
    setup_git_repo_with_remote(clone1_dir.path(), "https://github.com/acme/auth-api.git");

    let clone2_dir = tempdir().unwrap();
    setup_git_repo_with_remote(clone2_dir.path(), "git@github.com:acme/auth-api.git");

    let db = KnoxDb::open_memory().unwrap();

    // Register project for clone1 with normalized remote
    let proj_id = ProjectId::new();
    let proj = Project {
        id: proj_id.clone(),
        name: "auth-api".to_string(),
        path: clone1_dir.path().to_string_lossy().to_string(),
        remote: Some("github.com/acme/auth-api".to_string()),
        created_at: Utc::now(),
    };
    insert_project(&db, &proj).unwrap();

    // 1. Resolution by exact ID
    let res_by_id = resolve_project_identifier(&db, proj_id.as_str()).unwrap();
    assert_eq!(res_by_id.as_deref(), Some(proj_id.as_str()));

    // 2. Resolution by remote URL (SSH format)
    let res_by_ssh_remote =
        resolve_project_identifier(&db, "git@github.com:acme/auth-api.git").unwrap();
    assert_eq!(res_by_ssh_remote.as_deref(), Some(proj_id.as_str()));

    // 2b. Resolution by remote URL (HTTPS format with trailing slash)
    let res_by_https_remote =
        resolve_project_identifier(&db, "https://github.com/acme/auth-api/").unwrap();
    assert_eq!(res_by_https_remote.as_deref(), Some(proj_id.as_str()));

    // 2c. Resolution by clone 2's filesystem path (detects remote from clone2's .git/config)
    let res_by_clone2_path =
        resolve_project_identifier(&db, &clone2_dir.path().to_string_lossy()).unwrap();
    assert_eq!(
        res_by_clone2_path.as_deref(),
        Some(proj_id.as_str()),
        "clone2 path must resolve to clone1's project ID via shared remote"
    );

    // 3. Resolution by clone 1's path
    let res_by_clone1_path =
        resolve_project_identifier(&db, &clone1_dir.path().to_string_lossy()).unwrap();
    assert_eq!(res_by_clone1_path.as_deref(), Some(proj_id.as_str()));

    // 4. Resolution by unique display name
    let res_by_name = resolve_project_identifier(&db, "auth-api").unwrap();
    assert_eq!(res_by_name.as_deref(), Some(proj_id.as_str()));
}

#[test]
fn test_e3_init_in_clone_matches_existing_project() {
    let temp_root = tempdir().unwrap();
    let shared_db_path = temp_root.path().join("shared.db");

    let config = KnoxConfig {
        kg_path: shared_db_path,
        session_timeout: Duration::from_secs(1800),
        global_mode: true,
        transcript_path: None,
        ..Default::default()
    };

    // Clone 1: initialized with SSH remote
    let clone1 = temp_root.path().join("clone1_workspace");
    fs::create_dir_all(&clone1).unwrap();
    setup_git_repo_with_remote(&clone1, "git@github.com:umbrella/monorepo.git");

    cmd_init(&clone1, &config).unwrap();

    let knox1_md = fs::read_to_string(clone1.join("knox.md")).unwrap();
    let proj1_id = knox1_md
        .lines()
        .find_map(|line| line.trim().strip_prefix("project_id: \""))
        .and_then(|s| s.strip_suffix('"'))
        .unwrap()
        .to_string();

    // Clone 2: initialized with HTTPS remote
    let clone2 = temp_root.path().join("clone2_workspace");
    fs::create_dir_all(&clone2).unwrap();
    setup_git_repo_with_remote(&clone2, "https://github.com/umbrella/monorepo.git");

    cmd_init(&clone2, &config).unwrap();

    let knox2_md = fs::read_to_string(clone2.join("knox.md")).unwrap();
    let proj2_id = knox2_md
        .lines()
        .find_map(|line| line.trim().strip_prefix("project_id: \""))
        .and_then(|s| s.strip_suffix('"'))
        .unwrap()
        .to_string();

    assert_eq!(
        proj1_id, proj2_id,
        "knox init in clone2 must match existing project_id from clone1"
    );
}

#[test]
fn test_e4_same_remote_different_path_shared_memory() {
    let temp_root = tempdir().unwrap();
    let shared_db_path = temp_root.path().join("shared_memory.db");

    let config = KnoxConfig {
        kg_path: shared_db_path.clone(),
        session_timeout: Duration::from_secs(1800),
        global_mode: true,
        transcript_path: None,
        ..Default::default()
    };

    // Setup clone 1
    let clone1 = temp_root.path().join("clone_alpha");
    fs::create_dir_all(&clone1).unwrap();
    setup_git_repo_with_remote(&clone1, "git@github.com:myteam/core-engine.git");
    cmd_init(&clone1, &config).unwrap();

    // Setup clone 2 at a completely different path
    let clone2 = temp_root.path().join("clone_beta");
    fs::create_dir_all(&clone2).unwrap();
    setup_git_repo_with_remote(&clone2, "https://github.com/myteam/core-engine.git");
    cmd_init(&clone2, &config).unwrap();

    let db = KnoxDb::open(&shared_db_path).unwrap();

    // 1. Record an architectural decision from clone 1
    let session_clone1 = start_session(
        &db,
        "developer_alpha",
        Some(&clone1.to_string_lossy()),
    )
    .unwrap();

    let record_res = cmd_record(
        &db,
        json!({
            "session_id": session_clone1.id.to_string(),
            "actor": "developer_alpha",
            "project": clone1.to_string_lossy().to_string(),
            "entries": [{
                "type": "decide",
                "summary": "Mandate AES-256-GCM for storage encryption",
                "files": ["src/crypto.rs"],
                "rationale": "Compliance requirement FIPS-140-3",
                "alternatives": ["ChaCha20-Poly1305"]
            }]
        }),
    )
    .unwrap();
    assert_eq!(record_res["count"].as_u64().unwrap(), 1);

    // 2. Query context from clone 2 using clone 2's filesystem path as scope
    let ctx_clone2 = cmd_context(
        &db,
        json!({
            "project": clone2.to_string_lossy().to_string(),
            "format": "json"
        }),
    )
    .unwrap();

    let results = ctx_clone2["results"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0]["summary"],
        "Mandate AES-256-GCM for storage encryption"
    );

    // 3. Pre-flight guardrail check from clone 2 proposing a violation
    let check_clone2 = cmd_check(
        &db,
        json!({
            "project": clone2.to_string_lossy().to_string(),
            "files": ["src/crypto.rs"],
            "intent": "switch to ChaCha20-Poly1305"
        }),
    )
    .unwrap();

    assert_eq!(check_clone2["pass"], false);
    assert_eq!(check_clone2["violations_count"], 1);
    assert_eq!(
        check_clone2["violation_details"][0]["alternative"],
        "ChaCha20-Poly1305"
    );

    // 4. Start session from clone 2 and verify project ID matches
    let session_clone2 = start_session(
        &db,
        "developer_beta",
        Some(&clone2.to_string_lossy()),
    )
    .unwrap();

    assert_eq!(
        session_clone1.project_id, session_clone2.project_id,
        "both clone sessions must share the same canonical project UUID"
    );
}
