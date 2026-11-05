// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
use std::io::{self, Read};
fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let diagram: rustuml_parser::diagram::Diagram = serde_yml::from_str(&input).unwrap();
    print!("{}", serde_yml::to_string(&diagram).unwrap());
}
