// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Matrix-based oracle tests — systematic feature coverage.

use rustuml_oracle::matrix::{self, activity, class, sequence, state, validate};

// ---------------------------------------------------------------------------
// Sequence diagrams
// ---------------------------------------------------------------------------

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

#[test]
#[ignore]
fn sequence_breadth_matrix() {
    let cases = sequence::breadth_cases();
    eprintln!("Breadth matrix: {} cases", cases.len());
    let report = matrix::coverage_report(&cases);
    eprintln!("{report}");
    let results = validate::validate_all(&cases);
    let summary = validate::format_summary(&results);
    let failures: Vec<_> = results.iter().filter(|r| !r.passed()).collect();
    assert!(failures.is_empty(), "breadth matrix: {summary}");
}

// ---------------------------------------------------------------------------
// Class diagrams
// ---------------------------------------------------------------------------

#[test]
fn class_edge_cases() {
    let cases = class::edge_cases();
    let results = validate::validate_all(&cases);
    let summary = validate::format_summary(&results);
    let failures: Vec<_> = results.iter().filter(|r| !r.passed()).collect();
    assert!(failures.is_empty(), "class edge cases: {summary}");
}

#[test]
fn class_quick_matrix() {
    let cases = class::quick_cases();
    assert!(
        cases.len() >= 20,
        "class quick matrix should produce at least 20 cases, got {}",
        cases.len()
    );
    let results = validate::validate_all(&cases);
    let summary = validate::format_summary(&results);
    let failures: Vec<_> = results.iter().filter(|r| !r.passed()).collect();
    assert!(failures.is_empty(), "class quick matrix: {summary}");
}

// ---------------------------------------------------------------------------
// State diagrams
// ---------------------------------------------------------------------------

#[test]
fn state_edge_cases() {
    let cases = state::edge_cases();
    let results = validate::validate_all(&cases);
    let summary = validate::format_summary(&results);
    let failures: Vec<_> = results.iter().filter(|r| !r.passed()).collect();
    assert!(failures.is_empty(), "state edge cases: {summary}");
}

// ---------------------------------------------------------------------------
// Activity diagrams
// ---------------------------------------------------------------------------

#[test]
fn activity_edge_cases() {
    let cases = activity::edge_cases();
    let results = validate::validate_all(&cases);
    let summary = validate::format_summary(&results);
    let failures: Vec<_> = results.iter().filter(|r| !r.passed()).collect();
    assert!(failures.is_empty(), "activity edge cases: {summary}");
}

// ---------------------------------------------------------------------------
// Cross-cutting
// ---------------------------------------------------------------------------

#[test]
fn all_matrix_coverage() {
    let mut all_cases = Vec::new();
    all_cases.extend(sequence::quick_cases());
    all_cases.extend(sequence::edge_cases());
    all_cases.extend(class::quick_cases());
    all_cases.extend(class::edge_cases());
    all_cases.extend(state::edge_cases());
    all_cases.extend(activity::edge_cases());

    let report = matrix::coverage_report(&all_cases);

    assert!(
        report.total_cases >= 100,
        "should have at least 100 total matrix cases, got {}",
        report.total_cases
    );
    assert!(report.tag_counts.contains_key("arrow"));
    assert!(report.tag_counts.contains_key("entity"));
    assert!(report.tag_counts.contains_key("relationship"));
    assert!(report.tag_counts.contains_key("state"));
    assert!(report.tag_counts.contains_key("activity"));
    assert!(report.tag_counts.contains_key("edge"));
}
