// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Whole-diagram layout compression — a faithful port of PlantUML's
//! `net.sourceforge.plantuml.klimt.compress` package
//! (`CompressionXorYBuilder`, `SlotFinder`, `SlotSet`, `Slot`,
//! `CompressionTransform`).
//!
//! PlantUML lays activity diagrams out with generous, construct-local spacing
//! (each FTile is unaware of its neighbours' empty space), then collapses the
//! empty bands at the very end:
//!
//! ```text
//! // ActivityDiagram3.exportDiagramInternal (lines 203-204):
//! result = CompressionXorYBuilder.build(CompressionMode.ON_X, result);
//! result = CompressionXorYBuilder.build(CompressionMode.ON_Y, result);
//! ```
//!
//! The algorithm, per axis:
//!   1. Walk every drawn shape, recording the 1-D interval it OCCUPIES on the
//!      axis (`SlotFinder`). Shapes flagged ignorable for the axis (connectors,
//!      arrowheads — `UShapeIgnorableForCompression` / `Worm.setCompressionMode`)
//!      contribute nothing, so an empty corridor they pass through can collapse.
//!   2. Merge overlapping occupied intervals (`SlotSet.addSlot`).
//!   3. Take the complement — the empty gaps between clusters (`SlotSet.reverse`).
//!   4. Drop gaps `<= 2*margin` and shrink the rest by `margin` per side
//!      (`SlotSet.smaller(5.0)`) — leaving 5px of breathing room each side.
//!   5. Build a piecewise-affine transform that removes those (now-shrunk) gaps
//!      (`CompressionTransform`): `transform(v) = v - Σ gap sizes left of v`,
//!      partial for the gap containing `v`.
//!   6. Re-emit every coordinate (occupied AND ignorable shapes) through the
//!      transform; the canvas dimension is the transformed extent.
//!
//! Net effect: every empty band wider than `2*margin` (10px) collapses to
//! exactly `2*margin`. The transform is the identity on a drawing that already
//! has no band wider than 10px, so re-running it (or running it on
//! already-compressed output) is a no-op — it is **idempotent**.

/// `margin` passed to `SlotSet.smaller` — PlantUML calls `smaller(5.0)`.
pub const COMPRESS_MARGIN: f64 = 5.0;

/// `net.sourceforge.plantuml.klimt.compress.CompressionMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompressionMode {
    OnX,
    OnY,
}

/// A merged set of 1-D intervals — port of `SlotSet` over `Slot(start,end)`.
/// Invariant: every stored interval has `start < end` and no two intersect.
#[derive(Default, Clone, Debug)]
pub struct SlotSet {
    all: Vec<(f64, f64)>,
}

/// `Slot.intersect`: closed-interval overlap (touching endpoints count, matching
/// Java's `contains` which uses `>=`/`<=`).
fn intersects(a: (f64, f64), b: (f64, f64)) -> bool {
    let contains = |s: (f64, f64), v: f64| v >= s.0 && v <= s.1;
    contains(a, b.0) || contains(a, b.1) || contains(b, a.0) || contains(b, a.1)
}

impl SlotSet {
    pub fn new() -> Self {
        SlotSet { all: Vec::new() }
    }

    /// `SlotSet.addSlot`: insert `[start,end]`, merging every existing slot it
    /// intersects into one. Degenerate (`start >= end`) inputs are ignored —
    /// Java throws, but the gap-producing `reverse` can legitimately emit
    /// zero-width candidates that we simply drop.
    pub fn add_slot(&mut self, start: f64, end: f64) {
        if start >= end {
            return;
        }
        let mut merged = (start, end);
        let mut keep: Vec<(f64, f64)> = Vec::with_capacity(self.all.len() + 1);
        for s in self.all.drain(..) {
            if intersects(s, merged) {
                merged = (merged.0.min(s.0), merged.1.max(s.1));
            } else {
                keep.push(s);
            }
        }
        keep.push(merged);
        self.all = keep;
    }

    /// `SlotSet.reverse`: the gaps between consecutive occupied slots (sorted by
    /// start). The result's slots are the EMPTY space.
    pub fn reverse(&self) -> SlotSet {
        let mut sorted = self.all.clone();
        sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("no NaN coordinates"));
        let mut result = SlotSet::new();
        let mut last: Option<(f64, f64)> = None;
        for s in sorted {
            if let Some(l) = last {
                result.add_slot(l.1, s.0);
            }
            last = Some(s);
        }
        result
    }

    /// `SlotSet.smaller(margin)`: drop slots no wider than `2*margin`; shrink the
    /// rest inward by `margin` on each side.
    pub fn smaller(&self, margin: f64) -> SlotSet {
        let mut result = SlotSet::new();
        for &(s, e) in &self.all {
            if (e - s) <= 2.0 * margin {
                continue;
            }
            result.add_slot(s + margin, e - margin);
        }
        result
    }

    pub fn slots(&self) -> &[(f64, f64)] {
        &self.all
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }
}

/// Port of `CompressionTransform`: a piecewise-affine map that removes a set of
/// (already shrunk) empty slots. Build it from the OCCUPIED set via
/// [`CompressionTransform::from_occupied`] — that runs the
/// `reverse().smaller(margin)` pipeline, exactly as
/// `CompressionXorYBuilder.getAffineTransform` does.
#[derive(Clone, Debug)]
pub struct CompressionTransform {
    /// The compressible empty slots (`reverse().smaller(margin)`).
    slots: Vec<(f64, f64)>,
    /// A uniform translation added after slot compression. Zero for compression
    /// transforms; non-zero only for [`CompressionTransform::translate`] (swimlane
    /// V2 per-lane x/y shifts, which reuse the coordinate-rewrite machinery).
    offset: f64,
}

impl CompressionTransform {
    /// `CompressionXorYBuilder.getAffineTransform`:
    /// `slotFinder.getSlotSet().reverse().smaller(margin)` → `CompressionTransform`.
    pub fn from_occupied(occupied: &SlotSet, margin: f64) -> Self {
        CompressionTransform {
            slots: occupied.reverse().smaller(margin).all,
            offset: 0.0,
        }
    }

    /// The identity transform (no compressible slots).
    pub fn identity() -> Self {
        CompressionTransform {
            slots: Vec::new(),
            offset: 0.0,
        }
    }

    /// A pure translation by `d` (no compression). Swimlane V2 uses this with
    /// [`rewrite_axis`] to shift a whole lane's emitted fragment into its column.
    pub fn translate(d: f64) -> Self {
        CompressionTransform {
            slots: Vec::new(),
            offset: d,
        }
    }

    pub fn is_identity(&self) -> bool {
        self.slots.is_empty() && self.offset == 0.0
    }

    /// The uniform translation added after slot compression.
    pub fn offset(&self) -> f64 {
        self.offset
    }

    /// Set the uniform post-compression translation (swimlane V2 re-anchoring).
    pub fn set_offset(&mut self, offset: f64) {
        self.offset = offset;
    }

    /// `CompressionTransform.transform`: `v - getCompressDelta(v)`, where the
    /// delta sums the sizes of every compressible slot lying left of `v` (partial
    /// for a slot that contains `v` — collapsing `v` toward the slot start). The
    /// `offset` is added after (zero for compression transforms).
    pub fn transform(&self, v: f64) -> f64 {
        let mut delta = 0.0;
        for &(s, e) in &self.slots {
            if s > v {
                continue;
            }
            if v > e {
                delta += e - s;
            } else {
                delta += v - s;
            }
        }
        v - delta + self.offset
    }
}

/// Swimlane V2: shift every X coordinate in an SVG fragment by `dx` (rects move
/// with width, ellipses keep radius, polygons/paths/lines/text all translate).
/// Reuses the compression coordinate-rewrite machinery via a pure translation.
pub fn shift_x(svg: &str, dx: f64) -> String {
    if dx == 0.0 {
        return svg.to_string();
    }
    rewrite_axis(
        svg,
        CompressionMode::OnX,
        &CompressionTransform::translate(dx),
    )
}

/// Swimlane V2: apply an arbitrary X-axis [`CompressionTransform`] to an SVG
/// fragment (used to collapse an if's inter-branch slack within a lane while
/// leaving the lane dividers/chrome at their uncompressed positions).
pub fn apply_x(svg: &str, tf: &CompressionTransform) -> String {
    if tf.is_identity() {
        return svg.to_string();
    }
    rewrite_axis(svg, CompressionMode::OnX, tf)
}

/// Swimlane V2: apply an X-axis [`CompressionTransform`] then a constant
/// `offset` (re-anchoring so the leading margin is not collapsed). Equivalent to
/// `transform(v) + offset` for every X coordinate.
pub fn apply_x_offset(svg: &str, tf: &CompressionTransform, offset: f64) -> String {
    let mut t = tf.clone();
    t.set_offset(t.offset() + offset);
    if t.is_identity() {
        return svg.to_string();
    }
    rewrite_axis(svg, CompressionMode::OnX, &t)
}

/// Swimlane V2: shift every Y coordinate in an SVG fragment by `dy` (used to drop
/// content below the lane-title band). Mirror of [`shift_x`] on the Y axis.
pub fn shift_y(svg: &str, dy: f64) -> String {
    if dy == 0.0 {
        return svg.to_string();
    }
    rewrite_axis(
        svg,
        CompressionMode::OnY,
        &CompressionTransform::translate(dy),
    )
}

/// Swimlane V2: the `[minX, maxX]` span of every drawn coordinate in an SVG
/// fragment — including `<line>`, since PlantUML's per-lane `LimitFinder` records
/// all drawn shapes for the lane `MinMax` (unlike compression, which treats flow
/// lines as transparent). `None` if the fragment has no coordinates.
pub fn x_bounds(svg: &str) -> Option<(f64, f64)> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut acc = |a: f64, b: f64| {
        lo = lo.min(a);
        hi = hi.max(b);
    };
    static RECT: OnceLock<Regex> = OnceLock::new();
    for c in re(
        r#"<rect\b[^>]*\bwidth="([-\d.]+)"[^>]*\bx="([-\d.]+)""#,
        &RECT,
    )
    .captures_iter(svg)
    {
        let (w, xx) = (num(&c[1]), num(&c[2]));
        acc(xx, xx + w);
    }
    static ELL: OnceLock<Regex> = OnceLock::new();
    for c in re(
        r#"<ellipse\b[^>]*\bcx="([-\d.]+)"[^>]*\brx="([-\d.]+)""#,
        &ELL,
    )
    .captures_iter(svg)
    {
        let (cx, rx) = (num(&c[1]), num(&c[2]));
        acc(cx - rx, cx + rx);
    }
    static POLY: OnceLock<Regex> = OnceLock::new();
    for c in re(r#"<polygon\b[^>]*\bpoints="([^"]+)""#, &POLY).captures_iter(svg) {
        if let Some((a, b)) = points_bbox(&c[1], true) {
            acc(a, b);
        }
    }
    static PATH: OnceLock<Regex> = OnceLock::new();
    for c in re(r#"<path\b[^>]*\bd="([^"]+)""#, &PATH).captures_iter(svg) {
        if let Some((a, b)) = path_bbox(&c[1], true) {
            acc(a, b);
        }
    }
    static TEXT: OnceLock<Regex> = OnceLock::new();
    for c in re(
        r#"<text\b[^>]*\btextLength="([-\d.]+)"[^>]*\bx="([-\d.]+)""#,
        &TEXT,
    )
    .captures_iter(svg)
    {
        let (tl, xx) = (num(&c[1]), num(&c[2]));
        acc(xx, xx + tl);
    }
    static LINE: OnceLock<Regex> = OnceLock::new();
    for c in re(
        r#"<line\b[^>]*\bx1="([-\d.]+)"[^>]*\bx2="([-\d.]+)""#,
        &LINE,
    )
    .captures_iter(svg)
    {
        let (x1, x2) = (num(&c[1]), num(&c[2]));
        acc(x1.min(x2), x1.max(x2));
    }
    (lo <= hi).then_some((lo, hi))
}

/// Swimlane V2: the maximum Y coordinate in an SVG fragment (bottom of content),
/// across rect y+height, ellipse cy+ry, polygon/path points, line y1/y2, text y.
/// Used to size lane dividers to the content bottom. `None` if no Y coords.
pub fn y_max(svg: &str) -> Option<f64> {
    let mut hi = f64::NEG_INFINITY;
    static RECT: OnceLock<Regex> = OnceLock::new();
    for c in re(
        r#"<rect\b[^>]*\bheight="([-\d.]+)"[^>]*\by="([-\d.]+)""#,
        &RECT,
    )
    .captures_iter(svg)
    {
        hi = hi.max(num(&c[1]) + num(&c[2]));
    }
    static ELL: OnceLock<Regex> = OnceLock::new();
    for c in re(
        r#"<ellipse\b[^>]*\bcy="([-\d.]+)"[^>]*\bry="([-\d.]+)""#,
        &ELL,
    )
    .captures_iter(svg)
    {
        hi = hi.max(num(&c[1]) + num(&c[2]));
    }
    static POLY: OnceLock<Regex> = OnceLock::new();
    for c in re(r#"<polygon\b[^>]*\bpoints="([^"]+)""#, &POLY).captures_iter(svg) {
        if let Some((_, b)) = points_bbox(&c[1], false) {
            hi = hi.max(b);
        }
    }
    static PATH: OnceLock<Regex> = OnceLock::new();
    for c in re(r#"<path\b[^>]*\bd="([^"]+)""#, &PATH).captures_iter(svg) {
        if let Some((_, b)) = path_bbox(&c[1], false) {
            hi = hi.max(b);
        }
    }
    static LINE: OnceLock<Regex> = OnceLock::new();
    for c in re(
        r#"<line\b[^>]*\by1="([-\d.]+)"[^>]*\by2="([-\d.]+)""#,
        &LINE,
    )
    .captures_iter(svg)
    {
        hi = hi.max(num(&c[1]).max(num(&c[2])));
    }
    static TEXT: OnceLock<Regex> = OnceLock::new();
    for c in re(r#"<text\b[^>]*\by="([-\d.]+)""#, &TEXT).captures_iter(svg) {
        hi = hi.max(num(&c[1]));
    }
    (hi > f64::NEG_INFINITY).then_some(hi)
}

/// A drawn primitive with enough geometry to (a) report its occupied interval on
/// each axis for the `SlotFinder` pass and (b) be re-emitted through a
/// [`CompressionTransform`]. The renderer records these alongside its SVG so the
/// whole-diagram pass can run before serialization.
///
/// `ignore_x`/`ignore_y` mark a shape transparent to compression on that axis
/// (PlantUML's `UShapeIgnorableForCompression`): flow connectors are ignorable
/// on both axes; arrowhead decorations on an ignorable worm are ignorable on X
/// only (`Worm.setCompressionMode(ON_X)`), so they still block Y compression.
#[derive(Clone, Debug)]
pub enum Prim {
    /// Axis-aligned box: `[x, x+w] × [y, y+h]`.
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    /// `[cx-rx, cx+rx] × [cy-ry, cy+ry]`.
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
    },
    /// Bounding box of its points.
    Polygon {
        points: Vec<(f64, f64)>,
        ignore_x: bool,
        ignore_y: bool,
    },
    /// A connector worm / opale path. Carries its point set for bbox + transform.
    /// Flow connectors are ignorable on both axes.
    Path {
        points: Vec<(f64, f64)>,
        ignore_x: bool,
        ignore_y: bool,
    },
    /// A straight connector segment — ignorable on both axes by default.
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        ignore_x: bool,
        ignore_y: bool,
    },
    /// Text anchored at `(x, y)` (baseline left) with measured `w` advance.
    /// `TextLimitFinder` uses the laid-out text extent; height is folded into the
    /// owning box for occupancy, so only the x-advance matters for ON_X and the
    /// glyph band `[y-ascent, y+descent]` (approximated by `h`) for ON_Y.
    Text {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
}

impl Prim {
    /// The interval this primitive occupies on `mode`'s axis, or `None` when it
    /// is ignorable there (so it neither blocks nor produces a slot).
    fn occupied(&self, mode: CompressionMode) -> Option<(f64, f64)> {
        match (self, mode) {
            (Prim::Rect { x, w, .. }, CompressionMode::OnX) => Some((*x, x + w)),
            (Prim::Rect { y, h, .. }, CompressionMode::OnY) => Some((*y, y + h)),
            (Prim::Ellipse { cx, rx, .. }, CompressionMode::OnX) => Some((cx - rx, cx + rx)),
            (Prim::Ellipse { cy, ry, .. }, CompressionMode::OnY) => Some((cy - ry, cy + ry)),
            (
                Prim::Polygon {
                    points, ignore_x, ..
                },
                CompressionMode::OnX,
            ) => (!ignore_x).then(|| bbox_axis(points, true)).flatten(),
            (
                Prim::Polygon {
                    points, ignore_y, ..
                },
                CompressionMode::OnY,
            ) => (!ignore_y).then(|| bbox_axis(points, false)).flatten(),
            (
                Prim::Path {
                    points, ignore_x, ..
                },
                CompressionMode::OnX,
            ) => (!ignore_x).then(|| bbox_axis(points, true)).flatten(),
            (
                Prim::Path {
                    points, ignore_y, ..
                },
                CompressionMode::OnY,
            ) => (!ignore_y).then(|| bbox_axis(points, false)).flatten(),
            (Prim::Line { x1, x2, ignore_x, .. }, CompressionMode::OnX) => {
                (!ignore_x).then(|| (x1.min(*x2), x1.max(*x2)))
            }
            (Prim::Line { y1, y2, ignore_y, .. }, CompressionMode::OnY) => {
                (!ignore_y).then(|| (y1.min(*y2), y1.max(*y2)))
            }
            (Prim::Text { x, w, .. }, CompressionMode::OnX) => Some((*x, x + w)),
            (Prim::Text { y, h, .. }, CompressionMode::OnY) => Some((*y, y + h)),
        }
    }

    /// Re-map this primitive's coordinates on `mode`'s axis through `tf`. Sizes
    /// derived from two coordinates (width/height) are remapped as
    /// `tf(far) - tf(near)` so edges land correctly even across a collapsed band.
    fn apply(&mut self, mode: CompressionMode, tf: &CompressionTransform) {
        match mode {
            CompressionMode::OnX => match self {
                Prim::Rect { x, w, .. } => {
                    let nx = tf.transform(*x);
                    *w = tf.transform(*x + *w) - nx;
                    *x = nx;
                }
                Prim::Ellipse { cx, rx, .. } => {
                    let l = tf.transform(*cx - *rx);
                    let r = tf.transform(*cx + *rx);
                    *cx = (l + r) / 2.0;
                    *rx = (r - l) / 2.0;
                }
                Prim::Polygon { points, .. } | Prim::Path { points, .. } => {
                    for p in points.iter_mut() {
                        p.0 = tf.transform(p.0);
                    }
                }
                Prim::Line { x1, x2, .. } => {
                    *x1 = tf.transform(*x1);
                    *x2 = tf.transform(*x2);
                }
                Prim::Text { x, w, .. } => {
                    let nx = tf.transform(*x);
                    *w = tf.transform(*x + *w) - nx;
                    *x = nx;
                }
            },
            CompressionMode::OnY => match self {
                Prim::Rect { y, h, .. } => {
                    let ny = tf.transform(*y);
                    *h = tf.transform(*y + *h) - ny;
                    *y = ny;
                }
                Prim::Ellipse { cy, ry, .. } => {
                    let t = tf.transform(*cy - *ry);
                    let b = tf.transform(*cy + *ry);
                    *cy = (t + b) / 2.0;
                    *ry = (b - t) / 2.0;
                }
                Prim::Polygon { points, .. } | Prim::Path { points, .. } => {
                    for p in points.iter_mut() {
                        p.1 = tf.transform(p.1);
                    }
                }
                Prim::Line { y1, y2, .. } => {
                    *y1 = tf.transform(*y1);
                    *y2 = tf.transform(*y2);
                }
                Prim::Text { y, h, .. } => {
                    let ny = tf.transform(*y);
                    *h = tf.transform(*y + *h) - ny;
                    *y = ny;
                }
            },
        }
    }
}

/// Bounding interval of a point set on the x- (`x_axis=true`) or y-axis.
fn bbox_axis(points: &[(f64, f64)], x_axis: bool) -> Option<(f64, f64)> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for &(px, py) in points {
        let v = if x_axis { px } else { py };
        lo = lo.min(v);
        hi = hi.max(v);
    }
    (lo <= hi).then_some((lo, hi))
}

/// Build the compression transform for one axis from the primitives' occupied
/// intervals. Pure (no mutation) so callers can inspect the transform — e.g. to
/// map the canvas dimension — before applying it.
pub fn transform_for(prims: &[Prim], mode: CompressionMode, margin: f64) -> CompressionTransform {
    let mut occ = SlotSet::new();
    for p in prims {
        if let Some((s, e)) = p.occupied(mode) {
            occ.add_slot(s, e);
        }
    }
    CompressionTransform::from_occupied(&occ, margin)
}

/// Run PlantUML's whole-diagram pass: ON_X then ON_Y (the order in
/// `ActivityDiagram3`). Mutates `prims` in place and returns the two transforms
/// so the caller can map the canvas width/height (`x_tf(width)`, `y_tf(height)`).
pub fn compress(prims: &mut [Prim], margin: f64) -> (CompressionTransform, CompressionTransform) {
    let x_tf = transform_for(prims, CompressionMode::OnX, margin);
    for p in prims.iter_mut() {
        p.apply(CompressionMode::OnX, &x_tf);
    }
    let y_tf = transform_for(prims, CompressionMode::OnY, margin);
    for p in prims.iter_mut() {
        p.apply(CompressionMode::OnY, &y_tf);
    }
    (x_tf, y_tf)
}

// ---------------------------------------------------------------------------
// Activity-diagram wiring: post-process the rendered SVG buffers.
//
// `SvgEmitter` keeps occupied shapes (`shapes`) and ignorable connectors
// (`connectors`) in separate buffers — which IS the `SlotFinder` occupied vs
// `UShapeIgnorableForCompression` classification. So we can run the faithful
// whole-diagram pass as a string post-process: read occupancy from `shapes`
// only, build the ON_X then ON_Y transforms, and rewrite every coordinate in
// BOTH buffers. Activity output uses only absolute path commands (verified
// across the golden corpus), so path `d` rewriting is well-defined.
//
// Numbers are re-emitted via the renderer's own `fmt_coord`, and an identity
// transform (no empty band wider than `2*margin`) skips its axis entirely,
// leaving the buffers byte-identical — so already-compressed (passing)
// diagrams are provably untouched.
// ---------------------------------------------------------------------------

use crate::activity::FORK_BAR_COMPRESS_MARKER;
use crate::plantuml_metrics::fmt_coord;
use regex::Regex;
use std::sync::OnceLock;

/// Whole-diagram compression of a rendered activity diagram. Returns the
/// transformed `(shapes, connectors)` buffers and the `(x, y)` transforms so the
/// caller can remap the canvas dimensions (`x_tf.transform(width)` etc.).
pub fn compress_activity_buffers(
    shapes: &str,
    connectors: &str,
    margin: f64,
) -> (String, String, CompressionTransform, CompressionTransform) {
    // ON_X occupancy = all shapes PLUS connector POLYGONS (arrowheads) and PATHS
    // (worms). PlantUML's SlotFinder records those as occupied; only `ULine` is
    // exempt (SlotFinder.draw has no ULine case). `parse_occupancy` has no <line>
    // branch, so parsing the connectors buffer adds exactly polygons + paths and
    // ignores the flow lines — matching PlantUML (verified by instrumenting
    // CompressionXorYBuilder: switch case-gaps compress 20→10, if_nested 17→10).
    let mut occ_x = parse_occupancy(shapes, CompressionMode::OnX);
    for &(s, e) in parse_occupancy(connectors, CompressionMode::OnX).slots() {
        occ_x.add_slot(s, e);
    }
    let x_tf = CompressionTransform::from_occupied(&occ_x, margin);
    let (shapes, connectors) = if x_tf.is_identity() {
        (shapes.to_string(), connectors.to_string())
    } else {
        (
            rewrite_axis(shapes, CompressionMode::OnX, &x_tf),
            rewrite_axis(connectors, CompressionMode::OnX, &x_tf),
        )
    };
    // ON_Y is deferred: it requires modelling vertical flow connectors as
    // OCCUPIED on the y-axis (they fill the inter-node space; only arrowhead
    // decorations are ignorable, and on X only). Reading occupancy from `shapes`
    // alone — correct for ON_X — wrongly collapses those connector gaps on Y.
    // Until that connector-y-occupancy model lands, ON_Y is the identity.
    let y_tf = CompressionTransform::identity();
    // Strip the internal `ignoreForCompressionOnX` sentinel before serialization
    // (emitted as `data-fork-compress=""`). It must never reach the comparator.
    let shapes = shapes
        .replace(&format!("{FORK_BAR_COMPRESS_MARKER}=\"\" "), "")
        .replace(&format!(" {FORK_BAR_COMPRESS_MARKER}=\"\""), "");
    (shapes, connectors, x_tf, y_tf)
}

fn re(pattern: &str, cell: &'static OnceLock<Regex>) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).unwrap())
}

fn num(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

/// Read the occupied 1-D intervals on `mode`'s axis from the SHAPES buffer.
fn parse_occupancy(shapes: &str, mode: CompressionMode) -> SlotSet {
    let mut occ = SlotSet::new();
    let x = mode == CompressionMode::OnX;

    static RECT: OnceLock<Regex> = OnceLock::new();
    let rect = re(
        r#"<rect\b[^>]*\bheight="([-\d.]+)"[^>]*\bwidth="([-\d.]+)"[^>]*\bx="([-\d.]+)"[^>]*\by="([-\d.]+)""#,
        &RECT,
    );
    for c in rect.captures_iter(shapes) {
        // A fork bar tagged `ignoreForCompressionOnX` (FtileBlackBlock) contributes
        // NO X-occupancy, so its reclaimable middle-gap corridor collapses and the
        // bar shrinks with it. It still BLOCKS on Y and is still remapped by
        // `rewrite_axis`.
        if x && c[0].contains(FORK_BAR_COMPRESS_MARKER) {
            continue;
        }
        if x {
            let (xx, w) = (num(&c[3]), num(&c[2]));
            occ.add_slot(xx, xx + w);
        } else {
            let (yy, h) = (num(&c[4]), num(&c[1]));
            occ.add_slot(yy, yy + h);
        }
    }

    static ELL: OnceLock<Regex> = OnceLock::new();
    let ell = re(
        r#"<ellipse\b[^>]*\bcx="([-\d.]+)"[^>]*\bcy="([-\d.]+)"[^>]*\brx="([-\d.]+)"[^>]*\bry="([-\d.]+)""#,
        &ELL,
    );
    for c in ell.captures_iter(shapes) {
        if x {
            let (cx, rx) = (num(&c[1]), num(&c[3]));
            occ.add_slot(cx - rx, cx + rx);
        } else {
            let (cy, ry) = (num(&c[2]), num(&c[4]));
            occ.add_slot(cy - ry, cy + ry);
        }
    }

    static POLY: OnceLock<Regex> = OnceLock::new();
    let poly = re(r#"<polygon\b[^>]*\bpoints="([^"]+)""#, &POLY);
    for c in poly.captures_iter(shapes) {
        if let Some((lo, hi)) = points_bbox(&c[1], x) {
            occ.add_slot(lo, hi);
        }
    }

    static PATH: OnceLock<Regex> = OnceLock::new();
    let path = re(r#"<path\b[^>]*\bd="([^"]+)""#, &PATH);
    for c in path.captures_iter(shapes) {
        if let Some((lo, hi)) = path_bbox(&c[1], x) {
            occ.add_slot(lo, hi);
        }
    }

    static TEXT: OnceLock<Regex> = OnceLock::new();
    let text = re(
        r#"<text\b[^>]*\bfont-size="([-\d.]+)"[^>]*\btextLength="([-\d.]+)"[^>]*\bx="([-\d.]+)"[^>]*\by="([-\d.]+)""#,
        &TEXT,
    );
    for c in text.captures_iter(shapes) {
        if x {
            let (xx, tl) = (num(&c[3]), num(&c[2]));
            occ.add_slot(xx, xx + tl);
        } else {
            // Glyph band around the baseline `y`: PlantUML's TextLimitFinder uses
            // the laid-out ascent/descent; approximate with the font size above
            // the baseline (descent is small and text usually sits inside a box).
            let (fs, yy) = (num(&c[1]), num(&c[4]));
            occ.add_slot(yy - fs, yy);
        }
    }

    occ
}

/// Bounding interval on the chosen axis of a `points="x,y x,y ..."` list.
fn points_bbox(points: &str, x_axis: bool) -> Option<(f64, f64)> {
    let nums: Vec<f64> = points
        .split([' ', ','])
        .filter(|t| !t.is_empty())
        .map(num)
        .collect();
    let start = if x_axis { 0 } else { 1 };
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut i = start;
    while i < nums.len() {
        lo = lo.min(nums[i]);
        hi = hi.max(nums[i]);
        i += 2;
    }
    (lo <= hi).then_some((lo, hi))
}

/// Per-command coordinate layout of an SVG path (absolute commands only):
/// returns, for the run of numeric args following a command letter, which slots
/// are x-coords (`true`), y-coords (`false`), or neither (`None`), as a repeating
/// group.
fn path_arg_axes(cmd: char) -> &'static [Option<bool>] {
    match cmd.to_ascii_uppercase() {
        'M' | 'L' | 'T' => &[Some(true), Some(false)],
        'C' => &[Some(true), Some(false), Some(true), Some(false), Some(true), Some(false)],
        'S' | 'Q' => &[Some(true), Some(false), Some(true), Some(false)],
        'A' => &[None, None, None, None, None, Some(true), Some(false)],
        'H' => &[Some(true)],
        'V' => &[Some(false)],
        _ => &[],
    }
}

/// Walk a path `d`, yielding each numeric token with whether it is an x-coord,
/// a y-coord, or neither (`None`).
fn path_coords(d: &str) -> Vec<(f64, Option<bool>)> {
    let mut out = Vec::new();
    let mut group: &'static [Option<bool>] = &[];
    let mut gi = 0usize;
    let mut tok = String::new();
    let flush = |tok: &mut String, out: &mut Vec<(f64, Option<bool>)>, group: &[Option<bool>], gi: &mut usize| {
        if tok.is_empty() {
            return;
        }
        let v = num(tok);
        tok.clear();
        let axis = if group.is_empty() {
            None
        } else {
            let a = group[*gi % group.len()];
            *gi += 1;
            a
        };
        out.push((v, axis));
    };
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            flush(&mut tok, &mut out, group, &mut gi);
            group = path_arg_axes(ch);
            gi = 0;
            out.push((f64::NAN, None)); // command-letter marker (NAN, skipped on rebuild)
        } else if ch == ',' || ch == ' ' {
            flush(&mut tok, &mut out, group, &mut gi);
        } else {
            tok.push(ch);
        }
    }
    flush(&mut tok, &mut out, group, &mut gi);
    out
}

fn path_bbox(d: &str, x_axis: bool) -> Option<(f64, f64)> {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for (v, axis) in path_coords(d) {
        if axis == Some(x_axis) {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    (lo <= hi).then_some((lo, hi))
}

/// Rewrite every coordinate on `mode`'s axis in one SVG buffer through `tf`.
fn rewrite_axis(svg: &str, mode: CompressionMode, tf: &CompressionTransform) -> String {
    let x = mode == CompressionMode::OnX;
    let mut out = svg.to_string();

    // <rect>: x/width (or y/height) move together so the far edge maps correctly.
    static RECT: OnceLock<Regex> = OnceLock::new();
    let rect = re(r#"<rect\b[^>]*?/>"#, &RECT);
    out = rect
        .replace_all(&out, |c: &regex::Captures| {
            let el = &c[0];
            if x {
                rewrite_pair(el, "x", "width", tf)
            } else {
                rewrite_pair(el, "y", "height", tf)
            }
        })
        .into_owned();

    // <ellipse>: cx/rx (or cy/ry).
    static ELL: OnceLock<Regex> = OnceLock::new();
    let ell = re(r#"<ellipse\b[^>]*?/>"#, &ELL);
    out = ell
        .replace_all(&out, |c: &regex::Captures| {
            let el = &c[0];
            let (center, radius) = if x { ("cx", "rx") } else { ("cy", "ry") };
            rewrite_center_radius(el, center, radius, tf)
        })
        .into_owned();

    // <polygon points>.
    static POLY: OnceLock<Regex> = OnceLock::new();
    let poly = re(r#"(<polygon\b[^>]*\bpoints=")([^"]+)(")"#, &POLY);
    out = poly
        .replace_all(&out, |c: &regex::Captures| {
            format!("{}{}{}", &c[1], rewrite_points(&c[2], x, tf), &c[3])
        })
        .into_owned();

    // <path d>.
    static PATH: OnceLock<Regex> = OnceLock::new();
    let path = re(r#"(<path\b[^>]*\bd=")([^"]+)(")"#, &PATH);
    out = path
        .replace_all(&out, |c: &regex::Captures| {
            format!("{}{}{}", &c[1], rewrite_path_d(&c[2], x, tf), &c[3])
        })
        .into_owned();

    // <line>: x1/x2 (or y1/y2).
    static LINE: OnceLock<Regex> = OnceLock::new();
    let line = re(r#"<line\b[^>]*?/>"#, &LINE);
    out = line
        .replace_all(&out, |c: &regex::Captures| {
            let el = &c[0];
            let (a, b) = if x { ("x1", "x2") } else { ("y1", "y2") };
            let el = rewrite_attr(el, a, tf);
            rewrite_attr(&el, b, tf)
        })
        .into_owned();

    // <text>: x/textLength on X; y on Y.
    static TEXT: OnceLock<Regex> = OnceLock::new();
    let text = re(r#"<text\b[^>]*?>"#, &TEXT);
    out = text
        .replace_all(&out, |c: &regex::Captures| {
            let el = &c[0];
            if x {
                rewrite_pair(el, "x", "textLength", tf)
            } else {
                rewrite_attr(el, "y", tf)
            }
        })
        .into_owned();

    out
}

/// Replace a single numeric attribute `name="V"` with `name="tf(V)"`.
fn rewrite_attr(el: &str, name: &str, tf: &CompressionTransform) -> String {
    let pat = format!(r#"{name}="([-\d.]+)""#);
    let rx = Regex::new(&pat).unwrap();
    rx.replace(el, |c: &regex::Captures| {
        format!(r#"{}="{}""#, name, fmt_coord(tf.transform(num(&c[1]))))
    })
    .into_owned()
}

/// Replace a `pos`/`len` attribute pair (x/width, y/height, x/textLength) so the
/// far edge `pos+len` maps through `tf` and the length stays the span between the
/// transformed edges.
fn rewrite_pair(el: &str, pos: &str, len: &str, tf: &CompressionTransform) -> String {
    let pos_v = attr_val(el, pos);
    let len_v = attr_val(el, len);
    let (Some(p), Some(l)) = (pos_v, len_v) else {
        return el.to_string();
    };
    let np = tf.transform(p);
    let nl = tf.transform(p + l) - np;
    let el = set_attr(el, pos, np);
    set_attr(&el, len, nl)
}

/// Replace a center/radius pair so `[c-r, c+r]` maps through `tf`.
fn rewrite_center_radius(el: &str, center: &str, radius: &str, tf: &CompressionTransform) -> String {
    let (Some(c), Some(r)) = (attr_val(el, center), attr_val(el, radius)) else {
        return el.to_string();
    };
    let lo = tf.transform(c - r);
    let hi = tf.transform(c + r);
    let el = set_attr(el, center, (lo + hi) / 2.0);
    set_attr(&el, radius, (hi - lo) / 2.0)
}

fn attr_val(el: &str, name: &str) -> Option<f64> {
    let rx = Regex::new(&format!(r#"\b{name}="([-\d.]+)""#)).unwrap();
    rx.captures(el).map(|c| num(&c[1]))
}

fn set_attr(el: &str, name: &str, v: f64) -> String {
    let rx = Regex::new(&format!(r#"(\b{name}=")[-\d.]+(")"#)).unwrap();
    rx.replace(el, |c: &regex::Captures| format!("{}{}{}", &c[1], fmt_coord(v), &c[2]))
        .into_owned()
}

fn rewrite_points(points: &str, x_axis: bool, tf: &CompressionTransform) -> String {
    let nums: Vec<&str> = points.split(',').collect();
    // points are "x,y,x,y,..." (comma-separated). Transform every other entry.
    let flat: Vec<f64> = nums.iter().map(|s| num(s.trim())).collect();
    let mut out: Vec<String> = Vec::with_capacity(flat.len());
    for (i, &v) in flat.iter().enumerate() {
        let is_x = i % 2 == 0;
        if is_x == x_axis {
            out.push(fmt_coord(tf.transform(v)));
        } else {
            out.push(fmt_coord(v));
        }
    }
    out.join(",")
}

fn rewrite_path_d(d: &str, x_axis: bool, tf: &CompressionTransform) -> String {
    // Char-scan that preserves every separator and command letter verbatim,
    // transforming only number tokens. `group`/`gi` track which argument slot of
    // the current command a number occupies (x, y, or neither).
    let mut out = String::with_capacity(d.len() + 16);
    let mut group: &'static [Option<bool>] = &[];
    let mut gi = 0usize;
    let mut numbuf = String::new();
    let flush = |numbuf: &mut String, out: &mut String, group: &[Option<bool>], gi: &mut usize| {
        if numbuf.is_empty() {
            return;
        }
        let v = num(numbuf);
        numbuf.clear();
        let axis = if group.is_empty() {
            None
        } else {
            let a = group[*gi % group.len()];
            *gi += 1;
            a
        };
        let nv = if axis == Some(x_axis) { tf.transform(v) } else { v };
        out.push_str(&fmt_coord(nv));
    };
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            flush(&mut numbuf, &mut out, group, &mut gi);
            group = path_arg_axes(ch);
            gi = 0;
            out.push(ch);
        } else if ch.is_ascii_digit() || ch == '.' {
            numbuf.push(ch);
        } else if ch == '-' {
            // A '-' starts a new number; flush any in-progress token first
            // (activity output uses explicit separators, so this is belt-and-braces).
            flush(&mut numbuf, &mut out, group, &mut gi);
            numbuf.push(ch);
        } else {
            // Separator (space/comma/etc.) — emit pending number, copy verbatim.
            flush(&mut numbuf, &mut out, group, &mut gi);
            out.push(ch);
        }
    }
    flush(&mut numbuf, &mut out, group, &mut gi);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: f64 = COMPRESS_MARGIN; // 5.0

    fn occ(intervals: &[(f64, f64)]) -> SlotSet {
        let mut s = SlotSet::new();
        for &(a, b) in intervals {
            s.add_slot(a, b);
        }
        s
    }

    fn compress_x_bounds_round(svg: &str) -> Option<(f64, f64)> {
        x_bounds(svg).map(|(a, b)| ((a * 1e6).round() / 1e6, (b * 1e6).round() / 1e6))
    }

    #[test]
    fn shift_x_translates_all_shape_kinds() {
        let svg = concat!(
            r#"<rect x="10" y="5" width="30" height="8"/>"#,
            r#"<ellipse cx="50" cy="9" rx="4" ry="4"/>"#,
            r#"<polygon points="60,1,70,2,60,3"/>"#,
            r#"<line x1="80" x2="90" y1="1" y2="1"/>"#,
            r#"<text x="100" y="2" textLength="12">hi</text>"#,
        );
        let out = shift_x(svg, 100.0);
        assert!(
            out.contains(r#"<rect x="110" y="5" width="30" height="8"/>"#),
            "{out}"
        );
        assert!(
            out.contains(r#"cx="150""#) && out.contains(r#"rx="4""#),
            "{out}"
        );
        assert!(out.contains(r#"points="160,1,170,2,160,3""#), "{out}");
        assert!(
            out.contains(r#"x1="180""#) && out.contains(r#"x2="190""#),
            "{out}"
        );
        assert!(
            out.contains(r#"x="200""#) && out.contains(r#"textLength="12""#),
            "{out}"
        );
    }

    #[test]
    fn shift_x_zero_is_noop() {
        let svg = r#"<rect x="10" y="5" width="30" height="8"/>"#;
        assert_eq!(shift_x(svg, 0.0), svg);
    }

    #[test]
    fn x_bounds_spans_all_kinds_incl_lines() {
        let svg = concat!(
            r#"<rect fill="x" height="8" width="30" x="10" y="5"/>"#, // [10,40]
            r#"<ellipse cx="50" cy="9" rx="4" ry="4"/>"#,             // [46,54]
            r#"<line x1="80" x2="90" y1="1" y2="1"/>"#,               // [80,90]
        );
        assert_eq!(compress_x_bounds_round(svg), Some((10.0, 90.0)));
        assert_eq!(x_bounds(""), None);
    }

    #[test]
    fn add_slot_merges_overlaps() {
        let s = occ(&[(0.0, 10.0), (5.0, 20.0), (40.0, 50.0)]);
        let mut got = s.slots().to_vec();
        got.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        assert_eq!(got, vec![(0.0, 20.0), (40.0, 50.0)]);
    }

    #[test]
    fn reverse_yields_gaps() {
        let gaps = occ(&[(0.0, 10.0), (40.0, 60.0)]).reverse();
        assert_eq!(gaps.slots(), &[(10.0, 40.0)]);
    }

    #[test]
    fn smaller_drops_narrow_and_shrinks_wide() {
        // gap of exactly 10 is dropped (<= 2*margin); gap of 30 shrinks to 20.
        let set = occ(&[(10.0, 20.0), (10.0, 50.0)]); // -> merged (10,50)
        // craft directly: two gaps via reverse
        let gaps = occ(&[(0.0, 10.0), (20.0, 30.0), (60.0, 90.0)]).reverse();
        // gaps = (10,20) size 10  and (30,60) size 30
        let sm = gaps.smaller(M);
        // (10,20) dropped; (30,60) -> (35,55)
        assert_eq!(sm.slots(), &[(35.0, 55.0)]);
        let _ = set;
    }

    #[test]
    fn band_wider_than_10_collapses_to_10() {
        // occupied clusters [0,20] and [40,60]; empty band [20,40] = 20px wide.
        let tf = CompressionTransform::from_occupied(&occ(&[(0.0, 20.0), (40.0, 60.0)]), M);
        // The 20px band collapses to 10px → everything right of it shifts -10.
        assert_eq!(tf.transform(0.0), 0.0);
        assert_eq!(tf.transform(20.0), 20.0); // left cluster edge: unchanged
        assert_eq!(tf.transform(40.0), 30.0); // right cluster edge: -10
        assert_eq!(tf.transform(60.0), 50.0);
        // a point inside the shrunk slot (25,35) clamps to its start (25).
        assert_eq!(tf.transform(30.0), 25.0);
        assert_eq!(tf.transform(25.0), 25.0);
        assert_eq!(tf.transform(35.0), 25.0);
    }

    #[test]
    fn band_exactly_10_is_untouched() {
        let tf = CompressionTransform::from_occupied(&occ(&[(0.0, 10.0), (20.0, 30.0)]), M);
        assert!(tf.is_identity());
        assert_eq!(tf.transform(30.0), 30.0);
    }

    #[test]
    fn fully_occupied_is_identity() {
        let tf = CompressionTransform::from_occupied(&occ(&[(0.0, 100.0)]), M);
        assert!(tf.is_identity());
        assert_eq!(tf.transform(50.0), 50.0);
    }

    #[test]
    fn idempotent_on_compressed_output() {
        // First pass on a drawing with a wide band.
        let prims_occ = occ(&[(0.0, 20.0), (40.0, 60.0)]);
        let tf1 = CompressionTransform::from_occupied(&prims_occ, M);
        // The compressed occupied set: [0,20] stays, [40,60] -> [30,50].
        let compressed = occ(&[
            (tf1.transform(0.0), tf1.transform(20.0)),
            (tf1.transform(40.0), tf1.transform(60.0)),
        ]);
        // Re-deriving a transform from the compressed output is the identity:
        // the remaining band is exactly 10px.
        let tf2 = CompressionTransform::from_occupied(&compressed, M);
        assert!(tf2.is_identity(), "second pass must be a no-op (idempotence)");
    }

    #[test]
    fn multiple_bands_accumulate_delta() {
        // bands [20,40] (20px) and [70,100] (30px): right-of-both shifts by
        // -10 + -20 = -30.
        let tf = CompressionTransform::from_occupied(
            &occ(&[(0.0, 20.0), (40.0, 70.0), (100.0, 120.0)]),
            M,
        );
        assert_eq!(tf.transform(120.0), 90.0);
        assert_eq!(tf.transform(70.0), 60.0); // after first band only
    }

    #[test]
    fn rect_compresses_and_keeps_width() {
        // A box at x=40 of width 20, with an occupying box at [0,20] before it,
        // leaving a 20px empty band that collapses to 10.
        let mut prims = vec![
            Prim::Rect { x: 0.0, y: 0.0, w: 20.0, h: 10.0 },
            Prim::Rect { x: 40.0, y: 0.0, w: 20.0, h: 10.0 },
        ];
        compress(&mut prims, M);
        if let Prim::Rect { x, w, .. } = prims[1] {
            assert_eq!(x, 30.0, "right box shifts left by the collapsed 10px");
            assert_eq!(w, 20.0, "width preserved");
        } else {
            panic!();
        }
    }

    #[test]
    fn connectors_are_transparent_to_compression() {
        // An ignorable connector LINE spanning the empty corridor must NOT block
        // the corridor from collapsing — boxes either side still compress.
        let mut prims = vec![
            Prim::Rect { x: 0.0, y: 0.0, w: 20.0, h: 10.0 },
            Prim::Line { x1: 20.0, y1: 5.0, x2: 40.0, y2: 5.0, ignore_x: true, ignore_y: true },
            Prim::Rect { x: 40.0, y: 0.0, w: 20.0, h: 10.0 },
        ];
        compress(&mut prims, M);
        if let Prim::Rect { x, .. } = prims[2] {
            assert_eq!(x, 30.0, "corridor collapsed despite the spanning connector");
        } else {
            panic!();
        }
        // The connector endpoints were still remapped onto the new geometry.
        if let Prim::Line { x2, .. } = prims[1] {
            assert_eq!(x2, 30.0);
        } else {
            panic!();
        }
    }

    #[test]
    fn arrowhead_ignorable_on_x_blocks_on_y() {
        let head = Prim::Polygon {
            points: vec![(100.0, 100.0), (108.0, 104.0), (100.0, 108.0)],
            ignore_x: true,
            ignore_y: false,
        };
        assert_eq!(head.occupied(CompressionMode::OnX), None);
        assert_eq!(head.occupied(CompressionMode::OnY), Some((100.0, 108.0)));
    }
}
