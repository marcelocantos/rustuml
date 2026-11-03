// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! TIM preprocessor — handles variables, defines, conditionals,
//! includes, and comments before diagram-specific parsing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::diagram::SpriteData;

mod date_format;
mod debug;
mod stdlib;
mod themes;

use debug::{RenderClock, TzSpec};

/// A fatal TIM evaluation error with its source location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreprocessError {
    pub line: usize,
    pub message: String,
}

/// Evaluated source and parser projection, or a fatal error with partial output.
pub struct PreprocessOutput {
    /// Fatal TIM error. Callers must check this before consuming partial output.
    /// parse_with_base must map it to ParseError (owned by the parse worker).
    pub error: Option<PreprocessError>,
    pub lines: Vec<String>,
    pub sprites: HashMap<String, SpriteData>,
    /// PlantUML's post-TIM `UmlSource.getPlainString("\n")`.
    ///
    /// SVG resources hash this expanded source, rather than the raw input,
    /// when deriving deterministic filter and gradient identifiers.
    pub uml_source: String,
}

const SOURCE_LINE_PREFIX: &str = "\x1ERSL:";
const SOURCE_LINE_SEPARATOR: char = '\x1F';

pub(crate) fn source_line_marker(source_line: usize, line: &str) -> String {
    format!("{SOURCE_LINE_PREFIX}{source_line}{SOURCE_LINE_SEPARATOR}{line}")
}

pub(crate) fn split_source_line_marker(line: &str) -> Option<(usize, &str)> {
    let rest = line.strip_prefix(SOURCE_LINE_PREFIX)?;
    let (source_line, line) = rest.split_once(SOURCE_LINE_SEPARATOR)?;
    let source_line = source_line.parse().ok()?;
    Some((source_line, line))
}

fn strip_source_line_markers(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| {
            split_source_line_marker(&line).map_or(line.clone(), |(_, text)| text.to_string())
        })
        .collect()
}

/// Preprocess PlantUML source, expanding TIM directives.
pub fn preprocess(input: &str) -> Vec<String> {
    preprocess_full(input, None).lines
}

/// Preprocess with a base directory for resolving `!include` paths.
pub fn preprocess_with_base(input: &str, base_dir: &Path) -> Vec<String> {
    preprocess_full(input, Some(base_dir.to_path_buf())).lines
}

/// Preprocess PlantUML source and return both expanded lines and sprite
/// definitions collected from `sprite $name { ... }` blocks.
pub fn preprocess_full(input: &str, base_dir: Option<PathBuf>) -> PreprocessOutput {
    let mut output = preprocess_full_inner(input, base_dir, false, false);
    output.lines = strip_source_line_markers(output.lines);
    output
}

pub(crate) fn preprocess_full_for_parse(
    input: &str,
    base_dir: Option<PathBuf>,
) -> PreprocessOutput {
    preprocess_full_inner(input, base_dir, true, false)
}

pub(crate) fn preprocess_full_for_sequence_parse(
    input: &str,
    base_dir: Option<PathBuf>,
) -> PreprocessOutput {
    preprocess_full_inner(input, base_dir, true, true)
}

fn preprocess_full_inner(
    input: &str,
    base_dir: Option<PathBuf>,
    mark_function_body_source_lines: bool,
    preserve_teoz_pragma: bool,
) -> PreprocessOutput {
    // Java TContext.resultList is produced once; UmlSource and diagram parsing
    // consume that same evaluated stream. Never rerun TIM to compute identity.
    let mut ctx = PreprocessContext::new(base_dir, mark_function_body_source_lines);
    ctx.preserve_teoz_pragma = preserve_teoz_pragma;
    let events = ctx.process(input);
    let mut identity = Vec::new();
    let mut lines = Vec::new();
    let mut theme_tail = Vec::new();
    project_events(&events, &mut identity, &mut lines, &mut theme_tail);
    let uml_source = uml_source_string(&identity, preserve_teoz_pragma);
    if !theme_tail.is_empty() {
        theme_tail.push(format!(
            "skinparam __svgIdSeed {}",
            svg_id_seed_prefix(&uml_source)
        ));
        lines.extend(theme_tail);
    }
    // Sprite extraction belongs to parser lowering, after TIM evaluation.
    // Java leaves every evaluated sprite line in TContext.resultList/UmlSource.
    let sprites = extract_sprites(&mut lines);
    PreprocessOutput {
        lines,
        sprites,
        uml_source,
        error: ctx.error,
    }
}

/// A TIM execution result, before parser-specific lowering. Plain text has one
/// owner: both projections see exactly this value, including its origin marker.
/// Java provenance: TContext.addPlain/executeTheme/executeOption and UmlSource.
#[derive(Clone, Debug)]
enum EvaluatedEvent {
    Plain(String),
    Boundary(String),
    ParserOnly(String),
    Theme {
        name: String,
        source_line: usize,
        body: Vec<EvaluatedEvent>,
    },
}

fn project_events(
    events: &[EvaluatedEvent],
    identity: &mut Vec<String>,
    lines: &mut Vec<String>,
    theme_tail: &mut Vec<String>,
) {
    for event in events {
        match event {
            EvaluatedEvent::Plain(line) => {
                identity.push(
                    split_source_line_marker(line)
                        .map_or(line.as_str(), |(_, text)| text)
                        .to_string(),
                );
                lines.push(line.clone());
            }
            EvaluatedEvent::Boundary(line) => identity.push(line.clone()),
            EvaluatedEvent::ParserOnly(line) => lines.push(line.clone()),
            EvaluatedEvent::Theme {
                name,
                source_line,
                body,
            } => {
                let mut expanded = Vec::new();
                let mut nested_tail = Vec::new();
                project_events(body, identity, &mut expanded, &mut nested_tail);
                lines.push(source_line_marker(
                    *source_line,
                    &format!("skinparam __theme {name}"),
                ));
                // Retain the parser's theme envelope contract. Only this
                // projection flattens grouped properties or relocates them.
                theme_tail.push(format!("skinparam __theme_body_start {name}"));
                theme_tail.extend(themes::flatten_theme_output(&expanded));
                theme_tail.extend(nested_tail);
                theme_tail.push(format!("skinparam __theme_body_end {name}"));
            }
        }
    }
}

fn extract_sprites(lines: &mut [String]) -> HashMap<String, SpriteData> {
    let mut sprites: HashMap<String, SpriteData> = HashMap::new();
    let mut current: Option<String> = None;
    for line in lines {
        let text = split_source_line_marker(line)
            .map_or(line.as_str(), |(_, text)| text)
            .trim();
        if let Some(name) = &current {
            if text == "}" {
                current = None;
            } else if !text.is_empty()
                && text.chars().all(|c| c.is_ascii_hexdigit())
                && let Some(sprite) = sprites.get_mut(name)
            {
                sprite.rows.push(text.to_string());
                sprite.height = sprite.rows.len() as u32;
            }
            line.clear();
        } else if text.starts_with("sprite ") || text.starts_with("sprite\t") {
            if text.ends_with('{') {
                let rest = text["sprite ".len()..].trim_start();
                let rest = rest.strip_prefix('$').unwrap_or(rest);
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                let (width, height) = parse_sprite_dimensions(rest);
                sprites.insert(
                    name.clone(),
                    SpriteData {
                        width,
                        height,
                        rows: Vec::new(),
                    },
                );
                current = Some(name);
            }
            line.clear();
        }
    }
    sprites
}

fn svg_id_seed_prefix(source: &str) -> String {
    // Java provenance: `StringUtils.seed` is a wrapping signed-long 31x hash
    // over UTF-16 code units; `SvgGraphics.getSeed` formats its absolute value
    // in radix 36.
    let mut hash: u64 = 1_125_899_906_842_597;
    for unit in source.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(unit));
    }
    let signed = hash as i64;
    let mut value = if signed == i64::MIN {
        i64::MAX as u64 + 1
    } else {
        signed.unsigned_abs()
    };
    if value == 0 {
        return "0".to_string();
    }
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut result = Vec::new();
    while value > 0 {
        result.push(digits[(value % 36) as usize]);
        value /= 36;
    }
    result.reverse();
    String::from_utf8(result).expect("base36 digits are ASCII")
}

fn uml_source_string(lines: &[String], collapse_ending_backslashes: bool) -> String {
    let mut source = String::new();
    if collapse_ending_backslashes {
        let mut pending = String::new();
        for line in lines {
            if line.ends_with('\\') && !line.ends_with("\\\\") {
                pending.push_str(&line[..line.len() - 1]);
            } else {
                pending.push_str(line);
                source.push_str(&pending);
                source.push('\n');
                pending.clear();
            }
        }
    } else {
        for line in lines {
            source.push_str(line);
            source.push('\n');
        }
    }
    source
}

/// Preprocess with a base directory, returning full output including sprites.
pub fn preprocess_full_with_base(input: &str, base_dir: &Path) -> PreprocessOutput {
    preprocess_full(input, Some(base_dir.to_path_buf()))
}

// ---------------------------------------------------------------------------
// Value type
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl Value {
    fn to_display(&self) -> String {
        match self {
            Value::Str(s) => s.clone(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                // Avoid trailing zeros but keep at least one decimal if float
                if *f == (*f as i64) as f64 {
                    format!("{}", *f as i64)
                } else {
                    format!("{f}")
                }
            }
            Value::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
        }
    }

    fn to_number(&self) -> f64 {
        match self {
            Value::Int(n) => *n as f64,
            Value::Float(f) => *f,
            Value::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Str(s) => s.parse::<f64>().unwrap_or(0.0),
        }
    }

    fn is_numeric(&self) -> bool {
        match self {
            Value::Int(_) | Value::Float(_) => true,
            Value::Str(s) => s.parse::<f64>().is_ok(),
            Value::Bool(_) => true,
        }
    }

    fn from_str_auto(s: &str) -> Value {
        let s = s.trim();
        if s == "true" {
            return Value::Bool(true);
        }
        if s == "false" {
            return Value::Bool(false);
        }
        if let Ok(n) = s.parse::<i64>() {
            return Value::Int(n);
        }
        if let Ok(f) = s.parse::<f64>() {
            return Value::Float(f);
        }
        Value::Str(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Named color map
// ---------------------------------------------------------------------------

fn named_color(name: &str) -> Option<(u8, u8, u8)> {
    let lower = name.to_lowercase();
    Some(match lower.as_str() {
        "red" => (255, 0, 0),
        "green" => (0, 128, 0),
        "blue" => (0, 0, 255),
        "yellow" => (255, 255, 0),
        "orange" => (255, 165, 0),
        "purple" => (128, 0, 128),
        "pink" => (255, 192, 203),
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "gray" | "grey" => (128, 128, 128),
        "cyan" => (0, 255, 255),
        "magenta" | "fuchsia" => (255, 0, 255),
        "lime" => (0, 255, 0),
        "brown" => (165, 42, 42),
        "navy" => (0, 0, 128),
        "teal" => (0, 128, 128),
        "silver" => (192, 192, 192),
        "gold" => (255, 215, 0),
        "maroon" => (128, 0, 0),
        "olive" => (128, 128, 0),
        "aqua" => (0, 255, 255),
        "indigo" => (75, 0, 130),
        "violet" => (238, 130, 238),
        "coral" => (255, 127, 80),
        "salmon" => (250, 128, 114),
        "tan" => (210, 180, 140),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "plum" => (221, 160, 221),
        "orchid" => (218, 112, 214),
        "tomato" => (255, 99, 71),
        "crimson" => (220, 20, 60),
        "darkblue" => (0, 0, 139),
        "darkgreen" => (0, 100, 0),
        "darkred" => (139, 0, 0),
        "darkgray" | "darkgrey" => (169, 169, 169),
        "lightblue" => (173, 216, 230),
        "lightgreen" => (144, 238, 144),
        "lightgray" | "lightgrey" => (211, 211, 211),
        "lightyellow" => (255, 255, 224),
        _ => return None,
    })
}

fn parse_color(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.trim().trim_matches('"');
    if let Some(hex) = s.strip_prefix('#')
        && hex.len() == 6
    {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some((r, g, b));
    }
    named_color(s)
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < 1e-10 {
        return (0.0, 0.0, l * 100.0);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < 1e-10 {
        let mut h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
        h
    } else if (max - g).abs() < 1e-10 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h * 60.0, s * 100.0, l * 100.0)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let s = s / 100.0;
    let l = l / 100.0;
    let h = ((h % 360.0) + 360.0) % 360.0;

    if s.abs() < 1e-10 {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h = h / 360.0;

    let hue_to_rgb = |t: f64| -> f64 {
        let mut t = t;
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };

    let r = (hue_to_rgb(h + 1.0 / 3.0) * 255.0).round() as u8;
    let g = (hue_to_rgb(h) * 255.0).round() as u8;
    let b = (hue_to_rgb(h - 1.0 / 3.0) * 255.0).round() as u8;
    (r, g, b)
}

fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

// ---------------------------------------------------------------------------
// Core context
// ---------------------------------------------------------------------------

struct PreprocessContext {
    /// `!$var = value` style variables.
    defines: HashMap<String, String>,
    /// `!define TOKEN value` style macros.
    token_defines: HashMap<String, String>,
    /// `!definelong TOKEN(params)` style multi-line macros.
    definelong_macros: HashMap<String, DefineLongDef>,
    cond_stack: Vec<CondState>,
    in_block_comment: bool,
    archimate_enabled: bool,
    in_diagram_block: bool,
    /// True when inside an @startebnf block — disables single-quote comment
    /// stripping since EBNF uses `'` as a terminal delimiter.
    in_ebnf_block: bool,
    seen_start_tag: bool,
    /// Rust's parse_block API supports definitions in a shared preamble and
    /// keeps their one-based file origins (source_with_original_line_offset).
    preamble_body: bool,
    base_dir: Option<PathBuf>,
    include_depth: usize,
    foreach_stack: Vec<ForEachState>,
    while_stack: Vec<WhileState>,
    functions: HashMap<String, FunctionDef>,
    collecting_function: Option<String>,
    collecting_definelong: Option<String>,
    subs: HashMap<String, Vec<BufferedLine>>,
    collecting_sub: Option<String>,
    collecting_sub_lines: Vec<BufferedLine>,
    /// Local variable scopes for function calls (stack of saved scopes).
    local_vars: Vec<LocalMemory>,
    /// Pending return value from a `!return` inside a function body.
    /// Set by process_one_line when `!return` is encountered while active.
    return_signal: Option<Value>,
    /// 1-based line currently being processed within the active input chunk.
    current_source_line: usize,
    /// Wall-clock snapshot captured once at render start. Drives `%date()`
    /// so all calls within a single render see the same instant and zone.
    /// Defaults to system time + local timezone; both overridable via
    /// `RUSTUML_DEBUG` (see [`debug`]) for deterministic golden tests.
    render_clock: RenderClock,
    /// Once loop expansion produces generated content, subsequent original
    /// lines need explicit source-origin markers because output position no
    /// longer equals PlantUML's `data-source-line`.
    mark_source_lines: bool,
    source_line_override: Option<usize>,
    mark_function_body_source_lines: bool,
    preserve_teoz_pragma: bool,
    /// Effects produced while evaluating the current statement expression.
    error: Option<PreprocessError>,
    expression_events: Vec<EvaluatedEvent>,
    text_consumed: bool,
}

const MAX_INCLUDE_DEPTH: usize = 10;
const MAX_WHILE_ITERATIONS: usize = 10000;

// TMemoryLocal separates explicit locals/arguments from automatically local
// names. A global introduced later takes precedence over an automatic local.
#[derive(Default)]
struct LocalMemory {
    overrides: HashMap<String, String>,
    locals: HashMap<String, String>,
}

struct CondState {
    active: bool,
    has_matched: bool,
}

struct ForEachState {
    var_name: String,
    values: Vec<String>,
    body_lines: Vec<BufferedLine>,
}

struct WhileState {
    condition: String,
    body_lines: Vec<BufferedLine>,
}

#[derive(Clone)]
struct BufferedLine {
    text: String,
    raw_source_line: usize,
    source_line: usize,
}

#[derive(Clone)]
struct FunctionDef {
    params: Vec<FuncParam>,
    body: Vec<BufferedLine>,
}

#[derive(Clone)]
struct FuncParam {
    name: String,
    default: Option<Value>,
}

#[derive(Clone)]
struct DefineLongDef {
    params: Vec<String>,
    body: Vec<BufferedLine>,
}

impl PreprocessContext {
    fn new(base_dir: Option<PathBuf>, mark_function_body_source_lines: bool) -> Self {
        Self {
            defines: HashMap::new(),
            token_defines: HashMap::new(),
            definelong_macros: HashMap::new(),
            cond_stack: Vec::new(),
            in_block_comment: false,
            archimate_enabled: false,
            in_diagram_block: false,
            in_ebnf_block: false,
            seen_start_tag: false,
            preamble_body: false,
            base_dir,
            include_depth: 0,
            foreach_stack: Vec::new(),
            while_stack: Vec::new(),
            functions: HashMap::new(),
            collecting_function: None,
            collecting_definelong: None,
            subs: HashMap::new(),
            collecting_sub: None,
            collecting_sub_lines: Vec::new(),
            local_vars: Vec::new(),
            return_signal: None,
            current_source_line: 0,
            render_clock: RenderClock::from_env(),
            mark_source_lines: false,
            source_line_override: None,
            mark_function_body_source_lines,
            preserve_teoz_pragma: false,
            error: None,
            expression_events: Vec::new(),
            text_consumed: false,
        }
    }

    fn is_active(&self) -> bool {
        self.cond_stack.iter().all(|c| c.active)
    }

    /// Push a blank placeholder for a directive line that produced no diagram
    /// content. PlantUML preserves the original line position of surviving
    /// content (it strips only the `@startuml` line), so its `data-source-line`
    /// attribute equals the original 1-based file line minus one. Keeping a
    /// placeholder for every consumed directive line keeps our preprocessed
    /// stream — which the diagram parser indexes 1-based — aligned with that.
    /// Placeholders are only emitted while collecting a single top-level
    /// diagram block; nested includes manage their own numbering.
    fn push_directive_placeholder(&self, output: &mut Vec<EvaluatedEvent>) {
        // PlantUML `TFunctionImpl.executeReturnFunction` and
        // `executeProcedureInternal` feed their body through
        // `TContext.executeLines`; consumed body directives do not enter the
        // resulting diagram stream. Placeholders only represent directives
        // from the user's top-level source, never generated function bodies.
        if self.include_depth == 0 && self.in_diagram_block && self.local_vars.is_empty() {
            output.push(EvaluatedEvent::ParserOnly(String::new()));
        }
    }

    fn current_diagram_source_line(&self) -> usize {
        if self.include_depth == 0
            && self.in_diagram_block
            && self.seen_start_tag
            && !self.preamble_body
        {
            self.current_source_line.saturating_sub(1)
        } else {
            self.current_source_line
        }
    }

    fn current_buffered_line(&self, line: &str) -> BufferedLine {
        BufferedLine {
            text: line.to_string(),
            raw_source_line: self.current_source_line,
            source_line: self.current_diagram_source_line(),
        }
    }

    fn push_content_line(&self, output: &mut Vec<EvaluatedEvent>, line: String) {
        let line = if split_source_line_marker(&line).is_none()
            && (self.mark_source_lines || !self.expression_events.is_empty())
            && !line.is_empty()
        {
            source_line_marker(
                self.source_line_override
                    .unwrap_or_else(|| self.current_diagram_source_line()),
                &line,
            )
        } else {
            line
        };
        output.push(EvaluatedEvent::Plain(line));
    }

    fn process_buffered_line(&mut self, line: &BufferedLine, output: &mut Vec<EvaluatedEvent>) {
        let saved_source_line = self.current_source_line;
        let saved_mark_source_lines = self.mark_source_lines;
        let saved_source_line_override = self.source_line_override;
        self.current_source_line = line.raw_source_line;
        self.mark_source_lines = true;
        self.source_line_override = Some(line.source_line);
        self.process_one_line(&line.text, output);
        self.source_line_override = saved_source_line_override;
        self.mark_source_lines = saved_mark_source_lines;
        self.current_source_line = saved_source_line;
    }

    fn get_var(&self, name: &str) -> Option<&String> {
        // TMemoryLocal.getVariable and forkFromGlobal: sibling calls see the
        // global memory, never their caller's overrides or fresh local names.
        let local = self.local_vars.last();
        local
            .and_then(|scope| scope.overrides.get(name))
            .or_else(|| self.defines.get(name))
            .or_else(|| local.and_then(|scope| scope.locals.get(name)))
    }

    fn set_var(&mut self, name: &str, value: &str) {
        // TMemoryLocal.putVariable: override, existing global, then new local.
        if let Some(scope) = self.local_vars.last_mut() {
            if scope.overrides.contains_key(name) {
                scope.overrides.insert(name.to_string(), value.to_string());
                return;
            }
            if !self.defines.contains_key(name) {
                scope.locals.insert(name.to_string(), value.to_string());
                return;
            }
        }
        self.defines.insert(name.to_string(), value.to_string());
    }

    fn set_local_var(&mut self, name: &str, value: &str) {
        if let Some(scope) = self.local_vars.last_mut() {
            scope.overrides.insert(name.to_string(), value.to_string());
        } else {
            self.set_var(name, value);
        }
    }

    fn remove_var(&mut self, name: &str) {
        // TMemoryLocal.removeVariable uses the same precedence as getVariable.
        if let Some(scope) = self.local_vars.last_mut() {
            if scope.overrides.remove(name).is_some() {
                return;
            }
            if !self.defines.contains_key(name) {
                scope.locals.remove(name);
                return;
            }
        }
        self.defines.remove(name);
    }

    fn process(&mut self, input: &str) -> Vec<EvaluatedEvent> {
        let lines: Vec<&str> = input.lines().collect();
        self.process_lines(&lines)
    }

    fn process_lines(&mut self, lines: &[&str]) -> Vec<EvaluatedEvent> {
        let mut output = Vec::new();

        let saved_source_line = self.current_source_line;
        for (idx, &line) in lines.iter().enumerate() {
            self.current_source_line = idx + 1;
            self.process_one_line(line, &mut output);
        }
        self.current_source_line = saved_source_line;

        output
    }

    fn process_one_line(&mut self, line: &str, output: &mut Vec<EvaluatedEvent>) {
        // TContext.resultList is shared by return-value and procedure execution.
        // Each statement owns a sink; nested statements preserve the caller's
        // already-emitted effects and contribute their own before its content.
        if self.error.is_some() {
            return;
        }
        let outer = std::mem::take(&mut self.expression_events);
        let outer_consumed = std::mem::replace(&mut self.text_consumed, false);
        let mut content = Vec::new();
        self.execute_one_line(line, &mut content);
        if !self.expression_events.is_empty() {
            self.mark_source_lines = true;
        }
        if self.error.is_none() {
            output.append(&mut self.expression_events);
            output.extend(content);
        }
        self.expression_events = outer;
        self.text_consumed = outer_consumed;
    }

    fn execute_one_line(&mut self, line: &str, output: &mut Vec<EvaluatedEvent>) {
        let trimmed = line.trim();

        // Handle block comments. We emit empty placeholder lines for each
        // comment line so downstream parsers preserve original source line
        // numbers (PlantUML's `data-source-line` attribute matches the
        // user's editor view).
        if self.in_block_comment {
            if trimmed.ends_with("'/") {
                self.in_block_comment = false;
            }
            self.push_directive_placeholder(output);
            return;
        }
        // TLineType: a whole-line /'...'/ is a short comment; a /'
        // without a closing '/ starts a long comment. Prefix comments with
        // following content are handled by StringLocated.removeInnerComment.
        if trimmed.starts_with("/'") && (!trimmed.contains("'/") || trimmed.ends_with("'/")) {
            self.in_block_comment = !trimmed.contains("'/");
            self.push_directive_placeholder(output);
            return;
        }

        // Skip single-line comments — but NOT inside EBNF blocks where
        // single quotes delimit terminals. Emit a placeholder so source
        // line numbers stay aligned downstream.
        if !self.in_ebnf_block && trimmed.starts_with('\'') {
            self.push_directive_placeholder(output);
            return;
        }

        // Java StringLocated.removeInnerComment leaves trailing apostrophes
        // alone and removes only its supported block-comment forms.
        let line_no_comment = if self.in_ebnf_block {
            line.to_string()
        } else {
            remove_inner_comment(line)
        };
        let trimmed = line_no_comment.trim();

        // Handle @start/@end tags.
        // PlantUML renders only the first block. Files without any
        // @start tag pass through entirely.
        if self.include_depth == 0 {
            if trimmed.starts_with("@start") {
                if !self.seen_start_tag {
                    self.seen_start_tag = true;
                    self.in_diagram_block = true;
                    self.in_ebnf_block = trimmed.starts_with("@startebnf");
                    output.push(EvaluatedEvent::Boundary(line_no_comment));
                }
                return;
            }
            if trimmed.starts_with("@end") {
                if self.in_diagram_block {
                    output.push(EvaluatedEvent::Boundary(line_no_comment));
                }
                self.in_diagram_block = false;
                self.in_ebnf_block = false;
                return;
            }
            if self.seen_start_tag && !self.in_diagram_block {
                return;
            }
        } else if trimmed.starts_with("@start") || trimmed.starts_with("@end") {
            return;
        }

        // Collecting definelong body.
        if self.collecting_definelong.is_some() {
            if trimmed == "!enddefinelong" {
                self.collecting_definelong = None;
            } else if let Some(name) = self.collecting_definelong.clone() {
                let buffered = self.current_buffered_line(line);
                if let Some(dl) = self.definelong_macros.get_mut(&name) {
                    dl.body.push(buffered);
                }
            }
            self.push_directive_placeholder(output);
            return;
        }

        // CodeIteratorSub buffers then replays the body at its definition,
        // before the conditional/function iterators. Capture origins as well
        // as text; no special theme-only subblock execution path exists.
        if let Some(name) = self.collecting_sub.clone() {
            self.push_directive_placeholder(output);
            if trimmed == "!endsub" {
                self.collecting_sub = None;
                let body = std::mem::take(&mut self.collecting_sub_lines);
                self.subs.insert(name, body.clone());
                for buffered in &body {
                    self.process_buffered_line(buffered, output);
                }
                self.mark_source_lines = true;
            } else {
                self.collecting_sub_lines
                    .push(self.current_buffered_line(&line_no_comment));
            }
            return;
        }

        // Function definition collection.
        if self.try_function_def(&line_no_comment) {
            // Derivation: parse_block_keeps_preamble_procedure_body_lines and
            // parse_block_sequence_with_preamble_meta_procedure_stays_sequence
            // specify one-based file origins for this Rust API extension.
            self.preamble_body |= self.include_depth == 0 && !self.seen_start_tag;
            self.push_directive_placeholder(output);
            return;
        }

        // While buffering.
        if self.try_while(&line_no_comment, output) {
            return;
        }

        // Foreach buffering.
        if self.try_foreach(&line_no_comment, output) {
            return;
        }

        // Directives that are consumed silently.
        if self.try_silent_directive(trimmed) {
            self.push_directive_placeholder(output);
            return;
        }

        // Startsub.
        if let Some(name) = trimmed.strip_prefix("!startsub ") {
            self.collecting_sub = Some(name.trim().to_string());
            self.collecting_sub_lines.clear();
            self.push_directive_placeholder(output);
            return;
        }
        if trimmed == "!endsub" {
            self.push_directive_placeholder(output);
            return;
        }

        // Definelong.
        if self.try_definelong(trimmed) {
            self.preamble_body |= self.include_depth == 0 && !self.seen_start_tag;
            self.push_directive_placeholder(output);
            return;
        }

        // Process TIM directives. Each consumes a single source line and emits
        // no diagram content; push a blank placeholder so surviving content
        // keeps its original line offset (see `push_directive_placeholder`).
        if self.try_define(trimmed) {
            self.push_directive_placeholder(output);
            return;
        }
        if self.try_local_var(trimmed) {
            self.push_directive_placeholder(output);
            return;
        }
        if self.try_conditional(trimmed) {
            self.push_directive_placeholder(output);
            return;
        }

        // !return expr — set the return signal if we're inside an active block.
        if let Some(expr) = trimmed.strip_prefix("!return ") {
            if self.is_active() {
                // Do NOT pre-substitute here: eval_expr_to_value handles
                // substitution internally, and pre-substituting first would
                // cause token_defines with quoted values (e.g. !define VERSION
                // "2.0") to have their quotes stripped during concatenation.
                self.return_signal = Some(self.eval_expr_to_value(expr));
            }
            self.push_directive_placeholder(output);
            return;
        }

        if self.try_undefine(trimmed) {
            self.push_directive_placeholder(output);
            return;
        }
        if let Some(included_lines) = self.try_include(trimmed) {
            if self.is_active() {
                output.extend(included_lines);
            }
            // Include expansion changes the output index independently of the
            // user's source file. Mark every following user line with its
            // original diagram position so downstream `data-source-line`
            // attributes do not inherit the included file's length.
            self.mark_source_lines = true;
            return;
        }
        if let Some(included_lines) = self.try_includesub(trimmed) {
            if self.is_active() {
                output.extend(included_lines);
            }
            return;
        }
        if let Some(theme_lines) = self.try_theme(trimmed) {
            output.extend(theme_lines);
            return;
        }
        if let Some(option_lines) = self.try_option(trimmed) {
            output.extend(option_lines);
            return;
        }

        // TContext.executeDumpMemory logs diagnostics only, never resultList.
        if trimmed == "!dump_memory" || trimmed.starts_with("!dump_memory ") {
            self.push_directive_placeholder(output);
            return;
        }

        // Function/procedure invocation as standalone line.
        if self.try_function_call(&line_no_comment, output) {
            return;
        }

        // Only output lines when all conditions are active.
        if self.is_active() {
            // Expand archimate macros before variable substitution.
            let line_to_process =
                if let Some(expanded) = self.try_expand_archimate(line_no_comment.trim()) {
                    expanded
                } else {
                    // Try definelong macro expansion.
                    let subst = self.substitute_vars(&line_no_comment);
                    let (after_definelong, expanded_multiline) =
                        self.try_expand_definelong(&subst).unwrap_or((subst, false));
                    if expanded_multiline {
                        // Java TFunctionImpl.executeProcedureInternal feeds
                        // multiline LEGACY_DEFINELONG bodies back through
                        // TContext.executeLines with their StringLocated
                        // definition origins. Following caller lines therefore
                        // need explicit origins once expansion changes length.
                        self.mark_source_lines = true;
                    }
                    // Expand any inline $func(args) calls in output content.
                    after_definelong
                };
            // A definelong expansion can produce multiple lines joined with '\n'.
            // Split them so each line is processed individually by the parser.
            // Blank lines are pushed too so that source-line indices match the
            // input file (parsers skip blank lines themselves on the `trim().is_empty()`
            // check, but the index advances). PlantUML's `data-source-line` counts
            // blank lines but excludes the dropped `@startuml`.
            if self.text_consumed {
                return;
            }
            for expanded_line in line_to_process.split('\n') {
                self.push_content_line(output, expanded_line.to_string());
            }
        } else {
            // Line suppressed by an inactive conditional branch. It still
            // occupied a source line, so emit a placeholder to keep downstream
            // `data-source-line` numbering aligned with the original file.
            self.push_directive_placeholder(output);
        }
    }

    fn try_silent_directive(&mut self, line: &str) -> bool {
        // EaterLog.analyze evaluates its argument exactly once, even though
        // the diagnostic text itself never enters TContext.resultList.
        if let Some(rest) = line.strip_prefix("!log")
            && (rest.is_empty() || rest.starts_with(char::is_whitespace))
        {
            if self.is_active() {
                self.substitute_vars(rest.trim_start());
            }
            return true;
        }
        // !log, !pragma, !assert — consume silently.
        // !includeurl / !import — URL fetching deferred; strip the line silently.
        if self.preserve_teoz_pragma {
            let mut parts = line.split_whitespace();
            if matches!(parts.next(), Some("!pragma"))
                && matches!(parts.next(), Some("teoz"))
                && matches!(parts.next(), Some("true"))
                && parts.next().is_none()
            {
                return false;
            }
        }
        if line.starts_with("!log ")
            || line.starts_with("!log\t")
            || line == "!log"
            || line.starts_with("!pragma ")
            || line.starts_with("!pragma\t")
            || line.starts_with("!assert ")
            || line.starts_with("!assert\t")
            || line.starts_with("!includeurl ")
            || line.starts_with("!includeurl\t")
            || line.starts_with("!import ")
            || line.starts_with("!import\t")
        {
            return true;
        }
        false
    }

    fn try_define(&mut self, line: &str) -> bool {
        static RE_DEFINE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!define\s+(\w+)(?:\s+(.+))?$").unwrap());
        static RE_VAR: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!\$(\w+)\s*=\s*(.+)$").unwrap());
        static RE_LEGACY_VAR: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!(\w+)\s*=\s*(.+)$").unwrap());

        // !define with args: !define MACRO(a,b) body
        static RE_DEFINE_ARGS: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!define\s+(\w+)\(([^)]*)\)\s+(.+)$").unwrap());

        if let Some(caps) = RE_DEFINE_ARGS.captures(line) {
            if self.is_active() {
                let name = caps[1].to_string();
                let params: Vec<String> = caps[2]
                    .split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
                let body = caps[3].to_string();
                let body = self.current_buffered_line(&body);
                // Store as a definelong with a single-line body.
                self.definelong_macros.insert(
                    name,
                    DefineLongDef {
                        params,
                        body: vec![body],
                    },
                );
            }
            return true;
        }

        if let Some(caps) = RE_DEFINE.captures(line) {
            if self.is_active() {
                let name = caps[1].to_string();
                let value = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
                self.token_defines.insert(name, value);
            }
            return true;
        }
        if let Some(caps) = RE_VAR.captures(line) {
            if self.is_active() {
                let name = caps[1].to_string();
                let raw_value = caps[2].trim();
                let value = self.eval_expr_to_value(raw_value);
                self.set_var(&name, &value.to_display());
            }
            return true;
        }
        if let Some(caps) = RE_LEGACY_VAR.captures(line) {
            if self.is_active() {
                self.token_defines
                    .insert(caps[1].to_string(), caps[2].trim().to_string());
            }
            return true;
        }
        false
    }

    fn try_local_var(&mut self, line: &str) -> bool {
        // EaterAffectation.analyze selects LOCAL/GLOBAL before putVariable.
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!(local|global)\s+\$(\w+)\s*=\s*(.+)$").unwrap());
        if let Some(caps) = RE.captures(line) {
            if self.is_active() {
                let name = &caps[2];
                let value = self.eval_expr_to_value(caps[3].trim()).to_display();
                if &caps[1] == "global" {
                    self.defines.insert(name.to_string(), value);
                } else {
                    self.set_local_var(name, &value);
                }
            }
            return true;
        }
        false
    }

    fn try_definelong(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!definelong\s+(\w+)(?:\(([^)]*)\))?\s*$").unwrap());

        if let Some(caps) = RE.captures(line) {
            if self.is_active() {
                let name = caps[1].to_string();
                let params: Vec<String> = if let Some(p) = caps.get(2) {
                    p.as_str()
                        .split(',')
                        .map(|p| p.trim().to_string())
                        .filter(|p| !p.is_empty())
                        .collect()
                } else {
                    Vec::new()
                };
                self.definelong_macros.insert(
                    name.clone(),
                    DefineLongDef {
                        params,
                        body: Vec::new(),
                    },
                );
                self.collecting_definelong = Some(name);
            }
            return true;
        }
        if line == "!enddefinelong" {
            self.collecting_definelong = None;
            return true;
        }
        false
    }

    fn try_expand_definelong(&self, line: &str) -> Option<(String, bool)> {
        let trimmed = line.trim();
        // Try each definelong macro.
        for (name, dl) in &self.definelong_macros {
            // Check if line contains a macro call: NAME(args)
            if let Some(idx) = trimmed.find(&format!("{name}(")) {
                let after = &trimmed[idx + name.len()..];
                if let Some(end) = find_matching_paren(after) {
                    let args_str = &after[1..end];
                    let args = split_args(args_str);

                    // Expand: substitute params in body using whole-word replacement
                    // to avoid a short param name (e.g. "a") matching as a
                    // substring inside another param name (e.g. "label").
                    let mut result_lines = Vec::new();
                    for body_line in &dl.body {
                        let mut expanded = body_line.text.clone();
                        for (i, param) in dl.params.iter().enumerate() {
                            if let Some(arg) = args.get(i) {
                                // Use word-boundary regex so that a param like
                                // "a" does not replace the "a" inside "label".
                                if let Ok(re) =
                                    Regex::new(&format!(r"\b{}\b", regex::escape(param)))
                                {
                                    expanded = re.replace_all(&expanded, arg.as_str()).to_string();
                                }
                            }
                        }
                        // Apply ## token-pasting: collapse "foo##bar" → "foobar".
                        expanded = expanded.replace("##", "");
                        // TFunctionImpl.finalizeEnddefinelong converts a
                        // one-line body to LEGACY_DEFINE, which expands inline
                        // at the caller. Multiline bodies remain procedures and
                        // retain each StringLocated definition line.
                        if dl.body.len() > 1 {
                            expanded = source_line_marker(body_line.source_line, &expanded);
                        }
                        result_lines.push(expanded);
                    }
                    // For single-line macros, replace inline.
                    if result_lines.len() == 1 {
                        let prefix = &trimmed[..idx];
                        let suffix = &trimmed[idx + name.len() + end + 1..];
                        return Some((format!("{prefix}{}{suffix}", result_lines[0]), false));
                    }
                    return Some((result_lines.join("\n"), true));
                }
            }
            // Bare name match (no parens, no params).
            if dl.params.is_empty() && trimmed.contains(name.as_str()) {
                // Only substitute word-boundary matches.
                if let Ok(re) = Regex::new(&format!(r"\b{}\b", regex::escape(name)))
                    && re.is_match(trimmed)
                {
                    let body = dl
                        .body
                        .iter()
                        .map(|line| {
                            if dl.body.len() > 1 {
                                source_line_marker(line.source_line, &line.text)
                            } else {
                                line.text.clone()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    let result = re.replace_all(trimmed, body.as_str()).to_string();
                    return Some((result, dl.body.len() > 1));
                }
            }
        }
        None
    }

    fn try_function_def(&mut self, line: &str) -> bool {
        let trimmed = line.trim();
        // Currently collecting a function body?
        if let Some(func_name) = &self.collecting_function.clone() {
            if trimmed == "!endfunction" || trimmed == "!endprocedure" {
                self.collecting_function = None;
            } else {
                let raw_source_line = self.current_source_line;
                let source_line = self.current_diagram_source_line();
                let buffered = BufferedLine {
                    text: line.to_string(),
                    raw_source_line,
                    source_line,
                };
                if let Some(func) = self.functions.get_mut(func_name) {
                    func.body.push(buffered);
                }
            }
            return true;
        }

        // TIM also accepts a complete unquoted function on one line:
        // `!unquoted function $name($arg) !return expression`.
        static INLINE_RETURN_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^!(?:unquoted\s+)?function\s+\$?(\w+)\s*\((.*)\)\s+!return\s+(.+)$")
                .unwrap()
        });
        if let Some(caps) = INLINE_RETURN_RE.captures(trimmed) {
            let name = caps[1].to_string();
            let params = self.parse_func_params(&caps[2]);
            self.functions.insert(
                name,
                FunctionDef {
                    params,
                    body: vec![BufferedLine {
                        text: format!("!return {}", &caps[3]),
                        raw_source_line: self.current_source_line,
                        source_line: self.current_diagram_source_line(),
                    }],
                },
            );
            return true;
        }

        // !function $name($param1, $param2 = "default")
        // !procedure name() is also accepted by PlantUML themes.
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^!(?:function|procedure)\s+\$?(\w+)\s*\((.*)\)$").unwrap()
        });

        if let Some(caps) = RE.captures(trimmed) {
            let name = caps[1].to_string();
            let params = self.parse_func_params(&caps[2]);

            self.functions.insert(
                name.clone(),
                FunctionDef {
                    params,
                    body: Vec::new(),
                },
            );
            self.collecting_function = Some(name);
            return true;
        }

        false
    }

    fn parse_func_params(&mut self, parameters: &str) -> Vec<FuncParam> {
        if parameters.trim().is_empty() {
            return Vec::new();
        }
        split_builtin_args(parameters)
            .into_iter()
            .map(|parameter| {
                let (name, default) =
                    parameter
                        .split_once('=')
                        .map_or((parameter, None), |(name, expression)| {
                            // Eater.eatDeclareFunction stores the evaluated TValue now.
                            (name, Some(self.eval_expr_to_value(expression)))
                        });
                FuncParam {
                    name: name.trim().trim_start_matches('$').to_string(),
                    default,
                }
            })
            .collect()
    }

    /// Call a function and return its return value (if any) plus any output lines.
    fn call_function(
        &mut self,
        name: &str,
        args: &[String],
    ) -> (Option<Value>, Vec<EvaluatedEvent>) {
        let func = match self.functions.get(name) {
            Some(f) => f.clone(),
            None => return (None, Vec::new()),
        };

        // TFunctionImpl.getNewMemory binds evaluated arguments in a new
        // sibling frame without temporarily changing global variables.
        let mut scope = LocalMemory::default();
        for (i, param) in func.params.iter().enumerate() {
            let value = if let Some(arg) = args.get(i) {
                self.eval_expr_to_value(arg).to_display()
            } else {
                param
                    .default
                    .as_ref()
                    .map(Value::to_display)
                    .unwrap_or_default()
            };
            scope.overrides.insert(param.name.clone(), value);
        }
        self.local_vars.push(scope);
        // ExecutionContexts belongs to TMemory too; an early return from a
        // nested conditional must not leave that conditional in the caller.
        let outer_conditions = std::mem::take(&mut self.cond_stack);
        let outer_whiles = std::mem::take(&mut self.while_stack);
        let outer_foreaches = std::mem::take(&mut self.foreach_stack);

        // Save and clear any pending return signal from an outer context.
        let outer_return = self.return_signal.take();

        let mut output_lines = Vec::new();

        for body_line in &func.body {
            // Process through the preprocessor (handles !if/!while/!return etc.)
            if self.mark_function_body_source_lines {
                self.process_buffered_line(body_line, &mut output_lines);
            } else {
                self.process_one_line(&body_line.text, &mut output_lines);
            }
            // Check if process_one_line set a return signal.
            if self.return_signal.is_some() {
                break;
            }
        }

        let return_value = self.return_signal.take();
        // Restore outer return signal.
        self.return_signal = outer_return;

        // Pop local scope.
        self.local_vars.pop();

        self.cond_stack = outer_conditions;
        self.while_stack = outer_whiles;
        self.foreach_stack = outer_foreaches;

        (return_value, output_lines)
    }

    fn try_function_call(&mut self, line: &str, output: &mut Vec<EvaluatedEvent>) -> bool {
        // $funcName("arg1", "arg2"). PlantUML themes also use bare
        // procedure calls like `basic_style()` inside skinparam blocks.
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\$?(\w+)\s*\((.*)\)$").unwrap());

        let trimmed = line.trim();
        if let Some(caps) = RE.captures(trimmed) {
            let open = trimmed
                .find('(')
                .expect("call regex includes opening parenthesis");
            if find_matching_paren_from(trimmed, open) != Some(trimmed.len() - 1) {
                return false;
            }

            let name = caps[1].to_string();
            let args_str = &caps[2];

            if !self.functions.contains_key(&name) {
                return false;
            }

            if !self.is_active() {
                self.push_directive_placeholder(output);
                return true;
            }

            let args = parse_call_args(args_str);
            let (ret, mut lines) = self.call_function(&name, &args);
            let indent = &line[..line.len() - line.trim_start().len()];
            if !indent.is_empty()
                && let Some(EvaluatedEvent::Plain(generated)) = lines.first_mut()
            {
                if let Some((source_line, text)) = split_source_line_marker(generated) {
                    *generated = source_line_marker(source_line, &format!("{indent}{text}"));
                } else {
                    generated.insert_str(0, indent);
                }
            }
            let trailing = &line[line.trim_end().len()..];
            if !trailing.is_empty()
                && let Some(EvaluatedEvent::Plain(generated)) = lines.last_mut()
            {
                generated.push_str(trailing);
            }
            output.extend(lines);
            if let Some(value) = ret {
                let value = value.to_display();
                if !value.is_empty() {
                    self.push_content_line(output, value);
                }
            }

            // Java keeps StringLocated origins on both generated body lines
            // and subsequent caller lines. Output length is not an origin:
            // a procedure can emit zero, one or many lines, or a theme event.
            self.mark_source_lines = true;

            return true;
        }

        false
    }

    /// Evaluate a function call in an expression context, returning the return value.
    fn eval_function_call(&mut self, name: &str, args_str: &str) -> Value {
        let args = parse_call_args(args_str);
        let (ret, lines) = self.call_function(name, &args);
        self.expression_events.extend(lines);
        ret.unwrap_or(Value::Str(String::new()))
    }

    fn try_foreach(&mut self, line: &str, output: &mut Vec<EvaluatedEvent>) -> bool {
        let trimmed = line.trim();
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^!foreach\s+\$(\w+)\s+in\s+\[(.+)\]$"#).unwrap());

        if let Some(caps) = RE.captures(trimmed) {
            if !self.is_active() {
                return true;
            }
            let var_name = caps[1].to_string();
            let values_str = &caps[2];

            let values: Vec<String> = values_str
                .split(',')
                .map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string())
                .collect();

            self.foreach_stack.push(ForEachState {
                var_name,
                values,
                body_lines: Vec::new(),
            });
            return true;
        }

        if (trimmed == "!endfor" || trimmed == "!endforeach")
            && let Some(foreach) = self.foreach_stack.pop()
        {
            if self.is_active() {
                for val in &foreach.values {
                    let old_val = self.get_var(&foreach.var_name).cloned();
                    self.set_var(&foreach.var_name, val);

                    for body_line in &foreach.body_lines {
                        self.process_buffered_line(body_line, output);
                    }

                    match old_val {
                        Some(v) => {
                            self.set_var(&foreach.var_name, &v);
                        }
                        None => {
                            self.remove_var(&foreach.var_name);
                        }
                    }
                }
                self.mark_source_lines = true;
            }
            return true;
        }

        // If we're inside a foreach, buffer the line.
        if self.foreach_stack.last().is_some() {
            let buffered = self.current_buffered_line(line);
            if let Some(foreach) = self.foreach_stack.last_mut() {
                foreach.body_lines.push(buffered);
            }
            return true;
        }

        false
    }

    fn try_while(&mut self, line: &str, output: &mut Vec<EvaluatedEvent>) -> bool {
        let trimmed = line.trim();
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^!while\s+(.+)$").unwrap());

        if let Some(caps) = RE.captures(trimmed) {
            if !self.is_active() {
                return true;
            }
            let condition = caps[1].to_string();
            self.while_stack.push(WhileState {
                condition,
                body_lines: Vec::new(),
            });
            return true;
        }

        if trimmed == "!endwhile" {
            if let Some(while_state) = self.while_stack.pop()
                && self.is_active()
            {
                let mut iterations = 0;
                loop {
                    if self.error.is_some() || iterations >= MAX_WHILE_ITERATIONS {
                        break;
                    }
                    if !self.eval_bool(&while_state.condition) {
                        break;
                    }

                    for body_line in &while_state.body_lines {
                        self.process_buffered_line(body_line, output);
                    }

                    iterations += 1;
                }
                self.mark_source_lines = true;
            }
            return true;
        }

        // If we're inside a while, buffer the line.
        if self.while_stack.last().is_some() {
            let buffered = self.current_buffered_line(line);
            if let Some(while_state) = self.while_stack.last_mut() {
                while_state.body_lines.push(buffered);
            }
            return true;
        }

        false
    }

    fn try_include(&mut self, line: &str) -> Option<Vec<EvaluatedEvent>> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^!include(?:_once)?\s+(?:"([^"]+)"|<([^>]+)>|(\S+))$"#).unwrap()
        });

        let caps = RE.captures(line)?;
        // CodeIteratorIf suppresses includes before TContext executes them.
        if !self.is_active() {
            return Some(Vec::new());
        }
        // Group 1: quoted, Group 2: angle-bracket, Group 3: bare path.
        let quoted_path = caps.get(1).map(|m| m.as_str());
        let angle_path = caps.get(2).map(|m| m.as_str());
        let bare_path = caps.get(3).map(|m| m.as_str());
        let display_path = quoted_path
            .or(angle_path)
            .or(bare_path)
            .unwrap_or("<unknown>");

        if self.include_depth >= MAX_INCLUDE_DEPTH {
            return Some(vec![EvaluatedEvent::ParserOnly(format!(
                "' WARNING: max include depth ({MAX_INCLUDE_DEPTH}) reached for {display_path}"
            ))]);
        }

        // Handle known stdlib includes.
        if angle_path == Some("archimate/Archimate") || bare_path == Some("archimate/Archimate") {
            self.archimate_enabled = true;
            return Some(vec![]);
        }

        // Angle-bracket includes resolve via stdlib.
        if let Some(stdlib_path) = angle_path {
            return self.try_include_stdlib(stdlib_path);
        }

        let path_str = quoted_path.or(bare_path)?;
        let file_path = if let Some(base) = &self.base_dir {
            base.join(path_str)
        } else {
            PathBuf::from(path_str)
        };

        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                self.include_depth += 1;
                let lines = self.process(&content);
                self.include_depth -= 1;
                Some(lines)
            }
            Err(_) => Some(vec![]),
        }
    }

    /// Resolve and include a stdlib path (from angle-bracket includes).
    /// Returns `Some(lines)` always — the include directive is consumed even
    /// if the stdlib root or file is not found (silently ignored).
    fn try_include_stdlib(&mut self, include_path: &str) -> Option<Vec<EvaluatedEvent>> {
        let Some(stdlib_root) = stdlib::get_stdlib_root() else {
            return Some(vec![]);
        };
        let Some(resolved) = stdlib::resolve_stdlib_path(&stdlib_root, include_path) else {
            return Some(vec![]);
        };
        match std::fs::read_to_string(&resolved) {
            Ok(content) => {
                self.include_depth += 1;
                let lines = self.process(&content);
                self.include_depth -= 1;
                Some(lines)
            }
            Err(_) => Some(vec![]),
        }
    }

    fn try_includesub(&mut self, line: &str) -> Option<Vec<EvaluatedEvent>> {
        // !includesub file.puml!SUBNAME
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^!includesub\s+(?:"([^"]+)"|(\S+))!(\w+)$"#).unwrap());

        let caps = RE.captures(line)?;
        let path_str = caps.get(1).or(caps.get(2)).map(|m| m.as_str())?;
        let sub_name = &caps[3];
        if !self.is_active() {
            return Some(Vec::new());
        }

        if self.include_depth >= MAX_INCLUDE_DEPTH {
            return Some(vec![]);
        }

        let file_path = if let Some(base) = &self.base_dir {
            base.join(path_str)
        } else {
            PathBuf::from(path_str)
        };

        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                // Parse the file to extract subs.
                let mut temp_ctx = PreprocessContext::new(
                    self.base_dir.clone(),
                    self.mark_function_body_source_lines,
                );
                temp_ctx.include_depth = self.include_depth + 1;
                temp_ctx.preserve_teoz_pragma = self.preserve_teoz_pragma;
                let _ = temp_ctx.process(&content);
                // Now extract the named sub.
                if let Some(sub_lines) = temp_ctx.subs.get(sub_name) {
                    let mut output = Vec::new();
                    for line in sub_lines {
                        self.process_buffered_line(line, &mut output);
                    }
                    self.mark_source_lines = true;
                    Some(output)
                } else {
                    Some(vec![])
                }
            }
            Err(_) => Some(vec![]),
        }
    }

    fn try_theme(&mut self, line: &str) -> Option<Vec<EvaluatedEvent>> {
        let rest = line.strip_prefix("!theme ")?;
        let mut output = Vec::new();
        if !self.is_active() || self.include_depth >= MAX_INCLUDE_DEPTH {
            self.push_directive_placeholder(&mut output);
            return Some(output);
        }
        // EaterTheme.analyze applies functions and variables to both expressions.
        let (name, from) =
            rest.to_ascii_lowercase()
                .find(" from ")
                .map_or((rest.trim(), None), |index| {
                    (
                        rest[..index].trim(),
                        Some(rest[index + " from ".len()..].trim()),
                    )
                });
        let from = from.map(|value| self.substitute_vars(value));
        let name = self.substitute_vars(name);
        let source = if let Some(from) = from {
            // Local custom themes use the same naming convention as ThemeUtils.
            // Remote fetching remains outside the no-runtime-dependency product.
            let directory = self
                .base_dir
                .as_deref()
                .unwrap_or_else(|| Path::new("."))
                .join(from);
            std::fs::read_to_string(directory.join(format!("puml-theme-{name}.puml"))).ok()
        } else {
            themes::get_theme_source(&name).map(str::to_string)
        };
        if let Some(source) = source {
            let source_line = self
                .source_line_override
                .unwrap_or_else(|| self.current_diagram_source_line());
            let saved_mark = self.mark_function_body_source_lines;
            self.mark_function_body_source_lines = false;
            self.include_depth += 1;
            let body = self.process(themes::strip_front_matter(&source));
            self.include_depth -= 1;
            self.mark_function_body_source_lines = saved_mark;
            output.push(EvaluatedEvent::Theme {
                name,
                source_line,
                body,
            });
        } else {
            self.push_directive_placeholder(&mut output);
        }
        // Theme execution can add arbitrary generated content; subsequent
        // statements retain their StringLocated origin, including inactive loads.
        self.mark_source_lines = true;
        Some(output)
    }

    fn try_option(&self, line: &str) -> Option<Vec<EvaluatedEvent>> {
        let rest = line.strip_prefix("!option ")?;
        let mut parts = rest.split_whitespace();
        let name = parts.next()?;
        let value = parts.next().unwrap_or("true");
        let mut output = Vec::new();
        if self.is_active()
            && name.eq_ignore_ascii_case("handwritten")
            && value.eq_ignore_ascii_case("true")
        {
            output.push(EvaluatedEvent::ParserOnly(
                "skinparam __optionHandwritten true".to_string(),
            ));
        } else {
            self.push_directive_placeholder(&mut output);
        }
        Some(output)
    }

    fn try_undefine(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!undef(?:ine)?\s+\$?(\w+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            // CodeIteratorIf suppresses inactive lines before TContext reaches
            // EaterUndef. An inactive branch must not mutate outer memory.
            if self.is_active() {
                let name = &caps[1];
                self.remove_var(name);
                self.token_defines.remove(name);
            }
            return true;
        }
        false
    }

    fn try_conditional(&mut self, line: &str) -> bool {
        static RE_IF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^!if\s+(.+)$"#).unwrap());
        static RE_ELSEIF: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^!elseif\s+(.+)$"#).unwrap());
        static RE_IFDEF: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!ifdef\s+(.+)$").unwrap());
        static RE_IFNDEF: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!ifndef\s+\$?(\w+)$").unwrap());

        if let Some(caps) = RE_IFDEF.captures(line) {
            // PlantUML EaterIfdef delegates the complete expression to
            // EvalBoolean, whose Truth callback checks variables and
            // functions. Preserve its !, &&, || and parenthesis grammar.
            let defined = eval_defined_expression(&caps[1], |symbol| {
                let name = symbol.strip_prefix('$').unwrap_or(symbol);
                self.get_var(name).is_some()
                    || self.token_defines.contains_key(name)
                    || self.functions.contains_key(name)
            })
            .unwrap_or(false);
            self.cond_stack.push(CondState {
                active: defined && self.is_active(),
                has_matched: defined,
            });
            return true;
        }
        if let Some(caps) = RE_IFNDEF.captures(line) {
            let name = &caps[1];
            let defined = self.defines.contains_key(name)
                || self.token_defines.contains_key(name)
                || self.get_var(name).is_some();
            self.cond_stack.push(CondState {
                active: !defined && self.is_active(),
                has_matched: !defined,
            });
            return true;
        }
        if let Some(caps) = RE_IF.captures(line) {
            let expr = caps[1].trim();
            let result = self.is_active() && self.eval_bool(expr);
            self.cond_stack.push(CondState {
                active: result && self.is_active(),
                has_matched: result,
            });
            return true;
        }
        if let Some(caps) = RE_ELSEIF.captures(line) {
            let n = self.cond_stack.len();
            let parent_active = n <= 1 || self.cond_stack[..n - 1].iter().all(|c| c.active);
            let already_matched = self.cond_stack.last().is_some_and(|c| c.has_matched);
            let result = if already_matched {
                false
            } else {
                let expr = caps[1].trim();
                parent_active && self.eval_bool(expr)
            };
            if let Some(cond) = self.cond_stack.last_mut() {
                if already_matched {
                    cond.active = false;
                } else {
                    cond.active = result && parent_active;
                    if result {
                        cond.has_matched = true;
                    }
                }
            }
            return true;
        }
        if line == "!else" {
            let n = self.cond_stack.len();
            let parent_active = n <= 1 || self.cond_stack[..n - 1].iter().all(|c| c.active);
            if let Some(cond) = self.cond_stack.last_mut() {
                cond.active = !cond.has_matched && parent_active;
                cond.has_matched = true;
            }
            return true;
        }
        if line == "!endif" {
            self.cond_stack.pop();
            return true;
        }
        false
    }

    // ------------------------------------------------------------------
    // Expression evaluation
    // ------------------------------------------------------------------

    /// Evaluate an expression string to a Value.
    fn eval_expr_to_value(&mut self, expression: &str) -> Value {
        let expr = expression.trim();
        // TokenType.QUOTED_STRING / Eater.addUpTo: both quote forms are
        // literal; backslash is not an escape and contents are not interpolated.
        if let Some(quote) = expr.chars().next().filter(|c| matches!(c, '\'' | '"'))
            && let Some(end) = expr[1..].find(quote).map(|n| n + 1)
            && end + 1 == expr.len()
        {
            return Value::Str(expr[1..end].to_string());
        }
        if expr.starts_with('(') && find_matching_paren(expr) == Some(expr.len() - 1) {
            return self.eval_expr_to_value(&expr[1..expr.len() - 1]);
        }
        for ops in [&['+', '-'][..], &['*', '/', '%'][..]] {
            if let Some((position, op)) = ops
                .iter()
                .filter_map(|op| find_top_level_op_pos(expr, *op).map(|position| (position, *op)))
                .max_by_key(|(position, _)| *position)
            {
                let left = self.eval_expr_to_value(&expr[..position]);
                let right = self.eval_expr_to_value(&expr[position + 1..]);
                if op == '+' && (matches!(left, Value::Str(_)) || matches!(right, Value::Str(_))) {
                    return Value::Str(format!("{}{}", left.to_display(), right.to_display()));
                }
                let (a, b) = (left.to_number(), right.to_number());
                return number_value(match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' if b != 0.0 => a / b,
                    '%' if b != 0.0 => a % b,
                    _ => 0.0,
                });
            }
        }
        if let Some(open) = expr.find('(')
            && find_matching_paren_from(expr, open) == Some(expr.len() - 1)
        {
            let name = expr[..open].trim();
            let args = &expr[open + 1..expr.len() - 1];
            if let Some(name) = name.strip_prefix('%') {
                return Value::from_str_auto(&self.eval_builtin_with_expr_args(name, args));
            }
            let name = name.trim_start_matches('$');
            if self.functions.contains_key(name) {
                return self.eval_function_call(name, args);
            }
        }
        if let Some(name) = expr.strip_prefix('$')
            && let Some(value) = self.get_var(name).or_else(|| self.token_defines.get(name))
        {
            return Value::from_str_auto(value);
        }
        if let Some(value) = self.token_defines.get(expr) {
            return Value::Str(value.clone());
        }
        if expr == "true" {
            return Value::Bool(true);
        }
        if expr == "false" {
            return Value::Bool(false);
        }
        if let Ok(number) = expr.parse::<i64>() {
            return Value::Int(number);
        }
        if let Ok(number) = expr.parse::<f64>() {
            return Value::Float(number);
        }
        Value::Str(self.substitute_vars(expr))
    }

    // ------------------------------------------------------------------
    // Boolean evaluation for !if / !while conditions
    // ------------------------------------------------------------------

    fn eval_bool(&mut self, expr: &str) -> bool {
        let expr = expr.trim();

        // Strip outer parens.
        if expr.starts_with('(') && expr.ends_with(')') {
            let inner = &expr[1..expr.len() - 1];
            // Check if parens are balanced (i.e., they truly wrap the whole expression).
            if parens_balanced(inner) {
                return self.eval_bool(inner);
            }
        }

        // ||
        if let Some(pos) = find_top_level_logical_op(expr, "||") {
            let left = &expr[..pos];
            let right = &expr[pos + 2..];
            return self.eval_bool(left) || self.eval_bool(right);
        }

        // &&
        if let Some(pos) = find_top_level_logical_op(expr, "&&") {
            let left = &expr[..pos];
            let right = &expr[pos + 2..];
            return self.eval_bool(left) && self.eval_bool(right);
        }

        // Negation: !expr (but not != which is comparison).
        if let Some(rest) = expr.strip_prefix('!') {
            let rest = rest.trim();
            if !rest.is_empty() && !rest.starts_with('=') {
                return !self.eval_bool(rest);
            }
        }

        // Comparison operators: ==, !=, <=, >=, <, >
        for &op in &["==", "!=", "<=", ">=", "<", ">"] {
            if let Some(pos) = find_top_level_comparison(expr, op) {
                let left = expr[..pos].trim();
                let right = expr[pos + op.len()..].trim();
                return self.eval_comparison(left, right, op);
            }
        }

        // %true / %false
        if expr == "%true" || expr == "%true()" || expr == "true" {
            return true;
        }
        if expr == "%false" || expr == "%false()" || expr == "false" {
            return false;
        }

        // Numeric: non-zero is true.
        if let Ok(n) = expr.parse::<f64>() {
            return n != 0.0;
        }

        // Try evaluating as an expression (handles function calls, variable substitution).
        let val = self.eval_expr_to_value(expr);
        match val {
            Value::Bool(b) => b,
            Value::Int(n) => n != 0,
            Value::Float(f) => f != 0.0,
            Value::Str(ref s) => {
                // If the expression changed after evaluation, re-evaluate as bool.
                if s.trim() != expr {
                    let s_clone = s.clone();
                    return self.eval_bool(&s_clone);
                }
                // Non-empty string is true.
                !s.is_empty()
            }
        }
    }

    fn eval_comparison(&mut self, left: &str, right: &str, op: &str) -> bool {
        let lval = self.eval_expr_to_value(left);
        let rval = self.eval_expr_to_value(right);

        // If both are numeric, compare numerically.
        if lval.is_numeric() && rval.is_numeric() {
            let l = lval.to_number();
            let r = rval.to_number();
            return match op {
                "==" => (l - r).abs() < 1e-10,
                "!=" => (l - r).abs() >= 1e-10,
                "<" => l < r,
                ">" => l > r,
                "<=" => l <= r,
                ">=" => l >= r,
                _ => false,
            };
        }

        // String comparison.
        let l = lval.to_display();
        let r = rval.to_display();
        match op {
            "==" => l == r,
            "!=" => l != r,
            "<" => l < r,
            ">" => l > r,
            "<=" => l <= r,
            ">=" => l >= r,
            _ => false,
        }
    }

    // ------------------------------------------------------------------
    // Variable substitution
    // ------------------------------------------------------------------

    fn substitute_vars(&mut self, line: &str) -> String {
        let mut result = String::new();
        let mut offset = 0;
        while offset < line.len() {
            let ch = line[offset..].chars().next().unwrap();
            if ch == '<'
                && line[offset..].starts_with("<$")
                && let Some(end) = line[offset..].find('>')
            {
                result.push_str(&line[offset..offset + end + 1]);
                offset += end + 1;
                continue;
            }
            if matches!(ch, '$' | '%') {
                let name_end = line[offset + 1..]
                    .char_indices()
                    .find(|(_, c)| !c.is_alphanumeric() && *c != '_')
                    .map_or(line.len(), |(i, _)| offset + 1 + i);
                let name = &line[offset + 1..name_end];
                if line[name_end..].starts_with('(')
                    && (ch == '%' || self.functions.contains_key(name))
                    && let Some(close) = find_matching_paren_from(line, name_end)
                {
                    if ch == '%' {
                        result.push_str(
                            &self.eval_builtin_with_expr_args(name, &line[name_end + 1..close]),
                        );
                    } else {
                        let args = parse_call_args(&line[name_end + 1..close]);
                        let (value, mut events) = self.call_function(name, &args);
                        if let Some(value) = value {
                            self.expression_events.extend(events);
                            result.push_str(&value.to_display());
                        } else {
                            // TContext.pendingAdd / appendToLastResult. Keep
                            // every event variant, including themes and options.
                            if let Some(EvaluatedEvent::Plain(first)) = events
                                .iter_mut()
                                .find(|event| matches!(event, EvaluatedEvent::Plain(_)))
                            {
                                let (origin, text) = split_source_line_marker(first)
                                    .map_or((None, first.as_str()), |(n, t)| (Some(n), t));
                                let text = format!("{result}{text}");
                                *first =
                                    origin.map_or(text.clone(), |n| source_line_marker(n, &text));
                            }
                            if let Some(EvaluatedEvent::Plain(last)) = events
                                .iter_mut()
                                .rev()
                                .find(|event| matches!(event, EvaluatedEvent::Plain(_)))
                            {
                                last.push_str(&line[close + 1..]);
                            }
                            self.expression_events.extend(events);
                            self.text_consumed = true;
                            return String::new();
                        }
                    }
                    offset = close + 1;
                    continue;
                }
                if ch == '$'
                    && let Some(value) = self.get_var(name).or_else(|| self.token_defines.get(name))
                {
                    result.push_str(value);
                    offset = name_end;
                    continue;
                }
            }
            result.push(ch);
            offset += ch.len_utf8();
        }
        // Then apply !define token macros (word-boundary bare-word substitution).
        // Iterate until stable (up to 20 rounds) so that expansions containing
        // other macro names are themselves expanded (e.g. LABEL = "Step STEP"
        // with STEP = 1 → "Step 1").
        if !self.token_defines.is_empty() {
            for _ in 0..20 {
                let mut out = String::with_capacity(result.len());
                let chars: Vec<char> = result.chars().collect();
                let mut i = 0;
                let mut changed = false;
                while i < chars.len() {
                    // Find the start of a word (alphanumeric or underscore).
                    if chars[i].is_alphanumeric() || chars[i] == '_' {
                        // Find end of word.
                        let word_start = i;
                        while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                            i += 1;
                        }
                        let word: String = chars[word_start..i].iter().collect();
                        if let Some(replacement) = self.token_defines.get(&word) {
                            out.push_str(replacement);
                            changed = true;
                        } else {
                            out.push_str(&word);
                        }
                    } else {
                        out.push(chars[i]);
                        i += 1;
                    }
                }
                result = out;
                if !changed {
                    break;
                }
            }
        }

        result
    }

    /// Expand Archimate macros.
    fn try_expand_archimate(&self, line: &str) -> Option<String> {
        if !self.archimate_enabled {
            return None;
        }

        static ELEM_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^([A-Z][A-Za-z]+)_([A-Za-z]+)\s*\(\s*(\w+)\s*,\s*"([^"]*)"\s*\)$"#)
                .unwrap()
        });
        static REL_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^Rel_([A-Za-z_]+)\s*\(\s*(\w+)\s*,\s*(\w+)\s*,\s*"([^"]*)"\s*\)$"#)
                .unwrap()
        });

        if let Some(caps) = REL_RE.captures(line) {
            let rel_kind = &caps[1];
            let from = &caps[2];
            let to = &caps[3];
            let label = &caps[4];
            // PlantUML reports the stdlib procedure body line for Archimate
            // relation macros; Archimate.puml defines all `Rel_*` wrappers on
            // the same hidden arrow statement line.
            return Some(source_line_marker(
                349,
                &format!("archimate_rel {rel_kind} {from} {to} \"{label}\""),
            ));
        }

        if let Some(caps) = ELEM_RE.captures(line) {
            let layer = &caps[1];
            let kind = &caps[2];
            let id = &caps[3];
            let label = &caps[4];
            let expanded = format!("archimate_element {layer} {kind} {id} \"{label}\"");
            return Some(match self.archimate_element_source_line(layer, kind) {
                Some(source_line) => source_line_marker(source_line, &expanded),
                None => expanded,
            });
        }

        None
    }

    fn archimate_element_source_line(&self, layer: &str, kind: &str) -> Option<usize> {
        // Provenance: PlantUML stdlib `archimate/Archimate.puml`; these are
        // the body lines whose expanded `archimate` statements receive
        // `data-source-line` in Java's DESCRIPTION renderer.
        match (layer, kind) {
            ("Business", "Actor") => Some(110),
            ("Business", "Role") => Some(117),
            ("Business", "Process") => Some(127),
            ("Business", "Function") => Some(130),
            ("Application", "Component") => Some(167),
            ("Application", "Service") => Some(191),
            ("Technology", "Node") => Some(203),
            ("Technology", "SystemSoftware") => Some(210),
            ("Technology", "CommunicationNetwork") => Some(222),
            ("Technology", "Artifact") => Some(247),
            ("Motivation", "Stakeholder") => Some(270),
            ("Motivation", "Driver") => Some(274),
            ("Motivation", "Assessment") => Some(277),
            ("Motivation", "Goal") => Some(280),
            ("Motivation", "Principle") => Some(286),
            ("Motivation", "Requirement") => Some(289),
            ("Motivation", "Constraint") => Some(292),
            ("Implementation", "WorkPackage") => Some(311),
            ("Implementation", "Deliverable") => Some(314),
            ("Implementation", "Plateau") => Some(320),
            ("Implementation", "Gap") => Some(323),
            _ => None,
        }
    }

    // ------------------------------------------------------------------
    // Built-in %functions
    // ------------------------------------------------------------------

    /// Evaluate a builtin function call where each argument is itself an
    /// expression (possibly containing `$variables`).  This avoids the
    /// common problem where pre-substituted variable values contain commas
    /// (e.g. `%strlen($str)` with `$str = "Hello, World!"` should pass the
    /// full 13-character string, not split on the comma).
    fn eval_builtin_with_expr_args(&mut self, func: &str, args_raw: &str) -> String {
        if args_raw.trim().is_empty() {
            return self.eval_one_builtin_from_values(func, &[]);
        }

        // Split on top-level commas in the raw arg string (before substitution).
        let raw_parts = split_builtin_args(args_raw);
        // Evaluate each argument as an expression.
        let evaluated: Vec<String> = raw_parts
            .iter()
            .map(|a| {
                let a = a.trim();
                let val = self.eval_expr_to_value(a);
                val.to_display()
            })
            .collect();
        let refs: Vec<&str> = evaluated.iter().map(|s| s.as_str()).collect();
        self.eval_one_builtin_from_values(func, &refs)
    }

    /// Core builtin dispatcher taking pre-evaluated string arguments.
    fn eval_one_builtin_from_values(&mut self, func: &str, args: &[&str]) -> String {
        self.eval_one_builtin_args(func, args)
    }

    fn eval_one_builtin_args(&mut self, func: &str, args: &[&str]) -> String {
        match func {
            "strlen" => args
                .first()
                .map_or("0".to_string(), |s| s.len().to_string()),
            "substr" => {
                let s = args.first().copied().unwrap_or("");
                let start: usize = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
                let len: usize = args
                    .get(2)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| s.len().saturating_sub(start));
                s.chars().skip(start).take(len).collect()
            }
            "strpos" => {
                let haystack = args.first().copied().unwrap_or("");
                let needle = args.get(1).copied().unwrap_or("");
                haystack
                    .find(needle)
                    .map_or("-1".to_string(), |p| p.to_string())
            }
            "upper" => args.first().map_or(String::new(), |s| s.to_uppercase()),
            "lower" => args.first().map_or(String::new(), |s| s.to_lowercase()),
            "newline" => "\n".to_string(),
            "tab" => "\t".to_string(),
            // PlantUML's %true() and %false() return numeric 1 and 0, not
            // the strings "true"/"false". This matches the behaviour when
            // these values are used in arithmetic or display contexts.
            "true" => "1".to_string(),
            "false" => "0".to_string(),
            "date" => {
                let clock = &self.render_clock;
                if args.is_empty() {
                    return format_java_date(clock.epoch_ms as i64, &clock.tz);
                }
                // DateFunction.executeReturnFunction converts its second TValue
                // to a signed Java int (seconds), then multiplies by 1000L.
                let epoch_ms = if let Some(seconds) = args.get(1) {
                    let Ok(seconds) = seconds.parse::<i32>() else {
                        self.error = Some(PreprocessError {
                            line: self.current_diagram_source_line(),
                            message: "Date seconds outside signed 32-bit range".into(),
                        });
                        return String::new();
                    };
                    i64::from(seconds) * 1000
                } else {
                    clock.epoch_ms as i64
                };
                let tz = clock.pattern_timezone_at(epoch_ms);
                match date_format::format(epoch_ms, &tz, args[0]) {
                    Ok(formatted) => formatted,
                    Err(message) => {
                        self.error = Some(PreprocessError {
                            line: self.current_diagram_source_line(),
                            message,
                        });
                        String::new()
                    }
                }
            }
            "size" => {
                // Count array elements if the argument looks like a JSON array [a, b, c].
                // Otherwise fall back to string length.
                let s = args.first().copied().unwrap_or("");
                let trimmed = s.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    let inner = &trimmed[1..trimmed.len() - 1];
                    if inner.trim().is_empty() {
                        "0".to_string()
                    } else {
                        split_args(inner).len().to_string()
                    }
                } else {
                    s.len().to_string()
                }
            }
            "string" => args.first().map_or(String::new(), |s| s.to_string()),
            "intval" => args
                .first()
                .and_then(|s| s.parse::<f64>().ok())
                .map_or("0".to_string(), |n| format!("{}", n as i64)),
            "not" => {
                let val = args.first().copied().unwrap_or("false");
                if val == "true" || val == "1" {
                    "false"
                } else {
                    "true"
                }
                .to_string()
            }
            "chr" => {
                let code: u32 = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
                char::from_u32(code).map_or(String::new(), |c| c.to_string())
            }
            "darken" => {
                // PlantUML `%darken(color, ratio)` decreases luminance by a
                // *relative* fraction: `L -= L * (ratio / 100)`. Match Java's
                // `HColorSimple.darken` / `HSLColor` exactly.
                let color_str = args.first().copied().unwrap_or("");
                let amount: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                if let Some((r, g, b)) = parse_color(color_str) {
                    let (h, s, l) = rgb_to_hsl(r, g, b);
                    let new_l = (l - l * (amount / 100.0)).max(0.0);
                    let (r2, g2, b2) = hsl_to_rgb(h, s, new_l);
                    rgb_to_hex(r2, g2, b2)
                } else {
                    color_str.to_string()
                }
            }
            "lighten" => {
                // PlantUML `%lighten(color, ratio)` increases luminance by a
                // *relative* fraction: `L += L * (ratio / 100)`. Match Java's
                // `HColorSimple.lighten` / `HSLColor` exactly.
                let color_str = args.first().copied().unwrap_or("");
                let amount: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                if let Some((r, g, b)) = parse_color(color_str) {
                    let (h, s, l) = rgb_to_hsl(r, g, b);
                    let new_l = (l + l * (amount / 100.0)).min(100.0);
                    let (r2, g2, b2) = hsl_to_rgb(h, s, new_l);
                    rgb_to_hex(r2, g2, b2)
                } else {
                    color_str.to_string()
                }
            }
            "hsl_color" => {
                let h: f64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let s: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100.0);
                let l: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(50.0);
                let (r, g, b) = hsl_to_rgb(h, s, l);
                rgb_to_hex(r, g, b)
            }
            "is_dark" => {
                let color_str = args.first().copied().unwrap_or("");
                if let Some((r, g, b)) = parse_color(color_str) {
                    let (_, _, l) = rgb_to_hsl(r, g, b);
                    if l < 50.0 { "true" } else { "false" }.to_string()
                } else {
                    "false".to_string()
                }
            }
            "is_light" => {
                let color_str = args.first().copied().unwrap_or("");
                if let Some((r, g, b)) = parse_color(color_str) {
                    let (_, _, l) = rgb_to_hsl(r, g, b);
                    if l >= 50.0 { "true" } else { "false" }.to_string()
                } else {
                    "false".to_string()
                }
            }
            "variable_exists" => {
                // VariableExists calls memory.getVariable with the exact name.
                // Our TIM map omits the leading '$'. EaterAffectationDefine
                // stores bare !define names in global memory as well.
                // TValue.fromBoolean renders numeric 1 or 0.
                let name = args.first().copied().unwrap_or("");
                if name
                    .strip_prefix('$')
                    .and_then(|name| self.get_var(name))
                    .is_some()
                    || self.token_defines.contains_key(name)
                {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            "is_defined" => {
                let name = args.first().copied().unwrap_or("");
                if self.defines.contains_key(name) || self.token_defines.contains_key(name) {
                    "true"
                } else {
                    "false"
                }
                .to_string()
            }
            "function_exists" => {
                // PlantUML requires the function name to include the leading '$'.
                // %function_exists("$myFunc") → true, %function_exists("myFunc") → false.
                let name = args.first().copied().unwrap_or("");
                let lookup = name.strip_prefix('$').unwrap_or("");
                if !lookup.is_empty() && self.functions.contains_key(lookup) {
                    "true"
                } else {
                    "false"
                }
                .to_string()
            }
            "get_variable_value" => {
                let name = args.first().copied().unwrap_or("");
                name.strip_prefix('$')
                    .and_then(|name| self.get_var(name).cloned())
                    .or_else(|| self.token_defines.get(name).cloned())
                    .unwrap_or_default()
            }
            "set_variable_value" => {
                // In expression context we can't mutate; handled specially.
                // Return empty.
                String::new()
            }
            "filename" => {
                // PlantUML's `%filename()` returns the source file name (basename
                // without extension) when called from a real file. The picoweb /
                // "From string" pipeline used to generate our goldens produces
                // empty for this builtin (no source file). Return empty to match
                // that golden behaviour; file-API callers would need to thread
                // the actual filename through a separate channel.
                String::new()
            }
            "file_exists" => {
                // Always return false — file system access is not permitted in the
                // preprocessor (matches PlantUML sandboxed/server behaviour).
                "false".to_string()
            }
            "float" => {
                // %float("3.14") — parse string to float, return as string.
                let s = args.first().copied().unwrap_or("0");
                s.parse::<f64>()
                    .map_or_else(|_| "0".to_string(), |f| format!("{f}"))
            }
            "dec2hex" => {
                // %dec2hex(255) → "ff"
                let n: i64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
                format!("{n:x}")
            }
            "hex2dec" => {
                // %hex2dec("ff") → "255"
                let s = args.first().copied().unwrap_or("0");
                i64::from_str_radix(s.trim_start_matches("0x"), 16)
                    .map_or_else(|_| "0".to_string(), |n| n.to_string())
            }
            "dirpath" => {
                // %dirpath() — return the directory of the current file.
                // Without file context we return "." as a placeholder.
                self.base_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".".to_string())
            }
            "feature" => {
                // %feature("key") — query a PlantUML feature flag.
                // We don't support feature flags; always return "false".
                "false".to_string()
            }
            _ => {
                // Unknown function — return empty string.  A non-empty passthrough
                // like "%func(args)" confuses eval_bool (it would be truthy) and
                // loops incorrectly.  Returning "" is falsy and safe.
                String::new()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Format epoch milliseconds as Java's `Date.toString()`: `"dow mon dd hh:mm:ss tz yyyy"`.
///
/// Renders wall clock in the supplied timezone. Java's `Date.toString()`
/// has no sub-second precision, so the millisecond remainder is discarded.
fn format_java_date(epoch_ms: i64, tz: &TzSpec) -> String {
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let s = epoch_ms.div_euclid(1000) + i64::from(tz.offset_secs);
    let secs = ((s % 86400) + 86400) % 86400;
    let hh = secs / 3600;
    let mm = (secs % 3600) / 60;
    let ss = secs % 60;
    let dow = DAYS[(s.div_euclid(86400).rem_euclid(7)) as usize];

    // Days since epoch → year/month/day (civil calendar).
    let mut days = s.div_euclid(86400);
    let mut year = 1970i64;
    // DateFunction accepts negative epoch seconds; walk the Gregorian calendar
    // backwards as well as forwards from the Unix epoch.
    while days < 0 {
        year -= 1;
        days += if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366
        } else {
            365
        };
    }
    loop {
        let ylen = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366
        } else {
            365
        };
        if days < ylen {
            break;
        }
        days -= ylen;
        year += 1;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let mdays = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0usize;
    for (i, &ml) in mdays.iter().enumerate() {
        if days < ml {
            month = i;
            break;
        }
        days -= ml;
    }
    let day = days + 1;

    format!(
        "{dow} {mon} {day:02} {hh:02}:{mm:02}:{ss:02} {tz} {year}",
        mon = MONTHS[month],
        tz = tz.name,
    )
}

fn number_value(f: f64) -> Value {
    if f == (f as i64) as f64 && f.abs() < i64::MAX as f64 {
        Value::Int(f as i64)
    } else {
        Value::Float(f)
    }
}

fn parse_call_args(args_str: &str) -> Vec<String> {
    if args_str.trim().is_empty() {
        return Vec::new();
    }
    let args = split_args(args_str);
    args.into_iter().map(|a| a.trim().to_string()).collect()
}

// Eater.eatAndGetQuotedString consumes to the next matching quote, with
// no backslash escape. Keep byte offsets so non-ASCII token contents are safe.
fn syntax_positions(text: &str) -> Vec<(usize, char, i32)> {
    let mut result = Vec::new();
    let mut quote = None;
    let mut depth = 0;
    for (index, ch) in text.char_indices() {
        if let Some(end) = quote {
            if ch == end {
                quote = None;
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else {
            result.push((index, ch, depth));
            if ch == '(' {
                depth += 1;
            }
            if ch == ')' {
                depth -= 1;
            }
        }
    }
    result
}

fn split_args(text: &str) -> Vec<String> {
    split_builtin_args(text)
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .collect()
}

fn split_builtin_args(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (index, ch, depth) in syntax_positions(text) {
        if ch == ',' && depth == 0 {
            parts.push(&text[start..index]);
            start = index + 1;
        }
    }
    parts.push(&text[start..]);
    parts
}

fn find_matching_paren(text: &str) -> Option<usize> {
    text.starts_with('(')
        .then(|| find_matching_paren_from(text, 0))
        .flatten()
}

fn find_matching_paren_from(text: &str, open: usize) -> Option<usize> {
    syntax_positions(&text[open..])
        .into_iter()
        .find(|(_, ch, depth)| *ch == ')' && *depth == 1)
        .map(|(index, _, _)| open + index)
}

fn find_top_level_op_pos(expr: &str, op: char) -> Option<usize> {
    syntax_positions(expr)
        .into_iter()
        .filter(|(index, ch, depth)| {
            if *ch != op || *depth != 0 || *index == 0 {
                return false;
            }
            if op == '-' && expr[*index..].starts_with("->") {
                return false;
            }
            let before = expr[..*index].trim_end();
            !before.is_empty()
                && !before.ends_with(['+', '-', '*', '/', '%', '(', '=', '<', '>', '!'])
        })
        .map(|(index, _, _)| index)
        .next_back()
}

fn find_top_level_logical_op(expr: &str, op: &str) -> Option<usize> {
    syntax_positions(expr)
        .into_iter()
        .filter(|(index, _, depth)| *depth == 0 && expr[*index..].starts_with(op))
        .map(|(index, _, _)| index)
        .next_back()
}

fn find_top_level_comparison(expr: &str, op: &str) -> Option<usize> {
    syntax_positions(expr)
        .into_iter()
        .find(|(index, _, depth)| {
            if *depth != 0 || !expr[*index..].starts_with(op) {
                return false;
            }
            if op == "<" && (expr[*index..].starts_with("<=") || expr[*index..].starts_with("<>")) {
                return false;
            }
            if op == ">" && (expr[*index..].starts_with(">=") || expr[..*index].ends_with('-')) {
                return false;
            }
            true
        })
        .map(|(index, _, _)| index)
}

fn eval_defined_expression(expression: &str, truth: impl FnMut(&str) -> bool) -> Option<bool> {
    DefinedExpressionParser {
        input: expression.as_bytes(),
        pos: 0,
        truth,
    }
    .eval()
}

/// Recursive-descent port of PlantUML `preproc.EvalBoolean`, used by
/// `EaterIfdef`: `!` binds tighter than `&`, which binds tighter than `|`.
struct DefinedExpressionParser<'a, F> {
    input: &'a [u8],
    pos: usize,
    truth: F,
}

impl<F> DefinedExpressionParser<'_, F>
where
    F: FnMut(&str) -> bool,
{
    fn eval(mut self) -> Option<bool> {
        let value = self.parse_expression()?;
        self.skip_spaces();
        (self.pos == self.input.len()).then_some(value)
    }

    fn parse_expression(&mut self) -> Option<bool> {
        let mut value = self.parse_term()?;
        while self.eat(b'|') {
            self.eat(b'|');
            value |= self.parse_term()?;
        }
        Some(value)
    }

    fn parse_term(&mut self) -> Option<bool> {
        let mut value = self.parse_factor()?;
        while self.eat(b'&') {
            self.eat(b'&');
            value &= self.parse_factor()?;
        }
        Some(value)
    }

    fn parse_factor(&mut self) -> Option<bool> {
        if self.eat(b'!') {
            return self.parse_factor().map(|value| !value);
        }
        if self.eat(b'(') {
            let value = self.parse_expression()?;
            return self.eat(b')').then_some(value);
        }

        self.skip_spaces();
        let start = self.pos;
        while self
            .input
            .get(self.pos)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$'))
        {
            self.pos += 1;
        }
        if self.pos == start {
            return None;
        }
        let symbol = std::str::from_utf8(&self.input[start..self.pos]).ok()?;
        Some((self.truth)(symbol))
    }

    fn eat(&mut self, expected: u8) -> bool {
        self.skip_spaces();
        if self.input.get(self.pos) == Some(&expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn skip_spaces(&mut self) {
        while self
            .input
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }
}

fn parens_balanced(text: &str) -> bool {
    let positions = syntax_positions(text);
    !positions
        .iter()
        .any(|(_, ch, depth)| *ch == ')' && *depth <= 0)
        && positions.last().is_none_or(|(_, ch, depth)| {
            depth
                + if *ch == '(' {
                    1
                } else if *ch == ')' {
                    -1
                } else {
                    0
                }
                == 0
        })
}

/// Parse optional `[WxH/Z]` dimension spec from a sprite header line.
///
/// The `rest` parameter is the text after `sprite $` (name included).
/// Returns `(width, height)` where 0 means "infer from pixel data".
fn parse_sprite_dimensions(rest: &str) -> (u32, u32) {
    // Look for `[WxH/Z]` or `[WxH]` pattern anywhere in the rest string.
    if let Some(start) = rest.find('[')
        && let Some(end) = rest[start..].find(']')
    {
        let spec = &rest[start + 1..start + end];
        // Format: WxH/Z  or WxH
        let wh = spec.split('/').next().unwrap_or(spec);
        let mut parts = wh.splitn(2, 'x');
        let w: u32 = parts
            .next()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let h: u32 = parts
            .next()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        return (w, h);
    }
    (0, 0)
}

// StringLocated.removeInnerComment: only leading/trailing block comments and
// the special /'''[-\w]*'''/ form are removed; a trailing apostrophe is plain.
fn remove_inner_comment(line: &str) -> String {
    let trimmed = line.trim();
    let content = if trimmed.starts_with("/'") {
        line.find("'/").map_or(line, |end| &line[end + 2..])
    } else if trimmed.ends_with("'/") {
        line.rfind("/'").map_or(line, |start| &line[..start])
    } else {
        line
    };
    static SPECIAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/'''[-\w]*'''/").unwrap());
    SPECIAL.replace_all(content, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preprocess and drop blank placeholder lines. The preprocessor emits a
    /// blank line for every consumed directive so that surviving content keeps
    /// its original file line position (matching PlantUML's `data-source-line`).
    /// Content-focused tests use this helper to assert on the expanded output
    /// without threading the numbering placeholders through every expectation.
    fn pp(input: &str) -> Vec<String> {
        preprocess(input)
            .into_iter()
            .filter(|l| !l.is_empty())
            .collect()
    }

    #[test]
    fn expression_effects_reach_identity_and_parser_once() {
        let source = "@startuml\n!$calls = 0\n!function $configure()\n!$calls = $calls + 1\n!theme cerulean\n!return \"ok\"\n!endfunction\n!$label = $configure()\nclass \"$label $calls\" as Grove\n@enduml";
        let output = preprocess_full_for_parse(source, None);
        assert!(output.error.is_none());
        assert!(
            output
                .uml_source
                .ends_with("class \"ok 1\" as Grove\n@enduml\n")
        );
        assert_eq!(
            output
                .uml_source
                .matches("skinparam backgroundColor transparent")
                .count(),
            1
        );
        assert!(
            output
                .lines
                .iter()
                .any(|line| split_source_line_marker(line) == Some((8, "class \"ok 1\" as Grove")))
        );
        assert!(output.lines.iter().any(|line| {
            split_source_line_marker(line)
                .is_some_and(|(_, text)| text == "skinparam __theme cerulean")
        }));
    }

    #[test]
    fn diagnostic_evaluation_is_ordered_and_not_replayed() {
        let source = "@startuml\n!$n = 0\n!function $tick($digit)\n!$n = $n * 10 + $digit\n!return $digit\n!endfunction\n!log $tick(1) $tick(2)\n!if 0\n!log $tick(9)\n!endif\n!log $tick(3)\nclass Record$n\n@enduml";
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass Record123\n@enduml\n"
        );
    }

    #[test]
    fn declaration_defaults_capture_nested_values_once() {
        let source = "@startuml\n!$seed = \"first\"\n!function $value($ignored)\n!return $seed\n!endfunction\n!procedure $emit($label = $value('é,(x)'))\nclass \"$label\" as Grove\n!endprocedure\n!$seed = \"second\"\n$emit()\n@enduml";
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass \"first\" as Grove\n@enduml\n"
        );
    }

    #[test]
    fn quoted_lookup_tokens_preserve_names_and_unicode_boundaries() {
        let source = "@startuml\n!$cedar = \"green\"\n!function $name($ignored, $key)\n!return $key\n!endfunction\n!$a = %variable_exists(\"$\" + 'cedar')\n!$b = %get_variable_value($name('é,(x)', '$cedar'))\nclass \"$a / $b\" as Grove\n@enduml";
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass \"1 / green\" as Grove\n@enduml\n"
        );
    }

    #[test]
    fn a_label_with_parentheses_is_not_a_standalone_call() {
        let source = "@startuml\n!function $double($n)\n!return $n * 2\n!endfunction\nAlice -> Bob : double(3) = $double(3)\n@enduml";
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nAlice -> Bob : double(3) = 6\n@enduml\n"
        );
    }

    #[test]
    fn invalid_date_returns_a_located_preprocessing_error() {
        for expr in [
            "%date(\"QQ\", 0)",
            "%date(\"yyyy\", 2147483648)",
            "%date(\"XXXX\", 0)",
        ] {
            let output = preprocess_full(
                &format!("@startuml\n!$value = {expr}\nclass Lost\n@enduml"),
                None,
            );
            assert_eq!(output.error.as_ref().map(|error| error.line), Some(1));
            assert!(!output.uml_source.contains("class Lost"));
        }
    }

    #[test]
    fn evaluated_sprite_is_shared_by_identity_and_extraction() {
        let source = "@startuml\n!$pixels = \"FA\"\nsprite $badge [2x1/16] {\n$pixels\n}\nclass Ledger\n@enduml";
        let output = preprocess_full(source, None);
        assert_eq!(
            output.uml_source,
            "@startuml\nsprite $badge [2x1/16] {\nFA\n}\nclass Ledger\n@enduml\n"
        );
        assert_eq!(output.sprites["badge"].rows, ["FA"]);
        assert!(!output.lines.iter().any(|line| line == "FA"));
    }

    #[test]
    fn subblock_execution_updates_content_and_identity_once() {
        let source = "@startuml\n!$count = 0\n!startsub setup\n!$count = $count + 1\nskinparam classBorderColor #124578\n!endsub\nclass Record$count\n@enduml";
        let output = preprocess_full(source, None);
        assert_eq!(
            output.uml_source,
            "@startuml\nskinparam classBorderColor #124578\nclass Record1\n@enduml\n"
        );
        assert!(output.lines.iter().any(|line| line == "class Record1"));
        assert!(
            output
                .lines
                .iter()
                .any(|line| line == "skinparam classBorderColor #124578")
        );
    }

    #[test]
    fn inactive_theme_and_option_keep_following_source_origin() {
        let source = "@startuml\n!if 0\n!theme minty\n!option handwritten true\n!endif\nclass Ledger\n@enduml";
        let output = preprocess_full_for_parse(source, None);
        assert_eq!(output.uml_source, "@startuml\nclass Ledger\n@enduml\n");
        assert_eq!(output.lines[4], "class Ledger");
    }

    #[test]
    fn procedure_frames_are_siblings_rooted_at_global_memory() {
        let source = r#"@startuml
!$shared = "global"
!procedure $inner()
inside $shared $fresh
!endprocedure
!procedure $outer($shared)
!$fresh = "private"
$inner()
outer $shared $fresh
!endprocedure
$outer("argument")
after $shared $fresh
@enduml"#;
        assert_eq!(
            pp(source),
            [
                "inside global $fresh",
                "outer argument private",
                "after global $fresh"
            ]
        );
    }

    #[test]
    fn explicit_global_assignment_does_not_replace_parameter_override() {
        let source = r#"@startuml
!$name = "before"
!procedure $set($name)
!global $name = "after"
local $name
!endprocedure
$set("argument")
global $name
@enduml"#;
        assert_eq!(pp(source), ["local argument", "global after"]);
    }

    #[test]
    fn lookup_names_remain_literal_and_observe_current_frame() {
        let source = r#"@startuml
!$shade = "value"
!procedure $inspect($local)
%variable_exists("$shade") %variable_exists("shade") %variable_exists("$local")
!endprocedure
$inspect("payload")
%variable_exists("$local")
@enduml"#;
        assert_eq!(pp(source), ["1 0 1", "0"]);
    }

    #[test]
    fn explicit_date_uses_signed_seconds_and_expression_arguments() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.render_clock.tz = TzSpec {
            name: "UTC".to_string(),
            offset_secs: 0,
        };
        // Check the formatter independently of the host zone and current year.
        assert_eq!(
            date_format::format(-86400000, &ctx.render_clock.tz, "yyyy-MM-dd EEE").unwrap(),
            "1969-12-31 Wed"
        );
        let result = ctx
            .eval_expr_to_value("%date(\"yyyy\", 946684800)")
            .to_display();
        assert_eq!(result, "2000");
        assert_eq!(
            ctx.substitute_vars("%date(\"yyyy\", 946684800 + 86400)"),
            "2000"
        );
        assert_eq!(ctx.substitute_vars("%date(\"\", 0)"), "");
    }

    #[test]
    fn caller_origin_survives_a_procedure_that_emits_a_theme() {
        let input = "@startuml\n!procedure $setup()\n!theme cerulean\n!endprocedure\n$setup()\nclass Ledger\n@enduml";
        let output = preprocess_full_for_parse(input, None);
        assert!(
            output
                .lines
                .iter()
                .any(|line| split_source_line_marker(line) == Some((5, "class Ledger")))
        );
    }

    #[test]
    fn legacy_define_names_are_visible_to_lookup() {
        let source = "@startuml\n!define TOKEN amber\n%variable_exists(\"TOKEN\") %get_variable_value(\"TOKEN\")\n@enduml";
        assert_eq!(pp(source), ["1 amber"]);
    }

    #[test]
    fn strips_start_end_tags() {
        let input = "@startuml\nAlice -> Bob : hello\n@enduml\n";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello"]);
    }

    #[test]
    fn handles_startuml_with_name() {
        let input = "@startuml MyDiagram\nA -> B\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn handles_non_uml_tags() {
        let input = "@startjson\n{\"key\": \"value\"}\n@endjson";
        let lines = pp(input);
        assert_eq!(lines, vec!["{\"key\": \"value\"}"]);
    }

    #[test]
    fn define_substitution() {
        let input = "@startuml\n!define NAME Alice\n$NAME -> Bob : hello\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello"]);
    }

    #[test]
    fn var_assignment() {
        let input = "@startuml\n!$name = \"World\"\nAlice -> $name : hi\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> World : hi"]);
    }

    #[test]
    fn ifdef_defined() {
        let input = "@startuml\n!define FEATURE\n!ifdef FEATURE\nAlice -> Bob\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob"]);
    }

    #[test]
    fn ifdef_not_defined() {
        let input = "@startuml\n!ifdef FEATURE\nAlice -> Bob\n!endif\n@enduml";
        let lines = pp(input);
        assert!(lines.is_empty());
    }

    #[test]
    fn ifndef() {
        let input = "@startuml\n!ifndef FEATURE\nAlice -> Bob\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob"]);
    }

    #[test]
    fn if_else() {
        let input = "@startuml\n!define MODE prod\n!if $MODE == \"prod\"\nA -> B : production\n!else\nA -> B : dev\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B : production"]);
    }

    #[test]
    fn if_else_false_branch() {
        let input = "@startuml\n!define MODE dev\n!if $MODE == \"prod\"\nA -> B : production\n!else\nA -> B : dev\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B : dev"]);
    }

    #[test]
    fn logical_conditions_keep_renamed_and_or_branches() {
        let input = r#"@startuml
!$quartz = 7
!$saffron = 12
!if ($quartz > 3) && ($saffron > 9)
both
!endif
!if ($quartz > 99) || ($saffron == 12)
either
!endif
@enduml"#;

        assert_eq!(pp(input), vec!["both", "either"]);
    }

    #[test]
    fn ifdef_expression_and_inactive_undef_follow_java_truth_model() {
        let input = r#"@startuml
!define QUARTZ amber
!define SAFFRON gold
!ifdef QUARTZ && (!$MISSING || SAFFRON)
compound
!endif
!ifdef MISSING
!undef $QUARTZ
!endif
!ifdef QUARTZ
still-defined
!endif
!define QUARTZ violet
value-QUARTZ
!undef QUARTZ
!ifndef QUARTZ
now-undefined
!endif
@enduml"#;

        assert_eq!(
            pp(input),
            vec!["compound", "still-defined", "value-violet", "now-undefined"]
        );
    }

    #[test]
    fn single_line_comment() {
        let input = "@startuml\n' This is a comment\nAlice -> Bob\n@enduml";
        let lines = preprocess(input);
        // Empty placeholder for the comment line keeps downstream source-line
        // numbering aligned with the original input.
        assert_eq!(lines, vec!["", "Alice -> Bob"]);
    }

    #[test]
    fn block_comment() {
        let input = "@startuml\n/'\nThis is a\nblock comment\n'/\nAlice -> Bob\n@enduml";
        let lines = preprocess(input);
        // Each comment line emits an empty placeholder so original line
        // numbers survive into the parser.
        assert_eq!(lines, vec!["", "", "", "", "Alice -> Bob"]);
    }

    #[test]
    fn trailing_apostrophe_is_plain_text() {
        let input = "@startuml\nAlice -> Bob : hello ' with comment\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello ' with comment"]);
    }

    #[test]
    fn undef() {
        let input = "@startuml\n!define X yes\n!ifdef X\nA -> B\n!endif\n!undef X\n!ifdef X\nC -> D\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn nested_conditionals() {
        let input =
            "@startuml\n!define A\n!define B\n!ifdef A\n!ifdef B\ndeep\n!endif\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["deep"]);
    }

    #[test]
    fn nested_conditional_outer_false() {
        let input = "@startuml\n!ifdef MISSING\n!ifdef ALSO_MISSING\nnope\n!endif\n!endif\n@enduml";
        let lines = pp(input);
        assert!(lines.is_empty());
    }

    #[test]
    fn multiple_vars() {
        let input = "@startuml\n!define FROM Alice\n!define TO Bob\n!define MSG hello\n$FROM -> $TO : $MSG\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello"]);
    }

    #[test]
    fn empty_define() {
        let input = "@startuml\n!define FEATURE\n!ifdef FEATURE\nyes\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["yes"]);
    }

    #[test]
    fn include_file() {
        let dir = std::env::temp_dir().join("rustuml_test_include");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("common.puml"), "A -> B : included\n").unwrap();

        let input = "@startuml\n!include common.puml\nC -> D : local\n@enduml";
        let lines = preprocess_with_base(input, &dir);
        assert_eq!(lines, vec!["A -> B : included", "C -> D : local"]);
        let parse_lines = preprocess_full_for_parse(input, Some(dir.clone())).lines;
        let local = parse_lines
            .iter()
            .find_map(|line| {
                split_source_line_marker(line).filter(|(_, text)| *text == "C -> D : local")
            })
            .expect("following user line keeps an explicit source origin");
        assert_eq!(local.0, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn include_missing_file_is_silent() {
        let input = "@startuml\n!include nonexistent.puml\nA -> B\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn include_with_defines() {
        let dir = std::env::temp_dir().join("rustuml_test_include_defines");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("defs.puml"), "!define AUTHOR Alice\n").unwrap();

        let input = "@startuml\n!include defs.puml\n$AUTHOR -> Bob : hi\n@enduml";
        let lines = preprocess_with_base(input, &dir);
        assert_eq!(lines, vec!["Alice -> Bob : hi"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn foreach_basic() {
        let input = "@startuml\n!foreach $name in [\"Alice\", \"Bob\", \"Charlie\"]\nparticipant $name\n!endfor\n@enduml";
        let lines = pp(input);
        assert_eq!(
            lines,
            vec![
                "participant Alice",
                "participant Bob",
                "participant Charlie"
            ]
        );
    }

    #[test]
    fn foreach_with_message() {
        let input = "@startuml\n!foreach $x in [\"a\", \"b\"]\nAlice -> Bob : $x\n!endfor\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob : a", "Alice -> Bob : b"]);
    }

    #[test]
    fn foreach_preserves_other_vars() {
        let input = "@startuml\n!define WHO Alice\n!foreach $x in [\"hello\", \"world\"]\n$WHO -> Bob : $x\n!endfor\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello", "Alice -> Bob : world"]);
    }

    #[test]
    fn foreach_endforeach() {
        let input = "@startuml\n!foreach $x in [\"a\"]\nline $x\n!endforeach\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["line a"]);
    }

    #[test]
    fn function_basic() {
        let input = "@startuml\n!function $greet($name)\nAlice -> $name : hello\n!endfunction\n$greet(\"Bob\")\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello"]);
    }

    #[test]
    fn function_multiple_params() {
        let input = "@startuml\n!function $msg($from, $to, $text)\n$from -> $to : $text\n!endfunction\n$msg(\"Alice\", \"Bob\", \"hi\")\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob : hi"]);
    }

    #[test]
    fn function_no_params() {
        let input = "@startuml\n!function $header()\ntitle My Diagram\n!endfunction\n$header()\nAlice -> Bob\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["title My Diagram", "Alice -> Bob"]);
    }

    #[test]
    fn function_called_multiple_times() {
        let input = "@startuml\n!function $arrow($to)\nAlice -> $to : msg\n!endfunction\n$arrow(\"Bob\")\n$arrow(\"Charlie\")\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["Alice -> Bob : msg", "Alice -> Charlie : msg"]);
    }

    #[test]
    fn multiline_definelong_keeps_definition_and_caller_source_locations() {
        let input = r#"@startuml
!definelong RECORD_CLASS(name, field)
class name {
  +field: String
}
!enddefinelong
RECORD_CLASS(QuartzLedger, token)
RECORD_CLASS(SaffronArchive, label)
QuartzLedger --> SaffronArchive
@enduml"#;

        let lines = preprocess_full_for_parse(input, None).lines;
        let classes: Vec<_> = lines
            .iter()
            .filter_map(|line| split_source_line_marker(line))
            .filter(|(_, text)| text.starts_with("class "))
            .collect();
        assert_eq!(
            classes,
            vec![(2, "class QuartzLedger {"), (2, "class SaffronArchive {")]
        );
        assert!(lines.iter().any(|line| {
            split_source_line_marker(line) == Some((8, "QuartzLedger --> SaffronArchive"))
        }));
    }

    #[test]
    fn one_line_definelong_remains_inline_at_the_caller() {
        let input = r#"@startuml
!definelong FIELD(name, kind)
  +name: kind
!enddefinelong

class QuartzLedger {
  FIELD(token, String)
}
@enduml"#;

        let lines = preprocess_full_for_parse(input, None).lines;
        assert_eq!(lines[4], "class QuartzLedger {");
        assert_eq!(lines[5], "  +token: String");
    }

    #[test]
    fn return_function_calls_do_not_emit_body_directive_placeholders() {
        let input = r#"@startuml
!function $compose_badge($label = "Harbor")
  !return "[" + $label + "]"
!endfunction
note as Ledger
  $compose_badge()
  $compose_badge("Quartz")
end note
@enduml"#;

        assert_eq!(
            preprocess(input),
            vec![
                "",
                "",
                "",
                "note as Ledger",
                "[Harbor]",
                "[Quartz]",
                "end note",
            ]
        );
    }

    #[test]
    fn nested_return_functions_expand_with_renamed_arguments() {
        let input = r#"@startuml
!function $inner_token($seed)
  !return "<" + $seed + ">"
!endfunction
!function $outer_token($payload)
  !return "{" + $payload + "}"
!endfunction
note : $outer_token($inner_token("Saffron"))
@enduml"#;

        assert_eq!(pp(input), vec!["note : {<Saffron>}"]);
    }

    #[test]
    fn parser_preprocessing_keeps_procedure_body_locations() {
        // PlantUML TFunctionImpl.addBody stores StringLocated values from the
        // definition, and executeProcedureInternal passes that same body to
        // TContext.executeLines. A fresh Java oracle render of these renamed
        // entities therefore reports data-source-line="2" for both classes.
        let input = "@startuml\n\
!procedure $record($name)\n\
  class $name {\n\
    +marker: String\n\
  }\n\
!endprocedure\n\
$record(NorthwindLedger)\n\
$record(SaffronArchive)\n\
@enduml";

        let lines = preprocess_full_for_parse(input, None).lines;
        let class_locations: Vec<_> = lines
            .iter()
            .filter_map(|line| split_source_line_marker(line))
            .filter(|(_, text)| text.trim_start().starts_with("class "))
            .collect();

        assert_eq!(
            class_locations,
            vec![
                (2, "class NorthwindLedger {"),
                (2, "class SaffronArchive {"),
            ]
        );
    }

    #[test]
    fn ordinary_parser_preprocessing_still_consumes_teoz_pragma() {
        let input = "@startuml\n!pragma teoz true\nclass Ledger\n@enduml";
        let lines = preprocess_full_for_parse(input, None).lines;

        assert!(!lines.iter().any(|line| line.contains("!pragma teoz")));
    }

    #[test]
    fn procedure_syntax() {
        let input = "@startuml\n!procedure $setup($name)\nparticipant $name\n!endprocedure\n$setup(\"Alice\")\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["participant Alice"]);
    }

    #[test]
    fn procedure_identity_matches_java_post_tim_uml_source() {
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
        let output = preprocess_full(input, None);
        let expected = concat!(
            "@startuml\n",
            "\n",
            "    note as N_FreshLedger\n",
            "      <back:LightCyan>**AUDIT**: fresh payload</back>\n",
            "    end note\n",
            "@enduml\n",
        );

        assert_eq!(output.uml_source, expected);
        assert_eq!(svg_id_seed_prefix(&output.uml_source), "sm7v2tneejd6");
    }

    #[test]
    fn source_identity_preserves_raw_empty_lines_after_start() {
        let input = concat!(
            "@startuml\n",
            "\n",
            "\n",
            "skinparam shadowing true\n",
            "start\n",
            ":Fresh intake;\n",
            "if (route?) then (north)\n",
            "  :North alpha;\n",
            "  :North beta;\n",
            "else (south)\n",
            "  :South gamma;\n",
            "endif\n",
            ":Fresh archive;\n",
            "stop\n",
            "@enduml\n",
        );
        let output = preprocess_full(input, None);

        assert_eq!(
            output.uml_source,
            concat!(
                "@startuml\n",
                "\n",
                "\n",
                "skinparam shadowing true\n",
                "start\n",
                ":Fresh intake;\n",
                "if (route?) then (north)\n",
                "  :North alpha;\n",
                "  :North beta;\n",
                "else (south)\n",
                "  :South gamma;\n",
                "endif\n",
                ":Fresh archive;\n",
                "stop\n",
                "@enduml\n",
            ),
        );
    }

    // New tests for added features.

    #[test]
    fn variable_arithmetic() {
        let input = "@startuml\n!$x = 10\n!$y = 20\n!$sum = $x + $y\nnote : $sum\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : 30"]);
    }

    #[test]
    fn while_loop() {
        let input = "@startuml\n!$i = 1\n!while $i <= 3\nline $i\n!$i = $i + 1\n!endwhile\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["line 1", "line 2", "line 3"]);
    }

    #[test]
    fn elseif_branch() {
        let input = "@startuml\n!$x = 2\n!if $x == 1\nfirst\n!elseif $x == 2\nsecond\n!else\nother\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["second"]);
    }

    #[test]
    fn pragma_consumed() {
        let input = "@startuml\n!pragma teoz true\nA -> B\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn function_return_value() {
        let input = "@startuml\n!function $double($n)\n!return $n * 2\n!endfunction\n!$x = $double(5)\nnote : $x\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : 10"]);
    }

    #[test]
    fn numeric_comparison() {
        let input = "@startuml\n!$x = 5\n!if $x > 3\nyes\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["yes"]);
    }

    // ---------------------------------------------------------------------------
    // Tests for !includeurl, !import, !dump_memory (new directives)
    // ---------------------------------------------------------------------------

    #[test]
    fn includeurl_is_stripped_silently() {
        // !includeurl should consume the line without producing output or error.
        let input = "@startuml\n!includeurl https://example.com/common.puml\nA -> B\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn import_is_stripped_silently() {
        // !import is an alias for !includeurl — also silently consumed.
        let input = "@startuml\n!import https://example.com/lib.puml\nA -> B\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn dump_memory_no_vars() {
        // Java TContext.executeDumpMemory adds no source line.
        let input = "@startuml\n!dump_memory\nA -> B\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn dump_memory_with_vars() {
        let input = "@startuml\n!$foo = \"bar\"\n!$baz = \"qux\"\n!dump_memory\n@enduml";
        let lines = pp(input);
        assert!(lines.is_empty());
    }

    #[test]
    fn dump_memory_inactive_branch() {
        // !dump_memory inside an inactive branch should produce no output.
        let input = "@startuml\n!ifdef MISSING\n!dump_memory\n!endif\nA -> B\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    // ---------------------------------------------------------------------------
    // Tests for new built-in functions
    // ---------------------------------------------------------------------------

    #[test]
    fn builtin_float() {
        let input = "@startuml\n!$v = %float(\"3.14\")\nnote : $v\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : 3.14"]);
    }

    #[test]
    fn builtin_float_integer_string() {
        let input = "@startuml\n!$v = %float(\"42\")\nnote : $v\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : 42"]);
    }

    #[test]
    fn builtin_dec2hex() {
        let input = "@startuml\n!$h = %dec2hex(255)\nnote : $h\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : ff"]);
    }

    #[test]
    fn builtin_dec2hex_zero() {
        let input = "@startuml\n!$h = %dec2hex(0)\nnote : $h\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : 0"]);
    }

    #[test]
    fn builtin_hex2dec() {
        let input = "@startuml\n!$d = %hex2dec(\"ff\")\nnote : $d\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : 255"]);
    }

    #[test]
    fn builtin_hex2dec_zero() {
        let input = "@startuml\n!$d = %hex2dec(\"0\")\nnote : $d\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : 0"]);
    }

    #[test]
    fn builtin_dirpath_no_base() {
        // Without a base dir, %dirpath() returns ".".
        let input = "@startuml\n!$p = %dirpath()\nnote : $p\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : ."]);
    }

    #[test]
    fn builtin_dirpath_with_base() {
        let dir = std::env::temp_dir().join("rustuml_test_dirpath");
        let _ = std::fs::create_dir_all(&dir);
        let input = "@startuml\n!$p = %dirpath()\nnote : $p\n@enduml";
        let lines: Vec<String> = preprocess_with_base(input, &dir)
            .into_iter()
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(lines, vec![format!("note : {}", dir.display())]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn builtin_feature_returns_false() {
        // All features are unsupported; always returns "false".
        let input = "@startuml\n!$f = %feature(\"dark-mode\")\nnote : $f\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : false"]);
    }

    #[test]
    fn builtin_feature_in_conditional() {
        // %feature() returning "false" means the ifdef branch is inactive.
        let input = "@startuml\n!if %feature(\"dark-mode\") == \"true\"\ndark\n!else\nlight\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["light"]);
    }

    #[test]
    fn builtin_get_variable_value_requires_dollar_name() {
        let input = "@startuml\n!$myVar = \"hello\"\n!$a = %get_variable_value(\"myVar\")\n!$b = %get_variable_value(\"$myVar\")\nnote : [$a]|[$b]\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : []|[hello]"]);
    }

    #[test]
    fn sprite_block_is_collected() {
        let input =
            "@startuml\nsprite $disk [8x8/16] {\nFF00FF00\n00FF00FF\n}\nnote : hello\n@enduml";
        let out = preprocess_full(input, None);
        // The sprite block should NOT appear as content in the preprocessed
        // lines, but each block line is replaced by an empty placeholder so
        // downstream source-line numbers stay aligned with the original source.
        assert!(
            !out.lines.iter().any(|l| l.contains("FF00FF00")),
            "sprite pixel rows should not appear in output lines"
        );
        // 4 sprite-block lines (opener, 2 pixel rows, closer) → 4 placeholders.
        assert_eq!(out.lines, vec!["", "", "", "", "note : hello"]);
        // The sprite should be collected.
        assert!(
            out.sprites.contains_key("disk"),
            "sprite 'disk' should be parsed"
        );
        let s = &out.sprites["disk"];
        assert_eq!(s.rows, vec!["FF00FF00", "00FF00FF"]);
        assert_eq!(s.width, 8);
        assert_eq!(s.height, 2);
    }

    #[test]
    fn sprite_ref_preserved_in_output() {
        // <$name> references in diagram lines should be preserved.
        let input =
            "@startuml\nsprite $icon [4x4/16] {\nFFFF\nFFFF\n}\nnote : <$icon> hello\n@enduml";
        let out = preprocess_full(input, None);
        // 4 sprite-block lines become empty placeholders (source-line alignment).
        assert_eq!(out.lines, vec!["", "", "", "", "note : <$icon> hello"]);
        assert!(out.sprites.contains_key("icon"));
    }

    #[test]
    fn sprite_without_dimensions() {
        // Sprite blocks without [WxH/Z] annotation should still be parsed.
        let input = "@startuml\nsprite $dot {\nFF\n00\n}\nnote : test\n@enduml";
        let out = preprocess_full(input, None);
        assert!(
            out.sprites.contains_key("dot"),
            "sprite 'dot' should be parsed"
        );
        let s = &out.sprites["dot"];
        assert_eq!(s.rows, vec!["FF", "00"]);
    }

    #[test]
    fn include_angle_bracket_resolves_stdlib() {
        // Set up a fake stdlib directory with a .puml file.
        let dir = std::env::temp_dir().join("rustuml_test_stdlib_include");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("C4")).unwrap();
        std::fs::write(
            dir.join("C4/C4_Context.puml"),
            "!define C4_CONTEXT_LOADED\n",
        )
        .unwrap();

        // Use the thread-local override so the preprocessor finds our fake stdlib.
        stdlib::set_stdlib_override(Some(dir.clone()));

        let input = "@startuml\n!include <C4/C4_Context>\n!ifdef C4_CONTEXT_LOADED\nloaded\n!endif\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["loaded"]);

        stdlib::set_stdlib_override(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn include_angle_bracket_missing_is_silent() {
        // When the stdlib override points to a directory that exists but
        // doesn't contain the requested file, the include is silently ignored.
        let dir = std::env::temp_dir().join("rustuml_test_stdlib_missing_file");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        stdlib::set_stdlib_override(Some(dir.clone()));

        let input = "@startuml\n!include <nonexistent/Lib>\nA -> B\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["A -> B"]);

        stdlib::set_stdlib_override(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn include_quoted_still_resolves_relative() {
        // Quoted includes should still resolve relative to base_dir, not stdlib.
        let dir = std::env::temp_dir().join("rustuml_test_quoted_include");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("local.puml"), "participant Local\n").unwrap();

        let input = "@startuml\n!include \"local.puml\"\n@enduml";
        let lines = preprocess_with_base(input, &dir);
        assert_eq!(lines, vec!["participant Local"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn theme_expands_sub_blocks() {
        let input = "@startuml\n!theme aws-orange\nstart\n@enduml";
        let lines = pp(input);
        assert!(lines.contains(&"skinparam activityBarColor #1D8102".to_string()));
        assert!(lines.contains(&"skinparam activityStartColor #0073bb".to_string()));
        assert!(lines.contains(&"skinparam arrowThickness 3".to_string()));
        assert!(lines.contains(&"skinparam sequenceArrowThickness 3".to_string()));
    }

    #[test]
    fn theme_preserves_java_svg_seed_for_fresh_three_branch_activity() {
        let input = concat!(
            "@startuml\n",
            "!theme aws-orange\n",
            "start\n",
            ":Receive renamed parcel;\n",
            "if (fresh route?) then (primary)\n",
            "  :Validate primary lane;\n",
            "elseif (secondary route?) then (secondary)\n",
            "  :Validate secondary lane;\n",
            "else (fallback)\n",
            "  :Record fallback lane;\n",
            "endif\n",
            "stop\n",
            "@enduml\n",
        );
        let output = preprocess_full_for_parse(input, None);
        let seed_line = output
            .lines
            .iter()
            .find(|line| line.starts_with("skinparam __svgIdSeed "));
        assert!(
            seed_line.is_some_and(|line| line == "skinparam __svgIdSeed 1unuin6l8h5nq"),
            "SVG IDs use StringUtils.seed over the unflattened in-place theme expansion: {seed_line:?}"
        );
    }

    #[test]
    fn one_line_unquoted_function_expands_in_theme_style_content() {
        let output = preprocess(
            "@startuml\n\
             !unquoted function $join($left, $right) !return $left + $right\n\
             skinparam activityBackgroundColor $join(#123456, |#ABCDEF)\n\
             start\n\
             :Renamed action;\n\
             @enduml",
        );
        assert!(
            output
                .iter()
                .any(|line| line == "skinparam activityBackgroundColor #123456|#ABCDEF")
        );
    }

    #[test]
    fn option_handwritten_becomes_internal_skinparam() {
        let input = "@startuml\n!option handwritten true\nstart\n@enduml";
        let lines = pp(input);
        assert!(lines.contains(&"skinparam __optionHandwritten true".to_string()));
    }

    #[test]
    fn sketchy_theme_enables_handwritten_option() {
        let input = "@startuml\n!theme sketchy\nstart\n@enduml";
        let lines = pp(input);
        assert!(lines.contains(&"skinparam __optionHandwritten true".to_string()));
    }

    #[test]
    fn archimate_include_expands_renamed_element_and_relation_macros() {
        let output = preprocess(
            "@startuml\n\
             !include <archimate/Archimate>\n\
             Business_Process(novel_process_41, \"Fresh Process\")\n\
             Technology_Node(novel_node_43, \"Fresh Node\")\n\
             Rel_Serving(novel_node_43, novel_process_41, \"supports\")\n\
             @enduml",
        );

        assert!(
            output.iter().any(|line| line
                == "archimate_element Business Process novel_process_41 \"Fresh Process\""),
            "{output:?}"
        );
        assert!(
            output.iter().any(
                |line| line == "archimate_element Technology Node novel_node_43 \"Fresh Node\""
            ),
            "{output:?}"
        );
        assert!(
            output
                .iter()
                .any(|line| line
                    == "archimate_rel Serving novel_node_43 novel_process_41 \"supports\""),
            "{output:?}"
        );
    }

    #[test]
    fn theme_bare_procedures_expand_inside_skinparam_blocks() {
        let input = "@startuml\n!theme reddress-lightred\nstart\n@enduml";
        let lines = pp(input);
        assert!(lines.contains(&"skinparam activitybackgroundColor ccc".to_string()));
        assert!(lines.contains(&"skinparam activityborderColor aaa".to_string()));
        assert!(lines.contains(&"skinparam activityfontName Verdana".to_string()));
        assert!(lines.contains(&"skinparam activityfontSize 11".to_string()));
        assert!(lines.contains(&"skinparam activityarrowColor 000".to_string()));
        assert!(lines.contains(&"skinparam activityarrowFontName Verdana".to_string()));
    }

    #[test]
    fn include_angle_bracket_with_extension() {
        // Angle-bracket includes with explicit extension should also work.
        let dir = std::env::temp_dir().join("rustuml_test_stdlib_explicit_ext");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("lib/common.iuml"), "participant StdLib\n").unwrap();

        stdlib::set_stdlib_override(Some(dir.clone()));

        let input = "@startuml\n!include <lib/common.iuml>\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["participant StdLib"]);

        stdlib::set_stdlib_override(None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
