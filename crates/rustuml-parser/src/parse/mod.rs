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

/// Extract the optional tooltip from a `[[url{tooltip} label]]` link on the
/// given line. PlantUML uses the `{...}` content as the anchor's `title`
/// attribute (the URL is the title otherwise). A bare ` label` does not change
/// the title, so it is not returned here. Returns `None` when there is no
/// `[[ ]]` link or no `{...}` tooltip within it.
pub fn extract_link_tooltip(line: &str) -> Option<String> {
    let start = line.find("[[")?;
    let rel_end = line[start..].find("]]")?;
    let inner = &line[start + 2..start + rel_end];
    let brace_start = inner.find('{')?;
    let brace_end = inner[brace_start..].find('}')? + brace_start;
    let tip = inner[brace_start + 1..brace_end].trim();
    if tip.is_empty() {
        None
    } else {
        Some(tip.to_string())
    }
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

/// Truncate the preprocessed line list at the first standalone `newpage`
/// directive. PlantUML's SVG renderer emits only the first page of a multipage
/// (non-sequence) diagram, so everything from `newpage` onward is dropped. A
/// `newpage <title>` form also delimits the page and is dropped along with its
/// argument.
fn truncate_at_newpage(lines: Vec<String>) -> Vec<String> {
    if let Some(idx) = lines.iter().position(|l| {
        let t = source_text(l).trim();
        t == "newpage" || t.starts_with("newpage ") || t.starts_with("newpage\t")
    }) {
        let mut lines = lines;
        lines.truncate(idx);
        lines
    } else {
        lines
    }
}

pub(crate) fn source_line_and_trimmed(fallback: usize, line: &str) -> (usize, &str) {
    let (source_line, text) =
        preprocess::split_source_line_marker(line).unwrap_or((fallback, line));
    (source_line, text.trim())
}

pub(crate) fn source_text(line: &str) -> &str {
    preprocess::split_source_line_marker(line).map_or(line, |(_, text)| text)
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

    // `allowmixing` is a class-diagram directive: it permits mixing other
    // element kinds (state, object, etc.) into a CLASS diagram. When it appears
    // alongside an explicit class-style declaration, the diagram is CLASS even
    // if state/object signals would otherwise score higher.
    let mut has_allowmixing = false;
    let mut has_skinparam = false;
    let mut has_meta_only_class_default = false;
    let mut has_class_dependency_arrow = false;
    let mut has_class_association_line = false;
    let mut has_direction_directive = false;
    let mut has_floating_note = false;
    let mut has_interface_decl = false;
    let mut has_component_leaf_keyword = false;
    let mut has_non_interface_class_decl = false;

    for line in lines {
        let trimmed = source_text(line).trim();
        // Normalize internal tabs to spaces so keyword detection works regardless
        // of whether the source uses spaces or tabs as separators.
        let tab_normalized;
        let trimmed = if trimmed.contains('\t') {
            tab_normalized = trimmed.replace('\t', " ");
            tab_normalized.as_str()
        } else {
            trimmed
        };

        if trimmed.starts_with("skinparam ") {
            has_skinparam = true;
        }
        if matches!(
            trimmed,
            "left to right direction" | "top to bottom direction"
        ) {
            has_direction_directive = true;
        }

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
        // A `state ` line is only a state declaration when an identifier or
        // quoted name follows. Inside an entity/class body, `state : TYPE` is a
        // member named "state" (the typed `name : type` form), not a state
        // declaration — exclude it so an ER entity with a `state` column does
        // not get misrouted to a STATE diagram.
        let state_decl = trimmed.starts_with("state ")
            && !trimmed.contains("<<")
            && !trimmed["state ".len()..].trim_start().starts_with(':');
        if trimmed.starts_with("[*]")
            || trimmed.contains("> [*]")
            || trimmed.contains(">[*]")
            || state_decl
        {
            scores[3] += 50;
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
            let deployment_keyword_arg = !after_kw.starts_with(':');
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
                && deployment_keyword_arg
            {
                scores[7] += 5;
                if matches!(kw, "artifact" | "cloud" | "database" | "node" | "queue")
                    && !trimmed.contains('{')
                {
                    has_component_leaf_keyword = true;
                }
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
        // `[...]` after the `:` of a transition label is a state-diagram guard
        // expression (e.g. `A --> B : [count < 3]` or `... : event [guard]`)
        // — do not credit it as a component-style bracket reference.
        let bracket_is_transition_guard = if let Some(colon) = trimmed.find(':') {
            let before = &trimmed[..colon];
            let after = &trimmed[colon + 1..];
            (before.contains("->") || before.contains("<-"))
                && after.contains('[')
                && after.contains(']')
        } else {
            false
        };
        if !looks_like_member
            && !bracket_is_transition_guard
            && trimmed.contains('[')
            && trimmed.contains(']')
            && !trimmed.contains("[*]")
            && !trimmed.contains("[[")
            && !trimmed.contains("[#")
            && !trimmed.starts_with("return ")
            // `autonumber "<b>[000]"` uses brackets inside its format string;
            // that is a sequence-diagram directive, not a component reference.
            && !trimmed.starts_with("autonumber")
        {
            scores[5] += 5;
        }
        // `autonumber` (with or without start/step/format) is sequence-only.
        if trimmed == "autonumber" || trimmed.starts_with("autonumber ") {
            scores[0] += 10;
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
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("annotation ")
            || trimmed.starts_with("circle ")
            || trimmed.starts_with("diamond ")
            || trimmed.contains("<|--")
            || trimmed.contains("..|>")
        {
            scores[1] += 10;
            has_non_interface_class_decl = true;
        }
        if trimmed.starts_with("interface ") {
            scores[1] += 10;
            has_interface_decl = true;
        }
        if trimmed.contains("..>") || trimmed.contains("<..") {
            has_class_dependency_arrow = true;
        }
        if looks_like_bare_class_association(trimmed) {
            has_class_association_line = true;
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
        if trimmed.starts_with("entity ")
            && (trimmed.ends_with('{') || trimmed.ends_with("{{") || trimmed.ends_with("{}"))
        {
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
        if trimmed.starts_with("note as ") || trimmed.starts_with("note \"") {
            has_floating_note = true;
        }
        // A leading `note : text` line is parsed by Java PlantUML as a CLASS
        // note/entity diagram, even when later lines contain weak sequence-style
        // arrows. Sequence notes use a side qualifier (`note right :`, etc.) or
        // `note over`, so this bare form is a class signal.
        if trimmed.starts_with("note :") {
            scores[1] += 10;
        }
        // `legend`, `header`/`endheader`, `footer`/`endfooter` — these are
        // meta elements that PlantUML defaults to CLASS when no other content
        // exists. Track them as a fallback rather than scoring them, so a
        // title/header expanded from a preamble procedure cannot outvote a
        // real sequence message or state transition in the same block.
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
            || trimmed.starts_with("title ")
            || trimmed == "title"
        {
            has_meta_only_class_default = true;
        }
        // Archimate -- preprocessor-expanded lines are unambiguous.
        if trimmed.starts_with("archimate_element ") || trimmed.starts_with("archimate_rel ") {
            scores[9] += 20;
        }
        // `allowmixing` directive (class-diagram only).
        if trimmed == "allowmixing" || trimmed.starts_with("allowmixing ") {
            has_allowmixing = true;
        }
    }

    // Standalone floating notes (`note as X` or `note "text" as X`) can attach
    // to class/object/deployment diagrams. Score class/object always so
    // note-only diagrams tie back to CLASS by the default ordering; score
    // deployment only when a real deployment keyword was already seen.
    if has_floating_note {
        scores[1] += 10;
        scores[2] += 10;
        if scores[7] > 0 {
            scores[7] += 10;
        }
    }

    // Plain class-style relationship lines default to CLASS in Java PlantUML:
    // `A ..> B`, but also bare association forms like `A .. B` and `C -- D`.
    // The same spellings are valid for object, component, and deployment links,
    // so treat them as class signals only when no explicit non-class
    // container/entity syntax has appeared.
    if (has_class_dependency_arrow || has_class_association_line)
        && scores[2] == 0
        && scores[5] == 0
        && scores[7] == 0
    {
        scores[1] += 10;
    }

    if has_interface_decl && has_component_leaf_keyword && !has_non_interface_class_decl {
        let competing = scores[1].max(scores[7]);
        if scores[5] <= competing {
            scores[5] = competing + 1;
        }
    }

    // Direction directives (`left to right direction`, `top to bottom
    // direction`) are handled by Java PlantUML's graph-style UML path. With no
    // stronger explicit diagram syntax, weak `A -> B` arrows become CLASS
    // dependencies rather than SEQUENCE messages.
    if has_direction_directive
        && scores[0] > 0
        && scores[1] == 0
        && scores[2] == 0
        && scores[3] == 0
        && scores[4] == 0
        && scores[5] == 0
        && scores[6] == 0
        && scores[7] == 0
        && scores[8] == 0
        && scores[9] == 0
    {
        scores[1] = scores[0] + 1;
    }

    // `allowmixing` + an explicit class declaration => CLASS, overriding any
    // state/object/etc. signals from the mixed-in elements. scores[1] (CLASS)
    // is non-zero only when a real class-style declaration (class/interface/
    // enum/abstract/annotation/inheritance) was seen, so this never fires on
    // an `allowmixing` diagram that has no class content (e.g. participant +
    // component, which Java PlantUML rejects as an error rather than CLASS).
    if has_allowmixing && scores[1] > 0 {
        let other_max = scores
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 1)
            .map(|(_, &s)| s)
            .max()
            .unwrap_or(0);
        if scores[1] <= other_max {
            scores[1] = other_max + 1;
        }
    }

    if (has_skinparam || has_meta_only_class_default) && scores.iter().all(|&s| s == 0) {
        return UmlSubtype::Class;
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

fn looks_like_bare_class_association(line: &str) -> bool {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r#"^(?:"[^"]+"|[\w./:]+)\s*(?:-{2,}|\.{2,})\s*(?:"[^"]+"|[\w./:]+)(?:\s*:\s*.+)?$"#,
        )
        .unwrap()
    });
    RE.is_match(line)
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
    /// 1-based source line of the block's top-level `@startXXX`.
    pub start_line: usize,
    /// 0-based index of this block in the file.
    pub index: usize,
}

struct OpenDiagramBlock<'a> {
    typ: String,
    name: Option<String>,
    start_line: usize,
    lines: Vec<&'a str>,
    depth: usize,
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
    // depth counts how many @start tags are open; the block closes when it
    // drops back to 0 on an @end.
    let mut current_start: Option<OpenDiagramBlock<'_>> = None;
    let mut index = 0usize;

    for (line_idx, line) in input.lines().enumerate() {
        let source_line = line_idx + 1;
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("@start") {
            if current_start.is_none() {
                // Starting a new top-level block.
                let mut parts = rest.split_whitespace();
                let typ = parts.next().unwrap_or("uml").to_string();
                let name = parts.next().map(|s| s.to_string());
                current_start = Some(OpenDiagramBlock {
                    typ,
                    name,
                    start_line: source_line,
                    lines: vec![line],
                    depth: 1,
                });
            } else if let Some(block) = &mut current_start {
                // Nested @start inside an open block — treat as content.
                // PlantUML allows @startjson embedded inside @startuml etc.
                block.lines.push(line);
                block.depth += 1;
            }
        } else if trimmed.starts_with("@end") {
            if let Some(block) = &mut current_start {
                block.lines.push(line);
                block.depth -= 1;
                if block.depth == 0 {
                    // Outer block is closed — emit it.
                    let block = current_start.take().unwrap();
                    let source = build_source(&preamble_lines, &block.lines);
                    blocks.push(DiagramBlock {
                        name: block.name,
                        typ: block.typ,
                        source,
                        start_line: block.start_line,
                        index,
                    });
                    index += 1;
                }
            }
            // If there's no open block, this is a stray @end — ignore it.
        } else if let Some(block) = &mut current_start {
            block.lines.push(line);
        } else {
            // Before the first block: accumulate as preamble.
            preamble_lines.push(line);
        }
    }

    // If a block was started but never closed, emit it anyway.
    if let Some(block) = current_start.take() {
        let source = build_source(&preamble_lines, &block.lines);
        blocks.push(DiagramBlock {
            name: block.name,
            typ: block.typ,
            source,
            start_line: block.start_line,
            index,
        });
    }

    blocks
}

fn source_with_original_line_offset(block: &DiagramBlock) -> String {
    if block.start_line <= 1 || preamble_contains_body_expansion(&block.source) {
        return block.source.clone();
    }

    let mut source = "\n".repeat(block.start_line - 1);
    source.push_str(&block.source);
    source
}

fn preamble_contains_body_expansion(source: &str) -> bool {
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("@start") {
            return false;
        }
        if trimmed.starts_with("!procedure ")
            || trimmed.starts_with("!function ")
            || trimmed.starts_with("!definelong ")
        {
            return true;
        }
    }
    false
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
        .map(|block| parse_with_base(&source_with_original_line_offset(&block), None))
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
    parse_with_base(&source_with_original_line_offset(&block), None)
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
    parse_with_base(&source_with_original_line_offset(&block), None)
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
    let mut preprocess_out = match base_dir {
        Some(dir) => preprocess::preprocess_full_for_parse(input, Some(dir.to_path_buf())),
        None => preprocess::preprocess_full_for_parse(input, None),
    };
    let uml_subtype = (typ == "uml").then(|| detect_uml_subtype(&preprocess_out.lines));
    if matches!(uml_subtype, Some(UmlSubtype::Sequence)) {
        preprocess_out = match base_dir {
            Some(dir) => {
                preprocess::preprocess_full_for_sequence_parse(input, Some(dir.to_path_buf()))
            }
            None => preprocess::preprocess_full_for_sequence_parse(input, None),
        };
    }
    let lines = preprocess_out.lines;
    let sprites = preprocess_out.sprites;

    // For non-sequence UML diagrams a `newpage` directive splits the diagram
    // into multiple pages, but PlantUML's SVG output renders only the first
    // page. Sequence diagrams handle `newpage` as a paginating event with their
    // own multipage rendering, so they keep all lines.
    let lines = if matches!(uml_subtype, Some(UmlSubtype::Sequence)) {
        lines
    } else {
        truncate_at_newpage(lines)
    };

    let mut diagram = match typ {
        "uml" => match uml_subtype.expect("uml subtype computed above") {
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

    // Stash the diagram source for downstream seed computation (filter
    // UIDs, gradient/shadow ids). Mirrors PlantUML's
    // `UmlSource.getPlainString("\n")` — every line of the @startuml ...
    // @enduml block (including the markers) joined by `\n` with a trailing
    // `\n`. PlantUML's preprocessor preserves these markers; ours strips
    // them, so we rebuild the equivalent string from the raw input.
    {
        let meta = diagram.meta_mut();
        meta.source = Some(seed_source_string(input));
    }

    Ok(diagram)
}

/// Reconstruct the seed-source string that PlantUML's
/// `UmlSource.getPlainString("\n")` would have produced for `input`. That
/// string is built from the `source` list inside `UmlSource`, which in
/// practice is "every line between (and including) `@startXXX` and
/// `@endXXX`, with a trailing `\n`". For inputs that omit the markers we
/// take the input as-is. The output is used purely to derive deterministic
/// element ids (filter UIDs etc.) via `StringUtils.seed`.
fn seed_source_string(input: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;
    let mut saw_marker = false;
    for line in input.lines() {
        let trimmed = line.trim_start();
        if !in_block {
            if trimmed.starts_with("@start") {
                in_block = true;
                saw_marker = true;
                out.push_str(line);
                out.push('\n');
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
        if trimmed.starts_with("@end") {
            break;
        }
    }
    let _ = in_block;
    if saw_marker {
        out
    } else {
        // Headerless input — match PlantUML's `\n`-joined form with a
        // trailing newline.
        let mut joined = String::new();
        for line in input.lines() {
            joined.push_str(line);
            joined.push('\n');
        }
        joined
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
    fn skinparam_only_uml_defaults_to_class() {
        let input = "@startuml\nskinparam backgroundColor #FFFEF0\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn leading_bare_note_colon_routes_to_class() {
        let input = "@startuml\nnote : x = 1\nAlice -> Bob : Message 1\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn inline_empty_entity_body_routes_to_class() {
        let input = "@startuml\nentity MyType {}\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn dotted_dependency_arrows_route_to_class() {
        let input = "@startuml\nA ..> B: dotted\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));

        let input = "@startuml\nA ..>> B: dotted thick\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn bare_association_lines_route_to_class() {
        let input = "@startuml\nA .. B\nC -- D\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));

        let input = "@startuml\ncom.example.A -- com.example.B\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn sequence_label_with_dashes_stays_sequence() {
        let input = "@startuml\nAlice -> Bob : --strike text--\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Sequence(_)));
    }

    #[test]
    fn direction_directive_with_weak_arrows_routes_to_class() {
        let input = "@startuml\nleft to right direction\nAlice -> Bob : hello\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn dotted_dependency_arrows_preserve_explicit_non_class_types() {
        let input = "@startuml\nobject Source\nobject Target\nSource ..> Target\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Object(_)));

        let input = "@startuml\nnode NodeA\nnode NodeB\nNodeA ..> NodeB\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Deployment(_)));

        let input = "@startuml\ncomponent A\ncomponent B\nA ..> B\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Component(_)));
    }

    #[test]
    fn interface_plus_component_leaf_routes_to_component() {
        let input = "@startuml\ninterface MyA\nartifact MyB\nMyA --> MyB\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Component(_)));

        let input = "@startuml\ninterface MyA\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn bare_association_lines_preserve_explicit_non_class_types() {
        let input = "@startuml\nobject Source\nobject Target\nSource -- Target\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Object(_)));

        let input = "@startuml\nnode NodeA\nnode NodeB\nNodeA -- NodeB\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Deployment(_)));

        let input = "@startuml\ncomponent A\ncomponent B\nA -- B\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Component(_)));
    }

    #[test]
    fn object_with_floating_note_stays_object() {
        let input = "@startuml\nobject Server {\n  ip = \"192.168.1.1\"\n}\nnote \"text\" as N1\nServer .. N1\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Object(_)));
    }

    #[test]
    fn deployment_with_floating_note_stays_deployment() {
        let input = "@startuml\nnode Server\nnote \"Primary server\" as N1\nN1 .. Server\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Deployment(_)));
    }

    #[test]
    fn note_only_diagram_stays_class() {
        let input = "@startuml\nnote as N\n  file: example.puml\nend note\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn comment_only_uml_stays_sequence_welcome_path() {
        let input = "@startuml\n' comment only\n@enduml";
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
    fn split_records_block_start_lines() {
        let input = "!define ALICE Alice\n\n@startuml first\nAlice -> Bob\n@enduml\n\n@startuml second\nBob -> Alice\n@enduml";
        let blocks = split_blocks(input);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].start_line, 3);
        assert_eq!(blocks[1].start_line, 7);
    }

    #[test]
    fn parse_block_preserves_absolute_source_lines() {
        let input = "!define ALICE Alice\n!define BOB Bob\n\n@startuml\nALICE -> BOB : Hello\nBOB --> ALICE : Hi\n@enduml";
        let diagram = parse_block(input, 0).unwrap();
        let Diagram::Sequence(seq) = diagram else {
            panic!("expected sequence diagram");
        };
        assert_eq!(seq.participants[0].source_line, 5);
        let crate::diagram::sequence::Event::Message(message) = &seq.events[0] else {
            panic!("expected message event");
        };
        assert_eq!(message.source_line, 5);
    }

    #[test]
    fn parse_block_keeps_preamble_procedure_body_lines() {
        let input = concat!(
            "!procedure $stdflow($from, $to)\n",
            "  $from -> $to : request\n",
            "  $to --> $from : response\n",
            "!endprocedure\n",
            "\n",
            "@startuml\n",
            "$stdflow(\"Alice\", \"Bob\")\n",
            "@enduml"
        );
        let diagram = parse_block(input, 0).unwrap();
        let Diagram::Sequence(seq) = diagram else {
            panic!("expected sequence diagram");
        };
        let crate::diagram::sequence::Event::Message(first) = &seq.events[0] else {
            panic!("expected first message event");
        };
        let crate::diagram::sequence::Event::Message(second) = &seq.events[1] else {
            panic!("expected second message event");
        };
        assert_eq!(first.source_line, 2);
        assert_eq!(second.source_line, 3);
    }

    #[test]
    fn parse_block_keeps_inline_procedure_definition_lines() {
        let input = concat!(
            "@startuml\n",
            "!procedure $request($from, $to, $msg)\n",
            "  $from -> $to : $msg\n",
            "  activate $to\n",
            "  $to --> $from : response\n",
            "  deactivate $to\n",
            "!endprocedure\n",
            "\n",
            "$request(Client, Server, \"GET /users\")\n",
            "$request(Client, Server, \"POST /users\")\n",
            "@enduml"
        );
        let diagram = parse_block(input, 0).unwrap();
        let Diagram::Sequence(seq) = diagram else {
            panic!("expected sequence diagram");
        };
        assert_eq!(seq.participants[0].source_line, 2);
        assert_eq!(seq.participants[1].source_line, 2);
        let crate::diagram::sequence::Event::Message(first) = &seq.events[0] else {
            panic!("expected first message event");
        };
        let crate::diagram::sequence::Event::Message(first_response) = &seq.events[2] else {
            panic!("expected first response event");
        };
        let crate::diagram::sequence::Event::Message(second) = &seq.events[4] else {
            panic!("expected second message event");
        };
        let crate::diagram::sequence::Event::Message(second_response) = &seq.events[6] else {
            panic!("expected second response event");
        };
        assert_eq!(first.source_line, 2);
        assert_eq!(first_response.source_line, 4);
        assert_eq!(second.source_line, 2);
        assert_eq!(second_response.source_line, 4);
    }

    #[test]
    fn parse_block_sequence_with_preamble_meta_procedure_stays_sequence() {
        let input = concat!(
            "!procedure $stdheader()\n",
            "  title Standard Header\n",
            "  header Generated by RustUML\n",
            "!endprocedure\n",
            "\n",
            "@startuml\n",
            "$stdheader()\n",
            "Alice -> Bob : Hello\n",
            "@enduml"
        );
        let diagram = parse_block(input, 0).unwrap();
        let Diagram::Sequence(seq) = diagram else {
            panic!("expected sequence diagram");
        };
        assert_eq!(seq.meta.title_line, Some(2));
        assert_eq!(seq.meta.header_line, Some(3));
        assert_eq!(seq.participants[0].source_line, 8);
        assert_eq!(seq.participants[1].source_line, 8);
        let crate::diagram::sequence::Event::Message(message) = &seq.events[0] else {
            panic!("expected message event");
        };
        assert_eq!(message.source_line, 8);
    }

    #[test]
    fn parse_block_keeps_while_body_and_following_source_lines() {
        let input = concat!(
            "@startuml\n",
            "!$i = 1\n",
            "!while $i <= 3\n",
            "  participant \"P$i\" as P$i\n",
            "  !$i = $i + 1\n",
            "!endwhile\n",
            "\n",
            "P1 -> P2 : step 1\n",
            "P2 -> P3 : step 2\n",
            "@enduml"
        );
        let diagram = parse_block(input, 0).unwrap();
        let Diagram::Sequence(seq) = diagram else {
            panic!("expected sequence diagram");
        };
        assert_eq!(seq.participants.len(), 3);
        assert!(seq.participants.iter().all(|p| p.source_line == 3));

        let crate::diagram::sequence::Event::Message(first) = &seq.events[0] else {
            panic!("expected first message event");
        };
        let crate::diagram::sequence::Event::Message(second) = &seq.events[1] else {
            panic!("expected second message event");
        };
        assert_eq!(first.source_line, 7);
        assert_eq!(second.source_line, 8);
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
