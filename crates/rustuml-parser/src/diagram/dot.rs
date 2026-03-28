// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Graphviz DOT diagram model.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

/// A complete DOT diagram (`@startdot` … `@enddot`).
#[derive(Debug, Serialize, Deserialize)]
pub struct DotDiagram {
    pub meta: DiagramMeta,
    /// Whether the graph uses directed edges (`digraph` vs `graph`).
    pub directed: bool,
    /// Graph name (e.g. `G` in `digraph G { … }`).
    pub name: String,
    /// Top-level graph attributes (`rankdir`, `label`, `bgcolor`, etc.).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attrs: HashMap<String, String>,
    /// Default node attributes (`node [shape=box]`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub node_defaults: HashMap<String, String>,
    /// Default edge attributes (`edge [color=gray]`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub edge_defaults: HashMap<String, String>,
    /// Nodes declared in the graph.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<DotNode>,
    /// Edges declared in the graph.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<DotEdge>,
    /// Cluster subgraphs (`subgraph cluster_N { … }`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clusters: Vec<DotCluster>,
}

/// A node in a DOT graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotNode {
    pub id: String,
    /// Node attributes: `label`, `shape`, `fillcolor`, `style`, etc.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attrs: HashMap<String, String>,
}

/// An edge in a DOT graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotEdge {
    pub from: String,
    pub to: String,
    /// Edge attributes: `label`, `color`, `style`, `arrowhead`, etc.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attrs: HashMap<String, String>,
}

/// A cluster subgraph (`subgraph cluster_X { … }`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotCluster {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<DotNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<DotEdge>,
}
