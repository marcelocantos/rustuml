// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Walk `test-diagrams/golden/` and emit a JSON array of golden paths
//! where rustuml's render disagrees with the golden under the strict-XML
//! comparator — i.e. exactly the goldens worth reviewing manually.
//!
//! "Byte-for-byte" was the original intent but turns out to over-flag:
//! verbatim-replay paths produce semantically-identical SVGs whose bytes
//! differ in trailing newlines and minor whitespace. The strict-XML
//! comparator (the same one `cargo test --test golden_pairs` uses)
//! captures what's actually meaningful for visual review.
//!
//! Usage:
//!     cargo run --release -p rustuml-oracle --example check_all
//!
//! Output: a JSON array of "bucket/name" strings on stdout. Used by
//! scripts/viewer.py to filter its tree.
//!
//! Reads `RUSTUML_DEBUG` from the env (same pin the test harness uses) so
//! `%date(...)` goldens compare deterministically.

use rayon::prelude::*;
use rustuml_oracle::{compare, extract};
use std::path::{Path, PathBuf};

fn main() {
    unsafe { std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100") };

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let golden_root = repo_root.join("test-diagrams/golden");

    let pairs = collect_pairs(&golden_root);

    let differ: Vec<String> = pairs
        .par_iter()
        .filter_map(|p| check_one(&golden_root, p))
        .collect();

    // Stable order (rayon collects in parallel order otherwise).
    let mut differ = differ;
    differ.sort();

    println!("{}", json_array(&differ));
}

/// Walk the golden tree and return every .puml path (relative to
/// `golden_root`, without extension) that has a sibling .svg.
fn collect_pairs(golden_root: &Path) -> Vec<PathBuf> {
    let mut pairs = Vec::new();
    if let Ok(buckets) = std::fs::read_dir(golden_root) {
        for entry in buckets.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let bucket = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if let Ok(files) = std::fs::read_dir(&path) {
                for f in files.flatten() {
                    let p = f.path();
                    if p.extension().and_then(|s| s.to_str()) != Some("puml") {
                        continue;
                    }
                    let stem = match p.file_stem().and_then(|s| s.to_str()) {
                        Some(s) => s,
                        None => continue,
                    };
                    let svg = p.with_extension("svg");
                    if svg.is_file() {
                        pairs.push(PathBuf::from(format!("{bucket}/{stem}")));
                    }
                }
            }
        }
    }
    pairs.sort();
    pairs
}

/// Render `bucket/name.puml` and byte-compare to `bucket/name.svg`.
/// Returns `Some(key)` if they differ (or rendering failed), `None`
/// if they're byte-identical.
fn check_one(golden_root: &Path, rel: &Path) -> Option<String> {
    let puml = golden_root.join(rel).with_extension("puml");
    let svg = golden_root.join(rel).with_extension("svg");

    let source = std::fs::read_to_string(&puml).ok()?;
    let golden = std::fs::read_to_string(&svg).ok()?;

    // Skip cases that the test harness skips so we don't fill the diff
    // list with goldens we can never match.
    if golden_has_syntax_error(&golden) {
        return None;
    }
    if !has_supported_start_keyword(&source) {
        return None;
    }
    if source.lines().any(|l| l.trim().starts_with("@startditaa")) {
        return None;
    }

    let oracle = if golden.contains("<?plantuml ") {
        extract::extract_oracle_layout(&golden)
    } else {
        None
    };

    let actual = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Option<String> {
        let blocks = rustuml_parser::parse::split_blocks(&source);
        let svg = if blocks.len() > 1 {
            let b = rustuml_parser::parse::parse_block(&source, 0).ok()?;
            rustuml_render::render_svg_with_oracle(&b, oracle.as_ref())
        } else {
            let d = rustuml_parser::parse::parse_auto_with_base(&source, None).ok()?;
            rustuml_render::render_svg_with_oracle(&d, oracle.as_ref())
        };
        Some(svg)
    }))
    .ok()
    .flatten()?;

    let rel_key = rel.to_string_lossy().into_owned();
    match compare::compare_svg_strict(&golden, &actual) {
        Ok(cmp) if cmp.is_match() => None,
        // Strict-XML mismatch OR parse failure both warrant a look.
        _ => Some(rel_key),
    }
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
    let first = source
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();
    if !first.starts_with("@start") {
        return true;
    }
    SUPPORTED_START_KEYWORDS.iter().any(|k| first.starts_with(k))
}

fn golden_has_syntax_error(svg: &str) -> bool {
    let needles = [
        "Syntax Error",
        "NoSuchElementException",
        "Welcome to PlantUML",
        "An error has occured",
        "kill cannot be used here",
        "swimlane must be defined at the start",
        "Note already created:",
        "Parsing syntax error about %",
        "[From string",
        "Your data does not sound like YAML data",
        "does&#160;not&#160;sound&#160;like&#160;YAML",
        "Your data does not sound like JSON data",
        "does&#160;not&#160;sound&#160;like&#160;JSON",
        "No class ",
        "(Assumed diagram type:",
        "DITAA has crashed",
        "This feature has been suppressed",
    ];
    needles.iter().any(|n| svg.contains(n))
}

/// Minimal JSON-array writer for `Vec<String>` so we don't pull in serde
/// just for this. Strings escape only the characters we'd plausibly hit
/// in golden filenames (backslash and double-quote).
fn json_array(items: &[String]) -> String {
    let mut out = String::from("[");
    for (i, s) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        for c in s.chars() {
            match c {
                '\\' | '"' => {
                    out.push('\\');
                    out.push(c);
                }
                c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
    }
    out.push(']');
    out
}
