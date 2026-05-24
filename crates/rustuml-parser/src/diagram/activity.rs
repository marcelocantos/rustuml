// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram model (v3 / new syntax).

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A complete activity diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDiagram {
    pub meta: DiagramMeta,
    pub steps: Vec<ActivityStep>,
}

/// A step in an activity diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityStep {
    Start,
    Stop,
    End,
    Action(String),
    If(IfBlock),
    ElseIf(ElseIfBranch),
    Else(Option<String>),
    EndIf,
    Switch(String),
    Case(String),
    EndSwitch,
    While(WhileBlock),
    EndWhile(Option<String>),
    Repeat,
    RepeatWhile(RepeatWhileBlock),
    Fork,
    ForkAgain,
    EndFork,
    Split,
    SplitAgain,
    EndSplit,
    Swimlane(String),
    Partition(PartitionBlock),
    EndPartition,
    Note(NoteBlock),
    DeprecatedColorAction(DeprecatedColorAction),
    Arrow(ArrowStep),
    Backward(String),
    Break,
    Detach,
    Kill,
}

/// Deprecated `#color:text;` syntax -- renders a warning banner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecatedColorAction {
    pub color: String,
    pub text: String,
}

/// An explicit arrow step (`->`, `-->`, `-[#color]->`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowStep {
    pub dashed: bool,
    pub color: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfBlock {
    pub condition: String,
    pub then_label: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElseIfBranch {
    pub condition: String,
    pub then_label: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhileBlock {
    pub condition: String,
    pub is_label: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

/// Note position (left or right of the action).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotePosition {
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteBlock {
    pub text: String,
    pub color: Option<String>,
    pub position: NotePosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionBlock {
    pub name: String,
    pub color: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeatWhileBlock {
    pub condition: String,
    /// Label for the loop-back branch (e.g. `is (yes)` in `repeat while (c) is (yes)`).
    pub is_label: Option<String>,
    /// Label for the exit branch (e.g. `not (no)` in `repeat while (c) is (y) not (no)`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_label: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}
