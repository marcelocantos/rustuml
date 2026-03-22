// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram feature matrix.

use super::MatrixCase;

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
