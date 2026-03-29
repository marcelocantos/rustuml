// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

#include "rustuml_helpers.h"

void rustuml_node_pos(Agnode_t *n, double *x, double *y) {
    pointf p = ND_coord(n);
    *x = p.x;
    *y = p.y;
}

void rustuml_node_size(Agnode_t *n, double *w, double *h) {
    *w = ND_width(n);
    *h = ND_height(n);
}

size_t rustuml_edge_spl_count(Agedge_t *e) {
    splines *spl = ED_spl(e);
    if (!spl) return 0;
    return spl->size;
}

size_t rustuml_edge_bezier_points(Agedge_t *e, size_t idx,
                                  double *out_pts, size_t max_pts) {
    splines *spl = ED_spl(e);
    if (!spl || idx >= spl->size) return 0;

    bezier *bz = &spl->list[idx];
    size_t n = bz->size;
    if (n > max_pts) n = max_pts;

    for (size_t i = 0; i < n; i++) {
        out_pts[i * 2]     = bz->list[i].x;
        out_pts[i * 2 + 1] = bz->list[i].y;
    }
    return n;
}

void rustuml_edge_bezier_arrows(Agedge_t *e, size_t idx,
                                int *sflag, double *sp_x, double *sp_y,
                                int *eflag, double *ep_x, double *ep_y) {
    splines *spl = ED_spl(e);
    if (!spl || idx >= spl->size) {
        *sflag = 0; *eflag = 0;
        return;
    }

    bezier *bz = &spl->list[idx];
    *sflag = (int)bz->sflag;
    *sp_x = bz->sp.x;
    *sp_y = bz->sp.y;
    *eflag = (int)bz->eflag;
    *ep_x = bz->ep.x;
    *ep_y = bz->ep.y;
}
