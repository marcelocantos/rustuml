// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static size_t allocation_index, fail_index;
static void *fault_malloc(size_t size) {
    return ++allocation_index == fail_index ? NULL : malloc(size);
}
static void *fault_calloc(size_t count, size_t size) {
    return ++allocation_index == fail_index ? NULL : calloc(count, size);
}
static void *fault_realloc(void *ptr, size_t size) {
    return ++allocation_index == fail_index ? NULL : realloc(ptr, size);
}
#define malloc fault_malloc
#define calloc fault_calloc
#define realloc fault_realloc
#include "vendor/graphviz/lib/rustuml_helpers.c"
#undef malloc
#undef calloc
#undef realloc
int main(int argc, char **argv) {
    if (argc != 3) return 2;
    fail_index = strtoul(argv[1], NULL, 10);
    FILE *f = fopen(argv[2], "rb");
    if (!f) return 3;
    if (fseek(f, 0, SEEK_END)) return 4;
    long size = ftell(f);
    if (size < 0 || fseek(f, 0, SEEK_SET)) return 4;
    char *source = malloc((size_t)size + 1);
    if (!source || fread(source, 1, (size_t)size, f) != (size_t)size) return 5;
    fclose(f);
    source[size] = '\0';
    textlabel_t label = {0};
    label.text = source;
    int supported = rustuml_make_empty_grid_label(&label);
    printf("{\"supported\":%d,\"allocations\":%zu,\"width\":%g,\"height\":%g}\n",
           supported, allocation_index, label.dimen.x, label.dimen.y);
    if (supported) free_html_label(label.u.html, 1);
    free(source);
    return supported ? 0 : 10;
}
