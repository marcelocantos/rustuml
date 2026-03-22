// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Diagram models — the typed ASTs produced by parsing.

pub mod class;
pub mod sequence;

/// A parsed diagram, ready for layout and rendering.
#[derive(Debug)]
pub enum Diagram {
    Sequence(sequence::SequenceDiagram),
    Class(class::ClassDiagram),
    // Future: State, Activity, Component, UseCase, etc.
}

/// Source location for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

/// Common metadata that any diagram can carry.
#[derive(Debug, Default)]
pub struct DiagramMeta {
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
}
