// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        eprintln!("Usage: rustuml [options] <file>");
        eprintln!("       cat file | rustuml [options] -");
        eprintln!("       cat file | rustuml -pipe [options]");
        eprintln!();
        eprintln!("Input formats: PlantUML (.puml), YAML (.yaml/.yml), JSON (.json)");
        eprintln!("Format is auto-detected from content, or use file extension.");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -pipe                 Read from stdin, write to stdout (PlantUML compatible)");
        eprintln!("  -tsvg                 Output SVG (default)");
        eprintln!("  -tpng                 Output PNG");
        eprintln!("  -tpdf                 Output PDF");
        eprintln!("  -ttxt                 Output ASCII art text (sequence diagrams)");
        eprintln!("  --ast                 Print parsed AST (Debug format)");
        eprintln!("  --yaml                Print parsed diagram as YAML");
        eprintln!("  --theme=NAME          Use built-in theme (default, modern)");
        eprintln!("  --theme-file=PATH     Load theme from YAML file");
        eprintln!("  --version             Print version");
        eprintln!("  --help                Print this help");
        eprintln!("  --help-agent          Print agent integration guide");
        std::process::exit(if args.len() < 2 { 1 } else { 0 });
    }

    if args[1] == "--help-agent" {
        print_agent_guide();
        return;
    }

    if args[1] == "--version" {
        println!("rustuml {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let mut output_mode = OutputMode::Svg;
    let mut input_arg = None;
    let mut pipe_mode = false;
    let mut theme = rustuml_render::style::Theme::default();

    for arg in &args[1..] {
        match arg.as_str() {
            "-pipe" => pipe_mode = true,
            "--ast" => output_mode = OutputMode::Ast,
            "--yaml" => output_mode = OutputMode::Yaml,
            "-tsvg" => output_mode = OutputMode::Svg,
            "-tpng" => output_mode = OutputMode::Png,
            "-tpdf" => output_mode = OutputMode::Pdf,
            "-ttxt" => output_mode = OutputMode::Txt,
            "--theme=modern" => theme = rustuml_render::style::Theme::modern(),
            "--theme=default" => theme = rustuml_render::style::Theme::default(),
            s if s.starts_with("--theme-file=") => {
                let path = &s[13..];
                let yaml = std::fs::read_to_string(path).unwrap_or_else(|e| {
                    eprintln!("error reading theme file {path}: {e}");
                    std::process::exit(1);
                });
                theme = serde_yaml::from_str(&yaml).unwrap_or_else(|e| {
                    eprintln!("error parsing theme file: {e}");
                    std::process::exit(1);
                });
            }
            s if s.starts_with("--theme=") => {
                eprintln!("unknown theme: {}", &s[8..]);
                eprintln!("available: default, modern");
                eprintln!("or use --theme-file=path/to/theme.yaml");
                std::process::exit(1);
            }
            _ => input_arg = Some(arg.as_str()),
        }
    }

    // -pipe flag is an alias for reading from stdin (PlantUML compatibility).
    // It takes precedence over any positional file argument.
    if pipe_mode {
        input_arg = Some("-");
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

    let base_dir = if input_path != "-" {
        std::path::Path::new(input_path).parent()
    } else {
        None
    };

    match rustuml_parser::parse::parse_auto_with_base(&input, base_dir) {
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
            OutputMode::Pdf => {
                let svg = rustuml_render::render_svg_with_theme(&diagram, &theme);
                match rustuml_render::pdf::svg_to_pdf(&svg) {
                    Ok(bytes) => {
                        use std::io::Write;
                        std::io::stdout().write_all(&bytes).expect("write failed");
                    }
                    Err(e) => {
                        eprintln!("PDF error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            OutputMode::Png => {
                let svg = rustuml_render::render_svg_with_theme(&diagram, &theme);
                match rustuml_render::png::svg_to_png(&svg) {
                    Ok(bytes) => {
                        use std::io::Write;
                        std::io::stdout().write_all(&bytes).expect("write failed");
                    }
                    Err(e) => {
                        eprintln!("PNG error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            OutputMode::Txt => {
                print!("{}", rustuml_render::render_ascii(&diagram));
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
    Png,
    Pdf,
    Txt,
    Ast,
    Yaml,
}

fn print_agent_guide() {
    println!("# rustuml Agent Integration Guide");
    println!();
    println!("rustuml converts PlantUML, YAML, or JSON diagram descriptions to SVG or PNG.");
    println!();
    println!("## Input Formats");
    println!("- **PlantUML**: Standard @startuml/@enduml text syntax");
    println!("- **YAML**: Structured diagram model (type: Sequence/Class/State/Activity)");
    println!("- **JSON**: Same model as YAML, JSON-encoded");
    println!();
    println!("YAML/JSON is recommended for AI agents — no escaping or syntax ambiguity.");
    println!("Use `rustuml --yaml <file.puml>` to convert PlantUML to YAML for reference.");
    println!();
    println!("## Supported Diagram Types");
    println!("Sequence, Class, State, Activity, Component, UseCase");
    println!();
    println!("## Output Formats");
    println!("- SVG (default): `rustuml -tsvg input.puml`");
    println!("- PNG: `rustuml -tpng input.puml > output.png`");
    println!("- ASCII text: `rustuml -ttxt input.puml` (sequence diagrams only)");
    println!();
    println!("## Themes");
    println!("- `--theme=default`: Classic PlantUML colors");
    println!("- `--theme=modern`: Cleaner, lighter palette");
    println!("- `--theme-file=path.yaml`: Custom theme from YAML file");
    println!();
    println!("## Preprocessor");
    println!("Supports !define, !$var, !ifdef/!ifndef/!if/!else/!endif, !include, comments.");
    println!();
    println!("## Skinparams");
    println!("Inline style overrides: `skinparam participantBackgroundColor #FF0000`");
}
