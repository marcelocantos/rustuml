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
    /// Rows (tasks and separators) in order, for rendering.
    pub rows: Vec<GanttRow>,
    /// Project start date (YYYY-MM-DD), if specified with `Project starts`.
    pub project_start: Option<String>,
    /// Days of week that are closed (0=Monday, 1=Tuesday, ..., 6=Sunday).
    pub closed_days: Vec<u8>,
    /// Print scale (e.g. "daily", "weekly", "monthly").
    pub printscale: Option<String>,
    /// Resources mentioned in `on {Resource}` clauses, in order of first appearance.
    pub resources: Vec<GanttResource>,
    /// Notes attached to tasks.
    pub notes: Vec<GanttNote>,
}

/// A note block attached to a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttNote {
    /// Lines of note text.
    pub lines: Vec<String>,
}

/// A row in the Gantt chart (task or separator).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GanttRow {
    /// A task row.
    Task(String),
    /// A separator row with an optional label (from `-- Label --`).
    Separator(String),
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
    /// Duration in days (0 = milestone).
    pub duration: u32,
    /// How the start date is determined.
    pub start: TaskStart,
    /// Optional task color (CSS color name or hex).
    pub color: Option<String>,
    /// Resource assignments for this task (may be multiple with percentages).
    pub resources: Vec<TaskResource>,
}

/// A resource assignment for a task (e.g. "Alice" at 100% or "Alice" at 50%).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResource {
    /// Resource name.
    pub name: String,
    /// Percentage (1-100, default 100).
    pub percent: u32,
}

/// A resource that appears in `on {Resource}` clauses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttResource {
    /// Resource name.
    pub name: String,
}
