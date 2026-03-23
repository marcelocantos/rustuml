// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! TIM preprocessor — handles variables, defines, conditionals,
//! includes, and comments before diagram-specific parsing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

/// Preprocess PlantUML source, expanding TIM directives.
///
/// Handles:
/// - `!define NAME VALUE` / `!$var = value` — variable definitions
/// - `$variable` substitution in lines
/// - `!if`, `!else`, `!endif` — conditional blocks
/// - `!ifdef`, `!ifndef` — existence checks
/// - `!include <path>` — file inclusion (relative to base_dir)
/// - `' single-line comments` and `/' ... '/` block comments
/// - Strips `@startuml`/`@enduml` tags
pub fn preprocess(input: &str) -> Vec<String> {
    let mut ctx = PreprocessContext::new(None);
    ctx.process(input)
}

/// Preprocess with a base directory for resolving `!include` paths.
pub fn preprocess_with_base(input: &str, base_dir: &Path) -> Vec<String> {
    let mut ctx = PreprocessContext::new(Some(base_dir.to_path_buf()));
    ctx.process(input)
}

struct PreprocessContext {
    defines: HashMap<String, String>,
    cond_stack: Vec<CondState>,
    in_block_comment: bool,
    base_dir: Option<PathBuf>,
    include_depth: usize,
    foreach_stack: Vec<ForEachState>,
    functions: HashMap<String, FunctionDef>,
    collecting_function: Option<String>,
}

const MAX_INCLUDE_DEPTH: usize = 10;

struct CondState {
    active: bool,
    has_matched: bool,
}

struct ForEachState {
    var_name: String,
    values: Vec<String>,
    body_lines: Vec<String>,
}

#[derive(Clone)]
struct FunctionDef {
    params: Vec<String>,
    body: Vec<String>,
}

impl PreprocessContext {
    fn new(base_dir: Option<PathBuf>) -> Self {
        Self {
            defines: HashMap::new(),
            cond_stack: Vec::new(),
            in_block_comment: false,
            base_dir,
            include_depth: 0,
            foreach_stack: Vec::new(),
            functions: HashMap::new(),
            collecting_function: None,
        }
    }

    fn is_active(&self) -> bool {
        self.cond_stack.iter().all(|c| c.active)
    }

    fn process(&mut self, input: &str) -> Vec<String> {
        let mut output = Vec::new();

        for line in input.lines() {
            let trimmed = line.trim();

            // Handle block comments.
            if self.in_block_comment {
                if trimmed.contains("'/") {
                    self.in_block_comment = false;
                }
                continue;
            }
            if trimmed.starts_with("/'") {
                if !trimmed.contains("'/") || trimmed.ends_with("/'") {
                    self.in_block_comment = true;
                }
                continue;
            }

            // Skip single-line comments.
            if trimmed.starts_with('\'') {
                continue;
            }

            // Strip inline comments.
            let line_no_comment = strip_inline_comment(line);
            let trimmed = line_no_comment.trim();

            // Skip @start/@end tags.
            if trimmed.starts_with("@start") || trimmed.starts_with("@end") {
                continue;
            }

            // Function definition collection — must be checked first.
            if self.try_function_def(trimmed) {
                continue;
            }

            // Foreach buffering — must be checked before other directives.
            if self.try_foreach(trimmed, &mut output) {
                continue;
            }

            // Function invocation.
            if self.try_function_call(trimmed, &mut output) {
                continue;
            }

            // Process TIM directives.
            if self.try_define(trimmed) {
                continue;
            }
            if self.try_conditional(trimmed) {
                continue;
            }
            if self.try_undefine(trimmed) {
                continue;
            }
            if let Some(included_lines) = self.try_include(trimmed) {
                if self.is_active() {
                    output.extend(included_lines);
                }
                continue;
            }
            if self.try_theme(trimmed, &mut output) {
                continue;
            }

            // Only output lines when all conditions are active.
            if self.is_active() {
                let expanded = self.substitute_vars(&line_no_comment);
                if !expanded.trim().is_empty() {
                    output.push(expanded);
                }
            }
        }

        output
    }

    fn try_define(&mut self, line: &str) -> bool {
        static RE_DEFINE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!define\s+(\w+)(?:\s+(.+))?$").unwrap());
        static RE_VAR: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!\$(\w+)\s*=\s*(.+)$").unwrap());

        if let Some(caps) = RE_DEFINE.captures(line) {
            if self.is_active() {
                let name = caps[1].to_string();
                let value = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
                self.defines.insert(name, value);
            }
            return true;
        }
        if let Some(caps) = RE_VAR.captures(line) {
            if self.is_active() {
                let name = caps[1].to_string();
                let value = caps[2].trim().trim_matches('"').to_string();
                self.defines.insert(name, value);
            }
            return true;
        }
        false
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

        // !function $name($param1, $param2)
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^!(?:function|procedure)\s+\$(\w+)\s*\(([^)]*)\)$").unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let params: Vec<String> = if caps[2].trim().is_empty() {
                Vec::new()
            } else {
                caps[2]
                    .split(',')
                    .map(|p| p.trim().trim_start_matches('$').to_string())
                    .collect()
            };

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

    fn try_function_call(&mut self, line: &str, output: &mut Vec<String>) -> bool {
        // $funcName("arg1", "arg2")
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\$(\w+)\s*\(([^)]*)\)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let name = &caps[1];
            let args_str = &caps[2];

            let func = match self.functions.get(name) {
                Some(f) => f.clone(),
                None => return false,
            };

            if !self.is_active() {
                return true;
            }

            let args: Vec<String> = if args_str.trim().is_empty() {
                Vec::new()
            } else {
                args_str
                    .split(',')
                    .map(|a| a.trim().trim_matches('"').to_string())
                    .collect()
            };

            // Save current values, set params.
            let mut saved: Vec<(String, Option<String>)> = Vec::new();
            for (i, param) in func.params.iter().enumerate() {
                saved.push((param.clone(), self.defines.get(param).cloned()));
                if let Some(arg) = args.get(i) {
                    self.defines.insert(param.clone(), arg.clone());
                }
            }

            // Expand body lines.
            for body_line in &func.body {
                let expanded = self.substitute_vars(body_line);
                if !expanded.trim().is_empty() {
                    output.push(expanded);
                }
            }

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

            return true;
        }

        false
    }

    fn try_foreach(&mut self, line: &str, output: &mut Vec<String>) -> bool {
        // !foreach $var in ["a", "b", "c"]
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^!foreach\s+\$(\w+)\s+in\s+\[(.+)\]$"#).unwrap());

        if let Some(caps) = RE.captures(line) {
            if !self.is_active() {
                return true;
            }
            let var_name = caps[1].to_string();
            let values_str = &caps[2];

            // Parse the values list (comma-separated, possibly quoted).
            let values: Vec<String> = values_str
                .split(',')
                .map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string())
                .collect();

            // Collect body lines until !endfor.
            // This is a simplified implementation that doesn't support nesting.
            // For now, we store the foreach state and handle it in process().
            // Actually, since we process line-by-line, we need a different approach.
            // Store the foreach and buffer lines until !endfor.
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
                // Expand: for each value, substitute and process body lines.
                for val in &foreach.values {
                    let old_val = self.defines.get(&foreach.var_name).cloned();
                    self.defines.insert(foreach.var_name.clone(), val.clone());

                    for body_line in &foreach.body_lines {
                        let expanded = self.substitute_vars(body_line);
                        if !expanded.trim().is_empty() {
                            output.push(expanded);
                        }
                    }

                    // Restore previous value.
                    match old_val {
                        Some(v) => {
                            self.defines.insert(foreach.var_name.clone(), v);
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

    fn try_include(&mut self, line: &str) -> Option<Vec<String>> {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^!include(?:_once)?\s+(?:"([^"]+)"|(\S+))$"#).unwrap());

        let caps = RE.captures(line)?;
        let path_str = caps.get(1).or(caps.get(2)).map(|m| m.as_str())?;

        if self.include_depth >= MAX_INCLUDE_DEPTH {
            // Prevent infinite recursion.
            return Some(vec![format!(
                "' WARNING: max include depth ({MAX_INCLUDE_DEPTH}) reached for {path_str}"
            )]);
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
            Err(_) => {
                // Silently skip missing includes (matches PlantUML behavior
                // for optional includes).
                Some(vec![])
            }
        }
    }

    fn try_theme(&self, line: &str, output: &mut Vec<String>) -> bool {
        if let Some(rest) = line.strip_prefix("!theme ") {
            let theme_name = rest.trim();
            if self.is_active() && !theme_name.is_empty() {
                // Emit as a synthetic skinparam for the renderer to pick up.
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
            self.defines.remove(&caps[1]);
            return true;
        }
        false
    }

    fn try_conditional(&mut self, line: &str) -> bool {
        static RE_IF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^!if\s+(.+)$"#).unwrap());
        static RE_IFDEF: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!ifdef\s+(\w+)$").unwrap());
        static RE_IFNDEF: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^!ifndef\s+(\w+)$").unwrap());

        if let Some(caps) = RE_IFDEF.captures(line) {
            let defined = self.defines.contains_key(&caps[1]);
            self.cond_stack.push(CondState {
                active: defined && self.is_active(),
                has_matched: defined,
            });
            return true;
        }
        if let Some(caps) = RE_IFNDEF.captures(line) {
            let not_defined = !self.defines.contains_key(&caps[1]);
            self.cond_stack.push(CondState {
                active: not_defined && self.is_active(),
                has_matched: not_defined,
            });
            return true;
        }
        if let Some(caps) = RE_IF.captures(line) {
            let expr = caps[1].trim();
            let result = self.eval_condition(expr);
            self.cond_stack.push(CondState {
                active: result && self.is_active(),
                has_matched: result,
            });
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

    fn eval_condition(&self, expr: &str) -> bool {
        // Simple evaluation: check for variable comparisons and existence.
        let expr = expr.trim();

        // $var == "value"
        static RE_EQ: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^\$(\w+)\s*==\s*"([^"]*)"$"#).unwrap());
        static RE_NEQ: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^\$(\w+)\s*!=\s*"([^"]*)"$"#).unwrap());

        if let Some(caps) = RE_EQ.captures(expr) {
            let var = &caps[1];
            let expected = &caps[2];
            return self.defines.get(var).is_some_and(|v| v == expected);
        }
        if let Some(caps) = RE_NEQ.captures(expr) {
            let var = &caps[1];
            let expected = &caps[2];
            return self.defines.get(var).is_none_or(|v| v != expected);
        }

        // %variable_exists("name")
        if let Some(name) = expr
            .strip_prefix("%variable_exists(\"")
            .and_then(|s| s.strip_suffix("\")"))
        {
            return self.defines.contains_key(name);
        }

        // Bare variable existence: %true / %false
        if expr == "%true" {
            return true;
        }
        if expr == "%false" {
            return false;
        }

        // Default: treat non-empty as true.
        !expr.is_empty()
    }

    fn substitute_vars(&self, line: &str) -> String {
        static VAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$(\w+)").unwrap());

        let after_vars = VAR_RE
            .replace_all(line, |caps: &regex::Captures| {
                let name = &caps[1];
                self.defines
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| format!("${name}"))
            })
            .to_string();

        // Evaluate built-in %functions.
        eval_builtin_functions(&after_vars)
    }
}

/// Evaluate built-in %functions in a string.
fn eval_builtin_functions(input: &str) -> String {
    static FUNC_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%(\w+)\(([^)]*)\)").unwrap());

    FUNC_RE
        .replace_all(input, |caps: &regex::Captures| {
            let func = &caps[1];
            let args_raw = &caps[2];
            let args: Vec<&str> = if args_raw.trim().is_empty() {
                Vec::new()
            } else {
                args_raw
                    .split(',')
                    .map(|a| a.trim().trim_matches('"'))
                    .collect()
            };

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
                        .unwrap_or(s.len() - start);
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
                "true" => "true".to_string(),
                "false" => "false".to_string(),
                "date" => {
                    // Simplified: return ISO date.
                    "2026-03-22".to_string()
                }
                "size" => args
                    .first()
                    .map_or("0".to_string(), |s| s.len().to_string()),
                "string" => args.first().map_or(String::new(), |s| s.to_string()),
                "intval" => args
                    .first()
                    .and_then(|s| s.parse::<i64>().ok())
                    .map_or("0".to_string(), |n| n.to_string()),
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
                "variable_exists" => {
                    // This is used in !if conditions, already handled there.
                    // In regular text, just return the string.
                    format!("%variable_exists({})", args_raw)
                }
                _ => {
                    // Unknown function — pass through.
                    format!("%{func}({args_raw})")
                }
            }
        })
        .to_string()
}

fn strip_inline_comment(line: &str) -> String {
    // PlantUML inline comments start with ' but not inside strings.
    // ' starts a comment only when at position 0 (after trim) or preceded by
    // whitespace. This avoids stripping possessives like [Task 1]'s end.
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
            // Only treat as comment start if at position 0 or preceded by whitespace.
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
        // Create a temp file for inclusion.
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
}
