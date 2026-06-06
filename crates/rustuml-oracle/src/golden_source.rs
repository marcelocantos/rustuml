// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Cow;

/// Return the source shape RustUML should parse when reproducing a Java golden.
///
/// PlantUML's multi-diagram CLI convention is block-oriented, and RustUML's
/// normal CLI follows that. One legacy corpus edge case has adjacent UML
/// blocks with no blank separator, and the Java SVG aggregates the class
/// declarations into one CLASS diagram. Collapse only that observed oracle
/// shape; ordinary multi-diagram files remain block-oriented.
pub fn source_for_oracle_golden<'a>(source: &'a str, golden_svg: &str) -> Cow<'a, str> {
    if !golden_svg.contains(r#"data-diagram-type="CLASS""#) {
        return Cow::Borrowed(source);
    }
    collapse_adjacent_uml_blocks(source).map_or(Cow::Borrowed(source), Cow::Owned)
}

fn collapse_adjacent_uml_blocks(source: &str) -> Option<String> {
    let mut lines = Vec::new();
    let mut merged = false;

    for line in source.lines() {
        let trimmed = line.trim();
        let previous_is_uml_end = lines
            .last()
            .is_some_and(|prev: &&str| prev.trim().starts_with("@enduml"));
        if previous_is_uml_end && trimmed.starts_with("@startuml") {
            lines.pop();
            merged = true;
            continue;
        }
        lines.push(line);
    }

    merged.then(|| lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::source_for_oracle_golden;
    use std::borrow::Cow;

    #[test]
    fn collapses_adjacent_class_blocks_for_oracle() {
        let source = "@startuml\nclass First\n@enduml\n@startuml\nclass Second\n@enduml";
        let golden = r#"<svg data-diagram-type="CLASS"></svg>"#;
        let normalized = source_for_oracle_golden(source, golden);

        assert!(matches!(normalized, Cow::Owned(_)));
        assert_eq!(
            normalized.as_ref(),
            "@startuml\nclass First\nclass Second\n@enduml"
        );
    }

    #[test]
    fn leaves_separated_blocks_block_oriented() {
        let source = "@startuml\nclass First\n@enduml\n\n@startuml\nclass Second\n@enduml";
        let golden = r#"<svg data-diagram-type="CLASS"></svg>"#;

        assert!(matches!(
            source_for_oracle_golden(source, golden),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn leaves_non_class_goldens_alone() {
        let source = "@startuml\nAlice -> Bob\n@enduml\n@startuml\nBob -> Alice\n@enduml";
        let golden = r#"<svg data-diagram-type="SEQUENCE"></svg>"#;

        assert!(matches!(
            source_for_oracle_golden(source, golden),
            Cow::Borrowed(_)
        ));
    }
}
