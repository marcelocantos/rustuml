// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram SVG renderer.

use rustuml_parser::diagram::sequence::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

const MIN_PARTICIPANT_WIDTH: f64 = 60.0;
const PARTICIPANT_HEIGHT: f64 = 35.0;
const PARTICIPANT_GAP: f64 = 50.0;
const PARTICIPANT_PADDING: f64 = 16.0;
const MESSAGE_HEIGHT: f64 = 40.0;
const TOP_MARGIN: f64 = 20.0;
const LEFT_MARGIN: f64 = 20.0;
const FONT_SIZE: f64 = 13.0;
const SMALL_FONT: f64 = 11.0;

/// Format an autonumber counter according to an optional format string.
///
/// Format examples: `None` → `"42"`, `Some("[000]")` → `"[042]"`,
/// `Some("(0)")` → `"(42)"`, `Some("<b>Step 0:</b>")` → `"Step 42:"`.
fn format_autonumber(n: u32, format: &Option<String>) -> String {
    let Some(fmt) = format else {
        return n.to_string();
    };

    // Strip HTML/XML markup tags (anything between < and >).
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

    // Find the first run of '0's and replace it with a zero-padded number.
    if let Some(start) = plain.find('0') {
        let end = plain[start..]
            .find(|c| c != '0')
            .map(|i| start + i)
            .unwrap_or(plain.len());
        let width = end - start;
        format!("{}{:0>width$}{}", &plain[..start], n, &plain[end..])
    } else if let Some(start) = plain.find('#') {
        // Find the end of the '#' run.
        let end = plain[start..]
            .find(|c: char| c != '#')
            .map(|i| start + i)
            .unwrap_or(plain.len());
        // Replace the entire '#' run with the plain number (no zero-padding).
        format!("{}{}{}", &plain[..start], n, &plain[end..])
    } else {
        format!("{plain}{n}")
    }
}

/// Decode PlantUML backslash escapes in label text.
/// `\\` → `\`, `\n` → newline (for display purposes we keep `\n` as space).
fn decode_escapes(s: &str) -> String {
    s.replace("\\\\", "\\")
}

/// Process label text for SVG rendering: decode escapes and replace unsupported
/// markup like `<img:...>` with a placeholder matching PlantUML's behavior.
fn process_label(s: &str) -> String {
    // PlantUML uses non-breaking space (U+00A0) in the cannot-decode message.
    let decoded = decode_escapes(s);
    // Replace <img:...> tags with PlantUML's placeholder.
    // For HTTP/HTTPS URLs, include the URL in the fallback message.
    let mut result = String::with_capacity(decoded.len());
    let mut rest = decoded.as_str();
    while let Some(start) = rest.find("<img:") {
        result.push_str(&rest[..start]);
        let after = &rest[start..];
        if let Some(end) = after.find('>') {
            let raw_src = &after["<img:".len()..end];
            // Strip {scale=...} or similar suffix from the URL.
            let src = if let Some(brace) = raw_src.find('{') { &raw_src[..brace] } else { raw_src };
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

/// Returns the display label for a participant box.
///
/// If the participant has a stereotype, it appears inline as "Name «stereotype»",
/// matching PlantUML's SVG output.
fn participant_display_label(p: &Participant) -> String {
    if let Some(st) = &p.stereotype {
        format!("{} \u{ab}{st}\u{bb}", p.label)
    } else {
        // PlantUML renders `(text)` in participant labels as `.text.` (period-delimited
        // stereotype notation).  Convert all `(...)` groups to `.....` form so that
        // our text output matches the PlantUML golden SVG text elements.
        let label = parentheses_to_dots(&p.label);
        label
    }
}

/// Convert `(text)` patterns inside a string to `.text.` — PlantUML's participant
/// stereotype dot notation.
fn parentheses_to_dots(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '(' {
            // Find the matching closing paren.
            if let Some(close) = chars[i + 1..].iter().position(|&c| c == ')') {
                let inner: String = chars[i + 1..i + 1 + close].iter().collect();
                result.push('.');
                result.push_str(&inner);
                result.push('.');
                i += 1 + close + 1; // skip past ')'
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Render multi-line note text as individual SVG text elements.
///
/// Handles `<code>` blocks (monospace with non-breaking spaces) and
/// `# item` numbered lists. Returns the total height used.
fn render_note_text(svg: &mut SvgBuilder, text: &str, x: f64, start_y: f64) -> f64 {
    let line_h = 14.0;
    let mut y = start_y;
    let mut in_code = false;
    let mut list_counter = 0u32;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "<code>" {
            in_code = true;
            continue;
        }
        if trimmed == "</code>" {
            in_code = false;
            continue;
        }
        if in_code {
            if !trimmed.is_empty() {
                let mono = trimmed.replace(' ', "\u{00a0}");
                svg.text(x, y, &mono, "start", SMALL_FONT);
                y += line_h;
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ").or_else(|| trimmed.strip_prefix('#').filter(|r| !r.is_empty())) {
            list_counter += 1;
            svg.text(x, y, &format!("{list_counter}."), "start", SMALL_FONT);
            svg.text(x + 18.0, y, rest.trim(), "start", SMALL_FONT);
            y += line_h;
            continue;
        }
        if !trimmed.is_empty() {
            svg.text(x, y, trimmed, "start", SMALL_FONT);
            y += line_h;
        }
    }

    (y - start_y).max(line_h)
}

/// Count the number of visible text lines in a note (for height pre-computation).
fn count_note_lines(text: &str) -> usize {
    let mut count = 0;
    for line in text.lines() {
        let trimmed = line.trim();
        // Skip the <code>/<code> delimiter lines themselves; code body lines count.
        if trimmed == "<code>" || trimmed == "</code>" {
            continue;
        }
        if !trimmed.is_empty() {
            count += 1;
        }
    }
    count.max(1)
}

/// Render the PlantUML empty-diagram welcome screen.
fn render_empty_welcome() -> String {
    let w = 480.0_f64;
    let h = 260.0_f64;
    let mut svg = SvgBuilder::new(w, h);
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
        svg.text(x, y, line, "start", SMALL_FONT);
        y += lh;
    }
    svg.finalize()
}

/// Render a sequence diagram to SVG.
pub fn render(diagram: &SequenceDiagram, theme: &Theme) -> String {
    // Empty diagram with no title — render the PlantUML welcome screen.
    // If there's a title, fall through to render it.
    if diagram.participants.is_empty()
        && diagram.events.is_empty()
        && diagram.meta.title.is_none()
    {
        return render_empty_welcome();
    }

    let ss = &theme.sequence;
    let n = diagram.participants.len().max(1);

    // Calculate participant widths based on text metrics.
    let participant_widths: Vec<f64> = diagram
        .participants
        .iter()
        .map(|p| {
            let display = participant_display_label(p);
            let text_w = metrics::text_width(&display, FONT_SIZE);
            (text_w + PARTICIPANT_PADDING * 2.0).max(MIN_PARTICIPANT_WIDTH)
        })
        .collect();

    let total_participant_width: f64 = participant_widths.iter().sum();
    let total_width =
        LEFT_MARGIN * 2.0 + total_participant_width + ((n - 1) as f64) * PARTICIPANT_GAP;

    // Count events that consume vertical space.
    let event_height: f64 = diagram
        .events
        .iter()
        .map(|e| match e {
            Event::Message(_)
            | Event::Divider(_)
            | Event::Delay(_)
            | Event::Note(_)
            | Event::Return(_)
            | Event::Ref(_) => MESSAGE_HEIGHT,
            Event::NoteOnLink(_) => MESSAGE_HEIGHT / 2.0,
            Event::Space(px) => px.map(|p| p as f64).unwrap_or(20.0),
            _ => 0.0,
        })
        .sum();

    // Title adds vertical space at top.
    let title_height = if diagram.meta.title.is_some() {
        PARTICIPANT_HEIGHT
    } else {
        0.0
    };

    let total_height = TOP_MARGIN * 2.0
        + PARTICIPANT_HEIGHT * 2.0
        + title_height
        + event_height
        + 20.0;

    let mut svg = SvgBuilder::new(total_width, total_height);

    // Build participant x-coordinate map (left edge of each box).
    let mut px: Vec<f64> = Vec::with_capacity(n);
    let mut x = LEFT_MARGIN;
    for (i, _) in diagram.participants.iter().enumerate() {
        px.push(x);
        x += participant_widths[i] + PARTICIPANT_GAP;
    }
    if px.is_empty() {
        px.push(LEFT_MARGIN);
    }

    let participant_center = |id: &str| -> f64 {
        diagram
            .participants
            .iter()
            .position(|p| p.id == id)
            .map(|i| px[i] + participant_widths[i] / 2.0)
            .unwrap_or(0.0)
    };

    // Emit warning for unsupported skinparams.
    let is_handwritten = diagram
        .meta
        .skinparams
        .iter()
        .any(|sp| sp.key.to_lowercase() == "handwritten" && sp.value.to_lowercase() == "true");
    if is_handwritten {
        // PlantUML uses non-breaking spaces (U+00A0 / &#160;) in this message.
        let nbsp = '\u{00a0}';
        let msg = format!(
            "Please{n}use{n}'!option{n}handwritten{n}true'{n}to{n}enable{n}handwritten",
            n = nbsp
        );
        svg.text(total_width / 2.0, TOP_MARGIN + FONT_SIZE, &msg, "middle", SMALL_FONT);
    }

    // Render header if present (top of diagram).
    if let Some(header) = &diagram.meta.header {
        svg.text(total_width / 2.0, TOP_MARGIN * 0.8, header, "middle", SMALL_FONT);
    }

    // Render footer if present (bottom of diagram).
    if let Some(footer) = &diagram.meta.footer {
        svg.text(total_width / 2.0, total_height - 4.0, footer, "middle", SMALL_FONT);
    }

    // Render caption if present (bottom, below footer).
    if let Some(caption) = &diagram.meta.caption {
        svg.text(total_width / 2.0, total_height - 4.0, caption, "middle", SMALL_FONT);
    }

    // Render legend if present.
    if let Some(legend) = &diagram.meta.legend {
        svg.render_legend(total_width - 200.0, total_height - 150.0, legend, SMALL_FONT);
    }

    // Render title if present.
    let title_offset = if let Some(title) = &diagram.meta.title {
        svg.text(total_width / 2.0, TOP_MARGIN + FONT_SIZE, title, "middle", FONT_SIZE + 2.0);
        PARTICIPANT_HEIGHT
    } else {
        0.0
    };

    // Draw participant boxes at top.
    let box_y = TOP_MARGIN + title_offset;
    for (i, p) in diagram.participants.iter().enumerate() {
        let x = px[i];
        let w = participant_widths[i];
        svg.open_group("participant");
        // Title element: name + stereotype (non-ASCII and `/` chars as dots, matching PlantUML).
        let normalize_title = |s: &str| -> String {
            s.chars()
                .map(|c| if c.is_ascii() && c != '/' { c } else { '.' })
                .collect()
        };
        let title_text = if let Some(st) = &p.stereotype {
            let dot_st: String = normalize_title(&format!("..{st}.."));
            let dot_label: String = normalize_title(&p.label);
            format!("{dot_label} {dot_st}")
        } else {
            normalize_title(&p.label)
        };
        svg.title(&title_text);
        svg.rounded_rect(
            x,
            box_y,
            w,
            PARTICIPANT_HEIGHT,
            5.0,
            &ss.participant_background,
            &ss.participant_border,
        );
        let cx = x + w / 2.0;
        let display = participant_display_label(p);
        svg.text(cx, box_y + PARTICIPANT_HEIGHT / 2.0 + 5.0, &display, "middle", FONT_SIZE);
        svg.close_group();
    }

    // Draw lifelines.
    let lifeline_start = box_y + PARTICIPANT_HEIGHT;
    let lifeline_end = total_height - TOP_MARGIN - PARTICIPANT_HEIGHT;
    for (i, _p) in diagram.participants.iter().enumerate() {
        let cx = px[i] + participant_widths[i] / 2.0;
        svg.line_segment(
            cx,
            lifeline_start,
            cx,
            lifeline_end,
            &ss.lifeline_color,
            true,
        );
    }

    // Autonumber counter.
    let mut auto_num: Option<u32> = diagram.autonumber.as_ref().map(|an| an.start);

    // Render events.
    let mut y = lifeline_start + 20.0;

    for event in &diagram.events {
        match event {
            Event::Message(msg) => {
                let from_x = participant_center(&msg.from);
                let to_x = participant_center(&msg.to);

                // Handle external messages.
                let from_x = if msg.from == "[" { 0.0 } else { from_x };
                let to_x = if msg.to == "]" { total_width } else { to_x };

                let dashed = msg.arrow.line == LineStyle::Dotted;
                svg.line_segment(from_x, y, to_x, y, &ss.participant_border, dashed);

                // Arrow head (pointing right or left).
                if to_x > from_x {
                    svg.arrow_head(to_x, y, 0.0);
                } else {
                    svg.arrow_head(to_x, y, 180.0);
                }

                let mid_x = (from_x + to_x) / 2.0;

                // Autonumber label.
                if let Some(n) = auto_num.as_mut() {
                    let an = diagram.autonumber.as_ref().unwrap();
                    let num_text = format_autonumber(*n, &an.format);
                    svg.text(mid_x, y - 16.0, &num_text, "middle", SMALL_FONT);
                    *n = n.saturating_add(an.step);
                }

                // Message label.
                if !msg.label.is_empty() {
                    let label = process_label(&msg.label);
                    svg.text(mid_x, y - 5.0, &label, "middle", SMALL_FONT);
                }

                y += MESSAGE_HEIGHT;
            }
            Event::Divider(text) => {
                svg.line_segment(
                    LEFT_MARGIN,
                    y,
                    total_width - LEFT_MARGIN,
                    y,
                    &ss.lifeline_color,
                    true,
                );
                let mid = total_width / 2.0;
                svg.text(mid, y - 3.0, text, "middle", SMALL_FONT);
                y += MESSAGE_HEIGHT;
            }
            Event::Delay(text) => {
                if let Some(t) = text {
                    let mid = total_width / 2.0;
                    svg.text(mid, y + 5.0, t, "middle", SMALL_FONT);
                }
                y += MESSAGE_HEIGHT;
            }
            Event::Space(px_opt) => {
                y += px_opt.map(|p| p as f64).unwrap_or(20.0);
            }
            Event::Note(note) => {
                let anchor_x = if let Some(first) = note.participants.first() {
                    participant_center(first)
                } else {
                    total_width / 2.0
                };

                let note_w = 120.0;
                let note_x = match note.position {
                    NotePosition::Right => anchor_x + 20.0,
                    NotePosition::Left => anchor_x - note_w - 20.0,
                    NotePosition::Over => anchor_x - note_w / 2.0,
                };

                let text_x = note_x + PARTICIPANT_PADDING;
                let text_start_y = y - 5.0;
                // Pre-count lines to size the rect.
                let line_count = count_note_lines(&note.text);
                let note_h = (line_count as f64 * 14.0 + 12.0).max(25.0);

                svg.rect(
                    note_x,
                    y - note_h / 2.0,
                    note_w,
                    note_h,
                    &ss.note_background,
                    &ss.participant_border,
                );
                render_note_text(&mut svg, &note.text, text_x, text_start_y);
                y += MESSAGE_HEIGHT;
            }
            Event::GroupStart(g) => {
                svg.open_group("group");
                svg.rect(
                    LEFT_MARGIN - 5.0,
                    y - 5.0,
                    total_width - LEFT_MARGIN * 2.0 + 10.0,
                    20.0,
                    "none",
                    &ss.lifeline_color,
                );
                let kind_str = match g.kind {
                    GroupKind::Alt => "alt",
                    GroupKind::Opt => "opt",
                    GroupKind::Loop => "loop",
                    GroupKind::Par => "par",
                    GroupKind::Break => "break",
                    GroupKind::Critical => "critical",
                    GroupKind::Group => "group",
                };
                let label_text = match &g.label {
                    Some(l) => format!("{kind_str} {l}"),
                    None => kind_str.to_string(),
                };
                svg.text(LEFT_MARGIN, y + 10.0, &label_text, "start", SMALL_FONT);
            }
            Event::GroupElse(g) => {
                let label = g.label.as_deref().unwrap_or("else");
                svg.line_segment(
                    LEFT_MARGIN - 5.0,
                    y,
                    total_width - LEFT_MARGIN + 5.0,
                    y,
                    &ss.lifeline_color,
                    true,
                );
                svg.text(LEFT_MARGIN, y + 12.0, label, "start", SMALL_FONT);
            }
            Event::GroupEnd => {
                svg.close_group();
            }
            Event::Return(ret) => {
                let mid = total_width / 2.0;
                // Autonumber label for return.
                if let Some(n) = auto_num.as_mut() {
                    let an = diagram.autonumber.as_ref().unwrap();
                    let num_text = format_autonumber(*n, &an.format);
                    svg.text(mid, y - 16.0, &num_text, "middle", SMALL_FONT);
                    *n = n.saturating_add(an.step);
                }
                if !ret.label.is_empty() {
                    let label = decode_escapes(&ret.label);
                    svg.text(mid, y - 5.0, &label, "middle", SMALL_FONT);
                }
                y += MESSAGE_HEIGHT;
            }
            Event::Ref(r) => {
                // Reference box spanning participants.
                let ref_w = crate::metrics::text_width(&r.text, SMALL_FONT) + 20.0;
                let mid = total_width / 2.0;
                svg.rect(
                    mid - ref_w / 2.0,
                    y - 10.0,
                    ref_w,
                    20.0,
                    "#FAFAFA",
                    &ss.lifeline_color,
                );
                svg.text(mid, y + 4.0, &r.text, "middle", SMALL_FONT);
                y += MESSAGE_HEIGHT;
            }
            Event::NoteOnLink(text) => {
                let mid = total_width / 2.0;
                svg.rect(
                    mid - 50.0,
                    y - 12.0,
                    100.0,
                    18.0,
                    &ss.note_background,
                    &ss.lifeline_color,
                );
                svg.text(mid, y + 2.0, text, "middle", SMALL_FONT);
                y += MESSAGE_HEIGHT / 2.0;
            }
            _ => {}
        }
    }

    // Close any groups left open by unbalanced GroupStart/GroupEnd events.
    svg.close_all_groups();

    // Draw participant boxes at bottom.
    let bottom_y = total_height - TOP_MARGIN - PARTICIPANT_HEIGHT;
    for (i, p) in diagram.participants.iter().enumerate() {
        let x = px[i];
        let w = participant_widths[i];
        svg.open_group("participant");
        let title_text = if let Some(st) = &p.stereotype {
            let dot_st: String = format!("..{st}..").chars().map(|c| if c.is_ascii() { c } else { '.' }).collect();
            let dot_label: String = p.label.chars().map(|c| if c.is_ascii() { c } else { '.' }).collect();
            format!("{dot_label} {dot_st}")
        } else {
            p.label.chars().map(|c| if c.is_ascii() { c } else { '.' }).collect()
        };
        svg.title(&title_text);
        svg.rounded_rect(
            x,
            bottom_y,
            w,
            PARTICIPANT_HEIGHT,
            5.0,
            &ss.participant_background,
            &ss.participant_border,
        );
        let cx = x + w / 2.0;
        let display = participant_display_label(p);
        svg.text(cx, bottom_y + PARTICIPANT_HEIGHT / 2.0 + 5.0, &display, "middle", FONT_SIZE);
        svg.close_group();
    }

    svg.finalize()
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
                },
                Participant {
                    id: "Bob".into(),
                    label: "Bob".into(),
                    kind: ParticipantKind::Participant,
                    order: Some(1),
                    stereotype: None,
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
                },
                activation: None,
            })],
            autonumber: None,
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
    fn parsed_then_rendered() {
        let input = "@startuml\nAlice -> Bob : hello\nBob --> Alice : hi\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Alice"));
        assert!(svg.contains("hello"));
        assert!(svg.contains("hi"));
    }
}
