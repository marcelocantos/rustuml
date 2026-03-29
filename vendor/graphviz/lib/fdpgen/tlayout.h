// Minimal stub for RustUML vendored build.
#pragma once

#include <fdpgen/fdp.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    int numIters;
    double T0;
    double K;
    double C;
    int loopcnt;
} xparams;

extern void fdp_initParams(graph_t *);
extern void fdp_tLayout(graph_t *, xparams *);

#ifdef __cplusplus
}
#endif
