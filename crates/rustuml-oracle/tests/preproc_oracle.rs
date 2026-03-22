// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Preprocessor oracle tests — compare our TIM preprocessor output
//! against Java PlantUML's -preproc output.

use rustuml_oracle::runner;

/// Compare our preprocessor output against Java PlantUML.
/// Requires PLANTUML_JAR to be set (skipped if not available).
fn compare_preproc(input: &str) -> Option<(String, String)> {
    let jar = match runner::find_jar() {
        Ok(j) => j,
        Err(_) => return None, // Skip if JAR not found.
    };

    let java_output = match runner::run_preproc(&jar, input) {
        Ok(o) => o,
        Err(_) => return None,
    };

    let lines = rustuml_parser::preprocess::preprocess(input);
    let rust_output = format!("@startuml\n{}\n@enduml", lines.join("\n"));

    Some((
        java_output.trim().to_string(),
        rust_output.trim().to_string(),
    ))
}

#[test]
fn simple_passthrough() {
    let input = "@startuml\nAlice -> Bob : hello\n@enduml";
    if let Some((java, rust)) = compare_preproc(input) {
        assert_eq!(java, rust, "simple passthrough should match");
    }
}

#[test]
fn define_substitution() {
    let input = "@startuml\n!define NAME Alice\n$NAME -> Bob : hello\n@enduml";
    if let Some((java, rust)) = compare_preproc(input) {
        assert!(java.contains("Alice -> Bob"), "Java should expand: {java}");
        assert!(rust.contains("Alice -> Bob"), "Rust should expand: {rust}");
    }
}

#[test]
fn ifdef_defined() {
    let input = "@startuml\n!define FEATURE\n!ifdef FEATURE\nAlice -> Bob : yes\n!endif\n@enduml";
    if let Some((java, rust)) = compare_preproc(input) {
        assert!(java.contains("Alice -> Bob"), "Java: {java}");
        assert!(rust.contains("Alice -> Bob"), "Rust: {rust}");
    }
}

#[test]
fn ifdef_not_defined() {
    let input = "@startuml\n!ifdef MISSING\nAlice -> Bob : no\n!endif\nBob -> Alice : yes\n@enduml";
    if let Some((java, rust)) = compare_preproc(input) {
        assert!(
            !java.contains("Alice -> Bob : no"),
            "Java should skip: {java}"
        );
        assert!(
            !rust.contains("Alice -> Bob : no"),
            "Rust should skip: {rust}"
        );
        assert!(
            java.contains("Bob -> Alice : yes"),
            "Java should include: {java}"
        );
        assert!(
            rust.contains("Bob -> Alice : yes"),
            "Rust should include: {rust}"
        );
    }
}

#[test]
fn comments_stripped() {
    let input = "@startuml\n' This is a comment\nAlice -> Bob : hello\n@enduml";
    if let Some((java, rust)) = compare_preproc(input) {
        assert!(!java.contains("This is a comment"), "Java: {java}");
        assert!(!rust.contains("This is a comment"), "Rust: {rust}");
        assert!(java.contains("Alice -> Bob"), "Java: {java}");
        assert!(rust.contains("Alice -> Bob"), "Rust: {rust}");
    }
}
