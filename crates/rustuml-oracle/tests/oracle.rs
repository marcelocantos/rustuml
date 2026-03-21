// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use rustuml_oracle::compare::{self, extract_elements};
use rustuml_oracle::generator;
use rustuml_oracle::runner;

// ---------------------------------------------------------------------------
// Sequence diagram tests
// ---------------------------------------------------------------------------

#[test]
fn simple_sequence_produces_valid_svg() {
    let input = generator::simple_sequence();
    let svg = runner::render_svg(&input).expect("failed to render");

    assert!(
        svg.starts_with("<svg"),
        "output should be SVG: {}",
        &svg[..100.min(svg.len())]
    );

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    assert!(!elements.is_empty(), "SVG should have elements");

    let classes: Vec<&str> = elements
        .iter()
        .flat_map(|e| {
            e.attrs
                .iter()
                .filter(|(k, _)| k == "class")
                .map(|(_, v)| v.as_str())
        })
        .collect();

    assert!(
        classes.iter().any(|c| c.contains("participant")),
        "should have participant elements, got classes: {classes:?}"
    );
    assert!(
        classes.iter().any(|c| c.contains("message")),
        "should have message elements, got classes: {classes:?}"
    );
}

#[test]
fn simple_sequence_preproc_roundtrips() {
    let input = generator::simple_sequence();
    let jar = runner::find_jar().expect("JAR not found");
    let preproc = runner::run_preproc(&jar, &input).expect("failed to run preproc");

    assert!(
        preproc.contains("Alice -> Bob : hello"),
        "preproc should contain the original source: {preproc}"
    );
}

#[test]
fn identical_svg_outputs_compare_equal() {
    let input = generator::simple_sequence();
    let svg1 = runner::render_svg(&input).expect("run 1 failed");
    let svg2 = runner::render_svg(&input).expect("run 2 failed");

    let result = compare::compare_svg(&svg1, &svg2).expect("comparison failed");
    assert!(result.is_match(), "identical runs should match: {result}");
}

#[test]
fn different_diagrams_compare_different() {
    let svg1 = runner::render_svg(&generator::simple_sequence()).expect("run 1 failed");
    let svg2 = runner::render_svg(&generator::multi_message_sequence(3)).expect("run 2 failed");

    let result = compare::compare_svg(&svg1, &svg2).expect("comparison failed");
    assert!(
        !result.is_match(),
        "different diagrams should not match structurally"
    );
}

#[test]
fn multi_message_sequence_has_correct_message_count() {
    let n = 5;
    let input = generator::multi_message_sequence(n);
    let svg = runner::render_svg(&input).expect("failed to render");

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let message_count = elements
        .iter()
        .filter(|e| e.attrs.iter().any(|(k, v)| k == "class" && v == "message"))
        .count();

    assert_eq!(
        message_count, n,
        "expected {n} message elements, got {message_count}"
    );
}

#[test]
fn sequence_with_notes_has_note_text() {
    let input = generator::sequence_with_features(&generator::SequenceOptions {
        notes: true,
        ..Default::default()
    });
    let svg = runner::render_svg(&input).expect("failed to render");

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let has_note_text = elements
        .iter()
        .any(|e| e.text.as_deref() == Some("Processing..."));

    assert!(
        has_note_text,
        "diagram with notes should contain note text 'Processing...'"
    );
}

#[test]
fn sequence_with_groups_has_group_label() {
    let input = generator::sequence_with_features(&generator::SequenceOptions {
        groups: true,
        reply_arrows: true,
        ..Default::default()
    });
    let svg = runner::render_svg(&input).expect("failed to render");

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let has_group_label = elements
        .iter()
        .any(|e| e.text.as_deref() == Some("Transaction"));

    assert!(
        has_group_label,
        "diagram with groups should contain group label 'Transaction'"
    );
}

// ---------------------------------------------------------------------------
// Class diagram tests
// ---------------------------------------------------------------------------

#[test]
fn class_diagram_produces_valid_svg() {
    let svg =
        runner::render_svg(&generator::class_diagram()).expect("failed to render class diagram");

    assert!(
        svg.contains("data-diagram-type"),
        "should have diagram type"
    );

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let texts: Vec<&str> = elements.iter().filter_map(|e| e.text.as_deref()).collect();

    assert!(
        texts.iter().any(|t| *t == "Animal"),
        "should contain 'Animal' text, got: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| *t == "Dog"),
        "should contain 'Dog' text, got: {texts:?}"
    );
}

#[test]
fn class_diagram_with_relationships_produces_valid_svg() {
    let svg = runner::render_svg(&generator::class_diagram_with_relationships())
        .expect("failed to render");

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let texts: Vec<&str> = elements.iter().filter_map(|e| e.text.as_deref()).collect();

    for expected in &["Drawable", "Shape", "Circle", "Canvas"] {
        assert!(
            texts.iter().any(|t| t == expected),
            "should contain '{expected}' text, got: {texts:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// State diagram tests
// ---------------------------------------------------------------------------

#[test]
fn state_diagram_produces_valid_svg() {
    let svg =
        runner::render_svg(&generator::state_diagram()).expect("failed to render state diagram");

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let texts: Vec<&str> = elements.iter().filter_map(|e| e.text.as_deref()).collect();

    assert!(
        texts.iter().any(|t| *t == "Active"),
        "should contain 'Active' state, got: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| *t == "Inactive"),
        "should contain 'Inactive' state, got: {texts:?}"
    );
}

#[test]
fn state_diagram_nested_produces_valid_svg() {
    let svg = runner::render_svg(&generator::state_diagram_nested())
        .expect("failed to render nested state diagram");

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let texts: Vec<&str> = elements.iter().filter_map(|e| e.text.as_deref()).collect();

    assert!(
        texts.iter().any(|t| *t == "Running"),
        "should contain 'Running' state, got: {texts:?}"
    );
}

// ---------------------------------------------------------------------------
// Activity diagram tests
// ---------------------------------------------------------------------------

#[test]
fn activity_diagram_produces_valid_svg() {
    let svg = runner::render_svg(&generator::activity_diagram())
        .expect("failed to render activity diagram");

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let texts: Vec<&str> = elements.iter().filter_map(|e| e.text.as_deref()).collect();

    assert!(
        texts.iter().any(|t| t.contains("Step 1")),
        "should contain 'Step 1', got: {texts:?}"
    );
}

#[test]
fn activity_diagram_with_fork_produces_valid_svg() {
    let svg = runner::render_svg(&generator::activity_diagram_with_fork())
        .expect("failed to render fork activity diagram");

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let texts: Vec<&str> = elements.iter().filter_map(|e| e.text.as_deref()).collect();

    for expected in &["Initialize", "Task A", "Task B", "Finalize"] {
        assert!(
            texts.iter().any(|t| t == expected),
            "should contain '{expected}', got: {texts:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Component diagram tests
// ---------------------------------------------------------------------------

#[test]
fn component_diagram_produces_valid_svg() {
    let svg = runner::render_svg(&generator::component_diagram())
        .expect("failed to render component diagram");

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let texts: Vec<&str> = elements.iter().filter_map(|e| e.text.as_deref()).collect();

    for expected in &["Web Server", "Database", "Cache"] {
        assert!(
            texts.iter().any(|t| t == expected),
            "should contain '{expected}', got: {texts:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Use case diagram tests
// ---------------------------------------------------------------------------

#[test]
fn use_case_diagram_produces_valid_svg() {
    let svg = runner::render_svg(&generator::use_case_diagram())
        .expect("failed to render use case diagram");

    let elements = extract_elements(&svg).expect("failed to parse SVG");
    let texts: Vec<&str> = elements.iter().filter_map(|e| e.text.as_deref()).collect();

    for expected in &["User", "Admin", "Login", "View Dashboard", "Manage Users"] {
        assert!(
            texts.iter().any(|t| t == expected),
            "should contain '{expected}', got: {texts:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Cross-diagram structural comparison tests
// ---------------------------------------------------------------------------

#[test]
fn same_diagram_type_different_content_detects_differences() {
    let svg1 = runner::render_svg(&generator::class_diagram()).expect("render 1 failed");
    let svg2 = runner::render_svg(&generator::class_diagram_with_relationships())
        .expect("render 2 failed");

    let result = compare::compare_svg(&svg1, &svg2).expect("comparison failed");
    assert!(
        !result.is_match(),
        "different class diagrams should not match"
    );
}

#[test]
fn identical_class_diagrams_compare_equal() {
    let input = generator::class_diagram();
    let svg1 = runner::render_svg(&input).expect("run 1 failed");
    let svg2 = runner::render_svg(&input).expect("run 2 failed");

    let result = compare::compare_svg(&svg1, &svg2).expect("comparison failed");
    assert!(
        result.is_match(),
        "identical class diagrams should match: {result}"
    );
}

// ---------------------------------------------------------------------------
// Combinatorial test — all generated cases produce valid, self-consistent SVG
// ---------------------------------------------------------------------------

#[test]
fn all_generated_cases_produce_valid_svg() {
    let cases = generator::all_cases(10);
    let mut failures = Vec::new();

    for case in &cases {
        // Each case must render to valid SVG.
        let svg = match runner::render_svg(&case.source) {
            Ok(svg) => svg,
            Err(e) => {
                failures.push(format!("{}: render failed: {e}", case.name));
                continue;
            }
        };

        if !svg.starts_with("<svg") {
            failures.push(format!("{}: output is not SVG", case.name));
            continue;
        }

        // Must parse as valid XML.
        if let Err(e) = extract_elements(&svg) {
            failures.push(format!("{}: SVG parse failed: {e}", case.name));
            continue;
        }

        // Rendering the same input twice must produce structurally identical SVG.
        let svg2 = match runner::render_svg(&case.source) {
            Ok(svg) => svg,
            Err(e) => {
                failures.push(format!("{}: second render failed: {e}", case.name));
                continue;
            }
        };

        match compare::compare_svg(&svg, &svg2) {
            Ok(result) if !result.is_match() => {
                failures.push(format!(
                    "{}: identical inputs produced different SVG: {result}",
                    case.name
                ));
            }
            Err(e) => {
                failures.push(format!("{}: comparison failed: {e}", case.name));
            }
            _ => {}
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} cases failed:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
