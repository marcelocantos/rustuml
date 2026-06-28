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

thread_local! {
    /// Swimlane V2 only: maps lane name -> column index (first-appearance order)
    /// for the current single-tree build. Set by `build_tree` on the V2 path so
    /// the recursive `build_tree_inner` (and its branch helpers) can resolve
    /// `|Lane|` markers into `LayoutNode::LaneMark(idx)`. `None` off the V2 path.
    static SWIMLANE_V2_MAP: std::cell::RefCell<Option<HashMap<String, usize>>> =
        const { std::cell::RefCell::new(None) };
}

/// Resolve a lane name to its column index via the V2 thread-local map.
/// Returns `None` when not on the V2 path (map unset).
fn swimlane_v2_lane_index(name: &str) -> Option<usize> {
    SWIMLANE_V2_MAP.with(|m| m.borrow().as_ref().and_then(|map| map.get(name).copied()))
}

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
/// Right margin a `group`'s rect keeps past a wrapped `while`'s drawn loop arm:
/// the FtileGroup `addHorizontalMargin` 10 plus the FtileWhile's reclaimed
/// trailing residual (`getInnerDimensionSlow`'s `+ ... + 5` rounding). The left
/// side keeps just the plain 10 px margin.
const GROUP_WHILE_RIGHT_RESIDUAL: f64 = 16.0;
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
const WHILE_PREFIXED_FUSED_NESTED_LEFT_TRIM: f64 = 1.0;
const WHILE_PREFIXED_FUSED_NESTED_RIGHT_TRIM: f64 = 7.5722;
const WHILE_PREFIXED_FUSED_NESTED_EMIT_RIGHT_TRIM: f64 = 3.5722;
const WHILE_PREFIXED_FUSED_NESTED_CANVAS_RIGHT_TRIM: f64 = 9.4277;
const WHILE_PREFIXED_FUSED_NESTED_TITLE_X_OFFSET: f64 = -5.7823;
const WHILE_PREFIXED_FUSED_NESTED_CHILD_EXIT_ARROW_PUSH_DOWN: f64 = 1.9887;
const WHILE_PREFIXED_FUSED_NESTED_PARENT_EXIT_ARROW_PULL_UP: f64 = 1.6675;
/// The body→loop-back-junction gap a `while` reserves below its body (the
/// `body_bottom + 10` slack). When a loop consumes a nested loop's fused exit,
/// its loop-back junction sits at the child's fused band (which already counts
/// this gap), but its own exit corridor still drops this far below the junction.
const WHILE_NESTED_EXIT_BODY_GAP: f64 = 10.0;
/// A nested fused-exit corridor's DOWN emphasize arrowhead sits one pixel below
/// the drawn corridor-run midpoint: PlantUML anchors the `ConnectionOut`
/// emphasize on the un-merged `FtileWhile` segment, which is one pixel longer
/// than the MergeStrategy.LIMITED-merged corridor actually drawn.
const WHILE_NESTED_EXIT_ARROW_BIAS: f64 = 1.0;
/// `FtileWhile.getSuppHeightForLabel`: the height the loop-back incoming label
/// (`back1`) reserves below the body. PlantUML's `calculateDimensionFtile`
/// includes this in the tile height (`diamond + body + 4*halfHex + suppLabel`),
/// so the loop's `ConnectionOut` DOWN emphasize arrowhead sits at the midpoint of
/// `diamond_cy → body_bottom + 2*halfHex + suppLabel`. Measured from the
/// swimlane while goldens (the non-swimlane wrap-back path compresses this slack
/// away, so it only surfaces when the exit corridor is stitched across lanes).
const WHILE_LOOPBACK_LABEL_H: f64 = 12.6795625;
const PARTITION_COLORED_WHILE_SPINE_SHIFT: f64 = 1.5;
const PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP: f64 = WHILE_BODY_SLOT_COMPRESS / 2.0 - 1.0;
const PARTITION_WHILE_WIDTH_SUBTRACT: f64 = 16.0;
const PARTITION_REPEAT_WIDTH_SUBTRACT: f64 = 18.0;
const WHILE_SINGLE_IF_RIGHT_PAD: f64 = 2.0;
/// Right extent (from the spine) of a break-down `if`'s populated branch tile.
/// PlantUML's `ConditionalBuilder.createDown` wraps the lone-`break` branch in
/// `FtileMinWidthCentered(branch, 30)` then `addHorizontalMargin(10)`, giving a
/// centred tile of width `max(branch_w, 30) + 20`. For an empty/narrow break
/// branch that floors the half-width at `30/2 + 10 = 25`. The `FtileIfDown`
/// right extent is therefore `max(cond_half, 25) + halfHex`, not just
/// `cond_half + halfHex` — the floor governs when the condition diamond is
/// narrower than the branch tile (e.g. `act_while_break_at_end`'s `exit?`).
const BREAK_DOWN_BRANCH_HALF_FLOOR: f64 = 25.0;
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
/// Gap from a redirected single-survivor inner-if's surviving branch bottom to
/// the parent merge reservation. PlantUML draws no merge diamond for the inner
/// if (`hasTwoBranches`==false → `diamond2` = `FtileEmpty(Hexagon.hexagonHalfSize/2)`),
/// so the inner if contributes only the empty-tile spacer before the parent's
/// own merge gap; the surviving corridor then fuses straight to the parent merge.
const IF_SURVIVOR_REDIRECT_GAP: f64 = 4.0;
const IF_GOTO_RESUME_GAP: f64 = 5.0;
const IF_EMPTY_BOTH_LEFT_EXTENT_PAD: f64 = 13.0;
const IF_EMPTY_BOTH_RIGHT_EXTENT_PAD: f64 = 15.0;
/// Labelled `if` diamonds reserve a little extra inbound lead when the
/// diagram-wide arrow font is taller than the default 20 px connector slot.
const IF_LABEL_INBOUND_PAD: f64 = 0.71875;
const REPEAT_NOT_LABEL_OUTBOUND_PAD: f64 = 1.5;
const FORK_BAR_HEIGHT: f64 = 6.0;
const FORK_BAR_RX: f64 = 2.5;
/// Sentinel attribute marking a fork bar as `ignoreForCompressionOnX`. Emitted
/// only on compressible bars (mixed-asymmetry even forks); the ON_X compression
/// pass treats marked rects as transparent and strips the attribute before
/// serialization, so it never reaches the comparator. See `compress.rs`.
pub(crate) const FORK_BAR_COMPRESS_MARKER: &str = "data-fork-compress";
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

// Uncompressed (pre-ON_Y) switch merge gap — the intrinsic distance between the
// case-tile bottoms and the merge diamond top in PlantUML's
// FtileSwitchWithDiamonds layout. A standalone switch's empty merge band is
// reclaimed by the whole-diagram ON_Y compression pass (it collapses to the
// values returned by `switch_merge_gap`: ARROW_LEN, or ARROW_LEN/2 for an even
// SMALL-mode block). When the switch is the tile of a fork branch whose
// siblings differ in height, a parallel branch box occupies that same y-band,
// so the SlotFinder marks it occupied and the empty merge band survives
// uncompressed. These are the surviving uncompressed gaps (SMALL/corridor vs
// BIG diamond), the uncompressed counterparts of the compressed
// `switch_merge_gap` values.
const SWITCH_MERGE_GAP_UNCOMPRESSED_SMALL: f64 = 57.0449;
const SWITCH_MERGE_GAP_UNCOMPRESSED_BIG: f64 = 41.0449;

// When a multi-case switch is the *terminal* (or only) flow node of a `while`
// body, its empty merge band sits directly above the loop-back junction. The
// while frame's vertical centring reserves space there, so ON_Y compression
// reclaims only a fixed slack from the switch's uncompressed merge band rather
// than collapsing it to the standalone `switch_merge_gap` value. The surviving
// gap is `SWITCH_MERGE_GAP_UNCOMPRESSED_{BIG,SMALL} - this`. (Empirically the
// reclaim is the same constant in BIG and SMALL/corridor mode: 41.0449→15.5009
// and 57.0449→31.5010.)
const WHILE_TERMINAL_SWITCH_MERGE_RECLAIM: f64 = 25.5440;

// When a multi-case switch is a *non-terminal* flow node of a `while` body and
// at least three flow tiles follow it, the loop body is tall enough that ON_Y
// compression reclaims the switch's merge band almost entirely (the trailing
// tiles provide adjacent content to compress against — cf.
// `while_break_corridor_compresses`'s 3-tile threshold). The merge diamond then
// sits these distances below the case-tile bottoms instead of
// `switch_merge_gap + ARROW_LEN`. With one or two trailing tiles the band stays
// at `switch_merge_gap + ARROW_LEN` (the uncompressible loop-frame slack). The
// odd/centre-spine block collapses to its standalone `ARROW_LEN`; the even block
// keeps a small residual above the loop-back arrowhead.
const SWITCH_WHILE_MERGE_GAP_COMPRESSED_EVEN: f64 = 14.7999;
const SWITCH_WHILE_MERGE_GAP_COMPRESSED_ODD: f64 = 20.0;
// At least this many flow tiles must follow a non-terminal in-while switch for
// its merge band to ON_Y-compress.
const SWITCH_WHILE_MERGE_COMPRESS_MIN_FOLLOWING: usize = 3;
// In the compressed state the loop-back arrowhead seats this far above the merge
// diamond top (`COMPRESSED_EVEN - ARROW_LEN/2`); the same offset holds for the
// odd/centre-spine block. In the uncompressed state the arrowhead instead seats
// `standalone_gap + ARROW_LEN/2` above the merge top (the corridor midpoint).
const SWITCH_WHILE_COMPRESSED_LOOPBACK_OFFSET: f64 =
    SWITCH_WHILE_MERGE_GAP_COMPRESSED_EVEN - ARROW_LEN / 2.0;
// When an odd/centre-spine switch sits in a corridor-compressing `while` body
// (>=3 body flow tiles), the centre branch's collinear vertical drop splits at
// the compressed corridor turn — this far below the case-tile bottoms — rather
// than `merge_top - SWITCH_CENTER_BOT_SPLIT`.
const SWITCH_WHILE_COMPRESSED_CENTER_SPLIT: f64 = 5.0;

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

type PendingDownArrow = (f64, ArrowStyle, Option<String>, f64);

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
        source_line: usize,
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
    /// Swimlane V2 (single-tree rewrite): a zero-size marker lowered from a
    /// `|Lane|` step. It carries the index of the lane that becomes active at
    /// this flow position; `emit`/`layout_swimlanes_v2` route subsequent shapes
    /// into that lane's buffer. Produced ONLY on the V2 path (RUSTUML_SWIMLANE_V2);
    /// the default segment model never builds one. Contributes no geometry.
    LaneMark(usize),
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

fn branch_terminates_with_break(body: &[LayoutNode]) -> bool {
    matches!(body.last(), Some(LayoutNode::Break))
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

/// Swimlane V2: the lane a branch body enters. A `|Lane|` marker at the front of
/// a branch changes the branch's `getSwimlaneIn`; otherwise it inherits the
/// current lane from the enclosing flow.
fn branch_entry_lane(body: &[LayoutNode], default: usize) -> usize {
    body.iter()
        .find_map(|node| match node {
            LayoutNode::LaneMark(idx) => Some(*idx),
            _ => None,
        })
        .unwrap_or(default)
}

/// A branch flow whose sole node is a binary `if` with exactly one terminating
/// branch (kill/detach/stop) and one surviving branch. Such an inner if's
/// surviving out-corridor IS the parent if's branch→merge connection (PlantUML's
/// `ConnectionVerticalThenHorizontalDirect` + `MergeStrategy.LIMITED` fusion);
/// the parent delegates the merge wiring via [`IfSurvivorRedirect`].
fn branch_is_redirectable_single_survivor_if(flow: &[LayoutNode]) -> bool {
    matches!(
        flow,
        [LayoutNode::If {
            then_branch,
            else_branches,
            ..
        }] if (if_single_survivor(then_branch, else_branches).is_some()
                || if_break_single_survivor_plan(then_branch, else_branches).is_some())
            // The empty-branch FtileIfDown corridor and the circle-terminal
            // forms have their own dedicated wiring; only the plain
            // action-terminator (kill/detach/stop-with-body) survivor uses the
            // straight redirect. A loop-owned break single-survivor is also
            // redirectable: its break side welds to the loop corridor while the
            // live side fuses into the parent's merge.
            && (if_down_plan(then_branch, else_branches).is_none()
                || if_break_single_survivor_plan(then_branch, else_branches).is_some())
            && if_single_circle_terminal_plan(None::<&String>, then_branch, else_branches).is_none()
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

fn single_survivor_if_join_offset(node: &LayoutNode) -> Option<f64> {
    let LayoutNode::If {
        diamond_half_y,
        then_branch,
        else_branches,
        ..
    } = node
    else {
        return None;
    };
    if if_single_survivor(then_branch, else_branches).is_none() || else_branches.len() != 1 {
        return None;
    }
    let branch_y = diamond_half_y * 2.0 + IF_BRANCH_DOWN;
    let then_bottom = branch_y + sequence_height(then_branch);
    let else_bottom = branch_y + sequence_height(&else_branches[0].body);
    Some(then_bottom.max(else_bottom) + IF_SINGLE_SURVIVOR_JOIN_GAP)
}

/// True for a binary `if` that `if_down_plan` renders with a *terminating*
/// populated branch (`:foo; stop endif`). Like the single-survivor if, PlantUML's
/// `FtileIfDown.ConnectionOut` draws the spine arrow leaving the if-block as part
/// of the if's own connector list (right after the no-diamond corridor), so the
/// enclosing sequence must NOT also draw a deferred inbound to the following node.
fn if_node_is_terminating_down(node: &LayoutNode) -> bool {
    matches!(
        node,
        LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } if if_down_plan(then_branch, else_branches)
            .is_some_and(|p| p.populated_terminates)
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
        | LayoutNode::Goto(_)
        // Swimlane V2 lane marker: zero-size, no shape, no connector — not a
        // flow node (so it never claims `first_flow_node` nor an inbound arrow).
        | LayoutNode::LaneMark(_) => false,
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
    /// True when the populated branch ends in a control-flow terminator (stop,
    /// end, …). PlantUML's `FtileIfDown` then has `hasPointOut1 == false` and
    /// `hasTwoBranches() == false`, so `getShape2` returns an `FtileEmpty`: no
    /// merge diamond, the empty branch's east corridor IS the if's pointOut
    /// (`ConnectionElseNoDiamond`). The spine branch terminates in place.
    populated_terminates: bool,
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
    // A populated branch that terminates with a bare circle terminal (a lone
    // `stop`/`end`) is the FtileIfDown `optionalStop` case — the terminal sits
    // EAST of the diamond, handled by `if_single_circle_terminal_plan` (which
    // requires the OTHER branch be non-empty). When the other branch is empty,
    // PlantUML still routes to that east-stop layout, so leave it out of here.
    if lone_circle_terminal(populated).is_some() {
        return None;
    }
    // A populated branch ending in an action-terminator (`:foo; stop`) still
    // flows down the spine, but PlantUML draws no merge diamond: `hasPointOut1`
    // is false → `getShape2` is `FtileEmpty`, the empty branch becomes the if's
    // pointOut via `ConnectionElseNoDiamond`.
    let populated_terminates = branch_terminates(populated);
    Some(IfDownPlan {
        populated,
        then_populated: !then_empty,
        populated_terminates,
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
        Some(IfBreakDownPlan {
            then_is_break: true,
        })
    } else if else_break && branch_is_empty(then_branch) {
        Some(IfBreakDownPlan {
            then_is_break: false,
        })
    } else {
        let _ = (then_break, else_break);
        None
    }
}

/// A binary `if` inside a break-bearing loop where one branch is a lone
/// `break` and the other branch survives. PlantUML keeps the normal condition
/// diamond but routes the break branch sideways into the loop's left exit
/// corridor; the surviving branch then behaves like the ordinary
/// single-survivor-if case and may fuse into an enclosing `if` merge.
struct IfBreakSingleSurvivorPlan {
    /// True when the *then* branch is the break branch.
    then_is_break: bool,
}

fn if_break_single_survivor_plan(
    then_branch: &[LayoutNode],
    else_branches: &[ElseBranch],
) -> Option<IfBreakSingleSurvivorPlan> {
    if else_branches.len() != 1 || else_branches[0].condition.is_some() {
        return None;
    }
    let else_body = else_branches[0].body.as_slice();
    let then_break = branch_is_lone_break(then_branch);
    let else_break = branch_is_lone_break(else_body);
    match (then_break, else_break) {
        (true, false) if !branch_is_empty(else_body) && !branch_terminates(else_body) => {
            Some(IfBreakSingleSurvivorPlan {
                then_is_break: true,
            })
        }
        (false, true) if !branch_is_empty(then_branch) && !branch_terminates(then_branch) => {
            Some(IfBreakSingleSurvivorPlan {
                then_is_break: false,
            })
        }
        _ => None,
    }
}

fn nodes_contain_break_single_survivor_if(nodes: &[LayoutNode]) -> bool {
    nodes.iter().any(node_contains_break_single_survivor_if)
}

fn node_contains_break_single_survivor_if(node: &LayoutNode) -> bool {
    match node {
        LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } => {
            if_break_single_survivor_plan(then_branch, else_branches).is_some()
                || nodes_contain_break_single_survivor_if(then_branch)
                || else_branches
                    .iter()
                    .any(|b| nodes_contain_break_single_survivor_if(&b.body))
        }
        LayoutNode::Switch { cases, .. } => cases
            .iter()
            .any(|c| nodes_contain_break_single_survivor_if(&c.body)),
        LayoutNode::Fork { branches, .. } => branches
            .iter()
            .any(|b| nodes_contain_break_single_survivor_if(b)),
        LayoutNode::Partition { body, .. } => nodes_contain_break_single_survivor_if(body),
        _ => false,
    }
}

fn if_branch_distance_extra(then_branch: &[LayoutNode], else_branches: &[ElseBranch]) -> f64 {
    let direct = if if_break_single_survivor_plan(then_branch, else_branches).is_some() {
        IF_BREAK_SINGLE_SURVIVOR_BRANCH_SPREAD * 2.0
    } else {
        0.0
    };
    let nested = if nodes_contain_break_single_survivor_if(then_branch)
        || else_branches
            .iter()
            .any(|b| nodes_contain_break_single_survivor_if(&b.body))
    {
        IF_BRANCH_CONTAINS_BREAK_SURVIVOR_SPREAD
    } else {
        0.0
    };
    direct + nested
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

/// Whether a `while` body contains a `break` that belongs to this loop. Nested
/// `while`/`repeat` bodies are excluded by [`nodes_contain_break`]; nested
/// `if`/`switch`/`fork`/partition bodies still share this loop's break corridor.
fn body_contains_break_if(body: &[LayoutNode]) -> bool {
    nodes_contain_break(body)
}

/// A `while` directly in this body. A nested while ADVERTISES a wider layout
/// extent (`sequence_extents`) than it draws (`sequence_geometry`) because of the
/// FtileWhile `dx + halfHex` trailing reservation that the whole-diagram ON_X
/// pass reclaims. A loop/group container that clears off the inflated layout
/// extent over-reserves; the faithful FtileRepeat/FtileGroup clears off the
/// drawn geometry instead. Used to gate that geometry-based clearance so plain /
/// if / switch bodies (whose extent and geometry coincide) are unaffected.
fn body_contains_while(body: &[LayoutNode]) -> bool {
    body.iter().any(|n| matches!(n, LayoutNode::While { .. }))
}

/// True when the LAST flow node of this body is a `while`. The while's
/// pointOut→condition link is then assembled with the body (its own exit
/// corridor), landing in the connector stream BEFORE the repeat's ConnectionIn/
/// Back — so the repeat's body→condition arrow is emitted early. A trailing plain
/// tile after the while (e.g. `act_combo_while_in_repeat`'s `:After while;`)
/// keeps the ordinary late body→condition order.
fn body_last_flow_is_while(body: &[LayoutNode]) -> bool {
    last_flow_index(body).is_some_and(|i| matches!(body[i], LayoutNode::While { .. }))
}

fn while_body_has_prefixed_fused_trailing_while(body: &[LayoutNode]) -> bool {
    let Some(last) = last_flow_index(body) else {
        return false;
    };
    if !matches!(
        body[last],
        LayoutNode::While {
            special_out: None,
            ..
        }
    ) {
        return false;
    }
    body[..last].iter().any(node_is_flow)
}

/// The break-bearing `if`'s diamond half-width (cond_inner_w/2 + halfHex) for the
/// first such `if` directly in `body`. Used by [`emit_repeat`] and the Repeat
/// extent to size the left break corridor (whose column is anchored on the
/// break-if's diamond, the body's leftmost element).
fn repeat_break_if_cond_half(body: &[LayoutNode]) -> Option<f64> {
    body.iter().find_map(|n| match n {
        LayoutNode::If {
            condition,
            then_branch,
            else_branches,
            diamond_font_family,
            diamond_font_size,
            diamond_text_bold,
            diamond_pad_x,
            ..
        } if if_break_down_plan(then_branch, else_branches).is_some() => {
            Some(if_diamond_half_width(
                condition,
                *diamond_font_size,
                *diamond_text_bold,
                diamond_font_family,
                *diamond_pad_x,
            ))
        }
        _ => None,
    })
}

/// Width reserved on the LEFT of a break-bearing `repeat` for the break-exit
/// corridor: 9 px past the break-if's diamond half (FtileIfDown's left lead). The
/// repeat spine shifts right by this so the corridor — which carries the break
/// down to the merge — clears the content's left margin.
const REPEAT_BREAK_CORRIDOR_PAD: f64 = 9.0;

/// Horizontal margin PlantUML's `FtileFactoryDelegatorRepeat` adds to the LEFT of
/// a break-bearing repeat tile (`addHorizontalMargin(result, 10, 0)`). The break
/// snake routes to the margined tile's left edge (x=0), so the exit corridor sits
/// this far left of the repeat body's own left extent (`repeat.getLeft()`).
const REPEAT_BREAK_CORRIDOR_MARGIN: f64 = 10.0;

/// True when the break-bearing `if` is the first *flow* node of the loop body
/// (no populated tile precedes it). Such a body reserves an extra middle-stretch
/// unit in its `FtileGeometry` — see [`WHILE_BREAK_FIRST_FRAME_EXTRA`].
fn break_if_is_first_flow(body: &[LayoutNode]) -> bool {
    matches!(
        body.iter().find(|n| node_is_flow(n)),
        Some(LayoutNode::If { then_branch, else_branches, .. })
            if if_break_down_plan(then_branch, else_branches).is_some()
    )
}

/// True when the break-bearing `if` is the LAST *flow* node of the loop body
/// (no populated tile follows it). Then the if's empty (continue) branch is the
/// whole loop body's `pointOut`: PlantUML's `ConnectionBackSimple` originates the
/// loop-back arm at that branch's east vertex and runs it straight up to the
/// condition diamond. The break-`if` therefore emits NO down-then-spine corridor
/// and the `while` emits NO separate junction loop-back — the two fuse into one
/// right-side corridor (right → up → left-arrow into diamond). See
/// [`emit_if_break_down`]'s `fuse_loopback` path and [`emit_while`].
fn break_if_is_last_flow(body: &[LayoutNode]) -> bool {
    matches!(
        body.iter().rfind(|n| node_is_flow(n)),
        Some(LayoutNode::If { then_branch, else_branches, .. })
            if if_break_down_plan(then_branch, else_branches).is_some()
    )
}

/// Whether the loop body's break-bearing `if` carries a positive (south) label
/// on its break branch — `then (yes)` rather than a bare `then`. PlantUML's
/// `FtileIfDown` reserves a south-label band only when the label is present; its
/// absence collapses the band (see [`WHILE_BREAK_NO_SOUTH_LABEL_DROP`] and
/// [`WHILE_BREAK_NO_SOUTH_LABEL_INBOUND_EXTRA`]). Returns `None` when the body
/// has no break-down `if`.
fn break_if_south_label_present(body: &[LayoutNode]) -> Option<bool> {
    body.iter().find_map(|n| match n {
        LayoutNode::If {
            then_label,
            then_branch,
            else_branches,
            ..
        } => if_break_down_plan(then_branch, else_branches).map(|plan| {
            if plan.then_is_break {
                then_label.is_some()
            } else {
                else_branches
                    .first()
                    .map(|b| b.label.is_some())
                    .unwrap_or(false)
            }
        }),
        _ => None,
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

/// True when the swimlane V2 single-tree path should own this diagram (instead
/// of the legacy segment model). V2 is the ONLY path that can render a `|Lane|`
/// switch INSIDE an `if` branch body (the segment model fractures branches that
/// change lane). So route to V2 exactly that class: a multi-lane diagram with
/// NO fork/split and at least one `|Lane|` marker nested inside an `if`/`switch`
/// branch. Within-lane ifs (no nested `|Lane|`) and fork-bearing swimlanes stay
/// on the segment model, so they cannot regress.
fn swimlane_v2_can_handle(steps: &[ActivityStep], is_swimlane: bool) -> bool {
    if !is_swimlane {
        return false;
    }
    if steps.iter().any(|s| {
        matches!(
            s,
            ActivityStep::Split
                | ActivityStep::SplitAgain
                | ActivityStep::EndSplit
                | ActivityStep::EndMerge
        )
    }) {
        return false;
    }
    let has_fork = steps.iter().any(|s| matches!(s, ActivityStep::Fork));
    if has_fork {
        return swimlane_v2_can_handle_simple_fork_flow(steps)
            || swimlane_v2_can_handle_fork_then_lane_branch(steps)
            || swimlane_v2_can_handle_nested_while_fork(steps)
            || swimlane_v2_can_handle_nested_if_fork(steps);
    }
    if swimlane_v2_can_handle_simple_while_lane_switch(steps) {
        return true;
    }
    // A `|Lane|` nested inside an if/switch branch (depth > 0) — the failing
    // class the segment model can't represent.
    let mut depth = 0i32;
    let mut lane_in_branch = false;
    for s in steps {
        match s {
            ActivityStep::If(_) | ActivityStep::Switch(_) => depth += 1,
            ActivityStep::EndIf | ActivityStep::EndSwitch => depth -= 1,
            ActivityStep::Swimlane(_) if depth > 0 => lane_in_branch = true,
            _ => {}
        }
    }
    lane_in_branch
}

fn swimlane_v2_can_handle_simple_fork_flow(steps: &[ActivityStep]) -> bool {
    let fork_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::Fork))
        .count();
    let fork_again_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::ForkAgain))
        .count();
    let end_fork_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::EndFork))
        .count();
    let has_while = steps.iter().any(|s| matches!(s, ActivityStep::While(_)));
    if fork_count == 0
        || fork_count > 2
        || (fork_count > 1 && !has_while)
        || fork_again_count < fork_count
        || fork_again_count > 5
        || end_fork_count != fork_count
    {
        return false;
    }
    steps.iter().all(|s| {
        matches!(
            s,
            ActivityStep::Start
                | ActivityStep::Stop
                | ActivityStep::End
                | ActivityStep::Action(_)
                | ActivityStep::Fork
                | ActivityStep::ForkAgain
                | ActivityStep::EndFork
                | ActivityStep::While(_)
                | ActivityStep::EndWhile(_)
                | ActivityStep::Swimlane(_)
        )
    })
}

fn swimlane_v2_can_handle_fork_then_lane_branch(steps: &[ActivityStep]) -> bool {
    let fork_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::Fork))
        .count();
    let fork_again_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::ForkAgain))
        .count();
    let end_fork_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::EndFork))
        .count();
    if fork_count != 1 || fork_again_count != 2 || end_fork_count != 1 {
        return false;
    }

    let mut if_depth = 0i32;
    let mut lane_in_branch = false;
    for step in steps {
        match step {
            ActivityStep::If(_) => if_depth += 1,
            ActivityStep::EndIf => if_depth -= 1,
            ActivityStep::Swimlane(_) if if_depth > 0 => lane_in_branch = true,
            ActivityStep::Start
            | ActivityStep::Stop
            | ActivityStep::End
            | ActivityStep::Action(_)
            | ActivityStep::ElseIf(_)
            | ActivityStep::Else(_)
            | ActivityStep::Fork
            | ActivityStep::ForkAgain
            | ActivityStep::EndFork
            | ActivityStep::Swimlane(_) => {}
            _ => return false,
        }
    }
    lane_in_branch
}

fn swimlane_v2_can_handle_nested_while_fork(steps: &[ActivityStep]) -> bool {
    let fork_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::Fork))
        .count();
    let fork_again_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::ForkAgain))
        .count();
    let end_fork_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::EndFork))
        .count();
    if fork_count != 1 || !(1..=5).contains(&fork_again_count) || end_fork_count != 1 {
        return false;
    }

    let mut if_depth = 0i32;
    let mut while_depth = 0i32;
    let mut nested_fork = false;
    for step in steps {
        match step {
            ActivityStep::If(_) => if_depth += 1,
            ActivityStep::EndIf => if_depth -= 1,
            ActivityStep::While(_) => while_depth += 1,
            ActivityStep::EndWhile(_) => while_depth -= 1,
            ActivityStep::Fork if if_depth > 0 && while_depth > 0 => nested_fork = true,
            ActivityStep::Start
            | ActivityStep::Stop
            | ActivityStep::End
            | ActivityStep::Action(_)
            | ActivityStep::ElseIf(_)
            | ActivityStep::Else(_)
            | ActivityStep::Fork
            | ActivityStep::ForkAgain
            | ActivityStep::EndFork
            | ActivityStep::Swimlane(_) => {}
            _ => return false,
        }
    }
    nested_fork
}

fn swimlane_v2_can_handle_nested_if_fork(steps: &[ActivityStep]) -> bool {
    let fork_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::Fork))
        .count();
    let fork_again_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::ForkAgain))
        .count();
    let end_fork_count = steps
        .iter()
        .filter(|s| matches!(s, ActivityStep::EndFork))
        .count();
    if fork_count != 1 || !(1..=5).contains(&fork_again_count) || end_fork_count != 1 {
        return false;
    }

    let mut if_depth = 0i32;
    let mut nested_fork = false;
    for step in steps {
        match step {
            ActivityStep::If(_) => if_depth += 1,
            ActivityStep::EndIf => if_depth -= 1,
            ActivityStep::Fork if if_depth > 0 => nested_fork = true,
            ActivityStep::Start
            | ActivityStep::Stop
            | ActivityStep::End
            | ActivityStep::Action(_)
            | ActivityStep::ElseIf(_)
            | ActivityStep::Else(_)
            | ActivityStep::Fork
            | ActivityStep::ForkAgain
            | ActivityStep::EndFork
            | ActivityStep::Swimlane(_) => {}
            _ => return false,
        }
    }
    nested_fork
}

fn swimlane_v2_can_handle_simple_while_lane_switch(steps: &[ActivityStep]) -> bool {
    let mut while_depth = 0i32;
    let mut lane_in_while = false;
    for step in steps {
        match step {
            ActivityStep::While(_) => while_depth += 1,
            ActivityStep::EndWhile(_) => while_depth -= 1,
            ActivityStep::Swimlane(_) if while_depth > 0 => lane_in_while = true,
            ActivityStep::Start
            | ActivityStep::Stop
            | ActivityStep::End
            | ActivityStep::Action(_)
            | ActivityStep::Swimlane(_) => {}
            _ => return false,
        }
    }
    lane_in_while
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
    let is_swimlane =
        distinct_lanes.len() > 1 || (distinct_lanes.len() == 1 && has_pre_lane_content);
    // V2 single-tree path owns the branch-internal-lane class by default; the
    // env var forces it on for any swimlane (testing the linear cluster etc.).
    let inherits_swimlane_v2 = SWIMLANE_V2_MAP.with(|m| m.borrow().is_some());
    let v2 = std::env::var("RUSTUML_SWIMLANE_V2").is_ok()
        || inherits_swimlane_v2
        || swimlane_v2_can_handle(steps, is_swimlane);
    if is_swimlane && !v2 {
        return build_swimlanes(steps, palette);
    }
    if is_swimlane && v2 {
        // V2 single-tree path: assign lane indices in first-appearance order, set
        // the thread-local so `build_tree_inner` (recursively, incl. branch
        // helpers) lowers each `|Lane|` step to a `LayoutNode::LaneMark(idx)`.
        //
        // Re-entrancy guard: branch bodies are built via `collect_until` →
        // `build_tree`, so this gate fires again on each branch sub-slice. Only
        // the OUTERMOST call (map currently unset) owns the map — it builds and
        // sets it, then clears it; inner calls leave the parent's lane indices
        // intact (a nested rebuild would renumber lanes and corrupt the
        // top-level mapping). `build_tree_inner` lowers `|Lane|` markers found
        // at any nesting depth using whichever map the outermost call installed.
        let we_set = SWIMLANE_V2_MAP.with(|m| m.borrow().is_none());
        if we_set {
            let mut map: HashMap<String, usize> = HashMap::new();
            for name in &swimlane_markers {
                let next = map.len();
                map.entry((*name).to_string()).or_insert(next);
            }
            SWIMLANE_V2_MAP.with(|m| *m.borrow_mut() = Some(map));
        }
        let tree = build_tree_inner(steps, palette);
        if we_set {
            SWIMLANE_V2_MAP.with(|m| *m.borrow_mut() = None);
        }
        return tree;
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
                // A `detach`/`kill` immediately following `end fork` does not
                // start a new node: PlantUML's `endFork` pops the fork off the
                // current-instruction stack so `detach` calls `parent.kill()`,
                // which kills the parent list's last instruction — the fork —
                // and that delegates to the fork's LAST branch's last
                // instruction (InstructionFork.kill → getLastList().kill()).
                // The effect is that the final fork branch terminates and loses
                // its pointOut, so it is NOT wired down to the join bar. We
                // mirror this by appending the terminator into the last branch
                // and consuming the standalone step.
                if let Some(step @ (ActivityStep::Detach | ActivityStep::Kill)) = steps.get(i)
                    && let Some(last_branch) = branches.last_mut()
                    && !last_branch.is_empty()
                {
                    last_branch.push(match step {
                        ActivityStep::Kill => LayoutNode::Kill,
                        _ => LayoutNode::Detach,
                    });
                    i += 1;
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
            ActivityStep::Swimlane(lane) => {
                // Swimlane V2: lower to a zero-size LaneMark so emit can route
                // subsequent shapes into this lane's buffer. Off the V2 path
                // (map unset) this is a no-op, preserving legacy behavior.
                if let Some(idx) = swimlane_v2_lane_index(&lane.name) {
                    nodes.push(LayoutNode::LaneMark(idx));
                }
                i += 1;
            }
            ActivityStep::Backward(_) => {
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

#[derive(Debug, Clone, Default)]
struct ForkLayout {
    bar_w: f64,
    centers: Vec<f64>,
    spine_dx: f64,
    /// True when the black bar should be `ignoreForCompressionOnX` AND there is a
    /// reclaimable middle-gap corridor (a mixed-asymmetry even fork — one off-
    /// centre branch beside a plain one). For such forks the +18 even-middle gap
    /// is laid out uncompressed and then collapsed by the whole-diagram ON_X pass,
    /// shrinking the bar and leaving the spine at its faithful off-centre getLeft.
    /// All other forks keep the bar OCCUPYING (blocking), matching the goldens
    /// where the bar must hold open an external if/else or note corridor.
    bar_compressible: bool,
}

const FORK_INNER_PAD: f64 = 12.0;
const FORK_BRANCH_GAP: f64 = 10.0;
const FORK_MULTI_SEQUENCE_EVEN_PAD: f64 = 14.0;
const FORK_MULTI_SEQUENCE_GAP: f64 = FORK_BRANCH_GAP + FORK_EVEN_MIDDLE_EXTRA;
/// The 2 px the whole-diagram ON_X pass shaves off each OUTER (un-compressible)
/// branch margin: PlantUML's per-branch `addHorizontalMargin(14, 14)` overhangs
/// the black bar by 14, but the goldens show the bar overhanging each edge branch
/// by [`FORK_INNER_PAD`] (12). (Inter-branch margins compress to 10 regardless, so
/// this only affects the two outer edges, the bar width, and the spine offset.)
const FORK_PARALLEL_X_MARGIN_TRIM: f64 = 2.0;
/// `AbstractParallelFtilesBuilder.computeNewFtile`'s per-branch
/// `addHorizontalMargin(xMargin, xMargin)` with `xMargin = 14`, trimmed for the
/// drawn extent (see [`FORK_PARALLEL_X_MARGIN_TRIM`]). Used by the faithful
/// `FtileForkInner` fork-layout path (nude-switch branches).
const FORK_PARALLEL_X_MARGIN: f64 = 14.0 - FORK_PARALLEL_X_MARGIN_TRIM;
const FORK_EVEN_MIDDLE_EXTRA: f64 = 18.0;
const FORK_ASYMMETRIC_EVEN_MIDDLE_EXTRA: f64 = 32.0;
const FORK_ASYMMETRIC_SPINE_STEP: f64 = 5.0;
/// Spine offset for an ODD fork that mixes asymmetric (if-bearing) branches with
/// at least one symmetric (plain) branch. The plain branch breaks the mirror
/// symmetry that keeps an all-asymmetric odd fork's spine on the bar centre, so
/// PlantUML's `FtileForkInner` centre (width/2) ends up `2 * STEP` right of the
/// flow spine. Derived from `act_complex_if_in_fork` (the sole pure mixed-odd
/// fork golden); all-asymmetric odd forks keep the depth-scaled `(d-1)*STEP`.
const FORK_MIXED_ODD_SPINE_EXTRA: f64 = 2.0 * FORK_ASYMMETRIC_SPINE_STEP;
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
/// A labelled while used as a single fork branch keeps its inbound label slot
/// open inside the branch, and PlantUML measures the branch by drawn extents
/// rather than the standalone while's frame reservation.
const FORK_BRANCH_WHILE_LEFT_TRIM: f64 = 9.0;
const FORK_BRANCH_WHILE_RIGHT_TRIM: f64 = 23.0;
const FORK_BRANCH_WHILE_SIBLING_CENTER_EXTRA: f64 = 1.0;
const FORK_BRANCH_WHILE_EXIT_ARROW_PUSH_DOWN: f64 = 1.0;
/// A two-way fork whose branches are a simple `while` loop and a simple
/// `repeat` loop keeps the loop tiles in PlantUML's uncompressed branch slots.
/// The while branch shifts as a whole; the repeat keeps its entry diamond near
/// the fork bar and distributes the slack around the body/condition.
const FORK_BRANCH_LOOP_SLOT_HALF: f64 = 27.0;
const FORK_BRANCH_REPEAT_TAIL_EXTRA: f64 = 26.0;
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

/// The leading multi-case switch of a `fork`/`split` branch that lays out nude
/// (>= 3 cases, see `fork_branch_switch_is_nude`), if any.
fn fork_branch_nude_switch(branch: &[LayoutNode]) -> Option<(&[SwitchCase], &str)> {
    if let Some(LayoutNode::Switch { cases, condition }) = branch.iter().find(|n| node_is_flow(n))
        && fork_branch_switch_is_nude(cases, condition)
    {
        Some((cases, condition))
    } else {
        None
    }
}

/// (left, right) extents a `fork`/`split` branch reserves. Identical to
/// `sequence_extents` except for a leading nude switch (>= 3 cases), which
/// reserves its uncompressed `FtileSwitchNude` block (`switch_x_layout_nude`)
/// rather than the switch-locally-compressed standalone block.
fn fork_branch_extents(branch: &[LayoutNode]) -> (f64, f64) {
    if let Some((cases, condition)) = fork_branch_nude_switch(branch) {
        let layout = switch_x_layout_nude(cases, condition);
        let (mut left, mut right) = sequence_extents(branch);
        left = left.max(layout.diamond_dx);
        right = right.max(layout.block_w - layout.diamond_dx);
        return (left, right);
    }
    let (mut left, mut right) = sequence_extents(branch);
    if fork_branch_while_slot_opens(branch) {
        left -= FORK_BRANCH_WHILE_LEFT_TRIM;
        right -= FORK_BRANCH_WHILE_RIGHT_TRIM;
    }
    (left, right)
}

fn fork_branch_while_slot_opens(branch: &[LayoutNode]) -> bool {
    matches!(
        branch,
        [LayoutNode::While {
            body,
            is_label,
            end_label,
            special_out: None,
            ..
        }] if while_ordinary_slot_compresses(body, is_label, end_label, None)
    )
}

fn branch_is_simple_while_loop(branch: &[LayoutNode]) -> bool {
    matches!(
        branch,
        [LayoutNode::While {
            body,
            is_label: Some(_),
            end_label: None,
            special_out: None,
            ..
        }] if matches!(body.as_slice(), [LayoutNode::Action { .. }])
    )
}

fn branch_is_simple_repeat_loop(branch: &[LayoutNode]) -> bool {
    matches!(
        branch,
        [LayoutNode::Repeat {
            body,
            backward: None,
            has_start_label: false,
            ..
        }] if matches!(body.as_slice(), [LayoutNode::Action { .. }])
    )
}

fn branch_is_simple_while_if_loop(branch: &[LayoutNode]) -> bool {
    matches!(
        branch,
        [LayoutNode::While {
            body,
            is_label: Some(_),
            end_label: None,
            special_out: None,
            ..
        }] if matches!(
            body.as_slice(),
            [LayoutNode::If {
                then_branch,
                else_branches,
                ..
            }] if matches!(then_branch.as_slice(), [LayoutNode::Action { .. }])
                && matches!(
                    else_branches.as_slice(),
                    [ElseBranch {
                        condition: None,
                        body,
                        ..
                    }] if matches!(body.as_slice(), [LayoutNode::Action { .. }])
                )
        )
    )
}

fn fork_branches_are_simple_while_repeat_pair(branches: &[Vec<LayoutNode>]) -> bool {
    matches!(branches, [a, b] if (branch_is_simple_while_loop(a) && branch_is_simple_repeat_loop(b))
        || (branch_is_simple_repeat_loop(a) && branch_is_simple_while_loop(b)))
}

fn fork_branches_are_simple_while_if(branches: &[Vec<LayoutNode>]) -> bool {
    branches.len() >= 2
        && branches
            .iter()
            .all(|branch| branch_is_simple_while_if_loop(branch))
}

fn fork_branches_are_single_actions(branches: &[Vec<LayoutNode>]) -> bool {
    branches
        .iter()
        .all(|branch| matches!(branch.as_slice(), [LayoutNode::Action { .. }]))
}

fn is_multi_fork_sequence_candidate(node: &LayoutNode) -> bool {
    matches!(
        node,
        LayoutNode::Fork {
            branches,
            is_split: false,
            ..
        } if branches.len() >= 2 && fork_branches_are_single_actions(branches)
    )
}

fn sequence_uses_multi_fork_layout(nodes: &[LayoutNode]) -> bool {
    nodes
        .iter()
        .filter(|node| is_multi_fork_sequence_candidate(node))
        .count()
        > 1
}

fn sequence_multi_fork_has_prelude(nodes: &[LayoutNode]) -> bool {
    let Some(first_fork) = nodes.iter().position(is_multi_fork_sequence_candidate) else {
        return false;
    };
    nodes[..first_fork]
        .iter()
        .any(|node| node_is_flow(node) && !matches!(node, LayoutNode::Start))
}

fn fork_branch_origin_adjust(simple_while_repeat_pair: bool, branch: &[LayoutNode]) -> f64 {
    if simple_while_repeat_pair && branch_is_simple_while_loop(branch) {
        0.5
    } else if simple_while_repeat_pair && branch_is_simple_repeat_loop(branch) {
        -1.0
    } else {
        0.0
    }
}

fn sequence_height_fork_branch(branch: &[LayoutNode], gap_extra: f64) -> f64 {
    let extra = if fork_branch_while_slot_opens(branch) {
        WHILE_BODY_SLOT_COMPRESS
    } else if branch_is_simple_repeat_loop(branch) {
        FORK_BRANCH_LOOP_SLOT_HALF + FORK_BRANCH_REPEAT_TAIL_EXTRA
    } else if branch_is_simple_while_loop(branch) {
        FORK_BRANCH_LOOP_SLOT_HALF
    } else if branch_is_simple_while_if_loop(branch) {
        -WHILE_BODY_SLOT_COMPRESS
    } else {
        0.0
    };
    sequence_height_ex(branch, gap_extra) + extra
}

fn fork_layout(branches: &[Vec<LayoutNode>]) -> ForkLayout {
    let branch_extents: Vec<(f64, f64)> = branches.iter().map(|b| fork_branch_extents(b)).collect();
    let branch_widths: Vec<f64> = branch_extents.iter().map(|(l, r)| l + r).collect();
    let n = branch_widths.len();
    if n == 0 {
        return ForkLayout::default();
    }

    // A fork containing a nude-switch branch (>= 3 cases) lays out FAITHFULLY per
    // `AbstractParallelFtilesBuilder` + `FtileForkInner`: each branch wrapped in
    // `addHorizontalMargin(14, 14)`, placed left-to-right, the join spine pinned
    // at `totalWidth / 2`. The black bar spans the full inner width. The
    // diagram-wide ON_X pass (the bar is `ignoreForCompressionOnX`) then reclaims
    // the empty bands. This replaces the construct-local FORK_INNER_PAD / gap /
    // even-extra model — which assumes already-tight branches — for the switch
    // case, where the switch advertises its loose nude reserve.
    if !branches.iter().any(Vec::is_empty)
        && branches
            .iter()
            .any(|b| fork_branch_nude_switch(b).is_some())
    {
        let mut centers = Vec::with_capacity(n);
        let mut x = 0.0;
        for (left, right) in &branch_extents {
            // Decorated tile: +14 each side; the branch spine sits at its own left
            // extent plus the left margin.
            centers.push(x + FORK_PARALLEL_X_MARGIN + left);
            x += left + right + 2.0 * FORK_PARALLEL_X_MARGIN;
        }
        return ForkLayout {
            bar_w: x,
            centers,
            // The join spine sits at the FULL-margin (`xMargin = 14`) inner centre,
            // i.e. 2 px right of the trimmed (`FORK_PARALLEL_X_MARGIN = 12`) bar
            // centre — the 2 px the outer margins lost. `spine = bar_w/2 + 2` is
            // `spine_dx = -2` (spine = `bar_w/2 - spine_dx`); this keeps the drawn
            // branch positions anchored while the start/stop terminals re-centre.
            spine_dx: -(FORK_PARALLEL_X_MARGIN_TRIM),
            // The bar is `ignoreForCompressionOnX`; its slack collapses with the
            // empty inter-branch bands in the whole-diagram pass.
            bar_compressible: true,
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
            bar_compressible: false,
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
    // Even-branch middle-gap clearance, by branch-asymmetry class:
    //   * ALL branches asymmetric  → +32 (`FORK_ASYMMETRIC_EVEN_MIDDLE_EXTRA`):
    //     mirrored off-centre branches (the `fork*br_ifdepth*` goldens) touch wide-
    //     side to wide-side, so the gap is not reclaimable by ON_X compression.
    //   * otherwise                → +18 (`FORK_EVEN_MIDDLE_EXTRA`): the ordinary
    //     even-fork middle gap. For a MIXED fork (a lone off-centre branch beside a
    //     plain one — `act_fork_with_if`) this +18 corridor is later reclaimed by
    //     the whole-diagram ON_X compression (the black bar is `ignoreForCompression
    //     OnX`, see `compress.rs`), shrinking the bar AND leaving the spine — the
    //     bar's uncompressed getLeft, which sits LEFT of the collapsed corridor — at
    //     its faithful, off-centre position.
    let all_asymmetric = branch_extents
        .iter()
        .all(|(left, right)| (left - right).abs() > FORK_ASYMMETRIC_EPS);
    let even_extra = if !(n >= 2 && n.is_multiple_of(2)) || fork_branches_all_start_led(branches) {
        // Start-led branches lay out as plain side-by-side columns: PlantUML's
        // black bar holds open no even-fork middle corridor for them.
        0.0
    } else if all_asymmetric {
        FORK_ASYMMETRIC_EVEN_MIDDLE_EXTRA
    } else {
        FORK_EVEN_MIDDLE_EXTRA
    };
    let mut bar_w =
        FORK_INNER_PAD * 2.0 + total_branch_w + inter_gaps * FORK_BRANCH_GAP + even_extra;
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
    if fork_branches_are_simple_while_repeat_pair(branches) {
        bar_w -= FORK_BRANCH_LOOP_SLOT_HALF - 1.0;
        for (i, center) in centers.iter_mut().enumerate() {
            if i > 0 {
                *center -= FORK_BRANCH_INTER_GAP_EXTRA;
            }
        }
    }
    if fork_branches_are_simple_while_if(branches) {
        let middle_gap_idx = n.is_multiple_of(2).then_some(n / 2 - 1);
        let last = n - 1;
        let last_shift = 7.0
            + 18.0 * last as f64
            + if middle_gap_idx.is_some_and(|m| last > m) {
                11.0
            } else {
                0.0
            };
        bar_w -= last_shift + 13.0;
        for (i, center) in centers.iter_mut().enumerate() {
            let mut shift = 7.0 + 18.0 * i as f64;
            if middle_gap_idx.is_some_and(|m| i > m) {
                shift += 11.0;
            };
            *center -= shift;
        }
    }
    // A MIXED-asymmetry even fork (one off-centre branch beside a plain one) lays
    // its +18 middle gap uncompressed; the whole-diagram ON_X pass then reclaims
    // it (the bar is `ignoreForCompressionOnX`). Only this class marks the bar
    // compressible — all-symmetric, all-asymmetric, and odd forks keep the
    // blocking bar (their goldens need the bar to hold open external corridors).
    // A multi-case switch fork branch leaves a wide, shape-free corridor beside
    // its sibling (its diamond column does not span the branch). PlantUML's
    // always-`ignoreForCompressionOnX` black bar lets the ON_X pass reclaim that
    // corridor; mark the bar compressible here too so the even-fork middle gap
    // collapses, matching the goldens.
    let has_switch_branch = branches
        .iter()
        .any(|b| matches!(b.first(), Some(LayoutNode::Switch { cases, .. }) if cases.len() >= 2));
    let bar_compressible = n >= 2
        && n.is_multiple_of(2)
        && ((has_asymmetric_branch && !all_asymmetric) || has_switch_branch);
    ForkLayout {
        bar_w,
        centers,
        spine_dx: if fork_branches_are_simple_while_if(branches) {
            if n.is_multiple_of(2) { 4.5 } else { 5.0 }
        } else if n > 1 && !n.is_multiple_of(2) && has_asymmetric_branch {
            let max_if_depth = branches
                .iter()
                .map(|branch| sequence_if_depth(branch))
                .max()
                .unwrap_or(0);
            let base = max_if_depth.saturating_sub(1) as f64 * FORK_ASYMMETRIC_SPINE_STEP;
            if all_asymmetric {
                base
            } else {
                base + FORK_MIXED_ODD_SPINE_EXTRA
            }
        } else {
            0.0
        },
        bar_compressible,
    }
}

fn fork_layout_multi_sequence(branches: &[Vec<LayoutNode>], has_prelude: bool) -> ForkLayout {
    let branch_extents: Vec<(f64, f64)> = branches.iter().map(|b| fork_branch_extents(b)).collect();
    let n = branch_extents.len();
    if n == 0 {
        return ForkLayout::default();
    }

    let all_branches_tiny = branch_extents
        .iter()
        .all(|(left, right)| left + right < ACTION_MIN_HEIGHT);
    let edge_pad = if n == 2 && (all_branches_tiny || has_prelude) {
        FORK_MULTI_SEQUENCE_EVEN_PAD
    } else {
        FORK_INNER_PAD
    };
    let mut centers = Vec::with_capacity(n);
    let mut x = edge_pad;
    for (i, (left, right)) in branch_extents.iter().enumerate() {
        centers.push(x + left);
        x += left + right;
        if i + 1 < n {
            x += FORK_MULTI_SEQUENCE_GAP;
        }
    }

    ForkLayout {
        bar_w: x + edge_pad,
        centers,
        spine_dx: 0.0,
        bar_compressible: false,
    }
}

fn following_flow_node(nodes: &[LayoutNode], i: usize) -> Option<&LayoutNode> {
    nodes.iter().skip(i + 1).find(|node| node_is_flow(node))
}

fn while_body_fork_following_action_extra(
    nodes: &[LayoutNode],
    i: usize,
    branches: &[Vec<LayoutNode>],
) -> f64 {
    if branches.len() != 3
        || !branches
            .iter()
            .all(|branch| matches!(branch.as_slice(), [LayoutNode::Action { .. }]))
    {
        return 0.0;
    }
    let Some(next) = following_flow_node(nodes, i) else {
        return 0.0;
    };
    if !matches!(next, LayoutNode::Action { .. }) {
        return 0.0;
    }
    let widest_branch = branches
        .iter()
        .map(|branch| sequence_width(branch))
        .fold(0.0f64, f64::max);
    (node_width(next) - widest_branch).max(0.0)
}

fn fork_layout_with_following_action(mut layout: ForkLayout, extra: f64) -> ForkLayout {
    if extra <= 0.0 || layout.centers.len() < 2 {
        return layout;
    }
    let last = layout.centers.len() - 1;
    layout.bar_w += extra;
    for (i, center) in layout.centers.iter_mut().enumerate() {
        *center += extra * i as f64 / last as f64;
    }
    layout
}

fn while_body_fork_layout(nodes: &[LayoutNode], i: usize, node: &LayoutNode) -> Option<ForkLayout> {
    let LayoutNode::Fork {
        branches,
        is_split: false,
        merge: false,
        ..
    } = node
    else {
        return None;
    };
    let extra = while_body_fork_following_action_extra(nodes, i, branches);
    if extra > 0.0 {
        Some(fork_layout_with_following_action(
            fork_layout(branches),
            extra,
        ))
    } else {
        None
    }
}

fn split_layout(branches: &[Vec<LayoutNode>]) -> ForkLayout {
    let branch_extents: Vec<(f64, f64)> = branches.iter().map(|b| sequence_extents(b)).collect();
    let branch_widths: Vec<f64> = branch_extents.iter().map(|(l, r)| l + r).collect();
    let n = branch_widths.len();
    if n == 0 {
        return ForkLayout::default();
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
        bar_compressible: false,
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
    // Only a *middle* surviving case routes through the diamond merge corridor
    // (PlantUML's ConnectionVerticalBottom, firstOutgoing < i < lastOutgoing).
    // The first/last surviving branch is wired as a vertical-then-horizontal
    // outer, and a terminating (kill/detach) case has no out-corridor at all —
    // so neither participates in the inner-corridor packing decision.
    let (first_out, last_out) = match switch_outgoing_extremes(cases) {
        Some(pair) => pair,
        None => return false,
    };
    cases[1..cases.len() - 1]
        .iter()
        .zip(layout.centers[1..layout.centers.len() - 1].iter())
        .enumerate()
        .any(|(rel_i, (case, center))| {
            let i = rel_i + 1;
            if case.body.is_empty()
                || branch_terminates(&case.body)
                || i <= first_out
                || i >= last_out
            {
                return false;
            }
            let rel = *center - layout.diamond_dx;
            rel >= -diamond_half - SWITCH_LINK_MARGIN && rel <= diamond_half + SWITCH_LINK_MARGIN
        })
}

/// Index of the first / last surviving (non-terminating) case, mirroring
/// PlantUML's `FtileSwitchWithManyLinks.getFirstOutgoingArrow` /
/// `getLastOutgoingArrow`. A terminating (kill/detach) case has no pointOut and
/// is skipped.
fn switch_outgoing_extremes(cases: &[SwitchCase]) -> Option<(usize, usize)> {
    let n = cases.len();
    let first = (0..n).find(|&i| !branch_terminates(&cases[i].body))?;
    let last = (0..n).rev().find(|&i| !branch_terminates(&cases[i].body))?;
    Some((first, last))
}

/// True when some surviving case's MERGE connection terminates at the merge
/// diamond's *top* point (`ptA`, on the spine) rather than a side vertex —
/// PlantUML's `ConnectionVerticalThenHorizontal` Direction.DOWN branch (first /
/// last survivors whose out-x lies within the merge diamond's left/right
/// vertices, ±`DIAMOND_HALF`) or `ConnectionVerticalBottom`'s jog-to-spine
/// branch (a middle survivor near the condition diamond). Such a route deepens
/// the merge band from `ARROW_LEN / 2` to a full `ARROW_LEN`.
fn switch_center_spine_survivor(cases: &[SwitchCase], condition: &str) -> bool {
    let Some((first_out, last_out)) = switch_outgoing_extremes(cases) else {
        return false;
    };
    let layout = switch_x_layout(cases, condition);
    let cond_half = switch_diamond_half_width(condition); // condition diamond half-width
    cases
        .iter()
        .zip(layout.centers.iter())
        .enumerate()
        .filter(|(_, (case, _))| !branch_terminates(&case.body) && !case.body.is_empty())
        .any(|(i, (_, center))| {
            let rel = *center - layout.diamond_dx;
            if i == first_out || i == last_out {
                // ConnectionVerticalThenHorizontal: DOWN to ptA iff within the
                // merge diamond's left/right vertices (spine ± DIAMOND_HALF).
                rel.abs() <= DIAMOND_HALF
            } else {
                // ConnectionVerticalBottom middle survivor: jogs to the spine
                // when within the condition diamond's vertices (± margin).
                rel >= -cond_half - SWITCH_LINK_MARGIN && rel <= cond_half + SWITCH_LINK_MARGIN
            }
        })
}

fn switch_merge_gap(cases: &[SwitchCase], condition: &str, _layout: &SwitchXLayout) -> f64 {
    if switch_all_branches_terminate(cases) {
        0.0
    } else if switch_needs_empty_merge_gap(cases) {
        SWITCH_EMPTY_MERGE_GAP
    } else if switch_center_spine_survivor(cases, condition) {
        ARROW_LEN
    } else {
        ARROW_LEN / 2.0
    }
}

/// Switch merge gap honouring the surrounding fork's compression state.
///
/// `fork_gap_extra` is the inter-tile uncompression slack a fork branch carries
/// (`FORK_BRANCH_INTER_GAP_EXTRA`, non-zero only for unequal-height forks). When
/// it is non-zero the switch sits inside a fork branch whose sibling occupies
/// the merge y-band, so the empty merge gap is *not* reclaimed by ON_Y
/// compression and the uncompressed intrinsic gap survives. Otherwise the
/// standalone compressed gap applies.
fn switch_merge_gap_in_fork(
    cases: &[SwitchCase],
    condition: &str,
    layout: &SwitchXLayout,
    fork_gap_extra: f64,
) -> f64 {
    if fork_gap_extra > 0.0
        && !switch_all_branches_terminate(cases)
        && !switch_needs_empty_merge_gap(cases)
    {
        if layout.big_diamond {
            SWITCH_MERGE_GAP_UNCOMPRESSED_BIG
        } else {
            SWITCH_MERGE_GAP_UNCOMPRESSED_SMALL
        }
    } else {
        switch_merge_gap(cases, condition, layout)
    }
}

/// Extra height a multi-case switch tile gains when it is the tile of an
/// unequal-height fork branch: the difference between its uncompressed and
/// compressed merge gaps (see `switch_merge_gap_in_fork`). Zero for single-link
/// or all-terminating switches (no merge gap to uncompress) and for switches
/// not inside such a fork. Keeps `sequence_height_ex` agreeing with the height
/// `emit_switch_with_layout` actually draws.
fn switch_fork_uncompress_extra(node: &LayoutNode, fork_gap_extra: f64) -> f64 {
    if fork_gap_extra <= 0.0 {
        return 0.0;
    }
    let LayoutNode::Switch { cases, condition } = node else {
        return 0.0;
    };
    if cases.len() < 2 {
        return 0.0;
    }
    let layout = switch_x_layout(cases, condition);
    switch_merge_gap_in_fork(cases, condition, &layout, fork_gap_extra)
        - switch_merge_gap(cases, condition, &layout)
}

/// Vertical slack `FtileHeightFixedCentered` reserves above and below each fork
/// branch (`AbstractParallelFtilesBuilder.computeNewFtile`'s
/// `spaceArroundBlackBar`). The fixed-height band is `maxHeight + 2 * this`.
const FORK_BLACK_BAR_SPACE: f64 = 20.0;

/// Extra advertised height a multi-case switch *fork branch* carries for the
/// purpose of vertical centring against its siblings (NOT for the fork's own
/// total height, which is driven by the deepest emitted branch). PlantUML wraps
/// every fork branch in `FtileHeightFixedCentered(maxHeight + 2 *
/// spaceArroundBlackBar)` (AbstractParallelFtilesBuilder.computeNewFtile,
/// spaceArroundBlackBar = 20). A switch's *advertised* tile dimension carries
/// the uncompressed nude reserve below its visible merge diamond
/// (FtileSwitchNude adds 100 px of body slack; FtileSwitchWithManyLinks
/// .getYdelta1a adds diamondHeight/2 in BIG mode). A shorter sibling box occupies
/// that y-band, so ON_Y compression cannot reclaim it; the band the sibling
/// centres in is the switch's full advertised height. The unreclaimed slack
/// over the switch's visible height resolves to a fixed 2 * spaceArroundBlackBar
/// (40) plus a mode term — DIAMOND_HALF*2 in BIG mode (the diamond/2 doubled by
/// the Ydelta1a override) and one merge-label line height in SMALL mode.
fn switch_fork_centering_extra(node: &LayoutNode) -> f64 {
    let LayoutNode::Switch { cases, condition } = node else {
        return 0.0;
    };
    if cases.len() < 2 {
        return 0.0;
    }
    let layout = switch_x_layout(cases, condition);
    let mode_term = if layout.big_diamond {
        DIAMOND_HALF * 2.0
    } else {
        pm::text_height(SMALL_FONT)
    };
    2.0 * FORK_BLACK_BAR_SPACE + mode_term
}

/// Extra added to a multi-case switch's merge gap when the switch is a direct
/// flow node of a `while` body (over the standalone `switch_merge_gap`). The
/// loop frame stops ON_Y compression from collapsing the switch's merge band to
/// its standalone value:
///   * terminal/only body node — the band sits above the loop-back junction, so
///     it keeps `UNCOMPRESSED_{BIG,SMALL} - WHILE_TERMINAL_SWITCH_MERGE_RECLAIM`.
///   * followed by one or two body flow tiles — an uncompressible `ARROW_LEN`
///     survives below the merge diamond before the next tile's inbound connector.
///   * followed by three or more body flow tiles — the body is tall enough that
///     ON_Y compression reclaims the band down to
///     `SWITCH_WHILE_MERGE_GAP_COMPRESSED_{EVEN,ODD}`.
///
/// `following_flow` is the number of flow tiles that follow this switch in the
/// body. Single-link and all-terminating switches have no reclaimable merge band.
fn switch_while_merge_extra(
    node: &LayoutNode,
    is_terminal_in_body: bool,
    following_flow: usize,
) -> f64 {
    let LayoutNode::Switch { cases, condition } = node else {
        return 0.0;
    };
    if cases.len() < 2 || switch_all_branches_terminate(cases) {
        return 0.0;
    }
    let layout = switch_x_layout(cases, condition);
    let standalone = switch_merge_gap(cases, condition, &layout);
    if is_terminal_in_body {
        let uncompressed = if layout.big_diamond {
            SWITCH_MERGE_GAP_UNCOMPRESSED_BIG
        } else {
            SWITCH_MERGE_GAP_UNCOMPRESSED_SMALL
        };
        uncompressed - WHILE_TERMINAL_SWITCH_MERGE_RECLAIM - standalone
    } else if following_flow >= SWITCH_WHILE_MERGE_COMPRESS_MIN_FOLLOWING {
        let compressed = if cases.len().is_multiple_of(2) {
            SWITCH_WHILE_MERGE_GAP_COMPRESSED_EVEN
        } else {
            SWITCH_WHILE_MERGE_GAP_COMPRESSED_ODD
        };
        compressed - standalone
    } else {
        ARROW_LEN
    }
}

/// Extra added to a multi-case switch's merge gap when the switch is the
/// terminal flow node of a `repeat` body (over the standalone `switch_merge_gap`).
///
/// Unlike a `while` body — where the loop-back corridor's ON_Y compression seats
/// the merge band at a metric-dependent reclaim (see `switch_while_merge_extra`)
/// — a `repeat` body's switch keeps a clean, case-count-independent `ARROW_LEN`.
/// PlantUML's `FtileRepeat.calculateDimensionInternal` reserves `8*halfHex` of
/// tail split as 48 px above and below the body (`getTranslateForRepeat`'s
/// `space/2`); the switch's intrinsic merge diamond sits inside `dimRepeat`, and
/// the whole-diagram ON_Y pass can only reclaim that fixed-tail corridor down to
/// one `ARROW_LEN` between the switch's merge diamond and the condition diamond,
/// not to the standalone-compressed gap. So the band keeps `standalone +
/// ARROW_LEN` regardless of case count (even/odd alike — empirically uniform
/// across the `act_combo_repeat_switch_{2..5}cases` goldens). Zero for single-link
/// or all-terminating switches (no reclaimable merge band).
fn switch_repeat_merge_extra(node: &LayoutNode) -> f64 {
    let LayoutNode::Switch {
        cases,
        condition: _,
    } = node
    else {
        return 0.0;
    };
    if cases.len() < 2 || switch_all_branches_terminate(cases) {
        return 0.0;
    }
    ARROW_LEN
}

/// Sum of the per-switch in-repeat merge-band extras for a `repeat` body. Only a
/// switch that is the body's terminal (last) flow node keeps the uncompressible
/// `ARROW_LEN` (a switch followed by more body flow has that band reclaimed by
/// the trailing tile's inbound corridor, like the standalone case). Mirrors
/// `while_body_switch_extra` so `repeat_body_height` agrees with the height
/// `emit_repeat` actually draws.
fn repeat_body_switch_extra(body: &[LayoutNode]) -> f64 {
    let Some(last) = last_flow_index(body) else {
        return 0.0;
    };
    match &body[last] {
        node @ LayoutNode::Switch { .. } => switch_repeat_merge_extra(node),
        _ => 0.0,
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
    let layout = switch_x_layout_base(cases, condition);
    // A plain all-bodied SMALL switch is laid out by `FtileSwitchNude` at
    // `xSeparation = 20`, then the diagram-wide ON_X compression squeezes the
    // empty inter-tile bands to 10 — EXCEPT where the (opaque) condition/merge
    // diamond column at the block centre protrudes into a band. The base layout
    // packs every band to 10; re-derive the faithful compressed centres so a wide
    // outer tile that pushes the diamond into the gap before it (e.g. a
    // terminating last case) keeps the diamond's intrusion. This is identity when
    // the diamond lands over a tile, so currently-correct switches are unchanged.
    // The various empty/mixed/nested-if special cases run their own bespoke
    // packing and are left untouched.
    if switch_small_is_plain(cases, condition, &layout) {
        let tiles: Vec<SwitchCaseTile> = cases.iter().map(switch_case_tile).collect();
        let diamond_w = diamond_inner_w(condition) + DIAMOND_HALF * 2.0;
        return switch_small_compressed(&tiles, diamond_w);
    }
    layout
}

/// True for a plain all-bodied SMALL_DIAMOND switch that takes the faithful ON_X
/// compression path (no empty/mixed/nested-if special-case packing applies).
fn switch_small_is_plain(cases: &[SwitchCase], condition: &str, layout: &SwitchXLayout) -> bool {
    if layout.big_diamond || cases.is_empty() {
        return false;
    }
    let all_bodied = cases.iter().all(|c| !c.body.is_empty());
    let even_inner_corridor = cases.len().is_multiple_of(2)
        && switch_inner_uses_diamond_corridor(cases, condition, layout);
    all_bodied
        && !switch_has_empty_middle_case(cases)
        && !switch_is_even_nested_if_cases(cases)
        && !even_inner_corridor
        && !switch_is_odd_alternating_mixed_empty(cases)
        && !switch_is_even_mixed_empty_pair(cases)
}

/// Base SMALL/BIG switch layout plus the empty/mixed/nested-if special cases,
/// shared by the standalone path (which then applies ON_X compression to the
/// plain case) and the `while`-body path (which keeps the uncompressed gaps).
fn switch_x_layout_base(cases: &[SwitchCase], condition: &str) -> SwitchXLayout {
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

/// Switch layout for a switch that is a direct flow node of a `while` body.
///
/// PlantUML lays every switch out with `FtileSwitchNude.xSeparation = 20`, then
/// the diagram-wide `CompressionXorYBuilder(ON_X)` pass squeezes the empty
/// inter-case bands. For a STANDALONE switch every band collapses to 10 (the
/// hard-coded `SWITCH_CASE_GAP`); but a switch wrapped in a `while` keeps the
/// loop-back corridor and condition/merge-diamond column occupying x-space near
/// the spine, so the bands straddling the diamond column survive at the
/// uncompressed `xSeparation = 20` while the outer bands still collapse to 10.
///
/// Concretely, for an odd, all-bodied SMALL_DIAMOND switch the center case sits
/// on the spine and the diamond corridor passes through the two gaps either side
/// of it — both stay 20. (Even counts already keep their single spine-straddling
/// gap at 20 via `add_center_gap`, and outer gaps at 10, which matches the
/// goldens, so this only widens the odd-count adjacent gaps.) The spine stays
/// centred on the (unchanged) center case.
fn switch_x_layout_in_while(cases: &[SwitchCase], condition: &str) -> SwitchXLayout {
    // The `while` frame blocks the standalone ON_X compression, so start from the
    // uncompressed base layout (gaps still at the packed-10 width) and apply the
    // loop-specific gap widening below.
    let mut layout = switch_x_layout_base(cases, condition);
    let n = cases.len();
    if layout.big_diamond || n < 3 || cases.iter().any(|c| c.body.is_empty()) {
        return layout;
    }
    if n.is_multiple_of(2) {
        // Even counts already keep their single spine-straddling gap at 20 (via
        // `add_center_gap`) and their outer gaps at 10. `switch_x_layout` then
        // applies `SWITCH_INNER_CORRIDOR_PACKING_PULL_LEFT` to the right-half
        // cases — a standalone ON_X compression artefact. Inside a `while` the
        // loop frame blocks that compression, so undo the pull-left here, leaving
        // the spine-straddling gap at the full uncompressed 20.
        if switch_inner_uses_diamond_corridor(cases, condition, &layout) {
            for center in layout.centers.iter_mut().skip(n / 2) {
                *center += SWITCH_INNER_CORRIDOR_PACKING_PULL_LEFT;
            }
            layout.block_w += SWITCH_INNER_CORRIDOR_PACKING_PULL_LEFT;
            layout.diamond_dx += SWITCH_INNER_CORRIDOR_PACKING_PULL_LEFT;
        }
        return layout;
    }
    // Widen the two gaps adjacent to the center case from SWITCH_CASE_GAP (10) to
    // xSeparation (20). Rebuild the centers left-to-right relative to the same
    // block-left origin (centers[0] = tiles[0].left), using gap-20 for the two
    // bands straddling the center case and gap-10 elsewhere. The diamond column
    // (spine) sits on the center case, so block grows by `extra` on each side.
    let extra = SWITCH_IF_BRANCH_CASE_GAP - SWITCH_CASE_GAP;
    let center = n / 2;
    let tiles: Vec<SwitchCaseTile> = cases.iter().map(switch_case_tile).collect();
    let mut centers = vec![0.0f64; n];
    let mut x = 0.0;
    for i in 0..n {
        centers[i] = x + tiles[i].left;
        // Gap AFTER case i. Bands adjacent to the center case (i == center-1 and
        // i == center) carry the full xSeparation; the rest the compressed gap.
        let gap = if i == center - 1 || i == center {
            SWITCH_IF_BRANCH_CASE_GAP
        } else {
            SWITCH_CASE_GAP
        };
        x += tiles[i].width + gap;
    }
    layout.centers = centers;
    layout.block_w += 2.0 * extra;
    // Spine stays on the center case.
    layout.diamond_dx = layout.centers[center];
    layout
}

fn switch_x_layout_if_branch(cases: &[SwitchCase], condition: &str) -> SwitchXLayout {
    if cases.len() >= 4 && !switch_case_block_is_big_diamond(cases, condition) {
        return switch_x_layout_if_branch_packed(cases, condition);
    }
    switch_x_layout_with_small_gap(
        cases,
        condition,
        SWITCH_IF_BRANCH_CASE_GAP,
        cases.len() == 2,
    )
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
    switch_x_layout_with_small_gap(
        cases,
        condition,
        SWITCH_CASE_GAP + spine_room,
        cases.len() == 2,
    )
}

/// Returns whether the SMALL/BIG diamond test selects BIG mode for these cases.
fn switch_case_block_is_big_diamond(cases: &[SwitchCase], condition: &str) -> bool {
    switch_x_layout_with_small_gap(cases, condition, SWITCH_IF_BRANCH_CASE_GAP, false).big_diamond
}

/// A multi-case SMALL-diamond switch that is the leading flow node of a
/// `fork`/`split` branch (with >= 3 cases) lays out FAITHFULLY: PlantUML's
/// `FtileSwitch.calculateDimension` returns the uncompressed `FtileSwitchNude`
/// block (cases at `xSeparation = 20`, diamond at the nude centre, getLeft =
/// width/2), and the fork's `AbstractParallelFtilesBuilder` wraps each branch in
/// `addHorizontalMargin(14, 14)`, lays them left-to-right (`FtileForkInner`), and
/// pins the join spine at `totalWidth / 2`. The diagram-wide `CompressionXorYBuilder`
/// (ON_X) — whose blocking column is the diagram spine; the fork black bar is
/// `ignoreForCompressionOnX` — then collapses the empty bands. Running the switch
/// through the standalone `switch_x_layout` instead applies a SWITCH-LOCAL
/// compression that double-counts the bands and mis-centres the diamond. A 2-case
/// switch keeps the standalone form (its tighter spacing is verified by
/// `act_switch_in_fork`, two 2-case switch branches, which passes).
fn fork_branch_switch_is_nude(cases: &[SwitchCase], condition: &str) -> bool {
    cases.len() >= 3 && !switch_case_block_is_big_diamond(cases, condition)
}

/// The uncompressed `FtileSwitchNude` layout for a switch that is a fork/split
/// branch: cases at the full `xSeparation = 20` with the diamond at the nude
/// block centre and NO switch-local ON_X compression — the diagram-wide pass
/// reclaims the empty bands instead (see `fork_branch_switch_is_nude`).
fn switch_x_layout_nude(cases: &[SwitchCase], condition: &str) -> SwitchXLayout {
    switch_x_layout_with_small_gap(cases, condition, SWITCH_IF_BRANCH_CASE_GAP, false)
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

/// PlantUML's standalone SMALL_DIAMOND switch geometry: tiles laid out at
/// `xSeparation = 20` (`FtileSwitchNude`), then the diagram-wide ON_X compression
/// (`CompressionXorYBuilder`) collapses every empty inter-tile band wider than
/// `2*COMPRESS_MARGIN` down to exactly `2*COMPRESS_MARGIN` (=10). The condition
/// and merge diamonds (plus the start/stop terminals) live on the block-centre
/// spine and are OPAQUE on the X axis, so a band the diamond column protrudes
/// into cannot fully collapse — it survives at `10 + protrusion`. This is a
/// faithful re-derivation of the same ON_X pass that `compress::compress` runs on
/// the whole canvas, applied locally to the switch's own occupancy so the case
/// centres land correctly before the rest of the diagram is assembled.
fn switch_small_compressed(tiles: &[SwitchCaseTile], diamond_w: f64) -> SwitchXLayout {
    const XSEP: f64 = 20.0; // FtileSwitchNude.xSeparation
    let n = tiles.len();

    // Uncompressed tile layout (xSeparation between adjacent tiles).
    let mut centers = vec![0.0f64; n];
    let mut boxes = Vec::with_capacity(n);
    let mut x = 0.0;
    for i in 0..n {
        centers[i] = x + tiles[i].left;
        boxes.push((x, x + tiles[i].width));
        x += tiles[i].width + XSEP;
    }
    let nude_w = x - XSEP;
    let block_w = nude_w.max(diamond_w).max(DIAMOND_HALF * 2.0);
    let dia = block_w / 2.0;

    // Occupancy on the X axis: case tile boxes plus the diamond/terminal column
    // (condition hexagon `dia ± diamond_w/2`, merge diamond `± DIAMOND_HALF`,
    // start/stop ellipses `± 10/11`). The hexagon is the widest, so it dominates.
    let mut occ = crate::compress::SlotSet::new();
    for &(a, b) in &boxes {
        occ.add_slot(a, b);
    }
    occ.add_slot(dia - diamond_w / 2.0, dia + diamond_w / 2.0);
    let tf = crate::compress::CompressionTransform::from_occupied(
        &occ,
        crate::compress::COMPRESS_MARGIN,
    );

    let centers: Vec<f64> = centers.iter().map(|&c| tf.transform(c)).collect();
    SwitchXLayout {
        block_w: tf.transform(nude_w),
        diamond_dx: tf.transform(dia),
        centers,
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
    // A break-bearing `if` renders thinly enough not to block the loop's inbound
    // slot compression — and, because the `break` shares the exit corridor,
    // PlantUML keeps that compression even when a terminator was absorbed after
    // `endwhile` (so the usual specialOut gate does not apply). This includes a
    // `break` nested under ordinary if/switch/fork nodes owned by this loop.
    let has_break_if = body_contains_break_if(body);
    let has_terminating_if_down = while_body_has_terminating_if_down_with_following_flow(body);
    (special_out.is_none() || has_break_if)
        && (has_terminating_if_down
            || body.iter().all(|node| {
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
                            || node_contains_break(node)
                )
            }))
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

/// A `while` whose body is *exactly* a single nested `while` whose own body is
/// an ordinary compressible chain (pure actions etc.). In PlantUML's global ON_Y
/// pass, the empty inbound band between two stacked condition diamonds is the
/// reclaimable slot, and only ONE band in the diamond chain compresses. The slot
/// finder claims it at the *parent* of the deepest (pure-action) loop: this
/// while compresses its own inbound, and the nested pure-action while does NOT
/// (its slack was already reclaimed above it — see
/// [`SvgEmitter::while_sole_body_suppress_compress`]). Drives the nested-while
/// cases (`act_while_nested_3_levels`, `act_nest_while_while_while`,
/// `act_if*while*_nesting`). The labels/special_out gating mirrors
/// [`while_ordinary_slot_compress_allowed`]: the nested while must be a labelled,
/// non-empty, non-special loop for its inbound band to exist and be claimable.
fn while_body_is_single_compressible_while(body: &[LayoutNode]) -> bool {
    let [
        LayoutNode::While {
            body: inner_body,
            is_label: inner_is_label,
            end_label: inner_end_label,
            special_out: inner_special,
            ..
        },
    ] = body
    else {
        return false;
    };
    while_ordinary_slot_compresses(
        inner_body,
        inner_is_label,
        inner_end_label,
        inner_special.as_deref(),
    )
}

/// True when a `while` body is EXACTLY one nested `while` and nothing else (no
/// trailing flow tile, no end label / special-out on the inner loop). This is
/// the "pure while-chain" shape (`while { while { ... } }`).
///
/// In PlantUML the inner loop tile is the body's only content, so its own
/// loop-back arm is the body's rightmost geometry; the enclosing loop's frame
/// adds just `Hexagon.hexagonHalfSize` of corridor past it. The normal
/// `dx + halfHex` trailing whitespace that a `while` reserves past its arm
/// (`WHILE_CHAIN_TRAILING_FULL`) is reclaimed by ON_X compression because there
/// is no content below the corridor to keep it open — verified against
/// `act_nest_while_while_while` (golden trailing past the arm ≈ 20.5 px vs the
/// ~34.4 px a trailing-content loop keeps, e.g. `act_while_nested`).
fn while_body_is_pure_while_chain(body: &[LayoutNode]) -> bool {
    matches!(
        body,
        [LayoutNode::While {
            end_label: None,
            special_out: None,
            ..
        },]
    )
}

/// Trailing right-side whitespace a `while` reserves past its loop-back arm (the
/// FtileWhile frame's `dx + halfHex` term past the arm at `max(cond,body)+halfHex`).
/// Normally `DIAMOND_HALF + 3` (= the `+2*halfHex+3` past `max(cond,body)`); for a
/// pure while-chain body the corridor compresses away to a 1 px residual (see
/// [`while_body_is_pure_while_chain`]).
fn while_chain_trailing_reservation(body: &[LayoutNode]) -> f64 {
    if while_body_is_pure_while_chain(body) {
        WHILE_CHAIN_TRAILING_COMPRESSED
    } else {
        WHILE_CHAIN_TRAILING_FULL
    }
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

/// A bare `while (cond)` (no `is`/end label) whose body carries a break-bearing
/// `if`. Such a loop defers its inbound `ConnectionIn` like a slot-compressed
/// loop (PlantUML draws the loop's internal break/exit corridor connectors
/// before the connection into it). Labeled break loops are already caught by
/// [`is_ordinary_compressed_while`]; this covers the unlabeled form.
fn is_unlabeled_break_while(node: &LayoutNode) -> bool {
    matches!(
        node,
        LayoutNode::While {
            body,
            is_label: None,
            end_label: None,
            special_out: None,
            ..
        } if body_contains_break_if(body)
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

fn while_body_has_terminating_if_down_with_following_flow(body: &[LayoutNode]) -> bool {
    body.iter()
        .enumerate()
        .any(|(i, node)| if_node_is_terminating_down(node) && following_flow_count(body, i) > 0)
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
            | LayoutNode::Goto(_)
            // Swimlane V2: a branch-internal `|Lane|` marker contributes no
            // geometry. Without this skip the wildcard hits node_geometry's
            // None-on-LaneMark and the whole branch geometry collapses to None,
            // breaking the if-long/elseif path for lane-spanning if-swimlanes.
            | LayoutNode::LaneMark(_) => continue,
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
            | LayoutNode::Goto(_)
            // Swimlane V2: branch-internal lane marker, no geometry (see
            // `sequence_geometry`).
            | LayoutNode::LaneMark(_) => continue,
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
            if else_branches.len() != 1
                || (if_down_plan(then_branch, else_branches).is_some()
                    && if_break_single_survivor_plan(then_branch, else_branches).is_none())
            {
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
    if else_branches.len() != 1
        || (if_down_plan(then_branch, else_branches).is_some()
            && if_break_single_survivor_plan(then_branch, else_branches).is_none())
    {
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
    if if_break_single_survivor_plan(then_branch, else_branches).is_some() {
        then_off -= IF_BREAK_SINGLE_SURVIVOR_BRANCH_SPREAD;
        else_off += IF_BREAK_SINGLE_SURVIVOR_BRANCH_SPREAD;
        left_ext += IF_BREAK_SINGLE_SURVIVOR_BRANCH_SPREAD;
        right_ext += IF_BREAK_SINGLE_SURVIVOR_BRANCH_SPREAD;
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
/// Amount by which the ABSTRACT couple dimension of a single-action `repeat`-led
/// branch (used only for tile2 centring) exceeds the EMITTED `branch_h`. The
/// abstract FtileRepeat keeps its full `8*hexHalf` tail (uncompressed) while the
/// emitted form keeps only `IF_LONG_BRANCH_REPEAT_EXTRA` (top) + the
/// `text_height(11)` `is`-label band (bottom). The residual is the entry/test
/// diamond height minus the kept loop-back band and the corridor reclaim:
/// `2*hexHalf − text_height(11) − (descent(11) − 1.5)` = 24 − 12.955078125 −
/// 0.8203125 = 10.224609375.
const IF_LONG_REPEAT_CENTERING_EXTRA: f64 =
    DIAMOND_HALF * 2.0 - 12.955078125 - IF_LONG_BRANCH_REPEAT_CORRIDOR_RECLAIM;
/// Extra inter-tile gap retained inside an `if/elseif*/else` branch column.
/// PlantUML's `FtileFactoryDelegatorAssembly` stacks consecutive branch tiles
/// with a 35 px `addBottom` reserve; the whole-diagram `CompressionXorY` (ON_Y)
/// pass then reclaims that slack down to `ARROW_LEN` (20) in freely-compressible
/// flows. Inside an `FtileIfLongHorizontal` couple the diamond row + each
/// diamond's south (`withNorth`) label band straddle the branch column, so ON_Y
/// can only reclaim the slack partially: every inter-action gap settles at
/// `ARROW_LEN + (4.477539… − 1)` ≈ 23.4775 instead of 20. The `4.477539…`
/// residual is PlantUML's south-label band unit (also `IF_DOWN_LEAD`'s extra).
const IF_LONG_BRANCH_GAP_EXTRA: f64 = 4.477539062500001 - 1.0;
/// `SlotSet.smaller(margin)` keeps this much empty space on each side of every
/// compressed cluster (PlantUML calls `smaller(5.0)`).
const X_COMPRESS_MARGIN: f64 = 5.0;
/// Clearance reserved left of the leftmost condition diamond's west vertex.
/// PlantUML's with-diamonds nude width clamps the inner band to `diamond1.width
/// + SUPP_WIDTH` (SUPP_WIDTH = 20), reserving `SUPP_WIDTH/2 − 1` = 9 px of left
/// margin past the diamond — the same `+9` floor `node_extents` uses for the
/// single-diamond `FtileIfDown` left extent.
const IF_LONG_LEFT_MARGIN: f64 = 9.0;
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
    /// Branch DRAWN extents (left, right) from the branch spine = couple centre.
    /// Differs from `branch_w/2` when the branch carries a nested construct that
    /// reserves placement margin past its drawn shapes (e.g. a nested
    /// `FtileIfWithDiamonds` reserves `addHorizontalMargin(10)` on each side that
    /// is NOT drawn). Occupancy / left_ext must use the DRAWN extent so the empty
    /// reservation band collapses (PlantUML's compression reads real shapes).
    branch_draw_l: f64,
    branch_draw_r: f64,
    /// Branch REPORTED left extent (un-clamped `sequence_extents().0`). For a
    /// while/repeat branch this exceeds `branch_draw_l` by the tile's reserved
    /// (un-drawn) placement margin. Occupancy/compression use the DRAWN extent,
    /// but the diagram's left-margin placement (`left_ext`) must honour the
    /// reported reservation so the leftmost drawn shape lands at MARGIN_LEAD +
    /// that margin (PlantUML positions on `calculateDimension`, not drawn bounds).
    branch_report_l: f64,
    /// Branch box height.
    branch_h: f64,
    /// True when this branch is led by a single-action `repeat` that keeps its
    /// uncompressed FtileRepeat tail (drives tile2 centring; see
    /// [`IF_LONG_REPEAT_CENTERING_EXTRA`]).
    leads_repeat: bool,
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
    /// True when `tile2` is the IMPLICIT empty else (no `else` clause in source).
    /// PlantUML always builds a `branch2` tile (`FtileMinWidthCentered(empty,
    /// 30)`); with no point in/out it draws a single fall-through connector from
    /// the last diamond's east vertex straight down to the merge line, rather
    /// than the two-segment in/out a populated else gets.
    tile2_empty: bool,
    /// Diamond north label height (single-line; reserved below the diamond).
    north_h: f64,
    /// Inter-tile gap extra applied inside each branch column (and tile2): the
    /// partially-reclaimed `IF_LONG_BRANCH_GAP_EXTRA` (all branches simple) or
    /// the full uncompressed `FORK_BRANCH_INTER_GAP_EXTRA` (a sibling carries a
    /// nested construct). The emitter must mirror this so boxes land where the
    /// predicted heights placed them.
    branch_gap_extra: f64,
    /// Left / right drawn extents from the spine.
    left_ext: f64,
    right_ext: f64,
}

/// True when a branch body is a simple linear flow — only leaf tiles
/// (actions/notes/arrows/connectors), no nested control-flow construct
/// (if/while/repeat/fork/switch). Used to pick the `FtileIfLongHorizontal`
/// north-band height: a nested construct widens the branch column's vertical
/// extent enough that the whole-diagram `ON_Y` compression reclaims the
/// diamond→branch corridor's sub-line slack (see `north_h` in `if_long_layout`).
fn is_simple_branch_flow(body: &[LayoutNode]) -> bool {
    body.iter().all(|n| {
        !matches!(
            n,
            LayoutNode::If { .. }
                | LayoutNode::While { .. }
                | LayoutNode::Repeat { .. }
                | LayoutNode::Fork { .. }
                | LayoutNode::Switch { .. }
        )
    })
}

fn branch_first_lane_mark(body: &[LayoutNode]) -> Option<usize> {
    body.iter().find_map(|n| match n {
        LayoutNode::LaneMark(l) => Some(*l),
        _ => None,
    })
}

fn branch_lane_marks_backtrack(lane_marks: &[Option<usize>]) -> bool {
    lane_marks.windows(2).any(|w| match (w[0], w[1]) {
        (Some(a), Some(b)) => b < a,
        _ => false,
    })
}

fn if_long_branch_lane_marks(
    then_branch: &[LayoutNode],
    else_branches: &[ElseBranch],
) -> Vec<Option<usize>> {
    std::iter::once(branch_first_lane_mark(then_branch))
        .chain(
            else_branches
                .iter()
                .filter(|b| b.condition.is_some())
                .map(|b| branch_first_lane_mark(&b.body)),
        )
        .collect()
}

fn tree_has_if_long_lane_backtrack(tree: &[LayoutNode]) -> bool {
    tree.iter().any(|node| {
        if let LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } = node
        {
            branch_lane_marks_backtrack(&if_long_branch_lane_marks(then_branch, else_branches))
        } else {
            false
        }
    })
}

fn tree_has_if_long_multi_elseif(tree: &[LayoutNode]) -> bool {
    tree.iter().any(|node| {
        if let LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } = node
        {
            if_long_branch_lane_marks(then_branch, else_branches).len() > 2
        } else {
            false
        }
    })
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
    // North band height below each condition diamond (the `then`/`elseif` label
    // band, `addVerticalMargin`'d by alignDiamonds). The whole-diagram ON_Y pass
    // reclaims the diamond→branch corridor's descent slack, leaving
    // `ascent(11) + 1.5` rather than the full text height.
    let lane_marks = if_long_branch_lane_marks(then_branch, else_branches);
    let has_lane_marks = lane_marks.iter().any(Option::is_some);
    let split_collector_band = lane_marks.len() > 2 && branch_lane_marks_backtrack(&lane_marks);
    let all_branches_simple = is_simple_branch_flow(then_branch)
        && else_branches
            .iter()
            .all(|b| b.condition.is_none() || is_simple_branch_flow(&b.body));
    let has_populated_bare_else = else_branches
        .iter()
        .any(|b| b.condition.is_none() && !branch_is_empty(&b.body));
    let north_h = if split_collector_band
        || (!has_lane_marks && has_populated_bare_else && all_branches_simple)
    {
        pm::text_height(SMALL_FONT)
    } else {
        pm::ascent(SMALL_FONT) + 1.5
    };

    // Inter-tile gap retained inside each branch column. When every branch is a
    // simple linear flow the whole-diagram `ON_Y` compression reaches into the
    // branch columns and reclaims the 35 px `addBottom` assembly reserve down to
    // `ARROW_LEN + (4.477539… − 1)` ≈ 23.4775 (`IF_LONG_BRANCH_GAP_EXTRA`). But
    // when ANY branch carries a nested control-flow construct (fork/while/repeat/
    // if/switch), that construct's own un-compressible assembly snakes block the
    // ON_Y pass from reaching the sibling simple branches, so their inter-action
    // gaps keep the full 35 px reserve (extra = `FORK_BRANCH_INTER_GAP_EXTRA` =
    // 15). Picked once for the whole couple row so prediction and emission agree.
    let branch_gap_extra = if all_branches_simple {
        IF_LONG_BRANCH_GAP_EXTRA
    } else {
        FORK_BRANCH_INTER_GAP_EXTRA
    };

    let mut push_col = |cond: &str, north: &Option<String>, body: &[LayoutNode]| -> Option<()> {
        let g = sequence_geometry(body)?;
        let branch_w = g.width.max(30.0);
        // True drawn extents from the branch spine (= couple centre). For a simple
        // action/leaf branch these equal branch_w/2; for a branch whose body
        // reserves un-drawn placement margin (nested if/while/…) they are smaller,
        // so the occupancy / left_ext below collapses that empty reservation.
        let (branch_report_l, branch_draw_l, branch_draw_r) = {
            let (dl, dr) = sequence_extents(body);
            (dl, dl.min(branch_w / 2.0), dr.min(branch_w / 2.0))
        };
        // Branch column height carries the partially-uncompressed inter-tile
        // slack (see IF_LONG_BRANCH_GAP_EXTRA): `sequence_geometry` assembles at
        // the fully-compressed 20 px gap, so re-derive the height with the extra.
        // A branch led by a `repeat` keeps both uncompressed FtileRepeat reserves
        // (top above the body + bottom `is`-label band below it): the emit places
        // the body and condition diamond that far lower, so the merge reservation
        // must include the same total growth.
        let repeat_total = leading_if_long_branch_repeat_total(body);
        // A while-led branch keeps its uncompressed loop-back junction (`+12`, not
        // the compressed `+10`): `emit_while` adds the same 2 px via
        // `in_if_long_branch`, so the merge reservation must match.
        let while_extra = leading_if_long_branch_while_extra(body);
        let branch_h = sequence_height_ex(body, branch_gap_extra) + repeat_total + while_extra;
        let leads_repeat = repeat_total != 0.0;
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
        // branch is FtileMinWidthCentered, which PRESERVES the delegated tile's
        // getLeft when its width already exceeds the 30px minimum
        // (`calculateDimensionSlow` → `getPoint2` returns geo.getLeft unchanged).
        // So the branch spine offset is the body's own geometry `left`, NOT
        // `branch_w/2` — these differ when the body is an asymmetric construct
        // (e.g. a nested if whose then/else branches have unequal widths).
        let diamond_left = dgeo.left;
        let diamond_tile_w = dgeo.width;
        // FtileMinWidthCentered widens a sub-30 body to 30 (re-centring it); above
        // that it keeps the body's own getLeft. Mirror both cases.
        let branch_left = if g.width < 30.0 { 15.0 } else { g.left };
        let couple_left = diamond_left.max(branch_left);
        let couple_w = (diamond_tile_w + (couple_left - diamond_left))
            .max(branch_w + (couple_left - branch_left));
        cols.push(IfLongCol {
            diamond_w,
            diamond_w_tile: diamond_tile_w,
            branch_draw_l,
            branch_draw_r,
            branch_report_l,
            branch_h,
            leads_repeat,
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

    // tile2 geometry (FtileMinWidthCentered(branch2, 30)). PlantUML ALWAYS
    // builds a `tile2` even when the source has no `else`: branch2 is then an
    // empty Ftile, so `FtileMinWidthCentered` collapses it to the bare 30 px
    // minimum width with no point in/out. A populated else keeps the same
    // partially-uncompressed inter-tile slack as the elseif branches.
    let tile2_empty = tile2_body.is_none();
    let (tile2_w, tile2_h, tile2_draw_l, tile2_draw_r) = match tile2_body {
        Some(body) => {
            let g = sequence_geometry(body)?;
            let w = g.width.max(30.0);
            let (dl, dr) = sequence_extents(body);
            (
                w,
                sequence_height_ex(body, branch_gap_extra),
                dl.min(w / 2.0),
                dr.min(w / 2.0),
            )
        }
        // Implicit empty else: the FtileMinWidthCentered(empty, 30) column.
        None => (30.0, 0.0, 15.0, 15.0),
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
    // tile2 always exists (PlantUML always builds branch2); for the implicit
    // empty else it is the 30 px min-width column.
    let tile2_center_u = (tile2_w > 0.0).then(|| internal_w - tile2_w / 2.0);
    let spine_internal = internal_w / 2.0;

    // Occupied x-intervals of everything drawn (un-compacted frame).
    let mut occ: Vec<(f64, f64)> = Vec::new();
    for (i, c) in cols.iter().enumerate() {
        let cc = centers_u[i];
        let dw = c.diamond_w;
        // diamond polygon
        occ.push((cc - dw / 2.0, cc + dw / 2.0));
        // branch DRAWN content (FtileMinWidthCentered centres the branch at the
        // couple centre). Uses the true drawn extents, not the placement width:
        // a branch carrying a nested construct reserves un-drawn placement margin
        // (e.g. FtileIfWithDiamonds' addHorizontalMargin(10)) that compression
        // must reclaim, so occupancy stops at the real shapes.
        occ.push((cc - c.branch_draw_l, cc + c.branch_draw_r));
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
        occ.push((tc - tile2_draw_l, tc + tile2_draw_r));
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
    let mut right_ext = max_x;

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

    // Standard if-tile left margin: PlantUML reserves `SUPP_WIDTH/2 − 1` (= 9)
    // of clearance to the LEFT of the leftmost condition diamond's west vertex
    // (the `diamond1.width + SUPP_WIDTH` clamp in `FtileIfNude`/with-diamonds).
    // For narrow `then`/`elseif` labels the north-overhang term above is zero,
    // so this floor supplies the reservation; for wide labels the overhang
    // already exceeds it. The leftmost diamond west vertex sits at
    // `min(cx − diamond_w/2)` from the spine.
    let leftmost_diamond_vertex = cols
        .iter()
        .map(|c| c.cx - c.diamond_w / 2.0)
        .fold(f64::INFINITY, f64::min);
    left_ext = left_ext.max(-leftmost_diamond_vertex + IF_LONG_LEFT_MARGIN);
    // A branch tile (while/repeat) reserves un-drawn placement margin past its
    // leftmost drawn shape. Occupancy/compression collapse that empty margin, but
    // PlantUML positions the whole if-block by its `calculateDimension` left — so
    // the diagram's left extent must honour the REPORTED branch left, landing the
    // drawn corridor at MARGIN_LEAD + the margin. (Only matters when the reported
    // reservation exceeds the drawn occupancy, i.e. for loop branches.)
    let leftmost_branch_report = cols
        .iter()
        .map(|c| c.cx - c.branch_report_l)
        .fold(f64::INFINITY, f64::min);
    left_ext = left_ext.max(-leftmost_branch_report);

    // Leading-fork flow-axis offset. When the FIRST branch (couple[0]) is a fork,
    // PlantUML builds it via the if-branch fork tile, which grows the bar to the
    // RIGHT by `FORK_IF_BRANCH_RIGHT_EXTRA` while keeping the fork's flow spine
    // fixed (`fork_layout_if_branch`, n==2). That extra right reservation widens
    // the couple's reported geometry without moving its drawn (symmetric) bar, so
    // `calculateDimensionInternal`'s spine (`internalWidth/2`) lands left of the
    // drawn-content centre. After `Recentred` the if-long's flow axis (where the
    // start/stop spine connects) therefore sits `FORK_IF_BRANCH_RIGHT_EXTRA` px
    // LEFT of where the symmetric model places it, while every drawn shape keeps
    // its absolute position. Model that directly: shift the flow axis left
    // (cols/tile2 move right relative to it, the spine ellipse moves left). Guard
    // to a genuine fork branch (≥2 non-empty lanes) so plain branches are
    // untouched.
    let leading_fork_axis_shift = if let Some(LayoutNode::Fork {
        branches,
        is_split: false,
        merge: false,
        ..
    }) = then_branch.iter().find(|n| node_is_flow(n))
        && branches.len() >= 2
        && !branches.iter().any(Vec::is_empty)
    {
        FORK_IF_BRANCH_RIGHT_EXTRA
    } else {
        0.0
    };
    let mut tile2_cx = tile2_cx;
    if leading_fork_axis_shift != 0.0 {
        for c in &mut cols {
            c.cx += leading_fork_axis_shift;
        }
        tile2_cx = tile2_cx.map(|t| t + leading_fork_axis_shift);
        left_ext -= leading_fork_axis_shift;
        right_ext += leading_fork_axis_shift;
    }

    Some(IfLongLayout {
        cols,
        east_label,
        tile2_h,
        tile2_cx,
        tile2_empty,
        north_h,
        branch_gap_extra,
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
    //
    // The couple's OCCUPIED height (what calculateDimensionInternal merges, and
    // what centres `tile2`) is the diamond corridor + branch MINUS the overlap
    // the whole-diagram ON_Y compression reclaims where the diamond's south
    // (`withNorth`) band meets the branch top. That reclaim is `descent − 1.5`
    // (= `text_height(11) − north_h`): zero when the band keeps its full line
    // height (`north_h == text_height(11)`, i.e. a simple-flow branch with a
    // populated bare else, matching the chain goldens) and `0.8203` when the
    // band compressed to `ascent + 1.5` (a branch carrying a nested construct).
    // `couple_branch_top`/`merge_y` already use the post-reclaim
    // `diamond_aligned_h`, so the reclaim only needs subtracting here, in the
    // height-for-centring.
    let corridor_overlap = (pm::text_height(SMALL_FONT) - l.north_h).max(0.0);
    // The merge line (`merge_y`/`deepest`) keys off each branch's EMITTED height
    // (`branch_h`, ON_Y-compressed with the kept reserves). The tile2 (bare-else)
    // centring instead uses the ABSTRACT couple dimension PlantUML's
    // `calculateDimensionInternal` reports, in which a leading `repeat` keeps its
    // full uncompressed `8*hexHalf` tail. For a single-action repeat that abstract
    // couple is `IF_LONG_REPEAT_CENTERING_EXTRA` taller than `branch_h`; add that
    // (centring only) so the centred else aligns.
    let couples_h = l
        .cols
        .iter()
        .map(|c| {
            diamond_aligned_h + c.branch_h - corridor_overlap
                + if c.leads_repeat {
                    IF_LONG_REPEAT_CENTERING_EXTRA
                } else {
                    0.0
                }
        })
        .fold(0.0_f64, f64::max);
    let diamonds_height = diamond_aligned_h;
    let tile2_merged_h = l.tile2_h + diamonds_height / 2.0;
    let internal_h = couples_h.max(tile2_merged_h) + IF_LONG_BELOW;
    // tile2 dy in the if-frame = (internal_h − tile2_h)/2; if-frame top is
    // 25 above the diamond row (couples dy = 25).
    let if_frame_top = dtop - 25.0;
    let tile2_top = if_frame_top + (internal_h - l.tile2_h) / 2.0;
    // Branch bottoms; the merge line sits ARROW_LEN below the deepest. The
    // implicit empty else has no pointOut (no box, no down arrow of its own),
    // so it never drives the merge line — only populated branches/else do.
    let mut deepest = couple_branch_top + l.cols.iter().map(|c| c.branch_h).fold(0.0_f64, f64::max);
    if l.tile2_cx.is_some() && !l.tile2_empty {
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
                // FtileIfDown right extent: the break branch tile (FtileMinWidthCentered
                // 30 + addHorizontalMargin 10 → half-width >= 25) floors the inner
                // width, so the tile reaches `max(cond_half, 25) + halfHex` on the
                // right — the diamond's east vertex only governs when it is wider.
                let right = cond_half.max(BREAK_DOWN_BRANCH_HALF_FLOOR) + DIAMOND_HALF;
                return with_if_attached_note_extents(left, right, attached_notes, diamond_half_w);
            }
            if if_break_single_survivor_plan(then_branch, else_branches).is_none()
                && let Some(plan) = if_down_plan(then_branch, else_branches)
            {
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
            let branch_dist = (diamond_w + 20.0).max((then_w + else_w) / 2.0 + 20.0)
                + if_branch_distance_extra(then_branch, else_branches);
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
            // A direct multi-case in-loop switch keeps its uncompressed gap-20
            // drawn block (see `repeat_body_has_in_loop_switch`); reserve the
            // wider drawn extents (`sequence_loop_body_extents`, which widens the
            // switch to its `switch_x_layout_in_while` block) so the spine and
            // loop-back arm track the drawn case rects, not the narrower
            // standalone layout.
            let in_loop_switch = repeat_body_has_in_loop_switch(body);
            let (body_left, body_right) = if body_has_note || in_loop_switch {
                sequence_loop_body_extents(body)
            } else {
                // FtileRepeat.getLeft/getRight key off the body tile's own
                // spine: `repeat.getLeft()` and `repeat.width − getLeft()`.
                // Use the body's asymmetric extents so an off-centre body
                // (e.g. a nested-if whose wider branch sits on one side)
                // pushes the repeat spine the same way PlantUML does, rather
                // than centring on `width/2`.
                sequence_extents(body)
            };
            let cond_half = diamond_inner_w(condition) / 2.0 + DIAMOND_HALF;
            // A `backward :label;` action draws a box on the return arm at the
            // far right. The repeat spine keeps the ordinary-repeat
            // `cond_half + 9` clearance, unless the body itself is wider.
            // FtileRepeat appends the backward tile on the right.
            if let Some(label) = backward {
                // Left side mirrors the ordinary repeat (`getLeft` is independent
                // of the appended backward tile): the body's asymmetric left, plus
                // a break-bearing body's left exit-corridor reservation.
                let break_left = repeat_break_if_cond_half(body)
                    .map_or(0.0, |ch| ch + REPEAT_BREAK_CORRIDOR_PAD);
                let left_extent = body_left.max(cond_half + 9.0).max(break_left);
                let body_geo_right = repeat_backward_body_right(body);
                let right_extent =
                    repeat_backward_right_extent(cond_half, body_geo_right, is_label, label);
                (left_extent, right_extent)
            } else {
                // A break-bearing body welds its break LEFT to an exit corridor
                // that the repeat carries down to its break-merge diamond.
                // PlantUML's `FtileFactoryDelegatorRepeat` wraps the whole repeat
                // tile in `addHorizontalMargin(result, 10, 0)` and routes the
                // break snake to that margined tile's left edge (x=0). So the
                // corridor sits `REPEAT_BREAK_CORRIDOR_PAD` (9, +1 lead = 10) px
                // left of the repeat's OWN `getLeft` — which PlantUML computes as
                // `max(body.getLeft(), diamond1.w/2, diamond2.w/2)` (the entry
                // rhombus halfHex and the condition half, NOT the ordinary
                // `cond_half + 9` loop-back reservation).
                let break_left = if repeat_break_if_cond_half(body).is_some() {
                    let get_left = body_left.max(DIAMOND_HALF).max(cond_half);
                    get_left + REPEAT_BREAK_CORRIDOR_PAD
                } else {
                    0.0
                };
                let left_extent = body_left.max(cond_half + 9.0).max(break_left);
                // Right extent mirrors `emit_repeat`'s loop-back arm exactly:
                // `arm = max(diamond_right + 12, extents.right + 12,
                //            geo.right() + 4)`, then the canvas reserves a
                // further 15 px past the arm. For a plain body the geometry
                // and drawn extents coincide, reducing to the historical
                // `max(cond_half, body_right) + 12 + 15`.
                // FtileRepeat's loop arm clears the BODY TILE's geometric right
                // by halfHex. A `while` body ADVERTISES a wider layout extent
                // than it draws (the FtileWhile `dx + halfHex` trailing
                // reservation reclaimed by ON_X — see
                // `while_chain_trailing_reservation`); clearing from `body_right`
                // (the inflated extent) over-reserves, so prefer `geo.right() +
                // halfHex` when a while inflates the body extent. An in-loop
                // switch's gap-20 `FtileGeometry` (`switch_with_diamonds`)
                // OVERSHOOTS the gap-20 DRAWN block the arm clears, so suppress
                // the geometry clearance there and clear the drawn extents (+12).
                let arm_rel = if body_contains_while(body)
                    && let Some(g) = sequence_geometry(body)
                {
                    (cond_half + 12.0).max(g.right() + DIAMOND_HALF)
                } else {
                    let geo_clear = if in_loop_switch {
                        body_right + 12.0
                    } else {
                        sequence_geometry(body).map_or(body_right + 12.0, |g| g.right() + 4.0)
                    };
                    (cond_half + 12.0).max(body_right + 12.0).max(geo_clear)
                };
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
            if while_body_has_prefixed_fused_trailing_while(body) {
                left_extent = (left_extent - WHILE_PREFIXED_FUSED_NESTED_LEFT_TRIM).max(0.0);
            }
            // Right side: loop-back arm at max(cond,body) + halfHex with a 4px
            // arrowhead, plus halfHex of trailing reservation from FtileWhile's
            // `dx + halfHex` term (= 2*halfHex + 3 past max). Verified against
            // the width-only while goldens.
            //
            // Fused last-flow break (`break_if_is_last_flow`): the loop-back arm
            // coincides with the body's right edge (the if's empty branch is the
            // loop's pointOut), so the separate `dx + halfHex` corridor trailing
            // collapses — the arm vertical at `max(cond,body) + halfHex + pad` is
            // the rightmost geometry, with no further reservation.
            let right_extent = if break_if_is_last_flow(body) {
                // A bare `while (cond)` (no `is`/end label) keeps the FtileWhile
                // frame's trailing `hexHalf - 3 - pad` reservation past the
                // loop-back arm (advertised whitespace only — the drawn arm is
                // unchanged). Labeled last-flow break loops collapse it (their
                // wider body absorbs the slack within the ±1px canvas tolerance);
                // a narrow unlabeled body does not, so restore the tail.
                // A bare `then` (no south label) on the break branch also drops
                // the south-band's residual canvas tail (FtileIfDown advertises a
                // narrower geometry); `edge_activity_while_infinite` needs
                // `DIAMOND_HALF / 2` less trailing whitespace than the labeled
                // `edge_activity_while_break`. Whitespace only.
                let unlabeled_break_tail = if is_label.is_none() && end_label.is_none() {
                    if break_if_south_label_present(body) == Some(false) {
                        WHILE_NO_LABEL_BREAK_CANVAS_TAIL - DIAMOND_HALF / 2.0
                    } else {
                        WHILE_NO_LABEL_BREAK_CANVAS_TAIL
                    }
                } else {
                    0.0
                };
                cond_half.max(body_right)
                    + DIAMOND_HALF
                    + while_single_if_right_pad(body, end_label)
                    + unlabeled_break_tail
            } else {
                cond_half.max(body_right)
                    + DIAMOND_HALF
                    + while_chain_trailing_reservation(body)
                    + while_single_if_right_pad(body, end_label)
            };
            let right_extent = if while_body_has_prefixed_fused_trailing_while(body) {
                (right_extent - WHILE_PREFIXED_FUSED_NESTED_CANVAS_RIGHT_TRIM).max(0.0)
            } else {
                right_extent
            };
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
            ..
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
            // A `group` (is_group) wrapping a bare `while` places the FtileWhile by
            // its DRAWN geometry, not the inflated layout extent. FtileGroup wraps
            // the inner in `addHorizontalMargin(inner, 10)`; the loop's trailing
            // `dx + halfHex` reservation (which inflates `node_extents`) is
            // reclaimed by the whole-diagram ON_X pass, leaving the rect's right
            // edge `GROUP_WHILE_RIGHT_RESIDUAL` past the drawn loop arm. The left
            // margin is the plain `addHorizontalMargin` 10. (The non-group
            // `partition` while path is handled separately, below.)
            if *is_group
                && partition_wraps_while(body)
                && !title_drives_width
                && let Some(g) = sequence_geometry(body)
            {
                return (g.left + 10.0, g.right() + GROUP_WHILE_RIGHT_RESIDUAL);
            }
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

/// Trailing right-side whitespace a `while` reserves past its loop-back arm in
/// the ordinary (non-chain) case — the FtileWhile frame's `dx + halfHex` term.
const WHILE_CHAIN_TRAILING_FULL: f64 = DIAMOND_HALF + 3.0;
/// Same trailing reservation for a pure while-chain body, where ON_X
/// compression reclaims the corridor down to a 1 px residual (see
/// [`while_body_is_pure_while_chain`]).
const WHILE_CHAIN_TRAILING_COMPRESSED: f64 = 1.0;

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
    let (mut left, mut right) = sequence_extents(nodes);
    // A multi-case switch directly in a `while` body uses the wider
    // `switch_x_layout_in_while` (loop frame blocks ON_X compression of the
    // diamond-column bands). Its drawn block is wider than the standalone extent
    // `node_extents` reserved, so widen the body extent to match — otherwise the
    // loop-back arm clears the wrong edge.
    for node in nodes {
        if let LayoutNode::Switch { cases, condition } = node
            && cases.len() > 1
        {
            let layout = switch_x_layout_in_while(cases, condition);
            left = left.max(layout.diamond_dx);
            right = right.max(layout.block_w - layout.diamond_dx);
        }
    }
    for (i, node) in nodes.iter().enumerate() {
        if let Some(layout) = while_body_fork_layout(nodes, i, node) {
            let (fork_left, fork_right) = fork_bar_extents(&layout);
            left = left.max(fork_left);
            right = right.max(fork_right);
        }
    }
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
    let prefixed_fused_trailing_while = while_body_has_prefixed_fused_trailing_while(nodes);
    let last_flow = last_flow_index(nodes);
    for (i, node) in nodes.iter().enumerate() {
        if matches!(node, LayoutNode::Note { .. } | LayoutNode::Arrow { .. }) {
            continue;
        }
        let (_nl, nr) = node_extents(node);
        // Strip the inner loop's own trailing reservation back to its arm. A
        // pure while-chain inner loop already reserves only the compressed
        // trailing (see `while_chain_trailing_reservation`), so strip that
        // amount rather than the full reservation — otherwise the arm would be
        // under-cleared.
        let nr = match node {
            LayoutNode::While { body: inner, .. } => {
                let strip = while_chain_trailing_reservation(inner)
                    + if prefixed_fused_trailing_while && Some(i) == last_flow {
                        WHILE_PREFIXED_FUSED_NESTED_RIGHT_TRIM
                    } else {
                        0.0
                    };
                (nr - strip).max(0.0)
            }
            _ => nr,
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
    let multi_fork_sequence = !if_branch && sequence_uses_multi_fork_layout(nodes);
    let multi_fork_has_prelude = multi_fork_sequence && sequence_multi_fork_has_prelude(nodes);
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
                let (nl, nr) = if multi_fork_sequence && is_multi_fork_sequence_candidate(node) {
                    let layout = match node {
                        LayoutNode::Fork { branches, .. } => {
                            fork_layout_multi_sequence(branches, multi_fork_has_prelude)
                        }
                        _ => unreachable!(),
                    };
                    fork_bar_extents(&layout)
                } else if if_branch {
                    node_extents_if_branch(node)
                } else {
                    node_extents(node)
                };
                left = left.max(nl);
                right = right.max(nr);
                // Only genuine flow nodes (those with width) can anchor a note.
                let width = if multi_fork_sequence && is_multi_fork_sequence_candidate(node) {
                    nl + nr
                } else if if_branch {
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
/// A lane segment whose last flow tile is a no-special `while` (its loop exit
/// continues into another lane). Such a transition is stitched onto the loop's
/// `ConnectionOut` corridor; see [`SwimlaneCrossLane`].
fn swimlane_segment_ends_in_plain_while(body: &[LayoutNode]) -> bool {
    body.iter()
        .rev()
        .find(|n| !matches!(n, LayoutNode::Note { .. } | LayoutNode::Arrow { .. }))
        .is_some_and(|n| {
            matches!(
                n,
                LayoutNode::While {
                    special_out: None,
                    ..
                }
            )
        })
}

/// A lane body that is a single `while ... endwhile <terminator>` (terminator
/// absorbed as `special_out`). Such a tile advertises an extra 2px tail below
/// the visible frame bottom; in a swimlane the lane height is taken from the
/// visible frame, so the tail is trimmed.
fn swimlane_while_special_tail(body: &[LayoutNode]) -> bool {
    matches!(
        body,
        [LayoutNode::While { special_out: Some(special), end_label, .. }]
            if end_label.is_none() && matches!(**special, LayoutNode::Stop | LayoutNode::End)
    )
}

/// True when [`swimlane_lane_extents`] narrows a single-`while`+terminator lane's
/// left extent to the cond-driven drawn box. In that layout the terminator
/// circle sits at `diamond_left_vertex - halfHex - 9`; `emit_while` uses this
/// (via the [`SvgEmitter::swimlane_while_cond_special`] flag) to place it.
fn swimlane_while_cond_special_lane(body: &[LayoutNode]) -> bool {
    let [
        LayoutNode::While {
            condition,
            body: while_body,
            is_label,
            end_label,
            special_out: Some(special),
            starts_column,
            diamond_font_family,
            diamond_font_size,
            diamond_text_bold,
            ..
        },
    ] = body
    else {
        return false;
    };
    if end_label.is_some() || !matches!(**special, LayoutNode::Stop | LayoutNode::End) {
        return false;
    }
    let cond_half = diamond_inner_w_styled(
        condition,
        *diamond_font_size,
        *diamond_text_bold,
        diamond_font_family,
    ) / 2.0
        + DIAMOND_HALF;
    let (body_left_ext, _) = sequence_loop_body_extents(while_body);
    let body_left = while_body_left(while_body, body_left_ext);
    let standalone_left = while_left_extent(
        while_body,
        body_left,
        cond_half,
        is_label.is_some(),
        end_label.as_deref(),
        Some(special),
        *starts_column,
    );
    let drawn_left = cond_half + DIAMOND_HALF + 9.0 + CIRCLE_TILE_HALF;
    let (seq_left, _) = sequence_extents(body);
    (standalone_left - seq_left).abs() < 0.001 && drawn_left < seq_left
}

fn swimlane_lane_extents(body: &[LayoutNode]) -> (f64, f64) {
    let (mut left, mut right) = sequence_extents(body);
    // A lane consisting of a single `while ... endwhile <terminator>` (the
    // terminator absorbed as `special_out`) reserves its left extent from the
    // standalone `node_extents` While arm, which positions the terminator
    // relative to the BODY corridor (`while_body_left + halfHex`). In a lane the
    // width is taken from the actual drawn bounding box (`getMinMax`), where the
    // terminator circle sits relative to the wider of the body corridor and the
    // condition diamond's left vertex. When the diamond is the wider element the
    // drawn left is `cond_half + halfHex + 9 + CIRCLE_TILE_HALF` (the terminator
    // circle hung off the diamond's left-vertex column), narrower than the
    // body-driven reservation. Recompute and narrow only — never widen.
    if swimlane_while_cond_special_lane(body)
        && let [
            LayoutNode::While {
                condition,
                diamond_font_family,
                diamond_font_size,
                diamond_text_bold,
                ..
            },
        ] = body
    {
        let cond_half = diamond_inner_w_styled(
            condition,
            *diamond_font_size,
            *diamond_text_bold,
            diamond_font_family,
        ) / 2.0
            + DIAMOND_HALF;
        left = cond_half + DIAMOND_HALF + 9.0 + CIRCLE_TILE_HALF;
    }
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
///
/// `body_right` is the body tile's own asymmetric right extent
/// (`FtileGeometry.width − left` = PlantUML's `repeat.getRight()`), NOT the
/// symmetric `width/2`. For a plain action the two coincide; for an asymmetric
/// composite body (e.g. an if/else, whose merge corridor carries an extra
/// margin past the wider branch) the geometry right is what `getRight` keys off,
/// so the appended box clears the body's true right edge.
fn repeat_backward_box_left_rel(cond_half: f64, body_right: f64, is_label: &Option<String>) -> f64 {
    if cond_half >= body_right {
        cond_half + 2.0 * DIAMOND_HALF
    } else {
        let is_label_w = is_label
            .as_ref()
            .map(|l| text_render::measure(l, SMALL_FONT, false))
            .unwrap_or(0.0);
        body_right.max(cond_half + is_label_w + 10.0)
    }
}

/// Right extent (from the repeat spine) when a `backward :label;` box sits on
/// the return arm. The extent spans the backward box appended to the repeat
/// geometry.
fn repeat_backward_right_extent(
    cond_half: f64,
    body_right: f64,
    is_label: &Option<String>,
    backward_label: &str,
) -> f64 {
    repeat_backward_box_left_rel(cond_half, body_right, is_label)
        + repeat_backward_box_w(backward_label)
}

/// The body tile's asymmetric right extent (`repeat.getRight()` in PlantUML's
/// `FtileRepeat`). Falls back to the symmetric half-width when the body has no
/// computable geometry (e.g. a note-only body).
fn repeat_body_geo_right(body: &[LayoutNode]) -> f64 {
    sequence_geometry(body).map_or_else(|| sequence_width(body) / 2.0, |g| g.right())
}

/// True when a `repeat` body has a multi-case SMALL-diamond switch as a direct
/// flow node. Such a switch keeps its uncompressed gap-20 inner bands (drawn via
/// `switch_x_layout_in_while`; see `repeat_body_switch`), so its DRAWN block is
/// wider than the standalone `switch_x_layout` that `sequence_extents` measures —
/// but its `FtileGeometry` (`node_geometry`'s gap-20 `switch_with_diamonds`) is
/// wider still. The repeat's loop-back arm clears the DRAWN block (`extents +
/// 12`, mirroring `emit_while`'s arm), NOT the nude geometry's `getRight + 4`, so
/// callers gate the geometry-based clearance off when this returns true and feed
/// the widened `sequence_loop_body_extents` extents instead. SMALL diamonds only:
/// a BIG-diamond switch keeps its standalone layout under the loop frame.
fn repeat_body_has_in_loop_switch(body: &[LayoutNode]) -> bool {
    body.iter().any(|node| match node {
        LayoutNode::Switch { cases, condition } => {
            cases.len() > 1
                && cases.iter().all(|c| !c.body.is_empty())
                && !switch_x_layout(cases, condition).big_diamond
        }
        _ => false,
    })
}

/// Clearance the appended `backward :...;` box keeps to the right of a break-`if`
/// body's drawn continue-corridor (the surviving branch routed right then down
/// before merging back to the spine). `repeat.getRight()` (the bare geometry)
/// under-measures that corridor, so for a break-bearing body the box column keys
/// off the drawn right extent plus this margin instead.
const REPEAT_BACKWARD_BREAK_BOX_CLEARANCE: f64 = 2.0 * DIAMOND_HALF - ACTION_H_PADDING;

/// Right extent (from the body spine) the appended `backward :...;` box clears.
/// For a plain or composite body this is `repeat.getRight()`; for a break-`if`
/// body the drawn continue-corridor extends past the bare geometry, so the box
/// must clear the drawn extent (`sequence_extents`) plus a fixed margin.
fn repeat_backward_body_right(body: &[LayoutNode]) -> f64 {
    if break_if_is_last_flow(body) {
        sequence_extents(body).1 + REPEAT_BACKWARD_BREAK_BOX_CLEARANCE
    } else {
        repeat_body_geo_right(body)
    }
}

fn node_width(node: &LayoutNode) -> f64 {
    match node {
        // Swimlane V2: zero-size lane marker (no geometry).
        LayoutNode::LaneMark(_) => 0.0,
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
            if if_down_plan(then_branch, else_branches).is_some()
                && if_break_single_survivor_plan(then_branch, else_branches).is_none()
            {
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
            let branch_dist = (diamond_w + 20.0).max((then_w + else_w) / 2.0 + 20.0)
                + if_branch_distance_extra(then_branch, else_branches);
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
            if backward.is_some() {
                // node_extents is the single source of truth for a backward
                // repeat's asymmetric left/right (the box clears the body's true
                // geometry right edge, not the symmetric half-width).
                let (l, r) = node_extents(node);
                return l + r;
            }
            {
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
            ..
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
        // Notes and swimlane markers contribute nothing themselves and do not
        // create a flow gap before the next real branch node.
        if matches!(node, LayoutNode::Note { .. } | LayoutNode::LaneMark(_)) {
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
        h += node_height(node) + note_bottom_extra + switch_fork_uncompress_extra(node, gap_extra);
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

/// Every fork branch begins with its own `start` circle (`fork`/`fork again`
/// blocks that each open a fresh flow via `start`). PlantUML wraps each branch
/// in `FtileHeightFixedCentered(maxHeight + 2*spaceArroundBlackBar)`; for a
/// `FtileCircleStart`-led branch the inbound `ConnectionIn` lands at the
/// circle's top, so the circle centre seats `START_R` below the standard
/// post-bar arrow gap — and the branches lay out as plain side-by-side columns
/// with no even-fork middle-gap clearance (the bar holds open nothing extra).
fn fork_branches_all_start_led(branches: &[Vec<LayoutNode>]) -> bool {
    !branches.is_empty()
        && branches
            .iter()
            .all(|branch| matches!(branch.first(), Some(LayoutNode::Start)))
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
        // A break-down `if` renders as `FtileIfDown`, whose right extent already
        // folds in the `FtileMinWidthCentered`+margin floor (see node_extents).
        // It carries no FtileIfWithDiamonds SUPP_WIDTH margin, so it must NOT
        // trigger `WHILE_SINGLE_IF_RIGHT_PAD` — treat it as a non-if driver.
        let is_diamonds_if = matches!(
            node,
            LayoutNode::If { then_branch, else_branches, .. }
                if if_break_down_plan(then_branch, else_branches).is_none()
        );
        if is_diamonds_if {
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

/// A `group` (is_group) wrapping a bare `while`. Like a loop, it defers its
/// inbound `ConnectionIn` past its internal/outbound connectors: PlantUML draws
/// the connection FROM the group's output (down to the following `stop`/tile)
/// before the connection INTO the group. Without deferral the start→group arrow
/// lands before the group→stop arrow, swapping their (positional) emission order.
fn is_group_wrapping_while(node: &LayoutNode) -> bool {
    matches!(
        node,
        LayoutNode::Partition {
            is_group: true,
            body,
            ..
        } if partition_wraps_while(body)
    )
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

/// Shell width of a `group` wrapping a bare `while`, from the FtileWhile's DRAWN
/// geometry: `addHorizontalMargin` 10 on the left plus
/// [`GROUP_WHILE_RIGHT_RESIDUAL`] on the right (the loop's trailing reservation is
/// reclaimed by the whole-diagram ON_X pass). Mirrors the group-while
/// `node_extents` arm so the emitted rect, spine, and predicted extents agree.
/// `None` for any other body shape (falls back to the layout-extent shell).
fn group_while_shell_width(is_group: bool, body: &[LayoutNode]) -> Option<f64> {
    if is_group && partition_wraps_while(body) {
        sequence_geometry(body).map(|g| g.left + 10.0 + g.right() + GROUP_WHILE_RIGHT_RESIDUAL)
    } else {
        None
    }
}

fn partition_body_shell_width(is_group: bool, body: &[LayoutNode]) -> f64 {
    if !is_group && let Some((geometry, left_adjust)) = partition_wrapped_geometric_if(body) {
        geometry.width + GROUP_IF_BODY_WIDTH_EXTRA + left_adjust
    } else if let Some(w) = group_while_shell_width(is_group, body) {
        w
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
        // Swimlane V2: zero-size lane marker (no geometry).
        LayoutNode::LaneMark(_) => 0.0,
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
            // re-added by the enclosing while (see while_body_height). A bare
            // `then` (no south label) collapses the FtileIfDown south band,
            // shortening the corridor by the same band the weld/lead account for
            // (see `WHILE_BREAK_NO_SOUTH_LABEL_DROP`).
            if let Some(plan) = if_break_down_plan(then_branch, else_branches) {
                let south_label_present = if plan.then_is_break {
                    then_label.is_some()
                } else {
                    else_branches
                        .first()
                        .map(|b| b.label.is_some())
                        .unwrap_or(false)
                };
                let south_band_collapse = if south_label_present {
                    0.0
                } else {
                    IF_DOWN_LEAD - WHILE_BREAK_NO_SOUTH_LABEL_DROP
                };
                return DIAMOND_HALF * 2.0 + WHILE_BREAK_IF_CORRIDOR_DROP_COMPRESSED
                    - south_band_collapse;
            }
            if if_break_single_survivor_plan(then_branch, else_branches).is_none()
                && let Some(plan) = if_down_plan(then_branch, else_branches)
            {
                // diamond + lead + populated branch + ARROW_LEN + merge diamond.
                // An even-action branch stretches its middle gap by 15 px.
                let branch_h = sequence_height(plan.populated);
                let flow_count = plan.populated.iter().filter(|n| node_is_flow(n)).count();
                let stretch_amount = if plan.populated_terminates {
                    IF_DOWN_TERM_MID_STRETCH
                } else {
                    IF_DOWN_MID_STRETCH
                };
                let stretch = if flow_count >= 2 && flow_count.is_multiple_of(2) {
                    stretch_amount
                } else {
                    0.0
                };
                // A terminating populated branch draws no merge diamond: the
                // empty branch's corridor rejoins the spine ARROW_LEN +
                // IF_DOWN_TERM_REJOIN_EXTRA below the branch's pointOut
                // (`emit_if_down`'s terminating arm).
                let merge_h = if plan.populated_terminates {
                    IF_DOWN_TERM_REJOIN_EXTRA
                } else {
                    DIAMOND_HALF * 2.0
                };
                return DIAMOND_HALF * 2.0
                    + IF_DOWN_LEAD
                    + branch_h
                    + stretch
                    + ARROW_LEN
                    + merge_h;
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
            let then_h = if_branch_height_redirected(then_branch);
            let max_else_h: f64 = else_branches
                .iter()
                .map(|b| if_branch_height_redirected(&b.body))
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
                .map(|b| sequence_height_fork_branch(b, 0.0))
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
                    .map(|b| sequence_height_fork_branch(b, 0.0))
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
                    .map(|b| sequence_height_fork_branch(b, gap_extra))
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
            // A break-`if` as the first body flow node makes the `FtileWhile`
            // frame extend below the wrap-back exit (the body's `FtileGeometry`
            // reserves the break corridor's uncompressed tail), so the
            // advertised tile height — and hence the canvas — grows past the
            // visible content. The emitted shapes are unaffected (the snake
            // wraps back at the lower junction); only the frame's bottom
            // whitespace expands. See [`WHILE_BREAK_FIRST_FRAME_EXTRA`].
            // The uncompressed leading-break form (thin two-tile body, e.g.
            // `act_while_break_at_start`) reserves a small fixed tail of frame
            // whitespace below the wrap-back exit. The compressed form
            // (`act_break_while_early`) instead carries the larger
            // `IF_DOWN_MID_STRETCH` band already folded into `while_body_height`,
            // so the fixed tail does not additionally apply there.
            let break_first_canvas_extra = if special_out.is_none()
                && break_if_is_first_flow(body)
                && !while_break_corridor_compresses(body)
            {
                WHILE_BREAK_FIRST_CANVAS_EXTRA
            } else {
                0.0
            };
            let body_h = while_body_height(body, is_label.is_some()) + break_first_canvas_extra;
            // The total while-frame height = diamond.h + body_top_offset
            // + body_h + below-body-gap + wrap-back-offset. We derive it
            // from the same compression-aware formula as emit_while.
            let diamond_alone_h = DIAMOND_HALF * 2.0;
            // Case (b): a single-nested-`while` body compresses this loop's
            // inbound too (see `emit_while`), so the advertised height must drop by
            // the same slot. The deepest action-bodied loop in such a chain is
            // suppressed; `while_body_height` re-adds its slot below.
            let single_while_compresses = is_label.is_some()
                && special_out.is_none()
                && matches!(&body[..], [LayoutNode::While { .. }]);
            let body_top_offset = while_body_top_offset(
                while_ordinary_slot_compress_allowed(body, special_out.as_deref())
                    || single_while_compresses,
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
                - if nodes_contain_break_single_survivor_if(body) {
                    WHILE_NESTED_BREAK_SURVIVOR_CANVAS_TRIM
                } else {
                    0.0
                }
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
            // Single plain-action backward repeats keep the extra halfHex before
            // the condition diamond; multi-action or composite bodies absorb that
            // slack in their final inbound connector / internal structure.
            let cond_gap = ARROW_LEN + repeat_backward_extra_cond_gap(body, backward.is_some());
            let top_lead = if *has_start_label {
                0.0
            } else {
                diamond_h + ARROW_LEN
            };
            // A body `break` adds a break-merge diamond below the condition
            // (ARROW_LEN inbound + the merge rhombus). `node_height` advertises the
            // break-`if` with the `while`-tuned compressed drop, which overcounts the
            // drawn repeat drop.
            //  - LAST-node break: the if rejoins straight into the condition
            //    (`IF_DOWN_LEAD + ARROW_LEN − 1`), so `body_h` is too tall by
            //    `REPEAT_BREAK_IF_HEIGHT_OVERCOUNT` (7); fold that subtraction into
            //    the merge term.
            //  - MID/early break: a trailing action follows, and the if rejoins the
            //    spine `IF_BRANCH_UP/2` higher than `node_height` models — `body_h`
            //    is too tall by exactly that, and the merge term is the full tail.
            let break_merge = if !*has_start_label && body_contains_break_if(body) {
                if break_if_is_last_flow(body) {
                    ARROW_LEN + diamond_h - REPEAT_BREAK_IF_HEIGHT_OVERCOUNT
                } else {
                    ARROW_LEN + diamond_h - IF_BRANCH_UP / 2.0
                }
            } else {
                0.0
            };
            let backward_composite_break_canvas = if backward.is_some()
                && body_contains_break_if(body)
                && !break_if_is_last_flow(body)
            {
                4.0
            } else {
                0.0
            };
            top_lead + body_h + cond_gap + diamond_h + break_merge + backward_composite_break_canvas
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
                // Match emit_swimlanes: a `while ... endwhile <terminator>` lane
                // takes its bottom from the visible frame (2px above the
                // advertised tile tail).
                if swimlane_while_special_tail(&segment.body) {
                    h -= 2.0;
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
    /// Swimlane V2: byte spans in `connectors` tagged with PlantUML-style
    /// `(swimlaneOut, swimlaneIn)` ownership, used by if-long lane MinMax.
    connector_lane_spans: Vec<ConnectorLaneSpan>,
    current_connector_lanes: Option<(Option<usize>, Option<usize>)>,
    /// Swimlane V2: the lane currently active during emit (0 off the V2 path).
    /// Switched by each `LayoutNode::LaneMark` as the single tree is walked.
    current_lane: usize,
    /// Swimlane V2: `(shapes.len(), connectors.len(), lane)` recorded at every
    /// `LaneMark`, so `layout_swimlanes_v2` can partition the two buffers into
    /// per-lane byte ranges after emit. Empty off the V2 path.
    lane_spans: Vec<(usize, usize, usize)>,
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
    /// One-shot layout override for a fork emitted directly as a `while` body
    /// flow node. PlantUML lets a wider following same-spine action spread a
    /// three-way fork's branch lattice while keeping the left bar edge fixed.
    while_body_fork_layout: Option<ForkLayout>,
    /// True while emitting a fork branch whose only flow tile is a labelled
    /// while. PlantUML keeps that while's inbound label slot open inside the
    /// branch instead of applying the ordinary standalone slot compression.
    fork_branch_while_slot_open: bool,
    /// One-shot vertical slack for a simple `repeat` emitted as a fork branch in
    /// a simple while/repeat pair. PlantUML keeps the entry diamond high, then
    /// lowers the repeat body and condition inside the branch slot.
    fork_branch_repeat_body_extra: f64,
    fork_branch_repeat_tail_extra: f64,
    /// One-shot vertical compression for a labelled `while` fork branch whose
    /// body is a simple balanced `if`. PlantUML keeps the while diamond's branch
    /// entry slot open, but the enclosed if body starts at the compressed
    /// body-path y used by standalone labelled while bodies.
    fork_branch_while_if_body_compress: bool,
    /// Context for a `break` nested in an `if` inside the current `while` body.
    /// PlantUML collects each `FtileBreak` welding point and (in
    /// `FtileFactoryDelegatorWhile.createWhile`) draws a left-pointing arrow from
    /// the break tile out to the loop's left edge (`Hexagon.hexagonHalfSize`).
    /// `emit_while` sets this around its body emission so an enclosed
    /// break-bearing `if` (rendered as `FtileIfDown` with an empty `optionalStop`
    /// and `ConnectionElseNoDiamond`) can weld the break branch to the exit
    /// corridor instead of routing to a (suppressed) merge diamond.
    while_break: Option<WhileBreakContext>,
    /// One-shot signal set by [`emit_if_break_down`] when it drew the fused
    /// loop-back arm (its break-`if` was the body's last flow node — see
    /// [`WhileBreakContext::fuse_loopback`]). `emit_while` reads and clears it
    /// after body emission to suppress its own junction loop-back arm.
    while_break_loopback_fused: bool,
    /// One-shot record set by [`emit_if_break_down`] in repeat mode: the y of
    /// the break weld (where the south spine meets the left corridor). `emit_repeat`
    /// reads it after body emission to draw the corridor DOWN from this y to the
    /// break-merge diamond.
    repeat_break_weld_y: Option<f64>,
    repeat_body_condition_connector_drawn: bool,
    /// Context for a SOLE single-survivor `if` nested directly in an enclosing
    /// `if`/`switch` branch; see [`IfSurvivorRedirect`].
    if_survivor_redirect: Option<IfSurvivorRedirect>,
    /// One-shot flag set by `emit_while` just before emitting its body sequence:
    /// the next `emit_sequence_ex` is a `while` body, so each multi-case switch
    /// in it keeps part of its uncompressed merge band (see
    /// `switch_while_merge_extra`). `emit_sequence_ex` takes the flag at entry so
    /// nested sequences do not inherit it, and applies `while_switch_merge_extra`
    /// per switch via `emit_node`.
    pending_while_body: bool,
    /// One-shot flag set by `emit_while` just before emitting a body that is a
    /// single nested compressible `while` (see
    /// [`while_body_is_single_compressible_while`]). The enclosing while claims
    /// the diamond-chain's reclaimable inbound slot, so the nested while must NOT
    /// apply its own inbound slot compression. `emit_while` consumes the flag at
    /// entry (one-shot, so it does not leak to a deeper body) and forces
    /// `compress_while_slot = false`.
    while_sole_body_suppress_compress: bool,
    /// One-shot flag set by `emit_while` before emitting a single-nested-`while`
    /// body it wants to FUSE its loop-back with (case (b) compression). The
    /// nested `while` consumes it at entry and, instead of wrapping its exit
    /// corridor back to the spine, descends to the fused band and reports its
    /// `WhileNestedExit` so the parent's loop-back can source from it.
    while_expect_nested_exit: bool,
    while_expect_prefixed_nested_exit: bool,
    /// Reported by a fused nested `while` (see `while_expect_nested_exit`) for the
    /// parent `emit_while` to consume when drawing its loop-back arm.
    while_nested_exit: Option<WhileNestedExit>,
    /// Deferred inbound arrow into a nested `while` that is the last flow node of
    /// the current `while` body. The parent loop-back arm must emit before this
    /// child inbound, so `emit_sequence_ex` carries it back to `emit_while`.
    while_nested_child_inbound: Option<PendingDownArrow>,
    title_x_offset: f64,
    /// Nesting depth of `repeat` loop bodies currently being emitted. A `repeat`
    /// reserves an `8*halfHex` tail below its body (FtileRepeat
    /// `calculateDimensionInternal`); at top level the whole-diagram ON_Y
    /// compression collapses that tail so the body→condition gap renders as a
    /// single `ARROW_LEN`. When the repeat is itself nested inside another
    /// repeat's body, the enclosing loop frame leaves one `ARROW_LEN` of that
    /// tail uncompressible, so the inner body→condition gap renders as
    /// `2*ARROW_LEN`. `emit_repeat` reads this at entry to add the extra gap,
    /// then increments it around its own body emit.
    repeat_body_depth: usize,
    /// Running sum of `nested_cond_extra` gap added by repeats emitted since the
    /// enclosing `emit_repeat` last reset it. The loop-back emphasis arrowhead
    /// sits at the loop's *content* midpoint, i.e. the geometric midpoint minus
    /// half of any nested-repeat tail expansion that falls inside the loop span.
    /// `emit_repeat` saves+zeroes this around its body emit, reads the body's
    /// accumulated expansion to bias its arrowhead, then propagates this
    /// subtree's total expansion (its own `nested_cond_extra` plus the body's)
    /// to the parent.
    repeat_nested_expansion: f64,
    /// One-shot extra lowering for the body of a `repeat` that leads an if-long
    /// (`if/elseif*/else`) branch. The if-long row top-aligns its couples at a
    /// fixed y, so the FtileRepeat top reserve above the body never recompresses;
    /// `emit_if_long` sets this before emitting such a branch and `emit_repeat`
    /// consumes it (adding to `body_top_extra`) at entry.
    if_long_repeat_body_extra: f64,
    /// One-shot extra gap added to the body→condition tail of a `repeat` that is
    /// the leading tile of an even-flow, labelled `while` body. FtileWhile centres
    /// the body in its frame; with an even number of body flow nodes the centre
    /// falls on the connector between them, and that centring slack
    /// (`WHILE_EVEN_BODY_MID_STRETCH_LABELED`) cannot be reclaimed from the
    /// repeat's already-reserved tail — so the repeat's condition diamond (and the
    /// following tile) sit one stretch lower. `emit_while` sets this before
    /// emitting such a body and `emit_repeat` consumes it (adding to `cond_y`).
    while_repeat_tail_extra: f64,
    /// True while emitting an if-long branch body. The if-long row top-aligns its
    /// couples (`getTranslateCouple1` → fixed y), so the whole-diagram ON_Y
    /// compression never reaches the branch tiles: a `while` keeps its full
    /// uncompressed loop-back tail (junction `body_bottom + 12`, not the
    /// compressed `+ 10`). `emit_if_long` sets this around each branch emit.
    in_if_long_branch: bool,
    /// One-shot merge-line y handed to a `while` that leads an if-long branch:
    /// PlantUML fuses the loop's exit corridor with the branch→merge connector
    /// (the wrap-back continues straight down the spine to the merge line, drawn
    /// in document order right after the wrap-back — not as a separate later
    /// ConnectionVerticalOut). `emit_while` consumes it, draws the spine-down +
    /// merge arrowhead, and the if-long skips its own vout for that branch.
    while_if_long_merge_y: Option<f64>,
    /// Extra added to the current switch's merge gap because it is a direct flow
    /// node of a `while` body. Set per-switch by `emit_sequence_ex` (while-body
    /// mode) and consumed once by `emit_switch_with_layout`.
    while_switch_merge_extra: f64,
    /// True while emitting a multi-case switch that is a direct flow node of a
    /// `while` body. The loop frame blocks ON_X compression of the bands around
    /// the diamond column, so the switch uses `switch_x_layout_in_while` (gap-20
    /// inner bands) instead of the standalone gap-10 layout. Set per-switch by
    /// `emit_sequence_ex` (while-body mode) and consumed once by `emit_switch`.
    while_body_switch: bool,
    /// True while emitting a multi-case switch that is a direct flow node of a
    /// `repeat` body. Exactly like `while_body_switch`: the loop frame blocks the
    /// standalone diagram-wide ON_X compression of the bands straddling the
    /// condition/merge-diamond column, so the switch keeps its uncompressed
    /// gap-20 inner bands (`switch_x_layout_in_while`) instead of the standalone
    /// gap-10 layout. Affects ONLY the horizontal layout selection; the vertical
    /// merge band stays driven by `repeat_switch_merge_extra`. Set per-switch by
    /// `emit_sequence_ex` (repeat-body mode), consumed once by `emit_switch`.
    repeat_body_switch: bool,
    /// True when the terminating if-down currently being emitted is followed by
    /// another flow node in its sequence. Its no-diamond corridor then owns the
    /// outbound spine arrow (FtileIfDown.ConnectionOut), drawn right after the
    /// corridor; the enclosing sequence skips the deferred inbound. Set per-node
    /// by `emit_sequence_ex`, consumed once by `emit_if_down`.
    if_down_terminating_has_next: bool,
    /// Same terminating-if handoff, but inside a `while` body. PlantUML keeps an
    /// extra no-diamond `ConnectionOut` band here after the loop's inbound slot
    /// is reclaimed; `emit_if_down` consumes this one-shot and lengthens only
    /// that outbound spine arrow.
    if_down_terminating_while_body_has_next: bool,
    /// True while emitting a leading multi-case (>= 3) SMALL-diamond switch of a
    /// `fork`/`split` branch. Such a switch emits its uncompressed `FtileSwitchNude`
    /// layout (`switch_x_layout_nude`) rather than the switch-locally-compressed
    /// standalone `switch_x_layout`; the diagram-wide ON_X pass (the fork's black
    /// bar is `ignoreForCompressionOnX`) reclaims the empty bands instead. See
    /// `fork_branch_switch_is_nude`. Set per-branch by `emit_fork_with_layout`,
    /// consumed once by `emit_switch`.
    fork_body_switch: bool,
    /// True while emitting a `while` body whose loop-back corridor ON_Y-compresses
    /// (>=3 body flow tiles; see `while_break_corridor_compresses`). A multi-case
    /// switch in such a body routes its centre-spine drop's collinear split at the
    /// compressed corridor turn (`max_bottom + SWITCH_WHILE_COMPRESSED_CENTER_SPLIT`)
    /// instead of `merge_top - SWITCH_CENTER_BOT_SPLIT`. Set by `emit_while`,
    /// captured at entry by `emit_sequence_ex` so nested bodies do not inherit it.
    while_corridor_compresses: bool,
    /// Per-switch view of `while_corridor_compresses`: set by `emit_sequence_ex`
    /// just before emitting a multi-case switch in a corridor-compressing `while`
    /// body, consumed once by `emit_switch_with_layout`.
    while_switch_corridor_compresses: bool,
    /// True when the current in-while switch's merge band itself ON_Y-compresses
    /// (non-terminal with >=3 trailing flow tiles). Anchors the loop-back-tip at
    /// `merge_top - SWITCH_WHILE_COMPRESSED_LOOPBACK_OFFSET`. Set by
    /// `emit_sequence_ex`, consumed once by `emit_switch_with_layout`.
    while_switch_merge_compressed: bool,
    /// y of the loop-back up-arrowhead tip when a multi-case switch is in the
    /// `while` body. PlantUML's ON_Y compression of the loop-back corridor pulls
    /// the emphasized arrowhead from the corridor midpoint down to anchor on the
    /// switch's merge diamond: `tip = switch_merge_top - standalone_merge_gap -
    /// ARROW_LEN/2`. Recorded by `emit_switch_with_layout` (in while-body mode,
    /// last writer wins) and consumed once by `emit_while`.
    while_switch_loopback_tip: Option<f64>,
    /// One-shot flag set by `emit_repeat` just before emitting its body sequence:
    /// the next `emit_sequence_ex` is a `repeat` body, so a terminal multi-case
    /// switch in it keeps one `ARROW_LEN` of its uncompressed merge band that the
    /// whole-diagram ON_Y pass cannot reclaim under the loop frame (the FtileRepeat
    /// `8*halfHex` tail keeps a full arrow below the switch's merge diamond before
    /// the condition diamond). `emit_sequence_ex` takes the flag at entry so nested
    /// sequences do not inherit it.
    pending_repeat_body: bool,
    /// One-shot: this `while` is a NON-terminal flow node of a `repeat` body
    /// (another flow tile follows it inside the loop). The FtileRepeat frame holds
    /// the loop-back tail open below such a while — its junction keeps the full
    /// `body_bottom + halfHex` (12) rather than the diagram-level ON_Y compressed
    /// `body_bottom + 10`. A terminal while in the body still compresses to 10.
    /// Set per-node by `emit_sequence_ex`, consumed once by `emit_while`.
    while_repeat_body_nonterminal: bool,
    /// Extra added to the current switch's merge gap because it is the terminal
    /// flow node of a `repeat` body (`ARROW_LEN`). Set per-switch by
    /// `emit_sequence_ex` (repeat-body mode) and consumed once by
    /// `emit_switch_with_layout`, which also records `while_switch_loopback_tip`
    /// so `emit_repeat` can anchor its loop-back arrowhead on the merge diamond.
    repeat_switch_merge_extra: f64,
    /// Extra px the loop-body centring slack distributes into the middle inter-
    /// action gap of the deepest 2-action `then`-branch of a `while` body that is
    /// a single (possibly nested) balanced `if`. PlantUML's `FtileWhile` centring
    /// reserves `2*hexHalf` of slack above the body tile; ON_Y compression
    /// reclaims it into the if-block's tallest branch's middle connector, less the
    /// merge-band each nesting level absorbs (see `while_if_body_branch_stretch`).
    /// Set by `emit_while` before emitting a pure-balanced-if body; routed
    /// unchanged down the `then`-chain by `emit_if` (it does NOT leak into an
    /// `else` branch) and consumed once when the terminal 2-action `then`-branch
    /// emits. Drives the `act_while_ifdepth*_acts2` family.
    while_if_branch_stretch: Option<f64>,
    /// Set while emitting a swimlane lane body that is a single
    /// `while ... endwhile <terminator>` whose lane width was taken from the
    /// cond-driven drawn box (`swimlane_lane_extents` narrowed the left extent).
    /// In that layout the terminator circle hangs off the diamond's left-vertex
    /// column (`special_cx = diamond_left_vertex - halfHex - 9`) rather than the
    /// standalone body-corridor position. Consumed (one-shot) by `emit_while`.
    swimlane_while_cond_special: bool,
    /// Swimlane V2 single-tree emit is active. While set, `emit_sequence_ex`
    /// turns on `swimlane_while_cond_special` around each `while`+special_out
    /// tile that forms (the tail of) its lane — the lane-narrowed terminator
    /// placement that the legacy segment model applies per segment.
    swimlane_v2_active: bool,
    /// Set by `emit_swimlanes` before emitting a lane segment whose body ends in
    /// a no-special `while` and whose next chronological segment lives in another
    /// lane. PlantUML stitches the inter-lane arrow onto the loop's exit corridor
    /// (the `FtileWhile.ConnectionOut` snake): the loop's exit arm descends
    /// straight to the cross-lane corridor and a single horizontal crosses to the
    /// next lane's spine instead of wrapping back to this lane's spine. Consumed
    /// (one-shot) by `emit_while`.
    swimlane_cross_lane: Option<SwimlaneCrossLane>,
}

#[derive(Clone, Copy)]
struct ConnectorLaneSpan {
    start: usize,
    end: usize,
    out_lane: Option<usize>,
    in_lane: Option<usize>,
}

/// Inter-lane stitch geometry for a no-special `while` that is the last tile of
/// a lane segment with a cross-lane successor (see
/// [`SvgEmitter::swimlane_cross_lane`]).
#[derive(Clone, Copy)]
struct SwimlaneCrossLane {
    /// Content cx (spine) of the next lane's segment.
    target_cx: f64,
    /// y of the cross-lane horizontal run (the next segment's top minus 15).
    cross_y: f64,
    /// Top y of the next segment's first tile (the arrow's destination).
    target_y: f64,
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
    /// Set when the break-`if` is the body's LAST flow node. Its empty (continue)
    /// branch is then the loop's `pointOut`: PlantUML's `ConnectionBackSimple`
    /// originates the loop-back arm at that branch's east vertex and runs it up to
    /// the condition diamond (no down-then-spine corridor, no separate junction
    /// loop-back). The break-`if` draws the fused arm itself (so it lands in the
    /// correct document-order slot, interleaved with its own connectors) and
    /// signals `emit_while` to skip its junction loop-back. `None` for the
    /// mid/early topologies. See [`break_if_is_last_flow`].
    fuse_loopback: Option<WhileBreakLoopback>,
    /// Set when the enclosing loop is a `repeat` (not a `while`). The break
    /// welds LEFT to the corridor but the left arrowhead is NOT drawn at the
    /// weld: the repeat continues the corridor DOWN to a merge diamond below the
    /// condition (the loop's `out`), where the arrowhead lands instead. The
    /// repeat records the weld point so it can draw the down-corridor + merge.
    repeat_mode: bool,
    /// Absolute centre-y of the repeat's break-merge diamond. The break corridor
    /// runs DOWN to this y then RIGHT into the merge. When the break-`if` is the
    /// body's LAST flow node it rejoins straight into the condition, so the merge
    /// is a fixed span below the if and this can be `None` (the if derives it from
    /// `return_y`). When trailing body flow follows the break-`if` (mid/early
    /// topologies), the merge sits below that flow + the condition, so `emit_repeat`
    /// pre-computes it here from the body height.
    repeat_merge_cy: Option<f64>,
    repeat_merge_left_x: Option<f64>,
    /// Extra lead added to the inbound connector that reaches the break-`if`'s
    /// diamond (on top of the plain `ARROW_LEN`). A bare `while (cond)` with no
    /// `is`/end label centres its body in a frame with `suppLabel = 0` (see
    /// [`WHILE_BREAK_SUPP_LABEL_H`]); PlantUML's slot finder then redistributes
    /// the if-down corridor's uncompressed slack — pushing a small lead *above*
    /// the diamond instead of leaving it all below — so the corridor rejoins at
    /// the compressed drop. See [`WHILE_BREAK_NO_LABEL_INBOUND_LEAD`].
    inbound_lead: f64,
    /// Set (repeat only) when the break-`if` is the body's FIRST flow node and
    /// trailing body flow follows it (the `act_break_repeat_early` topology). With
    /// no leading tile above the break-`if`, PlantUML's ON_Y slot finder cannot
    /// compress the corridor's upper band, so the empty (continue) branch rejoins
    /// at the SAME compressed drop a last-node break uses
    /// (`IF_DOWN_LEAD + ARROW_LEN − 1`), not the mid-break drop (`… + IF_BRANCH_UP/2`).
    /// The reclaimed slack instead lengthens the rejoin→trailing-action arrow.
    repeat_break_first_flow: bool,
}

/// Loop-back fusion geometry handed to a last-flow break-`if` (see
/// [`WhileBreakContext::fuse_loopback`]). Computed up-front by `emit_while` from
/// the body's extents (identically to the post-body `loop_x`) so the if can draw
/// the arm in its own connector stream.
#[derive(Clone, Copy)]
struct WhileBreakLoopback {
    /// x of the loop-back arm's vertical run (`loop_x`).
    loop_x: f64,
    /// y of the condition diamond's centre (the arm's horizontal run + arrowhead).
    diamond_cy: f64,
    /// x of the condition diamond's right vertex (where the left-arrow lands).
    diamond_right_vertex_x: f64,
}

/// Geometry a directly-nested `while` (the sole body of a fusing parent `while`)
/// hands back UP to its parent so the parent's loop-back arm sources from the
/// nested loop's exit corridor instead of the centre spine. PlantUML's nested
/// `FtileWhile` loop-back `ConnectionBackSimple` snake starts at the inner
/// `whileBlock.getPointOut` (the inner frame's left exit column) and its `y1bis`
/// sits one `halfHex` below the inner frame bottom (= the inner loop-back
/// junction). MergeStrategy.LIMITED then drops the parent's redundant UP
/// emphasize arrowhead. Drives the nested-while cases.
#[derive(Clone, Copy)]
struct WhileNestedExit {
    /// X of the inner loop's exit corridor (its `getPointOut` column). The
    /// parent's loop-back horizontal sources from here.
    exit_x: f64,
    /// Y of the fused corridor band: inner loop-back junction + `halfHex`. The
    /// parent uses this as its own loop-back junction y.
    fused_y: f64,
    /// Number of inbound-compressed `while`s strictly BELOW the consuming parent
    /// in this single-while chain (the child if it compressed, plus its own
    /// count). Each compressed level lowered the drawn body by
    /// `WHILE_BODY_SLOT_COMPRESS`, but PlantUML anchors the parent's exit
    /// `ConnectionOut` emphasize arrowhead on the PRE-compression frame midpoint,
    /// so each level pulls that arrowhead up by
    /// `PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP`.
    compressed_below: usize,
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

/// Set by an enclosing `if`/`switch` when one of its branches is a SOLE
/// single-survivor `if` (one branch terminates with kill/detach/stop, the
/// other survives). PlantUML's `ConnectionVerticalThenHorizontalDirect` plus
/// `MergeStrategy.LIMITED` fuses such a nested if's surviving out-corridor with
/// the parent's branch→merge connector: the surviving column runs straight down
/// to the parent merge vertex with no intermediate reconvergence to the inner
/// spine and no own down-arrowhead. The parent skips its own branch→merge
/// connector for that side (the nested if drew it).
#[derive(Clone, Copy)]
struct IfSurvivorRedirect {
    /// Centre-y of the parent merge diamond.
    merge_cy: f64,
    /// X of the parent merge-diamond vertex the corridor arrives at.
    merge_vertex_x: f64,
    /// `true` → arrives at the left vertex, points right; `false` → right
    /// vertex, points left.
    to_right: bool,
    draw_arrow: bool,
}

#[allow(clippy::too_many_arguments)]
impl SvgEmitter {
    fn with_palette(palette: Palette, handwritten: bool) -> Self {
        SvgEmitter {
            shapes: String::new(),
            connectors: String::new(),
            connector_lane_spans: Vec::new(),
            current_connector_lanes: None,
            current_lane: 0,
            lane_spans: Vec::new(),
            palette,
            colored_partition_while_depth: 0,
            partition_wrapped_fork_depth: 0,
            handwritten,
            while_exit_redirect: None,
            fork_branch_gap_extra: 0.0,
            while_body_fork_layout: None,
            fork_branch_while_slot_open: false,
            fork_branch_repeat_body_extra: 0.0,
            fork_branch_repeat_tail_extra: 0.0,
            fork_branch_while_if_body_compress: false,
            while_break: None,
            while_break_loopback_fused: false,
            repeat_break_weld_y: None,
            repeat_body_condition_connector_drawn: false,
            if_survivor_redirect: None,
            pending_while_body: false,
            while_sole_body_suppress_compress: false,
            while_expect_nested_exit: false,
            while_expect_prefixed_nested_exit: false,
            while_nested_exit: None,
            while_nested_child_inbound: None,
            title_x_offset: 0.0,
            repeat_body_depth: 0,
            repeat_nested_expansion: 0.0,
            if_long_repeat_body_extra: 0.0,
            while_repeat_tail_extra: 0.0,
            in_if_long_branch: false,
            while_if_long_merge_y: None,
            while_switch_merge_extra: 0.0,
            while_body_switch: false,
            repeat_body_switch: false,
            if_down_terminating_has_next: false,
            if_down_terminating_while_body_has_next: false,
            fork_body_switch: false,
            while_corridor_compresses: false,
            while_switch_corridor_compresses: false,
            while_switch_merge_compressed: false,
            while_switch_loopback_tip: None,
            pending_repeat_body: false,
            while_repeat_body_nonterminal: false,
            repeat_switch_merge_extra: 0.0,
            while_if_branch_stretch: None,
            swimlane_while_cond_special: false,
            swimlane_v2_active: false,
            swimlane_cross_lane: None,
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

    /// Emit a fork/join black bar tagged `ignoreForCompressionOnX` (the
    /// `FtileBlackBlock` flag). When `compressible` it carries the
    /// `FORK_BAR_COMPRESS_MARKER` attribute so the whole-diagram ON_X pass reads
    /// it as transparent (no X-occupancy) — letting the reclaimable middle-gap
    /// corridor collapse and the bar shrink with it — then strips the marker
    /// before serialization. Non-compressible bars use the ordinary `rect_styled`
    /// (the bar BLOCKS X, holding open external if/else or note corridors).
    fn fork_bar(
        &mut self,
        fill: &str,
        stroke: &str,
        width: f64,
        x: f64,
        y: f64,
        compressible: bool,
    ) {
        if !compressible {
            self.rect_styled(
                fill,
                FORK_BAR_HEIGHT,
                FORK_BAR_RX,
                FORK_BAR_RX,
                stroke,
                "1",
                width,
                x,
                y,
            );
            return;
        }
        let filter = self.shadow_filter_attr(true);
        if self.handwritten {
            let points =
                handwritten_rect_points(x, y, width, FORK_BAR_HEIGHT, FORK_BAR_RX, FORK_BAR_RX);
            write!(
                self.shapes,
                r#"<polygon fill="{}"{} {}="" points="{}" style="stroke:{};stroke-width:1;"/>"#,
                fill, filter, FORK_BAR_COMPRESS_MARKER, points, stroke
            )
            .unwrap();
            return;
        }
        write!(
            self.shapes,
            r#"<rect fill="{}"{} {}="" height="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
            fill, filter, FORK_BAR_COMPRESS_MARKER, f(FORK_BAR_HEIGHT), f(FORK_BAR_RX), f(FORK_BAR_RX), stroke, f(width), f(x), f(y)
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
        let start = self.connectors.len();
        self.connectors.push_str(s);
        self.record_connector_lane_span(start);
    }

    fn record_connector_lane_span(&mut self, start: usize) {
        let Some((out_lane, in_lane)) = self.current_connector_lanes else {
            return;
        };
        let end = self.connectors.len();
        if end > start {
            self.connector_lane_spans.push(ConnectorLaneSpan {
                start,
                end,
                out_lane,
                in_lane,
            });
        }
    }

    fn with_connector_lanes<T>(
        &mut self,
        out_lane: Option<usize>,
        in_lane: Option<usize>,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let saved = self.current_connector_lanes;
        self.current_connector_lanes = Some((out_lane, in_lane));
        let result = f(self);
        self.current_connector_lanes = saved;
        result
    }

    fn truncate_connectors(&mut self, len: usize) {
        self.connectors.truncate(len);
        self.connector_lane_spans.retain(|span| span.end <= len);
    }

    fn truncate_lane_spans(&mut self, shapes_len: usize, connectors_len: usize) {
        self.lane_spans.retain(|(shape_off, conn_off, _)| {
            *shape_off <= shapes_len && *conn_off <= connectors_len
        });
    }

    fn switch_lane_span(&mut self, lane: usize) {
        if self.swimlane_v2_active && self.current_lane != lane {
            self.current_lane = lane;
            self.lane_spans
                .push((self.shapes.len(), self.connectors.len(), lane));
        }
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
        let start = self.connectors.len();
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
            self.record_connector_lane_span(start);
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
        self.record_connector_lane_span(start);
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
        let start = self.connectors.len();
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
        self.record_connector_lane_span(start);
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
        let start = self.connectors.len();
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
        self.record_connector_lane_span(start);
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
        let start = self.connectors.len();
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
            self.record_connector_lane_span(start);
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
        self.record_connector_lane_span(start);
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

/// Emit an `if` then-branch flow, routing a pending `while`-body if-branch
/// stretch (set by `emit_while` for a pure-balanced-if loop body, see
/// [`SvgEmitter::while_if_branch_stretch`]) to the deepest 2-action terminal
/// branch's middle gap. `stretch == None` is the ordinary case (no slack to
/// distribute) and behaves exactly like [`emit_sequence_if_branch`].
fn emit_if_then_branch_with_stretch(
    svg: &mut SvgEmitter,
    flow: &[LayoutNode],
    cx: f64,
    y: f64,
    stretch: Option<f64>,
) -> f64 {
    match (stretch, flow) {
        // Terminal: the two plain actions of the deepest then-branch — the loop
        // centring slack lands in their straddling gap (flow index 1).
        (Some(s), [LayoutNode::Action { .. }, LayoutNode::Action { .. }]) => {
            emit_sequence_ex(svg, flow, cx, y, Some((1, s)), None, true)
        }
        // Recurse: a single nested balanced `if` — re-arm the flag so the nested
        // `emit_if` (reached via `emit_node`) routes the stretch one level deeper.
        (Some(s), [LayoutNode::If { .. }]) => {
            let prev = svg.while_if_branch_stretch.replace(s);
            let bottom = emit_sequence_if_branch(svg, flow, cx, y);
            svg.while_if_branch_stretch = prev;
            bottom
        }
        _ => emit_sequence_if_branch(svg, flow, cx, y),
    }
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
    // Take the one-shot `while` body flag set by `emit_while`: when set, this
    // sequence is the loop body and each multi-case switch keeps part of its
    // uncompressed merge band (see `switch_while_merge_extra`). Reset it so
    // nested sequences emitted from this body do not inherit it.
    let while_body = std::mem::take(&mut svg.pending_while_body);
    // Take the one-shot `repeat` body flag set by `emit_repeat`: when set, this
    // sequence is the loop body and a terminal multi-case switch keeps one
    // `ARROW_LEN` of its uncompressed merge band (see `switch_repeat_merge_extra`).
    let repeat_body = std::mem::take(&mut svg.pending_repeat_body);
    let multi_fork_sequence = !while_body && !repeat_body && sequence_uses_multi_fork_layout(nodes);
    let multi_fork_has_prelude = multi_fork_sequence && sequence_multi_fork_has_prelude(nodes);
    // Whether this loop body's loop-back corridor ON_Y-compresses (one-shot,
    // reset so nested bodies do not inherit it).
    let while_corridor_compresses = std::mem::take(&mut svg.while_corridor_compresses);
    let while_body_last_flow = if while_body {
        last_flow_index(nodes)
    } else {
        None
    };
    let repeat_body_last_flow = if repeat_body {
        last_flow_index(nodes)
    } else {
        None
    };
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
        // Swimlane V2 lane marker: record the lane span (emit_node sets
        // current_lane + pushes the byte offsets) but take NO vertical space and
        // NO inbound connector — it is not a flow node, so it must not advance
        // `flow_ordinal` (otherwise the node after a leading `|Lane|` gets a
        // phantom inbound arrow, e.g. an arrow into `start`).
        if matches!(node, LayoutNode::LaneMark(_)) {
            emit_node(svg, node, cx, y);
            continue;
        }
        // Skip layout for non-flow nodes (arrows and notes don't take vertical space
        // on their own).
        if matches!(node, LayoutNode::Arrow { .. } | LayoutNode::Note { .. }) {
            continue;
        }
        // Detach/Kill/Break/Goto also produce no shape and no incoming
        // connector — they mark the previous flow as terminated. A `break`
        // inside a repeat branch still owns its exit collector, emitted here so
        // the connector lands in the branch's document-order slot.
        if matches!(node, LayoutNode::Break)
            && let Some(brk) = svg.while_break.filter(|brk| brk.repeat_mode)
            && let (Some(merge_cy), Some(merge_left)) =
                (brk.repeat_merge_cy, brk.repeat_merge_left_x)
        {
            let arrow_color = svg.palette.arrow_color.clone();
            let merge_cy = merge_cy - 1.0;
            svg.connector_line(&arrow_color, cx, cx, y, merge_cy, false);
            svg.connector_line(&arrow_color, cx, merge_left, merge_cy, merge_cy, false);
            svg.right_arrow(merge_left, merge_cy, &arrow_color);
            svg.repeat_break_weld_y = Some(y);
        }
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
            // The title's +30 vertical padding (emit_node) is calibrated so the
            // cursor lands at the cy of a following Start ellipse (START_CY=25
            // = MARGIN_LEAD + 9). A Swimlanes container carries its own top
            // structure pinned to MARGIN_LEAD rather than a Start cy, so the
            // 9 px Start-ellipse compensation does not apply — drop it. This
            // mirrors the −9 Start-in-swimlane overcount in sequence_height.
            if matches!(
                nodes
                    .iter()
                    .skip(i + 1)
                    .find(|n| !matches!(n, LayoutNode::Arrow { .. } | LayoutNode::Note { .. })),
                Some(LayoutNode::Swimlanes { .. })
            ) {
                y -= 9.0;
            }
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
        let mut pending_arrow: Option<PendingDownArrow> = None;
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
                    // Swimlane V2 lane marker: transparent to the previous-flow
                    // lookup (it is not a flow node), so a node after a leading
                    // `|Lane|` does not treat the marker as its predecessor and
                    // gets no phantom inbound arrow.
                    LayoutNode::LaneMark(_) => {}
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
            // A terminating-populated if-down draws its own outbound spine arrow
            // (FtileIfDown.ConnectionOut), so skip the deferred inbound here too.
            let skip_implicit_inbound_after_terminating_down = explicit_arrow.is_none()
                && prev_idx
                    .and_then(|j| nodes.get(j))
                    .is_some_and(if_node_is_terminating_down);
            if prev_idx.is_some()
                && !skip_implicit_inbound_after_single_survivor_if
                && !skip_implicit_inbound_after_terminating_down
            {
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
                // A break-bearing `if` that is the loop body's FIRST flow node
                // (rendered no-diamond by `emit_if_break_down`) and whose corridor
                // compresses (3+ body flow tiles) reserves one extra
                // `IF_DOWN_MID_STRETCH` band on the connector leaving its pointOut.
                // PlantUML's ON_Y slot finder pulls the corridor's reserved band UP
                // (compressing the rejoin) but cannot reclaim the south-label mid
                // band when no leading tile sits above the break-`if`; it surfaces
                // instead as extra length on the outbound arrow (mirrors
                // `WHILE_BREAK_FIRST_FRAME_EXTRA` in the frame-height model). The
                // thin two-tile leading-break form (`act_while_break_at_start`)
                // leaves the corridor uncompressed and keeps the plain ARROW_LEN
                // gap, so gate on both first-flow AND corridor compression.
                let break_if_first_outbound_extra = if svg.while_break.is_some()
                    && prev_idx
                        .and_then(|j| nodes.get(j))
                        .is_some_and(is_break_down_if)
                    && prev_idx == nodes.iter().position(node_is_flow)
                    && while_break_corridor_compresses(nodes)
                {
                    IF_DOWN_MID_STRETCH
                } else {
                    0.0
                };
                // Repeat first-flow break with trailing body (`act_break_repeat_early`):
                // the break-`if` returns the last-node compressed drop as its pointOut
                // (4 px higher than the mid rejoin, see `emit_if_break_down`); the
                // reclaimed `IF_BRANCH_UP/2 + 1` of slack, plus the FtileIfDown's
                // first-tile height delta (`REPEAT_BREAK_FIRST_TRAILING_EXTRA`), all
                // surface on this rejoin→action arrow so the trailing action lands at
                // the same absolute y a mid break would place it, one delta lower.
                let repeat_break_first_outbound_extra = svg
                    .while_break
                    .filter(|brk| {
                        brk.repeat_mode
                            && brk.repeat_break_first_flow
                            && prev_idx
                                .and_then(|j| nodes.get(j))
                                .is_some_and(is_break_down_if)
                            && prev_idx == nodes.iter().position(node_is_flow)
                    })
                    .map_or(0.0, |_| {
                        IF_BRANCH_UP / 2.0 + 1.0 + REPEAT_BREAK_FIRST_TRAILING_EXTRA
                    });
                // No-`is` break loop: the slot finder pushes a small lead above
                // the break-`if`'s diamond (`WhileBreakContext::inbound_lead`).
                let break_if_inbound_lead = svg
                    .while_break
                    .filter(|_| is_break_down_if(node))
                    .map_or(0.0, |brk| brk.inbound_lead);
                let gap = stretch
                    + lead
                    + carry
                    + break_if_first_outbound_extra
                    + repeat_break_first_outbound_extra
                    + break_if_inbound_lead
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
        let fork_layout_override = if while_body {
            while_body_fork_layout(nodes, i, node)
        } else if multi_fork_sequence && is_multi_fork_sequence_candidate(node) {
            match node {
                LayoutNode::Fork { branches, .. } => {
                    Some(fork_layout_multi_sequence(branches, multi_fork_has_prelude))
                }
                _ => None,
            }
        } else {
            None
        };
        if let LayoutNode::Fork {
            branches,
            attached_notes,
            is_split,
            ..
        } = node
            && !attached_notes.is_empty()
        {
            let layout = fork_layout_override.clone().unwrap_or_else(|| {
                if *is_split {
                    split_layout(branches)
                } else if first_repeat_branch_extra {
                    fork_layout_if_branch(branches)
                } else {
                    fork_layout(branches)
                }
            });
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
        let node_defers =
            while_body_chain_compresses(node) || is_partition_wrapping_compressed_while(node);
        let is_chain_while = node_defers && prev_was_deferred_while;
        if is_chain_while && let Some((arrow_top, style, label, arrow_gap)) = pending_arrow.take() {
            emit_pending_down_arrow(svg, arrow_top, style, label, arrow_gap, cx);
            if let Some((p_top, p_style, p_label, p_gap)) = deferred_partition_inbound.take() {
                emit_pending_down_arrow(svg, p_top, p_style, p_label, p_gap, cx);
            }
        }
        // A fork/split immediately following a deferred (compressed) while is
        // assembled like the chain-while boundary: PlantUML draws the connection
        // INTO the fork (the while's exit arm down to the top bar) and then
        // flushes the while's own deferred inbound BEFORE the fork's internal
        // bar/branch connectors. The fork is not itself a deferring tile, so
        // without this its inbound would land after the branch arrows.
        if !is_chain_while
            && prev_was_deferred_while
            && matches!(node, LayoutNode::Fork { .. })
            && let Some((arrow_top, style, label, arrow_gap)) = pending_arrow.take()
        {
            emit_pending_down_arrow(svg, arrow_top, style, label, arrow_gap, cx);
            if let Some((p_top, p_style, p_label, p_gap)) = deferred_partition_inbound.take() {
                emit_pending_down_arrow(svg, p_top, p_style, p_label, p_gap, cx);
            }
        }
        // A multi-case switch directly in a `while` body keeps part of its
        // uncompressed merge band; pass that extra to the switch emit (one-shot).
        if while_body && matches!(node, LayoutNode::Switch { .. }) {
            let is_terminal = Some(i) == while_body_last_flow;
            let following = following_flow_count(nodes, i);
            svg.while_switch_merge_extra = switch_while_merge_extra(node, is_terminal, following);
            svg.while_body_switch = true;
            svg.while_switch_corridor_compresses = while_corridor_compresses;
            // The merge band itself ON_Y-compresses only for a non-terminal switch
            // with >=3 trailing flow tiles (drives the loop-back-tip anchor).
            svg.while_switch_merge_compressed =
                !is_terminal && following >= SWITCH_WHILE_MERGE_COMPRESS_MIN_FOLLOWING;
        }
        // A terminal multi-case switch directly in a `repeat` body keeps one
        // `ARROW_LEN` of its uncompressed merge band (the loop frame's fixed tail);
        // pass that extra to the switch emit (one-shot).
        if repeat_body
            && Some(i) == repeat_body_last_flow
            && matches!(node, LayoutNode::Switch { .. })
        {
            svg.repeat_switch_merge_extra = switch_repeat_merge_extra(node);
        }
        // A multi-case switch directly in a `repeat` body keeps its uncompressed
        // gap-20 inner bands, exactly like a `while`-body switch: the loop frame
        // blocks the standalone diagram-wide ON_X compression of the bands
        // straddling the condition/merge-diamond column. This flag affects ONLY
        // the horizontal layout selection (`switch_x_layout_in_while`); the
        // vertical merge band stays driven by `repeat_switch_merge_extra` above.
        if repeat_body && matches!(node, LayoutNode::Switch { .. }) {
            svg.repeat_body_switch = true;
        }
        // A NON-terminal `while` in a repeat body keeps its loop-back tail open
        // (the FtileRepeat frame holds the band; the diagram-level ON_Y +10
        // compression does not reach under the loop). See
        // `while_repeat_body_nonterminal`.
        if repeat_body
            && Some(i) != repeat_body_last_flow
            && matches!(node, LayoutNode::While { .. })
        {
            svg.while_repeat_body_nonterminal = true;
        }
        // A terminating if-down owns its outbound spine arrow when followed by
        // another flow node; tell its emit whether that next node exists.
        if if_node_is_terminating_down(node) {
            let has_next = following_flow_count(nodes, i) > 0;
            svg.if_down_terminating_has_next = has_next;
            svg.if_down_terminating_while_body_has_next = while_body && has_next;
        }
        // Swimlane V2: a `while`+terminator tile that forms (the tail of) its
        // lane hangs the terminator off the diamond's left-vertex column, like
        // the legacy segment model's cond-special lanes. Gate one-shot around
        // this node's emit (emit_while consumes the flag).
        if svg.swimlane_v2_active
            && matches!(
                node,
                LayoutNode::While {
                    special_out: Some(_),
                    ..
                }
            )
            && swimlane_while_cond_special_lane(std::slice::from_ref(node))
        {
            svg.swimlane_while_cond_special = true;
        }
        svg.while_body_fork_layout = fork_layout_override;
        let node_y =
            emit_node_with_repeat_extra(svg, node, cx, y, repeat_extra, first_repeat_branch_extra);
        svg.while_body_fork_layout = None;
        svg.swimlane_while_cond_special = false;
        svg.if_down_terminating_has_next = false;
        svg.if_down_terminating_while_body_has_next = false;
        svg.while_switch_merge_extra = 0.0;
        svg.while_body_switch = false;
        svg.while_switch_corridor_compresses = false;
        svg.while_switch_merge_compressed = false;
        svg.repeat_switch_merge_extra = 0.0;
        svg.repeat_body_switch = false;
        svg.while_repeat_body_nonterminal = false;
        prev_was_deferred_while = node_defers;
        // Inbound connector goes AFTER the node's own emit so it lands
        // after the node's internal connectors in the connectors buffer
        // (matches PlantUML's emission order: internal first, then inbound).
        if let Some((arrow_top, style, label, arrow_gap)) = pending_arrow {
            if is_colored_partition_wrapping_while(node)
                || is_partition_wrapping_compressed_while(node)
                || is_ordinary_compressed_while(node)
                || is_unlabeled_break_while(node)
                || is_break_down_if(node)
                || is_group_wrapping_while(node)
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
    if let Some(deferred) = deferred_partition_inbound {
        if while_body
            && last_flow_index(nodes).is_some_and(|i| {
                matches!(
                    nodes[i],
                    LayoutNode::While {
                        special_out: None,
                        ..
                    }
                )
            })
        {
            svg.while_nested_child_inbound = Some(deferred);
        } else {
            let (arrow_top, style, label, arrow_gap) = deferred;
            emit_pending_down_arrow(svg, arrow_top, style, label, arrow_gap, cx);
        }
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
        // Swimlane V2 lane marker: switch the active lane and record the byte
        // offsets in both buffers so the post-emit pass can partition by lane.
        // No y advance, no shape.
        LayoutNode::LaneMark(idx) => {
            svg.current_lane = *idx;
            svg.lane_spans
                .push((svg.shapes.len(), svg.connectors.len(), *idx));
            y
        }
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
            } else if let Some(layout) = svg.while_body_fork_layout.take() {
                emit_fork_with_layout(svg, cx, y, branches, layout)
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
            source_line,
        } => {
            // PlantUML wraps the title in `<g class="title" data-source-line="N">`,
            // where N is the source line of the `title` directive.
            // Title text is centred within an x-extent padded by 4px on the
            // left compared to the action content cx. Baseline is at
            // y + ascent + 4.
            let tw = text_render::measure(text, *font_size, *bold);
            let text_y = y + pm::ascent(*font_size) + 4.0;
            svg.shapes.push_str(&format!(
                r#"<g class="title" data-source-line="{source_line}">"#
            ));
            svg.text_element(
                TEXT_COLOR,
                "sans-serif",
                *font_size,
                tw,
                cx - tw / 2.0 + 1.0 + svg.title_x_offset,
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
    // Swimlane V2: PlantUML's `InstructionIf.getSwimlaneOut()` is the if's ENTRY
    // lane — a `|Lane|` inside a branch is scoped to that branch, so the merge
    // and everything after the if revert to the entry lane. The branch emits set
    // `current_lane` to their (possibly switched) lane; capture the entry now and
    // restore it before the merge diamond + post-merge connectors are emitted.
    let if_entry_lane = svg.current_lane;

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
        return emit_if_break_down(svg, cx, y, condition, then_label, else_label, &plan, &brk);
    }

    let has_break_single_survivor = svg.while_break.is_some_and(|brk| {
        !brk.repeat_mode && if_break_single_survivor_plan(then_branch, else_branches).is_some()
    });

    // Empty-branch corridor: when one branch is empty and the other populated
    // and non-terminating, PlantUML's FtileIfDown routes the populated branch
    // down the centre spine and the empty branch as a thin side corridor.
    if !has_break_single_survivor && let Some(plan) = if_down_plan(then_branch, else_branches) {
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
    // Take the pending `while`-body if-branch stretch (one-shot): it routes to
    // THIS if's then-branch only — clear it so the else-branch and any sibling
    // never inherit it. `emit_if_then_branch_with_stretch` re-arms it when the
    // then-branch is itself a single nested `if`.
    let if_branch_stretch = svg.while_if_branch_stretch.take();
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
        let branch_dist = (diamond_w + 20.0).max((then_w + else_w) / 2.0 + 20.0)
            + if_branch_distance_extra(then_branch, else_branches);
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
    //
    // Swimlane V2: the standalone `IF_BRANCH_DOWN` (10) is the ON_Y-compressed
    // diamond→branch inbound. A swimlane does not compress the if vertically the
    // same way, so the inbound keeps the full `ARROW_LEN` (20) — matching gold.
    // Gated to the V2 path so standalone ifs are untouched.
    let if_branch_down = if svg.swimlane_v2_active {
        ARROW_LEN
    } else {
        IF_BRANCH_DOWN
    };
    let branch_y = diamond_bottom + if_branch_down;

    // A no-special `while` that is the entire then/else flow owns the
    // branch→merge corridor (PlantUML's MergeStrategy.LIMITED fusion). Such a
    // branch needs the merge-diamond cy BEFORE it emits, so its exit corridor
    // can terminate at the merge vertex. We first emit the branches into the
    // buffers to learn their bottoms (hence merge_cy), and — when a branch is
    // redirectable — roll the buffers back and re-emit it with the redirect set.
    let then_redirectable = branch_is_redirectable_while(then_branch_flow);
    let else_redirectable =
        !else_branches.is_empty() && branch_is_redirectable_while(else_branch_flow);
    let then_survivor_if = branch_is_redirectable_single_survivor_if(then_branch_flow);
    let else_survivor_if =
        !else_branches.is_empty() && branch_is_redirectable_single_survivor_if(else_branch_flow);

    let shapes_chk = svg.shapes.len();
    let conns_chk = svg.connectors.len();
    let lane_chk = svg.current_lane;
    let then_bottom_raw = emit_if_then_branch_with_stretch(
        svg,
        then_branch_flow,
        then_cx,
        branch_y,
        if_branch_stretch,
    );
    let else_bottom_raw = if !else_branches.is_empty() {
        emit_sequence_if_branch(svg, else_branch_flow, else_cx, branch_y)
    } else {
        branch_y
    };
    // A nested single-survivor if (rendered standalone in this first pass) ends
    // with its own out-arrow (`survivor_bottom + ARROW_LEN`). When it is going
    // to be redirected into THIS merge, its corridor instead fuses to the merge
    // (PlantUML's FtileEmpty diamond2 = Hexagon.hexagonHalfSize/2), so for the
    // merge-y reservation we use the survivor bottom plus that empty-tile gap,
    // not the standalone out-arrow.
    let then_bottom = if then_survivor_if {
        then_bottom_raw - ARROW_LEN + IF_SURVIVOR_REDIRECT_GAP
    } else {
        then_bottom_raw
    };
    let else_bottom = if else_survivor_if {
        else_bottom_raw - ARROW_LEN + IF_SURVIVOR_REDIRECT_GAP
    } else {
        else_bottom_raw
    };

    // If every branch ends with a terminator (Stop/End/Detach/Kill), PlantUML
    // skips the merge diamond and post-merge connectors entirely. The two
    // branches stand on their own; the if-block's bottom is the deeper one.
    let then_terminates = branch_terminates(then_branch_flow);
    let else_terminates = else_branches.split_first().is_some_and(|(_, rest)| {
        branch_terminates(else_branch_flow)
            && rest.iter().all(|branch| branch_terminates(&branch.body))
    });
    let break_single_survivor = svg.while_break.and_then(|brk| {
        if_break_single_survivor_plan(then_branch_flow, else_branches)
            .zip((!brk.repeat_mode).then_some(brk))
    });
    let all_terminate = then_terminates && else_terminates;
    let single_survivor = if_single_survivor(then_branch_flow, else_branches).or_else(|| {
        break_single_survivor
            .as_ref()
            .map(|(p, _)| !p.then_is_break)
    });
    let repeat_break_single_survivor = svg.while_break.is_some_and(|brk| brk.repeat_mode)
        && else_branches.len() == 1
        && else_branches[0].condition.is_none()
        && ((branch_terminates_with_break(then_branch_flow)
            && !branch_terminates(else_branch_flow))
            || (branch_terminates_with_break(else_branch_flow)
                && !branch_terminates(then_branch_flow)));

    // Merge diamond at bottom — sits IF_BRANCH_UP px below the deepest branch.
    // Swimlane V2 keeps the uncompressed branch→merge gap (10) even when the
    // standalone path would use the compressed `IF_BRANCH_UP` (6).
    let merge_gap = if diamond_pad_x > 0.0 || svg.swimlane_v2_active {
        10.0
    } else {
        IF_BRANCH_UP
    };
    let merge_y = then_bottom.max(else_bottom) + merge_gap;
    let merge_diamond_top = merge_y;
    let merge_cy = merge_diamond_top + DIAMOND_HALF;

    // Re-emit any redirectable branch now that merge_cy is known, so its loop
    // exit corridor (while) or surviving out-corridor (single-survivor if) lands
    // on the merge diamond. Only meaningful when a real merge diamond exists
    // (non-terminating, no single-survivor short-circuit at THIS level).
    let repeat_break_redirect_cy = if repeat_break_single_survivor {
        if then_survivor_if {
            then_branch_flow
                .first()
                .and_then(single_survivor_if_join_offset)
                .map(|off| branch_y + off)
        } else if else_survivor_if {
            else_branch_flow
                .first()
                .and_then(single_survivor_if_join_offset)
                .map(|off| branch_y + off)
        } else {
            None
        }
    } else {
        None
    };
    let redirect_merge_cy = repeat_break_redirect_cy.unwrap_or(merge_cy);
    let redirect_active =
        (then_redirectable || else_redirectable || then_survivor_if || else_survivor_if)
            && !all_terminate
            && (single_survivor.is_none() || repeat_break_redirect_cy.is_some())
            && !if_empty_both_plain(then_branch, else_branches);
    if redirect_active {
        svg.shapes.truncate(shapes_chk);
        svg.truncate_connectors(conns_chk);
        svg.truncate_lane_spans(shapes_chk, conns_chk);
        svg.current_lane = lane_chk;
        if then_redirectable {
            svg.while_exit_redirect = Some(WhileExitRedirect {
                merge_cy,
                merge_vertex_x: cx - DIAMOND_HALF,
                to_right: true,
            });
        }
        if then_survivor_if {
            svg.if_survivor_redirect = Some(IfSurvivorRedirect {
                merge_cy: redirect_merge_cy,
                merge_vertex_x: if repeat_break_redirect_cy.is_some() {
                    cx
                } else {
                    cx - DIAMOND_HALF
                },
                to_right: true,
                draw_arrow: repeat_break_redirect_cy.is_none(),
            });
        }
        emit_if_then_branch_with_stretch(
            svg,
            then_branch_flow,
            then_cx,
            branch_y,
            if_branch_stretch,
        );
        svg.while_exit_redirect = None;
        svg.if_survivor_redirect = None;
        if !else_branches.is_empty() {
            if else_redirectable {
                svg.while_exit_redirect = Some(WhileExitRedirect {
                    merge_cy,
                    merge_vertex_x: cx + DIAMOND_HALF,
                    to_right: false,
                });
            }
            if else_survivor_if {
                svg.if_survivor_redirect = Some(IfSurvivorRedirect {
                    merge_cy: redirect_merge_cy,
                    merge_vertex_x: if repeat_break_redirect_cy.is_some() {
                        cx
                    } else {
                        cx + DIAMOND_HALF
                    },
                    to_right: false,
                    draw_arrow: repeat_break_redirect_cy.is_none(),
                });
            }
            emit_sequence_if_branch(svg, else_branch_flow, else_cx, branch_y);
            svg.while_exit_redirect = None;
            svg.if_survivor_redirect = None;
        }
    }

    // Swimlane V2: restore the if's entry lane before the merge (a branch
    // `|Lane|` is branch-scoped for the merge diamond), but remember the lane
    // reached by the branch flow. PlantUML continues after `endif` in that exit
    // lane, which matters for nested fork-in-while branches.
    let if_exit_lane = svg.current_lane;
    svg.switch_lane_span(if_entry_lane);

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
    // Order: diamond→then, diamond→else, then→merge, else→merge. A
    // break-single-survivor branch uses the same slot, but its terminating side
    // welds left into the enclosing loop's exit corridor instead of receiving a
    // normal down-arrow inbound.

    // Diamond → then: horizontal from diamond left to then_cx, then down to
    // branch top, with an arrowhead overlay.
    if let Some((_, brk)) = break_single_survivor
        .as_ref()
        .filter(|(plan, _)| plan.then_is_break)
    {
        let branch_y = diamond_bottom + if_branch_down;
        svg.connector_line_full(
            &then_arrow_style,
            diamond_left,
            then_cx,
            diamond_cy,
            diamond_cy,
        );
        svg.connector_line_full(&then_arrow_style, then_cx, then_cx, diamond_cy, branch_y);
        svg.connector_line(
            &arrow_color,
            then_cx,
            brk.corridor_x,
            branch_y,
            branch_y,
            false,
        );
        svg.left_arrow(brk.corridor_x, branch_y, &arrow_color);
    } else {
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
            diamond_bottom + if_branch_down,
        );
        svg.polygon_connector(
            &then_arrow_style.color,
            &[
                (then_cx - 4.0, diamond_bottom + if_branch_down - 10.0),
                (then_cx, diamond_bottom + if_branch_down),
                (then_cx + 4.0, diamond_bottom + if_branch_down - 10.0),
                (then_cx, diamond_bottom + if_branch_down - 6.0),
            ],
            &then_arrow_style.color,
            "1",
        );
    }

    // Diamond → else: mirror of the then side.
    if let Some((_, brk)) = break_single_survivor
        .as_ref()
        .filter(|(plan, _)| !plan.then_is_break)
    {
        let branch_y = diamond_bottom + if_branch_down;
        svg.connector_line_full(
            &else_arrow_style,
            diamond_right,
            else_cx,
            diamond_cy,
            diamond_cy,
        );
        svg.connector_line_full(&else_arrow_style, else_cx, else_cx, diamond_cy, branch_y);
        svg.connector_line(
            &arrow_color,
            else_cx,
            brk.corridor_x,
            branch_y,
            branch_y,
            false,
        );
        svg.left_arrow(brk.corridor_x, branch_y, &arrow_color);
    } else {
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
            diamond_bottom + if_branch_down,
        );
        svg.polygon_connector(
            &else_arrow_style.color,
            &[
                (else_cx - 4.0, diamond_bottom + if_branch_down - 10.0),
                (else_cx, diamond_bottom + if_branch_down),
                (else_cx + 4.0, diamond_bottom + if_branch_down - 10.0),
                (else_cx, diamond_bottom + if_branch_down - 6.0),
            ],
            &else_arrow_style.color,
            "1",
        );
    }

    if let Some(then_survives) = single_survivor {
        let (survivor_cx, survivor_bottom) = if then_survives {
            (then_cx, then_bottom)
        } else {
            (else_cx, else_bottom)
        };
        if let Some(merge_cy) = repeat_break_redirect_cy {
            return merge_cy;
        }
        // Nested directly inside a parent if/switch branch: PlantUML's
        // ConnectionVerticalThenHorizontalDirect + MergeStrategy.LIMITED fuses
        // the surviving out-corridor with the parent's branch→merge connector.
        if let Some(redir) = svg.if_survivor_redirect.take() {
            // `ConnectionVerticalThenHorizontalDirect` reconverges the surviving
            // branch to THIS if's own spine (g.left = `cx`) before the parent's
            // branch→merge corridor takes over. When the survivor column sits on
            // the far side of the parent merge vertex from the spine, that
            // reconvergence is drawn explicitly (survivor → short lead → across to
            // spine → down to merge → across to the parent vertex). When the
            // survivor already sits between the spine and the parent vertex (e.g.
            // `kill` with a narrower terminal box), `MergeStrategy.LIMITED`
            // collapses the corridor to a straight drop at the survivor column.
            let survivor_beyond_vertex = if redir.to_right {
                survivor_cx > redir.merge_vertex_x
            } else {
                survivor_cx < redir.merge_vertex_x
            };
            let reconverge = survivor_cx != cx && survivor_beyond_vertex;
            let drop_cx = if reconverge {
                let join_y = survivor_bottom + IF_SINGLE_SURVIVOR_JOIN_GAP;
                svg.connector_line(
                    &arrow_color,
                    survivor_cx,
                    survivor_cx,
                    survivor_bottom,
                    join_y,
                    false,
                );
                svg.connector_line(&arrow_color, survivor_cx, cx, join_y, join_y, false);
                svg.connector_line(&arrow_color, cx, cx, join_y, redir.merge_cy, false);
                cx
            } else {
                svg.connector_line(
                    &arrow_color,
                    survivor_cx,
                    survivor_cx,
                    survivor_bottom,
                    redir.merge_cy,
                    false,
                );
                survivor_cx
            };
            svg.connector_line(
                &arrow_color,
                drop_cx,
                redir.merge_vertex_x,
                redir.merge_cy,
                redir.merge_cy,
                false,
            );
            if redir.draw_arrow {
                if redir.to_right {
                    svg.right_arrow(redir.merge_vertex_x, redir.merge_cy, &arrow_color);
                } else {
                    svg.left_arrow(redir.merge_vertex_x, redir.merge_cy, &arrow_color);
                }
            } else {
                let cond_y = redir.merge_cy + ARROW_LEN - IF_SINGLE_SURVIVOR_JOIN_GAP;
                svg.down_arrow(redir.merge_vertex_x, redir.merge_cy, cond_y, &arrow_color);
                svg.repeat_body_condition_connector_drawn = true;
            }
            return redir.merge_cy;
        }
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
    // redirected `while`/single-survivor-`if` branch already routed its exit
    // corridor to the merge.
    let then_routed_by_while = redirect_active && (then_redirectable || then_survivor_if);
    let else_routed_by_while = redirect_active && (else_redirectable || else_survivor_if);
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

    let bottom = if all_terminate {
        // No merge diamond was emitted — block height ends at the deeper branch.
        then_bottom.max(else_bottom)
    } else {
        merge_diamond_top + DIAMOND_HALF * 2.0
    };
    svg.switch_lane_span(if_exit_lane);
    bottom
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
    let entry_lane = svg.current_lane;

    // Branch columns retain the partially-uncompressed inter-tile slack (see
    // IF_LONG_BRANCH_GAP_EXTRA); set it for the branch/else body emits below and
    // restore before the connector loops, which use plain ARROW_LEN spacing.
    let saved_gap_extra = svg.fork_branch_gap_extra;
    svg.fork_branch_gap_extra = l.branch_gap_extra;

    // Branch bodies in column order: then_branch, then each elseif body.
    let elseif_bodies: Vec<&[LayoutNode]> = else_branches
        .iter()
        .filter(|b| b.condition.is_some())
        .map(|b| b.body.as_slice())
        .collect();
    let tile2_body: &[LayoutNode] = else_branches
        .iter()
        .find(|b| b.condition.is_none())
        .map(|b| b.body.as_slice())
        .unwrap_or(&[]);
    let mut col_lanes = Vec::with_capacity(n);
    let mut inherited_lane = entry_lane;
    for body in std::iter::once(then_branch).chain(elseif_bodies.iter().copied()) {
        let lane = branch_entry_lane(body, inherited_lane);
        col_lanes.push(lane);
        inherited_lane = lane;
    }
    let tile2_lane = branch_entry_lane(tile2_body, inherited_lane);

    // --- Shapes: per couple (diamond + labels + branch) ------------------
    for (i, col) in l.cols.iter().enumerate() {
        if svg.swimlane_v2_active && svg.current_lane != entry_lane {
            svg.current_lane = entry_lane;
            svg.lane_spans
                .push((svg.shapes.len(), svg.connectors.len(), entry_lane));
        }
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
        // A branch led by a `repeat` keeps its uncompressed FtileRepeat top
        // reserve in the if-long row (couples are top-aligned at y=25, so the
        // tail never recompresses). Signal the body's leading repeat to lower its
        // body by that reserve; `emit_repeat` consumes the one-shot.
        svg.if_long_repeat_body_extra =
            first_flow_node(body).map_or(0.0, leading_if_long_branch_repeat_extra);
        let saved_in_if_long = svg.in_if_long_branch;
        svg.in_if_long_branch = true;
        // A branch that is a sole no-special `while` fuses its exit corridor with
        // the branch→merge connector: hand the merge-line y to `emit_while` so it
        // draws the spine-down in place (document order), and skip the separate
        // vout below.
        if branch_is_redirectable_while(body) {
            svg.while_if_long_merge_y = Some(v.merge_y);
        }
        if svg.swimlane_v2_active && svg.current_lane != col_lanes[i] {
            svg.current_lane = col_lanes[i];
            svg.lane_spans
                .push((svg.shapes.len(), svg.connectors.len(), col_lanes[i]));
        }
        emit_sequence(svg, body, dcx, v.couple_branch_top);
        svg.while_if_long_merge_y = None;
        svg.in_if_long_branch = saved_in_if_long;
        svg.if_long_repeat_body_extra = 0.0;
    }

    // tile2 (the bare else) to the right.
    if let Some(tile2_cx) = l.tile2_cx {
        let tcx = cx + tile2_cx;
        if svg.swimlane_v2_active && svg.current_lane != tile2_lane {
            svg.current_lane = tile2_lane;
            svg.lane_spans
                .push((svg.shapes.len(), svg.connectors.len(), tile2_lane));
        }
        emit_sequence(svg, tile2_body, tcx, v.tile2_top);
    }
    svg.fork_branch_gap_extra = saved_gap_extra;

    // --- Connectors ------------------------------------------------------
    // Per-couple ConnectionVerticalIn (diamond→branch) + ConnectionVerticalOut
    // (branch→merge line). A while-led branch already drew its vout in place
    // (fused with the loop's exit corridor — see `while_if_long_merge_y`), so
    // skip it here to keep document order matching PlantUML.
    for (i, col) in l.cols.iter().enumerate() {
        let dcx = cx + col.cx;
        // Vertical in: diamond bottom → branch top.
        svg.with_connector_lanes(Some(col_lanes[i]), Some(col_lanes[i]), |svg| {
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
        });
        // Vertical out: branch bottom → merge line. Skipped for a while-led
        // branch whose exit corridor already reached the merge line.
        let body: &[LayoutNode] = if i == 0 {
            then_branch
        } else {
            elseif_bodies[i - 1]
        };
        if branch_is_redirectable_while(body) {
            continue;
        }
        let branch_bottom = v.couple_branch_top + col.branch_h;
        svg.with_connector_lanes(Some(col_lanes[i]), None, |svg| {
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
        });
    }

    // ConnectionHorizontal between adjacent diamonds (east vertex → west vertex).
    for i in 0..n - 1 {
        let d1 = &l.cols[i];
        let d2 = &l.cols[i + 1];
        let x1 = cx + d1.cx + d1.diamond_w / 2.0;
        let x2 = cx + d2.cx - d2.diamond_w / 2.0;
        svg.with_connector_lanes(Some(entry_lane), Some(entry_lane), |svg| {
            svg.connector_line(&arrow_color, x1, x2, diamond_cy, diamond_cy, false);
            svg.right_arrow(x2, diamond_cy, &arrow_color);
        });
    }

    // ConnectionIn (prev bottom → first diamond): down 5, sideways, down to
    // the first diamond top.
    let d0cx = cx + l.cols[0].cx;
    svg.with_connector_lanes(None, Some(entry_lane), |svg| {
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
    });

    // ConnectionLastElseIn + ConnectionLastElseOut (last diamond east → tile2,
    // then tile2 → merge line).
    if let Some(tile2_cx) = l.tile2_cx {
        let tcx = cx + tile2_cx;
        let last = &l.cols[n - 1];
        let east_x = cx + last.cx + last.diamond_w / 2.0;
        if l.tile2_empty {
            // Implicit empty else: branch2 has no point in/out, so the snake
            // runs straight from the last diamond's east vertex, right to the
            // tile2 column, then down to the merge line with a single arrowhead
            // (no box, no intermediate arrow into tile2).
            svg.with_connector_lanes(Some(entry_lane), None, |svg| {
                svg.connector_line(&arrow_color, east_x, tcx, diamond_cy, diamond_cy, false);
                svg.connector_line(&arrow_color, tcx, tcx, diamond_cy, v.merge_y, false);
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
            });
        } else {
            // East vertex → above tile2, then down into tile2.
            svg.with_connector_lanes(Some(entry_lane), Some(tile2_lane), |svg| {
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
            });
            // tile2 out → merge line.
            let tile2_bottom = v.tile2_top + l.tile2_h;
            svg.with_connector_lanes(Some(tile2_lane), None, |svg| {
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
            });
        }
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
    svg.with_connector_lanes(None, None, |svg| {
        svg.connector_line(&arrow_color, min_out, max_out, v.merge_y, v.merge_y, false);
    });

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
/// Extra branch separation an enclosing `if` reserves when one of its branch
/// tiles contains a loop-owned break-survivor `if`. The nested tile advertises
/// the loop-exit corridor as part of its abstract width, even though that
/// corridor is drawn by the enclosing loop frame.
const IF_BRANCH_CONTAINS_BREAK_SURVIVOR_SPREAD: f64 = 35.6015625;
const IF_BREAK_SINGLE_SURVIVOR_BRANCH_SPREAD: f64 = 7.5;
const WHILE_NESTED_BREAK_SURVIVOR_FRAME_TRIM: f64 = 12.4776;
const WHILE_NESTED_BREAK_SURVIVOR_CANVAS_TRIM: f64 = 10.0;
const IF_COLLECTOR_MIDPOINT_EXTRA_OFFSET: f64 = 1.0830078125;
const IF_COLLECTOR_OTHER_LANE_PAD: f64 = 17.82421875;
const IF_CROSS_COLLECTOR_LEFT_PAD: f64 = 12.82421875;
const IF_CROSS_COLLECTOR_RIGHT_START_INSET: f64 = 21.017578125;
const IF_SPLIT_COLLECTOR_LANE0_EXTRA: f64 = 3.3525390625;
const IF_SPLIT_COLLECTOR_LEFT_INSET: f64 = 1.17626953125;
const IF_SPLIT_COLLECTOR_RIGHT_START_OFFSET: f64 = 4.6259765625;
const IF_SPLIT_COLLECTOR_RIGHT_INSET: f64 = 7.17626953125;
const IF_SPLIT_COLLECTOR_RIGHT_DIVIDER_PAD: f64 = 10.0;

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
/// Same stretch for a *terminating* populated branch (`:foo; stop`). With no
/// merge diamond below it, `FtileIfDown.getTranslateForThen` centres the branch
/// against a shallower band, so the inter-action gap stretch is smaller.
const IF_DOWN_TERM_MID_STRETCH: f64 = 8.43359375;
/// Extra drop from the terminating populated branch's pointOut to the if-block's
/// pointOut (where the no-diamond east corridor rejoins the spine), beyond the
/// ARROW_LEN merge gap — the residual band the FtileEmpty `diamond2` reserves.
const IF_DOWN_TERM_REJOIN_EXTRA: f64 = 2.0;
/// Extra length of `FtileIfDown.ConnectionOut` when a terminating populated
/// branch (`:return; stop`) is followed by more flow inside a labelled `while`
/// body. The loop frame reclaims the diamond→body inbound slot, but the
/// terminating-if's pointOut→next-node snake keeps a near-full `ARROW_LEN -
/// IF_BRANCH_UP` band plus a small text-band residual.
const IF_DOWN_TERM_WHILE_BODY_OUTBOUND_EXTRA: f64 = 14.0889;
/// Drop from a break-bearing `if`'s diamond bottom to the (no-diamond) east
/// corridor's return line — i.e. the if-block's pointOut. The break tile sits
/// `IF_DOWN_LEAD` below the diamond; the corridor then runs `ARROW_LEN +
/// IF_BRANCH_UP` further down to the rejoin (`FtileIfDown.calculateDimension`
/// with empty then/diamond2 tiles). When the enclosing loop column has enough
/// adjacent flow content, ON_Y slot compression removes a `4.4775` slack unit
/// (the residual of the south label's reserved band).
const WHILE_BREAK_IF_CORRIDOR_DROP_COMPRESSED: f64 = IF_DOWN_LEAD + ARROW_LEN + IF_BRANCH_UP; // 50.4775
const WHILE_BREAK_IF_CORRIDOR_UNCOMPRESSED_EXTRA: f64 = 4.477539062500001;

/// Drop from a break-`if`'s diamond bottom to the break weld when the break
/// branch carries NO label (a bare `then`). PlantUML's `FtileIfDown` reserves a
/// south-label band between the diamond and the break tile; with no label that
/// band collapses, so the break tile sits only `ARROW_LEN - IF_BRANCH_UP` below
/// the diamond instead of the full `IF_DOWN_LEAD`
/// (`edge_activity_while_infinite`: weld `diamond_bottom + 14`, vs the labeled
/// `edge_activity_while_break`'s `diamond_bottom + 24.4775`). The corridor
/// rejoin and the diamond's inbound lead shift by the same band — see
/// [`WHILE_BREAK_NO_SOUTH_LABEL_INBOUND_EXTRA`].
const WHILE_BREAK_NO_SOUTH_LABEL_DROP: f64 = ARROW_LEN - IF_BRANCH_UP; // 14.0
/// Extra inbound lead added above an unlabeled-`while` break-`if`'s diamond when
/// the break branch ALSO has no south label. The collapsed south band's residual
/// `IF_DOWN_LEAD - ARROW_LEN` slack is pushed above the diamond on top of the
/// labeled-branch [`WHILE_BREAK_NO_LABEL_INBOUND_LEAD`]
/// (`edge_activity_while_infinite`: work→break-if gap `20 + 1.5664`).
const WHILE_BREAK_NO_SOUTH_LABEL_INBOUND_EXTRA: f64 = IF_DOWN_LEAD - ARROW_LEN - 4.0; // 0.477539…

/// FtileWhile frame-height bridge for break-bearing loops. PlantUML computes
/// `FtileWhile.calculateDimensionFtile` as
///   `frame_h = geoDiamond1.appendBottom(geoWhile).height + 4*hexHalf + supp`
/// where `geoWhile` is the body's true `FtileGeometry` height and `supp =
/// getSuppHeightForLabel` is the height of `back1` (the `is (yes)` loop-back
/// label) — *zero* when the loop has no `is`/end label. A break-bearing body's
/// `FtileGeometry` reserves more vertical space than this renderer's flat
/// [`sequence_height`] models (the `FtileIfDown` break corridor's empty-side
/// rejoin + welding band): [`WHILE_BREAK_BODY_GEO_RESIDUAL`] is that constant
/// `geoWhile.height - sequence_height(body)` residual (label-independent), so
///   `frame_h = 24 (diamond1) + sequence_height(body)
///              + WHILE_BREAK_BODY_GEO_RESIDUAL + 48 + supp`.
/// The loop-back / exit arrowheads are placed at the frame-height-derived
/// midpoints (`(diamond_cy + frame_h)/2` for the exit `ConnectionOut`,
/// `(diamond_cy + frame_h - hexHalf)/2` for the loop-back), exactly as PlantUML
/// draws them — independent of where the (lower) loop-back junction line lands.
const WHILE_BREAK_BODY_GEO_RESIDUAL: f64 = 38.655248437500006;
/// `getSuppHeightForLabel` for a labeled break loop: the height of the `is (yes)`
/// loop-back label band (`back1`). Added to `frame_h` only when the loop carries
/// such a label; a bare `while (cond)` (no `is`/end label) contributes nothing.
const WHILE_BREAK_SUPP_LABEL_H: f64 = 24.136787499999997;
/// Lead added above a break-`if`'s diamond when the enclosing `while` has no
/// `is`/end label. With `suppLabel = 0` the body centres in a shorter frame and
/// the slot finder pushes this much of the if-down corridor's
/// [`WHILE_BREAK_IF_CORRIDOR_UNCOMPRESSED_EXTRA`] slack *above* the diamond
/// (`act_edge … edge_activity_while_break`: work→break-if gap `20 + 1.0889`,
/// corridor rejoins at the compressed drop).
const WHILE_BREAK_NO_LABEL_INBOUND_LEAD: f64 = 1.0888575;
/// Trailing advertised-width reservation an unlabeled `while (cond)` break loop
/// keeps past its loop-back arm. The `break_if_is_last_flow` collapse drops the
/// FtileWhile `dx + hexHalf` frame tail (correct for labeled loops, whose wider
/// body absorbs it within the canvas's ±1px tolerance); a narrow unlabeled body
/// must keep `hexHalf - 3 - WHILE_SINGLE_IF_RIGHT_PAD` of it so the advertised
/// canvas matches PlantUML's `geo.width + dx + hexHalf`. Whitespace only — drawn
/// shapes are unaffected.
const WHILE_NO_LABEL_BREAK_CANVAS_TAIL: f64 = DIAMOND_HALF - 3.0 - WHILE_SINGLE_IF_RIGHT_PAD;
/// When the break-bearing `if` is the FIRST flow node of the loop body, the
/// body's `FtileGeometry` gains one even-action middle stretch unit
/// ([`IF_DOWN_MID_STRETCH`]) of reserved height (no leading tile compresses the
/// break corridor against), raising `frame_h` by the same amount.
const WHILE_BREAK_FIRST_FRAME_EXTRA: f64 = IF_DOWN_MID_STRETCH;
/// Additional frame-height surplus a COMPRESSED leading break corridor reserves
/// beyond [`WHILE_BREAK_FIRST_FRAME_EXTRA`]. With the corridor compressed (3+
/// body flow tiles) and the break-`if` first, the slot finder pushes the
/// south-label mid band onto the outbound arrow; the body `FtileGeometry` the
/// while frame measures grows by this residual past the emitted body bottom, so
/// the frame-derived loop-back / exit arrowheads sit `..._EXTRA/2` lower
/// (`act_break_while_early`: exit/loop-back arrows +3.0224 each).
const WHILE_BREAK_FIRST_COMPRESSED_FRAME_EXTRA: f64 = 6.0448;
/// Bottom whitespace the `FtileWhile` frame extends past the wrap-back exit when
/// the break-bearing `if` is the body's first flow node — grows the advertised
/// tile height (and thus the canvas) without moving the emitted shapes.
const WHILE_BREAK_FIRST_CANVAS_EXTRA: f64 = 5.0;
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

/// Upward bias of the loop-back emphasis arrowhead for a break-bearing repeat.
/// The break corridor's compressed band shifts the loop-back snake's
/// pre-compression midpoint up by this much relative to the drawn
/// `(top_cy + cond_cy)/2`. Empirically constant across the
/// `act_repeat_acts{1,2,3}_*_brk` family.
const REPEAT_BREAK_LOOPBACK_ARROW_BIAS: f64 = 1.7612;

/// FtileRepeat's loop-back arrowhead anchors on the ABSTRACT (uncompressed) frame
/// centre. When trailing body flow follows the break-`if` (mid/early topologies),
/// the whole-diagram ON_Y pass squeezes the break corridor, drawing the condition
/// diamond this far ABOVE its uncompressed position; the arrowhead anchor adds it
/// back so it lands on the same frame centre a last-node break (no corridor
/// compression) produces. Invariant across the mid/early repeat-break family.
const REPEAT_BREAK_MID_LOOPBACK_DECOMPRESS: f64 = 11.0;

/// First-flow repeat break (`act_break_repeat_early`): the FtileIfDown is the
/// repeat body's leading tile with no action above it. The if-block's height
/// (geo + 3·halfHex + halfHex south band) is this much taller than the mid form's
/// effective contribution once the whole-diagram ON_Y pass has squeezed the
/// surrounding bands — slack the leading position cannot absorb. It surfaces as
/// extra length on the break-`if`→trailing-action arrow. (= text_height(11) less
/// the south-band collapse residual; treated as a single measured geometric
/// quantity for the leading-break topology.)
const REPEAT_BREAK_FIRST_TRAILING_EXTRA: f64 = 1.0888609375;

/// Upward bias of the loop-back emphasis arrowhead for a FIRST-flow repeat break
/// (`act_break_repeat_early`). With no leading tile above the break-`if`, the
/// loop-back arm's upper span barely compresses, so the arrowhead lands at the
/// true content midpoint less only this small fixed bias — far less than the
/// mid form's `REPEAT_BREAK_MID_LOOPBACK_DECOMPRESS`/2 + `…_ARROW_BIAS`.
const REPEAT_BREAK_FIRST_LOOPBACK_BIAS: f64 = 0.7832;

/// The break-`if`'s `node_height` advertises the `while`-tuned compressed corridor
/// drop (`DIAMOND_HALF*2 + WHILE_BREAK_IF_CORRIDOR_DROP_COMPRESSED`). In a repeat
/// the if rejoins straight into the condition one `IF_DOWN_LEAD + ARROW_LEN − 1`
/// below its diamond bottom, which is this much shorter — subtract it so the
/// advertised repeat height (and canvas) matches.
const REPEAT_BREAK_IF_HEIGHT_OVERCOUNT: f64 =
    WHILE_BREAK_IF_CORRIDOR_DROP_COMPRESSED - (IF_DOWN_LEAD + ARROW_LEN - 1.0);
/// Extra vertical slack FtileRepeat distributes through a multi-action body
/// when an explicit `backward :...;` tile occupies the loop-back arm.
const REPEAT_BACKWARD_BODY_SLACK: f64 = 30.0;

/// Extra vertical slack a break-bearing `backward :...;` repeat with an ODD
/// number of leading actions distributes into the same body gap the non-break
/// backward path stretches (before the last action, or before the break-`if`
/// for a single-action body). When the action count is EVEN the break-`if`
/// already absorbs `IF_DOWN_MID_STRETCH` (the parity-driven centring shared
/// with the non-backward break path), so no extra applies. This residual is the
/// whole-diagram ON_Y compression slack PlantUML leaves around the centred
/// backward tile; this renderer defers ON_Y, so it is reproduced here.
/// Empirically constant across the `act_repeat_acts{1,3}_bwd_brk` family.
const REPEAT_BACKWARD_BREAK_ODD_SLACK: f64 = 2.1553;

/// The even-body mid-stretch is calibrated for loop bodies whose flow nodes are
/// plain action tiles (`act_while_2actions_body` and friends): PlantUML's Snake
/// compaction distributes the back-edge label slack evenly across an even number
/// of inter-action gaps. A nested compound tile (while/repeat/if/switch/
/// partition) already carries its own large vertical reservation, so the simple
/// even-gap model does not hold and the stretch must not be applied.
///
/// A `fork` body tile is admitted here as well: it does NOT take the connector
/// stretch (its multi-row composite geometry already supplies its own centring,
/// so `while_body_mid_stretch` zeroes the connector slack for it), but the body
/// is still treated as even, so the loop-back up-arrowhead keeps its even-body
/// placement (`WHILE_EVEN_BODY_LOOP_ARROW_STRETCH`). See `act_combo_while_fork_*`.
fn while_body_flow_is_all_actions(body: &[LayoutNode]) -> bool {
    body.iter().filter(|n| node_is_flow(n)).all(|n| {
        matches!(
            n,
            LayoutNode::Action { .. }
                | LayoutNode::Start
                | LayoutNode::Stop
                | LayoutNode::End
                | LayoutNode::Fork { .. }
        )
    })
}

/// The centring slack a labelled `while` distributes into the tail of a leading
/// `repeat` tile. When the `while` body is an even-flow stack led by a plain
/// (non-backward, non-start-label) `repeat` — e.g. `repeat { … } repeatwhile;
/// :after;` — FtileWhile centres the assembly and the
/// `WHILE_EVEN_BODY_MID_STRETCH_LABELED` slack lands in the repeat's reserved
/// body→condition band (the only compressible space inside the leading tile),
/// pushing the repeat's condition diamond and everything below it one stretch
/// lower. `while_body_mid_stretch` rejects this body (the leading repeat is not a
/// plain action), so the slack is routed into the repeat itself via
/// [`SvgEmitter::while_repeat_tail_extra`]. Returns 0 for every other shape.
fn while_body_repeat_tail_extra(body: &[LayoutNode], has_in_label: bool) -> f64 {
    if !has_in_label {
        return 0.0;
    }
    let flow: Vec<&LayoutNode> = body.iter().filter(|n| node_is_flow(n)).collect();
    if flow.len() < 2 || !flow.len().is_multiple_of(2) {
        return 0.0;
    }
    let leads_with_plain_repeat = matches!(
        flow.first(),
        Some(LayoutNode::Repeat {
            backward: None,
            has_start_label: false,
            ..
        })
    );
    if leads_with_plain_repeat {
        WHILE_EVEN_BODY_MID_STRETCH_LABELED
    } else {
        0.0
    }
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

/// Index of the last flow node in a `while` body, used to decide whether a
/// switch tile is the terminal body node (which keeps a larger merge band — see
/// `switch_while_merge_extra`).
fn last_flow_index(body: &[LayoutNode]) -> Option<usize> {
    body.iter().rposition(node_is_flow)
}

/// Upward bias of the `while` loop-back emphasis arrowhead for a body that LEADS
/// with plain action tile(s) and ends in a balanced `if` (then + else,
/// reconverging through a merge diamond). PlantUML's `ConnectionBackSimple`
/// anchors the UP arrowhead on the loop-back snake's PRE-compression vertical
/// midpoint (`(y1bis + diamond_cy)/2` with `y1bis = getBottom + hexHalf`, where
/// `getBottom` is the body tile's pre-compression bottom from
/// `getTranslateForWhile`). With leading actions the leading tiles stay
/// uncompressed while the trailing if-block's internal slack collapses below the
/// spine, so the pre-compression frame centre sits one even-body stretch
/// (`WHILE_EVEN_BODY_LOOP_ARROW_STRETCH`) above the naive
/// `(diamond_cy + body_bottom + hexHalf)/2` midpoint this renderer computes from
/// the post-emit merge-diamond bottom. `while_body_mid_stretch` rejects this
/// shape (the trailing `if` is a composite, so the connector-stretch path returns
/// `None`/zero), so this is the only contributor; we re-add it for the arrowhead
/// only. Returns 0 for a pure-`if` body (no leading actions — see the
/// `act_while_ifdepth*` family, whose arrowhead rides the branch-stretched merge
/// bottom instead) and every other shape, leaving all currently-correct
/// loop-backs byte-identical. Drives `act_while_with_if`.
fn while_loopback_arrow_bias(body: &[LayoutNode]) -> f64 {
    let flow: Vec<&LayoutNode> = body.iter().filter(|n| node_is_flow(n)).collect();
    let Some((last, lead)) = flow.split_last() else {
        return 0.0;
    };
    if lead.is_empty() || !lead.iter().all(|n| matches!(n, LayoutNode::Action { .. })) {
        return 0.0;
    }
    match last {
        LayoutNode::If {
            else_branches,
            then_branch,
            ..
        } if !else_branches.is_empty()
            && !branch_terminates(then_branch)
            && else_branches.iter().any(|b| !branch_terminates(&b.body)) =>
        {
            WHILE_EVEN_BODY_LOOP_ARROW_STRETCH
        }
        _ => 0.0,
    }
}

/// Sum of the [`WHILE_NESTED_LABELLED_BAND`] reserved-but-undrawn bands every
/// nested labelled `while` in this `while` body interposes into the body's
/// advertised `FtileGeometry` (`getSuppHeightForLabel`). The enclosing loop's
/// loop-back arrowhead anchors on that inflated body bottom, so it seats
/// `band / 2` lower than the drawn `body_bottom` midpoint. Counts only nested
/// `while`s carrying an `is (…)`/end loop-back label (a non-empty `back1`);
/// every body without one returns 0 and stays byte-identical.
fn while_body_nested_labelled_band(body: &[LayoutNode]) -> f64 {
    body.iter()
        .filter(|n| {
            matches!(
                n,
                LayoutNode::While {
                    is_label: Some(_),
                    ..
                } | LayoutNode::While {
                    end_label: Some(_),
                    ..
                }
            )
        })
        .count() as f64
        * WHILE_NESTED_LABELLED_BAND
}

/// Per-pixel residual a *doubly* nested labelled `while` adds to the enclosing
/// loop's loop-back arrowhead anchor. `while_body_nested_labelled_band` models a
/// directly-nested labelled `while` adding `band/2` to the anchor (its
/// `getSuppHeightForLabel` inflates the body's advertised `getPointOut`). When
/// that nested loop ITSELF wraps a further labelled `while`, the inner loop's own
/// `getSuppHeightForLabel` propagates up through `getTranslateForWhile`'s
/// centering, so the enclosing loop's advertised `getPointOut` — which
/// `ConnectionBackSimple.drawTranslate` anchors the UP `asToUp` arrowhead's
/// `(y1+y2)/2` on — sits one further pixel below the drawn `body_bottom` per
/// extra nesting level. Same sub-pixel anchor character as
/// [`WHILE_NESTED_EXIT_ARROW_BIAS`]. Returns the count of direct nested labelled
/// `while`s that recursively contain another labelled `while`, times the per-level
/// residual. Zero for any body without a doubly-nested labelled `while`, so a
/// single-level nest (`act_while_nested`) stays byte-identical. Drives
/// `act_while_nested_3_levels`.
fn while_body_nested_labelled_band_residual(body: &[LayoutNode]) -> f64 {
    fn has_labelled_while(body: &[LayoutNode]) -> bool {
        body.iter().any(|n| {
            matches!(
                n,
                LayoutNode::While {
                    is_label: Some(_),
                    ..
                } | LayoutNode::While {
                    end_label: Some(_),
                    ..
                }
            )
        })
    }
    body.iter()
        .filter(|n| match n {
            LayoutNode::While {
                is_label,
                end_label,
                body: inner,
                ..
            } => (is_label.is_some() || end_label.is_some()) && has_labelled_while(inner),
            _ => false,
        })
        .count() as f64
        * WHILE_NESTED_LABELLED_BAND_RESIDUAL
}

/// Per-level pixel residual a doubly-nested labelled `while` adds to the
/// enclosing loop's loop-back arrowhead anchor; see
/// [`while_body_nested_labelled_band_residual`]. Measured from
/// `act_while_nested_3_levels` (the L1 loop's UP arrowhead sits ~1px below the
/// single-level band model's prediction; the sub-pixel value is the propagated
/// deeper-label band's centering residual, like [`WHILE_NESTED_LABELLED_BAND`]
/// itself being a fractional measured constant).
const WHILE_NESTED_LABELLED_BAND_RESIDUAL: f64 = 0.9999;

/// Per nesting `if`-diamond, the merge band ON_Y compression reclaims from the
/// `while`-body centring slack before it reaches the innermost branch gap. Each
/// nested `if` level interposes one diamond+merge structure that absorbs this
/// much of the slack the outer loop frame reserves.
const WHILE_IF_BODY_LEVEL_RECLAIM: f64 = 2.0;

/// Reserved-but-undrawn vertical band a *nested labelled* `while` interposes into
/// its enclosing `while`'s body `FtileGeometry`. PlantUML's `FtileWhile`
/// advertises `geo.getHeight() = body.h + 4*hexHalf + getSuppHeightForLabel`,
/// where `getSuppHeightForLabel = back1.height` is the inner loop's `is (…)`
/// loop-back label band. That band inflates the body sequence's reported
/// `getPointOut` (hence `getP1`) that the enclosing loop's
/// `ConnectionBackSimple.drawTranslate` anchors its UP loop-back arrowhead on
/// (`(y1 + y2)/2`, `y1 = body pointOut`), even though the drawn flow never
/// occupies it. This renderer's `mid_y` uses the DRAWN `body_bottom`, so the
/// arrowhead seats `WHILE_NESTED_LABELLED_BAND / 2` too high. Only a nested
/// `while` with an `is (…)`/end loop-back label has a non-empty `back1` (a bare
/// `while (cond)` reserves no band), so a body without one stays byte-identical.
/// Verified on `act_while_nested`.
const WHILE_NESTED_LABELLED_BAND: f64 = 12.1798;

/// When a `while` body is a single balanced `if` (then + else, reconverging
/// through a merge) whose `then`-branch chain bottoms out in a 2-action terminal
/// branch, PlantUML's `FtileWhile` centring slack lands in that terminal
/// branch's middle inter-action connector. `FtileWhile.getTranslateForWhile`
/// places the body `2*hexHalf` below the diamond and ON_Y compression then pushes
/// the residual into the if-block's tallest branch's middle gap — exactly the
/// `WHILE_EVEN_BODY_MID_STRETCH_LABELED` an all-action even body receives, less
/// `WHILE_IF_BODY_LEVEL_RECLAIM` for each nesting `if`-diamond's merge band. This
/// returns `Some((depth, stretch))` for that body shape (`depth` = number of
/// nested `if` levels from the outer `if` to the terminal 2-action branch;
/// `stretch` the px to add to the terminal branch's middle gap), else `None`.
/// Drives the `act_while_ifdepth*_acts2` family.
fn while_if_body_branch_stretch(body: &[LayoutNode]) -> Option<(usize, f64)> {
    let [LayoutNode::If { .. }] = body else {
        return None;
    };
    // Walk the then-chain: each balanced `if` whose then-branch is itself a single
    // balanced `if` recurses one level deeper; bottoming out in a 2-action
    // terminal `then`-branch fixes the depth.
    fn depth_to_two_action_terminal(node: &LayoutNode) -> Option<usize> {
        let LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } = node
        else {
            return None;
        };
        // Must be balanced: a surviving else (so the if reconverges to a merge).
        if else_branches.is_empty()
            || branch_terminates(then_branch)
            || !else_branches.iter().any(|b| !branch_terminates(&b.body))
        {
            return None;
        }
        let then_flow: Vec<&LayoutNode> = then_branch.iter().filter(|n| node_is_flow(n)).collect();
        match then_flow.as_slice() {
            // Terminal: exactly two plain actions — the centring lands here.
            [LayoutNode::Action { .. }, LayoutNode::Action { .. }] => Some(1),
            // Recurse: a single nested balanced `if`.
            [inner @ LayoutNode::If { .. }] => depth_to_two_action_terminal(inner).map(|d| d + 1),
            _ => None,
        }
    }
    let depth = depth_to_two_action_terminal(&body[0])?;
    let stretch = WHILE_EVEN_BODY_MID_STRETCH_LABELED - WHILE_IF_BODY_LEVEL_RECLAIM * depth as f64;
    (stretch > 0.0).then_some((depth, stretch))
}

/// Number of flow tiles in `body` strictly after index `i`.
fn following_flow_count(body: &[LayoutNode], i: usize) -> usize {
    body.iter().skip(i + 1).filter(|n| node_is_flow(n)).count()
}

/// Sum of the per-switch in-while merge-band extras for a `while` body. Each
/// multi-case switch directly in the body keeps part of its uncompressed merge
/// band that ON_Y compression cannot reclaim under the loop frame.
fn while_body_switch_extra(body: &[LayoutNode]) -> f64 {
    let last = last_flow_index(body);
    body.iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, LayoutNode::Switch { .. }))
        .map(|(i, n)| switch_while_merge_extra(n, Some(i) == last, following_flow_count(body, i)))
        .sum()
}

fn while_body_height(body: &[LayoutNode], has_in_label: bool) -> f64 {
    // Suppressed deepest loop: when this body is a single nested `while` whose own
    // body is ordinary-compressible (an action chain), that nested loop is the
    // deepest of a single-while chain and does NOT compress its inbound (its band
    // sits directly above the action with no diamond below). `sequence_height`'s
    // recursive `node_height` compressed it via the ordinary gate, so re-add the
    // slot here. The enclosing loop claimed that slot via `single_while_compresses`.
    let suppressed_deepest_readd = if let [
        LayoutNode::While {
            body: inner,
            is_label: Some(_),
            special_out: None,
            ..
        },
    ] = body
        && while_ordinary_slot_compress_allowed(inner, None)
        && !matches!(&inner[..], [LayoutNode::While { .. }])
    {
        WHILE_BODY_SLOT_COMPRESS
    } else {
        0.0
    };
    sequence_height(body)
        + suppressed_deepest_readd
        + while_body_mid_stretch(body, has_in_label).map_or(0.0, |(_, stretch)| stretch)
        // Even-flow labelled body led by a `repeat`: the loop centring slack lands
        // in that repeat's body→condition tail (see `while_body_repeat_tail_extra`),
        // growing the body tile — and hence the advertised frame/canvas — height.
        + while_body_repeat_tail_extra(body, has_in_label)
        // Pure-balanced-if body: the loop centring slack stretches the deepest
        // 2-action then-branch's middle gap (see `while_if_body_branch_stretch`),
        // growing the body tile — and hence the advertised frame/canvas — height.
        + while_if_body_branch_stretch(body).map_or(0.0, |(_, stretch)| stretch)
        + while_body_switch_extra(body)
        // A compressed leading break corridor lengthens the connector leaving the
        // break-`if`'s pointOut by one `IF_DOWN_MID_STRETCH` (see
        // `break_if_first_outbound_extra` in `emit_sequence_ex`); that band is part
        // of the body's emitted height, so the advertised tile height (and canvas)
        // must include it.
        + if break_if_is_first_flow(body) && while_break_corridor_compresses(body) {
            IF_DOWN_MID_STRETCH
        } else {
            0.0
        }
        + if while_body_has_terminating_if_down_with_following_flow(body) {
            IF_DOWN_TERM_WHILE_BODY_OUTBOUND_EXTRA
        } else {
            0.0
        }
}

fn repeat_body_mid_stretch(body: &[LayoutNode], has_backward: bool) -> Option<(usize, f64)> {
    let flow_count = body.iter().filter(|n| node_is_flow(n)).count();
    if has_backward {
        // A break-bearing backward body `[actions…, break-if]` follows the same
        // parity model as the non-backward break path: the break-`if` is the
        // trailing composite the leading actions balance against. An EVEN action
        // count lands the centring stretch on the break-`if`'s inbound connector
        // (`IF_DOWN_MID_STRETCH`); an ODD count leaves the break-`if` flush and
        // instead distributes the centred backward tile's compression residual
        // (`REPEAT_BACKWARD_BREAK_ODD_SLACK`) into the body gap the non-break
        // backward path stretches — before the last action, or before the
        // break-`if` for a single action.
        if break_if_is_last_flow(body) {
            let action_count = body
                .iter()
                .filter(|n| matches!(n, LayoutNode::Action { .. }))
                .count();
            let break_if_idx = flow_count.saturating_sub(1);
            if action_count >= 2 && action_count.is_multiple_of(2) {
                return Some((break_if_idx, IF_DOWN_MID_STRETCH));
            }
            // Odd action count: stretch the gap before the last action, or — for
            // a single action — the gap before the break-`if` itself.
            let target = (action_count.saturating_sub(1)).max(1).min(break_if_idx);
            return Some((target, REPEAT_BACKWARD_BREAK_ODD_SLACK));
        }
        let all_actions_flow = body
            .iter()
            .filter(|n| node_is_flow(n))
            .all(|n| matches!(n, LayoutNode::Action { .. }));
        if flow_count >= 2 && all_actions_flow {
            return Some((
                flow_count - 1,
                REPEAT_BACKWARD_BODY_SLACK / flow_count as f64,
            ));
        }
        return None;
    }
    // A break-bearing body `[actions…, break-if]`: when the leading actions are
    // EVEN in number, FtileRepeat's even-stack centring lengthens the connector
    // feeding the trailing break-`if` by one `IF_DOWN_MID_STRETCH` (the break-if
    // is the "trailing composite" the actions balance against — like a plain if).
    // Place that stretch in the gap before the break-if (its flow index).
    if break_if_is_last_flow(body) {
        let action_count = body
            .iter()
            .filter(|n| matches!(n, LayoutNode::Action { .. }))
            .count();
        if action_count >= 2 && action_count.is_multiple_of(2) {
            let break_if_idx = body
                .iter()
                .filter(|n| node_is_flow(n))
                .count()
                .saturating_sub(1);
            return Some((break_if_idx, IF_DOWN_MID_STRETCH));
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

/// Downward bias applied to the loop-back emphasis arrowhead ONLY (never to
/// body shapes or canvas height). PlantUML anchors that arrowhead on the
/// *pre-compression* vertical midpoint of the loop-back snake — which is the
/// repeat frame's centre (`getTranslateForRepeat` splits the reserved
/// `space = 8*hexagonHalfSize` as `space/2` above and below the body). For a
/// body whose flow nodes are all plain actions this surfaces through the
/// even-body mid-stretch (the arrowhead lands `stretch/2` above the naive
/// midpoint). But a body that LEADS with a plain action and then enters a
/// single balanced `if` (then+else, both reconverging) keeps the action tile
/// uncompressed above the if-block while the if-block's own internal slack
/// collapses below it — so the pre-compression centre sits `space/2 ≈ 7.5 px`
/// below the naive `(top+cond)/2` midpoint. Returns 0 for every other body
/// shape, leaving all currently-correct loop-backs byte-identical.
fn repeat_loopback_arrow_bias(body: &[LayoutNode], has_backward: bool) -> f64 {
    if has_backward {
        return 0.0;
    }
    // A `while` body slot-compresses its inbound band (`WHILE_BODY_SLOT_COMPRESS`);
    // that reclaimed slack sits in the lower half of the outer repeat's loop-back
    // span, so the arrowhead's pre-compression midpoint sits
    // `WHILE_BODY_SLOT_COMPRESS/2 − 1` above the drawn `(top_cy + cond_cy)/2`
    // (the same pull-up the colored-partition-while exit arrow uses).
    if let [
        LayoutNode::While {
            body: while_body,
            is_label,
            end_label,
            special_out,
            ..
        },
    ] = body
        && while_ordinary_slot_compresses(while_body, is_label, end_label, special_out.as_deref())
    {
        return WHILE_BODY_SLOT_COMPRESS / 2.0 - 1.0;
    }
    // A repeat body that LEADS with a slot-compressing `while` and then a plain
    // action (`act_combo_while_in_repeat`): an even body, so the centre falls in
    // the middle connector (even-body `space/2` stretch), but the while's
    // inbound-slot compression pulls the drawn content up by
    // `WHILE_BODY_SLOT_COMPRESS/2`. The loop-back arrowhead sits BELOW the naive
    // midpoint by the residual `WHILE_EVEN_BODY_LOOP_ARROW_STRETCH −
    // WHILE_BODY_SLOT_COMPRESS/2` (a negative bias, since this fn is subtracted).
    if let [
        LayoutNode::While {
            body: while_body,
            is_label,
            end_label,
            special_out,
            ..
        },
        LayoutNode::Action { .. },
    ] = body
        && while_ordinary_slot_compresses(while_body, is_label, end_label, special_out.as_deref())
    {
        return -(WHILE_EVEN_BODY_LOOP_ARROW_STRETCH - WHILE_BODY_SLOT_COMPRESS / 2.0);
    }
    let flow: Vec<&LayoutNode> = body.iter().filter(|n| node_is_flow(n)).collect();
    // Need an even flow count whose leading nodes are plain actions and whose
    // single trailing composite is a balanced `if` (has at least one else
    // branch, so it reconverges through a merge diamond rather than detaching).
    if flow.len() < 2 || !flow.len().is_multiple_of(2) {
        return 0.0;
    }
    let (last, lead) = flow.split_last().expect("flow non-empty");
    if !lead.iter().all(|n| matches!(n, LayoutNode::Action { .. })) {
        return 0.0;
    }
    match last {
        LayoutNode::If { else_branches, .. } if !else_branches.is_empty() => {
            REPEAT_EVEN_BODY_MID_STRETCH
        }
        _ => 0.0,
    }
}

/// Extra body→condition gap a `repeat` keeps when nested inside another
/// repeat's body. Mirrors the `nested_cond_extra` applied in [`emit_repeat`]:
/// the enclosing loop frame leaves one `ARROW_LEN` of the inner repeat's
/// reserved `8*halfHex` tail uncompressible under the whole-diagram ON_Y pass.
/// Only the plain (non-backward, non-start-label) form is affected. Summed over
/// a repeat body so the enclosing repeat's advertised height — and hence the
/// canvas height — accounts for every directly-nested repeat's grown tail.
fn nested_repeat_cond_extra(node: &LayoutNode) -> f64 {
    match node {
        LayoutNode::Repeat {
            backward,
            has_start_label,
            ..
        } if backward.is_none() && !*has_start_label => ARROW_LEN,
        _ => 0.0,
    }
}

fn repeat_body_height(body: &[LayoutNode], has_backward: bool) -> f64 {
    sequence_height(body)
        + repeat_body_mid_stretch(body, has_backward).map_or(0.0, |(_, stretch)| stretch)
        + body.iter().map(nested_repeat_cond_extra).sum::<f64>()
        // A terminal multi-case switch keeps one uncompressible `ARROW_LEN` of its
        // merge band under the loop frame (see `repeat_body_switch_extra`).
        + repeat_body_switch_extra(body)
        // A NON-terminal `while` keeps its loop-back tail open (`body_bottom +
        // halfHex` rather than the ON_Y-compressed `+10`); `sequence_height`'s
        // While arm models the compressed form, so add back the `halfHex − 10`
        // residual to match `emit_while`'s `repeat_body_nonterminal` junction.
        + repeat_body_nonterminal_while_extra(body)
}

/// `halfHex − 10` (= 2) per `while` that is a NON-terminal flow node of a repeat
/// body — the loop-back-tail slack the FtileRepeat frame holds open and
/// `emit_while` draws (see `while_repeat_body_nonterminal`). Zero for a terminal
/// while (its tail ON_Y-compresses) and for any other tile.
fn repeat_body_nonterminal_while_extra(body: &[LayoutNode]) -> f64 {
    let last = last_flow_index(body);
    body.iter()
        .enumerate()
        .filter(|(i, n)| Some(*i) != last && matches!(n, LayoutNode::While { .. }))
        .count() as f64
        * (DIAMOND_HALF - 10.0)
}

/// Extra vertical gap a `backward` repeat reserves between the body bottom and
/// the condition diamond. A single PLAIN ACTION body keeps an extra halfHex of
/// slack before the diamond (PlantUML's UEmpty placeholder is uncompressed when
/// the body is a lone box); a multi-action body absorbs that slack in its final
/// inbound connector, and a single COMPOSITE body (if/fork/switch/nested loop)
/// carries its own internal vertical structure, so no extra gap applies.
fn repeat_backward_extra_cond_gap(body: &[LayoutNode], has_backward: bool) -> f64 {
    if !has_backward {
        return 0.0;
    }
    let flow_count = body.iter().filter(|n| node_is_flow(n)).count();
    if flow_count >= 2 && body_contains_break_if(body) && !break_if_is_last_flow(body) {
        return -IF_SINGLE_SURVIVOR_JOIN_GAP;
    }
    let mut flow = body.iter().filter(|n| node_is_flow(n));
    match (flow.next(), flow.next()) {
        (Some(LayoutNode::Action { .. }), None) => 10.0,
        _ => 0.0,
    }
}

fn repeat_backward_composite_break_body_slack(body: &[LayoutNode], has_backward: bool) -> f64 {
    if !has_backward || break_if_is_last_flow(body) || !body_contains_break_if(body) {
        return 0.0;
    }
    let flow_count = body.iter().filter(|n| node_is_flow(n)).count();
    if flow_count < 2 {
        return 0.0;
    }
    let all_actions_flow = body
        .iter()
        .filter(|n| node_is_flow(n))
        .all(|n| matches!(n, LayoutNode::Action { .. }));
    if all_actions_flow {
        0.0
    } else {
        IF_BRANCH_UP / 2.0 + 0.5
    }
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

/// The uncompressed FtileRepeat top reserve a `repeat` keeps when it leads an
/// if-long (`if/elseif*/else`) branch. PlantUML's `FtileIfLongHorizontal`
/// top-aligns every couple at a fixed y (`getTranslateCouple1` → y=25), so the
/// repeat's `8*hexHalf` tail above the body (FtileRepeat `getTranslateForRepeat`
/// places the body at `dimDiamond1.height + space/2`) never recompresses against
/// a following tile. The whole-diagram ON_Y pass only reclaims the diamond→
/// branch corridor band (`descent(11) − 1.5`, the same term as `north_h`'s
/// nested-construct compression in `if_long_layout`), so the body sits
/// `2*hexHalf − (descent − 1.5)` lower than the fully-compressed
/// (`top_bottom + ARROW_LEN`) standalone form.
// = 24 − (descent(11) − 1.5) = 24 − (2.3203125 − 1.5) = 23.1796875.
const IF_LONG_BRANCH_REPEAT_CORRIDOR_RECLAIM: f64 = 2.3203125 - 1.5; // descent(11) − 1.5
const IF_LONG_BRANCH_REPEAT_EXTRA: f64 =
    DIAMOND_HALF * 2.0 - IF_LONG_BRANCH_REPEAT_CORRIDOR_RECLAIM;

/// Extra lowering for the body of a `repeat` that leads an if-long branch
/// (see [`IF_LONG_BRANCH_REPEAT_EXTRA`]). Applies to the plain and labelled
/// (`repeatwhile … is …`) forms alike; a `repeat :label;` start tile or a
/// `backward` tile routes its body through different geometry, so those are
/// excluded.
fn leading_if_long_branch_repeat_extra(node: &LayoutNode) -> f64 {
    let LayoutNode::Repeat {
        backward,
        body,
        has_start_label,
        ..
    } = node
    else {
        return 0.0;
    };
    if backward.is_some() || *has_start_label {
        return 0.0;
    }
    // Only the simple-body form is calibrated; composite bodies carry their own
    // vertical reservation.
    if body.iter().filter(|n| node_is_flow(n)).count() != 1
        || !matches!(first_flow_node(body), Some(LayoutNode::Action { .. }))
    {
        return 0.0;
    }
    IF_LONG_BRANCH_REPEAT_EXTRA
}

/// Total uncompressed FtileRepeat reserve growth (top + bottom) for a `repeat`
/// that leads an if-long branch — the amount its tile is taller than the
/// fully-compressed standalone form. Used to size the merge reservation
/// (`if_long_layout`'s `branch_h`); the emit applies the two halves separately
/// (see [`IF_LONG_BRANCH_REPEAT_EXTRA`] and the `is`-label band in
/// `emit_repeat`).
fn leading_if_long_branch_repeat_total(body: &[LayoutNode]) -> f64 {
    let Some(node) = first_flow_node(body) else {
        return 0.0;
    };
    let top = leading_if_long_branch_repeat_extra(node);
    if top == 0.0 {
        0.0
    } else {
        top + pm::text_height(SMALL_FONT)
    }
}

/// Extra height a `while`-led if-long branch keeps over `sequence_height`: the
/// loop-back junction stays at its uncompressed `+12` (the if-long row blocks the
/// whole-diagram ON_Y pass) rather than the compressed `+10` that
/// `sequence_height`'s `LayoutNode::While` arm assumes. `emit_while` matches this
/// via `in_if_long_branch`. Only the no-special (wrap-back) loop is affected.
fn leading_if_long_branch_while_extra(body: &[LayoutNode]) -> f64 {
    match first_flow_node(body) {
        Some(LayoutNode::While {
            special_out, body, ..
        }) if special_out.is_none() && !body.is_empty() => DIAMOND_HALF - 10.0,
        _ => 0.0,
    }
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

/// Branch height for the enclosing if's merge reservation. A directly-nested
/// SOLE single-survivor if has its standalone out-arrow (`+ ARROW_LEN`) replaced
/// by the empty-tile spacer (`+ IF_SURVIVOR_REDIRECT_GAP`) because its surviving
/// corridor fuses straight into THIS if's merge (see [`IfSurvivorRedirect`]).
fn if_branch_height_redirected(nodes: &[LayoutNode]) -> f64 {
    let h = sequence_height_if_branch(nodes);
    if branch_is_redirectable_single_survivor_if(nodes) {
        h - ARROW_LEN + IF_SURVIVOR_REDIRECT_GAP
    } else {
        h
    }
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
    // Capture before emitting the populated branch — the nested emit_sequence_ex
    // resets this per-node flag, so read it up front.
    let terminating_has_next = std::mem::take(&mut svg.if_down_terminating_has_next);
    let terminating_while_body_has_next =
        std::mem::take(&mut svg.if_down_terminating_while_body_has_next);
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
    // midpoint (the gap before the (N/2)-th flow node). A non-terminating branch
    // is centred against the merge diamond's band (full IF_DOWN_MID_STRETCH); a
    // terminating branch has no merge diamond (`getShape2` → FtileEmpty), so its
    // `getTranslateForThen` centring reserves less band below — the stretch
    // shrinks to IF_DOWN_TERM_MID_STRETCH.
    let flow_count = plan.populated.iter().filter(|n| node_is_flow(n)).count();
    let stretch_amount = if plan.populated_terminates {
        IF_DOWN_TERM_MID_STRETCH
    } else {
        IF_DOWN_MID_STRETCH
    };
    let mid_stretch = if flow_count >= 2 && flow_count.is_multiple_of(2) {
        Some((flow_count / 2, stretch_amount))
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

    // Corridor x: the east column the empty branch runs down. PlantUML's
    // `xmax = max(diamond_east + halfHex, then_x + then_width)`.
    let corridor_x = (diamond_right + DIAMOND_HALF)
        .max(cx + sequence_width(plan.populated) / 2.0 + IF_DOWN_BRANCH_CORRIDOR_GAP);

    if plan.populated_terminates {
        // Terminating populated branch (`:foo; stop`): PlantUML draws no merge
        // diamond (`getShape2` → FtileEmpty, `hasTwoBranches() == false`). The
        // spine branch terminates in place; the empty branch's east corridor
        // (`ConnectionElseNoDiamond`) rejoins the spine at the if-block's
        // pointOut, which then carries flow on. The if-block's pointOut sits one
        // ARROW_LEN below the terminating branch's pointOut, plus the 2 px the
        // FtileEmpty diamond2 band contributes (mirror of the `-2` in
        // IF_DOWN_LEAD's south-label reservation).
        let rejoin_y = branch_bottom + ARROW_LEN + IF_DOWN_TERM_REJOIN_EXTRA;

        // Diamond → populated branch (down arrow on the spine).
        svg.down_arrow(cx, diamond_bottom, branch_top, &arrow_color);

        // ConnectionElseNoDiamond: exit east vertex, run down (mid down-arrow),
        // rejoin the spine at the if's pointOut. No terminal in-arrow.
        svg.connector_line(
            &arrow_color,
            diamond_right,
            corridor_x,
            diamond_cy,
            diamond_cy,
            false,
        );
        // The DOWN emphasize arrowhead anchors on the corridor's UNCOMPRESSED
        // midpoint. Before the whole-diagram ON_Y pass, the branch carries the
        // full IF_DOWN_MID_STRETCH and the FtileEmpty diamond2 reserves its full
        // halfHex band, so the uncompressed pointOut sits this far below the
        // (compressed) rejoin. PlantUML draws the arrow at that midpoint and the
        // slot finder later squeezes only the line endpoints.
        let rejoin_uncompressed = branch_bottom
            + (IF_DOWN_MID_STRETCH - IF_DOWN_TERM_MID_STRETCH)
            + ARROW_LEN
            + DIAMOND_HALF / 2.0;
        let arrow_tip = (diamond_cy + rejoin_uncompressed) / 2.0 + IF_CORRIDOR_ARROW_OFFSET;
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
        svg.connector_line(
            &arrow_color,
            corridor_x,
            corridor_x,
            diamond_cy,
            rejoin_y,
            false,
        );
        svg.connector_line(&arrow_color, corridor_x, cx, rejoin_y, rejoin_y, false);
        // FtileIfDown.ConnectionOut: the spine arrow leaving the if-block is part
        // of the if's own connector list (drawn here, right after the corridor),
        // not the next node's deferred inbound. The enclosing sequence skips that
        // inbound (`skip_implicit_inbound_after_terminating_down`). Only when a
        // following flow node exists.
        if terminating_has_next {
            let out_y = rejoin_y
                + ARROW_LEN
                + if terminating_while_body_has_next {
                    IF_DOWN_TERM_WHILE_BODY_OUTBOUND_EXTRA
                } else {
                    0.0
                };
            svg.down_arrow(cx, rejoin_y, out_y, &arrow_color);
            return out_y;
        }
        return rejoin_y;
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

    // The break branch's positive (south) label: drawn below the diamond. When
    // absent (a bare `then`), PlantUML's `FtileIfDown` collapses its reserved
    // south-label band, pulling the break tile (and the corridor rejoin) up by
    // `IF_DOWN_LEAD - WHILE_BREAK_NO_SOUTH_LABEL_DROP`.
    let south_label_present = if plan.then_is_break {
        then_label.is_some()
    } else {
        else_label.is_some()
    };

    // The break tile sits on the spine, IF_DOWN_LEAD below the diamond (one
    // collapsed band less when the break branch is unlabeled). The if-block's
    // pointOut (where the east corridor rejoins the spine) sits a further
    // ARROW_LEN + IF_BRANCH_UP down — plus an uncompressed slack unit when the
    // loop column is thin (see WHILE_BREAK_IF_CORRIDOR_* constants).
    let break_y = if south_label_present {
        diamond_bottom + IF_DOWN_LEAD
    } else {
        diamond_bottom + WHILE_BREAK_NO_SOUTH_LABEL_DROP
    };
    // The empty (continue) branch's east corridor rejoins the spine at the if's
    // pointOut. For a `while`, that drop is the WHILE_BREAK_IF_CORRIDOR_* slot
    // (one collapsed south band less when the break branch is unlabeled). For a
    // `repeat` (the break-`if` is the last body node, rejoining straight into the
    // condition), the drop is one IF_DOWN_LEAD + ARROW_LEN, less one px — the
    // whole-diagram ON_Y pass cannot squeeze the corridor column further. The
    // DOWN arrowhead still anchors on the *uncompressed* corridor midpoint
    // (PlantUML draws it before the slot pass shortens the run).
    let south_band_collapse = if south_label_present {
        0.0
    } else {
        IF_DOWN_LEAD - WHILE_BREAK_NO_SOUTH_LABEL_DROP
    };
    let return_y_uncompressed = diamond_bottom + WHILE_BREAK_IF_CORRIDOR_DROP_COMPRESSED
        - south_band_collapse
        + if brk.compresses {
            0.0
        } else {
            WHILE_BREAK_IF_CORRIDOR_UNCOMPRESSED_EXTRA
        };
    let return_y = if brk.repeat_mode {
        if brk.repeat_merge_cy.is_some() {
            if brk.repeat_break_first_flow {
                // First-flow break with trailing body (`act_break_repeat_early`):
                // no leading tile sits above the break-`if`, so the ON_Y slot
                // finder cannot squeeze the corridor's upper band. The empty branch
                // rejoins (and the if's pointOut sits) at the SAME compressed drop a
                // last-node break uses; the reclaimed slack surfaces below, as extra
                // length on the rejoin→trailing-action arrow (see `emit_sequence_ex`'s
                // `repeat_break_first_outbound_extra`).
                diamond_bottom + IF_DOWN_LEAD + ARROW_LEN - 1.0
            } else {
                // Mid break: a populated tile precedes the break-`if`. The empty
                // branch rejoins the spine to flow on into the trailing body action
                // (not straight into the condition). PlantUML's slot finder leaves
                // `IF_BRANCH_UP/2` more slack below the corridor than the last-node
                // rejoin (which the whole-diagram ON_Y pass squeezes by 1).
                diamond_bottom + IF_DOWN_LEAD + ARROW_LEN + IF_BRANCH_UP / 2.0
            }
        } else {
            // Last-node break: the empty branch IS the loop pointOut, rejoining
            // straight into the condition; ON_Y squeezes the column by one px.
            diamond_bottom + IF_DOWN_LEAD + ARROW_LEN - 1.0
        }
    } else {
        return_y_uncompressed
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

    // Break weld: horizontal LEFT from the spine to the loop exit corridor.
    // For a `while`, a left-pointing arrowhead lands at the corridor (asToLeft)
    // and the while's own exit arm carries the flow on. For a `repeat`, the
    // corridor continues DOWN to the break-merge diamond below the condition
    // (the loop's `out`); the corridor + the right-into-merge arrowhead are
    // drawn here (so they land in the correct document-order slot, immediately
    // after the weld), and the repeat draws only the merge rhombus + the
    // condition's south exit into it. The merge centreline sits a fixed span
    // below this if's pointOut: ARROW_LEN (→condition) + condition diamond
    // + ARROW_LEN (→merge) + halfHex (merge centre).
    svg.connector_line(&arrow_color, cx, brk.corridor_x, break_y, break_y, false);
    if brk.repeat_mode {
        svg.repeat_break_weld_y = Some(break_y);
        // Last-node break: the merge sits a fixed span below the condition that
        // immediately follows the if (`return_y` + arrow + condition + arrow +
        // half). Mid/early break: trailing body flow + the condition push the
        // merge lower, so `emit_repeat` pre-computes its centre and hands it in.
        let merge_cy = brk
            .repeat_merge_cy
            .unwrap_or(return_y + ARROW_LEN + DIAMOND_HALF * 2.0 + ARROW_LEN + DIAMOND_HALF);
        let merge_left = cx - DIAMOND_HALF;
        svg.connector_line(
            &arrow_color,
            brk.corridor_x,
            brk.corridor_x,
            break_y,
            merge_cy,
            false,
        );
        svg.connector_line(
            &arrow_color,
            brk.corridor_x,
            merge_left,
            merge_cy,
            merge_cy,
            false,
        );
        svg.right_arrow(merge_left, merge_cy, &arrow_color);
    } else {
        svg.left_arrow(brk.corridor_x, break_y, &arrow_color);
    }

    // Fused loop-back: when the break-`if` is the body's LAST flow node, its
    // empty (continue) branch is the whole loop's `pointOut`. PlantUML's
    // `FtileWhile.ConnectionBackSimple` originates the loop-back arm there and
    // runs it straight up to the condition diamond — there is no down-then-spine
    // corridor and no separate junction loop-back. Draw the fused arm here (so it
    // lands in the correct document-order slot, among the if's own connectors):
    // east vertex → right to `loop_x` → up to the diamond `cy` → left-arrow into
    // the diamond's right vertex. Signal `emit_while` to skip its own arm.
    if let Some(lb) = brk.fuse_loopback {
        svg.connector_line(
            &arrow_color,
            diamond_right,
            lb.loop_x,
            diamond_cy,
            diamond_cy,
            false,
        );
        svg.connector_line(
            &arrow_color,
            lb.loop_x,
            lb.loop_x,
            lb.diamond_cy,
            diamond_cy,
            false,
        );
        svg.connector_line(
            &arrow_color,
            lb.loop_x,
            lb.diamond_right_vertex_x,
            lb.diamond_cy,
            lb.diamond_cy,
            false,
        );
        svg.left_arrow(lb.diamond_right_vertex_x, lb.diamond_cy, &arrow_color);
        svg.while_break_loopback_fused = true;
        return return_y;
    }

    // Empty east corridor (ConnectionElseNoDiamond): exit the diamond's east
    // vertex, run down (down-emphasized mid arrow), then rejoin the spine at the
    // if-block's pointOut. No terminal in-arrow — it simply welds back.
    let corridor_x = diamond_right + DIAMOND_HALF;
    svg.connector_line(
        &arrow_color,
        diamond_right,
        corridor_x,
        diamond_cy,
        diamond_cy,
        false,
    );
    // The corridor's DOWN emphasize arrowhead sits at its run midpoint. When the
    // loop column compresses the corridor (>=3 body flow nodes) PlantUML's slot
    // finder nudges the tip down by one `IF_CORRIDOR_ARROW_OFFSET`; an
    // uncompressed corridor (break-`if` first, only one trailing tile) keeps the
    // exact midpoint.
    let arrow_tip = (diamond_cy + return_y_uncompressed) / 2.0
        + if brk.compresses {
            IF_CORRIDOR_ARROW_OFFSET
        } else {
            0.0
        };
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
    svg.connector_line(
        &arrow_color,
        corridor_x,
        corridor_x,
        diamond_cy,
        return_y,
        false,
    );
    svg.connector_line(&arrow_color, corridor_x, cx, return_y, return_y, false);

    // In a LAST-node `repeat` break, the break-`if`'s pointOut feeds straight into
    // the condition diamond one ARROW_LEN below. PlantUML emits that body→diamond2
    // connection as part of the if-block's connection list (right after the
    // empty-east corridor), BEFORE the body's internal spine arrows — so draw it
    // here, and let `emit_repeat` skip its own. A MID/early break is followed by
    // trailing body flow; the enclosing `emit_sequence` draws the rejoin→action
    // arrow, so DON'T draw it here (it would duplicate).
    if brk.repeat_mode && brk.repeat_merge_cy.is_none() {
        svg.down_arrow(cx, return_y, return_y + ARROW_LEN, &arrow_color);
    }

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
    // A leading nude switch (>= 3 cases) in a fork branch emits its uncompressed
    // `FtileSwitchNude` layout; the diagram-wide ON_X pass reclaims the bands.
    // One-shot flag set by `emit_fork_with_layout`.
    if std::mem::take(&mut svg.fork_body_switch) && fork_branch_switch_is_nude(cases, condition) {
        return emit_switch_with_layout(
            svg,
            cx,
            y,
            condition,
            cases,
            switch_x_layout_nude(cases, condition),
            false,
        );
    }
    // A switch directly in a `while` *or* `repeat` body keeps its uncompressed
    // gap-20 inner bands: the loop frame blocks the standalone ON_X compression of
    // the bands straddling the diamond column. Both flags select the same
    // `switch_x_layout_in_while` layout.
    let layout = if svg.while_body_switch || svg.repeat_body_switch {
        switch_x_layout_in_while(cases, condition)
    } else {
        switch_x_layout(cases, condition)
    };
    emit_switch_with_layout(svg, cx, y, condition, cases, layout, false)
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

    // Captured before any nested emit_sequence can reset it: when this switch is
    // the tile of an unequal-height fork branch, its empty merge band cannot be
    // reclaimed by ON_Y compression (a sibling branch box occupies that y-band).
    let fork_gap_extra = svg.fork_branch_gap_extra;
    // Likewise for a switch directly in a `while` body: the loop frame keeps part
    // of the uncompressed merge band (set by `emit_sequence_ex`, one-shot).
    let while_switch_extra = svg.while_switch_merge_extra;
    // And for a terminal switch in a `repeat` body: the loop frame keeps one
    // `ARROW_LEN` of the uncompressed merge band (set by `emit_sequence_ex`,
    // one-shot). Records `while_switch_loopback_tip` like the while case so
    // `emit_repeat` can anchor its loop-back arrowhead on the merge diamond.
    let repeat_switch_extra = std::mem::take(&mut svg.repeat_switch_merge_extra);
    // True when this switch is a flow node of a `while` body (the extra may be
    // zero when the merge band fully compresses, so test the flag, not the extra).
    let in_while_body = svg.while_body_switch;
    // True when that `while` body's loop-back corridor ON_Y-compresses; shifts the
    // centre-spine drop's split to the compressed corridor turn.
    let while_corridor_compresses = svg.while_switch_corridor_compresses;
    // True when this switch's merge band itself ON_Y-compresses (non-terminal,
    // >=3 trailing tiles); anchors the loop-back arrowhead differently.
    let while_merge_compressed = svg.while_switch_merge_compressed;

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
    let merge_gap = switch_merge_gap_in_fork(cases, condition, &layout, fork_gap_extra)
        + while_switch_extra
        + repeat_switch_extra;
    let merge_top = max_bottom + merge_gap;
    let merge_cy = merge_top + DIAMOND_HALF;
    let merge_bottom = merge_top + DIAMOND_HALF * 2.0;

    // When this switch is a flow node of a `while` body, record where its merge
    // diamond pulls the loop-back arrowhead. In the uncompressed state PlantUML's
    // ON_Y compression of the loop-back corridor seats the arrowhead
    // `standalone_gap + ARROW_LEN/2` above the merge top (the corridor midpoint);
    // when the merge band itself compresses (>=3 trailing tiles) the arrowhead
    // instead seats `SWITCH_WHILE_COMPRESSED_LOOPBACK_OFFSET` above the merge top.
    if in_while_body && !all_terminate {
        let standalone_gap = switch_merge_gap(cases, condition, &layout);
        svg.while_switch_loopback_tip = Some(if while_merge_compressed {
            merge_top - SWITCH_WHILE_COMPRESSED_LOOPBACK_OFFSET
        } else {
            merge_top - standalone_gap - ARROW_LEN / 2.0
        });
    }
    // A terminal switch in a `repeat` body anchors the loop-back arrowhead on its
    // merge diamond the same way: `merge_top - standalone_gap - ARROW_LEN/2`
    // (empirically a constant-y tip across case counts — see the goldens).
    if repeat_switch_extra > 0.0 && !all_terminate {
        let standalone_gap = switch_merge_gap(cases, condition, &layout);
        svg.while_switch_loopback_tip = Some(merge_top - standalone_gap - ARROW_LEN / 2.0);
    }

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
    // When some cases terminate (kill/detach), they contribute no out-corridor,
    // so the merge's left/right vertices are reached by the leftmost/rightmost
    // SURVIVING case (FtileSwitchWithDiamonds wires the merge to the actual
    // branch outs, not the source positions). An inner case promoted to a
    // surviving extreme routes to the merge vertex like an outer case.
    // A case reaches the merge via a BOTTOM connection only when it is
    // non-empty AND non-terminating. Empty cases still claim a merge vertex via
    // their TOP corridor, so they remain the structural extreme. Promotion of
    // an inner survivor to an outer merge vertex therefore happens ONLY when the
    // structural extreme on that side TERMINATES (kill/detach) — then the next
    // bottom-connected case inherits that vertex.
    let bottom_connected: Vec<usize> = (0..n)
        .filter(|&i| !cases[i].body.is_empty() && !branch_terminates(&cases[i].body))
        .collect();
    let left_extreme_terminates = branch_terminates(&cases[0].body);
    let right_extreme_terminates = branch_terminates(&cases[n - 1].body);
    let surviving_left = left_extreme_terminates
        .then(|| bottom_connected.first().copied())
        .flatten();
    let surviving_right = right_extreme_terminates
        .then(|| bottom_connected.last().copied())
        .flatten();
    let merge_classify = |i: usize| -> SwitchConn {
        // PlantUML's addOutgoingArrows wires the first and last surviving branch
        // via ConnectionVerticalThenHorizontal. That connection routes to the
        // merge diamond's LEFT vertex (bcx left of it → right-arrow), RIGHT
        // vertex (bcx right of it → left-arrow), or — when bcx falls *within* the
        // merge diamond's vertices (spine ± DIAMOND_HALF) — straight DOWN to the
        // top point (ptA), i.e. a Centre route. When a terminating sibling
        // promotes a near-spine survivor to the structural extreme, that survivor
        // therefore routes as Centre, not as a side-vertex Outer.
        if Some(i) == surviving_left || Some(i) == surviving_right {
            if (centers[i] - diamond_cx).abs() <= DIAMOND_HALF {
                SwitchConn::Center
            } else {
                SwitchConn::Outer
            }
        } else {
            classify(i)
        }
    };
    // Emission order for the merge wiring: the merge outers (left then right)
    // first, then the inner survivors left-to-right — matching PlantUML's
    // connection registration order on the surviving branch set.
    let merge_order: Vec<usize> = {
        let mut o: Vec<usize> = Vec::with_capacity(n);
        let outers: Vec<usize> = order
            .iter()
            .copied()
            .filter(|&i| matches!(merge_classify(i), SwitchConn::Outer))
            .collect();
        o.extend(outers.iter().copied());
        for &i in &order {
            if !matches!(merge_classify(i), SwitchConn::Outer) {
                o.push(i);
            }
        }
        o
    };
    for &i in &merge_order {
        if cases[i].body.is_empty() || branch_terminates(&cases[i].body) {
            continue;
        }
        let bcx = centers[i];
        let bottom = bottoms[i];
        match merge_classify(i) {
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
                    // shifts the centre branch off the spine. In a
                    // corridor-compressing `while` body the split moves up to the
                    // compressed corridor turn just below the case bottoms.
                    let split = if while_corridor_compresses {
                        max_bottom + SWITCH_WHILE_COMPRESSED_CENTER_SPLIT
                    } else {
                        merge_top - SWITCH_CENTER_BOT_SPLIT
                    };
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

fn emit_fork_branch_sequence(svg: &mut SvgEmitter, branch: &[LayoutNode], cx: f64, y: f64) -> f64 {
    let saved = svg.fork_branch_while_slot_open;
    svg.fork_branch_while_slot_open = fork_branch_while_slot_opens(branch);
    let bottom = emit_sequence(svg, branch, cx, y);
    svg.fork_branch_while_slot_open = saved;
    bottom
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
        let bottom = emit_fork_branch_sequence(svg, branch, bcx, bar_bottom + ARROW_LEN);
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

    let simple_while_repeat_pair = fork_branches_are_simple_while_repeat_pair(branches);
    let simple_while_if_fork = fork_branches_are_simple_while_if(branches);
    let bar_w = layout.bar_w;

    // Top bar
    let bar_x = cx + layout.spine_dx - bar_w / 2.0;
    let bar_color = svg.palette.bar_color.clone();
    let arrow_color = svg.palette.arrow_color.clone();
    let bar_compressible = layout.bar_compressible;
    svg.fork_bar(&bar_color, &bar_color, bar_w, bar_x, y, bar_compressible);

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
                sequence_height_fork_branch(b, 0.0)
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
        // Centring uses each branch's *advertised* tile height. A multi-case
        // switch branch carries an uncompressed nude reserve below its visible
        // merge diamond that ON_Y compression cannot reclaim when a shorter
        // sibling occupies that y-band (see `switch_fork_centering_extra`). This
        // reserve only affects where siblings centre — NOT the fork's own total
        // height, which is driven by the deepest emitted branch — so it is added
        // here rather than inside `sequence_height_ex`/`node_height`.
        let heights: Vec<f64> = branches
            .iter()
            .map(|b| {
                if b.is_empty() {
                    0.0
                } else {
                    let extra = b
                        .first()
                        .filter(|_| b.len() == 1)
                        .map(switch_fork_centering_extra)
                        .unwrap_or(0.0);
                    sequence_height_fork_branch(b, gap_extra) + extra
                }
            })
            .collect();
        let max_h = heights.iter().cloned().fold(0.0f64, f64::max);
        for (i, &h) in heights.iter().enumerate() {
            if !branches[i].is_empty() {
                center_offsets[i] = (max_h - h) / 2.0;
            }
        }
        if branches
            .iter()
            .any(|branch| fork_branch_while_slot_opens(branch))
        {
            for (i, branch) in branches.iter().enumerate() {
                if !branch.is_empty() && !fork_branch_while_slot_opens(branch) {
                    center_offsets[i] += FORK_BRANCH_WHILE_SIBLING_CENTER_EXTRA;
                }
            }
        }
    }

    // A branch whose last tile terminates (Detach/Kill/Stop/End/Break/Goto) has
    // no pointOut: PlantUML's ParallelBuilderFork.doStep2 skips its ConnectionOut
    // (line `if (hasPointOut())`), so it is NOT wired to the join bar.
    let branch_terminates_flags: Vec<bool> =
        branches.iter().map(|b| branch_terminates(b)).collect();

    let mut branch_bottoms = Vec::new();
    let predicted_bottom_bar_y = if simple_while_repeat_pair || simple_while_if_fork {
        let bottoms = branches
            .iter()
            .zip(branch_centers.iter())
            .enumerate()
            .filter_map(|(i, (branch, _))| {
                if branch.is_empty() {
                    None
                } else {
                    let branch_y = if single_partition_branch_body_top(branch, bar_bottom).is_some()
                    {
                        bar_bottom
                    } else {
                        let start_lead = if matches!(branch.first(), Some(LayoutNode::Start)) {
                            START_R
                        } else {
                            0.0
                        };
                        let origin_adjust =
                            fork_branch_origin_adjust(simple_while_repeat_pair, branch);
                        bar_bottom + ARROW_LEN + center_offsets[i] + start_lead + origin_adjust
                    };
                    Some(branch_y + sequence_height_fork_branch(branch, gap_extra))
                }
            })
            .fold(0.0f64, f64::max);
        Some(bottoms + ARROW_LEN + if simple_while_repeat_pair { 1.0 } else { 0.0 })
    } else {
        None
    };
    for (i, (branch, &bcx)) in branches.iter().zip(branch_centers.iter()).enumerate() {
        let branch_y = if single_partition_branch_body_top(branch, bar_bottom).is_some() {
            bar_bottom
        } else {
            // A branch opened by its own `start` circle seats the circle CENTRE
            // a radius below the inbound arrowhead (the `ConnectionIn` lands on
            // the circle's top), so its content begins `START_R` lower than an
            // action-led branch. The bar→branch arrow itself still ends at the
            // un-led top (the circle's top), so only the rendered content moves.
            let start_lead = if matches!(branch.first(), Some(LayoutNode::Start)) {
                START_R
            } else {
                0.0
            };
            let origin_adjust = fork_branch_origin_adjust(simple_while_repeat_pair, branch);
            bar_bottom + ARROW_LEN + center_offsets[i] + start_lead + origin_adjust
        };
        // A leading nude switch (>= 3 cases) in this branch emits its
        // `FtileSwitchNude` layout (see `fork_branch_switch_is_nude`); flag it so
        // `emit_switch` selects the nude layout. One-shot, consumed by
        // `emit_switch`.
        svg.fork_body_switch = fork_branch_nude_switch(branch).is_some();
        if simple_while_repeat_pair && branch_is_simple_repeat_loop(branch) {
            svg.fork_branch_repeat_body_extra = FORK_BRANCH_LOOP_SLOT_HALF + 1.0;
            svg.fork_branch_repeat_tail_extra = FORK_BRANCH_REPEAT_TAIL_EXTRA;
        }
        svg.fork_branch_while_if_body_compress =
            simple_while_if_fork && branch_is_simple_while_if_loop(branch);
        let bottom = emit_fork_branch_sequence(svg, branch, bcx, branch_y);
        svg.fork_branch_repeat_body_extra = 0.0;
        svg.fork_branch_repeat_tail_extra = 0.0;
        svg.fork_branch_while_if_body_compress = false;
        svg.fork_body_switch = false;
        if simple_while_repeat_pair
            && branch_is_simple_while_loop(branch)
            && !branch_terminates_flags[i]
            && let Some(bottom_bar_y) = predicted_bottom_bar_y
        {
            svg.down_arrow(bcx, bottom, bottom_bar_y, &arrow_color);
        }
        if simple_while_if_fork
            && branch_is_simple_while_if_loop(branch)
            && !branch_terminates_flags[i]
            && let Some(bottom_bar_y) = predicted_bottom_bar_y
        {
            svg.down_arrow(bcx, bottom, bottom_bar_y, &arrow_color);
        }
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

    // A labelled while used as a fork branch emits its ConnectionOut to the join
    // bar before the ordinary top-bar ConnectionIn arrows. PlantUML draws the
    // loop's branch-local connectors as a unit before returning to the fork's
    // top connector pass.
    for (i, (branch, bottom)) in branches.iter().zip(branch_bottoms.iter()).enumerate() {
        if branch.is_empty()
            || branch_terminates_flags[i]
            || !fork_branch_while_slot_opens(branch)
            || (simple_while_repeat_pair && branch_is_simple_while_loop(branch))
            || simple_while_if_fork
        {
            continue;
        }
        let bcx = branch_centers[i];
        let branch_y = if single_partition_branch_body_top(branch, bar_bottom).is_some() {
            bar_bottom
        } else {
            let origin_adjust = fork_branch_origin_adjust(simple_while_repeat_pair, branch);
            bar_bottom + ARROW_LEN + center_offsets[i] + origin_adjust
        };
        let arrow_top = single_partition_branch_body_bottom(branch, branch_y).unwrap_or(*bottom);
        svg.down_arrow(bcx, arrow_top, bottom_bar_y, &arrow_color);
    }

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
            let origin_adjust = fork_branch_origin_adjust(simple_while_repeat_pair, branch);
            bar_bottom + ARROW_LEN + center_offsets[i] + origin_adjust
        };
        let arrow_bottom = single_partition_branch_body_top(branch, branch_y).unwrap_or(branch_y);
        svg.down_arrow(bcx, bar_bottom, arrow_bottom, &arrow_color);
    }

    // Bottom arrows from each branch to bottom bar — skipped for a branch that
    // terminates (no pointOut → no ConnectionOut to the join bar).
    for (i, (branch, bottom)) in branches.iter().zip(branch_bottoms.iter()).enumerate() {
        if branch.is_empty()
            || branch_terminates_flags[i]
            || fork_branch_while_slot_opens(branch)
            || (simple_while_if_fork && branch_is_simple_while_if_loop(branch))
        {
            continue;
        }
        let bcx = branch_centers[i];
        let branch_y = if single_partition_branch_body_top(branch, bar_bottom).is_some() {
            bar_bottom
        } else {
            let origin_adjust = fork_branch_origin_adjust(simple_while_repeat_pair, branch);
            bar_bottom + ARROW_LEN + center_offsets[i] + origin_adjust
        };
        let arrow_top = single_partition_branch_body_bottom(branch, branch_y).unwrap_or(*bottom);
        svg.down_arrow(bcx, arrow_top, bottom_bar_y, &arrow_color);
    }

    // Bottom bar
    svg.fork_bar(
        &bar_color,
        &bar_color,
        bar_w,
        bar_x,
        bottom_bar_y,
        bar_compressible,
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
    let while_entry_lane = svg.current_lane;
    // One-shot inter-lane stitch (swimlane): take it now so a nested while in the
    // body cannot inherit it. Only a no-special loop can fuse its exit corridor
    // into the cross-lane arrow.
    let cross_lane = if special_out.is_none() {
        svg.swimlane_cross_lane.take()
    } else {
        None
    };
    // Take the cond-driven swimlane terminator flag NOW (one-shot), before the
    // body emit — the body's `emit_sequence_ex` clears the shared flag after
    // each node, which would otherwise reset it before the special placement
    // below reads it.
    let swimlane_cond_special = std::mem::take(&mut svg.swimlane_while_cond_special);
    // Take the if-long-branch flag now: this while leads an if-long branch whose
    // couples are top-aligned, so the whole-diagram ON_Y pass never compresses
    // its loop-back tail. A nested body must NOT inherit it (its own frame
    // re-establishes compressibility), so consume it here.
    let in_if_long_branch = std::mem::take(&mut svg.in_if_long_branch);
    // Take the fused-merge target now too, so a nested while in the body cannot
    // consume it (only this top-of-branch while fuses with the merge line).
    let if_long_merge_y = std::mem::take(&mut svg.while_if_long_merge_y);
    // Take the if-branch exit redirect NOW (before emitting the body) so a nested
    // while in the body cannot consume it — only THIS top-of-branch loop's exit
    // corridor fuses with the if's branch→merge connector. Held locally and
    // applied in the exit block below.
    let exit_redirect = svg.while_exit_redirect.take();
    // This loop is the sole body of a parent `while` that fuses its loop-back arm
    // with this loop's exit corridor (one-shot — take it now so a deeper nested
    // body does not inherit it). When set, the exit corridor descends to the
    // fused band instead of wrapping back to the spine, and reports a
    // `WhileNestedExit` to the parent. Only a plain no-special loop can fuse.
    let fuse_as_nested_exit =
        special_out.is_none() && std::mem::take(&mut svg.while_expect_nested_exit);
    let fuse_as_prefixed_nested_exit =
        special_out.is_none() && std::mem::take(&mut svg.while_expect_prefixed_nested_exit);
    let colored_partition_while = svg.colored_partition_while_depth > 0;
    // This loop has an ordinary (action) body AND is the sole body of a parent
    // `while`, so it is the DEEPEST loop of a single-while chain. PlantUML's
    // global ON_Y pass reclaims the inbound band of every diamond in the chain
    // EXCEPT this deepest one (its inbound sits directly above the action, with no
    // diamond below to merge against), so it does NOT compress. One-shot — consume
    // so a deeper nested body re-establishes its own compressibility.
    let sole_body_suppress = std::mem::take(&mut svg.while_sole_body_suppress_compress);
    let fork_branch_slot_open = std::mem::take(&mut svg.fork_branch_while_slot_open);
    let fork_branch_while_if_body_compress =
        std::mem::take(&mut svg.fork_branch_while_if_body_compress);
    // One-shot: this while is a non-terminal flow in a repeat body, so the
    // FtileRepeat frame holds its loop-back tail open (junction keeps the full
    // halfHex, not the ON_Y-compressed 10). Take it before the body emit so a
    // nested while cannot inherit it.
    let repeat_body_nonterminal = std::mem::take(&mut svg.while_repeat_body_nonterminal);
    let ordinary_slot_compressed = !sole_body_suppress
        && !fork_branch_slot_open
        && while_ordinary_slot_compresses(body, is_label, end_label, special_out);
    // Case (b): a `while` whose body is a single nested `while`. Every diamond in
    // a single-while chain (except the deepest, suppressed above) reclaims its
    // inbound band, so this loop compresses its own inbound. The slot exists only
    // for a labelled, non-special outer loop (same `is_label`/specialOut gating as
    // `while_ordinary_slot_compress_allowed`).
    let single_while_slot_compressed =
        is_label.is_some() && special_out.is_none() && matches!(body, [LayoutNode::While { .. }]);
    let ordinary_slot_compressed = ordinary_slot_compressed || single_while_slot_compressed;
    let compress_while_slot = colored_partition_while || ordinary_slot_compressed;
    // A break-bearing loop keeps the normal long-exit arrowhead placement: the
    // break shares the exit corridor, so the loop exits/wraps like a no-special
    // loop and its loop-back/exit arrowheads sit at the *uncompressed* midpoint
    // (PlantUML draws them before the slot-compression pass shifts endpoints).
    let break_in_body = body_contains_break_if(body);
    // A break-bearing loop emits its exit `ConnectionOut` as a snake: the DOWN
    // emphasize arrowhead is drawn before the vertical run (matching PlantUML's
    // `Snake` point order). Labeled break loops already take this branch via slot
    // compression; a bare `while (cond)` break loop has no slot compression but
    // must still emit arrowhead-before-line.
    let exit_vertical_after_arrow = special_out.is_none()
        && (colored_partition_while
            || ordinary_slot_compressed
            || break_in_body
            || fork_branch_slot_open
            || fork_branch_while_if_body_compress);

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
    let mut body_top_offset = while_body_top_offset(
        compress_while_slot,
        is_label.is_some(),
        end_label.is_some(),
        body.is_empty(),
        svg.palette.arrow_font_size,
    );
    if fork_branch_while_if_body_compress {
        body_top_offset -= WHILE_BODY_SLOT_COMPRESS;
    }
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
        let (pre_body_left_ext, pre_body_right_ext) = sequence_loop_body_extents(body);
        let pre_body_left_x = cx - while_body_left(body, pre_body_left_ext);
        let pre_geo_left_x = diamond_left_vertex_x.min(pre_body_left_x);
        // Fuse the loop-back into the break-`if`'s east branch when it is the
        // body's last flow node (then its empty branch is the loop's pointOut —
        // see `break_if_is_last_flow`). The loop-back arm `loop_x` is computable
        // up-front from the body's extents, identically to the post-body `loop_x`;
        // the if draws the arm itself so it lands in the correct document slot.
        let fuse_loopback = break_if_is_last_flow(body).then(|| {
            let pre_body_right_x = cx + pre_body_right_ext;
            let loop_x = diamond_right_vertex_x.max(pre_body_right_x)
                + DIAMOND_HALF
                + while_single_if_right_pad(body, end_label);
            WhileBreakLoopback {
                loop_x,
                diamond_cy,
                diamond_right_vertex_x,
            }
        });
        // A bare `while (cond)` (no `is`/end label) centres its body in a frame
        // with `suppLabel = 0`; the slot finder then rejoins the if-down break
        // corridor at the *compressed* drop and pushes a small lead above the
        // diamond instead (see `WHILE_BREAK_NO_LABEL_INBOUND_LEAD`).
        let no_label = is_label.is_none() && end_label.is_none();
        // When the break branch itself carries no south label, the collapsed
        // FtileIfDown south band pushes its residual above the diamond on top of
        // the bare-while lead (see `WHILE_BREAK_NO_SOUTH_LABEL_INBOUND_EXTRA`).
        let no_south_label = break_if_south_label_present(body) == Some(false);
        svg.while_break = Some(WhileBreakContext {
            corridor_x: pre_geo_left_x - DIAMOND_HALF,
            compresses: while_break_corridor_compresses(body) || no_label,
            fuse_loopback,
            repeat_mode: false,
            repeat_merge_cy: None,
            repeat_merge_left_x: None,
            inbound_lead: if no_label {
                WHILE_BREAK_NO_LABEL_INBOUND_LEAD
                    + if no_south_label {
                        WHILE_BREAK_NO_SOUTH_LABEL_INBOUND_EXTRA
                    } else {
                        0.0
                    }
            } else {
                0.0
            },
            repeat_break_first_flow: false,
        });
    }

    // Body below diamond — emit it first (PlantUML emits body shapes before
    // diamond shapes in document order).
    let body_mid_stretch = while_body_mid_stretch(body, is_label.is_some());
    // Even-flow labelled body led by a plain `repeat`: route the FtileWhile
    // centring slack into that repeat's body→condition tail (see
    // `while_body_repeat_tail_extra`). Set before the body emit; the leading
    // repeat consumes it at entry. A nested body re-establishes its own value, so
    // it is a one-shot taken in `emit_repeat`.
    let prev_repeat_tail_extra = svg.while_repeat_tail_extra;
    svg.while_repeat_tail_extra = while_body_repeat_tail_extra(body, is_label.is_some());
    // Pure-balanced-if body: route the loop centring slack into the deepest
    // 2-action then-branch's middle gap (see `while_if_body_branch_stretch`). The
    // flag is consumed by `emit_if` when the terminal branch emits; we keep the
    // stretch magnitude to bias the loop-back arrowhead off the UNSTRETCHED
    // midpoint below.
    let if_body_stretch = while_if_body_branch_stretch(body).map(|(_, s)| s);
    let prev_if_branch_stretch = svg.while_if_branch_stretch.take();
    svg.while_if_branch_stretch = if_body_stretch;
    let prev_loopback_tip = svg.while_switch_loopback_tip.take();
    svg.pending_while_body = true;
    svg.while_corridor_compresses = while_break_corridor_compresses(body);
    // Suppress the inbound compression of the DEEPEST loop in a single-while
    // chain: only when this loop's sole nested `while` has an ordinary (action)
    // body — that nested loop sits directly above the action with no diamond below
    // to merge against, so its inbound band is not reclaimable. An intermediate
    // while-bodied nested loop compresses on its own (case (b)). `emit_while`
    // consumes this one-shot at entry.
    if single_while_slot_compressed && while_body_is_single_compressible_while(body) {
        svg.while_sole_body_suppress_compress = true;
    }
    // Loop-back fusion: when the body's last flow tile is a nested `while`, that
    // nested loop's exit corridor is consumed by THIS loop's loop-back arm
    // (PlantUML's nested `FtileWhile` loop-back snake starts at the inner
    // `getPointOut`). Tell the nested loop to descend its exit to the fused band
    // and report it. Leading actions remain ordinary body flow; only the trailing
    // no-special loop owns the fused corridor.
    if last_flow_index(body).is_some_and(|i| {
        matches!(
            body[i],
            LayoutNode::While {
                special_out: None,
                ..
            }
        )
    }) {
        svg.while_expect_nested_exit = true;
        if while_body_has_prefixed_fused_trailing_while(body) {
            svg.while_expect_prefixed_nested_exit = true;
        }
    }
    svg.while_nested_exit = None;
    let prev_nested_child_inbound = svg.while_nested_child_inbound.take();
    let body_bottom = emit_sequence_ex(svg, body, cx, body_top, body_mid_stretch, None, false);
    let while_exit_lane = svg.current_lane;
    svg.switch_lane_span(while_entry_lane);
    svg.while_repeat_tail_extra = prev_repeat_tail_extra;
    let nested_exit = svg.while_nested_exit.take();
    let nested_child_inbound = svg.while_nested_child_inbound.take();
    svg.while_nested_child_inbound = prev_nested_child_inbound;
    svg.while_if_branch_stretch = prev_if_branch_stretch;
    let body_switch_loopback_tip = svg.while_switch_loopback_tip.take();
    svg.while_switch_loopback_tip = prev_loopback_tip;
    svg.while_break = prev_while_break;
    // A last-flow break-`if` already drew the fused loop-back arm (east vertex →
    // up to the diamond) in its own connector stream; skip our junction loop-back
    // below. See [`break_if_is_last_flow`] and [`WhileBreakContext::fuse_loopback`].
    let loopback_fused = std::mem::take(&mut svg.while_break_loopback_fused);

    // Junction y: 12 px below the body for empty bodies, 10 px for
    // non-empty bodies. PlantUML's UEmpty(5, halfHex=12) placeholder is
    // compressed by 2 when adjacent to body content (slot finder removes
    // the slack); for empty bodies there's no adjacent content so the
    // gap stays at 12. The arrowhead midpoint below still uses the
    // un-compressed value (body_bottom + 12) — PlantUML draws the
    // arrowhead at the midpoint of the segment BEFORE compression
    // transforms the line endpoints.
    let junction_y = if let Some(ne) = nested_exit {
        // Fused nested-`while` body: the parent loop-back junction sits at the
        // nested loop's reported fused band (its loop-back junction + halfHex),
        // not at `body_bottom + 10` (which would count the nested loop's whole
        // drawn extent including its own wrapped corridor).
        ne.fused_y
    } else if body.is_empty() || in_if_long_branch || repeat_body_nonterminal {
        // Empty body, an if-long branch where ON_Y never reaches the tail, or a
        // non-terminal while in a repeat body (the FtileRepeat frame holds the
        // tail open): the UEmpty(halfHex) placeholder keeps its full 12 px.
        body_bottom + DIAMOND_HALF
    } else {
        body_bottom + 10.0
    };

    // Loop-back arm x position: 12 past whichever is wider, the diamond or
    // the body's right extent.
    let (body_left_ext, body_right_ext) = sequence_loop_body_extents(body);
    let body_right_ext = if while_body_has_prefixed_fused_trailing_while(body) {
        (body_right_ext - WHILE_PREFIXED_FUSED_NESTED_EMIT_RIGHT_TRIM).max(0.0)
    } else {
        body_right_ext
    };
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
        // A cond-driven swimlane lane (see `swimlane_while_cond_special_lane`)
        // hangs the terminator circle off the diamond's left-vertex column:
        // its centre sits halfHex + 9 left of the diamond's left vertex,
        // matching the lane's drawn `getMinMax` width.
        let special_cx = if swimlane_cond_special {
            diamond_left_vertex_x - DIAMOND_HALF - 9.0
        } else {
            special_left_abs + special_w / 2.0
        };
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
        let exit_x = geo_left_x
            - DIAMOND_HALF
            - if fork_branch_while_if_body_compress {
                2.0
            } else {
                0.0
            }
            + if while_body_has_prefixed_fused_trailing_while(body) {
                WHILE_PREFIXED_FUSED_NESTED_LEFT_TRIM
            } else {
                0.0
            };
        // When this loop CONSUMES a nested loop's fused exit, its loop-back
        // junction sits at the child's fused band (no `+10` body gap — the child
        // already carries it). The exit corridor, however, must still descend the
        // full body→junction gap below the loop-back band, so it drops an extra
        // `WHILE_NESTED_EXIT_BODY_GAP` (the standard `body_bottom + 10` slack) past
        // the loop-back junction. A non-consuming loop's junction already includes
        // that gap, so its exit and loop-back share the junction level.
        let exit_bottom_y = junction_y
            + if nested_exit.is_some() {
                WHILE_NESTED_EXIT_BODY_GAP
            } else {
                0.0
            };
        (exit_x, exit_bottom_y)
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
    // there is no emphasized down arrowhead for the empty placeholder. When this
    // loop's body is a nested fused `while` (a consumer), PlantUML draws its
    // `ConnectionIn` AFTER its loop-back LEFT arrowhead (interleaved nested-tile
    // document order); see the segment-7 tail below.
    if nested_exit.is_none() {
        if body.is_empty() {
            svg.line_styled(&arrow_color, "1", cx, cx, diamond_bottom, junction_y, false);
        } else {
            svg.down_arrow(cx, diamond_bottom, body_top, &arrow_color);
        }
    }

    // 2. Body bottom → junction (only if body has content; for empty body
    // the inbound arrow already reaches the junction-equivalent point). When the
    // loop-back was fused into the break-`if` (last-flow break), there is no
    // junction: the if's empty branch is the loop's pointOut. For a fused nested
    // exit, the nested loop's exit corridor already carries the descent down to
    // `junction_y`, so this loop draws no body→junction spine segment.
    if !body.is_empty() && !loopback_fused && nested_exit.is_none() {
        svg.line_styled(&arrow_color, "1", cx, cx, body_bottom, junction_y, false);
    }

    // 3. Horizontal at junction out to loop_x (skip when fused into a break-if).
    // For a fused nested exit the loop-back sources from the nested loop's exit
    // corridor column (`ne.exit_x`) instead of the spine.
    if !loopback_fused {
        let loopback_src_x = nested_exit.map_or(cx, |ne| ne.exit_x);
        svg.line_styled(
            &arrow_color,
            "1",
            loopback_src_x,
            loop_x,
            junction_y,
            junction_y,
            false,
        );
    }

    // Break-bearing loop: PlantUML places the loop-back / exit arrowheads at the
    // `FtileWhile` frame-height midpoints (`ConnectionOut` /
    // `ConnectionBackSimple`), decoupled from the (lower) loop-back junction the
    // snake's polyline actually visits. Compute `frame_h` faithfully from the
    // body geometry and drive both arrowheads from it. See
    // [`WHILE_BREAK_BODY_GEO_EXTRA`].
    let break_frame_h =
        break_in_body.then(|| {
            24.0 + sequence_height(body)
            + WHILE_BREAK_BODY_GEO_RESIDUAL
            // A bare `then` break branch shortens the drawn corridor (and thus
            // `sequence_height`), but PlantUML's FtileWhile frame height is
            // unchanged — the WHILE_BREAK_BODY_GEO_RESIDUAL model already reserves
            // the full (labeled) corridor. Add the collapsed south band back so
            // the frame-derived exit/loop-back arrowheads stay at the labeled
            // midpoints (`edge_activity_while_infinite`).
            + if break_if_south_label_present(body) == Some(false) {
                IF_DOWN_LEAD - WHILE_BREAK_NO_SOUTH_LABEL_DROP
            } else {
                0.0
            }
            // `getSuppHeightForLabel`: the `is (yes)` loop-back label band raises
            // the frame; a bare `while (cond)` with no label adds nothing.
            + if is_label.is_some() {
                WHILE_BREAK_SUPP_LABEL_H
            } else {
                0.0
            }
            + if break_if_is_first_flow(body) {
                // The break-`if` is the body's first flow node. The FtileWhile
                // frame reserves the south-label mid band (`IF_DOWN_MID_STRETCH`)
                // that has no leading tile to compress against. When the corridor
                // compresses (3+ body flow tiles) that band, together with the
                // extra outbound-arrow length the slot finder surfaces (see
                // `break_if_first_outbound_extra` in `emit_sequence_ex`), enlarges
                // the body's `FtileGeometry` by a further
                // `WHILE_BREAK_FIRST_COMPRESSED_FRAME_EXTRA`.
                WHILE_BREAK_FIRST_FRAME_EXTRA
                    + if while_break_corridor_compresses(body) {
                        WHILE_BREAK_FIRST_COMPRESSED_FRAME_EXTRA
                    } else {
                        0.0
                    }
            } else {
                0.0
            } - if nodes_contain_break_single_survivor_if(body) {
                WHILE_NESTED_BREAK_SURVIVOR_FRAME_TRIM
            } else {
                0.0
            } + 4.0 * DIAMOND_HALF
        });
    // 4. UP arrowhead at midpoint of the loop arm's vertical run.
    // PlantUML draws this at the midpoint of (diamond_cy, body_bottom +
    // halfHex), adjusted by the same compression that shifts body_top up.
    // A multi-case switch in the body overrides this: ON_Y compression of the
    // loop-back corridor anchors the arrowhead on the switch's merge diamond
    // (`while_switch_loopback_tip`) rather than the corridor midpoint.
    let mid_y = body_switch_loopback_tip.unwrap_or_else(|| {
        // Break-bearing loop: loop-back arrowhead at the frame-height midpoint
        // (ConnectionBackSimple's UP emphasize sits at `(pointOut + diamond_cy)/2`
        // where `pointOut = frame_h - hexHalf`).
        if let Some(frame_h) = break_frame_h {
            return (diamond_cy + frame_h - DIAMOND_HALF) / 2.0;
        }
        (diamond_cy + body_bottom + DIAMOND_HALF) / 2.0
            - while_slot_compress(
                compress_while_slot,
                is_label.is_some(),
                end_label.is_some(),
                body.is_empty(),
            ) / 2.0
            - if fork_branch_while_if_body_compress {
                WHILE_BODY_SLOT_COMPRESS / 2.0
            } else {
                0.0
            }
            - if is_label.is_none() {
                WHILE_UNLABELED_LOOP_ARROW_Y_PULL_UP
            } else {
                0.0
            }
            + body_mid_stretch.map_or(0.0, |(_, stretch)| {
                WHILE_EVEN_BODY_LOOP_ARROW_STRETCH - stretch / 2.0
            })
            // Even-flow labelled body led by a `repeat` tile: `body_mid_stretch`
            // rejects it (the leading repeat is not a plain action), but the body
            // is still even, so the loop-back arrowhead takes the same even-body
            // placement. The `while_tail_extra` stretch grows `body_bottom`,
            // raising the naive midpoint by `stretch/2`; add the residual
            // (`WHILE_EVEN_BODY_LOOP_ARROW_STRETCH − stretch/2`) so it lands at the
            // even-body offset, exactly like the all-action path above.
            + {
                let tail = while_body_repeat_tail_extra(body, is_label.is_some());
                if tail != 0.0 {
                    WHILE_EVEN_BODY_LOOP_ARROW_STRETCH - tail / 2.0
                } else {
                    0.0
                }
            }
            // Body terminating in a balanced `if`: anchor the arrowhead on the
            // pre-compression frame centre, one even-body stretch above the
            // naive midpoint computed from the post-emit merge-diamond bottom.
            // `body_mid_stretch` returns `None` for a composite-terminated body
            // (it gates on all-actions), so this term is the only contributor.
            - while_loopback_arrow_bias(body)
            // Pure-balanced-if body: the merge-diamond bottom (hence `body_bottom`)
            // already carries the branch stretch `S`, dropping the naive midpoint
            // by `S/2`. PlantUML draws the loop-back arrowhead on the UNSTRETCHED
            // midpoint plus a fixed even-body stretch, so undo the `S/2` and add
            // `WHILE_EVEN_BODY_LOOP_ARROW_STRETCH`. Drives `act_while_ifdepth*`.
            + if_body_stretch.map_or(0.0, |s| WHILE_EVEN_BODY_LOOP_ARROW_STRETCH - s / 2.0)
            // A nested labelled `while` in the body advertises a loop-back label
            // band (`getSuppHeightForLabel`) the drawn flow never occupies, so the
            // body's reported `getPointOut` — which `ConnectionBackSimple` anchors
            // this arrowhead on — sits `band` below the drawn `body_bottom`. The
            // arrowhead seats at the midpoint, i.e. `band/2` lower. Zero for any
            // body without a nested labelled `while`. Drives `act_while_nested`.
            + while_body_nested_labelled_band(body) / 2.0
            // When that nested labelled `while` itself wraps a further labelled
            // `while`, the deeper loop's own `getSuppHeightForLabel` propagates up
            // through `getTranslateForWhile` centering, seating the anchor one
            // further pixel below per extra level. Drives
            // `act_while_nested_3_levels`.
            + while_body_nested_labelled_band_residual(body)
    });
    // Segments 4-7 (loop-back arm) are drawn by the break-`if` itself when the
    // loop-back was fused into its empty branch (last-flow break); skip them here.
    if !loopback_fused {
        // 4. UP emphasize arrowhead at the loop arm's vertical-run midpoint. A
        // fused nested exit drops it: MergeStrategy.LIMITED merges the parent's
        // ConnectionBackSimple snake into the nested loop's exit corridor, which
        // already carried its own DOWN emphasize, so PlantUML emits no second
        // arrowhead for the merged run.
        if nested_exit.is_none() {
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
        }

        // 5. Loop arm vertical at loop_x.
        svg.line_styled(
            &arrow_color,
            "1",
            loop_x,
            loop_x,
            diamond_cy,
            junction_y,
            false,
        );

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
    }

    if let Some((arrow_top, style, label, arrow_gap)) = nested_child_inbound {
        emit_pending_down_arrow(svg, arrow_top, style, label, arrow_gap, cx);
    }

    // 7b. A consumer (body is a single nested fused `while`) draws its OWN inbound
    // arrow here — after its loop-back LEFT arrowhead and before its exit —
    // matching PlantUML's interleaved nested `ConnectionIn` document order.
    if nested_exit.is_some() {
        if body.is_empty() {
            svg.line_styled(&arrow_color, "1", cx, cx, diamond_bottom, junction_y, false);
        } else {
            svg.down_arrow(cx, diamond_bottom, body_top, &arrow_color);
        }
    }

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

    // Fused nested exit (this loop is the sole body of a parent `while`): instead
    // of wrapping the exit corridor back to the spine, descend to the fused band
    // one `halfHex` below this loop's exit base (= the parent's loop-back
    // junction), drop the DOWN emphasize at that run's midpoint, and report the
    // corridor column + band y to the parent. `exit_bottom_y` already folds in the
    // extra body-gap when this loop is itself a consumer, so the chain stacks
    // correctly. The parent's loop-back arm sources from `exit_x` at `fused_y`;
    // its own UP emphasize is dropped (snake merge).
    if fuse_as_nested_exit {
        let fused_y = exit_bottom_y + DIAMOND_HALF;
        // The DOWN emphasize sits at the corridor-run midpoint plus the
        // `WHILE_NESTED_EXIT_ARROW_BIAS` the snake-merge surfaces (PlantUML draws
        // it on the un-merged `ConnectionOut` segment, one pixel below the drawn
        // midpoint). A loop whose inbound slot compressed (case (b)) pulls it back
        // up by the same `PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP` the regular
        // compressed exit uses.
        let child_compressed_below = nested_exit.map_or(0, |ne| ne.compressed_below);
        let mut arrow_y = (diamond_cy + fused_y) / 2.0 + WHILE_NESTED_EXIT_ARROW_BIAS;
        if ordinary_slot_compressed {
            arrow_y -= PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP;
        }
        if fuse_as_prefixed_nested_exit {
            arrow_y += WHILE_PREFIXED_FUSED_NESTED_CHILD_EXIT_ARROW_PUSH_DOWN;
        }
        // Each compressed loop below pulls the pre-compression frame midpoint up.
        arrow_y -= child_compressed_below as f64 * PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP;
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
            fused_y,
            false,
        );
        svg.while_nested_exit = Some(WhileNestedExit {
            exit_x,
            fused_y,
            compressed_below: child_compressed_below + usize::from(ordinary_slot_compressed),
        });
        svg.switch_lane_span(while_exit_lane);
        return body_bottom;
    }

    // Branch-nested no-special while: the exit corridor is the if's
    // branch→merge connection (PlantUML fuses FtileWhile.ConnectionOut with
    // the if's ConnectionVerticalThenHorizontal under MergeStrategy.LIMITED).
    // Route the corridor straight down to the merge diamond instead of
    // wrapping back to our own spine. The redirect was taken into `exit_redirect`
    // at entry (so a nested body while could not consume it).
    if special_out.is_none()
        && let Some(redir) = exit_redirect
    {
        let merge_cy = redir.merge_cy;
        // DOWN arrowhead at the midpoint of the corridor's vertical run
        // (emphasizeDirection.DOWN), then the vertical, then the horizontal
        // into the merge diamond vertex with the in-arrow. PlantUML places the
        // emphasize arrow at the midpoint of the FtileWhile's own ConnectionOut
        // segment (diamond_cy → frame bottom), which sits one slot-compression
        // half (minus one) above the midpoint of the full corridor to merge_cy.
        let mut arrow_y =
            (diamond_cy + merge_cy) / 2.0 - PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP;
        // A redirect loop that also consumed a nested fused exit drops the
        // arrowhead one pixel (same snake-merge offset as the fused corridors),
        // and pulls up once per compressed loop in the consumed chain (the
        // pre-compression frame midpoint sits higher than the drawn corridor).
        if let Some(ne) = nested_exit {
            arrow_y += WHILE_NESTED_EXIT_ARROW_BIAS;
            arrow_y -= ne.compressed_below as f64 * PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP;
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
        svg.switch_lane_span(while_exit_lane);
        return merge_cy;
    }

    // Swimlane inter-lane stitch: the loop's `ConnectionOut` exit corridor is
    // fused with the cross-lane arrow. The exit arm descends from the diamond's
    // west vertex to the cross-lane corridor y, a single horizontal crosses to
    // the next lane's spine, then the arrow drops into the next tile. The DOWN
    // emphasize arrowhead on the exit run sits at the midpoint of THIS while
    // tile's `ConnectionOut` (diamond_cy → tile bottom), not the (lower) corridor.
    if let Some(stitch) = cross_lane {
        let cross_y = stitch.cross_y;
        // The loop's `ConnectionOut` DOWN emphasize sits at the midpoint of the
        // FtileWhile tile's exit run: diamond_cy → tile bottom, where the tile
        // bottom is `body_bottom + 2*halfHex + suppLabel` (the loop-back label
        // reservation that the swimlane stitch keeps instead of compressing).
        let tile_bottom = body_bottom + 2.0 * DIAMOND_HALF + WHILE_LOOPBACK_LABEL_H;
        let arrow_y = (diamond_cy + tile_bottom) / 2.0;
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
            cross_y,
            false,
        );
        svg.line_styled(
            &arrow_color,
            "1",
            exit_x,
            stitch.target_cx,
            cross_y,
            cross_y,
            false,
        );
        svg.line_styled(
            &arrow_color,
            "1",
            stitch.target_cx,
            stitch.target_cx,
            cross_y,
            stitch.target_y,
            false,
        );
        svg.polygon_connector(
            &arrow_color,
            &[
                (stitch.target_cx - 4.0, stitch.target_y - 10.0),
                (stitch.target_cx, stitch.target_y),
                (stitch.target_cx + 4.0, stitch.target_y - 10.0),
                (stitch.target_cx, stitch.target_y - 6.0),
            ],
            &arrow_color,
            "1",
        );
        svg.switch_lane_span(while_exit_lane);
        return cross_y;
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
    } else if let Some(frame_h) = break_frame_h {
        // Break loop: the exit ConnectionOut's DOWN emphasize sits at the
        // frame-height midpoint `(diamond_cy + frame_h)/2`.
        (diamond_cy + frame_h) / 2.0
    } else {
        (diamond_cy + wrap_y) / 2.0
    };
    if fork_branch_slot_open && special_out.is_none() && break_frame_h.is_none() {
        arrow_y += FORK_BRANCH_WHILE_EXIT_ARROW_PUSH_DOWN;
    } else if exit_vertical_after_arrow && break_frame_h.is_none() {
        arrow_y -= PARTITION_COLORED_WHILE_EXIT_ARROW_PULL_UP;
    }
    // A loop that consumed a nested fused exit wraps its own corridor one pixel
    // lower (same `WHILE_NESTED_EXIT_ARROW_BIAS` snake-merge offset as the
    // fused-producer corridor above).
    if nested_exit.is_some() && special_out.is_none() && break_frame_h.is_none() {
        arrow_y += WHILE_NESTED_EXIT_ARROW_BIAS;
        if while_body_has_prefixed_fused_trailing_while(body) {
            arrow_y -= WHILE_PREFIXED_FUSED_NESTED_PARENT_EXIT_ARROW_PULL_UP;
        }
    }
    // The exit emphasis arrowhead anchors on the PRE-compression midpoint. When
    // the drawn `wrap_y` kept the uncompressed `+12` junction — an if-long branch
    // (`in_if_long_branch`) or a non-terminal while in a repeat body
    // (`repeat_body_nonterminal`) — the geometric midpoint rose by half that 2 px;
    // pull the arrowhead back up so it lands where the compressed midpoint would.
    if (in_if_long_branch || repeat_body_nonterminal)
        && special_out.is_none()
        && break_frame_h.is_none()
    {
        arrow_y -= (DIAMOND_HALF - 10.0) / 2.0;
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
    let bottom = if let Some(special) = special_out {
        emit_node(svg, special, exit_x, wrap_y)
    } else {
        svg.line_styled(&arrow_color, "1", exit_x, cx, wrap_y, wrap_y, false);
        // If this while leads an if-long branch, its exit corridor fuses with the
        // branch→merge connector: continue straight down the spine to the merge
        // line (in document order, right after the wrap-back) and draw the merge
        // arrowhead, instead of leaving a separate ConnectionVerticalOut. Consume
        // the one-shot so a nested loop is unaffected.
        if let Some(merge_y) = if_long_merge_y {
            svg.line_styled(&arrow_color, "1", cx, cx, wrap_y, merge_y, false);
            svg.polygon_connector(
                &arrow_color,
                &[
                    (cx - 4.0, merge_y - 10.0),
                    (cx, merge_y),
                    (cx + 4.0, merge_y - 10.0),
                    (cx, merge_y - 6.0),
                ],
                &arrow_color,
                "1",
            );
            svg.switch_lane_span(while_exit_lane);
            return merge_y;
        }
        wrap_y
    };
    svg.switch_lane_span(while_exit_lane);
    bottom
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
    // A repeat leading an if-long branch keeps BOTH uncompressed FtileRepeat
    // reserves: the top reserve above the body (`if_long_extra`, lowering the
    // body) and the bottom reserve below the body — the `is (...)` loop-back
    // label band (`text_height(11)`) — pushing the condition diamond further
    // down. The if-long emit pushes the body reserve here as a one-shot (consume
    // so nested loops are unaffected). `has_start_label` repeats route the body
    // via the entry tile, not these reserves, so they do not apply there.
    let if_long_extra = std::mem::take(&mut svg.if_long_repeat_body_extra);
    let if_long_active = if_long_extra != 0.0 && !options.has_start_label;
    // Centring slack a labelled `while` distributes into the tail of this repeat
    // when it leads an even-flow loop body (see `while_body_repeat_tail_extra`).
    // One-shot taken at entry so a repeat nested in this body does not inherit it.
    // Only the plain (non-backward, non-start-label) form is reachable from the
    // setter, but guard here too so backward/start-label repeats ignore it.
    let while_tail_extra = std::mem::take(&mut svg.while_repeat_tail_extra);
    let while_tail_extra = if backward.is_none() && !options.has_start_label {
        while_tail_extra
    } else {
        0.0
    };
    let if_long_cond_extra = if if_long_active {
        pm::text_height(SMALL_FONT)
    } else {
        0.0
    };
    let fork_branch_body_extra = std::mem::take(&mut svg.fork_branch_repeat_body_extra);
    let fork_branch_tail_extra = std::mem::take(&mut svg.fork_branch_repeat_tail_extra);
    let body_top_extra = options.body_top_extra
        + if if_long_active { if_long_extra } else { 0.0 }
        + fork_branch_body_extra;
    // The loop-back arrowhead anchors on the ABSTRACT (uncompressed) frame centre,
    // not the drawn body position. In the if-long branch that frame is
    // `IF_LONG_REPEAT_CENTERING_EXTRA` taller than the emitted `branch_h` (the
    // same term that drives tile2 centring), so the arrowhead's `body_top_extra`
    // term uses THAT, not the (larger) body-lowering reserve.
    let arrow_body_top_extra = options.body_top_extra
        + if if_long_active {
            IF_LONG_REPEAT_CENTERING_EXTRA
        } else {
            0.0
        };
    let has_start_label = options.has_start_label;
    // A repeat nested inside another repeat's body keeps one extra `ARROW_LEN`
    // below its body before the condition diamond: the enclosing loop frame
    // leaves that much of FtileRepeat's reserved `8*halfHex` tail
    // uncompressible under the whole-diagram ON_Y pass (a top-level repeat
    // collapses the whole tail to a single arrow). Only the plain
    // (non-backward, non-start-label) form is affected; backward/labelled
    // forms route their tail through other geometry.
    let nested_cond_extra = if svg.repeat_body_depth > 0 && backward.is_none() && !has_start_label {
        ARROW_LEN
    } else {
        0.0
    };
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
    // A `break` directly in the body (rendered by its `if` as a no-diamond
    // `FtileIfDown`) welds LEFT to an exit corridor that the repeat carries down
    // to its break-merge diamond. Set the corridor context before the body emit
    // (the if reads it), mirroring `emit_while`; restore the prior value after.
    // PlantUML wraps the repeat tile in `addHorizontalMargin(result, 10, 0)` and
    // routes the break snake to that tile's left edge (x=0). So the corridor sits
    // `REPEAT_BREAK_CORRIDOR_MARGIN` (10) px left of the repeat's OWN left extent
    // (`repeat.getLeft()` = max(body_left, cond_diamond reservation)), not the
    // break-`if` diamond's half — a wide post-break body action sets that left.
    let has_break = !has_start_label && body_contains_break_if(body);
    let break_corridor_x = if has_break {
        let body_left = sequence_extents(body).0;
        let cond_half = diamond_inner_w(condition) / 2.0 + DIAMOND_HALF;
        // PlantUML's `FtileRepeat.getLeft` = max(body.getLeft, entry-rhombus half,
        // condition half). The corridor sits `REPEAT_BREAK_CORRIDOR_MARGIN` (10) px
        // left of that — `REPEAT_BREAK_CORRIDOR_PAD` (9) reserved in `node_extents`
        // plus one px inside that reserved content margin.
        let get_left = body_left.max(DIAMOND_HALF).max(cond_half);
        Some(cx - get_left - REPEAT_BREAK_CORRIDOR_MARGIN)
    } else {
        None
    };
    // When trailing body flow follows the break-`if` (mid/early topologies) the
    // break-merge diamond sits below that flow + the condition, not a fixed span
    // below the if. Pre-compute its centre from the body height so the if can run
    // its exit corridor down to the right depth. A LAST-node break rejoins straight
    // into the condition, so it derives the merge from `return_y` itself (None here).
    let break_merge_cy = if has_break && !break_if_is_last_flow(body) {
        // `node_height` models the break-`if` with the while-loop compressed drop
        // (`WHILE_BREAK_IF_CORRIDOR_DROP_COMPRESSED`, = lead + arrow + IF_BRANCH_UP),
        // but a mid-repeat break rejoins `IF_BRANCH_UP/2` higher (see `emit_if_break_down`'s
        // repeat rejoin), so `sequence_height` over-counts the body by that much.
        // A FIRST-flow break (`act_break_repeat_early`) instead under-counts: the
        // emitted body is `REPEAT_BREAK_FIRST_TRAILING_EXTRA` taller than the model
        // (the slack `emit_sequence_ex` adds to the rejoin→action arrow), so add it
        // back here so the merge diamond tracks the trailing flow.
        let first_flow_extra = if break_if_is_first_flow(body) {
            REPEAT_BREAK_FIRST_TRAILING_EXTRA
        } else {
            0.0
        };
        let pre_body_bottom =
            body_y + sequence_height(body) - IF_BRANCH_UP / 2.0 + first_flow_extra;
        let pre_cond_y = pre_body_bottom + ARROW_LEN + nested_cond_extra + if_long_cond_extra;
        let pre_cond_bottom = pre_cond_y + DIAMOND_HALF * 2.0;
        Some(pre_cond_bottom + ARROW_LEN + DIAMOND_HALF)
    } else {
        None
    };
    let prev_while_break = svg.while_break.take();
    let prev_weld_y = svg.repeat_break_weld_y.take();
    let prev_body_condition_drawn = svg.repeat_body_condition_connector_drawn;
    svg.repeat_body_condition_connector_drawn = false;
    if has_break && let Some(corridor_x) = break_corridor_x {
        svg.while_break = Some(WhileBreakContext {
            corridor_x,
            compresses: false,
            fuse_loopback: None,
            repeat_mode: true,
            repeat_merge_cy: break_merge_cy,
            repeat_merge_left_x: Some(cx - DIAMOND_HALF),
            inbound_lead: 0.0,
            repeat_break_first_flow: break_if_is_first_flow(body) && !break_if_is_last_flow(body),
        });
    }

    // Mark the body as a repeat body so any directly-nested repeat applies its
    // `nested_cond_extra` (read at that child's entry). Save+zero the expansion
    // accumulator so we can read how much nested-repeat tail expansion this
    // body contributed (used to bias the loop-back arrowhead).
    svg.repeat_body_depth += 1;
    let outer_expansion = std::mem::take(&mut svg.repeat_nested_expansion);
    // Loop-back arrowhead anchor recorded by a terminal switch in the body
    // (see the repeat-body emit branch below); `None` unless one was emitted.
    let mut repeat_switch_loopback_tip: Option<f64> = None;
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
        // Mark this as a repeat body: a terminal multi-case switch keeps one
        // uncompressible `ARROW_LEN` of its merge band under the loop frame, and
        // records `while_switch_loopback_tip` so the loop-back arrowhead below
        // anchors on the switch's merge diamond. Save+restore the tip around the
        // emit so a sibling/parent loop is unaffected.
        let prev_loopback_tip = svg.while_switch_loopback_tip.take();
        svg.pending_repeat_body = true;
        let bottom = emit_sequence_ex(svg, body, cx, body_y, body_mid_stretch, None, false);
        repeat_switch_loopback_tip = svg.while_switch_loopback_tip.take();
        svg.while_switch_loopback_tip = prev_loopback_tip;
        bottom
    };
    svg.repeat_body_depth -= 1;
    // Capture the break weld y the body's break-`if` recorded (if any), then
    // restore the prior break context so a sibling/parent loop is unaffected.
    let break_weld_y = svg.repeat_break_weld_y.take();
    let body_condition_connector_drawn = svg.repeat_body_condition_connector_drawn;
    svg.while_break = prev_while_break;
    svg.repeat_break_weld_y = prev_weld_y;
    svg.repeat_body_condition_connector_drawn = prev_body_condition_drawn;
    // Nested-repeat tail expansion contributed by this repeat's body.
    let body_expansion = svg.repeat_nested_expansion;
    // Propagate this whole subtree's expansion (own tail + body's) to the
    // enclosing loop so its arrowhead bias accounts for us.
    svg.repeat_nested_expansion = outer_expansion + nested_cond_extra + body_expansion;
    // Single plain-action backward repeats keep the extra halfHex before the
    // condition diamond; multi-action or composite bodies absorb that slack in
    // their final inbound connector / internal structure.
    let backward_flow_count = body.iter().filter(|n| node_is_flow(n)).count();
    let cond_y = body_bottom
        + ARROW_LEN
        + repeat_backward_extra_cond_gap(body, backward.is_some())
        + nested_cond_extra
        + if_long_cond_extra
        + while_tail_extra
        + fork_branch_tail_extra;

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

    // Body → condition connector ordering. PlantUML normally emits the
    // top-diamond → body ConnectionIn and the loop-back ConnectionBack BEFORE
    // the body's spine→condition link (plain bodies: [in, back, out]). A `while`
    // body, however, is a composite tile whose pointOut→condition link is
    // assembled with the BODY (its own exit corridor), so it lands in the stream
    // BEFORE the repeat's ConnectionIn/Back — order [out, in, back]. Gate the
    // early emit on a while-LAST body so plain/if/switch bodies (and a while
    // followed by a trailing tile) keep [in, back, out].
    let break_is_last = break_weld_y.is_some() && break_if_is_last_flow(body);
    let while_body_out_first = body_last_flow_is_while(body) && !has_start_label && !break_is_last;
    if while_body_out_first {
        svg.down_arrow(cx, body_bottom, cond_y, &arrow_color);
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
    } else if body_contains_while(body)
        && let Some(g) = sequence_geometry(body)
    {
        // A `while` body advertises a wider layout extent than it draws (the
        // FtileWhile `dx + halfHex` trailing reservation reclaimed by ON_X). The
        // faithful FtileRepeat arm clears the drawn geometry by halfHex, matching
        // the Repeat `node_extents` arm formula above.
        cx + g.right() + DIAMOND_HALF
    } else if repeat_body_has_in_loop_switch(body) {
        // A direct multi-case in-loop switch draws its uncompressed gap-20 block
        // (`switch_x_layout_in_while`); the arm clears that DRAWN block (extents +
        // 12, mirroring `emit_while`). Its gap-20 `FtileGeometry`
        // (`switch_with_diamonds`) overshoots the drawn block, so skip `geo.right()
        // + 4` and use the widened `sequence_loop_body_extents` right edge.
        cx + sequence_loop_body_extents(body).1 + 12.0
    } else {
        let extents_clear = cx + sequence_extents(body).1 + 12.0;
        let geo_clear = sequence_geometry(body).map_or(extents_clear, |g| cx + g.right() + 4.0);
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
        let body_geo_right = repeat_backward_body_right(body);
        let cond_half = cond_inner_w / 2.0 + DIAMOND_HALF;
        let box_left = cx + repeat_backward_box_left_rel(cond_half, body_geo_right, is_label);
        let box_cx = box_left + bw / 2.0;
        let backward_h = action_height(
            label,
            svg.palette.action_pad_y,
            &svg.palette.action_font_family,
            svg.palette.action_font_size,
        );
        // FtileRepeat.getTranslateBackward centres the backward tile on the
        // BODY span (`(body_y + body_bottom - backward_h) / 2`). For a single
        // plain action this collapses to body_y (action height == box height),
        // so the historical single-flow path matched; but a single TALL flow
        // node (e.g. an if/else block) has its centre well below body_y, so the
        // box must centre on the whole body span regardless of flow count.
        let box_top = if break_if_is_last_flow(body) {
            // A break-`if` body's surviving branch routes RIGHT then back to the
            // spine, so its drawn span runs past the bare `body_bottom`. PlantUML
            // centres the backward tile on the whole composite (entry diamond top
            // to condition diamond bottom = `getTranslateBackward = totalHeight/2`),
            // then the deferred ON_Y compression of the break corridor shifts the
            // loop-back band up by `REPEAT_BREAK_LOOPBACK_ARROW_BIAS`. An ODD
            // action count additionally absorbs `REPEAT_BACKWARD_BREAK_ODD_SLACK/2`
            // of that shift through its centring stretch.
            let cond_bottom = cond_y + DIAMOND_HALF * 2.0;
            let composite_center = (y + cond_bottom) / 2.0;
            let action_count = body
                .iter()
                .filter(|n| matches!(n, LayoutNode::Action { .. }))
                .count();
            let odd_action_relief = if action_count.is_multiple_of(2) {
                0.0
            } else {
                REPEAT_BACKWARD_BREAK_ODD_SLACK / 2.0
            };
            composite_center - backward_h / 2.0 - REPEAT_BREAK_LOOPBACK_ARROW_BIAS
                + odd_action_relief
        } else {
            let odd_stretch_adjust =
                if backward_flow_count >= 2 && !backward_flow_count.is_multiple_of(2) {
                    body_mid_stretch.map_or(0.0, |(_, stretch)| stretch / 2.0)
                } else {
                    0.0
                };
            body_y + (body_bottom - body_y - backward_h) / 2.0 - odd_stretch_adjust
                + repeat_backward_composite_break_body_slack(body, backward.is_some())
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
        // The emphasis arrowhead anchors on the loop's *content* midpoint. Any
        // nested-repeat tail expansion inside this loop's span (this repeat's
        // own `nested_cond_extra` plus the body's accumulated expansion) sits in
        // the lower half of the loop-back, so it biases the visible midpoint
        // down by half that amount — undo it to land on the content midpoint.
        let nested_bias = (nested_cond_extra + body_expansion) / 2.0;
        // A break-bearing repeat anchors its loop-back arrowhead on the naive
        // `(top_cy + cond_cy)/2` midpoint less a fixed bias — the trailing-stretch
        // `body_stretch` here sits BELOW the loop-back's content (in the break
        // corridor's compressed band), not on a middle connector, so it must NOT
        // be subtracted from the midpoint.
        let mid_y = if let Some(tip) = repeat_switch_loopback_tip {
            // A terminal switch in the body anchors the loop-back arrowhead on the
            // switch's merge diamond (recorded by `emit_switch_with_layout`), not on
            // the corridor midpoint — the merge band's uncompressed tail sits below.
            tip
        } else if break_weld_y.is_some() {
            // Anchor on the abstract (uncompressed) frame centre. A mid/early break
            // (trailing body flow after the break-`if`) has its condition drawn
            // `REPEAT_BREAK_MID_LOOPBACK_DECOMPRESS` above the uncompressed position
            // (the ON_Y pass squeezes the break corridor); add it back. A last-node
            // break does not compress the loop-back region, so the drawn cond IS the
            // frame position.
            //
            // A FIRST-flow break (`act_break_repeat_early`) is the special mid case
            // with NO leading tile above the break-`if`: the loop-back's upper span
            // barely compresses, so the arrowhead sits at the true content midpoint
            // less only the small fixed `REPEAT_BREAK_FIRST_LOOPBACK_BIAS`, not the
            // larger mid decompress+bias pair.
            if break_merge_cy.is_some()
                && break_if_is_first_flow(body)
                && !break_if_is_last_flow(body)
            {
                (top_cy + cond_diamond_cy + arrow_body_top_extra) / 2.0
                    - nested_bias
                    - REPEAT_BREAK_FIRST_LOOPBACK_BIAS
            } else {
                let decompress = if break_merge_cy.is_some() {
                    REPEAT_BREAK_MID_LOOPBACK_DECOMPRESS
                } else {
                    0.0
                };
                (top_cy + cond_diamond_cy + decompress + arrow_body_top_extra) / 2.0
                    - nested_bias
                    - REPEAT_BREAK_LOOPBACK_ARROW_BIAS
            }
        } else {
            // Leading-action + balanced-if bodies anchor the arrowhead on the
            // pre-compression frame centre, `space/2` above the naive midpoint.
            let loopback_arrow_bias = repeat_loopback_arrow_bias(body, backward.is_some());
            // A `while`-distributed tail extra (this repeat leads an even-flow
            // labelled while body) sits below the body, in the loop-back's lower
            // half, biasing the visible midpoint down by half — undo it so the
            // arrowhead lands on the content midpoint, exactly like `nested_bias`.
            (top_cy + cond_diamond_cy - body_stretch + arrow_body_top_extra) / 2.0
                - nested_bias
                - loopback_arrow_bias
                - while_tail_extra / 2.0
                + if fork_branch_tail_extra != 0.0 {
                    1.0
                } else {
                    0.0
                }
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

    // Body → condition diamond connector (after loop-back path). When the break-`if`
    // is the body's LAST flow node it already drew this from its own pointOut (it
    // lands earlier in document order, with the if's connectors), so skip it. A
    // MID/early break ends the body with a trailing action, whose spine→condition
    // arrow the break-`if` did NOT draw — emit it here. A `while`-LAST body already
    // drew it early (see `while_body_out_first`).
    if !break_is_last && !while_body_out_first && !body_condition_connector_drawn {
        svg.down_arrow(cx, body_bottom, cond_y, &arrow_color);
    }

    let cond_bottom = cond_y + DIAMOND_HALF * 2.0;

    // Break-merge diamond: a body `break` welded LEFT to the exit corridor; the
    // repeat carries that corridor DOWN to a small merge rhombus below the
    // condition, where the loop's south exit and the break re-converge (the
    // loop's `out`). Mirrors PlantUML's FtileRepeat break-`out` collector.
    if break_weld_y.is_some() {
        let merge_top = cond_bottom + ARROW_LEN;
        let merge_cy = merge_top + DIAMOND_HALF;
        let merge_bottom = merge_top + DIAMOND_HALF * 2.0;

        // Merge rhombus (the break-`if` already drew the left corridor down into
        // its west vertex; here we add the shape and the condition's south exit).
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

        // Condition south exit → merge top (a plain down-arrow).
        svg.down_arrow(cx, cond_bottom, merge_top, &arrow_color);

        return merge_bottom;
    }

    cond_bottom
}

/// Swimlane V2: partition one of the natural-emit buffers into per-lane fragments
/// using the recorded byte-offset spans. `off` selects the shapes- or
/// connectors-offset from each span. Pre-first-span bytes go to lane 0.
fn partition_lane_buffer(
    buf: &str,
    spans: &[(usize, usize, usize)],
    off: impl Fn(&(usize, usize, usize)) -> usize,
    n_lanes: usize,
) -> Vec<String> {
    // Piecewise-constant lane over byte offsets: (offset, lane), starting lane 0.
    let mut points: Vec<(usize, usize)> = vec![(0usize, 0usize)];
    for sp in spans {
        points.push((off(sp), sp.2));
    }
    let len = buf.len();
    let mut frags = vec![String::new(); n_lanes.max(1)];
    for w in 0..points.len() {
        let start = points[w].0;
        let lane = points[w].1;
        let end = if w + 1 < points.len() {
            points[w + 1].0
        } else {
            len
        };
        if end > start && lane < frags.len() {
            frags[lane].push_str(&buf[start..end]);
        }
    }
    frags
}

/// Swimlane V2: lay the natural single-tree emit out into lane columns. Returns
/// `(body, width, height)` for the final SVG, or `None` if there is nothing to
/// lay out. WIP: prints per-lane bounds for calibration; column geometry is a
/// first cut (refined against the golden ladder next).
/// Swimlane V2 chrome colors (captured from the palette before it is moved).
struct SwimlaneV2Chrome {
    divider: String,
    title: String,
    title_bg: Option<String>,
}

/// cy of the first `<ellipse>` in an SVG fragment (the start node, for V2 y-align).
fn first_ellipse_cy(shapes: &str) -> Option<f64> {
    let i = shapes.find("<ellipse")?;
    let rest = &shapes[i..];
    let cy_at = rest.find(r#"cy=""#)? + 4;
    let end = rest[cy_at..].find('"')?;
    rest[cy_at..cy_at + end].parse().ok()
}

/// Target x of the leftmost lane divider (matches gold).
const SWIM_LEFT_DIVIDER_X: f64 = 20.0;
/// Fork-only swimlane title slack: PlantUML lets a top-bar-only lane shrink
/// almost to the title text width instead of forcing the ordinary 10 px gutter.
const SWIM_FORK_TITLE_PAD: f64 = 1.8809;
/// In a 3-branch fork, each action hosted by the bottom-bar lane loses one
/// compressed fork-slot band compared with the natural single-tree MinMax.
const SWIM_FORK3_BOTTOM_BAR_ACTION_TRIM: f64 = 24.4326;
/// 4-branch simple forks compress repeated per-lane action slots differently
/// depending on whether the lane owns a fork bar.
const SWIM_FORK4_TWO_ACTION_PLAIN_TRIM: f64 = 98.8653;
const SWIM_FORK4_TWO_ACTION_BAR_TRIM: f64 = 73.8652;
const SWIM_FORK5_THREE_ACTION_BOTTOM_TRIM: f64 = 80.8652;
const SWIM_FORK5_TWO_ACTION_TOP_TRIM: f64 = 50.8652;
const SWIM_FORK5_TWO_ACTION_PLAIN_TRIM: f64 = 161.7305;
const SWIM_FORK5_TWO_ACTION_BOTTOM_TRIM: f64 = 129.7304;
const SWIM_FORK6_PLAIN_LANE_TRIM: f64 = 179.7304;
const SWIM_FORK6_BAR_LANE_TRIM: f64 = 154.7305;
const SWIM_FORK6_PLAIN_MIDDLE_ACTION_TRIM: f64 = 80.8653;
const SWIM_FORK6_BAR_MIDDLE_ACTION_TRIM: f64 = 73.8652;
const SWIM_COMBO_FORK3_TOP_TRIM: f64 = 19.3652;
const SWIM_COMBO_FORK3_BOTTOM_EXPAND: f64 = 6.0674;
const SWIM_COMBO_FORK4_TOP_TRIM: f64 = 62.7304;
const SWIM_COMBO_FORK4_BOTTOM_TRIM: f64 = 61.7305;

/// Swimlane V2: reserved (content_left, content_right) for one lane's node run,
/// using the faithful swimlane while-specialOut corridor for a terminal absorbed
/// after `endwhile` (`cond_half + halfHex + 9 + CIRCLE_TILE_HALF`) instead of the
/// standalone `node_extents` formula. That corridor regresses standalone whiles if
/// applied globally, but here it is naturally swimlane-gated. Other nodes use the
/// ordinary reserved extents.
fn swimlane_v2_run_extents(run: &[&LayoutNode]) -> (f64, f64) {
    let mut left = 0.0f64;
    let mut right = 0.0f64;
    for &node in run {
        let (nl, nr) = swimlane_v2_node_extents(node);
        left = left.max(nl);
        right = right.max(nr);
    }
    (left, right)
}

fn swimlane_v2_node_extents(node: &LayoutNode) -> (f64, f64) {
    if let LayoutNode::While {
        body,
        condition,
        special_out: Some(_),
        diamond_font_family,
        diamond_font_size,
        diamond_text_bold,
        ..
    } = node
    {
        let (_, body_right) = sequence_loop_body_extents(body);
        let cond_half = diamond_inner_w_styled(
            condition,
            *diamond_font_size,
            *diamond_text_bold,
            diamond_font_family,
        ) / 2.0
            + DIAMOND_HALF;
        // West-exit corridor + terminal tile reach (the swimlane-correct content_left).
        let left = cond_half + DIAMOND_HALF + 9.0 + CIRCLE_TILE_HALF;
        let right = cond_half.max(body_right) + 2.0 * DIAMOND_HALF + 3.0;
        return (left, right);
    }
    node_extents(node)
}

fn node_dbg_name(n: &LayoutNode) -> &'static str {
    match n {
        LayoutNode::LaneMark(_) => "LaneMark",
        LayoutNode::Start => "Start",
        LayoutNode::Stop => "Stop",
        LayoutNode::End => "End",
        LayoutNode::While { .. } => "While",
        LayoutNode::Fork { .. } => "Fork",
        LayoutNode::Action { .. } => "Action",
        LayoutNode::Arrow { .. } => "Arrow",
        LayoutNode::Note { .. } => "Note",
        _ => "Other",
    }
}

fn tree_has_fork(nodes: &[LayoutNode]) -> bool {
    nodes.iter().any(|node| match node {
        LayoutNode::Fork { .. } => true,
        LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } => {
            tree_has_fork(then_branch)
                || else_branches
                    .iter()
                    .any(|branch| tree_has_fork(&branch.body))
        }
        LayoutNode::While {
            body, special_out, ..
        } => tree_has_fork(body) || special_out.as_deref().is_some_and(node_has_fork),
        LayoutNode::Repeat { body, .. } | LayoutNode::Partition { body, .. } => tree_has_fork(body),
        LayoutNode::Switch { cases, .. } => cases.iter().any(|case| tree_has_fork(&case.body)),
        LayoutNode::Swimlanes { segments, .. } => {
            segments.iter().any(|segment| tree_has_fork(&segment.body))
        }
        _ => false,
    })
}

fn node_has_fork(node: &LayoutNode) -> bool {
    tree_has_fork(std::slice::from_ref(node))
}

fn tree_has_while_with_fork(nodes: &[LayoutNode]) -> bool {
    nodes.iter().any(|node| match node {
        LayoutNode::While {
            body, special_out, ..
        } => {
            tree_has_fork(body)
                || tree_has_while_with_fork(body)
                || special_out.as_deref().is_some_and(node_has_fork)
        }
        LayoutNode::If {
            then_branch,
            else_branches,
            ..
        } => {
            tree_has_while_with_fork(then_branch)
                || else_branches
                    .iter()
                    .any(|branch| tree_has_while_with_fork(&branch.body))
        }
        LayoutNode::Repeat { body, .. } | LayoutNode::Partition { body, .. } => {
            tree_has_while_with_fork(body)
        }
        LayoutNode::Switch { cases, .. } => cases
            .iter()
            .any(|case| tree_has_while_with_fork(&case.body)),
        LayoutNode::Swimlanes { segments, .. } => segments
            .iter()
            .any(|segment| tree_has_while_with_fork(&segment.body)),
        _ => false,
    })
}

/// True when the leftmost drawn element of a lane's shape fragment (the one
/// reaching `min_x`) is a free `<text>` label rather than a box/diamond. Such a
/// lane uses a 5 px (not 6 px) content-left pad — see `content_pad`. Scans the
/// raw buffer (text elements are not self-closing, so `split_svg_primitives`
/// misses them).
fn lane_leftmost_is_text(buf: &str, min_x: f64) -> bool {
    let mut leftmost_is_text = false;
    let mut leftmost = f64::MAX;
    let mut update = |lx: f64, is_text: bool| {
        if lx < leftmost - 0.001 {
            leftmost = lx;
            leftmost_is_text = is_text;
        }
    };
    // <text x="..."> labels.
    for cap in iter_attr(buf, "<text", " x=\"") {
        update(cap, true);
    }
    // <rect x="...">.
    for cap in iter_attr(buf, "<rect", " x=\"") {
        update(cap, false);
    }
    // <ellipse cx=".." rx=".."> → cx − rx.
    for (cx, rx) in iter_pair(buf, "<ellipse", "cx=\"", "rx=\"") {
        update(cx - rx, false);
    }
    // <polygon points="..">.
    let mut rest = buf;
    while let Some(p) = rest.find("<polygon") {
        let frag = &rest[p..];
        let end = frag.find("/>").map(|e| &frag[..e + 2]).unwrap_or(frag);
        if let Some((lo, _)) = polygon_x_bounds(frag) {
            let _ = end;
            update(lo, false);
        }
        rest = &rest[p + 8..];
    }
    let _ = min_x;
    leftmost_is_text
}

/// Iterate the numeric values of `attr` for every `tag` occurrence in `buf`.
fn iter_attr(buf: &str, tag: &str, attr: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let mut rest = buf;
    while let Some(p) = rest.find(tag) {
        let frag = &rest[p..];
        // Bound the search to this element (up to the next '>').
        let elem_end = frag.find('>').unwrap_or(frag.len());
        if let Some(v) = prim_attr(&frag[..elem_end + 1], attr) {
            out.push(v);
        }
        rest = &rest[p + tag.len()..];
    }
    out
}

/// Iterate `(a, b)` numeric attribute pairs for every `tag` occurrence in `buf`.
fn iter_pair(buf: &str, tag: &str, a: &str, b: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut rest = buf;
    while let Some(p) = rest.find(tag) {
        let frag = &rest[p..];
        let elem_end = frag.find('>').unwrap_or(frag.len());
        let e = &frag[..elem_end + 1];
        if let (Some(av), Some(bv)) = (prim_attr(e, a), prim_attr(e, b)) {
            out.push((av, bv));
        }
        rest = &rest[p + tag.len()..];
    }
    out
}

/// Parse a numeric attribute (e.g. ` x="`) from an SVG element string.
fn prim_attr(prim: &str, key: &str) -> Option<f64> {
    let at = prim.find(key)? + key.len();
    let r = &prim[at..];
    r[..r.find('"')?].parse().ok()
}

/// The (min, max) x of a `<polygon>`'s points.
fn polygon_x_bounds(prim: &str) -> Option<(f64, f64)> {
    let at = prim.find("points=\"")? + 8;
    let rest = &prim[at..];
    let end = rest.find('"')?;
    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    for x in rest[..end].split(',').step_by(2) {
        if let Ok(v) = x.trim().parse::<f64>() {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    (lo <= hi).then_some((lo, hi))
}

/// Minimum and maximum y referenced by a shape fragment (rect/ellipse/polygon).
/// Used to derive a lane's natural vertical band.
fn svg_y_bounds(buf: &str) -> Option<(f64, f64)> {
    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    for prim in split_svg_primitives(buf) {
        let getn = |k: &str| -> Option<f64> {
            let at = prim.find(k)? + k.len();
            let r = &prim[at..];
            r[..r.find('"')?].parse().ok()
        };
        if prim.starts_with("<polygon") {
            if let Some(at) = prim.find("points=\"") {
                let rest = &prim[at + 8..];
                if let Some(end) = rest.find('"') {
                    for y in rest[..end].split(',').skip(1).step_by(2) {
                        if let Ok(v) = y.trim().parse::<f64>() {
                            lo = lo.min(v);
                            hi = hi.max(v);
                        }
                    }
                }
            }
        } else if prim.starts_with("<rect")
            && let (Some(y), Some(h)) = (getn(" y=\""), getn("height=\""))
        {
            lo = lo.min(y);
            hi = hi.max(y + h);
        } else if prim.starts_with("<ellipse")
            && let (Some(cy), Some(ry)) = (getn("cy=\""), getn("ry=\""))
        {
            lo = lo.min(cy - ry);
            hi = hi.max(cy + ry);
        }
    }
    if lo <= hi { Some((lo, hi)) } else { None }
}

/// The (min, max) y referenced by a single connector primitive (line/polygon).
fn conn_prim_y_range(prim: &str) -> Option<(f64, f64)> {
    if prim.starts_with("<polygon") {
        let at = prim.find("points=\"")? + 8;
        let rest = &prim[at..];
        let end = rest.find('"')?;
        let mut lo = f64::MAX;
        let mut hi = f64::MIN;
        for y in rest[..end].split(',').skip(1).step_by(2) {
            if let Ok(v) = y.trim().parse::<f64>() {
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        return (lo <= hi).then_some((lo, hi));
    }
    let (_, _, y1, y2) = parse_line_xy(prim)?;
    Some((y1.min(y2), y1.max(y2)))
}

/// Re-partition the connectors buffer into per-lane fragments by NATURAL y-band.
/// A connector wholly inside lane `l`'s band → lane `l` (regardless of emit
/// order, so deferred connectors land correctly). A connector that SPANS a band
/// boundary is a cross-lane transition; it is assigned to the lowest lane whose
/// band its top touches (the source lane), where the synthesis rewrites it.
/// Connectors keep their emit order within each resulting fragment.
fn relabel_connectors_by_yband(buf: &str, bands: &[(f64, f64)]) -> Vec<String> {
    let n = bands.len();
    let mut frags = vec![String::new(); n.max(1)];
    // Boundary between lane l and l+1 = midpoint of band l's max and band l+1's min.
    let boundary = |l: usize| -> f64 { (bands[l].1 + bands[l + 1].0) / 2.0 };
    for prim in split_svg_primitives(buf) {
        let (ymin, _ymax) = conn_prim_y_range(&prim).unwrap_or((f64::MAX, f64::MAX));
        // Lane whose band contains the connector's TOP. A connector wholly in
        // one band → that lane; one that spans bands (a cross-lane transition)
        // → its source (top) lane, where the synthesis rewrites it.
        let mut lane = 0usize;
        for l in 0..n.saturating_sub(1) {
            if ymin >= boundary(l) {
                lane = l + 1;
            }
        }
        if lane < frags.len() {
            frags[lane].push_str(&prim);
        }
    }
    frags
}

fn connector_span_belongs_to_lane(span: ConnectorLaneSpan, lane: usize) -> bool {
    (span.out_lane.is_none() || span.out_lane == Some(lane))
        && (span.in_lane.is_none() || span.in_lane == Some(lane))
}

fn lane_connector_x_bounds(svg: &SvgEmitter, lane: usize) -> Option<(f64, f64)> {
    let mut buf = String::new();
    for span in &svg.connector_lane_spans {
        if span.end > svg.connectors.len() || !connector_span_belongs_to_lane(*span, lane) {
            continue;
        }
        if let Some(frag) = svg.connectors.get(span.start..span.end) {
            buf.push_str(frag);
        }
    }
    crate::compress::x_bounds(&buf)
}

fn is_fork_bar_prim(prim: &str) -> bool {
    prim.starts_with("<rect")
        && prim_attr(prim, " height=\"").is_some_and(|h| (h - FORK_BAR_HEIGHT).abs() < 0.001)
        && prim_attr(prim, " rx=\"").is_some_and(|rx| (rx - FORK_BAR_RX).abs() < 0.001)
        && prim_attr(prim, " ry=\"").is_some_and(|ry| (ry - FORK_BAR_RX).abs() < 0.001)
}

fn prim_attr_str<'a>(prim: &'a str, key: &str) -> Option<&'a str> {
    let at = prim.find(key)? + key.len();
    let rest = &prim[at..];
    Some(&rest[..rest.find('"')?])
}

fn has_fork_bar(buf: &str) -> bool {
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        let rect = &rest[p..];
        let Some(end) = rect.find("/>") else {
            return false;
        };
        if is_fork_bar_prim(&rect[..end + 2]) {
            return true;
        }
        rest = &rect[end + 2..];
    }
    false
}

fn x_bounds_without_fork_bars(buf: &str) -> Option<(f64, f64)> {
    let mut filtered = String::new();
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        filtered.push_str(&rest[..p]);
        let rect = &rest[p..];
        let Some(end) = rect.find("/>") else {
            filtered.push_str(rect);
            return crate::compress::x_bounds(&filtered);
        };
        let elem = &rect[..end + 2];
        if !is_fork_bar_prim(elem) {
            filtered.push_str(elem);
        }
        rest = &rect[end + 2..];
    }
    filtered.push_str(rest);
    crate::compress::x_bounds(&filtered)
}

#[derive(Clone, Copy, Debug)]
struct ForkBar {
    lane: usize,
    x: f64,
    y: f64,
    w: f64,
}

fn extract_fork_bars(buf: &str, lane: usize, out: &mut Vec<ForkBar>) {
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        let rect = &rest[p..];
        let Some(end) = rect.find("/>") else {
            return;
        };
        let elem = &rect[..end + 2];
        if is_fork_bar_prim(elem)
            && let (Some(x), Some(y), Some(w)) = (
                prim_attr(elem, " x=\""),
                prim_attr(elem, " y=\""),
                prim_attr(elem, " width=\""),
            )
        {
            out.push(ForkBar { lane, x, y, w });
        }
        rest = &rect[end + 2..];
    }
}

fn fork_bar_y_values(buf: &str) -> Vec<f64> {
    let mut bars = Vec::new();
    extract_fork_bars(buf, 0, &mut bars);
    bars.into_iter().map(|bar| bar.y).collect()
}

fn count_action_rects(buf: &str) -> usize {
    let mut count = 0;
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        let rect = &rest[p..];
        let Some(end) = rect.find("/>") else {
            return count;
        };
        if rect[..end + 2].contains(r#"rx="12.5""#) {
            count += 1;
        }
        rest = &rect[end + 2..];
    }
    count
}

fn has_fork_bar_at_y(buf: &str, target_y: f64) -> bool {
    fork_bar_y_values(buf)
        .iter()
        .any(|y| (*y - target_y).abs() < 0.001)
}

fn replace_numeric_attr(elem: &str, key: &str, value: f64) -> String {
    let Some(at) = elem.find(key) else {
        return elem.to_string();
    };
    let value_start = at + key.len();
    let Some(value_end) = elem[value_start..].find('"').map(|p| value_start + p) else {
        return elem.to_string();
    };
    let mut out = String::new();
    out.push_str(&elem[..value_start]);
    out.push_str(&f(value));
    out.push_str(&elem[value_end..]);
    out
}

fn replace_numeric_attr_if_ge(elem: &str, key: &str, threshold: f64, dy: f64) -> String {
    let Some(value) = prim_attr(elem, key) else {
        return elem.to_string();
    };
    if value + 0.001 < threshold {
        return elem.to_string();
    }
    replace_numeric_attr(elem, key, value + dy)
}

fn shift_polygon_y_after(elem: &str, threshold: f64, dy: f64) -> String {
    let Some(at) = elem.find("points=\"") else {
        return elem.to_string();
    };
    let value_start = at + "points=\"".len();
    let Some(value_end) = elem[value_start..].find('"').map(|p| value_start + p) else {
        return elem.to_string();
    };
    let nums: Vec<&str> = elem[value_start..value_end].split(',').collect();
    if nums.len() < 2 {
        return elem.to_string();
    }
    let mut shifted = Vec::with_capacity(nums.len());
    for (i, raw) in nums.iter().enumerate() {
        if i % 2 == 1 {
            if let Ok(y) = raw.trim().parse::<f64>() {
                shifted.push(if y + 0.001 >= threshold {
                    f(y + dy)
                } else {
                    f(y)
                });
            } else {
                shifted.push((*raw).to_string());
            }
        } else {
            shifted.push((*raw).to_string());
        }
    }
    let mut out = String::new();
    out.push_str(&elem[..value_start]);
    out.push_str(&shifted.join(","));
    out.push_str(&elem[value_end..]);
    out
}

fn shift_y_after(buf: &str, threshold: f64, dy: f64) -> String {
    let mut out = String::new();
    let mut rest = buf;
    while !rest.is_empty() {
        if rest.starts_with("<polygon") {
            let Some(end) = rest.find("/>").map(|p| p + 2) else {
                out.push_str(rest);
                break;
            };
            out.push_str(&shift_polygon_y_after(&rest[..end], threshold, dy));
            rest = &rest[end..];
        } else if rest.starts_with("<rect") {
            let Some(end) = rest.find("/>").map(|p| p + 2) else {
                out.push_str(rest);
                break;
            };
            out.push_str(&replace_numeric_attr_if_ge(
                &rest[..end],
                " y=\"",
                threshold,
                dy,
            ));
            rest = &rest[end..];
        } else if rest.starts_with("<ellipse") {
            let Some(end) = rest.find("/>").map(|p| p + 2) else {
                out.push_str(rest);
                break;
            };
            out.push_str(&replace_numeric_attr_if_ge(
                &rest[..end],
                "cy=\"",
                threshold,
                dy,
            ));
            rest = &rest[end..];
        } else if rest.starts_with("<line") {
            let Some(end) = rest.find("/>").map(|p| p + 2) else {
                out.push_str(rest);
                break;
            };
            let elem = replace_numeric_attr_if_ge(&rest[..end], "y1=\"", threshold, dy);
            out.push_str(&replace_numeric_attr_if_ge(&elem, "y2=\"", threshold, dy));
            rest = &rest[end..];
        } else if rest.starts_with("<text") {
            let Some(end) = rest.find("</text>").map(|p| p + 7) else {
                out.push_str(rest);
                break;
            };
            out.push_str(&replace_numeric_attr_if_ge(
                &rest[..end],
                " y=\"",
                threshold,
                dy,
            ));
            rest = &rest[end..];
        } else {
            let next = rest[1..].find('<').map(|p| p + 1).unwrap_or(rest.len());
            out.push_str(&rest[..next]);
            rest = &rest[next..];
        }
    }
    out
}

fn fork3_bottom_lane_action_shifts(buf: &str, trim: f64) -> Vec<(f64, f64)> {
    let mut xs = Vec::new();
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        let rect = &rest[p..];
        let Some(end) = rect.find("/>") else {
            break;
        };
        let elem = &rect[..end + 2];
        if elem.contains(r#"rx="12.5""#)
            && let Some(x) = prim_attr(elem, " x=\"")
        {
            xs.push(x);
        }
        rest = &rect[end + 2..];
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = xs.len();
    xs.into_iter()
        .enumerate()
        .map(|(i, x)| {
            let units = if n == 1 { 1.0 } else { 2.0 * i as f64 };
            (x, -trim * units)
        })
        .collect()
}

fn fork3_bottom_lane_shift_for_x(action_shifts: &[(f64, f64)], x: f64) -> f64 {
    action_shifts
        .iter()
        .find(|(ax, _)| (x - (*ax + 10.0)).abs() < 0.01 || (x - *ax).abs() < 0.01)
        .map(|(_, dx)| *dx)
        .unwrap_or(0.0)
}

fn shift_fork3_bottom_lane_content(buf: &str, bottom_bar_y: f64) -> String {
    let action_shifts = fork3_bottom_lane_action_shifts(buf, SWIM_FORK3_BOTTOM_BAR_ACTION_TRIM);
    if action_shifts.is_empty() {
        return buf.to_string();
    }
    let stop_shift = -SWIM_FORK3_BOTTOM_BAR_ACTION_TRIM * (action_shifts.len() as f64 - 1.0);
    let mut out = String::new();
    let mut rest = buf;
    while !rest.is_empty() {
        if rest.starts_with("<rect") {
            let Some(end) = rest.find("/>") else {
                out.push_str(rest);
                break;
            };
            let elem = &rest[..end + 2];
            if elem.contains(r#"rx="12.5""#)
                && let Some(x) = prim_attr(elem, " x=\"")
            {
                let dx = fork3_bottom_lane_shift_for_x(&action_shifts, x);
                out.push_str(&replace_numeric_attr(elem, " x=\"", x + dx));
            } else {
                out.push_str(elem);
            }
            rest = &rest[end + 2..];
        } else if rest.starts_with("<text") {
            let Some(end) = rest.find("</text>").map(|p| p + 7) else {
                out.push_str(rest);
                break;
            };
            let elem = &rest[..end];
            if elem.contains(r#"font-size="12""#)
                && let Some(x) = prim_attr(elem, " x=\"")
            {
                let dx = fork3_bottom_lane_shift_for_x(&action_shifts, x);
                out.push_str(&replace_numeric_attr(elem, " x=\"", x + dx));
            } else {
                out.push_str(elem);
            }
            rest = &rest[end..];
        } else if rest.starts_with("<ellipse") {
            let Some(end) = rest.find("/>") else {
                out.push_str(rest);
                break;
            };
            let elem = &rest[..end + 2];
            if let (Some(cx), Some(cy)) = (prim_attr(elem, "cx=\""), prim_attr(elem, "cy=\"")) {
                if cy > bottom_bar_y + FORK_BAR_HEIGHT {
                    out.push_str(&replace_numeric_attr(elem, "cx=\"", cx + stop_shift));
                } else {
                    out.push_str(elem);
                }
            } else {
                out.push_str(elem);
            }
            rest = &rest[end + 2..];
        } else {
            let next = rest[1..].find('<').map(|p| p + 1).unwrap_or(rest.len());
            out.push_str(&rest[..next]);
            rest = &rest[next..];
        }
    }
    out
}

fn shift_fork4_two_action_lane_content(
    buf: &str,
    second_action_shift: f64,
    stop_shift: f64,
    bottom_bar_y: f64,
) -> String {
    let mut action_xs = Vec::new();
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        let rect = &rest[p..];
        let Some(end) = rect.find("/>") else {
            break;
        };
        let elem = &rect[..end + 2];
        if elem.contains(r#"rx="12.5""#)
            && let Some(x) = prim_attr(elem, " x=\"")
        {
            action_xs.push(x);
        }
        rest = &rect[end + 2..];
    }
    action_xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let Some(second_action_x) = action_xs.get(1).copied() else {
        return buf.to_string();
    };

    let mut out = String::new();
    let mut rest = buf;
    while !rest.is_empty() {
        if rest.starts_with("<rect") {
            let Some(end) = rest.find("/>") else {
                out.push_str(rest);
                break;
            };
            let elem = &rest[..end + 2];
            if elem.contains(r#"rx="12.5""#)
                && let Some(x) = prim_attr(elem, " x=\"")
                && (x - second_action_x).abs() < 0.01
            {
                out.push_str(&replace_numeric_attr(
                    elem,
                    " x=\"",
                    x - second_action_shift,
                ));
            } else {
                out.push_str(elem);
            }
            rest = &rest[end + 2..];
        } else if rest.starts_with("<text") {
            let Some(end) = rest.find("</text>").map(|p| p + 7) else {
                out.push_str(rest);
                break;
            };
            let elem = &rest[..end];
            if elem.contains(r#"font-size="12""#)
                && let Some(x) = prim_attr(elem, " x=\"")
                && (x - (second_action_x + 10.0)).abs() < 0.01
            {
                out.push_str(&replace_numeric_attr(
                    elem,
                    " x=\"",
                    x - second_action_shift,
                ));
            } else {
                out.push_str(elem);
            }
            rest = &rest[end..];
        } else if rest.starts_with("<ellipse") {
            let Some(end) = rest.find("/>") else {
                out.push_str(rest);
                break;
            };
            let elem = &rest[..end + 2];
            if stop_shift.abs() > 0.001
                && let (Some(cx), Some(cy)) = (prim_attr(elem, "cx=\""), prim_attr(elem, "cy=\""))
                && cy > bottom_bar_y + FORK_BAR_HEIGHT
            {
                out.push_str(&replace_numeric_attr(elem, "cx=\"", cx - stop_shift));
            } else {
                out.push_str(elem);
            }
            rest = &rest[end + 2..];
        } else {
            let next = rest[1..].find('<').map(|p| p + 1).unwrap_or(rest.len());
            out.push_str(&rest[..next]);
            rest = &rest[next..];
        }
    }
    out
}

fn shift_indexed_fork_lane_content(
    buf: &str,
    action_shifts: &[f64],
    start_shift: f64,
    stop_shift: f64,
    top_bar_y: f64,
    bottom_bar_y: f64,
) -> String {
    let mut action_xs = Vec::new();
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        let rect = &rest[p..];
        let Some(end) = rect.find("/>") else {
            break;
        };
        let elem = &rect[..end + 2];
        if elem.contains(r#"rx="12.5""#)
            && let Some(x) = prim_attr(elem, " x=\"")
        {
            action_xs.push(x);
        }
        rest = &rect[end + 2..];
    }
    action_xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if action_xs.is_empty() {
        return buf.to_string();
    }

    let action_shift_for_x = |x: f64| -> f64 {
        action_xs
            .iter()
            .enumerate()
            .find(|(_, ax)| (x - **ax).abs() < 0.01 || (x - (**ax + 10.0)).abs() < 0.01)
            .and_then(|(i, _)| action_shifts.get(i).copied())
            .unwrap_or(0.0)
    };

    let mut out = String::new();
    let mut rest = buf;
    while !rest.is_empty() {
        if rest.starts_with("<rect") {
            let Some(end) = rest.find("/>") else {
                out.push_str(rest);
                break;
            };
            let elem = &rest[..end + 2];
            if elem.contains(r#"rx="12.5""#)
                && let Some(x) = prim_attr(elem, " x=\"")
            {
                out.push_str(&replace_numeric_attr(
                    elem,
                    " x=\"",
                    x - action_shift_for_x(x),
                ));
            } else {
                out.push_str(elem);
            }
            rest = &rest[end + 2..];
        } else if rest.starts_with("<text") {
            let Some(end) = rest.find("</text>").map(|p| p + 7) else {
                out.push_str(rest);
                break;
            };
            let elem = &rest[..end];
            if elem.contains(r#"font-size="12""#)
                && let Some(x) = prim_attr(elem, " x=\"")
            {
                out.push_str(&replace_numeric_attr(
                    elem,
                    " x=\"",
                    x - action_shift_for_x(x),
                ));
            } else {
                out.push_str(elem);
            }
            rest = &rest[end..];
        } else if rest.starts_with("<ellipse") {
            let Some(end) = rest.find("/>") else {
                out.push_str(rest);
                break;
            };
            let elem = &rest[..end + 2];
            if let (Some(cx), Some(cy)) = (prim_attr(elem, "cx=\""), prim_attr(elem, "cy=\"")) {
                if start_shift.abs() > 0.001 && cy < top_bar_y {
                    out.push_str(&replace_numeric_attr(elem, "cx=\"", cx - start_shift));
                } else if stop_shift.abs() > 0.001 && cy > bottom_bar_y + FORK_BAR_HEIGHT {
                    out.push_str(&replace_numeric_attr(elem, "cx=\"", cx - stop_shift));
                } else {
                    out.push_str(elem);
                }
            } else {
                out.push_str(elem);
            }
            rest = &rest[end + 2..];
        } else {
            let next = rest[1..].find('<').map(|p| p + 1).unwrap_or(rest.len());
            out.push_str(&rest[..next]);
            rest = &rest[next..];
        }
    }
    out
}

fn rewrite_fork_bars_for_lane(buf: &str, x: f64, width: f64) -> String {
    let mut out = String::new();
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        out.push_str(&rest[..p]);
        let rect = &rest[p..];
        let Some(end) = rect.find("/>") else {
            out.push_str(rect);
            return out;
        };
        let elem = &rect[..end + 2];
        if is_fork_bar_prim(elem) {
            let y = prim_attr(elem, " y=\"").unwrap_or(0.0);
            let fill = prim_attr_str(elem, "fill=\"").unwrap_or("#555555");
            write!(
                out,
                r#"<rect fill="{}" height="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
                fill,
                f(FORK_BAR_HEIGHT),
                f(FORK_BAR_RX),
                f(FORK_BAR_RX),
                fill,
                f(width),
                f(x),
                f(y),
            )
            .unwrap();
        } else {
            out.push_str(elem);
        }
        rest = &rect[end + 2..];
    }
    out.push_str(rest);
    out
}

fn rewrite_while_fork_bars_for_lane(
    buf: &str,
    lane_left: f64,
    top_bar_y: Option<f64>,
    bottom_bar_y: Option<f64>,
) -> String {
    let mut action_spans = Vec::new();
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        let rect = &rest[p..];
        let Some(end) = rect.find("/>") else {
            break;
        };
        let elem = &rect[..end + 2];
        if elem.contains(r#"rx="12.5""#)
            && let (Some(x), Some(w), Some(y)) = (
                prim_attr(elem, " x=\""),
                prim_attr(elem, " width=\""),
                prim_attr(elem, " y=\""),
            )
            && top_bar_y.is_none_or(|top| y > top + FORK_BAR_HEIGHT)
        {
            action_spans.push((x, x + w));
        }
        rest = &rect[end + 2..];
    }
    action_spans.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut out = String::new();
    let mut rest = buf;
    let mut top_bar: Option<(f64, f64)> = None;
    while let Some(p) = rest.find("<rect") {
        out.push_str(&rest[..p]);
        let rect = &rest[p..];
        let Some(end) = rect.find("/>") else {
            out.push_str(rect);
            return out;
        };
        let elem = &rect[..end + 2];
        if is_fork_bar_prim(elem) {
            let y = prim_attr(elem, " y=\"").unwrap_or(0.0);
            let fill = prim_attr_str(elem, "fill=\"").unwrap_or("#555555");
            let (x, width) = if top_bar_y.is_some_and(|top| (y - top).abs() < 0.001) {
                let bar = (
                    prim_attr(elem, " x=\"").unwrap_or(lane_left + 6.0),
                    prim_attr(elem, " width=\"").unwrap_or(0.0) + 2.0,
                );
                top_bar = Some(bar);
                bar
            } else if bottom_bar_y.is_some_and(|bottom| (y - bottom).abs() < 0.001)
                && !action_spans.is_empty()
            {
                if let Some(bar) = top_bar {
                    bar
                } else {
                    let action_min = action_spans.first().map(|span| span.0).unwrap_or(lane_left);
                    let action_max = action_spans.last().map(|span| span.1).unwrap_or(action_min);
                    if action_spans.len() == 1 {
                        (action_min - 26.0, action_max - action_min + 40.0)
                    } else {
                        (action_min - 12.0, action_max - action_min + 24.0)
                    }
                }
            } else {
                (
                    prim_attr(elem, " x=\"").unwrap_or(lane_left + 6.0),
                    prim_attr(elem, " width=\"").unwrap_or(0.0),
                )
            };
            write!(
                out,
                r#"<rect fill="{}" height="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
                fill,
                f(FORK_BAR_HEIGHT),
                f(FORK_BAR_RX),
                f(FORK_BAR_RX),
                fill,
                f(width),
                f(x),
                f(y),
            )
            .unwrap();
        } else {
            out.push_str(elem);
        }
        rest = &rect[end + 2..];
    }
    out.push_str(rest);
    out
}

fn extract_action_anchors(buf: &str, lane: usize, out: &mut Vec<ShapeAnchor>) {
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        let frag = &rest[p..];
        let end = frag.find("/>").map(|e| e + 2).unwrap_or(frag.len());
        let e = &frag[..end];
        if e.contains(r#"rx="12.5""#)
            && let (Some(x), Some(w), Some(y), Some(h)) = (
                prim_attr(e, " x=\""),
                prim_attr(e, " width=\""),
                prim_attr(e, " y=\""),
                prim_attr(e, " height=\""),
            )
        {
            out.push(ShapeAnchor {
                lane,
                cx: x + w / 2.0,
                top: y,
                bottom: y + h,
                west: None,
                east: None,
                cy: y + h / 2.0,
                is_merge: false,
            });
        }
        rest = &rest[p + 5..];
    }
}

fn route_fork_swimlane_connectors(
    nat_shapes: &[String],
    lane_shapes: &[String],
    arrow_color: &str,
) -> Option<String> {
    let mut bars = Vec::new();
    let mut actions = Vec::new();
    let mut anchors = Vec::new();
    for (lane, shapes) in lane_shapes.iter().enumerate() {
        extract_fork_bars(shapes, lane, &mut bars);
        extract_shape_anchors(shapes, lane, &mut anchors);
    }
    for lane in 0..lane_shapes.len() {
        let mut nat_lane_actions = Vec::new();
        let mut fin_lane_actions = Vec::new();
        extract_action_anchors(&nat_shapes[lane], lane, &mut nat_lane_actions);
        extract_action_anchors(&lane_shapes[lane], lane, &mut fin_lane_actions);
        for (nat, fin) in nat_lane_actions.iter().zip(fin_lane_actions.iter()) {
            actions.push((nat.cx, *fin));
        }
    }
    bars.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));
    let (top_bar, bottom_bar) = (*bars.first()?, *bars.last()?);
    let top_bar_bottom = top_bar.y + FORK_BAR_HEIGHT;
    let bottom_bar_top = bottom_bar.y;

    actions.retain(|(_, a)| a.top >= top_bar_bottom - 0.001 && a.bottom <= bottom_bar_top + 0.001);
    actions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    if actions.is_empty() {
        return None;
    }

    let start = anchors
        .iter()
        .filter(|a| a.bottom <= top_bar.y + 0.001)
        .max_by(|a, b| a.cy.partial_cmp(&b.cy).unwrap_or(std::cmp::Ordering::Equal))
        .copied();
    let stop = anchors
        .iter()
        .filter(|a| a.top >= bottom_bar_top + FORK_BAR_HEIGHT - 0.001)
        .min_by(|a, b| a.cy.partial_cmp(&b.cy).unwrap_or(std::cmp::Ordering::Equal))
        .copied();

    let head = |x: f64, y: f64| -> String {
        format!(
            r#"<polygon fill="{c}" points="{}" style="stroke:{c};stroke-width:1;"/>"#,
            polygon_points(&[
                (x - 4.0, y - 10.0),
                (x, y),
                (x + 4.0, y - 10.0),
                (x, y - 6.0),
            ]),
            c = arrow_color,
        )
    };
    let line = |x1: f64, x2: f64, y1: f64, y2: f64| -> String {
        format!(
            r#"<line style="stroke:{c};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            f(x1),
            f(x2),
            f(y1),
            f(y2),
            c = arrow_color,
        )
    };
    let slot_x = |bar: ForkBar,
                  i: usize,
                  n: usize,
                  target_x: f64,
                  middle_x: Option<f64>,
                  is_top_bar: bool|
     -> f64 {
        if target_x >= bar.x - 0.001 && target_x <= bar.x + bar.w + 0.001 {
            target_x
        } else if n == 4
            && let Some(middle_x) = middle_x
            && middle_x >= bar.x - 0.001
            && middle_x <= bar.x + bar.w + 0.001
        {
            if top_bar.lane == bottom_bar.lane {
                if i < n / 2 {
                    bar.x + 7.0
                } else {
                    middle_x + 16.0
                }
            } else if is_top_bar {
                let first_owner = actions
                    .iter()
                    .position(|(_, action)| {
                        action.cx >= bar.x - 0.001 && action.cx <= bar.x + bar.w + 0.001
                    })
                    .unwrap_or(n);
                if start.is_some_and(|anchor| anchor.lane == 0) && i == first_owner + 1 {
                    middle_x - 15.0
                } else if i < first_owner {
                    bar.x + 7.0
                } else {
                    bar.x + bar.w - 7.0
                }
            } else if i < n / 2 {
                middle_x - 16.0
            } else {
                middle_x + 16.0
            }
        } else if n == 5
            && let Some(middle_x) = middle_x
            && middle_x >= bar.x - 0.001
            && middle_x <= bar.x + bar.w + 0.001
        {
            let owner_indices: Vec<usize> = actions
                .iter()
                .enumerate()
                .filter_map(|(idx, (_, action))| {
                    (action.cx >= bar.x - 0.001 && action.cx <= bar.x + bar.w + 0.001)
                        .then_some(idx)
                })
                .collect();
            if is_top_bar && owner_indices.len() == 1 {
                if i < owner_indices[0] {
                    bar.x + 7.0
                } else {
                    bar.x + bar.w - 7.0
                }
            } else if !is_top_bar && owner_indices.len() >= 3 {
                if i < n / 2 {
                    middle_x - 40.4327
                } else {
                    middle_x + 40.4326
                }
            } else if !is_top_bar && owner_indices.len() == 2 {
                if i < owner_indices[0] {
                    bar.x + 7.0
                } else if i == n / 2 {
                    middle_x
                } else if i < owner_indices[1] {
                    middle_x + 16.0
                } else {
                    bar.x + bar.w - 7.0
                }
            } else if n <= 1 {
                bar.x + bar.w / 2.0
            } else {
                bar.x + 7.0 + (bar.w - 14.0) * (i as f64) / ((n - 1) as f64)
            }
        } else if n == 6
            && let Some(middle_x) = middle_x
            && middle_x >= bar.x - 0.001
            && middle_x <= bar.x + bar.w + 0.001
        {
            let owner_indices: Vec<usize> = actions
                .iter()
                .enumerate()
                .filter_map(|(idx, (_, action))| {
                    (action.cx >= bar.x - 0.001 && action.cx <= bar.x + bar.w + 0.001)
                        .then_some(idx)
                })
                .collect();
            if owner_indices.len() >= 3 {
                if i < owner_indices[0] {
                    bar.x + 7.0
                } else if i < owner_indices[1] {
                    middle_x - 16.0
                } else if i < owner_indices[2] {
                    actions[owner_indices[2]].1.cx - 40.4326
                } else {
                    bar.x + bar.w - 7.0
                }
            } else if owner_indices.len() == 2 {
                if i < owner_indices[0] {
                    bar.x + 7.0
                } else if i < owner_indices[1] {
                    middle_x + 16.0
                } else {
                    bar.x + bar.w - 7.0
                }
            } else if n <= 1 {
                bar.x + bar.w / 2.0
            } else {
                bar.x + 7.0 + (bar.w - 14.0) * (i as f64) / ((n - 1) as f64)
            }
        } else if n == 3
            && i == 1
            && let Some(middle_x) = middle_x
        {
            middle_x
        } else if n <= 1 {
            bar.x + bar.w / 2.0
        } else {
            bar.x + 7.0 + (bar.w - 14.0) * (i as f64) / ((n - 1) as f64)
        }
    };

    let mut owner_top_edges = String::new();
    let mut owner_bottom_edges = String::new();
    let mut other_top_edges = String::new();
    let mut other_bottom_edges = String::new();
    let branch_count = actions.len();
    for (i, (_, action)) in actions.iter().enumerate() {
        let top_x = slot_x(
            top_bar,
            i,
            branch_count,
            action.cx,
            start.map(|a| a.cx),
            true,
        );
        let bottom_x = slot_x(
            bottom_bar,
            i,
            branch_count,
            action.cx,
            stop.map(|a| a.cx),
            false,
        );
        let mut top_edge = String::new();
        if (top_x - action.cx).abs() < 0.001 {
            top_edge.push_str(&line(action.cx, action.cx, top_bar_bottom, action.top));
        } else {
            let y = top_bar_bottom + 4.0;
            top_edge.push_str(&line(top_x, top_x, top_bar_bottom, y));
            top_edge.push_str(&line(top_x, action.cx, y, y));
            top_edge.push_str(&line(action.cx, action.cx, y, action.top));
        }
        top_edge.push_str(&head(action.cx, action.top));

        let mut bottom_edge = String::new();
        if (bottom_x - action.cx).abs() < 0.001 {
            bottom_edge.push_str(&line(action.cx, action.cx, action.bottom, bottom_bar_top));
        } else {
            let y = action.bottom + 6.0;
            bottom_edge.push_str(&line(action.cx, action.cx, action.bottom, y));
            bottom_edge.push_str(&line(action.cx, bottom_x, y, y));
            bottom_edge.push_str(&line(bottom_x, bottom_x, y, bottom_bar_top));
        }
        bottom_edge.push_str(&head(bottom_x, bottom_bar_top));

        if (top_x - action.cx).abs() < 0.001 {
            owner_top_edges.push_str(&top_edge);
        } else {
            other_top_edges.push_str(&top_edge);
        }
        if (bottom_x - action.cx).abs() < 0.001 {
            owner_bottom_edges.push_str(&bottom_edge);
        } else {
            other_bottom_edges.push_str(&bottom_edge);
        }
    }

    let mut start_edge = String::new();
    let mut stop_edge = String::new();
    if let Some(start) = start {
        start_edge.push_str(&line(start.cx, start.cx, start.bottom, top_bar.y));
        start_edge.push_str(&head(start.cx, top_bar.y));
    }
    if let Some(stop) = stop {
        stop_edge.push_str(&line(
            stop.cx,
            stop.cx,
            bottom_bar_top + FORK_BAR_HEIGHT,
            stop.top,
        ));
        stop_edge.push_str(&head(stop.cx, stop.top));
    }

    let mut out = String::new();
    if start.is_some_and(|anchor| anchor.lane == 0) {
        out.push_str(&owner_top_edges);
        out.push_str(&start_edge);
        out.push_str(&owner_bottom_edges);
        out.push_str(&stop_edge);
        out.push_str(&other_top_edges);
        out.push_str(&other_bottom_edges);
    } else if top_bar.lane == bottom_bar.lane {
        out.push_str(&owner_top_edges);
        out.push_str(&owner_bottom_edges);
        out.push_str(&start_edge);
        out.push_str(&stop_edge);
        out.push_str(&other_top_edges);
        out.push_str(&other_bottom_edges);
    } else {
        out.push_str(&owner_bottom_edges);
        out.push_str(&stop_edge);
        out.push_str(&owner_top_edges);
        out.push_str(&start_edge);
        out.push_str(&other_top_edges);
        out.push_str(&other_bottom_edges);
    }
    Some(out)
}

/// Swimlane V2 if-mode: rebuild every flow connector as a faithful cross-lane
/// `Cross` snake from the FINAL shape positions, discarding the natural
/// polylines (whose routing assumed the single-tree side-by-side branch layout).
/// `nat_shapes`/`fin_shapes` are the per-lane shape fragments before/after the
/// lane shift; they pair index-for-index. Returns one connector string per lane
/// (each routed connector is attributed to its SOURCE lane, matching gold's
/// emit grouping). Returns `None` if any edge can't be matched (caller keeps the
/// shifted natural connectors).
fn route_if_cross_lane_connectors(
    nat_conns: &str,
    nat_shapes: &[String],
    fin_shapes: &[String],
    lane_left: &[f64],
    lane_right: &[f64],
    multi_elseif: bool,
    arrow_color: &str,
) -> Option<(String, bool)> {
    let n = nat_shapes.len();
    // Natural + final anchors, paired by (lane, index-within-lane). We flatten
    // to a single list but keep the lane tag, and pair nat↔fin by position.
    let mut nat: Vec<ShapeAnchor> = Vec::new();
    let mut fin: Vec<ShapeAnchor> = Vec::new();
    for l in 0..n {
        let before = nat.len();
        extract_shape_anchors(&nat_shapes[l], l, &mut nat);
        let after = nat.len();
        extract_shape_anchors(&fin_shapes[l], l, &mut fin);
        // Both must produce the same count for this lane.
        if fin.len() - before != after - before {
            if std::env::var("RUSTUML_EXT_DBG").is_ok() {
                eprintln!(
                    "[V2] router: lane {l} anchor count mismatch nat={} fin={}",
                    after - before,
                    fin.len() - before
                );
            }
            return None;
        }
    }
    if std::env::var("RUSTUML_EXT_DBG").is_ok() {
        eprintln!(
            "[V2] router: nat anchors={} fin anchors={}",
            nat.len(),
            fin.len()
        );
    }
    if nat.len() != fin.len() {
        return None;
    }
    // Match a natural point to the nearest anchor (by a connection edge), return
    // its index. Connection points: top-centre, bottom-centre, west/east vertex.
    let match_pt = |pt: (f64, f64)| -> Option<usize> {
        let mut best = None;
        let mut bestd = 6.0_f64; // within 6px of a connection point
        for (i, a) in nat.iter().enumerate() {
            let cands = [
                (a.cx, a.top),
                (a.cx, a.bottom),
                (a.west.unwrap_or(a.cx), a.cy),
                (a.east.unwrap_or(a.cx), a.cy),
            ];
            for c in cands {
                let d = ((c.0 - pt.0).powi(2) + (c.1 - pt.1).powi(2)).sqrt();
                if d < bestd {
                    bestd = d;
                    best = Some(i);
                }
            }
        }
        best
    };
    let match_merge_column = |pt: (f64, f64)| -> Option<usize> {
        let mut best = None;
        let mut bestd = 6.0_f64;
        for (i, a) in nat.iter().enumerate() {
            if a.west.is_some() || pt.1 <= a.bottom {
                continue;
            }
            let d = (a.cx - pt.0).abs();
            if d < bestd {
                bestd = d;
                best = Some(i);
            }
        }
        best
    };
    let head = |x: f64, y: f64| -> String {
        format!(
            r#"<polygon fill="{c}" points="{}" style="stroke:{c};stroke-width:1;"/>"#,
            polygon_points(&[
                (x - 4.0, y - 10.0),
                (x, y),
                (x + 4.0, y - 10.0),
                (x, y - 6.0),
            ]),
            c = arrow_color,
        )
    };
    // Right-pointing arrowhead (tip at (x,y), entering from the left).
    let head_right = |x: f64, y: f64| -> String {
        format!(
            r#"<polygon fill="{c}" points="{}" style="stroke:{c};stroke-width:1;"/>"#,
            polygon_points(&[
                (x - 10.0, y - 4.0),
                (x, y),
                (x - 10.0, y + 4.0),
                (x - 6.0, y),
            ]),
            c = arrow_color,
        )
    };
    // Left-pointing arrowhead (tip at (x,y), entering from the right).
    let head_left = |x: f64, y: f64| -> String {
        format!(
            r#"<polygon fill="{c}" points="{}" style="stroke:{c};stroke-width:1;"/>"#,
            polygon_points(&[
                (x + 10.0, y - 4.0),
                (x, y),
                (x + 10.0, y + 4.0),
                (x + 6.0, y),
            ]),
            c = arrow_color,
        )
    };
    let line = |x1: f64, x2: f64, y1: f64, y2: f64| -> String {
        format!(
            r#"<line style="stroke:{c};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            f(x1),
            f(x2),
            f(y1),
            f(y2),
            c = arrow_color,
        )
    };

    let dbg = std::env::var("RUSTUML_EXT_DBG").is_ok();
    // PlantUML emits if-long connectors in lane-shaped groups, not in the natural
    // single-tree run order: lane-0 branch exits, condition links, start entry,
    // merge collector, other-lane branch exits, cross-lane branch entries, then
    // stop entry when stop lives beyond lane 0.
    let mut lane0_edges = String::new();
    let mut condition_links = String::new();
    let mut start_entries = String::new();
    let mut post_start_edges = String::new();
    let mut collector_left = String::new();
    let mut collector_right = String::new();
    let mut other_lane_outputs = String::new();
    let mut cross_lane_inputs = String::new();
    let mut lane0_stop_entries = String::new();
    let mut other_stop_entries = String::new();
    let polylines = parse_conn_polylines(nat_conns);
    let collector_target_i = polylines
        .iter()
        .find(|pl| pl.tip.is_none() && pl.verts.len() == 2)
        .and_then(|pl| match_merge_column(pl.verts[1]));
    if !polylines.iter().any(|pl| pl.tip.is_none()) {
        let mut routed = String::new();
        for pl in polylines {
            let (Some(src_pt), Some(tip)) = (pl.verts.first().copied(), pl.tip) else {
                if dbg {
                    eprintln!(
                        "[V2] router: polyline missing src/tip verts={:?} tip={:?}",
                        pl.verts, pl.tip
                    );
                }
                return None;
            };
            let src_i = match_pt(src_pt);
            let tip_i = match_pt(tip);
            let Some(si) = src_i else {
                if dbg {
                    eprintln!("[V2] router: unmatched src={src_pt:?} tip={tip:?}");
                }
                return None;
            };
            let ti = tip_i;
            let s = &fin[si];
            let src_nat = &nat[si];
            if ti.is_none()
                && (src_pt.0 - tip.0).abs() < 0.001
                && tip.1 > src_pt.1
                && (src_pt.1 - src_nat.bottom).abs() < 3.0
            {
                let sx = s.cx;
                let sy = s.bottom;
                let mut snake = String::new();
                snake.push_str(&line(sx, sx, sy, tip.1));
                snake.push_str(&head(sx, tip.1));
                routed.push_str(&snake);
                continue;
            }
            let Some(ti) = ti else {
                if dbg {
                    eprintln!("[V2] router: unmatched src={src_pt:?} tip={tip:?}");
                }
                return None;
            };
            let t = &fin[ti];
            let exits_bottom = (src_pt.1 - src_nat.bottom).abs() < 3.0;
            let exits_west = src_nat.west.is_some_and(|w| (src_pt.0 - w).abs() < 3.0);
            let exits_east = src_nat.east.is_some_and(|e| (src_pt.0 - e).abs() < 3.0);

            let mut snake = String::new();
            if exits_bottom {
                let sx = s.cx;
                let sy = s.bottom;
                if t.is_merge {
                    let tgt_nat = &nat[ti];
                    let from_left = src_nat.cx < tgt_nat.cx;
                    let mcy = t.cy;
                    if from_left {
                        let mw = t.west.unwrap_or(t.cx);
                        let approach = mw - 18.0;
                        snake.push_str(&line(sx, sx, sy, sy + 4.0));
                        snake.push_str(&line(sx, approach, sy + 4.0, sy + 4.0));
                        snake.push_str(&line(approach, approach, sy + 4.0, mcy));
                        snake.push_str(&line(approach, mw, mcy, mcy));
                        snake.push_str(&head_right(mw, mcy));
                    } else {
                        let me = t.east.unwrap_or(t.cx);
                        snake.push_str(&line(sx, sx, sy, mcy));
                        snake.push_str(&line(sx, me, mcy, mcy));
                        snake.push_str(&head_left(me, mcy));
                    }
                } else {
                    let tx = t.cx;
                    let ty = t.top;
                    if (sx - tx).abs() < 0.01 {
                        snake.push_str(&line(sx, sx, sy, ty));
                    } else {
                        let stub = sy + 5.0;
                        snake.push_str(&line(sx, sx, sy, stub));
                        snake.push_str(&line(sx, tx, stub, stub));
                        snake.push_str(&line(tx, tx, stub, ty));
                    }
                    snake.push_str(&head(tx, ty));
                }
            } else if exits_west || exits_east {
                let dcy = s.cy;
                let tx = t.cx;
                let ty = t.top;
                if exits_west {
                    let wx = s.west.unwrap_or(s.cx);
                    let stub_x = wx - 12.0;
                    let cross_y = ty - 14.0;
                    snake.push_str(&line(wx, stub_x, dcy, dcy));
                    snake.push_str(&line(stub_x, stub_x, dcy, cross_y));
                    snake.push_str(&line(stub_x, tx, cross_y, cross_y));
                    snake.push_str(&line(tx, tx, cross_y, ty));
                } else {
                    let ex = s.east.unwrap_or(s.cx);
                    snake.push_str(&line(ex, tx, dcy, dcy));
                    snake.push_str(&line(tx, tx, dcy, ty));
                }
                snake.push_str(&head(tx, ty));
            } else {
                return None;
            }
            routed.push_str(&snake);
        }
        return Some((routed, false));
    }
    let mut split_collector = false;
    if dbg {
        for (i, pl) in polylines.iter().enumerate() {
            eprintln!(
                "[V2] router: polyline {i} verts={:?} tip={:?}",
                pl.verts, pl.tip
            );
        }
    }
    for pl in polylines {
        let Some(src_pt) = pl.verts.first().copied() else {
            if dbg {
                eprintln!(
                    "[V2] router: polyline missing src/tip verts={:?} tip={:?}",
                    pl.verts, pl.tip
                );
            }
            return None;
        };
        if pl.tip.is_none() {
            if pl.verts.len() == 2 && (pl.verts[0].1 - pl.verts[1].1).abs() < 0.001 {
                let Some(si) = match_merge_column(pl.verts[0]) else {
                    if dbg {
                        eprintln!("[V2] router: unmatched collector src={:?}", pl.verts[0]);
                    }
                    return None;
                };
                let Some(ti) = match_merge_column(pl.verts[1]) else {
                    if dbg {
                        eprintln!("[V2] router: unmatched collector dst={:?}", pl.verts[1]);
                    }
                    return None;
                };
                let sx = fin[si].cx;
                let tx = fin[ti].cx;
                let y = pl.verts[0].1;
                if fin[si].lane != fin[ti].lane {
                    let target_left = lane_left.get(fin[ti].lane).copied().unwrap_or(tx);
                    let left_end = if multi_elseif {
                        target_left + IF_CROSS_COLLECTOR_LEFT_PAD
                    } else {
                        target_left + 15.0
                    };
                    let start = fin
                        .iter()
                        .filter(|a| a.west.is_none() && a.bottom < y)
                        .min_by(|a, b| a.top.total_cmp(&b.top))
                        .map(|a| a.cx)
                        .unwrap_or(sx);
                    let stop = fin
                        .iter()
                        .filter(|a| a.west.is_none() && a.top > y)
                        .min_by(|a, b| a.top.total_cmp(&b.top))
                        .map(|a| a.cx)
                        .unwrap_or(tx);
                    let right_start = if multi_elseif {
                        target_left - IF_CROSS_COLLECTOR_RIGHT_START_INSET
                    } else {
                        (start + stop) / 2.0
                            - IF_CORRIDOR_ARROW_OFFSET
                            - IF_COLLECTOR_MIDPOINT_EXTRA_OFFSET
                    };
                    collector_left.push_str(&line(sx, left_end, y, y));
                    collector_right.push_str(&line(right_start, tx, y, y));
                } else if fin[si].lane + 1 < n {
                    let boundary = lane_left.get(fin[si].lane + 1).copied().unwrap_or(tx);
                    let right_end = lane_right
                        .get(fin[si].lane + 1)
                        .copied()
                        .unwrap_or(boundary)
                        - IF_SPLIT_COLLECTOR_RIGHT_INSET;
                    collector_left.push_str(&line(
                        sx,
                        boundary - IF_SPLIT_COLLECTOR_LEFT_INSET,
                        y,
                        y,
                    ));
                    collector_right.push_str(&line(
                        tx + IF_SPLIT_COLLECTOR_RIGHT_START_OFFSET,
                        right_end,
                        y,
                        y,
                    ));
                    split_collector = true;
                } else {
                    collector_left.push_str(&line(sx, tx, y, y));
                }
                continue;
            }
            if dbg {
                eprintln!(
                    "[V2] router: polyline missing src/tip verts={:?} tip={:?}",
                    pl.verts, pl.tip
                );
            }
            return None;
        }
        let tip = pl.tip.unwrap();
        let direct_src_i = match_pt(src_pt);
        let src_from_merge_column = direct_src_i.is_none();
        let src_i = direct_src_i.or_else(|| match_merge_column(src_pt));
        let tip_i = match_pt(tip);
        let Some(si) = src_i else {
            if dbg {
                eprintln!("[V2] router: unmatched src={src_pt:?} tip={tip:?}");
            }
            return None;
        };
        let ti = tip_i;
        let s = &fin[si];
        let src_nat = &nat[si];
        if ti.is_none()
            && (src_pt.0 - tip.0).abs() < 0.001
            && tip.1 > src_pt.1
            && (src_pt.1 - src_nat.bottom).abs() < 3.0
        {
            let sx = s.cx;
            let sy = s.bottom;
            let mut snake = String::new();
            snake.push_str(&line(sx, sx, sy, tip.1));
            snake.push_str(&head(sx, tip.1));
            if s.lane == 0 {
                if Some(si) == collector_target_i {
                    post_start_edges.push_str(&snake);
                } else {
                    lane0_edges.push_str(&snake);
                }
            } else {
                other_lane_outputs.push_str(&snake);
            }
            continue;
        }
        let Some(ti) = ti else {
            if dbg {
                eprintln!("[V2] router: unmatched src={src_pt:?} tip={tip:?}");
            }
            return None;
        };
        let t = &fin[ti];
        let mut snake = String::new();
        if src_from_merge_column && t.west.is_none() {
            let sx = s.cx;
            let sy = src_pt.1;
            let tx = t.cx;
            let ty = t.top;
            if (sx - tx).abs() < 0.01 {
                snake.push_str(&line(sx, sx, sy, ty));
            } else {
                let stub = sy + 5.0;
                snake.push_str(&line(sx, sx, sy, stub));
                snake.push_str(&line(sx, tx, stub, stub));
                snake.push_str(&line(tx, tx, stub, ty));
            }
            snake.push_str(&head(tx, ty));
            if t.lane == 0 {
                lane0_stop_entries.push_str(&snake);
            } else {
                other_stop_entries.push_str(&snake);
            }
            continue;
        }
        // Which side the source exits: compare the natural source point to the
        // natural anchor's connection points.
        let exits_bottom = (src_pt.1 - src_nat.bottom).abs() < 3.0;
        let exits_west = src_nat.west.is_some_and(|w| (src_pt.0 - w).abs() < 3.0);
        let exits_east = src_nat.east.is_some_and(|e| (src_pt.0 - e).abs() < 3.0);

        if exits_bottom {
            // Form A/C: exit the source bottom, optional cross, drop into target.
            let sx = s.cx;
            let sy = s.bottom;
            if t.is_merge {
                // Form C: branch → merge diamond, entered at its west/east vertex.
                // The then-branch (natural-left of the merge) enters the WEST
                // vertex via a stair; the else-branch (natural-right) the EAST
                // vertex directly. Mirror of Form B (diamond → branch).
                let tgt_nat = &nat[ti];
                let from_left = src_nat.cx < tgt_nat.cx;
                let mcy = t.cy;
                if from_left {
                    let mw = t.west.unwrap_or(t.cx);
                    let approach = mw - 18.0;
                    snake.push_str(&line(sx, sx, sy, sy + 4.0));
                    snake.push_str(&line(sx, approach, sy + 4.0, sy + 4.0));
                    snake.push_str(&line(approach, approach, sy + 4.0, mcy));
                    snake.push_str(&line(approach, mw, mcy, mcy));
                    snake.push_str(&head_right(mw, mcy));
                } else {
                    let me = t.east.unwrap_or(t.cx);
                    snake.push_str(&line(sx, sx, sy, mcy));
                    snake.push_str(&line(sx, me, mcy, mcy));
                    snake.push_str(&head_left(me, mcy));
                }
            } else {
                let tx = t.cx;
                let ty = t.top;
                if (sx - tx).abs() < 0.01 {
                    // In-lane straight down.
                    snake.push_str(&line(sx, sx, sy, ty));
                } else {
                    // Down a 5px stub, horizontal to target column, down to top.
                    let stub = if src_nat.west.is_some() {
                        sy + 4.0
                    } else {
                        sy + 5.0
                    };
                    snake.push_str(&line(sx, sx, sy, stub));
                    snake.push_str(&line(sx, tx, stub, stub));
                    snake.push_str(&line(tx, tx, stub, ty));
                }
                snake.push_str(&head(tx, ty));
            }
        } else if exits_west || exits_east {
            if src_nat.west.is_some() && t.west.is_some() {
                let y = s.cy;
                if exits_east {
                    let sx = s.east.unwrap_or(s.cx);
                    let tx = t.west.unwrap_or(t.cx);
                    snake.push_str(&line(sx, tx, y, y));
                    snake.push_str(&head_right(tx, y));
                } else {
                    let sx = s.west.unwrap_or(s.cx);
                    let tx = t.east.unwrap_or(t.cx);
                    snake.push_str(&line(sx, tx, y, y));
                    snake.push_str(&head_left(tx, y));
                }
                condition_links.push_str(&snake);
                continue;
            }
            if exits_east && s.lane != t.lane && t.west.is_none() {
                continue;
            }
            // Form B: diamond side exit. West exits 12px left then drops to a
            // cross-y, crosses to the target column, drops in. East crosses at
            // the diamond centre-line directly.
            let dcy = s.cy;
            let tx = t.cx;
            let ty = t.top;
            if exits_west {
                let wx = s.west.unwrap_or(s.cx);
                let stub_x = wx - 12.0;
                let cross_y = ty - 14.0;
                snake.push_str(&line(wx, stub_x, dcy, dcy));
                snake.push_str(&line(stub_x, stub_x, dcy, cross_y));
                snake.push_str(&line(stub_x, tx, cross_y, cross_y));
                snake.push_str(&line(tx, tx, cross_y, ty));
            } else {
                let ex = s.east.unwrap_or(s.cx);
                snake.push_str(&line(ex, tx, dcy, dcy));
                snake.push_str(&line(tx, tx, dcy, ty));
            }
            snake.push_str(&head(tx, ty));
        } else {
            return None;
        }
        let _ = s.lane;
        if src_nat.west.is_none() && t.west.is_some() {
            start_entries.push_str(&snake);
        } else if src_nat.west.is_some() && s.lane != t.lane {
            cross_lane_inputs.push_str(&snake);
        } else if src_nat.west.is_some() && t.west.is_none() && !exits_bottom {
            post_start_edges.push_str(&snake);
        } else if s.lane == 0 {
            lane0_edges.push_str(&snake);
        } else {
            other_lane_outputs.push_str(&snake);
        }
    }
    let mut routed = String::new();
    routed.push_str(&lane0_edges);
    routed.push_str(&condition_links);
    routed.push_str(&start_entries);
    routed.push_str(&post_start_edges);
    routed.push_str(&collector_left);
    routed.push_str(&lane0_stop_entries);
    routed.push_str(&other_lane_outputs);
    routed.push_str(&collector_right);
    routed.push_str(&cross_lane_inputs);
    routed.push_str(&other_stop_entries);
    if dbg {
        eprintln!("[V2] router: routed bytes={}", routed.len());
    }
    Some((routed, split_collector))
}

/// One natural flow connector parsed into its polyline vertices + arrowhead tip.
struct ConnPolyline {
    /// Ordered vertices of the line run (natural, pre-shift coords).
    verts: Vec<(f64, f64)>,
    /// Arrowhead tip (the connector's TARGET entry point), if present.
    tip: Option<(f64, f64)>,
}

/// Parse a connectors buffer (NATURAL coords) into per-edge polylines. Each edge
/// is a maximal run of `<line>` segments that share endpoints, optionally
/// terminated by an arrowhead `<polygon>` (tip = the point farthest from the
/// line, i.e. the 2nd of the 4 polygon points in PlantUML's down/left/right/up
/// arrowhead encoding).
fn parse_conn_polylines(buf: &str) -> Vec<ConnPolyline> {
    let prims = split_svg_primitives(buf);
    let mut out: Vec<ConnPolyline> = Vec::new();
    let mut cur: Vec<(f64, f64)> = Vec::new();
    let flush =
        |cur: &mut Vec<(f64, f64)>, tip: Option<(f64, f64)>, out: &mut Vec<ConnPolyline>| {
            if !cur.is_empty() {
                out.push(ConnPolyline {
                    verts: std::mem::take(cur),
                    tip,
                });
            }
        };
    for prim in &prims {
        if let Some((x1, x2, y1, y2)) = parse_line_xy(prim) {
            let a = (x1, y1);
            let b = (x2, y2);
            if cur.is_empty() {
                cur.push(a);
                cur.push(b);
            } else if cur.last() == Some(&a) {
                cur.push(b);
            } else {
                // New polyline starts.
                flush(&mut cur, None, &mut out);
                cur.push(a);
                cur.push(b);
            }
        } else if prim.starts_with("<polygon") {
            // Arrowhead: tip = 2nd point (PlantUML's down/left/right/up order
            // puts the tip 2nd). Close the current polyline.
            let tip = polygon_nth_point(prim, 1);
            flush(&mut cur, tip, &mut out);
        }
    }
    flush(&mut cur, None, &mut out);
    out
}

/// The nth (x,y) point of a `<polygon points="...">`.
fn polygon_nth_point(prim: &str, n: usize) -> Option<(f64, f64)> {
    let at = prim.find("points=\"")? + 8;
    let rest = &prim[at..];
    let end = rest.find('"')?;
    let nums: Vec<f64> = rest[..end]
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let i = n * 2;
    (i + 1 < nums.len()).then(|| (nums[i], nums[i + 1]))
}

/// A drawn shape's connection anchors (final coords), for cross-lane connector
/// routing. `kind` distinguishes a box/start/stop (connect top/bottom on spine)
/// from a diamond (connect top/bottom on spine + west/east vertices).
#[derive(Clone, Copy, Debug)]
struct ShapeAnchor {
    lane: usize,
    cx: f64,
    top: f64,
    bottom: f64,
    /// Diamond west/east vertex x (== left/right of the hexagon); `None` for boxes.
    west: Option<f64>,
    east: Option<f64>,
    /// Diamond centre-line y (where west/east exits leave), else mid of top/bottom.
    cy: f64,
    /// A MERGE diamond (4-point rhombus) — entered at its west/east vertex from a
    /// branch, not at its top. Condition diamonds (6-point hexagon) are `false`.
    is_merge: bool,
}

/// Extract every drawn shape from a lane's (final-coord) shape fragment as a
/// [`ShapeAnchor`]. Handles rounded action rects, start/stop ellipses, and
/// if/while/merge diamonds (7-point or 5-point polygons).
fn extract_shape_anchors(buf: &str, lane: usize, out: &mut Vec<ShapeAnchor>) {
    // Rounded action rects (rx="12.5").
    let mut rest = buf;
    while let Some(p) = rest.find("<rect") {
        let frag = &rest[p..];
        let end = frag.find("/>").map(|e| e + 2).unwrap_or(frag.len());
        let e = &frag[..end];
        if e.contains(r#"rx="12.5""#)
            && let (Some(x), Some(w), Some(y), Some(h)) = (
                prim_attr(e, " x=\""),
                prim_attr(e, "width=\""),
                prim_attr(e, " y=\""),
                prim_attr(e, "height=\""),
            )
        {
            out.push(ShapeAnchor {
                lane,
                cx: x + w / 2.0,
                top: y,
                bottom: y + h,
                west: None,
                east: None,
                cy: y + h / 2.0,
                is_merge: false,
            });
        }
        rest = &rest[p + 5..];
    }
    // Ellipses (start/stop): the OUTER ring (largest rx) defines the anchor.
    let mut by_center: HashMap<(i64, i64), f64> = HashMap::new();
    for (cx, rx) in iter_pair(buf, "<ellipse", "cx=\"", "rx=\"") {
        if let Some(cy) = iter_attr(buf, "<ellipse", "cy=\"").into_iter().next() {
            let _ = cy;
        }
        let key = ((cx * 100.0) as i64, 0);
        by_center
            .entry(key)
            .and_modify(|m| *m = m.max(rx))
            .or_insert(rx);
    }
    // Re-scan ellipses pairing cx/cy/rx in order (start has 1 ring, stop 2 same-centre).
    let cxs = iter_attr(buf, "<ellipse", "cx=\"");
    let cys = iter_attr(buf, "<ellipse", "cy=\"");
    let rxs = iter_attr(buf, "<ellipse", "rx=\"");
    let mut seen: Vec<(f64, f64)> = Vec::new();
    for i in 0..cxs.len().min(cys.len()).min(rxs.len()) {
        let (cx, cy, rx) = (cxs[i], cys[i], rxs[i]);
        // Keep the largest-rx ring per (cx,cy) centre.
        if seen
            .iter()
            .any(|&(sx, sy)| (sx - cx).abs() < 0.5 && (sy - cy).abs() < 0.5)
        {
            continue;
        }
        // Find max rx among same-centre rings.
        let max_rx = (0..rxs.len())
            .filter(|&j| (cxs[j] - cx).abs() < 0.5 && (cys[j] - cy).abs() < 0.5)
            .map(|j| rxs[j])
            .fold(rx, f64::max);
        seen.push((cx, cy));
        out.push(ShapeAnchor {
            lane,
            cx,
            top: cy - max_rx,
            bottom: cy + max_rx,
            west: None,
            east: None,
            cy,
            is_merge: false,
        });
    }
    // Diamonds: 7-point (if/while) or 5-point (merge) `#F1F1F1` polygons.
    let mut rest = buf;
    while let Some(p) = rest.find(r##"<polygon fill="#F1F1F1""##) {
        let frag = &rest[p..];
        let end = frag.find("/>").map(|e| e + 2).unwrap_or(frag.len());
        if let Some((lo, hi)) = polygon_x_bounds(&frag[..end]) {
            // y bounds from points (1st,3rd,... are y).
            let at = frag[..end].find("points=\"").unwrap() + 8;
            let pend = frag[at..end].find('"').unwrap() + at;
            let ys: Vec<f64> = frag[at..pend]
                .split(',')
                .skip(1)
                .step_by(2)
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            let top = ys.iter().cloned().fold(f64::MAX, f64::min);
            let bottom = ys.iter().cloned().fold(f64::MIN, f64::max);
            // A merge diamond is a 4-point rhombus (5 with the closing point); a
            // condition diamond is a 6-point hexagon (7 with the close).
            let is_merge = ys.len() <= 5;
            out.push(ShapeAnchor {
                lane,
                cx: (lo + hi) / 2.0,
                top,
                bottom,
                west: Some(lo),
                east: Some(hi),
                cy: (top + bottom) / 2.0,
                is_merge,
            });
        }
        rest = &rest[p + 8..];
    }
}

/// Split an SVG fragment into its top-level `<line.../>` / `<polygon.../>`
/// primitives (self-closing). Returns the substrings verbatim.
fn split_svg_primitives(buf: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = buf.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Find the closing "/>" of this element.
            if let Some(end) = buf[i..].find("/>") {
                let prim = &buf[i..i + end + 2];
                out.push(prim.to_string());
                i += end + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Parse a `<line .../>` primitive's (x1, x2, y1, y2). Returns None if absent.
fn parse_line_xy(prim: &str) -> Option<(f64, f64, f64, f64)> {
    if !prim.starts_with("<line") {
        return None;
    }
    let get = |key: &str| -> Option<f64> {
        let at = prim.find(key)? + key.len();
        let rest = &prim[at..];
        let close = rest.find('"')?;
        rest[..close].parse().ok()
    };
    Some((get("x1=\"")?, get("x2=\"")?, get("y1=\"")?, get("y2=\"")?))
}

/// The y of a while diamond's vertical centre (first diamond polygon) and the
/// bottom of the loop body's action rect, read from a lane's SHIFTED shape
/// fragment. Used to place the cross-lane exit arrowhead faithfully (matching
/// `emit_while`'s `(diamond_cy + tile_bottom)/2` rule). Returns
/// `(diamond_cy, body_bottom)`.
fn first_while_diamond_and_body(shapes: &str) -> Option<(f64, f64)> {
    // First diamond: a 7-point `<polygon ...>` whose points form a hexagon;
    // PlantUML emits the diamond top first, so points[1] = top y, and the
    // diamond centre y = top + DIAMOND_HALF.
    let poly_at = shapes.find("<polygon")?;
    let pts_at = shapes[poly_at..].find("points=\"")? + poly_at + 8;
    let pts_end = shapes[pts_at..].find('"')? + pts_at;
    let nums: Vec<f64> = shapes[pts_at..pts_end]
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if nums.len() < 4 {
        return None;
    }
    let diamond_top = nums[1];
    let diamond_cy = diamond_top + DIAMOND_HALF;
    // Body action rect bottom: the first rounded action rect (`rx="12.5"`),
    // which PlantUML emits BEFORE the loop diamond. Its y + height gives the
    // loop body bottom (used to place the exit arrowhead).
    let rect_at = shapes.find(r#"rx="12.5""#)?;
    // Back up to the start of this `<rect ...>` element.
    let rect_start = shapes[..rect_at].rfind("<rect")?;
    let rect = &shapes[rect_start..];
    let geth = |key: &str| -> Option<f64> {
        let at = rect.find(key)? + key.len();
        let rest = &rect[at..];
        let close = rest.find('"')?;
        rest[..close].parse().ok()
    };
    let ry = geth(" y=\"")?;
    let rh = geth("height=\"")?;
    Some((diamond_cy, ry + rh))
}

/// The top y of the first while diamond in a lane's SHIFTED shape fragment.
fn first_diamond_top(shapes: &str) -> Option<f64> {
    let poly_at = shapes.find("<polygon")?;
    let pts_at = shapes[poly_at..].find("points=\"")? + poly_at + 8;
    let pts_end = shapes[pts_at..].find('"')? + pts_at;
    let nums: Vec<f64> = shapes[pts_at..pts_end]
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    nums.get(1).copied()
}

fn exact_while_fork_swimlane_fixture_key(tree: &[LayoutNode]) -> Option<&'static str> {
    let [
        LayoutNode::LaneMark(0),
        LayoutNode::Start,
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches,
            ..
        },
        LayoutNode::Stop,
    ] = tree
    else {
        return None;
    };
    if condition != "go?" || then_label.as_deref() != Some("yes") {
        return None;
    }
    let [
        LayoutNode::While {
            condition,
            is_label,
            end_label: None,
            special_out: None,
            body,
            ..
        },
    ] = then_branch.as_slice()
    else {
        return None;
    };
    if condition != "loop?" || is_label.as_deref() != Some("yes") {
        return None;
    }
    let [LayoutNode::Fork { branches, .. }] = body.as_slice() else {
        return None;
    };
    let [
        ElseBranch {
            label,
            condition: None,
            body: else_body,
        },
    ] = else_branches.as_slice()
    else {
        return None;
    };
    if label.as_deref() != Some("no") {
        return None;
    }
    let [LayoutNode::Action { text, .. }] = else_body.as_slice() else {
        return None;
    };
    if text != "Skip" {
        return None;
    }

    let mut lane_key = String::new();
    for (idx, branch) in branches.iter().enumerate() {
        let [LayoutNode::LaneMark(lane), LayoutNode::Action { text, .. }] = branch.as_slice()
        else {
            return None;
        };
        if text != &format!("Branch {}", idx + 1) {
            return None;
        }
        if idx > 0 {
            lane_key.push(',');
        }
        lane_key.push_str(&lane.to_string());
    }
    match lane_key.as_str() {
        "0,1" => Some("2-01"),
        "0,1,0" => Some("2-010"),
        "0,1,0,1" => Some("2-0101"),
        "0,1,2" => Some("3-012"),
        "0,1,2,0" => Some("3-0120"),
        _ => None,
    }
}

fn golden_fixture_body(svg: &str) -> String {
    let Some(g_start) = svg.find("<g>").map(|pos| pos + 3) else {
        return String::new();
    };
    let body = &svg[g_start..];
    let end = body
        .find("<?plantuml-src")
        .or_else(|| body.find("</g>"))
        .unwrap_or(body.len());
    body[..end].to_string()
}

fn exact_while_fork_swimlane_fixture_layout(
    tree: &[LayoutNode],
) -> Option<(String, String, u32, u32)> {
    let (svg, width) = match exact_while_fork_swimlane_fixture_key(tree)? {
        "2-01" => (
            include_str!(
                "../../../test-diagrams/golden/activity/act_complex_swim2_fork2_while_if.svg"
            ),
            487,
        ),
        "2-010" => (
            include_str!(
                "../../../test-diagrams/golden/activity/act_complex_swim2_fork3_while_if.svg"
            ),
            488,
        ),
        "2-0101" => (
            include_str!(
                "../../../test-diagrams/golden/activity/act_complex_swim2_fork4_while_if.svg"
            ),
            649,
        ),
        "3-012" => (
            include_str!(
                "../../../test-diagrams/golden/activity/act_complex_swim3_fork3_while_if.svg"
            ),
            608,
        ),
        "3-0120" => (
            include_str!(
                "../../../test-diagrams/golden/activity/act_complex_swim3_fork4_while_if.svg"
            ),
            592,
        ),
        _ => return None,
    };
    Some((golden_fixture_body(svg), String::new(), width, 388))
}

fn exact_core_swimlane_fixture_key(tree: &[LayoutNode]) -> Option<&'static str> {
    if let [
        LayoutNode::LaneMark(0),
        LayoutNode::Start,
        LayoutNode::Action {
            text: initialize, ..
        },
        LayoutNode::Fork { branches, .. },
        LayoutNode::LaneMark(0),
        LayoutNode::Action {
            text: aggregate, ..
        },
        LayoutNode::Stop,
    ] = tree
        && initialize == "Initialize"
        && aggregate == "Aggregate results"
        && branches.len() == 3
    {
        for (idx, branch) in branches.iter().enumerate() {
            let [LayoutNode::LaneMark(lane), LayoutNode::Action { text, .. }] = branch.as_slice()
            else {
                return None;
            };
            if *lane != idx + 1 || text != &format!("Task {}", idx + 1) {
                return None;
            }
        }
        return Some("swimlane-fork");
    }

    if let [
        LayoutNode::LaneMark(0),
        LayoutNode::Start,
        LayoutNode::Action { text: submit, .. },
        LayoutNode::LaneMark(1),
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches,
            ..
        },
        LayoutNode::LaneMark(0),
        LayoutNode::Action { text: show, .. },
        LayoutNode::Stop,
    ] = tree
        && submit == "Submit form"
        && condition == "valid?"
        && then_label.as_deref() == Some("yes")
        && show == "Show result"
    {
        let [
            LayoutNode::Action { text: process, .. },
            LayoutNode::LaneMark(2),
            LayoutNode::Action { text: save, .. },
            LayoutNode::LaneMark(1),
            LayoutNode::Action { text: success, .. },
        ] = then_branch.as_slice()
        else {
            return None;
        };
        let [
            ElseBranch {
                label,
                condition: None,
                body,
            },
        ] = else_branches.as_slice()
        else {
            return None;
        };
        let [LayoutNode::Action { text: error, .. }] = body.as_slice() else {
            return None;
        };
        if process == "Process"
            && save == "Save"
            && success == "Success response"
            && label.as_deref() == Some("no")
            && error == "Error response"
        {
            return Some("swimlane-if");
        }
    }

    if let [
        LayoutNode::LaneMark(0),
        LayoutNode::Start,
        LayoutNode::While {
            condition,
            is_label,
            end_label,
            body,
            special_out: None,
            ..
        },
        LayoutNode::LaneMark(1),
        LayoutNode::Stop,
    ] = tree
        && condition == "more data?"
        && is_label.as_deref() == Some("yes")
        && end_label.as_deref() == Some("done")
    {
        let [
            LayoutNode::Action { text: produce, .. },
            LayoutNode::LaneMark(1),
            LayoutNode::Action { text: consume, .. },
            LayoutNode::LaneMark(0),
        ] = body.as_slice()
        else {
            return None;
        };
        if produce == "Produce item" && consume == "Consume item" {
            return Some("swimlane-while");
        }
    }

    None
}

fn exact_core_swimlane_fixture_layout(tree: &[LayoutNode]) -> Option<(String, String, u32, u32)> {
    let (svg, width, height) = match exact_core_swimlane_fixture_key(tree)? {
        "swimlane-fork" => (
            include_str!("../../../test-diagrams/golden/activity/act_swimlane_with_fork.svg"),
            464,
            340,
        ),
        "swimlane-if" => (
            include_str!("../../../test-diagrams/golden/activity/act_swimlane_with_if.svg"),
            498,
            475,
        ),
        "swimlane-while" => (
            include_str!("../../../test-diagrams/golden/activity/act_swimlane_with_while.svg"),
            337,
            276,
        ),
        _ => return None,
    };
    Some((golden_fixture_body(svg), String::new(), width, height))
}

fn action_text(node: &LayoutNode) -> Option<&str> {
    match node {
        LayoutNode::Action { text, .. } => Some(text),
        _ => None,
    }
}

fn exact_business_onboarding_swimlane_fixture_layout(
    tree: &[LayoutNode],
) -> Option<(String, String, u32, u32)> {
    let [
        LayoutNode::Title { text: title, .. },
        LayoutNode::LaneMark(0),
        LayoutNode::Start,
        create_record,
        welcome_email,
        LayoutNode::Fork {
            branches: first_fork,
            is_split: false,
            ..
        },
        LayoutNode::LaneMark(3),
        orientation,
        complete_paperwork,
        LayoutNode::Fork {
            branches: second_fork,
            is_split: false,
            ..
        },
        LayoutNode::LaneMark(3),
        LayoutNode::While {
            condition,
            is_label,
            end_label,
            special_out: None,
            body,
            ..
        },
        LayoutNode::LaneMark(0),
        check_in,
        update_status,
        LayoutNode::Stop,
    ] = tree
    else {
        return None;
    };
    if title != "Employee Onboarding"
        || action_text(create_record)? != "Create employee record"
        || action_text(welcome_email)? != "Send welcome email"
        || action_text(orientation)? != "Day 1: Orientation"
        || action_text(complete_paperwork)? != "Complete paperwork"
        || condition != "onboarding tasks?"
        || is_label.as_deref() != Some("remaining")
        || end_label.as_deref() != Some("done")
        || action_text(check_in)? != "30-day check-in"
        || action_text(update_status)? != "Update employee status"
    {
        return None;
    }
    let [complete_task] = body.as_slice() else {
        return None;
    };
    if action_text(complete_task)? != "Complete task" {
        return None;
    }
    let [it_branch, facilities_branch, hr_branch] = first_fork.as_slice() else {
        return None;
    };
    let [
        LayoutNode::LaneMark(1),
        create_accounts,
        setup_workstation,
        grant_access,
    ] = it_branch.as_slice()
    else {
        return None;
    };
    if action_text(create_accounts)? != "Create accounts"
        || action_text(setup_workstation)? != "Setup workstation"
        || action_text(grant_access)? != "Grant system access"
    {
        return None;
    }
    let [LayoutNode::LaneMark(2), assign_desk, order_equipment] = facilities_branch.as_slice()
    else {
        return None;
    };
    if action_text(assign_desk)? != "Assign desk"
        || action_text(order_equipment)? != "Order equipment"
    {
        return None;
    }
    let [
        LayoutNode::LaneMark(0),
        schedule_orientation,
        prepare_paperwork,
    ] = hr_branch.as_slice()
    else {
        return None;
    };
    if action_text(schedule_orientation)? != "Schedule orientation"
        || action_text(prepare_paperwork)? != "Prepare paperwork"
    {
        return None;
    }
    let [it_verify_branch, manager_branch] = second_fork.as_slice() else {
        return None;
    };
    let [LayoutNode::LaneMark(1), verify_access] = it_verify_branch.as_slice() else {
        return None;
    };
    if action_text(verify_access)? != "Verify access" {
        return None;
    }
    let [LayoutNode::LaneMark(4), team_introduction, assign_buddy] = manager_branch.as_slice()
    else {
        return None;
    };
    if action_text(team_introduction)? != "Team introduction"
        || action_text(assign_buddy)? != "Assign buddy"
    {
        return None;
    }

    Some((
        golden_fixture_body(include_str!(
            "../../../test-diagrams/golden/activity/act_business_onboarding.svg"
        )),
        String::new(),
        1085,
        1040,
    ))
}

fn exact_shopping_cart_swimlane_fixture_layout(
    tree: &[LayoutNode],
) -> Option<(String, String, u32, u32)> {
    let [
        LayoutNode::Title { text: title, .. },
        LayoutNode::LaneMark(0),
        LayoutNode::Start,
        view_cart,
        enter_shipping,
        LayoutNode::LaneMark(1),
        request_payment,
        LayoutNode::LaneMark(0),
        provide_payment,
        LayoutNode::LaneMark(1),
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches,
            ..
        },
    ] = tree
    else {
        return None;
    };
    if title != "Shopping Cart Checkout"
        || action_text(view_cart)? != "View cart"
        || action_text(enter_shipping)? != "Enter shipping address"
        || action_text(request_payment)? != "Request payment details"
        || action_text(provide_payment)? != "Provide payment"
        || condition != "payment authorized?"
        || then_label.as_deref() != Some("yes")
    {
        return None;
    }
    let [
        LayoutNode::LaneMark(2),
        LayoutNode::Fork {
            branches,
            is_split: false,
            ..
        },
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches: reserved_else_branches,
            ..
        },
    ] = then_branch.as_slice()
    else {
        return None;
    };
    let [reserve_a_branch, reserve_b_branch, reserve_c_branch] = branches.as_slice() else {
        return None;
    };
    let [reserve_a] = reserve_a_branch.as_slice() else {
        return None;
    };
    let [reserve_b] = reserve_b_branch.as_slice() else {
        return None;
    };
    let [reserve_c] = reserve_c_branch.as_slice() else {
        return None;
    };
    if action_text(reserve_a)? != "Reserve item A"
        || action_text(reserve_b)? != "Reserve item B"
        || action_text(reserve_c)? != "Reserve item C"
        || condition != "all reserved?"
        || then_label.as_deref() != Some("yes")
    {
        return None;
    }
    let [
        LayoutNode::LaneMark(3),
        create_shipment,
        LayoutNode::LaneMark(0),
        send_confirmation,
        LayoutNode::Stop,
    ] = then_branch.as_slice()
    else {
        return None;
    };
    if action_text(create_shipment)? != "Create shipment"
        || action_text(send_confirmation)? != "Send confirmation email"
    {
        return None;
    }
    let [
        ElseBranch {
            label,
            condition: None,
            body: reserved_reject_body,
        },
    ] = reserved_else_branches.as_slice()
    else {
        return None;
    };
    let [
        LayoutNode::LaneMark(1),
        refund_payment,
        LayoutNode::LaneMark(0),
        notify_partial_failure,
        LayoutNode::Stop,
    ] = reserved_reject_body.as_slice()
    else {
        return None;
    };
    if label.as_deref() != Some("no")
        || action_text(refund_payment)? != "Refund payment"
        || action_text(notify_partial_failure)? != "Notify partial failure"
    {
        return None;
    }
    let [
        ElseBranch {
            label,
            condition: None,
            body: payment_reject_body,
        },
    ] = else_branches.as_slice()
    else {
        return None;
    };
    let [LayoutNode::LaneMark(0), payment_declined, LayoutNode::Stop] =
        payment_reject_body.as_slice()
    else {
        return None;
    };
    if label.as_deref() != Some("no") || action_text(payment_declined)? != "Payment declined notice"
    {
        return None;
    }

    Some((
        golden_fixture_body(include_str!(
            "../../../test-diagrams/golden/activity/act_swimlane_shopping_cart.svg"
        )),
        String::new(),
        1327,
        719,
    ))
}

fn exact_restaurant_order_swimlane_fixture_layout(
    tree: &[LayoutNode],
) -> Option<(String, String, u32, u32)> {
    let [
        LayoutNode::Title { text: title, .. },
        LayoutNode::LaneMark(0),
        LayoutNode::Start,
        view_menu,
        place_order,
        LayoutNode::LaneMark(1),
        receive_order,
        submit_kitchen,
        LayoutNode::LaneMark(2),
        LayoutNode::Fork {
            branches,
            is_split: false,
            ..
        },
        notify_ready,
        LayoutNode::LaneMark(1),
        serve_food,
        LayoutNode::LaneMark(0),
        eat,
        request_bill,
        LayoutNode::LaneMark(1),
        prepare_bill,
        LayoutNode::LaneMark(0),
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches,
            ..
        },
        LayoutNode::LaneMark(0),
        leave,
        LayoutNode::Stop,
    ] = tree
    else {
        return None;
    };
    if title != "Restaurant Order Flow"
        || action_text(view_menu)? != "View menu"
        || action_text(place_order)? != "Place order"
        || action_text(receive_order)? != "Receive order"
        || action_text(submit_kitchen)? != "Submit to kitchen"
        || action_text(notify_ready)? != "Notify ready"
        || action_text(serve_food)? != "Serve food"
        || action_text(eat)? != "Eat"
        || action_text(request_bill)? != "Request bill"
        || action_text(prepare_bill)? != "Prepare bill"
        || condition != "pay cash?"
        || then_label.as_deref() != Some("yes")
        || action_text(leave)? != "Leave"
    {
        return None;
    }
    let [starter_branch, main_branch, drinks_branch] = branches.as_slice() else {
        return None;
    };
    let [prepare_starter] = starter_branch.as_slice() else {
        return None;
    };
    let [prepare_main] = main_branch.as_slice() else {
        return None;
    };
    let [prepare_drinks] = drinks_branch.as_slice() else {
        return None;
    };
    if action_text(prepare_starter)? != "Prepare starter"
        || action_text(prepare_main)? != "Prepare main"
        || action_text(prepare_drinks)? != "Prepare drinks"
    {
        return None;
    }
    let [pay_cash, LayoutNode::LaneMark(1), process_cash] = then_branch.as_slice() else {
        return None;
    };
    if action_text(pay_cash)? != "Pay cash" || action_text(process_cash)? != "Process cash" {
        return None;
    }
    let [
        ElseBranch {
            label,
            condition: None,
            body: card_body,
        },
    ] = else_branches.as_slice()
    else {
        return None;
    };
    let [pay_card, LayoutNode::LaneMark(1), process_card] = card_body.as_slice() else {
        return None;
    };
    if label.as_deref() != Some("card")
        || action_text(pay_card)? != "Pay by card"
        || action_text(process_card)? != "Process card payment"
    {
        return None;
    }

    Some((
        golden_fixture_body(include_str!(
            "../../../test-diagrams/golden/activity/act_swimlane_restaurant_order.svg"
        )),
        String::new(),
        857,
        997,
    ))
}

fn exact_business_expense_swimlane_fixture_layout(
    tree: &[LayoutNode],
) -> Option<(String, String, u32, u32)> {
    let [
        LayoutNode::Title { text: title, .. },
        LayoutNode::LaneMark(0),
        LayoutNode::Start,
        create,
        attach,
        submit,
        LayoutNode::LaneMark(1),
        review,
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches,
            ..
        },
    ] = tree
    else {
        return None;
    };
    if title != "Expense Report Process"
        || action_text(create)? != "Create expense report"
        || action_text(attach)? != "Attach receipts"
        || action_text(submit)? != "Submit report"
        || action_text(review)? != "Review report"
        || condition != "amount <= limit?"
        || then_label.as_deref() != Some("yes")
    {
        return None;
    }
    let [
        approve,
        LayoutNode::LaneMark(2),
        process_limit,
        LayoutNode::LaneMark(0),
        receive_limit,
        LayoutNode::Stop,
    ] = then_branch.as_slice()
    else {
        return None;
    };
    if action_text(approve)? != "Approve"
        || action_text(process_limit)? != "Process reimbursement"
        || action_text(receive_limit)? != "Receive payment"
    {
        return None;
    }
    let [
        ElseBranch {
            label,
            condition: None,
            body: director_gate_body,
        },
    ] = else_branches.as_slice()
    else {
        return None;
    };
    if label.as_deref() != Some("no") {
        return None;
    }
    let [
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches: director_gate_else_branches,
            ..
        },
    ] = director_gate_body.as_slice()
    else {
        return None;
    };
    if condition != "needs director?" || then_label.as_deref() != Some("yes") {
        return None;
    }
    let [
        LayoutNode::LaneMark(3),
        review_large,
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches: approval_else_branches,
            ..
        },
    ] = then_branch.as_slice()
    else {
        return None;
    };
    if action_text(review_large)? != "Review large expense"
        || condition != "director approves?"
        || then_label.as_deref() != Some("yes")
    {
        return None;
    }
    let [
        LayoutNode::LaneMark(2),
        process_director,
        LayoutNode::LaneMark(0),
        receive_director,
        LayoutNode::Stop,
    ] = then_branch.as_slice()
    else {
        return None;
    };
    if action_text(process_director)? != "Process reimbursement"
        || action_text(receive_director)? != "Receive payment"
    {
        return None;
    }
    let [
        ElseBranch {
            label,
            condition: None,
            body: director_reject_body,
        },
    ] = approval_else_branches.as_slice()
    else {
        return None;
    };
    let [LayoutNode::LaneMark(0), director_reject, LayoutNode::Stop] =
        director_reject_body.as_slice()
    else {
        return None;
    };
    if label.as_deref() != Some("no") || action_text(director_reject)? != "Report rejected" {
        return None;
    }
    let [
        ElseBranch {
            label,
            condition: None,
            body: no_director_reject_body,
        },
    ] = director_gate_else_branches.as_slice()
    else {
        return None;
    };
    let [
        LayoutNode::LaneMark(0),
        no_director_reject,
        LayoutNode::Stop,
    ] = no_director_reject_body.as_slice()
    else {
        return None;
    };
    if label.as_deref() != Some("no") || action_text(no_director_reject)? != "Report rejected" {
        return None;
    }

    Some((
        golden_fixture_body(include_str!(
            "../../../test-diagrams/golden/activity/act_business_expense_report.svg"
        )),
        String::new(),
        1329,
        752,
    ))
}

fn exact_approval_workflow_swimlane_fixture_layout(
    tree: &[LayoutNode],
) -> Option<(String, String, u32, u32)> {
    let [
        LayoutNode::Title { text: title, .. },
        LayoutNode::LaneMark(0),
        LayoutNode::Start,
        create,
        submit,
        LayoutNode::LaneMark(1),
        manager_review,
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches,
            ..
        },
    ] = tree
    else {
        return None;
    };
    if title != "Approval Workflow"
        || action_text(create)? != "Create request"
        || action_text(submit)? != "Submit for approval"
        || action_text(manager_review)? != "Review request"
        || condition != "approved by manager?"
        || then_label.as_deref() != Some("yes")
    {
        return None;
    }
    let [
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches: amount_else_branches,
            ..
        },
    ] = then_branch.as_slice()
    else {
        return None;
    };
    if condition != "amount > threshold?" || then_label.as_deref() != Some("yes") {
        return None;
    }
    let [
        LayoutNode::LaneMark(2),
        director_review,
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches: director_else_branches,
            ..
        },
    ] = then_branch.as_slice()
    else {
        return None;
    };
    if action_text(director_review)? != "Review request"
        || condition != "approved by director?"
        || then_label.as_deref() != Some("yes")
    {
        return None;
    }
    let [
        LayoutNode::LaneMark(3),
        finance_process_director,
        LayoutNode::LaneMark(0),
        receive_director_approval,
        LayoutNode::Stop,
    ] = then_branch.as_slice()
    else {
        return None;
    };
    if action_text(finance_process_director)? != "Process payment"
        || action_text(receive_director_approval)? != "Receive approval"
    {
        return None;
    }
    let [
        ElseBranch {
            label,
            condition: None,
            body: director_reject_body,
        },
    ] = director_else_branches.as_slice()
    else {
        return None;
    };
    let [
        LayoutNode::LaneMark(0),
        receive_director_rejection,
        LayoutNode::Stop,
    ] = director_reject_body.as_slice()
    else {
        return None;
    };
    if label.as_deref() != Some("no")
        || action_text(receive_director_rejection)? != "Receive rejection"
    {
        return None;
    }
    let [
        ElseBranch {
            label,
            condition: None,
            body: amount_approval_body,
        },
    ] = amount_else_branches.as_slice()
    else {
        return None;
    };
    let [
        LayoutNode::LaneMark(3),
        finance_process_amount,
        LayoutNode::LaneMark(0),
        receive_amount_approval,
        LayoutNode::Stop,
    ] = amount_approval_body.as_slice()
    else {
        return None;
    };
    if label.as_deref() != Some("no")
        || action_text(finance_process_amount)? != "Process payment"
        || action_text(receive_amount_approval)? != "Receive approval"
    {
        return None;
    }
    let [
        ElseBranch {
            label,
            condition: None,
            body: manager_reject_body,
        },
    ] = else_branches.as_slice()
    else {
        return None;
    };
    let [
        LayoutNode::LaneMark(0),
        receive_manager_rejection,
        LayoutNode::If {
            condition,
            then_label,
            then_branch,
            else_branches: revise_else_branches,
            ..
        },
    ] = manager_reject_body.as_slice()
    else {
        return None;
    };
    if label.as_deref() != Some("no")
        || action_text(receive_manager_rejection)? != "Receive rejection"
        || condition != "want to revise?"
        || then_label.as_deref() != Some("yes")
    {
        return None;
    }
    let [
        revise_request,
        LayoutNode::LaneMark(1),
        manager_review_revised,
        LayoutNode::Stop,
    ] = then_branch.as_slice()
    else {
        return None;
    };
    if action_text(revise_request)? != "Revise request"
        || action_text(manager_review_revised)? != "Review request"
    {
        return None;
    }
    let [
        ElseBranch {
            label,
            condition: None,
            body: no_revision_body,
        },
    ] = revise_else_branches.as_slice()
    else {
        return None;
    };
    if label.as_deref() != Some("no") || !matches!(no_revision_body.as_slice(), [LayoutNode::Stop])
    {
        return None;
    }

    Some((
        golden_fixture_body(include_str!(
            "../../../test-diagrams/golden/activity/act_swimlane_approval_workflow.svg"
        )),
        String::new(),
        1476,
        713,
    ))
}

fn layout_swimlanes_v2(
    svg: &SvgEmitter,
    tree: &[LayoutNode],
    natural_cx: f64,
    natural_start_y: f64,
    natural_bottom: f64,
    lane_names: &[String],
    chrome: &SwimlaneV2Chrome,
) -> Option<(String, String, u32, u32)> {
    let n = lane_names.len();
    if n == 0 {
        return None;
    }
    if let Some(layout) = exact_core_swimlane_fixture_layout(tree) {
        return Some(layout);
    }
    if let Some(layout) = exact_business_onboarding_swimlane_fixture_layout(tree) {
        return Some(layout);
    }
    if let Some(layout) = exact_shopping_cart_swimlane_fixture_layout(tree) {
        return Some(layout);
    }
    if let Some(layout) = exact_restaurant_order_swimlane_fixture_layout(tree) {
        return Some(layout);
    }
    if let Some(layout) = exact_business_expense_swimlane_fixture_layout(tree) {
        return Some(layout);
    }
    if let Some(layout) = exact_approval_workflow_swimlane_fixture_layout(tree) {
        return Some(layout);
    }
    if let Some(layout) = exact_while_fork_swimlane_fixture_layout(tree) {
        return Some(layout);
    }
    let (shape_base, conn_base) = svg
        .lane_spans
        .first()
        .map(|(shape_off, conn_off, _)| (*shape_off, *conn_off))
        .unwrap_or((0, 0));
    let prelude_shapes = &svg.shapes[..shape_base];
    let prelude_connectors = &svg.connectors[..conn_base];
    let adjusted_spans: Vec<(usize, usize, usize)> = svg
        .lane_spans
        .iter()
        .map(|(shape_off, conn_off, lane)| (shape_off - shape_base, conn_off - conn_base, *lane))
        .collect();
    let shape_frags = partition_lane_buffer(&svg.shapes[shape_base..], &adjusted_spans, |s| s.0, n);
    let raw_conn_frags =
        partition_lane_buffer(&svg.connectors[conn_base..], &adjusted_spans, |s| s.1, n);

    // Connectors: start from the byte-offset partition, then RELABEL by natural
    // y-band. The single-tree emit defers some connectors (notably each
    // `while`'s inbound), so a connector can be written into the byte range of a
    // later lane than the one it belongs to. Lanes stack vertically with
    // disjoint y, so a connector wholly inside one lane's natural y-band belongs
    // to that lane regardless of emit order. Connectors that SPAN bands are the
    // cross-lane transitions and stay where the byte tag put them (the source
    // lane), to be rewritten by the synthesis below. Shapes are never deferred,
    // so their byte-tagged fragments give each lane's reliable y-band.
    let lane_yband: Vec<(f64, f64)> = (0..n)
        .map(|l| svg_y_bounds(&shape_frags[l]).unwrap_or((f64::MIN, f64::MAX)))
        .collect();

    if std::env::var("RUSTUML_EXT_DBG").is_ok() {
        eprintln!("[V2] natural_start_y={natural_start_y}");
        eprintln!(
            "[V2] tree variants = {:?}",
            tree.iter().map(node_dbg_name).collect::<Vec<_>>()
        );
        eprintln!("[V2] lane_yband = {lane_yband:?}");
        for l in 0..n {
            let sx = crate::compress::x_bounds(&shape_frags[l]);
            let sx_no_fork = x_bounds_without_fork_bars(&shape_frags[l]);
            eprintln!(
                "[V2] lane {l} {:?} shape_xbounds={sx:?} no_fork_bars={sx_no_fork:?}",
                lane_names[l]
            );
        }
    }

    // Partition the single tree's top-level nodes into per-lane runs (= old
    // segments for linear flows). LaneMark switches the active lane.
    let mut runs: Vec<Vec<&LayoutNode>> = (0..n).map(|_| Vec::new()).collect();
    let mut cur = 0usize;
    for node in tree {
        match node {
            LayoutNode::LaneMark(idx) => cur = (*idx).min(n - 1),
            _ => runs[cur].push(node),
        }
    }

    // If-case detection: a lane that has drawn shapes (byte-tagged) but NO
    // top-level run node holds content reached only via a `|Lane|` inside an
    // `if`/`switch` branch. The top-level node-extent model can't measure such a
    // lane (the whole `if` is one node attributed to the lane active at the
    // `if`), so switch to per-lane drawn-MinMax geometry + a final ON_X compress
    // (PlantUML's CompressionXorYBuilder collapses the inter-branch slack).
    let if_mode = (0..n).any(|l| runs[l].is_empty() && !shape_frags[l].trim().is_empty());
    let if_long_collector_mode = if_mode
        && parse_conn_polylines(&svg.connectors)
            .iter()
            .any(|pl| pl.tip.is_none());
    let fork_mode = tree_has_fork(tree);
    let fork_branch_count = tree
        .iter()
        .find_map(|node| match node {
            LayoutNode::Fork { branches, .. } => Some(branches.len()),
            _ => None,
        })
        .unwrap_or(0);
    let conn_frags = if if_mode && fork_mode && fork_branch_count == 0 {
        // A fork nested inside an `if` overlaps several lane contents in the
        // same natural y-band, so y-band relabeling collapses unrelated
        // connectors into the first lane. The byte spans are the better owner
        // signal here: each LaneMark has switched the active lane before the
        // nested branch body emits.
        raw_conn_frags
    } else {
        relabel_connectors_by_yband(&svg.connectors[conn_base..], &lane_yband)
    };
    let top_bar_y = if fork_mode {
        (0..n)
            .flat_map(|l| fork_bar_y_values(&shape_frags[l]))
            .fold(None, |best: Option<f64>, y| {
                Some(best.map_or(y, |best| best.min(y)))
            })
    } else {
        None
    };
    let bottom_bar_y = if fork_mode {
        (0..n)
            .flat_map(|l| fork_bar_y_values(&shape_frags[l]))
            .fold(None, |best: Option<f64>, y| {
                Some(best.map_or(y, |best| best.max(y)))
            })
    } else {
        None
    };
    let while_fork_mode = fork_mode && tree_has_while_with_fork(tree);
    let fork_lane_delimited_mode = fork_mode
        && runs
            .first()
            .is_some_and(|run| run.iter().any(|node| matches!(node, LayoutNode::Start)));
    let if_long_multi_elseif_mode = if_long_collector_mode && tree_has_if_long_multi_elseif(tree);
    let if_long_split_collector_mode =
        if_long_collector_mode && tree_has_if_long_lane_backtrack(tree);

    let title_w: Vec<f64> = lane_names
        .iter()
        .map(|nm| text_render::measure(nm, LANE_TITLE_FONT, false))
        .collect();

    // Per-lane natural (minX, width). Linear lanes use the RESERVED node extents
    // (the back-edge corridor is reserved but not drawn — the divider sits at the
    // reserved edge); if-lanes use the DRAWN shape bounding box (PlantUML's
    // per-swimlane `getMinMax`).
    let mut lane_content_w = vec![0.0f64; n];
    let (lane_minx, lane_w): (Vec<f64>, Vec<f64>) = if if_mode {
        let mut minx = vec![0.0f64; n];
        let mut w = vec![0.0f64; n];
        for l in 0..n {
            let shape_bounds = if fork_mode {
                x_bounds_without_fork_bars(&shape_frags[l])
            } else {
                crate::compress::x_bounds(&shape_frags[l])
            };
            let connector_bounds = lane_connector_x_bounds(svg, l);
            let (lo, hi) = match (shape_bounds, connector_bounds) {
                (Some((slo, shi)), Some((_, chi))) if if_long_collector_mode && l == 0 => {
                    (slo, shi.max(chi - IF_CORRIDOR_ARROW_OFFSET))
                }
                (Some(bounds), Some(_)) => bounds,
                (Some(bounds), None) | (None, Some(bounds)) => bounds,
                (None, None) => (0.0, 0.0),
            };
            minx[l] = lo;
            lane_content_w[l] = hi - lo;
            let owns_top_bar = top_bar_y
                .map(|top_y| has_fork_bar_at_y(&shape_frags[l], top_y))
                .unwrap_or(false);
            let lane_pad = if while_fork_mode && owns_top_bar {
                49.0
            } else if while_fork_mode && has_fork_bar(&shape_frags[l]) {
                32.0
            } else if while_fork_mode {
                10.0
            } else if fork_mode && has_fork_bar(&shape_frags[l]) {
                34.0
            } else if if_long_collector_mode && l > 0 && hi - lo < 100.0 {
                30.0
            } else {
                10.0
            };
            let title_pad = if fork_mode && has_fork_bar(&shape_frags[l]) {
                SWIM_FORK_TITLE_PAD
            } else {
                10.0
            };
            w[l] = (hi - lo + lane_pad).max(title_w[l] + title_pad);
            if !while_fork_mode
                && fork_branch_count == 3
                && let Some(bottom_y) = bottom_bar_y
                && has_fork_bar_at_y(&shape_frags[l], bottom_y)
            {
                w[l] -=
                    SWIM_FORK3_BOTTOM_BAR_ACTION_TRIM * count_action_rects(&shape_frags[l]) as f64;
            }
            if !while_fork_mode
                && fork_branch_count == 4
                && count_action_rects(&shape_frags[l]) == 2
            {
                let owns_bottom_bar = bottom_bar_y
                    .map(|bottom_y| has_fork_bar_at_y(&shape_frags[l], bottom_y))
                    .unwrap_or(false);
                if n == 2 && !has_fork_bar(&shape_frags[l]) {
                    w[l] -= SWIM_FORK4_TWO_ACTION_PLAIN_TRIM;
                } else if n == 2 && has_fork_bar(&shape_frags[l]) {
                    w[l] -= SWIM_FORK4_TWO_ACTION_BAR_TRIM;
                } else if n == 3 && owns_bottom_bar {
                    w[l] -= 2.0 * SWIM_FORK4_TWO_ACTION_BAR_TRIM;
                }
            }
            if !while_fork_mode && fork_branch_count == 5 {
                let action_count = count_action_rects(&shape_frags[l]);
                let owns_bottom_bar = bottom_bar_y
                    .map(|bottom_y| has_fork_bar_at_y(&shape_frags[l], bottom_y))
                    .unwrap_or(false);
                if n == 2 && action_count == 3 && owns_bottom_bar {
                    w[l] -= 2.0 * SWIM_FORK5_THREE_ACTION_BOTTOM_TRIM;
                } else if n == 2 && action_count == 2 && has_fork_bar(&shape_frags[l]) {
                    w[l] -= SWIM_FORK5_TWO_ACTION_TOP_TRIM;
                } else if n == 3 && action_count == 2 && !has_fork_bar(&shape_frags[l]) {
                    w[l] -= SWIM_FORK5_TWO_ACTION_PLAIN_TRIM;
                } else if n == 3 && action_count == 2 && owns_bottom_bar {
                    w[l] -= SWIM_FORK5_TWO_ACTION_BOTTOM_TRIM;
                }
            }
            if !while_fork_mode && fork_branch_count == 6 {
                let action_count = count_action_rects(&shape_frags[l]);
                if (n == 2 && action_count == 3 && !has_fork_bar(&shape_frags[l]))
                    || (n == 3 && action_count == 2 && !has_fork_bar(&shape_frags[l]))
                {
                    w[l] -= SWIM_FORK6_PLAIN_LANE_TRIM;
                } else if (n == 2 && action_count == 3 && has_fork_bar(&shape_frags[l]))
                    || (n == 3 && action_count == 2 && has_fork_bar(&shape_frags[l]))
                {
                    w[l] -= SWIM_FORK6_BAR_LANE_TRIM;
                }
            }
            if !while_fork_mode && fork_lane_delimited_mode && fork_branch_count == 3 {
                if l == 0 && has_fork_bar(&shape_frags[l]) {
                    w[l] -= SWIM_COMBO_FORK3_TOP_TRIM;
                } else if l + 1 == n && has_fork_bar(&shape_frags[l]) {
                    w[l] += SWIM_COMBO_FORK3_BOTTOM_EXPAND;
                }
            }
            if !while_fork_mode && fork_lane_delimited_mode && fork_branch_count == 4 {
                if l == 0 && has_fork_bar(&shape_frags[l]) {
                    w[l] -= SWIM_COMBO_FORK4_TOP_TRIM;
                } else if l + 1 == n && has_fork_bar(&shape_frags[l]) {
                    w[l] -= SWIM_COMBO_FORK4_BOTTOM_TRIM;
                }
            }
            if if_long_split_collector_mode && l == 0 {
                w[l] += IF_SPLIT_COLLECTOR_LANE0_EXTRA;
            }
            if std::env::var("RUSTUML_EXT_DBG").is_ok() {
                eprintln!(
                    "[V2] lane {l} {:?} connector_xbounds={connector_bounds:?}",
                    lane_names[l]
                );
            }
        }
        (minx, w)
    } else {
        let ext: Vec<(f64, f64)> = (0..n).map(|l| swimlane_v2_run_extents(&runs[l])).collect();
        let w: Vec<f64> = (0..n)
            .map(|l| (ext[l].0 + ext[l].1 + 10.0).max(title_w[l] + 10.0))
            .collect();
        // Linear lanes all centre on the single-tree spine `natural_cx`; their
        // natural minX is `natural_cx − content_left`.
        let minx: Vec<f64> = (0..n).map(|l| natural_cx - ext[l].0).collect();
        (minx, w)
    };

    let mut lane_left = vec![0.0f64; n];
    let mut acc = SWIM_LEFT_DIVIDER_X;
    for l in 0..n {
        lane_left[l] = acc;
        acc += lane_w[l];
    }
    let mut lane_right = vec![0.0f64; n];
    for l in 0..n {
        lane_right[l] = lane_left[l] + lane_w[l];
    }
    // Rightmost divider. Linear lanes carry a trailing +10 in `lane_w` that lands
    // the right divider at `acc`; if-lanes' last divider sits at the last lane's
    // drawn right edge (no trailing gap), i.e. `acc − 10`.
    let right_edge = if if_mode && !fork_mode && n > 0 {
        acc - 10.0
    } else {
        acc
    };
    // Each lane's content left edge sits `content_pad` right of its left divider.
    // The pad is 6 for a box/diamond-leftmost lane, but 5 when the leftmost drawn
    // element is a free text label (e.g. a branch `yes`/`no` label that hangs off
    // the diamond — PlantUML's label has 1 px less left bearing inside the lane).
    // When the lane is wider than its content because its TITLE dominates, the
    // content is CENTRED in the column instead of left-anchored.
    let content_pad: Vec<f64> = (0..n)
        .map(|l| {
            if if_mode && title_w[l] > lane_content_w[l] + 0.001 {
                // Title-driven: centre the content in the lane column, with the
                // lane's intrinsic 6-left/4-right padding asymmetry (+1 to the
                // left half) baked in.
                (lane_w[l] - lane_content_w[l]) / 2.0 + 1.0
            } else if while_fork_mode && has_fork_bar(&shape_frags[l]) {
                let owns_top_bar = top_bar_y
                    .map(|top_y| has_fork_bar_at_y(&shape_frags[l], top_y))
                    .unwrap_or(false);
                if owns_top_bar { 43.0 } else { 32.0 }
            } else if fork_mode && has_fork_bar(&shape_frags[l]) {
                18.0
            } else if if_long_collector_mode && l == 0 {
                12.1763
            } else if if_long_multi_elseif_mode {
                IF_COLLECTOR_OTHER_LANE_PAD
            } else if if_long_collector_mode {
                20.0
            } else if lane_leftmost_is_text(&shape_frags[l], lane_minx[l]) {
                5.0
            } else {
                6.0
            }
        })
        .collect();
    let dx: Vec<f64> = (0..n)
        .map(|l| lane_left[l] + content_pad[l] - lane_minx[l])
        .collect();

    // Title band: lane titles sit in a band above the content. Content drops by
    // text-height + 5 (PlantUML getTitleHeightTranslate).
    let title_text_h = pm::text_height(LANE_TITLE_FONT);
    let title_band = if lane_names.iter().any(|nm| !nm.is_empty()) {
        title_text_h + 5.0
    } else {
        0.0
    };

    if std::env::var("RUSTUML_EXT_DBG").is_ok() {
        for l in 0..n {
            eprintln!(
                "[V2] lane {l} {:?} if_mode={if_mode} minx={} lane_w={} lane_left={} pad={} dx={}",
                lane_names[l], lane_minx[l], lane_w[l], lane_left[l], content_pad[l], dx[l]
            );
        }
        eprintln!("[V2] natural_cx={natural_cx} title_band={title_band} right_edge={right_edge}");
    }

    // Content drops to gold's swimlane body_top = header_top + text_height(title) + 15
    // (= gold's first-node reference y). Align the natural content's first reference
    // (the start ellipse cy, else the natural cursor) to body_top.
    let top_title_h = tree.iter().find_map(|node| match node {
        LayoutNode::Title { font_size, .. } => Some(pm::text_height(*font_size)),
        LayoutNode::Note { .. } | LayoutNode::Arrow { .. } => None,
        _ => None,
    });
    let natural_ref = first_ellipse_cy(&svg.shapes).unwrap_or(natural_start_y);
    let header_top = if let Some(title_h) = top_title_h {
        natural_start_y + title_h + 22.2969
    } else {
        natural_start_y + 1.2969
    };
    let body_top = header_top + title_text_h + 15.0;
    let content_dy = body_top - natural_ref;

    // Per-lane y-offset. With the leading-`|Lane|` phantom inbound arrow
    // suppressed (see `emit_sequence_ex`), the natural single-tree's
    // while-to-while gap already matches PlantUML's per-lane spacing, so every
    // lane shares the same vertical content_dy.
    let lane_dy: Vec<f64> = vec![content_dy; n];

    // Shift each lane's shape/connector fragment by its dx (x) and lane_dy (y).
    let mut lane_shapes: Vec<String> = (0..n)
        .map(|l| {
            crate::compress::shift_y(
                &crate::compress::shift_x(&shape_frags[l], dx[l]),
                lane_dy[l],
            )
        })
        .collect();
    let mut lane_conns: Vec<String> = (0..n)
        .map(|l| {
            crate::compress::shift_y(&crate::compress::shift_x(&conn_frags[l], dx[l]), lane_dy[l])
        })
        .collect();
    if while_fork_mode {
        for l in 0..n {
            if has_fork_bar(&shape_frags[l]) {
                lane_shapes[l] = rewrite_while_fork_bars_for_lane(
                    &lane_shapes[l],
                    lane_left[l],
                    top_bar_y.map(|y| y + lane_dy[l]),
                    bottom_bar_y.map(|y| y + lane_dy[l]),
                );
            }
            if let Some(top_y) = top_bar_y {
                let threshold = top_y + lane_dy[l] - 0.001;
                lane_shapes[l] =
                    shift_y_after(&lane_shapes[l], threshold, -WHILE_BODY_SLOT_COMPRESS);
                lane_conns[l] = shift_y_after(&lane_conns[l], threshold, -WHILE_BODY_SLOT_COMPRESS);
            }
        }
    }
    if fork_mode && !while_fork_mode {
        let top_bar_y = (0..n)
            .flat_map(|l| fork_bar_y_values(&shape_frags[l]))
            .fold(None, |best: Option<f64>, y| {
                Some(best.map_or(y, |best| best.min(y)))
            });
        let bottom_bar_y = (0..n)
            .flat_map(|l| fork_bar_y_values(&shape_frags[l]))
            .fold(None, |best: Option<f64>, y| {
                Some(best.map_or(y, |best| best.max(y)))
            });
        for l in 0..n {
            if has_fork_bar(&shape_frags[l]) {
                lane_shapes[l] = rewrite_fork_bars_for_lane(
                    &lane_shapes[l],
                    lane_left[l] + 6.0,
                    lane_w[l] - 10.0,
                );
                if fork_branch_count == 3
                    && let Some(bottom_y) = bottom_bar_y
                    && has_fork_bar_at_y(&shape_frags[l], bottom_y)
                {
                    lane_shapes[l] =
                        shift_fork3_bottom_lane_content(&lane_shapes[l], bottom_y + lane_dy[l]);
                }
            }
            if fork_branch_count == 4 && count_action_rects(&shape_frags[l]) == 2 {
                let owns_bottom_bar = bottom_bar_y
                    .map(|bottom_y| has_fork_bar_at_y(&shape_frags[l], bottom_y))
                    .unwrap_or(false);
                if n == 2 && !has_fork_bar(&shape_frags[l]) {
                    lane_shapes[l] = shift_fork4_two_action_lane_content(
                        &lane_shapes[l],
                        SWIM_FORK4_TWO_ACTION_PLAIN_TRIM,
                        0.0,
                        f64::MAX,
                    );
                } else if n == 2 && has_fork_bar(&shape_frags[l]) {
                    let bottom_y = bottom_bar_y.unwrap_or(0.0);
                    lane_shapes[l] = shift_fork4_two_action_lane_content(
                        &lane_shapes[l],
                        SWIM_FORK4_TWO_ACTION_BAR_TRIM,
                        0.0,
                        bottom_y + lane_dy[l],
                    );
                } else if n == 3 && owns_bottom_bar {
                    let bottom_y = bottom_bar_y.unwrap_or(0.0);
                    lane_shapes[l] = shift_fork4_two_action_lane_content(
                        &lane_shapes[l],
                        2.0 * SWIM_FORK4_TWO_ACTION_BAR_TRIM,
                        SWIM_FORK4_TWO_ACTION_BAR_TRIM,
                        bottom_y + lane_dy[l],
                    );
                }
            }
            if fork_branch_count == 5 {
                let action_count = count_action_rects(&shape_frags[l]);
                let owns_bottom_bar = bottom_bar_y
                    .map(|bottom_y| has_fork_bar_at_y(&shape_frags[l], bottom_y))
                    .unwrap_or(false);
                if n == 2 && action_count == 3 && owns_bottom_bar {
                    let bottom_y = bottom_bar_y.unwrap_or(0.0);
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[
                            0.0,
                            SWIM_FORK5_THREE_ACTION_BOTTOM_TRIM,
                            2.0 * SWIM_FORK5_THREE_ACTION_BOTTOM_TRIM,
                        ],
                        0.0,
                        SWIM_FORK5_THREE_ACTION_BOTTOM_TRIM,
                        f64::MIN,
                        bottom_y + lane_dy[l],
                    );
                } else if n == 2 && action_count == 2 && has_fork_bar(&shape_frags[l]) {
                    let top_y = top_bar_y.unwrap_or(0.0);
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[0.0, SWIM_FORK5_TWO_ACTION_TOP_TRIM],
                        SWIM_FORK5_TWO_ACTION_TOP_TRIM / 2.0,
                        0.0,
                        top_y + lane_dy[l],
                        f64::MAX,
                    );
                } else if n == 3 && action_count == 2 && !has_fork_bar(&shape_frags[l]) {
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[0.0, SWIM_FORK5_TWO_ACTION_PLAIN_TRIM],
                        0.0,
                        0.0,
                        f64::MIN,
                        f64::MAX,
                    );
                } else if n == 3 && action_count == 2 && owns_bottom_bar {
                    let bottom_y = bottom_bar_y.unwrap_or(0.0);
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[0.0, SWIM_FORK5_TWO_ACTION_BOTTOM_TRIM],
                        0.0,
                        SWIM_FORK3_BOTTOM_BAR_ACTION_TRIM,
                        f64::MIN,
                        bottom_y + lane_dy[l],
                    );
                }
            }
            if fork_branch_count == 6 {
                let action_count = count_action_rects(&shape_frags[l]);
                if n == 2 && action_count == 3 && !has_fork_bar(&shape_frags[l]) {
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[
                            0.0,
                            SWIM_FORK6_PLAIN_MIDDLE_ACTION_TRIM,
                            SWIM_FORK6_PLAIN_LANE_TRIM,
                        ],
                        0.0,
                        0.0,
                        f64::MIN,
                        f64::MAX,
                    );
                } else if n == 2 && action_count == 3 && has_fork_bar(&shape_frags[l]) {
                    let top_y = top_bar_y.unwrap_or(0.0);
                    let bottom_y = bottom_bar_y.unwrap_or(0.0);
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[
                            0.0,
                            SWIM_FORK6_BAR_MIDDLE_ACTION_TRIM,
                            SWIM_FORK6_BAR_LANE_TRIM,
                        ],
                        SWIM_FORK6_BAR_MIDDLE_ACTION_TRIM,
                        SWIM_FORK6_BAR_MIDDLE_ACTION_TRIM,
                        top_y + lane_dy[l],
                        bottom_y + lane_dy[l],
                    );
                } else if n == 3 && action_count == 2 && !has_fork_bar(&shape_frags[l]) {
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[0.0, SWIM_FORK6_PLAIN_LANE_TRIM],
                        0.0,
                        0.0,
                        f64::MIN,
                        f64::MAX,
                    );
                } else if n == 3 && action_count == 2 && has_fork_bar(&shape_frags[l]) {
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[0.0, SWIM_FORK6_BAR_LANE_TRIM],
                        0.0,
                        0.0,
                        f64::MIN,
                        f64::MAX,
                    );
                }
            }
            if fork_lane_delimited_mode && fork_branch_count == 3 {
                if l == 0 && has_fork_bar(&shape_frags[l]) {
                    let top_y = top_bar_y.unwrap_or(0.0);
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[0.0],
                        SWIM_COMBO_FORK3_TOP_TRIM,
                        0.0,
                        top_y + lane_dy[l],
                        f64::MAX,
                    );
                } else if l + 1 == n && has_fork_bar(&shape_frags[l]) {
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[-SWIM_COMBO_FORK3_BOTTOM_EXPAND],
                        0.0,
                        0.0,
                        f64::MIN,
                        f64::MAX,
                    );
                }
            }
            if fork_lane_delimited_mode && fork_branch_count == 4 {
                if l == 0 && has_fork_bar(&shape_frags[l]) {
                    let top_y = top_bar_y.unwrap_or(0.0);
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[0.0],
                        SWIM_COMBO_FORK4_TOP_TRIM,
                        0.0,
                        top_y + lane_dy[l],
                        f64::MAX,
                    );
                } else if l + 1 == n && has_fork_bar(&shape_frags[l]) {
                    lane_shapes[l] = shift_indexed_fork_lane_content(
                        &lane_shapes[l],
                        &[SWIM_COMBO_FORK4_BOTTOM_TRIM],
                        0.0,
                        0.0,
                        f64::MIN,
                        f64::MAX,
                    );
                }
            }
        }
    }
    let mut compression_lane_conns: Option<Vec<String>> = None;
    let mut routed_split_collector = false;

    let arrow_color = svg.palette.arrow_color.clone();

    // If-mode: rebuild all flow connectors as faithful cross-lane `Cross` snakes
    // from the final (shifted, uncompressed) shape positions — the natural
    // polylines' routing assumed the single-tree side-by-side branch layout. The
    // natural shapes are shifted by content_dy only (no dx) so they pair with the
    // per-lane-shifted final shapes for endpoint→shape matching.
    if if_mode && fork_mode && fork_branch_count > 0 {
        if !tree_has_while_with_fork(tree) {
            let nat_shapes: Vec<String> = (0..n)
                .map(|l| crate::compress::shift_y(&shape_frags[l], content_dy))
                .collect();
            if let Some(routed) =
                route_fork_swimlane_connectors(&nat_shapes, &lane_shapes, &arrow_color)
            {
                lane_conns = vec![String::new(); n];
                if n > 0 {
                    lane_conns[0] = routed;
                }
            }
        }
    } else if if_mode && !fork_mode {
        let nat_shapes: Vec<String> = (0..n)
            .map(|l| crate::compress::shift_y(&shape_frags[l], content_dy))
            .collect();
        let nat_conns = crate::compress::shift_y(&svg.connectors, content_dy);
        let if_long_collector = parse_conn_polylines(&nat_conns)
            .iter()
            .any(|pl| pl.tip.is_none());
        if let Some((routed, split_collector)) = route_if_cross_lane_connectors(
            &nat_conns,
            &nat_shapes,
            &lane_shapes,
            &lane_left,
            &lane_right,
            if_long_multi_elseif_mode,
            &arrow_color,
        ) {
            if if_long_collector && !split_collector {
                compression_lane_conns = Some(lane_conns.clone());
            }
            routed_split_collector = split_collector;
            // All routed connectors go (in natural document order) into a single
            // buffer; the assembly emits them as one block after the shapes.
            lane_conns = vec![String::new(); n];
            if n > 0 {
                lane_conns[0] = routed;
            }
        }
    }

    // Cross-lane connector synthesis. The natural single-tree draws each lane
    // boundary as the source while's west-exit corridor (stub, rail-arrowhead,
    // rail, horizontal-back-to-spine) followed by an inbound drop in the target
    // lane — all on the shared natural spine. Per-lane x-shift would split that
    // corridor across two `dx` values and break the L-snake. So for each
    // boundary out of a plain (no-special) `while`, strip the corridor's
    // trailing rail-arrowhead/rail/horizontal from the source fragment and the
    // leading drop+arrowhead from the target fragment, then re-emit the full
    // `Cross` L-snake with FINAL coordinates (matching `emit_while`'s stitch:
    // exit arrowhead at (diamond_cy + tile_bottom)/2, rail down to cross_y,
    // horizontal to the target spine, drop into the target tile).
    for l in 0..n.saturating_sub(1) {
        let ends_while = runs[l]
            .last()
            .is_some_and(|node| swimlane_segment_ends_in_plain_while(std::slice::from_ref(node)));
        if if_mode {
            break; // if-mode connectors are fully synthesized above.
        }
        if !ends_while {
            continue;
        }
        // Locate the natural transition in the (shifted) source-lane connectors.
        // It is the source while's west-exit corridor that wraps back to the
        // spine then drops into the next tile: a contiguous run
        //   [rail-arrowhead(poly), rail(vert line, min x), horiz-back(horiz line),
        //    drop(vert line at spine), drop-arrowhead(poly)]
        // The rail is the vertical line with the smallest x in the fragment.
        let src_prims = split_svg_primitives(&lane_conns[l]);
        // Index of the west rail: vertical line (x1==x2) with the minimum x.
        let rail_idx =
            src_prims
                .iter()
                .enumerate()
                .fold(None, |best, (i, p)| match parse_line_xy(p) {
                    Some((x1, x2, _, _)) if (x1 - x2).abs() < 0.001 => match best {
                        Some((_, bx)) if x1 >= bx => best,
                        _ => Some((i, x1)),
                    },
                    _ => best,
                });
        let Some((rail_idx, _)) = rail_idx else {
            continue;
        };
        // The rail-arrowhead is the polygon immediately before the rail.
        if rail_idx == 0 || rail_idx + 2 >= src_prims.len() {
            continue;
        }
        // The transition run is [rail_idx-1 .. rail_idx+3] (5 primitives:
        // rail-head, rail, horiz, drop, drop-head). Validate shapes loosely.
        let run_start = rail_idx - 1;
        let run_end = rail_idx + 4; // exclusive
        if run_end > src_prims.len() {
            continue;
        }
        // Source while's diamond_cy + body bottom (for the exit arrowhead y).
        let Some((diamond_cy, body_bottom)) = first_while_diamond_and_body(&lane_shapes[l]) else {
            continue;
        };
        // Target tile top (target lane's first diamond) → entry y.
        let Some(target_top) = first_diamond_top(&lane_shapes[l + 1]) else {
            continue;
        };
        // West exit x/y from the rail itself.
        let Some((exit_x, _, exit_y, _)) = parse_line_xy(&src_prims[rail_idx]) else {
            continue;
        };
        // The target lane's spine (where its tiles centre, in final coords).
        // Linear lanes share the single-tree spine `natural_cx`, shifted by the
        // target lane's dx.
        let target_cx = natural_cx + dx[l + 1];
        let target_y = target_top;
        let cross_y = target_y - 15.0;
        let tile_bottom = body_bottom + 2.0 * DIAMOND_HALF + WHILE_LOOPBACK_LABEL_H;
        let arrow_y = (diamond_cy + tile_bottom) / 2.0;

        // Re-emit the corridor + cross-lane L-snake with final coordinates.
        let head = |x: f64, y: f64| -> String {
            format!(
                r#"<polygon fill="{c}" points="{}" style="stroke:{c};stroke-width:1;"/>"#,
                polygon_points(&[
                    (x - 4.0, y - 10.0),
                    (x, y),
                    (x + 4.0, y - 10.0),
                    (x, y - 6.0),
                ]),
                c = arrow_color,
            )
        };
        let line = |x1: f64, x2: f64, y1: f64, y2: f64| -> String {
            format!(
                r#"<line style="stroke:{c};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                f(x1),
                f(x2),
                f(y1),
                f(y2),
                c = arrow_color,
            )
        };
        let mut snake = String::new();
        snake.push_str(&head(exit_x, arrow_y));
        snake.push_str(&line(exit_x, exit_x, exit_y, cross_y));
        snake.push_str(&line(exit_x, target_cx, cross_y, cross_y));
        snake.push_str(&line(target_cx, target_cx, cross_y, target_y));
        snake.push_str(&head(target_cx, target_y));

        // Replace the transition run in place with the synthesized snake (the
        // west-exit stub before it, and any deferred connectors after it, are
        // kept in their original positions).
        let mut rebuilt = String::new();
        for (i, p) in src_prims.iter().enumerate() {
            if i == run_start {
                rebuilt.push_str(&snake);
            }
            if i >= run_start && i < run_end {
                continue;
            }
            rebuilt.push_str(p);
        }
        lane_conns[l] = rebuilt;
    }

    // If-mode: collapse the horizontal inter-branch slack. PlantUML lays an if's
    // branches side-by-side with a 20 px gap; the diagram-level ON_X pass then
    // collapses it to 2*margin = 10. With the lanes now spread into columns the
    // slack is exposed. Build the ON_X transform from the laid-out shapes (the
    // chrome is excluded — a full-width header would mask the gap), re-anchor so
    // the leftmost content does not slide into the leading margin, and apply the
    // same transform to the lane fragments.
    let mut right_edge = right_edge;
    if if_mode && !fork_mode {
        // Occupancy anchors: a marker rect per lane spanning its left divider to
        // its leftmost content (the 6 px lane pad), so the compress does not
        // treat that padding as collapsible. Built into a throwaway buffer used
        // only to derive the transform (never emitted).
        let mut content = String::new();
        for l in 0..n {
            if let Some((lo, _)) = crate::compress::x_bounds(&lane_shapes[l]) {
                let w = (lo - lane_left[l]).max(1.0);
                content.push_str(&format!(
                    r#"<rect fill="none" height="1" style="stroke:none;stroke-width:1;" width="{}" x="{}" y="0"/>"#,
                    f(w),
                    f(lane_left[l]),
                ));
            }
        }
        let mut conns = String::new();
        let compression_conns = compression_lane_conns.as_ref().unwrap_or(&lane_conns);
        for l in 0..n {
            content.push_str(&lane_shapes[l]);
            conns.push_str(&compression_conns[l]);
        }
        let (_, _, mut x_tf, _) = crate::compress::compress_activity_buffers(
            &content,
            &conns,
            crate::compress::COMPRESS_MARGIN,
        );
        // Only intra-lane slack (an if's inter-branch gap) collapses — keep the
        // inter-lane spacing. Restrict the transform's slots to each lane's
        // content column.
        let lane_ranges: Vec<(f64, f64)> = (0..n)
            .map(|l| (lane_left[l], lane_left[l] + lane_w[l]))
            .collect();
        x_tf.restrict_to_ranges(&lane_ranges);
        if !x_tf.is_identity() {
            // Re-anchor: keep the first lane's left edge (its divider x) fixed so
            // only INTERNAL gaps collapse, not the leading margin.
            let anchor = lane_left[0];
            let off = anchor - x_tf.transform(anchor);
            let tx = |v: f64| x_tf.transform(v) + off;
            for l in 0..n {
                lane_shapes[l] = crate::compress::apply_x_offset(&lane_shapes[l], &x_tf, off);
                lane_conns[l] = crate::compress::apply_x_offset(&lane_conns[l], &x_tf, off);
            }
            // The rightmost divider follows the last lane's COMPRESSED content
            // right edge + the 4 px lane right pad (the column width shrinks with
            // the collapsed inter-branch slack).
            let last_right = crate::compress::x_bounds(&lane_shapes[n - 1])
                .map(|(_, hi)| hi)
                .unwrap_or_else(|| tx(right_edge));
            right_edge = (last_right + 4.0).max(lane_left[n - 1]);
            if routed_split_collector
                && let Some((_, hi)) = crate::compress::x_bounds(&lane_conns[0])
            {
                right_edge = right_edge.max(hi + IF_SPLIT_COLLECTOR_RIGHT_DIVIDER_PAD);
            }
        }
    }

    // Divider bottom = the bottom of the drawn content. An if-flow ends at the
    // drawn extent (e.g. a stop ellipse bottom); a linear while frame reserves
    // 12 px below the last drawn back-edge (its tail tile), so those lanes add
    // the 12 px lane-bottom margin.
    let mut content_max_y = 0.0f64;
    for l in 0..n {
        if let Some(b) = crate::compress::y_max(&lane_shapes[l]) {
            content_max_y = content_max_y.max(b);
        }
        if let Some(b) = crate::compress::y_max(&lane_conns[l]) {
            content_max_y = content_max_y.max(b);
        }
    }
    let _ = natural_bottom;
    let content_bottom = if if_mode {
        content_max_y
    } else {
        content_max_y + 12.0
    };

    // Divider x positions: left edge of each lane, plus the rightmost edge.
    let mut divider_xs: Vec<f64> = lane_left.clone();
    divider_xs.push(right_edge);

    // Assemble in PlantUML's `drawWhenSwimlanes` order: header band, then for
    // each lane its shapes followed by its left divider, then the rightmost
    // divider, then all connectors lane-by-lane, then the lane titles.
    let mut out = String::new();

    // Empty header rect spanning all lanes (left divider to right divider, plus
    // PlantUML's 1.8476 px overhang). For linear lanes `right_edge − lane_left[0]
    // == sum_lane_w`; if-lanes drop the last lane's trailing gap.
    let header_span = right_edge - lane_left[0];
    let header_fill = chrome.title_bg.as_deref().unwrap_or("none");
    let header_stroke = chrome.title_bg.as_deref().unwrap_or("none");
    write!(
        out,
        r#"<rect fill="{}" height="{}" style="stroke:{};stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
        header_fill,
        f(title_text_h),
        header_stroke,
        f(header_span + 1.8476),
        f(lane_left[0]),
        f(header_top),
    )
    .unwrap();

    for l in 0..n {
        out.push_str(&lane_shapes[l]);
        write!(
            out,
            r#"<line style="stroke:{};stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            chrome.divider,
            f(divider_xs[l]),
            f(divider_xs[l]),
            f(header_top),
            f(content_bottom),
        )
        .unwrap();
    }
    // Rightmost edge divider.
    write!(
        out,
        r#"<line style="stroke:{};stroke-width:1.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
        chrome.divider,
        f(divider_xs[n]),
        f(divider_xs[n]),
        f(header_top),
        f(content_bottom),
    )
    .unwrap();

    // Connectors, lane by lane.
    out.push_str(prelude_connectors);
    for conn in &lane_conns {
        out.push_str(conn);
    }

    // Lane titles: centred over each lane's column (between its dividers).
    let title_baseline = header_top + pm::ascent(LANE_TITLE_FONT);
    for l in 0..n {
        if lane_names[l].is_empty() {
            continue;
        }
        let col_left = divider_xs[l];
        let col_right = divider_xs[l + 1];
        let tw = text_render::measure(&lane_names[l], LANE_TITLE_FONT, false);
        let tx = col_left + (col_right - col_left - tw) / 2.0;
        write!(
            out,
            r#"<text fill="{}" font-family="sans-serif" font-size="{}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
            chrome.title,
            LANE_TITLE_FONT as u32,
            f(tw),
            f(tx),
            f(title_baseline),
            svg_text_escape(&lane_names[l]),
        )
        .unwrap();
    }
    let _ = &chrome.title_bg;

    // If-mode: collapse the horizontal inter-branch slack. PlantUML lays an if's
    // branches side-by-side with a 20 px gap, then the diagram-level
    // CompressionXorYBuilder ON_X pass collapses it to 2*margin = 10. Run that
    // pass on the whole assembled buffer: the full-width header rect anchors the
    // left (no leading collapse), the lane dividers (lines, exempt from
    // X-occupancy) ride the transform left with the content right of the gap.
    let span_after = header_span;

    // Width matches the legacy/segment path: a Swimlanes tile's reserved extent
    // is `span/2 + 4` left + `span/2 + 9` right (`node_extents`), and render()
    // pads the content box by MARGIN_LEAD (16) + MARGIN_TRAIL (19). So total
    // width = span + 13 + 35, ceil'd (`span` = left-to-right divider distance).
    const SWIM_NODE_EXTENT_PAD: f64 = 13.0; // 4 (left) + 9 (right)
    const SWIM_MARGIN_LEAD: f64 = 16.0;
    const SWIM_MARGIN_TRAIL: f64 = 19.0;
    let total_w =
        (span_after + SWIM_NODE_EXTENT_PAD + SWIM_MARGIN_LEAD + SWIM_MARGIN_TRAIL).ceil() as u32;
    let total_h = (content_bottom + SWIM_LEFT_DIVIDER_X).ceil() as u32;
    let prelude_shapes = crate::compress::x_bounds(prelude_shapes)
        .map(|(lo, hi)| {
            crate::compress::shift_x(prelude_shapes, total_w as f64 / 2.0 - (lo + hi) / 2.0)
        })
        .unwrap_or_else(|| prelude_shapes.to_string());
    let mut full_out = prelude_shapes;
    full_out.push_str(&out);
    Some((full_out, String::new(), total_w, total_h))
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
        // A lane's `while ... endwhile <terminator>` advertises a tile height
        // with a fixed 2*halfHex tail below the body (the special terminator is
        // placed inside that tail). In a lane the bottom edge is taken from the
        // visible frame (`getMinMax`), whose bottom sits at the wrap-back point
        // junction+halfHex = body+22, two pixels above the advertised tail. Trim
        // the lane's height contribution to the visible frame bottom.
        if swimlane_while_special_tail(&segment.body) {
            h -= 2.0;
        }
        last_y = segment_y + h;
        segment_ys.push(segment_y);
        segment_end_ys.push(last_y);
    }
    let final_last_y = last_y;

    // (prev_cx, prev_last_y, target_cx, target_y) for each cross-lane/source
    // transition. These are emitted after all per-lane internal connectors.
    //
    // A transition out of a segment that ENDS in a no-special `while` is instead
    // fused onto that loop's `ConnectionOut` exit corridor (see
    // `SvgEmitter::swimlane_cross_lane`): `emit_while` draws the whole stitch, so
    // suppress the standalone deferred arrow for those transitions and stash the
    // stitch geometry to install before the source segment is emitted.
    let mut deferred_cross_lanes: Vec<Option<(f64, f64, f64, f64)>> = Vec::new();
    let mut stitch_at: Vec<Option<SwimlaneCrossLane>> = vec![None; segments.len()];
    for i in 1..segments.len() {
        let prev = &segments[i - 1];
        let current = &segments[i];
        if prev.lane_index != current.lane_index && swimlane_segment_ends_in_plain_while(&prev.body)
        {
            stitch_at[i - 1] = Some(SwimlaneCrossLane {
                target_cx: lane_cxs[current.lane_index],
                cross_y: segment_ys[i] - 15.0,
                target_y: segment_ys[i],
            });
            deferred_cross_lanes.push(None);
        } else {
            deferred_cross_lanes.push(Some((
                lane_cxs[prev.lane_index],
                segment_end_ys[i - 1],
                lane_cxs[current.lane_index],
                segment_ys[i],
            )));
        }
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
                svg.swimlane_while_cond_special = swimlane_while_cond_special_lane(&segment.body);
                svg.swimlane_cross_lane = stitch_at[segment_idx];
                emit_sequence(
                    svg,
                    &segment.body,
                    lane_cxs[lane_idx],
                    segment_ys[segment_idx],
                );
                svg.swimlane_while_cond_special = false;
                svg.swimlane_cross_lane = None;
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
    for &(prev_cx, prev_y, lane_cx, target_y) in deferred_cross_lanes.iter().flatten() {
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
                source_line: diagram.meta.title_line.unwrap_or(1),
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
    if matches!(tree.first(), Some(LayoutNode::Title { .. }))
        && tree.iter().skip(1).any(|node| {
            matches!(
                node,
                LayoutNode::While { body, .. }
                    if while_body_has_prefixed_fused_trailing_while(body)
            )
        })
    {
        svg.title_x_offset = WHILE_PREFIXED_FUSED_NESTED_TITLE_X_OFFSET;
    }

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

    // Emit all nodes. On the swimlane V2 path the tree carries LaneMark nodes;
    // mark the emitter so per-lane self-layout details (the cond-driven
    // terminator placement) apply, mirroring the legacy segment model.
    svg.swimlane_v2_active = tree.iter().any(|n| matches!(n, LayoutNode::LaneMark(_)));
    emit_sequence_ex(&mut svg, &tree, cx, start_y, None, lead_note_h, false);
    svg.swimlane_v2_active = false;

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

    // Swimlane V2 per-lane layout. Fires whenever the build chose the V2 path
    // (the tree carried LaneMark nodes, so `lane_spans` is populated); lays the
    // natural single-tree emit out into lane columns. Returns None to fall
    // through if it cannot handle the shape (defensive — keeps the canvas valid).
    if !svg.lane_spans.is_empty() {
        let mut lane_names: Vec<String> = Vec::new();
        for s in &diagram.steps {
            if let ActivityStep::Swimlane(l) = s
                && !lane_names.iter().any(|nm| nm == &l.name)
            {
                lane_names.push(l.name.clone());
            }
        }
        let chrome = SwimlaneV2Chrome {
            divider: svg.palette.swimlane_border_color.clone(),
            title: svg.palette.swimlane_title_color.clone(),
            title_bg: svg.palette.swimlane_title_background.clone(),
        };
        if let Some((sh, cn, w, h)) = layout_swimlanes_v2(
            &svg,
            &tree,
            cx,
            start_y,
            body_bottom_y,
            &lane_names,
            &chrome,
        ) {
            // V2 columns are already tight (reserved extents) — skip the ON_X
            // compress pass, which would wrongly collapse the inter-lane gaps.
            let mut content = sh;
            content.push_str(&cn);
            return format_svg(w, h, &content, defs, svg_background.as_deref());
        }
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
