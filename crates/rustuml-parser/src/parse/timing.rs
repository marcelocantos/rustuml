// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Timing diagram parser.
//!
//! Handles `robust`, `concise`, and `binary` timelines,
//! `@N` / `@+N` time-point directives, `highlight`, and `@T1 <-> @T2 : label`
//! annotations.

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
    /// Highlighted regions.
    highlights: Vec<Highlight>,
    /// Time-range annotations.
    annotations: Vec<Annotation>,
    /// Optional scale.
    scale: Option<Scale>,
    /// Clock timelines: (timeline_id, period). Used to auto-generate time points.
    clock_periods: Vec<(String, i64)>,
    /// Notes attached to timelines.
    notes: Vec<TimingNote>,
}

impl TimingParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            timelines: Vec::new(),
            current_time: 0,
            time_points: BTreeSet::new(),
            highlights: Vec::new(),
            annotations: Vec::new(),
            scale: None,
            clock_periods: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn finish(self) -> TimingDiagram {
        let mut time_points = self.time_points;
        // Generate clock-related time points: for each clock with period N,
        // add t_min + N/2, t_min + N, t_min + 3N/2, ... up to t_max + N.
        if !self.clock_periods.is_empty() {
            let t_min = time_points.iter().next().copied().unwrap_or(0);
            let t_max = time_points.iter().next_back().copied().unwrap_or(0);
            for (_, period) in &self.clock_periods {
                if *period <= 0 {
                    continue;
                }
                // Add half-period boundaries as time points.
                let half = period / 2;
                if half <= 0 {
                    continue;
                }
                let mut t = t_min + half;
                while t <= t_max + period {
                    time_points.insert(t);
                    t += half;
                }
            }
        }
        let time_points = time_points.into_iter().collect();
        TimingDiagram {
            meta: self.meta,
            timelines: self.timelines,
            time_points,
            highlights: self.highlights,
            annotations: self.annotations,
            scale: self.scale,
            notes: self.notes,
        }
    }

    fn parse_line(&mut self, line_num: usize, line: &str) -> Result<(), ParseError> {
        // Skip @startuml / @enduml.
        if line.starts_with('@') {
            self.try_time_point(line);
            self.try_annotation(line);
            return Ok(());
        }

        if self.try_meta(line) {
            return Ok(());
        }
        if self.try_timeline_decl(line) {
            return Ok(());
        }
        if self.try_clock_decl(line) {
            return Ok(());
        }
        if self.try_state_change(line) {
            return Ok(());
        }
        if self.try_highlight(line) {
            return Ok(());
        }
        if self.try_scale(line) {
            return Ok(());
        }
        if self.try_note(line) {
            return Ok(());
        }

        // Collect skinparam directives into metadata.
        if let Some(rest) = line.strip_prefix("skinparam ") {
            if let Some((key, value)) = rest.split_once(' ') {
                self.meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            return Ok(());
        }

        // Silently ignore unrecognised lines.
        let _ = line_num;
        Ok(())
    }

    /// Try to parse `@N` (absolute) or `@+N` (relative) time marker.
    fn try_time_point(&mut self, line: &str) -> bool {
        static RE_ABS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^@(-?\d+)$").unwrap());
        static RE_REL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^@\+(\d+)$").unwrap());

        if let Some(caps) = RE_ABS.captures(line)
            && let Ok(t) = caps[1].parse::<i64>()
        {
            self.current_time = t;
            self.time_points.insert(t);
            return true;
        }
        if let Some(caps) = RE_REL.captures(line)
            && let Ok(delta) = caps[1].parse::<i64>()
        {
            self.current_time += delta;
            self.time_points.insert(self.current_time);
            return true;
        }
        false
    }

    /// Try `@T1 <-> @T2 : label` annotation.
    fn try_annotation(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^@(-?\d+)\s*<->\s*@(-?\d+)\s*:\s*(.+)$").unwrap());
        if let Some(caps) = RE.captures(line)
            && let Ok(t1) = caps[1].parse::<i64>()
            && let Ok(t2) = caps[2].parse::<i64>()
        {
            let label = caps[3].trim().to_string();
            self.annotations.push(Annotation {
                from: t1.min(t2),
                to: t1.max(t2),
                label,
            });
            return true;
        }
        false
    }

    /// Try `title`, `header`, `footer`.
    fn try_meta(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("title ") {
            self.meta.title = Some(super::strip_title_quotes(rest).to_string());
            return true;
        }
        if let Some(rest) = line.strip_prefix("header ") {
            self.meta.header = Some(rest.trim().to_string());
            return true;
        }
        if let Some(rest) = line.strip_prefix("footer ") {
            self.meta.footer = Some(rest.trim().to_string());
            return true;
        }
        false
    }

    /// Try `robust "Label" as Alias`, `concise "Label" as Alias`,
    /// or `binary "Label" as Alias`.
    fn try_timeline_decl(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^(robust|concise|binary)\s+"([^"]+)"(?:\s+as\s+(\w+))?$"#).unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let kind = match &caps[1] {
                "robust" => TimelineKind::Robust,
                "binary" => TimelineKind::Binary,
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

    /// Try `clock "Label" as Alias with period N`.
    /// Creates a Binary timeline. The clock auto-toggles each half-period,
    /// but we store it as a plain binary timeline; the actual waveform is
    /// generated from state changes in the diagram. For the purposes of
    /// label rendering, we just register the timeline.
    fn try_clock_decl(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^clock\s+"([^"]+)"(?:\s+as\s+(\w+))?\s+with\s+period\s+(\d+)$"#).unwrap()
        });
        if let Some(caps) = RE.captures(line) {
            let label = caps[1].to_string();
            let id = caps
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| label.clone());
            let period: i64 = caps[3].parse().unwrap_or(10);
            // Store period for time-point generation in finish().
            self.clock_periods.push((id.clone(), period));
            // Register as Binary timeline.
            if !self.timelines.iter().any(|t| t.id == id) {
                self.timelines.push(Timeline {
                    id,
                    label,
                    kind: TimelineKind::Binary,
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
            let state_raw = caps[2].trim().to_string();
            // Strip optional color prefix `#color : state` → just keep state name.
            // PlantUML supports `proc is #blue : active` but we store only the state name.
            let state = if let Some(colon_pos) = state_raw.find(" : ") {
                state_raw[colon_pos + 3..].trim().to_string()
            } else {
                state_raw
            };
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

    /// Try `highlight T1 to T2 #color : label` or `highlight T1 to T2 : label`.
    fn try_highlight(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^highlight\s+(-?\d+)\s+to\s+(-?\d+)(?:\s+(#\S+))?(?:\s*:\s*(.+))?$")
                .unwrap()
        });
        if let Some(caps) = RE.captures(line)
            && let Ok(t1) = caps[1].parse::<i64>()
            && let Ok(t2) = caps[2].parse::<i64>()
        {
            let color = caps.get(3).map(|m| m.as_str().to_string());
            let label = caps.get(4).map(|m| m.as_str().trim().to_string());
            self.highlights.push(Highlight {
                from: t1.min(t2),
                to: t1.max(t2),
                color,
                label,
            });
            return true;
        }
        false
    }

    /// Try `note top of X : text` or `note bottom of X : text`.
    fn try_note(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^note\s+(top|bottom)\s+of\s+(\w+)\s*:\s*(.+)$").unwrap());
        if let Some(caps) = RE.captures(line) {
            let above = &caps[1] == "top";
            let timeline_id = caps[2].to_string();
            let text = caps[3].trim().to_string();
            self.notes.push(TimingNote {
                timeline_id,
                at: self.current_time,
                text,
                above,
            });
            return true;
        }
        // Consume bare `note` lines silently.
        line.starts_with("note ")
    }

    /// Try `scale N as M pixels` or `scale N`.
    fn try_scale(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^scale\s+(\d+)(?:\s+as\s+(\d+)\s+pixels?)?$").unwrap());
        if let Some(caps) = RE.captures(line)
            && let Ok(units) = caps[1].parse::<i64>()
        {
            let pixels = caps
                .get(2)
                .and_then(|m| m.as_str().parse::<i64>().ok())
                .unwrap_or(50); // default 50px per N units
            self.scale = Some(Scale { units, pixels });
            return true;
        }
        false
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
    fn binary_timeline() {
        let d = parse("binary \"CLK\" as clk\n@0\nclk is low\n@50\nclk is high");
        assert_eq!(d.timelines.len(), 1);
        assert_eq!(d.timelines[0].kind, TimelineKind::Binary);
        assert_eq!(d.timelines[0].label, "CLK");
        assert_eq!(d.timelines[0].changes[1].state, "high");
    }

    #[test]
    fn relative_time_points() {
        let d = parse("robust \"X\" as X\n@0\nX is A\n@+50\nX is B\n@+100\nX is C");
        assert_eq!(d.time_points, vec![0, 50, 150]);
        assert_eq!(d.timelines[0].changes[2].at, 150);
    }

    #[test]
    fn highlight_directive() {
        let d = parse(
            "robust \"S\" as s\n\
             highlight 100 to 200 #lightyellow : critical\n\
             @0\ns is low\n@100\ns is high\n@200\ns is low",
        );
        assert_eq!(d.highlights.len(), 1);
        assert_eq!(d.highlights[0].from, 100);
        assert_eq!(d.highlights[0].to, 200);
        assert_eq!(d.highlights[0].label.as_deref(), Some("critical"));
    }

    #[test]
    fn annotation_directive() {
        let d = parse(
            "robust \"TX\" as tx\n@0\ntx is idle\n@50\ntx is active\n@100\ntx is idle\n\
             @50 <-> @100 : propagation 0ms",
        );
        assert_eq!(d.annotations.len(), 1);
        assert_eq!(d.annotations[0].label, "propagation 0ms");
    }

    #[test]
    fn title_header_footer() {
        let d = parse(
            "title My Title\nheader My Header\nfooter My Footer\n\
             robust \"S\" as s\n@0\ns is low",
        );
        assert_eq!(d.meta.title.as_deref(), Some("My Title"));
        assert_eq!(d.meta.header.as_deref(), Some("My Header"));
        assert_eq!(d.meta.footer.as_deref(), Some("My Footer"));
    }

    #[test]
    fn scale_directive() {
        let d = parse("scale 10 as 50 pixels\nrobust \"S\" as s\n@0\ns is low");
        let scale = d.scale.unwrap();
        assert_eq!(scale.units, 10);
        assert_eq!(scale.pixels, 50);
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
