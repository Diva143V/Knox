//! View dispatch and renderers for each GUI wizard step and cockpit tab.

pub mod graph_view;
pub mod ingest_view;
pub mod mcp_view;
pub mod query_view;
pub mod scan_view;
pub mod stats_view;
pub mod welcome;
pub mod workspace;

use eframe::egui::Ui;

use crate::gui::canvas::GraphCanvas;
use crate::gui::state::{AppState, GuiTab};

/// Dispatches rendering to the currently selected view.
pub fn render_active_view(ui: &mut Ui, state: &mut AppState, canvas: &mut GraphCanvas) {
    match state.current_tab {
        GuiTab::Welcome => welcome::render_welcome_view(ui, state),
        GuiTab::Workspace => workspace::render_workspace_view(ui, state),
        GuiTab::Graph => graph_view::render_graph_view(ui, state, canvas),
        GuiTab::Stats => stats_view::render_stats_view(ui, state),
        GuiTab::Query => query_view::render_query_view(ui, state),
        GuiTab::Ingest => ingest_view::render_ingest_view(ui, state),
        GuiTab::Scanner => scan_view::render_scan_view(ui, state),
        GuiTab::Mcp => mcp_view::render_mcp_view(ui, state),
    }
}
