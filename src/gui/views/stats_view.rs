//! Step 4: Metric cards and distribution statistics for the knowledge graph.

use eframe::egui::{self, Color32, ProgressBar, Ui};

use crate::gui::state::{AppState, GuiTab};
use crate::gui::theme::{
    action_color, actor_color, card_frame, render_badge, render_installer_header,
};

/// Renders the Metrics and Knowledge Graph Statistics view.
pub fn render_stats_view(ui: &mut Ui, state: &mut AppState) {
    render_installer_header(
        ui,
        4,
        "Metrics & Graph Health",
        "Aggregate action counts, actor distribution, and decision supersession ratios",
    );

    render_metric_cards(ui, state);
    ui.add_space(12.0);

    render_distribution_bars(ui, state);
    ui.add_space(16.0);

    render_nav_buttons(ui, state);
}

fn render_metric_cards(ui: &mut Ui, state: &AppState) {
    let (actions, sessions, projects, last_act) = if let Some(ref st) = state.status {
        (
            st.total_actions.to_string(),
            st.total_sessions.to_string(),
            st.projects_tracked.to_string(),
            st.last_action_timestamp
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Never".to_string()),
        )
    } else {
        ("0".into(), "0".into(), "0".into(), "None".into())
    };

    ui.horizontal(|ui| {
        render_single_stat(ui, "TOTAL ACTIONS", &actions, "⚡");
        render_single_stat(ui, "TOTAL SESSIONS", &sessions, "⏱️");
        render_single_stat(ui, "PROJECTS", &projects, "📂");
        render_single_stat(ui, "LAST UPDATE", &last_act, "📅");
    });
}

fn render_single_stat(ui: &mut Ui, title: &str, value: &str, icon: &str) {
    card_frame().show(ui, |ui| {
        ui.set_width(180.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(icon).size(20.0));
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .size(10.0)
                        .color(Color32::from_rgb(148, 163, 184)),
                );
                ui.heading(egui::RichText::new(value).size(18.0).strong());
            });
        });
    });
}

fn render_distribution_bars(ui: &mut Ui, state: &AppState) {
    card_frame().show(ui, |ui| {
        ui.heading("Actions by Actor & Type");
        ui.separator();

        if let Some(ref st) = state.status {
            let total = st.total_actions.max(1) as f32;

            ui.columns(2, |columns| {
                columns[0].label(egui::RichText::new("By Actor:").strong());
                for (actor, count) in &st.actions_by_actor {
                    let frac = *count as f32 / total;
                    columns[0].horizontal(|ui| {
                        render_badge(ui, actor, actor_color(actor), Color32::WHITE);
                        ui.add(ProgressBar::new(frac).text(format!(
                            "{} ({}%)",
                            count,
                            (frac * 100.0) as u32
                        )));
                    });
                }

                columns[1].label(egui::RichText::new("By Action Type:").strong());
                for (act_type, count) in &st.actions_by_type {
                    let frac = *count as f32 / total;
                    columns[1].horizontal(|ui| {
                        render_badge(ui, act_type, action_color(act_type), Color32::WHITE);
                        ui.add(ProgressBar::new(frac).text(format!(
                            "{} ({}%)",
                            count,
                            (frac * 100.0) as u32
                        )));
                    });
                }
            });
        }
    });
}

fn render_nav_buttons(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("⏴ Back: Knowledge Graph").clicked() {
            state.current_tab = GuiTab::Graph;
        }
        ui.add_space(8.0);
        if ui
            .button(
                egui::RichText::new("Next: Live Query Console ➔")
                    .size(14.0)
                    .strong(),
            )
            .clicked()
        {
            state.current_tab = GuiTab::Query;
        }
    });
}
