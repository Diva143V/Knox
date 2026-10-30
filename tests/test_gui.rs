//! Integration tests for Knox GUI state, themes, and 2D canvas graph builder.

use std::path::PathBuf;

use chrono::Utc;
use knox::cli::{Commands, KnoxCli};
use knox::core::config::KnoxConfig;
use knox::core::ids::{ActionId, SessionId};
use knox::core::types::KnoxAction;
use knox::gui::canvas::GraphCanvas;
use knox::gui::state::{AppState, GuiTab};
use knox::gui::theme::{action_color, actor_color};

#[test]
fn test_gui_tab_titles_and_icons() {
    let tabs = [
        GuiTab::Welcome,
        GuiTab::Workspace,
        GuiTab::Graph,
        GuiTab::Stats,
        GuiTab::Query,
        GuiTab::Ingest,
        GuiTab::Scanner,
        GuiTab::Mcp,
    ];

    assert_eq!(tabs.len(), 8);
    for tab in tabs {
        assert!(!tab.title().is_empty());
        assert!(!tab.icon().is_empty());
    }
}

#[test]
fn test_gui_theme_colors() {
    let write_col = action_color("write");
    let plan_col = action_color("plan");
    let decide_col = action_color("decide");
    let debug_col = action_color("debug");

    assert_ne!(write_col, plan_col);
    assert_ne!(plan_col, decide_col);
    assert_ne!(decide_col, debug_col);

    let human_col = actor_color("human");
    let anti_col = actor_color("antigravity");
    assert_ne!(human_col, anti_col);
}

#[test]
fn test_gui_state_initialization() {
    let config = KnoxConfig {
        kg_path: PathBuf::from("target/test_knox_gui.db"),
        session_timeout: std::time::Duration::from_secs(30 * 60),
        global_mode: true,
    };

    let mut state = AppState::new(config, Some(PathBuf::from(".")));
    assert_eq!(state.current_tab, GuiTab::Welcome);
    assert!(state.toast.is_none());

    state.set_toast("Test alert", false);
    assert!(state.toast.is_some());
    let (msg, is_err) = state.toast.unwrap();
    assert_eq!(msg, "Test alert");
    assert!(!is_err);
}

#[test]
fn test_gui_canvas_builder_and_simulation() {
    let act1 = KnoxAction {
        id: ActionId("act-1".to_string()),
        actor: "antigravity".to_string(),
        action_type: "plan".to_string(),
        summary: "Initialize architecture".to_string(),
        files: vec!["src/main.rs".to_string()],
        tags: vec!["arch".to_string()],
        confidence: Some(1.0),
        timestamp: Utc::now(),
        session_id: Some(SessionId("sess-1".to_string())),
        parent_action_id: None,
        supersedes: None,
        is_current: true,
    };

    let act2 = KnoxAction {
        id: ActionId("act-2".to_string()),
        actor: "antigravity".to_string(),
        action_type: "write".to_string(),
        summary: "Write core types".to_string(),
        files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
        tags: vec!["code".to_string()],
        confidence: Some(1.0),
        timestamp: Utc::now(),
        session_id: Some(SessionId("sess-1".to_string())),
        parent_action_id: Some(ActionId("act-1".to_string())),
        supersedes: None,
        is_current: true,
    };

    let mut canvas = GraphCanvas::default();
    canvas.build_from_actions(&[act1, act2], false);

    // 2 actions + 2 unique files (src/main.rs, src/lib.rs) = 4 nodes
    assert_eq!(canvas.nodes.len(), 4);
    // act1->main.rs, act2->main.rs, act2->lib.rs, act1->act2 (follows) = 4 edges
    assert!(canvas.edges.len() >= 4);

    let initial_pos = canvas.nodes[0].pos;
    canvas.step_simulation();
    let after_pos = canvas.nodes[0].pos;
    // Position should be non-NaN after simulation step
    assert!(!after_pos.x.is_nan() && !after_pos.y.is_nan());
    assert_ne!(initial_pos, after_pos);
}

#[test]
fn test_gui_cli_gui_dispatch() {
    use clap::Parser;

    let cli = KnoxCli::try_parse_from(["knox", "gui", "--path", "my/project"]).unwrap();
    match cli.command {
        Commands::Gui(args) => {
            assert_eq!(args.path, Some(PathBuf::from("my/project")));
        }
        _ => panic!("Expected Gui command"),
    }
}
