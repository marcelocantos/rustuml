// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        eprintln!("Usage: rustuml [options] <file>");
        eprintln!("       cat file | rustuml [options] -");
        eprintln!();
        eprintln!("Input formats: PlantUML (.puml), YAML (.yaml/.yml), JSON (.json)");
        eprintln!("Format is auto-detected from content, or use file extension.");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -tsvg       Output SVG (default)");
        eprintln!("  --ast       Print parsed AST (Debug format)");
        eprintln!("  --yaml      Print parsed diagram as YAML");
        eprintln!("  --version   Print version");
        eprintln!("  --help      Print this help");
        std::process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    if args[1] == "--version" {
        println!("rustuml {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let mut output_mode = OutputMode::Svg;
    let mut input_arg = None;
    let mut theme_name = "default";

    for arg in &args[1..] {
        match arg.as_str() {
            "--ast" => output_mode = OutputMode::Ast,
            "--yaml" => output_mode = OutputMode::Yaml,
            "-tsvg" => output_mode = OutputMode::Svg,
            "--theme=modern" => theme_name = "modern",
            "--theme=default" => theme_name = "default",
            s if s.starts_with("--theme=") => {
                eprintln!("unknown theme: {}", &s[8..]);
                eprintln!("available: default, modern");
                std::process::exit(1);
            }
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

    let theme = match theme_name {
        "modern" => rustuml_render::style::Theme::modern(),
        _ => rustuml_render::style::Theme::default(),
    };

    match rustuml_parser::parse::parse_auto(&input) {
        Ok(diagram) => match output_mode {
            OutputMode::Ast => println!("{diagram:#?}"),
            OutputMode::Yaml => {
                print!(
                    "{}",
                    serde_yaml::to_string(&diagram).expect("YAML serialization failed")
                );
            }
            OutputMode::Svg => {
                print!(
                    "{}",
                    rustuml_render::render_svg_with_theme(&diagram, &theme)
                );
            }
        },
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
    }
}

enum OutputMode {
    Svg,
    Ast,
    Yaml,
}
