// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Systematic test matrix — composable feature-based test generation.
//!
//! Instead of hand-writing test cases, this module lets you describe
//! diagram features as composable building blocks. The matrix engine
//! generates all valid combinations and validates them against the
//! Java PlantUML oracle.
//!
//! # Architecture
//!
//! - **Feature**: A single syntax element (e.g., "solid arrow", "actor participant")
//! - **FeatureSet**: A group of related features for one diagram aspect
//! - **DiagramTemplate**: Combines feature sets into complete diagrams
//! - **Validation**: Tiered checks (renders, valid SVG, expected content, deterministic)
//!
//! As parsing/rendering coverage grows, just add new `FeatureSet`s —
//! the matrix engine generates combinations automatically.

pub mod features;
pub mod sequence;
pub mod validate;

use std::fmt;

/// A generated test case with metadata.
#[derive(Clone)]
pub struct MatrixCase {
    /// Human-readable name for the test case.
    pub name: String,
    /// PlantUML source text.
    pub source: String,
    /// Features exercised by this case.
    pub tags: Vec<&'static str>,
    /// Expected text content that should appear in the SVG.
    pub expected_texts: Vec<String>,
}

impl fmt::Debug for MatrixCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MatrixCase")
            .field("name", &self.name)
            .field("tags", &self.tags)
            .finish()
    }
}

/// A feature set — a dimension of variation for diagram generation.
pub trait FeatureSet {
    /// Returns all variants in this feature set.
    fn variants(&self) -> Vec<FeatureVariant>;

    /// The name of this feature dimension (e.g., "arrow_style", "participant_type").
    fn dimension(&self) -> &'static str;
}

/// A single variant within a feature set.
#[derive(Clone, Debug)]
pub struct FeatureVariant {
    /// Short name (e.g., "solid", "dotted", "actor").
    pub name: String,
    /// PlantUML syntax fragment this variant contributes.
    pub syntax: String,
    /// Feature tags for coverage tracking.
    pub tags: Vec<&'static str>,
    /// Text expected in SVG output when this variant is used.
    pub expected_texts: Vec<String>,
}

/// Generates all cases for a diagram type by combining feature sets.
pub fn generate_matrix(
    diagram_type: &str,
    wrapper: fn(&[&FeatureVariant]) -> Option<MatrixCase>,
    feature_sets: &[&dyn FeatureSet],
) -> Vec<MatrixCase> {
    let mut cases = Vec::new();

    if feature_sets.is_empty() {
        return cases;
    }

    // Generate the Cartesian product of all feature sets.
    let variant_lists: Vec<Vec<FeatureVariant>> =
        feature_sets.iter().map(|fs| fs.variants()).collect();

    let mut indices = vec![0usize; variant_lists.len()];
    loop {
        let selected: Vec<&FeatureVariant> = indices
            .iter()
            .enumerate()
            .map(|(dim, &idx)| &variant_lists[dim][idx])
            .collect();

        if let Some(mut case) = wrapper(&selected) {
            // Prefix name with diagram type.
            case.name = format!("{diagram_type}/{}", case.name);
            cases.push(case);
        }

        // Advance indices (odometer-style).
        let mut dim = variant_lists.len();
        loop {
            if dim == 0 {
                return cases;
            }
            dim -= 1;
            indices[dim] += 1;
            if indices[dim] < variant_lists[dim].len() {
                break;
            }
            indices[dim] = 0;
        }
    }
}

/// Coverage report — which features have been tested.
pub fn coverage_report(cases: &[MatrixCase]) -> CoverageReport {
    let mut tag_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for case in cases {
        for tag in &case.tags {
            *tag_counts.entry(tag).or_default() += 1;
        }
    }
    CoverageReport {
        total_cases: cases.len(),
        tag_counts,
    }
}

pub struct CoverageReport {
    pub total_cases: usize,
    pub tag_counts: std::collections::HashMap<&'static str, usize>,
}

impl fmt::Display for CoverageReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Coverage: {} total cases", self.total_cases)?;
        let mut tags: Vec<_> = self.tag_counts.iter().collect();
        tags.sort_by_key(|(k, _)| *k);
        for (tag, count) in tags {
            writeln!(f, "  {tag}: {count} cases")?;
        }
        Ok(())
    }
}
