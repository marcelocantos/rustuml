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
}

/// A state the timeline enters at a particular time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    /// Absolute time value (from `@N` directives).
    pub at: i64,
    /// The state name (e.g., `Idle`, `Processing`).
    pub state: String,
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
