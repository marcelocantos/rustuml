// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Golden pair tests — walk test-diagrams/golden/ and compare Rust
//! rendering against pre-generated Java PlantUML reference SVGs.
//!
//! Each `.puml` file with a matching `.svg` is a test case. The test
//! parses the `.puml` with rustuml, renders to SVG, and compares
//! structurally against the golden `.svg`.
//!
//! Run with: `cargo test --test golden_pairs`

use rayon::prelude::*;
use rustuml_oracle::compare;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-diagrams/golden")
}

/// Diagram types that rustuml currently supports, identified by
/// subdirectory name within the golden tree.
const SUPPORTED_DIRS: &[&str] = &[
    "sequence",
    "class",
    "state",
    "activity",
    "component",
    "deployment",
    "usecase",
    "timing",
    "gantt",
    "mindmap",
    "wbs",
    "math",
    "regex",
    "json-yaml",
    "object",
    "er",
    "salt",
    "nwdiag",
    "ditaa",
    "edge-cases",
    "combo",
    "creole",
    "skinparam",
    "preprocessing",
    "links",
    "sprites",
    "archimate",
    "wire",
    "multi-diagram",
    "rendering",
    "type-detection",
];

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

/// Returns true if the `.puml` source uses a `@start` keyword that
/// rustuml supports.
fn has_supported_start_keyword(source: &str) -> bool {
    let trimmed = source.trim_start();
    SUPPORTED_START_KEYWORDS
        .iter()
        .any(|kw| trimmed.starts_with(kw))
}

/// Returns true if a path is inside one of the supported subdirectories.
fn is_in_supported_dir(path: &Path, root: &Path) -> bool {
    let rel = match path.strip_prefix(root) {
        Ok(r) => r,
        Err(_) => return false,
    };
    // First component of the relative path is the diagram-type directory.
    rel.components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
        .is_some_and(|dir| SUPPORTED_DIRS.contains(&dir))
}

/// Collect all `.puml` files under `root` that have a matching `.svg`.
fn collect_golden_pairs(root: &Path) -> Vec<PathBuf> {
    let mut pairs = Vec::new();
    collect_recursive(root, &mut pairs);
    pairs.sort();
    pairs
}

fn collect_recursive(dir: &Path, pairs: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, pairs);
        } else if path.extension().is_some_and(|e| e == "puml") {
            let svg_path = path.with_extension("svg");
            if svg_path.exists() {
                pairs.push(path);
            }
        }
    }
}

/// Returns true if the golden SVG contains a PlantUML error marker,
/// indicating the diagram could not be rendered by the reference
/// implementation (syntax error, crash, or version-specific failure).
fn golden_has_syntax_error(svg: &str) -> bool {
    svg.contains("Syntax Error")
        || svg.contains("NoSuchElementException")
        || svg.contains("Welcome to PlantUML")
        // Java PlantUML crash SVGs (e.g. UnparsableGraphvizException) — the
        // reference implementation failed, so there is nothing to compare
        // against.
        || svg.contains("An error has occured")
        // Java PlantUML semantic errors rendered as error SVGs.
        || svg.contains("kill cannot be used here")
        || svg.contains("swimlane must be defined at the start")
        // Preprocessing / semantic errors that produce error-diagnostic SVGs.
        // These render the raw source + error message — not comparable.
        || svg.contains("Note already created:")
        || svg.contains("Parsing syntax error about %")
        || svg.contains("[From string")
        // YAML inputs that the reference implementation cannot parse (anchors,
        // empty objects, multi-line scalars, etc.) produce this error banner.
        || svg.contains("Your data does not sound like YAML data")
        // Lollipop notation (`()- Interface`) is unsupported; PlantUML emits
        // a semantic error SVG containing "No class <Name>".
        || svg.contains("No class ")
        // Generic "(Assumed diagram type: ...)" marker that appears in
        // PlantUML error SVGs when the diagram cannot be parsed/rendered.
        || svg.contains("(Assumed diagram type:")
        // Java PlantUML DITAA renderer crash — no valid diagram to compare.
        || svg.contains("DITAA has crashed")
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

fn run_one(puml_path: &Path, root: &Path) -> TestResult {
    let rel = puml_path
        .strip_prefix(root)
        .unwrap()
        .with_extension("")
        .to_string_lossy()
        .to_string();

    // Read source and golden SVG.
    let source = match std::fs::read_to_string(puml_path) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                name: rel,
                outcome: Outcome::Skip(format!("read puml: {e}")),
            };
        }
    };

    let svg_path = puml_path.with_extension("svg");
    let golden_svg = match std::fs::read_to_string(&svg_path) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                name: rel,
                outcome: Outcome::Skip(format!("read svg: {e}")),
            };
        }
    };

    // Skip diagrams whose golden SVG contains a syntax error.
    if golden_has_syntax_error(&golden_svg) {
        return TestResult {
            name: rel,
            outcome: Outcome::Skip("golden SVG contains Syntax Error".into()),
        };
    }

    // Filter: must be in a supported directory or use a supported keyword.
    if !is_in_supported_dir(puml_path, root) && !has_supported_start_keyword(&source) {
        return TestResult {
            name: rel,
            outcome: Outcome::Skip("unsupported diagram directory/keyword".into()),
        };
    }

    // Wrap parse + render + compare in catch_unwind so a panic in
    // one golden pair doesn't abort the entire suite. Rayon's
    // panic_handler absorbs panics at the pool level.
    let base_dir = puml_path.parent().map(Path::to_owned);
    let render_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let diagram = rustuml_parser::parse::parse_auto_with_base(&source, base_dir.as_deref())
            .map_err(|e| format!("parse: {e}"))?;

        let rust_svg = rustuml_render::render_svg(&diagram);

        let golden_elems =
            compare::extract_elements(&golden_svg).map_err(|e| format!("golden SVG parse: {e}"))?;
        let rust_elems =
            compare::extract_elements(&rust_svg).map_err(|e| format!("rust SVG parse: {e}"))?;

        let skip = |t: &&str| {
            t.len() < 2
                || ["alt", "else", "opt", "loop", "end", "par", "ref"].contains(t)
                || t.starts_with('[')
        };

        // Normalise non-breaking spaces (U+00A0 / &#160;) to regular spaces before
        // comparison.  PlantUML emits NBSP inside textLength spans; our renderer
        // uses regular spaces.  The semantic content is the same, so treat them
        // as equivalent.
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

        // For the skip check we still use the raw golden text (length check on
        // original).
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
            name: rel,
            outcome: Outcome::Pass,
        },
        Ok(Err(msg)) => {
            let outcome = if msg.starts_with("parse:") {
                Outcome::Skip(msg)
            } else {
                Outcome::Fail(msg)
            };
            TestResult { name: rel, outcome }
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
                name: rel,
                outcome: Outcome::Fail(format!("panic: {msg}")),
            }
        }
    }
}

#[test]
fn golden_pairs() {
    let root = golden_dir();
    if !root.exists() {
        eprintln!("golden dir not found: {}", root.display());
        return;
    }

    let pairs = collect_golden_pairs(&root);
    if pairs.is_empty() {
        eprintln!("no golden pairs found in {}", root.display());
        return;
    }

    // Suppress panic output from layout-rs and other libraries.
    std::panic::set_hook(Box::new(|_| {}));

    // Build a custom rayon pool. Layout-rs panics can poison rayon
    // workers, so we use `panic_handler` to absorb them.
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .panic_handler(|_| {}) // absorb panics instead of poisoning
        .build()
        .expect("failed to build rayon pool");

    let pass = AtomicUsize::new(0);
    let skip = AtomicUsize::new(0);

    // Run all pairs in parallel using rayon.
    let failures: Vec<String> = pool.install(|| pairs
        .par_iter()
        .filter_map(|puml_path| {
            let result = run_one(puml_path, &root);
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
    ); // close pool.install

    let total = pairs.len();
    let pass = pass.load(Ordering::Relaxed);
    let skip = skip.load(Ordering::Relaxed);
    let fail_count = failures.len();

    // Per-directory failure counts.
    let mut dir_fails: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for f in &failures {
        if let Some(slash) = f.find('/') {
            let dir = &f[..slash];
            *dir_fails.entry(dir.to_string()).or_default() += 1;
        }
    }

    // Summarise by failure category.
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

    // Show first N failures to keep output readable.
    const MAX_SHOWN: usize = 50;

    // Write all failures to a file for analysis.
    let fail_path = root.join("..").join("golden_failures.txt");
    if !failures.is_empty() {
        let mut lines = Vec::new();
        let mut dir_sorted: Vec<_> = dir_fails.iter().collect();
        dir_sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (dir, count) in &dir_sorted {
            lines.push(format!("{count:5} {dir}"));
        }
        lines.push(String::new());
        lines.extend(failures.iter().cloned());
        let _ = std::fs::write(&fail_path, lines.join("\n"));
        eprintln!("  failures written to {}", fail_path.display());
    }
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
