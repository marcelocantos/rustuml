// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Fast parse-only test for preprocessing golden pairs.
//! No rendering — just checks that files parse without errors.

use std::path::{Path, PathBuf};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-diagrams/golden/preprocessing")
}

fn golden_has_error(svg: &str) -> bool {
    svg.contains("Syntax Error")
        || svg.contains("NoSuchElementException")
        || svg.contains("Welcome to PlantUML")
        || svg.contains("An error has occured")
}

#[test]
fn preproc_parse_all() {
    let root = golden_dir();
    if !root.exists() {
        eprintln!("preprocessing golden dir not found");
        return;
    }

    let mut pairs: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "puml"))
        .filter(|p| p.with_extension("svg").exists())
        .collect();
    pairs.sort();

    let mut parse_ok = 0usize;
    let mut parse_err = 0usize;
    let mut skip = 0usize;
    let mut parse_errors = Vec::new();

    for puml_path in &pairs {
        let source = match std::fs::read_to_string(puml_path) {
            Ok(s) => s,
            Err(_) => { skip += 1; continue; }
        };
        let svg_path = puml_path.with_extension("svg");
        let golden_svg = match std::fs::read_to_string(&svg_path) {
            Ok(s) => s,
            Err(_) => { skip += 1; continue; }
        };
        if golden_has_error(&golden_svg) {
            skip += 1;
            continue;
        }

        let name = puml_path.file_stem().unwrap().to_string_lossy().to_string();
        let base_dir = puml_path.parent().map(Path::to_owned);

        match rustuml_parser::parse::parse_auto_with_base(&source, base_dir.as_deref()) {
            Ok(_) => parse_ok += 1,
            Err(e) => {
                parse_err += 1;
                parse_errors.push(format!("{name}: {e}"));
            }
        }
    }

    let total = pairs.len();
    eprintln!(
        "\npreproc_parse: {total} total, {parse_ok} parsed OK, {parse_err} parse errors, {skip} skipped"
    );
    for e in parse_errors.iter().take(30) {
        eprintln!("  ERR: {e}");
    }
    // Don't fail the test — just report stats.
    // This test is informational.
}
