// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram model (v3 / new syntax).

use super::DiagramMeta;

/// A complete activity diagram.
#[derive(Debug)]
pub struct ActivityDiagram {
    pub meta: DiagramMeta,
    pub steps: Vec<ActivityStep>,
}

/// A step in an activity diagram.
#[derive(Debug)]
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
    RepeatWhile(String),
    Fork,
    ForkAgain,
    EndFork,
    Split,
    SplitAgain,
    EndSplit,
    Swimlane(String),
    Partition(String),
    EndPartition,
    Note(String),
    Detach,
    Kill,
}

#[derive(Debug)]
pub struct IfBlock {
    pub condition: String,
    pub then_label: Option<String>,
}

#[derive(Debug)]
pub struct ElseIfBranch {
    pub condition: String,
    pub then_label: Option<String>,
}

#[derive(Debug)]
pub struct WhileBlock {
    pub condition: String,
    pub is_label: Option<String>,
}
