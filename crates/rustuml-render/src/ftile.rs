// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Faithful port of PlantUML's activity-diagram FTile layout engine
//! (`net.sourceforge.plantuml.activitydiagram3.ftile`).
//!
//! This is a *port*, not an approximation: the geometry is computed exactly as
//! PlantUML does — each tile reports an [`FtileGeometry`] (size plus the flow
//! anchors), and composites combine children through the same merge/translate
//! arithmetic. Golden pairs only *validate* this; they never define a constant.
//!
//! Java references (in `~/work/github.com/plantuml/plantuml`):
//! - `ftile/FtileGeometry.java` — the geometry record + combinators.
//! - `ftile/FtileGeometryMerger.java` — vertical (append-bottom) composition.
//! - `ftile/FtileAssemblySimple.java` — linear stacking + child translates.
//! - `ftile/vertical/FtileCircleStart.java` (SIZE 20), `FtileCircleStop.java`
//!   (SIZE 22), `FtileBox.java` (text + style padding).

/// Each tile's size and flow anchors, mirroring `FtileGeometry`.
///
/// `left` is the x of the in/out flow spine (`getPointIn`/`getPointOut` are at
/// `(left, in_y)` / `(left, out_y)`). `out_y == None` is PlantUML's
/// `Double.MIN_NORMAL` sentinel for a terminal tile (stop/end) with no outgoing
/// flow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FtileGeometry {
    pub width: f64,
    pub height: f64,
    pub left: f64,
    pub in_y: f64,
    pub out_y: Option<f64>,
}

impl FtileGeometry {
    pub fn new(width: f64, height: f64, left: f64, in_y: f64, out_y: Option<f64>) -> Self {
        Self { width, height, left, in_y, out_y }
    }

    /// `getRight()` = width − left (the extent to the right of the spine).
    pub fn right(&self) -> f64 {
        self.width - self.left
    }

    /// `FtileGeometryMerger`: stack `self` on top of `other`, aligning their
    /// flow spines (`left`). Result spine is the wider of the two; width is the
    /// max occupied span; height adds; in_y is self's; out_y is other's shifted
    /// down by self's height (terminal if other is terminal).
    pub fn append_bottom(&self, other: &FtileGeometry) -> FtileGeometry {
        let left = self.left.max(other.left);
        let dx1 = left - self.left;
        let dx2 = left - other.left;
        let width = (self.width + dx1).max(other.width + dx2);
        let height = self.height + other.height;
        let out_y = other.out_y.map(|o| o + self.height);
        FtileGeometry { width, height, left, in_y: self.in_y, out_y }
    }

    /// `addTop`: reserve `north` above; shifts in_y/out_y down.
    pub fn add_top(&self, north: f64) -> FtileGeometry {
        FtileGeometry {
            height: self.height + north,
            in_y: self.in_y + north,
            out_y: self.out_y.map(|o| o + north),
            ..*self
        }
    }

    /// `addBottom`: reserve `south` below; anchors unchanged.
    pub fn add_bottom(&self, south: f64) -> FtileGeometry {
        FtileGeometry { height: self.height + south, ..*self }
    }

    /// `incLeft`: widen on the left, moving the spine right with it.
    pub fn inc_left(&self, missing: f64) -> FtileGeometry {
        FtileGeometry { width: self.width + missing, left: self.left + missing, ..*self }
    }

    /// `incRight`: widen on the right; spine unchanged.
    pub fn inc_right(&self, missing: f64) -> FtileGeometry {
        FtileGeometry { width: self.width + missing, ..*self }
    }

    // --- Leaf tiles (vertical/*) ---

    /// `FtileCircleStart`: `CircleStart.SIZE` = 20.
    pub fn circle_start() -> Self {
        Self::new(20.0, 20.0, 10.0, 0.0, Some(20.0))
    }

    /// `FtileCircleStop`: `SIZE` = 22, terminal (no out point).
    pub fn circle_stop() -> Self {
        Self::new(22.0, 22.0, 11.0, 0.0, None)
    }

    /// `FtileCircleEnd`: the encircled-stop end marker, `SIZE` = 22, terminal.
    pub fn circle_end() -> Self {
        Self::new(22.0, 22.0, 11.0, 0.0, None)
    }

    /// `FtileBox` (action): the text block inflated by the style padding;
    /// `left` centres the spine. Caller supplies the already-measured content
    /// width/height and the per-side padding (from the resolved style).
    pub fn box_tile(text_w: f64, text_h: f64, pad_l: f64, pad_r: f64, pad_t: f64, pad_b: f64) -> Self {
        let width = text_w + pad_l + pad_r;
        let height = text_h + pad_t + pad_b;
        Self::new(width, height, width / 2.0, 0.0, Some(height))
    }
}

/// Fold a linear sequence of tiles into one geometry (`FtileAssemblySimple`
/// applied left-to-right). Returns `None` for an empty sequence.
pub fn assemble_linear(tiles: &[FtileGeometry]) -> Option<FtileGeometry> {
    let mut iter = tiles.iter();
    let first = *iter.next()?;
    Some(iter.fold(first, |acc, t| acc.append_bottom(t)))
}

/// Child translate within a linear assembly (`getTranslated1`/`getTranslated2`):
/// each child's spine aligns to the assembly spine, and the y is the cumulative
/// height of the tiles above it. Returns `(dx, dy)` per input tile.
pub fn linear_translates(tiles: &[FtileGeometry]) -> Vec<(f64, f64)> {
    let Some(asm) = assemble_linear(tiles) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(tiles.len());
    let mut y = 0.0;
    for t in tiles {
        out.push((asm.left - t.left, y));
        y += t.height;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hand-verified against FtileGeometryMerger.java.
    #[test]
    fn append_bottom_arithmetic() {
        // geo1 spine left=10 (width 20), geo2 spine left=30 (width 50).
        let g1 = FtileGeometry::new(20.0, 20.0, 10.0, 0.0, Some(20.0));
        let g2 = FtileGeometry::new(50.0, 34.0, 30.0, 0.0, Some(34.0));
        let m = g1.append_bottom(&g2);
        assert_eq!(m.left, 30.0); // max(10,30)
        // width = max(20 + (30-10), 50 + (30-30)) = max(40,50) = 50
        assert_eq!(m.width, 50.0);
        assert_eq!(m.height, 54.0); // 20+34
        assert_eq!(m.in_y, 0.0); // geo1.in_y
        assert_eq!(m.out_y, Some(54.0)); // geo2.out_y(34) + geo1.height(20)
    }

    #[test]
    fn terminal_propagates() {
        let action = FtileGeometry::box_tile(50.0, 14.0, 0.0, 0.0, 0.0, 0.0);
        let stop = FtileGeometry::circle_stop();
        let m = action.append_bottom(&stop);
        assert_eq!(m.out_y, None); // stop is terminal → assembly terminal
        assert_eq!(m.height, 14.0 + 22.0);
    }

    #[test]
    fn start_action_stop_geometry() {
        // start (20x20, left 10) → action (box) → stop (22x22, left 11).
        let action = FtileGeometry::box_tile(60.0, 14.0, 5.0, 5.0, 3.0, 3.0); // 70 x 20, left 35
        assert_eq!(action.width, 70.0);
        assert_eq!(action.left, 35.0);
        let asm = assemble_linear(&[
            FtileGeometry::circle_start(),
            action,
            FtileGeometry::circle_stop(),
        ])
        .unwrap();
        // spine = max(10, 35, 11) = 35; width = max(20+25, 70+0, 22+24) = 70.
        assert_eq!(asm.left, 35.0);
        assert_eq!(asm.width, 70.0);
        assert_eq!(asm.height, 20.0 + 20.0 + 22.0);
        assert_eq!(asm.out_y, None); // ends in stop

        let tr = linear_translates(&[
            FtileGeometry::circle_start(),
            action,
            FtileGeometry::circle_stop(),
        ]);
        // start spine 10 → dx 25 at y 0; action spine 35 → dx 0 at y 20; stop spine 11 → dx 24 at y 40.
        assert_eq!(tr, vec![(25.0, 0.0), (0.0, 20.0), (24.0, 40.0)]);
    }
}
