// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
// Evidence-only invocation of the unchanged standing-harness algorithm on new inputs.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let source = std::fs::read_to_string(&args[1]).unwrap();
    let golden = std::fs::read_to_string(&args[2]).unwrap();
    if rustuml_oracle::harness::golden_has_syntax_error(&golden) {
        eprintln!("checker preserved a Java error page instead of valid output");
        std::process::exit(2);
    }
    let blocks = rustuml_parser::parse::split_blocks(&source);
    let diagram = if blocks.len() > 1 {
        rustuml_parser::parse::parse_block(&source, 0)
    } else {
        rustuml_parser::parse::parse_auto_with_base(&source, None)
    };
    let diagram = match diagram {
        Ok(diagram) => diagram,
        Err(error) => { eprintln!("parse failed: {error}"); std::process::exit(3); }
    };
    let svg = rustuml_render::render_svg(&diagram);
    match rustuml_oracle::compare::compare_svg_strict(&golden, &svg) {
        Ok(result) => { println!("{result}"); std::process::exit(if result.is_match() { 0 } else { 1 }); }
        Err(error) => { eprintln!("compare failed: {error}"); std::process::exit(3); }
    }
}
