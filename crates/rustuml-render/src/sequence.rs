// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's Java implementation exactly —
//! same element structure, attributes, coordinates, and font metrics.

use std::collections::HashMap;
use std::fmt::Write;

use rustuml_parser::diagram::sequence::*;

use crate::style::Theme;

/// Resolve a PlantUML color string (e.g., "#blue", "#FF0000") to a CSS hex color.
pub(crate) fn resolve_color(color: &str) -> String {
    let name = color.strip_prefix('#').unwrap_or(color);
    // If it's already a hex color (starts with digit or uppercase hex)
    if name.len() == 6 && name.chars().all(|c| c.is_ascii_hexdigit()) {
        return format!("#{}", name.to_uppercase());
    }
    match name.to_lowercase().as_str() {
        "white" => "#FFFFFF".to_string(),
        "black" => "#000000".to_string(),
        "red" => "#FF0000".to_string(),
        "green" => "#008000".to_string(),
        "blue" => "#0000FF".to_string(),
        "yellow" => "#FFFF00".to_string(),
        "orange" => "#FFA500".to_string(),
        "purple" => "#800080".to_string(),
        "pink" => "#FFC0CB".to_string(),
        "cyan" => "#00FFFF".to_string(),
        "magenta" => "#FF00FF".to_string(),
        "gray" | "grey" => "#808080".to_string(),
        "lightblue" => "#ADD8E6".to_string(),
        "lightgreen" => "#90EE90".to_string(),
        "lightyellow" => "#FFFFE0".to_string(),
        "lightcoral" => "#F08080".to_string(),
        "lightcyan" => "#E0FFFF".to_string(),
        "lightpink" => "#FFB6C1".to_string(),
        "darkblue" => "#00008B".to_string(),
        "darkgreen" => "#006400".to_string(),
        "darkred" => "#8B0000".to_string(),
        "coral" => "#FF7F50".to_string(),
        "gold" => "#FFD700".to_string(),
        "salmon" => "#FA8072".to_string(),
        "lightsalmon" => "#FFA07A".to_string(),
        "plum" => "#DDA0DD".to_string(),
        "lime" => "#00FF00".to_string(),
        "navy" => "#000080".to_string(),
        "olive" => "#808000".to_string(),
        "teal" => "#008080".to_string(),
        "aqua" => "#00FFFF".to_string(),
        "fuchsia" => "#FF00FF".to_string(),
        "maroon" => "#800000".to_string(),
        "silver" => "#C0C0C0".to_string(),
        "lightgray" | "lightgrey" => "#D3D3D3".to_string(),
        "darkgray" | "darkgrey" => "#A9A9A9".to_string(),
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

/// Compute text width at a given font size using exact PlantUML Java AWT metrics.
fn text_width(text: &str, font_size: f64) -> f64 {
    crate::metrics::plantuml_text_width(text, font_size)
}

/// Compute bold text width at a given font size using exact PlantUML Java AWT Bold metrics.
fn bold_text_width(text: &str, font_size: f64) -> f64 {
    crate::metrics::plantuml_bold_text_width(text, font_size)
}

/// Format an f64 as a PlantUML-compatible coordinate string.
/// PlantUML uses `String.format(Locale.US, "%.4f", x)` then trims trailing zeros.
/// We must NOT pre-round via multiply/round/divide — that introduces precision errors
/// at boundary values (e.g., 76.023449... * 10000 = 760234.5 exactly, which rounds
/// to 760235 via round-half-away-from-zero, but Java formats the original double to
/// 76.0234 because the 5th decimal of the ACTUAL value is 4).
fn fmt_coord(v: f64) -> String {
    // Format to 4 decimal places (Rust uses round-half-to-even, matching Java's
    // behavior for non-midpoint values, which is the vast majority of cases).
    let s = format!("{v:.4}");

    // Check if the result is an integer (all zeros after decimal point)
    if s.ends_with(".0000") {
        // Return just the integer part
        return s[..s.len() - 5].to_string();
    }

    // Trim trailing zeros after decimal point
    let s = s.trim_end_matches('0');
    // Don't leave a trailing decimal point
    let s = s.trim_end_matches('.');
    s.to_string()
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

// ---------------------------------------------------------------------------
// Group layout constants (reverse-engineered from golden SVGs)
// ---------------------------------------------------------------------------

/// Vertical space consumed by a group start header.
const GROUP_HEADER_HEIGHT: f64 = 17.3106;
/// Vertical space consumed by a group else divider.
const GROUP_ELSE_HEIGHT: f64 = 9.0;
/// Left margin for group frame.
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

/// Emit text content to the SVG, handling creole markup by splitting into
/// separate text elements for each styled segment.
#[allow(dead_code)]
fn emit_text(buf: &mut String, x: f64, y: f64, content: &str, font_size: f64) {
    if has_creole_markup(content) {
        // For creole text, emit the plain text content (stripped of markup).
        // This ensures golden test text-label matching works.
        // The full creole styling (bold, italic, etc.) will be added later.
        let plain = strip_creole(content);
        if !plain.is_empty() {
            let tw = text_width(&plain, font_size);
            let size_str = if font_size == 13.0 { "13" } else { "14" };
            write!(
                buf,
                r##"<text fill="#000000" font-family="sans-serif" font-size="{size_str}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                fmt_coord(tw),
                fmt_coord(x),
                fmt_coord(y),
                escape_xml(&plain),
            )
            .unwrap();
        }
    } else {
        let tw = text_width(content, font_size);
        let size_str = if font_size == 13.0 {
            "13"
        } else if font_size == 11.0 {
            "11"
        } else {
            "14"
        };
        write!(
            buf,
            r##"<text fill="#000000" font-family="sans-serif" font-size="{size_str}" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            fmt_coord(tw),
            fmt_coord(x),
            fmt_coord(y),
            escape_xml(content),
        )
        .unwrap();
    }
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
    /// Optional stereotype text
    stereotype: Option<String>,
    /// Text width at font-size 14
    text_width: f64,
    /// Stereotype display text width at font-size 11 (if any)
    stereotype_width: f64,
    /// Box width = text_width + 2 * BOX_TEXT_X_PAD
    box_width: f64,
    /// Box height (taller for stereotyped participants)
    box_height: f64,
    /// Left x of participant box
    box_x: f64,
    /// Center x (exact box center, used for messages and lifeline rect)
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
}

impl PlantUmlSvg {
    fn new() -> Self {
        Self {
            buf: String::with_capacity(4096),
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
            r##"<line style="stroke:#181818;stroke-width:0.5;stroke-dasharray:5,5;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
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
            r##"<rect fill="#E2E2F0" height="{}" rx="{}" ry="{}" style="stroke:#181818;stroke-width:0.5;" width="{}" x="{}" y="{}"/>"##,
            fmt_coord(rect_h),
            fmt_coord(HEAD_BOX_RX),
            fmt_coord(HEAD_BOX_RX),
            fmt_coord(rect_w),
            fmt_coord(rect_x),
            fmt_coord(rect_y),
        )
        .unwrap();

        // Stereotype text (above participant name, smaller font)
        if let Some((st_text, st_width)) = stereotype {
            let st_display = format!("\u{ab}{st_text}\u{bb}");
            let st_y = text_y - 13.0; // stereotype is above the name
            write!(
                self.buf,
                r##"<text fill="#000000" font-family="sans-serif" font-size="11" font-style="italic" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                fmt_coord(st_width),
                fmt_coord(text_x),
                fmt_coord(st_y),
                escape_xml(&st_display),
            )
            .unwrap();
        }

        write!(
            self.buf,
            r##"<text fill="#000000" font-family="sans-serif" font-size="14" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            fmt_coord(text_len),
            fmt_coord(text_x),
            fmt_coord(text_y),
            escape_xml(text_content),
        )
        .unwrap();

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
        autonumber: Option<(&str, f64)>, // (number_text, number_width)
    ) {
        write!(
            self.buf,
            r##"<g class="message" data-entity-1="{entity1}" data-entity-2="{entity2}" data-source-line="{source_line}" id="msg{msg_id}">"##,
            entity1 = escape_xml(entity1),
            entity2 = escape_xml(entity2),
        )
        .unwrap();

        write!(
            self.buf,
            r##"<polygon fill="{color}" points="{arrow_points}" style="stroke:{color};stroke-width:1;"/>"##,
        )
        .unwrap();

        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:1;{line_style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(line_x1),
            fmt_coord(line_x2),
            fmt_coord(line_y),
            fmt_coord(line_y),
        )
        .unwrap();

        let label_x = if let Some((num_text, num_w)) = autonumber {
            // Bold autonumber text
            write!(
                self.buf,
                r##"<text fill="#000000" font-family="sans-serif" font-size="13" font-weight="700" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                fmt_coord(num_w),
                fmt_coord(text_x),
                fmt_coord(text_y),
                escape_xml(num_text),
            )
            .unwrap();
            text_x + num_w + AUTONUMBER_LABEL_GAP
        } else {
            text_x
        };

        if !text_content.is_empty() {
            write!(
                self.buf,
                r##"<text fill="#000000" font-family="sans-serif" font-size="13" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                fmt_coord(text_len),
                fmt_coord(label_x),
                fmt_coord(text_y),
                escape_xml(text_content),
            )
            .unwrap();
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
        autonumber: Option<(&str, f64)>, // (number_text, number_width)
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

        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(tip_x),
            fmt_coord(back_x),
            fmt_coord(tip_y),
            fmt_coord(up_y),
        )
        .unwrap();

        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(tip_x),
            fmt_coord(back_x),
            fmt_coord(tip_y),
            fmt_coord(down_y),
        )
        .unwrap();

        write!(
            self.buf,
            r##"<line style="stroke:{color};stroke-width:1;{line_style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            fmt_coord(line_x1),
            fmt_coord(line_x2),
            fmt_coord(line_y),
            fmt_coord(line_y),
        )
        .unwrap();

        let label_x = if let Some((num_text, num_w)) = autonumber {
            // Bold autonumber text
            write!(
                self.buf,
                r##"<text fill="#000000" font-family="sans-serif" font-size="13" font-weight="700" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                fmt_coord(num_w),
                fmt_coord(text_x),
                fmt_coord(text_y),
                escape_xml(num_text),
            )
            .unwrap();
            text_x + num_w + AUTONUMBER_LABEL_GAP
        } else {
            text_x
        };

        if !text_content.is_empty() {
            write!(
                self.buf,
                r##"<text fill="#000000" font-family="sans-serif" font-size="13" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                fmt_coord(text_len),
                fmt_coord(label_x),
                fmt_coord(text_y),
                escape_xml(text_content),
            )
            .unwrap();
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

    let lines: &[&str] = &[
        "Welcome to PlantUML!",
        "\u{00a0}",
        "You can start with a simple UML Diagram like:",
        "\u{00a0}",
        "Bob->Alice:\u{00a0}Hello",
        "\u{00a0}",
        "Or",
        "\u{00a0}",
        "class\u{00a0}Example",
        "\u{00a0}",
        "You will find more information about PlantUML syntax on",
        "https://plantuml.com",
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

/// Render a sequence diagram to SVG matching PlantUML's exact output.
pub fn render(diagram: &SequenceDiagram, _theme: &Theme) -> String {
    // Empty diagram with no title — render the PlantUML welcome screen.
    if diagram.participants.is_empty() && diagram.events.is_empty() && diagram.meta.title.is_none()
    {
        return render_empty_welcome();
    }

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
            let bw = max_text_w + 2.0 * BOX_TEXT_X_PAD;
            // Box height is taller for stereotyped participants
            let bh = if st.is_some() {
                HEAD_BOX_H + 15.0 // extra space for stereotype line
            } else {
                HEAD_BOX_H
            };
            ParticipantLayout {
                idx,
                id: p.id.clone(),
                label,
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
                    let label = process_label(&msg.label);
                    let label_w = text_width(&label, MSG_FONT_SIZE);

                    // Autonumber adds bold text + gap before the label
                    let autonumber_extra = if let Some(num) = spacing_auto_num.as_ref() {
                        let an = diagram.autonumber.as_ref().unwrap();
                        let num_text = format_autonumber(*num, &an.format);
                        bold_text_width(&num_text, MSG_FONT_SIZE) + AUTONUMBER_LABEL_GAP
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
                        let per_pair = needed / (right - left) as f64;
                        for slot in &mut pair_max_label_width[left..right] {
                            *slot = slot.max(per_pair);
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

                        // Autonumber adds bold text + gap before the label
                        let autonumber_extra = if let Some(num) = spacing_auto_num.as_ref() {
                            let an = diagram.autonumber.as_ref().unwrap();
                            let num_text = format_autonumber(*num, &an.format);
                            bold_text_width(&num_text, MSG_FONT_SIZE) + AUTONUMBER_LABEL_GAP
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
                            let per_pair = needed / (right - left) as f64;
                            for slot in &mut pair_max_label_width[left..right] {
                                *slot = slot.max(per_pair);
                            }
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
            Event::Note(note) => {
                // Notes spanning multiple participants need gap between them.
                if note.position == NotePosition::Over && note.participants.len() >= 2 {
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
                        // Note needs: text_width + 20 (padding + fold) + 38 (margins around lifelines)
                        // But this is the total span across all pairs, so divide evenly.
                        let note_w = max_tw.ceil() + 20.0;
                        let span_pairs = (li - fi) as f64;
                        let per_pair = (note_w + 38.0) / span_pairs;
                        let (left, right) = if fi < li { (fi, li) } else { (li, fi) };
                        for slot in &mut pair_max_label_width[left..right] {
                            *slot = slot.max(per_pair);
                        }
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
    // A 'note over' on the first participant starts at x=5 (HEAD_BOX_Y) and extends
    // rightward. The participant center must be far enough right that the note
    // width, centered on the participant, doesn't extend past the left margin.
    // PlantUML uses: center_x = max(default, note_right_edge - box_width/2)
    // where note_right_edge = HEAD_BOX_Y + note_w.
    let mut min_first_center_x: f64 = 0.0;
    for event in &diagram.events {
        if let Event::Note(note) = event
            && note.position == NotePosition::Over
            && note.participants.len() == 1
            && let Some(&idx) = id_to_idx.get(note.participants[0].as_str())
            && idx == 0
        {
            let max_tw = note
                .text
                .lines()
                .map(|l| text_width(l.trim(), MSG_FONT_SIZE))
                .fold(0.0_f64, f64::max);
            let note_w = max_tw.ceil() + 20.0;
            // The note is positioned centered on the participant. For the left edge
            // to be at HEAD_BOX_Y, the center must be at HEAD_BOX_Y + note_w/2.
            // But PlantUML uses a different formula — looking at golden data:
            // Alice center = 58.3618, note goes from 5 to 113 (width 108).
            // 58.3618 - 5 = 53.3618 (left extent from center)
            // 113 - 58.3618 = 54.6382 (right extent from center)
            // note_w/2 = 54. So it's almost centered but not exactly.
            // The center is at box_x + box_width/2 where box_x = center_x - box_width/2.
            // Alice box_width = 46.7236. center = 5 + 46.7236/2 = 28.3618.
            // But golden center is 58.3618 = 28.3618 + 30.
            // 30 = (note_w - box_width) / 2 = (108 - 46.7236) / 2 = 30.6382. Not exactly 30.
            // Actually: golden box_x = 35. So shift from default (5) = 30.
            // (note_w - box_width) / 2 = (108 - 46.7236) / 2 = 30.6382.
            // Round down: floor(30.6382) = 30. So box_x = 5 + 30 = 35. center = 35 + 23.3618 = 58.3618.
            let shift = ((note_w - participants[0].box_width) / 2.0).floor();
            let min_cx = HEAD_BOX_Y + shift.max(0.0) + participants[0].box_width / 2.0;
            min_first_center_x = min_first_center_x.max(min_cx);
        }
    }

    // -----------------------------------------------------------------------
    // Phase 3: Assign x positions
    // -----------------------------------------------------------------------

    let mut participants = participants;
    if !participants.is_empty() {
        // First participant center must be at least min_first_center_x (for notes)
        // and at least HEAD_BOX_Y + box_width/2 (to fit the box).
        let default_center = HEAD_BOX_Y + participants[0].box_width / 2.0;
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

    // -----------------------------------------------------------------------
    // Phase 4: Pre-compute y positions for each event and vertical dimensions
    // -----------------------------------------------------------------------

    // Use the maximum box height across all participants
    let max_box_h = participants
        .iter()
        .map(|p| p.box_height)
        .fold(HEAD_BOX_H, f64::max);
    let lifeline_top = HEAD_BOX_Y + max_box_h + LIFELINE_Y_OFFSET;

    // Pre-compute message y positions. PlantUML sizes each message step
    // dynamically: messages with label text get extra height for the text line.
    // Notes consume vertical space (note height + gap) and count as events
    // for msg_count (so subsequent messages use msg_step, not first_msg_offset).
    // Groups add header/else/end vertical space.
    let mut event_y_positions: Vec<f64> = Vec::new();
    let mut msg_count: u32 = 0;
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
                Event::Message(_) | Event::Return(_) => {
                    if msg_count == 0 {
                        y += first_msg_offset(has_text);
                    } else {
                        y += msg_step(has_text);
                    }
                    event_y_positions.push(y);
                    msg_count += 1;
                }
                Event::Divider(_) => {
                    // Dividers take extra vertical space for the double-line separator.
                    // PlantUML adds msg_step(true) + MSG_BASE_STEP for the visual.
                    if msg_count == 0 {
                        y += first_msg_offset(has_text) + MSG_BASE_STEP;
                    } else {
                        y += msg_step(has_text) + MSG_BASE_STEP;
                    }
                    event_y_positions.push(y);
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
                    // The event y for a note is: note_top + 7.0 + num_lines * MSG_TEXT_HEIGHT
                    // This ensures the next message at y + msg_step lands correctly.
                    let note_event_y = note_top + 7.0 + num_lines as f64 * MSG_TEXT_HEIGHT;
                    y = note_event_y;
                    event_y_positions.push(y);
                    msg_count += 1; // note counts as an event for spacing
                }
                Event::GroupStart(_) => {
                    // Group start adds vertical space for the header.
                    if msg_count == 0 {
                        y += NOTE_GAP_FIRST;
                    } else {
                        y += GROUP_HEADER_HEIGHT;
                    }
                    event_y_positions.push(y);
                    // Don't increment msg_count — the group header itself isn't a message
                }
                Event::GroupElse(_) => {
                    // Else divider adds vertical space.
                    y += GROUP_ELSE_HEIGHT;
                    event_y_positions.push(y);
                }
                Event::GroupEnd => {
                    // Group end adds a small gap.
                    y += 5.0;
                    event_y_positions.push(y);
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
    }

    // Compute tail box y based on message count.
    // With 0 messages, PlantUML uses a minimum lifeline height of 20px.
    // With messages, the tail starts TAIL_GAP below the last message.
    let last_msg_y = event_y_positions
        .iter()
        .copied()
        .last()
        .unwrap_or(lifeline_top);
    let tail_box_y = if msg_count > 0 {
        last_msg_y + TAIL_GAP
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
            let note_content_w = max_line_width.ceil() + 20.0; // 6 + text + 4 + 10(fold)
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
                        // "across" note — spans full width, handled separately
                    } else if note.participants.len() == 1 {
                        if let Some(&idx) = id_to_idx.get(note.participants[0].as_str()) {
                            let cx = participants[idx].center_x;
                            let note_right = cx + note_content_w / 2.0;
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
    let effective_right = last_box_right.max(if max_note_right > 0.0 {
        max_note_right + 1.0
    } else {
        0.0
    });
    // If groups are present, the group frame extends beyond the last participant.
    // Add extra margin for the group frame (margin + activation bars + padding).
    let has_groups = diagram
        .events
        .iter()
        .any(|e| matches!(e, Event::GroupStart(_)));
    let group_frame_margin = if has_groups {
        GROUP_FRAME_MARGIN + ACTIVATION_WIDTH + 4.0 // frame margin + activation bar + padding
    } else {
        0.0
    };
    let svg_width = (effective_right + RIGHT_MARGIN + group_frame_margin).ceil() as u32;
    let svg_height = if diagram.hide_footbox {
        lifeline_bottom.ceil() as u32
    } else {
        (tail_box_y + max_box_h + BOTTOM_MARGIN).ceil() as u32
    };

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
    }

    let mut activation_bars: Vec<ActivationBar> = Vec::new();
    {
        let mut tracker = ActivationTracker::new();
        // Track open activations: (participant_id, event_idx, color)
        let mut open_activations: Vec<(String, usize, Option<String>)> = Vec::new();
        let mut last_event_idx: usize = 0;

        for (ev_idx, event) in diagram.events.iter().enumerate() {
            match event {
                Event::Message(msg) => {
                    last_event_idx = ev_idx;

                    // Process activation changes from ++ / -- on message
                    if let Some(act) = &msg.activation {
                        match act {
                            ActivationChange::Activate => {
                                tracker.activate(&msg.to);
                                open_activations.push((
                                    msg.to.clone(),
                                    ev_idx,
                                    msg.activation_color.clone(),
                                ));
                            }
                            ActivationChange::Deactivate => {
                                tracker.deactivate(&msg.from);
                                if let Some(pos) = open_activations
                                    .iter()
                                    .rposition(|(id, _, _)| id == &msg.from)
                                {
                                    let (pid, start_idx, color) = open_activations.remove(pos);
                                    activation_bars.push(ActivationBar {
                                        participant_id: pid,
                                        start_event_idx: start_idx,
                                        end_event_idx: ev_idx,
                                        color,
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
                    tracker.activate(id);
                    open_activations.push((id.clone(), last_event_idx, color.clone()));
                }
                Event::Deactivate(id) => {
                    tracker.deactivate(id);
                    if let Some(pos) = open_activations.iter().rposition(|(pid, _, _)| pid == id) {
                        let (pid, start_idx, color) = open_activations.remove(pos);
                        activation_bars.push(ActivationBar {
                            participant_id: pid,
                            start_event_idx: start_idx,
                            end_event_idx: last_event_idx,
                            color,
                        });
                    }
                }
                Event::Return(_) => {
                    last_event_idx = ev_idx;
                    // Return deactivates the most recently activated participant
                    if let Some(pos) = open_activations.len().checked_sub(1) {
                        let (pid, start_idx, color) = open_activations.remove(pos);
                        tracker.deactivate(&pid);
                        activation_bars.push(ActivationBar {
                            participant_id: pid,
                            start_event_idx: start_idx,
                            end_event_idx: ev_idx,
                            color,
                        });
                    }
                }
                _ => {}
            }
        }

        // Close any remaining open activations — extend to the last event
        let final_idx = diagram.events.len().saturating_sub(1);
        for (pid, start_idx, color) in open_activations {
            activation_bars.push(ActivationBar {
                participant_id: pid,
                start_event_idx: start_idx,
                end_event_idx: final_idx,
                color,
            });
        }

        // Sort by start event index to match PlantUML's rendering order
        activation_bars.sort_by_key(|b| b.start_event_idx);
    }

    // Helper to look up y position for an event
    let event_y =
        |idx: usize| -> f64 { event_y_positions.get(idx).copied().unwrap_or(lifeline_top) };

    // -----------------------------------------------------------------------
    // Phase 6: Generate SVG
    // -----------------------------------------------------------------------

    let mut svg = PlantUmlSvg::new();
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
        let tw = text_width(&msg, 11.0);
        let mid_x = svg_width as f64 / 2.0;
        write!(
            svg.buf,
            r##"<text fill="#000000" font-family="sans-serif" font-size="11" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            fmt_coord(tw),
            fmt_coord(mid_x),
            fmt_coord(HEAD_BOX_Y + 13.0),
            escape_xml(&msg),
        )
        .unwrap();
    }

    // Render title if present
    if let Some(title) = &diagram.meta.title {
        let tw = text_width(title, 15.0);
        let mid_x = svg_width as f64 / 2.0;
        write!(
            svg.buf,
            r##"<text fill="#000000" font-family="sans-serif" font-size="15" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            fmt_coord(tw),
            fmt_coord(mid_x),
            fmt_coord(HEAD_BOX_Y + 13.0),
            escape_xml(title),
        )
        .unwrap();
    }

    // Render header if present
    if let Some(header) = &diagram.meta.header {
        let tw = text_width(header, 11.0);
        let mid_x = svg_width as f64 / 2.0;
        write!(
            svg.buf,
            r##"<text fill="#000000" font-family="sans-serif" font-size="11" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            fmt_coord(tw),
            fmt_coord(mid_x),
            fmt_coord(HEAD_BOX_Y * 0.8),
            escape_xml(header),
        )
        .unwrap();
    }

    // Render footer if present
    if let Some(footer) = &diagram.meta.footer {
        let tw = text_width(footer, 11.0);
        let mid_x = svg_width as f64 / 2.0;
        write!(
            svg.buf,
            r##"<text fill="#000000" font-family="sans-serif" font-size="11" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            fmt_coord(tw),
            fmt_coord(mid_x),
            fmt_coord(svg_height as f64 - 4.0),
            escape_xml(footer),
        )
        .unwrap();
    }

    // Render caption if present
    if let Some(caption) = &diagram.meta.caption {
        let tw = text_width(caption, 11.0);
        let mid_x = svg_width as f64 / 2.0;
        write!(
            svg.buf,
            r##"<text fill="#000000" font-family="sans-serif" font-size="11" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            fmt_coord(tw),
            fmt_coord(mid_x),
            fmt_coord(svg_height as f64 - 4.0),
            escape_xml(caption),
        )
        .unwrap();
    }

    // Render legend if present
    if let Some(legend) = &diagram.meta.legend {
        // Emit each non-empty line of the legend as a text element
        let lx = svg_width as f64 - 200.0;
        let mut ly = svg_height as f64 - 150.0;
        for line in legend.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let stripped = if trimmed.contains('|') {
                    // Table cells: extract cell text
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
                    strip_creole(trimmed)
                };
                if !stripped.is_empty() {
                    let tw = text_width(&stripped, 11.0);
                    write!(
                        svg.buf,
                        r##"<text fill="#000000" font-family="sans-serif" font-size="11" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                        fmt_coord(tw),
                        fmt_coord(lx),
                        fmt_coord(ly),
                        escape_xml(&stripped),
                    )
                    .unwrap();
                }
            }
            ly += 14.0;
        }
    }

    // Activation bars are rendered BEFORE lifelines in PlantUML's SVG.
    // Actually, looking at golden SVGs: activation bars appear first (before lifelines),
    // then lifelines, then participant heads, then tails, then activation bars AGAIN,
    // then messages.

    // First pass: activation bars (rendered twice in PlantUML's SVG)
    for bar in &activation_bars {
        let cx = center_of(&bar.participant_id);
        let bar_x = cx - ACTIVATION_HALF_W;
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

    // Participant head and tail boxes (interleaved per participant, matching PlantUML order)
    for (i, p) in participants.iter().enumerate() {
        let part_uid = format!("part{}", p.idx + 1);
        let text_x = p.box_x + BOX_TEXT_X_PAD;

        // Head box
        let head_text_y =
            HEAD_BOX_Y + BOX_TEXT_Y_OFFSET + if p.stereotype.is_some() { 7.5 } else { 0.0 };
        let stereo_arg = p.stereotype.as_ref().map(|s| {
            let display = format!("\u{ab}{s}\u{bb}");
            (display, p.stereotype_width)
        });
        let stereo_ref = stereo_arg.as_ref().map(|(s, w)| (s.as_str(), *w));
        svg.participant_box(
            &part_uid,
            &p.id,
            source_line_for(&diagram.participants[i].id),
            "head",
            p.box_x,
            HEAD_BOX_Y,
            p.box_width,
            p.box_height,
            text_x,
            head_text_y,
            &p.label,
            p.text_width,
            stereo_ref,
        );

        // Tail box (skip if hide footbox)
        if !diagram.hide_footbox {
            let tail_text_y =
                tail_box_y + BOX_TEXT_Y_OFFSET + if p.stereotype.is_some() { 7.5 } else { 0.0 };
            let stereo_arg = p.stereotype.as_ref().map(|s| {
                let display = format!("\u{ab}{s}\u{bb}");
                (display, p.stereotype_width)
            });
            let stereo_ref = stereo_arg.as_ref().map(|(s, w)| (s.as_str(), *w));
            svg.participant_box(
                &part_uid,
                &p.id,
                source_line_for(&diagram.participants[i].id),
                "tail",
                p.box_x,
                tail_box_y,
                p.box_width,
                p.box_height,
                text_x,
                tail_text_y,
                &p.label,
                p.text_width,
                stereo_ref,
            );
        }
    }

    // Second pass: activation bars again (PlantUML renders them twice)
    for bar in &activation_bars {
        let cx = center_of(&bar.participant_id);
        let bar_x = cx - ACTIVATION_HALF_W;
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
                let is_right = to_x > from_x;
                let is_dotted = msg.arrow.line == LineStyle::Dotted;
                let is_open = msg.arrow.head == ArrowHead::Open;

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

                // Arrow color (default #181818)
                let arrow_color = msg
                    .arrow
                    .color
                    .as_ref()
                    .map(|c| resolve_color(c))
                    .unwrap_or_else(|| "#181818".to_string());

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
                let autonumber_info: Option<(String, f64)> = auto_num.as_ref().map(|n| {
                    let an = diagram.autonumber.as_ref().unwrap();
                    let num_text = format_autonumber(*n, &an.format);
                    let num_w = bold_text_width(&num_text, MSG_FONT_SIZE);
                    (num_text, num_w)
                });
                let autonumber_ref = autonumber_info.as_ref().map(|(t, w)| (t.as_str(), *w));

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
                let text_y_pos = msg_y - 4.7422;
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

                    if is_open {
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

                    if is_open {
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
                let ret_autonumber_info: Option<(String, f64)> = auto_num.as_ref().map(|n| {
                    let an = diagram.autonumber.as_ref().unwrap();
                    let num_text = format_autonumber(*n, &an.format);
                    let num_w = bold_text_width(&num_text, MSG_FONT_SIZE);
                    (num_text, num_w)
                });
                let ret_autonumber_ref =
                    ret_autonumber_info.as_ref().map(|(t, w)| (t.as_str(), *w));

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
                let text_y_pos = msg_y - 4.7422;

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
                // Emit divider text
                let tw = text_width(text, MSG_FONT_SIZE);
                let mid_x = if !participants.is_empty() {
                    (participants[0].center_x + participants[participants.len() - 1].center_x) / 2.0
                } else {
                    50.0
                };
                write!(
                    svg.buf,
                    r##"<text fill="#000000" font-family="sans-serif" font-size="13" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                    fmt_coord(tw),
                    fmt_coord(mid_x),
                    fmt_coord(msg_y - 3.0),
                    escape_xml(text),
                )
                .unwrap();
            }
            Event::Delay(Some(t)) => {
                let tw = text_width(t, MSG_FONT_SIZE);
                let mid_x = if !participants.is_empty() {
                    (participants[0].center_x + participants[participants.len() - 1].center_x) / 2.0
                } else {
                    50.0
                };
                write!(
                    svg.buf,
                    r##"<text fill="#000000" font-family="sans-serif" font-size="13" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                    fmt_coord(tw),
                    fmt_coord(mid_x),
                    fmt_coord(msg_y + 5.0),
                    escape_xml(t),
                )
                .unwrap();
            }
            Event::Delay(None) => {}
            Event::Space(_px_opt) => {
                // y position already accounted for in event_y_positions
            }
            Event::Note(note) => {
                // Compute note dimensions and position.
                let lines: Vec<&str> = note.text.lines().collect();
                let num_lines = lines.len().max(1);
                let note_height = NOTE_BASE_HEIGHT + (num_lines as f64 - 1.0) * NOTE_LINE_HEIGHT;

                // Derive note_top from the event y:
                // event_y = note_top + 7.0 + num_lines * MSG_TEXT_HEIGHT
                let note_top = msg_y - 7.0 - num_lines as f64 * MSG_TEXT_HEIGHT;
                let note_bottom = note_top + note_height;

                // Compute max text width across all lines.
                let max_text_w = lines
                    .iter()
                    .map(|l| text_width(l.trim(), MSG_FONT_SIZE))
                    .fold(0.0_f64, f64::max);
                let note_content_w = max_text_w.ceil() + 20.0; // 6(pad) + text + 4(pad) + 10(fold)

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
                            // "across" note — spans full diagram width
                            (HEAD_BOX_Y, svg_width as f64 - RIGHT_MARGIN - 1.0)
                        } else if note.participants.len() == 1 {
                            // Centered on single participant
                            let cx = note
                                .participants
                                .first()
                                .and_then(|id| id_to_idx.get(id.as_str()))
                                .map(|&i| participants[i].center_x)
                                .unwrap_or(50.0);
                            let half_w = note_content_w / 2.0;
                            let left = (cx - half_w).max(HEAD_BOX_Y);
                            (left, left + note_content_w)
                        } else {
                            // Spanning multiple participants
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
                            let note_w = note_content_w.max(span + 38.0);
                            let mid = (first_ll + last_ll) / 2.0;
                            let left = (mid - note_w / 2.0).max(HEAD_BOX_Y);
                            (left, left + note_w)
                        }
                    }
                };

                // Emit the note polygon (body with folded corner).
                let fold_x = note_right - NOTE_FOLD_SIZE;
                let fold_y = note_top + NOTE_FOLD_SIZE;
                write!(
                    svg.buf,
                    r##"<path d="M{left},{top} L{left},{bottom} L{right},{bottom} L{right},{fold_y} L{fold_x},{top} L{left},{top}" fill="{NOTE_FILL}" style="stroke:#181818;stroke-width:0.5;"/>"##,
                    left = fmt_coord(note_left),
                    top = fmt_coord(note_top),
                    bottom = fmt_coord(note_bottom),
                    right = fmt_coord(note_right),
                    fold_y = fmt_coord(fold_y),
                    fold_x = fmt_coord(fold_x),
                )
                .unwrap();

                // Emit the fold triangle.
                write!(
                    svg.buf,
                    r##"<path d="M{fold_x},{top} L{fold_x},{fold_y} L{right},{fold_y} L{fold_x},{top}" fill="{NOTE_FILL}" style="stroke:#181818;stroke-width:0.5;"/>"##,
                    fold_x = fmt_coord(fold_x),
                    top = fmt_coord(note_top),
                    fold_y = fmt_coord(fold_y),
                    right = fmt_coord(note_right),
                )
                .unwrap();

                // Emit note text lines.
                let text_x = note_left + NOTE_TEXT_X_PAD;
                let mut text_y = note_top + NOTE_TEXT_Y_OFFSET;
                for line in &lines {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        text_y += NOTE_TEXT_LINE_SPACING;
                        continue;
                    }
                    let tw = text_width(trimmed, MSG_FONT_SIZE);
                    write!(
                        svg.buf,
                        r##"<text fill="#000000" font-family="sans-serif" font-size="13" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                        fmt_coord(tw),
                        fmt_coord(text_x),
                        fmt_coord(text_y),
                        escape_xml(trimmed),
                    )
                    .unwrap();
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

                // Compute the group frame extent. We need to find the matching
                // GroupEnd to know the full height. For now, emit the header
                // elements and the frame will be approximate.
                let frame_left = GROUP_FRAME_MARGIN;
                let frame_right = svg_width as f64 - GROUP_FRAME_MARGIN;

                // Find matching group end to compute frame height
                let mut depth = 1u32;
                let mut frame_bottom = msg_y + 50.0; // fallback
                for future_idx in (ev_idx + 1)..events.len() {
                    match &events[future_idx] {
                        Event::GroupStart(_) => depth += 1,
                        Event::GroupEnd => {
                            depth -= 1;
                            if depth == 0 {
                                frame_bottom = event_y_positions[future_idx];
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                let frame_top = msg_y;
                let frame_height = frame_bottom - frame_top;

                // Emit group frame rect (rendered twice in PlantUML)
                for _ in 0..2 {
                    write!(
                        svg.buf,
                        r##"<rect fill="none" height="{}" style="stroke:#000000;stroke-width:1.5;" width="{}" x="{}" y="{}"/>"##,
                        fmt_coord(frame_height),
                        fmt_coord(frame_right - frame_left),
                        fmt_coord(frame_left),
                        fmt_coord(frame_top),
                    )
                    .unwrap();
                }

                // Emit header tab (pentagon shape)
                let kind_w = text_width(kind_str, MSG_FONT_SIZE);
                let tab_right = frame_left + kind_w + 30.0;
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

                // Emit kind text (bold)
                let kind_tw = text_width(kind_str, MSG_FONT_SIZE);
                write!(
                    svg.buf,
                    r##"<text fill="#000000" font-family="sans-serif" font-size="13" font-weight="700" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                    fmt_coord(kind_tw),
                    fmt_coord(frame_left + 15.0),
                    fmt_coord(frame_top + 13.5684),
                    escape_xml(kind_str),
                )
                .unwrap();

                // Emit guard label if present (in brackets)
                if let Some(label) = &g.label {
                    let guard = format!("[{label}]");
                    let guard_w = text_width(&guard, 11.0);
                    write!(
                        svg.buf,
                        r##"<text fill="#000000" font-family="sans-serif" font-size="11" font-weight="700" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                        fmt_coord(guard_w),
                        fmt_coord(tab_right + 15.0),
                        fmt_coord(frame_top + 12.6348),
                        escape_xml(&guard),
                    )
                    .unwrap();
                }
            }
            Event::GroupElse(g) => {
                // Emit else dashed divider line
                let frame_left = GROUP_FRAME_MARGIN;
                let frame_right = svg_width as f64 - GROUP_FRAME_MARGIN;
                write!(
                    svg.buf,
                    r##"<line style="stroke:#000000;stroke-width:1;stroke-dasharray:2,2;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
                    fmt_coord(frame_left),
                    fmt_coord(frame_right),
                    fmt_coord(msg_y),
                    fmt_coord(msg_y),
                )
                .unwrap();

                // Emit else label (in brackets if present)
                let label_text = if let Some(label) = &g.label {
                    format!("[{label}]")
                } else {
                    "[else]".to_string()
                };
                let tw = text_width(&label_text, 11.0);
                write!(
                    svg.buf,
                    r##"<text fill="#000000" font-family="sans-serif" font-size="11" font-weight="700" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                    fmt_coord(tw),
                    fmt_coord(frame_left + 5.0),
                    fmt_coord(msg_y + 10.6348),
                    escape_xml(&label_text),
                )
                .unwrap();
            }
            Event::GroupEnd => {
                // Group end is handled by the frame rect emitted at GroupStart
            }
            Event::NoteOnLink(text) => {
                let tw = text_width(text, 13.0);
                let mid_x = if !participants.is_empty() {
                    (participants[0].center_x + participants[participants.len() - 1].center_x) / 2.0
                } else {
                    50.0
                };
                write!(
                    svg.buf,
                    r##"<text fill="#000000" font-family="sans-serif" font-size="13" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                    fmt_coord(tw),
                    fmt_coord(mid_x),
                    fmt_coord(msg_y + 2.0),
                    escape_xml(text),
                )
                .unwrap();
            }
            Event::Ref(r) => {
                let tw = text_width(&r.text, 13.0);
                let mid_x = if !participants.is_empty() {
                    (participants[0].center_x + participants[participants.len() - 1].center_x) / 2.0
                } else {
                    50.0
                };
                write!(
                    svg.buf,
                    r##"<text fill="#000000" font-family="sans-serif" font-size="13" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                    fmt_coord(tw),
                    fmt_coord(mid_x),
                    fmt_coord(msg_y + 4.0),
                    escape_xml(&r.text),
                )
                .unwrap();
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
            _ => {
                // Remaining events (Destroy, Create, NewPage)
                // don't emit visible text labels or change activation state.
            }
        }
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
                    source_line: 1,
                },
                Participant {
                    id: "Bob".into(),
                    label: "Bob".into(),
                    kind: ParticipantKind::Participant,
                    order: Some(1),
                    stereotype: None,
                    url: None,
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
