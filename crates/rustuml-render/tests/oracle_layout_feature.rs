// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! ENT-001: the shipped CLI crate must not enable the oracle-layout
//! generative path. rustuml-oracle is the only crate allowed to turn it on.

#[test]
fn rustuml_crate_does_not_enable_oracle_layout() {
    let manifest = include_str!("../../rustuml/Cargo.toml");
    assert!(
        !manifest.contains("oracle-layout"),
        "rustuml CLI must not enable oracle-layout"
    );
}

#[test]
fn rustuml_oracle_enables_oracle_layout() {
    let manifest = include_str!("../../rustuml-oracle/Cargo.toml");
    assert!(
        manifest.contains("features = [\"oracle-layout\"]"),
        "rustuml-oracle must enable oracle-layout"
    );
}

#[test]
fn render_crate_declares_oracle_layout_feature() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains("oracle-layout = []"),
        "rustuml-render must declare oracle-layout"
    );
}

#[test]
fn render_svg_with_oracle_is_feature_gated() {
    let lib = include_str!("../src/lib.rs");
    let needle = "pub fn render_svg_with_oracle";
    let pos = lib
        .find(needle)
        .expect("render_svg_with_oracle must exist");
    let prefix = &lib[..pos];
    let gated = prefix
        .rmatch_indices("#[cfg(feature = \"oracle-layout\")]")
        .next()
        .is_some_and(|(i, _)| pos - i < 200);
    assert!(
        gated,
        "render_svg_with_oracle must sit behind #[cfg(feature = \"oracle-layout\")]"
    );
}
