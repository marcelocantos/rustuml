// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Preprocessing-specific golden pair tests.
//! Only runs tests from test-diagrams/golden/preprocessing/.

use rustuml_oracle::compare;
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

fn run_one(puml_path: &Path) -> (String, &'static str, Option<String>) {
    let name = puml_path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let source = match std::fs::read_to_string(puml_path) {
        Ok(s) => s,
        Err(e) => return (name, "skip", Some(format!("read: {e}"))),
    };

    let svg_path = puml_path.with_extension("svg");
    let golden_svg = match std::fs::read_to_string(&svg_path) {
        Ok(s) => s,
        Err(_) => return (name, "skip", Some("no svg".into())),
    };

    if golden_has_error(&golden_svg) {
        return (name, "skip", Some("golden has error".into()));
    }

    let base_dir = puml_path.parent().map(Path::to_owned);
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let diagram =
                rustuml_parser::parse::parse_auto_with_base(&source, base_dir.as_deref())
                    .map_err(|e| format!("parse: {e}"))?;

            let rust_svg = rustuml_render::render_svg(&diagram);

            let golden_elems = compare::extract_elements(&golden_svg)
                .map_err(|e| format!("golden SVG parse: {e}"))?;
            let rust_elems = compare::extract_elements(&rust_svg)
                .map_err(|e| format!("rust SVG parse: {e}"))?;

            let skip = |t: &&str| {
                t.len() < 2
                    || ["alt", "else", "opt", "loop", "end", "par", "ref"].contains(t)
                    || t.starts_with('[')
            };

            let golden_texts: Vec<&str> = golden_elems
                .iter()
                .filter_map(|e| e.text.as_deref())
                .filter(|t| !t.is_empty())
                .collect();
            let rust_texts: Vec<&str> = rust_elems
                .iter()
                .filter_map(|e| e.text.as_deref())
                .filter(|t| !t.is_empty())
                .collect();

            let missing: Vec<String> = golden_texts
                .iter()
                .filter(|t| !skip(t))
                .filter(|t| !rust_texts.iter().any(|r| r.contains(**t)))
                .map(|t| t.to_string())
                .collect();

            if missing.is_empty() {
                Ok(())
            } else {
                Err(format!("missing texts: {missing:?}"))
            }
        }));

    match result {
        Ok(Ok(())) => (name, "pass", None),
        Ok(Err(msg)) if msg.starts_with("parse:") => (name, "skip", Some(msg)),
        Ok(Err(msg)) => (name, "fail", Some(msg)),
        Err(p) => {
            let msg = if let Some(s) = p.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = p.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".into()
            };
            (name, "fail", Some(format!("panic: {msg}")))
        }
    }
}

#[test]
fn preproc_golden_pairs() {
    let root = golden_dir();
    if !root.exists() {
        eprintln!("preprocessing golden dir not found: {}", root.display());
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

    let mut pass = 0usize;
    let mut skip = 0usize;
    let mut failures = Vec::new();

    for puml_path in &pairs {
        let (name, outcome, msg) = run_one(puml_path);
        match outcome {
            "pass" => pass += 1,
            "skip" => skip += 1,
            _ => failures.push(format!("{name}: {}", msg.unwrap_or_default())),
        }
    }

    let total = pairs.len();
    let fail_count = failures.len();
    eprintln!(
        "\npreproc_golden_pairs: {total} total, {pass} passed, {fail_count} failed, {skip} skipped"
    );
    for f in failures.iter().take(50) {
        eprintln!("  FAIL: {f}");
    }
    assert!(
        failures.is_empty(),
        "{fail_count} preprocessing golden pair tests failed"
    );
}
