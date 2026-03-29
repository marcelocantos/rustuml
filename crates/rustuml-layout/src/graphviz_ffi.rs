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
    pub fn agclose(g: *mut Agraph_t) -> c_int;

    // ── Node/edge construction ──
    pub fn agnode(g: *mut Agraph_t, name: *const c_char, create: c_int) -> *mut Agnode_t;
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
}
