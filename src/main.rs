//! Knox CLI binary entry point.
//!
//! Parses command-line arguments using `clap` and dispatches them to Layer 3 command handlers.

use anyhow::Context;
use clap::Parser;

use knox::cli::{
    Commands, HookAction, HookArgs, IngestArgs, InitArgs, KnoxCli, QueryArgs, ScanArgs, StatusArgs,
};
use knox::commands::{
    cmd_hook_install, cmd_hook_uninstall, cmd_ingest, cmd_init, cmd_query, cmd_scan, cmd_status,
    format_ingest_report, format_query_results_json, format_query_results_table,
    format_scan_report, format_status_json, format_status_report,
};
use knox::core::config::KnoxConfig;
use knox::storage::db::KnoxDb;

fn main() -> anyhow::Result<()> {
    // Initialize logging subscriber (KNOX_LOG or RUST_LOG, defaulting to WARN)
    let env_filter = std::env::var("KNOX_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .unwrap_or_else(|_| "warn".to_string());

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(env_filter))
        .init();

    let config = KnoxConfig::from_env();

    if std::env::args().len() <= 1 {
        return print_welcome_guide(&config);
    }

    let cli = KnoxCli::parse();
    dispatch(cli, &config)
}

/// Prints a friendly introductory guide when Knox is executed without subcommands.
fn print_welcome_guide(config: &KnoxConfig) -> anyhow::Result<()> {
    println!(
        "Knox 🛡️ v{} — Knowledge Graph for AI-Assisted Development",
        env!("CARGO_PKG_VERSION")
    );
    println!("=======================================================\n");
    println!("Knox is a Command-Line Interface (CLI) and Model Context Protocol (MCP) server.");
    println!("It runs in the background of your editor or from the terminal (not a GUI app).\n");

    if config.kg_path.exists() {
        if let Ok(db) = KnoxDb::open(&config.kg_path) {
            if let Ok(report) = knox::commands::cmd_status(&db) {
                println!("Current Database Status ({}):", config.kg_path.display());
                println!("  • Total Actions:    {}", report.total_actions);
                println!("  • Total Sessions:   {}", report.total_sessions);
                println!("  • Projects Tracked: {}", report.projects_tracked);
                println!();
            }
        }
    }

    println!("Common Commands (run from terminal):");
    println!("  knox.exe init .              Initialize Knox in current project workspace");
    println!("  knox.exe status              Display knowledge graph metrics and stats");
    println!("  knox.exe query <PATTERN>     Query actions/decisions by file, tag, or keyword");
    println!("  knox.exe scan                Detect unrecorded human edits in staged git changes");
    println!("  knox.exe hook install        Install automatic git pre-commit hook");
    println!("  knox.exe serve --stdio       Start MCP server for Cursor, Claude, Antigravity\n");
    println!("For full documentation and all options, run: knox.exe --help");

    Ok(())
}

/// Opens the Knox SQLite database with user-friendly error context.
fn open_db(config: &KnoxConfig) -> anyhow::Result<KnoxDb> {
    KnoxDb::open(&config.kg_path).with_context(|| {
        format!(
            "Failed to open database at {}. Run 'knox init' to initialize your workspace.",
            config.kg_path.display()
        )
    })
}

/// Dispatches parsed CLI commands to their respective command implementations.
fn dispatch(cli: KnoxCli, config: &KnoxConfig) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init(args) => handle_init(args, config),
        Commands::Ingest(args) => handle_ingest(args, &open_db(config)?),
        Commands::Query(args) => handle_query(args, &open_db(config)?),
        Commands::Status(args) => handle_status(args, &open_db(config)?),
        Commands::Scan(args) => handle_scan(args, &open_db(config)?),
        Commands::Hook(args) => handle_hook(args),
        Commands::Serve(_args) => handle_serve(config),
    }
}

fn handle_init(args: InitArgs, config: &KnoxConfig) -> anyhow::Result<()> {
    cmd_init(&args.path, config)
        .with_context(|| format!("Failed to initialize workspace at {}", args.path.display()))?;
    println!("Initialized Knox workspace at {}", args.path.display());
    Ok(())
}

fn handle_ingest(args: IngestArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let report = cmd_ingest(&args.transcript_path, db).with_context(|| {
        format!(
            "Failed to ingest transcript {}",
            args.transcript_path.display()
        )
    })?;
    println!("{}", format_ingest_report(&report));
    Ok(())
}

fn handle_scan(args: ScanArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let report = cmd_scan(&args.project, db).with_context(|| {
        format!(
            "Failed to scan staged git files in {}",
            args.project.display()
        )
    })?;
    println!("{}", format_scan_report(&report));
    Ok(())
}

fn handle_serve(config: &KnoxConfig) -> anyhow::Result<()> {
    let db = open_db(config)?;
    let rt = tokio::runtime::Runtime::new()
        .context("Failed to initialize Tokio runtime for MCP server")?;
    rt.block_on(knox::mcp::run_mcp_server(db))
        .context("MCP server terminated with error")
}

/// Executes and formats results for `knox query`.
fn handle_query(args: QueryArgs, db: &KnoxDb) -> anyhow::Result<()> {
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

/// Executes and formats results for `knox status`.
fn handle_status(args: StatusArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let report = cmd_status(db).context("Failed to generate status report")?;

    if args.json {
        let json_text = format_status_json(&report)?;
        println!("{}", json_text);
    } else {
        println!("{}", format_status_report(&report));
    }
    Ok(())
}

/// Handles git hook installation and removal.
fn handle_hook(args: HookArgs) -> anyhow::Result<()> {
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
