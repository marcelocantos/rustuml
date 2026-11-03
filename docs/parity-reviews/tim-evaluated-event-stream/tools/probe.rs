// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args[1].as_str() {
        "expand" | "lines" => {
            let source = std::fs::read_to_string(&args[2]).unwrap();
            let result = rustuml_parser::preprocess::preprocess_full(&source, None);
            if args[1] == "expand" { print!("{}", result.uml_source); }
            else { print!("{}", result.lines.join("\n")); }
        }
        "compare" => {
            let expected = std::fs::read_to_string(&args[2]).unwrap();
            if rustuml_oracle::harness::golden_has_syntax_error(&expected) {
                eprintln!("invalid Java reference"); std::process::exit(2);
            }
            let actual = std::fs::read_to_string(&args[3]).unwrap();
            match rustuml_oracle::compare::compare_svg_strict(&expected, &actual) {
                Ok(result) => { println!("{result}"); std::process::exit(if result.is_match() {0} else {1}); }
                Err(error) => { eprintln!("{error}"); std::process::exit(3); }
            }
        }
        _ => panic!("expand source | lines source | compare expected actual"),
    }
}
