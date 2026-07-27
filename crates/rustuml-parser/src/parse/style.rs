// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Parser-owned construction of the ordered style program.

use std::collections::HashMap;

use crate::diagram::style::{StyleDeclaration, StyleOrigin, StyleProgram, StyleScheme};
use crate::preprocess;

// Java provenance: `StyleLoader#DELTA_PRIORITY_FOR_STEREOTYPE`.
const STEREOTYPE_PRIORITY: i64 = 1_000;

#[derive(Clone)]
enum InputOrigin {
    User,
    Theme(String),
}

impl InputOrigin {
    fn style(&self) -> StyleOrigin {
        match self {
            Self::User => StyleOrigin::UserStyle,
            Self::Theme(theme) => StyleOrigin::ThemeStyle {
                theme: theme.clone(),
            },
        }
    }

    fn skinparam(&self) -> StyleOrigin {
        match self {
            Self::User => StyleOrigin::UserSkinParam,
            Self::Theme(theme) => StyleOrigin::ThemeSkinParam {
                theme: theme.clone(),
            },
        }
    }
}

enum StyleInput {
    Block {
        source: String,
        origin: StyleOrigin,
        source_line: usize,
    },
    SkinParam {
        key: String,
        value: String,
        origin: StyleOrigin,
        source_line: usize,
    },
}

impl StyleInput {
    fn set_source_line(&mut self, source_line: usize) {
        match self {
            Self::Block {
                source_line: input_line,
                ..
            }
            | Self::SkinParam {
                source_line: input_line,
                ..
            } => *input_line = source_line,
        }
    }
}

struct ThemeBody {
    name: String,
    start: usize,
    end: usize,
    inputs: Vec<StyleInput>,
    used: bool,
}

struct ProgramBuilder {
    declarations: Vec<StyleDeclaration>,
    next_epoch: u64,
}

struct DeclarationContext {
    scheme: StyleScheme,
    origin: StyleOrigin,
    source_line: usize,
    epoch: u64,
}

impl ProgramBuilder {
    fn new() -> Self {
        Self {
            declarations: Vec::new(),
            next_epoch: 0,
        }
    }

    fn next_epoch(&mut self) -> u64 {
        let epoch = self.next_epoch;
        self.next_epoch = self.next_epoch.saturating_add(1);
        epoch
    }

    fn push(
        &mut self,
        signature: &StyleSignature,
        property: &str,
        value: String,
        context: DeclarationContext,
    ) {
        let stereotype_priority = if signature.stereotypes.is_empty() {
            0
        } else {
            STEREOTYPE_PRIORITY
        };
        self.declarations.push(StyleDeclaration {
            selector: signature.selector.clone(),
            stereotypes: signature.stereotypes.clone(),
            depth: signature.depth,
            star: signature.star,
            property: property.to_string(),
            value,
            scheme: context.scheme,
            source_line: context.source_line,
            epoch: context.epoch,
            priority: i64::try_from(context.epoch)
                .unwrap_or(i64::MAX)
                .saturating_add(stereotype_priority),
            origin: context.origin,
        });
    }

    fn finish(self) -> StyleProgram {
        StyleProgram {
            declarations: self.declarations,
        }
    }
}

#[derive(Clone, Default)]
struct StyleSignature {
    selector: Vec<String>,
    stereotypes: Vec<String>,
    depth: Option<u32>,
    star: bool,
}

/// Build the renderer-neutral style program and mask `<style>` blocks from
/// diagram-family parsers. Theme bodies remain physically relocated for source
/// line compatibility, but are replayed here at their `!theme` marker, matching
/// `TContext#executeTheme`.
pub(super) fn extract_style_program(lines: &mut [String]) -> StyleProgram {
    let mut bodies = find_theme_bodies(lines);
    for body in &mut bodies {
        body.inputs = collect_inputs(
            lines,
            body.start + 1,
            body.end,
            &InputOrigin::Theme(body.name.clone()),
        );
    }

    let mut logical = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if let Some(body) = bodies.iter().find(|body| body.start == index) {
            index = body.end + 1;
            continue;
        }

        let text = source_text(&lines[index]).trim();
        if let Some((key, value)) = parse_skinparam(text)
            && key.eq_ignore_ascii_case("__theme")
        {
            let source_line = source_line(lines, index);
            if let Some(body) = bodies
                .iter_mut()
                .find(|body| !body.used && body.name.eq_ignore_ascii_case(&value))
            {
                body.used = true;
                for input in &mut body.inputs {
                    input.set_source_line(source_line);
                }
                logical.append(&mut body.inputs);
            }
            index += 1;
            continue;
        }

        let (mut inputs, next) = collect_one_input(lines, index, lines.len(), &InputOrigin::User);
        logical.append(&mut inputs);
        index = next;
    }

    let mut builder = ProgramBuilder::new();
    for input in logical {
        match input {
            StyleInput::Block {
                source,
                origin,
                source_line,
            } => {
                parse_style_block(&source, origin, source_line, &mut builder);
            }
            StyleInput::SkinParam {
                key,
                value,
                origin,
                source_line,
            } => {
                convert_skinparam(&key, &value, origin, source_line, &mut builder);
            }
        }
    }
    builder.finish()
}

fn find_theme_bodies(lines: &[String]) -> Vec<ThemeBody> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let Some((key, name)) = parse_skinparam(source_text(&lines[index]).trim()) else {
            index += 1;
            continue;
        };
        if !key.eq_ignore_ascii_case("__theme_body_start") {
            index += 1;
            continue;
        }
        let end = (index + 1..lines.len())
            .find(|candidate| {
                parse_skinparam(source_text(&lines[*candidate]).trim()).is_some_and(
                    |(candidate_key, candidate_name)| {
                        candidate_key.eq_ignore_ascii_case("__theme_body_end")
                            && candidate_name.eq_ignore_ascii_case(&name)
                    },
                )
            })
            .unwrap_or(lines.len());
        result.push(ThemeBody {
            name,
            start: index,
            end,
            inputs: Vec::new(),
            used: false,
        });
        index = end.saturating_add(1);
    }
    result
}

fn collect_inputs(
    lines: &mut [String],
    start: usize,
    end: usize,
    origin: &InputOrigin,
) -> Vec<StyleInput> {
    let mut result = Vec::new();
    let mut index = start;
    while index < end {
        let (mut inputs, next) = collect_one_input(lines, index, end, origin);
        result.append(&mut inputs);
        index = next;
    }
    result
}

fn collect_one_input(
    lines: &mut [String],
    index: usize,
    end: usize,
    origin: &InputOrigin,
) -> (Vec<StyleInput>, usize) {
    let text = source_text(&lines[index]).trim();
    if starts_style_block(text)
        && let Some(close) =
            (index..end).find(|candidate| ends_style_block(source_text(&lines[*candidate])))
    {
        let source_line = source_line(lines, index);
        let mut source = String::new();
        for line in &mut lines[index..=close] {
            source.push_str(source_text(line));
            source.push('\n');
            line.clear();
        }
        return (
            vec![StyleInput::Block {
                source,
                origin: origin.style(),
                source_line,
            }],
            close + 1,
        );
    }

    if let Some((prefix, value)) = parse_skinparam(text) {
        if value == "{" || prefix.ends_with('{') {
            let prefix = prefix.trim_end_matches('{').trim().to_string();
            let close = (index + 1..end)
                .find(|candidate| source_text(&lines[*candidate]).trim() == "}")
                .unwrap_or(end);
            let mut result = Vec::new();
            for candidate in index + 1..close {
                let entry = source_text(&lines[candidate]).trim();
                if let Some((key, value)) = split_key_value(entry) {
                    result.push(StyleInput::SkinParam {
                        key: format!("{prefix}{key}"),
                        value,
                        origin: origin.skinparam(),
                        source_line: source_line(lines, candidate),
                    });
                }
            }
            return (result, close.saturating_add(1));
        }
        if !prefix.starts_with("__") {
            return (
                vec![StyleInput::SkinParam {
                    key: prefix,
                    value,
                    origin: origin.skinparam(),
                    source_line: source_line(lines, index),
                }],
                index + 1,
            );
        }
    }

    (Vec::new(), index + 1)
}

fn source_text(line: &str) -> &str {
    preprocess::split_source_line_marker(line).map_or(line, |(_, text)| text)
}

fn source_line(lines: &[String], index: usize) -> usize {
    preprocess::split_source_line_marker(&lines[index]).map_or(index + 1, |(line, _)| line)
}

fn starts_style_block(line: &str) -> bool {
    line.get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<style>"))
}

fn ends_style_block(line: &str) -> bool {
    line.to_ascii_lowercase().contains("</style>")
}

fn parse_skinparam(line: &str) -> Option<(String, String)> {
    let rest = line
        .get(..10)
        .filter(|prefix| prefix.eq_ignore_ascii_case("skinparam "))
        .map(|_| &line[10..])?
        .trim();
    if let Some(prefix) = rest.strip_suffix('{')
        && !prefix.contains(char::is_whitespace)
    {
        return Some((prefix.to_string(), "{".to_string()));
    }
    split_key_value(rest)
}

fn split_key_value(line: &str) -> Option<(String, String)> {
    let split = line.find(char::is_whitespace)?;
    let key = line[..split].trim();
    let value = line[split..].trim();
    if key.is_empty() || value.is_empty() {
        return None;
    }
    Some((key.to_string(), value.to_string()))
}

#[derive(Clone)]
enum CssToken {
    Word(String),
    Open,
    Close,
    Comma,
    Star,
    Colon,
    Semicolon,
    Newline,
    Media,
}

fn tokenize_style(source: &str) -> Vec<CssToken> {
    // Java provenance: `StyleParser#parse(CharInspector)`, including its three
    // comment forms, punctuation tokens, quoted strings, and media marker.
    let chars = source.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            ' ' | '\t' => index += 1,
            '\n' | '\r' => {
                tokens.push(CssToken::Newline);
                index += 1;
            }
            '/' if chars.get(index + 1) == Some(&'/') => {
                index += 2;
                while index < chars.len() && !matches!(chars[index], '\n' | '\r') {
                    index += 1;
                }
            }
            '/' if chars.get(index + 1) == Some(&'*') => {
                index += 2;
                while index + 1 < chars.len() && !(chars[index] == '*' && chars[index + 1] == '/') {
                    index += 1;
                }
                index = (index + 2).min(chars.len());
            }
            '/' if chars.get(index + 1) == Some(&'\'') => {
                index += 2;
                while index + 1 < chars.len() && !(chars[index] == '\'' && chars[index + 1] == '/')
                {
                    index += 1;
                }
                index = (index + 2).min(chars.len());
            }
            '{' => {
                tokens.push(CssToken::Open);
                index += 1;
            }
            '}' => {
                tokens.push(CssToken::Close);
                index += 1;
            }
            ',' => {
                tokens.push(CssToken::Comma);
                index += 1;
            }
            '*' => {
                tokens.push(CssToken::Star);
                index += 1;
            }
            ':' => {
                tokens.push(CssToken::Colon);
                index += 1;
            }
            ';' => {
                tokens.push(CssToken::Semicolon);
                index += 1;
            }
            '@' => {
                while index < chars.len() && chars[index] != '{' {
                    index += 1;
                }
                if index < chars.len() {
                    index += 1;
                }
                tokens.push(CssToken::Media);
            }
            '"' => {
                index += 1;
                let mut word = String::new();
                while index < chars.len() && chars[index] != '"' {
                    word.push(chars[index]);
                    index += 1;
                }
                if index < chars.len() {
                    index += 1;
                }
                tokens.push(CssToken::Word(word));
            }
            '.' => {
                // Java `StyleParser#readString` keeps spaces inside a
                // dot-prefixed stereotype selector.
                let mut word = String::new();
                while index < chars.len()
                    && !matches!(
                        chars[index],
                        '\n' | '\r' | '{' | '}' | ',' | ':' | ';' | '\t'
                    )
                {
                    word.push(chars[index]);
                    index += 1;
                }
                tokens.push(CssToken::Word(word.trim().to_string()));
            }
            _ => {
                let mut word = String::new();
                while index < chars.len()
                    && !matches!(
                        chars[index],
                        ' ' | '\t' | '\n' | '\r' | '{' | '}' | ',' | '*' | ':' | ';'
                    )
                {
                    word.push(chars[index]);
                    index += 1;
                }
                if !word.is_empty() {
                    tokens.push(CssToken::Word(word));
                }
            }
        }
    }
    tokens
}

fn parse_style_block(
    source: &str,
    origin: StyleOrigin,
    source_line: usize,
    builder: &mut ProgramBuilder,
) {
    // Java provenance: `StyleParser#parse(BlocLines)` and
    // `style.parser.Context#push`/`toStyles`.
    let tokens = tokenize_style(source);
    let mut contexts = vec![vec![StyleSignature::default()]];
    let mut variables = HashMap::<String, String>::new();
    let mut scheme = StyleScheme::Regular;
    let mut index = 0;

    while index < tokens.len() {
        match &tokens[index] {
            CssToken::Newline | CssToken::Semicolon => {
                index += 1;
            }
            CssToken::Media => {
                scheme = StyleScheme::Dark;
                index += 1;
            }
            CssToken::Close => {
                if contexts.len() > 1 {
                    contexts.pop();
                }
                index += 1;
            }
            CssToken::Word(word)
                if word.eq_ignore_ascii_case("<style>")
                    || word.eq_ignore_ascii_case("</style>") =>
            {
                index += 1;
            }
            CssToken::Word(_) | CssToken::Colon => {
                if let Some((selectors, next)) = read_selector(&tokens, index) {
                    let parent = contexts.last().cloned().unwrap_or_default();
                    contexts.push(expand_selectors(&parent, &selectors));
                    index = next;
                    continue;
                }

                let CssToken::Word(property) = &tokens[index] else {
                    index += 1;
                    continue;
                };
                let property = property.clone();
                index += 1;
                while matches!(tokens.get(index), Some(CssToken::Colon)) {
                    index += 1;
                }
                let (mut value, next) = read_css_value(&tokens, index);
                index = next;

                if property.starts_with("--") {
                    variables.insert(property.trim_start_matches('-').to_string(), value);
                    continue;
                }
                let Some(property) = canonical_property(&property) else {
                    continue;
                };
                if let Some(variable) = css_variable_name(&value)
                    && let Some(replacement) = variables.get(variable)
                {
                    value = replacement.clone();
                }
                let epoch = builder.next_epoch();
                for signature in contexts.last().into_iter().flatten() {
                    if signature.selector.is_empty()
                        && signature.stereotypes.is_empty()
                        && signature.depth.is_none()
                    {
                        continue;
                    }
                    builder.push(
                        signature,
                        property,
                        value.clone(),
                        DeclarationContext {
                            scheme,
                            origin: origin.clone(),
                            source_line,
                            epoch,
                        },
                    );
                }
            }
            CssToken::Open | CssToken::Comma | CssToken::Star => {
                index += 1;
            }
        }
    }
}

fn read_selector(tokens: &[CssToken], start: usize) -> Option<(Vec<String>, usize)> {
    let mut selectors = Vec::new();
    let mut index = start;
    loop {
        let mut selector = String::new();
        if matches!(tokens.get(index), Some(CssToken::Colon)) {
            selector.push(':');
            index += 1;
        }
        let Some(CssToken::Word(word)) = tokens.get(index) else {
            return None;
        };
        selector.push_str(word);
        index += 1;
        if matches!(tokens.get(index), Some(CssToken::Star)) {
            selector.push('*');
            index += 1;
        }
        selectors.push(selector);
        skip_newlines(tokens, &mut index);
        match tokens.get(index) {
            Some(CssToken::Comma) => {
                index += 1;
                skip_newlines(tokens, &mut index);
            }
            Some(CssToken::Open) => return Some((selectors, index + 1)),
            _ => return None,
        }
    }
}

fn skip_newlines(tokens: &[CssToken], index: &mut usize) {
    while matches!(tokens.get(*index), Some(CssToken::Newline)) {
        *index += 1;
    }
}

fn expand_selectors(parents: &[StyleSignature], selectors: &[String]) -> Vec<StyleSignature> {
    let mut result = Vec::new();
    for selector in selectors {
        for parent in parents {
            let mut signature = parent.clone();
            let mut selector = selector.trim_start_matches(':').to_string();
            if selector.ends_with('*') {
                signature.star = true;
                selector.pop();
            }
            if let Some(depth) = selector
                .strip_prefix("depth(")
                .and_then(|value| value.strip_suffix(')'))
                .and_then(|value| value.parse::<u32>().ok())
            {
                signature.depth = Some(depth);
            } else if selector.starts_with('.') || !is_selector_name(&selector) {
                let stereotype = clean_stereotype(&selector);
                if !stereotype.is_empty() && !signature.stereotypes.contains(&stereotype) {
                    signature.stereotypes.push(stereotype);
                }
            } else {
                let selector = canonical_selector(&selector);
                if !signature.selector.contains(&selector) {
                    signature.selector.push(selector);
                }
            }
            result.push(signature);
        }
    }
    result
}

fn read_css_value(tokens: &[CssToken], mut index: usize) -> (String, usize) {
    let mut value = String::new();
    while let Some(token) = tokens.get(index) {
        match token {
            CssToken::Newline | CssToken::Semicolon | CssToken::Close => break,
            CssToken::Word(word) => {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(word);
            }
            CssToken::Comma => value.push(','),
            CssToken::Colon => {
                value.push(':');
                if let Some(CssToken::Word(word)) = tokens.get(index + 1) {
                    value.push_str(word);
                    index += 1;
                }
            }
            CssToken::Star => value.push('*'),
            CssToken::Open => value.push('{'),
            CssToken::Media => {}
        }
        index += 1;
    }
    (value, index)
}

fn css_variable_name(value: &str) -> Option<&str> {
    value
        .strip_prefix("var(--")
        .and_then(|value| value.strip_suffix(')'))
}

fn canonical_property(property: &str) -> Option<&'static str> {
    // Java provenance: `PName#getFromName` accepts enum names
    // case-insensitively and drops unknown properties.
    const PROPERTIES: &[&str] = &[
        "shadowing",
        "fontname",
        "fontcolor",
        "fontsize",
        "fontstyle",
        "fontweight",
        "backgroundcolor",
        "roundcorner",
        "linethickness",
        "diagonalcorner",
        "hyperlinkcolor",
        "hyperlinkunderlinestyle",
        "hyperlinkunderlinethickness",
        "headcolor",
        "linecolor",
        "linestyle",
        "padding",
        "margin",
        "maximumwidth",
        "minimumwidth",
        "exportedname",
        "image",
        "horizontalalignment",
        "showstereotype",
        "imageposition",
        "markershape",
        "markersize",
        "markercolor",
        "barwidth",
    ];
    PROPERTIES
        .iter()
        .copied()
        .find(|candidate| candidate.eq_ignore_ascii_case(property))
}

fn canonical_selector(selector: &str) -> String {
    selector
        .chars()
        .filter(|character| *character != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn clean_stereotype(stereotype: &str) -> String {
    stereotype
        .trim_start_matches('.')
        .chars()
        .filter(|character| !matches!(character, '_' | '.'))
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_selector_name(selector: &str) -> bool {
    // Java provenance: `SName#retrieve`; unknown selector names become
    // stereotype identities in `style.parser.Context#push`.
    const SELECTORS: &[&str] = &[
        "action",
        "activationbox",
        "activity",
        "activitybar",
        "activitydiagram",
        "actor",
        "agent",
        "analog",
        "annotation",
        "archimate",
        "area",
        "arrow",
        "artifact",
        "axis",
        "bar",
        "binary",
        "boundary",
        "box",
        "boxless",
        "business",
        "caption",
        "card",
        "cardinality",
        "chartdiagram",
        "chenattribute",
        "cheneerdiagram",
        "chenentity",
        "chenrelationship",
        "circle",
        "class",
        "classdiagram",
        "clickable",
        "clock",
        "closed",
        "cloud",
        "collection",
        "collections",
        "component",
        "componentdiagram",
        "composite",
        "concise",
        "constraintarrow",
        "control",
        "database",
        "day",
        "delay",
        "destroy",
        "diamond",
        "document",
        "ebnf",
        "element",
        "end",
        "entity",
        "file",
        "filesdiagram",
        "folder",
        "footer",
        "frame",
        "ganttdiagram",
        "generic",
        "gitdiagram",
        "goto",
        "grid",
        "group",
        "groupheader",
        "haxis",
        "header",
        "hexagon",
        "highlight",
        "hnote",
        "iemandatory",
        "interface",
        "json",
        "jsondiagram",
        "label",
        "leafnode",
        "legend",
        "lifeline",
        "line",
        "mainframe",
        "map",
        "milestone",
        "mindmapdiagram",
        "month",
        "network",
        "newpage",
        "node",
        "note",
        "nwdiagdiagram",
        "object",
        "objectdiagram",
        "package",
        "packetdiagdiagram",
        "participant",
        "partition",
        "person",
        "port",
        "private",
        "process",
        "protected",
        "public",
        "qualified",
        "queue",
        "rectangle",
        "reference",
        "referenceheader",
        "regex",
        "requirement",
        "rnote",
        "robust",
        "root",
        "rootnode",
        "saltdiagram",
        "scatter",
        "separator",
        "sequencediagram",
        "server",
        "spot",
        "spotabstractclass",
        "spotannotation",
        "spotclass",
        "spotdataclass",
        "spotentity",
        "spotenum",
        "spotexception",
        "spotinterface",
        "spotmetaclass",
        "spotprotocol",
        "spotrecord",
        "spotstereotype",
        "stack",
        "start",
        "state",
        "statebody",
        "statediagram",
        "stereotype",
        "stop",
        "storage",
        "swimlane",
        "task",
        "timegrid",
        "timeline",
        "timingdiagram",
        "title",
        "undone",
        "unstarted",
        "usecase",
        "vaxis",
        "verticalseparator",
        "visibilityicon",
        "wbsdiagram",
        "yamldiagram",
        "year",
    ];
    let canonical = canonical_selector(selector);
    SELECTORS.contains(&canonical.as_str())
}

#[derive(Clone)]
struct LegacyMapping {
    property: &'static str,
    selector: Vec<&'static str>,
}

fn convert_skinparam(
    raw_key: &str,
    raw_value: &str,
    origin: StyleOrigin,
    source_line: usize,
    builder: &mut ProgramBuilder,
) {
    for (key, stereotypes) in canonical_skinparam_keys(raw_key) {
        let mut value = normalize_legacy_value(&key, raw_value);
        let mappings = legacy_mappings(&key);
        if mappings.is_empty() {
            let special = if key == "shadowing" {
                Some(LegacyMapping {
                    property: "shadowing",
                    selector: vec!["root"],
                })
            } else if key == "noteshadowing" {
                Some(LegacyMapping {
                    property: "shadowing",
                    selector: vec!["root", "note"],
                })
            } else {
                None
            };
            if let Some(mapping) = special {
                push_legacy_mapping(
                    mapping,
                    &stereotypes,
                    normalize_shadowing(&value),
                    origin.clone(),
                    source_line,
                    builder,
                );
            }
            continue;
        }

        if value.contains(';') {
            let values = value.split(';').map(str::to_string).collect::<Vec<_>>();
            value = values.first().cloned().unwrap_or_default();
            for extra in values.iter().skip(1) {
                push_complex_legacy_value(
                    extra,
                    &mappings,
                    &stereotypes,
                    origin.clone(),
                    source_line,
                    builder,
                );
            }
        } else if value.starts_with("text:") {
            push_complex_legacy_value(
                &value,
                &mappings,
                &stereotypes,
                origin.clone(),
                source_line,
                builder,
            );
            continue;
        }

        if value != " " {
            for mapping in mappings {
                push_legacy_mapping(
                    mapping,
                    &stereotypes,
                    value.clone(),
                    origin.clone(),
                    source_line,
                    builder,
                );
            }
        }
    }
}

fn canonical_skinparam_keys(raw_key: &str) -> Vec<(String, Vec<String>)> {
    // Java provenance: `SkinParam#cleanForKeySlow`.
    let mut cleaned = raw_key.trim().to_ascii_lowercase().replace(['_', '.'], "");
    cleaned = cleaned
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
        cleaned = cleaned.replace(&format!("{prefix}arrow"), "arrow");
    }
    if cleaned.ends_with("align") {
        cleaned.truncate(cleaned.len() - "align".len());
        cleaned.push_str("alignment");
    }

    let mut stereotypes = Vec::new();
    let mut base = cleaned.clone();
    while let Some(start) = base.find("<<")
        && let Some(relative_end) = base[start + 2..].find(">>")
    {
        let end = start + 2 + relative_end;
        stereotypes.push(clean_stereotype(&base[start + 2..end]));
        base.replace_range(start..end + 2, "");
    }
    if stereotypes.is_empty() {
        return vec![(cleaned, Vec::new())];
    }
    stereotypes
        .into_iter()
        .map(|stereotype| {
            (
                base.clone(),
                stereotype.split('&').map(clean_stereotype).collect(),
            )
        })
        .collect()
}

fn normalize_legacy_value(key: &str, raw_value: &str) -> String {
    // Java provenance: `FromSkinparamToStyle#convertNow`.
    let value = raw_value.trim();
    if key.ends_with("shadowing") {
        if value.eq_ignore_ascii_case("false") {
            return "0".to_string();
        }
        if value.eq_ignore_ascii_case("true") {
            return "3".to_string();
        }
    } else if key == "hyperlinkunderline" {
        if value.eq_ignore_ascii_case("false") {
            return "0".to_string();
        }
        if value.eq_ignore_ascii_case("true") {
            return "1".to_string();
        }
    }
    if value.eq_ignore_ascii_case("right:right") {
        "right".to_string()
    } else if value.eq_ignore_ascii_case("dotted") {
        "1;3".to_string()
    } else if value.eq_ignore_ascii_case("dashed") {
        "7;7".to_string()
    } else {
        value.to_string()
    }
}

fn normalize_shadowing(value: &str) -> String {
    if value.eq_ignore_ascii_case("false") || value.eq_ignore_ascii_case("no") {
        "0".to_string()
    } else if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes") {
        "3".to_string()
    } else {
        value.to_string()
    }
}

fn push_complex_legacy_value(
    value: &str,
    mappings: &[LegacyMapping],
    stereotypes: &[String],
    origin: StyleOrigin,
    source_line: usize,
    builder: &mut ProgramBuilder,
) {
    let (property, value) = if let Some(value) = value.strip_prefix("text:") {
        ("fontcolor", value)
    } else if value.starts_with("line.dotted") {
        ("linestyle", "1;3")
    } else if value.starts_with("line.dashed") {
        ("linestyle", "7;7")
    } else if value.to_ascii_lowercase().contains("bold") {
        ("linethickness", "2")
    } else {
        return;
    };
    for mapping in mappings {
        push_legacy_mapping(
            LegacyMapping {
                property,
                selector: mapping.selector.clone(),
            },
            stereotypes,
            value.to_string(),
            origin.clone(),
            source_line,
            builder,
        );
    }
}

fn push_legacy_mapping(
    mapping: LegacyMapping,
    stereotypes: &[String],
    value: String,
    origin: StyleOrigin,
    source_line: usize,
    builder: &mut ProgramBuilder,
) {
    let signature = StyleSignature {
        selector: mapping
            .selector
            .iter()
            .map(|token| token.to_string())
            .collect(),
        stereotypes: stereotypes.to_vec(),
        depth: None,
        star: false,
    };
    let epoch = builder.next_epoch();
    builder.push(
        &signature,
        mapping.property,
        value,
        DeclarationContext {
            scheme: StyleScheme::Regular,
            origin,
            source_line,
            epoch,
        },
    );
}

/// Semantic equivalent of PlantUML's `FromSkinparamToStyle` knowledge table at
/// revision 71806a23780b04a5ccde2f8ceb5121edad5eb711. The table remains sparse:
/// unknown and behavioral skinparams deliberately produce no style mutation.
fn legacy_mappings(key: &str) -> Vec<LegacyMapping> {
    const DIRECT: &[(&str, &str, &[&str])] = &[
        (
            "participantclickablebackgroundcolor",
            "backgroundcolor",
            &["participant", "clickable"],
        ),
        (
            "participantclickablebordercolor",
            "linecolor",
            &["participant", "clickable"],
        ),
        ("defaultfontsize", "fontsize", &["element"]),
        ("sequencestereotypefontsize", "fontsize", &["stereotype"]),
        ("sequencestereotypefontstyle", "fontstyle", &["stereotype"]),
        ("sequencestereotypefontcolor", "fontcolor", &["stereotype"]),
        ("sequencestereotypefontname", "fontname", &["stereotype"]),
        ("sequencereferencebordercolor", "linecolor", &["reference"]),
        (
            "sequencereferencebordercolor",
            "linecolor",
            &["referenceheader"],
        ),
        (
            "sequencereferencebackgroundcolor",
            "backgroundcolor",
            &["reference"],
        ),
        (
            "sequencereferenceheaderbackgroundcolor",
            "backgroundcolor",
            &["referenceheader"],
        ),
        ("sequencegroupborderthickness", "linethickness", &["group"]),
        ("sequencegroupbordercolor", "linecolor", &["group"]),
        ("sequencegroupbordercolor", "linecolor", &["groupheader"]),
        (
            "sequencegroupbackgroundcolor",
            "backgroundcolor",
            &["groupheader"],
        ),
        ("sequenceboxbordercolor", "linecolor", &["box"]),
        ("sequenceboxbackgroundcolor", "backgroundcolor", &["box"]),
        ("sequenceboxfontcolor", "fontcolor", &["box"]),
        ("sequencelifelinebordercolor", "linecolor", &["lifeline"]),
        (
            "sequencelifelinebackgroundcolor",
            "backgroundcolor",
            &["activationbox"],
        ),
        ("sequencedelaybordercolor", "linecolor", &["delay"]),
        (
            "sequencedividerbackgroundcolor",
            "backgroundcolor",
            &["separator"],
        ),
        ("sequencedividerbordercolor", "linecolor", &["separator"]),
        (
            "sequencedividerborderthickness",
            "linethickness",
            &["separator"],
        ),
        (
            "sequencemessagealignment",
            "horizontalalignment",
            &["arrow"],
        ),
        ("noteborderthickness", "linethickness", &["note"]),
        ("notebordercolor", "linecolor", &["note"]),
        ("notebackgroundcolor", "backgroundcolor", &["note"]),
        ("packagebackgroundcolor", "backgroundcolor", &["group"]),
        ("packagebordercolor", "linecolor", &["group"]),
        ("partitionbordercolor", "linecolor", &["composite"]),
        (
            "partitionbackgroundcolor",
            "backgroundcolor",
            &["composite"],
        ),
        ("hyperlinkcolor", "hyperlinkcolor", &["root"]),
        (
            "activitystartcolor",
            "backgroundcolor",
            &["circle", "start"],
        ),
        ("activityendcolor", "linecolor", &["circle", "end"]),
        ("activitystopcolor", "linecolor", &["circle", "stop"]),
        ("activitybarcolor", "backgroundcolor", &["activitybar"]),
        ("activitybordercolor", "linecolor", &["activity"]),
        ("activityborderthickness", "linethickness", &["activity"]),
        ("activitybackgroundcolor", "backgroundcolor", &["activity"]),
        (
            "activitydiamondbackgroundcolor",
            "backgroundcolor",
            &["diamond"],
        ),
        ("activitydiamondbordercolor", "linecolor", &["diamond"]),
        ("arrowthickness", "linethickness", &["arrow"]),
        ("arrowcolor", "linecolor", &["arrow"]),
        ("arrowstyle", "linestyle", &["arrow"]),
        ("arrowheadcolor", "headcolor", &["arrow"]),
        ("defaulttextalignment", "horizontalalignment", &["root"]),
        ("defaultfontname", "fontname", &["root"]),
        ("defaultfontcolor", "fontcolor", &["root"]),
        (
            "swimlanetitlebackgroundcolor",
            "backgroundcolor",
            &["swimlane"],
        ),
        ("swimlanebordercolor", "linecolor", &["swimlane"]),
        ("swimlaneborderthickness", "linethickness", &["swimlane"]),
        ("roundcorner", "roundcorner", &["root"]),
        ("titleborderthickness", "linethickness", &["title"]),
        ("titlebordercolor", "linecolor", &["title"]),
        ("titlebackgroundcolor", "backgroundcolor", &["title"]),
        ("titleborderroundcorner", "roundcorner", &["title"]),
        ("legendborderthickness", "linethickness", &["legend"]),
        ("legendbordercolor", "linecolor", &["legend"]),
        ("legendbackgroundcolor", "backgroundcolor", &["legend"]),
        ("legendborderroundcorner", "roundcorner", &["legend"]),
        ("notetextalignment", "horizontalalignment", &["note"]),
        ("backgroundcolor", "backgroundcolor", &["document"]),
        (
            "classbackgroundcolor",
            "backgroundcolor",
            &["element", "class"],
        ),
        ("classbordercolor", "linecolor", &["element", "class"]),
        ("classfontsize", "fontsize", &["element", "class", "header"]),
        (
            "classfontstyle",
            "fontstyle",
            &["element", "class", "header"],
        ),
        (
            "classfontcolor",
            "fontcolor",
            &["element", "class", "header"],
        ),
        ("classfontname", "fontname", &["element", "class", "header"]),
        ("classattributefontsize", "fontsize", &["element", "class"]),
        (
            "classattributefontstyle",
            "fontstyle",
            &["element", "class"],
        ),
        (
            "classattributefontcolor",
            "fontcolor",
            &["element", "class"],
        ),
        ("classattributefontname", "fontname", &["element", "class"]),
        (
            "classborderthickness",
            "linethickness",
            &["element", "class"],
        ),
        (
            "classheaderbackgroundcolor",
            "backgroundcolor",
            &["element", "class", "header"],
        ),
        ("objectbackgroundcolor", "backgroundcolor", &["object"]),
        ("objectbordercolor", "linecolor", &["object"]),
        ("objectborderthickness", "linethickness", &["object"]),
        ("statebackgroundcolor", "backgroundcolor", &["state"]),
        ("statebordercolor", "linecolor", &["state"]),
        ("stateborderthickness", "linethickness", &["state"]),
        (
            "iconprivatecolor",
            "linecolor",
            &["visibilityicon", "private"],
        ),
        (
            "iconprivatebackgroundcolor",
            "backgroundcolor",
            &["visibilityicon", "private"],
        ),
        (
            "iconpackagecolor",
            "linecolor",
            &["visibilityicon", "package"],
        ),
        (
            "iconpackagebackgroundcolor",
            "backgroundcolor",
            &["visibilityicon", "package"],
        ),
        (
            "iconprotectedcolor",
            "linecolor",
            &["visibilityicon", "protected"],
        ),
        (
            "iconprotectedbackgroundcolor",
            "backgroundcolor",
            &["visibilityicon", "protected"],
        ),
        (
            "iconpubliccolor",
            "linecolor",
            &["visibilityicon", "public"],
        ),
        (
            "iconpublicbackgroundcolor",
            "backgroundcolor",
            &["visibilityicon", "public"],
        ),
        ("minclasswidth", "minimumwidth", &[]),
        ("wrapwidth", "maximumwidth", &["element"]),
        (
            "hyperlinkunderline",
            "hyperlinkunderlinethickness",
            &["element"],
        ),
        (
            "stereotypealignment",
            "horizontalalignment",
            &["stereotype"],
        ),
        (
            "stereotypeabackgroundcolor",
            "backgroundcolor",
            &["spotabstractclass"],
        ),
        (
            "stereotypeabordercolor",
            "linecolor",
            &["spotabstractclass"],
        ),
        (
            "stereotypecbackgroundcolor",
            "backgroundcolor",
            &["spotclass"],
        ),
        ("stereotypecbordercolor", "linecolor", &["spotclass"]),
        (
            "stereotypeebackgroundcolor",
            "backgroundcolor",
            &["spotenum"],
        ),
        ("stereotypeebordercolor", "linecolor", &["spotenum"]),
        (
            "stereotypeibackgroundcolor",
            "backgroundcolor",
            &["spotinterface"],
        ),
        ("stereotypeibordercolor", "linecolor", &["spotinterface"]),
        (
            "stereotypenbackgroundcolor",
            "backgroundcolor",
            &["spotannotation"],
        ),
        ("stereotypenbordercolor", "linecolor", &["spotannotation"]),
    ];
    const FONT_PREFIXES: &[(&str, &[&str])] = &[
        ("header", &["document", "header"]),
        ("footer", &["document", "footer"]),
        ("caption", &["document", "caption"]),
        ("sequencereference", &["reference"]),
        ("sequencereference", &["referenceheader"]),
        ("sequencegroup", &["group"]),
        ("sequencegroupheader", &["groupheader"]),
        ("sequencedelay", &["delay"]),
        ("sequencedivider", &["separator"]),
        ("note", &["note"]),
        ("partition", &["composite"]),
        ("activity", &["activity"]),
        ("activitydiamond", &["diamond"]),
        ("arrow", &["arrow"]),
        ("swimlanetitle", &["swimlane"]),
        ("title", &["document", "title"]),
        ("legend", &["legend"]),
        ("object", &["object"]),
        ("objectattribute", &["object"]),
        ("state", &["state"]),
        ("stateattribute", &["state"]),
    ];
    const MAGIC: &[&str] = &[
        "participant",
        "boundary",
        "control",
        "collections",
        "actor",
        "database",
        "entity",
        "package",
        "agent",
        "artifact",
        "card",
        "interface",
        "cloud",
        "component",
        "file",
        "folder",
        "frame",
        "hexagon",
        "node",
        "person",
        "queue",
        "rectangle",
        "stack",
        "storage",
        "usecase",
        "map",
        "archimate",
        "hnote",
        "rnote",
    ];

    let mut result = DIRECT
        .iter()
        .filter(|(candidate, _, _)| *candidate == key)
        .map(|(_, property, selector)| LegacyMapping {
            property,
            selector: selector.to_vec(),
        })
        .collect::<Vec<_>>();

    for (prefix, selector) in FONT_PREFIXES {
        for (suffix, property) in [
            ("fontsize", "fontsize"),
            ("fontstyle", "fontstyle"),
            ("fontcolor", "fontcolor"),
            ("fontname", "fontname"),
        ] {
            if key == format!("{prefix}{suffix}") {
                result.push(LegacyMapping {
                    property,
                    selector: selector.to_vec(),
                });
            }
        }
    }

    for name in MAGIC {
        let mappings = [
            ("backgroundcolor", "backgroundcolor"),
            ("bordercolor", "linecolor"),
            ("borderthickness", "linethickness"),
            ("roundcorner", "roundcorner"),
            ("diagonalcorner", "diagonalcorner"),
            ("borderstyle", "linestyle"),
            ("fontsize", "fontsize"),
            ("fontstyle", "fontstyle"),
            ("fontcolor", "fontcolor"),
            ("fontname", "fontname"),
            ("shadowing", "shadowing"),
        ];
        for (suffix, property) in mappings {
            if key == format!("{name}{suffix}") {
                result.push(LegacyMapping {
                    property,
                    selector: vec![*name],
                });
            }
        }
        for (suffix, property) in [
            ("stereotypefontsize", "fontsize"),
            ("stereotypefontstyle", "fontstyle"),
            ("stereotypefontcolor", "fontcolor"),
            ("stereotypefontname", "fontname"),
        ] {
            if key == format!("{name}{suffix}") {
                result.push(LegacyMapping {
                    property,
                    selector: vec!["stereotype", *name],
                });
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_program(source: &str) -> StyleProgram {
        let mut lines = source.lines().map(str::to_string).collect::<Vec<_>>();
        extract_style_program(&mut lines)
    }

    #[test]
    fn css_parser_expands_nested_comma_selectors_and_stereotype_priority() {
        let program = parse_program(
            "@startuml\n\
             <style>\n\
               --accent: #13579B;\n\
               classDiagram, stateDiagram {\n\
                 element {\n\
                   .critical {\n\
                     LineColor: var(--accent)\n\
                   }\n\
                 }\n\
               }\n\
             </style>\n\
             class FreshStyleProbe\n\
             @enduml",
        );

        let declarations = &program.declarations;
        assert_eq!(declarations.len(), 2);
        assert_eq!(declarations[0].selector, ["classdiagram", "element"]);
        assert_eq!(declarations[1].selector, ["statediagram", "element"]);
        assert_eq!(declarations[0].stereotypes, ["critical"]);
        assert_eq!(declarations[0].property, "linecolor");
        assert_eq!(declarations[0].value, "#13579B");
        assert_eq!(declarations[0].epoch, declarations[1].epoch);
        assert_eq!(
            declarations[0].priority,
            i64::try_from(declarations[0].epoch).unwrap() + STEREOTYPE_PRIORITY
        );
    }

    #[test]
    fn css_parser_preserves_depth_star_and_dark_scheme() {
        let program = parse_program(
            "@startmindmap\n\
             <style>\n\
               mindmapDiagram* {\n\
                 :depth(4) { FontSize 19 }\n\
               }\n\
               @media (prefers-color-scheme:dark) {\n\
                 root { FontColor #F0F0F0 }\n\
               }\n\
             </style>\n\
             * Fresh root\n\
             @endmindmap",
        );

        assert_eq!(program.declarations.len(), 2);
        assert_eq!(program.declarations[0].selector, ["mindmapdiagram"]);
        assert_eq!(program.declarations[0].depth, Some(4));
        assert!(program.declarations[0].star);
        assert_eq!(program.declarations[1].scheme, StyleScheme::Dark);
    }

    #[test]
    fn legacy_conversion_is_canonical_sparse_and_stereotype_aware() {
        let program = parse_program(
            "@startuml\n\
             skinparam State.Arrow_Color #123456\n\
             skinparam class<<Fresh_Service>> {\n\
               BackgroundColor #ABCDEF\n\
               BorderThickness 4\n\
             }\n\
             class FreshService <<Fresh.Service>>\n\
             @enduml",
        );

        let arrow = program
            .declarations
            .iter()
            .find(|declaration| declaration.selector == ["arrow"])
            .unwrap();
        assert_eq!(arrow.property, "linecolor");
        assert_eq!(arrow.value, "#123456");

        let qualified = program
            .declarations
            .iter()
            .filter(|declaration| declaration.stereotypes == ["freshservice"])
            .collect::<Vec<_>>();
        assert_eq!(qualified.len(), 2);
        assert_eq!(qualified[0].selector, ["element", "class"]);
        assert_eq!(qualified[0].property, "backgroundcolor");
        assert_eq!(qualified[1].property, "linethickness");
        assert!(
            qualified
                .iter()
                .all(|declaration| declaration.priority >= STEREOTYPE_PRIORITY)
        );
    }

    #[test]
    fn theme_body_is_replayed_between_user_declarations() {
        let program = parse_program(
            "@startuml\n\
             skinparam ArrowColor #111111\n\
             skinparam __theme metal\n\
             skinparam ArrowColor #333333\n\
             class A\n\
             skinparam __theme_body_start metal\n\
             <style>\n\
               root { FontColor #222222 }\n\
             </style>\n\
             skinparam ArrowColor #222222\n\
             skinparam __theme_body_end metal\n\
             @enduml",
        );

        let values = program
            .declarations
            .iter()
            .filter(|declaration| {
                declaration.selector == ["arrow"] && declaration.property == "linecolor"
            })
            .map(|declaration| declaration.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(values, ["#111111", "#222222", "#333333"]);
        assert!(matches!(
            &program.declarations[1].origin,
            StyleOrigin::ThemeStyle { .. }
        ));
    }
}
