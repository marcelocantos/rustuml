// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

// Link against the tested revision's unchanged rustuml_oracle rlib. This
// adapter compares actual CLI output; it does not reimplement the comparator.
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let golden = std::fs::read_to_string(&args[1]).expect("read Java SVG");
    let actual = std::fs::read_to_string(&args[2]).expect("read CLI SVG");
    if golden.trim().is_empty() || rustuml_oracle::harness::golden_has_syntax_error(&golden) {
        eprintln!("invalid reference (retained negative evidence)");
        std::process::exit(2);
    }
    match rustuml_oracle::compare::compare_svg_strict(&golden, &actual) {
        Ok(result) => {
            println!("{result}");
            if !result.is_match() {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("SVG parse/comparison error: {error}");
            std::process::exit(3);
        }
    }
}
