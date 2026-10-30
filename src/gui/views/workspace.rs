//! Step 2: Workspace initialization wizard, knox.md generator, and git hook management.

use std::path::PathBuf;

use eframe::egui::{self, Color32, Ui};

use crate::commands::{cmd_hook_install, cmd_hook_uninstall, cmd_init};
use crate::gui::state::{AppState, GuiTab};
use crate::gui::theme::{COLOR_SUCCESS, card_frame, render_badge, render_installer_header};

/// Renders the Workspace setup and Git Hook management view.
pub fn render_workspace_view(ui: &mut Ui, state: &mut AppState) {
    render_installer_header(
        ui,
        2,
        "Workspace & Git Hook Setup",
        "Initialize project directory, generate knox.md, and install pre-commit protection",
    );

    render_workspace_picker(ui, state);
    ui.add_space(12.0);

    render_hook_manager(ui, state);
    ui.add_space(12.0);

    render_knox_md_preview(ui, state);
    ui.add_space(16.0);

    render_nav_buttons(ui, state);
}

fn render_workspace_picker(ui: &mut Ui, state: &mut AppState) {
    card_frame().show(ui, |ui| {
        ui.heading("Target Project Workspace");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let mut path_str = state.workspace_path.display().to_string();
            if ui.text_edit_singleline(&mut path_str).changed() {
                state.workspace_path = PathBuf::from(path_str);
            }

            if ui.button("📁 Browse...").clicked() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    state.workspace_path = folder;
                }
            }

            if ui
                .button(egui::RichText::new("🚀 Initialize Knox (init)").strong())
                .clicked()
            {
                handle_init_action(state);
            }
        });
    });
}

fn handle_init_action(state: &mut AppState) {
    let path = state.workspace_path.clone();
    match cmd_init(&path, &state.config) {
        Ok(()) => {
            state.refresh_data();
            let knox_md_path = path.join("knox.md");
            if let Ok(content) = std::fs::read_to_string(&knox_md_path) {
                state.knox_md_content = Some(content);
            }
            state.set_toast("Successfully initialized workspace and knox.md!", false);
        }
        Err(e) => {
            state.set_toast(format!("Init failed: {}", e), true);
        }
    }
}

fn render_hook_manager(ui: &mut Ui, state: &mut AppState) {
    card_frame().show(ui, |ui| {
        ui.heading("Git Pre-Commit Hook (Human Edit Protection)");
        ui.label(
            "The git hook runs 'knox scan' before every commit to ensure manual human code \
             changes are registered in the graph.",
        );
        ui.add_space(6.0);

        let hook_file = state
            .workspace_path
            .join(".git")
            .join("hooks")
            .join("pre-commit");
        let is_installed = hook_file.exists();
        state.is_hook_installed = is_installed;

        ui.horizontal(|ui| {
            let badge_text = if is_installed {
                "Hook Installed"
            } else {
                "Not Installed"
            };
            let badge_col = if is_installed {
                COLOR_SUCCESS
            } else {
                Color32::from_rgb(148, 163, 184)
            };
            render_badge(ui, badge_text, badge_col, Color32::WHITE);

            if !is_installed {
                if ui.button("Install Pre-Commit Hook").clicked() {
                    match cmd_hook_install(&state.workspace_path) {
                        Ok(()) => state.set_toast("Git pre-commit hook installed!", false),
                        Err(e) => state.set_toast(format!("Hook install error: {}", e), true),
                    }
                }
            } else if ui.button("Uninstall Hook").clicked() {
                match cmd_hook_uninstall(&state.workspace_path) {
                    Ok(()) => state.set_toast("Git hook uninstalled.", false),
                    Err(e) => state.set_toast(format!("Hook uninstall error: {}", e), true),
                }
            }
        });
    });
}

fn render_knox_md_preview(ui: &mut Ui, state: &AppState) {
    if let Some(ref content) = state.knox_md_content {
        card_frame().show(ui, |ui| {
            ui.heading("Generated knox.md Preview");
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    ui.monospace(content);
                });
        });
    }
}

fn render_nav_buttons(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("⏴ Back: Welcome").clicked() {
            state.current_tab = GuiTab::Welcome;
        }
        ui.add_space(8.0);
        if ui
            .button(
                egui::RichText::new("Next: Knowledge Graph Canvas ➔")
                    .size(14.0)
                    .strong(),
            )
            .clicked()
        {
            state.current_tab = GuiTab::Graph;
        }
    });
}
