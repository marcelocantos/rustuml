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
//! ```
//!
//! Lines beginning with `'` (comment), blank lines, and unrecognised lines
//! are silently skipped.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::gantt::*;

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
}

impl GanttParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            tasks: Vec::new(),
        }
    }

    fn finish(self) -> GanttDiagram {
        GanttDiagram {
            meta: self.meta,
            tasks: self.tasks,
        }
    }

    fn parse_line(&mut self, line_num: usize, line: &str) -> Result<(), ParseError> {
        // Combined: [name] lasts N days and starts at [other]'s end
        if self.try_combined(line) {
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
        // project starts [date] — skip silently
        if line.starts_with("project ") {
            return Ok(());
        }
        // title
        if let Some(rest) = line.strip_prefix("title ") {
            self.meta.title = Some(rest.trim().to_string());
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
        // Silently ignore unknown directives (scale, printscale, etc.)
        Ok(())
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

    /// Fully insert or update a task with both duration and start.
    fn upsert_task(&mut self, name: String, duration: u32, start: TaskStart) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.name == name) {
            task.duration = duration;
            task.start = start;
        } else {
            self.tasks.push(GanttTask {
                name,
                duration,
                start,
            });
        }
    }

    /// Insert or update duration only; preserve existing start if present.
    fn upsert_task_lasts_only(&mut self, name: String, duration: u32) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.name == name) {
            task.duration = duration;
        } else {
            self.tasks.push(GanttTask {
                name,
                duration,
                start: TaskStart::Day(0),
            });
        }
    }

    /// Insert or update start only; preserve existing duration.
    fn upsert_start(&mut self, name: String, start: TaskStart) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.name == name) {
            task.start = start;
        } else {
            self.tasks.push(GanttTask {
                name,
                duration: 1,
                start,
            });
        }
    }
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
}
