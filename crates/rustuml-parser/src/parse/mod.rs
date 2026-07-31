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

mod style;

use crate::diagram::Diagram;
use crate::diagram::class::PackageKind;
use crate::preprocess;

const NAMED_NOTE_CODE_PATTERN: &str = r"[\p{L}\p{N}_.]+";

// Java provenance: `ColorTrieNode` registers this case-insensitive inventory
// for `HColorSet.parseSimpleColor`, including PlantUML's ArchiMate aliases.
const PLANTUML_NAMED_COLORS: &str = "
aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond
blue blueviolet brown burlywood cadetblue chartreuse chocolate coral
cornflowerblue cornsilk crimson cyan darkblue darkcyan darkgoldenrod darkgray
darkgrey darkgreen darkkhaki darkmagenta darkolivegreen darkorange darkorchid
darkred darksalmon darkseagreen darkslateblue darkslategray darkslategrey
darkturquoise darkviolet deeppink deepskyblue dimgray dimgrey dodgerblue
firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite gold goldenrod
gray grey green greenyellow honeydew hotpink indianred indigo ivory khaki
lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan
lightgoldenrodyellow lightgray lightgrey lightgreen lightpink lightsalmon
lightseagreen lightskyblue lightslategray lightslategrey lightsteelblue
lightyellow lime limegreen linen magenta maroon mediumaquamarine mediumblue
mediumorchid mediumpurple mediumseagreen mediumslateblue mediumspringgreen
mediumturquoise mediumvioletred midnightblue mintcream mistyrose moccasin
navajowhite navy oldlace olive olivedrab orange orangered orchid palegoldenrod
palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum
powderblue purple red rosybrown royalblue saddlebrown salmon sandybrown seagreen
seashell sienna silver skyblue slateblue slategray slategrey snow springgreen
steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke yellow
yellowgreen business application motivation strategy technology physical
implementation
";

fn named_note_simple_color_is_resolvable(value: &str) -> bool {
    if matches!(value.len(), 1 | 3 | 6 | 8)
        && value.chars().all(|character| character.is_ascii_hexdigit())
    {
        return true;
    }
    PLANTUML_NAMED_COLORS
        .split_ascii_whitespace()
        .any(|name| name.eq_ignore_ascii_case(value))
}

fn named_note_whole_color_is_resolvable(value: &str) -> bool {
    // `HColorSet.parseColor` handles these sentinels before delegating to
    // `parseSimpleColor`; gradient endpoints bypass this branch.
    matches!(
        value.to_ascii_lowercase().as_str(),
        "transparent" | "background" | "automatic"
    ) || named_note_simple_color_is_resolvable(value)
}

fn named_note_color_is_valid(color: &str) -> bool {
    let Some(value) = color.strip_prefix('#') else {
        return false;
    };
    let is_word = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
    };
    let separators = value
        .char_indices()
        .filter(|(_, character)| matches!(character, '-' | '\\' | '|' | '/'))
        .collect::<Vec<_>>();
    match separators.as_slice() {
        [] => value.len() >= 2 && is_word(value) && named_note_whole_color_is_resolvable(value),
        [(index, separator)] => {
            let right_index = *index + separator.len_utf8();
            let left = &value[..*index];
            let right = &value[right_index..];
            is_word(left)
                && is_word(right)
                && named_note_simple_color_is_resolvable(left)
                && named_note_simple_color_is_resolvable(right)
        }
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NamedNoteCommand {
    pub display: Option<String>,
    pub code: String,
    pub tags: Vec<String>,
    pub stereotype: Option<String>,
    pub color: Option<String>,
}

fn named_note_decorations(suffix: &str) -> Option<(Vec<String>, Option<String>, Option<String>)> {
    static DECORATIONS: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r#"^\s*(?:(\$[^\s{}\"<>$]+(?:\s+\$[^\s{}\"<>$]+)*))?\s*(?:(<<.+?>>))?\s*(#[^\s]+)?\s*$"#,
        )
        .unwrap()
    });
    let captures = DECORATIONS.captures(suffix)?;
    let tags = captures
        .get(1)
        .map(|tags| {
            tags.as_str()
                .split_whitespace()
                .map(|tag| tag.trim_start_matches('$').to_string())
                .collect()
        })
        .unwrap_or_default();
    let stereotype = captures.get(2).map(|stereotype| {
        stereotype
            .as_str()
            .trim_start_matches("<<")
            .trim_end_matches(">>")
            .to_string()
    });
    let color = captures.get(3).map(|color| color.as_str().to_string());
    if color
        .as_deref()
        .is_some_and(|color| !named_note_color_is_valid(color))
    {
        return None;
    }
    Some((tags, stereotype, color))
}

pub(super) fn parse_named_note_inline(line: &str) -> Option<NamedNoteCommand> {
    static COMMAND: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(&format!(
            r#"^(?i:note)\s+\"([^\"]+)\"\s+(?i:as)\s+({NAMED_NOTE_CODE_PATTERN})(.*)$"#
        ))
        .unwrap()
    });
    let captures = COMMAND.captures(line)?;
    let (tags, stereotype, color) = named_note_decorations(captures.get(3)?.as_str())?;
    Some(NamedNoteCommand {
        display: Some(captures.get(1)?.as_str().replace("\\n", "\n")),
        code: captures.get(2)?.as_str().to_string(),
        tags,
        stereotype,
        color,
    })
}

pub(super) fn looks_like_named_note_inline_command(line: &str) -> bool {
    static PREFIX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r#"^(?i:note)\s+"[^"]*"\s+(?i:as)(?:\s|$)"#).unwrap()
    });
    PREFIX.is_match(line)
}

pub(super) fn parse_named_note_multiline(line: &str) -> Option<NamedNoteCommand> {
    static COMMAND: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(&format!(
            r"^(?i:note)\s+(?i:as)\s+({NAMED_NOTE_CODE_PATTERN})(.*)$"
        ))
        .unwrap()
    });
    let captures = COMMAND.captures(line)?;
    let (tags, stereotype, color) = named_note_decorations(captures.get(2)?.as_str())?;
    Some(NamedNoteCommand {
        display: None,
        code: captures.get(1)?.as_str().to_string(),
        tags,
        stereotype,
        color,
    })
}

pub(super) fn looks_like_named_note_multiline_command(line: &str) -> bool {
    static PREFIX: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^(?i:note)\s+(?i:as)(?:\s|$)").unwrap());
    PREFIX.is_match(line)
}

/// Return the ordinary-key identity used by PlantUML's
/// `SkinParam.cleanForKeySlow`.
fn normalized_skinparam_spelling(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace(['_', '.'], "")
}

/// Return the ordinary-key identity used by Java
/// `SkinParam.cleanForKeySlow`.
///
/// Renderers use this when consuming retained source spellings from
/// [`crate::diagram::DiagramMeta::skinparams`].
pub fn canonical_skinparam_key(key: &str) -> String {
    let mut canonical = normalized_skinparam_spelling(key);
    canonical = canonical
        .replace("sequenceparticipant", "participant")
        .replace("sequenceactor", "actor");
    for prefix in [
        "activity",
        "class",
        "component",
        "object",
        "sequence",
        "state",
        "usecase",
    ] {
        canonical = canonical.replace(&format!("{prefix}arrow"), "arrow");
    }
    if canonical.ends_with("align") {
        canonical.truncate(canonical.len() - "align".len());
        canonical.push_str("alignment");
    }
    canonical
}

/// Reconstruct theme execution order and apply PlantUML's ordinary
/// `SkinParam.setParam` key identity to parsed metadata.
fn collapse_reassigned_skinparams(diagram: &mut Diagram) {
    let params = &mut diagram.meta_mut().skinparams;
    let drained = std::mem::take(params);
    let mut source = Vec::with_capacity(drained.len());
    let mut theme_bodies = std::collections::VecDeque::new();
    let mut iter = drained.into_iter();

    while let Some(param) = iter.next() {
        if !param.key.eq_ignore_ascii_case("__theme_body_start") {
            source.push(param);
            continue;
        }
        let name = param.value;
        let mut body = Vec::new();
        for body_param in iter.by_ref() {
            if body_param.key.eq_ignore_ascii_case("__theme_body_end")
                && body_param.value.eq_ignore_ascii_case(&name)
            {
                break;
            }
            body.push(body_param);
        }
        theme_bodies.push_back((name, body));
    }

    // `TContext.executeTheme` runs each body at the directive before reading
    // the next user line. The preprocessor relocates bodies only to preserve
    // source-line numbers, so splice them back into that logical order here.
    let mut logical = Vec::with_capacity(source.len());
    let mut theme_segment = 0usize;
    for param in source {
        let theme_name = param
            .key
            .eq_ignore_ascii_case("__theme")
            .then(|| param.value.clone());
        logical.push((None, param));
        let Some(theme_name) = theme_name else {
            continue;
        };
        let Some(body_index) = theme_bodies
            .iter()
            .position(|(name, _)| name.eq_ignore_ascii_case(&theme_name))
        else {
            continue;
        };
        let (_, body) = theme_bodies.remove(body_index).unwrap();
        theme_segment += 1;
        logical.extend(
            body.into_iter()
                .map(|body_param| (Some(theme_segment), body_param)),
        );
    }

    let mut effective: Vec<(Option<String>, crate::diagram::SkinParam)> =
        Vec::with_capacity(logical.len());
    for (theme_segment, mut param) in logical {
        if param.key.starts_with("__") {
            effective.push((None, param));
            continue;
        }
        let replacement_key = if let Some(theme_segment) = theme_segment {
            format!("theme:{theme_segment}:{}", param.key.to_ascii_lowercase())
        } else {
            let canonical = canonical_skinparam_key(&param.key);
            param.key = if canonical == "arrowcolor" {
                canonical.clone()
            } else {
                normalized_skinparam_spelling(&param.key)
            };
            format!("ordinary:{canonical}")
        };
        if let Some(existing) = effective
            .iter()
            .position(|(key, _)| key.as_deref() == Some(replacement_key.as_str()))
        {
            effective.remove(existing);
        }
        effective.push((Some(replacement_key), param));
    }

    *params = effective.into_iter().map(|(_, param)| param).collect();
}

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

/// PlantUML's `CommandAllowMixing#getRegexConcat` anchors `allow_?mixing` at
/// both ends, while `Pattern2#compileInternal` makes the match case-insensitive.
/// Keep factory selection and command consumption on this one grammar.
pub(crate) fn is_allow_mixing_command(line: &str) -> bool {
    let line = line.trim();
    line.eq_ignore_ascii_case("allowmixing") || line.eq_ignore_ascii_case("allow_mixing")
}

fn resembles_allow_mixing_command(line: &str) -> bool {
    if is_allow_mixing_command(line) {
        return false;
    }
    let lower = line.trim().to_ascii_lowercase();
    let mut words = lower.split_whitespace();
    let Some(first) = words.next() else {
        return false;
    };
    if first == "allow" {
        return words.next().is_some_and(|word| word == "mixing");
    }
    let Some(suffix) = first.strip_prefix("allow") else {
        return false;
    };
    if suffix.trim_start_matches('_') != "mixing" {
        return false;
    }
    if !matches!(first, "allowmixing" | "allow_mixing") {
        return true;
    }
    words.next().is_some_and(|word| {
        !word
            .chars()
            .next()
            .is_some_and(|ch| matches!(ch, '-' | '.' | '<' | '>' | '*' | 'o'))
    })
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
///
/// This models `PSystemBuilder.createPSystem`, which tries
/// `SequenceDiagramFactory`, `ClassDiagramFactory`, and the later UML command
/// factories in order, accepting the first factory whose commands parse the
/// complete source. Explicit JSON and YAML starts bypass this competition via
/// `UmlSource.getDiagramTypes` and their dedicated `JsonDiagramFactory` or
/// `YamlDiagramFactory`.
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
    let mut has_class_lollipop_command = false;
    let mut has_direction_directive = false;
    let mut has_floating_note = false;
    let mut has_interface_decl = false;
    let mut has_component_bracket_interface_decl = false;
    let mut has_component_leaf_keyword = false;
    let mut has_quoted_deployment_container = false;
    let mut has_component_package_container = false;
    let mut has_top_level_component_leaf = false;
    let mut has_description_only_leaf = false;
    let mut has_non_interface_class_decl = false;
    let mut has_class_factory_decl = false;
    let mut has_entity_class_factory_decl = false;
    let mut class_factory_rejected_by_mixed_leaf = false;
    let mut has_class_symbol_container = false;
    let mut has_native_object_or_map = false;
    let mut object_containers_all_ordinary = true;
    let mut quoted_shared_deployment_containers = 0i32;
    let mut brace_depth = 0usize;
    let mut class_leaf_body_depth = None;

    let multiline_note_payload = completed_multiline_note_payload(lines);
    for (line_index, line) in lines.iter().enumerate() {
        // Java `PSystemCommandFactory` accumulates a complete multiline
        // command before factory selection continues. `CommandMultilines2`
        // validates the opener and terminator; note-body lines are display
        // data and cannot become participant or DESCRIPTION commands.
        if multiline_note_payload[line_index] {
            continue;
        }
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
        let inside_class_leaf_body =
            class_leaf_body_depth.is_some_and(|depth| brace_depth >= depth);
        if is_allow_mixing_command(trimmed) {
            has_allowmixing = true;
        }
        has_class_lollipop_command |= looks_like_class_lollipop_command(trimmed);
        // `CommandCreateElementFull2(NORMAL_KEYWORD)` is registered after
        // native class/object declarations and covers
        // `CommandCreateElementFull.ALL_TYPES` plus `state`. Its execution
        // error abandons this ClassDiagramFactory candidate immediately when
        // `allowmixing` has not already run.
        const MIXED_ONLY_CLASS_KEYWORDS: &[&str] = &[
            "person",
            "artifact",
            "actor/",
            "actor",
            "folder",
            "card",
            "file",
            "package",
            "rectangle",
            "hexagon",
            "label",
            "node",
            "frame",
            "cloud",
            "action",
            "process",
            "database",
            "queue",
            "stack",
            "storage",
            "agent",
            "usecase/",
            "usecase",
            "component",
            "boundary",
            "control",
            "collections",
            "port",
            "portin",
            "portout",
            "state",
        ];
        // Pattern2.compileInternal applies CASE_INSENSITIVE to PlantUML
        // command regexes, including CommandCreateElementFull2.
        let leading_keyword = trimmed
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if trimmed.trim_end().ends_with('{')
            && (leading_keyword == "namespace"
                || PackageKind::from_command_symbol(&leading_keyword).is_some())
        {
            // `CommandPackage`, `CommandNamespace`, and
            // `CommandPackageWithUSymbol` are registered before the mixed
            // DESCRIPTION leaf consumer in ClassDiagramFactory.
            has_class_symbol_container = true;
            has_class_factory_decl = true;
            if !matches!(leading_keyword.as_str(), "package" | "namespace") {
                object_containers_all_ordinary = false;
            }
        }
        if !has_allowmixing
            && !trimmed.contains('{')
            && MIXED_ONLY_CLASS_KEYWORDS.contains(&leading_keyword.as_str())
            && (leading_keyword != "component" || looks_like_component_keyword_declaration(trimmed))
        {
            class_factory_rejected_by_mixed_leaf = true;
        }
        if !inside_class_leaf_body && component::looks_like_description_bracket_command(trimmed) {
            class_factory_rejected_by_mixed_leaf = true;
        }
        let looks_like_sequence_message = sequence::looks_like_message(trimmed);
        let looks_like_sequence_participant = !trimmed.ends_with('{')
            && (trimmed.starts_with("participant ")
                || trimmed.starts_with("actor ")
                || trimmed.starts_with("boundary ")
                || trimmed.starts_with("control ")
                || trimmed.starts_with("database ")
                || trimmed.starts_with("collections ")
                || trimmed.starts_with("queue ")
                || trimmed.starts_with("entity "));
        let looks_like_page_meta = trimmed == "header"
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
            || trimmed == "title"
            || trimmed.starts_with("title ")
            || trimmed == "caption"
            || trimmed.starts_with("caption ");
        let top_level = brace_depth == 0;
        let entity_declaration = trimmed
            .strip_prefix("entity ")
            .map(str::trim)
            .filter(|declaration| !declaration.is_empty());
        // Java's CommandCreateClass accepts a brace-free entity declaration
        // and an optional empty `{ }`, while CommandCreateClassMultilines
        // accepts exactly one final opening brace.
        let entity_inline_empty_body = entity_declaration.is_some_and(|declaration| {
            declaration
                .strip_suffix('}')
                .map(str::trim_end)
                .and_then(|declaration| declaration.strip_suffix('{'))
                .is_some_and(|declaration| !declaration.trim().is_empty())
        });
        let entity_multiline_body = entity_declaration.is_some_and(|declaration| {
            declaration
                .strip_suffix('{')
                .map(str::trim_end)
                .is_some_and(|declaration| !declaration.is_empty() && !declaration.ends_with('{'))
        });
        let entity_bare_declaration = entity_declaration
            .is_some_and(|declaration| !declaration.contains('{') && !declaration.contains('}'));

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
            // `Pattern2#compileInternal` makes DESCRIPTION command regexes
            // case-insensitive. Factory selection must consume the same
            // semantic token as the parsers behind it.
            let normalized_keyword = trimmed[..kw_end].to_ascii_lowercase();
            let kw = normalized_keyword.as_str();
            let package_with_brace = kw == "package" && trimmed.contains('{');
            if package_with_brace {
                has_component_package_container = true;
            }
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
            // Rust splits Java's DescriptionDiagramFactory between Component
            // and Deployment models. These valid DESCRIPTION leaves are not
            // consumed by the Component parser, so one occurrence rejects that
            // candidate regardless of other component declarations.
            has_description_only_leaf |= !trimmed.contains('{')
                && matches!(kw, "card" | "stack" | "file" | "agent")
                && kw_end < trimmed.len()
                && deployment_keyword_arg;
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
            if is_deploy_exclusive_container {
                // Deployment-exclusive containers override component score.
                scores[7] += 20;
            }
            if is_quoted_container {
                has_quoted_deployment_container = true;
                if !is_deploy_exclusive_container {
                    quoted_shared_deployment_containers += 1;
                }
            }
        }
        // Component — weighted strongly so that a single `component` keyword
        // beats multiple `interface` lines that would otherwise score for class.
        if leading_keyword == "component" && trimmed["component".len()..].starts_with(' ') {
            scores[5] += 15;
            if !trimmed.contains('{') && top_level {
                has_top_level_component_leaf = true;
            }
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
            && !looks_like_sequence_message
            && !looks_like_sequence_participant
            && !looks_like_page_meta
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
        let normalized_note_line = trimmed.to_ascii_lowercase();
        if normalized_note_line.starts_with("note ")
            && (normalized_note_line.contains(" right of ")
                || normalized_note_line.contains(" left of ")
                || normalized_note_line.contains(" top of ")
                || normalized_note_line.contains(" bottom of "))
        {
            scores[0] += 5; // sequence
            scores[1] += 5; // class
            scores[5] += 5; // component
            scores[7] += 5; // deployment
        }
        // Object / map — strong unique keywords.
        if trimmed.starts_with("object ") || trimmed.starts_with("map ") {
            scores[2] += 10;
            has_native_object_or_map = true;
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
            has_class_factory_decl = true;
        }
        if trimmed.starts_with("interface ") {
            scores[1] += 10;
            has_interface_decl = true;
            has_class_factory_decl = true;
            if trimmed.starts_with("interface [") {
                // Java's component interface command accepts the bracket-name
                // form, while ClassDiagramFactory's interface declaration does
                // not consume it.
                has_component_bracket_interface_decl = true;
            }
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
        // Entity declarations are accepted by ClassDiagramFactory in all three
        // forms. Only the brace forms reject SequenceDiagramFactory on their
        // own; a bare entity remains a valid sequence participant.
        if entity_inline_empty_body || entity_multiline_body {
            scores[1] += 15;
        }
        if entity_bare_declaration || entity_inline_empty_body || entity_multiline_body {
            has_entity_class_factory_decl = true;
            has_class_factory_decl = true;
        }
        // Sequence. Skip lines that end with `{` — those are container blocks
        // (class diagram packages) not sequence participants.
        // `entity` is also a sequence participant type.
        if looks_like_sequence_participant && !trimmed.starts_with("actor ") {
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
        if trimmed.contains("->") || trimmed.contains("-->") || looks_like_sequence_message {
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
        if normalized_note_line.starts_with("note as ")
            || normalized_note_line.starts_with("note \"")
        {
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
        let opens = trimmed.chars().filter(|&c| c == '{').count();
        let closes = trimmed.chars().filter(|&c| c == '}').count();
        if !inside_class_leaf_body && opens > 0 && looks_like_class_leaf_body_opener(trimmed) {
            class_leaf_body_depth = Some(brace_depth + opens);
        }
        brace_depth = brace_depth.saturating_add(opens).saturating_sub(closes);
        if class_leaf_body_depth.is_some_and(|depth| brace_depth < depth) {
            class_leaf_body_depth = None;
        }
    }

    // Java keeps trying a factory only while every source command is
    // consumable. Once a mixed-only leaf rejects ClassDiagramFactory, a later
    // command cannot revive that candidate.
    let class_factory_viable = has_class_factory_decl
        && !class_factory_rejected_by_mixed_leaf
        && !has_component_bracket_interface_decl
        && scores[9] == 0;
    let object_only_container_model = has_native_object_or_map
        && object_containers_all_ordinary
        && !has_allowmixing
        && !has_non_interface_class_decl
        && !has_interface_decl
        && !has_entity_class_factory_decl;

    // `CommandLinkLollipop` belongs to the earlier ClassDiagramFactory. When
    // every source command remains consumable there, factory order wins over
    // DESCRIPTION's coincidental interpretation of an owner named Component.
    if has_class_lollipop_command && class_factory_viable {
        let other_max = scores
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 1)
            .map(|(_, &score)| score)
            .max()
            .unwrap_or(0);
        if scores[1] <= other_max {
            scores[1] = other_max + 1;
        }
    }

    // Every complete braced USymbol command is consumable by the earlier
    // ClassDiagramFactory. When no mixed leaf has rejected that candidate,
    // preserve factory order instead of letting deployment keyword weights
    // steal the same command.
    if has_class_symbol_container && class_factory_viable && !object_only_container_model {
        let other_max = scores
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 1)
            .map(|(_, &score)| score)
            .max()
            .unwrap_or(0);
        if scores[1] <= other_max {
            scores[1] = other_max + 1;
        }
    }

    // CommandPackageWithUSymbol makes a quoted shared container valid for both
    // CLASS and DESCRIPTION, but not SEQUENCE. If the complete source remains
    // consumable by ClassDiagramFactory, Java's earlier class factory wins.
    if quoted_shared_deployment_containers > 0
        && has_entity_class_factory_decl
        && class_factory_viable
    {
        scores[1] = scores[1].max(scores[7]);
    }

    // Java tries ClassDiagramFactory before the later shared-container
    // factories. Quoted node/frame/cloud/database containers only need their
    // deployment bonus when no declaration makes the class grammar viable.
    if !class_factory_viable {
        scores[7] += 20 * quoted_shared_deployment_containers;
    }

    if has_description_only_leaf {
        let competing = scores
            .iter()
            .enumerate()
            .filter(|&(index, _)| index != 7)
            .map(|(_, &score)| score)
            .max()
            .unwrap_or(0);
        scores[7] = scores[7].max(competing + 1);
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

    if has_quoted_deployment_container
        && !class_factory_viable
        && !has_component_package_container
        && !has_top_level_component_leaf
    {
        let other_max = scores
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 7)
            .map(|(_, &s)| s)
            .max()
            .unwrap_or(0);
        if scores[7] <= other_max {
            scores[7] = other_max + 1;
        }
    }

    if has_quoted_deployment_container
        && !class_factory_viable
        && (has_component_package_container || has_top_level_component_leaf)
    {
        let other_max = scores
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != 5)
            .map(|(_, &s)| s)
            .max()
            .unwrap_or(0);
        if scores[5] <= other_max {
            scores[5] = other_max + 1;
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

    // `allowmixing` is itself a class-diagram command. Java selects the class
    // factory even when every following leaf uses description syntax; the
    // selected parser remains responsible for rejecting commands it cannot
    // consume.
    if has_allowmixing && !class_factory_rejected_by_mixed_leaf {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NoteCommandFamily {
    Note,
    Hexagonal,
    Rectangular,
}

fn note_command_parts(line: &str) -> Option<(NoteCommandFamily, &str)> {
    let mut command = line.trim_start();
    command = command.strip_prefix('&').unwrap_or(command).trim_start();
    command = command.strip_prefix('/').unwrap_or(command).trim_start();
    let keyword_end = command.find(char::is_whitespace).unwrap_or(command.len());
    let keyword = &command[..keyword_end];
    let rest = command[keyword_end..].trim_start();
    let family = if keyword.eq_ignore_ascii_case("note") {
        NoteCommandFamily::Note
    } else if keyword.eq_ignore_ascii_case("hnote") {
        NoteCommandFamily::Hexagonal
    } else if keyword.eq_ignore_ascii_case("rnote") {
        NoteCommandFamily::Rectangular
    } else {
        return None;
    };
    Some((family, rest))
}

pub(super) fn note_command_family(line: &str) -> Option<NoteCommandFamily> {
    note_command_parts(line).map(|(family, _)| family)
}

pub(super) fn is_ordinary_note_terminator(line: &str) -> bool {
    let line = line.trim().to_ascii_lowercase();
    if line == "endnote" {
        return true;
    }
    let Some(rest) = line.strip_prefix("end") else {
        return false;
    };
    let mut chars = rest.chars();
    chars.next().is_some_and(char::is_whitespace) && chars.as_str() == "note"
}

pub(super) fn is_note_family_terminator(line: &str) -> bool {
    let line = line.trim();
    is_ordinary_note_terminator(line)
        || line.eq_ignore_ascii_case("end hnote")
        || line.eq_ignore_ascii_case("endhnote")
        || line.eq_ignore_ascii_case("end rnote")
        || line.eq_ignore_ascii_case("endrnote")
}

fn completed_multiline_note_payload(lines: &[String]) -> Vec<bool> {
    let mut payload = vec![false; lines.len()];
    let mut line_index = 0;
    while line_index < lines.len() {
        let opener = source_text(&lines[line_index]).trim();
        let Some((family, after_keyword)) = note_command_parts(opener) else {
            line_index += 1;
            continue;
        };
        if opener.contains(':') || after_keyword.starts_with('"') {
            line_index += 1;
            continue;
        }

        let closes_with_brace = family == NoteCommandFamily::Note && opener.ends_with('{');
        let sequence_shaped = family != NoteCommandFamily::Note || {
            let rest = after_keyword.to_ascii_lowercase();
            (rest.starts_with("left")
                || rest.starts_with("right")
                || rest.starts_with("over")
                || rest.starts_with("across"))
                && !rest.contains(" of ")
        };
        let candidates = (line_index + 1)..lines.len();
        let terminator = if closes_with_brace {
            candidates
                .clone()
                .find(|&candidate| source_text(&lines[candidate]).trim() == "}")
        } else if family == NoteCommandFamily::Note {
            candidates
                .clone()
                .find(|&candidate| is_ordinary_note_terminator(source_text(&lines[candidate])))
                .or_else(|| {
                    if sequence_shaped {
                        candidates.clone().find(|&candidate| {
                            is_note_family_terminator(source_text(&lines[candidate]))
                        })
                    } else {
                        None
                    }
                })
        } else {
            candidates
                .clone()
                .find(|&candidate| is_note_family_terminator(source_text(&lines[candidate])))
        };
        let Some(terminator) = terminator else {
            // Only completed commands own their interior. An unterminated
            // opener must not hide arbitrary later syntax from dispatch.
            line_index += 1;
            continue;
        };
        payload[(line_index + 1)..=terminator].fill(true);
        line_index = terminator + 1;
    }
    payload
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

fn looks_like_class_lollipop_command(line: &str) -> bool {
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r#"^(?:"[^"]+"|[\p{L}\p{N}_./:]+)\s*(?:"[^"]+")?\s*(?:[-=.]+\([()]|[()]\)[-=.]+)\s*(?:"[^"]+")?\s*(?:"[^"]+"|[\p{L}\p{N}_./:]+)(?:\s*:\s*.+)?$"#,
        )
        .unwrap()
    });
    RE.is_match(line)
}

/// `CommandCreateElementFull` only treats `component` as a declaration when
/// the text after the keyword starts a legal name. An entity whose identifier
/// happens to be `Component` may instead begin a relationship command.
fn looks_like_component_keyword_declaration(line: &str) -> bool {
    const KEYWORD: &str = "component";
    if line.len() <= KEYWORD.len()
        || !line[..KEYWORD.len()].eq_ignore_ascii_case(KEYWORD)
        || !line[KEYWORD.len()..].starts_with(char::is_whitespace)
    {
        return false;
    }
    line[KEYWORD.len()..]
        .trim_start()
        .chars()
        .next()
        .is_some_and(|ch| ch == '"' || ch == '_' || ch.is_alphanumeric())
}

fn looks_like_class_leaf_body_opener(line: &str) -> bool {
    if !line.trim_end().ends_with('{') {
        return false;
    }
    let normalized = line.trim_start().to_ascii_lowercase();
    let mut words = normalized.split_whitespace();
    let first = words.next().unwrap_or_default();
    matches!(
        first,
        "class"
            | "interface"
            | "enum"
            | "annotation"
            | "entity"
            | "object"
            | "map"
            | "protocol"
            | "struct"
            | "exception"
            | "metaclass"
            | "stereotype"
    ) || (first == "abstract" && words.next() == Some("class"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    let mut style_program = style::extract_style_program(&mut preprocess_out.lines);
    if typ == "uml" {
        for (index, line) in preprocess_out.lines.iter().enumerate() {
            let (source_line, trimmed) = source_line_and_trimmed(index + 1, line);
            if resembles_allow_mixing_command(trimmed) {
                return Err(ParseError {
                    line: source_line,
                    message: "invalid allowmixing command".to_string(),
                });
            }
        }
    }
    let uml_subtype = (typ == "uml").then(|| detect_uml_subtype(&preprocess_out.lines));
    if matches!(uml_subtype, Some(UmlSubtype::Sequence)) {
        preprocess_out = match base_dir {
            Some(dir) => {
                preprocess::preprocess_full_for_sequence_parse(input, Some(dir.to_path_buf()))
            }
            None => preprocess::preprocess_full_for_sequence_parse(input, None),
        };
        style_program = style::extract_style_program(&mut preprocess_out.lines);
    }
    let lines = preprocess_out.lines;
    let sprites = preprocess_out.sprites;
    let uml_source = preprocess_out.uml_source;

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

    collapse_reassigned_skinparams(&mut diagram);
    diagram.meta_mut().style_program = style_program;

    // Inject sprite definitions from the preprocessor into the diagram's meta.
    if !sprites.is_empty() {
        let meta = diagram.meta_mut();
        meta.sprites = sprites;
    }

    // Stash PlantUML's post-TIM `UmlSource.getPlainString("\n")` for
    // downstream seed computation (filter UIDs, gradient/shadow ids).
    // `PSystemBuilder.createPSystem` constructs `UmlSource` from the expanded
    // preprocessor stream rather than the raw input.
    {
        let meta = diagram.meta_mut();
        meta.source = Some(uml_source);
    }

    Ok(diagram)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_note_command_grammar_preserves_ordered_decorations() {
        let inline = parse_named_note_inline(
            r#"note "payload" as Métrique.Δelta_7 $audit $retained <<Ledger>> #MistyRose"#,
        )
        .unwrap();
        assert_eq!(inline.display.as_deref(), Some("payload"));
        assert_eq!(inline.code, "Métrique.Δelta_7");
        assert_eq!(inline.tags, ["audit", "retained"]);
        assert_eq!(inline.stereotype.as_deref(), Some("Ledger"));
        assert_eq!(inline.color.as_deref(), Some("#MistyRose"));

        let multiline = parse_named_note_multiline(
            "note as Ledger.C464 $audit $retained <<Ledger>> #LightBlue",
        )
        .unwrap();
        assert_eq!(multiline.display, None);
        assert_eq!(multiline.code, "Ledger.C464");
        assert_eq!(multiline.tags, inline.tags);
        assert_eq!(multiline.stereotype, inline.stereotype);

        assert!(parse_named_note_inline(r#"note "payload" as ValidPrefix-invalid"#).is_none());
        assert!(parse_named_note_multiline("note as ValidCode #Red <<WrongOrder>>").is_none());
    }

    #[test]
    fn named_note_colors_pass_color_parser_and_hcolor_resolution() {
        for color in [
            "#MistyRose",
            "#bUsInEsS",
            "#ABC",
            "#123456",
            "#12345678",
            "#A-B",
            "#Red/LightBlue",
            "#transparent",
            "#background",
            "#automatic",
        ] {
            let inline = parse_named_note_inline(&format!(
                "note \"inline payload\" as InlineLedger {color}"
            ))
            .unwrap();
            let multiline =
                parse_named_note_multiline(&format!("note as MultilineLedger {color}")).unwrap();
            assert_eq!(inline.color.as_deref(), Some(color));
            assert_eq!(multiline.color.as_deref(), Some(color));
        }

        for color in [
            "#A",
            "#NoSuchColor",
            "#back:LightBlue;line.dashed:Red",
            "#Red-UnknownColor",
            "#Red-Blue-Green",
            "#transparent/Red",
            "#automatic-Blue",
            "#Red|background",
        ] {
            assert!(
                parse_named_note_inline(&format!(
                    "note \"invalid payload\" as InvalidLedger {color}"
                ))
                .is_none(),
                "{color}"
            );
            assert!(
                parse_named_note_multiline(&format!("note as InvalidLedger {color}")).is_none(),
                "{color}"
            );
        }
    }

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
    fn multiline_note_bodies_are_opaque_to_uml_subtype_scoring() {
        let lines = |source: &str| source.lines().map(str::to_string).collect::<Vec<_>>();

        for body in [
            "control queue hold",
            "boundary transit edge",
            "node payload text",
            "state payload text",
        ] {
            let source = format!("note as DispatchMemo\n{body}\nend note");
            assert_eq!(detect_uml_subtype(&lines(&source)), UmlSubtype::Class);
        }

        let attached = lines(
            "class Ledger\n\
             note right of Ledger\n\
             control is display text\n\
             boundary is display text too\n\
             end note",
        );
        assert_eq!(detect_uml_subtype(&attached), UmlSubtype::Class);

        let bracketed = lines(
            "class Ledger\n\
             note left of Ledger #LightYellow {\n\
             control is display text\n\
             boundary is display text too\n\
             }",
        );
        assert_eq!(detect_uml_subtype(&bracketed), UmlSubtype::Class);

        let mixed_case = lines("NoTe as DispatchMemo\ncontrol is display text\nEnD NoTe");
        assert_eq!(detect_uml_subtype(&mixed_case), UmlSubtype::Class);

        let wrong_family_text = lines(
            "note as DispatchMemo\n\
             end hnote\n\
             component remains display text\n\
             end note\n\
             class Ledger",
        );
        assert_eq!(detect_uml_subtype(&wrong_family_text), UmlSubtype::Class);

        let tab_terminator =
            lines("note as DispatchMemo\ncontrol is display text\nend\tnote\nclass Ledger");
        assert_eq!(detect_uml_subtype(&tab_terminator), UmlSubtype::Class);

        for source in [
            "participant A\nhnote over A\ncomponent payload\nend hnote",
            "participant A\nrnote right of A\nnode payload\nendrnote",
        ] {
            assert_eq!(detect_uml_subtype(&lines(source)), UmlSubtype::Sequence);
        }
    }

    #[test]
    fn multiline_note_boundaries_leave_real_dispatch_syntax_visible() {
        let lines = |source: &str| source.lines().map(str::to_string).collect::<Vec<_>>();

        let post_terminator = lines(
            "note as DispatchMemo\n\
             control is display text\n\
             end note\n\
             node RuntimeNode",
        );
        assert_eq!(detect_uml_subtype(&post_terminator), UmlSubtype::Deployment);

        let unterminated = lines("note as DispatchMemo\ncontrol RuntimeControl");
        assert_eq!(detect_uml_subtype(&unterminated), UmlSubtype::Deployment);

        let single_line = lines(
            "note right of Ledger : control remains inline text\n\
             node RuntimeNode",
        );
        assert_eq!(detect_uml_subtype(&single_line), UmlSubtype::Deployment);
    }

    #[test]
    fn description_only_leaves_reject_the_component_subtype() {
        let lines = |source: &str| source.lines().map(str::to_string).collect::<Vec<_>>();

        for keyword in ["card", "stack", "file", "agent"] {
            for source in [
                format!("component SharedComponent\n{keyword} ExclusiveLeaf"),
                format!("{keyword} \"Exclusive Leaf\" as ExclusiveLeaf\ncomponent SharedComponent"),
            ] {
                assert_eq!(
                    detect_uml_subtype(&lines(&source)),
                    UmlSubtype::Deployment,
                    "{source}"
                );
            }
        }

        for keyword in ["artifact", "storage"] {
            let source = format!("component SharedComponent\n{keyword} SharedLeaf");
            assert_eq!(
                detect_uml_subtype(&lines(&source)),
                UmlSubtype::Component,
                "{source}"
            );
        }
    }

    #[test]
    fn parses_simple_sequence() {
        let input = "@startuml\nAlice -> Bob : hello\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Sequence(_)));
    }

    #[test]
    fn standalone_bare_entity_remains_sequence() {
        let input = "@startuml\nentity Ledger\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Sequence(_)));
    }

    #[test]
    fn quoted_shared_containers_with_entity_forms_select_class() {
        let declarations = [
            ("bare", "entity Ledger"),
            ("inline empty", "entity Ledger {}"),
            (
                "multiline body",
                "entity Ledger {\n  +id : UUID\n  +post(entry : Entry)\n}",
            ),
        ];

        for keyword in ["frame", "node", "cloud", "database"] {
            for (form, declaration) in declarations {
                let input = format!(
                    "@startuml\n{keyword} \"Shared Boundary\" as Shared {{\n{declaration}\n}}\n@enduml"
                );
                assert!(
                    matches!(parse(&input).unwrap(), Diagram::Class(_)),
                    "{keyword} with {form} entity"
                );
            }
        }
    }

    #[test]
    fn quoted_shared_containers_with_explicit_classes_select_class() {
        for keyword in ["node", "frame", "cloud", "database"] {
            let input = format!(
                "@startuml\n{keyword} \"Fresh Shared Container\" as Shared {{\nclass FreshLeaf\n}}\n@enduml"
            );
            assert!(
                matches!(parse(&input).unwrap(), Diagram::Class(_)),
                "{keyword}"
            );
        }
    }

    #[test]
    fn symbol_container_does_not_steal_expanded_archimate_source() {
        let input = "@startuml\n\
                     rectangle \"Business Layer\" {\n\
                       archimate_element Business Actor customer \"Customer\"\n\
                       archimate_element Business Process checkout \"Checkout\"\n\
                       archimate_rel Serving checkout customer \"serves\"\n\
                     }\n\
                     @enduml";
        assert!(matches!(parse(input).unwrap(), Diagram::Archimate(_)));
    }

    #[test]
    fn symbol_container_does_not_steal_component_bracket_interface() {
        let input = "@startuml\n\
                     component RenamedShell9803 {\n\
                       interface [Renamed Audit Store 9817] as Store9817\n\
                     }\n\
                     @enduml";
        assert!(matches!(parse(input).unwrap(), Diagram::Component(_)));
    }

    #[test]
    fn description_bracket_commands_reject_shared_class_container_candidate() {
        let input = "@startuml\n\
                     component FreshFrontend7311 {\n\
                       [Fresh Login 7313] as Login7313\n\
                       [Fresh Dashboard 7317]\n\
                     }\n\
                     component FreshBackend7321 {\n\
                       [Fresh Auth 7323] as Auth7323\n\
                     }\n\
                     Login7313 -right-> Auth7323 : calls\n\
                     [Fresh Dashboard 7317] --> Auth7323 : fetches\n\
                     @enduml";
        let Diagram::Component(diagram) = parse(input).unwrap() else {
            panic!("description bracket commands must select Component");
        };
        assert_eq!(diagram.packages.len(), 2);
        assert_eq!(diagram.connections.len(), 2);
        assert!(diagram.components.iter().any(|component| {
            component.id == "Login7313" && component.label == "Fresh Login 7313"
        }));
        assert!(diagram.components.iter().any(|component| {
            component.id == "Fresh_Dashboard_7317" && component.label == "Fresh Dashboard 7317"
        }));
    }

    #[test]
    fn description_bracket_alias_directions_preserve_code_and_display() {
        let input = "@startuml\n\
                     [Fresh Primary 7331] as Primary7331\n\
                     Secondary7333 as [Fresh Secondary 7333]\n\
                     Primary7331 --> Secondary7333\n\
                     @enduml";
        let Diagram::Component(diagram) = parse(input).unwrap() else {
            panic!("bracket aliases must select Component");
        };
        assert!(diagram.components.iter().any(|component| {
            component.id == "Primary7331" && component.label == "Fresh Primary 7331"
        }));
        assert!(diagram.components.iter().any(|component| {
            component.id == "Secondary7333" && component.label == "Fresh Secondary 7333"
        }));
    }

    #[test]
    fn bracket_text_consumed_by_class_body_does_not_reject_class_factory() {
        let input = "@startuml\n\
                     package FreshTypes7341 {\n\
                       class FreshRecord7343 {\n\
                         +values : String[]\n\
                         [literal member row]\n\
                       }\n\
                     }\n\
                     @enduml";
        assert!(matches!(parse(input).unwrap(), Diagram::Class(_)));
    }

    #[test]
    fn quoted_shared_container_without_class_signal_stays_deployment() {
        let input =
            "@startuml\nnode \"Fresh Runtime Host\" as Host {\nartifact FreshBinary\n}\n@enduml";
        assert!(matches!(parse(input).unwrap(), Diagram::Deployment(_)));
    }

    #[test]
    fn component_leaf_makes_quoted_entity_source_fall_through_to_description() {
        for input in [
            "@startuml\n\
             top to bottom direction\n\
             entity Ledger\n\
             node \"Runtime Edge\" as Runtime {\n\
               component API\n\
             }\n\
             @enduml",
            "@startuml\n\
             left to right direction\n\
             cloud \"Runtime Edge\" {\n\
               component API\n\
             }\n\
             entity Ledger\n\
             @enduml",
        ] {
            assert!(matches!(parse(input).unwrap(), Diagram::Deployment(_)));
        }
    }

    #[test]
    fn mixed_description_keyword_family_rejects_the_class_factory() {
        for keyword in ["artifact", "DATABASE", "QuEuE", "rectangle", "BOUNDARY"] {
            let input = format!(
                "@startuml\n\
                 entity FreshLedger6111 {{}}\n\
                 node \"Fresh Runtime 6113\" as Runtime6113 {{\n\
                   {keyword} FreshMixed6117\n\
                 }}\n\
                 @enduml"
            );
            assert!(
                matches!(parse(&input).unwrap(), Diagram::Deployment(_)),
                "{keyword}"
            );
        }
    }

    #[test]
    fn uppercase_mixed_leaf_survives_description_factory_dispatch() {
        let input = "@startuml\n\
            package FreshOuter6121 {\n\
              cloud \"Fresh Shared Cloud 6123\" {\n\
                interface FreshContract6127\n\
                QUEUE FreshDeliveryQueue6131\n\
              }\n\
            }\n\
            @enduml";
        let parsed = parse(input).unwrap();
        let Diagram::Component(diagram) = parsed else {
            panic!("expected component-backed description diagram, got {parsed:?}");
        };
        assert!(diagram.components.iter().any(|component| {
            component.id == "FreshDeliveryQueue6131"
                && component.kind == crate::diagram::component::ComponentElementKind::Queue
        }));
    }

    #[test]
    fn root_description_commands_are_case_insensitive_during_dispatch() {
        for input in [
            "@startuml\n\
             left to right direction\n\
             DATABASE AuditStore7011\n\
             AuditStore7011 --> Ledger7013\n\
             @enduml",
            "@startuml\n\
             NoDe \"Runtime Mesh 7021\" As Mesh7023 {\n\
               QuEuE DeliveryQueue7027\n\
             }\n\
             @enduml",
        ] {
            assert!(matches!(parse(input).unwrap(), Diagram::Deployment(_)));
        }

        let Diagram::Component(diagram) =
            parse("@startuml\nCoMpOnEnT \"API Display 7031\" As Api7031\n@enduml").unwrap()
        else {
            panic!("expected component-backed description diagram");
        };
        assert!(
            diagram
                .components
                .iter()
                .any(|component| component.id == "Api7031" && component.label == "API Display 7031")
        );
    }

    #[test]
    fn native_class_keyword_family_keeps_the_earlier_class_factory() {
        for declaration in [
            "entity FreshNative6211",
            "interface FreshNative6211",
            "circle FreshNative6211",
        ] {
            let input = format!(
                "@startuml\n\
                 node \"Fresh Runtime 6213\" as Runtime6213 {{\n\
                   {declaration}\n\
                 }}\n\
                 @enduml"
            );
            assert!(
                matches!(parse(&input).unwrap(), Diagram::Class(_)),
                "{declaration}"
            );
        }
    }

    #[test]
    fn allowmixing_keeps_quoted_entity_and_component_source_in_class_factory() {
        for keyword in ["component", "DATABASE", "rectangle", "BoUnDaRy"] {
            let input = format!(
                "@startuml\n\
                 entity FreshLedger6311 {{}}\n\
                 allowmixing\n\
                 node \"Fresh Runtime 6313\" as Runtime6313 {{\n\
                   {keyword} FreshMixed6317\n\
                 }}\n\
                 @enduml"
            );
            assert!(
                matches!(parse(&input).unwrap(), Diagram::Class(_)),
                "{keyword}"
            );
        }
    }

    #[test]
    fn parse_carries_post_tim_uml_source_for_svg_identity() {
        let input = concat!(
            "@startuml\n",
            "!procedure $emit_badge($alias, $tone, $label, $message)\n",
            "  !if $message != \"\"\n",
            "    note as $alias\n",
            "      <back:$tone>**$label**: $message</back>\n",
            "    end note\n",
            "  !endif\n",
            "!endprocedure\n",
            "\n",
            "!$enabled = true\n",
            "!if $enabled\n",
            "$emit_badge(N_FreshLedger, LightCyan, \"AUDIT\", \"fresh payload\")\n",
            "!endif\n",
            "@enduml\n",
        );
        let diagram = parse(input).unwrap();

        assert_eq!(
            diagram.meta().source.as_deref(),
            Some(concat!(
                "@startuml\n",
                "\n",
                "    note as N_FreshLedger\n",
                "      <back:LightCyan>**AUDIT**: fresh payload</back>\n",
                "    end note\n",
                "@enduml\n",
            )),
        );
    }

    #[test]
    fn skinparam_only_uml_defaults_to_class() {
        let input = "@startuml\nskinparam backgroundColor #FFFEF0\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn repeated_skinparams_expose_normalized_latest_spelling_and_value() {
        let input = concat!(
            "@startuml\n",
            "skinparam ClassBackgroundColor #13579B\n",
            "class FreshLedgerOne {}\n",
            "class FreshLedgerTwo {}\n",
            "skinparam classBackgroundColor #2468AC\n",
            "class FreshLedgerThree {}\n",
            "class FreshLedgerFour {}\n",
            "class FreshLedgerFive {}\n",
            "@enduml\n",
        );
        let diagram = parse(input).unwrap();
        let matching: Vec<_> = diagram
            .meta()
            .skinparams
            .iter()
            .filter(|param| param.key.eq_ignore_ascii_case("classBackgroundColor"))
            .collect();

        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].key, "classbackgroundcolor");
        assert_eq!(matching[0].value, "#2468AC");
    }

    #[test]
    fn alternating_arrow_aliases_collapse_by_java_key_identity() {
        let input = concat!(
            "@startuml\n",
            "skinparam StAtEaRrOwCoLoR #1565C0\n",
            "skinparam ArrowColor #6A1B9A\n",
            "skinparam stateArrowColor #00838F\n",
            "skinparam aRrOwCoLoR #3949AB\n",
            "skinparam StAtEaRrOwCoLoR #2E7D32\n",
            "[*] --> FreshRelay\n",
            "FreshRelay --> [*]\n",
            "@enduml\n",
        );
        let diagram = parse(input).unwrap();
        let matching: Vec<_> = diagram
            .meta()
            .skinparams
            .iter()
            .filter(|param| canonical_skinparam_key(&param.key) == "arrowcolor")
            .collect();

        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].key, "arrowcolor");
        assert_eq!(matching[0].value, "#2E7D32");
    }

    #[test]
    fn every_ordinary_family_arrow_alias_uses_shared_storage_identity() {
        let input = concat!(
            "@startuml\n",
            "skinparam Activity.Arrow_Color #1565C0\n",
            "skinparam Class_Arrow.Color #6A1B9A\n",
            "skinparam Component.Arrow_Color #00838F\n",
            "skinparam Object_Arrow.Color #3949AB\n",
            "skinparam Sequence.Arrow_Color #AD1457\n",
            "skinparam State_Arrow.Color #EF6C00\n",
            "skinparam UseCase.Arrow_Color #2E7D32\n",
            "[*] --> FreshRelay\n",
            "FreshRelay --> [*]\n",
            "@enduml\n",
        );
        let diagram = parse(input).unwrap();
        let matching: Vec<_> = diagram
            .meta()
            .skinparams
            .iter()
            .filter(|param| canonical_skinparam_key(&param.key) == "arrowcolor")
            .collect();

        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].key, "arrowcolor");
        assert_eq!(matching[0].value, "#2E7D32");
    }

    #[test]
    fn separator_and_alignment_aliases_share_java_key_identity() {
        let input = concat!(
            "@startuml\n",
            "skinparam default_text_align left\n",
            "skinparam default.text.alignment right\n",
            "class FreshLedger\n",
            "@enduml\n",
        );
        let diagram = parse(input).unwrap();
        let matching: Vec<_> = diagram
            .meta()
            .skinparams
            .iter()
            .filter(|param| canonical_skinparam_key(&param.key) == "defaulttextalignment")
            .collect();

        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].key, "defaulttextalignment");
        assert_eq!(matching[0].value, "right");
    }

    #[test]
    fn canonical_skinparam_key_matches_java_cleanup_rules() {
        assert_eq!(
            canonical_skinparam_key(" State.Arrow_Font.Name "),
            "arrowfontname"
        );
        assert_eq!(
            canonical_skinparam_key("default.text_align"),
            "defaulttextalignment"
        );
        assert_eq!(
            canonical_skinparam_key("Sequence.Participant_Padding"),
            "participantpadding"
        );
    }

    #[test]
    fn replacement_identity_retains_latest_family_spelling() {
        let input = concat!(
            "@startuml\n",
            "skinparam participantPadding 7\n",
            "skinparam Sequence.Participant_Padding 15\n",
            "Alice -> Bob : Fresh message\n",
            "@enduml\n",
        );
        let diagram = parse(input).unwrap();
        let matching: Vec<_> = diagram
            .meta()
            .skinparams
            .iter()
            .filter(|param| canonical_skinparam_key(&param.key) == "participantpadding")
            .collect();

        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].key, "sequenceparticipantpadding");
        assert_eq!(matching[0].value, "15");
    }

    #[test]
    fn synthetic_theme_family_keys_keep_separate_scoped_identities() {
        let input = concat!(
            "@startuml\n",
            "skinparam stateArrowColor #1565C0\n",
            "skinparam ArrowColor #6A1B9A\n",
            "skinparam __theme fresh-synthetic-theme\n",
            "skinparam __theme_body_start fresh-synthetic-theme\n",
            "skinparam classArrowColor #AD1457\n",
            "skinparam stateArrowColor #2E7D32\n",
            "skinparam usecaseArrowColor #EF6C00\n",
            "skinparam __theme_body_end fresh-synthetic-theme\n",
            "[*] --> FreshRelay\n",
            "FreshRelay --> [*]\n",
            "@enduml\n",
        );
        let diagram = parse(input).unwrap();
        let params = &diagram.meta().skinparams;

        assert_eq!(
            params
                .iter()
                .filter(|param| canonical_skinparam_key(&param.key) == "arrowcolor")
                .count(),
            4
        );
        assert_eq!(
            params
                .iter()
                .find(|param| param.key == "arrowcolor")
                .unwrap()
                .value,
            "#6A1B9A"
        );
        assert!(
            params
                .iter()
                .any(|param| { param.key == "classArrowColor" && param.value == "#AD1457" })
        );
        assert!(
            params
                .iter()
                .any(|param| { param.key == "usecaseArrowColor" && param.value == "#EF6C00" })
        );
    }

    #[test]
    fn real_theme_body_keeps_directive_order_against_user_skinparams() {
        let after_theme = parse(concat!(
            "@startuml\n",
            "!theme metal\n",
            "skinparam State.Arrow_Color #C62828\n",
            "[*] --> FreshRelay\n",
            "FreshRelay --> [*]\n",
            "@enduml\n",
        ))
        .unwrap();
        let after_params = &after_theme.meta().skinparams;
        let theme_position = after_params
            .iter()
            .position(|param| param.key.eq_ignore_ascii_case("__theme"))
            .unwrap();
        let user_position = after_params
            .iter()
            .position(|param| {
                canonical_skinparam_key(&param.key) == "arrowcolor" && param.value == "#C62828"
            })
            .unwrap();
        assert!(theme_position < user_position);

        let before_theme = parse(concat!(
            "@startuml\n",
            "skinparam State.Arrow_Color #C62828\n",
            "!theme metal\n",
            "[*] --> FreshRelay\n",
            "FreshRelay --> [*]\n",
            "@enduml\n",
        ))
        .unwrap();
        let before_params = &before_theme.meta().skinparams;
        let user_position = before_params
            .iter()
            .position(|param| {
                canonical_skinparam_key(&param.key) == "arrowcolor" && param.value == "#C62828"
            })
            .unwrap();
        let theme_position = before_params
            .iter()
            .position(|param| param.key.eq_ignore_ascii_case("__theme"))
            .unwrap();
        assert!(user_position < theme_position);
    }

    #[test]
    fn repeated_theme_bodies_keep_intervening_user_order() {
        let diagram = parse(concat!(
            "@startuml\n",
            "!theme metal\n",
            "skinparam StateBorderColor #C62828\n",
            "!theme minty\n",
            "[*] --> FreshRelay\n",
            "FreshRelay --> [*]\n",
            "@enduml\n",
        ))
        .unwrap();
        let params = &diagram.meta().skinparams;
        let theme_positions = params
            .iter()
            .enumerate()
            .filter(|(_, param)| param.key.eq_ignore_ascii_case("__theme"))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let user_position = params
            .iter()
            .position(|param| {
                param.key == "statebordercolor" && param.value.eq_ignore_ascii_case("#C62828")
            })
            .unwrap();

        assert_eq!(theme_positions.len(), 2);
        assert!(theme_positions[0] < user_position);
        assert!(user_position < theme_positions[1]);
    }

    #[test]
    fn style_program_preserves_sparse_css_and_raw_skinparam_compatibility() {
        let diagram = parse(concat!(
            "@startuml\n",
            "<style>\n",
            "  root { LineColor #123456; Padding 17 }\n",
            "</style>\n",
            "skinparam State.Arrow_Color #654321\n",
            "[*] --> FreshStyledState\n",
            "@enduml\n",
        ))
        .unwrap();

        let declarations = &diagram.meta().style_program.declarations;
        assert_eq!(declarations.len(), 3);
        assert_eq!(declarations[0].selector, ["root"]);
        assert_eq!(declarations[0].property, "linecolor");
        assert_eq!(declarations[0].source_line, 1);
        assert_eq!(declarations[1].property, "padding");
        assert_eq!(declarations[1].source_line, 1);
        assert_eq!(declarations[2].selector, ["arrow"]);
        assert_eq!(declarations[2].value, "#654321");
        assert_eq!(declarations[2].source_line, 4);
        assert!(
            declarations
                .windows(2)
                .all(|pair| pair[0].epoch < pair[1].epoch)
        );

        assert!(
            diagram
                .meta()
                .skinparams
                .iter()
                .any(|param| param.key == "arrowcolor" && param.value == "#654321")
        );
    }

    #[test]
    fn real_theme_css_and_skinparams_execute_at_the_theme_epoch() {
        use crate::diagram::style::StyleOrigin;

        let diagram = parse(concat!(
            "@startuml\n",
            "skinparam ArrowColor #101010\n",
            "!theme metal\n",
            "skinparam ArrowColor #303030\n",
            "class FreshThemeEpochA\n",
            "class FreshThemeEpochB\n",
            "FreshThemeEpochA --> FreshThemeEpochB\n",
            "@enduml\n",
        ))
        .unwrap();
        let declarations = &diagram.meta().style_program.declarations;

        let before = declarations
            .iter()
            .find(|declaration| declaration.value == "#101010")
            .unwrap()
            .epoch;
        let after = declarations
            .iter()
            .find(|declaration| declaration.value == "#303030")
            .unwrap()
            .epoch;
        let theme_epochs = declarations
            .iter()
            .filter_map(|declaration| match &declaration.origin {
                StyleOrigin::ThemeStyle { theme } | StyleOrigin::ThemeSkinParam { theme }
                    if theme == "metal" =>
                {
                    Some(declaration.epoch)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(!theme_epochs.is_empty());
        assert!(declarations.iter().any(|declaration| {
            matches!(
                &declaration.origin,
                StyleOrigin::ThemeStyle { theme } if theme == "metal"
            )
        }));
        assert!(declarations.iter().any(|declaration| {
            matches!(
                &declaration.origin,
                StyleOrigin::ThemeSkinParam { theme } if theme == "metal"
            )
        }));
        assert!(theme_epochs.iter().all(|epoch| before < *epoch));
        assert!(theme_epochs.iter().all(|epoch| *epoch < after));
    }

    #[test]
    fn style_source_lines_recover_entity_and_link_snapshot_boundaries() {
        use crate::diagram::style::StyleOrigin;

        let diagram = parse(concat!(
            "@startuml\n",
            "class FreshBeforeStyle\n",
            "<style>\n",
            "  classDiagram { element { LineColor #1A2B3C } }\n",
            "</style>\n",
            "class FreshBeforeTheme\n",
            "FreshBeforeStyle --> FreshBeforeTheme\n",
            "!theme metal\n",
            "class FreshAfterTheme\n",
            "FreshBeforeTheme --> FreshAfterTheme\n",
            "@enduml\n",
        ))
        .unwrap();
        let Diagram::Class(class) = &diagram else {
            panic!("expected class diagram");
        };
        let declaration_line = |origin: &StyleOrigin| {
            class
                .meta
                .style_program
                .declarations
                .iter()
                .find(|declaration| &declaration.origin == origin)
                .unwrap()
                .source_line
        };
        let entity_line = |id: &str| {
            class
                .entities
                .iter()
                .find(|entity| entity.id == id)
                .unwrap()
                .source_line
        };

        let user_style_line = declaration_line(&StyleOrigin::UserStyle);
        let theme_style_line = class
            .meta
            .style_program
            .declarations
            .iter()
            .find_map(|declaration| {
                matches!(
                    &declaration.origin,
                    StyleOrigin::ThemeStyle { theme } if theme == "metal"
                )
                .then_some(declaration.source_line)
            })
            .unwrap();
        assert_eq!(user_style_line, 2);
        assert_eq!(theme_style_line, 7);
        assert!(entity_line("FreshBeforeStyle") < user_style_line);
        assert!(user_style_line < entity_line("FreshBeforeTheme"));
        assert!(class.relationships[0].source_line < theme_style_line);
        assert!(theme_style_line < entity_line("FreshAfterTheme"));
        assert!(theme_style_line < class.relationships[1].source_line);
        assert!(class.meta.style_program.declarations.iter().all(
            |declaration| !matches!(&declaration.origin, StyleOrigin::ThemeStyle { theme } if theme == "metal")
                || declaration.source_line == theme_style_line
        ));
    }

    #[test]
    fn repeated_themes_keep_intervening_user_style_epoch() {
        use crate::diagram::style::StyleOrigin;

        let diagram = parse(concat!(
            "@startuml\n",
            "!theme metal\n",
            "skinparam StateBorderColor #C62828\n",
            "!theme minty\n",
            "[*] --> FreshThemeRelay\n",
            "FreshThemeRelay --> [*]\n",
            "@enduml\n",
        ))
        .unwrap();
        let declarations = &diagram.meta().style_program.declarations;
        let metal_max = declarations
            .iter()
            .filter_map(|declaration| match &declaration.origin {
                StyleOrigin::ThemeStyle { theme } | StyleOrigin::ThemeSkinParam { theme }
                    if theme == "metal" =>
                {
                    Some(declaration.epoch)
                }
                _ => None,
            })
            .max()
            .unwrap();
        let user_epoch = declarations
            .iter()
            .find(|declaration| {
                matches!(&declaration.origin, StyleOrigin::UserSkinParam)
                    && declaration.selector == ["state"]
                    && declaration.property == "linecolor"
                    && declaration.value == "#C62828"
            })
            .unwrap()
            .epoch;
        let minty_min = declarations
            .iter()
            .filter_map(|declaration| match &declaration.origin {
                StyleOrigin::ThemeStyle { theme } | StyleOrigin::ThemeSkinParam { theme }
                    if theme == "minty" =>
                {
                    Some(declaration.epoch)
                }
                _ => None,
            })
            .min()
            .unwrap();

        assert!(metal_max < user_epoch);
        assert!(user_epoch < minty_min);
    }

    #[test]
    fn style_program_retains_repeated_writes_that_raw_compatibility_collapses() {
        let diagram = parse(concat!(
            "@startuml\n",
            "skinparam ArrowColor #111111\n",
            "skinparam State.Arrow_Color #222222\n",
            "skinparam UseCase_Arrow.Color #333333\n",
            "[*] --> FreshOrderedRelay\n",
            "@enduml\n",
        ))
        .unwrap();

        let writes = diagram
            .meta()
            .style_program
            .declarations
            .iter()
            .filter(|declaration| {
                declaration.selector == ["arrow"] && declaration.property == "linecolor"
            })
            .map(|declaration| declaration.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(writes, ["#111111", "#222222", "#333333"]);
        assert_eq!(
            diagram
                .meta()
                .skinparams
                .iter()
                .filter(|param| canonical_skinparam_key(&param.key) == "arrowcolor")
                .count(),
            1
        );
    }

    #[test]
    fn leading_bare_note_colon_routes_to_class() {
        let input = "@startuml\nnote : x = 1\nAlice -> Bob : Message 1\n@enduml";
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn braced_package_usymbol_commands_keep_class_factory_precedence() {
        for (keyword, expected_kind) in PackageKind::COMMAND_SYMBOLS {
            let code = format!("{keyword}_container");
            let declaration = format!("{keyword} \"{keyword} container\" as {code} {{");
            let input = format!(
                "@startuml\n\
                 {declaration}\n\
                   class NestedLeaf\n\
                 }}\n\
                 @enduml"
            );
            let Diagram::Class(diagram) = parse(&input).unwrap() else {
                panic!("{keyword} did not select ClassDiagramFactory");
            };
            let package = diagram
                .packages
                .iter()
                .find(|package| package.name == code)
                .unwrap_or_else(|| panic!("{keyword} did not create its package quark"));
            assert_eq!(package.kind, expected_kind, "{keyword}");
            assert!(
                diagram
                    .entities
                    .iter()
                    .any(|entity| entity.id == format!("{code}.NestedLeaf")),
                "{keyword}"
            );
        }
    }

    #[test]
    fn malformed_new_package_usymbol_opener_reaches_class_error() {
        let error = parse(
            "@startuml\n\
             storage \"Broken\" <<Open> {\n\
               class EscapedLeaf\n\
             }\n\
             @enduml",
        )
        .unwrap_err();
        assert_eq!(error.line, 1);
        assert_eq!(
            error.message,
            "invalid package or symbol-container declaration"
        );
    }

    #[test]
    fn quoted_deployment_containers_beat_nested_component_leaves() {
        let input = r#"@startuml
cloud "Kubernetes Cluster" {
  node "Master Node" {
    component "API Server"
    component "Scheduler"
  }
}
@enduml"#;
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Deployment(_)));
    }

    #[test]
    fn component_package_containers_beat_quoted_deployment_containers() {
        let input = r#"@startuml
node "IoT Device" {
  component Sensor
}
package "Backend" {
  component "Data Ingestion" as DI
}
Sensor --> DI
@enduml"#;
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Component(_)));
    }

    #[test]
    fn quoted_shared_container_with_interface_routes_to_class() {
        let input = r#"@startuml
node "Service Boundary" {
  interface Gateway
}
Gateway --> Audit
@enduml"#;
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Class(_)));
    }

    #[test]
    fn quoted_node_with_component_leaf_stays_deployment() {
        let input = r#"@startuml
node "Runtime Boundary" as Runtime {
  component API
}
@enduml"#;
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Deployment(_)));
    }

    #[test]
    fn interface_with_component_leaf_without_class_container_stays_component() {
        let input = r#"@startuml
interface Gateway
component Application
Application --> Gateway
@enduml"#;
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Component(_)));
    }

    #[test]
    fn top_level_component_leaf_beats_quoted_database_container() {
        let input = r#"@startuml
database "Main Store" {
  component "Read Replica" as RR
}
component Application
Application --> RR
@enduml"#;
        let diagram = parse(input).unwrap();
        assert!(matches!(diagram, Diagram::Component(_)));
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
    fn component_spelling_on_a_class_lollipop_is_not_declaration_evidence() {
        for owner in ["Component", "component", "FreshComponentOwner"] {
            let input = format!("@startuml\nclass {owner}\n{owner} --() FreshPort\n@enduml");
            assert!(
                matches!(parse(&input).unwrap(), Diagram::Class(_)),
                "{input}"
            );
        }

        let real_component =
            "@startuml\ncomponent FreshAdapter\nFreshAdapter --() FreshPort\n@enduml";
        assert!(matches!(
            parse(real_component).unwrap(),
            Diagram::Component(_)
        ));
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
    fn ordinary_containers_with_only_objects_or_maps_stay_object_diagrams() {
        let package = r#"@startuml
package "Renamed Domain" {
  object Account {
    id = 7
  }
  map "Flags" as flags {
    active => true
  }
}
Account --> flags
@enduml"#;
        assert!(matches!(parse(package).unwrap(), Diagram::Object(_)));

        let namespace = r#"@startuml
namespace net.example.fresh {
  map "Settings" as settings {
    mode => strict
  }
}
@enduml"#;
        assert!(matches!(parse(namespace).unwrap(), Diagram::Object(_)));
    }

    #[test]
    fn object_leaves_do_not_steal_class_only_or_symbol_container_models() {
        let mixed_class = r#"@startuml
package "Renamed Domain" {
  object Account
  class Policy
}
@enduml"#;
        assert!(matches!(parse(mixed_class).unwrap(), Diagram::Class(_)));

        let symbol_container = r#"@startuml
rectangle "Renamed Domain" {
  object Account
}
@enduml"#;
        assert!(matches!(
            parse(symbol_container).unwrap(),
            Diagram::Class(_)
        ));

        let allow_mixing = r#"@startuml
allowmixing
package "Renamed Domain" {
  object Account
}
@enduml"#;
        assert!(matches!(parse(allow_mixing).unwrap(), Diagram::Class(_)));
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
    fn ambiguous_and_markerless_inputs_follow_plantuml_factory_selection() {
        // `ClassDiagramFactory.initCommandsList` owns `CommandAllowMixing`, so
        // mixed states remain elements in a class diagram.
        let mixed = r#"@startuml
allowmixing
class Scheduler {
  +dispatch()
}
state Waiting
state Executing
Waiting --> Executing : submit
Scheduler .. Executing : controls
@enduml"#;
        assert!(matches!(parse(mixed).unwrap(), Diagram::Class(_)));

        let class_arrow =
            "@startuml\nclass Origin\nclass Destination\nOrigin -> Destination : sends\n@enduml";
        assert!(matches!(parse(class_arrow).unwrap(), Diagram::Class(_)));

        let state_arrows = r#"@startuml
state Queued
state Working
state Complete
Queued -> Working : claim
Working -> Complete : finish
Complete -> Queued : retry
@enduml"#;
        assert!(matches!(parse(state_arrows).unwrap(), Diagram::State(_)));

        // Markerless input still goes through the UML factory ordering. The
        // class factory accepts floating notes and their association, while
        // the state factory accepts pseudostate transitions.
        let notes = r#"note "Primary diagnostic" as First
note "Secondary diagnostic" as Second
First .. Second"#;
        assert!(matches!(parse(notes).unwrap(), Diagram::Class(_)));

        let states = r#"[*] --> Ready
Ready --> Suspended : hold
Suspended --> Ready : resume
Ready --> [*] : close"#;
        assert!(matches!(parse(states).unwrap(), Diagram::State(_)));
    }

    #[test]
    fn description_only_allowmixing_selects_class_factory() {
        let input = r#"@startuml
allowmixing
actor "Renamed Operator" as Operator
usecase "Renamed Approval" as Approval
component "Renamed Policy" as Policy
queue "Renamed Retry" as Retry
Operator --> Approval
Approval --> Policy
Policy --> Retry
@enduml"#;

        let Diagram::Class(diagram) = parse(input).unwrap() else {
            panic!("allowmixing must select the class parser");
        };
        assert_eq!(diagram.entities.len(), 4);
        assert!(diagram.entities.iter().all(|entity| entity.id != "all"));
        assert!(diagram.entities.iter().all(|entity| entity.id != "wmixing"));
    }

    #[test]
    fn allow_mixing_uses_one_exact_case_insensitive_command_grammar() {
        for command in ["allowmixing", "ALLOWMIXING", "allow_mixing", "AlLoW_MiXiNg"] {
            assert!(is_allow_mixing_command(command));
            let input = format!("@startuml\n{command}\nactor \"Operator\" as Operator\n@enduml");
            assert!(matches!(parse(&input).unwrap(), Diagram::Class(_)));
        }

        for invalid in [
            "allowmixing trailing",
            "allow_mixing trailing",
            "allow__mixing",
            "allow mixing",
        ] {
            assert!(!is_allow_mixing_command(invalid));
            let input = format!("@startuml\n{invalid}\n@enduml");
            assert!(parse(&input).is_err());
        }

        let relationship = parse("@startuml\nallowmixing --> X\n@enduml").unwrap();
        assert!(matches!(relationship, Diagram::Sequence(_)));
    }

    #[test]
    fn explicit_and_named_starts_preserve_their_semantic_family() {
        assert!(matches!(
            parse("@startjson\n{\"renamed\": [1, 2, 3]}\n@endjson").unwrap(),
            Diagram::Json(_)
        ));
        assert!(matches!(
            parse("@startyaml\nrenamed:\n  - one\n  - two\n@endyaml").unwrap(),
            Diagram::Json(_)
        ));

        let named_class =
            "@startuml renamed_model\nclass Parent\nclass Child extends Parent\n@enduml";
        assert!(matches!(parse(named_class).unwrap(), Diagram::Class(_)));

        let explicit_state =
            "@startuml\n[*] --> Open\nOpen --> Closed : finish\nClosed --> [*]\n@enduml";
        assert!(matches!(parse(explicit_state).unwrap(), Diagram::State(_)));
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
    fn styled_external_arrows_remain_sequence_dispatch_evidence() {
        let input = concat!(
            "@startuml\n",
            "participant Amber\n",
            "participant Indigo\n",
            "[-[#C2185B,dashed]> Amber : found\n",
            "Amber -[dotted,#2E7D32]>] : lost\n",
            "Amber -[hidden]> Indigo : concealed\n",
            "Indigo -[bold]> Amber : visible\n",
            "@enduml\n",
        );
        assert!(matches!(parse(input).unwrap(), Diagram::Sequence(_)));
    }

    #[test]
    fn quoted_participant_style_text_is_not_component_evidence() {
        let input = concat!(
            "@startuml\n",
            "\"Source [hidden]: 9401\" -[#AD1457,dashed]> \"Target: 9403\" : visible\n",
            "\"Target: 9403\" -> \"Source [hidden]: 9401\" : return\n",
            "@enduml\n",
        );
        let Diagram::Sequence(sequence) = parse(input).unwrap() else {
            panic!("expected sequence diagram");
        };
        assert_eq!(sequence.events.len(), 2);
        assert_eq!(sequence.participants[0].label, "Source [hidden]: 9401");
    }

    #[test]
    fn sequence_commands_own_brackets_in_participants_and_page_meta() {
        let input = concat!(
            "@startuml\n",
            "header Header ox footer [hidden] without a shaft\n",
            "footer Footer xo header [dotted,#455A64] without a shaft\n",
            "participant \"Ingress [hidden]: 15101\" as Ingress15101\n",
            "participant \"Worker [dotted,#455A64]: 15103\" as Worker15103\n",
            "Ingress15101 -> Worker15103 : plain arrow\n",
            "Worker15103 -[#7B1FA2,dashed]> Ingress15101 : styled arrow\n",
            "@enduml\n",
        );

        let Diagram::Sequence(sequence) = parse(input).unwrap() else {
            panic!("expected sequence diagram");
        };
        assert_eq!(sequence.events.len(), 2);
        assert_eq!(sequence.participants[0].label, "Ingress [hidden]: 15101");
        assert_eq!(
            sequence.meta.header.as_deref(),
            Some("Header ox footer [hidden] without a shaft")
        );
    }

    #[test]
    fn meta_keywords_without_a_hyphen_shaft_are_not_sequence_messages() {
        assert!(!sequence::looks_like_message("footer"));
        assert!(!sequence::looks_like_message("header"));
        assert!(sequence::looks_like_message("A o-> B"));
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
