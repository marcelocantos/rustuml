// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram feature matrix.

use super::MatrixCase;

/// Activity diagram edge cases.
pub fn edge_cases() -> Vec<MatrixCase> {
    vec![
        MatrixCase {
            name: "activity/edge/start_stop_only".into(),
            source: "@startuml\nstart\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:minimal"],
            expected_texts: vec![],
        },
        MatrixCase {
            name: "activity/edge/single_action".into(),
            source: "@startuml\nstart\n:Hello;\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:single"],
            expected_texts: vec!["Hello".into()],
        },
        MatrixCase {
            name: "activity/edge/many_actions".into(),
            source: {
                let mut s = String::from("@startuml\nstart\n");
                for i in 1..=15 {
                    s.push_str(&format!(":Step {i};\n"));
                }
                s.push_str("stop\n@enduml\n");
                s
            },
            tags: vec!["edge", "activity", "activity:many"],
            expected_texts: vec!["Step 1".into(), "Step 15".into()],
        },
        MatrixCase {
            name: "activity/edge/nested_if".into(),
            source: "@startuml\nstart\nif (a?) then (yes)\n  if (b?) then (yes)\n    :deep;\n  else (no)\n    :other;\n  endif\nelse (no)\n  :skip;\nendif\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:nested_if"],
            expected_texts: vec!["deep".into()],
        },
        MatrixCase {
            name: "activity/edge/elseif_chain".into(),
            source: "@startuml\nstart\nif (x > 0?) then (positive)\n  :pos;\nelseif (x < 0?) then (negative)\n  :neg;\nelseif (x = 0?) then (zero)\n  :zero;\nelse (unknown)\n  :err;\nendif\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:elseif"],
            expected_texts: vec!["pos".into(), "neg".into()],
        },
        MatrixCase {
            name: "activity/edge/while_loop".into(),
            source: "@startuml\nstart\nwhile (has more?) is (yes)\n  :process item;\nendwhile (no)\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:while"],
            expected_texts: vec!["process item".into()],
        },
        MatrixCase {
            name: "activity/edge/repeat_loop".into(),
            source: "@startuml\nstart\nrepeat\n  :action;\nrepeat while (again?) is (yes) not (no)\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:repeat"],
            expected_texts: vec!["action".into()],
        },
        MatrixCase {
            name: "activity/edge/fork_three".into(),
            source: "@startuml\nstart\nfork\n  :Task A;\nfork again\n  :Task B;\nfork again\n  :Task C;\nend fork\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:fork"],
            expected_texts: vec!["Task A".into(), "Task B".into(), "Task C".into()],
        },
        MatrixCase {
            name: "activity/edge/split_three".into(),
            source: "@startuml\nstart\nsplit\n  :Path A;\nsplit again\n  :Path B;\nsplit again\n  :Path C;\nend split\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:split"],
            expected_texts: vec!["Path A".into(), "Path B".into()],
        },
        MatrixCase {
            name: "activity/edge/switch_many".into(),
            source: "@startuml\nstart\nswitch (choice?)\ncase ( A )\n  :action A;\ncase ( B )\n  :action B;\ncase ( C )\n  :action C;\nendswitch\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:switch"],
            expected_texts: vec!["action A".into()],
        },
        MatrixCase {
            name: "activity/edge/swimlanes".into(),
            source: "@startuml\n|Lane1|\nstart\n:task1;\n|Lane2|\n:task2;\n|Lane3|\n:task3;\n|Lane1|\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:swimlanes"],
            expected_texts: vec!["task1".into(), "task2".into()],
        },
        MatrixCase {
            name: "activity/edge/partition".into(),
            source: "@startuml\nstart\npartition Init {\n  :step1;\n  :step2;\n}\npartition Process {\n  :step3;\n}\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:partition"],
            expected_texts: vec!["step1".into()],
        },
        MatrixCase {
            name: "activity/edge/detach".into(),
            source: "@startuml\nstart\nif (error?) then (yes)\n  :log;\n  detach\nelse (no)\n  :continue;\nendif\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:detach"],
            expected_texts: vec!["log".into()],
        },
        MatrixCase {
            name: "activity/edge/complex_flow".into(),
            source: "@startuml\nstart\n:Init;\nif (valid?) then (yes)\n  fork\n    :Process A;\n  fork again\n    :Process B;\n  end fork\n  :Merge;\nelse (no)\n  :Error;\n  detach\nendif\nwhile (more?) is (yes)\n  :Iterate;\nendwhile (no)\nstop\n@enduml\n".into(),
            tags: vec!["edge", "activity", "activity:complex"],
            expected_texts: vec!["Init".into(), "Merge".into()],
        },
    ]
}
