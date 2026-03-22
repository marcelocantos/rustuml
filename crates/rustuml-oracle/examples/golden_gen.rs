// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Generate golden SVG files from all test matrix cases.
//!
//! Usage: cargo run -p rustuml-oracle --example golden_gen -- <server_url> <output_dir>

use std::path::PathBuf;

use rustuml_oracle::matrix::{
    MatrixCase, activity, class, component, deployment, sequence, state, usecase,
};
use rustuml_oracle::runner;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let url = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("http://127.0.0.1:8787");
    let output_dir = PathBuf::from(
        args.get(2)
            .map(|s| s.as_str())
            .unwrap_or("test-fixtures/golden"),
    );

    // Set PLANTUML_URL for the runner.
    // SAFETY: Single-threaded context at program startup.
    unsafe { std::env::set_var("PLANTUML_URL", url) };

    // Collect all matrix cases.
    let mut cases: Vec<MatrixCase> = Vec::new();
    cases.extend(sequence::quick_cases());
    cases.extend(sequence::medium_cases());
    cases.extend(sequence::large_cases());
    cases.extend(sequence::edge_cases());
    cases.extend(class::quick_cases());
    cases.extend(class::medium_cases());
    cases.extend(class::edge_cases());
    cases.extend(state::edge_cases());
    cases.extend(activity::edge_cases());
    cases.extend(deployment::edge_cases());
    cases.extend(component::matrix_cases());
    cases.extend(component::edge_cases());
    cases.extend(usecase::matrix_cases());
    cases.extend(usecase::edge_cases());

    println!("Generating {} golden files...", cases.len());

    let mut ok = 0;
    let mut skip = 0;
    let mut fail = 0;

    for (idx, case) in cases.iter().enumerate() {
        // Brief pause every 10 requests to avoid overwhelming the server.
        if idx > 0 && idx % 10 == 0 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let safe_name = case.name.replace('/', "_").replace(' ', "_");
        let dir = output_dir.join("matrix");
        std::fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join(format!("{safe_name}.svg"));

        match runner::render_svg(&case.source) {
            Ok(svg) if svg.starts_with("<svg") => {
                std::fs::write(&path, &svg).expect("write");
                ok += 1;
            }
            Ok(_) => {
                skip += 1;
            }
            Err(e) => {
                eprintln!("  FAIL: {} — {e}", case.name);
                fail += 1;
            }
        }
    }

    println!("Done: {ok} generated, {skip} skipped, {fail} failed");
    println!("Golden files in: {}", output_dir.display());
}
