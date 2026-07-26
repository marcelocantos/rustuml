// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

#include "rustuml_helpers.h"

#include <common/const.h>
#include <common/htmltable.h>
#include <limits.h>
#include <stdint.h>
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
    static const char *const row_table_prefix =
        "<TABLE BGCOLOR=\"#000001\" BORDER=\"0\" CELLBORDER=\"0\" "
        "CELLSPACING=\"0\" CELLPADDING=\"0\">";
    static const char *const row_prefix =
        "<TR><TD FIXEDSIZE=\"TRUE\" WIDTH=\"";
    static const char *const height_marker = "\" HEIGHT=\"";
    static const char *const port_marker = "\" PORT=\"";
    static const char *const row_suffix = "\"></TD></TR>";
    const char *width_attr;
    const char *height_attr;
    char *width_end;
    char *height_end;
    unsigned long width;
    unsigned long height;

    if (!label || !label->text || strncmp(label->text, prefix, strlen(prefix)) != 0)
        return 0;

    if (strncmp(label->text, row_table_prefix, strlen(row_table_prefix)) == 0) {
        const char *cursor = label->text + strlen(row_table_prefix);
        size_t row_count = 0;
        unsigned long table_width = 0;
        unsigned long table_height = 0;

        while (strncmp(cursor, row_prefix, strlen(row_prefix)) == 0) {
            char *end;
            unsigned long row_width =
                strtoul(cursor + strlen(row_prefix), &end, 10);
            end = strchr(end, '"');
            if (row_width == 0 || !end ||
                strncmp(end, height_marker, strlen(height_marker)) != 0)
                return 0;
            unsigned long row_height =
                strtoul(end + strlen(height_marker), &end, 10);
            if (row_height == 0)
                return 0;
            if (strncmp(end, port_marker, strlen(port_marker)) == 0) {
                end = strchr(end + strlen(port_marker), '"');
                if (!end)
                    return 0;
            }
            if (strncmp(end, row_suffix, strlen(row_suffix)) != 0)
                return 0;
            if (row_count == 0)
                table_width = row_width;
            else if (table_width != row_width)
                return 0;
            if (row_count == UINT16_MAX ||
                table_height > ULONG_MAX - row_height)
                return 0;
            table_height += row_height;
            row_count++;
            cursor = end + strlen(row_suffix);
        }
        if (row_count == 0 || strcmp(cursor, "</TABLE>") != 0)
            return 0;

        htmllabel_t *html = calloc(1, sizeof(*html));
        htmltbl_t *table = calloc(1, sizeof(*table));
        htmlcell_t **cells = calloc(row_count + 1, sizeof(*cells));
        double top = (double)table_height / 2.0;
        cursor = label->text + strlen(row_table_prefix);

        if (!html || !table || !cells) {
            free(html);
            free(table);
            free(cells);
            return 0;
        }

        html->kind = HTML_TBL;
        html->u.tbl = table;
        table->cells = cells;
        table->row_count = row_count;
        table->column_count = 1;
        // `poly_init` adds Graphviz's standard plaintext margins around this
        // centered table after the label has been constructed.
        table->data.box.LL.x = -(double)table_width / 2.0;
        table->data.box.LL.y = -(double)table_height / 2.0;
        table->data.box.UR.x = (double)table_width / 2.0;
        table->data.box.UR.y = (double)table_height / 2.0;

        for (size_t row = 0; row < row_count; row++) {
            char *end;
            strtoul(cursor + strlen(row_prefix), &end, 10);
            end = strchr(end, '"');
            unsigned long row_height =
                strtoul(end + strlen(height_marker), &end, 10);
            char *port = NULL;
            if (strncmp(end, port_marker, strlen(port_marker)) == 0) {
                const char *start = end + strlen(port_marker);
                const char *finish = strchr(start, '"');
                size_t length = (size_t)(finish - start);
                port = malloc(length + 1);
                if (!port) {
                    free_html_label(html, 1);
                    return 0;
                }
                memcpy(port, start, length);
                port[length] = '\0';
                end = (char *)finish;
            }

            htmlcell_t *cell = calloc(1, sizeof(*cell));
            if (!cell) {
                free(port);
                free_html_label(html, 1);
                return 0;
            }
            cells[row] = cell;
            cell->parent = table;
            cell->row = (uint16_t)row;
            cell->colspan = 1;
            cell->rowspan = 1;
            cell->data.port = port;
            cell->data.sides = LEFT | RIGHT;
            cell->data.box.LL.x = -(double)table_width / 2.0;
            cell->data.box.UR.x = (double)table_width / 2.0;
            cell->data.box.UR.y = top;
            cell->data.box.LL.y = top - (double)row_height;
            top = cell->data.box.LL.y;
            cursor = end + strlen(row_suffix);
        }

        label->dimen.x = (double)table_width;
        label->dimen.y = (double)table_height;
        label->space.x = 0.0;
        label->space.y = 0.0;
        label->u.html = html;
        return 1;
    }

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
