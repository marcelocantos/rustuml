// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram parser (v3 / new syntax).

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::activity::*;

pub fn parse_activity(lines: &[String]) -> Result<ActivityDiagram, ParseError> {
    let mut parser = ActivityParser::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if parser.pending_note.is_some() {
                parser.accumulate_note_line("");
            }
            continue;
        }
        parser.parse_line(i + 1, trimmed)?;
    }
    Ok(parser.finish())
}

struct PendingNote {
    position: NotePosition,
    color: Option<String>,
    lines: Vec<String>,
}

struct ActivityParser {
    meta: DiagramMeta,
    steps: Vec<ActivityStep>,
    pending_note: Option<PendingNote>,
}

impl ActivityParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            steps: Vec::new(),
            pending_note: None,
        }
    }

    fn finish(self) -> ActivityDiagram {
        ActivityDiagram {
            meta: self.meta,
            steps: self.steps,
        }
    }

    fn accumulate_note_line(&mut self, line: &str) {
        if let Some(ref mut pn) = self.pending_note {
            pn.lines.push(line.to_string());
        }
    }

    fn flush_pending_note(&mut self) {
        if let Some(pn) = self.pending_note.take() {
            let text = pn.lines.join("\n");
            let text = text.trim().to_string();
            if !text.is_empty() {
                self.steps.push(ActivityStep::Note(NoteBlock {
                    text,
                    color: pn.color,
                    position: pn.position,
                }));
            }
        }
    }

    fn parse_line(&mut self, _line_num: usize, line: &str) -> Result<(), ParseError> {
        if self.pending_note.is_some() {
            if line == "end note" {
                self.flush_pending_note();
            } else {
                self.accumulate_note_line(line);
            }
            return Ok(());
        }

        match line {
            "start" => self.steps.push(ActivityStep::Start),
            "stop" => self.steps.push(ActivityStep::Stop),
            "end" => self.steps.push(ActivityStep::End),
            "endif" => self.steps.push(ActivityStep::EndIf),
            "endswitch" => self.steps.push(ActivityStep::EndSwitch),
            "fork" => self.steps.push(ActivityStep::Fork),
            "fork again" => self.steps.push(ActivityStep::ForkAgain),
            "end fork" => self.steps.push(ActivityStep::EndFork),
            "split" => self.steps.push(ActivityStep::Split),
            "split again" => self.steps.push(ActivityStep::SplitAgain),
            "end split" => self.steps.push(ActivityStep::EndSplit),
            "repeat" => self.steps.push(ActivityStep::Repeat),
            "break" => self.steps.push(ActivityStep::Break),
            "detach" => self.steps.push(ActivityStep::Detach),
            "kill" => self.steps.push(ActivityStep::Kill),
            _ => {
                if !self.try_meta(line)
                    && !self.try_action(line)
                    && !self.try_deprecated_color_action(line)
                    && !self.try_arrow(line)
                    && !self.try_backward(line)
                    && !self.try_if(line)
                    && !self.try_elseif(line)
                    && !self.try_else(line)
                    && !self.try_while(line)
                    && !self.try_endwhile(line)
                    && !self.try_repeat_while(line)
                    && !self.try_switch(line)
                    && !self.try_case(line)
                    && !self.try_swimlane(line)
                    && !self.try_partition(line)
                    && !self.try_note(line)
                {
                    // Silently ignore unknown lines.
                }
            }
        }
        Ok(())
    }

    fn try_meta(&mut self, line: &str) -> bool {
        static RE_TITLE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^title\s+(.+)$").unwrap());
        static RE_HEADER: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^header\s+(.+)$").unwrap());
        static RE_FOOTER: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^footer\s+(.+)$").unwrap());
        static RE_CAPTION: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^caption\s+(.+)$").unwrap());

        if let Some(caps) = RE_TITLE.captures(line) {
            self.meta.title = Some(super::strip_title_quotes(&caps[1]).to_string());
            return true;
        }
        if let Some(caps) = RE_HEADER.captures(line) {
            self.meta.header = Some(caps[1].trim().to_string());
            return true;
        }
        if let Some(caps) = RE_FOOTER.captures(line) {
            self.meta.footer = Some(caps[1].trim().to_string());
            return true;
        }
        if let Some(caps) = RE_CAPTION.captures(line) {
            self.meta.caption = Some(caps[1].trim().to_string());
            return true;
        }
        false
    }

    fn try_action(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^:(.+);$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps
                .push(ActivityStep::Action(caps[1].trim().to_string()));
            true
        } else {
            false
        }
    }

    fn try_deprecated_color_action(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(#(?:[0-9A-Fa-f]{6}|[A-Za-z]+)):(.+);$").unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let raw_color = caps[1].trim().to_string();
            let color = if raw_color.len() == 7
                && raw_color.starts_with('#')
                && raw_color[1..].chars().all(|c| c.is_ascii_hexdigit())
            {
                format!("#{}", raw_color[1..].to_uppercase())
            } else {
                raw_color
            };
            let text = caps[2].trim().to_string();
            self.steps
                .push(ActivityStep::DeprecatedColorAction(DeprecatedColorAction {
                    color,
                    text,
                }));
            true
        } else {
            false
        }
    }

    fn try_arrow(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(-(?:\[([^\]]+)\])?-?->)\s*(.+)?$").unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let arrow_str = &caps[1];
            let dashed = arrow_str.contains("-->");
            let color = caps.get(2).map(|m| {
                let s = m.as_str().trim();
                if s.starts_with('#') && s.len() == 7 && s[1..].chars().all(|c| c.is_ascii_hexdigit()) {
                    format!("#{}", s[1..].to_uppercase())
                } else {
                    s.to_string()
                }
            });
            let label = caps.get(3).map(|m| {
                let s = m.as_str().trim();
                s.trim_end_matches(';').trim().to_string()
            }).filter(|s| !s.is_empty());
            self.steps.push(ActivityStep::Arrow(ArrowStep {
                dashed,
                color,
                label,
            }));
            true
        } else {
            false
        }
    }

    fn try_backward(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^backward\s*:(.+);$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps
                .push(ActivityStep::Backward(caps[1].trim().to_string()));
            true
        } else {
            false
        }
    }

    fn try_if(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^if\s*\((.+?)\)\s*then\s*(?:\((.+?)\))?$").unwrap());
        static RE_BARE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^if\s*\((.+?)\)\s*$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps.push(ActivityStep::If(IfBlock {
                condition: caps[1].trim().to_string(),
                then_label: caps.get(2).map(|m| m.as_str().trim().to_string()),
            }));
            true
        } else if let Some(caps) = RE_BARE.captures(line) {
            self.steps.push(ActivityStep::If(IfBlock {
                condition: caps[1].trim().to_string(),
                then_label: None,
            }));
            true
        } else {
            false
        }
    }

    fn try_elseif(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^elseif\s*\((.+?)\)\s*then\s*(?:\((.+?)\))?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps.push(ActivityStep::ElseIf(ElseIfBranch {
                condition: caps[1].trim().to_string(),
                then_label: caps.get(2).map(|m| m.as_str().trim().to_string()),
            }));
            true
        } else {
            false
        }
    }

    fn try_else(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^else\s*(?:\((.+?)\))?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let label = caps.get(1).map(|m| m.as_str().trim().to_string());
            self.steps.push(ActivityStep::Else(label));
            true
        } else {
            false
        }
    }

    fn try_while(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^while\s*\((.+?)\)\s*(?:is\s*\((.+?)\))?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps.push(ActivityStep::While(WhileBlock {
                condition: caps[1].trim().to_string(),
                is_label: caps.get(2).map(|m| m.as_str().trim().to_string()),
            }));
            true
        } else {
            false
        }
    }

    fn try_endwhile(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^endwhile\s*(?:\((.+?)\))?$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let label = caps.get(1).map(|m| m.as_str().trim().to_string());
            self.steps.push(ActivityStep::EndWhile(label));
            true
        } else {
            false
        }
    }

    fn try_repeat_while(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^repeat\s*while\s*\((.+?)\)\s*(?:is\s*\((.+?)\))?").unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            self.steps.push(ActivityStep::RepeatWhile(RepeatWhileBlock {
                condition: caps[1].trim().to_string(),
                is_label: caps.get(2).map(|m| m.as_str().trim().to_string()),
            }));
            true
        } else {
            false
        }
    }

    fn try_switch(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^switch\s*\((.+?)\)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps
                .push(ActivityStep::Switch(caps[1].trim().to_string()));
            true
        } else {
            false
        }
    }

    fn try_case(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^case\s*\(\s*(.+?)\s*\)$").unwrap());

        if line == "default" {
            self.steps.push(ActivityStep::Case("default".to_string()));
            return true;
        }
        if let Some(caps) = RE.captures(line) {
            self.steps
                .push(ActivityStep::Case(caps[1].trim().to_string()));
            true
        } else {
            false
        }
    }

    fn try_swimlane(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\|([^|]+)\|$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.steps.push(ActivityStep::Swimlane(caps[1].to_string()));
            true
        } else {
            false
        }
    }

    fn try_partition(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^partition\s+(?:(#[A-Za-z0-9]+)\s+)?(?:"([^"]+)"|([A-Za-z_]\w*))\s*\{?"#,
            )
            .unwrap()
        });

        if line == "}" {
            self.steps.push(ActivityStep::EndPartition);
            return true;
        }
        if let Some(caps) = RE.captures(line) {
            let color = caps.get(1).map(|m| m.as_str().to_string());
            let name = caps
                .get(2)
                .or(caps.get(3))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            self.steps
                .push(ActivityStep::Partition(PartitionBlock { name, color }));
            true
        } else {
            false
        }
    }

    fn try_note(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^note\s+(left|right)(?:\s+(#[0-9A-Fa-f]{6}|#\w+))?(?:\s*:\s*(.+))?$")
                .unwrap()
        });

        if !line.starts_with("note ") {
            return false;
        }

        if let Some(caps) = RE.captures(line) {
            let position = if &caps[1] == "left" {
                NotePosition::Left
            } else {
                NotePosition::Right
            };
            let color = caps.get(2).map(|m| {
                let s = m.as_str();
                if s.starts_with('#') && s.len() == 7 && s[1..].chars().all(|c| c.is_ascii_hexdigit()) {
                    format!("#{}", s[1..].to_uppercase())
                } else {
                    s.to_string()
                }
            });
            if let Some(inline_text) = caps.get(3) {
                let text = inline_text.as_str().trim().to_string();
                if !text.is_empty() {
                    self.steps.push(ActivityStep::Note(NoteBlock {
                        text,
                        color,
                        position,
                    }));
                }
            } else {
                self.pending_note = Some(PendingNote {
                    position,
                    color,
                    lines: Vec::new(),
                });
            }
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> ActivityDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_activity(&lines).unwrap()
    }

    #[test]
    fn basic_activity() {
        let d = parse("start\n:Step 1;\n:Step 2;\nstop");
        assert_eq!(d.steps.len(), 4);
        assert!(matches!(d.steps[0], ActivityStep::Start));
        assert!(matches!(d.steps[1], ActivityStep::Action(ref s) if s == "Step 1"));
        assert!(matches!(d.steps[3], ActivityStep::Stop));
    }

    #[test]
    fn if_else() {
        let d = parse(
            "start\nif (x > 0?) then (yes)\n  :positive;\nelse (no)\n  :negative;\nendif\nstop",
        );
        assert!(matches!(d.steps[1], ActivityStep::If(_)));
        if let ActivityStep::If(ref b) = d.steps[1] {
            assert_eq!(b.condition, "x > 0?");
            assert_eq!(b.then_label.as_deref(), Some("yes"));
        }
        assert!(matches!(d.steps[3], ActivityStep::Else(Some(ref s)) if s == "no"));
        assert!(matches!(d.steps[5], ActivityStep::EndIf));
    }

    #[test]
    fn elseif() {
        let d = parse(
            "start\nif (a?) then (yes)\n  :a;\nelseif (b?) then (maybe)\n  :b;\nelse (no)\n  :c;\nendif\nstop",
        );
        assert!(matches!(d.steps[3], ActivityStep::ElseIf(_)));
    }

    #[test]
    fn while_loop() {
        let d = parse("start\nwhile (cond?) is (yes)\n  :process;\nendwhile (no)\nstop");
        assert!(matches!(d.steps[1], ActivityStep::While(_)));
        if let ActivityStep::While(ref w) = d.steps[1] {
            assert_eq!(w.condition, "cond?");
            assert_eq!(w.is_label.as_deref(), Some("yes"));
        }
        assert!(matches!(d.steps[3], ActivityStep::EndWhile(Some(ref s)) if s == "no"));
    }

    #[test]
    fn repeat_loop() {
        let d = parse("start\nrepeat\n  :action;\nrepeat while (again?)\nstop");
        assert!(matches!(d.steps[1], ActivityStep::Repeat));
        assert!(matches!(d.steps[3], ActivityStep::RepeatWhile(ref b) if b.condition == "again?"));
    }

    #[test]
    fn fork() {
        let d = parse("start\nfork\n  :A;\nfork again\n  :B;\nend fork\nstop");
        assert!(matches!(d.steps[1], ActivityStep::Fork));
        assert!(matches!(d.steps[3], ActivityStep::ForkAgain));
        assert!(matches!(d.steps[5], ActivityStep::EndFork));
    }

    #[test]
    fn split() {
        let d = parse("start\nsplit\n  :A;\nsplit again\n  :B;\nend split\nstop");
        assert!(matches!(d.steps[1], ActivityStep::Split));
        assert!(matches!(d.steps[3], ActivityStep::SplitAgain));
        assert!(matches!(d.steps[5], ActivityStep::EndSplit));
    }

    #[test]
    fn switch_case() {
        let d = parse(
            "start\nswitch (test?)\ncase ( A )\n  :action A;\ncase ( B )\n  :action B;\nendswitch\nstop",
        );
        assert!(matches!(d.steps[1], ActivityStep::Switch(ref s) if s == "test?"));
        assert!(matches!(d.steps[2], ActivityStep::Case(ref s) if s == "A"));
        assert!(matches!(d.steps[4], ActivityStep::Case(ref s) if s == "B"));
        assert!(matches!(d.steps[6], ActivityStep::EndSwitch));
    }

    #[test]
    fn swimlanes() {
        let d = parse("|Lane1|\nstart\n:task1;\n|Lane2|\n:task2;\nstop");
        assert!(matches!(d.steps[0], ActivityStep::Swimlane(ref s) if s == "Lane1"));
        assert!(matches!(d.steps[3], ActivityStep::Swimlane(ref s) if s == "Lane2"));
    }

    #[test]
    fn swimlane_with_spaces() {
        let d = parse("|New Employee|\nstart\n:task;\nstop");
        assert!(matches!(d.steps[0], ActivityStep::Swimlane(ref s) if s == "New Employee"));
    }

    #[test]
    fn partition() {
        let d = parse("start\npartition Init {\n  :step1;\n}\nstop");
        assert!(matches!(d.steps[1], ActivityStep::Partition(ref s) if s.name == "Init"));
        assert!(matches!(d.steps[3], ActivityStep::EndPartition));
    }

    #[test]
    fn detach_and_kill() {
        let d = parse("start\ndetach");
        assert!(matches!(d.steps[1], ActivityStep::Detach));

        let d2 = parse("start\nkill");
        assert!(matches!(d2.steps[1], ActivityStep::Kill));
    }

    #[test]
    fn title_parsed() {
        let d = parse("title My Diagram\nstart\nstop");
        assert_eq!(d.meta.title.as_deref(), Some("My Diagram"));
    }

    #[test]
    fn deprecated_color_action() {
        let d = parse("start\n#blue:Do something;\nstop");
        assert!(matches!(d.steps[1], ActivityStep::DeprecatedColorAction(ref a) if a.text == "Do something" && a.color == "#blue"));
    }

    #[test]
    fn arrow_steps() {
        let d = parse("start\n:A;\n->\n:B;\n-->\n:C;\nstop");
        assert!(matches!(d.steps[2], ActivityStep::Arrow(ref a) if !a.dashed && a.label.is_none()));
        assert!(matches!(d.steps[4], ActivityStep::Arrow(ref a) if a.dashed && a.label.is_none()));
    }

    #[test]
    fn arrow_with_label() {
        let d = parse("start\n:A;\n--> label;\n:B;\nstop");
        if let ActivityStep::Arrow(ref a) = d.steps[2] {
            assert!(a.dashed);
            assert_eq!(a.label.as_deref(), Some("label"));
        } else {
            panic!("expected Arrow");
        }
    }

    #[test]
    fn backward_step() {
        let d = parse("start\nrepeat\n:action;\nbackward :retry;\nrepeat while (again?)\nstop");
        assert!(matches!(d.steps[3], ActivityStep::Backward(ref s) if s == "retry"));
    }
}
