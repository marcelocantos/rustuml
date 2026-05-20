// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::state::*;

pub fn parse_state(lines: &[String]) -> Result<StateDiagram, ParseError> {
    let mut parser = StateParser::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            // Empty lines may terminate a multi-line note with content already
            // accumulated — keep buffering (blank lines are part of note body).
            if let Some(buf) = &mut parser.note_buffer {
                buf.text.push('\n');
            }
            continue;
        }
        parser.parse_line(i + 1, trimmed)?;
    }
    // Flush any unclosed note buffer.
    parser.flush_note();
    Ok(parser.finish())
}

/// Accumulator for a multi-line note body.
struct NoteBuffer {
    kind: StateNoteKind,
    text: String,
}

struct StateParser {
    meta: DiagramMeta,
    states: Vec<State>,
    transitions: Vec<Transition>,
    notes: Vec<StateNote>,
    /// Active multi-line note being accumulated.
    note_buffer: Option<NoteBuffer>,
    /// Current 1-based source line number (set before each parse_line call).
    current_line: usize,
}

impl StateParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            states: Vec::new(),
            transitions: Vec::new(),
            notes: Vec::new(),
            note_buffer: None,
            current_line: 0,
        }
    }

    fn flush_note(&mut self) {
        if let Some(buf) = self.note_buffer.take() {
            let text = buf.text.trim().to_string();
            if !text.is_empty() {
                self.notes.push(StateNote {
                    text,
                    kind: buf.kind,
                });
            }
        }
    }

    fn finish(self) -> StateDiagram {
        StateDiagram {
            meta: self.meta,
            states: self.states,
            transitions: self.transitions,
            notes: self.notes,
        }
    }

    fn ensure_state(&mut self, id: &str) -> String {
        let id = id.trim().to_string();
        // Pseudo-states ([*], [H], [H*]) are handled by the renderer directly
        // and do not need a corresponding State entry in the states list.
        if id.starts_with('[') && id.ends_with(']') {
            return id;
        }
        if !self.states.iter().any(|s| s.id == id) {
            self.states.push(State {
                id: id.clone(),
                label: id.clone(),
                kind: StateKind::Normal,
                descriptions: Vec::new(),
                substates: Vec::new(),
                source_line: self.current_line,
            });
        }
        id
    }

    fn parse_line(&mut self, line_num: usize, line: &str) -> Result<(), ParseError> {
        self.current_line = line_num;
        // Handle multi-line note body accumulation.
        if self.note_buffer.is_some() {
            if line == "end note" || line == "endnote" {
                self.flush_note();
            } else {
                let buf = self.note_buffer.as_mut().unwrap();
                if !buf.text.is_empty() {
                    buf.text.push('\n');
                }
                buf.text.push_str(line);
            }
            return Ok(());
        }

        // Title directive.
        if let Some(rest) = line.strip_prefix("title ") {
            self.meta.title = Some(super::strip_title_quotes(rest).to_string());
            return Ok(());
        }
        // Parse skinparam directives.
        if let Some(rest) = line.strip_prefix("skinparam ") {
            if let Some((key, value)) = rest.split_once(' ') {
                self.meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            return Ok(());
        }
        // `hide empty description[s]` / `show empty description[s]` — capture
        // as skinparam so the renderer can switch to the no-divider 40px box.
        // Other `hide ` / `show ` decoration lines are still ignored.
        if let Some(rest) = line
            .strip_prefix("hide ")
            .or_else(|| line.strip_prefix("show "))
        {
            let show = line.starts_with("show ");
            if matches!(rest, "empty description" | "empty descriptions") {
                self.meta.skinparams.push(crate::diagram::SkinParam {
                    key: "hideEmptyDescription".to_string(),
                    value: if show { "false" } else { "true" }.to_string(),
                });
            }
            return Ok(());
        }

        if self.try_transition(line) {
            return Ok(());
        }
        if self.try_state_decl(line) {
            return Ok(());
        }
        if self.try_state_description(line) {
            return Ok(());
        }
        if self.try_hide(line) {
            return Ok(());
        }
        if self.try_note(line) {
            return Ok(());
        }
        // Silently ignore unknown lines (}, state body, etc.)
        Ok(())
    }

    fn try_transition(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            // State IDs may include dots for substate references (e.g. `S.H`).
            // Pseudo-states: [*] (initial/final), [H] (shallow history), [H*] (deep history).
            Regex::new(
                r"^(\[[\w*]*\]|[\w.]+)\s*-+(?:left|right|up|down|le|ri|do)?-*>\s*(\[[\w*]*\]|[\w.]+)(?:\s*:\s*(.+))?$",
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let from = self.ensure_state(&caps[1]);
            let to = self.ensure_state(&caps[2]);
            let label = caps.get(3).map(|m| m.as_str().trim().to_string());
            self.transitions.push(Transition {
                from,
                to,
                label,
                source_line: self.current_line,
            });
            true
        } else {
            false
        }
    }

    fn try_state_decl(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^state\s+(?:"([^"]+)"\s+as\s+)?(\w+)(?:\s*<<(\w+\*?)>>)?(?:\s*(#\w+))?(?:\s*\{)?$"#)
                .unwrap()
        });
        // Also handles: state ID : description
        static RE_DESC: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^state\s+(?:"([^"]+)"\s+as\s+)?(\w+)\s*:\s*(.+)$"#).unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let label = caps
                .get(1)
                .map_or_else(|| caps[2].to_string(), |m| m.as_str().to_string());
            let id = caps[2].to_string();
            let stereotype = caps.get(3).map(|m| m.as_str());

            let kind = match stereotype {
                Some("start") => StateKind::Initial,
                Some("end") => StateKind::Final,
                Some("choice") => StateKind::Choice,
                Some("fork") => StateKind::Fork,
                Some("join") => StateKind::Join,
                Some("history") => StateKind::History,
                Some("history*") => StateKind::DeepHistory,
                _ => StateKind::Normal,
            };

            if let Some(state) = self.states.iter_mut().find(|s| s.id == id) {
                state.label = label;
                state.kind = kind;
                if state.source_line == 0 {
                    state.source_line = self.current_line;
                }
            } else {
                self.states.push(State {
                    id: id.clone(),
                    label,
                    kind,
                    descriptions: Vec::new(),
                    substates: Vec::new(),
                    source_line: self.current_line,
                });
            }
            true
        } else if let Some(caps) = RE_DESC.captures(line) {
            let label = caps
                .get(1)
                .map_or_else(|| caps[2].to_string(), |m| m.as_str().to_string());
            let id = caps[2].to_string();
            let desc = caps[3].trim().to_string();
            if let Some(state) = self.states.iter_mut().find(|s| s.id == id) {
                state.label = label;
                state.descriptions.push(desc);
            } else {
                self.states.push(State {
                    id: id.clone(),
                    label,
                    kind: StateKind::Normal,
                    descriptions: vec![desc],
                    substates: Vec::new(),
                    source_line: self.current_line,
                });
            }
            true
        } else {
            false
        }
    }

    fn try_state_description(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)\s*:\s*(.+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let id = caps[1].to_string();
            let desc = caps[2].trim().to_string();
            self.ensure_state(&id);
            if let Some(state) = self.states.iter_mut().find(|s| s.id == id) {
                state.descriptions.push(desc);
            }
            true
        } else {
            false
        }
    }

    fn try_hide(&mut self, line: &str) -> bool {
        line.starts_with("hide ")
    }

    fn try_note(&mut self, line: &str) -> bool {
        if !line.starts_with("note") {
            return false;
        }

        // `note on link` — note on the most recent transition.
        if line == "note on link"
            || line.starts_with("note on link ")
            || line.starts_with("note on link:")
        {
            let inline = line
                .strip_prefix("note on link")
                .and_then(|r| {
                    r.strip_prefix(" : ")
                        .or_else(|| r.strip_prefix(": "))
                        .or_else(|| r.strip_prefix(':'))
                })
                .map(|s| s.trim());
            if let Some(text) = inline.filter(|t| !t.is_empty()) {
                self.notes.push(StateNote {
                    text: text.to_string(),
                    kind: StateNoteKind::OnLink,
                });
            } else {
                self.note_buffer = Some(NoteBuffer {
                    kind: StateNoteKind::OnLink,
                    text: String::new(),
                });
            }
            return true;
        }

        // `note "floating text" as ALIAS`
        {
            static RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r#"^note\s+"([^"]+)"\s+as\s+\w+$"#).unwrap());
            if let Some(caps) = RE.captures(line) {
                let text = caps[1].to_string();
                self.notes.push(StateNote {
                    text,
                    kind: StateNoteKind::Floating,
                });
                return true;
            }
        }

        // `note left of <state> [: text]` or `note right of <state> [: text]`
        {
            static RE: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new(r"^note\s+(left|right)\s+of\s+(\w+)(?:\s*:\s*(.+))?$").unwrap()
            });
            if let Some(caps) = RE.captures(line) {
                let side = &caps[1];
                let state_id = caps[2].to_string();
                let kind = if side == "left" {
                    StateNoteKind::LeftOf(state_id)
                } else {
                    StateNoteKind::RightOf(state_id)
                };
                if let Some(text) = caps
                    .get(3)
                    .map(|m| m.as_str().trim().to_string())
                    .filter(|t| !t.is_empty())
                {
                    self.notes.push(StateNote { text, kind });
                } else {
                    self.note_buffer = Some(NoteBuffer {
                        kind,
                        text: String::new(),
                    });
                }
                return true;
            }
        }

        // `note left [: text]` / `note right [: text]` (no "of <state>")
        {
            static RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"^note\s+(left|right)(?:\s*:\s*(.+))?$").unwrap());
            if let Some(caps) = RE.captures(line) {
                let side = &caps[1];
                let kind = if side == "left" {
                    StateNoteKind::LeftOf(String::new())
                } else {
                    StateNoteKind::RightOf(String::new())
                };
                if let Some(text) = caps
                    .get(2)
                    .map(|m| m.as_str().trim().to_string())
                    .filter(|t| !t.is_empty())
                {
                    self.notes.push(StateNote { text, kind });
                } else {
                    self.note_buffer = Some(NoteBuffer {
                        kind,
                        text: String::new(),
                    });
                }
                return true;
            }
        }

        // Catch-all: any remaining `note ...` line is silently consumed.
        if line.starts_with("note ") {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> StateDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_state(&lines).unwrap()
    }

    #[test]
    fn basic_transitions() {
        let d = parse("[*] --> Active\nActive --> Inactive : disable\nActive --> [*] : close");
        assert_eq!(d.transitions.len(), 3);
        assert_eq!(d.transitions[0].from, "[*]");
        assert_eq!(d.transitions[0].to, "Active");
        assert_eq!(d.transitions[1].label.as_deref(), Some("disable"));
    }

    #[test]
    fn state_stereotypes() {
        let d = parse(
            "state s1 <<start>>\nstate s2 <<end>>\nstate s3 <<choice>>\n\
             state s4 <<fork>>\nstate s5 <<join>>",
        );
        assert_eq!(d.states[0].kind, StateKind::Initial);
        assert_eq!(d.states[1].kind, StateKind::Final);
        assert_eq!(d.states[2].kind, StateKind::Choice);
        assert_eq!(d.states[3].kind, StateKind::Fork);
        assert_eq!(d.states[4].kind, StateKind::Join);
    }

    #[test]
    fn state_descriptions() {
        let d = parse("state Active\nActive : entry / initialize\nActive : do / process");
        assert_eq!(d.states[0].descriptions.len(), 2);
        assert_eq!(d.states[0].descriptions[0], "entry / initialize");
    }

    #[test]
    fn state_alias() {
        let d = parse("state \"Running State\" as Running\n[*] --> Running");
        assert_eq!(d.states[0].id, "Running");
        assert_eq!(d.states[0].label, "Running State");
    }

    #[test]
    fn hide_empty() {
        let d = parse("hide empty description\nstate A\nstate B\nA --> B");
        assert_eq!(d.states.len(), 2);
        assert_eq!(d.transitions.len(), 1);
    }

    #[test]
    fn note_right_of_state() {
        let d = parse("[*] --> A\nnote right of A : Note 1\nA --> [*]");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "Note 1");
        assert!(matches!(&d.notes[0].kind, StateNoteKind::RightOf(id) if id == "A"));
    }

    #[test]
    fn note_left_of_state() {
        let d = parse("[*] --> A\nnote left of A : Left note\nA --> [*]");
        assert_eq!(d.notes.len(), 1);
        assert!(matches!(&d.notes[0].kind, StateNoteKind::LeftOf(id) if id == "A"));
    }

    #[test]
    fn note_multiline() {
        let d = parse("[*] --> A\nnote right of A\n  line 1\n  line 2\nend note\nA --> [*]");
        assert_eq!(d.notes.len(), 1);
        assert!(d.notes[0].text.contains("line 1"));
        assert!(d.notes[0].text.contains("line 2"));
    }

    #[test]
    fn floating_note() {
        let d = parse("note \"Floating note 1\" as FN1\n[*] --> A\nA --> [*]");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "Floating note 1");
        assert!(matches!(&d.notes[0].kind, StateNoteKind::Floating));
    }

    #[test]
    fn note_on_link() {
        let d = parse("[*] --> A\nA --> [*]\nnote on link\n  link note\nend note");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "link note");
        assert!(matches!(&d.notes[0].kind, StateNoteKind::OnLink));
    }
}
