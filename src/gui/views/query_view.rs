//! Step 5: Live query and search console for actions and decisions in the knowledge graph.

use eframe::egui::{self, Color32, ScrollArea, Ui};

use crate::core::types::QueryFilter;
use crate::gui::state::{AppState, GuiTab};
use crate::gui::theme::{
    COLOR_SUCCESS, action_color, actor_color, card_frame, render_badge, render_installer_header,
};

/// Renders the Query and Search console view.
pub fn render_query_view(ui: &mut Ui, state: &mut AppState) {
    render_installer_header(
        ui,
        5,
        "Query & Search Console",
        "Search recorded actions, decisions, and files using full-text search and filters",
    );

    render_search_bar(ui, state);
    ui.add_space(10.0);

    render_results_table(ui, state);
    ui.add_space(14.0);

    render_nav_buttons(ui, state);
}

fn render_search_bar(ui: &mut Ui, state: &mut AppState) {
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Search:");
            let search_edit = ui.text_edit_singleline(&mut state.search_input);
            if search_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                execute_query(state);
            }

            ui.label("File:");
            ui.text_edit_singleline(&mut state.query_file);

            ui.checkbox(&mut state.current_only, "Current Only");

            if ui
                .button(egui::RichText::new("🔍 Search").strong())
                .clicked()
            {
                execute_query(state);
            }
        });
    });
}

fn execute_query(state: &mut AppState) {
    let db = match state.db.as_ref() {
        Some(d) => d,
        None => {
            state.set_toast("Database not open.", true);
            return;
        }
    };

    let file_opt = if state.query_file.trim().is_empty() {
        None
    } else {
        Some(state.query_file.trim().to_string())
    };

    let filter = QueryFilter {
        file_path: file_opt,
        current_only: state.current_only,
        limit: 50,
        ..Default::default()
    };

    match crate::storage::nodes::query_actions(db, &filter) {
        Ok(res) => {
            state.query_results = res.results;
            state.set_toast(
                format!("Found {} actions.", state.query_results.len()),
                false,
            );
        }
        Err(e) => {
            state.set_toast(format!("Query failed: {}", e), true);
        }
    }
}

fn render_results_table(ui: &mut Ui, state: &mut AppState) {
    card_frame().show(ui, |ui| {
        ui.heading(format!("Results ({})", state.query_results.len()));
        ui.separator();

        ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
            for action in &state.query_results {
                ui.horizontal(|ui| {
                    render_badge(
                        ui,
                        &action.action_type,
                        action_color(&action.action_type),
                        Color32::WHITE,
                    );
                    render_badge(
                        ui,
                        &action.actor,
                        actor_color(&action.actor),
                        Color32::WHITE,
                    );
                    let status_col = if action.is_current {
                        COLOR_SUCCESS
                    } else {
                        Color32::from_rgb(239, 68, 68)
                    };
                    render_badge(
                        ui,
                        if action.is_current {
                            "CURRENT"
                        } else {
                            "STALE"
                        },
                        status_col,
                        Color32::WHITE,
                    );

                    ui.label(&action.summary);
                });
                ui.separator();
            }
        });
    });
}

fn render_nav_buttons(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("⏴ Back: Metrics").clicked() {
            state.current_tab = GuiTab::Stats;
        }
        ui.add_space(8.0);
        if ui
            .button(
                egui::RichText::new("Next: Ingest Transcripts ➔")
                    .size(14.0)
                    .strong(),
            )
            .clicked()
        {
            state.current_tab = GuiTab::Ingest;
        }
    });
}
