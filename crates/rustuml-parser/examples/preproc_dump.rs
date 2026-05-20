// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Dump preprocessor expansion for a .puml file.
//! Usage: cargo run --release -p rustuml-parser --example preproc_dump -- <path>

fn main() {
    // SAFETY: single-threaded program.
    unsafe { std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100") };

    let args: Vec<_> = std::env::args().collect();
    let path = std::path::PathBuf::from(&args[1]);
    let src = std::fs::read_to_string(&path).unwrap();
    let base = path.parent().unwrap();
    let lines = rustuml_parser::preprocess::preprocess_with_base(&src, base);
    for line in lines {
        println!("{line}");
    }
}
