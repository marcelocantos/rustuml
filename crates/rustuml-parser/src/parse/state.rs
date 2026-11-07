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
    parser.predeclare_state_structure(lines)?;
    parser.reset_after_predeclaration();
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
        if parser.first_content_line.is_none() && !trimmed.starts_with('\'') {
            parser.first_content_line = Some(source_line);
        }
        parser.parse_line(source_line, trimmed)?;
    }
    // Flush any unclosed note buffer.
    parser.flush_note();
    parser.check_final_error()?;
    Ok(parser.finish())
}

/// A concurrent-region separator is a line made entirely of two or more `-`
/// (horizontal split) or two or more `|` (vertical split). PlantUML treats
/// `--`, `---`, `||`, etc. inside a composite as region dividers.
fn is_region_separator(line: &str) -> bool {
    (line.len() >= 2 && line.bytes().all(|b| b == b'-'))
        || (line.len() >= 2 && line.bytes().all(|b| b == b'|'))
}

/// How a command uses a state name.
///
/// Java provenance: `CommandLinkStateCommon.getEntity` and
/// `CommandCreateState.executeArg` call `StateDiagram.checkConcurrentStateOk`.
/// `CommandAddField.executeArg` only calls `quarkInContext` and does not.
#[derive(Clone, Copy)]
enum StateNameUse {
    Link,
    Declaration,
    Unchecked,
}

/// `StateDiagram.checkConcurrentStateOkInternal` tests
/// `GroupType.CONCURRENT_STATE` (StateDiagram.java:75-90). The `CONC` prefix
/// (`:58`, `getUniqueSequence2` at `:201-204`) is only the synthetic name.
/// A group has that type while the last `gotoGroup` that entered it was a
/// separator's `muteToGroupType(CONCURRENT_STATE)` (CucaDiagram.java:350-356).
fn parent_is_concurrent(concurrent_now: &[String], parent: Option<&str>) -> bool {
    parent.is_some_and(|parent| concurrent_now.iter().any(|id| id == parent))
}

/// `StateDiagram.checkConcurrentStateOkInternal` (StateDiagram.java:75-90).
fn check_concurrent_state_ok(
    concurrent_now: &[String],
    existing_parent: Option<&str>,
    scope: Option<&str>,
) -> bool {
    if parent_is_concurrent(concurrent_now, scope) && scope != existing_parent {
        return false;
    }
    if parent_is_concurrent(concurrent_now, existing_parent) && scope != existing_parent {
        return false;
    }
    true
}

/// The short name of a quark id: its last `.` segment.
fn short_name(id: &str) -> &str {
    id.rsplit('.').next().unwrap_or(id)
}

fn concurrent_state_error(line: usize, use_kind: &StateNameUse, name: &str) -> ParseError {
    let message = match use_kind {
        StateNameUse::Link => format!("The state {name} cannot be used here."),
        StateNameUse::Declaration => format!(
            "The state {name} has been created in a concurrent state : it cannot be used here."
        ),
        StateNameUse::Unchecked => unreachable!("unchecked state names are not rejected"),
    };
    ParseError { line, message }
}

/// `HColorSet.parseSimpleColor` plus the symbolic colors `parseColor` accepts
/// before it. Names are the pinned `ColorTrieNode` registry already listed by
/// `sequence::valid_return_color`. `sequence.rs` is not edited from this slice.
fn state_hcolor_simple(raw: &str) -> bool {
    let color = raw.strip_prefix('#').unwrap_or(raw);
    if matches!(color.len(), 1 | 3 | 6 | 8) && color.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return true;
    }
    matches!(
        color.to_ascii_lowercase().as_str(),
        "aliceblue"
            | "antiquewhite"
            | "application"
            | "aqua"
            | "aquamarine"
            | "automatic"
            | "azure"
            | "background"
            | "beige"
            | "bisque"
            | "black"
            | "blanchedalmond"
            | "blue"
            | "blueviolet"
            | "brown"
            | "burlywood"
            | "business"
            | "cadetblue"
            | "chartreuse"
            | "chocolate"
            | "coral"
            | "cornflowerblue"
            | "cornsilk"
            | "crimson"
            | "cyan"
            | "darkblue"
            | "darkcyan"
            | "darkgoldenrod"
            | "darkgray"
            | "darkgreen"
            | "darkgrey"
            | "darkkhaki"
            | "darkmagenta"
            | "darkolivegreen"
            | "darkorange"
            | "darkorchid"
            | "darkred"
            | "darksalmon"
            | "darkseagreen"
            | "darkslateblue"
            | "darkslategray"
            | "darkslategrey"
            | "darkturquoise"
            | "darkviolet"
            | "deeppink"
            | "deepskyblue"
            | "dimgray"
            | "dimgrey"
            | "dodgerblue"
            | "firebrick"
            | "floralwhite"
            | "forestgreen"
            | "fuchsia"
            | "gainsboro"
            | "ghostwhite"
            | "gold"
            | "goldenrod"
            | "gray"
            | "green"
            | "greenyellow"
            | "grey"
            | "honeydew"
            | "hotpink"
            | "implementation"
            | "indianred"
            | "indigo"
            | "ivory"
            | "khaki"
            | "lavender"
            | "lavenderblush"
            | "lawngreen"
            | "lemonchiffon"
            | "lightblue"
            | "lightcoral"
            | "lightcyan"
            | "lightgoldenrodyellow"
            | "lightgray"
            | "lightgreen"
            | "lightgrey"
            | "lightpink"
            | "lightsalmon"
            | "lightseagreen"
            | "lightskyblue"
            | "lightslategray"
            | "lightslategrey"
            | "lightsteelblue"
            | "lightyellow"
            | "lime"
            | "limegreen"
            | "linen"
            | "magenta"
            | "maroon"
            | "mediumaquamarine"
            | "mediumblue"
            | "mediumorchid"
            | "mediumpurple"
            | "mediumseagreen"
            | "mediumslateblue"
            | "mediumspringgreen"
            | "mediumturquoise"
            | "mediumvioletred"
            | "midnightblue"
            | "mintcream"
            | "mistyrose"
            | "moccasin"
            | "motivation"
            | "navajowhite"
            | "navy"
            | "oldlace"
            | "olive"
            | "olivedrab"
            | "orange"
            | "orangered"
            | "orchid"
            | "palegoldenrod"
            | "palegreen"
            | "paleturquoise"
            | "palevioletred"
            | "papayawhip"
            | "peachpuff"
            | "peru"
            | "physical"
            | "pink"
            | "plum"
            | "powderblue"
            | "purple"
            | "red"
            | "rosybrown"
            | "royalblue"
            | "saddlebrown"
            | "salmon"
            | "sandybrown"
            | "seagreen"
            | "seashell"
            | "sienna"
            | "silver"
            | "skyblue"
            | "slateblue"
            | "slategray"
            | "slategrey"
            | "snow"
            | "springgreen"
            | "steelblue"
            | "strategy"
            | "tan"
            | "teal"
            | "technology"
            | "thistle"
            | "tomato"
            | "transparent"
            | "turquoise"
            | "violet"
            | "wheat"
            | "white"
            | "whitesmoke"
            | "yellow"
            | "yellowgreen"
    )
}

/// `HColorSet.parseColor` (HColorSet.java:75-116) for a token the state
/// declaration regex has already captured: a simple color, or the first
/// `-`, `\`, `|`, or `/` gradient whose two sides are simple colors.
fn state_hcolor_accepted(token: &str) -> bool {
    let color = token.strip_prefix('#').unwrap_or(token);
    if state_hcolor_simple(color) {
        return true;
    }
    let bytes = color.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(byte, b'-' | b'\\' | b'|' | b'/') {
            let left = &color[..index];
            let right = &color[index + 1..];
            return !left.is_empty()
                && !right.is_empty()
                && state_hcolor_simple(left)
                && state_hcolor_simple(right);
        }
    }
    false
}

fn state_color_error(line: usize) -> ParseError {
    ParseError {
        line,
        message: "No such color".to_string(),
    }
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
    direction: StateLayoutDirection,
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
    /// Region quarks created or reused by a separator. Java quarks persist
    /// across passes (only the counter and the group stack are reset in
    /// `CucaDiagram.startingPass`, CucaDiagram.java:159-165), and every quark
    /// counts in `Plasma.countByName` (Plasma.java:63-72), so a synthetic
    /// region that has no `State` entry still takes part in short-name
    /// resolution.
    region_ids: Vec<String>,
    /// Quarks whose group type is `CONCURRENT_STATE` right now. A separator
    /// mutes its region to that type; `state X {` and `X[H]` mute X back to
    /// `STATE` (`gotoGroup` → `muteToGroupType`, CucaDiagram.java:350-356).
    concurrent_now: Vec<String>,
    /// Source line of the first content line. `StateDiagram.checkFinalError`
    /// is located there (see `check_final_error`).
    first_content_line: Option<usize>,
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
            direction: StateLayoutDirection::TopToBottom,
            states: Vec::new(),
            transitions: Vec::new(),
            notes: Vec::new(),
            note_buffer: None,
            next_note_creation_order: 0,
            current_line: 0,
            skinparam_block_prefix: None,
            scope_stack: Vec::new(),
            conc_counter: 1,
            region_ids: Vec::new(),
            concurrent_now: Vec::new(),
            first_content_line: None,
        }
    }

    /// Fully-qualified id of the current scope (the enclosing composite), or
    /// `None` at top level.
    fn current_scope(&self) -> Option<&str> {
        self.scope_stack.last().map(|f| f.current.as_str())
    }

    /// Populate the quark-equivalent state tree before transition parsing.
    ///
    /// Java provenance: `PSystemCommandFactory.createSystem` rewinds the full
    /// source for each `ParserPass`. State declarations, composite scopes,
    /// descriptions, and concurrent separators run in pass one, while
    /// `CommandLinkStateCommon` runs only in pass two.
    fn predeclare_state_structure(&mut self, lines: &[String]) -> Result<(), ParseError> {
        let mut in_note = false;
        let mut in_skinparam_block = false;

        for (i, line) in lines.iter().enumerate() {
            let (source_line, trimmed) = super::source_line_and_trimmed(i + 1, line);
            self.current_line = source_line;

            if in_note {
                if matches!(trimmed, "end note" | "endnote") {
                    in_note = false;
                }
                continue;
            }
            if in_skinparam_block {
                if trimmed == "}" {
                    in_skinparam_block = false;
                }
                continue;
            }
            if trimmed.is_empty() {
                continue;
            }
            let is_note_command = trimmed == "note"
                || trimmed
                    .strip_prefix("note")
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(char::is_whitespace);
            if is_note_command {
                in_note = !trimmed.contains(':') && !trimmed.starts_with("note \"");
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("skinparam ") {
                in_skinparam_block = rest.trim_end().ends_with('{')
                    || rest.split_once(' ').is_some_and(|(_, v)| v.trim() == "{");
                continue;
            }
            if is_region_separator(trimmed) {
                self.advance_concurrent_region(trimmed);
                continue;
            }
            if trimmed == "}" {
                self.scope_stack.pop();
                continue;
            }
            if !self.try_state_decl(trimmed)? {
                let _ = self.try_state_description(trimmed)?;
            }
        }
        Ok(())
    }

    fn reset_after_predeclaration(&mut self) {
        for state in &mut self.states {
            state.descriptions.clear();
            state.concurrent_separator = None;
            state.concurrent_regions.clear();
            // The prepass fixes quark identity and declaration metadata, but
            // `source_line` models first textual use for renderer ordering.
            // Let the full pass repopulate it from the first reference or
            // declaration it encounters.
            state.source_line = 0;
        }
        self.scope_stack.clear();
        self.conc_counter = 1;
        self.current_line = 0;
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
        self.qualify_in(raw, self.current_scope())
    }

    fn qualify_in(&self, raw: &str, scope: Option<&str>) -> String {
        let raw = raw.trim();
        match scope {
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
    /// `CucaDiagram.quarkInContextSafe(true, name)` (CucaDiagram.java:248-290).
    /// For an unqualified name, that method reuses the quark when
    /// `countByName(name) == 1` and that quark is not the current group.
    /// Otherwise it takes the child of the current group. State diagrams
    /// therefore share an already-created `Inner` across later sibling
    /// composites.
    ///
    /// `StateDiagram.checkConcurrentStateOk` rejects the resolved quark across
    /// concurrent-region boundaries. Link and declaration commands report
    /// the refusal. A field line does not.
    fn resolve_state_id(&self, raw: &str, use_kind: StateNameUse) -> Result<String, ParseError> {
        self.resolve_state_id_in(raw, self.current_scope(), use_kind)
    }

    /// `Plasma.countByName` and `firstWithName` (Plasma.java:96-108) over the
    /// quarks this parser models: every `State` plus every separator region.
    fn quarks_named(&self, name: &str) -> Vec<&str> {
        let mut ids: Vec<&str> = Vec::new();
        let states = self.states.iter().map(|state| state.id.as_str());
        for id in states.chain(self.region_ids.iter().map(String::as_str)) {
            if short_name(id) == name && !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }

    /// The parent quark of a modelled quark. A `State` records its parent.
    /// A synthetic region is `<owner>.CONCn`, created as a child of its owner.
    fn quark_parent(&self, id: &str) -> Option<String> {
        if let Some(state) = self.states.iter().find(|state| state.id == id) {
            return state.parent.clone();
        }
        id.rsplit_once('.').map(|(parent, _)| parent.to_string())
    }

    fn is_synthetic_region(&self, id: &str) -> bool {
        self.region_ids.iter().any(|region| region == id)
            && !self.states.iter().any(|state| state.id == id)
    }

    fn resolve_state_id_in(
        &self,
        raw: &str,
        scope: Option<&str>,
        use_kind: StateNameUse,
    ) -> Result<String, ParseError> {
        let raw = raw.trim();
        // `CommandLinkStateCommon.getEntity` returns the current group when
        // its short name equals the token, before `quarkInContext` and
        // `checkConcurrentStateOk`. A composite born in a concurrent region
        // can still be the source of a link drawn inside itself.
        // `CommandCreateState` has no such short-circuit.
        if matches!(use_kind, StateNameUse::Link)
            && let Some(scope) = scope
            && short_name(scope) == raw
        {
            return Ok(scope.to_string());
        }
        if raw.starts_with('[') && raw.ends_with(']') || raw.contains('.') {
            let id = self.qualify_in(raw, scope);
            self.reject_resolved_state(&id, raw, scope, &use_kind)?;
            return Ok(id);
        }

        // `CucaDiagram.quarkInContextSafe` (CucaDiagram.java:264-272): one
        // quark of that short name, which is not the current group, is the
        // quark, including one that lives in a concurrent region. The
        // concurrent check then accepts or rejects it. Any other count takes
        // the current group's child.
        let named = self.quarks_named(raw);
        if let [sole] = named.as_slice()
            && Some(*sole) != scope
            // A synthetic region has no `State` the renderer can draw as a
            // link endpoint or leaf, so it is not handed out here. Java
            // resolves to the region group itself. Declared residue.
            && !self.is_synthetic_region(sole)
        {
            let parent = self.quark_parent(sole);
            self.reject_parent(parent.as_deref(), raw, scope, &use_kind)?;
            return Ok((*sole).to_string());
        }

        let id = self.qualify_in(raw, scope);
        self.reject_resolved_state(&id, raw, scope, &use_kind)?;
        Ok(id)
    }

    fn reject_resolved_state(
        &self,
        id: &str,
        raw: &str,
        scope: Option<&str>,
        use_kind: &StateNameUse,
    ) -> Result<(), ParseError> {
        let Some(state) = self.states.iter().find(|state| state.id == id) else {
            return Ok(());
        };
        self.reject_parent(state.parent.as_deref(), raw, scope, use_kind)
    }

    fn reject_parent(
        &self,
        existing_parent: Option<&str>,
        raw: &str,
        scope: Option<&str>,
        use_kind: &StateNameUse,
    ) -> Result<(), ParseError> {
        if matches!(use_kind, StateNameUse::Unchecked)
            || check_concurrent_state_ok(&self.concurrent_now, existing_parent, scope)
        {
            return Ok(());
        }
        Err(concurrent_state_error(self.current_line, use_kind, raw))
    }

    /// `gotoGroup(..., GroupType.STATE)` mutes a reopened quark back to a
    /// plain state (CucaDiagram.java:350-356).
    fn mute_to_state(&mut self, id: &str) {
        self.concurrent_now.retain(|region| region != id);
    }

    /// The parent of a transition endpoint. A pseudo-state marker carries
    /// the group it was created in (`getStart`/`getEnd`/`getHistorical`
    /// create a child of the current group, StateDiagram.java:92-190).
    fn endpoint_parent(&self, endpoint: &str) -> Option<String> {
        if endpoint.starts_with('[')
            && let Some(close) = endpoint.find(']')
        {
            let scope = &endpoint[close + 1..];
            return (!scope.is_empty()).then(|| scope.to_string());
        }
        self.quark_parent(endpoint)
    }

    /// `StateDiagram.getGroupParentIfItIsConcurrentState`
    /// (StateDiagram.java:256-266): the nearest ancestor whose group type is
    /// `CONCURRENT_STATE`, walking every level.
    fn nearest_concurrent_ancestor(&self, endpoint: &str) -> Option<String> {
        let mut parent = self.endpoint_parent(endpoint);
        while let Some(id) = parent {
            if self.concurrent_now.contains(&id) {
                return Some(id);
            }
            parent = self.quark_parent(&id);
        }
        None
    }

    /// `StateDiagram.checkFinalError` (StateDiagram.java:232-254) refuses a
    /// link whose endpoints have different nearest concurrent ancestors.
    ///
    /// Java never shows its own message. `PSystemCommandFactory.finalizeDiagram`
    /// locates the error at `it.next()` of the rewound iterator, the first
    /// line after `@startuml` (PSystemCommandFactory.java:115-121), so its
    /// trace holds two lines and `PSystemError.score` is 20
    /// (PSystemError.java:379-382). `SequenceDiagramFactory` runs before
    /// `StateDiagramFactory` (PSystemBuilder.java:131-135). It fails on a
    /// leading state command with "Syntax Error?" and the same score, and
    /// `PSystemErrorUtils.mergeV2` keeps the first maximum
    /// (PSystemErrorUtils.java:85-92). The report is that line-1 syntax error.
    fn check_final_error(&self) -> Result<(), ParseError> {
        for transition in &self.transitions {
            let from = self.nearest_concurrent_ancestor(&transition.from);
            let to = self.nearest_concurrent_ancestor(&transition.to);
            if from != to {
                return Err(ParseError {
                    line: self.first_content_line.unwrap_or(1),
                    message: "Syntax Error?".to_string(),
                });
            }
        }
        Ok(())
    }

    /// Record where Java creates the newest note entity and resolve its
    /// `of <state>` anchor in the same quark context.
    fn scope_last_note(&mut self) {
        let parent = self.current_scope().map(String::from);
        let anchor = self.notes.last().and_then(|note| match &note.kind {
            StateNoteKind::LeftOf(anchor) | StateNoteKind::RightOf(anchor)
                if !anchor.is_empty() =>
            {
                Some(anchor.clone())
            }
            _ => None,
        });
        let anchor_id = anchor
            .and_then(|anchor| self.resolve_state_id(&anchor, StateNameUse::Unchecked).ok())
            .filter(|id| self.states.iter().any(|state| state.id == *id));
        if let Some(note) = self.notes.last_mut() {
            note.parent = parent;
            note.anchor_id = anchor_id;
        }
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
                    parent: None,
                    anchor_id: None,
                });
                self.scope_last_note();
            }
        }
    }

    fn note_metadata(&mut self) -> (usize, usize) {
        let creation_order = self.next_note_creation_order;
        self.next_note_creation_order += 1;
        (self.current_line, creation_order)
    }

    fn finish(mut self) -> StateDiagram {
        // StateDiagramFactory registers CommandFactoryNoteOnEntity for
        // ParserPass.THREE: an anchor first created by a later line (a pass-two
        // transition) is known when the note executes.
        for index in 0..self.notes.len() {
            let note = &self.notes[index];
            let anchor = match &note.kind {
                StateNoteKind::LeftOf(anchor) | StateNoteKind::RightOf(anchor)
                    if note.anchor_id.is_none() && !anchor.is_empty() =>
                {
                    anchor
                }
                _ => continue,
            };
            let Ok(id) =
                self.resolve_state_id_in(anchor, note.parent.as_deref(), StateNameUse::Unchecked)
            else {
                continue;
            };
            if self.states.iter().any(|state| state.id == id) {
                self.notes[index].anchor_id = Some(id);
            }
        }
        StateDiagram {
            meta: self.meta,
            direction: self.direction,
            states: self.states,
            transitions: self.transitions,
            notes: self.notes,
        }
    }

    fn ensure_state(&mut self, raw: &str, use_kind: StateNameUse) -> Result<String, ParseError> {
        let raw = raw.trim();
        // Pseudo-states ([*], [H], [H*]) are handled by the renderer directly
        // and do not need a corresponding State entry in the states list. They
        // are still scope-qualified so the renderer can map them to the right
        // composite's start/end pseudo-state.
        if raw.starts_with('[') && raw.ends_with(']') {
            return Ok(self.qualify(raw));
        }
        // CommandLinkStateCommon: `X[H]` / `X[H*]` call StateDiagram
        // getHistorical/getDeepHistory(location, X), which resolve X with
        // quarkInContext(true, ...), enter it as a STATE group and create the
        // history pseudo-state inside it.
        for marker in ["[H*]", "[H]"] {
            if let Some(base) = raw.strip_suffix(marker)
                && !base.is_empty()
            {
                let group = self.ensure_state(base, use_kind)?;
                self.mute_to_state(&group);
                if let Some(state) = self.states.iter_mut().find(|state| state.id == group) {
                    state.composite = true;
                }
                return Ok(format!("{marker}{group}"));
            }
        }
        let id = self.resolve_state_id(raw, use_kind)?;
        let parent = self.current_scope().map(String::from);
        if let Some(state) = self.states.iter_mut().find(|state| state.id == id) {
            if state.source_line == 0 {
                state.source_line = self.current_line;
            }
        } else {
            self.states.push(State {
                id: id.clone(),
                label: raw.to_string(),
                source_line: self.current_line,
                parent,
                ..State::default()
            });
        }
        Ok(id)
    }

    fn advance_concurrent_region(&mut self, line: &str) {
        let Some(frame) = self.scope_stack.last() else {
            return;
        };
        // `StateDiagram.concurrentState` (StateDiagram.java:194-208) first
        // leaves a current concurrent group, so the current quark is the
        // owning composite. Then it names the new group with
        // `getUniqueSequence2("CONC")` (CucaDiagram.java:816-818): one
        // counter per diagram, reset at each pass, advanced by every
        // separator whether the name then reuses a quark or not. The letters
        // are only that sequence. The region type is the group flag
        // `gotoGroup` applies with `muteToGroupType(CONCURRENT_STATE)`.
        let base = frame.base.clone();
        let separator = line.chars().next();
        if let Some(state) = self.states.iter_mut().find(|state| state.id == base) {
            state.concurrent_separator = separator;
        }
        self.conc_counter += 1;
        let sequence_name = format!("CONC{}", self.conc_counter);
        // `quarkInContext(true, sequenceName)` (CucaDiagram.java:264-272):
        // the sole quark of that short name is reused unless it is the owner.
        // The count includes earlier regions, so a leaf declared inside a
        // region under the same name does not capture the next pass's
        // separator. Otherwise the owner's child is the region.
        let named = self.quarks_named(&sequence_name);
        let reused = match named.as_slice() {
            [sole] if *sole != base => Some((*sole).to_string()),
            _ => None,
        };
        let region_id = if let Some(id) = reused {
            if let Some(state) = self.states.iter_mut().find(|state| state.id == id) {
                state.composite = true;
                state.concurrent_separator = separator;
            }
            id
        } else {
            format!("{base}.{sequence_name}")
        };
        let frame = self.scope_stack.last_mut().expect("scope frame");
        frame.current = region_id.clone();
        if !self.region_ids.iter().any(|id| id == &region_id) {
            self.region_ids.push(region_id.clone());
        }
        if !self.concurrent_now.iter().any(|id| id == &region_id) {
            self.concurrent_now.push(region_id.clone());
        }
        if let Some(state) = self.states.iter_mut().find(|state| state.id == base) {
            state
                .concurrent_regions
                .push(crate::diagram::state::ConcurrentRegion {
                    id: region_id,
                    source_line: self.current_line,
                });
        }
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

        if line.eq_ignore_ascii_case("left to right direction") {
            self.direction = StateLayoutDirection::LeftToRight;
            return Ok(());
        }
        if line.eq_ignore_ascii_case("top to bottom direction") {
            self.direction = StateLayoutDirection::TopToBottom;
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
            self.advance_concurrent_region(line);
            return Ok(());
        }

        // Closing brace of a composite block: pop the current scope.
        if line == "}" {
            self.scope_stack.pop();
            return Ok(());
        }

        if self.try_transition(line)? {
            return Ok(());
        }
        if self.try_state_decl(line)? {
            return Ok(());
        }
        if self.try_state_description(line)? {
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

    fn try_transition(&mut self, line: &str) -> Result<bool, ParseError> {
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
                r"^(\[[\w*]*\]|[\w.]+(?:\[H\*?\])?)\s*([-.<>][-.<>\[\]#,=\w]*[->])\s*(\[[\w*]*\]|[\w.]+(?:\[H\*?\])?)(?:\s*:\s*(.+))?$",
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let from = self.ensure_state(&caps[1], StateNameUse::Link)?;
            let to = self.ensure_state(&caps[3], StateNameUse::Link)?;
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
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn try_state_decl(&mut self, line: &str) -> Result<bool, ParseError> {
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
            let id = self.resolve_state_id(&raw_id, StateNameUse::Declaration)?;
            let parent = self.current_scope().map(String::from);
            let stereotype = caps.get(3).map(|m| m.as_str());
            // A trailing `{` opens a composite block; mark the state and push
            // its qualified id so nested declarations/transitions qualify
            // against it.
            let is_composite = line.trim_end().ends_with('{');

            // Java provenance: `CommandCreateState.getTypeFromStereotype`
            // recognizes reserved state-image stereotypes case-insensitively.
            let normalized_stereotype = stereotype.map(str::to_ascii_lowercase);
            let kind = match normalized_stereotype.as_deref() {
                Some("start") => StateKind::Initial,
                Some("end") => StateKind::Final,
                Some("choice") => StateKind::Choice,
                Some("fork") => StateKind::Fork,
                Some("join") => StateKind::Join,
                Some("history") => StateKind::History,
                Some("history*") => StateKind::DeepHistory,
                Some("entrypoint") => StateKind::EntryPoint,
                Some("exitpoint") => StateKind::ExitPoint,
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
            if let Some(fill) = fill.as_deref()
                && !state_hcolor_accepted(fill)
            {
                return Err(state_color_error(self.current_line));
            }
            if let Some(stroke) = stroke.as_deref()
                && !state_hcolor_accepted(stroke)
            {
                return Err(state_color_error(self.current_line));
            }

            if let Some(state) = self.states.iter_mut().find(|s| s.id == id) {
                state.label = label;
                state.kind = kind;
                state.composite |= is_composite;
                if state.decl_line.is_none() {
                    state.decl_line = Some(self.current_line);
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
                self.mute_to_state(&id);
                self.scope_stack.push(ScopeFrame {
                    base: id.clone(),
                    current: id,
                });
            }
            Ok(true)
        } else if let Some(caps) = RE_DESC.captures(line) {
            let label = caps
                .get(1)
                .map_or_else(|| caps[2].to_string(), |m| m.as_str().to_string());
            let id = self.resolve_state_id(&caps[2], StateNameUse::Declaration)?;
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
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn try_state_description(&mut self, line: &str) -> Result<bool, ParseError> {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)\s*:\s*(.+)$").unwrap());

        if let Some(caps) = RE.captures(line) {
            let id = self.ensure_state(&caps[1], StateNameUse::Unchecked)?;
            let desc = caps[2].trim().to_string();
            if let Some(state) = self.states.iter_mut().find(|s| s.id == id) {
                state.descriptions.push(desc);
                if state.decl_line.is_none() {
                    state.decl_line = Some(self.current_line);
                }
            }
            Ok(true)
        } else {
            Ok(false)
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
                    parent: None,
                    anchor_id: None,
                });
                self.scope_last_note();
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
                    parent: None,
                    anchor_id: None,
                });
                self.scope_last_note();
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
                        parent: None,
                        anchor_id: None,
                    });
                    self.scope_last_note();
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
                        parent: None,
                        anchor_id: None,
                    });
                    self.scope_last_note();
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

    #[test]
    fn note_anchors_resolve_in_pass_three_after_later_transitions() {
        // StateDiagramFactory: CommandFactoryNoteOnEntity runs in ParserPass.THREE.
        let lines = [
            "note right of Quay : wet",
            "state Dock {",
            "  note left of Crane : tall",
            "}",
            "Quay --> Dock",
            "Dock.Crane --> Quay",
        ]
        .map(String::from);
        let diagram = parse_state(&lines).unwrap();
        assert_eq!(diagram.notes[0].anchor_id.as_deref(), Some("Quay"));
        assert_eq!(diagram.notes[1].anchor_id.as_deref(), Some("Dock.Crane"));
        assert_eq!(diagram.notes[1].parent.as_deref(), Some("Dock"));
    }

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
            composite
                .concurrent_regions
                .iter()
                .map(|region| region.id.as_str())
                .collect::<Vec<_>>(),
            vec!["ParallelHarbor.CONC2"],
        );
        assert_eq!(
            d.states
                .iter()
                .find(|state| state.label == "Violet")
                .and_then(|state| state.parent.as_deref()),
            Some("ParallelHarbor.CONC2"),
        );
    }

    #[test]
    fn direction_commands_update_state_layout_direction() {
        let left_to_right = parse(
            "left to right direction\n\
             [*] --> FreshCopperRelay\n\
             FreshCopperRelay --> [*]",
        );
        assert_eq!(left_to_right.direction, StateLayoutDirection::LeftToRight);

        let reset = parse(
            "left to right direction\n\
             top to bottom direction\n\
             [*] --> FreshAzureRelay\n\
             FreshAzureRelay --> [*]",
        );
        assert_eq!(reset.direction, StateLayoutDirection::TopToBottom);
    }

    #[test]
    fn consecutive_separators_preserve_empty_concurrent_groups() {
        let d = parse(
            "state ParallelHarbor {\n\
             [*] --> Copper\n\
             --\n\
             --\n\
             [*] --> Violet\n\
             }",
        );

        let composite = d
            .states
            .iter()
            .find(|state| state.id == "ParallelHarbor")
            .unwrap();
        assert_eq!(
            composite
                .concurrent_regions
                .iter()
                .map(|region| (region.id.as_str(), region.source_line))
                .collect::<Vec<_>>(),
            vec![("ParallelHarbor.CONC2", 3), ("ParallelHarbor.CONC3", 4),],
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
    fn pass_one_nested_declaration_precedes_an_earlier_transition() {
        let d = parse(
            "OuterHarbor --> InnerRelay\n\
             state OuterHarbor {\n\
             state InnerRelay {\n\
             [*] --> CopperNode\n\
             CopperNode --> [*]\n\
             }\n\
             }",
        );

        assert_eq!(d.transitions[0].to, "OuterHarbor.InnerRelay");
        assert!(d.states.iter().any(|state| {
            state.id == "OuterHarbor.InnerRelay" && state.parent.as_deref() == Some("OuterHarbor")
        }));
        assert!(d.states.iter().any(|state| {
            state.id == "OuterHarbor.InnerRelay.CopperNode"
                && state.parent.as_deref() == Some("OuterHarbor.InnerRelay")
        }));
        assert!(!d.states.iter().any(|state| state.id == "InnerRelay"));
    }

    #[test]
    fn pass_one_description_precedes_an_earlier_transition() {
        let d = parse(
            "OuterHarbor --> DeferredRelay\n\
             state OuterHarbor {\n\
             DeferredRelay : declared after the transition\n\
             }",
        );

        assert_eq!(d.transitions[0].to, "OuterHarbor.DeferredRelay");
        let deferred = d
            .states
            .iter()
            .find(|state| state.id == "OuterHarbor.DeferredRelay")
            .unwrap();
        assert_eq!(deferred.descriptions, vec!["declared after the transition"]);
    }

    #[test]
    fn pass_two_transition_without_a_later_declaration_stays_at_root() {
        let d = parse(
            "AnchorNode --> ImplicitRelay\n\
             state UnrelatedShell {\n\
             [*] --> NestedNode\n\
             }",
        );

        assert_eq!(d.transitions[0].from, "AnchorNode");
        assert_eq!(d.transitions[0].to, "ImplicitRelay");
        assert!(
            d.states
                .iter()
                .any(|state| { state.id == "ImplicitRelay" && state.parent.is_none() })
        );
    }

    #[test]
    fn reused_root_quark_is_not_reparented_by_a_nested_declaration() {
        let d = parse(
            "state SharedRelay\n\
             state OuterHarbor {\n\
             state SharedRelay {\n\
             [*] --> NestedNode\n\
             }\n\
             }",
        );

        let shared = d
            .states
            .iter()
            .find(|state| state.id == "SharedRelay")
            .unwrap();
        assert!(shared.parent.is_none());
        assert!(shared.composite);
        assert!(d.states.iter().any(|state| {
            state.id == "SharedRelay.NestedNode" && state.parent.as_deref() == Some("SharedRelay")
        }));
    }

    #[test]
    fn predeclaration_skips_state_like_multiline_note_content() {
        let d = parse(
            "note right of VisibleNode\n\
             state PhantomShell {\n\
             PhantomNode --> PhantomExit\n\
             end note\n\
             state VisibleNode",
        );

        assert!(d.states.iter().any(|state| state.id == "VisibleNode"));
        assert!(!d.states.iter().any(|state| state.id == "PhantomShell"));
        assert!(!d.states.iter().any(|state| state.id == "PhantomNode"));
    }

    #[test]
    fn predeclaration_keeps_note_prefixed_state_descriptions() {
        let d = parse(
            "Anchor --> noteworthy\n\
             state Outer {\n\
             noteworthy : declared after the transition\n\
             }",
        );

        assert_eq!(d.transitions[0].to, "Outer.noteworthy");
        assert!(d.states.iter().any(|state| {
            state.id == "Outer.noteworthy"
                && state.descriptions == ["declared after the transition"]
        }));
        assert!(!d.states.iter().any(|state| state.id == "noteworthy"));
    }

    #[test]
    fn same_named_child_in_a_later_region_is_rejected() {
        // The first `SharedRelay` is the sole quark, parented by the composite
        // rather than by a concurrent group. The second link runs in
        // `CONC2`, so `checkConcurrentStateOk` refuses it. A second quark is
        // not created: `quarkInContextSafe` only mints a new child when
        // `countByName` is not 1.
        let error = parse_err(
            "state ParallelVault {\n\
             [*] --> SharedRelay\n\
             --\n\
             [*] --> SharedRelay\n\
             }",
        );
        assert_eq!(error.line, 4);
        assert_eq!(error.message, "The state SharedRelay cannot be used here.");
    }

    #[test]
    fn user_composite_whose_name_starts_with_conc_is_not_a_concurrent_region() {
        // GroupType.CONCURRENT_STATE is the group the separator creates.
        // CONCEPT is an ordinary composite whose name happens to share the
        // synthetic prefix.
        let diagram = parse(
            "state CONCEPT {\n\
             state g5ckQuern\n\
             }\n\
             [*] --> g5ckQuern\n",
        );
        assert!(diagram.states.iter().any(|state| state.id == "CONCEPT"));
        assert!(
            diagram
                .states
                .iter()
                .any(|state| state.id == "CONCEPT.g5ckQuern")
        );
        assert!(diagram.transitions.iter().any(|transition| {
            transition.from == "[*]" && transition.to == "CONCEPT.g5ckQuern"
        }));
    }

    #[test]
    fn separator_reuses_the_sole_quark_of_the_sequence_name() {
        // CW `region0_child_named_conc2`: the separator's sequence name is
        // CONC2, and that quark already exists inside House. Java reuses it
        // and mutes it, so Wing is its child and the later link names it.
        let diagram = parse(
            "state House {\n\
             state CONC2\n\
             --\n\
             state Wing\n\
             }\n\
             [*] --> CONC2\n",
        );
        let house = diagram
            .states
            .iter()
            .find(|state| state.id == "House")
            .unwrap();
        assert_eq!(
            house
                .concurrent_regions
                .iter()
                .map(|region| region.id.as_str())
                .collect::<Vec<_>>(),
            vec!["House.CONC2"]
        );
        let conc = diagram
            .states
            .iter()
            .find(|state| state.id == "House.CONC2")
            .unwrap();
        assert!(conc.composite);
        assert_eq!(
            diagram
                .states
                .iter()
                .filter(|state| state.id == "House.CONC2")
                .count(),
            1
        );
        assert!(diagram.states.iter().any(|state| {
            state.id == "House.CONC2.Wing" && state.parent.as_deref() == Some("House.CONC2")
        }));
        assert_eq!(diagram.transitions[0].to, "House.CONC2");
    }

    #[test]
    fn separator_beside_an_existing_conc_quark_rejects_the_outside_link() {
        // CV `region_user_conc_beside_separator`, body lines without @startuml.
        // The separator reuses the root CONC2 quark. Pier's parent is then a
        // concurrent group, and the root link is refused.
        let error = parse_err(
            "state CONC2 {\n\
             state Pier\n\
             }\n\
             state House {\n\
             state Porch\n\
             --\n\
             state Wing\n\
             }\n\
             [*] --> Pier\n\
             [*] --> Porch\n",
        );
        assert_eq!(error.line, 9);
        assert_eq!(error.message, "The state Pier cannot be used here.");
    }

    #[test]
    fn link_before_a_later_region_declaration_is_rejected() {
        // Pass one creates `g5ckMidden` inside the separator's group.
        // Pass two then refuses the earlier link, whose current group is
        // not that concurrent parent.
        let error = parse_err(
            "state g5ckQuern {\n\
             [*] --> g5ckMidden\n\
             --\n\
             state g5ckMidden\n\
             }",
        );
        assert_eq!(error.line, 2);
        assert_eq!(error.message, "The state g5ckMidden cannot be used here.");
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
            "state s1 <<START>>\nstate s2 <<End>>\nstate s3 <<Choice>>\n\
             state s4 <<FORK>>\nstate s5 <<join>>\nstate s6 <<HISTORY*>>",
        );
        assert_eq!(d.states[0].kind, StateKind::Initial);
        assert_eq!(d.states[0].stereotype, None);
        assert_eq!(d.states[1].kind, StateKind::Final);
        assert_eq!(d.states[1].stereotype, None);
        assert_eq!(d.states[2].kind, StateKind::Choice);
        assert_eq!(d.states[3].kind, StateKind::Fork);
        assert_eq!(d.states[4].kind, StateKind::Join);
        assert_eq!(d.states[5].kind, StateKind::DeepHistory);
    }

    #[test]
    fn predeclared_pseudo_keeps_first_reference_and_declaration_lines_distinct() {
        let d = parse("[*] --> decision\nstate decision <<choice>>");
        let decision = d
            .states
            .iter()
            .find(|state| state.id == "decision")
            .unwrap();

        assert_eq!(decision.source_line, 1);
        assert_eq!(decision.decl_line, Some(2));
        assert_eq!(decision.kind, StateKind::Choice);
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

    #[test]
    fn suffix_history_endpoints_enter_the_named_group() {
        // StateDiagram.getHistorical/getDeepHistory(location, idShort).
        let d = parse(
            "state Outer {\n\
             state Cellar {\n\
             [*] --> Cask\n\
             }\n\
             Porter --> Cellar[H]\n\
             }\n\
             Porter --> Attic[H*]",
        );
        let endpoints: Vec<_> = d.transitions.iter().map(|t| t.to.as_str()).collect();
        assert_eq!(
            endpoints,
            ["Outer.Cellar.Cask", "[H]Outer.Cellar", "[H*]Attic"]
        );
        for group in ["Outer.Cellar", "Attic"] {
            assert!(
                d.states.iter().any(|s| s.id == group && s.composite),
                "{group}"
            );
        }
    }

    fn parse_err(input: &str) -> ParseError {
        let lines: Vec<String> = input.lines().map(str::to_string).collect();
        parse_state(&lines).unwrap_err()
    }

    #[test]
    fn renamed_link_into_a_concurrent_region_is_rejected() {
        let error = parse_err(
            "state Kiln {\n\
             Ash --> Coal\n\
             --\n\
             Flame --> Glow\n\
             }\n\
             Stoke --> Flame",
        );
        assert_eq!(error.line, 6);
        assert_eq!(error.message, "The state Flame cannot be used here.");
    }

    #[test]
    fn renamed_link_from_a_region_to_a_region_zero_sibling_is_rejected() {
        let error = parse_err(
            "state Kiln {\n\
             Ash --> Coal\n\
             --\n\
             Flame --> Ash\n\
             }",
        );
        assert_eq!(error.line, 4);
        assert_eq!(error.message, "The state Ash cannot be used here.");
    }

    #[test]
    fn renamed_declaration_of_a_concurrent_state_is_rejected() {
        let error = parse_err(
            "state Kiln {\n\
             --\n\
             state Flame\n\
             }\n\
             state Flame",
        );
        assert_eq!(error.line, 5);
        assert_eq!(
            error.message,
            "The state Flame has been created in a concurrent state : it cannot be used here."
        );
    }

    #[test]
    fn renamed_description_of_a_concurrent_state_stays_on_that_quark() {
        let diagram = parse(
            "state Kiln {\n\
             --\n\
             state Flame\n\
             }\n\
             Flame : bright",
        );
        let flame = diagram
            .states
            .iter()
            .find(|state| state.label == "Flame")
            .unwrap();
        assert_eq!(flame.parent.as_deref(), Some("Kiln.CONC2"));
        assert_eq!(flame.descriptions, ["bright"]);
        assert_eq!(
            diagram
                .states
                .iter()
                .filter(|state| state.id.rsplit('.').next() == Some("Flame"))
                .count(),
            1
        );
    }

    #[test]
    fn renamed_link_to_the_current_composite_stays_legal() {
        let diagram = parse(
            "state Envelope {\n\
             state Head\n\
             --\n\
             state Mixed {\n\
             state Leaf\n\
             [*] --> Leaf\n\
             Mixed --> Leaf : keep\n\
             }\n\
             }",
        );
        assert!(diagram.transitions.iter().any(|transition| {
            transition.from == "Envelope.CONC2.Mixed"
                && transition.to == "Envelope.CONC2.Mixed.Leaf"
        }));
    }

    #[test]
    fn renamed_link_to_a_region_zero_child_stays_legal() {
        let diagram = parse(
            "state Kiln {\n\
             Ash --> Coal\n\
             --\n\
             Flame --> Glow\n\
             }\n\
             Stoke --> Ash",
        );
        assert!(
            diagram
                .transitions
                .iter()
                .any(|transition| { transition.from == "Stoke" && transition.to == "Kiln.Ash" })
        );
    }

    #[test]
    fn renamed_state_colors_follow_hcolorset() {
        let diagram = parse("state Boil #lightblue ##red\nstate Calm #red-blue\nBoil --> Calm");
        let boil = diagram
            .states
            .iter()
            .find(|state| state.id == "Boil")
            .unwrap();
        assert_eq!(boil.fill.as_deref(), Some("lightblue"));
        assert_eq!(boil.stroke.as_deref(), Some("red"));
        let calm = diagram
            .states
            .iter()
            .find(|state| state.id == "Calm")
            .unwrap();
        assert_eq!(calm.fill.as_deref(), Some("red-blue"));

        for (source, line) in [
            ("state Boil ##RebeccaPurple", 1),
            ("state Boil #NotAColor", 1),
            ("state Boil #GGGGGG", 1),
        ] {
            let error = parse_err(source);
            assert_eq!(error.line, line, "{source}");
            assert_eq!(error.message, "No such color");
        }
    }

    #[test]
    fn region_quark_counts_against_a_same_named_declaration() {
        // Pass one: the separator creates Granary.CONC2, and `state CONC2`
        // inside it becomes Granary.CONC2.CONC2. Pass two: two quarks are
        // named CONC2, so the separator takes Granary's own child again and
        // the root link creates a third quark instead of reusing either.
        let diagram = parse(
            "state Granary {\n\
             --\n\
             state CONC2\n\
             state Silo\n\
             }\n\
             [*] --> CONC2\n",
        );
        let granary = diagram.states.iter().find(|s| s.id == "Granary").unwrap();
        let regions: Vec<_> = granary
            .concurrent_regions
            .iter()
            .map(|r| r.id.as_str())
            .collect();
        assert_eq!(regions, vec!["Granary.CONC2"]);
        assert!(diagram.states.iter().any(|s| s.id == "Granary.CONC2.CONC2"));
        assert!(diagram.states.iter().any(|s| s.id == "Granary.CONC2.Silo"));
        assert_eq!(diagram.transitions[0].to, "CONC2");
    }

    #[test]
    fn later_separator_compares_the_sole_quark_with_the_owner() {
        // The third region is CONC4. Its only same-named quark is the leaf
        // declared inside it on pass one, which is not the owner, but the
        // region quark itself makes the count two.
        let diagram = parse(
            "state Mill {\n\
             --\n\
             --\n\
             --\n\
             state CONC4\n\
             }\n\
             [*] --> CONC4\n",
        );
        let mill = diagram.states.iter().find(|s| s.id == "Mill").unwrap();
        let regions: Vec<_> = mill
            .concurrent_regions
            .iter()
            .map(|r| r.id.as_str())
            .collect();
        assert_eq!(regions, vec!["Mill.CONC2", "Mill.CONC3", "Mill.CONC4"]);
        assert!(diagram.states.iter().any(|s| s.id == "Mill.CONC4.CONC4"));
        assert_eq!(diagram.transitions[0].to, "CONC4");
    }

    #[test]
    fn link_to_a_state_under_a_deeper_concurrent_ancestor_is_a_final_error() {
        // Loft's direct parent Attic is a plain state, so the link command
        // passes. Attic sits in Keep.CONC2, so checkFinalError refuses the
        // link. Java reports the sequence factory's line-1 syntax error.
        let error = parse_err(
            "state Keep {\n\
             --\n\
             state Attic {\n\
             state Loft\n\
             }\n\
             }\n\
             [*] --> Loft\n",
        );
        assert_eq!(error.line, 1);
        assert_eq!(error.message, "Syntax Error?");
    }

    #[test]
    fn link_inside_one_concurrent_ancestor_passes_the_final_check() {
        let diagram = parse(
            "state Keep {\n\
             --\n\
             state Attic {\n\
             state Loft\n\
             }\n\
             [*] --> Attic\n\
             }\n",
        );
        assert_eq!(diagram.transitions.len(), 1);
    }

    #[test]
    fn reopening_a_reused_region_mutes_it_back_to_a_state() {
        // The separator in Byre reuses the root composite CONC2 and mutes it
        // to a concurrent group. The later `state CONC2 {` mutes it back to
        // STATE, so on pass two the root link into its child Pen passes both
        // the link check and the final check.
        let diagram = parse(
            "state CONC2 {\n\
             state Pen\n\
             }\n\
             state Byre {\n\
             state Stall\n\
             --\n\
             state Rack\n\
             }\n\
             state CONC2 {\n\
             }\n\
             [*] --> Pen\n",
        );
        assert!(diagram.transitions.iter().any(|t| t.to == "CONC2.Pen"));
    }

    #[test]
    fn a_reused_region_left_concurrent_rejects_the_root_link() {
        let error = parse_err(
            "state CONC2 {\n\
             state Pen\n\
             }\n\
             state Byre {\n\
             state Stall\n\
             --\n\
             state Rack\n\
             }\n\
             [*] --> Pen\n",
        );
        assert_eq!(error.line, 9);
        assert_eq!(error.message, "The state Pen cannot be used here.");
    }
}
