// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

#include "rustuml_helpers.h"

#include <stdlib.h>

void rustuml_node_pos(Agnode_t *n, double *x, double *y) {
    pointf p = ND_coord(n);
    *x = p.x;
    *y = p.y;
}

void rustuml_node_size(Agnode_t *n, double *w, double *h) {
    *w = ND_width(n);
    *h = ND_height(n);
}

void rustuml_graph_bb(Agraph_t *g, double *ll_x, double *ll_y,
                      double *ur_x, double *ur_y) {
    boxf bb = GD_bb(g);
    *ll_x = bb.LL.x;
    *ll_y = bb.LL.y;
    *ur_x = bb.UR.x;
    *ur_y = bb.UR.y;
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

int rustuml_edge_label_box(Agedge_t *e, int kind,
                           double *x, double *y,
                           double *width, double *height) {
    textlabel_t *label = NULL;
    if (kind == 0) label = ED_label(e);
    else if (kind == 1) label = ED_tail_label(e);
    else if (kind == 2) label = ED_head_label(e);

    if (!label || !label->set) return 0;
    *x = label->pos.x;
    *y = label->pos.y;
    *width = label->dimen.x;
    *height = label->dimen.y;
    return 1;
}

static void override_label_dimensions(Agedge_t *e, textlabel_t *label,
                                      const char *width_name,
                                      const char *height_name) {
    if (!label) return;
    const char *width = agget(e, (char *)width_name);
    const char *height = agget(e, (char *)height_name);
    if (!width || !height || !*width || !*height) return;

    label->dimen.x = strtod(width, NULL);
    label->dimen.y = strtod(height, NULL);
    label->space = label->dimen;
}

void rustuml_override_edge_label_dimensions(Agedge_t *e) {
    override_label_dimensions(e, ED_label(e),
                              "rustuml_label_width", "rustuml_label_height");
    override_label_dimensions(e, ED_tail_label(e),
                              "rustuml_tail_label_width", "rustuml_tail_label_height");
    override_label_dimensions(e, ED_head_label(e),
                              "rustuml_head_label_width", "rustuml_head_label_height");
}
