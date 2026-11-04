// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
// Recovery checker fault injection; derived from checker3c's transcript-073.
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
static unsigned allocation_index, fail_at;
static void *checked_calloc(size_t n, size_t z) { return ++allocation_index == fail_at ? NULL : calloc(n,z); }
static void *checked_malloc(size_t z) { return ++allocation_index == fail_at ? NULL : malloc(z); }
static void *checked_realloc(void *p,size_t z) { return ++allocation_index == fail_at ? NULL : realloc(p,z); }
#define calloc checked_calloc
#define malloc checked_malloc
#define realloc checked_realloc
#include "rustuml_helpers.c"
#undef calloc
#undef malloc
#undef realloc
int main(int argc,char **argv) {
    if(argc != 3) return 2;
    fail_at=(unsigned)strtoul(argv[1],NULL,10);
    FILE *f=fopen(argv[2],"rb"); if(!f) return 3;
    if(fseek(f,0,SEEK_END)) return 4;
    long n=ftell(f); if(n<0 || n>10000000 || fseek(f,0,SEEK_SET)) return 4;
    char *data=malloc((size_t)n+1); if(!data) return 5;
    if(fread(data,1,(size_t)n,f)!=(size_t)n) return 6;
    fclose(f);data[n]=0;
    textlabel_t label={0};label.text=data;
    int accepted=rustuml_make_empty_grid_label(&label);
    if(accepted)free_html_label(label.u.html,1);
    free(data);
    return accepted ? 0 : 10;
}
