// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        eprintln!("Usage: rustuml [options] <file.puml>");
        eprintln!("       cat file.puml | rustuml [options] -");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -tsvg       Output SVG (default)");
        eprintln!("  --ast       Print parsed AST instead of rendering");
        eprintln!("  --version   Print version");
        eprintln!("  --help      Print this help");
        std::process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    if args[1] == "--version" {
        println!("rustuml {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let mut ast_mode = false;
    let mut input_arg = None;

    for arg in &args[1..] {
        match arg.as_str() {
            "--ast" => ast_mode = true,
            "-tsvg" => {} // default, accepted but no-op
            _ => input_arg = Some(arg.as_str()),
        }
    }

    let Some(input_path) = input_arg else {
        eprintln!("error: no input file specified");
        std::process::exit(1);
    };

    let input = if input_path == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .expect("failed to read stdin");
        buf
    } else {
        std::fs::read_to_string(input_path).unwrap_or_else(|e| {
            eprintln!("error: {input_path}: {e}");
            std::process::exit(1);
        })
    };

    match rustuml_parser::parse::parse(&input) {
        Ok(diagram) => {
            if ast_mode {
                println!("{diagram:#?}");
            } else {
                print!("{}", rustuml_render::render_svg(&diagram));
            }
        }
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
    }
}
