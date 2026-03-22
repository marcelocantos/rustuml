// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram feature matrix.

use super::MatrixCase;

/// Deployment diagram edge cases.
pub fn edge_cases() -> Vec<MatrixCase> {
    vec![
        MatrixCase {
            name: "deploy/edge/single_node".into(),
            source: "@startuml\nnode Server\n@enduml\n".into(),
            tags: vec!["edge", "deployment", "deployment:single"],
            expected_texts: vec!["Server".into()],
        },
        MatrixCase {
            name: "deploy/edge/all_node_types".into(),
            source: "@startuml\nnode N\nartifact A\ncloud C\ndatabase D\nstorage S\nframe F\nfolder Fo\nN --> A\nC --> D\nS --> F\n@enduml\n".into(),
            tags: vec!["edge", "deployment", "deployment:types"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "deploy/edge/nested".into(),
            source: "@startuml\nnode \"Web Server\" as ws {\n  artifact app\n}\ndatabase DB\nws --> DB\n@enduml\n".into(),
            tags: vec!["edge", "deployment", "deployment:nested"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "deploy/edge/cloud_topology".into(),
            source: "@startuml\ncloud Internet\nnode LB\nnode App1\nnode App2\ndatabase DB\nInternet --> LB\nLB --> App1\nLB --> App2\nApp1 --> DB\nApp2 --> DB\n@enduml\n".into(),
            tags: vec!["edge", "deployment", "deployment:topology"],
            expected_texts: vec!["Internet".into()],
        },
        MatrixCase {
            name: "deploy/edge/labeled_connections".into(),
            source: "@startuml\nnode A\nnode B\nA --> B : HTTPS\nA --> B : SSH\n@enduml\n".into(),
            tags: vec!["edge", "deployment", "deployment:labels"],
            expected_texts: vec!["HTTPS".into()],
        },
    ]
}
