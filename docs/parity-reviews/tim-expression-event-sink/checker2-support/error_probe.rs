// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
// Diagnostics only: public typed preprocessor and parser APIs, never a parity substitute.
fn main() {
    for path in std::env::args().skip(1) {
        let source = std::fs::read_to_string(&path).unwrap();
        let pre = rustuml_parser::preprocess::preprocess_full(&source, None);
        println!("FILE {path}\nPRE_ERROR {:?}\nPRE_SOURCE {:?}", pre.error, pre.uml_source);
        match rustuml_parser::parse::parse(&source) {
            Ok(_) => println!("PARSE_OK"),
            Err(e) => println!("PARSE_ERROR {} {:?}", e.line, e.message),
        }
    }
}
