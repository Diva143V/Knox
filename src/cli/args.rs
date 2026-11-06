//! Command-line argument definitions and parsers for Knox CLI.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Knox CLI entry point and root argument parser.
#[derive(Parser, Clone, Debug, PartialEq, Eq)]
#[command(
    name = "knox",
    version,
    about = "Knowledge graph for AI-assisted development",
    long_about = "Knox maintains a persistent, append-only knowledge graph from IDE conversation history and git staged changes."
)]
pub struct KnoxCli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands for Knox.
#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum Commands {
    /// Launch the interactive Knox Desktop GUI application.
    Gui,
    /// Initialize Knox in a project workspace and generate knox.md instructions.
    Init(InitArgs),
    /// Ingest and parse a conversation transcript file into the knowledge graph.
    Ingest(IngestArgs),
    /// Query actions and decisions in the knowledge graph.
    Query(QueryArgs),
    /// Display aggregate knowledge graph status and statistics.
    Status(StatusArgs),
    /// Start the MCP stdio server for IDE agent integration.
    Serve(ServeArgs),
    /// Scan git staged changes to detect unrecorded human edits.
    Scan(ScanArgs),
    /// Manage git pre-commit hook integration.
    Hook(HookArgs),
    /// Inspect, apply, restore, or split a database migration.
    Migrate(MigrationArgs),
    /// Export project memory as newline-delimited JSON.
    Export(ExportArgs),
    /// Import project memory from a JSONL export.
    Import(ImportArgs),
    /// Check operational health, query telemetry, and database integrity.
    Doctor(DoctorArgs),
    /// Display retrieval performance, token efficiency, and empirical metrics.
    Metrics(MetricsArgs),
    /// Inspect model profiles, test embedding and answering connectivity.
    Model(ModelArgs),
    /// Ask a technical question grounded in project memory with verified citations.
    Ask(AskArgs),
    /// Pre-flight guardrail check against active architectural constraints.
    Check(CheckArgs),
}

/// Arguments for `knox init`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct InitArgs {
    /// Path to project workspace to initialize (defaults to current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Arguments for `knox ingest`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct IngestArgs {
    /// Path to conversation transcript file (e.g. transcript.jsonl).
    pub transcript_path: PathBuf,
    /// Explain each classification decision (rule ID, confidence, reason).
    #[arg(long)]
    pub explain: bool,
}

/// Arguments for `knox query`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct QueryArgs {
    /// Search pattern (file path, keyword, or concept).
    pub pattern: String,

    /// Filter by project identifier or name.
    #[arg(short, long)]
    pub project: Option<String>,

    /// Filter by specific file path.
    #[arg(short, long)]
    pub file: Option<String>,

    /// Maximum number of results to display (default: 20).
    #[arg(short, long, default_value = "20")]
    pub limit: usize,

    /// Format output as JSON instead of tabular text.
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `knox status`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct StatusArgs {
    /// Format output as JSON.
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `knox serve`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct ServeArgs {
    /// Run using stdio JSON-RPC transport.
    #[arg(long, default_value_t = true)]
    pub stdio: bool,
}

/// Arguments for `knox scan`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct ScanArgs {
    /// Project workspace directory to scan (defaults to current directory).
    #[arg(default_value = ".")]
    pub project: PathBuf,
}

/// Action to perform for git hook management.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookAction {
    /// Install git pre-commit and post-commit hooks.
    Install,
    /// Uninstall git hooks.
    Uninstall,
    /// Record a git commit message into project memory.
    RecordCommit,
}

/// Arguments for `knox hook`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct HookArgs {
    /// Hook operation to execute.
    #[arg(value_enum)]
    pub action: HookAction,

    /// Path to project repository (defaults to current directory).
    #[arg(short, long, default_value = ".")]
    pub project: PathBuf,

    /// Path to file containing commit message (optional).
    #[arg(long)]
    pub message_file: Option<PathBuf>,

    /// Raw commit message string (optional).
    #[arg(long)]
    pub message: Option<String>,
}

/// Arguments for `knox doctor`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct DoctorArgs {
    /// Filter health diagnostics by project identifier.
    #[arg(short, long)]
    pub project: Option<String>,

    /// Format output as JSON instead of human-readable report.
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `knox metrics`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct MetricsArgs {
    /// Filter metrics by project identifier.
    #[arg(short, long)]
    pub project: Option<String>,

    /// Include empirical guardrail threshold calibration report (50 labeled pairs).
    #[arg(long)]
    pub calibration: bool,

    /// Format output as JSON instead of human-readable report.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct MigrationArgs {
    /// Migration operation.
    #[command(subcommand)]
    pub action: MigrationAction,
    /// Database path (defaults to the configured Knox database).
    #[arg(long)]
    pub db: Option<PathBuf>,
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum MigrationAction {
    /// Report schema and integrity state without writing.
    Check,
    /// Back up and apply pending migrations.
    Apply,
    /// Restore the newest adjacent migration backup.
    Restore,
    /// Create one filtered database per project.
    Split {
        /// Destination directory for split databases.
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct ExportArgs {
    /// Project path, ID, or unique name (defaults to every project in the database).
    #[arg(long)]
    pub project: Option<String>,
    /// Write to a file instead of stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct ImportArgs {
    /// JSONL file produced by `knox export`.
    pub input: PathBuf,
}

/// Arguments for `knox model`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct ModelArgs {
    #[command(subcommand)]
    pub action: Option<ModelSubcommand>,
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum ModelSubcommand {
    /// Display active model profile, endpoints, and configured models.
    Status,
    /// Test model connectivity, embedding generation, and answering against active profile.
    Test {
        /// Optional prompt or query to test.
        #[arg(default_value = "What architectural decisions have been made in Knox?")]
        query: String,
    },
}

/// Arguments for `knox ask`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct AskArgs {
    /// Natural language question to ask across project memory.
    pub question: String,

    /// Filter/scope to a specific project (defaults to current directory/project).
    #[arg(short, long)]
    pub project: Option<String>,

    /// Filter candidate context to a specific file path.
    #[arg(short, long)]
    pub file: Option<String>,

    /// Enforce strict citation verification (default: true).
    #[arg(long, default_value_t = true)]
    pub strict: bool,

    /// Format output as JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `knox check`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct CheckArgs {
    /// Target file paths to verify against active architectural constraints.
    #[arg(required = true)]
    pub files: Vec<String>,

    /// Proposed intention or action description to test against rejected alternatives.
    #[arg(short, long)]
    pub intent: Option<String>,

    /// Scope to a specific project.
    #[arg(short, long)]
    pub project: Option<String>,

    /// Token budget for check output (default: 600).
    #[arg(short, long, default_value = "600")]
    pub budget: usize,

    /// Enforce strict violation matching.
    #[arg(long)]
    pub strict: bool,

    /// Format output as JSON instead of human-readable report.
    #[arg(long)]
    pub json: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_defaults_to_none() {
        let cli = KnoxCli::try_parse_from(["knox"]).unwrap();
        assert_eq!(cli.command, None);
    }

    #[test]
    fn test_parse_gui_subcommand() {
        let cli = KnoxCli::try_parse_from(["knox", "gui"]).unwrap();
        assert_eq!(cli.command, Some(Commands::Gui));
    }

    #[test]
    fn test_parse_init_defaults_and_custom_path() {
        let cli_default = KnoxCli::try_parse_from(["knox", "init"]).unwrap();
        match cli_default.command {
            Some(Commands::Init(args)) => assert_eq!(args.path, PathBuf::from(".")),
            _ => panic!("Expected Init command"),
        }

        let cli_custom = KnoxCli::try_parse_from(["knox", "init", "subfolder/project"]).unwrap();
        match cli_custom.command {
            Some(Commands::Init(args)) => assert_eq!(args.path, PathBuf::from("subfolder/project")),
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_parse_ingest_argument() {
        let cli = KnoxCli::try_parse_from(["knox", "ingest", "fixtures/transcript.jsonl"]).unwrap();
        match cli.command {
            Some(Commands::Ingest(args)) => {
                assert_eq!(
                    args.transcript_path,
                    PathBuf::from("fixtures/transcript.jsonl")
                )
            }
            _ => panic!("Expected Ingest command"),
        }
    }

    #[test]
    fn test_parse_query_with_options() {
        let cli = KnoxCli::try_parse_from([
            "knox",
            "query",
            "parser",
            "--project",
            "knox-core",
            "--file",
            "src/parser.rs",
            "--limit",
            "50",
            "--json",
        ])
        .unwrap();

        match cli.command {
            Some(Commands::Query(args)) => {
                assert_eq!(args.pattern, "parser");
                assert_eq!(args.project, Some("knox-core".to_string()));
                assert_eq!(args.file, Some("src/parser.rs".to_string()));
                assert_eq!(args.limit, 50);
                assert!(args.json);
            }
            _ => panic!("Expected Query command"),
        }
    }

    #[test]
    fn test_parse_export_and_import() {
        let export = KnoxCli::try_parse_from([
            "knox",
            "export",
            "--project",
            ".",
            "--output",
            "memory.jsonl",
        ])
        .unwrap();
        match export.command {
            Some(Commands::Export(args)) => {
                assert_eq!(args.project.as_deref(), Some("."));
                assert_eq!(args.output, Some(PathBuf::from("memory.jsonl")));
            }
            _ => panic!("Expected Export command"),
        }

        let import = KnoxCli::try_parse_from(["knox", "import", "memory.jsonl"]).unwrap();
        match import.command {
            Some(Commands::Import(args)) => {
                assert_eq!(args.input, PathBuf::from("memory.jsonl"));
            }
            _ => panic!("Expected Import command"),
        }
    }

    #[test]
    fn test_parse_status_flags() {
        let cli = KnoxCli::try_parse_from(["knox", "status", "--json"]).unwrap();
        match cli.command {
            Some(Commands::Status(args)) => assert!(args.json),
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_parse_serve_flags() {
        let cli = KnoxCli::try_parse_from(["knox", "serve", "--stdio"]).unwrap();
        match cli.command {
            Some(Commands::Serve(args)) => assert!(args.stdio),
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_parse_scan_argument() {
        let cli_default = KnoxCli::try_parse_from(["knox", "scan"]).unwrap();
        match cli_default.command {
            Some(Commands::Scan(args)) => assert_eq!(args.project, PathBuf::from(".")),
            _ => panic!("Expected Scan command"),
        }

        let cli_custom = KnoxCli::try_parse_from(["knox", "scan", "custom/dir"]).unwrap();
        match cli_custom.command {
            Some(Commands::Scan(args)) => assert_eq!(args.project, PathBuf::from("custom/dir")),
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_parse_hook_actions() {
        let cli_install = KnoxCli::try_parse_from(["knox", "hook", "install"]).unwrap();
        match cli_install.command {
            Some(Commands::Hook(args)) => {
                assert_eq!(args.action, HookAction::Install);
                assert_eq!(args.project, PathBuf::from("."));
            }
            _ => panic!("Expected Hook command"),
        }

        let cli_uninstall =
            KnoxCli::try_parse_from(["knox", "hook", "uninstall", "--project", "repo_dir"])
                .unwrap();
        match cli_uninstall.command {
            Some(Commands::Hook(args)) => {
                assert_eq!(args.action, HookAction::Uninstall);
                assert_eq!(args.project, PathBuf::from("repo_dir"));
            }
            _ => panic!("Expected Hook command"),
        }
    }

    #[test]
    fn test_parse_migration_operations() {
        let cli =
            KnoxCli::try_parse_from(["knox", "migrate", "split", "--output", "copies"]).unwrap();
        match cli.command {
            Some(Commands::Migrate(args)) => {
                assert_eq!(
                    args.action,
                    MigrationAction::Split {
                        output: PathBuf::from("copies")
                    }
                );
            }
            _ => panic!("Expected Migrate command"),
        }
    }

    #[test]
    fn test_parse_ask_subcommand() {
        let cli = KnoxCli::try_parse_from([
            "knox",
            "ask",
            "What database is used?",
            "-p",
            "knox",
            "--json",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Ask(args)) => {
                assert_eq!(args.question, "What database is used?");
                assert_eq!(args.project, Some("knox".to_string()));
                assert!(args.json);
                assert!(args.strict);
            }
            _ => panic!("Expected Ask subcommand"),
        }
    }

    #[test]
    fn test_parse_check_subcommand() {
        let cli = KnoxCli::try_parse_from([
            "knox",
            "check",
            "src/db.rs",
            "src/lib.rs",
            "--intent",
            "Use Redis",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Check(args)) => {
                assert_eq!(args.files, vec!["src/db.rs", "src/lib.rs"]);
                assert_eq!(args.intent, Some("Use Redis".to_string()));
                assert_eq!(args.budget, 600);
                assert!(!args.strict);
            }
            _ => panic!("Expected Check subcommand"),
        }
    }
}
