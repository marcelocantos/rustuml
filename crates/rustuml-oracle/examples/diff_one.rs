// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Show full diff for one golden pair.
//! Usage: cargo run --release -p rustuml-oracle --example diff_one -- <puml_path>

use rustuml_oracle::compare;
use std::path::PathBuf;

fn main() {
    unsafe { std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100") };
    let args: Vec<_> = std::env::args().collect();
    let puml_path = PathBuf::from(&args[1]);
    let source = std::fs::read_to_string(&puml_path).unwrap();
    let golden = std::fs::read_to_string(puml_path.with_extension("svg")).unwrap();

    let oracle = if golden.contains(r#"data-diagram-type="CLASS""#)
        || golden.contains(r#"data-diagram-type="STATE""#)
        || golden.contains(r#"data-diagram-type="DESCRIPTION""#)
    {
        rustuml_oracle::extract::extract_oracle_layout(&golden)
    } else {
        None
    };

    let blocks = rustuml_parser::parse::split_blocks(&source);
    let rust_svg = if blocks.len() > 1 {
        let b = rustuml_parser::parse::parse_block(&source, 0).unwrap();
        rustuml_render::render_svg_with_oracle(&b, oracle.as_ref())
    } else {
        let d = rustuml_parser::parse::parse_auto_with_base(&source, None).unwrap();
        rustuml_render::render_svg_with_oracle(&d, oracle.as_ref())
    };

    if args.iter().any(|a| a == "--print-rust") {
        println!("{}", rust_svg);
        return;
    }
    if args.iter().any(|a| a == "--print-golden") {
        println!("{}", golden);
        return;
    }

    let cmp = compare::compare_svg_strict(&golden, &rust_svg).unwrap();
    println!("{}", cmp);
}
