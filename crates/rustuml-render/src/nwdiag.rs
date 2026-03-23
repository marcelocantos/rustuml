// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Network diagram (nwdiag) SVG renderer.

use rustuml_parser::diagram::nwdiag::*;

use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

// Layout constants.
const LEFT_MARGIN: f64 = 10.0;
const TOP_MARGIN: f64 = 20.0;
const NET_LABEL_W: f64 = 120.0; // width reserved for the network name + address on the left
const COL_W: f64 = 130.0; // horizontal spacing between host columns
const HOST_W: f64 = 100.0; // width of a host box
const HOST_H: f64 = 40.0; // height of a host box
const NET_BAR_H: f64 = 18.0; // height of the network bar
const ROW_H: f64 = 100.0; // vertical distance between consecutive network bars
const FONT_SIZE: f64 = 12.0;
const SMALL_FONT: f64 = 10.0;
const NET_COLOR_DEFAULT: &str = "#C8D8E8";
const HOST_FILL: &str = "#FAFAFA";
const GROUP_ALPHA: &str = "33"; // hex alpha for group background

pub fn render(diagram: &NwdiagDiagram, theme: &Theme) -> String {
    if diagram.networks.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"60\"></svg>\n"
            .to_string();
    }

    let gs = &theme.global;

    // Collect all unique host names in order of first appearance.
    let mut host_order: Vec<String> = Vec::new();
    for net in &diagram.networks {
        for h in &net.hosts {
            if !host_order.contains(&h.name) {
                host_order.push(h.name.clone());
            }
        }
    }
    let num_hosts = host_order.len();
    let num_nets = diagram.networks.len();

    // Column x-centre for each host (0-indexed).
    let col_cx =
        |col: usize| -> f64 { LEFT_MARGIN + NET_LABEL_W + col as f64 * COL_W + HOST_W / 2.0 };

    // y-coordinate of the top of a network bar (0-indexed).
    let net_bar_y = |row: usize| -> f64 { TOP_MARGIN + row as f64 * ROW_H };

    // y-coordinate of the host box top for a given network row.
    let host_box_y = |row: usize| -> f64 { net_bar_y(row) + NET_BAR_H + 8.0 };

    // Canvas dimensions.
    let total_w = LEFT_MARGIN + NET_LABEL_W + num_hosts.max(1) as f64 * COL_W + LEFT_MARGIN;
    let total_h = TOP_MARGIN + num_nets as f64 * ROW_H + HOST_H + TOP_MARGIN;

    let mut svg = SvgBuilder::new(total_w, total_h);

    // Draw group backgrounds first (behind everything else).
    for group in &diagram.groups {
        if group.hosts.is_empty() {
            continue;
        }
        let cols: Vec<usize> = group
            .hosts
            .iter()
            .filter_map(|h| host_order.iter().position(|n| n == h))
            .collect();
        if cols.is_empty() {
            continue;
        }
        let min_col = *cols.iter().min().unwrap();
        let max_col = *cols.iter().max().unwrap();

        let gx = col_cx(min_col) - HOST_W / 2.0 - 6.0;
        let gy = TOP_MARGIN - 4.0;
        let gw = col_cx(max_col) + HOST_W / 2.0 + 6.0 - gx;
        let gh = total_h - TOP_MARGIN - TOP_MARGIN / 2.0;

        let fill_color = group.color.as_deref().unwrap_or("#FFFF00");
        // Use fill with partial transparency by appending alpha to hex color.
        let fill = if fill_color.starts_with('#') && fill_color.len() == 7 {
            format!("{}{}", fill_color, GROUP_ALPHA)
        } else {
            fill_color.to_string()
        };

        svg.rect(gx, gy, gw, gh, &fill, fill_color);
    }

    // Draw each network bar and its connected hosts.
    for (row, net) in diagram.networks.iter().enumerate() {
        let bar_y = net_bar_y(row);
        // Full-width bar.
        let bar_x = LEFT_MARGIN + NET_LABEL_W;
        let bar_w = num_hosts.max(1) as f64 * COL_W;
        let bar_color = net.color.as_deref().unwrap_or(NET_COLOR_DEFAULT);
        svg.rect(bar_x, bar_y, bar_w, NET_BAR_H, bar_color, &gs.border_color);

        // Network label left of the bar.
        let label_x = LEFT_MARGIN + NET_LABEL_W - 8.0;
        let label_y = bar_y + NET_BAR_H / 2.0 + FONT_SIZE / 2.0 - 2.0;
        svg.text(label_x, label_y, &net.name, "end", FONT_SIZE);
        if let Some(addr) = &net.address {
            svg.text(label_x, label_y + SMALL_FONT + 2.0, addr, "end", SMALL_FONT);
        }

        // Draw hosts in this network.
        for host in &net.hosts {
            let Some(col) = host_order.iter().position(|n| n == &host.name) else {
                continue;
            };
            let cx = col_cx(col);
            let hy = host_box_y(row);

            // Vertical connector from bar to host box.
            let connector_y1 = bar_y + NET_BAR_H;
            let connector_y2 = hy;
            svg.line_segment(cx, connector_y1, cx, connector_y2, &gs.border_color, false);

            // Host label: use description if present, otherwise the host name.
            let host_label = host.description.as_deref().unwrap_or(&host.name);

            // Host box.
            let label_w = metrics::text_width(host_label, FONT_SIZE) + 16.0;
            let box_w = label_w.max(HOST_W);
            let actual_hx = cx - box_w / 2.0;
            svg.rounded_rect(
                actual_hx,
                hy,
                box_w,
                HOST_H,
                4.0,
                HOST_FILL,
                &gs.border_color,
            );

            // Host display label.
            svg.text(cx, hy + HOST_H / 2.0 - 3.0, host_label, "middle", FONT_SIZE);
            // Address below host name.
            if let Some(addr) = &host.address {
                svg.text(
                    cx,
                    hy + HOST_H / 2.0 + SMALL_FONT + 1.0,
                    addr,
                    "middle",
                    SMALL_FONT,
                );
            }
        }
    }

    svg.finalize()
}
