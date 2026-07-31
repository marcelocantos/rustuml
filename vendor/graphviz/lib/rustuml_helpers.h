// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

// Thin C helper functions exposing Graphviz internal struct fields
// that are hidden behind macros (ND_coord, ED_spl, etc.).
// Called from Rust FFI — avoids replicating fragile struct layouts.

#pragma once

#include "config.h"
#include <common/types.h>
#include <cgraph/cgraph.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ── Node coordinate access ──

// Get the laid-out (x, y) position of a node (in points).
void rustuml_node_pos(Agnode_t *n, double *x, double *y);

// Get the bounding box dimensions of a node (width/height in inches).
void rustuml_node_size(Agnode_t *n, double *w, double *h);

// Get a laid-out graph or subgraph bounding box in points.
void rustuml_graph_bb(Agraph_t *g, double *ll_x, double *ll_y,
                      double *ur_x, double *ur_y);

// ── Edge spline access ──

// Returns the number of bezier curves in the edge spline, or 0 if none.
size_t rustuml_edge_spl_count(Agedge_t *e);

// Get the i-th bezier curve's control points.
// Returns the number of points written, or 0 on error.
// `out_pts` must have room for at least `max_pts` pointf-sized elements.
// Points are pairs of doubles (x0, y0, x1, y1, ...).
size_t rustuml_edge_bezier_points(Agedge_t *e, size_t idx,
                                  double *out_pts, size_t max_pts);

// Get arrow endpoint info for the i-th bezier of an edge.
// sflag/eflag: 0 = no arrow, nonzero = has arrow.
// sp/ep: the start/end arrow tip points.
void rustuml_edge_bezier_arrows(Agedge_t *e, size_t idx,
                                int *sflag, double *sp_x, double *sp_y,
                                int *eflag, double *ep_x, double *ep_y);

// Get a solved edge label box.
// kind: 0 = center, 1 = tail, 2 = head, 3 = external center.
// Returns 1 when the requested label exists and has a solved position.
int rustuml_edge_label_box(Agedge_t *e, int kind,
                           double *x, double *y,
                           double *width, double *height);

// Apply the dimensions of RustUML's generated empty fixed-size HTML table.
// Returns 1 only for the exact internal placeholder shape.
int rustuml_make_fixed_html_table_label(textlabel_t *label);

// Parse PlantUML Smetana's exact `_dim_<width>_<height>_` text-span marker.
// Returns 1 on success and leaves the outputs unchanged on failure.
int rustuml_parse_text_span_dimensions(const char *text,
                                       double *width, double *height);

// Returns 1 when a record node opted into PlantUML text-span dimensions.
int rustuml_node_uses_text_span_dimensions(Agnode_t *node);

#ifdef __cplusplus
}
#endif
