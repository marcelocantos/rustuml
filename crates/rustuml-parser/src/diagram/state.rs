// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram model.

use super::DiagramMeta;

/// A complete state diagram.
#[derive(Debug)]
pub struct StateDiagram {
    pub meta: DiagramMeta,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
}

/// A state in a state diagram.
#[derive(Debug, Clone)]
pub struct State {
    pub id: String,
    pub label: String,
    pub kind: StateKind,
    pub descriptions: Vec<String>,
    pub substates: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateKind {
    #[default]
    Normal,
    Initial,
    Final,
    Choice,
    Fork,
    Join,
    History,
    DeepHistory,
}

/// A transition between states.
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}
