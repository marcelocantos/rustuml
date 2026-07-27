// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram parser.
//!
//! Parses preprocessed lines into a `SequenceDiagram` model.

use std::sync::LazyLock;

use regex::{Match, Regex};

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::sequence::*;

static INLINE_ARROW_STYLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\[((?:#\w+|(?i:dashed|dotted|hidden|bold))(?:,(?:#\w+|(?i:dashed|dotted|hidden|bold)))*)\]",
    )
    .unwrap()
});

/// Parse preprocessed lines into a sequence diagram.
pub fn parse_sequence(lines: &[String]) -> Result<SequenceDiagram, ParseError> {
    let mut parser = SeqParser::new();

    for (i, line) in lines.iter().enumerate() {
        let (source_line, trimmed) = super::source_line_and_trimmed(i + 1, line);
        let in_note = parser.note_buffer.is_some();
        if trimmed.is_empty() && !in_note {
            continue;
        }
        let text = if in_note {
            super::source_text(line)
        } else {
            trimmed
        };
        parser.parse_line(source_line, text)?;
    }

    Ok(parser.finish())
}

struct SeqParser {
    meta: DiagramMeta,
    participants: Vec<Participant>,
    events: Vec<Event>,
    autonumber: Option<AutoNumber>,
    participant_ids: Vec<String>,
    /// Multiline note accumulator.
    note_buffer: Option<NoteBuffer>,
    /// Multiline ref accumulator.
    ref_buffer: Option<RefBuffer>,
    /// Last message source and target (for bare "note left"/"note right").
    last_message: Option<(String, String)>,
    /// Whether we are inside a `legend ... endlegend` block.
    in_legend: bool,
    /// Whether `hide footbox` was specified.
    hide_footbox: bool,
    /// Whether `!pragma teoz true` was specified.
    teoz: bool,
    /// Whether `autoactivate on` is active for subsequent messages.
    autoactivate: bool,
    /// Current 1-based source line number (set before each parse_line call).
    current_line: usize,
    /// Named participant boxes (`box ... end box`).
    boxes: Vec<ParticipantBox>,
    /// Index into `boxes` of the box currently being collected, if any.
    current_box: Option<usize>,
    /// Prefix of an open `skinparam X { ... }` block.
    skinparam_block_prefix: Option<String>,
}

struct NoteBuffer {
    position: NotePosition,
    participants: Vec<String>,
    lines: Vec<String>,
    shape: NoteShape,
    color: Option<String>,
    on_message: bool,
    source_line: usize,
}

struct RefBuffer {
    participants: Vec<String>,
    lines: Vec<String>,
    source_line: usize,
}

impl SeqParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            participants: Vec::new(),
            events: Vec::new(),
            autonumber: None,
            participant_ids: Vec::new(),
            note_buffer: None,
            ref_buffer: None,
            last_message: None,
            in_legend: false,
            hide_footbox: false,
            teoz: false,
            autoactivate: false,
            current_line: 0,
            boxes: Vec::new(),
            current_box: None,
            skinparam_block_prefix: None,
        }
    }

    fn finish(self) -> SequenceDiagram {
        SequenceDiagram {
            meta: self.meta,
            participants: self.participants,
            events: self.events,
            autonumber: self.autonumber,
            hide_footbox: self.hide_footbox,
            teoz: self.teoz,
            boxes: self.boxes,
        }
    }

    fn ensure_participant(&mut self, id: &str) -> String {
        self.ensure_participant_at(id, self.current_line)
    }

    fn ensure_participant_at(&mut self, id: &str, source_line: usize) -> String {
        let id = id.trim().to_string();
        if !self.participant_ids.contains(&id) {
            self.participant_ids.push(id.clone());
            self.participants.push(Participant {
                id: id.clone(),
                label: id.clone(),
                kind: ParticipantKind::default(),
                order: Some(self.participants.len()),
                stereotype: None,
                url: None,
                color: None,
                source_line,
            });
        }
        id
    }

    fn parse_line(&mut self, line_num: usize, line: &str) -> Result<(), ParseError> {
        self.current_line = line_num;

        if let Some(prefix) = self.skinparam_block_prefix.clone() {
            if line == "}" {
                self.skinparam_block_prefix = None;
            } else if let Some((key, value)) = line.split_once(char::is_whitespace) {
                let key = key.trim();
                let value = value.trim();
                if !key.is_empty() && !value.is_empty() {
                    self.meta.skinparams.push(crate::diagram::SkinParam {
                        key: format!("{prefix}{key}"),
                        value: value.to_string(),
                    });
                }
            }
            return Ok(());
        }

        // Handle multiline ref buffering.
        if self.ref_buffer.is_some() {
            if line == "end ref" {
                let buf = self.ref_buffer.take().unwrap();
                let text = buf.lines.join("\n");
                self.events.push(Event::Ref(Ref {
                    participants: buf.participants,
                    text,
                    source_line: buf.source_line,
                }));
            } else if let Some(buf) = &mut self.ref_buffer {
                buf.lines.push(line.trim().to_string());
            }
            return Ok(());
        }

        // Handle multiline note buffering.
        if self.note_buffer.is_some() {
            let trimmed = line.trim();
            if trimmed == "endnote" || trimmed == "end note" {
                let buf = self.note_buffer.take().unwrap();
                let text = note_text_from_lines(&buf.lines);
                self.events.push(Event::Note(Note {
                    position: buf.position,
                    participants: buf.participants,
                    text,
                    shape: buf.shape,
                    color: buf.color,
                    on_message: buf.on_message,
                    source_line: buf.source_line,
                }));
            } else if let Some(buf) = &mut self.note_buffer {
                buf.lines.push(line.to_string());
            }
            return Ok(());
        }

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
        if self.try_autoactivate(line) {
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
        // try_box must come before try_message: "box" would otherwise be parsed
        // as a message b -[o]-> x because 'o' and 'x' are valid arrow chars.
        if self.try_box(line) {
            return Ok(());
        }
        // try_meta before try_message: `title <back:cyan>...</back>` would
        // otherwise be parsed as a "title <- back" message because the
        // arrow regex accepts `<` and `>` as arrow characters. Title /
        // header / footer / caption / legend all start with explicit
        // keywords, so checking them first cannot conflict with anything
        // a message line is allowed to look like.
        if self.try_meta(line) {
            return Ok(());
        }
        if self.try_pragma(line) {
            return Ok(());
        }
        if self.try_message(line) {
            return Ok(());
        }
        if self.try_ref(line) {
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
        Ok(())
    }

    fn try_participant_decl(&mut self, line: &str) -> bool {
        let (url, clean_line) = super::extract_link_url(line);
        let line = clean_line.as_str();
        // Matches these forms (label = display text, id = entity key):
        //   1. keyword "Long Label" as alias  <<stereotype>>
        //   2. keyword alias as "Long Label"  <<stereotype>>
        //   3. keyword Label as alias         <<stereotype>>  (both unquoted)
        //   4. keyword SimpleName             <<stereotype>>
        //   5. keyword "Long Label"           <<stereotype>>  (no alias; id = label)
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(participant|actor|boundary|control|entity|database|collections|queue)\s+(?:"([^"]+)"\s+as\s+(\w+)|(\w+)\s+as\s+"([^"]+)"|(\w+)\s+as\s+(\w+)|"([^"]+)"|(\w+))(?:\s+<<([^>]+)>>)?(?:\s+(#\S+))?(?:\s+order\s+(\d+))?"#,
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let kind = parse_participant_kind(&caps[1]);
            // `quoted` tracks whether the display text came from a `"..."` form.
            // PlantUML's grammar matches the stereotype (`STEREO`) only OUTSIDE the
            // quoted display string (`FULL` = `[%g]([^%g]+)[%g]`), so a `<<...>>`
            // that appears WITHIN a quoted label is literal display text, not a
            // stereotype (see CommandParticipantA). We therefore only mine an
            // inline stereotype from UNQUOTED labels.
            let (raw_label, id, quoted) = if let Some(quoted) = caps.get(2) {
                // Form 1: "Long Label" as alias
                (quoted.as_str().to_string(), caps[3].to_string(), true)
            } else if let Some(alias) = caps.get(4) {
                // Form 2: alias as "Long Label"
                let lbl = caps.get(5).map_or("", |m| m.as_str()).to_string();
                (lbl, alias.as_str().to_string(), true)
            } else if let Some(label) = caps.get(6) {
                // Form 3: Label as alias (both unquoted) — id is the alias.
                (label.as_str().to_string(), caps[7].to_string(), false)
            } else if let Some(quoted) = caps.get(8) {
                // Form 5: "Long Label" (no alias; id = label)
                let lbl = quoted.as_str().to_string();
                (lbl.clone(), lbl, true)
            } else {
                // Form 4: SimpleName
                let name = caps[9].to_string();
                (name.clone(), name, false)
            };
            // Extract `<<stereotype>>` from within the label text (e.g.
            // `Service 1 <<internal>>`) only for unquoted labels — a quoted
            // display string keeps `<<...>>` as literal text.
            let (label, label_stereotype) = if quoted {
                (raw_label, None)
            } else {
                extract_stereotype_from_label(&raw_label)
            };
            let stereotype = caps
                .get(10)
                .map(|m| m.as_str().to_string())
                .or(label_stereotype);

            let color = caps.get(11).map(|m| m.as_str().to_string());

            // Explicit `order N` sets the layout sort key; otherwise it defaults
            // to the declaration index. Implicit participants (created by a
            // message) also default to their declaration index, so a participant
            // declared with a large `order N` can be positioned to the right of a
            // later, implicitly-created one (matching Java PlantUML).
            let explicit_order = caps.get(12).and_then(|m| m.as_str().parse::<usize>().ok());

            if !self.participant_ids.contains(&id) {
                self.participant_ids.push(id.clone());
                let idx = self.participants.len();
                self.participants.push(Participant {
                    id: id.clone(),
                    label,
                    kind,
                    order: Some(explicit_order.unwrap_or(idx)),
                    stereotype,
                    url,
                    color,
                    source_line: self.current_line,
                });
                if let Some(bi) = self.current_box {
                    self.boxes[bi].members.push(idx);
                }
            }
            true
        } else {
            false
        }
    }

    fn try_message(&mut self, line: &str) -> bool {
        // Allow optional #color after activation modifier (++ #blue, -- #red, etc.)
        // Supports both simple names (\w+) and quoted names ("...").
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^("(?:[^"]+)"|\w+)\s*([-<>.\\/ox]+)\s*("(?:[^"]+)"|\w+)\s*(?:((?:\+\+|--|!!))\s*(#\S+)?\s*)?(?::\s*(.*))?$"#,
            )
            .unwrap()
        });

        // Extract arrow colour only from the arrow header. Message labels can
        // contain local Creole links like `[[#anchor label]]`, whose inner
        // `[#anchor label]` must not be mistaken for an arrow colour.
        let (arrow_style, stripped) = strip_arrow_style_annotation(line);
        let line = stripped.as_str();

        if let Some(caps) = RE.captures(line) {
            // Strip surrounding quotes from quoted participant names.
            let unquote = |s: &str| -> String {
                if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                    s[1..s.len() - 1].to_string()
                } else {
                    s.to_string()
                }
            };
            let from_raw = unquote(&caps[1]);
            let arrow_str = &caps[2];
            let to_raw = unquote(&caps[3]);
            let activation_str = caps.get(4).map(|m| m.as_str());
            let activation_color = caps.get(5).map(|m| m.as_str().to_string());
            let label = message_label(line, caps.get(6));

            let mut arrow = parse_arrow(arrow_str);
            apply_inline_arrow_style(&mut arrow, arrow_style);
            let activation = activation_str
                .map(parse_activation)
                .or_else(|| self.autoactivation_for(&arrow));

            // Ensure participants in textual order (left-to-right as written)
            // so the participant list preserves declaration order.
            let left_id = self.ensure_participant(&from_raw);
            let right_id = self.ensure_participant(&to_raw);

            // For left-pointing arrows (Alice <- Bob), swap from/to so that
            // from=sender(Bob), to=receiver(Alice), matching PlantUML semantics.
            let (from_id, to_id) = if arrow.direction == ArrowDirection::RightToLeft {
                (right_id, left_id)
            } else {
                (left_id, right_id)
            };

            self.last_message = Some((from_id.clone(), to_id.clone()));
            self.events.push(Event::Message(Message {
                from: from_id,
                to: to_id,
                label,
                arrow,
                activation,
                activation_color,
                source_line: self.current_line,
            }));
            true
        } else {
            false
        }
    }

    fn try_external_message(&mut self, line: &str) -> bool {
        static RE_IN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"^\[([-<>.\\/ox]+)\s*(\w+)\s*(?:((?:\+\+|--|!!))\s*(#\S+)?\s*)?(?::\s*(.*))?$",
            )
            .unwrap()
        });
        static RE_OUT: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(\w+)\s*([-<>.\\/ox]+)([\[\]])\s*(?:((?:\+\+|--|!!))\s*(#\S+)?\s*)?(?::\s*(.*))?$")
                .unwrap()
        });
        let (arrow_style, stripped) = strip_arrow_style_annotation(line);
        let line = stripped.as_str();

        if let Some(caps) = RE_IN.captures(line) {
            let mut arrow = parse_arrow(&caps[1]);
            arrow.direction = ArrowDirection::LeftToRight;
            apply_inline_arrow_style(&mut arrow, arrow_style);
            let to = self.ensure_participant(&caps[2]);
            let activation = caps.get(3).map(|m| parse_activation(m.as_str()));
            let activation_color = caps.get(4).map(|m| m.as_str().to_string());
            let label = message_label(line, caps.get(5));
            self.events.push(Event::Message(Message {
                from: "[".to_string(),
                to,
                label,
                arrow,
                activation,
                activation_color,
                source_line: self.current_line,
            }));
            true
        } else if let Some(caps) = RE_OUT.captures(line) {
            let from = self.ensure_participant(&caps[1]);
            let mut arrow = parse_arrow(&caps[2]);
            arrow.direction = ArrowDirection::LeftToRight;
            apply_inline_arrow_style(&mut arrow, arrow_style);
            let activation = caps.get(4).map(|m| parse_activation(m.as_str()));
            let activation_color = caps.get(5).map(|m| m.as_str().to_string());
            let label = message_label(line, caps.get(6));
            self.events.push(Event::Message(Message {
                from,
                to: caps[3].to_string(),
                label,
                arrow,
                activation,
                activation_color,
                source_line: self.current_line,
            }));
            true
        } else {
            false
        }
    }

    fn try_note(&mut self, line: &str) -> bool {
        // `note : text` — Java PlantUML creates a participant named "note" with
        // a note box above/beside it showing the text.  We declare the "note"
        // participant and emit a Note event.
        if let Some(text) = line.strip_prefix("note :").map(|s| s.trim()) {
            if !text.is_empty() {
                let participant_id = self.ensure_participant("note");
                self.events.push(Event::Note(Note {
                    position: NotePosition::Over,
                    participants: vec![participant_id],
                    text: text.to_string(),
                    shape: NoteShape::Note,
                    color: None,
                    on_message: false,
                    source_line: self.current_line,
                }));
            }
            return true;
        }

        // Note on link (attaches to previous message).
        if let Some(rest) = line.strip_prefix("note on link") {
            let text = rest.trim().trim_start_matches(':').trim().to_string();
            self.events.push(Event::NoteOnLink(text));
            return true;
        }

        // Also handle "note across" (with optional color)
        if let Some(rest) = line.strip_prefix("note across") {
            // Parse optional color: "note across #color : text" or "note across #color"
            let rest = rest.trim();
            let (color, rest) = if rest.starts_with('#') {
                let end = rest
                    .find(|c: char| c.is_whitespace() || c == ':')
                    .unwrap_or(rest.len());
                (Some(rest[..end].to_string()), rest[end..].trim())
            } else {
                (None, rest)
            };
            let text = rest.trim_start_matches(':').trim().to_string();
            if text.is_empty() {
                self.note_buffer = Some(NoteBuffer {
                    position: NotePosition::Over,
                    participants: Vec::new(),
                    lines: Vec::new(),
                    shape: NoteShape::Note,
                    color,
                    on_message: false,
                    source_line: self.current_line,
                });
            } else {
                self.events.push(Event::Note(Note {
                    position: NotePosition::Over,
                    participants: Vec::new(),
                    text,
                    shape: NoteShape::Note,
                    color,
                    on_message: false,
                    source_line: self.current_line,
                }));
            }
            return true;
        }

        // Allow optional color (#xxx) after participant list.
        // Capture shape prefix (h/r/note), position, participants, color, inline text.
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^(h|r)?note\s+(left|right|over)\s*(?:of\s+)?(\w+(?:\s*,\s*\w+)*)?\s*(#\S+)?\s*(?::\s*(.*))?$").unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let shape = match caps.get(1).map(|m| m.as_str()) {
                Some("h") => NoteShape::Hexagonal,
                Some("r") => NoteShape::Rectangular,
                _ => NoteShape::Note,
            };
            let position = match &caps[2] {
                "left" => NotePosition::Left,
                "right" => NotePosition::Right,
                "over" => NotePosition::Over,
                _ => NotePosition::Right,
            };
            let inline_text = caps.get(5);
            let participant_source_line = if inline_text.is_some() {
                self.current_line
            } else {
                self.current_line + 1
            };
            let mut participants: Vec<String> = caps.get(3).map_or(Vec::new(), |m| {
                m.as_str()
                    .split(',')
                    .map(|s| self.ensure_participant_at(s.trim(), participant_source_line))
                    .collect()
            });
            // Bare "note left" / "note right" (no participant) attaches to the
            // last message and straddles its arrow band (on_message). PlantUML
            // anchors such notes to the message's leftmost/rightmost endpoint by
            // screen position, so store BOTH endpoints and let the renderer pick.
            // "note left"  → left of the leftmost endpoint
            // "note right" → right of the rightmost endpoint
            let mut on_message = false;
            if participants.is_empty()
                && position != NotePosition::Over
                && let Some((from, to)) = &self.last_message
            {
                participants = vec![from.clone(), to.clone()];
                on_message = true;
            }
            let color = caps.get(4).map(|m| m.as_str().to_string());

            if let Some(text_match) = inline_text {
                if participants.is_empty() && position != NotePosition::Over && !on_message {
                    return true;
                }
                // Inline note: note right : text
                let text = text_match.as_str().trim().to_string();
                self.events.push(Event::Note(Note {
                    position,
                    participants,
                    text,
                    shape,
                    color,
                    on_message,
                    source_line: self.current_line,
                }));
            } else {
                // Multiline note: note right\n...\nendnote
                self.note_buffer = Some(NoteBuffer {
                    position,
                    participants,
                    lines: Vec::new(),
                    shape,
                    color,
                    on_message,
                    source_line: self.current_line,
                });
            }
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
            self.events.push(Event::GroupStart(GroupStart {
                kind,
                label,
                source_line: self.current_line,
            }));
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
            self.events.push(Event::GroupElse(GroupElse {
                label,
                source_line: self.current_line,
            }));
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
        static RE_START: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^autonumber(?:\s+(\d+))?(?:\s+(\d+))?(?:\s+"([^"]*)")?$"#).unwrap()
        });
        static RE_RESUME: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^autonumber\s+resume(?:\s+(\d+))?(?:\s+"([^"]*)")?$"#).unwrap()
        });

        if line == "autonumber stop" {
            self.push_autonumber(AutonumberCmd::Stop);
            return true;
        }
        if line.starts_with("autonumber resume")
            && let Some(caps) = RE_RESUME.captures(line)
        {
            let step = caps.get(1).and_then(|m| m.as_str().parse().ok());
            let format = caps.get(2).map(|m| m.as_str().to_string());
            self.push_autonumber(AutonumberCmd::Resume { step, format });
            return true;
        }
        if let Some(caps) = RE_START.captures(line) {
            let start = caps.get(1).map_or(1, |m| m.as_str().parse().unwrap_or(1));
            let step = caps.get(2).map_or(1, |m| m.as_str().parse().unwrap_or(1));
            let format = caps.get(3).map(|m| m.as_str().to_string());
            self.push_autonumber(AutonumberCmd::Start {
                start,
                step,
                format,
            });
            return true;
        }
        false
    }

    /// Record an autonumber directive: keep the first `Start` in the legacy
    /// `self.autonumber` field (so initial layout still sees it) and always emit
    /// an event so mid-stream changes (stop/resume/restart) take effect.
    fn push_autonumber(&mut self, cmd: AutonumberCmd) {
        if self.autonumber.is_none()
            && let AutonumberCmd::Start {
                start,
                step,
                format,
            } = &cmd
        {
            self.autonumber = Some(AutoNumber {
                start: *start,
                step: *step,
                format: format.clone(),
            });
        }
        self.events.push(Event::Autonumber(cmd));
    }

    fn try_activate_deactivate(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(activate|deactivate)\s+(\w+)(?:\s+(#\S+))?").unwrap());

        if let Some(caps) = RE.captures(line) {
            let id = self.ensure_participant(&caps[2]);
            let color = caps.get(3).map(|m| m.as_str().to_string());
            match &caps[1] {
                "activate" => self.events.push(Event::Activate(id, color)),
                "deactivate" => self.events.push(Event::Deactivate(id)),
                _ => {}
            }
            true
        } else {
            false
        }
    }

    fn try_autoactivate(&mut self, line: &str) -> bool {
        let Some(rest) = line.strip_prefix("autoactivate") else {
            return false;
        };
        match rest.trim() {
            state if state.eq_ignore_ascii_case("on") => {
                self.autoactivate = true;
                true
            }
            state if state.eq_ignore_ascii_case("off") => {
                self.autoactivate = false;
                true
            }
            _ => false,
        }
    }

    fn autoactivation_for(&self, arrow: &Arrow) -> Option<ActivationChange> {
        if !self.autoactivate || arrow.head_half.is_some() || arrow.source_cross {
            return None;
        }
        match (arrow.line, arrow.head) {
            (LineStyle::Dotted, ArrowHead::Filled | ArrowHead::Open) => {
                Some(ActivationChange::Deactivate)
            }
            (LineStyle::Solid, ArrowHead::Filled | ArrowHead::Open) => {
                Some(ActivationChange::Activate)
            }
            _ => None,
        }
    }

    fn try_create_destroy(&mut self, line: &str) -> bool {
        static KW: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(create|destroy)\s+(.*)$").unwrap());
        // Identifier extraction from a participant declaration tail. Mirrors the
        // forms in `try_participant_decl`; the id is the alias when `as` is
        // present, otherwise the simple name (or quoted label).
        static REST: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r#"^(?:participant|actor|boundary|control|entity|database|collections|queue)?\s*(?:"[^"]+"\s+as\s+(\w+)|(\w+)\s+as\s+(?:"[^"]+"|\w+)|"([^"]+)"|(\w+))"#,
            )
            .unwrap()
        });

        let Some(kw) = KW.captures(line) else {
            return false;
        };
        let rest = kw[2].trim();
        let Some(caps) = REST.captures(rest) else {
            return false;
        };
        let id = caps
            .get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .or_else(|| caps.get(4))
            .map(|m| m.as_str().to_string());
        let Some(id) = id else {
            return false;
        };

        match &kw[1] {
            "create" => {
                // Register with the declared kind/alias/label if a full declaration
                // was given; otherwise ensure a plain participant exists.
                if !self.try_participant_decl(rest) {
                    self.ensure_participant(&id);
                }
                self.events.push(Event::Create(id));
            }
            "destroy" => self.events.push(Event::Destroy(id)),
            _ => {}
        }
        true
    }

    fn try_return(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("return") {
            let label = rest.trim().to_string();
            self.events.push(Event::Return(ReturnMessage {
                label,
                source_line: self.current_line,
            }));
            true
        } else {
            false
        }
    }

    fn try_ref(&mut self, line: &str) -> bool {
        // Inline: ref over A, B : text
        static RE_INLINE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^ref\s+over\s+(\w+(?:\s*,\s*\w+)*)\s*:\s*(.+)$").unwrap()
        });
        // Multiline start: ref over A, B  (no colon)
        static RE_START: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^ref\s+over\s+(\w+(?:\s*,\s*\w+)*)\s*$").unwrap());

        if let Some(caps) = RE_INLINE.captures(line) {
            let participants: Vec<String> = caps[1]
                .split(',')
                .map(|s| self.ensure_participant(s.trim()))
                .collect();
            let text = caps[2].trim().to_string();
            self.events.push(Event::Ref(Ref {
                participants,
                text,
                source_line: self.current_line,
            }));
            return true;
        }
        if let Some(caps) = RE_START.captures(line) {
            let participants: Vec<String> = caps[1]
                .split(',')
                .map(|s| self.ensure_participant(s.trim()))
                .collect();
            self.ref_buffer = Some(RefBuffer {
                participants,
                lines: Vec::new(),
                source_line: self.current_line,
            });
            return true;
        }
        false
    }

    fn try_meta(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("title ") {
            self.meta.title = Some(super::strip_title_quotes(rest).to_string());
            self.meta.title_line = Some(self.current_line);
            return true;
        }
        if let Some(rest) = line.strip_prefix("header ") {
            self.meta.header = Some(rest.trim().to_string());
            self.meta.header_line = Some(self.current_line);
            return true;
        }
        if let Some(rest) = line.strip_prefix("footer ") {
            self.meta.footer = Some(rest.trim().to_string());
            self.meta.footer_line = Some(self.current_line);
            return true;
        }
        if let Some(rest) = line.strip_prefix("caption ") {
            self.meta.caption = Some(rest.trim().to_string());
            self.meta.caption_line = Some(self.current_line);
            return true;
        }
        // Legend block: `legend` / `legend right` / `legend left` ... `endlegend`
        if line == "legend" || line.starts_with("legend ") {
            self.in_legend = true;
            self.meta.legend_line = Some(self.current_line);
            return true;
        }
        if self.in_legend {
            if line == "endlegend" || line == "end legend" {
                self.in_legend = false;
            } else {
                let l = self.meta.legend.get_or_insert_with(String::new);
                if !l.is_empty() {
                    l.push('\n');
                }
                l.push_str(line);
            }
            return true;
        }
        false
    }

    fn try_box(&mut self, line: &str) -> bool {
        // `box ["Title"] [#color] ... end box` groups consecutive participant
        // declarations into a titled, optionally coloured rectangle.
        if line == "end box" {
            self.current_box = None;
            return true;
        }
        if line == "box" || line.starts_with("box ") {
            let rest = line[3..].trim();
            // Title is an optional quoted string; colour is an optional #token.
            let (title, after_title) = if let Some(stripped) = rest.strip_prefix('"') {
                match stripped.find('"') {
                    Some(end) => (stripped[..end].to_string(), stripped[end + 1..].trim()),
                    None => (stripped.to_string(), ""),
                }
            } else {
                (String::new(), rest)
            };
            let color = after_title
                .split_whitespace()
                .find(|tok| tok.starts_with('#'))
                .map(|tok| tok.to_string());
            self.boxes.push(ParticipantBox {
                title,
                color,
                members: Vec::new(),
            });
            self.current_box = Some(self.boxes.len() - 1);
            return true;
        }
        false
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
        if let Some(rest) = line.strip_prefix("skinparam ") {
            let rest = rest.trim();
            if let Some(prefix) = rest.strip_suffix('{') {
                let prefix = prefix.trim();
                if !prefix.is_empty() {
                    self.skinparam_block_prefix = Some(prefix.to_string());
                }
                return true;
            }
            if let Some((key, value)) = rest.split_once(char::is_whitespace) {
                self.meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            true
        } else {
            false
        }
    }

    fn try_pragma(&mut self, line: &str) -> bool {
        let mut parts = line.split_whitespace();
        if matches!(parts.next(), Some("!pragma"))
            && matches!(parts.next(), Some("teoz"))
            && matches!(parts.next(), Some("true"))
            && parts.next().is_none()
        {
            self.teoz = true;
            return true;
        }
        false
    }

    fn try_hide(&mut self, line: &str) -> bool {
        if line == "hide footbox" {
            self.hide_footbox = true;
            return true;
        }
        line.starts_with("hide ")
    }
}

/// Extract a `<<stereotype>>` marker from a label string.
///
/// Returns `(cleaned_label, Some(stereotype))` if found, or `(label, None)`.
fn extract_stereotype_from_label(label: &str) -> (String, Option<String>) {
    if let Some(start) = label.find("<<")
        && let Some(rel_end) = label[start..].find(">>")
    {
        let end = start + rel_end;
        let stereotype = label[start + 2..end].trim().to_string();
        let cleaned = format!("{} {}", label[..start].trim(), label[end + 2..].trim())
            .trim()
            .to_string();
        return (cleaned, Some(stereotype));
    }
    (label.to_string(), None)
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

    let source_cross = s.starts_with('x');
    let head = if source_cross {
        ArrowHead::Filled
    } else if s.contains('x') {
        ArrowHead::Cross
    } else if s.contains('o') {
        ArrowHead::Circle
    } else if s.contains(">>") || s.contains("<<") {
        ArrowHead::Open
    } else {
        ArrowHead::Filled
    };

    // Half-arrowhead modifiers: `/` draws only the bottom wing, `\` only the
    // top wing. Doubling the modifier (`//`, `\\`) renders a thin open stroke
    // instead of a filled triangle.
    let (head_half, thin_head) = if s.contains("//") {
        (Some(ArrowHalf::Bottom), true)
    } else if s.contains("\\\\") {
        (Some(ArrowHalf::Top), true)
    } else if s.contains('/') {
        (Some(ArrowHalf::Bottom), false)
    } else if s.contains('\\') {
        (Some(ArrowHalf::Top), false)
    } else {
        (None, false)
    };

    let direction = if s.starts_with('<') && s.ends_with('>') {
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
        color: None,
        head_half,
        thin_head,
        source_cross,
    }
}

#[derive(Default)]
struct InlineArrowStyle {
    color: Option<String>,
    line: Option<LineStyle>,
}

fn strip_arrow_style_annotation(line: &str) -> (InlineArrowStyle, String) {
    let delimiter = sequence_message_delimiter(line);
    let (head, tail) = delimiter.map_or((line, None), |index| {
        (&line[..index], Some(&line[index + 1..]))
    });
    let mut style = InlineArrowStyle::default();
    if let Some(tokens) = INLINE_ARROW_STYLE
        .captures(head)
        .and_then(|captures| captures.get(1))
        .map(|matched| matched.as_str())
    {
        for token in tokens.split(',') {
            if token.eq_ignore_ascii_case("dashed") || token.eq_ignore_ascii_case("dotted") {
                style.line = Some(LineStyle::Dotted);
            } else if token.eq_ignore_ascii_case("hidden") {
                style.line = Some(LineStyle::Hidden);
            } else if !token.eq_ignore_ascii_case("bold") {
                style.color = Some(token.to_string());
            }
        }
    }

    let stripped_head = INLINE_ARROW_STYLE.replace(head, "");
    let stripped = match tail {
        Some(tail) => format!("{stripped_head}:{tail}"),
        None => stripped_head.into_owned(),
    };
    (style, stripped)
}

pub(super) fn has_inline_arrow_style(line: &str) -> bool {
    let header_end = sequence_message_delimiter(line).unwrap_or(line.len());
    INLINE_ARROW_STYLE.is_match(&line[..header_end])
}

fn sequence_message_delimiter(line: &str) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            quoted = !quoted;
        } else if ch == ':' && !quoted {
            return Some(index);
        }
    }
    None
}

fn apply_inline_arrow_style(arrow: &mut Arrow, style: InlineArrowStyle) {
    if let Some(line) = style.line {
        arrow.line = line;
    }
    if let Some(color) = style.color {
        arrow.color = Some(color);
    }
}

fn message_label(line: &str, matched: Option<Match<'_>>) -> String {
    let Some(matched) = matched else {
        return String::new();
    };
    let trimmed = matched.as_str().trim();
    if trimmed.starts_with("\\n")
        && let Some(colon) = line[..matched.start()].rfind(':')
    {
        let consumed = &line[colon + 1..matched.start()];
        if !consumed.is_empty() && consumed.chars().all(char::is_whitespace) {
            return format!("{consumed}{trimmed}");
        }
    }
    trimmed.to_string()
}

fn parse_activation(s: &str) -> ActivationChange {
    match s {
        "++" => ActivationChange::Activate,
        "--" => ActivationChange::Deactivate,
        "!!" => ActivationChange::Destroy,
        _ => ActivationChange::Activate,
    }
}

fn note_text_from_lines(lines: &[String]) -> String {
    let common_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.bytes()
                .take_while(|&b| b == b' ' || b == b'\t')
                .count()
        })
        .min()
        .unwrap_or(0);

    lines
        .iter()
        .map(|line| line.get(common_indent..).unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join("\n")
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
    fn message_label_preserves_space_before_escaped_newline() {
        let d = parse(r"Alice -> Bob: \n \t \\");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.label, r" \n \t \\");
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn local_link_message_label_is_not_arrow_color() {
        let d = parse("Alice -> Bob : [[#anchor local link]]");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.label, "[[#anchor local link]]");
            assert_eq!(m.arrow.color, None);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn quoted_participant_colon_does_not_expose_label_brackets() {
        let d = parse(r#""Cache: primary" -[#AD1457,dashed]> B : [[#anchor label]]"#);
        let Event::Message(message) = &d.events[0] else {
            panic!("expected message");
        };
        assert_eq!(message.from, "Cache: primary");
        assert_eq!(message.label, "[[#anchor label]]");
        assert_eq!(message.arrow.color.as_deref(), Some("#AD1457"));
        assert_eq!(message.arrow.line, LineStyle::Dotted);
    }

    #[test]
    fn inline_arrow_style_keeps_color_and_body_orthogonal() {
        let d = parse(
            "A -[#C2185B,dashed]> B : color first\n\
             B -[dotted,#1565C0]> A : style first\n\
             A -[#red,#blue]> B : final color\n\
             A -[bold]> B : accepted no-op\n\
             A -[hidden]> B : hidden",
        );
        for (index, color) in [(0, "#C2185B"), (1, "#1565C0"), (2, "#blue")] {
            let Event::Message(message) = &d.events[index] else {
                panic!("expected message");
            };
            assert_eq!(message.arrow.color.as_deref(), Some(color));
        }
        let Event::Message(first) = &d.events[0] else {
            panic!("expected message");
        };
        let Event::Message(second) = &d.events[1] else {
            panic!("expected message");
        };
        let Event::Message(bold) = &d.events[3] else {
            panic!("expected message");
        };
        let Event::Message(hidden) = &d.events[4] else {
            panic!("expected message");
        };
        assert_eq!(first.arrow.line, LineStyle::Dotted);
        assert_eq!(second.arrow.line, LineStyle::Dotted);
        assert_eq!(bold.arrow.line, LineStyle::Solid);
        assert_eq!(hidden.arrow.line, LineStyle::Hidden);
    }

    #[test]
    fn external_messages_share_inline_arrow_style_parser() {
        let d = parse(
            "[-[#AD1457,dashed]> Alice : found\n\
             Alice -[dotted,#2E7D32]>] : lost",
        );
        for (index, color) in [(0, "#AD1457"), (1, "#2E7D32")] {
            let Event::Message(message) = &d.events[index] else {
                panic!("expected message");
            };
            assert_eq!(message.arrow.color.as_deref(), Some(color));
            assert_eq!(message.arrow.line, LineStyle::Dotted);
        }
    }

    #[test]
    fn teoz_pragma_sets_sequence_flag() {
        let d = parse("!pragma teoz true\nAlice -> Bob : hello");
        assert!(d.teoz);
        assert_eq!(d.events.len(), 1);
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
    fn bidirectional_arrows() {
        let d = parse("A <-> B : solid\nA <--> B : dotted\nA <<->> B : open");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "A");
            assert_eq!(m.to, "B");
            assert_eq!(m.arrow.direction, ArrowDirection::Bidirectional);
            assert_eq!(m.arrow.line, LineStyle::Solid);
            assert_eq!(m.arrow.head, ArrowHead::Filled);
        } else {
            panic!("expected message");
        }
        if let Event::Message(m) = &d.events[1] {
            assert_eq!(m.from, "A");
            assert_eq!(m.to, "B");
            assert_eq!(m.arrow.direction, ArrowDirection::Bidirectional);
            assert_eq!(m.arrow.line, LineStyle::Dotted);
            assert_eq!(m.arrow.head, ArrowHead::Filled);
        } else {
            panic!("expected message");
        }
        if let Event::Message(m) = &d.events[2] {
            assert_eq!(m.from, "A");
            assert_eq!(m.to, "B");
            assert_eq!(m.arrow.direction, ArrowDirection::Bidirectional);
            assert_eq!(m.arrow.line, LineStyle::Solid);
            assert_eq!(m.arrow.head, ArrowHead::Open);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn source_cross_arrow_keeps_target_head() {
        let d = parse("A x-> B : found\nA ->x B : lost");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "A");
            assert_eq!(m.to, "B");
            assert!(m.arrow.source_cross);
            assert_eq!(m.arrow.head, ArrowHead::Filled);
        } else {
            panic!("expected message");
        }
        if let Event::Message(m) = &d.events[1] {
            assert_eq!(m.from, "A");
            assert_eq!(m.to, "B");
            assert!(!m.arrow.source_cross);
            assert_eq!(m.arrow.head, ArrowHead::Cross);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn dotted_external_incoming_arrow() {
        let d = parse("[--> Alice : found dotted");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "[");
            assert_eq!(m.to, "Alice");
            assert_eq!(m.arrow.line, LineStyle::Dotted);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn dotted_external_outgoing_arrow() {
        let d = parse("Alice -->] : lost dotted");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "Alice");
            assert_eq!(m.to, "]");
            assert_eq!(m.arrow.line, LineStyle::Dotted);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn dotted_external_outgoing_left_arrow() {
        let d = parse("Alice -->[ : lost left dotted");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "Alice");
            assert_eq!(m.to, "[");
            assert_eq!(m.label, "lost left dotted");
            assert_eq!(m.arrow.line, LineStyle::Dotted);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn external_incoming_activation_and_color() {
        let d = parse("[-> Alice ++ #red : found");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "[");
            assert_eq!(m.to, "Alice");
            assert_eq!(m.label, "found");
            assert_eq!(m.activation, Some(ActivationChange::Activate));
            assert_eq!(m.activation_color.as_deref(), Some("#red"));
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn external_outgoing_deactivation() {
        let d = parse("Alice -->] -- : lost return");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "Alice");
            assert_eq!(m.to, "]");
            assert_eq!(m.label, "lost return");
            assert_eq!(m.activation, Some(ActivationChange::Deactivate));
            assert_eq!(m.activation_color, None);
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
    fn skinparam_blocks_are_flattened() {
        let d = parse(
            "skinparam note {\n  BackgroundColor LightYellow\n  BorderColor Orange\n  FontColor DarkBrown\n}\nA -> B : msg\nnote right : hello",
        );
        assert_eq!(d.meta.skinparams.len(), 3);
        assert_eq!(d.meta.skinparams[0].key, "noteBackgroundColor");
        assert_eq!(d.meta.skinparams[0].value, "LightYellow");
        assert_eq!(d.meta.skinparams[1].key, "noteBorderColor");
        assert_eq!(d.meta.skinparams[1].value, "Orange");
        assert_eq!(d.meta.skinparams[2].key, "noteFontColor");
        assert_eq!(d.meta.skinparams[2].value, "DarkBrown");
    }

    #[test]
    fn bare_side_note_before_message_is_ignored() {
        let d = parse("note left : orphan\nA -> B : msg");
        assert_eq!(d.events.len(), 1);
        assert!(matches!(d.events[0], Event::Message(_)));
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
    fn autoactivate_directive_applies_to_subsequent_messages() {
        let d = parse(
            "autoactivate on\nA -> B : call\nB --> A : done\nautoactivate off\nA -> B : later",
        );
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].id, "A");
        assert_eq!(d.participants[1].id, "B");
        assert_eq!(d.events.len(), 3);
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.activation, Some(ActivationChange::Activate));
        } else {
            panic!("expected message");
        }
        if let Event::Message(m) = &d.events[1] {
            assert_eq!(m.activation, Some(ActivationChange::Deactivate));
        } else {
            panic!("expected message");
        }
        if let Event::Message(m) = &d.events[2] {
            assert_eq!(m.activation, None);
        } else {
            panic!("expected message");
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

    #[test]
    fn note_on_link() {
        let d = parse("A -> B : msg\nnote on link : link note");
        assert_eq!(d.events.len(), 2, "events: {:?}", d.events);
        assert!(
            matches!(&d.events[1], Event::NoteOnLink(s) if s == "link note"),
            "event[1]: {:?}",
            d.events[1]
        );
    }

    #[test]
    fn multiline_note() {
        let d = parse("A -> B : msg\nnote left\n  Line 1\n  Line 2\nendnote");
        assert_eq!(d.events.len(), 2);
        if let Event::Note(n) = &d.events[1] {
            assert_eq!(n.position, NotePosition::Left);
            assert_eq!(n.text, "Line 1\nLine 2");
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn multiline_note_preserves_relative_code_indent() {
        let d = parse(
            "A -> B : msg\nnote over A\n  <code>\n  function foo() {\n    return 42;\n  }\n  </code>\nend note",
        );
        if let Event::Note(n) = &d.events[1] {
            assert_eq!(n.text, "<code>\nfunction foo() {\n  return 42;\n}\n</code>");
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn multiline_note_end_note() {
        let d = parse("A -> B : msg\nnote right of B\n  First\n  Second\nend note");
        if let Event::Note(n) = &d.events[1] {
            assert_eq!(n.position, NotePosition::Right);
            assert!(n.text.contains("First"));
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn multiline_note_implicit_participant_uses_first_body_line_source() {
        let d = parse("\n\n\nnote over Alice\n  First\nend note\nAlice -> Bob");
        let alice = d
            .participants
            .iter()
            .find(|p| p.id == "Alice")
            .expect("Alice participant");
        assert_eq!(alice.source_line, 5);
    }

    #[test]
    fn participant_with_url() {
        let d = parse(
            "actor User [[https://example.com/user]]\nparticipant API [[https://example.com/api]]\nUser -> API : hello",
        );
        assert_eq!(d.participants.len(), 2);
        assert_eq!(
            d.participants[0].url.as_deref(),
            Some("https://example.com/user")
        );
        assert_eq!(
            d.participants[1].url.as_deref(),
            Some("https://example.com/api")
        );
    }

    #[test]
    fn participant_no_url() {
        let d = parse("participant Alice\nparticipant Bob\nAlice -> Bob : hi");
        assert_eq!(d.participants[0].url, None);
        assert_eq!(d.participants[1].url, None);
    }
}
