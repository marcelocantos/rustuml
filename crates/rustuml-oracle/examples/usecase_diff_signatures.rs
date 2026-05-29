// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Print first few diff signatures for usecase golden failures.
//! Usage: cargo run --release -p rustuml-oracle --example usecase_diff_signatures -- <max>

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

fn main() {
    unsafe { std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100") };
    std::panic::set_hook(Box::new(|_| {}));
    let args: Vec<_> = std::env::args().collect();
    let max: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);

    let root = golden_dir();
    let bucket = root.join("usecase");
    let mut pumls = Vec::new();
    collect_pumls(&bucket, &mut pumls);
    pumls.sort();

    let mut sig_count: HashMap<String, usize> = HashMap::new();
    let mut sig_examples: HashMap<String, Vec<String>> = HashMap::new();
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
        total += 1;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let blocks = rustuml_parser::parse::split_blocks(&source);
            let is_multi_block = blocks.len() > 1;
            let oracle = if golden.contains(r#"data-diagram-type="CLASS""#)
                || golden.contains(r#"data-diagram-type="STATE""#)
                || golden.contains(r#"data-diagram-type="DESCRIPTION""#)
            {
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
        match result {
            Ok(Ok(cmp)) => {
                if !cmp.is_match() {
                    failed += 1;
                    let n_diffs = cmp.differences.len();
                    let first = &cmp.differences[0];
                    let sig = first_diff_signature(first);
                    let bucket = sig.split('{').next().unwrap_or(&sig).trim().to_string();
                    *sig_count.entry(bucket.clone()).or_default() += 1;
                    sig_examples
                        .entry(bucket.clone())
                        .or_default()
                        .push(format!("{name} ({n_diffs}d): {sig}"));
                    if shown < max {
                        println!("FAIL {name} ({n_diffs} diffs)");
                        for (i, d) in cmp.differences.iter().take(3).enumerate() {
                            println!("  #{i} {}", first_diff_signature(d));
                        }
                        shown += 1;
                    }
                }
            }
            Ok(Err(_)) | Err(_) => {}
        }
    }

    println!("\n=== SUMMARY: {failed}/{total} failures ===");
    let mut sigs: Vec<_> = sig_count.iter().collect();
    sigs.sort_by(|a, b| b.1.cmp(a.1));
    for (sig, count) in sigs.iter().take(25) {
        println!("{:5}  {}", count, sig);
        if let Some(ex) = sig_examples.get(*sig) {
            for e in ex.iter().take(2) {
                println!("       - {e}");
            }
        }
    }
}
