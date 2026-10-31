//! Knox command dispatch logic.

use anyhow::Context;

use crate::cli::args::{
    Commands, HookAction, HookArgs, IngestArgs, InitArgs, KnoxCli, QueryArgs, ScanArgs, StatusArgs,
};
use crate::commands::{
    cmd_hook_install, cmd_hook_uninstall, cmd_ingest, cmd_init, cmd_query, cmd_scan, cmd_status,
    format_ingest_report, format_query_results_json, format_query_results_table,
    format_scan_report, format_status_json, format_status_report,
};
use crate::core::config::KnoxConfig;
use crate::storage::db::KnoxDb;

/// Opens the Knox SQLite database with user-friendly error context.
pub fn open_db(config: &KnoxConfig) -> anyhow::Result<KnoxDb> {
    KnoxDb::open(&config.kg_path).with_context(|| {
        format!(
            "Failed to open database at {}. Run 'knox init' to initialize your workspace.",
            config.kg_path.display()
        )
    })
}

/// Dispatches parsed CLI commands to their respective command implementations.
pub fn dispatch(cli: KnoxCli, config: &KnoxConfig) -> anyhow::Result<()> {
    match cli.command {
        Some(Commands::Gui) | None => print_welcome_guide(config),
        Some(Commands::Init(args)) => handle_init(args, config),
        Some(Commands::Ingest(args)) => handle_ingest(args, &open_db(config)?),
        Some(Commands::Query(args)) => handle_query(args, &open_db(config)?),
        Some(Commands::Status(args)) => handle_status(args, &open_db(config)?),
        Some(Commands::Scan(args)) => handle_scan(args, &open_db(config)?),
        Some(Commands::Hook(args)) => handle_hook(args),
        Some(Commands::Serve(_args)) => handle_serve(config),
    }
}

pub fn handle_init(args: InitArgs, config: &KnoxConfig) -> anyhow::Result<()> {
    cmd_init(&args.path, config)
        .with_context(|| format!("Failed to initialize workspace at {}", args.path.display()))?;
    println!("Initialized Knox workspace at {}", args.path.display());
    Ok(())
}

pub fn handle_ingest(args: IngestArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let report = cmd_ingest(&args.transcript_path, db).with_context(|| {
        format!(
            "Failed to ingest transcript {}",
            args.transcript_path.display()
        )
    })?;
    println!("{}", format_ingest_report(&report));
    Ok(())
}

pub fn handle_scan(args: ScanArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let report = cmd_scan(&args.project, db).with_context(|| {
        format!(
            "Failed to scan staged git files in {}",
            args.project.display()
        )
    })?;
    println!("{}", format_scan_report(&report));
    Ok(())
}

pub fn handle_serve(config: &KnoxConfig) -> anyhow::Result<()> {
    let db = open_db(config)?;
    let rt = tokio::runtime::Runtime::new()
        .context("Failed to initialize Tokio runtime for MCP server")?;
    rt.block_on(crate::mcp::run_mcp_server(db))
        .context("MCP server terminated with error")
}

pub fn handle_query(args: QueryArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let result = cmd_query(&args.pattern, db, None)
        .with_context(|| format!("Query failed for pattern '{}'", args.pattern))?;

    if args.json {
        let json_text = format_query_results_json(&result)?;
        println!("{}", json_text);
    } else {
        println!("{}", format_query_results_table(&result));
    }
    Ok(())
}

pub fn handle_status(args: StatusArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let report = cmd_status(db).context("Failed to generate status report")?;

    if args.json {
        let json_text = format_status_json(&report)?;
        println!("{}", json_text);
    } else {
        println!("{}", format_status_report(&report));
    }
    Ok(())
}

pub fn handle_hook(args: HookArgs) -> anyhow::Result<()> {
    match args.action {
        HookAction::Install => {
            cmd_hook_install(&args.project).with_context(|| {
                format!("Failed to install git hook in {}", args.project.display())
            })?;
            println!(
                "Installed Knox pre-commit hook in {}",
                args.project.display()
            );
        }
        HookAction::Uninstall => {
            cmd_hook_uninstall(&args.project).with_context(|| {
                format!(
                    "Failed to uninstall git hook from {}",
                    args.project.display()
                )
            })?;
            println!(
                "Uninstalled Knox pre-commit hook from {}",
                args.project.display()
            );
        }
    }
    Ok(())
}

/// Prints a friendly introductory guide when Knox is executed without subcommands.
pub fn print_welcome_guide(config: &KnoxConfig) -> anyhow::Result<()> {
    println!(
        "Knox 🛡️ v{} — Knowledge Graph for AI-Assisted Development",
        env!("CARGO_PKG_VERSION")
    );
    println!("=======================================================\n");
    println!("Knox is a Command-Line Interface (CLI), Desktop App, and MCP server.\n");

    if config.kg_path.exists() {
        if let Ok(db) = KnoxDb::open(&config.kg_path) {
            if let Ok(report) = crate::commands::cmd_status(&db) {
                println!("Current Database Status ({}):", config.kg_path.display());
                println!("  • Total Actions:    {}", report.total_actions);
                println!("  • Total Sessions:   {}", report.total_sessions);
                println!("  • Projects Tracked: {}", report.projects_tracked);
                println!();
            }
        }
    }

    println!("Common Commands (run from terminal):");
    println!("  knox.exe gui                 Launch interactive Knox Desktop application");
    println!("  knox.exe init .              Initialize Knox in current project workspace");
    println!("  knox.exe status              Display knowledge graph metrics and stats");
    println!("  knox.exe query <PATTERN>     Query actions/decisions by file, tag, or keyword");
    println!("  knox.exe scan                Detect unrecorded human edits in staged git changes");
    println!("  knox.exe hook install        Install automatic git pre-commit hook");
    println!("  knox.exe serve --stdio       Start MCP server for Cursor, Claude, Antigravity\n");
    println!("For full documentation and all options, run: knox.exe --help");

    Ok(())
}

