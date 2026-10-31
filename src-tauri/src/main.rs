//! Knox Unified Binary (CLI + Desktop GUI).
//!
//! If launched without arguments or with `knox gui`, this launches the Tauri Desktop window.
//! If launched with subcommands (e.g. `knox serve --stdio`, `knox query`, `knox init`),
//! this attaches to the console and runs the headless CLI / MCP server.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use knox::cli::{dispatch, print_welcome_guide, Commands, KnoxCli};
use knox::core::config::KnoxConfig;

#[cfg(windows)]
fn attach_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    unsafe {
        // Attempt to attach to parent console if invoked from terminal.
        // If stdin/stdout are already pipes (e.g. MCP agent child process), AttachConsole fails harmlessly.
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_console() {}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // If double-clicked without arguments or explicitly passed 'gui', launch desktop window
    if args.len() <= 1 || (args.len() == 2 && args[1] == "gui") {
        knox_desktop_lib::run();
        return Ok(());
    }

    // Otherwise, attach to parent console (if on Windows) for CLI / MCP execution
    attach_console();

    // Initialize logging subscriber (KNOX_LOG or RUST_LOG, defaulting to WARN)
    let env_filter = std::env::var("KNOX_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .unwrap_or_else(|_| "warn".to_string());

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(env_filter))
        .init();

    let config = KnoxConfig::from_env();
    let cli = KnoxCli::parse();

    match cli.command {
        Some(Commands::Gui) => {
            knox_desktop_lib::run();
            Ok(())
        }
        Some(_) => dispatch(cli, &config),
        None => print_welcome_guide(&config),
    }
}
