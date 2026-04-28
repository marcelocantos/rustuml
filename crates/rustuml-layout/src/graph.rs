// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! PlantUML-oriented graph layout API.
//!
//! Uses vendored Graphviz (dot algorithm) for hierarchical layout with
//! proper edge routing via cubic bezier splines.

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use crate::graphviz_ffi;

/// Graphviz uses global state and is not thread-safe.
/// All layout operations must be serialized.
static GRAPHVIZ_LOCK: Mutex<()> = Mutex::new(());

/// Direction of the graph layout.
#[derive(Clone, Copy, Debug, Default)]
pub enum Direction {
    #[default]
    TopToBottom,
    LeftToRight,
}

/// A graph builder that produces laid-out node positions and edge paths.
pub struct LayoutGraph {
    direction: Direction,
    nodes: Vec<(String, String, f64, f64, bool)>, // (id, label, w, h, is_circle)
    edges: Vec<(String, String, Option<String>)>, // (from, to, label)
}

impl LayoutGraph {
    /// Creates a new layout graph with the given direction.
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Adds a rectangular node. Returns true if new, false if duplicate.
    pub fn add_node(&mut self, id: &str, label: &str, width: f64, height: f64) -> bool {
        if self.nodes.iter().any(|(nid, ..)| nid == id) {
            return false;
        }
        self.nodes
            .push((id.to_string(), label.to_string(), width, height, false));
        true
    }

    /// Adds a circle-shaped node. Returns true if new, false if duplicate.
    pub fn add_circle_node(&mut self, id: &str, label: &str, diameter: f64) -> bool {
        if self.nodes.iter().any(|(nid, ..)| nid == id) {
            return false;
        }
        self.nodes
            .push((id.to_string(), label.to_string(), diameter, diameter, true));
        true
    }

    /// Adds an edge between two nodes by their ids.
    pub fn add_edge(&mut self, from: &str, to: &str, label: Option<&str>) {
        self.edges
            .push((from.to_string(), to.to_string(), label.map(String::from)));
    }

    /// Runs Graphviz dot layout and returns full results (positions + edge paths).
    /// Returns `None` if layout exceeds the timeout or panics.
    pub fn layout_full(self, timeout: Duration) -> Option<LayoutResult> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(self.layout_full_no_timeout());
        });
        rx.recv_timeout(timeout).ok()
    }

    /// Runs layout and returns only node positions (backwards compatibility).
    pub fn layout_positions(self, timeout: Duration) -> Option<Vec<NodePosition>> {
        self.layout_full(timeout)
            .map(|result| result.node_positions)
    }

    /// Runs layout without timeout. Prefer [`layout_full`] in production.
    pub fn layout_full_no_timeout(&self) -> LayoutResult {
        self.run_graphviz_layout()
    }

    /// Runs layout without timeout, returning only positions.
    pub fn layout_positions_no_timeout(&self) -> Vec<NodePosition> {
        self.layout_full_no_timeout().node_positions
    }

    fn run_graphviz_layout(&self) -> LayoutResult {
        let _lock = GRAPHVIZ_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // SAFETY: All Graphviz FFI calls are serialized by GRAPHVIZ_LOCK.
        // The GVC context, graph, and all node/edge handles are created and
        // freed within this scope. No pointers escape.
        unsafe { self.run_graphviz_layout_inner() }
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn run_graphviz_layout_inner(&self) -> LayoutResult {
        let gvc = graphviz_ffi::gvContext();

        // Register the statically-linked dot layout plugin.
        graphviz_ffi::gvAddLibrary(
            gvc,
            std::ptr::addr_of_mut!(graphviz_ffi::gvplugin_dot_layout_LTX_library),
        );

        let graph_name = CString::new("G").unwrap();
        let g = graphviz_ffi::agopen(
            graph_name.as_ptr(),
            graphviz_ffi::Agdirected,
            std::ptr::null_mut(),
        );

        // Set graph direction.
        let rankdir_key = CString::new("rankdir").unwrap();
        let rankdir_val = match self.direction {
            Direction::TopToBottom => CString::new("TB").unwrap(),
            Direction::LeftToRight => CString::new("LR").unwrap(),
        };
        let empty = CString::new("").unwrap();
        graphviz_ffi::agsafeset(
            g as *mut c_void,
            rankdir_key.as_ptr(),
            rankdir_val.as_ptr(),
            empty.as_ptr(),
        );

        // Build nodes.
        let mut node_handles: HashMap<String, *mut graphviz_ffi::Agnode_t> = HashMap::new();
        let mut node_order: Vec<String> = Vec::new();

        let width_key = CString::new("width").unwrap();
        let height_key = CString::new("height").unwrap();
        let shape_key = CString::new("shape").unwrap();
        let fixedsize_key = CString::new("fixedsize").unwrap();
        let fixedsize_val = CString::new("true").unwrap();
        let circle_val = CString::new("circle").unwrap();
        let box_val = CString::new("box").unwrap();

        for (id, _label, w, h, is_circle) in &self.nodes {
            let cid = CString::new(id.as_str()).unwrap();
            let node = graphviz_ffi::agnode(g, cid.as_ptr(), 1);

            // Graphviz uses inches for width/height.
            let w_inches = *w / 72.0;
            let h_inches = *h / 72.0;
            let w_str = CString::new(format!("{w_inches:.4}")).unwrap();
            let h_str = CString::new(format!("{h_inches:.4}")).unwrap();

            graphviz_ffi::agsafeset(
                node as *mut c_void,
                width_key.as_ptr(),
                w_str.as_ptr(),
                empty.as_ptr(),
            );
            graphviz_ffi::agsafeset(
                node as *mut c_void,
                height_key.as_ptr(),
                h_str.as_ptr(),
                empty.as_ptr(),
            );
            graphviz_ffi::agsafeset(
                node as *mut c_void,
                fixedsize_key.as_ptr(),
                fixedsize_val.as_ptr(),
                empty.as_ptr(),
            );
            let shape = if *is_circle { &circle_val } else { &box_val };
            graphviz_ffi::agsafeset(
                node as *mut c_void,
                shape_key.as_ptr(),
                shape.as_ptr(),
                empty.as_ptr(),
            );

            node_handles.insert(id.clone(), node);
            node_order.push(id.clone());
        }

        // Build edges — track insertion order for result mapping.
        let mut edge_specs: Vec<(String, String)> = Vec::new();
        for (from, to, label) in &self.edges {
            let Some(&from_h) = node_handles.get(from) else {
                continue;
            };
            let Some(&to_h) = node_handles.get(to) else {
                continue;
            };
            let edge_name = CString::new(format!("{from}__{to}")).unwrap();
            let edge = graphviz_ffi::agedge(g, from_h, to_h, edge_name.as_ptr(), 1);

            if let Some(lbl) = label {
                let label_key = CString::new("label").unwrap();
                let label_val = CString::new(lbl.as_str()).unwrap();
                graphviz_ffi::agsafeset(
                    edge as *mut c_void,
                    label_key.as_ptr(),
                    label_val.as_ptr(),
                    empty.as_ptr(),
                );
            }

            edge_specs.push((from.clone(), to.clone()));
        }

        // Run layout.
        let engine = CString::new("dot").unwrap();
        graphviz_ffi::gvLayout(gvc, g, engine.as_ptr());

        // Extract node positions.
        let mut node_positions = Vec::with_capacity(self.nodes.len());
        for id in &node_order {
            let node = node_handles[id];
            let mut cx: f64 = 0.0;
            let mut cy: f64 = 0.0;
            let mut w_in: f64 = 0.0;
            let mut h_in: f64 = 0.0;

            graphviz_ffi::rustuml_node_pos(node, &mut cx, &mut cy);
            graphviz_ffi::rustuml_node_size(node, &mut w_in, &mut h_in);

            // Graphviz coordinates are in points (72 per inch), centered.
            // Convert to top-left corner coordinates.
            let w = w_in * 72.0;
            let h = h_in * 72.0;
            node_positions.push(NodePosition {
                x: cx - w / 2.0,
                y: cy - h / 2.0,
                width: w,
                height: h,
            });
        }

        // Extract edge paths.
        let mut edge_paths = Vec::with_capacity(edge_specs.len());

        // Walk edges in graph order, matching to our edge_specs.
        let mut edge_idx = 0;
        let mut n = graphviz_ffi::agfstnode(g);
        while !n.is_null() {
            let mut e = graphviz_ffi::agfstout(g, n);
            while !e.is_null() {
                let spl_count = graphviz_ffi::rustuml_edge_spl_count(e);
                let mut points = Vec::new();

                for i in 0..spl_count {
                    // Max 256 points per bezier curve (generous).
                    let mut buf = vec![0.0f64; 256 * 2];
                    let n_pts =
                        graphviz_ffi::rustuml_edge_bezier_points(e, i, buf.as_mut_ptr(), 256);
                    for j in 0..n_pts {
                        points.push((buf[j * 2], buf[j * 2 + 1]));
                    }
                }

                // Get arrow endpoint info.
                let mut sflag: i32 = 0;
                let mut sp_x: f64 = 0.0;
                let mut sp_y: f64 = 0.0;
                let mut eflag: i32 = 0;
                let mut ep_x: f64 = 0.0;
                let mut ep_y: f64 = 0.0;

                if spl_count > 0 {
                    graphviz_ffi::rustuml_edge_bezier_arrows(
                        e, 0, &mut sflag, &mut sp_x, &mut sp_y, &mut eflag, &mut ep_x, &mut ep_y,
                    );
                }

                let (from, to) = if edge_idx < edge_specs.len() {
                    edge_specs[edge_idx].clone()
                } else {
                    (String::new(), String::new())
                };

                edge_paths.push(EdgePath {
                    from,
                    to,
                    points,
                    has_start_arrow: sflag != 0,
                    start_point: if sflag != 0 { Some((sp_x, sp_y)) } else { None },
                    has_end_arrow: eflag != 0,
                    end_point: if eflag != 0 { Some((ep_x, ep_y)) } else { None },
                });

                edge_idx += 1;
                e = graphviz_ffi::agnxtout(g, e);
            }
            n = graphviz_ffi::agnxtnode(g, n);
        }

        // Cleanup.
        graphviz_ffi::gvFreeLayout(gvc, g);
        graphviz_ffi::agclose(g);
        graphviz_ffi::gvFreeContext(gvc);

        // Graphviz uses math coordinates (Y increases upward).
        // Convert to screen coordinates (Y increases downward).
        let max_y = node_positions
            .iter()
            .map(|p| p.y + p.height)
            .fold(0.0f64, f64::max);

        for pos in &mut node_positions {
            pos.y = max_y - pos.y - pos.height;
        }
        for path in &mut edge_paths {
            for pt in &mut path.points {
                pt.1 = max_y - pt.1;
            }
            if let Some(ref mut sp) = path.start_point {
                sp.1 = max_y - sp.1;
            }
            if let Some(ref mut ep) = path.end_point {
                ep.1 = max_y - ep.1;
            }
        }

        LayoutResult {
            node_positions,
            edge_paths,
        }
    }
}

/// Full layout result with both node positions and edge routing.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub node_positions: Vec<NodePosition>,
    pub edge_paths: Vec<EdgePath>,
}

/// Position of a laid-out node (top-left corner).
#[derive(Debug, Clone, Copy)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Routed path of an edge as cubic bezier control points.
#[derive(Debug, Clone)]
pub struct EdgePath {
    /// Source node id.
    pub from: String,
    /// Target node id.
    pub to: String,
    /// Cubic bezier control points: groups of 4 (start, cp1, cp2, end).
    /// For multi-segment splines, segments share endpoints.
    pub points: Vec<(f64, f64)>,
    /// Whether this edge has a start arrowhead.
    pub has_start_arrow: bool,
    /// Arrow anchor at start (if present).
    pub start_point: Option<(f64, f64)>,
    /// Whether this edge has an end arrowhead.
    pub has_end_arrow: bool,
    /// Arrow anchor at end (if present).
    pub end_point: Option<(f64, f64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_graph_positions() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("a", "Alice", 100.0, 40.0);
        g.add_node("b", "Bob", 100.0, 40.0);
        g.add_edge("a", "b", Some("hello"));

        let result = g.layout_full_no_timeout();
        assert_eq!(result.node_positions.len(), 2);
        // In top-to-bottom, Alice should be above Bob.
        assert!(
            result.node_positions[0].y < result.node_positions[1].y,
            "Alice (y={}) should be above Bob (y={})",
            result.node_positions[0].y,
            result.node_positions[1].y
        );
    }

    #[test]
    fn edge_has_spline_points() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("a", "Alice", 100.0, 40.0);
        g.add_node("b", "Bob", 100.0, 40.0);
        g.add_edge("a", "b", None);

        let result = g.layout_full_no_timeout();
        assert_eq!(result.edge_paths.len(), 1);
        let path = &result.edge_paths[0];
        assert!(
            !path.points.is_empty(),
            "edge should have spline control points"
        );
        // Cubic bezier: should be 3N+1 points (1 start + N segments of 3).
        assert!(
            (path.points.len() - 1).is_multiple_of(3),
            "expected 3N+1 points, got {}",
            path.points.len()
        );
    }

    #[test]
    fn three_node_graph_layout() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("a", "A", 100.0, 40.0);
        g.add_node("b", "B", 100.0, 40.0);
        g.add_node("c", "C", 100.0, 40.0);
        g.add_edge("a", "b", None);
        g.add_edge("a", "c", None);

        let result = g.layout_full_no_timeout();
        assert_eq!(result.node_positions.len(), 3);
        assert_eq!(result.edge_paths.len(), 2);
    }

    #[test]
    fn left_to_right_layout() {
        let mut g = LayoutGraph::new(Direction::LeftToRight);
        g.add_node("a", "Start", 100.0, 40.0);
        g.add_node("b", "End", 100.0, 40.0);
        g.add_edge("a", "b", None);

        let result = g.layout_full_no_timeout();
        assert_eq!(result.node_positions.len(), 2);
        // In left-to-right, Start should be left of End.
        assert!(
            result.node_positions[0].x < result.node_positions[1].x,
            "Start (x={}) should be left of End (x={})",
            result.node_positions[0].x,
            result.node_positions[1].x
        );
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

        let result = g.layout_full_no_timeout();
        assert_eq!(result.node_positions.len(), 1);
        assert_eq!(result.edge_paths.len(), 0);
    }

    #[test]
    fn layout_with_timeout() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("a", "Alice", 100.0, 40.0);
        g.add_node("b", "Bob", 100.0, 40.0);
        g.add_edge("a", "b", Some("hello"));

        let result = g.layout_full(Duration::from_secs(5));
        assert!(result.is_some(), "layout should complete within timeout");
        let result = result.unwrap();
        assert_eq!(result.node_positions.len(), 2);
        assert_eq!(result.edge_paths.len(), 1);
    }

    #[test]
    fn circle_node_works() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        assert!(g.add_circle_node("s", "Start", 80.0));
        assert!(!g.add_circle_node("s", "Start", 80.0));

        let result = g.layout_full_no_timeout();
        assert_eq!(result.node_positions.len(), 1);
    }
}
