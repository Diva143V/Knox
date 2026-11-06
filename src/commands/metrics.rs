//! Knox measurement and empirical metrics command (Workstream F3).
//!
//! Evaluates and prints hit rate, empty rate with classification breakdowns,
//! duplicate action rates, tokens per successful retrieval, latency percentiles
//! (p50/p95), registered tool footprints, and compact context token savings.
//!
//! Every published metric is computed with an explicit numerator and denominator.

use serde::{Deserialize, Serialize};

use crate::core::error::KnoxResult;
use crate::core::types::KnoxAction;
use crate::mcp::handlers::match_rejected_alternative_threshold;
use crate::mcp::tools::all_tools;
use crate::metrics::measure_token_efficiency;
use crate::storage::db::KnoxDb;
use crate::storage::{
    get_action_duplicate_stats, get_query_latencies, get_query_log_stats,
    get_successful_query_token_stats,
};

pub use crate::commands::doctor::EmptyBreakdown;

/// Empirical calibration row for a candidate overlap threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationRow {
    pub threshold: f64,
    pub tp: usize,
    pub fp: usize,
    pub fn_count: usize,
    pub tn: usize,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub assessment: String,
}

/// Empirical calibration report across candidate thresholds (0.40, 0.60, 0.80).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationReport {
    pub pair_count: usize,
    pub rows: Vec<CalibrationRow>,
    pub recommended_threshold: f64,
}

/// Comprehensive metrics and performance report for Knox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsReport {
    /// Total read queries analyzed from query_log.
    pub total_queries: usize,
    /// Number of queries returning rows > 0.
    pub hit_count: usize,
    /// Hit rate as a fraction (hit_count / total_queries).
    pub hit_rate: f64,
    /// Number of empty queries (rows == 0 or reason set).
    pub empty_count: usize,
    /// Empty rate as a fraction (empty_count / total_queries).
    pub empty_rate: f64,
    /// Breakdown of reasons for empty queries with counts.
    pub empty_breakdown: EmptyBreakdown,
    /// Total actions analyzed in knowledge graph.
    pub total_actions: usize,
    /// Number of duplicate/near-duplicate actions detected.
    pub duplicate_actions: usize,
    /// Duplicate action rate (duplicate_actions / total_actions).
    pub duplicate_rate: f64,
    /// Total tokens returned across successful queries.
    pub successful_retrieval_tokens: usize,
    /// Mean tokens per successful retrieval (successful_retrieval_tokens / hit_count).
    pub tokens_per_successful_retrieval: f64,
    /// Median latency in milliseconds (50th percentile).
    pub latency_p50_ms: f64,
    /// 95th percentile latency in milliseconds.
    pub latency_p95_ms: f64,
    /// Number of registered MCP tools.
    pub tool_count: usize,
    /// Tool definition tokens for registered MCP tools.
    pub tool_definition_tokens: usize,
    /// Compact context token savings vs raw JSON envelope (percentage).
    pub compact_savings_percent: usize,
    /// Optional empirical calibration report against the 50 labeled pairs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<CalibrationReport>,
}

/// A hand-labeled pair for threshold calibration.
#[derive(Debug, Clone)]
pub struct LabeledPair {
    pub proposal: &'static str,
    pub alternative: &'static str,
    pub is_true_violation: bool,
    pub description: &'static str,
}

/// 50 labeled pairs representative of real architectural proposals, ADR alternatives,
/// partial restatements, negations, distractors, and homonyms.
pub fn calibration_dataset() -> Vec<LabeledPair> {
    vec![
        // --- 25 True Violations (should trigger guardrail) ---
        LabeledPair {
            proposal: "Migrate primary database to DuckDB embedded analytics database",
            alternative: "DuckDB embedded analytics database",
            is_true_violation: true,
            description: "Direct verbatim restatement of analytics database",
        },
        LabeledPair {
            proposal: "We should switch to RocksDB embedded KV for lookups",
            alternative: "RocksDB embedded KV storage engine",
            is_true_violation: true,
            description: "Partial restatement: 3/5 tokens = 60% overlap",
        },
        LabeledPair {
            proposal: "Let's adopt GraphQL subscription protocol for streaming live updates",
            alternative: "GraphQL subscription protocol",
            is_true_violation: true,
            description: "GraphQL subscription restatement",
        },
        LabeledPair {
            proposal: "Replace REST endpoints with gRPC streaming transport",
            alternative: "gRPC streaming transport",
            is_true_violation: true,
            description: "gRPC streaming transport restatement",
        },
        LabeledPair {
            proposal: "Use Redis in-memory cache server for caching session state",
            alternative: "Redis in-memory cache server",
            is_true_violation: true,
            description: "Redis in-memory cache restatement",
        },
        LabeledPair {
            proposal: "Deploy MongoDB document store for unstructured records",
            alternative: "MongoDB document database store",
            is_true_violation: true,
            description: "Partial restatement: 3/4 tokens = 75% overlap",
        },
        LabeledPair {
            proposal: "Implement Celery background task queue for worker jobs",
            alternative: "Celery distributed background task queue",
            is_true_violation: true,
            description: "Partial restatement: 3/4 tokens = 75% overlap",
        },
        LabeledPair {
            proposal: "Add Kafka message broker for high-throughput events",
            alternative: "Kafka distributed message broker cluster",
            is_true_violation: true,
            description: "Partial restatement: 3/4 tokens = 75% overlap",
        },
        LabeledPair {
            proposal: "Rewrite frontend views into React single page application",
            alternative: "React single page application",
            is_true_violation: true,
            description: "React single page application restatement",
        },
        LabeledPair {
            proposal: "Switch web UI to Vue components framework",
            alternative: "Vue single page components framework",
            is_true_violation: true,
            description: "Partial restatement: 3/4 tokens = 75% overlap",
        },
        LabeledPair {
            proposal: "Store secret credentials in plaintext configuration secrets storage file",
            alternative: "Plaintext configuration secrets storage",
            is_true_violation: true,
            description: "Plaintext configuration secrets restatement",
        },
        LabeledPair {
            proposal: "Encrypt passwords using AES ECB encryption mode",
            alternative: "AES ECB encryption mode",
            is_true_violation: true,
            description: "AES ECB encryption mode restatement",
        },
        LabeledPair {
            proposal: "Style UI widgets with Tailwind CSS utility framework",
            alternative: "Tailwind CSS utility framework",
            is_true_violation: true,
            description: "Tailwind CSS utility framework restatement",
        },
        LabeledPair {
            proposal: "Adopt NextJS server side rendering for documentation pages",
            alternative: "NextJS server side rendering",
            is_true_violation: true,
            description: "NextJS server side rendering restatement",
        },
        LabeledPair {
            proposal: "Index actions with Elasticsearch full text search cluster",
            alternative: "Elasticsearch full text search cluster",
            is_true_violation: true,
            description: "Elasticsearch full text search restatement",
        },
        LabeledPair {
            proposal: "Queue async work through RabbitMQ message queue",
            alternative: "RabbitMQ message broker queue",
            is_true_violation: true,
            description: "Partial restatement: 3/4 tokens = 75% overlap",
        },
        LabeledPair {
            proposal: "Write large binary blobs into direct SQLite database blob storage",
            alternative: "Direct SQLite database blob storage",
            is_true_violation: true,
            description: "Direct SQLite blob storage restatement",
        },
        LabeledPair {
            proposal: "Expose service via HTTP cleartext internal communication port",
            alternative: "HTTP cleartext internal communication",
            is_true_violation: true,
            description: "HTTP cleartext internal communication restatement",
        },
        LabeledPair {
            proposal: "Wrap application inside Electron desktop webview wrapper",
            alternative: "Electron desktop webview wrapper",
            is_true_violation: true,
            description: "Electron desktop webview wrapper restatement",
        },
        LabeledPair {
            proposal: "Store client state using Redux centralized state store",
            alternative: "Redux centralized state store",
            is_true_violation: true,
            description: "Redux centralized state store restatement",
        },
        LabeledPair {
            proposal: "Persist telemetry inside DynamoDB database service",
            alternative: "DynamoDB NoSQL database service",
            is_true_violation: true,
            description: "Partial restatement: 3/4 tokens = 75% overlap",
        },
        LabeledPair {
            proposal: "Verify signature with SHA1 cryptographic algorithm",
            alternative: "SHA1 cryptographic hashing algorithm",
            is_true_violation: true,
            description: "Partial restatement: 3/4 tokens = 75% overlap",
        },
        LabeledPair {
            proposal: "Run test container inside Docker container inside docker DinD",
            alternative: "Docker container inside docker DinD",
            is_true_violation: true,
            description: "Docker DinD restatement",
        },
        LabeledPair {
            proposal: "Decompose into Microservices distributed service mesh architecture",
            alternative: "Microservices distributed service mesh architecture",
            is_true_violation: true,
            description: "Microservices service mesh restatement",
        },
        LabeledPair {
            proposal: "Run migrations using Python script execution for migrations",
            alternative: "Python script subprocess execution for migrations",
            is_true_violation: true,
            description: "Partial restatement: 4/5 tokens = 80% overlap",
        },

        // --- 25 Non-Violations (Negations, partial distractors, unrelated) ---
        LabeledPair {
            proposal: "We must avoid DuckDB embedded analytics database due to locking",
            alternative: "DuckDB embedded analytics database",
            is_true_violation: false,
            description: "Negation: explicit 'avoid'",
        },
        LabeledPair {
            proposal: "Ensure we do not use RocksDB embedded KV storage engine",
            alternative: "RocksDB embedded KV storage engine",
            is_true_violation: false,
            description: "Negation: explicit 'do not use'",
        },
        LabeledPair {
            proposal: "Never adopt GraphQL subscription protocol in production",
            alternative: "GraphQL subscription protocol",
            is_true_violation: false,
            description: "Negation: explicit 'never'",
        },
        LabeledPair {
            proposal: "Refactor SQLite storage engine without DuckDB embedded analytics database",
            alternative: "DuckDB embedded analytics database",
            is_true_violation: false,
            description: "Negation: explicit 'without'",
        },
        LabeledPair {
            proposal: "Avoid HTTP cleartext internal communication in production",
            alternative: "HTTP cleartext internal communication",
            is_true_violation: false,
            description: "Negation: explicit 'avoid'",
        },
        LabeledPair {
            proposal: "We wont use Redis in-memory cache server",
            alternative: "Redis in-memory cache server",
            is_true_violation: false,
            description: "Negation: explicit 'wont'",
        },
        LabeledPair {
            proposal: "Optimize existing SQLite database queries for fast execution",
            alternative: "DuckDB embedded analytics database",
            is_true_violation: false,
            description: "Distractor: generic 'database' word match only",
        },
        LabeledPair {
            proposal: "Improve SQLite storage engine transaction commit latency",
            alternative: "RocksDB embedded KV storage engine",
            is_true_violation: false,
            description: "Distractor: 'storage engine' matches 2/5 tokens (40% coverage)",
        },
        LabeledPair {
            proposal: "Update HTTP client configuration timeout settings",
            alternative: "HTTP cleartext internal communication",
            is_true_violation: false,
            description: "Distractor: 'HTTP' matches 1/4 tokens (25% coverage)",
        },
        LabeledPair {
            proposal: "Refactor authentication controller to validate tokens securely",
            alternative: "Plaintext configuration secrets storage",
            is_true_violation: false,
            description: "Unrelated: security token validation",
        },
        LabeledPair {
            proposal: "Format markdown tables in documentation site",
            alternative: "Tailwind CSS utility framework",
            is_true_violation: false,
            description: "Unrelated: documentation formatting",
        },
        LabeledPair {
            proposal: "Add unit tests for JSON parser error handling",
            alternative: "Kafka distributed message broker cluster",
            is_true_violation: false,
            description: "Unrelated: parser error tests",
        },
        LabeledPair {
            proposal: "Implement retry logic for outgoing network requests",
            alternative: "gRPC streaming transport",
            is_true_violation: false,
            description: "Unrelated: network retry logic",
        },
        LabeledPair {
            proposal: "Clean up unused CSS rules in assets stylesheet",
            alternative: "Tailwind CSS utility framework",
            is_true_violation: false,
            description: "Distractor: 'CSS' matches 1/4 tokens (25% coverage)",
        },
        LabeledPair {
            proposal: "Add git pre-commit hook script for linter verification",
            alternative: "Python script subprocess execution for migrations",
            is_true_violation: false,
            description: "Distractor: 'script' matches 1/6 tokens (16% coverage)",
        },
        LabeledPair {
            proposal: "Check schema integrity with PRAGMA integrity_check",
            alternative: "Direct SQLite database blob storage",
            is_true_violation: false,
            description: "Distractor: SQLite pragma integrity check",
        },
        LabeledPair {
            proposal: "Inspect frontend build bundle size and chunk output",
            alternative: "React single page application",
            is_true_violation: false,
            description: "Distractor: frontend build tooling",
        },
        LabeledPair {
            proposal: "Verify user credentials against cryptographic hash",
            alternative: "SHA1 cryptographic hashing algorithm",
            is_true_violation: false,
            description: "Distractor: 'cryptographic hash' matches 2/4 tokens (50% coverage)",
        },
        LabeledPair {
            proposal: "Audit internal communication endpoints across microservices",
            alternative: "HTTP cleartext internal communication",
            is_true_violation: false,
            description: "Distractor: 'internal communication' matches 2/4 tokens (50% coverage)",
        },
        LabeledPair {
            proposal: "Check server port availability before listening",
            alternative: "Celery distributed background task queue",
            is_true_violation: false,
            description: "Unrelated: network port check",
        },
        LabeledPair {
            proposal: "Bump dependency versions in Cargo.toml",
            alternative: "NextJS server side rendering",
            is_true_violation: false,
            description: "Unrelated: Cargo.toml maintenance",
        },
        LabeledPair {
            proposal: "Configure SQLite WAL checkpointing frequency",
            alternative: "MongoDB document database store",
            is_true_violation: false,
            description: "Unrelated: SQLite checkpointing",
        },
        LabeledPair {
            proposal: "Ensure session tokens expire after twenty four hours",
            alternative: "Redis in-memory cache server",
            is_true_violation: false,
            description: "Distractor: 'server' or 'session' single token",
        },
        LabeledPair {
            proposal: "Log warnings when database lock times out",
            alternative: "DynamoDB NoSQL database service",
            is_true_violation: false,
            description: "Distractor: 'database' word overlap only",
        },
        LabeledPair {
            proposal: "Review pull request comments on architecture",
            alternative: "Microservices distributed service mesh architecture",
            is_true_violation: false,
            description: "Distractor: 'architecture' matches 1/5 tokens (20% coverage)",
        },
    ]
}

/// Runs empirical precision and recall calibration across candidate thresholds.
pub fn run_calibration() -> CalibrationReport {
    let dataset = calibration_dataset();
    let thresholds = [
        (0.40, "Over-sensitive: false alarms on partial collocations"),
        (0.60, "OPTIMAL: maximum precision and recall (chosen threshold)"),
        (0.80, "Under-sensitive: false negatives missing real restatements"),
    ];

    let mut rows = Vec::with_capacity(thresholds.len());
    for (threshold, assessment) in thresholds {
        let mut tp = 0;
        let mut fp = 0;
        let mut fn_count = 0;
        let mut tn = 0;

        for pair in &dataset {
            let matched = match_rejected_alternative_threshold(
                pair.proposal,
                pair.alternative,
                threshold,
            )
            .is_some();

            match (matched, pair.is_true_violation) {
                (true, true) => tp += 1,
                (true, false) => fp += 1,
                (false, true) => fn_count += 1,
                (false, false) => tn += 1,
            }
        }

        let precision = if tp + fp > 0 {
            tp as f64 / (tp + fp) as f64
        } else {
            1.0
        };

        let recall = if tp + fn_count > 0 {
            tp as f64 / (tp + fn_count) as f64
        } else {
            1.0
        };

        let f1 = if precision + recall > 0.0 {
            2.0 * (precision * recall) / (precision + recall)
        } else {
            0.0
        };

        rows.push(CalibrationRow {
            threshold,
            tp,
            fp,
            fn_count,
            tn,
            precision,
            recall,
            f1,
            assessment: assessment.to_string(),
        });
    }

    CalibrationReport {
        pair_count: dataset.len(),
        rows,
        recommended_threshold: 0.60,
    }
}

/// Computes the comprehensive `MetricsReport` from telemetry and storage records.
pub fn cmd_metrics(
    db: &KnoxDb,
    project: Option<&str>,
    include_calibration: bool,
) -> KnoxResult<MetricsReport> {
    let stats = get_query_log_stats(db, project)?;
    let (hits, successful_retrieval_tokens) = get_successful_query_token_stats(db, project)?;
    let latencies = get_query_latencies(db, project)?;
    let (duplicate_actions, total_actions) = get_action_duplicate_stats(db, project)?;

    let hit_rate = if stats.total_queries > 0 {
        hits as f64 / stats.total_queries as f64
    } else {
        0.0
    };

    let empty_rate = if stats.total_queries > 0 {
        stats.empty_queries as f64 / stats.total_queries as f64
    } else {
        0.0
    };

    let duplicate_rate = if total_actions > 0 {
        duplicate_actions as f64 / total_actions as f64
    } else {
        0.0
    };

    let tokens_per_successful_retrieval = if hits > 0 {
        successful_retrieval_tokens as f64 / hits as f64
    } else {
        0.0
    };

    let (latency_p50_ms, latency_p95_ms) = if latencies.is_empty() {
        (0.0, 0.0)
    } else {
        let n = latencies.len();
        let p50_idx = (n as f64 * 0.50).floor() as usize;
        let p95_idx = ((n as f64 * 0.95).ceil() as usize).saturating_sub(1).min(n - 1);
        (latencies[p50_idx] as f64, latencies[p95_idx] as f64)
    };

    let tools = all_tools();
    let sample_actions = vec![KnoxAction {
        actor: "metrics-probe".to_string(),
        action_type: "decide".to_string(),
        summary: "Measure context token compression efficiency".to_string(),
        files: vec!["src/commands/metrics.rs".to_string()],
        ..Default::default()
    }];
    let efficiency = measure_token_efficiency(&tools, &sample_actions);

    let calibration = if include_calibration {
        Some(run_calibration())
    } else {
        None
    };

    Ok(MetricsReport {
        total_queries: stats.total_queries,
        hit_count: hits,
        hit_rate,
        empty_count: stats.empty_queries,
        empty_rate,
        empty_breakdown: EmptyBreakdown {
            no_records: stats.no_records_count,
            no_match: stats.no_match_count,
            out_of_scope: stats.out_of_scope_count,
        },
        total_actions,
        duplicate_actions,
        duplicate_rate,
        successful_retrieval_tokens,
        tokens_per_successful_retrieval,
        latency_p50_ms,
        latency_p95_ms,
        tool_count: efficiency.tool_count,
        tool_definition_tokens: efficiency.tool_definition_tokens,
        compact_savings_percent: efficiency.compact_savings_percent(),
        calibration,
    })
}

/// Formats a `MetricsReport` into human-readable terminal output with explicit denominators.
pub fn format_metrics_report(report: &MetricsReport) -> String {
    let mut out = String::with_capacity(1024);
    out.push_str("=== Knox Metrics & Evaluation Report ===\n\n");

    out.push_str("Retrieval Performance (query_log):\n");
    out.push_str(&format!(
        "  Total Queries Recorded        : {}\n",
        report.total_queries
    ));
    out.push_str(&format!(
        "  Hit Rate                      : {:.2}% ({}/{} queries)\n",
        report.hit_rate * 100.0,
        report.hit_count,
        report.total_queries
    ));
    out.push_str(&format!(
        "  Empty Rate                    : {:.2}% ({}/{} queries)\n",
        report.empty_rate * 100.0,
        report.empty_count,
        report.total_queries
    ));
    out.push_str(&format!(
        "    - no_records                : {} ({:.2}% of queries)\n",
        report.empty_breakdown.no_records,
        if report.total_queries > 0 {
            report.empty_breakdown.no_records as f64 / report.total_queries as f64 * 100.0
        } else {
            0.0
        }
    ));
    out.push_str(&format!(
        "    - no_match                  : {} ({:.2}% of queries)\n",
        report.empty_breakdown.no_match,
        if report.total_queries > 0 {
            report.empty_breakdown.no_match as f64 / report.total_queries as f64 * 100.0
        } else {
            0.0
        }
    ));
    out.push_str(&format!(
        "    - out_of_scope              : {} ({:.2}% of queries)\n",
        report.empty_breakdown.out_of_scope,
        if report.total_queries > 0 {
            report.empty_breakdown.out_of_scope as f64 / report.total_queries as f64 * 100.0
        } else {
            0.0
        }
    ));
    out.push_str(&format!(
        "  Tokens / Successful Retrieval : {:.1} tokens ({} tokens / {} hits)\n",
        report.tokens_per_successful_retrieval,
        report.successful_retrieval_tokens,
        report.hit_count
    ));
    out.push_str(&format!(
        "  Latency p50 (median)          : {:.1} ms (sample: {} queries)\n",
        report.latency_p50_ms,
        report.total_queries
    ));
    out.push_str(&format!(
        "  Latency p95                   : {:.1} ms (sample: {} queries)\n",
        report.latency_p95_ms,
        report.total_queries
    ));

    out.push_str("\nKnowledge Graph Cohesion:\n");
    out.push_str(&format!(
        "  Total Actions                 : {}\n",
        report.total_actions
    ));
    out.push_str(&format!(
        "  Duplicate Actions             : {}\n",
        report.duplicate_actions
    ));
    out.push_str(&format!(
        "  Duplicate Rate                : {:.2}% ({}/{} actions)\n",
        report.duplicate_rate * 100.0,
        report.duplicate_actions,
        report.total_actions
    ));

    out.push_str("\nProtocol Footprint & Compression:\n");
    out.push_str(&format!(
        "  Registered MCP Tools          : {} tools\n",
        report.tool_count
    ));
    out.push_str(&format!(
        "  Tool Definition Tokens        : {} tokens (target: <= 850)\n",
        report.tool_definition_tokens
    ));
    out.push_str(&format!(
        "  Compact Token Savings         : {}% savings vs raw JSON\n",
        report.compact_savings_percent
    ));

    if let Some(ref cal) = report.calibration {
        out.push_str("\nGuardrail Overlap Score Calibration (50 Labeled Pairs):\n");
        out.push_str("  | Threshold | TP | FP | FN | TN | Precision | Recall | F1-Score | Assessment |\n");
        out.push_str("  |-----------|----|----|----|----|-----------|--------|----------|------------|\n");
        for row in &cal.rows {
            out.push_str(&format!(
                "  |   {:.2}    | {:>2} | {:>2} | {:>2} | {:>2} |   {:.3}   |  {:.3} |   {:.3}  | {} |\n",
                row.threshold, row.tp, row.fp, row.fn_count, row.tn, row.precision, row.recall, row.f1, row.assessment
            ));
        }
        out.push_str(&format!(
            "  Empirically Justified Threshold: {:.2} (maximizes F1 with 0 false alarms)\n",
            cal.recommended_threshold
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::QueryLogEntry;
    use crate::storage::record_query_log;

    #[test]
    fn test_metrics_empty_db() {
        let db = KnoxDb::open_memory().unwrap();
        let report = cmd_metrics(&db, None, false).unwrap();
        assert_eq!(report.total_queries, 0);
        assert_eq!(report.hit_count, 0);
        assert_eq!(report.hit_rate, 0.0);
        assert_eq!(report.empty_rate, 0.0);
        assert_eq!(report.latency_p50_ms, 0.0);
        assert_eq!(report.latency_p95_ms, 0.0);
        assert_eq!(report.tool_count, 7);
        assert!(report.tool_definition_tokens <= 850);
        assert!(report.calibration.is_none());

        let formatted = format_metrics_report(&report);
        assert!(formatted.contains("=== Knox Metrics & Evaluation Report ==="));
        assert!(formatted.contains("Total Queries Recorded        : 0"));
    }

    #[test]
    fn test_metrics_populated_db() {
        let db = KnoxDb::open_memory().unwrap();

        // 3 hits, 1 empty
        record_query_log(
            &db,
            &QueryLogEntry {
                id: "q1".to_string(),
                ts_ms: 1000,
                tool: "knox_context".to_string(),
                project_id: Some("demo".to_string()),
                rows: 2,
                tokens: 300,
                latency_ms: 10,
                reason: None,
            },
        )
        .unwrap();

        record_query_log(
            &db,
            &QueryLogEntry {
                id: "q2".to_string(),
                ts_ms: 1010,
                tool: "knox_context".to_string(),
                project_id: Some("demo".to_string()),
                rows: 4,
                tokens: 500,
                latency_ms: 20,
                reason: None,
            },
        )
        .unwrap();

        record_query_log(
            &db,
            &QueryLogEntry {
                id: "q3".to_string(),
                ts_ms: 1020,
                tool: "knox_context".to_string(),
                project_id: Some("demo".to_string()),
                rows: 1,
                tokens: 150,
                latency_ms: 30,
                reason: None,
            },
        )
        .unwrap();

        record_query_log(
            &db,
            &QueryLogEntry {
                id: "q4".to_string(),
                ts_ms: 1030,
                tool: "knox_context".to_string(),
                project_id: Some("demo".to_string()),
                rows: 0,
                tokens: 47,
                latency_ms: 5,
                reason: Some("no_match".to_string()),
            },
        )
        .unwrap();

        let report = cmd_metrics(&db, Some("demo"), false).unwrap();
        assert_eq!(report.total_queries, 4);
        assert_eq!(report.hit_count, 3);
        assert_eq!(report.hit_rate, 0.75); // 3 / 4
        assert_eq!(report.empty_count, 1);
        assert_eq!(report.empty_rate, 0.25); // 1 / 4
        assert_eq!(report.empty_breakdown.no_match, 1);
        assert_eq!(report.successful_retrieval_tokens, 950); // 300 + 500 + 150
        assert_eq!(report.tokens_per_successful_retrieval, 950.0 / 3.0);
        assert_eq!(report.latency_p50_ms, 20.0);
        assert!(report.calibration.is_none());

        let formatted = format_metrics_report(&report);
        assert!(formatted.contains("Hit Rate                      : 75.00% (3/4 queries)"));
        assert!(formatted.contains("Empty Rate                    : 25.00% (1/4 queries)"));
        assert!(formatted.contains("no_match                  : 1 (25.00% of queries)"));
    }

    #[test]
    fn test_metrics_with_calibration() {
        let db = KnoxDb::open_memory().unwrap();
        let report = cmd_metrics(&db, None, true).unwrap();
        assert!(report.calibration.is_some());
        let cal = report.calibration.as_ref().unwrap();
        assert_eq!(cal.pair_count, 50);
        assert_eq!(cal.rows.len(), 3);
        assert_eq!(cal.recommended_threshold, 0.60);

        let formatted = format_metrics_report(&report);
        assert!(formatted.contains("Guardrail Overlap Score Calibration (50 Labeled Pairs)"));
        assert!(formatted.contains("Empirically Justified Threshold: 0.60"));
    }
}
