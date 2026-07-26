// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! PlantUML-oriented graph layout API.
//!
//! Uses vendored Graphviz (dot algorithm) for hierarchical layout with
//! proper edge routing via cubic bezier splines.

use std::collections::{HashMap, HashSet};
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

/// Pixel dimensions of a renderer-owned SVEK cluster title placeholder.
#[derive(Clone, Copy, Debug)]
pub struct ClusterTitleSize {
    pub width: f64,
    pub height: f64,
}

/// One named row in a renderer-owned fixed HTML-table node.
#[derive(Clone, Debug)]
pub struct HtmlRowPort {
    pub id: String,
    pub position: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EdgePorts<'a> {
    pub tail: Option<&'a str>,
    pub head: Option<&'a str>,
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
    plantuml_svek_node_order: bool,
    plantuml_svek_inverted_starts: Vec<String>,
    plantuml_svek_line0_nodes: Vec<String>,
    plantuml_svek_line0_edges: Vec<(String, String)>,
    nodes: Vec<NodeSpec>,
    clusters: Vec<ClusterSpec>,
    together: Vec<TogetherSpec>,
    edges: Vec<EdgeSpec>,
    same_rank_pairs: Vec<(String, String)>,
}

impl LayoutGraph {
    /// Creates a new layout graph with the given direction.
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            spacing: None,
            plantuml_svek_node_order: false,
            plantuml_svek_inverted_starts: Vec::new(),
            plantuml_svek_line0_nodes: Vec::new(),
            plantuml_svek_line0_edges: Vec::new(),
            nodes: Vec::new(),
            clusters: Vec::new(),
            together: Vec::new(),
            edges: Vec::new(),
            same_rank_pairs: Vec::new(),
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

    /// Uses the node order emitted by PlantUML's
    /// `DotStringFactory.createDotString`: root leaves first, then cluster
    /// trees with each cluster's direct leaves before its children.
    pub fn with_plantuml_svek_node_order(mut self) -> Self {
        self.plantuml_svek_node_order = true;
        self
    }

    /// Creates an inverted SVEK link's start node before ordinary nodes.
    ///
    /// PlantUML `Cluster.getNodesOrderedTop` inserts each inverted link start
    /// at the front of the dot node stream before
    /// `Cluster.getNodesOrderedWithoutTop` emits the remaining nodes.
    pub fn add_plantuml_svek_inverted_start(&mut self, node_id: &str) {
        if !self
            .plantuml_svek_inverted_starts
            .iter()
            .any(|existing| existing == node_id)
        {
            self.plantuml_svek_inverted_starts
                .insert(0, node_id.to_string());
        }
    }

    /// Creates endpoints of a length-one SVEK edge before ordinary nodes.
    ///
    /// PlantUML's `DotStringFactory.createDotString` emits `lines0` after
    /// `Cluster.getNodesOrderedTop` but before
    /// `Cluster.getNodesOrderedWithoutTop`. Graphviz therefore creates any
    /// not-yet-declared endpoints in length-one edge order.
    pub fn add_plantuml_svek_line0_edge(&mut self, from: &str, to: &str) {
        for node_id in [from, to] {
            if !self
                .plantuml_svek_line0_nodes
                .iter()
                .any(|existing| existing == node_id)
            {
                self.plantuml_svek_line0_nodes.push(node_id.to_string());
            }
        }
        self.plantuml_svek_line0_edges
            .push((from.to_string(), to.to_string()));
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

    /// Adds a SVEK node whose painted image sits inside an HTML-table shield.
    ///
    /// PlantUML `SvekNode.appendHtml` uses this shape for images whose labels
    /// are painted outside their Graphviz image bounds. Graphviz's
    /// `fixedsize=shape` keeps edges on the image boundary while the generated
    /// fixed-size label reserves the same surrounding layout room.
    pub fn add_svek_shielded_node(
        &mut self,
        id: &str,
        width: f64,
        height: f64,
        shield_x: f64,
        shield_y: f64,
    ) -> bool {
        if self.nodes.iter().any(|node| node.id == id) {
            return false;
        }
        self.nodes.push(NodeSpec {
            id: id.to_string(),
            width,
            height,
            shape: NodeShape::SvekShielded { shield_x, shield_y },
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

    /// Adds an ellipse-shaped node with independent width and height.
    pub fn add_ellipse_node(&mut self, id: &str, _label: &str, width: f64, height: f64) -> bool {
        if self.nodes.iter().any(|node| node.id == id) {
            return false;
        }
        self.nodes.push(NodeSpec {
            id: id.to_string(),
            width,
            height,
            shape: NodeShape::Ellipse,
        });
        true
    }

    /// Adds a diamond-shaped node.
    ///
    /// PlantUML's `SvekNode.appendShapeInternal` maps
    /// `ShapeType.DIAMOND` to Graphviz's native `shape=diamond`, so routed
    /// splines contact the diagonal boundary rather than its bounding box.
    pub fn add_diamond_node(&mut self, id: &str, _label: &str, width: f64, height: f64) -> bool {
        if self.nodes.iter().any(|node| node.id == id) {
            return false;
        }
        self.nodes.push(NodeSpec {
            id: id.to_string(),
            width,
            height,
            shape: NodeShape::Diamond,
        });
        true
    }

    /// Adds the tiny point node SVEK uses as a routable package endpoint.
    ///
    /// `ClusterDotString.printInternal` emits
    /// `shape=point,width=.01,label=""` when a link targets a group.
    pub fn add_svek_cluster_endpoint(&mut self, id: &str) -> bool {
        if self.nodes.iter().any(|node| node.id == id) {
            return false;
        }
        self.nodes.push(NodeSpec {
            id: id.to_string(),
            width: 0.72,
            height: 0.72,
            shape: NodeShape::Point,
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

    /// Adds a plaintext node backed by a fixed-size HTML table with named rows.
    ///
    /// PlantUML SVEK's `SvekNode.appendLabelHtmlSpecialForLink` uses this
    /// shape for entities whose links target individual members. Graphviz
    /// owns the table envelope and routes edges to the named row ports; the
    /// renderer still draws the visible entity.
    pub fn add_fixed_html_row_node(
        &mut self,
        id: &str,
        width: f64,
        height: f64,
        ports: &[HtmlRowPort],
    ) -> bool {
        if self.nodes.iter().any(|node| node.id == id) {
            return false;
        }
        self.nodes.push(NodeSpec {
            id: id.to_string(),
            width,
            height,
            shape: NodeShape::FixedHtmlRows {
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
            kind: ClusterKind::NativeLabel(label.to_string()),
            parent: parent.map(String::from),
            nodes: Vec::new(),
            has_svek_endpoint: false,
        });
        true
    }

    /// Adds a PlantUML SVEK cluster with renderer-measured title geometry.
    ///
    /// `ClusterDotString.printInternal` wraps the real cluster in an unlabeled
    /// `p0` cluster and wraps its contents in an unlabeled `p1` cluster. The
    /// real title is represented to dot by an integer-sized fixed HTML table;
    /// the renderer draws the package title and chrome from the solved box.
    pub fn add_svek_cluster(
        &mut self,
        id: &str,
        parent: Option<&str>,
        title_size: ClusterTitleSize,
    ) -> bool {
        if self.clusters.iter().any(|c| c.id == id) {
            return false;
        }
        self.clusters.push(ClusterSpec {
            id: id.to_string(),
            kind: ClusterKind::Svek { title_size },
            parent: parent.map(String::from),
            nodes: Vec::new(),
            has_svek_endpoint: false,
        });
        true
    }

    /// Adds a node to an existing cluster/subgraph.
    pub fn add_cluster_node(&mut self, cluster_id: &str, node_id: &str) {
        if let Some(cluster) = self.clusters.iter_mut().find(|c| c.id == cluster_id) {
            cluster.nodes.push(node_id.to_string());
            if self
                .nodes
                .iter()
                .any(|node| node.id == node_id && matches!(node.shape, NodeShape::Point))
            {
                cluster.has_svek_endpoint = true;
            }
        }
    }

    /// Adds an invisible PlantUML `Together` subgraph.
    ///
    /// Java `Cluster.printTogether` emits a dot subgraph named from
    /// `getClusterId() + "t" + counter`, so Graphviz applies cluster margins.
    /// PlantUML uses those solved margins but does not paint separate chrome.
    pub fn add_together(&mut self, id: &str, cluster: Option<&str>, parent: Option<&str>) -> bool {
        if self.together.iter().any(|group| group.id == id) {
            return false;
        }
        self.together.push(TogetherSpec {
            id: id.to_string(),
            cluster: cluster.map(String::from),
            parent: parent.map(String::from),
            nodes: Vec::new(),
            clusters: Vec::new(),
        });
        true
    }

    /// Adds a leaf node to an invisible Together subgraph.
    pub fn add_together_node(&mut self, together_id: &str, node_id: &str) {
        if let Some(group) = self
            .together
            .iter_mut()
            .find(|group| group.id == together_id)
        {
            group.nodes.push(node_id.to_string());
        }
    }

    /// Adds a visible child cluster to an invisible Together subgraph.
    pub fn add_together_cluster(&mut self, together_id: &str, cluster_id: &str) {
        if let Some(group) = self
            .together
            .iter_mut()
            .find(|group| group.id == together_id)
        {
            group.clusters.push(cluster_id.to_string());
        }
    }

    /// Adds an edge between two nodes by their ids.
    pub fn add_edge(&mut self, from: &str, to: &str, label: Option<&str>) {
        self.add_edge_with_ports(from, to, label, None, None);
    }

    /// Adds an edge with an explicit Graphviz rank length.
    ///
    /// PlantUML `SvekEdge.appendLine` maps `Link.getLength() - 1` to dot's
    /// `minlen` attribute for non-horizontal links.
    pub fn add_edge_with_minlen(
        &mut self,
        from: &str,
        to: &str,
        label: Option<&str>,
        minlen: usize,
    ) {
        self.edges.push(EdgeSpec {
            from: from.to_string(),
            to: to.to_string(),
            label: label.map(String::from),
            tail_port: None,
            head_port: None,
            minlen: Some(minlen),
            label_size: None,
            tail_label_size: None,
            head_label_size: None,
        });
    }

    /// Constrains two nodes to the same rank, preserving their insertion order.
    pub fn add_same_rank(&mut self, first: &str, second: &str) {
        self.same_rank_pairs
            .push((first.to_string(), second.to_string()));
    }

    fn same_rank_owner(&self, first: &str, second: &str) -> SameRankOwner<'_> {
        if let Some(cluster) = self.clusters.iter().find(|cluster| {
            cluster.nodes.iter().any(|node| node == first)
                && cluster.nodes.iter().any(|node| node == second)
        }) {
            return SameRankOwner::Cluster(cluster.id.as_str());
        }

        let first_is_clustered = self
            .clusters
            .iter()
            .any(|cluster| cluster.nodes.iter().any(|node| node == first));
        let second_is_clustered = self
            .clusters
            .iter()
            .any(|cluster| cluster.nodes.iter().any(|node| node == second));
        if first_is_clustered || second_is_clustered {
            SameRankOwner::None
        } else {
            SameRankOwner::Root
        }
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
            minlen: None,
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
        self.add_edge_with_label_sizes_and_minlen(
            from,
            to,
            label_size,
            tail_label_size,
            head_label_size,
            None,
        );
    }

    /// Adds a measured-label edge bound to optional named node ports.
    pub fn add_edge_with_ports_and_label_sizes(
        &mut self,
        from: &str,
        to: &str,
        ports: EdgePorts<'_>,
        label_size: Option<EdgeLabelSize>,
        tail_label_size: Option<EdgeLabelSize>,
        head_label_size: Option<EdgeLabelSize>,
    ) {
        self.edges.push(EdgeSpec {
            from: from.to_string(),
            to: to.to_string(),
            label: None,
            tail_port: ports.tail.map(String::from),
            head_port: ports.head.map(String::from),
            minlen: None,
            label_size,
            tail_label_size,
            head_label_size,
        });
    }

    /// Adds a measured-label edge with an optional explicit dot rank length.
    pub fn add_edge_with_label_sizes_and_minlen(
        &mut self,
        from: &str,
        to: &str,
        label_size: Option<EdgeLabelSize>,
        tail_label_size: Option<EdgeLabelSize>,
        head_label_size: Option<EdgeLabelSize>,
        minlen: Option<usize>,
    ) {
        self.edges.push(EdgeSpec {
            from: from.to_string(),
            to: to.to_string(),
            label: None,
            tail_port: None,
            head_port: None,
            minlen,
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
        for (key, value) in [("remincross", "true"), ("searchsize", "500")] {
            let key = CString::new(key).unwrap();
            let value = CString::new(value).unwrap();
            graphviz_ffi::agsafeset(
                g as *mut c_void,
                key.as_ptr(),
                value.as_ptr(),
                empty.as_ptr(),
            );
        }

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

        // Opted-in SVEK clients follow `DotStringFactory.createDotString`,
        // which serialises the root cluster's direct leaves before child
        // clusters (`Cluster.printCluster2`). `GraphvizImageBuilder` collects
        // package members first, but that Bibliotekon insertion order is not
        // the order Graphviz receives. Preserve the selected creation order
        // while returning positions in the caller's original node order.
        let mut node_handles: HashMap<String, *mut graphviz_ffi::Agnode_t> = HashMap::new();
        let node_order: Vec<String> = self.nodes.iter().map(|spec| spec.id.clone()).collect();

        let width_key = CString::new("width").unwrap();
        let height_key = CString::new("height").unwrap();
        let shape_key = CString::new("shape").unwrap();
        let label_key = CString::new("label").unwrap();
        let fixedsize_key = CString::new("fixedsize").unwrap();
        let no_label_val = CString::new("").unwrap();
        let fixedsize_val = CString::new("true").unwrap();
        let circle_val = CString::new("circle").unwrap();
        let ellipse_val = CString::new("ellipse").unwrap();
        let diamond_val = CString::new("diamond").unwrap();
        let box_val = CString::new("box").unwrap();
        let point_val = CString::new("point").unwrap();
        let record_val = CString::new("record").unwrap();
        let plaintext_val = CString::new("plaintext").unwrap();
        let shape_fixedsize_val = CString::new("shape").unwrap();
        let arrowhead_key = CString::new("arrowhead").unwrap();
        let arrowtail_key = CString::new("arrowtail").unwrap();
        let headport_key = CString::new("headport").unwrap();
        let tailport_key = CString::new("tailport").unwrap();
        let minlen_key = CString::new("minlen").unwrap();
        let headlabel_key = CString::new("headlabel").unwrap();
        let taillabel_key = CString::new("taillabel").unwrap();
        let external_endpoint_labels_key =
            CString::new("rustuml_external_endpoint_labels").unwrap();
        let true_val = CString::new("true").unwrap();
        let no_arrow_val = CString::new("none").unwrap();

        macro_rules! create_node {
            ($node_idx:expr) => {{
                let spec = &self.nodes[$node_idx];
                let cid = CString::new(spec.id.as_str()).unwrap();
                let node = graphviz_ffi::agnode(g, cid.as_ptr(), 1);

                if let NodeShape::SvekShielded { shield_x, shield_y } = spec.shape {
                    let w_inches = spec.width / DOT_POINTS_PER_INCH;
                    let h_inches = spec.height / DOT_POINTS_PER_INCH;
                    let w_str = CString::new(format!("{w_inches:.6}")).unwrap();
                    let h_str = CString::new(format!("{h_inches:.6}")).unwrap();
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
                        shape_fixedsize_val.as_ptr(),
                        empty.as_ptr(),
                    );
                    graphviz_ffi::agsafeset(
                        node as *mut c_void,
                        shape_key.as_ptr(),
                        box_val.as_ptr(),
                        empty.as_ptr(),
                    );
                    let table =
                        CString::new(svek_shielded_node_table(spec, shield_x, shield_y)).unwrap();
                    graphviz_ffi::agsafeset_html(
                        node as *mut c_void,
                        label_key.as_ptr(),
                        table.as_ptr(),
                        empty.as_ptr(),
                    );
                } else {
                    let shape = match &spec.shape {
                        NodeShape::Box => &box_val,
                        NodeShape::Circle => &circle_val,
                        NodeShape::Ellipse => &ellipse_val,
                        NodeShape::Diamond => &diamond_val,
                        NodeShape::Point => &point_val,
                        NodeShape::Record { .. } => &record_val,
                        NodeShape::FixedHtmlRows { .. } => &plaintext_val,
                        NodeShape::SvekShielded { .. } => unreachable!(),
                    };
                    graphviz_ffi::agsafeset(
                        node as *mut c_void,
                        shape_key.as_ptr(),
                        shape.as_ptr(),
                        empty.as_ptr(),
                    );
                    if let NodeShape::FixedHtmlRows { ports } = &spec.shape {
                        let table =
                            CString::new(fixed_html_row_table(spec.width, spec.height, ports))
                                .unwrap();
                        graphviz_ffi::agsafeset_html(
                            node as *mut c_void,
                            label_key.as_ptr(),
                            table.as_ptr(),
                            empty.as_ptr(),
                        );
                    } else {
                        // Graphviz uses inches for width/height.
                        let w_inches = spec.width / DOT_POINTS_PER_INCH;
                        let h_inches = spec.height / DOT_POINTS_PER_INCH;
                        let w_str = CString::new(format!("{w_inches:.6}")).unwrap();
                        let h_str = CString::new(format!("{h_inches:.6}")).unwrap();

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
                        // PlantUML SVEK uses dot for geometry and renders
                        // entity labels itself.
                        let label_val = match &spec.shape {
                            NodeShape::Record { ports } => {
                                CString::new(record_label(ports)).unwrap()
                            }
                            NodeShape::Box
                            | NodeShape::Circle
                            | NodeShape::Ellipse
                            | NodeShape::Diamond
                            | NodeShape::Point => no_label_val.clone(),
                            NodeShape::FixedHtmlRows { .. } | NodeShape::SvekShielded { .. } => {
                                unreachable!()
                            }
                        };
                        graphviz_ffi::agsafeset(
                            node as *mut c_void,
                            label_key.as_ptr(),
                            label_val.as_ptr(),
                            empty.as_ptr(),
                        );
                    }
                }

                node_handles.insert(spec.id.clone(), node);
            }};
        }

        let graphviz_node_order = self.graphviz_node_creation_order();
        let early_node_count = graphviz_node_order
            .iter()
            .take_while(|&&node_idx| {
                let id = &self.nodes[node_idx].id;
                self.plantuml_svek_inverted_starts.contains(id)
                    || self.plantuml_svek_line0_nodes.contains(id)
            })
            .count();
        for &node_idx in &graphviz_node_order[..early_node_count] {
            create_node!(node_idx);
        }

        let mut edge_specs: HashMap<usize, (String, String)> = HashMap::new();
        macro_rules! create_edge {
            ($edge_idx:expr) => {{
                let edge_spec = &self.edges[$edge_idx];
                if let (Some(&from_h), Some(&to_h)) = (
                    node_handles.get(&edge_spec.from),
                    node_handles.get(&edge_spec.to),
                ) {
                    // PlantUML emits every relationship as a separate dot
                    // edge statement, so preserve caller-owned identity.
                    let edge_name = CString::new(format!(
                        "{}__{}__{}",
                        edge_spec.from, edge_spec.to, $edge_idx
                    ))
                    .unwrap();
                    let edge = graphviz_ffi::agedge(g, from_h, to_h, edge_name.as_ptr(), 1);

                    if let Some(lbl) = &edge_spec.label {
                        let label_val = CString::new(lbl.as_str()).unwrap();
                        graphviz_ffi::agsafeset(
                            edge as *mut c_void,
                            label_key.as_ptr(),
                            label_val.as_ptr(),
                            empty.as_ptr(),
                        );
                    }
                    for (edge_label_key, color, size) in [
                        (&label_key, "#000001", edge_spec.label_size),
                        (&taillabel_key, "#000002", edge_spec.tail_label_size),
                        (&headlabel_key, "#000003", edge_spec.head_label_size),
                    ] {
                        let Some(size) = size else { continue };
                        let table = CString::new(edge_label_table(size, color)).unwrap();
                        graphviz_ffi::agsafeset_html(
                            edge as *mut c_void,
                            edge_label_key.as_ptr(),
                            table.as_ptr(),
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
                    if let Some(minlen) = edge_spec.minlen {
                        let minlen_value = CString::new(minlen.to_string()).unwrap();
                        graphviz_ffi::agsafeset(
                            edge as *mut c_void,
                            minlen_key.as_ptr(),
                            minlen_value.as_ptr(),
                            empty.as_ptr(),
                        );
                    }
                    // SVEK renders link decorations itself after dot routes
                    // the undecorated spline.
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

                    edge_specs.insert(
                        edge as usize,
                        (edge_spec.from.clone(), edge_spec.to.clone()),
                    );
                }
            }};
        }

        let mut early_edge_indices = Vec::new();
        let mut claimed_edges = vec![false; self.edges.len()];
        for (from, to) in &self.plantuml_svek_line0_edges {
            if let Some((edge_idx, (_, claimed))) =
                self.edges.iter().zip(&mut claimed_edges).enumerate().find(
                    |(_, (edge, claimed))| {
                        !**claimed && edge.from == *from && edge.to == *to && edge.minlen == Some(0)
                    },
                )
            {
                *claimed = true;
                early_edge_indices.push(edge_idx);
            }
        }
        for &edge_idx in &early_edge_indices {
            create_edge!(edge_idx);
        }
        for &node_idx in &graphviz_node_order[early_node_count..] {
            create_node!(node_idx);
        }

        let rank_key = CString::new("rank").unwrap();
        let same_val = CString::new("same").unwrap();
        for (idx, (first, second)) in self.same_rank_pairs.iter().enumerate() {
            if !matches!(self.same_rank_owner(first, second), SameRankOwner::Root) {
                continue;
            }
            let name = CString::new(format!("same_rank_{idx}")).unwrap();
            let subgraph = graphviz_ffi::agsubg(g, name.as_ptr() as *mut _, 1);
            graphviz_ffi::agsafeset(
                subgraph as *mut c_void,
                rank_key.as_ptr(),
                same_val.as_ptr(),
                empty.as_ptr(),
            );
            for id in [first, second] {
                if let Some(&node) = node_handles.get(id) {
                    graphviz_ffi::agsubnode(subgraph, node, 1);
                }
            }
        }

        let mut cluster_handles: HashMap<String, *mut graphviz_ffi::Agraph_t> = HashMap::new();
        let mut together_handles: HashMap<String, *mut graphviz_ffi::Agraph_t> = HashMap::new();
        self.build_together_trees(None, g, &node_handles, &mut together_handles);
        let mut cluster_order: Vec<String> = Vec::new();
        let mut built_clusters = vec![false; self.clusters.len()];
        for idx in 0..self.clusters.len() {
            let parent_is_known = self.clusters[idx]
                .parent
                .as_ref()
                .is_some_and(|parent| self.clusters.iter().any(|cluster| &cluster.id == parent));
            if !parent_is_known {
                let parent = self
                    .together_parent_for_cluster(&self.clusters[idx].id)
                    .and_then(|group| together_handles.get(group))
                    .copied()
                    .unwrap_or(g);
                self.build_cluster_tree(
                    idx,
                    parent,
                    &node_handles,
                    &label_key,
                    &empty,
                    &mut cluster_handles,
                    &mut together_handles,
                    &mut cluster_order,
                    &mut built_clusters,
                );
            }
        }
        for idx in 0..self.clusters.len() {
            if !built_clusters[idx] {
                self.build_cluster_tree(
                    idx,
                    g,
                    &node_handles,
                    &label_key,
                    &empty,
                    &mut cluster_handles,
                    &mut together_handles,
                    &mut cluster_order,
                    &mut built_clusters,
                );
            }
        }

        // PlantUML emits the remaining `lines1` edges after ordinary nodes.
        for (edge_idx, claimed) in claimed_edges.iter().enumerate() {
            if !claimed {
                create_edge!(edge_idx);
            }
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

        // Walk edges in graph order and recover each caller-owned identity
        // from the cgraph edge pointer returned by `agedge`.
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

                let (from, to) = edge_specs
                    .get(&(e as usize))
                    .cloned()
                    .unwrap_or_else(|| (String::new(), String::new()));

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

                e = graphviz_ffi::agnxtout(g, e);
            }
            n = graphviz_ffi::agnxtnode(g, n);
        }

        let mut cluster_positions = Vec::with_capacity(cluster_order.len());
        let mut cluster_serialized_sizes = HashMap::with_capacity(cluster_order.len());
        for id in &cluster_order {
            let cluster = cluster_handles[id];
            let mut ll_x: f64 = 0.0;
            let mut ll_y: f64 = 0.0;
            let mut ur_x: f64 = 0.0;
            let mut ur_y: f64 = 0.0;
            graphviz_ffi::rustuml_graph_bb(cluster, &mut ll_x, &mut ll_y, &mut ur_x, &mut ur_y);
            // Java `DotStringFactory.solve` reconstructs cluster rectangles
            // from Graphviz's two-decimal SVG polygon, not its internal
            // floating-point bounding box.
            let serialized = |value: f64| (value * 100.0).round() / 100.0;
            cluster_serialized_sizes.insert(
                id.clone(),
                (
                    serialized(ur_x) - serialized(ll_x),
                    serialized(ur_y) - serialized(ll_y),
                ),
            );
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
            cluster_serialized_sizes,
            edge_paths,
            width: graph_ur_x - graph_ll_x,
            height: graph_ur_y - graph_ll_y,
            svg_y_origin: max_y,
        }
    }

    fn graphviz_node_creation_order(&self) -> Vec<usize> {
        let has_svek_clusters = self
            .clusters
            .iter()
            .any(|cluster| matches!(cluster.kind, ClusterKind::Svek { .. }));

        fn visit(
            cluster_idx: usize,
            clusters: &[ClusterSpec],
            nodes: &[NodeSpec],
            seen: &mut [bool],
            order: &mut Vec<usize>,
        ) {
            let cluster = &clusters[cluster_idx];
            for node_id in &cluster.nodes {
                if let Some(node_idx) = nodes.iter().position(|node| &node.id == node_id)
                    && !seen[node_idx]
                {
                    seen[node_idx] = true;
                    order.push(node_idx);
                }
            }
            for (child_idx, child) in clusters.iter().enumerate() {
                if child.parent.as_deref() == Some(cluster.id.as_str()) {
                    visit(child_idx, clusters, nodes, seen, order);
                }
            }
        }

        let mut seen = vec![false; self.nodes.len()];
        let mut order = Vec::with_capacity(self.nodes.len());
        for node_id in &self.plantuml_svek_inverted_starts {
            if let Some(node_idx) = self.nodes.iter().position(|node| &node.id == node_id)
                && !seen[node_idx]
            {
                seen[node_idx] = true;
                order.push(node_idx);
            }
        }
        for node_id in &self.plantuml_svek_line0_nodes {
            if let Some(node_idx) = self.nodes.iter().position(|node| &node.id == node_id)
                && !seen[node_idx]
            {
                seen[node_idx] = true;
                order.push(node_idx);
            }
        }
        if self.plantuml_svek_node_order && has_svek_clusters {
            let clustered_nodes: HashSet<&str> = self
                .clusters
                .iter()
                .flat_map(|cluster| cluster.nodes.iter().map(String::as_str))
                .collect();
            for (idx, node) in self.nodes.iter().enumerate() {
                if !clustered_nodes.contains(node.id.as_str()) {
                    seen[idx] = true;
                    order.push(idx);
                }
            }
        }
        if has_svek_clusters {
            for (idx, cluster) in self.clusters.iter().enumerate() {
                let parent_is_known = cluster
                    .parent
                    .as_ref()
                    .is_some_and(|parent| self.clusters.iter().any(|item| &item.id == parent));
                if !parent_is_known {
                    visit(idx, &self.clusters, &self.nodes, &mut seen, &mut order);
                }
            }
        }
        for (idx, was_seen) in seen.iter().enumerate() {
            if !was_seen {
                order.push(idx);
            }
        }
        order
    }

    #[allow(clippy::too_many_arguments, unsafe_op_in_unsafe_fn)]
    unsafe fn build_cluster_tree(
        &self,
        cluster_idx: usize,
        parent: *mut graphviz_ffi::Agraph_t,
        node_handles: &HashMap<String, *mut graphviz_ffi::Agnode_t>,
        label_key: &CString,
        empty: &CString,
        cluster_handles: &mut HashMap<String, *mut graphviz_ffi::Agraph_t>,
        together_handles: &mut HashMap<String, *mut graphviz_ffi::Agraph_t>,
        cluster_order: &mut Vec<String>,
        built: &mut [bool],
    ) {
        if built[cluster_idx] {
            return;
        }
        built[cluster_idx] = true;
        let cluster = &self.clusters[cluster_idx];

        let (real_cluster, member_parent) = match &cluster.kind {
            ClusterKind::NativeLabel(label) => {
                let name = CString::new(format!("cluster_{}", cluster.id)).unwrap();
                let real = graphviz_ffi::agsubg(parent, name.as_ptr() as *mut _, 1);
                let label = CString::new(label.as_str()).unwrap();
                graphviz_ffi::agsafeset(
                    real as *mut c_void,
                    label_key.as_ptr(),
                    label.as_ptr(),
                    empty.as_ptr(),
                );
                (real, real)
            }
            ClusterKind::Svek { title_size } => {
                let cluster_parent = if cluster.has_svek_endpoint {
                    let protection_name = CString::new(format!("cluster_{}a", cluster.id)).unwrap();
                    let protection =
                        graphviz_ffi::agsubg(parent, protection_name.as_ptr() as *mut _, 1);
                    let no_label = CString::new("").unwrap();
                    graphviz_ffi::agsafeset(
                        protection as *mut c_void,
                        label_key.as_ptr(),
                        no_label.as_ptr(),
                        empty.as_ptr(),
                    );
                    protection
                } else {
                    parent
                };
                let p0_name = CString::new(format!("cluster_{}p0", cluster.id)).unwrap();
                let p0 = graphviz_ffi::agsubg(cluster_parent, p0_name.as_ptr() as *mut _, 1);
                let no_label = CString::new("").unwrap();
                graphviz_ffi::agsafeset(
                    p0 as *mut c_void,
                    label_key.as_ptr(),
                    no_label.as_ptr(),
                    empty.as_ptr(),
                );

                let name = CString::new(format!("cluster_{}", cluster.id)).unwrap();
                let real = graphviz_ffi::agsubg(p0, name.as_ptr() as *mut _, 1);
                for (key, value) in [("style", "solid"), ("color", "#000004"), ("labeljust", "l")] {
                    let key = CString::new(key).unwrap();
                    let value = CString::new(value).unwrap();
                    graphviz_ffi::agsafeset(
                        real as *mut c_void,
                        key.as_ptr(),
                        value.as_ptr(),
                        empty.as_ptr(),
                    );
                }
                let table = CString::new(cluster_title_table(*title_size)).unwrap();
                graphviz_ffi::agsafeset_html(
                    real as *mut c_void,
                    label_key.as_ptr(),
                    table.as_ptr(),
                    empty.as_ptr(),
                );

                let member_parent = if cluster.has_svek_endpoint {
                    let protection_name = CString::new(format!("cluster_{}i", cluster.id)).unwrap();
                    let protection =
                        graphviz_ffi::agsubg(real, protection_name.as_ptr() as *mut _, 1);
                    graphviz_ffi::agsafeset(
                        protection as *mut c_void,
                        label_key.as_ptr(),
                        no_label.as_ptr(),
                        empty.as_ptr(),
                    );
                    protection
                } else {
                    real
                };
                let p1_name = CString::new(format!("cluster_{}p1", cluster.id)).unwrap();
                let p1 = graphviz_ffi::agsubg(member_parent, p1_name.as_ptr() as *mut _, 1);
                graphviz_ffi::agsafeset(
                    p1 as *mut c_void,
                    label_key.as_ptr(),
                    no_label.as_ptr(),
                    empty.as_ptr(),
                );
                (real, p1)
            }
        };

        cluster_handles.insert(cluster.id.clone(), real_cluster);
        cluster_order.push(cluster.id.clone());
        self.build_together_trees(
            Some(cluster.id.as_str()),
            member_parent,
            node_handles,
            together_handles,
        );
        for node_id in &cluster.nodes {
            if let Some(&node) = node_handles.get(node_id) {
                let node_parent = if cluster.has_svek_endpoint
                    && self
                        .nodes
                        .iter()
                        .any(|spec| spec.id == *node_id && matches!(spec.shape, NodeShape::Point))
                {
                    // Java emits the package special point directly in the
                    // real cluster, before opening the inner `i`/`p1`
                    // protection wrappers around ordinary members.
                    real_cluster
                } else {
                    member_parent
                };
                graphviz_ffi::agsubnode(node_parent, node, 1);
            }
        }
        // PlantUML `Cluster.appendRankSame` emits horizontal-link rank
        // subgraphs inside the cluster's `p1` protection wrapper. Keeping
        // these at the root makes Graphviz treat the members as external and
        // collapses a single container's solved bounds.
        for (idx, (first, second)) in self.same_rank_pairs.iter().enumerate() {
            if !matches!(
                self.same_rank_owner(first, second),
                SameRankOwner::Cluster(owner) if owner == cluster.id.as_str()
            ) {
                continue;
            }
            let name = CString::new(format!("same_rank_{idx}")).unwrap();
            let subgraph = graphviz_ffi::agsubg(member_parent, name.as_ptr() as *mut _, 1);
            let rank_key = CString::new("rank").unwrap();
            let same_val = CString::new("same").unwrap();
            graphviz_ffi::agsafeset(
                subgraph as *mut c_void,
                rank_key.as_ptr(),
                same_val.as_ptr(),
                empty.as_ptr(),
            );
            for id in [first, second] {
                if let Some(&node) = node_handles.get(id) {
                    graphviz_ffi::agsubnode(subgraph, node, 1);
                }
            }
        }
        for (child_idx, child) in self.clusters.iter().enumerate() {
            if child.parent.as_deref() == Some(cluster.id.as_str()) {
                let child_parent = self
                    .together_parent_for_cluster(&child.id)
                    .and_then(|group| together_handles.get(group))
                    .copied()
                    .unwrap_or(member_parent);
                self.build_cluster_tree(
                    child_idx,
                    child_parent,
                    node_handles,
                    label_key,
                    empty,
                    cluster_handles,
                    together_handles,
                    cluster_order,
                    built,
                );
            }
        }
    }

    fn together_parent_for_cluster(&self, cluster_id: &str) -> Option<&str> {
        self.together
            .iter()
            .find(|group| group.clusters.iter().any(|member| member == cluster_id))
            .map(|group| group.id.as_str())
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn build_together_trees(
        &self,
        owner_cluster: Option<&str>,
        parent_graph: *mut graphviz_ffi::Agraph_t,
        node_handles: &HashMap<String, *mut graphviz_ffi::Agnode_t>,
        handles: &mut HashMap<String, *mut graphviz_ffi::Agraph_t>,
    ) {
        for group in &self.together {
            if group.cluster.as_deref() == owner_cluster && group.parent.is_none() {
                self.build_together_tree(group, parent_graph, node_handles, handles);
            }
        }
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn build_together_tree(
        &self,
        group: &TogetherSpec,
        parent_graph: *mut graphviz_ffi::Agraph_t,
        node_handles: &HashMap<String, *mut graphviz_ffi::Agnode_t>,
        handles: &mut HashMap<String, *mut graphviz_ffi::Agraph_t>,
    ) {
        // The `cluster` prefix is behavioral: dot only applies the enclosing
        // margin that PlantUML relies on when the subgraph name has this prefix.
        let name = CString::new(format!("cluster_together_{}", group.id)).unwrap();
        let graph = graphviz_ffi::agsubg(parent_graph, name.as_ptr() as *mut _, 1);
        handles.insert(group.id.clone(), graph);
        for node_id in &group.nodes {
            if let Some(&node) = node_handles.get(node_id) {
                graphviz_ffi::agsubnode(graph, node, 1);
            }
        }
        for child in &self.together {
            if child.parent.as_deref() == Some(group.id.as_str()) {
                self.build_together_tree(child, graph, node_handles, handles);
            }
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

/// Faithful serialization of PlantUML
/// `SvekNode.appendLabelHtmlSpecialForLink` and `appendTr`.
fn fixed_html_row_table(width: f64, height: f64, ports: &[HtmlRowPort]) -> String {
    let mut sorted = ports.to_vec();
    sorted.sort_by(|left, right| left.position.total_cmp(&right.position));

    let mut table = String::from(
        r##"<TABLE BGCOLOR="#000001" BORDER="0" CELLBORDER="0" CELLSPACING="0" CELLPADDING="0">"##,
    );
    let mut sum = 0_i64;
    for port in sorted {
        let missing = (port.position - sum as f64) as i64;
        append_fixed_html_row(&mut table, width, missing, None);
        sum += missing;

        let row_height = port.height as i64;
        append_fixed_html_row(&mut table, width, row_height, Some(&port.id));
        sum += row_height;
    }
    append_fixed_html_row(&mut table, width, (height - sum as f64) as i64, None);
    table.push_str("</TABLE>");
    table
}

fn append_fixed_html_row(table: &mut String, width: f64, height: i64, port: Option<&str>) {
    if height <= 0 {
        return;
    }
    table.push_str("<TR><TD FIXEDSIZE=\"TRUE\" WIDTH=\"");
    table.push_str(&width.to_string());
    table.push_str("\" HEIGHT=\"");
    table.push_str(&height.to_string());
    if let Some(port) = port {
        table.push_str("\" PORT=\"");
        table.push_str(port);
    }
    table.push_str("\"></TD></TR>");
}

fn svek_shielded_node_table(spec: &NodeSpec, shield_x: f64, shield_y: f64) -> String {
    // Graphviz's HTML parser reads each fixed cell dimension as an integer.
    // Java `SvekNode.appendHtml` emits two shield cells around the image cell.
    let width = spec.width + shield_x.floor() * 2.0;
    let height = spec.height + shield_y.floor() * 2.0;
    format!(
        r##"<TABLE BGCOLOR="#000005" FIXEDSIZE="TRUE" WIDTH="{width}" HEIGHT="{height}"><TR><TD></TD></TR></TABLE>"##,
    )
}

fn escape_record_port(port: &str) -> String {
    port.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

fn dot_inches(pixel: f64) -> String {
    format!("{:.6}", pixel / DOT_POINTS_PER_INCH)
}

/// PlantUML `SvekEdge.appendTable` passes integer-truncated renderer dimensions
/// to dot as a fixed-size HTML table. The unique fill is how Java later locates
/// the solved table origin in Graphviz's SVG; Rust reads the solved box directly.
fn edge_label_table(size: EdgeLabelSize, color: &str) -> String {
    let width = size.width.max(1.0) as u64;
    let height = size.height.max(1.0) as u64;
    format!(
        r#"<TABLE BGCOLOR="{color}" FIXEDSIZE="TRUE" WIDTH="{width}" HEIGHT="{height}"><TR><TD></TD></TR></TABLE>"#
    )
}

/// `ClusterHeader` truncates title dimensions to integers before
/// `ClusterDotString.printInternal` subtracts five pixels from the height and
/// sends the result through `SvekEdge.appendTable`.
fn cluster_title_table(size: ClusterTitleSize) -> String {
    let width = size.width.max(1.0) as u64;
    let height = (size.height as i64 - 5).max(1) as u64;
    format!(
        r##"<TABLE BGCOLOR="#000004" FIXEDSIZE="TRUE" WIDTH="{width}" HEIGHT="{height}"><TR><TD></TD></TR></TABLE>"##
    )
}

/// Full layout result with both node positions and edge routing.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub node_positions: Vec<NodePosition>,
    pub cluster_positions: Vec<ClusterPosition>,
    /// Cluster dimensions after Graphviz's two-decimal SVG serialization.
    pub cluster_serialized_sizes: HashMap<String, (f64, f64)>,
    pub edge_paths: Vec<EdgePath>,
    /// Full solved Graphviz envelope, including edge-label constraints.
    pub width: f64,
    pub height: f64,
    /// Y origin used to convert Graphviz's mathematical coordinates to the
    /// normalized screen coordinates above.
    pub svg_y_origin: f64,
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
    Ellipse,
    Diamond,
    Point,
    Record { ports: Vec<String> },
    FixedHtmlRows { ports: Vec<HtmlRowPort> },
    SvekShielded { shield_x: f64, shield_y: f64 },
}

#[derive(Debug, Clone)]
struct EdgeSpec {
    from: String,
    to: String,
    label: Option<String>,
    tail_port: Option<String>,
    head_port: Option<String>,
    minlen: Option<usize>,
    label_size: Option<EdgeLabelSize>,
    tail_label_size: Option<EdgeLabelSize>,
    head_label_size: Option<EdgeLabelSize>,
}

#[derive(Debug, Clone)]
struct ClusterSpec {
    id: String,
    kind: ClusterKind,
    parent: Option<String>,
    nodes: Vec<String>,
    has_svek_endpoint: bool,
}

enum SameRankOwner<'a> {
    Root,
    Cluster(&'a str),
    None,
}

#[derive(Debug, Clone)]
struct TogetherSpec {
    id: String,
    cluster: Option<String>,
    parent: Option<String>,
    nodes: Vec<String>,
    clusters: Vec<String>,
}

#[derive(Debug, Clone)]
enum ClusterKind {
    NativeLabel(String),
    Svek { title_size: ClusterTitleSize },
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
    fn edge_paths_keep_identity_when_graphviz_traversal_reorders_edges() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("source_a_17", "", 80.0, 40.0);
        g.add_node("source_b_23", "", 80.0, 40.0);
        g.add_node("target_a_29", "", 80.0, 40.0);
        g.add_node("target_b_31", "", 80.0, 40.0);
        g.add_edge("source_b_23", "target_b_31", None);
        g.add_edge("source_a_17", "target_a_29", None);

        let result = g.layout_full_no_timeout();
        let center = |node: &NodePosition| (node.x + node.width / 2.0, node.y + node.height / 2.0);
        let distance =
            |point: (f64, f64), node: (f64, f64)| (point.0 - node.0).hypot(point.1 - node.1);
        let source_a = center(&result.node_positions[0]);
        let source_b = center(&result.node_positions[1]);

        let edge_a = result
            .edge_paths
            .iter()
            .find(|edge| edge.from == "source_a_17" && edge.to == "target_a_29")
            .expect("renamed A edge");
        let edge_b = result
            .edge_paths
            .iter()
            .find(|edge| edge.from == "source_b_23" && edge.to == "target_b_31")
            .expect("renamed B edge");
        assert!(
            distance(edge_a.points[0], source_a) < distance(edge_a.points[0], source_b),
            "A spline should start at source A"
        );
        assert!(
            distance(edge_b.points[0], source_b) < distance(edge_b.points[0], source_a),
            "B spline should start at source B"
        );
    }

    #[test]
    fn parallel_renamed_edges_keep_distinct_splines_and_label_boxes() {
        let mut graph = LayoutGraph::new(Direction::TopToBottom);
        graph.add_node("renamed_source_47", "", 53.0, 41.0);
        graph.add_node("renamed_target_53", "", 61.0, 43.0);
        for width in [37.0, 43.0, 59.0] {
            graph.add_edge_with_label_sizes(
                "renamed_source_47",
                "renamed_target_53",
                Some(EdgeLabelSize {
                    width,
                    height: 17.0,
                }),
                None,
                None,
            );
        }

        let result = graph.layout_full_no_timeout();
        assert_eq!(result.edge_paths.len(), 3);
        let mut label_x = result
            .edge_paths
            .iter()
            .map(|edge| edge.label.expect("parallel label").x)
            .collect::<Vec<_>>();
        label_x.sort_by(f64::total_cmp);
        label_x.dedup();
        assert_eq!(label_x.len(), 3);
        let mut starts = result
            .edge_paths
            .iter()
            .map(|edge| edge.points.first().copied().expect("parallel spline"))
            .collect::<Vec<_>>();
        starts.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.total_cmp(&right.1))
        });
        starts.dedup();
        assert_eq!(starts.len(), 3);
    }

    #[test]
    fn edge_minlen_expands_rank_distance_for_renamed_nodes() {
        let mut short = LayoutGraph::new(Direction::TopToBottom);
        short.add_node("source_41", "", 80.0, 40.0);
        short.add_node("target_43", "", 80.0, 40.0);
        short.add_edge_with_minlen("source_41", "target_43", None, 1);
        let short_result = short.layout_full_no_timeout();

        let mut long = LayoutGraph::new(Direction::TopToBottom);
        long.add_node("source_41", "", 80.0, 40.0);
        long.add_node("target_43", "", 80.0, 40.0);
        long.add_edge_with_minlen("source_41", "target_43", None, 3);
        let long_result = long.layout_full_no_timeout();

        let short_gap = short_result.node_positions[1].y - short_result.node_positions[0].y;
        let long_gap = long_result.node_positions[1].y - long_result.node_positions[0].y;
        assert!(long_gap > short_gap);
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
    fn sized_edge_labels_use_integer_truncated_html_table_dimensions() {
        let mut graph = LayoutGraph::new(Direction::TopToBottom);
        graph.add_node("renamed_a", "A", 40.0, 48.0);
        graph.add_node("renamed_b", "B", 40.0, 48.0);
        graph.add_edge_with_label_sizes(
            "renamed_a",
            "renamed_b",
            Some(EdgeLabelSize {
                width: 25.9,
                height: 15.9,
            }),
            None,
            None,
        );

        let result = graph.layout_full_no_timeout();
        let label = result.edge_paths[0].label.unwrap();
        assert_eq!(label.width, 25.0);
        assert_eq!(label.height, 15.0);
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
    fn same_rank_constraint_keeps_renamed_nodes_horizontal() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("note_renamed", "", 83.0, 41.0);
        g.add_node("target_renamed", "Target", 117.0, 64.0);
        g.add_same_rank("note_renamed", "target_renamed");
        g.add_edge("note_renamed", "target_renamed", None);

        let result = g.layout_full_no_timeout();
        assert_eq!(result.node_positions.len(), 2);
        let first_center = result.node_positions[0].y + result.node_positions[0].height / 2.0;
        let second_center = result.node_positions[1].y + result.node_positions[1].height / 2.0;
        assert_eq!(first_center, second_center);
        assert!(result.node_positions[0].x < result.node_positions[1].x);
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
    fn together_subgraph_expands_owning_cluster_without_becoming_visible() {
        let mut plain = LayoutGraph::new(Direction::TopToBottom).with_plantuml_svek_spacing();
        let mut grouped = LayoutGraph::new(Direction::TopToBottom).with_plantuml_svek_spacing();
        for graph in [&mut plain, &mut grouped] {
            graph.add_node("renamed_alpha_17", "", 49.0, 46.0);
            graph.add_node("renamed_beta_23", "", 48.0, 46.0);
            graph.add_node("renamed_sink_29", "", 50.0, 46.0);
            graph.add_svek_cluster(
                "renamed_outer_31",
                None,
                ClusterTitleSize {
                    width: 42.0,
                    height: 17.0,
                },
            );
            for node in ["renamed_alpha_17", "renamed_beta_23", "renamed_sink_29"] {
                graph.add_cluster_node("renamed_outer_31", node);
            }
            graph.add_edge("renamed_alpha_17", "renamed_sink_29", None);
            graph.add_edge("renamed_beta_23", "renamed_sink_29", None);
        }
        grouped.add_together("renamed_group_37", Some("renamed_outer_31"), None);
        grouped.add_together_node("renamed_group_37", "renamed_alpha_17");
        grouped.add_together_node("renamed_group_37", "renamed_beta_23");

        let plain_result = plain.layout_full_no_timeout();
        let grouped_result = grouped.layout_full_no_timeout();

        assert_eq!(grouped_result.cluster_positions.len(), 1);
        assert!(
            grouped_result.cluster_positions[0].width > plain_result.cluster_positions[0].width
        );
        assert!(
            grouped_result.cluster_positions[0].height > plain_result.cluster_positions[0].height
        );
    }

    #[test]
    fn same_rank_subgraphs_stay_inside_their_svek_cluster() {
        let mut graph = LayoutGraph::new(Direction::TopToBottom).with_plantuml_svek_spacing();
        for id in ["RenamedAlpha17", "RenamedBeta23", "RenamedGamma31"] {
            graph.add_node(id, "", 58.0, 46.0);
        }
        graph.add_svek_cluster(
            "RenamedContainer41",
            None,
            ClusterTitleSize {
                width: 131.0,
                height: 16.0,
            },
        );
        for id in ["RenamedAlpha17", "RenamedBeta23", "RenamedGamma31"] {
            graph.add_cluster_node("RenamedContainer41", id);
        }
        graph.add_same_rank("RenamedAlpha17", "RenamedBeta23");
        graph.add_same_rank("RenamedBeta23", "RenamedGamma31");
        graph.add_edge("RenamedAlpha17", "RenamedBeta23", None);
        graph.add_edge("RenamedBeta23", "RenamedGamma31", None);

        let result = graph.layout_full_no_timeout();
        let cluster = &result.cluster_positions[0];

        assert!(cluster.width > 250.0, "cluster must wrap all three members");
        assert!(cluster.height > 80.0, "cluster must retain its title band");
        for node in &result.node_positions {
            assert!(node.x >= cluster.x && node.x + node.width <= cluster.x + cluster.width);
            assert!(node.y >= cluster.y && node.y + node.height <= cluster.y + cluster.height);
        }
    }

    #[test]
    fn svek_clusters_follow_dot_serialization_order() {
        let mut g = LayoutGraph::new(Direction::TopToBottom).with_plantuml_svek_node_order();
        for id in [
            "RootBefore_7",
            "DirectZulu_19",
            "LeafBeta_23",
            "LeafAlpha_29",
            "DirectAlpha_31",
            "RootAfter_37",
        ] {
            g.add_node(id, id, 80.0, 40.0);
        }
        assert!(g.add_svek_cluster(
            "Outer_Renamed_17",
            None,
            ClusterTitleSize {
                width: 121.9,
                height: 16.9,
            },
        ));
        assert!(g.add_svek_cluster(
            "Inner_Q",
            Some("Outer_Renamed_17"),
            ClusterTitleSize {
                width: 53.9,
                height: 16.9,
            },
        ));
        g.add_cluster_node("Outer_Renamed_17", "DirectZulu_19");
        g.add_cluster_node("Outer_Renamed_17", "DirectAlpha_31");
        g.add_cluster_node("Inner_Q", "LeafBeta_23");
        g.add_cluster_node("Inner_Q", "LeafAlpha_29");

        assert_eq!(g.graphviz_node_creation_order(), vec![0, 5, 1, 4, 2, 3]);
        assert_eq!(
            cluster_title_table(ClusterTitleSize {
                width: 81.9,
                height: 16.9,
            }),
            r##"<TABLE BGCOLOR="#000004" FIXEDSIZE="TRUE" WIDTH="81" HEIGHT="11"><TR><TD></TD></TR></TABLE>"##
        );

        let result = g.layout_full_no_timeout();
        assert_eq!(
            result
                .cluster_positions
                .iter()
                .map(|cluster| cluster.id.as_str())
                .collect::<Vec<_>>(),
            ["Outer_Renamed_17", "Inner_Q"]
        );
        let outer = &result.cluster_positions[0];
        let inner = &result.cluster_positions[1];
        assert!(outer.x < inner.x);
        assert!(outer.y < inner.y);
        assert!(outer.x + outer.width > inner.x + inner.width);
        assert!(outer.y + outer.height > inner.y + inner.height);
    }

    #[test]
    fn inverted_svek_starts_precede_ordinary_node_creation() {
        let mut graph = LayoutGraph::new(Direction::TopToBottom);
        graph.add_node("DeclaredFirst", "", 40.0, 30.0);
        graph.add_node("InvertedStartOne", "", 40.0, 30.0);
        graph.add_node("InvertedStartTwo", "", 40.0, 30.0);

        // Java `Cluster.getNodesOrderedTop` uses `firsts.add(0, start)`, so
        // later inverted links precede earlier ones.
        graph.add_plantuml_svek_inverted_start("InvertedStartOne");
        graph.add_plantuml_svek_inverted_start("InvertedStartTwo");

        assert_eq!(graph.graphviz_node_creation_order(), vec![2, 1, 0]);
    }

    #[test]
    fn svek_line0_edges_create_endpoints_after_inverted_starts() {
        let mut graph = LayoutGraph::new(Direction::TopToBottom);
        graph.add_node("DeclaredFirst", "", 40.0, 30.0);
        graph.add_node("InvertedStart", "", 40.0, 30.0);
        graph.add_node("LineZeroFrom", "", 40.0, 30.0);
        graph.add_node("LineZeroTo", "", 40.0, 30.0);
        graph.add_node("DeclaredLast", "", 40.0, 30.0);

        graph.add_plantuml_svek_inverted_start("InvertedStart");
        graph.add_plantuml_svek_line0_edge("LineZeroFrom", "LineZeroTo");
        graph.add_edge_with_minlen("LineZeroFrom", "LineZeroTo", None, 0);

        assert_eq!(graph.graphviz_node_creation_order(), vec![1, 2, 3, 0, 4]);
        let result = graph.layout_full_no_timeout();
        assert_eq!(result.edge_paths.len(), 1);
        assert_eq!(result.edge_paths[0].from, "LineZeroFrom");
        assert_eq!(result.edge_paths[0].to, "LineZeroTo");
    }

    #[test]
    fn svek_cluster_title_table_sets_minimum_width() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        g.add_node("member", "member", 100.0, 40.0);
        assert!(g.add_svek_cluster(
            "Renamed_Wide_Title",
            None,
            ClusterTitleSize {
                width: 220.0,
                height: 21.0,
            },
        ));
        g.add_cluster_node("Renamed_Wide_Title", "member");

        let result = g.layout_full_no_timeout();
        let cluster = &result.cluster_positions[0];

        // `ClusterDotString.printInternal` emits the title as a fixed 220px
        // HTML table. Dot adds eight points of cluster margin on each side.
        assert_eq!(cluster.width, 236.0);
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
    fn ellipse_node_keeps_independent_dimensions() {
        let mut g = LayoutGraph::new(Direction::TopToBottom);
        assert!(g.add_ellipse_node("oval_renamed", "Review", 120.0, 40.0));
        assert!(!g.add_ellipse_node("oval_renamed", "Review", 120.0, 40.0));

        let result = g.layout_full_no_timeout();
        let oval = &result.node_positions[0];
        assert!((oval.width - 120.0).abs() < 0.01);
        assert!((oval.height - 40.0).abs() < 0.01);
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
    fn fixed_html_row_node_routes_distinct_named_rows() {
        let mut g = LayoutGraph::new(Direction::LeftToRight);
        assert!(g.add_fixed_html_row_node(
            "map",
            118.0,
            82.0,
            &[
                HtmlRowPort {
                    id: "row_top".to_string(),
                    position: 20.48,
                    height: 20.48,
                },
                HtmlRowPort {
                    id: "row_bottom".to_string(),
                    position: 61.46,
                    height: 20.48,
                },
            ],
        ));
        g.add_node("top_target", "", 60.0, 40.0);
        g.add_node("bottom_target", "", 60.0, 40.0);
        g.add_edge_with_ports("map", "top_target", None, Some("row_top"), None);
        g.add_edge_with_ports("map", "bottom_target", None, Some("row_bottom"), None);

        let result = g.layout_full_no_timeout();
        let map = &result.node_positions[0];
        assert!((map.width - 134.0).abs() < 0.01);
        assert!((map.height - 90.0).abs() < 0.01);
        let top = result
            .edge_paths
            .iter()
            .find(|edge| edge.to == "top_target")
            .unwrap();
        let bottom = result
            .edge_paths
            .iter()
            .find(|edge| edge.to == "bottom_target")
            .unwrap();
        assert!(
            top.points[0].1 < bottom.points[0].1,
            "named HTML rows must expose distinct vertical ports"
        );
    }

    #[test]
    fn fixed_html_rows_follow_java_integer_gap_serialization() {
        let table = fixed_html_row_table(
            118.0,
            82.0,
            &[
                HtmlRowPort {
                    id: "top_row".to_string(),
                    position: 20.48,
                    height: 20.48,
                },
                HtmlRowPort {
                    id: "bottom_row".to_string(),
                    position: 61.46,
                    height: 20.48,
                },
            ],
        );
        assert_eq!(
            table,
            r##"<TABLE BGCOLOR="#000001" BORDER="0" CELLBORDER="0" CELLSPACING="0" CELLPADDING="0"><TR><TD FIXEDSIZE="TRUE" WIDTH="118" HEIGHT="20"></TD></TR><TR><TD FIXEDSIZE="TRUE" WIDTH="118" HEIGHT="20" PORT="top_row"></TD></TR><TR><TD FIXEDSIZE="TRUE" WIDTH="118" HEIGHT="21"></TD></TR><TR><TD FIXEDSIZE="TRUE" WIDTH="118" HEIGHT="20" PORT="bottom_row"></TD></TR><TR><TD FIXEDSIZE="TRUE" WIDTH="118" HEIGHT="1"></TD></TR></TABLE>"##
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
