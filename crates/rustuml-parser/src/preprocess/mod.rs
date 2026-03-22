// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! TIM preprocessor — handles variables, functions, includes, and
//! control flow before diagram-specific parsing.
//!
//! For now this is a pass-through. The preprocessor will be fleshed
//! out as we add support for TIM directives.

/// Preprocess PlantUML source, expanding TIM directives.
///
/// Currently a pass-through that strips `@startuml`/`@enduml` and
/// returns the body lines.
pub fn preprocess(input: &str) -> Vec<String> {
    input
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("@start") && !trimmed.starts_with("@end")
        })
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_start_end_tags() {
        let input = "@startuml\nAlice -> Bob : hello\n@enduml\n";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello"]);
    }

    #[test]
    fn handles_startuml_with_name() {
        let input = "@startuml MyDiagram\nA -> B\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn handles_non_uml_tags() {
        let input = "@startjson\n{\"key\": \"value\"}\n@endjson";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["{\"key\": \"value\"}"]);
    }
}
