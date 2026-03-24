// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Ditaa (ASCII art → diagram) model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

#[derive(Debug, Serialize, Deserialize)]
pub struct DitaaDiagram {
    pub meta: DiagramMeta,
    pub grid_width: usize,
    pub grid_height: usize,
    pub shapes: Vec<DitaaShape>,
    pub connections: Vec<DitaaConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DitaaShape {
    /// Grid column of top-left corner.
    pub col: usize,
    /// Grid row of top-left corner.
    pub row: usize,
    /// Width in grid columns.
    pub width: usize,
    /// Height in grid rows.
    pub height: usize,
    pub kind: DitaaShapeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<DitaaColor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DitaaShapeKind {
    Box,
    RoundedBox,
    Document,
    Storage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DitaaColor {
    Cyan,
    Red,
    Green,
    Yellow,
    Blue,
}

impl DitaaColor {
    pub fn fill(&self) -> &'static str {
        match self {
            DitaaColor::Cyan => "#B0E0E6",
            DitaaColor::Red => "#FFB0B0",
            DitaaColor::Green => "#B0FFB0",
            DitaaColor::Yellow => "#FFFFA0",
            DitaaColor::Blue => "#B0B0FF",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DitaaConnection {
    pub segments: Vec<DitaaSegment>,
    pub dashed: bool,
    pub start_arrow: bool,
    pub end_arrow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DitaaSegment {
    pub start_col: usize,
    pub start_row: usize,
    pub end_col: usize,
    pub end_row: usize,
}
