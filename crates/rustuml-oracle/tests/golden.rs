// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Golden file tests — compare Rust rendering against pre-generated
//! Java PlantUML reference output.
//!
//! These tests don't require a running PlantUML server. The golden
//! files are generated once with `scripts/generate-golden.sh` and
//! committed to the repository.

use rustuml_oracle::compare;
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-fixtures/golden")
}

/// Load a golden SVG file.
fn load_golden(name: &str) -> Option<String> {
    let path = golden_dir().join(format!("{name}.svg"));
    std::fs::read_to_string(&path).ok()
}

/// Render a PlantUML input to SVG using our Rust renderer.
fn render_rust(input: &str) -> String {
    let diagram = rustuml_parser::parse::parse(input).expect("parse failed");
    rustuml_render::render_svg(&diagram)
}

/// Compare structural elements between golden and Rust SVGs.
/// We don't expect pixel-identical output, but we expect the same
/// semantic content (participant names, message labels, etc.).
fn assert_content_matches(golden_svg: &str, rust_svg: &str, name: &str) {
    let golden_elems = compare::extract_elements(golden_svg).expect("golden SVG parse failed");
    let rust_elems = compare::extract_elements(rust_svg).expect("rust SVG parse failed");

    // Extract text content from both.
    let golden_texts: Vec<&str> = golden_elems
        .iter()
        .filter_map(|e| e.text.as_deref())
        .filter(|t| !t.is_empty())
        .collect();
    let rust_texts: Vec<&str> = rust_elems
        .iter()
        .filter_map(|e| e.text.as_deref())
        .filter(|t| !t.is_empty())
        .collect();

    // Core content texts (participant names, message labels) should appear
    // in both. We skip very short texts (like "1", "2" from autonumber)
    // and structural labels (like "alt", "else") since these are
    // rendering details that may differ.
    let skip = |t: &&str| {
        t.len() < 2
            || ["alt", "else", "opt", "loop", "end", "par", "ref"].contains(t)
            || t.starts_with('[') // Group labels like [success]
    };

    let missing: Vec<&&str> = golden_texts
        .iter()
        .filter(|t| !skip(t))
        .filter(|t| !rust_texts.iter().any(|r| r.contains(**t)))
        .collect();

    assert!(
        missing.is_empty(),
        "[{name}] golden texts not found in Rust output: {missing:?}\n\
         Golden texts: {golden_texts:?}\n\
         Rust texts: {rust_texts:?}"
    );
}

macro_rules! golden_test {
    ($name:ident, $path:expr, $input:expr) => {
        #[test]
        fn $name() {
            if let Some(golden) = load_golden($path) {
                let rust = render_rust($input);
                assert_content_matches(&golden, &rust, $path);
            }
            // Skip if golden file doesn't exist (not yet generated).
        }
    };
}

golden_test!(
    seq_simple,
    "sequence/simple",
    "@startuml\nAlice -> Bob : hello\n@enduml"
);

golden_test!(
    seq_multi_message,
    "sequence/multi_message",
    "@startuml\nAlice -> Bob : message 1\nBob --> Alice : message 2\nAlice -> Bob : message 3\n@enduml"
);

golden_test!(
    seq_participant_types,
    "sequence/participant_types",
    "@startuml\nparticipant P\nactor A\nboundary B\ncontrol C\nentity E\ndatabase D\ncollections Co\nqueue Q\nP -> A : msg\n@enduml"
);

golden_test!(
    seq_notes,
    "sequence/notes",
    "@startuml\nA -> B : msg\nnote left : left note\nnote right : right note\nnote over A : over A\nnote over A, B : over both\n@enduml"
);

golden_test!(
    seq_alt_else,
    "sequence/alt_else",
    "@startuml\nA -> B : check\nalt success\n  B --> A : ok\nelse failure\n  B --> A : error\nend\n@enduml"
);

golden_test!(
    seq_autonumber,
    "sequence/autonumber",
    "@startuml\nautonumber\nA -> B : first\nB -> C : second\nC --> B : third\n@enduml"
);

golden_test!(
    class_basic,
    "class/basic",
    "@startuml\nclass Animal {\n  +name : String\n  +makeSound() : void\n}\nclass Dog extends Animal {\n  +fetch() : void\n}\nAnimal <|-- Dog\n@enduml"
);

golden_test!(
    class_enum,
    "class/enum",
    "@startuml\nenum Color {\n  RED\n  GREEN\n  BLUE\n}\n@enduml"
);

golden_test!(
    state_basic,
    "state/basic",
    "@startuml\n[*] --> Active\nActive --> Inactive : disable\nInactive --> Active : enable\nActive --> [*] : close\n@enduml"
);

golden_test!(
    activity_basic,
    "activity/basic",
    "@startuml\nstart\n:Step 1;\nif (condition?) then (yes)\n  :Step 2a;\nelse (no)\n  :Step 2b;\nendif\nstop\n@enduml"
);
