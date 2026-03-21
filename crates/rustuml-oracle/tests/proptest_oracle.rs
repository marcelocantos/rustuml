// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Property-based oracle tests using proptest.
//!
//! Generates random valid PlantUML diagrams and verifies the Java
//! PlantUML server produces valid, self-consistent SVG output.

use std::fmt::Write;

use proptest::prelude::*;
use rustuml_oracle::compare;
use rustuml_oracle::runner;

// ---------------------------------------------------------------------------
// Strategies for generating random PlantUML diagrams
// ---------------------------------------------------------------------------

fn participant_name() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Heidi",
    ])
    .prop_map(|s| s.to_string())
}

fn arrow_style() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["->", "-->", "->>", "-->>", "->x", "->o", "<->"])
}

fn message_label() -> impl Strategy<Value = String> {
    "[a-zA-Z ]{1,20}".prop_map(|s| s.trim().to_string())
}

fn sequence_diagram_strategy() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(participant_name(), 2..=5),
        prop::collection::vec(
            (0..8usize, 0..8usize, arrow_style(), message_label()),
            1..=15,
        ),
        proptest::bool::ANY,
        proptest::bool::ANY,
        proptest::bool::ANY,
    )
        .prop_map(
            |(participants, messages, divider, note, autonumber): (
                Vec<String>,
                Vec<(usize, usize, &str, String)>,
                bool,
                bool,
                bool,
            )| {
                let participants: Vec<String> = participants
                    .into_iter()
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
                let n = participants.len();
                if n < 2 {
                    return "@startuml\nAlice -> Bob : fallback\n@enduml\n".to_string();
                }

                let mut body = String::new();
                if autonumber {
                    writeln!(body, "autonumber").unwrap();
                }
                for p in &participants {
                    writeln!(body, "participant {p}").unwrap();
                }

                for (i, (from_raw, to_raw, arrow, label)) in messages.iter().enumerate() {
                    let from_i = from_raw % n;
                    let mut to_i = to_raw % n;
                    if to_i == from_i {
                        to_i = (to_i + 1) % n;
                    }
                    let from = &participants[from_i];
                    let to = &participants[to_i];
                    writeln!(body, "{from} {arrow} {to} : {label}").unwrap();

                    if divider && i == messages.len() / 2 {
                        writeln!(body, "== Phase 2 ==").unwrap();
                    }
                    if note && i == 0 {
                        writeln!(body, "note right : auto note").unwrap();
                    }
                }

                format!("@startuml\n{body}@enduml\n")
            },
        )
}

fn class_name() -> impl Strategy<Value = String> {
    "[A-Z][a-z]{2,8}".prop_map(|s| s.to_string())
}

fn visibility() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["+", "-", "#", "~"])
}

fn member_name() -> impl Strategy<Value = String> {
    "[a-z][a-zA-Z]{2,10}".prop_map(|s| s.to_string())
}

fn type_name() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "String", "int", "boolean", "double", "void", "List", "Map",
    ])
}

fn relationship() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["<|--", "..|>", "*--", "o--", "--", "..>"])
}

fn class_diagram_strategy() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(
            (
                class_name(),
                prop::collection::vec((visibility(), member_name(), type_name()), 0..=4),
            ),
            2..=6,
        ),
        prop::collection::vec((0..6usize, 0..6usize, relationship()), 0..=4),
    )
        .prop_map(
            |(classes, rels): (
                Vec<(String, Vec<(&str, String, &str)>)>,
                Vec<(usize, usize, &str)>,
            )| {
                let mut body = String::new();
                let n = classes.len();

                for (name, members) in &classes {
                    writeln!(body, "class {name} {{").unwrap();
                    for (vis, mname, mtype) in members {
                        writeln!(body, "  {vis}{mname} : {mtype}").unwrap();
                    }
                    writeln!(body, "}}").unwrap();
                }

                let names: Vec<&str> = classes.iter().map(|(name, _)| name.as_str()).collect();
                for (from_raw, to_raw, rel) in &rels {
                    let from_i = from_raw % n;
                    let mut to_i = to_raw % n;
                    if to_i == from_i {
                        to_i = (to_i + 1) % n;
                    }
                    let from = names[from_i];
                    let to = names[to_i];
                    writeln!(body, "{from} {rel} {to}").unwrap();
                }

                format!("@startuml\n{body}@enduml\n")
            },
        )
}

fn state_name() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "Idle", "Active", "Running", "Stopped", "Paused", "Error", "Ready", "Done",
    ])
    .prop_map(|s| s.to_string())
}

fn state_diagram_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(state_name(), 2..=6).prop_map(|states: Vec<String>| {
        let states: Vec<String> = states
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let n = states.len();
        if n < 2 {
            return "@startuml\n[*] --> Active\nActive --> [*]\n@enduml\n".to_string();
        }

        let mut body = String::new();
        writeln!(body, "[*] --> {}", states[0]).unwrap();
        for i in 0..n - 1 {
            writeln!(body, "{} --> {} : next", states[i], states[i + 1]).unwrap();
        }
        writeln!(body, "{} --> [*]", states[n - 1]).unwrap();

        format!("@startuml\n{body}@enduml\n")
    })
}

fn action_text() -> impl Strategy<Value = String> {
    "[A-Z][a-z ]{2,15}".prop_map(|s| s.trim().to_string())
}

fn activity_diagram_strategy() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(action_text(), 2..=8),
        proptest::bool::ANY,
        proptest::bool::ANY,
    )
        .prop_map(|(actions, condition, fork): (Vec<String>, bool, bool)| {
            let mut body = String::from("start\n");

            if condition && actions.len() >= 3 {
                writeln!(body, ":{};\n", actions[0]).unwrap();
                writeln!(body, "if (check?) then (yes)").unwrap();
                writeln!(body, "  :{};\n", actions[1]).unwrap();
                writeln!(body, "else (no)").unwrap();
                writeln!(body, "  :{};\n", actions[2]).unwrap();
                writeln!(body, "endif").unwrap();
                for a in &actions[3..] {
                    writeln!(body, ":{a};").unwrap();
                }
            } else if fork && actions.len() >= 2 {
                writeln!(body, "fork").unwrap();
                writeln!(body, "  :{};\n", actions[0]).unwrap();
                writeln!(body, "fork again").unwrap();
                writeln!(body, "  :{};\n", actions[1]).unwrap();
                writeln!(body, "end fork").unwrap();
                for a in &actions[2..] {
                    writeln!(body, ":{a};").unwrap();
                }
            } else {
                for a in &actions {
                    writeln!(body, ":{a};").unwrap();
                }
            }

            writeln!(body, "stop").unwrap();
            format!("@startuml\n{body}@enduml\n")
        })
}

// ---------------------------------------------------------------------------
// Property tests — 256 cases per diagram type = 1280 total
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn random_sequence_diagrams_produce_valid_svg(input in sequence_diagram_strategy()) {
        let svg = runner::render_svg(&input)
            .map_err(|e| TestCaseError::Fail(format!("render failed: {e}").into()))?;
        prop_assert!(svg.starts_with("<svg"), "output should be SVG");
        let _ = compare::extract_elements(&svg)
            .map_err(|e| TestCaseError::Fail(format!("parse failed: {e}").into()))?;
    }

    #[test]
    fn random_class_diagrams_produce_valid_svg(input in class_diagram_strategy()) {
        let svg = runner::render_svg(&input)
            .map_err(|e| TestCaseError::Fail(format!("render failed: {e}").into()))?;
        prop_assert!(svg.starts_with("<svg"), "output should be SVG");
        let _ = compare::extract_elements(&svg)
            .map_err(|e| TestCaseError::Fail(format!("parse failed: {e}").into()))?;
    }

    #[test]
    fn random_state_diagrams_produce_valid_svg(input in state_diagram_strategy()) {
        let svg = runner::render_svg(&input)
            .map_err(|e| TestCaseError::Fail(format!("render failed: {e}").into()))?;
        prop_assert!(svg.starts_with("<svg"), "output should be SVG");
        let _ = compare::extract_elements(&svg)
            .map_err(|e| TestCaseError::Fail(format!("parse failed: {e}").into()))?;
    }

    #[test]
    fn random_activity_diagrams_produce_valid_svg(input in activity_diagram_strategy()) {
        let svg = runner::render_svg(&input)
            .map_err(|e| TestCaseError::Fail(format!("render failed: {e}").into()))?;
        prop_assert!(svg.starts_with("<svg"), "output should be SVG");
        let _ = compare::extract_elements(&svg)
            .map_err(|e| TestCaseError::Fail(format!("parse failed: {e}").into()))?;
    }

    #[test]
    fn random_sequence_diagrams_are_deterministic(input in sequence_diagram_strategy()) {
        let svg1 = runner::render_svg(&input)
            .map_err(|e| TestCaseError::Fail(format!("render 1 failed: {e}").into()))?;
        let svg2 = runner::render_svg(&input)
            .map_err(|e| TestCaseError::Fail(format!("render 2 failed: {e}").into()))?;
        let result = compare::compare_svg(&svg1, &svg2)
            .map_err(|e| TestCaseError::Fail(format!("compare failed: {e}").into()))?;
        prop_assert!(result.is_match(), "identical inputs should produce identical SVG: {}", result);
    }
}
