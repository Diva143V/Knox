//! Main eframe application struct and top-level UI frame layout for Knox.

use std::path::PathBuf;

use eframe::egui::{self, Color32, TopBottomPanel, Ui};

use crate::core::config::KnoxConfig;
use crate::gui::canvas::GraphCanvas;
use crate::gui::state::{AppState, GuiTab};
use crate::gui::theme::{
    COLOR_BG_DARK, COLOR_PANEL_BG, COLOR_PRIMARY, COLOR_SUCCESS, render_badge,
};
use crate::gui::views::render_active_view;

/// Root GUI application struct for Knox.
pub struct KnoxApp {
    pub state: AppState,
    pub canvas: GraphCanvas,
}

impl KnoxApp {
    /// Creates a new KnoxApp instance with initial state and loaded graph.
    pub fn new(config: KnoxConfig, initial_path: Option<PathBuf>) -> Self {
        let state = AppState::new(config, initial_path);
        let mut canvas = GraphCanvas::default();
        canvas.build_from_actions(&state.graph_actions, state.hide_superseded);

        Self { state, canvas }
    }

    fn render_top_header(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("🛡️ KNOX")
                    .size(18.0)
                    .strong()
                    .color(COLOR_PRIMARY),
            );
            ui.label(
                egui::RichText::new("v1.1.0")
                    .size(12.0)
                    .color(Color32::from_rgb(148, 163, 184)),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let db_open = self.state.db.is_some();
                let (badge, color) = if db_open {
                    ("DB CONNECTED", COLOR_SUCCESS)
                } else {
                    ("NO DB", Color32::from_rgb(239, 68, 68))
                };
                render_badge(ui, badge, color, Color32::WHITE);
                ui.label(
                    egui::RichText::new(self.state.config.kg_path.display().to_string())
                        .size(11.0)
                        .color(Color32::from_rgb(148, 163, 184)),
                );
            });
        });
    }

    fn render_navigation_bar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
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

            for tab in tabs {
                let is_selected = self.state.current_tab == tab;
                let text = format!("{} {}", tab.icon(), tab.title());
                let btn = if is_selected {
                    egui::Button::new(egui::RichText::new(text).strong().color(Color32::WHITE))
                        .fill(COLOR_PRIMARY)
                } else {
                    egui::Button::new(
                        egui::RichText::new(text).color(Color32::from_rgb(203, 213, 225)),
                    )
                    .fill(COLOR_PANEL_BG)
                };

                if ui.add(btn).clicked() {
                    self.state.current_tab = tab;
                    if tab == GuiTab::Graph {
                        self.canvas.build_from_actions(
                            &self.state.graph_actions,
                            self.state.hide_superseded,
                        );
                    }
                }
            }
        });
    }

    fn render_bottom_status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if let Some((ref msg, is_err)) = self.state.toast {
                let col = if is_err {
                    Color32::from_rgb(239, 68, 68)
                } else {
                    COLOR_SUCCESS
                };
                ui.label(egui::RichText::new(msg).color(col).strong().size(12.0));
            } else {
                ui.label(
                    egui::RichText::new("Ready. Select a tab or follow the installer steps.")
                        .size(11.0)
                        .color(Color32::from_rgb(100, 116, 139)),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new("Knox Knowledge Graph Engine (MIT)")
                        .size(11.0)
                        .color(Color32::from_rgb(100, 116, 139)),
                );
            });
        });
    }
}

impl eframe::App for KnoxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set dark slate background style
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = COLOR_BG_DARK;
        ctx.set_visuals(visuals);

        TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(4.0);
            self.render_top_header(ui);
            ui.add_space(6.0);
            self.render_navigation_bar(ui);
            ui.add_space(4.0);
        });

        TopBottomPanel::bottom("bottom_status").show(ctx, |ui| {
            ui.add_space(4.0);
            self.render_bottom_status_bar(ui);
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            render_active_view(ui, &mut self.state, &mut self.canvas);
        });
    }
}
