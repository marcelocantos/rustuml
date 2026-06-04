// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Regex railroad diagram parser.

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::regex_diagram::{GroupKind, RegexDiagram, RegexNode};

/// Parse pre-processed lines into a [`RegexDiagram`].
pub fn parse_regex_diagram(lines: &[String]) -> Result<RegexDiagram, ParseError> {
    let mut meta = DiagramMeta::default();
    let mut pattern_lines = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("title ") {
            meta.title = Some(super::strip_title_quotes(rest).to_string());
            meta.title_line = Some(idx + 1);
            continue;
        }
        pattern_lines.push(line.as_str());
    }

    let pattern = pattern_lines.join("");

    let pattern = pattern.trim().to_string();

    let ast = if pattern.is_empty() {
        RegexNode::Sequence { items: vec![] }
    } else {
        let mut parser = Parser::new(&pattern);
        let node = parser.parse_alternation();
        // Flatten trivial sequences/alternations
        simplify(node)
    };

    Ok(RegexDiagram { meta, pattern, ast })
}

fn simplify(node: RegexNode) -> RegexNode {
    match node {
        RegexNode::Sequence { items } => {
            let items: Vec<_> = items.into_iter().map(simplify).collect();
            if items.len() == 1 {
                items.into_iter().next().unwrap()
            } else {
                RegexNode::Sequence { items }
            }
        }
        RegexNode::Alternation { branches } => {
            let branches: Vec<_> = branches.into_iter().map(simplify).collect();
            if branches.len() == 1 {
                branches.into_iter().next().unwrap()
            } else {
                RegexNode::Alternation { branches }
            }
        }
        RegexNode::Repeat { inner, min, max } => RegexNode::Repeat {
            inner: Box::new(simplify(*inner)),
            min,
            max,
        },
        RegexNode::Group { kind, inner } => RegexNode::Group {
            kind,
            inner: Box::new(simplify(*inner)),
        },
        other => other,
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(s: &str) -> Self {
        // Handle (?x) verbose mode: strip whitespace and comments
        let s = if s.starts_with("(?x)") || s.starts_with("(?X)") {
            s[4..]
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
        } else {
            s.to_string()
        };
        Self {
            chars: s.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn try_consume(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn parse_alternation(&mut self) -> RegexNode {
        let mut branches = vec![self.parse_sequence()];
        while self.try_consume('|') {
            branches.push(self.parse_sequence());
        }
        if branches.len() == 1 {
            branches.remove(0)
        } else {
            RegexNode::Alternation { branches }
        }
    }

    fn parse_sequence(&mut self) -> RegexNode {
        let mut items = Vec::new();
        loop {
            match self.peek() {
                None | Some(')') | Some('|') => break,
                _ => {
                    if let Some(node) = self.parse_quantified() {
                        items.push(node);
                    } else {
                        break;
                    }
                }
            }
        }
        if items.len() == 1 {
            items.remove(0)
        } else {
            RegexNode::Sequence { items }
        }
    }

    fn parse_quantified(&mut self) -> Option<RegexNode> {
        let atom = self.parse_atom()?;
        // Parse optional quantifier
        let quant = match self.peek() {
            Some('*') => {
                self.advance();
                self.consume_lazy();
                Some((0u32, None))
            }
            Some('+') => {
                self.advance();
                self.consume_lazy();
                Some((1u32, None))
            }
            Some('?') => {
                self.advance();
                self.consume_lazy();
                Some((0u32, Some(1u32)))
            }
            Some('{') => self.parse_counted_quantifier(),
            _ => None,
        };
        Some(if let Some((min, max)) = quant {
            RegexNode::Repeat {
                inner: Box::new(atom),
                min,
                max,
            }
        } else {
            atom
        })
    }

    fn consume_lazy(&mut self) {
        // Consume optional lazy modifier '?'
        if self.peek() == Some('?') {
            self.pos += 1;
        }
    }

    fn parse_counted_quantifier(&mut self) -> Option<(u32, Option<u32>)> {
        // Peek ahead to see if this is a valid {n} or {n,m}
        let start = self.pos;
        if self.peek() != Some('{') {
            return None;
        }
        self.pos += 1; // consume '{'
        let min = self.parse_digits()?;
        let max = if self.try_consume(',') {
            if self.peek() == Some('}') {
                None // {n,} = unlimited
            } else {
                Some(self.parse_digits()?)
            }
        } else {
            Some(min)
        };
        if !self.try_consume('}') {
            // Not a valid quantifier, backtrack
            self.pos = start;
            return None;
        }
        self.consume_lazy();
        Some((min, max))
    }

    fn parse_digits(&mut self) -> Option<u32> {
        let mut s = String::new();
        while matches!(self.peek(), Some('0'..='9')) {
            s.push(self.advance().unwrap());
        }
        if s.is_empty() { None } else { s.parse().ok() }
    }

    fn parse_atom(&mut self) -> Option<RegexNode> {
        match self.peek()? {
            '(' => Some(self.parse_group()),
            '[' => Some(self.parse_class()),
            '.' => {
                self.advance();
                Some(RegexNode::Special {
                    text: ".".to_string(),
                })
            }
            '^' => {
                self.advance();
                Some(RegexNode::Special {
                    text: "^".to_string(),
                })
            }
            '$' => {
                self.advance();
                Some(RegexNode::Special {
                    text: "$".to_string(),
                })
            }
            '\\' => Some(self.parse_escape()),
            c if c == ')' || c == '|' || c == '{' || c == '*' || c == '+' || c == '?' => None,
            _ => Some(self.parse_literal()),
        }
    }

    fn parse_literal(&mut self) -> RegexNode {
        let mut text = String::new();
        loop {
            match self.peek() {
                Some(c)
                    if c != '('
                        && c != ')'
                        && c != '['
                        && c != ']'
                        && c != '.'
                        && c != '^'
                        && c != '$'
                        && c != '\\'
                        && c != '|'
                        && c != '*'
                        && c != '+'
                        && c != '?'
                        && c != '{' =>
                {
                    // A quantifier binds only to the single preceding character.
                    // If the *next* character is a quantifier and we already have
                    // text accumulated, stop so this character starts a fresh atom
                    // (e.g. `ab+` → literal `a` then `b+`, not `ab+`).
                    if !text.is_empty() && matches!(self.peek2(), Some('*' | '+' | '?' | '{')) {
                        break;
                    }
                    text.push(c);
                    self.pos += 1;
                }
                _ => break,
            }
        }
        RegexNode::Literal { text }
    }

    fn parse_escape(&mut self) -> RegexNode {
        self.advance(); // consume '\'
        match self.advance() {
            Some(c @ ('d' | 'D' | 'w' | 'W' | 's' | 'S' | 'b' | 'B')) => RegexNode::Special {
                text: format!("\\{c}"),
            },
            Some(c @ ('1'..='9')) => RegexNode::Special {
                text: format!("\\{c}"),
            },
            Some('n') => RegexNode::Special {
                text: "\\n".to_string(),
            },
            Some('t') => RegexNode::Special {
                text: "\\t".to_string(),
            },
            Some('r') => RegexNode::Special {
                text: "\\r".to_string(),
            },
            // PlantUML's `isEscapedChar` set: these become ESCAPED_CHAR tokens
            // rendered as plain terminal boxes showing only the unescaped char
            // (e.g. `\.` -> a literal box containing `.`).
            Some(
                c @ ('.' | '*' | '\\' | '?' | '^' | '$' | '|' | '(' | ')' | '[' | ']' | '{' | '}'
                | '<' | '>'),
            ) => RegexNode::Literal {
                text: c.to_string(),
            },
            Some(c) => {
                // Any other escape (e.g. `\+`, `\/`) is treated as a metacharacter
                // class and rendered as a gray special box showing the backslash.
                RegexNode::Special {
                    text: format!("\\{c}"),
                }
            }
            None => RegexNode::Literal {
                text: "\\".to_string(),
            },
        }
    }

    fn parse_group(&mut self) -> RegexNode {
        self.advance(); // consume '('
        let kind = self.parse_group_kind();
        let inner = self.parse_alternation();
        self.try_consume(')');
        RegexNode::Group {
            kind,
            inner: Box::new(inner),
        }
    }

    fn parse_group_kind(&mut self) -> GroupKind {
        if self.peek() != Some('?') {
            return GroupKind::Capture;
        }
        self.pos += 1; // consume '?'
        match self.peek() {
            Some(':') => {
                self.pos += 1;
                GroupKind::NonCapture
            }
            Some('=') => {
                self.pos += 1;
                GroupKind::Lookahead { positive: true }
            }
            Some('!') => {
                self.pos += 1;
                GroupKind::Lookahead { positive: false }
            }
            Some('<') => {
                self.pos += 1;
                match self.peek() {
                    Some('=') => {
                        self.pos += 1;
                        GroupKind::Lookbehind { positive: true }
                    }
                    Some('!') => {
                        self.pos += 1;
                        GroupKind::Lookbehind { positive: false }
                    }
                    _ => {
                        // Named group (?<name>...)
                        let name = self.read_until('>');
                        self.try_consume('>');
                        GroupKind::Named { name }
                    }
                }
            }
            Some('P') => {
                self.pos += 1;
                if self.try_consume('<') {
                    let name = self.read_until('>');
                    self.try_consume('>');
                    GroupKind::Named { name }
                } else {
                    GroupKind::NonCapture
                }
            }
            Some(c) if "imsx".contains(c) => {
                let flags = self.read_flags();
                GroupKind::Flags { flags }
            }
            _ => GroupKind::NonCapture,
        }
    }

    fn read_flags(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if "imsx)".contains(c) {
                if c != ')' {
                    s.push(c);
                    self.pos += 1;
                } else {
                    // consume the ')' for flag-only groups like (?i)
                    self.pos += 1;
                    break;
                }
            } else {
                break;
            }
        }
        s
    }

    fn read_until(&mut self, end: char) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == end {
                break;
            }
            s.push(c);
            self.pos += 1;
        }
        s
    }

    fn parse_class(&mut self) -> RegexNode {
        self.advance(); // consume '['

        // Collect the raw group content (the characters up to the closing `]`),
        // keeping the leading negation `^` and escape sequences intact — this
        // mirrors PlantUML's GroupSplitter input.
        let mut group = String::new();
        loop {
            match self.peek() {
                None | Some(']') => {
                    self.try_consume(']');
                    break;
                }
                Some('\\') => {
                    self.advance();
                    group.push('\\');
                    if let Some(c) = self.advance() {
                        group.push(c);
                    }
                }
                Some(c) => {
                    self.advance();
                    group.push(c);
                }
            }
        }

        // Split into rendered items following PlantUML's GroupSplitter: a
        // range `x-y` (when a `-` separates two chars), an escape `\x`, or a
        // single character.
        let chars: Vec<char> = group.chars().collect();
        let mut items: Vec<String> = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if i + 2 < chars.len() && chars[i + 1] == '-' {
                items.push(chars[i..i + 3].iter().collect());
                i += 3;
            } else if i + 1 < chars.len() && chars[i] == '\\' {
                items.push(chars[i..i + 2].iter().collect());
                i += 2;
            } else {
                items.push(chars[i].to_string());
                i += 1;
            }
        }

        if items.is_empty() {
            items.push(String::new());
        }

        let negated = items.first().map(String::as_str) == Some("^");
        RegexNode::CharClass { items, negated }
    }
}
