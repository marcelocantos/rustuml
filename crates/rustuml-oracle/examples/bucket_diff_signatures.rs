// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Aggregate first-diff signatures across a golden bucket, mirroring the
//! golden_pairs harness oracle-extraction logic exactly.
//! Usage: cargo run --release -p rustuml-oracle --example bucket_diff_signatures -- <bucket> [max]

use rustuml_oracle::compare;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-diagrams/golden")
}

fn collect_pumls(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_pumls(&path, out);
        } else if path.extension().is_some_and(|e| e == "puml")
            && path.with_extension("svg").exists()
        {
            out.push(path);
        }
    }
}

fn first_diff_signature(diff: &compare::Difference) -> String {
    match diff {
        compare::Difference::ElementCount { expected, actual } => {
            format!("ElementCount expected={expected} actual={actual}")
        }
        compare::Difference::TagMismatch {
            expected, actual, ..
        } => {
            format!("TagMismatch <{expected}> vs <{actual}>")
        }
        compare::Difference::AttrMismatch {
            tag,
            expected_attrs,
            actual_attrs,
            ..
        } => {
            let exp_map: HashMap<&str, &str> = expected_attrs
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let act_map: HashMap<&str, &str> = actual_attrs
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let mut diff_keys: Vec<String> = Vec::new();
            for (k, v) in &exp_map {
                match act_map.get(k) {
                    Some(av) if av == v => {}
                    Some(av) => diff_keys.push(format!("{k}={v:?}!={av:?}")),
                    None => diff_keys.push(format!("{k}=missing")),
                }
            }
            for (k, _) in &act_map {
                if !exp_map.contains_key(k) {
                    diff_keys.push(format!("{k}=extra"));
                }
            }
            format!("AttrMismatch <{tag}> {{ {} }}", diff_keys.join(", "))
        }
        compare::Difference::TextMismatch {
            tag,
            expected,
            actual,
            ..
        } => {
            format!("TextMismatch <{tag}> {expected:?} vs {actual:?}")
        }
        compare::Difference::DepthMismatch {
            tag,
            expected,
            actual,
            ..
        } => {
            format!("DepthMismatch <{tag}> {expected} vs {actual}")
        }
    }
}

/// Collapse a signature to a coarse bucket key (drop the `{ ... }` detail and
/// any quoted literals) so near-identical diffs aggregate.
fn coarse(sig: &str) -> String {
    sig.split('{').next().unwrap_or(sig).trim().to_string()
}

fn main() {
    unsafe { std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100") };
    std::panic::set_hook(Box::new(|_| {}));
    let args: Vec<_> = std::env::args().collect();
    let bucket_name = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "activity".to_string());
    let max: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    let root = golden_dir();
    let bucket = root.join(&bucket_name);
    let mut pumls = Vec::new();
    collect_pumls(&bucket, &mut pumls);
    pumls.sort();

    let mut sig_count: HashMap<String, usize> = HashMap::new();
    let mut sig_examples: HashMap<String, Vec<String>> = HashMap::new();
    // Detailed AttrMismatch attribute-key frequency, to find shared root causes.
    let mut attr_key_count: HashMap<String, usize> = HashMap::new();
    let mut total = 0usize;
    let mut failed = 0usize;
    let mut shown = 0usize;

    for puml in &pumls {
        let name = puml
            .strip_prefix(&root)
            .unwrap()
            .with_extension("")
            .to_string_lossy()
            .to_string();
        let Ok(source) = std::fs::read_to_string(puml) else {
            continue;
        };
        let Ok(golden) = std::fs::read_to_string(puml.with_extension("svg")) else {
            continue;
        };
        if golden.contains("Syntax Error") || golden.contains("error has occured") {
            continue;
        }
        if source.lines().any(|l| l.trim().starts_with("@startditaa")) {
            continue;
        }
        total += 1;
        // Mirror run_one: oracle iff golden looks PlantUML-emitted.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let blocks = rustuml_parser::parse::split_blocks(&source);
            let is_multi_block = blocks.len() > 1;
            let oracle = if golden.contains("<?plantuml ") {
                rustuml_oracle::extract::extract_oracle_layout(&golden)
            } else {
                None
            };
            let rust_svg = if is_multi_block {
                let b =
                    rustuml_parser::parse::parse_block(&source, 0).map_err(|e| format!("{e}"))?;
                rustuml_render::render_svg_with_oracle(&b, oracle.as_ref())
            } else {
                let d = rustuml_parser::parse::parse_auto_with_base(&source, None)
                    .map_err(|e| format!("{e}"))?;
                rustuml_render::render_svg_with_oracle(&d, oracle.as_ref())
            };
            let cmp =
                compare::compare_svg_strict(&golden, &rust_svg).map_err(|e| format!("cmp: {e}"))?;
            Ok::<_, String>(cmp)
        }));
        if let Ok(Ok(cmp)) = result
            && !cmp.is_match()
        {
            failed += 1;
            let n_diffs = cmp.differences.len();
            let first = &cmp.differences[0];
            let sig = first_diff_signature(first);
            let key = coarse(&sig);
            *sig_count.entry(key.clone()).or_default() += 1;
            sig_examples
                .entry(key.clone())
                .or_default()
                .push(format!("{name} ({n_diffs}d): {sig}"));
            // Tally which attribute keys differ, across ALL diffs in this pair.
            for d in &cmp.differences {
                if let compare::Difference::AttrMismatch {
                    tag,
                    expected_attrs,
                    actual_attrs,
                    ..
                } = d
                {
                    let act: HashMap<&str, &str> = actual_attrs
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .collect();
                    for (k, v) in expected_attrs {
                        let differs = act.get(k.as_str()).map(|av| av != v).unwrap_or(true);
                        if differs {
                            *attr_key_count.entry(format!("{tag}@{k}")).or_default() += 1;
                        }
                    }
                }
            }
            if max > 0 && shown < max {
                println!("FAIL {name} ({n_diffs} diffs)");
                for (i, d) in cmp.differences.iter().take(5).enumerate() {
                    println!("  #{i} {}", first_diff_signature(d));
                }
                shown += 1;
            }
        }
    }

    println!("\n=== {bucket_name}: {failed}/{total} failures ===");
    println!("\n-- first-diff signature buckets --");
    let mut sigs: Vec<_> = sig_count.iter().collect();
    sigs.sort_by(|a, b| b.1.cmp(a.1));
    for (sig, count) in sigs.iter().take(25) {
        println!("{count:5}  {sig}");
        if let Some(ex) = sig_examples.get(*sig) {
            for e in ex.iter().take(2) {
                println!("       - {e}");
            }
        }
    }
    println!("\n-- attr-key diff frequency (all diffs, all pairs) --");
    let mut keys: Vec<_> = attr_key_count.iter().collect();
    keys.sort_by(|a, b| b.1.cmp(a.1));
    for (k, count) in keys.iter().take(30) {
        println!("{count:5}  {k}");
    }
}
