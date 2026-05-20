// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Render one golden PUML and compare against its golden SVG (strict XML).
//! Usage: cargo run --release -p rustuml-oracle --example diff_one -- <path/to/file.puml>

use rustuml_oracle::{compare, extract};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: diff_one <path/to/file.puml>");
        std::process::exit(2);
    }
    // Pin date — matches golden_pairs test config.
    // SAFETY: single-threaded program; set before any other code runs.
    unsafe { std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100") };

    let puml_path = std::path::PathBuf::from(&args[1]);
    let puml = std::fs::read_to_string(&puml_path).expect("read puml");
    let svg_path = puml_path.with_extension("svg");
    let golden = std::fs::read_to_string(&svg_path).expect("read svg");

    let oracle = if golden.contains(r#"data-diagram-type="CLASS""#)
        || golden.contains(r#"data-diagram-type="STATE""#)
        || golden.contains(r#"data-diagram-type="DESCRIPTION""#)
    {
        extract::extract_oracle_layout(&golden)
    } else {
        None
    };

    let blocks = rustuml_parser::parse::split_blocks(&puml);
    let rust_svg = if blocks.len() > 1 {
        let block0 = rustuml_parser::parse::parse_block(&puml, 0).expect("parse block");
        rustuml_render::render_svg_with_oracle(&block0, oracle.as_ref())
    } else {
        let diagram = rustuml_parser::parse::parse_auto_with_base(&puml, None).expect("parse");
        rustuml_render::render_svg_with_oracle(&diagram, oracle.as_ref())
    };

    let cmp = compare::compare_svg_strict(&golden, &rust_svg).expect("compare");
    if cmp.is_match() {
        println!("MATCH: {}", args[1]);
        return;
    }
    println!("DIFF: {} ({} differences)", args[1], cmp.differences.len());
    println!("{}", cmp);

    // Also write our SVG for inspection if requested via env var.
    if std::env::var_os("DUMP_RUST_SVG").is_some() {
        let out = puml_path.with_extension("rust.svg");
        std::fs::write(&out, &rust_svg).ok();
        println!("wrote {}", out.display());
    }
}
