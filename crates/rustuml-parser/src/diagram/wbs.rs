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

/// A single node in the WBS tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbsNode {
    /// Display label.
    pub label: String,
    /// Nesting depth (1 = root, 2 = first level child, …).
    pub depth: usize,
    /// Direct children of this node.
    pub children: Vec<WbsNode>,
}
