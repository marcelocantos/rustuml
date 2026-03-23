// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Regex railroad diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A regex railroad diagram produced by `@startregex`/`@endregex`.
#[derive(Debug, Serialize, Deserialize)]
pub struct RegexDiagram {
    pub meta: DiagramMeta,
    /// The raw regex pattern.
    pub pattern: String,
    /// The parsed AST.
    pub ast: RegexNode,
}

/// A node in the regex AST.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "node_kind")]
pub enum RegexNode {
    /// A literal string, e.g. `abc`. Rendered as a plain white rectangle.
    Literal { text: String },
    /// A special/metacharacter: `.`, `^`, `$`, `\d`, `\w`, `\s`, `\b`, `\1`, etc.
    /// Rendered as a gray rounded rectangle.
    Special { text: String },
    /// A character class `[...]`. Items are the pieces inside the brackets.
    CharClass { items: Vec<String> },
    /// A sequence of nodes rendered left-to-right.
    Sequence { items: Vec<RegexNode> },
    /// Alternation: one of several branches.
    Alternation { branches: Vec<RegexNode> },
    /// Repetition: quantify an inner node.
    Repeat {
        inner: Box<RegexNode>,
        min: u32,
        max: Option<u32>, // None = unlimited
    },
    /// A group (capturing, non-capturing, named, lookahead, lookbehind, flags).
    Group {
        kind: GroupKind,
        inner: Box<RegexNode>,
    },
}

/// The kind of a group node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "group_variant")]
pub enum GroupKind {
    Capture,
    NonCapture,
    Named { name: String },
    Lookahead { positive: bool },
    Lookbehind { positive: bool },
    Flags { flags: String },
}
