// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram parser.
//!
//! Parses preprocessed lines into a `SequenceDiagram` model.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::sequence::*;

/// Parse preprocessed lines into a sequence diagram.
pub fn parse_sequence(lines: &[String]) -> Result<SequenceDiagram, ParseError> {
    let mut parser = SeqParser::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        parser.parse_line(i + 1, trimmed)?;
    }

    Ok(parser.finish())
}

struct SeqParser {
    meta: DiagramMeta,
    participants: Vec<Participant>,
    events: Vec<Event>,
    autonumber: Option<AutoNumber>,
    /// Track participant order for auto-creation.
    participant_ids: Vec<String>,
}

impl SeqParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            participants: Vec::new(),
            events: Vec::new(),
            autonumber: None,
            participant_ids: Vec::new(),
        }
    }

    fn finish(self) -> SequenceDiagram {
        SequenceDiagram {
            meta: self.meta,
            participants: self.participants,
            events: self.events,
            autonumber: self.autonumber,
        }
    }

    fn ensure_participant(&mut self, id: &str) -> String {
        let id = id.trim().to_string();
        if !self.participant_ids.contains(&id) {
            self.participant_ids.push(id.clone());
            self.participants.push(Participant {
                id: id.clone(),
                label: id.clone(),
                kind: ParticipantKind::default(),
                order: Some(self.participants.len()),
            });
        }
        id
    }

    fn parse_line(&mut self, line_num: usize, line: &str) -> Result<(), ParseError> {
        // Try each pattern in priority order.
        // Keywords that could be confused with participant names must be
        // checked before the message regex.
        if self.try_autonumber(line) {
            return Ok(());
        }
        if self.try_return(line) {
            return Ok(());
        }
        if self.try_activate_deactivate(line) {
            return Ok(());
        }
        if self.try_create_destroy(line) {
            return Ok(());
        }
        if self.try_participant_decl(line) {
            return Ok(());
        }
        if self.try_group(line) {
            return Ok(());
        }
        if self.try_note(line) {
            return Ok(());
        }
        if self.try_divider(line) {
            return Ok(());
        }
        if self.try_delay(line) {
            return Ok(());
        }
        if self.try_space(line) {
            return Ok(());
        }
        if self.try_message(line) {
            return Ok(());
        }
        if self.try_ref(line) {
            return Ok(());
        }
        if self.try_meta(line) {
            return Ok(());
        }
        if self.try_box(line) {
            return Ok(());
        }
        if self.try_newpage(line) {
            return Ok(());
        }
        if self.try_skinparam(line) {
            return Ok(());
        }
        if self.try_hide(line) {
            return Ok(());
        }
        if self.try_external_message(line) {
            return Ok(());
        }

        // Unknown lines are silently ignored (matches PlantUML behavior).
        let _ = line_num;
        Ok(())
    }

    fn try_participant_decl(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(participant|actor|boundary|control|entity|database|collections|queue)\s+(?:"([^"]+)"\s+as\s+(\w+)|(\w+))"#,
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let kind = parse_participant_kind(&caps[1]);
            let (label, id) = if let Some(quoted) = caps.get(2) {
                (quoted.as_str().to_string(), caps[3].to_string())
            } else {
                let name = caps[4].to_string();
                (name.clone(), name)
            };

            if !self.participant_ids.contains(&id) {
                self.participant_ids.push(id.clone());
                self.participants.push(Participant {
                    id: id.clone(),
                    label,
                    kind,
                    order: Some(self.participants.len()),
                });
            }
            true
        } else {
            false
        }
    }

    fn try_message(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(\w+)\s*([-<>.\\/ox]+)\s*(\w+)\s*(?:((?:\+\+|--|!!))\s*)?(?::\s*(.*))?$")
                .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let from_raw = &caps[1];
            let arrow_str = &caps[2];
            let to_raw = &caps[3];
            let activation_str = caps.get(4).map(|m| m.as_str());
            let label = caps.get(5).map_or("", |m| m.as_str()).trim().to_string();

            let arrow = parse_arrow(arrow_str);
            let activation = activation_str.map(parse_activation);

            let from = self.ensure_participant(from_raw);
            let to = self.ensure_participant(to_raw);

            self.events.push(Event::Message(Message {
                from,
                to,
                label,
                arrow,
                activation,
            }));
            true
        } else {
            false
        }
    }

    fn try_external_message(&mut self, line: &str) -> bool {
        static RE_IN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\[[-=><]+\s*(\w+)\s*(?::\s*(.*))?$").unwrap());
        static RE_OUT: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(\w+)\s*[-=><]+\]\s*(?::\s*(.*))?$").unwrap());

        if let Some(caps) = RE_IN.captures(line) {
            let to = self.ensure_participant(&caps[1]);
            let label = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
            self.events.push(Event::Message(Message {
                from: "[".to_string(),
                to,
                label,
                arrow: Arrow {
                    line: LineStyle::Solid,
                    head: ArrowHead::Filled,
                    direction: ArrowDirection::LeftToRight,
                },
                activation: None,
            }));
            true
        } else if let Some(caps) = RE_OUT.captures(line) {
            let from = self.ensure_participant(&caps[1]);
            let label = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
            self.events.push(Event::Message(Message {
                from,
                to: "]".to_string(),
                label,
                arrow: Arrow {
                    line: LineStyle::Solid,
                    head: ArrowHead::Filled,
                    direction: ArrowDirection::LeftToRight,
                },
                activation: None,
            }));
            true
        } else {
            false
        }
    }

    fn try_note(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(?:h|r)?note\s+(left|right|over)\s*(?:of\s+)?(\w+(?:\s*,\s*\w+)*)?\s*(?::\s*(.*))?$").unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let position = match &caps[1] {
                "left" => NotePosition::Left,
                "right" => NotePosition::Right,
                "over" => NotePosition::Over,
                _ => NotePosition::Right,
            };
            let participants: Vec<String> = caps.get(2).map_or(Vec::new(), |m| {
                m.as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            });
            let text = caps.get(3).map_or("", |m| m.as_str()).trim().to_string();

            self.events.push(Event::Note(Note {
                position,
                participants,
                text,
            }));
            true
        } else {
            false
        }
    }

    fn try_group(&mut self, line: &str) -> bool {
        static RE_START: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(alt|opt|loop|par|break|critical|group)\s*(.*)$").unwrap()
        });
        static RE_ELSE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^else\s*(.*)$").unwrap());

        if line == "end" {
            self.events.push(Event::GroupEnd);
            return true;
        }

        if let Some(caps) = RE_START.captures(line) {
            let kind = match &caps[1] {
                "alt" => GroupKind::Alt,
                "opt" => GroupKind::Opt,
                "loop" => GroupKind::Loop,
                "par" => GroupKind::Par,
                "break" => GroupKind::Break,
                "critical" => GroupKind::Critical,
                "group" => GroupKind::Group,
                _ => GroupKind::Group,
            };
            let label = {
                let l = caps[2].trim();
                if l.is_empty() {
                    None
                } else {
                    Some(l.to_string())
                }
            };
            self.events
                .push(Event::GroupStart(GroupStart { kind, label }));
            return true;
        }

        if let Some(caps) = RE_ELSE.captures(line) {
            let label = {
                let l = caps[1].trim();
                if l.is_empty() {
                    None
                } else {
                    Some(l.to_string())
                }
            };
            self.events.push(Event::GroupElse(GroupElse { label }));
            return true;
        }

        false
    }

    fn try_divider(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^==\s*(.*?)\s*==$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.events.push(Event::Divider(caps[1].trim().to_string()));
            true
        } else {
            false
        }
    }

    fn try_delay(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\.\.\.(.*?)\.\.\.$").unwrap());

        if line == "..." {
            self.events.push(Event::Delay(None));
            return true;
        }
        if let Some(caps) = RE.captures(line) {
            let text = caps[1].trim();
            self.events.push(Event::Delay(if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            }));
            true
        } else {
            false
        }
    }

    fn try_space(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\|\|(\d+)\|\|$").unwrap());

        if line == "|||" {
            self.events.push(Event::Space(None));
            return true;
        }
        if let Some(caps) = RE.captures(line) {
            let px: u32 = caps[1].parse().unwrap_or(20);
            self.events.push(Event::Space(Some(px)));
            true
        } else {
            false
        }
    }

    fn try_autonumber(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^autonumber(?:\s+(\d+))?(?:\s+(\d+))?(?:\s+"([^"]*)")?$"#).unwrap()
        });

        if line == "autonumber" {
            self.autonumber = Some(AutoNumber {
                start: 1,
                step: 1,
                format: None,
            });
            return true;
        }
        if line == "autonumber stop" {
            self.autonumber = None;
            return true;
        }
        if line == "autonumber resume" {
            if self.autonumber.is_none() {
                self.autonumber = Some(AutoNumber {
                    start: 1,
                    step: 1,
                    format: None,
                });
            }
            return true;
        }
        if let Some(caps) = RE.captures(line) {
            let start = caps.get(1).map_or(1, |m| m.as_str().parse().unwrap_or(1));
            let step = caps.get(2).map_or(1, |m| m.as_str().parse().unwrap_or(1));
            let format = caps.get(3).map(|m| m.as_str().to_string());
            self.autonumber = Some(AutoNumber {
                start,
                step,
                format,
            });
            return true;
        }
        false
    }

    fn try_activate_deactivate(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(activate|deactivate)\s+(\w+)").unwrap());

        if let Some(caps) = RE.captures(line) {
            let id = self.ensure_participant(&caps[2]);
            match &caps[1] {
                "activate" => self.events.push(Event::Activate(id)),
                "deactivate" => self.events.push(Event::Deactivate(id)),
                _ => {}
            }
            true
        } else {
            false
        }
    }

    fn try_create_destroy(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(create|destroy)\s+(?:participant\s+)?(\w+)").unwrap());

        if let Some(caps) = RE.captures(line) {
            let id = caps[2].to_string();
            match &caps[1] {
                "create" => {
                    self.ensure_participant(&id);
                    self.events.push(Event::Create(id));
                }
                "destroy" => self.events.push(Event::Destroy(id)),
                _ => {}
            }
            true
        } else {
            false
        }
    }

    fn try_return(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("return") {
            let label = rest.trim().to_string();
            self.events.push(Event::Return(ReturnMessage { label }));
            true
        } else {
            false
        }
    }

    fn try_ref(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^ref\s+over\s+(\w+(?:\s*,\s*\w+)*)\s*:\s*(.+)$").unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let participants: Vec<String> =
                caps[1].split(',').map(|s| s.trim().to_string()).collect();
            let text = caps[2].trim().to_string();
            self.events.push(Event::Ref(Ref { participants, text }));
            true
        } else {
            false
        }
    }

    fn try_meta(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("title ") {
            self.meta.title = Some(rest.trim().to_string());
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
        if let Some(rest) = line.strip_prefix("caption ") {
            self.meta.caption = Some(rest.trim().to_string());
            return true;
        }
        false
    }

    fn try_box(&mut self, line: &str) -> bool {
        // Box declarations affect layout but for now we just skip them.
        line.starts_with("box ") || line == "end box"
    }

    fn try_newpage(&mut self, line: &str) -> bool {
        if line == "newpage" {
            self.events.push(Event::NewPage(None));
            return true;
        }
        if let Some(rest) = line.strip_prefix("newpage ") {
            self.events
                .push(Event::NewPage(Some(rest.trim().to_string())));
            return true;
        }
        false
    }

    fn try_skinparam(&mut self, line: &str) -> bool {
        line.starts_with("skinparam ")
    }

    fn try_hide(&mut self, line: &str) -> bool {
        line.starts_with("hide ")
    }
}

fn parse_participant_kind(s: &str) -> ParticipantKind {
    match s {
        "actor" => ParticipantKind::Actor,
        "boundary" => ParticipantKind::Boundary,
        "control" => ParticipantKind::Control,
        "entity" => ParticipantKind::Entity,
        "database" => ParticipantKind::Database,
        "collections" => ParticipantKind::Collections,
        "queue" => ParticipantKind::Queue,
        _ => ParticipantKind::Participant,
    }
}

fn parse_arrow(s: &str) -> Arrow {
    let line = if s.contains("--") {
        LineStyle::Dotted
    } else {
        LineStyle::Solid
    };

    let head = if s.contains('x') {
        ArrowHead::Cross
    } else if s.contains('o') {
        ArrowHead::Circle
    } else if s.contains(">>") || s.contains("<<") {
        ArrowHead::Open
    } else {
        ArrowHead::Filled
    };

    let direction = if s.contains("<->") {
        ArrowDirection::Bidirectional
    } else if s.contains("<-") || s.contains("<") && !s.contains("->") {
        ArrowDirection::RightToLeft
    } else {
        ArrowDirection::LeftToRight
    };

    Arrow {
        line,
        head,
        direction,
    }
}

fn parse_activation(s: &str) -> ActivationChange {
    match s {
        "++" => ActivationChange::Activate,
        "--" => ActivationChange::Deactivate,
        "!!" => ActivationChange::Destroy,
        _ => ActivationChange::Activate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> SequenceDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_sequence(&lines).unwrap()
    }

    #[test]
    fn simple_message() {
        let d = parse("Alice -> Bob : hello");
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].id, "Alice");
        assert_eq!(d.participants[1].id, "Bob");
        assert_eq!(d.events.len(), 1);
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "Alice");
            assert_eq!(m.to, "Bob");
            assert_eq!(m.label, "hello");
            assert_eq!(m.arrow.line, LineStyle::Solid);
            assert_eq!(m.arrow.head, ArrowHead::Filled);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn dotted_arrow() {
        let d = parse("A --> B : reply");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.arrow.line, LineStyle::Dotted);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn participant_declaration() {
        let d = parse("participant Alice\nactor Bob\nAlice -> Bob : hi");
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].kind, ParticipantKind::Participant);
        assert_eq!(d.participants[1].kind, ParticipantKind::Actor);
    }

    #[test]
    fn participant_alias() {
        let d = parse("participant \"Alice Johnson\" as A\nA -> A : self");
        assert_eq!(d.participants[0].id, "A");
        assert_eq!(d.participants[0].label, "Alice Johnson");
    }

    #[test]
    fn group_alt_else() {
        let d =
            parse("A -> B : check\nalt success\nB --> A : ok\nelse failure\nB --> A : err\nend");
        assert!(matches!(d.events[1], Event::GroupStart(_)));
        assert!(matches!(d.events[3], Event::GroupElse(_)));
        assert!(matches!(d.events[5], Event::GroupEnd));
    }

    #[test]
    fn note() {
        let d = parse("A -> B : msg\nnote right : hello");
        assert_eq!(d.events.len(), 2);
        if let Event::Note(n) = &d.events[1] {
            assert_eq!(n.position, NotePosition::Right);
            assert_eq!(n.text, "hello");
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn divider() {
        let d = parse("A -> B : before\n== Phase 2 ==\nA -> B : after");
        assert!(matches!(d.events[1], Event::Divider(_)));
        if let Event::Divider(text) = &d.events[1] {
            assert_eq!(text, "Phase 2");
        }
    }

    #[test]
    fn delay() {
        let d = parse("A -> B : before\n...5 minutes later...\nA -> B : after");
        assert!(matches!(d.events[1], Event::Delay(Some(_))));
        if let Event::Delay(Some(text)) = &d.events[1] {
            assert_eq!(text, "5 minutes later");
        }
    }

    #[test]
    fn spacing() {
        let d = parse("A -> B : m1\n|||\nA -> B : m2\n||45||\nA -> B : m3");
        assert!(matches!(d.events[1], Event::Space(None)));
        assert!(matches!(d.events[3], Event::Space(Some(45))));
    }

    #[test]
    fn autonumber() {
        let d = parse("autonumber\nA -> B : first\nB -> A : second");
        assert!(d.autonumber.is_some());
        let an = d.autonumber.unwrap();
        assert_eq!(an.start, 1);
        assert_eq!(an.step, 1);
    }

    #[test]
    fn autonumber_with_params() {
        let d = parse("autonumber 10 5 \"[000]\"");
        let an = d.autonumber.unwrap();
        assert_eq!(an.start, 10);
        assert_eq!(an.step, 5);
        assert_eq!(an.format.as_deref(), Some("[000]"));
    }

    #[test]
    fn activation() {
        let d = parse("A -> B ++ : activate\nB --> A -- : return");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.activation, Some(ActivationChange::Activate));
        }
        if let Event::Message(m) = &d.events[1] {
            assert_eq!(m.activation, Some(ActivationChange::Deactivate));
        }
    }

    #[test]
    fn create_destroy() {
        let d = parse("A -> B : normal\ncreate C\nA -> C : create\ndestroy C");
        assert!(matches!(d.events[1], Event::Create(_)));
        assert!(matches!(d.events[3], Event::Destroy(_)));
    }

    #[test]
    fn return_message() {
        let d = parse("A -> B : request\nreturn response");
        assert!(matches!(d.events[1], Event::Return(_)));
        if let Event::Return(r) = &d.events[1] {
            assert_eq!(r.label, "response");
        }
    }

    #[test]
    fn title_and_meta() {
        let d = parse("title My Diagram\nheader Top\nfooter Bottom\ncaption Fig 1\nA -> B : msg");
        assert_eq!(d.meta.title.as_deref(), Some("My Diagram"));
        assert_eq!(d.meta.header.as_deref(), Some("Top"));
        assert_eq!(d.meta.footer.as_deref(), Some("Bottom"));
        assert_eq!(d.meta.caption.as_deref(), Some("Fig 1"));
    }

    #[test]
    fn ref_over() {
        let d = parse("ref over A, B : See other diagram");
        if let Event::Ref(r) = &d.events[0] {
            assert_eq!(r.participants, vec!["A", "B"]);
            assert_eq!(r.text, "See other diagram");
        } else {
            panic!("expected ref");
        }
    }

    #[test]
    fn newpage() {
        let d = parse("A -> B : page1\nnewpage\nA -> B : page2");
        assert!(matches!(d.events[1], Event::NewPage(None)));
    }

    #[test]
    fn all_participant_types() {
        let d = parse(
            "participant P\nactor A\nboundary B\ncontrol C\n\
             entity E\ndatabase D\ncollections Co\nqueue Q",
        );
        assert_eq!(d.participants.len(), 8);
        assert_eq!(d.participants[1].kind, ParticipantKind::Actor);
        assert_eq!(d.participants[2].kind, ParticipantKind::Boundary);
        assert_eq!(d.participants[5].kind, ParticipantKind::Database);
        assert_eq!(d.participants[7].kind, ParticipantKind::Queue);
    }
}
