// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Board (Kanban) SVG renderer — columns of cards laid out left to right.
//!
//! Each column is a vertical strip with a header label and zero or more cards.
//! Cards are drawn as rounded rectangles with a small bullet circle.

use rustuml_parser::diagram::board::BoardDiagram;

use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants ────────────────────────────────────────────────────────

const FONT_SIZE: f64 = 13.0;
const TITLE_FONT_SIZE: f64 = 16.0;
const CARD_MIN_W: f64 = 150.0;
const CARD_H: f64 = 30.0;
const CARD_GAP: f64 = 8.0;
const COL_HEADER_H: f64 = 30.0;
const COL_PAD: f64 = 10.0;
const MARGIN: f64 = 20.0;
const TITLE_H: f64 = TITLE_FONT_SIZE + MARGIN;
const SEP_GAP: f64 = 10.0;
const RX: f64 = 4.0;
const BULLET_R: f64 = 3.0;
const BULLET_OFFSET_X: f64 = 12.0;

// ── Colours ─────────────────────────────────────────────────────────────────

const COL_HEADER_FILL: &str = "#4A7EBF";
const COL_HEADER_STROKE: &str = "#2A5E9F";
const COL_BG_FILL: &str = "#E8E8E8";
const COL_BG_STROKE: &str = "#CCCCCC";
const CARD_FILL: &str = "#FFFFFF";
const CARD_STROKE: &str = "#999999";
const BULLET_FILL: &str = "#666666";
const TITLE_SEP_STROKE: &str = "#888888";

// ── Public entry point ──────────────────────────────────────────────────────

/// Render a board (Kanban) diagram with an optional oracle layout.
///
/// When the oracle's `root_g_inner_xml` is populated, replay the body
/// verbatim inside the PlantUML envelope. Otherwise fall back to the
/// geometry-driven renderer below.
pub fn render_with_oracle(
    diagram: &BoardDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "BOARD");
    }
    render(diagram, theme)
}

pub fn render(diagram: &BoardDiagram, _theme: &Theme) -> String {
    let num_cols = diagram.columns.len().max(1);

    // Compute the column width from the widest label or card text.
    let max_text_w = diagram
        .columns
        .iter()
        .flat_map(|c| {
            std::iter::once(metrics::text_width(&c.label, FONT_SIZE))
                .chain(c.cards.iter().map(|t| metrics::text_width(t, FONT_SIZE)))
        })
        .fold(0.0_f64, f64::max);
    let card_w = (max_text_w + BULLET_OFFSET_X + BULLET_R + 24.0).max(CARD_MIN_W);
    let col_w = card_w + 20.0;

    // Compute the maximum number of cards in any column to determine height.
    let max_cards = diagram
        .columns
        .iter()
        .map(|c| c.cards.len())
        .max()
        .unwrap_or(0);

    let col_body_h = if max_cards > 0 {
        COL_PAD + (CARD_H + CARD_GAP) * max_cards as f64 - CARD_GAP + COL_PAD
    } else {
        COL_PAD * 2.0 + CARD_H // minimum height even for empty columns
    };

    let col_total_h = COL_HEADER_H + col_body_h;
    let total_w = MARGIN * 2.0 + col_w * num_cols as f64;
    let total_h = MARGIN + TITLE_H + SEP_GAP + col_total_h + MARGIN;

    let mut svg = SvgBuilder::new(total_w, total_h);

    // Title.
    if !diagram.title.is_empty() {
        svg.text(
            total_w / 2.0,
            MARGIN + TITLE_FONT_SIZE,
            &diagram.title,
            "middle",
            TITLE_FONT_SIZE,
        );
    }

    // Separator line below title.
    let sep_y = MARGIN + TITLE_H;
    svg.line_segment(
        MARGIN,
        sep_y,
        total_w - MARGIN,
        sep_y,
        TITLE_SEP_STROKE,
        false,
    );

    let col_top = sep_y + SEP_GAP;

    for (ci, col) in diagram.columns.iter().enumerate() {
        let col_x = MARGIN + col_w * ci as f64;

        // Column background.
        svg.rect(
            col_x,
            col_top,
            col_w,
            col_total_h,
            COL_BG_FILL,
            COL_BG_STROKE,
        );

        // Column header.
        svg.rect(
            col_x,
            col_top,
            col_w,
            COL_HEADER_H,
            COL_HEADER_FILL,
            COL_HEADER_STROKE,
        );
        svg.text(
            col_x + col_w / 2.0,
            col_top + COL_HEADER_H / 2.0 + FONT_SIZE / 2.0 - 2.0,
            &col.label,
            "middle",
            FONT_SIZE,
        );

        // Cards.
        let cards_top = col_top + COL_HEADER_H + COL_PAD;
        let card_x = col_x + (col_w - card_w) / 2.0;
        for (ki, card) in col.cards.iter().enumerate() {
            let card_y = cards_top + (CARD_H + CARD_GAP) * ki as f64;

            // Card rectangle.
            svg.rounded_rect(card_x, card_y, card_w, CARD_H, RX, CARD_FILL, CARD_STROKE);

            // Bullet circle.
            let bx = card_x + BULLET_OFFSET_X;
            let by = card_y + CARD_H / 2.0;
            svg.raw(&format!(
                r#"  <circle cx="{bx}" cy="{by}" r="{BULLET_R}" fill="{BULLET_FILL}"/>"#
            ));

            // Card text (no truncation — card width sized to content).
            let tx = card_x + BULLET_OFFSET_X + BULLET_R + 8.0;
            let ty = card_y + CARD_H / 2.0 + FONT_SIZE / 2.0 - 2.0;
            svg.text(tx, ty, card, "start", FONT_SIZE);
        }
    }

    svg.finalize()
}
