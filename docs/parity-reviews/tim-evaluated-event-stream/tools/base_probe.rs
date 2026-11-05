// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let source = std::fs::read_to_string(&args[1]).unwrap();
    let result = rustuml_parser::preprocess::preprocess_full_with_base(&source, std::path::Path::new(&args[2]));
    print!("{}", result.uml_source);
}
