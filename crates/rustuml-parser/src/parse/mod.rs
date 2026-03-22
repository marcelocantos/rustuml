// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Diagram parsing — turns preprocessed lines into diagram models.

pub mod sequence;

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
            // Extract the type: @startuml, @startjson, @startgantt, etc.
            let typ = rest.split_whitespace().next().unwrap_or(rest);
            return typ;
        }
    }
    "uml"
}

/// Parse PlantUML source into a typed diagram model.
pub fn parse(input: &str) -> Result<Diagram, ParseError> {
    let typ = detect_type(input);
    let lines = preprocess::preprocess(input);

    match typ {
        "uml" => {
            let seq = sequence::parse_sequence(&lines)?;
            Ok(Diagram::Sequence(seq))
        }
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
