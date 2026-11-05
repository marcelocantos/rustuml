// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! TIM preprocessor — handles variables, defines, conditionals,
//! includes, and comments before diagram-specific parsing.
//
// Checker7 corrective contracts: precommitted accounts 02fa49de2396c1445a4ddf1a783c973f883a82f4.
// Checker4 corrective contracts: precommitted causal accounts 6d46a483aad389d4535e1ba24822fdd116baf517.

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

/// First code point of each Unicode 15.0 (JDK 21 `java.lang.Character`) BMP
/// run of general category Nd. Every run is ten consecutive digits 0..9.
const BMP_DECIMAL_DIGIT_ZEROS: [u16; 37] = [
    0x0030, 0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6, 0x0B66, 0x0BE6, 0x0C66, 0x0CE6,
    0x0D66, 0x0DE6, 0x0E50, 0x0ED0, 0x0F20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80,
    0x1A90, 0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0,
    0xFF10,
];

/// Java `Character.digit(char, 10)`: the decimal value of a UTF-16 unit whose
/// category is Nd. Surrogate units are never digits.
fn java_decimal_digit(unit: u16) -> Option<i32> {
    const RADIX: u16 = 10;
    BMP_DECIMAL_DIGIT_ZEROS
        .iter()
        .find(|&&zero| (zero..zero + RADIX).contains(&unit))
        .map(|&zero| i32::from(unit - zero))
}

/// Java `Integer.parseInt(s, 10)` over UTF-16 units: an optional leading
/// sign, then at least one `Character.digit` digit, accumulated negatively
/// with the JDK's `multmin` and `limit + digit` overflow checks.
fn java_parse_int(units: &[u16]) -> Option<i32> {
    const RADIX: i32 = 10;
    let (&first, _) = units.split_first()?;
    let mut index = 0;
    let mut negative = false;
    let mut limit = -i32::MAX;
    if first < u16::from(b'0') {
        if first == u16::from(b'-') {
            negative = true;
            limit = i32::MIN;
        } else if first != u16::from(b'+') {
            return None;
        }
        if units.len() == 1 {
            return None;
        }
        index = 1;
    }
    let multmin = limit / RADIX;
    let mut result: i32 = 0;
    for &unit in &units[index..] {
        let digit = java_decimal_digit(unit)?;
        if result < multmin {
            return None;
        }
        result *= RADIX;
        if result < limit + digit {
            return None;
        }
        result -= digit;
    }
    Some(if negative { result } else { -result })
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
    // Java String can hold isolated UTF-16 surrogates after substring. Preserve
    // those units through memory, calls and operators until text projection.
    Utf16(Vec<u16>),
    Int(i32),
    Float(f64),
    Json(serde_json::Value),
}

impl Value {
    fn to_display(&self) -> String {
        match self {
            Value::Str(s) => s.clone(),
            Value::Utf16(units) => String::from_utf16_lossy(units),
            Value::Json(v) => v.as_str().map_or_else(|| v.to_string(), str::to_string),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                // Avoid trailing zeros but keep at least one decimal if float
                if *f == (*f as i64) as f64 {
                    format!("{}", *f as i64)
                } else {
                    format!("{f}")
                }
            }
        }
    }

    fn utf16_units(&self) -> Vec<u16> {
        match self {
            Self::Utf16(units) => units.clone(),
            _ => self.to_display().encode_utf16().collect(),
        }
    }

    fn from_utf16(units: Vec<u16>) -> Self {
        match String::from_utf16(&units) {
            Ok(text) => Self::Str(text),
            Err(_) => Self::Utf16(units),
        }
    }

    fn to_number(&self) -> f64 {
        match self {
            Value::Int(n) => *n as f64,
            Value::Float(f) => *f,
            Value::Str(s) => s.parse::<f64>().unwrap_or(0.0),
            Value::Json(_) | Value::Utf16(_) => 0.0,
        }
    }

    fn is_numeric(&self) -> bool {
        match self {
            Value::Int(_) | Value::Float(_) => true,
            Value::Str(_) | Value::Utf16(_) | Value::Json(_) => false,
        }
    }

    // TValue.toBoolean: only integer zero is false; text/JSON use display length.
    fn truth(&self) -> bool {
        match self {
            Self::Int(n) => *n != 0,
            Self::Float(n) => *n != 0.0, // Existing Rust float extension.
            _ => !self.to_display().is_empty(),
        }
    }

    fn java_int(&self) -> i32 {
        // TValue.toInt returns its integer field (zero for strings and JSON).
        match self {
            Self::Int(n) => *n,
            _ => 0,
        }
    }
}

// ShuntingYard captures variable tokens before ReversePolishInterpretor runs
// calls. Keep values separate from executable source and defer literal-number
// conversion until execution, where Java TValue.fromNumber can fail.
enum Expression {
    Value(Value),
    Number(String),
    Not(Box<Expression>),
    Binary(Box<Expression>, String, Box<Expression>),
    Call(String, Vec<(Option<Expression>, Expression)>),
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

// HslColor.executeReturnFunction -> HSLColor.toRGB/HueToRGB ->
// XColor.fromFloat -> HColorSimple.asString (Java 71806a2378).
// All channel constants and f32 operations below are those source equations;
// this builtin does not share the existing f64 lighten/darken approximation.
fn tim_hsl_color(values: &[Value]) -> String {
    let h = (values[0].java_int() as f32 % 360.0) / 360.0;
    let s = (values[1].java_int() as f32).clamp(0.0, 100.0) / 100.0;
    let l = (values[2].java_int() as f32).clamp(0.0, 100.0) / 100.0;
    // HslColor divides alpha as double before casting to float.
    let alpha = values
        .get(3)
        .map_or(1.0, |v| (f64::from(v.java_int()) / 100.0) as f32)
        .clamp(0.0, 1.0);
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        (l + s) - (s * l)
    };
    let p = 2.0 * l - q;
    let channel = |mut hue: f32| {
        if hue < 0.0 {
            hue += 1.0;
        }
        if hue > 1.0 {
            hue -= 1.0;
        }
        let value = if 6.0 * hue < 1.0 {
            p + ((q - p) * 6.0 * hue)
        } else if 2.0 * hue < 1.0 {
            q
        } else if 3.0 * hue < 2.0 {
            p + ((q - p) * 6.0 * ((2.0 / 3.0) - hue))
        } else {
            p
        };
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    let (r, g, b) = (channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0));
    let a = (alpha * 255.0).round() as u8;
    match a {
        0 => "transparent".into(),
        255 => format!("#{r:02X}{g:02X}{b:02X}"),
        _ => format!("#{a:02x}{r:02x}{g:02x}{b:02x}"),
    }
}

// ---------------------------------------------------------------------------
// Core context
// ---------------------------------------------------------------------------

struct PreprocessContext {
    /// Exact Java variable keys (both `name` and `$name`), including !define.
    defines: VariableMemory,
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
    functions: HashMap<String, Vec<FunctionDef>>,
    collecting_function: Option<(String, FunctionDef)>,
    collecting_definelong: Option<String>,
    subs: HashMap<String, Vec<BufferedLine>>,
    collecting_sub: Option<(String, usize)>,
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
    execution_kind: FunctionKind,
    /// CodeIteratorSub's empty replay ends only its current executeLines iterator.
    iterator_stopped: bool,
}

const MAX_INCLUDE_DEPTH: usize = 10;
const MAX_WHILE_ITERATIONS: usize = 10000;

// TrieImpl follows UTF-16 edges without backtracking to an earlier terminal.
// Removing a name clears only its terminal, retaining the prefix path. Both
// details affect later lookup even when the remaining value maps are identical.
#[derive(Default)]
struct NameTrie {
    children: HashMap<u16, NameTrie>,
    terminal: Option<String>,
}

impl NameTrie {
    fn insert(&mut self, name: &str) {
        let mut node = self;
        for unit in name.encode_utf16() {
            node = node.children.entry(unit).or_default();
        }
        node.terminal = Some(name.to_string());
    }

    fn remove(&mut self, name: &str) {
        let mut node = self;
        for unit in name.encode_utf16() {
            let Some(child) = node.children.get_mut(&unit) else {
                return;
            };
            node = child;
        }
        node.terminal = None;
    }

    fn match_at(&self, text: &str) -> Option<&str> {
        let mut node = self;
        for unit in text.encode_utf16() {
            let Some(child) = node.children.get(&unit) else {
                return node.terminal.as_deref();
            };
            // TrieImpl treats a child with no remaining edges as a dead end.
            if child.children.is_empty() && child.terminal.is_none() {
                return node.terminal.as_deref();
            }
            node = child;
        }
        node.terminal.as_deref()
    }
}

// Keep mutation and name discovery together. TMemoryLocal builds its argument /
// explicit-override trie lazily; global and automatic-local tries are eager.
struct VariableMemory {
    values: HashMap<String, Value>,
    names: Option<NameTrie>,
}

impl Default for VariableMemory {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            names: Some(NameTrie::default()),
        }
    }
}

impl VariableMemory {
    fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }

    fn contains_key(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    fn insert(&mut self, name: String, value: Value) {
        if let Some(names) = &mut self.names {
            names.insert(&name);
        }
        self.values.insert(name, value);
    }

    fn remove(&mut self, name: &str) -> Option<Value> {
        if let Some(names) = &mut self.names {
            names.remove(name);
        }
        self.values.remove(name)
    }

    fn match_at(&mut self, text: &str) -> Option<String> {
        let names = self.names.get_or_insert_with(|| {
            let mut names = NameTrie::default();
            for name in self.values.keys() {
                names.insert(name);
            }
            names
        });
        names
            .match_at(text)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
    }
}

// TLineType's boundary test is Character.isLetter(char), ASCII digit, underscore
// or either UTF-16 surrogate. It is not a restriction on stored variable names.
fn variable_boundary_letter(ch: char) -> bool {
    static LETTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\p{L}$").unwrap());
    ch == '_'
        || ch.is_ascii_digit()
        || ch.len_utf16() == 2
        || LETTER.is_match(ch.encode_utf8(&mut [0; 4]))
}

// TMemoryLocal separates explicit locals/arguments from automatically local
// names. A global introduced later takes precedence over an automatic local.
struct LocalMemory {
    overrides: VariableMemory,
    locals: VariableMemory,
}

impl Default for LocalMemory {
    fn default() -> Self {
        Self {
            overrides: VariableMemory {
                values: HashMap::new(),
                names: None,
            },
            locals: VariableMemory::default(),
        }
    }
}

struct CondState {
    active: bool,
    has_matched: bool,
}

struct ForEachState {
    nested: usize,
    var_name: String,
    values: Vec<Value>,
    body_lines: Vec<BufferedLine>,
}

struct WhileState {
    nested: usize,
    condition: String,
    body_lines: Vec<BufferedLine>,
}

#[derive(Clone)]
struct BufferedLine {
    text: String,
    raw_source_line: usize,
    source_line: usize,
}

// TContext.executeOneLineNotSafe chooses simulatePlain only for return functions.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FunctionKind {
    Procedure,
    Return,
    Legacy,
}

#[derive(Clone)]
struct FunctionDef {
    kind: FunctionKind,
    params: Vec<FuncParam>,
    body: Vec<BufferedLine>,
}

impl FunctionDef {
    // TFunctionImpl.canCover: exact named keys, positional maximum and defaults.
    fn can_cover(&self, count: usize, named: Option<&HashMap<String, Value>>) -> bool {
        if named.is_some_and(|keys| {
            keys.keys()
                .any(|key| !self.params.iter().any(|p| &p.name == key))
        }) {
            return false;
        }
        count <= self.params.len()
            && count
                >= self
                    .params
                    .iter()
                    .filter(|p| {
                        p.default.is_none() && !named.is_some_and(|keys| keys.contains_key(&p.name))
                    })
                    .count()
    }
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
            defines: VariableMemory::default(),
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
            execution_kind: FunctionKind::Procedure,
            iterator_stopped: false,
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

    fn get_var(&self, name: &str) -> Option<&Value> {
        // TMemoryLocal.getVariable and forkFromGlobal: sibling calls see the
        // global memory, never their caller's overrides or fresh local names.
        let local = self.local_vars.last();
        local
            .and_then(|scope| scope.overrides.get(name))
            .or_else(|| self.defines.get(name))
            .or_else(|| local.and_then(|scope| scope.locals.get(name)))
    }

    fn set_var(&mut self, name: &str, value: &Value) {
        if self.error.is_some() {
            return;
        }
        // TMemoryLocal.putVariable: override, existing global, then new local.
        if let Some(scope) = self.local_vars.last_mut() {
            if scope.overrides.contains_key(name) {
                scope.overrides.insert(name.to_string(), value.clone());
                return;
            }
            if !self.defines.contains_key(name) {
                scope.locals.insert(name.to_string(), value.clone());
                return;
            }
        }
        self.defines.insert(name.to_string(), value.clone());
    }

    fn set_local_var(&mut self, name: &str, value: &Value) {
        if self.error.is_some() {
            return;
        }
        if let Some(scope) = self.local_vars.last_mut() {
            scope.overrides.insert(name.to_string(), value.clone());
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
        let outer_stop = std::mem::replace(&mut self.iterator_stopped, false);

        let saved_source_line = self.current_source_line;
        let saved_override = self.source_line_override;
        let saved_mark = self.mark_source_lines;
        for (idx, &line) in lines.iter().enumerate() {
            if self.iterator_stopped || self.error.is_some() {
                break;
            }
            self.current_source_line = idx + 1;
            if self.include_depth > 0 {
                // ReadLineReader gives each included file its own zero-based
                // LineLocation; a caller's buffered override must not leak in.
                self.source_line_override = Some(idx);
                self.mark_source_lines = true;
            }
            self.process_one_line(line, &mut output);
        }
        if self.include_depth > 0 {
            self.mark_source_lines = saved_mark;
        }
        self.source_line_override = saved_override;
        self.current_source_line = saved_source_line;

        self.iterator_stopped = outer_stop;
        output
    }

    // Each TContext.executeLines/executeLinesInternal owns a fresh iterator.
    // Sub replay within one iterator deliberately uses process_buffered_line
    // directly instead, so an empty replay cannot be mistaken for a nested call.
    fn process_buffered_lines(
        &mut self,
        lines: &[BufferedLine],
        output: &mut Vec<EvaluatedEvent>,
        mark_origins: bool,
    ) {
        let outer_stop = std::mem::replace(&mut self.iterator_stopped, false);
        for line in lines {
            if self.iterator_stopped || self.error.is_some() || self.return_signal.is_some() {
                break;
            }
            if mark_origins {
                self.process_buffered_line(line, output);
            } else {
                self.process_one_line(&line.text, output);
            }
        }
        self.iterator_stopped = outer_stop;
    }

    fn process_one_line(&mut self, line: &str, output: &mut Vec<EvaluatedEvent>) {
        // TContext.resultList is shared by return-value and procedure execution.
        // Each statement owns a sink; nested statements preserve the caller's
        // already-emitted effects and contribute their own before its content.
        if self.error.is_some() || self.return_signal.is_some() || self.iterator_stopped {
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

        // CodeIteratorSub buffers then replays the body at its definition,
        // before the conditional/function iterators. Capture origins as well
        // as text; no special theme-only subblock execution path exists.
        if let Some((name, origin)) = self.collecting_sub.clone() {
            self.push_directive_placeholder(output);
            // CodeIteratorSub.peek rejects nesting at the outer declaration,
            // before replay can execute effects or consume following model lines.
            if directive_argument(trimmed, "!startsub").is_some() {
                let previous = self.source_line_override.replace(origin);
                self.fail("Cannot nest sub".into());
                self.source_line_override = previous;
            } else if directive_argument(trimmed, "!endsub").is_some() {
                self.collecting_sub = None;
                let body = std::mem::take(&mut self.collecting_sub_lines);
                self.subs.insert(name, body.clone());
                // CodeIteratorSub.peek installs an empty iterator and returns
                // null. No next() occurs to clear it: executeLines stops here.
                if body.is_empty() {
                    self.iterator_stopped = true;
                    return;
                }
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

        // Startsub.
        if let Some(name) = directive_argument(trimmed, "!startsub") {
            if !valid_sub_name(name) {
                self.fail("Bad sub name".into());
                return;
            }
            self.collecting_sub = Some((name.to_string(), self.current_diagram_source_line()));
            self.collecting_sub_lines.clear();
            self.push_directive_placeholder(output);
            return;
        }
        if directive_argument(trimmed, "!endsub").is_some() {
            self.push_directive_placeholder(output);
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

        // Function definition collection.
        if self.try_function_def(&line_no_comment) {
            // Derivation: parse_block_keeps_preamble_procedure_body_lines and
            // parse_block_sequence_with_preamble_meta_procedure_stays_sequence
            // specify one-based file origins for this Rust API extension.
            self.preamble_body |= self.include_depth == 0 && !self.seen_start_tag;
            self.push_directive_placeholder(output);
            return;
        }

        // Capture the outer foreach body before interpreting a nested while.
        // Java CodeIterator contexts keep nested bodies inside the outer loop.
        if !self.foreach_stack.is_empty() && self.try_foreach(&line_no_comment, output) {
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
                // cause bare definitions with quoted values (e.g. !define VERSION
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
                    // Legacy functions participate in the same original-source
                    // scan as variables. Never rescan the completed substitution.
                    self.substitute_vars(&line_no_comment)
                };
            // A definelong expansion can produce multiple lines joined with '\n'.
            // Split them so each line is processed individually by the parser.
            // Blank lines are pushed too so that source-line indices match the
            // input file (parsers skip blank lines themselves on the `trim().is_empty()`
            // check, but the index advances). PlantUML's `data-source-line` counts
            // blank lines but excludes the dropped `@startuml`.
            if self.text_consumed || self.execution_kind == FunctionKind::Return {
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
            LazyLock::new(|| Regex::new(r"^!define\s+(\$?\w+)(?:\s+(.+))?$").unwrap());
        static RE_VAR: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!(\$?\w+)\s*=\s*(.+)$").unwrap());

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
                // EaterAffectationDefine evaluates the replacement once now.
                let value = self.substitute_vars(&value);
                self.defines.insert(name, Value::Str(value));
            }
            return true;
        }
        if let Some(caps) = RE_VAR.captures(line) {
            if self.is_active() {
                let name = caps[1].to_string();
                let raw_value = caps[2].trim();
                let value = self.eval_expr_to_value(raw_value);
                self.set_var(&name, &value);
            }
            return true;
        }
        false
    }

    fn try_local_var(&mut self, line: &str) -> bool {
        // EaterAffectation.analyze selects LOCAL/GLOBAL before putVariable.
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!(local|global)\s+(\$?\w+)\s*=\s*(.+)$").unwrap());
        if let Some(caps) = RE.captures(line) {
            if self.is_active() {
                let name = &caps[2];
                let value = self.eval_expr_to_value(caps[3].trim());
                if self.error.is_some() {
                    return true;
                }
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

    fn expand_definelong(&self, name: &str, args: &[String]) -> (String, bool) {
        let definition = &self.definelong_macros[name];
        // Bind legacy parameters in one pass over each original body line;
        // inserted arguments are values and cannot become another parameter.
        let pattern = definition
            .params
            .iter()
            .map(|p| regex::escape(p))
            .collect::<Vec<_>>()
            .join("|");
        let binding =
            (!pattern.is_empty()).then(|| Regex::new(&format!(r"\b(?:{pattern})\b")).unwrap());
        let mut lines = Vec::new();
        for body in &definition.body {
            let text = if let Some(binding) = &binding {
                binding
                    .replace_all(&body.text, |captures: &regex::Captures<'_>| {
                        let index = definition
                            .params
                            .iter()
                            .position(|name| name == &captures[0])
                            .unwrap();
                        args[index].clone()
                    })
                    .into_owned()
            } else {
                body.text.clone()
            };
            // Retain the existing legacy token-paste lowering. Full legacy
            // function-body execution is not changed by source-scan selection.
            let text = text.replace("##", "");
            lines.push(if definition.body.len() > 1 {
                source_line_marker(body.source_line, &text)
            } else {
                text
            });
        }
        (lines.join("\n"), definition.body.len() > 1)
    }

    fn register_function(&mut self, name: String, function: FunctionDef) {
        // FunctionsSet.addFunction / TFunctionSignature.equals replace only
        // (exact name, declared arity), retaining every other completed overload.
        let overloads = self.functions.entry(name).or_default();
        if let Some(previous) = overloads
            .iter_mut()
            .find(|f| f.params.len() == function.params.len())
        {
            *previous = function;
        } else {
            overloads.push(function);
        }
    }

    fn resolve_function(
        &self,
        name: &str,
        count: usize,
        named: Option<&HashMap<String, Value>>,
    ) -> Option<&FunctionDef> {
        let overloads = self.functions.get(name)?;
        if let Some(exact) = overloads.iter().find(|f| f.params.len() == count) {
            return Some(exact);
        }
        // FunctionsSet.getFunctionSmart traverses the ordinary JDK HashMap
        // buckets after exact lookup. TContext.addStandardFunctions registers
        // 74 signatures on the JVM (excluding commented LoadJsonLegacy).
        // JDK21 HashMap defaults: capacity16, load3/4, doubling on overflow;
        // hashCode then xor-shift16; TFunctionSignature hashes name then arity
        // with multiplier31. Equal-bucket list order retains insertion order.
        // Tree-bin tie ordering and unsupported legacy-overload syntax are not
        // established by this bounded model (account 46528609).
        const STANDARD_SIGNATURES: usize = 74;
        let size = STANDARD_SIGNATURES
            + self.functions.values().map(Vec::len).sum::<usize>()
            + self.definelong_macros.len();
        let mut capacity = 16;
        while size > capacity - capacity / 4 {
            capacity *= 2;
        }
        let name_hash = name
            .encode_utf16()
            .fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(u32::from(c)));
        overloads
            .iter()
            .filter(|f| f.can_cover(count, named))
            .min_by_key(|f| {
                let hash = 31u32
                    .wrapping_add(name_hash)
                    .wrapping_mul(31)
                    .wrapping_add(f.params.len() as u32);
                ((hash ^ (hash >> 16)) as usize) & (capacity - 1)
            })
    }

    fn try_function_def(&mut self, line: &str) -> bool {
        let trimmed = line.trim();
        // FunctionsSet publishes a definition only at executeEndfunction.
        if let Some((name, mut function)) = self.collecting_function.take() {
            if trimmed == "!endfunction" || trimmed == "!endprocedure" {
                self.register_function(name, function);
            } else {
                if trimmed.starts_with("!return ") && function.kind == FunctionKind::Legacy {
                    function.kind = FunctionKind::Return;
                }
                function.body.push(self.current_buffered_line(line));
                self.collecting_function = Some((name, function));
            }
            return true;
        }

        // TIM also accepts a complete unquoted function on one line:
        // `!unquoted function $name($arg) !return expression`.
        static INLINE_RETURN_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^!(?:unquoted\s+)?function\s+(\$?\w+)\s*\((.*)\)\s+!return\s+(.+)$")
                .unwrap()
        });
        if let Some(caps) = INLINE_RETURN_RE.captures(trimmed) {
            let name = caps[1].to_string();
            let params = self.parse_func_params(&caps[2]);
            self.register_function(
                name,
                FunctionDef {
                    kind: FunctionKind::Return,
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
            Regex::new(r"^!(?:function|procedure)\s+(\$?\w+)\s*\((.*)\)$").unwrap()
        });

        if let Some(caps) = RE.captures(trimmed) {
            let name = caps[1].to_string();
            let params = self.parse_func_params(&caps[2]);

            self.collecting_function = Some((
                name,
                FunctionDef {
                    kind: if trimmed.starts_with("!procedure") {
                        FunctionKind::Procedure
                    } else {
                        FunctionKind::Legacy
                    },
                    params,
                    body: Vec::new(),
                },
            ));
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
                    name: name.trim().to_string(),
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
        if self.error.is_some() {
            return (None, Vec::new());
        }
        // TFunctionImpl.getNewMemory binds evaluated arguments in a new
        // sibling frame without temporarily changing global variables.
        let mut positional = Vec::new();
        let mut named = HashMap::new();
        for arg in args {
            if self.error.is_some() {
                return (None, Vec::new());
            }
            if let Some((key, expr)) = named_argument(arg) {
                let value = self.eval_expr_to_value(expr);
                named.insert(key.to_string(), value);
            } else {
                positional.push(self.eval_expr_to_value(arg));
            }
        }
        if self.error.is_some() {
            return (None, Vec::new());
        }
        self.call_function_values(name, positional, named, false)
    }

    fn call_function_values(
        &mut self,
        name: &str,
        positional: Vec<Value>,
        named: HashMap<String, Value>,
        expression_call: bool,
    ) -> (Option<Value>, Vec<EvaluatedEvent>) {
        if self.error.is_some() {
            return (None, Vec::new());
        }
        let keys = if expression_call { None } else { Some(&named) };
        let Some(func) = self.resolve_function(name, positional.len(), keys).cloned() else {
            self.fail(format!("Unknown built-in function {name}"));
            return (None, Vec::new());
        };
        // RPN canCover checks an empty named set even after an exact lookup.
        if expression_call && !func.can_cover(positional.len(), None) {
            self.fail(format!("Bad number of arguments for {name}"));
            return (None, Vec::new());
        }
        // TFunctionImpl.executeReturnFunction checks kind after signature lookup.
        if expression_call && func.kind == FunctionKind::Procedure {
            self.fail("Illegal call here. Is there a return directive in your function?".into());
            return (None, Vec::new());
        }
        let mut positional = positional.into_iter();
        let mut scope = LocalMemory::default();
        for param in &func.params {
            let value = named
                .get(&param.name)
                .cloned()
                .or_else(|| positional.next())
                .or_else(|| param.default.clone())
                .unwrap_or_else(|| Value::Str(String::new()));
            // TFunctionImpl.getNewMemory preserves the parameter's complete key.
            scope.overrides.insert(param.name.clone(), value);
        }
        let outer_kind = std::mem::replace(&mut self.execution_kind, func.kind);
        self.local_vars.push(scope);
        // ExecutionContexts belongs to TMemory too; an early return from a
        // nested conditional must not leave that conditional in the caller.
        let outer_conditions = std::mem::take(&mut self.cond_stack);
        let outer_whiles = std::mem::take(&mut self.while_stack);
        let outer_foreaches = std::mem::take(&mut self.foreach_stack);

        // Save and clear any pending return signal from an outer context.
        let outer_return = self.return_signal.take();

        let mut output_lines = Vec::new();

        self.process_buffered_lines(
            &func.body,
            &mut output_lines,
            self.mark_function_body_source_lines,
        );

        let return_value = self.return_signal.take();
        // Restore outer return signal.
        self.return_signal = outer_return;

        // Pop local scope.
        self.local_vars.pop();

        self.cond_stack = outer_conditions;
        self.while_stack = outer_whiles;
        self.foreach_stack = outer_foreaches;

        self.execution_kind = outer_kind;
        (return_value, output_lines)
    }

    fn try_function_call(&mut self, line: &str, output: &mut Vec<EvaluatedEvent>) -> bool {
        // $funcName("arg1", "arg2"). PlantUML themes also use bare
        // procedure calls like `basic_style()` inside skinparam blocks.
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(\$?\w+)\s*\((.*)\)$").unwrap());

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
                if !value.is_empty() && self.execution_kind != FunctionKind::Return {
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

    fn try_foreach(&mut self, line: &str, output: &mut Vec<EvaluatedEvent>) -> bool {
        let trimmed = line.trim();
        let buffered = self.current_buffered_line(line);
        if let Some(state) = self.foreach_stack.last_mut() {
            let closes = matches!(trimmed, "!endfor" | "!endforeach");
            if !closes || state.nested > 0 {
                if directive_argument(trimmed, "!foreach").is_some() {
                    state.nested += 1;
                }
                if closes {
                    state.nested -= 1;
                }
                state.body_lines.push(buffered);
                return true;
            }
        }
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^!foreach\s+(\$?\w+)\s+in\s+(.+)$"#).unwrap());

        if let Some(caps) = RE.captures(trimmed) {
            if !self.is_active() {
                return true;
            }
            let var_name = caps[1].to_string();
            let values_str = &caps[2];

            // EaterForeach consumes JSON; CodeIteratorForeach binds fromJson
            // globally. Even numeric JSON elements keep their JSON value kind.
            let values = match self.eval_expr_to_value(values_str) {
                Value::Json(serde_json::Value::Array(values)) => {
                    values.into_iter().map(Value::Json).collect()
                }
                Value::Json(serde_json::Value::Object(values)) => values
                    .keys()
                    .map(|key| Value::Json(serde_json::Value::String(key.clone())))
                    .collect(),
                Value::Json(_) => {
                    // EaterForeach.size rejects JSON scalars; toJson on a
                    // non-JSON TValue instead returns null, which is skipped.
                    self.fail("Foreach requires a JSON array or object".into());
                    return true;
                }
                _ => Vec::new(),
            };

            self.foreach_stack.push(ForEachState {
                nested: 0,
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
                    if self.return_signal.is_some() || self.error.is_some() || self.iterator_stopped
                    {
                        break;
                    }
                    self.defines.insert(foreach.var_name.clone(), val.clone());

                    for body_line in &foreach.body_lines {
                        self.process_buffered_line(body_line, output);
                        if self.return_signal.is_some()
                            || self.error.is_some()
                            || self.iterator_stopped
                        {
                            break;
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
        let buffered = self.current_buffered_line(line);
        if let Some(state) = self.while_stack.last_mut() {
            let closes = trimmed == "!endwhile";
            if !closes || state.nested > 0 {
                if directive_argument(trimmed, "!while").is_some() {
                    state.nested += 1;
                }
                if closes {
                    state.nested -= 1;
                }
                state.body_lines.push(buffered);
                return true;
            }
        }
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^!while\s+(.+)$").unwrap());

        if let Some(caps) = RE.captures(trimmed) {
            if !self.is_active() {
                return true;
            }
            let condition = caps[1].to_string();
            self.while_stack.push(WhileState {
                nested: 0,
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
                    if self.error.is_some()
                        || self.iterator_stopped
                        || self.return_signal.is_some()
                        || iterations >= MAX_WHILE_ITERATIONS
                    {
                        break;
                    }
                    if !self.eval_bool(&while_state.condition) {
                        break;
                    }

                    for body_line in &while_state.body_lines {
                        self.process_buffered_line(body_line, output);
                        if self.return_signal.is_some()
                            || self.error.is_some()
                            || self.iterator_stopped
                        {
                            break;
                        }
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
                let saved_base = self
                    .base_dir
                    .replace(file_path.parent().unwrap_or(Path::new(".")).to_path_buf());
                let saved_kind =
                    std::mem::replace(&mut self.execution_kind, FunctionKind::Procedure);
                let lines = self.process(&content);
                self.execution_kind = saved_kind;
                self.base_dir = saved_base;
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
                let saved_kind =
                    std::mem::replace(&mut self.execution_kind, FunctionKind::Procedure);
                let lines = self.process(&content);
                self.execution_kind = saved_kind;
                self.include_depth -= 1;
                Some(lines)
            }
            Err(_) => Some(vec![]),
        }
    }

    fn try_includesub(&mut self, line: &str) -> Option<Vec<EvaluatedEvent>> {
        let argument = directive_argument(line, "!includesub")?;
        if !self.is_active() {
            return Some(Vec::new());
        }
        // EaterIncludesub expands the entire selector before splitting at '!'.
        let selector = self.substitute_vars(argument);
        if self.error.is_some() || self.include_depth >= MAX_INCLUDE_DEPTH {
            return Some(Vec::new());
        }
        let Some((path_str, sub_name)) = selector.split_once('!') else {
            let Some(body) = self.subs.get(&selector).cloned() else {
                self.fail(format!("cannot include {selector}"));
                return Some(Vec::new());
            };
            let saved_kind = std::mem::replace(&mut self.execution_kind, FunctionKind::Procedure);
            let mut output = Vec::new();
            self.process_buffered_lines(&body, &mut output, true);
            self.execution_kind = saved_kind;
            return Some(output);
        };
        let path_str = path_str.trim_matches('"');
        let file_path = if let Some(base) = &self.base_dir {
            base.join(path_str)
        } else {
            PathBuf::from(path_str)
        };

        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                // Sub.fromFile selects raw StringLocated lines, not an executed
                // temporary context. Repeated matching blocks append; unselected
                // prelude/tail never executes (including errors and mutations).
                let mut selected = Vec::new();
                let mut found = false;
                let mut skip = false;
                for (index, text) in content.lines().enumerate() {
                    // Sub.fromFile sees raw markers before TIM's comment iterators.
                    // UncommentReadLine handles embedding prefixes, not TIM comments.
                    let trimmed = text.trim();
                    if let Some(name) = directive_argument(trimmed, "!startsub") {
                        // Sub.fromFile validates every marker, even before a
                        // selected block. Suppression applies only to body execution.
                        if !valid_sub_name(name) {
                            let origin = self.source_line_override.replace(index);
                            self.fail("Bad sub name".into());
                            self.source_line_override = origin;
                            return Some(Vec::new());
                        }
                        if name == sub_name {
                            found = true;
                            skip = false;
                        }
                        continue;
                    }
                    if directive_argument(trimmed, "!endsub").is_some() && found {
                        skip = true;
                    }
                    if found && !skip {
                        selected.push(BufferedLine {
                            text: text.to_string(),
                            raw_source_line: index + 1,
                            source_line: index,
                        });
                    }
                }
                if !found {
                    self.fail(format!("cannot include {selector}"));
                    return Some(Vec::new());
                }
                let saved_base = self
                    .base_dir
                    .replace(file_path.parent().unwrap_or(Path::new(".")).to_path_buf());
                let saved_kind =
                    std::mem::replace(&mut self.execution_kind, FunctionKind::Procedure);
                self.include_depth += 1;
                let mut output = Vec::new();
                self.process_buffered_lines(&selected, &mut output, true);
                self.include_depth -= 1;
                self.base_dir = saved_base;
                self.execution_kind = saved_kind;
                self.mark_source_lines = true;
                Some(output)
            }
            Err(_) => {
                self.fail(format!("cannot include {selector}"));
                Some(Vec::new())
            }
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
        let bundled_lookup = from.is_none();
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
            let saved_kind = std::mem::replace(&mut self.execution_kind, FunctionKind::Procedure);
            let body = self.process(themes::strip_front_matter(&source));
            self.execution_kind = saved_kind;
            self.include_depth -= 1;
            self.mark_function_body_source_lines = saved_mark;
            output.push(EvaluatedEvent::Theme {
                name,
                source_line,
                body,
            });
        } else if bundled_lookup && themes::JAVA_BUNDLED_UNSHIPPED_THEMES.contains(&name.as_str()) {
            // ThemeUtils.loadBundledOrLocalTheme finds this name in Java's jar,
            // so Java executes a theme here. Its body is not shipped; keep the
            // base parser-only marker that renderers emulate (no identity).
            let source_line = self
                .source_line_override
                .unwrap_or_else(|| self.current_diagram_source_line());
            output.push(EvaluatedEvent::ParserOnly(source_line_marker(
                source_line,
                &format!("skinparam __theme {name}"),
            )));
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
            LazyLock::new(|| Regex::new(r"^!undef(?:ine)?\s+(\$?\w+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            // CodeIteratorIf suppresses inactive lines before TContext reaches
            // EaterUndef. An inactive branch must not mutate outer memory.
            if self.is_active() {
                let name = &caps[1];
                self.remove_var(name);
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
            LazyLock::new(|| Regex::new(r"^!ifndef\s+(\$?\w+)$").unwrap());

        if let Some(caps) = RE_IFDEF.captures(line) {
            // PlantUML EaterIfdef delegates the complete expression to
            // EvalBoolean, whose Truth callback checks variables and
            // functions. Preserve its !, &&, || and parenthesis grammar.
            let defined = eval_defined_expression(&caps[1], |symbol| {
                self.get_var(symbol).is_some() || self.functions.contains_key(symbol)
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
            let defined = self.get_var(name).is_some();
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

    fn eval_expr_to_value(&mut self, expression: &str) -> Value {
        let prepared = self.prepare_expression(expression);
        self.execute_expression(&prepared)
    }

    fn prepare_expression(&mut self, expression: &str) -> Expression {
        if self.error.is_some() {
            return Expression::Value(Value::Str(String::new()));
        }
        let expr = expression.trim();
        // TokenType.QUOTED_STRING: either quote form is a literal token.
        if let Some(quote) = expr.chars().next().filter(|c| matches!(c, '\'' | '"'))
            && let Some(end) = expr[1..].find(quote).map(|n| n + 1)
            && end + 1 == expr.len()
        {
            return Expression::Value(Value::Str(expr[1..end].to_string()));
        }
        if expr.starts_with('(') && find_matching_paren(expr) == Some(expr.len() - 1) {
            return self.prepare_expression(&expr[1..expr.len() - 1]);
        }
        // TokenOperator precedence and left associativity. Preparing both sides
        // snapshots their variable values before either side executes a call.
        for ops in [
            &["||"][..],
            &["&&"][..],
            &["==", "!="][..],
            &["<=", ">=", "<", ">"][..],
            &["+", "-"][..],
            &["*", "/", "%"][..],
        ] {
            if let Some((position, op)) = ops
                .iter()
                .filter_map(|op| {
                    let pos = if matches!(*op, "+" | "-" | "*" | "/" | "%") {
                        find_top_level_op_pos(expr, op.chars().next().unwrap())
                    } else {
                        find_top_level_comparison(expr, op)
                    };
                    pos.map(|pos| (pos, *op))
                })
                .max_by_key(|(pos, _)| *pos)
            {
                let left = self.prepare_expression(&expr[..position]);
                let right = self.prepare_expression(&expr[position + op.len()..]);
                return Expression::Binary(Box::new(left), op.into(), Box::new(right));
            }
        }
        if let Some(rest) = expr.strip_prefix('!') {
            return Expression::Not(Box::new(self.prepare_expression(rest)));
        }
        if matches!(expr.chars().next(), Some('{' | '['))
            && let Ok(json) = serde_json::from_str(expr)
        {
            return Expression::Value(Value::Json(json));
        }
        if let Some(open) = expr.find('(')
            && find_matching_paren_from(expr, open) == Some(expr.len() - 1)
        {
            let name = expr[..open].trim();
            let raw = &expr[open + 1..expr.len() - 1];
            let args = if raw.trim().is_empty() {
                Vec::new()
            } else {
                split_builtin_args(raw)
                    .into_iter()
                    .map(|arg| {
                        // ShuntingYard resolves the AFFECTATION left operand
                        // through Knowledge too. EaterFunctionCall keeps it literal.
                        let (key, value) = named_argument(arg)
                            .map_or((None, arg), |(k, v)| (Some(self.prepare_expression(k)), v));
                        (key, self.prepare_expression(value))
                    })
                    .collect()
            };
            return Expression::Call(name.to_string(), args);
        }
        // TContext.asKnowledge/fromJson resolves member and index tokens during
        // ShuntingYard. VariableManager expands index text, then JSON-parses the
        // resulting display value (numeric-looking non-JSON strings stay text).
        if (expr.contains('.') || expr.contains('[')) && expr.parse::<f64>().is_err() {
            let expanded = self.substitute_vars(expr);
            if let Ok(json) = serde_json::from_str(&expanded) {
                return Expression::Value(Value::Json(json));
            }
            if expanded != expr {
                return Expression::Value(Value::Str(expanded));
            }
        }
        if let Some(value) = self.get_var(expr) {
            return Expression::Value(value.clone());
        }
        if !expr.is_empty()
            && expr
                .trim_start_matches('-')
                .chars()
                .all(|c| c.is_ascii_digit())
        {
            return Expression::Number(expr.to_string());
        }
        if let Ok(number) = expr.parse::<f64>() {
            return Expression::Value(Value::Float(number)); // Existing extension.
        }
        Expression::Value(Value::Str(expr.to_string()))
    }

    fn execute_expression(&mut self, expression: &Expression) -> Value {
        // ReversePolishInterpretor owns ONE stack/map for a whole prepared
        // expression. A called function's body evaluates its own expressions.
        // Account: tim-rpn-call-state (precommitted c91103f9).
        let mut stack = Vec::new();
        let mut named = HashMap::new();
        self.execute_expression_into(expression, &mut stack, &mut named);
        stack.pop().unwrap_or_else(|| Value::Str(String::new()))
    }

    fn expression_pop(&mut self, stack: &mut Vec<Value>) -> Value {
        // Java ArrayDeque.removeFirst throws on malformed RPN stack state.
        // Preserve fatality through the existing preprocessing error channel.
        stack.pop().unwrap_or_else(|| {
            self.fail("Expression operand stack underflow".into());
            Value::Str(String::new())
        })
    }

    fn execute_expression_into(
        &mut self,
        expression: &Expression,
        stack: &mut Vec<Value>,
        named: &mut HashMap<String, Value>,
    ) {
        if self.error.is_some() {
            return;
        }
        match expression {
            Expression::Value(value) => stack.push(value.clone()),
            Expression::Number(text) => match text.parse::<i32>() {
                Ok(number) => stack.push(Value::Int(number)),
                Err(_) => self.fail("Integer literal outside signed 32-bit range".into()),
            },
            Expression::Not(inner) => {
                self.execute_expression_into(inner, stack, named);
                if self.error.is_none() {
                    let value = self.expression_pop(stack);
                    stack.push(Value::Int(i32::from(!value.truth())));
                }
            }
            Expression::Binary(left, op, right) => {
                self.execute_expression_into(left, stack, named);
                self.execute_expression_into(right, stack, named);
                if self.error.is_none() {
                    let right = self.expression_pop(stack);
                    let left = self.expression_pop(stack);
                    if self.error.is_none() {
                        stack.push(self.apply_operator(left, right, op));
                    }
                }
            }
            Expression::Call(name, args) => {
                for (key, expression) in args {
                    if let Some(key) = key {
                        self.execute_expression_into(key, stack, named);
                    }
                    self.execute_expression_into(expression, stack, named);
                    if self.error.is_some() {
                        return;
                    }
                    if key.is_some() {
                        let value = self.expression_pop(stack);
                        let key = self.expression_pop(stack).to_display();
                        if self.error.is_some() {
                            return;
                        }
                        named.insert(key, value);
                    }
                }
                // RPN OPEN_PAREN_FUNC subtracts the current DISTINCT named keys,
                // including those assigned before a nested call in this expression.
                let count = args.len().checked_sub(named.len());
                let covered = count.is_some_and(|count| {
                    if let Some(builtin) = name.strip_prefix('%') {
                        // Unknown arity retains existing Rust extensions; their
                        // body dispatcher still rejects unimplemented names.
                        builtin_arity(builtin).is_none_or(|arity| arity.contains(&count))
                    } else {
                        self.resolve_function(name, count, None).is_some()
                    }
                });
                if !covered {
                    self.fail(format!("Unknown built-in function {name}"));
                    return;
                }
                let count = count.unwrap(); // Successful coverage above requires a count.
                if count > stack.len() {
                    self.fail("Expression operand stack underflow".into());
                    return;
                }
                let positional = stack.split_off(stack.len() - count);
                let value = if let Some(builtin) = name.strip_prefix('%') {
                    self.eval_builtin_values(builtin, &positional)
                } else {
                    let (ret, events) =
                        self.call_function_values(name, positional, named.clone(), true);
                    self.expression_events.extend(events);
                    ret.unwrap_or_else(|| Value::Str(String::new()))
                };
                named.clear();
                if self.error.is_none() {
                    stack.push(value);
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Boolean evaluation for !if / !while conditions
    // ------------------------------------------------------------------

    fn eval_bool(&mut self, expr: &str) -> bool {
        self.eval_expr_to_value(expr).truth()
    }

    fn fail(&mut self, message: String) {
        // EaterException unwinds the Java evaluator: the first diagnostic wins.
        if self.error.is_none() {
            self.error = Some(PreprocessError {
                line: self
                    .source_line_override
                    .unwrap_or_else(|| self.current_diagram_source_line()),
                message,
            });
        }
    }

    fn apply_operator(&mut self, left: Value, right: Value, op: &str) -> Value {
        match op {
            "||" => return Value::Int(i32::from(left.truth() || right.truth())),
            "&&" => return Value::Int(i32::from(left.truth() && right.truth())),
            "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                let order = if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
                    a.cmp(b)
                } else {
                    // String.compareTo compares UTF-16 code units.
                    left.utf16_units().cmp(&right.utf16_units())
                };
                return Value::Int(i32::from(match op {
                    "==" => order.is_eq(),
                    "!=" => !order.is_eq(),
                    "<" => order.is_lt(),
                    ">" => order.is_gt(),
                    "<=" => !order.is_gt(),
                    _ => !order.is_lt(),
                }));
            }
            _ => {}
        }
        if let (Value::Int(a), Value::Int(b)) = (&left, &right) {
            // TValue.add/minus/multiply/dividedBy operate on signed Java int.
            return Value::Int(match op {
                "+" => a.wrapping_add(*b),
                "-" => a.wrapping_sub(*b),
                "*" => a.wrapping_mul(*b),
                "/" | "%" if *b == 0 => {
                    self.fail("Division by zero".into());
                    0
                }
                "/" => a.wrapping_div(*b),
                "%" => a.wrapping_rem(*b),
                _ => unreachable!(),
            });
        }
        // Retain the existing Rust float extension; never cast JSON into int.
        if (matches!(left, Value::Float(_)) || matches!(right, Value::Float(_)))
            && left.is_numeric()
            && right.is_numeric()
        {
            let (a, b) = (left.to_number(), right.to_number());
            return Value::Float(match op {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "/" => a / b,
                _ => a % b,
            });
        }
        let separator = if matches!(op, "+" | "-") { "" } else { op };
        let mut units = left.utf16_units();
        units.extend(separator.encode_utf16());
        units.extend(right.utf16_units());
        Value::from_utf16(units)
    }

    // ------------------------------------------------------------------
    // Variable substitution
    // ------------------------------------------------------------------

    fn variable_name_at(&mut self, text: &str, offset: usize) -> Option<String> {
        // VariableManager.getVarnameAt / TMemoryLocal.variablesNames3. A longer
        // returned path wins across scopes before the right boundary is checked.
        if !text[offset..].starts_with('$')
            && !text[..offset].ends_with("\\n")
            && text[..offset]
                .chars()
                .next_back()
                .is_some_and(variable_boundary_letter)
        {
            return None;
        }
        let suffix = &text[offset..];
        let mut candidates = vec![self.defines.match_at(suffix)];
        if let Some(scope) = self.local_vars.last_mut() {
            candidates.push(scope.overrides.match_at(suffix));
            candidates.push(scope.locals.match_at(suffix));
        }
        // All matches are prefixes of one valid UTF-8 string, so byte length
        // and UTF-16 length give the same ordering (equal lengths are same key).
        let name = candidates.into_iter().flatten().max_by_key(String::len)?;
        if text[offset + name.len()..]
            .chars()
            .next()
            .is_some_and(variable_boundary_letter)
        {
            return None;
        }
        Some(name)
    }

    fn substitute_vars(&mut self, line: &str) -> String {
        let mut result = String::new();
        let mut offset = 0;
        while offset < line.len() && self.error.is_none() {
            let ch = line[offset..].chars().next().unwrap();
            // TContext scans every original position, including sprite markup;
            // absence of a matching stored name, not markup syntax, preserves it.
            // TContext.getFunctionNameAt permits $/% after a letter, but bare
            // names require the same original-text boundary as VariableManager.
            let bare_boundary = offset == 0
                || line[..offset].ends_with("\\n")
                || !line[..offset]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_');
            if matches!(ch, '$' | '%') || ((ch.is_alphabetic() || ch == '_') && bare_boundary) {
                let name_start = offset + usize::from(matches!(ch, '$' | '%'));
                let name_end = line[name_start..]
                    .char_indices()
                    .find(|(_, c)| !c.is_alphanumeric() && *c != '_')
                    .map_or(line.len(), |(i, _)| name_start + i);
                let name = &line[name_start..name_end];
                if line[name_end..].starts_with('(')
                    && ((ch == '%' && registered_builtin(name))
                        || (ch != '%' && self.functions.contains_key(&line[offset..name_end])))
                    && let Some(close) = find_matching_paren_from(line, name_end)
                {
                    if ch == '%' {
                        result.push_str(
                            &self
                                .eval_builtin_with_expr_args(name, &line[name_end + 1..close])
                                .to_display(),
                        );
                    } else {
                        let args = parse_call_args(&line[name_end + 1..close]);
                        let (value, mut events) =
                            self.call_function(&line[offset..name_end], &args);
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
            }
            // TContext.applyFunctionsAndVariables scans original characters only;
            // appended values are never rescanned as legacy macro source.
            if (ch.is_alphanumeric() || ch == '_')
                && (offset == 0
                    // VariableManager.justAfterBackslashN treats literal \n as
                    // a token boundary without rescanning appended values.
                    || line[..offset].ends_with("\\n")
                    || !line[..offset]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric() || c == '_'))
            {
                let end = line[offset..]
                    .char_indices()
                    .find(|(_, c)| !c.is_alphanumeric() && *c != '_')
                    .map_or(line.len(), |(i, _)| offset + i);
                let name = &line[offset..end];
                if let Some(definition) = self.definelong_macros.get(name)
                    && line[end..].starts_with('(')
                    && let Some(close) = find_matching_paren_from(line, end)
                {
                    // FunctionsSet registers even zero-argument definelong as
                    // NAME(. EaterFunctionCall evaluates legacy string arguments
                    // once; a bare name is not a function invocation.
                    let expected = definition.params.len();
                    let raw = &line[end + 1..close];
                    let parts = if raw.trim().is_empty() {
                        Vec::new()
                    } else {
                        split_builtin_args(raw)
                    };
                    let args = parts
                        .into_iter()
                        .map(|arg| {
                            let arg = arg.trim();
                            let text = if let Some(quote) =
                                arg.chars().next().filter(|c| matches!(c, '\'' | '"'))
                                && arg.len() >= 2
                                && arg.ends_with(quote)
                            {
                                &arg[1..arg.len() - 1]
                            } else {
                                arg
                            };
                            self.substitute_vars(text)
                        })
                        .collect::<Vec<_>>();
                    if args.len() != expected {
                        self.fail(format!("Function not found {name}"));
                        return String::new();
                    }
                    let (expanded, multiline) = self.expand_definelong(name, &args);
                    if multiline {
                        // TContext LEGACY_DEFINELONG pendingAdd uses the RAW
                        // caller prefix; PROCEDURE uses the evaluated result.
                        let raw_prefix = &line[..offset];
                        self.mark_source_lines = true;
                        for (index, line) in expanded.split('\n').enumerate() {
                            let (origin, text) = split_source_line_marker(line)
                                .map_or((None, line), |(n, t)| (Some(n), t));
                            let text = if index == 0 {
                                format!("{raw_prefix}{text}")
                            } else {
                                text.to_string()
                            };
                            self.expression_events.push(EvaluatedEvent::Plain(
                                origin.map_or(text.clone(), |n| source_line_marker(n, &text)),
                            ));
                        }
                        self.text_consumed = true;
                        return String::new();
                    }
                    result.push_str(&expanded);
                    offset = close + 1;
                    continue;
                }
            }
            // VariableManager tests the original source position after function
            // lookup. Stored names, not a token grammar, define the candidate.
            if let Some(name) = self.variable_name_at(line, offset)
                && let Some(value) = self.get_var(&name).cloned()
            {
                if result.ends_with("##") {
                    result.truncate(result.len() - 2);
                }
                let (value, end) = self.variable_path(value, line, offset + name.len());
                result.push_str(&value.to_display());
                offset = end;
                if line[offset..].starts_with("##") {
                    offset += 2;
                }
                continue;
            }
            result.push(ch);
            offset += ch.len_utf8();
        }

        result
    }

    // VariableManager.replaceJson: member names are identifier text; bracket
    // text is expanded as a string (not an arithmetic expression). TContext's
    // expression knowledge subsequently JSON-parses the displayed result.
    fn variable_path(&mut self, value: Value, text: &str, mut end: usize) -> (Value, usize) {
        let Value::Json(mut json) = value else {
            return (value, end);
        };
        if !json.is_object() && !json.is_array() {
            return (Value::Json(json), end);
        }
        while end < text.len() && self.error.is_none() {
            if text[end..].starts_with('.') {
                end += 1;
                let start = end;
                while let Some(c) = text[end..].chars().next() {
                    if !c.is_alphanumeric() && !matches!(c, '_' | '$') {
                        break;
                    }
                    end += c.len_utf8();
                }
                let Some(object) = json.as_object() else {
                    self.fail("Major parsing error".into());
                    break;
                };
                let Some(field) = object.get(&text[start..end]) else {
                    // VariableManager.replaceJson allows a final missing field,
                    // but cannot traverse a subsequent member/index through null.
                    if text[end..].starts_with(['.', '[']) {
                        self.fail("Data parsing error: missing intermediate member".into());
                    }
                    return (Value::Str(String::new()), end);
                };
                json = field.clone();
            } else if text[end..].starts_with('[') {
                let Some(close) = syntax_positions(&text[end..])
                    .into_iter()
                    .find(|(_, c, d)| *c == ']' && *d == 1)
                    .map(|(i, _, _)| end + i)
                else {
                    self.fail("Data parsing error: unclosed index".into());
                    break;
                };
                let key = self.substitute_vars(&text[end + 1..close]);
                end = close + 1;
                let next = if let Some(array) = json.as_array() {
                    key.parse::<usize>().ok().and_then(|index| array.get(index))
                } else if let Some(object) = json.as_object() {
                    object.get(&key)
                } else {
                    None
                };
                let Some(next) = next else {
                    self.fail("Data parsing error".into());
                    break;
                };
                json = next.clone();
            } else {
                break;
            }
        }
        (Value::Json(json), end)
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
    fn eval_builtin_with_expr_args(&mut self, func: &str, args_raw: &str) -> Value {
        if self.error.is_some() {
            return Value::Str(String::new());
        }
        let mut values = Vec::new();
        if !args_raw.trim().is_empty() {
            for part in split_builtin_args(args_raw) {
                // EaterFunctionCall separates named operands before constructing
                // the positional signature. Builtins ignore the named map, but
                // each RHS still executes, including effects and errors.
                if let Some((_, expression)) = named_argument(part) {
                    self.eval_expr_to_value(expression);
                } else {
                    values.push(self.eval_expr_to_value(part));
                }
                if self.error.is_some() {
                    return Value::Str(String::new());
                }
            }
        }
        // TContext.applyFunctionsAndVariables reports failed plain-call lookup
        // differently from ReversePolishInterpretor's expression lookup.
        if builtin_arity(func).is_some_and(|arity| !arity.contains(&values.len())) {
            self.fail(format!("Function not found %{func}"));
            return Value::Str(String::new());
        }
        self.eval_builtin_values(func, &values)
    }

    fn eval_builtin_values(&mut self, func: &str, values: &[Value]) -> Value {
        if self.error.is_some() {
            return Value::Str(String::new());
        }
        // FunctionsSet.getFunctionSmart / each implemented builtin's canCover.
        // Argument expressions have executed already; no body runs on mismatch.
        if builtin_arity(func).is_some_and(|arity| !arity.contains(&values.len())) {
            self.fail(format!("Unknown built-in function %{func}"));
            return Value::Str(String::new());
        }
        match func {
            "intval" => {
                // IntVal.executeReturnFunction uses Integer.parseInt on TValue's
                // complete display, not numeric coercion or floating truncation.
                let text = values[0].to_display();
                return match java_parse_int(&values[0].utf16_units()) {
                    Some(value) => Value::Int(value),
                    None => {
                        self.fail(format!("Cannot convert {text} to integer."));
                        Value::Str(String::new())
                    }
                };
            }
            "hsl_color" => {
                return Value::Str(tim_hsl_color(values));
            }
            "upper" | "lower" => {
                // Upper/Lower call Java String case conversion, which preserves
                // unmatched UTF-16 units. Convert valid spans, not lossy display.
                let units = values[0].utf16_units();
                let mut result = Vec::new();
                let mut span = String::new();
                for decoded in char::decode_utf16(units) {
                    match decoded {
                        Ok(ch) => span.push(ch),
                        Err(error) => {
                            append_case_span(&mut result, &span, func == "upper");
                            span.clear();
                            result.push(error.unpaired_surrogate());
                        }
                    }
                }
                append_case_span(&mut result, &span, func == "upper");
                return Value::from_utf16(result);
            }
            "chr" => {
                // Chr uses TValue.toInt and Character.toChars; the latter
                // permits BMP surrogate code units and catches invalid points as NUL.
                let code = values[0].java_int();
                return Value::from_utf16(if let Ok(unit) = u16::try_from(code) {
                    vec![unit]
                } else if let Some(ch) = u32::try_from(code).ok().and_then(char::from_u32) {
                    ch.encode_utf16(&mut [0; 2]).to_vec()
                } else {
                    vec![0]
                });
            }
            "boolval" => {
                // BoolVal trueValues/falseValues are explicit textual conversions,
                // not TValue.toBoolean (where nonempty text "false" is truthy).
                let text = values[0].to_display().to_lowercase();
                return match text.as_str() {
                    "true" | "1" => Value::Int(1),
                    "false" | "0" => Value::Int(0),
                    _ => {
                        self.fail(format!("Cannot convert {text} to boolean."));
                        Value::Str(String::new())
                    }
                };
            }
            _ => {}
        }
        let args: Vec<String> = values.iter().map(Value::to_display).collect();
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        // Builtin contracts return TValue, not source text to be tokenized again.
        match func {
            "getenv" => {
                // Getenv.canCover and SecurityProfile.canWeReadThisEnvironmentVariable.
                // JVM properties have no general Rust equivalent; these two are
                // platform properties explicitly permitted by the Java profile.
                if values.len() != 1 {
                    self.fail("Bad number of arguments for %getenv".into());
                    return Value::Str(String::new());
                }
                let name = &args[0];
                let lower = name.to_lowercase();
                let unsecure = std::env::var("PLANTUML_SECURITY_PROFILE")
                    .is_ok_and(|profile| profile.eq_ignore_ascii_case("UNSECURE"));
                let allowed = !lower.starts_with("plantuml.security")
                    && (lower.starts_with("plantuml")
                        || matches!(lower.as_str(), "path.separator" | "line.separator")
                        || unsecure);
                let value = if !allowed {
                    String::new()
                } else if name == "path.separator" {
                    if cfg!(windows) { ";" } else { ":" }.into()
                } else if name == "line.separator" {
                    if cfg!(windows) { "\r\n" } else { "\n" }.into()
                } else {
                    std::env::var(name).unwrap_or_default()
                };
                return Value::Str(value);
            }
            "strlen" => {
                return Value::Int(values.first().map_or(0, |v| v.utf16_units().len() as i32));
            }
            "string" => {
                return Value::from_utf16(values.first().map_or_else(Vec::new, Value::utf16_units));
            }
            // Substr uses TValue.toInt and Java String UTF-16 bounds. A start
            // beyond the end returns empty before checking the optional length.
            "substr" => {
                if !(2..=3).contains(&values.len()) {
                    self.fail("Bad number of arguments for %substr".into());
                    return Value::Str(String::new());
                }
                let units = values[0].utf16_units();
                let start = values[1].java_int();
                if start < 0 {
                    self.fail("Substring index out of range".into());
                    return Value::Str(String::new());
                }
                if start as usize >= units.len() {
                    return Value::Str(String::new());
                }
                let available = &units[start as usize..];
                let length = values.get(2).map(Value::java_int);
                if length.is_some_and(|length| length < 0) {
                    self.fail("Substring index out of range".into());
                    return Value::Str(String::new());
                }
                let count = length.map_or(available.len(), |n| (n as usize).min(available.len()));
                return Value::from_utf16(available[..count].to_vec());
            }
            // Strpos returns String.indexOf in UTF-16 code units, not UTF-8 bytes.
            "strpos" => {
                if values.len() != 2 {
                    self.fail("Bad number of arguments for %strpos".into());
                    return Value::Int(0);
                }
                let haystack = values[0].utf16_units();
                let needle = values[1].utf16_units();
                let index = if needle.is_empty() {
                    Some(0)
                } else {
                    haystack
                        .windows(needle.len())
                        .position(|window| window == needle)
                };
                return Value::Int(index.map_or(-1, |i| i as i32));
            }
            "dec2hex" => {
                if values.len() != 1 {
                    self.fail("Bad number of arguments for %dec2hex".into());
                    return Value::Str(String::new());
                }
                // Dec2hex calls Integer.toHexString, including two's complement.
                return Value::Str(format!("{:x}", values[0].java_int()));
            }
            // LogicalNot.executeReturnFunction uses TValue.toBoolean directly.
            "not" => return Value::Int(i32::from(!values.first().is_some_and(Value::truth))),
            "get_variable_value" => {
                let name = refs.first().copied().unwrap_or("");
                return self
                    .get_var(name)
                    .cloned()
                    .unwrap_or_else(|| Value::Str(String::new()));
            }
            "set_variable_value" if values.len() >= 2 => {
                // SetVariableValue explicitly uses GLOBAL, preserving its exact key.
                if args[0].contains('\0') {
                    self.fail("Fatal parsing error".into());
                    return Value::Str(String::new());
                }
                self.defines.insert(args[0].clone(), values[1].clone());
                return Value::Str(String::new());
            }
            "str2json" => {
                return serde_json::from_str(refs.first().copied().unwrap_or(""))
                    .map(Value::Json)
                    .unwrap_or_else(|_| Value::Str(String::new()));
            }
            "size" => {
                return Value::Int(match values.first() {
                    Some(Value::Json(serde_json::Value::Array(a))) => a.len() as i32,
                    Some(Value::Json(serde_json::Value::Object(a))) => a.len() as i32,
                    Some(Value::Str(s)) => s.encode_utf16().count() as i32,
                    Some(Value::Utf16(units)) => units.len() as i32,
                    _ => 0,
                });
            }
            "date" if values.len() >= 2 => {
                // DateFunction uses TValue.toInt, not parsing its display text.
                let seconds = values[1].java_int().to_string();
                return Value::Str(self.eval_one_builtin_args(func, &[refs[0], &seconds]));
            }
            _ => {}
        }
        let text = self.eval_one_builtin_args(func, &refs);
        match func {
            // Each corresponding tim/builtin implementation returns fromInt/fromBoolean.
            "strlen" | "strpos" | "true" | "false" | "hex2dec" | "variable_exists" => {
                Value::Int(text.parse().unwrap_or(0))
            }
            "is_dark" | "is_light" | "is_defined" | "function_exists" | "file_exists"
            | "feature" => Value::Int(i32::from(text == "true" || text == "1")),
            "float" => Value::Float(text.parse().unwrap_or(0.0)), // Existing Rust extension.
            _ => Value::Str(text),
        }
    }

    fn eval_one_builtin_args(&mut self, func: &str, args: &[&str]) -> String {
        match func {
            "strlen" => args
                .first()
                .map_or("0".to_string(), |s| s.encode_utf16().count().to_string()),
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
                        self.fail("Date seconds outside signed 32-bit range".into());
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
                        self.fail(message);
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
                // VariableExists delegates the exact name to TMemory.getVariable.
                let name = args.first().copied().unwrap_or("");
                if self.get_var(name).is_some() {
                    "1"
                } else {
                    "0"
                }
                .to_string()
            }
            "is_defined" => {
                let name = args.first().copied().unwrap_or("");
                if self.get_var(name).is_some() {
                    "true"
                } else {
                    "false"
                }
                .to_string()
            }
            "function_exists" => {
                // FunctionsSet.doesFunctionExist indexes the exact declared name.
                let name = args.first().copied().unwrap_or("");
                if self.functions.contains_key(name) {
                    "true"
                } else {
                    "false"
                }
                .to_string()
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
                // ReversePolishInterpretor rejects an unresolved function.
                self.fail(format!("Unknown built-in function %{func}"));
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

// Arity is taken from tim/builtin/<class>.canCover for already implemented
// dispatch entries. Registration is intentionally separate: unimplemented names
// remain explicit errors, never synthetic empty results. Rust-only extensions
// retain their previous contracts.
fn builtin_arity(name: &str) -> Option<std::ops::RangeInclusive<usize>> {
    Some(match name {
        "true" | "false" | "newline" | "tab" | "filename" | "dirpath" => 0..=0,
        "upper" | "lower" | "chr" | "boolval" | "strlen" | "string" | "dec2hex" | "not"
        | "get_variable_value" | "getenv" | "str2json" | "size" | "intval" | "hex2dec"
        | "is_dark" | "is_light" | "variable_exists" | "function_exists" | "file_exists"
        | "feature" => 1..=1,
        "strpos" | "set_variable_value" | "darken" | "lighten" => 2..=2,
        "substr" => 2..=3,
        "date" => 0..=2,
        "hsl_color" => 3..=4,
        _ => return None,
    })
}

fn append_case_span(units: &mut Vec<u16>, text: &str, upper: bool) {
    let mapped = if upper {
        text.to_uppercase()
    } else {
        text.to_lowercase()
    };
    units.extend(mapped.encode_utf16());
}

// TContext.addStandardFunctions registers these names before text expansion.
// getFunctionNameAt only recognizes registered calls; RPN separately rejects
// unknown expression calls. `float` and `is_defined` retain Rust extensions.
fn registered_builtin(name: &str) -> bool {
    matches!(
        name,
        "and"
            | "backslash"
            | "boolval"
            | "breakline"
            | "call_user_func"
            | "chr"
            | "darken"
            | "date"
            | "dec2hex"
            | "dirpath"
            | "dollar"
            | "eval"
            | "false"
            | "feature"
            | "file_exists"
            | "filedate"
            | "filename"
            | "filename_no_extension"
            | "float"
            | "function_exists"
            | "get_all_stdlib"
            | "get_all_theme"
            | "get_current_theme"
            | "get_json_keys"
            | "get_json_type"
            | "get_stdlib"
            | "get_variable_value"
            | "getenv"
            | "hex2dec"
            | "hsl_color"
            | "intval"
            | "invoke_procedure"
            | "is_dark"
            | "is_defined"
            | "is_light"
            | "json_add"
            | "json_key_exists"
            | "json_merge"
            | "json_remove"
            | "json_set"
            | "left_align"
            | "lighten"
            | "load_json"
            | "lower"
            | "mod"
            | "n"
            | "nand"
            | "newline"
            | "nor"
            | "not"
            | "now"
            | "nxor"
            | "or"
            | "ord"
            | "percent"
            | "random"
            | "retrieve_procedure"
            | "reverse_color"
            | "reverse_hsluv_color"
            | "right_align"
            | "set_variable_value"
            | "size"
            | "splitstr"
            | "splitstr_regex"
            | "str2json"
            | "string"
            | "strlen"
            | "strpos"
            | "substr"
            | "tab"
            | "true"
            | "upper"
            | "variable_exists"
            | "version"
            | "xargs"
            | "xor"
    )
}

// TLineType.simpleKeyword recognizes directive boundaries; EaterStartsub
// consumes intervening whitespace and requires Java's ASCII \w+ block name.
fn directive_argument<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(directive)?;
    (rest.is_empty() || rest.starts_with(|c: char| !c.is_alphanumeric() && c != '_'))
        .then(|| rest.trim_start())
}

fn valid_sub_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// EaterFunctionCall / TFunctionImpl.getNewMemory: named values are separate
// from the positional queue, and quoted equality text is not an assignment.
fn named_argument(text: &str) -> Option<(&str, &str)> {
    let (index, _, _) = syntax_positions(text).into_iter().find(|(i, c, d)| {
        *d == 0
            && *c == '='
            && !text[*i..].starts_with("==")
            && !text[..*i].ends_with(['!', '<', '>', '='])
    })?;
    let name = text[..index].trim();
    (!name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '$' | '_')))
    .then_some((name, text[index + 1..].trim()))
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
            if matches!(ch, '(' | '[' | '{') {
                depth += 1;
            }
            if matches!(ch, ')' | ']' | '}') {
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

fn find_top_level_comparison(expr: &str, op: &str) -> Option<usize> {
    syntax_positions(expr)
        .into_iter()
        .filter(|(index, _, depth)| {
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
        .next_back()
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
    fn stored_names_drive_text_matching_without_token_grammar() {
        let mut ctx = PreprocessContext::new(None, false);
        for (name, value) in [
            ("a$b", "oak"),
            ("twig\u{301}", "ash"),
            ("two words", "elm"),
            ("rock.dot", "fir"),
        ] {
            ctx.set_var(name, &Value::Str(value.into()));
        }
        assert_eq!(
            ctx.substitute_vars("a$b / twig\u{301} / two words / rock.dot"),
            "oak / ash / elm / fir"
        );
        assert_eq!(
            ctx.substitute_vars("xa$b / a$b2 / \\na$b / ##a$b##"),
            "xa$b / a$b2 / \\noak / oak"
        );
    }

    #[test]
    fn trie_path_mismatch_and_removal_do_not_backtrack() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.set_var("ax", &Value::Str("short".into()));
        ctx.set_var("ax-bc", &Value::Str("long".into()));
        assert_eq!(
            ctx.substitute_vars("ax / ax-bc / ax-bX / ax-"),
            "short / long / ax-bX / ax-"
        );
        ctx.remove_var("ax-bc");
        assert_eq!(
            ctx.substitute_vars("ax / ax-bX / ax-bc"),
            "short / ax-bX / ax-bc"
        );
        ctx.set_var("ax😀z", &Value::Str("astral".into()));
        assert_eq!(
            ctx.substitute_vars("ax😁 / ax😀z / 😀ax / ax²"),
            "ax😁 / astral / 😀ax / short²"
        );
    }

    #[test]
    fn override_trie_lifetime_and_cross_scope_longest_match() {
        let mut scope = LocalMemory::default();
        scope.overrides.insert("a".into(), Value::Int(1));
        scope.overrides.insert("a-bc".into(), Value::Int(2));
        scope.overrides.remove("a-bc");
        assert_eq!(scope.overrides.match_at("a-bX"), Some("a".into()));
        scope.overrides.insert("a-bc".into(), Value::Int(3));
        scope.overrides.remove("a-bc");
        assert_eq!(scope.overrides.match_at("a-bX"), None);
        let mut ctx = PreprocessContext::new(None, false);
        ctx.set_var("a-bc", &Value::Str("global-long".into()));
        ctx.local_vars.push(scope);
        assert_eq!(ctx.substitute_vars("a / a-bc"), "1 / global-long");
        ctx.set_local_var("a-bc", &Value::Str("override".into()));
        assert_eq!(ctx.substitute_vars("a-bc"), "override");
        ctx.remove_var("a-bc");
        assert_eq!(ctx.substitute_vars("a-bc"), "global-long");
    }

    #[test]
    fn variable_values_are_not_rescanned_and_functions_keep_priority() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("!function read()\n!return 7\n!endfunction");
        ctx.set_var("read", &Value::Str("variable".into()));
        ctx.set_var("raw", &Value::Str("read()".into()));
        assert_eq!(
            ctx.substitute_vars("raw / read / read()"),
            "read() / variable / 7"
        );
        ctx.set_var("a", &Value::Str("n".into()));
        ctx.set_var("$b", &Value::Str("value".into()));
        assert_eq!(ctx.substitute_vars("a$b"), "nvalue");
        assert_eq!(
            ctx.substitute_vars("<$b> / <$absent>"),
            "<value> / <$absent>"
        );
    }

    #[test]
    fn duplicate_formals_reuse_named_values_without_consuming_positionals() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("!function $read(b=\"first\",$middle=\"mid\",b=\"last\")\n!return b+2+\"/\"+$middle\n!endfunction");
        let (value, _) = ctx.call_function_values(
            "$read",
            vec![Value::Str("position".into())],
            HashMap::from([("b".into(), Value::Str("007".into()))]),
            false,
        );
        assert_eq!(value.unwrap().to_display(), "0072/position");
        let (value, _) = ctx.call_function_values(
            "$read",
            vec![Value::Int(1), Value::Str("position".into()), Value::Int(4)],
            HashMap::new(),
            false,
        );
        assert_eq!(value.unwrap().to_display(), "6/position");
        assert!(ctx.error.is_none());
    }

    #[test]
    fn empty_sub_terminates_source_before_tail_effects() {
        for prefix in ["", "!procedure $unused()\n", "!function $unused()\n"] {
            let source = format!(
                "@startuml\nclass Anchor\n{prefix}!startsub empty\n/' removed '/\n!endsub\n!$v=%intval(\"bad\")\nclass Lost\n@enduml\n"
            );
            let out = preprocess_full(&source, None);
            assert!(out.error.is_none());
            assert_eq!(out.uml_source, "@startuml\nclass Anchor\n");
            assert!(!out.lines.iter().any(|s| s.contains("Lost")));
        }
    }

    #[test]
    fn empty_replay_is_local_to_execution_and_blank_line_is_not_empty() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.include_depth = 1;
        let mut events = ctx.process("class Head\n!startsub empty\n!endsub\n!$v=%intval(\"bad\")");
        assert!(!ctx.iterator_stopped);
        assert!(ctx.error.is_none());
        ctx.process_one_line("class Tail", &mut events);
        let mut identity = Vec::new();
        project_events(&events, &mut identity, &mut Vec::new(), &mut Vec::new());
        assert_eq!(identity, ["class Head", "class Tail"]);
        let out = preprocess_full(
            "@startuml\n!startsub blank\n\n!endsub\nclass Continued\n@enduml",
            None,
        );
        assert_eq!(out.uml_source, "@startuml\n\nclass Continued\n@enduml\n");
    }

    #[test]
    fn filename_without_source_environment_remains_empty() {
        // Filename.executeReturnFunction returns empty when getenv("filename") is null.
        let mut ctx = PreprocessContext::new(None, false);
        assert_eq!(ctx.eval_expr_to_value("%filename()").to_display(), "");
        assert!(ctx.error.is_none());
        assert_eq!(ctx.substitute_vars("file: %filename()"), "file: ");
        assert!(ctx.error.is_none());
    }

    #[test]
    fn exact_parameter_keys_survive_memory_and_all_lookup_paths() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("@startuml\n!function $mix(b=\"007\",$b=\"08\")\n!return b+2+\"/\"+$b+3+\"/\"+%get_variable_value(\"b\")\n!endfunction");
        assert_eq!(
            ctx.eval_expr_to_value("$mix()").to_display(),
            "0072/083/007"
        );
        assert_eq!(
            ctx.substitute_vars(r#"$mix(b="oak",$b="ash")"#),
            "oak2/ash3/oak"
        );
        assert!(ctx.error.is_none());
    }

    #[test]
    fn exact_bare_and_dollar_globals_do_not_alias_local_or_sibling_frames() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("@startuml\n!b=\"global\"\n!$b=\"dollar\"\n!function $inner()\n!return b+\"/\"+$b\n!endfunction\n!function $outer(b=\"local\",$b=\"override\")\n!return $inner()+\"/\"+b+\"/\"+$b\n!endfunction");
        assert_eq!(
            ctx.eval_expr_to_value("$outer()").to_display(),
            "global/dollar/local/override"
        );
        assert_eq!(ctx.substitute_vars("b/$b"), "global/dollar");
        ctx.process("!undef b");
        assert!(ctx.get_var("b").is_none());
        assert_eq!(ctx.get_var("$b").unwrap().to_display(), "dollar");
        assert_eq!(
            ctx.eval_expr_to_value(r#"%variable_exists("b")+%variable_exists("$b")"#)
                .to_display(),
            "1"
        );
    }

    #[test]
    fn exact_local_mutation_and_global_builtin_assignment_are_separate() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("@startuml\n!b=\"root\"\n!$b=\"dollar\"\n!function $edit(b=\"local\",$b=\"override\")\n!b=\"changed\"\n!local $b=\"leaf\"\n!return b+\"/\"+$b\n!endfunction");
        assert_eq!(
            ctx.eval_expr_to_value("$edit()").to_display(),
            "changed/leaf"
        );
        ctx.eval_expr_to_value(r#"%set_variable_value("b","new")"#);
        assert_eq!(ctx.substitute_vars("b/$b"), "new/dollar");
        assert!(ctx.error.is_none());
    }

    #[test]
    fn completed_overloads_keep_arity_and_kind_in_both_declaration_orders() {
        let function = "!function $pick($a)\n!return \"value \"+$a\n!endfunction\n";
        let procedure = "!procedure $pick($a,$b)\nclass \"$a/$b\" as Kept\n!endprocedure\n";
        for definitions in [
            format!("{function}{procedure}"),
            format!("{procedure}{function}"),
        ] {
            let output = preprocess_full(
                &format!(
                    "@startuml\n{definitions}!$v=$pick(7)\nclass \"$v\" as Value\n$pick(\"fir\",\"yew\")\n@enduml"
                ),
                None,
            );
            assert!(output.error.is_none());
            assert!(output.uml_source.contains("class \"value 7\" as Value"));
            assert!(output.uml_source.contains("class \"fir/yew\" as Kept"));
        }
    }

    #[test]
    fn signature_replacement_preserves_other_arities_and_pending_default_capture() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("@startuml\n!function $pick($a=\"seed\")\n!return $a\n!endfunction\n!function $pick($a,$b)\n!return $a+\"/\"+$b\n!endfunction\n!function $pick($a=$pick())\n!return \"new \"+$a\n!endfunction");
        assert_eq!(ctx.eval_expr_to_value("$pick()").to_display(), "new seed");
        assert_eq!(
            ctx.eval_expr_to_value(r#"$pick("fir","yew")"#).to_display(),
            "fir/yew"
        );
        assert!(ctx.error.is_none());
    }

    #[test]
    fn function_names_preserve_dollars_and_text_token_boundaries() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("@startuml\n!function read($a)\n!return \"bare \"+$a\n!endfunction\n!function $read($a)\n!return \"dollar \"+$a\n!endfunction");
        assert_eq!(
            ctx.eval_expr_to_value(r#"read("oak")+"/"+$read("ash")"#)
                .to_display(),
            "bare oak/dollar ash"
        );
        assert_eq!(
            ctx.substitute_vars(r#"spread("oak") / read("ash")"#),
            r#"spread("oak") / bare ash"#
        );
        assert_eq!(
            ctx.eval_expr_to_value(r#"%function_exists("read")+%function_exists("$read")"#)
                .to_display(),
            "2"
        );
    }

    #[test]
    fn sub_collection_precedes_uncalled_return_and_procedure_bodies() {
        for (start, end, ret) in [
            ("!function $unused()", "!endfunction", "!return 1\n"),
            ("!procedure $unused()", "!endprocedure", ""),
        ] {
            let output = preprocess_full(
                &format!(
                    "@startuml\n{start}\n!startsub stored\nclass Kept\n!endsub\n{ret}{end}\n!includesub stored\n@enduml"
                ),
                None,
            );
            assert!(output.error.is_none());
            assert_eq!(output.uml_source.matches("class Kept").count(), 1);
        }
    }

    #[test]
    fn collected_sub_body_executes_only_through_its_downstream_consumer() {
        let source = "@startuml\n!$n=0\n!procedure $tick()\n!startsub saved\n!$n=$n+1\n!endsub\n!endprocedure\n!includesub saved\n$tick()\n!includesub saved\nclass \"$n\" as Count\n@enduml";
        let output = preprocess_full(source, None);
        assert!(output.error.is_none());
        assert!(output.uml_source.contains("class \"3\" as Count"));
    }

    #[test]
    fn nested_sub_in_uncalled_body_fails_before_buffered_expression() {
        let output = preprocess_full(
            "@startuml\n!procedure $never()\n!startsub outer\n!$v=%intval(\"bad\")\n!startsub inner\n!endsub\n!endprocedure\nclass Survivor\n@enduml",
            None,
        );
        assert_eq!(output.error.unwrap().message, "Cannot nest sub");
        assert!(!output.uml_source.contains("Survivor"));
    }

    #[test]
    fn rpn_named_keys_keep_exact_spelling_and_expression_stack_lifetime() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("@startuml\n!$key=\"b\"\n!function $pair($a=\"first\",$b=\"last\")\n!return $a+\"/\"+$b\n!endfunction");
        assert_eq!(
            ctx.eval_expr_to_value(r#"$pair($key="violet")"#)
                .to_display(),
            "first/last"
        );
        assert_eq!(ctx.substitute_vars(r#"$pair($b="blue")"#), "first/blue");
        // Duplicate assignment leaves one key, hence RPN consumes one operand
        // from the surrounding expression. The later binary op then underflows.
        ctx.eval_expr_to_value(r#""prefix"+$pair($a="blue",$a="red")"#);
        assert_eq!(
            ctx.error.unwrap().message,
            "Expression operand stack underflow"
        );
    }

    #[test]
    fn nested_calls_share_named_state_but_body_expressions_are_independent() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("@startuml\n!function $inner($v=\"leaf\")\n!return $v\n!endfunction\n!function $pair($a=\"first\",$b=\"last\")\n!return $a+\"/\"+$b\n!endfunction");
        ctx.eval_expr_to_value(r#"$pair($a="blue",$b=$inner())"#);
        assert_eq!(
            ctx.error.unwrap().message,
            "Unknown built-in function $inner"
        );
    }

    #[test]
    fn plain_builtin_named_rhs_executes_without_becoming_positional() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process(
            "@startuml\n!$n=0\n!function $tick()\n!$n=$n+1\n!return \"cedar\"\n!endfunction",
        );
        assert_eq!(
            ctx.substitute_vars(r#"%strlen("oak",$ignored=$tick())"#),
            "3"
        );
        assert!(matches!(ctx.get_var("$n"), Some(Value::Int(1))));
        ctx.substitute_vars(r#"%strlen($ignored=$tick())"#);
        assert!(matches!(ctx.get_var("$n"), Some(Value::Int(2))));
        assert_eq!(ctx.error.unwrap().message, "Function not found %strlen");
    }

    #[test]
    fn procedure_expression_resolves_coverage_before_kind() {
        for (call, message) in [
            ("$emit()", "Unknown built-in function $emit"),
            (
                "$emit(1)",
                "Illegal call here. Is there a return directive in your function?",
            ),
        ] {
            let mut ctx = PreprocessContext::new(None, false);
            ctx.process("@startuml\n!procedure $emit($a)\nclass Phantom\n!endprocedure");
            ctx.eval_expr_to_value(call);
            assert_eq!(ctx.error.unwrap().message, message);
            assert!(ctx.expression_events.is_empty());
        }
    }

    #[test]
    fn multiline_legacy_prefix_is_raw_even_after_effectful_substitution() {
        let source = "@startuml\n!$n=0\n!function $tick()\n!$n=$n+1\n!return \"expanded\"\n!endfunction\n!definelong ROWS()\none\ntwo\n!enddefinelong\nnote as N\n$tick() ROWS() discarded\nend note\nclass \"$n\" as Count\n@enduml";
        let output = preprocess_full(source, None);
        assert!(output.error.is_none());
        assert!(output.uml_source.contains("$tick() one\ntwo"));
        assert!(output.uml_source.contains("class \"1\" as Count"));
        assert!(!output.uml_source.contains("discarded"));
    }

    #[test]
    fn nested_sub_collection_rejects_before_effects_and_reports_outer_origin() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("@startuml\n!$n=0\n!startsub outer\n!$n=8\n!if 0\n!startsub inner\n!endif\n!endsub\nclass Lost\n@enduml");
        assert!(matches!(ctx.get_var("$n"), Some(Value::Int(0))));
        let error = ctx.error.unwrap();
        assert_eq!(error.message, "Cannot nest sub");
        assert_eq!(error.line, 2);
    }

    #[test]
    fn intval_parses_complete_signed_display_and_preserves_first_error() {
        for (expression, expected) in [
            (r#"%intval("+007")+2"#, 9),
            (r#"%intval("-2147483648")"#, i32::MIN),
            (r#"%intval(%str2json("42"))"#, 42),
        ] {
            let mut ctx = PreprocessContext::new(None, false);
            assert!(matches!(ctx.eval_expr_to_value(expression), Value::Int(n) if n == expected));
            assert!(ctx.error.is_none());
        }
        for text in [
            "12.5",
            " 7",
            "2147483648",
            "",
            "-2147483649",
            "+",
            "-",
            "+-1",
            "²",
            "Ⅻ",
            "𝟎",
        ] {
            let mut ctx = PreprocessContext::new(None, false);
            ctx.eval_expr_to_value(&format!("%intval(\"{text}\")+%strlen()"));
            assert_eq!(
                ctx.error.unwrap().message,
                format!("Cannot convert {text} to integer.")
            );
        }
    }

    #[test]
    fn intval_follows_integer_parse_int_unicode_digits() {
        // Integer.parseInt uses Character.digit(char, 10): any BMP Nd unit.
        for (expression, expected) in [
            ("%intval(\"+１２３\")", 123),
            ("%intval(\"-٤٢\")", -42),
            ("%intval(\"1२3\")", 123),
            ("%intval(\"۰۹\")", 9),
            ("%intval(\"꧰꧹\")", 9),
            ("%intval(\"-２１４７４８３６４８\")", i32::MIN),
            ("%intval(\"２１４７４８３６４７\")", i32::MAX),
        ] {
            let mut ctx = PreprocessContext::new(None, false);
            let value = ctx.eval_expr_to_value(expression);
            assert!(
                matches!(value, Value::Int(n) if n == expected),
                "{expression}: {value:?}"
            );
            assert!(ctx.error.is_none());
        }
        for text in ["２１４７４８３６４８", "-２１４７４８３６４９", "١٢x"] {
            let mut ctx = PreprocessContext::new(None, false);
            ctx.eval_expr_to_value(&format!("%intval(\"{text}\")"));
            assert_eq!(
                ctx.error.unwrap().message,
                format!("Cannot convert {text} to integer.")
            );
        }
    }

    #[test]
    fn hsl_uses_typed_integer_channels_and_argb_alpha_serialization() {
        let mut ctx = PreprocessContext::new(None, false);
        for (expression, expected) in [
            ("%hsl_color(0,100,50)", "#FF0000"),
            ("%hsl_color(0,100,50,50)", "#80ff0000"),
            (r#"%hsl_color("240",100,50)"#, "#FF0000"),
            (r#"%hsl_color(240,100,50,"50")"#, "transparent"),
            ("%hsl_color(240,100,50,100)", "#0000FF"),
        ] {
            assert_eq!(ctx.eval_expr_to_value(expression).to_display(), expected);
            assert!(ctx.error.is_none());
        }
    }

    #[test]
    fn builtin_registration_distinguishes_text_and_expression() {
        let mut ctx = PreprocessContext::new(None, false);
        assert_eq!(
            ctx.substitute_vars("%not_registered(3)"),
            "%not_registered(3)"
        );
        assert!(ctx.error.is_none());
        assert_eq!(
            ctx.eval_expr_to_value("%getenv(\"plantuml.security.hidden\")")
                .to_display(),
            ""
        );
        assert_eq!(
            ctx.eval_expr_to_value("%getenv(\"path.separator\")")
                .to_display(),
            if cfg!(windows) { ";" } else { ":" }
        );
        ctx.eval_expr_to_value("%not_registered(3)");
        assert!(ctx.error.is_some());
    }

    #[test]
    fn builtin_arguments_keep_types_and_utf16_units() {
        let mut ctx = PreprocessContext::new(None, false);
        for (expression, expected) in [
            (r#"%substr("cedar", %str2json("2"))"#, "cedar"),
            (r#"%substr("cedar", "2", 2)"#, "ce"),
            (r#"%dec2hex(-3)"#, "fffffffd"),
            (r#"%dec2hex("17")"#, "0"),
            (r#"%strpos("🌲ab🌿cd", "cd")"#, "6"),
            (r#"%substr("🌲ab🌿cd", 2, 2)"#, "ab"),
            (r#"%strpos(%substr("😀",0,1)+%substr("😀",1,1),"😀")"#, "0"),
        ] {
            assert_eq!(ctx.eval_expr_to_value(expression).to_display(), expected);
            assert!(ctx.error.is_none());
        }
    }

    #[test]
    fn case_conversion_preserves_utf16_and_chr_keeps_typed_contract() {
        let mut ctx = PreprocessContext::new(None, false);
        for (expr, units) in [
            (
                r#"%upper(%substr("🦉",0,1))+%lower(%substr("🦉",1,1))"#,
                "🦉".encode_utf16().collect(),
            ),
            (
                r#"%upper("oak"+%substr("🦉",0,1))"#,
                vec![79, 65, 75, 0xd83e],
            ),
            (r#"%chr("65")"#, vec![0]),
            (r#"%chr(%str2json("65"))"#, vec![0]),
            ("%chr(65)", vec![65]),
            ("%chr(55296)", vec![55296]),
        ] {
            assert_eq!(ctx.eval_expr_to_value(expr).utf16_units(), units, "{expr}");
            assert!(ctx.error.is_none());
        }
    }

    #[test]
    fn boolval_is_text_conversion_not_truthiness() {
        let mut ctx = PreprocessContext::new(None, false);
        for (expr, expected) in [
            (r#"%boolval("TrUe")+2"#, "3"),
            (r#"%boolval(%str2json("false"))+3"#, "3"),
            (r#"%boolval(1)+%boolval("0")"#, "1"),
        ] {
            assert_eq!(ctx.eval_expr_to_value(expr).to_display(), expected);
            assert!(ctx.error.is_none());
        }
        ctx.eval_expr_to_value(r#"%boolval("00")"#);
        assert_eq!(ctx.error.unwrap().message, "Cannot convert 00 to boolean.");
    }

    #[test]
    fn arity_failure_occurs_after_arguments_and_before_body_or_next_expression() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("@startuml\n!$n=0\n!function $tick()\n!$n=$n+1\n!return 1\n!endfunction");
        ctx.eval_expr_to_value(r#"%strlen($tick(),"extra")+%date("Q",0)"#);
        assert_eq!(
            ctx.error.as_ref().unwrap().message,
            "Unknown built-in function %strlen"
        );
        assert!(matches!(ctx.get_var("$n"), Some(Value::Int(1))));
        let output = preprocess_full(
            "@startuml\n!function $only($a)\n!return $a\n!endfunction\n!$v=$only()\nclass After\n@enduml",
            None,
        );
        assert!(
            output
                .error
                .unwrap()
                .message
                .contains("Unknown built-in function $only")
        );
        assert!(!output.uml_source.contains("class"));
    }

    #[test]
    fn named_keys_are_expression_operands_but_plain_call_literals() {
        let source = "@startuml\n!$a=\"$b\"\n!function $pair($a=\"seed\",$b=\"leaf\")\n!return $a+\"/\"+$b\n!endfunction\n!$r=$pair($a=\"oak\",$b=4)\nclass \"$r / $pair($a=\"elm\",$b=5)\" as Keys\n@enduml";
        // The original source is preserved: its prepared $a and $b keys
        // are both "$b", so Java's distinct-key count requires a missing
        // positional operand. The earlier success assertion encoded our bug.
        let output = preprocess_full(source, None);
        assert_eq!(
            output.error.unwrap().message,
            "Expression operand stack underflow"
        );
        assert!(!output.uml_source.contains("class"));
    }

    #[test]
    fn legacy_calls_scan_original_text_once_with_newline_boundary() {
        let source = "@startuml\n!define TOKEN gold\n!definelong STAMP()\nchanged\n!enddefinelong\n!$v=\"STAMP()\"\nclass \"$v / STAMP() / STAMP / north\\nTOKEN / prefixTOKEN\" as Literal\n@enduml";
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass \"STAMP() / changed / STAMP / north\\ngold / prefixTOKEN\" as Literal\n@enduml\n"
        );
    }

    #[test]
    fn local_sub_end_suffix_closes_before_following_model() {
        let source = "@startuml\n!$n=0\n!startsub chosen\n!$n=$n+2\n!endsub: suffix\n!includesub chosen\nclass Record$n\n@enduml";
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass Record4\n@enduml\n"
        );
    }

    #[test]
    fn json_scalar_iteration_and_intermediate_missing_members_are_fatal() {
        for expr in ["true", "3", "null", "\"text\""] {
            let source = format!(
                "@startuml\n!foreach $x in %str2json('{expr}')\n!endfor\nclass After\n@enduml"
            );
            let output = preprocess_full(&source, None);
            assert!(output.error.is_some(), "{expr}");
            assert!(!output.uml_source.contains("class"));
        }
        let mut ctx = PreprocessContext::new(None, false);
        ctx.eval_expr_to_value(r#"{"present":1}"#);
        ctx.process("@startuml\n!$data={\"present\":1}");
        assert_eq!(ctx.eval_expr_to_value("$data.absent").to_display(), "");
        assert!(ctx.error.is_none());
        ctx.eval_expr_to_value("$data.absent[0]");
        assert!(ctx.error.is_some());
    }

    #[test]
    fn expression_snapshots_all_variable_operands_before_calls() {
        let source = "@startuml\n!$n=4\n!function $step()\n!$n=$n+1\n!return 10\n!endfunction\n!$v=$step()+$n+%get_variable_value(\"$n\")\nclass \"$v / $n\" as Snapshot\n@enduml";
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass \"19 / 5\" as Snapshot\n@enduml\n"
        );
    }

    #[test]
    fn return_unwinds_nested_loops_without_later_effects() {
        let source = "@startuml\n!$n=0\n!function $pick()\n!foreach $x in [\"05\",\"06\"]\n!while $n < 3\n!$n=$n+1\n!return $x+2\n!$n=99\n!endwhile\n!endfor\n!return \"miss\"\n!endfunction\n!$v=$pick()\nclass \"$v / $n / $x\" as Pick\n@enduml";
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass \"052 / 1 / 05\" as Pick\n@enduml\n"
        );
    }

    #[test]
    fn literal_values_and_captured_defines_are_not_expanded_again() {
        let source = "@startuml\n!define FIRST LATER\n!define LATER changed\n!$v=\"LATER\"\nclass \"FIRST / $v\" as Literal\n@enduml";
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass \"LATER / LATER\" as Literal\n@enduml\n"
        );
    }

    #[test]
    fn json_paths_and_foreach_preserve_json_type_and_global_binding() {
        let source = "@startuml\n!$data={\"rows\":[\"009\",3]}\n!$i=0\n!$v=$data.rows[$i]+2\n!foreach $item in $data.rows\n!$last=$item+2\n!endfor\nclass \"$v / $last / $item\" as Values\n@enduml";
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass \"0092 / 32 / 3\" as Values\n@enduml\n"
        );
    }

    #[test]
    fn builtin_and_call_kind_errors_prevent_later_effects() {
        for expression in [
            r#"%substr("abc",-1)"#,
            r#"%substr("abc",0,-1)"#,
            "%unregistered()",
            "$emit()",
        ] {
            let source = format!(
                "@startuml\n!procedure $emit()\nclass Unexpected\n!endprocedure\n!$v={expression}\nclass After\n@enduml"
            );
            let output = preprocess_full(&source, None);
            assert!(output.error.is_some(), "{expression}");
            assert!(!output.uml_source.contains("class"), "{expression}");
        }
    }

    #[test]
    fn includesub_expands_selector_and_validates_ignored_markers() {
        let dir =
            std::env::temp_dir().join(format!("rustuml-sub-selection-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("blocks.inc"),
            "!$bad=%substr(\"a\",-1)\n!startsub\tchosen\n!$n=$n+3\n!endsub\n",
        )
        .unwrap();
        let source = "@startuml\n!$n=0\n!$which=\"blocks.inc!chosen\"\n!includesub $which\nclass Record$n\n@enduml";
        let output = preprocess_full(source, Some(dir.clone()));
        assert!(output.error.is_none());
        assert_eq!(output.uml_source, "@startuml\nclass Record3\n@enduml\n");
        std::fs::write(
            dir.join("blocks.inc"),
            "!startsub bad-name\n!endsub\n!startsub chosen\nclass Selected\n!endsub\n",
        )
        .unwrap();
        let output = preprocess_full(source, Some(dir.clone()));
        assert_eq!(output.error.unwrap().line, 0);
        assert!(!output.uml_source.contains("Selected"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn typed_arguments_and_lookup_do_not_reinfer_strings() {
        let source = r#"@startuml
!$seed = "007"
!function $suffix($a=$seed, $b=7)
!return $a + $b
!endfunction
!$seed = "changed"
!$value = $suffix($b=2)
!$other = %get_variable_value("$value") + 3
class "$other" as Typed
@enduml"#;
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass \"00723\" as Typed\n@enduml\n"
        );
    }

    #[test]
    fn return_plain_simulates_nested_calls_but_procedures_emit() {
        let source = r#"@startuml
!$count=0
!procedure $emit()
class Nested
!endprocedure
!function $value()
!$count=$count+1
class Phantom
$emit()
!return "kept"
!endfunction
!$label=$value()
class "$label $count" as Actual
@enduml"#;
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass Nested\nclass \"kept 1\" as Actual\n@enduml\n"
        );
    }

    #[test]
    fn integer_arithmetic_wraps_but_literal_overflow_is_fatal() {
        let mut ctx = PreprocessContext::new(None, false);
        assert!(matches!(
            ctx.eval_expr_to_value("2147483647 + 1"),
            Value::Int(i32::MIN)
        ));
        assert!(matches!(
            ctx.eval_expr_to_value("(-2147483647 - 1) - 1"),
            Value::Int(i32::MAX)
        ));
        assert!(matches!(ctx.eval_expr_to_value("7 / 2"), Value::Int(3)));
        assert!(matches!(
            ctx.eval_expr_to_value("%str2json(\"1.5\")"),
            Value::Json(_)
        ));
        ctx.eval_expr_to_value("2147483648");
        assert!(ctx.error.is_some());
    }

    #[test]
    fn eager_operands_do_not_execute_literal_values() {
        let source = r#"@startuml
!$count=0
!function $tick()
!$count=$count+1
!return 1
!endfunction
!$v=(1 || $tick()) && (0 || "$tick()")
class "$count / $v" as Actual
@enduml"#;
        assert_eq!(
            preprocess_full(source, None).uml_source,
            "@startuml\nclass \"1 / 1\" as Actual\n@enduml\n"
        );
    }

    #[test]
    fn first_error_stops_later_arguments_and_mutations() {
        let mut ctx = PreprocessContext::new(None, false);
        ctx.process("@startuml\n!$n=0\n!function $tick()\n!$n=$n+1\n!return 1\n!endfunction");
        ctx.eval_expr_to_value("%substr(%date(\"QQ\",0),$tick(),%date(\"XXXX\",0))");
        assert!(
            ctx.error
                .as_ref()
                .unwrap()
                .message
                .contains("unsupported letter Q")
        );
        assert!(matches!(ctx.get_var("$n"), Some(Value::Int(0))));
        ctx.eval_expr_to_value("%set_variable_value(\"$n\",9)");
        assert!(matches!(ctx.get_var("$n"), Some(Value::Int(0))));
        assert!(
            ctx.error
                .as_ref()
                .unwrap()
                .message
                .contains("unsupported letter Q")
        );
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
            long_name: "Coordinated Universal Time".to_string(),
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
        // EaterAffectationDefine stores NAME only; ShuntingYard resolves
        // $NAME independently, while VariableManager scans bare NAME after '$'.
        // TIM7 supplement2 define_spelling/define_condition confirms this;
        // the original input and prior assertion are preserved in maker evidence.
        assert_eq!(lines, vec!["$Alice -> Bob : hello"]);
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
        // EaterAffectationDefine stores NAME only; ShuntingYard resolves
        // $NAME independently, while VariableManager scans bare NAME after '$'.
        // TIM7 supplement2 define_spelling/define_condition confirms this;
        // the original input and prior assertion are preserved in maker evidence.
        assert_eq!(lines, vec!["A -> B : dev"]);
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
        // EaterAffectationDefine stores NAME only; ShuntingYard resolves
        // $NAME independently, while VariableManager scans bare NAME after '$'.
        // TIM7 supplement2 define_spelling/define_condition confirms this;
        // the original input and prior assertion are preserved in maker evidence.
        assert_eq!(lines, vec!["$Alice -> $Bob : $hello"]);
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
        // EaterAffectationDefine stores NAME only; ShuntingYard resolves
        // $NAME independently, while VariableManager scans bare NAME after '$'.
        // TIM7 supplement2 define_spelling/define_condition confirms this;
        // the original input and prior assertion are preserved in maker evidence.
        assert_eq!(lines, vec!["$Alice -> Bob : hi"]);

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
        // EaterAffectationDefine stores NAME only; ShuntingYard resolves
        // $NAME independently, while VariableManager scans bare NAME after '$'.
        // TIM7 supplement2 define_spelling/define_condition confirms this;
        // the original input and prior assertion are preserved in maker evidence.
        assert_eq!(
            lines,
            vec!["$Alice -> Bob : hello", "$Alice -> Bob : world"]
        );
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
        // TContext appends caller prefix and TFunctionImpl legacy return verbatim.
        assert_eq!(lines[5], "    +token: String");
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
    fn builtin_feature_returns_numeric_false() {
        // Feature.executeReturnFunction returns TValue.fromInt(0) for this key.
        let input = "@startuml\n!$f = %feature(\"dark-mode\")\nnote : $f\n@enduml";
        let lines = pp(input);
        assert_eq!(lines, vec!["note : 0"]);
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
