// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Reusable feature sets shared across diagram types.

use super::{FeatureSet, FeatureVariant};

/// A simple feature set built from a static list of variants.
pub struct StaticFeatureSet {
    pub dimension: &'static str,
    pub variants: Vec<FeatureVariant>,
}

impl FeatureSet for StaticFeatureSet {
    fn variants(&self) -> Vec<FeatureVariant> {
        self.variants.clone()
    }

    fn dimension(&self) -> &'static str {
        self.dimension
    }
}

/// Helper to build a feature variant concisely.
pub fn variant(
    name: &str,
    syntax: &str,
    tags: &[&'static str],
    expected: &[&str],
) -> FeatureVariant {
    FeatureVariant {
        name: name.to_string(),
        syntax: syntax.to_string(),
        tags: tags.to_vec(),
        expected_texts: expected.iter().map(|s| s.to_string()).collect(),
    }
}
