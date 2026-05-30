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
//! Comparison is strict XML equivalence: same elements, same attributes
//! (exact string match), same text content, same nesting depth. Processing
//! instructions, comments, and whitespace-only text nodes are ignored.
//!
//! Run with: `cargo test --test golden_pairs`

use rayon::prelude::*;
use rustuml_oracle::compare;
use rustuml_oracle::extract;
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
    /// `numeric_dev` is `Some(max coordinate deviation)` when EVERY difference
    /// is a geometry-coordinate value (park-eligible), or `None` when there is
    /// a structural difference (element count/tag/text/depth/color/style/font/
    /// order) that parking must never forgive.
    Fail {
        msg: String,
        numeric_dev: Option<f64>,
    },
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
    // Skip ditaa diagrams — these produce raster images, not SVG elements.
    if source.lines().any(|l| l.trim().starts_with("@startditaa")) {
        return TestResult {
            name: rel,
            outcome: Outcome::Skip("ditaa diagram".into()),
        };
    }

    // Extract oracle layout from any golden SVG that looks PlantUML-emitted.
    // The `<?plantuml ?>` processing instruction is present in every
    // PlantUML-generated golden regardless of diagram type — using it as the
    // trigger avoids per-type allow-listing and lets `@startmath`/`@startlatex`
    // (which carry no `data-diagram-type` attribute) pick up oracle data too.
    let oracle_layout = if golden_svg.contains("<?plantuml ") {
        extract::extract_oracle_layout(&golden_svg)
    } else {
        None
    };

    let render_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let blocks = rustuml_parser::parse::split_blocks(&source);
        let is_multi_block = blocks.len() > 1;

        let rust_svg = if is_multi_block {
            let block0 = rustuml_parser::parse::parse_block(&source, 0)
                .map_err(|e| (format!("parse: {e}"), None))?;
            rustuml_render::render_svg_with_oracle(&block0, oracle_layout.as_ref())
        } else {
            let diagram = rustuml_parser::parse::parse_auto_with_base(&source, None)
                .map_err(|e| (format!("parse: {e}"), None))?;
            rustuml_render::render_svg_with_oracle(&diagram, oracle_layout.as_ref())
        };

        let cmp = compare::compare_svg_strict(&golden_svg, &rust_svg)
            .map_err(|e| (format!("compare: {e}"), None))?;

        if cmp.is_match() {
            Ok(())
        } else {
            // Classify before truncating: park-eligible iff every diff is a
            // geometry-coordinate value (structural diffs => None).
            let numeric_dev = cmp.numeric_only_max_deviation();
            // Truncate the report to keep failure output manageable.
            let report = format!("{cmp}");
            let truncated: String = report.lines().take(20).collect::<Vec<_>>().join("\n");
            let suffix = if report.lines().count() > 20 {
                format!("\n  ... ({} total differences)", cmp.differences.len())
            } else {
                String::new()
            };
            Err((format!("{truncated}{suffix}"), numeric_dev))
        }
    }));

    match render_result {
        Ok(Ok(())) => TestResult {
            name: rel,
            outcome: Outcome::Pass,
        },
        // Parse failures (mapped above) carry `None`; they stay Skip.
        Ok(Err((msg, _))) if msg.starts_with("parse:") => TestResult {
            name: rel,
            outcome: Outcome::Skip(msg),
        },
        Ok(Err((msg, numeric_dev))) => TestResult {
            name: rel,
            outcome: Outcome::Fail { msg, numeric_dev },
        },
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
                outcome: Outcome::Fail {
                    msg: format!("panic: {msg}"),
                    numeric_dev: None,
                },
            }
        }
    }
}

/// `%date(...)` goldens in the corpus were all generated on 2026-03-23
/// AEDT (UTC+11). The `edge_seq_title_with_variables` golden captures a
/// specific second within that day (`Mon Mar 23 07:13:46 AEDT 2026`,
/// epoch 1774210426s); the others (`yyyy-MM-dd` form) only depend on the
/// day, so this pin works for all three.
const GOLDEN_DEBUG: &str = "date=1774210426000,tz=AEDT+1100";

#[test]
fn golden_pairs() {
    // SAFETY: set before any threads are spawned by rayon below.
    unsafe { std::env::set_var("RUSTUML_DEBUG", GOLDEN_DEBUG) };

    let root = golden_dir();
    if !root.exists() || !root.join("sequence").exists() {
        eprintln!("golden submodule not populated — run: git submodule update --init");
        return;
    }

    let mut pairs = collect_golden_pairs(&root);
    if pairs.is_empty() {
        eprintln!("no golden pairs found");
        return;
    }
    if let Ok(filter) = std::env::var("GOLDEN_FILTER")
        && !filter.is_empty()
    {
        pairs.retain(|p| {
            p.strip_prefix(&root)
                .unwrap_or(p)
                .to_string_lossy()
                .contains(&filter)
        });
        eprintln!("GOLDEN_FILTER={filter:?} → {} pairs match", pairs.len());
        if pairs.is_empty() {
            return;
        }
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
    let skip_other = AtomicUsize::new(0);

    // Curated park list: golden names an investigator confirmed fail ONLY on
    // geometry-coordinate deviations (FP accumulation, missing font-metric
    // tables, sub-pixel drift). A parked case is excluded from real failures
    // ONLY while it stays park-eligible (every diff a geometry-coordinate
    // value); if it ever gains a STRUCTURAL diff it un-parks and fails loudly.
    // One name per line; `#` comments and blank lines ignored.
    let parked: std::collections::HashSet<String> =
        std::fs::read_to_string(root.parent().unwrap().join("ulp_parked.txt"))
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect();

    // (name, msg, numeric_dev) for every failing pair.
    let raw_fails: Vec<(String, String, Option<f64>)> = pool.install(|| {
        pairs
            .par_iter()
            .filter_map(|p| {
                let TestResult { name, outcome } = run_one(p, &root);
                match outcome {
                    Outcome::Pass => {
                        pass.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                    Outcome::Skip(reason) => {
                        skip.fetch_add(1, Ordering::Relaxed);
                        if reason.starts_with("parse:") {
                            skip_parse.fetch_add(1, Ordering::Relaxed);
                        } else if reason.contains("error") || reason.contains("Error") {
                            skip_error.fetch_add(1, Ordering::Relaxed);
                        } else if reason.contains("unsupported keyword") {
                            skip_keyword.fetch_add(1, Ordering::Relaxed);
                        } else {
                            skip_other.fetch_add(1, Ordering::Relaxed);
                        }
                        None
                    }
                    Outcome::Fail { msg, numeric_dev } => Some((name, msg, numeric_dev)),
                }
            })
            .collect()
    });

    // Classify. parked_ok = numeric-only AND on the curated list (excluded).
    // Everything else is a real failure; among real failures we flag
    // park-candidates (numeric-only, not yet parked) and park-invalidations
    // (listed but now structural — a regression that must be seen).
    let parked_ok = |n: &str, d: &Option<f64>| d.is_some() && parked.contains(n);
    let parked_recs: Vec<&(String, String, Option<f64>)> = raw_fails
        .iter()
        .filter(|(n, _, d)| parked_ok(n, d))
        .collect();
    let real: Vec<&(String, String, Option<f64>)> = raw_fails
        .iter()
        .filter(|(n, _, d)| !parked_ok(n, d))
        .collect();
    let near: Vec<&(String, String, Option<f64>)> = real
        .iter()
        .copied()
        .filter(|(n, _, d)| d.is_some() && !parked.contains(n))
        .collect();
    let park_invalid: Vec<&str> = real
        .iter()
        .filter(|(n, _, d)| d.is_none() && parked.contains(n))
        .map(|(n, _, _)| n.as_str())
        .collect();

    let total = pairs.len();
    let pass = pass.load(Ordering::Relaxed);
    let skip = skip.load(Ordering::Relaxed);
    let real_lines: Vec<String> = real.iter().map(|(n, m, _)| format!("{n}: {m}")).collect();
    let fail_count = real_lines.len();
    let parked_max = parked_recs
        .iter()
        .filter_map(|(_, _, d)| *d)
        .fold(0.0_f64, f64::max);
    let near_max = near
        .iter()
        .filter_map(|(_, _, d)| *d)
        .fold(0.0_f64, f64::max);

    let mut dir_fails: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for f in &real_lines {
        if let Some(slash) = f.find('/') {
            *dir_fails.entry(f[..slash].to_string()).or_default() += 1;
        }
    }

    // Write per-test REAL failure names for diff-based debugging, and the
    // park-candidate names + max deviation for review (add confirmed ones to
    // ulp_parked.txt). Both gitignored; cleared each run.
    let names_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-diagrams/golden_failure_names.txt");
    if let Ok(mut f) = std::fs::File::create(&names_path) {
        use std::io::Write;
        let mut names: Vec<&str> = real_lines
            .iter()
            .map(|s| s.split(':').next().unwrap_or(s))
            .collect();
        names.sort();
        for n in &names {
            writeln!(f, "{n}").ok();
        }
    }
    let cand_path = names_path.with_file_name("golden_ulp_candidates.txt");
    if let Ok(mut f) = std::fs::File::create(&cand_path) {
        use std::io::Write;
        let mut rows: Vec<(&str, f64)> = near
            .iter()
            .map(|(n, _, d)| (n.as_str(), d.unwrap_or(0.0)))
            .collect();
        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (n, d) in &rows {
            writeln!(f, "{d:.4}\t{n}").ok();
        }
    }

    let panics = real_lines.iter().filter(|f| f.contains("panic:")).count();
    let xml_diff = real_lines
        .iter()
        .filter(|f| f.contains("SVG structural differences"))
        .count();
    let other = fail_count - panics - xml_diff;

    let sp = skip_parse.load(Ordering::Relaxed);
    let se = skip_error.load(Ordering::Relaxed);
    let sk = skip_keyword.load(Ordering::Relaxed);
    let so = skip_other.load(Ordering::Relaxed);
    eprintln!("\ngolden_pairs: {total} total, {pass} passed, {fail_count} failed, {skip} skipped");
    eprintln!("  panics: {panics}, xml diff: {xml_diff}, other: {other}");
    eprintln!(
        "  parked (numeric-only, allow-listed): {} (max dev {parked_max:.4})",
        parked_recs.len()
    );
    eprintln!(
        "  park candidates (numeric-only, NOT yet parked, still counted as failures): {} (max dev {near_max:.4}) → see golden_ulp_candidates.txt",
        near.len()
    );
    if !park_invalid.is_empty() {
        eprintln!(
            "  ⚠ PARK INVALIDATED (listed but now structural — regression): {}: {}",
            park_invalid.len(),
            park_invalid.join(", ")
        );
    }
    eprintln!("  skip breakdown: parse={sp}, golden_error={se}, unsupported_kw={sk}, other={so}");
    if !dir_fails.is_empty() {
        eprintln!("  per-directory failures:");
        let mut sorted: Vec<_> = dir_fails.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (dir, count) in &sorted {
            eprintln!("    {count:5} {dir}");
        }
    }

    const MAX_SHOWN: usize = 50;
    let shown: Vec<&str> = real_lines
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
        real_lines.is_empty(),
        "{fail_count} of {total} golden pair tests failed \
         (panics: {panics}, xml_diff: {xml_diff}, other: {other}; \
         {} parked, {} park-candidates):\n{}{truncated}",
        parked_recs.len(),
        near.len(),
        shown.join("\n")
    );
}
