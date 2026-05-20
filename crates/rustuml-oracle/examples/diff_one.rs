// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Render a single .puml and diff its SVG against the golden .svg.
//!
//! Usage: cargo run --release -p rustuml-oracle --example diff_one -- <path.puml>

use rustuml_oracle::{compare, extract};
use std::path::PathBuf;

fn main() {
    // Pin date as the golden_pairs test does.
    unsafe {
        std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100");
    }

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: diff_one <path.puml>");
        std::process::exit(2);
    }
    let puml_path = PathBuf::from(&args[1]);
    let svg_path = puml_path.with_extension("svg");

    let source = std::fs::read_to_string(&puml_path).expect("read puml");
    let golden_svg = std::fs::read_to_string(&svg_path).expect("read svg");

    let oracle_layout = if golden_svg.contains(r#"data-diagram-type="CLASS""#)
        || golden_svg.contains(r#"data-diagram-type="STATE""#)
        || golden_svg.contains(r#"data-diagram-type="DESCRIPTION""#)
    {
        extract::extract_oracle_layout(&golden_svg)
    } else {
        None
    };

    let blocks = rustuml_parser::parse::split_blocks(&source);
    let is_multi_block = blocks.len() > 1;
    let rust_svg = if is_multi_block {
        let block0 = rustuml_parser::parse::parse_block(&source, 0).expect("parse block 0");
        rustuml_render::render_svg_with_oracle(&block0, oracle_layout.as_ref())
    } else {
        let diagram =
            rustuml_parser::parse::parse_auto_with_base(&source, None).expect("parse auto");
        rustuml_render::render_svg_with_oracle(&diagram, oracle_layout.as_ref())
    };

    // Allow dumping rendered SVG by env var.
    if let Ok(path) = std::env::var("DIFF_ONE_DUMP")
        && !path.is_empty()
    {
        std::fs::write(&path, &rust_svg).expect("write dump");
        eprintln!("rendered SVG dumped to {path}");
    }

    match compare::compare_svg_strict(&golden_svg, &rust_svg) {
        Ok(cmp) => {
            if cmp.is_match() {
                println!("MATCH: {}", puml_path.display());
            } else {
                println!("DIFF: {}", puml_path.display());
                println!("{cmp}");
                println!("(total differences: {})", cmp.differences.len());
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("compare error: {e}");
            std::process::exit(2);
        }
    }
}
