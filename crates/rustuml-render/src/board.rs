// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Board (Kanban) SVG renderer — columns of cards laid out left to right.
//!
//! Each column is a vertical strip with a header label and zero or more cards.
//! Cards are drawn as rounded rectangles with a small bullet circle.

use rustuml_parser::diagram::board::BoardDiagram;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants ────────────────────────────────────────────────────────

const FONT_SIZE: f64 = 13.0;
const TITLE_FONT_SIZE: f64 = 16.0;
const CARD_W: f64 = 150.0;
const CARD_H: f64 = 30.0;
const CARD_GAP: f64 = 8.0;
const COL_W: f64 = 170.0;
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

pub fn render(diagram: &BoardDiagram, _theme: &Theme) -> String {
    let num_cols = diagram.columns.len().max(1);

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
    let total_w = MARGIN * 2.0 + COL_W * num_cols as f64;
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
    svg.line_segment(MARGIN, sep_y, total_w - MARGIN, sep_y, TITLE_SEP_STROKE, false);

    let col_top = sep_y + SEP_GAP;

    for (ci, col) in diagram.columns.iter().enumerate() {
        let col_x = MARGIN + COL_W * ci as f64;

        // Column background.
        svg.rect(
            col_x,
            col_top,
            COL_W,
            col_total_h,
            COL_BG_FILL,
            COL_BG_STROKE,
        );

        // Column header.
        svg.rect(
            col_x,
            col_top,
            COL_W,
            COL_HEADER_H,
            COL_HEADER_FILL,
            COL_HEADER_STROKE,
        );
        svg.text(
            col_x + COL_W / 2.0,
            col_top + COL_HEADER_H / 2.0 + FONT_SIZE / 2.0 - 2.0,
            &col.label,
            "middle",
            FONT_SIZE,
        );

        // Cards.
        let cards_top = col_top + COL_HEADER_H + COL_PAD;
        let card_x = col_x + (COL_W - CARD_W) / 2.0;
        for (ki, card) in col.cards.iter().enumerate() {
            let card_y = cards_top + (CARD_H + CARD_GAP) * ki as f64;

            // Card rectangle.
            svg.rounded_rect(card_x, card_y, CARD_W, CARD_H, RX, CARD_FILL, CARD_STROKE);

            // Bullet circle.
            let bx = card_x + BULLET_OFFSET_X;
            let by = card_y + CARD_H / 2.0;
            svg.raw(&format!(
                r#"  <circle cx="{bx}" cy="{by}" r="{BULLET_R}" fill="{BULLET_FILL}"/>"#
            ));

            // Card text.
            let tx = card_x + BULLET_OFFSET_X + BULLET_R + 8.0;
            let ty = card_y + CARD_H / 2.0 + FONT_SIZE / 2.0 - 2.0;
            // Truncate text if too wide.
            let max_text_w = CARD_W - BULLET_OFFSET_X - BULLET_R - 16.0;
            let label = truncate_to_width(card, FONT_SIZE, max_text_w);
            svg.text(tx, ty, &label, "start", FONT_SIZE);
        }
    }

    svg.finalize()
}

/// Truncate a label to fit within `max_w` pixels, appending "..." if needed.
fn truncate_to_width(text: &str, font_size: f64, max_w: f64) -> String {
    let w = metrics::text_width(text, font_size);
    if w <= max_w {
        return text.to_string();
    }
    // Binary search for the longest prefix that fits with ellipsis.
    let ellipsis_w = metrics::text_width("...", font_size);
    let target = max_w - ellipsis_w;
    let mut end = text.len();
    for (i, _) in text.char_indices().rev() {
        end = i;
        let sub = &text[..end];
        if metrics::text_width(sub, font_size) <= target {
            break;
        }
    }
    format!("{}...", &text[..end])
}
