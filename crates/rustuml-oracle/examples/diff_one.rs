// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Diff a single golden pair (rust vs java).
//! Usage: cargo run --release -p rustuml-oracle --example diff_one -- <path/to/file.puml>
//!
//! Prints the structural diff between Java PlantUML golden SVG and the
//! Rust renderer's output for the given .puml. Limit shown differences
//! with DIFF_LIMIT=N env var (default 30).

fn main() {
    // SAFETY: set before any threads spawned.
    unsafe {
        std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100");
    }

    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: diff_one <path/to/file.puml>");
        std::process::exit(2);
    }
    let puml_path = &args[1];
    let svg_path = std::path::PathBuf::from(puml_path).with_extension("svg");

    let source = std::fs::read_to_string(puml_path).expect("read puml");
    let golden = std::fs::read_to_string(&svg_path).expect("read golden svg");

    let oracle = if golden.contains(r#"data-diagram-type="STATE""#)
        || golden.contains(r#"data-diagram-type="CLASS""#)
        || golden.contains(r#"data-diagram-type="DESCRIPTION""#)
    {
        rustuml_oracle::extract::extract_oracle_layout(&golden)
    } else {
        None
    };

    let diagram = match rustuml_parser::parse::parse_auto_with_base(&source, None) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("PARSE ERROR: {e}");
            std::process::exit(1);
        }
    };
    let rust = rustuml_render::render_svg_with_oracle(&diagram, oracle.as_ref());

    let cmp = match rustuml_oracle::compare::compare_svg_strict(&golden, &rust) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("COMPARE ERROR: {e}");
            std::process::exit(1);
        }
    };

    let limit: usize = std::env::var("DIFF_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    if cmp.is_match() {
        println!("MATCH ({} elements)", "ok");
        return;
    }

    println!("FAIL: {} total differences", cmp.differences.len());
    for d in cmp.differences.iter().take(limit) {
        println!("  {d:?}");
    }
    if cmp.differences.len() > limit {
        println!(
            "  ... ({} more, set DIFF_LIMIT to see)",
            cmp.differences.len() - limit
        );
    }
}
