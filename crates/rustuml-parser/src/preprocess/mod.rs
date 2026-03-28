// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! TIM preprocessor — handles variables, defines, conditionals,
//! includes, and comments before diagram-specific parsing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::diagram::SpriteData;

/// Output from the preprocessor: expanded lines plus any sprite definitions.
pub struct PreprocessOutput {
    pub lines: Vec<String>,
    pub sprites: HashMap<String, SpriteData>,
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
    let mut ctx = PreprocessContext::new(base_dir);
    let lines = ctx.process(input);
    PreprocessOutput {
        lines,
        sprites: ctx.sprites,
    }
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
    in_sprite_block: bool,
    /// The name of the sprite currently being collected (if any).
    current_sprite_name: Option<String>,
    /// Sprite definitions parsed from `sprite $name { ... }` blocks.
    sprites: HashMap<String, SpriteData>,
    archimate_enabled: bool,
    in_diagram_block: bool,
    seen_start_tag: bool,
    base_dir: Option<PathBuf>,
    include_depth: usize,
    foreach_stack: Vec<ForEachState>,
    while_stack: Vec<WhileState>,
    functions: HashMap<String, FunctionDef>,
    collecting_function: Option<String>,
    collecting_definelong: Option<String>,
    subs: HashMap<String, Vec<String>>,
    collecting_sub: Option<String>,
    /// Local variable scopes for function calls (stack of saved scopes).
    local_vars: Vec<HashMap<String, String>>,
    /// Pending return value from a `!return` inside a function body.
    /// Set by process_one_line when `!return` is encountered while active.
    return_signal: Option<Value>,
}

const MAX_INCLUDE_DEPTH: usize = 10;
const MAX_WHILE_ITERATIONS: usize = 10000;

struct CondState {
    active: bool,
    has_matched: bool,
}

struct ForEachState {
    var_name: String,
    values: Vec<String>,
    body_lines: Vec<String>,
}

struct WhileState {
    condition: String,
    body_lines: Vec<String>,
}

#[derive(Clone)]
struct FunctionDef {
    params: Vec<FuncParam>,
    body: Vec<String>,
}

#[derive(Clone)]
struct FuncParam {
    name: String,
    default: Option<String>,
}

#[derive(Clone)]
struct DefineLongDef {
    params: Vec<String>,
    body: Vec<String>,
}

impl PreprocessContext {
    fn new(base_dir: Option<PathBuf>) -> Self {
        Self {
            defines: HashMap::new(),
            token_defines: HashMap::new(),
            definelong_macros: HashMap::new(),
            cond_stack: Vec::new(),
            in_block_comment: false,
            in_sprite_block: false,
            current_sprite_name: None,
            sprites: HashMap::new(),
            archimate_enabled: false,
            in_diagram_block: false,
            seen_start_tag: false,
            base_dir,
            include_depth: 0,
            foreach_stack: Vec::new(),
            while_stack: Vec::new(),
            functions: HashMap::new(),
            collecting_function: None,
            collecting_definelong: None,
            subs: HashMap::new(),
            collecting_sub: None,
            local_vars: Vec::new(),
            return_signal: None,
        }
    }

    fn is_active(&self) -> bool {
        self.cond_stack.iter().all(|c| c.active)
    }

    fn get_var(&self, name: &str) -> Option<&String> {
        // Check local scope first, then global.
        for scope in self.local_vars.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        self.defines.get(name)
    }

    fn set_var(&mut self, name: &str, value: &str) {
        // Update the innermost local scope that already holds this variable,
        // so that !local variables are updated in-place by plain !$var = ...
        for scope in self.local_vars.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value.to_string());
                return;
            }
        }
        self.defines.insert(name.to_string(), value.to_string());
    }

    fn set_local_var(&mut self, name: &str, value: &str) {
        if let Some(scope) = self.local_vars.last_mut() {
            scope.insert(name.to_string(), value.to_string());
        } else {
            // No local scope active, set global.
            self.set_var(name, value);
        }
    }

    fn process(&mut self, input: &str) -> Vec<String> {
        let lines: Vec<&str> = input.lines().collect();
        self.process_lines(&lines)
    }

    fn process_lines(&mut self, lines: &[&str]) -> Vec<String> {
        let mut output = Vec::new();

        for &line in lines {
            self.process_one_line(line, &mut output);
        }

        output
    }

    fn process_one_line(&mut self, line: &str, output: &mut Vec<String>) {
        let trimmed = line.trim();

        // Collect sprite pixel-data blocks.
        if self.in_sprite_block {
            if trimmed == "}" {
                self.in_sprite_block = false;
                self.current_sprite_name = None;
            } else if let Some(name) = self.current_sprite_name.clone() {
                // Accumulate pixel row into the sprite definition.
                if let Some(sprite) = self.sprites.get_mut(&name) {
                    // Only accept lines that look like hex pixel data.
                    if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
                        sprite.rows.push(trimmed.to_string());
                        sprite.height = sprite.rows.len() as u32;
                    }
                }
            }
            return;
        }

        // Handle block comments.
        if self.in_block_comment {
            if trimmed.contains("'/") {
                self.in_block_comment = false;
            }
            return;
        }
        if trimmed.starts_with("/'") {
            if !trimmed.contains("'/") || trimmed.ends_with("/'") {
                self.in_block_comment = true;
            }
            return;
        }

        // Skip single-line comments.
        if trimmed.starts_with('\'') {
            return;
        }

        // Strip inline comments.
        let line_no_comment = strip_inline_comment(line);
        let trimmed = line_no_comment.trim();

        // Handle @start/@end tags.
        // PlantUML renders only the first block. Files without any
        // @start tag pass through entirely.
        if self.include_depth == 0 {
            if trimmed.starts_with("@start") {
                if !self.seen_start_tag {
                    self.seen_start_tag = true;
                    self.in_diagram_block = true;
                }
                return;
            }
            if trimmed.starts_with("@end") {
                self.in_diagram_block = false;
                return;
            }
            if self.seen_start_tag && !self.in_diagram_block {
                return;
            }
        } else if trimmed.starts_with("@start") || trimmed.starts_with("@end") {
            return;
        }

        // Parse sprite definitions: `sprite $name [WxH/Z] {`
        if trimmed.starts_with("sprite ") || trimmed.starts_with("sprite\t") {
            if trimmed.ends_with('{') {
                // Parse: sprite $name [WxH/Z] {
                let rest = trimmed["sprite ".len()..].trim_start();
                // Strip leading $ if present.
                let rest = rest.strip_prefix('$').unwrap_or(rest);
                // Name is the first word (up to whitespace, '[', or '{').
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                // Try to parse [WxH/Z] dimensions if present.
                let (width, height) = parse_sprite_dimensions(rest);
                if !name.is_empty() {
                    self.sprites.insert(
                        name.clone(),
                        SpriteData {
                            width,
                            height,
                            rows: Vec::new(),
                        },
                    );
                    self.current_sprite_name = Some(name);
                }
                self.in_sprite_block = true;
            }
            return;
        }

        // Collecting definelong body.
        if self.collecting_definelong.is_some() {
            if trimmed == "!enddefinelong" {
                self.collecting_definelong = None;
            } else if let Some(name) = self.collecting_definelong.clone()
                && let Some(dl) = self.definelong_macros.get_mut(&name)
            {
                dl.body.push(line.to_string());
            }
            return;
        }

        // Collecting sub.
        if self.collecting_sub.is_some() {
            if trimmed == "!endsub" {
                self.collecting_sub = None;
            } else if let Some(name) = self.collecting_sub.clone() {
                self.subs.entry(name).or_default().push(line.to_string());
            }
            return;
        }

        // Function definition collection.
        if self.try_function_def(trimmed) {
            return;
        }

        // While buffering.
        if self.try_while(trimmed, output) {
            return;
        }

        // Foreach buffering.
        if self.try_foreach(trimmed, output) {
            return;
        }

        // Directives that are consumed silently.
        if self.try_silent_directive(trimmed) {
            return;
        }

        // Startsub.
        if let Some(name) = trimmed.strip_prefix("!startsub ") {
            self.collecting_sub = Some(name.trim().to_string());
            return;
        }
        if trimmed == "!endsub" {
            return;
        }

        // Definelong.
        if self.try_definelong(trimmed) {
            return;
        }

        // Process TIM directives.
        if self.try_define(trimmed) {
            return;
        }
        if self.try_local_var(trimmed) {
            return;
        }
        if self.try_conditional(trimmed) {
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
            return;
        }

        if self.try_undefine(trimmed) {
            return;
        }
        if let Some(included_lines) = self.try_include(trimmed) {
            if self.is_active() {
                output.extend(included_lines);
            }
            return;
        }
        if let Some(included_lines) = self.try_includesub(trimmed) {
            if self.is_active() {
                output.extend(included_lines);
            }
            return;
        }
        if self.try_theme(trimmed, output) {
            return;
        }

        // !dump_memory — emit a comment line with all current defines (debugging aid).
        if trimmed == "!dump_memory" {
            if self.is_active() {
                let mut pairs: Vec<String> = self
                    .defines
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect();
                pairs.sort();
                let token_pairs: Vec<String> = self
                    .token_defines
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect();
                let mut all = pairs;
                let mut sorted_tokens = token_pairs;
                sorted_tokens.sort();
                all.extend(sorted_tokens);
                output.push(format!("' [dump] {}", all.join(", ")));
            }
            return;
        }

        // Function/procedure invocation as standalone line.
        if self.try_function_call(trimmed, output) {
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
                    let after_definelong = self.try_expand_definelong(&subst).unwrap_or(subst);
                    // Expand any inline $func(args) calls in output content.
                    self.expand_inline_func_calls_in_line(&after_definelong)
                };
            // A definelong expansion can produce multiple lines joined with '\n'.
            // Split them so each line is processed individually by the parser.
            for expanded_line in line_to_process.split('\n') {
                if !expanded_line.trim().is_empty() {
                    output.push(expanded_line.to_string());
                }
            }
        }
    }

    fn try_silent_directive(&self, line: &str) -> bool {
        // !log, !pragma, !assert — consume silently.
        // !includeurl / !import — URL fetching deferred; strip the line silently.
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
        false
    }

    fn try_local_var(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!local\s+\$(\w+)\s*=\s*(.+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            if self.is_active() {
                let name = caps[1].to_string();
                let raw_value = caps[2].trim();
                let value = self.eval_expr_to_value(raw_value);
                self.set_local_var(&name, &value.to_display());
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

    fn try_expand_definelong(&self, line: &str) -> Option<String> {
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
                        let mut expanded = body_line.clone();
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
                        result_lines.push(expanded);
                    }
                    // For single-line macros, replace inline.
                    if result_lines.len() == 1 {
                        let prefix = &trimmed[..idx];
                        let suffix = &trimmed[idx + name.len() + end + 1..];
                        return Some(format!("{prefix}{}{suffix}", result_lines[0]));
                    }
                    return Some(result_lines.join("\n"));
                }
            }
            // Bare name match (no parens, no params).
            if dl.params.is_empty() && trimmed.contains(name.as_str()) {
                // Only substitute word-boundary matches.
                if let Ok(re) = Regex::new(&format!(r"\b{}\b", regex::escape(name)))
                    && re.is_match(trimmed)
                {
                    let body = dl.body.join("\n");
                    let result = re.replace_all(trimmed, body.as_str()).to_string();
                    return Some(result);
                }
            }
        }
        None
    }

    fn try_function_def(&mut self, line: &str) -> bool {
        // Currently collecting a function body?
        if let Some(func_name) = &self.collecting_function.clone() {
            if line == "!endfunction" || line == "!endprocedure" {
                self.collecting_function = None;
            } else if let Some(func) = self.functions.get_mut(func_name) {
                func.body.push(line.to_string());
            }
            return true;
        }

        // !function $name($param1, $param2 = "default")
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^!(?:function|procedure)\s+\$(\w+)\s*\(([^)]*)\)$").unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let params = parse_func_params(&caps[2]);

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

    /// Call a function and return its return value (if any) plus any output lines.
    fn call_function(&mut self, name: &str, args: &[String]) -> (Option<Value>, Vec<String>) {
        let func = match self.functions.get(name) {
            Some(f) => f.clone(),
            None => return (None, Vec::new()),
        };

        // Save current values, set params.
        let mut saved: Vec<(String, Option<String>)> = Vec::new();
        for (i, param) in func.params.iter().enumerate() {
            saved.push((param.name.clone(), self.get_var(&param.name).cloned()));
            let arg_val = if let Some(arg) = args.get(i) {
                arg.clone()
            } else if let Some(def) = &param.default {
                def.clone()
            } else {
                String::new()
            };
            self.set_var(&param.name, &arg_val);
        }

        // Push a local scope.
        self.local_vars.push(HashMap::new());

        // Save and clear any pending return signal from an outer context.
        let outer_return = self.return_signal.take();

        let mut output_lines = Vec::new();

        for body_line in &func.body {
            // Process through the preprocessor (handles !if/!while/!return etc.)
            self.process_one_line(body_line, &mut output_lines);
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

        // Restore saved values.
        for (param, old_val) in saved {
            match old_val {
                Some(v) => {
                    self.defines.insert(param, v);
                }
                None => {
                    self.defines.remove(&param);
                }
            }
        }

        (return_value, output_lines)
    }

    fn try_function_call(&mut self, line: &str, output: &mut Vec<String>) -> bool {
        // $funcName("arg1", "arg2")
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\$(\w+)\s*\(([^)]*)\)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let args_str = &caps[2];

            if !self.functions.contains_key(&name) {
                return false;
            }

            if !self.is_active() {
                return true;
            }

            let args = parse_call_args(args_str);
            let (ret, lines) = self.call_function(&name, &args);
            if lines.is_empty() {
                // If the function produced no output lines but returned a value,
                // emit the return value as an output line (e.g. note body calls).
                if let Some(val) = ret {
                    let s = val.to_display();
                    if !s.is_empty() {
                        output.push(s);
                    }
                }
            } else {
                output.extend(lines);
            }

            return true;
        }

        false
    }

    /// Evaluate a function call in an expression context, returning the return value.
    fn eval_function_call(&mut self, name: &str, args_str: &str) -> Value {
        let args = parse_call_args(args_str);
        let (ret, _lines) = self.call_function(name, &args);
        ret.unwrap_or(Value::Str(String::new()))
    }

    /// Expand inline `$func(args)` calls within a content line (output line).
    /// This allows function return values to appear inline in notes and messages.
    /// Only user-defined functions (not procedures) that return a value are expanded.
    fn expand_inline_func_calls_in_line(&mut self, line: &str) -> String {
        static INLINE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$(\w+)\(").unwrap());

        // Check if there are any $func( patterns at all.
        if !INLINE_RE.is_match(line) {
            return line.to_string();
        }

        let mut result = line.to_string();
        // Iterate to handle nested or multiple calls, up to a limit.
        for _ in 0..20 {
            // Find the LAST (innermost) $func( to evaluate inside-out.
            let Some(m) = INLINE_RE.find_iter(&result).last() else {
                break;
            };
            let start = m.start();
            let after_open = m.end();
            let func_name = &result[start + 1..after_open - 1].to_string();

            // Only expand if this function is defined.
            if !self.functions.contains_key(func_name.as_str()) {
                // No more user-defined calls to expand; stop.
                break;
            }

            let Some(close) = find_matching_paren_from(&result, after_open - 1) else {
                break;
            };
            let args_str = result[after_open..close].to_string();
            let ret_val = self.eval_function_call(func_name, &args_str);
            let replacement = ret_val.to_display();
            result = format!("{}{replacement}{}", &result[..start], &result[close + 1..]);
        }

        result
    }

    fn try_foreach(&mut self, line: &str, output: &mut Vec<String>) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^!foreach\s+\$(\w+)\s+in\s+\[(.+)\]$"#).unwrap());

        if let Some(caps) = RE.captures(line) {
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

        if (line == "!endfor" || line == "!endforeach")
            && let Some(foreach) = self.foreach_stack.pop()
        {
            if self.is_active() {
                for val in &foreach.values {
                    let old_val = self.get_var(&foreach.var_name).cloned();
                    self.set_var(&foreach.var_name, val);

                    for body_line in &foreach.body_lines {
                        let line_refs: Vec<&str> = vec![body_line.as_str()];
                        let expanded = self.process_lines(&line_refs);
                        output.extend(expanded);
                    }

                    match old_val {
                        Some(v) => {
                            self.set_var(&foreach.var_name, &v);
                        }
                        None => {
                            self.defines.remove(&foreach.var_name);
                        }
                    }
                }
            }
            return true;
        }

        // If we're inside a foreach, buffer the line.
        if let Some(foreach) = self.foreach_stack.last_mut() {
            foreach.body_lines.push(line.to_string());
            return true;
        }

        false
    }

    fn try_while(&mut self, line: &str, output: &mut Vec<String>) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^!while\s+(.+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
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

        if line == "!endwhile" {
            if let Some(while_state) = self.while_stack.pop()
                && self.is_active()
            {
                let mut iterations = 0;
                loop {
                    if iterations >= MAX_WHILE_ITERATIONS {
                        break;
                    }
                    let cond_expanded = self.substitute_vars(&while_state.condition);
                    if !self.eval_bool(&cond_expanded) {
                        break;
                    }

                    for body_line in &while_state.body_lines {
                        let line_refs: Vec<&str> = vec![body_line.as_str()];
                        let expanded = self.process_lines(&line_refs);
                        output.extend(expanded);
                    }

                    iterations += 1;
                }
            }
            return true;
        }

        // If we're inside a while, buffer the line.
        if let Some(while_state) = self.while_stack.last_mut() {
            while_state.body_lines.push(line.to_string());
            return true;
        }

        false
    }

    fn try_include(&mut self, line: &str) -> Option<Vec<String>> {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^!include(?:_once)?\s+(?:"([^"]+)"|(\S+))$"#).unwrap());

        let caps = RE.captures(line)?;
        let path_str = caps.get(1).or(caps.get(2)).map(|m| m.as_str())?;

        if self.include_depth >= MAX_INCLUDE_DEPTH {
            return Some(vec![format!(
                "' WARNING: max include depth ({MAX_INCLUDE_DEPTH}) reached for {path_str}"
            )]);
        }

        // Handle known stdlib includes.
        if path_str == "<archimate/Archimate>" || path_str == "archimate/Archimate" {
            self.archimate_enabled = true;
            return Some(vec![]);
        }
        if path_str.starts_with('<') {
            return Some(vec![]);
        }

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

    fn try_includesub(&mut self, line: &str) -> Option<Vec<String>> {
        // !includesub file.puml!SUBNAME
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^!includesub\s+(?:"([^"]+)"|(\S+))!(\w+)$"#).unwrap());

        let caps = RE.captures(line)?;
        let path_str = caps.get(1).or(caps.get(2)).map(|m| m.as_str())?;
        let sub_name = &caps[3];

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
                let mut temp_ctx = PreprocessContext::new(self.base_dir.clone());
                temp_ctx.include_depth = self.include_depth + 1;
                let _ = temp_ctx.process(&content);
                // Now extract the named sub.
                if let Some(sub_lines) = temp_ctx.subs.get(sub_name) {
                    let refs: Vec<&str> = sub_lines.iter().map(|s| s.as_str()).collect();
                    Some(self.process_lines(&refs))
                } else {
                    Some(vec![])
                }
            }
            Err(_) => Some(vec![]),
        }
    }

    fn try_theme(&self, line: &str, output: &mut Vec<String>) -> bool {
        if let Some(rest) = line.strip_prefix("!theme ") {
            let theme_name = rest.trim();
            if self.is_active() && !theme_name.is_empty() {
                output.push(format!("skinparam __theme {theme_name}"));
            }
            return true;
        }
        false
    }

    fn try_undefine(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!undef(?:ine)?\s+(\w+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let name = &caps[1];
            self.defines.remove(name);
            self.token_defines.remove(name);
            return true;
        }
        false
    }

    fn try_conditional(&mut self, line: &str) -> bool {
        static RE_IF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^!if\s+(.+)$"#).unwrap());
        static RE_ELSEIF: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^!elseif\s+(.+)$"#).unwrap());
        static RE_IFDEF: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!ifdef\s+(\w+)$").unwrap());
        static RE_IFNDEF: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!ifndef\s+(\w+)$").unwrap());

        if let Some(caps) = RE_IFDEF.captures(line) {
            let defined =
                self.defines.contains_key(&caps[1]) || self.token_defines.contains_key(&caps[1]);
            self.cond_stack.push(CondState {
                active: defined && self.is_active(),
                has_matched: defined,
            });
            return true;
        }
        if let Some(caps) = RE_IFNDEF.captures(line) {
            let not_defined =
                !self.defines.contains_key(&caps[1]) && !self.token_defines.contains_key(&caps[1]);
            self.cond_stack.push(CondState {
                active: not_defined && self.is_active(),
                has_matched: not_defined,
            });
            return true;
        }
        if let Some(caps) = RE_IF.captures(line) {
            let expr = caps[1].trim();
            let expanded = self.substitute_vars(expr);
            let result = self.eval_bool(&expanded);
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
                let expanded = self.substitute_vars(expr);
                self.eval_bool(&expanded)
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
    fn eval_expr_to_value(&mut self, expr: &str) -> Value {
        let expr = expr.trim();

        // Strip surrounding quotes.
        // Strip surrounding quotes only when the WHOLE expression is a single
        // quoted string (i.e. the closing `"` at position len-1 is genuinely
        // the end of the string that started at position 0, not part of a
        // later token in a compound expression like `"Hello, " + World + "!"`).
        if expr.starts_with('"') && expr.ends_with('"') && expr.len() >= 2 {
            // Walk the interior to find where the opening quote closes.  If the
            // first closing quote is at the very end, we have a single string.
            let inner_candidate = &expr[1..expr.len() - 1];
            // Check for an unescaped quote inside the inner part.  If there is
            // none, the last `"` is the matching close of the opening `"`.
            if !inner_candidate.contains('"') {
                let substituted = self.substitute_vars(inner_candidate);
                return Value::Str(substituted);
            }
            // The interior contains quotes — the expression is something like
            // `"Hello, " + x + "!"`.  Fall through to the general path.
        }

        // If the expression is a bare word that is a token_define, return its
        // raw value as a string (without further evaluation).  This preserves
        // quoted token values like `!define VERSION "2.0"` — when VERSION
        // appears in a string concatenation `"v" + VERSION`, the result is
        // `v"2.0"` (with quotes), matching PlantUML's text-substitution model.
        static BARE_WORD_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^[A-Za-z_]\w*$").unwrap());
        if BARE_WORD_RE.is_match(expr)
            && let Some(raw) = self.token_defines.get(expr).cloned()
        {
            return Value::Str(raw);
        }

        // Boolean literals (plain "true"/"false" without % prefix).
        if expr == "true" {
            return Value::Bool(true);
        }
        if expr == "false" {
            return Value::Bool(false);
        }
        // %true() and %false() are builtin functions that return 1 and 0 (numeric).
        // Fall through to the BUILTIN_CALL_RE path which dispatches to
        // eval_one_builtin_args("true"/"false", ...) returning "1"/"0".

        // If the expression is a bare builtin call (%func(...)), evaluate it
        // directly and return the result as a string.  This avoids the problem
        // where substitute_vars eagerly evaluates builtins (e.g. %date()) into
        // a plain string like "2026-03-23" which is then re-evaluated as
        // arithmetic by the code below.
        static BUILTIN_CALL_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^%(\w+)\(([^)]*)\)$").unwrap());
        if let Some(caps) = BUILTIN_CALL_RE.captures(expr) {
            let func_name = caps[1].to_string();
            let args_raw = caps[2].to_string();
            // Evaluate argument expressions so variables are resolved first.
            let evaluated = self.eval_builtin_with_expr_args(&func_name, &args_raw);
            return Value::from_str_auto(&evaluated);
        }

        // Before substituting variables, try to split at `+` on the original
        // expression.  This matters when one of the parts is a bare token_define
        // whose value contains quotes (e.g. `!define VERSION "2.0"`).  If we
        // substituted first we'd get `"v" + "2.0"` and the `"2.0"` would be
        // stripped of quotes; instead we want `"v" + VERSION` → `v` + `"2.0"`
        // → `v"2.0"` (PlantUML's text-substitution model).
        {
            let pre_parts = split_at_top_level_op(expr, '+');
            if pre_parts.len() > 1 {
                // Check whether any part is a bare token_define.
                let has_token_define = pre_parts.iter().any(|p| {
                    let t = p.trim();
                    BARE_WORD_RE.is_match(t) && self.token_defines.contains_key(t)
                });
                if has_token_define {
                    let mut result = String::new();
                    for part in &pre_parts {
                        let val = self.eval_expr_to_value(part.trim());
                        result.push_str(&val.to_display());
                    }
                    return Value::Str(result);
                }
            }
        }

        // Substitute variables first.
        let substituted = self.substitute_vars(expr);
        let substituted = substituted.trim();

        // Try to parse as a pure number.
        if let Ok(n) = substituted.parse::<i64>() {
            return Value::Int(n);
        }
        if let Ok(f) = substituted.parse::<f64>() {
            return Value::Float(f);
        }

        // Try string concatenation with +.
        if let Some(result) = self.try_eval_string_concat(substituted) {
            return result;
        }

        // Try arithmetic.
        if let Some(result) = self.try_eval_arithmetic(substituted) {
            return result;
        }

        // Function call: $funcName(args)
        if let Some(val) = self.try_eval_func_call(substituted) {
            return val;
        }

        // Fall back to string.
        Value::Str(substituted.to_string())
    }

    fn try_eval_func_call(&mut self, expr: &str) -> Option<Value> {
        // $funcName(args)
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\$(\w+)\(([^)]*)\)$").unwrap());

        if let Some(caps) = RE.captures(expr) {
            let name = caps[1].to_string();
            if self.functions.contains_key(&name) {
                let args_str = &caps[2];
                return Some(self.eval_function_call(&name, args_str));
            }
        }
        None
    }

    fn try_eval_string_concat(&mut self, expr: &str) -> Option<Value> {
        // Find top-level + that looks like string concatenation.
        // We check if it's NOT purely numeric.
        let parts = split_at_top_level_op(expr, '+');
        if parts.len() <= 1 {
            return None;
        }

        // Check if any part is a quoted string.
        let has_string = parts.iter().any(|p| {
            let t = p.trim();
            t.starts_with('"') || (t.parse::<f64>().is_err() && !t.starts_with('$'))
        });

        if has_string {
            let mut result = String::new();
            for part in &parts {
                let val = self.eval_expr_to_value(part.trim());
                result.push_str(&val.to_display());
            }
            return Some(Value::Str(result));
        }

        None
    }

    fn try_eval_arithmetic(&mut self, expr: &str) -> Option<Value> {
        // Find the lowest-precedence top-level operator.
        // Precedence (low to high): +/-, */%, unary-
        // Scan right-to-left for + and - first (to get left-associativity).
        let expr = expr.trim();

        // Try + and - (lowest precedence).
        for &op in &['+', '-'] {
            if let Some(pos) = find_top_level_op_pos(expr, op) {
                if pos == 0 {
                    continue; // unary minus
                }
                let left = &expr[..pos];
                let right = &expr[pos + 1..];
                let lval = self.eval_expr_to_value(left);
                let rval = self.eval_expr_to_value(right);

                // If either side is a non-numeric string, do string concat for +.
                if op == '+' && (!lval.is_numeric() || !rval.is_numeric()) {
                    return Some(Value::Str(format!(
                        "{}{}",
                        lval.to_display(),
                        rval.to_display()
                    )));
                }

                let result = match op {
                    '+' => lval.to_number() + rval.to_number(),
                    '-' => lval.to_number() - rval.to_number(),
                    _ => unreachable!(),
                };
                return Some(number_value(result));
            }
        }

        // Try *, /, % (higher precedence).
        for &op in &['*', '/', '%'] {
            if let Some(pos) = find_top_level_op_pos(expr, op) {
                let left = &expr[..pos];
                let right = &expr[pos + 1..];
                let lval = self.eval_expr_to_value(left);
                let rval = self.eval_expr_to_value(right);
                let result = match op {
                    '*' => lval.to_number() * rval.to_number(),
                    '/' => {
                        let r = rval.to_number();
                        if r == 0.0 { 0.0 } else { lval.to_number() / r }
                    }
                    '%' => {
                        let r = rval.to_number();
                        if r == 0.0 { 0.0 } else { lval.to_number() % r }
                    }
                    _ => unreachable!(),
                };
                return Some(number_value(result));
            }
        }

        // Parenthesized expression.
        if expr.starts_with('(') && expr.ends_with(')') {
            return Some(self.eval_expr_to_value(&expr[1..expr.len() - 1]));
        }

        // Function call.
        if let Some(val) = self.try_eval_func_call(expr) {
            return Some(val);
        }

        None
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

        // %variable_exists("name")
        if let Some(name) = expr
            .strip_prefix("%variable_exists(\"")
            .and_then(|s| s.strip_suffix("\")"))
        {
            return self.defines.contains_key(name) || self.token_defines.contains_key(name);
        }

        // %is_defined("name") — alias for variable_exists
        if let Some(name) = expr
            .strip_prefix("%is_defined(\"")
            .and_then(|s| s.strip_suffix("\")"))
        {
            return self.defines.contains_key(name) || self.token_defines.contains_key(name);
        }

        // %function_exists("name")
        if let Some(name) = expr
            .strip_prefix("%function_exists(\"")
            .and_then(|s| s.strip_suffix("\")"))
        {
            return self.functions.contains_key(name);
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

    fn substitute_vars(&self, line: &str) -> String {
        // Preserve sprite icon references `<$name>` — they are rendered as
        // inline images and must not be stripped or variable-substituted here.
        // Temporarily replace them with null-byte placeholders, process variable
        // substitution on the rest, then restore the sprite references.
        static SPRITE_REF_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<\$(\w+)>").unwrap());

        // Collect sprite ref names in order.
        let sprite_names: Vec<String> = SPRITE_REF_RE
            .captures_iter(line)
            .map(|c| c[1].to_string())
            .collect();

        // Replace each `<$name>` with a null-byte placeholder `\x00N\x00`.
        let mut counter = 0usize;
        let line_protected = SPRITE_REF_RE.replace_all(line, |_: &regex::Captures| {
            let ph = format!("\x00{counter}\x00");
            counter += 1;
            ph
        });
        let mut result = line_protected.into_owned();

        // First apply $variable substitution (also checks token_defines).
        result = self.substitute_inline_func_calls(&result);

        static VAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$(\w+)").unwrap());

        result = VAR_RE
            .replace_all(&result, |caps: &regex::Captures| {
                let name = &caps[1];
                self.get_var(name)
                    .or_else(|| self.token_defines.get(name))
                    .cloned()
                    .unwrap_or_else(|| format!("${name}"))
            })
            .to_string();

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

        let after_vars = result;

        // Evaluate built-in %functions.
        let mut processed = self.eval_builtin_functions(&after_vars);

        // Restore sprite references (replaced with placeholders above).
        for (i, name) in sprite_names.iter().enumerate() {
            let ph = format!("\x00{i}\x00");
            let sprite_tag = format!("<${name}>");
            processed = processed.replace(&ph, &sprite_tag);
        }

        processed
    }

    fn substitute_inline_func_calls(&self, line: &str) -> String {
        // We need &mut self to call functions, but we're in &self context here.
        // This is a limitation — inline function calls in output lines require
        // a different approach. For now, we handle it in eval_builtin_functions
        // for %string() etc. and rely on the caller to handle $func() calls
        // in assignment context.
        line.to_string()
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
            let from = &caps[2];
            let to = &caps[3];
            let label = caps[4].trim();
            if label.is_empty() {
                return Some(format!("{from} --> {to}"));
            } else {
                return Some(format!("{from} --> {to} : {label}"));
            }
        }

        if let Some(caps) = ELEM_RE.captures(line) {
            let id = &caps[3];
            let label = &caps[4];
            return Some(format!("rectangle \"{label}\" as {id}"));
        }

        None
    }

    // ------------------------------------------------------------------
    // Built-in %functions
    // ------------------------------------------------------------------

    fn eval_builtin_functions(&self, input: &str) -> String {
        // Handle nested function calls by repeatedly evaluating.
        let mut result = input.to_string();
        let mut iterations = 0;
        loop {
            let next = self.eval_builtins_once(&result);
            if next == result || iterations > 10 {
                break;
            }
            result = next;
            iterations += 1;
        }
        result
    }

    fn eval_builtins_once(&self, input: &str) -> String {
        // Find the innermost (last/rightmost) %func(...) call to evaluate
        // inside-out, so that nested calls like %not(%true()) work correctly.
        static FUNC_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%(\w+)\(").unwrap());

        let mut result = input.to_string();

        // Find the last (rightmost) %func( match — this is the innermost call
        // because the last match in a nested expression has no further %func(
        // calls before its closing paren.
        if let Some(m) = FUNC_RE.find_iter(&result).last() {
            let start = m.start();
            let after_open = m.end(); // position after '('
            if let Some(close) = find_matching_paren_from(&result, after_open - 1) {
                let func_name = &result[start + 1..after_open - 1];
                let args_raw = &result[after_open..close];
                let replacement = self.eval_one_builtin(func_name, args_raw);
                result = format!("{}{replacement}{}", &result[..start], &result[close + 1..]);
            }
        }

        result
    }

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
    fn eval_one_builtin_from_values(&self, func: &str, args: &[&str]) -> String {
        self.eval_one_builtin_args(func, args)
    }

    fn eval_one_builtin(&self, func: &str, args_raw: &str) -> String {
        let args: Vec<&str> = if args_raw.trim().is_empty() {
            Vec::new()
        } else {
            split_builtin_args(args_raw)
        };

        // Strip quotes from args.
        let args: Vec<&str> = args.iter().map(|a| a.trim().trim_matches('"')).collect();
        self.eval_one_builtin_args(func, &args)
    }

    fn eval_one_builtin_args(&self, func: &str, args: &[&str]) -> String {
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
            "date" => "2026-03-23".to_string(),
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
                let color_str = args.first().copied().unwrap_or("");
                let amount: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                if let Some((r, g, b)) = parse_color(color_str) {
                    let (h, s, l) = rgb_to_hsl(r, g, b);
                    let new_l = (l - amount).max(0.0);
                    let (r2, g2, b2) = hsl_to_rgb(h, s, new_l);
                    rgb_to_hex(r2, g2, b2)
                } else {
                    color_str.to_string()
                }
            }
            "lighten" => {
                let color_str = args.first().copied().unwrap_or("");
                let amount: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                if let Some((r, g, b)) = parse_color(color_str) {
                    let (h, s, l) = rgb_to_hsl(r, g, b);
                    let new_l = (l + amount).min(100.0);
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
                let name = args.first().copied().unwrap_or("");
                if self.defines.contains_key(name) || self.token_defines.contains_key(name) {
                    "true"
                } else {
                    "false"
                }
                .to_string()
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
                self.get_var(name)
                    .cloned()
                    .or_else(|| self.token_defines.get(name).cloned())
                    .unwrap_or_default()
            }
            "set_variable_value" => {
                // In expression context we can't mutate; handled specially.
                // Return empty.
                String::new()
            }
            "filename" => self
                .base_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
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

fn number_value(f: f64) -> Value {
    if f == (f as i64) as f64 && f.abs() < i64::MAX as f64 {
        Value::Int(f as i64)
    } else {
        Value::Float(f)
    }
}

fn parse_func_params(params_str: &str) -> Vec<FuncParam> {
    if params_str.trim().is_empty() {
        return Vec::new();
    }
    params_str
        .split(',')
        .map(|p| {
            let p = p.trim();
            // Check for default: $name = "value" or $name = value
            if let Some(eq_pos) = p.find('=') {
                let name = p[..eq_pos].trim().trim_start_matches('$').to_string();
                let default = p[eq_pos + 1..].trim().trim_matches('"').to_string();
                FuncParam {
                    name,
                    default: Some(default),
                }
            } else {
                FuncParam {
                    name: p.trim_start_matches('$').to_string(),
                    default: None,
                }
            }
        })
        .collect()
}

fn parse_call_args(args_str: &str) -> Vec<String> {
    if args_str.trim().is_empty() {
        return Vec::new();
    }
    let args = split_args(args_str);
    args.into_iter()
        .map(|a| a.trim().trim_matches('"').to_string())
        .collect()
}

fn split_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_quotes = false;

    for c in s.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
            current.push(c);
        } else if in_quotes {
            current.push(c);
        } else if c == '(' {
            depth += 1;
            current.push(c);
        } else if c == ')' {
            depth -= 1;
            current.push(c);
        } else if c == ',' && depth == 0 {
            args.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }
    args
}

fn split_builtin_args(s: &str) -> Vec<&str> {
    // Simple comma split respecting quotes and parens.
    let mut args = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut in_quotes = false;
    let bytes = s.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        if b == b'"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if b == b'(' {
                depth += 1;
            } else if b == b')' {
                depth -= 1;
            } else if b == b',' && depth == 0 {
                args.push(&s[start..i]);
                start = i + 1;
            }
        }
    }
    args.push(&s[start..]);
    args
}

fn find_matching_paren(s: &str) -> Option<usize> {
    // s starts with '('
    if !s.starts_with('(') {
        return None;
    }
    let mut depth = 0;
    let mut in_quotes = false;
    for (i, c) in s.chars().enumerate() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn find_matching_paren_from(s: &str, open_pos: usize) -> Option<usize> {
    let mut depth = 0;
    let mut in_quotes = false;
    for (i, c) in s[open_pos..].chars().enumerate() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + i);
                }
            }
        }
    }
    None
}

/// Find the rightmost top-level position of an operator (not inside parens or quotes).
fn find_top_level_op_pos(expr: &str, op: char) -> Option<usize> {
    let mut depth = 0;
    let mut in_quotes = false;
    let mut last_pos = None;
    let chars: Vec<char> = expr.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            } else if c == op && depth == 0 {
                // For -, don't match if it's part of ->
                if op == '-' && i + 1 < chars.len() && chars[i + 1] == '>' {
                    continue;
                }
                // Skip position 0: a leading operator character is a prefix
                // (e.g., leading '/' in '/users' is not division), not binary op.
                if i == 0 {
                    continue;
                }
                // Don't match if preceded by another operator (unary).
                let prev = chars[i - 1];
                if prev == '+'
                    || prev == '-'
                    || prev == '*'
                    || prev == '/'
                    || prev == '%'
                    || prev == '('
                    || prev == '='
                    || prev == '<'
                    || prev == '>'
                    || prev == '!'
                {
                    continue;
                }
                last_pos = Some(i);
            }
        }
    }
    last_pos
}

fn split_at_top_level_op(expr: &str, op: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut in_quotes = false;
    let mut current = String::new();

    for c in expr.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
            current.push(c);
        } else if in_quotes {
            current.push(c);
        } else if c == '(' {
            depth += 1;
            current.push(c);
        } else if c == ')' {
            depth -= 1;
            current.push(c);
        } else if c == op && depth == 0 {
            parts.push(current);
            current = String::new();
        } else {
            current.push(c);
        }
    }
    parts.push(current);
    parts
}

fn find_top_level_logical_op(expr: &str, op: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_quotes = false;
    let bytes = expr.as_bytes();
    let op_bytes = op.as_bytes();
    let mut last_pos = None;

    for i in 0..bytes.len() {
        if bytes[i] == b'"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if bytes[i] == b'(' {
                depth += 1;
            } else if bytes[i] == b')' {
                depth -= 1;
            } else if depth == 0
                && i + op_bytes.len() <= bytes.len()
                && &bytes[i..i + op_bytes.len()] == op_bytes
            {
                last_pos = Some(i);
            }
        }
    }
    last_pos
}

fn find_top_level_comparison(expr: &str, op: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_quotes = false;
    let bytes = expr.as_bytes();
    let op_bytes = op.as_bytes();

    // For == and !=, scan from left; for others, also from left.
    for i in 0..bytes.len() {
        if bytes[i] == b'"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if bytes[i] == b'(' {
                depth += 1;
            } else if bytes[i] == b')' {
                depth -= 1;
            } else if depth == 0
                && i + op_bytes.len() <= bytes.len()
                && &bytes[i..i + op_bytes.len()] == op_bytes
            {
                // For < and >, make sure it's not <= or >= or <> or <<.
                if op == "<"
                    && i + 1 < bytes.len()
                    && (bytes[i + 1] == b'=' || bytes[i + 1] == b'>')
                {
                    continue;
                }
                if op == ">" {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                        continue;
                    }
                    // Also skip -> (arrow).
                    if i > 0 && bytes[i - 1] == b'-' {
                        continue;
                    }
                }
                // For == make sure it's not part of !== or ===.
                return Some(i);
            }
        }
    }
    None
}

fn parens_balanced(s: &str) -> bool {
    let mut depth = 0;
    let mut in_quotes = false;
    for c in s.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
        }
    }
    depth == 0
}

/// Parse optional `[WxH/Z]` dimension spec from a sprite header line.
///
/// The `rest` parameter is the text after `sprite $` (name included).
/// Returns `(width, height)` where 0 means "infer from pixel data".
fn parse_sprite_dimensions(rest: &str) -> (u32, u32) {
    // Look for `[WxH/Z]` or `[WxH]` pattern anywhere in the rest string.
    if let Some(start) = rest.find('[') {
        if let Some(end) = rest[start..].find(']') {
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
    }
    (0, 0)
}

fn strip_inline_comment(line: &str) -> String {
    let mut in_quotes = false;
    let chars: Vec<char> = line.chars().collect();
    let bytes: Vec<usize> = line
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(line.len()))
        .collect();
    for (idx, &c) in chars.iter().enumerate() {
        let byte_pos = bytes[idx];
        if c == '"' {
            in_quotes = !in_quotes;
        } else if c == '\'' && !in_quotes {
            let preceded_by_space = idx == 0 || chars[idx - 1].is_whitespace();
            if preceded_by_space {
                return line[..byte_pos].to_string();
            }
        }
    }
    line.to_string()
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

    #[test]
    fn define_substitution() {
        let input = "@startuml\n!define NAME Alice\n$NAME -> Bob : hello\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello"]);
    }

    #[test]
    fn var_assignment() {
        let input = "@startuml\n!$name = \"World\"\nAlice -> $name : hi\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> World : hi"]);
    }

    #[test]
    fn ifdef_defined() {
        let input = "@startuml\n!define FEATURE\n!ifdef FEATURE\nAlice -> Bob\n!endif\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob"]);
    }

    #[test]
    fn ifdef_not_defined() {
        let input = "@startuml\n!ifdef FEATURE\nAlice -> Bob\n!endif\n@enduml";
        let lines = preprocess(input);
        assert!(lines.is_empty());
    }

    #[test]
    fn ifndef() {
        let input = "@startuml\n!ifndef FEATURE\nAlice -> Bob\n!endif\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob"]);
    }

    #[test]
    fn if_else() {
        let input = "@startuml\n!define MODE prod\n!if $MODE == \"prod\"\nA -> B : production\n!else\nA -> B : dev\n!endif\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["A -> B : production"]);
    }

    #[test]
    fn if_else_false_branch() {
        let input = "@startuml\n!define MODE dev\n!if $MODE == \"prod\"\nA -> B : production\n!else\nA -> B : dev\n!endif\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["A -> B : dev"]);
    }

    #[test]
    fn single_line_comment() {
        let input = "@startuml\n' This is a comment\nAlice -> Bob\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob"]);
    }

    #[test]
    fn block_comment() {
        let input = "@startuml\n/'\nThis is a\nblock comment\n'/\nAlice -> Bob\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob"]);
    }

    #[test]
    fn inline_comment() {
        let input = "@startuml\nAlice -> Bob : hello ' with comment\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello "]);
    }

    #[test]
    fn undef() {
        let input = "@startuml\n!define X yes\n!ifdef X\nA -> B\n!endif\n!undef X\n!ifdef X\nC -> D\n!endif\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn nested_conditionals() {
        let input =
            "@startuml\n!define A\n!define B\n!ifdef A\n!ifdef B\ndeep\n!endif\n!endif\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["deep"]);
    }

    #[test]
    fn nested_conditional_outer_false() {
        let input = "@startuml\n!ifdef MISSING\n!ifdef ALSO_MISSING\nnope\n!endif\n!endif\n@enduml";
        let lines = preprocess(input);
        assert!(lines.is_empty());
    }

    #[test]
    fn multiple_vars() {
        let input = "@startuml\n!define FROM Alice\n!define TO Bob\n!define MSG hello\n$FROM -> $TO : $MSG\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello"]);
    }

    #[test]
    fn empty_define() {
        let input = "@startuml\n!define FEATURE\n!ifdef FEATURE\nyes\n!endif\n@enduml";
        let lines = preprocess(input);
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

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn include_missing_file_is_silent() {
        let input = "@startuml\n!include nonexistent.puml\nA -> B\n@enduml";
        let lines = preprocess(input);
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
        let lines = preprocess(input);
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
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob : a", "Alice -> Bob : b"]);
    }

    #[test]
    fn foreach_preserves_other_vars() {
        let input = "@startuml\n!define WHO Alice\n!foreach $x in [\"hello\", \"world\"]\n$WHO -> Bob : $x\n!endfor\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello", "Alice -> Bob : world"]);
    }

    #[test]
    fn foreach_endforeach() {
        let input = "@startuml\n!foreach $x in [\"a\"]\nline $x\n!endforeach\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["line a"]);
    }

    #[test]
    fn function_basic() {
        let input = "@startuml\n!function $greet($name)\nAlice -> $name : hello\n!endfunction\n$greet(\"Bob\")\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob : hello"]);
    }

    #[test]
    fn function_multiple_params() {
        let input = "@startuml\n!function $msg($from, $to, $text)\n$from -> $to : $text\n!endfunction\n$msg(\"Alice\", \"Bob\", \"hi\")\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob : hi"]);
    }

    #[test]
    fn function_no_params() {
        let input = "@startuml\n!function $header()\ntitle My Diagram\n!endfunction\n$header()\nAlice -> Bob\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["title My Diagram", "Alice -> Bob"]);
    }

    #[test]
    fn function_called_multiple_times() {
        let input = "@startuml\n!function $arrow($to)\nAlice -> $to : msg\n!endfunction\n$arrow(\"Bob\")\n$arrow(\"Charlie\")\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["Alice -> Bob : msg", "Alice -> Charlie : msg"]);
    }

    #[test]
    fn procedure_syntax() {
        let input = "@startuml\n!procedure $setup($name)\nparticipant $name\n!endprocedure\n$setup(\"Alice\")\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["participant Alice"]);
    }

    // New tests for added features.

    #[test]
    fn variable_arithmetic() {
        let input = "@startuml\n!$x = 10\n!$y = 20\n!$sum = $x + $y\nnote : $sum\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["note : 30"]);
    }

    #[test]
    fn while_loop() {
        let input = "@startuml\n!$i = 1\n!while $i <= 3\nline $i\n!$i = $i + 1\n!endwhile\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["line 1", "line 2", "line 3"]);
    }

    #[test]
    fn elseif_branch() {
        let input = "@startuml\n!$x = 2\n!if $x == 1\nfirst\n!elseif $x == 2\nsecond\n!else\nother\n!endif\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["second"]);
    }

    #[test]
    fn pragma_consumed() {
        let input = "@startuml\n!pragma teoz true\nA -> B\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn function_return_value() {
        let input = "@startuml\n!function $double($n)\n!return $n * 2\n!endfunction\n!$x = $double(5)\nnote : $x\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["note : 10"]);
    }

    #[test]
    fn numeric_comparison() {
        let input = "@startuml\n!$x = 5\n!if $x > 3\nyes\n!endif\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["yes"]);
    }

    // ---------------------------------------------------------------------------
    // Tests for !includeurl, !import, !dump_memory (new directives)
    // ---------------------------------------------------------------------------

    #[test]
    fn includeurl_is_stripped_silently() {
        // !includeurl should consume the line without producing output or error.
        let input = "@startuml\n!includeurl https://example.com/common.puml\nA -> B\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn import_is_stripped_silently() {
        // !import is an alias for !includeurl — also silently consumed.
        let input = "@startuml\n!import https://example.com/lib.puml\nA -> B\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    #[test]
    fn dump_memory_no_vars() {
        // With no defines, !dump_memory emits an empty dump comment.
        let input = "@startuml\n!dump_memory\nA -> B\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["' [dump] ", "A -> B"]);
    }

    #[test]
    fn dump_memory_with_vars() {
        let input = "@startuml\n!$foo = \"bar\"\n!$baz = \"qux\"\n!dump_memory\n@enduml";
        let lines = preprocess(input);
        // Output must contain the dump comment. Variable order is sorted.
        assert_eq!(lines.len(), 1);
        let dump = &lines[0];
        assert!(dump.starts_with("' [dump] "), "got: {dump}");
        assert!(dump.contains("baz=qux"), "got: {dump}");
        assert!(dump.contains("foo=bar"), "got: {dump}");
    }

    #[test]
    fn dump_memory_inactive_branch() {
        // !dump_memory inside an inactive branch should produce no output.
        let input = "@startuml\n!ifdef MISSING\n!dump_memory\n!endif\nA -> B\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["A -> B"]);
    }

    // ---------------------------------------------------------------------------
    // Tests for new built-in functions
    // ---------------------------------------------------------------------------

    #[test]
    fn builtin_float() {
        let input = "@startuml\n!$v = %float(\"3.14\")\nnote : $v\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["note : 3.14"]);
    }

    #[test]
    fn builtin_float_integer_string() {
        let input = "@startuml\n!$v = %float(\"42\")\nnote : $v\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["note : 42"]);
    }

    #[test]
    fn builtin_dec2hex() {
        let input = "@startuml\n!$h = %dec2hex(255)\nnote : $h\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["note : ff"]);
    }

    #[test]
    fn builtin_dec2hex_zero() {
        let input = "@startuml\n!$h = %dec2hex(0)\nnote : $h\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["note : 0"]);
    }

    #[test]
    fn builtin_hex2dec() {
        let input = "@startuml\n!$d = %hex2dec(\"ff\")\nnote : $d\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["note : 255"]);
    }

    #[test]
    fn builtin_hex2dec_zero() {
        let input = "@startuml\n!$d = %hex2dec(\"0\")\nnote : $d\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["note : 0"]);
    }

    #[test]
    fn builtin_dirpath_no_base() {
        // Without a base dir, %dirpath() returns ".".
        let input = "@startuml\n!$p = %dirpath()\nnote : $p\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["note : ."]);
    }

    #[test]
    fn builtin_dirpath_with_base() {
        let dir = std::env::temp_dir().join("rustuml_test_dirpath");
        let _ = std::fs::create_dir_all(&dir);
        let input = "@startuml\n!$p = %dirpath()\nnote : $p\n@enduml";
        let lines = preprocess_with_base(input, &dir);
        assert_eq!(lines, vec![format!("note : {}", dir.display())]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn builtin_feature_returns_false() {
        // All features are unsupported; always returns "false".
        let input = "@startuml\n!$f = %feature(\"dark-mode\")\nnote : $f\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["note : false"]);
    }

    #[test]
    fn builtin_feature_in_conditional() {
        // %feature() returning "false" means the ifdef branch is inactive.
        let input = "@startuml\n!if %feature(\"dark-mode\") == \"true\"\ndark\n!else\nlight\n!endif\n@enduml";
        let lines = preprocess(input);
        assert_eq!(lines, vec!["light"]);
    }

    #[test]
    fn sprite_block_is_collected() {
        let input =
            "@startuml\nsprite $disk [8x8/16] {\nFF00FF00\n00FF00FF\n}\nnote : hello\n@enduml";
        let out = preprocess_full(input, None);
        // The sprite block should NOT appear in the preprocessed lines.
        assert!(
            !out.lines.iter().any(|l| l.contains("FF00FF00")),
            "sprite pixel rows should not appear in output lines"
        );
        assert_eq!(out.lines, vec!["note : hello"]);
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
        assert_eq!(out.lines, vec!["note : <$icon> hello"]);
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
}
