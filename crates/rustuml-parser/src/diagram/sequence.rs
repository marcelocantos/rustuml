// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram model.

use super::DiagramMeta;

/// A complete sequence diagram.
#[derive(Debug)]
pub struct SequenceDiagram {
    pub meta: DiagramMeta,
    pub participants: Vec<Participant>,
    pub events: Vec<Event>,
    pub autonumber: Option<AutoNumber>,
}

/// A participant (lifeline) in a sequence diagram.
#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub kind: ParticipantKind,
    pub order: Option<usize>,
}

/// The visual shape of a participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
#[derive(Debug)]
pub enum Event {
    Message(Message),
    Note(Note),
    GroupStart(GroupStart),
    GroupElse(GroupElse),
    GroupEnd,
    Divider(String),
    Delay(Option<String>),
    Space(Option<u32>),
    Ref(Ref),
    Activate(String),
    Deactivate(String),
    Destroy(String),
    Create(String),
    Return(ReturnMessage),
    NewPage(Option<String>),
}

/// A message arrow between participants.
#[derive(Debug)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub label: String,
    pub arrow: Arrow,
    pub activation: Option<ActivationChange>,
}

/// Arrow style for a message.
#[derive(Debug, Clone)]
pub struct Arrow {
    pub line: LineStyle,
    pub head: ArrowHead,
    pub direction: ArrowDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineStyle {
    #[default]
    Solid,
    Dotted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowHead {
    #[default]
    Filled,
    Open,
    Cross,
    Circle,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowDirection {
    LeftToRight,
    RightToLeft,
    Bidirectional,
    Self_,
}

/// Activation change triggered by a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationChange {
    Activate,
    Deactivate,
    Destroy,
}

/// A note attached to one or more participants.
#[derive(Debug)]
pub struct Note {
    pub position: NotePosition,
    pub participants: Vec<String>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotePosition {
    Left,
    Right,
    Over,
}

/// Start of a grouping construct (alt, opt, loop, etc.).
#[derive(Debug)]
pub struct GroupStart {
    pub kind: GroupKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug)]
pub struct GroupElse {
    pub label: Option<String>,
}

/// A reference to another diagram.
#[derive(Debug)]
pub struct Ref {
    pub participants: Vec<String>,
    pub text: String,
}

/// A return message (shorthand for dotted reply).
#[derive(Debug)]
pub struct ReturnMessage {
    pub label: String,
}

/// Auto-numbering configuration.
#[derive(Debug)]
pub struct AutoNumber {
    pub start: u32,
    pub step: u32,
    pub format: Option<String>,
}
