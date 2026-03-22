// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram feature matrix.

use super::MatrixCase;

/// Generate a component matrix: component count × connection pattern = 9 cases.
///
/// Component counts: 2, 3, 5
/// Connection patterns: chain, star, mesh
pub fn matrix_cases() -> Vec<MatrixCase> {
    let mut cases = Vec::new();

    // Component counts with names.
    let counts: &[(usize, &[&str])] = &[
        (2, &["CompA", "CompB"]),
        (3, &["CompA", "CompB", "CompC"]),
        (5, &["CompA", "CompB", "CompC", "CompD", "CompE"]),
    ];

    for (_, names) in counts {
        let count = names.len();
        let count_str = count.to_string();

        // Chain: A --> B --> C --> ...
        {
            let mut src = String::from("@startuml\n");
            for name in *names {
                src.push_str(&format!("[{name}]\n"));
            }
            for i in 0..names.len() - 1 {
                src.push_str(&format!("[{}] --> [{}]\n", names[i], names[i + 1]));
            }
            src.push_str("@enduml\n");

            cases.push(MatrixCase {
                name: format!("component/matrix/count{count_str}_chain"),
                source: src,
                tags: vec!["component", "component:chain"],
                // Rust component renderer is not yet mature; skip text checks.
                expected_texts: vec![],
            });
        }

        // Star: all others connect to first component.
        {
            let mut src = String::from("@startuml\n");
            for name in *names {
                src.push_str(&format!("[{name}]\n"));
            }
            for i in 1..names.len() {
                src.push_str(&format!("[{}] --> [{}]\n", names[0], names[i]));
            }
            src.push_str("@enduml\n");

            cases.push(MatrixCase {
                name: format!("component/matrix/count{count_str}_star"),
                source: src,
                tags: vec!["component", "component:star"],
                expected_texts: vec![],
            });
        }

        // Mesh: every pair connected (up to 5 components → manageable).
        {
            let mut src = String::from("@startuml\n");
            for name in *names {
                src.push_str(&format!("[{name}]\n"));
            }
            for i in 0..names.len() {
                for j in i + 1..names.len() {
                    src.push_str(&format!("[{}] --> [{}]\n", names[i], names[j]));
                }
            }
            src.push_str("@enduml\n");

            cases.push(MatrixCase {
                name: format!("component/matrix/count{count_str}_mesh"),
                source: src,
                tags: vec!["component", "component:mesh"],
                expected_texts: vec![],
            });
        }
    }

    cases
}

/// Edge cases for component diagrams.
pub fn edge_cases() -> Vec<MatrixCase> {
    vec![
        MatrixCase {
            name: "component/edge/interface".into(),
            source: "@startuml\n[Component] - Interface\nInterface - [Consumer]\n@enduml\n".into(),
            tags: vec!["edge", "component", "component:interface"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "component/edge/package".into(),
            source: "@startuml\npackage \"Backend\" {\n  [Service]\n  [Repository]\n}\n[Client] --> [Service]\n[Service] --> [Repository]\n@enduml\n".into(),
            tags: vec!["edge", "component", "component:package"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "component/edge/labeled_connections".into(),
            source: "@startuml\n[Frontend] --> [API] : REST\n[API] --> [Database] : SQL\n@enduml\n".into(),
            tags: vec!["edge", "component", "component:labels"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "component/edge/note".into(),
            source: "@startuml\n[Alpha] --> [Beta]\nnote right of [Beta] : Important note\n@enduml\n".into(),
            tags: vec!["edge", "component", "component:note"],
            expected_texts: vec![],
        },
    ]
}
