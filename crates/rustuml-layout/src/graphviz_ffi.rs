// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Minimal FFI bindings to vendored Graphviz C libraries.
//!
//! We only bind the functions needed for layout: graph construction,
//! layout computation, and coordinate/spline extraction.
//!
//! Coordinate access (ND_coord, ED_spl) uses C helper functions
//! (`rustuml_helpers.c`) rather than replicating fragile struct layouts.

#![allow(non_camel_case_types, dead_code)]

use std::os::raw::{c_char, c_int, c_void};

#[cfg(feature = "diagnostics")]
use std::ffi::CStr;

// ── Opaque Graphviz types ──

/// Graphviz context handle (opaque).
pub enum GVC_t {}

/// Graph handle (opaque).
pub enum Agraph_t {}

/// Node handle (opaque).
pub enum Agnode_t {}

/// Edge handle (opaque).
pub enum Agedge_t {}

/// Discipline handle (opaque).
pub enum Agdisc_t {}

/// Plugin library — used to register layout engines with GVC.
#[repr(C)]
pub struct gvplugin_library_t {
    _data: [u8; 0],
}

/// Graph descriptor — controls directed/strict/etc. properties.
/// Must match the C `struct Agdesc_s` bitfield layout.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Agdesc_t {
    _bitfields: u32,
}

unsafe extern "C" {
    // ── Predefined graph descriptors ──
    pub static Agdirected: Agdesc_t;
    pub static Agstrictdirected: Agdesc_t;
    pub static Agundirected: Agdesc_t;
    pub static Agstrictundirected: Agdesc_t;

    // ── GVC context ──
    pub fn gvContext() -> *mut GVC_t;
    pub fn gvFreeContext(gvc: *mut GVC_t) -> c_int;
    pub fn gvAddLibrary(gvc: *mut GVC_t, lib: *mut gvplugin_library_t);

    // ── Dot layout plugin library (statically linked) ──
    pub static mut gvplugin_dot_layout_LTX_library: gvplugin_library_t;

    // ── Graph construction ──
    pub fn agopen(name: *const c_char, kind: Agdesc_t, disc: *mut Agdisc_t) -> *mut Agraph_t;
    pub fn agsubg(g: *mut Agraph_t, name: *mut c_char, create: c_int) -> *mut Agraph_t;
    pub fn agclose(g: *mut Agraph_t) -> c_int;

    // ── Node/edge construction ──
    pub fn agnode(g: *mut Agraph_t, name: *const c_char, create: c_int) -> *mut Agnode_t;
    pub fn agsubnode(g: *mut Agraph_t, n: *mut Agnode_t, create: c_int) -> *mut Agnode_t;
    pub fn agedge(
        g: *mut Agraph_t,
        t: *mut Agnode_t,
        h: *mut Agnode_t,
        name: *const c_char,
        create: c_int,
    ) -> *mut Agedge_t;

    // ── Attribute setting ──
    pub fn agsafeset(
        obj: *mut c_void,
        name: *const c_char,
        val: *const c_char,
        def: *const c_char,
    ) -> c_int;
    pub fn agsafeset_html(
        obj: *mut c_void,
        name: *const c_char,
        val: *const c_char,
        def: *const c_char,
    ) -> c_int;

    // ── Layout ──
    pub fn gvLayout(gvc: *mut GVC_t, g: *mut Agraph_t, engine: *const c_char) -> c_int;
    pub fn gvFreeLayout(gvc: *mut GVC_t, g: *mut Agraph_t) -> c_int;

    // ── Graph traversal ──
    pub fn agfstnode(g: *mut Agraph_t) -> *mut Agnode_t;
    pub fn agnxtnode(g: *mut Agraph_t, n: *mut Agnode_t) -> *mut Agnode_t;
    pub fn agfstout(g: *mut Agraph_t, n: *mut Agnode_t) -> *mut Agedge_t;
    pub fn agnxtout(g: *mut Agraph_t, e: *mut Agedge_t) -> *mut Agedge_t;
    pub fn agnameof(obj: *mut c_void) -> *const c_char;

    // ── RustUML helper functions (wrappers around C macros) ──

    /// Get the laid-out position of a node (in points).
    pub fn rustuml_node_pos(n: *mut Agnode_t, x: *mut f64, y: *mut f64);

    /// Get the bounding box dimensions of a node (width/height in inches).
    pub fn rustuml_node_size(n: *mut Agnode_t, w: *mut f64, h: *mut f64);

    /// Get a laid-out graph or subgraph bounding box in points.
    pub fn rustuml_graph_bb(
        g: *mut Agraph_t,
        ll_x: *mut f64,
        ll_y: *mut f64,
        ur_x: *mut f64,
        ur_y: *mut f64,
    );

    /// Returns the number of bezier curves in the edge's spline, or 0 if none.
    pub fn rustuml_edge_spl_count(e: *mut Agedge_t) -> usize;

    /// Get the i-th bezier curve's control points.
    /// `out_pts` must have room for `max_pts * 2` doubles (x, y pairs).
    /// Returns the number of points written.
    pub fn rustuml_edge_bezier_points(
        e: *mut Agedge_t,
        idx: usize,
        out_pts: *mut f64,
        max_pts: usize,
    ) -> usize;

    /// Get arrow endpoint info for the i-th bezier of an edge.
    pub fn rustuml_edge_bezier_arrows(
        e: *mut Agedge_t,
        idx: usize,
        sflag: *mut c_int,
        sp_x: *mut f64,
        sp_y: *mut f64,
        eflag: *mut c_int,
        ep_x: *mut f64,
        ep_y: *mut f64,
    );

    /// Get a solved edge label box. kind: 0 = center, 1 = tail, 2 = head.
    pub fn rustuml_edge_label_box(
        e: *mut Agedge_t,
        kind: c_int,
        x: *mut f64,
        y: *mut f64,
        width: *mut f64,
        height: *mut f64,
    ) -> c_int;

    /// Parse PlantUML Smetana's exact `_dim_<width>_<height>_` span marker.
    pub fn rustuml_parse_text_span_dimensions(
        text: *const c_char,
        width: *mut f64,
        height: *mut f64,
    ) -> c_int;

    /// Returns nonzero for a node opted into PlantUML text-span dimensions.
    pub fn rustuml_node_uses_text_span_dimensions(node: *mut Agnode_t) -> c_int;

    #[cfg(feature = "diagnostics")]
    fn rustuml_graph_to_dot(g: *mut Agraph_t, length: *mut usize) -> *mut c_char;

    #[cfg(feature = "diagnostics")]
    fn rustuml_free_string(value: *mut c_char);
}

#[cfg(feature = "diagnostics")]
pub(crate) unsafe fn graph_to_dot(g: *mut Agraph_t) -> Option<String> {
    let mut length = 0;
    let value = unsafe { rustuml_graph_to_dot(g, &mut length) };
    if value.is_null() {
        return None;
    }
    let dot = unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned();
    unsafe { rustuml_free_string(value) };
    debug_assert_eq!(dot.len(), length);
    Some(dot)
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    #[test]
    fn parses_only_exact_dimensional_text_spans_across_ffi() {
        let encoded = CString::new("_dim_20.5_37.25_").unwrap();
        let mut width = -1.0;
        let mut height = -1.0;
        let parsed = unsafe {
            super::rustuml_parse_text_span_dimensions(encoded.as_ptr(), &mut width, &mut height)
        };
        assert_eq!(parsed, 1);
        assert_eq!((width, height), (20.5, 37.25));

        for text in [
            "renamed_record",
            "_dim_20.5_37.25",
            "_dim_-20_37_",
            "_dim_20_37_extra",
            "_dim_20..5_37_",
        ] {
            let encoded = CString::new(text).unwrap();
            width = -1.0;
            height = -1.0;
            let parsed = unsafe {
                super::rustuml_parse_text_span_dimensions(encoded.as_ptr(), &mut width, &mut height)
            };
            assert_eq!(parsed, 0, "{text}");
            assert_eq!((width, height), (-1.0, -1.0), "{text}");
        }
    }

    #[test]
    fn dimensional_record_marker_crosses_cgraph_boundary() {
        let graph_name = CString::new("dimension_marker_test").unwrap();
        let node_name = CString::new("renamed_record").unwrap();
        let key = CString::new("rustuml_text_span_dimensions").unwrap();
        let value = CString::new("true").unwrap();
        let empty = CString::new("").unwrap();

        unsafe {
            let graph = super::agopen(graph_name.as_ptr(), super::Agdirected, std::ptr::null_mut());
            assert!(!graph.is_null());
            let node = super::agnode(graph, node_name.as_ptr(), 1);
            assert!(!node.is_null());
            assert_eq!(
                super::agsafeset(node.cast(), key.as_ptr(), value.as_ptr(), empty.as_ptr(),),
                0
            );
            assert_eq!(super::rustuml_node_uses_text_span_dimensions(node), 1);
            assert_eq!(super::agclose(graph), 0);
        }
    }
}
