// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Render a single golden PUML using its golden SVG as oracle.
//! Usage: cargo run --release -p rustuml-oracle --example render_with_oracle -- <path/to/file.puml>

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let puml = std::fs::read_to_string(&args[1]).unwrap();
    let svg_path = std::path::PathBuf::from(&args[1]).with_extension("svg");
    let golden = std::fs::read_to_string(&svg_path).unwrap();
    let oracle = rustuml_oracle::extract::extract_oracle_layout(&golden).unwrap();
    let diagram = rustuml_parser::parse::parse_auto_with_base(&puml, None).unwrap();
    let rust = rustuml_render::render_svg_with_oracle(&diagram, Some(&oracle));
    println!("{}", rust);
}
