//! Knox Unified Binary (CLI + Desktop GUI).
//!
//! If launched without arguments or with `knox gui`, this launches the Tauri Desktop window.
//! If launched with subcommands (e.g. `knox serve --stdio`, `knox query`, `knox init`),
//! this attaches to the console and runs the headless CLI / MCP server.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use knox::cli::{Commands, KnoxCli, dispatch, print_welcome_guide};
use knox::core::config::KnoxConfig;

#[cfg(windows)]
fn attach_console_if_needed() {
    use std::fs::OpenOptions;
    use std::os::windows::io::IntoRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_PIPE, GetFileType};
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE,
        STD_OUTPUT_HANDLE, SetStdHandle,
    };

    unsafe {
        let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
        // If stdout is already an inherited pipe (e.g. spawned by an AI tool via stdio),
        // do NOT attach or rebind console handles! Keep agent transport completely clean.
        let file_type = GetFileType(stdout_handle);
        if file_type == FILE_TYPE_PIPE {
            return;
        }

        // If running directly in an interactive terminal (CMD, PowerShell, Windows Terminal),
        // attach to parent console and rebind standard streams to CONOUT$ / CONIN$.
        if AttachConsole(ATTACH_PARENT_PROCESS) != 0 {
            if let Ok(file) = OpenOptions::new().write(true).open("CONOUT$") {
                let handle = file.into_raw_handle();
                SetStdHandle(STD_OUTPUT_HANDLE, handle);
                SetStdHandle(STD_ERROR_HANDLE, handle);
            }
            if let Ok(file) = OpenOptions::new().read(true).open("CONIN$") {
                let handle = file.into_raw_handle();
                SetStdHandle(STD_INPUT_HANDLE, handle);
            }
        }
    }
}

#[cfg(not(windows))]
fn attach_console_if_needed() {}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // If double-clicked without arguments or explicitly passed 'gui', launch desktop window
    if args.len() <= 1 || (args.len() == 2 && args[1] == "gui") {
        knox_desktop_lib::run();
        return Ok(());
    }

    // Otherwise, attach to parent console (if on Windows and not already piped) for CLI / MCP execution
    attach_console_if_needed();

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
