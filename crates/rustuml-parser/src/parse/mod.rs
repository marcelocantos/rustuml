// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Diagram parsing — turns preprocessed lines into diagram models.

pub mod activity;
pub mod class;
pub mod sequence;
pub mod state;

use crate::diagram::Diagram;
use crate::preprocess;

/// Parse error with location context.
#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Detect the diagram type from the @start tag.
fn detect_type(input: &str) -> &str {
    for line in input.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@start") {
            let typ = rest.split_whitespace().next().unwrap_or(rest);
            return typ;
        }
    }
    "uml"
}

/// For @startuml, detect the specific UML subtype by scanning for keywords.
fn detect_uml_subtype(lines: &[String]) -> UmlSubtype {
    for line in lines {
        let trimmed = line.trim();
        // State diagram indicators.
        if trimmed.starts_with("[*]")
            || trimmed.contains("--> [*]")
            || (trimmed.starts_with("state ") && !trimmed.contains("<<"))
        {
            return UmlSubtype::State;
        }
        // Activity diagram indicators.
        if trimmed == "start"
            || trimmed == "stop"
            || trimmed.starts_with(':') && trimmed.ends_with(';')
            || trimmed.starts_with("if (")
            || trimmed.starts_with("while (")
            || trimmed.starts_with("switch (")
            || trimmed == "fork"
            || trimmed == "split"
            || trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 2
        {
            return UmlSubtype::Activity;
        }
        // Class diagram indicators.
        if trimmed.starts_with("class ")
            || trimmed.starts_with("abstract class ")
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("annotation ")
            || trimmed.starts_with("entity ")
            || trimmed.starts_with("package ")
            || trimmed.starts_with("together")
            || trimmed.contains("<|--")
            || trimmed.contains("..|>")
            || trimmed.contains("*--")
            || trimmed.contains("o--")
        {
            return UmlSubtype::Class;
        }
        // Sequence indicators.
        if trimmed.starts_with("participant ")
            || trimmed.starts_with("actor ")
            || trimmed.starts_with("boundary ")
            || trimmed.starts_with("control ")
            || trimmed.starts_with("database ")
            || trimmed.starts_with("collections ")
            || trimmed.starts_with("queue ")
            || trimmed.contains("->")
            || trimmed.contains("-->")
        {
            return UmlSubtype::Sequence;
        }
    }
    UmlSubtype::Sequence
}

enum UmlSubtype {
    Sequence,
    Class,
    State,
    Activity,
}

/// Parse PlantUML source into a typed diagram model.
pub fn parse(input: &str) -> Result<Diagram, ParseError> {
    let typ = detect_type(input);
    let lines = preprocess::preprocess(input);

    match typ {
        "uml" => match detect_uml_subtype(&lines) {
            UmlSubtype::Sequence => {
                let seq = sequence::parse_sequence(&lines)?;
                Ok(Diagram::Sequence(seq))
            }
            UmlSubtype::Class => {
                let cls = class::parse_class(&lines)?;
                Ok(Diagram::Class(cls))
            }
            UmlSubtype::State => {
                let st = state::parse_state(&lines)?;
                Ok(Diagram::State(st))
            }
            UmlSubtype::Activity => {
                let act = activity::parse_activity(&lines)?;
                Ok(Diagram::Activity(act))
            }
        },
        other => Err(ParseError {
            line: 1,
            message: format!("unsupported diagram type: @start{other}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_uml_type() {
        assert_eq!(detect_type("@startuml\nfoo\n@enduml"), "uml");
    }

    #[test]
    fn detects_json_type() {
        assert_eq!(detect_type("@startjson\n{}\n@endjson"), "json");
    }

    #[test]
    fn detects_gantt_type() {
        assert_eq!(detect_type("@startgantt\nfoo\n@endgantt"), "gantt");
    }

    #[test]
    fn parses_simple_sequence() {
        let input = "@startuml\nAlice -> Bob : hello\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Sequence(_)));
    }
}
