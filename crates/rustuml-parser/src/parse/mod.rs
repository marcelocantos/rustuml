// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Diagram parsing — turns preprocessed lines into diagram models.

pub mod activity;
pub mod class;
pub mod component;
pub mod deployment;
pub mod ditaa;
pub mod gantt;
pub mod json_diagram;
pub mod math;
pub mod mindmap;
pub mod nwdiag;
pub mod object;
pub mod regex_diagram;
pub mod salt;
pub mod sequence;
pub mod state;
pub mod timing;
pub mod usecase;
pub mod wbs;

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

/// Strip surrounding double-quotes from a title string, then trim whitespace.
pub fn strip_title_quotes(s: &str) -> &str {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

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

/// For @startuml, detect the specific UML subtype by scanning ALL lines
/// and counting indicator keywords. The type with the strongest signal wins.
fn detect_uml_subtype(lines: &[String]) -> UmlSubtype {
    let mut scores = [0i32; 9]; // Seq, Class, Object, State, Activity, Component, UseCase, Deployment, Timing

    for line in lines {
        let trimmed = line.trim();

        // Use case — must check before sequence (both use "actor").
        if trimmed.starts_with("usecase ") {
            scores[6] += 10;
        }
        // :Actor: shorthand (but not activity :action; lines).
        if trimmed.starts_with(':') && trimmed.ends_with(':') && !trimmed.ends_with(';') {
            scores[6] += 5;
        }
        // (UseCase) shorthand on its own line.
        if trimmed.starts_with('(') && trimmed.ends_with(')') {
            scores[6] += 5;
        }
        // State.
        if trimmed.starts_with("[*]")
            || trimmed.contains("--> [*]")
            || (trimmed.starts_with("state ") && !trimmed.contains("<<"))
        {
            scores[3] += 5;
        }
        // Activity (v3 new syntax).
        if trimmed == "start"
            || trimmed == "stop"
            || (trimmed.starts_with(':') && trimmed.ends_with(';'))
            || trimmed.starts_with("if (")
            || trimmed.starts_with("while (")
            || trimmed.starts_with("switch (")
            || trimmed == "fork"
            || trimmed == "split"
        {
            scores[4] += 5;
        }
        // Activity (v1 legacy syntax): `(*)`, `===NAME===`, or `if "cond" then`.
        if trimmed == "(*)"
            || trimmed.starts_with("(*) ")
            || trimmed.ends_with(" (*)")
            || (trimmed.starts_with("===") && trimmed.ends_with("==="))
            || (trimmed.starts_with("if \"") && trimmed.contains("\" then"))
        {
            scores[4] += 10;
        }
        // Deployment — check against the full keyword set.
        // "package" with a brace is excluded because it is heavily used in class
        // diagrams (package blocks); without a brace it is a deployment element.
        // "actor" is excluded because it is also a sequence/use-case participant
        // type; when only actor lines appear alongside arrows, the diagram is
        // almost certainly a sequence diagram, not a deployment diagram.
        {
            let kw_end = trimmed
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(trimmed.len());
            let kw = &trimmed[..kw_end];
            let package_with_brace = kw == "package" && trimmed.contains('{');
            // Truly deployment-exclusive container keywords (not shared with
            // component diagrams) used as containers (with `{`) are a strong
            // deployment signal.  `node`, `cloud`, `database`, `component`,
            // `frame`, `folder`, `rectangle` are shared with component diagrams,
            // so only keywords that are unique to deployment get the extra boost.
            const DEPLOY_EXCLUSIVE: &[&str] = &[
                "artifact", "storage", "card", "stack", "file", "agent",
            ];
            let is_deploy_exclusive_container =
                trimmed.contains('{') && DEPLOY_EXCLUSIVE.contains(&kw);
            // A deployment keyword with a QUOTED label and a `{` brace is a
            // strong deployment signal: component diagrams consistently use bare
            // identifiers for their containers; quoted names only appear in
            // deployment diagrams (e.g. `node "App Server" {`).
            let after_kw = trimmed[kw_end..].trim_start();
            let is_quoted_container = trimmed.contains('{')
                && after_kw.starts_with('"')
                && kw != "package"
                && kw != "actor"
                && deployment::DEPLOYMENT_KEYWORDS.contains(&kw);
            if !package_with_brace
                && kw != "actor"
                && deployment::DEPLOYMENT_KEYWORDS.contains(&kw)
                && kw_end < trimmed.len()
            {
                scores[7] += 5;
            }
            if is_deploy_exclusive_container || is_quoted_container {
                // Extra boost: deployment-exclusive container overrides component score.
                scores[7] += 15;
            }
        }
        // Component — weighted strongly so that a single `component` keyword
        // beats multiple `interface` lines that would otherwise score for class.
        if trimmed.starts_with("component ") {
            scores[5] += 15;
        }
        // Standalone `[Bracket]` syntax marks a component (leaf on its own line).
        // Exclude `[[url]]` PlantUML hyperlink syntax (double brackets).
        if trimmed.starts_with('[')
            && trimmed.ends_with(']')
            && !trimmed.starts_with("[*]")
            && !trimmed.starts_with("[[")
        {
            scores[5] += 10;
        }
        // `[Bracket]` appearing anywhere in a line (connection or standalone component
        // reference), e.g. `[Foo] - IFoo` or `IFoo - [Bar]`.
        // Exclude `[*]` (state diagram pseudo-states) and `[[url]]` hyperlinks.
        // Exclude member lines (starting with visibility prefix +/-/#/~) to avoid
        // false positive on array-typed fields like `+int[] intArray`.
        // Exclude `[#color]` notation used in sequence diagram colored arrows
        // (e.g. `Alice -[#red]> Bob`).
        // Exclude `return [...]` lines which are sequence diagram syntax.
        let looks_like_member = matches!(trimmed.chars().next(), Some('+' | '-' | '#' | '~'));
        if !looks_like_member
            && trimmed.contains('[')
            && trimmed.contains(']')
            && !trimmed.contains("[*]")
            && !trimmed.contains("[[")
            && !trimmed.contains("[#")
            && !trimmed.starts_with("return ")
        {
            scores[5] += 5;
        }
        // `interface` in a component context: score for both class and component.
        // `interface` alone still tips to class (class score ≥ component score in
        // the absence of `component` lines); combined with `component` keywords the
        // higher component-per-line weight causes component to win.
        if trimmed.starts_with("interface ") {
            scores[5] += 10; // component
        }
        // `note right/left/top/bottom of <id>` — valid in sequence, class, component,
        // and deployment diagrams. Score all four equally so that other keywords
        // determine the winner.
        if trimmed.starts_with("note ")
            && (trimmed.contains(" right of ")
                || trimmed.contains(" left of ")
                || trimmed.contains(" top of ")
                || trimmed.contains(" bottom of "))
        {
            scores[0] += 5; // sequence
            scores[1] += 5; // class
            scores[5] += 5; // component
            scores[7] += 5; // deployment
        }
        // Object / map — strong unique keywords.
        if trimmed.starts_with("object ") || trimmed.starts_with("map ") {
            scores[2] += 10;
        }
        // Class — use weight 10 so that class-specific keywords dominate
        // container keywords (cloud, folder, node, etc.) that are shared with
        // deployment diagrams.
        // Note: `entity` is excluded here because it is also a sequence participant
        // type; entity-with-body ({) is handled separately below.
        if trimmed.starts_with("class ")
            || trimmed.starts_with("abstract class ")
            || trimmed.starts_with("abstract ")
            || trimmed == "abstract"
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("annotation ")
            || trimmed.contains("<|--")
            || trimmed.contains("..|>")
            || trimmed.contains("*--")
            || trimmed.contains("o--")
        {
            scores[1] += 10;
        }
        // ER crow's foot notation is an unambiguous class/ER diagram signal.
        if trimmed.contains("||--")
            || trimmed.contains("}|--")
            || trimmed.contains("o|--")
            || trimmed.contains("|{--")
            || trimmed.contains("o{--")
        {
            scores[1] += 10;
        }
        // entity with a body block ({) is an unambiguous class/ER entity,
        // not a sequence participant.
        if trimmed.starts_with("entity ") && (trimmed.ends_with('{') || trimmed.ends_with("{{")) {
            scores[1] += 15;
        }
        // Sequence. Skip lines that end with `{` — those are container blocks
        // (class diagram packages) not sequence participants.
        if !trimmed.ends_with('{')
            && (trimmed.starts_with("participant ")
                || trimmed.starts_with("boundary ")
                || trimmed.starts_with("control ")
                || trimmed.starts_with("database ")
                || trimmed.starts_with("collections ")
                || trimmed.starts_with("queue ")
                || trimmed.starts_with("entity "))
        // entity is also a sequence participant type
        {
            scores[0] += 5;
        }
        // box / end box are unambiguously sequence-diagram keywords.
        if trimmed.starts_with("box ") || trimmed == "box" || trimmed == "end box" {
            scores[0] += 10;
        }
        // "actor" is ambiguous (sequence or use case) — give slight score to both.
        if trimmed.starts_with("actor ") {
            scores[0] += 2;
            scores[6] += 2;
        }
        // Arrows are weak sequence indicators.
        if trimmed.contains("->") || trimmed.contains("-->") {
            scores[0] += 1;
        }
        // `return` statement is sequence-diagram-specific syntax.
        if trimmed == "return" || trimmed.starts_with("return ") {
            scores[0] += 5;
        }
        // Timing — strong unique keywords.
        if trimmed.starts_with("robust ")
            || trimmed.starts_with("concise ")
            || trimmed.starts_with("binary ")
            || trimmed.starts_with("clock ")
        {
            scores[8] += 10;
        }
        // Standalone floating notes (`note as X`) are a class diagram feature in
        // Java PlantUML and produce CLASS-type SVG output.
        if trimmed.starts_with("note as ") {
            scores[1] += 10;
        }
        // `legend`, `header`/`endheader`, `footer`/`endfooter` — these are
        // meta elements that PlantUML defaults to CLASS when no other content exists.
        // Score them weakly for class so that a diagram with only meta elements
        // produces a CLASS diagram (not a sequence diagram by default).
        if trimmed == "legend"
            || trimmed.starts_with("legend ")
            || trimmed == "endlegend"
            || trimmed == "header"
            || trimmed.starts_with("header ")
            || trimmed.starts_with("left header")
            || trimmed.starts_with("right header")
            || trimmed.starts_with("center header")
            || trimmed == "endheader"
            || trimmed == "footer"
            || trimmed.starts_with("footer ")
            || trimmed.starts_with("left footer")
            || trimmed.starts_with("right footer")
            || trimmed.starts_with("center footer")
            || trimmed == "endfooter"
        {
            scores[1] += 1; // weak class signal
        }
    }

    let subtypes = [
        UmlSubtype::Sequence,
        UmlSubtype::Class,
        UmlSubtype::Object,
        UmlSubtype::State,
        UmlSubtype::Activity,
        UmlSubtype::Component,
        UmlSubtype::UseCase,
        UmlSubtype::Deployment,
        UmlSubtype::Timing,
    ];

    // Find the highest-scoring subtype. On ties, prefer earlier entries
    // (Sequence is the default).
    let max_score = scores.iter().copied().max().unwrap_or(0);
    let max_idx = scores.iter().position(|&s| s == max_score).unwrap_or(0);

    subtypes[max_idx]
}

#[derive(Clone, Copy)]
enum UmlSubtype {
    Sequence,
    Class,
    Object,
    State,
    Activity,
    Component,
    UseCase,
    Deployment,
    Timing,
}

/// Parse YAML input into a diagram model.
pub fn parse_yaml(input: &str) -> Result<Diagram, ParseError> {
    serde_yaml::from_str(input).map_err(|e| ParseError {
        line: e.location().map_or(0, |l| l.line()),
        message: format!("YAML parse error: {e}"),
    })
}

/// Parse JSON input into a diagram model.
pub fn parse_json(input: &str) -> Result<Diagram, ParseError> {
    serde_json::from_str(input).map_err(|e| ParseError {
        line: e.line(),
        message: format!("JSON parse error: {e}"),
    })
}

/// Detect input format and parse accordingly.
pub fn parse_auto(input: &str) -> Result<Diagram, ParseError> {
    parse_auto_with_base(input, None)
}

/// Detect input format and parse with a base directory for !include.
pub fn parse_auto_with_base(
    input: &str,
    base_dir: Option<&std::path::Path>,
) -> Result<Diagram, ParseError> {
    let trimmed = input.trim_start();
    if trimmed.starts_with('{') {
        parse_json(input)
    } else if trimmed.starts_with("type:") || trimmed.starts_with("---") {
        parse_yaml(input)
    } else {
        parse_with_base(input, base_dir)
    }
}

/// Parse PlantUML source into a typed diagram model.
pub fn parse(input: &str) -> Result<Diagram, ParseError> {
    parse_with_base(input, None)
}

/// Parse PlantUML with a base directory for !include resolution.
pub fn parse_with_base(
    input: &str,
    base_dir: Option<&std::path::Path>,
) -> Result<Diagram, ParseError> {
    let typ = detect_type(input);
    let lines = match base_dir {
        Some(dir) => preprocess::preprocess_with_base(input, dir),
        None => preprocess::preprocess(input),
    };

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
            UmlSubtype::Object => {
                let obj = object::parse_object(&lines)?;
                Ok(Diagram::Object(obj))
            }
            UmlSubtype::State => {
                let st = state::parse_state(&lines)?;
                Ok(Diagram::State(st))
            }
            UmlSubtype::Activity => {
                let act = activity::parse_activity(&lines)?;
                Ok(Diagram::Activity(act))
            }
            UmlSubtype::Component => {
                let comp = component::parse_component(&lines)?;
                Ok(Diagram::Component(comp))
            }
            UmlSubtype::UseCase => {
                let uc = usecase::parse_usecase(&lines)?;
                Ok(Diagram::UseCase(uc))
            }
            UmlSubtype::Deployment => {
                let dep = deployment::parse_deployment(&lines)?;
                Ok(Diagram::Deployment(dep))
            }
            UmlSubtype::Timing => {
                let td = timing::parse_timing(&lines)?;
                Ok(Diagram::Timing(td))
            }
        },
        "json" => {
            let jd = json_diagram::parse_json_diagram(&lines)?;
            Ok(Diagram::Json(jd))
        }
        "yaml" => {
            let jd = json_diagram::parse_yaml_diagram(&lines)?;
            Ok(Diagram::Json(jd))
        }
        "mindmap" => {
            let mm = mindmap::parse_mindmap(&lines)?;
            Ok(Diagram::MindMap(mm))
        }
        "gantt" => {
            let g = gantt::parse_gantt(&lines)?;
            Ok(Diagram::Gantt(g))
        }
        "wbs" => {
            let w = wbs::parse_wbs(&lines)?;
            Ok(Diagram::Wbs(w))
        }
        "math" => {
            let m = math::parse_math(&lines, false)?;
            Ok(Diagram::Math(m))
        }
        "latex" => {
            let m = math::parse_math(&lines, true)?;
            Ok(Diagram::Math(m))
        }
        "salt" => {
            let s = salt::parse_salt(&lines)?;
            Ok(Diagram::Salt(s))
        }
        "nwdiag" => {
            let nw = nwdiag::parse_nwdiag(&lines)?;
            Ok(Diagram::Nwdiag(nw))
        }
        "regex" => {
            let r = regex_diagram::parse_regex_diagram(&lines)?;
            Ok(Diagram::Regex(r))
        }
        "ditaa" => {
            let d = ditaa::parse_ditaa(&lines)?;
            Ok(Diagram::Ditaa(d))
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

    #[test]
    fn yaml_round_trip() {
        let input = "@startuml\nAlice -> Bob : hello\n@enduml";
        let diagram = parse(input).unwrap();
        let yaml = serde_yaml::to_string(&diagram).unwrap();
        let reparsed = parse_yaml(&yaml).unwrap();
        // Verify structure matches by re-serializing.
        let yaml2 = serde_yaml::to_string(&reparsed).unwrap();
        assert_eq!(yaml, yaml2);
    }

    #[test]
    fn json_round_trip() {
        let input = "@startuml\nAlice -> Bob : hello\n@enduml";
        let diagram = parse(input).unwrap();
        let json = serde_json::to_string(&diagram).unwrap();
        let reparsed = parse_json(&json).unwrap();
        let json2 = serde_json::to_string(&reparsed).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn auto_detect_yaml() {
        let yaml = "type: Sequence\ndiagram:\n  meta: {}\n  participants: []\n  events: []\n  autonumber: null";
        let diagram = parse_auto(yaml).unwrap();
        assert!(matches!(diagram, Diagram::Sequence(_)));
    }

    #[test]
    fn auto_detect_json() {
        let json = r#"{"type":"Sequence","diagram":{"meta":{},"participants":[],"events":[],"autonumber":null}}"#;
        let diagram = parse_auto(json).unwrap();
        assert!(matches!(diagram, Diagram::Sequence(_)));
    }

    #[test]
    fn auto_detect_plantuml() {
        let puml = "@startuml\nAlice -> Bob\n@enduml";
        let diagram = parse_auto(puml).unwrap();
        assert!(matches!(diagram, Diagram::Sequence(_)));
    }

    #[test]
    fn class_diagram_yaml_round_trip() {
        let input = "@startuml\nclass Foo {\n  +name : String\n}\nclass Bar\nFoo <|-- Bar\n@enduml";
        let diagram = parse(input).unwrap();
        let yaml = serde_yaml::to_string(&diagram).unwrap();
        let reparsed = parse_yaml(&yaml).unwrap();
        let yaml2 = serde_yaml::to_string(&reparsed).unwrap();
        assert_eq!(yaml, yaml2);
    }
}
