// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A complete sequence diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct SequenceDiagram {
    pub meta: DiagramMeta,
    pub participants: Vec<Participant>,
    pub events: Vec<Event>,
    pub autonumber: Option<AutoNumber>,
}

/// A participant (lifeline) in a sequence diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub kind: ParticipantKind,
    pub order: Option<usize>,
    /// Optional UML stereotype text, e.g. `<<service>>` → `"service"`.
    pub stereotype: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

/// The visual shape of a participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ParticipantKind {
    #[default]
    Participant,
    Actor,
    Boundary,
    Control,
    Entity,
    Database,
    Collections,
    Queue,
}

/// A sequence diagram event (message, note, group boundary, etc.).
#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    Message(Message),
    Note(Note),
    NoteOnLink(String),
    GroupStart(GroupStart),
    GroupElse(GroupElse),
    GroupEnd,
    Divider(String),
    Delay(Option<String>),
    Space(Option<u32>),
    Ref(Ref),
    Activate(String, Option<String>),
    Deactivate(String),
    Destroy(String),
    Create(String),
    Return(ReturnMessage),
    NewPage(Option<String>),
}

/// A message arrow between participants.
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub label: String,
    pub arrow: Arrow,
    pub activation: Option<ActivationChange>,
    /// Optional color for activation bar (e.g., "#blue", "#FF0000").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation_color: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

/// Arrow style for a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arrow {
    pub line: LineStyle,
    pub head: ArrowHead,
    pub direction: ArrowDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LineStyle {
    #[default]
    Solid,
    Dotted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ArrowHead {
    #[default]
    Filled,
    Open,
    Cross,
    Circle,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArrowDirection {
    LeftToRight,
    RightToLeft,
    Bidirectional,
    Self_,
}

/// Activation change triggered by a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationChange {
    Activate,
    Deactivate,
    Destroy,
}

/// A note attached to one or more participants.
#[derive(Debug, Serialize, Deserialize)]
pub struct Note {
    pub position: NotePosition,
    pub participants: Vec<String>,
    pub text: String,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotePosition {
    Left,
    Right,
    Over,
}

/// Start of a grouping construct (alt, opt, loop, etc.).
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupStart {
    pub kind: GroupKind,
    pub label: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupKind {
    Alt,
    Opt,
    Loop,
    Par,
    Break,
    Critical,
    Group,
}

/// An "else" branch within a group.
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupElse {
    pub label: Option<String>,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

/// A reference to another diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct Ref {
    pub participants: Vec<String>,
    pub text: String,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

/// A return message (shorthand for dotted reply).
#[derive(Debug, Serialize, Deserialize)]
pub struct ReturnMessage {
    pub label: String,
    /// 1-based line number within the `@startuml` block.
    #[serde(default)]
    pub source_line: usize,
}

/// Auto-numbering configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct AutoNumber {
    pub start: u32,
    pub step: u32,
    pub format: Option<String>,
}
