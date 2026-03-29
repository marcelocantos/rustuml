// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Build script: compile vendored Graphviz C libraries into a static archive.

use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor = manifest_dir.join("../../vendor/graphviz");
    let lib = vendor.join("lib");
    let include = vendor.join("include");

    let mut build = cc::Build::new();

    // Compiler settings
    build
        .warnings(false) // Graphviz code has many warnings; suppress them
        .extra_warnings(false)
        .opt_level(2);

    // Include paths — order matters.
    // "config.h" and "graphviz_version.h" and "builddate.h" live in include/
    build.include(&include);
    // lib/ is the root for <cdt/cdt.h>, <cgraph/cgraph.h>, <common/types.h> etc.
    build.include(&lib);
    // Some files use bare includes like "cghdr.h" from within cgraph/
    build.include(lib.join("cgraph"));
    // render.h includes "gvplugin.h" etc. with bare names
    build.include(lib.join("gvc"));
    // common files use bare includes like "types.h"
    build.include(lib.join("common"));
    // generated htmlparse.c is in common/ and needs the generated common/ subdir
    // for colortbl.h and entities.h
    build.include(lib.join("common")); // for common/colortbl.h, common/entities.h
    // pathplan headers referenced as "pathgeom.h" from within pathplan/
    build.include(lib.join("pathplan"));
    // pack headers
    build.include(lib.join("pack"));
    // cdt headers referenced directly
    build.include(lib.join("cdt"));
    // label headers
    build.include(lib.join("label"));

    // config.h (in include/) has all external library deps #undef'd.
    // No HAVE_EXPAT, no ENABLE_LTDL, no GD/Pango/Cairo/etc.

    // macOS: use Apple's native ar to produce 8-byte-aligned archives
    // (GNU ar from toolchains like .den/envs can produce misaligned ones
    // that the Apple linker rejects).
    if cfg!(target_os = "macos") {
        build.flag("-fno-common");
        build.archiver("/usr/bin/ar");
    }

    // ── cdt ──
    for f in &[
        "dtclose", "dtdisc", "dtextract", "dtflatten", "dthash", "dtmethod",
        "dtopen", "dtrenew", "dtrestore", "dtsize", "dtstat", "dtstrhash",
        "dttree", "dtview", "dtwalk",
    ] {
        build.file(lib.join(format!("cdt/{f}.c")));
    }

    // ── cgraph ──
    for f in &[
        "acyclic", "agerror", "apply", "attr", "edge", "grammar", "graph",
        "id", "imap", "ingraphs", "io", "node", "node_induce", "obj", "rec",
        "refstr", "scan", "subg", "tred", "unflatten", "utils", "write",
    ] {
        build.file(lib.join(format!("cgraph/{f}.c")));
    }

    // ── pathplan ──
    for f in &[
        "cvt", "inpoly", "route", "shortest", "shortestpth", "solvers",
        "triang", "util", "visibility",
    ] {
        build.file(lib.join(format!("pathplan/{f}.c")));
    }

    // ── util ──
    for f in &["arena", "base64", "gv_find_me", "gv_fopen", "list", "random", "xml"] {
        build.file(lib.join(format!("util/{f}.c")));
    }

    // ── xdot ──
    build.file(lib.join("xdot/xdot.c"));

    // ── label ──
    for f in &["index", "node", "rectangle", "split.q", "xlabels"] {
        build.file(lib.join(format!("label/{f}.c")));
    }

    // ── common ──
    for f in &[
        "args", "arrows", "colxlate", "ellipse", "emit", "geom", "globals",
        "htmllex", "htmlparse", "htmltable", "input", "labels", "ns",
        "output", "pointset", "postproc", "psusershape", "routespl",
        "shapes", "splines", "taper", "textspan", "textspan_lut", "timing",
        "utils",
    ] {
        build.file(lib.join(format!("common/{f}.c")));
    }

    // ── pack ──
    for f in &["ccomps", "pack"] {
        build.file(lib.join(format!("pack/{f}.c")));
    }

    // ── ortho ──
    for f in &[
        "fPQ", "maze", "ortho", "partition", "rawgraph", "sgraph", "trapezoid",
    ] {
        build.file(lib.join(format!("ortho/{f}.c")));
    }

    // ── dotgen ──
    for f in &[
        "acyclic", "aspect", "class1", "class2", "cluster", "compound",
        "conc", "decomp", "dotinit", "dotsplines", "fastgr", "flat",
        "mincross", "position", "rank", "sameport",
    ] {
        build.file(lib.join(format!("dotgen/{f}.c")));
    }

    // ── gvc ──
    for f in &[
        "gvc", "gvconfig", "gvcontext", "gvdevice", "gvevent", "gvjobs",
        "gvlayout", "gvloadimage", "gvplugin", "gvrender", "gvtextlayout",
        "gvtool_tred", "gvusershape",
    ] {
        build.file(lib.join(format!("gvc/{f}.c")));
    }

    // ── RustUML helper functions (C wrappers around Graphviz macros) ──
    build.file(lib.join("rustuml_helpers.c"));

    build.compile("graphviz");

    // Tell cargo to link the static library
    println!("cargo:rustc-link-lib=static=graphviz");
}
