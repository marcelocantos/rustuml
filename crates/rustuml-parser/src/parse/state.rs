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
            continue;
        }
        parser.parse_line(i + 1, trimmed)?;
    }
    Ok(parser.finish())
}

struct StateParser {
    meta: DiagramMeta,
    states: Vec<State>,
    transitions: Vec<Transition>,
}

impl StateParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            states: Vec::new(),
            transitions: Vec::new(),
        }
    }

    fn finish(self) -> StateDiagram {
        StateDiagram {
            meta: self.meta,
            states: self.states,
            transitions: self.transitions,
        }
    }

    fn ensure_state(&mut self, id: &str) -> String {
        let id = id.trim().to_string();
        if id == "[*]" {
            return id;
        }
        if !self.states.iter().any(|s| s.id == id) {
            self.states.push(State {
                id: id.clone(),
                label: id.clone(),
                kind: StateKind::Normal,
                descriptions: Vec::new(),
                substates: Vec::new(),
            });
        }
        id
    }

    fn parse_line(&mut self, _line_num: usize, line: &str) -> Result<(), ParseError> {
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
            Regex::new(
                r"^(\[?\*?\]?|[\w.]+)\s*-+(?:left|right|up|down|le|ri|do)?-*>\s*(\[?\*?\]?|[\w.]+)(?:\s*:\s*(.+))?$",
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let from = self.ensure_state(&caps[1]);
            let to = self.ensure_state(&caps[2]);
            let label = caps.get(3).map(|m| m.as_str().trim().to_string());
            self.transitions.push(Transition { from, to, label });
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
            } else {
                self.states.push(State {
                    id: id.clone(),
                    label,
                    kind,
                    descriptions: Vec::new(),
                    substates: Vec::new(),
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
        line.starts_with("note ")
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
}
