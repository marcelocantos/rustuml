// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Mind map diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

/// Which side of the root a branch extends toward.
///
/// Depth-1 nodes (`*`) are `Right` by convention; left-side branches use `-`
/// prefix and are `Left`.  Children inherit their ancestor's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Right,
    Left,
}

/// A single node in the mind map tree.
///
/// The `depth` field mirrors the number of leading `*` or `-` characters
/// (1 = root, 2 = first-level branch, etc.).  Children are stored in order
/// of appearance.  `side` indicates which side of the root this subtree lives
/// on: `Right` for `*`-prefixed nodes, `Left` for `-`-prefixed nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindMapNode {
    pub label: String,
    pub depth: usize,
    pub side: Side,
    pub children: Vec<MindMapNode>,
}

/// The complete mind map diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct MindMapDiagram {
    pub meta: DiagramMeta,
    /// Top-level roots (almost always exactly one, but the grammar permits
    /// multiple adjacent `*` lines at depth 1).
    pub roots: Vec<MindMapNode>,
}
