// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
use rustuml_parser::diagram::sequence::{Arrow, ArrowHead, Event};

fn main() {
    for token in ["->o", "-->>o", "o<--", "o<->o", "o-->>o", "-/o", "o\\--"] {
        let seq = rustuml_parser::parse::sequence::parse_sequence(&[format!("Quartz {token} Finch : retained")]).unwrap();
        let Event::Message(message) = &seq.events[0] else { panic!("expected message") };
        let arrow = &message.arrow;
        assert!(arrow.target_circle, "{token}");
        assert_ne!(arrow.head, ArrowHead::Circle, "{token}");
        let value = serde_json::to_value(arrow).unwrap();
        assert_eq!(value["target_circle"], true);
        let restored: Arrow = serde_json::from_value(value.clone()).unwrap();
        assert!(restored.target_circle);
        assert_eq!(serde_json::to_value(restored).unwrap(), value);
        let mut legacy = value.clone();
        legacy.as_object_mut().unwrap().remove("target_circle");
        let old: Arrow = serde_json::from_value(legacy).unwrap();
        assert!(!old.target_circle);
        assert!(serde_json::to_value(old).unwrap().get("target_circle").is_none());
        println!("{token}\t{}\t{}\t{}", message.from, message.to, value);
    }
}
