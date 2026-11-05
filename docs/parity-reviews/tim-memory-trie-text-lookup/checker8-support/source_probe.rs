// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
// Diagnostic only; shipped CLI measurements remain separate.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let source = std::fs::read_to_string(&args[1]).unwrap();
    let result = rustuml_parser::preprocess::preprocess_full(
        &source, Some(std::path::PathBuf::from(&args[2])));
    print!("{}", result.uml_source);
    if let Some(error) = result.error {
        eprintln!("{}: {}", error.line, error.message);
        std::process::exit(1);
    }
}
