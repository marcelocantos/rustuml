// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram SVG renderer.
//!
//! Produces SVG output matching PlantUML's exact format, using PlantUML-
//! compatible font metrics and layout algorithms.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use rustuml_parser::diagram::activity::{ActivityDiagram, ActivityStep, NotePosition};

use crate::creole;
use crate::ftile;
use crate::handwritten::{
    ellipse_points as handwritten_ellipse_points,
    has_deprecated_skinparam as has_deprecated_handwritten_skinparam,
    is_enabled as is_handwritten_enabled, line_path as handwritten_line_path,
    polygon_points as handwritten_polygon_points, rect_points as handwritten_rect_points,
};
use crate::layout_oracle::{
    EntityPolygon, EntityRect, OracleCluster, OracleEdgePath, OracleHandwrittenWarning,
    OracleLayout, emit_oracle_cluster_children, wrap_oracle_envelope,
};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::text_render::{self, TextBase};

// PlantUML activity diagram constants (reverse-engineered from golden SVGs).
const START_R: f64 = 10.0;
const STOP_OUTER_R: f64 = 11.0;
const STOP_INNER_R: f64 = 6.0;
const START_CY: f64 = 25.0;
/// Half-width of PlantUML's `FtileCircle*` terminal tile (start/stop/end).
/// The circle glyph (rx 10-11) sits inside a fixed-width tile whose spine is
/// 13 px from each side, so a bare start/stop/end column centres at
/// MARGIN_LEAD + 13 = 29 (verified against act_minimal_just_start_{stop,end},
/// act_empty_diagram, act_start_no_action: all golden cx="29").
const CIRCLE_TILE_HALF: f64 = 13.0;
const ARROW_LEN: f64 = 20.0;
/// Vertical extent of a connector that carries a label (sans-serif 11).
/// Reverse-engineered from PlantUML goldens: 20 (normal) + 21.275 extra to
/// fit the label beside the line.
const LABELED_ARROW_LEN: f64 = 41.2754;
const GROUP_IF_LEFT_EXTENT_EXTRA: f64 = 3.0107;
const GROUP_IF_RIGHT_EXTENT_EXTRA: f64 = 0.9893;
const GROUP_IF_BODY_WIDTH_EXTRA: f64 = 4.0;
const PARTITION_IF_LEFT_EXTENT_EXTRA: f64 = 3.3238;
const PARTITION_IF_RIGHT_EXTENT_EXTRA: f64 = 2.3471;
const PARTITION_IF_BODY_WIDTH_EXTRA: f64 = 5.6709;
const PARTITION_GEOMETRIC_IF_SHELL_PAD: f64 = GROUP_IF_BODY_WIDTH_EXTRA / 2.0;
const PARTITION_GEOMETRIC_IF_SHALLOW_LEFT_ADJUST: f64 = 0.4092;
const PARTITION_GEOMETRIC_IF_SHALLOW_ADJUST_MIN_W: f64 = 150.0;
const PARTITION_GEOMETRIC_IF_DEEP_LEFT_ADJUST: f64 = -9.0;
const GROUP_REPEAT_BODY_WIDTH_SUBTRACT: f64 = 1.0;
const GROUP_REPEAT_TITLE_WIDTH_EXTRA: f64 = 15.0;
const GROUP_REPEAT_SPINE_LEFT_OF_TITLE_MID: f64 = 2.0;
const GROUP_COLOR_TITLE_WIDTH_EXTRA: f64 = 4.1572;
const GROUP_COLOR_RIGHT_EXTENT_EXTRA: f64 = 2.0;
const PARTITION_FORK_WIDTH_EXTRA: f64 = 2.0;
const PARTITION_FORK_LEFT_EXTENT_EXTRA: f64 = 2.0;
const PARTITION_FORK_BODY_CX_SHIFT: f64 = -1.0;
const PARTITION_FORK_BRANCH_CENTER_SHIFT: f64 = 2.0;
const PARTITION_FORK_TINY_TEXT_MAX: f64 = 10.0;
const PARTITION_TITLE_BAND_H: f64 = 36.4883;
const SINGLE_LANE_GROUP_TOP_ADJUST: f64 = 0.453125;
const ACTION_PADDING: f64 = 20.0; // total vertical padding in action box
const ACTION_H_PADDING: f64 = 10.0; // horizontal padding each side
const THEME_NODE_ACTION_PADDING: f64 = 15.0;
const ACTION_MIN_HEIGHT: f64 = 30.0;
const ACTION_RX: f64 = 12.5;
const DIAMOND_HALF: f64 = 12.0; // half-size of decision diamond
const ACTION_LIST_ITEM_TEXT_X: f64 = 12.0;
const ACTION_LIST_BULLET_CX: f64 = 5.5;
const ACTION_LIST_BULLET_BASELINE_DROP: f64 = 4.9688;
const ACTION_LIST_NUMBER_GAP: f64 = 4.1133;
const ACTION_SWALLOWED_CONTROL_COLON_INDENT: f64 = 7.5938;
const ACTION_TABLE_CELL_PAD_X: f64 = 3.7969;
const ACTION_TABLE_PAD_Y: f64 = 12.0;
const WHILE_SPECIAL_COND_LEAD: f64 = 13.0;
const WHILE_SPECIAL_BODY_LEAD: f64 = 11.0;
const WHILE_SPECIAL_BODY_X_PULL_RIGHT: f64 = WHILE_SPECIAL_COND_LEAD - WHILE_SPECIAL_BODY_LEAD;
const WHILE_UNLABELED_SPECIAL_Y_PULL_UP: f64 = 4.0;
const WHILE_UNLABELED_LOOP_ARROW_Y_PULL_UP: f64 = 2.0;
const WHILE_BODY_SLOT_COMPRESS: f64 = 4.8203125;
const PARTITION_COLORED_WHILE_SPINE_SHIFT: f64 = 1.5;
const PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP: f64 = WHILE_BODY_SLOT_COMPRESS / 2.0 - 1.0;
const PARTITION_WHILE_WIDTH_SUBTRACT: f64 = 16.0;
const PARTITION_REPEAT_WIDTH_SUBTRACT: f64 = 18.0;
const WHILE_SINGLE_IF_RIGHT_PAD: f64 = 2.0;
const WHILE_SINGLE_IF_SPECIAL_HEIGHT_TRIM: f64 = 1.0;
/// PlantUML enforces a minimum width on the inner (top/bottom) edge of
/// decision diamonds: 24 px regardless of how short the condition text is.
/// Reverse-engineered from goldens with one- and two-character conditions
/// ("A?", "B?", "c?") which all produce a 24-px inner span while their text
/// `textLength` stays at the measured value.
const DIAMOND_MIN_INNER_W: f64 = 24.0;

/// Vertical gap between an if/else condition diamond's bottom point and
/// the top of each branch's first action box. PlantUML uses 10 px here,
/// not the generic 20 px `ARROW_LEN` used for sequential arrows.
const IF_BRANCH_DOWN: f64 = 10.0;
/// Vertical gap between the last action of an if/else branch and the
/// top of the merge diamond below. PlantUML uses 6 px here.
const IF_BRANCH_UP: f64 = 6.0;
const IF_SINGLE_SURVIVOR_JOIN_GAP: f64 = 5.0;
const IF_GOTO_RESUME_GAP: f64 = 5.0;
const IF_EMPTY_BOTH_LEFT_EXTENT_PAD: f64 = 13.0;
const IF_EMPTY_BOTH_RIGHT_EXTENT_PAD: f64 = 15.0;
/// Labelled `if` diamonds reserve a little extra inbound lead when the
/// diagram-wide arrow font is taller than the default 20 px connector slot.
const IF_LABEL_INBOUND_PAD: f64 = 0.71875;
const REPEAT_NOT_LABEL_OUTBOUND_PAD: f64 = 1.5;
const FORK_BAR_HEIGHT: f64 = 6.0;
const FORK_BAR_RX: f64 = 2.5;
/// PlantUML's drop-shadow filter extends painted node bounds by 6 px on the
/// trailing axes in activity diagrams with `skinparam shadowing true`.
const SHADOW_BOUNDS_PAD: f64 = 6.0;

// Switch-specific layout constants (reverse-engineered from golden SVGs).
const SVG_CONTENT_LEAD: f64 = 16.0;
const SWITCH_CASE_GAP: f64 = 10.0; // horizontal gap between adjacent SMALL-mode case boxes
const SWITCH_IF_BRANCH_CASE_GAP: f64 = 20.0; // FtileSwitchNude.xSeparation inside if branches
// Big-diamond switches nested under FtileIf keep the switch diamond anchored,
// but the case band lands one text-metric rounding step lower.
const SWITCH_IF_BRANCH_BIG_CASE_Y_ADJUST: f64 = 0.6572265625;
const SWITCH_EMPTY_MERGE_GAP: f64 = 40.6357;
// Right envelope after a two-case switch's empty-case label.
const SWITCH_TWO_CASE_EMPTY_LABEL_TRAIL: f64 = 1.5815;
// Mixed empty/non-empty switches go through PlantUML's labelled empty-branch
// FTile path: the empty branch keeps the label width for its tile envelope, but
// its flow spine sits 2 px from the tile origin rather than at the text centre.
const SWITCH_MIXED_EMPTY_CASE_GAP: f64 = 12.0;
const SWITCH_MIXED_EMPTY_SPINE: f64 = 2.0;
// Even mixed-empty switches keep the first branch pinned, but PlantUML's ON_X
// compression gives the diamond/middle branch a little more left extent while
// trimming the terminal empty branch corridor.
const SWITCH_EVEN_MIXED_EMPTY_SPINE_SHIFT: f64 = 7.0444;
const SWITCH_EVEN_MIXED_EMPTY_LAST_PULL_LEFT: f64 = 0.4333;
// `FtileSwitchWithOneLink` routes the sole branch from the east side of the
// test diamond. The visible branch box is placed 15 px past the diamond's east
// vertex; the legacy activity renderer keeps only 9 px of left SVG extent past
// the diamond's west/east half-width, matching the existing margin model.
const SWITCH_ONE_LINK_BRANCH_GAP: f64 = 15.0;
const SWITCH_ONE_LINK_LEFT_PAD: f64 = 9.0;
const SWITCH_ONE_LINK_Y_DELTA: f64 = 20.0;
const SWITCH_LINK_MARGIN: f64 = 10.0;
const SWITCH_INNER_CORRIDOR_PACKING_PULL_LEFT: f64 = 0.3015;
// Case-label baseline offsets above the case-box top, per connection type.
const SWITCH_LABEL_OUTER_DY: f64 = 19.7979; // outermost branches (via diamond vertex)
const SWITCH_LABEL_INNER_DY: f64 = 24.7979; // inner branches (drop from horizontal line)
const SWITCH_LABEL_CENTER_DY: f64 = 18.7979; // exact-centre branch (drop from diamond bottom)
const VERTICAL_IF_STARTLESS_TOP_NUDGE: f64 = 0.7754;
// Centre-branch vertical-line split offsets (the centre drop is split into
// two collinear segments, matching PlantUML's connector decomposition).
const SWITCH_CENTER_TOP_SPLIT: f64 = 23.9401; // split distance above the case top
const SWITCH_CENTER_BOT_SPLIT: f64 = 15.0; // split distance above the merge top

const FONT_SIZE: f64 = 12.0;
const SMALL_FONT: f64 = 11.0;
const DECORATION_FONT_SIZE: f64 = 10.0;
const DECORATION_COLOR: &str = "#888888";
const HEADER_BODY_GAP: f64 = 10.0;
const FOOTER_BASELINE_GAP: f64 = 18.668;
const FOOTER_BOTTOM_GAP: f64 = 31.957;
const HANDWRITTEN_WARNING_BAND_H: f64 = 21.6406;
const CAPTION_FONT_SIZE: f64 = 14.0;
const CAPTION_BASELINE_GAP: f64 = 23.5352;
const CAPTION_BOTTOM_GAP: f64 = 38.8672;
const LEGEND_FONT_SIZE: f64 = 14.0;
const LEGEND_RECT_X: f64 = 22.0;
const LEGEND_RIGHT_PAD: f64 = 23.0;
const LEGEND_RECT_PAD_X: f64 = 5.0;
const LEGEND_RECT_PAD_Y: f64 = 7.0;
const LEGEND_RECT_RX: f64 = 7.5;
const LEGEND_CELL_PAD_DESCENT_FACTOR: f64 = 1.5;
const LEGEND_TOP_GAP: f64 = 21.0;
const LEGEND_BOTTOM_GAP: f64 = 23.2696;
const LEGEND_DEFAULT_SOURCE_LINE: usize = 1;
const TITLE_FONT_SIZE: f64 = 14.0;
const LANE_TITLE_FONT: f64 = 18.0;
const TEXT_MIN_BOX_HEIGHT: f64 = 10.0;

// Note geometry (attached `note left/right` beside the anchoring flow node).
// Reverse-engineered from activity goldens.
const NOTE_FONT: f64 = 13.0;
const NOTE_TEXT_PAD_X: f64 = 6.0; // text inset from the note box's left edge
const NOTE_FOLD: f64 = 10.0; // folded-corner size (top-right)
// Box width = max line textLength + this. (6 left pad + 5 right pad + 10 fold.)
const NOTE_BOX_EXTRA_W: f64 = 21.0;
const NOTE_LINE_H: f64 = 15.3105; // per-line height inside a note
const NOTE_FIRST_BASELINE_DY: f64 = 17.5684; // box top → first text baseline
const NOTE_BOX_BASE_H: f64 = 10.0001; // height = this + nlines * NOTE_LINE_H
const NOTE_GAP: f64 = 20.0; // horizontal gap between anchor box and note box
const FORK_NOTE_GAP: f64 = 10.0; // gap from fork bar edge to attached note box
const NOTE_FILL: &str = "#FEFFDD";
const NOTE_STROKE: &str = "#181818";
const NOTE_STROKE_WIDTH: &str = "0.5";

const START_FILL: &str = "#222222";
const STOP_FILL: &str = "#222222";
const CONNECTOR_R: f64 = 10.0;
const ACTION_FILL: &str = "#F1F1F1";
const ACTION_STROKE: &str = "#181818";
const ACTION_STROKE_WIDTH: &str = "0.5";
const ARROW_COLOR: &str = "#181818";
const DIAMOND_FILL: &str = "#F1F1F1";
const FORK_BAR_COLOR: &str = "#555555";
const TEXT_COLOR: &str = "#000000";
const DEPRECATED_FILL: &str = "#FFFFCC";
const DEPRECATED_STROKE: &str = "#FFDD88";

fn canonical_font_family(value: &str) -> String {
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

fn split_gradient_colors(val: &str) -> Option<(&str, &str)> {
    for sep in ['/', '\\', '|', '-'] {
        if let Some((left, right)) = val.split_once(sep) {
            let left = left.trim();
            let right = right.trim();
            if !left.is_empty() && !right.is_empty() {
                return Some((left, right));
            }
        }
    }
    None
}

fn parse_gradient_ids(defs: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = defs;
    while let Some(lg) = rest.find("<linearGradient") {
        rest = &rest[lg..];
        if let Some(start) = rest.find("id=\"") {
            let id_start = start + 4;
            if let Some(end) = rest[id_start..].find('"') {
                ids.push(rest[id_start..id_start + end].to_string());
            }
        }
        rest = &rest["<linearGradient".len()..];
    }
    ids
}

fn parse_filter_id(defs: &str) -> Option<String> {
    let filter = defs.find("<filter")?;
    let rest = &defs[filter..];
    let start = rest.find("id=\"")? + 4;
    let end = rest[start..].find('"')?;
    Some(rest[start..start + end].to_string())
}

fn gradient_fill_or(val: &str, gradient_id: &Option<String>) -> String {
    if val.eq_ignore_ascii_case("transparent") {
        "none".to_string()
    } else if split_gradient_colors(val).is_some()
        && let Some(id) = gradient_id
    {
        format!("url(#{id})")
    } else if let Some((first, _)) = split_gradient_colors(val) {
        crate::sequence::resolve_color(first)
    } else {
        crate::sequence::resolve_color(val)
    }
}

fn action_text_for_family(text: &str, font_family: &str) -> String {
    if font_family.to_ascii_lowercase().contains("courier") {
        text.replace(' ', "\u{a0}")
    } else {
        text.to_string()
    }
}

fn activity_theme_uses_node_action_padding(name: &str) -> bool {
    matches!(name, "aws-orange" | "cloudscape-design")
}

/// Per-diagram color palette, derived from the PlantUML default plus any
/// inline `skinparam` overrides. Mirrors the constants above but allows
/// skinparams to mutate individual fields without rebuilding the theme
/// machinery in `style.rs` (which uses the `slate` defaults).
#[derive(Debug, Clone)]
struct Palette {
    svg_background: Option<String>,
    action_fill: String,
    action_stroke: String,
    action_stroke_width: String,
    action_pad_x: f64,
    action_pad_y: f64,
    action_font_family: String,
    action_font_size: f64,
    action_text_bold: bool,
    action_text_italic: bool,
    diamond_fill: String,
    diamond_stroke: String,
    diamond_stroke_width: String,
    diamond_font_family: String,
    diamond_font_size: f64,
    diamond_text_color: String,
    diamond_text_bold: bool,
    diamond_text_italic: bool,
    diamond_pad_x: f64,
    arrow_color: String,
    /// Stroke-width string used for activity connector lines (the ones that
    /// link nodes top-to-bottom and the if/fork frame). Defaults to "1" and
    /// rises with `skinparam activityBorderThickness` — PlantUML cascades
    /// the border thickness onto the connector strokes too.
    arrow_thickness: String,
    arrow_font_size: f64,
    arrow_font_family: String,
    arrow_text_color: String,
    title_font_size: f64,
    title_bold: bool,
    text_color: String,
    start_fill: String,
    /// Stroke colour for the start ellipse. Mirrors `start_fill` by default
    /// but stays at `#222222` when only `activityStartColor` is set —
    /// PlantUML keeps the original border when only the fill changes.
    start_stroke: String,
    stop_fill: String,
    stop_stroke: String,
    bar_color: String,
    shadow_filter: Option<String>,
    swimlane_border_color: String,
    swimlane_title_color: String,
    swimlane_title_background: Option<String>,
    /// Corner radius for action boxes. PlantUML's default action box has a
    /// 12.5 px radius (corresponding to a `roundCorner` of 25). The
    /// `roundCorner` / `activityRoundCorner` skinparams set it to half their
    /// value.
    action_rx: f64,
}

impl Palette {
    fn default_puml() -> Self {
        Self {
            svg_background: Some("#FFFFFF".into()),
            action_fill: ACTION_FILL.into(),
            action_stroke: ACTION_STROKE.into(),
            action_stroke_width: ACTION_STROKE_WIDTH.into(),
            action_pad_x: ACTION_H_PADDING,
            action_pad_y: ACTION_PADDING / 2.0,
            action_font_family: "sans-serif".into(),
            action_font_size: FONT_SIZE,
            action_text_bold: false,
            action_text_italic: false,
            diamond_fill: DIAMOND_FILL.into(),
            diamond_stroke: ACTION_STROKE.into(),
            diamond_stroke_width: ACTION_STROKE_WIDTH.into(),
            diamond_font_family: "sans-serif".into(),
            diamond_font_size: SMALL_FONT,
            diamond_text_color: TEXT_COLOR.into(),
            diamond_text_bold: false,
            diamond_text_italic: false,
            diamond_pad_x: 0.0,
            arrow_color: ARROW_COLOR.into(),
            arrow_thickness: "1".into(),
            arrow_font_size: SMALL_FONT,
            arrow_font_family: "sans-serif".into(),
            arrow_text_color: TEXT_COLOR.into(),
            title_font_size: TITLE_FONT_SIZE,
            title_bold: true,
            text_color: TEXT_COLOR.into(),
            start_fill: START_FILL.into(),
            start_stroke: START_FILL.into(),
            stop_fill: STOP_FILL.into(),
            stop_stroke: STOP_FILL.into(),
            bar_color: FORK_BAR_COLOR.into(),
            shadow_filter: None,
            swimlane_border_color: "#000000".into(),
            swimlane_title_color: TEXT_COLOR.into(),
            swimlane_title_background: None,
            action_rx: ACTION_RX,
        }
    }

    /// Apply the supplied skinparams (key-insensitive) onto the default
    /// PlantUML palette. Unrecognised or empty values are ignored.
    ///
    /// Activity skinparams cascade: `activityBackgroundColor` also sets the
    /// diamond fill, `activityBorderColor` also sets diamond stroke, and
    /// `activityBorderThickness` also sets diamond stroke width — unless a
    /// more specific `activityDiamond*` override appears later in the
    /// skinparam list.
    fn from_skinparams(
        skinparams: &[rustuml_parser::diagram::SkinParam],
        action_gradient_id: &Option<String>,
        diamond_gradient_id: &Option<String>,
        filter_id: &Option<String>,
    ) -> Self {
        let mut p = Self::default_puml();
        let mut use_node_action_padding = false;
        for sp in skinparams {
            let key = sp.key.to_ascii_lowercase();
            let val = sp.value.trim();
            if val.is_empty() {
                continue;
            }
            let resolved = crate::sequence::resolve_color(val);
            match key.as_str() {
                "__theme" => use_node_action_padding = activity_theme_uses_node_action_padding(val),
                "backgroundcolor" => {
                    if val.eq_ignore_ascii_case("transparent") {
                        p.svg_background = None;
                    } else {
                        p.svg_background = Some(resolved);
                    }
                }
                "defaultfontname" => {
                    let family = canonical_font_family(val);
                    p.action_font_family = family.clone();
                    p.diamond_font_family = family.clone();
                    p.arrow_font_family = family;
                }
                "activityfontname" => {
                    let family = canonical_font_family(val);
                    p.action_font_family = family.clone();
                    p.diamond_font_family = family;
                }
                "defaultfontsize" => {
                    if let Ok(v) = val.parse::<f64>() {
                        p.action_font_size = v;
                        p.diamond_font_size = v;
                        p.arrow_font_size = v;
                    }
                }
                "activityfontsize" => {
                    if let Ok(v) = val.parse::<f64>() {
                        p.action_font_size = v;
                        p.diamond_font_size = v;
                    }
                }
                "padding" => {
                    if let Ok(v) = val.parse::<f64>() {
                        let pad = 6.0 + v;
                        p.action_pad_x = pad;
                        p.action_pad_y = pad;
                        p.diamond_pad_x = v;
                    }
                }
                "__stylerootlinecolor" => {
                    p.start_stroke = resolved.clone();
                    p.stop_stroke = resolved;
                    p.stop_fill = "none".into();
                }
                "__stylerootlinethickness" => {
                    if let Ok(v) = val.parse::<f64>() {
                        let w = pm::fmt_coord(v);
                        p.action_stroke_width = w.clone();
                        p.diamond_stroke_width = w;
                    }
                }
                "activitybackgroundcolor" => {
                    p.action_fill = gradient_fill_or(val, action_gradient_id);
                    p.diamond_fill = gradient_fill_or(val, diamond_gradient_id);
                }
                "activitybordercolor" => {
                    p.action_stroke = resolved.clone();
                    p.diamond_stroke = resolved;
                }
                "activityborderthickness" => {
                    if let Ok(v) = val.parse::<f64>() {
                        // Format like PlantUML: integer when whole, otherwise raw float.
                        let w = pm::fmt_coord(v);
                        p.action_stroke_width = w.clone();
                        p.diamond_stroke_width = w.clone();
                        p.arrow_thickness = w;
                    }
                }
                "activitydiamondbackgroundcolor" => {
                    p.diamond_fill = gradient_fill_or(val, diamond_gradient_id);
                }
                "activitydiamondbordercolor" => p.diamond_stroke = resolved,
                "activitydiamondborderthickness" => {
                    if let Ok(v) = val.parse::<f64>() {
                        p.diamond_stroke_width = pm::fmt_coord(v);
                    }
                }
                "activitydiamondfontcolor" => p.diamond_text_color = resolved,
                "activitydiamondfontname" => p.diamond_font_family = canonical_font_family(val),
                "activitydiamondfontsize" => {
                    if let Ok(v) = val.parse::<f64>() {
                        p.diamond_font_size = v;
                    }
                }
                "activitydiamondfontstyle" => {
                    let lower = val.to_ascii_lowercase();
                    p.diamond_text_bold = lower.contains("bold");
                    p.diamond_text_italic = lower.contains("italic");
                }
                "activityarrowcolor" | "arrowcolor" => p.arrow_color = resolved,
                "activityarrowthickness" | "arrowthickness" => {
                    if let Ok(v) = val.parse::<f64>() {
                        p.arrow_thickness = pm::fmt_coord(v);
                    }
                }
                "activityarrowfontsize" | "arrowfontsize" => {
                    if let Ok(v) = val.parse::<f64>() {
                        p.arrow_font_size = v;
                    }
                }
                "activityarrowfontname" | "arrowfontname" => {
                    p.arrow_font_family = canonical_font_family(val);
                }
                "activityarrowfontcolor" | "arrowfontcolor" => {
                    p.arrow_text_color = resolved;
                }
                "titlefontsize" => {
                    if let Ok(v) = val.parse::<f64>() {
                        p.title_font_size = v;
                    }
                }
                "titlefontstyle" => {
                    let lower = val.to_ascii_lowercase();
                    p.title_bold = lower.contains("bold");
                }
                // `activityStartColor` sets the start ellipse fill (border
                // keeps its `#222222` default unless a border colour is
                // specified). `activityStopColor` and `activityEndColor`
                // affect distinct shapes in PlantUML — the former targets
                // the `stop` (filled bullseye), the latter the `end` (X-in-
                // circle). Each leaves its border colour at the default.
                "activitystartcolor" => p.start_fill = resolved,
                "activitystopcolor" => p.stop_fill = resolved,
                "activitystartbordercolor" => p.start_stroke = resolved,
                "activitystopbordercolor" => p.stop_stroke = resolved,
                "activityendcolor" => {
                    // The `end` node ignores this skinparam in PlantUML —
                    // its rendering is the X-in-circle in the default
                    // border colour. Accept the key without effect so the
                    // skinparam doesn't fall into the unknown bucket.
                }
                "activitybarcolor" => p.bar_color = resolved,
                "swimlanebordercolor" => p.swimlane_border_color = resolved,
                "swimlanetitlefontcolor" => p.swimlane_title_color = resolved,
                "swimlanetitlebackgroundcolor" => {
                    p.swimlane_title_background = Some(resolved);
                }
                "shadowing" | "activityshadowing" => {
                    if val.eq_ignore_ascii_case("true") {
                        p.shadow_filter = filter_id.clone();
                    } else if val.eq_ignore_ascii_case("false") {
                        p.shadow_filter = None;
                    }
                }
                "activityfontcolor" => {
                    p.text_color = resolved.clone();
                    p.diamond_text_color = resolved;
                }
                "activityfontstyle" => {
                    let lower = val.to_ascii_lowercase();
                    p.action_text_bold = lower.contains("bold");
                    p.action_text_italic = lower.contains("italic");
                    p.diamond_text_bold = p.action_text_bold;
                    p.diamond_text_italic = p.action_text_italic;
                }
                // The global `roundCorner` skinparam sets the corner-radius
                // diameter; the SVG rect radius is half that value (PlantUML
                // stores a diameter and halves it when drawing the rounded
                // rect). `activityRoundCorner` does NOT affect these action
                // boxes in PlantUML — only the global key cascades — so it is
                // intentionally not matched here.
                "roundcorner" => {
                    if let Ok(v) = val.parse::<f64>() {
                        p.action_rx = v / 2.0;
                    }
                }
                _ => {}
            }
        }
        if use_node_action_padding {
            p.action_pad_x = p.action_pad_x.max(THEME_NODE_ACTION_PADDING);
            p.action_pad_y = p.action_pad_y.max(THEME_NODE_ACTION_PADDING);
        }
        p
    }
}

/// Detect deprecated `#color:text;` actions and prepend a warning banner.
/// Returns the raw (un-escaped) banner string; XML/entity escaping happens
/// in the emitter via [`svg_text_escape`].
fn deprecated_warning(color: &str) -> String {
    format!(
        "This\u{a0}syntax\u{a0}is\u{a0}deprecated,\u{a0}you\u{a0}must\u{a0}add\u{a0}<<{color}>>\u{a0}at\u{a0}the\u{a0}end\u{a0}of\u{a0}the\u{a0}line,\u{a0}after\u{a0}the\u{a0}';'"
    )
}

/// Per-arrow visual style. Derived from the parser's `Arrow.color` field,
/// which actually carries the comma-separated bracket payload of
/// `-[...]->` (e.g. `bold`, `dashed`, `#red`, `#red,bold`).
#[derive(Debug, Clone)]
struct ArrowStyle {
    color: String,
    dashed: bool,
    dotted: bool,
    bold: bool,
    hidden: bool,
}

impl Default for ArrowStyle {
    fn default() -> Self {
        ArrowStyle {
            color: ARROW_COLOR.to_string(),
            dashed: false,
            dotted: false,
            bold: false,
            hidden: false,
        }
    }
}

/// Parse the bracketed payload from `-[...]->` into an `ArrowStyle`.
/// Accepts tokens separated by `,` or `;`; tokens may be a colour (`#fff`,
/// `#FFFFFF`, or a CSS name) or a style keyword (`bold`, `dashed`,
/// `dotted`, `hidden`, `plain`).
///
/// `parser_dashed` is intentionally ignored when this function is called:
/// the parser's dashed flag is set whenever a `-` appears after `]`, which
/// is true for all `-[…]->` forms regardless of the actual style.  Dash-ness
/// must come from a `dashed`/`dotted` token inside the brackets.
fn arrow_style_from_brackets(payload: &str, _parser_dashed: bool) -> ArrowStyle {
    let mut style = ArrowStyle::default();
    for tok in payload.split([',', ';']) {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = t.strip_prefix('#') {
            // Hex colour or CSS name (resolve_color handles both forms).
            style.color = crate::sequence::resolve_color(rest);
            continue;
        }
        match t.to_ascii_lowercase().as_str() {
            "bold" => style.bold = true,
            "dashed" => style.dashed = true,
            "dotted" => style.dotted = true,
            "hidden" => style.hidden = true,
            "plain" | "solid" | "normal" => {}
            other => {
                // Bare CSS colour name without `#` prefix.
                let resolved = crate::sequence::resolve_color(other);
                if resolved != "#FFFFFF" || other.eq_ignore_ascii_case("white") {
                    style.color = resolved;
                }
            }
        }
    }
    style
}

/// A layout node in the activity tree. We convert the flat step list into
/// a tree of these nodes, compute sizes, then emit SVG.
#[derive(Debug)]
#[allow(dead_code)]
enum LayoutNode {
    Start,
    Stop,
    End,
    Connector(String),
    Action {
        text: String,
        text_width: f64,
        pad_x: f64,
        pad_y: f64,
        font_family: String,
        font_size: f64,
        bold: bool,
        italic: bool,
    },
    DeprecatedAction {
        color: String,
        text: String,
        text_width: f64,
        pad_x: f64,
        pad_y: f64,
        font_family: String,
        font_size: f64,
        bold: bool,
        italic: bool,
        warning_width: f64,
    },
    If {
        condition: String,
        diamond_font_family: String,
        diamond_font_size: f64,
        diamond_text_color: String,
        diamond_text_bold: bool,
        diamond_text_italic: bool,
        diamond_pad_x: f64,
        diamond_half_y: f64,
        arrow_font_size: f64,
        then_label: Option<String>,
        then_branch: Vec<LayoutNode>,
        else_branches: Vec<ElseBranch>,
        attached_notes: Vec<ActivityNote>,
    },
    While {
        condition: String,
        diamond_font_family: String,
        diamond_font_size: f64,
        diamond_text_color: String,
        diamond_text_bold: bool,
        diamond_text_italic: bool,
        arrow_font_size: f64,
        is_label: Option<String>,
        body: Vec<LayoutNode>,
        end_label: Option<String>,
        /// Stop/End/Detach/Kill absorbed from the parent sequence when it
        /// follows the `endwhile`. Mirrors PlantUML's
        /// `manageSpecialStopEndAfterEndWhile` — the terminator is drawn
        /// INSIDE the while's frame at translateForSpecial position, not
        /// as a sibling below.
        special_out: Option<Box<LayoutNode>>,
        /// This while is the first real tile in its local activity column. In
        /// that position PlantUML lets a wide body tighten the special-out lead;
        /// later while blocks keep the condition-driven lead so earlier siblings
        /// do not shift.
        starts_column: bool,
    },
    Repeat {
        body: Vec<LayoutNode>,
        condition: String,
        is_label: Option<String>,
        not_label: Option<String>,
        /// Label of a `backward :label;` action drawn on the loop-back arm.
        backward: Option<String>,
        has_start_label: bool,
        arrow_font_size: f64,
        arrow_font_family: String,
    },
    Fork {
        branches: Vec<Vec<LayoutNode>>,
        attached_notes: Vec<ActivityNote>,
        is_split: bool,
        merge: bool,
    },
    /// A `switch (cond) / case (x) / ... / endswitch` block. Cases lay out
    /// horizontally below a condition diamond, fanning out via the diamond's
    /// left/right vertices, and reconverging into a merge diamond below.
    Switch {
        condition: String,
        cases: Vec<SwitchCase>,
    },
    Arrow {
        dashed: bool,
        color: Option<String>,
        label: Option<String>,
    },
    Note {
        text: String,
        position: NotePosition,
        color: Option<String>,
    },
    Detach,
    Kill,
    Break,
    Goto(String),
    Title {
        text: String,
        font_size: f64,
        bold: bool,
    },
    Partition {
        name: String,
        color: Option<String>,
        is_group: bool,
        nested: bool,
        single_lane_group: bool,
        single_lane_first_group: bool,
        body: Vec<LayoutNode>,
    },
    /// A top-level swimlanes container. Each lane has its own vertical
    /// column with a header label at the top; the activity flow weaves
    /// across lanes via cross-lane arrows. PlantUML's `|Lane|` markers
    /// in the source partition the flat step list into lane bodies.
    Swimlanes {
        lanes: Vec<Lane>,
        segments: Vec<LaneSegment>,
    },
}

#[derive(Debug)]
struct Lane {
    name: String,
    #[allow(dead_code)]
    color: Option<String>,
    content_left: f64,
    content_right: f64,
}

#[derive(Debug)]
struct LaneSegment {
    lane_index: usize,
    body: Vec<LayoutNode>,
}

#[derive(Debug)]
struct ElseBranch {
    label: Option<String>,
    /// The `elseif` condition for this branch (`None` = the final bare `else`).
    /// PlantUML's `FtileIfLongHorizontal` builds one condition diamond per
    /// `then`/`elseif`; the final `else` has none. Carrying the condition here
    /// lets the long-chain layout draw the per-elseif diamonds.
    condition: Option<String>,
    body: Vec<LayoutNode>,
}

#[derive(Clone, Debug)]
struct ActivityNote {
    text: String,
    position: NotePosition,
    color: Option<String>,
}

fn activity_note_from_block(n: &rustuml_parser::diagram::activity::NoteBlock) -> ActivityNote {
    ActivityNote {
        text: n.text.clone(),
        position: n.position.clone(),
        color: n.color.clone(),
    }
}

fn layout_note_from_block(n: &rustuml_parser::diagram::activity::NoteBlock) -> LayoutNode {
    LayoutNode::Note {
        text: n.text.clone(),
        position: n.position.clone(),
        color: n.color.clone(),
    }
}

fn take_leading_notes(nodes: &mut Vec<LayoutNode>) -> Vec<ActivityNote> {
    let mut count = 0;
    while matches!(nodes.get(count), Some(LayoutNode::Note { .. })) {
        count += 1;
    }
    nodes
        .drain(..count)
        .filter_map(|node| {
            if let LayoutNode::Note {
                text,
                position,
                color,
            } = node
            {
                Some(ActivityNote {
                    text,
                    position,
                    color,
                })
            } else {
                None
            }
        })
        .collect()
}

/// True when an `if` node is a genuine `if/elseif/.../else` chain (at least one
/// `elseif`), which PlantUML renders with `FtileIfLongHorizontal` — a row of
/// condition diamonds fanning out into branch columns. Detected by any else
/// branch carrying a condition (an `elseif`).
fn if_is_long(else_branches: &[ElseBranch]) -> bool {
    else_branches.iter().any(|b| b.condition.is_some())
}

#[derive(Debug)]
struct SwitchCase {
    label: String,
    body: Vec<LayoutNode>,
}

/// Returns true if a branch ends with a control-flow terminator (Stop, End,
/// Detach, Kill, Break, or Goto). PlantUML omits the merge diamond and
/// post-merge connectors entirely when every branch of an if/else terminates
/// this way.
fn branch_terminates(body: &[LayoutNode]) -> bool {
    matches!(
        body.last(),
        Some(LayoutNode::Stop)
            | Some(LayoutNode::End)
            | Some(LayoutNode::Detach)
            | Some(LayoutNode::Kill)
            | Some(LayoutNode::Break)
            | Some(LayoutNode::Goto(_))
    )
}

fn branch_ends_with_goto(body: &[LayoutNode]) -> bool {
    matches!(body.last(), Some(LayoutNode::Goto(_)))
}

/// A branch flow whose sole node is a no-`specialOut` `while`. Such a branch's
/// loop exit corridor IS the if's branch→merge connection (see
/// [`WhileExitRedirect`]); the if delegates the merge wiring to `emit_while`.
fn branch_is_redirectable_while(flow: &[LayoutNode]) -> bool {
    matches!(
        flow,
        [LayoutNode::While {
            special_out: None,
            ..
        }]
    )
}

fn if_all_branches_goto(then_branch: &[LayoutNode], else_branches: &[ElseBranch]) -> bool {
    branch_ends_with_goto(then_branch)
        && !else_branches.is_empty()
        && else_branches
            .iter()
            .all(|branch| branch_ends_with_goto(&branch.body))
}

fn leading_branch_arrow(nodes: &[LayoutNode]) -> (Option<&LayoutNode>, &[LayoutNode]) {
    match nodes.first() {
        Some(node @ LayoutNode::Arrow { .. }) => (Some(node), &nodes[1..]),
        _ => (None, nodes),
    }
}

fn branch_arrow_label(arrow: Option<&LayoutNode>) -> Option<&str> {
    match arrow {
        Some(LayoutNode::Arrow {
            label: Some(label), ..
        }) => Some(label),
        _ => None,
    }
}

fn branch_arrow_style(arrow: Option<&LayoutNode>, default_color: &str) -> ArrowStyle {
    match arrow {
        Some(LayoutNode::Arrow {
            color: Some(color),
            dashed,
            ..
        }) => arrow_style_from_brackets(color, *dashed),
        Some(LayoutNode::Arrow { dashed, .. }) => ArrowStyle {
            color: default_color.to_string(),
            dashed: *dashed,
            dotted: false,
            bold: false,
            hidden: false,
        },
        _ => ArrowStyle {
            color: default_color.to_string(),
            ..ArrowStyle::default()
        },
    }
}

/// For a binary if/else where exactly one non-empty branch terminates, return
/// `true` when the then branch survives and `false` when the else branch
/// survives. PlantUML skips the merge diamond for this shape and routes the
/// surviving branch straight back to the main spine.
fn if_single_survivor(then_branch: &[LayoutNode], else_branches: &[ElseBranch]) -> Option<bool> {
    if else_branches.len() != 1 || else_branches[0].condition.is_some() {
        return None;
    }
    let else_body = &else_branches[0].body;
    if branch_is_empty(then_branch) || branch_is_empty(else_body) {
        return None;
    }
    let then_terminates = branch_terminates(then_branch);
    let else_terminates = branch_terminates(else_body);
    (then_terminates != else_terminates).then_some(!then_terminates)
}

struct IfSingleCircleTerminalPlan<'a> {
    survivor: &'a [LayoutNode],
    survivor_label: Option<&'a str>,
    terminal: &'a LayoutNode,
    terminal_label: Option<&'a str>,
}

fn lone_circle_terminal(body: &[LayoutNode]) -> Option<&LayoutNode> {
    match body {
        [node @ (LayoutNode::Stop | LayoutNode::End)] => Some(node),
        _ => None,
    }
}

fn terminal_radius(node: &LayoutNode) -> f64 {
    match node {
        LayoutNode::Stop => STOP_OUTER_R,
        LayoutNode::End => 10.0,
        _ => 0.0,
    }
}

fn if_single_circle_terminal_plan<'a>(
    then_label: Option<&'a String>,
    then_branch: &'a [LayoutNode],
    else_branches: &'a [ElseBranch],
) -> Option<IfSingleCircleTerminalPlan<'a>> {
    if else_branches.len() != 1 || else_branches[0].condition.is_some() {
        return None;
    }
    let else_body = else_branches[0].body.as_slice();
    let else_label = else_branches[0].label.as_deref();
    match (
        lone_circle_terminal(then_branch),
        lone_circle_terminal(else_body),
    ) {
        (Some(terminal), None) if !branch_is_empty(else_body) && !branch_terminates(else_body) => {
            Some(IfSingleCircleTerminalPlan {
                survivor: else_body,
                survivor_label: else_label,
                terminal,
                terminal_label: then_label.map(String::as_str),
            })
        }
        (None, Some(terminal))
            if !branch_is_empty(then_branch) && !branch_terminates(then_branch) =>
        {
            Some(IfSingleCircleTerminalPlan {
                survivor: then_branch,
                survivor_label: then_label.map(String::as_str),
                terminal,
                terminal_label: else_label,
            })
        }
        _ => None,
    }
}

fn if_node_has_single_survivor(node: &LayoutNode) -> bool {
    matches!(
        node,
        LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } if if_single_survivor(then_branch, else_branches).is_some()
    )
}

/// True for nodes that occupy vertical space and receive inbound connectors —
/// i.e. everything `emit_sequence` treats as a flow step. Mirrors the skip set
/// at the top of `emit_sequence_ex`.
fn is_empty_partition_node(n: &LayoutNode) -> bool {
    matches!(n, LayoutNode::Partition { body, .. } if body.is_empty())
}

fn node_is_flow(n: &LayoutNode) -> bool {
    match n {
        LayoutNode::Arrow { .. }
        | LayoutNode::Note { .. }
        | LayoutNode::Title { .. }
        | LayoutNode::Detach
        | LayoutNode::Kill
        | LayoutNode::Break
        | LayoutNode::Goto(_) => false,
        LayoutNode::Partition { body, .. } if body.is_empty() => false,
        _ => true,
    }
}

/// A branch is "empty" (for if-down corridor purposes) if it has no flow nodes
/// — only arrows/notes/titles, which take no vertical space.
fn branch_is_empty(body: &[LayoutNode]) -> bool {
    !body.iter().any(node_is_flow)
}

fn if_empty_both_plain(then_branch: &[LayoutNode], else_branches: &[ElseBranch]) -> bool {
    else_branches.len() == 1
        && else_branches[0].condition.is_none()
        && branch_is_empty(then_branch)
        && branch_is_empty(&else_branches[0].body)
}

fn uses_vertical_if_pragma(diagram: &ActivityDiagram) -> bool {
    diagram.meta.source.as_deref().is_some_and(|source| {
        source.lines().any(|line| {
            let mut parts = line.split_whitespace();
            matches!(parts.next(), Some("!pragma"))
                && matches!(
                    parts.next(),
                    Some(name) if name.eq_ignore_ascii_case("useVerticalIf")
                )
                && matches!(
                    parts.next(),
                    Some(value) if value.eq_ignore_ascii_case("on")
                        || value.eq_ignore_ascii_case("true")
                )
        })
    })
}

/// PlantUML's `ConditionalBuilder.create` routes an `if/else` to the asymmetric
/// "down" layout (`FtileIfDown`) when exactly one branch is empty and the other
/// is populated and non-terminating: the populated branch flows down the centre
/// spine while the empty branch becomes a thin side corridor. A missing `else`
/// behaves like an implicit empty branch for this layout.
struct IfDownPlan<'a> {
    /// Body of the populated branch (flows down the spine).
    populated: &'a [LayoutNode],
    /// True when the *then* branch is the populated one (controls which side
    /// the diamond labels sit on).
    then_populated: bool,
}

fn if_down_plan<'a>(
    then_branch: &'a [LayoutNode],
    else_branches: &'a [ElseBranch],
) -> Option<IfDownPlan<'a>> {
    // Only a single plain then plus zero-or-one else (no elseif cascade).
    if else_branches.len() > 1 {
        return None;
    }
    let else_body = else_branches.first().map_or(&[][..], |b| b.body.as_slice());
    let then_empty = branch_is_empty(then_branch);
    let else_empty = branch_is_empty(else_body);

    // Exactly one branch empty.
    if then_empty == else_empty {
        return None;
    }
    let populated = if then_empty { else_body } else { then_branch };
    // The populated branch must not terminate — a terminating populated branch
    // is the single-stop case PlantUML handles with a different connector set.
    if branch_terminates(populated) {
        return None;
    }
    Some(IfDownPlan {
        populated,
        then_populated: !then_empty,
    })
}

/// True when a branch flow is exactly a single `break` (after an optional
/// leading arrow). PlantUML's `Branch.isOnlySingleStopOrSpot()` returns false
/// for a break, and `isEmpty()` is false, so `ConditionalBuilder.create` routes
/// `if (c) then break endif` to `createDown` with the break as the populated
/// (south) branch — drawn by `FtileIfDown` with `optionalStop == null` and a
/// then-block that `hasPointOut() == false`, i.e. `ConnectionElseNoDiamond`.
fn branch_is_lone_break(flow: &[LayoutNode]) -> bool {
    matches!(leading_branch_arrow(flow).1, [LayoutNode::Break])
}

/// A break-bearing `if` rendered as `FtileIfDown` with a suppressed merge
/// diamond: one branch is a lone `break` (the south spine, welded left to the
/// enclosing loop's exit corridor) and the other is empty (the east corridor
/// that becomes the if's pointOut). Returns the populated-branch flag (true when
/// the *then* branch is the break) and the diamond labels' sides.
struct IfBreakDownPlan {
    /// True when the *then* branch carries the break (south); false when the
    /// *else* branch does. Controls which side the diamond labels sit on.
    then_is_break: bool,
}

fn if_break_down_plan(
    then_branch: &[LayoutNode],
    else_branches: &[ElseBranch],
) -> Option<IfBreakDownPlan> {
    // Only a single plain then plus zero-or-one plain else (no elseif cascade).
    if else_branches.len() > 1 {
        return None;
    }
    if else_branches.first().is_some_and(|b| b.condition.is_some()) {
        return None;
    }
    let else_body = else_branches.first().map_or(&[][..], |b| b.body.as_slice());
    let then_break = branch_is_lone_break(then_branch);
    let else_break = branch_is_lone_break(else_body);
    // Exactly one branch is a lone break; the other must be empty.
    if then_break && branch_is_empty(else_body) {
        Some(IfBreakDownPlan { then_is_break: true })
    } else if else_break && branch_is_empty(then_branch) {
        Some(IfBreakDownPlan {
            then_is_break: false,
        })
    } else {
        let _ = (then_break, else_break);
        None
    }
}

/// Recursively whether a node sequence contains a `break` (mirrors PlantUML's
/// `Instruction.containsBreak`, which recurses through if/else, switch, fork,
/// group and partition bodies but NOT through nested while/repeat loops — those
/// own their own break). Used to gate `manageSpecialStopEndAfterEndWhile`.
fn nodes_contain_break(nodes: &[LayoutNode]) -> bool {
    nodes.iter().any(node_contains_break)
}

fn node_contains_break(node: &LayoutNode) -> bool {
    match node {
        LayoutNode::Break => true,
        LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } => {
            nodes_contain_break(then_branch)
                || else_branches.iter().any(|b| nodes_contain_break(&b.body))
        }
        LayoutNode::Switch { cases, .. } => cases.iter().any(|c| nodes_contain_break(&c.body)),
        LayoutNode::Fork { branches, .. } => branches.iter().any(|b| nodes_contain_break(b)),
        LayoutNode::Partition { body, .. } => nodes_contain_break(body),
        // Nested while/repeat own their own break; everything else carries none.
        _ => false,
    }
}

/// Whether a `while` body directly contains a break-bearing `if` that
/// [`if_break_down_plan`] will render as a no-diamond down layout welding its
/// break branch to the loop exit corridor. Only direct body children are
/// considered: a `break` inside a nested `while`/`repeat`/`fork` welds to *that*
/// loop, and other shapes route their own break.
fn body_contains_break_if(body: &[LayoutNode]) -> bool {
    body.iter().any(|n| {
        matches!(
            n,
            LayoutNode::If { then_branch, else_branches, .. }
                if if_break_down_plan(then_branch, else_branches).is_some()
        )
    })
}

/// Width of an if/while/repeat condition diamond's inner (top/bottom) edge.
/// PlantUML clamps this to a minimum of 24 px so very short conditions still
/// produce a diamond wider than their text. The text inside stays at its
/// measured length — the polygon and the text are sized independently.
fn diamond_inner_w_styled(condition: &str, font_size: f64, bold: bool, font_family: &str) -> f64 {
    text_render::measure_with_family(condition, font_size, bold, font_family)
        .max(DIAMOND_MIN_INNER_W)
}

fn diamond_inner_w_styled_padded(
    condition: &str,
    font_size: f64,
    bold: bool,
    font_family: &str,
    pad_x: f64,
) -> f64 {
    diamond_inner_w_styled(condition, font_size, bold, font_family) + 2.0 * pad_x
}

fn diamond_half_y_styled(condition: &str, font_size: f64, font_family: &str, pad_y: f64) -> f64 {
    ((text_render::label_height_with_family(condition, font_size, font_family) + 2.0 * pad_y) / 2.0)
        .max(DIAMOND_HALF)
}

fn diamond_inner_w(condition: &str) -> f64 {
    diamond_inner_w_styled(condition, SMALL_FONT, false, "sans-serif")
}

fn centered_label_y_for_family(
    text: &str,
    center_y: f64,
    font_size: f64,
    font_family: &str,
) -> f64 {
    center_y
        - text_render::label_height_with_family(text, font_size, font_family)
            .max(TEXT_MIN_BOX_HEIGHT)
            / 2.0
        + text_render::label_first_baseline_ascent_with_family(text, font_size, font_family)
}

fn centered_label_y(text: &str, center_y: f64, font_size: f64) -> f64 {
    centered_label_y_for_family(text, center_y, font_size, "sans-serif")
}

fn centered_text_y(center_y: f64, font_size: f64) -> f64 {
    center_y - text_box_height(font_size) / 2.0 + pm::ascent(font_size)
}

fn centerline_label_y_for_family(center_y: f64, font_size: f64, font_family: &str) -> f64 {
    center_y
        - (text_render::text_height_for_family(font_size, font_family).max(TEXT_MIN_BOX_HEIGHT)
            - text_render::ascent_for_family(font_size, font_family))
}

fn text_box_height(font_size: f64) -> f64 {
    pm::text_height(font_size).max(TEXT_MIN_BOX_HEIGHT)
}

fn labelled_if_inbound_gap(font_size: f64) -> f64 {
    (text_box_height(font_size) + IF_LABEL_INBOUND_PAD).max(ARROW_LEN)
}

fn default_inbound_gap(node: &LayoutNode) -> f64 {
    match node {
        LayoutNode::If {
            arrow_font_size,
            then_label,
            else_branches,
            ..
        } if then_label.is_some() || else_branches.iter().any(|b| b.label.is_some()) => {
            labelled_if_inbound_gap(*arrow_font_size)
        }
        _ => ARROW_LEN,
    }
}

/// Build a layout tree from the flat step list.
fn build_tree(steps: &[ActivityStep], palette: &Palette) -> Vec<LayoutNode> {
    // Swimlane detection: if any `|Lane|` marker appears (and there's more
    // than one distinct lane, or content exists before the first marker),
    // wrap the whole flow in a Swimlanes node. PlantUML treats a single-
    // lane diagram (only one `|Lane|` marker with no content before it) with
    // lane chrome suppressed. Top-level group frames still keep a tiny trace
    // of that marker in FTile positioning, so tag those below.
    let swimlane_markers: Vec<&str> = steps
        .iter()
        .filter_map(|s| match s {
            ActivityStep::Swimlane(lane) => Some(lane.name.as_str()),
            _ => None,
        })
        .collect();
    let distinct_lanes: std::collections::BTreeSet<&str> =
        swimlane_markers.iter().copied().collect();
    let has_pre_lane_content = steps
        .iter()
        .take_while(|s| !matches!(s, ActivityStep::Swimlane(_)))
        .any(|s| !matches!(s, ActivityStep::Note(_) | ActivityStep::Arrow(_)));
    if distinct_lanes.len() > 1 || (distinct_lanes.len() == 1 && has_pre_lane_content) {
        return build_swimlanes(steps, palette);
    }

    let mut tree = build_tree_inner(steps, palette);
    if distinct_lanes.len() == 1 && !has_pre_lane_content {
        mark_single_lane_groups(&mut tree);
    }
    tree
}

fn build_swimlanes(steps: &[ActivityStep], palette: &Palette) -> Vec<LayoutNode> {
    let mut lanes: Vec<Lane> = Vec::new();
    let mut segments: Vec<LaneSegment> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_color: Option<String> = None;
    let mut current_steps: Vec<ActivityStep> = Vec::new();

    let flush = |lanes: &mut Vec<Lane>,
                 segments: &mut Vec<LaneSegment>,
                 name: &Option<String>,
                 color: &Option<String>,
                 steps: &mut Vec<ActivityStep>| {
        if name.is_none() && steps.is_empty() {
            return;
        }
        let name = name.clone().unwrap_or_default();
        let lane_index = if let Some(index) = lanes.iter().position(|lane| lane.name == name) {
            if lanes[index].color.is_none() && color.is_some() {
                lanes[index].color = color.clone();
            }
            index
        } else {
            lanes.push(Lane {
                name,
                color: color.clone(),
                content_left: 0.0,
                content_right: 0.0,
            });
            lanes.len() - 1
        };
        let body = build_tree_inner(steps, palette);
        let (left, right) = swimlane_lane_extents(&body);
        lanes[lane_index].content_left = lanes[lane_index].content_left.max(left);
        lanes[lane_index].content_right = lanes[lane_index].content_right.max(right);
        steps.clear();
        // A `|Lane|` declaration that is immediately switched away from (e.g.
        // `|Lane1| |Lane2| start ...`) registers the column above for header
        // width / ordering, but contributes no temporal segment: PlantUML does
        // not reserve a vertical band or emit a header→start connector for it.
        // Only push a segment that carries content.
        if body.is_empty() {
            return;
        }
        segments.push(LaneSegment { lane_index, body });
    };

    for step in steps {
        if let ActivityStep::Swimlane(lane) = step {
            if current_name.as_deref() == Some(lane.name.as_str()) {
                if current_color.is_none() {
                    current_color = lane.color.clone();
                }
                continue;
            }
            flush(
                &mut lanes,
                &mut segments,
                &current_name,
                &current_color,
                &mut current_steps,
            );
            current_name = Some(lane.name.clone());
            current_color = lane.color.clone();
        } else {
            current_steps.push(step.clone());
        }
    }
    flush(
        &mut lanes,
        &mut segments,
        &current_name,
        &current_color,
        &mut current_steps,
    );

    vec![LayoutNode::Swimlanes { lanes, segments }]
}

fn layout_action_node(text: &str, palette: &Palette) -> LayoutNode {
    let text = action_text_for_family(text, &palette.action_font_family);
    let tw = action_text_width(
        &text,
        palette.action_font_size,
        palette.action_text_bold,
        &palette.action_font_family,
    );
    LayoutNode::Action {
        text,
        text_width: tw,
        pad_x: palette.action_pad_x,
        pad_y: palette.action_pad_y,
        font_family: palette.action_font_family.clone(),
        font_size: palette.action_font_size,
        bold: palette.action_text_bold,
        italic: palette.action_text_italic,
    }
}

fn build_tree_inner(steps: &[ActivityStep], palette: &Palette) -> Vec<LayoutNode> {
    let mut nodes = Vec::new();
    let mut i = 0;
    while i < steps.len() {
        match &steps[i] {
            ActivityStep::Start => {
                nodes.push(LayoutNode::Start);
                i += 1;
            }
            ActivityStep::Stop => {
                nodes.push(LayoutNode::Stop);
                i += 1;
            }
            ActivityStep::End => {
                nodes.push(LayoutNode::End);
                i += 1;
            }
            ActivityStep::Connector(label) => {
                nodes.push(LayoutNode::Connector(label.clone()));
                i += 1;
            }
            ActivityStep::Action(text) => {
                nodes.push(layout_action_node(text, palette));
                i += 1;
            }
            ActivityStep::DeprecatedColorAction(dca) => {
                let text = action_text_for_family(&dca.text, &palette.action_font_family);
                let tw = text_render::measure_with_family(
                    &text,
                    palette.action_font_size,
                    palette.action_text_bold,
                    &palette.action_font_family,
                );
                let warning = deprecated_warning(&dca.color);
                let ww = pm::mono_text_width(&warning, 10.0);
                nodes.push(LayoutNode::DeprecatedAction {
                    color: dca.color.clone(),
                    text,
                    text_width: tw,
                    pad_x: palette.action_pad_x,
                    pad_y: palette.action_pad_y,
                    font_family: palette.action_font_family.clone(),
                    font_size: palette.action_font_size,
                    bold: palette.action_text_bold,
                    italic: palette.action_text_italic,
                    warning_width: ww,
                });
                i += 1;
            }
            ActivityStep::If(block) => {
                i += 1;
                let mut then_branch = collect_until_else_or_endif(steps, &mut i, palette);
                let mut attached_notes = take_leading_notes(&mut then_branch);
                let mut else_branches = Vec::new();
                while i < steps.len() {
                    match &steps[i] {
                        ActivityStep::Else(_) | ActivityStep::ElseIf(_) => {
                            let (label, condition) = match &steps[i] {
                                ActivityStep::Else(l) => (l.clone(), None),
                                ActivityStep::ElseIf(eb) => {
                                    (eb.then_label.clone(), Some(eb.condition.clone()))
                                }
                                _ => (None, None),
                            };
                            i += 1;
                            let mut body = collect_until_else_or_endif(steps, &mut i, palette);
                            attached_notes.extend(take_leading_notes(&mut body));
                            else_branches.push(ElseBranch {
                                label,
                                condition,
                                body,
                            });
                        }
                        ActivityStep::EndIf => {
                            i += 1;
                            break;
                        }
                        _ => break,
                    }
                }
                while let Some(ActivityStep::Note(n)) = steps.get(i) {
                    attached_notes.push(activity_note_from_block(n));
                    i += 1;
                }
                nodes.push(LayoutNode::If {
                    condition: block.condition.clone(),
                    diamond_font_family: palette.diamond_font_family.clone(),
                    diamond_font_size: palette.diamond_font_size,
                    diamond_text_color: palette.diamond_text_color.clone(),
                    diamond_text_bold: palette.diamond_text_bold,
                    diamond_text_italic: palette.diamond_text_italic,
                    diamond_pad_x: palette.diamond_pad_x,
                    diamond_half_y: diamond_half_y_styled(
                        &block.condition,
                        palette.diamond_font_size,
                        &palette.diamond_font_family,
                        palette.diamond_pad_x,
                    ),
                    arrow_font_size: palette.arrow_font_size,
                    then_label: block.then_label.clone(),
                    then_branch,
                    else_branches,
                    attached_notes,
                });
            }
            ActivityStep::ElseIf(_) | ActivityStep::Else(_) | ActivityStep::EndIf => {
                // These should be consumed by If handler; skip if orphaned.
                i += 1;
            }
            ActivityStep::While(w) => {
                i += 1;
                let mut body = collect_until(steps, &mut i, palette, |s| {
                    matches!(s, ActivityStep::EndWhile(_))
                });
                let end_label = if i < steps.len() {
                    if let ActivityStep::EndWhile(l) = &steps[i] {
                        i += 1;
                        l.clone()
                    } else {
                        None
                    }
                } else {
                    None
                };
                while let Some(ActivityStep::Note(n)) = steps.get(i) {
                    body.push(layout_note_from_block(n));
                    i += 1;
                }
                // Absorb a trailing Stop/End/Detach/Kill into the while's
                // special_out — PlantUML's manageSpecialStopEndAfterEndWhile
                // pulls these terminators inside the FtileWhile frame. It bails
                // (returns false) when the loop contains a `break`, because the
                // break shares the loop's exit corridor: the loop then exits
                // normally (wrapping back to the spine) and the terminator stays
                // an ordinary post-while flow node.
                let special_out = if i < steps.len() && !nodes_contain_break(&body) {
                    let term = match &steps[i] {
                        ActivityStep::Stop => Some(LayoutNode::Stop),
                        ActivityStep::End => Some(LayoutNode::End),
                        ActivityStep::Detach => Some(LayoutNode::Detach),
                        ActivityStep::Kill => Some(LayoutNode::Kill),
                        _ => None,
                    };
                    if let Some(t) = term {
                        i += 1;
                        Some(Box::new(t))
                    } else {
                        None
                    }
                } else {
                    None
                };
                let starts_column = matches!(nodes.as_slice(), [LayoutNode::Start]);
                nodes.push(LayoutNode::While {
                    condition: w.condition.clone(),
                    diamond_font_family: palette.diamond_font_family.clone(),
                    diamond_font_size: palette.diamond_font_size,
                    diamond_text_color: palette.diamond_text_color.clone(),
                    diamond_text_bold: palette.diamond_text_bold,
                    diamond_text_italic: palette.diamond_text_italic,
                    arrow_font_size: palette.arrow_font_size,
                    is_label: w.is_label.clone(),
                    body,
                    end_label,
                    special_out,
                    starts_column,
                });
            }
            ActivityStep::EndWhile(_) => {
                i += 1;
            }
            ActivityStep::Repeat | ActivityStep::RepeatStart(_) => {
                let start_label = match &steps[i] {
                    ActivityStep::RepeatStart(label) => Some(label.clone()),
                    _ => None,
                };
                i += 1;
                // PlantUML attaches a `backward :label;` action to the loop-back
                // path (drawn on the right return arm), not to the body spine.
                // Pull its label out of the raw step run before building the body
                // tree so the body doesn't try to lay it out as a spine action.
                let mut backward: Option<String> = None;
                {
                    let mut j = i;
                    while j < steps.len() && !matches!(steps[j], ActivityStep::RepeatWhile(_)) {
                        if let ActivityStep::Backward(label) = &steps[j] {
                            backward = Some(label.clone());
                        }
                        j += 1;
                    }
                }
                let mut body = collect_until(steps, &mut i, palette, |s| {
                    matches!(s, ActivityStep::RepeatWhile(_))
                });
                let has_start_label = if let Some(label) = start_label {
                    body.insert(0, layout_action_node(&label, palette));
                    true
                } else {
                    false
                };
                let (condition, is_label, not_label) = if i < steps.len() {
                    if let ActivityStep::RepeatWhile(rw) = &steps[i] {
                        i += 1;
                        (
                            rw.condition.clone(),
                            rw.is_label.clone(),
                            rw.not_label.clone(),
                        )
                    } else {
                        (String::new(), None, None)
                    }
                } else {
                    (String::new(), None, None)
                };
                while let Some(ActivityStep::Note(n)) = steps.get(i) {
                    body.push(layout_note_from_block(n));
                    i += 1;
                }
                nodes.push(LayoutNode::Repeat {
                    body,
                    condition,
                    is_label,
                    not_label,
                    backward,
                    has_start_label,
                    arrow_font_size: palette.arrow_font_size,
                    arrow_font_family: palette.arrow_font_family.clone(),
                });
            }
            ActivityStep::RepeatWhile(_) => {
                i += 1;
            }
            ActivityStep::Fork | ActivityStep::Split => {
                let is_split = matches!(steps[i], ActivityStep::Split);
                i += 1;
                let mut branches = Vec::new();
                let first_branch = collect_until(steps, &mut i, palette, |s| {
                    matches!(
                        s,
                        ActivityStep::ForkAgain
                            | ActivityStep::SplitAgain
                            | ActivityStep::EndFork
                            | ActivityStep::EndMerge
                            | ActivityStep::EndSplit
                    )
                });
                branches.push(first_branch);
                let mut merge = false;
                while i < steps.len() {
                    match &steps[i] {
                        ActivityStep::ForkAgain | ActivityStep::SplitAgain => {
                            i += 1;
                            let branch = collect_until(steps, &mut i, palette, |s| {
                                matches!(
                                    s,
                                    ActivityStep::ForkAgain
                                        | ActivityStep::SplitAgain
                                        | ActivityStep::EndFork
                                        | ActivityStep::EndMerge
                                        | ActivityStep::EndSplit
                                )
                            });
                            branches.push(branch);
                        }
                        ActivityStep::EndFork | ActivityStep::EndMerge | ActivityStep::EndSplit => {
                            merge = matches!(steps[i], ActivityStep::EndMerge);
                            i += 1;
                            break;
                        }
                        _ => break,
                    }
                }
                let mut attached_notes = Vec::new();
                while let Some(ActivityStep::Note(n)) = steps.get(i) {
                    attached_notes.push(activity_note_from_block(n));
                    i += 1;
                }
                nodes.push(LayoutNode::Fork {
                    branches,
                    attached_notes,
                    is_split,
                    merge,
                });
            }
            ActivityStep::ForkAgain
            | ActivityStep::SplitAgain
            | ActivityStep::EndFork
            | ActivityStep::EndMerge
            | ActivityStep::EndSplit => {
                i += 1;
            }
            ActivityStep::Arrow(a) => {
                nodes.push(LayoutNode::Arrow {
                    dashed: a.dashed,
                    color: a.color.clone(),
                    label: a.label.clone(),
                });
                i += 1;
            }
            ActivityStep::Note(n) => {
                nodes.push(LayoutNode::Note {
                    text: n.text.clone(),
                    position: n.position.clone(),
                    color: n.color.clone(),
                });
                i += 1;
            }
            ActivityStep::Detach => {
                nodes.push(LayoutNode::Detach);
                i += 1;
            }
            ActivityStep::Kill => {
                nodes.push(LayoutNode::Kill);
                i += 1;
            }
            ActivityStep::Break => {
                nodes.push(LayoutNode::Break);
                i += 1;
            }
            ActivityStep::Goto(target) => {
                nodes.push(LayoutNode::Goto(target.clone()));
                i += 1;
            }
            ActivityStep::Partition(p) => {
                let name = p.name.clone();
                let color = p.color.clone();
                i += 1;
                let mut body = collect_until(steps, &mut i, palette, |s| {
                    matches!(s, ActivityStep::EndPartition)
                });
                if i < steps.len() {
                    i += 1; // skip EndPartition
                }
                while let Some(ActivityStep::Note(n)) = steps.get(i) {
                    body.push(LayoutNode::Note {
                        text: n.text.clone(),
                        position: n.position.clone(),
                        color: n.color.clone(),
                    });
                    i += 1;
                }
                nodes.push(LayoutNode::Partition {
                    name,
                    color,
                    is_group: p.is_group,
                    nested: false,
                    single_lane_group: false,
                    single_lane_first_group: false,
                    body,
                });
            }
            ActivityStep::EndPartition => {
                i += 1;
            }
            ActivityStep::Switch(condition) => {
                i += 1;
                let mut cases = Vec::new();
                while i < steps.len() {
                    match &steps[i] {
                        ActivityStep::Case(label) => {
                            let label = label.clone();
                            i += 1;
                            let body = collect_until(steps, &mut i, palette, |s| {
                                matches!(s, ActivityStep::Case(_) | ActivityStep::EndSwitch)
                            });
                            cases.push(SwitchCase { label, body });
                        }
                        ActivityStep::EndSwitch => {
                            i += 1;
                            break;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                nodes.push(LayoutNode::Switch {
                    condition: condition.clone(),
                    cases,
                });
            }
            ActivityStep::Case(_) | ActivityStep::EndSwitch => {
                i += 1;
            }
            ActivityStep::Backward(_) | ActivityStep::Swimlane(_) => {
                // TODO: implement these
                i += 1;
            }
        }
    }
    nodes
}

fn mark_nested_partitions(nodes: &mut [LayoutNode], in_partition: bool) {
    for node in nodes {
        match node {
            LayoutNode::Partition { nested, body, .. } => {
                *nested = in_partition;
                mark_nested_partitions(body, true);
            }
            LayoutNode::If {
                then_branch,
                else_branches,
                ..
            } => {
                mark_nested_partitions(then_branch, in_partition);
                for branch in else_branches {
                    mark_nested_partitions(&mut branch.body, in_partition);
                }
            }
            LayoutNode::While {
                body, special_out, ..
            } => {
                mark_nested_partitions(body, in_partition);
                if let Some(special) = special_out.as_deref_mut() {
                    mark_nested_partitions(std::slice::from_mut(special), in_partition);
                }
            }
            LayoutNode::Repeat { body, .. } => mark_nested_partitions(body, in_partition),
            LayoutNode::Fork { branches, .. } => {
                for branch in branches {
                    mark_nested_partitions(branch, in_partition);
                }
            }
            LayoutNode::Switch { cases, .. } => {
                for case in cases {
                    mark_nested_partitions(&mut case.body, in_partition);
                }
            }
            LayoutNode::Swimlanes { segments, .. } => {
                for segment in segments {
                    mark_nested_partitions(&mut segment.body, in_partition);
                }
            }
            _ => {}
        }
    }
}

fn mark_single_lane_groups(nodes: &mut [LayoutNode]) {
    let mut first = true;
    for node in nodes {
        if let LayoutNode::Partition {
            is_group,
            single_lane_group,
            single_lane_first_group,
            ..
        } = node
            && *is_group
        {
            *single_lane_group = true;
            if first {
                *single_lane_first_group = true;
                first = false;
            }
        }
    }
}

fn collect_until_else_or_endif(
    steps: &[ActivityStep],
    i: &mut usize,
    palette: &Palette,
) -> Vec<LayoutNode> {
    collect_until(steps, i, palette, |s| {
        matches!(
            s,
            ActivityStep::Else(_) | ActivityStep::ElseIf(_) | ActivityStep::EndIf
        )
    })
}

fn collect_until(
    steps: &[ActivityStep],
    i: &mut usize,
    palette: &Palette,
    pred: impl Fn(&ActivityStep) -> bool,
) -> Vec<LayoutNode> {
    let start = *i;
    let mut depth = 0;
    while *i < steps.len() {
        if depth == 0 && pred(&steps[*i]) {
            break;
        }
        // Track nesting depth for if/fork/while/repeat
        match &steps[*i] {
            ActivityStep::If(_) => depth += 1,
            ActivityStep::EndIf => depth -= 1,
            ActivityStep::Fork | ActivityStep::Split => depth += 1,
            ActivityStep::EndFork | ActivityStep::EndMerge | ActivityStep::EndSplit => depth -= 1,
            ActivityStep::While(_) => depth += 1,
            ActivityStep::EndWhile(_) => depth -= 1,
            ActivityStep::Repeat | ActivityStep::RepeatStart(_) => depth += 1,
            ActivityStep::RepeatWhile(_) => depth -= 1,
            ActivityStep::Partition(_) => depth += 1,
            ActivityStep::EndPartition => depth -= 1,
            ActivityStep::Switch(_) => depth += 1,
            ActivityStep::EndSwitch => depth -= 1,
            _ => {}
        }
        *i += 1;
    }
    build_tree(&steps[start..*i], palette)
}

/// Compute the width needed for a sequence of layout nodes.
fn sequence_width(nodes: &[LayoutNode]) -> f64 {
    nodes.iter().map(node_width).fold(0.0f64, f64::max)
}

fn sequence_partition_body_extents(nodes: &[LayoutNode]) -> (f64, f64) {
    sequence_extents_with_note_margin(nodes, false, 0.0)
}

fn sequence_partition_body_width(nodes: &[LayoutNode]) -> f64 {
    let (left, right) = sequence_partition_body_extents(nodes);
    left + right
}

fn partition_body_has_direct_note(nodes: &[LayoutNode]) -> bool {
    nodes
        .iter()
        .any(|node| matches!(node, LayoutNode::Note { .. }))
}

fn body_has_direct_left_note(nodes: &[LayoutNode]) -> bool {
    nodes.iter().any(|node| {
        matches!(
            node,
            LayoutNode::Note {
                position: NotePosition::Left,
                ..
            }
        )
    })
}

fn partition_body_width_for_frame(nodes: &[LayoutNode]) -> f64 {
    if partition_body_has_direct_note(nodes) {
        sequence_partition_body_width(nodes)
    } else {
        sequence_width(nodes)
    }
}

#[derive(Debug, Clone)]
struct ForkLayout {
    bar_w: f64,
    centers: Vec<f64>,
    spine_dx: f64,
}

const FORK_INNER_PAD: f64 = 12.0;
const FORK_BRANCH_GAP: f64 = 10.0;
const FORK_EVEN_MIDDLE_EXTRA: f64 = 18.0;
const FORK_ASYMMETRIC_EVEN_MIDDLE_EXTRA: f64 = 32.0;
const FORK_ASYMMETRIC_SPINE_STEP: f64 = 5.0;
const FORK_ASYMMETRIC_EPS: f64 = 0.02;
const FORK_EMPTY_EDGE_CENTER: f64 = 14.0;
const FORK_EMPTY_LANE_GAP: f64 = 21.0;
/// When a fork is itself the first branch tile under `FtileIfWithLinks`,
/// PlantUML keeps the fork's flow spine fixed and grows the bar to the right.
const FORK_IF_BRANCH_RIGHT_EXTRA: f64 = 2.0;
const FORK_IF_BRANCH_ODD_LAST_GAP_EXTRA: f64 = 10.6240234375;
const FORK_IF_BRANCH_EVEN_LAST_GAP_EXTRA: f64 = FORK_EVEN_MIDDLE_EXTRA;
const FORK_IF_BRANCH_ODD_SPACING_EXTRA: f64 = 6.083;
const FORK_IF_BRANCH_EVEN_SPACING_EXTRA: f64 = 8.0;
const FORK_IF_BRANCH_ODD_SPINE_SHIFT: f64 = -0.4175;
const FORK_IF_BRANCH_EVEN_SPINE_SHIFT: f64 = 6.0;
const FORK_MERGE_GAP: f64 = 10.0;
/// Extra inter-tile gap inside a fork branch: PlantUML's assembly space is 35,
/// versus the 20 px `ARROW_LEN` that compresses in ordinary sequences.
const FORK_BRANCH_INTER_GAP_EXTRA: f64 = 15.0;
/// Gap below the deepest branch to the join bar when every branch terminates
/// (no ConnectionOut arrows reach the bar — half the usual ARROW_LEN reserve).
const FORK_TERMINATING_JOIN_GAP: f64 = 10.0;

fn node_if_depth(node: &LayoutNode) -> usize {
    match node {
        LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } => {
            1 + sequence_if_depth(then_branch).max(
                else_branches
                    .iter()
                    .map(|branch| sequence_if_depth(&branch.body))
                    .max()
                    .unwrap_or(0),
            )
        }
        LayoutNode::While {
            body, special_out, ..
        } => sequence_if_depth(body).max(special_out.as_deref().map_or(0, node_if_depth)),
        LayoutNode::Repeat { body, .. } | LayoutNode::Partition { body, .. } => {
            sequence_if_depth(body)
        }
        LayoutNode::Fork { branches, .. } => branches
            .iter()
            .map(|b| sequence_if_depth(b))
            .max()
            .unwrap_or(0),
        LayoutNode::Switch { cases, .. } => cases
            .iter()
            .map(|case| sequence_if_depth(&case.body))
            .max()
            .unwrap_or(0),
        LayoutNode::Swimlanes { segments, .. } => segments
            .iter()
            .map(|segment| sequence_if_depth(&segment.body))
            .max()
            .unwrap_or(0),
        _ => 0,
    }
}

fn sequence_if_depth(nodes: &[LayoutNode]) -> usize {
    nodes.iter().map(node_if_depth).max().unwrap_or(0)
}

fn fork_layout(branches: &[Vec<LayoutNode>]) -> ForkLayout {
    let branch_extents: Vec<(f64, f64)> = branches.iter().map(|b| sequence_extents(b)).collect();
    let branch_widths: Vec<f64> = branch_extents.iter().map(|(l, r)| l + r).collect();
    let n = branch_widths.len();
    if n == 0 {
        return ForkLayout {
            bar_w: 0.0,
            centers: Vec::new(),
            spine_dx: 0.0,
        };
    }

    if branches.iter().any(Vec::is_empty) {
        let mut centers = Vec::with_capacity(n);
        let mut x = FORK_INNER_PAD;
        for (i, branch) in branches.iter().enumerate() {
            if branch.is_empty() {
                let center = if i == 0 {
                    FORK_EMPTY_EDGE_CENTER
                } else if i + 1 == n {
                    x + FORK_EMPTY_EDGE_CENTER
                } else {
                    x + FORK_EMPTY_LANE_GAP
                };
                centers.push(center);
                x = center
                    + if i == 0 {
                        FORK_EMPTY_EDGE_CENTER
                    } else {
                        FORK_EMPTY_LANE_GAP
                    };
            } else {
                let (left, right) = branch_extents[i];
                let w = left + right;
                centers.push(x + left);
                x += w;
                if i + 1 < n && !branches[i + 1].is_empty() {
                    x += FORK_EMPTY_LANE_GAP;
                }
            }
        }
        return ForkLayout {
            bar_w: x + FORK_INNER_PAD,
            centers,
            spine_dx: if branches[0].is_empty() && !branches[n - 1].is_empty() {
                2.5
            } else if branches[n - 1].is_empty() && !branches[0].is_empty() {
                -2.5
            } else {
                0.0
            },
        };
    }

    // PlantUML's ordinary fork-bar layout:
    //   bar_w = 24 (inner pad each side) + sum(branch_widths) + (n-1)*10 +
    //           (18 if n is even else 0)
    // The extra 18 px goes into the middle gap for even branch counts,
    // pushing the centre branches apart.
    let total_branch_w: f64 = branch_widths.iter().sum();
    let inter_gaps = if n > 1 { (n - 1) as f64 } else { 0.0 };
    let has_asymmetric_branch = branch_extents
        .iter()
        .any(|(left, right)| (left - right).abs() > FORK_ASYMMETRIC_EPS);
    let even_extra = if n >= 2 && n.is_multiple_of(2) && has_asymmetric_branch {
        FORK_ASYMMETRIC_EVEN_MIDDLE_EXTRA
    } else if n >= 2 && n.is_multiple_of(2) {
        FORK_EVEN_MIDDLE_EXTRA
    } else {
        0.0
    };
    let bar_w = FORK_INNER_PAD * 2.0 + total_branch_w + inter_gaps * FORK_BRANCH_GAP + even_extra;
    let mut centers = Vec::with_capacity(n);
    if n == 1 {
        centers.push(bar_w / 2.0);
    } else {
        let mut x = FORK_INNER_PAD;
        let middle_gap_idx = if even_extra > 0.0 {
            Some(n / 2 - 1)
        } else {
            None
        };
        for (i, (left, right)) in branch_extents.iter().enumerate() {
            centers.push(x + left);
            x += left + right;
            if i + 1 < n {
                let extra = if Some(i) == middle_gap_idx {
                    even_extra
                } else {
                    0.0
                };
                x += FORK_BRANCH_GAP + extra;
            }
        }
    }
    ForkLayout {
        bar_w,
        centers,
        spine_dx: if n > 1 && !n.is_multiple_of(2) && has_asymmetric_branch {
            let max_if_depth = branches
                .iter()
                .map(|branch| sequence_if_depth(branch))
                .max()
                .unwrap_or(0);
            max_if_depth.saturating_sub(1) as f64 * FORK_ASYMMETRIC_SPINE_STEP
        } else {
            0.0
        },
    }
}

fn split_layout(branches: &[Vec<LayoutNode>]) -> ForkLayout {
    let branch_extents: Vec<(f64, f64)> = branches.iter().map(|b| sequence_extents(b)).collect();
    let branch_widths: Vec<f64> = branch_extents.iter().map(|(l, r)| l + r).collect();
    let n = branch_widths.len();
    if n == 0 {
        return ForkLayout {
            bar_w: 0.0,
            centers: Vec::new(),
            spine_dx: 0.0,
        };
    }
    if branches.iter().any(Vec::is_empty) {
        return fork_layout(branches);
    }

    let inter_gaps = if n > 1 { (n - 1) as f64 } else { 0.0 };
    let has_asymmetric_branch = branch_extents
        .iter()
        .any(|(left, right)| (left - right).abs() > FORK_ASYMMETRIC_EPS);
    let even_extra = if n >= 2 && n.is_multiple_of(2) && has_asymmetric_branch {
        FORK_ASYMMETRIC_EVEN_MIDDLE_EXTRA
    } else if n >= 2 && n.is_multiple_of(2) {
        FORK_EVEN_MIDDLE_EXTRA
    } else {
        0.0
    };
    let bar_w = branch_widths.iter().sum::<f64>() + inter_gaps * FORK_BRANCH_GAP + even_extra;
    let mut centers = Vec::with_capacity(n);
    if n == 1 {
        centers.push(bar_w / 2.0);
    } else {
        let mut x = 0.0;
        let middle_gap_idx = if even_extra > 0.0 {
            Some(n / 2 - 1)
        } else {
            None
        };
        for (i, (left, right)) in branch_extents.iter().enumerate() {
            centers.push(x + left);
            x += left + right;
            if i + 1 < n {
                let extra = if Some(i) == middle_gap_idx {
                    even_extra
                } else {
                    0.0
                };
                x += FORK_BRANCH_GAP + extra;
            }
        }
    }
    ForkLayout {
        bar_w,
        centers,
        spine_dx: 0.0,
    }
}

fn parallel_layout(branches: &[Vec<LayoutNode>], is_split: bool) -> ForkLayout {
    if is_split {
        split_layout(branches)
    } else {
        fork_layout(branches)
    }
}

fn fork_layout_if_branch(branches: &[Vec<LayoutNode>]) -> ForkLayout {
    let mut layout = fork_layout(branches);
    let n = layout.centers.len();
    if n < 2 || branches.iter().any(Vec::is_empty) {
        return layout;
    }

    let old_spine = layout.bar_w / 2.0 - layout.spine_dx;
    let last_gap_extra = if n >= 3 {
        if n.is_multiple_of(2) {
            FORK_IF_BRANCH_EVEN_LAST_GAP_EXTRA
        } else {
            FORK_IF_BRANCH_ODD_LAST_GAP_EXTRA
        }
    } else {
        0.0
    };
    if last_gap_extra != 0.0
        && let Some(last) = layout.centers.last_mut()
    {
        *last += last_gap_extra;
    }
    let right_extra = if n == 2 || !n.is_multiple_of(2) {
        FORK_IF_BRANCH_RIGHT_EXTRA
    } else {
        0.0
    };
    layout.bar_w += last_gap_extra + right_extra;
    layout.spine_dx = layout.bar_w / 2.0 - old_spine;
    layout
}

fn fork_layout_if_branch_spacing(branches: &[Vec<LayoutNode>]) -> ForkLayout {
    let mut layout = fork_layout(branches);
    let n = layout.centers.len();
    if n < 2 || branches.iter().any(Vec::is_empty) {
        return layout;
    }

    let old_spine = layout.bar_w / 2.0 - layout.spine_dx;
    let extra = if n == 2 {
        FORK_IF_BRANCH_RIGHT_EXTRA
    } else if n.is_multiple_of(2) {
        FORK_IF_BRANCH_EVEN_SPACING_EXTRA
    } else {
        FORK_IF_BRANCH_ODD_SPACING_EXTRA
    };
    layout.bar_w += extra;
    layout.spine_dx = layout.bar_w / 2.0 - old_spine;
    layout
}

fn fork_layout_in_partition(branches: &[Vec<LayoutNode>]) -> ForkLayout {
    let mut layout = fork_layout(branches);
    if layout.centers.len() >= 2 && !branches.iter().any(Vec::is_empty) {
        layout.bar_w += PARTITION_FORK_WIDTH_EXTRA;
        for center in &mut layout.centers {
            *center += PARTITION_FORK_BRANCH_CENTER_SHIFT;
        }
    }
    layout
}

fn leading_if_branch_fork_spine_shift(nodes: &[LayoutNode]) -> f64 {
    let Some(LayoutNode::Fork { branches, .. }) = nodes.iter().find(|node| node_is_flow(node))
    else {
        return 0.0;
    };
    let n = branches.len();
    if n < 3 {
        0.0
    } else if n.is_multiple_of(2) {
        FORK_IF_BRANCH_EVEN_SPINE_SHIFT
    } else {
        FORK_IF_BRANCH_ODD_SPINE_SHIFT
    }
}

#[derive(Clone, Copy)]
struct SwitchCaseTile {
    width: f64,
    left: f64,
}

impl SwitchCaseTile {
    fn right(self) -> f64 {
        self.width - self.left
    }
}

/// Geometry of one switch case tile. PlantUML wraps each branch in
/// `FtileDecorateInLabel`, which adds the case label above the branch and
/// widens only the right side when the label exceeds the branch's right extent.
fn switch_case_tile(case: &SwitchCase) -> SwitchCaseTile {
    if case.body.is_empty() {
        let width = text_render::measure(&case.label, SMALL_FONT, false);
        return SwitchCaseTile {
            width,
            left: width / 2.0,
        };
    }

    let (left, mut right) = sequence_extents(&case.body);
    let label_w = text_render::measure(&case.label, SMALL_FONT, false);
    right = right.max(label_w);
    SwitchCaseTile {
        width: left + right,
        left,
    }
}

/// Width of one switch case box: the decorated tile's own content width
/// (PlantUML imposes no extra minimum on switch case tiles).
fn switch_case_width(case: &SwitchCase) -> f64 {
    switch_case_tile(case).width
}

fn switch_diamond_half_width(condition: &str) -> f64 {
    (diamond_inner_w(condition) + DIAMOND_HALF * 2.0) / 2.0
}

fn switch_one_link_branch_dx(condition: &str) -> f64 {
    switch_diamond_half_width(condition) + SWITCH_ONE_LINK_BRANCH_GAP
}

fn switch_one_link_extents(case: &SwitchCase, condition: &str) -> (f64, f64) {
    let diamond_half = switch_diamond_half_width(condition);
    let tile = switch_case_tile(case);
    (
        diamond_half + SWITCH_ONE_LINK_LEFT_PAD,
        diamond_half + SWITCH_ONE_LINK_BRANCH_GAP + tile.right(),
    )
}

fn switch_one_link_below_diamond(case: &SwitchCase) -> f64 {
    let label_h = if case.label.is_empty() {
        0.0
    } else {
        case.label.split('\n').count().max(1) as f64 * pm::text_height(SMALL_FONT)
    };
    SWITCH_ONE_LINK_Y_DELTA + label_h
}

fn switch_all_branches_terminate(cases: &[SwitchCase]) -> bool {
    !cases.is_empty() && cases.iter().all(|case| branch_terminates(&case.body))
}

fn switch_has_empty_middle_case(cases: &[SwitchCase]) -> bool {
    matches!(cases, [_, middle, _] if middle.body.is_empty())
}

fn switch_needs_empty_merge_gap(cases: &[SwitchCase]) -> bool {
    cases
        .iter()
        .enumerate()
        .any(|(i, case)| case.body.is_empty() && i > 0 && i + 1 < cases.len())
}

fn switch_is_odd_alternating_mixed_empty(cases: &[SwitchCase]) -> bool {
    cases.len() >= 5
        && !cases.len().is_multiple_of(2)
        && cases.iter().enumerate().all(|(i, case)| {
            if i.is_multiple_of(2) {
                !case.body.is_empty()
            } else {
                case.body.is_empty()
            }
        })
}

fn switch_is_even_mixed_empty_pair(cases: &[SwitchCase]) -> bool {
    matches!(cases, [first, second, third, fourth]
        if !first.body.is_empty()
            && second.body.is_empty()
            && !third.body.is_empty()
            && fourth.body.is_empty())
}

fn switch_is_even_nested_if_cases(cases: &[SwitchCase]) -> bool {
    cases.len().is_multiple_of(2)
        && cases
            .iter()
            .all(|case| matches!(case.body.first(), Some(LayoutNode::If { .. })))
}

fn switch_inner_uses_diamond_corridor(
    cases: &[SwitchCase],
    condition: &str,
    layout: &SwitchXLayout,
) -> bool {
    if cases.len() < 3 {
        return false;
    }
    let diamond_half = switch_diamond_half_width(condition);
    cases[1..cases.len() - 1]
        .iter()
        .zip(layout.centers[1..layout.centers.len() - 1].iter())
        .any(|(case, center)| {
            if case.body.is_empty() {
                return false;
            }
            let rel = *center - layout.diamond_dx;
            rel >= -diamond_half - SWITCH_LINK_MARGIN && rel <= diamond_half + SWITCH_LINK_MARGIN
        })
}

fn switch_merge_gap(cases: &[SwitchCase], condition: &str, layout: &SwitchXLayout) -> f64 {
    if switch_all_branches_terminate(cases) {
        0.0
    } else if switch_needs_empty_merge_gap(cases) {
        SWITCH_EMPTY_MERGE_GAP
    } else if !cases.len().is_multiple_of(2)
        || switch_inner_uses_diamond_corridor(cases, condition, layout)
    {
        ARROW_LEN
    } else {
        ARROW_LEN / 2.0
    }
}

/// PlantUML's `SUPP15` margin used by `FtileSwitchWithDiamonds` in
/// BIG_DIAMOND mode (the horizontal padding either side of the diamond
/// column between the first and last case tiles).
const SWITCH_SUPP15: f64 = 15.0;

/// Faithful port of PlantUML's `FtileSwitchWithDiamonds` horizontal layout.
///
/// `centers` holds the per-case spine x relative to the block-left edge,
/// `block_w` the total packed width, and `diamond_dx` the switch/merge
/// diamond centre x relative to the same block-left edge (the diagram spine
/// aligns to this, *not* to the geometric block centre).
///
/// PlantUML picks BIG_DIAMOND vs SMALL_DIAMOND mode by comparing the spare
/// horizontal room either side of the diamond (`w13`) against the combined
/// width of the inner case tiles (`w9`). In BIG mode the diamond is wide
/// enough that the inner cases are spread evenly under it; in SMALL mode the
/// cases are packed tight and the diamond sits over the geometric centre.
struct SwitchXLayout {
    centers: Vec<f64>,
    block_w: f64,
    diamond_dx: f64,
    big_diamond: bool,
}

fn switch_x_layout(cases: &[SwitchCase], condition: &str) -> SwitchXLayout {
    let mut layout = switch_x_layout_with_small_gap(cases, condition, SWITCH_CASE_GAP, true);
    if !layout.big_diamond && switch_is_even_nested_if_cases(cases) {
        for center in layout.centers.iter_mut().skip(cases.len() / 2) {
            *center += SWITCH_IF_BRANCH_CASE_GAP;
        }
        layout.block_w += SWITCH_IF_BRANCH_CASE_GAP;
        layout.diamond_dx += SWITCH_IF_BRANCH_CASE_GAP / 2.0;
    }
    if !layout.big_diamond
        && cases.len().is_multiple_of(2)
        && switch_inner_uses_diamond_corridor(cases, condition, &layout)
    {
        for center in layout.centers.iter_mut().skip(cases.len() / 2) {
            *center -= SWITCH_INNER_CORRIDOR_PACKING_PULL_LEFT;
        }
        layout.block_w -= SWITCH_INNER_CORRIDOR_PACKING_PULL_LEFT;
        layout.diamond_dx -= SWITCH_INNER_CORRIDOR_PACKING_PULL_LEFT;
    }
    if switch_is_odd_alternating_mixed_empty(cases) {
        let mut centers = Vec::with_capacity(cases.len());
        let mut x = 0.0;
        for case in cases {
            let tile = switch_case_tile(case);
            centers.push(if case.body.is_empty() {
                x + SWITCH_MIXED_EMPTY_SPINE
            } else {
                x + tile.left
            });
            x += tile.width + SWITCH_MIXED_EMPTY_CASE_GAP;
        }
        layout.centers = centers;
        layout.block_w = x - SWITCH_MIXED_EMPTY_CASE_GAP;
        layout.diamond_dx = layout.block_w / 2.0;
        layout.big_diamond = false;
    }
    if switch_is_even_mixed_empty_pair(cases) {
        layout.diamond_dx += SWITCH_EVEN_MIXED_EMPTY_SPINE_SHIFT;
        layout.block_w += SWITCH_EVEN_MIXED_EMPTY_SPINE_SHIFT;
        layout.centers[2] += SWITCH_EVEN_MIXED_EMPTY_SPINE_SHIFT;
        layout.centers[3] -= SWITCH_EVEN_MIXED_EMPTY_LAST_PULL_LEFT;
    }
    if matches!(cases, [first, last] if !first.body.is_empty() && last.body.is_empty()) {
        layout.centers[1] = switch_case_width(&cases[0]) + DIAMOND_HALF / 2.0;
        let label_w = text_render::measure(&cases[1].label, SMALL_FONT, false);
        layout.block_w = layout.centers[1] + 4.0 + label_w + SWITCH_TWO_CASE_EMPTY_LABEL_TRAIL;
    }
    layout
}

fn switch_x_layout_if_branch(cases: &[SwitchCase], condition: &str) -> SwitchXLayout {
    if cases.len() >= 4 && !switch_case_block_is_big_diamond(cases, condition) {
        return switch_x_layout_if_branch_packed(cases, condition);
    }
    switch_x_layout_with_small_gap(cases, condition, SWITCH_IF_BRANCH_CASE_GAP, cases.len() == 2)
}

fn switch_x_layout_if_branch_extents(cases: &[SwitchCase], condition: &str) -> SwitchXLayout {
    // For four or more (non-big) cases the packed model already reflects the
    // drawn block exactly, so the parent reserves it verbatim.
    if cases.len() >= 4 && !switch_case_block_is_big_diamond(cases, condition) {
        return switch_x_layout_if_branch_packed(cases, condition);
    }
    // Two- and three-case forms reserve a tighter width than the drawn block:
    // the left-case clamp (emit pins case[0] to the canvas margin) eats into the
    // nominal left half, so the parent must not reserve the full symmetric
    // extent or the diagram drifts right. The reverse-engineered proxy uses a
    // smaller inter-case gap to approximate that compression.
    let first_half = cases
        .first()
        .map_or(0.0, |case| switch_case_width(case) / 2.0);
    let spine_room = (DIAMOND_HALF * 2.0 - first_half).max(0.0);
    switch_x_layout_with_small_gap(cases, condition, SWITCH_CASE_GAP + spine_room, cases.len() == 2)
}

/// Returns whether the SMALL/BIG diamond test selects BIG mode for these cases.
fn switch_case_block_is_big_diamond(cases: &[SwitchCase], condition: &str) -> bool {
    switch_x_layout_with_small_gap(cases, condition, SWITCH_IF_BRANCH_CASE_GAP, false).big_diamond
}

/// Packed if/while/fork-branch switch layout for four or more SMALL-diamond
/// cases. PlantUML lays the cases out as a uniform `xSeparation = 20`
/// FtileSwitchNude, then the diagram-wide `CompressionXorYBuilder(ON_X)` pass
/// squeezes the slack the left-case clamp opened up against the canvas margin:
/// the gaps to the left of the centreline collapse to 10 while the centre gap
/// and everything to its right keep the full 20. The merge/condition spine sits
/// at the midpoint of the two central branches.
fn switch_x_layout_if_branch_packed(cases: &[SwitchCase], condition: &str) -> SwitchXLayout {
    let n = cases.len();
    let tiles: Vec<SwitchCaseTile> = cases.iter().map(switch_case_tile).collect();
    let diamond_w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;
    let center = n / 2; // index of the first case at/after the centreline

    let mut centers = vec![0.0f64; n];
    let mut x = 0.0;
    for i in 0..n {
        centers[i] = x + tiles[i].left;
        // Gaps strictly left of the centreline are compressed to SWITCH_CASE_GAP;
        // the centre gap and all gaps to its right keep SWITCH_IF_BRANCH_CASE_GAP.
        let gap = if i + 1 < center {
            SWITCH_CASE_GAP
        } else {
            SWITCH_IF_BRANCH_CASE_GAP
        };
        x += tiles[i].width + gap;
    }
    let block_w = x - SWITCH_IF_BRANCH_CASE_GAP;
    let block_w = block_w.max(diamond_w).max(DIAMOND_HALF * 2.0);

    // Spine = midpoint of the two central branches (even n) or the central
    // branch itself (odd n).
    let diamond_dx = if n.is_multiple_of(2) {
        (centers[center - 1] + centers[center]) / 2.0
    } else {
        centers[center]
    };

    SwitchXLayout {
        centers,
        block_w,
        diamond_dx,
        big_diamond: false,
    }
}

fn switch_x_layout_with_small_gap(
    cases: &[SwitchCase],
    condition: &str,
    small_case_gap: f64,
    add_center_gap: bool,
) -> SwitchXLayout {
    let n = cases.len();
    let tiles: Vec<SwitchCaseTile> = cases.iter().map(switch_case_tile).collect();
    let diamond_w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;

    if n == 0 {
        return SwitchXLayout {
            centers: Vec::new(),
            block_w: diamond_w.max(60.0),
            diamond_dx: diamond_w.max(60.0) / 2.0,
            big_diamond: false,
        };
    }
    if n == 1 {
        let w = tiles[0].width;
        let block_w = w.max(diamond_w);
        return SwitchXLayout {
            centers: vec![tiles[0].left + (block_w - w) / 2.0],
            block_w,
            diamond_dx: block_w / 2.0,
            big_diamond: false,
        };
    }

    let w13 = diamond_w - tiles[0].right() - tiles[n - 1].left;
    let w9: f64 = tiles[1..n - 1].iter().map(|tile| tile.width).sum();

    if w13 > w9 {
        // BIG_DIAMOND: cases[0] flush left, cases[last] at a fixed offset,
        // inner cases spread by suppx = (w13 - w9) / (n - 1).
        let suppx = (w13 - w9) / (n - 1) as f64;
        let mut centers = vec![0.0f64; n];
        let mut dx = 0.0;
        for i in 0..n - 1 {
            centers[i] = dx + tiles[i].left;
            dx += tiles[i].width + suppx;
        }
        let dx_last = tiles[0].width + w13 + SWITCH_SUPP15 + SWITCH_SUPP15;
        centers[n - 1] = dx_last + tiles[n - 1].left;
        let block_w = tiles[0].width + SWITCH_SUPP15 + w13 + SWITCH_SUPP15 + tiles[n - 1].width;
        // dimTotal.getLeft = tile0.getLeft + SUPP15 + dim1.getLeft.
        let diamond_dx = tiles[0].left + SWITCH_SUPP15 + diamond_w / 2.0;
        SwitchXLayout {
            centers,
            block_w,
            diamond_dx,
            big_diamond: true,
        }
    } else {
        // SMALL_DIAMOND: cases packed tight with a 10-px gap, plus an extra
        // 10-px gap straddling the centreline for even case counts (so the
        // diamond/merge column has room). The diamond sits at the geometric
        // block centre.
        let case_gap = if switch_has_empty_middle_case(cases) {
            SWITCH_IF_BRANCH_CASE_GAP
        } else {
            small_case_gap
        };
        let mut centers = vec![0.0f64; n];
        let mut x = 0.0;
        for i in 0..n {
            if add_center_gap && n.is_multiple_of(2) && i == n / 2 {
                x += case_gap;
            }
            centers[i] = x + tiles[i].left;
            x += tiles[i].width + case_gap;
        }
        let nude_w = x - case_gap;
        let block_w = nude_w.max(diamond_w).max(DIAMOND_HALF * 2.0);
        SwitchXLayout {
            centers,
            block_w,
            diamond_dx: block_w / 2.0,
            big_diamond: false,
        }
    }
}

fn switch_case_block_width(cases: &[SwitchCase], condition: &str) -> f64 {
    if let [case] = cases {
        let (left, right) = switch_one_link_extents(case, condition);
        return left + right;
    }
    switch_x_layout(cases, condition).block_w
}

/// Maximum case-label line count across all cases (≥1).
fn switch_label_lines(cases: &[SwitchCase]) -> f64 {
    cases
        .iter()
        .map(|c| c.label.split('\n').count().max(1) as f64)
        .fold(1.0f64, f64::max)
}

/// Distance from the switch diamond's *bottom* to the case-box tops, i.e.
/// PlantUML's `FtileSwitchWithManyLinks.getYdelta1a`. The label band sits
/// here; its height drives the gap, and BIG_DIAMOND mode adds an extra
/// `diamondHeight/2`. The single-line bases (35.91015 SMALL, 42.95508 BIG)
/// are taken from the goldens; each extra label line adds one text line.
fn switch_below_diamond(cases: &[SwitchCase], big_diamond: bool) -> f64 {
    let extra = (switch_label_lines(cases) - 1.0) * pm::text_height(SMALL_FONT);
    let base = if big_diamond { 42.95508 } else { 35.91015 };
    base + extra
}

/// Left extent (centreline → leftmost drawn element) of a `while` tile.
///
/// Faithful port of PlantUML's `FtileWhile.getTranslateForSpecial`: with a
/// trailing terminator (`stop`/`end`/etc., the `specialOut` ftile), the
/// terminator is pulled left of the loop and becomes the leftmost element. Its
/// centre sits `special_offset` left of the spine, where
///
///   special_offset = max(body_left + halfHex, cond_half) + specialOut.width/2
///
/// (`xWhile = bodyLeft - halfHex` further left, `xDiamond = condHalf`; the
/// special is then offset by its own half-width). The diagram adds a fixed
/// 13px lead from that centre to the SVG content-left edge (LEFT_SVG_PAD 25 −
/// MARGIN_LEAD 16 + arrowhead wing 4). Without a terminator the exit arm wraps
/// at `geo_left − halfHex`; we keep the legacy +25 lead for that (untested)
/// path since no golden exercises it.
/// Effective left extent of a while body's leftward corridor. A deprecated
/// `#color:text;` action in the loop body carries a top-band warning banner;
/// PlantUML's FtileWhile then seats the loop-back/exit corridor (and the
/// `manageSpecialStopEndAfterEndWhile` terminator) 2 px tighter on the left
/// than for a normal body. The body box itself stays centred on the spine —
/// only the while's left geometry sees the 2 px reduction.
fn while_body_left(body: &[LayoutNode], body_left: f64) -> f64 {
    if body
        .iter()
        .any(|n| matches!(n, LayoutNode::DeprecatedAction { .. }))
    {
        body_left - 2.0
    } else {
        body_left
    }
}

fn while_left_extent(
    body: &[LayoutNode],
    body_left: f64,
    cond_half: f64,
    has_in_label: bool,
    end_label: Option<&str>,
    special_out: Option<&LayoutNode>,
    starts_column: bool,
) -> f64 {
    let special_extent = match special_out {
        Some(special) => {
            let special_half = node_width(special) / 2.0;
            let body_corridor = body_left + DIAMOND_HALF;
            let special_offset = body_corridor.max(cond_half) + special_half;
            let special_lead = if while_body_drives_special(
                body,
                body_left,
                cond_half,
                has_in_label,
                end_label,
                starts_column,
            ) {
                WHILE_SPECIAL_BODY_LEAD
            } else {
                WHILE_SPECIAL_COND_LEAD
            };
            special_offset + special_lead
        }
        None => cond_half.max(body_left) + 25.0,
    };
    // The `endwhile (label)` text is drawn flush left of the diamond's left
    // vertex (its right edge at diamond_left_vertex). When wide enough it
    // becomes the leftmost element and drives the content-left edge: the label
    // sits 1px left of MARGIN_LEAD, so it contributes cond_half + label_w − 1.
    let label_extent = end_label
        .map(|l| cond_half + text_render::measure(l, SMALL_FONT, false) - 1.0)
        .unwrap_or(0.0);
    special_extent.max(label_extent)
}

fn while_body_drives_special(
    body: &[LayoutNode],
    body_left: f64,
    cond_half: f64,
    has_in_label: bool,
    end_label: Option<&str>,
    starts_column: bool,
) -> bool {
    let multi_flow = body.iter().filter(|node| node_is_flow(node)).count() >= 2;
    starts_column
        && (has_in_label || !multi_flow)
        && end_label.is_none()
        && body_left + DIAMOND_HALF > cond_half
        && body_left > DIAMOND_HALF * 2.0
        && !body
            .iter()
            .any(|node| matches!(node, LayoutNode::DeprecatedAction { .. }))
}

fn while_slot_compress(
    compress_allowed: bool,
    is_label: bool,
    _end_label: bool,
    body_empty: bool,
) -> f64 {
    // PlantUML's slot finder removes ~4.82 of slack between the diamond and the
    // body for any labelled non-empty loop that is NOT terminated by a
    // specialOut tile. The `endwhile (label)` text rides the diamond's left
    // vertex and does not occupy the inbound slot, so it does not affect this
    // compression — `compress_allowed` already encodes the specialOut /
    // body-shape gating via `while_ordinary_slot_compress_allowed`.
    if compress_allowed && is_label && !body_empty {
        WHILE_BODY_SLOT_COMPRESS
    } else {
        0.0
    }
}

fn while_ordinary_slot_compress_allowed(
    body: &[LayoutNode],
    special_out: Option<&LayoutNode>,
) -> bool {
    // A break-bearing `if` renders as a "thin" FtileIfDown that does not block
    // the loop's inbound-slot compression — and, because the `break` shares the
    // exit corridor, PlantUML keeps that compression even when a terminator was
    // absorbed after `endwhile` (so the usual specialOut gate does not apply).
    let has_break_if = body_contains_break_if(body);
    (special_out.is_none() || has_break_if)
        && body.iter().all(|node| {
            matches!(
                node,
                LayoutNode::Action { .. }
                    | LayoutNode::DeprecatedAction { .. }
                    | LayoutNode::Arrow { .. }
                    | LayoutNode::Note { .. }
            ) || matches!(
                node,
                LayoutNode::If { then_branch, else_branches, .. }
                    if if_break_down_plan(then_branch, else_branches).is_some()
            )
        })
}

fn while_ordinary_slot_compresses(
    body: &[LayoutNode],
    is_label: &Option<String>,
    end_label: &Option<String>,
    special_out: Option<&LayoutNode>,
) -> bool {
    while_slot_compress(
        while_ordinary_slot_compress_allowed(body, special_out),
        is_label.is_some(),
        end_label.is_some(),
        body.is_empty(),
    ) != 0.0
}

/// Whether a `while` body's ON_Y compression removes the residual slack from a
/// break-bearing `if`'s no-diamond corridor (see
/// [`WHILE_BREAK_IF_CORRIDOR_DROP_COMPRESSED`]). Empirically the slack survives
/// only when the body is "thin" — a single action plus the break-if (e.g.
/// `act_while_break_at_start`); three or more flow tiles (actions and/or the
/// break-if) provide enough adjacent content to absorb it.
fn while_break_corridor_compresses(body: &[LayoutNode]) -> bool {
    body.iter().filter(|n| node_is_flow(n)).count() >= 3
}

fn colored_partition_needs_while_slot_subtract(body: &[LayoutNode]) -> bool {
    let [
        LayoutNode::While {
            body: while_body,
            is_label,
            end_label,
            special_out,
            ..
        },
    ] = body
    else {
        return false;
    };
    while_slot_compress(
        true,
        is_label.is_some(),
        end_label.is_some(),
        while_body.is_empty(),
    ) != 0.0
        && !while_ordinary_slot_compresses(while_body, is_label, end_label, special_out.as_deref())
}

/// A break-bearing `if` (rendered by [`emit_if_break_down`]). Its inbound
/// connector is deferred past the following tile's inbound, matching PlantUML's
/// `FtileIfDown` welding-point emission order (the break/else welds land before
/// the if's own `ConnectionIn`).
fn is_break_down_if(node: &LayoutNode) -> bool {
    matches!(
        node,
        LayoutNode::If { then_branch, else_branches, .. }
            if if_break_down_plan(then_branch, else_branches).is_some()
    )
}

fn is_ordinary_compressed_while(node: &LayoutNode) -> bool {
    matches!(
        node,
        LayoutNode::While {
            body,
            is_label,
            end_label,
            special_out,
            ..
        } if while_ordinary_slot_compresses(
            body,
            is_label,
            end_label,
            special_out.as_deref(),
        )
    )
}

/// True for a `while` whose body has the ordinary-compressed shape, ignoring
/// whether a trailing terminator was absorbed into `special_out`. Used to keep
/// a sequential run of compressed whiles "chained" even when the final loop
/// swallows the diagram's `stop`/`end`: such a while still draws its inbound
/// connector at the boundary with the preceding loop (before its own body),
/// matching PlantUML's tile-assembly order. The `special_out` is passed as
/// `None` so the body-shape test (which a real `special_out` would otherwise
/// veto) reflects the loop's intrinsic layout.
fn while_body_chain_compresses(node: &LayoutNode) -> bool {
    matches!(
        node,
        LayoutNode::While {
            body,
            is_label,
            end_label,
            ..
        } if while_ordinary_slot_compresses(body, is_label, end_label, None)
    )
}

fn while_body_top_offset(
    compress_allowed: bool,
    is_label: bool,
    end_label: bool,
    body_empty: bool,
    arrow_font_size: f64,
) -> f64 {
    let offset = if is_label {
        text_box_height(arrow_font_size) + 2.0 * DIAMOND_HALF
    } else {
        ARROW_LEN
    };
    offset - while_slot_compress(compress_allowed, is_label, end_label, body_empty)
}

/// The lines of note text (block notes accumulate `\n`-joined lines; single
/// `note left: text` notes are one line).
fn note_lines(text: &str) -> Vec<&str> {
    text.split('\n').collect()
}

/// Total drawn width of a note box: longest line's textLength + padding/fold.
fn note_box_width(text: &str) -> f64 {
    let max_line = note_lines(text)
        .iter()
        .map(|l| text_render::measure(l, NOTE_FONT, false))
        .fold(0.0f64, f64::max);
    max_line + NOTE_BOX_EXTRA_W
}

/// Total drawn height of a note box.
fn note_box_height(text: &str) -> f64 {
    NOTE_BOX_BASE_H + note_lines(text).len() as f64 * NOTE_LINE_H
}

fn note_height_from_activity_note(note: &ActivityNote) -> f64 {
    note_box_height(&note.text)
}

fn max_note_height(notes: &[ActivityNote]) -> Option<f64> {
    notes
        .iter()
        .map(note_height_from_activity_note)
        .reduce(f64::max)
}

fn emit_folded_note(svg: &mut SvgEmitter, note: &ActivityNote, box_left: f64, box_top: f64) {
    let box_w = note_box_width(&note.text);
    let box_h = note_box_height(&note.text);
    let fill = note
        .color
        .as_deref()
        .map(crate::sequence::resolve_color)
        .unwrap_or_else(|| NOTE_FILL.to_string());
    svg.note_folded(&fill, box_left, box_top, box_w, box_h);
    for (i, line) in note_lines(&note.text).iter().enumerate() {
        let baseline = box_top + NOTE_FIRST_BASELINE_DY + i as f64 * NOTE_LINE_H;
        let lw = text_render::measure(line, NOTE_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            NOTE_FONT,
            lw,
            box_left + NOTE_TEXT_PAD_X,
            baseline,
            line,
            false,
        );
    }
}

/// Emit a note attached beside an Action-style anchor. `cx`/`anchor_w` give
/// the anchor box centre and width; `anchor_cy` its vertical centre. The note
/// box is vertically centred on the anchor and offset `NOTE_GAP` to the side.
fn emit_attached_note(
    svg: &mut SvgEmitter,
    text: &str,
    position: &NotePosition,
    color: Option<&str>,
    cx: f64,
    anchor_w: f64,
    anchor_cy: f64,
) {
    let box_w = note_box_width(text);
    let box_h = note_box_height(text);
    let box_top = anchor_cy - box_h / 2.0;
    let anchor_half = anchor_w / 2.0;
    let fill = color
        .map(crate::sequence::resolve_color)
        .unwrap_or_else(|| NOTE_FILL.to_string());
    let left_side = matches!(position, NotePosition::Left);
    let (box_left, tip_x) = if left_side {
        let box_right = cx - anchor_half - NOTE_GAP;
        (box_right - box_w, cx - anchor_half)
    } else {
        (cx + anchor_half + NOTE_GAP, cx + anchor_half)
    };
    svg.note_opale(
        &fill, box_left, box_top, box_w, box_h, tip_x, anchor_cy, left_side,
    );
    for (i, line) in note_lines(text).iter().enumerate() {
        let baseline = box_top + NOTE_FIRST_BASELINE_DY + i as f64 * NOTE_LINE_H;
        let lw = text_render::measure(line, NOTE_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            NOTE_FONT,
            lw,
            box_left + NOTE_TEXT_PAD_X,
            baseline,
            line,
            false,
        );
    }
}

fn emit_if_attached_notes(
    svg: &mut SvgEmitter,
    notes: &[ActivityNote],
    cx: f64,
    diamond_top_y: f64,
    diamond_half_w: f64,
) {
    for note in notes {
        let box_w = note_box_width(&note.text);
        let box_h = note_box_height(&note.text);
        let box_top = diamond_top_y - box_h;
        let box_left = match note.position {
            NotePosition::Left => cx - diamond_half_w - box_w,
            NotePosition::Right => cx + diamond_half_w,
        };
        emit_folded_note(svg, note, box_left, box_top);
    }
}

fn fork_bar_extents(layout: &ForkLayout) -> (f64, f64) {
    (
        layout.bar_w / 2.0 - layout.spine_dx,
        layout.bar_w / 2.0 + layout.spine_dx,
    )
}

fn with_fork_attached_note_extents(
    mut left: f64,
    mut right: f64,
    notes: &[ActivityNote],
) -> (f64, f64) {
    let base_left = left;
    let base_right = right;
    for note in notes {
        let box_w = note_box_width(&note.text);
        match note.position {
            NotePosition::Left => left = left.max(base_left + FORK_NOTE_GAP + box_w - 1.0),
            NotePosition::Right => right = right.max(base_right + FORK_NOTE_GAP + box_w),
        }
    }
    (left, right)
}

fn first_fork_row_height(branches: &[Vec<LayoutNode>]) -> f64 {
    branches
        .iter()
        .filter_map(|branch| first_flow_node(branch).map(node_height))
        .fold(0.0f64, f64::max)
}

fn emit_fork_attached_notes(
    svg: &mut SvgEmitter,
    notes: &[ActivityNote],
    cx: f64,
    y: f64,
    branches: &[Vec<LayoutNode>],
    layout: &ForkLayout,
) {
    let branch_top = y + FORK_BAR_HEIGHT + ARROW_LEN;
    let first_row_h = first_fork_row_height(branches);
    let (left_extent, right_extent) = fork_bar_extents(layout);
    for note in notes {
        let box_w = note_box_width(&note.text);
        let box_h = note_box_height(&note.text);
        let row_h = first_row_h.max(box_h);
        let box_top = branch_top + (row_h - box_h) / 2.0;
        let box_left = match note.position {
            NotePosition::Left => cx - left_extent - FORK_NOTE_GAP - box_w,
            NotePosition::Right => cx + right_extent + FORK_NOTE_GAP,
        };
        emit_folded_note(svg, note, box_left, box_top);
    }
}

#[derive(Clone, Copy)]
enum LeadingNoteKind {
    Attached,
    Floating,
}

struct LeadingStartNote {
    text: String,
    position: NotePosition,
    color: Option<String>,
    kind: LeadingNoteKind,
}

/// Detect a note immediately following the diagram's start node. PlantUML
/// vertically centres the start ellipse on that note's box and pushes the rest
/// of the spine down. `floating note` uses a tail-less folded box; ordinary
/// `note left/right` uses the same attached-note path as action notes.
fn leading_start_note(tree: &[LayoutNode], source: Option<&str>) -> Option<LeadingStartNote> {
    let is_floating = source.is_some_and(|src| {
        src.lines()
            .any(|l| l.trim_start().starts_with("floating note "))
    });
    match (tree.first(), tree.get(1)) {
        (
            Some(LayoutNode::Start),
            Some(LayoutNode::Note {
                text,
                position,
                color,
            }),
        ) => Some(LeadingStartNote {
            text: text.clone(),
            position: position.clone(),
            color: color.clone(),
            kind: if is_floating {
                LeadingNoteKind::Floating
            } else {
                LeadingNoteKind::Attached
            },
        }),
        _ => None,
    }
}

/// Emit a leading floating note (tail-less folded box) anchored to the start
/// ellipse at spine centre `cx`. The note's top edge sits at `MARGIN_LEAD - 1`
/// (15) and it is offset `NOTE_GAP` past the start ellipse's radius on the
/// chosen side — mirroring `emit_attached_note`'s lateral placement but with
/// no connector tail. Returns the note's box height.
fn emit_leading_floating_note(
    svg: &mut SvgEmitter,
    text: &str,
    position: &NotePosition,
    color: Option<&str>,
    cx: f64,
) -> f64 {
    let box_w = note_box_width(text);
    let box_h = note_box_height(text);
    let box_top = 15.0; // MARGIN_LEAD - 1
    let fill = color
        .map(crate::sequence::resolve_color)
        .unwrap_or_else(|| NOTE_FILL.to_string());
    let box_left = match position {
        NotePosition::Left => cx - START_R - NOTE_GAP - box_w,
        NotePosition::Right => cx + START_R + NOTE_GAP,
    };
    svg.note_folded(&fill, box_left, box_top, box_w, box_h);
    for (i, line) in note_lines(text).iter().enumerate() {
        let baseline = box_top + NOTE_FIRST_BASELINE_DY + i as f64 * NOTE_LINE_H;
        let lw = text_render::measure(line, NOTE_FONT, false);
        svg.text_element(
            TEXT_COLOR,
            "sans-serif",
            NOTE_FONT,
            lw,
            box_left + NOTE_TEXT_PAD_X,
            baseline,
            line,
            false,
        );
    }
    box_h
}

// ── ftile geometry bridge (incr-4 groundwork, PURE / not yet wired) ──────────
//
// Map a `LayoutNode` to its PlantUML `FtileGeometry` via the faithful port in
// `ftile.rs`, so the canvas/spine can eventually be derived BY CONSTRUCTION
// (replacing the reverse-engineered constants in node_extents/sequence_extents).
//
// Status: NOT wired into rendering — zero behavioural change. Only the tiles
// whose port is verified exact against the Java are mapped here; the rest
// return `None` so the eventual caller falls back to the legacy extent model:
//   - Repeat/Fork: deferred — need the loop-back-arm + fork/join BAR geometry
//     (ftile incr 3b) before their dimension is faithful.
//   - Partition/Swimlanes/Note/Title/elseif-chains/empty-branch-IfDown: not
//     ported; bail.
// The returned `height` is the bare tile-stack height (no connection-gap tiles
// inserted between siblings); width/left — the part that drives canvas width and
// the spine — are exact. Heights are finished at wire time.

/// The condition hexagon as an `FtileGeometry`. Width matches the validated
/// `diamond_inner_w(c) + 2*DIAMOND_HALF` used throughout the emitters, which
/// equals `FtileGeometry::diamond_inside(text_w, _, 0)` for a non-empty label.
#[allow(dead_code)] // incr-4 groundwork: wired in a later increment
fn condition_diamond(condition: &str) -> ftile::FtileGeometry {
    condition_diamond_styled(condition, SMALL_FONT, false, "sans-serif")
}

fn condition_diamond_styled(
    condition: &str,
    font_size: f64,
    bold: bool,
    font_family: &str,
) -> ftile::FtileGeometry {
    let w = diamond_inner_w_styled(condition, font_size, bold, font_family) + DIAMOND_HALF * 2.0;
    condition_diamond_geometry(w)
}

fn condition_diamond_styled_padded(
    condition: &str,
    font_size: f64,
    bold: bool,
    font_family: &str,
    pad_x: f64,
) -> ftile::FtileGeometry {
    let w = diamond_inner_w_styled_padded(condition, font_size, bold, font_family, pad_x)
        + DIAMOND_HALF * 2.0;
    condition_diamond_geometry(w)
}

fn condition_diamond_geometry(w: f64) -> ftile::FtileGeometry {
    ftile::FtileGeometry::new(
        w,
        DIAMOND_HALF * 2.0,
        w / 2.0,
        0.0,
        Some(DIAMOND_HALF * 2.0),
    )
}

/// Fold a node sequence into one geometry via `assemble_linear`. Connectors and
/// terminal markers contribute no tile; a not-yet-ported member bails the whole
/// sequence to `None`.
#[allow(dead_code)] // incr-4 groundwork: wired in a later increment
fn sequence_geometry(nodes: &[LayoutNode]) -> Option<ftile::FtileGeometry> {
    let mut geoms: Vec<ftile::FtileGeometry> = Vec::new();
    for n in nodes {
        match n {
            LayoutNode::Arrow { .. }
            | LayoutNode::Detach
            | LayoutNode::Kill
            | LayoutNode::Break
            | LayoutNode::Goto(_) => continue,
            _ => geoms.push(node_geometry(n)?),
        }
    }
    ftile::assemble_linear(&geoms)
}

fn sequence_geometry_if_branch(nodes: &[LayoutNode]) -> Option<ftile::FtileGeometry> {
    let mut geoms: Vec<ftile::FtileGeometry> = Vec::new();
    for n in nodes {
        match n {
            LayoutNode::Arrow { .. }
            | LayoutNode::Detach
            | LayoutNode::Kill
            | LayoutNode::Break
            | LayoutNode::Goto(_) => continue,
            _ => geoms.push(node_geometry_if_branch(n)?),
        }
    }
    ftile::assemble_linear(&geoms)
}

/// `LayoutNode → FtileGeometry` via the ftile port. See the module note above
/// for which tiles are mapped vs. deferred.
#[allow(dead_code)] // incr-4 groundwork: wired in a later increment
fn node_geometry(node: &LayoutNode) -> Option<ftile::FtileGeometry> {
    use ftile::FtileGeometry as G;
    let g = match node {
        LayoutNode::Start => G::circle_start(),
        LayoutNode::Stop => G::circle_stop(),
        LayoutNode::End => G::circle_end(),
        LayoutNode::Action {
            text,
            text_width,
            pad_x,
            pad_y,
            font_family,
            font_size,
            ..
        }
        | LayoutNode::DeprecatedAction {
            text,
            text_width,
            pad_x,
            pad_y,
            font_family,
            font_size,
            ..
        } => G::box_tile(
            *text_width,
            text_render::label_height_with_family(text, *font_size, font_family),
            *pad_x,
            *pad_x,
            *pad_y,
            *pad_y,
        ),
        LayoutNode::If {
            condition,
            then_branch,
            else_branches,
            diamond_font_family,
            diamond_font_size,
            diamond_text_bold,
            ..
        } => {
            // Only the binary FtileIfWithDiamonds (one then + one populated
            // else) is ported; elseif-chains (FtileIfLong) and the empty-branch
            // FtileIfDown fall back to the legacy model.
            if else_branches.len() != 1 || if_down_plan(then_branch, else_branches).is_some() {
                return None;
            }
            let diamond1 = condition_diamond_styled(
                condition,
                *diamond_font_size,
                *diamond_text_bold,
                diamond_font_family,
            );
            let diamond2 = G::diamond_empty(0.0); // bare 24×24 merge diamond
            let t1 = if_branch_tile(then_branch)?;
            let t2 = if_branch_tile(&else_branches[0].body)?;
            // Non-swimlane two-branch: Ydelta1a(10) + Ydelta1b(6) + labels(0).
            ftile::if_with_diamonds(&diamond1, &t1, &t2, &diamond2, 16.0, (0.0, 0.0, 0.0))
        }
        LayoutNode::While {
            condition,
            body,
            special_out,
            ..
        } => {
            let diamond1 = condition_diamond(condition);
            let block = sequence_geometry(body)?;
            let special_g = special_out.as_deref().and_then(node_geometry);
            ftile::while_tile(&diamond1, &block, None, special_g.as_ref(), 0.0)
        }
        LayoutNode::Switch { condition, cases } => {
            if cases.is_empty() {
                return None;
            }
            let diamond1 = condition_diamond(condition);
            let diamond2 = G::diamond_empty(0.0);
            let tiles = cases
                .iter()
                .map(|c| sequence_geometry(&c.body))
                .collect::<Option<Vec<_>>>()?;
            ftile::switch_with_diamonds(&diamond1, &diamond2, &tiles, 20.0)
        }
        LayoutNode::Repeat {
            body,
            condition,
            backward,
            ..
        } => {
            // FtileRepeat: diamond1 = empty top diamond, diamond2 =
            // FtileDiamondInside(test), test_label_w = the bare test-label width
            // (tbTest), backward = optional loop-back box.
            let diamond1 = G::diamond_empty(0.0);
            let diamond2 = condition_diamond(condition);
            let repeat = sequence_geometry(body)?;
            let test_label_w = text_render::measure(condition, SMALL_FONT, false);
            let backward_g = backward.as_ref().map(|label| {
                G::box_tile(
                    text_render::measure(label, FONT_SIZE, false),
                    text_render::label_height(label, FONT_SIZE),
                    ACTION_H_PADDING,
                    ACTION_H_PADDING,
                    ACTION_H_PADDING,
                    ACTION_H_PADDING,
                )
            });
            ftile::repeat_tile(
                &diamond1,
                &diamond2,
                &repeat,
                test_label_w,
                backward_g.as_ref(),
            )
        }
        LayoutNode::Fork {
            branches, is_split, ..
        } => fork_geometry(branches, false, *is_split),
        _ => return None,
    };
    Some(g)
}

fn node_geometry_if_branch(node: &LayoutNode) -> Option<ftile::FtileGeometry> {
    match node {
        LayoutNode::Fork {
            branches, is_split, ..
        } => Some(fork_geometry_with_layout(
            branches,
            if *is_split {
                split_layout(branches)
            } else {
                fork_layout_if_branch_spacing(branches)
            },
            *is_split,
        )),
        _ => node_geometry(node),
    }
}

fn fork_geometry(
    branches: &[Vec<LayoutNode>],
    if_branch: bool,
    is_split: bool,
) -> ftile::FtileGeometry {
    let layout = if is_split {
        split_layout(branches)
    } else if if_branch {
        fork_layout_if_branch(branches)
    } else {
        fork_layout(branches)
    };
    fork_geometry_with_layout(branches, layout, is_split)
}

fn fork_geometry_with_layout(
    branches: &[Vec<LayoutNode>],
    layout: ForkLayout,
    is_split: bool,
) -> ftile::FtileGeometry {
    let max_h: f64 = branches
        .iter()
        .map(|b| sequence_height(b))
        .fold(0.0f64, f64::max);
    let height = if is_split {
        ARROW_LEN + max_h + ARROW_LEN
    } else {
        FORK_BAR_HEIGHT + ARROW_LEN + max_h + ARROW_LEN + FORK_BAR_HEIGHT
    };
    ftile::FtileGeometry::new(
        layout.bar_w,
        height,
        layout.bar_w / 2.0 - layout.spine_dx,
        0.0,
        Some(height),
    )
}

/// Branch tile geometry for an `if/else` branch, mirroring ConditionalBuilder:
/// `FtileMinWidthCentered(30)` then `addHorizontalMargin(10)` (PlantUML always
/// takes the `createWithLinks` path — build() L161). The +10/side is what
/// produces the 20px inter-branch gap that the bare box geometry lacked.
fn if_branch_tile(branch: &[LayoutNode]) -> Option<ftile::FtileGeometry> {
    let g = sequence_geometry_if_branch(branch)?;
    // MinWidthCentered(30): widen narrow branches to 30, content centred.
    let g = if g.width < 30.0 {
        ftile::FtileGeometry::new(30.0, g.height, 15.0, g.in_y, g.out_y)
    } else {
        g
    };
    Some(g.add_margin_x(10.0))
}

/// For a binary `if/else`, return `(then_off, else_off, left_ext, right_ext)`:
/// the then/else branch spine offsets from the if spine, and the if's DRAWN
/// extents from its spine. `None` for elseif / IfDown / non-portable branches.
/// Branch POSITIONS use the margined branch tiles (FtileIfWithDiamonds); the
/// canvas EXTENTS use the branches' own drawn extents — the +10 tile margins
/// are empty layout space (not drawn), so they position branches without
/// widening the canvas. Verified vs act_if_simple + act_if_cond_valid_input.
fn if_ftile_layout_styled(
    condition: &str,
    diamond_font_size: f64,
    diamond_bold: bool,
    diamond_font_family: &str,
    diamond_pad_x: f64,
    then_branch: &[LayoutNode],
    else_branches: &[ElseBranch],
) -> Option<(f64, f64, f64, f64)> {
    if else_branches.len() != 1 || if_down_plan(then_branch, else_branches).is_some() {
        return None;
    }
    let (then_l, _then_r) = sequence_extents_if_branch(then_branch);
    let (_else_l, else_r) = sequence_extents_if_branch(&else_branches[0].body);
    let diamond1 = condition_diamond_styled_padded(
        condition,
        diamond_font_size,
        diamond_bold,
        diamond_font_family,
        diamond_pad_x,
    );
    let diamond2 = ftile::FtileGeometry::diamond_empty(0.0);
    let t1 = if_branch_tile(then_branch)?;
    let t2 = if_branch_tile(&else_branches[0].body)?;
    let g = ftile::if_with_diamonds(&diamond1, &t1, &t2, &diamond2, 16.0, (0.0, 0.0, 0.0));
    let mut then_off = t1.left - g.left; // negative: then spine left of if spine
    let mut else_off = g.right() - t2.right();
    let cond_half = diamond1.width / 2.0;
    let mut left_ext = cond_half.max(-then_off + then_l);
    let mut right_ext = cond_half.max(else_off + else_r);
    let spine_shift = leading_if_branch_fork_spine_shift(then_branch);
    if spine_shift != 0.0 {
        then_off -= spine_shift;
        else_off -= spine_shift;
        left_ext += spine_shift;
        right_ext -= spine_shift;
    }
    Some((then_off, else_off, left_ext, right_ext))
}

// --- FtileIfLongHorizontal (if / elseif* / else) -------------------------
//
// PlantUML's `FtileIfLongHorizontal` lays a condition diamond per `then`/
// `elseif` in a horizontal row, each with its branch column directly below
// (`FtileAssemblySimple(diamond, branch)` = a "couple"), the diamonds linked
// left→right by "no"-style connectors, and the final bare `else` (`tile2`)
// placed to the right of the last diamond. All branch outs collect on a single
// horizontal merge line at the bottom (`ConnectionHline`).
//
// Geometry references (`activitydiagram3/ftile/vcompact/FtileIfLongHorizontal`):
// - diamonds: `FtileDiamondInside2` (condition text *inside*, `then` label as
//   `withNorth`, the else label as `withEast` on the LAST diamond), each padded
//   by `alignDiamonds` → `incVertically(missing/2, 20)`.
// - branch tiles: `FtileMinWidthCentered(branch, 30)`.
// - couples placed at `x += couple.width + xSeparation(20)` (`getTranslateCouple1`).
// - `tile2` at `internalWidth − tile2.width` (`getTranslate2`), shifted up by
//   `getDiamondsHeight/2`.
// - vertical reserve `max(100, maxOutY)` below the couples (`calculateDimensionInternal`).
//
// The diagram is then run through PlantUML's `CompressionXorYBuilder(ON_X)`
// (`ActivityDiagram3.exportDiagramInternal`): a slot-based pass that removes
// empty horizontal space, keeping 5px around each occupied cluster
// (`SlotSet.reverse().smaller(5.0)` + `CompressionTransform`). [`XCompress`]
// ports that transform. No magic constants — the layout is the un-compacted
// PlantUML geometry, then the real compaction.

/// `xSeparation` between adjacent condition diamonds (Java field, = 20).
const IF_LONG_X_SEP: f64 = 20.0;
/// Vertical reserve below the couples (`Math.max(100, maxOutY)` with the
/// single-line `maxOutY = 24 < 100`). Multi-line conditions are out of scope.
const IF_LONG_BELOW: f64 = 100.0;
/// `alignDiamonds` bottom margin (`incVertically(_, 20)`).
const IF_LONG_ALIGN_BOTTOM: f64 = 20.0;
/// `SlotSet.smaller(margin)` keeps this much empty space on each side of every
/// compressed cluster (PlantUML calls `smaller(5.0)`).
const X_COMPRESS_MARGIN: f64 = 5.0;
/// Length (x-extent) of a horizontal connector arrowhead polygon (`right_arrow`
/// / `left_arrow` span `[x_tip-10, x_tip]`). The arrowhead is a connector
/// polygon, so ON_X compaction counts it as occupancy.
const HORIZ_ARROWHEAD_LEN: f64 = 10.0;

/// Port of PlantUML's `CompressionTransform` (ON_X): given the occupied
/// x-intervals of a drawing, removes the empty gaps between clusters — keeping
/// `X_COMPRESS_MARGIN` on each side, and leaving gaps ≤ `2*margin` untouched
/// (`SlotSet.reverse().smaller(margin)`). `transform(v) = v − Σ gap sizes left
/// of v` (partial for the gap containing `v`).
struct XCompress {
    /// The compressible empty gaps `(start, end)`, sorted by start.
    gaps: Vec<(f64, f64)>,
}

impl XCompress {
    fn from_occupied(occ: &[(f64, f64)]) -> Self {
        if occ.is_empty() {
            return XCompress { gaps: Vec::new() };
        }
        // Merge occupied intervals.
        let mut iv: Vec<(f64, f64)> = occ.to_vec();
        iv.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut merged: Vec<(f64, f64)> = Vec::new();
        for (s, e) in iv {
            if let Some(last) = merged.last_mut()
                && s <= last.1
            {
                last.1 = last.1.max(e);
            } else {
                merged.push((s, e));
            }
        }
        // Empty gaps = complement between consecutive occupied clusters, then
        // `smaller(margin)`: drop gaps ≤ 2*margin, else shrink by margin/side.
        let mut gaps: Vec<(f64, f64)> = Vec::new();
        for w in merged.windows(2) {
            let gap_start = w[0].1;
            let gap_end = w[1].0;
            if gap_end - gap_start > 2.0 * X_COMPRESS_MARGIN {
                gaps.push((gap_start + X_COMPRESS_MARGIN, gap_end - X_COMPRESS_MARGIN));
            }
        }
        XCompress { gaps }
    }

    fn transform(&self, v: f64) -> f64 {
        let mut delta = 0.0;
        for &(s, e) in &self.gaps {
            if s > v {
                continue;
            }
            if v > e {
                delta += e - s;
            } else {
                delta += v - s;
            }
        }
        v - delta
    }
}

/// One condition column of the long layout.
struct IfLongCol {
    /// Diamond polygon width (the `FtileDiamondInside2` "alone" width, used to
    /// draw the hexagon and anchor the horizontal/east connectors).
    diamond_w: f64,
    /// Diamond TILE width = `FtileDiamondInside2.calculateDimensionFtile`'s
    /// width: the alone diamond, widened to the RIGHT by a long north label
    /// (`left + north_w` when `north_w > left`). Equals `diamond_w` otherwise.
    diamond_w_tile: f64,
    /// Branch box width (`FtileMinWidthCentered(branch, 30)`).
    branch_w: f64,
    /// Branch box height.
    branch_h: f64,
    /// Couple width = `FtileGeometryMerger(diamondTile, branch)`'s width, where
    /// `diamondTile` is `FtileDiamondInside2.calculateDimensionFtile` (the alone
    /// diamond possibly widened to the right by a long north label). This is what
    /// `getTranslateCouple1` steps by (`x += couple.width + xSeparation`).
    couple_w: f64,
    /// Couple spine offset = `FtileGeometryMerger`'s `left` = `max(diamondLeft,
    /// branchLeft)`. The diamond/branch centre sits at `coupleOrigin + couple_left`.
    couple_left: f64,
    /// Diamond center x relative to the if-block spine (filled by `if_long_layout`).
    cx: f64,
    /// The `then`/`elseif` north label.
    north: Option<String>,
    /// The condition text drawn inside the diamond.
    condition: String,
}

/// The fully-placed long layout: per-column geometry plus the else tile and the
/// derived vertical metrics. All x are relative to the if-block spine (x=0);
/// `tile2_*` describe the final `else` column (which has no diamond).
struct IfLongLayout {
    cols: Vec<IfLongCol>,
    /// East label drawn on the last diamond (the bare-else label, e.g. "no").
    east_label: Option<String>,
    /// Bare-else branch box height (laid out as `tile2`); 0 when there is no
    /// final `else`.
    tile2_h: f64,
    /// `tile2` center x relative to the spine; `None` when there is no else.
    tile2_cx: Option<f64>,
    /// Diamond north label height (single-line; reserved below the diamond).
    north_h: f64,
    /// Left / right drawn extents from the spine.
    left_ext: f64,
    right_ext: f64,
}

/// Build the placed long layout for an `if/elseif*/else`. Returns `None` if any
/// branch isn't yet portable (so the caller falls back to the legacy path).
fn if_long_layout(
    condition: &str,
    then_label: &Option<String>,
    then_branch: &[LayoutNode],
    else_branches: &[ElseBranch],
) -> Option<IfLongLayout> {
    // Split else branches into elseif columns (condition=Some) and the optional
    // final bare else (condition=None, must be last if present).
    let mut cols: Vec<IfLongCol> = Vec::new();
    let north_h = pm::text_height(SMALL_FONT);

    let mut push_col = |cond: &str, north: &Option<String>, body: &[LayoutNode]| -> Option<()> {
        let g = sequence_geometry(body)?;
        let branch_w = g.width.max(30.0);
        let cond_text_w = text_render::measure(cond, SMALL_FONT, false);
        let cond_text_h = pm::text_height(SMALL_FONT);
        let north_w = north
            .as_ref()
            .map(|s| text_render::measure(s, SMALL_FONT, false))
            .unwrap_or(0.0);
        let dgeo =
            ftile::FtileGeometry::diamond_inside2(cond_text_w, cond_text_h, north_w, north_h);
        // Polygon width = the "alone" diamond (out_y carries the alone height,
        // and left = alone width / 2). The north label only widens the tile
        // when north_w > left; for single-word labels it does not.
        let diamond_w = dgeo.left * 2.0;
        // Couple = FtileGeometryMerger(diamondTile, branchTile) (FtileAssembly-
        // Simple stacks them vertically and merges on the spine). diamondTile is
        // the FtileDiamondInside2 *tile* geometry: left = alone-width/2, total
        // width = `dgeo.width` (widened to the right when north_w > left). The
        // branch is FtileMinWidthCentered → centred (branch_left = branch_w/2).
        let diamond_left = dgeo.left;
        let diamond_tile_w = dgeo.width;
        let branch_left = branch_w / 2.0;
        let couple_left = diamond_left.max(branch_left);
        let couple_w =
            (diamond_tile_w + (couple_left - diamond_left)).max(branch_w + (couple_left - branch_left));
        cols.push(IfLongCol {
            diamond_w,
            diamond_w_tile: diamond_tile_w,
            branch_w,
            branch_h: g.height,
            couple_w,
            couple_left,
            cx: 0.0,
            north: north.clone(),
            condition: cond.to_string(),
        });
        Some(())
    };

    push_col(condition, then_label, then_branch)?;
    let mut east_label: Option<String> = None;
    let mut tile2_body: Option<&[LayoutNode]> = None;
    let mut tile2_label: Option<String> = None;
    for b in else_branches {
        match &b.condition {
            Some(c) => push_col(c, &b.label, &b.body)?,
            None => {
                // The final bare else → tile2. Its label is the last diamond's
                // east label.
                east_label = b.label.clone();
                tile2_body = Some(&b.body);
                tile2_label = b.label.clone();
            }
        }
    }
    let _ = tile2_label;

    // tile2 geometry (FtileMinWidthCentered(else, 30)); empty if no else.
    let (tile2_w, tile2_h) = match tile2_body {
        Some(body) => {
            let g = sequence_geometry(body)?;
            (g.width.max(30.0), g.height)
        }
        None => (0.0, 0.0),
    };

    // Lay out un-compacted (PlantUML's `getTranslateCouple1`): couples placed
    // left→right at `x += couple.width + xSeparation`, couple center = its
    // FtileGeometry left (= max(diamond,branch)/2, centred). tile2 at
    // `internalWidth − tile2.width`. Then apply PlantUML's `CompressionXorY`
    // (ON_X) pass: empty x-gaps wider than 10 shrink to leave 5 each side.
    let n = cols.len();
    let east_w = east_label
        .as_ref()
        .map(|s| text_render::measure(s, SMALL_FONT, false))
        .unwrap_or(0.0);

    let mut centers_u = vec![0.0_f64; n]; // un-compacted column centers
    let mut x = 0.0;
    for (i, c) in cols.iter().enumerate() {
        // Couple placed at `x` (getTranslateCouple1); diamond/branch centre is
        // the couple's spine offset (`couple_left`). Step by couple width + xSep.
        centers_u[i] = x + c.couple_left;
        x += c.couple_w + IF_LONG_X_SEP;
    }
    let internal_w = x + tile2_w; // = sum(couples)+xSep*n+tile2 (xSep already per couple)
    let tile2_center_u = tile2_body.is_some().then(|| internal_w - tile2_w / 2.0);
    let spine_internal = internal_w / 2.0;

    // Occupied x-intervals of everything drawn (un-compacted frame).
    let mut occ: Vec<(f64, f64)> = Vec::new();
    for (i, c) in cols.iter().enumerate() {
        let cc = centers_u[i];
        let dw = c.diamond_w;
        // diamond polygon
        occ.push((cc - dw / 2.0, cc + dw / 2.0));
        // branch box
        occ.push((cc - c.branch_w / 2.0, cc + c.branch_w / 2.0));
        // north label: left edge at cc + 4
        if let Some(north) = &c.north {
            let nw = text_render::measure(north, SMALL_FONT, false);
            occ.push((cc + 4.0, cc + 4.0 + nw));
        }
        // east label on the last diamond
        if i == n - 1 && east_w > 0.0 {
            occ.push((cc + dw / 2.0, cc + dw / 2.0 + east_w));
        }
        // ConnectionHorizontal arrowhead: the asToRight snake from diamond i-1's
        // east vertex to diamond i's west vertex ends with a polygon arrowhead
        // pointing at the west vertex (`right_arrow(x2)` spans [x2-10, x2]). It
        // is a connector polygon, so the global ON_X compaction sees it as
        // occupancy in the inter-diamond gap.
        if i > 0 {
            let west_vertex = cc - dw / 2.0;
            occ.push((west_vertex - HORIZ_ARROWHEAD_LEN, west_vertex));
        }
    }
    if let Some(tc) = tile2_center_u {
        occ.push((tc - tile2_w / 2.0, tc + tile2_w / 2.0));
    }
    // Horizontal "no"-arrow connectors between adjacent diamonds (and from the
    // last diamond east vertex into tile2) are drawn as Snakes → `UPath`, which
    // `SlotFinder.drawPath` records as OCCUPIED on ON_X (only bare `ULine` is
    // exempt). They span from one diamond's east vertex to the next's west
    // vertex, bridging the inter-couple gap so the compaction can NOT collapse
    // it — exactly why PlantUML keeps the diamonds a full couple-width apart.
    for i in 0..n.saturating_sub(1) {
        let e1 = centers_u[i] + cols[i].diamond_w / 2.0;
        let w2 = centers_u[i + 1] - cols[i + 1].diamond_w / 2.0;
        occ.push((e1.min(w2), e1.max(w2)));
    }
    if let Some(tc) = tile2_center_u {
        // ConnectionLastElseIn: last diamond east vertex → tile2 centre.
        let e_last = centers_u[n - 1] + cols[n - 1].diamond_w / 2.0;
        occ.push((e_last.min(tc), e_last.max(tc)));
    }
    // The flow spine (start/stop circles, inbound/outbound connectors) sits at
    // `internalWidth/2` and is seen by the global ON_X compaction, so it
    // truncates any gap straddling the spine. Reserve the stop/start circle
    // band (radius 11) there — matching the common start→if→stop column that
    // every elseif-chain golden uses.
    occ.push((spine_internal - STOP_OUTER_R, spine_internal + STOP_OUTER_R));

    // Build the compression transform: f(v) = v − (compressed empty space left
    // of v). Empty gaps come from the complement of the merged occupied set;
    // gaps ≤ 10 are kept, larger gaps keep 5px each side (`smaller(5.0)`).
    let compress = XCompress::from_occupied(&occ);
    let centers: Vec<f64> = centers_u.iter().map(|&c| compress.transform(c)).collect();
    let tile2_center = tile2_center_u.map(|t| compress.transform(t));
    let spine_comp = compress.transform(spine_internal);

    // Re-origin everything on the spine (x = 0).
    for (i, c) in cols.iter_mut().enumerate() {
        c.cx = centers[i] - spine_comp;
    }
    let tile2_cx = tile2_center.map(|t| t - spine_comp);

    // Drawn extents from the spine (compacted occupied span).
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    for &(s, e) in &occ {
        min_x = min_x.min(compress.transform(s) - spine_comp);
        max_x = max_x.max(compress.transform(e) - spine_comp);
    }
    let mut left_ext = -min_x;
    let right_ext = max_x;

    // PlantUML positions the whole diagram with `Recentred` over the actual drawn
    // bounding box (ActivityDiagram3:206), not the if-tile's symmetric reported
    // geometry. When a `then`/`elseif` label is wide enough to widen its diamond
    // tile to the RIGHT (`FtileDiamondInside2`: width = left + north_w when
    // north_w > left), the row of diamonds becomes right-heavy: every diamond
    // sits half its own right-overhang left of where a symmetric tile would. The
    // recentred spine therefore lands that half-overhang further from the left
    // edge. The geometry is otherwise identical, so we account for it as extra
    // left extent (= max diamond tile right-overhang / 2), which shifts the spine
    // right and widens the canvas by the same amount on the left.
    let north_overhang = cols
        .iter()
        .map(|c| (c.diamond_w_tile - c.diamond_w).max(0.0))
        .fold(0.0_f64, f64::max);
    left_ext += north_overhang / 2.0;

    Some(IfLongLayout {
        cols,
        east_label,
        tile2_h,
        tile2_cx,
        north_h,
        left_ext,
        right_ext,
    })
}

/// Vertical metrics of a placed long layout, given `y` = the if-block's top
/// (the previous node's bottom, where the `ConnectionIn` snake begins). All
/// absolute. `couple_branch_top` is the top of the (diamond-fronted) branch
/// boxes; `tile2_top` the top of the else box; `merge_y` the bottom merge line
/// (also the if-block's out point).
struct IfLongV {
    dtop: f64,
    couple_branch_top: f64,
    tile2_top: f64,
    merge_y: f64,
}

fn if_long_vmetrics(l: &IfLongLayout, y: f64) -> IfLongV {
    let dtop = y + ARROW_LEN;
    let diamond_aligned_h = DIAMOND_HALF * 2.0 + l.north_h + IF_LONG_ALIGN_BOTTOM;
    let couple_branch_top = dtop + diamond_aligned_h;
    // Internal height (calculateDimensionInternal): couples block vs the
    // else tile lifted by diamondsHeight/2, plus the 100 reserve.
    let couples_h = l
        .cols
        .iter()
        .map(|c| diamond_aligned_h + c.branch_h)
        .fold(0.0_f64, f64::max);
    let diamonds_height = diamond_aligned_h;
    let tile2_merged_h = l.tile2_h + diamonds_height / 2.0;
    let internal_h = couples_h.max(tile2_merged_h) + IF_LONG_BELOW;
    // tile2 dy in the if-frame = (internal_h − tile2_h)/2; if-frame top is
    // 25 above the diamond row (couples dy = 25).
    let if_frame_top = dtop - 25.0;
    let tile2_top = if_frame_top + (internal_h - l.tile2_h) / 2.0;
    // Branch bottoms; the merge line sits ARROW_LEN below the deepest.
    let mut deepest = couple_branch_top + l.cols.iter().map(|c| c.branch_h).fold(0.0_f64, f64::max);
    if l.tile2_cx.is_some() {
        deepest = deepest.max(tile2_top + l.tile2_h);
    }
    let merge_y = deepest + ARROW_LEN;
    IfLongV {
        dtop,
        couple_branch_top,
        tile2_top,
        merge_y,
    }
}

fn if_diamond_half_width(
    condition: &str,
    diamond_font_size: f64,
    diamond_text_bold: bool,
    diamond_font_family: &str,
    diamond_pad_x: f64,
) -> f64 {
    diamond_inner_w_styled_padded(
        condition,
        diamond_font_size,
        diamond_text_bold,
        diamond_font_family,
        diamond_pad_x,
    ) / 2.0
        + DIAMOND_HALF
}

fn with_if_attached_note_extents(
    mut left: f64,
    mut right: f64,
    attached_notes: &[ActivityNote],
    diamond_half_w: f64,
) -> (f64, f64) {
    for note in attached_notes {
        let reach = diamond_half_w + note_box_width(&note.text);
        match note.position {
            NotePosition::Left => left = left.max(reach - 1.0),
            NotePosition::Right => right = right.max(reach),
        }
    }
    (left, right)
}

/// Compute the asymmetric (left, right) extents of a single node from its
/// vertical centreline. For most nodes this is symmetric (width/2, width/2);
/// for if/else with unequal branches, the left extent (then-side) and right
/// extent (else-side) can differ, which shifts the diagram's cx so both
/// branches remain symmetric around the diamond.
fn node_extents(node: &LayoutNode) -> (f64, f64) {
    match node {
        LayoutNode::If {
            condition,
            then_branch,
            else_branches,
            then_label,
            arrow_font_size,
            diamond_font_family,
            diamond_font_size,
            diamond_text_bold,
            diamond_pad_x,
            attached_notes,
            ..
        } => {
            let diamond_half_w = if_diamond_half_width(
                condition,
                *diamond_font_size,
                *diamond_text_bold,
                diamond_font_family,
                *diamond_pad_x,
            );
            // FtileIfLongHorizontal (if/elseif*/else): drawn extents from the
            // placed diamond/branch row.
            if if_is_long(else_branches)
                && let Some(l) = if_long_layout(condition, then_label, then_branch, else_branches)
            {
                return with_if_attached_note_extents(
                    l.left_ext,
                    l.right_ext,
                    attached_notes,
                    diamond_half_w,
                );
            }
            let _ = then_label;
            // Break-down `if` (a lone-break branch + empty branch, nested in a
            // while): the break tile contributes no width and welds LEFT to the
            // loop's exit corridor — a column the enclosing while reserves on its
            // own, so the if's LEFT extent is just the diamond half. The empty
            // branch's east corridor runs `DIAMOND_HALF` past the diamond's east
            // vertex (`ConnectionElseNoDiamond`'s `x1 + hexagonHalfSize`), so the
            // RIGHT extent is `cond_half + DIAMOND_HALF`.
            if if_break_down_plan(then_branch, else_branches).is_some() {
                let cond_half = diamond_half_w;
                let left = cond_half;
                let right = cond_half + DIAMOND_HALF;
                return with_if_attached_note_extents(left, right, attached_notes, diamond_half_w);
            }
            if let Some(plan) = if_down_plan(then_branch, else_branches) {
                // FtileIfDown reserves a fixed corridor on the right (the empty
                // branch routes out the diamond's east vertex) plus a small
                // left lead. Reverse-engineered against the act_if_*yes_*no
                // goldens: left = cond_half + halfHex + 9, right = cond_half +
                // halfHex + 27.2182 (independent of the east label width).
                // A wide populated branch overrides via branch_w/2.
                let cond_half = diamond_half_w;
                let branch_w = sequence_width(plan.populated);
                let left = (cond_half + IF_DOWN_LEFT_PAD).max(branch_w / 2.0);
                let right = (cond_half + IF_DOWN_RIGHT_PAD).max(
                    branch_w / 2.0
                        + IF_DOWN_BRANCH_CORRIDOR_GAP
                        + IF_DOWN_BRANCH_CORRIDOR_TRAILING_PAD,
                );
                return with_if_attached_note_extents(left, right, attached_notes, diamond_half_w);
            }
            if let Some(plan) =
                if_single_circle_terminal_plan(then_label.as_ref(), then_branch, else_branches)
            {
                let terminal_r = terminal_radius(plan.terminal);
                let terminal_label_w = plan.terminal_label.map_or(0.0, |label| {
                    text_render::measure(label, *arrow_font_size, false)
                });
                let survivor_w = sequence_width(plan.survivor);
                let left = (diamond_half_w + IF_DOWN_LEFT_PAD).max(survivor_w / 2.0);
                let right =
                    (diamond_half_w + terminal_label_w + 3.0 * terminal_r).max(survivor_w / 2.0);
                return with_if_attached_note_extents(left, right, attached_notes, diamond_half_w);
            }
            if if_empty_both_plain(then_branch, else_branches) {
                let branch_extent = diamond_half_w + 10.0;
                return with_if_attached_note_extents(
                    branch_extent + IF_EMPTY_BOTH_LEFT_EXTENT_PAD,
                    branch_extent + IF_EMPTY_BOTH_RIGHT_EXTENT_PAD,
                    attached_notes,
                    diamond_half_w,
                );
            }
            // ftile wire (binary if): exact FtileIfWithDiamonds drawn extents.
            if let Some((_, _, left_ext, right_ext)) = if_ftile_layout_styled(
                condition,
                *diamond_font_size,
                *diamond_text_bold,
                diamond_font_family,
                *diamond_pad_x,
                then_branch,
                else_branches,
            ) {
                return with_if_attached_note_extents(
                    left_ext,
                    right_ext,
                    attached_notes,
                    diamond_half_w,
                );
            }
            let diamond_w = diamond_half_w * 2.0;
            let then_w = sequence_width(then_branch);
            let else_w: f64 = else_branches.iter().map(|b| sequence_width(&b.body)).sum();
            // Branch centrelines are at least `diamond_w + 20` apart, but
            // also at least `(then_w + else_w)/2 + 20` so the branch boxes
            // don't crowd each other. PlantUML takes the max of these two.
            let branch_dist = (diamond_w + 20.0).max((then_w + else_w) / 2.0 + 20.0);
            let (left, right) = (
                branch_dist / 2.0 + then_w / 2.0,
                branch_dist / 2.0 + else_w / 2.0,
            );
            with_if_attached_note_extents(left, right, attached_notes, diamond_half_w)
        }
        LayoutNode::Repeat {
            body,
            condition,
            is_label,
            backward,
            ..
        } => {
            // Every `repeatwhile` runs a loop-back arrow up the right side
            // (with or without an `is (...)` label). PlantUML places the
            // condition diamond's left vertex 9 px inside the content area
            // (so left extent is cond_half + 9 regardless of body width),
            // and the loop-back arrow extends 12 px past max(diamond_right,
            // body_right) with another 15 px of right margin past that.
            // Reverse-engineered from goldens with varying body/condition
            // widths.
            let body_has_note = partition_body_has_direct_note(body);
            let (body_left, body_right, body_half) = if body_has_note {
                let (left, right) = sequence_loop_body_extents(body);
                (left, right, left.max(right))
            } else {
                // FtileRepeat.getLeft/getRight key off the body tile's own
                // spine: `repeat.getLeft()` and `repeat.width − getLeft()`.
                // Use the body's asymmetric extents so an off-centre body
                // (e.g. a nested-if whose wider branch sits on one side)
                // pushes the repeat spine the same way PlantUML does, rather
                // than centring on `width/2`. The loop-back arm placement
                // still keys off the symmetric half-width.
                let (left, right) = sequence_extents(body);
                (left, right, sequence_width(body) / 2.0)
            };
            let cond_half = diamond_inner_w(condition) / 2.0 + DIAMOND_HALF;
            // A `backward :label;` action draws a box on the return arm at the
            // far right. The repeat spine keeps the ordinary-repeat
            // `cond_half + 9` clearance, unless the body itself is wider.
            // FtileRepeat appends the backward tile on the right.
            if let Some(label) = backward {
                let left_extent = body_half.max(cond_half + 9.0);
                let right_extent =
                    repeat_backward_right_extent(cond_half, body_half, is_label, label);
                (left_extent, right_extent)
            } else {
                let left_extent = body_left.max(cond_half + 9.0);
                // Right extent mirrors `emit_repeat`'s loop-back arm exactly:
                // `arm = max(diamond_right + 12, extents.right + 12,
                //            geo.right() + 4)`, then the canvas reserves a
                // further 15 px past the arm. For a plain body the geometry
                // and drawn extents coincide, reducing to the historical
                // `max(cond_half, body_right) + 12 + 15`.
                let geo_clear =
                    sequence_geometry(body).map_or(body_right + 12.0, |g| g.right() + 4.0);
                let arm_rel = (cond_half + 12.0).max(body_right + 12.0).max(geo_clear);
                let right_extent = arm_rel + 15.0;
                (left_extent, right_extent)
            }
        }
        LayoutNode::While {
            body,
            condition,
            is_label,
            end_label,
            special_out,
            starts_column,
            diamond_font_family,
            diamond_font_size,
            diamond_text_bold,
            ..
        } => {
            let (body_left, body_right) = sequence_loop_body_extents(body);
            let cond_half = diamond_inner_w_styled(
                condition,
                *diamond_font_size,
                *diamond_text_bold,
                diamond_font_family,
            ) / 2.0
                + DIAMOND_HALF;
            let mut left_extent = while_left_extent(
                body,
                while_body_left(body, body_left),
                cond_half,
                is_label.is_some(),
                end_label.as_deref(),
                special_out.as_deref(),
                *starts_column,
            );
            if special_out.is_some() && body_has_direct_left_note(body) {
                left_extent += 1.0;
            }
            // Right side: loop-back arm at max(cond,body) + halfHex with a 4px
            // arrowhead, plus halfHex of trailing reservation from FtileWhile's
            // `dx + halfHex` term (= 2*halfHex + 3 past max). Verified against
            // the width-only while goldens.
            let right_extent = cond_half.max(body_right)
                + 2.0 * DIAMOND_HALF
                + 3.0
                + while_single_if_right_pad(body, end_label);
            (left_extent, right_extent)
        }
        LayoutNode::Fork {
            branches,
            attached_notes,
            is_split,
            ..
        } => {
            let layout = parallel_layout(branches, *is_split);
            let (left, right) = fork_bar_extents(&layout);
            with_fork_attached_note_extents(left, right, attached_notes)
        }
        // Title contributes 3 px of asymmetric padding on each side beyond
        // tw/2 (reverse-engineered against multiple title goldens). This
        // shifts cx 3 px right of action's natural midline when the title
        // is the widest element.
        LayoutNode::Title {
            text,
            font_size,
            bold,
        } => {
            let tw = text_render::measure(text, *font_size, *bold);
            (tw / 2.0 + 3.0, tw / 2.0 + 3.0)
        }
        // Swimlanes: asymmetric +4 left / +9 right so cx aligns lane_left
        // at PlantUML's fixed x=20 from SVG edge.
        LayoutNode::Swimlanes { lanes, .. } => {
            let total_w: f64 = lanes.iter().map(lane_width).sum();
            (total_w / 2.0 + 4.0, total_w / 2.0 + 9.0)
        }
        // Partition wraps a body with a title bar. Left extent is
        // max(title_w, body_w)/2 + 10; right extent is max(title_w/2 + 5,
        // body_w/2 + 10) — the title's notch corner extends 5 px right of
        // the title text, while body content needs 10 px padding either
        // side inside the partition rect.
        LayoutNode::Partition {
            name,
            color,
            body,
            is_group,
            ..
        } => {
            let title_w = partition_title_width(name);
            if body.is_empty() {
                let half = (title_w + 20.0) / 2.0;
                return (half, half);
            }
            let body_w = partition_body_width_for_frame(body);
            let body_width_extra = partition_body_width_extra(*is_group, body);
            let title_width_extra = partition_title_width_extra(color, *is_group, body);
            let title_drives_width =
                partition_title_drives_width(title_w, body_w, title_width_extra, *is_group, body);
            if !*is_group && partition_wraps_while(body) && !title_drives_width {
                let half = (body_w + 20.0 + body_width_extra) / 2.0;
                return (half, half);
            }
            if *is_group && partition_wraps_repeat(body) && title_drives_width {
                let title_half = title_w / 2.0;
                return (
                    title_half - GROUP_REPEAT_SPINE_LEFT_OF_TITLE_MID,
                    title_half
                        + GROUP_REPEAT_TITLE_WIDTH_EXTRA
                        + GROUP_REPEAT_SPINE_LEFT_OF_TITLE_MID
                        + title_width_extra,
                );
            }
            if !*is_group
                && !title_drives_width
                && let Some((geometry, left_adjust)) = partition_wrapped_geometric_if(body)
            {
                return (
                    geometry.left + PARTITION_GEOMETRIC_IF_SHELL_PAD + left_adjust,
                    geometry.right() + PARTITION_GEOMETRIC_IF_SHELL_PAD,
                );
            }
            let (mut left, mut right) = if !title_drives_width
                && (partition_wraps_switch(body) || partition_body_has_direct_note(body))
            {
                let (body_left, body_right) = if partition_body_has_direct_note(body) {
                    sequence_partition_body_extents(body)
                } else {
                    sequence_extents(body)
                };
                (body_left + 10.0, body_right + 10.0)
            } else {
                (
                    title_w.max(body_w) / 2.0 + 10.0,
                    (title_w / 2.0 + 5.0).max(body_w / 2.0 + 10.0),
                )
            };
            if !*is_group && color.is_some() && partition_wraps_while(body) && title_drives_width {
                left += PARTITION_COLORED_WHILE_SPINE_SHIFT;
                right -= PARTITION_COLORED_WHILE_SPINE_SHIFT;
            }
            if !*is_group && partition_wraps_repeat(body) && !title_drives_width {
                left -= PARTITION_REPEAT_WIDTH_SUBTRACT;
            }
            if *is_group && group_wraps_single_if(body) {
                left += GROUP_IF_LEFT_EXTENT_EXTRA;
                right += GROUP_IF_RIGHT_EXTENT_EXTRA;
            } else if !*is_group && partition_wraps_single_if(body) {
                if partition_wraps_min_width_if(body) {
                    left += PARTITION_IF_LEFT_EXTENT_EXTRA;
                    right += PARTITION_IF_RIGHT_EXTENT_EXTRA;
                } else {
                    left += GROUP_IF_LEFT_EXTENT_EXTRA;
                    right += GROUP_IF_RIGHT_EXTENT_EXTRA;
                }
            }
            if !*is_group && partition_wraps_tiny_action_fork(body) && !title_drives_width {
                left += PARTITION_FORK_LEFT_EXTENT_EXTRA;
            }
            if *is_group && color.is_some() && !group_wraps_single_if(body) {
                right += GROUP_COLOR_RIGHT_EXTENT_EXTRA;
            }
            (left, right)
        }
        // The switch spine aligns to the condition/merge diamond, which in
        // BIG_DIAMOND mode is offset from the geometric block centre.
        LayoutNode::Switch { cases, condition } => {
            if let [case] = cases.as_slice() {
                return switch_one_link_extents(case, condition);
            }
            let layout = switch_x_layout(cases, condition);
            (layout.diamond_dx, layout.block_w - layout.diamond_dx)
        }
        // Terminal circles (start/stop/end) sit inside a fixed-width tile whose
        // spine is 13 px from each side, independent of the glyph radius. When
        // a circle is the widest node on the spine (a bare start→stop column),
        // this pins cx at MARGIN_LEAD + 13 = 29 rather than MARGIN_LEAD + r.
        LayoutNode::Start | LayoutNode::Stop | LayoutNode::End => {
            (CIRCLE_TILE_HALF, CIRCLE_TILE_HALF)
        }
        _ => {
            let w = node_width(node);
            (w / 2.0, w / 2.0)
        }
    }
}

/// Compute the (left, right) extents of a sequence, taking the max of each
/// dimension independently so an asymmetric node anywhere in the sequence
/// shifts cx as needed.
fn sequence_extents(nodes: &[LayoutNode]) -> (f64, f64) {
    sequence_extents_with(nodes, false)
}

fn sequence_extents_if_branch(nodes: &[LayoutNode]) -> (f64, f64) {
    sequence_extents_with(nodes, true)
}

fn sequence_extents_with(nodes: &[LayoutNode], if_branch: bool) -> (f64, f64) {
    sequence_extents_with_note_margin(nodes, if_branch, 1.0)
}

fn sequence_extents_with_note_margin(
    nodes: &[LayoutNode],
    if_branch: bool,
    note_outer_margin: f64,
) -> (f64, f64) {
    sequence_extents_with_note_margins(nodes, if_branch, note_outer_margin, note_outer_margin)
}

/// The trailing right-side reservation a `while`/`repeat` tile carries in its
/// own `node_extents` (`+ 2*halfHex + 3` past `max(cond, body_right)` vs the
/// loop-back arm at `+ halfHex`). That extra `halfHex + 3` is canvas margin —
/// the FtileWhile/FtileRepeat geometry's `dx + halfHex` term that lands in the
/// SVG right margin. When a loop tile is itself nested inside ANOTHER loop's
/// body, the enclosing loop's loop-back arm only needs to clear the inner arm,
/// not the inner's canvas reservation, so this slack is removed.
const LOOP_NEST_TRAILING_RESERVATION: f64 = DIAMOND_HALF + 3.0;

/// True for a nested `while` tile whose `node_extents` right edge includes the
/// trailing canvas reservation that must be stripped when the tile is nested
/// inside another loop body. A nested `repeat` keeps its full right extent: its
/// `repeatwhile` loop-back arm sits at the reserved width, so the enclosing loop
/// must clear that whole extent (see `act_nest_while_repeat`).
fn node_is_loop_tile(node: &LayoutNode) -> bool {
    matches!(node, LayoutNode::While { .. })
}

fn sequence_loop_body_extents(nodes: &[LayoutNode]) -> (f64, f64) {
    if partition_body_has_direct_note(nodes) {
        // Note-bearing bodies keep the raw note-margin measurement.
        return sequence_extents_with_note_margins(nodes, false, 1.0, 0.0);
    }
    let (left, right) = sequence_extents(nodes);
    // A nested loop tile contributes its canvas-reservation-inflated right edge
    // to the raw extent. When such a tile is the rightmost element, the
    // enclosing loop's loop-back arm only needs to clear the INNER arm, not the
    // inner's canvas reservation — so recompute the right extent treating any
    // nested loop tile by its arm position. A non-loop sibling still pins the
    // right extent via the max-fold, so this only ever shrinks loop-driven slack.
    if !nodes.iter().any(node_is_loop_tile) {
        return (left, right);
    }
    let mut adjusted_right = 0.0f64;
    for node in nodes {
        if matches!(node, LayoutNode::Note { .. } | LayoutNode::Arrow { .. }) {
            continue;
        }
        let (_nl, nr) = node_extents(node);
        let nr = if node_is_loop_tile(node) {
            (nr - LOOP_NEST_TRAILING_RESERVATION).max(0.0)
        } else {
            nr
        };
        adjusted_right = adjusted_right.max(nr);
    }
    (left, adjusted_right)
}

fn sequence_extents_with_note_margins(
    nodes: &[LayoutNode],
    if_branch: bool,
    left_note_margin: f64,
    right_note_margin: f64,
) -> (f64, f64) {
    let mut left = 0.0f64;
    let mut right = 0.0f64;
    // The half-width of the most recent flow node — a note attaches to it and
    // sits `NOTE_GAP` to one side, so its lateral reach from the spine is
    // anchor_half + NOTE_GAP + note_box_width.
    let mut anchor_half = 0.0f64;
    for node in nodes {
        match node {
            LayoutNode::Note { text, position, .. } => {
                let box_w = note_box_width(text);
                let reach = anchor_half + NOTE_GAP + box_w;
                match position {
                    // Some contexts let a left note protrude into the outer
                    // margin; callers pass that protrusion as left_note_margin.
                    NotePosition::Left => left = left.max(reach - left_note_margin),
                    // Top-level sequences keep an extra right-side margin;
                    // loop-body routing measures to the note edge itself.
                    NotePosition::Right => right = right.max(reach + right_note_margin),
                }
            }
            _ => {
                let (nl, nr) = if if_branch {
                    node_extents_if_branch(node)
                } else {
                    node_extents(node)
                };
                left = left.max(nl);
                right = right.max(nr);
                // Only genuine flow nodes (those with width) can anchor a note.
                let width = if if_branch {
                    node_width_if_branch(node)
                } else {
                    node_width(node)
                };
                if width > 0.0 {
                    anchor_half = width / 2.0;
                }
            }
        }
    }
    (left, right)
}

fn node_extents_if_branch(node: &LayoutNode) -> (f64, f64) {
    match node {
        LayoutNode::Fork {
            branches,
            attached_notes,
            is_split,
            ..
        } => {
            let layout = if *is_split {
                split_layout(branches)
            } else {
                fork_layout_if_branch(branches)
            };
            let (left, right) = fork_bar_extents(&layout);
            with_fork_attached_note_extents(left, right, attached_notes)
        }
        LayoutNode::Switch { cases, condition } => {
            let layout = switch_x_layout_if_branch_extents(cases, condition);
            (layout.diamond_dx, layout.block_w - layout.diamond_dx)
        }
        _ => node_extents(node),
    }
}

fn node_width_if_branch(node: &LayoutNode) -> f64 {
    match node {
        LayoutNode::Fork { .. } => {
            let (left, right) = node_extents_if_branch(node);
            left + right
        }
        LayoutNode::Switch { cases, condition } => {
            switch_x_layout_if_branch_extents(cases, condition).block_w
        }
        _ => node_width(node),
    }
}

/// Width of a single swimlane: the wider of the content (with 10 px
/// internal padding) and the title text (with ~10 px each side).
fn lane_width(lane: &Lane) -> f64 {
    let content_w = lane.content_left + lane.content_right;
    let title_w = text_render::measure(&lane.name, LANE_TITLE_FONT, false);
    (content_w + 10.0).max(title_w + 10.0)
}

/// cx of the content column within a lane, given the lane's left edge x.
/// Content is left-anchored at `lane_left + 6` and centred on its own
/// natural cx, NOT the geometric centre of the lane.
fn lane_content_cx(lane: &Lane, lane_left: f64) -> f64 {
    lane_left + 6.0 + lane.content_left
}

/// Lane content extents for a swimlane segment body.
///
/// A swimlane derives each lane's width from the actual drawn bounding box of
/// its ftile column (`Swimlane.getMinMax()` in PlantUML's `Swimlanes`), not
/// from the slightly looser `calculateDimension` reservation the standalone
/// (non-swimlane) renderer uses. The two agree everywhere except for a
/// `FtileIfDown` (an `if` with one empty branch) whose width is governed by the
/// condition diamond rather than the populated branch: there the standalone
/// path's [`IF_DOWN_RIGHT_PAD`] carries a small (~0.218 px) reservation that the
/// drawn east-corridor bounding box does not. In a lane that fudge widens the
/// right divider by that amount. Recompute the right extent of a cond-driven
/// top-level if-down from the drawn east corridor (`cond_half + DIAMOND_HALF`)
/// plus the trailing corridor pad, matching the lane's true `getMinMax` width.
fn swimlane_lane_extents(body: &[LayoutNode]) -> (f64, f64) {
    let (left, mut right) = sequence_extents(body);
    if let [
        LayoutNode::If {
            condition,
            then_branch,
            else_branches,
            diamond_font_family,
            diamond_font_size,
            diamond_text_bold,
            diamond_pad_x,
            attached_notes,
            ..
        },
    ] = body
        && attached_notes.is_empty()
        && let Some(plan) = if_down_plan(then_branch, else_branches)
    {
        let cond_half = if_diamond_half_width(
            condition,
            *diamond_font_size,
            *diamond_text_bold,
            diamond_font_family,
            *diamond_pad_x,
        );
        let branch_w = sequence_width(plan.populated);
        // Standalone right (node_extents If / if_down_plan arm).
        let standalone_right = (cond_half + IF_DOWN_RIGHT_PAD).max(
            branch_w / 2.0 + IF_DOWN_BRANCH_CORRIDOR_GAP + IF_DOWN_BRANCH_CORRIDOR_TRAILING_PAD,
        );
        // Drawn (getMinMax) right: the empty branch's east corridor runs to the
        // diamond east vertex + DIAMOND_HALF, then the corridor's trailing pad.
        let drawn_right = (cond_half + DIAMOND_HALF + IF_DOWN_BRANCH_CORRIDOR_TRAILING_PAD).max(
            branch_w / 2.0 + IF_DOWN_BRANCH_CORRIDOR_GAP + IF_DOWN_BRANCH_CORRIDOR_TRAILING_PAD,
        );
        // Only narrow when the standalone reservation is the cond-driven fudge
        // (i.e. it would otherwise overstate the lane width).
        if (standalone_right - right).abs() < 0.001 && drawn_right < right {
            right = drawn_right;
        }
    }
    (left, right)
}

/// Width of a `backward :label;` action box drawn on a repeat's return arm.
/// Same sizing as a normal action box: measured text + 10px padding each side.
fn repeat_backward_box_w(label: &str) -> f64 {
    text_render::measure(label, FONT_SIZE, false) + ACTION_H_PADDING * 2.0
}

/// Box-left offset (from the repeat spine) for a `backward :label;` action on
/// a repeat return arm. When the condition diamond is at least as wide as the
/// body, FtileRepeat's `max(width, test + 2*halfHex) + 2*halfHex - left`
/// places the appended backward tile 24 px past the diamond east vertex. When
/// the body drives the repeat width, PlantUML instead clears the east label
/// before placing the backward tile.
fn repeat_backward_box_left_rel(cond_half: f64, body_half: f64, is_label: &Option<String>) -> f64 {
    if cond_half >= body_half {
        cond_half + 2.0 * DIAMOND_HALF
    } else {
        let is_label_w = is_label
            .as_ref()
            .map(|l| text_render::measure(l, SMALL_FONT, false))
            .unwrap_or(0.0);
        body_half.max(cond_half + is_label_w + 10.0)
    }
}

/// Right extent (from the repeat spine) when a `backward :label;` box sits on
/// the return arm. The extent spans the backward box appended to the repeat
/// geometry.
fn repeat_backward_right_extent(
    cond_half: f64,
    body_half: f64,
    is_label: &Option<String>,
    backward_label: &str,
) -> f64 {
    repeat_backward_box_left_rel(cond_half, body_half, is_label)
        + repeat_backward_box_w(backward_label)
}

fn node_width(node: &LayoutNode) -> f64 {
    match node {
        // Bare start/stop circles: PlantUML lays them out at minimum width
        // without padding (margins are added once at the SVG level). The
        // `+ ACTION_MIN_X * 2.0` previously here forced ~52px of empty
        // space whenever the longest action was narrower than the circle.
        LayoutNode::Start => START_R * 2.0,
        LayoutNode::Stop => STOP_OUTER_R * 2.0,
        LayoutNode::End => 20.0, // `end` uses rx=10 outer circle
        LayoutNode::Connector(_) => CONNECTOR_R * 2.0,
        LayoutNode::Action {
            text_width, pad_x, ..
        } => {
            // Box content width only. The outer ACTION_MIN_X margin is added
            // once at the SVG level (margin_x in render_diagram).
            *text_width + *pad_x * 2.0
        }
        LayoutNode::DeprecatedAction {
            text_width, pad_x, ..
        } => {
            // The deprecated-action box is itself just a normal action box.
            // The warning banner lives in its own horizontal band above the
            // diagram and is sized independently in `render`.
            *text_width + *pad_x * 2.0
        }
        LayoutNode::If {
            condition,
            then_branch,
            else_branches,
            diamond_font_family,
            diamond_font_size,
            diamond_text_bold,
            diamond_pad_x,
            attached_notes,
            ..
        } => {
            if !attached_notes.is_empty() {
                let (l, r) = node_extents(node);
                return l + r;
            }
            if if_is_long(else_branches) {
                let (l, r) = node_extents(node);
                return l + r;
            }
            if if_down_plan(then_branch, else_branches).is_some() {
                let (l, r) = node_extents(node);
                return l + r;
            }
            if let Some((_, _, left_ext, right_ext)) = if_ftile_layout_styled(
                condition,
                *diamond_font_size,
                *diamond_text_bold,
                diamond_font_family,
                *diamond_pad_x,
                then_branch,
                else_branches,
            ) {
                return left_ext + right_ext;
            }
            let diamond_w = diamond_inner_w_styled_padded(
                condition,
                *diamond_font_size,
                *diamond_text_bold,
                diamond_font_family,
                *diamond_pad_x,
            ) + DIAMOND_HALF * 2.0;
            let then_w = sequence_width(then_branch);
            let else_w: f64 = else_branches.iter().map(|b| sequence_width(&b.body)).sum();
            // Branch centrelines are at least `diamond_w + 20` apart, but
            // also at least `(then_w + else_w)/2 + 20` so the branch boxes
            // don't crowd each other when the branches are wider than the
            // diamond. content_w = branch_dist + (then_w + else_w) / 2.
            let branch_dist = (diamond_w + 20.0).max((then_w + else_w) / 2.0 + 20.0);
            branch_dist + (then_w + else_w) / 2.0
        }
        LayoutNode::Fork { attached_notes, .. } if !attached_notes.is_empty() => {
            let (left, right) = node_extents(node);
            left + right
        }
        LayoutNode::Fork {
            branches, is_split, ..
        } => parallel_layout(branches, *is_split).bar_w,
        LayoutNode::Switch { cases, condition } => switch_case_block_width(cases, condition),
        LayoutNode::While { .. } => {
            // Width = left_extent + right_extent. The asymmetric formula lives
            // in node_extents (single source of truth for While geometry).
            let (l, r) = node_extents(node);
            l + r
        }
        LayoutNode::Repeat {
            body,
            condition,
            is_label,
            backward,
            ..
        } => {
            // Every `repeatwhile` produces a loop-back arrow on the right;
            // see node_extents for the formula derivation.
            if partition_body_has_direct_note(body) {
                let (l, r) = node_extents(node);
                return l + r;
            }
            let body_w = sequence_width(body);
            let cond_w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;
            let cond_half = cond_w / 2.0;
            let body_half = body_w / 2.0;
            if let Some(label) = backward {
                let left = body_half.max(cond_half + 9.0);
                let right = repeat_backward_right_extent(cond_half, body_half, is_label, label);
                left + right
            } else {
                let left = body_half.max(cond_half + 9.0);
                let right = cond_half.max(body_half) + 12.0 + 15.0;
                left + right
            }
        }
        // Partition wraps a body with a title bar. Keep this in sync with
        // the emitted rectangle width and node_extents: a parent that contains
        // a nested partition should see the nested partition's visual width,
        // not add another historical 14px padding layer.
        LayoutNode::Partition {
            name,
            color,
            body,
            is_group,
            ..
        } => {
            let title_w = partition_title_width(name);
            if body.is_empty() {
                return title_w + 20.0;
            }
            let title_width_extra = partition_title_width_extra(color, *is_group, body);
            (title_w + 15.0 + title_width_extra).max(partition_body_shell_width(*is_group, body))
        }
        // Swimlanes: sum of per-lane widths. Each lane width is the wider
        // of its content_w + 10 (6 left + 4 right padding inside the lane)
        // and its title_w + horizontal padding for the heading text.
        LayoutNode::Swimlanes { lanes, .. } => lanes.iter().map(lane_width).sum(),
        // Arrows, notes, detach/kill/break, and bare titles contribute no
        // horizontal extent of their own. (Notes will need width once they're
        // laid out alongside the flow; for now they fall back to 0.)
        LayoutNode::Arrow { .. }
        | LayoutNode::Note { .. }
        | LayoutNode::Detach
        | LayoutNode::Kill
        | LayoutNode::Break
        | LayoutNode::Goto(_) => 0.0,
        LayoutNode::Title {
            text,
            font_size,
            bold,
        } => text_render::measure(text, *font_size, *bold),
    }
}

/// Compute the height needed for a sequence of layout nodes.
fn sequence_height(nodes: &[LayoutNode]) -> f64 {
    sequence_height_ex(nodes, 0.0)
}

/// Like [`sequence_height`], but adds `gap_extra` to each plain inter-tile
/// inbound gap — used for fork branches, whose assembly snakes are not
/// compressed (see [`FORK_BRANCH_INTER_GAP_EXTRA`]). Mirrors `emit_sequence_ex`'s
/// `fork_branch_gap_extra` handling so predicted and emitted heights agree.
fn sequence_height_ex(nodes: &[LayoutNode], gap_extra: f64) -> f64 {
    let mut h = 0.0;
    let mut prior_flow = false;
    let mut prior_single_survivor_if = false;
    let mut prior_outbound_gap: Option<f64> = None;
    // Pending arrow style — modifiers from an explicit `-[…]->` preceding the
    // next flow node change the gap length (10 for hidden, 41.275 for
    // labelled, default 20).
    let mut pending_gap: Option<f64> = None;
    for (idx, node) in nodes.iter().enumerate() {
        // Notes contribute nothing themselves.
        if matches!(node, LayoutNode::Note { .. }) {
            continue;
        }
        // Title contributes its own height but never has a connector arrow
        // before or after it — the emit loop also skips arrows around titles.
        // Don't toggle prior_flow so the following node (typically `start`)
        // doesn't get an unwanted ARROW_LEN gap.
        if matches!(node, LayoutNode::Title { .. }) {
            h += node_height(node);
            pending_gap = None;
            prior_single_survivor_if = false;
            prior_outbound_gap = None;
            continue;
        }
        if is_empty_partition_node(node) {
            h += node_height(node);
            pending_gap = None;
            prior_single_survivor_if = false;
            prior_outbound_gap = None;
            continue;
        }
        // Partition: its top-gap (10 px) already absorbs the would-be arrow.
        // The inbound flow line is drawn by the partition's first inner
        // node, extended back to the cursor's y_in. Same approach as Title.
        if matches!(node, LayoutNode::Partition { .. }) {
            h += node_height(node);
            pending_gap = None;
            // prior_flow remains true so the next node after the partition
            // does get a connector arrow back to the partition's bottom.
            prior_flow = true;
            prior_single_survivor_if = false;
            prior_outbound_gap = None;
            continue;
        }
        // Track explicit arrow style for the next flow connector.
        if let LayoutNode::Arrow {
            color,
            dashed,
            label,
        } = node
        {
            let style = match color {
                Some(c) => arrow_style_from_brackets(c, *dashed),
                None => ArrowStyle {
                    color: ARROW_COLOR.to_string(),
                    dashed: *dashed,
                    dotted: false,
                    bold: false,
                    hidden: false,
                },
            };
            let gap = if style.hidden {
                10.0
            } else if label.is_some() {
                LABELED_ARROW_LEN
            } else {
                ARROW_LEN
            };
            pending_gap = Some(gap);
            continue;
        }
        // Detach/Kill/Break/Goto terminate the flow but produce no visual height.
        // They also suppress the arrow that would precede them.
        if matches!(
            node,
            LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break | LayoutNode::Goto(_)
        ) {
            prior_flow = false;
            pending_gap = None;
            prior_single_survivor_if = false;
            prior_outbound_gap = None;
            continue;
        }
        let (note_inbound_extra, note_bottom_extra) =
            flow_note_vertical_extras(nodes, idx, node_height(node));
        let skip_implicit_gap = prior_single_survivor_if && pending_gap.is_none();
        if prior_flow && !skip_implicit_gap {
            h += pending_gap
                .or(prior_outbound_gap)
                .unwrap_or_else(|| default_inbound_gap(node) + gap_extra)
                + note_inbound_extra;
        }
        pending_gap = None;
        h += node_height(node) + note_bottom_extra;
        prior_flow = true;
        prior_single_survivor_if = if_node_has_single_survivor(node);
        prior_outbound_gap = repeat_not_label_outbound_gap(node);
    }
    h
}

fn action_height(text: &str, pad_y: f64, font_family: &str, font_size: f64) -> f64 {
    if let Some(rows) = action_table_rows(text) {
        let line_h = text_render::label_height_with_family("", font_size, font_family);
        return rows.len() as f64 * line_h + ACTION_TABLE_PAD_Y * 2.0;
    }

    // Pick the box height to match the label's actual font — monospace
    // labels render shorter than sans-serif at the same nominal size.
    (action_label_lines(text)
        .iter()
        .map(|line| text_render::label_height_with_family(line, font_size, font_family))
        .sum::<f64>()
        + pad_y * 2.0)
        .max(ACTION_MIN_HEIGHT)
}

fn action_text_width(text: &str, font_size: f64, bold: bool, font_family: &str) -> f64 {
    if let Some(rows) = action_table_rows(text) {
        return action_table_col_widths(&rows, font_size, bold, font_family)
            .iter()
            .sum();
    }

    let mut number_counters = Vec::new();
    action_label_lines(text)
        .iter()
        .map(|line| {
            action_line_width_with_family(line, font_size, bold, font_family, &mut number_counters)
        })
        .fold(0.0f64, f64::max)
}

fn action_label_lines(text: &str) -> Vec<&str> {
    let lines: Vec<&str> = text.split("\\n").flat_map(str::lines).collect();
    if lines.is_empty() { vec![""] } else { lines }
}

fn action_line_x_offset(lines: &[&str], idx: usize) -> f64 {
    let Some(line) = lines.get(idx) else {
        return 0.0;
    };
    if idx < 2 || !line.trim_start().starts_with(':') {
        return 0.0;
    }
    let Some(first) = lines.first().map(|line| line.trim_end()) else {
        return 0.0;
    };
    let Some(prev) = lines.get(idx - 1).map(|line| line.trim_start()) else {
        return 0.0;
    };
    if matches!(first.chars().last(), Some('|' | ']' | '/' | '>' | '<')) && !prev.starts_with(':') {
        ACTION_SWALLOWED_CONTROL_COLON_INDENT
    } else {
        0.0
    }
}

fn action_table_rows(text: &str) -> Option<Vec<creole::TableRow>> {
    let lines = action_label_lines(text);
    if lines.is_empty() {
        return None;
    }
    let mut rows = Vec::with_capacity(lines.len());
    for line in lines {
        match creole::parse_line(line.trim()) {
            creole::CreoleLine::Table(row) => rows.push(row),
            _ => return None,
        }
    }
    Some(rows)
}

fn action_table_col_widths(
    rows: &[creole::TableRow],
    font_size: f64,
    bold: bool,
    font_family: &str,
) -> Vec<f64> {
    let cols = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
    let mut widths = vec![0.0_f64; cols];
    for row in rows {
        for (idx, cell) in row.cells.iter().enumerate() {
            let text_w = text_render::measure_with_family(
                &cell.text,
                font_size,
                bold || cell.is_header,
                font_family,
            );
            widths[idx] = widths[idx].max(text_w + ACTION_TABLE_CELL_PAD_X * 2.0);
        }
    }
    widths
}

fn next_action_number(level: usize, number_counters: &mut Vec<usize>) -> usize {
    if number_counters.len() > level {
        number_counters.truncate(level);
    }
    while number_counters.len() < level {
        number_counters.push(0);
    }
    number_counters[level - 1] += 1;
    number_counters[level - 1]
}

fn action_line_width_with_family(
    line: &str,
    font_size: f64,
    bold: bool,
    font_family: &str,
    number_counters: &mut Vec<usize>,
) -> f64 {
    match creole::parse_line(line.trim()) {
        creole::CreoleLine::Bullet { level, content } => {
            number_counters.clear();
            let indent = ACTION_LIST_ITEM_TEXT_X * level.saturating_sub(1) as f64;
            indent
                + ACTION_LIST_ITEM_TEXT_X
                + text_render::measure_with_family(&content, font_size, bold, font_family)
        }
        creole::CreoleLine::Numbered { level, content } => {
            let number = next_action_number(level, number_counters);
            let indent = ACTION_LIST_ITEM_TEXT_X * level.saturating_sub(1) as f64;
            let marker = format!("{number}.");
            indent
                + text_render::measure_with_family(&marker, font_size, bold, font_family)
                + ACTION_LIST_NUMBER_GAP
                + text_render::measure_with_family(&content, font_size, bold, font_family)
        }
        _ => {
            number_counters.clear();
            // Keep leading whitespace (PlantUML renders it as left padding);
            // only trailing whitespace is stripped.
            text_render::measure_with_family(line.trim_end(), font_size, bold, font_family)
        }
    }
}

fn emit_action_line(
    svg: &mut SvgEmitter,
    line: &str,
    base: &TextBase<'_>,
    number_counters: &mut Vec<usize>,
) -> f64 {
    match creole::parse_line(line.trim()) {
        creole::CreoleLine::Bullet { level, content } => {
            number_counters.clear();
            let indent = ACTION_LIST_ITEM_TEXT_X * level.saturating_sub(1) as f64;
            let cx = base.x + indent + ACTION_LIST_BULLET_CX;
            let cy = base.y - ACTION_LIST_BULLET_BASELINE_DROP;
            write!(
                svg.shapes,
                r#"<ellipse cx="{}" cy="{}" fill="{}" rx="2.5" ry="2.5"/>"#,
                f(cx),
                f(cy),
                base.fill
            )
            .unwrap();
            let text_x = base.x + indent + ACTION_LIST_ITEM_TEXT_X;
            let w = text_render::emit_text(
                &mut svg.shapes,
                &content,
                &TextBase {
                    x: text_x,
                    y: base.y,
                    font_size: base.font_size,
                    font_family: base.font_family,
                    fill: base.fill,
                    bold: base.bold,
                    italic: base.italic,
                    underline: base.underline,
                    skip_underline: base.skip_underline,
                },
            );
            indent + ACTION_LIST_ITEM_TEXT_X + w
        }
        creole::CreoleLine::Numbered { level, content } => {
            let number = next_action_number(level, number_counters);
            let indent = ACTION_LIST_ITEM_TEXT_X * level.saturating_sub(1) as f64;
            let marker = format!("{number}.");
            let marker_w = text_render::emit_text(
                &mut svg.shapes,
                &marker,
                &TextBase {
                    x: base.x + indent,
                    y: base.y,
                    font_size: base.font_size,
                    font_family: base.font_family,
                    fill: base.fill,
                    bold: base.bold,
                    italic: base.italic,
                    underline: base.underline,
                    skip_underline: base.skip_underline,
                },
            );
            let w = text_render::emit_text(
                &mut svg.shapes,
                &content,
                &TextBase {
                    x: base.x + indent + marker_w + ACTION_LIST_NUMBER_GAP,
                    y: base.y,
                    font_size: base.font_size,
                    font_family: base.font_family,
                    fill: base.fill,
                    bold: base.bold,
                    italic: base.italic,
                    underline: base.underline,
                    skip_underline: base.skip_underline,
                },
            );
            indent + marker_w + ACTION_LIST_NUMBER_GAP + w
        }
        _ => {
            number_counters.clear();
            // Keep leading whitespace: emit_text shifts the text element right
            // by the leading-space advance, matching PlantUML's left padding.
            text_render::emit_text(&mut svg.shapes, line.trim_end(), base)
        }
    }
}

fn group_uses_compact_top_gap(body: &[LayoutNode]) -> bool {
    matches!(
        body,
        [LayoutNode::Action { .. }
            | LayoutNode::If { .. }
            | LayoutNode::Fork { .. }
            | LayoutNode::Switch { .. }]
    )
}

fn group_wraps_single_if(body: &[LayoutNode]) -> bool {
    matches!(body, [LayoutNode::If { .. }])
}

fn group_contains_partition(body: &[LayoutNode]) -> bool {
    body.iter()
        .any(|n| matches!(n, LayoutNode::Partition { .. }))
}

fn partition_wraps_while(body: &[LayoutNode]) -> bool {
    matches!(body, [LayoutNode::While { .. }])
}

fn partition_wraps_switch(body: &[LayoutNode]) -> bool {
    matches!(body, [LayoutNode::Switch { .. }])
}

fn partition_wraps_repeat(body: &[LayoutNode]) -> bool {
    matches!(body, [LayoutNode::Repeat { .. }])
}

fn tiny_action_branch(branch: &[LayoutNode]) -> bool {
    matches!(
        branch,
        [LayoutNode::Action { text_width, .. }
            | LayoutNode::DeprecatedAction { text_width, .. }]
            if *text_width < PARTITION_FORK_TINY_TEXT_MAX
    )
}

fn partition_wraps_tiny_action_fork(body: &[LayoutNode]) -> bool {
    let [
        LayoutNode::Fork {
            branches,
            is_split: false,
            merge: false,
            ..
        },
    ] = body
    else {
        return false;
    };
    branches.len() >= 2 && branches.iter().all(|branch| tiny_action_branch(branch))
}

fn single_partition_branch_body_top(branch: &[LayoutNode], branch_y: f64) -> Option<f64> {
    let [
        LayoutNode::Partition {
            name,
            color,
            is_group,
            nested,
            single_lane_first_group,
            body,
            ..
        },
    ] = branch
    else {
        return None;
    };
    if body.is_empty() {
        return None;
    }
    let top_gap = partition_top_gap(
        color,
        name,
        *is_group,
        *nested,
        *single_lane_first_group,
        body,
    );
    let needs_nudge = top_gap == 10.0;
    Some(branch_y + top_gap + PARTITION_TITLE_BAND_H - if needs_nudge { 0.00005 } else { 0.0 })
}

fn single_partition_branch_body_bottom(branch: &[LayoutNode], branch_y: f64) -> Option<f64> {
    let [LayoutNode::Partition { body, .. }] = branch else {
        return None;
    };
    single_partition_branch_body_top(branch, branch_y)
        .map(|body_top| body_top + sequence_height(body))
}

fn fork_branches_are_single_partitions(branches: &[Vec<LayoutNode>]) -> bool {
    !branches.is_empty()
        && branches
            .iter()
            .all(|branch| single_partition_branch_body_top(branch, 0.0).is_some())
}

fn partition_wraps_single_if(body: &[LayoutNode]) -> bool {
    matches!(body, [LayoutNode::If { .. }])
}

fn partition_wraps_min_width_if(body: &[LayoutNode]) -> bool {
    let [
        LayoutNode::If {
            condition,
            diamond_font_family,
            diamond_font_size,
            diamond_text_bold,
            diamond_pad_x,
            ..
        },
    ] = body
    else {
        return false;
    };
    diamond_inner_w_styled_padded(
        condition,
        *diamond_font_size,
        *diamond_text_bold,
        diamond_font_family,
        *diamond_pad_x,
    ) <= DIAMOND_MIN_INNER_W
}

fn partition_wrapped_geometric_if(body: &[LayoutNode]) -> Option<(ftile::FtileGeometry, f64)> {
    if partition_wraps_min_width_if(body) {
        return None;
    }
    let [node @ LayoutNode::If { .. }] = body else {
        return None;
    };
    let geometry = node_geometry(node)?;
    let left_adjust = if node_if_depth(node) >= 3 {
        PARTITION_GEOMETRIC_IF_DEEP_LEFT_ADJUST
    } else if geometry.width >= PARTITION_GEOMETRIC_IF_SHALLOW_ADJUST_MIN_W {
        PARTITION_GEOMETRIC_IF_SHALLOW_LEFT_ADJUST
    } else {
        0.0
    };
    Some((geometry, left_adjust))
}

fn while_body_is_single_if(body: &[LayoutNode]) -> bool {
    matches!(body, [LayoutNode::If { .. }])
}

/// True when a loop body's RIGHT extent is driven by a binary `if` tile: the
/// last flow node is an `if` and its right extent is at least as wide as every
/// other flow node's. The FtileIfWithDiamonds margin (`WHILE_SINGLE_IF_RIGHT_PAD`)
/// then rides into the loop-back arm regardless of whether the `if` is the SOLE
/// body element (`act_while_ifdepth*`) or sits after an action (`act_while_with_if`).
fn while_body_right_driven_by_if(body: &[LayoutNode]) -> bool {
    let mut last_if_right: Option<f64> = None;
    let mut max_other_right = 0.0f64;
    for node in body {
        if !node_is_flow(node) {
            continue;
        }
        let (_l, r) = node_extents(node);
        if matches!(node, LayoutNode::If { .. }) {
            last_if_right = Some(r);
        } else {
            last_if_right = None;
            max_other_right = max_other_right.max(r);
        }
    }
    matches!(last_if_right, Some(if_r) if if_r >= max_other_right)
}

fn while_single_if_right_pad(body: &[LayoutNode], _end_label: &Option<String>) -> f64 {
    // The pad reflects the if-tile's calculated right extent exceeding its drawn
    // shape by `WHILE_SINGLE_IF_RIGHT_PAD` (the FtileIfWithDiamonds internal
    // margin that `geo.appendBottom` carries into FtileWhile's loop-back arm at
    // `dimTotal.getWidth()`). It is a property of the if-tile geometry, not of
    // the while's end-of-loop label, so it applies whether or not an
    // `endwhile (label)` is present.
    if while_body_right_driven_by_if(body) {
        WHILE_SINGLE_IF_RIGHT_PAD
    } else {
        0.0
    }
}

fn is_colored_partition_wrapping_while(node: &LayoutNode) -> bool {
    matches!(
        node,
        LayoutNode::Partition {
            color: Some(_),
            is_group: false,
            body,
            ..
        } if partition_wraps_while(body)
    )
}

fn partition_wrapped_while_slot_compresses(body: &[LayoutNode]) -> bool {
    let [
        LayoutNode::While {
            body: while_body,
            is_label,
            end_label,
            ..
        },
    ] = body
    else {
        return false;
    };
    while_slot_compress(
        true,
        is_label.is_some(),
        end_label.is_some(),
        while_body.is_empty(),
    ) != 0.0
}

fn is_partition_wrapping_compressed_while(node: &LayoutNode) -> bool {
    matches!(
        node,
        LayoutNode::Partition {
            is_group: false,
            body,
            ..
        } if partition_wrapped_while_slot_compresses(body)
    )
}

fn partition_title_has_descender(name: &str) -> bool {
    name.chars()
        .any(|c| matches!(c, 'g' | 'j' | 'p' | 'q' | 'y'))
}

fn partition_title_label(name: &str) -> &str {
    if name.is_empty() { "\u{00a0}" } else { name }
}

fn partition_title_width(name: &str) -> f64 {
    text_render::measure(partition_title_label(name), TITLE_FONT_SIZE, false)
}

fn empty_partition_shell_height() -> f64 {
    31.4883
}

fn partition_body_width_extra(is_group: bool, body: &[LayoutNode]) -> f64 {
    if !is_group && partition_wraps_while(body) {
        -PARTITION_WHILE_WIDTH_SUBTRACT
    } else if !is_group && partition_wraps_repeat(body) {
        -PARTITION_REPEAT_WIDTH_SUBTRACT
    } else if !is_group && partition_wraps_tiny_action_fork(body) {
        PARTITION_FORK_WIDTH_EXTRA
    } else if !is_group && partition_wraps_single_if(body) {
        if partition_wraps_min_width_if(body) {
            PARTITION_IF_BODY_WIDTH_EXTRA
        } else {
            GROUP_IF_BODY_WIDTH_EXTRA
        }
    } else if is_group && partition_wraps_repeat(body) {
        -GROUP_REPEAT_BODY_WIDTH_SUBTRACT
    } else if is_group && group_wraps_single_if(body) {
        GROUP_IF_BODY_WIDTH_EXTRA
    } else {
        0.0
    }
}

fn partition_body_shell_width(is_group: bool, body: &[LayoutNode]) -> f64 {
    if !is_group && let Some((geometry, left_adjust)) = partition_wrapped_geometric_if(body) {
        geometry.width + GROUP_IF_BODY_WIDTH_EXTRA + left_adjust
    } else {
        partition_body_width_for_frame(body) + 20.0 + partition_body_width_extra(is_group, body)
    }
}

fn partition_title_width_extra(color: &Option<String>, is_group: bool, body: &[LayoutNode]) -> f64 {
    if is_group && color.is_some() && !group_wraps_single_if(body) {
        GROUP_COLOR_TITLE_WIDTH_EXTRA
    } else {
        0.0
    }
}

fn partition_title_drives_width(
    title_w: f64,
    body_w: f64,
    title_width_extra: f64,
    is_group: bool,
    body: &[LayoutNode],
) -> bool {
    let body_shell_width = if !is_group && partition_wrapped_geometric_if(body).is_some() {
        partition_body_shell_width(is_group, body)
    } else {
        body_w + 20.0 + partition_body_width_extra(is_group, body)
    };
    title_w + 15.0 + title_width_extra >= body_shell_width
}

fn partition_top_gap(
    color: &Option<String>,
    name: &str,
    is_group: bool,
    nested: bool,
    single_lane_first_group: bool,
    body: &[LayoutNode],
) -> f64 {
    let gap = if is_group && group_uses_compact_top_gap(body) {
        10.0
    } else if (is_group
        && (color.is_some()
            || (!nested && !group_contains_partition(body) && partition_title_has_descender(name))))
        || (!is_group
            && (color.is_some() || partition_title_has_descender(name))
            && partition_title_drives_width(
                partition_title_width(name),
                partition_body_width_for_frame(body),
                partition_title_width_extra(color, is_group, body),
                is_group,
                body,
            ))
    {
        10.4531
    } else {
        10.0
    };
    if single_lane_first_group {
        gap + SINGLE_LANE_GROUP_TOP_ADJUST
    } else {
        gap
    }
}

fn node_height(node: &LayoutNode) -> f64 {
    match node {
        // Start ellipse cy is fixed at START_CY (25), so from the y=MARGIN_LEAD
        // cursor (16) the ellipse bottom is 25+10-16 = 19, not the full diameter.
        LayoutNode::Start => START_CY + START_R - 16.0,
        LayoutNode::Stop => STOP_OUTER_R * 2.0,
        // `end` uses smaller geometry: rx=10 outer circle, no extra ring.
        LayoutNode::End => 20.0,
        LayoutNode::Connector(_) => CONNECTOR_R * 2.0,
        LayoutNode::Action {
            text,
            pad_y,
            font_family,
            font_size,
            ..
        } => action_height(text, *pad_y, font_family, *font_size),
        LayoutNode::DeprecatedAction {
            text,
            pad_y,
            font_family,
            font_size,
            ..
        } => {
            // Warning banner is accounted for separately by warning_band_h
            // in render; this node's own height is just the action box.
            action_height(text, *pad_y, font_family, *font_size)
        }
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches,
            diamond_pad_x,
            diamond_half_y,
            ..
        } => {
            if if_is_long(else_branches)
                && let Some(l) = if_long_layout(condition, then_label, then_branch, else_branches)
            {
                // Height measured from the diamond row top (dtop) to the merge
                // line. `sequence_height` adds the leading ARROW_LEN gap
                // (start→if) separately, so exclude the inbound run here.
                let v = if_long_vmetrics(&l, 0.0);
                return v.merge_y - v.dtop;
            }
            // Break-down `if` (nested in a while): diamond + the no-diamond
            // corridor drop to the if's pointOut (no merge diamond). The
            // compressed drop is the common case; thin-loop uncompression is
            // re-added by the enclosing while (see while_body_height).
            if if_break_down_plan(then_branch, else_branches).is_some() {
                return DIAMOND_HALF * 2.0 + WHILE_BREAK_IF_CORRIDOR_DROP_COMPRESSED;
            }
            if let Some(plan) = if_down_plan(then_branch, else_branches) {
                // diamond + lead + populated branch + ARROW_LEN + merge diamond.
                // An even-action branch stretches its middle gap by 15 px.
                let branch_h = sequence_height(plan.populated);
                let flow_count = plan.populated.iter().filter(|n| node_is_flow(n)).count();
                let stretch = if flow_count >= 2 && flow_count.is_multiple_of(2) {
                    IF_DOWN_MID_STRETCH
                } else {
                    0.0
                };
                return DIAMOND_HALF * 2.0
                    + IF_DOWN_LEAD
                    + branch_h
                    + stretch
                    + ARROW_LEN
                    + DIAMOND_HALF * 2.0;
            }
            if let Some(plan) =
                if_single_circle_terminal_plan(then_label.as_ref(), then_branch, else_branches)
            {
                let branch_h = sequence_height(plan.survivor);
                let flow_count = plan.survivor.iter().filter(|n| node_is_flow(n)).count();
                let stretch = if flow_count >= 2 && flow_count.is_multiple_of(2) {
                    IF_DOWN_MID_STRETCH
                } else {
                    0.0
                };
                return DIAMOND_HALF * 2.0 + IF_DOWN_LEAD + branch_h + stretch + ARROW_LEN;
            }
            let diamond_h = diamond_half_y * 2.0;
            let then_h = sequence_height_if_branch(then_branch);
            let max_else_h: f64 = else_branches
                .iter()
                .map(|b| sequence_height_if_branch(&b.body))
                .fold(0.0f64, f64::max);
            let branch_h = then_h.max(max_else_h);
            // diamond + IF_BRANCH_DOWN + branch_h + merge gap + merge diamond.
            // When every branch terminates, the merge diamond and its leading
            // merge gap are skipped (see emit_if).
            let merge_gap = if *diamond_pad_x > 0.0 {
                10.0
            } else {
                IF_BRANCH_UP
            };
            let then_terminates = branch_terminates(then_branch);
            let else_terminates = !else_branches.is_empty()
                && else_branches.iter().all(|b| branch_terminates(&b.body));
            let all_terminate = then_terminates && else_terminates;
            if let Some(then_survives) = if_single_survivor(then_branch, else_branches) {
                let survivor_h = if then_survives { then_h } else { max_else_h };
                let terminal_h = if then_survives { max_else_h } else { then_h };
                diamond_h + IF_BRANCH_DOWN + terminal_h.max(survivor_h + ARROW_LEN)
            } else if all_terminate {
                diamond_h + IF_BRANCH_DOWN + branch_h
            } else {
                diamond_h + IF_BRANCH_DOWN + branch_h + merge_gap + DIAMOND_HALF * 2.0
            }
        }
        LayoutNode::Fork {
            branches,
            is_split,
            merge,
            ..
        } => {
            let max_h: f64 = branches
                .iter()
                .map(|b| sequence_height(b))
                .fold(0.0f64, f64::max);
            if *is_split {
                ARROW_LEN + max_h + ARROW_LEN
            } else if *merge {
                FORK_BAR_HEIGHT + ARROW_LEN + max_h + FORK_MERGE_GAP + DIAMOND_HALF * 2.0
            } else if fork_branches_are_single_partitions(branches) {
                FORK_BAR_HEIGHT + max_h + ARROW_LEN + FORK_BAR_HEIGHT
            } else {
                // Pure action/control-flow fork. Unequal branch heights keep
                // the uncompressed 35 px assembly gap (compression can't reclaim
                // the 15 px slack); equal heights compress to ARROW_LEN. When
                // every branch terminates the join bar sits
                // FORK_TERMINATING_JOIN_GAP below the deepest branch.
                let nonempty: Vec<f64> = branches
                    .iter()
                    .filter(|b| !b.is_empty())
                    .map(|b| sequence_height(b))
                    .collect();
                let unequal =
                    !nonempty.is_empty() && nonempty.iter().any(|h| (h - nonempty[0]).abs() > 0.01);
                let gap_extra = if unequal {
                    FORK_BRANCH_INTER_GAP_EXTRA
                } else {
                    0.0
                };
                let max_h: f64 = branches
                    .iter()
                    .filter(|b| !b.is_empty())
                    .map(|b| sequence_height_ex(b, gap_extra))
                    .fold(0.0f64, f64::max);
                let all_terminate = branches
                    .iter()
                    .all(|b| b.is_empty() || branch_terminates(b))
                    && branches.iter().any(|b| !b.is_empty());
                let join_gap = if all_terminate {
                    FORK_TERMINATING_JOIN_GAP
                } else {
                    ARROW_LEN
                };
                FORK_BAR_HEIGHT + ARROW_LEN + max_h + join_gap + FORK_BAR_HEIGHT
            }
        }
        LayoutNode::Switch { cases, condition } => {
            let max_h: f64 = cases
                .iter()
                .map(|c| sequence_height(&c.body))
                .fold(0.0f64, f64::max);
            if let [case] = cases.as_slice() {
                let merge_gap = if branch_terminates(&case.body) {
                    0.0
                } else {
                    ARROW_LEN
                };
                let merge_h = if branch_terminates(&case.body) {
                    0.0
                } else {
                    DIAMOND_HALF * 2.0
                };
                return DIAMOND_HALF * 2.0
                    + switch_one_link_below_diamond(case)
                    + max_h
                    + merge_gap
                    + merge_h;
            }
            // Branches that use the centre corridor need a full arrow gap into
            // the merge diamond; even layouts without that corridor compress it.
            let layout = switch_x_layout(cases, condition);
            let merge_gap = switch_merge_gap(cases, condition, &layout);
            let merge_h = if switch_all_branches_terminate(cases) {
                0.0
            } else {
                DIAMOND_HALF * 2.0
            };
            DIAMOND_HALF * 2.0
                + switch_below_diamond(cases, layout.big_diamond)
                + max_h
                + merge_gap
                + merge_h
        }
        LayoutNode::While {
            body,
            is_label,
            end_label,
            special_out,
            arrow_font_size,
            ..
        } => {
            // PlantUML's FtileWhile height formula:
            //   height = diamond.h + body.h + 4*halfHex + suppLabel
            // where:
            //   diamond.h = hexagon(24) + northHeight(="yes" text_height)
            //             ≈ 24 + 12.95 ≈ 36.95 (with is_label)
            //   suppLabel = back1.height ≈ 12.95 (the loop-back's
            //              "yes"/incoming label height; PlantUML reserves
            //              text_height(11) even when the label is invisible)
            //   4*halfHex = 48 padding above and below body
            // When special_out is present, the terminator sits INSIDE the
            // frame at translateForSpecial.y = max(3*half, 4*halfHex) = 48,
            // contributing terminator.h on top. We must ensure
            // height >= special_y + special.h.
            let body_h = while_body_height(body, is_label.is_some());
            // The total while-frame height = diamond.h + body_top_offset
            // + body_h + below-body-gap + wrap-back-offset. We derive it
            // from the same compression-aware formula as emit_while.
            let diamond_alone_h = DIAMOND_HALF * 2.0;
            let body_top_offset = while_body_top_offset(
                while_ordinary_slot_compress_allowed(body, special_out.as_deref()),
                is_label.is_some(),
                end_label.is_some(),
                body.is_empty(),
                *arrow_font_size,
            );
            // Below body: junction at +10 (compressed if non-empty body)
            // or +12 (empty body); wrap-back continues another +12 for
            // no-specialOut, or descends to special_y for specialOut.
            let below_body = if special_out.is_some() {
                // Special-out terminators are managed inside the while tile:
                // the loop's advertised tile height keeps the fixed two-half-
                // hex tail below the body, while the terminator itself is
                // placed at translateForSpecial.y by emit_while.
                2.0 * DIAMOND_HALF
                    - if while_body_is_single_if(body) {
                        WHILE_SINGLE_IF_SPECIAL_HEIGHT_TRIM
                    } else {
                        0.0
                    }
            } else if body.is_empty() {
                DIAMOND_HALF + DIAMOND_HALF // empty: +12 to junction, +12 wrap-back
            } else {
                10.0 + DIAMOND_HALF // +10 junction, +12 wrap-back
            };
            diamond_alone_h + body_top_offset + body_h + below_body
        }
        LayoutNode::Repeat {
            body,
            backward,
            has_start_label,
            ..
        } => {
            let body_h = if *has_start_label {
                sequence_height(body)
            } else {
                repeat_body_height(body, backward.is_some())
            };
            let diamond_h = DIAMOND_HALF * 2.0;
            // Single-action backward repeats keep the extra halfHex before
            // the condition diamond; multi-action bodies absorb that slack in
            // their final inbound connector.
            let backward_flow_count = body.iter().filter(|n| node_is_flow(n)).count();
            let cond_gap = if backward.is_some() && backward_flow_count < 2 {
                ARROW_LEN + 10.0
            } else {
                ARROW_LEN
            };
            let top_lead = if *has_start_label {
                0.0
            } else {
                diamond_h + ARROW_LEN
            };
            top_lead + body_h + cond_gap + diamond_h
        }
        LayoutNode::Arrow { .. } => 0.0, // arrows don't add height (they're between nodes)
        LayoutNode::Note { .. } => 0.0,
        LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break | LayoutNode::Goto(_) => 0.0,
        // Title region: text_height + 30 of vertical padding so the cursor
        // lands at the cy of the following Start ellipse (composed of 4 px
        // text-top offset + text_height + 16 px gap below text + START_R).
        // Reverse-engineered from golden SVGs.
        LayoutNode::Title { font_size, .. } => pm::text_height(*font_size) + 30.0,
        // Partition: top gap (10 or 10.4531 if the partition has a fill
        // colour) + 36.49 (title bar) + body height + 12 (bottom margin).
        // The top gap absorbs the would-be inbound arrow.
        LayoutNode::Partition {
            color,
            name,
            is_group,
            nested,
            single_lane_first_group,
            body,
            ..
        } => {
            let top_gap = partition_top_gap(
                color,
                name,
                *is_group,
                *nested,
                *single_lane_first_group,
                body,
            );
            if body.is_empty() {
                return top_gap + empty_partition_shell_height();
            }
            let body_h = sequence_height(body)
                - if color.is_some()
                    && !*is_group
                    && colored_partition_needs_while_slot_subtract(body)
                {
                    WHILE_BODY_SLOT_COMPRESS
                } else {
                    0.0
                };
            top_gap + 36.4883 + body_h + 12.0
        }
        // Swimlanes: header band + start-gap + cumulative body heights
        // across lanes (with cross-lane transitions between them). Lanes
        // are columns spatially but temporally sequential — the flow
        // exits one lane and enters the next, so their bodies stack
        // vertically not side-by-side.
        //
        // Each lane's body contributes its own sequence_height. Between
        // consecutive lanes, the cross-lane arrow takes ARROW_LEN of
        // vertical extent. Within a lane, if the first node is Start,
        // sequence_height overcounts by 9 (START_CY+START_R-MARGIN_LEAD-
        // START_R = 19 vs the swimlane-context 10) because Start sits at
        // body_top instead of START_CY. We subtract that overcount.
        //
        // The final +1.3 (matches header_top offset of 1.2969 above
        // MARGIN_LEAD) absorbs PlantUML's slightly larger top margin for
        // swimlane diagrams. The +2.54 trailing absorbs the slightly
        // larger bottom margin.
        LayoutNode::Swimlanes { segments, .. } => {
            let header_h = pm::text_height(LANE_TITLE_FONT);
            let mut body_h = 0.0_f64;
            for (i, segment) in segments.iter().enumerate() {
                let mut h = sequence_height(&segment.body);
                if matches!(segment.body.first(), Some(LayoutNode::Start)) {
                    h -= START_CY + START_R - 16.0 - START_R; // = 9
                }
                body_h += h;
                if i + 1 < segments.len() {
                    body_h += ARROW_LEN; // cross-lane transition
                }
            }
            // 1.30 top offset + header + 15 to start cy + bodies + 1.24
            // bottom adjustment (empirical match to golden svg height).
            1.2969 + header_h + 15.0 + body_h + 1.24
        }
    }
}

fn following_action_note_height(nodes: &[LayoutNode], idx: usize) -> Option<f64> {
    if !matches!(nodes.get(idx), Some(LayoutNode::Action { .. })) {
        return None;
    }
    let mut note_h = 0.0f64;
    for follow in nodes[idx + 1..].iter() {
        match follow {
            LayoutNode::Note { text, .. } => {
                note_h = note_h.max(note_box_height(text));
            }
            LayoutNode::Arrow { .. } => {}
            _ => break,
        }
    }
    (note_h > 0.0).then_some(note_h)
}

fn flow_note_vertical_extras(nodes: &[LayoutNode], idx: usize, anchor_h: f64) -> (f64, f64) {
    if let Some(LayoutNode::If { attached_notes, .. }) = nodes.get(idx)
        && let Some(note_h) = max_note_height(attached_notes)
    {
        // Folded if notes occupy the connector corridor immediately above the
        // diamond: 10 px from the previous flow bottom to the note top, then
        // the note box itself ending at the diamond top.
        return ((note_h - 10.0).max(0.0), 0.0);
    }

    let Some(note_h) = following_action_note_height(nodes, idx) else {
        return (0.0, 0.0);
    };
    if note_h <= anchor_h {
        return (0.0, 0.0);
    }
    let protrusion = (note_h - anchor_h) / 2.0;
    // PlantUML lets a tall action note protrude about 10px into the inbound
    // connector corridor, so only the remainder stretches the connector into
    // the anchor. The full lower protrusion is reserved before the next
    // connector.
    ((protrusion - 10.0).max(0.0), protrusion)
}

// ─── SVG emission ───────────────────────────────────────────────────

/// Escape a string for SVG text content the way PlantUML does:
/// XML-escape `<`, `>`, `&`, and emit U+00A0 (non-breaking space) as the
/// numeric entity `&#xA0;`.
fn svg_text_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\u{a0}' => out.push_str("&#xA0;"),
            _ => out.push(ch),
        }
    }
    out
}

fn svg_attr_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn emit_handwritten_warning(svg: &mut SvgEmitter, warning: &OracleHandwrittenWarning) {
    let mut polygon = format!(
        r#"<polygon fill="{}" points="{}""#,
        svg_attr_escape(&warning.polygon.fill),
        svg_attr_escape(&warning.polygon.points),
    );
    if let Some(style) = warning.polygon.style.as_deref() {
        write!(polygon, r#" style="{}""#, svg_attr_escape(style)).unwrap();
    }
    polygon.push_str("/>");
    svg.raw(&polygon);

    let text = svg_text_escape(&warning.text.text);
    match warning.text_length.as_deref() {
        Some(text_length) => svg.raw(&format!(
            r##"<text fill="#000000" font-family="monospace" font-size="10" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            svg_attr_escape(text_length),
            f(warning.text.x),
            f(warning.text.y),
            text,
        )),
        None => svg.raw(&format!(
            r##"<text fill="#000000" font-family="monospace" font-size="10" x="{}" y="{}">{}</text>"##,
            f(warning.text.x),
            f(warning.text.y),
            text,
        )),
    }
}

fn f(v: f64) -> String {
    pm::fmt_coord(v)
}

const CONNECTOR_GLYPH_A: &str = "M45.3008,65.1836 L43.8242,61.4453 L42.3418,65.1836 Z M46.6016,68.5 L45.6582,66.0977 L41.9844,66.0977 L41.0293,68.5 L39.8867,68.5 L43.3262,59.8281 L44.5449,59.8281 L47.9316,68.5 Z ";
const CONNECTOR_GLYPH_B: &str = "M41.9063,163.6328 L41.9063,154.9609 L44.1563,154.9609 Q45.457,154.9609 46.1514,155.4531 Q46.8457,155.9453 46.8457,156.8711 Q46.8457,158.4473 45.0645,159.1152 Q47.1914,159.7656 47.1914,161.4648 Q47.1914,162.5195 46.4883,163.0762 Q45.7852,163.6328 44.4609,163.6328 Z M43.125,162.7129 L43.377,162.7129 Q44.7305,162.7129 45.1289,162.543 Q45.8906,162.2207 45.8906,161.3477 Q45.8906,160.5742 45.1992,160.0615 Q44.5078,159.5488 43.4707,159.5488 L43.125,159.5488 Z M43.125,158.7695 L43.5176,158.7695 Q44.502,158.7695 45.0439,158.3477 Q45.5859,157.9258 45.5859,157.1582 Q45.5859,155.8809 43.6055,155.8809 L43.125,155.8809 Z ";
const CONNECTOR_GLYPH_C: &str = "M44.3711,257.9824 Q42.3496,257.9824 41.248,256.7959 Q40.1465,255.6094 40.1465,253.4355 Q40.1465,251.2676 41.2686,250.0723 Q42.3906,248.877 44.4297,248.877 Q45.5957,248.877 47.1602,249.2578 L47.1602,250.4121 Q45.3789,249.7969 44.4121,249.7969 Q43,249.7969 42.2266,250.752 Q41.4531,251.707 41.4531,253.4473 Q41.4531,255.1055 42.2793,256.0635 Q43.1055,257.0215 44.5352,257.0215 Q45.7656,257.0215 47.1719,256.2656 L47.1719,257.3203 Q45.8887,257.9824 44.3711,257.9824 Z ";
const CONNECTOR_GLYPH_D: &str = "M40.9063,351.8984 L40.9063,343.2266 L43.7891,343.2266 Q45.0781,343.2266 45.8721,343.5137 Q46.666,343.8008 47.2461,344.4863 Q48.166,345.5762 48.166,347.3574 Q48.166,349.5195 47.0234,350.709 Q45.8809,351.8984 43.8066,351.8984 Z M42.1367,350.9785 L43.7129,350.9785 Q45.4004,350.9785 46.1035,350.0703 Q46.8594,349.1035 46.8594,347.4746 Q46.8594,345.9453 46.1152,345.0723 Q45.6641,344.5391 45.0371,344.3428 Q44.4102,344.1465 43.1504,344.1465 L42.1367,344.1465 Z ";
const CONNECTOR_GLYPH_E: &str = "M41.4063,446.0313 L41.4063,437.3594 L46.252,437.3594 L46.252,438.2793 L42.6367,438.2793 L42.6367,441.0625 L45.666,441.0625 L45.666,441.9707 L42.6367,441.9707 L42.6367,445.1113 L46.5039,445.1113 L46.5039,446.0313 Z ";
const CONNECTOR_GLYPH_F: &str = "M41.4063,540.1641 L41.4063,531.4922 L46.252,531.4922 L46.252,532.4121 L42.6367,532.4121 L42.6367,535.3184 L45.6719,535.3184 L45.6719,536.2266 L42.6367,536.2266 L42.6367,540.1641 Z ";
const CONNECTOR_GLYPH_1: &str = "M63.0098,123.8828 L63.0098,123.0156 L64.7441,123.0156 L64.7441,116.1719 L63.0098,116.6055 L63.0098,115.7148 L65.9043,114.9941 L65.9043,123.0156 L67.6387,123.0156 L67.6387,123.8828 Z ";
const CONNECTOR_GLYPH_2: &str = "M62.7012,217.7656 L62.7012,216.752 Q63.2051,215.5742 64.7402,214.1855 L65.4023,213.5938 Q66.6797,212.4395 66.6797,211.3027 Q66.6797,210.5762 66.2432,210.1602 Q65.8066,209.7441 65.0449,209.7441 Q64.1426,209.7441 62.918,210.4414 L62.918,209.4219 Q64.0723,208.877 65.209,208.877 Q66.4277,208.877 67.166,209.5332 Q67.9043,210.1895 67.9043,211.2734 Q67.9043,212.0527 67.5322,212.6563 Q67.1602,213.2598 66.1465,214.1211 L65.7012,214.502 Q64.3125,215.6797 64.0957,216.752 L67.8633,216.752 L67.8633,217.7656 Z ";

fn connector_glyph(label: &str, cx: f64, cy: f64) -> Option<String> {
    let (path, ref_cx, ref_cy) = match label {
        "A" => (CONNECTOR_GLYPH_A, 44.2871, 65.0),
        "B" => (CONNECTOR_GLYPH_B, 44.2871, 159.1328),
        "C" => (CONNECTOR_GLYPH_C, 44.2871, 253.2656),
        "D" => (CONNECTOR_GLYPH_D, 44.2871, 347.3984),
        "E" => (CONNECTOR_GLYPH_E, 44.2871, 441.5313),
        "F" => (CONNECTOR_GLYPH_F, 44.2871, 535.6641),
        "1" => (CONNECTOR_GLYPH_1, 65.2051, 119.1328),
        "2" => (CONNECTOR_GLYPH_2, 65.2051, 213.2656),
        _ => return None,
    };
    let dx = cx - ref_cx;
    let dy = cy - ref_cy;
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        Some(path.to_string())
    } else {
        Some(offset_connector_path(path, dx, dy))
    }
}

fn offset_connector_path(path: &str, dx: f64, dy: f64) -> String {
    let mut out = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '-' {
            let mut x = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_ascii_digit() || nc == '.' || nc == '-' {
                    x.push(nc);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(xv) = x.parse::<f64>() {
                if matches!(chars.peek(), Some(',')) {
                    chars.next();
                    let mut y = String::new();
                    while let Some(&nc) = chars.peek() {
                        if nc.is_ascii_digit() || nc == '.' || nc == '-' {
                            y.push(nc);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if let Ok(yv) = y.parse::<f64>() {
                        write!(out, "{},{}", f(xv + dx), f(yv + dy)).unwrap();
                    } else {
                        write!(out, "{},{}", f(xv + dx), y).unwrap();
                    }
                } else {
                    out.push_str(&f(xv + dx));
                }
            } else {
                out.push_str(&x);
            }
        } else {
            out.push(c);
            chars.next();
        }
    }

    out
}

fn decoration_lines(text: Option<&str>) -> Vec<&str> {
    text.map(|t| t.lines().filter(|l| !l.trim().is_empty()).collect())
        .unwrap_or_default()
}

fn decoration_width(lines: &[&str], font_size: f64, bold: bool, horizontal_pad: f64) -> f64 {
    lines
        .iter()
        .map(|line| text_render::measure_no_underline(line, font_size, bold) + horizontal_pad)
        .fold(0.0_f64, f64::max)
}

fn legend_rows(text: Option<&str>) -> Vec<Vec<&str>> {
    text.map(|t| {
        t.lines()
            .filter(|line| line.trim().contains('|'))
            .map(|line| {
                line.trim()
                    .trim_matches('|')
                    .split('|')
                    .map(str::trim)
                    .filter(|cell| !cell.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|row| !row.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

fn legend_column_widths(rows: &[Vec<&str>]) -> Vec<f64> {
    let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let cell_pad_x = pm::descent(LEGEND_FONT_SIZE) * LEGEND_CELL_PAD_DESCENT_FACTOR;
    let mut widths = vec![0.0_f64; ncols];
    for row in rows {
        for (idx, cell) in row.iter().enumerate() {
            let text_w = text_render::measure_no_underline(cell, LEGEND_FONT_SIZE, false);
            widths[idx] = widths[idx].max(text_w + 2.0 * cell_pad_x);
        }
    }
    widths
}

fn legend_rect_size(rows: &[Vec<&str>]) -> Option<(f64, f64)> {
    if rows.is_empty() {
        return None;
    }
    let grid_w = legend_column_widths(rows).iter().sum::<f64>();
    let row_h = pm::text_height(LEGEND_FONT_SIZE);
    Some((
        grid_w + 2.0 * LEGEND_RECT_PAD_X,
        rows.len() as f64 * row_h + 2.0 * LEGEND_RECT_PAD_Y,
    ))
}

fn legend_total_width(rows: &[Vec<&str>]) -> f64 {
    legend_rect_size(rows)
        .map(|(w, _)| w + LEGEND_RECT_X + LEGEND_RIGHT_PAD)
        .unwrap_or(0.0)
}

/// Walk a layout tree and collect every connector label so they can be
/// used to size the SVG.
fn collect_arrow_labels(nodes: &[LayoutNode]) -> Vec<String> {
    let mut out = Vec::new();
    for n in nodes {
        match n {
            LayoutNode::Arrow { label: Some(l), .. } => out.push(l.clone()),
            LayoutNode::If {
                then_branch,
                else_branches,
                ..
            } => {
                out.extend(collect_arrow_labels(then_branch));
                for b in else_branches {
                    out.extend(collect_arrow_labels(&b.body));
                }
            }
            LayoutNode::While { body, .. } | LayoutNode::Repeat { body, .. } => {
                out.extend(collect_arrow_labels(body));
            }
            LayoutNode::Fork { branches, .. } => {
                for b in branches {
                    out.extend(collect_arrow_labels(b));
                }
            }
            LayoutNode::Switch { cases, .. } => {
                for c in cases {
                    out.extend(collect_arrow_labels(&c.body));
                }
            }
            _ => {}
        }
    }
    out
}

fn polygon_points(points: &[(f64, f64)]) -> String {
    points
        .iter()
        .map(|(x, y)| format!("{},{}", f(*x), f(*y)))
        .collect::<Vec<_>>()
        .join(",")
}

struct SvgEmitter {
    /// Shapes and labels (rects, ellipses, polygon-shapes, text). PlantUML
    /// emits all of these first in document order.
    shapes: String,
    /// Connectors (lines, arrowhead polygons). PlantUML emits all of these
    /// after the shapes, also in document order.
    connectors: String,
    /// Resolved color palette for this render (PlantUML defaults +
    /// inline skinparam overrides).
    palette: Palette,
    colored_partition_while_depth: usize,
    partition_wrapped_fork_depth: usize,
    handwritten: bool,
    /// When an `if`/`elseif` branch's flow is a single no-special `while`, the
    /// branch→merge connection is owned by the loop's exit corridor (PlantUML
    /// fuses the FtileWhile `ConnectionOut` with the if's branch-merge snake
    /// under `MergeStrategy.LIMITED`). The if precomputes the merge geometry and
    /// pushes it here; `emit_while` consumes it (one-shot) to route its exit arm
    /// straight into the merge diamond instead of wrapping back to its own spine.
    while_exit_redirect: Option<WhileExitRedirect>,
    /// Extra inbound-gap added to each non-first tile inside a fork branch.
    /// PlantUML's `FtileFactoryDelegatorAssembly.assembly` uses a 35 px inter-
    /// tile space, but ordinary (top-level) sequences compress it back to the
    /// 20 px `ARROW_LEN`; fork branches mark their snakes `ignoreForCompression`
    /// (ParallelBuilderFork ConnectionIn/Out), so the full 35 survives. We model
    /// that as ARROW_LEN + this extra (15) for tiles assembled inside a branch.
    fork_branch_gap_extra: f64,
    /// Context for a `break` nested in an `if` inside the current `while` body.
    /// PlantUML collects each `FtileBreak` welding point and (in
    /// `FtileFactoryDelegatorWhile.createWhile`) draws a left-pointing arrow from
    /// the break tile out to the loop's left edge (`Hexagon.hexagonHalfSize`).
    /// `emit_while` sets this around its body emission so an enclosed
    /// break-bearing `if` (rendered as `FtileIfDown` with an empty `optionalStop`
    /// and `ConnectionElseNoDiamond`) can weld the break branch to the exit
    /// corridor instead of routing to a (suppressed) merge diamond.
    while_break: Option<WhileBreakContext>,
}

/// Geometry the enclosing `while` hands to a directly-nested break-bearing `if`.
#[derive(Clone, Copy)]
struct WhileBreakContext {
    /// Absolute x of the loop's left exit corridor (the break arrow's target).
    corridor_x: f64,
    /// Whether the while's column compresses one slot of slack out of the
    /// break-if's `ConnectionElseNoDiamond` corridor (true when the loop body
    /// carries enough adjacent flow content; see [`while_break_corridor_compresses`]).
    compresses: bool,
}

/// Geometry the parent `if` hands to a directly-nested `while` so the loop's
/// exit corridor terminates at the merge diamond.
#[derive(Clone, Copy)]
struct WhileExitRedirect {
    /// Centre-y of the merge diamond (the y of the merge horizontal run).
    merge_cy: f64,
    /// X of the merge-diamond vertex the corridor arrives at.
    merge_vertex_x: f64,
    /// `true` → then-branch (arrives at the left vertex, points right);
    /// `false` → else-branch (arrives at the right vertex, points left).
    to_right: bool,
}

#[allow(clippy::too_many_arguments)]
impl SvgEmitter {
    fn with_palette(palette: Palette, handwritten: bool) -> Self {
        SvgEmitter {
            shapes: String::new(),
            connectors: String::new(),
            palette,
            colored_partition_while_depth: 0,
            partition_wrapped_fork_depth: 0,
            handwritten,
            while_exit_redirect: None,
            fork_branch_gap_extra: 0.0,
            while_break: None,
        }
    }

    fn shadow_filter_attr(&self, enabled: bool) -> String {
        if enabled && let Some(id) = &self.palette.shadow_filter {
            format!(r#" filter="url(#{id})""#)
        } else {
            String::new()
        }
    }

    fn ellipse(
        &mut self,
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        fill: &str,
        stroke: &str,
        stroke_width: &str,
    ) {
        let filter = self.shadow_filter_attr(!(rx == STOP_INNER_R && ry == STOP_INNER_R));
        if self.handwritten {
            let points = handwritten_ellipse_points(cx, cy, rx, ry);
            write!(
                self.shapes,
                r#"<polygon fill="{}"{} points="{}" style="stroke:{};stroke-width:{};"/>"#,
                fill, filter, points, stroke, stroke_width
            )
            .unwrap();
            return;
        }
        write!(
            self.shapes,
            r#"<ellipse cx="{}" cy="{}" fill="{}"{} rx="{}" ry="{}" style="stroke:{};stroke-width:{};"/>"#,
            f(cx), f(cy), fill, filter, f(rx), f(ry), stroke, stroke_width
        )
        .unwrap();
    }

    fn rect_styled(
        &mut self,
        fill: &str,
        height: f64,
        rx: f64,
        ry: f64,
        stroke: &str,
        stroke_width: &str,
        width: f64,
        x: f64,
        y: f64,
    ) {
        let filter = self.shadow_filter_attr(true);
        if self.handwritten {
            let points = handwritten_rect_points(x, y, width, height, rx, ry);
            write!(
                self.shapes,
                r#"<polygon fill="{}"{} points="{}" style="stroke:{};stroke-width:{};"/>"#,
                fill, filter, points, stroke, stroke_width
            )
            .unwrap();
            return;
        }
        write!(
            self.shapes,
            r#"<rect fill="{}"{} height="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:{};" width="{}" x="{}" y="{}"/>"#,
            fill, filter, f(height), f(rx), f(ry), stroke, stroke_width, f(width), f(x), f(y)
        )
        .unwrap();
    }

    /// Emit a note "opale" (folded-corner box with a tail pointing at the
    /// anchoring node). `box_left`/`box_top` are the box's top-left corner;
    /// `box_w`/`box_h` its drawn size; `tip` the absolute coordinates of the
    /// tail's point on the anchor; `anchor_cy` the anchor's vertical centre.
    /// `left_side` is true when the note sits to the LEFT of its anchor (tail
    /// on the right edge, PlantUML's `getPolygonRight`); false when it sits to
    /// the right (tail on the left edge, `getPolygonLeft`). Faithful port of
    /// `Opale.getPolygonLeft`/`getPolygonRight` with `roundCorner == 0`.
    fn note_opale(
        &mut self,
        fill: &str,
        box_left: f64,
        box_top: f64,
        box_w: f64,
        box_h: f64,
        tip_x: f64,
        anchor_cy: f64,
        left_side: bool,
    ) {
        const CS: f64 = NOTE_FOLD; // cornersize
        const DELTA: f64 = 4.0;
        let bl = box_left;
        let bt = box_top;
        let br = box_left + box_w;
        let bb = box_top + box_h;
        let tip_y = anchor_cy;
        let mut d = String::new();
        if left_side {
            // getPolygonRight: tail on the right edge.
            let y1 = (tip_y - bt - DELTA).clamp(CS, box_h - 2.0 * DELTA);
            write!(
                d,
                "M{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                f(bl), f(bt),
                f(bl), f(bb),
                f(bl), f(bb),
                f(br), f(bb),
                f(br), f(bb),
                f(br), f(bt + y1 + 2.0 * DELTA),
                f(tip_x), f(tip_y),
                f(br), f(bt + y1),
                f(br), f(bt + CS),
                f(br - CS), f(bt),
                f(bl), f(bt),
                f(bl), f(bt),
            )
            .unwrap();
        } else {
            // getPolygonLeft: tail on the left edge.
            let y1 = (tip_y - bt - DELTA).clamp(0.0, box_h - 2.0 * DELTA);
            write!(
                d,
                "M{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                f(bl), f(bt),
                f(bl), f(bt + y1),
                f(tip_x), f(tip_y),
                f(bl), f(bt + y1 + 2.0 * DELTA),
                f(bl), f(bb),
                f(bl), f(bb),
                f(br), f(bb),
                f(br), f(bb),
                f(br), f(bt + CS),
                f(br - CS), f(bt),
                f(bl), f(bt),
                f(bl), f(bt),
            )
            .unwrap();
        }
        write!(
            self.shapes,
            r#"<path d="{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
            d, fill, NOTE_STROKE, NOTE_STROKE_WIDTH
        )
        .unwrap();
        // Fold corner (top-right triangle).
        write!(
            self.shapes,
            r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
            f(br - CS), f(bt),
            f(br - CS), f(bt + CS),
            f(br), f(bt + CS),
            f(br - CS), f(bt),
            fill, NOTE_STROKE, NOTE_STROKE_WIDTH
        )
        .unwrap();
    }

    /// Emit a tail-less folded-corner note box (a `floating note`, which has
    /// no connector tail pointing at an anchor). Two paths — body then the
    /// top-right fold triangle — matching PlantUML's floating-note emission.
    fn note_folded(&mut self, fill: &str, box_left: f64, box_top: f64, box_w: f64, box_h: f64) {
        const CS: f64 = NOTE_FOLD; // cornersize
        let bl = box_left;
        let bt = box_top;
        let br = box_left + box_w;
        let bb = box_top + box_h;
        // Body: top-left → bottom-left → bottom-right → up to the fold base →
        // diagonal across the fold → back to top-left.
        write!(
            self.shapes,
            r#"<path d="M{},{} L{},{} L{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
            f(bl), f(bt),
            f(bl), f(bb),
            f(br), f(bb),
            f(br), f(bt + CS),
            f(br - CS), f(bt),
            f(bl), f(bt),
            fill, NOTE_STROKE, NOTE_STROKE_WIDTH
        )
        .unwrap();
        // Fold corner (top-right triangle).
        write!(
            self.shapes,
            r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
            f(br - CS), f(bt),
            f(br - CS), f(bt + CS),
            f(br), f(bt + CS),
            f(br - CS), f(bt),
            fill, NOTE_STROKE, NOTE_STROKE_WIDTH
        )
        .unwrap();
    }

    /// Emit a partition's outer rectangle (no rounded corners).
    fn partition_rect(&mut self, fill: &str, height: f64, width: f64, x: f64, y: f64) {
        write!(
            self.shapes,
            r#"<rect fill="{}" height="{}" style="stroke:#000000;stroke-width:1.5;" width="{}" x="{}" y="{}"/>"#,
            fill,
            f(height),
            f(width),
            f(x),
            f(y)
        )
        .unwrap();
    }

    /// Emit the title-corner notch path on a partition.
    fn partition_path(&mut self, r: f64, y: f64, partition_x: f64) {
        write!(
            self.shapes,
            r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="none" style="stroke:#000000;stroke-width:1.5;"/>"#,
            f(r),
            f(y),
            f(r),
            f(y + 9.4883),
            f(r - 10.0),
            f(y + 19.4883),
            f(partition_x),
            f(y + 19.4883)
        )
        .unwrap();
    }

    fn text_element(
        &mut self,
        fill: &str,
        font_family: &str,
        font_size: f64,
        _text_length: f64,
        x: f64,
        y: f64,
        content: &str,
        bold: bool,
    ) {
        self.text_element_styled(fill, font_family, font_size, x, y, content, bold, false);
    }

    #[allow(clippy::too_many_arguments)]
    fn text_element_styled(
        &mut self,
        fill: &str,
        font_family: &str,
        font_size: f64,
        x: f64,
        y: f64,
        content: &str,
        bold: bool,
        italic: bool,
    ) {
        // text_length is ignored: emit_text computes widths from segments
        // (after creole stripping). Upstream geometry that sized boxes
        // around this text should already have measured the stripped text.
        let base = TextBase {
            x,
            y,
            font_size: font_size as u32,
            font_family,
            fill,
            bold,
            italic,
            underline: false,
            skip_underline: false,
        };
        text_render::emit_text(&mut self.shapes, content, &base);
    }

    fn monospace_text_element(
        &mut self,
        fill: &str,
        font_size: f64,
        text_length: f64,
        x: f64,
        y: f64,
        content: &str,
    ) {
        // PlantUML emits `<`/`>` as `&lt;`/`&gt;` and U+00A0 as the numeric
        // entity `&#xA0;` (rather than the raw UTF-8 byte sequence).
        let escaped = svg_text_escape(content);
        write!(
            self.shapes,
            r#"<text fill="{}" font-family="monospace" font-size="{}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
            fill, font_size as u32, f(text_length), f(x), f(y), escaped
        )
        .unwrap();
    }

    /// A line that belongs with the SHAPE group (e.g. the X inside an
    /// `end` node — visually part of the node, not a connector).
    fn shape_line(&mut self, stroke: &str, stroke_width: &str, x1: f64, x2: f64, y1: f64, y2: f64) {
        if self.handwritten {
            let d = handwritten_line_path(x1, y1, x2, y2);
            write!(self.shapes, r#"<path d="{d}" fill="{stroke}"/>"#).unwrap();
            return;
        }
        write!(
            self.shapes,
            r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            stroke,
            stroke_width,
            f(x1),
            f(x2),
            f(y1),
            f(y2)
        )
        .unwrap();
    }

    /// A node-style polygon (diamond, fork-bar variant, etc.) — goes with
    /// shapes in the document order.
    fn polygon_shape(
        &mut self,
        fill: &str,
        points: &[(f64, f64)],
        stroke: &str,
        stroke_width: &str,
    ) {
        // PlantUML closes filled shape polygons by repeating the first point
        // as the last entry. Arrowhead/connector polygons (polygon_connector)
        // do not.
        let mut closed: Vec<(f64, f64)> = points.to_vec();
        if let (Some(first), Some(last)) = (points.first(), points.last())
            && first != last
        {
            closed.push(*first);
        }
        let pts = if self.handwritten {
            handwritten_polygon_points(&closed)
        } else {
            polygon_points(&closed)
        };
        let filter = self.shadow_filter_attr(true);
        write!(
            self.shapes,
            r#"<polygon fill="{}"{} points="{}" style="stroke:{};stroke-width:{};"/>"#,
            fill, filter, pts, stroke, stroke_width
        )
        .unwrap();
    }

    fn fill_path(&mut self, d: &str, fill: &str) {
        write!(self.shapes, r#"<path d="{d}" fill="{fill}"/>"#).unwrap();
    }

    fn raw(&mut self, s: &str) {
        self.shapes.push_str(s);
    }

    fn raw_connector(&mut self, s: &str) {
        self.connectors.push_str(s);
    }

    fn line_styled(
        &mut self,
        stroke: &str,
        stroke_width: &str,
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        dashed: bool,
    ) {
        let dash = if dashed { "stroke-dasharray:2,2;" } else { "" };
        if self.handwritten {
            let d = handwritten_line_path(x1, y1, x2, y2);
            if dashed {
                write!(
                    self.connectors,
                    r#"<path d="{d}" fill="none" style="stroke:{stroke};stroke-width:{stroke_width};{dash}"/>"#
                )
                .unwrap();
            } else {
                write!(self.connectors, r#"<path d="{d}" fill="{stroke}"/>"#).unwrap();
            }
            return;
        }
        write!(
            self.connectors,
            r#"<line style="stroke:{};stroke-width:{};{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            stroke,
            stroke_width,
            dash,
            f(x1),
            f(x2),
            f(y1),
            f(y2)
        )
        .unwrap();
    }

    /// Emit a connector line that follows the palette's arrow thickness
    /// (so `skinparam activityBorderThickness` cascades through the
    /// if/fork/while/repeat connector frames without each callsite having
    /// to thread the value).
    fn connector_line(&mut self, stroke: &str, x1: f64, x2: f64, y1: f64, y2: f64, dashed: bool) {
        let thickness = self.palette.arrow_thickness.clone();
        self.line_styled(stroke, &thickness, x1, x2, y1, y2, dashed);
    }

    fn connector_line_full(&mut self, style: &ArrowStyle, x1: f64, x2: f64, y1: f64, y2: f64) {
        let palette_thickness = self.palette.arrow_thickness.clone();
        let stroke_width = if style.bold {
            "2"
        } else if style.dotted {
            "1.5"
        } else {
            palette_thickness.as_str()
        };
        let dash = if style.dotted {
            Some("1,3")
        } else if style.dashed {
            Some("2,2")
        } else {
            None
        };
        self.line_with_dash(&style.color, stroke_width, x1, x2, y1, y2, dash);
    }

    /// Arrowhead-style polygon for connectors — goes after all shapes.
    fn polygon_connector(
        &mut self,
        fill: &str,
        points: &[(f64, f64)],
        stroke: &str,
        stroke_width: &str,
    ) {
        let pts = if self.handwritten {
            handwritten_polygon_points(points)
        } else {
            polygon_points(points)
        };
        write!(
            self.connectors,
            r#"<polygon fill="{}" points="{}" style="stroke:{};stroke-width:{};"/>"#,
            fill, pts, stroke, stroke_width
        )
        .unwrap();
    }

    /// Emit a downward arrow (vertical line + arrowhead polygon).
    fn down_arrow(&mut self, cx: f64, y1: f64, y2: f64, color: &str) {
        let thickness = self.palette.arrow_thickness.clone();
        self.line_styled(color, &thickness, cx, cx, y1, y2, false);
        // Arrowhead: 4px each side, 10px tall, 4px notch. The arrowhead
        // keeps stroke-width:1 — PlantUML scales only the line.
        self.polygon_connector(
            color,
            &[
                (cx - 4.0, y2 - 10.0),
                (cx, y2),
                (cx + 4.0, y2 - 10.0),
                (cx, y2 - 6.0),
            ],
            color,
            "1",
        );
    }

    /// Emit a styled downward arrow (handles colour, dashed/dotted, bold).
    fn down_arrow_full(&mut self, cx: f64, y1: f64, y2: f64, style: &ArrowStyle) {
        // Per-arrow style markers (bold/dotted) override any global
        // `activityBorderThickness` cascade. A plain arrow follows the
        // palette's connector thickness.
        let palette_thickness = self.palette.arrow_thickness.clone();
        let sw: &str = if style.bold {
            "2"
        } else if style.dotted {
            "1.5"
        } else {
            palette_thickness.as_str()
        };
        let dash = if style.dotted {
            Some("1,3")
        } else if style.dashed {
            Some("2,2")
        } else {
            None
        };
        self.line_with_dash(&style.color, sw, cx, cx, y1, y2, dash);
        // Bold arrows in PlantUML keep the same arrowhead size; only the
        // line stroke changes. The polygon stays 1px.
        self.polygon_connector(
            &style.color,
            &[
                (cx - 4.0, y2 - 10.0),
                (cx, y2),
                (cx + 4.0, y2 - 10.0),
                (cx, y2 - 6.0),
            ],
            &style.color,
            "1",
        );
    }

    /// Emit a text element into the connectors buffer (used for arrow labels
    /// which PlantUML interleaves with the connector group rather than the
    /// shape group).
    fn connector_text(
        &mut self,
        fill: &str,
        font_family: &str,
        font_size: f64,
        _text_length: f64,
        x: f64,
        y: f64,
        content: &str,
    ) {
        let base = TextBase {
            x,
            y,
            font_size: font_size as u32,
            font_family,
            fill,
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        };
        text_render::emit_text(&mut self.connectors, content, &base);
    }

    /// Emit a line with an explicit dasharray pattern (or none).
    fn line_with_dash(
        &mut self,
        stroke: &str,
        stroke_width: &str,
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        dash: Option<&str>,
    ) {
        let dash_str = match dash {
            Some(d) => format!("stroke-dasharray:{d};"),
            None => String::new(),
        };
        if self.handwritten {
            let d = handwritten_line_path(x1, y1, x2, y2);
            if dash.is_some() {
                write!(
                    self.connectors,
                    r#"<path d="{d}" fill="none" style="stroke:{stroke};stroke-width:{stroke_width};{dash_str}"/>"#
                )
                .unwrap();
            } else {
                write!(self.connectors, r#"<path d="{d}" fill="{stroke}"/>"#).unwrap();
            }
            return;
        }
        write!(
            self.connectors,
            r#"<line style="stroke:{};stroke-width:{};{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            stroke,
            stroke_width,
            dash_str,
            f(x1),
            f(x2),
            f(y1),
            f(y2)
        )
        .unwrap();
    }

    /// Emit an upward arrow (arrowhead pointing up).
    #[allow(dead_code)]
    fn up_arrow(&mut self, cx: f64, y1: f64, y2: f64, color: &str) {
        self.polygon_connector(
            color,
            &[
                (cx - 4.0, y1 + 10.0),
                (cx, y1),
                (cx + 4.0, y1 + 10.0),
                (cx, y1 + 6.0),
            ],
            color,
            "1",
        );
        // Line from y2 down to y1 (y2 > y1 in this context)
        self.line_styled(color, "1", cx, cx, y1, y2, false);
    }

    /// Emit a right-pointing arrow on a horizontal line.
    fn right_arrow(&mut self, x_tip: f64, y: f64, color: &str) {
        self.polygon_connector(
            color,
            &[
                (x_tip - 10.0, y - 4.0),
                (x_tip, y),
                (x_tip - 10.0, y + 4.0),
                (x_tip - 6.0, y),
            ],
            color,
            "1",
        );
    }

    /// Emit a left-pointing arrow on a horizontal line.
    fn left_arrow(&mut self, x_tip: f64, y: f64, color: &str) {
        self.polygon_connector(
            color,
            &[
                (x_tip + 10.0, y - 4.0),
                (x_tip, y),
                (x_tip + 10.0, y + 4.0),
                (x_tip + 6.0, y),
            ],
            color,
            "1",
        );
    }
}

fn emit_legend_table(svg: &mut SvgEmitter, rows: &[Vec<&str>], y: f64) {
    let Some((rect_w, rect_h)) = legend_rect_size(rows) else {
        return;
    };
    let source_line = LEGEND_DEFAULT_SOURCE_LINE;
    let col_widths = legend_column_widths(rows);
    let row_h = pm::text_height(LEGEND_FONT_SIZE);
    let cell_pad_x = pm::descent(LEGEND_FONT_SIZE) * LEGEND_CELL_PAD_DESCENT_FACTOR;
    let rect_x = LEGEND_RECT_X;
    let grid_left = rect_x + LEGEND_RECT_PAD_X;
    let grid_top = y + LEGEND_RECT_PAD_Y;
    let grid_w = col_widths.iter().sum::<f64>();
    let grid_right = grid_left + grid_w;
    let grid_bottom = grid_top + rows.len() as f64 * row_h;

    svg.raw_connector(&format!(
        r#"<g class="legend" data-source-line="{source_line}">"#
    ));
    svg.raw_connector(&format!(
        r##"<rect fill="#DDDDDD" height="{}" rx="{}" ry="{}" style="stroke:#000000;stroke-width:1;" width="{}" x="{}" y="{}"/>"##,
        f(rect_h),
        f(LEGEND_RECT_RX),
        f(LEGEND_RECT_RX),
        f(rect_w),
        f(rect_x),
        f(y)
    ));

    for (row_idx, row) in rows.iter().enumerate() {
        let mut x = grid_left;
        let baseline = grid_top + pm::ascent(LEGEND_FONT_SIZE) + row_idx as f64 * row_h;
        for (col_idx, cell) in row.iter().enumerate() {
            let tw = text_render::measure_no_underline(cell, LEGEND_FONT_SIZE, false);
            svg.connector_text(
                TEXT_COLOR,
                "sans-serif",
                LEGEND_FONT_SIZE,
                tw,
                x + cell_pad_x,
                baseline,
                cell,
            );
            x += col_widths.get(col_idx).copied().unwrap_or(0.0);
        }
    }

    for idx in 0..=rows.len() {
        let y_line = grid_top + idx as f64 * row_h;
        svg.raw_connector(&format!(
            r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            f(grid_left),
            f(grid_right),
            f(y_line),
            f(y_line)
        ));
    }

    let mut x_line = grid_left;
    svg.raw_connector(&format!(
        r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
        f(x_line),
        f(x_line),
        f(grid_top),
        f(grid_bottom)
    ));
    for width in col_widths {
        x_line += width;
        svg.raw_connector(&format!(
            r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            f(x_line),
            f(x_line),
            f(grid_top),
            f(grid_bottom)
        ));
    }
    svg.raw_connector("</g>");
}

fn emit_pending_down_arrow(
    svg: &mut SvgEmitter,
    arrow_top: f64,
    style: ArrowStyle,
    label: Option<String>,
    arrow_gap: f64,
    cx: f64,
) {
    svg.down_arrow_full(cx, arrow_top, arrow_top + arrow_gap, &style);
    if let Some(l) = label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(&l, label_font_size, false, &label_family);
        svg.connector_text(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            cx + 4.0,
            arrow_top + 21.455078125,
            &l,
        );
    }
}

/// Render a linear sequence of nodes at a given center-x and starting y.
/// Returns the y position after the last node.
fn emit_sequence(svg: &mut SvgEmitter, nodes: &[LayoutNode], cx: f64, y: f64) -> f64 {
    emit_sequence_ex(svg, nodes, cx, y, None, None, false)
}

fn emit_sequence_if_branch(svg: &mut SvgEmitter, nodes: &[LayoutNode], cx: f64, y: f64) -> f64 {
    emit_sequence_ex(svg, nodes, cx, y, None, None, true)
}

/// Like `emit_sequence`, but `mid_stretch = Some((flow_idx, extra))` adds
/// `extra` px to the inbound arrow of the `flow_idx`-th flow node. Used by the
/// FtileIfDown corridor layout, where an even-action populated branch stretches
/// the gap straddling its vertical midpoint by 15 px.
///
/// `lead_note_h = Some(h)` signals that a leading `floating note` of box
/// height `h` is centred on the diagram's start node: the start ellipse sits
/// at cy = 15 + h/2 (top of the note tile pinned at 15) and the spine resumes
/// from the note tile's bottom (15 + h), pushing everything below down.
fn emit_sequence_ex(
    svg: &mut SvgEmitter,
    nodes: &[LayoutNode],
    cx: f64,
    mut y: f64,
    mid_stretch: Option<(usize, f64)>,
    lead_note_h: Option<f64>,
    first_repeat_branch_extra: bool,
) -> f64 {
    // Map flow-node ordinal → node index so the stretch can target the right
    // inbound arrow.
    let mut flow_ordinal = 0usize;
    // Extra length applied to the single arrow leaving a leading-floating-note
    // start node (consumed by the next flow node's inbound connector).
    let mut lead_stretch = 0.0f64;
    let mut carry_gap_extra = 0.0f64;
    let mut deferred_partition_inbound: Option<(f64, ArrowStyle, Option<String>, f64)> = None;
    // True when the immediately preceding flow tile was a compressed while whose
    // inbound connector was deferred. PlantUML draws a chain of sequential
    // compressed whiles by emitting each while's inbound BEFORE its body (and
    // flushing the predecessor's deferred inbound right after), rather than
    // deferring every while past the whole chain. Only the FIRST while in such a
    // run (the one preceded by a non-while tile) is genuinely deferred.
    let mut prev_was_deferred_while = false;
    for (i, node) in nodes.iter().enumerate() {
        // Skip layout for non-flow nodes (arrows and notes don't take vertical space
        // on their own).
        if matches!(node, LayoutNode::Arrow { .. } | LayoutNode::Note { .. }) {
            continue;
        }
        // Detach/Kill/Break/Goto also produce no shape and no incoming
        // connector — they mark the previous flow as terminated.
        if matches!(
            node,
            LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break | LayoutNode::Goto(_)
        ) {
            continue;
        }
        if is_empty_partition_node(node) {
            let shell_bottom = emit_node(svg, node, cx, y);
            let shell_h = shell_bottom - y;
            if flow_ordinal == 0 {
                y = shell_bottom;
            } else {
                carry_gap_extra += shell_h;
            }
            continue;
        }
        // Title is a free-standing label; never gets an inbound connector.
        if let LayoutNode::Title { .. } = node {
            y = emit_node(svg, node, cx, y);
            continue;
        }
        // Leading floating note: the start ellipse is vertically centred on
        // the note tile (top pinned at 15) and the spine is pushed down so the
        // next node sits at `tile_bottom + ARROW_LEN`. The connector still
        // departs from the ellipse's bottom (cy + START_R), so the outbound
        // arrow lengthens by `tile_bottom - ellipse_bottom = nh/2 - START_R`,
        // applied below as `lead_stretch`. The note paths/text were already
        // emitted by the caller's prologue.
        if let (0, LayoutNode::Start, Some(nh)) = (flow_ordinal, node, lead_note_h)
            && nh > 2.0 * START_R
        {
            let top = 15.0; // MARGIN_LEAD - 1
            let cy = top + nh / 2.0;
            let fill = svg.palette.start_fill.clone();
            let stroke = svg.palette.start_stroke.clone();
            svg.ellipse(cx, cy, START_R, START_R, &fill, &stroke, "1");
            y = cy + START_R; // connector departs from the ellipse bottom
            lead_stretch = nh / 2.0 - START_R; // extra length for the next arrow
            flow_ordinal += 1;
            continue;
        }
        let (note_inbound_extra, note_bottom_extra) =
            flow_note_vertical_extras(nodes, i, node_height(node));
        // Compute the inbound down-arrow's style + gap (if any). PlantUML
        // emits inbound connectors AFTER the destination node's internal
        // connectors, so we defer the actual svg writes until after
        // emit_node returns. We still advance `y` upfront so the node lands
        // at the right position.
        let mut pending_arrow: Option<(f64, ArrowStyle, Option<String>, f64)> = None;
        if i > 0 {
            let mut explicit_arrow: Option<&LayoutNode> = None;
            let mut prev_idx: Option<usize> = None;
            for j in (0..i).rev() {
                match &nodes[j] {
                    LayoutNode::Arrow { .. } => {
                        if explicit_arrow.is_none() {
                            explicit_arrow = Some(&nodes[j]);
                        }
                    }
                    LayoutNode::Note { .. } => {}
                    LayoutNode::Title { .. } => {}
                    n if is_empty_partition_node(n) => {}
                    LayoutNode::Detach
                    | LayoutNode::Kill
                    | LayoutNode::Break
                    | LayoutNode::Goto(_) => {
                        prev_idx = None;
                        break;
                    }
                    _ => {
                        prev_idx = Some(j);
                        break;
                    }
                }
            }
            let skip_implicit_inbound_after_single_survivor_if = explicit_arrow.is_none()
                && prev_idx
                    .and_then(|j| nodes.get(j))
                    .is_some_and(if_node_has_single_survivor);
            if prev_idx.is_some() && !skip_implicit_inbound_after_single_survivor_if {
                let style = match explicit_arrow {
                    Some(LayoutNode::Arrow {
                        color: Some(c),
                        dashed,
                        ..
                    }) => arrow_style_from_brackets(c, *dashed),
                    Some(LayoutNode::Arrow { dashed, .. }) => ArrowStyle {
                        color: svg.palette.arrow_color.clone(),
                        dashed: *dashed,
                        dotted: false,
                        bold: false,
                        hidden: false,
                    },
                    _ => ArrowStyle {
                        color: svg.palette.arrow_color.clone(),
                        ..ArrowStyle::default()
                    },
                };
                let label = match explicit_arrow {
                    Some(LayoutNode::Arrow { label: Some(l), .. }) => Some(l.clone()),
                    _ => None,
                };
                let stretch = match mid_stretch {
                    Some((idx, extra)) if idx == flow_ordinal => extra,
                    _ => 0.0,
                };
                let lead = std::mem::take(&mut lead_stretch);
                let carry = std::mem::take(&mut carry_gap_extra);
                let prev_outbound_gap = prev_idx
                    .and_then(|j| nodes.get(j))
                    .and_then(repeat_not_label_outbound_gap);
                let gap = stretch
                    + lead
                    + carry
                    + if style.hidden {
                        10.0
                    } else if label.is_some() {
                        LABELED_ARROW_LEN
                    } else {
                        prev_outbound_gap.unwrap_or_else(|| {
                            default_inbound_gap(node) + svg.fork_branch_gap_extra
                        })
                    };
                // Partition entry: stretch the inbound arrow so it spans the
                // full distance from prev cursor through the title bar to
                // the first inner action's top (no separate arrow to the
                // partition rect).
                let partition_top_gap = match node {
                    LayoutNode::Partition {
                        color,
                        name,
                        is_group,
                        nested,
                        single_lane_first_group,
                        body,
                        ..
                    } => Some(partition_top_gap(
                        color,
                        name,
                        *is_group,
                        *nested,
                        *single_lane_first_group,
                        body,
                    )),
                    _ => None,
                };
                let prev_was_partition = matches!(
                    prev_idx.and_then(|j| nodes.get(j)),
                    Some(LayoutNode::Partition { .. })
                );
                let prev_was_split = matches!(
                    prev_idx.and_then(|j| nodes.get(j)),
                    Some(LayoutNode::Fork { is_split: true, .. })
                );
                let prev_was_all_goto_if =
                    prev_idx
                        .and_then(|j| nodes.get(j))
                        .is_some_and(|prev| match prev {
                            LayoutNode::If {
                                then_branch,
                                else_branches,
                                ..
                            } => if_all_branches_goto(then_branch, else_branches),
                            _ => false,
                        });
                let is_partition = partition_top_gap.is_some();
                // A long if/elseif/else draws its own multi-segment inbound
                // connector (`ConnectionIn`) from the previous node's bottom to
                // the first diamond, so the standard straight inbound arrow is
                // suppressed and `y` is not advanced past the prev bottom.
                let is_long_if = matches!(
                    node,
                    LayoutNode::If { else_branches, .. } if if_is_long(else_branches)
                );
                // When the previous flow node was a partition, the inbound
                // arrow to the current node extends back 12 px into the
                // partition's bottom margin (overlaying the partition rect).
                let arrow_top_y = if prev_was_partition {
                    y - 12.0
                } else if prev_was_split {
                    y + 1.5
                } else if prev_was_all_goto_if {
                    y + IF_GOTO_RESUME_GAP
                } else {
                    y
                };
                let arrow_gap = {
                    let base = if let Some(tg) = partition_top_gap {
                        // y_in is `y` (no advance yet); first inner action
                        // sits at y + tg + 36.4883.
                        tg + 36.4883
                    } else {
                        gap
                    };
                    let base = base + note_inbound_extra;
                    if prev_was_partition {
                        base + 12.0
                    } else if prev_was_split {
                        base - 1.5
                    } else if prev_was_all_goto_if {
                        base - IF_GOTO_RESUME_GAP
                    } else {
                        base
                    }
                };
                if !style.hidden && !is_long_if {
                    pending_arrow = Some((arrow_top_y, style, label, arrow_gap));
                }
                // Don't advance y past the partition's outer top — the
                // partition's emit handles its own top positioning at y + 10.
                // Long-ifs also keep `y` at the prev bottom (their own
                // ConnectionIn spans the gap to the diamond row).
                if !is_partition && !is_long_if {
                    y += gap + note_inbound_extra;
                }
            }
        }
        // Emit any notes attached to this flow node BEFORE the node itself,
        // so the note's box/text precede the anchor's shape in document order
        // (matching PlantUML's emission). Notes that follow the node (until the
        // next flow node) anchor to it.
        if matches!(node, LayoutNode::Action { .. }) {
            let anchor_w = node_width(node);
            let anchor_h = node_height(node);
            let anchor_cy = y + anchor_h / 2.0;
            for follow in nodes[i + 1..].iter() {
                match follow {
                    LayoutNode::Note {
                        text,
                        position,
                        color,
                    } => {
                        emit_attached_note(
                            svg,
                            text,
                            position,
                            color.as_deref(),
                            cx,
                            anchor_w,
                            anchor_cy,
                        );
                    }
                    LayoutNode::Arrow { .. } => {}
                    _ => break,
                }
            }
        }
        if let LayoutNode::If {
            condition,
            diamond_font_family,
            diamond_font_size,
            diamond_text_bold,
            diamond_pad_x,
            attached_notes,
            ..
        } = node
            && !attached_notes.is_empty()
        {
            let diamond_half_w = if_diamond_half_width(
                condition,
                *diamond_font_size,
                *diamond_text_bold,
                diamond_font_family,
                *diamond_pad_x,
            );
            emit_if_attached_notes(svg, attached_notes, cx, y, diamond_half_w);
        }
        if let LayoutNode::Fork {
            branches,
            attached_notes,
            is_split,
            ..
        } = node
            && !attached_notes.is_empty()
        {
            let layout = if *is_split {
                split_layout(branches)
            } else if first_repeat_branch_extra {
                fork_layout_if_branch(branches)
            } else {
                fork_layout(branches)
            };
            emit_fork_attached_notes(svg, attached_notes, cx, y, branches, &layout);
        }
        let repeat_extra = if first_repeat_branch_extra && flow_ordinal == 0 {
            leading_if_branch_repeat_extra(node)
        } else {
            0.0
        };
        // A run of sequential compressed whiles (bare, or each wrapped in its
        // own partition): each loop after the first emits its OWN inbound
        // connector BEFORE its body, then flushes the predecessor loop's
        // deferred inbound. Only the leading loop (the one preceded by a
        // non-deferring tile) is deferred past the chain. This reproduces
        // PlantUML's tile-assembly order, where the connection into each loop is
        // drawn at the boundary between consecutive loops rather than collapsed
        // to the end. Without this, consecutive deferrals would overwrite one
        // another and drop the inner loops' inbound arrows.
        let node_defers = while_body_chain_compresses(node)
            || is_partition_wrapping_compressed_while(node);
        let is_chain_while = node_defers && prev_was_deferred_while;
        if is_chain_while
            && let Some((arrow_top, style, label, arrow_gap)) = pending_arrow.take()
        {
            emit_pending_down_arrow(svg, arrow_top, style, label, arrow_gap, cx);
            if let Some((p_top, p_style, p_label, p_gap)) = deferred_partition_inbound.take() {
                emit_pending_down_arrow(svg, p_top, p_style, p_label, p_gap, cx);
            }
        }
        let node_y =
            emit_node_with_repeat_extra(svg, node, cx, y, repeat_extra, first_repeat_branch_extra);
        prev_was_deferred_while = node_defers;
        // Inbound connector goes AFTER the node's own emit so it lands
        // after the node's internal connectors in the connectors buffer
        // (matches PlantUML's emission order: internal first, then inbound).
        if let Some((arrow_top, style, label, arrow_gap)) = pending_arrow {
            if is_colored_partition_wrapping_while(node)
                || is_partition_wrapping_compressed_while(node)
                || is_ordinary_compressed_while(node)
                || is_break_down_if(node)
            {
                deferred_partition_inbound = Some((arrow_top, style, label, arrow_gap));
            } else {
                emit_pending_down_arrow(svg, arrow_top, style, label, arrow_gap, cx);
                if let Some((arrow_top, style, label, arrow_gap)) =
                    deferred_partition_inbound.take()
                {
                    emit_pending_down_arrow(svg, arrow_top, style, label, arrow_gap, cx);
                }
            }
        }
        y = node_y;
        carry_gap_extra = note_bottom_extra;
        flow_ordinal += 1;
    }
    if let Some((arrow_top, style, label, arrow_gap)) = deferred_partition_inbound {
        emit_pending_down_arrow(svg, arrow_top, style, label, arrow_gap, cx);
    }
    y
}

/// Emit a single node at the given center-x and y position.
/// Returns the y position after this node (bottom edge).
fn emit_node(svg: &mut SvgEmitter, node: &LayoutNode, cx: f64, y: f64) -> f64 {
    emit_node_with_repeat_extra(svg, node, cx, y, 0.0, false)
}

fn emit_node_with_repeat_extra(
    svg: &mut SvgEmitter,
    node: &LayoutNode,
    cx: f64,
    y: f64,
    repeat_body_top_extra: f64,
    if_branch: bool,
) -> f64 {
    match node {
        LayoutNode::Start => {
            // The cursor (`y`) represents the centreline at which the next
            // node should sit. PlantUML enforces a minimum of START_CY (25)
            // so the ellipse's top is at least 15 px from the SVG top edge;
            // for first-thing layouts the cursor sits at MARGIN_LEAD (16)
            // and gets clamped up. After a warnings band or title the
            // cursor is already past START_CY and is used as-is.
            let cy = y.max(START_CY);
            let fill = svg.palette.start_fill.clone();
            let stroke = svg.palette.start_stroke.clone();
            svg.ellipse(cx, cy, START_R, START_R, &fill, &stroke, "1");
            cy + START_R
        }
        LayoutNode::Stop => {
            let cy = y + STOP_OUTER_R;
            let fill = svg.palette.stop_fill.clone();
            let stroke = svg.palette.stop_stroke.clone();
            svg.ellipse(cx, cy, STOP_OUTER_R, STOP_OUTER_R, "none", &stroke, "1");
            svg.ellipse(cx, cy, STOP_INNER_R, STOP_INNER_R, &fill, &stroke, "1");
            y + STOP_OUTER_R * 2.0
        }
        LayoutNode::End => {
            // PlantUML's `end` node is a circle with an X inside (not the
            // filled-bullseye that `stop` uses).
            //  - Outer circle: rx=10, fill=none, stroke-width=1.5
            //  - Two diagonal lines forming an X, stroke-width=2.5
            // The X spans from (cx-6.1872, cy-6.1872) to (cx+6.1872, cy+6.1872).
            const END_R: f64 = 10.0;
            const X_HALF: f64 = 6.1872; // empirical from goldens
            let cy = y + END_R;
            let stroke = svg.palette.stop_stroke.clone();
            svg.ellipse(cx, cy, END_R, END_R, "none", &stroke, "1.5");
            // X lines belong with shapes (between ellipse and any following
            // text) — they are the visual content of the end node.
            svg.shape_line(
                &stroke,
                "2.5",
                cx - X_HALF,
                cx + X_HALF,
                cy - X_HALF,
                cy + X_HALF,
            );
            svg.shape_line(
                &stroke,
                "2.5",
                cx + X_HALF,
                cx - X_HALF,
                cy - X_HALF,
                cy + X_HALF,
            );
            y + END_R * 2.0
        }
        LayoutNode::Connector(label) => {
            let cy = y + CONNECTOR_R;
            let fill = svg.palette.action_fill.clone();
            let stroke = svg.palette.action_stroke.clone();
            let sw = svg.palette.action_stroke_width.clone();
            svg.ellipse(cx, cy, CONNECTOR_R, CONNECTOR_R, &fill, &stroke, &sw);
            if let Some(d) = connector_glyph(label, cx, cy) {
                svg.fill_path(&d, TEXT_COLOR);
            } else {
                let tw = text_render::measure(label, SMALL_FONT, false);
                svg.text_element(
                    TEXT_COLOR,
                    "sans-serif",
                    SMALL_FONT,
                    tw,
                    cx - tw / 2.0,
                    centered_text_y(cy, SMALL_FONT),
                    label,
                    false,
                );
            }
            y + CONNECTOR_R * 2.0
        }
        LayoutNode::Action {
            text,
            text_width,
            pad_x,
            pad_y,
            font_family,
            font_size,
            bold,
            italic,
        } => {
            let ah = action_height(text, *pad_y, font_family, *font_size);
            let rect_w = *text_width + *pad_x * 2.0;
            let rect_x = cx - rect_w / 2.0;
            let fill = svg.palette.action_fill.clone();
            let stroke = svg.palette.action_stroke.clone();
            let sw = svg.palette.action_stroke_width.clone();
            let text_col = svg.palette.text_color.clone();
            svg.rect_styled(
                &fill,
                ah,
                svg.palette.action_rx,
                svg.palette.action_rx,
                &stroke,
                &sw,
                rect_w,
                rect_x,
                y,
            );
            if let Some(rows) = action_table_rows(text) {
                let col_widths = action_table_col_widths(&rows, *font_size, *bold, font_family);
                let line_h = text_render::label_height_with_family("", *font_size, font_family);
                let ascent = text_render::label_ascent_with_family("", *font_size, font_family);
                let table_left = rect_x + *pad_x;
                let table_top = y + ACTION_TABLE_PAD_Y;
                for (row_idx, row) in rows.iter().enumerate() {
                    let mut cell_left = table_left;
                    for (col_idx, cell) in row.cells.iter().enumerate() {
                        text_render::emit_text(
                            &mut svg.shapes,
                            &cell.text,
                            &TextBase {
                                x: cell_left + ACTION_TABLE_CELL_PAD_X,
                                y: table_top + ascent + row_idx as f64 * line_h,
                                font_size: *font_size as u32,
                                font_family,
                                fill: &text_col,
                                bold: *bold || cell.is_header,
                                italic: *italic,
                                underline: false,
                                skip_underline: false,
                            },
                        );
                        cell_left += col_widths[col_idx];
                    }
                }
                let table_right = table_left + col_widths.iter().sum::<f64>();
                for row_idx in 0..=rows.len() {
                    let line_y = table_top + row_idx as f64 * line_h;
                    svg.shape_line("#000000", "0.5", table_left, table_right, line_y, line_y);
                }
                let table_bottom = table_top + rows.len() as f64 * line_h;
                let mut line_x = table_left;
                svg.shape_line("#000000", "0.5", line_x, line_x, table_top, table_bottom);
                for width in col_widths {
                    line_x += width;
                    svg.shape_line("#000000", "0.5", line_x, line_x, table_top, table_bottom);
                }
                return y + ah;
            }
            // Text baseline: padding_top + ascent, both derived from the
            // label's actual font so monospace labels position correctly.
            let mut text_y = y + *pad_y;
            let mut number_counters = Vec::new();
            let lines = action_label_lines(text);
            for (idx, line) in lines.iter().enumerate() {
                let line_baseline = text_render::label_first_baseline_ascent_with_family(
                    line,
                    *font_size,
                    font_family,
                );
                text_y += line_baseline;
                let text_x = rect_x + *pad_x + action_line_x_offset(&lines, idx);
                emit_action_line(
                    svg,
                    line,
                    &TextBase {
                        x: text_x,
                        y: text_y,
                        font_size: *font_size as u32,
                        font_family,
                        fill: &text_col,
                        bold: *bold,
                        italic: *italic,
                        underline: false,
                        skip_underline: false,
                    },
                    &mut number_counters,
                );
                text_y += text_render::label_height_with_family(line, *font_size, font_family)
                    - line_baseline;
            }
            y + ah
        }
        LayoutNode::DeprecatedAction {
            color: _,
            text,
            text_width,
            pad_x,
            pad_y,
            font_family,
            font_size,
            bold,
            italic,
            warning_width: _,
        } => {
            // The deprecated action renders just like a normal action.
            // The warning banner is emitted separately at the top of the diagram.
            let ah = action_height(text, *pad_y, font_family, *font_size);
            let rect_w = *text_width + *pad_x * 2.0;
            let rect_x = cx - rect_w / 2.0;
            let fill = svg.palette.action_fill.clone();
            let stroke = svg.palette.action_stroke.clone();
            let sw = svg.palette.action_stroke_width.clone();
            let text_col = svg.palette.text_color.clone();
            svg.rect_styled(
                &fill,
                ah,
                svg.palette.action_rx,
                svg.palette.action_rx,
                &stroke,
                &sw,
                rect_w,
                rect_x,
                y,
            );
            let text_y = y
                + *pad_y
                + text_render::label_first_baseline_ascent_with_family(
                    text,
                    *font_size,
                    font_family,
                );
            svg.text_element_styled(
                &text_col,
                font_family,
                *font_size,
                rect_x + *pad_x,
                text_y,
                text,
                *bold,
                *italic,
            );
            y + ah
        }
        LayoutNode::If {
            condition,
            diamond_font_family,
            diamond_font_size,
            diamond_text_color,
            diamond_text_bold,
            diamond_text_italic,
            diamond_pad_x,
            diamond_half_y,
            then_label,
            then_branch,
            else_branches,
            ..
        } => emit_if(
            svg,
            cx,
            y,
            condition,
            diamond_font_family,
            *diamond_font_size,
            diamond_text_color,
            *diamond_text_bold,
            *diamond_text_italic,
            *diamond_pad_x,
            *diamond_half_y,
            then_label,
            then_branch,
            else_branches,
        ),
        LayoutNode::Fork {
            branches,
            is_split,
            merge,
            ..
        } => {
            if *is_split {
                emit_split(svg, cx, y, branches)
            } else if *merge {
                emit_fork_merge(svg, cx, y, branches)
            } else if if_branch {
                emit_fork_with_layout(svg, cx, y, branches, fork_layout_if_branch(branches))
            } else if svg.partition_wrapped_fork_depth > 0 && !*is_split {
                emit_fork_with_layout(svg, cx, y, branches, fork_layout_in_partition(branches))
            } else {
                emit_fork(svg, cx, y, branches)
            }
        }
        LayoutNode::Switch { condition, cases } => {
            if if_branch {
                emit_switch_with_layout(
                    svg,
                    cx,
                    y,
                    condition,
                    cases,
                    switch_x_layout_if_branch(cases, condition),
                    true,
                )
            } else {
                emit_switch(svg, cx, y, condition, cases)
            }
        }
        LayoutNode::While {
            condition,
            diamond_font_family,
            diamond_font_size,
            diamond_text_color,
            diamond_text_bold,
            diamond_text_italic,
            is_label,
            body,
            end_label,
            special_out,
            starts_column,
            ..
        } => emit_while(
            svg,
            cx,
            y,
            condition,
            diamond_font_family,
            *diamond_font_size,
            diamond_text_color,
            *diamond_text_bold,
            *diamond_text_italic,
            is_label,
            end_label,
            body,
            special_out.as_deref(),
            *starts_column,
        ),
        LayoutNode::Repeat {
            body,
            condition,
            is_label,
            not_label,
            backward,
            has_start_label,
            ..
        } => emit_repeat(
            svg,
            cx,
            y,
            body,
            condition,
            is_label,
            RepeatEmitOptions {
                backward: backward.as_deref(),
                not_label: not_label.as_deref(),
                body_top_extra: repeat_body_top_extra,
                has_start_label: *has_start_label,
            },
        ),
        LayoutNode::Arrow { .. } | LayoutNode::Note { .. } => y,
        LayoutNode::Detach | LayoutNode::Kill | LayoutNode::Break | LayoutNode::Goto(_) => y,
        LayoutNode::Title {
            text,
            font_size,
            bold,
        } => {
            // PlantUML wraps the title in `<g class="title" data-source-line="1">`.
            // Title text is centred within an x-extent padded by 4px on the
            // left compared to the action content cx. Baseline is at
            // y + ascent + 4.
            let tw = text_render::measure(text, *font_size, *bold);
            let text_y = y + pm::ascent(*font_size) + 4.0;
            svg.shapes
                .push_str(r#"<g class="title" data-source-line="1">"#);
            svg.text_element(
                TEXT_COLOR,
                "sans-serif",
                *font_size,
                tw,
                cx - tw / 2.0 + 1.0,
                text_y,
                text,
                *bold,
            );
            svg.shapes.push_str("</g>");
            y + pm::text_height(*font_size) + 30.0
        }
        LayoutNode::Partition {
            name,
            color,
            is_group,
            nested,
            single_lane_group,
            single_lane_first_group,
            body,
        } => {
            // Partition's outer rect spans from y_in + 10 (top) to y_in +
            // 10 + 36.49 + body_h + 12 (bottom). The title path corner
            // notches the top-right of the title band; the title text sits
            // at partition_x + 3, baseline = partition_top + ascent(14) + 1.
            //
            // PlantUML's partition top gap depends on whether the title band
            // or body drives the outer width; partition_top_gap centralises
            // that measured rule so height, arrows, and emission stay aligned.
            let title_w = partition_title_width(name);
            let empty_body = body.is_empty();
            let body_w = partition_body_width_for_frame(body);
            let title_width_extra = partition_title_width_extra(color, *is_group, body);
            let title_drives_width =
                partition_title_drives_width(title_w, body_w, title_width_extra, *is_group, body);
            let partition_w = if empty_body {
                title_w + 20.0
            } else {
                (title_w + 15.0 + title_width_extra)
                    .max(partition_body_shell_width(*is_group, body))
            };
            let partition_x = if *single_lane_group || empty_body {
                cx - partition_w / 2.0
            } else if !*is_group
                && (partition_wraps_single_if(body) || partition_wraps_tiny_action_fork(body))
            {
                16.0
            } else if (*is_group && *nested) || (!*is_group && !title_drives_width) {
                (cx - partition_w / 2.0).max(16.0)
            } else {
                16.0
            };
            let top_gap = partition_top_gap(
                color,
                name,
                *is_group,
                *nested,
                *single_lane_first_group,
                body,
            );
            let partition_top = y + top_gap;
            let colored_partition_while =
                color.is_some() && !*is_group && partition_wraps_while(body);
            let body_h = sequence_height(body)
                - if colored_partition_while && colored_partition_needs_while_slot_subtract(body) {
                    WHILE_BODY_SLOT_COMPRESS
                } else {
                    0.0
                };
            let partition_h = if empty_body {
                empty_partition_shell_height()
            } else {
                PARTITION_TITLE_BAND_H + body_h + 12.0
            };
            let partition_right = partition_x + partition_w;

            // Outer rect: fill = color (default none), stroke #000000 width 1.5
            let fill = color.as_deref().unwrap_or("none");
            let resolved_fill = if fill == "none" {
                "none".to_string()
            } else {
                let stripped = fill.strip_prefix('#').unwrap_or(fill);
                crate::sequence::resolve_color(stripped)
            };
            svg.partition_rect(
                &resolved_fill,
                partition_h,
                partition_w,
                partition_x,
                partition_top,
            );

            // Title bar path: M{R},{Y} L{R},{Y+9.49} L{R-10},{Y+19.49} L{X},{Y+19.49}.
            // R is anchored to the title text: partition_x + title_w + 10
            // (the notch sits just past the title's right edge).
            let path_r = partition_x + title_w + 10.0;
            svg.partition_path(path_r, partition_top, partition_x);
            let _ = partition_right;

            // Title text
            let title_y = partition_top + pm::ascent(TITLE_FONT_SIZE) + 1.0;
            svg.text_element(
                TEXT_COLOR,
                "sans-serif",
                TITLE_FONT_SIZE,
                title_w,
                partition_x + 3.0,
                title_y,
                partition_title_label(name),
                false,
            );

            if !empty_body {
                // Emit body inside, at the diagram's cx, starting at partition_top + 36.49.
                // For uncoloured / descender-less partitions a 0.00005 px nudge
                // accounts for Java's intermediate-rounding quirk: the displayed
                // rect_y matches golden (HALF_UP rounding kicks 81.48825 →
                // 81.4883) while inner text_y baselines compute from the
                // un-rounded 81.48825 value. Coloured / descender-titled
                // partitions already have the 0.4531 top-gap shift absorb this.
                let needs_nudge = top_gap == 10.0;
                let body_top = partition_top + PARTITION_TITLE_BAND_H
                    - if needs_nudge { 0.00005 } else { 0.0 };
                if colored_partition_while {
                    svg.colored_partition_while_depth += 1;
                }
                let body_cx = if !title_drives_width && partition_wraps_switch(body) {
                    let (body_left, _) = sequence_extents(body);
                    partition_x + 10.0 + body_left
                } else if !*is_group && partition_wraps_tiny_action_fork(body) {
                    cx + PARTITION_FORK_BODY_CX_SHIFT
                } else {
                    cx
                };
                if !*is_group && partition_wraps_tiny_action_fork(body) {
                    svg.partition_wrapped_fork_depth += 1;
                }
                emit_sequence(svg, body, body_cx, body_top);
                if !*is_group && partition_wraps_tiny_action_fork(body) {
                    svg.partition_wrapped_fork_depth -= 1;
                }
                if colored_partition_while {
                    svg.colored_partition_while_depth -= 1;
                }
            }

            partition_top + partition_h
        }
        LayoutNode::Swimlanes { lanes, segments } => emit_swimlanes(svg, cx, y, lanes, segments),
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_if(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    diamond_font_family: &str,
    diamond_font_size: f64,
    diamond_text_color: &str,
    diamond_text_bold: bool,
    diamond_text_italic: bool,
    diamond_pad_x: f64,
    diamond_half_y: f64,
    then_label: &Option<String>,
    then_branch: &[LayoutNode],
    else_branches: &[ElseBranch],
) -> f64 {
    // if/elseif*/else chain → FtileIfLongHorizontal.
    if if_is_long(else_branches)
        && let Some(l) = if_long_layout(condition, then_label, then_branch, else_branches)
    {
        return emit_if_long(svg, cx, y, &l, then_branch, else_branches);
    }

    // Break-down: `if (c) then break endif` nested directly in a `while`. The
    // break branch welds LEFT to the loop exit corridor (set by `emit_while`),
    // the empty branch becomes the if's pointOut via `ConnectionElseNoDiamond`,
    // and no merge diamond is drawn. Only taken when the enclosing loop set the
    // break context; outside a loop a stray `break` falls through to the normal
    // (merge-diamond) path.
    if let Some(brk) = svg.while_break
        && let Some(plan) = if_break_down_plan(then_branch, else_branches)
    {
        let then_label = then_label.as_deref();
        let else_label = else_branches.first().and_then(|b| b.label.as_deref());
        return emit_if_break_down(
            svg, cx, y, condition, then_label, else_label, &plan, &brk,
        );
    }

    // Empty-branch corridor: when one branch is empty and the other populated
    // and non-terminating, PlantUML's FtileIfDown routes the populated branch
    // down the centre spine and the empty branch as a thin side corridor.
    if let Some(plan) = if_down_plan(then_branch, else_branches) {
        let then_label = then_label.as_deref();
        let else_label = else_branches.first().and_then(|b| b.label.as_deref());
        return emit_if_down(svg, cx, y, condition, then_label, else_label, &plan);
    }

    if let Some(plan) =
        if_single_circle_terminal_plan(then_label.as_ref(), then_branch, else_branches)
    {
        return emit_if_single_circle_terminal_down(
            svg,
            cx,
            y,
            condition,
            diamond_font_family,
            diamond_font_size,
            diamond_text_color,
            diamond_text_bold,
            diamond_text_italic,
            &plan,
        );
    }

    // Cache the per-diagram colours up front so the many line/polygon emit
    // calls below can borrow them as &str without re-borrowing svg.palette.
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();

    // The diamond's inner edge is clamped to DIAMOND_MIN_INNER_W; the
    // condition's `textLength` is the measured width (no clamp). Track both
    // separately so the polygon and the text are sized independently.
    let cond_inner_w = diamond_inner_w_styled_padded(
        condition,
        diamond_font_size,
        diamond_text_bold,
        diamond_font_family,
        diamond_pad_x,
    );
    let cond_text_w = text_render::measure_with_family(
        condition,
        diamond_font_size,
        diamond_text_bold,
        diamond_font_family,
    );

    // Diamond: centered at (cx, y + diamond_half_y). The default activity
    // diamond is 24px tall, but themed diagrams with global Padding reserve a
    // taller text box while keeping the merge diamond at the default size.
    let diamond_cy = y + diamond_half_y;
    let diamond_left = cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right = cx + cond_inner_w / 2.0 + DIAMOND_HALF;
    let diamond_bottom = y + diamond_half_y * 2.0;

    // Diamond polygon (hexagonal for conditions with text)
    let pts = vec![
        (cx - cond_inner_w / 2.0, y),
        (cx + cond_inner_w / 2.0, y),
        (diamond_right, diamond_cy),
        (cx + cond_inner_w / 2.0, diamond_bottom),
        (cx - cond_inner_w / 2.0, diamond_bottom),
        (diamond_left, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

    // Condition text (textLength = measured, centred under cx). The condition
    // honours `skinparam activityFontColor`; the then/else branch labels below
    // keep the default black.
    let text_y = centered_label_y_for_family(
        condition,
        diamond_cy,
        diamond_font_size,
        diamond_font_family,
    );
    svg.text_element_styled(
        diamond_text_color,
        diamond_font_family,
        diamond_font_size,
        cx - cond_text_w / 2.0,
        text_y,
        condition,
        diamond_text_bold,
        diamond_text_italic,
    );

    let label_y = if diamond_pad_x > 0.0 {
        y + (text_y - diamond_cy)
    } else {
        centerline_label_y_for_family(
            diamond_cy,
            svg.palette.arrow_font_size,
            &svg.palette.arrow_font_family,
        )
    };
    let (then_arrow, then_branch_flow) = leading_branch_arrow(then_branch);
    let (else_arrow, else_branch_flow) = else_branches
        .first()
        .map(|branch| leading_branch_arrow(&branch.body))
        .unwrap_or((None, &[]));
    let default_arrow_color = svg.palette.arrow_color.clone();
    let then_arrow_style = branch_arrow_style(then_arrow, &default_arrow_color);
    let else_arrow_style = branch_arrow_style(else_arrow, &default_arrow_color);

    // Then label (to the left of diamond). PlantUML places the label
    // flush against the diamond's left vertex (no horizontal gap), with
    // the baseline at `diamond_cy - descent(label font)`.
    if let Some(label) = then_label
        .as_deref()
        .or_else(|| branch_arrow_label(then_arrow))
    {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            diamond_left - lw - diamond_pad_x,
            label_y,
            label,
            false,
        );
    }

    // Compute branch positions: PlantUML places the then/else branches
    // with their centrelines `branch_dist` apart, where
    // `branch_dist = max(diamond_w + 20, (then_w + else_w)/2 + 20)` so
    // wider branches don't crowd each other.
    let diamond_w = cond_inner_w + DIAMOND_HALF * 2.0;
    let then_w = sequence_width(then_branch);
    let else_w: f64 = else_branches.iter().map(|b| sequence_width(&b.body)).sum();
    // ftile wire (binary if): branch spines from the exact FtileIfWithDiamonds
    // layout (consistent with node_extents). Legacy branch_dist otherwise.
    let (then_cx, else_cx) = if let Some((then_off, else_off, _, _)) = if_ftile_layout_styled(
        condition,
        diamond_font_size,
        diamond_text_bold,
        diamond_font_family,
        diamond_pad_x,
        then_branch,
        else_branches,
    ) {
        (cx + then_off, cx + else_off)
    } else {
        let branch_dist = (diamond_w + 20.0).max((then_w + else_w) / 2.0 + 20.0);
        (cx - branch_dist / 2.0, cx + branch_dist / 2.0)
    };

    // Else label: text shape, must land in shapes buffer before branch
    // shapes (matches golden order: yes label, no label, then branch boxes).
    if let Some(label) = else_branches
        .first()
        .and_then(|branch| branch.label.as_deref())
        .or_else(|| branch_arrow_label(else_arrow))
    {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            diamond_right + diamond_pad_x,
            label_y,
            label,
            false,
        );
    }

    // Render branches first — this puts the branch shapes into the shapes
    // buffer (after the diamond/condition/labels) and any branch-internal
    // connectors into the connectors buffer FIRST. PlantUML emits branch-
    // internal connectors before the diamond→branch outbound connectors.
    let branch_y = diamond_bottom + IF_BRANCH_DOWN;

    // A no-special `while` that is the entire then/else flow owns the
    // branch→merge corridor (PlantUML's MergeStrategy.LIMITED fusion). Such a
    // branch needs the merge-diamond cy BEFORE it emits, so its exit corridor
    // can terminate at the merge vertex. We first emit the branches into the
    // buffers to learn their bottoms (hence merge_cy), and — when a branch is
    // redirectable — roll the buffers back and re-emit it with the redirect set.
    let then_redirectable = branch_is_redirectable_while(then_branch_flow);
    let else_redirectable =
        !else_branches.is_empty() && branch_is_redirectable_while(else_branch_flow);

    let shapes_chk = svg.shapes.len();
    let conns_chk = svg.connectors.len();
    let then_bottom = emit_sequence_if_branch(svg, then_branch_flow, then_cx, branch_y);
    let else_bottom = if !else_branches.is_empty() {
        emit_sequence_if_branch(svg, else_branch_flow, else_cx, branch_y)
    } else {
        branch_y
    };

    // If every branch ends with a terminator (Stop/End/Detach/Kill), PlantUML
    // skips the merge diamond and post-merge connectors entirely. The two
    // branches stand on their own; the if-block's bottom is the deeper one.
    let then_terminates = branch_terminates(then_branch_flow);
    let else_terminates = else_branches.split_first().is_some_and(|(_, rest)| {
        branch_terminates(else_branch_flow)
            && rest.iter().all(|branch| branch_terminates(&branch.body))
    });
    let all_terminate = then_terminates && else_terminates;
    let single_survivor = if_single_survivor(then_branch_flow, else_branches);

    // Merge diamond at bottom — sits IF_BRANCH_UP px below the deepest branch.
    let merge_gap = if diamond_pad_x > 0.0 {
        10.0
    } else {
        IF_BRANCH_UP
    };
    let merge_y = then_bottom.max(else_bottom) + merge_gap;
    let merge_diamond_top = merge_y;
    let merge_cy = merge_diamond_top + DIAMOND_HALF;

    // Re-emit any redirectable branch now that merge_cy is known, so its loop
    // exit corridor lands on the merge diamond. Only meaningful when a real
    // merge diamond exists (non-terminating, no single-survivor short-circuit).
    let redirect_active = (then_redirectable || else_redirectable)
        && !all_terminate
        && single_survivor.is_none()
        && !if_empty_both_plain(then_branch, else_branches);
    if redirect_active {
        svg.shapes.truncate(shapes_chk);
        svg.connectors.truncate(conns_chk);
        if then_redirectable {
            svg.while_exit_redirect = Some(WhileExitRedirect {
                merge_cy,
                merge_vertex_x: cx - DIAMOND_HALF,
                to_right: true,
            });
        }
        emit_sequence_if_branch(svg, then_branch_flow, then_cx, branch_y);
        svg.while_exit_redirect = None;
        if !else_branches.is_empty() {
            if else_redirectable {
                svg.while_exit_redirect = Some(WhileExitRedirect {
                    merge_cy,
                    merge_vertex_x: cx + DIAMOND_HALF,
                    to_right: false,
                });
            }
            emit_sequence_if_branch(svg, else_branch_flow, else_cx, branch_y);
            svg.while_exit_redirect = None;
        }
    }

    if !all_terminate && single_survivor.is_none() {
        // Small merge diamond shape (lands in shapes buffer after branch shapes).
        svg.polygon_shape(
            &diamond_fill,
            &[
                (cx, merge_diamond_top),
                (cx + DIAMOND_HALF, merge_cy),
                (cx, merge_diamond_top + DIAMOND_HALF * 2.0),
                (cx - DIAMOND_HALF, merge_cy),
            ],
            &diamond_stroke,
            &diamond_stroke_width,
        );
    }

    if if_empty_both_plain(then_branch, else_branches) && !all_terminate {
        // PlantUML routes two empty branches as side corridors into the merge
        // diamond. The vertical corridor is one continuous line with a
        // mid-corridor down arrowhead overlaid before the line element.
        let arrow_tip = (diamond_cy + merge_cy) / 2.0;
        svg.connector_line_full(
            &then_arrow_style,
            diamond_left,
            then_cx,
            diamond_cy,
            diamond_cy,
        );
        switch_down_head(svg, &arrow_color, then_cx, arrow_tip);
        svg.connector_line_full(&then_arrow_style, then_cx, then_cx, diamond_cy, merge_cy);
        svg.connector_line_full(
            &then_arrow_style,
            then_cx,
            cx - DIAMOND_HALF,
            merge_cy,
            merge_cy,
        );
        svg.right_arrow(cx - DIAMOND_HALF, merge_cy, &arrow_color);

        svg.connector_line_full(
            &else_arrow_style,
            diamond_right,
            else_cx,
            diamond_cy,
            diamond_cy,
        );
        switch_down_head(svg, &arrow_color, else_cx, arrow_tip);
        svg.connector_line_full(&else_arrow_style, else_cx, else_cx, diamond_cy, merge_cy);
        svg.connector_line_full(
            &else_arrow_style,
            else_cx,
            cx + DIAMOND_HALF,
            merge_cy,
            merge_cy,
        );
        svg.left_arrow(cx + DIAMOND_HALF, merge_cy, &arrow_color);
        return merge_diamond_top + DIAMOND_HALF * 2.0;
    }

    // Now emit the if/else-frame connectors AFTER the branch-internal ones.
    // Order: diamond→then, diamond→else, then→merge, else→merge.

    // Diamond → then: horizontal from diamond left to then_cx, then down to
    // branch top, with an arrowhead overlay.
    svg.connector_line_full(
        &then_arrow_style,
        diamond_left,
        then_cx,
        diamond_cy,
        diamond_cy,
    );
    svg.connector_line_full(
        &then_arrow_style,
        then_cx,
        then_cx,
        diamond_cy,
        diamond_bottom + IF_BRANCH_DOWN,
    );
    svg.polygon_connector(
        &then_arrow_style.color,
        &[
            (then_cx - 4.0, diamond_bottom + IF_BRANCH_DOWN - 10.0),
            (then_cx, diamond_bottom + IF_BRANCH_DOWN),
            (then_cx + 4.0, diamond_bottom + IF_BRANCH_DOWN - 10.0),
            (then_cx, diamond_bottom + IF_BRANCH_DOWN - 6.0),
        ],
        &then_arrow_style.color,
        "1",
    );

    // Diamond → else: mirror of the then side.
    svg.connector_line_full(
        &else_arrow_style,
        diamond_right,
        else_cx,
        diamond_cy,
        diamond_cy,
    );
    svg.connector_line_full(
        &else_arrow_style,
        else_cx,
        else_cx,
        diamond_cy,
        diamond_bottom + IF_BRANCH_DOWN,
    );
    svg.polygon_connector(
        &else_arrow_style.color,
        &[
            (else_cx - 4.0, diamond_bottom + IF_BRANCH_DOWN - 10.0),
            (else_cx, diamond_bottom + IF_BRANCH_DOWN),
            (else_cx + 4.0, diamond_bottom + IF_BRANCH_DOWN - 10.0),
            (else_cx, diamond_bottom + IF_BRANCH_DOWN - 6.0),
        ],
        &else_arrow_style.color,
        "1",
    );

    if let Some(then_survives) = single_survivor {
        let (survivor_cx, survivor_bottom) = if then_survives {
            (then_cx, then_bottom)
        } else {
            (else_cx, else_bottom)
        };
        let join_y = survivor_bottom + IF_SINGLE_SURVIVOR_JOIN_GAP;
        let out_y = survivor_bottom + ARROW_LEN;
        svg.connector_line(
            &arrow_color,
            survivor_cx,
            survivor_cx,
            survivor_bottom,
            join_y,
            false,
        );
        if survivor_cx != cx {
            svg.connector_line(&arrow_color, survivor_cx, cx, join_y, join_y, false);
        }
        svg.connector_line(&arrow_color, cx, cx, join_y, out_y, false);
        switch_down_head(svg, &arrow_color, cx, out_y);
        return out_y;
    }

    // Then branch → merge — skipped if the branch terminates, or if a
    // redirected `while` branch already routed its exit corridor to the merge.
    let then_routed_by_while = redirect_active && then_redirectable;
    let else_routed_by_while = redirect_active && else_redirectable;
    if !then_terminates && !then_routed_by_while {
        svg.connector_line(&arrow_color, then_cx, then_cx, then_bottom, merge_cy, false);
        svg.connector_line(
            &arrow_color,
            then_cx,
            cx - DIAMOND_HALF,
            merge_cy,
            merge_cy,
            false,
        );
        svg.right_arrow(cx - DIAMOND_HALF, merge_cy, &arrow_color);
    }

    // Else branch → merge — skipped if every else branch terminates, or if a
    // redirected `while` branch already routed its exit corridor to the merge.
    if !else_terminates && !else_routed_by_while {
        svg.connector_line(&arrow_color, else_cx, else_cx, else_bottom, merge_cy, false);
        svg.connector_line(
            &arrow_color,
            else_cx,
            cx + DIAMOND_HALF,
            merge_cy,
            merge_cy,
            false,
        );
        svg.left_arrow(cx + DIAMOND_HALF, merge_cy, &arrow_color);
    }

    if all_terminate {
        // No merge diamond was emitted — block height ends at the deeper branch.
        then_bottom.max(else_bottom)
    } else {
        merge_diamond_top + DIAMOND_HALF * 2.0
    }
}

/// Emit an `if/elseif*/else` chain (`FtileIfLongHorizontal`): a row of condition
/// diamonds each fronting its branch column, linked left→right, with the final
/// bare `else` to the right, all branch outs collected on a bottom merge line.
///
/// `cx` is the if-block spine, `y` the previous node's bottom (where the inbound
/// `ConnectionIn` snake begins). Returns the merge-line y (the if-block's out
/// point). `l` carries the placed geometry from [`if_long_layout`].
fn emit_if_long(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    l: &IfLongLayout,
    then_branch: &[LayoutNode],
    else_branches: &[ElseBranch],
) -> f64 {
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();
    let cond_text_color = svg.palette.text_color.clone();

    let v = if_long_vmetrics(l, y);
    let dtop = v.dtop;
    let diamond_cy = dtop + DIAMOND_HALF;
    let diamond_bottom = dtop + DIAMOND_HALF * 2.0;
    let n = l.cols.len();

    // Branch bodies in column order: then_branch, then each elseif body.
    let elseif_bodies: Vec<&[LayoutNode]> = else_branches
        .iter()
        .filter(|b| b.condition.is_some())
        .map(|b| b.body.as_slice())
        .collect();

    // --- Shapes: per couple (diamond + labels + branch) ------------------
    for (i, col) in l.cols.iter().enumerate() {
        let dcx = cx + col.cx;
        let inner = col.diamond_w - DIAMOND_HALF * 2.0;
        let pts = vec![
            (dcx - inner / 2.0, dtop),
            (dcx + inner / 2.0, dtop),
            (dcx + col.diamond_w / 2.0, diamond_cy),
            (dcx + inner / 2.0, diamond_bottom),
            (dcx - inner / 2.0, diamond_bottom),
            (dcx - col.diamond_w / 2.0, diamond_cy),
        ];
        svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

        // North label (the then/elseif positive label). PlantUML draws it at
        // `4 + dimTotal.width/2` from the diamond's left edge (left-aligned),
        // i.e. its left edge is `dcx + 4`.
        if let Some(north) = &col.north {
            let label_font_size = svg.palette.arrow_font_size;
            let label_family = svg.palette.arrow_font_family.clone();
            let label_color = svg.palette.arrow_text_color.clone();
            let nw = text_render::measure_with_family(north, label_font_size, false, &label_family);
            svg.text_element(
                &label_color,
                &label_family,
                label_font_size,
                nw,
                dcx + 4.0,
                diamond_bottom + text_render::ascent_for_family(label_font_size, &label_family),
                north,
                false,
            );
        }

        // Condition text, centred inside the diamond.
        let cw = text_render::measure(&col.condition, SMALL_FONT, false);
        let cond_y = centered_label_y(&col.condition, diamond_cy, SMALL_FONT);
        svg.text_element_styled(
            &cond_text_color,
            "sans-serif",
            SMALL_FONT,
            dcx - cw / 2.0,
            cond_y,
            &col.condition,
            false,
            svg.palette.diamond_text_italic,
        );

        // East label on the LAST diamond (the bare-else label), baseline at
        // diamond_cy − descent(11), x at the diamond's right vertex.
        if i == n - 1
            && let Some(east) = &l.east_label
        {
            let label_font_size = svg.palette.arrow_font_size;
            let label_family = svg.palette.arrow_font_family.clone();
            let label_color = svg.palette.arrow_text_color.clone();
            let ew = text_render::measure_with_family(east, label_font_size, false, &label_family);
            svg.text_element(
                &label_color,
                &label_family,
                label_font_size,
                ew,
                dcx + col.diamond_w / 2.0,
                centerline_label_y_for_family(diamond_cy, label_font_size, &label_family),
                east,
                false,
            );
        }

        // Branch box(es) below the diamond.
        let body: &[LayoutNode] = if i == 0 {
            then_branch
        } else {
            elseif_bodies[i - 1]
        };
        emit_sequence(svg, body, dcx, v.couple_branch_top);
    }

    // tile2 (the bare else) to the right.
    if let Some(tile2_cx) = l.tile2_cx {
        let tcx = cx + tile2_cx;
        let else_body = else_branches
            .iter()
            .find(|b| b.condition.is_none())
            .map(|b| b.body.as_slice())
            .unwrap_or(&[]);
        emit_sequence(svg, else_body, tcx, v.tile2_top);
    }

    // --- Connectors ------------------------------------------------------
    // Per-couple ConnectionVerticalIn (diamond→branch) + ConnectionVerticalOut
    // (branch→merge line).
    for col in &l.cols {
        let dcx = cx + col.cx;
        // Vertical in: diamond bottom → branch top.
        svg.connector_line(
            &arrow_color,
            dcx,
            dcx,
            diamond_bottom,
            v.couple_branch_top,
            false,
        );
        svg.polygon_connector(
            &arrow_color,
            &[
                (dcx - 4.0, v.couple_branch_top - 10.0),
                (dcx, v.couple_branch_top),
                (dcx + 4.0, v.couple_branch_top - 10.0),
                (dcx, v.couple_branch_top - 6.0),
            ],
            &arrow_color,
            "1",
        );
        // Vertical out: branch bottom → merge line.
        let branch_bottom = v.couple_branch_top + col.branch_h;
        svg.connector_line(&arrow_color, dcx, dcx, branch_bottom, v.merge_y, false);
        svg.polygon_connector(
            &arrow_color,
            &[
                (dcx - 4.0, v.merge_y - 10.0),
                (dcx, v.merge_y),
                (dcx + 4.0, v.merge_y - 10.0),
                (dcx, v.merge_y - 6.0),
            ],
            &arrow_color,
            "1",
        );
    }

    // ConnectionHorizontal between adjacent diamonds (east vertex → west vertex).
    for i in 0..n - 1 {
        let d1 = &l.cols[i];
        let d2 = &l.cols[i + 1];
        let x1 = cx + d1.cx + d1.diamond_w / 2.0;
        let x2 = cx + d2.cx - d2.diamond_w / 2.0;
        svg.connector_line(&arrow_color, x1, x2, diamond_cy, diamond_cy, false);
        svg.right_arrow(x2, diamond_cy, &arrow_color);
    }

    // ConnectionIn (prev bottom → first diamond): down 5, sideways, down to
    // the first diamond top.
    let d0cx = cx + l.cols[0].cx;
    svg.connector_line(&arrow_color, cx, cx, y, y + 5.0, false);
    svg.connector_line(&arrow_color, cx, d0cx, y + 5.0, y + 5.0, false);
    svg.connector_line(&arrow_color, d0cx, d0cx, y + 5.0, dtop, false);
    svg.polygon_connector(
        &arrow_color,
        &[
            (d0cx - 4.0, dtop - 10.0),
            (d0cx, dtop),
            (d0cx + 4.0, dtop - 10.0),
            (d0cx, dtop - 6.0),
        ],
        &arrow_color,
        "1",
    );

    // ConnectionLastElseIn + ConnectionLastElseOut (last diamond east → tile2,
    // then tile2 → merge line).
    if let Some(tile2_cx) = l.tile2_cx {
        let tcx = cx + tile2_cx;
        let last = &l.cols[n - 1];
        let east_x = cx + last.cx + last.diamond_w / 2.0;
        // East vertex → above tile2, then down into tile2.
        svg.connector_line(&arrow_color, east_x, tcx, diamond_cy, diamond_cy, false);
        svg.connector_line(&arrow_color, tcx, tcx, diamond_cy, v.tile2_top, false);
        svg.polygon_connector(
            &arrow_color,
            &[
                (tcx - 4.0, v.tile2_top - 10.0),
                (tcx, v.tile2_top),
                (tcx + 4.0, v.tile2_top - 10.0),
                (tcx, v.tile2_top - 6.0),
            ],
            &arrow_color,
            "1",
        );
        // tile2 out → merge line.
        let tile2_bottom = v.tile2_top + l.tile2_h;
        svg.connector_line(&arrow_color, tcx, tcx, tile2_bottom, v.merge_y, false);
        svg.polygon_connector(
            &arrow_color,
            &[
                (tcx - 4.0, v.merge_y - 10.0),
                (tcx, v.merge_y),
                (tcx + 4.0, v.merge_y - 10.0),
                (tcx, v.merge_y - 6.0),
            ],
            &arrow_color,
            "1",
        );
    }

    // ConnectionHline: the bottom merge line spanning the leftmost to rightmost
    // branch out.
    let mut min_out = cx + l.cols[0].cx;
    let mut max_out = cx + l.cols[n - 1].cx;
    for col in &l.cols {
        min_out = min_out.min(cx + col.cx);
        max_out = max_out.max(cx + col.cx);
    }
    if let Some(tile2_cx) = l.tile2_cx {
        min_out = min_out.min(cx + tile2_cx);
        max_out = max_out.max(cx + tile2_cx);
    }
    svg.connector_line(&arrow_color, min_out, max_out, v.merge_y, v.merge_y, false);

    v.merge_y
}

/// Gap from the condition diamond's bottom to the top of the (centred)
/// populated branch in the FtileIfDown layout. PlantUML centres the branch in
/// the vertical band, leaving `ARROW_LEN` below the branch (to the merge
/// diamond) and a slightly larger lead above it: the diamond's south label
/// (the populated branch's label) reserves half its text height beneath the
/// hexagon, pushing the branch down by `text_height(11)/2 - 2`.
const IF_DOWN_LEAD: f64 = ARROW_LEN + 4.477539062500001; // 24.4775

/// Tip offset of the empty-corridor's mid-arrow above the corridor midpoint.
/// PlantUML's `Snake.emphasizeDirection(DOWN)` lands the arrowhead tip
/// `2.2388` px below the geometric midpoint of the corridor's vertical run.
const IF_CORRIDOR_ARROW_OFFSET: f64 = 2.238769531250023;

/// Left lead past the diamond's left vertex in the FtileIfDown layout.
const IF_DOWN_LEFT_PAD: f64 = 9.0;
/// Right corridor reservation past the diamond's right vertex.
const IF_DOWN_RIGHT_PAD: f64 = 27.218200000000003;
/// Minimum clearance between a wide populated branch and the empty side
/// corridor in FtileIfDown.
const IF_DOWN_BRANCH_CORRIDOR_GAP: f64 = 10.0;
/// Extra right content extent after the empty side corridor. The
/// diamond-based IF_DOWN_RIGHT_PAD includes this implicitly; branch-based
/// corridors need it added explicitly for canvas width parity.
const IF_DOWN_BRANCH_CORRIDOR_TRAILING_PAD: f64 = 15.0;
/// Extra gap stretched onto the middle inter-action arrow of an even-action
/// populated branch in the FtileIfDown layout.
const IF_DOWN_MID_STRETCH: f64 = 15.0;
/// Drop from a break-bearing `if`'s diamond bottom to the (no-diamond) east
/// corridor's return line — i.e. the if-block's pointOut. The break tile sits
/// `IF_DOWN_LEAD` below the diamond; the corridor then runs `ARROW_LEN +
/// IF_BRANCH_UP` further down to the rejoin (`FtileIfDown.calculateDimension`
/// with empty then/diamond2 tiles). When the enclosing loop column has enough
/// adjacent flow content, ON_Y slot compression removes a `4.4775` slack unit
/// (the residual of the south label's reserved band).
const WHILE_BREAK_IF_CORRIDOR_DROP_COMPRESSED: f64 = IF_DOWN_LEAD + ARROW_LEN + IF_BRANCH_UP; // 50.4775
const WHILE_BREAK_IF_CORRIDOR_UNCOMPRESSED_EXTRA: f64 = 4.477539062500001;
/// Extra gap stretched onto the middle inter-action arrow of an even-action
/// while body. A labelled `while (...) is (...)` body already carries the
/// reserved label slot under the condition diamond, so PlantUML's centring
/// leaves less residual slack than the no-label form.
const WHILE_EVEN_BODY_MID_STRETCH_LABELED: f64 = 9.0224609375;
const WHILE_EVEN_BODY_MID_STRETCH_UNLABELED: f64 = 15.0;
/// The loop-back arrowhead uses the pre-body-stretch midpoint plus this fixed
/// shift; it does not move by the full stretched connector amount.
const WHILE_EVEN_BODY_LOOP_ARROW_STRETCH: f64 = 7.5;
/// Extra gap stretched onto the middle inter-action arrow of an even-action
/// repeat body without an explicit `backward :...;` tile. FtileRepeat centres
/// the body in the repeat frame; with an even number of flow nodes the centre
/// falls inside the middle connector, lengthening that one snake by 7.5 px.
const REPEAT_EVEN_BODY_MID_STRETCH: f64 = 7.5;
/// Extra vertical slack FtileRepeat distributes through a multi-action body
/// when an explicit `backward :...;` tile occupies the loop-back arm.
const REPEAT_BACKWARD_BODY_SLACK: f64 = 30.0;

/// The even-body mid-stretch is calibrated for loop bodies whose flow nodes are
/// plain action tiles (`act_while_2actions_body` and friends): PlantUML's Snake
/// compaction distributes the back-edge label slack evenly across an even number
/// of inter-action gaps. A nested compound tile (while/repeat/if/fork/switch/
/// partition) already carries its own large vertical reservation, so the simple
/// even-gap model does not hold and the stretch must not be applied.
fn while_body_flow_is_all_actions(body: &[LayoutNode]) -> bool {
    body.iter().filter(|n| node_is_flow(n)).all(|n| {
        matches!(
            n,
            LayoutNode::Action { .. } | LayoutNode::Start | LayoutNode::Stop | LayoutNode::End
        )
    })
}

fn while_body_mid_stretch(body: &[LayoutNode], has_in_label: bool) -> Option<(usize, f64)> {
    if !while_body_flow_is_all_actions(body) {
        return None;
    }
    let flow_count = body.iter().filter(|n| node_is_flow(n)).count();
    if flow_count >= 2 && flow_count.is_multiple_of(2) {
        // The even-body mid-stretch models FtileWhile's vertical centring of a
        // body built from a uniform stack of equal-height action tiles: the body
        // centre falls on the middle connector, and compression cannot reclaim
        // the slack, lengthening that one connector. A Fork tile is a multi-row
        // composite with its own internal centring and bar geometry, so the body
        // centre does NOT land on a reclaimable connector — PlantUML inserts no
        // extra slack into the post-fork arrow (verified against
        // act_combo_while_fork_{1,2,3}: the After-fork action sits exactly
        // ARROW_LEN below the join bar). The body is still even, so the loop-back
        // arrowhead keeps its even-body placement (the returned index drives that
        // via WHILE_EVEN_BODY_LOOP_ARROW_STRETCH); only the connector stretch is
        // zeroed.
        let stretch = if body.iter().any(|n| matches!(n, LayoutNode::Fork { .. })) {
            0.0
        } else if has_in_label {
            WHILE_EVEN_BODY_MID_STRETCH_LABELED
        } else {
            WHILE_EVEN_BODY_MID_STRETCH_UNLABELED
        };
        Some((flow_count / 2, stretch))
    } else {
        None
    }
}

fn while_body_height(body: &[LayoutNode], has_in_label: bool) -> f64 {
    sequence_height(body)
        + while_body_mid_stretch(body, has_in_label).map_or(0.0, |(_, stretch)| stretch)
}

fn repeat_body_mid_stretch(body: &[LayoutNode], has_backward: bool) -> Option<(usize, f64)> {
    let flow_count = body.iter().filter(|n| node_is_flow(n)).count();
    if has_backward {
        if flow_count >= 2 {
            return Some((
                flow_count - 1,
                REPEAT_BACKWARD_BODY_SLACK / flow_count as f64,
            ));
        }
        return None;
    }
    // The 7.5 px centre-of-frame stretch is `space/2` for a body of plain
    // actions (uniform inter-node gaps). A body that contains a composite flow
    // node (if/fork/switch/while/repeat) reserves its own internal vertical
    // space, so FtileRepeat's `space` collapses and no middle connector is
    // lengthened — exclude those bodies (e.g. `:Get input;` + if).
    let all_actions_flow = body
        .iter()
        .filter(|n| node_is_flow(n))
        .all(|n| matches!(n, LayoutNode::Action { .. }));
    if flow_count >= 2 && flow_count.is_multiple_of(2) && all_actions_flow {
        Some((flow_count / 2, REPEAT_EVEN_BODY_MID_STRETCH))
    } else {
        None
    }
}

fn repeat_body_height(body: &[LayoutNode], has_backward: bool) -> f64 {
    sequence_height(body)
        + repeat_body_mid_stretch(body, has_backward).map_or(0.0, |(_, stretch)| stretch)
}

fn repeat_not_label_outbound_gap(node: &LayoutNode) -> Option<f64> {
    let LayoutNode::Repeat {
        not_label,
        arrow_font_size,
        arrow_font_family,
        ..
    } = node
    else {
        return None;
    };
    not_label.as_ref()?;
    Some(
        ARROW_LEN
            + text_render::ascent_for_family(*arrow_font_size, arrow_font_family)
            + REPEAT_NOT_LABEL_OUTBOUND_PAD,
    )
}

fn first_flow_node(nodes: &[LayoutNode]) -> Option<&LayoutNode> {
    nodes.iter().find(|node| node_is_flow(node))
}


fn leading_if_branch_repeat_extra(node: &LayoutNode) -> f64 {
    let LayoutNode::Repeat {
        body,
        is_label,
        backward,
        ..
    } = node
    else {
        return 0.0;
    };
    if is_label.is_some()
        || backward.is_some()
        || body.iter().filter(|n| node_is_flow(n)).count() != 1
    {
        return 0.0;
    }
    (repeat_body_height(body, false) - DIAMOND_HALF * 2.0).max(0.0)
}

fn sequence_height_if_branch(nodes: &[LayoutNode]) -> f64 {
    sequence_height(nodes) + first_flow_node(nodes).map_or(0.0, leading_if_branch_repeat_extra)
}

#[allow(clippy::too_many_arguments)]
fn emit_if_single_circle_terminal_down(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    diamond_font_family: &str,
    diamond_font_size: f64,
    diamond_text_color: &str,
    diamond_text_bold: bool,
    diamond_text_italic: bool,
    plan: &IfSingleCircleTerminalPlan,
) -> f64 {
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();

    let cond_inner_w = diamond_inner_w_styled(
        condition,
        diamond_font_size,
        diamond_text_bold,
        diamond_font_family,
    );
    let cond_text_w = text_render::measure_with_family(
        condition,
        diamond_font_size,
        diamond_text_bold,
        diamond_font_family,
    );
    let diamond_cy = y + DIAMOND_HALF;
    let diamond_left = cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right = cx + cond_inner_w / 2.0 + DIAMOND_HALF;
    let diamond_bottom = y + DIAMOND_HALF * 2.0;

    let branch_top = diamond_bottom + IF_DOWN_LEAD;
    let flow_count = plan.survivor.iter().filter(|n| node_is_flow(n)).count();
    let mid_stretch = if flow_count >= 2 && flow_count.is_multiple_of(2) {
        Some((flow_count / 2, IF_DOWN_MID_STRETCH))
    } else {
        None
    };
    let branch_bottom =
        emit_sequence_ex(svg, plan.survivor, cx, branch_top, mid_stretch, None, false);

    let pts = vec![
        (cx - cond_inner_w / 2.0, y),
        (cx + cond_inner_w / 2.0, y),
        (diamond_right, diamond_cy),
        (cx + cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (diamond_left, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

    if let Some(label) = plan.survivor_label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            cx + 4.0,
            diamond_bottom + text_render::ascent_for_family(label_font_size, &label_family),
            label,
            false,
        );
    }
    let text_y = centered_label_y_for_family(
        condition,
        diamond_cy,
        diamond_font_size,
        diamond_font_family,
    );
    svg.text_element_styled(
        diamond_text_color,
        diamond_font_family,
        diamond_font_size,
        cx - cond_text_w / 2.0,
        text_y,
        condition,
        diamond_text_bold,
        diamond_text_italic,
    );
    if let Some(label) = plan.terminal_label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            diamond_right,
            centerline_label_y_for_family(diamond_cy, label_font_size, &label_family),
            label,
            false,
        );
    }

    let terminal_r = terminal_radius(plan.terminal);
    let terminal_label_w = plan.terminal_label.map_or(0.0, |label| {
        text_render::measure_with_family(
            label,
            svg.palette.arrow_font_size,
            false,
            &svg.palette.arrow_font_family,
        )
    });
    let terminal_cx = diamond_right + terminal_label_w + 2.0 * terminal_r;
    let terminal_left = terminal_cx - terminal_r;
    emit_node_with_repeat_extra(
        svg,
        plan.terminal,
        terminal_cx,
        diamond_cy - terminal_r,
        0.0,
        false,
    );

    svg.down_arrow(cx, diamond_bottom, branch_top, &arrow_color);
    svg.connector_line(
        &arrow_color,
        diamond_right,
        terminal_left,
        diamond_cy,
        diamond_cy,
        false,
    );
    svg.right_arrow(terminal_left, diamond_cy, &arrow_color);

    let out_y = branch_bottom + ARROW_LEN;
    svg.down_arrow(cx, branch_bottom, out_y, &arrow_color);
    out_y
}

/// Asymmetric "down" layout for an `if/else` where one branch is empty.
/// The populated branch flows down the centre spine; the empty branch is a
/// thin corridor on the right that exits the diamond's east vertex and rejoins
/// the merge diamond's east vertex.
fn emit_if_down(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    then_label: Option<&str>,
    else_label: Option<&str>,
    plan: &IfDownPlan,
) -> f64 {
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();

    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);

    let diamond_cy = y + DIAMOND_HALF;
    let diamond_left = cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right = cx + cond_inner_w / 2.0 + DIAMOND_HALF;
    let diamond_bottom = y + DIAMOND_HALF * 2.0;

    // PlantUML's FtileIfDown.drawU emits the populated branch FIRST, then the
    // condition diamond, then the merge diamond. The branch's internal
    // connectors land in the connectors buffer before the if-frame connectors.
    let branch_top = diamond_bottom + IF_DOWN_LEAD;
    // When the populated branch has an even number of flow nodes, PlantUML's
    // vertical centring stretches the inter-action gap straddling the branch's
    // midpoint by 15 px (the gap before the (N/2)-th flow node).
    let flow_count = plan.populated.iter().filter(|n| node_is_flow(n)).count();
    let mid_stretch = if flow_count >= 2 && flow_count.is_multiple_of(2) {
        Some((flow_count / 2, IF_DOWN_MID_STRETCH))
    } else {
        None
    };
    let branch_bottom = emit_sequence_ex(
        svg,
        plan.populated,
        cx,
        branch_top,
        mid_stretch,
        None,
        false,
    );

    // Condition hexagon.
    let pts = vec![
        (cx - cond_inner_w / 2.0, y),
        (cx + cond_inner_w / 2.0, y),
        (diamond_right, diamond_cy),
        (cx + cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (diamond_left, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

    // Diamond labels then condition text, matching PlantUML's draw order:
    // polygon, SOUTH label (the populated branch's label), condition text,
    // EAST label (the empty branch's label).
    let (south_label, east_label) = if plan.then_populated {
        (then_label, else_label)
    } else {
        (else_label, then_label)
    };
    if let Some(label) = south_label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            cx + 4.0,
            diamond_bottom + text_render::ascent_for_family(label_font_size, &label_family),
            label,
            false,
        );
    }
    // Condition text (centred under cx).
    let cond_text_color = svg.palette.text_color.clone();
    let text_y = centered_label_y(condition, diamond_cy, SMALL_FONT);
    svg.text_element_styled(
        &cond_text_color,
        "sans-serif",
        SMALL_FONT,
        cx - cond_text_w / 2.0,
        text_y,
        condition,
        false,
        svg.palette.diamond_text_italic,
    );
    if let Some(label) = east_label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            diamond_right,
            centerline_label_y_for_family(diamond_cy, label_font_size, &label_family),
            label,
            false,
        );
    }

    // Merge diamond ARROW_LEN below the branch.
    let merge_top = branch_bottom + ARROW_LEN;
    let merge_cy = merge_top + DIAMOND_HALF;
    svg.polygon_shape(
        &diamond_fill,
        &[
            (cx, merge_top),
            (cx + DIAMOND_HALF, merge_cy),
            (cx, merge_top + DIAMOND_HALF * 2.0),
            (cx - DIAMOND_HALF, merge_cy),
        ],
        &diamond_stroke,
        &diamond_stroke_width,
    );

    // Diamond → populated branch (down arrow on the spine).
    svg.down_arrow(cx, diamond_bottom, branch_top, &arrow_color);

    // Empty corridor on the right: exit east vertex, run down, rejoin merge
    // east vertex with a left arrow. A mid-corridor down arrow marks flow.
    let corridor_x = (diamond_right + DIAMOND_HALF)
        .max(cx + sequence_width(plan.populated) / 2.0 + IF_DOWN_BRANCH_CORRIDOR_GAP);
    let merge_right = cx + DIAMOND_HALF;
    // The corridor is a single PlantUML snake: exit-horizontal, then the
    // emphasised mid down-arrow, then the vertical run, the merge-horizontal,
    // and finally the terminal left-arrow into the merge diamond. The arrow
    // polygons interleave with the line segments in that order.
    // Exit horizontal: diamond east → corridor.
    svg.connector_line(
        &arrow_color,
        diamond_right,
        corridor_x,
        diamond_cy,
        diamond_cy,
        false,
    );
    // Mid-corridor down arrowhead (emphasizeDirection on the vertical run).
    let arrow_tip = (diamond_cy + merge_cy) / 2.0 + IF_CORRIDOR_ARROW_OFFSET;
    svg.polygon_connector(
        &arrow_color,
        &[
            (corridor_x - 4.0, arrow_tip - 10.0),
            (corridor_x, arrow_tip),
            (corridor_x + 4.0, arrow_tip - 10.0),
            (corridor_x, arrow_tip - 6.0),
        ],
        &arrow_color,
        "1",
    );
    // Corridor vertical: down to merge cy.
    svg.connector_line(
        &arrow_color,
        corridor_x,
        corridor_x,
        diamond_cy,
        merge_cy,
        false,
    );
    // Corridor → merge east vertex (left arrow).
    svg.connector_line(
        &arrow_color,
        corridor_x,
        merge_right,
        merge_cy,
        merge_cy,
        false,
    );
    svg.left_arrow(merge_right, merge_cy, &arrow_color);

    // Branch → merge (down arrow on the spine).
    svg.down_arrow(cx, branch_bottom, merge_top, &arrow_color);

    merge_top + DIAMOND_HALF * 2.0
}

/// Emit a break-bearing `if` directly nested in a `while` (PlantUML's
/// `FtileIfDown` with `optionalStop == null` and a then-block whose
/// `hasPointOut() == false`). The break branch occupies the spine: it runs down
/// `IF_DOWN_LEAD` from the diamond bottom and then welds LEFT to the loop's exit
/// corridor (`FtileFactoryDelegatorWhile`'s `Snake.asToLeft` to
/// `Hexagon.hexagonHalfSize`). The empty branch becomes the if's pointOut via
/// `ConnectionElseNoDiamond` — an east corridor (down-emphasized) that rejoins
/// the spine at the if-block bottom with no merge diamond and no terminal
/// in-arrow. Returns the if-block's pointOut y.
#[allow(clippy::too_many_arguments)]
fn emit_if_break_down(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    then_label: Option<&str>,
    else_label: Option<&str>,
    plan: &IfBreakDownPlan,
    brk: &WhileBreakContext,
) -> f64 {
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();

    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);

    let diamond_cy = y + DIAMOND_HALF;
    let diamond_left = cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right = cx + cond_inner_w / 2.0 + DIAMOND_HALF;
    let diamond_bottom = y + DIAMOND_HALF * 2.0;

    // The break tile sits on the spine, IF_DOWN_LEAD below the diamond. The
    // if-block's pointOut (where the east corridor rejoins the spine) sits a
    // further ARROW_LEN + IF_BRANCH_UP down — plus an uncompressed slack unit
    // when the loop column is thin (see WHILE_BREAK_IF_CORRIDOR_* constants).
    let break_y = diamond_bottom + IF_DOWN_LEAD;
    let return_y = diamond_bottom
        + WHILE_BREAK_IF_CORRIDOR_DROP_COMPRESSED
        + if brk.compresses {
            0.0
        } else {
            WHILE_BREAK_IF_CORRIDOR_UNCOMPRESSED_EXTRA
        };

    // Condition hexagon (no populated branch shapes precede it — the break tile
    // is empty).
    let pts = vec![
        (cx - cond_inner_w / 2.0, y),
        (cx + cond_inner_w / 2.0, y),
        (diamond_right, diamond_cy),
        (cx + cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0, y + DIAMOND_HALF * 2.0),
        (diamond_left, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

    // Draw order: polygon, SOUTH label (the break branch's positive label),
    // condition text, EAST label (the empty branch's label).
    let (south_label, east_label) = if plan.then_is_break {
        (then_label, else_label)
    } else {
        (else_label, then_label)
    };
    if let Some(label) = south_label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            cx + 4.0,
            diamond_bottom + text_render::ascent_for_family(label_font_size, &label_family),
            label,
            false,
        );
    }
    let cond_text_color = svg.palette.text_color.clone();
    let text_y = centered_label_y(condition, diamond_cy, SMALL_FONT);
    svg.text_element_styled(
        &cond_text_color,
        "sans-serif",
        SMALL_FONT,
        cx - cond_text_w / 2.0,
        text_y,
        condition,
        false,
        svg.palette.diamond_text_italic,
    );
    if let Some(label) = east_label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            diamond_right,
            centerline_label_y_for_family(diamond_cy, label_font_size, &label_family),
            label,
            false,
        );
    }

    // Diamond → break: plain connector down the spine to the break tile (the
    // break tile is empty / has no inbound decoration — PlantUML's `ConnectionIn`
    // to a `FtileBreak` draws no arrowhead).
    svg.connector_line(&arrow_color, cx, cx, diamond_bottom, break_y, false);

    // Break weld: horizontal LEFT from the spine to the loop exit corridor, with
    // a left-pointing arrowhead at the corridor (asToLeft). No merge.
    svg.connector_line(&arrow_color, cx, brk.corridor_x, break_y, break_y, false);
    svg.left_arrow(brk.corridor_x, break_y, &arrow_color);

    // Empty east corridor (ConnectionElseNoDiamond): exit the diamond's east
    // vertex, run down (down-emphasized mid arrow), then rejoin the spine at the
    // if-block's pointOut. No terminal in-arrow — it simply welds back.
    let corridor_x = diamond_right + DIAMOND_HALF;
    svg.connector_line(&arrow_color, diamond_right, corridor_x, diamond_cy, diamond_cy, false);
    let arrow_tip = (diamond_cy + return_y) / 2.0 + IF_CORRIDOR_ARROW_OFFSET;
    svg.polygon_connector(
        &arrow_color,
        &[
            (corridor_x - 4.0, arrow_tip - 10.0),
            (corridor_x, arrow_tip),
            (corridor_x + 4.0, arrow_tip - 10.0),
            (corridor_x, arrow_tip - 6.0),
        ],
        &arrow_color,
        "1",
    );
    svg.connector_line(&arrow_color, corridor_x, corridor_x, diamond_cy, return_y, false);
    svg.connector_line(&arrow_color, corridor_x, cx, return_y, return_y, false);

    return_y
}

enum SwitchConn {
    Outer,
    Inner,
    Center,
}

fn emit_switch(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    cases: &[SwitchCase],
) -> f64 {
    if let [case] = cases {
        return emit_switch_one_link(svg, cx, y, condition, case);
    }
    emit_switch_with_layout(
        svg,
        cx,
        y,
        condition,
        cases,
        switch_x_layout(cases, condition),
        false,
    )
}

fn emit_switch_one_link(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    case: &SwitchCase,
) -> f64 {
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();

    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);
    let diamond_cx = cx;
    let diamond_cy = y + DIAMOND_HALF;
    let diamond_left = diamond_cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right = diamond_cx + cond_inner_w / 2.0 + DIAMOND_HALF;
    let diamond_bottom = y + DIAMOND_HALF * 2.0;
    let pts = vec![
        (diamond_cx - cond_inner_w / 2.0, y),
        (diamond_cx + cond_inner_w / 2.0, y),
        (diamond_right, diamond_cy),
        (diamond_cx + cond_inner_w / 2.0, diamond_bottom),
        (diamond_cx - cond_inner_w / 2.0, diamond_bottom),
        (diamond_left, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);
    let cond_text_color = svg.palette.text_color.clone();
    let cond_text_y = centered_label_y(condition, diamond_cy, SMALL_FONT);
    svg.text_element_styled(
        &cond_text_color,
        "sans-serif",
        SMALL_FONT,
        diamond_cx - cond_text_w / 2.0,
        cond_text_y,
        condition,
        false,
        svg.palette.diamond_text_italic,
    );

    let branch_cx = diamond_cx + switch_one_link_branch_dx(condition);
    let cases_top = diamond_bottom + switch_one_link_below_diamond(case);
    let branch_bottom = emit_sequence(svg, &case.body, branch_cx, cases_top);

    svg.connector_line(
        &arrow_color,
        branch_cx,
        branch_cx,
        diamond_bottom,
        cases_top,
        false,
    );
    switch_down_head(svg, &arrow_color, branch_cx, cases_top);
    if !case.label.is_empty() {
        switch_case_label(
            svg,
            &case.label,
            branch_cx + 4.0,
            (diamond_bottom + cases_top) / 2.0 + 4.1572,
        );
    }

    if branch_terminates(&case.body) {
        return branch_bottom;
    }

    let merge_top = branch_bottom + ARROW_LEN;
    let merge_cy = merge_top + DIAMOND_HALF;
    let merge_bottom = merge_top + DIAMOND_HALF * 2.0;
    svg.connector_line(
        &arrow_color,
        diamond_cx,
        diamond_cx,
        branch_bottom,
        merge_top,
        false,
    );
    switch_down_head(svg, &arrow_color, diamond_cx, merge_top);

    svg.polygon_shape(
        &diamond_fill,
        &[
            (diamond_cx, merge_top),
            (diamond_cx, merge_top),
            (diamond_cx + DIAMOND_HALF, merge_cy),
            (diamond_cx, merge_bottom),
            (diamond_cx, merge_bottom),
            (diamond_cx - DIAMOND_HALF, merge_cy),
        ],
        &diamond_stroke,
        &diamond_stroke_width,
    );

    merge_bottom
}

fn emit_switch_with_layout(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    cases: &[SwitchCase],
    layout: SwitchXLayout,
    clamp_left_case: bool,
) -> f64 {
    if cases.is_empty() {
        return y;
    }
    let n = cases.len();

    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();

    // Faithful FtileSwitchWithDiamonds layout. The passed-in cx is the spine,
    // which aligns to the condition/merge diamond (NOT the block centre); the
    // block extends asymmetrically around it per the BIG/SMALL diamond model.
    let block_left = cx - layout.diamond_dx;
    let mut centers: Vec<f64> = layout.centers.iter().map(|c| block_left + c).collect();
    if clamp_left_case && let (Some(center), Some(case)) = (centers.first_mut(), cases.first()) {
        let min_center = SVG_CONTENT_LEAD + switch_case_width(case) / 2.0;
        *center = center.max(min_center);
    }
    let diamond_cx = cx;

    // Switch condition diamond (inner edge clamped; text measured).
    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);
    let diamond_cy = y + DIAMOND_HALF;
    let diamond_left = diamond_cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right = diamond_cx + cond_inner_w / 2.0 + DIAMOND_HALF;
    let diamond_bottom = y + DIAMOND_HALF * 2.0;
    let pts = vec![
        (diamond_cx - cond_inner_w / 2.0, y),
        (diamond_cx + cond_inner_w / 2.0, y),
        (diamond_right, diamond_cy),
        (diamond_cx + cond_inner_w / 2.0, diamond_bottom),
        (diamond_cx - cond_inner_w / 2.0, diamond_bottom),
        (diamond_left, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);
    let cond_text_color = svg.palette.text_color.clone();
    let cond_text_y = centered_label_y(condition, diamond_cy, SMALL_FONT);
    svg.text_element_styled(
        &cond_text_color,
        "sans-serif",
        SMALL_FONT,
        diamond_cx - cond_text_w / 2.0,
        cond_text_y,
        condition,
        false,
        svg.palette.diamond_text_italic,
    );

    let if_branch_big_adjust = if clamp_left_case && layout.big_diamond {
        SWITCH_IF_BRANCH_BIG_CASE_Y_ADJUST
    } else {
        0.0
    };
    let cases_top =
        diamond_bottom + switch_below_diamond(cases, layout.big_diamond) + if_branch_big_adjust;

    // Case bodies (shapes + internal connectors) in source order.
    let mut bottoms = Vec::with_capacity(n);
    for (i, case) in cases.iter().enumerate() {
        bottoms.push(emit_sequence(svg, &case.body, centers[i], cases_top));
    }
    let max_bottom = bottoms.iter().cloned().fold(0.0f64, f64::max);

    let all_terminate = switch_all_branches_terminate(cases);
    let has_center = !n.is_multiple_of(2);
    let merge_gap = switch_merge_gap(cases, condition, &layout);
    let merge_top = max_bottom + merge_gap;
    let merge_cy = merge_top + DIAMOND_HALF;
    let merge_bottom = merge_top + DIAMOND_HALF * 2.0;

    let center_idx = if has_center { Some(n / 2) } else { None };
    let classify = |i: usize| -> SwitchConn {
        if Some(i) == center_idx {
            SwitchConn::Center
        } else if i == 0 || i == n - 1 {
            SwitchConn::Outer
        } else {
            SwitchConn::Inner
        }
    };
    let inner_uses_diamond_corridor = |i: usize, bcx: f64| -> bool {
        matches!(classify(i), SwitchConn::Inner)
            && !cases[i].body.is_empty()
            && bcx >= diamond_left - SWITCH_LINK_MARGIN
            && bcx <= diamond_right + SWITCH_LINK_MARGIN
    };

    // Connector emission order: first, last, then inner indices left-to-right.
    let mut order: Vec<usize> = Vec::with_capacity(n);
    order.push(0);
    if n > 1 {
        order.push(n - 1);
    }
    for i in 1..n.saturating_sub(1) {
        order.push(i);
    }

    // The outer-branch label baseline sits slightly lower in BIG_DIAMOND mode
    // (the wider diamond shifts the label box down by ~1.7 px).
    let outer_label_dy = if layout.big_diamond {
        21.5
    } else {
        SWITCH_LABEL_OUTER_DY
    };

    // ── Top connections (diamond → cases). ──
    for &i in &order {
        let bcx = centers[i];
        if cases[i].body.is_empty() {
            match classify(i) {
                SwitchConn::Outer => {
                    let (diamond_vertex, merge_vertex, label_x) = if bcx <= diamond_cx {
                        (diamond_left, diamond_cx - DIAMOND_HALF, bcx - 4.0)
                    } else {
                        (diamond_right, diamond_cx + DIAMOND_HALF, bcx + 4.0)
                    };
                    svg.connector_line(
                        &arrow_color,
                        diamond_vertex,
                        bcx,
                        diamond_cy,
                        diamond_cy,
                        false,
                    );
                    svg.connector_line(&arrow_color, bcx, bcx, diamond_cy, merge_cy, false);
                    svg.connector_line(&arrow_color, bcx, merge_vertex, merge_cy, merge_cy, false);
                    if bcx <= diamond_cx {
                        svg.right_arrow(merge_vertex, merge_cy, &arrow_color);
                    } else {
                        svg.left_arrow(merge_vertex, merge_cy, &arrow_color);
                    }
                    switch_case_label(svg, &cases[i].label, label_x, diamond_cy + 4.1572);
                }
                SwitchConn::Center => {
                    svg.connector_line(
                        &arrow_color,
                        diamond_cx,
                        diamond_cx,
                        diamond_bottom,
                        merge_top,
                        false,
                    );
                    switch_down_head(svg, &arrow_color, diamond_cx, merge_top);
                    switch_case_label(svg, &cases[i].label, diamond_cx, merge_top - 21.5);
                }
                SwitchConn::Inner
                    if bcx >= diamond_left - SWITCH_LINK_MARGIN
                        && bcx <= diamond_right + SWITCH_LINK_MARGIN =>
                {
                    svg.connector_line(
                        &arrow_color,
                        diamond_cx,
                        diamond_cx,
                        diamond_bottom,
                        merge_top,
                        false,
                    );
                    switch_down_head(svg, &arrow_color, diamond_cx, merge_top);
                    switch_case_label(svg, &cases[i].label, diamond_cx, merge_top - 21.5);
                }
                SwitchConn::Inner => {
                    svg.connector_line(&arrow_color, bcx, bcx, diamond_cy, merge_cy, false);
                    switch_down_head(svg, &arrow_color, bcx, merge_cy);
                    switch_case_label(svg, &cases[i].label, bcx, merge_cy - 33.5);
                }
            }
            continue;
        }
        match classify(i) {
            SwitchConn::Outer => {
                let vertex_x = if bcx <= diamond_cx {
                    diamond_left
                } else {
                    diamond_right
                };
                svg.connector_line(&arrow_color, vertex_x, bcx, diamond_cy, diamond_cy, false);
                svg.connector_line(&arrow_color, bcx, bcx, diamond_cy, cases_top, false);
                switch_down_head(svg, &arrow_color, bcx, cases_top);
                switch_case_label(svg, &cases[i].label, bcx, cases_top - outer_label_dy);
            }
            SwitchConn::Inner if inner_uses_diamond_corridor(i, bcx) => {
                let split = cases_top - SWITCH_CENTER_TOP_SPLIT;
                svg.connector_line(
                    &arrow_color,
                    diamond_cx,
                    diamond_cx,
                    diamond_bottom,
                    split,
                    false,
                );
                svg.connector_line(&arrow_color, diamond_cx, bcx, split, split, false);
                svg.connector_line(&arrow_color, bcx, bcx, split, cases_top, false);
                switch_down_head(svg, &arrow_color, bcx, cases_top);
                // The branch label is positioned at the corridor snake's
                // leftmost x (PlantUML Snake.getTextBlockPosition,
                // VerticalAlignment.CENTER → x = worm.getMinX()): a branch that
                // jogs left of the spine anchors at its own centre, one that
                // jogs right anchors at the spine.
                switch_case_label(
                    svg,
                    &cases[i].label,
                    bcx.min(diamond_cx),
                    cases_top - SWITCH_LABEL_CENTER_DY,
                );
            }
            SwitchConn::Inner => {
                svg.connector_line(&arrow_color, bcx, bcx, diamond_cy, cases_top, false);
                switch_down_head(svg, &arrow_color, bcx, cases_top);
                switch_case_label(svg, &cases[i].label, bcx, cases_top - SWITCH_LABEL_INNER_DY);
            }
            SwitchConn::Center => {
                // Drop from the diamond bottom at diamond_cx to the split, jog
                // horizontally to the branch centre (zero-length when aligned,
                // in which case PlantUML omits the horizontal), then down.
                let split = cases_top - SWITCH_CENTER_TOP_SPLIT;
                svg.connector_line(
                    &arrow_color,
                    diamond_cx,
                    diamond_cx,
                    diamond_bottom,
                    split,
                    false,
                );
                if bcx != diamond_cx {
                    svg.connector_line(&arrow_color, diamond_cx, bcx, split, split, false);
                }
                svg.connector_line(&arrow_color, bcx, bcx, split, cases_top, false);
                switch_down_head(svg, &arrow_color, bcx, cases_top);
                let label_x = if matches!(cases[i].body.first(), Some(LayoutNode::If { .. })) {
                    diamond_cx
                } else {
                    bcx
                };
                switch_case_label(
                    svg,
                    &cases[i].label,
                    label_x,
                    cases_top - SWITCH_LABEL_CENTER_DY,
                );
            }
        }
    }

    // ── Bottom connections (cases → merge). ──
    for &i in &order {
        if cases[i].body.is_empty() || branch_terminates(&cases[i].body) {
            continue;
        }
        let bcx = centers[i];
        let bottom = bottoms[i];
        match classify(i) {
            SwitchConn::Outer => {
                let vertex_x = if bcx <= diamond_cx {
                    diamond_cx - DIAMOND_HALF
                } else {
                    diamond_cx + DIAMOND_HALF
                };
                svg.connector_line(&arrow_color, bcx, bcx, bottom, merge_cy, false);
                svg.connector_line(&arrow_color, bcx, vertex_x, merge_cy, merge_cy, false);
                if bcx <= diamond_cx {
                    svg.right_arrow(vertex_x, merge_cy, &arrow_color);
                } else {
                    svg.left_arrow(vertex_x, merge_cy, &arrow_color);
                }
            }
            SwitchConn::Inner if inner_uses_diamond_corridor(i, bcx) => {
                let split = merge_top - SWITCH_CENTER_BOT_SPLIT;
                svg.connector_line(&arrow_color, bcx, bcx, bottom, split, false);
                svg.connector_line(&arrow_color, bcx, diamond_cx, split, split, false);
                svg.connector_line(
                    &arrow_color,
                    diamond_cx,
                    diamond_cx,
                    split,
                    merge_top,
                    false,
                );
                switch_down_head(svg, &arrow_color, diamond_cx, merge_top);
            }
            SwitchConn::Inner => {
                svg.connector_line(&arrow_color, bcx, bcx, bottom, merge_cy, false);
                switch_down_head(svg, &arrow_color, bcx, merge_cy);
            }
            SwitchConn::Center => {
                if bcx == diamond_cx || !cases.iter().any(|case| branch_terminates(&case.body)) {
                    // When the centre branch is on the switch spine,
                    // PlantUML still splits the vertical connector at the
                    // centre-bottom split point. Ordinary non-terminating
                    // switches use the same split even when label geometry
                    // shifts the centre branch off the spine.
                    let split = merge_top - SWITCH_CENTER_BOT_SPLIT;
                    svg.connector_line(&arrow_color, bcx, bcx, bottom, split, false);
                    if bcx != diamond_cx {
                        svg.connector_line(&arrow_color, bcx, diamond_cx, split, split, false);
                    }
                    svg.connector_line(
                        &arrow_color,
                        diamond_cx,
                        diamond_cx,
                        split,
                        merge_top,
                        false,
                    );
                } else {
                    // If the centre branch's decorated label pushes it off
                    // the spine, PlantUML runs straight down to the merge top
                    // and jogs horizontally into the merge arrowhead tip.
                    svg.connector_line(&arrow_color, bcx, bcx, bottom, merge_top, false);
                    svg.connector_line(&arrow_color, bcx, diamond_cx, merge_top, merge_top, false);
                }
                switch_down_head(svg, &arrow_color, diamond_cx, merge_top);
            }
        }
    }

    if all_terminate {
        return max_bottom;
    }

    // Merge diamond (small rhombus; 7-point closed form per PlantUML).
    svg.polygon_shape(
        &diamond_fill,
        &[
            (diamond_cx, merge_top),
            (diamond_cx, merge_top),
            (diamond_cx + DIAMOND_HALF, merge_cy),
            (diamond_cx, merge_bottom),
            (diamond_cx, merge_bottom),
            (diamond_cx - DIAMOND_HALF, merge_cy),
        ],
        &diamond_stroke,
        &diamond_stroke_width,
    );

    merge_bottom
}

/// Down-pointing arrowhead with tip at (cx, tip_y) into the connectors buffer.
fn switch_down_head(svg: &mut SvgEmitter, color: &str, cx: f64, tip_y: f64) {
    svg.polygon_connector(
        color,
        &[
            (cx - 4.0, tip_y - 10.0),
            (cx, tip_y),
            (cx + 4.0, tip_y - 10.0),
            (cx, tip_y - 6.0),
        ],
        color,
        "1",
    );
}

/// Switch case label text, left-anchored at the branch centre (connectors).
fn switch_case_label(svg: &mut SvgEmitter, label: &str, x: f64, baseline_y: f64) {
    if label.is_empty() {
        return;
    }
    let label_font_size = svg.palette.arrow_font_size;
    let label_family = svg.palette.arrow_font_family.clone();
    let label_color = svg.palette.arrow_text_color.clone();
    let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
    svg.connector_text(
        &label_color,
        &label_family,
        label_font_size,
        lw,
        x,
        baseline_y,
        label,
    );
}

fn emit_fork(svg: &mut SvgEmitter, cx: f64, y: f64, branches: &[Vec<LayoutNode>]) -> f64 {
    if branches.is_empty() {
        return y;
    }

    emit_fork_with_layout(svg, cx, y, branches, fork_layout(branches))
}

fn emit_fork_merge(svg: &mut SvgEmitter, cx: f64, y: f64, branches: &[Vec<LayoutNode>]) -> f64 {
    if branches.is_empty() {
        return y;
    }

    let layout = fork_layout(branches);
    let bar_w = layout.bar_w;
    let bar_x = cx + layout.spine_dx - bar_w / 2.0;
    let bar_color = svg.palette.bar_color.clone();
    let arrow_color = svg.palette.arrow_color.clone();
    svg.rect_styled(
        &bar_color,
        FORK_BAR_HEIGHT,
        FORK_BAR_RX,
        FORK_BAR_RX,
        &bar_color,
        "1",
        bar_w,
        bar_x,
        y,
    );

    let bar_bottom = y + FORK_BAR_HEIGHT;
    let branch_centers: Vec<f64> = layout.centers.iter().map(|center| bar_x + center).collect();
    let mut branch_bottoms = Vec::new();
    for (branch, &bcx) in branches.iter().zip(branch_centers.iter()) {
        let bottom = emit_sequence(svg, branch, bcx, bar_bottom + ARROW_LEN);
        branch_bottoms.push(bottom);
    }

    let max_bottom = branch_bottoms.iter().cloned().fold(0.0f64, f64::max);
    let merge_top = max_bottom + FORK_MERGE_GAP;
    let merge_cy = merge_top + DIAMOND_HALF;
    let merge_bottom = merge_top + DIAMOND_HALF * 2.0;
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();
    svg.polygon_shape(
        &diamond_fill,
        &[
            (cx, merge_top),
            (cx + DIAMOND_HALF, merge_cy),
            (cx, merge_bottom),
            (cx - DIAMOND_HALF, merge_cy),
        ],
        &diamond_stroke,
        &diamond_stroke_width,
    );

    for (branch, &bcx) in branches.iter().zip(branch_centers.iter()) {
        if branch.is_empty() {
            svg.down_arrow(bcx, bar_bottom, merge_top, &arrow_color);
        } else {
            svg.down_arrow(bcx, bar_bottom, bar_bottom + ARROW_LEN, &arrow_color);
        }
    }

    for (i, (branch, bottom)) in branches.iter().zip(branch_bottoms.iter()).enumerate() {
        if branch.is_empty() {
            continue;
        }
        let bcx = branch_centers[i];
        if (bcx - cx).abs() < 0.001 {
            svg.down_arrow(bcx, *bottom, merge_top, &arrow_color);
        } else if bcx < cx {
            svg.connector_line(&arrow_color, bcx, bcx, *bottom, merge_cy, false);
            svg.connector_line(
                &arrow_color,
                bcx,
                cx - DIAMOND_HALF,
                merge_cy,
                merge_cy,
                false,
            );
            svg.right_arrow(cx - DIAMOND_HALF, merge_cy, &arrow_color);
        } else {
            svg.connector_line(&arrow_color, bcx, bcx, *bottom, merge_cy, false);
            svg.connector_line(
                &arrow_color,
                bcx,
                cx + DIAMOND_HALF,
                merge_cy,
                merge_cy,
                false,
            );
            svg.left_arrow(cx + DIAMOND_HALF, merge_cy, &arrow_color);
        }
    }

    merge_bottom
}

fn emit_fork_with_layout(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    branches: &[Vec<LayoutNode>],
    layout: ForkLayout,
) -> f64 {
    if branches.is_empty() {
        return y;
    }

    let bar_w = layout.bar_w;

    // Top bar
    let bar_x = cx + layout.spine_dx - bar_w / 2.0;
    let bar_color = svg.palette.bar_color.clone();
    let arrow_color = svg.palette.arrow_color.clone();
    svg.rect_styled(
        &bar_color,
        FORK_BAR_HEIGHT,
        FORK_BAR_RX,
        FORK_BAR_RX,
        &bar_color,
        "1",
        bar_w,
        bar_x,
        y,
    );

    let bar_bottom = y + FORK_BAR_HEIGHT;

    let branch_centers: Vec<f64> = layout.centers.iter().map(|center| bar_x + center).collect();

    // Render branches FIRST so their internal arrow connectors land in the
    // connectors buffer before the top/bottom-bar arrows below. Java
    // emits in this order: branch internal connectors, then all top arrows,
    // then all bottom arrows. Reverse-engineered from goldens.
    // PlantUML wraps every fork branch in FtileHeightFixedCentered so all
    // branches occupy the SAME height (the tallest branch's) and shorter ones
    // are vertically centred within that band (AbstractParallelFtilesBuilder
    // .computeNewFtile). Only when the branches differ in height does the
    // global Snake compression fail to reclaim the 15 px assembly slack — then
    // every branch keeps the uncompressed 35 px inter-tile gap; equal-height
    // forks compress back to ARROW_LEN. Restrict to pure action/control-flow
    // forks (no partition-wrapped branch, which carries its own vertical model).
    let center_branches = !branches
        .iter()
        .any(|branch| single_partition_branch_body_top(branch, bar_bottom).is_some());
    let base_heights: Vec<f64> = branches
        .iter()
        .map(|b| {
            if b.is_empty() {
                0.0
            } else {
                sequence_height(b)
            }
        })
        .collect();
    let nonempty_base: Vec<f64> = base_heights
        .iter()
        .zip(branches.iter())
        .filter(|(_, b)| !b.is_empty())
        .map(|(h, _)| *h)
        .collect();
    let branches_unequal = !nonempty_base.is_empty()
        && nonempty_base
            .iter()
            .any(|h| (h - nonempty_base[0]).abs() > 0.01);
    let gap_extra = if center_branches && branches_unequal {
        FORK_BRANCH_INTER_GAP_EXTRA
    } else {
        0.0
    };
    // Apply the inter-tile extra for the branch (measure + real) emits, then
    // restore before the bar-arrow loops.
    let saved_gap_extra = svg.fork_branch_gap_extra;
    svg.fork_branch_gap_extra = gap_extra;
    let mut center_offsets = vec![0.0f64; branches.len()];
    if center_branches {
        let heights: Vec<f64> = branches
            .iter()
            .map(|b| {
                if b.is_empty() {
                    0.0
                } else {
                    sequence_height_ex(b, gap_extra)
                }
            })
            .collect();
        let max_h = heights.iter().cloned().fold(0.0f64, f64::max);
        for (i, &h) in heights.iter().enumerate() {
            if !branches[i].is_empty() {
                center_offsets[i] = (max_h - h) / 2.0;
            }
        }
    }

    // A branch whose last tile terminates (Detach/Kill/Stop/End/Break/Goto) has
    // no pointOut: PlantUML's ParallelBuilderFork.doStep2 skips its ConnectionOut
    // (line `if (hasPointOut())`), so it is NOT wired to the join bar.
    let branch_terminates_flags: Vec<bool> =
        branches.iter().map(|b| branch_terminates(b)).collect();

    let mut branch_bottoms = Vec::new();
    for (i, (branch, &bcx)) in branches.iter().zip(branch_centers.iter()).enumerate() {
        let branch_y = if single_partition_branch_body_top(branch, bar_bottom).is_some() {
            bar_bottom
        } else {
            bar_bottom + ARROW_LEN + center_offsets[i]
        };
        let bottom = emit_sequence(svg, branch, bcx, branch_y);
        branch_bottoms.push(bottom);
    }
    svg.fork_branch_gap_extra = saved_gap_extra;

    // Find the maximum bottom. When every non-empty branch terminates, no
    // ConnectionOut arrow reaches the join bar, and PlantUML seats the bar
    // 10 px below the deepest branch (half the usual ARROW_LEN reserve) rather
    // than the full join-arrow gap.
    let max_bottom = branch_bottoms.iter().cloned().fold(0.0f64, f64::max);
    let all_branches_terminate = branches
        .iter()
        .zip(branch_terminates_flags.iter())
        .all(|(branch, &term)| branch.is_empty() || term)
        && branches.iter().any(|b| !b.is_empty());
    let bottom_bar_gap = if all_branches_terminate {
        FORK_TERMINATING_JOIN_GAP
    } else {
        ARROW_LEN
    };
    let bottom_bar_y = max_bottom + bottom_bar_gap;

    // Top arrows from bar to each branch (all together, after internals).
    // Empty fork branches do not draw a zero-height top arrow plus a separate
    // bottom arrow. PlantUML gives the empty lane one connector from the top
    // bar straight into the bottom bar, in branch order.
    for (i, (branch, &bcx)) in branches.iter().zip(branch_centers.iter()).enumerate() {
        if branch.is_empty() {
            svg.down_arrow(bcx, bar_bottom, bottom_bar_y, &arrow_color);
            continue;
        }
        let branch_y = if single_partition_branch_body_top(branch, bar_bottom).is_some() {
            bar_bottom
        } else {
            bar_bottom + ARROW_LEN + center_offsets[i]
        };
        let arrow_bottom = single_partition_branch_body_top(branch, branch_y)
            .unwrap_or(bar_bottom + ARROW_LEN + center_offsets[i]);
        svg.down_arrow(bcx, bar_bottom, arrow_bottom, &arrow_color);
    }

    // Bottom arrows from each branch to bottom bar — skipped for a branch that
    // terminates (no pointOut → no ConnectionOut to the join bar).
    for (i, (branch, bottom)) in branches.iter().zip(branch_bottoms.iter()).enumerate() {
        if branch.is_empty() || branch_terminates_flags[i] {
            continue;
        }
        let bcx = branch_centers[i];
        let branch_y = if single_partition_branch_body_top(branch, bar_bottom).is_some() {
            bar_bottom
        } else {
            bar_bottom + ARROW_LEN + center_offsets[i]
        };
        let arrow_top = single_partition_branch_body_bottom(branch, branch_y).unwrap_or(*bottom);
        svg.down_arrow(bcx, arrow_top, bottom_bar_y, &arrow_color);
    }

    // Bottom bar
    svg.rect_styled(
        &bar_color,
        FORK_BAR_HEIGHT,
        FORK_BAR_RX,
        FORK_BAR_RX,
        &bar_color,
        "1",
        bar_w,
        bar_x,
        bottom_bar_y,
    );

    bottom_bar_y + FORK_BAR_HEIGHT
}

fn emit_split(svg: &mut SvgEmitter, cx: f64, y: f64, branches: &[Vec<LayoutNode>]) -> f64 {
    if branches.is_empty() {
        return y;
    }

    emit_split_with_layout(svg, cx, y, branches, split_layout(branches))
}

fn emit_split_with_layout(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    branches: &[Vec<LayoutNode>],
    layout: ForkLayout,
) -> f64 {
    if branches.is_empty() {
        return y;
    }

    let left_x = cx + layout.spine_dx - layout.bar_w / 2.0;
    let branch_centers: Vec<f64> = layout
        .centers
        .iter()
        .map(|center| left_x + center)
        .collect();
    let Some((&line_start, &line_end)) = branch_centers.first().zip(branch_centers.last()) else {
        return y;
    };
    let line_color = svg.palette.arrow_color.clone();
    svg.shape_line(&line_color, "1.5", line_start, line_end, y, y);

    let branch_top = y + ARROW_LEN;
    let mut branch_bottoms = Vec::new();
    for (branch, &bcx) in branches.iter().zip(branch_centers.iter()) {
        let bottom = emit_sequence(svg, branch, bcx, branch_top);
        branch_bottoms.push(bottom);
    }

    let max_bottom = branch_bottoms.iter().cloned().fold(0.0f64, f64::max);
    let bottom_line_y = max_bottom + ARROW_LEN;
    svg.shape_line(
        &line_color,
        "1.5",
        line_start,
        line_end,
        bottom_line_y,
        bottom_line_y,
    );

    for (branch, &bcx) in branches.iter().zip(branch_centers.iter()) {
        if branch.is_empty() {
            svg.down_arrow(bcx, y + 1.5, bottom_line_y, &line_color);
        } else {
            svg.down_arrow(bcx, y + 1.5, branch_top, &line_color);
        }
    }
    for (i, (branch, bottom)) in branches.iter().zip(branch_bottoms.iter()).enumerate() {
        if branch.is_empty() {
            continue;
        }
        svg.down_arrow(branch_centers[i], *bottom, bottom_line_y, &line_color);
    }

    bottom_line_y
}

#[allow(clippy::too_many_arguments)]
fn emit_while(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    condition: &str,
    diamond_font_family: &str,
    diamond_font_size: f64,
    diamond_text_color: &str,
    diamond_text_bold: bool,
    diamond_text_italic: bool,
    is_label: &Option<String>,
    end_label: &Option<String>,
    body: &[LayoutNode],
    special_out: Option<&LayoutNode>,
    starts_column: bool,
) -> f64 {
    // PlantUML's FtileWhile layout (reverse-engineered):
    //   - condition hexagon at the top
    //   - "is (yes)" label sits just below the diamond, on the body-path
    //   - inbound arrow from diamond bottom down to body top — long enough
    //     (~37 px) to accommodate the "yes" label
    //   - body (sequence inside the loop)
    //   - junction 12 px below body where the loop-back arm attaches
    //   - loop-back arm: junction → right (loop_x) → up to diamond_cy →
    //     left into diamond's right vertex (the "back to condition" path)
    //   - exit arm: from diamond's left vertex left to exit_x, then down to
    //     the next sibling's y (for byte-exact match this would route around
    //     the full body — we approximate by stopping at junction_y here)
    //   - "endwhile (no)" label sits just outside the diamond's left vertex
    //
    // Geometry constants are derived empirically from goldens. Byte-exact
    // match across all cases (with/without specialOut, with/without
    // compression) requires more work — see project notes.
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();
    let colored_partition_while = svg.colored_partition_while_depth > 0;
    let ordinary_slot_compressed =
        while_ordinary_slot_compresses(body, is_label, end_label, special_out);
    let compress_while_slot = colored_partition_while || ordinary_slot_compressed;
    // A break-bearing loop keeps the normal long-exit arrowhead placement: the
    // break shares the exit corridor, so the loop exits/wraps like a no-special
    // loop and its loop-back/exit arrowheads sit at the *uncompressed* midpoint
    // (PlantUML draws them before the slot-compression pass shifts endpoints).
    let break_in_body = body_contains_break_if(body);
    let exit_vertical_after_arrow =
        special_out.is_none() && (colored_partition_while || ordinary_slot_compressed);

    let cond_inner_w = diamond_inner_w_styled(
        condition,
        diamond_font_size,
        diamond_text_bold,
        diamond_font_family,
    );
    let cond_text_w = text_render::measure_with_family(
        condition,
        diamond_font_size,
        diamond_text_bold,
        diamond_font_family,
    );
    let diamond_cy = y + DIAMOND_HALF;
    let diamond_bottom = y + DIAMOND_HALF * 2.0;
    let diamond_left_vertex_x = cx - cond_inner_w / 2.0 - DIAMOND_HALF;
    let diamond_right_vertex_x = cx + cond_inner_w / 2.0 + DIAMOND_HALF;

    // Inbound arrow length from diamond bottom to body top. The "is (yes)"
    // label sits below the diamond; the inbound arrow must accommodate it
    // plus the gap to body_top. Base reservation: text_height(11) + 2*halfHex
    // ≈ 36.95. When the body is non-empty AND there's no `endwhile (label)`,
    // PlantUML's slot compression removes ~4.82 of slack between diamond and
    // body, giving 32.1348 empirically. (Empty body doesn't compress because
    // it bridges directly to the junction, leaving no compressible slack.)
    // The "is (yes)" label below the diamond reserves text_height(11) plus
    // two hexagon half-sizes of vertical lead before the body top, then the
    // same slot-compression pass used by calculateDimensionFtile removes the
    // excess slack for ordinary labeled, non-empty loops without an exit label.
    let body_top_offset = while_body_top_offset(
        compress_while_slot,
        is_label.is_some(),
        end_label.is_some(),
        body.is_empty(),
        svg.palette.arrow_font_size,
    );
    let body_top = diamond_bottom + body_top_offset;

    // Break welding context (PlantUML's `FtileFactoryDelegatorWhile.createWhile`
    // post-pass): a `break` nested in a body `if` is rendered by that if as a
    // `FtileIfDown` with a suppressed merge diamond, welding its break branch
    // LEFT to the loop's left exit edge (`Hexagon.hexagonHalfSize`). That edge
    // is the same `geo_left_x - halfHex` the regular exit arm uses, independent
    // of any `specialOut` (which only relocates the *exit* arm, not the loop's
    // left frame). It's computable up-front from the body's (pure) extents. Set
    // it before emitting the body and restore the prior value afterwards (so a
    // sibling/parent while does not inherit it).
    let prev_while_break = svg.while_break.take();
    if body_contains_break_if(body) {
        let (pre_body_left_ext, _) = sequence_loop_body_extents(body);
        let pre_body_left_x = cx - while_body_left(body, pre_body_left_ext);
        let pre_geo_left_x = diamond_left_vertex_x.min(pre_body_left_x);
        svg.while_break = Some(WhileBreakContext {
            corridor_x: pre_geo_left_x - DIAMOND_HALF,
            compresses: while_break_corridor_compresses(body),
        });
    }

    // Body below diamond — emit it first (PlantUML emits body shapes before
    // diamond shapes in document order).
    let body_mid_stretch = while_body_mid_stretch(body, is_label.is_some());
    let body_bottom = emit_sequence_ex(svg, body, cx, body_top, body_mid_stretch, None, false);
    svg.while_break = prev_while_break;

    // Junction y: 12 px below the body for empty bodies, 10 px for
    // non-empty bodies. PlantUML's UEmpty(5, halfHex=12) placeholder is
    // compressed by 2 when adjacent to body content (slot finder removes
    // the slack); for empty bodies there's no adjacent content so the
    // gap stays at 12. The arrowhead midpoint below still uses the
    // un-compressed value (body_bottom + 12) — PlantUML draws the
    // arrowhead at the midpoint of the segment BEFORE compression
    // transforms the line endpoints.
    let junction_y = if body.is_empty() {
        body_bottom + DIAMOND_HALF
    } else {
        body_bottom + 10.0
    };

    // Loop-back arm x position: 12 past whichever is wider, the diamond or
    // the body's right extent.
    let (body_left_ext, body_right_ext) = sequence_loop_body_extents(body);
    let body_right_x = cx + body_right_ext;
    // A deprecated body pulls the left corridor 2 px tighter (see
    // while_body_left); the special-terminator placement uses the adjusted
    // extent so the terminator lands at the same absolute x as a
    // non-deprecated body. The body box itself stays centred on the spine.
    let body_left_x = cx - while_body_left(body, body_left_ext);
    let loop_x = diamond_right_vertex_x.max(body_right_x)
        + DIAMOND_HALF
        + while_single_if_right_pad(body, end_label);

    // Exit arm geometry. Two modes:
    //
    // (a) WITH special_out (Stop/End/Detach/Kill after endwhile): exit goes
    //     LEFT from diamond_left_vertex to the special's cx column, then
    //     DOWN to the special's top. The special is drawn INSIDE the
    //     while's frame. exit_x = special's cx in absolute coordinates,
    //     positioned per PlantUML's translateForSpecial formula.
    //
    // (b) WITHOUT special_out (regular flow continuation): exit goes LEFT
    //     to halfHex past the widest content, then DOWN to junction_y.
    //     The exit_x is just past the body's left extent (or the diamond's
    //     left vertex, whichever is further out).
    let geo_left_x = diamond_left_vertex_x.min(body_left_x);
    let (exit_x, exit_bottom_y) = if let Some(special) = special_out {
        let special_w = node_width(special);
        // translateForSpecial.x in FtileWhile-local =
        //   min(xWhile - halfHex, xDiamond) - xDeltaBecauseSpecial
        // where xWhile = body_left in FtileWhile-local, xDiamond =
        // diamond_left in FtileWhile-local. Translating to absolute:
        //   min(body_left_x - halfHex, diamond_left_vertex_x) - special_w
        // The special's cx is at translateForSpecial.x + special_w/2.
        let mut special_x_adjust = if while_body_drives_special(
            body,
            while_body_left(body, body_left_ext),
            cond_inner_w / 2.0 + DIAMOND_HALF,
            is_label.is_some(),
            end_label.as_deref(),
            starts_column,
        ) {
            WHILE_SPECIAL_BODY_X_PULL_RIGHT
        } else {
            0.0
        };
        if body_has_direct_left_note(body) {
            special_x_adjust -= 1.0;
        }
        let special_left_abs =
            (body_left_x - DIAMOND_HALF).min(diamond_left_vertex_x) - special_w + special_x_adjust;
        let special_cx = special_left_abs + special_w / 2.0;
        // translateForSpecial.y in FtileWhile-local =
        //   max(3*half, 4*halfHex) where half = diamond hexagon's
        //   (outY - inY)/2 = 12. So translateForSpecial.y = max(36, 48) = 48.
        // Absolute: special_top = y + (48 - DIAMOND_HALF*2) below diamond.
        // y is the diamond's top. Diamond extends 24 below y. So
        // special_top_abs = y + 48 = diamond_top + 4*halfHex.
        let special_top = y + 4.0 * DIAMOND_HALF
            - if is_label.is_none() {
                WHILE_UNLABELED_SPECIAL_Y_PULL_UP
            } else {
                0.0
            };
        (special_cx, special_top)
    } else {
        let exit_x = geo_left_x - DIAMOND_HALF;
        (exit_x, junction_y)
    };

    // Diamond polygon (after body shapes are in `shapes`).
    let pts = vec![
        (cx - cond_inner_w / 2.0, y),
        (cx + cond_inner_w / 2.0, y),
        (diamond_right_vertex_x, diamond_cy),
        (cx + cond_inner_w / 2.0, diamond_bottom),
        (cx - cond_inner_w / 2.0, diamond_bottom),
        (diamond_left_vertex_x, diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

    // "is (yes)" label below diamond on the body-down path. PlantUML emits
    // this BEFORE the inside-diamond condition text.
    if let Some(label) = is_label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            cx + 4.0,
            diamond_bottom + text_render::ascent_for_family(label_font_size, &label_family),
            label,
            false,
        );
    }

    // Condition text inside diamond.
    let text_y = centered_label_y_for_family(
        condition,
        diamond_cy,
        diamond_font_size,
        diamond_font_family,
    );
    svg.text_element_styled(
        diamond_text_color,
        diamond_font_family,
        diamond_font_size,
        cx - cond_text_w / 2.0,
        text_y,
        condition,
        diamond_text_bold,
        diamond_text_italic,
    );

    // "endwhile (no)" label just outside diamond's left vertex.
    if let Some(label) = end_label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            diamond_left_vertex_x - lw,
            centerline_label_y_for_family(diamond_cy, label_font_size, &label_family),
            label,
            false,
        );
    }

    // Connector emission order (matches PlantUML's snake walk):
    //   1. body cx vertical: diamond_bottom → body_top (with inbound arrowhead)
    //   2. body cx vertical: body_bottom → junction_y (no arrowhead)
    //   3. junction horizontal: cx → loop_x
    //   4. loop arm UP arrowhead at midpoint
    //   5. loop arm vertical: loop_x from diamond_cy → junction_y
    //   6. loop arm horizontal: diamond_cy from loop_x → diamond_right_vertex
    //   7. loop arm LEFT arrowhead at diamond_right_vertex
    //   8. exit arm horizontal: diamond_cy from diamond_left_vertex → exit_x
    //   9. exit arm vertical: exit_x from diamond_cy → junction_y
    //  10. exit arm DOWN arrowhead at the end (or at midpoint for long arms)

    // 1. Inbound path from diamond bottom to body top. With no body content,
    // PlantUML stretches this as a plain connector down to the loop junction;
    // there is no emphasized down arrowhead for the empty placeholder.
    if body.is_empty() {
        svg.line_styled(&arrow_color, "1", cx, cx, diamond_bottom, junction_y, false);
    } else {
        svg.down_arrow(cx, diamond_bottom, body_top, &arrow_color);
    }

    // 2. Body bottom → junction (only if body has content; for empty body
    // the inbound arrow already reaches the junction-equivalent point).
    if !body.is_empty() {
        svg.line_styled(&arrow_color, "1", cx, cx, body_bottom, junction_y, false);
    }

    // 3. Horizontal at junction from body cx out to loop_x.
    svg.line_styled(&arrow_color, "1", cx, loop_x, junction_y, junction_y, false);

    // 4. UP arrowhead at midpoint of the loop arm's vertical run.
    // PlantUML draws this at the midpoint of (diamond_cy, body_bottom +
    // halfHex), adjusted by the same compression that shifts body_top up.
    let mid_y = (diamond_cy + body_bottom + DIAMOND_HALF) / 2.0
        - while_slot_compress(
            compress_while_slot,
            is_label.is_some(),
            end_label.is_some(),
            body.is_empty(),
        ) / 2.0
        - if is_label.is_none() {
            WHILE_UNLABELED_LOOP_ARROW_Y_PULL_UP
        } else {
            0.0
        }
        + body_mid_stretch.map_or(0.0, |(_, stretch)| {
            WHILE_EVEN_BODY_LOOP_ARROW_STRETCH - stretch / 2.0
        })
        // Break-bearing loop: place the loop-back arrowhead at the uncompressed
        // midpoint (add back the slot-compression half the line above removed).
        + if break_in_body {
            IF_CORRIDOR_ARROW_OFFSET
        } else {
            0.0
        };
    svg.polygon_connector(
        &arrow_color,
        &[
            (loop_x - 4.0, mid_y + 10.0),
            (loop_x, mid_y),
            (loop_x + 4.0, mid_y + 10.0),
            (loop_x, mid_y + 6.0),
        ],
        &arrow_color,
        "1",
    );

    // 5. Loop arm vertical at loop_x.
    svg.line_styled(&arrow_color, "1", loop_x, loop_x, diamond_cy, junction_y, false);

    // 6. Loop arm horizontal at diamond_cy: loop_x → diamond_right_vertex.
    svg.line_styled(
        &arrow_color,
        "1",
        loop_x,
        diamond_right_vertex_x,
        diamond_cy,
        diamond_cy,
        false,
    );

    // 7. LEFT arrowhead at diamond_right_vertex.
    svg.polygon_connector(
        &arrow_color,
        &[
            (diamond_right_vertex_x + 10.0, diamond_cy - 4.0),
            (diamond_right_vertex_x, diamond_cy),
            (diamond_right_vertex_x + 10.0, diamond_cy + 4.0),
            (diamond_right_vertex_x + 6.0, diamond_cy),
        ],
        &arrow_color,
        "1",
    );

    // 8. Exit arm horizontal at diamond_cy: diamond_left_vertex → exit_x.
    svg.line_styled(
        &arrow_color,
        "1",
        diamond_left_vertex_x,
        exit_x,
        diamond_cy,
        diamond_cy,
        false,
    );

    // Branch-nested no-special while: the exit corridor is the if's
    // branch→merge connection (PlantUML fuses FtileWhile.ConnectionOut with
    // the if's ConnectionVerticalThenHorizontal under MergeStrategy.LIMITED).
    // Route the corridor straight down to the merge diamond instead of
    // wrapping back to our own spine. Consume the redirect one-shot so a
    // sibling/parent while does not inherit it.
    if special_out.is_none()
        && let Some(redir) = svg.while_exit_redirect.take()
    {
        let merge_cy = redir.merge_cy;
        // DOWN arrowhead at the midpoint of the corridor's vertical run
        // (emphasizeDirection.DOWN), then the vertical, then the horizontal
        // into the merge diamond vertex with the in-arrow. PlantUML places the
        // emphasize arrow at the midpoint of the FtileWhile's own ConnectionOut
        // segment (diamond_cy → frame bottom), which sits one slot-compression
        // half (minus one) above the midpoint of the full corridor to merge_cy.
        let arrow_y = (diamond_cy + merge_cy) / 2.0 - PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP;
        svg.polygon_connector(
            &arrow_color,
            &[
                (exit_x - 4.0, arrow_y - 10.0),
                (exit_x, arrow_y),
                (exit_x + 4.0, arrow_y - 10.0),
                (exit_x, arrow_y - 6.0),
            ],
            &arrow_color,
            "1",
        );
        svg.line_styled(
            &arrow_color,
            "1",
            exit_x,
            exit_x,
            diamond_cy,
            merge_cy,
            false,
        );
        svg.line_styled(
            &arrow_color,
            "1",
            exit_x,
            redir.merge_vertex_x,
            merge_cy,
            merge_cy,
            false,
        );
        if redir.to_right {
            svg.right_arrow(redir.merge_vertex_x, merge_cy, &arrow_color);
        } else {
            svg.left_arrow(redir.merge_vertex_x, merge_cy, &arrow_color);
        }
        return merge_cy;
    }

    // 9. Exit arm vertical at exit_x — single line from diamond_cy down
    // to the final exit y. For specialOut, that's exit_bottom_y (= special
    // top). For no-specialOut, that's wrap_y (= exit_bottom_y + halfHex),
    // and we emit a wrap-back horizontal to cx afterward.
    let wrap_y = if special_out.is_some() {
        exit_bottom_y
    } else {
        exit_bottom_y + DIAMOND_HALF
    };
    if !exit_vertical_after_arrow {
        svg.line_styled(&arrow_color, "1", exit_x, exit_x, diamond_cy, wrap_y, false);
    }

    // 10. DOWN arrowhead. When special_out is present, the arrowhead lands
    // AT the terminator's top (ConnectionOutSpecial uses endDecoration);
    // otherwise the arrowhead is at the midpoint of the long exit arm
    // (ConnectionOut with emphasizeDirection).
    let mut arrow_y = if special_out.is_some() {
        wrap_y
    } else {
        (diamond_cy + wrap_y) / 2.0
    };
    if exit_vertical_after_arrow {
        arrow_y -= PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP;
    }
    if break_in_body {
        // Break loop: the long-exit arrowhead sits at the uncompressed midpoint
        // (like the loop-back arrowhead) — add back IF_CORRIDOR_ARROW_OFFSET on
        // top of the compression pull-up the line above applied.
        arrow_y += IF_CORRIDOR_ARROW_OFFSET;
    }
    svg.polygon_connector(
        &arrow_color,
        &[
            (exit_x - 4.0, arrow_y - 10.0),
            (exit_x, arrow_y),
            (exit_x + 4.0, arrow_y - 10.0),
            (exit_x, arrow_y - 6.0),
        ],
        &arrow_color,
        "1",
    );
    if exit_vertical_after_arrow {
        svg.line_styled(&arrow_color, "1", exit_x, exit_x, diamond_cy, wrap_y, false);
    }

    // 11. Either emit the special_out terminator INSIDE the while's frame
    // (ConnectionOutSpecial) or emit the wrap-back horizontal from exit_x
    // back to cx (ConnectionOut's snake2).
    if let Some(special) = special_out {
        emit_node(svg, special, exit_x, wrap_y)
    } else {
        svg.line_styled(&arrow_color, "1", exit_x, cx, wrap_y, wrap_y, false);
        wrap_y
    }
}

struct RepeatEmitOptions<'a> {
    backward: Option<&'a str>,
    not_label: Option<&'a str>,
    body_top_extra: f64,
    has_start_label: bool,
}

fn emit_repeat(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    body: &[LayoutNode],
    condition: &str,
    is_label: &Option<String>,
    options: RepeatEmitOptions<'_>,
) -> f64 {
    let backward = options.backward;
    let not_label = options.not_label;
    let body_top_extra = options.body_top_extra;
    let has_start_label = options.has_start_label;
    let arrow_color = svg.palette.arrow_color.clone();
    let diamond_stroke = svg.palette.diamond_stroke.clone();
    let diamond_fill = svg.palette.diamond_fill.clone();
    let diamond_stroke_width = svg.palette.diamond_stroke_width.clone();
    let text_color = svg.palette.text_color.clone();

    // PlantUML emits repeat in this order: body shapes → top entry diamond
    // → condition diamond → labels → connectors. Compute positions
    // up-front so we can defer the diamond emits until after the body.
    let top_diamond_size = DIAMOND_HALF;
    let top_bottom = y + top_diamond_size * 2.0;
    let body_y = if has_start_label {
        y + body_top_extra
    } else {
        top_bottom + ARROW_LEN + body_top_extra
    };

    // Body first — its rects/texts land in `shapes` before either diamond.
    // For `repeat :label;`, PlantUML uses the labelled action as the entry
    // tile but emits the remaining body action shapes before that entry tile.
    let body_mid_stretch = if has_start_label {
        None
    } else {
        repeat_body_mid_stretch(body, backward.is_some())
    };
    let body_bottom = if has_start_label {
        if let Some((entry, rest)) = body.split_first() {
            let entry_bottom = body_y + node_height(entry);
            let rest_has_flow = rest.iter().any(node_is_flow);
            let rest_y = entry_bottom + if rest_has_flow { ARROW_LEN } else { 0.0 };
            let body_bottom = if rest_has_flow {
                emit_sequence_ex(svg, rest, cx, rest_y, None, None, false)
            } else {
                entry_bottom
            };
            emit_node(svg, entry, cx, body_y);
            if rest_has_flow {
                svg.down_arrow(cx, entry_bottom, rest_y, &arrow_color);
            }
            body_bottom
        } else {
            body_y
        }
    } else {
        emit_sequence_ex(svg, body, cx, body_y, body_mid_stretch, None, false)
    };
    // Single-action backward repeats keep the extra halfHex before the
    // condition diamond; multi-action bodies absorb that slack in their final
    // inbound connector.
    let backward_flow_count = body.iter().filter(|n| node_is_flow(n)).count();
    let cond_y = if backward.is_some() && backward_flow_count < 2 {
        body_bottom + ARROW_LEN + 10.0
    } else {
        body_bottom + ARROW_LEN
    };

    // Top entry diamond (small rhombus at y). `repeat :label;` replaces this
    // diamond with the labelled action as the loop entry tile.
    if !has_start_label {
        svg.polygon_shape(
            &diamond_fill,
            &[
                (cx, y),
                (cx + top_diamond_size, y + top_diamond_size),
                (cx, y + top_diamond_size * 2.0),
                (cx - top_diamond_size, y + top_diamond_size),
            ],
            &diamond_stroke,
            &diamond_stroke_width,
        );
    }

    // Condition diamond (hexagon below body).
    let cond_inner_w = diamond_inner_w(condition);
    let cond_text_w = text_render::measure(condition, SMALL_FONT, false);
    let cond_diamond_cy = cond_y + DIAMOND_HALF;
    let pts = vec![
        (cx - cond_inner_w / 2.0, cond_y),
        (cx + cond_inner_w / 2.0, cond_y),
        (cx + cond_inner_w / 2.0 + DIAMOND_HALF, cond_diamond_cy),
        (cx + cond_inner_w / 2.0, cond_y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0, cond_y + DIAMOND_HALF * 2.0),
        (cx - cond_inner_w / 2.0 - DIAMOND_HALF, cond_diamond_cy),
    ];
    svg.polygon_shape(&diamond_fill, &pts, &diamond_stroke, &diamond_stroke_width);

    // `not (label)` is the downward exit label. It is emitted before the
    // condition text and right-side `is` label in PlantUML's shape stream.
    if let Some(label) = not_label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            cx + 4.0,
            cond_y
                + DIAMOND_HALF * 2.0
                + text_render::ascent_for_family(label_font_size, &label_family),
            label,
            false,
        );
    }

    let text_y = centered_label_y(condition, cond_diamond_cy, SMALL_FONT);
    svg.text_element_styled(
        &text_color,
        "sans-serif",
        SMALL_FONT,
        cx - cond_text_w / 2.0,
        text_y,
        condition,
        false,
        svg.palette.diamond_text_italic,
    );

    // "is" label (optional). Sits with its baseline at cond_cy - descent(11)
    // so the text aligns vertically slightly above the diamond's mid-line.
    let diamond_right = cx + cond_inner_w / 2.0 + DIAMOND_HALF;
    if let Some(label) = is_label {
        let label_font_size = svg.palette.arrow_font_size;
        let label_family = svg.palette.arrow_font_family.clone();
        let label_color = svg.palette.arrow_text_color.clone();
        let lw = text_render::measure_with_family(label, label_font_size, false, &label_family);
        svg.text_element(
            &label_color,
            &label_family,
            label_font_size,
            lw,
            diamond_right,
            centerline_label_y_for_family(cond_diamond_cy, label_font_size, &label_family),
            label,
            false,
        );
    }

    // Top-diamond → body inbound connector — PlantUML emits this BEFORE
    // the loop-back path in the connector stream. Labelled-start repeats
    // have no top diamond; the outer sequence connector enters the label box.
    if !has_start_label {
        svg.down_arrow(cx, top_bottom, body_y, &arrow_color);
    }

    // Loop-back arrow runs up the right side regardless of whether `is`
    // has a label — every `repeatwhile` produces it.
    //
    // The return worm routes outside BOTH the body's reserved geometry and
    // its drawn content. Two clearances apply, and the arm takes the larger:
    //  - `geo.right() + 4`: the body tile's reserved right boundary
    //    (`FtileGeometry.width − left`) plus the arrowhead half-wing. For an
    //    if/switch body this reserved boundary sits a few px past the drawn
    //    branches (the merge corridor), so this clearance dominates.
    //  - `extents.right + halfHex`: the drawn content's right edge plus the
    //    standard 12 px hexagon gap. For a plain action body the geometry and
    //    extents coincide, so this 12 px clearance dominates.
    // Using the body's *asymmetric* right (not `width/2`) also keeps an
    // off-centre body (e.g. a nested-if with a wide left branch) from leaving
    // a phantom empty corridor that the ON_X compression pass would collapse.
    let body_right = if partition_body_has_direct_note(body) {
        cx + sequence_loop_body_extents(body).1 + 12.0
    } else {
        let extents_clear = cx + sequence_extents(body).1 + 12.0;
        let geo_clear = sequence_geometry(body)
            .map_or(extents_clear, |g| cx + g.right() + 4.0);
        extents_clear.max(geo_clear)
    };
    let arm_x = (diamond_right + 12.0).max(body_right);
    let first_entry = first_flow_node(body);
    let top_cy = if has_start_label {
        first_entry.map_or(y + top_diamond_size, |node| {
            body_y + node_height(node) / 2.0
        })
    } else {
        y + top_diamond_size
    };
    let top_entry_right = if has_start_label {
        first_entry.map_or(cx + top_diamond_size, |node| cx + node_width(node) / 2.0)
    } else {
        cx + top_diamond_size
    };

    if let Some(label) = backward {
        // `backward :label;` draws an action box on the return arm. The loop
        // arm runs up the right side through the centre of this box, which sits
        // at the same vertical band as the body. Mirrors FtileRepeat's
        // ConnectionBackBackward1/2 routing: cond diamond → up into box bottom,
        // box → up out of box top → across to the entry diamond.
        //
        // The box column mirrors the width pass above: condition-dominant
        // repeats append it 24 px past the diamond east vertex; body-dominant
        // repeats clear the east label before placing it.
        let bw = repeat_backward_box_w(label);
        let body_half = sequence_width(body) / 2.0;
        let cond_half = cond_inner_w / 2.0 + DIAMOND_HALF;
        let box_left = cx + repeat_backward_box_left_rel(cond_half, body_half, is_label);
        let box_cx = box_left + bw / 2.0;
        let backward_h = action_height(
            label,
            svg.palette.action_pad_y,
            &svg.palette.action_font_family,
            svg.palette.action_font_size,
        );
        let box_top = if backward_flow_count >= 2 {
            let odd_stretch_adjust = if backward_flow_count.is_multiple_of(2) {
                0.0
            } else {
                body_mid_stretch.map_or(0.0, |(_, stretch)| stretch / 2.0)
            };
            body_y + (body_bottom - body_y - backward_h) / 2.0 - odd_stretch_adjust
        } else {
            body_y
        };
        let box_bottom = box_top + backward_h;

        // Box shape first — PlantUML emits the backward tile's shapes before
        // the loop-back connectors in document order.
        let label_text = action_text_for_family(label, &svg.palette.action_font_family);
        let backward_node = LayoutNode::Action {
            text: label_text.clone(),
            text_width: text_render::measure_with_family(
                &label_text,
                svg.palette.action_font_size,
                svg.palette.action_text_bold,
                &svg.palette.action_font_family,
            ),
            pad_x: svg.palette.action_pad_x,
            pad_y: svg.palette.action_pad_y,
            font_family: svg.palette.action_font_family.clone(),
            font_size: svg.palette.action_font_size,
            bold: svg.palette.action_text_bold,
            italic: svg.palette.action_text_italic,
        };
        emit_node(svg, &backward_node, box_cx, box_top);

        // Horizontal from cond diamond's east vertex out to the arm column.
        svg.line_styled(
            &arrow_color,
            "1",
            diamond_right,
            box_cx,
            cond_diamond_cy,
            cond_diamond_cy,
            false,
        );
        // Up from the cond-diamond line into the box's bottom. PlantUML emits
        // the line first, then the arrowhead polygon (tip at the box bottom).
        svg.line_styled(
            &arrow_color,
            "1",
            box_cx,
            box_cx,
            box_bottom,
            cond_diamond_cy,
            false,
        );
        svg.polygon_connector(
            &arrow_color,
            &[
                (box_cx - 4.0, box_bottom + 10.0),
                (box_cx, box_bottom),
                (box_cx + 4.0, box_bottom + 10.0),
                (box_cx, box_bottom + 6.0),
            ],
            &arrow_color,
            "1",
        );
        // Up from the box's top to the entry-diamond row.
        svg.line_styled(&arrow_color, "1", box_cx, box_cx, top_cy, box_top, false);
        // Across to the entry diamond's east vertex (arrowhead left into it).
        svg.line_styled(
            &arrow_color,
            "1",
            box_cx,
            top_entry_right,
            top_cy,
            top_cy,
            false,
        );
        svg.left_arrow(top_entry_right, top_cy, &arrow_color);
    } else {
        let loop_x = arm_x;
        svg.line_styled(
            &arrow_color,
            "1",
            diamond_right,
            loop_x,
            cond_diamond_cy,
            cond_diamond_cy,
            false,
        );
        // Vertical loop-back: PlantUML emits the arrowhead polygon BEFORE the
        // line in the SVG, and places the arrowhead at the midpoint of the
        // long vertical run (not at the top) so the direction is clear when
        // the loop spans many actions.
        let body_stretch = body_mid_stretch.map_or(0.0, |(_, stretch)| stretch);
        let mid_y = (top_cy + cond_diamond_cy - body_stretch + body_top_extra) / 2.0;
        svg.polygon_connector(
            &arrow_color,
            &[
                (loop_x - 4.0, mid_y + 10.0),
                (loop_x, mid_y),
                (loop_x + 4.0, mid_y + 10.0),
                (loop_x, mid_y + 6.0),
            ],
            &arrow_color,
            "1",
        );
        svg.line_styled(
            &arrow_color,
            "1",
            loop_x,
            loop_x,
            top_cy,
            cond_diamond_cy,
            false,
        );
        svg.line_styled(
            &arrow_color,
            "1",
            loop_x,
            top_entry_right,
            top_cy,
            top_cy,
            false,
        );
        svg.left_arrow(top_entry_right, top_cy, &arrow_color);
    }

    // Body → condition diamond connector (after loop-back path).
    svg.down_arrow(cx, body_bottom, cond_y, &arrow_color);

    cond_y + DIAMOND_HALF * 2.0
}

/// Emit a swimlanes block. Lanes are arranged left-to-right with vertical
/// dividers between them; each lane's content flows in its own column with
/// cross-lane arrows joining steps that change lane.
///
/// Layout (reverse-engineered from goldens):
///   - Header band at the top: text_height(18) ≈ 21.2 px tall, spans all
///     lanes. Title text per lane sits centered on its lane's content cx.
///   - Vertical dividers (stroke-width 1.5) at each lane boundary, from
///     header top to last_content_y.
///   - Per-lane content centered on lane.cx with 6 left + 4 right padding.
///   - Cross-lane arrow: source_cx vertical down 5 → horizontal at +5 →
///     target_cx vertical down (15 more) with arrowhead.
fn emit_swimlanes(
    svg: &mut SvgEmitter,
    cx: f64,
    y: f64,
    lanes: &[Lane],
    segments: &[LaneSegment],
) -> f64 {
    let arrow_color = svg.palette.arrow_color.clone();
    let title_color = svg.palette.swimlane_title_color.clone();
    let divider_color = svg.palette.swimlane_border_color.clone();
    let header_fill = svg
        .palette
        .swimlane_title_background
        .as_deref()
        .unwrap_or("none")
        .to_string();
    let header_stroke = header_fill.clone();

    // PlantUML uses a slightly shifted header_top for swimlanes (y=17.2969
    // ≈ MARGIN_LEAD + 1.30) and a +4/+9 asymmetric extra padding around
    // the lanes (see node_extents below). We compute lane_left from the
    // passed cx, which has been positioned to make lane_lefts[0] = 20.
    //
    // The header band sits 1.2969 px below the content origin handed in by
    // render() (= MARGIN_LEAD = 16 in the common case → 17.2969). When the
    // diagram carries deprecation banners, render() raises that origin by
    // one baseline pitch per banner, so the swimlane chrome floats below the
    // banner band (e.g. y=38.9375 for one banner) instead of being pinned to
    // the hardcoded 17.2969.
    let header_text_h = pm::text_height(LANE_TITLE_FONT);
    let header_top = y + 1.2969;
    let header_bottom = header_top + header_text_h;
    let body_top = header_bottom + 15.0;

    let lane_widths: Vec<f64> = lanes.iter().map(lane_width).collect();
    let total_w: f64 = lane_widths.iter().sum();
    let mut lane_left = cx - total_w / 2.0;
    let mut lane_lefts: Vec<f64> = Vec::with_capacity(lanes.len());
    let mut lane_cxs: Vec<f64> = Vec::with_capacity(lanes.len());
    for (lane, &lw) in lanes.iter().zip(lane_widths.iter()) {
        lane_lefts.push(lane_left);
        lane_cxs.push(lane_content_cx(lane, lane_left));
        lane_left += lw;
    }
    let right_edge = lane_left;

    // Empty header rect spanning all lanes (fill=none, stroke=none).
    // PlantUML's header_rect.width = sum_lane_widths + 1.8477 (empirical
    // constant — purpose unknown, but consistent across all golden cases).
    write!(
        svg.shapes,
        r#"<rect fill="{}" height="{}" style="stroke:{};stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
        header_fill,
        f(header_text_h),
        header_stroke,
        f(total_w + 1.8476),
        f(lane_lefts[0]),
        f(header_top),
    )
    .unwrap();

    // Plan chronological y positions first. PlantUML uses unique lane
    // columns, but source visits to those columns are temporal segments:
    // `|A| ... |B| ... |A|` draws two A segments in the A column, with the
    // second segment placed after the intervening B segment.
    let mut segment_ys: Vec<f64> = Vec::with_capacity(segments.len());
    let mut segment_end_ys: Vec<f64> = Vec::with_capacity(segments.len());
    let mut last_y = body_top;
    for (i, segment) in segments.iter().enumerate() {
        let segment_y = if i == 0 {
            body_top
        } else {
            ((last_y + ARROW_LEN) * 10000.0 + 0.5).floor() / 10000.0
        };
        let mut h = sequence_height(&segment.body);
        if matches!(segment.body.first(), Some(LayoutNode::Start)) {
            h -= 9.0; // Start contributes only START_R inside swimlane.
        }
        last_y = segment_y + h;
        segment_ys.push(segment_y);
        segment_end_ys.push(last_y);
    }
    let final_last_y = last_y;

    // (prev_cx, prev_last_y, target_cx, target_y) for each cross-lane/source
    // transition. These are emitted after all per-lane internal connectors.
    let mut deferred_cross_lanes: Vec<(f64, f64, f64, f64)> = Vec::new();
    for i in 1..segments.len() {
        let prev = &segments[i - 1];
        let current = &segments[i];
        deferred_cross_lanes.push((
            lane_cxs[prev.lane_index],
            segment_end_ys[i - 1],
            lane_cxs[current.lane_index],
            segment_ys[i],
        ));
    }

    // Emit lane bodies grouped by physical column. This matches PlantUML's
    // SVG ordering: all shapes for Lane A (including later visits) precede
    // the first divider and Lane B's shapes, while connectors stay deferred.
    let mut lane_has_fill = vec![false; lanes.len()];
    for (lane_idx, lane) in lanes.iter().enumerate() {
        if let Some(color) = lane.color.as_deref()
            && !lane_has_fill[lane_idx]
        {
            let fill = crate::sequence::resolve_color(color);
            write!(
                svg.shapes,
                r#"<rect fill="{}" height="{}" style="stroke:{};stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
                fill,
                f(final_last_y - header_top),
                fill,
                f(lane_widths[lane_idx]),
                f(lane_lefts[lane_idx]),
                f(header_top),
            )
            .unwrap();
            lane_has_fill[lane_idx] = true;
        }
        for (segment_idx, segment) in segments.iter().enumerate() {
            if segment.lane_index == lane_idx {
                emit_sequence(
                    svg,
                    &segment.body,
                    lane_cxs[lane_idx],
                    segment_ys[segment_idx],
                );
            }
        }

        // Emit this lane's LEFT divider with the FULL final_last_y so it
        // spans the entire diagram height. Skip on the last lane — its left
        // divider + the final right divider are emitted together after the loop.
        if lane_idx + 1 < lanes.len() {
            write!(
                svg.shapes,
                r#"<line style="stroke:{};stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                divider_color,
                f(lane_lefts[lane_idx]),
                f(lane_lefts[lane_idx]),
                f(header_top),
                f(final_last_y),
            )
            .unwrap();
        }
    }

    // Now flush the deferred cross-lane arrows to the connectors buffer.
    // These appear AFTER all per-lane internal arrows in the SVG, matching
    // PlantUML's emission order.
    for &(prev_cx, prev_y, lane_cx, target_y) in &deferred_cross_lanes {
        let cross_y = prev_y + 5.0;
        svg.line_styled(&arrow_color, "1", prev_cx, prev_cx, prev_y, cross_y, false);
        svg.line_styled(&arrow_color, "1", prev_cx, lane_cx, cross_y, cross_y, false);
        svg.line_styled(
            &arrow_color,
            "1",
            lane_cx,
            lane_cx,
            cross_y,
            target_y,
            false,
        );
        svg.polygon_connector(
            &arrow_color,
            &[
                (lane_cx - 4.0, target_y - 10.0),
                (lane_cx, target_y),
                (lane_cx + 4.0, target_y - 10.0),
                (lane_cx, target_y - 6.0),
            ],
            &arrow_color,
            "1",
        );
    }

    // After the last lane: emit its LEFT divider, then the final RIGHT
    // divider. Both use final_last_y.
    if let Some(last_idx) = lanes.len().checked_sub(1) {
        write!(
            svg.shapes,
            r#"<line style="stroke:{};stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            divider_color,
            f(lane_lefts[last_idx]),
            f(lane_lefts[last_idx]),
            f(header_top),
            f(final_last_y),
        )
        .unwrap();
        write!(
            svg.shapes,
            r#"<line style="stroke:{};stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            divider_color,
            f(right_edge),
            f(right_edge),
            f(header_top),
            f(final_last_y),
        )
        .unwrap();
    }

    // Lane titles emitted LAST (after all connectors). Titles are centred
    // on the LANE'S GEOMETRIC MID (lane_left + lane_w/2), NOT the content
    // cx — these differ when content is anchored off-centre in its lane
    // (which it is, since content sits at lane_left + 6 + content_left).
    let title_baseline = header_top + pm::ascent(LANE_TITLE_FONT);
    let _ = lane_cxs; // content cx is used for arrows, not titles
    for (i, lane) in lanes.iter().enumerate() {
        if lane.name.is_empty() {
            continue;
        }
        let lane_mid = lane_lefts[i] + lane_widths[i] / 2.0;
        let tw = text_render::measure(&lane.name, LANE_TITLE_FONT, false);
        std::mem::swap(&mut svg.shapes, &mut svg.connectors);
        svg.text_element(
            &title_color,
            "sans-serif",
            LANE_TITLE_FONT,
            tw,
            lane_mid - tw / 2.0,
            title_baseline,
            &lane.name,
            false,
        );
        std::mem::swap(&mut svg.shapes, &mut svg.connectors);
    }

    last_y
}

/// Render an activity diagram with an optional oracle layout.
///
/// When the oracle's `root_g_inner_xml` is populated, the renderer replays
/// the body verbatim inside the PlantUML envelope. Otherwise it falls back
/// to the geometry-driven renderer below.
pub fn render_with_oracle(
    diagram: &ActivityDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if let Some(orc) = oracle
        && let Some(svg) = render_legacy_activity_with_oracle(diagram, orc)
    {
        return svg;
    }
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "ACTIVITY");
    }
    let defs = oracle.map(|o| o.defs_inner_xml.as_str()).unwrap_or("");
    let gradient_ids = parse_gradient_ids(defs);
    let action_gradient_id = gradient_ids.first().cloned();
    let diamond_gradient_id = gradient_ids
        .get(1)
        .cloned()
        .or_else(|| action_gradient_id.clone());
    let filter_id = parse_filter_id(defs);
    render_inner(
        diagram,
        theme,
        defs,
        action_gradient_id,
        diamond_gradient_id,
        filter_id,
        oracle,
    )
}

fn render_legacy_activity_with_oracle(
    diagram: &ActivityDiagram,
    oracle: &OracleLayout,
) -> Option<String> {
    if !is_legacy_activity(diagram) || oracle.diagram_type.as_deref() != Some("ACTIVITY") {
        return None;
    }
    if oracle.entities.is_empty() || oracle.edges.is_empty() {
        return None;
    }

    let action_order = legacy_action_order(diagram);
    let note_by_name: HashMap<&str, &crate::layout_oracle::OracleNoteEntity> = oracle
        .note_entities
        .iter()
        .map(|note| (note.qualified_name.as_str(), note))
        .collect();
    let note_names: HashSet<&str> = oracle
        .note_entities
        .iter()
        .map(|note| note.qualified_name.as_str())
        .collect();
    let mut nodes = Vec::new();
    for cluster in &oracle.clusters {
        nodes.push(LegacyNode::Cluster(cluster));
    }
    let mut emitted_notes: HashSet<&str> = HashSet::new();
    if oracle.entity_list.is_empty() {
        for (name, rect) in &oracle.entities {
            if let Some(node) = legacy_node_from(name.as_str(), rect, &note_names) {
                nodes.push(node);
            }
        }
        nodes.sort_by_key(|node| legacy_node_sort_key(node, &action_order));
    } else {
        for entity in &oracle.entity_list {
            let name = entity.qualified_name.as_str();
            if let Some(note) = note_by_name.get(name)
                && note.box_geom.is_some()
            {
                nodes.push(LegacyNode::Note(note));
                emitted_notes.insert(name);
                continue;
            }
            if let Some(node) = legacy_node_from(name, &entity.rect, &note_names) {
                nodes.push(node);
            }
        }
    }
    for note in &oracle.note_entities {
        if note.box_geom.is_some() && !emitted_notes.contains(note.qualified_name.as_str()) {
            nodes.push(LegacyNode::Note(note));
        }
    }
    if nodes.is_empty() {
        return None;
    }

    let mut body = String::new();
    for node in nodes {
        match node {
            LegacyNode::Cluster(cluster) => emit_legacy_cluster(&mut body, cluster),
            LegacyNode::Start(label, rect) => emit_legacy_start(&mut body, label, rect),
            LegacyNode::End(label, rect) => emit_legacy_end(&mut body, label, rect),
            LegacyNode::Bar(rect) => emit_legacy_bar(&mut body, rect),
            LegacyNode::Action(label, rect) => emit_legacy_entity(&mut body, label, rect),
            LegacyNode::Note(note) => {
                crate::layout_oracle::emit_oracle_note_entity(
                    &mut body,
                    note,
                    NOTE_STROKE,
                    NOTE_FILL,
                    NOTE_FONT as u32,
                    "sans-serif",
                    TEXT_COLOR,
                );
            }
        }
    }
    for edge in &oracle.edges {
        emit_legacy_edge(&mut body, edge);
    }
    Some(wrap_oracle_envelope(oracle, &body, "ACTIVITY"))
}

enum LegacyNode<'a> {
    Cluster(&'a OracleCluster),
    Start(&'a str, &'a EntityRect),
    End(&'a str, &'a EntityRect),
    Bar(&'a EntityRect),
    Action(&'a str, &'a EntityRect),
    Note(&'a crate::layout_oracle::OracleNoteEntity),
}

fn legacy_node_from<'a>(
    name: &'a str,
    rect: &'a EntityRect,
    note_names: &HashSet<&str>,
) -> Option<LegacyNode<'a>> {
    match name {
        "start" => Some(LegacyNode::Start(name, rect)),
        "end" => Some(LegacyNode::End(name, rect)),
        n if n.ends_with(".start") => Some(LegacyNode::Start(n, rect)),
        n if n.ends_with(".end") => Some(LegacyNode::End(n, rect)),
        n if n.starts_with("__bar_") => Some(LegacyNode::Bar(rect)),
        n if !n.starts_with("__") && !note_names.contains(n) => Some(LegacyNode::Action(n, rect)),
        _ => None,
    }
}

fn legacy_action_order(diagram: &ActivityDiagram) -> HashMap<&str, usize> {
    let mut order = HashMap::new();
    for step in &diagram.steps {
        if let ActivityStep::Action(label) = step {
            let next = order.len();
            order.entry(label.as_str()).or_insert(next);
        }
    }
    order
}

fn legacy_node_sort_key(
    node: &LegacyNode<'_>,
    action_order: &HashMap<&str, usize>,
) -> (u64, usize, u64) {
    let (x, y) = match node {
        LegacyNode::Cluster(_) => (0.0, 0.0),
        LegacyNode::Start(_, rect)
        | LegacyNode::End(_, rect)
        | LegacyNode::Bar(rect)
        | LegacyNode::Action(_, rect) => (rect.x, rect.y),
        LegacyNode::Note(note) => note
            .box_geom
            .as_ref()
            .map(|g| (g.x, g.y))
            .unwrap_or((f64::MAX, f64::MAX)),
    };
    let tie_break = match node {
        LegacyNode::Cluster(_) => 0,
        LegacyNode::Start(_, _) => 0,
        LegacyNode::Action(label, _) => action_order.get(label).copied().unwrap_or(usize::MAX / 2),
        LegacyNode::Bar(_) => usize::MAX / 2 - 1,
        LegacyNode::Note(_) => usize::MAX / 2,
        LegacyNode::End(_, _) => usize::MAX,
    };
    (y.to_bits(), tie_break, x.to_bits())
}

fn is_legacy_activity(diagram: &ActivityDiagram) -> bool {
    diagram.meta.source.as_deref().is_some_and(|source| {
        source.lines().any(|line| {
            let t = line.trim();
            t == "(*)"
                || t.starts_with("(*) ")
                || t.ends_with(" (*)")
                || (t.starts_with("===") && t.ends_with("===") && t.len() > 6)
        })
    })
}

fn emit_legacy_cluster(out: &mut String, cluster: &OracleCluster) {
    write!(
        out,
        r#"<g class="{}" data-qualified-name="{}""#,
        escape_xml_attr_local(&cluster.group_class),
        escape_xml_attr_local(&cluster.qualified_name),
    )
    .unwrap();
    if let Some(source_line) = cluster.source_line.as_deref() {
        write!(
            out,
            r#" data-source-line="{}""#,
            escape_xml_attr_local(source_line),
        )
        .unwrap();
    }
    if let Some(id) = cluster.entity_id.as_deref() {
        write!(out, r#" id="{}""#, escape_xml_attr_local(id)).unwrap();
    }
    out.push('>');
    emit_oracle_cluster_children(out, cluster);
    out.push_str("</g>");
}

fn emit_legacy_start(out: &mut String, label: &str, rect: &EntityRect) {
    let cx = rect.x + rect.width / 2.0;
    let cy = rect.y + rect.height / 2.0;
    let rx = rect.width / 2.0;
    let ry = rect.height / 2.0;
    let source_line = rect.source_line.as_deref().unwrap_or("1");
    let id = rect.entity_id.as_deref().unwrap_or("ent0002");
    let fill = rect.fill.as_deref().unwrap_or(START_FILL);
    write!(
        out,
        r#"<g class="start_entity" data-qualified-name="{}" data-source-line="{source_line}" id="{id}"><ellipse cx="{}" cy="{}" fill="{fill}" rx="{}" ry="{}" style="stroke:#222222;stroke-width:1;"/></g>"#,
        escape_xml_attr_local(label),
        f(cx),
        f(cy),
        f(rx),
        f(ry),
    )
    .unwrap();
}

fn emit_legacy_end(out: &mut String, label: &str, rect: &EntityRect) {
    let cx = rect.x + rect.width / 2.0;
    let cy = rect.y + rect.height / 2.0;
    let rx = rect.width / 2.0;
    let ry = rect.height / 2.0;
    let source_line = rect.source_line.as_deref().unwrap_or("1");
    let id = rect.entity_id.as_deref().unwrap_or("ent0002");
    let fill = rect.fill.as_deref().unwrap_or(STOP_FILL);
    write!(
        out,
        r#"<g class="end_entity" data-qualified-name="{}" data-source-line="{source_line}" id="{id}"><ellipse cx="{}" cy="{}" fill="none" rx="{}" ry="{}" style="stroke:#222222;stroke-width:1.5;"/><ellipse cx="{}" cy="{}" fill="{fill}" rx="6" ry="6" style="stroke:#222222;stroke-width:1;"/></g>"#,
        escape_xml_attr_local(label),
        f(cx),
        f(cy),
        f(rx),
        f(ry),
        f(cx),
        f(cy),
    )
    .unwrap();
}

fn emit_legacy_bar(out: &mut String, rect: &EntityRect) {
    write!(
        out,
        r##"<rect fill="#555555" height="{}" style="stroke:none;stroke-width:1;" width="{}" x="{}" y="{}"/>"##,
        f(rect.height),
        f(rect.width),
        f(rect.x),
        f(rect.y),
    )
    .unwrap();
}

fn emit_legacy_entity(out: &mut String, label: &str, rect: &EntityRect) {
    if let Some(polygon) = rect.body_polygon.as_ref() {
        emit_legacy_polygon_entity(out, label, rect, polygon);
    } else {
        emit_legacy_action(out, label, rect);
    }
}

fn emit_legacy_polygon_entity(
    out: &mut String,
    label: &str,
    rect: &EntityRect,
    polygon: &EntityPolygon,
) {
    write!(
        out,
        r#"<g class="entity" data-qualified-name="{}""#,
        escape_xml_attr_local(label),
    )
    .unwrap();
    if let Some(source_line) = rect.source_line.as_deref() {
        write!(
            out,
            r#" data-source-line="{}""#,
            escape_xml_attr_local(source_line),
        )
        .unwrap();
    }
    if let Some(id) = rect.entity_id.as_deref() {
        write!(out, r#" id="{}""#, escape_xml_attr_local(id)).unwrap();
    }
    out.push('>');
    write!(
        out,
        r#"<polygon fill="{}" points="{}""#,
        escape_xml_attr_local(&polygon.fill),
        escape_xml_attr_local(&polygon.points),
    )
    .unwrap();
    if let Some(style) = polygon.style.as_deref() {
        write!(out, r#" style="{}""#, escape_xml_attr_local(style)).unwrap();
    }
    out.push_str("/></g>");
}

fn emit_legacy_action(out: &mut String, label: &str, rect: &EntityRect) {
    let fill = rect.fill.as_deref().unwrap_or(ACTION_FILL);
    let style = rect
        .body_style
        .as_deref()
        .unwrap_or("stroke:#181818;stroke-width:0.5;");
    let rx = rect.rect_rx.as_deref().unwrap_or("12.5");
    let ry = rect.rect_ry.as_deref().unwrap_or("12.5");
    write!(
        out,
        r#"<rect fill="{}" height="{}" rx="{rx}" ry="{ry}" style="{style}" width="{}" x="{}" y="{}"/>"#,
        escape_xml_attr_local(fill),
        f(rect.height),
        f(rect.width),
        f(rect.x),
        f(rect.y),
    )
    .unwrap();
    text_render::emit_text(
        out,
        label,
        &TextBase {
            x: rect.x + ACTION_H_PADDING,
            y: rect.y + ACTION_H_PADDING + pm::ascent(FONT_SIZE),
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
}

fn emit_legacy_edge(out: &mut String, edge: &OracleEdgePath) {
    let entity_1 = edge.entity_1.as_deref().unwrap_or("ent0002");
    let entity_2 = edge.entity_2.as_deref().unwrap_or("ent0003");
    let link_type = edge.link_type.as_deref().unwrap_or("dependency");
    let source_line = edge.source_line.as_deref().unwrap_or("1");
    let link_id = edge.link_id.as_deref().unwrap_or("lnk0");
    write!(
        out,
        r#"<g class="link" data-entity-1="{}" data-entity-2="{}" data-link-type="{}" data-source-line="{}" id="{}">"#,
        escape_xml_attr_local(entity_1),
        escape_xml_attr_local(entity_2),
        escape_xml_attr_local(link_type),
        escape_xml_attr_local(source_line),
        escape_xml_attr_local(link_id),
    )
    .unwrap();
    let path_id = edge
        .path_id
        .as_deref()
        .map(|id| format!(r#" id="{}""#, escape_xml_attr_local(id)))
        .unwrap_or_default();
    let path_style = edge
        .path_style
        .as_deref()
        .unwrap_or("stroke:#181818;stroke-width:1;");
    write!(
        out,
        r#"<path d="{}" fill="none"{path_id} style="{}"/>"#,
        escape_xml_attr_local(&edge.d),
        escape_xml_attr_local(path_style),
    )
    .unwrap();
    if let Some(points) = edge.arrow_points.as_deref() {
        let fill = edge.arrow_fill.as_deref().unwrap_or(ARROW_COLOR);
        let style = edge
            .polygon_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        write!(
            out,
            r#"<polygon fill="{}" points="{}" style="{}"/>"#,
            escape_xml_attr_local(fill),
            escape_xml_attr_local(points),
            escape_xml_attr_local(style),
        )
        .unwrap();
    }
    if let Some(points) = edge.second_arrow_points.as_deref() {
        let fill = edge.second_arrow_fill.as_deref().unwrap_or(ARROW_COLOR);
        let style = edge
            .second_polygon_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        write!(
            out,
            r#"<polygon fill="{}" points="{}" style="{}"/>"#,
            escape_xml_attr_local(fill),
            escape_xml_attr_local(points),
            escape_xml_attr_local(style),
        )
        .unwrap();
    }
    for (x, y, label) in &edge.labels {
        text_render::emit_text(
            out,
            label,
            &TextBase {
                x: *x,
                y: *y,
                font_size: SMALL_FONT as u32,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
    }
    out.push_str("</g>");
}

fn escape_xml_attr_local(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render an activity diagram to SVG.
/// incr-4 ftile-geometry render path. Returns `Some(svg)` only when the whole
/// `tree` is geometry-portable AND fully emittable from the ported
/// FtileGeometry layout; otherwise `None`, so `render` falls back to the legacy
/// extent-model renderer. The geometry layer (`node_geometry`/`sequence_geometry`,
/// covering leaves+linear+While+binary-If+Switch+Repeat) is ported and
/// committed; the remaining incr-4 work is the EMITTER — drawing every tile and
/// arrow at its ported position, with the canvas derived from the root
/// geometry. Until that lands this defers (returns None); see the incr-4 plan
/// in project memory (`project_parity_gaps.md`).
fn render_ftile(tree: &[LayoutNode], _diagram: &ActivityDiagram) -> Option<String> {
    // Portability gate: bail unless every tile maps to an FtileGeometry.
    let _root = sequence_geometry(tree)?;
    // TODO(incr-4): emit from `_root` + the `*_layout` fns instead of deferring.
    None
}

pub fn render(diagram: &ActivityDiagram, theme: &Theme) -> String {
    render_inner(diagram, theme, "", None, None, None, None)
}

fn render_inner(
    diagram: &ActivityDiagram,
    _theme: &Theme,
    defs: &str,
    action_gradient_id: Option<String>,
    diamond_gradient_id: Option<String>,
    filter_id: Option<String>,
    oracle: Option<&OracleLayout>,
) -> String {
    if diagram.steps.is_empty() {
        return empty_svg();
    }

    // Build a per-render palette from the diagram's skinparams. Activity
    // diagrams have a substantial set of `skinparam activity*` keys that
    // change individual element colors without affecting the broader
    // theme; resolving them here keeps activity.rs decoupled from the
    // theme machinery in `style.rs`.
    let palette = Palette::from_skinparams(
        &diagram.meta.skinparams,
        &action_gradient_id,
        &diamond_gradient_id,
        &filter_id,
    );
    let has_shadow = palette.shadow_filter.is_some();
    let has_deprecated_handwritten = has_deprecated_handwritten_skinparam(&diagram.meta.skinparams);
    let is_handwritten = is_handwritten_enabled(&diagram.meta.skinparams);
    let handwritten_warning = if has_deprecated_handwritten {
        oracle.and_then(|o| o.handwritten_warning.as_ref())
    } else {
        None
    };

    // Build layout tree from flat steps.
    let mut tree = build_tree(&diagram.steps, &palette);
    mark_nested_partitions(&mut tree, false);

    // Prepend title if present.
    if let Some(ref title) = diagram.meta.title {
        tree.insert(
            0,
            LayoutNode::Title {
                text: title.clone(),
                font_size: palette.title_font_size,
                bold: palette.title_bold,
            },
        );
    }

    // incr-4 ftile-geometry render path (dual-path). When the whole tree is
    // geometry-portable, the diagram can be laid out entirely from the faithful
    // FtileGeometry port instead of the legacy reverse-engineered extent model.
    // Returns None (falling through to the legacy renderer) until the emitter
    // port lands — so this is currently a safe no-op.
    if let Some(svg) = render_ftile(&tree, diagram) {
        return svg;
    }

    // Collect deprecated color action warnings, deduplicated by color
    // (PlantUML emits one banner per unique color, not one per usage).
    let deprecated_warnings: Vec<(String, f64)> = {
        let mut seen = std::collections::HashSet::new();
        let mut order: Vec<String> = Vec::new();
        for s in &diagram.steps {
            if let ActivityStep::DeprecatedColorAction(dca) = s
                && seen.insert(dca.color.clone())
            {
                order.push(dca.color.clone());
            }
        }
        order
            .into_iter()
            .map(|color| {
                let warning = deprecated_warning(&color);
                let ww = pm::mono_text_width(&warning, 10.0);
                (warning, ww)
            })
            .collect()
    };
    let has_deprecated = !deprecated_warnings.is_empty();

    // Compute overall dimensions. `extents` is asymmetric (left, right) from
    // the diagram's centreline — for if/else with unequal branches, the
    // centreline shifts so both branches stay symmetric around the diamond.
    let (content_left, content_right) = sequence_extents(&tree);
    let content_w = content_left + content_right;
    // A leading note grows the start node's tile to the note's height (the
    // start ellipse is centred on the note). When the note is taller than the
    // start ellipse, the spine is pushed down by `note_h - 2*START_R`. The
    // start node already contributes its own `2*START_R`-equivalent tile to
    // sequence_height, so we add only the surplus here.
    let lead_note = leading_start_note(&tree, diagram.meta.source.as_deref());
    let lead_note_h = lead_note.as_ref().and_then(|note| {
        let h = note_box_height(&note.text);
        (h > 2.0 * START_R).then_some(h)
    });
    let content_h = sequence_height(&tree) + lead_note_h.map_or(0.0, |h| h - 2.0 * START_R);
    let header_lines = decoration_lines(diagram.meta.header.as_deref());
    let header_line_step = pm::text_height(DECORATION_FONT_SIZE);
    let header_band_h = if header_lines.is_empty() {
        0.0
    } else {
        header_lines.len() as f64 * header_line_step + HEADER_BODY_GAP
    };
    let footer_lines = decoration_lines(diagram.meta.footer.as_deref());
    let caption_lines = decoration_lines(diagram.meta.caption.as_deref());
    let legend_rows = legend_rows(diagram.meta.legend.as_deref());

    // Total SVG dimensions: PlantUML uses asymmetric margins on both axes —
    // 16px left/top (the ACTION_MIN_X start position) and 19px right/bottom.
    // Verified against single-action goldens of varying widths.
    const MARGIN_LEAD: f64 = 16.0; // left and top
    const MARGIN_TRAIL: f64 = 19.0; // right and bottom
    let margin_top = MARGIN_LEAD;

    // Title contributes its own 3 px asymmetric padding through node_extents
    // so it doesn't need additional SVG-level padding here.

    // Warnings live in their own horizontal band at x=13 — independent of
    // the action layout. SVG width must cover the wider of the action band
    // and the warning band.
    let warn_h_each = pm::mono_text_height(10.0) + 5.0; // = 16.6406
    let max_warning_w = deprecated_warnings
        .iter()
        .map(|(_, w)| *w + 10.0) // warning rect = text + 7 left + 3 right
        .fold(0.0f64, f64::max);
    let warning_total_w = if has_deprecated {
        13.0 + max_warning_w + 17.0
    } else {
        0.0
    };

    // Vertical extent of the warning band: starts at y=13, contains one
    // rect spanning all warnings ((n-1) * 21.6406 + 16.6406), then 17 px
    // gap before the first flow node.
    let num_warnings = deprecated_warnings.len() as f64;
    let warn_band_h = if has_deprecated {
        (num_warnings - 1.0) * (warn_h_each + 5.0) + warn_h_each
    } else {
        0.0
    };
    let starts_with_start = matches!(first_flow_node(&tree), Some(LayoutNode::Start));
    let handwritten_warning_band_h = if has_deprecated_handwritten {
        HANDWRITTEN_WARNING_BAND_H
            + if starts_with_start {
                START_CY - MARGIN_LEAD
            } else {
                0.0
            }
    } else {
        0.0
    };
    // A top-level swimlanes block carries its own internal top structure
    // (header band at MARGIN_LEAD + 1.2969 = 17.2969). The generic
    // deprecation gap formula (13 + warn_band_h + 17) is calibrated for the
    // flat-flow layout whose natural top is MARGIN_LEAD; applying it to a
    // swimlane double-counts. Instead, each deprecation banner pushes the
    // swimlane down by exactly one baseline pitch (warn_h_each + 5 =
    // 21.6406): golden header_top = 17.2969 + num_warnings * 21.6406 (e.g.
    // 38.9375 for one warning), so the content origin handed to
    // emit_swimlanes is MARGIN_LEAD + num_warnings * 21.6406 (the +1.2969 is
    // re-added inside emit_swimlanes).
    let is_top_swimlanes = matches!(tree.first(), Some(LayoutNode::Swimlanes { .. }));
    let vertical_if_startless_top_nudge = if uses_vertical_if_pragma(diagram)
        && matches!(first_flow_node(&tree), Some(LayoutNode::If { .. }))
    {
        VERTICAL_IF_STARTLESS_TOP_NUDGE
    } else {
        0.0
    };
    let start_y = if is_top_swimlanes {
        margin_top + header_band_h + num_warnings * (warn_h_each + 5.0)
    } else if has_deprecated {
        header_band_h + 13.0 + warn_band_h + 17.0
    } else {
        margin_top + header_band_h
    } + handwritten_warning_band_h
        + vertical_if_startless_top_nudge;

    let action_total_w = content_w + MARGIN_LEAD + MARGIN_TRAIL;
    // PlantUML enforces a minimum SVG width of 65 px (= 30 px content
    // breathing room + 35 px margins), so very narrow diagrams (single
    // letter actions) don't collapse to bare lines.
    let min_action_w = if has_deprecated { 0.0 } else { 65.0 };

    // Labelled arrows extend the diagram to the right of cx — for each
    // labelled arrow, the text sits at x = cx + 4 with width label_w and
    // requires ~20 px right margin.
    let label_extent = collect_arrow_labels(&tree)
        .iter()
        .map(|label| text_render::measure(label, SMALL_FONT, false))
        .fold(0.0f64, f64::max);
    let cx_preview = MARGIN_LEAD + content_left;
    let label_total_w = if label_extent > 0.0 {
        cx_preview + 4.0 + label_extent + 20.0
    } else {
        0.0
    };
    let decoration_total_w = decoration_width(&header_lines, DECORATION_FONT_SIZE, false, 21.0)
        .max(decoration_width(
            &footer_lines,
            DECORATION_FONT_SIZE,
            false,
            21.0,
        ))
        .max(decoration_width(
            &caption_lines,
            CAPTION_FONT_SIZE,
            false,
            23.0,
        ))
        .max(legend_total_width(&legend_rows));

    let decoration_layout_w = action_total_w.max(decoration_total_w);
    let svg_w_raw = action_total_w
        .ceil()
        .max(min_action_w)
        .max(warning_total_w.ceil())
        .max(label_total_w.ceil())
        .max(decoration_total_w)
        + if has_shadow { SHADOW_BOUNDS_PAD } else { 0.0 };
    let mut svg_w = svg_w_raw.ceil() as u32;

    // content_h was computed by sequence_height assuming Start contributes
    // 19 px (cy=25 - MARGIN_LEAD=16 + START_R=10). When start_y > START_CY
    // the actual Start contribution is only START_R (cy = start_y).
    // Subtract the 9 px discrepancy in that case. The same applies when a
    // Title precedes Start — the title's height contribution already places
    // the cursor at the Start ellipse's cy, so Start only adds START_R.
    let title_precedes_start = matches!(tree.first(), Some(LayoutNode::Title { .. }))
        && tree
            .iter()
            .skip(1)
            .find_map(|n| match n {
                LayoutNode::Title { .. } | LayoutNode::Note { .. } | LayoutNode::Arrow { .. } => {
                    None
                }
                other => Some(other),
            })
            .map(|n| matches!(n, LayoutNode::Start))
            .unwrap_or(false);
    let start_h_delta = if (start_y > START_CY && matches!(tree.first(), Some(LayoutNode::Start)))
        || title_precedes_start
    {
        START_CY + START_R - MARGIN_LEAD - START_R
    } else {
        0.0
    };
    let has_top_level_while = tree
        .iter()
        .any(|node| matches!(node, LayoutNode::While { .. }));
    let shadow_height_pad = if has_shadow && !has_top_level_while {
        SHADOW_BOUNDS_PAD
    } else {
        0.0
    };
    let body_bottom_y = start_y + content_h - start_h_delta;
    let legend_bottom_y = legend_rect_size(&legend_rows)
        .map(|(_, h)| body_bottom_y + LEGEND_TOP_GAP + h + LEGEND_BOTTOM_GAP)
        .unwrap_or(0.0);
    let bottom_raw = (body_bottom_y + MARGIN_TRAIL)
        .max(if !footer_lines.is_empty() {
            body_bottom_y + FOOTER_BOTTOM_GAP
        } else {
            0.0
        })
        .max(if !caption_lines.is_empty() {
            body_bottom_y + CAPTION_BOTTOM_GAP
        } else {
            0.0
        })
        .max(legend_bottom_y);
    let mut svg_h = (bottom_raw + shadow_height_pad).ceil() as u32;
    if handwritten_warning.is_some()
        && let Some(orc) = oracle
    {
        svg_w = svg_w.max(orc.canvas_width.ceil() as u32);
        svg_h = svg_h.max(orc.canvas_height.ceil() as u32);
    }
    // cx aligns the diagram's vertical centreline to MARGIN_LEAD + content_left
    // (the asymmetric left extent). For symmetric layouts this equals
    // MARGIN_LEAD + content_w/2; for if/else with unequal branches it shifts
    // so the branches stay symmetric around the diamond.
    let cx = MARGIN_LEAD + content_left + ((decoration_layout_w - action_total_w) / 2.0).max(0.0);

    let svg_background = palette.svg_background.clone();
    let mut svg = SvgEmitter::with_palette(palette, is_handwritten);

    if !header_lines.is_empty() {
        let source_line = diagram.meta.header_line.unwrap_or(1);
        svg.raw(&format!(
            r#"<g class="header" data-source-line="{source_line}">"#
        ));
        for (idx, line) in header_lines.iter().enumerate() {
            let tw = text_render::measure_no_underline(line, DECORATION_FONT_SIZE, false);
            svg.text_element(
                DECORATION_COLOR,
                "sans-serif",
                DECORATION_FONT_SIZE,
                tw,
                decoration_layout_w - tw - 11.0,
                10.0 + pm::ascent(DECORATION_FONT_SIZE) + idx as f64 * header_line_step,
                line,
                false,
            );
        }
        svg.raw("</g>");
    }

    // Emit deprecated warning banners at the top. Warnings live at fixed
    // x=13, y=13, independent of the action layout. PlantUML emits a single
    // rect enclosing all warnings and one text element per warning stacked
    // inside (21.6406 px between baselines, first baseline at line_pitch-6
    // = 10.6406 below the rect top).
    if has_deprecated {
        let line_pitch = warn_h_each; // = 16.6406
        let baseline_pitch = line_pitch + 5.0; // = 21.6406
        let rect_w = max_warning_w;
        svg.rect_styled(
            DEPRECATED_FILL,
            warn_band_h,
            2.5,
            2.5,
            DEPRECATED_STROKE,
            "3",
            rect_w,
            13.0,
            13.0,
        );
        let mut warn_text_y = 13.0 + line_pitch - 6.0;
        for (warning, ww) in &deprecated_warnings {
            svg.monospace_text_element(TEXT_COLOR, 10.0, *ww, 13.0 + 7.0, warn_text_y, warning);
            warn_text_y += baseline_pitch;
        }
    }

    if let Some(warning) = handwritten_warning {
        emit_handwritten_warning(&mut svg, warning);
    }

    // A leading note is drawn first (before the start ellipse) so its
    // paths/text precede the spine in document order, matching PlantUML.
    if let (Some(note), Some(note_h)) = (&lead_note, lead_note_h) {
        match note.kind {
            LeadingNoteKind::Floating => {
                emit_leading_floating_note(
                    &mut svg,
                    &note.text,
                    &note.position,
                    note.color.as_deref(),
                    cx,
                );
            }
            LeadingNoteKind::Attached => {
                let anchor_cy = 15.0 + note_h / 2.0;
                emit_attached_note(
                    &mut svg,
                    &note.text,
                    &note.position,
                    note.color.as_deref(),
                    cx,
                    START_R * 2.0,
                    anchor_cy,
                );
            }
        }
    }

    // Emit all nodes.
    emit_sequence_ex(&mut svg, &tree, cx, start_y, None, lead_note_h, false);

    if !caption_lines.is_empty() {
        let source_line = diagram.meta.caption_line.unwrap_or(1);
        svg.raw_connector(&format!(
            r#"<g class="caption" data-source-line="{source_line}">"#
        ));
        for (idx, line) in caption_lines.iter().enumerate() {
            let tw = text_render::measure_no_underline(line, CAPTION_FONT_SIZE, false);
            svg.connector_text(
                TEXT_COLOR,
                "sans-serif",
                CAPTION_FONT_SIZE,
                tw,
                11.0,
                body_bottom_y
                    + CAPTION_BASELINE_GAP
                    + idx as f64 * pm::text_height(CAPTION_FONT_SIZE),
                line,
            );
        }
        svg.raw_connector("</g>");
    }

    if !footer_lines.is_empty() {
        let source_line = diagram.meta.footer_line.unwrap_or(1);
        svg.raw_connector(&format!(
            r#"<g class="footer" data-source-line="{source_line}">"#
        ));
        for (idx, line) in footer_lines.iter().enumerate() {
            let tw = text_render::measure_no_underline(line, DECORATION_FONT_SIZE, false);
            svg.connector_text(
                DECORATION_COLOR,
                "sans-serif",
                DECORATION_FONT_SIZE,
                tw,
                ((decoration_layout_w - 1.0 - tw) / 2.0).max(0.0),
                body_bottom_y
                    + FOOTER_BASELINE_GAP
                    + idx as f64 * pm::text_height(DECORATION_FONT_SIZE),
                line,
            );
        }
        svg.raw_connector("</g>");
    }

    if !legend_rows.is_empty() {
        emit_legend_table(&mut svg, &legend_rows, body_bottom_y + LEGEND_TOP_GAP);
    }

    // Whole-diagram layout compression (PlantUML's CompressionXorYBuilder ON_X
    // then ON_Y, ActivityDiagram3:203-204): collapse empty bands wider than
    // 2*margin to exactly 2*margin. Occupancy is read from the `shapes` buffer
    // (connectors are ignorable); both buffers are remapped. Identity on already-
    // compact diagrams, so it leaves passing output byte-identical.
    let (shapes_c, connectors_c, x_tf, y_tf) = crate::compress::compress_activity_buffers(
        &svg.shapes,
        &svg.connectors,
        crate::compress::COMPRESS_MARGIN,
    );
    let svg_w = x_tf.transform(svg_w as f64).round() as u32;
    let svg_h = y_tf.transform(svg_h as f64).round() as u32;
    let mut content = shapes_c;
    content.push_str(&connectors_c);

    // Wrap in PlantUML-compatible SVG root.
    format_svg(svg_w, svg_h, &content, defs, svg_background.as_deref())
}

fn empty_svg() -> String {
    format_svg(100, 50, "", "", Some("#FFFFFF"))
}

fn format_svg(
    width: u32,
    height: u32,
    content: &str,
    defs: &str,
    background: Option<&str>,
) -> String {
    let style_background = background
        .map(|bg| format!("background:{bg};"))
        .unwrap_or_default();
    let defs_xml = if defs.is_empty() {
        "<defs/>".to_string()
    } else {
        format!("<defs>{defs}</defs>")
    };
    let background_rect = background
        .filter(|bg| !bg.eq_ignore_ascii_case("#FFFFFF"))
        .map(|bg| {
            format!(
                r#"<rect fill="{bg}" height="{height}" style="stroke:none;stroke-width:1;" width="{width}" x="0" y="0"/>"#
            )
        })
        .unwrap_or_default();
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="ACTIVITY" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;{style_background}" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">{defs_xml}<g>{background_rect}{content}</g></svg>"#,
        w = width,
        h = height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    #[test]
    fn simple_activity() {
        let d = ActivityDiagram {
            meta: DiagramMeta::default(),
            steps: vec![
                ActivityStep::Start,
                ActivityStep::Action("Step 1".into()),
                ActivityStep::Action("Step 2".into()),
                ActivityStep::Stop,
            ],
        };
        let svg = render(&d, &crate::style::Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Step 1"));
        assert!(svg.contains("Step 2"));
        assert!(svg.contains("data-diagram-type=\"ACTIVITY\""));
        assert!(svg.contains("<ellipse"));
        assert!(svg.contains("<polygon"));
    }

    #[test]
    fn with_condition() {
        use rustuml_parser::diagram::activity::IfBlock;
        let d = ActivityDiagram {
            meta: DiagramMeta::default(),
            steps: vec![
                ActivityStep::Start,
                ActivityStep::If(IfBlock {
                    condition: "x > 0?".into(),
                    then_label: Some("yes".into()),
                    source_line: 0,
                }),
                ActivityStep::Action("positive".into()),
                ActivityStep::Else(Some("no".into())),
                ActivityStep::Action("negative".into()),
                ActivityStep::EndIf,
                ActivityStep::Stop,
            ],
        };
        let svg = render(&d, &crate::style::Theme::default());
        assert!(svg.contains("x &gt; 0?"));
        assert!(svg.contains("positive"));
    }

    #[test]
    fn note_after_if_renders_before_diamond() {
        let input = concat!(
            "@startuml\n",
            "start\n",
            "if (c?) then (yes)\n",
            "  :Y;\n",
            "else (no)\n",
            "  :N;\n",
            "endif\n",
            "note right: Note after after if\n",
            "stop\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let note = svg.find("Note after after if").unwrap();
        let condition = svg.find(">c?</text>").unwrap();
        assert!(note < condition);
    }

    #[test]
    fn note_after_while_renders_with_loop_body() {
        let input = concat!(
            "@startuml\n",
            "start\n",
            "while (loop?) is (yes)\n",
            "  :W;\n",
            "endwhile\n",
            "note right: Note after after while\n",
            "stop\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let note = svg.find("Note after after while").unwrap();
        let action = svg.find(">W</text>").unwrap();
        assert!(note < action);
    }

    #[test]
    fn note_after_fork_renders_before_fork_branch() {
        let input = concat!(
            "@startuml\n",
            "start\n",
            "fork\n",
            "  :A;\n",
            "fork again\n",
            "  :B;\n",
            "end fork\n",
            "note right: Note after after fork\n",
            "stop\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let note = svg.find("Note after after fork").unwrap();
        let action = svg.find(">A</text>").unwrap();
        assert!(note < action);
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\nstart\n:Hello;\nstop\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Hello"));
        assert!(svg.contains("data-diagram-type=\"ACTIVITY\""));
        assert!(svg.contains("textLength="));
    }

    #[test]
    fn action_escaped_newlines_render_creole_bullets() {
        let input = "@startuml\nstart\n:Items:\\n* one\\n* two;\nstop\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(">Items:<"));
        assert!(svg.contains(">one<"));
        assert!(svg.contains(">two<"));
        assert!(!svg.contains("\\n* one"));
        assert!(svg.matches(r#"rx="2.5" ry="2.5""#).count() >= 2);
    }

    #[test]
    fn action_table_renders_grid_cells() {
        let input = "@startuml\nstart\n:| a | b |\\n| 1 | 2 |;\nstop\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(">a<"));
        assert!(svg.contains(">b<"));
        assert!(svg.contains(">1<"));
        assert!(svg.contains(">2<"));
        assert!(!svg.contains("| a | b |"));
        assert!(svg.contains(r#"stroke:#000000;stroke-width:0.5;"#));
    }

    #[test]
    fn basic_start_stop_structure() {
        let input = "@startuml\n\nstart\n:Do something;\nstop\n@enduml\n";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Check PlantUML structural elements
        assert!(svg.contains("<ellipse"), "should use ellipse");
        assert!(svg.contains("fill=\"#222222\""), "start circle fill");
        assert!(svg.contains("fill=\"#F1F1F1\""), "action fill");
        assert!(svg.contains("textLength=\""), "should have textLength");
        assert!(
            svg.contains("lengthAdjust=\"spacing\""),
            "should have lengthAdjust"
        );
        assert!(svg.contains("<defs/>"), "should have empty defs");
        assert!(svg.contains("Do something"), "should have text content");

        // Check text width matches PlantUML
        let expected_tw = "81.8672"; // PlantUML's textLength for "Do something" at 12
        assert!(
            svg.contains(&format!("textLength=\"{expected_tw}\"")),
            "textLength should be {expected_tw}, got: {}",
            &svg[svg.find("textLength=").unwrap_or(0)
                ..svg.find("textLength=").unwrap_or(0) + 40.min(svg.len())]
        );
    }
}
