// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Gantt chart diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A complete Gantt chart.
#[derive(Debug, Serialize, Deserialize)]
pub struct GanttDiagram {
    pub meta: DiagramMeta,
    pub tasks: Vec<GanttTask>,
}

/// How a task's start is specified.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum TaskStart {
    /// Absolute start offset in days from project start (day 0).
    Day(u32),
    /// Starts when another named task ends.
    AfterTask(String),
}

/// A single task in the Gantt chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttTask {
    /// Display name (without brackets).
    pub name: String,
    /// Duration in days.
    pub duration: u32,
    /// How the start date is determined.
    pub start: TaskStart,
}
