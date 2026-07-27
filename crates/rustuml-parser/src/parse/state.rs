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
        let (source_line, trimmed) = super::source_line_and_trimmed(i + 1, line);
        if trimmed.is_empty() {
            // Empty lines may terminate a multi-line note with content already
            // accumulated — keep buffering (blank lines are part of note body).
            if let Some(buf) = &mut parser.note_buffer {
                buf.text.push('\n');
            }
            continue;
        }
        parser.parse_line(source_line, trimmed)?;
    }
    // Flush any unclosed note buffer.
    parser.flush_note();
    Ok(parser.finish())
}

/// A concurrent-region separator is a line made entirely of two or more `-`
/// (horizontal split) or two or more `|` (vertical split). PlantUML treats
/// `--`, `---`, `||`, etc. inside a composite as region dividers.
fn is_region_separator(line: &str) -> bool {
    (line.len() >= 2 && line.bytes().all(|b| b == b'-'))
        || (line.len() >= 2 && line.bytes().all(|b| b == b'|'))
}

/// Parse PlantUML's bracketed state-link style into typed transition metadata.
///
/// Java provenance: `CommandLinkStateCommon.executeArg` passes
/// `ARROW_STYLE` to `Link.applyStyle`; `WithLinkType.applyOneStyle` resolves
/// `#color`, dashed/dotted/bold, and `thickness=N` tokens onto the link.
fn parse_transition_style(arrow: &str) -> TransitionStyle {
    static STYLE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]*)\]").unwrap());

    let mut style = TransitionStyle::default();
    for captures in STYLE_RE.captures_iter(arrow) {
        for token in captures[1].split([',', ';']).map(str::trim) {
            if let Some(color) = token.strip_prefix('#') {
                if !color.is_empty() {
                    style.color = Some(color.to_string());
                }
                continue;
            }
            if let Some((key, value)) = token.split_once('=')
                && key.eq_ignore_ascii_case("thickness")
            {
                style.thickness = value.trim().parse().ok();
                continue;
            }
            style.line_style = match token.to_ascii_lowercase().as_str() {
                "dashed" => Some(TransitionLineStyle::Dashed),
                "dotted" => Some(TransitionLineStyle::Dotted),
                "bold" => Some(TransitionLineStyle::Bold),
                _ => style.line_style,
            };
        }
    }
    style
}

/// Parse command orientation separately from the effective queue direction.
///
/// Java's `CommandLinkStateReverse` exchanges ENT1/ENT2 and defaults to LEFT;
/// `CommandLinkStateCommon.executeArg` later applies `Link.getInv()` for
/// LEFT/UP. The two operations are intentionally not collapsed here.
fn parse_transition_arrow(arrow: &str) -> TransitionArrow {
    static STYLE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[[^\]]*\]").unwrap());

    let command_reversed = arrow.starts_with('<');
    let without_styles = STYLE_RE.replace_all(arrow, "");
    let token = without_styles
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_lowercase();
    let direction = match token.as_str() {
        "left" | "le" | "l" => Some(TransitionDirection::Left),
        "right" | "ri" | "r" => Some(TransitionDirection::Right),
        "up" | "u" => Some(TransitionDirection::Up),
        "down" | "do" | "d" => Some(TransitionDirection::Down),
        _ if command_reversed => Some(TransitionDirection::Left),
        _ => None,
    };
    TransitionArrow {
        direction,
        command_reversed,
    }
}

/// Accumulator for a multi-line note body.
struct NoteBuffer {
    kind: StateNoteKind,
    text: String,
    source_line: usize,
    command_line: usize,
    creation_order: usize,
}

struct StateParser {
    meta: DiagramMeta,
    states: Vec<State>,
    transitions: Vec<Transition>,
    notes: Vec<StateNote>,
    /// Active multi-line note being accumulated.
    note_buffer: Option<NoteBuffer>,
    /// Stable creation order assigned when a note command starts.
    next_note_creation_order: usize,
    /// Current 1-based source line number (set before each parse_line call).
    current_line: usize,
    /// Active prefix when inside a `skinparam <prefix> { ... }` block.
    skinparam_block_prefix: Option<String>,
    /// Stack of enclosing composite-state scopes. Empty at top level; each
    /// `state X { … }` pushes a frame and the matching `}` pops it. A `--` (or
    /// `||`) region separator inside a composite advances the top frame to a
    /// synthetic concurrent-region sub-scope.
    scope_stack: Vec<ScopeFrame>,
    /// Diagram-wide concurrent-region counter. PlantUML names region sub-scopes
    /// `CONC2`, `CONC3`, … sequentially across *every* composite in the diagram
    /// (each composite's first region uses the composite's own scope and
    /// consumes no counter value; the counter's first emitted value is 2), so
    /// this is global rather than per-frame. Holds the highest CONC index
    /// allocated so far (initially 1, so the first region becomes `CONC2`).
    conc_counter: usize,
}

/// One enclosing-composite level on the scope stack.
struct ScopeFrame {
    /// Qualified id of the composite itself (region 0's scope).
    base: String,
    /// Active scope id: `base` for region 0, else `<base>.CONC{n}` where `n`
    /// is the diagram-wide counter value assigned when the region opened.
    current: String,
}

impl StateParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            states: Vec::new(),
            transitions: Vec::new(),
            notes: Vec::new(),
            note_buffer: None,
            next_note_creation_order: 0,
            current_line: 0,
            skinparam_block_prefix: None,
            scope_stack: Vec::new(),
            conc_counter: 1,
        }
    }

    /// Fully-qualified id of the current scope (the enclosing composite), or
    /// `None` at top level.
    fn current_scope(&self) -> Option<&str> {
        self.scope_stack.last().map(|f| f.current.as_str())
    }

    /// Resolve a raw state reference within the current scope.
    ///
    /// - `[*]` becomes a scoped pseudo-state marker `[*]<scope>` (the empty
    ///   scope yields plain `[*]`); the renderer splits the marker back into a
    ///   pseudo-state plus its owning composite.
    /// - `[H]` / `[H*]` history markers and other bracketed pseudo-states are
    ///   qualified the same way.
    /// - A plain name nested inside composite `Outer` resolves to `Outer.name`,
    ///   matching PlantUML's qualified entity naming. A name that already
    ///   carries its full scope prefix (rare, when authors dot-qualify) is left
    ///   untouched.
    fn qualify(&self, raw: &str) -> String {
        let raw = raw.trim();
        match self.current_scope() {
            None => raw.to_string(),
            Some(scope) => {
                if raw.starts_with('[') && raw.ends_with(']') {
                    format!("{raw}{scope}")
                } else if raw.starts_with(scope) && raw[scope.len()..].starts_with('.') {
                    raw.to_string()
                } else {
                    format!("{scope}.{raw}")
                }
            }
        }
    }

    /// Resolve an ordinary state name through PlantUML's shared quark tree.
    ///
    /// Java provenance: `StateDiagram` installs `.` as its namespace
    /// separator, and every state/link command calls
    /// `CucaDiagram.quarkInContextSafe(true, name)`. For an unqualified name,
    /// that method reuses the sole existing quark with the same short name
    /// before considering the current composite. State diagrams therefore
    /// share an already-created `Inner` across later sibling composites.
    ///
    /// `StateDiagram.checkConcurrentStateOk` rejects that reuse across
    /// concurrent-region boundaries, so those scopes retain independent
    /// children.
    fn resolve_state_id(&self, raw: &str) -> String {
        let raw = raw.trim();
        if raw.starts_with('[') && raw.ends_with(']') || raw.contains('.') {
            return self.qualify(raw);
        }

        let mut matches = self.states.iter().filter(|state| {
            let existing_is_concurrent = state
                .parent
                .as_deref()
                .and_then(|parent| parent.rsplit('.').next())
                .is_some_and(|segment| segment.starts_with("CONC"));
            let current_is_concurrent = self
                .current_scope()
                .and_then(|scope| scope.rsplit('.').next())
                .is_some_and(|segment| segment.starts_with("CONC"));
            state.id.rsplit('.').next() == Some(raw)
                && (!(existing_is_concurrent || current_is_concurrent)
                    || state.parent.as_deref() == self.current_scope())
        });
        if let Some(sole) = matches.next()
            && matches.next().is_none()
        {
            return sole.id.clone();
        }

        self.qualify(raw)
    }

    fn flush_note(&mut self) {
        if let Some(buf) = self.note_buffer.take() {
            let text = buf.text.trim().to_string();
            if !text.is_empty() {
                self.notes.push(StateNote {
                    text,
                    kind: buf.kind,
                    source_line: buf.source_line,
                    command_line: buf.command_line,
                    creation_order: buf.creation_order,
                });
            }
        }
    }

    fn note_metadata(&mut self) -> (usize, usize) {
        let creation_order = self.next_note_creation_order;
        self.next_note_creation_order += 1;
        (self.current_line, creation_order)
    }

    fn finish(self) -> StateDiagram {
        StateDiagram {
            meta: self.meta,
            states: self.states,
            transitions: self.transitions,
            notes: self.notes,
        }
    }

    fn ensure_state(&mut self, raw: &str) -> String {
        let raw = raw.trim();
        // Pseudo-states ([*], [H], [H*]) are handled by the renderer directly
        // and do not need a corresponding State entry in the states list. They
        // are still scope-qualified so the renderer can map them to the right
        // composite's start/end pseudo-state.
        if raw.starts_with('[') && raw.ends_with(']') {
            return self.qualify(raw);
        }
        let id = self.resolve_state_id(raw);
        let parent = self.current_scope().map(String::from);
        if !self.states.iter().any(|s| s.id == id) {
            self.states.push(State {
                id: id.clone(),
                label: raw.to_string(),
                source_line: self.current_line,
                parent,
                ..State::default()
            });
        }
        id
    }

    fn parse_line(&mut self, line_num: usize, line: &str) -> Result<(), ParseError> {
        self.current_line = line_num;
        // Inside a `skinparam <prefix> { ... }` block: collect `Key Value`
        // pairs as `<prefix><Key>` skinparams until the closing `}`.
        if let Some(prefix) = self.skinparam_block_prefix.clone() {
            if line == "}" {
                self.skinparam_block_prefix = None;
            } else if let Some((key, value)) = line.split_once(' ') {
                self.meta.skinparams.push(crate::diagram::SkinParam {
                    key: format!("{}{}", prefix, key.trim()),
                    value: value.trim().to_string(),
                });
            }
            return Ok(());
        }
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
            let rest = rest.trim();
            if let Some((key, value)) = rest.split_once(' ') {
                let value = value.trim();
                if value == "{" {
                    // Block form: `skinparam state {` — collect nested entries.
                    self.skinparam_block_prefix = Some(key.trim().to_string());
                } else {
                    self.meta.skinparams.push(crate::diagram::SkinParam {
                        key: key.trim().to_string(),
                        value: value.to_string(),
                    });
                }
            } else if let Some(key) = rest.strip_suffix('{') {
                // `skinparam state{` (no space) form.
                self.skinparam_block_prefix = Some(key.trim().to_string());
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

        // Concurrent-region separator inside a composite: a line of two or more
        // `-` (horizontal split) or `|` (vertical split) characters advances
        // the enclosing composite to its next concurrent region. PlantUML scopes
        // region 0 to the composite itself; region N≥1 lives in a synthetic
        // sub-scope `<Composite>.CONC{N+1}` so each region gets its own
        // `[*]` pseudo-states. Only meaningful inside a composite — at top level
        // the line is ignored.
        if is_region_separator(line) {
            if let Some(frame) = self.scope_stack.last_mut() {
                // Java provenance: `StateDiagram.concurrentState` stores
                // `direction` on both the owning state and each synthetic
                // concurrent group. The renderer only needs the owner's value
                // because all of its region images share one composition axis.
                if let Some(state) = self.states.iter_mut().find(|state| state.id == frame.base) {
                    state.concurrent_separator = line.chars().next();
                }
                self.conc_counter += 1;
                let n = self.conc_counter;
                frame.current = format!("{}.CONC{}", frame.base, n);
            }
            return Ok(());
        }

        // Closing brace of a composite block: pop the current scope.
        if line == "}" {
            self.scope_stack.pop();
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
            //
            // Arrow forms recognised between source and target:
            //   `-->`, `->`, `-up->`, `-down->`, `-left->`, `-right->`, `-le->`, ...
            //   `-[#color]->`, `-[#color,thickness=N]->`, etc. (bracketed style)
            //   `..>`, `.up.>`, `-[#blue]..->`, etc. (dotted variants)
            //   `<--`, `<.>`, `<-->` (reverse / bidirectional)
            // The regex consumes any non-space sequence between source and `>` /
            // `<` to keep this loose; typed direction and command orientation
            // are derived from the captured arrow token.
            Regex::new(
                r"^(\[[\w*]*\]|[\w.]+)\s*([-.<>][-.<>\[\]#,=\w]*[->])\s*(\[[\w*]*\]|[\w.]+)(?:\s*:\s*(.+))?$",
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let from = self.ensure_state(&caps[1]);
            let to = self.ensure_state(&caps[3]);
            let label = caps.get(4).map(|m| m.as_str().trim().to_string());
            let transition_style = parse_transition_style(&caps[2]);
            transition_style.record_in(&mut self.meta, self.transitions.len());
            self.transitions.push(Transition {
                from,
                to,
                label,
                arrow: parse_transition_arrow(&caps[2]),
                source_line: self.current_line,
            });
            true
        } else {
            false
        }
    }

    fn try_state_decl(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            // Trailing decoration accepted in any order:
            //   `#color`        — fill colour
            //   `##color`       — line (stroke) colour
            //   `##[dashed]col` — line style + colour
            //   `<<stereotype>>`
            //   `{`             — opens a composite block
            Regex::new(
                // The colour token accepts plain (`#color`), stroke
                // (`##color`), styled (`##[dashed]color`) and gradient
                // (`#c1/c2`, `#c1-c2`, `#c1\c2`) forms.
                r#"^state\s+(?:"([^"]+)"\s+as\s+)?(\w+)(?:\s*<<(\w+\*?)>>)?(?:\s*(##?(?:\[[^\]]*\])?\w+(?:[/\\|-]\w+)?))*(?:\s*\{)?$"#,
            )
            .unwrap()
        });
        // Capture every `#color` / `##color` / `##[style]color` token after
        // the state id so the renderer can recover fill / stroke styling
        // without an oracle.
        static COLOR_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(##?)(?:\[([^\]]*)\])?(\w+(?:[/\\|-]\w+)?)").unwrap());
        // Also handles: state ID : description
        static RE_DESC: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^state\s+(?:"([^"]+)"\s+as\s+)?(\w+)\s*:\s*(.+)$"#).unwrap()
        });
        // `state X [[url]]` / `state X [[url{tooltip}]]` — hyperlink decoration.
        // PlantUML accepts this anywhere after the id; pull it out before the
        // main match so the (otherwise strict) declaration regex still applies.
        static URL_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\[\[([^\]{}]+?)(?:\{([^}]*)\})?\]\]").unwrap());

        let mut url: Option<String> = None;
        let mut tooltip: Option<String> = None;
        let stripped;
        let line = if let Some(uc) = URL_RE.captures(line) {
            url = Some(uc[1].trim().to_string());
            tooltip = uc.get(2).map(|m| m.as_str().trim().to_string());
            stripped = URL_RE.replace(line, "").trim_end().to_string();
            stripped.as_str()
        } else {
            line
        };

        if let Some(caps) = RE.captures(line) {
            let label = caps
                .get(1)
                .map_or_else(|| caps[2].to_string(), |m| m.as_str().to_string());
            let raw_id = caps[2].to_string();
            let id = self.resolve_state_id(&raw_id);
            let parent = self.current_scope().map(String::from);
            let stereotype = caps.get(3).map(|m| m.as_str());
            // A trailing `{` opens a composite block; mark the state and push
            // its qualified id so nested declarations/transitions qualify
            // against it.
            let is_composite = line.trim_end().ends_with('{');

            let kind = match stereotype {
                Some("start") => StateKind::Initial,
                Some("end") => StateKind::Final,
                Some("choice") => StateKind::Choice,
                Some("fork") => StateKind::Fork,
                Some("join") => StateKind::Join,
                Some("history") => StateKind::History,
                Some("history*") => StateKind::DeepHistory,
                Some("entryPoint") => StateKind::EntryPoint,
                Some("exitPoint") => StateKind::ExitPoint,
                _ => StateKind::Normal,
            };
            let ordinary_stereotype = matches!(kind, StateKind::Normal)
                .then(|| stereotype.map(str::to_string))
                .flatten();

            // Walk every `#color` / `##color` / `##[style]color` token in
            // the trailing decoration. Each match's first group is the
            // hash prefix (`#` vs `##`), the optional second group is the
            // style modifier, and the third is the colour name.
            let mut fill: Option<String> = None;
            let mut stroke: Option<String> = None;
            let mut stroke_style: Option<String> = None;
            for cm in COLOR_RE.captures_iter(line) {
                // Skip the matches that overlap with the state name token
                // (e.g. `<<history*>>`) — those don't start with `#`.
                let prefix = &cm[1];
                let style = cm.get(2).map(|m| m.as_str().to_string());
                let color = cm[3].to_string();
                match prefix {
                    "##" if stroke.is_none() => {
                        stroke = Some(color);
                        stroke_style = style;
                    }
                    "#" if fill.is_none() => {
                        fill = Some(color);
                    }
                    _ => {}
                }
            }

            if let Some(state) = self.states.iter_mut().find(|s| s.id == id) {
                state.label = label;
                state.kind = kind;
                state.composite |= is_composite;
                if state.decl_line.is_none() {
                    state.decl_line = Some(self.current_line);
                }
                if state.parent.is_none() {
                    state.parent = parent;
                }
                if state.source_line == 0 {
                    state.source_line = self.current_line;
                }
                if state.fill.is_none() {
                    state.fill = fill;
                }
                if state.stroke.is_none() {
                    state.stroke = stroke;
                }
                if state.stroke_style.is_none() {
                    state.stroke_style = stroke_style;
                }
                if state.stereotype.is_none() {
                    state.stereotype = ordinary_stereotype;
                }
                if state.url.is_none() {
                    state.url = url;
                    state.tooltip = tooltip;
                }
            } else {
                self.states.push(State {
                    id: id.clone(),
                    label,
                    kind,
                    source_line: self.current_line,
                    decl_line: Some(self.current_line),
                    fill,
                    stroke,
                    stroke_style,
                    stereotype: ordinary_stereotype,
                    url,
                    tooltip,
                    composite: is_composite,
                    parent,
                    ..State::default()
                });
            }
            if is_composite {
                self.scope_stack.push(ScopeFrame {
                    base: id.clone(),
                    current: id,
                });
            }
            true
        } else if let Some(caps) = RE_DESC.captures(line) {
            let label = caps
                .get(1)
                .map_or_else(|| caps[2].to_string(), |m| m.as_str().to_string());
            let id = self.resolve_state_id(&caps[2]);
            let parent = self.current_scope().map(String::from);
            let desc = caps[3].trim().to_string();
            if let Some(state) = self.states.iter_mut().find(|s| s.id == id) {
                state.label = label;
                state.descriptions.push(desc);
                if state.decl_line.is_none() {
                    state.decl_line = Some(self.current_line);
                }
            } else {
                self.states.push(State {
                    id: id.clone(),
                    label,
                    descriptions: vec![desc],
                    source_line: self.current_line,
                    decl_line: Some(self.current_line),
                    parent,
                    ..State::default()
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
            let id = self.ensure_state(&caps[1]);
            let desc = caps[2].trim().to_string();
            if let Some(state) = self.states.iter_mut().find(|s| s.id == id) {
                state.descriptions.push(desc);
                if state.decl_line.is_none() {
                    state.decl_line = Some(self.current_line);
                }
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

        // `note [left|right|top|bottom] on link` mutates the most recently
        // created transition during PlantUML's second parser pass.
        {
            static RE: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new(
                    r"^note(?:\s+(left|right|top|bottom))?\s+(?:on|of)\s+link(?:\s*:\s*(.*))?$",
                )
                .unwrap()
            });
            let Some(caps) = RE.captures(line) else {
                return self.parse_non_link_note(line);
            };
            let Some(transition_index) = self.transitions.len().checked_sub(1) else {
                return true;
            };
            let position = match caps.get(1).map(|capture| capture.as_str()) {
                Some("left") => StateNotePosition::Left,
                Some("right") => StateNotePosition::Right,
                Some("top") => StateNotePosition::Top,
                Some("bottom") | None => StateNotePosition::Bottom,
                Some(_) => unreachable!("note-link regex validates position"),
            };
            let kind = StateNoteKind::OnLink {
                transition_index,
                position,
            };
            let inline = caps.get(2).map(|capture| capture.as_str().trim());
            if let Some(text) = inline.filter(|t| !t.is_empty()) {
                let (source_line, creation_order) = self.note_metadata();
                self.notes.push(StateNote {
                    text: text.to_string(),
                    kind,
                    source_line,
                    command_line: source_line,
                    creation_order,
                });
            } else {
                let (source_line, creation_order) = self.note_metadata();
                self.note_buffer = Some(NoteBuffer {
                    kind,
                    text: String::new(),
                    source_line: source_line + 1,
                    command_line: source_line,
                    creation_order,
                });
            }
            true
        }
    }

    fn parse_non_link_note(&mut self, line: &str) -> bool {
        if !line.starts_with("note") {
            return false;
        }

        // `note "floating text" as ALIAS`
        {
            static RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r#"^note\s+"([^"]+)"\s+as\s+(\w+)$"#).unwrap());
            if let Some(caps) = RE.captures(line) {
                let text = caps[1].to_string();
                let alias = caps[2].to_string();
                let (source_line, creation_order) = self.note_metadata();
                self.notes.push(StateNote {
                    text,
                    kind: StateNoteKind::Floating(Some(alias)),
                    source_line,
                    command_line: source_line,
                    creation_order,
                });
                return true;
            }
        }

        // `note as ALIAS` followed by a multiline body.
        {
            static RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"^note\s+as\s+([\w.]+)$").unwrap());
            if let Some(caps) = RE.captures(line) {
                let (source_line, creation_order) = self.note_metadata();
                self.note_buffer = Some(NoteBuffer {
                    kind: StateNoteKind::Floating(Some(caps[1].to_string())),
                    text: String::new(),
                    source_line: source_line + 1,
                    command_line: source_line,
                    creation_order,
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
                    let (source_line, creation_order) = self.note_metadata();
                    self.notes.push(StateNote {
                        text,
                        kind,
                        source_line,
                        command_line: source_line,
                        creation_order,
                    });
                } else {
                    let (source_line, creation_order) = self.note_metadata();
                    self.note_buffer = Some(NoteBuffer {
                        kind,
                        text: String::new(),
                        source_line: source_line + 1,
                        command_line: source_line,
                        creation_order,
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
                    let (source_line, creation_order) = self.note_metadata();
                    self.notes.push(StateNote {
                        text,
                        kind,
                        source_line,
                        command_line: source_line,
                        creation_order,
                    });
                } else {
                    let (source_line, creation_order) = self.note_metadata();
                    self.note_buffer = Some(NoteBuffer {
                        kind,
                        text: String::new(),
                        source_line: source_line + 1,
                        command_line: source_line,
                        creation_order,
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
    fn concurrent_separator_is_recorded_on_owning_composite() {
        let d = parse(
            "state ParallelHarbor {\n\
             [*] --> Copper\n\
             Copper --> [*]\n\
             ||\n\
             [*] --> Violet\n\
             Violet --> [*]\n\
             }",
        );

        let composite = d
            .states
            .iter()
            .find(|state| state.id == "ParallelHarbor")
            .unwrap();
        assert_eq!(composite.concurrent_separator, Some('|'));
        assert_eq!(
            d.states
                .iter()
                .find(|state| state.label == "Violet")
                .and_then(|state| state.parent.as_deref()),
            Some("ParallelHarbor.CONC2"),
        );
    }

    #[test]
    fn top_level_separator_does_not_consume_a_concurrent_scope_number() {
        let d = parse(
            "--\n\
             state ParallelHarbor {\n\
             [*] --> Copper\n\
             --\n\
             [*] --> Violet\n\
             }",
        );

        assert!(d.states.iter().any(|state| {
            state.label == "Violet" && state.parent.as_deref() == Some("ParallelHarbor.CONC2")
        }));
    }

    #[test]
    fn sibling_composites_reuse_the_sole_existing_short_name_quark() {
        let d = parse(
            "state CopperVault {\n\
             [*] --> SharedRelay\n\
             SharedRelay --> [*]\n\
             }\n\
             state VioletVault {\n\
             [*] --> SharedRelay\n\
             SharedRelay --> [*]\n\
             }",
        );

        assert_eq!(
            d.states
                .iter()
                .filter(|state| state.label == "SharedRelay")
                .map(|state| state.id.as_str())
                .collect::<Vec<_>>(),
            vec!["CopperVault.SharedRelay"]
        );
        assert_eq!(d.transitions[2].to, "CopperVault.SharedRelay");
        assert_eq!(d.transitions[3].from, "CopperVault.SharedRelay");
    }

    #[test]
    fn concurrent_regions_keep_same_named_children_scope_local() {
        let d = parse(
            "state ParallelVault {\n\
             [*] --> SharedRelay\n\
             --\n\
             [*] --> SharedRelay\n\
             }",
        );

        assert!(
            d.states
                .iter()
                .any(|state| state.id == "ParallelVault.SharedRelay")
        );
        assert!(
            d.states
                .iter()
                .any(|state| state.id == "ParallelVault.CONC2.SharedRelay")
        );
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
    fn transition_arrows_preserve_command_orientation_and_link_inversion() {
        let d = parse(
            "Alpha --> Beta\n\
             Beta -left-> Gamma\n\
             Gamma -up-> Delta\n\
             Delta <-- Epsilon",
        );

        assert_eq!(d.transitions[0].arrow, TransitionArrow::default());
        assert_eq!(
            d.transitions[1].arrow,
            TransitionArrow {
                direction: Some(TransitionDirection::Left),
                command_reversed: false,
            }
        );
        assert!(d.transitions[1].arrow.reverses_solved_endpoints());
        assert!(d.transitions[1].arrow.arrow_at_start());
        assert_eq!(
            d.transitions[2].arrow.direction,
            Some(TransitionDirection::Up)
        );
        assert_eq!(
            d.transitions[3].arrow,
            TransitionArrow {
                direction: Some(TransitionDirection::Left),
                command_reversed: true,
            }
        );
        assert!(!d.transitions[3].arrow.reverses_solved_endpoints());
        assert!(d.transitions[3].arrow.arrow_at_start());
        assert!(d.transitions[3].arrow.is_horizontal());
    }

    #[test]
    fn bracketed_transition_styles_are_typed() {
        let d = parse(
            "Alpha -[#darkcyan,dashed,thickness=2]-> Beta\n\
             Beta -right[#7B68EE,dotted]-> Gamma",
        );

        assert_eq!(
            d.transition_style(0),
            TransitionStyle {
                color: Some("darkcyan".to_string()),
                line_style: Some(TransitionLineStyle::Dashed),
                thickness: Some(2.0),
            }
        );
        assert_eq!(
            d.transition_style(1),
            TransitionStyle {
                color: Some("7B68EE".to_string()),
                line_style: Some(TransitionLineStyle::Dotted),
                thickness: None,
            }
        );
    }

    #[test]
    fn state_stereotypes() {
        let d = parse(
            "state s1 <<start>>\nstate s2 <<end>>\nstate s3 <<choice>>\n\
             state s4 <<fork>>\nstate s5 <<join>>",
        );
        assert_eq!(d.states[0].kind, StateKind::Initial);
        assert_eq!(d.states[0].stereotype, None);
        assert_eq!(d.states[1].kind, StateKind::Final);
        assert_eq!(d.states[1].stereotype, None);
        assert_eq!(d.states[2].kind, StateKind::Choice);
        assert_eq!(d.states[3].kind, StateKind::Fork);
        assert_eq!(d.states[4].kind, StateKind::Join);
    }

    #[test]
    fn ordinary_state_stereotype() {
        let d = parse("state A <<important>>");
        assert_eq!(d.states[0].kind, StateKind::Normal);
        assert_eq!(d.states[0].stereotype.as_deref(), Some("important"));
    }

    #[test]
    fn state_gradient_fill_preserves_colors_and_policy() {
        let d = parse(
            "state CopperRelay #cyan/pink\n\
             state AmberRelay #red|blue\n\
             state VioletRelay #green-yellow",
        );
        assert_eq!(d.states[0].fill.as_deref(), Some("cyan/pink"));
        assert_eq!(d.states[1].fill.as_deref(), Some("red|blue"));
        assert_eq!(d.states[2].fill.as_deref(), Some("green-yellow"));
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
        assert_eq!(d.notes[0].source_line, 3);
        assert_eq!(d.notes[0].command_line, 2);
        assert_eq!(d.notes[0].creation_order, 0);
    }

    #[test]
    fn notes_keep_command_lines_and_creation_order_across_multiline_flushes() {
        let d = parse(
            "[*] --> RenamedAlpha\n\
             note right of RenamedAlpha\n\
               first line\n\
               second line\n\
             end note\n\
             RenamedAlpha --> RenamedBeta\n\
             note left of RenamedBeta : third note\n\
             RenamedBeta --> [*]",
        );

        assert_eq!(
            d.notes
                .iter()
                .map(|note| (note.source_line, note.command_line, note.creation_order))
                .collect::<Vec<_>>(),
            [(3, 2, 0), (7, 7, 1)]
        );
    }

    #[test]
    fn floating_note() {
        let d = parse("note \"Floating note 1\" as FN1\n[*] --> A\nA --> [*]");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "Floating note 1");
        assert!(matches!(&d.notes[0].kind, StateNoteKind::Floating(Some(a)) if a == "FN1"));
    }

    #[test]
    fn note_on_link() {
        let d = parse("[*] --> A\nA --> [*]\nnote on link\n  link note\nend note");
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "link note");
        assert!(matches!(
            &d.notes[0].kind,
            StateNoteKind::OnLink {
                transition_index: 1,
                position: StateNotePosition::Bottom,
            }
        ));
    }

    #[test]
    fn positioned_link_note_keeps_transition_ownership() {
        let d = parse(
            "[*] --> RenamedAlpha\n\
             RenamedAlpha --> RenamedBeta : event\n\
             note left on link : owned note\n\
             RenamedBeta --> [*]",
        );
        assert!(matches!(
            &d.notes[0].kind,
            StateNoteKind::OnLink {
                transition_index: 1,
                position: StateNotePosition::Left,
            }
        ));
    }

    #[test]
    fn multiline_named_floating_note_keeps_alias_and_locations() {
        let d = parse(
            "note as FloatingAlias\n\
               renamed first line\n\
               renamed second line\n\
             end note\n\
             [*] --> A\n\
             A --> [*]",
        );
        assert!(matches!(
            &d.notes[0].kind,
            StateNoteKind::Floating(Some(alias)) if alias == "FloatingAlias"
        ));
        assert_eq!(d.notes[0].source_line, 2);
        assert_eq!(d.notes[0].command_line, 1);
    }
}
