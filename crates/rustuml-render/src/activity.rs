// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram SVG renderer.

use rustuml_parser::diagram::activity::*;

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

/// Render an activity diagram to SVG.
pub fn render(diagram: &ActivityDiagram) -> String {
    if diagram.steps.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Calculate dimensions.
    let n = diagram.steps.len();
    let total_height = MARGIN * 2.0 + n as f64 * (ACTION_HEIGHT + V_GAP);
    let total_width = MARGIN * 2.0 + ACTION_WIDTH + 100.0;
    let cx = total_width / 2.0;

    let mut svg = SvgBuilder::new(total_width, total_height);
    let mut y = MARGIN;

    for step in &diagram.steps {
        match step {
            ActivityStep::Start => {
                // Filled circle.
                svg.rect(
                    cx - CIRCLE_R,
                    y - CIRCLE_R,
                    CIRCLE_R * 2.0,
                    CIRCLE_R * 2.0,
                    "#000",
                    "#000",
                );
                y += CIRCLE_R * 2.0 + V_GAP / 2.0;
                // Connector line.
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
            }
            ActivityStep::Stop | ActivityStep::End => {
                // Connector line.
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                // Circle with inner circle (bullseye).
                let r = CIRCLE_R;
                svg.rect(cx - r, y, r * 2.0, r * 2.0, "#000", "#000");
                y += r * 2.0 + V_GAP / 2.0;
            }
            ActivityStep::Action(text) => {
                // Connector line from previous.
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                // Rounded action box.
                svg.rounded_rect(
                    cx - ACTION_WIDTH / 2.0,
                    y,
                    ACTION_WIDTH,
                    ACTION_HEIGHT,
                    10.0,
                    "#E2E2F0",
                    "#000",
                );
                svg.text(cx, y + ACTION_HEIGHT / 2.0 + 4.0, text, "middle", FONT_SIZE);
                y += ACTION_HEIGHT + V_GAP / 2.0;
            }
            ActivityStep::If(block) => {
                // Connector.
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                // Diamond (approximated with rect — proper SVG polygon TBD).
                svg.open_group("decision");
                svg.rect(
                    cx - DIAMOND_SIZE,
                    y,
                    DIAMOND_SIZE * 2.0,
                    DIAMOND_SIZE * 2.0,
                    "#FFFACD",
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
                    svg.text(cx - DIAMOND_SIZE - 5.0, y, l, "end", SMALL_FONT);
                }
                y += V_GAP / 4.0;
            }
            ActivityStep::EndIf => {
                // Merge point — small diamond.
                svg.rect(
                    cx - DIAMOND_SIZE / 2.0,
                    y,
                    DIAMOND_SIZE,
                    DIAMOND_SIZE,
                    "#FFFACD",
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
            ActivityStep::ForkAgain | ActivityStep::SplitAgain => {
                // Visual separator for parallel branches.
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
            ActivityStep::Partition(name) => {
                svg.open_group("partition");
                svg.rect(
                    MARGIN / 2.0,
                    y,
                    total_width - MARGIN,
                    ACTION_HEIGHT,
                    "none",
                    "#999",
                );
                svg.text(MARGIN, y + 15.0, name, "start", SMALL_FONT);
                y += 20.0;
            }
            ActivityStep::EndPartition => {
                svg.close_group();
                y += 10.0;
            }
            ActivityStep::Note(text) => {
                svg.rect(
                    cx + ACTION_WIDTH / 2.0 + 10.0,
                    y - 10.0,
                    100.0,
                    20.0,
                    "#FEFFDD",
                    "#000",
                );
                svg.text(
                    cx + ACTION_WIDTH / 2.0 + 15.0,
                    y + 4.0,
                    text,
                    "start",
                    SMALL_FONT,
                );
            }
            ActivityStep::While(w) => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.rect(
                    cx - DIAMOND_SIZE,
                    y,
                    DIAMOND_SIZE * 2.0,
                    DIAMOND_SIZE * 2.0,
                    "#FFFACD",
                    "#000",
                );
                svg.text(
                    cx,
                    y + DIAMOND_SIZE + 4.0,
                    &w.condition,
                    "middle",
                    SMALL_FONT,
                );
                y += DIAMOND_SIZE * 2.0 + V_GAP / 2.0;
            }
            ActivityStep::EndWhile(label) => {
                if let Some(l) = label {
                    svg.text(cx + DIAMOND_SIZE + 5.0, y, l, "start", SMALL_FONT);
                }
                y += V_GAP / 4.0;
            }
            ActivityStep::Switch(expr) => {
                svg.line_segment(cx, y - V_GAP / 2.0, cx, y, "#000", false);
                svg.rect(
                    cx - DIAMOND_SIZE,
                    y,
                    DIAMOND_SIZE * 2.0,
                    DIAMOND_SIZE * 2.0,
                    "#FFFACD",
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
        let svg = render(&d);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Step 1"));
        assert!(svg.contains("Step 2"));
    }

    #[test]
    fn with_condition() {
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
        let svg = render(&d);
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
