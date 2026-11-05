// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
use rustuml_parser::diagram::sequence::{Event, SequenceDiagram};
fn main() {
    let lines = ["& A -> B ++ : enter", "& return leave"].map(str::to_owned);
    let diagram = rustuml_parser::parse::sequence::parse_sequence(&lines).unwrap();
    let mut value = serde_json::to_value(&diagram).unwrap();
    for event in value["events"].as_array_mut().unwrap() {
        for kind in ["Message", "Return"] {
            if let Some(fields) = event.get_mut(kind) {
                assert_eq!(fields["parallel"], true);
                fields.as_object_mut().unwrap().remove("parallel");
            }
        }
    }
    let old: SequenceDiagram = serde_json::from_value(value).unwrap();
    assert!(matches!(&old.events[0], Event::Message(m) if !m.parallel));
    assert!(matches!(&old.events[1], Event::Return(m) if !m.parallel));
    println!("Message.parallel and ReturnMessage.parallel preserve true and default absent fields to false");
}
