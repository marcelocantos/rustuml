// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Matrix-based oracle tests — systematic feature coverage.

use rustuml_oracle::matrix::{self, sequence, validate};

#[test]
fn sequence_edge_cases() {
    let cases = sequence::edge_cases();
    let results = validate::validate_all(&cases);
    let summary = validate::format_summary(&results);
    let failures: Vec<_> = results.iter().filter(|r| !r.passed()).collect();
    assert!(failures.is_empty(), "edge cases: {summary}");
}

#[test]
fn sequence_quick_matrix() {
    let cases = sequence::quick_cases();
    assert!(
        cases.len() >= 30,
        "quick matrix should produce at least 30 cases, got {}",
        cases.len()
    );

    let results = validate::validate_all(&cases);
    let summary = validate::format_summary(&results);
    let failures: Vec<_> = results.iter().filter(|r| !r.passed()).collect();
    assert!(failures.is_empty(), "quick matrix: {summary}");
}

/// The full breadth matrix — every arrow × participant type × label ×
/// decoration × grouping × activation combination.
///
/// This is expensive (~7,680 combinations × 2 renders each = ~15,360
/// oracle calls). Run with `cargo test matrix_breadth -- --ignored`
/// for a thorough check, or let CI handle it.
#[test]
#[ignore]
fn sequence_breadth_matrix() {
    let cases = sequence::breadth_cases();
    eprintln!("Breadth matrix: {} cases", cases.len());

    // Print coverage report.
    let report = matrix::coverage_report(&cases);
    eprintln!("{report}");

    let results = validate::validate_all(&cases);
    let summary = validate::format_summary(&results);
    let failures: Vec<_> = results.iter().filter(|r| !r.passed()).collect();
    assert!(failures.is_empty(), "breadth matrix: {summary}");
}

#[test]
fn coverage_report_includes_all_dimensions() {
    let cases = sequence::quick_cases();
    let report = matrix::coverage_report(&cases);

    // Verify key feature dimensions have coverage.
    assert!(
        report.tag_counts.contains_key("arrow"),
        "should cover arrow styles"
    );
    assert!(
        report.tag_counts.contains_key("label"),
        "should cover label variants"
    );
    assert!(report.total_cases > 0, "should have test cases");
}
