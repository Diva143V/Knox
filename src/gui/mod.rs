//! Knox Desktop GUI and classic installer application subsystem.

pub mod app;
pub mod canvas;
pub mod state;
pub mod theme;
pub mod views;

pub use app::KnoxApp;
pub use state::{AppState, GuiTab};

use crate::cli::GuiArgs;
use crate::core::config::KnoxConfig;
use crate::core::error::{KnoxError, KnoxResult};

/// Launches the interactive Knox desktop GUI window.
pub fn run_gui(config: &KnoxConfig, args: GuiArgs) -> KnoxResult<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Knox 🛡️ v1.1.0 — Knowledge Graph & Installer"),
        ..Default::default()
    };

    let cfg_clone = config.clone();
    let initial_path = args.path;

    eframe::run_native(
        "Knox 🛡️ Knowledge Graph",
        options,
        Box::new(move |_cc| Ok(Box::new(KnoxApp::new(cfg_clone, initial_path)))),
    )
    .map_err(|e| KnoxError::InvalidInput(format!("Failed to launch GUI: {}", e)))
}
