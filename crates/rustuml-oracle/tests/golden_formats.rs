// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Format smoke tests — for every golden .puml file that Rust can parse and
//! render to SVG, verify that PNG, PDF, and EPS conversion does not crash.
//!
//! These tests do **not** compare output against golden files — they only
//! check that the conversion pipeline returns Ok and produces bytes with
//! the correct magic header.
//!
//! Gated behind `--features format-tests` since the additional rendering
//! adds significant time to the test suite.
//!
//! Run with: `cargo test --test golden_formats --features format-tests`

#![cfg(feature = "format-tests")]

use rayon::prelude::*;
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
    "@startdot",
    "@startboard",
    "@startgit",
    "@startebnf",
];

fn has_supported_start_keyword(source: &str) -> bool {
    let first_line = source
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();
    if !first_line.starts_with("@start") {
        return true;
    }
    SUPPORTED_START_KEYWORDS
        .iter()
        .any(|kw| first_line.starts_with(kw))
}

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
        || svg.contains("Your data does not sound like JSON data")
        || svg.contains("does&#160;not&#160;sound&#160;like&#160;JSON")
        || svg.contains("No class ")
        || svg.contains("(Assumed diagram type:")
        || svg.contains("DITAA has crashed")
        || svg.contains("This feature has been suppressed")
}

fn collect_golden_pumls(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_recursive(root, &mut files);
    files.sort();
    files
}

fn collect_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, files);
        } else if path.extension().is_some_and(|e| e == "puml") {
            if path.with_extension("svg").exists() {
                files.push(path);
            }
        }
    }
}

struct FormatResult {
    name: String,
    outcome: FormatOutcome,
}

enum FormatOutcome {
    Pass,
    Skip(String),
    Fail(String),
}

/// Try to parse and render the given .puml to SVG, then convert to PNG, PDF,
/// and EPS. Returns a failure message if any conversion errors.
fn run_one(puml_path: &Path, root: &Path, eps_sample: bool) -> FormatResult {
    let rel = puml_path
        .strip_prefix(root)
        .unwrap()
        .with_extension("")
        .to_string_lossy()
        .to_string();

    let source = match std::fs::read_to_string(puml_path) {
        Ok(s) => s,
        Err(e) => {
            return FormatResult {
                name: rel,
                outcome: FormatOutcome::Skip(format!("read puml: {e}")),
            };
        }
    };

    if source.contains("%date()") {
        return FormatResult {
            name: rel,
            outcome: FormatOutcome::Skip("non-deterministic %date()".into()),
        };
    }

    // Check golden SVG for known errors.
    let golden_svg = match std::fs::read_to_string(puml_path.with_extension("svg")) {
        Ok(s) => s,
        Err(e) => {
            return FormatResult {
                name: rel,
                outcome: FormatOutcome::Skip(format!("read svg: {e}")),
            };
        }
    };

    if golden_has_syntax_error(&golden_svg) {
        return FormatResult {
            name: rel,
            outcome: FormatOutcome::Skip("golden SVG contains error".into()),
        };
    }
    if !has_supported_start_keyword(&source) {
        return FormatResult {
            name: rel,
            outcome: FormatOutcome::Skip("unsupported keyword".into()),
        };
    }

    let is_multi_block = rustuml_parser::parse::split_blocks(&source).len() > 1;

    let render_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Parse.
        let diagram = if is_multi_block {
            rustuml_parser::parse::parse_block(&source, 0).map_err(|e| format!("parse: {e}"))?
        } else {
            rustuml_parser::parse::parse_auto_with_base(&source, None)
                .map_err(|e| format!("parse: {e}"))?
        };

        // Render to SVG.
        let svg = rustuml_render::render_svg(&diagram);

        let mut errors = Vec::new();

        // PNG conversion.
        match rustuml_render::png::svg_to_png(&svg) {
            Ok(png) => {
                if png.len() < 8 || &png[..4] != &[137, 80, 78, 71] {
                    errors.push("PNG: missing magic bytes".to_string());
                }
            }
            Err(e) => errors.push(format!("PNG: {e}")),
        }

        // PDF conversion.
        match rustuml_render::pdf::svg_to_pdf(&svg) {
            Ok(pdf) => {
                if pdf.len() < 5 || &pdf[..5] != b"%PDF-" {
                    errors.push("PDF: missing magic bytes".to_string());
                }
            }
            Err(e) => errors.push(format!("PDF: {e}")),
        }

        // EPS conversion (only for the sampled subset).
        if eps_sample {
            match rustuml_render::eps::svg_to_eps(&svg) {
                Ok(eps) => {
                    if eps.len() < 9 || &eps[..9] != b"%!PS-Adob" {
                        errors.push("EPS: missing magic bytes".to_string());
                    }
                }
                Err(e) => errors.push(format!("EPS: {e}")),
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }));

    match render_result {
        Ok(Ok(())) => FormatResult {
            name: rel,
            outcome: FormatOutcome::Pass,
        },
        Ok(Err(msg)) => {
            let outcome = if msg.starts_with("parse:") {
                FormatOutcome::Skip(msg)
            } else {
                FormatOutcome::Fail(msg)
            };
            FormatResult { name: rel, outcome }
        }
        Err(panic) => {
            let msg = if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".into()
            };
            FormatResult {
                name: rel,
                outcome: FormatOutcome::Fail(format!("panic: {msg}")),
            }
        }
    }
}

#[test]
fn golden_format_smoke() {
    let root = golden_dir();
    if !root.exists() || !root.join("sequence").exists() {
        eprintln!("golden submodule not populated — run: git submodule update --init");
        return;
    }

    let pairs = collect_golden_pumls(&root);
    if pairs.is_empty() {
        eprintln!("no golden pairs found");
        return;
    }
    eprintln!(
        "running {} golden format smoke tests (PNG + PDF for all, EPS for sample)...",
        pairs.len()
    );

    // Suppress panic output from catch_unwind.
    std::panic::set_hook(Box::new(|_| {}));

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .panic_handler(|_| {})
        .build()
        .expect("failed to build rayon pool");

    // EPS is the slowest conversion (rasterize + ASCII85 encode), so we only
    // run it on every Nth file to keep the total time reasonable.
    const EPS_SAMPLE_STRIDE: usize = 10;

    let pass = AtomicUsize::new(0);
    let skip = AtomicUsize::new(0);

    let failures: Vec<String> = pool.install(|| {
        pairs
            .par_iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let eps_sample = i % EPS_SAMPLE_STRIDE == 0;
                let r = run_one(p, &root, eps_sample);
                match r.outcome {
                    FormatOutcome::Pass => {
                        pass.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                    FormatOutcome::Skip(_) => {
                        skip.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                    FormatOutcome::Fail(ref msg) => Some(format!("{}: {msg}", r.name)),
                }
            })
            .collect()
    });

    let total = pairs.len();
    let pass = pass.load(Ordering::Relaxed);
    let skip = skip.load(Ordering::Relaxed);
    let fail_count = failures.len();
    let eps_count = (total + EPS_SAMPLE_STRIDE - 1) / EPS_SAMPLE_STRIDE;

    eprintln!(
        "\ngolden_format_smoke: {total} total, {pass} passed, {fail_count} failed, {skip} skipped"
    );
    eprintln!("  EPS tested on ~{eps_count} files (1/{EPS_SAMPLE_STRIDE} sample)");

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
        "{fail_count} of {total} golden format smoke tests failed:\n{}{truncated}",
        shown.join("\n")
    );
}
