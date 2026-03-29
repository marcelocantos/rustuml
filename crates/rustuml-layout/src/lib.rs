// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Hierarchical graph layout engine for RustUML.
//!
//! Uses vendored Graphviz (dot algorithm) for layout with proper edge
//! routing via cubic bezier splines.

pub mod graph;
mod graphviz_ffi;
