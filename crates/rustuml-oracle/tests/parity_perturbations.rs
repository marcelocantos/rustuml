// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Held-out maker/checker perturbations for T14 parity changes.

use rustuml_oracle::compare;
use std::path::PathBuf;

mod parity_evidence;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn held_out_perturbations_match_java_without_oracle_layout() {
    let root = repo_root();
    let heldouts = match parity_evidence::accepted_java_success_heldouts(&root) {
        Ok(heldouts) => heldouts,
        Err(violations) => {
            panic!(
                "invalid T14 parity review evidence:\n  {}",
                violations.join("\n  ")
            );
        }
    };

    let mut failures = Vec::new();
    for heldout in heldouts {
        let source_path = root.join(&heldout.source);
        let golden_path = root.join(&heldout.golden);
        let relative = &heldout.source;
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
