// Minimal stub for RustUML vendored build — only struct fdpParms_s is needed.
#pragma once

#include <common/render.h>

#ifdef __cplusplus
extern "C" {
#endif

struct fdpParms_s {
    int useGrid;
    int useNew;
    int numIters;
    int unscaled;
    double C;
    double Tfact;
    double K;
    double T0;
};
typedef struct fdpParms_s fdpParms_t;

extern void fdp_layout(Agraph_t *g);
extern void fdp_init_node_edge(Agraph_t *g);
extern void fdp_cleanup(Agraph_t *g);

#ifdef __cplusplus
}
#endif
