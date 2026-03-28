// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Git log diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A complete git log diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct GitDiagram {
    pub meta: DiagramMeta,
    pub commands: Vec<GitCommand>,
}

/// A command in a git diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitCommand {
    Commit {
        id: Option<String>,
        tag: Option<String>,
        commit_type: CommitType,
    },
    Branch(String),
    Checkout(String),
    Merge {
        branch: String,
        tag: Option<String>,
    },
    CherryPick {
        id: String,
    },
}

/// Visual style of a commit node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum CommitType {
    #[default]
    Normal,
    Reverse,
    Highlight,
}
