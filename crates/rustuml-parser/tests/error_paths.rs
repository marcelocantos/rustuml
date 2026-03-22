// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Error path tests — verify graceful handling of malformed input.

use rustuml_parser::parse;

#[test]
fn empty_input() {
    // Should parse as an empty sequence diagram, not crash.
    let result = parse::parse("");
    assert!(result.is_ok());
}

#[test]
fn just_startuml() {
    let result = parse::parse("@startuml\n@enduml");
    assert!(result.is_ok());
}

#[test]
fn no_start_tag() {
    // Raw content without @startuml should still parse.
    let result = parse::parse("Alice -> Bob : hello");
    assert!(result.is_ok());
}

#[test]
fn unsupported_diagram_type() {
    let result = parse::parse("@startfoo\nbar\n@endfoo");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .message
            .contains("unsupported diagram type")
    );
}

#[test]
fn malformed_arrow() {
    // Invalid arrow syntax should not crash — line is silently ignored.
    let result = parse::parse("@startuml\nAlice @@@ Bob : hello\n@enduml");
    assert!(result.is_ok());
}

#[test]
fn unclosed_group() {
    // Missing "end" — parser should still produce a result.
    let result =
        parse::parse("@startuml\nAlice -> Bob : hi\nalt branch\nBob -> Alice : reply\n@enduml");
    assert!(result.is_ok());
}

#[test]
fn unclosed_note() {
    // Multiline note without endnote — buffered text is lost but no crash.
    let result = parse::parse("@startuml\nAlice -> Bob : hi\nnote left\ntext\n@enduml");
    assert!(result.is_ok());
}

#[test]
fn unicode_everywhere() {
    let result = parse::parse("@startuml\n日本語 -> 中文 : 你好世界\n@enduml");
    assert!(result.is_ok());
}

#[test]
fn very_long_line() {
    let long_label = "x".repeat(10000);
    let input = format!("@startuml\nAlice -> Bob : {long_label}\n@enduml");
    let result = parse::parse(&input);
    assert!(result.is_ok());
}

#[test]
fn many_participants() {
    let mut input = String::from("@startuml\n");
    for i in 0..100 {
        input.push_str(&format!("participant P{i}\n"));
    }
    input.push_str("P0 -> P99 : msg\n@enduml\n");
    let result = parse::parse(&input);
    assert!(result.is_ok());
}

#[test]
fn deeply_nested_groups() {
    let mut input = String::from("@startuml\nAlice -> Bob : hi\n");
    for i in 0..20 {
        input.push_str(&format!("alt level{i}\n"));
    }
    input.push_str("Alice -> Bob : deep\n");
    for _ in 0..20 {
        input.push_str("end\n");
    }
    input.push_str("@enduml\n");
    let result = parse::parse(&input);
    assert!(result.is_ok());
}

#[test]
fn yaml_with_missing_fields() {
    // Minimal YAML that should still parse.
    let yaml = "type: Sequence\ndiagram:\n  meta: {}\n  participants: []\n  events: []\n  autonumber: null";
    let result = parse::parse_yaml(yaml);
    assert!(result.is_ok());
}

#[test]
fn json_with_missing_fields() {
    let json = r#"{"type":"Sequence","diagram":{"meta":{},"participants":[],"events":[],"autonumber":null}}"#;
    let result = parse::parse_json(json);
    assert!(result.is_ok());
}

#[test]
fn invalid_json() {
    let result = parse::parse_json("not json at all");
    assert!(result.is_err());
}

#[test]
fn invalid_yaml() {
    let result = parse::parse_yaml("type: Invalid\ndiagram: {not_a_real_field: true}");
    assert!(result.is_err());
}

#[test]
fn preprocessor_unclosed_ifdef() {
    // Unclosed !ifdef — lines after it are suppressed but no crash.
    let result = parse::parse("@startuml\n!ifdef MISSING\nAlice -> Bob\n@enduml");
    assert!(result.is_ok());
}

#[test]
fn preprocessor_extra_endif() {
    // Extra !endif — silently ignored.
    let result = parse::parse("@startuml\n!endif\nAlice -> Bob : hi\n@enduml");
    assert!(result.is_ok());
}

#[test]
fn class_with_no_members() {
    let result = parse::parse("@startuml\nclass Empty\n@enduml");
    assert!(result.is_ok());
}

#[test]
fn empty_class_body() {
    let result = parse::parse("@startuml\nclass Empty {\n}\n@enduml");
    assert!(result.is_ok());
}
