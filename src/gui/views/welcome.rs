//! Step 1: Classic installer welcome screen and system health diagnostics.

use eframe::egui::{self, Color32, Ui};

use crate::gui::state::{AppState, GuiTab};
use crate::gui::theme::{COLOR_SUCCESS, card_frame, render_badge, render_installer_header};

/// Renders the Welcome and System Diagnostics view.
pub fn render_welcome_view(ui: &mut Ui, state: &mut AppState) {
    render_installer_header(
        ui,
        1,
        "Welcome to Knox Knowledge Graph",
        "Persistent context, decisions, and action history for AI-assisted coding",
    );

    render_intro_card(ui);
    ui.add_space(12.0);

    render_diagnostics_card(ui, state);
    ui.add_space(16.0);

    render_wizard_buttons(ui, state);
}

fn render_intro_card(ui: &mut Ui) {
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🛡️").size(32.0));
            ui.vertical(|ui| {
                ui.heading("Knox v1.1.0 — Developer Cockpit & Installer");
                ui.label(
                    "Knox maintains an append-only, conversation-derived Knowledge Graph across \
                     Cursor, Claude, Antigravity, and Copilot sessions. Agents query context instead \
                     of re-discovering it from scratch.",
                );
            });
        });
    });
}

fn render_diagnostics_card(ui: &mut Ui, state: &AppState) {
    card_frame().show(ui, |ui| {
        ui.heading("System Diagnostics");
        ui.add_space(4.0);

        let db_path_str = state.config.kg_path.display().to_string();
        let db_exists = state.config.kg_path.exists();
        let db_status = if db_exists {
            "Ready (Connected)"
        } else {
            "Not Initialized"
        };
        let db_color = if db_exists {
            COLOR_SUCCESS
        } else {
            Color32::from_rgb(239, 68, 68)
        };

        ui.horizontal(|ui| {
            ui.label("Knowledge Graph Database:");
            ui.monospace(&db_path_str);
            render_badge(ui, db_status, db_color, Color32::WHITE);
        });

        if let Some(ref st) = state.status {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(format!("• Actions Recorded: {}", st.total_actions));
                ui.label(format!("• Active Sessions: {}", st.total_sessions));
                ui.label(format!("• Projects Tracked: {}", st.projects_tracked));
            });
        }
    });
}

fn render_wizard_buttons(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui
            .button(
                egui::RichText::new("Next: Setup Workspace & Git Hook ➔")
                    .size(14.0)
                    .strong(),
            )
            .clicked()
        {
            state.current_tab = GuiTab::Workspace;
        }

        ui.add_space(8.0);
        if ui.button("Direct to Knowledge Graph 🕸️").clicked() {
            state.current_tab = GuiTab::Graph;
        }

        if ui.button("MCP Integration Guide 🔌").clicked() {
            state.current_tab = GuiTab::Mcp;
        }
    });
}
