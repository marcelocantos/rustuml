// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! No-oracle product-parity tier (🎯T14).
//!
//! Renders every golden through `rustuml_render::render_svg` — the exact
//! path the CLI uses, with NO layout data extracted from the golden — and
//! compares against the golden with the same strict comparator as
//! `golden_pairs.rs`. This measures what the shipped binary can actually do,
//! not what the oracle-assisted test path can do.
//!
//! Differences from the strict tier, both deliberate:
//! - No `extract_oracle_layout` injection.
//! - No `golden_source::source_for_oracle_golden` normalization (it consults
//!   the golden to decide whether to rewrite the source; the CLI cannot).
//!
//! RATCHET SEMANTICS: per-family pass counts must EXACTLY match the
//! checked-in baseline `test-diagrams/no_oracle_baseline.txt`.
//! - A count below baseline is a product regression — fix the renderer.
//! - A count above baseline is progress that must be locked in — rerun with
//!   `HARNESS_WRITE_BASELINES=1` and commit the updated baseline in the same
//!   change. A stale baseline would let later regressions hide, so
//!   improvements fail the test until recorded.
//!
//! Run with: `cargo test --test golden_no_oracle --release`

use rayon::prelude::*;
use rustuml_oracle::compare;
use rustuml_oracle::harness::{
    GOLDEN_DEBUG, collect_golden_pairs, golden_dir, golden_has_syntax_error,
    has_supported_start_keyword, is_ditaa,
};
use std::collections::BTreeMap;
use std::path::Path;

enum Outcome {
    Pass,
    Skip,
    Fail,
}

fn run_one_no_oracle(puml_path: &Path, root: &Path) -> (String, Outcome) {
    let family = puml_path
        .strip_prefix(root)
        .unwrap()
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());

    let Ok(source) = std::fs::read_to_string(puml_path) else {
        return (family, Outcome::Skip);
    };
    let Ok(golden_svg) = std::fs::read_to_string(puml_path.with_extension("svg")) else {
        return (family, Outcome::Skip);
    };
    if golden_has_syntax_error(&golden_svg)
        || !has_supported_start_keyword(&source)
        || is_ditaa(&source)
    {
        return (family, Outcome::Skip);
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let blocks = rustuml_parser::parse::split_blocks(&source);
        let rust_svg = if blocks.len() > 1 {
            let block0 = rustuml_parser::parse::parse_block(&source, 0).map_err(|_| ())?;
            rustuml_render::render_svg(&block0)
        } else {
            let diagram =
                rustuml_parser::parse::parse_auto_with_base(&source, None).map_err(|_| ())?;
            rustuml_render::render_svg(&diagram)
        };
        let cmp = compare::compare_svg_strict(&golden_svg, &rust_svg).map_err(|_| ())?;
        Ok(cmp.is_match())
    }));

    match result {
        // Parse/compare errors count as failures here, NOT skips: the CLI
        // user sees them. (The strict tier skips parse errors because its
        // question is narrower.) Panics likewise.
        Ok(Ok(true)) => (family, Outcome::Pass),
        Ok(Ok(false)) | Ok(Err(())) | Err(_) => (family, Outcome::Fail),
    }
}

fn baseline_path() -> std::path::PathBuf {
    golden_dir()
        .parent()
        .unwrap()
        .join("no_oracle_baseline.txt")
}

fn format_baseline(counts: &BTreeMap<String, (usize, usize)>) -> String {
    let mut out = String::from(
        "# No-oracle product-parity baseline (🎯T14). One line per family:\n\
         # <family> <pass> <eligible>\n\
         # Regenerate ONLY to lock in verified progress:\n\
         #   HARNESS_WRITE_BASELINES=1 cargo test --test golden_no_oracle --release\n",
    );
    for (family, (pass, eligible)) in counts {
        out.push_str(&format!("{family} {pass} {eligible}\n"));
    }
    out
}

#[test]
fn golden_no_oracle() {
    // SAFETY: set before any threads are spawned by rayon below.
    unsafe { std::env::set_var("RUSTUML_DEBUG", GOLDEN_DEBUG) };

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
    eprintln!("running {} golden pairs (no-oracle tier)...", pairs.len());

    std::panic::set_hook(Box::new(|_| {}));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .panic_handler(|_| {})
        .build()
        .expect("failed to build rayon pool");

    let results: Vec<(String, Outcome)> = pool.install(|| {
        pairs
            .par_iter()
            .map(|p| run_one_no_oracle(p, &root))
            .collect()
    });
    let _ = std::panic::take_hook();

    // family → (pass, eligible). Skips don't count toward eligible.
    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (family, outcome) in results {
        let entry = counts.entry(family).or_insert((0, 0));
        match outcome {
            Outcome::Pass => {
                entry.0 += 1;
                entry.1 += 1;
            }
            Outcome::Fail => entry.1 += 1,
            Outcome::Skip => {}
        }
    }

    let total_pass: usize = counts.values().map(|(p, _)| p).sum();
    let total_eligible: usize = counts.values().map(|(_, e)| e).sum();
    eprintln!("\ngolden_no_oracle: {total_pass}/{total_eligible} eligible pairs pass per family:");
    for (family, (pass, eligible)) in &counts {
        eprintln!("  {family:<16} {pass:>5}/{eligible}");
    }

    let path = baseline_path();
    if std::env::var("HARNESS_WRITE_BASELINES").is_ok() {
        std::fs::write(&path, format_baseline(&counts)).expect("write baseline");
        eprintln!("baseline written to {}", path.display());
        return;
    }

    let baseline_text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "missing baseline {} ({e}); bootstrap with HARNESS_WRITE_BASELINES=1",
            path.display()
        )
    });
    let mut baseline: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for line in baseline_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        let (Some(family), Some(pass), Some(eligible)) = (it.next(), it.next(), it.next()) else {
            panic!("malformed baseline line: {line}");
        };
        baseline.insert(
            family.to_string(),
            (pass.parse().unwrap(), eligible.parse().unwrap()),
        );
    }

    let mut regressions = Vec::new();
    let mut improvements = Vec::new();
    let families: std::collections::BTreeSet<&String> =
        counts.keys().chain(baseline.keys()).collect();
    for family in families {
        let (cur_pass, cur_elig) = counts.get(family.as_str()).copied().unwrap_or((0, 0));
        let (base_pass, base_elig) = baseline.get(family.as_str()).copied().unwrap_or((0, 0));
        if cur_pass < base_pass {
            regressions.push(format!(
                "{family}: pass {base_pass} -> {cur_pass} (eligible {base_elig} -> {cur_elig})"
            ));
        } else if cur_pass > base_pass || cur_elig != base_elig {
            improvements.push(format!(
                "{family}: pass {base_pass} -> {cur_pass} (eligible {base_elig} -> {cur_elig})"
            ));
        }
    }

    assert!(
        regressions.is_empty(),
        "no-oracle PRODUCT REGRESSION — the CLI path renders fewer goldens than baseline:\n  {}",
        regressions.join("\n  ")
    );
    assert!(
        improvements.is_empty(),
        "no-oracle progress not locked in (or corpus changed) — rerun with \
         HARNESS_WRITE_BASELINES=1 and commit the baseline:\n  {}",
        improvements.join("\n  ")
    );
}
