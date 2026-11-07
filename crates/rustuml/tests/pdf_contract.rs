// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Oracle for Rust's PDF exporter.
//!
//! Pinned Java comments out the PDF route in `TitledDiagram.java` and
//! `TextBlockExporter12026.createUGraphic` throws `UnsupportedOperationException`
//! for every `-tpdf` request. The owner decision of 2026-11-06 keeps Rust's
//! exporter as a deliberate extension. This test never invokes Java and never
//! compares PDF bytes to a Java reference. The parity tiers render SVG only.

use std::path::PathBuf;
use std::process::{Command, Stdio};

const DEBUG_ENV: &str = "date=1774210426000,tz=AEDT+1100";

const DIAGRAMS: &[(&str, &str)] = &[
    (
        "sequence",
        "@startuml\nAlice -> Bob: hello\nBob --> Alice: ok\n@enduml\n",
    ),
    (
        "class",
        "@startuml\nclass Gauge\nclass Vista\nVista --|> Gauge\n@enduml\n",
    ),
    (
        "activity",
        "@startuml\nstart\n:measure the span;\nstop\n@enduml\n",
    ),
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn render_pdf(source: &str) -> Vec<u8> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rustuml"))
        .args(["-pipe", "-tpdf"])
        .env("RUSTUML_DEBUG", DEBUG_ENV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rustuml");
    std::io::Write::write_all(child.stdin.as_mut().unwrap(), source.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "pdf export failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// Header, classic xref, one page, and the same bytes on a second run.
fn assert_pdf(label: &str, pdf: &[u8]) {
    assert!(pdf.starts_with(b"%PDF-"), "{label}: missing PDF header");
    assert!(
        pdf.ends_with(b"%%EOF\n") || pdf.ends_with(b"%%EOF"),
        "{label}: missing %%EOF"
    );
    let text = String::from_utf8_lossy(pdf);
    let startxref = text
        .rfind("startxref\n")
        .unwrap_or_else(|| panic!("{label}: no startxref"));
    let offset: usize = text[startxref + "startxref\n".len()..]
        .lines()
        .next()
        .unwrap()
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("{label}: startxref is not an offset"));
    assert!(
        pdf.get(offset..)
            .is_some_and(|bytes| bytes.starts_with(b"xref\n")),
        "{label}: startxref {offset} does not land on the xref table"
    );
    let pages = text.matches("/Type /Pages").count();
    let page = text.matches("/Type /Page\n").count() + text.matches("/Type /Page ").count();
    assert_eq!(pages, 1, "{label}: page tree count");
    assert_eq!(page, 1, "{label}: page object count");
    let count_at = text.find("/Type /Pages").unwrap();
    let count = text[count_at..]
        .split_once("/Count ")
        .unwrap_or_else(|| panic!("{label}: page tree has no /Count"))
        .1;
    let count_value: usize = count
        .split(|ch: char| !ch.is_ascii_digit())
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(count_value, 1, "{label}: /Count");
}

#[test]
fn pdf_is_valid_and_deterministic_for_a_few_diagrams() {
    for (label, source) in DIAGRAMS {
        let first = render_pdf(source);
        let second = render_pdf(source);
        assert_pdf(label, &first);
        assert_eq!(first, second, "{label}: two runs differ");
    }
}

#[test]
fn parity_tiers_do_not_count_pdf_as_java_backed() {
    let root = repo_root();
    let tiers = [
        "crates/rustuml-oracle/src/runner.rs",
        "crates/rustuml-oracle/tests/golden_pairs.rs",
        "crates/rustuml-oracle/tests/golden_no_oracle.rs",
        "crates/rustuml-oracle/tests/parity_perturbations.rs",
    ];
    for relative in tiers {
        let text = std::fs::read_to_string(root.join(relative)).unwrap();
        assert!(!text.contains("-tpdf"), "{relative} asks for PDF");
        assert!(!text.contains("svg_to_pdf"), "{relative} converts PDF");
        assert!(
            !text.contains("FileFormat.PDF"),
            "{relative} treats PDF as a Java format"
        );
    }
    let runner = std::fs::read_to_string(root.join("crates/rustuml-oracle/src/runner.rs")).unwrap();
    assert!(runner.contains(r#"render(input, &["-tsvg"])"#));

    let formats =
        std::fs::read_to_string(root.join("crates/rustuml-oracle/tests/golden_formats.rs"))
            .unwrap();
    assert!(formats.contains("do **not** compare output against golden"));
    assert!(formats.contains("cfg(feature = \"format-tests\")"));

    let policy = std::fs::read_to_string(root.join("docs/reference-bootstrap.md")).unwrap();
    assert!(policy.contains("deliberate extension"));
    assert!(policy.contains("TitledDiagram.java"));
    assert!(policy.contains("UnsupportedOperationException"));
    assert!(policy.contains("b04ab259a343ba51a50db8d1df637160f31d342a66c4192e18dba63ea42ed04a"));
    assert!(policy.contains("Do not `brew upgrade`"));
}
