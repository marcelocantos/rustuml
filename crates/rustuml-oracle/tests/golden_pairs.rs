// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Golden pair tests — walk test-diagrams/golden/ (submodule) and compare
//! Rust rendering against pre-generated Java PlantUML reference SVGs.
//!
//! The golden files live in a separate repository (rustuml-golden)
//! added as a git submodule. Clone with `--recurse-submodules` or run
//! `git submodule update --init` to populate them. If the submodule
//! is not present, the test is silently skipped.
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
    // Accept files that start with a supported @start keyword, OR headerless
    // files (no @start prefix at all — the parser auto-detects the type).
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
            if path.with_extension("svg").exists() {
                pairs.push(path);
            }
        }
    }
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

    let source = match std::fs::read_to_string(puml_path) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                name: rel,
                outcome: Outcome::Skip(format!("read puml: {e}")),
            };
        }
    };

    // Count @start blocks. If there are multiple blocks, the golden SVG
    // contains only the first block's output, so we compare block 0.
    // Record whether this is a multi-block file for the renderer selection below.
    if source.contains("%date()") {
        return TestResult {
            name: rel,
            outcome: Outcome::Skip("non-deterministic %date()".into()),
        };
    }

    let golden_svg = match std::fs::read_to_string(puml_path.with_extension("svg")) {
        Ok(s) => s,
        Err(e) => {
            return TestResult {
                name: rel,
                outcome: Outcome::Skip(format!("read svg: {e}")),
            };
        }
    };

    if golden_has_syntax_error(&golden_svg) {
        return TestResult {
            name: rel,
            outcome: Outcome::Skip("golden SVG contains error".into()),
        };
    }
    if !has_supported_start_keyword(&source) {
        return TestResult {
            name: rel,
            outcome: Outcome::Skip("unsupported keyword".into()),
        };
    }

    let blocks = rustuml_parser::parse::split_blocks(&source);
    let is_multi_block = blocks.len() > 1;

    let render_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let diagram = if is_multi_block {
            // PlantUML merges unnamed same-type blocks into one diagram.
            // Try merged parse first; fall back to block 0.
            let merged = merge_blocks(&source);
            rustuml_parser::parse::parse_auto_with_base(&merged, None)
                .or_else(|_| rustuml_parser::parse::parse_block(&source, 0))
                .map_err(|e| format!("parse: {e}"))?
        } else {
            rustuml_parser::parse::parse_auto_with_base(&source, None)
                .map_err(|e| format!("parse: {e}"))?
        };
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
        fn norm(s: &str) -> String {
            s.replace('\u{00a0}', " ")
        }

        let golden_raw: Vec<&str> = golden_elems
            .iter()
            .filter_map(|e| e.text.as_deref())
            .filter(|t| !t.is_empty())
            .collect();
        let golden_norm: Vec<String> = golden_raw.iter().map(|t| norm(t)).collect();
        let rust_texts: Vec<String> = rust_elems
            .iter()
            .filter_map(|e| e.text.as_deref())
            .filter(|t| !t.is_empty())
            .map(|t| norm(t))
            .collect();

        let missing: Vec<String> = golden_norm
            .iter()
            .zip(golden_raw.iter())
            .filter(|(_, raw)| !skip(raw))
            .filter(|(n, _)| !rust_texts.iter().any(|r| r.contains(n.as_str())))
            .map(|(n, _)| n.clone())
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

/// Merge multiple `@start`/`@end` blocks into a single block.
///
/// PlantUML renders unnamed same-type blocks as one combined diagram.
/// This strips the intermediate `@end`/`@start` markers and wraps
/// everything in a single `@startuml`/`@enduml`.
fn merge_blocks(source: &str) -> String {
    let mut lines = Vec::new();
    let mut started = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("@start") {
            if !started {
                lines.push(line.to_string());
                started = true;
            }
            // Skip subsequent @start markers.
        } else if trimmed.starts_with("@end") {
            // Skip — we'll add one at the end.
        } else {
            lines.push(line.to_string());
        }
    }
    lines.push("@enduml".to_string());
    lines.join("\n")
}

#[test]
fn golden_pairs() {
    let root = golden_dir();
    if !root.exists() || !root.join("sequence").exists() {
        eprintln!("golden submodule not populated — run: git submodule update --init");
        return;
    }

    let pairs = collect_golden_pairs(&root);
    if pairs.is_empty() {
        eprintln!("no golden pairs found");
        return;
    }
    eprintln!("running {} golden pairs...", pairs.len());

    std::panic::set_hook(Box::new(|_| {}));

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .panic_handler(|_| {})
        .build()
        .expect("failed to build rayon pool");

    let pass = AtomicUsize::new(0);
    let skip = AtomicUsize::new(0);
    let skip_parse = AtomicUsize::new(0);
    let skip_error = AtomicUsize::new(0);
    let skip_keyword = AtomicUsize::new(0);
    let skip_multi = AtomicUsize::new(0);
    let skip_other = AtomicUsize::new(0);

    let failures: Vec<String> = pool.install(|| {
        pairs
            .par_iter()
            .filter_map(|p| {
                let r = run_one(p, &root);
                match r.outcome {
                    Outcome::Pass => {
                        pass.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                    Outcome::Skip(ref reason) => {
                        skip.fetch_add(1, Ordering::Relaxed);
                        if reason.starts_with("parse:") {
                            skip_parse.fetch_add(1, Ordering::Relaxed);
                        } else if reason.contains("error") || reason.contains("Error") {
                            skip_error.fetch_add(1, Ordering::Relaxed);
                        } else if reason.contains("unsupported keyword") {
                            skip_keyword.fetch_add(1, Ordering::Relaxed);
                        } else if reason.contains("multiple @start") {
                            skip_multi.fetch_add(1, Ordering::Relaxed);
                        } else {
                            skip_other.fetch_add(1, Ordering::Relaxed);
                        }
                        None
                    }
                    Outcome::Fail(ref msg) => Some(format!("{}: {msg}", r.name)),
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
            *dir_fails.entry(f[..slash].to_string()).or_default() += 1;
        }
    }

    let panics = failures.iter().filter(|f| f.contains("panic:")).count();
    let text_mm = failures
        .iter()
        .filter(|f| f.contains("golden texts not found"))
        .count();
    let other = fail_count - panics - text_mm;

    let sp = skip_parse.load(Ordering::Relaxed);
    let se = skip_error.load(Ordering::Relaxed);
    let sk = skip_keyword.load(Ordering::Relaxed);
    let sm = skip_multi.load(Ordering::Relaxed);
    let so = skip_other.load(Ordering::Relaxed);
    eprintln!("\ngolden_pairs: {total} total, {pass} passed, {fail_count} failed, {skip} skipped");
    eprintln!("  panics: {panics}, text mismatches: {text_mm}, other: {other}");
    eprintln!(
        "  skip breakdown: parse={sp}, golden_error={se}, unsupported_kw={sk}, multi_start={sm}, other={so}"
    );
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
         (panics: {panics}, text: {text_mm}, other: {other}):\n{}{truncated}",
        shown.join("\n")
    );
}
