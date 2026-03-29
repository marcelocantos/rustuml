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

#ifdef __cplusplus
}
#endif
