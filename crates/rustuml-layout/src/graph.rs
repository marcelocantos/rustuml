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

const DOT_POINTS_PER_INCH: f64 = 72.0;

/// Direction of the graph layout.
#[derive(Clone, Copy, Debug, Default)]
pub enum Direction {
    #[default]
    TopToBottom,
    LeftToRight,
}

/// Graph-level spacing inputs for Graphviz dot, expressed in pixels.
#[derive(Clone, Copy, Debug)]
pub struct GraphSpacing {
    pub node_sep_px: f64,
    pub rank_sep_px: f64,
}

/// Pixel dimensions of a renderer-owned edge label placeholder.
#[derive(Clone, Copy, Debug)]
pub struct EdgeLabelSize {
    pub width: f64,
    pub height: f64,
}

impl GraphSpacing {
    /// Non-activity SVEK minima from PlantUML
    /// `net.sourceforge.plantuml.svek.DotStringFactory`:
    /// `getMinNodeSep()` = 35 px, `getMinRankSep()` = 60 px, converted by
    /// `SvekUtils.pixelToInches(pixel)`.
    pub const PLANTUML_SVEK_DEFAULTS: Self = Self {
        node_sep_px: 35.0,
        rank_sep_px: 60.0,
    };

    pub const fn pixels(node_sep_px: f64, rank_sep_px: f64) -> Self {
        Self {
            node_sep_px,
            rank_sep_px,
        }
    }
}

/// A graph builder that produces laid-out node positions and edge paths.
pub struct LayoutGraph {
    direction: Direction,
    spacing: Option<GraphSpacing>,
    nodes: Vec<NodeSpec>,
    clusters: Vec<ClusterSpec>,
    edges: Vec<EdgeSpec>,
}

impl LayoutGraph {
    /// Creates a new layout graph with the given direction.
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            spacing: None,
            nodes: Vec::new(),
            clusters: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Sets graph-level dot spacing in pixels.
    pub fn with_spacing_pixels(mut self, node_sep_px: f64, rank_sep_px: f64) -> Self {
        self.spacing = Some(GraphSpacing::pixels(node_sep_px, rank_sep_px));
        self
    }

    /// Opts in to PlantUML's non-activity SVEK dot spacing minima.
    pub fn with_plantuml_svek_spacing(self) -> Self {
        self.with_spacing(GraphSpacing::PLANTUML_SVEK_DEFAULTS)
    }

    /// Sets graph-level dot spacing.
    pub fn with_spacing(mut self, spacing: GraphSpacing) -> Self {
        self.spacing = Some(spacing);
        self
    }

    /// Adds a rectangular node. Returns true if new, false if duplicate.
    pub fn add_node(&mut self, id: &str, _label: &str, width: f64, height: f64) -> bool {
        if self.nodes.iter().any(|node| node.id == id) {
            return false;
        }
        self.nodes.push(NodeSpec {
            id: id.to_string(),
            width,
            height,
            shape: NodeShape::Box,
        });
        true
    }

    /// Adds a circle-shaped node. Returns true if new, false if duplicate.
    pub fn add_circle_node(&mut self, id: &str, _label: &str, diameter: f64) -> bool {
        if self.nodes.iter().any(|node| node.id == id) {
            return false;
        }
        self.nodes.push(NodeSpec {
            id: id.to_string(),
            width: diameter,
            height: diameter,
            shape: NodeShape::Circle,
        });
        true
    }

    /// Adds a fixed-size Graphviz record node with named row ports.
    ///
    /// PlantUML's JSON/YAML Smetana path emits `_dim_...` record labels with
    /// row ports (`P0`, `P1`, ...), then routes nested-value connectors from
    /// `tailport=P{row}`. RustUML still renders the box itself; the record shape
    /// is used only to give dot row-level anchor points for splines.
    pub fn add_record_node(&mut self, id: &str, width: f64, height: f64, ports: &[String]) -> bool {
        if self.nodes.iter().any(|node| node.id == id) {
            return false;
        }
        self.nodes.push(NodeSpec {
            id: id.to_string(),
            width,
            height,
            shape: NodeShape::Record {
                ports: ports.to_vec(),
            },
        });
        true
    }

    /// Adds a Graphviz cluster/subgraph. Returns true if new, false if duplicate.
    ///
    /// This mirrors PlantUML SVEK's `ClusterDotString.printInternal`, which
    /// wraps package members in `subgraph cluster...` before dot layout and
    /// later draws package chrome from the resulting cluster bounds.
    pub fn add_cluster(&mut self, id: &str, label: &str, parent: Option<&str>) -> bool {
        if self.clusters.iter().any(|c| c.id == id) {
            return false;
        }
        self.clusters.push(ClusterSpec {
            id: id.to_string(),
            label: label.to_string(),
            parent: parent.map(String::from),
            nodes: Vec::new(),
        });
        true
    }

    /// Adds a node to an existing cluster/subgraph.
    pub fn add_cluster_node(&mut self, cluster_id: &str, node_id: &str) {
        if let Some(cluster) = self.clusters.iter_mut().find(|c| c.id == cluster_id) {
            cluster.nodes.push(node_id.to_string());
        }
    }

    /// Adds an edge between two nodes by their ids.
    pub fn add_edge(&mut self, from: &str, to: &str, label: Option<&str>) {
        self.add_edge_with_ports(from, to, label, None, None);
    }

    /// Adds an edge between two nodes, optionally binding Graphviz ports.
    pub fn add_edge_with_ports(
        &mut self,
        from: &str,
        to: &str,
        label: Option<&str>,
        tail_port: Option<&str>,
        head_port: Option<&str>,
    ) {
        self.edges.push(EdgeSpec {
            from: from.to_string(),
            to: to.to_string(),
            label: label.map(String::from),
            tail_port: tail_port.map(String::from),
            head_port: head_port.map(String::from),
            label_size: None,
            tail_label_size: None,
            head_label_size: None,
        });
    }

    /// Adds an edge whose center and endpoint labels are measured by the
    /// renderer. PlantUML `SvekEdge.appendLine` sends these dimensions to dot
    /// as fixed-size HTML tables, then draws the real text at the solved boxes.
    pub fn add_edge_with_label_sizes(
        &mut self,
        from: &str,
        to: &str,
        label_size: Option<EdgeLabelSize>,
        tail_label_size: Option<EdgeLabelSize>,
        head_label_size: Option<EdgeLabelSize>,
    ) {
        self.edges.push(EdgeSpec {
            from: from.to_string(),
            to: to.to_string(),
            label: None,
            tail_port: None,
            head_port: None,
            label_size,
            tail_label_size,
            head_label_size,
        });
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

        if let Some(spacing) = self.spacing {
            let nodesep_key = CString::new("nodesep").unwrap();
            let ranksep_key = CString::new("ranksep").unwrap();
            let nodesep_val = CString::new(dot_inches(spacing.node_sep_px)).unwrap();
            let ranksep_val = CString::new(dot_inches(spacing.rank_sep_px)).unwrap();

            graphviz_ffi::agsafeset(
                g as *mut c_void,
                nodesep_key.as_ptr(),
                nodesep_val.as_ptr(),
                empty.as_ptr(),
            );
            graphviz_ffi::agsafeset(
                g as *mut c_void,
                ranksep_key.as_ptr(),
                ranksep_val.as_ptr(),
                empty.as_ptr(),
            );
        }

        // Build nodes.
        let mut node_handles: HashMap<String, *mut graphviz_ffi::Agnode_t> = HashMap::new();
        let mut node_order: Vec<String> = Vec::new();

        let width_key = CString::new("width").unwrap();
        let height_key = CString::new("height").unwrap();
        let shape_key = CString::new("shape").unwrap();
        let label_key = CString::new("label").unwrap();
        let fixedsize_key = CString::new("fixedsize").unwrap();
        let no_label_val = CString::new("").unwrap();
        let fixedsize_val = CString::new("true").unwrap();
        let circle_val = CString::new("circle").unwrap();
        let box_val = CString::new("box").unwrap();
        let record_val = CString::new("record").unwrap();
        let arrowhead_key = CString::new("arrowhead").unwrap();
        let arrowtail_key = CString::new("arrowtail").unwrap();
        let headport_key = CString::new("headport").unwrap();
        let tailport_key = CString::new("tailport").unwrap();
        let headlabel_key = CString::new("headlabel").unwrap();
        let taillabel_key = CString::new("taillabel").unwrap();
        let label_width_key = CString::new("rustuml_label_width").unwrap();
        let label_height_key = CString::new("rustuml_label_height").unwrap();
        let tail_label_width_key = CString::new("rustuml_tail_label_width").unwrap();
        let tail_label_height_key = CString::new("rustuml_tail_label_height").unwrap();
        let head_label_width_key = CString::new("rustuml_head_label_width").unwrap();
        let head_label_height_key = CString::new("rustuml_head_label_height").unwrap();
        let external_endpoint_labels_key =
            CString::new("rustuml_external_endpoint_labels").unwrap();
        let true_val = CString::new("true").unwrap();
        let placeholder_label_val = CString::new(" ").unwrap();
        let no_arrow_val = CString::new("none").unwrap();

        for spec in &self.nodes {
            let cid = CString::new(spec.id.as_str()).unwrap();
            let node = graphviz_ffi::agnode(g, cid.as_ptr(), 1);

            // Graphviz uses inches for width/height.
            let w_inches = spec.width / DOT_POINTS_PER_INCH;
            let h_inches = spec.height / DOT_POINTS_PER_INCH;
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
            // PlantUML SVEK uses dot for geometry and renders entity labels
            // itself. Leaving Graphviz's default label (the node id) makes fixed
            // layout boxes warn and can feed label bounds back into routing.
            let label_val = match &spec.shape {
                NodeShape::Record { ports } => CString::new(record_label(ports)).unwrap(),
                NodeShape::Box | NodeShape::Circle => no_label_val.clone(),
            };
            graphviz_ffi::agsafeset(
                node as *mut c_void,
                label_key.as_ptr(),
                label_val.as_ptr(),
                empty.as_ptr(),
            );
            let shape = match spec.shape {
                NodeShape::Box => &box_val,
                NodeShape::Circle => &circle_val,
                NodeShape::Record { .. } => &record_val,
            };
            graphviz_ffi::agsafeset(
                node as *mut c_void,
                shape_key.as_ptr(),
                shape.as_ptr(),
                empty.as_ptr(),
            );

            node_handles.insert(spec.id.clone(), node);
            node_order.push(spec.id.clone());
        }

        // Build package clusters after nodes so each subgraph can include the
        // already-created node handles. Java SVEK writes `subgraph cluster...`
        // blocks around member shapes in `ClusterDotString.printInternal`;
        // Graphviz recognises the `cluster` prefix and computes `GD_bb` for
        // those package bounds.
        let mut cluster_handles: HashMap<String, *mut graphviz_ffi::Agraph_t> = HashMap::new();
        let mut cluster_order: Vec<String> = Vec::new();
        for cluster in &self.clusters {
            let parent = cluster
                .parent
                .as_ref()
                .and_then(|id| cluster_handles.get(id).copied())
                .unwrap_or(g);
            let name = CString::new(format!("cluster_{}", cluster.id)).unwrap();
            let subgraph = graphviz_ffi::agsubg(parent, name.as_ptr() as *mut _, 1);
            let label_val = CString::new(cluster.label.as_str()).unwrap();
            graphviz_ffi::agsafeset(
                subgraph as *mut c_void,
                label_key.as_ptr(),
                label_val.as_ptr(),
                empty.as_ptr(),
            );
            for node_id in &cluster.nodes {
                if let Some(&node) = node_handles.get(node_id) {
                    graphviz_ffi::agsubnode(subgraph, node, 1);
                }
            }
            cluster_handles.insert(cluster.id.clone(), subgraph);
            cluster_order.push(cluster.id.clone());
        }

        // Build edges — track insertion order for result mapping.
        let mut edge_specs: Vec<(String, String)> = Vec::new();
        for edge_spec in &self.edges {
            let Some(&from_h) = node_handles.get(&edge_spec.from) else {
                continue;
            };
            let Some(&to_h) = node_handles.get(&edge_spec.to) else {
                continue;
            };
            let edge_name = CString::new(format!("{}__{}", edge_spec.from, edge_spec.to)).unwrap();
            let edge = graphviz_ffi::agedge(g, from_h, to_h, edge_name.as_ptr(), 1);

            if let Some(lbl) = &edge_spec.label {
                let label_key = CString::new("label").unwrap();
                let label_val = CString::new(lbl.as_str()).unwrap();
                graphviz_ffi::agsafeset(
                    edge as *mut c_void,
                    label_key.as_ptr(),
                    label_val.as_ptr(),
                    empty.as_ptr(),
                );
            }
            for (label_key, width_key, height_key, size) in [
                (
                    &label_key,
                    &label_width_key,
                    &label_height_key,
                    edge_spec.label_size,
                ),
                (
                    &taillabel_key,
                    &tail_label_width_key,
                    &tail_label_height_key,
                    edge_spec.tail_label_size,
                ),
                (
                    &headlabel_key,
                    &head_label_width_key,
                    &head_label_height_key,
                    edge_spec.head_label_size,
                ),
            ] {
                let Some(size) = size else { continue };
                let width_val = CString::new(size.width.max(1.0).to_string()).unwrap();
                let height_val = CString::new(size.height.max(1.0).to_string()).unwrap();
                graphviz_ffi::agsafeset(
                    edge as *mut c_void,
                    label_key.as_ptr(),
                    placeholder_label_val.as_ptr(),
                    empty.as_ptr(),
                );
                graphviz_ffi::agsafeset(
                    edge as *mut c_void,
                    width_key.as_ptr(),
                    width_val.as_ptr(),
                    empty.as_ptr(),
                );
                graphviz_ffi::agsafeset(
                    edge as *mut c_void,
                    height_key.as_ptr(),
                    height_val.as_ptr(),
                    empty.as_ptr(),
                );
            }
            if edge_spec.tail_label_size.is_some() || edge_spec.head_label_size.is_some() {
                graphviz_ffi::agsafeset(
                    edge as *mut c_void,
                    external_endpoint_labels_key.as_ptr(),
                    true_val.as_ptr(),
                    empty.as_ptr(),
                );
            }
            if let Some(port) = &edge_spec.tail_port {
                let port_val = CString::new(port.as_str()).unwrap();
                graphviz_ffi::agsafeset(
                    edge as *mut c_void,
                    tailport_key.as_ptr(),
                    port_val.as_ptr(),
                    empty.as_ptr(),
                );
            }
            if let Some(port) = &edge_spec.head_port {
                let port_val = CString::new(port.as_str()).unwrap();
                graphviz_ffi::agsafeset(
                    edge as *mut c_void,
                    headport_key.as_ptr(),
                    port_val.as_ptr(),
                    empty.as_ptr(),
                );
            }
            // PlantUML SVEK lets dot route splines, then renders link
            // decorations itself (`SvekEdge.solveLine`). Keep Graphviz from
            // shortening splines for its own built-in arrowheads.
            graphviz_ffi::agsafeset(
                edge as *mut c_void,
                arrowhead_key.as_ptr(),
                no_arrow_val.as_ptr(),
                empty.as_ptr(),
            );
            graphviz_ffi::agsafeset(
                edge as *mut c_void,
                arrowtail_key.as_ptr(),
                no_arrow_val.as_ptr(),
                empty.as_ptr(),
            );

            edge_specs.push((edge_spec.from.clone(), edge_spec.to.clone()));
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
            let w = w_in * DOT_POINTS_PER_INCH;
            let h = h_in * DOT_POINTS_PER_INCH;
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
                    label: edge_label_position(e, 0),
                    tail_label: edge_label_position(e, 1),
                    head_label: edge_label_position(e, 2),
                });

                edge_idx += 1;
                e = graphviz_ffi::agnxtout(g, e);
            }
            n = graphviz_ffi::agnxtnode(g, n);
        }

        let mut cluster_positions = Vec::with_capacity(cluster_order.len());
        for id in &cluster_order {
            let cluster = cluster_handles[id];
            let mut ll_x: f64 = 0.0;
            let mut ll_y: f64 = 0.0;
            let mut ur_x: f64 = 0.0;
            let mut ur_y: f64 = 0.0;
            graphviz_ffi::rustuml_graph_bb(cluster, &mut ll_x, &mut ll_y, &mut ur_x, &mut ur_y);
            cluster_positions.push(ClusterPosition {
                id: id.clone(),
                x: ll_x,
                y: ll_y,
                width: ur_x - ll_x,
                height: ur_y - ll_y,
            });
        }

        let mut graph_ll_x = 0.0;
        let mut graph_ll_y = 0.0;
        let mut graph_ur_x = 0.0;
        let mut graph_ur_y = 0.0;
        graphviz_ffi::rustuml_graph_bb(
            g,
            &mut graph_ll_x,
            &mut graph_ll_y,
            &mut graph_ur_x,
            &mut graph_ur_y,
        );

        // Cleanup.
        graphviz_ffi::gvFreeLayout(gvc, g);
        graphviz_ffi::agclose(g);
        graphviz_ffi::gvFreeContext(gvc);

        // Graphviz uses math coordinates (Y increases upward).
        // Convert to screen coordinates (Y increases downward).
        let max_y = node_positions
            .iter()
            .map(|p| p.y + p.height)
            .chain(cluster_positions.iter().map(|p| p.y + p.height))
            .fold(0.0f64, f64::max);

        for pos in &mut node_positions {
            pos.y = max_y - pos.y - pos.height;
        }
        for pos in &mut cluster_positions {
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
            for label in [&mut path.label, &mut path.tail_label, &mut path.head_label]
                .into_iter()
                .flatten()
            {
                label.x -= label.width / 2.0;
                label.y = max_y - label.y - label.height / 2.0;
            }
        }

        LayoutResult {
            node_positions,
            cluster_positions,
            edge_paths,
            width: graph_ur_x - graph_ll_x,
            height: graph_ur_y - graph_ll_y,
        }
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn edge_label_position(
    edge: *mut graphviz_ffi::Agedge_t,
    kind: i32,
) -> Option<EdgeLabelPosition> {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut width = 0.0;
    let mut height = 0.0;
    (graphviz_ffi::rustuml_edge_label_box(edge, kind, &mut x, &mut y, &mut width, &mut height) != 0)
        .then_some(EdgeLabelPosition {
            x,
            y,
            width,
            height,
        })
}

fn record_label(ports: &[String]) -> String {
    if ports.is_empty() {
        return " ".to_string();
    }
    ports
        .iter()
        .map(|port| format!("<{}> ", escape_record_port(port)))
        .collect::<Vec<_>>()
        .join("|")
}

fn escape_record_port(port: &str) -> String {
    port.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

fn dot_inches(pixel: f64) -> String {
    format!("{:.6}", pixel / DOT_POINTS_PER_INCH)
}

/// Full layout result with both node positions and edge routing.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub node_positions: Vec<NodePosition>,
    pub cluster_positions: Vec<ClusterPosition>,
    pub edge_paths: Vec<EdgePath>,
    /// Full solved Graphviz envelope, including edge-label constraints.
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
struct NodeSpec {
    id: String,
    width: f64,
    height: f64,
    shape: NodeShape,
}

#[derive(Debug, Clone)]
enum NodeShape {
    Box,
    Circle,
    Record { ports: Vec<String> },
}

#[derive(Debug, Clone)]
struct EdgeSpec {
    from: String,
    to: String,
    label: Option<String>,
    tail_port: Option<String>,
    head_port: Option<String>,
    label_size: Option<EdgeLabelSize>,
    tail_label_size: Option<EdgeLabelSize>,
    head_label_size: Option<EdgeLabelSize>,
}

#[derive(Debug, Clone)]
struct ClusterSpec {
    id: String,
    label: String,
    parent: Option<String>,
    nodes: Vec<String>,
}

/// Position of a laid-out cluster/subgraph (top-left corner).
#[derive(Debug, Clone)]
pub struct ClusterPosition {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
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
    /// Center label box solved by Graphviz.
    pub label: Option<EdgeLabelPosition>,
    /// Tail label box solved by Graphviz.
    pub tail_label: Option<EdgeLabelPosition>,
    /// Head label box solved by Graphviz.
    pub head_label: Option<EdgeLabelPosition>,
}

/// Solved top-left label box in layout coordinates.
#[derive(Debug, Clone, Copy)]
pub struct EdgeLabelPosition {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
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
    fn sized_edge_labels_have_solved_boxes_and_expand_envelope() {
        let mut plain = LayoutGraph::new(Direction::TopToBottom);
        plain.add_node("a", "A", 40.0, 48.0);
        plain.add_node("b", "B", 40.0, 48.0);
        plain.add_edge("a", "b", None);
        let plain_result = plain.layout_full_no_timeout();

        let mut labeled = LayoutGraph::new(Direction::TopToBottom);
        labeled.add_node("a", "A", 40.0, 48.0);
        labeled.add_node("b", "B", 40.0, 48.0);
        labeled.add_edge_with_label_sizes(
            "a",
            "b",
            Some(EdgeLabelSize {
                width: 25.0,
                height: 15.0,
            }),
            Some(EdgeLabelSize {
                width: 33.0,
                height: 15.0,
            }),
            Some(EdgeLabelSize {
                width: 31.0,
                height: 15.0,
            }),
        );
        let labeled_result = labeled.layout_full_no_timeout();
        let edge = &labeled_result.edge_paths[0];

        assert!(edge.label.is_some());
        assert!(edge.tail_label.is_some());
        assert!(edge.head_label.is_some());
        assert!(labeled_result.width > plain_result.width);
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
    fn cluster_bounds_wrap_member_nodes() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("a", "A", 100.0, 40.0);
        g.add_node("b", "B", 100.0, 40.0);
        assert!(g.add_cluster("pkg", "pkg", None));
        g.add_cluster_node("pkg", "a");

        let result = g.layout_full_no_timeout();
        assert_eq!(result.cluster_positions.len(), 1);
        let cluster = &result.cluster_positions[0];
        let node = result
            .node_positions
            .first()
            .expect("cluster member node should be laid out");
        assert_eq!(cluster.id, "pkg");
        assert!(cluster.x <= node.x);
        assert!(cluster.y <= node.y);
        assert!(cluster.x + cluster.width >= node.x + node.width);
        assert!(cluster.y + cluster.height >= node.y + node.height);
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

    #[test]
    fn record_label_sanitizes_port_names() {
        assert_eq!(
            record_label(&[
                "P0".to_string(),
                "row:1".to_string(),
                "bad|port".to_string()
            ]),
            "<P0> |<row1> |<badport> "
        );
        assert_eq!(record_label(&[]), " ");
    }

    #[test]
    fn record_node_tail_port_routes_edge() {
        let mut g = LayoutGraph::new(Direction::LeftToRight);
        assert!(g.add_record_node("record", 90.0, 90.0, &["P0".to_string(), "P1".to_string()]));
        g.add_node("child", "Child", 60.0, 40.0);
        g.add_edge_with_ports("record", "child", None, Some("P1"), None);

        let result = g.layout_full_no_timeout();
        assert_eq!(result.node_positions.len(), 2);
        assert_eq!(result.edge_paths.len(), 1);
        assert!(
            !result.edge_paths[0].points.is_empty(),
            "record tail port should still produce a routed spline"
        );
    }

    #[test]
    fn spacing_pixels_set_dot_rank_gap() {
        const NODE_W: f64 = 100.0;
        const NODE_H: f64 = 40.0;
        const RANK_SEP_PX: f64 = 144.0;

        let mut default = LayoutGraph::new(Direction::TopToBottom);
        default.add_node("a", "A", NODE_W, NODE_H);
        default.add_node("b", "B", NODE_W, NODE_H);
        default.add_edge("a", "b", None);
        let default_positions = default.layout_positions_no_timeout();

        let mut spaced =
            LayoutGraph::new(Direction::TopToBottom).with_spacing_pixels(35.0, RANK_SEP_PX);
        spaced.add_node("a", "A", NODE_W, NODE_H);
        spaced.add_node("b", "B", NODE_W, NODE_H);
        spaced.add_edge("a", "b", None);
        let spaced_positions = spaced.layout_positions_no_timeout();

        let default_gap =
            default_positions[1].y - default_positions[0].y - default_positions[0].height;
        let spaced_gap = spaced_positions[1].y - spaced_positions[0].y - spaced_positions[0].height;
        assert!(
            spaced_gap > default_gap,
            "spacing should increase rank gap: default={default_gap}, spaced={spaced_gap}"
        );
        assert!(
            (spaced_gap - RANK_SEP_PX).abs() <= 1.0,
            "ranksep should be applied in pixels: expected {RANK_SEP_PX}, got {spaced_gap}"
        );
    }
}
