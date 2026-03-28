// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! PlantUML-oriented graph layout API.
//!
//! Provides a simple graph builder that produces laid-out SVG via the
//! Sugiyama hierarchical layout algorithm (layout-rs).

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

use layout::adt::dag::NodeHandle;
use layout::backends::svg::SVGWriter;
use layout::core::base::Orientation;
use layout::core::geometry::Point;
use layout::core::style::StyleAttr;
use layout::std_shapes::shapes::{Arrow, Element, ShapeKind};
use layout::topo::layout::VisualGraph;

/// Direction of the graph layout.
#[derive(Clone, Copy, Debug, Default)]
pub enum Direction {
    #[default]
    TopToBottom,
    LeftToRight,
}

impl From<Direction> for Orientation {
    fn from(d: Direction) -> Self {
        match d {
            Direction::TopToBottom => Orientation::TopToBottom,
            Direction::LeftToRight => Orientation::LeftToRight,
        }
    }
}

/// A graph builder that produces laid-out SVG.
pub struct LayoutGraph {
    vg: VisualGraph,
    direction: Orientation,
    nodes: HashMap<String, NodeHandle>,
}

impl LayoutGraph {
    /// Creates a new layout graph with the given direction.
    pub fn new(direction: Direction) -> Self {
        let orientation = direction.into();
        Self {
            vg: VisualGraph::new(orientation),
            direction: orientation,
            nodes: HashMap::new(),
        }
    }

    /// Adds a node with the given id, label, and content-aware dimensions.
    /// Returns true if the node is new, false if it already exists.
    pub fn add_node(&mut self, id: &str, label: &str, width: f64, height: f64) -> bool {
        if self.nodes.contains_key(id) {
            return false;
        }
        let shape = ShapeKind::new_box(label);
        let style = StyleAttr::simple();
        let size = Point::new(width, height);
        let element = Element::create(shape, style, self.direction, size);
        let handle = self.vg.add_node(element);
        self.nodes.insert(id.to_string(), handle);
        true
    }

    /// Adds a circle-shaped node (for actors, states, etc.) with the given
    /// diameter. Returns true if the node is new, false if it already exists.
    pub fn add_circle_node(&mut self, id: &str, label: &str, diameter: f64) -> bool {
        if self.nodes.contains_key(id) {
            return false;
        }
        let shape = ShapeKind::new_circle(label);
        let style = StyleAttr::simple();
        let size = Point::new(diameter, diameter);
        let element = Element::create(shape, style, self.direction, size);
        let handle = self.vg.add_node(element);
        self.nodes.insert(id.to_string(), handle);
        true
    }

    /// Adds an edge between two nodes by their ids, with an optional label.
    pub fn add_edge(&mut self, from: &str, to: &str, label: Option<&str>) {
        let Some(&from_h) = self.nodes.get(from) else {
            return;
        };
        let Some(&to_h) = self.nodes.get(to) else {
            return;
        };
        let arrow = match label {
            Some(l) => Arrow::simple(l),
            None => Arrow::default(),
        };
        self.vg.add_edge(arrow, from_h, to_h);
    }

    /// Runs the layout algorithm and returns SVG output.
    pub fn to_svg(&mut self) -> String {
        let mut svg = SVGWriter::new();
        self.vg.do_it(false, false, false, &mut svg);
        svg.finalize()
    }

    /// Runs layout and extracts node positions from the resulting SVG.
    ///
    /// Returns positions in the order nodes were added, as (x, y, width, height).
    /// Returns `None` if the layout algorithm exceeds the given timeout or
    /// panics (layout-rs is known to loop infinitely on some inputs).
    pub fn layout_positions(mut self, timeout: Duration) -> Option<Vec<NodePosition>> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(self.layout_positions_no_timeout());
        });
        rx.recv_timeout(timeout).ok()
    }

    /// Runs layout without a timeout guard. Prefer [`layout_positions`] in
    /// production code; this variant is useful for tests where layout-rs is
    /// known to terminate.
    pub fn layout_positions_no_timeout(&mut self) -> Vec<NodePosition> {
        let svg = self.to_svg();
        extract_positions(&svg)
    }
}

/// Position of a laid-out node.
#[derive(Debug, Clone, Copy)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Parse rect positions from layout-rs SVG output.
fn extract_positions(svg: &str) -> Vec<NodePosition> {
    let mut positions = Vec::new();

    for line in svg.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("<rect") {
            continue;
        }

        let x = parse_svg_attr(trimmed, "x").unwrap_or(0.0);
        let y = parse_svg_attr(trimmed, "y").unwrap_or(0.0);
        let w = parse_svg_attr(trimmed, "width").unwrap_or(100.0);
        let h = parse_svg_attr(trimmed, "height").unwrap_or(40.0);

        positions.push(NodePosition {
            x,
            y,
            width: w,
            height: h,
        });
    }

    positions
}

fn parse_svg_attr(element: &str, attr: &str) -> Option<f64> {
    let prefix = format!("{attr}=\"");
    let start = element.find(&prefix)? + prefix.len();
    let rest = &element[start..];
    let end = rest.find('"')?;
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_graph_produces_svg() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("a", "Alice", 100.0, 40.0);
        g.add_node("b", "Bob", 100.0, 40.0);
        g.add_edge("a", "b", Some("hello"));

        let svg = g.to_svg();
        assert!(svg.contains("<svg"), "should produce SVG");
        assert!(svg.contains("Alice"), "should contain node label");
        assert!(svg.contains("Bob"), "should contain node label");
    }

    #[test]
    fn class_diagram_layout() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("animal", "Animal", 120.0, 50.0);
        g.add_node("dog", "Dog", 100.0, 40.0);
        g.add_node("cat", "Cat", 100.0, 40.0);
        g.add_edge("animal", "dog", Some("extends"));
        g.add_edge("animal", "cat", Some("extends"));

        let svg = g.to_svg();
        assert!(svg.contains("Animal"));
        assert!(svg.contains("Dog"));
        assert!(svg.contains("Cat"));
    }

    #[test]
    fn left_to_right_layout() {
        let mut g = LayoutGraph::new(Direction::LeftToRight);
        g.add_node("a", "Start", 100.0, 40.0);
        g.add_node("b", "Middle", 100.0, 40.0);
        g.add_node("c", "End", 100.0, 40.0);
        g.add_edge("a", "b", None);
        g.add_edge("b", "c", None);

        let svg = g.to_svg();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn duplicate_node_returns_false() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        assert!(g.add_node("a", "Alice", 100.0, 40.0));
        assert!(!g.add_node("a", "Alice", 100.0, 40.0));
    }

    #[test]
    fn edge_to_nonexistent_node_is_ignored() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("a", "Alice", 100.0, 40.0);
        g.add_edge("a", "nonexistent", Some("test"));
        // Should not panic.
        let svg = g.to_svg();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn layout_positions_with_timeout() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("a", "Alice", 100.0, 40.0);
        g.add_node("b", "Bob", 100.0, 40.0);
        g.add_edge("a", "b", Some("hello"));

        let positions = g.layout_positions(Duration::from_secs(5));
        assert!(positions.is_some(), "layout should complete within timeout");
        let positions = positions.unwrap();
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn circle_node_accepts_diameter() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        assert!(g.add_circle_node("s", "Start", 80.0));
        assert!(!g.add_circle_node("s", "Start", 80.0));

        let svg = g.to_svg();
        assert!(svg.contains("<svg"));
    }
}
