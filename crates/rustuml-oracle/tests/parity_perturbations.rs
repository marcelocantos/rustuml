// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Held-out maker/checker perturbations for T14 parity changes.

use rustuml_oracle::compare;
use rustuml_oracle::harness::golden_has_syntax_error;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_sources(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "puml")
        {
            out.push(path);
        }
    }
}

#[test]
fn held_out_perturbations_match_java_without_oracle_layout() {
    let root = repo_root().join("test-diagrams/perturbations");
    let mut sources = Vec::new();
    collect_sources(&root, &mut sources);
    sources.sort();

    let mut failures = Vec::new();
    for source_path in sources {
        let golden_path = source_path.with_extension("svg");
        let relative = source_path.strip_prefix(repo_root()).unwrap();
        let source = match std::fs::read_to_string(&source_path) {
            Ok(source) => source,
            Err(error) => {
                failures.push(format!("{}: {error}", relative.display()));
                continue;
            }
        };
        let golden = match std::fs::read_to_string(&golden_path) {
            Ok(golden) => golden,
            Err(error) => {
                failures.push(format!("{}: missing Java SVG: {error}", relative.display()));
                continue;
            }
        };
        if golden_has_syntax_error(&golden) {
            failures.push(format!(
                "{}: checker preserved a Java error page instead of valid output",
                relative.display()
            ));
            continue;
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let blocks = rustuml_parser::parse::split_blocks(&source);
            let rust_svg = if blocks.len() > 1 {
                let block = rustuml_parser::parse::parse_block(&source, 0)
                    .map_err(|error| format!("parse failed: {error}"))?;
                rustuml_render::render_svg(&block)
            } else {
                let diagram = rustuml_parser::parse::parse_auto_with_base(&source, None)
                    .map_err(|error| format!("parse failed: {error}"))?;
                rustuml_render::render_svg(&diagram)
            };
            let comparison = compare::compare_svg_strict(&golden, &rust_svg)
                .map_err(|error| format!("compare failed: {error}"))?;
            Ok::<_, String>(comparison)
        }));
        match result {
            Ok(Ok(comparison)) if comparison.is_match() => {}
            Ok(Ok(comparison)) => failures.push(format!("{}: {comparison}", relative.display())),
            Ok(Err(error)) => failures.push(format!("{}: {error}", relative.display())),
            Err(_) => failures.push(format!("{}: renderer panicked", relative.display())),
        }
    }

    assert!(
        failures.is_empty(),
        "held-out no-oracle perturbations diverged from Java:\n  {}",
        failures.join("\n  ")
    );
}
