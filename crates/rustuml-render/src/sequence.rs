// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's Java implementation exactly —
//! same element structure, attributes, coordinates, and font metrics.

use std::collections::HashMap;
use std::fmt::Write;

use rustuml_parser::diagram::sequence::*;

use crate::creole::{self, CreoleLine};
use crate::handwritten::{
    JavaRandom as HandJavaRandom, ellipse_points as handwritten_ellipse_points,
    has_deprecated_skinparam as has_deprecated_handwritten_skinparam,
    is_enabled as is_handwritten_enabled, line_path as handwritten_line_path,
    path as handwritten_path, path_with_rnd as handwritten_path_with_rnd,
    polygon_points as handwritten_polygon_points, rect_points as handwritten_rect_points,
};
use crate::layout_oracle::{OracleHandwrittenWarning, OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics;
use crate::style::Theme;
use crate::text_render::{self, TextBase};

/// Resolve a PlantUML color string (e.g., "#blue", "#FF0000") to a CSS hex color.
pub(crate) fn resolve_color(color: &str) -> String {
    let name = color.strip_prefix('#').unwrap_or(color);
    // If it's already a hex color (starts with digit or uppercase hex)
    if name.len() == 6 && name.chars().all(|c| c.is_ascii_hexdigit()) {
        return format!("#{}", name.to_uppercase());
    }
    // 3-digit hex shorthand (#RGB → #RRGGBB), per PlantUML's HtmlColor parsing.
    if name.len() == 3 && name.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut out = String::with_capacity(7);
        out.push('#');
        for c in name.to_uppercase().chars() {
            out.push(c);
            out.push(c);
        }
        return out;
    }
    // Full CSS named colors (case-insensitive).
    match name.to_lowercase().as_str() {
        "aliceblue" => "#F0F8FF".to_string(),
        "antiquewhite" => "#FAEBD7".to_string(),
        "aqua" => "#00FFFF".to_string(),
        "aquamarine" => "#7FFFD4".to_string(),
        "azure" => "#F0FFFF".to_string(),
        "beige" => "#F5F5DC".to_string(),
        "bisque" => "#FFE4C4".to_string(),
        "black" => "#000000".to_string(),
        "blanchedalmond" => "#FFEBCD".to_string(),
        "blue" => "#0000FF".to_string(),
        "blueviolet" => "#8A2BE2".to_string(),
        "brown" => "#A52A2A".to_string(),
        "burlywood" => "#DEB887".to_string(),
        "cadetblue" => "#5F9EA0".to_string(),
        "chartreuse" => "#7FFF00".to_string(),
        "chocolate" => "#D2691E".to_string(),
        "coral" => "#FF7F50".to_string(),
        "cornflowerblue" => "#6495ED".to_string(),
        "cornsilk" => "#FFF8DC".to_string(),
        "crimson" => "#DC143C".to_string(),
        "cyan" => "#00FFFF".to_string(),
        "darkblue" => "#00008B".to_string(),
        "darkcyan" => "#008B8B".to_string(),
        "darkgoldenrod" => "#B8860B".to_string(),
        "darkgray" | "darkgrey" => "#A9A9A9".to_string(),
        "darkgreen" => "#006400".to_string(),
        "darkkhaki" => "#BDB76B".to_string(),
        "darkmagenta" => "#8B008B".to_string(),
        "darkolivegreen" => "#556B2F".to_string(),
        "darkorange" => "#FF8C00".to_string(),
        "darkorchid" => "#9932CC".to_string(),
        "darkred" => "#8B0000".to_string(),
        "darksalmon" => "#E9967A".to_string(),
        "darkseagreen" => "#8FBC8F".to_string(),
        "darkslateblue" => "#483D8B".to_string(),
        "darkslategray" | "darkslategrey" => "#2F4F4F".to_string(),
        "darkturquoise" => "#00CED1".to_string(),
        "darkviolet" => "#9400D3".to_string(),
        "deeppink" => "#FF1493".to_string(),
        "deepskyblue" => "#00BFFF".to_string(),
        "dimgray" | "dimgrey" => "#696969".to_string(),
        "dodgerblue" => "#1E90FF".to_string(),
        "firebrick" => "#B22222".to_string(),
        "floralwhite" => "#FFFAF0".to_string(),
        "forestgreen" => "#228B22".to_string(),
        "fuchsia" => "#FF00FF".to_string(),
        "gainsboro" => "#DCDCDC".to_string(),
        "ghostwhite" => "#F8F8FF".to_string(),
        "gold" => "#FFD700".to_string(),
        "goldenrod" => "#DAA520".to_string(),
        "gray" | "grey" => "#808080".to_string(),
        "green" => "#008000".to_string(),
        "greenyellow" => "#ADFF2F".to_string(),
        "honeydew" => "#F0FFF0".to_string(),
        "hotpink" => "#FF69B4".to_string(),
        "indianred" => "#CD5C5C".to_string(),
        "indigo" => "#4B0082".to_string(),
        "ivory" => "#FFFFF0".to_string(),
        "khaki" => "#F0E68C".to_string(),
        "lavender" => "#E6E6FA".to_string(),
        "lavenderblush" => "#FFF0F5".to_string(),
        "lawngreen" => "#7CFC00".to_string(),
        "lemonchiffon" => "#FFFACD".to_string(),
        "lightblue" => "#ADD8E6".to_string(),
        "lightcoral" => "#F08080".to_string(),
        "lightcyan" => "#E0FFFF".to_string(),
        "lightgoldenrodyellow" => "#FAFAD2".to_string(),
        "lightgray" | "lightgrey" => "#D3D3D3".to_string(),
        "lightgreen" => "#90EE90".to_string(),
        "lightpink" => "#FFB6C1".to_string(),
        "lightsalmon" => "#FFA07A".to_string(),
        "lightseagreen" => "#20B2AA".to_string(),
        "lightskyblue" => "#87CEFA".to_string(),
        "lightslategray" | "lightslategrey" => "#778899".to_string(),
        "lightsteelblue" => "#B0C4DE".to_string(),
        "lightyellow" => "#FFFFE0".to_string(),
        "lime" => "#00FF00".to_string(),
        "limegreen" => "#32CD32".to_string(),
        "linen" => "#FAF0E6".to_string(),
        "magenta" => "#FF00FF".to_string(),
        "maroon" => "#800000".to_string(),
        "mediumaquamarine" => "#66CDAA".to_string(),
        "mediumblue" => "#0000CD".to_string(),
        "mediumorchid" => "#BA55D3".to_string(),
        "mediumpurple" => "#9370DB".to_string(),
        "mediumseagreen" => "#3CB371".to_string(),
        "mediumslateblue" => "#7B68EE".to_string(),
        "mediumspringgreen" => "#00FA9A".to_string(),
        "mediumturquoise" => "#48D1CC".to_string(),
        "mediumvioletred" => "#C71585".to_string(),
        "midnightblue" => "#191970".to_string(),
        "mintcream" => "#F5FFFA".to_string(),
        "mistyrose" => "#FFE4E1".to_string(),
        "moccasin" => "#FFE4B5".to_string(),
        "navajowhite" => "#FFDEAD".to_string(),
        "navy" => "#000080".to_string(),
        "oldlace" => "#FDF5E6".to_string(),
        "olive" => "#808000".to_string(),
        "olivedrab" => "#6B8E23".to_string(),
        "orange" => "#FFA500".to_string(),
        "orangered" => "#FF4500".to_string(),
        "orchid" => "#DA70D6".to_string(),
        "palegoldenrod" => "#EEE8AA".to_string(),
        "palegreen" => "#98FB98".to_string(),
        "paleturquoise" => "#AFEEEE".to_string(),
        "palevioletred" => "#DB7093".to_string(),
        "papayawhip" => "#FFEFD5".to_string(),
        "peachpuff" => "#FFDAB9".to_string(),
        "peru" => "#CD853F".to_string(),
        "pink" => "#FFC0CB".to_string(),
        "plum" => "#DDA0DD".to_string(),
        "powderblue" => "#B0E0E6".to_string(),
        "purple" => "#800080".to_string(),
        "rebeccapurple" => "#663399".to_string(),
        "red" => "#FF0000".to_string(),
        "rosybrown" => "#BC8F8F".to_string(),
        "royalblue" => "#4169E1".to_string(),
        "saddlebrown" => "#8B4513".to_string(),
        "salmon" => "#FA8072".to_string(),
        "sandybrown" => "#F4A460".to_string(),
        "seagreen" => "#2E8B57".to_string(),
        "seashell" => "#FFF5EE".to_string(),
        "sienna" => "#A0522D".to_string(),
        "silver" => "#C0C0C0".to_string(),
        "skyblue" => "#87CEEB".to_string(),
        "slateblue" => "#6A5ACD".to_string(),
        "slategray" | "slategrey" => "#708090".to_string(),
        "snow" => "#FFFAFA".to_string(),
        "springgreen" => "#00FF7F".to_string(),
        "steelblue" => "#4682B4".to_string(),
        "tan" => "#D2B48C".to_string(),
        "teal" => "#008080".to_string(),
        "thistle" => "#D8BFD8".to_string(),
        "tomato" => "#FF6347".to_string(),
        "turquoise" => "#40E0D0".to_string(),
        "violet" => "#EE82EE".to_string(),
        "wheat" => "#F5DEB3".to_string(),
        "white" => "#FFFFFF".to_string(),
        "whitesmoke" => "#F5F5F5".to_string(),
        "yellow" => "#FFFF00".to_string(),
        "yellowgreen" => "#9ACD32".to_string(),
        _ => {
            // Try as-is if it looks like a hex value
            if name.chars().all(|c| c.is_ascii_hexdigit()) {
                format!("#{name}")
            } else {
                "#FFFFFF".to_string()
            }
        }
    }
}

/// Compute text width at a given font size, routing through the creole-aware
/// segment helper so layout measurements match what `text_render::emit_text`
/// will actually emit.
fn text_width(text: &str, font_size: f64) -> f64 {
    text_render::measure(text, font_size, false)
}

fn text_width_with_family(text: &str, font_size: f64, font_family: &str) -> f64 {
    text_render::measure_with_family(text, font_size, false, font_family)
}

/// Compute bold text width at a given font size, routing through the creole-aware
/// segment helper so layout measurements match what `text_render::emit_text`
/// will actually emit.
fn bold_text_width(text: &str, font_size: f64) -> f64 {
    text_render::measure(text, font_size, true)
}

fn bold_text_width_with_family(text: &str, font_size: f64, font_family: &str) -> f64 {
    text_render::measure_with_family(text, font_size, true, font_family)
}

// A divider label that begins with `=== ` is treated as a heading-style
// divider: the marker is consumed and each extra `=` raises the font size.
fn divider_label_and_font_size(text: &str, default_font_size: u32) -> (&str, u32, bool) {
    let trimmed = text.trim();
    let equals = trimmed.chars().take_while(|&ch| ch == '=').count();
    if equals >= 3 && trimmed.chars().nth(equals).is_some_and(char::is_whitespace) {
        (
            trimmed[equals..].trim_start(),
            default_font_size + (equals as u32 - 2),
            true,
        )
    } else {
        (trimmed, default_font_size, false)
    }
}

fn divider_label_box_width(
    label: &str,
    font_size: u32,
    font_family: &str,
    heading_style: bool,
) -> f64 {
    let tw = bold_text_width_with_family(label, font_size as f64, font_family);
    let marker_space = if heading_style {
        0.0
    } else {
        bold_text_width_with_family(" ", font_size as f64, font_family)
    };
    tw + 14.0 + marker_space
}

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

fn text_height_with_family(font_size: f64, font_family: &str) -> f64 {
    text_render::text_height_for_family(font_size, font_family)
}

fn ascent_with_family(font_size: f64, font_family: &str) -> f64 {
    text_render::ascent_for_family(font_size, font_family)
}

fn combined_footer_bottom_offset(font_family: &str) -> f64 {
    if font_family.eq_ignore_ascii_case("sans-serif") {
        8.4687
    } else {
        8.7778
    }
}

fn combined_caption_footer_baseline_gap(font_family: &str) -> f64 {
    if font_family.eq_ignore_ascii_case("sans-serif") {
        14.6211
    } else {
        14.3467
    }
}

/// Format an f64 as a PlantUML-compatible coordinate string.
///
/// PlantUML emits SVG coordinates via `String.format(Locale.US, "%.4f", x)`
/// followed by trailing-zero stripping (see PlantUML's SvgGraphics.format).
/// Java's `%.4f` uses HALF_UP rounding (e.g. `110.15625` → `"110.1563"`),
/// whereas Rust's `format!("{:.4}")` uses HALF_EVEN (banker's rounding,
/// → `"110.1562"`). For exact-string golden parity we round explicitly to
/// HALF_UP at the 4th decimal place before formatting.
fn fmt_coord(v: f64) -> String {
    // During a uniform-scale render (`skinparam dpi`/`scale`), defer to the
    // shared formatter, which emits full round-trippable precision so the final
    // scaling pass rounds once instead of double-rounding (see
    // `plantuml_metrics::fmt_coord`).
    if crate::plantuml_metrics::full_precision_active() {
        return crate::plantuml_metrics::fmt_coord(v);
    }
    // Integer fast-path preserves "25" rather than "25.0000" after trim.
    if v == v.floor() && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    // HALF_UP at 4 decimals: scale by 10000, add ±0.5, floor toward -inf.
    let scaled = v * 10000.0;
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        -((-scaled + 0.5).floor())
    };
    let s = format!("{:.4}", rounded / 10000.0);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

fn parse_svg_points(points: &str) -> Option<Vec<(f64, f64)>> {
    let nums: Vec<f64> = points
        .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if !nums.len().is_multiple_of(2) {
        return None;
    }
    Some(nums.chunks_exact(2).map(|p| (p[0], p[1])).collect())
}

/// Compute the *drawn* note box width based on text width and note shape.
///
/// PlantUML's `ComponentRoseNote.drawInternalU` draws the polygon at
/// `(int) getTextWidth` (an integer truncation) but the note box's outer
/// edges land on the area allocated for it, which equals the ceiling of the
/// preferred width. Empirically the drawn outer width is `ceil(text) + margin`.
fn note_content_width(max_text_w: f64, shape: NoteShape) -> f64 {
    if shape == NoteShape::Note && max_text_w == 0.0 {
        return 21.0;
    }
    note_content_width_raw(max_text_w, shape).ceil()
}

fn aligned_note_content_width(max_text_w: f64, shape: NoteShape, align: MessageAlign) -> f64 {
    let width = note_content_width(max_text_w, shape);
    if align == MessageAlign::Center && shape == NoteShape::Note {
        width + NOTE_FOLD_SIZE - 1.0
    } else {
        width
    }
}

/// Compute the *layout* (preferred) note width — the full-precision value Java
/// uses for spacing/canvas reservation. `ComponentRoseNote.getPreferredWidth`
/// returns `getTextWidth + 2*paddingX + deltaShadow` as a raw `double` (no
/// rounding); `NotesBoxes.ensureConstraints` and `NoteBox.getStartingX` consume
/// that raw value. Using the ceiled `note_content_width` for those purposes
/// loses the sub-pixel fraction and tips downstream `floor`/`ceil` boundaries,
/// shifting participants and the canvas right edge by 1px on many note cases.
///
/// The constants match `note_content_width`'s margins minus its `ceil`.
fn note_content_width_raw(max_text_w: f64, shape: NoteShape) -> f64 {
    match shape {
        NoteShape::Note => max_text_w + 20.0, // 6 pad + text + 4 pad + 10 fold
        NoteShape::Hexagonal => max_text_w + 23.0, // 10 indent + 2 pad + text + 1 pad + 10 indent
        NoteShape::Rectangular => max_text_w + 7.0, // 6 pad + text + 1 pad
    }
}

const NOTE_VISIBLE_RAW_MARGIN: f64 = 21.0;
const HNOTE_VISIBLE_RAW_MARGIN: f64 = 24.0;
const RNOTE_VISIBLE_RAW_MARGIN: f64 = 8.0;

fn single_note_visible_raw_width(
    max_text_w: f64,
    shape: NoteShape,
    note_global_padding: f64,
) -> f64 {
    let raw_margin = match shape {
        NoteShape::Note => NOTE_VISIBLE_RAW_MARGIN,
        NoteShape::Hexagonal => HNOTE_VISIBLE_RAW_MARGIN,
        NoteShape::Rectangular => RNOTE_VISIBLE_RAW_MARGIN,
    };
    max_text_w + raw_margin + 2.0 * note_global_padding
}

fn aligned_note_content_width_raw(max_text_w: f64, shape: NoteShape, align: MessageAlign) -> f64 {
    let width = note_content_width_raw(max_text_w, shape);
    if align == MessageAlign::Center && shape == NoteShape::Note {
        width + NOTE_FOLD_SIZE - 1.0
    } else {
        width
    }
}

// ---------------------------------------------------------------------------
// PlantUML layout constants (reverse-engineered from golden SVGs)
// ---------------------------------------------------------------------------

const HEAD_BOX_Y: f64 = 5.0;
const DEFAULT_PARTICIPANT_BORDER_THICKNESS: f64 = 0.5;
const HANDWRITTEN_WARNING_BAND_H: f64 = 21.6406;
const HEAD_BOX_H: f64 = 30.488281250; // exact Java double

// Create-message layout (reverse-engineered from golden SVGs).
// When `create X` precedes a message targeting X, PlantUML draws X's head box
// inline at the message instead of at the top, and the lifeline begins there.
/// Offset from the create message's arrow y up to the inline head box top.
const CREATE_BOX_TOP_OFFSET: f64 = 21.310575;
/// Offset from the create message's arrow y down to the created lifeline top.
const CREATE_LIFELINE_TOP_OFFSET: f64 = 9.4336;
/// Created queue heads draw their pill flush with the inline head base instead
/// of using the ordinary mixed-participant 5px head drop.
const CREATE_QUEUE_HEAD_OFFSET: f64 = 0.0;
/// Created queue lifelines start 2.5px higher than rectangular created heads.
const CREATE_QUEUE_LIFELINE_TOP_ADJUST: f64 = 2.5;
/// Created queue heads extend 5px less below the creating message arrow.
const CREATE_QUEUE_ADVANCE_ADJUST: f64 = 5.0;
/// Extra vertical advance added to the y cursor after a create message,
/// accounting for the inline head box that straddles the arrow. Reverse-engineered
/// from goldens (the next event sits this much further down than a normal step).
const CREATE_EXTRA_ADVANCE: f64 = 12.177753125;
/// Offset for an activation bar that begins on a create+activate message: the bar
/// starts below the inline head box rather than at the arrow.
const CREATE_BAR_OFFSET: f64 = 10.0;
/// Offset for a bare `activate` that precedes the first message: PlantUML draws
/// the bar starting one message step below the participant head.
const ACTIVATION_PRE_MESSAGE_OFFSET: f64 = 10.0;
const HEAD_BOX_RX: f64 = 2.5;
const BOX_TEXT_X_PAD: f64 = 7.0;
const BOX_TEXT_Y_OFFSET: f64 = 20.535156250; // exact Java double: baseline from box top
const PARTICIPANT_FONT_SIZE: f64 = 14.0;
const MSG_FONT_SIZE: f64 = 13.0;
/// Text height at message font size (ascent + descent from Java AWT LineMetrics).
const MSG_TEXT_HEIGHT: f64 = 15.310546875; // plantuml_metrics::text_height(13.0)
/// Base vertical step between messages (no label text).
const MSG_BASE_STEP: f64 = 14.0;
/// Base first-message offset from lifeline top (no label text).
const MSG_BASE_FIRST_OFFSET: f64 = 16.0;
const TAIL_GAP: f64 = 17.0; // gap from last msg y to tail box y
const SHADOW_LIVING_WIDTH_EXTRA: f64 = 3.0;
const SHADOW_VERTICAL_PAD: f64 = 3.0;
const SHADOW_NOTE_EXTRA: f64 = 1.5;
const SHADOW_CANVAS_RIGHT_PAD: f64 = 3.0;
const SHADOW_CANVAS_BOTTOM_PAD: f64 = 3.0;

/// Vertical space a `newpage` separator reserves in the page-1 flow. Java
/// `ComponentRoseNewpage.getPreferredHeight` returns 1; `prepareNewpage`
/// advances `freeY2` by exactly this, shifting the foot boxes and lifelines
/// of the (only-rendered) first page down by 1px.
const NEWPAGE_SEPARATOR_HEIGHT: f64 = 1.0;
/// The `newpage` separator rule sits this far above the foot-box top. Java
/// draws the rule at the page-1 `freeY2` (which, after the separator's own +1
/// advance, equals the foot-box top minus this gap) — a constant 10px across
/// all page-1 endings (message, group, divider, note).
const NEWPAGE_SEPARATOR_FOOT_GAP: f64 = 10.0;

/// Vertical band an unlabelled delay (`...`) reserves for its dotted `1,4`
/// lifeline gap. A labelled delay adds the label's text height on top.
const DELAY_BAND_HEIGHT: f64 = 28.0;
/// Font size of the delay (`...`) label text.
const DELAY_LABEL_FONT_SIZE: u32 = 11;
/// The delay band starts this far below the preceding message's arrow y.
const DELAY_BAND_TOP_PAD: f64 = 8.0;

/// Compute the vertical step for a message event.
/// Messages with label text get extra height for the text line.
fn msg_step(has_text: bool, text_height: f64) -> f64 {
    MSG_BASE_STEP + if has_text { text_height } else { 0.0 }
}

/// PlantUML `ComponentRoseReference` geometry for a `ref over … : text` box.
/// Header ("ref") is 13pt bold; body is 12pt. Faithful port: cornersize 10,
/// heightFooter 5, xMargin 2, header pad 30+15, marginX1/X2 4, marginY 4.
/// `header_w`/`header_h` are the integer corner-tab dims; `preferred_h` is the
/// real box height (rect height = `preferred_h - 5`).
struct RefBox {
    /// Integer corner-tab width `(int)getHeaderWidth` — for the tab path only.
    header_w: f64,
    header_h: f64,
    preferred_h: f64,
    /// `getPreferredWidth` — uses the *real* (untruncated) header width.
    pref_w: f64,
}

const REF_HEADER_FONT: f64 = 13.0;
const REF_BODY_FONT: f64 = 12.0;
const REF_CORNER: f64 = 10.0;
const REF_FOOTER: f64 = 5.0;
const REF_XMARGIN: f64 = 2.0;
/// Participant `outMargin` (ParticipantBox.java): living getMinX = box_x - this,
/// getMaxX = box_x + box_width + this.
const REF_OUT_MARGIN: f64 = 5.0;
/// Vertical gap from the preceding message arrow to the reference box top
/// (verified against the golden flow: box_top = prev_arrow_y + 8).
const REF_GAP_ABOVE: f64 = 8.0;
/// A reference that is the first event starts from the lifeline origin, not
/// from the ordinary first-message arrow slot.
const REF_FIRST_GAP: f64 = 10.0;
/// PlantUML leaves the first reference's visual top high, but still advances
/// the following flow two pixels lower than `preferred_h` alone.
const REF_FIRST_FLOW_EXTRA: f64 = 2.0;

fn ref_box(text: &str) -> RefBox {
    let ref_label_w = bold_text_width("ref", REF_HEADER_FONT);
    let header_w_real = ref_label_w + 45.0; // getHeaderWidth (= headerDim.w + 30 + 15)
    let header_w = header_w_real.floor(); // (int) cast, for the corner tab
    let header_h = (plantuml_metrics::text_height(REF_HEADER_FONT) + 2.0).floor();
    let n_lines = text.lines().count().max(1) as f64;
    let body_w = text
        .lines()
        .map(|l| text_width(l, REF_BODY_FONT))
        .fold(0.0_f64, f64::max);
    let text_h = n_lines * plantuml_metrics::text_height(REF_BODY_FONT) + 8.0;
    let preferred_h = text_h + (plantuml_metrics::text_height(REF_HEADER_FONT) + 2.0) + REF_FOOTER;
    // getTextWidth = body + marginX1(4) + marginX2(4); getPreferredWidth =
    // max(textWidth, headerWidth) + xMargin*2.
    let pref_w = (body_w + 8.0).max(header_w_real) + REF_XMARGIN * 2.0;
    RefBox {
        header_w,
        header_h,
        preferred_h,
        pref_w,
    }
}

/// Compute the first-message offset from lifeline top.
fn first_msg_offset(has_text: bool, text_height: f64) -> f64 {
    MSG_BASE_FIRST_OFFSET + if has_text { text_height } else { 0.0 }
}
const LIFELINE_Y_OFFSET: f64 = 1.0; // lifeline starts 1px below head box
const RIGHT_MARGIN: f64 = 10.0; // right margin beyond last box
const BOTTOM_MARGIN: f64 = 7.0; // bottom margin below tail box
const ARROW_SIZE: f64 = 10.0; // horizontal size of arrow polygon
const ARROW_HALF_H: f64 = 4.0; // vertical half-height of arrow polygon
const FILLED_ARROW_NOTCH: f64 = 4.0; // notch indent in filled arrow
const MSG_TEXT_LEFT_PAD: f64 = 7.0; // text offset from source lifeline
const LEFT_ARROW_TEXT_PAD: f64 = 16.0; // text offset from arrow tip (left arrows)
/// Java ComponentRoseArrow margins / deltas (AbstractTextualComponent super
/// args 7,7,1 and arrowDeltaX = 10). Used to position message labels when
/// `skinparam SequenceMessageAlign` is center or right.
const ARROW_MARGIN_X1: f64 = 7.0;
const ARROW_MARGIN_X2: f64 = 7.0;
const ARROW_DELTA_X: f64 = 10.0;

/// Message-label horizontal alignment selected by
/// `skinparam SequenceMessageAlign`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MessageAlign {
    Left,
    Center,
    Right,
}

/// Compute the message-label x given the left-aligned x that the renderer
/// already produces, the drawn arrow-line endpoints, the label width and the
/// arrow direction.
///
/// Ports `ComponentRoseArrow.drawInternalU` (lines ~165-179): the label is
/// drawn at `componentLeft + textPos`, where for LEFT `textPos = marginX1
/// (+ arrowDeltaX when the head is on the source/left side)`. The renderer's
/// `left_x` already equals `componentLeft + that LEFT textPos`, so we recover
/// `componentLeft` and apply the center/right formula. `width` is the arrow
/// component width = drawn line length + 6 (Java draws `len = width - 1` then
/// trims `arrowDeltaX/2` for the normal full head).
fn aligned_label_x(
    align: MessageAlign,
    left_x: f64,
    line_x1: f64,
    line_x2: f64,
    text_width: f64,
    is_right: bool,
    component_left_shift: f64,
) -> f64 {
    if align == MessageAlign::Left {
        return left_x;
    }
    let width = (line_x2 - line_x1).abs() + 6.0;
    // LEFT textPos: marginX1 for an LTR-normal arrow (head on right), or
    // marginX1 + arrowDeltaX for a reverse arrow (head on the source/left side).
    let left_text_pos = if is_right {
        ARROW_MARGIN_X1
    } else {
        ARROW_MARGIN_X1 + ARROW_DELTA_X
    };
    let component_left = left_x - left_text_pos - component_left_shift;
    let text_pos = match align {
        MessageAlign::Center => (width - text_width) / 2.0,
        MessageAlign::Right => {
            width - text_width - ARROW_MARGIN_X2 - if is_right { ARROW_DELTA_X } else { 0.0 }
        }
        MessageAlign::Left => unreachable!(),
    };
    component_left + text_pos
}

fn aligned_note_text_x(
    align: MessageAlign,
    left_x: f64,
    note_left: f64,
    note_right: f64,
    shape: NoteShape,
    text_width: f64,
) -> f64 {
    match align {
        MessageAlign::Left => left_x,
        MessageAlign::Center => (note_left + (note_right - note_left - text_width) / 2.0).round(),
        MessageAlign::Right => {
            let right_pad = match shape {
                NoteShape::Note => NOTE_FOLD_SIZE,
                NoteShape::Hexagonal => HNOTE_INDENT + 2.0,
                NoteShape::Rectangular => RNOTE_TEXT_X_PAD,
            };
            (note_right - right_pad - text_width).floor()
        }
    }
}
const ACTIVATION_WIDTH: f64 = 10.0;
const ACTIVATION_HALF_W: f64 = 5.0;
/// Gap between autonumber bold text and message label text.
const AUTONUMBER_LABEL_GAP: f64 = 4.0;
const LIFELINE_RECT_WIDTH: f64 = 8.0;
const MIN_LIFELINE_HEIGHT: f64 = 20.0;
// Arrow tip is 2px before the target lifeline center (non-activated)
const ARROW_TIP_GAP: f64 = 2.0;

// ---------------------------------------------------------------------------
// Self-message layout constants (reverse-engineered from golden SVGs)
// ---------------------------------------------------------------------------

/// Horizontal extension of the self-message loopback from center_x.
const SELF_MSG_EXTEND: f64 = 42.0;
/// Vertical drop of the self-message loopback.
const SELF_MSG_DROP: f64 = 13.0;
/// Text x offset from center_x for self-messages.
const SELF_MSG_TEXT_X_PAD: f64 = 7.0;
/// Extra right padding beyond self-message text/loopback.
const SELF_MSG_RIGHT_PAD: f64 = 2.0;
/// Bare `note right` attached to a self-message starts this far after
/// `floor(cx) + label_width`.
const SELF_MSG_RIGHT_NOTE_X_PAD: f64 = 19.0;
/// Bare `note right` attached to a self-message places the note top at
/// `arrow_y - first_line_height + this`.
const SELF_MSG_RIGHT_NOTE_Y_PAD: f64 = 2.5;
/// Extra canvas reservation PlantUML keeps for an active self-message on an
/// inline-created participant. The visible loopback geometry is unchanged, but
/// the computed right edge is wider in create+activate lifecycles.
const CREATED_ACTIVE_SELF_MSG_RIGHT_PAD: f64 = 16.0;
/// Narrower InGroupable right extent Java reports to enclosing group frames
/// for the same inline-created active self-message.
const CREATED_ACTIVE_GROUP_SELF_MSG_RIGHT_PAD: f64 = 10.0;
/// Extra canvas reservation for active self-messages on a participant whose
/// lifecycle is later ended by a standalone destroy.
const DESTROYED_ACTIVE_SELF_MSG_RIGHT_PAD: f64 = 15.0;
/// Minimum preferred width a self-message reserves in the gap to the next
/// participant. PlantUML's ComponentRoseSelfArrow.getPreferredWidth returns
/// `max(textWidth, arrowWidth + 5)` with arrowWidth = 45, i.e. min 50.
const SELF_MSG_MIN_PREF_WIDTH: f64 = 50.0;

// ---------------------------------------------------------------------------
// Participant-type shape constants (reverse-engineered from golden SVGs)
// ---------------------------------------------------------------------------

/// Actor stick-figure: head circle radius = 8, arm half-span = 13.
/// All offsets are relative to the region base_y (= HEAD_BOX_Y for head).
const ACTOR_HEAD_CY_OFFSET: f64 = 8.5; // ellipse cy relative to base_y
const ACTOR_SPINE_TOP_OFFSET: f64 = 16.5;
const ACTOR_SPINE_BOTTOM_OFFSET: f64 = 43.5;
const ACTOR_ARM_Y_OFFSET: f64 = 24.5;
const ACTOR_ARM_HALF: f64 = 13.0;
const ACTOR_LEG_BOTTOM_OFFSET: f64 = 58.5;
/// Extra height for actor beyond HEAD_BOX_H.
const ACTOR_EXTRA_H: f64 = 45.0;
/// Padding around actor text (3px each side).
const ACTOR_TEXT_PAD: f64 = 3.0;
/// Arm span of stick figure.
const ACTOR_ARM_SPAN: f64 = 26.0;
/// Actor head text: text baseline offset from base_y (figure first, then text).
const ACTOR_HEAD_TEXT_Y_OFFSET: f64 = 73.535156250;
/// Actor tail text: text baseline offset from base_y (text first, then figure).
const ACTOR_TAIL_TEXT_Y_OFFSET: f64 = 13.535156250;
/// Actor tail figure start offset (from base_y) — figure starts below text.
const ACTOR_TAIL_FIGURE_Y_OFFSET: f64 = 16.488281250;

/// Boundary/Control/Entity: circle radius = 12.
const STEREOTYPE_CIRCLE_R: f64 = 12.0;
/// Margin around the circle stickman (PlantUML Control/Entity/Boundary
/// `margin = 4`). The stickman's intrinsic width is `2*radius + 2*margin`.
const STEREOTYPE_CIRCLE_MARGIN: f64 = 4.0;
/// Circle center Y (from HEAD_BOX_Y).
const STEREOTYPE_CIRCLE_CY: f64 = 16.0; // 21 - 5 = 16 from HEAD_BOX_Y
/// Extra height for boundary/control/entity beyond HEAD_BOX_H.
const CIRCLE_SHAPE_EXTRA_H: f64 = 17.0;
/// Text baseline Y offset for circle-type shapes.
const CIRCLE_SHAPE_TEXT_Y_OFFSET: f64 = 45.535156250;

/// Boundary shape: vertical line extends from y_top to y_bottom, horizontal at center.
const BOUNDARY_LINE_TOP_OFFSET: f64 = 4.0; // relative to HEAD_BOX_Y
const BOUNDARY_LINE_BOTTOM_OFFSET: f64 = 28.0;
const BOUNDARY_LINE_TO_CIRCLE_GAP: f64 = 17.0; // horizontal gap from line to circle left

/// Database: cylinder dimensions.
const DB_CYLINDER_WIDTH: f64 = 36.0;
const DB_CYLINDER_HALF_W: f64 = 18.0;
const DB_CYLINDER_HEIGHT: f64 = 46.0; // body from top of shape to bottom
const DB_ELLIPSE_RY: f64 = 10.0; // top ellipse vertical radius (half of 20px visible curve)
/// Extra height for database beyond HEAD_BOX_H.
const DB_EXTRA_H: f64 = 31.0;
/// Text baseline Y offset for database shapes.
const DB_TEXT_Y_OFFSET: f64 = 59.535156250;

/// Collections: two stacked rectangles, no rounded corners.
/// The back rectangle is offset 4px right and 4px down from front.
const COLLECTIONS_OFFSET: f64 = 4.0;
/// Extra height for collections beyond HEAD_BOX_H.
const COLLECTIONS_EXTRA_H: f64 = 4.0;

// Queue: pill shape with same height as regular participant.

// ---------------------------------------------------------------------------
// Note layout constants (reverse-engineered from golden SVGs)
// ---------------------------------------------------------------------------

/// Size of the folded corner (both x and y).
const NOTE_FOLD_SIZE: f64 = 10.0;
/// Text left padding inside the note box.
const NOTE_TEXT_X_PAD: f64 = 6.0;
const RNOTE_TEXT_X_PAD: f64 = 4.0;
// Queue pill total horizontal padding (text + this = box width).
const QUEUE_TEXT_H_PAD: f64 = 20.0;
// Queue text inset from box left edge (cap radius).
const QUEUE_TEXT_X_PAD: f64 = 5.0;
/// Gap between previous event y and note top (non-first event).
const NOTE_GAP_AFTER_MSG: f64 = 13.0;
/// Gap between lifeline top and note top (first event).
const NOTE_GAP_FIRST: f64 = 15.0;
/// When several bare side notes attach to the same message arrow, PlantUML
/// keeps the note boxes in the ordinary first-line band but lowers the arrow
/// and label by this amount for each extra side note.
const MULTI_SIDE_NOTE_ARROW_Y_ADJUST: f64 = 3.0;
const NOTE_LIST_ITEM_TEXT_X: f64 = 12.0;
const NOTE_LIST_BULLET_CX: f64 = 5.5;
const NOTE_LIST_BULLET_BASELINE_DROP: f64 = 4.7578;
const NOTE_LIST_NUMBER_GAP: f64 = 4.1133;
const NOTE_IMAGE_TEXT_Y_SHIFT: f64 = 0.5596;
const NOTE_TABLE_CELL_PAD_X: f64 = 4.1133;
const NOTE_TABLE_HEADER_X_ADJUST: f64 = 0.1714;
const NOTE_TABLE_TOP_PAD: f64 = 7.0;
const NOTE_TABLE_BODY_EXTRA: f64 = 4.0;
const NOTE_RULE_TEXT_X: f64 = 8.5;
const NOTE_RULE_LEFT_PAD: f64 = 5.0;
const NOTE_RULE_SEGMENT_W: f64 = 13.5;
const NOTE_RULE_BASELINE_DROP: f64 = 4.4131;
const NOTE_RULE_WIDTH_EXTRA: f64 = 8.0;
const NOTE_HLINE_HEIGHT: f64 = 8.0;
const NOTE_HLINE_COMPACT_HEIGHT: f64 = 4.0;
const NOTE_HLINE_ASCENT: f64 = 4.4131;
const NOTE_HLINE_Y_DROP: f64 = 4.4131;
const NOTE_HLINE_AFTER_LIST_Y_EXTRA: f64 = 4.0;
const NOTE_HLINE_LEFT_PAD: f64 = 1.0;
const NOTE_HLINE_WIDTH_EXTRA: f64 = 19.0;
const PURE_UNDERLINE_MESSAGE_FLOW_EXTRA: f64 = MSG_TEXT_HEIGHT / 4.0;
/// PlantUML's text atoms reserve at least 10px height even when the font's real
/// line metrics are smaller (notably `defaultFontSize 8`).
fn atom_height_with_family(font_size: f64, font_family: &str) -> f64 {
    text_height_with_family(font_size, font_family).max(10.0)
}

fn descent_with_family(font_size: f64, font_family: &str) -> f64 {
    text_height_with_family(font_size, font_family) - ascent_with_family(font_size, font_family)
}

#[derive(Clone, Copy)]
struct RenderedLineMetrics {
    height: f64,
    ascent: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NoteLineKind {
    Normal,
    Code,
}

#[derive(Clone, Copy, Debug)]
struct NoteVisualLine<'a> {
    text: &'a str,
    kind: NoteLineKind,
}

fn note_visual_lines(text: &str) -> Vec<NoteVisualLine<'_>> {
    let mut lines = Vec::new();
    let mut in_code = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("<code>") {
            in_code = true;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("</code>") {
            in_code = false;
            continue;
        }
        lines.push(NoteVisualLine {
            text: line,
            kind: if in_code {
                NoteLineKind::Code
            } else {
                NoteLineKind::Normal
            },
        });
    }
    lines
}

fn is_single_hline(line: &str) -> bool {
    matches!(
        creole::parse_line(line.trim()),
        CreoleLine::HorizontalRule(creole::HorizontalRuleStyle::Single)
    )
}

fn is_note_list_line(line: &str) -> bool {
    matches!(
        creole::parse_line(line.trim()),
        CreoleLine::Bullet { .. } | CreoleLine::Numbered { .. }
    )
}

fn code_line_indent_and_body(line: &str) -> (usize, &str) {
    let indent = line
        .bytes()
        .take_while(|&b| b == b' ' || b == b'\t')
        .count();
    (indent, line.trim())
}

fn code_line_width(line: &str, font_size: f64) -> f64 {
    let (indent, body) = code_line_indent_and_body(line);
    plantuml_metrics::mono_text_width(&" ".repeat(indent), font_size)
        + plantuml_metrics::mono_text_width(body, font_size)
}

fn code_line_metrics(font_size: f64) -> RenderedLineMetrics {
    RenderedLineMetrics {
        height: atom_height_with_family(font_size, "monospace"),
        ascent: ascent_with_family(font_size, "monospace"),
    }
}

fn emit_code_note_line(buf: &mut String, line: &str, base: &TextBase<'_>) -> f64 {
    let font_size = base.font_size as f64;
    let (indent, body) = code_line_indent_and_body(line);
    let lead_w = plantuml_metrics::mono_text_width(&" ".repeat(indent), font_size);
    let text_w = plantuml_metrics::mono_text_width(body, font_size);
    if !body.is_empty() {
        let content = creole::escape_creole_text(body).replace(' ', "&#160;");
        write!(
            buf,
            r#"<text fill="{fill}" font-family="monospace" font-size="{font_size}" lengthAdjust="spacing" textLength="{text_len}" x="{x}" y="{y}">{content}</text>"#,
            fill = base.fill,
            font_size = base.font_size,
            text_len = fmt_coord(text_w),
            x = fmt_coord(base.x + lead_w),
            y = fmt_coord(base.y),
        )
        .unwrap();
    }
    lead_w + text_w
}

fn note_separator_label(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("__")?.strip_suffix("__")?;
    if inner.is_empty() { None } else { Some(inner) }
}

fn segment_font_size(seg: &creole::Segment, font_size: f64) -> f64 {
    seg.style.size.map_or(font_size, |s| s as f64)
}

fn segment_font_family<'a>(seg: &'a creole::Segment, font_family: &'a str) -> &'a str {
    if seg.style.monospace {
        "monospace"
    } else {
        seg.style.font_family.as_deref().unwrap_or(font_family)
    }
}

fn line_bottom_drop_with_family(font_size: f64, font_family: &str) -> f64 {
    atom_height_with_family(font_size, font_family) - ascent_with_family(font_size, font_family)
}

fn mixed_super_line_metrics_with_family(
    content: &str,
    font_size: f64,
    font_family: &str,
) -> Option<RenderedLineMetrics> {
    let segments = creole::parse_segments(content);
    let mut has_super = false;
    let mut has_base = false;
    let mut has_sub = false;
    for seg in segments.iter().filter(|seg| !seg.text.trim().is_empty()) {
        match seg.style.baseline_shift {
            Some("super") => has_super = true,
            Some("sub") => has_sub = true,
            _ => has_base = true,
        }
    }
    if !has_super || !has_base || has_sub {
        return None;
    }

    let first = segments.first()?;
    let line_bottom_drop = line_bottom_drop_with_family(
        segment_font_size(first, font_size),
        segment_font_family(first, font_family),
    );
    let mut ascent: f64 = 0.0;
    let mut descent: f64 = 0.0;
    for seg in segments.iter().filter(|seg| !seg.text.is_empty()) {
        let nominal_size = segment_font_size(seg, font_size);
        let family = segment_font_family(seg, font_family);
        let own_drop = line_bottom_drop_with_family(nominal_size, family);
        let line_descent_diff = line_bottom_drop - own_drop;
        let (emitted_size, y_offset) = match seg.style.baseline_shift {
            Some("super") => {
                let small = (nominal_size as i32 - 3).max(2) as f64;
                let descent_diff =
                    descent_with_family(nominal_size, family) - descent_with_family(small, family);
                (small, -6.0 + descent_diff + line_descent_diff)
            }
            _ => (nominal_size, line_descent_diff),
        };
        ascent = ascent.max(ascent_with_family(emitted_size, family) - y_offset);
        descent = descent.max(descent_with_family(emitted_size, family) + y_offset);
    }
    Some(RenderedLineMetrics {
        height: ascent + descent,
        ascent,
    })
}

fn pure_underline_message_flow_extra(text: &str) -> f64 {
    text.split("\\n")
        .filter(|line| note_separator_label(line).is_some())
        .count() as f64
        * PURE_UNDERLINE_MESSAGE_FLOW_EXTRA
}

fn event_pure_underline_flow_extra(event: &Event) -> f64 {
    match event {
        Event::Message(msg) => pure_underline_message_flow_extra(&process_label(&msg.label)),
        Event::Return(ret) if !ret.label.is_empty() => {
            pure_underline_message_flow_extra(&decode_backslash_escapes(&ret.label))
        }
        _ => 0.0,
    }
}

fn rendered_line_metrics_with_family(
    content: &str,
    font_size: f64,
    font_family: &str,
) -> RenderedLineMetrics {
    if let Some(metrics) = mixed_super_line_metrics_with_family(content, font_size, font_family) {
        return metrics;
    }
    RenderedLineMetrics {
        height: text_render::label_height_with_family(content, font_size, font_family).max(10.0),
        ascent: text_render::label_ascent_with_family(content, font_size, font_family),
    }
}

fn note_line_metrics_with_family(
    content: &str,
    font_size: f64,
    font_family: &str,
) -> RenderedLineMetrics {
    const SEPARATOR_HEIGHT_EXTRA: f64 = 7.6553;
    const SEPARATOR_ASCENT_ADJUST: f64 = -0.5;

    if is_single_hline(content) {
        return RenderedLineMetrics {
            height: NOTE_HLINE_HEIGHT,
            ascent: NOTE_HLINE_ASCENT,
        };
    }

    let styled_label = note_separator_label(content).or_else(|| whole_strike_label_inner(content));
    let label = styled_label.unwrap_or(content);
    let mut metrics = rendered_line_metrics_with_family(label, font_size, font_family);
    if styled_label.is_some() {
        metrics.height += SEPARATOR_HEIGHT_EXTRA;
        metrics.ascent += SEPARATOR_ASCENT_ADJUST;
    }
    metrics
}

fn note_visual_line_metrics_with_family(
    line: NoteVisualLine<'_>,
    font_size: f64,
    font_family: &str,
) -> RenderedLineMetrics {
    match line.kind {
        NoteLineKind::Normal => {
            note_line_metrics_with_family(line.text.trim(), font_size, font_family)
        }
        NoteLineKind::Code => code_line_metrics(font_size),
    }
}

fn first_segment_metrics_with_family(
    content: &str,
    font_size: f64,
    font_family: &str,
) -> RenderedLineMetrics {
    let segments = creole::parse_segments(content);
    let Some(first) = segments.first() else {
        return rendered_line_metrics_with_family(content, font_size, font_family);
    };
    let size = first.style.size.map_or(font_size, |s| s as f64);
    let family = if first.style.monospace {
        "monospace"
    } else if let Some(family) = first.style.font_family.as_deref() {
        family
    } else {
        font_family
    };
    RenderedLineMetrics {
        height: text_height_with_family(size, family).max(10.0),
        ascent: ascent_with_family(size, family),
    }
}

fn line_has_subscript_after_plain_first(content: &str) -> bool {
    let segments = creole::parse_segments(content);
    let Some(first) = segments.first() else {
        return false;
    };
    first.style.baseline_shift.is_none()
        && segments
            .iter()
            .skip(1)
            .any(|s| matches!(s.style.baseline_shift, Some("sub")))
}

fn all_shifted_line_metrics_with_family(
    content: &str,
    font_size: f64,
    font_family: &str,
) -> Option<(f64, f64)> {
    let segments = creole::parse_segments(content);
    let first_shift = segments.first()?.style.baseline_shift?;
    if !segments
        .iter()
        .all(|s| s.text.trim().is_empty() || s.style.baseline_shift == Some(first_shift))
    {
        return None;
    }

    let effective_size = (font_size - 3.0).max(2.0);
    let height = atom_height_with_family(effective_size, font_family);
    let baseline_drop = height - ascent_with_family(effective_size, font_family) + 2.0;
    let y_shift = plantuml_metrics::descent(font_size) - plantuml_metrics::descent(effective_size);
    let emitter_offset = match first_shift {
        "sub" => 3.0 + y_shift,
        "super" => -6.0 + y_shift,
        _ => return None,
    };
    Some((height, baseline_drop + emitter_offset))
}

fn rendered_label_y_drop_with_family(content: &str, font_size: f64, font_family: &str) -> f64 {
    if let Some(latex) = latex_label_content(content) {
        return crate::math::raw_latex_image(latex).height as f64 + 1.0;
    }
    let mut lines = content.split("\\n");
    let first = lines.next().unwrap_or("");
    if let Some((_, first_drop)) =
        all_shifted_line_metrics_with_family(first, font_size, font_family)
    {
        let remaining_height: f64 = lines
            .map(|line| rendered_line_metrics_with_family(line, font_size, font_family).height)
            .sum();
        return first_drop + remaining_height;
    }
    let first_metrics = first_segment_metrics_with_family(first, font_size, font_family);
    let remaining_height: f64 = lines
        .map(|line| rendered_line_metrics_with_family(line, font_size, font_family).height)
        .sum();
    let subscript_drop = if line_has_subscript_after_plain_first(first) {
        3.0
    } else {
        0.0
    };
    first_metrics.height - first_metrics.ascent + 2.0 + subscript_drop + remaining_height
}

fn message_label_width_with_family(
    text: &str,
    font_size: f64,
    bold: bool,
    font_family: &str,
) -> f64 {
    text.split("\\n")
        .map(|line| {
            if let Some(latex) = latex_label_content(line) {
                crate::math::raw_latex_image(latex).width as f64 + MSG_TEXT_LEFT_PAD
            } else if let Some(inner) = leading_star_bullet_italic_mono_inner(line) {
                let mono_italic = format!("//\"\"{inner}\"\"//");
                MESSAGE_STAR_BULLET_TEXT_OFFSET
                    + text_render::measure_with_family(&mono_italic, font_size, bold, font_family)
                    + text_render::measure_with_family("//**//", font_size, bold, font_family)
                    + text_render::measure_with_family("**", font_size, bold, font_family)
            } else {
                text_render::measure_with_family(line, font_size, bold, font_family)
            }
        })
        .fold(0.0, f64::max)
}

fn message_label_block_height_with_family(text: &str, font_size: f64, font_family: &str) -> f64 {
    text.split("\\n")
        .map(|line| {
            if let Some(latex) = latex_label_content(line) {
                crate::math::raw_latex_image(latex).height as f64 - 1.0
            } else if let Some((height, _)) =
                all_shifted_line_metrics_with_family(line, font_size, font_family)
            {
                height
            } else if let Some(metrics) =
                mixed_super_line_metrics_with_family(line, font_size, font_family)
            {
                metrics.height
            } else {
                rendered_line_metrics_with_family(line, font_size, font_family).height
                    + pure_underline_message_flow_extra(line)
            }
        })
        .sum()
}

fn latex_label_content(s: &str) -> Option<&str> {
    let trimmed = s.trim();
    trimmed
        .strip_prefix("<latex>")
        .and_then(|rest| rest.strip_suffix("</latex>"))
}

struct NoteTextMetrics {
    line_heights: Vec<f64>,
    first_height: f64,
    total_height: f64,
    body_height_extra: f64,
}

struct NoteTableLayout {
    rows: Vec<creole::TableRow>,
    col_widths: Vec<f64>,
    row_heights: Vec<f64>,
    row_ascents: Vec<f64>,
    grid_width: f64,
}

fn note_table_layout_with_family(
    text: &str,
    font_size: f64,
    font_family: &str,
) -> Option<NoteTableLayout> {
    let mut rows = Vec::new();
    for line in text.lines() {
        match creole::parse_line(line.trim()) {
            CreoleLine::Table(row) => rows.push(row),
            _ => return None,
        }
    }
    if rows.is_empty() {
        return None;
    }

    let cols = rows.iter().map(|row| row.cells.len()).max().unwrap_or(0);
    let mut col_widths = vec![0.0_f64; cols];
    let mut row_heights = Vec::with_capacity(rows.len());
    let mut row_ascents = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut row_height = atom_height_with_family(font_size, font_family);
        let mut row_ascent = ascent_with_family(font_size, font_family);
        for (idx, cell) in row.cells.iter().enumerate() {
            let text_w = text_render::measure_with_family(
                &cell.text,
                font_size,
                cell.is_header,
                font_family,
            );
            col_widths[idx] = col_widths[idx].max(text_w + NOTE_TABLE_CELL_PAD_X * 2.0);
            row_height = row_height.max(text_render::label_height_with_family(
                &cell.text,
                font_size,
                font_family,
            ));
            row_ascent = row_ascent.max(text_render::label_ascent_with_family(
                &cell.text,
                font_size,
                font_family,
            ));
        }
        row_heights.push(row_height);
        row_ascents.push(row_ascent);
    }
    let grid_width = col_widths.iter().sum();
    Some(NoteTableLayout {
        rows,
        col_widths,
        row_heights,
        row_ascents,
        grid_width,
    })
}

fn note_text_metrics_with_family(text: &str, font_size: f64, font_family: &str) -> NoteTextMetrics {
    if let Some(table) = note_table_layout_with_family(text, font_size, font_family) {
        let first_height = table.row_heights.first().copied().unwrap_or(0.0);
        let text_height: f64 = table.row_heights.iter().sum();
        return NoteTextMetrics {
            line_heights: table.row_heights,
            first_height,
            total_height: text_height + NOTE_TABLE_BODY_EXTRA,
            body_height_extra: NOTE_TABLE_BODY_EXTRA,
        };
    }

    let mut text_seen_before_list = false;
    let mut list_after_text = false;
    let mut smaller_size_after_first = false;
    let mut compact_hline_before_list_count = 0_usize;
    let visual_lines = note_visual_lines(text);
    let line_heights = visual_lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            if line.kind == NoteLineKind::Normal {
                let trimmed = line.text.trim();
                match creole::parse_line(trimmed) {
                    CreoleLine::Text(_) if !trimmed.is_empty() => text_seen_before_list = true,
                    CreoleLine::Bullet { .. } | CreoleLine::Numbered { .. } => {
                        list_after_text |= text_seen_before_list;
                    }
                    _ => {}
                }
                if is_single_hline(trimmed)
                    && visual_lines
                        .iter()
                        .skip(idx + 1)
                        .find(|next| !next.text.trim().is_empty())
                        .is_some_and(|next| is_note_list_line(next.text))
                {
                    // PlantUML compacts a divider directly before a list in the
                    // rendered line flow, then reserves the missing height for
                    // the note body and subsequent event spacing below.
                    compact_hline_before_list_count += 1;
                    return NOTE_HLINE_COMPACT_HEIGHT;
                }
                if idx > 0 {
                    smaller_size_after_first |= creole::parse_segments(trimmed)
                        .iter()
                        .any(|seg| seg.style.size.is_some_and(|size| (size as f64) < font_size));
                }
            }
            note_visual_line_metrics_with_family(*line, font_size, font_family).height
        })
        .collect::<Vec<_>>();
    if line_heights.is_empty() {
        return NoteTextMetrics {
            line_heights,
            first_height: 0.0,
            total_height: 0.0,
            body_height_extra: 0.0,
        };
    }
    let first_height = line_heights[0];
    let total_height = line_heights.iter().sum::<f64>()
        + compact_hline_before_list_count as f64 * NOTE_HLINE_AFTER_LIST_Y_EXTRA;
    // PlantUML's folded-note body extends one pixel below the text-flow
    // reservation for these multiline rich-text cases. Event spacing keeps
    // using `total_height`; only the drawn note body gets this correction.
    let body_height_extra = if list_after_text || smaller_size_after_first {
        1.0
    } else {
        0.0
    } + compact_hline_before_list_count as f64
        * (NOTE_HLINE_HEIGHT - NOTE_HLINE_COMPACT_HEIGHT);
    NoteTextMetrics {
        line_heights,
        first_height,
        total_height,
        body_height_extra,
    }
}

fn note_line_width_with_family(
    line: &str,
    font_size: f64,
    font_family: &str,
    number_counters: &mut Vec<usize>,
) -> f64 {
    match creole::parse_line(line.trim()) {
        CreoleLine::Bullet { level, content } => {
            number_counters.clear();
            let indent = NOTE_LIST_ITEM_TEXT_X * level.saturating_sub(1) as f64;
            indent
                + NOTE_LIST_ITEM_TEXT_X
                + text_width_with_family(&content, font_size, font_family)
        }
        CreoleLine::Numbered { level, content } => {
            let number = next_note_number(level, number_counters);
            let indent = NOTE_LIST_ITEM_TEXT_X * level.saturating_sub(1) as f64;
            let marker = format!("{number}.");
            indent
                + text_width_with_family(&marker, font_size, font_family)
                + NOTE_LIST_NUMBER_GAP
                + text_width_with_family(&content, font_size, font_family)
        }
        _ if let Some(label) = note_separator_label(line.trim()) => {
            number_counters.clear();
            text_width_with_family(label, font_size, font_family) + NOTE_RULE_WIDTH_EXTRA
        }
        _ if let Some(label) = whole_strike_label_inner(line.trim()) => {
            number_counters.clear();
            text_width_with_family(label, font_size, font_family) + NOTE_RULE_WIDTH_EXTRA
        }
        _ => {
            number_counters.clear();
            text_width_with_family(line.trim(), font_size, font_family)
        }
    }
}

fn note_visual_line_width_with_family(
    line: NoteVisualLine<'_>,
    font_size: f64,
    font_family: &str,
    number_counters: &mut Vec<usize>,
) -> f64 {
    match line.kind {
        NoteLineKind::Normal => {
            note_line_width_with_family(line.text.trim(), font_size, font_family, number_counters)
        }
        NoteLineKind::Code => {
            number_counters.clear();
            code_line_width(line.text, font_size)
        }
    }
}

fn note_max_line_width_with_family(text: &str, font_size: f64, font_family: &str) -> f64 {
    if let Some(table) = note_table_layout_with_family(text, font_size, font_family) {
        return table.grid_width;
    }

    let mut number_counters = Vec::new();
    note_visual_lines(text)
        .into_iter()
        .map(|line| {
            note_visual_line_width_with_family(line, font_size, font_family, &mut number_counters)
        })
        .fold(0.0_f64, f64::max)
}

fn next_note_number(level: usize, number_counters: &mut Vec<usize>) -> usize {
    if number_counters.len() > level {
        number_counters.truncate(level);
    }
    while number_counters.len() < level {
        number_counters.push(0);
    }
    number_counters[level - 1] += 1;
    number_counters[level - 1]
}

fn emit_note_line(
    buf: &mut String,
    line: &str,
    base: &TextBase<'_>,
    rule_stroke: &str,
    number_counters: &mut Vec<usize>,
) -> f64 {
    match creole::parse_line(line.trim()) {
        CreoleLine::Bullet { level, content } => {
            number_counters.clear();
            let indent = NOTE_LIST_ITEM_TEXT_X * level.saturating_sub(1) as f64;
            let cx = base.x + indent + NOTE_LIST_BULLET_CX;
            let cy = base.y - NOTE_LIST_BULLET_BASELINE_DROP;
            write!(
                buf,
                r##"<ellipse cx="{}" cy="{}" fill="{}" rx="2.5" ry="2.5"/>"##,
                fmt_coord(cx),
                fmt_coord(cy),
                base.fill
            )
            .unwrap();
            let text_x = base.x + indent + NOTE_LIST_ITEM_TEXT_X;
            let w = text_render::emit_text(
                buf,
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
            indent + NOTE_LIST_ITEM_TEXT_X + w
        }
        CreoleLine::Numbered { level, content } => {
            let number = next_note_number(level, number_counters);
            let indent = NOTE_LIST_ITEM_TEXT_X * level.saturating_sub(1) as f64;
            let marker = format!("{number}.");
            let marker_w = text_render::emit_text(
                buf,
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
                buf,
                &content,
                &TextBase {
                    x: base.x + indent + marker_w + NOTE_LIST_NUMBER_GAP,
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
            indent + marker_w + NOTE_LIST_NUMBER_GAP + w
        }
        _ if let Some(label) = note_separator_label(line.trim()) => {
            number_counters.clear();

            let text_x = base.x + NOTE_RULE_TEXT_X;
            let line_y = base.y - NOTE_RULE_BASELINE_DROP;
            write!(
                buf,
                r#"<line style="stroke:{};stroke-width:0.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                rule_stroke,
                fmt_coord(base.x - NOTE_RULE_LEFT_PAD),
                fmt_coord(text_x),
                fmt_coord(line_y),
                fmt_coord(line_y)
            )
            .unwrap();
            let text_w = text_render::emit_text(
                buf,
                label,
                &TextBase {
                    x: text_x,
                    y: base.y,
                    font_size: base.font_size,
                    font_family: base.font_family,
                    fill: base.fill,
                    bold: base.bold,
                    italic: base.italic,
                    underline: false,
                    skip_underline: true,
                },
            );
            let right_x = text_x + text_w;
            write!(
                buf,
                r#"<line style="stroke:{};stroke-width:0.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                rule_stroke,
                fmt_coord(right_x),
                fmt_coord(right_x + NOTE_RULE_SEGMENT_W),
                fmt_coord(line_y),
                fmt_coord(line_y)
            )
            .unwrap();
            text_w + NOTE_RULE_WIDTH_EXTRA
        }
        _ if let Some(label) = whole_strike_label_inner(line.trim()) => {
            number_counters.clear();
            let text_x = base.x + NOTE_RULE_TEXT_X;
            let line_y = base.y - NOTE_RULE_BASELINE_DROP;
            write!(
                buf,
                r#"<line style="stroke:{};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                rule_stroke,
                fmt_coord(base.x - NOTE_RULE_LEFT_PAD),
                fmt_coord(text_x),
                fmt_coord(line_y),
                fmt_coord(line_y)
            )
            .unwrap();
            let text_w = text_render::emit_text(
                buf,
                label,
                &TextBase {
                    x: text_x,
                    y: base.y,
                    font_size: base.font_size,
                    font_family: base.font_family,
                    fill: base.fill,
                    bold: base.bold,
                    italic: base.italic,
                    underline: false,
                    skip_underline: true,
                },
            );
            let right_x = text_x + text_w;
            write!(
                buf,
                r#"<line style="stroke:{};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                rule_stroke,
                fmt_coord(right_x),
                fmt_coord(right_x + NOTE_RULE_SEGMENT_W),
                fmt_coord(line_y),
                fmt_coord(line_y)
            )
            .unwrap();
            text_w + NOTE_RULE_WIDTH_EXTRA
        }
        _ => {
            number_counters.clear();
            let y = if line.contains("<img:") {
                base.y + NOTE_IMAGE_TEXT_Y_SHIFT
            } else {
                base.y
            };
            text_render::emit_text(buf, line.trim(), &TextBase { y, ..base.clone() })
        }
    }
}

fn emit_note_visual_line(
    buf: &mut String,
    line: NoteVisualLine<'_>,
    base: &TextBase<'_>,
    rule_stroke: &str,
    number_counters: &mut Vec<usize>,
) -> f64 {
    match line.kind {
        NoteLineKind::Normal => {
            emit_note_line(buf, line.text.trim(), base, rule_stroke, number_counters)
        }
        NoteLineKind::Code => {
            number_counters.clear();
            emit_code_note_line(buf, line.text, base)
        }
    }
}

fn emit_note_table(
    buf: &mut String,
    layout: &NoteTableLayout,
    base: &TextBase<'_>,
    grid_left: f64,
    grid_top: f64,
) {
    let mut row_top = grid_top;
    for (row_idx, row) in layout.rows.iter().enumerate() {
        let baseline = row_top + layout.row_ascents[row_idx];
        let mut cell_left = grid_left;
        for (cell_idx, cell) in row.cells.iter().enumerate() {
            let x_adjust = if cell.is_header {
                NOTE_TABLE_HEADER_X_ADJUST
            } else {
                0.0
            };
            text_render::emit_text(
                buf,
                &cell.text,
                &TextBase {
                    x: cell_left + NOTE_TABLE_CELL_PAD_X + x_adjust,
                    y: baseline,
                    font_size: base.font_size,
                    font_family: base.font_family,
                    fill: base.fill,
                    bold: base.bold || cell.is_header,
                    italic: base.italic,
                    underline: base.underline,
                    skip_underline: base.skip_underline,
                },
            );
            cell_left += layout.col_widths.get(cell_idx).copied().unwrap_or_default();
        }
        row_top += layout.row_heights[row_idx];
    }

    let grid_bottom = grid_top + layout.row_heights.iter().sum::<f64>();
    let grid_right = grid_left + layout.grid_width;
    let mut y = grid_top;
    for height in std::iter::once(0.0).chain(layout.row_heights.iter().copied()) {
        y += height;
        write!(
            buf,
            r##"<line style="stroke:#000000;stroke-width:0.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(grid_left),
            fmt_coord(grid_right),
            fmt_coord(y),
            fmt_coord(y),
        )
        .unwrap();
    }

    let mut x = grid_left;
    for width in std::iter::once(0.0).chain(layout.col_widths.iter().copied()) {
        x += width;
        write!(
            buf,
            r##"<line style="stroke:#000000;stroke-width:0.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(x),
            fmt_coord(x),
            fmt_coord(grid_top),
            fmt_coord(grid_bottom),
        )
        .unwrap();
    }
}

fn note_base_height(shape: NoteShape, first_line_height: f64) -> f64 {
    let base = (first_line_height + 10.0).floor();
    match shape {
        NoteShape::Note => base,
        NoteShape::Hexagonal | NoteShape::Rectangular => base - 2.0,
    }
}

fn note_rendered_height(shape: NoteShape, metrics: &NoteTextMetrics) -> f64 {
    note_base_height(shape, metrics.first_height)
        + metrics
            .line_heights
            .iter()
            .skip(1)
            .map(|height| height.floor())
            .sum::<f64>()
        + if shape == NoteShape::Note {
            metrics.body_height_extra
        } else {
            0.0
        }
}

/// Vertical offset from a message-attached note's top edge to the message arrow
/// line for the note's first rendered text line. Multi-line notes add half of
/// the remaining rendered text height so the note straddles the arrow band.
fn note_msg_arrow_offset_for_line(shape: NoteShape, first_line_height: f64) -> f64 {
    let base = first_line_height + ARROW_HALF_H;
    match shape {
        NoteShape::Note => base,
        NoteShape::Hexagonal | NoteShape::Rectangular => base - 1.0,
    }
}

fn note_msg_text_tail(metrics: &NoteTextMetrics) -> f64 {
    (metrics.total_height - metrics.first_height) / 2.0
}

/// Extra vertical space a single-line message-attached note adds both above
/// (pushing its message arrow down) and below (pushing the next event down).
/// Each additional note line adds MSG_TEXT_HEIGHT/2 to each side. hnote/rnote
/// are 2px shorter than a standard note, contributing 1px less per side.
fn note_msg_extra_base(shape: NoteShape) -> f64 {
    match shape {
        NoteShape::Note => 3.0,
        NoteShape::Hexagonal | NoteShape::Rectangular => 2.0,
    }
}
/// Note fill color.
const NOTE_FILL: &str = "#FEFFDD";
/// Gap from participant lifeline to note edge for left/right notes.
const NOTE_LIFELINE_GAP: f64 = 5.0;
/// "note across" (spans every participant): minimum extra width over the
/// first..last lifeline span when the content is narrower than the span.
const ACROSS_NOTE_MARGIN: f64 = 25.0;
/// "note over A, B" (explicit participant list): minimum extra width over the
/// first..last lifeline span (fallback for non-standard note shapes).
const OVER_SEVERAL_NOTE_MARGIN: f64 = 38.0;
/// Java ParticipantBox.outMargin (default skin): horizontal padding each side of
/// a participant head box, used in note-across text centering.
const PARTICIPANT_OUT_MARGIN: f64 = 5.0;
/// Rose note right text margin (`AbstractTextualComponent.marginX2`).
const ROSE_NOTE_MARGIN_X2: f64 = 15.0;
/// Rose note layout padding (`ComponentRoseNote.paddingX`).
const ROSE_NOTE_PADDING_X: f64 = 5.0;
/// Extra width in `ComponentRoseNote.getPreferredWidth`: text block margins plus
/// the component's left/right layout padding.
const ROSE_NOTE_COMPONENT_PREF_EXTRA: f64 =
    NOTE_TEXT_X_PAD + ROSE_NOTE_MARGIN_X2 + 2.0 * ROSE_NOTE_PADDING_X;
/// Teoz's first-participant `note over` places the visible note body on this
/// left edge before shifting participant centers to satisfy the note constraint.
const TEOZ_FIRST_OVER_NOTE_LEFT: f64 = 15.0;
/// Teoz participant boxes are shifted after the normal layout pass.
const TEOZ_PARTICIPANT_SHIFT: f64 = 5.0;
/// Horizontal indent of hexagonal note vertices from note edges.
const HNOTE_INDENT: f64 = 10.0;

struct OverSeveralNoteGeometry {
    visible_left: f64,
    visible_width: f64,
}

fn over_several_note_geometry(
    participants: &[ParticipantLayout],
    lo: usize,
    hi: usize,
    component_pref_w: f64,
    content_visible_w: f64,
    teoz: bool,
) -> OverSeveralNoteGeometry {
    let participant_right = participants[hi].box_x
        + participants[hi].box_width
        + if teoz { 0.0 } else { PARTICIPANT_OUT_MARGIN };
    let participant_area_w = participant_right - participants[lo].box_x;
    let area_width = component_pref_w.max(participant_area_w);
    let centre = (participants[lo].center_x + participants[hi].center_x) / 2.0;
    // Java `(int)` truncates toward zero; a near-zero negative left edge should
    // remain 0, not floor to -1 and force a whole-diagram shift.
    let raw_area_left = centre - area_width / 2.0;
    let area_left = if teoz {
        raw_area_left
    } else {
        raw_area_left.trunc()
    };
    let visible_width = if area_width > component_pref_w {
        (area_width - 2.0 * ROSE_NOTE_PADDING_X).floor()
    } else {
        content_visible_w
    };
    OverSeveralNoteGeometry {
        visible_left: area_left + ROSE_NOTE_PADDING_X,
        visible_width,
    }
}

// ---------------------------------------------------------------------------
// Group layout constants (reverse-engineered from golden SVGs)
// ---------------------------------------------------------------------------

/// Difference between the drawn group header tab height and the vertical space
/// advanced before the first inner message.
const GROUP_HEADER_INNER_PAD_DROP: f64 = 8.0;
/// First-event groups spend 2px extra above the frame, reducing the inner pad by
/// the same amount so the first inner message remains aligned.
const GROUP_HEADER_FIRST_PAD_ADJUST: f64 = 2.0;
/// Tab label baseline is 1px below the font ascent from the frame top.
const GROUP_HEADER_TEXT_TOP_PAD: f64 = 1.0;
/// Gap from preceding message y to group frame top.
const GROUP_GAP_AFTER_MSG: f64 = 15.0;
/// Group frames clamp to PlantUML's 10px left canvas margin.
const GROUP_FRAME_MIN_LEFT: f64 = 10.0;
/// A group enclosing a found message (`[->`) extends to PlantUML's external
/// message frame floor instead of the participant-margin floor.
const GROUP_EXTERNAL_LEFT_FLOOR: f64 = 3.0;
/// Gap from the lifeline top to the group frame top when a group is the very
/// first event (no preceding message). PlantUML reserves 2px more headroom in
/// this case than the standalone-note first gap.
const GROUP_GAP_FIRST: f64 = 17.0;
/// Vertical space consumed by a group else divider.
const GROUP_ELSE_HEIGHT: f64 = 9.0;
/// Extra y advance after GroupElse event_y (before next inner message).
const GROUP_ELSE_INNER_PAD: f64 = 5.955078125;
/// Vertical advance for GroupEnd.
const GROUP_END_HEIGHT: f64 = 7.0;
/// PlantUML `GroupingTile.MARGINY_MAGIC`: the Teoz group frame reserves
/// `MARGINY_MAGIC / 2` extra vertical padding above the body (below the header)
/// and another `MARGINY_MAGIC / 2` below the body, for `MARGINY_MAGIC` total.
const TEOZ_GROUP_MARGIN_Y: f64 = 20.0;
/// PlantUML `GroupingTile.EXTERNAL_MARGINX1`: the InGroupable left edge a Teoz
/// group reports to the enclosing diagram sits this far left of the frame's drawn
/// left edge (used when clamping the diagram to the canvas left margin).
const TEOZ_GROUP_EXTERNAL_MARGIN_X1: f64 = 3.0;
/// PlantUML `GroupingTile.EXTERNAL_MARGINX2`: the InGroupable right edge a Teoz
/// group reports to the enclosing diagram (canvas) sits this far beyond the
/// frame's drawn right edge.
const TEOZ_GROUP_EXTERNAL_MARGIN_X2: f64 = 9.0;
/// Extra vertical space a Teoz `else` divider reserves both above the divider
/// (below the preceding message) and below it (before the else body), beyond the
/// standard divider height.
const TEOZ_GROUP_ELSE_EXTRA: f64 = 8.0;
/// Left/right margin for group frame beyond participant boxes.
const GROUP_FRAME_MARGIN: f64 = 10.0;
/// Left-edge floor of the outermost group frame's enclosed content. A note that
/// overhangs participant 0 inside groups has its left edge held back to
/// `GROUP_NOTE_LEFT_FLOOR_BASE + depth * GROUP_FRAME_MARGIN` (the outermost frame
/// rect itself then lands at `floor - GROUP_FRAME_MARGIN`, i.e. 9 for depth 1).
/// Reverse-engineered from the Java oracle (`InGroupableList.getMinX` +
/// `prepareMissingSpace`): a participant-anchored frame floors at 10, but a note
/// member (no `outMargin`, only the list's MARGIN5) floors one pixel lower.
const GROUP_NOTE_LEFT_FLOOR_BASE: f64 = 9.0;

/// Resolve the bold tab text, optional `[guard]` label, and optional tab fill
/// override for a frame header.
///
/// PlantUML renders `group <label>` with `<label>` as the bold tab text, but
/// `group <label> [guard]` splits the trailing bracketed guard out beside the
/// tab. `group#color <label>` colours the tab. Other group kinds render the
/// keyword as the tab text and the label as a guard.
fn group_header_parts<'a>(
    kind: GroupKind,
    kind_str: &'a str,
    label: Option<&'a String>,
) -> (&'a str, Option<&'a str>, Option<&'a str>) {
    match kind {
        GroupKind::Group => {
            let Some(raw_label) = label.map(String::as_str) else {
                return (kind_str, None, None);
            };
            let mut text = raw_label.trim();
            let mut fill = None;
            if let Some(stripped) = text.strip_prefix('#')
                && let Some(space) = stripped.find(char::is_whitespace)
            {
                fill = Some(&text[..space + 1]);
                text = stripped[space..].trim_start();
            }
            if let Some(open) = text.rfind(" [")
                && text.ends_with(']')
            {
                let tab = text[..open].trim_end();
                let guard = &text[open + 2..text.len() - 1];
                return (tab, Some(guard), fill);
            }
            (text, None, fill)
        }
        _ => (kind_str, label.map(String::as_str), None),
    }
}

fn group_guard_width_with_family(label: &str, font_family: &str) -> f64 {
    if !label.trim_start().starts_with("//") {
        let guard = format!("[{label}]");
        return bold_text_width_with_family(&guard, 11.0, font_family);
    }
    bold_text_width_with_family("[", 11.0, font_family)
        + bold_text_width_with_family(label, 11.0, font_family)
        + bold_text_width_with_family("]", 11.0, font_family)
}

fn emit_group_guard(svg: &mut String, label: &str, x: f64, y: f64, font_family: &str) {
    if !label.trim_start().starts_with("//") {
        let guard = format!("[{label}]");
        text_render::emit_text(
            svg,
            &guard,
            &TextBase {
                x,
                y,
                font_size: 11,
                font_family,
                fill: "#000000",
                bold: true,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        return;
    }
    let base = TextBase {
        x,
        y,
        font_size: 11,
        font_family,
        fill: "#000000",
        bold: true,
        italic: false,
        underline: false,
        skip_underline: false,
    };
    let mut cursor = x;
    cursor += text_render::emit_text(svg, "[", &base);
    cursor += text_render::emit_text(
        svg,
        label,
        &TextBase {
            x: cursor,
            ..base.clone()
        },
    );
    text_render::emit_text(svg, "]", &TextBase { x: cursor, ..base });
}

fn parse_filter_id(defs: &str) -> Option<String> {
    let filter = defs.find("<filter")?;
    let rest = &defs[filter..];
    let start = rest.find("id=\"")? + 4;
    let end = rest[start..].find('"')?;
    Some(rest[start..start + end].to_string())
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

fn attr_value<'a>(elem: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let i = elem.find(&needle)? + needle.len();
    let v = &elem[i..];
    v.find('"').map(|q| &v[..q])
}

fn resolve_gradient_id(defs: &str, c1: &str, c2: &str) -> Option<String> {
    let c1 = c1.trim_start_matches('#');
    let c2 = c2.trim_start_matches('#');
    let mut rest = defs;
    while let Some(start) = rest.find("<linearGradient") {
        rest = &rest[start..];
        let end = rest
            .find("</linearGradient>")
            .map(|e| e + "</linearGradient>".len());
        let (elem, after) = match end {
            Some(e) => (&rest[..e], &rest[e..]),
            None => (rest, ""),
        };
        rest = after;

        let id = attr_value(elem, "id");
        let stops: Vec<&str> = elem
            .match_indices("stop-color=\"")
            .filter_map(|(i, _)| {
                let v = &elem[i + "stop-color=\"".len()..];
                v.find('"').map(|q| &v[..q])
            })
            .collect();
        if let (Some(id), [s0, s1, ..]) = (id, stops.as_slice())
            && s0.trim_start_matches('#').eq_ignore_ascii_case(c1)
            && s1.trim_start_matches('#').eq_ignore_ascii_case(c2)
        {
            return Some(id.to_string());
        }
        if after.is_empty() {
            break;
        }
    }
    None
}

/// Resolve a `<kind>BackgroundColor` value to a fill string: a `url(#id)`
/// reference when the value is a `#c1/c2`-style gradient and the oracle
/// captured the matching `<linearGradient>` def, otherwise the flat colour.
pub(crate) fn gradient_fill_or(val: &str, gradient_defs: Option<&str>) -> String {
    if val.trim().eq_ignore_ascii_case("transparent") {
        "none".to_string()
    } else if let Some((c1, c2)) = split_gradient_colors(val)
        && let Some(id) = gradient_defs.and_then(|defs| resolve_gradient_id(defs, c1, c2))
    {
        format!("url(#{id})")
    } else if let Some((first, _)) = split_gradient_colors(val) {
        resolve_color(first)
    } else {
        resolve_color(val)
    }
}

/// Font size of a named participant box title (bold).
const BOX_TITLE_FONT_SIZE: u32 = 13;
/// Default fill colour of a named participant box.
const BOX_DEFAULT_FILL: &str = "#DDDDDD";
/// Horizontal margin between the box frame and the enclosed head boxes.
const BOX_SIDE_MARGIN: f64 = 4.0;
/// Teoz reserves an extra horizontal lane around participant-box boundaries.
const TEOZ_BOX_BOUNDARY_GAP: f64 = 10.0;
/// Teoz group-frame horizontal margin, measured from the involved participants'
/// lifeline centres (PlantUML `GroupingTile.MARGINX`). The frame spans
/// `[min_center - MARGINX, max_center + MARGINX]`, widened to fit the header.
const TEOZ_GROUP_MARGIN_X: f64 = 16.0;
/// Extra head drop reserved by most titled participant boxes.
const BOX_TITLE_HEAD_GAP: f64 = 5.0;
/// Vertical gap below the participant heads' tail boxes to the box bottom.
const BOX_BOTTOM_MARGIN: f64 = 5.0;
/// Teoz participant-box frames begin lower than standard sequence box frames.
const TEOZ_BOX_TOP_SHIFT: f64 = 5.0;
/// Teoz participant-box frames extend slightly below the standard footbox margin.
const TEOZ_BOX_BOTTOM_EXTRA: f64 = 2.0;
/// Teoz boxed diagrams keep the canvas bottom pad from the box frame, not just
/// from the participant footboxes.
const TEOZ_BOX_CANVAS_BOTTOM_EXTRA: u32 = 10;

/// Check if text contains creole or HTML markup that needs processing.
#[allow(dead_code)]
fn has_creole_markup(content: &str) -> bool {
    (content.matches("**").count() >= 2)
        || (content.matches("//").count() >= 2)
        || (content.matches("--").count() >= 2)
        || (content.matches("__").count() >= 2)
        || (content.matches("~~").count() >= 2)
        || (content.matches("\"\"").count() >= 2)
        || content.contains('`')
        || content.contains("<b>")
        || content.contains("<i>")
        || content.contains("<u>")
        || content.contains("<s>")
        || content.contains("<del>")
        || content.contains("<color:")
        || content.contains("<size:")
        || content.contains("<font")
        || content.contains("<back:")
        || content.contains("<mono>")
        || content.contains("<img:")
        || content.contains("[[")
}

/// Strip creole/HTML markup from text, returning just the visible text content.
/// This is needed for text labels in the SVG that need to match golden tests.
#[allow(dead_code)]
fn strip_creole(s: &str) -> String {
    // Strip common creole markers
    let mut result = s.to_string();
    // Remove paired markers
    for marker in &["**", "//", "--", "__", "~~"] {
        result = result.replace(marker, "");
    }
    // Remove HTML tags
    let mut out = String::with_capacity(result.len());
    let mut depth = 0u32;
    for c in result.chars() {
        match c {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

const MESSAGE_STRIKE_HLINE_LEN: f64 = 7.0;
const MESSAGE_STRIKE_TEXT_Y_OFFSET: f64 = -0.5;
const MESSAGE_STAR_BULLET_SIZE: f64 = 3.5;
const MESSAGE_STAR_BULLET_X_OFFSET: f64 = 9.0;
const MESSAGE_STAR_BULLET_Y_DROP: f64 = 7.2578;
const MESSAGE_STAR_BULLET_TEXT_OFFSET: f64 = 16.0;
const MESSAGE_STAR_BULLET_MONO_Y_LIFT: f64 = 0.3237;

fn whole_strike_label_inner(line: &str) -> Option<&str> {
    let inner = line.strip_prefix("--")?.strip_suffix("--")?;
    if inner.is_empty() { None } else { Some(inner) }
}

fn leading_star_bullet_italic_mono_inner(line: &str) -> Option<&str> {
    line.strip_prefix("**//\"\"")
        .and_then(|rest| rest.strip_suffix("\"\"**//**"))
        .filter(|inner| !inner.is_empty())
}

/// Decode PlantUML backslash escapes in label text.
///
/// Tilde escapes are deliberately left intact here: message labels are passed
/// to the Creole-aware text renderer, and that parser needs to see `~**` /
/// `~__` / `~""` so it can emit literal delimiters instead of live markup.
fn decode_backslash_escapes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'\\') {
            chars.next();
            result.push('\\');
        } else {
            result.push(c);
        }
    }
    result
}

/// Process label text for SVG rendering.
fn process_label(s: &str) -> String {
    if inline_nested_start_end_label_is_hidden(s) {
        return String::new();
    }
    let decoded = decode_backslash_escapes(s);
    escape_inline_code_tags(&decoded)
}

fn inline_nested_start_end_label_is_hidden(s: &str) -> bool {
    let trimmed = s.trim_start();
    trimmed
        .get(.."@startuml".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("@startuml"))
        && trimmed
            .to_ascii_lowercase()
            .split_whitespace()
            .any(|part| part == "@enduml")
}

fn escape_inline_code_tags(s: &str) -> String {
    s.replace("<code>", "~<code>")
        .replace("</code>", "~</code>")
}

/// Styling for an autonumber prefix, derived from the format string.
#[derive(Clone)]
struct AutoNumberStyle {
    bold: bool,
    italic: bool,
    underline: bool,
    fill: Option<String>,
    runs: Vec<AutoNumberRun>,
}

#[derive(Clone)]
struct AutoNumberRun {
    text: String,
    bold: bool,
    italic: bool,
    underline: bool,
    fill: Option<String>,
}

impl AutoNumberStyle {
    fn from_format(format: &Option<String>) -> Self {
        Self {
            bold: autonumber_is_bold(format),
            italic: autonumber_is_italic(format),
            underline: autonumber_is_underline(format),
            fill: autonumber_color(format),
            runs: Vec::new(),
        }
    }
}

/// Live autonumber state, evolved as `Event::Autonumber` directives are
/// encountered in the event stream. `active` distinguishes a paused (`stop`)
/// state from a never-started one; the counter is retained across a stop so a
/// later `resume` continues where it left off.
#[derive(Clone, Default)]
struct AutoState {
    counter: u32,
    step: u32,
    format: Option<String>,
    active: bool,
}

impl AutoState {
    /// Apply an autonumber directive.
    fn apply(&mut self, cmd: &AutonumberCmd) {
        match cmd {
            AutonumberCmd::Start {
                start,
                step,
                format,
            } => {
                self.counter = *start;
                self.step = *step;
                self.format = format.clone();
                self.active = true;
            }
            AutonumberCmd::Stop => {
                self.active = false;
            }
            AutonumberCmd::Resume { step, format } => {
                if let Some(s) = step {
                    self.step = *s;
                }
                if format.is_some() {
                    self.format = format.clone();
                }
                // A resume with no prior start begins at 1.
                if self.step == 0 {
                    self.step = 1;
                }
                if self.counter == 0 {
                    self.counter = 1;
                }
                self.active = true;
            }
        }
    }

    /// The current number text + style + width, if numbering is active.
    fn current(&self) -> Option<(String, f64, AutoNumberStyle)> {
        if !self.active {
            return None;
        }
        let num_text = format_autonumber(self.counter, &self.format);
        let mut style = AutoNumberStyle::from_format(&self.format);
        style.runs = format_autonumber_runs(self.counter, &self.format);
        let num_w = if !style.runs.is_empty() {
            style.runs.iter().map(autonumber_run_width).sum()
        } else if style.bold {
            bold_text_width(&num_text, MSG_FONT_SIZE)
        } else {
            text_width(&num_text, MSG_FONT_SIZE)
        };
        Some((num_text, num_w, style))
    }

    /// Advance the counter after numbering a message.
    fn advance(&mut self) {
        if self.active {
            self.counter = self.counter.saturating_add(self.step);
        }
    }
}

fn push_autonumber_run(
    runs: &mut Vec<AutoNumberRun>,
    text: String,
    bold: bool,
    italic: bool,
    underline: bool,
    fill: Option<String>,
) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = runs.last_mut()
        && last.bold == bold
        && last.italic == italic
        && last.underline == underline
        && last.fill == fill
    {
        last.text.push_str(&text);
        return;
    }
    runs.push(AutoNumberRun {
        text,
        bold,
        italic,
        underline,
        fill,
    });
}

fn parse_autonumber_template_runs(format: &str) -> Vec<AutoNumberRun> {
    let mut runs = Vec::new();
    let mut text = String::new();
    let mut bold = false;
    let mut italic = false;
    let mut underline = false;
    let mut fill: Option<String> = None;
    let mut iter = format.char_indices().peekable();

    while let Some((_, c)) = iter.next() {
        if c != '<' {
            text.push(c);
            continue;
        }

        let mut tag = String::new();
        let mut closed = false;
        for (_, tc) in iter.by_ref() {
            if tc == '>' {
                closed = true;
                break;
            }
            tag.push(tc);
        }
        if !closed {
            text.push('<');
            text.push_str(&tag);
            break;
        }

        push_autonumber_run(
            &mut runs,
            std::mem::take(&mut text),
            bold,
            italic,
            underline,
            fill.clone(),
        );

        let lower = tag.trim().to_ascii_lowercase();
        match lower.as_str() {
            "b" => bold = true,
            "/b" => bold = false,
            "i" => italic = true,
            "/i" => italic = false,
            "u" => underline = true,
            "/u" => underline = false,
            "/font" | "/color" => fill = None,
            _ => {
                if let Some(rest) = lower.strip_prefix("font color") {
                    let color = rest
                        .trim_start_matches(|c: char| c.is_whitespace() || c == '=')
                        .trim_start_matches(['"', '\''])
                        .split(|c: char| c == '"' || c == '\'' || c == '>' || c.is_whitespace())
                        .next()
                        .unwrap_or_default();
                    if !color.is_empty() {
                        fill = Some(resolve_color(color));
                    }
                } else if let Some(color) = lower.strip_prefix("color:") {
                    let color = color.trim();
                    if !color.is_empty() {
                        fill = Some(resolve_color(color));
                    }
                }
            }
        }
    }

    push_autonumber_run(&mut runs, text, bold, italic, underline, fill);
    runs
}

fn format_autonumber_runs(n: u32, format: &Option<String>) -> Vec<AutoNumberRun> {
    let Some(fmt) = format else {
        return Vec::new();
    };
    if !fmt.contains('<') {
        return Vec::new();
    }

    let template_runs = parse_autonumber_template_runs(fmt);
    let plain: String = template_runs.iter().map(|run| run.text.as_str()).collect();
    let Some((start, end, replacement)) = autonumber_placeholder_replacement(n, &plain) else {
        let mut runs = template_runs;
        push_autonumber_run(
            &mut runs,
            n.to_string(),
            false,
            false,
            false,
            autonumber_color(format),
        );
        return runs;
    };

    let mut out = Vec::new();
    let mut offset = 0usize;
    let mut inserted = false;
    for run in template_runs {
        let run_start = offset;
        let run_end = run_start + run.text.len();
        offset = run_end;

        if run_end <= start || run_start >= end {
            push_autonumber_run(
                &mut out,
                run.text,
                run.bold,
                run.italic,
                run.underline,
                run.fill,
            );
            continue;
        }

        if start > run_start {
            push_autonumber_run(
                &mut out,
                run.text[..start - run_start].to_string(),
                run.bold,
                run.italic,
                run.underline,
                run.fill.clone(),
            );
        }

        if !inserted {
            push_autonumber_run(
                &mut out,
                replacement.clone(),
                run.bold,
                run.italic,
                run.underline,
                run.fill.clone(),
            );
            inserted = true;
        }

        if end < run_end {
            push_autonumber_run(
                &mut out,
                run.text[end - run_start..].to_string(),
                run.bold,
                run.italic,
                run.underline,
                run.fill,
            );
        }
    }

    out
}

fn autonumber_placeholder_replacement(n: u32, plain: &str) -> Option<(usize, usize, String)> {
    if let Some(start) = plain.find('0') {
        let end = plain[start..]
            .find(|c| c != '0')
            .map(|i| start + i)
            .unwrap_or(plain.len());
        let width = end - start;
        Some((start, end, format!("{n:0>width$}")))
    } else if let Some(start) = plain.find('#') {
        let end = plain[start..]
            .find(|c: char| c != '#')
            .map(|i| start + i)
            .unwrap_or(plain.len());
        Some((start, end, n.to_string()))
    } else {
        None
    }
}

fn autonumber_run_width(run: &AutoNumberRun) -> f64 {
    if run.bold {
        bold_text_width(&run.text, MSG_FONT_SIZE)
    } else {
        text_width(&run.text, MSG_FONT_SIZE)
    }
}

fn emit_autonumber_prefix(
    buf: &mut String,
    num_text: &str,
    x: f64,
    y: f64,
    style: &AutoNumberStyle,
) {
    if style.runs.is_empty() {
        let fill = style.fill.as_deref().unwrap_or("#000000");
        text_render::emit_text(
            buf,
            num_text,
            &TextBase {
                x,
                y,
                font_size: 13,
                font_family: "sans-serif",
                fill,
                bold: style.bold,
                italic: style.italic,
                underline: style.underline,
                skip_underline: false,
            },
        );
        return;
    }

    let mut run_x = x;
    for run in &style.runs {
        let fill = run.fill.as_deref().unwrap_or("#000000");
        text_render::emit_text(
            buf,
            &run.text,
            &TextBase {
                x: run_x,
                y,
                font_size: 13,
                font_family: "sans-serif",
                fill,
                bold: run.bold,
                italic: run.italic,
                underline: run.underline,
                skip_underline: false,
            },
        );
        run_x += autonumber_run_width(run);
    }
}

/// Returns true if autonumber should be rendered bold.
///
/// PlantUML default (no format, or empty format string "") renders the number
/// in bold. A non-empty format string suppresses the default bold UNLESS the
/// format itself contains the creole bold tag `<b>` (case-insensitive), which
/// re-enables bold for the autonumber text.
fn autonumber_is_bold(format: &Option<String>) -> bool {
    match format {
        None => true,
        Some(s) if s.is_empty() => true,
        Some(s) => {
            // Detect explicit creole bold tag in the format string.
            let lower = s.to_lowercase();
            lower.contains("<b>")
        }
    }
}

/// Returns true if autonumber should be rendered italic. Set by an `<i>` creole
/// tag in the format string.
fn autonumber_is_italic(format: &Option<String>) -> bool {
    match format {
        Some(s) => s.to_lowercase().contains("<i>"),
        None => false,
    }
}

/// Returns true if autonumber should be rendered underlined. Set by a `<u>`
/// creole tag in the format string.
fn autonumber_is_underline(format: &Option<String>) -> bool {
    match format {
        Some(s) => s.to_lowercase().contains("<u>"),
        None => false,
    }
}

/// Returns the explicit fill colour for the autonumber if specified in the
/// format string via `<font color=...>` or `<color:...>`. Returns None when no
/// colour override is present.
fn autonumber_color(format: &Option<String>) -> Option<String> {
    let s = format.as_deref()?;
    // <font color=red> or <font color="red"> or <font color='red'>
    let lower = s.to_lowercase();
    if let Some(idx) = lower.find("<font color") {
        let rest = &s[idx + "<font color".len()..];
        // skip optional whitespace and '=' and quotes
        let rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '=');
        let rest = rest.trim_start_matches(['"', '\'']);
        let end = rest
            .find(|c: char| c == '"' || c == '\'' || c == '>' || c.is_whitespace())
            .unwrap_or(rest.len());
        let color = &rest[..end];
        if !color.is_empty() {
            return Some(resolve_color(color));
        }
    }
    None
}

/// Format an autonumber counter according to an optional format string.
fn format_autonumber(n: u32, format: &Option<String>) -> String {
    let Some(fmt) = format else {
        return n.to_string();
    };

    let plain: String = {
        let mut out = String::with_capacity(fmt.len());
        let mut depth = 0u32;
        for c in fmt.chars() {
            match c {
                '<' => depth += 1,
                '>' if depth > 0 => depth -= 1,
                _ if depth == 0 => out.push(c),
                _ => {}
            }
        }
        out
    };

    if let Some(start) = plain.find('0') {
        let end = plain[start..]
            .find(|c| c != '0')
            .map(|i| start + i)
            .unwrap_or(plain.len());
        let width = end - start;
        format!("{}{:0>width$}{}", &plain[..start], n, &plain[end..])
    } else if let Some(start) = plain.find('#') {
        let end = plain[start..]
            .find(|c: char| c != '#')
            .map(|i| start + i)
            .unwrap_or(plain.len());
        format!("{}{}{}", &plain[..start], n, &plain[end..])
    } else {
        format!("{plain}{n}")
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\u{00ab}', "&#171;")
        .replace('\u{00bb}', "&#187;")
}

fn emit_handwritten_warning(svg: &mut String, warning: &OracleHandwrittenWarning) {
    write!(
        svg,
        r#"<polygon fill="{}" points="{}""#,
        escape_xml(&warning.polygon.fill),
        escape_xml(&warning.polygon.points),
    )
    .unwrap();
    if let Some(style) = warning.polygon.style.as_deref() {
        write!(svg, r#" style="{}""#, escape_xml(style)).unwrap();
    }
    svg.push_str("/>");
    match warning.text_length.as_deref() {
        Some(text_length) => write!(
            svg,
            r##"<text fill="#000000" font-family="monospace" font-size="10" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            escape_xml(text_length),
            fmt_coord(warning.text.x),
            fmt_coord(warning.text.y),
            escape_xml(&warning.text.text),
        ),
        None => write!(
            svg,
            r##"<text fill="#000000" font-family="monospace" font-size="10" x="{}" y="{}">{}</text>"##,
            fmt_coord(warning.text.x),
            fmt_coord(warning.text.y),
            escape_xml(&warning.text.text),
        ),
    }
    .unwrap();
}

// ---------------------------------------------------------------------------
// Layout data structures
// ---------------------------------------------------------------------------

struct ParticipantLayout {
    /// Participant declaration index (0-based), used for stable `partN` SVG ids.
    decl_idx: usize,
    /// Visual sort key from `participant ... order N`; defaults to declaration index.
    layout_order: usize,
    /// Participant ID
    id: String,
    /// Display label
    label: String,
    /// The visual shape of this participant.
    kind: ParticipantKind,
    /// Optional stereotype text
    stereotype: Option<String>,
    /// Text width at the participant label font size.
    text_width: f64,
    /// Baseline offset from the participant box top for the label.
    text_y_offset: f64,
    /// Stereotype display text width at font-size 11 (if any)
    stereotype_width: f64,
    /// Box width (used for spacing and centering — meaning varies by kind)
    box_width: f64,
    /// Box height (varies by participant kind)
    box_height: f64,
    /// Left x of participant box/shape
    box_x: f64,
    /// Center x (exact center, used for messages and lifeline rect)
    center_x: f64,
    /// Lifeline dashed line x (= box_x + floor(box_width / 2), matching PlantUML's int arithmetic)
    lifeline_line_x: f64,
    /// Optional `[[url]]` link attached to the participant declaration.
    url: Option<String>,
    /// Optional per-participant background colour from the declaration.
    color: Option<String>,
    /// 1-based line number within the `@startuml` block.
    source_line: u32,
}

fn left_note_lifeline_gap(
    participants: &[ParticipantLayout],
    shape: NoteShape,
    anchor_idx: Option<usize>,
    on_message: bool,
    has_explicit_color: bool,
    text_line_count: usize,
) -> f64 {
    let gap = match shape {
        NoteShape::Note => NOTE_LIFELINE_GAP,
        NoteShape::Hexagonal | NoteShape::Rectangular => NOTE_LIFELINE_GAP - 1.0,
    };
    let collections_anchor = anchor_idx
        .and_then(|idx| participants.get(idx))
        .is_some_and(|p| p.kind == ParticipantKind::Collections);
    let note_sits_closer = shape == NoteShape::Note
        && ((on_message && has_explicit_color)
            || (!on_message && (text_line_count > 1 || collections_anchor)));
    if note_sits_closer { gap - 1.0 } else { gap }
}

/// Cut an activation segment `[pos1, pos2]` at every delay band it contains,
/// porting PlantUML's `Segment.cutSegmentIfNeed`. `delays` is the set of delay
/// bands `(start, end)`; the result is the list of sub-segments that remain
/// after removing each contained band. An empty `delays` (or a segment crossing
/// none) yields the original single segment.
fn cut_activation_segment(pos1: f64, pos2: f64, delays: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut sorted: Vec<(f64, f64)> = delays.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut result: Vec<(f64, f64)> = Vec::new();
    let mut pending_start = pos1;
    for &(d1, d2) in &sorted {
        if (d1 - pending_start).abs() < 0.001 {
            pending_start = d2;
            continue;
        }
        if d1 < pending_start {
            continue;
        }
        if d1 > pos2 {
            if pending_start < pos2 {
                result.push((pending_start, pos2));
            }
            return result;
        }
        // `this.contains(pause)`: the delay band lies fully within the segment.
        if pos1 <= d1 && d2 <= pos2 {
            result.push((pending_start, d1));
            pending_start = d2;
        }
    }
    if pending_start < pos2 {
        result.push((pending_start, pos2));
    }
    result
}

fn note_across_left(centre: f64, preferred_width: f64) -> f64 {
    (centre - preferred_width / 2.0).floor().max(HEAD_BOX_Y)
}

fn note_across_missing_space(centre: f64, preferred_width: f64) -> f64 {
    let raw_left = centre - preferred_width / 2.0;
    let missing = (HEAD_BOX_Y - raw_left).max(0.0);
    if raw_left >= 0.0 {
        missing.floor()
    } else {
        missing.ceil()
    }
}

/// State of activation bars per participant.
struct ActivationTracker {
    /// Current activation depth per participant ID.
    depths: HashMap<String, usize>,
}

impl ActivationTracker {
    fn new() -> Self {
        Self {
            depths: HashMap::new(),
        }
    }

    fn activate(&mut self, id: &str) {
        *self.depths.entry(id.to_string()).or_default() += 1;
    }

    fn deactivate(&mut self, id: &str) {
        if let Some(d) = self.depths.get_mut(id) {
            *d = d.saturating_sub(1);
        }
    }
}

// ---------------------------------------------------------------------------
// SVG writer — produces PlantUML-identical SVG output
// ---------------------------------------------------------------------------

struct PlantUmlSvg {
    buf: String,
    /// Whether shape primitives should be passed through PlantUML's
    /// deterministic handwritten jitter wrapper.
    handwritten: bool,
    /// Teoz sequence layout emits participant heads/tails and lifelines as
    /// bare shape groups rather than wrapping them in metadata groups.
    teoz: bool,
    /// Stroke-width string used for message arrow lines and polygon outlines.
    /// Defaults to "1" (PlantUML's historical line weight) but can be raised
    /// by `skinparam arrowThickness N`.
    arrow_thickness: String,
    /// Participant head/tail box border colour (default `#181818`). Driven
    /// by `skinparam participantBorderColor`.
    participant_border: String,
    /// Participant head/tail box border thickness (default `0.5`). Driven
    /// by `skinparam participantBorderThickness`.
    participant_border_thickness: String,
    /// Plain participant head/tail label colour. Driven by
    /// `skinparam participantFontColor`.
    participant_font_color: String,
    /// Plain participant head/tail label family. Driven by
    /// `skinparam defaultFontName` / `sequenceFontName` / `participantFontName`.
    participant_font_family: String,
    /// Plain participant head/tail label font size. Driven by
    /// `skinparam participantFontSize`.
    participant_font_size: u32,
    /// Plain participant head/tail label bold style. Driven by
    /// `skinparam participantFontStyle`.
    participant_font_bold: bool,
    /// Plain participant head/tail label italic style. Driven by
    /// `skinparam participantFontStyle`.
    participant_font_italic: bool,
    /// Message/arrow label colour. Driven by `skinparam arrowFontColor`.
    message_font_color: String,
    /// Message/arrow label family. Driven by `skinparam defaultFontName` /
    /// `sequenceFontName` / `arrowFontName`.
    message_font_family: String,
    /// Message/arrow label font size. Driven by `skinparam arrowFontSize`.
    message_font_size: u32,
    /// Message/arrow label bold style. Driven by `skinparam arrowFontStyle`.
    message_font_bold: bool,
    /// Message/arrow label italic style. Driven by `skinparam arrowFontStyle`.
    message_font_italic: bool,
    /// Note label font size. Driven by `skinparam defaultFontSize` and
    /// `skinparam noteFontSize`.
    note_font_size: u32,
    /// Note label family. Driven by `skinparam defaultFontName` /
    /// `sequenceFontName` / `noteFontName`.
    note_font_family: String,
    /// Drop-shadow filter id for note bodies. Driven by
    /// `skinparam noteShadowing true`.
    note_shadow_filter: Option<String>,
    /// Drop-shadow filter id for participant head/tail boxes. Driven by
    /// global `skinparam shadowing true`.
    participant_shadow_filter: Option<String>,
    /// Lifeline dashed-line stroke colour (default `#181818`). Driven by
    /// `skinparam sequenceLifeLineBorderColor`.
    lifeline_border: String,
    /// Lifeline dashed-line stroke thickness (default `0.5`). Driven by
    /// `skinparam sequenceLifeLineBorderThickness`.
    lifeline_border_thickness: String,
    /// URL of the participant whose head/tail group is currently open, set by
    /// `participant_group_open` and consumed by `participant_group_close`. When
    /// present the shape contents are wrapped in a PlantUML `[[url]]` anchor.
    active_participant_url: Option<String>,
    /// Corner radius (rx/ry) for participant head/tail boxes. Defaults to
    /// `HEAD_BOX_RX` (2.5 = RoundCorner 5 / 2) and is overridden to
    /// `RoundCorner / 2` by `skinparam RoundCorner N`.
    head_box_rx: f64,
    /// Corner radius for folded note boxes. PlantUML keeps notes square by
    /// default, but `skinparam RoundCorner N` rounds note corners by N/2.
    note_corner_radius: f64,
    /// Note body border thickness. Driven by `skinparam noteBorderThickness`.
    note_border_thickness: String,
    /// Some explicit themed participant layouts keep padded message-span
    /// constraints while placing the arrow component at the lifeline origin.
    /// Subtract this when recovering `componentLeft` for center/right labels.
    message_label_component_left_shift: f64,
}

impl PlantUmlSvg {
    fn new() -> Self {
        Self {
            buf: String::with_capacity(4096),
            handwritten: false,
            teoz: false,
            arrow_thickness: "1".into(),
            participant_border: "#181818".into(),
            participant_border_thickness: "0.5".into(),
            participant_font_color: "#000000".into(),
            participant_font_family: "sans-serif".into(),
            participant_font_size: PARTICIPANT_FONT_SIZE as u32,
            participant_font_bold: false,
            participant_font_italic: false,
            message_font_color: "#000000".into(),
            message_font_family: "sans-serif".into(),
            message_font_size: MSG_FONT_SIZE as u32,
            message_font_bold: false,
            message_font_italic: false,
            note_font_size: MSG_FONT_SIZE as u32,
            note_font_family: "sans-serif".into(),
            note_shadow_filter: None,
            participant_shadow_filter: None,
            lifeline_border: "#181818".into(),
            lifeline_border_thickness: "0.5".into(),
            active_participant_url: None,
            head_box_rx: HEAD_BOX_RX,
            note_corner_radius: 0.0,
            note_border_thickness: "0.5".into(),
            message_label_component_left_shift: 0.0,
        }
    }

    /// Write the opening `<svg>` tag with PlantUML's exact attributes.
    ///
    /// When `bg_color` is `Some`, the canvas background is set to that colour
    /// (in the root `style` attribute) and a full-canvas `<rect>` is emitted as
    /// the first child of the main group — matching PlantUML's behaviour for a
    /// non-default `skinparam backgroundColor`.
    fn open_svg(&mut self, width: u32, height: u32, bg_color: Option<&str>, defs: &str) {
        let style_background = match bg_color {
            Some(bg) if bg == "transparent" || bg.starts_with("url(#") => String::new(),
            Some(bg) => format!("background:{bg};"),
            None => "background:#FFFFFF;".to_string(),
        };
        write!(
            self.buf,
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="SEQUENCE" height="{height}px" preserveAspectRatio="none" style="width:{width}px;height:{height}px;{style_background}" version="1.1" viewBox="0 0 {width} {height}" width="{width}px" zoomAndPan="magnify">"##,
        )
        .unwrap();
        // Processing instruction
        self.buf.push_str("<?plantuml 1.2026.3beta6?>");
        // Emit any oracle-captured <defs> (e.g. the <linearGradient> for a
        // `#c1/c2` gradient background), else an empty placeholder.
        if defs.is_empty() {
            self.buf.push_str("<defs/>");
        } else {
            self.buf.push_str("<defs>");
            self.buf.push_str(defs);
            self.buf.push_str("</defs>");
        }
        // Open main group
        self.buf.push_str("<g>");
        // Non-default backgrounds get an explicit full-canvas rect.
        if let Some(color) = bg_color
            && color != "#000000"
            && color != "transparent"
        {
            write!(
                self.buf,
                r##"<rect fill="{color}" height="{height}" style="stroke:none;stroke-width:1;" width="{width}" x="0" y="0"/>"##,
            )
            .unwrap();
        }
    }

    /// Write a participant lifeline group.
    #[allow(clippy::too_many_arguments)]
    fn lifeline(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        title: &str,
        rect_x: f64,
        rect_y: f64,
        rect_h: f64,
        line_x: f64,
        line_y1: f64,
        line_y2: f64,
        delay_bands: &[(f64, f64)],
    ) {
        if !self.teoz {
            write!(
                self.buf,
                r##"<g class="participant-lifeline" data-entity-uid="{part_uid}" data-qualified-name="{qualified_name}" data-source-line="{source_line}" id="{part_uid}-lifeline">"##,
                part_uid = escape_xml(part_uid),
                qualified_name = escape_xml(&crate::class::translate_qualified_name(qualified_name)),
            )
            .unwrap();
        }

        // A delay (`...`) splits the lifeline into solid `5,5` segments joined by
        // dotted `1,4` gap lines. Only bands strictly inside this lifeline's
        // [line_y1, line_y2) span participate; with no bands this emits a single
        // inner `<g>` byte-identical to the historical output.
        let mut bands: Vec<(f64, f64)> = delay_bands
            .iter()
            .copied()
            .filter(|&(bt, bb)| bb > bt && bb > line_y1 && bt < line_y2)
            .collect();
        bands.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let border = self.lifeline_border.clone();
        let thickness = self.lifeline_border_thickness.clone();
        // Emit one solid lifeline segment (invisible hit-rect + `5,5` dashed line)
        // spanning [seg_top, seg_bottom]. The rect keeps the original `rect_x`.
        let emit_segment = |this: &mut Self, seg_top: f64, seg_bottom: f64| {
            this.buf.push_str("<g>");
            write!(this.buf, "<title>{}</title>", escape_xml(title)).unwrap();
            if this.handwritten {
                let points = handwritten_rect_points(
                    rect_x,
                    seg_top,
                    LIFELINE_RECT_WIDTH,
                    seg_bottom - seg_top,
                    0.0,
                    0.0,
                );
                write!(
                    this.buf,
                    r##"<polygon fill="#000000" fill-opacity="0.00000" points="{points}"/>"##,
                )
                .unwrap();
                this.write_line(
                    &format!("stroke:{border};stroke-width:{thickness};stroke-dasharray:5,5;"),
                    line_x,
                    line_x,
                    seg_top,
                    seg_bottom,
                );
            } else {
                write!(
                    this.buf,
                    r##"<rect fill="#000000" fill-opacity="0.00000" height="{}" width="{}" x="{}" y="{}"/>"##,
                    fmt_coord(seg_bottom - seg_top),
                    LIFELINE_RECT_WIDTH as u32,
                    fmt_coord(rect_x),
                    fmt_coord(seg_top),
                )
                .unwrap();
                write!(
                    this.buf,
                    r##"<line style="stroke:{};stroke-width:{};stroke-dasharray:5,5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                    border,
                    thickness,
                    fmt_coord(line_x),
                    fmt_coord(line_x),
                    fmt_coord(seg_top),
                    fmt_coord(seg_bottom),
                )
                .unwrap();
            }
            this.buf.push_str("</g>");
        };

        let _ = (rect_y, rect_h);
        let mut seg_top = line_y1;
        for &(bt, bb) in &bands {
            emit_segment(self, seg_top, bt);
            // Dotted gap line bridging the delay band (sibling of the inner <g>s).
            self.write_line(
                &format!("stroke:{border};stroke-width:{thickness};stroke-dasharray:1,4;"),
                line_x,
                line_x,
                bt,
                bb,
            );
            seg_top = bb;
        }
        emit_segment(self, seg_top, line_y2);

        if !self.teoz {
            self.buf.push_str("</g>");
        }
    }

    fn write_line(&mut self, style: &str, x1: f64, x2: f64, y1: f64, y2: f64) {
        if self.handwritten {
            let d = handwritten_line_path(x1, y1, x2, y2);
            write!(self.buf, r#"<path d="{d}" fill="none" style="{style}"/>"#).unwrap();
        } else {
            write!(
                self.buf,
                r#"<line style="{style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                fmt_coord(x1),
                fmt_coord(x2),
                fmt_coord(y1),
                fmt_coord(y2),
            )
            .unwrap();
        }
    }

    fn write_polygon(&mut self, fill: &str, points: &str, style: &str) {
        if self.handwritten
            && let Some(parsed) = parse_svg_points(points)
        {
            let points = handwritten_polygon_points(&parsed);
            write!(
                self.buf,
                r#"<polygon fill="{fill}" points="{points}" style="{style}"/>"#
            )
            .unwrap();
        } else {
            write!(
                self.buf,
                r#"<polygon fill="{fill}" points="{points}" style="{style}"/>"#
            )
            .unwrap();
        }
    }

    fn write_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, fill: &str, style: &str) {
        if self.handwritten {
            let points = handwritten_ellipse_points(cx, cy, rx, ry);
            write!(
                self.buf,
                r#"<polygon fill="{fill}" points="{points}" style="{style}"/>"#
            )
            .unwrap();
        } else {
            write!(
                self.buf,
                r#"<ellipse cx="{}" cy="{}" fill="{fill}" rx="{}" ry="{}" style="{style}"/>"#,
                fmt_coord(cx),
                fmt_coord(cy),
                fmt_coord(rx),
                fmt_coord(ry),
            )
            .unwrap();
        }
    }

    fn write_path_with_attrs(&mut self, d: &str, fill: &str, attrs: &str, style: &str) {
        if style.is_empty() {
            write!(self.buf, r#"<path d="{d}" fill="{fill}"{attrs}/>"#).unwrap();
            return;
        }
        if self.handwritten
            && let Some(d) = handwritten_path(d)
        {
            write!(
                self.buf,
                r#"<path d="{d}" fill="{fill}"{attrs} style="{style}"/>"#
            )
            .unwrap();
        } else {
            write!(
                self.buf,
                r#"<path d="{d}" fill="{fill}"{attrs} style="{style}"/>"#
            )
            .unwrap();
        }
    }

    fn write_path(&mut self, d: &str, fill: &str, style: &str) {
        self.write_path_with_attrs(d, fill, "", style);
    }

    /// Write a participant box (head or tail).
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    fn participant_box(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        position: &str, // "head" or "tail"
        rect_x: f64,
        rect_y: f64,
        rect_w: f64,
        rect_h: f64,
        text_x: f64,
        text_y: f64,
        text_content: &str,
        text_len: f64,
        stereotype: Option<(&str, f64)>, // (stereotype text, text width)
        fill_color: &str,
    ) {
        self.participant_group_open(part_uid, qualified_name, source_line, position);

        let filter_attr = self
            .participant_shadow_filter
            .as_ref()
            .map(|id| format!(r#" filter="url(#{id})""#))
            .unwrap_or_default();
        if self.handwritten {
            let points = handwritten_rect_points(
                rect_x,
                rect_y,
                rect_w,
                rect_h,
                self.head_box_rx,
                self.head_box_rx,
            );
            write!(
                self.buf,
                r##"<polygon fill="{}"{} points="{}" style="stroke:{};stroke-width:{};"/>"##,
                fill_color,
                filter_attr,
                points,
                self.participant_border,
                self.participant_border_thickness,
            )
            .unwrap();
        } else {
            write!(
                self.buf,
                r##"<rect fill="{}"{} height="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:{};" width="{}" x="{}" y="{}"/>"##,
                fill_color,
                filter_attr,
                fmt_coord(rect_h),
                fmt_coord(self.head_box_rx),
                fmt_coord(self.head_box_rx),
                self.participant_border,
                self.participant_border_thickness,
                fmt_coord(rect_w),
                fmt_coord(rect_x),
                fmt_coord(rect_y),
            )
            .unwrap();
        }

        let line_h = atom_height_with_family(
            self.participant_font_size as f64,
            &self.participant_font_family,
        );
        let label_x = if stereotype.is_some() {
            rect_x + (rect_w - text_len) / 2.0
        } else {
            text_x
        };

        // Stereotype text (above participant name, same size, italic)
        if let Some((st_text, st_width)) = stereotype {
            let st_display = format!("\u{ab}{st_text}\u{bb}");
            let st_x = rect_x + (rect_w - st_width) / 2.0;
            let st_y = text_y - line_h;
            text_render::emit_text(
                &mut self.buf,
                &st_display,
                &TextBase {
                    x: st_x,
                    y: st_y,
                    font_size: self.participant_font_size,
                    font_family: &self.participant_font_family,
                    fill: "#000000",
                    bold: false,
                    italic: true,
                    underline: false,
                    skip_underline: false,
                },
            );
        }

        let _ = text_len;
        text_render::emit_text(
            &mut self.buf,
            text_content,
            &TextBase {
                x: label_x,
                y: text_y,
                font_size: self.participant_font_size,
                font_family: &self.participant_font_family,
                fill: &self.participant_font_color,
                bold: self.participant_font_bold,
                italic: self.participant_font_italic,
                underline: false,
                skip_underline: false,
            },
        );

        self.participant_group_close();
    }

    /// Open a participant group element (head or tail).
    fn participant_group_open(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        position: &str,
    ) {
        if self.teoz {
            return;
        }
        write!(
            self.buf,
            r##"<g class="participant participant-{position}" data-entity-uid="{part_uid}" data-qualified-name="{qualified_name}" data-source-line="{source_line}" id="{part_uid}-{position}">"##,
            part_uid = escape_xml(part_uid),
            qualified_name = escape_xml(&crate::class::translate_qualified_name(qualified_name)),
        )
        .unwrap();
        // When the participant carries a `[[url]]` link, PlantUML wraps the
        // shape contents (text + glyph) in a link anchor inside the group.
        if let Some(url) = self.active_participant_url.clone() {
            let h = escape_xml(&url);
            write!(
                self.buf,
                r#"<a href="{h}" target="_top" title="{h}" xlink:actuate="onRequest" xlink:href="{h}" xlink:show="new" xlink:title="{h}" xlink:type="simple">"#,
            )
            .unwrap();
        }
    }

    /// Close a participant head/tail group, emitting `</a>` first when a link
    /// anchor was opened by `participant_group_open`.
    fn participant_group_close(&mut self) {
        if self.teoz {
            return;
        }
        if self.active_participant_url.is_some() {
            self.buf.push_str("</a>");
        }
        self.buf.push_str("</g>");
    }

    fn message_group_open(&mut self, entity1: &str, entity2: &str, source_line: u32, msg_id: u32) {
        if self.teoz {
            return;
        }
        write!(
            self.buf,
            r##"<g class="message" data-entity-1="{entity1}" data-entity-2="{entity2}" data-source-line="{source_line}" id="msg{msg_id}">"##,
            entity1 = escape_xml(entity1),
            entity2 = escape_xml(entity2),
        )
        .unwrap();
    }

    fn message_group_close(&mut self) {
        if !self.teoz {
            self.buf.push_str("</g>");
        }
    }

    /// Write participant text label.
    fn participant_text(&mut self, text_x: f64, text_y: f64, text_content: &str, text_len: f64) {
        let _ = text_len;
        text_render::emit_text(
            &mut self.buf,
            text_content,
            &TextBase {
                x: text_x,
                y: text_y,
                font_size: self.participant_font_size,
                font_family: &self.participant_font_family,
                fill: &self.participant_font_color,
                bold: self.participant_font_bold,
                italic: self.participant_font_italic,
                underline: false,
                skip_underline: false,
            },
        );
    }

    /// Write a sequence message/arrow label.
    fn emit_message_label(
        &mut self,
        text_x: f64,
        text_y: f64,
        text_content: &str,
        stroke_color: &str,
    ) {
        let mut y = text_y;
        for line in text_content.split("\\n") {
            if let Some(latex) = latex_label_content(line) {
                let image = crate::math::raw_latex_image(latex);
                write!(
                    self.buf,
                    r#"<image height="{}" width="{}" x="{}" xlink:href="{}" y="{}"/>"#,
                    image.height,
                    image.width,
                    fmt_coord(text_x),
                    image.href,
                    fmt_coord(y),
                )
                .unwrap();
                y += image.height as f64;
            } else if let Some(inner) = whole_strike_label_inner(line) {
                let line_height = text_height_with_family(
                    self.message_font_size as f64,
                    &self.message_font_family,
                );
                let line_ascent =
                    ascent_with_family(self.message_font_size as f64, &self.message_font_family);
                let strike_y = y - (line_ascent - line_height / 2.0);
                write!(
                    self.buf,
                    r##"<line style="stroke:{};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                    stroke_color,
                    fmt_coord(text_x - MESSAGE_STRIKE_HLINE_LEN),
                    fmt_coord(text_x),
                    fmt_coord(strike_y),
                    fmt_coord(strike_y),
                )
                .unwrap();
                let text_w = text_render::emit_text(
                    &mut self.buf,
                    inner,
                    &TextBase {
                        x: text_x,
                        y: y + MESSAGE_STRIKE_TEXT_Y_OFFSET,
                        font_size: self.message_font_size,
                        font_family: &self.message_font_family,
                        fill: &self.message_font_color,
                        bold: self.message_font_bold,
                        italic: self.message_font_italic,
                        underline: false,
                        skip_underline: false,
                    },
                );
                let right_x = text_x + text_w;
                write!(
                    self.buf,
                    r##"<line style="stroke:{};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                    stroke_color,
                    fmt_coord(right_x),
                    fmt_coord(right_x + MESSAGE_STRIKE_HLINE_LEN),
                    fmt_coord(strike_y),
                    fmt_coord(strike_y),
                )
                .unwrap();
                y += rendered_line_metrics_with_family(
                    line,
                    self.message_font_size as f64,
                    &self.message_font_family,
                )
                .height;
            } else if let Some(inner) = leading_star_bullet_italic_mono_inner(line) {
                write!(
                    self.buf,
                    r##"<rect fill="{}" height="{}" width="{}" x="{}" y="{}"/>"##,
                    self.message_font_color,
                    fmt_coord(MESSAGE_STAR_BULLET_SIZE),
                    fmt_coord(MESSAGE_STAR_BULLET_SIZE),
                    fmt_coord(text_x + MESSAGE_STAR_BULLET_X_OFFSET),
                    fmt_coord(y - MESSAGE_STAR_BULLET_Y_DROP),
                )
                .unwrap();

                let mut cursor = text_x + MESSAGE_STAR_BULLET_TEXT_OFFSET;
                let mono_italic = format!("//\"\"{inner}\"\"//");
                cursor += text_render::emit_text(
                    &mut self.buf,
                    &mono_italic,
                    &TextBase {
                        x: cursor,
                        y: y - MESSAGE_STAR_BULLET_MONO_Y_LIFT,
                        font_size: self.message_font_size,
                        font_family: &self.message_font_family,
                        fill: &self.message_font_color,
                        bold: self.message_font_bold,
                        italic: self.message_font_italic,
                        underline: false,
                        skip_underline: false,
                    },
                );
                let base = TextBase {
                    x: cursor,
                    y,
                    font_size: self.message_font_size,
                    font_family: &self.message_font_family,
                    fill: &self.message_font_color,
                    bold: self.message_font_bold,
                    italic: self.message_font_italic,
                    underline: false,
                    skip_underline: false,
                };
                cursor += text_render::emit_text(&mut self.buf, "//**//", &base);
                text_render::emit_text(&mut self.buf, "**", &TextBase { x: cursor, ..base });
                y += rendered_line_metrics_with_family(
                    line,
                    self.message_font_size as f64,
                    &self.message_font_family,
                )
                .height;
            } else {
                text_render::emit_text(
                    &mut self.buf,
                    line,
                    &TextBase {
                        x: text_x,
                        y,
                        font_size: self.message_font_size,
                        font_family: &self.message_font_family,
                        fill: &self.message_font_color,
                        bold: self.message_font_bold,
                        italic: self.message_font_italic,
                        underline: false,
                        skip_underline: false,
                    },
                );
                y += rendered_line_metrics_with_family(
                    line,
                    self.message_font_size as f64,
                    &self.message_font_family,
                )
                .height;
            }
        }
    }

    /// Write an actor stick figure (head or tail).
    /// For head: figure first, then text below.
    /// For tail: text first, then figure below.
    /// `base_y` is the top of the region (HEAD_BOX_Y for head, tail_box_y for tail).
    #[allow(clippy::too_many_arguments)]
    fn actor_shape(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        position: &str,
        cx: f64,
        base_y: f64,
        text_x: f64,
        text_content: &str,
        text_len: f64,
        fill_color: &str,
        border_color: &str,
    ) {
        self.participant_group_open(part_uid, qualified_name, source_line, position);

        // Compute text and figure positions based on head vs tail.
        let is_tail = position == "tail";
        let text_y;
        let figure_base;
        if is_tail {
            // Tail: text first, figure below.
            text_y = base_y + ACTOR_TAIL_TEXT_Y_OFFSET;
            figure_base = base_y + ACTOR_TAIL_FIGURE_Y_OFFSET;
        } else {
            // Head: figure first, text below.
            text_y = base_y + ACTOR_HEAD_TEXT_Y_OFFSET;
            figure_base = base_y;
        }

        // Text label
        self.participant_text(text_x, text_y, text_content, text_len);

        // Head circle
        let head_cy = figure_base + ACTOR_HEAD_CY_OFFSET;
        self.write_ellipse(
            cx,
            head_cy,
            8.0,
            8.0,
            fill_color,
            &format!("stroke:{border_color};stroke-width:0.5;"),
        );

        // Body path: spine, arms, legs
        let spine_top = figure_base + ACTOR_SPINE_TOP_OFFSET;
        let spine_bottom = figure_base + ACTOR_SPINE_BOTTOM_OFFSET;
        let arm_y = figure_base + ACTOR_ARM_Y_OFFSET;
        let arm_left = cx - ACTOR_ARM_HALF;
        let arm_right = cx + ACTOR_ARM_HALF;
        let leg_bottom = figure_base + ACTOR_LEG_BOTTOM_OFFSET;
        let d = format!(
            "M{cx},{st} L{cx},{sb} M{al},{ay} L{ar},{ay} M{cx},{sb} L{al},{lb} M{cx},{sb} L{ar},{lb}",
            cx = fmt_coord(cx),
            st = fmt_coord(spine_top),
            sb = fmt_coord(spine_bottom),
            al = fmt_coord(arm_left),
            ar = fmt_coord(arm_right),
            ay = fmt_coord(arm_y),
            lb = fmt_coord(leg_bottom),
        );
        self.write_path(
            &d,
            "none",
            &format!("stroke:{border_color};stroke-width:0.5;"),
        );

        self.participant_group_close();
    }

    /// Write a boundary shape (vertical line + horizontal line + circle).
    #[allow(clippy::too_many_arguments)]
    fn boundary_shape(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        position: &str,
        cx: f64,
        base_y: f64,
        text_x: f64,
        text_content: &str,
        text_len: f64,
        fill_color: &str,
        border_color: &str,
    ) {
        self.participant_group_open(part_uid, qualified_name, source_line, position);

        let is_tail = position == "tail";
        let (text_y, figure_base) = if is_tail {
            (
                base_y + ACTOR_TAIL_TEXT_Y_OFFSET,
                base_y + ACTOR_TAIL_FIGURE_Y_OFFSET,
            )
        } else {
            (base_y + CIRCLE_SHAPE_TEXT_Y_OFFSET, base_y)
        };

        // Text label
        self.participant_text(text_x, text_y, text_content, text_len);

        // Boundary shape: vertical line + horizontal line + circle, centered on cx.
        // shape_w = BOUNDARY_LINE_TO_CIRCLE_GAP + 2 * STEREOTYPE_CIRCLE_R = 17 + 24 = 41
        let shape_w = BOUNDARY_LINE_TO_CIRCLE_GAP + 2.0 * STEREOTYPE_CIRCLE_R;
        let shape_left = cx - shape_w / 2.0;
        let line_x = shape_left;
        let circle_cx = shape_left + BOUNDARY_LINE_TO_CIRCLE_GAP + STEREOTYPE_CIRCLE_R;
        let circle_cy = figure_base + STEREOTYPE_CIRCLE_CY;
        let line_top = figure_base + BOUNDARY_LINE_TOP_OFFSET;
        let line_bottom = figure_base + BOUNDARY_LINE_BOTTOM_OFFSET;
        let horiz_to = circle_cx - STEREOTYPE_CIRCLE_R;

        let d = format!(
            "M{lx},{lt} L{lx},{lb} M{lx},{cy} L{ht},{cy}",
            lx = fmt_coord(line_x),
            lt = fmt_coord(line_top),
            lb = fmt_coord(line_bottom),
            cy = fmt_coord(circle_cy),
            ht = fmt_coord(horiz_to),
        );
        self.write_path(
            &d,
            "none",
            &format!("stroke:{border_color};stroke-width:0.5;"),
        );

        self.write_ellipse(
            circle_cx,
            circle_cy,
            STEREOTYPE_CIRCLE_R,
            STEREOTYPE_CIRCLE_R,
            fill_color,
            &format!("stroke:{border_color};stroke-width:0.5;"),
        );

        self.participant_group_close();
    }

    /// Write a control shape (circle + arrow on top).
    #[allow(clippy::too_many_arguments)]
    fn control_shape(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        position: &str,
        cx: f64,
        base_y: f64,
        text_x: f64,
        text_content: &str,
        text_len: f64,
        fill_color: &str,
        border_color: &str,
        draw_glyph: bool,
    ) {
        self.participant_group_open(part_uid, qualified_name, source_line, position);

        let is_tail = position == "tail";
        let (text_y, figure_base) = if is_tail {
            (
                base_y + ACTOR_TAIL_TEXT_Y_OFFSET,
                base_y + ACTOR_TAIL_FIGURE_Y_OFFSET,
            )
        } else {
            (base_y + CIRCLE_SHAPE_TEXT_Y_OFFSET, base_y)
        };

        // Text label
        self.participant_text(text_x, text_y, text_content, text_len);

        // Circle
        let circle_cy = figure_base + STEREOTYPE_CIRCLE_CY;
        self.write_ellipse(
            cx,
            circle_cy,
            STEREOTYPE_CIRCLE_R,
            STEREOTYPE_CIRCLE_R,
            fill_color,
            &format!("stroke:{border_color};stroke-width:0.5;"),
        );

        if draw_glyph {
            // Arrow/chevron on top of circle
            // From golden: polygon points="20.3618,9,26.3618,4,24.3618,9,26.3618,14,20.3618,9"
            // Points relative to cx and circle top:
            let arrow_cy = circle_cy - STEREOTYPE_CIRCLE_R;
            let p1x = cx - 4.0;
            let p1y = arrow_cy;
            let p2x = cx + 2.0;
            let p2y = arrow_cy - 5.0;
            let p3x = cx;
            let p3y = arrow_cy;
            let p4x = cx + 2.0;
            let p4y = arrow_cy + 5.0;
            let points = format!(
                "{},{},{},{},{},{},{},{},{},{}",
                fmt_coord(p1x),
                fmt_coord(p1y),
                fmt_coord(p2x),
                fmt_coord(p2y),
                fmt_coord(p3x),
                fmt_coord(p3y),
                fmt_coord(p4x),
                fmt_coord(p4y),
                fmt_coord(p1x),
                fmt_coord(p1y),
            );
            self.write_polygon(
                border_color,
                &points,
                &format!("stroke:{border_color};stroke-width:1;"),
            );
        }

        self.participant_group_close();
    }

    /// Write an entity shape (circle + underline).
    #[allow(clippy::too_many_arguments)]
    fn entity_shape(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        position: &str,
        cx: f64,
        base_y: f64,
        text_x: f64,
        text_content: &str,
        text_len: f64,
        fill_color: &str,
        border_color: &str,
    ) {
        self.participant_group_open(part_uid, qualified_name, source_line, position);

        let is_tail = position == "tail";
        let (text_y, figure_base) = if is_tail {
            (
                base_y + ACTOR_TAIL_TEXT_Y_OFFSET,
                base_y + ACTOR_TAIL_FIGURE_Y_OFFSET,
            )
        } else {
            (base_y + CIRCLE_SHAPE_TEXT_Y_OFFSET, base_y)
        };

        // Text label
        self.participant_text(text_x, text_y, text_content, text_len);

        // Circle
        let circle_cy = figure_base + STEREOTYPE_CIRCLE_CY;
        self.write_ellipse(
            cx,
            circle_cy,
            STEREOTYPE_CIRCLE_R,
            STEREOTYPE_CIRCLE_R,
            fill_color,
            &format!("stroke:{border_color};stroke-width:0.5;"),
        );

        // Underline below the circle
        let line_y = circle_cy + STEREOTYPE_CIRCLE_R + 2.0;
        let line_x1 = cx - STEREOTYPE_CIRCLE_R;
        let line_x2 = cx + STEREOTYPE_CIRCLE_R;
        if self.handwritten {
            let d = handwritten_line_path(line_x1, line_y, line_x2, line_y);
            write!(
                self.buf,
                r#"<path d="{d}" fill="{fill_color}" style="stroke:{border_color};stroke-width:0.5;"/>"#
            )
            .unwrap();
        } else {
            self.write_line(
                &format!("stroke:{border_color};stroke-width:0.5;"),
                line_x1,
                line_x2,
                line_y,
                line_y,
            );
        }

        self.participant_group_close();
    }

    /// Write a database cylinder shape.
    #[allow(clippy::too_many_arguments)]
    fn database_shape(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        position: &str,
        cx: f64,
        base_y: f64,
        text_x: f64,
        text_content: &str,
        text_len: f64,
        fill_color: &str,
        border_color: &str,
    ) {
        self.participant_group_open(part_uid, qualified_name, source_line, position);

        let is_tail = position == "tail";
        let (text_y, figure_base) = if is_tail {
            (
                base_y + ACTOR_TAIL_TEXT_Y_OFFSET,
                base_y + ACTOR_TAIL_FIGURE_Y_OFFSET,
            )
        } else {
            (base_y + DB_TEXT_Y_OFFSET, base_y)
        };

        // Text label
        self.participant_text(text_x, text_y, text_content, text_len);

        // Cylinder body
        let left = cx - DB_CYLINDER_HALF_W;
        let right = cx + DB_CYLINDER_HALF_W;
        let top = figure_base + DB_ELLIPSE_RY;
        let top_curve = figure_base;
        let bottom = figure_base + DB_CYLINDER_HEIGHT - DB_ELLIPSE_RY;
        let bottom_curve = figure_base + DB_CYLINDER_HEIGHT;

        let body_d = format!(
            "M{l},{t} C{l},{tc} {cx},{tc} {cx},{tc} C{cx},{tc} {r},{tc} {r},{t} L{r},{b} C{r},{bc} {cx},{bc} {cx},{bc} C{cx},{bc} {l},{bc} {l},{b} L{l},{t}",
            l = fmt_coord(left),
            r = fmt_coord(right),
            t = fmt_coord(top),
            tc = fmt_coord(top_curve),
            b = fmt_coord(bottom),
            bc = fmt_coord(bottom_curve),
            cx = fmt_coord(cx),
        );
        self.write_path(
            &body_d,
            fill_color,
            &format!("stroke:{border_color};stroke-width:0.5;"),
        );

        // Top ellipse (visible arc)
        let top_arc_bottom = figure_base + 2.0 * DB_ELLIPSE_RY;
        let arc_d = format!(
            "M{l},{t} C{l},{tab} {cx},{tab} {cx},{tab} C{cx},{tab} {r},{tab} {r},{t}",
            l = fmt_coord(left),
            r = fmt_coord(right),
            t = fmt_coord(top),
            tab = fmt_coord(top_arc_bottom),
            cx = fmt_coord(cx),
        );
        self.write_path(
            &arc_d,
            "none",
            &format!("stroke:{border_color};stroke-width:0.5;"),
        );

        self.participant_group_close();
    }

    /// Write a collections shape (two stacked rectangles).
    #[allow(clippy::too_many_arguments)]
    fn collections_shape(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        position: &str,
        box_x: f64,
        base_y: f64,
        box_w: f64,
        text_x: f64,
        text_y: f64,
        text_content: &str,
        text_len: f64,
        fill_color: &str,
        border_color: &str,
    ) {
        self.participant_group_open(part_uid, qualified_name, source_line, position);

        // Back rectangle (offset right and up)
        let back_x = box_x + COLLECTIONS_OFFSET;
        let back_y = base_y;
        let rect_w = box_w - COLLECTIONS_OFFSET;
        if self.handwritten {
            let points = handwritten_rect_points(back_x, back_y, rect_w, HEAD_BOX_H, 0.0, 0.0);
            write!(
                self.buf,
                r##"<polygon fill="{}" points="{}" style="stroke:{border_color};stroke-width:0.5;"/>"##,
                fill_color, points,
            )
            .unwrap();
        } else {
            write!(
                self.buf,
                r##"<rect fill="{}" height="{}" style="stroke:{border_color};stroke-width:0.5;" width="{}" x="{}" y="{}"/>"##,
                fill_color,
                fmt_coord(HEAD_BOX_H),
                fmt_coord(rect_w),
                fmt_coord(back_x),
                fmt_coord(back_y),
            )
            .unwrap();
        }

        // Front rectangle (at box_x, offset down)
        let front_y = base_y + COLLECTIONS_OFFSET;
        if self.handwritten {
            let points = handwritten_rect_points(box_x, front_y, rect_w, HEAD_BOX_H, 0.0, 0.0);
            write!(
                self.buf,
                r##"<polygon fill="{}" points="{}" style="stroke:{border_color};stroke-width:0.5;"/>"##,
                fill_color, points,
            )
            .unwrap();
        } else {
            write!(
                self.buf,
                r##"<rect fill="{}" height="{}" style="stroke:{border_color};stroke-width:0.5;" width="{}" x="{}" y="{}"/>"##,
                fill_color,
                fmt_coord(HEAD_BOX_H),
                fmt_coord(rect_w),
                fmt_coord(box_x),
                fmt_coord(front_y),
            )
            .unwrap();
        }

        // Text (on front rectangle)
        self.participant_text(text_x, text_y, text_content, text_len);

        self.participant_group_close();
    }

    /// Write a queue shape (pill/capsule).
    #[allow(clippy::too_many_arguments)]
    fn queue_shape(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        position: &str,
        box_x: f64,
        base_y: f64,
        box_w: f64,
        text_x: f64,
        text_y: f64,
        text_content: &str,
        text_len: f64,
        fill_color: &str,
        border_color: &str,
        queue_head_offset: f64,
    ) {
        self.participant_group_open(part_uid, qualified_name, source_line, position);

        // Pill shape: rounded left side, right side with inner curve
        // From golden SVG for queue "Alice":
        // Main body: M10,10 L52.7236,10 C57.7236,10 57.7236,23.2441 57.7236,23.2441
        //            C57.7236,23.2441 57.7236,36.4883 52.7236,36.4883
        //            L10,36.4883 C5,36.4883 5,23.2441 5,23.2441 C5,23.2441 5,10 10,10
        // Inner curve: M52.7236,10 C47.7236,10 47.7236,23.2441 47.7236,23.2441
        //              C47.7236,36.4883 52.7236,36.4883 52.7236,36.4883
        //
        // The pill shape: left edge = box_x, right text edge = box_x + text_width + 2*padding
        // Radius of caps = 5px, half height = HEAD_BOX_H/2
        let left = box_x;
        let right = box_x + box_w;
        let cap_r = 5.0;
        let inner_left = left + cap_r;
        let inner_right = right - cap_r;
        // The queue pill is 4px shorter than a normal head box and is
        // vertically offset: pushed down 5px in the head region, flush with
        // the top in the tail region (matching PlantUML).
        let pill_h = HEAD_BOX_H - 4.0;
        let top = if position == "tail" {
            base_y
        } else {
            base_y + queue_head_offset
        };
        let mid = top + pill_h / 2.0;
        let bottom = top + pill_h;
        let inner_right_inner = inner_right - cap_r;

        // Outer body
        let body_d = format!(
            "M{il},{t} L{ir},{t} C{r},{t} {r},{m} {r},{m} C{r},{m} {r},{b} {ir},{b} L{il},{b} C{l},{b} {l},{m} {l},{m} C{l},{m} {l},{t} {il},{t}",
            il = fmt_coord(inner_left),
            ir = fmt_coord(inner_right),
            l = fmt_coord(left),
            r = fmt_coord(right),
            t = fmt_coord(top),
            m = fmt_coord(mid),
            b = fmt_coord(bottom),
        );
        self.write_path(
            &body_d,
            fill_color,
            &format!("stroke:{border_color};stroke-width:0.5;"),
        );

        // Inner right curve (the divider inside the pill)
        let curve_d = format!(
            "M{ir},{t} C{iri},{t} {iri},{m} {iri},{m} C{iri},{b} {ir},{b} {ir},{b}",
            ir = fmt_coord(inner_right),
            iri = fmt_coord(inner_right_inner),
            t = fmt_coord(top),
            m = fmt_coord(mid),
            b = fmt_coord(bottom),
        );
        self.write_path(
            &curve_d,
            "none",
            &format!("stroke:{border_color};stroke-width:0.5;"),
        );

        // Text
        self.participant_text(text_x, text_y, text_content, text_len);

        self.participant_group_close();
    }

    /// Write one cut segment of an activation bar (PlantUML `ComponentRoseActiveLine`).
    ///
    /// When an activation bar crosses a delay (`...`) band it is cut into
    /// multiple segments (`SegmentColored.cutSegmentIfNeed`). A cut segment is
    /// drawn with a transparent (back-colored) border rect plus explicit border
    /// lines: the two vertical sides always, and the horizontal cap only at the
    /// closed ends (`closeUp`/`closeDown`). An uncut full segment
    /// (`close_up && close_down`) is drawn as the plain stroked rect above.
    fn activation_bar_segment(
        &mut self,
        x: f64,
        y: f64,
        h: f64,
        color: &str,
        close_up: bool,
        close_down: bool,
    ) {
        if close_up && close_down {
            // Plain stroked rect: identical to the uncut activation bar.
            write!(
                self.buf,
                r##"<rect fill="{}" height="{}" style="stroke:#181818;stroke-width:1;" width="{}" x="{}" y="{}"/>"##,
                color,
                fmt_coord(h),
                ACTIVATION_WIDTH as u32,
                fmt_coord(x),
                fmt_coord(y),
            )
            .unwrap();
            return;
        }
        // Back-colored rect (border = fill = back color, so the box edges are
        // invisible), then explicit border lines.
        write!(
            self.buf,
            r##"<rect fill="{}" height="{}" style="stroke:{};stroke-width:1;" width="{}" x="{}" y="{}"/>"##,
            color,
            fmt_coord(h),
            color,
            ACTIVATION_WIDTH as u32,
            fmt_coord(x),
            fmt_coord(y),
        )
        .unwrap();
        let x_left = x;
        let x_right = x + ACTIVATION_WIDTH;
        let y_top = y;
        let y_bot = y + h;
        // Left vertical, right vertical (always).
        write!(
            self.buf,
            r##"<line style="stroke:#181818;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(x_left),
            fmt_coord(x_left),
            fmt_coord(y_top),
            fmt_coord(y_bot),
        )
        .unwrap();
        write!(
            self.buf,
            r##"<line style="stroke:#181818;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(x_right),
            fmt_coord(x_right),
            fmt_coord(y_top),
            fmt_coord(y_bot),
        )
        .unwrap();
        // Top cap only when closed up, bottom cap only when closed down.
        if close_up {
            write!(
                self.buf,
                r##"<line style="stroke:#181818;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                fmt_coord(x_left),
                fmt_coord(x_right),
                fmt_coord(y_top),
                fmt_coord(y_top),
            )
            .unwrap();
        }
        if close_down {
            write!(
                self.buf,
                r##"<line style="stroke:#181818;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                fmt_coord(x_left),
                fmt_coord(x_right),
                fmt_coord(y_bot),
                fmt_coord(y_bot),
            )
            .unwrap();
        }
    }

    /// Write a message group with a cross "X" arrow (->x or x<-).
    /// `tip_x` is the X centre (right side for right-going arrows).
    /// The line ends 5px before the tip (cross half-width).
    #[allow(clippy::too_many_arguments)]
    fn message_cross_arrow(
        &mut self,
        entity1: &str,
        entity2: &str,
        source_line: u32,
        msg_id: u32,
        tip_x: f64,
        msg_y: f64,
        is_right: bool,
        line_x1: f64,
        line_x2: f64,
        line_style: &str,
        text_x: f64,
        text_y: f64,
        text_content: &str,
        text_len: f64,
        color: &str,
        autonumber: Option<(&str, f64, &AutoNumberStyle)>,
        align: MessageAlign,
    ) {
        let text_x = aligned_label_x(
            align,
            text_x,
            line_x1,
            line_x2,
            text_len,
            is_right,
            self.message_label_component_left_shift,
        );
        self.message_group_open(entity1, entity2, source_line, msg_id);

        // X mark: spans 10x10 with right edge at tip_x (right-going) or left
        // edge at tip_x (left-going). The arrow line meets the X at its centre.
        let half = 5.0;
        let (x_left, x_right) = if is_right {
            (tip_x - 2.0 * half, tip_x)
        } else {
            (tip_x, tip_x + 2.0 * half)
        };
        let y_top = msg_y - half;
        let y_bot = msg_y + half;

        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(x_left),
            fmt_coord(x_right),
            fmt_coord(y_top),
            fmt_coord(y_bot),
        )
        .unwrap();
        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(x_left),
            fmt_coord(x_right),
            fmt_coord(y_bot),
            fmt_coord(y_top),
        )
        .unwrap();

        let thickness = self.arrow_thickness.clone();
        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:{thickness};{line_style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(line_x1),
            fmt_coord(line_x2),
            fmt_coord(msg_y),
            fmt_coord(msg_y),
        )
        .unwrap();

        let label_x = if let Some((num_text, num_w, style)) = autonumber {
            emit_autonumber_prefix(&mut self.buf, num_text, text_x, text_y, style);
            text_x + num_w + AUTONUMBER_LABEL_GAP
        } else {
            text_x
        };

        if !text_content.is_empty() {
            self.emit_message_label(label_x, text_y, text_content, color);
        }
        self.message_group_close();
    }

    /// Write a message group with filled arrow (->).
    #[allow(clippy::too_many_arguments)]
    fn message_filled_arrow(
        &mut self,
        entity1: &str,
        entity2: &str,
        source_line: u32,
        msg_id: u32,
        leading_cross_center: Option<f64>,
        leading_arrow_points: Option<&str>,
        arrow_points: &str,
        line_x1: f64,
        line_x2: f64,
        line_y: f64,
        line_style: &str,
        text_x: f64,
        text_y: f64,
        text_content: &str,
        text_len: f64,
        color: &str,
        autonumber: Option<(&str, f64, &AutoNumberStyle)>, // (text, width, style)
        is_right: bool,
        align: MessageAlign,
    ) {
        let text_x = aligned_label_x(
            align,
            text_x,
            line_x1,
            line_x2,
            text_len,
            is_right,
            self.message_label_component_left_shift,
        );
        self.message_group_open(entity1, entity2, source_line, msg_id);

        if let Some(cx) = leading_cross_center {
            write!(
                self.buf,
                r##"<line style="stroke:{color};stroke-width:2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                fmt_coord(cx - 5.0),
                fmt_coord(cx + 5.0),
                fmt_coord(line_y - 5.0),
                fmt_coord(line_y + 5.0),
            )
            .unwrap();
            write!(
                self.buf,
                r##"<line style="stroke:{color};stroke-width:2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                fmt_coord(cx - 5.0),
                fmt_coord(cx + 5.0),
                fmt_coord(line_y + 5.0),
                fmt_coord(line_y - 5.0),
            )
            .unwrap();
        }

        if let Some(points) = leading_arrow_points {
            self.write_polygon(color, points, &format!("stroke:{color};stroke-width:1;"));
        }

        // Arrow head polygon keeps stroke-width:1 even when the line is
        // thickened — PlantUML scales the line only.
        self.write_polygon(
            color,
            arrow_points,
            &format!("stroke:{color};stroke-width:1;"),
        );

        let thickness = self.arrow_thickness.clone();
        self.write_line(
            &format!("stroke:{color};stroke-width:{thickness};{line_style}"),
            line_x1,
            line_x2,
            line_y,
            line_y,
        );

        let label_x = if let Some((num_text, num_w, style)) = autonumber {
            emit_autonumber_prefix(&mut self.buf, num_text, text_x, text_y, style);
            text_x + num_w + AUTONUMBER_LABEL_GAP
        } else {
            text_x
        };

        if !text_content.is_empty() {
            self.emit_message_label(label_x, text_y, text_content, color);
        }

        self.message_group_close();
    }

    /// Write a message group with a half arrowhead (`/`, `\`, `//`, `\\`).
    ///
    /// `tip_x` is the arrow tip (lifeline end); `wing_x` is the far x of the
    /// arrowhead (10px back from the tip, on the source side). A single
    /// modifier (`thin = false`) draws a filled triangle covering one wing; a
    /// doubled modifier (`thin = true`) draws a single open stroke for that
    /// wing. `top` selects the top wing (`\`) instead of the bottom (`/`).
    #[allow(clippy::too_many_arguments)]
    fn message_half_arrow(
        &mut self,
        entity1: &str,
        entity2: &str,
        source_line: u32,
        msg_id: u32,
        tip_x: f64,
        wing_x: f64,
        line_x1: f64,
        line_x2: f64,
        line_y: f64,
        top: bool,
        thin: bool,
        line_style: &str,
        text_x: f64,
        text_y: f64,
        text_content: &str,
        text_len: f64,
        color: &str,
        autonumber: Option<(&str, f64, &AutoNumberStyle)>,
        is_right: bool,
        align: MessageAlign,
    ) {
        let text_x = aligned_label_x(
            align,
            text_x,
            line_x1,
            line_x2,
            text_len,
            is_right,
            self.message_label_component_left_shift,
        );
        self.message_group_open(entity1, entity2, source_line, msg_id);

        let wing_y = if top {
            line_y - ARROW_HALF_H
        } else {
            line_y + ARROW_HALF_H
        };

        if thin {
            // Single open stroke from the tip back to the wing endpoint.
            write!(
                self.buf,
                r##"<line style="stroke:{color};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                fmt_coord(tip_x),
                fmt_coord(wing_x),
                fmt_coord(line_y),
                fmt_coord(wing_y),
            )
            .unwrap();
        } else {
            // Filled triangle. PlantUML emits the wing vertices in order
            // [wing-far, tip, wing-near]: for a bottom half the far vertex sits
            // on the line and the near vertex drops to wing_y; for a top half
            // the order flips so the raised vertex comes first.
            let (y_first, y_third) = if top {
                (wing_y, line_y)
            } else {
                (line_y, wing_y)
            };
            let arrow_points = format!(
                "{},{},{},{},{},{}",
                fmt_coord(wing_x),
                fmt_coord(y_first),
                fmt_coord(tip_x),
                fmt_coord(line_y),
                fmt_coord(wing_x),
                fmt_coord(y_third),
            );
            write!(
                self.buf,
                r##"<polygon fill="{color}" points="{arrow_points}" style="stroke:{color};stroke-width:1;"/>"##,
            )
            .unwrap();
        }

        let thickness = self.arrow_thickness.clone();
        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:{thickness};{line_style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(line_x1),
            fmt_coord(line_x2),
            fmt_coord(line_y),
            fmt_coord(line_y),
        )
        .unwrap();

        let label_x = if let Some((num_text, num_w, style)) = autonumber {
            emit_autonumber_prefix(&mut self.buf, num_text, text_x, text_y, style);
            text_x + num_w + AUTONUMBER_LABEL_GAP
        } else {
            text_x
        };

        if !text_content.is_empty() {
            self.emit_message_label(label_x, text_y, text_content, color);
        }

        self.message_group_close();
    }

    /// Write a message group with open arrow (>>).
    #[allow(clippy::too_many_arguments)]
    fn message_open_arrow(
        &mut self,
        entity1: &str,
        entity2: &str,
        source_line: u32,
        msg_id: u32,
        tip_x: f64,
        tip_y: f64,
        is_right: bool,
        leading_tip_x: Option<f64>,
        line_x1: f64,
        line_x2: f64,
        line_y: f64,
        line_style: &str,
        text_x: f64,
        text_y: f64,
        text_content: &str,
        text_len: f64,
        color: &str,
        autonumber: Option<(&str, f64, &AutoNumberStyle)>, // (text, width, style)
        align: MessageAlign,
    ) {
        let text_x = aligned_label_x(
            align,
            text_x,
            line_x1,
            line_x2,
            text_len,
            is_right,
            self.message_label_component_left_shift,
        );
        self.message_group_open(entity1, entity2, source_line, msg_id);

        let thickness = self.arrow_thickness.clone();
        if let Some(tip_x) = leading_tip_x {
            let back_x = if is_right {
                tip_x + ARROW_SIZE
            } else {
                tip_x - ARROW_SIZE
            };
            write!(
                self.buf,
                r##"<line style="stroke:{color};stroke-width:{thickness};" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                fmt_coord(tip_x),
                fmt_coord(back_x),
                fmt_coord(tip_y),
                fmt_coord(tip_y - ARROW_HALF_H),
            )
            .unwrap();
            write!(
                self.buf,
                r##"<line style="stroke:{color};stroke-width:{thickness};" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                fmt_coord(tip_x),
                fmt_coord(back_x),
                fmt_coord(tip_y),
                fmt_coord(tip_y + ARROW_HALF_H),
            )
            .unwrap();
        }

        // Open arrow: two lines forming a "V" shape
        let (back_x, up_y, down_y) = if is_right {
            (
                tip_x - ARROW_SIZE,
                tip_y - ARROW_HALF_H,
                tip_y + ARROW_HALF_H,
            )
        } else {
            (
                tip_x + ARROW_SIZE,
                tip_y - ARROW_HALF_H,
                tip_y + ARROW_HALF_H,
            )
        };

        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:{thickness};" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(tip_x),
            fmt_coord(back_x),
            fmt_coord(tip_y),
            fmt_coord(up_y),
        )
        .unwrap();

        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:{thickness};" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(tip_x),
            fmt_coord(back_x),
            fmt_coord(tip_y),
            fmt_coord(down_y),
        )
        .unwrap();

        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:{thickness};{line_style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(line_x1),
            fmt_coord(line_x2),
            fmt_coord(line_y),
            fmt_coord(line_y),
        )
        .unwrap();

        let label_x = if let Some((num_text, num_w, style)) = autonumber {
            emit_autonumber_prefix(&mut self.buf, num_text, text_x, text_y, style);
            text_x + num_w + AUTONUMBER_LABEL_GAP
        } else {
            text_x
        };

        if !text_content.is_empty() {
            let _ = text_len;
            self.emit_message_label(label_x, text_y, text_content, color);
        }

        self.message_group_close();
    }

    fn close_svg(&mut self, encoded_src: &str) {
        if !encoded_src.is_empty() {
            write!(self.buf, "<?plantuml-src {encoded_src}?>").unwrap();
        }
        self.buf.push_str("</g></svg>");
    }

    fn into_string(self) -> String {
        self.buf
    }
}

// ---------------------------------------------------------------------------
// Main render function
// ---------------------------------------------------------------------------

/// Render the PlantUML empty-diagram welcome screen.
fn render_empty_welcome() -> String {
    // Delegate to the old SvgBuilder-based renderer for the welcome screen.
    // The welcome screen doesn't need to match PlantUML exactly.
    let w = 480.0_f64;
    let h = 260.0_f64;
    let mut svg = crate::svg::SvgBuilder::new(w, h);
    let x = 10.0;
    let lh = 14.0;
    let mut y = 20.0;

    let welcome = format!("Welcome to {}!", crate::product_name());
    let info = format!(
        "You will find more information about {} syntax on",
        crate::product_name(),
    );
    let lines: &[&str] = &[
        &welcome,
        "\u{00a0}",
        "You can start with a simple UML Diagram like:",
        "\u{00a0}",
        "Bob->Alice:\u{00a0}Hello",
        "\u{00a0}",
        "Or",
        "\u{00a0}",
        "class\u{00a0}Example",
        "\u{00a0}",
        &info,
        crate::product_url(),
        "\u{00a0}",
        "(Details by typing",
        "license",
        "keyword)",
    ];
    for line in lines {
        svg.text(x, y, line, "start", 11.0);
        y += lh;
    }
    svg.finalize()
}

/// Dispatch participant shape rendering based on kind.
#[allow(clippy::too_many_arguments)]
fn render_participant_shape(
    svg: &mut PlantUmlSvg,
    part_uid: &str,
    qualified_name: &str,
    source_line: u32,
    position: &str,
    p: &ParticipantLayout,
    base_y: f64,
    _max_box_h: f64,
    fill_color: &str,
    border_color: &str,
    participant_inner_pad: f64,
    queue_head_offset: f64,
    draw_control_glyph: bool,
) {
    // Make the participant's link (if any) available to the group open/close
    // helpers so the shape contents get wrapped in a link anchor.
    svg.active_participant_url = p.url.clone();
    match p.kind {
        ParticipantKind::Actor
        | ParticipantKind::Boundary
        | ParticipantKind::Control
        | ParticipantKind::Entity
        | ParticipantKind::Database => {
            // All non-box shapes: text centered at center_x - (text_width + 2*pad)/2.
            let text_x = p.center_x - (p.text_width + 2.0 * ACTOR_TEXT_PAD) / 2.0;
            match p.kind {
                ParticipantKind::Actor => {
                    svg.actor_shape(
                        part_uid,
                        qualified_name,
                        source_line,
                        position,
                        p.center_x,
                        base_y,
                        text_x,
                        &p.label,
                        p.text_width,
                        fill_color,
                        border_color,
                    );
                }
                ParticipantKind::Boundary => {
                    svg.boundary_shape(
                        part_uid,
                        qualified_name,
                        source_line,
                        position,
                        p.center_x,
                        base_y,
                        text_x,
                        &p.label,
                        p.text_width,
                        fill_color,
                        border_color,
                    );
                }
                ParticipantKind::Control => {
                    svg.control_shape(
                        part_uid,
                        qualified_name,
                        source_line,
                        position,
                        p.center_x,
                        base_y,
                        text_x,
                        &p.label,
                        p.text_width,
                        fill_color,
                        border_color,
                        draw_control_glyph,
                    );
                }
                ParticipantKind::Entity => {
                    svg.entity_shape(
                        part_uid,
                        qualified_name,
                        source_line,
                        position,
                        p.center_x,
                        base_y,
                        text_x,
                        &p.label,
                        p.text_width,
                        fill_color,
                        border_color,
                    );
                }
                ParticipantKind::Database => {
                    svg.database_shape(
                        part_uid,
                        qualified_name,
                        source_line,
                        position,
                        p.center_x,
                        base_y,
                        text_x,
                        &p.label,
                        p.text_width,
                        fill_color,
                        border_color,
                    );
                }
                _ => unreachable!(),
            }
        }
        ParticipantKind::Collections => {
            let text_x = p.box_x + participant_inner_pad;
            let text_y = base_y + COLLECTIONS_OFFSET + BOX_TEXT_Y_OFFSET;
            svg.collections_shape(
                part_uid,
                qualified_name,
                source_line,
                position,
                p.box_x,
                base_y,
                p.box_width,
                text_x,
                text_y,
                &p.label,
                p.text_width,
                fill_color,
                border_color,
            );
        }
        ParticipantKind::Queue => {
            let text_x = p.box_x + QUEUE_TEXT_X_PAD;
            // Queue pill is shorter and offset; text baseline tracks the pill mid.
            let pill_top = if position == "tail" {
                base_y
            } else {
                base_y + queue_head_offset
            };
            let text_y = pill_top + (HEAD_BOX_H - 4.0) / 2.0 + 5.29102;
            svg.queue_shape(
                part_uid,
                qualified_name,
                source_line,
                position,
                p.box_x,
                base_y,
                p.box_width,
                text_x,
                text_y,
                &p.label,
                p.text_width,
                fill_color,
                border_color,
                queue_head_offset,
            );
        }
        ParticipantKind::Participant => {
            let text_x = p.box_x + participant_inner_pad;
            let text_y = base_y
                + p.text_y_offset
                + if p.stereotype.is_some() {
                    atom_height_with_family(
                        svg.participant_font_size as f64,
                        &svg.participant_font_family,
                    )
                } else {
                    0.0
                };
            let stereo_ref = p
                .stereotype
                .as_ref()
                .map(|s| (s.as_str(), p.stereotype_width));
            svg.participant_box(
                part_uid,
                qualified_name,
                source_line,
                position,
                p.box_x,
                base_y,
                p.box_width,
                p.box_height,
                text_x,
                text_y,
                &p.label,
                p.text_width,
                stereo_ref,
                fill_color,
            );
        }
    }
    svg.active_participant_url = None;
}

/// Render a sequence diagram with an optional oracle layout.
///
/// When the oracle's `root_g_inner_xml` is populated, the renderer replays
/// the body verbatim inside the PlantUML envelope. Otherwise it falls back
/// to the geometry-driven renderer below.
pub fn render_with_oracle(
    diagram: &SequenceDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "SEQUENCE");
    }
    render(diagram, theme, oracle)
}

/// Render a sequence diagram to SVG matching PlantUML's exact output.
///
/// `oracle` is consumed only for content that can't be reconstructed from the
/// source — currently the captured `<defs>` (gradient/filter definitions whose
/// ids are PlantUML hashes); the layout itself is computed from scratch.
pub fn render(diagram: &SequenceDiagram, _theme: &Theme, oracle: Option<&OracleLayout>) -> String {
    // Per-diagram skinparam overrides relevant to sequence arrow rendering.
    // These are read directly from the parser's skinparam list (rather than
    // the cascading `Theme`) so any value-less default tracks PlantUML's
    // historical colours rather than the workspace `slate` theme.
    let mut default_arrow_color = "#181818".to_string();
    let mut default_arrow_thickness: String = "1".to_string();
    let mut message_font_color = "#000000".to_string();
    let mut message_font_color_set = false;
    let mut message_font_family = "sans-serif".to_string();
    let mut message_font_size: u32 = MSG_FONT_SIZE as u32;
    let mut arrow_font_size_set = false;
    let mut message_font_bold = false;
    let mut message_font_italic = false;
    let mut participant_fill = "#E2E2F0".to_string();
    let mut participant_border = "#181818".to_string();
    let mut participant_border_thickness: String = "0.5".to_string();
    let mut participant_font_color = "#000000".to_string();
    let mut participant_font_color_set = false;
    let mut participant_font_family = "sans-serif".to_string();
    let mut participant_font_size: u32 = PARTICIPANT_FONT_SIZE as u32;
    let mut participant_font_bold = false;
    let mut participant_font_italic = false;
    let mut global_padding = 0.0;
    let mut theme_loaded = false;
    let mut participant_padding = 0.0;
    let mut participant_outer_padding_base: Option<f64> = None;
    let mut lifeline_background = "#FFFFFF".to_string();
    let mut lifeline_border = "#181818".to_string();
    let mut lifeline_border_thickness: String = "0.5".to_string();
    // Per-participant-kind background overrides. Each defaults to
    // `participant_fill`; the relevant `<kind>BackgroundColor` skinparam
    // (with or without the `sequence` prefix) sets it.
    let mut actor_fill_override: Option<String> = None;
    let mut actor_border_override: Option<String> = None;
    let mut boundary_fill_override: Option<String> = None;
    let mut boundary_border_override: Option<String> = None;
    let mut control_fill_override: Option<String> = None;
    let mut control_border_override: Option<String> = None;
    let mut entity_fill_override: Option<String> = None;
    let mut entity_border_override: Option<String> = None;
    let mut database_fill_override: Option<String> = None;
    let mut database_border_override: Option<String> = None;
    let mut collections_fill_override: Option<String> = None;
    let mut collections_border_override: Option<String> = None;
    let mut queue_fill_override: Option<String> = None;
    let mut queue_border_override: Option<String> = None;
    // Note fill/border overrides via `skinparam noteBackgroundColor` /
    // `noteBorderColor`. Default fill #FEFFDD, default border #181818.
    let mut note_fill_override: Option<String> = None;
    let mut note_border_override: Option<String> = None;
    let mut note_font_color = "#000000".to_string();
    let mut note_font_color_set = false;
    let mut note_font_family = "sans-serif".to_string();
    let mut note_font_size: u32 = MSG_FONT_SIZE as u32;
    let mut note_border_thickness = "0.5".to_string();
    let mut page_font_family = "sans-serif".to_string();
    let mut note_shadow_filter: Option<String> = None;
    let mut participant_shadow_filter: Option<String> = None;
    let mut sequence_shadowing = false;
    // Whether `ParticipantBackgroundColor` / `ParticipantBorderColor` were set
    // explicitly. These only affect the plain `participant` rectangle, so other
    // shape kinds must fall back to the (monochrome-aware) historical default
    // rather than the participant override.
    let mut participant_fill_set = false;
    let mut participant_border_set = false;
    // Canvas background. PlantUML only emits a full-canvas `<rect>` (and a
    // non-`#FFFFFF` `style="...background:...;"`) when `backgroundColor` is set
    // to a non-default value.
    let mut bg_color: Option<String> = None;
    // Divider (`== ... ==`) styling overrides. Defaults: #EEEEEE fill,
    // #000000 border and font, 13px font.
    let mut divider_fill = "#EEEEEE".to_string();
    let mut divider_border = "#000000".to_string();
    let mut divider_font_color = "#000000".to_string();
    let mut divider_font_family = "sans-serif".to_string();
    let mut divider_font_size: u32 = MSG_FONT_SIZE as u32;
    let mut group_background = "#EEEEEE".to_string();
    let mut group_header_font_family = "sans-serif".to_string();
    let mut group_header_font_size: u32 = MSG_FONT_SIZE as u32;
    // Message label horizontal alignment on the arrow span. PlantUML's
    // `skinparam SequenceMessageAlign` accepts left (default) | center | right.
    let mut message_align = MessageAlign::Left;
    let mut note_text_align = MessageAlign::Left;
    // Participant head/tail box corner radius. `skinparam RoundCorner N` sets
    // the box rx/ry to N/2 (default 2.5 = RoundCorner 5 / 2).
    let mut head_box_rx = HEAD_BOX_RX;
    let mut note_corner_radius = 0.0;
    // Gradient (`#c1/c2`) backgrounds reference captured `<linearGradient>`
    // ids by their stop colours. The defs themselves are spliced into `<defs>`
    // by `open_svg`; the renderer only needs the matching id.
    let gradient_defs = oracle.map(|o| o.defs_inner_xml.as_str());
    let filter_id: Option<String> = oracle
        .map(|o| o.defs_inner_xml.as_str())
        .and_then(parse_filter_id);
    for sp in &diagram.meta.skinparams {
        let key = sp.key.to_ascii_lowercase();
        let val = sp.value.trim();
        if val.is_empty() {
            continue;
        }
        match key.as_str() {
            "__theme" => {
                theme_loaded = true;
            }
            "defaultfontsize" => {
                if let Ok(v) = val.parse::<u32>() {
                    message_font_size = v;
                    participant_font_size = v;
                    note_font_size = v;
                    divider_font_size = v;
                    group_header_font_size = v;
                }
            }
            "__stylerootlinethickness" => {
                if let Ok(v) = val.parse::<f64>() {
                    let thickness = plantuml_metrics::fmt_coord(v);
                    participant_border_thickness = thickness.clone();
                    lifeline_border_thickness = thickness;
                }
            }
            "defaultfontname" => {
                let family = canonical_font_family(val);
                message_font_family = family.clone();
                participant_font_family = family.clone();
                note_font_family = family.clone();
                divider_font_family = family.clone();
                group_header_font_family = family.clone();
                page_font_family = family;
            }
            "sequencefontname" => {
                let family = canonical_font_family(val);
                message_font_family = family.clone();
                participant_font_family = family.clone();
                note_font_family = family.clone();
                divider_font_family = family.clone();
                group_header_font_family = family.clone();
                page_font_family = family;
            }
            "defaultfontcolor" => {
                let c = resolve_color(val);
                if !message_font_color_set {
                    message_font_color = c.clone();
                }
                if !participant_font_color_set {
                    participant_font_color = c.clone();
                }
                if !note_font_color_set {
                    note_font_color = c;
                }
            }
            "backgroundcolor" => {
                if val.eq_ignore_ascii_case("transparent") {
                    bg_color = Some("transparent".to_string());
                } else {
                    let c = gradient_fill_or(val, gradient_defs);
                    if c != "#FFFFFF" {
                        bg_color = Some(c);
                    }
                }
            }
            "arrowcolor" | "sequencearrowcolor" | "classarrowcolor" => {
                default_arrow_color = resolve_color(val);
            }
            "arrowthickness" | "sequencearrowthickness" => {
                if let Ok(v) = val.parse::<f64>() {
                    default_arrow_thickness = plantuml_metrics::fmt_coord(v);
                }
            }
            "arrowfontcolor" | "sequencearrowfontcolor" => {
                message_font_color = resolve_color(val);
                message_font_color_set = true;
            }
            "arrowfontname" | "sequencearrowfontname" => {
                message_font_family = canonical_font_family(val);
            }
            "arrowfontsize" | "sequencearrowfontsize" => {
                if let Ok(v) = val.parse::<u32>() {
                    message_font_size = v;
                    arrow_font_size_set = true;
                }
            }
            "arrowfontstyle" | "sequencearrowfontstyle" => {
                let style = val.to_ascii_lowercase();
                message_font_bold = style.contains("bold");
                message_font_italic = style.contains("italic");
            }
            "defaulttextalignment" => {
                let align = match val.to_ascii_lowercase().as_str() {
                    "center" => MessageAlign::Center,
                    "right" => MessageAlign::Right,
                    _ => MessageAlign::Left,
                };
                message_align = align;
                note_text_align = align;
            }
            "participantbackgroundcolor" | "sequenceparticipantbackgroundcolor" => {
                participant_fill = gradient_fill_or(val, gradient_defs);
                participant_fill_set = true;
            }
            "participantbordercolor" | "sequenceparticipantbordercolor" => {
                participant_border = resolve_color(val);
                participant_border_set = true;
            }
            "participantborderthickness" | "sequenceparticipantborderthickness" => {
                if let Ok(v) = val.parse::<f64>() {
                    participant_border_thickness = plantuml_metrics::fmt_coord(v);
                }
            }
            "participantfontcolor" | "sequenceparticipantfontcolor" => {
                participant_font_color = resolve_color(val);
                participant_font_color_set = true;
            }
            "participantfontname" | "sequenceparticipantfontname" => {
                participant_font_family = canonical_font_family(val);
            }
            "participantfontsize" | "sequenceparticipantfontsize" => {
                if let Ok(v) = val.parse::<u32>() {
                    participant_font_size = v;
                }
            }
            "participantfontstyle" | "sequenceparticipantfontstyle" => {
                let style = val.to_ascii_lowercase();
                participant_font_bold = style.contains("bold");
                participant_font_italic = style.contains("italic");
            }
            "participantpadding" => {
                if let Ok(v) = val.parse::<f64>() {
                    participant_padding = v;
                    participant_outer_padding_base = Some(v);
                }
            }
            "sequenceparticipantpadding" => {
                if let Ok(v) = val.parse::<f64>() {
                    participant_padding = v;
                }
            }
            "padding" | "sequencepadding" => {
                if let Ok(v) = val.parse::<f64>() {
                    global_padding = v;
                }
            }
            "sequencelifelinebordercolor" => {
                lifeline_border = resolve_color(val);
            }
            "sequencelifelinebackgroundcolor" => {
                lifeline_background = resolve_color(val);
            }
            "sequencelifelineborderthickness" => {
                // PlantUML honours the lifeline border *colour* but not this
                // *thickness*: every golden draws the dashed lifeline at the
                // default 0.5 regardless of the value. Accept the key without
                // effect so it doesn't fall through to unknown-skinparam paths.
            }
            "actorbackgroundcolor" | "sequenceactorbackgroundcolor" => {
                actor_fill_override = Some(resolve_color(val));
            }
            "actorbordercolor" | "sequenceactorbordercolor" => {
                actor_border_override = Some(resolve_color(val));
            }
            "boundarybackgroundcolor" | "sequenceboundarybackgroundcolor" => {
                boundary_fill_override = Some(resolve_color(val));
            }
            "boundarybordercolor" | "sequenceboundarybordercolor" => {
                boundary_border_override = Some(resolve_color(val));
            }
            "controlbackgroundcolor" | "sequencecontrolbackgroundcolor" => {
                control_fill_override = Some(resolve_color(val));
            }
            "controlbordercolor" | "sequencecontrolbordercolor" => {
                control_border_override = Some(resolve_color(val));
            }
            "entitybackgroundcolor" | "sequenceentitybackgroundcolor" => {
                entity_fill_override = Some(resolve_color(val));
            }
            "entitybordercolor" | "sequenceentitybordercolor" => {
                entity_border_override = Some(resolve_color(val));
            }
            "databasebackgroundcolor" | "sequencedatabasebackgroundcolor" => {
                database_fill_override = Some(resolve_color(val));
            }
            "databasebordercolor" | "sequencedatabasebordercolor" => {
                database_border_override = Some(resolve_color(val));
            }
            "collectionsbackgroundcolor" | "sequencecollectionsbackgroundcolor" => {
                collections_fill_override = Some(resolve_color(val));
            }
            "collectionsbordercolor" | "sequencecollectionsbordercolor" => {
                collections_border_override = Some(resolve_color(val));
            }
            "queuebackgroundcolor" | "sequencequeuebackgroundcolor" => {
                queue_fill_override = Some(resolve_color(val));
            }
            "queuebordercolor" | "sequencequeuebordercolor" => {
                queue_border_override = Some(resolve_color(val));
            }
            "notebackgroundcolor" | "sequencenotebackgroundcolor" => {
                note_fill_override = Some(gradient_fill_or(val, gradient_defs));
            }
            "notebordercolor" | "sequencenotebordercolor" => {
                note_border_override = Some(resolve_color(val));
            }
            "noteborderthickness" | "sequencenoteborderthickness" => {
                if let Ok(v) = val.parse::<f64>() {
                    note_border_thickness = plantuml_metrics::fmt_coord(v);
                }
            }
            "notefontcolor" | "sequencenotefontcolor" => {
                note_font_color = resolve_color(val);
                note_font_color_set = true;
            }
            "notefontname" | "sequencenotefontname" => {
                note_font_family = canonical_font_family(val);
            }
            "notefontsize" | "sequencenotefontsize" => {
                if let Ok(v) = val.parse::<u32>() {
                    note_font_size = v;
                }
            }
            "noteshadowing" | "sequencenoteshadowing" => {
                if val.eq_ignore_ascii_case("true") {
                    note_shadow_filter = filter_id.clone();
                } else if val.eq_ignore_ascii_case("false") {
                    note_shadow_filter = None;
                }
            }
            "shadowing" | "sequenceshadowing" => {
                if val.eq_ignore_ascii_case("true") {
                    sequence_shadowing = true;
                    participant_shadow_filter = filter_id.clone();
                    note_shadow_filter = filter_id.clone();
                } else if val.eq_ignore_ascii_case("false") {
                    sequence_shadowing = false;
                    participant_shadow_filter = None;
                    note_shadow_filter = None;
                }
            }
            "sequencedividerbackgroundcolor" => {
                divider_fill = resolve_color(val);
            }
            "sequencedividerbordercolor" => {
                divider_border = resolve_color(val);
            }
            "sequencedividerfontcolor" => {
                divider_font_color = resolve_color(val);
            }
            "sequencedividerfontname" => {
                divider_font_family = canonical_font_family(val);
            }
            "sequencedividerfontsize" => {
                if let Ok(v) = val.parse::<u32>() {
                    divider_font_size = v;
                }
            }
            "sequencegroupbackgroundcolor" => {
                group_background = gradient_fill_or(val, gradient_defs);
            }
            "sequencegroupheaderfontsize" => {
                if let Ok(v) = val.parse::<u32>() {
                    group_header_font_size = v;
                }
            }
            "sequencegroupheaderfontname" => {
                group_header_font_family = canonical_font_family(val);
            }
            "sequencemessagealign" => {
                message_align = match val.to_ascii_lowercase().as_str() {
                    "center" => MessageAlign::Center,
                    "right" => MessageAlign::Right,
                    _ => MessageAlign::Left,
                };
            }
            "roundcorner" => {
                if let Ok(v) = val.parse::<f64>() {
                    head_box_rx = v / 2.0;
                    note_corner_radius = v / 2.0;
                }
            }
            _ => {}
        }
    }
    // `skinparam monochrome true` desaturates the palette. The only colour the
    // sequence renderer normally emits beyond the already-monochrome #181818
    // stroke is the participant/shape background, which becomes #E3E3E3.
    let monochrome = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.eq_ignore_ascii_case("monochrome") && sp.value.trim().eq_ignore_ascii_case("true")
    });
    if monochrome && participant_fill == "#E2E2F0" {
        participant_fill = "#E3E3E3".to_string();
    }
    // Default fill/border for non-`participant` shape kinds (actor, boundary,
    // ...). These ignore `ParticipantBackgroundColor`/`ParticipantBorderColor`
    // but still honour monochrome. When the participant override was NOT set,
    // `participant_fill`/`participant_border` already hold the correct default.
    let nonparticipant_fill_default = if participant_fill_set {
        if monochrome { "#E3E3E3" } else { "#E2E2F0" }.to_string()
    } else {
        participant_fill.clone()
    };
    let nonparticipant_border_default = if participant_border_set {
        "#181818".to_string()
    } else {
        participant_border.clone()
    };
    let default_arrow_color = default_arrow_color.as_str();
    let default_arrow_thickness = default_arrow_thickness.as_str();
    let message_font_size_f = message_font_size as f64;
    let message_text_height = atom_height_with_family(message_font_size_f, &message_font_family);
    let group_header_font_size_f = group_header_font_size as f64;
    let group_header_height =
        text_height_with_family(group_header_font_size_f, &group_header_font_family) + 2.0;
    let group_inner_top_pad = group_header_height - GROUP_HEADER_INNER_PAD_DROP;
    let group_inner_top_pad_first = group_inner_top_pad - GROUP_HEADER_FIRST_PAD_ADJUST;
    let group_header_text_baseline =
        ascent_with_family(group_header_font_size_f, &group_header_font_family)
            + GROUP_HEADER_TEXT_TOP_PAD;
    // Bundled themes with `shadowing false` keep the regular 5px diagram top
    // margin and do not add global `Padding` to the leading participant
    // reservation. Shadowed themes reserve that padding for the shadow.
    let first_event_source_line = diagram
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Message(msg) => Some(msg.source_line),
            Event::Note(note) => Some(note.source_line),
            Event::Ref(r) => Some(r.source_line),
            Event::Return(ret) => Some(ret.source_line),
            Event::GroupStart(group) => Some(group.source_line),
            Event::GroupElse(group) => Some(group.source_line),
            Event::Divider(_)
            | Event::Delay(_)
            | Event::Space(_)
            | Event::GroupEnd
            | Event::NoteOnLink(_)
            | Event::Activate(_, _)
            | Event::Deactivate(_)
            | Event::Destroy(_)
            | Event::Create(_)
            | Event::Autonumber(_)
            | Event::NewPage(_) => None,
        })
        .min()
        .unwrap_or(usize::MAX);
    let has_leading_participant_declaration = diagram
        .participants
        .iter()
        .any(|p| p.source_line > 0 && p.source_line < first_event_source_line);
    let participant_border_thickness_value = participant_border_thickness
        .parse::<f64>()
        .unwrap_or(DEFAULT_PARTICIPANT_BORDER_THICKNESS);
    let explicit_nonshadowed_theme_head =
        theme_loaded && !sequence_shadowing && has_leading_participant_declaration;
    let has_theme_padding = global_padding > 0.0;
    let compact_nonshadowed_theme_head = explicit_nonshadowed_theme_head
        && (!has_theme_padding
            || participant_border_thickness_value <= DEFAULT_PARTICIPANT_BORDER_THICKNESS);
    let theme_margin_padding = if explicit_nonshadowed_theme_head && !compact_nonshadowed_theme_head
    {
        HEAD_BOX_Y
    } else {
        global_padding
    };
    let participant_outer_padding = participant_outer_padding_base
        .map(|v| {
            if compact_nonshadowed_theme_head {
                v
            } else if theme_loaded && sequence_shadowing {
                v + global_padding
            } else if explicit_nonshadowed_theme_head {
                v + 2.0 * theme_margin_padding
            } else {
                v + 2.0 * global_padding
            }
        })
        .unwrap_or(0.0);
    let explicit_global_padding = if theme_loaded { 0.0 } else { global_padding };
    let constraint_global_padding = if explicit_nonshadowed_theme_head {
        global_padding
    } else {
        explicit_global_padding
    };
    let note_global_padding = if theme_loaded && !sequence_shadowing {
        global_padding
    } else {
        explicit_global_padding
    };
    let theme_top_padding = if theme_loaded && sequence_shadowing {
        global_padding / 2.0
    } else if theme_loaded && !compact_nonshadowed_theme_head {
        theme_margin_padding
    } else {
        0.0
    };
    let has_deprecated_handwritten = has_deprecated_handwritten_skinparam(&diagram.meta.skinparams);
    let is_handwritten = is_handwritten_enabled(&diagram.meta.skinparams);
    let handwritten_warning_band_h = if has_deprecated_handwritten {
        HANDWRITTEN_WARNING_BAND_H
    } else {
        0.0
    };
    let group_frame_margin = GROUP_FRAME_MARGIN + participant_padding;
    let participant_box_gap = 10.0
        + 2.0 * participant_padding
        + if compact_nonshadowed_theme_head {
            2.0 * global_padding
        } else {
            0.0
        };
    let message_label_width = |text: &str| {
        message_label_width_with_family(
            text,
            message_font_size_f,
            message_font_bold,
            &message_font_family,
        )
    };
    let note_font_size_f = note_font_size as f64;
    // Empty diagram with no title — render the PlantUML welcome screen.
    if diagram.participants.is_empty() && diagram.events.is_empty() && diagram.meta.title.is_none()
    {
        return render_empty_welcome();
    }

    // Title-band height: when the diagram has a `title ...` directive, PlantUML
    // reserves a band above the participant heads for the rendered title lines.
    // The fixed 10px top + 11px bottom padding stays constant, but a Creole
    // `<size:...>` / `<font:...>` run changes the line ascent and height.
    const TITLE_FONT_SIZE: u32 = 14;
    const HEADER_FONT_SIZE: u32 = 10;
    const LEGEND_FONT_SIZE: u32 = 14;
    const LEGEND_LEFT: f64 = 12.0;
    const LEGEND_PAD: f64 = 5.0;
    const LEGEND_TOP_GAP: f64 = 14.0;
    const LEGEND_BOTTOM_PAD: f64 = 18.0;
    const TITLE_TOP_PAD: f64 = 10.0; // gap from y=0 to first title baseline (minus ascent)
    const TITLE_BOTTOM_PAD: f64 = 11.0; // gap from last title descent line to head top
    let title_lines: Vec<&str> = diagram
        .meta
        .title
        .as_deref()
        .map(|t| t.split("\\n").collect())
        .unwrap_or_default();
    let header_lines: Vec<&str> = diagram
        .meta
        .header
        .as_deref()
        .map(|h| h.split("\\n").collect())
        .unwrap_or_default();
    let legend_lines: Vec<&str> = diagram
        .meta
        .legend
        .as_deref()
        .map(|l| l.lines().map(str::trim).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default();
    let title_line_metrics = title_lines
        .iter()
        .map(|line| {
            rendered_line_metrics_with_family(line, TITLE_FONT_SIZE as f64, &page_font_family)
        })
        .collect::<Vec<_>>();
    let title_band_h = if title_lines.is_empty() {
        0.0
    } else {
        TITLE_TOP_PAD
            + title_line_metrics
                .iter()
                .map(|metrics| metrics.height)
                .sum::<f64>()
            + TITLE_BOTTOM_PAD
    };
    // Named participant boxes add a title band above the participant heads.
    // Most titled boxes reserve text_height(13) + 5; titled boxes containing a
    // queue omit the extra 5px gap and let the queue's own pill offset clear
    // the label.
    let participant_boxes: Vec<_> = diagram
        .boxes
        .iter()
        .filter(|b| !b.members.is_empty())
        .collect();
    let has_boxes = !participant_boxes.is_empty();
    let any_box_titled = participant_boxes.iter().any(|b| !b.title.is_empty());
    let any_box_member_is_queue = participant_boxes.iter().any(|b| {
        b.members.iter().any(|&pi| {
            diagram
                .participants
                .get(pi)
                .is_some_and(|p| p.kind == ParticipantKind::Queue)
        })
    });
    let box_band_h = if !has_boxes {
        0.0
    } else if any_box_titled {
        let title_gap = if any_box_member_is_queue {
            0.0
        } else {
            BOX_TITLE_HEAD_GAP
        };
        plantuml_metrics::text_height(BOX_TITLE_FONT_SIZE as f64) + title_gap
    } else {
        // Untitled boxes only contribute their top margin, no title line.
        BOX_TITLE_HEAD_GAP
    };
    // A `header` directive reserves a band above the heads: the header text
    // sits at the top (baseline y≈14.668), with literal `\n` adding more
    // right-aligned header text lines. Lines are spaced by text_height(10);
    // the reserved band adds PlantUML's one-pixel clearance once.
    let header_line_step = text_height_with_family(HEADER_FONT_SIZE as f64, &page_font_family);
    let header_band_h = if !header_lines.is_empty() {
        header_lines.len() as f64 * header_line_step + 1.0
    } else {
        0.0
    };
    let legend_line_metrics = legend_lines
        .iter()
        .map(|line| {
            rendered_line_metrics_with_family(line, LEGEND_FONT_SIZE as f64, &page_font_family)
        })
        .collect::<Vec<_>>();
    let legend_text_w = legend_lines
        .iter()
        .map(|line| text_width_with_family(line, LEGEND_FONT_SIZE as f64, &page_font_family))
        .fold(0.0_f64, f64::max);
    let legend_box_w = if legend_lines.is_empty() {
        0.0
    } else {
        legend_text_w + 2.0 * LEGEND_PAD
    };
    let legend_box_h = if legend_lines.is_empty() {
        0.0
    } else {
        legend_line_metrics
            .iter()
            .map(|metrics| metrics.height)
            .sum::<f64>()
            + 2.0 * LEGEND_PAD
    };
    let participant_inner_pad = BOX_TEXT_X_PAD + global_padding;
    let teoz_top_pad = if diagram.teoz { 5.0 } else { 0.0 };
    let head_box_y = HEAD_BOX_Y
        + handwritten_warning_band_h
        + teoz_top_pad
        + theme_top_padding
        + title_band_h
        + box_band_h
        + header_band_h;
    let participant_font_size_f = participant_font_size as f64;
    let participant_line_h =
        atom_height_with_family(participant_font_size_f, &participant_font_family);
    let participant_box_h = participant_line_h + 14.0 + 2.0 * global_padding;
    let participant_text_y_offset =
        ascent_with_family(participant_font_size_f, &participant_font_family)
            + 7.0
            + global_padding;
    let note_content_width_padded = |max_text_w: f64, shape: NoteShape, align: MessageAlign| {
        aligned_note_content_width(max_text_w, shape, align) + 2.0 * note_global_padding
    };
    let note_content_width_raw_padded = |max_text_w: f64, shape: NoteShape, align: MessageAlign| {
        aligned_note_content_width_raw(max_text_w, shape, align) + 2.0 * note_global_padding
    };
    let over_several_shape_position_width_raw_padded =
        |max_text_w: f64, shape: NoteShape, align: MessageAlign, min_width: f64, centre: f64| {
            let raw = note_content_width_raw_padded(max_text_w, shape, align).max(min_width);
            let raw_left = (centre - raw / 2.0).floor();
            if matches!(shape, NoteShape::Hexagonal | NoteShape::Rectangular)
                && raw_left < HEAD_BOX_Y
            {
                raw - 1.0
            } else {
                raw
            }
        };
    let note_rendered_height_padded = |shape: NoteShape, metrics: &NoteTextMetrics| {
        note_rendered_height(shape, metrics) + 2.0 * note_global_padding
    };

    // -----------------------------------------------------------------------
    // Phase 1: Compute participant layouts
    // -----------------------------------------------------------------------

    let all_participants_are_queues = !diagram.participants.is_empty()
        && diagram
            .participants
            .iter()
            .all(|p| p.kind == ParticipantKind::Queue);
    let queue_head_offset = if all_participants_are_queues {
        0.0
    } else {
        5.0
    };
    let queue_layout_h = if all_participants_are_queues {
        HEAD_BOX_H - 5.0
    } else {
        HEAD_BOX_H
    };

    let mut participants: Vec<ParticipantLayout> = diagram
        .participants
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let st = p.stereotype.clone();
            let st_display = st.as_ref().map(|s| format!("\u{ab}{s}\u{bb}"));
            let st_w = st_display
                .as_ref()
                .map(|s| {
                    text_width_with_family(s, participant_font_size_f, &participant_font_family)
                })
                .unwrap_or(0.0);
            let label = p.label.clone();
            let tw = if participant_font_bold {
                bold_text_width_with_family(
                    &label,
                    participant_font_size_f,
                    &participant_font_family,
                )
            } else {
                text_width_with_family(&label, participant_font_size_f, &participant_font_family)
            };
            // Box width must accommodate the display label (and stereotype if separate)
            let max_text_w = tw.max(st_w);

            // Compute kind-specific box width and height.
            let (bw, bh) = match p.kind {
                ParticipantKind::Actor => {
                    // Actor: width = max(arm_span, text_width) + 2*padding
                    let w = (max_text_w + 2.0 * ACTOR_TEXT_PAD).max(ACTOR_ARM_SPAN + 1.0);
                    let h = HEAD_BOX_H + ACTOR_EXTRA_H;
                    (w, h)
                }
                ParticipantKind::Boundary => {
                    // Boundary: vertical line at left, horizontal connector to a
                    // circle r=12. PlantUML stickman width = 2*r + left + 2*margin
                    // (24 + 17 + 8 = 49). Overall width = max(stickman, text+2*pad).
                    let shape_w = 2.0 * STEREOTYPE_CIRCLE_R
                        + BOUNDARY_LINE_TO_CIRCLE_GAP
                        + 2.0 * STEREOTYPE_CIRCLE_MARGIN;
                    let w = (max_text_w + 2.0 * ACTOR_TEXT_PAD).max(shape_w);
                    let h = HEAD_BOX_H + CIRCLE_SHAPE_EXTRA_H;
                    (w, h)
                }
                ParticipantKind::Control | ParticipantKind::Entity => {
                    // Control/Entity: circle r=12 with 4px margin all round.
                    // PlantUML width = max(stickman = 2*r + 2*margin, text + 2*pad).
                    let shape_w = 2.0 * STEREOTYPE_CIRCLE_R + 2.0 * STEREOTYPE_CIRCLE_MARGIN;
                    let w = (max_text_w + 2.0 * ACTOR_TEXT_PAD).max(shape_w);
                    let h = HEAD_BOX_H + CIRCLE_SHAPE_EXTRA_H;
                    (w, h)
                }
                ParticipantKind::Database => {
                    // Database: cylinder, text below.
                    let w = (max_text_w + 2.0 * ACTOR_TEXT_PAD).max(DB_CYLINDER_WIDTH);
                    let h = HEAD_BOX_H + DB_EXTRA_H;
                    (w, h)
                }
                ParticipantKind::Collections => {
                    // Collections: two stacked rectangles offset by COLLECTIONS_OFFSET.
                    // The layout width must include the stacking offset so the
                    // lifeline centres on the full visual span.
                    let w = max_text_w + 2.0 * participant_inner_pad + COLLECTIONS_OFFSET;
                    let h = HEAD_BOX_H + COLLECTIONS_EXTRA_H;
                    (w, h)
                }
                ParticipantKind::Queue => {
                    // Queue: pill shape, width = text + 20 (caps + padding).
                    let w = max_text_w + QUEUE_TEXT_H_PAD;
                    let h = queue_layout_h;
                    (w, h)
                }
                ParticipantKind::Participant => {
                    let w = max_text_w + 2.0 * participant_inner_pad;
                    // Box height is taller for stereotyped participants.
                    let h = if st.is_some() {
                        participant_box_h + participant_line_h
                    } else {
                        participant_box_h
                    };
                    (w, h)
                }
            };

            ParticipantLayout {
                decl_idx: idx,
                layout_order: p.order.unwrap_or(idx),
                id: p.id.clone(),
                label,
                kind: p.kind,
                stereotype: st,
                text_width: tw,
                text_y_offset: participant_text_y_offset,
                stereotype_width: st_w,
                box_width: bw,
                box_height: bh,
                box_x: 0.0,
                center_x: 0.0,
                lifeline_line_x: 0.0,
                url: p.url.clone(),
                color: p.color.clone(),
                source_line: p.source_line as u32,
            }
        })
        .collect();
    participants.sort_by_key(|p| (p.layout_order, p.decl_idx));

    // Build a lookup from participant ID to visual layout index.
    let id_to_idx: HashMap<String, usize> = participants
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.clone(), i))
        .collect();

    // -----------------------------------------------------------------------
    // Phase 1.5: Pre-scan groups to determine participant shifts
    // -----------------------------------------------------------------------

    // For each group, identify which participant indices are referenced by
    // messages within the group. The group frame must encompass those participants.
    // If a group includes the leftmost participant, all participants must shift
    // right to make room for the group frame margin.

    let mut group_left_shift_depth = 0usize;
    let mut group_left_external_shift_depth = 0usize;
    {
        // Scan for groups and collect the participant index range for each group
        let mut group_stack: Vec<(usize, usize, bool)> = Vec::new(); // (min_idx, max_idx, has_external_left)
        for event in &diagram.events {
            match event {
                Event::GroupStart(_) => {
                    group_stack.push((usize::MAX, 0, false));
                }
                Event::GroupEnd => {
                    let closed_depth = group_stack.len();
                    if let Some((min_idx, max_idx, has_external_left)) = group_stack.pop() {
                        if min_idx == 0 {
                            group_left_shift_depth = group_left_shift_depth.max(closed_depth);
                            if has_external_left {
                                group_left_external_shift_depth =
                                    group_left_external_shift_depth.max(closed_depth);
                            }
                        }
                        if min_idx <= max_idx
                            && let Some(parent) = group_stack.last_mut()
                        {
                            parent.0 = parent.0.min(min_idx);
                            parent.1 = parent.1.max(max_idx);
                            parent.2 |= has_external_left;
                        }
                    }
                }
                Event::Message(msg) if !group_stack.is_empty() => {
                    let fi = id_to_idx.get(msg.from.as_str()).copied();
                    let ti = id_to_idx.get(msg.to.as_str()).copied();
                    if let Some(top) = group_stack.last_mut() {
                        if let Some(fi) = fi {
                            top.0 = top.0.min(fi);
                            top.1 = top.1.max(fi);
                        }
                        if let Some(ti) = ti {
                            top.0 = top.0.min(ti);
                            top.1 = top.1.max(ti);
                        }
                        if msg.from == "[" {
                            top.2 = true;
                        }
                    }
                }
                Event::Return(_) if !group_stack.is_empty() => {
                    // Returns also affect the group extent — but we don't know
                    // which participants they involve without tracking the return
                    // stack. For simplicity, assume they're between the same
                    // participants as their activation context.
                }
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 2: Compute required gap between adjacent participant pairs
    // -----------------------------------------------------------------------

    // PlantUML's spacing is constraint-based: for each message, the distance
    // between the source and target participant centers must accommodate:
    //   arrow_only_width + source_lifeline_shift + target_lifeline_shift
    // where lifeline shifts account for activation bars.
    //
    // arrow_only_width = text_width + MSG_TEXT_LEFT_PAD + MSG_TEXT_LEFT_PAD + ARROW_SIZE
    //                  = text_width + 24 (the constant overhead for arrow + text padding)

    let n = participants.len();
    let mut pair_max_label_width = vec![0.0_f64; n.saturating_sub(1)];

    // Multi-span constraints (left, right_exclusive, needed_total_width).
    // Applied as a post-pass: only widen pairs in span if cumulative existing
    // width is insufficient. Matches PlantUML's behavior where spanning
    // messages don't force intermediate pairs to widen if already covered.
    let mut multi_span_constraints: Vec<(usize, usize, f64)> = Vec::new();

    // Track maximum right extent of self-messages (for SVG width calculation).
    let mut max_self_msg_right: f64 = 0.0;
    let mut self_msg_right_note_left_by_event: HashMap<usize, f64> = HashMap::new();

    // Track activation state as we scan events to compute per-message shifts.
    let mut activation_depth: HashMap<String, usize> = HashMap::new();

    // Track return stack during spacing phase to infer return from/to.
    // Each entry: (returned-from participant, returned-to participant).
    let mut spacing_return_stack: Vec<(String, String)> = Vec::new();
    let mut spacing_last_return_pair: Option<(String, String)> = None;

    // Track autonumber state during spacing phase to compute bold label widths.
    // Driven entirely by `Event::Autonumber` directives in stream order.
    let mut spacing_auto = AutoState::default();

    // Pending `create X` ids awaiting their first message — that message reserves
    // extra horizontal space for the inline head box centered on X's lifeline.
    let mut spacing_pending_create: Vec<String> = Vec::new();

    for event in &diagram.events {
        match event {
            Event::Create(id) => spacing_pending_create.push(id.clone()),
            Event::Message(msg) => {
                let from_idx = id_to_idx.get(msg.from.as_str()).copied();
                let to_idx = id_to_idx.get(msg.to.as_str()).copied();
                if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
                    if fi == ti {
                        // Self-message. PlantUML's Step1Message reserves a constraint
                        // in the gap *after* the source participant (getConstraintAfter):
                        //   length = arrowOnlyWidth + segment.getLength()
                        // and arrowOnlyWidth = ComponentRoseSelfArrow.getPreferredWidth
                        //   = max(textWidth, 50)
                        //   + segment.getLength()       (added a second time inside it)
                        // so the gap to the next participant must be at least
                        //   max(label_w + 2*MARGIN, 50) + 2 * segment_length
                        // where segment_length = leftShift + rightShift of the lifeline
                        // at this message level (0 when inactive, ACTIVATION_WIDTH at
                        // depth 1). This pushes a right-hand neighbour further right.
                        // (When fi is the rightmost participant there is no neighbour;
                        // the right extent then feeds the canvas edge after Phase 3.)
                        if fi + 1 < n {
                            let label = process_label(&msg.label);
                            let label_w = message_label_width(&label);

                            let autonumber_extra = if let Some((_, w, _)) = spacing_auto.current() {
                                w + AUTONUMBER_LABEL_GAP
                            } else {
                                0.0
                            };

                            let text_pref =
                                autonumber_extra + label_w + MSG_TEXT_LEFT_PAD + MSG_TEXT_LEFT_PAD;
                            let arrow_only_w = text_pref.max(SELF_MSG_MIN_PREF_WIDTH);

                            // Lifeline segment length at this level: left shift
                            // (ACTIVATION_HALF_W when active) + right shift
                            // (depth * ACTIVATION_HALF_W). Counted twice, matching
                            // PlantUML's double-add via getPreferredWidth.
                            let depth = activation_depth
                                .get(msg.from.as_str())
                                .copied()
                                .unwrap_or(0)
                                + usize::from(matches!(
                                    msg.activation,
                                    Some(ActivationChange::Activate)
                                ));
                            let segment_len = if depth > 0 {
                                ACTIVATION_HALF_W + depth as f64 * ACTIVATION_HALF_W
                            } else {
                                0.0
                            };

                            let needed = arrow_only_w + 2.0 * segment_len;
                            pair_max_label_width[fi] = pair_max_label_width[fi].max(needed);
                        }
                    } else {
                        let label = process_label(&msg.label);
                        let label_w = message_label_width(&label);

                        // Autonumber adds bold-or-plain text + gap before the label
                        // depending on whether a format string is set.
                        let autonumber_extra = if let Some((_, w, _)) = spacing_auto.current() {
                            w + AUTONUMBER_LABEL_GAP
                        } else {
                            0.0
                        };

                        // Base arrow width (text + padding + arrow)
                        let arrow_only_w = autonumber_extra
                            + label_w
                            + MSG_TEXT_LEFT_PAD
                            + MSG_TEXT_LEFT_PAD
                            + ARROW_SIZE
                            + 2.0 * constraint_global_padding;

                        // Lifeline shifts from activation bars at the current message level.
                        // The source's "right shift" and target's "left shift" come from
                        // activation bars extending from the lifeline center.
                        let from_depth = activation_depth
                            .get(msg.from.as_str())
                            .copied()
                            .unwrap_or(0);
                        let to_depth = activation_depth.get(msg.to.as_str()).copied().unwrap_or(0);

                        // Each active lifeline extends ACTIVATION_HALF_W from center.
                        let source_shift = if from_depth > 0 {
                            ACTIVATION_HALF_W
                        } else {
                            0.0
                        };
                        let target_shift = if to_depth > 0
                            || (diagram.teoz
                                && from_depth == 0
                                && matches!(msg.activation, Some(ActivationChange::Activate)))
                        {
                            ACTIVATION_HALF_W
                        } else {
                            0.0
                        };

                        // A message that creates its target reserves extra space
                        // for the inline head box centered on the target lifeline:
                        // half the box width sits left of center.
                        let create_extra = if let Some(pos) =
                            spacing_pending_create.iter().position(|id| *id == msg.to)
                        {
                            spacing_pending_create.remove(pos);
                            participants[ti].box_width / 2.0
                        } else {
                            0.0
                        };

                        let needed = arrow_only_w + source_shift + target_shift + create_extra;

                        let (left, right) = if fi < ti { (fi, ti) } else { (ti, fi) };
                        if right - left == 1 {
                            pair_max_label_width[left] = pair_max_label_width[left].max(needed);
                        } else {
                            // Defer multi-span constraint to post-pass.
                            multi_span_constraints.push((left, right, needed));
                        }
                    }
                }

                // Update activation state from message's activation change
                spacing_last_return_pair = Some((msg.to.clone(), msg.from.clone()));
                if let Some(act) = &msg.activation {
                    match act {
                        ActivationChange::Activate => {
                            *activation_depth.entry(msg.to.clone()).or_default() += 1;
                            spacing_return_stack.push((msg.to.clone(), msg.from.clone()));
                        }
                        ActivationChange::Deactivate => {
                            if let Some(d) = activation_depth.get_mut(&msg.from) {
                                *d = d.saturating_sub(1);
                            }
                        }
                        ActivationChange::Destroy => {
                            if let Some(d) = activation_depth.get_mut(&msg.to) {
                                *d = d.saturating_sub(1);
                            }
                        }
                    }
                }

                // Advance autonumber during spacing phase
                spacing_auto.advance();
            }
            Event::Return(ret) => {
                // Return messages need spacing computation like regular messages.
                // Pop the activation return stack to find from/to; without an
                // activation context, Java replies from the previous message's
                // receiver to its sender.
                // NOTE: compute spacing BEFORE deactivating — the return sender
                // is still activated at the point the message arrow is drawn.
                let stack_entry = spacing_return_stack.pop();
                if let Some((ret_from, ret_to)) = stack_entry
                    .clone()
                    .or_else(|| spacing_last_return_pair.clone())
                {
                    let fi_opt = id_to_idx.get(ret_from.as_str()).copied();
                    let ti_opt = id_to_idx.get(ret_to.as_str()).copied();
                    if let (Some(fi), Some(ti)) = (fi_opt, ti_opt) {
                        let label = if ret.label.is_empty() {
                            String::new()
                        } else {
                            decode_backslash_escapes(&ret.label)
                        };
                        let label_w = message_label_width(&label);

                        // Autonumber adds bold-or-plain text + gap before the label
                        // depending on whether a format string is set.
                        let autonumber_extra = if let Some((_, w, _)) = spacing_auto.current() {
                            w + AUTONUMBER_LABEL_GAP
                        } else {
                            0.0
                        };

                        // Same spacing formula as forward messages
                        let arrow_only_w = autonumber_extra
                            + label_w
                            + MSG_TEXT_LEFT_PAD
                            + MSG_TEXT_LEFT_PAD
                            + ARROW_SIZE
                            + 2.0 * constraint_global_padding;

                        let from_depth = activation_depth
                            .get(ret_from.as_str())
                            .copied()
                            .unwrap_or(0);
                        let to_depth = activation_depth.get(ret_to.as_str()).copied().unwrap_or(0);
                        let source_shift = if from_depth > 0 {
                            ACTIVATION_HALF_W
                        } else {
                            0.0
                        };
                        let target_shift = if to_depth > 0 { ACTIVATION_HALF_W } else { 0.0 };

                        let needed = arrow_only_w + source_shift + target_shift;

                        let (left, right) = if fi < ti { (fi, ti) } else { (ti, fi) };
                        if right - left == 1 {
                            pair_max_label_width[left] = pair_max_label_width[left].max(needed);
                        } else {
                            // Defer multi-span constraint to post-pass.
                            multi_span_constraints.push((left, right, needed));
                        }
                    }

                    // Deactivate AFTER spacing computation, but only when the
                    // return consumed an activation context.
                    if stack_entry.is_some()
                        && let Some(d) = activation_depth.get_mut(&ret_from)
                    {
                        *d = d.saturating_sub(1);
                    }
                }

                // Advance autonumber
                spacing_auto.advance();
            }
            // Non-standard notes over several participants (OVER_SEVERAL). Java
            // lets standard folded notes overhang outside the spanned participant
            // area; hnote/rnote still need the older outside-pair reservation.
            Event::Note(note)
                if note.position == NotePosition::Over
                    && note.participants.len() >= 2
                    && note.shape != NoteShape::Note =>
            {
                let first_idx = id_to_idx
                    .get(note.participants.first().unwrap().as_str())
                    .copied();
                let last_idx = id_to_idx
                    .get(note.participants.last().unwrap().as_str())
                    .copied();
                if let (Some(fi), Some(li)) = (first_idx, last_idx) {
                    let max_tw = note_max_line_width_with_family(
                        &note.text,
                        note_font_size_f,
                        &note_font_family,
                    );
                    let note_content_w =
                        note_content_width_padded(max_tw, note.shape, note_text_align);
                    let half = note_content_w / 2.0;
                    let (lo, hi) = if fi < li { (fi, li) } else { (li, fi) };
                    // Gap before the first spanned participant.
                    if lo > 0 {
                        pair_max_label_width[lo - 1] = pair_max_label_width[lo - 1].max(half);
                    }
                    // Gap after the last spanned participant.
                    if hi < pair_max_label_width.len() {
                        pair_max_label_width[hi] = pair_max_label_width[hi].max(half);
                    }
                }
            }
            Event::Activate(id, _) => {
                *activation_depth.entry(id.clone()).or_default() += 1;
                if let Some((ret_from, ret_to)) = spacing_last_return_pair.clone()
                    && ret_from == *id
                {
                    spacing_return_stack.push((ret_from, ret_to));
                }
            }
            Event::Deactivate(id) => {
                if let Some(d) = activation_depth.get_mut(id) {
                    *d = d.saturating_sub(1);
                }
                if let Some(pos) = spacing_return_stack
                    .iter()
                    .rposition(|(ret_from, _)| ret_from == id)
                {
                    spacing_return_stack.remove(pos);
                }
            }
            Event::Autonumber(cmd) => spacing_auto.apply(cmd),
            // A `ref over A, …, Z` box must span its covered participants: the
            // centre-to-centre distance across them is at least the box's
            // preferred width minus the two end half-boxes.
            Event::Ref(r) => {
                let rb = ref_box(&r.text);
                let pref_w = rb.pref_w;
                let idxs: Vec<usize> = r
                    .participants
                    .iter()
                    .filter_map(|p| id_to_idx.get(p.as_str()).copied())
                    .collect();
                if let (Some(&lo), Some(&hi)) = (idxs.iter().min(), idxs.iter().max()) {
                    if lo == hi {
                        if lo < pair_max_label_width.len() {
                            let needed = (pref_w - participants[lo].box_width).max(0.0);
                            pair_max_label_width[lo] = pair_max_label_width[lo].max(needed);
                        }
                    } else {
                        let needed = pref_w
                            - participants[lo].box_width / 2.0
                            - participants[hi].box_width / 2.0;
                        if hi - lo == 1 {
                            pair_max_label_width[lo] = pair_max_label_width[lo].max(needed);
                        } else {
                            multi_span_constraints.push((lo, hi, needed));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Compute minimum first-participant x offset due to notes that extend left.
    // "note over" on the first participant: centered on the lifeline, must not
    // extend past the left margin (x = HEAD_BOX_Y).
    // "note left of" on the first participant: positioned entirely to the left
    // of the lifeline, so the lifeline must be far enough right to fit the note.
    //
    let mut min_first_center_x: f64 = 0.0;
    let mut min_scan_group_depth = 0usize;
    for event in &diagram.events {
        match event {
            Event::GroupStart(_) => min_scan_group_depth += 1,
            Event::GroupEnd => min_scan_group_depth = min_scan_group_depth.saturating_sub(1),
            _ => {}
        }
        if let Event::Note(note) = event {
            // A message-attached Left note anchors to the leftmost endpoint by
            // index; otherwise the first listed participant.
            let first_part = if note.on_message && note.position == NotePosition::Left {
                note.participants
                    .iter()
                    .filter_map(|id| id_to_idx.get(id.as_str()).copied())
                    .min()
            } else {
                note.participants
                    .first()
                    .and_then(|id| id_to_idx.get(id.as_str()))
                    .copied()
            };
            match note.position {
                NotePosition::Over if note.participants.len() == 1 && first_part == Some(0) => {
                    let max_tw = note_max_line_width_with_family(
                        &note.text,
                        note_font_size_f,
                        &note_font_family,
                    );
                    // Java centres the note on participant 0 using the raw
                    // preferred width (`NoteBox.getStartingX` / `ensureConstraints`),
                    // and the diagram is shifted right so the note's left edge sits
                    // at the HEAD_BOX_Y margin. The participant box left edge then
                    // lands at the rounded position
                    // `round(HEAD_BOX_Y + (raw_note_w - box_width) / 2)`. Using the
                    // ceiled width with `floor` here drops the sub-pixel fraction and
                    // mis-rounds the box left by 1px on many cases.
                    let bw = participants[0].box_width;
                    let min_cx = if diagram.teoz {
                        let note_w =
                            single_note_visible_raw_width(max_tw, note.shape, note_global_padding);
                        TEOZ_FIRST_OVER_NOTE_LEFT - TEOZ_PARTICIPANT_SHIFT + note_w / 2.0
                    } else {
                        let note_w =
                            note_content_width_raw_padded(max_tw, note.shape, note_text_align);
                        let box_left = (HEAD_BOX_Y + (note_w - bw) / 2.0).max(HEAD_BOX_Y).round();
                        box_left + bw / 2.0
                    };
                    min_first_center_x = min_first_center_x.max(min_cx);
                }
                NotePosition::Left if first_part == Some(0) => {
                    // "note left of" on participant 0: the note extends left from the
                    // lifeline. The note right edge = floor(lifeline_line_x) - gap.
                    // The note left edge = note_right - note_content_w, which must be >= HEAD_BOX_Y.
                    // So: lifeline_line_x >= HEAD_BOX_Y + note_content_w + gap
                    // And: lifeline_line_x = center_x - box_width/2 + floor(box_width/2)
                    // Therefore: center_x >= HEAD_BOX_Y + note_content_w + gap
                    //                        + box_width/2 - floor(box_width/2)
                    // hnote/rnote sit 1px closer to the lifeline than a standard
                    // note (the same shape offset as note_msg_arrow_offset): their
                    // box right edge is gap-1 from the lifeline.
                    let max_tw = note_max_line_width_with_family(
                        &note.text,
                        note_font_size_f,
                        &note_font_family,
                    );
                    let note_content_w =
                        note_content_width_padded(max_tw, note.shape, note_text_align);
                    let gap = left_note_lifeline_gap(
                        &participants,
                        note.shape,
                        first_part,
                        note.on_message,
                        note.color.is_some(),
                        note.text.lines().count(),
                    );
                    let bw = participants[0].box_width;
                    let min_cx = HEAD_BOX_Y + note_content_w + gap + bw / 2.0 - (bw / 2.0).floor();
                    min_first_center_x = min_first_center_x.max(min_cx);
                }
                _ => {}
            }
        }
        // A found message `[-> X` on the first participant reserves left space
        // for its incoming arrow + label (arrow_len = center-6 = label_w+18,
        // so center = label_w + 24).
        if let Event::Message(msg) = event
            && msg.from == "["
            && id_to_idx.get(msg.to.as_str()) == Some(&0)
        {
            let label_w = message_label_width(&process_label(&msg.label));
            let group_pad = if min_scan_group_depth > 0 {
                MSG_TEXT_LEFT_PAD
            } else {
                0.0
            };
            min_first_center_x = min_first_center_x.max(label_w + 24.0 + group_pad);
        }
    }

    // Resolve multi-span constraints: only widen the rightmost pair if the
    // cumulative existing width across the span is less than needed. This
    // matches PlantUML — a message spanning multiple participants doesn't
    // force intermediate pairs to widen when neighbouring messages already
    // provide enough room.
    //
    // We also need to consider min_gap_boxes for each pair (from Phase 3),
    // so compute that ahead of the constraint pass.
    let shadow_preferred_width_extra = if sequence_shadowing {
        SHADOW_LIVING_WIDTH_EXTRA
    } else {
        0.0
    };
    let participant_layout_halves: Vec<f64> = participants
        .iter()
        .map(|p| (p.box_width + shadow_preferred_width_extra) / 2.0)
        .collect();
    let participant_box_members: Vec<Option<usize>> = participants
        .iter()
        .map(|p| {
            participant_boxes
                .iter()
                .position(|b| b.members.contains(&p.decl_idx))
        })
        .collect();
    let participant_box_member = |idx: usize| -> Option<usize> {
        participant_box_members.iter().copied().nth(idx).flatten()
    };
    let min_gap_boxes_for_pair = |i: usize| -> f64 {
        let teoz_box_gap = if diagram.teoz
            && has_boxes
            && participant_box_member(i) != participant_box_member(i + 1)
        {
            TEOZ_BOX_BOUNDARY_GAP
        } else {
            0.0
        };
        participant_layout_halves[i]
            + participant_layout_halves[i + 1]
            + participant_box_gap
            + teoz_box_gap
    };
    for &(left, right, needed) in &multi_span_constraints {
        let mut cumulative = 0.0_f64;
        for (i, w) in pair_max_label_width
            .iter()
            .enumerate()
            .take(right)
            .skip(left)
        {
            cumulative += w.max(min_gap_boxes_for_pair(i));
        }
        if cumulative < needed {
            // Add the deficit to the rightmost pair.
            let deficit = needed - cumulative;
            let last = right - 1;
            pair_max_label_width[last] =
                pair_max_label_width[last].max(min_gap_boxes_for_pair(last)) + deficit;
        }
    }

    // -----------------------------------------------------------------------
    // Phase 3: Assign x positions
    // -----------------------------------------------------------------------

    // Additional rightward shift forced by a title/header/caption/footer wider than the
    // participant span. PlantUML centres each such band on the participant-span
    // midpoint `(first.box_x + last.box_x + last.box_width - 1.0) / 2.0`; when the
    // band (at its own left margin) would push that midpoint right of where the
    // participants currently sit, the whole diagram shifts and the canvas grows
    // symmetrically by `2 * meta_shift`. Derived from seq_footer_variant_01
    // (footer 133.4033 @ m=0 -> shift 13.249, width 113->140), seq_title_basic
    // (title 159.1133 @ m=10 -> shift 36.104, width 113->186) and the caption
    // variants (m=1). See the C3 width-feedback note.
    let mut meta_shift: f64 = 0.0;
    if !participants.is_empty() {
        // First participant center must be at least min_first_center_x (for notes)
        // and at least HEAD_BOX_Y + box_width/2 (to fit the box).
        // When groups encompass the leftmost participant, PlantUML reserves one
        // frame margin per enclosing frame so each nested frame can still land at
        // the 10px canvas floor (outer at 10, next at 20, ...).
        // In Teoz mode a group frame overlays the participant region (its left
        // edge sits at `min_involved_center - GroupingTile.MARGINX`) rather than
        // pushing the leftmost participant right, so no left shift is reserved.
        let group_shift = if diagram.teoz {
            0.0
        } else if group_left_shift_depth > 0 {
            group_frame_margin * group_left_shift_depth as f64
                + HEAD_BOX_Y
                + if group_left_external_shift_depth > 0 {
                    MSG_TEXT_LEFT_PAD
                } else {
                    0.0
                }
        } else {
            0.0
        };
        let teoz_box_shift = if diagram.teoz && has_boxes {
            TEOZ_BOX_BOUNDARY_GAP
        } else {
            0.0
        };
        let default_center = HEAD_BOX_Y
            + participant_outer_padding
            + group_shift
            + teoz_box_shift
            + participant_layout_halves[0];
        participants[0].center_x = default_center.max(min_first_center_x);
        participants[0].box_x = participants[0].center_x - participant_layout_halves[0];
        // PlantUML computes lifeline line x as box_x + (int)(preferredWidth / 2)
        participants[0].lifeline_line_x =
            participants[0].box_x + participant_layout_halves[0].floor();

        for i in 1..n {
            // Minimum center gap, including PlantUML's participant edge padding.
            let min_gap_boxes = min_gap_boxes_for_pair(i - 1);

            // Gap from message labels
            let gap_from_labels = pair_max_label_width[i - 1];

            let gap = min_gap_boxes.max(gap_from_labels);
            participants[i].center_x = participants[i - 1].center_x + gap;
            participants[i].box_x = participants[i].center_x - participant_layout_halves[i];
            participants[i].lifeline_line_x =
                participants[i].box_x + participant_layout_halves[i].floor();
        }

        // Titled participant boxes feed back into layout: when the title is
        // wider than the enclosed heads, Java widens the frame and recentres
        // the member heads under it, pushing later participants right.
        if has_boxes {
            for b in participant_boxes.iter().copied() {
                if b.title.is_empty() {
                    continue;
                }
                let mut member_idxs: Vec<usize> = b
                    .members
                    .iter()
                    .filter_map(|&pi| participants.iter().position(|p| p.decl_idx == pi))
                    .collect();
                if member_idxs.is_empty() {
                    continue;
                }
                member_idxs.sort_unstable();
                let lo = *member_idxs.first().unwrap();
                let hi = *member_idxs.last().unwrap();
                let content_left = member_idxs
                    .iter()
                    .map(|&i| participants[i].box_x)
                    .fold(f64::INFINITY, f64::min);
                let content_right = member_idxs
                    .iter()
                    .map(|&i| participants[i].box_x + participants[i].box_width)
                    .fold(f64::NEG_INFINITY, f64::max);
                let frame_w = content_right - content_left + 2.0 * BOX_SIDE_MARGIN;
                let title_w = bold_text_width(&b.title, BOX_TITLE_FONT_SIZE as f64);
                let needed_w = title_w + 6.0;
                if needed_w <= frame_w {
                    continue;
                }
                let extra = needed_w - frame_w;
                for p in participants.iter_mut().take(hi + 1).skip(lo) {
                    p.center_x += extra / 2.0;
                    p.box_x += extra / 2.0;
                    p.lifeline_line_x += extra / 2.0;
                }
                for p in participants.iter_mut().skip(hi + 1) {
                    p.center_x += extra;
                    p.box_x += extra;
                    p.lifeline_line_x += extra;
                }
            }
        }

        // An OVER_SEVERAL note (note across, or note over A,B) is centered on the
        // midpoint of its first/last participant box centers and extends pw/2 each
        // side. If its left edge would fall left of the HEAD_BOX_Y margin, the
        // whole diagram must shift right. pw = max(content, round(span) + 25).
        let mut across_shift: f64 = 0.0;
        for event in &diagram.events {
            if let Event::Note(note) = event
                && note.position == NotePosition::Over
                && (note.participants.is_empty() || note.participants.len() >= 2)
            {
                let (lo, hi) = if note.participants.is_empty() {
                    (0, n - 1)
                } else {
                    let a = note
                        .participants
                        .first()
                        .and_then(|id| id_to_idx.get(id.as_str()))
                        .copied();
                    let b = note
                        .participants
                        .last()
                        .and_then(|id| id_to_idx.get(id.as_str()))
                        .copied();
                    match (a, b) {
                        (Some(a), Some(b)) if a <= b => (a, b),
                        (Some(a), Some(b)) => (b, a),
                        _ => continue,
                    }
                };
                let max_tw = note_max_line_width_with_family(
                    &note.text,
                    note_font_size_f,
                    &note_font_family,
                );
                // Use the same geometry as the draw path so notes that overhang
                // participant 0 reserve exactly the shift Java would need.
                let shift = if note.participants.is_empty() {
                    let span = participants[hi].lifeline_line_x - participants[lo].lifeline_line_x;
                    let min_width = span.round() + ACROSS_NOTE_MARGIN;
                    let centre = (participants[lo].center_x + participants[hi].center_x) / 2.0;
                    let pw = note_content_width_raw_padded(max_tw, note.shape, note_text_align)
                        .max(min_width);
                    note_across_missing_space(centre, pw)
                } else if note.shape == NoteShape::Note {
                    let component_pref_w =
                        max_tw + ROSE_NOTE_COMPONENT_PREF_EXTRA + 2.0 * note_global_padding;
                    let content_visible_w =
                        note_content_width_padded(max_tw, note.shape, note_text_align);
                    let note_left = over_several_note_geometry(
                        &participants,
                        lo,
                        hi,
                        component_pref_w,
                        content_visible_w,
                        diagram.teoz,
                    )
                    .visible_left;
                    (HEAD_BOX_Y - note_left).max(0.0).floor()
                } else {
                    let margin = OVER_SEVERAL_NOTE_MARGIN;
                    let span = participants[hi].lifeline_line_x - participants[lo].lifeline_line_x;
                    let min_width = span.round() + margin;
                    let centre = (participants[lo].center_x + participants[hi].center_x) / 2.0;
                    let pw = over_several_shape_position_width_raw_padded(
                        max_tw,
                        note.shape,
                        note_text_align,
                        min_width,
                        centre,
                    );
                    let note_left = (centre - pw / 2.0).floor();
                    (HEAD_BOX_Y - note_left).max(0.0).floor()
                };
                across_shift = across_shift.max(shift);
            }
        }
        if across_shift > 0.0 {
            let shift = across_shift.floor();
            for p in participants.iter_mut() {
                p.center_x += shift;
                p.box_x += shift;
                p.lifeline_line_x += shift;
            }
        }

        // A note that overhangs participant 0 while enclosed by group frames is
        // held back by those frames: its drawn left edge cannot fall left of
        // `GROUP_NOTE_LEFT_FLOOR_BASE + depth * group_frame_margin` (each frame
        // insets its content a further MARGIN10; the outermost frame rect then
        // lands one MARGIN10 further left). When the note's natural left (at the
        // current participant positions) is left of that floor, the whole diagram
        // shifts right by the integer deficit — mirroring Java's
        // `prepareMissingSpace` push driven by the group header's `getStartingX`.
        // The shift is integral so participant box_x values stay on whole pixels.
        let mut group_note_shift: f64 = 0.0;
        let mut depth: usize = 0;
        for event in &diagram.events {
            match event {
                Event::GroupStart(_) => depth += 1,
                Event::GroupEnd => depth = depth.saturating_sub(1),
                Event::Note(note) if depth > 0 => {
                    // Only notes anchored on (or extending left from) participant 0
                    // can push the left margin.
                    let first_part = if note.on_message && note.position == NotePosition::Left {
                        note.participants
                            .iter()
                            .filter_map(|id| id_to_idx.get(id.as_str()).copied())
                            .min()
                    } else {
                        note.participants
                            .first()
                            .and_then(|id| id_to_idx.get(id.as_str()))
                            .copied()
                    };
                    if first_part != Some(0) {
                        continue;
                    }
                    let max_tw = note_max_line_width_with_family(
                        &note.text,
                        note_font_size_f,
                        &note_font_family,
                    );
                    let floor = GROUP_NOTE_LEFT_FLOOR_BASE + depth as f64 * group_frame_margin;
                    let natural_left = match note.position {
                        NotePosition::Over if note.participants.len() == 1 => {
                            // Mirror the renderer: left = floor(centerX - raw_w/2).
                            let raw_margin = match note.shape {
                                NoteShape::Note => 21.0,
                                NoteShape::Hexagonal => 24.0,
                                NoteShape::Rectangular => 8.0,
                            };
                            let raw_w = max_tw + raw_margin;
                            Some((participants[0].center_x - raw_w / 2.0).floor())
                        }
                        NotePosition::Left => {
                            let note_content_w =
                                note_content_width_raw_padded(max_tw, note.shape, note_text_align);
                            let gap = left_note_lifeline_gap(
                                &participants,
                                note.shape,
                                Some(0),
                                false,
                                note.color.is_some(),
                                note.text.lines().count(),
                            );
                            // Java's group InGroupable reservation for a left
                            // side note lands one pixel left of the visible
                            // group-floor note body; that extra pixel is what
                            // drives the later missing-space participant push.
                            Some(
                                (participants[0].lifeline_line_x - gap - note_content_w).floor()
                                    - 1.0,
                            )
                        }
                        _ => None,
                    };
                    if let Some(nl) = natural_left {
                        group_note_shift = group_note_shift.max(floor - nl);
                    }
                }
                _ => {}
            }
        }
        if group_note_shift > 0.0 {
            let shift = group_note_shift.ceil();
            for p in participants.iter_mut() {
                p.center_x += shift;
                p.box_x += shift;
                p.lifeline_line_x += shift;
            }
        }

        if group_left_shift_depth > 0 && !participants.is_empty() {
            // The outermost frame's left edge: box-edge minus margin (standard),
            // or lifeline-centre minus MARGINX (Teoz). Shift the diagram right so
            // it never crosses the 10px canvas floor. In Teoz the floor applies to
            // the frame's *InGroupable* left edge (`frame_left - EXTERNAL_MARGINX1`)
            // and the frame is anchored to the post-shift lifeline centre, so both
            // the pending TEOZ_PARTICIPANT_SHIFT and EXTERNAL_MARGINX1 are folded in.
            let left = if diagram.teoz {
                participants[0].center_x + TEOZ_PARTICIPANT_SHIFT
                    - TEOZ_GROUP_MARGIN_X
                    - TEOZ_GROUP_EXTERNAL_MARGIN_X1
            } else {
                participants[0].box_x - group_frame_margin
            };
            let shift = (GROUP_FRAME_MIN_LEFT - left).max(0.0);
            if shift > 0.0 {
                for p in participants.iter_mut() {
                    p.center_x += shift;
                    p.box_x += shift;
                    p.lifeline_line_x += shift;
                }
            }
        }

        // Title/header/caption/footer band feedback: if any band is wider than the
        // current participant span, shift the participants so the span midpoint
        // lands under the band's centre. `c0` is the span midpoint on the
        // current (post-across-shift) layout; each band wants its own centre at
        // `left_margin + band_width / 2`.
        let first = &participants[0];
        let last = &participants[n - 1];
        let c0 = (first.box_x + last.box_x + last.box_width - 1.0) / 2.0;
        let mut want_center: f64 = c0;
        if !header_lines.is_empty() {
            // Header text right-aligns to the canvas with a 6px right inset.
            // When it would overhang the left edge, PlantUML shifts the same
            // participant span used by other page decorations. Multiline
            // headers use the widest rendered line.
            let w = header_lines
                .iter()
                .map(|line| {
                    text_width_with_family(line, HEADER_FONT_SIZE as f64, &page_font_family)
                })
                .fold(0.0_f64, f64::max);
            want_center = want_center.max(w / 2.0);
        }
        if let Some(footer) = &diagram.meta.footer {
            // Footer left margin is 0.
            let w = text_width_with_family(footer, 10.0, &page_font_family);
            want_center = want_center.max(w / 2.0);
        }
        if let Some(caption) = &diagram.meta.caption {
            // Caption left margin is 1.
            let w = text_width_with_family(caption, 14.0, &page_font_family);
            want_center = want_center.max(1.0 + w / 2.0);
        }
        if !legend_lines.is_empty() {
            // Legend boxes use a fixed 12px left margin and a 5px text inset on
            // each side; when wider than the participant span they shift the
            // same span right and grow the canvas symmetrically.
            want_center = want_center.max(LEGEND_LEFT + legend_box_w / 2.0);
        }
        if !title_lines.is_empty() {
            // Title left margin is 10; use the widest line.
            let w = title_lines
                .iter()
                .map(|line| text_render::measure(line, TITLE_FONT_SIZE as f64, true))
                .fold(0.0_f64, f64::max);
            want_center = want_center.max(10.0 + w / 2.0);
        }
        meta_shift = (want_center - c0).max(0.0);
        if meta_shift > 0.0 {
            for p in participants.iter_mut() {
                p.center_x += meta_shift;
                p.box_x += meta_shift;
                p.lifeline_line_x += meta_shift;
            }
        }
    }
    if diagram.teoz {
        for p in participants.iter_mut() {
            p.center_x += TEOZ_PARTICIPANT_SHIFT;
            p.box_x += TEOZ_PARTICIPANT_SHIFT;
            p.lifeline_line_x = p.center_x;
        }
    }

    let center_of = |id: &str| -> f64 {
        id_to_idx
            .get(id)
            .map(|&i| participants[i].center_x)
            .unwrap_or(0.0)
    };

    // Created participants: `create X` causes X's head box to be drawn inline at
    // the message that creates it (the first message targeting X at or after the
    // Create event) rather than at the top, with the lifeline starting there.
    // Map: created participant id -> index of its creating message event.
    let mut create_msg_idx: HashMap<String, usize> = HashMap::new();
    {
        let mut pending: Vec<String> = Vec::new();
        for (idx, event) in diagram.events.iter().enumerate() {
            match event {
                Event::Create(id) => pending.push(id.clone()),
                Event::Message(msg) => {
                    if let Some(pos) = pending.iter().position(|id| *id == msg.to) {
                        let id = pending.remove(pos);
                        create_msg_idx.entry(id).or_insert(idx);
                    }
                }
                _ => {}
            }
        }
    }
    // Map creating-message event index -> extra vertical advance for that event:
    // the base advance plus any extent by which a tall inline head shape (actor,
    // boundary, etc.) reaches below a rectangle box.
    let create_msg_extra: HashMap<usize, f64> = create_msg_idx
        .iter()
        .filter_map(|(id, &idx)| {
            id_to_idx.get(id).map(|&pi| {
                let extra_h = (participants[pi].box_height - HEAD_BOX_H).max(0.0);
                let queue_adjust = if participants[pi].kind == ParticipantKind::Queue {
                    CREATE_QUEUE_ADVANCE_ADJUST
                } else {
                    0.0
                };
                (idx, CREATE_EXTRA_ADVANCE + extra_h - queue_adjust)
            })
        })
        .collect();

    // Compute self-message right extent now that x positions are assigned.
    // Replay activation state to know whether the participant is active at the
    // moment of each self-message — shifts cx by ACTIVATION_HALF_W if so.
    {
        let mut act_depth: HashMap<String, usize> = HashMap::new();
        let mut extent_auto = AutoState::default();
        for (idx, event) in diagram.events.iter().enumerate() {
            match event {
                Event::Message(msg) => {
                    if msg.from == msg.to {
                        let cx_base = center_of(&msg.from);
                        let active = act_depth.get(msg.from.as_str()).copied().unwrap_or(0) > 0
                            || matches!(msg.activation, Some(ActivationChange::Activate));
                        let cx = if active {
                            cx_base + ACTIVATION_HALF_W
                        } else {
                            cx_base
                        };
                        let label = process_label(&msg.label);
                        let label_w = message_label_width(&label);
                        let autonumber_extra = if let Some((_, w, _)) = extent_auto.current() {
                            w + AUTONUMBER_LABEL_GAP
                        } else {
                            0.0
                        };
                        let loopback_right = cx + SELF_MSG_EXTEND;
                        let loopback_extent_right = loopback_right + SELF_MSG_RIGHT_PAD;
                        let destroyed_later =
                            diagram
                                .events
                                .iter()
                                .skip(idx + 1)
                                .any(|event| match event {
                                    Event::Destroy(id) => id == &msg.from,
                                    Event::Message(next) => {
                                        next.to == msg.from
                                            && matches!(
                                                next.activation,
                                                Some(ActivationChange::Destroy)
                                            )
                                    }
                                    _ => false,
                                });
                        let created_active_pad =
                            if active && create_msg_idx.contains_key(msg.from.as_str()) {
                                CREATED_ACTIVE_SELF_MSG_RIGHT_PAD
                            } else if active && destroyed_later {
                                DESTROYED_ACTIVE_SELF_MSG_RIGHT_PAD
                            } else {
                                0.0
                            };
                        let ordinary_active_self = active
                            && msg.activation.is_none()
                            && !create_msg_idx.contains_key(msg.from.as_str())
                            && !destroyed_later;
                        let text_extent_right = if ordinary_active_self {
                            cx + autonumber_extra + label_w + 2.0 * MSG_TEXT_LEFT_PAD + ARROW_SIZE
                        } else {
                            cx + SELF_MSG_TEXT_X_PAD
                                + autonumber_extra
                                + label_w
                                + SELF_MSG_RIGHT_PAD
                        };
                        let activation_stack_depth = match msg.activation {
                            Some(ActivationChange::Activate) => {
                                act_depth.get(msg.from.as_str()).copied().unwrap_or(0) + 1
                            }
                            Some(ActivationChange::Deactivate) => {
                                act_depth.get(msg.from.as_str()).copied().unwrap_or(0)
                            }
                            _ => 0,
                        };
                        let self_activation_pad = if activation_stack_depth > 0 {
                            5.0 + activation_stack_depth as f64 * ACTIVATION_WIDTH
                        } else {
                            0.0
                        };
                        let self_right = loopback_extent_right.max(text_extent_right)
                            + created_active_pad
                            + self_activation_pad;
                        max_self_msg_right = max_self_msg_right.max(self_right);
                        self_msg_right_note_left_by_event.insert(
                            idx,
                            cx.floor()
                                + autonumber_extra
                                + label_w
                                + SELF_MSG_RIGHT_NOTE_X_PAD
                                + created_active_pad,
                        );
                    }
                    // Update activation state from message activation flag.
                    if let Some(act) = &msg.activation {
                        match act {
                            ActivationChange::Activate => {
                                *act_depth.entry(msg.to.clone()).or_default() += 1;
                            }
                            ActivationChange::Deactivate => {
                                if let Some(d) = act_depth.get_mut(&msg.from) {
                                    *d = d.saturating_sub(1);
                                }
                            }
                            ActivationChange::Destroy => {
                                if let Some(d) = act_depth.get_mut(&msg.to) {
                                    *d = d.saturating_sub(1);
                                }
                            }
                        }
                    }
                    extent_auto.advance();
                }
                Event::Activate(id, _) => {
                    *act_depth.entry(id.clone()).or_default() += 1;
                }
                Event::Deactivate(id) => {
                    if let Some(d) = act_depth.get_mut(id) {
                        *d = d.saturating_sub(1);
                    }
                }
                Event::Return(_) => extent_auto.advance(),
                Event::Autonumber(cmd) => extent_auto.apply(cmd),
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 4: Pre-compute y positions for each event and vertical dimensions
    // -----------------------------------------------------------------------

    // Use the actual maximum box height across participants. Smaller
    // `participantFontSize` values shrink the head band rather than reserving
    // the default 14pt participant height.
    let max_box_h = participants
        .iter()
        .map(|p| p.box_height)
        .fold(0.0_f64, f64::max);
    let shadow_vertical_pad = if sequence_shadowing {
        SHADOW_VERTICAL_PAD
    } else {
        0.0
    };
    let shadow_note_extra = if sequence_shadowing {
        SHADOW_NOTE_EXTRA
    } else {
        0.0
    };
    let lifeline_top = head_box_y + max_box_h + LIFELINE_Y_OFFSET + shadow_vertical_pad;

    let event_message_text_height = |event: &Event| -> (bool, f64) {
        match event {
            Event::Message(msg) => {
                let label = process_label(&msg.label);
                (
                    !label.is_empty(),
                    message_label_block_height_with_family(
                        &label,
                        message_font_size_f,
                        &message_font_family,
                    ),
                )
            }
            Event::Return(ret) => {
                let label = if ret.label.is_empty() {
                    String::new()
                } else {
                    decode_backslash_escapes(&ret.label)
                };
                (
                    !label.is_empty(),
                    message_label_block_height_with_family(
                        &label,
                        message_font_size_f,
                        &message_font_family,
                    ),
                )
            }
            // An empty divider (`====`) has no label box/text line, so it
            // reserves no text height.
            Event::Divider(t) => {
                let (label, _, _) = divider_label_and_font_size(t, divider_font_size);
                (!label.is_empty(), MSG_TEXT_HEIGHT)
            }
            Event::Delay(t) => (t.is_some(), message_text_height),
            _ => (false, message_text_height),
        }
    };

    // Pre-scan: a bare `note left` / `note right` attached to a message
    // (Note.on_message) straddles that message's arrow band rather than
    // consuming its own vertical row. Map each such note event to the
    // preceding message event it belongs to, and track the maximum note
    // line-count per owning message (the tile grows for multi-line notes).
    let mut note_owner: HashMap<usize, usize> = HashMap::new();
    // Extra vertical space each owning message reserves above and below its
    // arrow for the attached note (max across multiple attached notes).
    let mut msg_note_extra: HashMap<usize, f64> = HashMap::new();
    let mut msg_note_y_adjust: HashMap<usize, f64> = HashMap::new();
    let mut msg_note_tail_extra: HashMap<usize, f64> = HashMap::new();
    let mut msg_side_note_count: HashMap<usize, usize> = HashMap::new();
    {
        let mut last_msg_idx: Option<usize> = None;
        for (idx, event) in diagram.events.iter().enumerate() {
            match event {
                Event::Message(_) | Event::Return(_) | Event::Delay(_) => {
                    last_msg_idx = Some(idx);
                }
                Event::Note(note) if note.on_message => {
                    if let Some(owner) = last_msg_idx {
                        note_owner.insert(idx, owner);
                        *msg_side_note_count.entry(owner).or_insert(0) += 1;
                        let metrics = note_text_metrics_with_family(
                            &note.text,
                            note_font_size_f,
                            &note_font_family,
                        );
                        let self_message_right_note = note.position == NotePosition::Right
                            && matches!(
                                diagram.events.get(owner),
                                Some(Event::Message(msg)) if msg.from == msg.to
                            );
                        let (owner_has_text, owner_text_height) =
                            event_message_text_height(&diagram.events[owner]);
                        let y_adjust = if arrow_font_size_set
                            && owner_has_text
                            && message_font_family.eq_ignore_ascii_case("sans-serif")
                        {
                            (MSG_TEXT_HEIGHT - owner_text_height) / 2.0
                        } else {
                            0.0
                        };
                        let extra = if self_message_right_note {
                            0.0
                        } else {
                            note_msg_extra_base(note.shape)
                                + note_msg_text_tail(&metrics)
                                + shadow_note_extra
                                + y_adjust
                        };
                        let e = msg_note_extra.entry(owner).or_insert(0.0);
                        *e = e.max(extra);
                        msg_note_y_adjust.insert(owner, y_adjust);
                        if note
                            .text
                            .lines()
                            .next()
                            .is_some_and(|line| note_separator_label(line).is_some())
                        {
                            let e = msg_note_tail_extra.entry(owner).or_insert(0.0);
                            *e = e.max(PURE_UNDERLINE_MESSAGE_FLOW_EXTRA);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut note_right_active_shift_by_event: HashMap<usize, f64> = HashMap::new();
    {
        let mut active_depth: HashMap<String, usize> = HashMap::new();
        let mut return_stack: Vec<String> = Vec::new();
        let mut last_return_from: Option<String> = None;
        for (idx, event) in diagram.events.iter().enumerate() {
            match event {
                Event::Message(msg) => {
                    last_return_from = Some(msg.to.clone());
                    if let Some(act) = &msg.activation {
                        match act {
                            ActivationChange::Activate => {
                                *active_depth.entry(msg.to.clone()).or_default() += 1;
                                return_stack.push(msg.to.clone());
                            }
                            ActivationChange::Deactivate => {
                                if let Some(d) = active_depth.get_mut(&msg.from) {
                                    *d = d.saturating_sub(1);
                                }
                                if let Some(pos) =
                                    return_stack.iter().rposition(|id| id == &msg.from)
                                {
                                    return_stack.remove(pos);
                                }
                            }
                            ActivationChange::Destroy => {
                                if let Some(d) = active_depth.get_mut(&msg.to) {
                                    *d = d.saturating_sub(1);
                                }
                            }
                        }
                    }
                }
                Event::Return(_) => {
                    if let Some(id) = return_stack.pop()
                        && let Some(d) = active_depth.get_mut(&id)
                    {
                        *d = d.saturating_sub(1);
                    }
                }
                Event::Activate(id, _) => {
                    *active_depth.entry(id.clone()).or_default() += 1;
                    if last_return_from.as_deref() == Some(id.as_str()) {
                        return_stack.push(id.clone());
                    }
                }
                Event::Deactivate(id) => {
                    if let Some(d) = active_depth.get_mut(id) {
                        *d = d.saturating_sub(1);
                    }
                    if let Some(pos) = return_stack.iter().rposition(|p| p == id) {
                        return_stack.remove(pos);
                    }
                }
                Event::Note(note)
                    if !note.on_message
                        && note.position == NotePosition::Right
                        && note
                            .participants
                            .first()
                            .and_then(|id| active_depth.get(id.as_str()))
                            .copied()
                            .unwrap_or(0)
                            > 0 =>
                {
                    note_right_active_shift_by_event.insert(idx, ACTIVATION_HALF_W);
                }
                _ => {}
            }
        }
    }

    // Pre-compute message y positions. PlantUML sizes each message step
    // dynamically: messages with label text get extra height for the text line.
    // Notes consume vertical space (note height + gap) and count as events
    // for msg_count (so subsequent messages use msg_step, not first_msg_offset).
    // `newpage` splits the diagram into pages; single-image SVG output renders
    // only the FIRST page (participant box widths/positions are still derived
    // from the whole diagram, so wide page-2 labels still set the canvas
    // width). Everything from the first `newpage` onward is dropped from the
    // height/message flow, and a horizontal separator rule is drawn at the
    // bottom of page 1. See Java `SequenceDiagramFileMaker` page handling and
    // `GraphicalNewpage`/`ComponentRoseNewpage`.
    let page1_end = diagram
        .events
        .iter()
        .position(|e| matches!(e, Event::NewPage(_)))
        .unwrap_or(diagram.events.len());
    let has_newpage = page1_end < diagram.events.len();

    // Groups add header/else/end vertical space.
    let mut event_y_positions: Vec<f64> = Vec::new();
    let mut msg_count: u32 = 0;
    let last_effective_y;
    {
        let mut y = lifeline_top;
        let message_vertical_padding = 2.0 * global_padding;
        for (idx, event) in diagram.events.iter().take(page1_end).enumerate() {
            let (has_text, event_text_height) = event_message_text_height(event);
            match event {
                Event::Message(msg) => {
                    let is_self = msg.from == msg.to;
                    // A message-attached note (bare `note left`/`note right`)
                    // straddles this message's arrow band. It adds equal extra
                    // space above (pushing the arrow down) and below (pushing the
                    // next event down).
                    let note_extra = msg_note_extra.get(&idx).copied();
                    if msg_count == 0 {
                        y += first_msg_offset(
                            has_text,
                            event_text_height - event_pure_underline_flow_extra(event),
                        ) + message_vertical_padding;
                    } else {
                        y += msg_step(has_text, event_text_height) + message_vertical_padding;
                    }
                    if let Some(extra) = note_extra {
                        y += extra;
                    }
                    event_y_positions.push(y);
                    if let Some(extra) = note_extra {
                        y += extra;
                    }
                    if let Some(extra) = msg_note_tail_extra.get(&idx).copied() {
                        y += extra;
                    }
                    if is_self {
                        // Self-messages have a loopback that drops below the top line.
                        // The next message's y step starts from the bottom of the loop.
                        y += SELF_MSG_DROP;
                    }
                    // A creating message draws an inline head box that straddles the
                    // arrow; the next event must clear the box bottom.
                    if let Some(&extra) = create_msg_extra.get(&idx) {
                        y += extra;
                    }
                    msg_count += 1;
                }
                Event::Return(_) => {
                    let note_extra = msg_note_extra.get(&idx).copied();
                    if msg_count == 0 {
                        y += first_msg_offset(
                            has_text,
                            event_text_height - event_pure_underline_flow_extra(event),
                        ) + message_vertical_padding;
                    } else {
                        y += msg_step(has_text, event_text_height) + message_vertical_padding;
                    }
                    if let Some(extra) = note_extra {
                        y += extra;
                    }
                    event_y_positions.push(y);
                    if let Some(extra) = note_extra {
                        y += extra;
                    }
                    if let Some(extra) = msg_note_tail_extra.get(&idx).copied() {
                        y += extra;
                    }
                    msg_count += 1;
                }
                Event::Divider(dt) => {
                    // Dividers take a total of msg_step + MSG_BASE_STEP vertical space.
                    // The event_y is positioned at the divider text baseline, which is
                    // at msg_step + 5.258 from the previous event. The remaining
                    // MSG_BASE_STEP - 5.258 = 8.742 adds to the gap before the next message.
                    const DIVIDER_TEXT_OFFSET: f64 = 5.2578;
                    const DIVIDER_TAIL_PAD: f64 = MSG_BASE_STEP - DIVIDER_TEXT_OFFSET;
                    // An empty divider (`====`) has no label box: has_text is
                    // false (so msg_step drops the full text height), but the
                    // strip still sits half a text-line lower than that.
                    let empty_adj = if dt.trim().is_empty() {
                        MSG_TEXT_HEIGHT / 2.0
                    } else {
                        0.0
                    };
                    if msg_count == 0 {
                        y += first_msg_offset(has_text, MSG_TEXT_HEIGHT) + DIVIDER_TEXT_OFFSET;
                    } else {
                        y += msg_step(has_text, MSG_TEXT_HEIGHT) + DIVIDER_TEXT_OFFSET;
                    }
                    // The empty divider's strip sits half a text-line lower, but
                    // this offset does not push subsequent events down.
                    event_y_positions.push(y + empty_adj);
                    // The divider's tail padding accounts for the space below the
                    // text baseline (the double lines extend above, but PlantUML
                    // also reserves space below for visual balance).
                    let (_, effective_font_size, _) =
                        divider_label_and_font_size(dt, divider_font_size);
                    let height_extra = if has_text {
                        text_height_with_family(effective_font_size as f64, &divider_font_family)
                            - MSG_TEXT_HEIGHT
                    } else {
                        0.0
                    };
                    y += DIVIDER_TAIL_PAD + height_extra;
                    msg_count += 1;
                }
                Event::Delay(t) => {
                    if msg_count == 0 {
                        y += first_msg_offset(has_text, MSG_TEXT_HEIGHT);
                    } else {
                        // A delay reserves a fixed 28px dotted band (plus the
                        // label height when labelled), not a normal message step.
                        y += DELAY_BAND_HEIGHT
                            + if t.is_some() {
                                plantuml_metrics::text_height(DELAY_LABEL_FONT_SIZE as f64)
                            } else {
                                0.0
                            };
                    }
                    event_y_positions.push(y);
                    msg_count += 1;
                }
                Event::Note(note) => {
                    let metrics = note_text_metrics_with_family(
                        &note.text,
                        note_font_size_f,
                        &note_font_family,
                    );
                    // hnote/rnote have a smaller base height (23 vs 25), reducing
                    // the vertical space consumed by 2px.
                    let note_y_extra = match note.shape {
                        NoteShape::Note => 7.0,
                        NoteShape::Hexagonal | NoteShape::Rectangular => 5.0,
                    };
                    if let Some(&owner) = note_owner.get(&idx) {
                        // Message-attached note: it straddles the owning message's
                        // arrow band and does NOT consume its own vertical row
                        // (the owning message already reserved the extra space).
                        // Position note_top so the arrow sits at
                        // note_top + NOTE_MSG_ARROW_OFFSET + (lines-1)*MSG_TEXT_HEIGHT/2.
                        let arrow_y = event_y_positions.get(owner).copied().unwrap_or(y);
                        let self_message_right_note = note.position == NotePosition::Right
                            && matches!(
                                diagram.events.get(owner),
                                Some(Event::Message(msg)) if msg.from == msg.to
                            );
                        let note_top = if self_message_right_note {
                            arrow_y - metrics.first_height + SELF_MSG_RIGHT_NOTE_Y_PAD
                        } else {
                            let separator_attach_adjust = note
                                .text
                                .lines()
                                .next()
                                .filter(|line| note_separator_label(line).is_some())
                                .map(|_| PURE_UNDERLINE_MESSAGE_FLOW_EXTRA)
                                .unwrap_or(0.0);
                            arrow_y
                                - note_msg_arrow_offset_for_line(note.shape, metrics.first_height)
                                - note_msg_text_tail(&metrics)
                                - 2.0 * note_global_padding
                                - shadow_note_extra
                                + msg_note_y_adjust.get(&owner).copied().unwrap_or(0.0)
                                + separator_attach_adjust
                        };
                        // The draw site derives note_top from event_y via
                        // note_top = event_y - note_y_extra - num_lines*MSG_TEXT_HEIGHT.
                        let note_event_y = note_top + note_y_extra + metrics.total_height;
                        event_y_positions.push(note_event_y);
                        // Do not advance y or increment msg_count.
                    } else {
                        // Standalone note: consumes vertical space. The note top is
                        // positioned relative to the current y cursor. After the
                        // note, subsequent events use msg_step (note counts as an event).
                        let note_top = if msg_count == 0 {
                            y + NOTE_GAP_FIRST
                        } else {
                            y + NOTE_GAP_AFTER_MSG
                        };
                        let note_event_y = note_top + note_y_extra + metrics.total_height;
                        y = note_event_y;
                        event_y_positions.push(y);
                        msg_count += 1; // note counts as an event for spacing
                    }
                }
                Event::GroupStart(_) => {
                    // Group frame top is offset from the preceding message (or
                    // the lifeline top when the group is the first event).
                    let inner_pad = if msg_count == 0 {
                        y += GROUP_GAP_FIRST;
                        group_inner_top_pad_first
                    } else {
                        y += GROUP_GAP_AFTER_MSG;
                        group_inner_top_pad
                    };
                    // Teoz: the frame top sits at the standard position MINUS the
                    // global teoz_top_pad (the frame is anchored to the body, not
                    // the head-shifted lifelines), and the header reserves
                    // MARGINY_MAGIC/2 of padding above the body in addition to the
                    // standard inner pad.
                    if diagram.teoz {
                        y -= teoz_top_pad;
                    }
                    event_y_positions.push(y);
                    // Advance y past the header so subsequent messages are positioned correctly.
                    y += inner_pad;
                    if diagram.teoz {
                        y += TEOZ_GROUP_MARGIN_Y / 2.0 + GROUP_HEADER_FIRST_PAD_ADJUST;
                    }
                    // Don't increment msg_count — the group header itself isn't a message
                }
                Event::GroupElse(g) => {
                    // Else divider adds vertical space. Teoz spaces the divider an
                    // extra TEOZ_GROUP_ELSE_EXTRA below the preceding message and
                    // reserves the same amount again before the else body.
                    y += GROUP_ELSE_HEIGHT;
                    if diagram.teoz {
                        y += TEOZ_GROUP_ELSE_EXTRA;
                    }
                    event_y_positions.push(y);
                    if g.label.is_some() {
                        // With a label: advance past the label text.
                        y += GROUP_ELSE_INNER_PAD;
                    } else {
                        // Without a label: PlantUML uses a tighter layout.
                        // The next message step (msg_step) overshoots by 7px
                        // because the else divider already contributed vertical
                        // space that partially overlaps the message base step.
                        y -= MSG_BASE_STEP - (MSG_BASE_FIRST_OFFSET - GROUP_ELSE_HEIGHT);
                    }
                    if diagram.teoz {
                        y += TEOZ_GROUP_ELSE_EXTRA;
                    }
                }
                Event::GroupEnd => {
                    // Group end: event_y marks the frame bottom,
                    // but the y cursor advances less (for tail gap calculation).
                    let empty_first_group = msg_count == 0
                        && matches!(
                            diagram.events.get(idx.saturating_sub(1)),
                            Some(Event::GroupStart(_))
                        );
                    let group_end_y = y
                        + GROUP_END_HEIGHT
                        + 1.0
                        + if empty_first_group {
                            GROUP_HEADER_FIRST_PAD_ADJUST
                        } else {
                            0.0
                        }
                        - if diagram.teoz { 2.0 } else { 0.0 };
                    event_y_positions.push(group_end_y);
                    y += GROUP_END_HEIGHT;
                    if diagram.teoz {
                        // The Teoz body reserves MARGINY_MAGIC/2 below itself; the
                        // following content steps from there.
                        y += GROUP_END_HEIGHT;
                    }
                }
                Event::Space(px_opt) => {
                    y += px_opt.map(|p| p as f64).unwrap_or(25.0);
                    event_y_positions.push(y);
                }
                Event::Ref(r) => {
                    // The reference box top sits REF_GAP_ABOVE below the
                    // previous arrow; the box consumes `preferred_h` of vertical
                    // flow measured from that previous arrow, so the next
                    // message steps from `prev_y + preferred_h` (verified vs the
                    // golden: box_top = call_y + 8, done = call_y + preferred_h
                    // + msg_step).
                    let rb = ref_box(&r.text);
                    let prev_y = y;
                    let first_ref = msg_count == 0;
                    let box_top = if first_ref {
                        prev_y + REF_FIRST_GAP
                    } else {
                        prev_y + REF_GAP_ABOVE
                    };
                    event_y_positions.push(box_top);
                    y = prev_y
                        + rb.preferred_h
                        + if first_ref { REF_FIRST_FLOW_EXTRA } else { 0.0 };
                    msg_count += 1;
                }
                Event::Activate(_, _) | Event::Deactivate(_) => {
                    event_y_positions.push(y);
                }
                _ => {
                    event_y_positions.push(y);
                }
            }
        }
        // The first `newpage` reserves NEWPAGE_SEPARATOR_HEIGHT in the page-1
        // flow (Java `prepareNewpage` advances `freeY2` by the separator
        // component's preferred height of 1), shifting the foot boxes and
        // lifeline bottoms of page 1 down by that amount.
        if has_newpage {
            y += NEWPAGE_SEPARATOR_HEIGHT;
        }
        last_effective_y = y;
    }

    // Compute tail box y based on message count.
    // With 0 messages, PlantUML uses a minimum lifeline height of 20px.
    // With messages, the tail starts TAIL_GAP below the last effective y
    // (which includes self-message drops).
    let tail_box_y = if msg_count > 0 {
        last_effective_y + TAIL_GAP
    } else {
        // Minimum lifeline: 20px, tail overlaps by LIFELINE_Y_OFFSET.
        // A leading `newpage` still reserves its 1px separator component in
        // Java's page-1 flow, even though no message was rendered before it.
        lifeline_top + MIN_LIFELINE_HEIGHT - LIFELINE_Y_OFFSET
            + if has_newpage {
                NEWPAGE_SEPARATOR_HEIGHT
            } else {
                0.0
            }
    };
    let tail_box_y = if diagram.teoz {
        tail_box_y - LIFELINE_Y_OFFSET
    } else {
        tail_box_y
    };
    let lifeline_bottom = if diagram.teoz {
        tail_box_y
    } else {
        tail_box_y + LIFELINE_Y_OFFSET
    };
    let lifeline_height = lifeline_bottom - lifeline_top;

    // SVG dimensions — account for notes that extend beyond participant boxes.
    let last_box_right = if participants.is_empty() {
        100.0
    } else {
        let last = &participants[n - 1];
        last.box_x + last.box_width
    };
    // Check if any note extends beyond the last participant box.
    // Dividers whose label box is wider than the participant span extend the
    // background strip (and hence the canvas) to box width + 12px each side.
    let mut max_divider_right: f64 = 0.0;
    for event in &diagram.events {
        if let Event::Divider(text) = event {
            let (label, effective_font_size, heading_style) =
                divider_label_and_font_size(text, divider_font_size);
            let label_box_w = divider_label_box_width(
                label,
                effective_font_size,
                &divider_font_family,
                heading_style,
            );
            max_divider_right = max_divider_right.max(label_box_w + 24.0);
        }
    }
    let mut max_note_right: f64 = 0.0;
    for (event_idx, event) in diagram.events.iter().enumerate() {
        if let Event::Note(note) = event {
            let max_line_width =
                note_max_line_width_with_family(&note.text, note_font_size_f, &note_font_family);
            let note_content_w =
                note_content_width_padded(max_line_width, note.shape, note_text_align);
            let raw_note_content_w =
                note_content_width_raw_padded(max_line_width, note.shape, note_text_align);
            match note.position {
                NotePosition::Right => {
                    if note.on_message
                        && let Some(&note_left) = note_owner
                            .get(&event_idx)
                            .and_then(|owner| self_msg_right_note_left_by_event.get(owner))
                    {
                        let mut note_right = note_left + note_content_w + NOTE_LIFELINE_GAP - 1.0;
                        if raw_note_content_w.fract() > 0.57 {
                            note_right += 1.0;
                        }
                        max_note_right = max_note_right.max(note_right);
                        continue;
                    }
                    // A message-attached note anchors to the message component's
                    // right endpoint, which follows the participant's visual
                    // centre rather than the integer lifeline line.
                    let anchor_x = if note.on_message {
                        note.participants
                            .iter()
                            .filter_map(|id| id_to_idx.get(id.as_str()))
                            .map(|&i| participants[i].center_x)
                            .fold(f64::MIN, f64::max)
                    } else {
                        note.participants
                            .first()
                            .and_then(|id| id_to_idx.get(id.as_str()))
                            .map(|&i| {
                                participants[i].center_x
                                    + note_right_active_shift_by_event
                                        .get(&event_idx)
                                        .copied()
                                        .unwrap_or(0.0)
                            })
                            .unwrap_or(f64::MIN)
                    };
                    if anchor_x != f64::MIN {
                        let gap = if note.on_message {
                            NOTE_LIFELINE_GAP - 1.0
                        } else {
                            NOTE_LIFELINE_GAP
                        };
                        let note_left = if note.on_message {
                            anchor_x.ceil() + gap
                        } else if diagram.teoz {
                            anchor_x + gap
                        } else {
                            (anchor_x + gap).floor()
                        };
                        let mut note_right = note_left + note_content_w;
                        // A message-attached note sits inside a message tile, which
                        // reserves an extra NOTE_LIFELINE_GAP of right margin.
                        // Its note box is drawn with the ceiled preferred width,
                        // while the tile/canvas reservation follows PlantUML's
                        // rounded raw-width extent. This can add one more pixel to
                        // the full-canvas background rect without moving the note.
                        if note.on_message {
                            note_right += NOTE_LIFELINE_GAP;
                            if raw_note_content_w.fract() > 0.57 {
                                note_right += 1.0;
                            }
                        } else if diagram.teoz {
                            note_right += NOTE_LIFELINE_GAP - 1.0;
                        }
                        max_note_right = max_note_right.max(note_right);
                    }
                }
                NotePosition::Over => {
                    if note.participants.is_empty() {
                        // "note across" — spans all participants. Right edge from
                        // the OVER_SEVERAL centre model (see draw site).
                        let across_right = if participants.is_empty() {
                            HEAD_BOX_Y + note_content_w
                        } else {
                            let first_ll = participants[0].lifeline_line_x;
                            let last_ll = participants[participants.len() - 1].lifeline_line_x;
                            let span = last_ll - first_ll;
                            let pw_raw = raw_note_content_w.max(span.round() + ACROSS_NOTE_MARGIN);
                            let pw = note_content_w.max(span.round() + ACROSS_NOTE_MARGIN);
                            let centre = (participants[0].center_x
                                + participants[participants.len() - 1].center_x)
                                / 2.0;
                            note_across_left(centre, pw_raw) + pw
                        };
                        max_note_right = max_note_right.max(across_right);
                    } else if note.participants.len() == 1 {
                        if let Some(&idx) = id_to_idx.get(note.participants[0].as_str()) {
                            // Java NoteBox.getMaxX = getStartingX + getPreferredWidth,
                            // where getStartingX = (int)(centerX - rawW/2) and the
                            // preferred width is the full-precision (un-ceiled) value.
                            // Using the ceiled width here drops the sub-pixel fraction
                            // and tips the canvas right edge / newpage separator x2 a
                            // whole pixel short.
                            let cx = participants[idx].center_x;
                            let raw_w = note_content_width_raw_padded(
                                max_line_width,
                                note.shape,
                                note_text_align,
                            );
                            let note_left = (cx - raw_w / 2.0).max(HEAD_BOX_Y).floor();
                            let note_right = note_left + raw_w;
                            max_note_right = max_note_right.max(note_right);
                        }
                    } else if let (Some(&first_idx), Some(&last_idx)) = (
                        id_to_idx.get(note.participants.first().unwrap().as_str()),
                        id_to_idx.get(note.participants.last().unwrap().as_str()),
                    ) {
                        let (lo, hi) = if first_idx <= last_idx {
                            (first_idx, last_idx)
                        } else {
                            (last_idx, first_idx)
                        };
                        let note_right = if note.shape == NoteShape::Note
                            || (diagram.teoz
                                && matches!(
                                    note.shape,
                                    NoteShape::Hexagonal | NoteShape::Rectangular
                                )) {
                            let component_pref_w = if note.shape == NoteShape::Note {
                                max_line_width
                                    + ROSE_NOTE_COMPONENT_PREF_EXTRA
                                    + 2.0 * note_global_padding
                            } else {
                                note_content_width_raw_padded(
                                    max_line_width,
                                    note.shape,
                                    note_text_align,
                                )
                            };
                            let geom = over_several_note_geometry(
                                &participants,
                                lo,
                                hi,
                                component_pref_w,
                                note_content_w,
                                diagram.teoz,
                            );
                            geom.visible_left + geom.visible_width
                        } else {
                            let span =
                                participants[hi].lifeline_line_x - participants[lo].lifeline_line_x;
                            let min_width = span.round() + OVER_SEVERAL_NOTE_MARGIN;
                            let note_w = note_content_w.max(min_width);
                            let centre =
                                (participants[lo].center_x + participants[hi].center_x) / 2.0;
                            let position_w = over_several_shape_position_width_raw_padded(
                                max_line_width,
                                note.shape,
                                note_text_align,
                                min_width,
                                centre,
                            );
                            (centre - position_w / 2.0).floor() + note_w
                        };
                        max_note_right = max_note_right.max(note_right);
                    }
                }
                NotePosition::Left => {} // left notes don't extend right
            }
        }
    }
    // A `ref over` box anchors at its leftmost covered participant and extends
    // right by its (preferred or span) width; over a single/narrow span it
    // overhangs the participant boxes and widens the canvas.
    let mut max_ref_right: f64 = 0.0;
    for event in &diagram.events {
        if let Event::Ref(r) = event {
            let rb = ref_box(&r.text);
            let mut r1 = f64::INFINITY;
            let mut mx = f64::NEG_INFINITY;
            for pid in &r.participants {
                if let Some(&pi) = id_to_idx.get(pid.as_str()) {
                    let p = &participants[pi];
                    r1 = r1.min(p.box_x - REF_OUT_MARGIN);
                    mx = mx.max(p.box_x + p.box_width + REF_OUT_MARGIN);
                }
            }
            if r1.is_finite() {
                let pref_w = rb.pref_w;
                // Only when the box is WIDER than the participant span does it
                // overhang and grow the canvas; a span-bound box fits within the
                // participant-derived right edge already.
                if pref_w > mx - r1 {
                    // Use the box-edge-equivalent (living right minus the
                    // participant outMargin), mirroring last_box_right, so the
                    // standard RIGHT_MARGIN applies on top.
                    max_ref_right = max_ref_right.max(r1 + pref_w - REF_OUT_MARGIN);
                }
            }
        }
    }
    // A lost message `X ->]` runs an arrow rightward from X by label_w+18 (plus
    // the arrowhead), extending the canvas to the right.
    let mut max_lost_right: f64 = 0.0;
    for event in &diagram.events {
        if let Event::Message(msg) = event
            && msg.to == "]"
            && let Some(&fi) = id_to_idx.get(msg.from.as_str())
        {
            let label_w = message_label_width(&process_label(&msg.label));
            // Canvas edge = arrow line end (label_w+18) + 1px stroke; the
            // arrowhead tip extends into the RIGHT_MARGIN.
            max_lost_right = max_lost_right.max(participants[fi].center_x + label_w + 19.0);
        }
    }
    // The widened participant-box frame can be the rightmost visible element;
    // compare its "content-equivalent" right edge so the standard right margin
    // is applied exactly once, like ref boxes above.
    let mut max_participant_box_right: f64 = 0.0;
    if has_boxes {
        for b in participant_boxes.iter().copied() {
            let members: Vec<&ParticipantLayout> = b
                .members
                .iter()
                .filter_map(|&pi| participants.iter().find(|p| p.decl_idx == pi))
                .collect();
            if members.is_empty() {
                continue;
            }
            let box_left = members
                .iter()
                .map(|p| p.box_x)
                .fold(f64::INFINITY, f64::min)
                - BOX_SIDE_MARGIN;
            let box_right = members
                .iter()
                .map(|p| p.box_x + p.box_width)
                .fold(f64::NEG_INFINITY, f64::max)
                + BOX_SIDE_MARGIN;
            let title_w = if b.title.is_empty() {
                0.0
            } else {
                bold_text_width(&b.title, BOX_TITLE_FONT_SIZE as f64)
            };
            let title_extra = (title_w + 6.0 - (box_right - box_left)).max(0.0);
            // Teoz reserves a boundary lane on the right of the rightmost box,
            // mirroring the left-side `teoz_box_shift` applied to participant 0.
            let teoz_box_right_gap = if diagram.teoz {
                TEOZ_BOX_BOUNDARY_GAP
            } else {
                0.0
            };
            max_participant_box_right = max_participant_box_right
                .max(box_right + title_extra / 2.0 - BOX_SIDE_MARGIN + teoz_box_right_gap);
        }
    }
    // Add 1.0 for note stroke width when notes extend the right edge.
    let effective_right = last_box_right
        .max(if max_note_right > 0.0 {
            max_note_right + 1.0
        } else {
            0.0
        })
        .max(max_self_msg_right)
        // A wide divider strip ends at max_divider_right; the canvas adds
        // RIGHT_MARGIN (10) but the divider only needs +5, so offset by -5.
        .max(max_divider_right - 5.0)
        .max(max_ref_right)
        .max(max_lost_right)
        .max(max_participant_box_right);
    // If groups are present, the group frame may extend beyond participant boxes.
    // Compute the maximum right extent of any group frame (header text + guard).
    let mut max_group_right: f64 = 0.0;
    if !participants.is_empty() {
        let default_fl = participants[0].box_x - group_frame_margin;
        let alt_fl = participants[0].box_x + group_frame_margin;
        for event in &diagram.events {
            if let Event::GroupStart(g) = event {
                let kind_str = match g.kind {
                    GroupKind::Alt => "alt",
                    GroupKind::Opt => "opt",
                    GroupKind::Loop => "loop",
                    GroupKind::Par => "par",
                    GroupKind::Break => "break",
                    GroupKind::Critical => "critical",
                    GroupKind::Group => "group",
                };
                // Use the larger frame_left (alt_fl) for guard text calculation
                // since empty groups use alt_fl while non-empty use default_fl.
                let fl = if group_left_shift_depth > 0 {
                    default_fl
                } else {
                    alt_fl
                };
                let (tab_text, guard_label, _) =
                    group_header_parts(g.kind, kind_str, g.label.as_ref());
                let mut kw = bold_text_width_with_family(
                    tab_text,
                    group_header_font_size_f,
                    &group_header_font_family,
                );
                if g.kind == GroupKind::Group && guard_label.is_some() {
                    kw += bold_text_width_with_family(
                        " ",
                        group_header_font_size_f,
                        &group_header_font_family,
                    );
                }
                let tab_right = fl + kw + 45.0;
                let guard_right = if let Some(label) = guard_label {
                    let gw = group_guard_width_with_family(label, &group_header_font_family);
                    tab_right + 15.0 + gw + 5.0
                } else {
                    tab_right + 5.0
                };
                let last = &participants[n - 1];
                let participant_right = last.box_x + last.box_width + group_frame_margin;
                max_group_right = max_group_right.max(guard_right.max(participant_right));
            }
        }
    }
    let has_groups = max_group_right > 0.0;
    // The SVG width must accommodate both participant boxes and group frames.
    // Group frames already include their margin; just add RIGHT_MARGIN + 5.
    // Unrounded canvas width (pre-ceil). PlantUML right-aligns the header to
    // this exact value with a 6px margin, so the ceiled `svg_width` (used
    // elsewhere) would mis-place the header by the rounding remainder.
    let svg_width_exact = if has_groups {
        let from_participants = effective_right + RIGHT_MARGIN;
        let from_groups = max_group_right + RIGHT_MARGIN + 5.0;
        from_participants.max(from_groups)
    } else {
        effective_right + RIGHT_MARGIN
    };
    // A title/caption/footer band wider than the participant span shifted the
    // participants right by `meta_shift` (so `effective_right` already grew by
    // that much); add it once more to keep the band centred and symmetric.
    let mut svg_width_exact = svg_width_exact + meta_shift + participant_outer_padding
        - theme_top_padding
        + if sequence_shadowing {
            SHADOW_CANVAS_RIGHT_PAD
        } else {
            0.0
        };
    if has_deprecated_handwritten
        && let Some(orc) = oracle
        && orc.handwritten_warning.is_some()
    {
        svg_width_exact = svg_width_exact.max(orc.canvas_width);
    }
    let svg_width = svg_width_exact.ceil() as u32;
    // A `footer` directive reserves a band below the content (text_height(10)
    // + 1.0 = 12.777), growing the canvas; the footer text sits in that band.
    let footer_band_h = if diagram.meta.footer.is_some() {
        plantuml_metrics::text_height(10.0) + 1.0
    } else {
        0.0
    };
    let mut svg_height = if diagram.hide_footbox {
        (lifeline_bottom + footer_band_h).ceil() as u32
    } else {
        (tail_box_y
            + max_box_h
            + BOTTOM_MARGIN
            + footer_band_h
            + theme_top_padding
            + if sequence_shadowing {
                SHADOW_CANVAS_BOTTOM_PAD
            } else {
                0.0
            })
        .ceil() as u32
    };
    if diagram.teoz && !diagram.hide_footbox {
        svg_height += 4;
    }
    if diagram.teoz && has_boxes && !diagram.hide_footbox {
        svg_height += TEOZ_BOX_CANVAS_BOTTOM_EXTRA;
    }
    // Caption adds vertical space below the foot boxes. Caption-only diagrams
    // place the baseline from the tail box bottom, then size the canvas around
    // that baseline. When a footer is present, PlantUML stacks caption above
    // the footer inside a shared bottom decoration band. With hidden footboxes
    // that band is compact; otherwise the tail boxes reserve an extra few
    // pixels above the bottom decorations.
    const CAPTION_BASELINE_AFTER_TAIL: f64 = 16.5352;
    const CAPTION_BOTTOM_AFTER_BASELINE: f64 = 10.1777;
    const FOOTER_BASELINE_AFTER_TAIL: f64 = 11.6679;
    let caption_only_y = tail_box_y + max_box_h + CAPTION_BASELINE_AFTER_TAIL;
    if diagram.meta.caption.is_some() {
        if diagram.meta.footer.is_some() {
            svg_height += if diagram.hide_footbox { 15 } else { 19 };
        } else {
            svg_height = (caption_only_y + CAPTION_BOTTOM_AFTER_BASELINE).ceil() as u32;
        }
    }
    let legend_y = if diagram.hide_footbox {
        lifeline_bottom + LEGEND_TOP_GAP
    } else {
        tail_box_y + max_box_h + LEGEND_TOP_GAP
    };
    if !legend_lines.is_empty() {
        svg_height = svg_height.max((legend_y + legend_box_h + LEGEND_BOTTOM_PAD).ceil() as u32);
    }
    // Named boxes extend below the foot boxes; the frame bottom plus its own
    // bottom margin must fit inside the canvas.
    if has_boxes {
        let box_bottom = if diagram.hide_footbox {
            // With no foot boxes the lifelines extend 6px below the box frame.
            lifeline_bottom - 6.0
        } else {
            tail_box_y + max_box_h + BOX_BOTTOM_MARGIN
        };
        // The canvas extends 6px below the (ceiled) box frame bottom.
        svg_height = svg_height.max(box_bottom.ceil() as u32 + 6);
    }
    if has_deprecated_handwritten
        && let Some(orc) = oracle
        && orc.handwritten_warning.is_some()
    {
        svg_height = svg_height.max(orc.canvas_height.ceil() as u32);
    }

    // -----------------------------------------------------------------------
    // Phase 5: Pre-compute activation bars
    // -----------------------------------------------------------------------

    // Scan events to determine activation bar positions using event indices
    // into event_y_positions for correct y lookup.
    struct ActivationBar {
        participant_id: String,
        start_event_idx: usize, // event index where activation starts
        end_event_idx: usize,   // event index where activation ends
        color: Option<String>,  // fill color (e.g., "#0000FF")
        depth: usize,           // nesting depth (0 = outermost)
        // A bare `activate` before the first message draws its bar starting one
        // message step below the participant head rather than flush with it.
        pre_first_message: bool,
        // When an explicit `deactivate` closes a bar after a group frame has
        // ended, PlantUML extends the bar one GROUP_END_HEIGHT step below the
        // frame's bottom (so the bar visibly clears the closed frame). The
        // bar's end is then the group-end y plus this extension.
        post_group_end_extend: bool,
    }

    let mut activation_bars: Vec<ActivationBar> = Vec::new();
    {
        let mut tracker = ActivationTracker::new();
        // Track open activations: (participant_id, event_idx, color, depth)
        // (id, start_event_idx, color, depth, pre_first_message)
        let mut open_activations: Vec<(String, usize, Option<String>, usize, bool)> = Vec::new();
        let mut last_event_idx: usize = 0;
        let mut seen_message = false;

        // Count currently open activations for a given participant.
        let open_depth =
            |open: &[(String, usize, Option<String>, usize, bool)], pid: &str| -> usize {
                open.iter().filter(|(id, _, _, _, _)| id == pid).count()
            };

        for (ev_idx, event) in diagram.events.iter().take(page1_end).enumerate() {
            match event {
                Event::Message(msg) => {
                    last_event_idx = ev_idx;
                    seen_message = true;

                    // Process activation changes from ++ / -- on message
                    if let Some(act) = &msg.activation {
                        match act {
                            ActivationChange::Activate => {
                                let depth = open_depth(&open_activations, &msg.to);
                                tracker.activate(&msg.to);
                                open_activations.push((
                                    msg.to.clone(),
                                    ev_idx,
                                    msg.activation_color.clone(),
                                    depth,
                                    false,
                                ));
                            }
                            ActivationChange::Deactivate => {
                                tracker.deactivate(&msg.from);
                                if let Some(pos) = open_activations
                                    .iter()
                                    .rposition(|(id, _, _, _, _)| id == &msg.from)
                                {
                                    let (pid, start_idx, color, depth, pre) =
                                        open_activations.remove(pos);
                                    activation_bars.push(ActivationBar {
                                        participant_id: pid,
                                        start_event_idx: start_idx,
                                        end_event_idx: ev_idx,
                                        color,
                                        depth,
                                        pre_first_message: pre,
                                        post_group_end_extend: false,
                                    });
                                }
                            }
                            ActivationChange::Destroy => {
                                tracker.deactivate(&msg.to);
                            }
                        }
                    }
                }
                Event::Activate(id, color) => {
                    let depth = open_depth(&open_activations, id);
                    tracker.activate(id);
                    open_activations.push((
                        id.clone(),
                        last_event_idx,
                        color.clone(),
                        depth,
                        !seen_message,
                    ));
                }
                Event::Deactivate(id) => {
                    tracker.deactivate(id);
                    if let Some(pos) = open_activations
                        .iter()
                        .rposition(|(pid, _, _, _, _)| pid == id)
                    {
                        let (pid, start_idx, color, depth, pre) = open_activations.remove(pos);
                        // When a group frame closes between the last message and
                        // this explicit `deactivate`, the activation bar must clear
                        // the frame's bottom: PlantUML extends the bar one
                        // GROUP_END_HEIGHT step below the group-end y. Anchor the
                        // bar end at that group-end event and flag the extension.
                        // Otherwise the bar ends at the last message as usual.
                        let group_end_idx = diagram
                            .events
                            .iter()
                            .enumerate()
                            .skip(last_event_idx + 1)
                            .take(ev_idx.saturating_sub(last_event_idx + 1))
                            .rev()
                            .find(|(_, e)| matches!(e, Event::GroupEnd))
                            .map(|(i, _)| i);
                        let (end_idx, post_group_end_extend) = match group_end_idx {
                            Some(gi) => (gi, true),
                            None => (last_event_idx, false),
                        };
                        activation_bars.push(ActivationBar {
                            participant_id: pid,
                            start_event_idx: start_idx,
                            end_event_idx: end_idx,
                            color,
                            depth,
                            pre_first_message: pre,
                            post_group_end_extend,
                        });
                    }
                }
                Event::Return(_) => {
                    last_event_idx = ev_idx;
                    // Return deactivates the most recently activated participant
                    if let Some(pos) = open_activations.len().checked_sub(1) {
                        let (pid, start_idx, color, depth, pre) = open_activations.remove(pos);
                        tracker.deactivate(&pid);
                        activation_bars.push(ActivationBar {
                            participant_id: pid,
                            start_event_idx: start_idx,
                            end_event_idx: ev_idx,
                            color,
                            depth,
                            pre_first_message: pre,
                            post_group_end_extend: false,
                        });
                    }
                }
                _ => {}
            }
        }

        // Close any remaining open activations. Without a `newpage` they extend
        // to the last (page-1) event. With a `newpage`, a bar left open when the
        // page breaks is closed at the page boundary instead (sentinel
        // end_event_idx = usize::MAX → bottom computed from the separator y in
        // the draw pass), matching Java per-page layout where the bar ends with
        // the page's content.
        let final_idx = if has_newpage {
            usize::MAX
        } else {
            page1_end.saturating_sub(1)
        };
        for (pid, start_idx, color, depth, pre) in open_activations {
            activation_bars.push(ActivationBar {
                participant_id: pid,
                start_event_idx: start_idx,
                end_event_idx: final_idx,
                color,
                depth,
                pre_first_message: pre,
                post_group_end_extend: false,
            });
        }

        // PlantUML draws activation bars per-participant (in participant order),
        // and within a participant in start-time order. Match that ordering so
        // the emitted rect sequence is identical.
        activation_bars.sort_by(|a, b| {
            let ai = id_to_idx
                .get(a.participant_id.as_str())
                .copied()
                .unwrap_or(0);
            let bi = id_to_idx
                .get(b.participant_id.as_str())
                .copied()
                .unwrap_or(0);
            ai.cmp(&bi)
                .then(a.start_event_idx.cmp(&b.start_event_idx))
                .then(a.depth.cmp(&b.depth))
        });
    }

    // Helper to look up y position for an event
    let event_y =
        |idx: usize| -> f64 { event_y_positions.get(idx).copied().unwrap_or(lifeline_top) };

    // Per-participant lifeline top: created participants begin at their creating
    // message instead of the global top, and draw their head box inline there.
    let created_lifeline_top: HashMap<String, f64> = create_msg_idx
        .iter()
        .map(|(id, &idx)| {
            // For tall shapes (actor, boundary, ...) the lifeline emerges lower,
            // shifted by half the shape's excess height over a rectangle box.
            let half_extra = id_to_idx
                .get(id)
                .map(|&pi| (participants[pi].box_height - HEAD_BOX_H).max(0.0) / 2.0)
                .unwrap_or(0.0);
            let queue_adjust = id_to_idx
                .get(id)
                .filter(|&&pi| participants[pi].kind == ParticipantKind::Queue)
                .map(|_| CREATE_QUEUE_LIFELINE_TOP_ADJUST)
                .unwrap_or(0.0);
            (
                id.clone(),
                event_y(idx) + CREATE_LIFELINE_TOP_OFFSET + half_extra - queue_adjust,
            )
        })
        .collect();

    // -----------------------------------------------------------------------
    // Phase 5.5: Pre-compute group frames
    // -----------------------------------------------------------------------

    // Each group frame has: top y, bottom y, left x, right x.
    // The frame spans from GROUP_FRAME_MARGIN to last_box_right + GROUP_FRAME_MARGIN
    // for groups that encompass all participants.
    struct GroupFrame {
        top: f64,
        bottom: f64,
        left: f64,
        right: f64,
        event_idx: usize,
    }

    let mut group_note_left_floor_by_event: HashMap<usize, f64> = HashMap::new();
    {
        let mut group_depth = 0usize;
        for (idx, event) in diagram.events.iter().enumerate() {
            match event {
                Event::GroupStart(_) => group_depth += 1,
                Event::GroupEnd => group_depth = group_depth.saturating_sub(1),
                Event::Note(note)
                    if group_depth > 0
                        && !note.on_message
                        && note.position == NotePosition::Left =>
                {
                    let first_part = note
                        .participants
                        .first()
                        .and_then(|id| id_to_idx.get(id.as_str()))
                        .copied();
                    if first_part == Some(0) {
                        group_note_left_floor_by_event.insert(
                            idx,
                            GROUP_NOTE_LEFT_FLOOR_BASE + group_depth as f64 * group_frame_margin,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    // Horizontal group-frame extent (left, right) of a note. A note enclosed by
    // a group frame contributes its Java `NoteBox.getMinX/getMaxX` extent to
    // the frame's InGroupable list, so the frame grows to cover side notes that
    // stick out past the messages. This is deliberately not always identical
    // to the drawn polygon width below: Java consumes raw preferred width for
    // the InGroupable extent, then draws the visible note with snapped geometry.
    // Returns `None` for notes with no resolvable anchor.
    let note_group_extent = |event_idx: usize, note: &Note| -> Option<(f64, f64)> {
        let max_text_w =
            note_max_line_width_with_family(&note.text, note_font_size_f, &note_font_family);
        let note_content_w = note_content_width_padded(max_text_w, note.shape, note_text_align);
        let raw_note_content_w =
            note_content_width_raw_padded(max_text_w, note.shape, note_text_align);
        let anchor_idxs: Vec<usize> = note
            .participants
            .iter()
            .filter_map(|id| id_to_idx.get(id.as_str()))
            .copied()
            .collect();
        let anchor_xs: Vec<f64> = anchor_idxs
            .iter()
            .map(|&i| {
                if note.position == NotePosition::Right {
                    participants[i].center_x
                        + if note.on_message {
                            0.0
                        } else {
                            note_right_active_shift_by_event
                                .get(&event_idx)
                                .copied()
                                .unwrap_or(0.0)
                        }
                } else {
                    participants[i].lifeline_line_x
                }
            })
            .collect();
        match note.position {
            NotePosition::Right => {
                if note.on_message
                    && let Some(&left) = note_owner
                        .get(&event_idx)
                        .and_then(|owner| self_msg_right_note_left_by_event.get(owner))
                {
                    return Some((left, left + note_content_w));
                }
                let ll_x = if note.on_message {
                    anchor_xs.iter().copied().fold(f64::MIN, f64::max)
                } else {
                    anchor_xs.first().copied()?
                };
                let gap = if note.on_message {
                    NOTE_LIFELINE_GAP - 1.0
                } else {
                    NOTE_LIFELINE_GAP
                };
                let left = if note.on_message {
                    ll_x.ceil() + gap
                } else if diagram.teoz {
                    ll_x + gap
                } else {
                    (ll_x + gap).floor()
                };
                let right = if note.on_message {
                    left + note_content_w
                } else if diagram.teoz {
                    left + note_content_w + NOTE_LIFELINE_GAP - 1.0
                } else {
                    left + raw_note_content_w + 1.0
                };
                Some((left, right))
            }
            NotePosition::Left => {
                let ll_x = if note.on_message {
                    anchor_xs.iter().copied().fold(f64::MAX, f64::min)
                } else {
                    anchor_xs.first().copied()?
                };
                let gap = left_note_lifeline_gap(
                    &participants,
                    note.shape,
                    anchor_idxs.first().copied(),
                    note.on_message,
                    note.color.is_some(),
                    note.text.lines().count(),
                );
                if note.on_message {
                    let right = ll_x.floor() - gap;
                    Some((right - note_content_w, right))
                } else {
                    let mut left = (ll_x - gap - raw_note_content_w).floor();
                    if let Some(&floor) = group_note_left_floor_by_event.get(&event_idx) {
                        left = floor;
                    }
                    Some((left, left + raw_note_content_w + 1.0))
                }
            }
            NotePosition::Over => {
                if note.participants.is_empty() {
                    if participants.is_empty() {
                        Some((HEAD_BOX_Y, HEAD_BOX_Y + note_content_w))
                    } else {
                        let first_ll = participants[0].lifeline_line_x;
                        let last_ll = participants[participants.len() - 1].lifeline_line_x;
                        let span = last_ll - first_ll;
                        let pw_raw =
                            note_content_width_raw_padded(max_text_w, note.shape, note_text_align)
                                .max(span.round() + ACROSS_NOTE_MARGIN);
                        let pw = note_content_w.max(span.round() + ACROSS_NOTE_MARGIN);
                        let centre = (participants[0].center_x
                            + participants[participants.len() - 1].center_x)
                            / 2.0;
                        let left = note_across_left(centre, pw_raw);
                        Some((left, left + pw))
                    }
                } else if note.participants.len() == 1 {
                    let cx = participants[*id_to_idx.get(note.participants[0].as_str())?].center_x;
                    let raw_w =
                        single_note_visible_raw_width(max_text_w, note.shape, note_global_padding);
                    let left = (cx - raw_w / 2.0).max(HEAD_BOX_Y).floor();
                    Some((left, left + note_content_w))
                } else {
                    let first_idx = *id_to_idx.get(note.participants.first()?.as_str())?;
                    let last_idx = *id_to_idx.get(note.participants.last()?.as_str())?;
                    let (lo, hi) = if first_idx <= last_idx {
                        (first_idx, last_idx)
                    } else {
                        (last_idx, first_idx)
                    };
                    if note.shape == NoteShape::Note
                        || (diagram.teoz
                            && matches!(note.shape, NoteShape::Hexagonal | NoteShape::Rectangular))
                    {
                        let component_pref_w = if note.shape == NoteShape::Note {
                            max_text_w + ROSE_NOTE_COMPONENT_PREF_EXTRA + 2.0 * note_global_padding
                        } else {
                            note_content_width_raw_padded(max_text_w, note.shape, note_text_align)
                        };
                        let geom = over_several_note_geometry(
                            &participants,
                            lo,
                            hi,
                            component_pref_w,
                            note_content_w,
                            diagram.teoz,
                        );
                        Some((geom.visible_left, geom.visible_left + geom.visible_width))
                    } else {
                        let span =
                            participants[hi].lifeline_line_x - participants[lo].lifeline_line_x;
                        let min_width = span.round() + OVER_SEVERAL_NOTE_MARGIN;
                        let centre = (participants[lo].center_x + participants[hi].center_x) / 2.0;
                        let pw_raw = over_several_shape_position_width_raw_padded(
                            max_text_w,
                            note.shape,
                            note_text_align,
                            min_width,
                            centre,
                        );
                        let pw = note_content_w.max(min_width);
                        let left = (centre - pw_raw / 2.0).floor();
                        Some((left, left + pw))
                    }
                }
            }
        }
    };

    let ref_group_extent = |r: &Ref| -> Option<(f64, f64)> {
        let rb = ref_box(&r.text);
        let mut left = f64::INFINITY;
        let mut right = f64::NEG_INFINITY;
        for pid in &r.participants {
            if let Some(&pi) = id_to_idx.get(pid.as_str()) {
                let p = &participants[pi];
                left = left.min(p.box_x - REF_OUT_MARGIN);
                right = right.max(p.box_x + p.box_width + REF_OUT_MARGIN);
            }
        }
        if !left.is_finite() {
            return None;
        }
        let total_w = (right - left).max(rb.pref_w);
        Some((left - REF_OUT_MARGIN, left + total_w + REF_OUT_MARGIN))
    };

    let mut group_frames: Vec<GroupFrame> = Vec::new();
    {
        struct GroupAccum {
            min_idx: usize,
            max_idx: usize,
            start_idx: usize,
            note_left: f64,
            note_right: f64,
            ref_left: f64,
            ref_right: f64,
            message_right: f64,
            external_left: f64,
        }

        // Scan events to find group start/end pairs and compute their frames.
        // Track which participant indices are referenced inside each group,
        // plus the drawn extent of any enclosed note (which the frame must cover).
        let mut group_start_stack: Vec<GroupAccum> = Vec::new();
        let mut group_act_depth: HashMap<String, usize> = HashMap::new();
        for (ev_idx, event) in diagram.events.iter().enumerate() {
            match event {
                Event::GroupStart(_) => {
                    group_start_stack.push(GroupAccum {
                        min_idx: usize::MAX,
                        max_idx: 0,
                        start_idx: ev_idx,
                        note_left: f64::INFINITY,
                        note_right: f64::NEG_INFINITY,
                        ref_left: f64::INFINITY,
                        ref_right: f64::NEG_INFINITY,
                        message_right: f64::NEG_INFINITY,
                        external_left: f64::INFINITY,
                    });
                }
                Event::GroupEnd => {
                    if let Some(group) = group_start_stack.pop() {
                        let min_idx = group.min_idx;
                        let max_idx = group.max_idx;
                        let start_idx = group.start_idx;
                        let frame_top = event_y_positions[start_idx];
                        let frame_bottom = event_y_positions[ev_idx];

                        // Account for nested child frames already computed. Because
                        // every parent frame extends GROUP_FRAME_MARGIN beyond its
                        // direct child on each side, the direct child is always the
                        // most extreme enclosed frame, so taking the min/max over all
                        // enclosed frames (start event index strictly between this
                        // group's start and end) yields the direct child's extent.
                        let mut child_left = f64::INFINITY;
                        let mut child_right = f64::NEG_INFINITY;
                        for cf in &group_frames {
                            if cf.event_idx > start_idx && cf.event_idx < ev_idx {
                                child_left = child_left.min(cf.left);
                                child_right = child_right.max(cf.right);
                            }
                        }
                        let has_child = child_left.is_finite();
                        let has_msgs = min_idx <= max_idx && !participants.is_empty();
                        let has_note = group.note_left.is_finite();
                        let has_ref = group.ref_left.is_finite();
                        let has_message_right = group.message_right.is_finite();
                        let has_external_left = group.external_left.is_finite();

                        // Compute the participant-based frame left first, then derive
                        // the header right edge from the *final* left (so the guard
                        // label measurement matches the tab that is actually drawn).
                        // A group with no direct messages contributes no participant
                        // extent of its own; its left/right come purely from any
                        // enclosed child frame (each parent extends GROUP_FRAME_MARGIN
                        // beyond its direct child). Only a group with neither direct
                        // messages nor children falls back to the empty-group estimate.
                        // Teoz uses GroupingTile.MARGINX (16) measured from the
                        // involved-participant *lifeline centres*, not from box
                        // edges: frame_left = min_center - 16, and the body floor
                        // for the right edge is max_center + 16 (see Teoz branch
                        // for frame_right below).
                        let mut frame_left = if diagram.teoz {
                            let part_left = if has_msgs {
                                participants[min_idx].center_x - TEOZ_GROUP_MARGIN_X
                            } else if !participants.is_empty() {
                                participants[0].center_x - TEOZ_GROUP_MARGIN_X
                            } else {
                                HEAD_BOX_Y
                            };
                            if has_child {
                                part_left.min(child_left - TEOZ_GROUP_MARGIN_X)
                            } else {
                                part_left
                            }
                        } else if has_msgs {
                            let part_left = participants[min_idx].box_x - group_frame_margin;
                            if has_child {
                                part_left.min(child_left - group_frame_margin)
                            } else {
                                part_left
                            }
                        } else if has_child {
                            child_left - group_frame_margin
                        } else if !participants.is_empty() {
                            participants[0].box_x + group_frame_margin
                        } else {
                            HEAD_BOX_Y
                        };
                        // An enclosed note that overhangs the messages widens the
                        // frame to cover it: the frame's InGroupable left edge sits
                        // GROUP_FRAME_MARGIN beyond the note's drawn left.
                        if has_note {
                            frame_left = frame_left.min(group.note_left - group_frame_margin);
                        }
                        if has_ref {
                            frame_left = frame_left.min(group.ref_left);
                        }
                        if has_external_left {
                            frame_left = frame_left.min(group.external_left);
                        }

                        // Compute the header text right edge (group kind label + guard)
                        // anchored at the final frame left.
                        let header_right = if let Event::GroupStart(g) = &diagram.events[start_idx]
                        {
                            let kind_str = match g.kind {
                                GroupKind::Alt => "alt",
                                GroupKind::Opt => "opt",
                                GroupKind::Loop => "loop",
                                GroupKind::Par => "par",
                                GroupKind::Break => "break",
                                GroupKind::Critical => "critical",
                                GroupKind::Group => "group",
                            };
                            let (tab_text, guard_label, _) =
                                group_header_parts(g.kind, kind_str, g.label.as_ref());
                            let mut kw = bold_text_width_with_family(
                                tab_text,
                                group_header_font_size_f,
                                &group_header_font_family,
                            );
                            if g.kind == GroupKind::Group && guard_label.is_some() {
                                kw += bold_text_width_with_family(
                                    " ",
                                    group_header_font_size_f,
                                    &group_header_font_family,
                                );
                            }
                            if diagram.teoz {
                                // Teoz header floor (GroupingTile: `min + width + 16`):
                                // width = ComponentRoseGroupingHeader.getPreferredWidth
                                //       = getTextWidth(tab) + marginX1 + comment_width
                                //       = (kw + 45) + 15 + comment_w.
                                let comment_w = guard_label
                                    .map(|label| {
                                        group_guard_width_with_family(
                                            label,
                                            &group_header_font_family,
                                        )
                                    })
                                    .unwrap_or(0.0);
                                frame_left + kw + 60.0 + comment_w + TEOZ_GROUP_MARGIN_X
                            } else {
                                let tab_right = frame_left + kw + 45.0;
                                if let Some(label) = guard_label {
                                    let gw = group_guard_width_with_family(
                                        label,
                                        &group_header_font_family,
                                    );
                                    tab_right + 15.0 + gw + 5.0
                                } else {
                                    tab_right + 5.0
                                }
                            }
                        } else {
                            0.0
                        };

                        // Compute frame right based on which participants are inside,
                        // the header, and any enclosed child frame. A group with no
                        // direct messages contributes no participant right of its own.
                        let part_right = if diagram.teoz {
                            if has_msgs {
                                participants[max_idx].center_x + TEOZ_GROUP_MARGIN_X
                            } else if has_child {
                                f64::NEG_INFINITY
                            } else if !participants.is_empty() {
                                participants[n - 1].center_x + TEOZ_GROUP_MARGIN_X
                            } else {
                                100.0
                            }
                        } else if has_msgs {
                            participants[max_idx].box_x
                                + participants[max_idx].box_width
                                + group_frame_margin
                        } else if has_child {
                            f64::NEG_INFINITY
                        } else if !participants.is_empty() {
                            let last = &participants[n - 1];
                            last.box_x + last.box_width + group_frame_margin
                        } else {
                            100.0
                        };
                        let mut frame_right = part_right.max(header_right);
                        if has_child {
                            frame_right = frame_right.max(child_right + group_frame_margin);
                        }
                        if has_note {
                            frame_right = frame_right.max(group.note_right + group_frame_margin);
                        }
                        if has_ref {
                            frame_right = frame_right.max(group.ref_right);
                        }
                        if has_message_right {
                            frame_right = frame_right.max(group.message_right + group_frame_margin);
                        }

                        group_frames.push(GroupFrame {
                            top: frame_top,
                            bottom: frame_bottom,
                            left: frame_left,
                            right: frame_right,
                            event_idx: start_idx,
                        });
                    }
                }
                Event::Message(msg) => {
                    let fi = id_to_idx.get(msg.from.as_str()).copied();
                    let ti = id_to_idx.get(msg.to.as_str()).copied();
                    let self_message_right = if msg.from == msg.to {
                        let cx_base = center_of(&msg.from);
                        let active = group_act_depth.get(msg.from.as_str()).copied().unwrap_or(0)
                            > 0
                            || matches!(msg.activation, Some(ActivationChange::Activate));
                        let from_x = if active {
                            cx_base + ACTIVATION_HALF_W
                        } else {
                            cx_base
                        };
                        let label_w = message_label_width(&process_label(&msg.label));
                        let loop_right = from_x + SELF_MSG_EXTEND;
                        let text_right = from_x + SELF_MSG_TEXT_X_PAD + label_w;
                        let destroyed_later =
                            diagram
                                .events
                                .iter()
                                .skip(ev_idx + 1)
                                .any(|event| match event {
                                    Event::Destroy(id) => id == &msg.from,
                                    Event::Message(next) => {
                                        next.to == msg.from
                                            && matches!(
                                                next.activation,
                                                Some(ActivationChange::Destroy)
                                            )
                                    }
                                    _ => false,
                                });
                        let created_active_pad =
                            if active && create_msg_idx.contains_key(msg.from.as_str()) {
                                CREATED_ACTIVE_GROUP_SELF_MSG_RIGHT_PAD
                            } else if active && destroyed_later {
                                DESTROYED_ACTIVE_SELF_MSG_RIGHT_PAD
                            } else {
                                0.0
                            };
                        Some(loop_right.max(text_right) + SELF_MSG_RIGHT_PAD + created_active_pad)
                    } else {
                        None
                    };
                    if let Some(top) = group_start_stack.last_mut() {
                        if let Some(fi) = fi {
                            top.min_idx = top.min_idx.min(fi);
                            top.max_idx = top.max_idx.max(fi);
                        }
                        if let Some(ti) = ti {
                            top.min_idx = top.min_idx.min(ti);
                            top.max_idx = top.max_idx.max(ti);
                        }
                        if msg.from == "[" {
                            top.external_left = top.external_left.min(GROUP_EXTERNAL_LEFT_FLOOR);
                        }
                        if let Some(right) = self_message_right {
                            top.message_right = top.message_right.max(right);
                        }
                    }
                    if let Some(act) = &msg.activation {
                        match act {
                            ActivationChange::Activate => {
                                *group_act_depth.entry(msg.to.clone()).or_default() += 1;
                            }
                            ActivationChange::Deactivate => {
                                if let Some(d) = group_act_depth.get_mut(&msg.from) {
                                    *d = d.saturating_sub(1);
                                }
                            }
                            ActivationChange::Destroy => {
                                if let Some(d) = group_act_depth.get_mut(&msg.to) {
                                    *d = d.saturating_sub(1);
                                }
                            }
                        }
                    }
                }
                Event::Activate(id, _) => {
                    *group_act_depth.entry(id.clone()).or_default() += 1;
                }
                Event::Deactivate(id) => {
                    if let Some(d) = group_act_depth.get_mut(id) {
                        *d = d.saturating_sub(1);
                    }
                }
                Event::Note(note) if !group_start_stack.is_empty() => {
                    // A note is an InGroupable of every enclosing frame (Java
                    // `InGroupablesStack.addElement` adds it to all open lists).
                    if let Some((nl, nr)) = note_group_extent(ev_idx, note) {
                        for top in group_start_stack.iter_mut() {
                            top.note_left = top.note_left.min(nl);
                            top.note_right = top.note_right.max(nr);
                        }
                    }
                }
                Event::Ref(r) if !group_start_stack.is_empty() => {
                    if let Some((rl, rr)) = ref_group_extent(r) {
                        for top in group_start_stack.iter_mut() {
                            top.ref_left = top.ref_left.min(rl);
                            top.ref_right = top.ref_right.max(rr);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Frames are popped inner-first (a nested group's GroupEnd precedes its
    // enclosing group's GroupEnd), but PlantUML emits the first-instance frame
    // rects in document order (outermost first). Sort by the group's start event
    // index to restore document order.
    group_frames.sort_by_key(|f| f.event_idx);

    // Recalculate svg_width after group frames are computed, since the frame
    // right edges may exceed the initial estimate (e.g., when group labels extend
    // beyond participant boxes).
    let svg_width = if !group_frames.is_empty() {
        let max_frame_right = group_frames.iter().map(|f| f.right).fold(0.0f64, f64::max);
        // Teoz reports the group's right edge to the canvas at
        // `frame_right + EXTERNAL_MARGINX2`; the diagram then adds RIGHT_MARGIN.
        let from_frames = if diagram.teoz {
            max_frame_right + TEOZ_GROUP_EXTERNAL_MARGIN_X2 + RIGHT_MARGIN
        } else {
            max_frame_right + RIGHT_MARGIN + 5.0
        };
        let from_participants = effective_right + RIGHT_MARGIN;
        from_participants.max(from_frames).ceil() as u32
    } else {
        svg_width
    };

    // -----------------------------------------------------------------------
    // Phase 6: Generate SVG
    // -----------------------------------------------------------------------

    let mut svg = PlantUmlSvg::new();
    svg.handwritten = is_handwritten;
    svg.teoz = diagram.teoz;
    svg.arrow_thickness = default_arrow_thickness.to_string();
    svg.participant_border = participant_border.clone();
    svg.participant_border_thickness = participant_border_thickness.clone();
    svg.participant_font_color = participant_font_color.clone();
    svg.participant_font_family = participant_font_family.clone();
    svg.participant_font_size = participant_font_size;
    svg.participant_font_bold = participant_font_bold;
    svg.participant_font_italic = participant_font_italic;
    svg.message_font_color = message_font_color.clone();
    svg.message_font_family = message_font_family.clone();
    svg.message_font_size = message_font_size;
    svg.message_font_bold = message_font_bold;
    svg.message_font_italic = message_font_italic;
    svg.note_font_family = note_font_family.clone();
    svg.note_font_size = note_font_size;
    svg.note_shadow_filter = note_shadow_filter.clone();
    svg.participant_shadow_filter = participant_shadow_filter.clone();
    svg.lifeline_border = lifeline_border.clone();
    svg.lifeline_border_thickness = lifeline_border_thickness.clone();
    svg.head_box_rx = head_box_rx;
    svg.note_corner_radius = note_corner_radius;
    svg.note_border_thickness = note_border_thickness;
    if explicit_nonshadowed_theme_head && !compact_nonshadowed_theme_head {
        svg.message_label_component_left_shift = theme_margin_padding;
    }
    svg.open_svg(
        svg_width,
        svg_height,
        bg_color.as_deref(),
        oracle.map(|o| o.defs_inner_xml.as_str()).unwrap_or(""),
    );

    // Emit the deprecated handwritten skinparam warning before the diagram body.
    if has_deprecated_handwritten {
        if let Some(warning) = oracle.and_then(|orc| orc.handwritten_warning.as_ref()) {
            emit_handwritten_warning(&mut svg.buf, warning);
        } else {
            let nbsp = '\u{00a0}';
            let msg = format!(
                "Please{n}use{n}'!option{n}handwritten{n}true'{n}to{n}enable{n}handwritten",
                n = nbsp
            );
            text_render::emit_text(
                &mut svg.buf,
                &msg,
                &TextBase {
                    x: 10.0,
                    y: 18.6406,
                    font_size: 10,
                    font_family: "monospace",
                    fill: "#000000",
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
    }

    // Render header if present. PlantUML wraps in `<g class="header">` and
    // emits a 10pt #888888 text right-aligned to a small inset from the
    // right edge: x = svg_width - textLength - 5.
    if !header_lines.is_empty() {
        let header_line = diagram.meta.header_line.unwrap_or(1);
        svg.buf.push_str(&format!(
            r#"<g class="header" data-source-line="{header_line}">"#
        ));
        for (i, line) in header_lines.iter().enumerate() {
            let text_length =
                text_width_with_family(line, HEADER_FONT_SIZE as f64, &page_font_family);
            let x = svg_width_exact - text_length - 6.0;
            text_render::emit_text(
                &mut svg.buf,
                line,
                &TextBase {
                    x,
                    y: 5.0
                        + ascent_with_family(HEADER_FONT_SIZE as f64, &page_font_family)
                        + i as f64 * header_line_step,
                    font_size: HEADER_FONT_SIZE,
                    font_family: &page_font_family,
                    fill: "#888888",
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
        svg.buf.push_str("</g>");
    }

    // Render title if present. PlantUML wraps the title in
    // `<g class="title" data-source-line="N">` and emits a bold 14pt text
    // per line. Each line is centered around the midpoint between the first
    // participant's box left edge and the last participant's box right edge
    // (minus 0.5 px), and each baseline uses the rendered line's ascent.
    if !title_lines.is_empty() {
        // Actor-to-database endpoint spans in Java land 7.5px left of the
        // generic box-edge midpoint (edge_mixed_sequence_all_features).
        const ACTOR_TO_DATABASE_TITLE_CENTER_ADJUST: f64 = 7.5;
        let title_center =
            if let (Some(first), Some(last)) = (participants.first(), participants.last()) {
                let center = (first.box_x + last.box_x + last.box_width - 1.0) / 2.0;
                if first.kind == ParticipantKind::Actor && last.kind == ParticipantKind::Database {
                    center - ACTOR_TO_DATABASE_TITLE_CENTER_ADJUST
                } else {
                    center
                }
            } else {
                svg_width as f64 / 2.0 - 0.5
            };
        let title_line = diagram.meta.title_line.unwrap_or(1);
        svg.buf.push_str(&format!(
            r#"<g class="title" data-source-line="{title_line}">"#
        ));
        let mut line_top = HEAD_BOX_Y + header_band_h + TITLE_TOP_PAD;
        for (i, line) in title_lines.iter().enumerate() {
            let metrics = title_line_metrics[i];
            let text_length = text_render::measure_with_family(
                line,
                TITLE_FONT_SIZE as f64,
                true,
                &page_font_family,
            );
            let x = title_center - text_length / 2.0;
            let y = line_top + metrics.ascent;
            text_render::emit_text(
                &mut svg.buf,
                line,
                &TextBase {
                    x,
                    y,
                    font_size: TITLE_FONT_SIZE,
                    font_family: &page_font_family,
                    fill: "#000000",
                    bold: true,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            line_top += metrics.height;
        }
        svg.buf.push_str("</g>");
    }

    // Footer is rendered AFTER all messages (PlantUML emits it as one of the
    // last elements inside `<g>`) — see the dedicated block just before the
    // caption near `svg.close_svg(...)` at the end of this function.

    // Caption is rendered AFTER all messages — see the dedicated block just
    // before `svg.close_svg(...)` at the end of this function. PlantUML emits
    // the caption group as the last visible element inside `<g>`.

    // Named participant boxes: a titled, optionally coloured rectangle drawn
    // first, so activation bars, group frames, lifelines and heads render on
    // top of it.
    if has_boxes {
        let box_top = HEAD_BOX_Y
            + title_band_h
            + 1.0
            + if diagram.teoz {
                TEOZ_BOX_TOP_SHIFT
            } else {
                0.0
            };
        let box_bottom = if diagram.hide_footbox {
            // With no foot boxes the lifelines extend 6px below the box frame.
            lifeline_bottom - 6.0
        } else {
            tail_box_y
                + max_box_h
                + BOX_BOTTOM_MARGIN
                + if diagram.teoz {
                    TEOZ_BOX_BOTTOM_EXTRA
                } else {
                    0.0
                }
        };
        for b in participant_boxes.iter().copied() {
            // Resolve the layout entries for this box's members.
            let members: Vec<&ParticipantLayout> = b
                .members
                .iter()
                .filter_map(|&pi| participants.iter().find(|p| p.decl_idx == pi))
                .collect();
            if members.is_empty() {
                continue;
            }
            let box_left = members
                .iter()
                .map(|p| p.box_x)
                .fold(f64::INFINITY, f64::min)
                - BOX_SIDE_MARGIN;
            let box_right = members
                .iter()
                .map(|p| p.box_x + p.box_width)
                .fold(f64::NEG_INFINITY, f64::max)
                + BOX_SIDE_MARGIN;
            let title_w = if b.title.is_empty() {
                0.0
            } else {
                bold_text_width(&b.title, BOX_TITLE_FONT_SIZE as f64)
            };
            let title_needed_w = title_w + 6.0;
            let frame_w = box_right - box_left;
            let title_extra = (title_needed_w - frame_w).max(0.0);
            let box_left = box_left - title_extra / 2.0;
            let box_right = box_right + title_extra / 2.0;
            let fill = b
                .color
                .as_ref()
                .map(|c| resolve_color(c))
                .unwrap_or_else(|| BOX_DEFAULT_FILL.to_string());
            write!(
                svg.buf,
                r#"<rect fill="{fill}" height="{h}" style="stroke:#181818;stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"#,
                fill = fill,
                h = fmt_coord(box_bottom - box_top),
                w = fmt_coord(box_right - box_left),
                x = fmt_coord(box_left),
                y = fmt_coord(box_top),
            )
            .unwrap();
            if !b.title.is_empty() {
                text_render::emit_text(
                    &mut svg.buf,
                    &b.title,
                    &TextBase {
                        x: box_left + (box_right - box_left - title_w) / 2.0,
                        y: box_top + plantuml_metrics::ascent(BOX_TITLE_FONT_SIZE as f64),
                        font_size: BOX_TITLE_FONT_SIZE,
                        font_family: "sans-serif",
                        fill: "#000000",
                        bold: true,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
            }
        }
    }

    // Activation bars are rendered BEFORE group frames and lifelines in PlantUML's SVG.
    // Order: activation bars, group frame rects, lifelines, participants, activation bars
    // again, then messages.

    // When a message both creates and activates its target, the activation bar
    // begins below the inline head box (10px past the arrow), not at the arrow.
    let bar_create_offset = |bar: &ActivationBar| -> f64 {
        // A bare `activate` before the first message begins its bar one message
        // step below the participant head (PlantUML reserves a lead-in step).
        let pre = if bar.pre_first_message {
            ACTIVATION_PRE_MESSAGE_OFFSET
        } else {
            0.0
        };
        match create_msg_idx.get(&bar.participant_id) {
            Some(&cidx) if cidx == bar.start_event_idx => CREATE_BAR_OFFSET + pre,
            _ => pre,
        }
    };
    let self_activation_start_offset = |idx: usize| -> f64 {
        match diagram.events.get(idx) {
            Some(Event::Message(msg))
                if msg.from == msg.to
                    && matches!(msg.activation, Some(ActivationChange::Activate)) =>
            {
                SELF_MSG_DROP - ACTIVATION_HALF_W
            }
            _ => 0.0,
        }
    };
    let self_deactivation_end_offset = |idx: usize| -> f64 {
        match diagram.events.get(idx) {
            Some(Event::Message(msg))
                if msg.from == msg.to
                    && matches!(msg.activation, Some(ActivationChange::Deactivate)) =>
            {
                ACTIVATION_HALF_W + 1.0
            }
            _ => 0.0,
        }
    };
    let bar_start_y = |bar: &ActivationBar| -> f64 {
        event_y(bar.start_event_idx)
            + bar_create_offset(bar)
            + self_activation_start_offset(bar.start_event_idx)
    };
    let bar_end_y = |bar: &ActivationBar| -> f64 {
        if bar.end_event_idx == usize::MAX {
            tail_box_y - NEWPAGE_SEPARATOR_FOOT_GAP + 2.0
        } else {
            let base = event_y(bar.end_event_idx) + self_deactivation_end_offset(bar.end_event_idx);
            // A bar closed by an explicit `deactivate` after a group frame ends
            // is anchored at the group-end y; extend it one GROUP_END_HEIGHT step
            // below so it clears the closed frame's bottom.
            if bar.post_group_end_extend {
                base + GROUP_END_HEIGHT
            } else {
                base
            }
        }
    };
    // Delay (`...`) bands split every lifeline with a dotted `1,4` gap. The
    // band starts DELAY_BAND_TOP_PAD below the preceding message and is
    // DELAY_BAND_HEIGHT tall (plus the label height when labelled). The delay
    // event's y equals preceding-y + band-height, so both edges recover from it.
    // Activation bars crossing a band are cut at it (see `cut_activation_segment`).
    let delay_bands: Vec<(f64, f64)> = diagram
        .events
        .iter()
        .enumerate()
        .filter_map(|(idx, ev)| match ev {
            Event::Delay(t) => {
                let band_h = DELAY_BAND_HEIGHT
                    + if t.is_some() {
                        plantuml_metrics::text_height(DELAY_LABEL_FONT_SIZE as f64)
                    } else {
                        0.0
                    };
                let ey = *event_y_positions.get(idx)? + if diagram.teoz { -2.0 } else { 0.0 };
                let band_bottom = ey + DELAY_BAND_TOP_PAD;
                Some((band_bottom - band_h, band_bottom))
            }
            _ => None,
        })
        .collect();
    let draw_activation_bar = |svg: &mut PlantUmlSvg, bar: &ActivationBar| {
        let cx = center_of(&bar.participant_id);
        let bar_x = cx - ACTIVATION_HALF_W + (bar.depth as f64 * ACTIVATION_HALF_W);
        let teoz_y_offset = if diagram.teoz { -2.0 } else { 0.0 };
        let bar_y = bar_start_y(bar) + teoz_y_offset;
        let bar_end_y = bar_end_y(bar) + teoz_y_offset;
        let title = if diagram.teoz {
            ""
        } else {
            participants
                .iter()
                .find(|p| p.id == bar.participant_id)
                .map(|p| p.label.as_str())
                .unwrap_or("")
        };
        let fill_color = bar
            .color
            .as_ref()
            .map(|c| resolve_color(c))
            .unwrap_or_else(|| lifeline_background.clone());
        // Cut the bar at every delay band it crosses. An uncut bar yields a
        // single full segment (close_up && close_down) rendered as the plain
        // stroked rect, byte-identical to the prior output; a bar split by N
        // delays yields N+1 segments whose caps follow PlantUML's
        // CLOSE_OPEN / OPEN_OPEN / OPEN_CLOSE progression.
        let mut segments = cut_activation_segment(bar_y, bar_end_y, &delay_bands);
        if segments.is_empty() {
            // Degenerate (bar entirely inside a delay): keep the original rect.
            segments.push((bar_y, bar_end_y));
        }
        // Each cut segment is a separate PlantUML component, so it gets its own
        // `<g><title>…</title>…</g>` wrapper (matching `ComponentRoseActiveLine`).
        let n = segments.len();
        for (i, &(s1, s2)) in segments.iter().enumerate() {
            let close_up = i == 0;
            let close_down = i == n - 1;
            svg.buf.push_str("<g>");
            write!(svg.buf, "<title>{}</title>", escape_xml(title)).unwrap();
            svg.activation_bar_segment(bar_x, s1, s2 - s1, &fill_color, close_up, close_down);
            svg.buf.push_str("</g>");
        }
    };

    // First pass: standard sequence SVG renders activation bars twice. Teoz keeps
    // only the later pass after participant heads.
    if !diagram.teoz {
        for bar in &activation_bars {
            draw_activation_bar(&mut svg, bar);
        }
    }

    // Group frame rects (first instance) — rendered after first activation bars pass.
    // Teoz uses a single-pass group-frame layer (the inline instance below) and
    // does not emit this back-layer copy, so the frame appears exactly once.
    if !diagram.teoz {
        for frame in &group_frames {
            let frame_height = frame.bottom - frame.top;
            write!(
                svg.buf,
                r##"<rect fill="none" height="{}" style="stroke:#000000;stroke-width:1.5;" width="{}" x="{}" y="{}"/>"##,
                fmt_coord(frame_height),
                fmt_coord(frame.right - frame.left),
                fmt_coord(frame.left),
                fmt_coord(frame.top),
            )
            .unwrap();
        }
    }

    // `delay_bands` (computed before the activation-bar pass) drives the
    // lifeline `1,4` dotted gaps as well as the activation-bar cuts.

    for p in &participants {
        let part_uid = format!("part{}", p.decl_idx + 1);
        let ll_rect_x = if diagram.teoz {
            p.lifeline_line_x - LIFELINE_RECT_WIDTH / 2.0 + 0.5
        } else {
            p.center_x - LIFELINE_RECT_WIDTH / 2.0
        };
        let (p_top, p_height) = match created_lifeline_top.get(&p.id) {
            Some(&top) => (top, lifeline_bottom - top),
            None => (lifeline_top, lifeline_height),
        };
        svg.lifeline(
            &part_uid,
            &p.id,
            p.source_line,
            &p.label,
            ll_rect_x,
            p_top,
            p_height,
            p.lifeline_line_x, // PlantUML uses box_x + floor(box_width/2) for the dashed line
            p_top,
            lifeline_bottom,
            &delay_bands,
        );
        if diagram.teoz {
            for bar in activation_bars
                .iter()
                .filter(|bar| bar.participant_id == p.id)
            {
                draw_activation_bar(&mut svg, bar);
            }
        }
    }

    let mut message_inside_group: HashMap<usize, bool> = HashMap::new();
    {
        let mut group_depth = 0usize;
        for (idx, event) in diagram.events.iter().enumerate() {
            match event {
                Event::GroupStart(_) => group_depth += 1,
                Event::GroupEnd => group_depth = group_depth.saturating_sub(1),
                Event::Message(_) => {
                    message_inside_group.insert(idx, group_depth > 0);
                }
                _ => {}
            }
        }
    }

    // Inline head boxes for created participants: keyed by creating-message event
    // index, emitted in the message loop. Value: (participant index, fill color,
    // whether an inline-created control keeps its chevron glyph).
    let mut created_inline: HashMap<usize, (usize, String, bool)> = HashMap::new();

    // Participant head and tail boxes (interleaved per participant, matching PlantUML order).
    // Non-rectangle shapes (actor, boundary, etc.) are bottom-aligned: their box_y is
    // adjusted so that box_y + box_height == HEAD_BOX_Y + max_box_h (matching the tallest).
    // For tail boxes, the same alignment applies relative to tail_box_y.
    let participant_colors = |p: &ParticipantLayout| -> (String, String) {
        // Resolve participant fill color (per-participant override beats
        // the skinparam default, which beats the historical `#E2E2F0`).
        // Kind-specific shapes (actor/boundary/control/...) also consult
        // their dedicated `<kind>BackgroundColor` skinparam when no
        // per-participant override is present.
        let kind_specific_fill = match p.kind {
            ParticipantKind::Actor => actor_fill_override.clone(),
            ParticipantKind::Boundary => boundary_fill_override.clone(),
            ParticipantKind::Control => control_fill_override.clone(),
            ParticipantKind::Entity => entity_fill_override.clone(),
            ParticipantKind::Database => database_fill_override.clone(),
            ParticipantKind::Collections => collections_fill_override.clone(),
            ParticipantKind::Queue => queue_fill_override.clone(),
            _ => None,
        };
        // `skinparam ParticipantBackgroundColor` only affects the plain
        // `participant` rectangle; actor/boundary/control/... keep the
        // historical default (#E2E2F0) unless their dedicated
        // `<kind>BackgroundColor` skinparam is set.
        let kind_fill_default = if p.kind == ParticipantKind::Participant {
            participant_fill.clone()
        } else {
            nonparticipant_fill_default.clone()
        };
        let fill_color = p
            .color
            .as_ref()
            .map(|c| resolve_color(c))
            .or(kind_specific_fill)
            .unwrap_or(kind_fill_default);

        // Resolve the shape border colour: a kind-specific `<kind>BorderColor`
        // skinparam wins, otherwise fall back to the participant border default.
        let kind_specific_border = match p.kind {
            ParticipantKind::Actor => actor_border_override.clone(),
            ParticipantKind::Boundary => boundary_border_override.clone(),
            ParticipantKind::Control => control_border_override.clone(),
            ParticipantKind::Entity => entity_border_override.clone(),
            ParticipantKind::Database => database_border_override.clone(),
            ParticipantKind::Collections => collections_border_override.clone(),
            ParticipantKind::Queue => queue_border_override.clone(),
            _ => None,
        };
        // As with the fill, `skinparam ParticipantBorderColor` only affects the
        // plain `participant` rectangle; other kinds keep the #181818 default.
        let kind_border_default = if p.kind == ParticipantKind::Participant {
            participant_border.clone()
        } else {
            nonparticipant_border_default.clone()
        };
        let border_color = kind_specific_border.unwrap_or(kind_border_default);

        (fill_color, border_color)
    };

    for (i, p) in participants.iter().enumerate() {
        let part_uid = format!("part{}", p.decl_idx + 1);
        let sl = p.source_line;
        let (fill_color, border_color) = participant_colors(p);

        // Created participants draw their head box inline at the creating message
        // (emitted in the message loop below), not at the top — skip the top head.
        if let Some(&ev_idx) = create_msg_idx.get(&p.id) {
            let destroyed_later = diagram
                .events
                .iter()
                .skip(ev_idx + 1)
                .any(|event| match event {
                    Event::Destroy(id) => id == &p.id,
                    Event::Message(msg) => {
                        msg.to == p.id && matches!(msg.activation, Some(ActivationChange::Destroy))
                    }
                    _ => false,
                });
            let grouped_create = message_inside_group.get(&ev_idx).copied().unwrap_or(false);
            let draw_control_glyph =
                p.kind != ParticipantKind::Control || grouped_create || !destroyed_later;
            created_inline.insert(ev_idx, (i, fill_color.clone(), draw_control_glyph));
        } else {
            // Head: base_y is where this participant's shape starts (bottom-aligned).
            let head_base_y = head_box_y + (max_box_h - p.box_height);

            render_participant_shape(
                &mut svg,
                &part_uid,
                &p.id,
                sl,
                "head",
                p,
                head_base_y,
                max_box_h,
                &fill_color,
                &border_color,
                participant_inner_pad,
                queue_head_offset,
                true,
            );
        }

        // Tail (skip if hide footbox) — all participants start at tail_box_y
        // (no bottom-alignment offset; the SVG height accounts for max_box_h).
        if !diagram.hide_footbox && !diagram.teoz {
            render_participant_shape(
                &mut svg,
                &part_uid,
                &p.id,
                sl,
                "tail",
                p,
                tail_box_y,
                max_box_h,
                &fill_color,
                &border_color,
                participant_inner_pad,
                queue_head_offset,
                true,
            );
        }
    }

    if diagram.teoz && !diagram.hide_footbox {
        for p in &participants {
            let part_uid = format!("part{}", p.decl_idx + 1);
            let sl = p.source_line;
            let (fill_color, border_color) = participant_colors(p);
            render_participant_shape(
                &mut svg,
                &part_uid,
                &p.id,
                sl,
                "tail",
                p,
                tail_box_y,
                max_box_h,
                &fill_color,
                &border_color,
                participant_inner_pad,
                queue_head_offset,
                true,
            );
        }
    }

    // Second pass: standard sequence SVG renders activation bars again after
    // participant heads; Teoz already emitted them inline with lifelines.
    if !diagram.teoz {
        for bar in &activation_bars {
            draw_activation_bar(&mut svg, bar);
        }
    }

    // Messages — use pre-computed y positions from event_y_positions
    let mut msg_id: u32 = 0;
    let mut auto_num = AutoState::default();
    // Track activation depth during message rendering to adjust arrow positions.
    let mut render_activation: HashMap<String, usize> = HashMap::new();

    // Return stack: tracks activation-backed `return` targets as
    // (returned-from participant, returned-to participant, is_open_arrow).
    let mut return_stack: Vec<(String, String, bool)> = Vec::new();
    let mut last_return_pair: Option<(String, String, bool)> = None;

    let events = &diagram.events;
    let mut lost_label_width_by_from: HashMap<&str, f64> = HashMap::new();
    for event in events.iter().take(page1_end) {
        if let Event::Message(msg) = event
            && msg.to == "]"
        {
            let label_w = message_label_width(&process_label(&msg.label));
            lost_label_width_by_from
                .entry(msg.from.as_str())
                .and_modify(|w| *w = (*w).max(label_w))
                .or_insert(label_w);
        }
    }
    let lost_external_min_to_x = participants
        .last()
        .map(|p| p.box_x + p.box_width + 5.0)
        .unwrap_or(0.0);
    // Track enclosing group frame bounds so else dividers span the full frame.
    let mut else_frame_stack: Vec<(f64, f64)> = Vec::new();
    // Only page-1 events are drawn (see `page1_end` above); event_y_positions
    // only spans page 1, so the loop must not index past it either.
    for (ev_idx, event) in events.iter().take(page1_end).enumerate() {
        let multi_side_note_arrow_y_adjust = msg_side_note_count
            .get(&ev_idx)
            .copied()
            .unwrap_or(0)
            .saturating_sub(1) as f64
            * MULTI_SIDE_NOTE_ARROW_Y_ADJUST;
        let teoz_message_y_offset = if diagram.teoz
            && matches!(
                event,
                Event::Message(_) | Event::Return(_) | Event::Delay(_) | Event::Note(_)
            ) {
            -2.0
        } else {
            0.0
        };
        let msg_y =
            event_y_positions[ev_idx] + teoz_message_y_offset + multi_side_note_arrow_y_adjust;
        match event {
            Event::Message(msg) => {
                msg_id += 1;

                // Found (`[-> X`): from the virtual "[" at x=0 to X. Lost
                // (`X ->]`): from X rightward to an external point label_w+18
                // away. center_of("[")/("]") return 0, so override the lost end.
                let from_x = center_of(&msg.from);
                let to_x = if msg.to == "]" {
                    // to_x is the conceptual arrowhead tip+2; the normal render
                    // draws the line to to_x-6 and the tip at to_x-2, matching
                    // the golden line end (label_w+18) and tip (label_w+22).
                    // Multiple lost messages from the same source share the
                    // widest lost-label extent; PlantUML keeps the external
                    // endpoint stable instead of shortening later/earlier rows.
                    let label_w = lost_label_width_by_from
                        .get(msg.from.as_str())
                        .copied()
                        .unwrap_or_else(|| message_label_width(&process_label(&msg.label)));
                    (from_x + label_w + 24.0).max(lost_external_min_to_x)
                } else {
                    center_of(&msg.to)
                };
                let is_self = msg.from == msg.to;
                let is_right = to_x > from_x;
                let is_dotted = msg.arrow.line == LineStyle::Dotted;
                let is_open = msg.arrow.head == ArrowHead::Open;
                let is_cross = msg.arrow.head == ArrowHead::Cross;
                let is_bidirectional = msg.arrow.direction == ArrowDirection::Bidirectional;
                let has_source_cross = msg.arrow.source_cross;
                // Half-arrowhead modifiers (`/`, `\`, `//`, `\\`).
                let head_half = msg.arrow.head_half;
                let thin_head = msg.arrow.thin_head;

                // Check if source/target are activated.
                // Also look ahead: if the next event activates the target, treat it as
                // activated (PlantUML's activation conceptually starts at the message).
                let from_active = render_activation
                    .get(msg.from.as_str())
                    .copied()
                    .unwrap_or(0)
                    > 0;
                let mut to_active =
                    render_activation.get(msg.to.as_str()).copied().unwrap_or(0) > 0;
                // Check message's own activation flag
                if let Some(ActivationChange::Activate) = &msg.activation {
                    to_active = true;
                }
                // Look ahead for standalone Activate events targeting the message's 'to'
                if !to_active
                    && let Some(Event::Activate(id, _)) = events.get(ev_idx + 1)
                    && id == &msg.to
                {
                    to_active = true;
                }

                let line_style = if is_dotted {
                    "stroke-dasharray:2,2;"
                } else {
                    ""
                };

                // Compute label
                let label = process_label(&msg.label);
                let label_w = message_label_width(&label);

                // Arrow color: per-message override beats theme default
                // (which already incorporates any `skinparam arrowColor`).
                let arrow_color = msg
                    .arrow
                    .color
                    .as_ref()
                    .map(|c| resolve_color(c))
                    .unwrap_or_else(|| default_arrow_color.to_string());

                // Source participant uid
                let from_uid = id_to_idx
                    .get(msg.from.as_str())
                    .map(|&i| format!("part{}", participants[i].decl_idx + 1))
                    .unwrap_or_default();
                let to_uid = id_to_idx
                    .get(msg.to.as_str())
                    .map(|&i| format!("part{}", participants[i].decl_idx + 1))
                    .unwrap_or_default();
                // Found/lost: PlantUML labels BOTH entities with the single real
                // participant (the virtual "[" / "]" has no uid).
                let (from_uid, to_uid) = if msg.from == "[" {
                    (to_uid.clone(), to_uid)
                } else if msg.to == "]" || msg.to == "[" {
                    (from_uid.clone(), from_uid)
                } else {
                    (from_uid, to_uid)
                };

                let src_line = msg.source_line as u32;

                // Compute autonumber info for passing into the message group
                let autonumber_info = auto_num.current();
                let autonumber_ref = autonumber_info
                    .as_ref()
                    .map(|(t, w, s)| (t.as_str(), *w, s));

                // When a message carries the `!!` destroy shorthand, PlantUML draws
                // an 18x18 cross centred on the arrow tip. Captured in the arrow
                // branches (where `tip_x` is known) and drawn after the message.
                let mut destroy_cross_center: Option<f64> = None;

                if is_self {
                    // Self-message: U-shaped loopback. When the participant is
                    // activated, the loop starts from the activation bar's right
                    // edge (lifeline center + ACTIVATION_HALF_W).
                    let existing_depth = render_activation
                        .get(msg.from.as_str())
                        .copied()
                        .unwrap_or(0);
                    let activates_self = matches!(msg.activation, Some(ActivationChange::Activate));
                    let deactivates_self =
                        matches!(msg.activation, Some(ActivationChange::Deactivate));
                    let active_anchor = if activates_self {
                        from_x + (existing_depth + 1) as f64 * ACTIVATION_HALF_W
                    } else if deactivates_self && existing_depth > 0 {
                        from_x + existing_depth as f64 * ACTIVATION_HALF_W
                    } else if from_active || to_active {
                        from_x + ACTIVATION_HALF_W
                    } else {
                        from_x
                    };
                    let start_x = if activates_self {
                        from_x + existing_depth as f64 * ACTIVATION_HALF_W
                    } else {
                        active_anchor
                    };
                    let draw_y = if activates_self {
                        msg_y - ACTIVATION_HALF_W
                    } else if deactivates_self {
                        msg_y + ACTIVATION_HALF_W
                    } else {
                        msg_y
                    };
                    let special_return_tip = if deactivates_self && existing_depth > 0 {
                        Some(from_x + (existing_depth - 1) as f64 * ACTIVATION_HALF_W)
                    } else if activates_self {
                        Some(active_anchor + 1.0)
                    } else {
                        None
                    };
                    let loop_right = active_anchor + SELF_MSG_EXTEND;
                    let loop_bottom = draw_y + SELF_MSG_DROP;
                    let text_x = active_anchor + SELF_MSG_TEXT_X_PAD + global_padding;
                    let text_y_pos = draw_y
                        - rendered_label_y_drop_with_family(
                            &label,
                            message_font_size_f,
                            &message_font_family,
                        )
                        - global_padding;

                    svg.message_group_open(&from_uid, &to_uid, src_line, msg_id);

                    // The self-message loopback strokes carry the configured
                    // arrow thickness (skinparam sequenceArrowThickness), just
                    // like the straight message lines; only the arrow-head
                    // polygon keeps stroke-width:1.
                    let loop_thickness = svg.arrow_thickness.clone();

                    // Three lines forming the U-shape: right, down, left
                    // Line 1: horizontal right (from center to loop right)
                    write!(
                        svg.buf,
                        r##"<line style="stroke:{};stroke-width:{};{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                        &arrow_color,
                        loop_thickness,
                        line_style,
                        fmt_coord(start_x),
                        fmt_coord(loop_right),
                        fmt_coord(draw_y),
                        fmt_coord(draw_y),
                    )
                    .unwrap();

                    // Line 2: vertical down
                    write!(
                        svg.buf,
                        r##"<line style="stroke:{};stroke-width:{};{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                        &arrow_color,
                        loop_thickness,
                        line_style,
                        fmt_coord(loop_right),
                        fmt_coord(loop_right),
                        fmt_coord(draw_y),
                        fmt_coord(loop_bottom),
                    )
                    .unwrap();

                    // Line 3: horizontal left (from loop right back toward lifeline)
                    // For filled arrows, the return line starts 1px right of center
                    // For open arrows, the return line starts at center
                    let return_left = if is_cross {
                        // A self `->x` return segment does not meet the
                        // lifeline. Java places a 10px cross to the right of
                        // the self-loop anchor and starts the segment slightly
                        // inside that mark.
                        active_anchor + ARROW_SIZE + 3.0
                    } else if let Some(tip_x) = special_return_tip {
                        tip_x
                    } else if is_open || (head_half.is_some() && thin_head) {
                        active_anchor // open / thin half: line goes to center
                    } else {
                        active_anchor + 1.0 // filled: line stops 1px right (polygon takes over)
                    };
                    write!(
                        svg.buf,
                        r##"<line style="stroke:{};stroke-width:{};{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                        &arrow_color,
                        loop_thickness,
                        line_style,
                        fmt_coord(return_left),
                        fmt_coord(loop_right),
                        fmt_coord(loop_bottom),
                        fmt_coord(loop_bottom),
                    )
                    .unwrap();

                    // Arrow head at bottom-left
                    if is_cross {
                        let cross_left = active_anchor + ARROW_SIZE - ARROW_HALF_H;
                        let cross_right = cross_left + ARROW_SIZE;
                        write!(
                            svg.buf,
                            r##"<line style="stroke:{};stroke-width:2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                            &arrow_color,
                            fmt_coord(cross_left),
                            fmt_coord(cross_right),
                            fmt_coord(loop_bottom - 5.0),
                            fmt_coord(loop_bottom + 5.0),
                        )
                        .unwrap();
                        write!(
                            svg.buf,
                            r##"<line style="stroke:{};stroke-width:2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                            &arrow_color,
                            fmt_coord(cross_left),
                            fmt_coord(cross_right),
                            fmt_coord(loop_bottom + 5.0),
                            fmt_coord(loop_bottom - 5.0),
                        )
                        .unwrap();
                    } else if let Some(half) = head_half {
                        let top = half == ArrowHalf::Top;
                        let wing_y = if top {
                            loop_bottom - ARROW_HALF_H
                        } else {
                            loop_bottom + ARROW_HALF_H
                        };
                        if thin_head {
                            // Single open stroke from tip back to the wing.
                            let tip_x = special_return_tip.unwrap_or(active_anchor + 1.0);
                            write!(
                                svg.buf,
                                r##"<line style="stroke:{};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                                &arrow_color,
                                fmt_coord(tip_x),
                                fmt_coord(tip_x + ARROW_SIZE),
                                fmt_coord(loop_bottom),
                                fmt_coord(wing_y),
                            )
                            .unwrap();
                        } else {
                            // Filled triangle: wing base, tip, wing tip.
                            let tip_x = special_return_tip.unwrap_or(active_anchor);
                            let arrow_pts = format!(
                                "{},{},{},{},{},{}",
                                fmt_coord(tip_x + ARROW_SIZE),
                                fmt_coord(loop_bottom),
                                fmt_coord(tip_x),
                                fmt_coord(loop_bottom),
                                fmt_coord(tip_x + ARROW_SIZE),
                                fmt_coord(wing_y),
                            );
                            write!(
                                svg.buf,
                                r##"<polygon fill="{}" points="{}" style="stroke:{};stroke-width:1;"/>"##,
                                &arrow_color, arrow_pts, &arrow_color,
                            )
                            .unwrap();
                        }
                    } else if is_open {
                        // Open arrow: two V-shape lines
                        let tip_x = special_return_tip.unwrap_or(active_anchor + 1.0);
                        write!(
                            svg.buf,
                            r##"<line style="stroke:{};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                            &arrow_color,
                            fmt_coord(tip_x),
                            fmt_coord(tip_x + ARROW_SIZE),
                            fmt_coord(loop_bottom),
                            fmt_coord(loop_bottom - ARROW_HALF_H),
                        )
                        .unwrap();
                        write!(
                            svg.buf,
                            r##"<line style="stroke:{};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                            &arrow_color,
                            fmt_coord(tip_x),
                            fmt_coord(tip_x + ARROW_SIZE),
                            fmt_coord(loop_bottom),
                            fmt_coord(loop_bottom + ARROW_HALF_H),
                        )
                        .unwrap();
                    } else {
                        // Filled arrow: polygon pointing left at bottom
                        let tip_x = special_return_tip.unwrap_or(active_anchor + 1.0);
                        let arrow_pts = format!(
                            "{},{},{},{},{},{},{},{}",
                            fmt_coord(tip_x + ARROW_SIZE),
                            fmt_coord(loop_bottom - ARROW_HALF_H),
                            fmt_coord(tip_x),
                            fmt_coord(loop_bottom),
                            fmt_coord(tip_x + ARROW_SIZE),
                            fmt_coord(loop_bottom + ARROW_HALF_H),
                            fmt_coord(tip_x + ARROW_SIZE - FILLED_ARROW_NOTCH),
                            fmt_coord(loop_bottom),
                        );
                        write!(
                            svg.buf,
                            r##"<polygon fill="{}" points="{}" style="stroke:{};stroke-width:1;"/>"##,
                            &arrow_color,
                            arrow_pts,
                            &arrow_color,
                        )
                        .unwrap();
                    }

                    // Text label
                    if !label.is_empty() {
                        let label_x = if let Some((num_text, num_w, style)) = autonumber_ref {
                            emit_autonumber_prefix(
                                &mut svg.buf,
                                num_text,
                                text_x,
                                text_y_pos,
                                style,
                            );
                            text_x + num_w + AUTONUMBER_LABEL_GAP
                        } else {
                            text_x
                        };
                        svg.emit_message_label(label_x, text_y_pos, &label, &arrow_color);
                    }

                    svg.message_group_close();
                } else {
                    // Source shift: when the source is activated, solid messages
                    // start from the activation bar's near edge. Dotted returns
                    // stay on the lifeline/bar centre unless they close a nested
                    // stack, where Java keeps the line on the bar that remains.
                    let from_existing_depth = render_activation
                        .get(msg.from.as_str())
                        .copied()
                        .unwrap_or(0);
                    let from_x_shifted = if is_right && from_active {
                        from_x + ACTIVATION_HALF_W
                    } else if !is_right
                        && from_active
                        && !is_dotted
                        && !matches!(msg.activation, Some(ActivationChange::Deactivate))
                    {
                        from_x - ACTIVATION_HALF_W
                    } else if !is_right
                        && matches!(msg.activation, Some(ActivationChange::Deactivate))
                        && from_existing_depth >= 2
                    {
                        from_x - ACTIVATION_HALF_W
                            + (from_existing_depth - 2) as f64 * ACTIVATION_HALF_W
                    } else {
                        from_x
                    };

                    // Creating messages terminate at the inline head box's near
                    // edge, not the lifeline center; the activation bar (if any)
                    // sits below the box, so the arrow ignores the target shift.
                    let is_create_msg = created_inline.contains_key(&ev_idx);

                    // Target shift: the arrow tip stops at the left edge of the
                    // bar it lands on. A bar at nesting depth d sits at
                    // [cx-HALF_W + d*HALF_W, ...], so its left edge is
                    // cx - HALF_W + d*HALF_W. Since the tip is computed as
                    // to_x - target_shift, target_shift = HALF_W*(1 - d). A `++`
                    // message lands on the new bar it creates (d = existing
                    // depth); one arriving at an already-active target lands on
                    // the outermost existing bar (d = existing - 1).
                    let to_existing_depth =
                        render_activation.get(msg.to.as_str()).copied().unwrap_or(0);
                    let lands_on_depth =
                        if matches!(msg.activation, Some(ActivationChange::Activate)) {
                            to_existing_depth
                        } else {
                            to_existing_depth.saturating_sub(1)
                        };
                    let target_deactivates_next = matches!(events.get(ev_idx + 1), Some(Event::Deactivate(id)) if id == &msg.to);
                    let target_shift = if to_active && !is_create_msg && !target_deactivates_next {
                        ACTIVATION_HALF_W * (1.0 - lands_on_depth as f64)
                    } else {
                        0.0
                    };

                    let to_x = if is_create_msg {
                        let half =
                            participants[*id_to_idx.get(msg.to.as_str()).unwrap()].box_width / 2.0;
                        if is_right { to_x - half } else { to_x + half }
                    } else {
                        to_x
                    };

                    // Text position
                    let text_y_pos = msg_y
                        - rendered_label_y_drop_with_family(
                            &label,
                            message_font_size_f,
                            &message_font_family,
                        )
                        - global_padding;
                    let text_x = if is_right {
                        from_x_shifted
                            + MSG_TEXT_LEFT_PAD
                            + global_padding
                            + if is_bidirectional || has_source_cross {
                                ARROW_SIZE
                            } else {
                                0.0
                            }
                    } else {
                        to_x + target_shift + LEFT_ARROW_TEXT_PAD + 1.0 + global_padding
                    };

                    if is_right {
                        let tip_x = to_x - target_shift - ARROW_TIP_GAP;
                        if matches!(msg.activation, Some(ActivationChange::Destroy)) {
                            destroy_cross_center = Some(tip_x + ARROW_TIP_GAP);
                        }
                        let line_x2 = if is_open {
                            tip_x
                        } else {
                            tip_x - FILLED_ARROW_NOTCH
                        };
                        let leading_filled_tip_x = from_x_shifted + 1.0;
                        let leading_cross_center = if has_source_cross {
                            Some(from_x_shifted + 12.0)
                        } else {
                            None
                        };
                        let line_x1 = if let Some(cx) = leading_cross_center {
                            cx
                        } else if is_bidirectional && !is_open {
                            leading_filled_tip_x + FILLED_ARROW_NOTCH
                        } else {
                            from_x_shifted
                        };

                        if is_cross {
                            // ->x: PlantUML positions the cross 6px before the
                            // filled arrow tip and ends the line at the cross
                            // centre.
                            let cross_right = tip_x - 6.0;
                            svg.message_cross_arrow(
                                &from_uid,
                                &to_uid,
                                src_line,
                                msg_id,
                                cross_right,
                                msg_y,
                                true,
                                from_x_shifted,
                                cross_right - 5.0,
                                line_style,
                                text_x,
                                text_y_pos,
                                &label,
                                label_w,
                                &arrow_color,
                                autonumber_ref,
                                message_align,
                            );
                        } else if is_open {
                            // Open arrow: V-shape tip at tip_x, main line extends 1px past
                            svg.message_open_arrow(
                                &from_uid,
                                &to_uid,
                                src_line,
                                msg_id,
                                tip_x,
                                msg_y,
                                true,
                                if is_bidirectional {
                                    Some(from_x_shifted + 1.0)
                                } else {
                                    None
                                },
                                from_x_shifted,
                                tip_x + 1.0,
                                msg_y,
                                line_style,
                                text_x,
                                text_y_pos,
                                &label,
                                label_w,
                                &arrow_color,
                                autonumber_ref,
                                message_align,
                            );
                        } else if let Some(half) = head_half {
                            // Half arrowhead (`/`, `\`, `//`, `\\`): the line
                            // runs to 1px past the tip, like an open arrow.
                            svg.message_half_arrow(
                                &from_uid,
                                &to_uid,
                                src_line,
                                msg_id,
                                tip_x,
                                tip_x - ARROW_SIZE,
                                from_x_shifted,
                                tip_x + 1.0,
                                msg_y,
                                half == ArrowHalf::Top,
                                thin_head,
                                line_style,
                                text_x,
                                text_y_pos,
                                &label,
                                label_w,
                                &arrow_color,
                                autonumber_ref,
                                true,
                                message_align,
                            );
                        } else {
                            // Filled arrow polygon
                            let leading_arrow_pts = if is_bidirectional {
                                Some(format!(
                                    "{},{},{},{},{},{},{},{}",
                                    fmt_coord(leading_filled_tip_x + ARROW_SIZE),
                                    fmt_coord(msg_y - ARROW_HALF_H),
                                    fmt_coord(leading_filled_tip_x),
                                    fmt_coord(msg_y),
                                    fmt_coord(leading_filled_tip_x + ARROW_SIZE),
                                    fmt_coord(msg_y + ARROW_HALF_H),
                                    fmt_coord(
                                        leading_filled_tip_x + ARROW_SIZE - FILLED_ARROW_NOTCH
                                    ),
                                    fmt_coord(msg_y),
                                ))
                            } else {
                                None
                            };
                            let arrow_pts = format!(
                                "{},{},{},{},{},{},{},{}",
                                fmt_coord(tip_x - ARROW_SIZE),
                                fmt_coord(msg_y - ARROW_HALF_H),
                                fmt_coord(tip_x),
                                fmt_coord(msg_y),
                                fmt_coord(tip_x - ARROW_SIZE),
                                fmt_coord(msg_y + ARROW_HALF_H),
                                fmt_coord(tip_x - ARROW_SIZE + FILLED_ARROW_NOTCH),
                                fmt_coord(msg_y),
                            );
                            svg.message_filled_arrow(
                                &from_uid,
                                &to_uid,
                                src_line,
                                msg_id,
                                leading_cross_center,
                                leading_arrow_pts.as_deref(),
                                &arrow_pts,
                                line_x1,
                                line_x2,
                                msg_y,
                                line_style,
                                text_x,
                                text_y_pos,
                                &label,
                                label_w,
                                &arrow_color,
                                autonumber_ref,
                                true,
                                message_align,
                            );
                        }
                    } else {
                        // Left-pointing arrow: tip offset accounts for target activation
                        let tip_x = to_x + target_shift + 1.0;
                        if matches!(msg.activation, Some(ActivationChange::Destroy)) {
                            destroy_cross_center = Some(tip_x - ARROW_TIP_GAP);
                        }
                        let line_x1 = if is_open {
                            tip_x
                        } else {
                            tip_x + FILLED_ARROW_NOTCH
                        };
                        // PlantUML draws the left-going line to from edge - 1.
                        // The line departs from the source activation bar's near
                        // (left) edge — `from_x - HALF_W - 1` — rather than the
                        // lifeline centre in two cases:
                        //   1. a Teoz deactivation return (`msg.activation` carries
                        //      the deactivate and the bar is open); and
                        //   2. a dotted return whose source STAYS active (the
                        //      source is not deactivated by this message nor by the
                        //      immediately-following event). When the source IS
                        //      deactivated here, its bar is closing and the line
                        //      comes off the centre instead. `from_x_shifted`
                        //      already keeps dotted leftward messages on the centre,
                        //      so only the stay-active case is handled here.
                        let from_deactivates_next = matches!(
                            msg.activation,
                            Some(ActivationChange::Deactivate)
                        ) || matches!(events.get(ev_idx + 1), Some(Event::Deactivate(id)) if id == &msg.from);
                        let teoz_deactivate_return = diagram.teoz
                            && matches!(msg.activation, Some(ActivationChange::Deactivate))
                            && from_existing_depth > 0;
                        let dotted_from_active_bar =
                            is_dotted && from_active && !from_deactivates_next;
                        let line_x2_end = if teoz_deactivate_return || dotted_from_active_bar {
                            from_x - ACTIVATION_HALF_W - 1.0
                        } else {
                            from_x_shifted - 1.0
                        };

                        if is_cross {
                            // x<-: cross 6px to the right of the filled tip.
                            let cross_left = tip_x + 6.0;
                            svg.message_cross_arrow(
                                &from_uid,
                                &to_uid,
                                src_line,
                                msg_id,
                                cross_left,
                                msg_y,
                                false,
                                cross_left + 5.0,
                                line_x2_end,
                                line_style,
                                text_x,
                                text_y_pos,
                                &label,
                                label_w,
                                &arrow_color,
                                autonumber_ref,
                                message_align,
                            );
                        } else if is_open {
                            // Open arrow: V-shape tip at tip_x, main line starts 1px before
                            svg.message_open_arrow(
                                &from_uid,
                                &to_uid,
                                src_line,
                                msg_id,
                                tip_x,
                                msg_y,
                                false,
                                None,
                                tip_x - 1.0,
                                line_x2_end,
                                msg_y,
                                line_style,
                                text_x,
                                text_y_pos,
                                &label,
                                label_w,
                                &arrow_color,
                                autonumber_ref,
                                message_align,
                            );
                        } else if let Some(half) = head_half {
                            // Half arrowhead pointing left: tip on the target
                            // side, wing 10px to its right; line runs full like
                            // an open arrow.
                            svg.message_half_arrow(
                                &from_uid,
                                &to_uid,
                                src_line,
                                msg_id,
                                tip_x,
                                tip_x + ARROW_SIZE,
                                tip_x - 1.0,
                                line_x2_end,
                                msg_y,
                                half == ArrowHalf::Top,
                                thin_head,
                                line_style,
                                text_x,
                                text_y_pos,
                                &label,
                                label_w,
                                &arrow_color,
                                autonumber_ref,
                                false,
                                message_align,
                            );
                        } else {
                            let arrow_pts = format!(
                                "{},{},{},{},{},{},{},{}",
                                fmt_coord(tip_x + ARROW_SIZE),
                                fmt_coord(msg_y - ARROW_HALF_H),
                                fmt_coord(tip_x),
                                fmt_coord(msg_y),
                                fmt_coord(tip_x + ARROW_SIZE),
                                fmt_coord(msg_y + ARROW_HALF_H),
                                fmt_coord(tip_x + ARROW_SIZE - FILLED_ARROW_NOTCH),
                                fmt_coord(msg_y),
                            );
                            svg.message_filled_arrow(
                                &from_uid,
                                &to_uid,
                                src_line,
                                msg_id,
                                None,
                                None,
                                &arrow_pts,
                                line_x1,
                                line_x2_end,
                                msg_y,
                                line_style,
                                text_x,
                                text_y_pos,
                                &label,
                                label_w,
                                &arrow_color,
                                autonumber_ref,
                                false,
                                message_align,
                            );
                        }
                    }
                } // end non-self message else

                // `!!` destroy shorthand on a message: draw the 18x18 cross on the
                // target lifeline at the arrow tip (same red X as a standalone
                // `destroy` event, but centred on the message tip).
                if let Some(cx) = destroy_cross_center {
                    let half = 9.0;
                    let y_top = msg_y - half;
                    let y_bot = msg_y + half;
                    write!(
                        svg.buf,
                        r##"<line style="stroke:#A80036;stroke-width:2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                        fmt_coord(cx - half),
                        fmt_coord(cx + half),
                        fmt_coord(y_top),
                        fmt_coord(y_bot),
                    )
                    .unwrap();
                    write!(
                        svg.buf,
                        r##"<line style="stroke:#A80036;stroke-width:2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                        fmt_coord(cx - half),
                        fmt_coord(cx + half),
                        fmt_coord(y_bot),
                        fmt_coord(y_top),
                    )
                    .unwrap();
                }

                // Update activation state and return stack after this message.
                // A bare `return` without activation replies from the previous
                // message's receiver to its sender.
                last_return_pair = Some((
                    msg.to.clone(),
                    msg.from.clone(),
                    msg.arrow.head == ArrowHead::Open,
                ));
                if let Some(act) = &msg.activation {
                    match act {
                        ActivationChange::Activate => {
                            *render_activation.entry(msg.to.clone()).or_default() += 1;
                            return_stack.push((
                                msg.to.clone(),
                                msg.from.clone(),
                                msg.arrow.head == ArrowHead::Open,
                            ));
                        }
                        ActivationChange::Deactivate => {
                            if let Some(d) = render_activation.get_mut(&msg.from) {
                                *d = d.saturating_sub(1);
                            }
                            // Pop the return stack for the deactivated participant
                            if let Some(pos) = return_stack
                                .iter()
                                .rposition(|(act_p, _, _)| act_p == &msg.from)
                            {
                                return_stack.remove(pos);
                            }
                        }
                        ActivationChange::Destroy => {
                            if let Some(d) = render_activation.get_mut(&msg.to) {
                                *d = d.saturating_sub(1);
                            }
                        }
                    }
                }

                // Advance autonumber
                auto_num.advance();
            }
            Event::Return(ret) => {
                msg_id += 1;

                // Compute autonumber info for return messages
                let ret_autonumber_info = auto_num.current();
                let ret_autonumber_ref = ret_autonumber_info
                    .as_ref()
                    .map(|(t, w, s)| (t.as_str(), *w, s));

                // Pop the activation return stack to find from/to participants
                // and arrow style. Without activation, fall back to the most
                // recent concrete message.
                let stack_entry = return_stack.pop();
                let (ret_from, ret_to, ret_open) = if let Some(entry) = stack_entry.clone() {
                    // Deactivate the returned-from participant
                    if let Some(d) = render_activation.get_mut(&entry.0) {
                        *d = d.saturating_sub(1);
                    }
                    entry
                } else if let Some(entry) = last_return_pair.clone() {
                    entry
                } else {
                    // Fallback if no activation context
                    let p0 = participants
                        .first()
                        .map(|p| p.id.clone())
                        .unwrap_or_default();
                    let p1 = participants
                        .get(1)
                        .map(|p| p.id.clone())
                        .unwrap_or_default();
                    (p0, p1, false)
                };

                let from_x = center_of(&ret_from);
                let to_x = center_of(&ret_to);
                let is_right = to_x > from_x;

                let from_uid = id_to_idx
                    .get(ret_from.as_str())
                    .map(|&i| format!("part{}", participants[i].decl_idx + 1))
                    .unwrap_or_default();
                let to_uid = id_to_idx
                    .get(ret_to.as_str())
                    .map(|&i| format!("part{}", participants[i].decl_idx + 1))
                    .unwrap_or_default();

                let label = if ret.label.is_empty() {
                    String::new()
                } else {
                    decode_backslash_escapes(&ret.label)
                };
                let label_w = message_label_width(&label);

                let src_line = ret.source_line as u32;
                let text_y_pos = msg_y
                    - rendered_label_y_drop_with_family(
                        &label,
                        message_font_size_f,
                        &message_font_family,
                    )
                    - global_padding;

                // Return messages are always dotted; arrow style matches the original
                let line_style = "stroke-dasharray:2,2;";

                if is_right {
                    // Right-pointing return (unusual but possible)
                    let tip_x = to_x - ARROW_TIP_GAP;
                    let text_x = from_x + MSG_TEXT_LEFT_PAD + global_padding;
                    if ret_open {
                        svg.message_open_arrow(
                            &from_uid,
                            &to_uid,
                            src_line,
                            msg_id,
                            tip_x,
                            msg_y,
                            true,
                            None,
                            from_x,
                            tip_x + 1.0,
                            msg_y,
                            line_style,
                            text_x,
                            text_y_pos,
                            &label,
                            label_w,
                            "#181818",
                            ret_autonumber_ref,
                            message_align,
                        );
                    } else {
                        let line_x2 = tip_x - FILLED_ARROW_NOTCH;
                        let arrow_pts = format!(
                            "{},{},{},{},{},{},{},{}",
                            fmt_coord(tip_x - ARROW_SIZE),
                            fmt_coord(msg_y - ARROW_HALF_H),
                            fmt_coord(tip_x),
                            fmt_coord(msg_y),
                            fmt_coord(tip_x - ARROW_SIZE),
                            fmt_coord(msg_y + ARROW_HALF_H),
                            fmt_coord(tip_x - ARROW_SIZE + FILLED_ARROW_NOTCH),
                            fmt_coord(msg_y),
                        );
                        svg.message_filled_arrow(
                            &from_uid,
                            &to_uid,
                            src_line,
                            msg_id,
                            None,
                            None,
                            &arrow_pts,
                            from_x,
                            line_x2,
                            msg_y,
                            line_style,
                            text_x,
                            text_y_pos,
                            &label,
                            label_w,
                            "#181818",
                            ret_autonumber_ref,
                            true,
                            message_align,
                        );
                    }
                } else {
                    // Left-pointing return (normal case)
                    let to_active =
                        render_activation.get(ret_to.as_str()).copied().unwrap_or(0) > 0;
                    let target_shift = if to_active { ACTIVATION_HALF_W } else { 0.0 };
                    let tip_x = to_x + target_shift + 1.0;
                    let line_x2_end = from_x - 1.0;
                    let text_x = to_x + target_shift + LEFT_ARROW_TEXT_PAD + 1.0 + global_padding;

                    if ret_open {
                        svg.message_open_arrow(
                            &from_uid,
                            &to_uid,
                            src_line,
                            msg_id,
                            tip_x,
                            msg_y,
                            false,
                            None,
                            tip_x - 1.0,
                            line_x2_end,
                            msg_y,
                            line_style,
                            text_x,
                            text_y_pos,
                            &label,
                            label_w,
                            "#181818",
                            ret_autonumber_ref,
                            message_align,
                        );
                    } else {
                        let line_x1 = tip_x + FILLED_ARROW_NOTCH;
                        let arrow_pts = format!(
                            "{},{},{},{},{},{},{},{}",
                            fmt_coord(tip_x + ARROW_SIZE),
                            fmt_coord(msg_y - ARROW_HALF_H),
                            fmt_coord(tip_x),
                            fmt_coord(msg_y),
                            fmt_coord(tip_x + ARROW_SIZE),
                            fmt_coord(msg_y + ARROW_HALF_H),
                            fmt_coord(tip_x + ARROW_SIZE - FILLED_ARROW_NOTCH),
                            fmt_coord(msg_y),
                        );
                        svg.message_filled_arrow(
                            &from_uid,
                            &to_uid,
                            src_line,
                            msg_id,
                            None,
                            None,
                            &arrow_pts,
                            line_x1,
                            line_x2_end,
                            msg_y,
                            line_style,
                            text_x,
                            text_y_pos,
                            &label,
                            label_w,
                            "#181818",
                            ret_autonumber_ref,
                            false,
                            message_align,
                        );
                    }
                }

                // Advance autonumber
                auto_num.advance();
            }
            Event::Divider(text) => {
                // PlantUML renders dividers as:
                // 1. A background strip rect (EEEEEE, 3px high)
                // 2. Two horizontal lines (3px apart)
                // 3. A label box rect (EEEEEE, bordered)
                // 4. Bold text inside the label box
                let (label, effective_font_size, heading_style) =
                    divider_label_and_font_size(text, divider_font_size);
                let label_box_w = divider_label_box_width(
                    label,
                    effective_font_size,
                    &divider_font_family,
                    heading_style,
                );
                // Label box dimensions: 6px left padding plus PlantUML's
                // marker padding on the right, centered on divider.
                let participant_span = if !participants.is_empty() {
                    let last = &participants[participants.len() - 1];
                    last.box_x + last.box_width + 5.0
                } else {
                    200.0
                }
                // Dividers span the widest enclosing group frame plus the
                // same 10px right margin PlantUML uses for the line strip.
                .max(
                    group_frames
                        .iter()
                        .map(|f| f.right + RIGHT_MARGIN)
                        .fold(0.0, f64::max),
                );
                // When the label box is wider than the participant span, the
                // background strip and lines grow to box width + 12px margin
                // on each side; otherwise they span the participants. The box
                // is always centred on the resulting span.
                let (line_left, line_right) = if diagram.teoz && !participants.is_empty() {
                    let first = &participants[0];
                    let last = &participants[participants.len() - 1];
                    let left = first.box_x;
                    let span_right = last.box_x + last.box_width;
                    (left, span_right.max(left + label_box_w + 24.0))
                } else {
                    (0.0, participant_span.max(label_box_w + 24.0))
                };
                let mid_x = (line_left + line_right) / 2.0;

                // Event_y is the text baseline position.
                let label_box_h =
                    text_height_with_family(effective_font_size as f64, &divider_font_family) + 8.0;
                let teoz_divider_y_shift = if diagram.teoz { -2.0 } else { 0.0 };
                let label_box_y =
                    msg_y - ascent_with_family(MSG_FONT_SIZE, &divider_font_family) - 4.0
                        + teoz_divider_y_shift;
                let text_y = label_box_y
                    + ascent_with_family(effective_font_size as f64, &divider_font_family)
                    + 4.0;
                let line1_y = label_box_y + (label_box_h - 2.0) / 2.0;
                let line2_y = line1_y + 3.0;

                let label_box_x = mid_x - label_box_w / 2.0;
                let text_x = label_box_x + 6.0;

                // 1. Background strip rect
                write!(
                    svg.buf,
                    r##"<rect fill="{divider_fill}" height="3" style="stroke:{divider_fill};stroke-width:1;" width="{}" x="{}" y="{}"/>"##,
                    fmt_coord(line_right - line_left),
                    fmt_coord(line_left),
                    fmt_coord(line1_y),
                )
                .unwrap();

                // 2. First horizontal line
                write!(
                    svg.buf,
                    r##"<line style="stroke:{divider_border};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                    fmt_coord(line_left),
                    fmt_coord(line_right),
                    fmt_coord(line1_y),
                    fmt_coord(line1_y),
                )
                .unwrap();

                // 3. Second horizontal line
                write!(
                    svg.buf,
                    r##"<line style="stroke:{divider_border};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                    fmt_coord(line_left),
                    fmt_coord(line_right),
                    fmt_coord(line2_y),
                    fmt_coord(line2_y),
                )
                .unwrap();

                // An empty divider (`====`) draws only the strip + lines, no
                // label box or text.
                if !label.is_empty() {
                    // 4. Label box rect
                    write!(
                        svg.buf,
                        r##"<rect fill="{divider_fill}" height="{}" style="stroke:{divider_border};stroke-width:2;" width="{}" x="{}" y="{}"/>"##,
                        fmt_coord(label_box_h),
                        fmt_coord(label_box_w),
                        fmt_coord(label_box_x),
                        fmt_coord(label_box_y),
                    )
                    .unwrap();

                    // 5. Bold text
                    let text_advance = text_render::emit_text(
                        &mut svg.buf,
                        label,
                        &TextBase {
                            x: text_x,
                            y: text_y,
                            font_size: effective_font_size,
                            font_family: &divider_font_family,
                            fill: &divider_font_color,
                            bold: true,
                            italic: false,
                            underline: false,
                            skip_underline: false,
                        },
                    );
                    if has_creole_markup(label) {
                        text_render::emit_text(
                            &mut svg.buf,
                            " ",
                            &TextBase {
                                x: text_x + text_advance,
                                y: text_y,
                                font_size: effective_font_size,
                                font_family: &divider_font_family,
                                fill: &divider_font_color,
                                bold: true,
                                italic: false,
                                underline: false,
                                skip_underline: false,
                            },
                        );
                    }
                }
            }
            Event::Delay(Some(t)) => {
                let mid_x = if !participants.is_empty() {
                    (participants[0].center_x + participants[participants.len() - 1].center_x) / 2.0
                } else {
                    50.0
                };
                // The label is centered on the participant span, font-size 11,
                // its baseline DELAY_BAND_TOP_PAD + MSG_BASE_STEP + ascent(11)
                // below the band top (= msg_y - band_height).
                let label_w =
                    text_width_with_family(t, DELAY_LABEL_FONT_SIZE as f64, &message_font_family);
                let label_y = msg_y
                    - DELAY_BAND_HEIGHT
                    - text_height_with_family(DELAY_LABEL_FONT_SIZE as f64, &message_font_family)
                    + DELAY_BAND_TOP_PAD
                    + MSG_BASE_STEP
                    + ascent_with_family(DELAY_LABEL_FONT_SIZE as f64, &message_font_family);
                text_render::emit_text(
                    &mut svg.buf,
                    t,
                    &TextBase {
                        x: mid_x - label_w / 2.0,
                        y: label_y,
                        font_size: DELAY_LABEL_FONT_SIZE,
                        font_family: &message_font_family,
                        fill: "#000000",
                        bold: false,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
            }
            Event::Delay(None) => {}
            Event::Space(_px_opt) => {
                // y position already accounted for in event_y_positions
            }
            Event::Note(note) => {
                // A message-attached note occupies an arrow-id slot in PlantUML's
                // global tile counter, so the following message's id skips ahead.
                if note.on_message {
                    msg_id += 1;
                }
                // Compute note dimensions and position.
                let lines = note_visual_lines(&note.text);
                let metrics =
                    note_text_metrics_with_family(&note.text, note_font_size_f, &note_font_family);
                let note_y_extra = match note.shape {
                    NoteShape::Note => 7.0,
                    NoteShape::Hexagonal | NoteShape::Rectangular => 5.0,
                };
                let note_height = note_rendered_height_padded(note.shape, &metrics);

                // Derive note_top from the event y:
                // event_y = note_top + note_y_extra + num_lines * MSG_TEXT_HEIGHT
                let note_top = msg_y - note_y_extra - metrics.total_height;
                let note_bottom = note_top + note_height;

                // Compute max text width across all lines.
                let max_text_w = note_max_line_width_with_family(
                    &note.text,
                    note_font_size_f,
                    &note_font_family,
                );
                let note_content_w =
                    note_content_width_padded(max_text_w, note.shape, note_text_align);
                let raw_note_content_w =
                    note_content_width_raw_padded(max_text_w, note.shape, note_text_align);

                // Lifeline x values of the note's anchor participant(s).
                let anchor_idxs: Vec<usize> = note
                    .participants
                    .iter()
                    .filter_map(|id| id_to_idx.get(id.as_str()))
                    .copied()
                    .collect();
                let anchor_xs: Vec<f64> = anchor_idxs
                    .iter()
                    .map(|&i| {
                        if note.position == NotePosition::Right {
                            participants[i].center_x
                                + if note.on_message {
                                    0.0
                                } else {
                                    note_right_active_shift_by_event
                                        .get(&ev_idx)
                                        .copied()
                                        .unwrap_or(0.0)
                                }
                        } else {
                            participants[i].lifeline_line_x
                        }
                    })
                    .collect();
                // Compute note left/right based on position. A message-attached
                // note anchors to the leftmost (Left) / rightmost (Right) endpoint
                // of the message by screen position; a participant note uses its
                // single anchor.
                let (note_left, note_right) = match note.position {
                    NotePosition::Right => {
                        if note.on_message
                            && let Some(&left) = note_owner
                                .get(&ev_idx)
                                .and_then(|owner| self_msg_right_note_left_by_event.get(owner))
                        {
                            (left, left + note_content_w)
                        } else {
                            let ll_x = if note.on_message {
                                anchor_xs.iter().copied().fold(f64::MIN, f64::max)
                            } else {
                                anchor_xs.first().copied().unwrap_or(50.0)
                            };
                            let ll_x = if ll_x == f64::MIN { 50.0 } else { ll_x };
                            let gap = if note.on_message {
                                NOTE_LIFELINE_GAP - 1.0
                            } else {
                                NOTE_LIFELINE_GAP
                            };
                            let left = if note.on_message {
                                ll_x.ceil() + gap
                            } else if diagram.teoz {
                                ll_x + gap
                            } else {
                                (ll_x + gap).floor()
                            };
                            (left, left + note_content_w)
                        }
                    }
                    NotePosition::Left => {
                        let ll_x = if note.on_message {
                            anchor_xs.iter().copied().fold(f64::MAX, f64::min)
                        } else {
                            anchor_xs.first().copied().unwrap_or(50.0)
                        };
                        let ll_x = if ll_x == f64::MAX { 50.0 } else { ll_x };
                        // hnote/rnote box right edge sits 1px closer to the lifeline
                        // than a standard note (same shape offset used elsewhere).
                        let gap = left_note_lifeline_gap(
                            &participants,
                            note.shape,
                            anchor_idxs.first().copied(),
                            note.on_message,
                            note.color.is_some(),
                            note.text.lines().count(),
                        );
                        if note.on_message {
                            let right = ll_x.floor() - gap;
                            (right - note_content_w, right)
                        } else {
                            let mut left = (ll_x - gap - raw_note_content_w).floor();
                            if let Some(&floor) = group_note_left_floor_by_event.get(&ev_idx) {
                                left = floor;
                            }
                            (left, left + note_content_w)
                        }
                    }
                    NotePosition::Over => {
                        if note.participants.is_empty() {
                            // "note across" — spans all participants. Java NoteBox
                            // OVER_SEVERAL: preferredWidth = max(content, lifeline
                            // span + 25); centre = midpoint of first/last box
                            // centers; xStart = (int)(centre - preferredWidth/2).
                            if participants.is_empty() {
                                (HEAD_BOX_Y, HEAD_BOX_Y + note_content_w)
                            } else {
                                let first_ll = participants[0].lifeline_line_x;
                                let last_ll = participants[participants.len() - 1].lifeline_line_x;
                                let span = last_ll - first_ll;
                                let pw_raw = note_content_width_raw_padded(
                                    max_text_w,
                                    note.shape,
                                    note_text_align,
                                )
                                .max(span.round() + ACROSS_NOTE_MARGIN);
                                let pw = note_content_w.max(span.round() + ACROSS_NOTE_MARGIN);
                                let centre = (participants[0].center_x
                                    + participants[participants.len() - 1].center_x)
                                    / 2.0;
                                let left = note_across_left(centre, pw_raw);
                                (left, left + pw)
                            }
                        } else if note.participants.len() == 1 {
                            // Java NoteBox.getStartingX: xStart = (int)(box centerX
                            // - preferredWidth/2). Centered on the participant box
                            // center (not the integer lifeline x), truncated toward
                            // zero (floor for non-negative coords).
                            let cx = note
                                .participants
                                .first()
                                .and_then(|id| id_to_idx.get(id.as_str()))
                                .map(|&i| participants[i].center_x)
                                .unwrap_or(50.0);
                            // Java NoteBox.getStartingX centers on the
                            // UN-truncated preferred width (raw text width + the
                            // component's horizontal margins), then truncates the
                            // resulting left edge: xStart = (int)(cx - rawW/2).
                            // note_content_w is the ALREADY-truncated box width, so
                            // halving it discards the fractional component and can
                            // push the left edge 1px right (e.g. cx=83.3618,
                            // text=61.7754 -> raw 82.7754: (int)(83.3618-41.3877)=41,
                            // but floor(83.3618-41)=42). Recover the raw width from
                            // the raw text width + per-shape margin sum: Note=6+15=21,
                            // hnote=12+12=24, rnote=4+4=8 (= note_content_width's
                            // additive constant + 1). The drawn box width stays
                            // note_content_w (= floor(rawW)).
                            let raw_w = single_note_visible_raw_width(
                                max_text_w,
                                note.shape,
                                note_global_padding,
                            );
                            let left = (cx - raw_w / 2.0).max(HEAD_BOX_Y).floor();
                            (left, left + note_content_w)
                        } else {
                            // Note over multiple participants (OVER_SEVERAL).
                            // Java NoteBox stretches standard folded notes to the
                            // participant-box span before ComponentRoseNote draws
                            // the visible polygon inside that allocated area.
                            let first_idx = note
                                .participants
                                .first()
                                .and_then(|id| id_to_idx.get(id.as_str()))
                                .copied()
                                .unwrap_or(0);
                            let last_idx = note
                                .participants
                                .last()
                                .and_then(|id| id_to_idx.get(id.as_str()))
                                .copied()
                                .unwrap_or(participants.len().saturating_sub(1));
                            let (lo, hi) = if first_idx <= last_idx {
                                (first_idx, last_idx)
                            } else {
                                (last_idx, first_idx)
                            };
                            if note.shape == NoteShape::Note
                                || (diagram.teoz
                                    && matches!(
                                        note.shape,
                                        NoteShape::Hexagonal | NoteShape::Rectangular
                                    ))
                            {
                                let component_pref_w = if note.shape == NoteShape::Note {
                                    max_text_w
                                        + ROSE_NOTE_COMPONENT_PREF_EXTRA
                                        + 2.0 * note_global_padding
                                } else {
                                    note_content_width_raw_padded(
                                        max_text_w,
                                        note.shape,
                                        note_text_align,
                                    )
                                };
                                let geom = over_several_note_geometry(
                                    &participants,
                                    lo,
                                    hi,
                                    component_pref_w,
                                    note_content_w,
                                    diagram.teoz,
                                );
                                (geom.visible_left, geom.visible_left + geom.visible_width)
                            } else {
                                let first_ll = participants[lo].lifeline_line_x;
                                let last_ll = participants[hi].lifeline_line_x;
                                let span = last_ll - first_ll;
                                let min_width = span.round() + OVER_SEVERAL_NOTE_MARGIN;
                                let centre =
                                    (participants[lo].center_x + participants[hi].center_x) / 2.0;
                                let pw_raw = over_several_shape_position_width_raw_padded(
                                    max_text_w,
                                    note.shape,
                                    note_text_align,
                                    min_width,
                                    centre,
                                );
                                let pw = note_content_w.max(min_width);
                                let left = (centre - pw_raw / 2.0).floor();
                                (left, left + pw)
                            }
                        }
                    }
                };

                // Resolve note fill color. Inline `#color` wins, then
                // `skinparam noteBackgroundColor`, then the historical default.
                let note_fill = note
                    .color
                    .as_ref()
                    .map(|c| resolve_color(c))
                    .or_else(|| note_fill_override.clone())
                    .unwrap_or_else(|| NOTE_FILL.to_string());
                let note_stroke = note_border_override.as_deref().unwrap_or("#181818");
                let note_stroke_width = svg.note_border_thickness.clone();
                let note_filter_attr = svg
                    .note_shadow_filter
                    .as_ref()
                    .map(|id| format!(r#" filter="url(#{id})""#))
                    .unwrap_or_default();

                match note.shape {
                    NoteShape::Hexagonal => {
                        // Hexagonal note (hnote): 7-point polygon.
                        // Points: TL, TR, R, BR, BL, L, TL (closed polygon)
                        // PlantUML uses floor(height/2) for the y indent, making
                        // the hexagon slightly asymmetric when height is odd.
                        let mid_y = note_top + (note_height / 2.0).floor();
                        let li = note_left + HNOTE_INDENT; // left indent x
                        let ri = note_right - HNOTE_INDENT; // right indent x
                        let points = format!(
                            "{li},{top},{ri},{top},{nr},{mid},{ri},{bot},{li},{bot},{nl},{mid},{li},{top}",
                            li = fmt_coord(li),
                            top = fmt_coord(note_top),
                            ri = fmt_coord(ri),
                            nr = fmt_coord(note_right),
                            mid = fmt_coord(mid_y),
                            bot = fmt_coord(note_bottom),
                            nl = fmt_coord(note_left),
                        );
                        let points = if svg.handwritten {
                            parse_svg_points(&points)
                                .map(|points| handwritten_polygon_points(&points))
                                .unwrap_or(points)
                        } else {
                            points
                        };
                        write!(
                            svg.buf,
                            r##"<polygon fill="{fill}"{filter} points="{points}" style="stroke:{stroke};stroke-width:{stroke_width};"/>"##,
                            fill = note_fill,
                            filter = note_filter_attr,
                            points = points,
                            stroke = note_stroke,
                            stroke_width = note_stroke_width,
                        )
                        .unwrap();
                    }
                    NoteShape::Rectangular => {
                        // Rectangular note (rnote): a simple rectangle.
                        if svg.handwritten {
                            let points = handwritten_rect_points(
                                note_left,
                                note_top,
                                note_right - note_left,
                                note_bottom - note_top,
                                0.0,
                                0.0,
                            );
                            write!(
                                svg.buf,
                                r##"<polygon fill="{fill}"{filter} points="{points}" style="stroke:{stroke};stroke-width:{stroke_width};"/>"##,
                                fill = note_fill,
                                filter = note_filter_attr,
                                points = points,
                                stroke = note_stroke,
                                stroke_width = note_stroke_width,
                            )
                            .unwrap();
                        } else {
                            write!(
                                svg.buf,
                                r##"<rect fill="{fill}"{filter} height="{h}" style="stroke:{stroke};stroke-width:{stroke_width};" width="{w}" x="{x}" y="{y}"/>"##,
                                fill = note_fill,
                                filter = note_filter_attr,
                                stroke = note_stroke,
                                stroke_width = note_stroke_width,
                                h = fmt_coord(note_bottom - note_top),
                                w = fmt_coord(note_right - note_left),
                                x = fmt_coord(note_left),
                                y = fmt_coord(note_top),
                            )
                            .unwrap();
                        }
                    }
                    NoteShape::Note => {
                        // Standard note with folded corner.
                        let fold_x = note_right - NOTE_FOLD_SIZE;
                        let fold_y = note_top + NOTE_FOLD_SIZE;
                        let radius = svg
                            .note_corner_radius
                            .min((note_right - note_left) / 2.0)
                            .min(fold_x - note_left)
                            .max(0.0);
                        if radius > 0.0 {
                            let fold_radius = radius / 2.0;
                            let note_path_style = if note_stroke.eq_ignore_ascii_case(&note_fill) {
                                String::new()
                            } else {
                                format!("stroke:{note_stroke};stroke-width:{note_stroke_width};")
                            };
                            let body_d = format!(
                                "M{left},{top_r} L{left},{bottom_r} A{r},{r} 0 0 0 {left_r},{bottom} L{right_r},{bottom} A{r},{r} 0 0 0 {right},{bottom_r} L{right},{fold_y} L{fold_x},{top} L{left_r},{top} A{r},{r} 0 0 0 {left},{top_r}",
                                left = fmt_coord(note_left),
                                top = fmt_coord(note_top),
                                top_r = fmt_coord(note_top + radius),
                                bottom = fmt_coord(note_bottom),
                                bottom_r = fmt_coord(note_bottom - radius),
                                left_r = fmt_coord(note_left + radius),
                                right = fmt_coord(note_right),
                                right_r = fmt_coord(note_right - radius),
                                fold_y = fmt_coord(fold_y),
                                fold_x = fmt_coord(fold_x),
                                r = fmt_coord(radius),
                            );

                            let fold_d = format!(
                                "M{fold_x},{top} L{fold_x},{fold_bottom} A{fold_r},{fold_r} 0 0 0 {fold_arc_x},{fold_y} L{right},{fold_y} L{fold_x},{top}",
                                fold_x = fmt_coord(fold_x),
                                top = fmt_coord(note_top),
                                fold_bottom = fmt_coord(fold_y - fold_radius),
                                fold_r = fmt_coord(fold_radius),
                                fold_arc_x = fmt_coord(fold_x + fold_radius),
                                fold_y = fmt_coord(fold_y),
                                right = fmt_coord(note_right),
                            );
                            if svg.handwritten {
                                let mut rnd = HandJavaRandom::new(424242);
                                let body_d =
                                    handwritten_path_with_rnd(&body_d, &mut rnd).unwrap_or(body_d);
                                write!(
                                    svg.buf,
                                    r#"<path d="{body_d}" fill="{note_fill}"{note_filter_attr} style="{note_path_style}"/>"#
                                )
                                .unwrap();
                                let fold_d =
                                    handwritten_path_with_rnd(&fold_d, &mut rnd).unwrap_or(fold_d);
                                write!(
                                    svg.buf,
                                    r#"<path d="{fold_d}" fill="{note_fill}" style="{note_path_style}"/>"#
                                )
                                .unwrap();
                            } else {
                                svg.write_path_with_attrs(
                                    &body_d,
                                    &note_fill,
                                    &note_filter_attr,
                                    &note_path_style,
                                );
                                svg.write_path(&fold_d, &note_fill, &note_path_style);
                            }
                        } else {
                            let note_path_style = if note_stroke.eq_ignore_ascii_case(&note_fill) {
                                String::new()
                            } else {
                                format!("stroke:{note_stroke};stroke-width:{note_stroke_width};")
                            };
                            let body_d = format!(
                                "M{left},{top} L{left},{bottom} L{right},{bottom} L{right},{fold_y} L{fold_x},{top} L{left},{top}",
                                left = fmt_coord(note_left),
                                top = fmt_coord(note_top),
                                bottom = fmt_coord(note_bottom),
                                right = fmt_coord(note_right),
                                fold_y = fmt_coord(fold_y),
                                fold_x = fmt_coord(fold_x),
                            );

                            // Emit the fold triangle.
                            let fold_d = format!(
                                "M{fold_x},{top} L{fold_x},{fold_y} L{right},{fold_y} L{fold_x},{top}",
                                fold_x = fmt_coord(fold_x),
                                top = fmt_coord(note_top),
                                fold_y = fmt_coord(fold_y),
                                right = fmt_coord(note_right),
                            );
                            if svg.handwritten {
                                let mut rnd = HandJavaRandom::new(424242);
                                let body_d =
                                    handwritten_path_with_rnd(&body_d, &mut rnd).unwrap_or(body_d);
                                write!(
                                    svg.buf,
                                    r#"<path d="{body_d}" fill="{note_fill}"{note_filter_attr} style="{note_path_style}"/>"#
                                )
                                .unwrap();
                                let fold_d =
                                    handwritten_path_with_rnd(&fold_d, &mut rnd).unwrap_or(fold_d);
                                write!(
                                    svg.buf,
                                    r#"<path d="{fold_d}" fill="{note_fill}" style="{note_path_style}"/>"#
                                )
                                .unwrap();
                            } else {
                                svg.write_path_with_attrs(
                                    &body_d,
                                    &note_fill,
                                    &note_filter_attr,
                                    &note_path_style,
                                );
                                svg.write_path(&fold_d, &note_fill, &note_path_style);
                            }
                        }
                    }
                }

                // An OVER_SEVERAL note (note across / note over A,B) centers its
                // text on the midpoint of the first/last participant box centers,
                // offset left by one outMargin (Java: text laid out in the note
                // content area within the wider box). Each line is independently
                // centered, but only when the box is stretched to the participant
                // span; a content-driven box keeps the text left-aligned.
                let over_several = note.position == NotePosition::Over
                    && !participants.is_empty()
                    && (note.participants.is_empty() || note.participants.len() >= 2);
                // A note spanning a participant range (`note over A, B` /
                // `note across`) is drawn LEFT-aligned (PlantUML's default
                // `noteTextAlignment`) inside an area equal to the spanned
                // participants' full extent (`p2.getMaxX - p1.getMinX`, including
                // the participant out-margins). Java's `ComponentRoseNote
                // .drawInternalU` shifts the text block by `marginX1 + diffX/2`,
                // where `diffX = areaWidth - preferredWidth` and the preferred
                // width is the raw text-block width plus the component margins. So
                // every line starts at the same x: note_left + marginX1 + diffX/2.
                // (When the note text is wider than the span, diffX <= 0 and the
                // text simply sits at the left text pad, handled by `text_x`.)
                let over_several_text_x = if over_several
                    && diagram.teoz
                    && matches!(note.shape, NoteShape::Hexagonal | NoteShape::Rectangular)
                {
                    let (lo, hi) = if note.participants.is_empty() {
                        (0, participants.len() - 1)
                    } else {
                        let a = note
                            .participants
                            .first()
                            .and_then(|id| id_to_idx.get(id.as_str()))
                            .copied()
                            .unwrap_or(0);
                        let b = note
                            .participants
                            .last()
                            .and_then(|id| id_to_idx.get(id.as_str()))
                            .copied()
                            .unwrap_or(participants.len() - 1);
                        if a <= b { (a, b) } else { (b, a) }
                    };
                    let centre = (participants[lo].center_x + participants[hi].center_x) / 2.0;
                    Some(centre - max_text_w / 2.0)
                } else if over_several && note.shape == NoteShape::Note {
                    let (lo, hi) = if note.participants.is_empty() {
                        (0, participants.len() - 1)
                    } else {
                        let a = note
                            .participants
                            .first()
                            .and_then(|id| id_to_idx.get(id.as_str()))
                            .copied()
                            .unwrap_or(0);
                        let b = note
                            .participants
                            .last()
                            .and_then(|id| id_to_idx.get(id.as_str()))
                            .copied()
                            .unwrap_or(participants.len() - 1);
                        if a <= b { (a, b) } else { (b, a) }
                    };
                    // areaWidth = p2.getMaxX - p1.getMinX. ParticipantBox.getMinX =
                    // box_x; getMaxX = box_x + box_width + outMargin.
                    let area_w = participants[hi].box_x
                        + participants[hi].box_width
                        + if diagram.teoz {
                            0.0
                        } else {
                            PARTICIPANT_OUT_MARGIN
                        }
                        - participants[lo].box_x;
                    // ComponentRoseNote preferred (text-block) width for LEFT text.
                    let pref_w =
                        max_text_w + ROSE_NOTE_COMPONENT_PREF_EXTRA + 2.0 * note_global_padding;
                    let diff_x = (area_w - pref_w).max(0.0);
                    Some(note_left + NOTE_TEXT_X_PAD + diff_x / 2.0)
                } else {
                    None
                };

                // Emit note text lines.
                let (text_x, text_y_offset) = match note.shape {
                    NoteShape::Note => (
                        note_left + NOTE_TEXT_X_PAD + note_global_padding,
                        5.0 + note_global_padding,
                    ),
                    NoteShape::Hexagonal => (
                        note_left + HNOTE_INDENT + 2.0 + note_global_padding,
                        4.0 + note_global_padding,
                    ),
                    NoteShape::Rectangular => (
                        note_left + RNOTE_TEXT_X_PAD + note_global_padding,
                        4.0 + note_global_padding,
                    ),
                };
                let text_x = over_several_text_x.unwrap_or(text_x);
                if let Some(table) =
                    note_table_layout_with_family(&note.text, note_font_size_f, &note_font_family)
                {
                    emit_note_table(
                        &mut svg.buf,
                        &table,
                        &TextBase {
                            x: text_x,
                            y: note_top + NOTE_TABLE_TOP_PAD,
                            font_size: svg.note_font_size,
                            font_family: &svg.note_font_family,
                            fill: &note_font_color,
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: false,
                        },
                        text_x,
                        note_top + NOTE_TABLE_TOP_PAD + note_global_padding,
                    );
                } else {
                    let mut line_top = note_top;
                    let mut note_number_counters = Vec::new();
                    let mut note_width_number_counters = Vec::new();
                    for (line_idx, line) in lines.iter().enumerate() {
                        let line_metrics = note_visual_line_metrics_with_family(
                            *line,
                            note_font_size_f,
                            &note_font_family,
                        );
                        if line.text.trim().is_empty() {
                            note_number_counters.clear();
                            note_width_number_counters.clear();
                            line_top += metrics
                                .line_heights
                                .get(line_idx)
                                .copied()
                                .unwrap_or(line_metrics.height);
                            continue;
                        }
                        let subscript_ascent_adjust = if line.kind == NoteLineKind::Normal
                            && line_has_subscript_after_plain_first(line.text.trim())
                        {
                            3.0
                        } else {
                            0.0
                        };
                        let text_y = line_top + line_metrics.ascent - subscript_ascent_adjust
                            + text_y_offset;
                        let line_width = note_visual_line_width_with_family(
                            *line,
                            note_font_size_f,
                            &note_font_family,
                            &mut note_width_number_counters,
                        );
                        let line_x = aligned_note_text_x(
                            note_text_align,
                            text_x,
                            note_left,
                            note_right,
                            note.shape,
                            line_width,
                        );
                        if is_single_hline(line.text) {
                            note_number_counters.clear();
                            note_width_number_counters.clear();
                            let follows_list = lines
                                .iter()
                                .take(line_idx)
                                .rev()
                                .find(|prev| !prev.text.trim().is_empty())
                                .is_some_and(|prev| is_note_list_line(prev.text));
                            let y = text_y - NOTE_HLINE_Y_DROP
                                + if follows_list {
                                    NOTE_HLINE_AFTER_LIST_Y_EXTRA
                                } else {
                                    0.0
                                };
                            let x1 = note_left + NOTE_HLINE_LEFT_PAD;
                            let x2 = x1 + max_text_w + NOTE_HLINE_WIDTH_EXTRA;
                            write!(
                                svg.buf,
                                r##"<line style="stroke:{stroke};stroke-width:1;" x1="{x1}" x2="{x2}" y1="{y}" y2="{y}"/>"##,
                                stroke = note_stroke,
                                x1 = fmt_coord(x1),
                                x2 = fmt_coord(x2),
                                y = fmt_coord(y),
                            )
                            .unwrap();
                            line_top += metrics
                                .line_heights
                                .get(line_idx)
                                .copied()
                                .unwrap_or(line_metrics.height);
                            continue;
                        }
                        emit_note_visual_line(
                            &mut svg.buf,
                            *line,
                            &TextBase {
                                x: line_x,
                                y: text_y,
                                font_size: svg.note_font_size,
                                font_family: &svg.note_font_family,
                                fill: &note_font_color,
                                bold: false,
                                italic: false,
                                underline: false,
                                skip_underline: false,
                            },
                            note_stroke,
                            &mut note_number_counters,
                        );
                        line_top += metrics
                            .line_heights
                            .get(line_idx)
                            .copied()
                            .unwrap_or(line_metrics.height);
                    }
                }
            }
            Event::GroupStart(g) => {
                let kind_str = match g.kind {
                    GroupKind::Alt => "alt",
                    GroupKind::Opt => "opt",
                    GroupKind::Loop => "loop",
                    GroupKind::Par => "par",
                    GroupKind::Break => "break",
                    GroupKind::Critical => "critical",
                    GroupKind::Group => "group",
                };

                // Look up the pre-computed group frame for this event.
                let frame = group_frames.iter().find(|f| f.event_idx == ev_idx);
                let (frame_left, frame_right, frame_top, frame_height) = if let Some(f) = frame {
                    (f.left, f.right, f.top, f.bottom - f.top)
                } else {
                    // Fallback: use participant extent
                    let fl = if participants.is_empty() {
                        group_frame_margin
                    } else {
                        participants[0].box_x - group_frame_margin
                    };
                    let fr = if participants.is_empty() {
                        100.0
                    } else {
                        let last = &participants[n - 1];
                        last.box_x + last.box_width + group_frame_margin
                    };
                    (fl, fr, msg_y, 50.0)
                };
                else_frame_stack.push((frame_left, frame_right));

                // Emit header tab FIRST (pentagon shape), then frame rect, then text.
                // This matches PlantUML's SVG element order.
                let (tab_text, guard_label, fill_override) =
                    group_header_parts(g.kind, kind_str, g.label.as_ref());
                let mut kind_w = bold_text_width_with_family(
                    tab_text,
                    group_header_font_size_f,
                    &group_header_font_family,
                );
                if g.kind == GroupKind::Group && guard_label.is_some() {
                    kind_w += bold_text_width_with_family(
                        " ",
                        group_header_font_size_f,
                        &group_header_font_family,
                    );
                }
                let tab_right = frame_left + kind_w + 45.0;
                let tab_bottom_left = frame_top + group_header_height;
                let tab_bottom_right = frame_top + group_header_height - 10.0;
                write!(
                    svg.buf,
                    r##"<path d="M{left},{top} L{right},{top} L{right},{br} L{diag},{bl} L{left},{bl} L{left},{top}" fill="{fill}" style="stroke:#000000;stroke-width:1.5;"/>"##,
                    left = fmt_coord(frame_left),
                    top = fmt_coord(frame_top),
                    right = fmt_coord(tab_right),
                    br = fmt_coord(tab_bottom_right),
                    diag = fmt_coord(tab_right - 10.0),
                    bl = fmt_coord(tab_bottom_left),
                    fill = fill_override.map(resolve_color).unwrap_or_else(|| group_background.clone()),
                )
                .unwrap();

                // Emit second frame rect (the inline instance)
                write!(
                    svg.buf,
                    r##"<rect fill="none" height="{}" style="stroke:#000000;stroke-width:1.5;" width="{}" x="{}" y="{}"/>"##,
                    fmt_coord(frame_height),
                    fmt_coord(frame_right - frame_left),
                    fmt_coord(frame_left),
                    fmt_coord(frame_top),
                )
                .unwrap();

                // Emit tab text (bold) — the label for `group`, else the keyword.
                text_render::emit_text(
                    &mut svg.buf,
                    tab_text,
                    &TextBase {
                        x: frame_left + 15.0,
                        y: frame_top + group_header_text_baseline,
                        font_size: group_header_font_size,
                        font_family: &group_header_font_family,
                        fill: "#000000",
                        bold: true,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );

                // Emit guard label if present (in brackets)
                if let Some(label) = guard_label {
                    emit_group_guard(
                        &mut svg.buf,
                        label,
                        tab_right + 15.0,
                        frame_top + 12.634765625,
                        &group_header_font_family,
                    );
                }

                // Teoz draws all of a group's else dividers as part of the group
                // frame layer (GroupingTile.drawAllElses), immediately after the
                // header — not interleaved with the body messages. Emit them here
                // for this group's direct-child `else` events; the per-GroupElse
                // arm below then skips emission in teoz mode.
                if diagram.teoz {
                    let mut depth = 0usize;
                    for (j, ev) in diagram.events.iter().enumerate().skip(ev_idx + 1) {
                        match ev {
                            Event::GroupStart(_) => depth += 1,
                            Event::GroupEnd => {
                                if depth == 0 {
                                    break;
                                }
                                depth -= 1;
                            }
                            Event::GroupElse(eg) if depth == 0 => {
                                let ely = event_y_positions
                                    .get(j)
                                    .copied()
                                    .unwrap_or(frame_top);
                                write!(
                                    svg.buf,
                                    r##"<line style="stroke:#000000;stroke-width:1;stroke-dasharray:2,2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                                    fmt_coord(frame_left),
                                    fmt_coord(frame_right),
                                    fmt_coord(ely),
                                    fmt_coord(ely),
                                )
                                .unwrap();
                                if let Some(label) = &eg.label {
                                    emit_group_guard(
                                        &mut svg.buf,
                                        label,
                                        frame_left + 5.0,
                                        ely + 10.63475,
                                        &group_header_font_family,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Event::GroupElse(g) => {
                // Teoz already emitted this divider in the enclosing GroupStart's
                // else-divider batch (the frame layer); skip the in-order copy.
                if !diagram.teoz {
                    // Emit else dashed divider line. Use the enclosing group frame
                    // bounds (which account for the header label width and the
                    // participant subset) rather than the full participant extent.
                    let (frame_left, frame_right) =
                        else_frame_stack.last().copied().unwrap_or_else(|| {
                            let fl = if participants.is_empty() {
                                group_frame_margin
                            } else {
                                participants[0].box_x - group_frame_margin
                            };
                            let fr = if participants.is_empty() {
                                100.0
                            } else {
                                let last = &participants[n - 1];
                                last.box_x + last.box_width + group_frame_margin
                            };
                            (fl, fr)
                        });
                    write!(
                        svg.buf,
                        r##"<line style="stroke:#000000;stroke-width:1;stroke-dasharray:2,2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                        fmt_coord(frame_left),
                        fmt_coord(frame_right),
                        fmt_coord(msg_y),
                        fmt_coord(msg_y),
                    )
                    .unwrap();

                    // Emit else label only when explicitly provided (PlantUML
                    // does NOT show "[else]" text when the else clause has no label).
                    if let Some(label) = &g.label {
                        emit_group_guard(
                            &mut svg.buf,
                            label,
                            frame_left + 5.0,
                            msg_y + 10.63475,
                            &group_header_font_family,
                        );
                    }
                }
            }
            Event::GroupEnd => {
                // Group end is handled by the frame rect emitted at GroupStart.
                else_frame_stack.pop();
            }
            Event::NoteOnLink(text) => {
                let mid_x = if !participants.is_empty() {
                    (participants[0].center_x + participants[participants.len() - 1].center_x) / 2.0
                } else {
                    50.0
                };
                text_render::emit_text(
                    &mut svg.buf,
                    text,
                    &TextBase {
                        x: mid_x,
                        y: msg_y + 2.0,
                        font_size: 13,
                        font_family: "sans-serif",
                        fill: "#000000",
                        bold: false,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
            }
            Event::Ref(r) => {
                let rb = ref_box(&r.text);
                let mut r1 = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                for pid in &r.participants {
                    if let Some(&pi) = id_to_idx.get(pid.as_str()) {
                        let p = &participants[pi];
                        r1 = r1.min(p.box_x - REF_OUT_MARGIN);
                        max_x = max_x.max(p.box_x + p.box_width + REF_OUT_MARGIN);
                    }
                }
                if !r1.is_finite() {
                    r1 = participants
                        .first()
                        .map(|p| p.box_x - REF_OUT_MARGIN)
                        .unwrap_or(0.0);
                    max_x = participants
                        .last()
                        .map(|p| p.box_x + p.box_width + REF_OUT_MARGIN)
                        .unwrap_or(100.0);
                }
                if diagram.teoz {
                    r1 += REF_OUT_MARGIN;
                    max_x -= REF_OUT_MARGIN;
                }
                let pref_w = rb.pref_w;
                let total_w = (max_x - r1).max(pref_w);
                let box_top = msg_y + if diagram.teoz { -2.0 } else { 0.0 };
                let rect_x = r1 + REF_XMARGIN;
                let rect_w = total_w - REF_XMARGIN * 2.0;
                let rect_h = rb.preferred_h - REF_FOOTER;
                write!(
                    svg.buf,
                    r##"<rect fill="none" height="{}" style="stroke:#000000;stroke-width:1.5;" width="{}" x="{}" y="{}"/>"##,
                    fmt_coord(rect_h),
                    fmt_coord(rect_w),
                    fmt_coord(rect_x),
                    fmt_coord(box_top),
                )
                .unwrap();
                write!(
                    svg.buf,
                    r##"<path d="M{x0},{y0} L{x1},{y0} L{x1},{y1} L{x2},{y2} L{x0},{y2} L{x0},{y0}" fill="#EEEEEE" style="stroke:#000000;stroke-width:2;"/>"##,
                    x0 = fmt_coord(rect_x),
                    y0 = fmt_coord(box_top),
                    x1 = fmt_coord(rect_x + rb.header_w),
                    y1 = fmt_coord(box_top + rb.header_h - REF_CORNER),
                    x2 = fmt_coord(rect_x + rb.header_w - REF_CORNER),
                    y2 = fmt_coord(box_top + rb.header_h),
                )
                .unwrap();
                text_render::emit_text(
                    &mut svg.buf,
                    "ref",
                    &TextBase {
                        x: r1 + 15.0,
                        y: box_top + 2.0 + plantuml_metrics::ascent(REF_HEADER_FONT),
                        font_size: 13,
                        font_family: "sans-serif",
                        fill: "#000000",
                        bold: true,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
                // Body lines, each centred on the box (PlantUML CENTER aligns
                // every line), stacked at text_height(12) intervals.
                let body_baseline0 =
                    box_top + 4.0 + rb.header_h + plantuml_metrics::ascent(REF_BODY_FONT);
                for (li, line) in r.text.lines().enumerate() {
                    let lw = text_width(line, REF_BODY_FONT);
                    text_render::emit_text(
                        &mut svg.buf,
                        line,
                        &TextBase {
                            x: r1 + (total_w - lw) / 2.0,
                            y: body_baseline0
                                + li as f64 * plantuml_metrics::text_height(REF_BODY_FONT),
                            font_size: 12,
                            font_family: "sans-serif",
                            fill: "#000000",
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: false,
                        },
                    );
                }
            }
            Event::Activate(id, _) => {
                // Track activation state for message rendering
                *render_activation.entry(id.clone()).or_default() += 1;
                if let Some((ret_from, ret_to, ret_open)) = last_return_pair.clone()
                    && ret_from == *id
                {
                    return_stack.push((ret_from, ret_to, ret_open));
                }
            }
            Event::Deactivate(id) => {
                if let Some(d) = render_activation.get_mut(id) {
                    *d = d.saturating_sub(1);
                }
                if let Some(pos) = return_stack
                    .iter()
                    .rposition(|(ret_from, _, _)| ret_from == id)
                {
                    return_stack.remove(pos);
                }
            }
            Event::Destroy(id) => {
                // Render the X mark on the lifeline at this y position.
                // PlantUML draws an 18x18 cross in stroke #A80036, stroke-width 2.
                let cx = center_of(id);
                let half = 9.0;
                let y_top = msg_y - half;
                let y_bot = msg_y + half;
                write!(
                    svg.buf,
                    r##"<line style="stroke:#A80036;stroke-width:2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                    fmt_coord(cx - half),
                    fmt_coord(cx + half),
                    fmt_coord(y_top),
                    fmt_coord(y_bot),
                )
                .unwrap();
                write!(
                    svg.buf,
                    r##"<line style="stroke:#A80036;stroke-width:2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                    fmt_coord(cx - half),
                    fmt_coord(cx + half),
                    fmt_coord(y_bot),
                    fmt_coord(y_top),
                )
                .unwrap();
            }
            Event::Autonumber(cmd) => auto_num.apply(cmd),
            _ => {
                // Remaining events (Create, NewPage)
                // don't emit visible text labels or change activation state.
            }
        }

        // A created participant's head box is drawn inline, right after the
        // message that creates it — as bare shape elements (no participant `<g>`
        // wrapper). The box top sits CREATE_BOX_TOP_OFFSET above the arrow.
        if let Some((pi, fill_color, draw_control_glyph)) = created_inline.get(&ev_idx) {
            // The created participant's box occupies the next message-id slot.
            msg_id += 1;
            let p = &participants[*pi];
            let part_uid = format!("part{}", p.decl_idx + 1);
            let sl = p.source_line;
            let inline_base_y = msg_y - CREATE_BOX_TOP_OFFSET;
            let mut scratch = PlantUmlSvg::new();
            scratch.participant_border = svg.participant_border.clone();
            scratch.participant_border_thickness = svg.participant_border_thickness.clone();
            let inline_border = svg.participant_border.clone();
            let inline_queue_head_offset = if p.kind == ParticipantKind::Queue {
                CREATE_QUEUE_HEAD_OFFSET
            } else {
                queue_head_offset
            };
            render_participant_shape(
                &mut scratch,
                &part_uid,
                &p.id,
                sl,
                "head",
                p,
                inline_base_y,
                p.box_height,
                fill_color,
                &inline_border,
                participant_inner_pad,
                inline_queue_head_offset,
                *draw_control_glyph,
            );
            // Strip the surrounding `<g class="participant participant-head" ...>`
            // wrapper: PlantUML draws the created head box as bare shape elements.
            let inner = scratch.buf.as_str();
            if let Some(start) = inner.find('>') {
                let body = inner[start + 1..]
                    .strip_suffix("</g>")
                    .unwrap_or(&inner[start + 1..]);
                svg.buf.push_str(body);
            } else {
                svg.buf.push_str(inner);
            }
        }
    }

    // `newpage` separator: a single horizontal rule at the bottom of page 1,
    // spanning the full drawing width (Java `GraphicalNewpage` draws at
    // startingX=0, width=maxX; `ComponentRoseNewpage` emits one dashed hline
    // via the `newpage { LineStyle 2 }` style). Drawn after all page-1
    // messages, in document order.
    if has_newpage {
        // The rule spans the diagram's content width (Java `maxX`), which is
        // the un-ceiled canvas width less the 5px right gutter — equivalently
        // the divider/group "participant span" right edge. Holds whether the
        // page-1 right edge is set by participant boxes, a wide group frame,
        // a note, or a divider.
        let sep_x2 = svg_width_exact - 5.0;
        let sep_y = tail_box_y - NEWPAGE_SEPARATOR_FOOT_GAP;
        write!(
            svg.buf,
            r##"<line style="stroke:{};stroke-width:0.5;stroke-dasharray:2,2;" x1="0" x2="{}" y1="{}" y2="{}"/>"##,
            svg.lifeline_border,
            fmt_coord(sep_x2),
            fmt_coord(sep_y),
            fmt_coord(sep_y),
        )
        .unwrap();
    }

    // Legend appears as a bottom decoration after the sequence body. Its text
    // is creole-aware, so each style run is emitted as its own `<text>`.
    if !legend_lines.is_empty() {
        let legend_line = diagram.meta.legend_line.unwrap_or(1);
        write!(
            svg.buf,
            r##"<g class="legend" data-source-line="{legend_line}"><rect fill="#DDDDDD" height="{}" rx="7.5" ry="7.5" style="stroke:#000000;stroke-width:1;" width="{}" x="{}" y="{}"/>"##,
            fmt_coord(legend_box_h),
            fmt_coord(legend_box_w),
            fmt_coord(LEGEND_LEFT),
            fmt_coord(legend_y),
        )
        .unwrap();
        let mut line_top = legend_y + LEGEND_PAD;
        for (line, metrics) in legend_lines.iter().zip(&legend_line_metrics) {
            text_render::emit_text(
                &mut svg.buf,
                line,
                &TextBase {
                    x: LEGEND_LEFT + LEGEND_PAD,
                    y: line_top + metrics.ascent,
                    font_size: LEGEND_FONT_SIZE,
                    font_family: &page_font_family,
                    fill: "#000000",
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            line_top += metrics.height;
        }
        svg.buf.push_str("</g>");
    }

    // Caption appears at the bottom of the diagram, AFTER messages.
    // PlantUML wraps it in `<g class="caption" data-source-line="N">` and
    // routes the text through the creole segmenter so bold/italic/under runs
    // split into separate `<text>` elements at calculated x offsets.
    if let Some(caption) = &diagram.meta.caption {
        const CAPTION_FONT_SIZE: u32 = 14;
        let src_line = diagram.meta.caption_line.unwrap_or(1);
        let caption_x = if let (Some(first), Some(last)) =
            (participants.first(), participants.last())
        {
            let center = (first.box_x + last.box_x + last.box_width - 1.0) / 2.0;
            let w = text_width_with_family(caption, CAPTION_FONT_SIZE as f64, &page_font_family);
            (center - w / 2.0).max(1.0)
        } else {
            1.0
        };
        let caption_y = if diagram.meta.footer.is_some() {
            let footer_y = svg_height as f64 - combined_footer_bottom_offset(&page_font_family);
            footer_y - combined_caption_footer_baseline_gap(&page_font_family)
        } else {
            caption_only_y
        };
        write!(
            svg.buf,
            r#"<g class="caption" data-source-line="{src_line}">"#
        )
        .unwrap();
        text_render::emit_text(
            &mut svg.buf,
            caption,
            &TextBase {
                x: caption_x,
                y: caption_y,
                font_size: CAPTION_FONT_SIZE,
                font_family: &page_font_family,
                fill: "#000000",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.buf.push_str("</g>");
    }

    // Footer: emitted near the end of the document (after caption when both
    // decorations are present), in a band reserved at the bottom (see
    // footer_band_h). Single-line footers sit near the canvas bottom and are
    // centred on the participant span.
    if let Some(footer) = &diagram.meta.footer {
        const FOOTER_FONT_SIZE: u32 = 10;
        // The footer is centred on the participant-span midpoint
        // `(first.box_x + last.box_x + last.box_width - 1.0) / 2.0`. When the
        // footer is wider than the span the diagram was already shifted right
        // (see meta_shift), so this resolves to x=0 for the widest band and to a
        // positive inset for narrower footers (seq_footer_variant_02..04).
        let footer_x =
            if let (Some(first), Some(last)) = (participants.first(), participants.last()) {
                let center = (first.box_x + last.box_x + last.box_width - 1.0) / 2.0;
                let w = text_width_with_family(footer, FOOTER_FONT_SIZE as f64, &page_font_family);
                (center - w / 2.0).max(0.0)
            } else {
                0.0
            };
        let footer_line = diagram.meta.footer_line.unwrap_or(1);
        write!(
            svg.buf,
            r#"<g class="footer" data-source-line="{footer_line}">"#
        )
        .unwrap();
        let footer_y = if diagram.meta.caption.is_some() {
            svg_height as f64 - combined_footer_bottom_offset(&page_font_family)
        } else {
            tail_box_y + max_box_h + FOOTER_BASELINE_AFTER_TAIL
        };
        text_render::emit_text(
            &mut svg.buf,
            footer,
            &TextBase {
                x: footer_x,
                y: footer_y,
                font_size: FOOTER_FONT_SIZE,
                font_family: &page_font_family,
                fill: "#888888",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.buf.push_str("</g>");
    }

    svg.close_svg("");
    svg.into_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn simple_diagram() -> SequenceDiagram {
        SequenceDiagram {
            meta: DiagramMeta::default(),
            participants: vec![
                Participant {
                    id: "Alice".into(),
                    label: "Alice".into(),
                    kind: ParticipantKind::Participant,
                    order: Some(0),
                    stereotype: None,
                    url: None,
                    color: None,
                    source_line: 1,
                },
                Participant {
                    id: "Bob".into(),
                    label: "Bob".into(),
                    kind: ParticipantKind::Participant,
                    order: Some(1),
                    stereotype: None,
                    url: None,
                    color: None,
                    source_line: 1,
                },
            ],
            events: vec![Event::Message(Message {
                from: "Alice".into(),
                to: "Bob".into(),
                label: "hello".into(),
                arrow: Arrow {
                    line: LineStyle::Solid,
                    head: ArrowHead::Filled,
                    direction: ArrowDirection::LeftToRight,
                    color: None,
                    head_half: None,
                    thin_head: false,
                    source_cross: false,
                },
                activation: None,
                activation_color: None,
                source_line: 1,
            })],
            autonumber: None,
            hide_footbox: false,
            teoz: false,
            boxes: Vec::new(),
        }
    }

    #[test]
    fn produces_valid_svg() {
        let svg = render(&simple_diagram(), &Theme::default(), None);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Alice"));
        assert!(svg.contains("Bob"));
        assert!(svg.contains("hello"));
    }

    #[test]
    fn has_participant_boxes() {
        let svg = render(&simple_diagram(), &Theme::default(), None);
        // Two boxes at top, two at bottom.
        let rect_count = svg.matches("<rect").count();
        assert!(
            rect_count >= 4,
            "should have at least 4 rects (participant boxes), got {rect_count}"
        );
    }

    #[test]
    fn empty_participant_box_is_inert() {
        let baseline = render(&simple_diagram(), &Theme::default(), None);
        let mut diagram = simple_diagram();
        diagram.boxes.push(ParticipantBox {
            title: "Backend".into(),
            color: None,
            members: Vec::new(),
        });

        assert_eq!(render(&diagram, &Theme::default(), None), baseline);
    }

    #[test]
    fn has_lifelines() {
        let svg = render(&simple_diagram(), &Theme::default(), None);
        // Dashed vertical lines.
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn has_arrow() {
        let svg = render(&simple_diagram(), &Theme::default(), None);
        assert!(svg.contains("<polygon"), "should have arrow head polygon");
    }

    #[test]
    fn leading_empty_group_frame_spans_header_height() {
        let mut diagram = simple_diagram();
        diagram.events.insert(
            0,
            Event::GroupStart(GroupStart {
                kind: GroupKind::Group,
                label: Some("emptyGroup".to_string()),
                source_line: 1,
            }),
        );
        diagram.events.insert(1, Event::GroupEnd);

        let svg = render(&diagram, &Theme::default(), None);
        assert!(
            svg.contains(r#"height="17.3105" style="stroke:#000000;stroke-width:1.5;""#),
            "leading empty group frame should span the full header tab height"
        );
    }

    #[test]
    fn has_plantuml_attributes() {
        let svg = render(&simple_diagram(), &Theme::default(), None);
        assert!(
            svg.contains(r##"data-diagram-type="SEQUENCE""##),
            "should have SEQUENCE data attribute"
        );
        assert!(
            svg.contains("plantuml"),
            "should have plantuml processing instruction"
        );
        assert!(svg.contains("<defs/>"), "should have empty defs element");
        assert!(
            svg.contains("textLength="),
            "should have textLength on text elements"
        );
        assert!(
            svg.contains("lengthAdjust="),
            "should have lengthAdjust on text elements"
        );
        assert!(
            svg.contains("participant-lifeline"),
            "should have lifeline groups"
        );
        assert!(svg.contains("participant-head"), "should have head groups");
        assert!(svg.contains("participant-tail"), "should have tail groups");
    }

    #[test]
    fn gradient_fill_accepts_dash_separator() {
        let defs = r##"<linearGradient id="gabc0"><stop offset="0%" stop-color="#59B6EC"/><stop offset="100%" stop-color="#2FA4E7"/></linearGradient>"##;
        assert_eq!(
            gradient_fill_or("#59B6EC-#2FA4E7", Some(defs)),
            "url(#gabc0)"
        );
    }

    #[test]
    fn gradient_fill_matches_stop_colors() {
        let defs = r##"<linearGradient id="g0"><stop offset="0%" stop-color="#D3F198"/><stop offset="100%" stop-color="#B5E853"/></linearGradient><linearGradient id="g1"><stop offset="0%" stop-color="#BB91B2"/><stop offset="100%" stop-color="#885E7F"/></linearGradient>"##;
        assert_eq!(gradient_fill_or("#bb91b2-#885E7F", Some(defs)), "url(#g1)");
    }

    #[test]
    fn gradient_fill_without_defs_uses_first_stop() {
        assert_eq!(gradient_fill_or("#59B6EC-#2FA4E7", None), "#59B6EC");
    }

    #[test]
    fn gradient_fill_transparent_is_svg_none() {
        assert_eq!(gradient_fill_or("transparent", None), "none");
    }

    #[test]
    fn has_correct_font_metrics() {
        // Verify PlantUML-compatible text widths
        let alice_w = text_width("Alice", 14.0);
        assert!(
            (alice_w - 32.7236).abs() < 0.001,
            "Alice@14 width = {alice_w}, expected 32.7236"
        );
        let bob_w = text_width("Bob", 14.0);
        assert!(
            (bob_w - 25.4639).abs() < 0.001,
            "Bob@14 width = {bob_w}, expected 25.4639"
        );
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\nAlice -> Bob : hello\nBob --> Alice : hi\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Alice"));
        assert!(svg.contains("hello"));
        assert!(svg.contains("hi"));
    }

    #[test]
    fn arrow_font_size_keeps_attached_note_spacing() {
        let input = concat!(
            "@startuml\n",
            "skinparam arrowFontSize 10\n",
            "Alice -> Bob : hello\n",
            "Bob --> Alice : world\n",
            "note right : note\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"y1="94.8096" y2="94.8096""#));
        assert!(svg.contains(r#"M86,77.2656 L86,102.2656"#));
    }

    #[test]
    fn sequence_font_name_skinparams_apply_to_text_families() {
        let input = concat!(
            "@startuml\n",
            "skinparam participantFontName Verdana\n",
            "skinparam arrowFontName Verdana\n",
            "skinparam noteFontName Verdana\n",
            "Alice -> Bob : hello\n",
            "note right : note\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"font-family="Verdana" font-size="14""#));
        assert!(svg.contains(r#"font-family="Verdana" font-size="13""#));
        assert!(svg.contains(">note</text>"));
    }

    #[test]
    fn message_label_tilde_escapes_creole_delimiters() {
        let input = "@startuml\nAlice -> Bob : ~**not bold~**\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(">**not bold**</text>"));
        assert!(!svg.contains(r#">not bold</text>"#));
    }

    #[test]
    fn message_label_code_tag_is_literal() {
        let input = "@startuml\nAlice -> Bob : call <code>doSomething()</code>\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("call &lt;code&gt;doSomething()&lt;/code&gt;"));
        assert!(!svg.contains("font-family=\"monospace\""));
    }

    #[test]
    fn multiline_note_code_block_renders_as_monospace_lines() {
        let input = concat!(
            "@startuml\n",
            "Alice -> Bob : request\n",
            "note over Alice\n",
            "  <code>\n",
            "  function foo() {\n",
            "    return 42;\n",
            "  }\n",
            "  </code>\n",
            "end note\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"font-family="monospace" font-size="13""#));
        assert!(svg.contains(">function&#160;foo()&#160;{</text>"));
        assert!(svg.contains(">return&#160;42;</text>"));
        assert!(!svg.contains("&lt;code&gt;"));
    }

    #[test]
    fn whole_strikethrough_note_line_renders_with_side_rules() {
        let input = concat!(
            "@startuml\n",
            "Alice -> Bob : hello\n",
            "note over Alice\n",
            "  --strikethrough--\n",
            "end note\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"<line style="stroke:#181818;stroke-width:1;""#));
        assert!(svg.contains(">strikethrough</text>"));
        assert!(!svg.contains(r#"text-decoration="line-through""#));
    }

    #[test]
    fn crossing_star_italic_mono_message_renders_as_bullet_line() {
        let input = "@startuml\nAlice -> Bob : **//\"\"bold italic mono\"\"**//**\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"<rect fill="#000000" height="3.5" width="3.5""##));
        assert!(svg.contains(
            r##"<text fill="#000000" font-family="monospace" font-size="13" font-style="italic""##
        ));
        assert!(svg.contains(">bold&#160;italic&#160;mono</text>"));
        assert!(!svg.contains("font-weight=\"700\""));
        assert!(!svg.contains("//&quot;&quot;bold italic mono&quot;&quot;"));
    }

    #[test]
    fn image_fallback_in_message_and_note_keeps_monospace_run() {
        let input = concat!(
            "@startuml\n",
            "Alice -> Bob : message with <img:sprite.png>\n",
            "note right : note with <img:sprite.png>\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(">message with</text>"));
        assert!(svg.contains(">note with</text>"));
        assert!(svg.contains(r#"font-family="monospace" font-size="14""#));
        assert!(!svg.contains("message with (Cannot"));
        assert!(!svg.contains("note with (Cannot"));
    }

    #[test]
    fn table_note_renders_cells_and_grid() {
        let input = concat!(
            "@startuml\n",
            "Alice -> Bob : hello\n",
            "note over Alice, Bob\n",
            "  |= Key |= Value |\n",
            "  | name | Alice |\n",
            "end note\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(">Key</text>"));
        assert!(svg.contains(">Value</text>"));
        assert!(svg.contains(r#"stroke:#000000;stroke-width:0.5;"#));
        assert!(!svg.contains("|= Key |= Value |"));
    }

    #[test]
    fn inline_nested_start_end_message_label_is_hidden() {
        let input = "@startuml\nAlice -> Bob : @startuml nested @enduml\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(!svg.contains("@startuml nested @enduml"));
        assert!(!svg.contains(r##"<text fill="#000000" font-family="sans-serif" font-size="13""##));
        assert!(svg.contains(r#"width="112px""#));
        assert!(svg.contains(r#"height="107px""#));
    }

    #[test]
    fn leading_italic_group_guard_keeps_brackets_plain() {
        let input = concat!(
            "@startuml\n",
            "alt **happy path**\n",
            "  Alice -> Bob : ok\n",
            "else //sad path//\n",
            "  Alice -> Bob : error\n",
            "end\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#">[</text>"#));
        assert!(svg.contains(r#"font-style="italic""#));
        assert!(svg.contains(r#">sad path</text>"#));
        assert!(svg.contains(r#">]</text>"#));
        assert!(!svg.contains("[//sad path//]"));
    }

    #[test]
    fn text_width_matches_plantuml() {
        // Verify the character width table against known PlantUML golden values.
        let cases = [
            ("Alice", 14.0, 32.7236),
            ("Bob", 14.0, 25.4639),
            ("A", 14.0, 9.6592),
            ("B", 14.0, 8.0527),
            ("request", 13.0, 47.5439),
            ("response", 13.0, 57.2939),
            ("1", 13.0, 8.2202),
        ];
        for (text, size, expected) in cases {
            let actual = text_width(text, size);
            assert!(
                (actual - expected).abs() < 0.01,
                "{text}@{size}: actual={actual}, expected={expected}"
            );
        }
    }
}
