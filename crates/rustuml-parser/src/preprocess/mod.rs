// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! TIM preprocessor — handles variables, defines, conditionals,
//! includes, and comments before diagram-specific parsing.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

/// Preprocess PlantUML source, expanding TIM directives.
///
/// Handles:
/// - `!define NAME VALUE` / `!$var = value` — variable definitions
/// - `$variable` substitution in lines
/// - `!if`, `!else`, `!endif` — conditional blocks
/// - `!ifdef`, `!ifndef` — existence checks
/// - `' single-line comments` and `/' ... '/` block comments
/// - Strips `@startuml`/`@enduml` tags
pub fn preprocess(input: &str) -> Vec<String> {
    let mut ctx = PreprocessContext::new();
    ctx.process(input)
}

struct PreprocessContext {
    defines: HashMap<String, String>,
    /// Stack of conditional states. Each entry is (active, has_matched).
    /// `active`: whether we're currently outputting lines.
    /// `has_matched`: whether any branch of this if/elseif/else has been taken.
    cond_stack: Vec<CondState>,
    in_block_comment: bool,
}

struct CondState {
    active: bool,
    has_matched: bool,
}

impl PreprocessContext {
    fn new() -> Self {
        Self {
            defines: HashMap::new(),
            cond_stack: Vec::new(),
            in_block_comment: false,
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
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$(\w+)").unwrap());

        RE.replace_all(line, |caps: &regex::Captures| {
            let name = &caps[1];
            self.defines
                .get(name)
                .cloned()
                .unwrap_or_else(|| format!("${name}"))
        })
        .to_string()
    }
}

fn strip_inline_comment(line: &str) -> String {
    // PlantUML inline comments start with ' but not inside strings.
    // Simple heuristic: strip from first ' that's not inside quotes.
    let mut in_quotes = false;
    for (i, c) in line.char_indices() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if c == '\'' && !in_quotes {
            return line[..i].to_string();
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
}
