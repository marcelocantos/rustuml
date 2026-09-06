// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Product-path driver for the honesty ratchet (🎯T14).
//!
//! `tools/ratchet/ratchet.py` needs three commands from a project: one that
//! lists the eligible universe, one that runs the SHIPPED path over the
//! corpus, and one that scores the outputs against the frozen reference.
//! This binary is those three, and nothing else — it holds no expectations
//! of its own.
//!
//! The distinction from `tests/golden_no_oracle.rs` is deliberate and is the
//! whole point: that test calls `rustuml_render::render_svg` in-process, so
//! it measures a library path that resembles the CLI. `produce` here spawns
//! the built `rustuml` binary exactly as a user runs it — argument parsing,
//! block selection, base directory, theme defaults and all. Only the second
//! number is the shipped product's.
//!
//! `RUSTUML_DEBUG` is set to `harness::GOLDEN_DEBUG` for every render. That
//! pins the clock the `%date(...)` goldens were generated against; it injects
//! nothing about the expected output.
//!
//!   ratchet_gate universe --corpus DIR
//!   ratchet_gate produce  --corpus DIR --out DIR --bin PATH
//!   ratchet_gate score    --corpus DIR --ref DIR --out DIR

use rustuml_oracle::compare::compare_svg_strict;
use rustuml_oracle::harness::{
    GOLDEN_DEBUG, collect_golden_pairs, golden_has_syntax_error, has_supported_start_keyword,
    is_ditaa,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Fallback when the OS will not report the parallelism available.
const FALLBACK_THREADS: usize = 4;

const USAGE: &str = "usage:
  ratchet_gate universe --corpus DIR
  ratchet_gate produce  --corpus DIR --out DIR --bin PATH
  ratchet_gate score    --corpus DIR --ref DIR --out DIR";

/// One corpus entry: which family it belongs to, its case id (the path below
/// the family directory, extension stripped), and where its source lives.
struct Case {
    family: String,
    id: String,
    source: PathBuf,
}

fn threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(FALLBACK_THREADS)
}

/// Map each thread's slice of `items` through `f` and concatenate in order.
fn map_parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Send + Sync + Copy) -> Vec<R> {
    let chunk = items.len().div_ceil(threads().max(1)).max(1);
    let chunks: Vec<&[T]> = items.chunks(chunk).collect();
    let mut results: Vec<Vec<R>> = Vec::with_capacity(chunks.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = chunks
            .iter()
            .map(|slice| scope.spawn(move || slice.iter().map(f).collect::<Vec<R>>()))
            .collect();
        for handle in handles {
            results.push(handle.join().expect("worker thread panicked"));
        }
    });
    results.into_iter().flatten().collect()
}

/// The eligible universe: corpus pairs minus the frozen skip rules shared
/// with both test tiers (`harness`). These rules may only ever subtract, and
/// the ratchet locks the resulting size, so widening one is visible as
/// UNIVERSE_DRIFT rather than as a free improvement.
fn eligible_cases(corpus: &Path) -> Vec<Case> {
    let mut cases = Vec::new();
    for source in collect_golden_pairs(corpus) {
        let Ok(rel) = source.strip_prefix(corpus) else {
            continue;
        };
        let mut components = rel.components();
        let Some(family) = components.next() else {
            continue;
        };
        let family = family.as_os_str().to_string_lossy().to_string();
        let id = components
            .as_path()
            .with_extension("")
            .to_string_lossy()
            .to_string();
        if id.is_empty() {
            continue;
        }
        let (Ok(text), Ok(golden)) = (
            std::fs::read_to_string(&source),
            std::fs::read_to_string(source.with_extension("svg")),
        ) else {
            continue;
        };
        if golden_has_syntax_error(&golden)
            || !has_supported_start_keyword(&text)
            || is_ditaa(&text)
        {
            continue;
        }
        cases.push(Case { family, id, source });
    }
    cases
}

/// Where a case's SVG lives under a tree rooted at `dir` (the produced
/// output tree and the reference tree share this layout).
fn svg_path(dir: &Path, case: &Case) -> PathBuf {
    dir.join(&case.family).join(&case.id).with_extension("svg")
}

fn by_family<T>(
    cases: &[Case],
    values: &[T],
    keep: impl Fn(&T) -> bool,
) -> BTreeMap<String, (usize, usize)> {
    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (case, value) in cases.iter().zip(values) {
        let entry = counts.entry(case.family.clone()).or_insert((0, 0));
        entry.1 += 1;
        if keep(value) {
            entry.0 += 1;
        }
    }
    counts
}

fn cmd_universe(corpus: &Path) {
    let mut families: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for case in eligible_cases(corpus) {
        families.entry(case.family).or_default().push(case.id);
    }
    println!(
        "{}",
        serde_json::to_string(&families).expect("serialize universe")
    );
}

/// Render every eligible case through the shipped binary. A render that
/// fails leaves no output file, which the scorer counts as a failure — the
/// CLI user sees the same thing.
fn cmd_produce(corpus: &Path, out: &Path, binary: &Path) {
    if !binary.is_file() {
        eprintln!(
            "ratchet_gate: shipped binary {} not found",
            binary.display()
        );
        std::process::exit(1);
    }
    let cases = eligible_cases(corpus);
    // Serially, before any worker runs: parallel create_dir_all on the same
    // ancestors races.
    for case in &cases {
        if let Some(parent) = svg_path(out, case).parent() {
            std::fs::create_dir_all(parent).expect("create output directory");
        }
    }
    let rendered = map_parallel(&cases, |case| {
        let output = Command::new(binary)
            .arg("-tsvg")
            .arg(&case.source)
            .env("RUSTUML_DEBUG", GOLDEN_DEBUG)
            .output();
        match output {
            Ok(result) if result.status.success() && !result.stdout.is_empty() => {
                std::fs::write(svg_path(out, case), &result.stdout).is_ok()
            }
            _ => false,
        }
    });
    let written = rendered.iter().filter(|ok| **ok).count();
    eprintln!(
        "ratchet_gate produce: {written}/{} eligible cases rendered by {}",
        cases.len(),
        binary.display()
    );
}

fn cmd_score(corpus: &Path, reference: &Path, out: &Path) {
    let cases = eligible_cases(corpus);
    let passed = map_parallel(&cases, |case| {
        let (Ok(actual), Ok(expected)) = (
            std::fs::read_to_string(svg_path(out, case)),
            std::fs::read_to_string(svg_path(reference, case)),
        ) else {
            return false;
        };
        compare_svg_strict(&expected, &actual).is_ok_and(|cmp| cmp.is_match())
    });
    let counts = by_family(&cases, &passed, |ok| *ok);
    let metrics: BTreeMap<&String, BTreeMap<&str, usize>> = counts
        .iter()
        .map(|(family, (pass, eligible))| {
            (
                family,
                BTreeMap::from([("pass", *pass), ("eligible", *eligible)]),
            )
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string(&metrics).expect("serialize metrics")
    );
}

fn flag(args: &[String], name: &str) -> Option<PathBuf> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
}

fn require(args: &[String], name: &str) -> PathBuf {
    flag(args, name).unwrap_or_else(|| {
        eprintln!("ratchet_gate: missing {name}\n{USAGE}");
        std::process::exit(2);
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("universe") => cmd_universe(&require(&args, "--corpus")),
        Some("produce") => cmd_produce(
            &require(&args, "--corpus"),
            &require(&args, "--out"),
            &require(&args, "--bin"),
        ),
        Some("score") => cmd_score(
            &require(&args, "--corpus"),
            &require(&args, "--ref"),
            &require(&args, "--out"),
        ),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
}
