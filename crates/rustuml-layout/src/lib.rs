// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Hierarchical graph layout engine for RustUML.
//!
//! Uses vendored Graphviz (dot algorithm) for layout with configurable edge
//! routing.

pub mod graph;
mod graphviz_ffi;

#[cfg(feature = "diagnostics")]
pub use graph::{LayoutDiagnostics, capture_layout_diagnostics};
