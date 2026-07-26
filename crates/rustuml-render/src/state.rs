// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State diagram SVG renderer.
//!
//! Produces PlantUML-compatible SVG output with matching element structure,
//! attributes, and styling.

use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgeLabelSize, EdgePath, LayoutGraph, NodePosition};
use rustuml_parser::diagram::state::*;

use crate::handwritten::has_deprecated_skinparam as has_deprecated_handwritten_skinparam;
use crate::layout_oracle::{
    EntityPath, EntityPolygon, EntityRect, OracleEdgePath, OracleHandwrittenWarning, OracleLayout,
    wrap_oracle_envelope,
};
use crate::style::Theme;
use crate::text_render::{self, TextBase};

#[derive(Debug)]
struct StateGradient {
    color1: String,
    color2: String,
    policy: char,
    id: String,
}

fn split_state_gradient(value: &str) -> Option<(&str, &str, char)> {
    for policy in ['-', '\\', '|', '/'] {
        if let Some((color1, color2)) = value.split_once(policy) {
            let color1 = color1.trim();
            let color2 = color2.trim();
            if !color1.is_empty() && !color2.is_empty() {
                return Some((color1, color2, policy));
            }
        }
    }
    None
}

fn state_gradients(diagram: &StateDiagram) -> Vec<StateGradient> {
    let source = diagram.meta.source.as_deref().unwrap_or("");
    let mut gradients: Vec<StateGradient> = Vec::new();
    for state in &diagram.states {
        let Some((raw1, raw2, policy)) = state.fill.as_deref().and_then(split_state_gradient)
        else {
            continue;
        };
        let color1 = crate::sequence::resolve_color(raw1);
        let color2 = crate::sequence::resolve_color(raw2);
        if gradients.iter().any(|gradient| {
            gradient.color1 == color1 && gradient.color2 == color2 && gradient.policy == policy
        }) {
            continue;
        }
        let id = crate::filter_registry::gradient_id_for(source, gradients.len());
        gradients.push(StateGradient {
            color1,
            color2,
            policy,
            id,
        });
    }
    gradients
}

fn state_gradient_fill(value: &str, gradients: &[StateGradient]) -> String {
    let Some((raw1, raw2, policy)) = split_state_gradient(value) else {
        return crate::sequence::resolve_color(value);
    };
    let color1 = crate::sequence::resolve_color(raw1);
    let color2 = crate::sequence::resolve_color(raw2);
    gradients
        .iter()
        .find(|gradient| {
            gradient.color1 == color1 && gradient.color2 == color2 && gradient.policy == policy
        })
        .map_or(color1, |gradient| format!("url(#{})", gradient.id))
}

fn emit_state_gradient_defs(svg: &mut String, gradients: &[StateGradient]) {
    for gradient in gradients {
        // Java provenance: `SvgGraphics.createSvgGradient` maps the
        // HColorGradient separator policy to these four endpoint pairs. The
        // DOM serializer emits attributes alphabetically.
        let (x1, x2, y1, y2) = match gradient.policy {
            '|' => ("0%", "100%", "50%", "50%"),
            '\\' => ("0%", "100%", "100%", "0%"),
            '-' => ("50%", "50%", "0%", "100%"),
            _ => ("0%", "100%", "0%", "100%"),
        };
        write!(
            svg,
            r#"<linearGradient id="{}" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}"><stop offset="0%" stop-color="{}"/><stop offset="100%" stop-color="{}"/></linearGradient>"#,
            gradient.id, gradient.color1, gradient.color2,
        )
        .unwrap();
    }
}

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
/// Shared dimension delta around the merged state title/body text.
///
/// Java provenance: `EntityImageState.calculateDimensionSlow` applies
/// `MARGIN * 2 + 2 * MARGIN_LINE`, where both constants are five pixels.
const STATE_DIMENSION_PADDING: f64 = 20.0;
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
/// State SVEK body origin after PlantUML shifts the laid-out graph.
///
/// Java provenance: `net.sourceforge.plantuml.svek.SvekResult.calculateDimension`
/// calls `clusterManager.moveDelta(6 - minX, 6 - minY)` and then returns
/// `minMax.getDimension().delta(15, 15)`. `TextBlockUtils.getMinMax` includes
/// the state rectangle's horizontal stroke overscan but the start circle's
/// vertical bound is already integral, yielding visible x=7 and y=6 origins.
/// The remaining 14px of the dimension delta trails the painted graph.
const SVEK_ORIGIN_X: f64 = 7.0;
const SVEK_ORIGIN_Y: f64 = 6.0;
const SVEK_TRAILING_PAD: f64 = 14.0;
/// Extra width after the right edge of a painted autonomous edge label.
///
/// Java provenance: `SvekResult.calculateDimension` takes the painted
/// `LimitFinder` span and adds 15px; the translated graph's painted minimum is
/// x=5, so a rightmost text edge contributes `right - 5 + 15`.
const AUTONOMOUS_LABEL_TRAILING: f64 = 10.0;
/// Rank separation used by the autonomous image builder inside a composite.
///
/// Java provenance: `DotStringFactory.getMinRankSep()` falls back to dot's
/// 36-pixel default when `InnerStateAutonom` creates its independent builder.
const AUTONOMOUS_RANK_SEP: f64 = 36.0;

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
/// Java `ExtremityArrow.getDecorationLength` retracts the final Bezier endpoint
/// and control point by six pixels while leaving the arrow tip at dot's solved
/// node-boundary contact.
const ARROW_DECORATION_LENGTH: f64 = 6.0;

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
    fn from_next(counter: usize) -> Self {
        Self {
            entity_counter: counter,
            link_counter: counter,
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

fn is_history_marker(id: &str) -> bool {
    id == "[H]" || id == "[H*]"
}

fn history_marker_label(id: &str) -> &'static str {
    if id == "[H*]" { "H*" } else { "H" }
}

/// Compute the width of a state box based on its label and descriptions.
fn state_box_width(label: &str, descriptions: &[String]) -> f64 {
    let label_w = text_render::measure(label, STATE_FONT_SIZE, false) + STATE_DIMENSION_PADDING;
    let desc_w = descriptions
        .iter()
        .map(|d| text_render::measure(d, DESC_FONT_SIZE, false) + STATE_DIMENSION_PADDING)
        .fold(0.0_f64, f64::max);
    label_w.max(desc_w).max(STATE_MIN_WIDTH)
}

/// Compute the height of a state box given its number of description lines.
fn state_box_height(desc_count: usize) -> f64 {
    let content_height = crate::plantuml_metrics::text_height(STATE_FONT_SIZE)
        + desc_count as f64 * crate::plantuml_metrics::text_height(DESC_FONT_SIZE)
        + STATE_DIMENSION_PADDING;
    content_height.max(STATE_BOX_HEIGHT)
}

/// Node height for layout purposes.
fn node_height(id: &str, state_def: Option<&State>, hide_empty_desc: bool) -> f64 {
    if is_history_marker(id) {
        END_OUTER_RADIUS * 2.0
    } else if is_pseudo_state(id) {
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
    if is_history_marker(id) {
        END_OUTER_RADIUS * 2.0
    } else if is_pseudo_state(id) {
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

/// Dimensions and Graphviz shape used for one flat state node.
///
/// Java provenance: `EntityImageCircleStart.calculateDimensionSlow` delegates
/// to the 20px `CircleStart`; `EntityImageCircleEnd.calculateDimensionSlow`
/// returns 22x22. `SvekNode.appendShape` sends both to dot as circles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StateLayoutShape {
    Box,
    Circle,
    Diamond,
}

fn layout_node_size(
    id: &str,
    state_def: Option<&State>,
    hide_empty_desc: bool,
) -> (f64, f64, StateLayoutShape) {
    if id == "__start__" || id.starts_with("__start__:") {
        return (
            START_RADIUS * 2.0,
            START_RADIUS * 2.0,
            StateLayoutShape::Circle,
        );
    }
    if id == "__end__" || id.starts_with("__end__:") {
        return (
            END_OUTER_RADIUS * 2.0,
            END_OUTER_RADIUS * 2.0,
            StateLayoutShape::Circle,
        );
    }
    let shape = match state_def.map(|state| state.kind) {
        Some(StateKind::Initial | StateKind::Final) => StateLayoutShape::Circle,
        Some(StateKind::Choice) => StateLayoutShape::Diamond,
        _ => StateLayoutShape::Box,
    };
    (
        node_width(id, state_def),
        node_height(id, state_def, hide_empty_desc),
        shape,
    )
}

/// Register a State entity with Graphviz using PlantUML's SVEK shape.
///
/// Java provenance: `SvekNode.appendShapeInternal` maps
/// `ShapeType.DIAMOND` to Graphviz's native `shape=diamond`, so incident
/// splines clip against the diagonal boundary rather than its bounding box.
fn add_state_layout_node(
    layout: &mut LayoutGraph,
    id: &str,
    width: f64,
    height: f64,
    shape: StateLayoutShape,
) {
    match shape {
        StateLayoutShape::Box => {
            layout.add_node(id, id, width, height);
        }
        StateLayoutShape::Circle => {
            layout.add_circle_node(id, id, width.max(height));
        }
        StateLayoutShape::Diamond => {
            layout.add_diamond_node(id, id, width, height);
        }
    }
}

/// Return the vertical translation applied by `SvekResult.calculateDimension`.
///
/// `TextBlockUtils.getMinMax` delegates rectangles to
/// `LimitFinder.drawRectangle`, whose top bound is `y - 1`, while ellipses use
/// their unexpanded `y`. Consequently, a state box on Graphviz's top rank
/// moves the whole SVEK body to y=7; a start/end ellipse on that rank moves it
/// to y=6.
fn svek_origin_y_for_layout(
    diagram: &StateDiagram,
    state_ids: &[String],
    positions: &[NodePosition],
) -> f64 {
    let Some(top) = positions
        .iter()
        .map(|position| quantize_svek_coord(position.y))
        .reduce(f64::min)
    else {
        return SVEK_ORIGIN_Y;
    };

    let rectangle_on_top_rank = state_ids.iter().zip(positions).any(|(id, position)| {
        if (quantize_svek_coord(position.y) - top).abs() > f64::EPSILON {
            return false;
        }
        let state = diagram.states.iter().find(|state| state.id == *id);
        state.is_some_and(|state| {
            matches!(
                state.kind,
                StateKind::Normal | StateKind::Fork | StateKind::Join
            )
        })
    });

    SVEK_ORIGIN_Y + f64::from(rectangle_on_top_rank)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExplicitTransitionDirection {
    Up,
    Down,
    Left,
    Right,
}

fn transition_source_text<'a>(
    diagram: &'a StateDiagram,
    transition: &Transition,
) -> Option<&'a str> {
    let source = diagram.meta.source.as_deref()?;
    let marker_offset = source
        .lines()
        .next()
        .is_some_and(|line| line.trim_start().starts_with("@start"))
        as usize;
    let source_index = transition
        .source_line
        .checked_sub(1)?
        .checked_add(marker_offset)?;
    source.lines().nth(source_index)
}

/// Recover the named direction carried by an explicit state transition arrow.
///
/// Java provenance: `CommandLinkStateCommon.getDirection` passes
/// `ARROW_DIRECTION` through `StringUtils.getQueueDirection`.
fn explicit_transition_direction(
    diagram: &StateDiagram,
    transition: &Transition,
) -> Option<ExplicitTransitionDirection> {
    let lowercase = transition_source_text(diagram, transition)?.to_ascii_lowercase();
    [
        ("-left", ExplicitTransitionDirection::Left),
        ("-right", ExplicitTransitionDirection::Right),
        ("-up", ExplicitTransitionDirection::Up),
        ("-down", ExplicitTransitionDirection::Down),
    ]
    .into_iter()
    .find_map(|(token, direction)| lowercase.contains(token).then_some(direction))
}

/// Recover the Graphviz rank length carried by a vertical transition arrow.
///
/// The state parser deliberately keeps exact arrow syntax in
/// `DiagramMeta::source` for downstream consumers. Java's
/// `CommandLinkStateCommon.executeArg` concatenates `ARROW_BODY1` and
/// `ARROW_BODY2` into the link queue, while `SvekEdge.appendLine` serialises
/// `minlen = Link.getLength() - 1`. Direction and style tokens sit between
/// those two dash runs, so counting the dashes recovers the same queue length.
fn transition_svek_minlen(diagram: &StateDiagram, transition: &Transition) -> Option<usize> {
    if matches!(
        explicit_transition_direction(diagram, transition),
        Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Right)
    ) {
        return Some(0);
    }
    let line = transition_source_text(diagram, transition)?;
    let arrow_clause = line.split_once(" :").map_or(line, |(arrow, _)| arrow);
    let queue_len = arrow_clause.bytes().filter(|byte| *byte == b'-').count();
    queue_len.checked_sub(1)
}

/// PlantUML parses Graphviz's SVG, whose node and spline coordinates are
/// serialized to two decimal places, before `SvekNode`/`SvekEdge` paint them.
fn quantize_svek_coord(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
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
    } else if let Some(font_name) = font_name {
        if font_name
            .trim_matches(|c| c == '"' || c == '\'')
            .eq_ignore_ascii_case("Arial")
        {
            arial_text_width(text, font_size, bold)
        } else {
            text_render::measure_with_family(text, font_size, bold, font_name)
        }
    } else {
        text_render::measure(text, font_size, bold)
    }
}

fn canonical_state_font_family(value: &str) -> String {
    let raw = value.trim();
    let quoted = (raw.starts_with('"') && raw.ends_with('"'))
        || (raw.starts_with('\'') && raw.ends_with('\''));
    let trimmed = raw.trim_matches('"').trim_matches('\'');
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("sansserif")
        || trimmed.eq_ignore_ascii_case("sans-serif")
    {
        "sans-serif".to_string()
    } else if quoted {
        format!("'{trimmed}'")
    } else {
        trimmed.to_string()
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

fn emit_entity_polygon(svg: &mut String, polygon: &EntityPolygon) {
    write!(
        svg,
        r#"<polygon fill="{}" points="{}""#,
        escape_attr(&polygon.fill),
        escape_attr(&polygon.points),
    )
    .unwrap();
    if let Some(style) = polygon.style.as_deref() {
        write!(svg, r#" style="{}""#, escape_attr(style)).unwrap();
    }
    svg.push_str("/>");
}

fn emit_entity_path(svg: &mut String, path: &EntityPath) {
    write!(
        svg,
        r#"<path d="{}" fill="{}""#,
        escape_attr(&path.d),
        escape_attr(&path.fill),
    )
    .unwrap();
    if let Some(style) = path.style.as_deref() {
        write!(svg, r#" style="{}""#, escape_attr(style)).unwrap();
    }
    svg.push_str("/>");
}

fn edge_path_id_attr(edge: &OracleEdgePath) -> String {
    edge.path_id
        .as_deref()
        .map(|id| format!(r#" id="{}""#, escape_attr(id)))
        .unwrap_or_default()
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
    // A pseudo-state first seen as a transition endpoint may later be upgraded
    // by an explicit `state X <<choice/fork/join>>` declaration. PlantUML's
    // explicit pseudo declaration is emitted before the lazily-created `.start.`
    // / `.end.` nodes, even when the transition line appears first.
    let mut promoted_pseudo: Vec<&State> = diagram
        .states
        .iter()
        .filter(|s| {
            s.decl_line.is_some_and(|decl| s.source_line < decl)
                && !matches!(s.kind, StateKind::Normal)
        })
        .collect();
    promoted_pseudo.sort_by_key(|s| s.decl_line.unwrap_or(s.source_line));
    for s in promoted_pseudo {
        push_entity(0, s.id.clone());
    }
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

struct StateSvgIds {
    entity_ids: Vec<(String, String)>,
    link_ids: Vec<String>,
    next_counter: usize,
}

/// Map one parsed transition endpoint to its renderer-owned layout identity.
///
/// Composite-local `[*]` references arrive from the parser as `[*]Outer`.
/// They still denote two distinct entities: the source occurrence is
/// `Outer..start.Outer`, while the target occurrence is
/// `Outer..end.Outer`.
fn state_endpoint_layout_id(id: &str, is_source: bool) -> String {
    if let Some(scope) = id.strip_prefix("[*]") {
        let kind = if is_source { "__start__" } else { "__end__" };
        if scope.is_empty() {
            kind.to_string()
        } else {
            format!("{kind}:{scope}")
        }
    } else {
        id.to_string()
    }
}

/// Allocate state entity/link ids in PlantUML's construction order.
///
/// Java provenance: `CucaDiagram.startingPass` resets `cpt1` to one for every
/// parser pass. `CommandCreateState.executeArg` creates explicit leaves in pass
/// one. In pass two, those leaves already exist, while
/// `CommandLinkStateCommon.executeArg` lazily creates missing endpoints and
/// then constructs each `Link`. The reset deliberately permits an explicit
/// state and a lazy start/end state to share an `entNNNN` value.
fn allocate_state_svg_ids(diagram: &StateDiagram, state_ids: &[String]) -> StateSvgIds {
    let mut entity_ids: Vec<(String, String)> = Vec::new();
    let mut link_ids = vec![String::new(); diagram.transitions.len()];

    let mut declarations: Vec<(usize, usize, &str)> = Vec::new();
    for (idx, state) in diagram.states.iter().enumerate() {
        if let Some(line) = state.decl_line {
            declarations.push((line, idx, state.id.as_str()));
        }
    }
    declarations.sort_by_key(|(line, seq, _)| (*line, *seq));
    let mut pass_one_counter = 2usize;
    for (_, _, id) in declarations {
        if state_ids.iter().any(|state_id| state_id == id)
            && !entity_ids.iter().any(|(seen, _)| seen == id)
        {
            entity_ids.push((id.to_string(), format!("ent{pass_one_counter:04}")));
            pass_one_counter += 1;
        }
    }

    let mut pass_two_counter = 2usize;
    let mut transitions: Vec<(usize, &Transition)> =
        diagram.transitions.iter().enumerate().collect();
    transitions.sort_by_key(|(idx, transition)| (transition.source_line, *idx));
    for (idx, transition) in transitions {
        for (endpoint, is_source) in [(&transition.from, true), (&transition.to, false)] {
            let mapped_id = state_endpoint_layout_id(endpoint, is_source);
            let id = if state_ids.iter().any(|state_id| state_id == &mapped_id) {
                mapped_id
            } else {
                endpoint.clone()
            };
            if state_ids.iter().any(|state_id| state_id == &id)
                && !entity_ids.iter().any(|(seen, _)| seen == &id)
            {
                entity_ids.push((id, format!("ent{pass_two_counter:04}")));
                pass_two_counter += 1;
            }
        }
        link_ids[idx] = format!("lnk{pass_two_counter}");
        pass_two_counter += 1;
        // `CommandLinkStateCommon.executeArg` first constructs the forward
        // `Link`, then `Link.getInv()` constructs the stored reverse link for
        // left/up arrows. Both consume a UID from the pass-two counter.
        if matches!(
            explicit_transition_direction(diagram, transition),
            Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Up)
        ) {
            pass_two_counter += 1;
            link_ids[idx] = format!("lnk{}", pass_two_counter - 1);
        }
    }

    for id in state_ids {
        if !entity_ids.iter().any(|(seen, _)| seen == id) {
            entity_ids.push((id.clone(), format!("ent{pass_two_counter:04}")));
            pass_two_counter += 1;
        }
    }
    for id in link_ids.iter_mut().filter(|id| id.is_empty()) {
        *id = format!("lnk{pass_two_counter}");
        pass_two_counter += 1;
    }

    StateSvgIds {
        entity_ids,
        link_ids,
        next_counter: pass_two_counter,
    }
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
    /// Root style line colour from modern themes, used by pseudo-state chrome.
    root_line_color: Option<String>,
    /// Theme/skinparam colour for the start pseudo-state, when specified.
    start_color: Option<String>,
    /// Theme/skinparam colour for the end pseudo-state, when specified.
    end_color: Option<String>,
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
        let root_line_color = color("__styleRootLineColor");
        let stroke = color("stateBorderColor")
            .or_else(|| root_line_color.clone())
            .unwrap_or_else(|| DEFAULT_STROKE_COLOR.to_string());
        let border_thickness = find("stateBorderThickness")
            .or_else(|| find("__styleRootLineThickness"))
            .or_else(|| find("borderThickness"))
            .and_then(|v| v.parse::<f64>().ok())
            .map(fmt_f)
            .unwrap_or_else(|| "0.5".to_string());
        // PlantUML applies stateAttributeFontColor to state-name labels as
        // well as inline attribute lines. Modern themes define both through
        // `$primary_scheme()` and then override AttributeFontColor for state
        // labels, so the attribute colour wins when present.
        let text_color = color("stateAttributeFontColor")
            .or_else(|| color("__styleRootFontColor"))
            .or_else(|| color("stateFontColor"))
            .or_else(|| color("defaultFontColor"))
            .unwrap_or_else(|| DEFAULT_TEXT_COLOR.to_string());
        let state_fill =
            color("stateBackgroundColor").unwrap_or_else(|| DEFAULT_STATE_FILL.to_string());
        let arrow_color = color("stateArrowColor").unwrap_or_else(|| stroke.clone());
        let start_color = color("stateStartColor");
        let end_color = color("stateEndColor");
        Self {
            stroke,
            border_thickness,
            text_color,
            state_fill,
            arrow_color,
            root_line_color,
            start_color,
            end_color,
        }
    }
}

struct StateArrowFont {
    color: String,
    family: String,
    size: u32,
    bold: bool,
    italic: bool,
}

impl StateArrowFont {
    fn from_diagram(diagram: &StateDiagram) -> Self {
        let find = |keys: &[&str]| {
            diagram
                .meta
                .skinparams
                .iter()
                .rev()
                .find(|sp| keys.iter().any(|key| sp.key.eq_ignore_ascii_case(key)))
        };
        let color = find(&["stateArrowFontColor", "arrowFontColor"])
            .map(|sp| crate::sequence::resolve_color(sp.value.trim()))
            .unwrap_or_else(|| DEFAULT_TEXT_COLOR.to_string());
        let family = find(&[
            "stateArrowFontName",
            "arrowFontName",
            "defaultFontName",
            "fontName",
        ])
        .map(|sp| canonical_state_font_family(sp.value.trim()))
        .unwrap_or_else(|| "sans-serif".to_string());
        let size = find(&["stateArrowFontSize", "arrowFontSize", "defaultFontSize"])
            .and_then(|sp| sp.value.trim().parse::<u32>().ok())
            .unwrap_or(LINK_FONT_SIZE as u32);
        let style = find(&["stateArrowFontStyle", "arrowFontStyle"])
            .map(|sp| sp.value.to_ascii_lowercase())
            .unwrap_or_default();
        Self {
            color,
            family,
            size,
            bold: style.contains("bold"),
            italic: style.contains("italic"),
        }
    }
}

#[derive(Clone)]
struct AutonomousScopeLayout {
    ids: Vec<String>,
    positions: Vec<(String, f64, f64, f64, f64)>,
    edge_paths: Vec<EdgePath>,
    transition_indices: Vec<usize>,
    origin_x: f64,
    origin_y: f64,
    width: f64,
    height: f64,
}

struct AutonomousComposite<'a> {
    state: &'a State,
    inner: AutonomousScopeLayout,
    attribute_height: f64,
    field_margin: f64,
    width: f64,
    height: f64,
}

struct AutonomousRenderContext<'a> {
    diagram: &'a StateDiagram,
    entity_ids: &'a [(String, String)],
    allocated_ids: &'a StateSvgIds,
    skin: &'a StateSkin,
    arrow_font: &'a StateArrowFont,
}

fn endpoint_is_in_composite(diagram: &StateDiagram, endpoint: &str, scope: &str) -> bool {
    endpoint
        .strip_prefix("[*]")
        .is_some_and(|pseudo_scope| pseudo_scope == scope)
        || diagram
            .states
            .iter()
            .find(|state| state.id == endpoint)
            .and_then(|state| state.parent.as_deref())
            == Some(scope)
}

fn collect_autonomous_scope_ids<F>(
    diagram: &StateDiagram,
    transition_indices: &[usize],
    include_state: F,
) -> Vec<String>
where
    F: Fn(&State) -> bool,
{
    let mut ids = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut declared: Vec<(usize, usize, &State)> = diagram
        .states
        .iter()
        .enumerate()
        .filter(|(_, state)| include_state(state))
        .filter_map(|(index, state)| state.decl_line.map(|line| (line, index, state)))
        .collect();
    declared.sort_by_key(|(line, index, _)| (*line, *index));
    for (_, _, state) in declared {
        if seen.insert(state.id.clone()) {
            ids.push(state.id.clone());
        }
    }

    let mut ordered_transitions = transition_indices.to_vec();
    ordered_transitions.sort_by_key(|index| (diagram.transitions[*index].source_line, *index));
    for index in ordered_transitions {
        let transition = &diagram.transitions[index];
        for (endpoint, is_source) in [(&transition.from, true), (&transition.to, false)] {
            let id = state_endpoint_layout_id(endpoint, is_source);
            if seen.insert(id.clone()) {
                ids.push(id);
            }
        }
    }

    for state in &diagram.states {
        if include_state(state) && seen.insert(state.id.clone()) {
            ids.push(state.id.clone());
        }
    }
    ids
}

fn layout_autonomous_scope(
    diagram: &StateDiagram,
    ids: Vec<String>,
    transition_indices: Vec<usize>,
    node_sizes: &[(String, f64, f64, StateLayoutShape)],
    arrow_font: &StateArrowFont,
    plantuml_spacing: bool,
) -> Option<AutonomousScopeLayout> {
    let mut layout = LayoutGraph::new(Direction::TopToBottom);
    if plantuml_spacing {
        layout = layout.with_plantuml_svek_spacing();
    } else {
        layout = layout.with_spacing_pixels(
            rustuml_layout::graph::GraphSpacing::PLANTUML_SVEK_DEFAULTS.node_sep_px,
            AUTONOMOUS_RANK_SEP,
        );
    }
    for id in &ids {
        let (_, width, height, shape) = node_sizes.iter().find(|entry| &entry.0 == id)?;
        add_state_layout_node(&mut layout, id, *width, *height, *shape);
    }
    for index in &transition_indices {
        let transition = &diagram.transitions[*index];
        let from = state_endpoint_layout_id(&transition.from, true);
        let to = state_endpoint_layout_id(&transition.to, false);
        let direction = explicit_transition_direction(diagram, transition);
        let (layout_from, layout_to) = if matches!(
            direction,
            Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Up)
        ) {
            (&to, &from)
        } else {
            (&from, &to)
        };
        let horizontal = matches!(
            direction,
            Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Right)
        );
        if matches!(
            direction,
            Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Up)
        ) {
            // `CommandLinkStateCommon.executeArg` calls `Link.getInv` for
            // LEFT/UP transitions. `Cluster.getNodesOrderedTop` emits that
            // inverted link's start before ordinary SVEK nodes.
            layout.add_plantuml_svek_inverted_start(layout_from);
        }
        if horizontal {
            // Horizontal State arrows have queue length one. PlantUML emits
            // those `lines0` edges before ordinary nodes, without forcing a
            // rank-same block under the default `SkinParam.useRankSame`.
            layout.add_plantuml_svek_line0_edge(layout_from, layout_to);
        }
        let label_size = transition.label.as_deref().map(|label| EdgeLabelSize {
            width: text_render::measure_with_family(
                label,
                arrow_font.size as f64,
                arrow_font.bold,
                &arrow_font.family,
            ) + 2.0,
            height: (text_render::label_height(label, arrow_font.size as f64) + 2.0).floor(),
        });
        layout.add_edge_with_label_sizes_and_minlen(
            layout_from,
            layout_to,
            label_size,
            None,
            None,
            transition_svek_minlen(diagram, transition),
        );
    }

    let result = layout.layout_full(std::time::Duration::from_secs(5))?;
    if result.node_positions.len() < ids.len() {
        return None;
    }
    let top = result
        .node_positions
        .iter()
        .map(|position| quantize_svek_coord(position.y))
        .reduce(f64::min)
        .unwrap_or(0.0);
    let rectangle_on_top = ids
        .iter()
        .zip(&result.node_positions)
        .any(|(id, position)| {
            (quantize_svek_coord(position.y) - top).abs() <= f64::EPSILON
                && node_sizes
                    .iter()
                    .find(|entry| &entry.0 == id)
                    .is_some_and(|entry| entry.3 == StateLayoutShape::Box)
        });
    let origin_x = SVEK_ORIGIN_X;
    let origin_y = SVEK_ORIGIN_Y + f64::from(rectangle_on_top);
    let mut positions = Vec::with_capacity(ids.len());
    let mut max_x = 0.0_f64;
    let mut max_y = 0.0_f64;
    for (id, position) in ids.iter().zip(&result.node_positions) {
        let (_, width, height, _) = node_sizes.iter().find(|entry| &entry.0 == id)?;
        let x = quantize_svek_coord(position.x);
        let y = quantize_svek_coord(position.y);
        positions.push((
            id.clone(),
            x + origin_x + width / 2.0,
            y + origin_y + height / 2.0,
            *width,
            *height,
        ));
        max_x = max_x.max(x + width);
        max_y = max_y.max(y + height);
    }
    if transition_indices
        .iter()
        .any(|index| diagram.transitions[*index].label.is_some())
    {
        max_x = max_x.max(result.width);
        max_y = max_y.max(result.height);
    }

    Some(AutonomousScopeLayout {
        ids,
        positions,
        edge_paths: result.edge_paths,
        transition_indices,
        origin_x,
        origin_y,
        width: origin_x + max_x + SVEK_TRAILING_PAD,
        height: origin_y + max_y + SVEK_TRAILING_PAD,
    })
}

/// Build the one-level autonomous state image used by
/// `GroupMakerState.getImage` and `InnerStateAutonom`.
///
/// This first generative slice deliberately accepts one unstyled composite
/// with direct normal-state children. Unsupported nested/concurrent/annotated
/// forms stay on the existing renderer until their own Java mechanism is
/// ported.
fn build_autonomous_composite<'a>(
    diagram: &'a StateDiagram,
    arrow_font: &StateArrowFont,
) -> Option<(AutonomousComposite<'a>, AutonomousScopeLayout)> {
    if !diagram.notes.is_empty()
        || diagram.meta.title.is_some()
        || !diagram.meta.skinparams.is_empty()
    {
        return None;
    }
    let composites: Vec<&State> = diagram
        .states
        .iter()
        .filter(|state| state.composite)
        .collect();
    let [composite] = composites.as_slice() else {
        return None;
    };
    if composite.parent.is_some()
        || composite.stereotype.is_some()
        || composite.stroke.is_some()
        || composite.url.is_some()
    {
        return None;
    }
    let children: Vec<&State> = diagram
        .states
        .iter()
        .filter(|state| state.parent.as_deref() == Some(composite.id.as_str()))
        .collect();
    if children.is_empty()
        || children.iter().any(|state| {
            state.composite
                || !matches!(state.kind, StateKind::Normal)
                || state.stereotype.is_some()
                || state.fill.is_some()
                || state.stroke.is_some()
                || state.url.is_some()
                || !state.descriptions.is_empty()
        })
        || diagram.states.iter().any(|state| {
            state.parent.is_some() && state.parent.as_deref() != Some(composite.id.as_str())
                || !matches!(state.kind, StateKind::Normal)
        })
    {
        return None;
    }

    let mut inner_transition_indices = Vec::new();
    let mut outer_transition_indices = Vec::new();
    for (index, transition) in diagram.transitions.iter().enumerate() {
        let from_inner = endpoint_is_in_composite(diagram, &transition.from, &composite.id);
        let to_inner = endpoint_is_in_composite(diagram, &transition.to, &composite.id);
        match (from_inner, to_inner) {
            (true, true) => inner_transition_indices.push(index),
            (false, false) => outer_transition_indices.push(index),
            _ => return None,
        }
    }
    if inner_transition_indices.is_empty() || outer_transition_indices.is_empty() {
        return None;
    }

    let inner_ids = collect_autonomous_scope_ids(diagram, &inner_transition_indices, |state| {
        state.parent.as_deref() == Some(composite.id.as_str())
    });
    let inner_sizes: Vec<(String, f64, f64, StateLayoutShape)> = inner_ids
        .iter()
        .map(|id| {
            let state = diagram.states.iter().find(|state| state.id == *id);
            let (width, height, shape) = layout_node_size(id, state, false);
            (id.clone(), width, height, shape)
        })
        .collect();
    // Java creates a fresh `GeneralImageBuilder` for the composite's children.
    // It uses dot's default 36px rank separation, not the outer SVEK minimum.
    let mut inner = layout_autonomous_scope(
        diagram,
        inner_ids,
        inner_transition_indices,
        &inner_sizes,
        arrow_font,
        false,
    )?;
    // `SvekResult.calculateDimension` measures the autonomous image from its
    // painted `MinMax`, rather than from the translated outer-canvas origin.
    // `LimitFinder.drawRectangle` contributes the two horizontal stroke-limit
    // pixels; the vertical ellipse bound needs no corresponding allowance.
    inner.width = inner.width - inner.origin_x + 2.0;
    inner.height -= inner.origin_y;
    for transition_index in &inner.transition_indices {
        let transition = &diagram.transitions[*transition_index];
        let Some(label) = transition.label.as_deref() else {
            continue;
        };
        let mut from = state_endpoint_layout_id(&transition.from, true);
        let mut to = state_endpoint_layout_id(&transition.to, false);
        if matches!(
            explicit_transition_direction(diagram, transition),
            Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Up)
        ) {
            std::mem::swap(&mut from, &mut to);
        }
        let Some(label_position) = inner
            .edge_paths
            .iter()
            .find(|edge| edge.from == from && edge.to == to)
            .and_then(|edge| edge.label)
        else {
            continue;
        };
        let label_right = quantize_svek_coord(label_position.x)
            + inner.origin_x
            + 1.0
            + text_render::measure_with_family(
                label,
                arrow_font.size as f64,
                arrow_font.bold,
                &arrow_font.family,
            );
        inner.width = inner.width.max(label_right + AUTONOMOUS_LABEL_TRAILING);
    }
    let title_height = crate::plantuml_metrics::text_height(STATE_FONT_SIZE);
    let attribute_height =
        composite.descriptions.len() as f64 * crate::plantuml_metrics::text_height(DESC_FONT_SIZE);
    let attribute_width = composite
        .descriptions
        .iter()
        .map(|description| text_render::measure(description, DESC_FONT_SIZE, false))
        .fold(0.0_f64, f64::max);
    let field_margin = if attribute_height > 0.0 { 5.0 } else { 0.0 };
    let width = inner
        .width
        .max(text_render::measure(
            &composite.label,
            STATE_FONT_SIZE,
            false,
        ))
        .max(attribute_width)
        + STATE_DIMENSION_PADDING
        + field_margin;
    let height =
        inner.height + title_height + attribute_height + STATE_DIMENSION_PADDING + field_margin;

    let outer_ids = collect_autonomous_scope_ids(diagram, &outer_transition_indices, |state| {
        state.parent.is_none()
    });
    let outer_sizes: Vec<(String, f64, f64, StateLayoutShape)> = outer_ids
        .iter()
        .map(|id| {
            if id == &composite.id {
                (id.clone(), width, height, StateLayoutShape::Box)
            } else {
                let state = diagram.states.iter().find(|state| state.id == *id);
                let (node_width, node_height, shape) = layout_node_size(id, state, false);
                (id.clone(), node_width, node_height, shape)
            }
        })
        .collect();
    let outer = layout_autonomous_scope(
        diagram,
        outer_ids,
        outer_transition_indices,
        &outer_sizes,
        arrow_font,
        true,
    )?;

    Some((
        AutonomousComposite {
            state: composite,
            inner,
            attribute_height,
            field_margin,
            width,
            height,
        },
        outer,
    ))
}

fn autonomous_entity_id<'a>(entity_ids: &'a [(String, String)], id: &str) -> &'a str {
    entity_ids
        .iter()
        .find(|(entity_id, _)| entity_id == id)
        .map(|(_, svg_id)| svg_id.as_str())
        .unwrap_or("ent0002")
}

fn autonomous_pseudo_name(id: &str) -> Option<String> {
    if id == "__start__" {
        Some(".start.".to_string())
    } else if id == "__end__" {
        Some(".end.".to_string())
    } else if let Some(scope) = id.strip_prefix("__start__:") {
        Some(format!("{scope}..start.{scope}"))
    } else {
        id.strip_prefix("__end__:")
            .map(|scope| format!("{scope}..end.{scope}"))
    }
}

fn autonomous_pseudo_source_line(diagram: &StateDiagram, id: &str) -> usize {
    diagram
        .transitions
        .iter()
        .find_map(|transition| {
            (state_endpoint_layout_id(&transition.from, true) == id
                || state_endpoint_layout_id(&transition.to, false) == id)
                .then_some(transition.source_line)
        })
        .unwrap_or(1)
}

fn autonomous_endpoint_display(diagram: &StateDiagram, endpoint: &str, is_source: bool) -> String {
    if let Some(scope) = endpoint.strip_prefix("[*]") {
        let pseudo = if is_source { "*start*" } else { "*end*" };
        let scope_label = diagram
            .states
            .iter()
            .find(|state| state.id == scope)
            .map(|state| state.label.as_str())
            .unwrap_or(scope);
        format!("{pseudo}{scope_label}")
    } else {
        diagram
            .states
            .iter()
            .find(|state| state.id == endpoint)
            .map(|state| state.label.clone())
            .unwrap_or_else(|| endpoint.to_string())
    }
}

fn emit_autonomous_scope_entities(
    svg: &mut String,
    context: &AutonomousRenderContext<'_>,
    scope: &AutonomousScopeLayout,
    offset: (f64, f64),
    skip_id: Option<&str>,
) {
    let (offset_x, offset_y) = offset;
    for (id, cx, cy, width, height) in &scope.positions {
        if skip_id == Some(id.as_str()) {
            continue;
        }
        let cx = cx + offset_x;
        let cy = cy + offset_y;
        if let Some(qualified_name) = autonomous_pseudo_name(id) {
            let source_line = autonomous_pseudo_source_line(context.diagram, id);
            if id.starts_with("__start__") {
                write!(
                    svg,
                    r#"<g class="start_entity" data-qualified-name="{qualified_name}" data-source-line="{source_line}" id="{}"><ellipse cx="{}" cy="{}" fill="{PSEUDO_COLOR}" rx="{START_RADIUS}" ry="{START_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/></g>"#,
                    autonomous_entity_id(context.entity_ids, id),
                    fmt_f(cx),
                    fmt_f(cy),
                )
                .unwrap();
            } else {
                write!(
                    svg,
                    r#"<g class="end_entity" data-qualified-name="{qualified_name}" data-source-line="{source_line}" id="{}"><ellipse cx="{}" cy="{}" fill="none" rx="{END_OUTER_RADIUS}" ry="{END_OUTER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/><ellipse cx="{}" cy="{}" fill="{PSEUDO_COLOR}" rx="{END_INNER_RADIUS}" ry="{END_INNER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/></g>"#,
                    autonomous_entity_id(context.entity_ids, id),
                    fmt_f(cx),
                    fmt_f(cy),
                    fmt_f(cx),
                    fmt_f(cy),
                )
                .unwrap();
            }
            continue;
        }

        let Some(state) = context.diagram.states.iter().find(|state| state.id == *id) else {
            continue;
        };
        let box_x = cx - width / 2.0;
        let box_y = cy - height / 2.0;
        let fill = state
            .fill
            .as_deref()
            .map(crate::sequence::resolve_color)
            .unwrap_or_else(|| context.skin.state_fill.clone());
        write!(
            svg,
            r#"<g class="entity" data-qualified-name="{}" id="{}"><rect fill="{fill}" height="{}" rx="{STATE_RX}" ry="{STATE_RX}" style="stroke:{};stroke-width:{};" width="{}" x="{}" y="{}"/><line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            escape_attr(&state.id),
            autonomous_entity_id(context.entity_ids, id),
            fmt_f(*height),
            context.skin.stroke,
            context.skin.border_thickness,
            fmt_f(*width),
            fmt_f(box_x),
            fmt_f(box_y),
            context.skin.stroke,
            context.skin.border_thickness,
            fmt_f(box_x),
            fmt_f(box_x + width),
            fmt_f(box_y + DIVIDER_OFFSET),
            fmt_f(box_y + DIVIDER_OFFSET),
        )
        .unwrap();
        let text_width = text_render::measure(&state.label, STATE_FONT_SIZE, false);
        let mut text = String::new();
        text_render::emit_text(
            &mut text,
            &state.label,
            &TextBase {
                x: cx - text_width / 2.0,
                y: box_y + NAME_BASELINE_OFFSET,
                font_size: STATE_FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: &context.skin.text_color,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.push_str(&text);
        svg.push_str("</g>");
    }
}

fn emit_autonomous_scope_links(
    svg: &mut String,
    context: &AutonomousRenderContext<'_>,
    scope: &AutonomousScopeLayout,
    offset: (f64, f64),
) {
    let (offset_x, offset_y) = offset;
    let pos_of = |id: &str| -> (f64, f64, f64, f64) {
        scope
            .positions
            .iter()
            .find(|(position_id, _, _, _, _)| position_id == id)
            .map(|(_, x, y, width, height)| (x + offset_x, y + offset_y, *width, *height))
            .unwrap_or((offset_x, offset_y, STATE_MIN_WIDTH, STATE_BOX_HEIGHT))
    };
    let mut consumed_edge_paths = vec![false; scope.edge_paths.len()];
    for transition_index in &scope.transition_indices {
        let transition = &context.diagram.transitions[*transition_index];
        let style = context.diagram.transition_style(*transition_index);
        let explicit_color = style.color.as_deref().map(crate::sequence::resolve_color);
        let color = explicit_color
            .as_deref()
            .unwrap_or(&context.skin.arrow_color);
        let stroke = transition_stroke_style(color, &style);
        let from = state_endpoint_layout_id(&transition.from, true);
        let to = state_endpoint_layout_id(&transition.to, false);
        let inverted = matches!(
            explicit_transition_direction(context.diagram, transition),
            Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Up)
        );
        let (edge_from, edge_to) = if inverted { (&to, &from) } else { (&from, &to) };
        let from_name = autonomous_endpoint_display(context.diagram, &transition.from, true);
        let to_name = autonomous_endpoint_display(context.diagram, &transition.to, false);
        let (edge_from_name, edge_to_name) = if inverted {
            (&to_name, &from_name)
        } else {
            (&from_name, &to_name)
        };
        if inverted {
            write!(
                svg,
                "<!--reverse link {edge_from_name} to {edge_to_name}-->"
            )
            .unwrap();
        } else {
            write!(svg, "<!--link {edge_from_name} to {edge_to_name}-->").unwrap();
        }
        write!(
            svg,
            r#"<g class="link" data-entity-1="{}" data-entity-2="{}" data-link-type="dependency" data-source-line="{}" id="{}">"#,
            autonomous_entity_id(context.entity_ids, edge_from),
            autonomous_entity_id(context.entity_ids, edge_to),
            transition.source_line,
            context
                .allocated_ids
                .link_ids
                .get(*transition_index)
                .map(String::as_str)
                .unwrap_or("lnk2"),
        )
        .unwrap();

        let edge_path = routed_edge_path_for_transition(
            &scope.edge_paths,
            &mut consumed_edge_paths,
            edge_from,
            edge_to,
            scope.origin_x + offset_x,
            scope.origin_y + offset_y,
            &pos_of,
        );
        if let Some(edge_path) = edge_path
            && !edge_path.points.is_empty()
        {
            let mut points: Vec<(f64, f64)> = edge_path
                .points
                .iter()
                .map(|(x, y)| {
                    (
                        quantize_svek_coord(*x) + scope.origin_x + offset_x,
                        quantize_svek_coord(*y) + scope.origin_y + offset_y,
                    )
                })
                .collect();
            let (arrow_control, arrow_tip) = if inverted {
                (points.get(1).copied().unwrap_or(points[0]), points[0])
            } else {
                (
                    points
                        .get(points.len().saturating_sub(2))
                        .copied()
                        .unwrap_or(points[0]),
                    points[points.len() - 1],
                )
            };
            if inverted {
                retract_dependency_arrow_path_start(&mut points);
            } else {
                retract_dependency_arrow_path(&mut points);
            }
            let mut path = format!("M{},{}", fmt_f(points[0].0), fmt_f(points[0].1));
            let mut point_index = 1;
            while point_index + 2 < points.len() {
                write!(
                    path,
                    " C{},{} {},{} {},{}",
                    fmt_f(points[point_index].0),
                    fmt_f(points[point_index].1),
                    fmt_f(points[point_index + 1].0),
                    fmt_f(points[point_index + 1].1),
                    fmt_f(points[point_index + 2].0),
                    fmt_f(points[point_index + 2].1),
                )
                .unwrap();
                point_index += 3;
            }
            write!(
                svg,
                r#"<path d="{path}" fill="none" id="{edge_from_name}-{}-{edge_to_name}" style="{stroke}"/>"#,
                if inverted { "backto" } else { "to" },
            )
            .unwrap();
            render_arrowhead(svg, arrow_control, arrow_tip, color);

            if let Some(label) = &transition.label {
                let (label_x, label_y) = edge_path
                    .label
                    .map(|position| {
                        (
                            quantize_svek_coord(position.x) + scope.origin_x + offset_x + 1.0,
                            quantize_svek_coord(position.y)
                                + scope.origin_y
                                + offset_y
                                + 1.0
                                + text_render::label_ascent(label, context.arrow_font.size as f64),
                        )
                    })
                    .unwrap_or_else(|| {
                        let first = points[0];
                        let last = points[points.len() - 1];
                        ((first.0 + last.0) / 2.0 + 1.0, (first.1 + last.1) / 2.0)
                    });
                let mut text = String::new();
                text_render::emit_text(
                    &mut text,
                    label,
                    &TextBase {
                        x: label_x,
                        y: label_y,
                        font_size: context.arrow_font.size,
                        font_family: &context.arrow_font.family,
                        fill: &context.arrow_font.color,
                        bold: context.arrow_font.bold,
                        italic: context.arrow_font.italic,
                        underline: false,
                        skip_underline: false,
                    },
                );
                svg.push_str(&text);
            }
        }
        svg.push_str("</g>");
    }
}

fn render_autonomous_composite(diagram: &StateDiagram) -> Option<String> {
    let skin = StateSkin::from_diagram(diagram);
    let arrow_font = StateArrowFont::from_diagram(diagram);
    let (composite, outer) = build_autonomous_composite(diagram, &arrow_font)?;
    let mut all_ids = outer.ids.clone();
    for id in &composite.inner.ids {
        if !all_ids.contains(id) {
            all_ids.push(id.clone());
        }
    }
    let allocated_ids = allocate_state_svg_ids(diagram, &all_ids);
    let context = AutonomousRenderContext {
        diagram,
        entity_ids: &allocated_ids.entity_ids,
        allocated_ids: &allocated_ids,
        skin: &skin,
        arrow_font: &arrow_font,
    };
    let width = outer.width.ceil() as i64;
    let height = outer.height.ceil() as i64;
    let mut svg = String::with_capacity(4096);
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="STATE" height="{height}px" preserveAspectRatio="none" style="width:{width}px;height:{height}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {width} {height}" width="{width}px" zoomAndPan="magnify"><?plantuml ?><defs/><g>"#,
    )
    .unwrap();

    let (_, composite_cx, composite_cy, _, _) = outer
        .positions
        .iter()
        .find(|(id, _, _, _, _)| id == &composite.state.id)?;
    let box_x = composite_cx - composite.width / 2.0;
    let box_y = composite_cy - composite.height / 2.0;
    let title_divider_y = box_y + DIVIDER_OFFSET;
    let header_divider_y = title_divider_y + composite.attribute_height + composite.field_margin;
    let right = box_x + composite.width;
    let header_fill = composite
        .state
        .fill
        .as_deref()
        .map(crate::sequence::resolve_color)
        .unwrap_or_else(|| skin.state_fill.clone());
    write!(
        svg,
        r#"<path d="M{},{} L{},{} A{STATE_RX},{STATE_RX} 0 0 1 {},{} L{},{} L{},{} L{},{} A{STATE_RX},{STATE_RX} 0 0 1 {},{}" fill="{}"/><rect fill="none" height="{}" rx="{STATE_RX}" ry="{STATE_RX}" style="stroke:{};stroke-width:{};" width="{}" x="{}" y="{}"/><line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
        fmt_f(box_x + STATE_RX),
        fmt_f(box_y),
        fmt_f(right - STATE_RX),
        fmt_f(box_y),
        fmt_f(right),
        fmt_f(box_y + STATE_RX),
        fmt_f(right),
        fmt_f(header_divider_y),
        fmt_f(box_x),
        fmt_f(header_divider_y),
        fmt_f(box_x),
        fmt_f(box_y + STATE_RX),
        fmt_f(box_x + STATE_RX),
        fmt_f(box_y),
        header_fill,
        fmt_f(composite.height),
        skin.stroke,
        skin.border_thickness,
        fmt_f(composite.width),
        fmt_f(box_x),
        fmt_f(box_y),
        skin.stroke,
        skin.border_thickness,
        fmt_f(box_x),
        fmt_f(right),
        fmt_f(header_divider_y),
        fmt_f(header_divider_y),
    )
    .unwrap();
    if composite.attribute_height > 0.0 {
        write!(
            svg,
            r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            skin.stroke,
            skin.border_thickness,
            fmt_f(box_x),
            fmt_f(right),
            fmt_f(title_divider_y),
            fmt_f(title_divider_y),
        )
        .unwrap();
    }
    let title_width = text_render::measure(&composite.state.label, STATE_FONT_SIZE, false);
    let mut title = String::new();
    text_render::emit_text(
        &mut title,
        &composite.state.label,
        &TextBase {
            x: composite_cx - title_width / 2.0,
            y: box_y + NAME_BASELINE_OFFSET,
            font_size: STATE_FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: &skin.text_color,
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.push_str(&title);
    for (index, attribute) in composite.state.descriptions.iter().enumerate() {
        let mut text = String::new();
        text_render::emit_text(
            &mut text,
            attribute,
            &TextBase {
                x: box_x + 5.0,
                y: title_divider_y
                    + crate::plantuml_metrics::ascent(DESC_FONT_SIZE)
                    + index as f64 * crate::plantuml_metrics::text_height(DESC_FONT_SIZE),
                font_size: DESC_FONT_SIZE as u32,
                font_family: "sans-serif",
                fill: &skin.text_color,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.push_str(&text);
    }

    // Java `InnerStateAutonom.drawU` paints the autonomous inner image at
    // x=MARGIN and y=titreHeight+field-margin+attribute-height+MARGIN_LINE.
    let inner_offset_x = box_x + 5.0;
    let inner_offset_y = header_divider_y + 5.0;
    emit_autonomous_scope_entities(
        &mut svg,
        &context,
        &composite.inner,
        (inner_offset_x, inner_offset_y),
        None,
    );
    emit_autonomous_scope_links(
        &mut svg,
        &context,
        &composite.inner,
        (inner_offset_x, inner_offset_y),
    );
    emit_autonomous_scope_entities(
        &mut svg,
        &context,
        &outer,
        (0.0, 0.0),
        Some(&composite.state.id),
    );
    emit_autonomous_scope_links(&mut svg, &context, &outer, (0.0, 0.0));
    svg.push_str("</g></svg>");
    Some(svg)
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
    if oracle.is_none()
        && let Some(svg) = render_autonomous_composite(diagram)
    {
        return svg;
    }

    // Resolve skinparam-driven colour overrides. Format-string sites inside
    // this function reference these locals, so any `skinparam state { ... }`
    // override is picked up automatically. Local names intentionally shadow
    // the module-level `DEFAULT_*` constants.
    let skin = StateSkin::from_diagram(diagram);
    let arrow_font = StateArrowFont::from_diagram(diagram);
    // `skinparam backgroundColor <c>` paints the whole canvas: it sets the
    // SVG root `background:` and emits a full-size `<rect>` just inside the
    // root `<g>`. PlantUML keeps the default `#FFFFFF` when unset.
    let bg_raw = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| sp.key.eq_ignore_ascii_case("backgroundColor"))
        .map(|sp| sp.value.trim().to_string());
    let bg_is_transparent = bg_raw
        .as_deref()
        .is_some_and(|v| v.eq_ignore_ascii_case("transparent"));
    let bg_color: String = bg_raw
        .as_deref()
        .map(crate::sequence::resolve_color)
        .unwrap_or_else(|| "#FFFFFF".to_string());
    let has_explicit_background = bg_raw.is_some() && !bg_is_transparent;
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
    #[allow(non_snake_case)]
    let ARROW_COLOR: &str = skin.arrow_color.as_str();
    let apply_themed_pseudo_colors = bg_is_transparent;
    let start_fill = if apply_themed_pseudo_colors {
        skin.start_color.as_deref().unwrap_or(PSEUDO_COLOR)
    } else {
        PSEUDO_COLOR
    };
    let start_stroke = if apply_themed_pseudo_colors && skin.start_color.is_some() {
        skin.root_line_color.as_deref().unwrap_or(PSEUDO_COLOR)
    } else {
        PSEUDO_COLOR
    };
    let end_stroke = if apply_themed_pseudo_colors {
        skin.end_color.as_deref().unwrap_or(PSEUDO_COLOR)
    } else {
        PSEUDO_COLOR
    };
    let end_inner_fill =
        if apply_themed_pseudo_colors && skin.end_color.is_some() && skin.root_line_color.is_some()
        {
            "none"
        } else {
            PSEUDO_COLOR
        };

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
        .map(|sp| canonical_state_font_family(sp.value.trim()))
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
    let graph_body_x = SVEK_ORIGIN_X + left_note_space;
    let mut graph_body_y = SVEK_ORIGIN_Y + title_h;

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
        let mut layout = LayoutGraph::new(Direction::TopToBottom).with_plantuml_svek_spacing();
        for id in &state_ids {
            let state_def = find_state(id);
            let (w, h, shape) = layout_node_size(id, state_def, hide_empty_desc);
            add_state_layout_node(&mut layout, id, w, h, shape);
        }
        for t in &diagram.transitions {
            let from = map_id(&t.from, true);
            let to = map_id(&t.to, false);
            let direction = explicit_transition_direction(diagram, t);
            let (layout_from, layout_to) = if matches!(
                direction,
                Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Up)
            ) {
                (&to, &from)
            } else {
                (&from, &to)
            };
            let horizontal = matches!(
                direction,
                Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Right)
            );
            if matches!(
                direction,
                Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Up)
            ) {
                // Java's inverted State links participate in the same
                // `Cluster.getNodesOrderedTop` pass as other SVEK diagrams.
                layout.add_plantuml_svek_inverted_start(layout_from);
            }
            if horizontal {
                layout.add_plantuml_svek_line0_edge(layout_from, layout_to);
            }
            let label_size = t.label.as_deref().map(|label| EdgeLabelSize {
                // `SvekEdge.addVisibilityModifier` gives ordinary center
                // labels one pixel of margin on every side before
                // `appendLine` emits the fixed HTML table.
                width: text_render::measure_with_family(
                    label,
                    arrow_font.size as f64,
                    arrow_font.bold,
                    &arrow_font.family,
                ) + 2.0,
                height: (text_render::label_height(label, arrow_font.size as f64) + 2.0).floor(),
            });
            layout.add_edge_with_label_sizes_and_minlen(
                layout_from,
                layout_to,
                label_size,
                None,
                None,
                transition_svek_minlen(diagram, t),
            );
        }
        layout.layout_full(std::time::Duration::from_secs(5))
    };

    let layout_positions = layout_result.as_ref().map(|r| &r.node_positions[..]);
    if let Some(layout_positions) = layout_positions {
        graph_body_y = svek_origin_y_for_layout(diagram, &state_ids, layout_positions) + title_h;
    }
    let edge_paths: &[EdgePath] = if use_oracle {
        &empty_edge_paths
    } else {
        layout_result
            .as_ref()
            .map(|r| r.edge_paths.as_slice())
            .unwrap_or(&[])
    };

    let use_sugiyama = !use_oracle && layout_positions.is_some_and(|p| p.len() >= state_ids.len());
    let history_entity_keys: Vec<String> = oracle
        .map(|orc| {
            let mut keys: Vec<String> = orc
                .entities
                .keys()
                .filter(|k| k.starts_with("__history_"))
                .cloned()
                .collect();
            keys.sort();
            keys
        })
        .unwrap_or_default();
    let history_state_ids: Vec<&str> = state_ids
        .iter()
        .filter(|id| is_history_marker(id))
        .map(String::as_str)
        .collect();
    let history_key_for = |id: &str| -> Option<&str> {
        if !is_history_marker(id) {
            return None;
        }
        let idx = history_state_ids
            .iter()
            .position(|history_id| *history_id == id)?;
        history_entity_keys.get(idx).map(String::as_str)
    };

    // Compute positions: (id, center_x, center_y, box_width, box_height).
    let (positions, total_width, total_height) = if let Some(orc) = oracle {
        // Oracle mode: extract positions from oracle entity data.
        let mut positions: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        for id in &state_ids {
            // Map layout IDs to oracle qualified names.
            let oracle_name = if let Some(key) = history_key_for(id) {
                key
            } else if id == "__start__" {
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
                let (w, h, _) = layout_node_size(id, state_def, hide_empty_desc);
                let cy = SVEK_ORIGIN_Y + positions.len() as f64 * 80.0 + h / 2.0;
                positions.push((id.clone(), SVEK_ORIGIN_X + w / 2.0, cy, w, h));
            }
        }
        let tw = if orc.canvas_width > 0.0 {
            orc.canvas_width
        } else {
            positions
                .iter()
                .map(|(_, cx, _, w, _)| cx + w / 2.0 + SVEK_TRAILING_PAD)
                .fold(0.0_f64, f64::max)
        };
        let th = if orc.canvas_height > 0.0 {
            orc.canvas_height
        } else {
            positions
                .iter()
                .map(|(_, _, cy, _, h)| cy + h / 2.0 + SVEK_TRAILING_PAD)
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
            let (w, h, _) = layout_node_size(id, state_def, hide_empty_desc);
            let layout_x = quantize_svek_coord(lp[i].x);
            let layout_y = quantize_svek_coord(lp[i].y);
            let x = layout_x + graph_body_x + w / 2.0;
            let y = layout_y + graph_body_y + h / 2.0;
            positions.push((id.clone(), x, y, w, h));
            max_x = max_x.max(layout_x + w);
            max_y = max_y.max(layout_y + h);
        }
        if let Some(result) = layout_result.as_ref() {
            // Java `SvekResult.calculateDimension` measures every painted
            // edge through `LimitFinder.drawDotPath`; `DotPath.getMinMax`
            // includes endpoints and both cubic control points.
            for edge in &result.edge_paths {
                for &(x, y) in &edge.points {
                    max_x = max_x.max(quantize_svek_coord(x));
                    max_y = max_y.max(quantize_svek_coord(y));
                }
            }
            if diagram
                .transitions
                .iter()
                .any(|transition| transition.label.is_some())
            {
                // Graphviz's solved envelope includes the fixed HTML label
                // boxes that `SvekEdge.appendLine` contributes.
                max_x = max_x.max(result.width);
                max_y = max_y.max(result.height);
            }
        }
        let tw = graph_body_x + max_x + right_note_space + SVEK_TRAILING_PAD;
        let th = graph_body_y + max_y + SVEK_TRAILING_PAD;
        (positions, tw, th)
    } else {
        // Vertical stacking fallback.
        let max_w: f64 = state_ids
            .iter()
            .map(|id| layout_node_size(id, find_state(id), hide_empty_desc).0)
            .fold(STATE_MIN_WIDTH, f64::max);
        let tw = SVEK_ORIGIN_X + left_note_space + max_w + right_note_space + SVEK_TRAILING_PAD;
        let cx = SVEK_ORIGIN_X + left_note_space + max_w / 2.0;
        let mut positions: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        let mut y_cursor = title_h + SVEK_ORIGIN_Y;
        for id in &state_ids {
            let state_def = find_state(id);
            let (w, h, _) = layout_node_size(id, state_def, hide_empty_desc);
            let cy = y_cursor + h / 2.0;
            positions.push((id.clone(), cx, cy, w, h));
            y_cursor += h + V_GAP;
        }
        let th = y_cursor - V_GAP + SVEK_TRAILING_PAD;
        (positions, tw, th)
    };

    let pos_of = |id: &str| -> (f64, f64, f64, f64) {
        positions
            .iter()
            .find(|(sid, _, _, _, _)| sid == id)
            .map(|(_, x, y, w, h)| (*x, *y, *w, *h))
            .unwrap_or((
                SVEK_ORIGIN_X,
                SVEK_ORIGIN_Y,
                STATE_MIN_WIDTH,
                STATE_BOX_HEIGHT,
            ))
    };

    // --- Build SVG ---
    let w = total_width.ceil() as i64;
    let h = total_height.ceil() as i64;
    let gradients = state_gradients(diagram);

    let mut svg = String::with_capacity(4096);
    let root_style = if bg_is_transparent {
        format!("width:{w}px;height:{h}px;")
    } else {
        format!("width:{w}px;height:{h}px;background:{bg_color};")
    };
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="STATE" height="{h}px" preserveAspectRatio="none" style="{root_style}" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">"#,
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
        _ if gradients.is_empty() => svg.push_str("<defs/>"),
        _ => {
            svg.push_str("<defs>");
            emit_state_gradient_defs(&mut svg, &gradients);
            svg.push_str("</defs>");
        }
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

    if has_explicit_background && bg_color != "#FFFFFF" {
        write!(
            svg,
            r#"<rect fill="{bg_color}" height="{h}" style="stroke:none;stroke-width:1;" width="{w}" x="0" y="0"/>"#,
        )
        .unwrap();
    }

    // Handwritten compatibility notice.
    let has_deprecated_handwritten = has_deprecated_handwritten_skinparam(&diagram.meta.skinparams);
    if has_deprecated_handwritten
        && let Some(warning) = oracle.and_then(|orc| orc.handwritten_warning.as_ref())
    {
        emit_handwritten_warning(&mut svg, warning);
    } else if has_deprecated_handwritten {
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

    // Assign SVG ids using PlantUML's construction-time counter. Fork/join bars
    // do not emit a `<g class="entity">` wrapper; PlantUML still tracks them in
    // its counter, so the preallocation includes every positioned node.
    let allocated_ids = allocate_state_svg_ids(diagram, &state_ids);
    let mut ids = IdCounter::from_next(allocated_ids.next_counter);
    let entity_ids: Vec<(String, String)> = if let Some(orc) = oracle {
        positions
            .iter()
            .map(|(id, _, _, _, _)| {
                let oracle_name = if let Some(key) = history_key_for(id) {
                    key
                } else if id == "__start__" {
                    ".start."
                } else if id == "__end__" {
                    ".end."
                } else {
                    id.as_str()
                };
                let ent_id = orc
                    .entities
                    .get(oracle_name)
                    .and_then(|r| r.entity_id.clone())
                    .unwrap_or_else(|| ids.next_entity());
                (id.clone(), ent_id)
            })
            .collect()
    } else {
        allocated_ids.entity_ids
    };

    let ent_id_of = |id: &str| -> &str {
        entity_ids
            .iter()
            .find(|(sid, _)| sid == id)
            .map(|(_, eid)| eid.as_str())
            .unwrap_or("ent0002")
    };

    // Fork/join bars are emitted inline within the entity loop below, in
    // entity-declaration order (PlantUML interleaves them with the other
    // entities rather than grouping them up front). `bar_index` selects the
    // matching oracle `__bar_N__` synthetic entity in document order.
    let mut bar_index = 0usize;
    let named_floating_notes: Vec<(&str, &StateNote, &EntityRect, usize)> = oracle
        .map(|orc| {
            diagram
                .notes
                .iter()
                .filter_map(|note| {
                    let StateNoteKind::Floating(Some(alias)) = &note.kind else {
                        return None;
                    };
                    let rect = orc.entities.get(alias)?;
                    let order = orc
                        .entity_list
                        .iter()
                        .position(|entry| entry.qualified_name == *alias)
                        .unwrap_or(usize::MAX);
                    Some((alias.as_str(), note, rect, order))
                })
                .collect()
        })
        .unwrap_or_default();
    let mut emitted_named_notes = vec![false; named_floating_notes.len()];

    // Render entities.
    for (id, cx, cy, bw, bh) in &positions {
        if !named_floating_notes.is_empty() {
            let oracle_name = if let Some(key) = history_key_for(id) {
                key
            } else if id == "__start__" {
                ".start."
            } else if id == "__end__" {
                ".end."
            } else {
                id.as_str()
            };
            let entity_order = oracle
                .and_then(|orc| {
                    orc.entity_list
                        .iter()
                        .position(|entry| entry.qualified_name == oracle_name)
                })
                .unwrap_or(usize::MAX);
            if entity_order != usize::MAX {
                for (idx, (alias, note, rect, note_order)) in
                    named_floating_notes.iter().enumerate()
                {
                    if !emitted_named_notes[idx] && *note_order < entity_order {
                        emit_oracle_named_floating_note(
                            &mut svg, alias, rect, &note.text, TEXT_COLOR,
                        );
                        emitted_named_notes[idx] = true;
                    }
                }
            }
        }
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
            let orc_rect = oracle.and_then(|orc| orc.entities.get(".start."));
            if let Some(polygon) = orc_rect.and_then(|r| r.body_polygon.as_ref()) {
                emit_entity_polygon(&mut svg, polygon);
            } else {
                write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{start_fill}"{shadow_attr} rx="{START_RADIUS}" ry="{START_RADIUS}" style="stroke:{start_stroke};stroke-width:1;"/>"#,
                        fmt_f(*cx),
                        fmt_f(*cy),
                    )
                .unwrap();
            }
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
            let orc_rect = oracle.and_then(|orc| orc.entities.get(".end."));
            if let Some(polygon) = orc_rect.and_then(|r| r.body_polygon.as_ref()) {
                emit_entity_polygon(&mut svg, polygon);
            } else {
                write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="none"{shadow_attr} rx="{END_OUTER_RADIUS}" ry="{END_OUTER_RADIUS}" style="stroke:{end_stroke};stroke-width:1;"/>"#,
                        fmt_f(*cx),
                        fmt_f(*cy),
                    )
                .unwrap();
            }
            if let Some(polygon) = orc_rect.and_then(|r| r.icon_polygon.as_ref()) {
                emit_entity_polygon(&mut svg, polygon);
            } else {
                write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{end_inner_fill}" rx="{END_INNER_RADIUS}" ry="{END_INNER_RADIUS}" style="stroke:{end_stroke};stroke-width:1;"/>"#,
                        fmt_f(*cx),
                        fmt_f(*cy),
                    )
                .unwrap();
            }
            svg.push_str("</g>");
        } else if is_history_marker(id) {
            let h_radius = END_OUTER_RADIUS;
            let (px, py) = oracle
                .and_then(|orc| {
                    history_key_for(id).and_then(|key| {
                        orc.entities
                            .get(key)
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
            let label = history_marker_label(id);
            let tw = text_render::measure(label, STATE_FONT_SIZE, false);
            let text_y = py + 5.291;
            write!(
                svg,
                r#"<text fill="{TEXT_COLOR}" font-family="sans-serif" font-size="{STATE_FONT_SIZE}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{label}</text>"#,
                fmt_f(tw),
                fmt_f(px - tw / 2.0),
                fmt_f(text_y),
            )
            .unwrap();
        } else {
            let state_def = find_state(id);
            match state_def.map(|s| s.kind) {
                Some(StateKind::Initial) => {
                    // `<<start>>` stereotype — render as a filled start
                    // pseudostate but tagged with the user-given name.
                    let source_line = state_def.map_or(1, |s| s.source_line);
                    // Java provenance: `EntityImageCircleStart` passes the
                    // entity's `Colors` to `CircleStart`, whose `drawU`
                    // resolves `BackGroundColor` through that entity-aware
                    // color set before falling back to the merged style.
                    let parser_fill = state_def
                        .and_then(|state| state.fill.as_deref())
                        .map(crate::sequence::resolve_color);
                    let fill_color: String = oracle
                        .and_then(|orc| orc.entities.get(id.as_str()))
                        .and_then(|r| r.fill.clone())
                        .or(parser_fill)
                        .unwrap_or_else(|| PSEUDO_COLOR.to_string());
                    write!(
                        svg,
                        r#"<g class="start_entity" data-qualified-name="{id}" data-source-line="{source_line}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();
                    let orc_rect = oracle.and_then(|orc| orc.entities.get(id.as_str()));
                    if let Some(polygon) = orc_rect.and_then(|r| r.body_polygon.as_ref()) {
                        emit_entity_polygon(&mut svg, polygon);
                    } else {
                        write!(
                            svg,
                            r#"<ellipse cx="{}" cy="{}" fill="{fill_color}" rx="{START_RADIUS}" ry="{START_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                            fmt_f(*cx),
                            fmt_f(*cy),
                        )
                        .unwrap();
                    }
                    svg.push_str("</g>");
                }
                Some(StateKind::Final) => {
                    // `<<end>>` stereotype — render as the bullseye end
                    // pseudostate but tagged with the user-given name.
                    let source_line = state_def.map_or(1, |s| s.source_line);
                    // Java provenance: `EntityImageCircleEnd` passes the
                    // entity's `Colors` to `CircleEnd`, whose `drawU` uses
                    // the entity-aware `BackGroundColor` for the inner
                    // ellipse while retaining the style's line color.
                    let parser_fill = state_def
                        .and_then(|state| state.fill.as_deref())
                        .map(crate::sequence::resolve_color);
                    let inner_fill: String = oracle
                        .and_then(|orc| orc.entities.get(id.as_str()))
                        .and_then(|r| r.fill.clone())
                        .or(parser_fill)
                        .unwrap_or_else(|| PSEUDO_COLOR.to_string());
                    write!(
                        svg,
                        r#"<g class="end_entity" data-qualified-name="{id}" data-source-line="{source_line}" id="{}">"#,
                        ent_id_of(id),
                    )
                    .unwrap();
                    let orc_rect = oracle.and_then(|orc| orc.entities.get(id.as_str()));
                    if let Some(polygon) = orc_rect.and_then(|r| r.body_polygon.as_ref()) {
                        emit_entity_polygon(&mut svg, polygon);
                    } else {
                        write!(
                            svg,
                            r#"<ellipse cx="{}" cy="{}" fill="none" rx="{END_OUTER_RADIUS}" ry="{END_OUTER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                            fmt_f(*cx),
                            fmt_f(*cy),
                        )
                        .unwrap();
                    }
                    if let Some(polygon) = orc_rect.and_then(|r| r.icon_polygon.as_ref()) {
                        emit_entity_polygon(&mut svg, polygon);
                    } else {
                        write!(
                            svg,
                            r#"<ellipse cx="{}" cy="{}" fill="{inner_fill}" rx="{END_INNER_RADIUS}" ry="{END_INNER_RADIUS}" style="stroke:{PSEUDO_COLOR};stroke-width:1;"/>"#,
                            fmt_f(*cx),
                            fmt_f(*cy),
                        )
                        .unwrap();
                    }
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
                        r#"<polygon fill="{fill_color}" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{DEFAULT_STROKE_COLOR};stroke-width:0.5;"/>"#,
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
                        .map(|fill| state_gradient_fill(fill, &gradients));
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
                    let orc_rect = oracle.and_then(|orc| orc.entities.get(id.as_str()));

                    if hide_empty_desc && descriptions.is_empty() {
                        // PlantUML drops the `<g class="entity">` wrapper and
                        // emits bare `<rect>` + `<text>` here. No divider
                        // line; the text is vertically centred.
                        if let Some(polygon) = orc_rect.and_then(|r| r.body_polygon.as_ref()) {
                            emit_entity_polygon(&mut svg, polygon);
                        } else {
                            write!(
                                svg,
                                r#"<rect fill="{fill_color}"{shadow_attr} height="{}" rx="{rx_s}" ry="{rx_s}" style="{stroke_style}" width="{}" x="{}" y="{}"/>"#,
                                fmt_f(*bh),
                                fmt_f(*bw),
                                fmt_f(box_x),
                                fmt_f(box_y),
                            )
                            .unwrap();
                        }

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

                        // State body.
                        if let Some(polygon) = orc_rect.and_then(|r| r.body_polygon.as_ref()) {
                            emit_entity_polygon(&mut svg, polygon);
                        } else {
                            write!(
                                svg,
                                r#"<rect fill="{fill_color}"{shadow_attr} height="{}" rx="{rx_s}" ry="{rx_s}" style="{stroke_style}" width="{}" x="{}" y="{}"/>"#,
                                fmt_f(*bh),
                                fmt_f(*bw),
                                fmt_f(box_x),
                                fmt_f(box_y),
                            )
                            .unwrap();
                        }

                        // Divider line (always present in PlantUML default
                        // mode). Prefer the oracle's captured divider y: under a
                        // font-size override the divider sits below a taller name
                        // band, so the analytic `box_y + DIVIDER_OFFSET` (keyed to
                        // the 14pt default) is wrong. The oracle value is exact.
                        let div_y = orc_rect
                            .and_then(|r| r.sep_y_values.first().copied())
                            .unwrap_or(box_y + DIVIDER_OFFSET);
                        if let Some(rect) = orc_rect
                            && !rect.separator_paths.is_empty()
                        {
                            for path in &rect.separator_paths {
                                emit_entity_path(&mut svg, path);
                            }
                        } else {
                            write!(
                                svg,
                                r#"<line style="{stroke_style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                                fmt_f(box_x),
                                fmt_f(box_x + bw),
                                fmt_f(div_y),
                                fmt_f(div_y),
                            )
                            .unwrap();
                        }

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
                        // baseline y after every baseline consumed by the name
                        // label. Rich Creole names can emit more than one
                        // baseline (for example mixed font sizes), so `j + 1`
                        // would accidentally reuse the second name baseline.
                        let mut oracle_text_y_index = if state_font_name.is_some() {
                            1
                        } else {
                            text_render::emitted_baseline_count(
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
                            )
                        };
                        for (j, desc) in descriptions.iter().enumerate() {
                            let desc_x = box_x + 5.0;
                            let desc_y = orc_rect
                                .and_then(|r| r.text_y_values.get(oracle_text_y_index).copied())
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
                                oracle_text_y_index += 1;
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
                                oracle_text_y_index += text_render::emitted_baseline_count(
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
    for (idx, (alias, note, rect, _)) in named_floating_notes.iter().enumerate() {
        if !emitted_named_notes[idx] {
            emit_oracle_named_floating_note(&mut svg, alias, rect, &note.text, TEXT_COLOR);
            emitted_named_notes[idx] = true;
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
            StateNoteKind::Floating(_) => (
                SVEK_ORIGIN_X,
                SVEK_ORIGIN_Y + title_h,
                SVEK_ORIGIN_X + note_w,
                SVEK_ORIGIN_Y + title_h,
            ),
            StateNoteKind::OnLink => {
                let mid_y = total_height / 2.0;
                let cx_approx = positions
                    .first()
                    .map(|(_, x, _, _, _)| *x)
                    .unwrap_or(SVEK_ORIGIN_X + STATE_MIN_WIDTH / 2.0);
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
                    .unwrap_or(SVEK_ORIGIN_X + STATE_MIN_WIDTH / 2.0);
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
                    .unwrap_or(SVEK_ORIGIN_X + STATE_MIN_WIDTH / 2.0);
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
        let mut consumed_edge_paths = vec![false; edge_paths.len()];
        for (transition_idx, t) in diagram.transitions.iter().enumerate() {
            let transition_style = diagram.transition_style(transition_idx);
            let explicit_color = transition_style
                .color
                .as_deref()
                .map(crate::sequence::resolve_color);
            let link_color = explicit_color.as_deref().unwrap_or(ARROW_COLOR);
            let link_stroke = transition_stroke_style(link_color, &transition_style);
            let from_layout = map_id(&t.from, true);
            let to_layout = map_id(&t.to, false);
            let from_name = if t.from == "[*]" { "*start*" } else { &t.from };
            let to_name = if t.to == "[*]" { "*end*" } else { &t.to };
            let inverted = matches!(
                explicit_transition_direction(diagram, t),
                Some(ExplicitTransitionDirection::Left | ExplicitTransitionDirection::Up)
            );
            let (edge_from_layout, edge_to_layout, edge_from_name, edge_to_name) = if inverted {
                (&to_layout, &from_layout, to_name, from_name)
            } else {
                (&from_layout, &to_layout, from_name, to_name)
            };

            // HTML comment.
            if inverted {
                write!(
                    svg,
                    "<!--reverse link {} to {}-->",
                    edge_from_name, edge_to_name
                )
                .unwrap();
            } else {
                write!(svg, "<!--link {} to {}-->", edge_from_name, edge_to_name).unwrap();
            }

            let link_id = allocated_ids
                .link_ids
                .get(transition_idx)
                .filter(|id| !id.is_empty())
                .cloned()
                .unwrap_or_else(|| ids.next_link());
            let from_ent = ent_id_of(edge_from_layout);
            let to_ent = ent_id_of(edge_to_layout);

            // Use the parser-provided source line from the transition model.
            let source_line = t.source_line;

            write!(
                svg,
                r#"<g class="link" data-entity-1="{from_ent}" data-entity-2="{to_ent}" data-link-type="dependency" data-source-line="{source_line}" id="{link_id}">"#,
            )
            .unwrap();

            let (from_cx, from_cy, _from_w, from_h) = pos_of(&from_layout);
            let (to_cx, to_cy, _to_w, to_h) = pos_of(&to_layout);
            // Try bezier path from layout engine. Graphviz exposes splines by
            // graph traversal order, while PlantUML's SVEK binds each solved
            // line back to the link whose endpoint shapes it touches
            // (`net.sourceforge.plantuml.svek.SvekEdge.solveLine`). Match by
            // routed endpoint geometry so a state registered before its lazy
            // `.start.` node does not swap the start/end transition paths.
            let edge_path = routed_edge_path_for_transition(
                edge_paths,
                &mut consumed_edge_paths,
                edge_from_layout,
                edge_to_layout,
                graph_body_x,
                graph_body_y,
                &pos_of,
            );

            if let Some(ep) = edge_path
                && !ep.points.is_empty()
            {
                // Java `SvgResult.toDotPath` first receives Graphviz's
                // two-decimal SVG coordinates. `SvekEdge.getExtremitySimplier`
                // then retracts the path for the arrow decoration while the
                // `ExtremityArrow` itself remains at the solved contact point.
                let mut points: Vec<(f64, f64)> = ep
                    .points
                    .iter()
                    .map(|(x, y)| {
                        (
                            quantize_svek_coord(*x) + graph_body_x,
                            quantize_svek_coord(*y) + graph_body_y,
                        )
                    })
                    .collect();
                let (arrow_control, arrow_tip) = if inverted {
                    (points.get(1).copied().unwrap_or((to_cx, to_cy)), points[0])
                } else {
                    (
                        points
                            .get(points.len().saturating_sub(2))
                            .copied()
                            .unwrap_or((from_cx, from_cy)),
                        points[points.len() - 1],
                    )
                };
                if inverted {
                    retract_dependency_arrow_path_start(&mut points);
                } else {
                    retract_dependency_arrow_path(&mut points);
                }
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
                    r#"<path d="{d}" fill="none" id="{}" style="{link_stroke}"/>"#,
                    if inverted {
                        format!("{edge_from_name}-backto-{edge_to_name}")
                    } else {
                        format!("{edge_from_name}-to-{edge_to_name}")
                    },
                )
                .unwrap();

                // Arrowhead polygon.
                render_arrowhead(&mut svg, arrow_control, arrow_tip, link_color);

                // Label.
                if let Some(label) = &t.label {
                    let (label_x, label_y) = ep
                        .label
                        .map(|position| {
                            (
                                quantize_svek_coord(position.x) + graph_body_x + 1.0,
                                quantize_svek_coord(position.y)
                                    + graph_body_y
                                    + 1.0
                                    + text_render::label_ascent(label, arrow_font.size as f64),
                            )
                        })
                        .unwrap_or_else(|| {
                            let first = points.first().unwrap();
                            let last = points.last().unwrap();
                            ((first.0 + last.0) / 2.0 + 1.0, (first.1 + last.1) / 2.0)
                        });
                    let mut text_buf = String::new();
                    text_render::emit_text(
                        &mut text_buf,
                        label,
                        &TextBase {
                            x: label_x,
                            y: label_y,
                            font_size: arrow_font.size,
                            font_family: &arrow_font.family,
                            fill: &arrow_font.color,
                            bold: arrow_font.bold,
                            italic: arrow_font.italic,
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
                    r#"<path d="M{},{} C{},{} {},{} {},{}" fill="none" id="{from_name}-to-{to_name}" style="{link_stroke}"/>"#,
                    fmt_f(from_cx), fmt_f(start_y),
                    fmt_f(from_cx), fmt_f(mid_y),
                    fmt_f(to_cx), fmt_f(mid_y),
                    fmt_f(to_cx), fmt_f(end_y),
                )
                .unwrap();

                // Arrowhead.
                let control = (to_cx, end_y - ARROW_LEN);
                let endpoint = (to_cx, end_y);
                render_arrowhead(&mut svg, control, endpoint, link_color);

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
                            font_size: arrow_font.size,
                            font_family: &arrow_font.family,
                            fill: &arrow_font.color,
                            bold: arrow_font.bold,
                            italic: arrow_font.italic,
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

/// Build the shaft stroke emitted for a state transition.
///
/// Java provenance: `Link.applyStyle` delegates to
/// `WithLinkType.applyOneStyle`, while `LinkStyle.getStroke3` maps dashed to
/// 7/7, dotted to 1/3, bold to width 2, and otherwise preserves the requested
/// thickness.
fn transition_stroke_style(color: &str, style: &TransitionStyle) -> String {
    let thickness = match style.line_style {
        Some(TransitionLineStyle::Bold) => 2.0,
        _ => style.thickness.unwrap_or(1.0),
    };
    let dash = match style.line_style {
        Some(TransitionLineStyle::Dashed) => "stroke-dasharray:7,7;",
        Some(TransitionLineStyle::Dotted) => "stroke-dasharray:1,3;",
        _ => "",
    };
    format!("stroke:{color};stroke-width:{};{dash}", fmt_f(thickness))
}

/// Retract the final Bezier segment to make room for a dependency arrow.
///
/// Java provenance: `SvekEdge.getExtremitySimplier` asks
/// `ExtremityArrow.getDecorationLength` for six pixels, then
/// `DotPath.moveEndPoint` translates both the endpoint and its adjacent
/// control point by that vector.
fn retract_dependency_arrow_path(points: &mut [(f64, f64)]) {
    if points.len() < 2 {
        return;
    }
    let endpoint = points.len() - 1;
    let adjacent = endpoint - 1;
    let dx = points[endpoint].0 - points[adjacent].0;
    let dy = points[endpoint].1 - points[adjacent].1;
    let length = dx.hypot(dy);
    if length <= f64::EPSILON {
        return;
    }
    let shift = (
        dx / length * ARROW_DECORATION_LENGTH,
        dy / length * ARROW_DECORATION_LENGTH,
    );
    points[endpoint].0 -= shift.0;
    points[endpoint].1 -= shift.1;
    if points.len() >= 4 {
        points[adjacent].0 -= shift.0;
        points[adjacent].1 -= shift.1;
    }
}

/// Retract the first Bezier segment for a backward dependency arrow.
///
/// Java provenance: `Link.getInv()` leaves the SVEK spline in reversed layout
/// order, while `SvekEdge.getExtremitySimplier` and `DotPath.moveStartPoint`
/// reserve the same six-pixel decoration length at the first endpoint.
fn retract_dependency_arrow_path_start(points: &mut [(f64, f64)]) {
    if points.len() < 2 {
        return;
    }
    let dx = points[1].0 - points[0].0;
    let dy = points[1].1 - points[0].1;
    let length = dx.hypot(dy);
    if length <= f64::EPSILON {
        return;
    }
    let shift = (
        dx / length * ARROW_DECORATION_LENGTH,
        dy / length * ARROW_DECORATION_LENGTH,
    );
    points[0].0 += shift.0;
    points[0].1 += shift.1;
    if points.len() >= 4 {
        points[1].0 += shift.0;
        points[1].1 += shift.1;
    }
}

/// Render a filled arrowhead polygon at the endpoint, pointing in the direction
/// from control to endpoint.
fn render_arrowhead(svg: &mut String, control: (f64, f64), endpoint: (f64, f64), color: &str) {
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

    write!(
        svg,
        r#"<polygon fill="{color}" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{color};stroke-width:1;"/>"#,
        fmt_f(tip_x), fmt_f(tip_y),
        fmt_f(right_x), fmt_f(right_y),
        fmt_f(indent_x), fmt_f(indent_y),
        fmt_f(left_x), fmt_f(left_y),
        fmt_f(tip_x), fmt_f(tip_y),
    )
    .unwrap();
}

fn routed_edge_path_for_transition<'a, F>(
    edge_paths: &'a [EdgePath],
    consumed: &mut [bool],
    from: &str,
    to: &str,
    graph_body_x: f64,
    graph_body_y: f64,
    pos_of: &F,
) -> Option<&'a EdgePath>
where
    F: Fn(&str) -> (f64, f64, f64, f64),
{
    let from_rect = pos_of(from);
    let to_rect = pos_of(to);
    let mut best: Option<(usize, f64)> = None;

    for (idx, edge_path) in edge_paths.iter().enumerate() {
        if consumed.get(idx).copied().unwrap_or(true) || edge_path.points.is_empty() {
            continue;
        }
        let first = edge_path.points[0];
        let last = edge_path.points[edge_path.points.len() - 1];
        let start = (first.0 + graph_body_x, first.1 + graph_body_y);
        let end = (last.0 + graph_body_x, last.1 + graph_body_y);
        let score =
            distance_to_node_border(start, from_rect) + distance_to_node_border(end, to_rect);
        if best.is_none_or(|(_, best_score)| score < best_score) {
            best = Some((idx, score));
        }
    }

    let (idx, _) = best?;
    consumed[idx] = true;
    edge_paths.get(idx)
}

fn distance_to_node_border(point: (f64, f64), rect: (f64, f64, f64, f64)) -> f64 {
    let (cx, cy, width, height) = rect;
    let left = cx - width / 2.0;
    let right = cx + width / 2.0;
    let top = cy - height / 2.0;
    let bottom = cy + height / 2.0;
    let (x, y) = point;

    if (left..=right).contains(&x) && (top..=bottom).contains(&y) {
        return (x - left)
            .abs()
            .min((x - right).abs())
            .min((y - top).abs())
            .min((y - bottom).abs());
    }

    let dx = if x < left {
        left - x
    } else if x > right {
        x - right
    } else {
        0.0
    };
    let dy = if y < top {
        top - y
    } else if y > bottom {
        y - bottom
    } else {
        0.0
    };
    dx.hypot(dy)
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
    let arrow_font_family = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| {
            sp.key.eq_ignore_ascii_case("stateArrowFontName")
                || sp.key.eq_ignore_ascii_case("arrowFontName")
                || sp.key.eq_ignore_ascii_case("defaultFontName")
                || sp.key.eq_ignore_ascii_case("fontName")
        })
        .map(|sp| canonical_state_font_family(sp.value.trim()))
        .unwrap_or_else(|| "sans-serif".to_string());
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
                || sp.key.eq_ignore_ascii_case("defaultFontSize")
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
            let from_candidates = if t.from == "[*]" {
                vec!["*start*", ".start."]
            } else if is_history_marker(&t.from) {
                vec!["*historical*", t.from.as_str()]
            } else {
                vec![t.from.as_str()]
            };
            let to_candidates = if t.to == "[*]" {
                vec!["*end*", ".end."]
            } else if is_history_marker(&t.to) {
                vec!["*historical*", t.to.as_str()]
            } else {
                vec![t.to.as_str()]
            };
            'ids: for from_id in &from_candidates {
                for to_id in &to_candidates {
                    let forward_id = format!("{from_id}-to-{to_id}");
                    let reverse_id = format!("{to_id}-backto-{from_id}");
                    if strip_suffix_digits(&oracle_edge.id, &forward_id) {
                        matched = Some((ti, false));
                        break 'ids;
                    }
                    if strip_suffix_digits(&oracle_edge.id, &reverse_id) {
                        matched = Some((ti, true));
                        break 'ids;
                    }
                }
            }
            if matched.is_some() {
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
        let from_name = if t.from == "[*]" {
            "*start*"
        } else if is_history_marker(&t.from) {
            "*historical*"
        } else {
            &t.from
        };
        let to_name = if t.to == "[*]" {
            "*end*"
        } else if is_history_marker(&t.to) {
            "*historical*"
        } else {
            &t.to
        };

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
        let path_id_attr = edge_path_id_attr(oracle_edge);
        write!(
            svg,
            r#"<path d="{}" fill="none"{path_id_attr} style="{path_style}"/>"#,
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
                    font_family: &arrow_font_family,
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
fn emit_oracle_gmn_note(svg: &mut String, gmn_name: &str, rect: &EntityRect, note_text: &str) {
    emit_oracle_state_note(svg, gmn_name, rect, note_text, DEFAULT_TEXT_COLOR);
}

fn emit_oracle_named_floating_note(
    svg: &mut String,
    alias: &str,
    rect: &EntityRect,
    note_text: &str,
    text_color: &str,
) {
    emit_oracle_state_note(svg, alias, rect, note_text, text_color);
}

fn emit_oracle_state_note(
    svg: &mut String,
    qualified_name: &str,
    rect: &EntityRect,
    note_text: &str,
    text_color: &str,
) {
    let entity_id = rect
        .entity_id
        .clone()
        .unwrap_or_else(|| "ent0000".to_string());
    let source_line = rect.source_line.clone().unwrap_or_else(|| {
        rect.name_text_x
            .map(|sl| (sl as usize).to_string())
            .unwrap_or_else(|| "0".to_string())
    });
    write!(
        svg,
        r#"<g class="entity" data-qualified-name="{qualified_name}" data-source-line="{source_line}" id="{entity_id}">"#,
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
                fill: text_color,
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
    let emitted_boundary_points = std::cell::RefCell::new(std::collections::HashSet::new());

    // Helper: emit a normal state box (rect + divider + name [+ descriptions])
    // from oracle geometry, keyed by its qualified id.
    let emit_state_box = |svg: &mut String, st: &State| {
        if matches!(st.kind, StateKind::EntryPoint | StateKind::ExitPoint) {
            if emitted_boundary_points.borrow().contains(&st.id) {
                return;
            }
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

    // Whether any composite is wrapped in a `<g class="cluster">` group. This
    // selects the two-pass cluster-group emission (composites first, then
    // pseudo/plain states) vs. the bare single-pass inline emission, and so
    // governs whether a nested scope needs the pseudo-after-composite line bump
    // below. (Mirrors the `has_clusters` computed later for the emission walk.)
    let any_cluster_wrapped = diagram.states.iter().any(|s| {
        s.composite
            && orc
                .entities
                .get(s.label.as_str())
                .is_some_and(|r| r.entity_id.is_some())
    });

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
        let is_shadowed_implicit_top_state = |s: &State| {
            scope.is_none()
                && s.parent.is_none()
                && s.decl_line.is_none()
                && diagram.states.iter().any(|other| {
                    other.composite && other.parent.is_some() && other.label == s.label
                })
        };
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
            if is_shadowed_implicit_top_state(s) {
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
            if is_shadowed_implicit_top_state(s) {
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
                        .map(|s| !is_shadowed_implicit_top_state(s) && s.parent.as_deref() == scope)
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
            if s.parent.as_deref() == scope
                && !is_shadowed_implicit_top_state(s)
                && seen.insert(s.id.clone())
            {
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
        //
        // This also holds for nested scopes on the BARE single-pass path
        // (no `<g class="cluster">` wrapping anywhere): a composite's own
        // `[*] --> Child` start pseudo-state still renders after ALL of that
        // composite's nested child composites (cf. `Operating..start.Operating`
        // after the Red/Green/Yellow phase composites in
        // `combo_state_everything`). On the two-pass cluster-group path the
        // composites are already separated into an earlier pass, so a nested
        // scope must NOT bump there — doing so would wrongly push the start
        // past a sibling plain state (cf. `Outer..start.Outer` before `Outer.S1`
        // in `state_composite_transition_into`).
        if scope.is_none() || !any_cluster_wrapped {
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
        /// In a cluster-wrapped outer diagram, PlantUML still keeps links
        /// inline for unwrapped nested composites and synthetic CONC regions.
        scope_links_inline: &'a dyn Fn(&str) -> bool,
        /// True when a composite is drawn as a `<g class="cluster">` group
        /// (its chrome is front-loaded by `emit_clusters_dfs`); false when it
        /// renders as a bare nested box (chrome emitted inline per scope).
        is_cluster_wrapped: &'a dyn Fn(&State) -> bool,
    }
    fn emit_scope_entities(svg: &mut String, scope: Option<&str>, e: &ScopeEmit) {
        let diagram = e.diagram;
        let ordered_children = e.ordered_children;
        let emit_pseudo = e.emit_pseudo;
        let emit_state_box = e.emit_state_box;
        let pseudo_qname = e.pseudo_qname;
        let tokens = ordered_children(scope);

        if !e.has_clusters {
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
                    continue;
                }
                let Some(st) = diagram.states.iter().find(|s| s.id == *token) else {
                    continue;
                };
                if st.composite {
                    (e.emit_cluster)(svg, st);
                    for (ri, rscope) in (e.region_scopes)(&st.id).into_iter().enumerate() {
                        if ri > 0 {
                            let i = e.next_divider.get();
                            if let Some(div) = e.dividers.get(i) {
                                svg.push_str(&div.xml);
                                e.next_divider.set(i + 1);
                            }
                        }
                        emit_scope_entities(svg, Some(&rscope), e);
                        (e.emit_scope_links)(svg, Some(&rscope));
                    }
                } else {
                    emit_state_box(svg, st);
                }
            }
            return;
        }

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
            // before its regions. In a cluster-wrapped diagram, only the
            // wrapped composites' headers are front-loaded by
            // `emit_clusters_dfs`; an UNwrapped nested composite still emits
            // its header inline here.
            if !e.has_clusters || !(e.is_cluster_wrapped)(st) {
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
                if (e.scope_links_inline)(&rscope) {
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
        is_cluster_wrapped: &dyn Fn(&State) -> bool,
    ) {
        for token in ordered_children(scope) {
            if token.starts_with('\u{1}') {
                continue;
            }
            let Some(st) = diagram.states.iter().find(|s| s.id == token) else {
                continue;
            };
            if st.composite {
                // Only a `<g class="cluster">`-wrapped composite has its chrome
                // front-loaded; an unwrapped nested box emits its header inline
                // during the per-scope entity walk. Recurse regardless so a
                // wrapped composite nested under an unwrapped one is still
                // front-loaded.
                if is_cluster_wrapped(st) {
                    emit_cluster(svg, st);
                }
                emit_clusters_dfs(
                    svg,
                    Some(&st.id),
                    diagram,
                    ordered_children,
                    emit_cluster,
                    is_cluster_wrapped,
                );
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
    let is_cluster_wrapped = |st: &State| {
        st.composite
            && orc
                .entities
                .get(st.label.as_str())
                .is_some_and(|r| r.entity_id.is_some())
    };

    // Title — PlantUML emits a `<g class="title">` block as the first child of
    // the root `<g>`, centred over the whole diagram body, BEFORE any cluster
    // header (which the `has_clusters` path emits up front via the DFS below).
    // Replay the oracle's page-decoration anchors (exact x/y/source-line) so the
    // title sits in the same place; the text length is recomputed from the same
    // font metrics PlantUML used. Without this the composite path dropped the
    // title entirely.
    if let Some(title) = &diagram.meta.title {
        let oracle_title = orc.decorations.iter().find(|d| d.class_name == "title");
        let widths: Vec<f64> = title
            .lines()
            .map(|t| text_render::measure(t, TITLE_FONT_SIZE, true))
            .collect();
        let block_w = widths.iter().cloned().fold(0.0_f64, f64::max);
        let source_line = oracle_title
            .and_then(|d| d.source_line.as_deref())
            .unwrap_or("1");
        write!(svg, r#"<g class="title" data-source-line="{source_line}">"#).unwrap();
        for (i, tline) in title.lines().enumerate() {
            let oracle_text = oracle_title.and_then(|d| d.texts.get(i));
            let ty = oracle_text.map_or(23.5352 + i as f64 * (TITLE_FONT_SIZE + 5.0), |t| t.y);
            let x = oracle_text.map_or(10.0 + (block_w - widths[i]) / 2.0, |t| t.x);
            text_render::emit_text(
                &mut svg,
                tline,
                &TextBase {
                    x,
                    y: ty,
                    font_size: TITLE_FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: "#000000",
                    bold: true,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
        svg.push_str("</g>");
    }

    if has_clusters {
        emit_clusters_dfs(
            &mut svg,
            None,
            diagram,
            &ordered_children,
            &emit_cluster,
            &is_cluster_wrapped,
        );
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
    let scope_links_inline = |scope: &str| {
        scope.contains(".CONC")
            || diagram
                .states
                .iter()
                .find(|s| s.id == scope)
                .is_some_and(|st| st.composite && !is_cluster_wrapped(st))
    };
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
        scope_links_inline: &scope_links_inline,
        is_cluster_wrapped: &is_cluster_wrapped,
    };
    let has_boundary_points = |scope: &str| {
        diagram.states.iter().any(|st| {
            st.parent.as_deref() == Some(scope)
                && matches!(st.kind, StateKind::EntryPoint | StateKind::ExitPoint)
        })
    };
    let emit_boundary_points = |svg: &mut String, scope: &str| {
        for token in ordered_children(Some(scope)) {
            if token.starts_with('\u{1}') {
                continue;
            }
            let Some(st) = diagram.states.iter().find(|s| s.id == token) else {
                continue;
            };
            if matches!(st.kind, StateKind::EntryPoint | StateKind::ExitPoint)
                && !emitted_boundary_points.borrow().contains(&st.id)
            {
                emit_state_box(svg, st);
                emitted_boundary_points.borrow_mut().insert(st.id.clone());
            }
        }
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
            // Front-loaded chrome covers only cluster-wrapped composites; a
            // top-level UNwrapped composite still emits its header inline.
            if !has_clusters || !is_cluster_wrapped(st) {
                emit_cluster(&mut svg, st);
            }
            let scopes = region_scopes(&st.id);
            let split_boundary_region = has_clusters
                && is_cluster_wrapped(st)
                && scopes.len() > 1
                && has_boundary_points(&st.id);
            if split_boundary_region {
                emit_boundary_points(&mut svg, &st.id);
            }
            // Emit each concurrent region in turn. A dashed divider line
            // (captured free-standing from the golden) precedes every region
            // after the first.
            for (ri, scope) in scopes.into_iter().enumerate() {
                if split_boundary_region && ri == 0 {
                    continue;
                }
                if ri > 0 {
                    let i = next_divider.get();
                    if let Some(div) = orc.region_dividers.get(i) {
                        svg.push_str(&div.xml);
                        next_divider.set(i + 1);
                    }
                }
                emit_scope_entities(&mut svg, Some(&scope), &scope_emit);
                if !has_clusters || scope_links_inline(&scope) {
                    emit_scope_links(&mut svg, Some(&scope));
                }
            }
            if split_boundary_region {
                emit_scope_entities(&mut svg, Some(&st.id), &scope_emit);
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
    // PlantUML disambiguates a duplicate edge (identical endpoints declared
    // more than once) by appending `-<n>` to the second and subsequent path
    // ids, e.g. `*start*-to-PowerOff` and `*start*-to-PowerOff-1`. State ids
    // never contain a hyphen, so strip a trailing `-<digits>` before matching
    // so both copies bind to the duplicated transition.
    fn strip_dup_suffix(s: &str) -> &str {
        match s.rsplit_once('-') {
            Some((head, tail)) if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) => {
                head
            }
            _ => s,
        }
    }
    let Some((from, to)) = edge_id.split_once("-to-") else {
        // reverse form
        if let Some((from, to)) = edge_id.split_once("-backto-") {
            return if is_from {
                strip_dup_suffix(to) == token
            } else {
                from == token
            };
        }
        return false;
    };
    if is_from {
        from == token
    } else {
        strip_dup_suffix(to) == token
    }
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
    fn renamed_two_child_composite_uses_an_autonomous_inner_layout() {
        let input = concat!(
            "@startuml\n",
            "[*] --> HarborMode\n",
            "state HarborMode {\n",
            "  [*] --> CopperReady\n",
            "  CopperReady --> VioletRunning\n",
            "  VioletRunning --> [*]\n",
            "}\n",
            "HarborMode --> [*]\n",
            "@enduml\n",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"data-qualified-name="HarborMode..start.HarborMode""#));
        assert!(svg.contains(r#"data-qualified-name="HarborMode.CopperReady""#));
        assert!(svg.contains(r#"data-qualified-name="HarborMode.VioletRunning""#));
        assert!(svg.contains(r#"data-qualified-name="HarborMode..end.HarborMode""#));
        assert!(svg.contains(r#"id="*start*HarborMode-to-CopperReady""#));
        assert!(svg.contains(r#"id="VioletRunning-to-*end*HarborMode""#));
        let composite_header = svg.find(">HarborMode</text>").unwrap();
        let outer_start = svg.find(r#"data-qualified-name=".start.""#).unwrap();
        assert!(composite_header < outer_start);
    }

    #[test]
    fn renamed_composite_header_measures_fields_and_explicit_fill() {
        let input = concat!(
            "@startuml\n",
            "state HarborMode : first renamed field\n",
            "state HarborMode : second renamed field\n",
            "state HarborMode #PaleGreen {\n",
            "  [*] --> CopperReady\n",
            "  CopperReady --> VioletRunning\n",
            "  VioletRunning --> [*]\n",
            "}\n",
            "[*] --> HarborMode\n",
            "HarborMode --> [*]\n",
            "@enduml\n",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"fill="#98FB98""##));
        assert!(svg.contains(">first renamed field</text>"));
        assert!(svg.contains(">second renamed field</text>"));
        assert!(svg.contains(r#"data-qualified-name="HarborMode.CopperReady""#));
        assert!(svg.contains(r#"data-qualified-name="HarborMode.VioletRunning""#));
    }

    #[test]
    fn renamed_composite_edge_labels_expand_the_painted_bound() {
        let input = concat!(
            "@startuml\n",
            "state HarborMode {\n",
            "  [*] --> CopperReady\n",
            "  CopperReady --> VioletRunning : unusually wide renamed handoff\n",
            "  VioletRunning --> AmberDone : final renamed step\n",
            "  AmberDone --> [*]\n",
            "}\n",
            "[*] --> HarborMode\n",
            "HarborMode --> [*]\n",
            "@enduml\n",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"width="323px""#), "{svg}");
        assert!(svg.contains(">unusually wide renamed handoff</text>"));
        assert!(svg.contains(">final renamed step</text>"));
    }

    #[test]
    fn state_font_name_skinparams_apply_to_state_and_arrow_text() {
        let input = concat!(
            "@startuml\n",
            "skinparam stateFontName Verdana\n",
            "skinparam stateArrowFontName Verdana\n",
            "[*] --> Idle\n",
            "Idle --> Active : start\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"font-family="Verdana""#));
        assert!(svg.contains(">start</text>"));
    }

    #[test]
    fn promoted_pseudo_state_emits_before_lazy_start() {
        let input = concat!(
            "@startuml\n",
            "[*] --> choice\n",
            "state choice <<choice>>\n",
            "choice --> A\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let choice = svg.find(r#"data-qualified-name="choice""#).unwrap();
        let start = svg.find(r#"data-qualified-name=".start.""#).unwrap();
        assert!(choice < start);
    }

    #[test]
    fn renamed_choice_uses_native_svek_diamond_for_routing() {
        let input = concat!(
            "@startuml\n",
            "[*] --> DecisionHarbor733\n",
            "state DecisionHarbor733 <<choice>>\n",
            "DecisionHarbor733 --> AmberRoute739\n",
            "DecisionHarbor733 --> IndigoRoute743\n",
            "DecisionHarbor733 --> QuartzRoute751\n",
            "@enduml\n",
        );
        let parsed = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::State(diagram) = &parsed else {
            panic!("expected state diagram");
        };
        let choice = diagram
            .states
            .iter()
            .find(|state| state.id == "DecisionHarbor733")
            .unwrap();

        assert_eq!(
            layout_node_size(&choice.id, Some(choice), false).2,
            StateLayoutShape::Diamond
        );
        let svg = render(diagram, &Theme::default());
        assert!(svg.contains(r#"data-qualified-name="DecisionHarbor733""#));
        assert!(svg.contains("<polygon"));
        assert!(svg.matches(r#"<g class="link""#).count() >= 4);
    }

    #[test]
    fn bracket_history_marker_renders_as_pseudo_state() {
        let input = concat!(
            "@startuml\n",
            "[*] --> State1\n",
            "State1 --> State2\n",
            "State2 --> [H]\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(">H</text>"));
        assert!(!svg.contains(r#"data-qualified-name="[H]""#));
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
    fn flat_state_svek_geometry_and_parser_pass_uids() {
        let input = concat!(
            "@startuml\n",
            "state \"Quiescent Delta 43\" as QD43\n",
            "[*] --> QD43\n",
            "QD43 --> [*]\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"height="232px""#));
        assert!(svg.contains(
            r##"data-qualified-name="QD43" id="ent0002"><rect fill="#F1F1F1" height="50""##
        ));
        assert!(svg.contains(r#" x="7" y="86""#));
        assert!(svg.contains(
            r#"data-qualified-name=".start." data-source-line="2" id="ent0002"><ellipse cx="81.65" cy="16""#
        ));
        assert!(svg.contains(
            r#"data-qualified-name=".end." data-source-line="3" id="ent0004"><ellipse cx="81.65" cy="207""#
        ));
        assert!(svg.contains(
            r#"data-entity-1="ent0002" data-entity-2="ent0002" data-link-type="dependency" data-source-line="2" id="lnk3""#
        ));
        assert!(svg.contains(r#"<path d="M81.65,26.26 C81.65,39.95 81.65,60.07 81.65,79.52""#));
        assert!(svg.contains(
            r##"<polygon fill="#181818" points="81.65,85.52,85.65,76.52,81.65,80.52,77.65,76.52,81.65,85.52""##
        ));
        assert!(svg.contains(
            r#"data-entity-1="ent0002" data-entity-2="ent0004" data-link-type="dependency" data-source-line="3" id="lnk5""#
        ));
    }

    #[test]
    fn transition_styles_color_shaft_and_arrowhead() {
        let input = concat!(
            "@startuml\n",
            "skinparam stateArrowColor orange\n",
            "state \"Signal Amber 71\" as SA71\n",
            "state \"Signal Violet 29\" as SV29\n",
            "[*] -[#darkcyan]-> SA71\n",
            "SA71 -[#7B68EE,dashed]-> SV29\n",
            "SV29 --> [*]\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"id="*start*-to-SA71" style="stroke:#008B8B;stroke-width:1;""##));
        assert!(svg.contains(
            r##"id="SA71-to-SV29" style="stroke:#7B68EE;stroke-width:1;stroke-dasharray:7,7;""##
        ));
        assert!(svg.contains(r##"id="SV29-to-*end*" style="stroke:#FFA500;stroke-width:1;""##));
        assert!(svg.contains(r##"<polygon fill="#008B8B""##));
        assert!(svg.contains(r##"<polygon fill="#7B68EE""##));
        assert!(svg.contains(r##"<polygon fill="#FFA500""##));
    }

    #[test]
    fn renamed_styled_down_arrow_uses_its_full_rank_length() {
        let short_input = concat!(
            "@startuml\n",
            "[*] --> SignalHarbor17\n",
            "SignalHarbor17 -down[#darkcyan]-> SignalMesa29\n",
            "SignalMesa29 --> [*]\n",
            "@enduml",
        );
        let long_input = concat!(
            "@startuml\n",
            "[*] --> SignalHarbor17\n",
            "SignalHarbor17 -down[#darkcyan]----> SignalMesa29\n",
            "SignalMesa29 --> [*]\n",
            "@enduml",
        );
        let short = rustuml_parser::parse::parse(short_input).unwrap();
        let long = rustuml_parser::parse::parse(long_input).unwrap();
        let (
            rustuml_parser::diagram::Diagram::State(short),
            rustuml_parser::diagram::Diagram::State(long),
        ) = (&short, &long)
        else {
            panic!("expected state diagrams");
        };

        assert_eq!(transition_svek_minlen(long, &long.transitions[1]), Some(4));
        let short_svg = render(short, &Theme::default());
        let long_svg = render(long, &Theme::default());
        let svg_height = |svg: &str| {
            svg.split_once("height=\"")
                .and_then(|(_, tail)| tail.split_once("px\""))
                .and_then(|(height, _)| height.parse::<f64>().ok())
                .unwrap()
        };
        assert!(svg_height(&long_svg) > svg_height(&short_svg));
    }

    #[test]
    fn renamed_ordinary_arrow_queues_preserve_each_svek_minlen() {
        let input = concat!(
            "@startuml\n",
            "[*] --> QuartzHarbor701\n",
            "QuartzHarbor701 -> AmberRelay709\n",
            "AmberRelay709 ----> IndigoMesa719\n",
            "IndigoMesa719 --> [*]\n",
            "@enduml\n",
        );
        let parsed = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::State(diagram) = &parsed else {
            panic!("expected state diagram");
        };

        let minlens = diagram
            .transitions
            .iter()
            .map(|transition| transition_svek_minlen(diagram, transition))
            .collect::<Vec<_>>();
        assert_eq!(minlens, vec![Some(1), Some(0), Some(3), Some(1)]);
    }

    #[test]
    fn renamed_cardinal_arrows_preserve_layout_order_and_backward_decoration() {
        let input = concat!(
            "@startuml\n",
            "[*] --> CopperRelay17\n",
            "CopperRelay17 -left[#darkcyan]-> AzureDepot29\n",
            "AzureDepot29 -up[#darkcyan]--> VioletHarbor41\n",
            "VioletHarbor41 --> [*]\n",
            "@enduml",
        );
        let parsed = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::State(diagram) = &parsed else {
            panic!("expected state diagram");
        };

        assert_eq!(
            explicit_transition_direction(diagram, &diagram.transitions[1]),
            Some(ExplicitTransitionDirection::Left)
        );
        assert_eq!(
            explicit_transition_direction(diagram, &diagram.transitions[2]),
            Some(ExplicitTransitionDirection::Up)
        );
        assert_eq!(
            transition_svek_minlen(diagram, &diagram.transitions[2]),
            Some(2)
        );

        let svg = render(diagram, &Theme::default());
        // Fresh PlantUML reference. The mixed LEFT/UP inversions exercise
        // promoted starts, the intervening length-one edge stream, and the
        // complete routed-path envelope.
        assert!(svg.contains(r#"viewBox="0 0 298 263""#), "{svg}");
        assert!(svg.contains("<!--reverse link AzureDepot29 to CopperRelay17-->"));
        assert!(svg.contains(r#"id="AzureDepot29-backto-CopperRelay17""#));
        assert!(svg.contains("<!--reverse link VioletHarbor41 to AzureDepot29-->"));
        assert!(svg.contains(r#"id="VioletHarbor41-backto-AzureDepot29""#));
    }

    #[test]
    fn renamed_up_arrow_uses_rectangle_limit_finder_origin() {
        let input = concat!(
            "@startuml\n",
            "[*] --> CopperRelay701\n",
            "CopperRelay701 -up[#darkcyan]-> AzureDepot709\n",
            "AzureDepot709 --> [*]\n",
            "@enduml\n",
        );
        let parsed = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::State(diagram) = &parsed else {
            panic!("expected state diagram");
        };

        let svg = render(diagram, &Theme::default());
        assert!(svg.contains(r#"width="261px""#));
        assert!(svg.contains(r#"height="181px""#));
        assert!(svg.contains(r#"data-qualified-name=".start.""#));
        assert!(svg.contains(r#"<ellipse cx="72.87" cy="32""#));
        assert!(svg.contains(r#"x="119.17" y="7""#));
        assert!(svg.contains(r#"id="AzureDepot709-backto-CopperRelay701""#));
    }

    #[test]
    fn renamed_state_body_merges_text_before_dimension_padding() {
        let input = concat!(
            "@startuml\n",
            "state Meridian701 : phase alpha 709\n",
            "state Meridian701 : phase beta 719\n",
            "state Meridian701 : phase gamma 727\n",
            "[*] --> Meridian701\n",
            "Meridian701 --> [*]\n",
            "@enduml\n",
        );
        let parsed = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::State(diagram) = &parsed else {
            panic!("expected state diagram");
        };

        let svg = render(diagram, &Theme::default());
        assert!(svg.contains(r#"width="149px""#));
        assert!(svg.contains(">phase alpha 709</text>"));
        assert!(svg.contains(">phase beta 719</text>"));
        assert!(svg.contains(">phase gamma 727</text>"));
    }

    #[test]
    fn renamed_transition_label_uses_svek_solved_box() {
        let input = concat!(
            "@startuml\n",
            "[*] --> Signal701\n",
            "Signal701 --> Archive709 : renamed event 719 [gate 727] / commit 733\n",
            "Archive709 --> [*]\n",
            "@enduml\n",
        );
        let parsed = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::State(diagram) = &parsed else {
            panic!("expected state diagram");
        };

        let svg = render(diagram, &Theme::default());
        assert!(svg.contains(r#"width="354px""#));
        assert!(svg.contains(">renamed event 719 [gate 727] / commit 733</text>"));
    }

    #[test]
    fn renamed_state_and_arrow_fonts_resolve_independently() {
        let input = concat!(
            "@startuml\n",
            "skinparam stateFontColor DarkRed\n",
            "skinparam ArrowFontColor DarkCyan\n",
            "[*] --> Signal809\n",
            "Signal809 --> Archive811 : renamed event 821\n",
            "Archive811 --> [*]\n",
            "@enduml\n",
        );
        let parsed = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::State(diagram) = &parsed else {
            panic!("expected state diagram");
        };

        let svg = render(diagram, &Theme::default());
        assert!(svg.contains(r##"<text fill="#8B0000""##));
        assert!(svg.contains(r##"<text fill="#008B8B""##));
        assert!(svg.contains(">renamed event 821</text>"));
    }

    #[test]
    fn renamed_state_gradients_are_seeded_deduplicated_and_policy_aware() {
        let input = concat!(
            "@startuml\n",
            "state CopperRelay701 #cyan/pink\n",
            "state AmberRelay709 #cyan/pink\n",
            "state VioletRelay719 #red|blue\n",
            "[*] --> CopperRelay701\n",
            "CopperRelay701 --> AmberRelay709\n",
            "AmberRelay709 --> VioletRelay719\n",
            "VioletRelay719 --> [*]\n",
            "@enduml\n",
        );
        let parsed = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::State(diagram) = &parsed else {
            panic!("expected state diagram");
        };
        let gradient0 = crate::filter_registry::gradient_id_for(input, 0);
        let gradient1 = crate::filter_registry::gradient_id_for(input, 1);

        let svg = render(diagram, &Theme::default());
        assert!(svg.contains(&format!(
            r##"<linearGradient id="{gradient0}" x1="0%" x2="100%" y1="0%" y2="100%"><stop offset="0%" stop-color="#00FFFF"/><stop offset="100%" stop-color="#FFC0CB"/></linearGradient>"##
        )));
        assert!(svg.contains(&format!(
            r##"<linearGradient id="{gradient1}" x1="0%" x2="100%" y1="50%" y2="50%"><stop offset="0%" stop-color="#FF0000"/><stop offset="100%" stop-color="#0000FF"/></linearGradient>"##
        )));
        assert_eq!(
            svg.matches(&format!(r#"fill="url(#{gradient0})""#)).count(),
            2
        );
        assert_eq!(
            svg.matches(&format!(r#"fill="url(#{gradient1})""#)).count(),
            1
        );
    }

    #[test]
    fn named_start_and_end_states_use_their_entity_background_colors() {
        let input = concat!(
            "@startuml\n",
            "state \"Wake Gate 73\" as WakeGate73 <<start>> #12AB34\n",
            "state \"Archive Vault 91\" as ArchiveVault91 <<end>> #A1B2C3\n",
            "[*] --> WakeGate73\n",
            "WakeGate73 --> ArchiveVault91\n",
            "ArchiveVault91 --> [*]\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"data-qualified-name="WakeGate73""##));
        assert!(svg.contains(r##"<g class="start_entity""##));
        assert!(svg.contains(r##"fill="#12AB34" rx="10" ry="10""##));
        assert!(svg.contains(r##"data-qualified-name="ArchiveVault91""##));
        assert!(svg.contains(r##"<g class="end_entity""##));
        assert!(svg.contains(r##"fill="#A1B2C3" rx="6" ry="6""##));
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
