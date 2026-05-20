// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Print full strict-XML diff for a single golden puml file.
//! Usage: cargo run --release -p rustuml-oracle --example diff_one -- <path/to/file.puml>

fn main() {
    // SAFETY: single-threaded program.
    unsafe { std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100") };

    let args: Vec<_> = std::env::args().collect();
    let puml_path = std::path::PathBuf::from(&args[1]);
    let puml = std::fs::read_to_string(&puml_path).unwrap();
    let svg_path = puml_path.with_extension("svg");
    let golden = std::fs::read_to_string(&svg_path).unwrap();

    let blocks = rustuml_parser::parse::split_blocks(&puml);
    let is_multi_block = blocks.len() > 1;
    let oracle = if golden.contains(r#"data-diagram-type="CLASS""#)
        || golden.contains(r#"data-diagram-type="STATE""#)
        || golden.contains(r#"data-diagram-type="DESCRIPTION""#)
    {
        rustuml_oracle::extract::extract_oracle_layout(&golden)
    } else {
        None
    };

    let rust = if is_multi_block {
        let b0 = rustuml_parser::parse::parse_block(&puml, 0).unwrap();
        rustuml_render::render_svg_with_oracle(&b0, oracle.as_ref())
    } else {
        let d = rustuml_parser::parse::parse_auto_with_base(&puml, None).unwrap();
        rustuml_render::render_svg_with_oracle(&d, oracle.as_ref())
    };

    let cmp = rustuml_oracle::compare::compare_svg_strict(&golden, &rust).unwrap();
    if cmp.is_match() {
        println!("MATCH");
        return;
    }
    println!("{cmp}");

    // Also write the rendered svg for inspection.
    if let Ok(out) = std::env::var("DIFF_OUT_SVG") {
        std::fs::write(&out, &rust).unwrap();
        eprintln!("wrote rust render to {out}");
    }
}
