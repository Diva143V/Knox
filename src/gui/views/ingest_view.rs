//! Step 6: Transcript ingestion studio with file picker and bulk import execution.

use eframe::egui::{self, Ui};

use crate::commands::cmd_ingest;
use crate::gui::state::{AppState, GuiTab};
use crate::gui::theme::{card_frame, render_installer_header};

/// Renders the Transcript Ingestion Studio view.
pub fn render_ingest_view(ui: &mut Ui, state: &mut AppState) {
    render_installer_header(
        ui,
        6,
        "Transcript Ingestion Studio",
        "Import existing AI agent conversation transcripts into the Knowledge Graph",
    );

    render_picker_card(ui, state);
    ui.add_space(12.0);

    render_status_card(ui, state);
    ui.add_space(16.0);

    render_nav_buttons(ui, state);
}

fn render_picker_card(ui: &mut Ui, state: &mut AppState) {
    card_frame().show(ui, |ui| {
        ui.heading("Select Conversation Transcript (.jsonl)");
        ui.label("Supported formats: Antigravity transcript.jsonl (with more IDEs coming in v1.2)");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            let path_display = state
                .transcript_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "No file selected...".to_string());
            ui.monospace(&path_display);

            if ui.button("📂 Browse...").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Transcripts", &["jsonl", "json"])
                    .pick_file()
                {
                    state.transcript_path = Some(file);
                }
            }

            if ui
                .button(egui::RichText::new("📥 Ingest Transcript").strong())
                .clicked()
            {
                handle_ingest_action(state);
            }
        });
    });
}

fn handle_ingest_action(state: &mut AppState) {
    let transcript = match state.transcript_path.clone() {
        Some(p) => p,
        None => {
            state.set_toast("Please select a transcript file first.", true);
            return;
        }
    };

    let db = match state.db.as_ref() {
        Some(d) => d,
        None => {
            state.set_toast("Database not open.", true);
            return;
        }
    };

    match cmd_ingest(&transcript, db) {
        Ok(report) => {
            state.refresh_data();
            let msg = format!(
                "Imported {} actions across {} files ({} sessions).",
                report.actions_imported, report.files_touched, report.sessions_created
            );
            state.ingest_status = Some(msg.clone());
            state.set_toast(msg, false);
        }
        Err(e) => {
            let err_msg = format!("Ingestion failed: {}", e);
            state.ingest_status = Some(err_msg.clone());
            state.set_toast(err_msg, true);
        }
    }
}

fn render_status_card(ui: &mut Ui, state: &AppState) {
    if let Some(ref status) = state.ingest_status {
        card_frame().show(ui, |ui| {
            ui.heading("Ingestion Report");
            ui.separator();
            ui.label(status);
        });
    }
}

fn render_nav_buttons(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("⏴ Back: Query Console").clicked() {
            state.current_tab = GuiTab::Query;
        }
        ui.add_space(8.0);
        if ui
            .button(
                egui::RichText::new("Next: Human Edit Scanner ➔")
                    .size(14.0)
                    .strong(),
            )
            .clicked()
        {
            state.current_tab = GuiTab::Scanner;
        }
    });
}
