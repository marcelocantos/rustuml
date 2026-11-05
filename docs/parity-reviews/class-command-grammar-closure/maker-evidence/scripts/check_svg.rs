// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
// Compare two already-rendered SVGs with the repository's unchanged comparator.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = std::env::args().skip(1).collect();
    if paths.len() != 2 {
        return Err("usage: check_svg JAVA.svg CLI.svg".into());
    }
    let golden = std::fs::read_to_string(&paths[0])?;
    let actual = std::fs::read_to_string(&paths[1])?;
    if rustuml_oracle::harness::golden_has_syntax_error(&golden) {
        eprintln!("invalid Java reference");
        std::process::exit(2);
    }
    let comparison = rustuml_oracle::compare::compare_svg_strict(&golden, &actual)?;
    println!("{comparison}");
    if !comparison.is_match() {
        std::process::exit(1);
    }
    Ok(())
}
