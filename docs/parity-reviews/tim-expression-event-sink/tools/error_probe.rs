// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
fn main() {
    let source = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let result = rustuml_parser::preprocess::preprocess_full(&source, None);
    if let Some(error) = result.error {
        eprintln!("{}\t{}", error.line, error.message);
        std::process::exit(2);
    }
    print!("{}", result.uml_source);
}
