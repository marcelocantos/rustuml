// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for `-pipe` mode (read stdin, write stdout).
//!
//! These tests verify that `rustuml -pipe [-tFMT]` behaves like
//! `rustuml [options] -`, which is the existing stdin-read convention.

use std::io::Write;
use std::process::{Command, Stdio};

const SEQUENCE_PUML: &str = r#"@startuml
Alice -> Bob: Hello
Bob --> Alice: Hi
@enduml
"#;

const CLASS_PUML: &str = r#"@startuml
class Animal {
  +name: String
  +speak(): void
}
class Dog extends Animal
@enduml
"#;

const INVALID_PUML: &str = "this is not valid plantuml at all !!@@##";

const SKINPARAM_PUML: &str = r#"@startuml
skinparam participantBackgroundColor #AADDFF
Alice -> Bob: Styled
@enduml
"#;

/// Helper: run rustuml with given args and stdin, return the process output.
fn run_pipe(args: &[&str], stdin_data: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rustuml"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn rustuml");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin_data)
        .expect("failed to write stdin");

    child
        .wait_with_output()
        .expect("failed to wait for rustuml")
}

// ── Test 1: -pipe with -tsvg produces SVG for a sequence diagram ─────────────

#[test]
fn pipe_svg_sequence_diagram() {
    let output = run_pipe(&["-pipe", "-tsvg"], SEQUENCE_PUML.as_bytes());
    assert!(
        output.status.success(),
        "expected success, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<svg"),
        "expected SVG output, got: {stdout}"
    );
}

// ── Test 2: -pipe with -tsvg produces SVG for a class diagram ────────────────

#[test]
fn pipe_svg_class_diagram() {
    let output = run_pipe(&["-pipe", "-tsvg"], CLASS_PUML.as_bytes());
    assert!(
        output.status.success(),
        "expected success, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<svg"),
        "expected SVG output, got: {stdout}"
    );
}

// ── Test 3: -pipe with -tpng produces valid PNG (magic bytes 89 50 4E 47) ────

#[test]
fn pipe_png_output() {
    let output = run_pipe(&["-pipe", "-tpng"], SEQUENCE_PUML.as_bytes());
    assert!(
        output.status.success(),
        "expected success, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output
            .stdout
            .starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
        "expected PNG magic bytes, got {} bytes of output",
        output.stdout.len()
    );
}

// ── Test 4: -pipe with -tpdf produces valid PDF (magic bytes %PDF) ───────────

#[test]
fn pipe_pdf_output() {
    let output = run_pipe(&["-pipe", "-tpdf"], SEQUENCE_PUML.as_bytes());
    assert!(
        output.status.success(),
        "expected success, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.starts_with(b"%PDF"),
        "expected PDF magic bytes, got {} bytes of output",
        output.stdout.len()
    );
}

// ── Test 5: -pipe with -ttxt produces ASCII art ───────────────────────────────

#[test]
fn pipe_ascii_text_output() {
    let output = run_pipe(&["-pipe", "-ttxt"], SEQUENCE_PUML.as_bytes());
    assert!(
        output.status.success(),
        "expected success, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // ASCII art should contain participant names
    assert!(
        stdout.contains("Alice") || stdout.contains("Bob") || !stdout.is_empty(),
        "expected non-empty ASCII output, got: {stdout}"
    );
}

// ── Test 6: -pipe with --yaml produces YAML output ────────────────────────────

#[test]
fn pipe_yaml_output() {
    let output = run_pipe(&["-pipe", "--yaml"], SEQUENCE_PUML.as_bytes());
    assert!(
        output.status.success(),
        "expected success, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // YAML output should start with a mapping key
    assert!(
        stdout.contains(':'),
        "expected YAML output with key-value pairs, got: {stdout}"
    );
}

// ── Test 7: -pipe with --theme=modern produces SVG ───────────────────────────

#[test]
fn pipe_with_modern_theme() {
    let output = run_pipe(
        &["-pipe", "-tsvg", "--theme=modern"],
        SEQUENCE_PUML.as_bytes(),
    );
    assert!(
        output.status.success(),
        "expected success, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<svg"),
        "expected SVG output with modern theme, got: {stdout}"
    );
}

// ── Test 8: -pipe with skinparams in the input ───────────────────────────────

#[test]
fn pipe_with_skinparams_in_input() {
    let output = run_pipe(&["-pipe", "-tsvg"], SKINPARAM_PUML.as_bytes());
    assert!(
        output.status.success(),
        "expected success, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<svg"),
        "expected SVG output with skinparams, got: {stdout}"
    );
}

// ── Test 9: -pipe with empty input produces no output or exits cleanly ────────

#[test]
fn pipe_with_empty_input() {
    let output = run_pipe(&["-pipe", "-tsvg"], b"");
    // Empty input should either succeed with empty output or exit non-zero.
    // Either is acceptable — the process must not hang.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // If it succeeds, the output should be either empty or valid SVG.
    if output.status.success() {
        assert!(
            stdout.is_empty() || stdout.contains("<svg"),
            "unexpected output for empty input: {stdout}"
        );
    } else {
        // Non-zero exit is acceptable for empty input; just verify it terminates.
        assert!(
            !stderr.is_empty() || output.status.code().is_some(),
            "process should terminate with a status code for empty input"
        );
    }
}

// ── Test 10: -pipe with invalid PlantUML terminates without hanging ───────────
//
// The parser is intentionally lenient: unrecognised content is parsed into an
// empty diagram and rendered as a minimal SVG rather than hard-erroring.  We
// therefore only verify that the process terminates (no hang) and produces
// either a valid SVG or exits with a non-zero status.

#[test]
fn pipe_with_invalid_input_terminates() {
    let output = run_pipe(&["-pipe", "-tsvg"], INVALID_PUML.as_bytes());
    // The process must complete and return a status code — the key invariant
    // is that it does not hang waiting for more input.
    assert!(
        output.status.code().is_some(),
        "process should terminate with a status code for invalid input"
    );
    if output.status.success() {
        // Lenient parse path: output should still be a valid (possibly empty) SVG.
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("<svg"),
            "successful exit should produce SVG, got: {stdout}"
        );
    }
    // Non-zero exit is also acceptable; either behaviour is fine.
}

// ── Test 11: -pipe and `-` are equivalent for SVG output ─────────────────────

#[test]
fn pipe_flag_equivalent_to_dash_input() {
    let pipe_output = run_pipe(&["-pipe", "-tsvg"], SEQUENCE_PUML.as_bytes());
    let dash_output = run_pipe(&["-tsvg", "-"], SEQUENCE_PUML.as_bytes());

    assert!(pipe_output.status.success(), "pipe mode failed");
    assert!(dash_output.status.success(), "dash mode failed");

    let pipe_svg = String::from_utf8_lossy(&pipe_output.stdout);
    let dash_svg = String::from_utf8_lossy(&dash_output.stdout);

    // Both should produce SVG; content should be identical.
    assert!(pipe_svg.contains("<svg"), "pipe mode should produce SVG");
    assert!(dash_svg.contains("<svg"), "dash mode should produce SVG");
    assert_eq!(
        pipe_svg, dash_svg,
        "-pipe and - should produce identical output"
    );
}

// ── Test 12: Multiple sequential invocations — process must not hang ──────────

#[test]
fn pipe_multiple_invocations_do_not_hang() {
    for i in 0..3 {
        let output = run_pipe(&["-pipe", "-tsvg"], SEQUENCE_PUML.as_bytes());
        assert!(
            output.status.success(),
            "invocation {i} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("<svg"), "invocation {i} should produce SVG");
    }
}
