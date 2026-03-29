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

extern void fdp_xLayout(graph_t *, xparams *);

#ifdef __cplusplus
}
#endif
