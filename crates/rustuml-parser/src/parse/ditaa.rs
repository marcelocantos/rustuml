// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Ditaa (ASCII art → diagram) parser.
//!
//! Scans a 2D character grid to find rectangular boxes (delimited by `+`, `-`, `|`),
//! rounded boxes (`/`, `\`), connection lines (`-`, `=`, `.`, `:`, `|`, arrows),
//! colour tags (`{c}`, `{r}`, `{g}`, `{y}`, `{b}`), and text within shapes.

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::ditaa::*;

/// Parse `@startditaa` … `@endditaa` lines into a `DitaaDiagram`.
pub fn parse_ditaa(lines: &[String]) -> Result<DitaaDiagram, ParseError> {
    // Extract the body between @startditaa and @endditaa.
    let body: Vec<&str> = lines
        .iter()
        .map(|s| s.as_str())
        .filter(|l| {
            let t = l.trim();
            !t.starts_with("@start") && !t.starts_with("@end") && !t.is_empty()
        })
        .collect();

    if body.is_empty() {
        return Ok(DitaaDiagram {
            meta: DiagramMeta::default(),
            grid_width: 0,
            grid_height: 0,
            shapes: Vec::new(),
            connections: Vec::new(),
        });
    }

    // Build the grid, padding short lines.
    let grid_width = body.iter().map(|l| l.len()).max().unwrap_or(0);
    let grid_height = body.len();
    let grid: Vec<Vec<char>> = body
        .iter()
        .map(|l| {
            let mut row: Vec<char> = l.chars().collect();
            row.resize(grid_width, ' ');
            row
        })
        .collect();

    let mut used = vec![vec![false; grid_width]; grid_height];
    let mut shapes = Vec::new();
    let mut connections = Vec::new();

    // Pass 1: find rectangular shapes.
    find_shapes(&grid, &mut used, &mut shapes);

    // Pass 2: find connections.
    find_connections(&grid, &used, &shapes, &mut connections);

    Ok(DitaaDiagram {
        meta: DiagramMeta::default(),
        grid_width,
        grid_height,
        shapes,
        connections,
    })
}

/// Scan for rectangular shapes top-left to bottom-right.
fn find_shapes(grid: &[Vec<char>], used: &mut [Vec<bool>], shapes: &mut Vec<DitaaShape>) {
    let h = grid.len();
    if h == 0 {
        return;
    }
    let w = grid[0].len();

    for r in 0..h {
        for c in 0..w {
            if used[r][c] {
                continue;
            }
            let ch = grid[r][c];
            // Normal box starts with '+'
            if ch == '+' {
                if let Some(shape) = try_box(grid, r, c, DitaaShapeKind::Box) {
                    mark_used(used, &shape);
                    shapes.push(shape);
                }
            }
            // Rounded box starts with '/'
            else if ch == '/'
                && c + 1 < w
                && grid[r][c + 1] == '-'
                && let Some(shape) = try_rounded_box(grid, r, c)
            {
                mark_used(used, &shape);
                shapes.push(shape);
            }
        }
    }
}

/// Try to trace a rectangular box starting at (r, c) where grid[r][c] == '+'.
fn try_box(grid: &[Vec<char>], r: usize, c: usize, kind: DitaaShapeKind) -> Option<DitaaShape> {
    let h = grid.len();
    let w = grid[0].len();

    // Scan right along the top edge to find the matching '+'.
    let mut right = c + 1;
    while right < w {
        let ch = grid[r][right];
        if ch == '+' {
            break;
        }
        if ch != '-' && ch != '/' {
            return None;
        }
        right += 1;
    }
    if right >= w || grid[r][right] != '+' {
        return None;
    }

    // Scan down from (r, c) to find bottom-left '+'.
    let mut bottom = r + 1;
    while bottom < h {
        let ch = grid[bottom][c];
        if ch == '+' {
            break;
        }
        if ch != '|' {
            return None;
        }
        bottom += 1;
    }
    if bottom >= h || grid[bottom][c] != '+' {
        return None;
    }

    // Verify bottom edge.
    for &ch in &grid[bottom][(c + 1)..right] {
        if ch != '-' && ch != '+' && ch != '/' {
            return None;
        }
    }
    if grid[bottom][right] != '+' {
        return None;
    }

    // Verify right edge.
    for row in &grid[(r + 1)..bottom] {
        if row[right] != '|' {
            return None;
        }
    }

    // Check for document shape: bottom edge contains '/'.
    let actual_kind = if (c + 1..right).any(|cc| grid[bottom][cc] == '/') {
        DitaaShapeKind::Document
    } else if c > 0 && grid[r][c] == '/' {
        DitaaShapeKind::Storage
    } else {
        kind
    };

    // Extract text and colour from interior.
    let (text, color) = extract_text_and_color(grid, r + 1, c + 1, bottom, right);

    Some(DitaaShape {
        col: c,
        row: r,
        width: right - c + 1,
        height: bottom - r + 1,
        kind: actual_kind,
        text,
        color,
    })
}

/// Try to trace a rounded box starting at (r, c) where grid[r][c] == '/'.
fn try_rounded_box(grid: &[Vec<char>], r: usize, c: usize) -> Option<DitaaShape> {
    let h = grid.len();
    let w = grid[0].len();

    // Scan right along the top edge to find '\'.
    let mut right = c + 1;
    while right < w {
        let ch = grid[r][right];
        if ch == '\\' {
            break;
        }
        if ch != '-' {
            return None;
        }
        right += 1;
    }
    if right >= w || grid[r][right] != '\\' {
        return None;
    }

    // Scan down from (r, c) to find bottom-left '\'.
    let mut bottom = r + 1;
    while bottom < h {
        let ch = grid[bottom][c];
        if ch == '\\' {
            break;
        }
        if ch != '|' {
            return None;
        }
        bottom += 1;
    }
    if bottom >= h || grid[bottom][c] != '\\' {
        return None;
    }

    // Verify bottom edge.
    for &ch in &grid[bottom][(c + 1)..right] {
        if ch != '-' {
            return None;
        }
    }
    if grid[bottom][right] != '/' {
        return None;
    }

    // Verify right edge.
    for row in &grid[(r + 1)..bottom] {
        if row[right] != '|' {
            return None;
        }
    }

    let (text, color) = extract_text_and_color(grid, r + 1, c + 1, bottom, right);

    Some(DitaaShape {
        col: c,
        row: r,
        width: right - c + 1,
        height: bottom - r + 1,
        kind: DitaaShapeKind::RoundedBox,
        text,
        color,
    })
}

/// Extract text content and optional colour tag from the interior of a shape.
fn extract_text_and_color(
    grid: &[Vec<char>],
    top: usize,
    left: usize,
    bottom: usize,
    right: usize,
) -> (Option<String>, Option<DitaaColor>) {
    let mut color = None;
    let mut text_lines: Vec<String> = Vec::new();

    for row in &grid[top..bottom] {
        let line: String = row[left..right].iter().collect();
        let trimmed = line.trim();

        // Check for colour tag.
        if let Some(c) = parse_color_tag(trimmed) {
            color = Some(c);
            continue;
        }

        if !trimmed.is_empty() {
            text_lines.push(trimmed.to_string());
        }
    }

    let text = if text_lines.is_empty() {
        None
    } else {
        Some(text_lines.join("\n"))
    };

    (text, color)
}

fn parse_color_tag(s: &str) -> Option<DitaaColor> {
    // Tags like {c}, {r}, {g}, {y}, {b} possibly with surrounding text.
    if s.contains("{c}") {
        Some(DitaaColor::Cyan)
    } else if s.contains("{r}") {
        Some(DitaaColor::Red)
    } else if s.contains("{g}") {
        Some(DitaaColor::Green)
    } else if s.contains("{y}") {
        Some(DitaaColor::Yellow)
    } else if s.contains("{b}") {
        Some(DitaaColor::Blue)
    } else {
        None
    }
}

/// Mark grid cells belonging to a shape as used.
fn mark_used(used: &mut [Vec<bool>], shape: &DitaaShape) {
    for rr in shape.row..shape.row + shape.height {
        for cc in shape.col..shape.col + shape.width {
            if rr < used.len() && cc < used[0].len() {
                used[rr][cc] = true;
            }
        }
    }
}

/// Find connection lines and arrows in the grid.
fn find_connections(
    grid: &[Vec<char>],
    used: &[Vec<bool>],
    shapes: &[DitaaShape],
    connections: &mut Vec<DitaaConnection>,
) {
    let h = grid.len();
    if h == 0 {
        return;
    }
    let w = grid[0].len();

    let mut visited = vec![vec![false; w]; h];

    for r in 0..h {
        for c in 0..w {
            if visited[r][c] || used[r][c] {
                continue;
            }
            let ch = grid[r][c];

            // Horizontal connections: `-` or `.` runs.
            if ch == '-' || ch == '=' {
                if let Some(conn) = trace_horizontal(grid, used, shapes, &mut visited, r, c, false)
                {
                    connections.push(conn);
                }
            } else if ch == '.' {
                if let Some(conn) = trace_horizontal(grid, used, shapes, &mut visited, r, c, true) {
                    connections.push(conn);
                }
            }
            // Vertical connections: `|` or `:` runs.
            else if ch == '|' {
                if let Some(conn) = trace_vertical(grid, used, shapes, &mut visited, r, c, false) {
                    connections.push(conn);
                }
            } else if ch == ':'
                && let Some(conn) = trace_vertical(grid, used, shapes, &mut visited, r, c, true)
            {
                connections.push(conn);
            }
        }
    }
}

fn trace_horizontal(
    grid: &[Vec<char>],
    used: &[Vec<bool>],
    _shapes: &[DitaaShape],
    visited: &mut [Vec<bool>],
    r: usize,
    start_c: usize,
    dashed: bool,
) -> Option<DitaaConnection> {
    let w = grid[0].len();
    let expected = if dashed { '.' } else { '-' };
    let alt = if dashed { '.' } else { '=' };

    let mut end_c = start_c;
    while end_c < w && !used[r][end_c] {
        let ch = grid[r][end_c];
        if ch == expected || ch == alt || ch == '>' || ch == '<' || ch == '+' {
            visited[r][end_c] = true;
            end_c += 1;
        } else {
            break;
        }
    }

    if end_c <= start_c + 1 {
        return None;
    }

    let actual_end = end_c - 1;
    let start_arrow = grid[r][start_c] == '<';
    let end_arrow = grid[r][actual_end] == '>';

    Some(DitaaConnection {
        segments: vec![DitaaSegment {
            start_col: start_c,
            start_row: r,
            end_col: actual_end,
            end_row: r,
        }],
        dashed,
        start_arrow,
        end_arrow,
    })
}

fn trace_vertical(
    grid: &[Vec<char>],
    used: &[Vec<bool>],
    _shapes: &[DitaaShape],
    visited: &mut [Vec<bool>],
    start_r: usize,
    c: usize,
    dashed: bool,
) -> Option<DitaaConnection> {
    let h = grid.len();
    let expected = if dashed { ':' } else { '|' };

    let mut end_r = start_r;
    while end_r < h && !used[end_r][c] {
        let ch = grid[end_r][c];
        if ch == expected || ch == 'v' || ch == '^' || ch == '+' {
            visited[end_r][c] = true;
            end_r += 1;
        } else {
            break;
        }
    }

    if end_r <= start_r + 1 {
        return None;
    }

    let actual_end = end_r - 1;
    let start_arrow = grid[start_r][c] == '^';
    let end_arrow = grid[actual_end][c] == 'v';

    Some(DitaaConnection {
        segments: vec![DitaaSegment {
            start_col: c,
            start_row: start_r,
            end_col: c,
            end_row: actual_end,
        }],
        dashed,
        start_arrow,
        end_arrow,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> DitaaDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_ditaa(&lines).unwrap()
    }

    #[test]
    fn basic_box() {
        let d = parse("@startditaa\n+------+\n| Box  |\n+------+\n@endditaa");
        assert_eq!(d.shapes.len(), 1);
        assert_eq!(d.shapes[0].kind, DitaaShapeKind::Box);
        assert_eq!(d.shapes[0].text.as_deref(), Some("Box"));
    }

    #[test]
    fn two_boxes() {
        let d = parse(
            "@startditaa\n+--------+   +--------+\n|        |   |        |\n| Box A  |   | Box B  |\n|        |   |        |\n+--------+   +--------+\n@endditaa",
        );
        assert_eq!(d.shapes.len(), 2);
    }

    #[test]
    fn rounded_box() {
        let d = parse("@startditaa\n/-------\\\n| Round |\n\\-------/\n@endditaa");
        assert_eq!(d.shapes.len(), 1);
        assert_eq!(d.shapes[0].kind, DitaaShapeKind::RoundedBox);
        assert_eq!(d.shapes[0].text.as_deref(), Some("Round"));
    }

    #[test]
    fn color_tag() {
        let d = parse("@startditaa\n+--------+\n|{c}     |\n|  Cyan  |\n+--------+\n@endditaa");
        assert_eq!(d.shapes.len(), 1);
        assert_eq!(d.shapes[0].color, Some(DitaaColor::Cyan));
        assert_eq!(d.shapes[0].text.as_deref(), Some("Cyan"));
    }

    #[test]
    fn horizontal_arrow() {
        let d = parse("@startditaa\n+---+     +---+\n| A |---->| B |\n+---+     +---+\n@endditaa");
        assert_eq!(d.shapes.len(), 2);
        assert!(!d.connections.is_empty());
        assert!(d.connections[0].end_arrow);
    }

    #[test]
    fn dashed_connection() {
        let d = parse("@startditaa\n+---+    +---+\n| A |....| B |\n+---+    +---+\n@endditaa");
        assert!(!d.connections.is_empty());
        assert!(d.connections[0].dashed);
    }

    #[test]
    fn vertical_arrow() {
        let d = parse("@startditaa\n+---+\n| A |\n+---+\n  |\n  v\n+---+\n| B |\n+---+\n@endditaa");
        assert_eq!(d.shapes.len(), 2);
        assert!(!d.connections.is_empty());
        assert!(d.connections[0].end_arrow);
    }

    #[test]
    fn empty_diagram() {
        let d = parse("@startditaa\n@endditaa");
        assert_eq!(d.shapes.len(), 0);
        assert_eq!(d.connections.len(), 0);
    }
}
