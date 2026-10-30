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
    pub command: Commands,
}

/// Available subcommands for Knox.
#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum Commands {
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
    /// Install git pre-commit hook.
    Install,
    /// Uninstall git pre-commit hook.
    Uninstall,
}

/// Arguments for `knox hook`.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct HookArgs {
    /// Hook operation to execute (install or uninstall).
    #[arg(value_enum)]
    pub action: HookAction,

    /// Path to project repository (defaults to current directory).
    #[arg(short, long, default_value = ".")]
    pub project: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init_defaults_and_custom_path() {
        let cli_default = KnoxCli::try_parse_from(["knox", "init"]).unwrap();
        match cli_default.command {
            Commands::Init(args) => assert_eq!(args.path, PathBuf::from(".")),
            _ => panic!("Expected Init command"),
        }

        let cli_custom = KnoxCli::try_parse_from(["knox", "init", "subfolder/project"]).unwrap();
        match cli_custom.command {
            Commands::Init(args) => assert_eq!(args.path, PathBuf::from("subfolder/project")),
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_parse_ingest_argument() {
        let cli = KnoxCli::try_parse_from(["knox", "ingest", "fixtures/transcript.jsonl"]).unwrap();
        match cli.command {
            Commands::Ingest(args) => {
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
            Commands::Query(args) => {
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
    fn test_parse_status_flags() {
        let cli = KnoxCli::try_parse_from(["knox", "status", "--json"]).unwrap();
        match cli.command {
            Commands::Status(args) => assert!(args.json),
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_parse_serve_flags() {
        let cli = KnoxCli::try_parse_from(["knox", "serve", "--stdio"]).unwrap();
        match cli.command {
            Commands::Serve(args) => assert!(args.stdio),
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_parse_scan_argument() {
        let cli_default = KnoxCli::try_parse_from(["knox", "scan"]).unwrap();
        match cli_default.command {
            Commands::Scan(args) => assert_eq!(args.project, PathBuf::from(".")),
            _ => panic!("Expected Scan command"),
        }

        let cli_custom = KnoxCli::try_parse_from(["knox", "scan", "custom/dir"]).unwrap();
        match cli_custom.command {
            Commands::Scan(args) => assert_eq!(args.project, PathBuf::from("custom/dir")),
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_parse_hook_actions() {
        let cli_install = KnoxCli::try_parse_from(["knox", "hook", "install"]).unwrap();
        match cli_install.command {
            Commands::Hook(args) => {
                assert_eq!(args.action, HookAction::Install);
                assert_eq!(args.project, PathBuf::from("."));
            }
            _ => panic!("Expected Hook command"),
        }

        let cli_uninstall =
            KnoxCli::try_parse_from(["knox", "hook", "uninstall", "--project", "repo_dir"])
                .unwrap();
        match cli_uninstall.command {
            Commands::Hook(args) => {
                assert_eq!(args.action, HookAction::Uninstall);
                assert_eq!(args.project, PathBuf::from("repo_dir"));
            }
            _ => panic!("Expected Hook command"),
        }
    }
}
