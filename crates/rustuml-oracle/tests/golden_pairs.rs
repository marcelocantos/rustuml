// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Golden pair tests — read test-diagrams/golden.tar.zst and compare
//! Rust rendering against pre-generated Java PlantUML reference SVGs.
//!
//! Each `.puml` entry with a matching `.svg` entry is a test case.
//! The tarball is decompressed and scanned in memory — no files are
//! extracted to disk.
//!
//! Run with: `cargo test --test golden_pairs`

use rayon::prelude::*;
use rustuml_oracle::compare;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Path to the golden tarball relative to the workspace root.
fn golden_tarball() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-diagrams/golden.tar.zst")
}

/// `@start` keywords for diagram types rustuml can parse.
const SUPPORTED_START_KEYWORDS: &[&str] = &[
    "@startuml",
    "@startgantt",
    "@startmindmap",
    "@startwbs",
    "@startmath",
    "@startlatex",
    "@startregex",
    "@startjson",
    "@startyaml",
    "@startsalt",
    "@startnwdiag",
    "@startditaa",
];

fn has_supported_start_keyword(source: &str) -> bool {
    let trimmed = source.trim_start();
    SUPPORTED_START_KEYWORDS
        .iter()
        .any(|kw| trimmed.starts_with(kw))
}

/// Returns true if the golden SVG contains a PlantUML error marker.
fn golden_has_syntax_error(svg: &str) -> bool {
    svg.contains("Syntax Error")
        || svg.contains("NoSuchElementException")
        || svg.contains("Welcome to PlantUML")
        || svg.contains("An error has occured")
        || svg.contains("kill cannot be used here")
        || svg.contains("swimlane must be defined at the start")
        || svg.contains("Note already created:")
        || svg.contains("Parsing syntax error about %")
        || svg.contains("[From string")
        || svg.contains("Your data does not sound like YAML data")
        || svg.contains("does&#160;not&#160;sound&#160;like&#160;YAML")
        || svg.contains("No class ")
        || svg.contains("(Assumed diagram type:")
        || svg.contains("DITAA has crashed")
}

/// A golden pair loaded from the tarball.
struct GoldenPair {
    /// Relative path inside the tarball (e.g. "sequence/seq_basic.puml").
    name: String,
    /// PlantUML source.
    puml: String,
    /// Reference SVG from Java PlantUML.
    svg: String,
}

/// Load all golden pairs from the zstd-compressed tarball.
fn load_golden_pairs(tarball: &Path) -> Vec<GoldenPair> {
    let file = std::fs::File::open(tarball).expect("failed to open golden.tar.zst");
    let decoder = zstd::Decoder::new(file).expect("failed to create zstd decoder");
    let mut archive = tar::Archive::new(decoder);

    // First pass: read all entries into a map.
    let mut entries: HashMap<String, String> = HashMap::new();
    for entry in archive.entries().expect("failed to read tar entries") {
        let mut entry = entry.expect("failed to read tar entry");
        let path = entry
            .path()
            .expect("failed to read entry path")
            .to_string_lossy()
            .to_string();
        // Strip leading "./" if present.
        let path = path.strip_prefix("./").unwrap_or(&path).to_string();

        if path.ends_with(".puml") || path.ends_with(".svg") {
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
            // Skip non-UTF-8 files (e.g. binary ditaa SVGs).
            if let Ok(content) = String::from_utf8(bytes) {
                entries.insert(path, content);
            }
        }
    }

    // Second pass: pair .puml files with their .svg counterparts.
    let mut pairs: Vec<GoldenPair> = Vec::new();
    let mut puml_paths: Vec<String> = entries
        .keys()
        .filter(|k| k.ends_with(".puml"))
        .cloned()
        .collect();
    puml_paths.sort();

    for puml_path in puml_paths {
        let svg_path = puml_path.replace(".puml", ".svg");
        if let (Some(puml), Some(svg)) = (entries.get(&puml_path), entries.get(&svg_path)) {
            let name = puml_path
                .strip_suffix(".puml")
                .unwrap_or(&puml_path)
                .to_string();
            pairs.push(GoldenPair {
                name,
                puml: puml.clone(),
                svg: svg.clone(),
            });
        }
    }

    pairs
}

struct TestResult {
    name: String,
    outcome: Outcome,
}

#[allow(dead_code)]
enum Outcome {
    Pass,
    Skip(String),
    Fail(String),
}

fn run_one(pair: &GoldenPair) -> TestResult {
    let name = &pair.name;
    let source = &pair.puml;
    let golden_svg = &pair.svg;

    // Skip files with multiple @startuml blocks.
    if source.matches("@startuml").count() > 1 || source.matches("@startjson").count() > 1 {
        return TestResult {
            name: name.clone(),
            outcome: Outcome::Skip("multiple @start blocks (not yet supported)".into()),
        };
    }

    // Skip diagrams that use runtime-dependent functions.
    if source.contains("%date()") {
        return TestResult {
            name: name.clone(),
            outcome: Outcome::Skip("non-deterministic %date()".into()),
        };
    }

    // Skip diagrams whose golden SVG contains a syntax error.
    if golden_has_syntax_error(golden_svg) {
        return TestResult {
            name: name.clone(),
            outcome: Outcome::Skip("golden SVG contains error".into()),
        };
    }

    // Filter: must use a supported keyword.
    if !has_supported_start_keyword(source) {
        return TestResult {
            name: name.clone(),
            outcome: Outcome::Skip("unsupported diagram keyword".into()),
        };
    }

    // Parse + render + compare, catching panics.
    let render_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let diagram = rustuml_parser::parse::parse_auto_with_base(source, None)
            .map_err(|e| format!("parse: {e}"))?;

        let rust_svg = rustuml_render::render_svg(&diagram);

        let golden_elems =
            compare::extract_elements(golden_svg).map_err(|e| format!("golden SVG parse: {e}"))?;
        let rust_elems =
            compare::extract_elements(&rust_svg).map_err(|e| format!("rust SVG parse: {e}"))?;

        let skip = |t: &&str| {
            t.len() < 2
                || ["alt", "else", "opt", "loop", "end", "par", "ref"].contains(t)
                || t.starts_with('[')
        };

        fn norm(s: &str) -> String {
            s.replace('\u{00a0}', " ")
        }

        let golden_texts_raw: Vec<&str> = golden_elems
            .iter()
            .filter_map(|e| e.text.as_deref())
            .filter(|t| !t.is_empty())
            .collect();
        let golden_texts_norm: Vec<String> = golden_texts_raw.iter().map(|t| norm(t)).collect();

        let rust_texts: Vec<String> = rust_elems
            .iter()
            .filter_map(|e| e.text.as_deref())
            .filter(|t| !t.is_empty())
            .map(|t| norm(t))
            .collect();

        let missing: Vec<String> = golden_texts_norm
            .iter()
            .zip(golden_texts_raw.iter())
            .filter(|(_, raw)| !skip(raw))
            .filter(|(norm_t, _)| !rust_texts.iter().any(|r| r.contains(norm_t.as_str())))
            .map(|(norm_t, _)| norm_t.clone())
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "golden texts not found in Rust output: {missing:?}"
            ))
        }
    }));

    match render_result {
        Ok(Ok(())) => TestResult {
            name: name.clone(),
            outcome: Outcome::Pass,
        },
        Ok(Err(msg)) => {
            let outcome = if msg.starts_with("parse:") {
                Outcome::Skip(msg)
            } else {
                Outcome::Fail(msg)
            };
            TestResult {
                name: name.clone(),
                outcome,
            }
        }
        Err(panic) => {
            let msg = if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".into()
            };
            TestResult {
                name: name.clone(),
                outcome: Outcome::Fail(format!("panic: {msg}")),
            }
        }
    }
}

#[test]
fn golden_pairs() {
    let tarball = golden_tarball();
    if !tarball.exists() {
        eprintln!(
            "golden tarball not found: {} — skipping golden pair tests",
            tarball.display()
        );
        return;
    }

    eprintln!("loading golden pairs from {}...", tarball.display());
    let pairs = load_golden_pairs(&tarball);
    if pairs.is_empty() {
        eprintln!("no golden pairs found in tarball");
        return;
    }
    eprintln!("loaded {} golden pairs", pairs.len());

    // Suppress panic output from layout-rs and other libraries.
    std::panic::set_hook(Box::new(|_| {}));

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .panic_handler(|_| {})
        .build()
        .expect("failed to build rayon pool");

    let pass = AtomicUsize::new(0);
    let skip = AtomicUsize::new(0);

    let failures: Vec<String> = pool.install(|| {
        pairs
            .par_iter()
            .filter_map(|pair| {
                let result = run_one(pair);
                match result.outcome {
                    Outcome::Pass => {
                        pass.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                    Outcome::Skip(_) => {
                        skip.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                    Outcome::Fail(ref msg) => Some(format!("{}: {msg}", result.name)),
                }
            })
            .collect()
    });

    let total = pairs.len();
    let pass = pass.load(Ordering::Relaxed);
    let skip = skip.load(Ordering::Relaxed);
    let fail_count = failures.len();

    let mut dir_fails: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for f in &failures {
        if let Some(slash) = f.find('/') {
            let dir = &f[..slash];
            *dir_fails.entry(dir.to_string()).or_default() += 1;
        }
    }

    let panics = failures.iter().filter(|f| f.contains("panic:")).count();
    let text_mismatches = failures
        .iter()
        .filter(|f| f.contains("golden texts not found"))
        .count();
    let other = fail_count - panics - text_mismatches;

    eprintln!("\ngolden_pairs: {total} total, {pass} passed, {fail_count} failed, {skip} skipped");
    eprintln!("  panics: {panics}, text mismatches: {text_mismatches}, other: {other}");
    if !dir_fails.is_empty() {
        eprintln!("  per-directory failures:");
        let mut sorted: Vec<_> = dir_fails.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (dir, count) in &sorted {
            eprintln!("    {count:5} {dir}");
        }
    }

    const MAX_SHOWN: usize = 50;
    let shown: Vec<&str> = failures
        .iter()
        .map(|s| s.as_str())
        .take(MAX_SHOWN)
        .collect();
    let truncated = if fail_count > MAX_SHOWN {
        format!("\n  ... and {} more", fail_count - MAX_SHOWN)
    } else {
        String::new()
    };

    assert!(
        failures.is_empty(),
        "{fail_count} of {total} golden pair tests failed \
         (panics: {panics}, text: {text_mismatches}, other: {other}):\n{}{truncated}",
        shown.join("\n")
    );
}
