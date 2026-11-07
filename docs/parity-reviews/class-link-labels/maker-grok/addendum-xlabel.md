# Addendum — ortho center labels use xlabel

Written after the first draft was measured, and before the placement edit.
The inset `ORTHO_XLABEL_VERTICAL_INSET` (5.0) is not changed.

## What the first draft did

Tip binary `9c34537e0224b9a249a924e9e63dbd4055c6f935713a789a6e2c4a942b3a9e47`
against relay `shared-svg-round9-c9a` Java stdout. Strict SVG compare is
`compare_svg_strict` (`GEOM_EPS` 0.02, frozen, not edited). A local attribute
compare with that tolerance, and HTML-unescaped text, says every class
`c9a_af_*` case is clean except `c9a_af_ortho_vd`:

- viewBox `0 0 104 178` on both sides
- textLength 61.0962, x 7, on both sides
- y 82.1098 (Java) versus 84.1103 (Rust)

That is the precommitted synthesizer result (predicted baseline 84.1098).
Size 9, `\n`, `ArrowFontStyle`, Courier New italic, `<<use>>`, and the
size-16 quantifiers are inside the tolerance. `c9a_af_labelarrow_vd` stays
inside it at font size 13, so `(int)(size * 0.80)` did not move the default
guide arrow.

## Placement edit this addendum authorises

`SvekEdge.appendLine` 423–431 writes `xlabel` when `dotSplines == ORTHO`.
`solve` reads that table back with `getXY` (734–741). Graphviz stores the
solved xlabel on `ED_xlabel`. `postproc.c` 577 assigns `lp->pos = centerPt(xlp)`
after the lower-left search, so the pointer Rust already treats as a center
(`graph.rs` then `x -= width/2`, `y = max_y - y - height/2`) is the same
center for an xlabel.

Class ortho center labels therefore go out as `xlabel`, and the readback
uses `ED_xlabel` when `ED_label` is absent. The synthesizer still runs when
Graphviz returns no center box. It also still runs for the vertical
`ExtremityDoubleLine` / `ExtremityCircleCrowfoot` route: that route is
rewritten after layout by `normalize_ortho_er_vertical_route`, so an xlabel
solved on the pre-rewrite spline would no longer sit on the line. Those
edges keep today's omitted label and the inset of 5.0.
`ORTHO_ER_VERTICAL_ROUTE_DELTAS` is not edited.
