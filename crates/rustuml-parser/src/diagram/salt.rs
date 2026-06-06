// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Salt (UI wireframe) diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A Salt diagram produced by `@startsalt`/`@endsalt`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SaltDiagram {
    pub meta: DiagramMeta,
    /// The root widget block.
    pub root: SaltBlock,
}

/// A Salt container block, delimited by `{...}`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SaltBlock {
    pub kind: BlockKind,
    /// Title for group-box blocks (`{^Title`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Rows of cells within the block.
    pub rows: Vec<SaltRow>,
}

/// The kind of block, determined by the character(s) after `{`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    /// Plain container: `{`.
    Plain,
    /// Container with an outer border only: `{+` (DRAW_OUTSIDE).
    Frame,
    /// Table with visible grid lines: `{#`.
    Table,
    /// Tree widget: `{T`.
    Tree,
    /// Menu widget: `{*`.
    Menu,
    /// Tab bar: `{/`.
    Tabs,
    /// Scrollable container with both scrollbars: `{S`.
    Scroll,
    /// Scrollable container, vertical scrollbar only: `{SI`.
    ScrollInput,
    /// Scrollable container, horizontal scrollbar only: `{S-`.
    ScrollHorizontal,
}

/// A row of cells within a block, separated by `|`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SaltRow {
    pub cells: Vec<SaltWidget>,
}

/// An individual Salt widget or nested block.
#[derive(Debug, Serialize, Deserialize)]
pub enum SaltWidget {
    /// A nested block `{...}` acting as a cell in a grid row.
    Block(Box<SaltBlock>),
    /// A push button: `[Label]`.
    Button(String),
    /// A text input field: `"text"`.
    TextField(String),
    /// A password input field: `"*****   "`.
    PasswordField(String),
    /// A slider control: `[----------]`.
    Slider(String),
    /// A checkbox: `[X] label` or `[ ] label`.
    Checkbox { checked: bool, label: String },
    /// A radio button: `(X) label` or `( ) label`.
    Radio { selected: bool, label: String },
    /// A dropdown/combo box: `^Label^`.
    Dropdown(String),
    /// An opened dropdown/combo box: `^Label^ ^`.
    DropdownOpen(String),
    /// A plain text label.
    Label(String),
    /// A horizontal separator line.
    Separator(SeparatorKind),
    /// A tree node with nesting depth (number of leading `+` chars).
    TreeNode { depth: usize, label: String },
}

/// The visual style of a horizontal separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeparatorKind {
    /// Dotted line: `..`
    Dots,
    /// Double line: `===`
    Double,
    /// Single dashed line: `---`
    Single,
    /// Solid underline: `_`
    Solid,
}
