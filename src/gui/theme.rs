//! Styling, color schemes, and visual helpers for the Knox desktop GUI.

use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, Stroke};

/// Primary palette colors.
pub const COLOR_BG_DARK: Color32 = Color32::from_rgb(15, 23, 42);
pub const COLOR_PANEL_BG: Color32 = Color32::from_rgb(30, 41, 59);
pub const COLOR_PRIMARY: Color32 = Color32::from_rgb(59, 130, 246);
pub const COLOR_SUCCESS: Color32 = Color32::from_rgb(16, 185, 129);
pub const COLOR_PLAN: Color32 = Color32::from_rgb(139, 92, 246);
pub const COLOR_DECIDE: Color32 = Color32::from_rgb(245, 158, 11);
pub const COLOR_DEBUG: Color32 = Color32::from_rgb(239, 68, 68);
pub const COLOR_FILE: Color32 = Color32::from_rgb(6, 182, 212);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);

/// Returns the characteristic color for a given action type.
pub fn action_color(action_type: &str) -> Color32 {
    match action_type.to_lowercase().as_str() {
        "write" | "human_write" => COLOR_SUCCESS,
        "plan" => COLOR_PLAN,
        "decide" => COLOR_DECIDE,
        "debug" => COLOR_DEBUG,
        _ => COLOR_PRIMARY,
    }
}

/// Returns the characteristic color for an actor name.
pub fn actor_color(actor: &str) -> Color32 {
    match actor.to_lowercase().as_str() {
        "human" => Color32::from_rgb(236, 72, 153),
        "antigravity" => COLOR_PRIMARY,
        "cursor" => Color32::from_rgb(168, 85, 247),
        "copilot" => Color32::from_rgb(56, 189, 248),
        _ => COLOR_TEXT_MUTED,
    }
}

/// Renders a pill badge with custom background and text color.
pub fn render_badge(ui: &mut egui::Ui, text: &str, bg: Color32, fg: Color32) {
    Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(fg).size(11.0).strong());
        });
}

/// Returns a styled card frame for dashboard sections.
pub fn card_frame() -> Frame {
    Frame::new()
        .fill(COLOR_PANEL_BG)
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
        .inner_margin(Margin::same(12))
}

/// Renders the classic installer wizard step banner.
pub fn render_installer_header(ui: &mut egui::Ui, step: usize, title: &str, subtitle: &str) {
    ui.horizontal(|ui| {
        render_badge(ui, &format!("STEP {}", step), COLOR_PRIMARY, Color32::WHITE);
        ui.heading(egui::RichText::new(title).strong().size(18.0));
    });
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(subtitle)
            .color(COLOR_TEXT_MUTED)
            .size(12.0),
    );
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
}
