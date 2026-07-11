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
//! - `ftile/vertical/FtileDiamond*.java` — the test/merge diamonds.
//! - `ftile/vcompact/cond/FtileIf*`, `FtileSwitch*`, `vcompact/FtileWhile`,
//!   `FtileRepeat`, `FtileForkInner` — the composite `calculateDimensionFtile`.

/// `Hexagon.hexagonHalfSize` — the unit the FTile engine spaces everything by.
pub const HEXAGON_HALF: f64 = 12.0;

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
        Self {
            width,
            height,
            left,
            in_y,
            out_y,
        }
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
        FtileGeometry {
            width,
            height,
            left,
            in_y: self.in_y,
            out_y,
        }
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
        FtileGeometry {
            height: self.height + south,
            ..*self
        }
    }

    /// `incLeft`: widen on the left, moving the spine right with it.
    pub fn inc_left(&self, missing: f64) -> FtileGeometry {
        FtileGeometry {
            width: self.width + missing,
            left: self.left + missing,
            ..*self
        }
    }

    /// `incRight`: widen on the right; spine unchanged.
    pub fn inc_right(&self, missing: f64) -> FtileGeometry {
        FtileGeometry {
            width: self.width + missing,
            ..*self
        }
    }

    /// `incHeight`: grow height only (anchors unchanged).
    pub fn inc_height(&self, north: f64) -> FtileGeometry {
        FtileGeometry {
            height: self.height + north,
            ..*self
        }
    }

    /// `incInY`: shift the in anchor down; height and out unchanged.
    pub fn inc_in_y(&self, missing: f64) -> FtileGeometry {
        FtileGeometry {
            in_y: self.in_y + missing,
            ..*self
        }
    }

    /// `incVertically`: reserve `missing1` above the in anchor and `missing2`
    /// below; both anchors shift down by `missing1`.
    pub fn inc_vertically(&self, missing1: f64, missing2: f64) -> FtileGeometry {
        FtileGeometry {
            height: self.height + missing1 + missing2,
            in_y: self.in_y + missing1,
            out_y: self.out_y.map(|o| o + missing1),
            ..*self
        }
    }

    /// `addDim`: grow width and height, pushing the out anchor down with the
    /// extra height (the in anchor stays put).
    pub fn add_dim(&self, delta_w: f64, delta_h: f64) -> FtileGeometry {
        FtileGeometry {
            width: self.width + delta_w,
            height: self.height + delta_h,
            out_y: self.out_y.map(|o| o + delta_h),
            ..*self
        }
    }

    /// `addMarginX`: pad both sides equally; the spine recentres.
    pub fn add_margin_x(&self, margin: f64) -> FtileGeometry {
        FtileGeometry {
            width: self.width + 2.0 * margin,
            left: self.left + margin,
            ..*self
        }
    }

    /// `addMarginX(m1, m2)`: pad the two sides independently.
    pub fn add_margin_x2(&self, margin1: f64, margin2: f64) -> FtileGeometry {
        FtileGeometry {
            width: self.width + margin1 + margin2,
            left: self.left + margin1,
            ..*self
        }
    }

    /// `fixedHeight`: force the height, keeping anchors.
    pub fn fixed_height(&self, height: f64) -> FtileGeometry {
        FtileGeometry { height, ..*self }
    }

    /// `ensureHeight`: grow to `new_height` only if currently shorter.
    pub fn ensure_height(&self, new_height: f64) -> FtileGeometry {
        if self.height > new_height {
            *self
        } else {
            self.fixed_height(new_height)
        }
    }

    /// `withoutPointOut`: drop the out anchor (mark terminal).
    pub fn without_point_out(&self) -> FtileGeometry {
        FtileGeometry {
            out_y: None,
            ..*self
        }
    }

    /// `hasPointOut`.
    pub fn has_out(&self) -> bool {
        self.out_y.is_some()
    }

    /// `translate`: shift the spine/anchors by `(dx, dy)`.
    pub fn translate(&self, dx: f64, dy: f64) -> FtileGeometry {
        FtileGeometry {
            left: self.left + dx,
            in_y: self.in_y + dy,
            out_y: self.out_y.map(|o| o + dy),
            ..*self
        }
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
    pub fn box_tile(
        text_w: f64,
        text_h: f64,
        pad_l: f64,
        pad_r: f64,
        pad_t: f64,
        pad_b: f64,
    ) -> Self {
        let width = text_w + pad_l + pad_r;
        let height = text_h + pad_t + pad_b;
        Self::new(width, height, width / 2.0, 0.0, Some(height))
    }

    // --- Diamonds (vertical/FtileDiamond*) ---

    /// `FtileDiamond` (`ConditionStyle.EMPTY_DIAMOND`): a bare 24×24 lozenge
    /// with a `north` label reserved *above* the in anchor. `north_h` is the
    /// measured height of the north text (0 when there is none).
    pub fn diamond_empty(north_h: f64) -> Self {
        let w = HEXAGON_HALF * 2.0;
        let h = HEXAGON_HALF * 2.0 + north_h;
        Self::new(w, h, w / 2.0, north_h, Some(h))
    }

    /// The "alone" diamond box shared by `FtileDiamondInside`/`Inside2`:
    /// the label clamped to at least 24×24 then widened by 24 (hexagon point
    /// insets). Empty label collapses to 24×24.
    fn diamond_alone(label_w: f64, label_h: f64) -> Self {
        let (w, h) = if label_w == 0.0 || label_h == 0.0 {
            (HEXAGON_HALF * 2.0, HEXAGON_HALF * 2.0)
        } else {
            (
                label_w.max(HEXAGON_HALF * 2.0) + HEXAGON_HALF * 2.0,
                label_h.max(HEXAGON_HALF * 2.0),
            )
        };
        Self::new(w, h, w / 2.0, 0.0, Some(h))
    }

    /// `FtileDiamondInside` (`ConditionStyle.INSIDE_HEXAGON`): the alone box
    /// with the `north` label height appended at the bottom (the out anchor
    /// stays at the diamond's own bottom). `north_h` is the north text height.
    pub fn diamond_inside(label_w: f64, label_h: f64, north_h: f64) -> Self {
        Self::diamond_alone(label_w, label_h).inc_height(north_h)
    }

    /// `FtileDiamondInside2`: like `inside`, but the north label can also widen
    /// the tile past the diamond when `north_w > left`.
    pub fn diamond_inside2(label_w: f64, label_h: f64, north_w: f64, north_h: f64) -> Self {
        let diamond = Self::diamond_alone(label_w, label_h);
        let height = diamond.height + north_h;
        let left = diamond.width / 2.0;
        let width = if north_w > left {
            left + north_w
        } else {
            diamond.width
        };
        Self::new(width, height, left, 0.0, Some(diamond.height))
    }

    /// `FtileDiamondSquare` (`ConditionStyle.INSIDE_DIAMOND`): the label padded
    /// by 24 on each axis (or a bare 24×24 when empty).
    pub fn diamond_square(label_w: f64, label_h: f64) -> Self {
        let (w, h) = if label_w == 0.0 || label_h == 0.0 {
            (HEXAGON_HALF * 2.0, HEXAGON_HALF * 2.0)
        } else {
            (label_w + HEXAGON_HALF * 2.0, label_h + HEXAGON_HALF * 2.0)
        };
        Self::new(w, h, w / 2.0, 0.0, Some(h))
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

// --- Composite tiles (calculateDimensionFtile) ---

/// `FtileIfWithDiamonds.calculateDimensionInternalSlow` + the `FtileIfNude`
/// terminal-strip rule. `diamond1` is the test diamond (top), `diamond2` the
/// merge diamond (bottom), `t1`/`t2` the two branch tiles laid side by side.
///
/// `delta_h` is the extra vertical reserve `getYdelta1a + getYdelta1b +
/// getYdeltaForLabels` (the gap above branches, the gap below the test, and the
/// merge diamond's west/east label height) — the dimension only uses their sum.
/// `note` carries the opale-note deltas (`xDeltaNote`, `yDeltaNote`,
/// `suppWidthNode`) and is `(0,0,0)` when there are no positioned notes.
pub fn if_with_diamonds(
    diamond1: &FtileGeometry,
    t1: &FtileGeometry,
    t2: &FtileGeometry,
    diamond2: &FtileGeometry,
    delta_h: f64,
    note: (f64, f64, f64),
) -> FtileGeometry {
    let (x_delta_note, y_delta_note, supp_width_node) = note;
    const SUPP_WIDTH: f64 = 20.0;

    // FtileIfNude.calculateDimensionInternalSlow with the with-diamonds
    // widthInner override (clamp to diamond1.width + SUPP_WIDTH).
    let inner_margin = (t1.right() + t2.left).max(diamond1.width + SUPP_WIDTH);
    let width = x_delta_note + t1.left + inner_margin + t2.right() + supp_width_node;
    let dim12_h = t1.height.max(t2.height); // XDimension2D.mergeLR height
    let nude_h = y_delta_note + dim12_h;
    let nude = FtileGeometry::new(
        width,
        nude_h,
        x_delta_note + t1.left + inner_margin / 2.0,
        y_delta_note,
        Some(nude_h),
    );

    let all = diamond1.append_bottom(&nude).append_bottom(diamond2);
    let result = all.add_dim(0.0, delta_h).inc_in_y(y_delta_note);

    // FtileIfNude.calculateDimensionFtile: terminal unless a branch flows out.
    if t1.has_out() || t2.has_out() {
        result
    } else {
        result.without_point_out()
    }
}

/// `FtileWhile.calculateDimensionFtile`. `diamond1` is the test diamond,
/// `while_block` the loop body, `backward` the optional explicit backward tile,
/// `special_out` the optional `kill`/`detach` out tile (only its width matters),
/// and `supp_label_h` the height of the backward-edge label (`back1`).
pub fn while_tile(
    diamond1: &FtileGeometry,
    while_block: &FtileGeometry,
    backward: Option<&FtileGeometry>,
    special_out: Option<&FtileGeometry>,
    supp_label_h: f64,
) -> FtileGeometry {
    let geo = diamond1.append_bottom(while_block);
    let height = geo.height + 4.0 * HEXAGON_HALF + supp_label_h;
    let dx = 2.0 * HEXAGON_HALF;
    let backward_w = backward.map_or(0.0, |b| b.width);
    let x_delta_special = special_out.map_or(0.0, |s| s.width);
    FtileGeometry::new(
        x_delta_special + geo.width + dx + HEXAGON_HALF + backward_w,
        height,
        x_delta_special + geo.left + dx,
        diamond1.in_y,
        Some(height),
    )
}

/// `FtileRepeat.calculateDimensionInternal` + `calculateDimensionFtile`.
/// `diamond1`/`diamond2` are the start/back diamonds, `repeat` the body,
/// `test_label_w` the width of the loop test label, `backward` the optional
/// explicit backward tile.
pub fn repeat_tile(
    diamond1: &FtileGeometry,
    diamond2: &FtileGeometry,
    repeat: &FtileGeometry,
    test_label_w: f64,
    backward: Option<&FtileGeometry>,
) -> FtileGeometry {
    let half_max = (diamond1.width / 2.0).max(diamond2.width / 2.0);
    let get_left = repeat.left.max(half_max);
    let get_right = repeat.right().max(half_max);

    let mut width = (get_left + get_right).max(test_label_w + 2.0 * HEXAGON_HALF);
    if let Some(b) = backward {
        width += b.width;
    }
    let height = diamond1.height + repeat.height + diamond2.height + 8.0 * HEXAGON_HALF;
    FtileGeometry::new(
        width + 2.0 * HEXAGON_HALF,
        height,
        get_left,
        0.0,
        Some(height),
    )
}

/// `FtileForkInner.calculateDimensionFtile`: the branches laid side by side
/// (sum of widths, max height). The fork/join bars are added by the parallel
/// builder wrapping and belong to the drawing increment.
pub fn fork_inner(forks: &[FtileGeometry]) -> Option<FtileGeometry> {
    if forks.is_empty() {
        return None;
    }
    let width: f64 = forks.iter().map(|f| f.width).sum();
    let height = forks.iter().map(|f| f.height).fold(0.0_f64, f64::max);
    Some(FtileGeometry::new(
        width,
        height,
        width / 2.0,
        0.0,
        Some(height),
    ))
}

/// `FtileSwitchNude.calculateDimensionInternalSlow`: branch tiles laid
/// left-to-right, separated by `x_separation` (20), padded 100 tall. Always
/// terminal (no out anchor), per the 4-arg `FtileGeometry` constructor.
pub fn switch_nude(tiles: &[FtileGeometry], x_separation: f64) -> Option<FtileGeometry> {
    if tiles.is_empty() {
        return None;
    }
    let width: f64 =
        tiles.iter().map(|t| t.width).sum::<f64>() + x_separation * (tiles.len() as f64 - 1.0);
    let height = tiles.iter().map(|t| t.height).fold(0.0_f64, f64::max) + 100.0;
    Some(FtileGeometry::new(width, height, width / 2.0, 0.0, None))
}

/// `FtileSwitchWithDiamonds.calculateDimensionInternalSlow`. Picks BIG vs SMALL
/// diamond mode from `w13` vs `w9` (the middle-tiles width), then composes.
/// `diamond1`/`diamond2` are the test/merge diamonds. `tiles` must be non-empty.
pub fn switch_with_diamonds(
    diamond1: &FtileGeometry,
    diamond2: &FtileGeometry,
    tiles: &[FtileGeometry],
    x_separation: f64,
) -> FtileGeometry {
    const SUPP15: f64 = 15.0;
    const Y_DELTA_1A: f64 = 20.0;
    const Y_DELTA_1B: f64 = 10.0;

    let nude = switch_nude(tiles, x_separation).expect("switch needs at least one branch");
    let first = &tiles[0];
    let last = &tiles[tiles.len() - 1];

    let w13 = diamond1.width - first.right() - last.left;
    // Middle tiles (all but first/last); empty for ≤2 branches. `get` avoids a
    // `1..0` slice panic when there is a single branch.
    let w9: f64 = tiles
        .get(1..tiles.len().saturating_sub(1))
        .unwrap_or(&[])
        .iter()
        .map(|t| t.width)
        .sum();

    if w13 > w9 {
        // BIG_DIAMOND
        let height = diamond1.height + nude.height + diamond2.height + Y_DELTA_1A + Y_DELTA_1B;
        let width = first.width + SUPP15 + w13 + SUPP15 + last.width;
        FtileGeometry::new(
            width,
            height,
            first.left + SUPP15 + diamond1.left,
            0.0,
            Some(height),
        )
    } else {
        // SMALL_DIAMOND
        let all = diamond1.append_bottom(&nude).append_bottom(diamond2);
        all.add_dim(0.0, Y_DELTA_1A + Y_DELTA_1B)
    }
}

// --- Child translates (getTranslateFor*) ---
//
// Each composite positions its children relative to its own spine. These
// mirror the Java `getTranslateFor*` helpers exactly. They read only the
// composite's width/height/left, so the (un-stripped) internal geometry and the
// final terminal-stripped geometry are interchangeable here. Offsets are
// `(dx, dy)` relative to the composite's top-left.

/// Child offsets within an `FtileWhile`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WhileLayout {
    pub diamond1: (f64, f64),
    pub body: (f64, f64),
    pub backward: Option<(f64, f64)>,
    pub special: Option<(f64, f64)>,
}

/// `FtileWhile.getTranslateFor*`. `total` is the composite geometry from
/// [`while_tile`]; `supp_label_h` is the same backward-label height passed there.
pub fn while_layout(
    total: &FtileGeometry,
    diamond1: &FtileGeometry,
    body: &FtileGeometry,
    backward: Option<&FtileGeometry>,
    special: Option<&FtileGeometry>,
    supp_label_h: f64,
) -> WhileLayout {
    let diamond1_t = (total.left - diamond1.left, 0.0);
    let body_y =
        diamond1.height + (total.height - diamond1.height - body.height - supp_label_h) / 2.0;
    let body_t = (total.left - body.left, body_y);
    let backward_t = backward.map(|b| (total.width - b.width, (total.height - b.height) / 2.0));
    let special_t = special.map(|s| {
        let half = (diamond1
            .out_y
            .expect("while test diamond has an out anchor")
            - diamond1.in_y)
            / 2.0;
        let y1 = (3.0 * half).max(4.0 * HEXAGON_HALF);
        let x_while = body_t.0 - HEXAGON_HALF;
        let x_diamond = diamond1_t.0;
        (x_while.min(x_diamond) - s.width, y1)
    });
    WhileLayout {
        diamond1: diamond1_t,
        body: body_t,
        backward: backward_t,
        special: special_t,
    }
}

/// Child offsets within an `FtileRepeat`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RepeatLayout {
    pub diamond1: (f64, f64),
    pub diamond2: (f64, f64),
    pub repeat: (f64, f64),
    pub backward: Option<(f64, f64)>,
}

/// `FtileRepeat.getTranslateFor*`. `total` is the composite from [`repeat_tile`].
pub fn repeat_layout(
    total: &FtileGeometry,
    diamond1: &FtileGeometry,
    diamond2: &FtileGeometry,
    repeat: &FtileGeometry,
    backward: Option<&FtileGeometry>,
) -> RepeatLayout {
    let space = total.height - diamond1.height - diamond2.height - repeat.height;
    let repeat_t = (total.left - repeat.left, diamond1.height + space / 2.0);
    let diamond1_t = (total.left - diamond1.width / 2.0, 0.0);
    let diamond2_t = (
        total.left - diamond2.width / 2.0,
        total.height - diamond2.height,
    );
    let backward_t = backward.map(|b| (total.width - b.width, (total.height - b.height) / 2.0));
    RepeatLayout {
        diamond1: diamond1_t,
        diamond2: diamond2_t,
        repeat: repeat_t,
        backward: backward_t,
    }
}

/// Child offsets within an `FtileIfWithDiamonds`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IfLayout {
    pub diamond1: (f64, f64),
    pub diamond2: (f64, f64),
    pub branch1: (f64, f64),
    pub branch2: (f64, f64),
}

/// `FtileIfWithDiamonds.getTranslate*` (over `FtileIfNude`'s branch translates).
/// `total` is the composite from [`if_with_diamonds`]; `y_delta_1a` is
/// `getYdelta1a` (the gap below the test diamond); `note` is the same opale-note
/// tuple `(xDeltaNote, yDeltaNote, suppWidthNode)` passed there.
pub fn if_layout(
    total: &FtileGeometry,
    diamond1: &FtileGeometry,
    diamond2: &FtileGeometry,
    tile2: &FtileGeometry,
    y_delta_1a: f64,
    note: (f64, f64, f64),
) -> IfLayout {
    let (x_delta_note, y_delta_note, supp_width_node) = note;
    let branch_y = y_delta_note + diamond1.height + y_delta_1a;
    IfLayout {
        diamond1: (total.left - diamond1.left, y_delta_note),
        diamond2: (
            total.left - diamond2.width / 2.0,
            total.height - diamond2.height,
        ),
        branch1: (x_delta_note, branch_y),
        branch2: (total.width - tile2.width - supp_width_node, branch_y),
    }
}

/// `FtileForkInner.getTranslateFor`: branches laid left-to-right at the running
/// x offset (each at y = 0). The bar wrapping shifts these in the drawing layer.
pub fn fork_inner_translates(forks: &[FtileGeometry]) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(forks.len());
    let mut x = 0.0;
    for f in forks {
        out.push((x, 0.0));
        x += f.width;
    }
    out
}

/// Child offsets within an `FtileSwitchWithDiamonds`. `tiles` is one `(dx, dy)`
/// per branch, in branch order; `diamond2` is positioned for drawing only when
/// the composite still has an out anchor (`total.has_out()`).
#[derive(Clone, Debug, PartialEq)]
pub struct SwitchLayout {
    pub diamond1: (f64, f64),
    pub diamond2: (f64, f64),
    pub tiles: Vec<(f64, f64)>,
}

/// `FtileSwitchWithDiamonds.getTranslate*` (over `FtileSwitchNude.getTranslateNude`).
/// `total` is the composite from [`switch_with_diamonds`]; `tiles`/`x_separation`
/// must match what was passed there so the BIG/SMALL mode is recomputed
/// identically. All branch tiles share the y `diamond1.height + ydelta1a(20)`
/// (`getTranslateMain` is dy-only); the modes differ only in the per-branch dx.
pub fn switch_layout(
    total: &FtileGeometry,
    diamond1: &FtileGeometry,
    diamond2: &FtileGeometry,
    tiles: &[FtileGeometry],
    x_separation: f64,
) -> SwitchLayout {
    const SUPP15: f64 = 15.0;
    const Y_DELTA_1A: f64 = 20.0;

    let n = tiles.len();
    let first = &tiles[0];
    let last = &tiles[n - 1];

    // Mode discriminant, recomputed exactly as switch_with_diamonds.
    let w13 = diamond1.width - first.right() - last.left;
    // `get` avoids a `1..0` slice panic for a single branch (no middle tiles).
    let w9: f64 = tiles
        .get(1..n.saturating_sub(1))
        .unwrap_or(&[])
        .iter()
        .map(|t| t.width)
        .sum();
    let big = w13 > w9;

    let dy = diamond1.height + Y_DELTA_1A; // getTranslateMain (dy only)
    let mut tiles_t = Vec::with_capacity(n);
    if big {
        // suppx is unused for n == 1 (only the last-tile branch runs); guard the
        // divide so we don't synthesise an inf/NaN that Java never materialises.
        let suppx = if n > 1 {
            (w13 - w9) / (n as f64 - 1.0)
        } else {
            0.0
        };
        let mut dx = 0.0;
        for (i, t) in tiles.iter().enumerate() {
            if i == n - 1 {
                tiles_t.push((first.width + w13 + 2.0 * SUPP15, dy));
            } else {
                tiles_t.push((dx, dy));
                dx += t.width + suppx;
            }
        }
    } else {
        let mut dx = 0.0;
        for t in tiles {
            tiles_t.push((dx, dy));
            dx += t.width + x_separation;
        }
    }

    SwitchLayout {
        diamond1: (total.left - diamond1.left, 0.0),
        diamond2: (
            total.left - diamond2.width / 2.0,
            total.height - diamond2.height,
        ),
        tiles: tiles_t,
    }
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

    // --- Diamonds (hand-verified vs FtileDiamond*.java) ---

    #[test]
    fn diamond_empty_reserves_north_above() {
        assert_eq!(
            FtileGeometry::diamond_empty(0.0),
            FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0))
        );
        // north_h reserved above the in anchor (inY = north_h).
        assert_eq!(
            FtileGeometry::diamond_empty(14.0),
            FtileGeometry::new(24.0, 38.0, 12.0, 14.0, Some(38.0))
        );
    }

    #[test]
    fn diamond_inside_alone_and_north() {
        // label 30×14 → alone max(30,24)+24 = 54 wide, max(14,24) = 24 tall; +north 10 below.
        assert_eq!(
            FtileGeometry::diamond_inside(30.0, 14.0, 10.0),
            FtileGeometry::new(54.0, 34.0, 27.0, 0.0, Some(24.0))
        );
        // empty label collapses to 24×24.
        assert_eq!(
            FtileGeometry::diamond_inside(0.0, 0.0, 0.0),
            FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0))
        );
    }

    #[test]
    fn diamond_square_pads_both_axes() {
        assert_eq!(
            FtileGeometry::diamond_square(40.0, 16.0),
            FtileGeometry::new(64.0, 40.0, 32.0, 0.0, Some(40.0))
        );
    }

    #[test]
    fn diamond_inside2_north_can_widen() {
        // alone 54×24, left 27; north 80 wide > 27 → width 27+80 = 107, out at diamond bottom 24.
        assert_eq!(
            FtileGeometry::diamond_inside2(30.0, 14.0, 80.0, 10.0),
            FtileGeometry::new(107.0, 34.0, 27.0, 0.0, Some(24.0))
        );
    }

    // --- Composites (hand-verified vs the calculateDimension* methods) ---

    #[test]
    fn if_with_diamonds_geometry() {
        let diamond1 = FtileGeometry::new(50.0, 30.0, 25.0, 0.0, Some(30.0));
        let t1 = FtileGeometry::new(40.0, 20.0, 20.0, 0.0, Some(20.0));
        let t2 = FtileGeometry::new(60.0, 24.0, 30.0, 0.0, Some(24.0));
        let diamond2 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let g = if_with_diamonds(&diamond1, &t1, &t2, &diamond2, 16.0, (0.0, 0.0, 0.0));
        // inner_margin = max(t1.right 20 + t2.left 30, d1.w 50 + 20) = 70; width = 20+70+30 = 120.
        // stack d1 ⊕ nude(120 tall 24 spine 55) ⊕ d2 → 78 tall, then +16 delta → 94.
        assert_eq!(g, FtileGeometry::new(120.0, 94.0, 55.0, 0.0, Some(94.0)));
    }

    #[test]
    fn if_with_diamonds_terminal_when_both_branches_terminal() {
        let diamond1 = FtileGeometry::new(50.0, 30.0, 25.0, 0.0, Some(30.0));
        let t1 = FtileGeometry::new(40.0, 20.0, 20.0, 0.0, None); // both branches end (e.g. stop)
        let t2 = FtileGeometry::new(60.0, 24.0, 30.0, 0.0, None);
        let diamond2 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let g = if_with_diamonds(&diamond1, &t1, &t2, &diamond2, 16.0, (0.0, 0.0, 0.0));
        assert_eq!(g.out_y, None);
    }

    #[test]
    fn while_tile_geometry() {
        let diamond1 = FtileGeometry::new(40.0, 30.0, 20.0, 8.0, Some(30.0));
        let while_block = FtileGeometry::new(60.0, 50.0, 30.0, 0.0, Some(50.0));
        let g = while_tile(&diamond1, &while_block, None, None, 12.0);
        // geo = d1 ⊕ body = 60×80 spine 30 inY 8; height = 80 + 48 + 12 = 140;
        // width = 60 + 24 + 12 = 96; left = 30 + 24 = 54.
        assert_eq!(g, FtileGeometry::new(96.0, 140.0, 54.0, 8.0, Some(140.0)));
    }

    #[test]
    fn repeat_tile_geometry() {
        let d1 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let d2 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let repeat = FtileGeometry::new(50.0, 40.0, 25.0, 0.0, Some(40.0));
        let g = repeat_tile(&d1, &d2, &repeat, 10.0, None);
        // get_left = max(25,12) = 25; get_right = max(25,12) = 25; width = max(50, 34) = 50;
        // height = 24+40+24+96 = 184; width += 24 → 74.
        assert_eq!(g, FtileGeometry::new(74.0, 184.0, 25.0, 0.0, Some(184.0)));
    }

    #[test]
    fn fork_inner_side_by_side() {
        let forks = [
            FtileGeometry::new(40.0, 30.0, 20.0, 0.0, Some(30.0)),
            FtileGeometry::new(60.0, 50.0, 30.0, 0.0, Some(50.0)),
        ];
        assert_eq!(
            fork_inner(&forks),
            Some(FtileGeometry::new(100.0, 50.0, 50.0, 0.0, Some(50.0)))
        );
        assert_eq!(fork_inner(&[]), None);
    }

    #[test]
    fn switch_nude_is_terminal() {
        let tiles = [
            FtileGeometry::new(40.0, 30.0, 20.0, 0.0, Some(30.0)),
            FtileGeometry::new(50.0, 40.0, 25.0, 0.0, Some(40.0)),
            FtileGeometry::new(30.0, 20.0, 15.0, 0.0, Some(20.0)),
        ];
        // width = 120 + 20*2 = 160; height = 40 + 100 = 140; out None.
        assert_eq!(
            switch_nude(&tiles, 20.0),
            Some(FtileGeometry::new(160.0, 140.0, 80.0, 0.0, None))
        );
    }

    #[test]
    fn switch_small_diamond_mode() {
        let d1 = FtileGeometry::new(60.0, 30.0, 30.0, 0.0, Some(30.0));
        let d2 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let tiles = [
            FtileGeometry::new(40.0, 30.0, 20.0, 0.0, Some(30.0)),
            FtileGeometry::new(50.0, 40.0, 25.0, 0.0, Some(40.0)),
            FtileGeometry::new(30.0, 20.0, 15.0, 0.0, Some(20.0)),
        ];
        // w13 = 60 - 20 - 15 = 25; w9 = 50; 25 <= 50 → SMALL.
        let g = switch_with_diamonds(&d1, &d2, &tiles, 20.0);
        assert_eq!(g, FtileGeometry::new(160.0, 224.0, 80.0, 0.0, Some(224.0)));
    }

    #[test]
    fn switch_big_diamond_mode() {
        let d1 = FtileGeometry::new(200.0, 30.0, 100.0, 0.0, Some(30.0));
        let d2 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let tiles = [
            FtileGeometry::new(40.0, 30.0, 20.0, 0.0, Some(30.0)),
            FtileGeometry::new(50.0, 40.0, 25.0, 0.0, Some(40.0)),
            FtileGeometry::new(30.0, 20.0, 15.0, 0.0, Some(20.0)),
        ];
        // w13 = 200 - 20 - 15 = 165; w9 = 50; 165 > 50 → BIG.
        // height = 30 + 140 + 24 + 20 + 10 = 224; width = 40 + 15 + 165 + 15 + 30 = 265;
        // left = 20 + 15 + 100 = 135.
        let g = switch_with_diamonds(&d1, &d2, &tiles, 20.0);
        assert_eq!(g, FtileGeometry::new(265.0, 224.0, 135.0, 0.0, Some(224.0)));
    }

    // --- Child translates (hand-verified vs getTranslateFor*) ---

    #[test]
    fn while_layout_centres_body_below_diamond() {
        let diamond1 = FtileGeometry::new(40.0, 30.0, 20.0, 8.0, Some(30.0));
        let body = FtileGeometry::new(60.0, 50.0, 30.0, 0.0, Some(50.0));
        let total = while_tile(&diamond1, &body, None, None, 12.0); // (96,140,54,8)
        let l = while_layout(&total, &diamond1, &body, None, None, 12.0);
        // diamond1 spine to total spine: 54-20 = 34 at y 0.
        assert_eq!(l.diamond1, (34.0, 0.0));
        // body y = 30 + (140-30-50-12)/2 = 30+24 = 54; x = 54-30 = 24.
        assert_eq!(l.body, (24.0, 54.0));
        assert_eq!(l.backward, None);
        assert_eq!(l.special, None);
    }

    #[test]
    fn repeat_layout_positions_diamonds_and_body() {
        let d1 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let d2 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let repeat = FtileGeometry::new(50.0, 40.0, 25.0, 0.0, Some(40.0));
        let total = repeat_tile(&d1, &d2, &repeat, 10.0, None); // (74,184,25,0)
        let l = repeat_layout(&total, &d1, &d2, &repeat, None);
        // space = 184-24-24-40 = 96; body y = 24+48 = 72; x = 25-25 = 0.
        assert_eq!(l.repeat, (0.0, 72.0));
        assert_eq!(l.diamond1, (13.0, 0.0)); // 25 - 12
        assert_eq!(l.diamond2, (13.0, 160.0)); // 25 - 12, 184 - 24
        assert_eq!(l.backward, None);
    }

    #[test]
    fn if_layout_places_branches_and_diamonds() {
        let diamond1 = FtileGeometry::new(50.0, 30.0, 25.0, 0.0, Some(30.0));
        let t1 = FtileGeometry::new(40.0, 20.0, 20.0, 0.0, Some(20.0));
        let t2 = FtileGeometry::new(60.0, 24.0, 30.0, 0.0, Some(24.0));
        let diamond2 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let total = if_with_diamonds(&diamond1, &t1, &t2, &diamond2, 16.0, (0.0, 0.0, 0.0)); // (120,94,55)
        let l = if_layout(&total, &diamond1, &diamond2, &t2, 10.0, (0.0, 0.0, 0.0));
        assert_eq!(l.diamond1, (30.0, 0.0)); // 55 - 25
        assert_eq!(l.diamond2, (43.0, 70.0)); // 55 - 12, 94 - 24
        assert_eq!(l.branch1, (0.0, 40.0)); // 0, 0 + 30 + 10
        assert_eq!(l.branch2, (60.0, 40.0)); // 120 - 60 - 0, 40
    }

    #[test]
    fn fork_translates_accumulate_x() {
        let forks = [
            FtileGeometry::new(40.0, 30.0, 20.0, 0.0, Some(30.0)),
            FtileGeometry::new(60.0, 50.0, 30.0, 0.0, Some(50.0)),
        ];
        assert_eq!(fork_inner_translates(&forks), vec![(0.0, 0.0), (40.0, 0.0)]);
    }

    #[test]
    fn switch_layout_small_diamond_lays_branches_with_separation() {
        // Same fixture as switch_small_diamond_mode: total (160,224,80).
        let d1 = FtileGeometry::new(60.0, 30.0, 30.0, 0.0, Some(30.0));
        let d2 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let tiles = [
            FtileGeometry::new(40.0, 30.0, 20.0, 0.0, Some(30.0)),
            FtileGeometry::new(50.0, 40.0, 25.0, 0.0, Some(40.0)),
            FtileGeometry::new(30.0, 20.0, 15.0, 0.0, Some(20.0)),
        ];
        let total = switch_with_diamonds(&d1, &d2, &tiles, 20.0); // (160,224,80)
        let l = switch_layout(&total, &d1, &d2, &tiles, 20.0);
        // dy = d1.height 30 + ydelta1a 20 = 50; nude dx = cumulative width+20.
        assert_eq!(l.tiles, vec![(0.0, 50.0), (60.0, 50.0), (130.0, 50.0)]);
        assert_eq!(l.diamond1, (50.0, 0.0)); // 80 - 30
        assert_eq!(l.diamond2, (68.0, 200.0)); // 80 - 12, 224 - 24
        // Last branch fills the canvas: 130 + 30 = 160 = total width.
        assert_eq!(l.tiles[2].0 + tiles[2].width, total.width);
    }

    #[test]
    fn switch_single_branch_does_not_panic() {
        // A switch with one case has no middle tiles; the w9 slice must not
        // panic (`tiles[1..0]`).
        let d1 = FtileGeometry::new(60.0, 30.0, 30.0, 0.0, Some(30.0));
        let d2 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let tiles = [FtileGeometry::new(40.0, 30.0, 20.0, 0.0, Some(30.0))];
        let g = switch_with_diamonds(&d1, &d2, &tiles, 20.0);
        let l = switch_layout(&g, &d1, &d2, &tiles, 20.0);
        assert_eq!(l.tiles.len(), 1);
    }

    #[test]
    fn switch_layout_big_diamond_distributes_supp_gap() {
        // Same fixture as switch_big_diamond_mode: total (265,224,135).
        let d1 = FtileGeometry::new(200.0, 30.0, 100.0, 0.0, Some(30.0));
        let d2 = FtileGeometry::new(24.0, 24.0, 12.0, 0.0, Some(24.0));
        let tiles = [
            FtileGeometry::new(40.0, 30.0, 20.0, 0.0, Some(30.0)),
            FtileGeometry::new(50.0, 40.0, 25.0, 0.0, Some(40.0)),
            FtileGeometry::new(30.0, 20.0, 15.0, 0.0, Some(20.0)),
        ];
        let total = switch_with_diamonds(&d1, &d2, &tiles, 20.0); // (265,224,135)
        let l = switch_layout(&total, &d1, &d2, &tiles, 20.0);
        // w13 = 165, w9 = 50, suppx = (165-50)/2 = 57.5; dy = 50.
        // tile0 dx 0; tile1 dx = 40 + 57.5 = 97.5; last tile dx9 = 40 + 165 + 30 = 235.
        assert_eq!(l.tiles, vec![(0.0, 50.0), (97.5, 50.0), (235.0, 50.0)]);
        assert_eq!(l.diamond1, (35.0, 0.0)); // 135 - 100
        assert_eq!(l.diamond2, (123.0, 200.0)); // 135 - 12, 224 - 24
        // Last branch reaches the right edge: 235 + 30 = 265 = total width.
        assert_eq!(l.tiles[2].0 + tiles[2].width, total.width);
    }
}
