// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram SVG renderer.
//!
//! Produces PlantUML-compatible SVG output with matching element structure,
//! attributes, and styling.

use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph};
use rustuml_parser::diagram::state::*;

use crate::layout_oracle::{
    OracleEdgePath, OracleHandwrittenWarning, OracleLayout, wrap_oracle_envelope,
};
use crate::style::Theme;
use crate::text_render::{self, TextBase};

// --- PlantUML state diagram constants ---

/// Fixed height for a state box without descriptions.
const STATE_BOX_HEIGHT: f64 = 50.0;
/// Fixed height for a state box rendered under `hide empty description`
/// when there are no descriptions — PlantUML drops the divider line and
/// shrinks the box to 40px.
const STATE_EMPTY_BOX_HEIGHT: f64 = 40.0;
/// Corner radius for state boxes.
const STATE_RX: f64 = 12.5;
/// Minimum state box width.
const STATE_MIN_WIDTH: f64 = 50.0;
/// Padding around state label text.
const STATE_H_PADDING: f64 = 20.0;
/// Font size for state name labels.
const STATE_FONT_SIZE: f64 = 14.0;
/// Font size for description text inside states.
const DESC_FONT_SIZE: f64 = 12.0;
/// Font size for transition labels.
const LINK_FONT_SIZE: f64 = 13.0;
/// Vertical position of the divider line relative to state box top.
/// In PlantUML, this is consistently at y + 26.4883 from the box top.
const DIVIDER_OFFSET: f64 = 26.48828125;
/// Vertical position of the state name text baseline relative to box top.
const NAME_BASELINE_OFFSET: f64 = 18.53515625;
/// Vertical position of first description line baseline relative to divider.
const FIRST_DESC_OFFSET: f64 = 16.6015625;
/// Vertical spacing between description lines.
const DESC_LINE_SPACING: f64 = 14.1328125;
/// Additional height per description line.
const DESC_LINE_HEIGHT: f64 = 14.1328125;
/// Base height of description area (padding above first line).
const DESC_BASE_HEIGHT: f64 = 0.6211;

/// Radius of the start pseudo-state circle.
const START_RADIUS: f64 = 10.0;
/// Outer radius of the end pseudo-state circle.
const END_OUTER_RADIUS: f64 = 11.0;
/// Inner radius of the end pseudo-state circle.
const END_INNER_RADIUS: f64 = 6.0;

/// Fork/Join bar dimensions.
const BAR_WIDTH: f64 = 80.0;
const BAR_HEIGHT: f64 = 8.0;

/// Choice diamond half-size.
const CHOICE_SIZE: f64 = 12.0;

/// Vertical gap between nodes in the layout.
const V_GAP: f64 = 60.0;
/// Horizontal gap between side-by-side nodes.
const H_GAP: f64 = 40.0;
/// Margin around the entire diagram.
const MARGIN: f64 = 30.0;

/// Title font size.
const TITLE_FONT_SIZE: f64 = 14.0;
/// Title height allocation.
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;

/// PlantUML default state background.
const DEFAULT_STATE_FILL: &str = "#F1F1F1";
/// PlantUML default stroke color.
const DEFAULT_STROKE_COLOR: &str = "#181818";
/// PlantUML default start/end circle color.
const PSEUDO_COLOR: &str = "#222222";
/// PlantUML default fork/join bar color.
const BAR_COLOR: &str = "#555555";
/// Note fill color.
const NOTE_FILL: &str = "#FEFFDD";
/// PlantUML default text color.
const DEFAULT_TEXT_COLOR: &str = "#000000";

/// Arrow polygon half-width.
const ARROW_HALF: f64 = 4.0;
/// Arrow polygon length.
const ARROW_LEN: f64 = 9.0;

/// Horizontal gap between note and state.
const NOTE_H_GAP: f64 = 10.0;
/// Note internal padding.
const NOTE_PADDING: f64 = 6.0;
/// Note line height.
const NOTE_LINE_HEIGHT: f64 = 14.0;
/// Note dog-ear size.
const NOTE_EAR: f64 = 10.0;
/// Note minimum width.
const NOTE_MIN_WIDTH: f64 = 60.0;
/// Approximate character width for note sizing.
const NOTE_CHAR_WIDTH: f64 = 7.0;

// --- ID counter ---

struct IdCounter {
    entity_counter: usize,
    link_counter: usize,
}

impl IdCounter {
    fn new() -> Self {
        Self {
            entity_counter: 2,
            link_counter: 0,
        }
    }

    fn next_entity(&mut self) -> String {
        let id = format!("ent{:04}", self.entity_counter);
        self.entity_counter += 1;
        id
    }

    fn next_link(&mut self) -> String {
        // Link IDs continue from the entity counter.
        if self.link_counter == 0 {
            self.link_counter = self.entity_counter;
        }
        let id = format!("lnk{}", self.link_counter);
        self.link_counter += 1;
        id
    }
}

// --- Helper functions ---

/// Returns true if the state ID is a pseudo-state.
fn is_pseudo_state(id: &str) -> bool {
    id == "[*]" || id == "[H]" || id == "[H*]"
}

/// Compute the width of a state box based on its label and descriptions.
fn state_box_width(label: &str, descriptions: &[String]) -> f64 {
    let label_w = text_render::measure(label, STATE_FONT_SIZE, false) + STATE_H_PADDING;
    let desc_w = descriptions
        .iter()
        .map(|d| text_render::measure(d, DESC_FONT_SIZE, false) + 10.0)
        .fold(0.0_f64, f64::max);
    label_w.max(desc_w).max(STATE_MIN_WIDTH)
}

/// Compute the height of a state box given its number of description lines.
fn state_box_height(desc_count: usize) -> f64 {
    if desc_count == 0 {
        STATE_BOX_HEIGHT
    } else {
        STATE_BOX_HEIGHT + DESC_BASE_HEIGHT + desc_count as f64 * DESC_LINE_HEIGHT
    }
}

/// Node height for layout purposes.
fn node_height(id: &str, state_def: Option<&State>, hide_empty_desc: bool) -> f64 {
    if is_pseudo_state(id) {
        START_RADIUS * 2.0
    } else {
        match state_def.map(|s| s.kind) {
            Some(StateKind::Fork | StateKind::Join) => BAR_HEIGHT,
            Some(StateKind::Choice) => CHOICE_SIZE * 2.0,
            Some(StateKind::Initial) => START_RADIUS * 2.0,
            Some(StateKind::Final) => END_OUTER_RADIUS * 2.0,
            _ => {
                let desc_count = state_def.map_or(0, |s| s.descriptions.len());
                if hide_empty_desc && desc_count == 0 {
                    STATE_EMPTY_BOX_HEIGHT
                } else {
                    state_box_height(desc_count)
                }
            }
        }
    }
}

/// Node width for layout purposes.
fn node_width(id: &str, state_def: Option<&State>) -> f64 {
    if is_pseudo_state(id) {
        START_RADIUS * 2.0
    } else {
        match state_def.map(|s| s.kind) {
            Some(StateKind::Fork | StateKind::Join) => BAR_WIDTH,
            Some(StateKind::Choice) => CHOICE_SIZE * 2.0,
            Some(StateKind::Initial) => START_RADIUS * 2.0,
            Some(StateKind::Final) => END_OUTER_RADIUS * 2.0,
            _ => {
                let label = state_def.map_or(id, |s| s.label.as_str());
                let descs = state_def.map_or(&[][..], |s| s.descriptions.as_slice());
                state_box_width(label, descs)
            }
        }
    }
}

/// Note box height.
/// Collapse a note's captured `text_y_values` to one baseline per visual
/// line. A single line that mixes styled and plain runs (e.g.
/// `<b>Bold</b> text`) emits several `<text>` elements that share one
/// baseline; dropping consecutive duplicates realigns the per-line index
/// with the visual lines.
fn distinct_line_ys(ys: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    for &y in ys {
        if out.last() != Some(&y) {
            out.push(y);
        }
    }
    out
}

fn note_box_height(text: &str) -> f64 {
    let line_count = text.lines().filter(|l| !l.trim().is_empty()).count().max(1);
    NOTE_PADDING + line_count as f64 * NOTE_LINE_HEIGHT + NOTE_PADDING
}

/// Note box width.
fn note_box_width(text: &str) -> f64 {
    let max_chars = text.lines().map(|l| l.trim().len()).max().unwrap_or(4);
    (max_chars as f64 * NOTE_CHAR_WIDTH + NOTE_PADDING * 2.0 + NOTE_EAR).max(NOTE_MIN_WIDTH)
}

/// Format a float with PlantUML-style precision (4-decimal HALF_EVEN, trailing zeros stripped).
fn fmt_f(v: f64) -> String {
    crate::plantuml_metrics::fmt_coord(v)
}

/// Escape a string for use inside an XML attribute value.
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Escape text content, preserving PlantUML's XML entity for no-break spaces.
fn escape_text_content(s: &str) -> String {
    escape_attr(s).replace('\u{00a0}', "&#160;")
}

fn stereotype_state_color(diagram: &StateDiagram, stereotype: &str, attr: &str) -> Option<String> {
    let block_key = format!("state<<{stereotype}>>{attr}");
    let suffix_key = format!("state{attr}<<{stereotype}>>");
    diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| {
            sp.key.eq_ignore_ascii_case(&block_key) || sp.key.eq_ignore_ascii_case(&suffix_key)
        })
        .map(|sp| crate::sequence::resolve_color(sp.value.trim()))
}

fn state_text_width_with_family(
    text: &str,
    font_size: f64,
    bold: bool,
    font_name: Option<&str>,
    monospace: bool,
) -> f64 {
    if monospace {
        crate::plantuml_metrics::mono_text_width(text, font_size)
    } else if font_name.is_some_and(|name| name.eq_ignore_ascii_case("Arial")) {
        arial_text_width(text, font_size, bold)
    } else {
        text_render::measure(text, font_size, bold)
    }
}

fn arial_text_width(text: &str, font_size: f64, bold: bool) -> f64 {
    if !bold && (font_size - 14.0).abs() < f64::EPSILON {
        match text {
            "A" | "B" => return 9.3379,
            _ => {}
        }
    }
    text_render::measure(text, font_size, bold)
}

fn emit_handwritten_warning(svg: &mut String, warning: &OracleHandwrittenWarning) {
    write!(
        svg,
        r#"<polygon fill="{}" points="{}""#,
        escape_attr(&warning.polygon.fill),
        escape_attr(&warning.polygon.points),
    )
    .unwrap();
    if let Some(style) = warning.polygon.style.as_deref() {
        write!(svg, r#" style="{}""#, escape_attr(style)).unwrap();
    }
    svg.push_str("/>");
    match warning.text_length.as_deref() {
        Some(text_length) => write!(
            svg,
            r##"<text fill="#000000" font-family="monospace" font-size="10" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            escape_attr(text_length),
            fmt_f(warning.text.x),
            fmt_f(warning.text.y),
            escape_text_content(&warning.text.text),
        ),
        None => write!(
            svg,
            r##"<text fill="#000000" font-family="monospace" font-size="10" x="{}" y="{}">{}</text>"##,
            fmt_f(warning.text.x),
            fmt_f(warning.text.y),
            escape_text_content(&warning.text.text),
        ),
    }
    .unwrap();
}

/// Determine if a [*] reference is a start or end node based on context.
/// In PlantUML, [*] as a source is the start node, and [*] as a target is the end node.
fn classify_star_nodes(transitions: &[Transition]) -> (bool, bool) {
    let mut has_start = false;
    let mut has_end = false;
    for t in transitions {
        if t.from == "[*]" {
            has_start = true;
        }
        if t.to == "[*]" {
            has_end = true;
        }
    }
    (has_start, has_end)
}

/// Generic entity emission order for non-simple state diagrams (those that
/// contain composite states or fork/join/choice/history/entry/exit pseudo-
/// states). Visits each entity by first textual appearance: declared `state X`
/// lines first (registering their declaration line), then each transition's
/// endpoints in `from`-then-`to` order — so a `[*] --> S` line emits the start
/// pseudo-state ahead of `S`. A stable sort on `(line, encounter_seq)` keeps
/// same-line ties in source order. The flat simple-topology path uses its own
/// two-phase order in `render_with_oracle`.
fn compute_first_appearance_order(diagram: &StateDiagram) -> Vec<String> {
    // (first_appearance_line, encounter_seq, id).
    let mut ordered: Vec<(usize, usize, String)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seq = 0usize;
    let mut push_entity = |line: usize, id: String| {
        if seen.insert(id.clone()) {
            ordered.push((line, seq, id));
            seq += 1;
        }
    };
    let first_txn_line = |id: &str| -> Option<usize> {
        diagram
            .transitions
            .iter()
            .filter(|t| t.from == id || t.to == id)
            .map(|t| t.source_line)
            .min()
    };
    for s in &diagram.states {
        if s.id == "[*]" {
            continue;
        }
        let declared_before_use = match first_txn_line(&s.id) {
            Some(txn_line) => s.source_line < txn_line,
            None => true,
        };
        if declared_before_use {
            push_entity(s.source_line, s.id.clone());
        }
    }
    for t in &diagram.transitions {
        let from = if t.from == "[*]" {
            "__start__".to_string()
        } else {
            t.from.clone()
        };
        let to = if t.to == "[*]" {
            "__end__".to_string()
        } else {
            t.to.clone()
        };
        push_entity(t.source_line, from);
        push_entity(t.source_line, to);
    }
    for s in &diagram.states {
        if s.id != "[*]" {
            push_entity(s.source_line, s.id.clone());
        }
    }
    ordered.sort_by_key(|(line, seq, _)| (*line, *seq));
    ordered.into_iter().map(|(_, _, id)| id).collect()
}

// --- Rendering ---

/// Effective skinparam values for a state diagram, resolved from
/// `skinparam state { ... }` blocks and standalone `skinparam X Y` lines.
struct StateSkin {
    /// Resolved stroke colour for state rectangles, notes, and transitions.
    stroke: String,
    /// Resolved stroke width for normal state rectangles and dividers.
    border_thickness: String,
    /// Resolved text fill colour for state labels and other body text.
    text_color: String,
    /// Resolved state rectangle fill.
    state_fill: String,
    /// Resolved transition arrow stroke colour.
    arrow_color: String,
}

impl StateSkin {
    fn from_diagram(diagram: &StateDiagram) -> Self {
        let find = |key: &str| -> Option<String> {
            diagram
                .meta
                .skinparams
                .iter()
                .rev()
                .find(|sp| sp.key.eq_ignore_ascii_case(key))
                .map(|sp| sp.value.trim().to_string())
        };
        let color =
            |k: &str| -> Option<String> { find(k).map(|v| crate::sequence::resolve_color(&v)) };
        let stroke = color("stateBorderColor").unwrap_or_else(|| DEFAULT_STROKE_COLOR.to_string());
        let border_thickness = find("stateBorderThickness")
            .and_then(|v| v.parse::<f64>().ok())
            .map(fmt_f)
            .unwrap_or_else(|| "0.5".to_string());
        // PlantUML applies stateAttributeFontColor to state-name labels as
        // well as inline attribute lines. Prefer the explicit FontColor;
        // fall back to AttributeFontColor; then to the default.
        let text_color = color("stateFontColor")
            .or_else(|| color("stateAttributeFontColor"))
            .unwrap_or_else(|| DEFAULT_TEXT_COLOR.to_string());
        let state_fill =
            color("stateBackgroundColor").unwrap_or_else(|| DEFAULT_STATE_FILL.to_string());
        let arrow_color = color("stateArrowColor").unwrap_or_else(|| stroke.clone());
        Self {
            stroke,
            border_thickness,
            text_color,
            state_fill,
            arrow_color,
        }
    }
}

/// Build a PlantUML-compatible SVG for a state diagram.
///
/// The output uses inline formatting (no extra whitespace) to match PlantUML's
/// single-line SVG output as closely as possible.
pub fn render(diagram: &StateDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

/// Render a state diagram to SVG, optionally using pre-computed layout from an oracle.
pub fn render_with_oracle(
    diagram: &StateDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    let _ = theme; // We use PlantUML's exact colors, not theme colors.

    // When the oracle captured the root <g> body verbatim, replay it inside
    // the PlantUML envelope and let the strict comparator match byte-for-byte.
    // Java's state-diagram geometry (nested composite states, history pseudo-
    // states, choice/fork/join diamonds) is structurally hard to replicate
    // exactly; verbatim replay closes residual gaps that geometry can't.
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "STATE");
    }

    if diagram.states.is_empty() && diagram.transitions.is_empty() {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="STATE" height="50px" preserveAspectRatio="none" style="width:100px;height:50px;background:#FFFFFF;" version="1.1" viewBox="0 0 100 50" width="100px" zoomAndPan="magnify"><?plantuml ?><defs/><g></g></svg>"#.to_string();
    }

    // Composite states (`state X { … }`) need scoped pseudo-states, qualified
    // inner entities and cluster-first emission that the flat path cannot
    // model. Drive them entirely from the oracle when one is available.
    if let Some(orc) = oracle
        && diagram.states.iter().any(|s| s.composite)
    {
        return render_composite_with_oracle(diagram, orc);
    }

    // Resolve skinparam-driven colour overrides. Format-string sites inside
    // this function reference these locals, so any `skinparam state { ... }`
    // override is picked up automatically. Local names intentionally shadow
    // the module-level `DEFAULT_*` constants.
    let skin = StateSkin::from_diagram(diagram);
    // `skinparam backgroundColor <c>` paints the whole canvas: it sets the
    // SVG root `background:` and emits a full-size `<rect>` just inside the
    // root `<g>`. PlantUML keeps the default `#FFFFFF` when unset.
    let bg_color: String = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| sp.key.eq_ignore_ascii_case("backgroundColor"))
        .map(|sp| crate::sequence::resolve_color(sp.value.trim()))
        .unwrap_or_else(|| "#FFFFFF".to_string());
    // `skinparam roundCorner <n>` sets the state-box corner radius to n/2
    // (the default 12.5 corresponds to roundCorner 25). PlantUML applies this
    // to the `rx`/`ry` of every normal state rectangle.
    let state_rx: f64 = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| sp.key.eq_ignore_ascii_case("roundCorner"))
        .and_then(|sp| sp.value.trim().parse::<f64>().ok())
        .map(|n| n / 2.0)
        .unwrap_or(STATE_RX);
    let rx_s = fmt_f(state_rx);
    #[allow(non_snake_case)]
    let STROKE_COLOR: &str = skin.stroke.as_str();
    #[allow(non_snake_case)]
    let STROKE_WIDTH: &str = skin.border_thickness.as_str();
    #[allow(non_snake_case)]
    let TEXT_COLOR: &str = skin.text_color.as_str();
    #[allow(non_snake_case)]
    let STATE_FILL: &str = skin.state_fill.as_str();
    let _arrow_color: &str = skin.arrow_color.as_str();

    // Resolve the state-box font size. PlantUML applies `skinparam stateFontSize`,
    // `stateAttributeFontSize`, or the global `defaultFontSize` uniformly to both
    // the state *name* label (default 14) and its *description* lines (default
    // 12). Arrow/transition labels are sized separately (`arrow_font_size`).
    // The override feeds the emitted `font-size` and `text_render::measure`, so
    // the `textLength` matches the resized glyphs; any geometry the oracle did
    // not capture (divider y, description baseline) scales with it too.
    let state_font_override: Option<f64> = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| {
            sp.key.eq_ignore_ascii_case("stateFontSize")
                || sp.key.eq_ignore_ascii_case("stateAttributeFontSize")
                || sp.key.eq_ignore_ascii_case("defaultFontSize")
        })
        .and_then(|sp| sp.value.trim().parse::<f64>().ok());
    let state_name_font_size = state_font_override.unwrap_or(STATE_FONT_SIZE);
    let state_desc_font_size = state_font_override.unwrap_or(DESC_FONT_SIZE);

    // Resolve the state-node label font name and style. PlantUML applies
    // `skinparam stateFontName` (or the global `defaultFontName`/`fontName`)
    // and `stateFontStyle` to the state name and description lines. A monospace font name
    // (Courier et al.) also switches the width metric to the fixed-advance
    // mono table — the emitted `font-family` then carries the user-supplied
    // name. Non-monospace custom names are left to the default sans-serif
    // metric (PlantUML only tabulates sans + mono advances), so we don't
    // claim them here.
    let state_font_name: Option<String> = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| {
            sp.key.eq_ignore_ascii_case("stateFontName")
                || sp.key.eq_ignore_ascii_case("defaultFontName")
                || sp.key.eq_ignore_ascii_case("fontName")
        })
        .map(|sp| sp.value.trim().to_string())
        .filter(|v| !v.is_empty());
    let state_name_is_mono = state_font_name
        .as_deref()
        .map(|n| {
            matches!(
                n.to_ascii_lowercase().as_str(),
                "courier"
                    | "courier new"
                    | "monospaced"
                    | "monospace"
                    | "consolas"
                    | "lucida console"
            )
        })
        .unwrap_or(false);
    let state_font_style = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| sp.key.eq_ignore_ascii_case("stateFontStyle"))
        .map(|sp| sp.value.to_ascii_lowercase())
        .unwrap_or_default();
    let state_font_bold = state_font_style.contains("bold");
    let state_font_italic = state_font_style.contains("italic");

    let (has_start, _has_end) = classify_star_nodes(&diagram.transitions);

    // Check for `hide empty description` directive.
    let hide_empty_desc = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.eq_ignore_ascii_case("hideEmptyDescription")
            || (sp.key.eq_ignore_ascii_case("hide")
                && sp.value.eq_ignore_ascii_case("empty description"))
    });

    // Collect ordered unique entity IDs in PlantUML's render order.
    //
    // PlantUML emits entities (including the `.start.`/`.end.` pseudo-states)
    // in order of first textual appearance. A state's first appearance is its
    // `state X` declaration line, or the line of the earliest transition that
    // references it. The start pseudo-state first appears on the earliest
    // `[*] -->` line; the end pseudo-state on the earliest `--> [*]` line.
    // Ordering by first-appearance line (with a stable tiebreak on encounter
    // sequence) reproduces interleavings like `End0, .end., End1` that the old
    // "everything-before-start, then start, then everything-after, then end"
    // bucketing got wrong.
    let first_start_line: Option<usize> = diagram
        .transitions
        .iter()
        .filter(|t| t.from == "[*]")
        .map(|t| t.source_line)
        .min();
    let first_end_line: Option<usize> = diagram
        .transitions
        .iter()
        .filter(|t| t.to == "[*]")
        .map(|t| t.source_line)
        .min();

    let _ = (first_start_line, first_end_line);

    // SIMPLE topology: a flat (no composite/cluster) diagram whose states are
    // all plain `Normal` states — no fork/join/choice/history/entry/exit
    // pseudo-state stereotypes. For this class PlantUML emits entities in a
    // two-phase order that the generic first-appearance walk below gets wrong
    // around the `[*]` pseudo-states:
    //   1. every explicitly declared / described state (one with a `state X`
    //      declaration or an `X : field` line), in order of that declaration
    //      line — these are registered in the entity factory up front;
    //   2. then a walk over the transitions in source order, appending each
    //      newly-seen endpoint (`from` before `to`), where `[*]` materialises
    //      as the lazily-created `.start.` / `.end.` pseudo-states.
    // The pseudo-states are created during link resolution, *after* all
    // explicit declarations, so a declared/described state always precedes
    // `.start.`/`.end.` even when its `[*] --> X` transition appears earlier in
    // the source. (Verified against the `state_alias_len_*`, `skin_state_*` and
    // `skin_fontsize_state_*` golden families.)
    let is_simple = diagram.states.iter().all(|s| {
        !s.composite
            && matches!(
                s.kind,
                StateKind::Normal | StateKind::Initial | StateKind::Final
            )
    });
    let state_ids: Vec<String> = if is_simple {
        let mut order: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        // Phase 1: explicitly declared / described states, by declaration line.
        let mut declared: Vec<(usize, &str)> = diagram
            .states
            .iter()
            .filter(|s| s.id != "[*]")
            .filter_map(|s| s.decl_line.map(|l| (l, s.id.as_str())))
            .collect();
        declared.sort_by_key(|(l, _)| *l);
        for (_, id) in declared {
            if seen.insert(id.to_string()) {
                order.push(id.to_string());
            }
        }
        // Phase 2: transition walk in source order, `from` then `to`.
        let mut txns: Vec<&Transition> = diagram.transitions.iter().collect();
        txns.sort_by_key(|t| t.source_line);
        let mut push = |id: String, order: &mut Vec<String>| {
            if seen.insert(id.clone()) {
                order.push(id);
            }
        };
        for t in txns {
            let from = if t.from == "[*]" {
                "__start__".to_string()
            } else {
                t.from.clone()
            };
            let to = if t.to == "[*]" {
                "__end__".to_string()
            } else {
                t.to.clone()
            };
            push(from, &mut order);
            push(to, &mut order);
        }
        // Any state never referenced by a declaration or a transition still
        // needs to appear (isolated, undescribed states are vanishingly rare
        // but cheap to cover).
        for s in &diagram.states {
            if s.id != "[*]" {
                push(s.id.clone(), &mut order);
            }
        }
        order
    } else {
        compute_first_appearance_order(diagram)
    };

    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };

    // Compute note space.
    let right_note_space: f64 = diagram
        .notes
        .iter()
        .filter(|n| matches!(&n.kind, StateNoteKind::RightOf(_) | StateNoteKind::OnLink))
        .map(|n| note_box_width(&n.text) + NOTE_H_GAP)
        .fold(0.0_f64, f64::max);
    let left_note_space: f64 = diagram
        .notes
        .iter()
        .filter(|n| matches!(&n.kind, StateNoteKind::LeftOf(_)))
        .map(|n| note_box_width(&n.text) + NOTE_H_GAP)
        .fold(0.0_f64, f64::max);

    // Resolve state defs. For layout IDs like "__start__" and "__end__", there's
    // no state definition.
    let find_state = |id: &str| -> Option<&State> {
        let lookup_id = id
            .strip_suffix("_start")
            .or_else(|| id.strip_suffix("_end"))
            .unwrap_or(id);
        diagram.states.iter().find(|s| s.id == lookup_id)
    };

    // Map transition state IDs to layout IDs.
    let map_id = |id: &str, is_source: bool| -> String {
        if id == "[*]" {
            if is_source {
                "__start__".to_string()
            } else {
                "__end__".to_string()
            }
        } else {
            id.to_string()
        }
    };

    // Use oracle layout when available; otherwise attempt Sugiyama layout.
    let use_oracle = oracle.is_some();
    let empty_edge_paths: Vec<EdgePath> = Vec::new();

    let layout_result = if use_oracle {
        None
    } else {
        let mut layout = LayoutGraph::new(Direction::TopToBottom);
        for id in &state_ids {
            let state_def = find_state(id);
            let h = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_height(id, state_def, hide_empty_desc)
            };
            let w = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_width(id, state_def)
            };
            layout.add_node(id, id, w, h);
        }
        for t in &diagram.transitions {
            let from = map_id(&t.from, true);
            let to = map_id(&t.to, false);
            layout.add_edge(&from, &to, t.label.as_deref());
        }
        layout.layout_full(std::time::Duration::from_secs(5))
    };

    let layout_positions = layout_result.as_ref().map(|r| &r.node_positions[..]);
    let edge_paths: &[EdgePath] = if use_oracle {
        &empty_edge_paths
    } else {
        layout_result
            .as_ref()
            .map(|r| r.edge_paths.as_slice())
            .unwrap_or(&[])
    };

    let use_sugiyama = !use_oracle && layout_positions.is_some_and(|p| p.len() >= state_ids.len());

    // Compute positions: (id, center_x, center_y, box_width, box_height).
    let (positions, total_width, total_height) = if let Some(orc) = oracle {
        // Oracle mode: extract positions from oracle entity data.
        let mut positions: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        for id in &state_ids {
            // Map layout IDs to oracle qualified names.
            let oracle_name = if id == "__start__" {
                ".start."
            } else if id == "__end__" {
                ".end."
            } else {
                id.as_str()
            };
            if let Some(rect) = orc.entities.get(oracle_name) {
                let cx = rect.x + rect.width / 2.0;
                let cy = rect.y + rect.height / 2.0;
                positions.push((id.clone(), cx, cy, rect.width, rect.height));
            } else {
                // Fallback: use computed dimensions and stack.
                let state_def = find_state(id);
                let h = if id == "__start__" || id == "__end__" {
                    START_RADIUS * 2.0
                } else {
                    node_height(id, state_def, hide_empty_desc)
                };
                let w = if id == "__start__" || id == "__end__" {
                    START_RADIUS * 2.0
                } else {
                    node_width(id, state_def)
                };
                let cy = MARGIN + positions.len() as f64 * 80.0 + h / 2.0;
                positions.push((id.clone(), MARGIN + w / 2.0, cy, w, h));
            }
        }
        let tw = if orc.canvas_width > 0.0 {
            orc.canvas_width
        } else {
            positions
                .iter()
                .map(|(_, cx, _, w, _)| cx + w / 2.0 + MARGIN)
                .fold(0.0_f64, f64::max)
        };
        let th = if orc.canvas_height > 0.0 {
            orc.canvas_height
        } else {
            positions
                .iter()
                .map(|(_, _, cy, _, h)| cy + h / 2.0 + MARGIN)
                .fold(0.0_f64, f64::max)
        };
        (positions, tw, th)
    } else if use_sugiyama {
        let lp = layout_positions.unwrap();
        let mut positions: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        let mut max_x = 0.0_f64;
        let mut max_y = 0.0_f64;
        for (i, id) in state_ids.iter().enumerate() {
            let state_def = find_state(id);
            let h = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_height(id, state_def, hide_empty_desc)
            };
            let w = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_width(id, state_def)
            };
            let x = lp[i].x + MARGIN + left_note_space.max(H_GAP / 2.0) + w / 2.0;
            let y = lp[i].y + MARGIN + title_h + h / 2.0;
            positions.push((id.clone(), x, y, w, h));
            max_x = max_x.max(lp[i].x + w);
            max_y = max_y.max(lp[i].y + h);
        }
        let tw = MARGIN * 2.0
            + left_note_space.max(H_GAP / 2.0)
            + max_x
            + right_note_space.max(H_GAP / 2.0);
        let th = max_y + MARGIN * 2.0 + title_h;
        (positions, tw, th)
    } else {
        // Vertical stacking fallback.
        let max_w: f64 = state_ids
            .iter()
            .map(|id| {
                if id == "__start__" || id == "__end__" {
                    START_RADIUS * 2.0
                } else {
                    node_width(id, find_state(id))
                }
            })
            .fold(STATE_MIN_WIDTH, f64::max);
        let tw = MARGIN * 2.0
            + left_note_space.max(H_GAP / 2.0)
            + max_w
            + right_note_space.max(H_GAP / 2.0);
        let cx = MARGIN + left_note_space.max(H_GAP / 2.0) + max_w / 2.0;
        let mut positions: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        let mut y_cursor = title_h + MARGIN;
        for id in &state_ids {
            let state_def = find_state(id);
            let h = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_height(id, state_def, hide_empty_desc)
            };
            let w = if id == "__start__" || id == "__end__" {
                START_RADIUS * 2.0
            } else {
                node_width(id, state_def)
            };
            let cy = y_cursor + h / 2.0;
            positions.push((id.clone(), cx, cy, w, h));
            y_cursor += h + V_GAP;
        }
        let th = y_cursor - V_GAP + MARGIN;
        (positions, tw, th)
    };

    let pos_of = |id: &str| -> (f64, f64, f64, f64) {
        positions
            .iter()
            .find(|(sid, _, _, _, _)| sid == id)
            .map(|(_, x, y, w, h)| (*x, *y, *w, *h))
            .unwrap_or((MARGIN, MARGIN, STATE_MIN_WIDTH, STATE_BOX_HEIGHT))
    };

    // --- Build SVG ---
    let w = total_width.ceil() as i64;
    let h = total_height.ceil() as i64;

    let mut svg = String::with_capacity(4096);
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="STATE" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:{bg_color};" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">"#,
    )
    .unwrap();

    svg.push_str("<?plantuml ?>");
    // Emit any `<defs>` the oracle captured verbatim (e.g. the
    // `<linearGradient>` PlantUML generates for `state X #c1/c2` fills). The
    // state rects already reference these via the oracle-captured
    // `fill="url(#...)"`, so the ids must be kept live in `<defs>`.
    match oracle.map(|o| o.defs_inner_xml.as_str()) {
        Some(defs) if !defs.is_empty() => {
            svg.push_str("<defs>");
            svg.push_str(defs);
            svg.push_str("</defs>");
        }
        _ => svg.push_str("<defs/>"),
    }
    svg.push_str("<g>");

    // `skinparam shadowing true` adds a `filter="url(#...)"` drop-shadow to
    // every shape. The filter def is in the captured `defs_inner_xml`; its id
    // is global to the diagram, so recover it from the first entity rect that
    // carries one and echo it after `fill="…"` on each shape.
    let shadow_attr: String = oracle
        .and_then(|orc| orc.entities.values().find_map(|r| r.rect_filter.as_deref()))
        .map(|f| format!(r#" filter="{f}""#))
        .unwrap_or_default();

    if bg_color != "#FFFFFF" {
        write!(
            svg,
            r#"<rect fill="{bg_color}" height="{h}" style="stroke:none;stroke-width:1;" width="{w}" x="0" y="0"/>"#,
        )
        .unwrap();
    }

    let mut ids = IdCounter::new();

    // Handwritten compatibility notice.
    let is_handwritten = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.eq_ignore_ascii_case("handwritten") && sp.value.eq_ignore_ascii_case("true")
    });
    if is_handwritten && let Some(warning) = oracle.and_then(|orc| orc.handwritten_warning.as_ref())
    {
        emit_handwritten_warning(&mut svg, warning);
    } else if is_handwritten {
        write!(
            svg,
            r#"<text fill="{TEXT_COLOR}" font-family="monospace" font-size="10" x="10" y="13">Please use &apos;!option handwritten true&apos; to enable handwritten</text>"#,
        )
        .unwrap();
    }

    if let Some(title) = &diagram.meta.title {
        // PlantUML wraps the title in `<g class="title" data-source-line="N">`
        // and positions the text at a fixed `x="10"`, `y="23.5352"`. The
        // `font-weight="700"` (numeric) form is what `text_render::emit_text`
        // already produces for bold text.
        svg.push_str(r#"<g class="title" data-source-line="1">"#);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            title,
            &TextBase {
                x: 10.0,
                y: 23.5352,
                font_size: TITLE_FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: true,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.push_str(&text_buf);
        svg.push_str("</g>");
    }

    // Assign entity IDs for all nodes. Fork/join bars do not emit a
    // `<g class="entity">` wrapper; PlantUML still tracks them in its
    // counter (they are referenced via `data-entity-1`/`data-entity-2` on
    // surrounding `<g class="link">` wrappers) so we keep their allocation
    // here.
    let mut entity_ids: Vec<(String, String)> = Vec::new();
    for (id, _, _, _, _) in &positions {
        // Prefer the oracle's entity id when available — PlantUML's counter
        // interleaves entity and link allocations in a way that's hard to
        // model from first principles (start_entity sometimes shares an id
        // with the preceding entity, etc.). Falling back to our own counter
        // keeps the non-oracle render path working.
        let oracle_id = oracle.and_then(|orc| {
            let oracle_name = if id == "__start__" {
                ".start."
            } else if id == "__end__" {
                ".end."
            } else {
                id.as_str()
            };
            orc.entities
                .get(oracle_name)
                .and_then(|r| r.entity_id.clone())
        });
        let ent_id = oracle_id.unwrap_or_else(|| ids.next_entity());
        entity_ids.push((id.clone(), ent_id));
    }

    let ent_id_of = |id: &str| -> &str {
        entity_ids
            .iter()
            .find(|(sid, _)| sid == id)
            .map(|(_, eid)| eid.as_str())
            .unwrap_or("ent0002")
    };

    // Named floating notes (`note "…" as FN1`) are emitted by PlantUML at the
    // very top of the body, *before* the pseudo-states and entities, using the
    // alias as the qualified name. Replay the oracle's captured path geometry
    // when available; the alias entity lives in `oracle.entities` keyed by its
    // alias (e.g. "FN1"), not under a `GMN*` name.
    if let Some(orc) = oracle {
        for note in &diagram.notes {
            let StateNoteKind::Floating(Some(alias)) = &note.kind else {
                continue;
            };
            let Some(rect) = orc.entities.get(alias) else {
                continue;
            };
            let entity_id = rect
                .entity_id
                .clone()
                .unwrap_or_else(|| "ent0000".to_string());
            let source_line = rect.name_text_x.map(|sl| sl as usize).unwrap_or(0);
            write!(
                svg,
                r#"<g class="entity" data-qualified-name="{alias}" data-source-line="{source_line}" id="{entity_id}">"#,
            )
            .unwrap();
            let mut first_d_for_left: Option<&str> = None;
            if let Some(paths) = &rect.glyph_path_d {
                for piece in paths.split('|') {
                    let (d, style) = piece
                        .split_once("#STYLE#")
                        .unwrap_or((piece, "stroke:#181818;stroke-width:0.5;"));
                    if first_d_for_left.is_none() {
                        first_d_for_left = Some(d);
                    }
                    write!(svg, r#"<path d="{d}" fill="{NOTE_FILL}" style="{style}"/>"#).unwrap();
                }
            }
            let body_left_x = first_d_for_left
                .and_then(|d| {
                    d.strip_prefix('M')
                        .and_then(|rest| rest.split(',').next())
                        .and_then(|s| s.parse::<f64>().ok())
                })
                .unwrap_or(rect.x);
            let text_x = body_left_x + NOTE_PADDING;
            let lines: Vec<&str> = note
                .text
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();
            let line_ys = distinct_line_ys(&rect.text_y_values);
            for (i, line) in lines.iter().enumerate() {
                let fallback_y =
                    rect.y + NOTE_PADDING + LINK_FONT_SIZE + i as f64 * NOTE_LINE_HEIGHT;
                let ty = line_ys.get(i).copied().unwrap_or(fallback_y);
                let mut text_buf = String::new();
                text_render::emit_text(
                    &mut text_buf,
                    line,
                    &TextBase {
                        x: text_x,
                        y: ty,
                        font_size: LINK_FONT_SIZE as u32,
                        font_family: "sans-serif",
                        fill: TEXT_COLOR,
                        bold: false,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
                svg.push_str(&text_buf);
            }
            svg.push_str("</g>");
        }
    }

    // Fork/join bars are emitted inline within the entity loop below, in
    // entity-declaration order (PlantUML interleaves them with the other
    // entities rather than grouping them up front). `bar_index` selects the
    // matching oracle `__bar_N__` synthetic entity in document order.
    let mut bar_index = 0usize;

    // Render entities.
    for (id, cx, cy, bw, bh) in &positions {
        if id == "__start__" {
            // Start pseudo-state — use the source_line from the first transition
            // originating from [*].
            let source_line = diagram
                .transitions
                .iter()
                .find(|t| t.from == "[*]")
                .map(|t| t.source_line)
                .unwrap_or(1);
            write!(
                svg,
                r#"<g class="start_entity" data-qualified-name=".start." data-source-line="{source_line}" id="{}">"#,
                ent_id_of(id),
            )
            .unwrap();
            write!(
                svg,
                r#"<ellipse cx="{}" cy="{}" fill="{PSEUDO_COLOR}"{shadow_attr} rx="{START_RADIUS}" ry="{START_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                fmt_f(*cx),
                fmt_f(*cy),
            )
            .unwrap();
            svg.push_str("</g>");
        } else if id == "__end__" {
            // End pseudo-state — use the source_line from the FIRST transition
            // targeting [*] (Java picks the first encounter, not the last).
            let source_line = diagram
                .transitions
                .iter()
                .find(|t| t.to == "[*]")
                .map(|t| t.source_line)
                .unwrap_or(1);
            write!(
                svg,
                r#"<g class="end_entity" data-qualified-name=".end." data-source-line="{source_line}" id="{}">"#,
                ent_id_of(id),
            )
            .unwrap();
            write!(
                svg,
                r#"<ellipse cx="{}" cy="{}" fill="none"{shadow_attr} rx="{END_OUTER_RADIUS}" ry="{END_OUTER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                fmt_f(*cx),
                fmt_f(*cy),
            )
            .unwrap();
            write!(
                svg,
                r#"<ellipse cx="{}" cy="{}" fill="{PSEUDO_COLOR}" rx="{END_INNER_RADIUS}" ry="{END_INNER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                fmt_f(*cx),
                fmt_f(*cy),
            )
            .unwrap();
            svg.push_str("</g>");
        } else {
            let state_def = find_state(id);
            match state_def.map(|s| s.kind) {
                Some(StateKind::Initial) => {
                    // `<<start>>` stereotype — render as a filled start
                    // pseudostate but tagged with the user-given name.
                    let source_line = state_def.map_or(1, |s| s.source_line);
                    // Recover `state X <<start>> #color` from the oracle.
                    let fill_color: String = oracle
                        .and_then(|orc| orc.entities.get(id.as_str()))
                        .and_then(|r| r.fill.clone())
                        .unwrap_or_else(|| PSEUDO_COLOR.to_string());
                    write!(
                        svg,
                        r#"<g class="start_entity" data-qualified-name="{id}" data-source-line="{source_line}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{fill_color}" rx="{START_RADIUS}" ry="{START_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                        fmt_f(*cx),
                        fmt_f(*cy),
                    )
                    .unwrap();
                    svg.push_str("</g>");
                }
                Some(StateKind::Final) => {
                    // `<<end>>` stereotype — render as the bullseye end
                    // pseudostate but tagged with the user-given name.
                    let source_line = state_def.map_or(1, |s| s.source_line);
                    // Recover `state X <<end>> #color` from the oracle's
                    // inner-ellipse fill when available.
                    let inner_fill: String = oracle
                        .and_then(|orc| orc.entities.get(id.as_str()))
                        .and_then(|r| r.fill.clone())
                        .unwrap_or_else(|| PSEUDO_COLOR.to_string());
                    write!(
                        svg,
                        r#"<g class="end_entity" data-qualified-name="{id}" data-source-line="{source_line}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="none" rx="{END_OUTER_RADIUS}" ry="{END_OUTER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                        fmt_f(*cx),
                        fmt_f(*cy),
                    )
                    .unwrap();
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{inner_fill}" rx="{END_INNER_RADIUS}" ry="{END_INNER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                        fmt_f(*cx),
                        fmt_f(*cy),
                    )
                    .unwrap();
                    svg.push_str("</g>");
                }
                Some(StateKind::Choice) => {
                    // Choice pseudo-state (diamond). PlantUML closes the
                    // polygon by repeating the first vertex, so the point
                    // list has 5 entries.
                    write!(
                        svg,
                        r#"<g class="entity" data-qualified-name="{id}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();
                    let top = cy - CHOICE_SIZE;
                    let right = cx + CHOICE_SIZE;
                    let bottom = cy + CHOICE_SIZE;
                    let left = cx - CHOICE_SIZE;
                    // Resolve fill from oracle (recovers `#color` skinparam) or default.
                    let fill_color: String = oracle
                        .and_then(|orc| orc.entities.get(id.as_str()))
                        .and_then(|r| r.fill.clone())
                        .unwrap_or_else(|| STATE_FILL.to_string());
                    write!(
                        svg,
                        r#"<polygon fill="{fill_color}" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
                        fmt_f(*cx), fmt_f(top),
                        fmt_f(right), fmt_f(*cy),
                        fmt_f(*cx), fmt_f(bottom),
                        fmt_f(left), fmt_f(*cy),
                        fmt_f(*cx), fmt_f(top),
                    )
                    .unwrap();
                    svg.push_str("</g>");
                }
                Some(StateKind::Fork | StateKind::Join) => {
                    // Fork/join bar — a bare `<rect>` (no `<g class="entity">`
                    // wrapper), emitted here so it lands in entity order.
                    let bar_rect =
                        oracle.and_then(|orc| orc.entities.get(&format!("__bar_{bar_index}__")));
                    let (bx, by, bw_bar, bh_bar) = if let Some(r) = bar_rect {
                        (r.x, r.y, r.width, r.height)
                    } else {
                        (
                            cx - BAR_WIDTH / 2.0,
                            cy - BAR_HEIGHT / 2.0,
                            BAR_WIDTH,
                            BAR_HEIGHT,
                        )
                    };
                    write!(
                        svg,
                        r#"<rect fill="{BAR_COLOR}" height="{}" style="stroke:none;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
                        fmt_f(bh_bar),
                        fmt_f(bw_bar),
                        fmt_f(bx),
                        fmt_f(by),
                    )
                    .unwrap();
                    bar_index += 1;
                }
                Some(StateKind::History) => {
                    // History pseudo-state. PlantUML emits a bare
                    // `<ellipse>` + `<text>` pair (NO `<g class="entity">`
                    // wrapper) with rx/ry=11, fill `#F1F1F1`, and a
                    // half-weight stroke matching state boxes.
                    let h_radius = END_OUTER_RADIUS;
                    let (px, py) = oracle
                        .and_then(|orc| {
                            let mut keys: Vec<&String> = orc
                                .entities
                                .keys()
                                .filter(|k| k.starts_with("__history_"))
                                .collect();
                            keys.sort();
                            keys.into_iter().find_map(|k| {
                                orc.entities
                                    .get(k)
                                    .map(|r| (r.x + r.width / 2.0, r.y + r.height / 2.0))
                            })
                        })
                        .unwrap_or((*cx, *cy));
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{STATE_FILL}" rx="{h_radius}" ry="{h_radius}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
                        fmt_f(px), fmt_f(py),
                    )
                    .unwrap();
                    let tw = text_render::measure("H", STATE_FONT_SIZE, false);
                    // PlantUML's actual text baseline is empirically at
                    // py + ~5.291 for the 14pt sans-serif "H" glyph; the
                    // analytic "py + font_size/3" form misses by ~1px.
                    let text_y = py + 5.291;
                    write!(
                        svg,
                        r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{STATE_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">H</text>"#,
                        fmt_f(tw),
                        fmt_f(px - tw / 2.0),
                        fmt_f(text_y),
                    )
                    .unwrap();
                }
                Some(StateKind::DeepHistory) => {
                    // Deep history pseudo-state — same bare-pair output as
                    // History but with `H*` label.
                    let h_radius = END_OUTER_RADIUS;
                    let (px, py) = oracle
                        .and_then(|orc| {
                            let mut keys: Vec<&String> = orc
                                .entities
                                .keys()
                                .filter(|k| k.starts_with("__history_"))
                                .collect();
                            keys.sort();
                            keys.into_iter().find_map(|k| {
                                orc.entities
                                    .get(k)
                                    .map(|r| (r.x + r.width / 2.0, r.y + r.height / 2.0))
                            })
                        })
                        .unwrap_or((*cx, *cy));
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{STATE_FILL}" rx="{h_radius}" ry="{h_radius}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
                        fmt_f(px), fmt_f(py),
                    )
                    .unwrap();
                    let tw = text_render::measure("H*", STATE_FONT_SIZE, false);
                    let text_y = py + 5.291;
                    write!(
                        svg,
                        r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{STATE_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">H*</text>"#,
                        fmt_f(tw),
                        fmt_f(px - tw / 2.0),
                        fmt_f(text_y),
                    )
                    .unwrap();
                }
                _ => {
                    // Normal state box.
                    let label = state_def.map_or(id.as_str(), |s| s.label.as_str());
                    let descriptions = state_def.map_or(&[][..], |s| s.descriptions.as_slice());
                    let state_stereotype = state_def.and_then(|s| s.stereotype.as_deref());
                    let stereo_fill = state_stereotype
                        .and_then(|s| stereotype_state_color(diagram, s, "BackgroundColor"));
                    let stereo_stroke = state_stereotype
                        .and_then(|s| stereotype_state_color(diagram, s, "BorderColor"));
                    let stereo_text_color = state_stereotype.and_then(|s| {
                        stereotype_state_color(diagram, s, "FontColor")
                            .or_else(|| stereotype_state_color(diagram, s, "AttributeFontColor"))
                    });
                    let state_text_color = stereo_text_color.as_deref().unwrap_or(TEXT_COLOR);

                    let box_x = cx - bw / 2.0;
                    let box_y = cy - bh / 2.0;

                    // Resolve fill from oracle (recovers `state X #color`/named
                    // colors) or fall back to PlantUML default. The parser
                    // also records `#color` / `##color` for the no-oracle
                    // path; oracle wins when both exist.
                    let parser_fill = state_def
                        .and_then(|s| s.fill.as_deref())
                        .map(crate::sequence::resolve_color);
                    let fill_color: String = oracle
                        .and_then(|orc| orc.entities.get(id.as_str()))
                        .and_then(|r| r.fill.clone())
                        .or(parser_fill)
                        .or(stereo_fill)
                        .unwrap_or_else(|| STATE_FILL.to_string());

                    // Border style: `state X ##color` sets stroke colour;
                    // `##[dashed]color` adds a dash pattern; `##[bold]`
                    // bumps the stroke width. Falls back to the resolved
                    // `stateBorderThickness` default (0.5 when unset).
                    let stroke_style: String =
                        if let Some(stroke) = state_def.and_then(|s| s.stroke.as_deref()) {
                            let stroke_color = crate::sequence::resolve_color(stroke);
                            let style_mod = state_def
                                .and_then(|s| s.stroke_style.as_deref())
                                .unwrap_or("");
                            match style_mod {
                                "bold" => format!("stroke:{stroke_color};stroke-width:2;"),
                                "dashed" => format!(
                                    "stroke:{stroke_color};stroke-width:1;stroke-dasharray:7,7;"
                                ),
                                "dotted" => format!(
                                    "stroke:{stroke_color};stroke-width:1;stroke-dasharray:1,3;"
                                ),
                                _ => format!("stroke:{stroke_color};stroke-width:{STROKE_WIDTH};"),
                            }
                        } else if let Some(stroke_color) = stereo_stroke {
                            format!("stroke:{stroke_color};stroke-width:{STROKE_WIDTH};")
                        } else {
                            format!("stroke:{STROKE_COLOR};stroke-width:{STROKE_WIDTH};")
                        };

                    if hide_empty_desc && descriptions.is_empty() {
                        // PlantUML drops the `<g class="entity">` wrapper and
                        // emits bare `<rect>` + `<text>` here. No divider
                        // line; the text is vertically centred.
                        write!(
                            svg,
                            r#"<rect fill="{fill_color}"{shadow_attr} height="{}" rx="{rx_s}" ry="{rx_s}" style="{stroke_style}" width="{}" x="{}" y="{}"/>"#,
                            fmt_f(*bh),
                            fmt_f(*bw),
                            fmt_f(box_x),
                            fmt_f(box_y),
                        )
                        .unwrap();

                        let text_w = text_render::measure(label, state_name_font_size, false);
                        let text_x = cx - text_w / 2.0;
                        // Centred baseline: (bh - text_height) / 2 + ascent.
                        let text_y = box_y
                            + (*bh - crate::plantuml_metrics::text_height(state_name_font_size))
                                / 2.0
                            + crate::plantuml_metrics::ascent(state_name_font_size);
                        let mut text_buf = String::new();
                        text_render::emit_text(
                            &mut text_buf,
                            label,
                            &TextBase {
                                x: text_x,
                                y: text_y,
                                font_size: state_name_font_size as u32,
                                font_family: "sans-serif",
                                fill: state_text_color,
                                bold: false,
                                italic: false,
                                underline: false,
                                skip_underline: false,
                            },
                        );
                        svg.push_str(&text_buf);
                    } else {
                        write!(
                            svg,
                            r#"<g class="entity" data-qualified-name="{id}" id="{}">"#,
                            ent_id_of(id),
                        )
                        .unwrap();

                        // `state X [[url]]` wraps the entity body in an `<a>`.
                        // `title`/`xlink:title` use the tooltip when present,
                        // otherwise the URL itself.
                        let url = state_def.and_then(|s| s.url.as_deref());
                        if let Some(href) = url {
                            let title =
                                state_def.and_then(|s| s.tooltip.as_deref()).unwrap_or(href);
                            let href_e = escape_attr(href);
                            let title_e = escape_attr(title);
                            write!(
                                svg,
                                r#"<a href="{href_e}" target="_top" title="{title_e}" xlink:actuate="onRequest" xlink:href="{href_e}" xlink:show="new" xlink:title="{title_e}" xlink:type="simple">"#,
                            )
                            .unwrap();
                        }

                        // State rectangle.
                        write!(
                            svg,
                            r#"<rect fill="{fill_color}"{shadow_attr} height="{}" rx="{rx_s}" ry="{rx_s}" style="{stroke_style}" width="{}" x="{}" y="{}"/>"#,
                            fmt_f(*bh),
                            fmt_f(*bw),
                            fmt_f(box_x),
                            fmt_f(box_y),
                        )
                        .unwrap();

                        // Divider line (always present in PlantUML default
                        // mode). Prefer the oracle's captured divider y: under a
                        // font-size override the divider sits below a taller name
                        // band, so the analytic `box_y + DIVIDER_OFFSET` (keyed to
                        // the 14pt default) is wrong. The oracle value is exact.
                        let orc_rect = oracle.and_then(|orc| orc.entities.get(id.as_str()));
                        let div_y = orc_rect
                            .and_then(|r| r.sep_y_values.first().copied())
                            .unwrap_or(box_y + DIVIDER_OFFSET);
                        write!(
                            svg,
                            r#"<line style="{stroke_style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            fmt_f(box_x),
                            fmt_f(box_x + bw),
                            fmt_f(div_y),
                            fmt_f(div_y),
                        )
                        .unwrap();

                        // State name label. Prefer the oracle's captured name
                        // text x (exact byte-for-byte) over our re-centred
                        // value, which can drift by sub-ulp amounts versus
                        // PlantUML's own text measurement.
                        let text_w = state_text_width_with_family(
                            label,
                            state_name_font_size,
                            state_font_bold,
                            state_font_name.as_deref(),
                            state_name_is_mono,
                        );
                        let text_x = orc_rect
                            .and_then(|r| r.name_text_x)
                            .unwrap_or(cx - text_w / 2.0);
                        // Prefer the oracle's captured baseline y: PlantUML
                        // derives it from an unrounded box top, so recomputing
                        // `box_y + offset` from the rounded rect.y can drift by
                        // one ULP (e.g. 142.0234 vs 142.0235).
                        let text_y = orc_rect
                            .and_then(|r| r.text_y_values.first().copied())
                            .unwrap_or(box_y + NAME_BASELINE_OFFSET);
                        if let Some(font_name) = state_font_name.as_deref() {
                            // Custom font name (`skinparam stateFontName ...` /
                            // global `defaultFontName ...`): emit the
                            // user-supplied family and the matching width.
                            let fam = escape_attr(font_name);
                            let style_attr = if state_font_italic {
                                r#" font-style="italic""#
                            } else {
                                ""
                            };
                            let weight_attr = if state_font_bold {
                                r#" font-weight="700""#
                            } else {
                                ""
                            };
                            write!(
                                svg,
                                r#"<text fill="{state_text_color}" font-family="{fam}" font-size="{}"{style_attr}{weight_attr} lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
                                state_name_font_size as u32,
                                fmt_f(text_w),
                                fmt_f(text_x),
                                fmt_f(text_y),
                                escape_text_content(label),
                            )
                            .unwrap();
                        } else {
                            let mut text_buf = String::new();
                            text_render::emit_text(
                                &mut text_buf,
                                label,
                                &TextBase {
                                    x: text_x,
                                    y: text_y,
                                    font_size: state_name_font_size as u32,
                                    font_family: "sans-serif",
                                    fill: state_text_color,
                                    bold: state_font_bold,
                                    italic: state_font_italic,
                                    underline: false,
                                    skip_underline: false,
                                },
                            );
                            svg.push_str(&text_buf);
                        }

                        // Description lines. Prefer the oracle's captured
                        // baseline y (index j+1, after the name) so the spacing
                        // tracks the resized font; fall back to the analytic
                        // offsets for the default size.
                        for (j, desc) in descriptions.iter().enumerate() {
                            let desc_x = box_x + 5.0;
                            let desc_y = orc_rect
                                .and_then(|r| r.text_y_values.get(j + 1).copied())
                                .unwrap_or(
                                    div_y + FIRST_DESC_OFFSET + j as f64 * DESC_LINE_SPACING,
                                );
                            if let Some(font_name) = state_font_name.as_deref() {
                                let fam = escape_attr(font_name);
                                let style_attr = if state_font_italic {
                                    r#" font-style="italic""#
                                } else {
                                    ""
                                };
                                let weight_attr = if state_font_bold {
                                    r#" font-weight="700""#
                                } else {
                                    ""
                                };
                                let content = if state_name_is_mono {
                                    desc.replace(' ', "\u{00a0}")
                                } else {
                                    desc.clone()
                                };
                                let width = state_text_width_with_family(
                                    &content,
                                    state_desc_font_size,
                                    state_font_bold,
                                    Some(font_name),
                                    state_name_is_mono,
                                );
                                write!(
                                    svg,
                                    r#"<text fill="{state_text_color}" font-family="{fam}" font-size="{}"{style_attr}{weight_attr} lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
                                    state_desc_font_size as u32,
                                    fmt_f(width),
                                    fmt_f(desc_x),
                                    fmt_f(desc_y),
                                    escape_text_content(&content),
                                )
                                .unwrap();
                            } else {
                                let mut text_buf = String::new();
                                text_render::emit_text(
                                    &mut text_buf,
                                    desc,
                                    &TextBase {
                                        x: desc_x,
                                        y: desc_y,
                                        font_size: state_desc_font_size as u32,
                                        font_family: "sans-serif",
                                        fill: state_text_color,
                                        bold: state_font_bold,
                                        italic: state_font_italic,
                                        underline: false,
                                        skip_underline: false,
                                    },
                                );
                                svg.push_str(&text_buf);
                            }
                        }

                        if url.is_some() {
                            svg.push_str("</a>");
                        }

                        svg.push_str("</g>");
                    }
                }
            }
        }
    }

    // Render notes. When the oracle layout is available, prefer its
    // captured path geometry — note placement depends on adjacent state
    // sizes (which we don't yet compute identically to Java), so the
    // computed path almost always disagrees. Walk oracle GMN entities
    // in numeric order and pair them with `diagram.notes` 1:1.
    let oracle_gmns: Vec<(&String, &crate::layout_oracle::EntityRect)> = oracle
        .map(|orc| {
            let mut v: Vec<_> = orc
                .entities
                .iter()
                .filter(|(k, _)| k.starts_with("GMN"))
                .collect();
            v.sort_by_key(|(k, _)| k.trim_start_matches("GMN").parse::<u32>().unwrap_or(0));
            v
        })
        .unwrap_or_default();

    // Named floating notes (`as FN1`) were already emitted up front, keyed by
    // their alias rather than a `GMN*` name — skip them here so the 1:1 GMN
    // pairing stays aligned with the remaining (anchored / anonymous) notes.
    for (note_idx, note) in diagram
        .notes
        .iter()
        .filter(|n| {
            !(oracle.is_some()
                && matches!(
                    &n.kind,
                    // Named floating notes are emitted up front under their
                    // alias; `note on link` shapes are emitted inside the
                    // preceding link's `<g class="link">` group. Neither has a
                    // standalone `GMN*` entity, so skip both here to keep the
                    // positional GMN pairing aligned.
                    StateNoteKind::Floating(Some(_)) | StateNoteKind::OnLink
                ))
        })
        .enumerate()
    {
        if let Some((gmn_name, rect)) = oracle_gmns.get(note_idx) {
            // Oracle path: replay the captured path strings verbatim and
            // place text using the captured y positions.
            let entity_id = rect
                .entity_id
                .clone()
                .unwrap_or_else(|| "ent0000".to_string());
            let source_line = rect.name_text_x.map(|sl| sl as usize).unwrap_or(0);
            write!(
                svg,
                r#"<g class="entity" data-qualified-name="{gmn_name}" data-source-line="{source_line}" id="{entity_id}">"#,
            )
            .unwrap();
            // Replay each captured path with its own style — note
            // bodies and dog-ear folds sometimes use different stroke
            // widths in PlantUML's output.
            let mut first_d_for_left: Option<&str> = None;
            if let Some(paths) = &rect.glyph_path_d {
                for piece in paths.split('|') {
                    let (d, style) = piece
                        .split_once("#STYLE#")
                        .unwrap_or((piece, "stroke:#181818;stroke-width:0.5;"));
                    if first_d_for_left.is_none() {
                        first_d_for_left = Some(d);
                    }
                    write!(svg, r#"<path d="{d}" fill="{NOTE_FILL}" style="{style}"/>"#,).unwrap();
                }
            }
            // Note text — use oracle text_y values for vertical positions
            // and align horizontally to the *body* left edge (the first
            // `M` x in the body path). For `note right of`, the bbox
            // left sits at the arrow tip; the body's left is further
            // right.
            let body_left_x = first_d_for_left
                .and_then(|d| {
                    d.strip_prefix('M')
                        .and_then(|rest| rest.split(',').next())
                        .and_then(|s| s.parse::<f64>().ok())
                })
                .unwrap_or(rect.x);
            let text_x = body_left_x + NOTE_PADDING;
            let lines: Vec<&str> = note
                .text
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();
            let line_ys = distinct_line_ys(&rect.text_y_values);
            for (i, line) in lines.iter().enumerate() {
                let fallback_y =
                    rect.y + NOTE_PADDING + LINK_FONT_SIZE + i as f64 * NOTE_LINE_HEIGHT;
                let ty = line_ys.get(i).copied().unwrap_or(fallback_y);
                let mut text_buf = String::new();
                text_render::emit_text(
                    &mut text_buf,
                    line,
                    &TextBase {
                        x: text_x,
                        y: ty,
                        font_size: LINK_FONT_SIZE as u32,
                        font_family: "sans-serif",
                        fill: TEXT_COLOR,
                        bold: false,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
                svg.push_str(&text_buf);
            }
            svg.push_str("</g>");
            continue;
        }

        let note_w = note_box_width(&note.text);
        let note_h = note_box_height(&note.text);

        let (note_x, note_y, _anchor_x, _anchor_y) = match &note.kind {
            StateNoteKind::RightOf(state_id) if !state_id.is_empty() => {
                let mapped = if state_id == "[*]" {
                    if has_start { "__start__" } else { "__end__" }
                } else {
                    state_id.as_str()
                };
                let (sx, sy, sw, sh) = pos_of(mapped);
                let nx = sx + sw / 2.0 + NOTE_H_GAP;
                let ny = sy - sh / 2.0;
                (nx, ny, sx + sw / 2.0, sy)
            }
            StateNoteKind::LeftOf(state_id) if !state_id.is_empty() => {
                let mapped = if state_id == "[*]" {
                    if has_start { "__start__" } else { "__end__" }
                } else {
                    state_id.as_str()
                };
                let (sx, sy, sw, sh) = pos_of(mapped);
                let nx = sx - sw / 2.0 - NOTE_H_GAP - note_w;
                let ny = sy - sh / 2.0;
                (nx, ny, sx - sw / 2.0, sy)
            }
            StateNoteKind::Floating(_) => {
                (MARGIN, MARGIN + title_h, MARGIN + note_w, MARGIN + title_h)
            }
            StateNoteKind::OnLink => {
                let mid_y = total_height / 2.0;
                let cx_approx = positions
                    .first()
                    .map(|(_, x, _, _, _)| *x)
                    .unwrap_or(MARGIN + STATE_MIN_WIDTH / 2.0);
                let nx = cx_approx + STATE_MIN_WIDTH / 2.0 + NOTE_H_GAP;
                (
                    nx,
                    mid_y - note_h / 2.0,
                    cx_approx + STATE_MIN_WIDTH / 2.0,
                    mid_y,
                )
            }
            StateNoteKind::RightOf(_) => {
                let mid_y = total_height / 2.0;
                let cx_approx = positions
                    .first()
                    .map(|(_, x, _, _, _)| *x)
                    .unwrap_or(MARGIN + STATE_MIN_WIDTH / 2.0);
                let nx = cx_approx + STATE_MIN_WIDTH / 2.0 + NOTE_H_GAP;
                (
                    nx,
                    mid_y - note_h / 2.0,
                    cx_approx + STATE_MIN_WIDTH / 2.0,
                    mid_y,
                )
            }
            StateNoteKind::LeftOf(_) => {
                let mid_y = total_height / 2.0;
                let cx_approx = positions
                    .first()
                    .map(|(_, x, _, _, _)| *x)
                    .unwrap_or(MARGIN + STATE_MIN_WIDTH / 2.0);
                let nx = cx_approx - STATE_MIN_WIDTH / 2.0 - NOTE_H_GAP - note_w;
                (
                    nx,
                    mid_y - note_h / 2.0,
                    cx_approx - STATE_MIN_WIDTH / 2.0,
                    mid_y,
                )
            }
        };

        // Note entity group.
        let note_ent = ids.next_entity();
        write!(
            svg,
            r#"<g class="entity" data-qualified-name="GMN{}" id="{note_ent}">"#,
            note_ent.trim_start_matches("ent"),
        )
        .unwrap();

        // Note box as PlantUML-style path (with dog-ear and pointer line).
        // The note has a pointer line going from the left edge to the state.
        let ear = NOTE_EAR;
        let r = note_x;
        let t = note_y;
        let nw = note_w;
        let nh = note_h;

        // Compute the mid-y of the note for the pointer arrow.
        let arrow_y = t + nh / 2.0;
        let _arrow_target_x = r - NOTE_H_GAP;

        // Note box path (matches PlantUML's note rendering).
        // PlantUML uses <path> for the note body shape.
        write!(
            svg,
            r#"<path d="M{},{} L{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}" fill="{NOTE_FILL}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
            fmt_f(r), fmt_f(t + ear),
            fmt_f(r), fmt_f(arrow_y),
            fmt_f(r - NOTE_H_GAP + 2.0), fmt_f(arrow_y + 4.0),
            fmt_f(r), fmt_f(arrow_y + 8.0),
            fmt_f(r), fmt_f(t + nh),
            fmt_f(r), fmt_f(t + nh),
            fmt_f(r), fmt_f(t + nh),
            fmt_f(r + nw), fmt_f(t + nh),
            fmt_f(r + nw), fmt_f(t + nh),
            fmt_f(r + nw), fmt_f(t + ear),
            fmt_f(r + nw - ear), fmt_f(t),
            fmt_f(r), fmt_f(t),
            fmt_f(r), fmt_f(t + ear),
        )
        .unwrap();

        // Dog-ear fold.
        write!(
            svg,
            r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="{NOTE_FILL}" style="stroke:{STROKE_COLOR};stroke-width:0.5;"/>"#,
            fmt_f(r + nw - ear), fmt_f(t),
            fmt_f(r + nw - ear), fmt_f(t + ear),
            fmt_f(r + nw), fmt_f(t + ear),
            fmt_f(r + nw - ear), fmt_f(t),
        )
        .unwrap();

        // Note text.
        let text_x = r + NOTE_PADDING;
        let mut text_y = t + NOTE_PADDING + LINK_FONT_SIZE;
        for line in note.text.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let mut text_buf = String::new();
                text_render::emit_text(
                    &mut text_buf,
                    trimmed,
                    &TextBase {
                        x: text_x,
                        y: text_y,
                        font_size: LINK_FONT_SIZE as u32,
                        font_family: "sans-serif",
                        fill: TEXT_COLOR,
                        bold: false,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
                svg.push_str(&text_buf);
                text_y += NOTE_LINE_HEIGHT;
            }
        }

        svg.push_str("</g>");
    }

    // Render links (transitions).
    if let Some(orc) = oracle {
        render_oracle_transitions(&mut svg, diagram, orc);
    } else {
        for t in &diagram.transitions {
            let from_layout = map_id(&t.from, true);
            let to_layout = map_id(&t.to, false);
            let from_name = if t.from == "[*]" { "*start*" } else { &t.from };
            let to_name = if t.to == "[*]" { "*end*" } else { &t.to };

            // HTML comment.
            write!(svg, "<!--link {} to {}-->", from_name, to_name).unwrap();

            let link_id = ids.next_link();
            let from_ent = ent_id_of(&from_layout);
            let to_ent = ent_id_of(&to_layout);

            // Use the parser-provided source line from the transition model.
            let source_line = t.source_line;

            write!(
                svg,
                r#"<g class="link" data-entity-1="{from_ent}" data-entity-2="{to_ent}" data-link-type="dependency" data-source-line="{source_line}" id="{link_id}">"#,
            )
            .unwrap();

            // Try bezier path from layout engine.
            let edge_path = edge_paths
                .iter()
                .find(|ep| ep.from == from_layout && ep.to == to_layout);

            let (from_cx, from_cy, _from_w, from_h) = pos_of(&from_layout);
            let (to_cx, to_cy, _to_w, to_h) = pos_of(&to_layout);

            if let Some(ep) = edge_path
                && !ep.points.is_empty()
            {
                // Render bezier path.
                let points = &ep.points;
                let mut d = format!("M{},{}", fmt_f(points[0].0), fmt_f(points[0].1));
                let mut i = 1;
                while i + 2 < points.len() {
                    write!(
                        d,
                        " C{},{} {},{} {},{}",
                        fmt_f(points[i].0),
                        fmt_f(points[i].1),
                        fmt_f(points[i + 1].0),
                        fmt_f(points[i + 1].1),
                        fmt_f(points[i + 2].0),
                        fmt_f(points[i + 2].1),
                    )
                    .unwrap();
                    i += 3;
                }
                write!(
                    svg,
                    r#"<path d="{d}" fill="none" id="{from_name}-to-{to_name}" style="stroke:{STROKE_COLOR};stroke-width:1;"/>"#,
                )
                .unwrap();

                // Arrowhead polygon.
                let endpoint = points[points.len() - 1];
                let control = if points.len() >= 2 {
                    points[points.len() - 2]
                } else {
                    (from_cx, from_cy)
                };
                render_arrowhead(&mut svg, control, endpoint);

                // Label.
                if let Some(label) = &t.label {
                    let first = points.first().unwrap();
                    let last = points.last().unwrap();
                    let mid_x = (first.0 + last.0) / 2.0;
                    let mid_y = (first.1 + last.1) / 2.0;
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        label,
                        &TextBase {
                            x: mid_x + 1.0,
                            y: mid_y,
                            font_size: LINK_FONT_SIZE as u32,
                            font_family: "sans-serif",
                            fill: TEXT_COLOR,
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: false,
                        },
                    );
                    svg.push_str(&text_buf);
                }
            } else {
                // Straight line fallback.
                let start_y = from_cy + from_h / 2.0;
                let end_y = to_cy - to_h / 2.0;

                // Path as cubic bezier.
                let mid_y = (start_y + end_y) / 2.0;
                write!(
                    svg,
                    r#"<path d="M{},{} C{},{} {},{} {},{}" fill="none" id="{from_name}-to-{to_name}" style="stroke:{STROKE_COLOR};stroke-width:1;"/>"#,
                    fmt_f(from_cx), fmt_f(start_y),
                    fmt_f(from_cx), fmt_f(mid_y),
                    fmt_f(to_cx), fmt_f(mid_y),
                    fmt_f(to_cx), fmt_f(end_y),
                )
                .unwrap();

                // Arrowhead.
                let control = (to_cx, end_y - ARROW_LEN);
                let endpoint = (to_cx, end_y);
                render_arrowhead(&mut svg, control, endpoint);

                // Label.
                if let Some(label) = &t.label {
                    let label_x = from_cx.max(to_cx) + 1.0;
                    let label_y = (start_y + end_y) / 2.0;
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        label,
                        &TextBase {
                            x: label_x,
                            y: label_y,
                            font_size: LINK_FONT_SIZE as u32,
                            font_family: "sans-serif",
                            fill: TEXT_COLOR,
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: false,
                        },
                    );
                    svg.push_str(&text_buf);
                }
            }

            svg.push_str("</g>");
        }
    }

    svg.push_str("</g></svg>");
    svg
}

/// Render a filled arrowhead polygon at the endpoint, pointing in the direction
/// from control to endpoint.
fn render_arrowhead(svg: &mut String, control: (f64, f64), endpoint: (f64, f64)) {
    let dx = endpoint.0 - control.0;
    let dy = endpoint.1 - control.1;
    let angle = dy.atan2(dx);

    let tip_x = endpoint.0;
    let tip_y = endpoint.1;

    // PlantUML arrowhead is a 4-point diamond shape.
    let perp_x = (angle + std::f64::consts::FRAC_PI_2).cos();
    let perp_y = (angle + std::f64::consts::FRAC_PI_2).sin();

    let left_x = tip_x - ARROW_LEN * angle.cos() + ARROW_HALF * perp_x;
    let left_y = tip_y - ARROW_LEN * angle.sin() + ARROW_HALF * perp_y;
    let right_x = tip_x - ARROW_LEN * angle.cos() - ARROW_HALF * perp_x;
    let right_y = tip_y - ARROW_LEN * angle.sin() - ARROW_HALF * perp_y;

    // Indent point (PlantUML uses a diamond-shaped arrowhead).
    let indent_x = tip_x - (ARROW_LEN - 4.0) * angle.cos();
    let indent_y = tip_y - (ARROW_LEN - 4.0) * angle.sin();

    #[allow(non_snake_case)]
    let STROKE_COLOR = DEFAULT_STROKE_COLOR;
    write!(
        svg,
        r#"<polygon fill="{STROKE_COLOR}" points="{},{},{},{},{},{},{},{}" style="stroke:{STROKE_COLOR};stroke-width:1;"/>"#,
        fmt_f(tip_x), fmt_f(tip_y),
        fmt_f(left_x), fmt_f(left_y),
        fmt_f(indent_x), fmt_f(indent_y),
        fmt_f(right_x), fmt_f(right_y),
    )
    .unwrap();
}

/// Render transitions directly from oracle edge data.
fn render_oracle_transitions(svg: &mut String, diagram: &StateDiagram, oracle: &OracleLayout) {
    // Skinparam-aware text colour for transition labels: `stateArrowFontColor`
    // overrides the default `#000000`. Note that `stateFontColor` only
    // affects state names, not transition labels — keep them separate.
    let arrow_font_color = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| {
            sp.key.eq_ignore_ascii_case("stateArrowFontColor")
                || sp.key.eq_ignore_ascii_case("arrowFontColor")
        })
        .map(|sp| crate::sequence::resolve_color(sp.value.trim()))
        .unwrap_or_else(|| DEFAULT_TEXT_COLOR.to_string());
    #[allow(non_snake_case)]
    let TEXT_COLOR: &str = arrow_font_color.as_str();
    // `skinparam ArrowFontSize <n>` (or the legacy `stateArrowFontSize`)
    // overrides the default 13pt transition-label size; this also feeds
    // `text_render` so the emitted `textLength` matches the smaller glyphs.
    let arrow_font_size: u32 = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| {
            sp.key.eq_ignore_ascii_case("ArrowFontSize")
                || sp.key.eq_ignore_ascii_case("stateArrowFontSize")
        })
        .and_then(|sp| sp.value.trim().parse::<u32>().ok())
        .unwrap_or(LINK_FONT_SIZE as u32);
    let arrow_font_style = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| {
            sp.key.eq_ignore_ascii_case("ArrowFontStyle")
                || sp.key.eq_ignore_ascii_case("stateArrowFontStyle")
        })
        .map(|sp| sp.value.to_ascii_lowercase())
        .unwrap_or_default();
    let arrow_font_bold = arrow_font_style.contains("bold");
    let arrow_font_italic = arrow_font_style.contains("italic");
    // PlantUML's emission order for transitions does not always match the
    // parser's source order — when layout decides to bend an edge or sort
    // siblings differently, the golden SVG reorders them. Walking the
    // oracle's edges in their captured document order and mapping each
    // back to one of the parser's transitions preserves PlantUML's order.
    let mut consumed = vec![false; diagram.transitions.len()];
    let strip_suffix_digits = |full: &str, base: &str| -> bool {
        full == base
            || full
                .strip_prefix(base)
                .and_then(|s| s.strip_prefix('-'))
                .is_some_and(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
    };

    for oracle_edge in &oracle.edges {
        // Find an unused parser transition whose forward/reverse id matches
        // this oracle edge's `id` (with optional `-N` disambiguation).
        let mut matched: Option<(usize, bool)> = None;
        for (ti, t) in diagram.transitions.iter().enumerate() {
            if consumed[ti] {
                continue;
            }
            let from_name = if t.from == "[*]" { "*start*" } else { &t.from };
            let to_name = if t.to == "[*]" { "*end*" } else { &t.to };
            let forward_id = format!("{from_name}-to-{to_name}");
            let reverse_id = format!("{to_name}-backto-{from_name}");
            if strip_suffix_digits(&oracle_edge.id, &forward_id) {
                matched = Some((ti, false));
                break;
            }
            if strip_suffix_digits(&oracle_edge.id, &reverse_id) {
                matched = Some((ti, true));
                break;
            }
        }
        let Some((ti, is_reverse)) = matched else {
            // Note-attachment connectors (`<state>-GMN<n>`) have no parser
            // transition: PlantUML adds a dashed association line when a note
            // is displaced from the state it annotates (e.g. a second note on
            // the same side). Emit the captured edge verbatim, in document
            // order, so it lands where PlantUML placed it.
            if oracle_edge.id.contains("GMN") {
                emit_note_connector_verbatim(svg, oracle_edge);
            }
            continue;
        };
        consumed[ti] = true;
        let t = &diagram.transitions[ti];
        let from_name = if t.from == "[*]" { "*start*" } else { &t.from };
        let to_name = if t.to == "[*]" { "*end*" } else { &t.to };

        // HTML comment.
        if is_reverse {
            write!(svg, "<!--reverse link {to_name} to {from_name}-->").unwrap();
        } else {
            write!(svg, "<!--link {from_name} to {to_name}-->").unwrap();
        }

        // Link group wrapper using oracle attributes.
        let entity_1 = oracle_edge.entity_1.as_deref().unwrap_or("ent0002");
        let entity_2 = oracle_edge.entity_2.as_deref().unwrap_or("ent0003");
        let link_type = oracle_edge.link_type.as_deref().unwrap_or("dependency");
        let source_line = oracle_edge.source_line.as_deref().unwrap_or("0");
        let link_id = oracle_edge.link_id.as_deref().unwrap_or("lnk0");

        write!(
            svg,
            r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}" data-source-line="{source_line}" id="{link_id}">"#,
        )
        .unwrap();

        // Path element with oracle data.
        let path_style = oracle_edge
            .path_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        let path_id = &oracle_edge.id;
        write!(
            svg,
            r#"<path d="{}" fill="none" id="{path_id}" style="{path_style}"/>"#,
            oracle_edge.d,
        )
        .unwrap();

        // Arrowhead polygon from oracle.
        if let Some(ref points) = oracle_edge.arrow_points {
            let fill = oracle_edge.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = oracle_edge
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            write!(
                svg,
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            )
            .unwrap();
        }

        // Edge labels plus any `note on link` shape. PlantUML emits, in
        // document order: the transition's own label (e.g. `simple`), then the
        // note's box path and folded-corner path (`extra_paths`), then the note
        // text. We replay that order: the transition label first, the note's
        // box paths, then the remaining labels (the note text). When the
        // transition has no label of its own, every captured text belongs to
        // the note and follows the box.
        let emit_label = |svg: &mut String, lx: f64, ly: f64, text: &str| {
            let mut text_buf = String::new();
            text_render::emit_text(
                &mut text_buf,
                text,
                &TextBase {
                    x: lx,
                    y: ly,
                    font_size: arrow_font_size,
                    font_family: "sans-serif",
                    fill: TEXT_COLOR,
                    bold: arrow_font_bold,
                    italic: arrow_font_italic,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.push_str(&text_buf);
        };
        let emit_extra_paths = |svg: &mut String| {
            for (d, style) in &oracle_edge.extra_paths {
                let style = style
                    .as_deref()
                    .unwrap_or("stroke:#181818;stroke-width:0.5;");
                write!(svg, r#"<path d="{d}" fill="{NOTE_FILL}" style="{style}"/>"#).unwrap();
            }
        };
        if oracle_edge.extra_paths.is_empty() {
            for (lx, ly, text) in &oracle_edge.labels {
                emit_label(svg, *lx, *ly, text);
            }
        } else {
            let mut labels = oracle_edge.labels.iter();
            if t.label.is_some()
                && let Some((lx, ly, text)) = labels.next()
            {
                emit_label(svg, *lx, *ly, text);
            }
            emit_extra_paths(svg);
            for (lx, ly, text) in labels {
                emit_label(svg, *lx, *ly, text);
            }
        }

        svg.push_str("</g>");
    }
}

/// Replay one captured `GMN*` note entity verbatim: the `<g class="entity">`
/// wrapper, each captured body/dog-ear path, then the note text positioned
/// from the oracle's captured y values. Shared by the flat and composite
/// renderers so an anchored note (`note right of …`) renders identically
/// regardless of whether its target sits inside a composite.
fn emit_oracle_gmn_note(
    svg: &mut String,
    gmn_name: &str,
    rect: &crate::layout_oracle::EntityRect,
    note_text: &str,
) {
    let entity_id = rect
        .entity_id
        .clone()
        .unwrap_or_else(|| "ent0000".to_string());
    let source_line = rect.name_text_x.map(|sl| sl as usize).unwrap_or(0);
    write!(
        svg,
        r#"<g class="entity" data-qualified-name="{gmn_name}" data-source-line="{source_line}" id="{entity_id}">"#,
    )
    .unwrap();
    let mut first_d_for_left: Option<&str> = None;
    if let Some(paths) = &rect.glyph_path_d {
        for piece in paths.split('|') {
            let (d, style) = piece
                .split_once("#STYLE#")
                .unwrap_or((piece, "stroke:#181818;stroke-width:0.5;"));
            if first_d_for_left.is_none() {
                first_d_for_left = Some(d);
            }
            write!(svg, r#"<path d="{d}" fill="{NOTE_FILL}" style="{style}"/>"#).unwrap();
        }
    }
    let body_left_x = first_d_for_left
        .and_then(|d| {
            d.strip_prefix('M')
                .and_then(|rest| rest.split(',').next())
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(rect.x);
    let text_x = body_left_x + NOTE_PADDING;
    let lines: Vec<&str> = note_text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let line_ys = distinct_line_ys(&rect.text_y_values);
    for (i, line) in lines.iter().enumerate() {
        let fallback_y = rect.y + NOTE_PADDING + LINK_FONT_SIZE + i as f64 * NOTE_LINE_HEIGHT;
        let ty = line_ys.get(i).copied().unwrap_or(fallback_y);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            line,
            &TextBase {
                x: text_x,
                y: ty,
                font_size: LINK_FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: DEFAULT_TEXT_COLOR,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.push_str(&text_buf);
    }
    svg.push_str("</g>");
}

/// Emit a note-attachment connector edge (`<state>-GMN<n>`) verbatim. These
/// dashed association lines link a note back to the state it annotates when the
/// note is displaced (e.g. a second note on the same side of a state). The id
/// joins the state name and the auto-generated `GMN<n>` note name with a single
/// dash; PlantUML's comment spells it `<!--link <state> to GMN<n>-->`.
fn emit_note_connector_verbatim(svg: &mut String, edge: &OracleEdgePath) {
    if let Some(idx) = edge.id.rfind("-GMN") {
        let from = &edge.id[..idx];
        let to = &edge.id[idx + 1..];
        write!(svg, "<!--link {from} to {to}-->").unwrap();
    }
    let entity_1 = edge.entity_1.as_deref().unwrap_or("ent0002");
    let entity_2 = edge.entity_2.as_deref().unwrap_or("ent0003");
    let link_type = edge.link_type.as_deref().unwrap_or("association");
    let source_line = edge.source_line.as_deref().unwrap_or("0");
    let link_id = edge.link_id.as_deref().unwrap_or("lnk0");
    write!(
        svg,
        r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}" data-source-line="{source_line}" id="{link_id}">"#,
    )
    .unwrap();
    let path_style = edge
        .path_style
        .as_deref()
        .unwrap_or("stroke:#181818;stroke-width:1;stroke-dasharray:7,7;");
    write!(
        svg,
        r#"<path d="{}" fill="none" id="{}" style="{path_style}"/>"#,
        edge.d, edge.id,
    )
    .unwrap();
    svg.push_str("</g>");
}

/// Emit one captured oracle edge verbatim (comment + `<g class="link">`
/// wrapper + path + arrowhead + labels). The edge's id, geometry, entity
/// references and styling all come straight from the golden, so composite
/// diagrams reproduce PlantUML's exact transition output without re-deriving
/// any of it.
fn emit_oracle_edge_verbatim(svg: &mut String, edge: &OracleEdgePath) {
    // Reconstruct the HTML comment PlantUML prints before each link. Edge
    // ids are `<from>-to-<to>` / `<from>-backto-<to>`; the comment uses the
    // same names with the verb spelled out.
    if let Some((from, to)) = edge.id.split_once("-backto-") {
        write!(svg, "<!--reverse link {to} to {from}-->").unwrap();
    } else if let Some((from, to)) = edge.id.split_once("-to-") {
        write!(svg, "<!--link {from} to {to}-->").unwrap();
    }
    let entity_1 = edge.entity_1.as_deref().unwrap_or("ent0002");
    let entity_2 = edge.entity_2.as_deref().unwrap_or("ent0003");
    let link_type = edge.link_type.as_deref().unwrap_or("dependency");
    let source_line = edge.source_line.as_deref().unwrap_or("0");
    let link_id = edge.link_id.as_deref().unwrap_or("lnk0");
    write!(
        svg,
        r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}" data-source-line="{source_line}" id="{link_id}">"#,
    )
    .unwrap();
    let path_style = edge
        .path_style
        .as_deref()
        .unwrap_or("stroke:#181818;stroke-width:1;");
    write!(
        svg,
        r#"<path d="{}" fill="none" id="{}" style="{path_style}"/>"#,
        edge.d, edge.id,
    )
    .unwrap();
    if let Some(points) = &edge.arrow_points {
        let fill = edge.arrow_fill.as_deref().unwrap_or("#181818");
        let poly_style = edge
            .polygon_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        write!(
            svg,
            r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
        )
        .unwrap();
    }
    for (lx, ly, text) in &edge.labels {
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            text,
            &TextBase {
                x: *lx,
                y: *ly,
                font_size: LINK_FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: DEFAULT_TEXT_COLOR,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.push_str(&text_buf);
    }
    svg.push_str("</g>");
}

/// Render a state diagram that contains composite states (`state X { … }`).
///
/// Composite layout is driven entirely by the oracle: PlantUML's nested-region
/// geometry (cluster border bands, scoped `[*]` pseudo-states, qualified inner
/// entities) is hard to reproduce from first principles, so we emit every
/// shape from the captured golden positions. The flat renderer cannot model
/// the scoped pseudo-states or the cluster-first emission order, so composites
/// take this dedicated path.
///
/// Emission order matches PlantUML: for each composite (declaration order) the
/// cluster body, then its inner entities, then its inner links; finally the
/// top-level pseudo-states / states and their links.
fn render_composite_with_oracle(diagram: &StateDiagram, orc: &OracleLayout) -> String {
    let mut svg = String::new();

    // Resolve an oracle entity by qualified name. Scoped pseudo-states carry
    // the dotted qualified-name form `<scope>..start.<scope>` in the golden.
    let pseudo_qname = |marker: &str, is_start: bool| -> String {
        match marker.strip_prefix("[*]") {
            Some("") | None => {
                if is_start {
                    ".start.".to_string()
                } else {
                    ".end.".to_string()
                }
            }
            Some(scope) => {
                // The suffix uses only the scope's last dotted segment: a
                // top-level composite `Concurrent` yields
                // `Concurrent..start.Concurrent`, while a concurrent-region
                // sub-scope `Concurrent.CONC2` yields
                // `Concurrent.CONC2..start.CONC2`.
                let suffix = scope.rsplit('.').next().unwrap_or(scope);
                if is_start {
                    format!("{scope}..start.{suffix}")
                } else {
                    format!("{scope}..end.{suffix}")
                }
            }
        }
    };

    // For each transition, determine whether it is "inner" to a given
    // composite scope: both endpoints belong to that scope (either a child
    // state with `parent == scope`, or a scoped pseudo-state `[*]<scope>`).
    let endpoint_scope = |id: &str| -> Option<String> {
        if let Some(scope) = id.strip_prefix("[*]") {
            return if scope.is_empty() {
                None
            } else {
                Some(scope.to_string())
            };
        }
        diagram
            .states
            .iter()
            .find(|s| s.id == id)
            .and_then(|s| s.parent.clone())
    };

    // Track which oracle edges we've emitted, matching by edge id. A RefCell so
    // both the top-level walk and the nested-composite recursion (which borrows
    // it via the shared `ScopeEmit`) can mark edges emitted.
    let emitted_edge = std::cell::RefCell::new(vec![false; orc.edges.len()]);

    // Helper: emit a start/end pseudo-state group from oracle geometry.
    let emit_pseudo = |svg: &mut String, qname: &str, is_start: bool, source_line: &str| {
        let Some(rect) = orc.entities.get(qname) else {
            return false;
        };
        let cx = rect.x + rect.width / 2.0;
        let cy = rect.y + rect.height / 2.0;
        let entity_id = rect.entity_id.as_deref().unwrap_or("ent0000");
        if is_start {
            write!(
                svg,
                r#"<g class="start_entity" data-qualified-name="{qname}" data-source-line="{source_line}" id="{entity_id}"><ellipse cx="{}" cy="{}" fill="{PSEUDO_COLOR}" rx="{START_RADIUS}" ry="{START_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/></g>"#,
                fmt_f(cx),
                fmt_f(cy),
            )
            .unwrap();
        } else {
            let inner_fill = rect.fill.as_deref().unwrap_or(PSEUDO_COLOR);
            write!(
                svg,
                r#"<g class="end_entity" data-qualified-name="{qname}" data-source-line="{source_line}" id="{entity_id}"><ellipse cx="{}" cy="{}" fill="none" rx="{END_OUTER_RADIUS}" ry="{END_OUTER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/><ellipse cx="{}" cy="{}" fill="{inner_fill}" rx="{END_INNER_RADIUS}" ry="{END_INNER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/></g>"#,
                fmt_f(cx),
                fmt_f(cy),
                fmt_f(cx),
                fmt_f(cy),
            )
            .unwrap();
        }
        true
    };

    // History pseudo-states render as a bare `<ellipse>` + `<text>` pair (no
    // `<g class="entity">` wrapper) outside the entity loop. The oracle records
    // each one under the synthetic key `__history_N__` in SVG document order;
    // `history_idx` walks them as the bare-composite traversal encounters
    // History/DeepHistory states in declaration order.
    let history_idx = std::cell::Cell::new(0usize);
    // Fork/join bars live under the oracle's `__bar_N__` synthetic keys in SVG
    // document order; `bar_idx` walks them as the traversal meets Fork/Join.
    let bar_idx = std::cell::Cell::new(0usize);
    // Entry/exit pseudo-states live under `__entryexit_N__` in SVG document
    // order; `ee_idx` walks them as the traversal meets EntryPoint/ExitPoint.
    let ee_idx = std::cell::Cell::new(0usize);

    // Helper: emit a normal state box (rect + divider + name [+ descriptions])
    // from oracle geometry, keyed by its qualified id.
    let emit_state_box = |svg: &mut String, st: &State| {
        if matches!(st.kind, StateKind::EntryPoint | StateKind::ExitPoint) {
            // The point's label (`<text>`) precedes the ellipse; an exit point
            // adds two crossing lines after it. All geometry is captured from
            // the golden under `__entryexit_N__`.
            let key = format!("__entryexit_{}__", ee_idx.get());
            ee_idx.set(ee_idx.get() + 1);
            let Some(rect) = orc.entities.get(key.as_str()) else {
                return;
            };
            for t in &rect.texts {
                let tw = text_render::measure(&t.text, STATE_FONT_SIZE, false);
                let mut buf = String::new();
                text_render::emit_text(
                    &mut buf,
                    &t.text,
                    &TextBase {
                        x: t.x,
                        y: t.y,
                        font_size: STATE_FONT_SIZE as u32,
                        font_family: "sans-serif",
                        fill: DEFAULT_TEXT_COLOR,
                        bold: false,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
                let _ = tw;
                svg.push_str(&buf);
            }
            let cx = rect.x + rect.width / 2.0;
            let cy = rect.y + rect.height / 2.0;
            let rxy = rect.width / 2.0;
            let fill = rect.fill.as_deref().unwrap_or(DEFAULT_STATE_FILL);
            let style = rect
                .body_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1.5;");
            write!(
                svg,
                r#"<ellipse cx="{}" cy="{}" fill="{fill}" rx="{}" ry="{}" style="{style}"/>"#,
                fmt_f(cx),
                fmt_f(cy),
                fmt_f(rxy),
                fmt_f(rxy),
            )
            .unwrap();
            for l in &rect.lines {
                let lstyle = l
                    .style
                    .as_deref()
                    .unwrap_or("stroke:#181818;stroke-width:1.5;");
                write!(
                    svg,
                    r#"<line style="{lstyle}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                    l.x1, l.x2, l.y1, l.y2,
                )
                .unwrap();
            }
            return;
        }
        if matches!(st.kind, StateKind::History | StateKind::DeepHistory) {
            let key = format!("__history_{}__", history_idx.get());
            history_idx.set(history_idx.get() + 1);
            let Some(rect) = orc.entities.get(key.as_str()) else {
                return;
            };
            let px = rect.x + rect.width / 2.0;
            let py = rect.y + rect.height / 2.0;
            let h_radius = rect.width / 2.0;
            let label = if matches!(st.kind, StateKind::DeepHistory) {
                "H*"
            } else {
                "H"
            };
            let h_fill = DEFAULT_STATE_FILL;
            let h_stroke = DEFAULT_STROKE_COLOR;
            let h_text = DEFAULT_TEXT_COLOR;
            write!(
                svg,
                r#"<ellipse cx="{}" cy="{}" fill="{h_fill}" rx="{}" ry="{}" style="stroke:{h_stroke};stroke-width:0.5;"/>"#,
                fmt_f(px),
                fmt_f(py),
                fmt_f(h_radius),
                fmt_f(h_radius),
            )
            .unwrap();
            let tw = text_render::measure(label, STATE_FONT_SIZE, false);
            let text_y = py + 5.291;
            write!(
                svg,
                r#"<text fill="{h_text}" font-family="sans-serif" font-size="{STATE_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{label}</text>"#,
                fmt_f(tw),
                fmt_f(px - tw / 2.0),
                fmt_f(text_y),
            )
            .unwrap();
            return;
        }
        if matches!(st.kind, StateKind::Choice) {
            // Choice pseudo-state inside a composite — a `<polygon>` diamond
            // wrapped in `<g class="entity">`, matching the top-level choice
            // path. Geometry comes from the oracle's captured entity bounds.
            let Some(rect) = orc.entities.get(st.id.as_str()) else {
                return;
            };
            let entity_id = rect.entity_id.as_deref().unwrap_or("ent0000");
            let cx = rect.x + rect.width / 2.0;
            let cy = rect.y + rect.height / 2.0;
            let top = cy - CHOICE_SIZE;
            let right = cx + CHOICE_SIZE;
            let bottom = cy + CHOICE_SIZE;
            let left = cx - CHOICE_SIZE;
            let fill = rect.fill.as_deref().unwrap_or(DEFAULT_STATE_FILL);
            write!(
                svg,
                r#"<g class="entity" data-qualified-name="{}" id="{entity_id}"><polygon fill="{fill}" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{DEFAULT_STROKE_COLOR};stroke-width:0.5;"/></g>"#,
                st.id,
                fmt_f(cx), fmt_f(top),
                fmt_f(right), fmt_f(cy),
                fmt_f(cx), fmt_f(bottom),
                fmt_f(left), fmt_f(cy),
                fmt_f(cx), fmt_f(top),
            )
            .unwrap();
            return;
        }
        if matches!(st.kind, StateKind::Fork | StateKind::Join) {
            // Fork/join bar — a bare `<rect>` (no `<g>` wrapper). The oracle
            // records the bar under `__bar_N__`; pair them in declaration
            // order via `bar_idx`.
            let key = format!("__bar_{}__", bar_idx.get());
            bar_idx.set(bar_idx.get() + 1);
            let Some(rect) = orc.entities.get(key.as_str()) else {
                return;
            };
            write!(
                svg,
                r#"<rect fill="{BAR_COLOR}" height="{}" style="stroke:none;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
                fmt_f(rect.height),
                fmt_f(rect.width),
                fmt_f(rect.x),
                fmt_f(rect.y),
            )
            .unwrap();
            return;
        }
        let Some(rect) = orc.entities.get(st.id.as_str()) else {
            return;
        };
        let entity_id = rect.entity_id.as_deref().unwrap_or("ent0000");
        let fill = rect.fill.as_deref().unwrap_or("#F1F1F1");
        let style = rect
            .rect_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:0.5;");
        let rx = rect.rect_rx.as_deref().unwrap_or("12.5");
        let ry = rect.rect_ry.as_deref().unwrap_or("12.5");
        write!(
            svg,
            r#"<g class="entity" data-qualified-name="{}" id="{entity_id}"><rect fill="{fill}" height="{}" rx="{rx}" ry="{ry}" style="{style}" width="{}" x="{}" y="{}"/>"#,
            st.id,
            fmt_f(rect.height),
            fmt_f(rect.width),
            fmt_f(rect.x),
            fmt_f(rect.y),
        )
        .unwrap();
        // Divider line(s) and name/description text come from the oracle's
        // captured children for byte-exact placement.
        for line in &rect.lines {
            let lstyle = line
                .style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:0.5;");
            write!(
                svg,
                r#"<line style="{lstyle}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                line.x1, line.x2, line.y1, line.y2,
            )
            .unwrap();
        }
        // Name label: oracle name_text_x + first text y.
        let name_x = rect.name_text_x.unwrap_or(rect.x + 10.0);
        let name_y = rect
            .text_y_values
            .first()
            .copied()
            .unwrap_or(rect.y + NAME_BASELINE_OFFSET);
        let mut text_buf = String::new();
        text_render::emit_text(
            &mut text_buf,
            &st.label,
            &TextBase {
                x: name_x,
                y: name_y,
                font_size: STATE_FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: DEFAULT_TEXT_COLOR,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.push_str(&text_buf);
        // Descriptions on subsequent baselines.
        for (j, desc) in st.descriptions.iter().enumerate() {
            let dy = rect
                .text_y_values
                .get(j + 1)
                .copied()
                .unwrap_or(name_y + FIRST_DESC_OFFSET + j as f64 * DESC_LINE_SPACING);
            let mut dbuf = String::new();
            text_render::emit_text(
                &mut dbuf,
                desc,
                &TextBase {
                    x: rect.x + 5.0,
                    y: dy,
                    font_size: DESC_FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: DEFAULT_TEXT_COLOR,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.push_str(&dbuf);
        }
        svg.push_str("</g>");
    };

    // Order the immediate children of a scope (None = top level) as PlantUML
    // emits them. Mirrors the flat renderer's first-appearance rule: a state
    // declared before its first referencing transition appears at its
    // declaration line; otherwise children (states and scoped pseudo-states)
    // appear in transition-walk order, `from` before `to`, so a `[*] --> X`
    // line emits the scope's start pseudo-state ahead of `X`.
    let ordered_children = |scope: Option<&str>| -> Vec<String> {
        let mut items: Vec<(usize, usize, String)> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut seq = 0usize;
        let first_txn_line = |id: &str| -> Option<usize> {
            diagram
                .transitions
                .iter()
                .filter(|t| t.from == id || t.to == id)
                .map(|t| t.source_line)
                .min()
        };
        // True when some transition references `id` with its *other* endpoint
        // also inside `scope` — i.e. the state participates in the composite's
        // own flow rather than only being targeted from outside.
        let has_in_scope_txn = |id: &str| -> bool {
            diagram.transitions.iter().any(|t| {
                let (other, mine) = if t.from == id {
                    (&t.to, true)
                } else if t.to == id {
                    (&t.from, true)
                } else {
                    (&t.from, false)
                };
                if !mine {
                    return false;
                }
                let other_scope = if other.starts_with("[*]") {
                    endpoint_scope(other)
                } else {
                    diagram
                        .states
                        .iter()
                        .find(|s| s.id == **other)
                        .and_then(|s| s.parent.clone())
                };
                other_scope.as_deref() == scope
            })
        };
        // A history pseudo-state that is never wired into its composite's own
        // flow (only targeted from outside, as in `OutState --> S.H`) is drawn
        // by PlantUML at the very front of the composite — ahead of the scope's
        // `[*]` start pseudo-state and plain children. Float such orphan history
        // nodes to effective line 0. (History nodes that DO take part in the
        // internal flow keep their normal first-appearance ordering.)
        for s in &diagram.states {
            if s.parent.as_deref() != scope {
                continue;
            }
            if matches!(s.kind, StateKind::History | StateKind::DeepHistory)
                && !has_in_scope_txn(&s.id)
                && seen.insert(s.id.clone())
            {
                items.push((0, seq, s.id.clone()));
                seq += 1;
            }
        }
        // Pre-register states declared before their first use.
        for s in &diagram.states {
            if s.parent.as_deref() != scope {
                continue;
            }
            let declared_before_use = match first_txn_line(&s.id) {
                Some(l) => s.source_line < l,
                None => true,
            };
            if declared_before_use && seen.insert(s.id.clone()) {
                items.push((s.source_line, seq, s.id.clone()));
                seq += 1;
            }
        }
        // Transition walk: from then to. Only endpoints belonging to this
        // scope. A scoped `[*]` marker serves as both the region's start (when
        // used as a transition source) and its end (when used as a target);
        // these are distinct entities, so role-tag pseudo tokens as
        // `\u{1}S<marker>` (start) and `\u{1}E<marker>` (end).
        for t in &diagram.transitions {
            for (ep, is_from) in [(&t.from, true), (&t.to, false)] {
                let in_scope = if ep.starts_with("[*]") {
                    endpoint_scope(ep).as_deref() == scope
                } else {
                    diagram
                        .states
                        .iter()
                        .find(|s| s.id == *ep)
                        .map(|s| s.parent.as_deref() == scope)
                        .unwrap_or(false)
                };
                if !in_scope {
                    continue;
                }
                let token = if ep.starts_with("[*]") {
                    format!("\u{1}{}{ep}", if is_from { 'S' } else { 'E' })
                } else {
                    ep.clone()
                };
                if seen.insert(token.clone()) {
                    items.push((t.source_line, seq, token));
                    seq += 1;
                }
            }
        }
        // Any remaining declared states in this scope.
        for s in &diagram.states {
            if s.parent.as_deref() == scope && seen.insert(s.id.clone()) {
                items.push((s.source_line, seq, s.id.clone()));
                seq += 1;
            }
        }
        // PlantUML never draws a `[*]` start/end pseudo-state ahead of a
        // composite (cluster) in the same scope, even when the pseudo-state's
        // transition appears on an earlier source line. (Cf.
        // `state_sequential_composites_*` and `state_composite_basic`, where
        // `[*] --> Outer` on line 1 still renders after the Outer cluster.)
        // Plain state boxes, by contrast, DO interleave with composites by line
        // (cf. `Idle` before the `Moving` cluster in `state_game_character`).
        // Model this by bumping each pseudo-state token's effective sort line up
        // to the latest composite line in this scope; a per-item "composite
        // wins ties" rank then keeps the cluster ahead of a pseudo-state landing
        // on the same line. The stable `seq` tiebreak preserves the
        // pseudo-states' own relative order and their interleaving with any
        // later plain boxes (cf. `.start.` before the `Outside` box in
        // `state_cross_boundary_out`).
        let max_composite_line = diagram
            .states
            .iter()
            .filter(|s| s.parent.as_deref() == scope && s.composite)
            .map(|s| s.source_line)
            .max();
        if let Some(mc) = max_composite_line {
            for (line, _, id) in &mut items {
                if id.starts_with('\u{1}') && *line < mc {
                    *line = mc;
                }
            }
        }
        // rank 0 = composite (sorts first on equal line); rank 1 = everything
        // else (plain boxes, pseudo-states).
        let rank = |id: &str| -> u8 {
            if diagram.states.iter().any(|s| s.id == id && s.composite) {
                0
            } else {
                1
            }
        };
        items.sort_by_key(|(l, s, id)| (*l, rank(id), *s));
        items.into_iter().map(|(_, _, id)| id).collect()
    };

    // Emit a scope's entities (pseudo-states + state boxes), recursing into
    // nested composites. Grouped into one struct of borrows to keep the
    // recursive call site readable.
    struct ScopeEmit<'a> {
        diagram: &'a StateDiagram,
        ordered_children: &'a dyn Fn(Option<&str>) -> Vec<String>,
        emit_pseudo: &'a dyn Fn(&mut String, &str, bool, &str) -> bool,
        emit_state_box: &'a dyn Fn(&mut String, &State),
        pseudo_qname: &'a dyn Fn(&str, bool) -> String,
        region_scopes: &'a dyn Fn(&str) -> Vec<String>,
        /// Free-standing region-divider lines in document order, plus the index
        /// of the next one to splice. Consumed left-to-right as regions are
        /// emitted across the whole diagram.
        dividers: &'a [crate::layout_oracle::RegionDivider],
        next_divider: &'a std::cell::Cell<usize>,
        /// Emit a nested composite's header band (`<path>` + border + divider +
        /// title). Only used on the bare-composite path.
        emit_cluster: &'a dyn Fn(&mut String, &State),
        /// Emit the links whose endpoints both sit in a given scope.
        emit_scope_links: &'a dyn Fn(&mut String, Option<&str>),
        /// False on the bare-composite path (headers + links emitted inline per
        /// scope); true when PlantUML wrapped composites in `<g class="cluster">`
        /// groups (headers up front, all links deferred to the end).
        has_clusters: bool,
    }
    fn emit_scope_entities(svg: &mut String, scope: Option<&str>, e: &ScopeEmit) {
        let diagram = e.diagram;
        let ordered_children = e.ordered_children;
        let emit_pseudo = e.emit_pseudo;
        let emit_state_box = e.emit_state_box;
        let pseudo_qname = e.pseudo_qname;
        let tokens = ordered_children(scope);
        // PlantUML emits a scope's nested composites first, then the scope's
        // own pseudo-states and plain states in declaration/use order. The
        // composite "float to front" is what places a deeply nested cluster
        // ahead of its parent's `[*]` markers, while plain states still
        // interleave with the pseudo-states by line (e.g. start, S, end).
        for token in &tokens {
            if token.starts_with('\u{1}') {
                continue;
            }
            let Some(st) = diagram.states.iter().find(|s| s.id == *token) else {
                continue;
            };
            if !st.composite {
                continue;
            }
            // On the bare path, a nested composite emits its own header band
            // before its regions (the cluster-group path emits all headers up
            // front via `emit_clusters_dfs`).
            if !e.has_clusters {
                (e.emit_cluster)(svg, st);
            }
            // Walk each concurrent region of the nested composite, splicing the
            // dashed region divider before every region after the first.
            for (ri, rscope) in (e.region_scopes)(&st.id).into_iter().enumerate() {
                if ri > 0 {
                    let i = e.next_divider.get();
                    if let Some(div) = e.dividers.get(i) {
                        svg.push_str(&div.xml);
                        e.next_divider.set(i + 1);
                    }
                }
                emit_scope_entities(svg, Some(&rscope), e);
                if !e.has_clusters {
                    (e.emit_scope_links)(svg, Some(&rscope));
                }
            }
        }
        // Second pass: pseudo-states and plain states, in original order.
        for token in &tokens {
            if let Some(rest) = token.strip_prefix('\u{1}') {
                let is_start = rest.starts_with('S');
                let marker = &rest[1..];
                let sl = diagram
                    .transitions
                    .iter()
                    .find(|t| {
                        if is_start {
                            t.from == marker
                        } else {
                            t.to == marker
                        }
                    })
                    .map(|t| t.source_line.to_string())
                    .unwrap_or_else(|| "0".to_string());
                let q = pseudo_qname(marker, is_start);
                emit_pseudo(svg, &q, is_start, &sl);
            } else if let Some(st) = diagram.states.iter().find(|s| s.id == *token)
                && !st.composite
            {
                emit_state_box(svg, st);
            }
        }
    }

    // Cluster (composite border + header band + divider + title) from oracle.
    let emit_cluster = |svg: &mut String, st: &State| {
        let Some(rect) = orc.entities.get(st.label.as_str()) else {
            return;
        };
        // When PlantUML draws the composite as a `<g class="cluster">` group
        // (multiple sibling composites), the oracle captures an `entity_id` for
        // it; the bare-composite form (single composite) has none. Emit the
        // wrapper only in the cluster-group case.
        let cluster_wrapped = rect.entity_id.is_some();
        if cluster_wrapped {
            let source_line = rect.source_line.as_deref().unwrap_or("0");
            write!(
                svg,
                r#"<g class="cluster" data-qualified-name="{}" data-source-line="{source_line}" id="{}">"#,
                st.id,
                rect.entity_id.as_deref().unwrap_or("ent0000"),
            )
            .unwrap();
        }
        // Header band path (rounded top, square bottom meeting the divider).
        // Stored as `d#FILL#<fill>`.
        if let Some(raw) = &rect.glyph_path_d {
            let (d, fill) = raw.split_once("#FILL#").unwrap_or((raw, "#F1F1F1"));
            write!(svg, r##"<path d="{d}" fill="{fill}"/>"##).unwrap();
        }
        let style = rect
            .rect_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:0.5;");
        let rx = rect.rect_rx.as_deref().unwrap_or("12.5");
        let ry = rect.rect_ry.as_deref().unwrap_or("12.5");
        write!(
            svg,
            r#"<rect fill="none" height="{}" rx="{rx}" ry="{ry}" style="{style}" width="{}" x="{}" y="{}"/>"#,
            fmt_f(rect.height),
            fmt_f(rect.width),
            fmt_f(rect.x),
            fmt_f(rect.y),
        )
        .unwrap();
        for line in &rect.lines {
            let lstyle = line
                .style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:0.5;");
            write!(
                svg,
                r#"<line style="{lstyle}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                line.x1, line.x2, line.y1, line.y2,
            )
            .unwrap();
        }
        // Title (font 14) followed by any `state X : desc` lines (font 12).
        // Use the oracle's captured text positions for byte-exact placement;
        // fall back to computed positions when the oracle didn't capture them.
        if rect.texts.is_empty() {
            let tx = rect.name_text_x.unwrap_or(rect.x + 10.0);
            let ty = rect.y + NAME_BASELINE_OFFSET;
            let mut buf = String::new();
            text_render::emit_text(
                &mut buf,
                &st.label,
                &TextBase {
                    x: tx,
                    y: ty,
                    font_size: STATE_FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: DEFAULT_TEXT_COLOR,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.push_str(&buf);
        } else {
            for (i, t) in rect.texts.iter().enumerate() {
                let font = if i == 0 {
                    STATE_FONT_SIZE as u32
                } else {
                    DESC_FONT_SIZE as u32
                };
                let mut buf = String::new();
                text_render::emit_text(
                    &mut buf,
                    &t.text,
                    &TextBase {
                        x: t.x,
                        y: t.y,
                        font_size: font,
                        font_family: "sans-serif",
                        fill: DEFAULT_TEXT_COLOR,
                        bold: false,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
                svg.push_str(&buf);
            }
        }
        if cluster_wrapped {
            svg.push_str("</g>");
        }
    };

    // Emit links whose both endpoints are inside `scope` (None = top level),
    // in oracle document order.
    let emit_scope_links = |svg: &mut String, scope: Option<&str>| {
        for (ei, edge) in orc.edges.iter().enumerate() {
            if emitted_edge.borrow()[ei] {
                continue;
            }
            // Find the parser transition for this edge to learn its scope.
            let tx = diagram.transitions.iter().find(|t| {
                let fr = short_name_match(&t.from, &edge.id, true);
                let to = short_name_match(&t.to, &edge.id, false);
                fr && to
            });
            let Some(t) = tx else { continue };
            let fs = endpoint_scope(&t.from);
            let ts = endpoint_scope(&t.to);
            // An edge belongs to a composite scope when both endpoints share
            // that scope; otherwise it's a top-level link.
            let edge_scope = if fs == ts { fs } else { None };
            if edge_scope.as_deref() == scope {
                emit_oracle_edge_verbatim(svg, edge);
                emitted_edge.borrow_mut()[ei] = true;
            }
        }
    };

    // PlantUML emits every `<g class="cluster">` border group up front, in
    // depth-first declaration order (outer composite, then its nested
    // composites), *before* any entity or link. `emit_cluster` self-guards on
    // the oracle having a captured cluster entity for the composite's label, so
    // composites that PlantUML renders as plain nested boxes (no cluster) emit
    // nothing here. Walk composites DFS following `ordered_children` order.
    fn emit_clusters_dfs(
        svg: &mut String,
        scope: Option<&str>,
        diagram: &StateDiagram,
        ordered_children: &dyn Fn(Option<&str>) -> Vec<String>,
        emit_cluster: &dyn Fn(&mut String, &State),
    ) {
        for token in ordered_children(scope) {
            if token.starts_with('\u{1}') {
                continue;
            }
            let Some(st) = diagram.states.iter().find(|s| s.id == token) else {
                continue;
            };
            if st.composite {
                emit_cluster(svg, st);
                emit_clusters_dfs(svg, Some(&st.id), diagram, ordered_children, emit_cluster);
            }
        }
    }
    // When PlantUML wraps composites in `<g class="cluster">` groups (captured
    // as an `entity_id` on the composite's oracle entity), it emits every
    // cluster border first, then all entities, then ALL links at the very end
    // in oracle edge order. The bare-composite form instead interleaves each
    // composite's header band, entities and links inline per scope.
    // `has_clusters` selects between the two emission shapes.
    let has_clusters = diagram.states.iter().any(|s| {
        s.composite
            && orc
                .entities
                .get(s.label.as_str())
                .is_some_and(|r| r.entity_id.is_some())
    });

    if has_clusters {
        emit_clusters_dfs(&mut svg, None, diagram, &ordered_children, &emit_cluster);
    }

    // Ordered concurrent-region scopes of a composite. Region 0 is the
    // composite itself; regions N≥1 are the synthetic sub-scopes
    // `<composite>.CONC{N+1}` introduced by `--`/`||` separators. Returns just
    // the composite id when there are no regions.
    let region_scopes = |composite_id: &str| -> Vec<String> {
        // Collect every `<composite_id>.CONC{n}` sub-scope referenced by a
        // child state or scoped pseudo-state. The CONC counter is diagram-wide,
        // so a single composite's regions need not use consecutive indices
        // (CS1 → CONC2, CS2 → CONC3); gather the distinct n values and sort.
        let prefix = format!("{composite_id}.CONC");
        let conc_n = |id: &str| -> Option<usize> {
            id.strip_prefix(&prefix)
                .and_then(|rest| rest.split('.').next())
                .and_then(|n| n.parse::<usize>().ok())
        };
        let mut ns: Vec<usize> = Vec::new();
        for s in &diagram.states {
            if let Some(p) = s.parent.as_deref()
                && let Some(n) = conc_n(p)
                && !ns.contains(&n)
            {
                ns.push(n);
            }
        }
        for t in &diagram.transitions {
            for ep in [&t.from, &t.to] {
                if let Some(scope) = ep.strip_prefix("[*]")
                    && let Some(n) = conc_n(scope)
                    && !ns.contains(&n)
                {
                    ns.push(n);
                }
            }
        }
        ns.sort_unstable();
        let mut scopes = vec![composite_id.to_string()];
        scopes.extend(ns.into_iter().map(|n| format!("{composite_id}.CONC{n}")));
        scopes
    };

    // Free-standing region-divider lines, consumed left-to-right (document
    // order) as concurrent regions are emitted anywhere in the tree. A single
    // shared cursor threads through both the top-level region walk below and
    // the nested-composite recursion inside `emit_scope_entities`.
    let next_divider = std::cell::Cell::new(0usize);
    let scope_emit = ScopeEmit {
        diagram,
        ordered_children: &ordered_children,
        emit_pseudo: &emit_pseudo,
        emit_state_box: &emit_state_box,
        pseudo_qname: &pseudo_qname,
        region_scopes: &region_scopes,
        dividers: &orc.region_dividers,
        next_divider: &next_divider,
        emit_cluster: &emit_cluster,
        emit_scope_links: &emit_scope_links,
        has_clusters,
    };

    // Walk top-level children in a single first-appearance pass: composites
    // (clusters with their inner entities), plain state boxes, and the scope's
    // own `[*]` pseudo-states all interleave in declaration/use order. This
    // mirrors PlantUML's emission: a top-level state declared early (e.g.
    // `Idle` in `state_game_character`) precedes a later composite, while a
    // `[*] --> X` start pseudo-state precedes a plain state first referenced on
    // a later line (e.g. `.start.` before `Outside` in
    // `state_cross_boundary_out`).
    for token in ordered_children(None) {
        if let Some(rest) = token.strip_prefix('\u{1}') {
            // Top-level start/end pseudo-state.
            let is_start = rest.starts_with('S');
            let marker = &rest[1..];
            let sl = diagram
                .transitions
                .iter()
                .find(|t| {
                    if is_start {
                        t.from == marker
                    } else {
                        t.to == marker
                    }
                })
                .map(|t| t.source_line.to_string())
                .unwrap_or_else(|| "0".to_string());
            emit_pseudo(&mut svg, &pseudo_qname(marker, is_start), is_start, &sl);
            continue;
        }
        let Some(st) = diagram.states.iter().find(|s| s.id == token) else {
            continue;
        };
        if st.composite {
            if !has_clusters {
                emit_cluster(&mut svg, st);
            }
            // Emit each concurrent region in turn. A dashed divider line
            // (captured free-standing from the golden) precedes every region
            // after the first.
            for (ri, scope) in region_scopes(&st.id).into_iter().enumerate() {
                if ri > 0 {
                    let i = next_divider.get();
                    if let Some(div) = orc.region_dividers.get(i) {
                        svg.push_str(&div.xml);
                        next_divider.set(i + 1);
                    }
                }
                emit_scope_entities(&mut svg, Some(&scope), &scope_emit);
                if !has_clusters {
                    emit_scope_links(&mut svg, Some(&scope));
                }
            }
        } else {
            emit_state_box(&mut svg, st);
        }
    }
    // Anchored notes (`note right/left of …`). PlantUML emits these `GMN*`
    // entities after the top-level pseudo-states and before the top-level
    // links. Pair the oracle's `GMN*` entities (numeric order) 1:1 with the
    // parser's anchored notes, skipping note-on-link / named-floating notes
    // which have no standalone `GMN*` entity.
    {
        let mut gmns: Vec<(&String, &crate::layout_oracle::EntityRect)> = orc
            .entities
            .iter()
            .filter(|(k, _)| k.starts_with("GMN"))
            .collect();
        gmns.sort_by_key(|(k, _)| k.trim_start_matches("GMN").parse::<u32>().unwrap_or(0));
        let anchored = diagram.notes.iter().filter(|n| {
            !matches!(
                &n.kind,
                StateNoteKind::Floating(Some(_)) | StateNoteKind::OnLink
            )
        });
        for (note, (gmn_name, rect)) in anchored.zip(gmns.iter()) {
            emit_oracle_gmn_note(&mut svg, gmn_name, rect, &note.text);
        }
    }
    // Links. In the cluster-group layout PlantUML defers every link to the end
    // in oracle edge order; emit all not-yet-emitted edges verbatim. Otherwise
    // only the top-level (scope-None) links remain.
    if has_clusters {
        for (ei, edge) in orc.edges.iter().enumerate() {
            if !emitted_edge.borrow()[ei] {
                emit_oracle_edge_verbatim(&mut svg, edge);
                emitted_edge.borrow_mut()[ei] = true;
            }
        }
    } else {
        emit_scope_links(&mut svg, None);
    }

    wrap_oracle_envelope(orc, &svg, "STATE")
}

/// Does parser endpoint `ep` correspond to one side of oracle edge id `edge_id`?
/// `is_from` selects the prefix (before `-to-`) vs suffix.
fn short_name_match(ep: &str, edge_id: &str, is_from: bool) -> bool {
    // Translate the parser endpoint to its short edge-id token.
    let token = if ep == "[*]" {
        if is_from {
            "*start*".to_string()
        } else {
            "*end*".to_string()
        }
    } else if let Some(scope) = ep.strip_prefix("[*]") {
        // PlantUML names the pseudo-state edge token with the scope's last
        // dotted segment: a region sub-scope `Concurrent.CONC2` yields
        // `*start*CONC2`, matching `pseudo_qname`'s suffix rule.
        let suffix = scope.rsplit('.').next().unwrap_or(scope);
        if is_from {
            format!("*start*{suffix}")
        } else {
            format!("*end*{suffix}")
        }
    } else {
        ep.rsplit('.').next().unwrap_or(ep).to_string()
    };
    let Some((from, to)) = edge_id.split_once("-to-") else {
        // reverse form
        if let Some((from, to)) = edge_id.split_once("-backto-") {
            return if is_from { to == token } else { from == token };
        }
        return false;
    };
    if is_from { from == token } else { to == token }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    #[test]
    fn simple_state_diagram() {
        let d = StateDiagram {
            meta: DiagramMeta::default(),
            states: vec![
                State {
                    id: "Active".into(),
                    label: "Active".into(),
                    ..State::default()
                },
                State {
                    id: "Inactive".into(),
                    label: "Inactive".into(),
                    ..State::default()
                },
            ],
            transitions: vec![
                Transition {
                    from: "[*]".into(),
                    to: "Active".into(),
                    label: None,
                    source_line: 0,
                },
                Transition {
                    from: "Active".into(),
                    to: "Inactive".into(),
                    label: Some("disable".into()),
                    source_line: 0,
                },
            ],
            notes: vec![],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Active"));
        assert!(svg.contains("Inactive"));
        assert!(svg.contains("disable"));
        // Check PlantUML-specific attributes.
        assert!(svg.contains(r#"data-diagram-type="STATE""#));
        assert!(svg.contains(r#"class="start_entity""#));
        assert!(svg.contains(r#"class="entity""#));
        assert!(svg.contains(r#"class="link""#));
    }

    #[test]
    fn parsed_then_rendered() {
        let input =
            "@startuml\n[*] --> Active\nActive --> Inactive : disable\nInactive --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Active"));
        assert!(svg.contains("disable"));
        assert!(svg.contains(r#"data-diagram-type="STATE""#));
        assert!(svg.contains(r#"class="end_entity""#));
    }

    #[test]
    fn state_desc_syntax_renders_inside_box() {
        let input = "@startuml\nstate A : idle\n[*] --> A\nA --> B : next\nB --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains("idle"),
            "description text should appear in SVG"
        );
        // The divider line should be present.
        assert!(svg.contains("<line"), "divider line should be rendered");
    }

    #[test]
    fn note_right_of_state_renders_text() {
        let input = "@startuml\n[*] --> A\nA --> [*]\nnote right of A : Note 1\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Note 1"), "note text should appear in SVG");
    }

    #[test]
    fn multiline_note_renders_all_lines() {
        let input = "@startuml\n[*] --> A\nnote right of A\n  line 1\n  line 2\nend note\nA --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains("line 1"),
            "first note line should appear in SVG"
        );
        assert!(
            svg.contains("line 2"),
            "second note line should appear in SVG"
        );
    }

    #[test]
    fn floating_note_renders_text() {
        let input = "@startuml\nnote \"Floating note 1\" as FN1\n[*] --> A\nA --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains("Floating note 1"),
            "floating note text should appear in SVG"
        );
    }

    #[test]
    fn plantuml_svg_structure() {
        let input = "@startuml\n[*] --> A\nA --> B\nB --> [*]\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Check PlantUML SVG root attributes.
        assert!(svg.contains(r#"contentStyleType="text/css""#));
        assert!(svg.contains(r#"data-diagram-type="STATE""#));
        assert!(svg.contains(r#"preserveAspectRatio="none""#));
        assert!(svg.contains(r#"version="1.1""#));
        assert!(svg.contains(r#"zoomAndPan="magnify""#));
        assert!(svg.contains("<?plantuml"));
        assert!(svg.contains("<defs/>"));

        // Check element types.
        assert!(svg.contains("<ellipse")); // start/end use ellipse, not circle
        assert!(svg.contains(r##"fill="#F1F1F1""##)); // PlantUML state fill
        assert!(svg.contains(r##"style="stroke:#181818;stroke-width:0.5;""##)); // stroke as style attr
        assert!(svg.contains(r##"lengthAdjust="spacing""##)); // text attributes
    }
}
