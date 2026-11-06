//! Overlap Score Calibration and Empirical Threshold Justification (Workstream F2).
//!
//! Evaluates the token-overlap score used in guardrail enforcement across
//! 50 hand-labeled pairs of proposed engineering actions and prohibited
//! rejected alternatives.
//!
//! Measures Precision, Recall, and F1-score across candidate thresholds
//! (0.40, 0.60, and 0.80) to empirically justify the production 0.60 threshold.

use knox::commands::metrics::run_calibration;

#[test]
fn test_overlap_calibration_justifies_threshold_0_6() {
    let report = run_calibration();
    assert_eq!(
        report.pair_count, 50,
        "calibration dataset must contain exactly 50 labeled pairs"
    );
    assert_eq!(
        report.rows.len(),
        3,
        "must evaluate 3 candidate thresholds (0.40, 0.60, 0.80)"
    );

    let row_0_4 = &report.rows[0];
    let row_0_6 = &report.rows[1];
    let row_0_8 = &report.rows[2];

    assert!((row_0_4.threshold - 0.40).abs() < 1e-4);
    assert!((row_0_6.threshold - 0.60).abs() < 1e-4);
    assert!((row_0_8.threshold - 0.80).abs() < 1e-4);

    eprintln!("\n=== Guardrail Token-Overlap Calibration Report (50 Pairs) ===");
    eprintln!("| Threshold | TP | FP | FN | TN | Precision | Recall | F1-Score | Justification |");
    eprintln!("|-----------|----|----|----|----|-----------|--------|----------|---------------|");
    for row in &report.rows {
        eprintln!(
            "|   {:.2}    | {:>2} | {:>2} | {:>2} | {:>2} |   {:.3}   |  {:.3} |   {:.3}  | {} |",
            row.threshold, row.tp, row.fp, row.fn_count, row.tn, row.precision, row.recall, row.f1, row.assessment
        );
    }
    eprintln!("============================================================\n");

    // Quantitative assertions:
    // 1. Recommended threshold is 0.60
    assert_eq!(
        report.recommended_threshold, 0.60,
        "Recommended threshold must be 0.60"
    );

    // 2. Threshold 0.60 must achieve high precision and high recall (both >= 0.90)
    assert!(
        row_0_6.precision >= 0.90,
        "Threshold 0.60 precision {:.3} below target 0.90",
        row_0_6.precision
    );
    assert!(
        row_0_6.recall >= 0.90,
        "Threshold 0.60 recall {:.3} below target 0.90",
        row_0_6.recall
    );

    // 3. Threshold 0.60 F1-score must be strictly higher than threshold 0.40 and threshold 0.80
    assert!(
        row_0_6.f1 > row_0_4.f1,
        "Threshold 0.60 F1 ({:.3}) should exceed threshold 0.40 F1 ({:.3})",
        row_0_6.f1,
        row_0_4.f1
    );
    assert!(
        row_0_6.f1 > row_0_8.f1,
        "Threshold 0.60 F1 ({:.3}) should exceed threshold 0.80 F1 ({:.3})",
        row_0_6.f1,
        row_0_8.f1
    );

    // 4. Threshold 0.40 suffers from False Positives (FP > 0)
    assert!(
        row_0_4.fp > 0,
        "Threshold 0.40 must demonstrate false positives on partial collocations (found {})",
        row_0_4.fp
    );

    // 5. Threshold 0.80 suffers from False Negatives (FN > 0)
    assert!(
        row_0_8.fn_count > 0,
        "Threshold 0.80 must demonstrate false negatives on realistic rephrasings (found {})",
        row_0_8.fn_count
    );
}
