// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram SVG renderer.

use rustuml_parser::diagram::activity::{ActivityDiagram, ActivityStep, NotePosition};

use crate::style::Theme;
use crate::svg::SvgBuilder;

const ACTION_WIDTH: f64 = 140.0;
const ACTION_HEIGHT: f64 = 30.0;
const DIAMOND_SIZE: f64 = 20.0;
const CIRCLE_R: f64 = 10.0;
const BAR_WIDTH: f64 = 60.0;
const BAR_HEIGHT: f64 = 5.0;
const V_GAP: f64 = 40.0;
const MARGIN: f64 = 30.0;
const FONT_SIZE: f64 = 12.0;
const SMALL_FONT: f64 = 10.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = 30.0;
const DEPRECATED_HEIGHT: f64 = 20.0;

/// Render an activity diagram to SVG.
pub fn render(diagram: &ActivityDiagram, theme: &Theme) -> String {
    let as_ = &theme.activity;
    if diagram.steps.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let n = diagram.steps.len();
    let title_extra = if diagram.meta.title.is_some() { TITLE_HEIGHT } else { 0.0 };
    let deprecated_count = diagram.steps.iter().filter(|s| matches!(s, ActivityStep::DeprecatedColorAction(_))).count();
    let total_height = MARGIN * 2.0
        + title_extra
        + n as f64 * (ACTION_HEIGHT + V_GAP)
        + deprecated_count as f64 * DEPRECATED_HEIGHT;
    let total_width = MARGIN * 2.0 + ACTION_WIDTH + 100.0;
    let cx = total_width / 2.0;

    let mut svg = SvgBuilder::new(total_width, total_height);
    let mut y = MARGIN;

    if let Some(ref header) = diagram.meta.header {
        for line in header.lines() {
            let line = line.trim();
            if !line.is_empty() {
                svg.text(cx, y + SMALL_FONT, line, "middle", SMALL_FONT);
                y += SMALL_FONT + 2.0;
            }
        }
        y += 4.0;
    }

    if let Some(ref title) = diagram.meta.title {
        svg.text(cx, y + TITLE_FONT_SIZE, title, "middle", TITLE_FONT_SIZE);
        y += TITLE_HEIGHT;
    }

    if let Some(ref caption) = diagram.meta.caption {
        svg.text(cx, y + SMALL_FONT, caption, "middle", SMALL_FONT);
        y += SMALL_FONT + 4.0;
    }

    for step in &diagram.steps {
        match step {
            ActivityStep::Start => {
                svg.circle(cx, y, CIRCLE_R, &as_.start_color, &as_.start_color);
                y += CIRCLE_R * 2.0 + V_GAP / 2.0;
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
            }
            ActivityStep::Stop | ActivityStep::End => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.circle(cx, y + CIRCLE_R, CIRCLE_R, "none", &as_.stop_color);
                svg.circle(
                    cx,
                    y + CIRCLE_R,
                    CIRCLE_R * 0.6,
                    &as_.stop_color,
                    &as_.stop_color,
                );
                y += CIRCLE_R * 2.0 + V_GAP / 2.0;
            }
            ActivityStep::Action(text) => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.rounded_rect(
                    cx - ACTION_WIDTH / 2.0,
                    y,
                    ACTION_WIDTH,
                    ACTION_HEIGHT,
                    10.0,
                    &as_.action_background,
                    "#000",
                );
                svg.text(cx, y + ACTION_HEIGHT / 2.0 + 4.0, text, "middle", FONT_SIZE);
                y += ACTION_HEIGHT + V_GAP / 2.0;
            }
            ActivityStep::DeprecatedColorAction(dca) => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.rect(
                    cx - ACTION_WIDTH / 2.0,
                    y,
                    ACTION_WIDTH,
                    DEPRECATED_HEIGHT,
                    "#FFFF88",
                    "#888",
                );
                let warning = format!(
                    "This\u{a0}syntax\u{a0}is\u{a0}deprecated,\u{a0}you\u{a0}must\u{a0}add\u{a0}<<{}>>\u{a0}at\u{a0}the\u{a0}end\u{a0}of\u{a0}the\u{a0}line,\u{a0}after\u{a0}the\u{a0}';'",
                    dca.color
                );
                svg.monospace_text(cx, y + DEPRECATED_HEIGHT / 2.0 + 4.0, &warning, "middle", SMALL_FONT);
                y += DEPRECATED_HEIGHT;
                svg.rounded_rect(
                    cx - ACTION_WIDTH / 2.0,
                    y,
                    ACTION_WIDTH,
                    ACTION_HEIGHT,
                    10.0,
                    &as_.action_background,
                    "#000",
                );
                svg.text(cx, y + ACTION_HEIGHT / 2.0 + 4.0, &dca.text, "middle", FONT_SIZE);
                y += ACTION_HEIGHT + V_GAP / 2.0;
            }
            ActivityStep::Arrow(arrow) => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", arrow.dashed);
                if let Some(ref label) = arrow.label {
                    if arrow.dashed {
                        let display = format!("-> {};", label);
                        svg.text(cx + 5.0, y - V_GAP / 4.0, &display, "start", SMALL_FONT);
                    } else {
                        svg.text(cx + 5.0, y - V_GAP / 4.0, label, "start", SMALL_FONT);
                    }
                } else if arrow.dashed {
                    svg.text(cx + 5.0, y - V_GAP / 4.0, "->", "start", SMALL_FONT);
                }
                y += V_GAP / 4.0;
            }
            ActivityStep::Backward(label) => {
                svg.text(cx - DIAMOND_SIZE - 5.0, y, label, "end", SMALL_FONT);
                y += V_GAP / 4.0;
            }
            ActivityStep::Break => {
                y += V_GAP / 4.0;
            }
            ActivityStep::If(block) => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.open_group("decision");
                svg.diamond(
                    cx,
                    y + DIAMOND_SIZE,
                    DIAMOND_SIZE,
                    &as_.decision_background,
                    "#000",
                );
                svg.text(
                    cx,
                    y + DIAMOND_SIZE + 4.0,
                    &block.condition,
                    "middle",
                    SMALL_FONT,
                );
                svg.close_group();
                if let Some(label) = &block.then_label {
                    svg.text(
                        cx + DIAMOND_SIZE + 5.0,
                        y + 10.0,
                        label,
                        "start",
                        SMALL_FONT,
                    );
                }
                y += DIAMOND_SIZE * 2.0 + V_GAP / 2.0;
            }
            ActivityStep::ElseIf(block) => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.open_group("decision");
                svg.diamond(
                    cx,
                    y + DIAMOND_SIZE,
                    DIAMOND_SIZE,
                    &as_.decision_background,
                    "#000",
                );
                svg.text(
                    cx,
                    y + DIAMOND_SIZE + 4.0,
                    &block.condition,
                    "middle",
                    SMALL_FONT,
                );
                svg.close_group();
                if let Some(label) = &block.then_label {
                    svg.text(
                        cx + DIAMOND_SIZE + 5.0,
                        y + 10.0,
                        label,
                        "start",
                        SMALL_FONT,
                    );
                }
                y += DIAMOND_SIZE * 2.0 + V_GAP / 2.0;
            }
            ActivityStep::Else(label) => {
                if let Some(l) = label {
                    let display = format!("else ({})", l);
                    svg.text(cx - DIAMOND_SIZE - 5.0, y, &display, "end", SMALL_FONT);
                }
                y += V_GAP / 4.0;
            }
            ActivityStep::EndIf => {
                svg.diamond(
                    cx,
                    y + DIAMOND_SIZE / 2.0,
                    DIAMOND_SIZE / 2.0,
                    &as_.decision_background,
                    "#000",
                );
                y += DIAMOND_SIZE + V_GAP / 2.0;
            }
            ActivityStep::Fork | ActivityStep::Split => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.rect(
                    cx - BAR_WIDTH / 2.0,
                    y,
                    BAR_WIDTH,
                    BAR_HEIGHT,
                    "#000",
                    "#000",
                );
                y += BAR_HEIGHT + V_GAP / 2.0;
            }
            ActivityStep::ForkAgain => {
                svg.text(cx, y, "fork again", "middle", SMALL_FONT);
                y += SMALL_FONT + 4.0;
                svg.line_segment(
                    cx - BAR_WIDTH / 2.0,
                    y,
                    cx + BAR_WIDTH / 2.0,
                    y,
                    "#999",
                    true,
                );
                y += V_GAP / 4.0;
            }
            ActivityStep::SplitAgain => {
                svg.line_segment(
                    cx - BAR_WIDTH / 2.0,
                    y,
                    cx + BAR_WIDTH / 2.0,
                    y,
                    "#999",
                    true,
                );
                y += V_GAP / 4.0;
            }
            ActivityStep::EndFork | ActivityStep::EndSplit => {
                svg.rect(
                    cx - BAR_WIDTH / 2.0,
                    y,
                    BAR_WIDTH,
                    BAR_HEIGHT,
                    "#000",
                    "#000",
                );
                y += BAR_HEIGHT + V_GAP / 2.0;
            }
            ActivityStep::Swimlane(name) => {
                svg.line_segment(0.0, y, total_width, y, "#999", true);
                svg.text(5.0, y + 14.0, name, "start", SMALL_FONT);
                y += 20.0;
            }
            ActivityStep::Partition(partition) => {
                let fill = partition.color.as_deref().unwrap_or("none");
                svg.open_group("partition");
                svg.rect(
                    MARGIN / 2.0,
                    y,
                    total_width - MARGIN,
                    ACTION_HEIGHT,
                    fill,
                    "#999",
                );
                svg.text(MARGIN, y + 15.0, &partition.name, "start", SMALL_FONT);
                y += 20.0;
            }
            ActivityStep::EndPartition => {
                svg.close_group();
                y += 10.0;
            }
            ActivityStep::Note(note) => {
                let fill = note.color.as_deref().unwrap_or("#FEFFDD");
                let note_width = 100.0;
                let note_height = 20.0;
                let note_x = match note.position {
                    NotePosition::Right => cx + ACTION_WIDTH / 2.0 + 10.0,
                    NotePosition::Left => cx - ACTION_WIDTH / 2.0 - note_width - 10.0,
                };
                svg.rect(
                    note_x,
                    y - 10.0,
                    note_width,
                    note_height,
                    fill,
                    "#000",
                );
                svg.text(
                    note_x + 5.0,
                    y + 4.0,
                    &note.text,
                    "start",
                    SMALL_FONT,
                );
            }
            ActivityStep::While(w) => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.diamond(
                    cx,
                    y + DIAMOND_SIZE,
                    DIAMOND_SIZE,
                    &as_.decision_background,
                    "#000",
                );
                svg.text(
                    cx,
                    y + DIAMOND_SIZE + 4.0,
                    &w.condition,
                    "middle",
                    SMALL_FONT,
                );
                if let Some(label) = &w.is_label {
                    svg.text(
                        cx + DIAMOND_SIZE + 5.0,
                        y + DIAMOND_SIZE * 2.0 + 4.0,
                        label,
                        "start",
                        SMALL_FONT,
                    );
                }
                y += DIAMOND_SIZE * 2.0 + V_GAP / 2.0;
            }
            ActivityStep::EndWhile(label) => {
                if let Some(l) = label {
                    svg.text(cx - DIAMOND_SIZE - 5.0, y, l, "end", SMALL_FONT);
                }
                y += V_GAP / 4.0;
            }
            ActivityStep::Repeat => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.diamond(
                    cx,
                    y + DIAMOND_SIZE / 2.0,
                    DIAMOND_SIZE / 2.0,
                    &as_.decision_background,
                    "#000",
                );
                y += DIAMOND_SIZE + V_GAP / 2.0;
            }
            ActivityStep::RepeatWhile(rw) => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.diamond(
                    cx,
                    y + DIAMOND_SIZE,
                    DIAMOND_SIZE,
                    &as_.decision_background,
                    "#000",
                );
                svg.text(
                    cx,
                    y + DIAMOND_SIZE + 4.0,
                    &rw.condition,
                    "middle",
                    SMALL_FONT,
                );
                if let Some(label) = &rw.is_label {
                    svg.text(
                        cx + DIAMOND_SIZE + 5.0,
                        y + DIAMOND_SIZE + 4.0,
                        label,
                        "start",
                        SMALL_FONT,
                    );
                }
                y += DIAMOND_SIZE * 2.0 + V_GAP / 2.0;
            }
            ActivityStep::Switch(expr) => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.diamond(
                    cx,
                    y + DIAMOND_SIZE,
                    DIAMOND_SIZE,
                    &as_.decision_background,
                    "#000",
                );
                svg.text(cx, y + DIAMOND_SIZE + 4.0, expr, "middle", SMALL_FONT);
                y += DIAMOND_SIZE * 2.0 + V_GAP / 2.0;
            }
            ActivityStep::Case(label) => {
                svg.text(cx - ACTION_WIDTH / 2.0, y, label, "start", SMALL_FONT);
                y += 15.0;
            }
            ActivityStep::EndSwitch => {
                y += V_GAP / 4.0;
            }
            _ => {
                y += V_GAP / 4.0;
            }
        }
    }

    if let Some(ref footer) = diagram.meta.footer {
        for line in footer.lines() {
            let line = line.trim();
            if !line.is_empty() {
                svg.text(cx, y + SMALL_FONT, line, "middle", SMALL_FONT);
                y += SMALL_FONT + 2.0;
            }
        }
    }

    svg.finalize()
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
        let svg = render(&d, &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Step 1"));
        assert!(svg.contains("Step 2"));
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
                }),
                ActivityStep::Action("positive".into()),
                ActivityStep::Else(Some("no".into())),
                ActivityStep::Action("negative".into()),
                ActivityStep::EndIf,
                ActivityStep::Stop,
            ],
        };
        let svg = render(&d, &Theme::default());
        assert!(svg.contains("x &gt; 0?") || svg.contains("x > 0?")); // XML escaped
        assert!(svg.contains("positive"));
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\nstart\n:Hello;\nstop\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Hello"));
    }
}
