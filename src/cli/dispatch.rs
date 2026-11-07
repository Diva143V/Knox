//! Knox command dispatch logic.

use anyhow::Context;

use crate::cli::args::{
    AskArgs, CheckArgs, Commands, DoctorArgs, ExportArgs, HookAction, HookArgs, ImportArgs, IngestArgs, InitArgs, KnoxCli,
    MetricsArgs, MigrationAction, MigrationArgs, ModelArgs, ModelSubcommand, QueryArgs, ScanArgs, StatusArgs, TraceArgs,
};
use crate::commands::{
    cmd_ask, cmd_check, cmd_doctor, cmd_export, cmd_hook_install, cmd_hook_uninstall, cmd_ingest, cmd_init, cmd_metrics,
    cmd_query, cmd_scan, cmd_status, cmd_trace, export_to_file, format_ask_response, format_doctor_report, format_ingest_report,
    format_metrics_report, format_query_results_json, format_query_results_table,
    format_scan_report, format_status_json, format_status_report, import_from_file,
    migration_apply, migration_check, migration_restore, migration_split,
};
use crate::core::config::KnoxConfig;
use crate::storage::db::KnoxDb;

/// Opens the Knox SQLite database with user-friendly error context.
pub fn open_db(config: &KnoxConfig) -> anyhow::Result<KnoxDb> {
    let db_path = config.resolve_db_path();
    KnoxDb::open(&db_path).with_context(|| {
        format!(
            "Failed to open database at {}. Run 'knox init' to initialize your workspace.",
            db_path.display()
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
        Some(Commands::Hook(args)) => handle_hook(args, config),
        Some(Commands::Migrate(args)) => handle_migrate(args, config),
        Some(Commands::Export(args)) => handle_export(args, &open_db(config)?),
        Some(Commands::Import(args)) => handle_import(args, &open_db(config)?),
        Some(Commands::Doctor(args)) => handle_doctor(args, &open_db(config)?),
        Some(Commands::Metrics(args)) => handle_metrics(args, &open_db(config)?),
        Some(Commands::Model(args)) => handle_model(args, config),
        Some(Commands::Ask(args)) => handle_ask(args, &open_db(config)?),
        Some(Commands::Check(args)) => handle_check(args, &open_db(config)?),
        Some(Commands::Trace(args)) => handle_trace(args, &open_db(config)?),
        Some(Commands::Serve(_args)) => handle_serve(config),
    }
}

pub fn handle_doctor(args: DoctorArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let report = cmd_doctor(db, args.project.as_deref())?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", format_doctor_report(&report));
    }
    Ok(())
}

pub fn handle_metrics(args: MetricsArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let report = cmd_metrics(db, args.project.as_deref(), args.calibration)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", format_metrics_report(&report));
    }
    Ok(())
}

pub fn handle_model(args: ModelArgs, config: &KnoxConfig) -> anyhow::Result<()> {
    match args.action {
        Some(ModelSubcommand::Test { query }) => {
            crate::commands::cmd_model_test(config, &query)?;
        }
        Some(ModelSubcommand::Status) | None => {
            crate::commands::cmd_model_status(config)?;
        }
    }
    Ok(())
}

pub fn handle_ask(args: AskArgs, db: &KnoxDb) -> anyhow::Result<()> {
    // If project is not explicitly passed, attempt resolving project from current directory "."
    let project_arg = match args.project {
        Some(p) => Some(p),
        None => {
            if let Ok(Some(proj)) = crate::storage::sessions::resolve_project_identifier(db, ".") {
                Some(proj)
            } else {
                None
            }
        }
    };
    let params = serde_json::json!({
        "question": args.question,
        "project": project_arg,
        "file_path": args.file,
        "strict_citations": args.strict,
    });
    let res = cmd_ask(db, params).context("Failed to execute knox ask")?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        print!("{}", format_ask_response(&res));
    }
    Ok(())
}

pub fn handle_check(args: CheckArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let mut params = serde_json::json!({
        "files": args.files,
        "budget": args.budget,
        "strict": args.strict,
        "format": if args.json { "json" } else { "markdown" },
    });
    if let Some(intent) = args.intent {
        params["intent"] = serde_json::json!(intent);
    }
    if let Some(project) = args.project {
        params["project"] = serde_json::json!(project);
    }
    let res = cmd_check(db, params).context("Failed to execute knox check")?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else if let Some(md) = res.get("markdown").and_then(serde_json::Value::as_str) {
        println!("{md}");
    } else {
        println!("{}", serde_json::to_string_pretty(&res)?);
    }
    Ok(())
}

pub fn handle_trace(args: TraceArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let mut params = serde_json::json!({
        "depth": args.depth,
        "max_nodes": args.max_nodes,
        "direction": args.direction,
        "budget": args.budget,
        "format": if args.json { "json" } else { "markdown" },
    });
    if let Some(from) = args.from {
        params["from"] = serde_json::json!(from);
    }
    if let Some(file) = args.file {
        params["file_path"] = serde_json::json!(file);
    }
    let res = cmd_trace(db, params).context("Failed to execute knox trace")?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else if let Some(md) = res.get("markdown").and_then(serde_json::Value::as_str) {
        println!("{md}");
    } else {
        println!("{}", serde_json::to_string_pretty(&res)?);
    }
    Ok(())
}

pub fn handle_migrate(args: MigrationArgs, config: &KnoxConfig) -> anyhow::Result<()> {
    let path = args.db.unwrap_or_else(|| config.resolve_db_path());
    match args.action {
        MigrationAction::Check => println!(
            "{}",
            serde_json::to_string_pretty(&migration_check(&path)?)?
        ),
        MigrationAction::Apply => {
            let (report, backup) = migration_apply(&path)?;
            println!(
                "Migration applied. Backup: {}\n{}",
                backup.display(),
                serde_json::to_string_pretty(&report)?
            );
        }
        MigrationAction::Restore => println!("Restored {}", migration_restore(&path)?.display()),
        MigrationAction::Split { output } => {
            for created in migration_split(&path, &output)? {
                println!("{}", created.display());
            }
        }
    }
    Ok(())
}

pub fn handle_init(args: InitArgs, config: &KnoxConfig) -> anyhow::Result<()> {
    cmd_init(&args.path, config)
        .with_context(|| format!("Failed to initialize workspace at {}", args.path.display()))?;
    // Registry failures must never make a successful init look broken.
    if let Err(err) = crate::commands::register_workspace(&args.path) {
        eprintln!("warning: could not update the Knox project registry: {err}");
    }
    println!("Initialized Knox workspace at {}", args.path.display());
    Ok(())
}

pub fn handle_export(args: ExportArgs, db: &KnoxDb) -> anyhow::Result<()> {
    match args.output {
        Some(path) => {
            let report = export_to_file(db, args.project.as_deref(), &path)
                .with_context(|| format!("Failed to export to {}", path.display()))?;
            println!(
                "Exported {} projects, {} sessions, {} actions to {}",
                report.projects,
                report.sessions,
                report.actions,
                path.display()
            );
        }
        None => {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            let report = cmd_export(db, args.project.as_deref(), &mut lock)
                .context("Failed to export project memory")?;
            eprintln!(
                "Exported {} projects, {} sessions, {} actions",
                report.projects, report.sessions, report.actions
            );
        }
    }
    Ok(())
}

pub fn handle_import(args: ImportArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let report = import_from_file(db, &args.input)
        .with_context(|| format!("Failed to import {}", args.input.display()))?;
    println!(
        "Imported {} projects, {} sessions, {} actions ({} already present)",
        report.projects, report.sessions, report.actions, report.skipped
    );
    Ok(())
}

pub fn handle_ingest(args: IngestArgs, db: &KnoxDb) -> anyhow::Result<()> {
    let report = cmd_ingest(&args.transcript_path, db, args.explain).with_context(|| {
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
    let result = cmd_query(
        &args.pattern,
        db,
        None,
        args.project,
        args.file,
        Some(args.limit),
    )
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

pub fn handle_hook(args: HookArgs, config: &KnoxConfig) -> anyhow::Result<()> {
    match args.action {
        HookAction::Install => {
            cmd_hook_install(&args.project).with_context(|| {
                format!("Failed to install git hooks in {}", args.project.display())
            })?;
            println!(
                "Installed Knox git hooks in {}",
                args.project.display()
            );
        }
        HookAction::Uninstall => {
            cmd_hook_uninstall(&args.project).with_context(|| {
                format!(
                    "Failed to uninstall git hooks from {}",
                    args.project.display()
                )
            })?;
            println!(
                "Uninstalled Knox git hooks from {}",
                args.project.display()
            );
        }
        HookAction::RecordCommit => {
            let db = open_db(config)?;
            if let Some(id) = crate::commands::hook::cmd_hook_record_commit(
                &args.project,
                &db,
                args.message_file.as_deref(),
                args.message.as_deref(),
            ).with_context(|| {
                format!("Failed to record commit in {}", args.project.display())
            })? {
                println!("Recorded commit into action {}", id);
            }
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

    let db_path = config.resolve_db_path();
    if db_path.exists() {
        if let Ok(db) = KnoxDb::open(&db_path) {
            if let Ok(report) = crate::commands::cmd_status(&db) {
                println!("Current Database Status ({}):", db_path.display());
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
