// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Board (Kanban) diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

/// A column in a Kanban board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardColumn {
    pub label: String,
    pub cards: Vec<String>,
}

/// The complete board diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardDiagram {
    pub meta: DiagramMeta,
    pub title: String,
    pub columns: Vec<BoardColumn>,
}
