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
}

impl CompressionTransform {
    /// `CompressionXorYBuilder.getAffineTransform`:
    /// `slotFinder.getSlotSet().reverse().smaller(margin)` → `CompressionTransform`.
    pub fn from_occupied(occupied: &SlotSet, margin: f64) -> Self {
        CompressionTransform {
            slots: occupied.reverse().smaller(margin).all,
        }
    }

    /// The identity transform (no compressible slots).
    pub fn identity() -> Self {
        CompressionTransform { slots: Vec::new() }
    }

    pub fn is_identity(&self) -> bool {
        self.slots.is_empty()
    }

    /// `CompressionTransform.transform`: `v - getCompressDelta(v)`, where the
    /// delta sums the sizes of every compressible slot lying left of `v` (partial
    /// for a slot that contains `v` — collapsing `v` toward the slot start).
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
        v - delta
    }
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
