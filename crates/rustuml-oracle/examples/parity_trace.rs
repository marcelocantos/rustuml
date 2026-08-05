// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Capture paired Java/Rust parity checkpoints for one valid PlantUML input.
//!
//! Usage: cargo run -p rustuml-oracle --example parity_trace -- <input.puml> [--out <dir>]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rustuml_layout::capture_layout_diagnostics;
use rustuml_oracle::compare;
use rustuml_oracle::java_trace::capture_java_svek;
use rustuml_oracle::trace::{
    DotFingerprint, dot_fingerprint, first_dot_fingerprint_difference, first_svg_difference_shape,
};
use serde::Serialize;

#[derive(Serialize)]
struct StudySummary {
    schema_version: u32,
    source: String,
    java_revision: String,
    java_jar_sha256: String,
    first_observed_divergence: Option<String>,
    checkpoints: Vec<Checkpoint>,
}

#[derive(Serialize)]
struct Checkpoint {
    name: String,
    status: String,
    detail: Option<String>,
    artifacts: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("parity_trace: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    unsafe { std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100") };
    let (input, output_dir) = arguments()?;
    let source = std::fs::read_to_string(&input)
        .with_context(|| format!("cannot read {}", input.display()))?;
    let diagram = rustuml_parser::parse::parse_auto_with_base(&source, input.parent())
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("cannot create {}", output_dir.display()))?;

    let model = serde_json::to_string_pretty(&diagram)?;
    std::fs::write(output_dir.join("rust-model.json"), model)?;
    let (rust_svg, rust_capture) =
        capture_layout_diagnostics(|| rustuml_render::render_svg(&diagram));
    let rust_layouts = &rust_capture.layouts;
    std::fs::write(output_dir.join("rust.svg"), &rust_svg)?;
    for (index, layout) in rust_layouts.iter().enumerate() {
        std::fs::write(
            output_dir.join(format!("rust-layout-{index}-request.dot")),
            &layout.request_dot,
        )?;
        std::fs::write(
            output_dir.join(format!("rust-layout-{index}-solved.dot")),
            &layout.solved_dot,
        )?;
    }

    let java = capture_java_svek(&source, &output_dir)?;
    std::fs::write(output_dir.join("java.svg"), &java.rendered_svg)?;
    std::fs::write(output_dir.join("java_svek.dot"), &java.request_dot)?;
    std::fs::write(output_dir.join("java_svek.svg"), &java.solved_svg)?;

    let java_layout = dot_fingerprint(&java.request_dot);
    std::fs::write(
        output_dir.join("java-layout-fingerprint.json"),
        serde_json::to_string_pretty(&java_layout)?,
    )?;
    let rust_fingerprints = rust_layouts
        .iter()
        .map(|layout| dot_fingerprint(&layout.request_dot))
        .collect::<Vec<DotFingerprint>>();
    std::fs::write(
        output_dir.join("rust-layout-fingerprints.json"),
        serde_json::to_string_pretty(&rust_fingerprints)?,
    )?;

    let layout_difference = if rust_capture.timed_out > 0 || rust_capture.failed > 0 {
        Some(format!(
            "Rust layout capture incomplete: {} timed out, {} failed",
            rust_capture.timed_out, rust_capture.failed
        ))
    } else {
        match rust_fingerprints.as_slice() {
            [] => Some("Rust produced no Graphviz request".to_string()),
            [fingerprint] => first_dot_fingerprint_difference(&java_layout, fingerprint),
            many => Some(format!(
                "Java produced one Graphviz request; Rust produced {}",
                many.len()
            )),
        }
    };
    let svg_comparison = compare::compare_svg_strict(&java.rendered_svg, &rust_svg)
        .map_err(|error| anyhow::anyhow!("strict SVG comparison failed: {error}"))?;
    std::fs::write(output_dir.join("svg.diff.txt"), svg_comparison.to_string())?;
    let svg_difference = first_svg_difference_shape(&svg_comparison);

    let mut layout_request_artifacts = vec![
        "java_svek.dot".to_string(),
        "java-layout-fingerprint.json".to_string(),
        "rust-layout-fingerprints.json".to_string(),
    ];
    let mut layout_result_artifacts = vec!["java_svek.svg".to_string()];
    for index in 0..rust_layouts.len() {
        layout_request_artifacts.push(format!("rust-layout-{index}-request.dot"));
        layout_result_artifacts.push(format!("rust-layout-{index}-solved.dot"));
    }

    let first_observed_divergence = if layout_difference.is_some() {
        Some("layout-request".to_string())
    } else if svg_difference.is_some() {
        Some("paint-stream".to_string())
    } else {
        None
    };
    let summary = StudySummary {
        schema_version: 2,
        source: input.to_string_lossy().into_owned(),
        java_revision: java.revision,
        java_jar_sha256: java.jar_sha256,
        first_observed_divergence: first_observed_divergence.clone(),
        checkpoints: vec![
            Checkpoint {
                name: "semantic-model".to_string(),
                status: "rust-captured".to_string(),
                detail: Some(
                    "Java semantic projection is not yet installed; this checkpoint cannot adjudicate divergence"
                        .to_string(),
                ),
                artifacts: vec!["rust-model.json".to_string()],
            },
            Checkpoint {
                name: "layout-request".to_string(),
                status: if layout_difference.is_some() {
                    "diverged".to_string()
                } else {
                    "covered-fields-match".to_string()
                },
                detail: layout_difference.or_else(|| {
                    Some(
                        "No difference in the canonical fields covered by this projection; this is not proof of complete DOT equivalence"
                            .to_string(),
                    )
                }),
                artifacts: layout_request_artifacts,
            },
            Checkpoint {
                name: "layout-result".to_string(),
                status: "captured-not-compared".to_string(),
                detail: Some(
                    "Raw Java Graphviz SVG and solved Rust Graphviz DOT are preserved for forensic comparison"
                        .to_string(),
                ),
                artifacts: layout_result_artifacts,
            },
            Checkpoint {
                name: "paint-stream".to_string(),
                status: if svg_comparison.is_match() {
                    "exact".to_string()
                } else {
                    "diverged".to_string()
                },
                detail: svg_difference,
                artifacts: vec![
                    "java.svg".to_string(),
                    "rust.svg".to_string(),
                    "svg.diff.txt".to_string(),
                ],
            },
        ],
    };
    std::fs::write(
        output_dir.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;

    match first_observed_divergence {
        Some(checkpoint) => println!(
            "first observed divergence: {checkpoint}\nartifacts: {}",
            output_dir.display()
        ),
        None => println!(
            "all comparable checkpoints match\nartifacts: {}",
            output_dir.display()
        ),
    }
    Ok(())
}

fn arguments() -> Result<(PathBuf, PathBuf)> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let Some(input) = arguments.first().map(PathBuf::from) else {
        bail!("usage: parity_trace <input.puml> [--out <directory>]");
    };
    let output_dir = arguments
        .iter()
        .position(|argument| argument == "--out")
        .and_then(|index| arguments.get(index + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_dir(&input));
    Ok((input, output_dir))
}

fn default_output_dir(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("study");
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parity-trace")
        .join(stem)
}
