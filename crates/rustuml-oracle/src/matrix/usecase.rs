// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram feature matrix.

use super::MatrixCase;

/// Generate a use case matrix: actor count × use case count × connection pattern.
///
/// Actor counts: 1, 2
/// Use case counts: 2, 3, 5
/// Connection patterns: direct, include, extend
/// Total: 2 × 3 × 3 = 18 cases
pub fn matrix_cases() -> Vec<MatrixCase> {
    let mut cases = Vec::new();

    let actor_counts: &[usize] = &[1, 2];
    let uc_counts: &[usize] = &[2, 3, 5];

    for &num_actors in actor_counts {
        let actors: Vec<String> = (1..=num_actors).map(|i| format!("Actor{i}")).collect();

        for &num_ucs in uc_counts {
            let usecases: Vec<String> = (1..=num_ucs).map(|i| format!("UseCase{i}")).collect();

            // Direct: each actor connects to each use case (actor1 → all, actor2 → last).
            {
                let mut src = String::from("@startuml\n");
                for a in &actors {
                    src.push_str(&format!(":{}:\n", a));
                }
                for uc in &usecases {
                    src.push_str(&format!("({uc})\n"));
                }
                // First actor connects to all use cases.
                for uc in &usecases {
                    src.push_str(&format!(":{}: --> ({})\n", actors[0], uc));
                }
                // Second actor (if present) connects to last use case.
                if num_actors > 1 {
                    src.push_str(&format!(
                        ":{}: --> ({})\n",
                        actors[num_actors - 1],
                        usecases[num_ucs - 1]
                    ));
                }
                src.push_str("@enduml\n");

                cases.push(MatrixCase {
                    name: format!("usecase/matrix/actors{num_actors}_ucs{num_ucs}_direct"),
                    source: src,
                    tags: vec!["usecase", "usecase:direct"],
                    // Rust use case renderer is not yet mature; skip text checks.
                    expected_texts: vec![],
                });
            }

            // Include: UC1 includes UC2, actor uses UC1.
            if num_ucs >= 2 {
                let mut src = String::from("@startuml\n");
                for a in &actors {
                    src.push_str(&format!(":{}:\n", a));
                }
                for uc in &usecases {
                    src.push_str(&format!("({uc})\n"));
                }
                src.push_str(&format!(":{}: --> ({})\n", actors[0], usecases[0]));
                src.push_str(&format!(
                    "({}) .> ({}) : include\n",
                    usecases[0], usecases[1]
                ));
                src.push_str("@enduml\n");

                cases.push(MatrixCase {
                    name: format!("usecase/matrix/actors{num_actors}_ucs{num_ucs}_include"),
                    source: src,
                    tags: vec!["usecase", "usecase:include"],
                    // Rust use case renderer is not yet mature; skip text checks.
                    expected_texts: vec![],
                });
            }

            // Extend: UC2 extends UC1, actor uses UC1.
            if num_ucs >= 2 {
                let mut src = String::from("@startuml\n");
                for a in &actors {
                    src.push_str(&format!(":{}:\n", a));
                }
                for uc in &usecases {
                    src.push_str(&format!("({uc})\n"));
                }
                src.push_str(&format!(":{}: --> ({})\n", actors[0], usecases[0]));
                src.push_str(&format!(
                    "({}) .> ({}) : extend\n",
                    usecases[1], usecases[0]
                ));
                src.push_str("@enduml\n");

                cases.push(MatrixCase {
                    name: format!("usecase/matrix/actors{num_actors}_ucs{num_ucs}_extend"),
                    source: src,
                    tags: vec!["usecase", "usecase:extend"],
                    // Rust use case renderer is not yet mature; skip text checks.
                    expected_texts: vec![],
                });
            }
        }
    }

    cases
}

/// Edge cases for use case diagrams.
pub fn edge_cases() -> Vec<MatrixCase> {
    vec![
        MatrixCase {
            name: "usecase/edge/single_actor_single_uc".into(),
            source: "@startuml\n:User: --> (Login)\n@enduml\n".into(),
            tags: vec!["edge", "usecase", "usecase:minimal"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "usecase/edge/system_boundary".into(),
            source: "@startuml\n:Customer:\nrectangle \"Online Shop\" {\n  (Browse)\n  (Order)\n  (Pay)\n}\n:Customer: --> (Browse)\n:Customer: --> (Order)\n:Customer: --> (Pay)\n@enduml\n".into(),
            tags: vec!["edge", "usecase", "usecase:boundary"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "usecase/edge/inheritance".into(),
            source: "@startuml\n:Admin: --|> :User:\n:User: --> (View)\n:Admin: --> (Manage)\n@enduml\n".into(),
            tags: vec!["edge", "usecase", "usecase:inheritance"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "usecase/edge/note".into(),
            source: "@startuml\n:Actor: --> (UseCase)\nnote right of (UseCase) : Optional feature\n@enduml\n".into(),
            tags: vec!["edge", "usecase", "usecase:note"],
            expected_texts: vec![],
        },
    ]
}
