// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        eprintln!("Usage: rustuml <file.puml>");
        eprintln!("       cat file.puml | rustuml -");
        eprintln!();
        eprintln!(
            "Parses PlantUML input and prints the diagram model (rendering not yet implemented)."
        );
        std::process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    if args[1] == "--version" {
        println!("rustuml {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let input = if args[1] == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .expect("failed to read stdin");
        buf
    } else {
        std::fs::read_to_string(&args[1]).unwrap_or_else(|e| {
            eprintln!("error: {}: {e}", args[1]);
            std::process::exit(1);
        })
    };

    match rustuml_parser::parse::parse(&input) {
        Ok(diagram) => {
            println!("{diagram:#?}");
        }
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
    }
}
