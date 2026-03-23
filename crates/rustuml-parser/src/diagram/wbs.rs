// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Work Breakdown Structure (WBS) diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

/// A WBS diagram: a top-down hierarchy of labeled nodes.
#[derive(Debug, Serialize, Deserialize)]
pub struct WbsDiagram {
    pub meta: DiagramMeta,
    /// The root nodes (depth = 1, i.e. a single `*` line).  In practice
    /// well-formed WBS has exactly one root, but we allow multiple.
    pub nodes: Vec<WbsNode>,
}

/// Which side of the root a WBS branch grows from.
///
/// In PlantUML WBS, `*` / `**` / `***` prefix nodes are on the **right** side
/// and `--` / `---` etc. prefix nodes are on the **left** side.  The root
/// (depth 1) itself has no side — it belongs to both halves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WbsSide {
    Right,
    Left,
}

/// A single node in the WBS tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbsNode {
    /// Display label.
    pub label: String,
    /// Nesting depth (1 = root, 2 = first level child, …).
    pub depth: usize,
    /// Which side of the root this node grows from.
    /// For the root node (depth = 1) this is always `Right` (ignored by the
    /// renderer; the root is centred between both subtrees).
    pub side: WbsSide,
    /// Direct children of this node.
    pub children: Vec<WbsNode>,
}
