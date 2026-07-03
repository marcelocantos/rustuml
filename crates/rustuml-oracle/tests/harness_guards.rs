// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Honesty guards for the parity harness (🎯T14.1).
//!
//! These tests pin the mechanisms by which "parity" could be gamed rather
//! than earned. Each guard encodes a lesson from a real incident in this
//! repo's history (verbatim golden replay, May 2026; fixture-echo
//! `include_str!` guards, June 2026). Loosening any of them is a deliberate,
//! user-signed-off act — never a side effect of chasing a failing golden.

use rustuml_oracle::harness::{collect_golden_pairs, golden_dir, golden_has_syntax_error};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Shipping crates must not embed golden test data. `include_str!` /
/// `include_bytes!` of anything under test-diagrams/ compiles the expected
/// answer into the product — the June 2026 fixture-echo incident.
///
/// The allow-list below is a RATCHET inherited from that incident. 🎯T14.2
/// drives it to zero, after which this list must stay empty forever.
/// Provenance COMMENTS citing goldens are fine and encouraged; this guard
/// only matches the compile-time embedding vector.
#[test]
fn no_golden_data_embedded_in_shipping_crates() {
    // (crate-relative file, max allowed embedding lines)
    const ALLOWED: &[(&str, usize)] = &[
        // The June 2026 swimlane/showcase fixture echoes. To be deleted in
        // 🎯T14.2; must never grow.
        ("crates/rustuml-render/src/activity.rs", 20),
    ];
    const SHIPPING_CRATES: &[&str] = &[
        "crates/rustuml",
        "crates/rustuml-parser",
        "crates/rustuml-render",
        "crates/rustuml-layout",
        "crates/rustuml-math",
    ];

    let root = repo_root();
    let mut violations = Vec::new();
    for krate in SHIPPING_CRATES {
        let mut files = Vec::new();
        rust_sources(&root.join(krate).join("src"), &mut files);
        for file in files {
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            // Non-comment lines referencing the corpus. Covers single-line
            // and multi-line include_str!/include_bytes! forms (the path
            // string always names test-diagrams); provenance comments are
            // exempt by the comment check.
            let count = text
                .lines()
                .filter(|l| l.contains("test-diagrams") && !l.trim_start().starts_with("//"))
                .count();
            if count == 0 {
                continue;
            }
            let rel = file
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            let allowed = ALLOWED
                .iter()
                .find(|(f, _)| *f == rel)
                .map(|(_, n)| *n)
                .unwrap_or(0);
            if count > allowed {
                violations.push(format!(
                    "{rel}: {count} golden references (allowed {allowed})"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "golden test data referenced from shipping crate source — this embeds \
         the expected answer in the product (see 🎯T14.2):\n  {}",
        violations.join("\n  ")
    );
}

/// The number of goldens classified as Java error pages (and therefore
/// skipped by both parity tiers) is pinned. If this fails, either the corpus
/// changed (update deliberately, citing the submodule bump) or an error
/// marker was added to `golden_has_syntax_error` — which reclassifies
/// failures as skips and requires explicit user sign-off.
#[test]
fn error_golden_census_is_frozen() {
    const EXPECTED_ERROR_GOLDENS: usize = 1199;

    let root = golden_dir();
    if !root.exists() || !root.join("sequence").exists() {
        eprintln!("golden submodule not populated — census skipped");
        return;
    }
    let census = collect_golden_pairs(&root)
        .iter()
        .filter(|p| {
            std::fs::read_to_string(p.with_extension("svg"))
                .is_ok_and(|svg| golden_has_syntax_error(&svg))
        })
        .count();
    assert_eq!(
        census, EXPECTED_ERROR_GOLDENS,
        "error-golden census changed ({EXPECTED_ERROR_GOLDENS} -> {census}). If the corpus \
         grew, update the constant citing the submodule bump. If a marker string was added \
         to golden_has_syntax_error, stop: that converts failures into skips."
    );
}

/// Count of high-precision decimal literals (3+ decimal places) per
/// rustuml-render source file, ratcheted against a checked-in baseline.
///
/// Goodhart guard (oracle-first rule 6): a new measured constant is only
/// legitimate when it traces to a structural counterpart in the Java
/// reference or an extracted metrics table, cited in an adjacent comment.
/// This guard can't read comments, so it makes growth VISIBLE: adding a
/// high-precision literal fails until the baseline is updated in the same
/// change, where the diff reviewer checks the provenance comment.
/// Decreases are progress; lock them in with HARNESS_WRITE_BASELINES=1.
#[test]
fn decimal_literal_ratchet() {
    let root = repo_root();
    let baseline_path = root.join("crates/rustuml-oracle/tests/decimal_literal_baseline.txt");

    let mut files = Vec::new();
    rust_sources(&root.join("crates/rustuml-render/src"), &mut files);
    files.sort();

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let n = count_high_precision_literals(&text);
        if n > 0 {
            let rel = file
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            counts.insert(rel, n);
        }
    }

    if std::env::var("HARNESS_WRITE_BASELINES").is_ok() {
        let mut out = String::from(
            "# High-precision (3+ dp) decimal-literal counts per rustuml-render source file.\n\
             # Ratchet: counts may only grow alongside a provenance comment citing the Java\n\
             # source or an extracted metrics table, reviewed in the same change.\n\
             # Regenerate: HARNESS_WRITE_BASELINES=1 cargo test --test harness_guards\n",
        );
        for (file, n) in &counts {
            out.push_str(&format!("{file} {n}\n"));
        }
        std::fs::write(&baseline_path, out).expect("write baseline");
        eprintln!("baseline written to {}", baseline_path.display());
        return;
    }

    let baseline_text = std::fs::read_to_string(&baseline_path).unwrap_or_else(|e| {
        panic!(
            "missing baseline {} ({e}); bootstrap with HARNESS_WRITE_BASELINES=1",
            baseline_path.display()
        )
    });
    let mut baseline: BTreeMap<String, usize> = BTreeMap::new();
    for line in baseline_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (file, n) = line.rsplit_once(' ').expect("malformed baseline line");
        baseline.insert(file.to_string(), n.parse().expect("malformed count"));
    }

    let mut violations = Vec::new();
    for (file, n) in &counts {
        let allowed = baseline.get(file).copied().unwrap_or(0);
        if *n > allowed {
            violations.push(format!("{file}: {allowed} -> {n}"));
        }
    }
    assert!(
        violations.is_empty(),
        "new high-precision decimal literals in rustuml-render — each needs a provenance \
         comment (Java source or metrics-table derivation) and a reviewed baseline update \
         (HARNESS_WRITE_BASELINES=1):\n  {}",
        violations.join("\n  ")
    );
}

/// Matches `<digit>.<digit>{3,}` — the signature of a measured/curve-fit
/// constant as opposed to a designed one (2.5, 10.0).
fn count_high_precision_literals(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'.'
            && i > 0
            && bytes[i - 1].is_ascii_digit()
            && bytes.len() - i > 3
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
        {
            count += 1;
            // Skip the rest of this number so one literal counts once.
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    count
}

/// The comparator's slack and the park list are frozen. Changing either
/// redefines what "parity" means — user sign-off territory.
#[test]
fn comparator_constants_frozen() {
    assert_eq!(
        rustuml_oracle::compare::GEOM_EPS,
        0.02,
        "GEOM_EPS changed — this redefines strict parity and needs user sign-off"
    );

    let parked = std::fs::read_to_string(repo_root().join("test-diagrams/ulp_parked.txt"))
        .unwrap_or_default();
    let entries: Vec<&str> = parked
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    assert!(
        entries.is_empty(),
        "ulp_parked.txt gained entries — parking excludes failures from the count and \
         needs user sign-off: {entries:?}"
    );
}
