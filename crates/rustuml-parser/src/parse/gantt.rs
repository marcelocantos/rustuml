// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Gantt chart parser.
//!
//! Recognised syntax (subset of PlantUML Gantt):
//!
//! ```text
//! [Task name] lasts N days
//! [Task name] starts at [Other task]'s end
//! [Task name] lasts N days and starts at [Other task]'s end
//! [Task name] lasts N day
//! [Task name] is colored in <Color>
//! Project starts YYYY-MM-DD
//! saturday are closed
//! sunday are closed
//! ```
//!
//! Lines beginning with `'` (comment), blank lines, and unrecognised lines
//! are silently skipped.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::gantt::{
    GanttDiagram, GanttNote, GanttResource, GanttRow, GanttTask, TaskResource, TaskStart,
};

/// Parse pre-processed lines into a [`GanttDiagram`].
pub fn parse_gantt(lines: &[String]) -> Result<GanttDiagram, ParseError> {
    let mut parser = GanttParser::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('\'') {
            continue;
        }
        parser.parse_line(i + 1, trimmed)?;
    }
    Ok(parser.finish())
}

struct GanttParser {
    meta: DiagramMeta,
    tasks: Vec<GanttTask>,
    /// Ordered rows (task names and separators) for rendering.
    rows: Vec<GanttRow>,
    project_start: Option<String>,
    closed_days: Vec<u8>,
    /// Name of the last task that was explicitly defined (for `then` syntax).
    last_task: Option<String>,
    /// Print scale directive (e.g. "daily", "weekly", "monthly").
    printscale: Option<String>,
    /// Resources in order of first appearance.
    resources: Vec<GanttResource>,
    /// Whether we are inside a `note ... end note` block.
    in_note: bool,
    /// Lines collected for the current note block.
    current_note_lines: Vec<String>,
    /// All notes collected.
    notes: Vec<GanttNote>,
}

impl GanttParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            tasks: Vec::new(),
            rows: Vec::new(),
            project_start: None,
            closed_days: Vec::new(),
            last_task: None,
            printscale: None,
            resources: Vec::new(),
            in_note: false,
            current_note_lines: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn finish(self) -> GanttDiagram {
        GanttDiagram {
            meta: self.meta,
            tasks: self.tasks,
            rows: self.rows,
            project_start: self.project_start,
            closed_days: self.closed_days,
            printscale: self.printscale,
            resources: self.resources,
            notes: self.notes,
        }
    }

    fn parse_line(&mut self, line_num: usize, line: &str) -> Result<(), ParseError> {
        // Note block: collect body lines, end note terminates block.
        if self.in_note {
            if line.trim() == "end note" {
                self.in_note = false;
                if !self.current_note_lines.is_empty() {
                    self.notes.push(GanttNote {
                        lines: std::mem::take(&mut self.current_note_lines),
                    });
                }
            } else {
                self.current_note_lines.push(line.trim().to_string());
            }
            return Ok(());
        }
        // `note bottom/top` — start a note block.
        if line.starts_with("note ") {
            self.in_note = true;
            self.current_note_lines.clear();
            return Ok(());
        }
        // `-- Label --` separator row.
        if let Some(label) = try_separator(line) {
            self.rows.push(GanttRow::Separator(label));
            return Ok(());
        }
        // `printscale weekly/daily/monthly`
        if let Some(scale) = line.strip_prefix("printscale ") {
            self.printscale = Some(scale.trim().to_string());
            return Ok(());
        }
        // `then [name] lasts N days` — implicit AfterTask(last_task)
        if self.try_then(line) {
            return Ok(());
        }
        // Combined: [name] lasts N days and starts at [other]'s end
        if self.try_combined(line) {
            return Ok(());
        }
        // [name] on {Resource} lasts N days
        if self.try_resource_lasts(line) {
            return Ok(());
        }
        // [name] on {Resource} lasts N days and starts at [other]'s end
        if self.try_resource_combined(line) {
            return Ok(());
        }
        // [name] lasts N days
        if self.try_lasts(line) {
            return Ok(());
        }
        // [name] starts at [other]'s end
        if self.try_starts_after(line, line_num)? {
            return Ok(());
        }
        // [name] happens at [other]'s end — milestone (zero-duration task)
        if self.try_happens_at(line) {
            return Ok(());
        }
        // [name] happens YYYY-MM-DD — absolute date milestone
        if self.try_happens_date(line) {
            return Ok(());
        }
        // [name] is colored in <Color>
        if self.try_colored(line) {
            return Ok(());
        }
        // [name] is N% completed — ignore completion percentage
        if self.try_completed(line) {
            return Ok(());
        }
        // Project starts YYYY-MM-DD
        if self.try_project_starts(line) {
            return Ok(());
        }
        // <day> are closed
        if self.try_closed_day(line) {
            return Ok(());
        }
        // project starts [date] — fallback skip
        if line.to_lowercase().starts_with("project ") {
            return Ok(());
        }
        // title
        if let Some(rest) = line.strip_prefix("title ") {
            self.meta.title = Some(super::strip_title_quotes(rest).to_string());
            return Ok(());
        }
        // header / footer
        if let Some(rest) = line.strip_prefix("header ") {
            self.meta.header = Some(rest.trim().to_string());
            return Ok(());
        }
        if let Some(rest) = line.strip_prefix("footer ") {
            self.meta.footer = Some(rest.trim().to_string());
            return Ok(());
        }
        // Silently ignore unknown directives (scale, etc.)
        Ok(())
    }

    /// `[name] on {Resource}[{Resource2:N%}...] lasts N days`
    fn try_resource_lasts(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\[([^\]]+)\]\s+on\s+((?:\{[^}]+\})+)\s+lasts\s+(\d+)\s+days?$").unwrap()
        });
        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let resource_str = caps[2].to_string();
            let duration: u32 = caps[3].parse().unwrap_or(1);
            let task_resources = parse_task_resources(&resource_str);
            for tr in &task_resources {
                self.register_resource(tr.name.clone());
            }
            self.upsert_task_with_resources(name, duration, TaskStart::Day(0), task_resources);
            true
        } else {
            false
        }
    }

    /// `[name] on {Resource}[{Resource2:N%}...] lasts N days and starts at [other]'s end`
    fn try_resource_combined(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"^\[([^\]]+)\]\s+on\s+((?:\{[^}]+\})+)\s+lasts\s+(\d+)\s+days?\s+and\s+starts\s+at\s+\[([^\]]+)\]'s\s+end$",
            )
            .unwrap()
        });
        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let resource_str = caps[2].to_string();
            let duration: u32 = caps[3].parse().unwrap_or(1);
            let dep = caps[4].to_string();
            let task_resources = parse_task_resources(&resource_str);
            for tr in &task_resources {
                self.register_resource(tr.name.clone());
            }
            self.upsert_task_with_resources(
                name,
                duration,
                TaskStart::AfterTask(dep),
                task_resources,
            );
            true
        } else {
            false
        }
    }

    /// `[name] happens YYYY-MM-DD` or `[name] happens at YYYY-MM-DD` — absolute date milestone
    fn try_happens_date(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\[([^\]]+)\]\s+happens\s+(?:at\s+)?(\d{4}-\d{2}-\d{2})$").unwrap()
        });
        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let date = caps[2].to_string();
            // If project_start is set, compute offset from it; otherwise compute from
            // 1970-01-01 (PlantUML Unix epoch behavior).
            let start = if let Some(ref ps) = self.project_start {
                date_diff_days(ps, &date)
                    .map(TaskStart::Day)
                    .unwrap_or(TaskStart::Day(0))
            } else {
                // No project start: use days since 1970-01-01 as offset.
                date_diff_days("1970-01-01", &date)
                    .map(TaskStart::Day)
                    .unwrap_or(TaskStart::Day(0))
            };
            self.upsert_task(name, 0, start);
            true
        } else {
            false
        }
    }

    /// `then [name] lasts N days` — implicit start after last_task
    fn try_then(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^then\s+\[([^\]]+)\]\s+lasts\s+(\d+)\s+days?$").unwrap());
        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let duration: u32 = caps[2].parse().unwrap_or(1);
            let start = if let Some(prev) = &self.last_task {
                TaskStart::AfterTask(prev.clone())
            } else {
                TaskStart::Day(0)
            };
            self.upsert_task(name, duration, start);
            true
        } else {
            false
        }
    }

    /// `[name] happens at [other]'s end` — zero-duration milestone
    fn try_happens_at(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\[([^\]]+)\]\s+happens\s+at\s+\[([^\]]+)\]'s\s+end$").unwrap()
        });
        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let dep = caps[2].to_string();
            self.upsert_task(name, 0, TaskStart::AfterTask(dep));
            true
        } else {
            false
        }
    }

    /// `[name] lasts N days and starts at [other]'s end`
    fn try_combined(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"^\[([^\]]+)\]\s+lasts\s+(\d+)\s+days?\s+and\s+starts\s+at\s+\[([^\]]+)\]'s\s+end$",
            )
            .unwrap()
        });
        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let duration: u32 = caps[2].parse().unwrap_or(1);
            let dep = caps[3].to_string();
            self.upsert_task(name, duration, TaskStart::AfterTask(dep));
            true
        } else {
            false
        }
    }

    /// `[name] lasts N days`
    fn try_lasts(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\[([^\]]+)\]\s+lasts\s+(\d+)\s+days?$").unwrap());
        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let duration: u32 = caps[2].parse().unwrap_or(1);
            // Only set start if not already present (starts at overrides).
            self.upsert_task_lasts_only(name, duration);
            true
        } else {
            false
        }
    }

    /// `[name] starts at [other]'s end`
    fn try_starts_after(&mut self, line: &str, _line_num: usize) -> Result<bool, ParseError> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\[([^\]]+)\]\s+starts\s+at\s+\[([^\]]+)\]'s\s+end$").unwrap()
        });
        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let dep = caps[2].to_string();
            self.upsert_start(name, TaskStart::AfterTask(dep));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// `[name] is colored in <Color>`
    fn try_colored(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\[([^\]]+)\]\s+is\s+colored\s+in\s+(\S+)$").unwrap());
        if let Some(caps) = RE.captures(line) {
            let name = caps[1].to_string();
            let color = caps[2].to_string();
            self.upsert_color(name, color);
            true
        } else {
            false
        }
    }

    /// `[name] is N% completed` — parsed but ignored
    fn try_completed(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\[([^\]]+)\]\s+is\s+\d+%\s+completed$").unwrap());
        RE.is_match(line)
    }

    /// `Project starts YYYY-MM-DD`
    fn try_project_starts(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?i)^[Pp]roject\s+starts\s+(\d{4}-\d{2}-\d{2})$").unwrap()
        });
        if let Some(caps) = RE.captures(line) {
            self.project_start = Some(caps[1].to_string());
            true
        } else {
            false
        }
    }

    /// `<dayname> are closed` — e.g. `saturday are closed`
    fn try_closed_day(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"(?i)^(monday|tuesday|wednesday|thursday|friday|saturday|sunday)\s+are\s+closed$",
            )
            .unwrap()
        });
        if let Some(caps) = RE.captures(line) {
            let day_num = match caps[1].to_lowercase().as_str() {
                "monday" => 0u8,
                "tuesday" => 1,
                "wednesday" => 2,
                "thursday" => 3,
                "friday" => 4,
                "saturday" => 5,
                "sunday" => 6,
                _ => return false,
            };
            if !self.closed_days.contains(&day_num) {
                self.closed_days.push(day_num);
            }
            true
        } else {
            false
        }
    }

    /// Fully insert or update a task with both duration and start.
    fn upsert_task(&mut self, name: String, duration: u32, start: TaskStart) {
        self.last_task = Some(name.clone());
        if let Some(task) = self.tasks.iter_mut().find(|t| t.name == name) {
            task.duration = duration;
            task.start = start;
        } else {
            if !self
                .rows
                .iter()
                .any(|r| matches!(r, GanttRow::Task(n) if n == &name))
            {
                self.rows.push(GanttRow::Task(name.clone()));
            }
            self.tasks.push(GanttTask {
                name,
                duration,
                start,
                color: None,
                resources: Vec::new(),
            });
        }
    }

    /// Insert or update duration only; preserve existing start if present.
    fn upsert_task_lasts_only(&mut self, name: String, duration: u32) {
        self.last_task = Some(name.clone());
        if let Some(task) = self.tasks.iter_mut().find(|t| t.name == name) {
            task.duration = duration;
        } else {
            if !self
                .rows
                .iter()
                .any(|r| matches!(r, GanttRow::Task(n) if n == &name))
            {
                self.rows.push(GanttRow::Task(name.clone()));
            }
            self.tasks.push(GanttTask {
                name,
                duration,
                start: TaskStart::Day(0),
                color: None,
                resources: Vec::new(),
            });
        }
    }

    /// Insert or update start only; preserve existing duration.
    fn upsert_start(&mut self, name: String, start: TaskStart) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.name == name) {
            task.start = start;
        } else {
            if !self
                .rows
                .iter()
                .any(|r| matches!(r, GanttRow::Task(n) if n == &name))
            {
                self.rows.push(GanttRow::Task(name.clone()));
            }
            self.tasks.push(GanttTask {
                name,
                duration: 1,
                start,
                color: None,
                resources: Vec::new(),
            });
        }
    }

    /// Insert or update color only; preserve existing task.
    fn upsert_color(&mut self, name: String, color: String) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.name == name) {
            task.color = Some(color);
        } else {
            // Task not seen yet — insert placeholder; duration/start will be filled later.
            self.tasks.push(GanttTask {
                name,
                duration: 1,
                start: TaskStart::Day(0),
                color: Some(color),
                resources: Vec::new(),
            });
        }
    }

    /// Insert or update a task with resource assignments.
    fn upsert_task_with_resources(
        &mut self,
        name: String,
        duration: u32,
        start: TaskStart,
        task_resources: Vec<TaskResource>,
    ) {
        self.last_task = Some(name.clone());
        if let Some(task) = self.tasks.iter_mut().find(|t| t.name == name) {
            task.duration = duration;
            task.start = start;
            task.resources = task_resources;
        } else {
            if !self
                .rows
                .iter()
                .any(|r| matches!(r, GanttRow::Task(n) if n == &name))
            {
                self.rows.push(GanttRow::Task(name.clone()));
            }
            self.tasks.push(GanttTask {
                name,
                duration,
                start,
                color: None,
                resources: task_resources,
            });
        }
    }

    /// Register a resource if not already seen.
    fn register_resource(&mut self, name: String) {
        if !self.resources.iter().any(|r| r.name == name) {
            self.resources.push(GanttResource { name });
        }
    }
}

/// Parse `{Alice}` or `{Alice:50%}{Bob:50%}` resource strings into a list of TaskResource.
fn parse_task_resources(s: &str) -> Vec<TaskResource> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{([^}:]+)(?::(\d+)%)?\}").unwrap());
    RE.captures_iter(s)
        .map(|caps| {
            let name = caps[1].trim().to_string();
            let percent: u32 = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(100);
            TaskResource { name, percent }
        })
        .collect()
}

/// Parse `-- Label --` separator lines. Returns `Some(label)` if matched.
fn try_separator(line: &str) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^--\s*(.*?)\s*--$").unwrap());
    RE.captures(line).map(|caps| caps[1].to_string())
}

/// Compute the number of calendar days between two dates (YYYY-MM-DD).
/// Returns `None` if parsing fails. Returns 0 if dates are equal.
fn date_diff_days(from: &str, to: &str) -> Option<u32> {
    fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return None;
        }
        let y: i32 = parts[0].parse().ok()?;
        let m: u32 = parts[1].parse().ok()?;
        let d: u32 = parts[2].parse().ok()?;
        Some((y, m, d))
    }
    fn to_jdn(y: i32, m: u32, d: u32) -> i64 {
        let a = (14 - m as i32) / 12;
        let yr = y + 4800 - a;
        let mo = m as i32 + 12 * a - 3;
        d as i64 + (153 * mo + 2) as i64 / 5 + 365 * yr as i64 + yr as i64 / 4 - yr as i64 / 100
            + yr as i64 / 400
            - 32045
    }
    let (fy, fm, fd) = parse_date(from)?;
    let (ty, tm, td) = parse_date(to)?;
    let diff = to_jdn(ty, tm, td) - to_jdn(fy, fm, fd);
    if diff < 0 { None } else { Some(diff as u32) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> GanttDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_gantt(&lines).unwrap()
    }

    #[test]
    fn basic_task() {
        let d = parse("[Task 1] lasts 5 days");
        assert_eq!(d.tasks.len(), 1);
        assert_eq!(d.tasks[0].name, "Task 1");
        assert_eq!(d.tasks[0].duration, 5);
        assert!(matches!(d.tasks[0].start, TaskStart::Day(0)));
    }

    #[test]
    fn singular_day() {
        let d = parse("[Task 1] lasts 1 day");
        assert_eq!(d.tasks[0].duration, 1);
    }

    #[test]
    fn starts_after_dependency() {
        let d = parse(
            "[Task 1] lasts 5 days\n[Task 2] lasts 3 days\n[Task 2] starts at [Task 1]'s end",
        );
        assert_eq!(d.tasks.len(), 2);
        assert!(matches!(&d.tasks[1].start, TaskStart::AfterTask(dep) if dep == "Task 1"));
    }

    #[test]
    fn combined_syntax() {
        let d = parse("[Task 2] lasts 3 days and starts at [Task 1]'s end");
        assert_eq!(d.tasks.len(), 1);
        assert_eq!(d.tasks[0].duration, 3);
        assert!(matches!(&d.tasks[0].start, TaskStart::AfterTask(dep) if dep == "Task 1"));
    }

    #[test]
    fn dependency_chain() {
        let input = "[Task 1] lasts 5 days\n\
                     [Task 2] lasts 3 days\n\
                     [Task 2] starts at [Task 1]'s end\n\
                     [Task 3] lasts 2 days\n\
                     [Task 3] starts at [Task 2]'s end";
        let d = parse(input);
        assert_eq!(d.tasks.len(), 3);
        assert!(matches!(&d.tasks[2].start, TaskStart::AfterTask(dep) if dep == "Task 2"));
    }

    #[test]
    fn title_parsed() {
        let d = parse("title My Project\n[T1] lasts 2 days");
        assert_eq!(d.meta.title.as_deref(), Some("My Project"));
    }

    #[test]
    fn unknown_lines_ignored() {
        let d = parse("scale 1.5\n[Task 1] lasts 3 days\nprintscale daily");
        assert_eq!(d.tasks.len(), 1);
    }

    #[test]
    fn project_start_parsed() {
        let d = parse("Project starts 2024-01-01\n[T1] lasts 3 days");
        assert_eq!(d.project_start.as_deref(), Some("2024-01-01"));
    }

    #[test]
    fn closed_days_parsed() {
        let d = parse("saturday are closed\nsunday are closed\n[T1] lasts 3 days");
        assert!(d.closed_days.contains(&5));
        assert!(d.closed_days.contains(&6));
    }

    #[test]
    fn colored_task_parsed() {
        let d = parse("[T1] lasts 3 days\n[T1] is colored in Coral");
        assert_eq!(d.tasks[0].color.as_deref(), Some("Coral"));
    }
}
