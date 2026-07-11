// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Board (Kanban) diagram model.
//!
//! PlantUML renders `@startboard` as a two-row grid of boxes (a degenerate
//! work-breakdown layout). The diagram is an ordered list of items, each
//! tagged with the marker that introduced it: the implicit root (the first
//! non-marker line), a `+` column header, or a `*` card.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

/// The marker that introduced a board item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoardItemKind {
    /// The first non-marker line — the board's root box.
    Root,
    /// A `+`-prefixed column header.
    Column,
    /// A `*`-prefixed card.
    Card,
}

/// A single box in the board, in document order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardItem {
    pub kind: BoardItemKind,
    /// The rendered label (markers stripped per PlantUML's quirky rules).
    pub label: String,
}

/// The complete board diagram — a flat, ordered list of items.
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardDiagram {
    pub meta: DiagramMeta,
    pub items: Vec<BoardItem>,
}
