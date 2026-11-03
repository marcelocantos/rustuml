// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
// Diagnostic only: solve unchanged Java DOT with the shipped vendored dot engine.
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use rustuml_layout::graph::{Direction, LayoutGraph};

unsafe extern "C" {
    fn agmemread(input: *const c_char) -> *mut c_void;
    fn gvContext() -> *mut c_void;
    fn gvAddLibrary(context: *mut c_void, library: *mut c_void);
    static mut gvplugin_dot_layout_LTX_library: [u8; 0];
    fn gvLayout(context: *mut c_void, graph: *mut c_void, engine: *const c_char) -> c_int;
    fn gvFreeLayout(context: *mut c_void, graph: *mut c_void) -> c_int;
    fn gvFreeContext(context: *mut c_void) -> c_int;
    fn agclose(graph: *mut c_void) -> c_int;
    fn agfstnode(graph: *mut c_void) -> *mut c_void;
    fn agnxtnode(graph: *mut c_void, node: *mut c_void) -> *mut c_void;
    fn agnameof(object: *mut c_void) -> *const c_char;
    fn agfstsubg(graph: *mut c_void) -> *mut c_void;
    fn agnxtsubg(graph: *mut c_void) -> *mut c_void;
    fn rustuml_node_pos(node: *mut c_void, x: *mut f64, y: *mut f64);
    fn rustuml_node_size(node: *mut c_void, w: *mut f64, h: *mut f64);
    fn rustuml_graph_bb(graph: *mut c_void, x: *mut f64, y: *mut f64, w: *mut f64, h: *mut f64);
}

unsafe fn groups(graph: *mut c_void) {
    // The cgraph subgraph ownership tree is acyclic; inputs are checker-owned DOT.
    let mut bounds = [0.0; 4];
    unsafe {
        rustuml_graph_bb(graph, &mut bounds[0], &mut bounds[1], &mut bounds[2], &mut bounds[3]);
        println!("graph {} {:?}", CStr::from_ptr(agnameof(graph)).to_string_lossy(), bounds);
        let mut child = agfstsubg(graph);
        while !child.is_null() {
            groups(child);
            child = agnxtsubg(child);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args().nth(1).ok_or("usage: raw_dot JAVA.dot")?;
    let source = CString::new(std::fs::read(input)?)?;
    // Reference the crate so its exact Cargo-built static native libraries link.
    std::hint::black_box(LayoutGraph::new(Direction::TopToBottom));
    // All objects belong to this single-threaded diagnostic, and are freed below.
    unsafe {
        let graph = agmemread(source.as_ptr());
        if graph.is_null() { return Err("DOT parse failed".into()); }
        let context = gvContext();
        if context.is_null() { agclose(graph); return Err("context failed".into()); }
        gvAddLibrary(context, std::ptr::addr_of_mut!(gvplugin_dot_layout_LTX_library).cast());
        if gvLayout(context, graph, c"dot".as_ptr()) != 0 {
            agclose(graph); gvFreeContext(context); return Err("dot layout failed".into());
        }
        groups(graph);
        let mut node = agfstnode(graph);
        while !node.is_null() {
            let (mut x,mut y,mut w,mut h)=(0.0,0.0,0.0,0.0);
            rustuml_node_pos(node, &mut x, &mut y);
            rustuml_node_size(node, &mut w, &mut h);
            println!("node {} center=({x},{y}) inches=({w},{h})", CStr::from_ptr(agnameof(node)).to_string_lossy());
            node=agnxtnode(graph,node);
        }
        gvFreeLayout(context,graph); agclose(graph); gvFreeContext(context);
    }
    Ok(())
}
