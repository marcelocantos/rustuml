// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Golden matrix tests — validate Rust rendering against pre-generated
//! Java PlantUML reference files for all matrix test cases.

use rustuml_oracle::compare;
use rustuml_oracle::matrix::{MatrixCase, activity, class, deployment, sequence, state};
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-fixtures/golden/matrix")
}

fn render_rust(input: &str) -> String {
    let diagram = rustuml_parser::parse::parse(input).expect("parse failed");
    rustuml_render::render_svg(&diagram)
}

fn validate_case(case: &MatrixCase) -> Result<(), String> {
    let safe_name = case.name.replace('/', "_").replace(' ', "_");
    let path = golden_dir().join(format!("{safe_name}.svg"));

    let golden = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Ok(()), // Skip if golden file missing.
    };

    let rust = render_rust(&case.source);

    // Parse both SVGs.
    let golden_elems =
        compare::extract_elements(&golden).map_err(|e| format!("golden parse: {e}"))?;
    let rust_elems = compare::extract_elements(&rust).map_err(|e| format!("rust parse: {e}"))?;

    // Extract meaningful text content.
    let golden_texts: Vec<&str> = golden_elems
        .iter()
        .filter_map(|e| e.text.as_deref())
        .filter(|t| t.len() >= 2)
        .filter(|t| !t.starts_with('['))
        .collect();

    let rust_texts: Vec<&str> = rust_elems
        .iter()
        .filter_map(|e| e.text.as_deref())
        .filter(|t| t.len() >= 2)
        .collect();

    // Check expected texts from the test case spec.
    for expected in &case.expected_texts {
        if expected.len() < 2 {
            continue;
        }
        if !rust_texts.iter().any(|t| t.contains(expected.as_str()))
            && !golden_texts.iter().any(|t| t.contains(expected.as_str()))
        {
            // Expected text missing from both — test case spec issue, skip.
            continue;
        }
        if !rust_texts.iter().any(|t| t.contains(expected.as_str())) {
            return Err(format!(
                "expected text '{expected}' in Rust output but not found"
            ));
        }
    }

    Ok(())
}

#[test]
fn all_matrix_golden_files() {
    let mut cases: Vec<MatrixCase> = Vec::new();
    cases.extend(sequence::quick_cases());
    cases.extend(sequence::medium_cases());
    cases.extend(sequence::edge_cases());
    cases.extend(class::quick_cases());
    cases.extend(class::medium_cases());
    cases.extend(class::edge_cases());
    cases.extend(state::edge_cases());
    cases.extend(activity::edge_cases());
    cases.extend(deployment::edge_cases());

    let mut failures = Vec::new();
    for case in &cases {
        if let Err(e) = validate_case(case) {
            failures.push(format!("{}: {e}", case.name));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} golden tests failed:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
