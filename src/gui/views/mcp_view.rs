//! Step 8: MCP server monitor and IDE integration configuration generator.

use eframe::egui::{self, ScrollArea, Ui};

use crate::gui::state::{AppState, GuiTab};
use crate::gui::theme::{card_frame, render_installer_header};
use crate::mcp::tools::all_tools;

/// Renders the MCP Server and IDE Integration configuration view.
pub fn render_mcp_view(ui: &mut Ui, state: &mut AppState) {
    render_installer_header(
        ui,
        8,
        "MCP Server & IDE Integration",
        "Connect Cursor, Claude Desktop, and Antigravity to Knox via Model Context Protocol",
    );

    render_ide_config_cards(ui, state);
    ui.add_space(12.0);

    render_tools_directory(ui);
    ui.add_space(16.0);

    render_nav_buttons(ui, state);
}

fn render_ide_config_cards(ui: &mut Ui, state: &mut AppState) {
    card_frame().show(ui, |ui| {
        ui.heading("One-Click IDE Configuration");
        ui.label(
            "Add this JSON configuration snippet to your Cursor (Settings → MCP) or \
             claude_desktop_config.json:",
        );
        ui.add_space(4.0);

        let exe_path = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("knox.exe"))
            .display()
            .to_string()
            .replace('\\', "\\\\");

        let json_config = format!(
            "{{\n  \"mcpServers\": {{\n    \"knox\": {{\n      \"command\": \"{}\",\n      \"args\": [\"serve\", \"--stdio\"]\n    }}\n  }}\n}}",
            exe_path
        );

        ui.monospace(&json_config);
        ui.add_space(6.0);

        if ui.button("📋 Copy JSON to Clipboard").clicked() {
            ui.ctx().copy_text(json_config);
            state.set_toast("MCP configuration copied to clipboard!", false);
        }
    });
}

fn render_tools_directory(ui: &mut Ui) {
    card_frame().show(ui, |ui| {
        ui.heading("Registered MCP Tools (9 Tools)");
        ui.separator();

        ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for tool in all_tools() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&tool.name).strong());
                    ui.label(format!("— {}", tool.description));
                });
            }
        });
    });
}

fn render_nav_buttons(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("⏴ Back: Human Edit Scanner").clicked() {
            state.current_tab = GuiTab::Scanner;
        }
        ui.add_space(8.0);
        if ui.button("Return to Welcome 🏠").clicked() {
            state.current_tab = GuiTab::Welcome;
        }
        if ui.button("Explore Knowledge Graph 🕸️").clicked() {
            state.current_tab = GuiTab::Graph;
        }
    });
}
