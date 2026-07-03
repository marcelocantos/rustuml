// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Shared primitives for the golden-pair test harnesses.
//!
//! Both parity tiers use these: the strict tier (`tests/golden_pairs.rs`,
//! which injects oracle layout extracted from the golden) and the no-oracle
//! product tier (`tests/golden_no_oracle.rs`, which renders through the same
//! path the CLI uses). Keeping the corpus walk, skip classification, and the
//! determinism pin in one place guarantees the two tiers agree on what a
//! testable golden is.

use std::path::{Path, PathBuf};

/// Root of the golden corpus (git submodule; may be absent).
pub fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-diagrams/golden")
}

pub const SUPPORTED_START_KEYWORDS: &[&str] = &[
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

pub fn has_supported_start_keyword(source: &str) -> bool {
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

/// True when the golden SVG is a Java PlantUML error page rather than a
/// rendered diagram. Such pairs are skipped by both tiers: reproducing
/// Java's error screen byte-for-byte is not a parity goal.
///
/// HONESTY GUARD: `tests/harness_guards.rs` pins the number of goldens this
/// matches. Adding a marker string here reclassifies failures as skips —
/// that census must be updated deliberately, with user sign-off, never as a
/// side effect of chasing a failing golden.
pub fn golden_has_syntax_error(svg: &str) -> bool {
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

/// Ditaa produces raster images, not SVG elements — skipped by both tiers.
pub fn is_ditaa(source: &str) -> bool {
    source.lines().any(|l| l.trim().starts_with("@startditaa"))
}

/// `%date(...)` goldens in the corpus were all generated on 2026-03-23
/// AEDT (UTC+11). The `edge_seq_title_with_variables` golden captures a
/// specific second within that day (`Mon Mar 23 07:13:46 AEDT 2026`,
/// epoch 1774210426s); the others (`yyyy-MM-dd` form) only depend on the
/// day, so this pin works for all three.
pub const GOLDEN_DEBUG: &str = "date=1774210426000,tz=AEDT+1100";

/// All .puml files under `root` that have a sibling .svg golden, sorted.
pub fn collect_golden_pairs(root: &Path) -> Vec<PathBuf> {
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
        } else if path.extension().is_some_and(|e| e == "puml")
            && path.with_extension("svg").exists()
        {
            pairs.push(path);
        }
    }
}
