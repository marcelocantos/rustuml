// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram feature matrix.

use super::MatrixCase;

/// State diagram quick combinatorial matrix.
///
/// Generates 20 cases: 5 state kinds × 4 transition patterns.
pub fn quick_cases() -> Vec<MatrixCase> {
    // (kind_name, state_decls, primary_renders_label, kind_tags)
    //
    // `primary_renders_label` is false for symbol stereotypes (<<choice>>,
    // <<fork>>, <<join>>) because PlantUML renders them as graphical symbols
    // without a visible text label for the state name.
    let state_kinds: &[(&str, &str, bool, &[&str])] = &[
        (
            "normal",
            "state Active\nstate Idle",
            true,
            &["state:normal"],
        ),
        (
            "with_desc",
            "state Active\nActive : entry / init\nActive : do / run\nstate Idle",
            true,
            &["state:desc"],
        ),
        (
            "stereotype_choice",
            "state Active <<choice>>\nstate Idle",
            false,
            &["state:stereotype", "state:choice"],
        ),
        (
            "stereotype_fork",
            "state Active <<fork>>\nstate Idle",
            false,
            &["state:stereotype", "state:fork"],
        ),
        (
            "stereotype_join",
            "state Active <<join>>\nstate Idle",
            false,
            &["state:stereotype", "state:join"],
        ),
    ];

    // (pattern_name, transition_body, expected_when_label_shown, expected_always)
    //
    // `expected_when_label_shown` texts require the primary state to render its
    // name (e.g. "Active"). `expected_always` texts come from transition labels
    // and appear regardless of state kind.
    let transition_patterns: &[(&str, &str, &[&str], &[&str])] = &[
        (
            "simple",
            "[*] --> Active\nActive --> Idle\nIdle --> [*]",
            &["Active"],
            &["Idle"],
        ),
        (
            "chain",
            "[*] --> Active\nActive --> Idle : step1\nIdle --> Active : step2\nIdle --> [*] : done",
            &[],
            &["step1", "step2"],
        ),
        (
            "cycle",
            "[*] --> Active\nActive --> Idle : pause\nIdle --> Active : resume\nActive --> [*] : end",
            &[],
            &["pause", "resume"],
        ),
        (
            "to_final",
            "[*] --> Active\nActive --> [*] : success\nActive --> Idle : wait\nIdle --> [*] : timeout",
            &[],
            &["success", "timeout"],
        ),
    ];

    let mut cases = Vec::new();

    for (kind_name, state_decls, primary_renders_label, kind_tags) in state_kinds {
        for (pattern_name, transitions, expected_labeled, expected_always) in transition_patterns {
            let source = format!("@startuml\n{state_decls}\n{transitions}\n@enduml\n");

            let mut expected: Vec<String> =
                expected_always.iter().map(|s| (*s).to_string()).collect();
            if *primary_renders_label {
                expected.extend(expected_labeled.iter().map(|s| (*s).to_string()));
            }

            let mut tags = vec!["state", "quick"];
            tags.extend_from_slice(kind_tags);
            tags.push(match *pattern_name {
                "simple" => "state:transition:simple",
                "chain" => "state:transition:chain",
                "cycle" => "state:transition:cycle",
                "to_final" => "state:transition:to_final",
                _ => "state:transition",
            });

            cases.push(MatrixCase {
                name: format!("state/quick/{kind_name}/{pattern_name}"),
                source,
                tags,
                expected_texts: expected,
            });
        }
    }

    cases
}

/// State diagram edge cases.
pub fn edge_cases() -> Vec<MatrixCase> {
    vec![
        MatrixCase {
            name: "state/edge/single_state".into(),
            source: "@startuml\n[*] --> Active\nActive --> [*]\n@enduml\n".into(),
            tags: vec!["edge", "state", "state:single"],
            expected_texts: vec!["Active".into()],
        },
        MatrixCase {
            name: "state/edge/linear_chain".into(),
            source: "@startuml\n[*] --> A\nA --> B\nB --> C\nC --> D\nD --> [*]\n@enduml\n".into(),
            tags: vec!["edge", "state", "state:chain"],
            expected_texts: vec!["A".into(), "D".into()],
        },
        MatrixCase {
            name: "state/edge/cycle".into(),
            source: "@startuml\n[*] --> A\nA --> B\nB --> A : retry\nB --> [*] : done\n@enduml\n".into(),
            tags: vec!["edge", "state", "state:cycle"],
            expected_texts: vec!["retry".into()],
        },
        MatrixCase {
            name: "state/edge/nested_composite".into(),
            source: "@startuml\nstate Running {\n  [*] --> Processing\n  Processing --> Waiting : pause\n  Waiting --> Processing : resume\n}\n[*] --> Running\nRunning --> [*] : shutdown\n@enduml\n".into(),
            tags: vec!["edge", "state", "state:nested"],
            expected_texts: vec!["Running".into()],
        },
        MatrixCase {
            name: "state/edge/concurrent_regions".into(),
            source: "@startuml\nstate Active {\n  state \"Sub1\" as s1\n  state \"Sub2\" as s2\n  --\n  state \"Sub3\" as s3\n}\n[*] --> Active\n@enduml\n".into(),
            tags: vec!["edge", "state", "state:concurrent"],
            expected_texts: vec!["Active".into()],
        },
        MatrixCase {
            name: "state/edge/all_stereotypes".into(),
            source: "@startuml\nstate s1 <<start>>\nstate s2 <<end>>\nstate s3 <<choice>>\nstate s4 <<fork>>\nstate s5 <<join>>\ns1 --> s3\ns3 --> s4 : yes\ns3 --> s2 : no\ns4 --> s5\n@enduml\n".into(),
            tags: vec!["edge", "state", "state:stereotypes"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "state/edge/descriptions".into(),
            source: "@startuml\nstate Active\nActive : entry / init\nActive : do / process\nActive : exit / cleanup\n[*] --> Active\n@enduml\n".into(),
            tags: vec!["edge", "state", "state:descriptions"],
            expected_texts: vec!["Active".into()],
        },
        MatrixCase {
            name: "state/edge/colored_states".into(),
            source: "@startuml\nstate Active #LightGreen\nstate Error #FF0000\n[*] --> Active\nActive --> Error : fail\n@enduml\n".into(),
            tags: vec!["edge", "state", "state:colors"],
            expected_texts: vec!["Active".into(), "Error".into()],
        },
        MatrixCase {
            name: "state/edge/many_states".into(),
            source: {
                let mut s = String::from("@startuml\n[*] --> S0\n");
                for i in 0..10 {
                    s.push_str(&format!("S{i} --> S{}\n", i + 1));
                }
                s.push_str("S10 --> [*]\n@enduml\n");
                s
            },
            tags: vec!["edge", "state", "state:many"],
            expected_texts: vec!["S0".into(), "S10".into()],
        },
        MatrixCase {
            name: "state/edge/notes".into(),
            source: "@startuml\nstate Active\nnote right of Active : Important state\n[*] --> Active\nActive --> [*]\n@enduml\n".into(),
            tags: vec!["edge", "state", "state:notes"],
            expected_texts: vec!["Active".into()],
        },
    ]
}
