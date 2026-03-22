// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Validation tiers for matrix test cases.

use crate::compare;
use crate::runner;

use super::MatrixCase;

/// Result of validating a single test case.
#[derive(Debug)]
pub struct ValidationResult {
    pub case_name: String,
    pub renders: bool,
    pub valid_svg: bool,
    pub expected_content: bool,
    pub deterministic: bool,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn passed(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate a test case against the Java PlantUML oracle.
///
/// Tiers:
/// 1. **Renders** — PlantUML server returns SVG (not an error)
/// 2. **Valid SVG** — output parses as valid XML/SVG
/// 3. **Expected content** — SVG contains expected text elements
/// 4. **Deterministic** — rendering the same input twice produces identical structure
pub fn validate(case: &MatrixCase) -> ValidationResult {
    let mut result = ValidationResult {
        case_name: case.name.clone(),
        renders: false,
        valid_svg: false,
        expected_content: false,
        deterministic: false,
        errors: Vec::new(),
    };

    // Tier 1: Renders.
    let svg = match runner::render_svg(&case.source) {
        Ok(svg) => svg,
        Err(e) => {
            result.errors.push(format!("render failed: {e}"));
            return result;
        }
    };

    if !svg.starts_with("<svg") {
        result.errors.push("output is not SVG".into());
        return result;
    }
    result.renders = true;

    // Tier 2: Valid SVG.
    let elements = match compare::extract_elements(&svg) {
        Ok(elems) => elems,
        Err(e) => {
            result.errors.push(format!("SVG parse failed: {e}"));
            return result;
        }
    };
    result.valid_svg = true;

    // Tier 3: Expected content.
    let all_text: Vec<&str> = elements.iter().filter_map(|e| e.text.as_deref()).collect();
    let mut content_ok = true;
    for expected in &case.expected_texts {
        if !all_text.iter().any(|t| t.contains(expected.as_str())) {
            result
                .errors
                .push(format!("missing expected text: '{expected}'"));
            content_ok = false;
        }
    }
    result.expected_content = content_ok;

    // Tier 4: Deterministic.
    match runner::render_svg(&case.source) {
        Ok(svg2) => match compare::compare_svg(&svg, &svg2) {
            Ok(cmp) => {
                if cmp.is_match() {
                    result.deterministic = true;
                } else {
                    result.errors.push(format!("non-deterministic: {cmp}"));
                }
            }
            Err(e) => {
                result.errors.push(format!("comparison failed: {e}"));
            }
        },
        Err(e) => {
            result.errors.push(format!("second render failed: {e}"));
        }
    }

    result
}

/// Validate all cases and return a summary.
pub fn validate_all(cases: &[MatrixCase]) -> Vec<ValidationResult> {
    cases.iter().map(validate).collect()
}

/// Format a validation summary for test output.
pub fn format_summary(results: &[ValidationResult]) -> String {
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed()).count();
    let failed = total - passed;

    let mut out = format!("{passed}/{total} passed");
    if failed > 0 {
        out.push_str(&format!(", {failed} failed:\n"));
        for r in results.iter().filter(|r| !r.passed()) {
            out.push_str(&format!("  {}: {}\n", r.case_name, r.errors.join("; ")));
        }
    }
    out
}
