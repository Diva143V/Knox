//! Step 7: Git staged changes scanner and human edit recording.

use eframe::egui::{self, Ui};

use crate::commands::cmd_scan;
use crate::gui::state::{AppState, GuiTab};
use crate::gui::theme::{card_frame, render_installer_header};

/// Renders the Human Edit Git Scanner view.
pub fn render_scan_view(ui: &mut Ui, state: &mut AppState) {
    render_installer_header(
        ui,
        7,
        "Human Edit Scanner (Git diff)",
        "Scan staged git changes to detect and record unrecorded manual human edits",
    );

    render_scan_controls(ui, state);
    ui.add_space(12.0);

    render_scan_results(ui, state);
    ui.add_space(16.0);

    render_nav_buttons(ui, state);
}

fn render_scan_controls(ui: &mut Ui, state: &mut AppState) {
    card_frame().show(ui, |ui| {
        ui.heading("Workspace Git Status");
        ui.label(format!(
            "Scanning workspace: {}",
            state.workspace_path.display()
        ));
        ui.add_space(6.0);

        if ui
            .button(egui::RichText::new("🔍 Scan Staged Changes Now").strong())
            .clicked()
        {
            handle_scan_action(state);
        }
    });
}

fn handle_scan_action(state: &mut AppState) {
    let db = match state.db.as_ref() {
        Some(d) => d,
        None => {
            state.set_toast("Database not open.", true);
            return;
        }
    };

    match cmd_scan(&state.workspace_path, db) {
        Ok(report) => {
            state.refresh_data();
            let msg = format!(
                "Scan complete: {} human edits detected and recorded.",
                report.human_edits_detected
            );
            state.set_toast(msg, false);
            state.scan_report = Some(report);
        }
        Err(e) => {
            state.set_toast(format!("Git scan error: {}", e), true);
        }
    }
}

fn render_scan_results(ui: &mut Ui, state: &AppState) {
    if let Some(ref report) = state.scan_report {
        card_frame().show(ui, |ui| {
            ui.heading("Scan Results");
            ui.separator();
            ui.label(format!(
                "• Human edits detected: {}",
                report.human_edits_detected
            ));
            ui.label(format!(
                "• Actions recorded:     {}",
                report.actions_recorded
            ));

            if let Some(ref diff) = report.diff_stat {
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Diff Statistics:").strong());
                ui.monospace(diff);
            }

            if !report.files_scanned.is_empty() {
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Files Scanned:").strong());
                for f in &report.files_scanned {
                    ui.monospace(format!("• {}", f));
                }
            }
        });
    }
}

fn render_nav_buttons(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("⏴ Back: Ingestion Studio").clicked() {
            state.current_tab = GuiTab::Ingest;
        }
        ui.add_space(8.0);
        if ui
            .button(
                egui::RichText::new("Next: MCP Server & Config ➔")
                    .size(14.0)
                    .strong(),
            )
            .clicked()
        {
            state.current_tab = GuiTab::Mcp;
        }
    });
}
