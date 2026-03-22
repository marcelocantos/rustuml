// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Timing diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A complete timing diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct TimingDiagram {
    pub meta: DiagramMeta,
    /// Timelines declared with `robust` or `concise`.
    pub timelines: Vec<Timeline>,
    /// All time points that appear in the diagram, sorted ascending.
    pub time_points: Vec<i64>,
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
}

/// A state the timeline enters at a particular time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    /// Absolute time value (from `@N` directives).
    pub at: i64,
    /// The state name (e.g., `Idle`, `Processing`).
    pub state: String,
}
