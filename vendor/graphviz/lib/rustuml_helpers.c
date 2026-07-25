// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

#include "rustuml_helpers.h"

#include <stdlib.h>
#include <string.h>

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

int rustuml_make_fixed_html_table_label(textlabel_t *label) {
    static const char *const prefix = "<TABLE BGCOLOR=\"#00000";
    static const char *const body = "\"><TR><TD></TD></TR></TABLE>";
    const char *width_attr;
    const char *height_attr;
    char *width_end;
    char *height_end;
    unsigned long width;
    unsigned long height;

    if (!label || !label->text || strncmp(label->text, prefix, strlen(prefix)) != 0)
        return 0;
    if (!strstr(label->text, " FIXEDSIZE=\"TRUE\" ") || !strstr(label->text, body))
        return 0;

    width_attr = strstr(label->text, " WIDTH=\"");
    height_attr = strstr(label->text, " HEIGHT=\"");
    if (!width_attr || !height_attr)
        return 0;

    width = strtoul(width_attr + strlen(" WIDTH=\""), &width_end, 10);
    height = strtoul(height_attr + strlen(" HEIGHT=\""), &height_end, 10);
    if (width == 0 || height == 0 || *width_end != '"' || *height_end != '"')
        return 0;

    // RustUML uses Graphviz only for layout. This is the no-Expat equivalent
    // of Graphviz's fixed HTML table sizing, not a renderer side channel.
    label->html = false;
    label->dimen.x = (double)width;
    label->dimen.y = (double)height;
    // `make_html_label` sets table dimensions but leaves `space` at the zero
    // value allocated by `make_label`. External endpoint-label placement
    // observes that distinction.
    label->space.x = 0.0;
    label->space.y = 0.0;
    return 1;
}
