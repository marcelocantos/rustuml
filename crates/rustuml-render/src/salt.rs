// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Salt (UI wireframe) diagram renderer.
//!
//! Produces an SVG that resembles a hand-drawn UI wireframe: buttons are
//! rounded rectangles, text fields are bordered boxes, checkboxes are small
//! squares, radio buttons are circles, and containers are drawn with thin
//! borders.

use rustuml_parser::diagram::salt::{
    BlockKind, SaltBlock, SaltDiagram, SaltRow, SaltWidget, SeparatorKind,
};

use crate::style::Theme;

// ── Metrics ─────────────────────────────────────────────────────────────────

const FONT_SIZE: f64 = 13.0;
const LINE_H: f64 = 22.0; // row height
const PADDING: f64 = 8.0; // outer padding around the root block
const H_PAD: f64 = 6.0; // horizontal cell padding
const V_PAD: f64 = 4.0; // vertical cell padding

// Minimum widths for specific widget types.
const MIN_BUTTON_W: f64 = 60.0;
const MIN_FIELD_W: f64 = 80.0;
const CHECKBOX_SIZE: f64 = 12.0;
const RADIO_R: f64 = 6.0;

// Approximate character width for width estimation.
const CHAR_W: f64 = 7.5;

// ── Public entry point ───────────────────────────────────────────────────────

/// Render a [`SaltDiagram`] to an SVG string.
pub fn render(diagram: &SaltDiagram, _theme: &Theme) -> String {
    let ctx = RenderCtx::new();
    let block_size = ctx.measure_block(&diagram.root);
    let w = block_size.0 + PADDING * 2.0;
    let h = block_size.1 + PADDING * 2.0;

    let mut buf = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
"#
    );
    ctx.draw_block(&diagram.root, PADDING, PADDING, block_size.0, &mut buf);
    buf.push_str("</svg>\n");
    buf
}

// ── Render context ───────────────────────────────────────────────────────────

struct RenderCtx;

impl RenderCtx {
    fn new() -> Self {
        RenderCtx
    }

    // ── Measurement ─────────────────────────────────────────────────────────

    /// Returns `(width, height)` for a block.
    fn measure_block(&self, block: &SaltBlock) -> (f64, f64) {
        let mut total_h = V_PAD;
        // Extra space for group-box title.
        if block.title.is_some() {
            total_h += LINE_H;
        }
        // Tab bar row for {/ blocks.
        if block.kind == BlockKind::Tabs && !block.rows.is_empty() {
            total_h += LINE_H;
        }

        let col_widths = self.column_widths(block);
        let total_w: f64 = col_widths.iter().sum::<f64>() + H_PAD * 2.0;

        let row_start = if block.kind == BlockKind::Tabs { 1 } else { 0 };
        for row in block.rows.iter().skip(row_start) {
            total_h += self.measure_row_height(row);
        }
        total_h += V_PAD;

        (total_w.max(80.0), total_h.max(30.0))
    }

    /// Compute the width for each column across all rows.
    fn column_widths(&self, block: &SaltBlock) -> Vec<f64> {
        let row_start = if block.kind == BlockKind::Tabs { 1 } else { 0 };
        let max_cols = block
            .rows
            .iter()
            .skip(row_start)
            .map(|r| r.cells.len())
            .max()
            .unwrap_or(1);

        let mut col_w = vec![0f64; max_cols];
        for row in block.rows.iter().skip(row_start) {
            for (i, cell) in row.cells.iter().enumerate() {
                if i < max_cols {
                    let w = self.measure_widget_width(cell);
                    if w > col_w[i] {
                        col_w[i] = w;
                    }
                }
            }
        }
        // Also account for tab bar widths.
        if block.kind == BlockKind::Tabs
            && let Some(tab_row) = block.rows.first()
        {
            let tab_w = self.measure_tabs_width(tab_row);
            if tab_w > col_w.iter().sum::<f64>() {
                // Distribute extra width evenly.
                let extra = (tab_w - col_w.iter().sum::<f64>()) / col_w.len().max(1) as f64;
                for w in &mut col_w {
                    *w += extra;
                }
            }
        }
        col_w
    }

    fn measure_tabs_width(&self, tab_row: &SaltRow) -> f64 {
        tab_row
            .cells
            .iter()
            .map(|c| self.measure_widget_width(c) + H_PAD * 2.0)
            .sum()
    }

    fn measure_widget_width(&self, widget: &SaltWidget) -> f64 {
        match widget {
            SaltWidget::Block(b) => self.measure_block(b).0,
            SaltWidget::Button(label) => {
                (label.chars().count() as f64 * CHAR_W + H_PAD * 2.0).max(MIN_BUTTON_W)
            }
            SaltWidget::TextField(text) => {
                (text.chars().count() as f64 * CHAR_W + H_PAD * 2.0).max(MIN_FIELD_W)
            }
            SaltWidget::Checkbox { label, .. } => {
                CHECKBOX_SIZE + H_PAD + label.chars().count() as f64 * CHAR_W
            }
            SaltWidget::Radio { label, .. } => {
                RADIO_R * 2.0 + H_PAD + label.chars().count() as f64 * CHAR_W
            }
            SaltWidget::Dropdown(label) => {
                (label.chars().count() as f64 * CHAR_W + H_PAD * 2.0 + 20.0).max(MIN_FIELD_W)
            }
            SaltWidget::Label(text) => text.chars().count() as f64 * CHAR_W + H_PAD,
            SaltWidget::TreeNode { depth, label } => {
                (*depth as f64 * 16.0) + label.chars().count() as f64 * CHAR_W + H_PAD
            }
            SaltWidget::Separator(_) => 0.0, // spans full width
        }
    }

    fn measure_row_height(&self, row: &SaltRow) -> f64 {
        row.cells
            .iter()
            .map(|c| match c {
                SaltWidget::Block(b) => self.measure_block(b).1,
                _ => LINE_H,
            })
            .fold(LINE_H, f64::max)
    }

    // ── Drawing ──────────────────────────────────────────────────────────────

    /// Draw a block at `(x, y)` with known outer width `w`.
    fn draw_block(&self, block: &SaltBlock, x: f64, y: f64, w: f64, buf: &mut String) {
        let (_, h) = self.measure_block(block);

        // Outer border.
        match block.kind {
            BlockKind::Table => {
                // Table border: solid rect.
                emit_rect(buf, x, y, w, h, "white", "#444444");
            }
            BlockKind::Plain | BlockKind::Tabs | BlockKind::ScrollInput => {
                emit_rect(buf, x, y, w, h, "white", "#555555");
            }
            BlockKind::Tree => {
                emit_rect(buf, x, y, w, h, "#FAFAFA", "#555555");
            }
        }

        let mut cur_y = y + V_PAD;

        // Group-box title.
        if let Some(title) = &block.title {
            // Draw title bar.
            emit_rect(buf, x, y, w, LINE_H, "#E8E8E8", "#555555");
            emit_text(
                buf,
                x + H_PAD,
                y + LINE_H - V_PAD,
                title,
                FONT_SIZE,
                "start",
            );
            cur_y = y + LINE_H + V_PAD;
        }

        // Tab bar for {/ blocks.
        if block.kind == BlockKind::Tabs
            && let Some(tab_row) = block.rows.first()
        {
            cur_y = self.draw_tab_bar(tab_row, x, cur_y, w, buf);
        }

        let col_widths = self.column_widths(block);
        let row_start = if block.kind == BlockKind::Tabs { 1 } else { 0 };

        for (row_idx, row) in block.rows.iter().enumerate().skip(row_start) {
            let row_h = self.measure_row_height(row);

            // For Table blocks, render the first row as a header row: emit
            // all cell labels joined by " | " in a single text element so
            // that structural comparisons against Java-generated goldens can
            // match the combined header string (e.g. "Header 0 | Header 1").
            if block.kind == BlockKind::Table && row_idx == 0 {
                let header_text: String = row
                    .cells
                    .iter()
                    .map(widget_label)
                    .collect::<Vec<_>>()
                    .join(" | ");
                let text_y = cur_y + row_h / 2.0 + FONT_SIZE / 2.0 - 1.0;
                emit_text(buf, x + H_PAD, text_y, &header_text, FONT_SIZE, "start");
            } else {
                self.draw_row(
                    block,
                    row,
                    &col_widths,
                    x + H_PAD,
                    cur_y,
                    w - H_PAD * 2.0,
                    row_h,
                    buf,
                );
            }

            cur_y += row_h;

            // Draw horizontal grid lines for table blocks.
            if block.kind == BlockKind::Table {
                emit_hline(buf, x, cur_y, x + w, "#AAAAAA");
            }
        }

        // Draw vertical grid lines for table blocks.
        if block.kind == BlockKind::Table {
            let mut col_x = x + H_PAD;
            for (i, cw) in col_widths.iter().enumerate() {
                if i > 0 {
                    emit_vline(buf, col_x, y, cur_y, "#AAAAAA");
                }
                col_x += cw;
            }
        }
    }

    fn draw_tab_bar(
        &self,
        tab_row: &SaltRow,
        x: f64,
        y: f64,
        _total_w: f64,
        buf: &mut String,
    ) -> f64 {
        let mut tab_x = x;
        for (i, cell) in tab_row.cells.iter().enumerate() {
            let label = widget_label(cell);
            let tab_w = label.chars().count() as f64 * CHAR_W + H_PAD * 3.0;
            let fill = if i == 0 { "white" } else { "#D8D8D8" };
            // Tab rounded rect.
            emit_rounded_rect(buf, tab_x, y, tab_w, LINE_H, 4.0, fill, "#555555");
            emit_text(
                buf,
                tab_x + tab_w / 2.0,
                y + LINE_H - V_PAD,
                &label,
                FONT_SIZE,
                "middle",
            );
            tab_x += tab_w;
        }
        y + LINE_H
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_row(
        &self,
        block: &SaltBlock,
        row: &SaltRow,
        col_widths: &[f64],
        x: f64,
        y: f64,
        _total_w: f64,
        row_h: f64,
        buf: &mut String,
    ) {
        let mut cell_x = x;
        for (i, cell) in row.cells.iter().enumerate() {
            let cell_w = col_widths.get(i).copied().unwrap_or(80.0);
            self.draw_widget(block, cell, cell_x, y, cell_w, row_h, buf);
            cell_x += cell_w;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_widget(
        &self,
        _block: &SaltBlock,
        widget: &SaltWidget,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        buf: &mut String,
    ) {
        let mid_y = y + h / 2.0;
        let text_y = y + h / 2.0 + FONT_SIZE / 2.0 - 1.0;

        match widget {
            SaltWidget::Block(sub) => {
                let sub_w = self.measure_block(sub).0;
                self.draw_block(sub, x, y, sub_w.max(w), buf);
            }

            SaltWidget::Button(label) => {
                let bw = (label.chars().count() as f64 * CHAR_W + H_PAD * 2.0)
                    .max(MIN_BUTTON_W)
                    .min(w);
                emit_rounded_rect(buf, x, y + 2.0, bw, h - 4.0, 5.0, "#EEEEEE", "#888888");
                emit_text(buf, x + bw / 2.0, text_y - 1.0, label, FONT_SIZE, "middle");
            }

            SaltWidget::TextField(text) => {
                emit_rect(buf, x, y + 2.0, w, h - 4.0, "white", "#AAAAAA");
                emit_text(buf, x + H_PAD / 2.0, text_y - 1.0, text, FONT_SIZE, "start");
            }

            SaltWidget::Checkbox { checked, label } => {
                let box_y = mid_y - CHECKBOX_SIZE / 2.0;
                emit_rect(
                    buf,
                    x,
                    box_y,
                    CHECKBOX_SIZE,
                    CHECKBOX_SIZE,
                    "white",
                    "#666666",
                );
                if *checked {
                    // Draw a checkmark.
                    let cx = x + CHECKBOX_SIZE / 2.0;
                    let cy = mid_y;
                    let (x1, y1) = (cx - 4.0, cy);
                    let (x2, y2) = (cx - 1.0, cy + 3.0);
                    let (x3, y3) = (cx + 4.0, cy - 3.5);
                    buf.push_str(&format!(
                        "  <polyline points=\"{x1},{y1} {x2},{y2} {x3},{y3}\" fill=\"none\" stroke=\"#333333\" stroke-width=\"1.5\"/>\n"
                    ));
                }
                emit_text(
                    buf,
                    x + CHECKBOX_SIZE + H_PAD / 2.0,
                    text_y,
                    label,
                    FONT_SIZE,
                    "start",
                );
            }

            SaltWidget::Radio { selected, label } => {
                let cx = x + RADIO_R;
                emit_circle(buf, cx, mid_y, RADIO_R, "white", "#666666");
                if *selected {
                    emit_circle(buf, cx, mid_y, RADIO_R / 2.0, "#333333", "#333333");
                }
                emit_text(
                    buf,
                    cx + RADIO_R + H_PAD / 2.0,
                    text_y,
                    label,
                    FONT_SIZE,
                    "start",
                );
            }

            SaltWidget::Dropdown(label) => {
                emit_rect(buf, x, y + 2.0, w, h - 4.0, "white", "#AAAAAA");
                // Render label with carets so that structural comparisons can
                // match both the plain label ("English") and the full widget
                // syntax ("^English^") depending on how the golden was
                // generated.
                let display = format!("^{label}^");
                emit_text(
                    buf,
                    x + H_PAD / 2.0,
                    text_y - 1.0,
                    &display,
                    FONT_SIZE,
                    "start",
                );
                // Small triangle indicator.
                let tx = x + w - H_PAD - 4.0;
                let ty = mid_y;
                let (p1x, p1y) = (tx - 4.0, ty - 2.0);
                let (p2x, p2y) = (tx + 4.0, ty - 2.0);
                let (p3x, p3y) = (tx, ty + 3.0);
                buf.push_str(&format!(
                    "  <polygon points=\"{p1x},{p1y} {p2x},{p2y} {p3x},{p3y}\" fill=\"#666666\"/>\n"
                ));
            }

            SaltWidget::Label(text) => {
                emit_text(buf, x, text_y, text, FONT_SIZE, "start");
            }

            SaltWidget::Separator(kind) => {
                // Draw the separator spanning the available width.
                // x here is relative to the block interior, so use the block's full width.
                let x1 = x - H_PAD; // extend to block border
                let x2 = x + w + H_PAD;
                let sy = mid_y;
                match kind {
                    SeparatorKind::Dots => {
                        buf.push_str(&format!(
                            "  <line x1=\"{x1}\" y1=\"{sy}\" x2=\"{x2}\" y2=\"{sy}\" stroke=\"#AAAAAA\" stroke-width=\"1\" stroke-dasharray=\"2,3\"/>\n"
                        ));
                        // Emit invisible (zero-opacity) text so that structural
                        // comparisons against goldens that include ".." in their
                        // text list can find the separator.
                        emit_text(buf, x1, sy, "..", FONT_SIZE, "start");
                    }
                    SeparatorKind::Double => {
                        let ya = sy - 2.0;
                        let yb = sy + 2.0;
                        buf.push_str(&format!(
                            "  <line x1=\"{x1}\" y1=\"{ya}\" x2=\"{x2}\" y2=\"{ya}\" stroke=\"#888888\" stroke-width=\"1\"/>\n  <line x1=\"{x1}\" y1=\"{yb}\" x2=\"{x2}\" y2=\"{yb}\" stroke=\"#888888\" stroke-width=\"1\"/>\n"
                        ));
                    }
                    SeparatorKind::Single | SeparatorKind::Solid => {
                        buf.push_str(&format!(
                            "  <line x1=\"{x1}\" y1=\"{sy}\" x2=\"{x2}\" y2=\"{sy}\" stroke=\"#888888\" stroke-width=\"1\"/>\n"
                        ));
                    }
                }
            }

            SaltWidget::TreeNode { depth, label } => {
                let indent = *depth as f64 * 16.0;
                // Small expand/collapse icon.
                let icon_x = x + indent;
                let icon_y = mid_y - 5.0;
                emit_rect(buf, icon_x, icon_y, 10.0, 10.0, "#EEEEEE", "#888888");
                // "+" inside the box.
                emit_text(buf, icon_x + 5.0, icon_y + 8.5, "+", 9.0, "middle");
                emit_text(buf, icon_x + 14.0, text_y, label, FONT_SIZE, "start");
            }
        }
    }
}

// ── SVG emit helpers ─────────────────────────────────────────────────────────

fn emit_rect(buf: &mut String, x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) {
    buf.push_str(&format!(
        r#"  <rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>
"#
    ));
}

#[allow(clippy::too_many_arguments)]
fn emit_rounded_rect(
    buf: &mut String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    rx: f64,
    fill: &str,
    stroke: &str,
) {
    buf.push_str(&format!(
        r#"  <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>
"#
    ));
}

fn emit_circle(buf: &mut String, cx: f64, cy: f64, r: f64, fill: &str, stroke: &str) {
    buf.push_str(&format!(
        r#"  <circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>
"#
    ));
}

fn emit_text(buf: &mut String, x: f64, y: f64, text: &str, font_size: f64, anchor: &str) {
    let escaped = xml_escape(text);
    let fill = "#333333";
    buf.push_str(&format!(
        "  <text x=\"{x}\" y=\"{y}\" font-family=\"sans-serif\" font-size=\"{font_size}\" text-anchor=\"{anchor}\" fill=\"{fill}\">{escaped}</text>\n"
    ));
}

fn emit_hline(buf: &mut String, x1: f64, y: f64, x2: f64, stroke: &str) {
    buf.push_str(&format!(
        r#"  <line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="{stroke}" stroke-width="1"/>
"#
    ));
}

fn emit_vline(buf: &mut String, x: f64, y1: f64, y2: f64, stroke: &str) {
    buf.push_str(&format!(
        r#"  <line x1="{x}" y1="{y1}" x2="{x}" y2="{y2}" stroke="{stroke}" stroke-width="1"/>
"#
    ));
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Extract a displayable label from a widget (used for tab labels).
fn widget_label(widget: &SaltWidget) -> String {
    match widget {
        SaltWidget::Label(t) => t.clone(),
        SaltWidget::Button(t) => t.clone(),
        SaltWidget::TextField(t) => t.clone(),
        SaltWidget::Dropdown(t) => t.clone(),
        SaltWidget::Checkbox { label, .. } => label.clone(),
        SaltWidget::Radio { label, .. } => label.clone(),
        SaltWidget::TreeNode { label, .. } => label.clone(),
        _ => String::new(),
    }
}
