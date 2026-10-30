//! Shared application state, navigation tabs, and caching for the Knox GUI.

use std::path::PathBuf;

use crate::core::config::KnoxConfig;
use crate::core::types::{KnoxAction, ScanReport, StatusReport};
use crate::storage::db::KnoxDb;

/// Navigation tabs representing the wizard steps and developer cockpit views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiTab {
    Welcome = 0,
    Workspace = 1,
    Graph = 2,
    Stats = 3,
    Query = 4,
    Ingest = 5,
    Scanner = 6,
    Mcp = 7,
}

impl GuiTab {
    /// Returns the user-facing title of the tab.
    pub fn title(&self) -> &'static str {
        match self {
            GuiTab::Welcome => "1. Welcome",
            GuiTab::Workspace => "2. Workspace & Hook",
            GuiTab::Graph => "3. Knowledge Graph",
            GuiTab::Stats => "4. Metrics & Stats",
            GuiTab::Query => "5. Query Console",
            GuiTab::Ingest => "6. Ingest Transcripts",
            GuiTab::Scanner => "7. Human Edit Scan",
            GuiTab::Mcp => "8. MCP Server & Config",
        }
    }

    /// Returns the icon / symbol for the tab.
    pub fn icon(&self) -> &'static str {
        match self {
            GuiTab::Welcome => "🏠",
            GuiTab::Workspace => "📦",
            GuiTab::Graph => "🕸️",
            GuiTab::Stats => "📊",
            GuiTab::Query => "🔍",
            GuiTab::Ingest => "📥",
            GuiTab::Scanner => "🛡️",
            GuiTab::Mcp => "🔌",
        }
    }
}

/// Central state of the Knox GUI application.
pub struct AppState {
    pub config: KnoxConfig,
    pub current_tab: GuiTab,
    pub db: Option<KnoxDb>,
    pub status: Option<StatusReport>,
    pub toast: Option<(String, bool)>,

    // Workspace & Hook Wizard
    pub workspace_path: PathBuf,
    pub knox_md_content: Option<String>,
    pub is_hook_installed: bool,

    // Graph Visualizer
    pub graph_actions: Vec<KnoxAction>,
    pub selected_action: Option<KnoxAction>,
    pub filter_actor: String,
    pub filter_type: String,
    pub search_highlight: String,
    pub hide_superseded: bool,

    // Query Console
    pub search_input: String,
    pub query_file: String,
    pub current_only: bool,
    pub query_results: Vec<KnoxAction>,

    // Ingestion Studio
    pub transcript_path: Option<PathBuf>,
    pub ingest_status: Option<String>,

    // Scanner
    pub scan_report: Option<ScanReport>,
}

impl AppState {
    /// Initializes application state, attempting to open the database from config.
    pub fn new(config: KnoxConfig, initial_path: Option<PathBuf>) -> Self {
        let db = KnoxDb::open(&config.kg_path).ok();
        let status = db
            .as_ref()
            .and_then(|d| crate::commands::cmd_status(d).ok());
        let workspace_path = initial_path.unwrap_or_else(|| PathBuf::from("."));

        let mut state = Self {
            config,
            current_tab: GuiTab::Welcome,
            db,
            status,
            toast: None,
            workspace_path,
            knox_md_content: None,
            is_hook_installed: false,
            graph_actions: Vec::new(),
            selected_action: None,
            filter_actor: "All".to_string(),
            filter_type: "All".to_string(),
            search_highlight: String::new(),
            hide_superseded: false,
            search_input: String::new(),
            query_file: String::new(),
            current_only: true,
            query_results: Vec::new(),
            transcript_path: None,
            ingest_status: None,
            scan_report: None,
        };

        state.refresh_data();
        state
    }

    /// Refreshes database connection, status metrics, and graph actions.
    pub fn refresh_data(&mut self) {
        if self.db.is_none() {
            self.db = KnoxDb::open(&self.config.kg_path).ok();
        }

        if let Some(ref db) = self.db {
            self.status = crate::commands::cmd_status(db).ok();
            let filter = crate::core::types::QueryFilter {
                current_only: false,
                limit: 200,
                ..Default::default()
            };
            if let Ok(res) = crate::storage::nodes::query_actions(db, &filter) {
                self.graph_actions = res.results;
            }
        }
    }

    /// Sets a temporary toast notification.
    pub fn set_toast(&mut self, message: impl Into<String>, is_error: bool) {
        self.toast = Some((message.into(), is_error));
    }
}
