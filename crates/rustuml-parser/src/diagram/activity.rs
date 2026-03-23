// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram model (v3 / new syntax).

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A complete activity diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityDiagram {
    pub meta: DiagramMeta,
    pub steps: Vec<ActivityStep>,
}

/// A step in an activity diagram.
#[derive(Debug, Serialize, Deserialize)]
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
    Detach,
    Kill,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IfBlock {
    pub condition: String,
    pub then_label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElseIfBranch {
    pub condition: String,
    pub then_label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhileBlock {
    pub condition: String,
    pub is_label: Option<String>,
}

/// Note position (left or right of the action).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotePosition {
    Left,
    Right,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoteBlock {
    pub text: String,
    pub color: Option<String>,
    pub position: NotePosition,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PartitionBlock {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepeatWhileBlock {
    pub condition: String,
    pub is_label: Option<String>,
}
