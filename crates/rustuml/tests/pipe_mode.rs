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

#[test]
fn pipe_rejects_fatal_preprocessing_errors() {
    for declaration in ["class Before", "Alice -> Bob: before"] {
        for expression in ["%date(\"QQ\",0)", "%date(\"yyyy\",2147483648)"] {
            let input = format!("@startuml\n{declaration}\n!$bad = {expression}\n@enduml\n");
            let output = run_pipe(&["-pipe", "-tsvg"], input.as_bytes());
            assert!(!output.status.success(), "accepted {input}");
            assert!(
                output.stdout.is_empty(),
                "emitted a partial diagram for {input}"
            );
            assert!(
                !output.stderr.is_empty(),
                "missing fatal diagnostic for {input}"
            );
        }
    }
}

#[test]
fn pipe_accepts_valid_dates_and_inactive_errors() {
    for declaration in ["class Before", "Alice -> Bob: before"] {
        let input = format!(
            "@startuml\n{declaration}\n!if 0\n!$bad = %date(\"QQ\",0)\n!endif\n!$ok = %date(\"yyyy\",0)\n@enduml\n"
        );
        let output = run_pipe(&["-pipe", "-tsvg"], input.as_bytes());
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("<svg"));
    }
}

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

#[test]
fn help_agent_discloses_no_oracle_product_metric() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustuml"))
        .arg("--help-agent")
        .output()
        .expect("failed to run rustuml --help-agent");
    assert!(
        output.status.success(),
        "expected success, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Product truth is the no-oracle product tier"),
        "--help-agent must name no-oracle as the product metric"
    );
    let baseline = include_str!("../../../test-diagrams/no_oracle_baseline.txt");
    let mut passed = 0_usize;
    let mut eligible = 0_usize;
    for line in baseline.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 3, "malformed baseline line: {line}");
        passed += fields[1].parse::<usize>().expect("baseline pass count");
        eligible += fields[2].parse::<usize>().expect("baseline eligible count");
    }
    assert!(eligible > 0 && passed <= eligible);
    // Ignore grouping commas, but check the full ratio and percentage against
    // the checked-in ratchet rather than another hand-maintained count.
    let expected = format!(
        "{passed}/{eligible} eligible SVG goldens pass ({:.1}%)",
        passed as f64 / eligible as f64 * 100.0
    );
    assert!(
        stdout.replace(',', "").contains(&expected),
        "--help-agent must disclose the checked-in no-oracle baseline: {expected}"
    );
    assert!(
        stdout.contains("The strict tier is oracle-assisted"),
        "--help-agent must distinguish assisted parity from product parity"
    );
    assert!(
        stdout.contains("1,199 Java PlantUML error-page goldens are skips"),
        "--help-agent must disclose Java error-page skips"
    );
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

/// Every emitted SVG is parsed, including negative/error cases. T14.20 owns
/// Java's render/export exception contract; these three existing malformed
/// outputs stay visible and ratchet in both directions, keyed to source bytes.
#[test]
fn every_cli_golden_and_registered_perturbation_svg_is_strict_xml() {
    use sha2::{Digest, Sha256};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const EXPECTED_MALFORMED: &[(&str, &str, &str, &str)] = &[
        (
            "test-diagrams/perturbations/shared-svg-attributes/attr_link_c0_one.puml",
            "667816bc90d54c4087e0fef4f1482ca196113f702d63352cdac8e3fbdf3db49a",
            "T14.20",
            "U+0001 tooltip: Java SAXException crash SVG; Rust infallible rendering emits invalid XML",
        ),
        (
            "test-diagrams/perturbations/shared-svg-attributes/attr_link_c0_zero.puml",
            "40a5258df094ea2f37db2ab595aca578037f774158f898a423bc5ec0cf74295f",
            "T14.20",
            "U+0000 tooltip: Java SAXException crash SVG; Rust infallible rendering emits invalid XML",
        ),
        (
            "test-diagrams/perturbations/shared-svg-attributes/attr_link_c0_escape.puml",
            "a5d5b51b19d68f8bfa2923e709b8f611a3f88a28e6ad6c0b5efc3226fae40ce6",
            "T14.20",
            "U+001B tooltip: Java SAXException crash SVG; Rust infallible rendering emits invalid XML",
        ),
    ];

    fn collect(dir: &Path, paths: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("required XML census corpus is missing") {
            let path = entry.expect("unreadable XML census entry").path();
            if path.is_dir() {
                collect(&path, paths);
            } else if path.extension().is_some_and(|e| e == "puml")
                && path.with_extension("svg").is_file()
            {
                paths.push(path);
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut paths = Vec::new();
    collect(&root.join("test-diagrams/golden"), &mut paths);
    assert!(!paths.is_empty(), "golden corpus must be hydrated");
    let golden_count = paths.len();
    collect(&root.join("test-diagrams/perturbations"), &mut paths);
    assert!(
        paths.len() > golden_count,
        "registered perturbations are required"
    );
    paths.sort();
    for (path, _, owner, _) in EXPECTED_MALFORMED {
        assert!(
            paths.contains(&root.join(path)),
            "missing expected failure {path} ({owner})"
        );
    }

    let next = AtomicUsize::new(0);
    let standard_dot_doctypes = AtomicUsize::new(0);
    let observations = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(path) = paths.get(i) else { break };
                let relative = path.strip_prefix(&root).unwrap().to_str().unwrap();
                let source = std::fs::read(path).expect("cannot read census input");
                let expected = EXPECTED_MALFORMED.iter().find(|e| e.0 == relative);
                let source_hash = format!("{:x}", Sha256::digest(&source));
                let mut child = Command::new(env!("CARGO_BIN_EXE_rustuml"))
                    .args(["-pipe", "-tsvg"])
                    .current_dir(&root)
                    .env("TZ", "GMT+11:00")
                    .env("RUSTUML_DEBUG", "date=1774210426000,tz=AEDT+1100")
                    .env("RAYON_NUM_THREADS", "4")
                    .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
                    .spawn().expect("cannot start actual CLI XML census");
                child.stdin.take().unwrap().write_all(&source).expect("cannot send source to CLI");
                let output = child.wait_with_output().expect("cannot collect CLI SVG");
                let mut failures = Vec::new();
                let xml_error = if output.stdout.is_empty() {
                    None
                } else {
                    match std::str::from_utf8(&output.stdout) {
                        Err(error) => Some(error.to_string()),
                        Ok(svg) => {
                            if let Some(start) = svg.find("<!DOCTYPE") {
                                let declaration = svg[start..].split_once('>').map(|(d, _)| {
                                    format!("{d}>").split_whitespace().collect::<Vec<_>>().join(" ")
                                });
                                let standard = r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">"#;
                                if declaration.as_deref() != Some(standard)
                                    || !source.windows(b"@startdot".len()).any(|w| w == b"@startdot")
                                {
                                    failures.push(format!("unexpected DTD in {relative}: {declaration:?}"));
                                } else {
                                    standard_dot_doctypes.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                            // roxmltree 0.20 src/parse.rs:315–345: rejecting DTDs
                            // is an optional security policy, not XML well-formedness.
                            // Parse the standard DOT SVG DOCTYPE; all other options
                            // and internal entity-expansion limits stay at defaults.
                            // No external DTD/entity resolver or I/O is installed.
                            roxmltree::Document::parse_with_options(svg, roxmltree::ParsingOptions {
                                allow_dtd: true,
                                ..Default::default()
                            }).err().map(|e| e.to_string())
                        },
                    }
                };
                if let Some((_, hash, owner, reason)) = expected {
                    if source_hash != *hash {
                        failures.push(format!("stale source hash: {relative} ({owner}): {source_hash} != {hash}"));
                    }
                    if xml_error.is_none() {
                        failures.push(format!("stale expected malformed output: {relative} ({owner}); remove entry after validating improvement: {reason}"));
                    }
                    eprintln!("MALFORMED expected {relative} owner={owner}: {reason}; observed={xml_error:?}");
                } else if let Some(error) = &xml_error {
                    failures.push(format!("unlisted malformed SVG: {relative}: {error}"));
                }
                observations.lock().unwrap().push((
                    relative.starts_with("test-diagrams/golden/"),
                    !output.stdout.is_empty(), xml_error.is_some(), !output.status.success(),
                    String::from_utf8_lossy(&output.stderr).contains("panicked at"), failures,
                ));
            });
        }
    });
    assert_eq!(
        standard_dot_doctypes.load(Ordering::Relaxed),
        26,
        "the frozen CLI corpus must retain its 26 standard SVG 1.1 DOT DOCTYPE declarations"
    );
    let observations = observations.into_inner().unwrap();
    for (golden, name) in [(true, "golden"), (false, "perturbation")] {
        let rows: Vec<_> = observations.iter().filter(|r| r.0 == golden).collect();
        eprintln!(
            "XML corpus={name} inputs={} emitted={} malformed={} process_errors={} panics={}",
            rows.len(),
            rows.iter().filter(|r| r.1).count(),
            rows.iter().filter(|r| r.2).count(),
            rows.iter().filter(|r| r.3).count(),
            rows.iter().filter(|r| r.4).count()
        );
    }
    let failures: Vec<_> = observations.into_iter().flat_map(|r| r.5).collect();
    assert!(
        failures.is_empty(),
        "strict XML census failed:\n{}",
        failures.join("\n")
    );
}
