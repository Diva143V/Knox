//! Step 3: Interactive Knowledge Graph visualizer and node inspector.

use eframe::egui::{self, Color32, Ui};

use crate::core::types::KnoxAction;
use crate::gui::canvas::GraphCanvas;
use crate::gui::state::AppState;
use crate::gui::theme::{
    COLOR_PRIMARY, action_color, actor_color, card_frame, render_badge, render_installer_header,
};

/// Renders the Knowledge Graph canvas and inspector panel.
pub fn render_graph_view(ui: &mut Ui, state: &mut AppState, canvas: &mut GraphCanvas) {
    render_installer_header(
        ui,
        3,
        "Interactive Knowledge Graph",
        "Visual 2D canvas of actions, files, causal chains, and decisions",
    );

    render_canvas_toolbar(ui, state, canvas);
    ui.add_space(8.0);

    let mut clicked_action = None;
    ui.columns(2, |columns| {
        // Left Column: Interactive Canvas (70% width)
        let canvas_response = canvas.render(&mut columns[0]);
        if let Some(act) = canvas_response {
            clicked_action = Some(act);
        }

        // Right Column: Node Inspector Drawer (30% width)
        render_inspector_panel(&mut columns[1], state);
    });

    if let Some(act) = clicked_action {
        state.selected_action = Some(act);
    }
}

fn render_canvas_toolbar(ui: &mut Ui, state: &mut AppState, canvas: &mut GraphCanvas) {
    ui.horizontal(|ui| {
        if ui.button("🔄 Reload Data").clicked() {
            state.refresh_data();
            canvas.build_from_actions(&state.graph_actions, state.hide_superseded);
        }

        let sim_text = if canvas.is_simulating {
            "⏸ Pause Physics"
        } else {
            "▶ Play Physics"
        };
        if ui.button(sim_text).clicked() {
            canvas.is_simulating = !canvas.is_simulating;
        }

        if ui.button("🎯 Center View").clicked() {
            canvas.pan = egui::Vec2::ZERO;
            canvas.zoom = 1.0;
        }

        ui.separator();
        if ui
            .checkbox(&mut state.hide_superseded, "Hide Superseded")
            .changed()
        {
            canvas.build_from_actions(&state.graph_actions, state.hide_superseded);
        }

        ui.label(format!(
            "Nodes: {} | Edges: {}",
            canvas.nodes.len(),
            canvas.edges.len()
        ));
    });
}

fn render_inspector_panel(ui: &mut Ui, state: &mut AppState) {
    card_frame().show(ui, |ui| {
        ui.heading("Node Inspector");
        ui.separator();

        if let Some(ref action) = state.selected_action {
            render_action_details(ui, action);
        } else {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new(
                    "Click any node on the canvas to inspect its details, \
                                    causal links, and affected files.",
                )
                .color(Color32::from_rgb(148, 163, 184)),
            );
        }
    });
}

fn render_action_details(ui: &mut Ui, action: &KnoxAction) {
    ui.horizontal(|ui| {
        render_badge(
            ui,
            &action.action_type.to_uppercase(),
            action_color(&action.action_type),
            Color32::WHITE,
        );
        render_badge(
            ui,
            &action.actor,
            actor_color(&action.actor),
            Color32::WHITE,
        );
        if !action.is_current {
            render_badge(
                ui,
                "SUPERSEDED",
                Color32::from_rgb(239, 68, 68),
                Color32::WHITE,
            );
        }
    });

    ui.add_space(6.0);
    ui.label(egui::RichText::new("Summary:").strong());
    ui.label(&action.summary);

    ui.add_space(6.0);
    ui.label(egui::RichText::new("Timestamp:").strong());
    ui.monospace(action.timestamp.to_rfc3339());

    if !action.files.is_empty() {
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Affected Files:").strong());
        for f in &action.files {
            ui.monospace(format!("• {}", f));
        }
    }

    if let Some(ref pid) = action.parent_action_id {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(format!("Parent Action: {}", pid.0)).color(COLOR_PRIMARY));
    }
}
