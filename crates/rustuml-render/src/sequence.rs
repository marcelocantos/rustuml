// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's Java implementation exactly —
//! same element structure, attributes, coordinates, and font metrics.

use std::collections::HashMap;
use std::fmt::Write;

use rustuml_parser::diagram::sequence::*;

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
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

/// Compute bold text width at a given font size, routing through the creole-aware
/// segment helper so layout measurements match what `text_render::emit_text`
/// will actually emit.
fn bold_text_width(text: &str, font_size: f64) -> f64 {
    text_render::measure(text, font_size, true)
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

/// Compute note content width based on text width and note shape.
fn note_content_width(max_text_w: f64, shape: NoteShape) -> f64 {
    match shape {
        NoteShape::Note => max_text_w.ceil() + 20.0, // 6 pad + text + 4 pad + 10 fold
        NoteShape::Hexagonal => max_text_w.ceil() + 23.0, // 10 indent + 2 pad + text + 1 pad + 10 indent
        NoteShape::Rectangular => max_text_w.ceil() + 7.0, // 6 pad + text + 1 pad
    }
}

// ---------------------------------------------------------------------------
// PlantUML layout constants (reverse-engineered from golden SVGs)
// ---------------------------------------------------------------------------

const HEAD_BOX_Y: f64 = 5.0;
const HEAD_BOX_H: f64 = 30.488281250; // exact Java double
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

/// Compute the vertical step for a message event.
/// Messages with label text get extra height for the text line.
fn msg_step(has_text: bool) -> f64 {
    MSG_BASE_STEP + if has_text { MSG_TEXT_HEIGHT } else { 0.0 }
}

/// Compute the first-message offset from lifeline top.
fn first_msg_offset(has_text: bool) -> f64 {
    MSG_BASE_FIRST_OFFSET + if has_text { MSG_TEXT_HEIGHT } else { 0.0 }
}
const LIFELINE_Y_OFFSET: f64 = 1.0; // lifeline starts 1px below head box
const RIGHT_MARGIN: f64 = 10.0; // right margin beyond last box
const BOTTOM_MARGIN: f64 = 7.0; // bottom margin below tail box
const ARROW_SIZE: f64 = 10.0; // horizontal size of arrow polygon
const ARROW_HALF_H: f64 = 4.0; // vertical half-height of arrow polygon
const FILLED_ARROW_NOTCH: f64 = 4.0; // notch indent in filled arrow
const MSG_TEXT_LEFT_PAD: f64 = 7.0; // text offset from source lifeline
const LEFT_ARROW_TEXT_PAD: f64 = 16.0; // text offset from arrow tip (left arrows)
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
/// Extra width padding for boundary shape.
const BOUNDARY_EXTRA_WIDTH: f64 = 8.0;

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

/// Base height of a single-line note box.
const NOTE_BASE_HEIGHT: f64 = 25.0;
/// Additional height per extra line in a multi-line note.
const NOTE_LINE_HEIGHT: f64 = 15.0;
/// Size of the folded corner (both x and y).
const NOTE_FOLD_SIZE: f64 = 10.0;
/// Text left padding inside the note box.
const NOTE_TEXT_X_PAD: f64 = 6.0;
const RNOTE_TEXT_X_PAD: f64 = 4.0;
// Queue pill total horizontal padding (text + this = box width).
const QUEUE_TEXT_H_PAD: f64 = 20.0;
// Queue text inset from box left edge (cap radius).
const QUEUE_TEXT_X_PAD: f64 = 5.0;
/// Vertical offset from note top to the text baseline of the first line.
const NOTE_TEXT_Y_OFFSET: f64 = 17.568359375; // exact Java double
/// Line spacing between text lines in a multi-line note (= MSG_TEXT_HEIGHT).
const NOTE_TEXT_LINE_SPACING: f64 = MSG_TEXT_HEIGHT; // 15.310546875
/// Gap between previous event y and note top (non-first event).
const NOTE_GAP_AFTER_MSG: f64 = 13.0;
/// Gap between lifeline top and note top (first event).
const NOTE_GAP_FIRST: f64 = 15.0;
/// Note fill color.
const NOTE_FILL: &str = "#FEFFDD";
/// Gap from participant lifeline to note edge for left/right notes.
const NOTE_LIFELINE_GAP: f64 = 5.0;
/// Base height for hexagonal (hnote) and rectangular (rnote) notes.
const HNOTE_BASE_HEIGHT: f64 = 23.0;
/// Horizontal indent of hexagonal note vertices from note edges.
const HNOTE_INDENT: f64 = 10.0;
/// Text y offset for hnote/rnote (1px less than standard note).
const HNOTE_TEXT_Y_OFFSET: f64 = NOTE_TEXT_Y_OFFSET - 1.0;

// ---------------------------------------------------------------------------
// Group layout constants (reverse-engineered from golden SVGs)
// ---------------------------------------------------------------------------

/// Vertical space consumed by a group start header.
#[allow(dead_code)]
const GROUP_HEADER_HEIGHT: f64 = 17.310546875;
/// Gap from preceding message y to group frame top.
const GROUP_GAP_AFTER_MSG: f64 = 15.0;
/// Extra y advance after GroupStart event_y (before first inner message).
const GROUP_INNER_TOP_PAD: f64 = 9.310546875;
/// Vertical space consumed by a group else divider.
const GROUP_ELSE_HEIGHT: f64 = 9.0;
/// Extra y advance after GroupElse event_y (before next inner message).
const GROUP_ELSE_INNER_PAD: f64 = 5.955078125;
/// Vertical advance for GroupEnd.
const GROUP_END_HEIGHT: f64 = 7.0;
/// Left/right margin for group frame beyond participant boxes.
const GROUP_FRAME_MARGIN: f64 = 10.0;

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

/// Decode PlantUML backslash and tilde escapes in label text.
/// `\\` → `\`, `~X` → `X` when X is a markup character (*/_-"<[#).
/// `~~` is NOT a tilde escape — it's strikethrough or literal tildes.
fn decode_escapes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'\\') {
            chars.next();
            result.push('\\');
        } else if c == '~' {
            if let Some(&next) = chars.peek() {
                // Tilde escape: consume only if next char is a creole markup char
                // (but NOT another tilde — ~~ is strikethrough, not an escape)
                if next != '~' && "*/_-\"<[#".contains(next) {
                    chars.next();
                    result.push(next);
                } else {
                    result.push('~');
                }
            } else {
                result.push('~');
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Process label text for SVG rendering: decode escapes and replace unsupported
/// markup like `<img:...>` with a placeholder matching PlantUML's behavior.
fn process_label(s: &str) -> String {
    let decoded = decode_escapes(s);
    let mut result = String::with_capacity(decoded.len());
    let mut rest = decoded.as_str();
    while let Some(start) = rest.find("<img:") {
        result.push_str(&rest[..start]);
        let after = &rest[start..];
        if let Some(end) = after.find('>') {
            let raw_src = &after["<img:".len()..end];
            let src = if let Some(brace) = raw_src.find('{') {
                &raw_src[..brace]
            } else {
                raw_src
            };
            if src.starts_with("https://") || src.starts_with("http://") {
                result.push_str(&format!("(Cannot\u{00a0}decode:\u{00a0}{src})"));
            } else {
                result.push_str("(Cannot\u{00a0}decode)");
            }
            rest = &after[end + 1..];
        } else {
            result.push_str(after);
            return result;
        }
    }
    result.push_str(rest);
    result
}

/// Styling for an autonumber prefix, derived from the format string.
#[derive(Clone)]
struct AutoNumberStyle {
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
        }
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

// ---------------------------------------------------------------------------
// Layout data structures
// ---------------------------------------------------------------------------

struct ParticipantLayout {
    /// Participant index (0-based)
    idx: usize,
    /// Participant ID
    id: String,
    /// Display label
    label: String,
    /// The visual shape of this participant.
    kind: ParticipantKind,
    /// Optional stereotype text
    stereotype: Option<String>,
    /// Text width at font-size 14
    text_width: f64,
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
    /// Lifeline dashed-line stroke colour (default `#181818`). Driven by
    /// `skinparam sequenceLifeLineBorderColor`.
    lifeline_border: String,
    /// Lifeline dashed-line stroke thickness (default `0.5`). Driven by
    /// `skinparam sequenceLifeLineBorderThickness`.
    lifeline_border_thickness: String,
}

impl PlantUmlSvg {
    fn new() -> Self {
        Self {
            buf: String::with_capacity(4096),
            arrow_thickness: "1".into(),
            participant_border: "#181818".into(),
            participant_border_thickness: "0.5".into(),
            lifeline_border: "#181818".into(),
            lifeline_border_thickness: "0.5".into(),
        }
    }

    /// Write the opening `<svg>` tag with PlantUML's exact attributes.
    fn open_svg(&mut self, width: u32, height: u32) {
        write!(
            self.buf,
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="SEQUENCE" height="{height}px" preserveAspectRatio="none" style="width:{width}px;height:{height}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {width} {height}" width="{width}px" zoomAndPan="magnify">"##,
        )
        .unwrap();
        // Processing instruction
        self.buf.push_str("<?plantuml 1.2026.3beta6?>");
        // Empty defs
        self.buf.push_str("<defs/>");
        // Open main group
        self.buf.push_str("<g>");
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
    ) {
        write!(
            self.buf,
            r##"<g class="participant-lifeline" data-entity-uid="{part_uid}" data-qualified-name="{qualified_name}" data-source-line="{source_line}" id="{part_uid}-lifeline">"##,
            part_uid = escape_xml(part_uid),
            qualified_name = escape_xml(qualified_name),
        )
        .unwrap();

        // Inner group with title, invisible rect, and dashed line
        self.buf.push_str("<g>");
        write!(self.buf, "<title>{}</title>", escape_xml(title),).unwrap();

        write!(
            self.buf,
            r##"<rect fill="#000000" fill-opacity="0.00000" height="{}" width="{}" x="{}" y="{}"/>"##,
            fmt_coord(rect_h),
            LIFELINE_RECT_WIDTH as u32,
            fmt_coord(rect_x),
            fmt_coord(rect_y),
        )
        .unwrap();

        write!(
            self.buf,
            r##"<line style="stroke:{};stroke-width:{};stroke-dasharray:5,5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            self.lifeline_border,
            self.lifeline_border_thickness,
            fmt_coord(line_x),
            fmt_coord(line_x),
            fmt_coord(line_y1),
            fmt_coord(line_y2),
        )
        .unwrap();

        self.buf.push_str("</g>");
        self.buf.push_str("</g>");
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
        write!(
            self.buf,
            r##"<g class="participant participant-{position}" data-entity-uid="{part_uid}" data-qualified-name="{qualified_name}" data-source-line="{source_line}" id="{part_uid}-{position}">"##,
            part_uid = escape_xml(part_uid),
            qualified_name = escape_xml(qualified_name),
        )
        .unwrap();

        write!(
            self.buf,
            r##"<rect fill="{}" height="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:{};" width="{}" x="{}" y="{}"/>"##,
            fill_color,
            fmt_coord(rect_h),
            fmt_coord(HEAD_BOX_RX),
            fmt_coord(HEAD_BOX_RX),
            self.participant_border,
            self.participant_border_thickness,
            fmt_coord(rect_w),
            fmt_coord(rect_x),
            fmt_coord(rect_y),
        )
        .unwrap();

        // Stereotype text (above participant name, smaller font)
        if let Some((st_text, _st_width)) = stereotype {
            let st_display = format!("\u{ab}{st_text}\u{bb}");
            let st_y = text_y - 13.0; // stereotype is above the name
            text_render::emit_text(
                &mut self.buf,
                &st_display,
                &TextBase {
                    x: text_x,
                    y: st_y,
                    font_size: 11,
                    font_family: "sans-serif",
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
                x: text_x,
                y: text_y,
                font_size: 14,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );

        self.buf.push_str("</g>");
    }

    /// Open a participant group element (head or tail).
    fn participant_group_open(
        &mut self,
        part_uid: &str,
        qualified_name: &str,
        source_line: u32,
        position: &str,
    ) {
        write!(
            self.buf,
            r##"<g class="participant participant-{position}" data-entity-uid="{part_uid}" data-qualified-name="{qualified_name}" data-source-line="{source_line}" id="{part_uid}-{position}">"##,
            part_uid = escape_xml(part_uid),
            qualified_name = escape_xml(qualified_name),
        )
        .unwrap();
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
                font_size: 14,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
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
        write!(
            self.buf,
            r##"<ellipse cx="{}" cy="{}" fill="{}" rx="8" ry="8" style="stroke:#181818;stroke-width:0.5;"/>"##,
            fmt_coord(cx),
            fmt_coord(head_cy),
            fill_color,
        )
        .unwrap();

        // Body path: spine, arms, legs
        let spine_top = figure_base + ACTOR_SPINE_TOP_OFFSET;
        let spine_bottom = figure_base + ACTOR_SPINE_BOTTOM_OFFSET;
        let arm_y = figure_base + ACTOR_ARM_Y_OFFSET;
        let arm_left = cx - ACTOR_ARM_HALF;
        let arm_right = cx + ACTOR_ARM_HALF;
        let leg_bottom = figure_base + ACTOR_LEG_BOTTOM_OFFSET;
        write!(
            self.buf,
            r##"<path d="M{cx},{st} L{cx},{sb} M{al},{ay} L{ar},{ay} M{cx},{sb} L{al},{lb} M{cx},{sb} L{ar},{lb}" fill="none" style="stroke:#181818;stroke-width:0.5;"/>"##,
            cx = fmt_coord(cx),
            st = fmt_coord(spine_top),
            sb = fmt_coord(spine_bottom),
            al = fmt_coord(arm_left),
            ar = fmt_coord(arm_right),
            ay = fmt_coord(arm_y),
            lb = fmt_coord(leg_bottom),
        )
        .unwrap();

        self.buf.push_str("</g>");
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

        write!(
            self.buf,
            r##"<path d="M{lx},{lt} L{lx},{lb} M{lx},{cy} L{ht},{cy}" fill="none" style="stroke:#181818;stroke-width:0.5;"/>"##,
            lx = fmt_coord(line_x),
            lt = fmt_coord(line_top),
            lb = fmt_coord(line_bottom),
            cy = fmt_coord(circle_cy),
            ht = fmt_coord(horiz_to),
        )
        .unwrap();

        write!(
            self.buf,
            r##"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="stroke:#181818;stroke-width:0.5;"/>"##,
            fmt_coord(circle_cx),
            fmt_coord(circle_cy),
            fill_color,
            fmt_coord(STEREOTYPE_CIRCLE_R),
            fmt_coord(STEREOTYPE_CIRCLE_R),
        )
        .unwrap();

        self.buf.push_str("</g>");
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
        write!(
            self.buf,
            r##"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="stroke:#181818;stroke-width:0.5;"/>"##,
            fmt_coord(cx),
            fmt_coord(circle_cy),
            fill_color,
            fmt_coord(STEREOTYPE_CIRCLE_R),
            fmt_coord(STEREOTYPE_CIRCLE_R),
        )
        .unwrap();

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
        write!(
            self.buf,
            r##"<polygon fill="#181818" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:#181818;stroke-width:1;"/>"##,
            fmt_coord(p1x), fmt_coord(p1y),
            fmt_coord(p2x), fmt_coord(p2y),
            fmt_coord(p3x), fmt_coord(p3y),
            fmt_coord(p4x), fmt_coord(p4y),
            fmt_coord(p1x), fmt_coord(p1y),
        )
        .unwrap();

        self.buf.push_str("</g>");
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
        write!(
            self.buf,
            r##"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="stroke:#181818;stroke-width:0.5;"/>"##,
            fmt_coord(cx),
            fmt_coord(circle_cy),
            fill_color,
            fmt_coord(STEREOTYPE_CIRCLE_R),
            fmt_coord(STEREOTYPE_CIRCLE_R),
        )
        .unwrap();

        // Underline below the circle
        let line_y = circle_cy + STEREOTYPE_CIRCLE_R + 2.0;
        let line_x1 = cx - STEREOTYPE_CIRCLE_R;
        let line_x2 = cx + STEREOTYPE_CIRCLE_R;
        write!(
            self.buf,
            r##"<line style="stroke:#181818;stroke-width:0.5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(line_x1),
            fmt_coord(line_x2),
            fmt_coord(line_y),
            fmt_coord(line_y),
        )
        .unwrap();

        self.buf.push_str("</g>");
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

        write!(
            self.buf,
            r##"<path d="M{l},{t} C{l},{tc} {cx},{tc} {cx},{tc} C{cx},{tc} {r},{tc} {r},{t} L{r},{b} C{r},{bc} {cx},{bc} {cx},{bc} C{cx},{bc} {l},{bc} {l},{b} L{l},{t}" fill="{fc}" style="stroke:#181818;stroke-width:0.5;"/>"##,
            l = fmt_coord(left),
            r = fmt_coord(right),
            t = fmt_coord(top),
            tc = fmt_coord(top_curve),
            b = fmt_coord(bottom),
            bc = fmt_coord(bottom_curve),
            cx = fmt_coord(cx),
            fc = fill_color,
        )
        .unwrap();

        // Top ellipse (visible arc)
        let top_arc_bottom = figure_base + 2.0 * DB_ELLIPSE_RY;
        write!(
            self.buf,
            r##"<path d="M{l},{t} C{l},{tab} {cx},{tab} {cx},{tab} C{cx},{tab} {r},{tab} {r},{t}" fill="none" style="stroke:#181818;stroke-width:0.5;"/>"##,
            l = fmt_coord(left),
            r = fmt_coord(right),
            t = fmt_coord(top),
            tab = fmt_coord(top_arc_bottom),
            cx = fmt_coord(cx),
        )
        .unwrap();

        self.buf.push_str("</g>");
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
    ) {
        self.participant_group_open(part_uid, qualified_name, source_line, position);

        // Back rectangle (offset right and up)
        let back_x = box_x + COLLECTIONS_OFFSET;
        let back_y = base_y;
        write!(
            self.buf,
            r##"<rect fill="{}" height="{}" style="stroke:#181818;stroke-width:0.5;" width="{}" x="{}" y="{}"/>"##,
            fill_color,
            fmt_coord(HEAD_BOX_H),
            fmt_coord(box_w - COLLECTIONS_OFFSET),
            fmt_coord(back_x),
            fmt_coord(back_y),
        )
        .unwrap();

        // Front rectangle (at box_x, offset down)
        let front_y = base_y + COLLECTIONS_OFFSET;
        write!(
            self.buf,
            r##"<rect fill="{}" height="{}" style="stroke:#181818;stroke-width:0.5;" width="{}" x="{}" y="{}"/>"##,
            fill_color,
            fmt_coord(HEAD_BOX_H),
            fmt_coord(box_w - COLLECTIONS_OFFSET),
            fmt_coord(box_x),
            fmt_coord(front_y),
        )
        .unwrap();

        // Text (on front rectangle)
        self.participant_text(text_x, text_y, text_content, text_len);

        self.buf.push_str("</g>");
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
            base_y + 5.0
        };
        let mid = top + pill_h / 2.0;
        let bottom = top + pill_h;
        let inner_right_inner = inner_right - cap_r;

        // Outer body
        write!(
            self.buf,
            r##"<path d="M{il},{t} L{ir},{t} C{r},{t} {r},{m} {r},{m} C{r},{m} {r},{b} {ir},{b} L{il},{b} C{l},{b} {l},{m} {l},{m} C{l},{m} {l},{t} {il},{t}" fill="{fc}" style="stroke:#181818;stroke-width:0.5;"/>"##,
            il = fmt_coord(inner_left),
            ir = fmt_coord(inner_right),
            l = fmt_coord(left),
            r = fmt_coord(right),
            t = fmt_coord(top),
            m = fmt_coord(mid),
            b = fmt_coord(bottom),
            fc = fill_color,
        )
        .unwrap();

        // Inner right curve (the divider inside the pill)
        write!(
            self.buf,
            r##"<path d="M{ir},{t} C{iri},{t} {iri},{m} {iri},{m} C{iri},{b} {ir},{b} {ir},{b}" fill="none" style="stroke:#181818;stroke-width:0.5;"/>"##,
            ir = fmt_coord(inner_right),
            iri = fmt_coord(inner_right_inner),
            t = fmt_coord(top),
            m = fmt_coord(mid),
            b = fmt_coord(bottom),
        )
        .unwrap();

        // Text
        self.participant_text(text_x, text_y, text_content, text_len);

        self.buf.push_str("</g>");
    }

    /// Write an activation bar with optional fill color.
    fn activation_bar(&mut self, title: &str, x: f64, y: f64, h: f64, color: &str) {
        self.buf.push_str("<g>");
        write!(self.buf, "<title>{}</title>", escape_xml(title)).unwrap();
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
        self.buf.push_str("</g>");
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
        _text_len: f64,
        color: &str,
        autonumber: Option<(&str, f64, &AutoNumberStyle)>,
    ) {
        write!(
            self.buf,
            r##"<g class="message" data-entity-1="{entity1}" data-entity-2="{entity2}" data-source-line="{source_line}" id="msg{msg_id}">"##,
            entity1 = escape_xml(entity1),
            entity2 = escape_xml(entity2),
        )
        .unwrap();

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
            let fill = style.fill.as_deref().unwrap_or("#000000");
            text_render::emit_text(
                &mut self.buf,
                num_text,
                &TextBase {
                    x: text_x,
                    y: text_y,
                    font_size: 13,
                    font_family: "sans-serif",
                    fill,
                    bold: style.bold,
                    italic: style.italic,
                    underline: style.underline,
                    skip_underline: false,
                },
            );
            text_x + num_w + AUTONUMBER_LABEL_GAP
        } else {
            text_x
        };

        if !text_content.is_empty() {
            text_render::emit_text(
                &mut self.buf,
                text_content,
                &TextBase {
                    x: label_x,
                    y: text_y,
                    font_size: MSG_FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: "#000000",
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
        self.buf.push_str("</g>");
    }

    /// Write a message group with filled arrow (->).
    #[allow(clippy::too_many_arguments)]
    fn message_filled_arrow(
        &mut self,
        entity1: &str,
        entity2: &str,
        source_line: u32,
        msg_id: u32,
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
    ) {
        write!(
            self.buf,
            r##"<g class="message" data-entity-1="{entity1}" data-entity-2="{entity2}" data-source-line="{source_line}" id="msg{msg_id}">"##,
            entity1 = escape_xml(entity1),
            entity2 = escape_xml(entity2),
        )
        .unwrap();

        // Arrow head polygon keeps stroke-width:1 even when the line is
        // thickened — PlantUML scales the line only.
        write!(
            self.buf,
            r##"<polygon fill="{color}" points="{arrow_points}" style="stroke:{color};stroke-width:1;"/>"##,
        )
        .unwrap();

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
            // Autonumber styling derived from creole tags in the format string.
            let fill = style.fill.as_deref().unwrap_or("#000000");
            text_render::emit_text(
                &mut self.buf,
                num_text,
                &TextBase {
                    x: text_x,
                    y: text_y,
                    font_size: 13,
                    font_family: "sans-serif",
                    fill,
                    bold: style.bold,
                    italic: style.italic,
                    underline: style.underline,
                    skip_underline: false,
                },
            );
            text_x + num_w + AUTONUMBER_LABEL_GAP
        } else {
            text_x
        };

        if !text_content.is_empty() {
            let _ = text_len;
            text_render::emit_text(
                &mut self.buf,
                text_content,
                &TextBase {
                    x: label_x,
                    y: text_y,
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

        self.buf.push_str("</g>");
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
    ) {
        write!(
            self.buf,
            r##"<g class="message" data-entity-1="{entity1}" data-entity-2="{entity2}" data-source-line="{source_line}" id="msg{msg_id}">"##,
            entity1 = escape_xml(entity1),
            entity2 = escape_xml(entity2),
        )
        .unwrap();

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

        let thickness = self.arrow_thickness.clone();
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
            // Autonumber styling derived from creole tags in the format string.
            let fill = style.fill.as_deref().unwrap_or("#000000");
            text_render::emit_text(
                &mut self.buf,
                num_text,
                &TextBase {
                    x: text_x,
                    y: text_y,
                    font_size: 13,
                    font_family: "sans-serif",
                    fill,
                    bold: style.bold,
                    italic: style.italic,
                    underline: style.underline,
                    skip_underline: false,
                },
            );
            text_x + num_w + AUTONUMBER_LABEL_GAP
        } else {
            text_x
        };

        if !text_content.is_empty() {
            let _ = text_len;
            text_render::emit_text(
                &mut self.buf,
                text_content,
                &TextBase {
                    x: label_x,
                    y: text_y,
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

        self.buf.push_str("</g>");
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
) {
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
                    );
                }
                _ => unreachable!(),
            }
        }
        ParticipantKind::Collections => {
            let text_x = p.box_x + BOX_TEXT_X_PAD;
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
            );
        }
        ParticipantKind::Queue => {
            let text_x = p.box_x + QUEUE_TEXT_X_PAD;
            // Queue pill is shorter and offset; text baseline tracks the pill mid.
            let pill_top = if position == "tail" {
                base_y
            } else {
                base_y + 5.0
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
            );
        }
        ParticipantKind::Participant => {
            let text_x = p.box_x + BOX_TEXT_X_PAD;
            let text_y =
                base_y + BOX_TEXT_Y_OFFSET + if p.stereotype.is_some() { 7.5 } else { 0.0 };
            let stereo_arg = p.stereotype.as_ref().map(|s| {
                let display = format!("\u{ab}{s}\u{bb}");
                (display, p.stereotype_width)
            });
            let stereo_ref = stereo_arg.as_ref().map(|(s, w)| (s.as_str(), *w));
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
    render(diagram, theme)
}

/// Render a sequence diagram to SVG matching PlantUML's exact output.
pub fn render(diagram: &SequenceDiagram, _theme: &Theme) -> String {
    // Per-diagram skinparam overrides relevant to sequence arrow rendering.
    // These are read directly from the parser's skinparam list (rather than
    // the cascading `Theme`) so any value-less default tracks PlantUML's
    // historical colours rather than the workspace `slate` theme.
    let mut default_arrow_color = "#181818".to_string();
    let mut default_arrow_thickness: String = "1".to_string();
    let mut participant_fill = "#E2E2F0".to_string();
    let mut participant_border = "#181818".to_string();
    let mut participant_border_thickness: String = "0.5".to_string();
    let mut lifeline_border = "#181818".to_string();
    let mut lifeline_border_thickness: String = "0.5".to_string();
    // Per-participant-kind background overrides. Each defaults to
    // `participant_fill`; the relevant `<kind>BackgroundColor` skinparam
    // (with or without the `sequence` prefix) sets it.
    let mut actor_fill_override: Option<String> = None;
    let mut _actor_border_override: Option<String> = None;
    let mut boundary_fill_override: Option<String> = None;
    let mut control_fill_override: Option<String> = None;
    let mut entity_fill_override: Option<String> = None;
    let mut database_fill_override: Option<String> = None;
    let mut collections_fill_override: Option<String> = None;
    let mut queue_fill_override: Option<String> = None;
    for sp in &diagram.meta.skinparams {
        let key = sp.key.to_ascii_lowercase();
        let val = sp.value.trim();
        if val.is_empty() {
            continue;
        }
        match key.as_str() {
            "arrowcolor" | "sequencearrowcolor" => {
                default_arrow_color = resolve_color(val);
            }
            "arrowthickness" | "sequencearrowthickness" => {
                if let Ok(v) = val.parse::<f64>() {
                    default_arrow_thickness = plantuml_metrics::fmt_coord(v);
                }
            }
            "participantbackgroundcolor" | "sequenceparticipantbackgroundcolor" => {
                participant_fill = resolve_color(val);
            }
            "participantbordercolor" | "sequenceparticipantbordercolor" => {
                participant_border = resolve_color(val);
            }
            "participantborderthickness" | "sequenceparticipantborderthickness" => {
                if let Ok(v) = val.parse::<f64>() {
                    participant_border_thickness = plantuml_metrics::fmt_coord(v);
                }
            }
            "sequencelifelinebordercolor" => {
                lifeline_border = resolve_color(val);
            }
            "sequencelifelineborderthickness" => {
                if let Ok(v) = val.parse::<f64>() {
                    lifeline_border_thickness = plantuml_metrics::fmt_coord(v);
                }
            }
            "actorbackgroundcolor" | "sequenceactorbackgroundcolor" => {
                actor_fill_override = Some(resolve_color(val));
            }
            "actorbordercolor" | "sequenceactorbordercolor" => {
                _actor_border_override = Some(resolve_color(val));
            }
            "boundarybackgroundcolor" | "sequenceboundarybackgroundcolor" => {
                boundary_fill_override = Some(resolve_color(val));
            }
            "controlbackgroundcolor" | "sequencecontrolbackgroundcolor" => {
                control_fill_override = Some(resolve_color(val));
            }
            "entitybackgroundcolor" | "sequenceentitybackgroundcolor" => {
                entity_fill_override = Some(resolve_color(val));
            }
            "databasebackgroundcolor" | "sequencedatabasebackgroundcolor" => {
                database_fill_override = Some(resolve_color(val));
            }
            "collectionsbackgroundcolor" | "sequencecollectionsbackgroundcolor" => {
                collections_fill_override = Some(resolve_color(val));
            }
            "queuebackgroundcolor" | "sequencequeuebackgroundcolor" => {
                queue_fill_override = Some(resolve_color(val));
            }
            _ => {}
        }
    }
    let default_arrow_color = default_arrow_color.as_str();
    let default_arrow_thickness = default_arrow_thickness.as_str();
    // Empty diagram with no title — render the PlantUML welcome screen.
    if diagram.participants.is_empty() && diagram.events.is_empty() && diagram.meta.title.is_none()
    {
        return render_empty_welcome();
    }

    // Title-band height: when the diagram has a `title ...` directive, PlantUML
    // reserves a band above the participant heads for the bold 14pt text.
    // Empirical formula from goldens:
    //   band = 21 + n_lines * text_height(14)
    // where 21 = 10 (top pad to first baseline) + 11 (descent + gap to head box)
    // and text_height(14) = 16.48828125 (Java AWT LineMetrics).
    // The participant top, lifeline top, message Ys, tail box Y, and the SVG
    // height all shift down by this band.
    const TITLE_FONT_SIZE: u32 = 14;
    const TITLE_TOP_PAD: f64 = 10.0; // gap from y=0 to first title baseline (minus ascent)
    const TITLE_BOTTOM_PAD: f64 = 11.0; // gap from last title descent line to head top
    let title_lines: Vec<&str> = diagram
        .meta
        .title
        .as_deref()
        .map(|t| t.split("\\n").collect())
        .unwrap_or_default();
    let title_band_h = if title_lines.is_empty() {
        0.0
    } else {
        TITLE_TOP_PAD
            + title_lines.len() as f64 * plantuml_metrics::text_height(TITLE_FONT_SIZE as f64)
            + TITLE_BOTTOM_PAD
    };
    let head_box_y = HEAD_BOX_Y + title_band_h;

    // -----------------------------------------------------------------------
    // Phase 1: Compute participant layouts
    // -----------------------------------------------------------------------

    let participants: Vec<ParticipantLayout> = diagram
        .participants
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let st = p.stereotype.clone();
            let st_display = st.as_ref().map(|s| format!("\u{ab}{s}\u{bb}"));
            let st_w = st_display
                .as_ref()
                .map(|s| text_width(s, 11.0))
                .unwrap_or(0.0);
            // Display label includes stereotype inline (matching PlantUML)
            let label = if let Some(ref st_text) = st {
                format!("{} \u{ab}{st_text}\u{bb}", p.label)
            } else {
                p.label.clone()
            };
            let tw = text_width(&label, PARTICIPANT_FONT_SIZE);
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
                    // Boundary: line + circle, wider than regular
                    // The shape is: vertical line at left, horizontal line to circle, circle r=12.
                    // Width = text_width + extra padding for the boundary adornments.
                    let shape_w = BOUNDARY_LINE_TO_CIRCLE_GAP + 2.0 * STEREOTYPE_CIRCLE_R;
                    let w = max_text_w.max(shape_w) + BOUNDARY_EXTRA_WIDTH;
                    let h = HEAD_BOX_H + CIRCLE_SHAPE_EXTRA_H;
                    (w, h)
                }
                ParticipantKind::Control | ParticipantKind::Entity => {
                    // Control/Entity: circle r=12, text below.
                    let shape_w = 2.0 * STEREOTYPE_CIRCLE_R;
                    let w = max_text_w.max(shape_w) + 2.0 * ACTOR_TEXT_PAD;
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
                    let w = max_text_w + 2.0 * BOX_TEXT_X_PAD + COLLECTIONS_OFFSET;
                    let h = HEAD_BOX_H + COLLECTIONS_EXTRA_H;
                    (w, h)
                }
                ParticipantKind::Queue => {
                    // Queue: pill shape, width = text + 20 (caps + padding).
                    let w = max_text_w + QUEUE_TEXT_H_PAD;
                    let h = HEAD_BOX_H;
                    (w, h)
                }
                ParticipantKind::Participant => {
                    let w = max_text_w + 2.0 * BOX_TEXT_X_PAD;
                    // Box height is taller for stereotyped participants.
                    let h = if st.is_some() {
                        HEAD_BOX_H + 15.0
                    } else {
                        HEAD_BOX_H
                    };
                    (w, h)
                }
            };

            ParticipantLayout {
                idx,
                id: p.id.clone(),
                label,
                kind: p.kind,
                stereotype: st,
                text_width: tw,
                stereotype_width: st_w,
                box_width: bw,
                box_height: bh,
                box_x: 0.0,
                center_x: 0.0,
                lifeline_line_x: 0.0,
            }
        })
        .collect();

    // Build a lookup from participant ID to index (owned keys to avoid borrow issues)
    let id_to_idx: HashMap<String, usize> =
        participants.iter().map(|p| (p.id.clone(), p.idx)).collect();

    // -----------------------------------------------------------------------
    // Phase 1.5: Pre-scan groups to determine participant shifts
    // -----------------------------------------------------------------------

    // For each group, identify which participant indices are referenced by
    // messages within the group. The group frame must encompass those participants.
    // If a group includes the leftmost participant, all participants must shift
    // right to make room for the group frame margin.

    let mut group_needs_left_shift = false;
    {
        // Scan for groups and collect the participant index range for each group
        let mut group_stack: Vec<(usize, usize)> = Vec::new(); // (min_idx, max_idx)
        for event in &diagram.events {
            match event {
                Event::GroupStart(_) => {
                    group_stack.push((usize::MAX, 0));
                }
                Event::GroupEnd => {
                    if let Some((min_idx, _max_idx)) = group_stack.pop()
                        && min_idx == 0
                    {
                        group_needs_left_shift = true;
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

    /// Extra left margin added to participant positions when groups encompass
    /// the leftmost participant. This makes room for the group frame.
    const GROUP_LEFT_SHIFT: f64 = 15.0;

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

    // Track activation state as we scan events to compute per-message shifts.
    let mut activation_depth: HashMap<String, usize> = HashMap::new();

    // Track return stack during spacing phase to infer return from/to.
    // Each entry: (activated_participant, sender)
    let mut spacing_return_stack: Vec<(String, String)> = Vec::new();

    // Track autonumber counter during spacing phase to compute bold label widths.
    let mut spacing_auto_num: Option<u32> = diagram.autonumber.as_ref().map(|an| an.start);

    for event in &diagram.events {
        match event {
            Event::Message(msg) => {
                let from_idx = id_to_idx.get(msg.from.as_str()).copied();
                let to_idx = id_to_idx.get(msg.to.as_str()).copied();
                if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
                    if fi == ti {
                        // Self-message: no pair spacing needed.
                        // Width tracking deferred to after Phase 3 (x positions assigned).
                    } else {
                        let label = process_label(&msg.label);
                        let label_w = text_width(&label, MSG_FONT_SIZE);

                        // Autonumber adds bold-or-plain text + gap before the label
                        // depending on whether a format string is set.
                        let autonumber_extra = if let Some(num) = spacing_auto_num.as_ref() {
                            let an = diagram.autonumber.as_ref().unwrap();
                            let num_text = format_autonumber(*num, &an.format);
                            let w = if autonumber_is_bold(&an.format) {
                                bold_text_width(&num_text, MSG_FONT_SIZE)
                            } else {
                                text_width(&num_text, MSG_FONT_SIZE)
                            };
                            w + AUTONUMBER_LABEL_GAP
                        } else {
                            0.0
                        };

                        // Base arrow width (text + padding + arrow)
                        let arrow_only_w = autonumber_extra
                            + label_w
                            + MSG_TEXT_LEFT_PAD
                            + MSG_TEXT_LEFT_PAD
                            + ARROW_SIZE;

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
                }

                // Update activation state from message's activation change
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
                if let Some(num) = spacing_auto_num.as_mut() {
                    let an = diagram.autonumber.as_ref().unwrap();
                    *num = num.saturating_add(an.step);
                }
            }
            Event::Return(ret) => {
                // Return messages need spacing computation like regular messages.
                // Pop the return stack to find from/to.
                // NOTE: compute spacing BEFORE deactivating — the return sender
                // is still activated at the point the message arrow is drawn.
                if let Some((ret_from, ret_to)) = spacing_return_stack.pop() {
                    let fi_opt = id_to_idx.get(ret_from.as_str()).copied();
                    let ti_opt = id_to_idx.get(ret_to.as_str()).copied();
                    if let (Some(fi), Some(ti)) = (fi_opt, ti_opt) {
                        let label = if ret.label.is_empty() {
                            String::new()
                        } else {
                            decode_escapes(&ret.label)
                        };
                        let label_w = text_width(&label, MSG_FONT_SIZE);

                        // Autonumber adds bold-or-plain text + gap before the label
                        // depending on whether a format string is set.
                        let autonumber_extra = if let Some(num) = spacing_auto_num.as_ref() {
                            let an = diagram.autonumber.as_ref().unwrap();
                            let num_text = format_autonumber(*num, &an.format);
                            let w = if autonumber_is_bold(&an.format) {
                                bold_text_width(&num_text, MSG_FONT_SIZE)
                            } else {
                                text_width(&num_text, MSG_FONT_SIZE)
                            };
                            w + AUTONUMBER_LABEL_GAP
                        } else {
                            0.0
                        };

                        // Same spacing formula as forward messages
                        let arrow_only_w = autonumber_extra
                            + label_w
                            + MSG_TEXT_LEFT_PAD
                            + MSG_TEXT_LEFT_PAD
                            + ARROW_SIZE;

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

                    // Deactivate AFTER spacing computation
                    if let Some(d) = activation_depth.get_mut(&ret_from) {
                        *d = d.saturating_sub(1);
                    }
                }

                // Advance autonumber
                if let Some(num) = spacing_auto_num.as_mut() {
                    let an = diagram.autonumber.as_ref().unwrap();
                    *num = num.saturating_add(an.step);
                }
            }
            // Notes spanning multiple participants need gap between them.
            Event::Note(note)
                if note.position == NotePosition::Over && note.participants.len() >= 2 =>
            {
                let first_idx = id_to_idx
                    .get(note.participants.first().unwrap().as_str())
                    .copied();
                let last_idx = id_to_idx
                    .get(note.participants.last().unwrap().as_str())
                    .copied();
                if let (Some(fi), Some(li)) = (first_idx, last_idx) {
                    let max_tw = note
                        .text
                        .lines()
                        .map(|l| text_width(l.trim(), MSG_FONT_SIZE))
                        .fold(0.0_f64, f64::max);
                    // The rendered note width = max(note_content_w, span + 38).
                    // The note fits if span >= note_content_w - 38. So the minimum
                    // total span is (note_content_w - 38), divided evenly across pairs.
                    let note_content_w = note_content_width(max_tw, note.shape);
                    let span_pairs = (li - fi) as f64;
                    let per_pair = ((note_content_w - 38.0) / span_pairs).max(0.0);
                    let (left, right) = if fi < li { (fi, li) } else { (li, fi) };
                    for slot in &mut pair_max_label_width[left..right] {
                        *slot = slot.max(per_pair);
                    }
                }
            }
            Event::Activate(id, _) => {
                *activation_depth.entry(id.clone()).or_default() += 1;
            }
            Event::Deactivate(id) => {
                if let Some(d) = activation_depth.get_mut(id) {
                    *d = d.saturating_sub(1);
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
    let mut min_first_center_x: f64 = 0.0;
    for event in &diagram.events {
        if let Event::Note(note) = event {
            let first_part = note
                .participants
                .first()
                .and_then(|id| id_to_idx.get(id.as_str()))
                .copied();
            match note.position {
                NotePosition::Over if note.participants.len() == 1 && first_part == Some(0) => {
                    let max_tw = note
                        .text
                        .lines()
                        .map(|l| text_width(l.trim(), MSG_FONT_SIZE))
                        .fold(0.0_f64, f64::max);
                    let note_w = note_content_width(max_tw, note.shape);
                    let shift = ((note_w - participants[0].box_width) / 2.0).floor();
                    let min_cx = HEAD_BOX_Y + shift.max(0.0) + participants[0].box_width / 2.0;
                    min_first_center_x = min_first_center_x.max(min_cx);
                }
                NotePosition::Left if first_part == Some(0) => {
                    // "note left of" on participant 0: the note extends left from the
                    // lifeline. The note right edge = floor(lifeline_line_x) - NOTE_LIFELINE_GAP.
                    // The note left edge = note_right - note_content_w, which must be >= HEAD_BOX_Y.
                    // So: lifeline_line_x >= HEAD_BOX_Y + note_content_w + NOTE_LIFELINE_GAP
                    // And: lifeline_line_x = center_x - box_width/2 + floor(box_width/2)
                    // Therefore: center_x >= HEAD_BOX_Y + note_content_w + NOTE_LIFELINE_GAP
                    //                        + box_width/2 - floor(box_width/2)
                    let max_tw = note
                        .text
                        .lines()
                        .map(|l| text_width(l.trim(), MSG_FONT_SIZE))
                        .fold(0.0_f64, f64::max);
                    let note_content_w = note_content_width(max_tw, note.shape);
                    let bw = participants[0].box_width;
                    let min_cx = HEAD_BOX_Y + note_content_w + NOTE_LIFELINE_GAP + bw / 2.0
                        - (bw / 2.0).floor();
                    min_first_center_x = min_first_center_x.max(min_cx);
                }
                _ => {}
            }
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
    let min_gap_boxes_for_pair = |i: usize| -> f64 {
        participants[i].box_width / 2.0 + participants[i + 1].box_width / 2.0 + 10.0
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

    let mut participants = participants;
    if !participants.is_empty() {
        // First participant center must be at least min_first_center_x (for notes)
        // and at least HEAD_BOX_Y + box_width/2 (to fit the box).
        // When groups encompass the leftmost participant, shift right for the frame margin.
        let group_shift = if group_needs_left_shift {
            GROUP_LEFT_SHIFT
        } else {
            0.0
        };
        let default_center = HEAD_BOX_Y + group_shift + participants[0].box_width / 2.0;
        participants[0].center_x = default_center.max(min_first_center_x);
        participants[0].box_x = participants[0].center_x - participants[0].box_width / 2.0;
        // PlantUML computes lifeline line x as box_x + (int)(box_width / 2)
        participants[0].lifeline_line_x =
            participants[0].box_x + (participants[0].box_width / 2.0).floor();

        for i in 1..n {
            // Minimum gap between centers: ensure boxes don't overlap
            let min_gap_boxes =
                participants[i - 1].box_width / 2.0 + participants[i].box_width / 2.0 + 10.0; // minimum 10px between box edges

            // Gap from message labels
            let gap_from_labels = pair_max_label_width[i - 1];

            let gap = min_gap_boxes.max(gap_from_labels);
            participants[i].center_x = participants[i - 1].center_x + gap;
            participants[i].box_x = participants[i].center_x - participants[i].box_width / 2.0;
            participants[i].lifeline_line_x =
                participants[i].box_x + (participants[i].box_width / 2.0).floor();
        }
    }

    let center_of = |id: &str| -> f64 {
        id_to_idx
            .get(id)
            .map(|&i| participants[i].center_x)
            .unwrap_or(0.0)
    };

    // Compute self-message right extent now that x positions are assigned.
    // Replay activation state to know whether the participant is active at the
    // moment of each self-message — shifts cx by ACTIVATION_HALF_W if so.
    {
        let mut act_depth: HashMap<String, usize> = HashMap::new();
        for event in &diagram.events {
            if let Event::Message(msg) = event {
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
                    let label_w = text_width(&label, MSG_FONT_SIZE);
                    let loopback_right = cx + SELF_MSG_EXTEND;
                    let text_right = cx + SELF_MSG_TEXT_X_PAD + label_w;
                    let self_right = loopback_right.max(text_right) + SELF_MSG_RIGHT_PAD;
                    max_self_msg_right = max_self_msg_right.max(self_right);
                }
                // Update activation state from message activation flag
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
            } else if let Event::Activate(id, _) = event {
                *act_depth.entry(id.clone()).or_default() += 1;
            } else if let Event::Deactivate(id) = event
                && let Some(d) = act_depth.get_mut(id)
            {
                *d = d.saturating_sub(1);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 4: Pre-compute y positions for each event and vertical dimensions
    // -----------------------------------------------------------------------

    // Use the maximum box height across all participants
    let max_box_h = participants
        .iter()
        .map(|p| p.box_height)
        .fold(HEAD_BOX_H, f64::max);
    let lifeline_top = head_box_y + max_box_h + LIFELINE_Y_OFFSET;

    // Pre-compute message y positions. PlantUML sizes each message step
    // dynamically: messages with label text get extra height for the text line.
    // Notes consume vertical space (note height + gap) and count as events
    // for msg_count (so subsequent messages use msg_step, not first_msg_offset).
    // Groups add header/else/end vertical space.
    let mut event_y_positions: Vec<f64> = Vec::new();
    let mut msg_count: u32 = 0;
    let last_effective_y;
    {
        let mut y = lifeline_top;
        for event in &diagram.events {
            let has_text = match event {
                Event::Message(msg) => {
                    let label = process_label(&msg.label);
                    !label.is_empty()
                }
                Event::Return(ret) => !ret.label.is_empty(),
                Event::Divider(_) => true,
                Event::Delay(t) => t.is_some(),
                _ => false,
            };
            match event {
                Event::Message(msg) => {
                    let is_self = msg.from == msg.to;
                    if msg_count == 0 {
                        y += first_msg_offset(has_text);
                    } else {
                        y += msg_step(has_text);
                    }
                    event_y_positions.push(y);
                    if is_self {
                        // Self-messages have a loopback that drops below the top line.
                        // The next message's y step starts from the bottom of the loop.
                        y += SELF_MSG_DROP;
                    }
                    msg_count += 1;
                }
                Event::Return(_) => {
                    if msg_count == 0 {
                        y += first_msg_offset(has_text);
                    } else {
                        y += msg_step(has_text);
                    }
                    event_y_positions.push(y);
                    msg_count += 1;
                }
                Event::Divider(_) => {
                    // Dividers take a total of msg_step + MSG_BASE_STEP vertical space.
                    // The event_y is positioned at the divider text baseline, which is
                    // at msg_step + 5.258 from the previous event. The remaining
                    // MSG_BASE_STEP - 5.258 = 8.742 adds to the gap before the next message.
                    const DIVIDER_TEXT_OFFSET: f64 = 5.2578;
                    const DIVIDER_TAIL_PAD: f64 = MSG_BASE_STEP - DIVIDER_TEXT_OFFSET;
                    if msg_count == 0 {
                        y += first_msg_offset(has_text) + DIVIDER_TEXT_OFFSET;
                    } else {
                        y += msg_step(has_text) + DIVIDER_TEXT_OFFSET;
                    }
                    event_y_positions.push(y);
                    // The divider's tail padding accounts for the space below the
                    // text baseline (the double lines extend above, but PlantUML
                    // also reserves space below for visual balance).
                    y += DIVIDER_TAIL_PAD;
                    msg_count += 1;
                }
                Event::Delay(_) => {
                    if msg_count == 0 {
                        y += first_msg_offset(has_text);
                    } else {
                        y += msg_step(has_text);
                    }
                    event_y_positions.push(y);
                    msg_count += 1;
                }
                Event::Note(note) => {
                    // Notes consume vertical space. The note top is positioned
                    // relative to the current y cursor. After the note, subsequent
                    // events use msg_step (note counts as an event).
                    let note_top = if msg_count == 0 {
                        y + NOTE_GAP_FIRST
                    } else {
                        y + NOTE_GAP_AFTER_MSG
                    };
                    let num_lines = note.text.lines().count().max(1);
                    // hnote/rnote have a smaller base height (23 vs 25), reducing
                    // the vertical space consumed by 2px.
                    let note_y_extra = match note.shape {
                        NoteShape::Note => 7.0,
                        NoteShape::Hexagonal | NoteShape::Rectangular => 5.0,
                    };
                    let note_event_y = note_top + note_y_extra + num_lines as f64 * MSG_TEXT_HEIGHT;
                    y = note_event_y;
                    event_y_positions.push(y);
                    msg_count += 1; // note counts as an event for spacing
                }
                Event::GroupStart(_) => {
                    // Group frame top is offset from the preceding message.
                    if msg_count == 0 {
                        y += NOTE_GAP_FIRST;
                    } else {
                        y += GROUP_GAP_AFTER_MSG;
                    }
                    event_y_positions.push(y);
                    // Advance y past the header so subsequent messages are positioned correctly.
                    y += GROUP_INNER_TOP_PAD;
                    // Don't increment msg_count — the group header itself isn't a message
                }
                Event::GroupElse(g) => {
                    // Else divider adds vertical space.
                    y += GROUP_ELSE_HEIGHT;
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
                }
                Event::GroupEnd => {
                    // Group end: event_y marks the frame bottom,
                    // but the y cursor advances less (for tail gap calculation).
                    let group_end_y = y + GROUP_END_HEIGHT + 1.0;
                    event_y_positions.push(group_end_y);
                    y += GROUP_END_HEIGHT;
                }
                Event::Space(px_opt) => {
                    y += px_opt.map(|p| p as f64).unwrap_or(20.0);
                    event_y_positions.push(y);
                }
                Event::Activate(_, _) | Event::Deactivate(_) => {
                    event_y_positions.push(y);
                }
                _ => {
                    event_y_positions.push(y);
                }
            }
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
        // Minimum lifeline: 20px, tail overlaps by LIFELINE_Y_OFFSET
        lifeline_top + MIN_LIFELINE_HEIGHT - LIFELINE_Y_OFFSET
    };
    let lifeline_bottom = tail_box_y + LIFELINE_Y_OFFSET;
    let lifeline_height = lifeline_bottom - lifeline_top;

    // SVG dimensions — account for notes that extend beyond participant boxes.
    let last_box_right = if participants.is_empty() {
        100.0
    } else {
        let last = &participants[n - 1];
        last.box_x + last.box_width
    };
    // Check if any note extends beyond the last participant box.
    let mut max_note_right: f64 = 0.0;
    for event in &diagram.events {
        if let Event::Note(note) = event {
            let max_line_width = note
                .text
                .lines()
                .map(|l| text_width(l.trim(), MSG_FONT_SIZE))
                .fold(0.0_f64, f64::max);
            let note_content_w = note_content_width(max_line_width, note.shape);
            match note.position {
                NotePosition::Right => {
                    if let Some(first) = note.participants.first()
                        && let Some(&idx) = id_to_idx.get(first.as_str())
                    {
                        let ll_x = participants[idx].lifeline_line_x;
                        let note_right = ll_x.ceil() + NOTE_LIFELINE_GAP + note_content_w;
                        max_note_right = max_note_right.max(note_right);
                    }
                }
                NotePosition::Over => {
                    if note.participants.is_empty() {
                        // "across" note — starts at HEAD_BOX_Y (5) and extends rightward
                        // by note_content_w. The SVG must be wide enough to contain it.
                        let across_right = HEAD_BOX_Y + note_content_w;
                        max_note_right = max_note_right.max(across_right);
                    } else if note.participants.len() == 1 {
                        if let Some(&idx) = id_to_idx.get(note.participants[0].as_str()) {
                            let ll_x = participants[idx].lifeline_line_x;
                            let note_left = (ll_x - note_content_w / 2.0).max(HEAD_BOX_Y).floor();
                            let note_right = note_left + note_content_w;
                            max_note_right = max_note_right.max(note_right);
                        }
                    } else if let (Some(&first_idx), Some(&last_idx)) = (
                        id_to_idx.get(note.participants.first().unwrap().as_str()),
                        id_to_idx.get(note.participants.last().unwrap().as_str()),
                    ) {
                        let first_ll = participants[first_idx].lifeline_line_x;
                        let last_ll = participants[last_idx].lifeline_line_x;
                        let span = last_ll - first_ll;
                        let note_w = note_content_w.max(span + 38.0);
                        let mid = (first_ll + last_ll) / 2.0;
                        let note_right = mid + note_w / 2.0;
                        max_note_right = max_note_right.max(note_right);
                    }
                }
                NotePosition::Left => {} // left notes don't extend right
            }
        }
    }
    // Add 1.0 for note stroke width when notes extend the right edge.
    let effective_right = last_box_right
        .max(if max_note_right > 0.0 {
            max_note_right + 1.0
        } else {
            0.0
        })
        .max(max_self_msg_right);
    // If groups are present, the group frame may extend beyond participant boxes.
    // Compute the maximum right extent of any group frame (header text + guard).
    let mut max_group_right: f64 = 0.0;
    if !participants.is_empty() {
        let default_fl = participants[0].box_x - GROUP_FRAME_MARGIN;
        let alt_fl = participants[0].box_x + GROUP_FRAME_MARGIN;
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
                let fl = if group_needs_left_shift {
                    default_fl
                } else {
                    alt_fl
                };
                let kw = bold_text_width(kind_str, MSG_FONT_SIZE);
                let tab_right = fl + kw + 45.0;
                let guard_right = if let Some(label) = &g.label {
                    let guard = format!("[{label}]");
                    let gw = bold_text_width(&guard, 11.0);
                    tab_right + 15.0 + gw + 5.0
                } else {
                    tab_right + 5.0
                };
                let last = &participants[n - 1];
                let participant_right = last.box_x + last.box_width + GROUP_FRAME_MARGIN;
                max_group_right = max_group_right.max(guard_right.max(participant_right));
            }
        }
    }
    let has_groups = max_group_right > 0.0;
    // The SVG width must accommodate both participant boxes and group frames.
    // Group frames already include their margin; just add RIGHT_MARGIN + 5.
    let svg_width = if has_groups {
        let from_participants = effective_right + RIGHT_MARGIN + GROUP_FRAME_MARGIN + 5.0;
        let from_groups = max_group_right + RIGHT_MARGIN + 5.0;
        from_participants.max(from_groups).ceil() as u32
    } else {
        (effective_right + RIGHT_MARGIN).ceil() as u32
    };
    let mut svg_height = if diagram.hide_footbox {
        lifeline_bottom.ceil() as u32
    } else {
        (tail_box_y + max_box_h + BOTTOM_MARGIN).ceil() as u32
    };
    // Caption adds 20 px of vertical space below the foot boxes (one 14-px
    // text line + descent + bottom margin). The strict golden height for a
    // basic two-message caption diagram is 172 vs 152 without caption — a
    // delta of 20 pixels that maps to a fixed extension here.
    if diagram.meta.caption.is_some() {
        svg_height += 20;
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
    }

    let mut activation_bars: Vec<ActivationBar> = Vec::new();
    {
        let mut tracker = ActivationTracker::new();
        // Track open activations: (participant_id, event_idx, color, depth)
        let mut open_activations: Vec<(String, usize, Option<String>, usize)> = Vec::new();
        let mut last_event_idx: usize = 0;

        // Count currently open activations for a given participant.
        let open_depth = |open: &[(String, usize, Option<String>, usize)], pid: &str| -> usize {
            open.iter().filter(|(id, _, _, _)| id == pid).count()
        };

        for (ev_idx, event) in diagram.events.iter().enumerate() {
            match event {
                Event::Message(msg) => {
                    last_event_idx = ev_idx;

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
                                ));
                            }
                            ActivationChange::Deactivate => {
                                tracker.deactivate(&msg.from);
                                if let Some(pos) = open_activations
                                    .iter()
                                    .rposition(|(id, _, _, _)| id == &msg.from)
                                {
                                    let (pid, start_idx, color, depth) =
                                        open_activations.remove(pos);
                                    activation_bars.push(ActivationBar {
                                        participant_id: pid,
                                        start_event_idx: start_idx,
                                        end_event_idx: ev_idx,
                                        color,
                                        depth,
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
                    open_activations.push((id.clone(), last_event_idx, color.clone(), depth));
                }
                Event::Deactivate(id) => {
                    tracker.deactivate(id);
                    if let Some(pos) = open_activations
                        .iter()
                        .rposition(|(pid, _, _, _)| pid == id)
                    {
                        let (pid, start_idx, color, depth) = open_activations.remove(pos);
                        activation_bars.push(ActivationBar {
                            participant_id: pid,
                            start_event_idx: start_idx,
                            end_event_idx: last_event_idx,
                            color,
                            depth,
                        });
                    }
                }
                Event::Return(_) => {
                    last_event_idx = ev_idx;
                    // Return deactivates the most recently activated participant
                    if let Some(pos) = open_activations.len().checked_sub(1) {
                        let (pid, start_idx, color, depth) = open_activations.remove(pos);
                        tracker.deactivate(&pid);
                        activation_bars.push(ActivationBar {
                            participant_id: pid,
                            start_event_idx: start_idx,
                            end_event_idx: ev_idx,
                            color,
                            depth,
                        });
                    }
                }
                _ => {}
            }
        }

        // Close any remaining open activations — extend to the last event
        let final_idx = diagram.events.len().saturating_sub(1);
        for (pid, start_idx, color, depth) in open_activations {
            activation_bars.push(ActivationBar {
                participant_id: pid,
                start_event_idx: start_idx,
                end_event_idx: final_idx,
                color,
                depth,
            });
        }

        // Sort by start event index to match PlantUML's rendering order
        activation_bars.sort_by_key(|b| b.start_event_idx);
    }

    // Helper to look up y position for an event
    let event_y =
        |idx: usize| -> f64 { event_y_positions.get(idx).copied().unwrap_or(lifeline_top) };

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

    let mut group_frames: Vec<GroupFrame> = Vec::new();
    {
        // Scan events to find group start/end pairs and compute their frames.
        // Track which participant indices are referenced inside each group.
        // (min_idx, max_idx, start_event_idx)
        let mut group_start_stack: Vec<(usize, usize, usize)> = Vec::new();
        for (ev_idx, event) in diagram.events.iter().enumerate() {
            match event {
                Event::GroupStart(_) => {
                    group_start_stack.push((usize::MAX, 0, ev_idx));
                }
                Event::GroupEnd => {
                    if let Some((min_idx, max_idx, start_idx)) = group_start_stack.pop() {
                        let frame_top = event_y_positions[start_idx];
                        let frame_bottom = event_y_positions[ev_idx];

                        // Compute the header text right edge (group kind label + guard).
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
                            let fl = if min_idx <= max_idx && !participants.is_empty() {
                                participants[min_idx].box_x - GROUP_FRAME_MARGIN
                            } else if !participants.is_empty() {
                                participants[0].box_x + GROUP_FRAME_MARGIN
                            } else {
                                HEAD_BOX_Y
                            };
                            let kw = bold_text_width(kind_str, MSG_FONT_SIZE);
                            let tab_right = fl + kw + 45.0;
                            if let Some(label) = &g.label {
                                let guard = format!("[{label}]");
                                let gw = bold_text_width(&guard, 11.0);
                                tab_right + 15.0 + gw + 5.0
                            } else {
                                tab_right + 5.0
                            }
                        } else {
                            0.0
                        };

                        // Compute frame left/right based on which participants are inside.
                        let (frame_left, frame_right) =
                            if min_idx <= max_idx && !participants.is_empty() {
                                // Group has messages — frame encompasses those participants
                                let fl = participants[min_idx].box_x - GROUP_FRAME_MARGIN;
                                let fr = (participants[max_idx].box_x
                                    + participants[max_idx].box_width
                                    + GROUP_FRAME_MARGIN)
                                    .max(header_right);
                                (fl, fr)
                            } else if !participants.is_empty() {
                                // Empty group — frame left at first box + margin
                                let fl = participants[0].box_x + GROUP_FRAME_MARGIN;

                                let last = &participants[n - 1];
                                let participant_right =
                                    last.box_x + last.box_width + GROUP_FRAME_MARGIN;
                                let fr = participant_right.max(header_right);
                                (fl, fr)
                            } else {
                                (HEAD_BOX_Y, 100.0)
                            };

                        group_frames.push(GroupFrame {
                            top: frame_top,
                            bottom: frame_bottom,
                            left: frame_left,
                            right: frame_right,
                            event_idx: start_idx,
                        });
                    }
                }
                Event::Message(msg) if !group_start_stack.is_empty() => {
                    let fi = id_to_idx.get(msg.from.as_str()).copied();
                    let ti = id_to_idx.get(msg.to.as_str()).copied();
                    if let Some(top) = group_start_stack.last_mut() {
                        if let Some(fi) = fi {
                            top.0 = top.0.min(fi);
                            top.1 = top.1.max(fi);
                        }
                        if let Some(ti) = ti {
                            top.0 = top.0.min(ti);
                            top.1 = top.1.max(ti);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Recalculate svg_width after group frames are computed, since the frame
    // right edges may exceed the initial estimate (e.g., when group labels extend
    // beyond participant boxes).
    let svg_width = if !group_frames.is_empty() {
        let max_frame_right = group_frames.iter().map(|f| f.right).fold(0.0f64, f64::max);
        let from_frames = max_frame_right + RIGHT_MARGIN + 5.0;
        let from_participants = effective_right + RIGHT_MARGIN + GROUP_FRAME_MARGIN + 5.0;
        from_participants.max(from_frames).ceil() as u32
    } else {
        svg_width
    };

    // -----------------------------------------------------------------------
    // Phase 6: Generate SVG
    // -----------------------------------------------------------------------

    let mut svg = PlantUmlSvg::new();
    svg.arrow_thickness = default_arrow_thickness.to_string();
    svg.participant_border = participant_border.clone();
    svg.participant_border_thickness = participant_border_thickness.clone();
    svg.lifeline_border = lifeline_border.clone();
    svg.lifeline_border_thickness = lifeline_border_thickness.clone();
    svg.open_svg(svg_width, svg_height);

    // Emit handwritten warning if present
    let is_handwritten = diagram
        .meta
        .skinparams
        .iter()
        .any(|sp| sp.key.to_lowercase() == "handwritten" && sp.value.to_lowercase() == "true");
    if is_handwritten {
        let nbsp = '\u{00a0}';
        let msg = format!(
            "Please{n}use{n}'!option{n}handwritten{n}true'{n}to{n}enable{n}handwritten",
            n = nbsp
        );
        let mid_x = svg_width as f64 / 2.0;
        text_render::emit_text(
            &mut svg.buf,
            &msg,
            &TextBase {
                x: mid_x,
                y: HEAD_BOX_Y + 13.0,
                font_size: 11,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
    }

    // Render title if present. PlantUML wraps the title in
    // `<g class="title" data-source-line="N">` and emits a bold 14pt text
    // per line. Each line is centered around the midpoint between the first
    // participant's box left edge and the last participant's box right edge
    // (minus 0.5 px), and the first baseline sits at HEAD_BOX_Y + TITLE_TOP_PAD
    // + ascent(14). Subsequent lines step down by text_height(14).
    if !title_lines.is_empty() {
        let title_center =
            if let (Some(first), Some(last)) = (participants.first(), participants.last()) {
                (first.box_x + last.box_x + last.box_width - 1.0) / 2.0
            } else {
                svg_width as f64 / 2.0 - 0.5
            };
        let line_height = plantuml_metrics::text_height(TITLE_FONT_SIZE as f64);
        let first_baseline_y =
            HEAD_BOX_Y + TITLE_TOP_PAD + plantuml_metrics::ascent(TITLE_FONT_SIZE as f64);
        svg.buf
            .push_str(r#"<g class="title" data-source-line="1">"#);
        for (i, line) in title_lines.iter().enumerate() {
            let text_length = text_render::measure(line, TITLE_FONT_SIZE as f64, true);
            let x = title_center - text_length / 2.0;
            let y = first_baseline_y + i as f64 * line_height;
            text_render::emit_text(
                &mut svg.buf,
                line,
                &TextBase {
                    x,
                    y,
                    font_size: TITLE_FONT_SIZE,
                    font_family: "sans-serif",
                    fill: "#000000",
                    bold: true,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
        svg.buf.push_str("</g>");
    }

    // Render header if present. PlantUML wraps in `<g class="header">` and
    // emits a 10pt #888888 text right-aligned to a small inset from the
    // right edge: x = svg_width - textLength - 5.
    if let Some(header) = &diagram.meta.header {
        const HEADER_FONT_SIZE: u32 = 10;
        let text_length = text_render::measure(header, HEADER_FONT_SIZE as f64, false);
        let x = svg_width as f64 - text_length - 5.0;
        svg.buf
            .push_str(r#"<g class="header" data-source-line="1">"#);
        text_render::emit_text(
            &mut svg.buf,
            header,
            &TextBase {
                x,
                y: 14.668,
                font_size: HEADER_FONT_SIZE,
                font_family: "sans-serif",
                fill: "#888888",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.buf.push_str("</g>");
    }

    // Render footer if present. PlantUML wraps in `<g class="footer">` and
    // emits a 10pt #888888 text at the left edge (x=0).
    if let Some(footer) = &diagram.meta.footer {
        const FOOTER_FONT_SIZE: u32 = 10;
        svg.buf
            .push_str(r#"<g class="footer" data-source-line="1">"#);
        text_render::emit_text(
            &mut svg.buf,
            footer,
            &TextBase {
                x: 0.0,
                y: svg_height as f64 - 4.0,
                font_size: FOOTER_FONT_SIZE,
                font_family: "sans-serif",
                fill: "#888888",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.buf.push_str("</g>");
    }

    // Caption is rendered AFTER all messages — see the dedicated block just
    // before `svg.close_svg(...)` at the end of this function. PlantUML emits
    // the caption group as the last visible element inside `<g>`.

    // Render legend if present. Pass the raw legend line through the creole
    // segmenter so bold/italic/under runs split into separate <text> elements.
    if let Some(legend) = &diagram.meta.legend {
        let lx = svg_width as f64 - 200.0;
        let mut ly = svg_height as f64 - 150.0;
        for line in legend.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let label = if trimmed.contains('|') {
                    // Table cells: extract cell text. (Strip HTML cell decoration.)
                    trimmed
                        .trim_matches('|')
                        .split('|')
                        .map(|cell| {
                            let cell = cell.trim();
                            if cell.starts_with('<')
                                && let Some(pos) = cell.find('>')
                            {
                                return cell[pos + 1..].trim();
                            }
                            cell
                        })
                        .filter(|c| !c.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    trimmed.to_string()
                };
                if !label.is_empty() {
                    text_render::emit_text(
                        &mut svg.buf,
                        &label,
                        &TextBase {
                            x: lx,
                            y: ly,
                            font_size: 11,
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
            ly += 14.0;
        }
    }

    // Activation bars are rendered BEFORE group frames and lifelines in PlantUML's SVG.
    // Order: activation bars, group frame rects, lifelines, participants, activation bars
    // again, then messages.

    // First pass: activation bars (rendered twice in PlantUML's SVG)
    for bar in &activation_bars {
        let cx = center_of(&bar.participant_id);
        let bar_x = cx - ACTIVATION_HALF_W + (bar.depth as f64 * ACTIVATION_HALF_W);
        let bar_y = event_y(bar.start_event_idx);
        let bar_end_y = event_y(bar.end_event_idx);
        let bar_h = bar_end_y - bar_y;
        let title = &participants
            .iter()
            .find(|p| p.id == bar.participant_id)
            .map(|p| p.label.clone())
            .unwrap_or_default();
        let fill_color = bar
            .color
            .as_ref()
            .map(|c| resolve_color(c))
            .unwrap_or_else(|| "#FFFFFF".to_string());
        svg.activation_bar(title, bar_x, bar_y, bar_h, &fill_color);
    }

    // Group frame rects (first instance) — rendered after first activation bars pass.
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

    // Lifelines
    let source_line_for = |id: &str| -> u32 {
        diagram
            .participants
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.source_line as u32)
            .unwrap_or(1)
    };

    for p in &participants {
        let part_uid = format!("part{}", p.idx + 1);
        let ll_rect_x = p.center_x - LIFELINE_RECT_WIDTH / 2.0;
        svg.lifeline(
            &part_uid,
            &p.id,
            source_line_for(&p.id),
            &p.label,
            ll_rect_x,
            lifeline_top,
            lifeline_height,
            p.lifeline_line_x, // PlantUML uses box_x + floor(box_width/2) for the dashed line
            lifeline_top,
            lifeline_bottom,
        );
    }

    // Participant head and tail boxes (interleaved per participant, matching PlantUML order).
    // Non-rectangle shapes (actor, boundary, etc.) are bottom-aligned: their box_y is
    // adjusted so that box_y + box_height == HEAD_BOX_Y + max_box_h (matching the tallest).
    // For tail boxes, the same alignment applies relative to tail_box_y.
    for (i, p) in participants.iter().enumerate() {
        let part_uid = format!("part{}", p.idx + 1);
        let sl = source_line_for(&diagram.participants[i].id);

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
        let fill_color = diagram.participants[i]
            .color
            .as_ref()
            .map(|c| resolve_color(c))
            .or(kind_specific_fill)
            .unwrap_or_else(|| participant_fill.clone());

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
        );

        // Tail (skip if hide footbox) — all participants start at tail_box_y
        // (no bottom-alignment offset; the SVG height accounts for max_box_h).
        if !diagram.hide_footbox {
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
            );
        }
    }

    // Second pass: activation bars again (PlantUML renders them twice)
    for bar in &activation_bars {
        let cx = center_of(&bar.participant_id);
        let bar_x = cx - ACTIVATION_HALF_W + (bar.depth as f64 * ACTIVATION_HALF_W);
        let bar_y = event_y(bar.start_event_idx);
        let bar_end_y = event_y(bar.end_event_idx);
        let bar_h = bar_end_y - bar_y;
        let title = &participants
            .iter()
            .find(|p| p.id == bar.participant_id)
            .map(|p| p.label.clone())
            .unwrap_or_default();
        let fill_color = bar
            .color
            .as_ref()
            .map(|c| resolve_color(c))
            .unwrap_or_else(|| "#FFFFFF".to_string());
        svg.activation_bar(title, bar_x, bar_y, bar_h, &fill_color);
    }

    // Messages — use pre-computed y positions from event_y_positions
    let mut msg_id: u32 = 0;
    let mut auto_num: Option<u32> = diagram.autonumber.as_ref().map(|an| an.start);
    // Track activation depth during message rendering to adjust arrow positions.
    let mut render_activation: HashMap<String, usize> = HashMap::new();

    // Return stack: tracks (activated_participant, activating_sender) for `return` keyword.
    // Return stack: (activated_participant, sender, is_open_arrow)
    let mut return_stack: Vec<(String, String, bool)> = Vec::new();

    let events = &diagram.events;
    for (ev_idx, event) in events.iter().enumerate() {
        let msg_y = event_y_positions[ev_idx];
        match event {
            Event::Message(msg) => {
                msg_id += 1;

                let from_x = center_of(&msg.from);
                let to_x = center_of(&msg.to);
                let is_self = msg.from == msg.to;
                let is_right = to_x > from_x;
                let is_dotted = msg.arrow.line == LineStyle::Dotted;
                let is_open = msg.arrow.head == ArrowHead::Open;
                let is_cross = msg.arrow.head == ArrowHead::Cross;

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
                let label_w = text_width(&label, MSG_FONT_SIZE);

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
                    .map(|i| format!("part{}", i + 1))
                    .unwrap_or_default();
                let to_uid = id_to_idx
                    .get(msg.to.as_str())
                    .map(|i| format!("part{}", i + 1))
                    .unwrap_or_default();

                let src_line = msg.source_line as u32;

                // Compute autonumber info for passing into the message group
                let autonumber_info: Option<(String, f64, AutoNumberStyle)> =
                    auto_num.as_ref().map(|n| {
                        let an = diagram.autonumber.as_ref().unwrap();
                        let num_text = format_autonumber(*n, &an.format);
                        let style = AutoNumberStyle::from_format(&an.format);
                        let num_w = if style.bold {
                            bold_text_width(&num_text, MSG_FONT_SIZE)
                        } else {
                            text_width(&num_text, MSG_FONT_SIZE)
                        };
                        (num_text, num_w, style)
                    });
                let autonumber_ref = autonumber_info
                    .as_ref()
                    .map(|(t, w, s)| (t.as_str(), *w, s));

                if is_self {
                    // Self-message: U-shaped loopback. When the participant is
                    // activated, the loop starts from the activation bar's right
                    // edge (lifeline center + ACTIVATION_HALF_W).
                    let cx = if from_active || to_active {
                        from_x + ACTIVATION_HALF_W
                    } else {
                        from_x
                    };
                    let loop_right = cx + SELF_MSG_EXTEND;
                    let loop_bottom = msg_y + SELF_MSG_DROP;
                    let text_x = cx + SELF_MSG_TEXT_X_PAD;
                    let text_y_pos = msg_y - 4.742187500;

                    // Open the message group
                    write!(
                        svg.buf,
                        r##"<g class="message" data-entity-1="{}" data-entity-2="{}" data-source-line="{}" id="msg{}">"##,
                        escape_xml(&from_uid),
                        escape_xml(&to_uid),
                        src_line,
                        msg_id,
                    )
                    .unwrap();

                    // Three lines forming the U-shape: right, down, left
                    // Line 1: horizontal right (from center to loop right)
                    write!(
                        svg.buf,
                        r##"<line style="stroke:{};stroke-width:1;{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                        &arrow_color,
                        line_style,
                        fmt_coord(cx),
                        fmt_coord(loop_right),
                        fmt_coord(msg_y),
                        fmt_coord(msg_y),
                    )
                    .unwrap();

                    // Line 2: vertical down
                    write!(
                        svg.buf,
                        r##"<line style="stroke:{};stroke-width:1;{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                        &arrow_color,
                        line_style,
                        fmt_coord(loop_right),
                        fmt_coord(loop_right),
                        fmt_coord(msg_y),
                        fmt_coord(loop_bottom),
                    )
                    .unwrap();

                    // Line 3: horizontal left (from loop right back toward lifeline)
                    // For filled arrows, the return line starts 1px right of center
                    // For open arrows, the return line starts at center
                    let return_left = if is_open {
                        cx // open: line goes to center
                    } else {
                        cx + 1.0 // filled: line stops 1px right (polygon takes over)
                    };
                    write!(
                        svg.buf,
                        r##"<line style="stroke:{};stroke-width:1;{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                        &arrow_color,
                        line_style,
                        fmt_coord(return_left),
                        fmt_coord(loop_right),
                        fmt_coord(loop_bottom),
                        fmt_coord(loop_bottom),
                    )
                    .unwrap();

                    // Arrow head at bottom-left
                    if is_open {
                        // Open arrow: two V-shape lines
                        let tip_x = cx + 1.0;
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
                        let tip_x = cx + 1.0;
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
                        text_render::emit_text(
                            &mut svg.buf,
                            &label,
                            &TextBase {
                                x: text_x,
                                y: text_y_pos,
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

                    svg.buf.push_str("</g>");
                } else {
                    // Source shift: when the source is activated and sending right,
                    // the message line starts from the activation bar's right edge.
                    // For left-pointing messages, PlantUML does NOT shift the source endpoint.
                    let from_x_shifted = if is_right && from_active {
                        from_x + ACTIVATION_HALF_W
                    } else {
                        from_x
                    };

                    // Target shift: when the target is activated, the arrow tip
                    // stops at the activation bar edge.
                    let target_shift = if to_active { ACTIVATION_HALF_W } else { 0.0 };

                    // Text position
                    let text_y_pos = msg_y - 4.742187500;
                    let text_x = if is_right {
                        from_x_shifted + MSG_TEXT_LEFT_PAD
                    } else {
                        to_x + target_shift + LEFT_ARROW_TEXT_PAD + 1.0
                    };

                    if is_right {
                        let tip_x = to_x - target_shift - ARROW_TIP_GAP;
                        let line_x2 = if is_open {
                            tip_x
                        } else {
                            tip_x - FILLED_ARROW_NOTCH
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
                            );
                        } else {
                            // Filled arrow polygon
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
                                &arrow_pts,
                                from_x_shifted,
                                line_x2,
                                msg_y,
                                line_style,
                                text_x,
                                text_y_pos,
                                &label,
                                label_w,
                                &arrow_color,
                                autonumber_ref,
                            );
                        }
                    } else {
                        // Left-pointing arrow: tip offset accounts for target activation
                        let tip_x = to_x + target_shift + 1.0;
                        let line_x1 = if is_open {
                            tip_x
                        } else {
                            tip_x + FILLED_ARROW_NOTCH
                        };
                        // PlantUML draws the left-going line to from edge - 1
                        let line_x2_end = from_x_shifted - 1.0;

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
                            );
                        }
                    }
                } // end non-self message else

                // Update activation state and return stack after this message
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
                if let Some(n) = auto_num.as_mut() {
                    let an = diagram.autonumber.as_ref().unwrap();
                    *n = n.saturating_add(an.step);
                }
            }
            Event::Return(ret) => {
                msg_id += 1;

                // Compute autonumber info for return messages
                let ret_autonumber_info: Option<(String, f64, AutoNumberStyle)> =
                    auto_num.as_ref().map(|n| {
                        let an = diagram.autonumber.as_ref().unwrap();
                        let num_text = format_autonumber(*n, &an.format);
                        let style = AutoNumberStyle::from_format(&an.format);
                        let num_w = if style.bold {
                            bold_text_width(&num_text, MSG_FONT_SIZE)
                        } else {
                            text_width(&num_text, MSG_FONT_SIZE)
                        };
                        (num_text, num_w, style)
                    });
                let ret_autonumber_ref = ret_autonumber_info
                    .as_ref()
                    .map(|(t, w, s)| (t.as_str(), *w, s));

                // Pop the return stack to find from/to participants and arrow style
                let (ret_from, ret_to, ret_open) = if let Some(entry) = return_stack.pop() {
                    // Deactivate the returned-from participant
                    if let Some(d) = render_activation.get_mut(&entry.0) {
                        *d = d.saturating_sub(1);
                    }
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
                    .map(|i| format!("part{}", i + 1))
                    .unwrap_or_default();
                let to_uid = id_to_idx
                    .get(ret_to.as_str())
                    .map(|i| format!("part{}", i + 1))
                    .unwrap_or_default();

                let label = if ret.label.is_empty() {
                    String::new()
                } else {
                    decode_escapes(&ret.label)
                };
                let label_w = text_width(&label, MSG_FONT_SIZE);

                let src_line = ret.source_line as u32;
                let text_y_pos = msg_y - 4.742187500;

                // Return messages are always dotted; arrow style matches the original
                let line_style = "stroke-dasharray:2,2;";

                if is_right {
                    // Right-pointing return (unusual but possible)
                    let tip_x = to_x - ARROW_TIP_GAP;
                    let text_x = from_x + MSG_TEXT_LEFT_PAD;
                    if ret_open {
                        svg.message_open_arrow(
                            &from_uid,
                            &to_uid,
                            src_line,
                            msg_id,
                            tip_x,
                            msg_y,
                            true,
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
                        );
                    }
                } else {
                    // Left-pointing return (normal case)
                    let to_active =
                        render_activation.get(ret_to.as_str()).copied().unwrap_or(0) > 0;
                    let target_shift = if to_active { ACTIVATION_HALF_W } else { 0.0 };
                    let tip_x = to_x + target_shift + 1.0;
                    let line_x2_end = from_x - 1.0;
                    let text_x = to_x + target_shift + LEFT_ARROW_TEXT_PAD + 1.0;

                    if ret_open {
                        svg.message_open_arrow(
                            &from_uid,
                            &to_uid,
                            src_line,
                            msg_id,
                            tip_x,
                            msg_y,
                            false,
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
                        );
                    }
                }

                // Advance autonumber
                if let Some(n) = auto_num.as_mut() {
                    let an = diagram.autonumber.as_ref().unwrap();
                    *n = n.saturating_add(an.step);
                }
            }
            Event::Divider(text) => {
                // PlantUML renders dividers as:
                // 1. A background strip rect (EEEEEE, 3px high)
                // 2. Two horizontal lines (3px apart)
                // 3. A label box rect (EEEEEE, bordered)
                // 4. Bold text inside the label box
                let tw = bold_text_width(text, MSG_FONT_SIZE);
                let (line_left, line_right) = if !participants.is_empty() {
                    let last = &participants[participants.len() - 1];
                    (0.0, last.box_x + last.box_width + 5.0)
                } else {
                    (0.0, 200.0)
                };
                let mid_x = (line_left + line_right) / 2.0;

                // Event_y is the text baseline position.
                let text_y = msg_y;
                let line2_y = text_y - 2.9131;
                let line1_y = line2_y - 3.0;

                // Label box dimensions: 6px padding on each side, centered on divider
                let label_box_w = tw + 2.0 * 6.0 + 6.2847; // PlantUML adds extra padding
                let label_box_h = 23.3105;
                let label_box_x = mid_x - label_box_w / 2.0;
                let label_box_y = line1_y - 10.6553; // Box extends above the lines
                let text_x = label_box_x + 6.0;

                // 1. Background strip rect
                write!(
                    svg.buf,
                    r##"<rect fill="#EEEEEE" height="3" style="stroke:#EEEEEE;stroke-width:1;" width="{}" x="{}" y="{}"/>"##,
                    fmt_coord(line_right - line_left),
                    fmt_coord(line_left),
                    fmt_coord(line1_y),
                )
                .unwrap();

                // 2. First horizontal line
                write!(
                    svg.buf,
                    r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                    fmt_coord(line_left),
                    fmt_coord(line_right),
                    fmt_coord(line1_y),
                    fmt_coord(line1_y),
                )
                .unwrap();

                // 3. Second horizontal line
                write!(
                    svg.buf,
                    r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                    fmt_coord(line_left),
                    fmt_coord(line_right),
                    fmt_coord(line2_y),
                    fmt_coord(line2_y),
                )
                .unwrap();

                // 4. Label box rect
                write!(
                    svg.buf,
                    r##"<rect fill="#EEEEEE" height="{}" style="stroke:#000000;stroke-width:2;" width="{}" x="{}" y="{}"/>"##,
                    fmt_coord(label_box_h),
                    fmt_coord(label_box_w),
                    fmt_coord(label_box_x),
                    fmt_coord(label_box_y),
                )
                .unwrap();

                // 5. Bold text
                text_render::emit_text(
                    &mut svg.buf,
                    text,
                    &TextBase {
                        x: text_x,
                        y: text_y,
                        font_size: 13,
                        font_family: "sans-serif",
                        fill: "#000000",
                        bold: true,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );
            }
            Event::Delay(Some(t)) => {
                let mid_x = if !participants.is_empty() {
                    (participants[0].center_x + participants[participants.len() - 1].center_x) / 2.0
                } else {
                    50.0
                };
                text_render::emit_text(
                    &mut svg.buf,
                    t,
                    &TextBase {
                        x: mid_x,
                        y: msg_y + 5.0,
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
            Event::Delay(None) => {}
            Event::Space(_px_opt) => {
                // y position already accounted for in event_y_positions
            }
            Event::Note(note) => {
                // Compute note dimensions and position.
                let lines: Vec<&str> = note.text.lines().collect();
                let num_lines = lines.len().max(1);
                let (base_h, note_y_extra) = match note.shape {
                    NoteShape::Note => (NOTE_BASE_HEIGHT, 7.0),
                    NoteShape::Hexagonal | NoteShape::Rectangular => (HNOTE_BASE_HEIGHT, 5.0),
                };
                let note_height = base_h + (num_lines as f64 - 1.0) * NOTE_LINE_HEIGHT;

                // Derive note_top from the event y:
                // event_y = note_top + note_y_extra + num_lines * MSG_TEXT_HEIGHT
                let note_top = msg_y - note_y_extra - num_lines as f64 * MSG_TEXT_HEIGHT;
                let note_bottom = note_top + note_height;

                // Compute max text width across all lines.
                let max_text_w = lines
                    .iter()
                    .map(|l| text_width(l.trim(), MSG_FONT_SIZE))
                    .fold(0.0_f64, f64::max);
                let note_content_w = note_content_width(max_text_w, note.shape);

                // Compute note left/right based on position.
                let (note_left, note_right) = match note.position {
                    NotePosition::Right => {
                        let ll_x = note
                            .participants
                            .first()
                            .and_then(|id| id_to_idx.get(id.as_str()))
                            .map(|&i| participants[i].lifeline_line_x)
                            .unwrap_or(50.0);
                        let left = ll_x.ceil() + NOTE_LIFELINE_GAP;
                        (left, left + note_content_w)
                    }
                    NotePosition::Left => {
                        let ll_x = note
                            .participants
                            .first()
                            .and_then(|id| id_to_idx.get(id.as_str()))
                            .map(|&i| participants[i].lifeline_line_x)
                            .unwrap_or(50.0);
                        let right = ll_x.floor() - NOTE_LIFELINE_GAP;
                        (right - note_content_w, right)
                    }
                    NotePosition::Over => {
                        if note.participants.is_empty() {
                            // "across" note — starts at HEAD_BOX_Y and extends by note_content_w
                            (HEAD_BOX_Y, HEAD_BOX_Y + note_content_w)
                        } else if note.participants.len() == 1 {
                            // Centered on participant's lifeline_line_x (integer-based).
                            // PlantUML floors the left position.
                            let ll_x = note
                                .participants
                                .first()
                                .and_then(|id| id_to_idx.get(id.as_str()))
                                .map(|&i| participants[i].lifeline_line_x)
                                .unwrap_or(50.0);
                            let half_w = note_content_w / 2.0;
                            let left = (ll_x - half_w).max(HEAD_BOX_Y).floor();
                            (left, left + note_content_w)
                        } else {
                            // Spanning multiple participants.
                            // When the note text fits within the participant span + margins,
                            // position at first_ll - 19 .. last_ll + 19.
                            // When the text is wider, center the note around the midpoint
                            // and round the left edge.
                            let first_ll = note
                                .participants
                                .first()
                                .and_then(|id| id_to_idx.get(id.as_str()))
                                .map(|&i| participants[i].lifeline_line_x)
                                .unwrap_or(20.0);
                            let last_ll = note
                                .participants
                                .last()
                                .and_then(|id| id_to_idx.get(id.as_str()))
                                .map(|&i| participants[i].lifeline_line_x)
                                .unwrap_or(80.0);
                            let span = last_ll - first_ll;
                            if note_content_w <= span + 38.0 {
                                // Text fits: use fixed margins
                                let left = first_ll - 19.0;
                                let right = last_ll + 19.0;
                                (left.max(HEAD_BOX_Y), right)
                            } else {
                                // Text wider than span: center and round
                                let mid = (first_ll + last_ll) / 2.0;
                                let left = (mid - note_content_w / 2.0).max(HEAD_BOX_Y).round();
                                (left, left + note_content_w)
                            }
                        }
                    }
                };

                // Resolve note fill color.
                let note_fill = note
                    .color
                    .as_ref()
                    .map(|c| resolve_color(c))
                    .unwrap_or_else(|| NOTE_FILL.to_string());

                match note.shape {
                    NoteShape::Hexagonal => {
                        // Hexagonal note (hnote): 7-point polygon.
                        // Points: TL, TR, R, BR, BL, L, TL (closed polygon)
                        // PlantUML uses floor(height/2) for the y indent, making
                        // the hexagon slightly asymmetric when height is odd.
                        let mid_y = note_top + (note_height / 2.0).floor();
                        let li = note_left + HNOTE_INDENT; // left indent x
                        let ri = note_right - HNOTE_INDENT; // right indent x
                        write!(
                            svg.buf,
                            r##"<polygon fill="{fill}" points="{li},{top},{ri},{top},{nr},{mid},{ri},{bot},{li},{bot},{nl},{mid},{li},{top}" style="stroke:#181818;stroke-width:0.5;"/>"##,
                            fill = note_fill,
                            li = fmt_coord(li),
                            top = fmt_coord(note_top),
                            ri = fmt_coord(ri),
                            nr = fmt_coord(note_right),
                            mid = fmt_coord(mid_y),
                            bot = fmt_coord(note_bottom),
                            nl = fmt_coord(note_left),
                        )
                        .unwrap();
                    }
                    NoteShape::Rectangular => {
                        // Rectangular note (rnote): a simple rectangle.
                        write!(
                            svg.buf,
                            r##"<rect fill="{fill}" height="{h}" style="stroke:#181818;stroke-width:0.5;" width="{w}" x="{x}" y="{y}"/>"##,
                            fill = note_fill,
                            h = fmt_coord(note_bottom - note_top),
                            w = fmt_coord(note_right - note_left),
                            x = fmt_coord(note_left),
                            y = fmt_coord(note_top),
                        )
                        .unwrap();
                    }
                    NoteShape::Note => {
                        // Standard note with folded corner.
                        let fold_x = note_right - NOTE_FOLD_SIZE;
                        let fold_y = note_top + NOTE_FOLD_SIZE;
                        write!(
                            svg.buf,
                            r##"<path d="M{left},{top} L{left},{bottom} L{right},{bottom} L{right},{fold_y} L{fold_x},{top} L{left},{top}" fill="{fill}" style="stroke:#181818;stroke-width:0.5;"/>"##,
                            left = fmt_coord(note_left),
                            top = fmt_coord(note_top),
                            bottom = fmt_coord(note_bottom),
                            right = fmt_coord(note_right),
                            fold_y = fmt_coord(fold_y),
                            fold_x = fmt_coord(fold_x),
                            fill = note_fill,
                        )
                        .unwrap();

                        // Emit the fold triangle.
                        write!(
                            svg.buf,
                            r##"<path d="M{fold_x},{top} L{fold_x},{fold_y} L{right},{fold_y} L{fold_x},{top}" fill="{fill}" style="stroke:#181818;stroke-width:0.5;"/>"##,
                            fold_x = fmt_coord(fold_x),
                            top = fmt_coord(note_top),
                            fold_y = fmt_coord(fold_y),
                            right = fmt_coord(note_right),
                            fill = note_fill,
                        )
                        .unwrap();
                    }
                }

                // Emit note text lines.
                let (text_x, text_y_offset) = match note.shape {
                    NoteShape::Note => (note_left + NOTE_TEXT_X_PAD, NOTE_TEXT_Y_OFFSET),
                    NoteShape::Hexagonal => (note_left + HNOTE_INDENT + 2.0, HNOTE_TEXT_Y_OFFSET),
                    NoteShape::Rectangular => (note_left + RNOTE_TEXT_X_PAD, HNOTE_TEXT_Y_OFFSET),
                };
                let mut text_y = note_top + text_y_offset;
                for line in &lines {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        text_y += NOTE_TEXT_LINE_SPACING;
                        continue;
                    }
                    text_render::emit_text(
                        &mut svg.buf,
                        trimmed,
                        &TextBase {
                            x: text_x,
                            y: text_y,
                            font_size: 13,
                            font_family: "sans-serif",
                            fill: "#000000",
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: false,
                        },
                    );
                    text_y += NOTE_TEXT_LINE_SPACING;
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
                        GROUP_FRAME_MARGIN
                    } else {
                        participants[0].box_x - GROUP_FRAME_MARGIN
                    };
                    let fr = if participants.is_empty() {
                        100.0
                    } else {
                        let last = &participants[n - 1];
                        last.box_x + last.box_width + GROUP_FRAME_MARGIN
                    };
                    (fl, fr, msg_y, 50.0)
                };

                // Emit header tab FIRST (pentagon shape), then frame rect, then text.
                // This matches PlantUML's SVG element order.
                let kind_w = bold_text_width(kind_str, MSG_FONT_SIZE);
                let tab_right = frame_left + kind_w + 45.0;
                let tab_bottom_left = frame_top + 17.3106;
                let tab_bottom_right = frame_top + 7.3106;
                write!(
                    svg.buf,
                    r##"<path d="M{left},{top} L{right},{top} L{right},{br} L{diag},{bl} L{left},{bl} L{left},{top}" fill="#EEEEEE" style="stroke:#000000;stroke-width:1.5;"/>"##,
                    left = fmt_coord(frame_left),
                    top = fmt_coord(frame_top),
                    right = fmt_coord(tab_right),
                    br = fmt_coord(tab_bottom_right),
                    diag = fmt_coord(tab_right - 10.0),
                    bl = fmt_coord(tab_bottom_left),
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

                // Emit kind text (bold)
                text_render::emit_text(
                    &mut svg.buf,
                    kind_str,
                    &TextBase {
                        x: frame_left + 15.0,
                        y: frame_top + 13.5684,
                        font_size: 13,
                        font_family: "sans-serif",
                        fill: "#000000",
                        bold: true,
                        italic: false,
                        underline: false,
                        skip_underline: false,
                    },
                );

                // Emit guard label if present (in brackets)
                if let Some(label) = &g.label {
                    let guard = format!("[{label}]");
                    text_render::emit_text(
                        &mut svg.buf,
                        &guard,
                        &TextBase {
                            x: tab_right + 15.0,
                            y: frame_top + 12.6348,
                            font_size: 11,
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
            Event::GroupElse(g) => {
                // Emit else dashed divider line
                // Find the enclosing group frame
                let frame_left = if participants.is_empty() {
                    GROUP_FRAME_MARGIN
                } else {
                    participants[0].box_x - GROUP_FRAME_MARGIN
                };
                let frame_right = if participants.is_empty() {
                    100.0
                } else {
                    let last = &participants[n - 1];
                    last.box_x + last.box_width + GROUP_FRAME_MARGIN
                };
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
                    let label_text = format!("[{label}]");
                    text_render::emit_text(
                        &mut svg.buf,
                        &label_text,
                        &TextBase {
                            x: frame_left + 5.0,
                            y: msg_y + 10.63475,
                            font_size: 11,
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
            Event::GroupEnd => {
                // Group end is handled by the frame rect emitted at GroupStart
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
                let mid_x = if !participants.is_empty() {
                    (participants[0].center_x + participants[participants.len() - 1].center_x) / 2.0
                } else {
                    50.0
                };
                text_render::emit_text(
                    &mut svg.buf,
                    &r.text,
                    &TextBase {
                        x: mid_x,
                        y: msg_y + 4.0,
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
            Event::Activate(id, _) => {
                // Track activation state for message rendering
                *render_activation.entry(id.clone()).or_default() += 1;
            }
            Event::Deactivate(id) => {
                if let Some(d) = render_activation.get_mut(id) {
                    *d = d.saturating_sub(1);
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
            _ => {
                // Remaining events (Create, NewPage)
                // don't emit visible text labels or change activation state.
            }
        }
    }

    // Caption appears at the bottom of the diagram, AFTER messages.
    // PlantUML wraps it in `<g class="caption" data-source-line="N">` and
    // routes the text through the creole segmenter so bold/italic/under runs
    // split into separate `<text>` elements at calculated x offsets.
    if let Some(caption) = &diagram.meta.caption {
        const CAPTION_FONT_SIZE: u32 = 14;
        const CAPTION_BOTTOM_OFFSET: f64 = 10.8672;
        // We don't yet track caption_line in DiagramMeta for sequence diagrams,
        // so fall back to 1 (matches captions defined at top of source files).
        let src_line: u32 = 1;
        write!(
            svg.buf,
            r#"<g class="caption" data-source-line="{src_line}">"#
        )
        .unwrap();
        text_render::emit_text(
            &mut svg.buf,
            caption,
            &TextBase {
                x: 1.0,
                y: svg_height as f64 - CAPTION_BOTTOM_OFFSET,
                font_size: CAPTION_FONT_SIZE,
                font_family: "sans-serif",
                fill: "#000000",
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
                },
                activation: None,
                activation_color: None,
                source_line: 1,
            })],
            autonumber: None,
            hide_footbox: false,
        }
    }

    #[test]
    fn produces_valid_svg() {
        let svg = render(&simple_diagram(), &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Alice"));
        assert!(svg.contains("Bob"));
        assert!(svg.contains("hello"));
    }

    #[test]
    fn has_participant_boxes() {
        let svg = render(&simple_diagram(), &Theme::default());
        // Two boxes at top, two at bottom.
        let rect_count = svg.matches("<rect").count();
        assert!(
            rect_count >= 4,
            "should have at least 4 rects (participant boxes), got {rect_count}"
        );
    }

    #[test]
    fn has_lifelines() {
        let svg = render(&simple_diagram(), &Theme::default());
        // Dashed vertical lines.
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn has_arrow() {
        let svg = render(&simple_diagram(), &Theme::default());
        assert!(svg.contains("<polygon"), "should have arrow head polygon");
    }

    #[test]
    fn has_plantuml_attributes() {
        let svg = render(&simple_diagram(), &Theme::default());
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
