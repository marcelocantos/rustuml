// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Diagram parsing — turns preprocessed lines into diagram models.

pub mod activity;
pub mod archimate;
pub mod board;
pub mod class;
pub mod component;
pub mod deployment;
pub mod ditaa;
pub mod dot;
pub mod ebnf;
pub mod gantt;
pub mod git_diagram;
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

pub fn extract_link_url(line: &str) -> (Option<String>, String) {
    if let Some(start) = line.find("[[")
        && let Some(rel_end) = line[start..].find("]]")
    {
        let inner = &line[start + 2..start + rel_end];
        let url = inner.split(['{', ' ']).next().unwrap_or("").to_string();
        let remaining = format!(
            "{}{}",
            &line[..start],
            line[start + rel_end + 2..].trim_start()
        );
        if url.is_empty() {
            return (None, remaining.trim().to_string());
        }
        return (Some(url), remaining.trim().to_string());
    }
    (None, line.to_string())
}

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
///
/// If the input has an outer `@startuml` wrapper with an inner `@startXxx`
/// (e.g. `@startjson` nested inside `@startuml`), the inner type wins because
/// PlantUML allows embedding any `@start*` block inside `@startuml`.
fn detect_type(input: &str) -> &str {
    let mut first_type: Option<&str> = None;
    for line in input.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@start") {
            let typ = rest.split_whitespace().next().unwrap_or(rest);
            match first_type {
                None => {
                    first_type = Some(typ);
                }
                Some("uml") => {
                    // An inner @start inside @startuml overrides the outer uml type.
                    if typ != "uml" {
                        return typ;
                    }
                }
                _ => {
                    // Already have a specific (non-uml) type; stop scanning.
                    break;
                }
            }
        }
    }
    first_type.unwrap_or("uml")
}

/// For @startuml, detect the specific UML subtype by scanning ALL lines
/// and counting indicator keywords. The type with the strongest signal wins.
fn detect_uml_subtype(lines: &[String]) -> UmlSubtype {
    let mut scores = [0i32; 10]; // Seq, Class, Object, State, Activity, Component, UseCase, Deployment, Timing

    for line in lines {
        let trimmed = line.trim();
        // Normalize internal tabs to spaces so keyword detection works regardless
        // of whether the source uses spaces or tabs as separators.
        let tab_normalized;
        let trimmed = if trimmed.contains('\t') {
            tab_normalized = trimmed.replace('\t', " ");
            tab_normalized.as_str()
        } else {
            trimmed
        };

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
        //
        // `[*]` is the unmistakable state-diagram pseudostate marker; any line
        // mentioning it (as source `[*] ...->`, target `...-> [*]`, or a
        // bracketed coloured arrow `-[#blue]-> [*]`) is a strong state signal.
        // Weighted heavily so chains of `A --> B` arrows can't overwhelm a
        // single `[*] --> X` line, and so that diagrams mixing floating
        // notes (class-typed) with `[*]` transitions still parse as state.
        if trimmed.starts_with("[*]")
            || trimmed.contains("> [*]")
            || trimmed.contains(">[*]")
            || (trimmed.starts_with("state ") && !trimmed.contains("<<"))
        {
            scores[3] += 20;
        }
        // `note on link` annotates transitions (state diagrams) and connections
        // (use case diagrams). Score it for state so that state diagrams beat
        // the sequence scoring from `note left/right of` lines, but also score
        // use case so that a use case diagram with `usecase` keywords wins over
        // the state signal when both are present.
        if trimmed == "note on link"
            || trimmed.starts_with("note on link ")
            || trimmed.starts_with("note on link:")
        {
            scores[3] += 15; // state
            scores[6] += 15; // use case
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
        // Note: `===NAME===` must have non-`=` content between the delimiters.
        // A line like `====` starts AND ends with `===` but has no name — it is
        // a sequence-diagram divider, not a legacy activity sync-bar.
        let is_legacy_syncbar = trimmed.starts_with("===")
            && trimmed.ends_with("===")
            && trimmed.len() > 6
            && trimmed[3..trimmed.len() - 3].contains(|c: char| c != '=');
        if trimmed == "(*)"
            || trimmed.starts_with("(*) ")
            || trimmed.ends_with(" (*)")
            || is_legacy_syncbar
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
            const DEPLOY_EXCLUSIVE: &[&str] =
                &["artifact", "storage", "card", "stack", "file", "agent"];
            let is_deploy_exclusive_container =
                trimmed.contains('{') && DEPLOY_EXCLUSIVE.contains(&kw);
            // A deployment keyword with a QUOTED label and a `{` brace is a
            // strong deployment signal: component diagrams consistently use bare
            // identifiers for their containers; quoted names only appear in
            // deployment diagrams (e.g. `node "App Server" {`).
            // Exception: `rectangle` is shared between use case diagrams (system
            // boundary) and deployment diagrams; do not apply the quoted-container
            // boost to it so that `usecase` keywords can tip the balance.
            let after_kw = trimmed[kw_end..].trim_start();
            let is_quoted_container = trimmed.contains('{')
                && after_kw.starts_with('"')
                && kw != "package"
                && kw != "actor"
                && kw != "rectangle"
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
                scores[7] += 20;
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
        // Note: `*--` and `o--` are NOT scored for class here because they are
        // also used in object diagrams; `object` keyword presence disambiguates.
        if trimmed.starts_with("class ")
            || trimmed.starts_with("abstract class ")
            || trimmed.starts_with("abstract ")
            || trimmed == "abstract"
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("annotation ")
            || trimmed.contains("<|--")
            || trimmed.contains("..|>")
        {
            scores[1] += 10;
        }
        // `*--` and `o--` score for class only when no `object` keyword is present.
        // In object diagrams they denote composition/aggregation links.
        // We defer the disambiguation: score both, but object gets a tiebreak boost
        // from `object` keyword lines, which score object at +10 each.
        if trimmed.contains("*--") || trimmed.contains("o--") {
            scores[1] += 10;
            // Also score object so that a diagram with `object` declarations plus
            // *-- / o-- links stays an object diagram rather than tipping to class.
            scores[2] += 5;
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
        // Standalone floating notes (`note as X` or `note "text" as X`) are a
        // class diagram feature in Java PlantUML and produce CLASS-type SVG output.
        if trimmed.starts_with("note as ") || trimmed.starts_with("note \"") {
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
        // Archimate -- preprocessor-expanded lines are unambiguous.
        if trimmed.starts_with("archimate_element ") || trimmed.starts_with("archimate_rel ") {
            scores[9] += 20;
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
        UmlSubtype::Archimate,
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
    Archimate,
}

/// A single extracted block from a multi-block PlantUML file.
#[derive(Debug, Clone)]
pub struct DiagramBlock {
    /// Optional name from `@startXXX name` (the word after the type keyword).
    pub name: Option<String>,
    /// The diagram type (e.g. "uml", "json", "gantt").
    pub typ: String,
    /// Full block text including @start/@end lines, plus any leading preamble.
    pub source: String,
    /// 0-based index of this block in the file.
    pub index: usize,
}

/// Split a PlantUML file into individual blocks.
///
/// Lines before the first `@startXXX` are treated as a "preamble" (shared
/// `!define`, `!function`, etc.) and prepended to every block's source so
/// that preprocessing sees the definitions.
///
/// Content between blocks (whitespace, comments) is silently skipped.
pub fn split_blocks(input: &str) -> Vec<DiagramBlock> {
    let mut blocks = Vec::new();
    let mut preamble_lines: Vec<&str> = Vec::new();
    // (outer-type, name, lines, nesting-depth)
    // nesting_depth counts how many @start tags are open; the block closes
    // when depth drops back to 0 on an @end.
    let mut current_start: Option<(String, Option<String>, Vec<&str>, usize)> = None;
    let mut index = 0usize;

    for line in input.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("@start") {
            if current_start.is_none() {
                // Starting a new top-level block.
                let mut parts = rest.split_whitespace();
                let typ = parts.next().unwrap_or("uml").to_string();
                let name = parts.next().map(|s| s.to_string());
                current_start = Some((typ, name, vec![line], 1));
            } else if let Some((_, _, ref mut block_lines, ref mut depth)) = current_start {
                // Nested @start inside an open block — treat as content.
                // PlantUML allows @startjson embedded inside @startuml etc.
                block_lines.push(line);
                *depth += 1;
            }
        } else if trimmed.starts_with("@end") {
            if let Some((_, _, ref mut block_lines, ref mut depth)) = current_start {
                block_lines.push(line);
                *depth -= 1;
                if *depth == 0 {
                    // Outer block is closed — emit it.
                    let (typ, name, block_lines, _) = current_start.take().unwrap();
                    let source = build_source(&preamble_lines, &block_lines);
                    blocks.push(DiagramBlock {
                        name,
                        typ,
                        source,
                        index,
                    });
                    index += 1;
                }
            }
            // If there's no open block, this is a stray @end — ignore it.
        } else if let Some((_, _, ref mut block_lines, _)) = current_start {
            block_lines.push(line);
        } else {
            // Before the first block: accumulate as preamble.
            preamble_lines.push(line);
        }
    }

    // If a block was started but never closed, emit it anyway.
    if let Some((typ, name, block_lines, _)) = current_start.take() {
        let source = build_source(&preamble_lines, &block_lines);
        blocks.push(DiagramBlock {
            name,
            typ,
            source,
            index,
        });
    }

    blocks
}

fn build_source(preamble: &[&str], block_lines: &[&str]) -> String {
    if preamble.is_empty() {
        block_lines.join("\n")
    } else {
        let mut s = preamble.join("\n");
        s.push('\n');
        s.push_str(&block_lines.join("\n"));
        s
    }
}

/// Parse all `@start`/`@end` blocks in a file, returning one result per block.
///
/// Lines before the first `@start` are treated as a shared preamble and
/// prepended to each block before parsing.
pub fn parse_all(input: &str) -> Vec<Result<Diagram, ParseError>> {
    split_blocks(input)
        .into_iter()
        .map(|block| parse_with_base(&block.source, None))
        .collect()
}

/// Parse only the block at the given 0-based index.
///
/// Returns `Err` with a descriptive message if the index is out of range.
pub fn parse_block(input: &str, index: usize) -> Result<Diagram, ParseError> {
    let blocks = split_blocks(input);
    if blocks.is_empty() {
        return parse_with_base(input, None);
    }
    let block = blocks.into_iter().nth(index).ok_or_else(|| ParseError {
        line: 1,
        message: format!(
            "block index {index} out of range (file has {} block(s))",
            split_blocks(input).len()
        ),
    })?;
    parse_with_base(&block.source, None)
}

/// Parse only the block with the given name (from `@startXXX name`).
///
/// If multiple blocks share the same name, the first one is returned.
/// Returns `Err` if no block with that name exists.
pub fn parse_named(input: &str, name: &str) -> Result<Diagram, ParseError> {
    let blocks = split_blocks(input);
    let block = blocks
        .into_iter()
        .find(|b| b.name.as_deref() == Some(name))
        .ok_or_else(|| ParseError {
            line: 1,
            message: format!("no block named {name:?} found in input"),
        })?;
    parse_with_base(&block.source, None)
}

/// Parse YAML input into a diagram model.
pub fn parse_yaml(input: &str) -> Result<Diagram, ParseError> {
    serde_yml::from_str(input).map_err(|e| ParseError {
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
        // Try model JSON first; fall back to @startjson-style data visualization.
        parse_json(input).or_else(|_| {
            let lines: Vec<String> = input.lines().map(|l| l.to_string()).collect();
            let diagram = json_diagram::parse_json_diagram(&lines)?;
            Ok(Diagram::Json(diagram))
        })
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
    let preprocess_out = match base_dir {
        Some(dir) => preprocess::preprocess_full_with_base(input, dir),
        None => preprocess::preprocess_full(input, None),
    };
    let lines = preprocess_out.lines;
    let sprites = preprocess_out.sprites;

    let mut diagram = match typ {
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
            UmlSubtype::Archimate => {
                let arch = archimate::parse_archimate(&lines)?;
                Ok(Diagram::Archimate(arch))
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
        "git" => {
            let g = git_diagram::parse_git(&lines)?;
            Ok(Diagram::Git(g))
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
        "dot" => {
            let d = dot::parse_dot(&lines)?;
            Ok(Diagram::Dot(d))
        }
        "board" => {
            let b = board::parse_board(&lines)?;
            Ok(Diagram::Board(b))
        }
        "ebnf" => {
            let e = ebnf::parse_ebnf(&lines)?;
            Ok(Diagram::Ebnf(e))
        }
        other => Err(ParseError {
            line: 1,
            message: format!("unsupported diagram type: @start{other}"),
        }),
    }?;

    // Inject sprite definitions from the preprocessor into the diagram's meta.
    if !sprites.is_empty() {
        let meta = diagram.meta_mut();
        meta.sprites = sprites;
    }

    Ok(diagram)
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
        let yaml = serde_yml::to_string(&diagram).unwrap();
        let reparsed = parse_yaml(&yaml).unwrap();
        // Verify structure matches by re-serializing.
        let yaml2 = serde_yml::to_string(&reparsed).unwrap();
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
        let yaml = serde_yml::to_string(&diagram).unwrap();
        let reparsed = parse_yaml(&yaml).unwrap();
        let yaml2 = serde_yml::to_string(&reparsed).unwrap();
        assert_eq!(yaml, yaml2);
    }

    // ── multi-block splitting ─────────────────────────────────────────────────

    #[test]
    fn split_two_blocks_same_type() {
        let input =
            "@startuml\nAlice -> Bob : Hello\n@enduml\n\n@startuml\nBob -> Alice : Hi\n@enduml";
        let blocks = split_blocks(input);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].typ, "uml");
        assert_eq!(blocks[0].index, 0);
        assert_eq!(blocks[1].typ, "uml");
        assert_eq!(blocks[1].index, 1);
    }

    #[test]
    fn split_three_blocks_mixed_types() {
        let input = concat!(
            "@startuml\nAlice -> Bob\n@enduml\n",
            "@startjson\n{\"key\": \"value\"}\n@endjson\n",
            "@startgantt\n[Task] lasts 3 days\n@endgantt"
        );
        let blocks = split_blocks(input);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].typ, "uml");
        assert_eq!(blocks[1].typ, "json");
        assert_eq!(blocks[2].typ, "gantt");
    }

    #[test]
    fn split_named_blocks() {
        let input =
            "@startuml first\nAlice -> Bob\n@enduml\n@startuml second\nBob -> Alice\n@enduml";
        let blocks = split_blocks(input);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].name.as_deref(), Some("first"));
        assert_eq!(blocks[1].name.as_deref(), Some("second"));
    }

    #[test]
    fn split_unnamed_blocks_have_no_name() {
        let input = "@startuml\nAlice -> Bob\n@enduml\n@startuml\nBob -> Alice\n@enduml";
        let blocks = split_blocks(input);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].name.is_none());
        assert!(blocks[1].name.is_none());
    }

    #[test]
    fn split_preserves_preamble() {
        let input = "!define ALICE Alice\n@startuml\nALICE -> Bob\n@enduml\n@startuml\nALICE -> Carol\n@enduml";
        let blocks = split_blocks(input);
        assert_eq!(blocks.len(), 2);
        // Preamble should be prepended to each block's source.
        assert!(blocks[0].source.contains("!define ALICE Alice"));
        assert!(blocks[1].source.contains("!define ALICE Alice"));
    }

    #[test]
    fn parse_all_two_blocks() {
        let input =
            "@startuml\nAlice -> Bob : Hello\n@enduml\n@startuml\nBob -> Alice : Hi\n@enduml";
        let results = parse_all(input);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
        assert!(matches!(results[0].as_ref().unwrap(), Diagram::Sequence(_)));
        assert!(matches!(results[1].as_ref().unwrap(), Diagram::Sequence(_)));
    }

    #[test]
    fn parse_all_mixed_types() {
        let input = concat!(
            "@startuml\nAlice -> Bob : Hello\n@enduml\n",
            "@startjson\n{\"key\": \"val\"}\n@endjson"
        );
        let results = parse_all(input);
        assert_eq!(results.len(), 2);
        assert!(matches!(results[0].as_ref().unwrap(), Diagram::Sequence(_)));
        assert!(matches!(results[1].as_ref().unwrap(), Diagram::Json(_)));
    }

    #[test]
    fn parse_named_finds_correct_block() {
        let input = "@startuml first\nAlice -> Bob : Hello\n@enduml\n@startuml second\nclass Foo {}\n@enduml";
        let diagram = parse_named(input, "first").unwrap();
        assert!(matches!(diagram, Diagram::Sequence(_)));
        let diagram = parse_named(input, "second").unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn parse_named_missing_returns_error() {
        let input = "@startuml first\nAlice -> Bob\n@enduml";
        let result = parse_named(input, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("nonexistent"));
    }

    #[test]
    fn parse_block_index_selects_correct_block() {
        let input = "@startuml\nAlice -> Bob : First\n@enduml\n@startuml\nclass Foo {}\n@enduml";
        let d0 = parse_block(input, 0).unwrap();
        assert!(matches!(d0, Diagram::Sequence(_)));
        let d1 = parse_block(input, 1).unwrap();
        assert!(matches!(d1, Diagram::Class(_)));
    }

    #[test]
    fn parse_block_out_of_range_returns_error() {
        let input = "@startuml\nAlice -> Bob\n@enduml";
        let result = parse_block(input, 5);
        assert!(result.is_err());
    }

    #[test]
    fn split_single_block_still_works() {
        let input = "@startuml\nAlice -> Bob\n@enduml";
        let blocks = split_blocks(input);
        assert_eq!(blocks.len(), 1);
    }
}

#[cfg(test)]
mod link_url_tests {
    use super::extract_link_url;

    #[test]
    fn basic_url() {
        let (url, rest) = extract_link_url("class Foo [[https://example.com]] {");
        assert_eq!(url.as_deref(), Some("https://example.com"));
        assert_eq!(rest, "class Foo {");
    }

    #[test]
    fn url_with_tooltip() {
        let (url, rest) = extract_link_url("class Foo [[https://example.com{tooltip}]]");
        assert_eq!(url.as_deref(), Some("https://example.com"));
        assert_eq!(rest, "class Foo");
    }

    #[test]
    fn url_with_label() {
        let (url, rest) = extract_link_url("class Foo [[https://example.com Label]]");
        assert_eq!(url.as_deref(), Some("https://example.com"));
        assert_eq!(rest, "class Foo");
    }

    #[test]
    fn url_with_tooltip_and_label() {
        let (url, rest) = extract_link_url("class Foo [[https://example.com{tip} Label]]");
        assert_eq!(url.as_deref(), Some("https://example.com"));
        assert_eq!(rest, "class Foo");
    }

    #[test]
    fn no_url() {
        let (url, rest) = extract_link_url("class Foo {");
        assert_eq!(url, None);
        assert_eq!(rest, "class Foo {");
    }

    #[test]
    fn empty_brackets() {
        let (url, rest) = extract_link_url("class Foo [[]]");
        assert_eq!(url, None);
        assert_eq!(rest, "class Foo");
    }
}
