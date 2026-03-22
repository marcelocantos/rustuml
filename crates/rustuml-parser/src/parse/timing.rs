// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Timing diagram parser.
//!
//! Handles `robust` and `concise` timelines and `@N` time-point directives.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::timing::*;

/// Parse preprocessed lines into a [`TimingDiagram`].
pub fn parse_timing(lines: &[String]) -> Result<TimingDiagram, ParseError> {
    let mut parser = TimingParser::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        parser.parse_line(i + 1, trimmed)?;
    }
    Ok(parser.finish())
}

struct TimingParser {
    meta: DiagramMeta,
    timelines: Vec<Timeline>,
    /// Current active time (set by `@N` directives).
    current_time: i64,
    /// All time values encountered.
    time_points: BTreeSet<i64>,
}

impl TimingParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            timelines: Vec::new(),
            current_time: 0,
            time_points: BTreeSet::new(),
        }
    }

    fn finish(self) -> TimingDiagram {
        let time_points = self.time_points.into_iter().collect();
        TimingDiagram {
            meta: self.meta,
            timelines: self.timelines,
            time_points,
        }
    }

    fn parse_line(&mut self, line_num: usize, line: &str) -> Result<(), ParseError> {
        // Skip @startuml / @enduml.
        if line.starts_with('@') {
            self.try_time_point(line);
            return Ok(());
        }

        if self.try_timeline_decl(line) {
            return Ok(());
        }
        if self.try_state_change(line) {
            return Ok(());
        }

        // Silently ignore unrecognised lines (skinparam, title, etc.).
        let _ = line_num;
        Ok(())
    }

    /// Try to parse `@N` (absolute time marker).
    fn try_time_point(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^@(-?\d+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            if let Ok(t) = caps[1].parse::<i64>() {
                self.current_time = t;
                self.time_points.insert(t);
                return true;
            }
        }
        false
    }

    /// Try `robust "Label" as Alias` or `concise "Label" as Alias`.
    fn try_timeline_decl(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^(robust|concise)\s+"([^"]+)"(?:\s+as\s+(\w+))?$"#).unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let kind = match &caps[1] {
                "robust" => TimelineKind::Robust,
                _ => TimelineKind::Concise,
            };
            let label = caps[2].to_string();
            let id = caps
                .get(3)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| label.clone());

            // Only add if not already declared.
            if !self.timelines.iter().any(|t| t.id == id) {
                self.timelines.push(Timeline {
                    id,
                    label,
                    kind,
                    changes: Vec::new(),
                });
            }
            true
        } else {
            false
        }
    }

    /// Try `Alias is State`.
    fn try_state_change(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)\s+is\s+(.+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let id = caps[1].to_string();
            let state = caps[2].trim().to_string();
            if let Some(tl) = self.timelines.iter_mut().find(|t| t.id == id) {
                tl.changes.push(StateChange {
                    at: self.current_time,
                    state,
                });
                self.time_points.insert(self.current_time);
                true
            } else {
                // Unknown alias — ignore.
                false
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> TimingDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_timing(&lines).unwrap()
    }

    #[test]
    fn basic_timing() {
        let d = parse(
            "robust \"Web\" as W\n\
             concise \"User\" as U\n\
             @0\n\
             W is Idle\n\
             U is Idle\n\
             @100\n\
             W is Processing\n\
             U is Waiting\n\
             @300\n\
             W is Idle\n\
             U is Idle",
        );
        assert_eq!(d.timelines.len(), 2);
        assert_eq!(d.timelines[0].id, "W");
        assert_eq!(d.timelines[0].label, "Web");
        assert_eq!(d.timelines[0].kind, TimelineKind::Robust);
        assert_eq!(d.timelines[1].kind, TimelineKind::Concise);
        assert_eq!(d.timelines[0].changes.len(), 3);
        assert_eq!(d.timelines[0].changes[0].at, 0);
        assert_eq!(d.timelines[0].changes[0].state, "Idle");
        assert_eq!(d.timelines[0].changes[1].at, 100);
        assert_eq!(d.timelines[0].changes[1].state, "Processing");
        assert_eq!(d.time_points, vec![0, 100, 300]);
    }

    #[test]
    fn timeline_without_alias() {
        let d = parse("robust \"Server\"\n@0\nServer is Idle");
        // Without an alias the id equals the label.
        assert_eq!(d.timelines[0].id, "Server");
        assert_eq!(d.timelines[0].label, "Server");
    }

    #[test]
    fn time_points_sorted() {
        let d = parse("robust \"X\" as X\n@300\nX is A\n@0\nX is B\n@150\nX is C");
        assert_eq!(d.time_points, vec![0, 150, 300]);
    }

    #[test]
    fn full_plantuml_input() {
        let d = parse(
            "@startuml\n\
             robust \"Web\" as W\n\
             concise \"User\" as U\n\
             @0\n\
             W is Idle\n\
             U is Idle\n\
             @100\n\
             W is Processing\n\
             U is Waiting\n\
             @300\n\
             W is Idle\n\
             U is Idle\n\
             @enduml",
        );
        assert_eq!(d.timelines.len(), 2);
        assert_eq!(d.time_points.len(), 3);
    }
}
