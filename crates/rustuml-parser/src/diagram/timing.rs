// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Timing diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A complete timing diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct TimingDiagram {
    pub meta: DiagramMeta,
    /// Timelines declared with `robust`, `concise`, or `binary`.
    pub timelines: Vec<Timeline>,
    /// All time points that appear in the diagram, sorted ascending.
    pub time_points: Vec<i64>,
    /// Highlighted time ranges (from `highlight T1 to T2 #color : label`).
    pub highlights: Vec<Highlight>,
    /// Time-range annotations (from `@T1 <-> @T2 : label`).
    pub annotations: Vec<Annotation>,
    /// Scale: N time units equals M pixels (from `scale N as M pixels`).
    /// When set, the axis shows a tick every N units across the full span.
    pub scale: Option<Scale>,
    /// Notes (`note top/bottom of X : text`).
    #[serde(default)]
    pub notes: Vec<TimingNote>,
    /// Source line (`data-source-line`) of the `title` directive, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_line: Option<usize>,
    /// Source line of the `header` directive, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_line: Option<usize>,
    /// Source line of the `footer` directive, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer_line: Option<usize>,
}

/// A note attached to a timeline at a specific time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingNote {
    /// Timeline ID the note is attached to.
    pub timeline_id: String,
    /// Absolute time at which the note appears.
    pub at: i64,
    /// Note text.
    pub text: String,
    /// Whether the note appears above (`top`) or below (`bottom`) the timeline.
    pub above: bool,
}

/// One named timeline (participant) in a timing diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    /// Alias used in state-change directives (the `as X` part, or the
    /// display name if no alias is given).
    pub id: String,
    /// Display label (the quoted string, e.g. `"Web Browser"`).
    pub label: String,
    /// Visual style of this timeline.
    pub kind: TimelineKind,
    /// Ordered list of state transitions.
    pub changes: Vec<StateChange>,
    /// Clock parameters when `kind == TimelineKind::Clock`. A clock generates
    /// its own square-wave from `period`/`pulse`/`offset` rather than from
    /// `changes` (which are unused). Mirrors PlantUML's `PlayerClock`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock: Option<ClockSpec>,
}

/// Clock waveform parameters (PlantUML `clock ... with period N [pulse P] [offset O]`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClockSpec {
    /// Full cycle length, in time units.
    pub period: i64,
    /// High-level width per cycle; `0` means `period / 2` (PlantUML default).
    pub pulse: i64,
    /// Initial low offset before the first rising edge.
    pub offset: i64,
}

/// Visual style of a timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineKind {
    /// Multi-state solid-block style (PlantUML `robust`).
    Robust,
    /// Thin-line concise style (PlantUML `concise`).
    Concise,
    /// Two-level digital signal (PlantUML `binary`).
    Binary,
    /// Auto-generated periodic square wave (PlantUML `clock`).
    Clock,
}

/// A state the timeline enters at a particular time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    /// Absolute time value (from `@N` directives).
    pub at: i64,
    /// The state name (e.g., `Idle`, `Processing`).
    pub state: String,
    /// True when the state assignment appeared before the first explicit `@N`
    /// marker. PlantUML renders that as an initial segment immediately before
    /// the first ruler tick rather than as a zero-length transition at time 0.
    #[serde(default, skip_serializing_if = "is_false")]
    pub before_first_time: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// A highlighted time range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub from: i64,
    pub to: i64,
    pub color: Option<String>,
    pub label: Option<String>,
}

/// A bidirectional time-range annotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub from: i64,
    pub to: i64,
    pub label: String,
}

/// Scale directive: `scale N as M pixels`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Scale {
    /// Number of time units per tick interval shown on the axis.
    pub units: i64,
    /// Number of pixels for those units.
    pub pixels: i64,
}
