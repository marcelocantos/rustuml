// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Minimize a valid PlantUML case while preserving its first strict SVG divergence shape.
//!
//! Usage: cargo run -p rustuml-oracle --example parity_reduce -- <input.puml> [--out <dir>]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use rustuml_layout::capture_layout_diagnostics;
use rustuml_oracle::compare;
use rustuml_oracle::harness::golden_has_syntax_error;
use rustuml_oracle::java_trace::JavaSvekTracer;
use rustuml_oracle::reduce::reduce_lines;
use rustuml_oracle::trace::{
    dot_fingerprint, dot_fingerprint_differences, first_svg_difference_shape,
};
use serde::Serialize;

struct Evaluation {
    java_svg: String,
    java_request_dot: String,
    java_solved_svg: String,
    rust_svg: String,
    rust_request_dots: Vec<String>,
    rust_solved_dots: Vec<String>,
    signature: DivergenceSignature,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
struct DivergenceSignature {
    layout_request: Vec<String>,
    paint_stream: Option<String>,
}

#[derive(Serialize)]
struct ReductionReport {
    schema_version: u32,
    source: String,
    java_revision: String,
    java_jar: String,
    java_jar_sha256: String,
    signature: DivergenceSignature,
    original_line_count: usize,
    final_line_count: usize,
    attempts: usize,
    accepted: usize,
    rejected_candidates: usize,
    validity_gate: &'static str,
    claim_scope: &'static str,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("parity_reduce: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    unsafe { std::env::set_var("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100") };
    let (input, output_dir) = arguments()?;
    let source = std::fs::read_to_string(&input)
        .with_context(|| format!("cannot read {}", input.display()))?;
    let base = input.parent().unwrap_or_else(|| Path::new("."));
    let tracer = JavaSvekTracer::new()?;
    let trace_output = temporary_trace_output()?;
    let original = evaluate(&source, base, &tracer, &trace_output)?
        .context("original input is not valid in both PlantUML and RustUML")?;
    ensure!(
        original.signature.paint_stream.is_some(),
        "original input has no strict SVG divergence to reduce"
    );
    ensure!(
        !original.signature.layout_request.is_empty(),
        "reducer requires an observable layout-request projection divergence"
    );
    let signature = original.signature.clone();

    let mut rejected_candidates = 0_usize;
    let reduction = reduce_lines(&source, |candidate| {
        let preserves = evaluate(candidate, base, &tracer, &trace_output)?
            .is_some_and(|evaluation| evaluation.signature == signature);
        if !preserves {
            rejected_candidates += 1;
        }
        Ok::<_, anyhow::Error>(preserves)
    })?;
    let final_evaluation = evaluate(&reduction.source, base, &tracer, &trace_output)?
        .context("reducer produced an invalid final candidate")?;
    ensure!(
        final_evaluation.signature == signature,
        "reducer did not preserve the target divergence"
    );
    let _ = std::fs::remove_dir_all(&trace_output);

    std::fs::create_dir_all(&output_dir)
        .with_context(|| format!("cannot create {}", output_dir.display()))?;
    std::fs::write(output_dir.join("original.puml"), &source)?;
    std::fs::write(output_dir.join("reduced.puml"), &reduction.source)?;
    write_evaluation(&output_dir, "original", &original)?;
    write_evaluation(&output_dir, "reduced", &final_evaluation)?;
    let report = ReductionReport {
        schema_version: 2,
        source: input.to_string_lossy().into_owned(),
        java_revision: tracer.revision().to_string(),
        java_jar: tracer.jar_path().to_string_lossy().into_owned(),
        java_jar_sha256: tracer.jar_sha256().to_string(),
        signature: signature.clone(),
        original_line_count: reduction.original_line_count,
        final_line_count: reduction.final_line_count,
        attempts: reduction.attempts,
        accepted: reduction.accepted,
        rejected_candidates,
        validity_gate: "RustUML parse and traced layout succeed; PlantUML SVG is not classified by harness::golden_has_syntax_error",
        claim_scope: "One-minimal for the complete disagreement set in the declared layout-request projection and the paint-stream signature; not proof of complete mechanism identity",
    };
    std::fs::write(
        output_dir.join("reduction.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    println!(
        "reduced {} to {} lines in {} probes\nlayout signature: {}\npaint signature: {}\nartifacts: {}",
        reduction.original_line_count,
        reduction.final_line_count,
        reduction.attempts,
        signature.layout_request.join(" | "),
        signature.paint_stream.as_deref().unwrap_or("none"),
        output_dir.display()
    );
    Ok(())
}

fn evaluate(
    source: &str,
    base: &Path,
    tracer: &JavaSvekTracer,
    trace_output: &Path,
) -> Result<Option<Evaluation>> {
    let diagram = match rustuml_parser::parse::parse_auto_with_base(source, Some(base)) {
        Ok(diagram) => diagram,
        Err(_) => return Ok(None),
    };
    let (rust_svg, rust_capture) =
        capture_layout_diagnostics(|| rustuml_render::render_svg(&diagram));
    if rust_capture.timed_out > 0 || rust_capture.failed > 0 {
        return Ok(None);
    }
    let rust_layouts = &rust_capture.layouts;
    let Some(java) = tracer.capture_candidate(source, trace_output)? else {
        return Ok(None);
    };
    if golden_has_syntax_error(&java.rendered_svg) {
        return Ok(None);
    }
    let java_fingerprint = dot_fingerprint(&java.request_dot);
    let layout_request = match rust_layouts.as_slice() {
        [] => vec!["Rust produced no Graphviz request".to_string()],
        [layout] => {
            dot_fingerprint_differences(&java_fingerprint, &dot_fingerprint(&layout.request_dot))
        }
        many => vec![format!(
            "Java produced one Graphviz request; Rust produced {}",
            many.len()
        )],
    };
    let comparison = compare::compare_svg_strict(&java.rendered_svg, &rust_svg)
        .map_err(|error| anyhow::anyhow!("strict SVG comparison failed: {error}"))?;
    Ok(Some(Evaluation {
        java_svg: java.rendered_svg,
        java_request_dot: java.request_dot,
        java_solved_svg: java.solved_svg,
        rust_svg,
        rust_request_dots: rust_layouts
            .iter()
            .map(|layout| layout.request_dot.clone())
            .collect(),
        rust_solved_dots: rust_layouts
            .iter()
            .map(|layout| layout.solved_dot.clone())
            .collect(),
        signature: DivergenceSignature {
            layout_request,
            paint_stream: first_svg_difference_shape(&comparison),
        },
    }))
}

fn write_evaluation(output_dir: &Path, prefix: &str, evaluation: &Evaluation) -> Result<()> {
    std::fs::write(
        output_dir.join(format!("{prefix}-java.svg")),
        &evaluation.java_svg,
    )?;
    std::fs::write(
        output_dir.join(format!("{prefix}-java-request.dot")),
        &evaluation.java_request_dot,
    )?;
    std::fs::write(
        output_dir.join(format!("{prefix}-java-solved.svg")),
        &evaluation.java_solved_svg,
    )?;
    std::fs::write(
        output_dir.join(format!("{prefix}-rust.svg")),
        &evaluation.rust_svg,
    )?;
    for (index, request) in evaluation.rust_request_dots.iter().enumerate() {
        std::fs::write(
            output_dir.join(format!("{prefix}-rust-{index}-request.dot")),
            request,
        )?;
    }
    for (index, solved) in evaluation.rust_solved_dots.iter().enumerate() {
        std::fs::write(
            output_dir.join(format!("{prefix}-rust-{index}-solved.dot")),
            solved,
        )?;
    }
    Ok(())
}

fn temporary_trace_output() -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!("rustuml-parity-reduce-{}", std::process::id()));
    std::fs::create_dir_all(&path).with_context(|| format!("cannot create {}", path.display()))?;
    Ok(path)
}

fn arguments() -> Result<(PathBuf, PathBuf)> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let Some(input) = arguments.first().map(PathBuf::from) else {
        bail!("usage: parity_reduce <input.puml> [--out <directory>]");
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
        .unwrap_or("reduction");
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parity-reduce")
        .join(stem)
}
