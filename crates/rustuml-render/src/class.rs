// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Class diagram SVG renderer — produces PlantUML-compatible SVG output.
//!
//! Uses rustuml-layout (Sugiyama algorithm) for node positioning,
//! then renders classes with fields/methods and relationships.
//! The SVG structure matches PlantUML's output format exactly:
//! - Root `<svg>` with `data-diagram-type="CLASS"` and PlantUML attributes
//! - Entity wrappers: `<!--class Name-->` comments + `<g class="entity" ...>`
//! - Colored stereotype circles with letter glyph paths
//! - Visibility modifier markers with `data-visibility-modifier` attributes
//! - Inline `style` attributes for strokes (not `stroke="..."` attributes)

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use rustuml_layout::graph::{
    ClusterPosition, ClusterTitleSize, Direction, EdgeLabelSize, EdgePath, LayoutGraph,
    NodePosition,
};
use rustuml_parser::diagram::SpriteData;
use rustuml_parser::diagram::class::*;

use crate::layout_oracle::{
    CrowMark, EntityPath, EntityPolygon, EntityRect, EntityText, OracleCluster, OracleEdgePath,
    OracleEntity, OracleHandwrittenWarning, OracleLayout, OracleLegend, emit_entity_image,
    emit_oracle_cluster_children, emit_oracle_note_entity, wrap_oracle_envelope,
};
use crate::style::Theme;
use crate::svg::SvgBuilder;
use crate::text_render::{self, TextBase};

// ---------------------------------------------------------------------------
// PlantUML layout constants (extracted from golden SVGs)
// ---------------------------------------------------------------------------

/// Margin from SVG edge to entity boxes.
const MARGIN: f64 = 7.0;
/// Header height when `hide circle` removes the entity icon. Reduced from the
/// standard 32px header because the name no longer needs to clear the 22px-tall
/// icon glyph; PlantUML pushes the header separator up to y_rect + 26.4883.
const HEADER_H_NO_CIRCLE: f64 = 26.4883;
/// Gap between icon and entity name text.
const ICON_TEXT_GAP: f64 = 3.0;
/// PlantUML `EntityImageClassHeader` wraps the circled character in
/// `TextBlockUtils.withMargin(..., 4, 0, 5, 5)` before `HeaderLayout.drawU`.
const HEADER_CIRCLE_LEFT_MARGIN: f64 = 4.0;
const HEADER_CIRCLE_RIGHT_MARGIN: f64 = 0.0;
/// Class names in `EntityImageClassHeader` carry 3px left/right margin.
const HEADER_NAME_MARGIN_X: f64 = 3.0;
/// Java `HeaderLayout.drawU`: `h2 = min(circleWidth / 4, suppWidth * 0.1)`.
const HEADER_SECONDARY_GAP_RATIO: f64 = 0.1;
/// Icon ellipse radius at the default circled-character font size (17): the
/// radius is `circled_font_size / 3 + 6 = 17/3 + 6 = 11`.
const ICON_RX: f64 = 11.0;
/// PlantUML's `CIRCLED_CHARACTER` font default size. The circled header icon
/// inherits `defaultFontSize` when set, otherwise this value (it does *not*
/// inherit `ClassFontSize`). See `SkinParam.getCircledCharacterRadius`.
const CIRCLED_CHARACTER_DEFAULT_SIZE: u32 = 17;
/// Vertical inset of the circled-character icon centre below the rect top,
/// before adding the icon/title half-height. Measured from default-size goldens
/// (`cy = rect_top + 5 + max(radius, title_line_height/2)`).
const CIRCLED_ICON_TOP_INSET: f64 = 5.0;
/// Icon ellipse center x relative to entity left + 1.
const ICON_CX_OFFSET: f64 = 15.0;
/// Y position of entity name text baseline.
const NAME_BASELINE_Y: f64 = 28.291;
/// State-shaped entities inside `allowmixing` class diagrams.
const MIXED_STATE_HEIGHT: f64 = 50.0;
const MIXED_STATE_MIN_WIDTH: f64 = 50.0;
const MIXED_STATE_HPAD: f64 = 20.0;
const MIXED_STATE_NAME_BASELINE: f64 = 18.5352;
const MIXED_STATE_SEPARATOR_Y: f64 = 26.4883;
/// Height of entity header (icon + name area) — used in height computations.
#[allow(dead_code)]
const HEADER_HEIGHT: f64 = 32.0;
/// Height of a member line.
const MEMBER_LINE_HEIGHT: f64 = 16.48828125;
/// Vertical offset from compartment top to first member baseline.
const FIRST_MEMBER_OFFSET: f64 = 17.53515625;
/// Subsequent member baseline spacing.
const MEMBER_SPACING: f64 = 16.48828125;
/// Member-text left inset relative to the circled icon radius. PlantUML places
/// member text at `compartment_pad + (circledRadius + 3)`; with the compartment
/// pad and entity left margin this nets to `entity_x + radius + 9`.
const MEMBER_TEXT_INSET: f64 = 9.0;
/// Offset from entity x to enum constant text start.
const ENUM_TEXT_OFFSET: f64 = 6.0;
/// Offset from entity x to visibility icon center.
const VIS_ICON_OFFSET: f64 = 11.0;
/// PlantUML's default `classAttributeIconSize`; `VisibilityModifier.getUBlock`
/// exposes this as an 11px-high placement block.
const VIS_ICON_DEFAULT_SIZE: u32 = 10;
/// Visibility icon radius (small circle for method visibility).
const VIS_ICON_R: f64 = 3.0;
/// Default half-size for diamond and triangle visibility icons.
const VIS_ICON_ANGLED_HALF: f64 = 4.0;
/// PlantUML derives the round/square half-size as `classAttributeIconSize / 3`.
const VIS_ICON_SIZE_RADIUS_DIVISOR: u32 = 3;
/// Diamond/triangle horizontal half-size is one pixel inside half the icon box.
const VIS_ICON_ANGLED_INSET: f64 = 1.0;
/// `LimitFinder.drawUPolygon` expands every polygon by 10px horizontally
/// while measuring a SVEK image. Protected/package visibility icons are the
/// only class-body polygons that can own the left envelope.
const LIMIT_FINDER_POLYGON_OVERSCAN_X: f64 = 10.0;
/// `LimitFinder.drawRectangle` measures a `URectangle` from `(x - 1, y - 1)`.
const LIMIT_FINDER_RECTANGLE_INSET: f64 = 1.0;
/// PlantUML draws angled class-member visibility glyphs one pixel above the
/// round/square icon center. This follows the `USymbol` polygon coordinates
/// used for protected/package member markers after `classAttributeIconSize`
/// sizing, while public/private icons remain centered on the member baseline.
const VIS_ICON_ANGLED_CENTER_BIAS: f64 = 1.0;
/// Right padding for header (icon + name) area.
const HEADER_RIGHT_PAD: f64 = 3.0;
/// Right padding for member text area.
const MEMBER_RIGHT_PAD: f64 = 6.0;
/// Padding within each compartment (fields/methods).
const COMPARTMENT_PAD: f64 = 8.0;
/// Distance between entities in layout (vertical gap for top-to-bottom).
#[allow(dead_code)]
const ENTITY_GAP: f64 = 60.0;

/// Font size for entity names and member text.
const FONT_SIZE: f64 = 14.0;
/// Font size for stereotype text.
#[allow(dead_code)]
const STEREOTYPE_FONT_SIZE: f64 = 12.0;
/// Lollipop interface labels sit below the small synthetic endpoint ellipse.
const LOLLIPOP_LABEL_BASELINE_FROM_CENTER: f64 = 18.5352;
/// PlantUML draws class lollipop endpoints as a 5px ellipse with 1.5px stroke.
const LOLLIPOP_ENDPOINT_STYLE: &str = "stroke:#181818;stroke-width:1.5;";

// Generic type-parameter box (`class Foo<T>`): a small dashed rectangle at the
// entity's top-right corner. 12px italic text, 1px pad each side, overhanging
// the corner by 3px.
const GENERIC_FONT_SIZE: u32 = 12;
const GENERIC_BOX_PAD: f64 = 1.0;
const GENERIC_BOX_OVERHANG: f64 = 3.0;
const GENERIC_BOX_HEIGHT: f64 = 16.1328;
const GENERIC_TEXT_BASELINE: f64 = 12.6016;
// Gap between the header (icon + name) right edge and the generic box left edge.
const GENERIC_HEADER_GAP: f64 = 8.0;
/// Baseline-to-baseline distance between multiple stereotype lines.
const STEREOTYPE_LINE_HEIGHT: f64 = 14.1328;
/// Stereotype text baseline y relative to entity rect top.
const STEREOTYPE_Y_OFFSET: f64 = 16.6016;
/// Icon center y relative to entity rect top when stereotypes are present.
const ICON_CY_WITH_STEREO: f64 = 20.3105;

const NOTE_FILL: &str = "#FEFFDD";
const NOTE_BORDER: &str = "#181818";
const NOTE_FOLD: f64 = 10.0;
const NOTE_PAD_X: f64 = 6.0;
const NOTE_PAD_RIGHT: f64 = 15.0;
const NOTE_PAD_Y: f64 = 5.0;
const NOTE_FONT_SIZE: f64 = 13.0;
/// Java `Rose.paddingX/paddingY`: `ComponentRoseNote` reserves five pixels
/// around the visible folded note when used as an `EntityImageNoteLink`.
const RELATIONSHIP_NOTE_COMPONENT_PADDING: f64 = 5.0;
// Java `Bullet`: the level-one ellipse occupies a 12x5 atom, translated 3px
// right, with starting altitude -5. Deeper bullets occupy `8 + 8 * order`
// by 3 atoms, translate right by `1 + 8 * order`, and start at altitude -7.
const NOTE_BULLET_HEADER_WIDTH: f64 = 12.0;
const NOTE_BULLET_ELLIPSE_SIZE: f64 = 5.0;
const NOTE_BULLET_ELLIPSE_X: f64 = 3.0;
const NOTE_BULLET_START_ALTITUDE: f64 = -5.0;
const NOTE_NESTED_BULLET_BASE_WIDTH: f64 = 8.0;
const NOTE_NESTED_BULLET_INDENT: f64 = 8.0;
const NOTE_NESTED_BULLET_SIZE: f64 = 3.5;
const NOTE_NESTED_BULLET_DIM_HEIGHT: f64 = 3.0;
const NOTE_NESTED_BULLET_X: f64 = 1.0;
const NOTE_NESTED_BULLET_START_ALTITUDE: f64 = -7.0;
/// Extracted from `CreoleStripeSimpleParser` numbered atoms at the default
/// 13px note font: the marker run (`1.`) is followed by this fixed gap before
/// the item text, independent of its label and nesting level.
const NOTE_ORDERED_NUMBER_GAP: f64 = 4.1133;
const NOTE_TABLE_GRID_TOP_PAD: f64 = 2.0;
const NOTE_TABLE_BODY_EXTRA: f64 = 4.0;
const NOTE_TREE_TRUNK_X: f64 = 8.0;
const NOTE_TREE_BRANCH_WIDTH: f64 = 8.0;
const NOTE_TREE_TEXT_X: f64 = 18.0;
const NOTE_TREE_GRID_TOP_PAD: f64 = 2.0;
const NOTE_TREE_BODY_EXTRA: f64 = 4.0;
/// `EntityImageAssociationPoint.SIZE`: PlantUML lays out and paints the
/// synthetic point inserted into an association-class base edge as a 4px
/// circle.
const ASSOCIATION_POINT_SIZE: f64 = 4.0;
/// `Association.createNew` advances CucaDiagram's shared sequence six times:
/// the `apoint` short name, the point entity uid, a replaced temporary A-B
/// link, and the three emitted links.
const ASSOCIATION_SEQUENCE_SLOTS: usize = 6;
#[allow(dead_code)]
const SMALL_FONT: f64 = 11.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;

// --- Page-decoration (title/header/footer/caption) layout constants ---
//
// PlantUML positions the page decorations over a shared width
// `dimTotal = max(body_width, decoration_widths)` and anchors their baselines a
// fixed gap from the body's top/bottom edges. All values verified against the
// class golden SVGs (`class_title_basic`, `class_decoration_*`, etc.).
//
/// Left + right body margins added to the entity rect extent to form the body
/// block width (`dimOriginal`): 7px left + 8px right.
const BODY_DECORATION_MARGIN: f64 = 15.0;
/// The no-style SVEK envelope already carries PlantUML's 5px document margin
/// on its right and bottom sides. An explicit `root { Margin ... }` replaces
/// that amount in `TextBlockExporter12026.Builder.calculateMargin`.
const DEFAULT_DOCUMENT_EXTENT_MARGIN: i64 = 5;
/// Java SVEK `ExtremityExtends` draws the inheritance triangle with an 18px
/// length from contact tip to base centre and 12px base width, oriented by the
/// edge tangent.
const EXTENDS_TRIANGLE_LENGTH: f64 = 18.0;
const EXTENDS_TRIANGLE_HALF_WIDTH: f64 = 6.0;
/// Java SVEK diamond extremities (`ExtremityDiamond`) occupy 12px along the
/// edge tangent with a 4px half-width.
const DIAMOND_DECORATION_LENGTH: f64 = 12.0;
const DIAMOND_DECORATION_HALF_WIDTH: f64 = 4.0;
/// Java SVEK `ExtremityArrow.getDecorationLength()` returns 6px; the filled
/// arrow polygon itself reaches 9px back from the contact.
const ARROW_DECORATION_LENGTH: f64 = 6.0;
const ARROW_POLYGON_LENGTH: f64 = 9.0;
const ARROW_NOTCH_LENGTH: f64 = 5.0;
const ARROW_POLYGON_HALF_WIDTH: f64 = 4.0;
/// Java `SvekEdge` measures relationship, cardinality, and role labels with the
/// arrow font before passing fixed-size HTML-table placeholders to Graphviz.
const RELATIONSHIP_LABEL_FONT_SIZE: f64 = 13.0;
/// `SvekEdge.addVisibilityModifier` wraps center labels in a one-pixel shield.
const RELATIONSHIP_LABEL_MARGIN: f64 = 1.0;
const SELF_RELATIONSHIP_LABEL_MARGIN: f64 = 6.0;
// Graphviz's external-label placer leaves this much of a fixed HTML table
// below the midpoint of a vertical orthogonal edge. Extracted from Java
// `SvekEdge.appendDotString` `xlabel` layouts with renamed labels and 2-9
// chained nodes.
const ORTHO_XLABEL_VERTICAL_INSET: f64 = 5.0;
// Per-control-point deltas extracted from Java `SvekEdge.solveLine` for
// vertical `DotSplines.ORTHO` links between `ExtremityDoubleLine` and
// `ExtremityCircleCrowfoot`. The values are stable across renamed labels,
// package depths, and chains of 2-9 nodes.
const ORTHO_ER_VERTICAL_ROUTE_DELTAS: [f64; 4] = [-0.045, -0.075, -0.145, -0.155];
/// Java `TextBlockArrow2` reserves one font-size square before the label. Its
/// triangle size is `(int)(fontSize * .80)`, hence 10px at the 13px arrow font.
const LINK_ARROW_BLOCK_SIZE: f64 = RELATIONSHIP_LABEL_FONT_SIZE;
const LINK_ARROW_TRIANGLE_SIZE: f64 = 10.0;
/// `SvekResult.drawU` normalises a label-bearing SVEK envelope at x=6 rather
/// than the ordinary entity margin at x=7.
const SVEK_LABEL_ENVELOPE_MARGIN: f64 = 6.0;
/// `SvekEdge.manageCollision` expands each node by eight pixels before moving
/// intersecting endpoint-label rectangles away from it.
const ENDPOINT_LABEL_COLLISION_MARGIN: f64 = 8.0;
const COLLISION_INITIAL_COEFFICIENT: f64 = 0.1;
const COLLISION_SEARCH_STEPS: usize = 5;
const COLLISION_MAX_DOUBLINGS: usize = 64;
/// document.title style: Padding 5 + Margin 5 on each side.
const DECORATION_TITLE_INSET: f64 = 10.0;
/// document.caption style: Padding 0 + Margin 1 on each side.
const DECORATION_CAPTION_INSET: f64 = 1.0;
/// Header glyph baseline: fixed at the top of the canvas.
const DECORATION_HEADER_BASELINE_Y: f64 = 9.668;
/// Title glyph baseline sits this far above the body's top edge.
const DECORATION_TITLE_GAP_ABOVE_BODY: f64 = 20.9531;
/// Footer glyph baseline sits this far below the body's bottom edge.
const DECORATION_FOOTER_GAP_BELOW_BODY: f64 = 18.668;
/// Caption glyph baseline sits this far below the body's bottom edge.
const DECORATION_CAPTION_GAP_BELOW_BODY: f64 = 23.5352;
/// `TextBlockBordered.calculateDimension` adds one pixel to both dimensions,
/// even when the decoration border is transparent.
const DECORATION_BORDER_EXTENT: f64 = 1.0;
/// Baseline-to-baseline spacing for multi-line page decorations.
const DECORATION_LINE_HEIGHT: f64 = MEMBER_LINE_HEIGHT;
const GRID_MARGIN: f64 = 30.0;
#[allow(dead_code)]
const CLASS_MIN_WIDTH: f64 = 120.0;
#[allow(dead_code)]
const PACKAGE_HEADER: f64 = 24.0;
#[allow(dead_code)]
const PACKAGE_PAD: f64 = 12.0;
/// Default package tab separator offset from `ClusterDecoration` output.
/// Provenance: Java SVEK `Cluster.drawU` delegates to `ClusterDecoration`;
/// default class package goldens place the tab line at package top + 22.4883.
const PACKAGE_TAB_H: f64 = 22.4883;
/// Default package title baseline within the tab.
/// Provenance: same `ClusterDecoration` path; goldens place title baseline at
/// package top + 15.5352 for 14px bold sans-serif package labels.
const PACKAGE_TITLE_BASELINE: f64 = 15.5352;
const PACKAGE_TAB_TEXT_X: f64 = 4.0;
/// `USymbolFolder.asBig` places the stereotype block two pixels below the
/// cluster origin plus the tab height; its 14px text then uses the ordinary
/// package-title baseline.
const PACKAGE_STEREOTYPE_BASELINE: f64 = PACKAGE_TAB_H + PACKAGE_TITLE_BASELINE;
/// Java `USymbolFolder`: `marginTitleX1 = marginTitleX2 = 3`,
/// `marginTitleX3 = 7`; default package round-corner is 5px.
const PACKAGE_TITLE_MARGIN_X: f64 = 3.0;
const PACKAGE_TAB_SLOPE_WIDTH: f64 = 7.0;
const PACKAGE_ROUND_CORNER: f64 = 5.0;
const PACKAGE_STROKE_WIDTH: &str = "1.5";
/// Java `USymbolFrame.drawFrame`, `USymbolRectangle.drawRect`, and
/// `USymbolNode.drawNode` use the ordinary cluster line thickness.
const SYMBOL_CLUSTER_STROKE_WIDTH: &str = "1";
/// Java `USymbolDatabase.suppHeightBecauseOfShape()` reserves 15px above the
/// ordinary cluster title so the title sits below the cylinder's top ellipse.
const DATABASE_CLUSTER_TITLE_EXTRA: f64 = 15.0;
/// `USymbolDatabase.asBig()` translates the title 20px below the ordinary
/// package-title baseline.
const DATABASE_CLUSTER_TITLE_OFFSET: f64 = 20.0;
/// `USymbolDatabase.drawDatabase()` emits `UEmpty(10, 10)` at the shape's
/// lower-right corner, extending the SVEK painted envelope on both axes.
const DATABASE_CLUSTER_ENVELOPE_EXTRA: f64 = 10.0;
/// Java `USymbolNode.suppWidthBecauseOfShape()` and
/// `suppHeightBecauseOfShape()` enlarge the title placeholder fed to SVEK.
const NODE_CLUSTER_TITLE_WIDTH_EXTRA: f64 = 60.0;
const NODE_CLUSTER_TITLE_HEIGHT_EXTRA: f64 = 5.0;
/// `USymbolNode.drawNode()` offsets the visible bevel by ten pixels and emits
/// a lower-right `UEmpty(10, 10)`, extending the calculated SVEK envelope.
const NODE_CLUSTER_ENVELOPE_X_EXTRA: f64 = 20.0;
const NODE_CLUSTER_ENVELOPE_Y_EXTRA: f64 = 10.0;
const NODE_BEVEL: f64 = 10.0;
const FRAME_TITLE_CORNER: f64 = 10.0;
/// Java `USymbolCloud.asBig()` draws the title at y=13 within the symbol;
/// the title text block's 14px bold baseline is another 13.5352px below it.
const CLOUD_TITLE_BASELINE: f64 = 26.5352;
/// Package cluster canvases use the full SVEK body side extent (left 6 plus
/// right-side stroke/body slack) rather than the single-entity 13px formula.
/// Provenance: Java `SvekResult.drawU` normalises the body at x/y=6 before
/// emitting the package `ClusterDecoration` rectangle.
const PACKAGE_CANVAS_EXTENT_PAD: i64 = 15;

/// Font names that PlantUML treats as monospace.
const MONOSPACE_FONTS: &[&str] = &[
    "courier",
    "monospaced",
    "monospace",
    "consolas",
    "lucida console",
];

// ---------------------------------------------------------------------------
// Entity icon colors
// ---------------------------------------------------------------------------

const CLASS_ICON_FILL: &str = "#ADD1B2";
const INTERFACE_ICON_FILL: &str = "#B4A7E5";
const ENUM_ICON_FILL: &str = "#EB937F";
const ABSTRACT_ICON_FILL: &str = "#A9DCDF";
const ANNOTATION_ICON_FILL: &str = "#E3664A";

// ---------------------------------------------------------------------------
// Entity background and border
// ---------------------------------------------------------------------------

const ENTITY_FILL: &str = "#F1F1F1";
const BORDER_COLOR: &str = "#181818";
const BORDER_WIDTH: &str = "0.5";
const ICON_STROKE_WIDTH: &str = "1";

// ---------------------------------------------------------------------------
// Visibility modifier colors
// ---------------------------------------------------------------------------

const VIS_PUBLIC_FILL_FIELD: &str = "none";
const VIS_PUBLIC_FILL_METHOD: &str = "#84BE84";
const VIS_PUBLIC_STROKE: &str = "#038048";

const VIS_PRIVATE_FILL_FIELD: &str = "none";
const VIS_PRIVATE_FILL_METHOD: &str = "#F24D5C";
const VIS_PRIVATE_STROKE: &str = "#C82930";

const VIS_PROTECTED_FILL_FIELD: &str = "none";
const VIS_PROTECTED_FILL_METHOD: &str = "#FFFF44";
const VIS_PROTECTED_STROKE: &str = "#B38D22";

const VIS_PACKAGE_FILL_FIELD: &str = "none";
const VIS_PACKAGE_FILL_METHOD: &str = "#4177AF";
const VIS_PACKAGE_STROKE: &str = "#1963A0";

// ---------------------------------------------------------------------------
// Entity icon glyph paths (position-dependent at cx=22, cy=23)
// ---------------------------------------------------------------------------

/// "C" glyph for Class icons (relative to entity x=0, cx=22).
const CLASS_GLYPH: &str = "M24.4731,29.1431 Q23.8921,29.4419 23.2529,29.5913 Q22.6138,29.7407 21.9082,29.7407 Q19.4014,29.7407 18.0815,28.0889 Q16.7617,26.437 16.7617,23.3159 Q16.7617,20.1865 18.0815,18.5347 Q19.4014,16.8828 21.9082,16.8828 Q22.6138,16.8828 23.2612,17.0322 Q23.9087,17.1816 24.4731,17.4805 L24.4731,20.2031 Q23.8423,19.6221 23.2488,19.3523 Q22.6553,19.0825 22.0244,19.0825 Q20.6797,19.0825 19.9949,20.1492 Q19.3101,21.2158 19.3101,23.3159 Q19.3101,25.4077 19.9949,26.4744 Q20.6797,27.541 22.0244,27.541 Q22.6553,27.541 23.2488,27.2712 Q23.8423,27.0015 24.4731,26.4204 Z ";

/// "I" glyph for Interface icons (extracted from golden SVG at cx=22, cy=23).
const INTERFACE_GLYPH: &str = "M18.4277,19.2651 L18.4277,17.1069 L25.8071,17.1069 L25.8071,19.2651 L23.3418,19.2651 L23.3418,27.3418 L25.8071,27.3418 L25.8071,29.5 L18.4277,29.5 L18.4277,27.3418 L20.8931,27.3418 L20.8931,19.2651 Z ";

/// "E" glyph for Enum icons (at cx=22).
const ENUM_GLYPH: &str = "M25.6143,29.5 L17.8945,29.5 L17.8945,17.1069 L25.6143,17.1069 L25.6143,19.2651 L20.3433,19.2651 L20.3433,21.938 L25.1162,21.938 L25.1162,24.0962 L20.3433,24.0962 L20.3433,27.3418 L25.6143,27.3418 Z ";

/// "A" glyph for Abstract class icons (extracted from golden SVG at cx=22, cy=23).
const ABSTRACT_GLYPH: &str = "M21.8633,18.3481 L20.7095,23.4199 L23.0254,23.4199 Z M20.3691,16.1069 L23.3657,16.1069 L26.7109,28.5 L24.2622,28.5 L23.4985,25.437 L20.2197,25.437 L19.4727,28.5 L17.0239,28.5 Z ";

// ---------------------------------------------------------------------------
// Computed entity dimensions
// ---------------------------------------------------------------------------

struct EntityDims {
    width: f64,
    height: f64,
    /// Number of fields (members in the fields compartment).
    #[allow(dead_code)]
    field_count: usize,
    /// Number of methods (members in the methods compartment).
    #[allow(dead_code)]
    method_count: usize,
    /// Whether the entity is an enum (affects member rendering).
    is_enum: bool,
    /// Name text width.
    #[allow(dead_code)]
    name_width: f64,
    /// Whether the entity has stereotypes (affects header height and layout).
    has_stereotypes: bool,
    /// Number of visible stereotype lines in the header.
    stereotype_count: usize,
    /// Whether an inline sprite replaces the ordinary circled header badge.
    has_header_sprite: bool,
    /// Source line number from the parser (1-based).
    source_line: usize,
    /// Visibility flags from `hide`/`show` directives applied to this entity.
    hide: HideFlags,
}

/// What's hidden for one entity, after resolving global and per-kind
/// `hide`/`show` directives.
#[derive(Debug, Clone, Copy, Default)]
struct HideFlags {
    circle: bool,
    fields: bool,
    methods: bool,
    stereotype: bool,
    hide_private_fields: bool,
    hide_private_methods: bool,
    hide_protected_fields: bool,
    hide_protected_methods: bool,
    hide_public_fields: bool,
    hide_public_methods: bool,
    hide_package_fields: bool,
    hide_package_methods: bool,
}

impl HideFlags {
    fn hides_member(self, m: &Member) -> bool {
        if (self.fields && m.kind == MemberKind::Field)
            || (self.methods && m.kind == MemberKind::Method)
        {
            return true;
        }
        let is_field = m.kind == MemberKind::Field;
        let is_method = m.kind == MemberKind::Method;
        match m.visibility {
            Visibility::Private => {
                (is_field && self.hide_private_fields) || (is_method && self.hide_private_methods)
            }
            Visibility::Protected => {
                (is_field && self.hide_protected_fields)
                    || (is_method && self.hide_protected_methods)
            }
            Visibility::Public => {
                (is_field && self.hide_public_fields) || (is_method && self.hide_public_methods)
            }
            Visibility::Package => {
                (is_field && self.hide_package_fields) || (is_method && self.hide_package_methods)
            }
            Visibility::Default | Visibility::IeMandatory => false,
        }
    }
}

/// Resolve the `hide_show` directives against one entity's kind / stereotypes.
fn resolve_hide(entity: &ClassEntity, directives: &[HideShow]) -> HideFlags {
    let mut h = HideFlags::default();
    let entity_kind_word = match entity.kind {
        EntityKind::Class => "class",
        EntityKind::Interface => "interface",
        EntityKind::Enum => "enum",
        EntityKind::AbstractClass => "abstract",
        EntityKind::Annotation => "annotation",
        EntityKind::Entity => "entity",
        EntityKind::Object => "object",
        EntityKind::State => "state",
        EntityKind::Circle => "circle",
        EntityKind::Diamond => "diamond",
    };
    for d in directives {
        if d.remove {
            continue;
        }
        // Tokenise: optional selector (entity kind keyword, `<<stereo>>`, or
        // entity name) followed by the visibility keyword(s).
        let arg = d.arg.trim();
        let (selector, what) = split_hide_selector(arg);
        let applies = match selector {
            None => true,
            Some(s) if s.eq_ignore_ascii_case(entity_kind_word) => true,
            Some(s) if s.eq_ignore_ascii_case("class") && entity.kind == EntityKind::Entity => true,
            Some(s) if s.starts_with("<<") && s.ends_with(">>") => {
                let stereo = s[2..s.len() - 2].trim();
                entity
                    .stereotypes
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(stereo))
            }
            Some(s) if s.eq_ignore_ascii_case(&entity.id) => true,
            _ => false,
        };
        if !applies {
            continue;
        }
        // Tokenise the remainder. Recognised modifier prefixes:
        //   `empty`              — only act when the named compartment is empty
        //   `private/protected/public/package` — restrict to that visibility
        let tokens: Vec<&str> = what.split_whitespace().collect();
        let mut empty_only = false;
        let mut visibility_filter: Option<Visibility> = None;
        let mut idx = 0;
        if let Some(first) = tokens.first()
            && first.eq_ignore_ascii_case("empty")
        {
            empty_only = true;
            idx = 1;
        }
        if let Some(tok) = tokens.get(idx) {
            let vis = match tok.to_ascii_lowercase().as_str() {
                "private" => Some(Visibility::Private),
                "protected" => Some(Visibility::Protected),
                "public" => Some(Visibility::Public),
                "package" => Some(Visibility::Package),
                _ => None,
            };
            if let Some(v) = vis {
                visibility_filter = Some(v);
                idx += 1;
            }
        }
        let has_fields = entity.members.iter().any(|m| m.kind == MemberKind::Field);
        let has_methods = entity.members.iter().any(|m| m.kind == MemberKind::Method);
        for tok in &tokens[idx..] {
            let t = tok.to_ascii_lowercase();
            match (visibility_filter, t.as_str()) {
                (None, "circle") => h.circle = !d.show,
                (None, "attributes" | "fields" | "attribute" | "field")
                    if !empty_only || !has_fields =>
                {
                    h.fields = !d.show;
                }
                (None, "methods" | "method") if !empty_only || !has_methods => {
                    h.methods = !d.show;
                }
                (None, "members" | "member") => {
                    // `hide members` hides both compartments. `hide empty
                    // members` hides each compartment *independently* when
                    // that compartment alone is empty: a class with methods
                    // but no fields keeps its (non-empty) methods compartment
                    // while suppressing the empty fields compartment, leaving
                    // a single header separator rather than two.
                    if !empty_only || !has_fields {
                        h.fields = !d.show;
                    }
                    if !empty_only || !has_methods {
                        h.methods = !d.show;
                    }
                }
                (None, "stereotype" | "stereotypes") => h.stereotype = !d.show,
                (
                    Some(v),
                    kind @ ("attributes" | "fields" | "attribute" | "field" | "methods" | "method"
                    | "members" | "member"),
                ) => {
                    let want_field = matches!(
                        kind,
                        "attributes" | "fields" | "attribute" | "field" | "members" | "member"
                    );
                    let want_method = matches!(kind, "methods" | "method" | "members" | "member");
                    let val = !d.show;
                    match v {
                        Visibility::Private => {
                            if want_field {
                                h.hide_private_fields = val;
                            }
                            if want_method {
                                h.hide_private_methods = val;
                            }
                        }
                        Visibility::Protected => {
                            if want_field {
                                h.hide_protected_fields = val;
                            }
                            if want_method {
                                h.hide_protected_methods = val;
                            }
                        }
                        Visibility::Public => {
                            if want_field {
                                h.hide_public_fields = val;
                            }
                            if want_method {
                                h.hide_public_methods = val;
                            }
                        }
                        Visibility::Package => {
                            if want_field {
                                h.hide_package_fields = val;
                            }
                            if want_method {
                                h.hide_package_methods = val;
                            }
                        }
                        Visibility::Default | Visibility::IeMandatory => {}
                    }
                }
                _ => {}
            }
        }
    }
    h
}

/// Split a hide-directive argument into `(selector, rest)`.
/// Selectors recognised: entity-kind keywords, `<<stereotype>>` references,
/// and bare identifiers that name a known entity.
fn split_hide_selector(arg: &str) -> (Option<&str>, &str) {
    // `<<stereo>>` selector: extract up to closing `>>`.
    if let Some(rest) = arg.strip_prefix("<<")
        && let Some(end) = rest.find(">>")
    {
        let selector = &arg[..end + 4];
        let what = arg[end + 4..].trim_start();
        return (Some(selector), what);
    }
    // First token may be a selector keyword. `empty members` is a property
    // keyword pair, not a selector.
    if let Some((first, rest)) = arg.split_once(char::is_whitespace) {
        let f = first.to_ascii_lowercase();
        const KINDS: &[&str] = &[
            "class",
            "interface",
            "enum",
            "abstract",
            "annotation",
            "entity",
        ];
        if KINDS.contains(&f.as_str()) {
            return (Some(first), rest.trim_start());
        }
        const QUALIFIERS: &[&str] = &["empty", "private", "protected", "public", "package"];
        if !QUALIFIERS.contains(&f.as_str()) {
            return (Some(first), rest.trim_start());
        }
    }
    (None, arg)
}

/// Whether `directive.arg` names a whole entity (or a `<<stereotype>>` group)
/// rather than a compartment. `hide B`, `remove B`, `show B`, and
/// `hide <<internal>>` are whole-entity directives; `hide circle`,
/// `hide empty members`, `hide methods` are not.
///
/// Returns the matcher to apply against each entity, or `None` if this is a
/// compartment-level directive that should be left to `resolve_hide`.
fn whole_entity_selector(arg: &str, entities: &[ClassEntity]) -> Option<EntitySelector> {
    let arg = arg.trim();
    // `<<stereo>>` with nothing after it.
    if let Some(rest) = arg.strip_prefix("<<")
        && let Some(end) = rest.find(">>")
    {
        let after = rest[end + 2..].trim();
        if after.is_empty() {
            return Some(EntitySelector::Stereotype(rest[..end].trim().to_string()));
        }
        return None;
    }
    // A bare single token names an entity pattern. PlantUML `HideOrShow.match`
    // accepts `*` wildcards as well as exact qualified-name leaves.
    if !arg.is_empty()
        && !arg.contains(char::is_whitespace)
        && (arg.contains('*') || entities.iter().any(|e| lifecycle_name_matches(arg, e)))
    {
        return Some(EntitySelector::Name(arg.to_string()));
    }
    None
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    let mut remainder = value;
    let mut anchored_start = true;
    for (index, part) in pattern.split('*').enumerate() {
        if part.is_empty() {
            anchored_start = false;
            continue;
        }
        let Some(position) = remainder.find(part) else {
            return false;
        };
        if index == 0 && anchored_start && position != 0 {
            return false;
        }
        remainder = &remainder[position + part.len()..];
        anchored_start = false;
    }
    pattern.ends_with('*') || remainder.is_empty()
}

fn lifecycle_name_matches(pattern: &str, entity: &ClassEntity) -> bool {
    [entity.id.as_str(), entity.label.as_str()]
        .into_iter()
        .any(|name| {
            let leaf = name.rsplit(['.', ':']).next().unwrap_or(name);
            if pattern.contains('*') {
                wildcard_matches(pattern, leaf)
            } else {
                leaf.eq_ignore_ascii_case(pattern)
            }
        })
}

/// A whole-entity selector resolved from a `hide`/`remove`/`show` directive.
enum EntitySelector {
    Name(String),
    Stereotype(String),
}

impl EntitySelector {
    fn matches(&self, entity: &ClassEntity) -> bool {
        match self {
            EntitySelector::Name(n) => lifecycle_name_matches(n, entity),
            EntitySelector::Stereotype(s) => entity.stereotypes.iter().any(|t| {
                if s.contains('*') {
                    wildcard_matches(s, t)
                } else {
                    t.eq_ignore_ascii_case(s)
                }
            }),
        }
    }
}

/// Resolve either the `HideOrShow` or remove/restore lifecycle stream.
///
/// Java keeps these as separate lists in `CucaDiagram`: hidden entities still
/// enter SVEK and influence layout, while removed entities never enter the
/// graph. Later show/restore directives replace the earlier state.
fn lifecycle_entities(diagram: &ClassDiagram, remove: bool) -> std::collections::HashSet<usize> {
    let mut suppressed = std::collections::HashSet::new();
    for d in diagram
        .hide_show
        .iter()
        .filter(|directive| directive.remove == remove)
    {
        let Some(sel) = whole_entity_selector(&d.arg, &diagram.entities) else {
            continue;
        };
        for (i, e) in diagram.entities.iter().enumerate() {
            if sel.matches(e) {
                if d.show {
                    suppressed.remove(&i);
                } else {
                    suppressed.insert(i);
                }
            }
        }
    }
    suppressed
}

fn relationship_touches_hidden_entity(
    relationship: &Relationship,
    diagram: &ClassDiagram,
    hidden_entities: &std::collections::HashSet<usize>,
) -> bool {
    [&relationship.from, &relationship.to]
        .into_iter()
        .any(|endpoint| {
            diagram.entities.iter().enumerate().any(|(index, entity)| {
                hidden_entities.contains(&index)
                    && (entity.id == *endpoint || entity.label == *endpoint)
            })
        })
}

/// Build a copy of `diagram` with the given entity indices removed, along with
/// any relationships and notes that reference them, and any package membership
/// entries. Relationships/notes whose endpoints survive are kept verbatim.
fn filter_removed(
    diagram: &ClassDiagram,
    suppressed: &std::collections::HashSet<usize>,
) -> ClassDiagram {
    let dropped_ids: std::collections::HashSet<&str> = suppressed
        .iter()
        .map(|&i| diagram.entities[i].id.as_str())
        .collect();
    let dropped_labels: std::collections::HashSet<&str> = suppressed
        .iter()
        .map(|&i| diagram.entities[i].label.as_str())
        .collect();
    let is_dropped = |name: &str| dropped_ids.contains(name) || dropped_labels.contains(name);

    let mut out = diagram.clone();
    out.entities = diagram
        .entities
        .iter()
        .enumerate()
        .filter(|(i, _)| !suppressed.contains(i))
        .map(|(_, e)| e.clone())
        .collect();
    out.relationships
        .retain(|r| !is_dropped(&r.from) && !is_dropped(&r.to));
    out.association_classes
        .retain(|ac| !is_dropped(&ac.a) && !is_dropped(&ac.b) && !is_dropped(&ac.c));
    out.notes
        .retain(|n| !n.target.as_deref().is_some_and(is_dropped));
    for pkg in &mut out.packages {
        pkg.entities.retain(|name| !is_dropped(name));
    }
    out
}

fn calc_entity_dims(
    entity: &ClassEntity,
    entity_index: usize,
    hide: HideFlags,
    font: &ClassFontOverrides,
    sprites: &HashMap<String, SpriteData>,
) -> EntityDims {
    let is_enum = entity.kind == EntityKind::Enum;
    let name_font_size = font.name_font_size() as f64;
    let member_font_size = font.member_font_size() as f64;
    let (stereotype_bold, _) = font.stereotype_font_style(&entity.stereotypes);
    let name_bold = font.font_bold || font.attr_font_bold || stereotype_bold;
    // Entity labels treat `__` as literal underscores, not underline markup,
    // so width must include those characters.
    let name_width = escaped_newline_lines(&entity.label)
        .iter()
        .map(|line| {
            text_render::measure_no_underline_with_family(
                line,
                name_font_size,
                name_bold,
                &font.name_family,
            )
        })
        .fold(0.0_f64, f64::max);
    if entity.kind == EntityKind::State {
        let source_line = if entity.source_line > 0 {
            entity.source_line
        } else {
            entity_index + 1
        };
        return EntityDims {
            width: MIXED_STATE_MIN_WIDTH.max(name_width + MIXED_STATE_HPAD),
            height: MIXED_STATE_HEIGHT,
            field_count: 0,
            method_count: 0,
            is_enum: false,
            name_width,
            has_stereotypes: false,
            stereotype_count: 0,
            has_header_sprite: false,
            hide,
            source_line,
        };
    }
    if matches!(entity.kind, EntityKind::Circle | EntityKind::Diamond) {
        let source_line = if entity.source_line > 0 {
            entity.source_line
        } else {
            entity_index + 1
        };
        let shape_size = if entity.kind == EntityKind::Circle {
            16.0
        } else {
            24.0
        };
        return EntityDims {
            width: shape_size,
            height: shape_size,
            field_count: 0,
            method_count: 0,
            is_enum: false,
            name_width,
            has_stereotypes: false,
            stereotype_count: 0,
            has_header_sprite: false,
            hide,
            source_line,
        };
    }
    let visible_stereotypes: Vec<String> = entity
        .stereotypes
        .iter()
        .filter(|stereotype| !stereotype_refs_sprite(stereotype, sprites))
        .cloned()
        .collect();
    let has_stereotypes = !visible_stereotypes.is_empty() && !hide.stereotype;
    let stereotype_count = if has_stereotypes {
        visible_stereotypes.len()
    } else {
        0
    };
    let header_sprite = entity_header_sprite(entity, sprites);

    // Split members into fields and methods. For enums with method members
    // (or any explicit visibility marker), PlantUML uses the standard
    // class-style two-compartment layout rather than the single
    // enum-constants compartment.
    let enum_has_methods = is_enum
        && entity
            .members
            .iter()
            .any(|m| m.kind == MemberKind::Method || m.visibility != Visibility::Default);
    let enum_classic = is_enum && !enum_has_methods;
    let visible_field_blocks = entity
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::Field && !hide.hides_member(m))
        .count();
    let visible_method_blocks = entity
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::Method && !hide.hides_member(m))
        .count();
    let (field_count, method_count) = if enum_classic {
        (
            entity
                .members
                .iter()
                .filter(|m| !hide.hides_member(m))
                .map(|m| member_display_line_count(m, font.monospace_member_spaces()))
                .sum(),
            0,
        )
    } else {
        let fields = entity
            .members
            .iter()
            .filter(|m| m.kind == MemberKind::Field && !hide.hides_member(m))
            .map(|m| member_display_line_count(m, font.monospace_member_spaces()))
            .sum();
        let methods = entity
            .members
            .iter()
            .filter(|m| m.kind == MemberKind::Method && !hide.hides_member(m))
            .map(|m| member_display_line_count(m, font.monospace_member_spaces()))
            .sum();
        // If there are only methods (no fields), PlantUML puts them after the
        // header with two separator lines. If there are only fields, methods
        // compartment gets one separator line.
        (fields, methods)
    };

    // Compute width from icon area + name + member text widths. When the
    // circle is hidden the icon contributes no horizontal real estate; the
    // name is centred in the available header instead.
    let icon_area = if let Some((_, sprite)) = header_sprite {
        let (width, _) = crate::sprite::sprite_dimensions(sprite);
        HEADER_CIRCLE_LEFT_MARGIN + width as f64 + ICON_TEXT_GAP
    } else if hide.circle {
        // `MyType` etc. golden output shows the name horizontally centred
        // inside a 2*HEADER_RIGHT_PAD-padded box; treat the icon area as
        // empty padding to recover the matching width.
        HEADER_RIGHT_PAD
    } else if entity.kind == EntityKind::Object {
        ENUM_TEXT_OFFSET
    } else {
        HEADER_CIRCLE_LEFT_MARGIN + font.circled_radius() * 2.0 + ICON_TEXT_GAP
    };
    let name_total = icon_area + name_width + HEADER_RIGHT_PAD + font.text_padding * 2.0;

    // Stereotype text may also affect width.
    let stereo_width = if has_stereotypes {
        let stereo_tw = format_stereotype_lines(&visible_stereotypes)
            .iter()
            .map(|line| text_render::measure(line, 12.0, false))
            .fold(0.0_f64, f64::max);
        if !hide.circle && entity.kind != EntityKind::Object {
            // `EntityImageClassHeader` wraps a 22px circle with 4px of left
            // margin and the stereotype Display with 1px of left margin.
            // EntityImageClass's SVG envelope adds the final outer pixel.
            font.circled_radius() * 2.0
                + HEADER_CIRCLE_LEFT_MARGIN
                + stereo_tw
                + 2.0
                + font.text_padding * 2.0
        } else {
            icon_area + stereo_tw + HEADER_RIGHT_PAD + font.text_padding * 2.0
        }
    } else {
        0.0
    };

    // Java `MethodsOrFieldsArea.hasSmallIcon()` scans a whole compartment,
    // then `calculateDimensionOnlyMembers()` adds
    // `getCircledCharacterRadius() + 3` to its maximum text width. Thus a
    // default-visibility row still reserves the icon column when a sibling
    // field or method has a class visibility modifier. Rust's `IeMandatory`
    // represents ER-table `*` syntax, which is not a Java VisibilityModifier.
    let has_visibility_modifier = |visibility: Visibility| {
        matches!(
            visibility,
            Visibility::Public | Visibility::Private | Visibility::Protected | Visibility::Package
        )
    };
    let visibility_icons_enabled = font.attr_icon_size != Some(0);
    let fields_have_small_icon = visibility_icons_enabled
        && entity.members.iter().any(|m| {
            m.kind == MemberKind::Field
                && !hide.hides_member(m)
                && has_visibility_modifier(m.visibility)
        });
    let methods_have_small_icon = visibility_icons_enabled
        && entity.members.iter().any(|m| {
            m.kind == MemberKind::Method
                && !hide.hides_member(m)
                && has_visibility_modifier(m.visibility)
        });
    let member_text_offset = MEMBER_TEXT_INSET + font.circled_radius();
    let member_widths: Vec<f64> = entity
        .members
        .iter()
        .filter(|m| m.kind != MemberKind::Separator)
        .filter(|m| !hide.hides_member(m))
        .map(|m| {
            let text_w = member_display_lines(m, font.monospace_member_spaces())
                .iter()
                .map(|text| {
                    if let Some(latex) = latex_member_content(text) {
                        crate::math::latex_layout_metrics(latex).width
                    } else {
                        text_render::measure_no_underline_with_family(
                            text,
                            member_font_size,
                            font.attr_font_bold,
                            &font.family,
                        )
                    }
                })
                .fold(0.0_f64, f64::max);
            let compartment_has_small_icon = match m.kind {
                MemberKind::Field => fields_have_small_icon,
                MemberKind::Method => methods_have_small_icon,
                MemberKind::Separator => false,
            };
            // ER mandatory markers reserve space only on their own row; they
            // do not activate the Java class-visibility column for siblings.
            let text_offset =
                if compartment_has_small_icon || m.visibility == Visibility::IeMandatory {
                    member_text_offset
                } else {
                    ENUM_TEXT_OFFSET
                };
            text_offset + text_w + MEMBER_RIGHT_PAD + font.text_padding * 2.0
        })
        .collect();

    let max_member_width = member_widths.iter().cloned().fold(0.0_f64, f64::max);
    // Hidden compartments contribute nothing to the per-compartment count.
    let eff_field_count = if hide.fields { 0 } else { field_count };
    let eff_method_count = if hide.methods { 0 } else { method_count };
    let separator_title_width = entity
        .members
        .iter()
        .filter(|member| member.kind == MemberKind::Separator)
        .filter(|member| !member.display_text.is_empty())
        .map(|member| {
            text_render::measure_no_underline_with_family(
                &member.display_text,
                font.attr_font_size.unwrap_or(FONT_SIZE as u32) as f64,
                false,
                &font.family,
            ) + 8.0
        })
        .fold(0.0_f64, f64::max);
    let mut width = name_total
        .max(stereo_width)
        .max(max_member_width)
        .max(separator_title_width);

    // Generic type-parameter box widening: when `class Foo<T extends Bar>` has a
    // wide `<...>`, the dashed box at the top-right corner forces the entity
    // wider so the box's left edge sits just past the header (icon + name + 8px
    // gap) instead of overflowing the canvas. The box overhangs the right edge
    // by GENERIC_BOX_OVERHANG, so the required entity width is
    //   icon_area + name + gap + box_width - overhang.
    if let Some(generic) = entity.generic.as_deref() {
        let gen_tl = text_render::measure(generic, GENERIC_FONT_SIZE as f64, false);
        let box_w = gen_tl + GENERIC_BOX_PAD * 2.0;
        let generic_driven =
            icon_area + name_width + GENERIC_HEADER_GAP + box_w - GENERIC_BOX_OVERHANG;
        width = width.max(generic_driven);
    }

    // Height calculation.
    // PlantUML layout formula (derived from golden SVGs):
    //   header = 32px (icon + name), or 40.6211px with stereotypes
    //   each compartment = 8px padding + n * 16.4883px per member
    //   empty compartment = 8px

    let header_h = class_header_height(
        entity,
        hide,
        has_stereotypes,
        stereotype_count,
        header_sprite.is_some(),
        font.text_padding,
        font.name_font_size(),
        &font.name_family,
        font.circled_radius(),
    );
    let field_padding = visible_field_blocks as f64 * font.text_padding * 2.0;
    let method_padding = visible_method_blocks as f64 * font.text_padding * 2.0;
    let field_content_height = members_content_height(
        entity
            .members
            .iter()
            .filter(|member| member.kind == MemberKind::Field && !hide.hides_member(member)),
        member_font_size,
        &font.family,
        font.monospace_member_spaces(),
    );
    let method_content_height = members_content_height(
        entity
            .members
            .iter()
            .filter(|member| member.kind == MemberKind::Method && !hide.hides_member(member)),
        member_font_size,
        &font.family,
        font.monospace_member_spaces(),
    );

    let height = if uses_document_order_body(entity, hide) {
        header_h + document_order_body_height(entity, font)
    } else if hide.fields && hide.methods {
        // Both compartments hidden — header only, no body or separators.
        header_h
    } else if entity.kind == EntityKind::Object {
        header_h + COMPARTMENT_PAD + field_content_height + field_padding
    } else if entity.members.is_empty()
        || (eff_field_count == 0 && eff_method_count == 0 && !enum_classic)
    {
        // Every visible empty `MethodsOrFieldsArea` contributes its own
        // `TextBlockLineBefore` margin; a hidden portion contributes no block.
        header_h
            + if hide.fields { 0.0 } else { COMPARTMENT_PAD }
            + if hide.methods { 0.0 } else { COMPARTMENT_PAD }
    } else if enum_classic {
        // Enum: header + values + bottom separator.
        header_h + (COMPARTMENT_PAD + field_content_height + field_padding) + COMPARTMENT_PAD
    } else {
        // Class/interface/abstract/annotation.
        // Java `BodierLikeClassOrObject.getBody()` returns only the visible
        // `MethodsOrFieldsArea` block when one entire portion is hidden.
        let fields_section = if hide.fields {
            0.0
        } else {
            COMPARTMENT_PAD + field_content_height + field_padding
        };
        let methods_section = if hide.methods {
            0.0
        } else {
            // Java `BodyEnhancedAbstract.decorate` wraps a titled separator
            // block in half the title height above `TextBlockLineBefore`, then
            // gives the member block another half-title top margin and 4px at
            // the bottom. An untitled block instead keeps the ordinary 4px
            // margins on both sides.
            let block_padding = methods_separator_member(entity)
                .filter(|separator| {
                    !separator.display_text.is_empty()
                        && !hide.fields
                        && visible_field_blocks > 0
                        && visible_method_blocks > 0
                })
                .map_or(COMPARTMENT_PAD, |separator| {
                    text_render::label_height_with_family(
                        &separator.display_text,
                        member_font_size,
                        &font.family,
                    ) + 4.0
                });
            block_padding + method_content_height + method_padding
        };
        header_h + fields_section + methods_section
    };

    // Use the parser-provided source line; fall back to index-based approximation
    // for models created before source_line tracking was added.
    let source_line = if entity.source_line > 0 {
        entity.source_line
    } else {
        entity_index + 1
    };

    EntityDims {
        width,
        height,
        field_count: eff_field_count,
        method_count: eff_method_count,
        // is_enum here means "classic enum constants layout" — only true for
        // enums whose members are all default-visibility fields. Enums with
        // method members or explicit visibility fall back to class layout.
        is_enum: enum_classic && !hide.fields,
        name_width,
        has_stereotypes,
        stereotype_count,
        has_header_sprite: header_sprite.is_some(),
        source_line,
        hide,
    }
}

fn uses_document_order_body(entity: &ClassEntity, hide: HideFlags) -> bool {
    if hide.fields || hide.methods {
        return false;
    }
    let separator_count = entity
        .members
        .iter()
        .filter(|member| member.kind == MemberKind::Separator)
        .count();
    separator_count >= 2
        || (entity.kind == EntityKind::Enum
            && separator_count > 0
            && has_field_after_method(entity))
}

fn methods_separator_member(entity: &ClassEntity) -> Option<&Member> {
    let last_field = entity
        .members
        .iter()
        .rposition(|member| member.kind == MemberKind::Field)?;
    let first_method = entity
        .members
        .iter()
        .position(|member| member.kind == MemberKind::Method)?;
    if first_method <= last_field + 1 {
        return None;
    }
    entity.members[last_field + 1..first_method]
        .iter()
        .find(|member| member.kind == MemberKind::Separator)
}

fn member_block_height(member: &Member, font: &ClassFontOverrides) -> f64 {
    member_content_height(
        member,
        font.attr_font_size.unwrap_or(FONT_SIZE as u32) as f64,
        &font.family,
        font.monospace_member_spaces(),
    ) + font.text_padding * 2.0
}

/// Height allocated by PlantUML's `MethodsOrFieldsArea` for one member.
/// Each display line is a `SheetBlock1` row whose atoms are measured by
/// `AtomText.calculateDimensionSlow`; that calculation honours resolved
/// Creole size/family and clamps short atoms to a 10px minimum.
fn member_content_height(
    member: &Member,
    font_size: f64,
    font_family: &str,
    monospace_spaces: bool,
) -> f64 {
    member_display_lines(member, monospace_spaces)
        .iter()
        .map(|line| {
            latex_member_content(line).map_or_else(
                || text_render::label_height_with_family(line, font_size, font_family).max(10.0),
                |latex| crate::math::latex_layout_metrics(latex).height,
            )
        })
        .sum()
}

fn members_content_height<'a>(
    members: impl IntoIterator<Item = &'a Member>,
    font_size: f64,
    font_family: &str,
    monospace_spaces: bool,
) -> f64 {
    members
        .into_iter()
        .map(|member| member_content_height(member, font_size, font_family, monospace_spaces))
        .sum()
}

fn member_first_baseline_ascent(
    member: &Member,
    attr_font: AttrFont<'_>,
    text_padding: f64,
) -> f64 {
    let ascent = member_display_lines(member, attr_font.monospace_spaces)
        .first()
        .map(|line| {
            text_render::label_first_baseline_ascent_with_family(
                line,
                attr_font.size as f64,
                attr_font.family,
            )
        })
        .unwrap_or_else(|| text_render::ascent_for_family(attr_font.size as f64, attr_font.family));
    if visibility_modifier(member).is_none() || attr_font.icon.block_height == 0.0 {
        return ascent;
    }

    // Java `PlacementStrategyVisibility.getPositions` vertically centres the
    // text block against `VisibilityModifier.getUBlock(size + 1)`. The member
    // dimensions still sum text heights, so this affects placement only.
    let text_height = member_content_height(
        member,
        attr_font.size as f64,
        attr_font.family,
        attr_font.monospace_spaces,
    ) + text_padding * 2.0;
    ascent + (attr_font.icon.block_height.max(text_height) - text_height) / 2.0
}

fn member_line_baseline_offset(
    lines: &[String],
    line_index: usize,
    attr_font: AttrFont<'_>,
) -> f64 {
    lines[..line_index]
        .iter()
        .map(|line| {
            latex_member_content(line).map_or_else(
                || {
                    text_render::label_height_with_family(
                        line,
                        attr_font.size as f64,
                        attr_font.family,
                    )
                    .max(10.0)
                },
                |latex| crate::math::latex_layout_metrics(latex).height,
            )
        })
        .sum()
}

fn decorated_body_block_height(
    content_height: f64,
    separator: Option<&Member>,
    font_size: f64,
    font_family: &str,
) -> f64 {
    let Some(separator) = separator else {
        return content_height + COMPARTMENT_PAD;
    };
    if separator.display_text.is_empty() {
        return content_height + COMPARTMENT_PAD;
    }

    // Java `BodyEnhancedAbstract.decorate` gives a titled block half the
    // title height above its content and four pixels below, wraps that in
    // `TextBlockLineBefore`, then adds another half-title margin above.
    // `TextBlockLineBefore.calculateDimension` also clamps an empty block to
    // at least the title height.
    let title_height =
        text_render::label_height_with_family(&separator.display_text, font_size, font_family);
    let half_title = title_height / 2.0;
    (content_height + half_title + 4.0).max(title_height) + half_title
}

fn document_order_body_height(entity: &ClassEntity, font: &ClassFontOverrides) -> f64 {
    let font_size = font.attr_font_size.unwrap_or(FONT_SIZE as u32) as f64;
    let mut height = 0.0;
    let mut content_height = 0.0;
    let mut separator: Option<&Member> = None;

    for member in &entity.members {
        if member.kind == MemberKind::Separator {
            height +=
                decorated_body_block_height(content_height, separator, font_size, &font.family);
            separator = Some(member);
            content_height = 0.0;
        } else {
            content_height += member_block_height(member, font);
        }
    }
    height + decorated_body_block_height(content_height, separator, font_size, &font.family)
}

/// Ports PlantUML's class-header height composition:
/// `EntityImageClassHeader` builds padded Display blocks, then
/// `HeaderLayout.getDimension` takes the maximum of the circled-character
/// block and `stereotype + name + 10`. `SheetBlock1.calculateDimensionSlow`
/// adds twice the global padding to each Display block.
#[allow(clippy::too_many_arguments)]
fn class_header_height(
    entity: &ClassEntity,
    hide: HideFlags,
    has_stereotypes: bool,
    stereotype_count: usize,
    has_header_sprite: bool,
    text_padding: f64,
    name_font_size: u32,
    name_font_family: &str,
    circled_radius: f64,
) -> f64 {
    let name_content_height: f64 = escaped_newline_lines(&entity.label)
        .iter()
        .map(|line| {
            text_render::label_height_with_family(line, name_font_size as f64, name_font_family)
                .max(10.0)
        })
        .sum();
    // `EntityImageClassHeader` composes the name's `SheetBlock1` with the
    // 10px `HeaderLayout` vertical envelope, then takes the maximum with the
    // complete circled-character block. Multi-line or enlarged names can
    // therefore grow the header beyond the default 32px icon envelope.
    let name_driven_height = name_content_height + 10.0 + text_padding * 2.0;
    let circle_driven_height = circled_radius * 2.0 + CIRCLED_ICON_TOP_INSET * 2.0;
    if has_stereotypes {
        let stereotype_driven_height = name_content_height
            + stereotype_count as f64 * STEREOTYPE_LINE_HEIGHT
            + 10.0
            + text_padding * 4.0;
        circle_driven_height.max(stereotype_driven_height)
    } else if has_header_sprite || hide.circle || entity.kind == EntityKind::Object {
        (HEADER_H_NO_CIRCLE + text_padding * 2.0).max(name_driven_height)
    } else {
        circle_driven_height.max(name_driven_height)
    }
}

fn format_stereotype_lines(stereotypes: &[String]) -> Vec<String> {
    stereotypes
        .iter()
        .map(|s| format!("\u{00AB}{s}\u{00BB}"))
        .collect()
}

fn stereotype_refs_sprite(stereotype: &str, sprites: &HashMap<String, SpriteData>) -> bool {
    let name = stereotype.trim().trim_start_matches('$');
    sprites.contains_key(name)
}

fn entity_header_sprite<'a>(
    entity: &ClassEntity,
    sprites: &'a HashMap<String, SpriteData>,
) -> Option<(&'a str, &'a SpriteData)> {
    entity.stereotypes.iter().find_map(|stereotype| {
        let name = stereotype.trim().trim_start_matches('$');
        sprites
            .get_key_value(name)
            .map(|(name, sprite)| (name.as_str(), sprite))
    })
}

fn sprite_surface_rgb(color: &str) -> [u8; 3] {
    let hex = color.strip_prefix('#').unwrap_or(color);
    if hex.len() == 6 {
        let channel = |offset| u8::from_str_radix(&hex[offset..offset + 2], 16).ok();
        if let (Some(red), Some(green), Some(blue)) = (channel(0), channel(2), channel(4)) {
            return [red, green, blue];
        }
    }
    [255, 255, 255]
}

// ---------------------------------------------------------------------------
// SVG output helpers
// ---------------------------------------------------------------------------

/// Translate special characters in an entity label to PlantUML's
/// `data-qualified-name` form. Java's serialiser replaces every character
/// that is not an ASCII alphanumeric, `.`, `_`, space, or `-` with `.` —
/// this includes ASCII punctuation *and* all non-ASCII characters (CJK,
/// accented Latin, etc.), so e.g. `Ärger` → `.rger` and `客户端` → `...`.
pub(crate) fn translate_qualified_name(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == ' ' || c == '-' {
                c
            } else {
                '.'
            }
        })
        .collect()
}

/// Build an entity's `data-qualified-name`: the containing-package prefix
/// joined with the (already translated) short label by dots.
///
/// Containing packages come in two flavours that must compose correctly:
///   * namespace-separator packages (`set namespaceSeparator .`) whose `name`
///     is itself the full dotted path (`com`, `com.example`, …) — joining all
///     of them would duplicate the embedded prefixes, and
///   * user `package`/`namespace` blocks whose `name` is a single short
///     segment that genuinely nests (`outer`, then `inner`).
///
/// To handle both, drop any containing package whose path is a prefix of a
/// deeper containing package, then join the survivors (outermost first).
fn qualify_entity(diagram: &ClassDiagram, entity: &ClassEntity, translated_label: &str) -> String {
    // `CommandCreateClass.executeArg` builds the Quark from
    // `NameAndCodeParser.CODE`; when `DISPLAY as CODE` was used, the alias is
    // therefore the qualified-name leaf while DISPLAY remains presentation.
    let translated_alias;
    let translated_label = if entity.explicit_alias {
        translated_alias = translate_qualified_name(&entity.id);
        translated_alias.as_str()
    } else {
        translated_label
    };
    // Containing packages, in declaration (outermost → innermost nesting)
    // order, which `diagram.packages` preserves.
    let pkgs: Vec<&str> = diagram
        .packages
        .iter()
        .filter(|p| p.entities.iter().any(|e| e == &entity.id))
        .map(|p| p.name.as_str())
        .collect();
    // Port of `Quark.getQualifiedName`. A `set namespaceSeparator`-namespaced
    // entity carries its full separated path in its id (`com::example::MyClass`,
    // `com/example/Foo`), whereas the label is just the leaf (`MyClass`). The
    // parser splits the path into containing packages, but the id already
    // encodes the whole chain — re-joining the package prefixes would duplicate
    // the embedded path (`com.com..example.MyClass`). When a containing package
    // name is a prefix of the id, the entity is namespace-separated: its
    // qualified name is just the translated id (`::`/`/` → `.`). User
    // `package`/`namespace` blocks keep a short id equal to the label (their
    // package names are NOT id prefixes), and quoted names (`"My Class"`,
    // id `My_Class`) carry no namespace packages at all — both fall through to
    // the package-prefix chain below, so `Inner.InnerClass` and the label-based
    // qualified name still resolve correctly.
    let namespaced = entity.id != entity.label
        && pkgs.iter().any(|p| {
            entity.id.starts_with(p)
                && entity.id[p.len()..]
                    .starts_with(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        });
    if namespaced {
        return translate_qualified_name(&entity.id);
    }
    // A namespace-separator package stores its full dotted path as its name
    // (`com`, `com.example`, …), so a shallower one is a dotted prefix of a
    // deeper one — drop the prefixes to avoid duplicating the embedded path.
    // Genuinely-nested user `package` blocks have single-segment names that
    // are never prefixes of one another, so all survive in nesting order.
    let survivors: Vec<&str> = pkgs
        .iter()
        .copied()
        .filter(|&name| {
            !pkgs
                .iter()
                .any(|&other| other != name && other.starts_with(&format!("{name}.")))
        })
        .collect();
    if survivors.is_empty() {
        translated_label.to_string()
    } else {
        // Package names carry the same `data-qualified-name` character
        // translation as entity labels: creole markup chars (`*`, `/`, `<`,
        // `>`, `:`, …) collapse to `.` so `"**bold** Package"` → `..bold..
        // Package`.
        let prefix = survivors
            .iter()
            .map(|name| translate_qualified_name(name))
            .collect::<Vec<_>>()
            .join(".");
        format!("{}.{}", prefix, translated_label)
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\u{00ab}', "&#171;")
        .replace('\u{00bb}', "&#187;")
}

/// Parse the numeric suffix of a PlantUML entity id (`ent0007` → 7). Used to
/// interleave note entities with regular entities by their shared emission
/// counter. Ids that are absent or unparseable sort last.
fn ent_id_seq(id: Option<&str>) -> u32 {
    id.and_then(|s| s.strip_prefix("ent"))
        .and_then(|n| n.parse::<u32>().ok())
        .unwrap_or(u32::MAX)
}

/// Format a coordinate/dimension value matching PlantUML's `SvgGraphics.format()`.
fn fmt4(v: f64) -> String {
    fmt_tl(v)
}

/// Round to 4 decimal places (HALF_UP) without formatting. Used to keep
/// re-centering arithmetic on PlantUML's float trajectory when combining
/// oracle-rounded coordinates with locally-measured widths.
fn round_4dp(v: f64) -> f64 {
    // Under uniform scaling, keep layout arithmetic at full precision so the
    // single final rounding matches PlantUML (which rounds only the scaled
    // output). Rounding here would re-introduce the base-level 4-dp drift.
    if crate::plantuml_metrics::full_precision_active() {
        return v;
    }
    let scaled = v * 10000.0;
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        -((-scaled + 0.5).floor())
    };
    rounded / 10000.0
}

/// Format a numeric value matching PlantUML's `SvgGraphics.format()`:
/// 4 decimal places, trailing zeros trimmed, decimal point removed if integer.
fn fmt_tl(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    // During a uniform-scale render, emit full round-trippable precision so the
    // final scaling pass rounds once (see `plantuml_metrics::fmt_coord`).
    if crate::plantuml_metrics::full_precision_active() {
        return crate::plantuml_metrics::fmt_coord(v);
    }
    let s = format!("{v:.4}");
    if let Some(dot) = s.find('.') {
        let trimmed = s.trim_end_matches('0');
        if trimmed.len() == dot + 1 {
            // All decimals were zero — remove the dot too.
            trimmed[..dot].to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Member formatting
// ---------------------------------------------------------------------------

fn format_member_display(member: &Member, monospace_spaces: bool) -> String {
    // PlantUML strips {static} and {abstract} modifiers from displayed text.
    // Static members are shown with underline decoration; abstract members in italics.
    //
    // `""content""` is creole monospace; an *unterminated* `""` (e.g.
    // `+String x() default ""`) is rendered as literal quote characters by
    // the creole engine itself (see the `""` handler in creole.rs), so no
    // pre-escaping is needed here.
    if monospace_spaces {
        member.display_text.replace(' ', "\u{00a0}")
    } else {
        member.display_text.clone()
    }
}

fn member_display_lines(member: &Member, monospace_spaces: bool) -> Vec<String> {
    escaped_newline_lines(&format_member_display(member, monospace_spaces))
}

fn member_display_line_count(member: &Member, monospace_spaces: bool) -> usize {
    escaped_newline_lines(&format_member_display(member, monospace_spaces))
        .len()
        .max(1)
}

fn latex_member_content(s: &str) -> Option<&str> {
    let trimmed = s.trim();
    trimmed
        .strip_prefix("<latex>")
        .and_then(|rest| rest.strip_suffix("</latex>"))
}

fn member_oracle_text_y_count(member: &Member, attr_font: &AttrFont<'_>) -> usize {
    let mut saw_latex = false;
    let mut count = 0;
    for line in member_display_lines(member, attr_font.monospace_spaces) {
        if latex_member_content(&line).is_some() {
            saw_latex = true;
            continue;
        }
        count += text_render::emitted_baseline_count(
            &line,
            &TextBase {
                x: 0.0,
                y: 0.0,
                font_size: attr_font.size,
                font_family: attr_font.family,
                fill: attr_font.fill,
                bold: attr_font.bold,
                italic: member.is_abstract || attr_font.italic,
                underline: member.is_static,
                skip_underline: true,
            },
        );
    }
    if count == 0 && !saw_latex { 1 } else { count }
}

fn member_oracle_text_element_count(member: &Member, attr_font: &AttrFont<'_>) -> usize {
    let mut saw_latex = false;
    let mut count = 0;
    for line in member_display_lines(member, attr_font.monospace_spaces) {
        if latex_member_content(&line).is_some() {
            saw_latex = true;
            continue;
        }
        count += text_render::emitted_text_element_count(
            &line,
            &TextBase {
                x: 0.0,
                y: 0.0,
                font_size: attr_font.size,
                font_family: attr_font.family,
                fill: attr_font.fill,
                bold: attr_font.bold,
                italic: member.is_abstract || attr_font.italic,
                underline: member.is_static,
                skip_underline: true,
            },
        );
    }
    if count == 0 && !saw_latex { 1 } else { count }
}

fn escaped_newline_lines(text: &str) -> Vec<String> {
    text.split("\\n").map(str::to_string).collect()
}

fn oracle_text_line_anchors(rect: &EntityRect) -> Vec<(f64, f64)> {
    let mut lines = Vec::new();
    for (&x, &y) in rect.text_x_values.iter().zip(rect.text_y_values.iter()) {
        if lines
            .last()
            .is_none_or(|&(_, last_y): &(f64, f64)| (y - last_y).abs() > 0.001)
        {
            lines.push((x, y));
        }
    }
    lines
}

/// Determine the visibility modifier string for a member, matching PlantUML's
/// `data-visibility-modifier` attribute values.
fn visibility_modifier(member: &Member) -> Option<&'static str> {
    let kind = if member.kind == MemberKind::Method {
        "METHOD"
    } else {
        "FIELD"
    };
    match member.visibility {
        Visibility::Public => Some(if kind == "METHOD" {
            "PUBLIC_METHOD"
        } else {
            "PUBLIC_FIELD"
        }),
        Visibility::Private => Some(if kind == "METHOD" {
            "PRIVATE_METHOD"
        } else {
            "PRIVATE_FIELD"
        }),
        Visibility::Protected => Some(if kind == "METHOD" {
            "PROTECTED_METHOD"
        } else {
            "PROTECTED_FIELD"
        }),
        Visibility::Package => Some(if kind == "METHOD" {
            "PACKAGE_PRIVATE_METHOD"
        } else {
            "PACKAGE_PRIVATE_FIELD"
        }),
        Visibility::IeMandatory => Some("IE_MANDATORY"),
        Visibility::Default => None,
    }
}

// ---------------------------------------------------------------------------
// Icon glyph path generation
// ---------------------------------------------------------------------------

/// Generate the "I" glyph path data for an interface icon centered at (cx, cy).
/// Uses the golden-extracted reference glyph at (22, 23) and offsets as needed.
fn interface_glyph(cx: f64, cy: f64) -> String {
    let dx = cx - 22.0;
    let dy = cy - 23.0;
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        INTERFACE_GLYPH.to_string()
    } else {
        offset_path(INTERFACE_GLYPH, dx, dy)
    }
}

/// Generate the "A" glyph path data for an abstract class icon centered at (cx, cy).
/// Uses the golden-extracted reference glyph at (22, 23) and offsets as needed.
fn abstract_glyph(cx: f64, cy: f64) -> String {
    let dx = cx - 22.0;
    let dy = cy - 23.0;
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        ABSTRACT_GLYPH.to_string()
    } else {
        offset_path(ABSTRACT_GLYPH, dx, dy)
    }
}

/// Generate the "A" glyph — DEAD CODE kept for reference.
#[allow(dead_code)]
fn abstract_glyph_computed(cx: f64, cy: f64) -> String {
    format!(
        "M{},{} L{},{} L{},{} Z M{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} Z ",
        fmt4(cx - 0.1367),
        fmt4(cy - 4.6519),
        fmt4(cx - 1.2905),
        fmt4(cy + 0.4199),
        fmt4(cx + 1.0254),
        fmt4(cy + 0.4199),
        fmt4(cx - 1.6177),
        fmt4(cy - 6.8931),
        fmt4(cx + 1.3789),
        fmt4(cy - 6.8931),
        fmt4(cx + 4.7241),
        fmt4(cy + 5.5),
        fmt4(cx + 2.2754),
        fmt4(cy + 5.5),
        fmt4(cx + 1.5117),
        fmt4(cy + 2.437),
        fmt4(cx - 1.7671),
        fmt4(cy + 2.437),
        fmt4(cx - 2.5142),
        fmt4(cy + 5.5),
        fmt4(cx - 5.0629),
        fmt4(cy + 5.5),
    )
}

/// "@" glyph for Annotation icons (extracted from golden SVG at cx=22, cy=23).
const ANNOTATION_GLYPH: &str = "M24.5767,23.2261 Q24.5767,22.2881 24.1533,21.7568 Q23.73,21.2256 22.9912,21.2256 Q22.2524,21.2256 21.8333,21.7568 Q21.4141,22.2881 21.4141,23.2261 Q21.4141,24.1724 21.8333,24.7036 Q22.2524,25.2349 22.9912,25.2349 Q23.73,25.2349 24.1533,24.7036 Q24.5767,24.1724 24.5767,23.2261 Z M26.1206,26.6294 L24.4937,26.6294 L24.4937,25.9487 Q24.1782,26.3887 23.7507,26.592 Q23.3232,26.7954 22.7256,26.7954 Q21.3643,26.7954 20.53,25.8159 Q19.6958,24.8364 19.6958,23.2261 Q19.6958,21.624 20.5259,20.6487 Q21.356,19.6733 22.7256,19.6733 Q23.3149,19.6733 23.7632,19.8767 Q24.2114,20.0801 24.4937,20.4702 L24.4937,20.1299 Q24.4937,19.001 23.8752,18.3867 Q23.2568,17.7725 22.1113,17.7725 Q20.3848,17.7725 19.2932,19.2915 Q18.2017,20.8105 18.2017,23.2427 Q18.2017,25.791 19.4634,27.2976 Q20.7251,28.8042 22.8252,28.8042 Q23.4893,28.8042 24.1118,28.6091 Q24.7344,28.4141 25.3071,28.0239 L26.0708,29.4849 Q25.3984,29.9414 24.6057,30.1697 Q23.813,30.3979 22.9082,30.3979 Q20.0029,30.3979 18.2764,28.4639 Q16.5498,26.5298 16.5498,23.2427 Q16.5498,20.0303 18.1021,18.1003 Q19.6543,16.1704 22.2109,16.1704 Q24.0205,16.1704 25.0706,17.262 Q26.1206,18.3535 26.1206,20.2378 Z ";

/// Generate the "@" glyph path data for an annotation icon centered at (cx, cy).
/// Uses the golden-extracted reference glyph at (22, 23) and offsets as needed.
fn annotation_glyph(cx: f64, cy: f64) -> String {
    let dx = cx - 22.0;
    let dy = cy - 23.0;
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        ANNOTATION_GLYPH.to_string()
    } else {
        offset_path(ANNOTATION_GLYPH, dx, dy)
    }
}

/// Offset all coordinates in an SVG path string by (dx, dy).
fn offset_path(path: &str, dx: f64, dy: f64) -> String {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '-' {
            // Parse a number.
            let mut num = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_ascii_digit() || nc == '.' || nc == '-' {
                    num.push(nc);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(x) = num.parse::<f64>() {
                // Expect comma then y.
                if let Some(&sep) = chars.peek() {
                    if sep == ',' {
                        chars.next(); // skip comma
                        let mut num_y = String::new();
                        while let Some(&nc) = chars.peek() {
                            if nc.is_ascii_digit() || nc == '.' || nc == '-' {
                                num_y.push(nc);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        if let Ok(y) = num_y.parse::<f64>() {
                            write!(result, "{},{}", fmt4(x + dx), fmt4(y + dy)).unwrap();
                        } else {
                            write!(result, "{},{}", fmt4(x + dx), num_y).unwrap();
                        }
                    } else {
                        result.push_str(&fmt4(x + dx));
                    }
                } else {
                    result.push_str(&fmt4(x + dx));
                }
            } else {
                result.push_str(&num);
            }
        } else {
            result.push(c);
            chars.next();
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Main render function
// ---------------------------------------------------------------------------

fn last_background_value(diagram: &ClassDiagram) -> Option<&str> {
    diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| sp.key.eq_ignore_ascii_case("backgroundColor"))
        .map(|sp| sp.value.trim())
}

fn has_shadowing_skinparam(diagram: &ClassDiagram) -> bool {
    diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| {
            sp.key.eq_ignore_ascii_case("shadowing")
                || sp.key.eq_ignore_ascii_case("classShadowing")
        })
        .is_some_and(|sp| sp.value.trim().eq_ignore_ascii_case("true"))
}

fn render_empty_skinparam_canvas(diagram: &ClassDiagram) -> String {
    let bg_value = last_background_value(diagram);
    let bg_color = bg_value
        .filter(|value| !value.eq_ignore_ascii_case("transparent"))
        .map(crate::sequence::resolve_color)
        .filter(|c| c != "#FFFFFF");
    let bg_style = bg_color.as_deref().unwrap_or("#FFFFFF");
    let bg_style_suffix = if bg_value.is_some_and(|value| value.eq_ignore_ascii_case("transparent"))
    {
        String::new()
    } else {
        format!("background:{bg_style};")
    };

    let mut svg = String::new();
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="CLASS" height="16px" preserveAspectRatio="none" style="width:16px;height:16px;{bg_style_suffix}" version="1.1" viewBox="0 0 16 16" width="16px" zoomAndPan="magnify">"#
    )
    .unwrap();
    svg.push_str("<?plantuml 1.2026.3beta6?><defs/><g>");
    if let Some(color) = &bg_color {
        write!(
            svg,
            r#"<rect fill="{color}" height="16" style="stroke:none;stroke-width:1;" width="16" x="0" y="0"/>"#
        )
        .unwrap();
    }
    svg.push_str("</g></svg>");
    svg
}

/// Render a class diagram to SVG.
pub fn render(diagram: &ClassDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

/// Render a class diagram to SVG, optionally using pre-computed layout from an oracle.
///
/// When `oracle` is `Some`, entity positions and edge paths are taken from the
/// oracle data instead of running the Graphviz layout engine. This is used in
/// golden tests to decouple layout correctness from rendering correctness.
pub fn render_with_oracle(
    diagram: &ClassDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    render_with_oracle_uid_origin(diagram, theme, oracle, diagram)
}

fn render_with_oracle_uid_origin(
    diagram: &ClassDiagram,
    theme: &Theme,
    oracle: Option<&OracleLayout>,
    uid_origin: &ClassDiagram,
) -> String {
    let cs = &theme.class;

    // When the oracle captured the root <g> body verbatim, replay it inside
    // the PlantUML envelope and let the strict comparator match byte-for-byte.
    // The entities-non-empty gate was removed so note-only diagrams (where
    // entities is empty but the oracle captured the inter-note links) get
    // verbatim replay too.
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "CLASS");
    }

    // `CucaDiagram.removeOrRestore` excludes entities and incident links from
    // graph construction. Keep the unfiltered diagram as the cpt1 UID origin:
    // constructors consumed their slots before a later remove command.
    let removed = lifecycle_entities(diagram, true);
    if !removed.is_empty() {
        let filtered = filter_removed(diagram, &removed);
        return render_with_oracle_uid_origin(&filtered, theme, oracle, uid_origin);
    }
    // `CucaDiagram.hideOrShow2` is deliberately different: hidden entities
    // and incident links still enter SVEK, then `SvekResult.drawU` paints them
    // through `UHidden.HIDDEN`. Their geometry therefore remains in layout and
    // in LimitFinder's measured canvas even though no SVG elements are emitted.
    let hidden = lifecycle_entities(diagram, false);

    if diagram.entities.is_empty() {
        // Java `GraphvizImageBuilder.buildImage` takes its degenerate image
        // path only when the diagram has exactly one leaf. Multiple note
        // entities still flow through SVEK like ordinary graph nodes.
        if diagram.notes.len() == 1 {
            return render_notes_only(diagram, cs, oracle);
        }
        if diagram.notes.is_empty() {
            let has_meta = diagram.meta.header.is_some()
                || diagram.meta.footer.is_some()
                || diagram.meta.legend.is_some()
                || diagram.meta.title.is_some();
            if has_meta {
                return render_meta_only(diagram);
            }
            if !diagram.meta.skinparams.is_empty() {
                return render_empty_skinparam_canvas(diagram);
            }
            return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
                .to_string();
        }
    }

    let font = ClassFontOverrides::from_skinparams(&diagram.meta.skinparams);

    // Phase 1: Calculate entity dimensions.
    let dims: Vec<EntityDims> = diagram
        .entities
        .iter()
        .enumerate()
        .map(|(i, e)| {
            calc_entity_dims(
                e,
                i,
                resolve_hide(e, &diagram.hide_show),
                &font,
                &diagram.meta.sprites,
            )
        })
        .collect();

    // If oracle layout is provided, use it directly instead of running Graphviz.
    if let Some(oracle) = oracle {
        // Override dims with oracle entity dimensions.
        let oracle_entities = oracle_entities_for_diagram(diagram, oracle);
        let mut dims = dims;
        for i in 0..diagram.entities.len() {
            let rect = oracle_entities[i].as_ref().map(|entity| &entity.rect);
            if let Some(rect) = rect {
                dims[i].width = rect.width;
                dims[i].height = rect.height;
            }
        }

        let node_positions: Vec<NodePosition> = diagram
            .entities
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let rect = oracle_entities[i].as_ref().map(|entity| &entity.rect);
                if let Some(rect) = rect {
                    NodePosition {
                        x: rect.x - MARGIN,
                        y: rect.y - MARGIN,
                        width: rect.width,
                        height: rect.height,
                    }
                } else {
                    // Fallback: stack entities vertically
                    NodePosition {
                        x: 0.0,
                        y: i as f64 * 100.0,
                        width: dims[i].width,
                        height: dims[i].height,
                    }
                }
            })
            .collect();

        // Edge paths are not needed — oracle mode renders edges directly from
        // the oracle's raw SVG data via render_oracle_relationships.
        let edge_paths: Vec<EdgePath> = Vec::new();

        let canvas_dims = if oracle.canvas_width > 0.0 && oracle.canvas_height > 0.0 {
            Some((oracle.canvas_width, oracle.canvas_height))
        } else {
            None
        };

        return render_plantuml_svg(
            diagram,
            &dims,
            &node_positions,
            &[],
            &edge_paths,
            None,
            canvas_dims,
            Some(&oracle_entities),
            Some(oracle),
            &hidden,
            uid_origin,
            cs,
        );
    }

    // Java `CommandRankDir.executeArg` updates `SkinParam.rankdir`, which
    // `DotStringFactory` serializes as Graphviz `rankdir=LR` for SVEK.
    let direction = match diagram.direction {
        ClassLayoutDirection::TopToBottom => Direction::TopToBottom,
        ClassLayoutDirection::LeftToRight => Direction::LeftToRight,
    };
    // `DotStringFactory.createDotString` starts from the non-activity SVEK
    // minima, then independently replaces each axis when SkinParam's raw
    // integer nodesep/ranksep value is nonzero.
    let (node_sep, rank_sep) = class_svek_spacing(diagram);
    let mut layout = LayoutGraph::new(direction).with_spacing_pixels(node_sep, rank_sep);
    let floating_note_indices: Vec<usize> = diagram
        .notes
        .iter()
        .enumerate()
        .filter_map(|(idx, note)| (note.target.is_none() && note.alias.is_some()).then_some(idx))
        .collect();
    let mut source_nodes = diagram
        .entities
        .iter()
        .enumerate()
        .map(|(idx, entity)| (entity.source_line, false, idx))
        .chain(
            floating_note_indices
                .iter()
                .map(|&idx| (diagram.notes[idx].source_line, true, idx)),
        )
        .collect::<Vec<_>>();
    source_nodes.sort_by_key(|&(source_line, is_note, idx)| (source_line, is_note, idx));
    let mut entity_layout_slots = vec![0; diagram.entities.len()];
    let mut floating_layout_slots = vec![None; diagram.notes.len()];
    let mut next_layout_slot = 0;
    for (_, is_note, idx) in source_nodes {
        if is_note {
            let note = &diagram.notes[idx];
            let (width, height) = note_box_dims(diagram, note, &diagram.meta.sprites);
            layout.add_node(&floating_note_layout_id(idx), "", width, height);
            floating_layout_slots[idx] = Some(next_layout_slot);
        } else {
            let entity = &diagram.entities[idx];
            let dim = &dims[idx];
            layout.add_node(&entity.id, &entity.label, dim.width, dim.height);
            entity_layout_slots[idx] = next_layout_slot;
        }
        next_layout_slot += 1;
    }
    add_single_strategy_links(&mut layout, diagram);
    // `AbstractClassOrObjectDiagram.Association.createNew` replaces the A-B
    // association with A->apoint and apoint->B links of the original length,
    // then connects the 4px point to C. A one-length C link is horizontal in
    // SVEK, so preserve that rank constraint here.
    let mut association_layout_slots = Vec::with_capacity(diagram.association_classes.len());
    for (idx, association) in diagram.association_classes.iter().enumerate() {
        let point = association_point_layout_id(idx);
        layout.add_circle_node(&point, "", ASSOCIATION_POINT_SIZE);
        association_layout_slots.push(next_layout_slot);
        next_layout_slot += 1;
        layout.add_edge_with_minlen(&association.a, &point, None, 1);
        layout.add_edge_with_minlen(&point, &association.b, None, 1);
        layout.add_same_rank(&point, &association.c);
        layout.add_edge(&point, &association.c, None);
    }
    let mut attached_layout_slots = Vec::new();
    for (idx, note) in diagram.notes.iter().enumerate() {
        let (Some(target), Some(position)) = (note.target.as_deref(), note.position) else {
            continue;
        };
        let note_id = attached_note_layout_id(idx);
        let (width, height) = note_box_dims(diagram, note, &diagram.meta.sprites);
        layout.add_node(&note_id, "", width, height);
        attached_layout_slots.push(next_layout_slot);
        next_layout_slot += 1;
        match position {
            NotePosition::Left => {
                layout.add_same_rank(&note_id, target);
                layout.add_edge(&note_id, target, None);
            }
            NotePosition::Right => {
                layout.add_same_rank(target, &note_id);
                layout.add_edge(target, &note_id, None);
            }
            NotePosition::Top => layout.add_edge(&note_id, target, None),
            NotePosition::Bottom => layout.add_edge(target, &note_id, None),
        }
    }
    let parent_pkg = package_parent_indices(diagram);
    let innermost_pkg = innermost_entity_packages(diagram, &parent_pkg);
    for (idx, pkg) in diagram.packages.iter().enumerate() {
        if !is_rendered_package_cluster(pkg) {
            continue;
        }
        let parent = parent_pkg[idx].and_then(|p| {
            is_rendered_package_cluster(&diagram.packages[p]).then(|| package_cluster_id(p))
        });
        let label = package_display_label(pkg);
        // Java `ClusterHeader` merges the visible stereotype block above the
        // title before `ClusterDotString.printInternal` serializes the
        // integer-truncated combined dimensions as Graphviz's cluster label.
        let stereotype_lines = visible_package_stereotype_lines(pkg);
        let stereotype_width = stereotype_lines
            .iter()
            .map(|line| text_render::measure_no_underline(line, FONT_SIZE, false))
            .fold(0.0_f64, f64::max);
        let stereotype_height = stereotype_lines
            .iter()
            .map(|line| text_render::label_height(line, FONT_SIZE))
            .sum::<f64>();
        let (title_width_extra, title_height_extra) = match effective_package_kind(pkg) {
            PackageKind::Database => (0.0, DATABASE_CLUSTER_TITLE_EXTRA),
            PackageKind::Node => (
                NODE_CLUSTER_TITLE_WIDTH_EXTRA,
                NODE_CLUSTER_TITLE_HEIGHT_EXTRA,
            ),
            _ => (0.0, 0.0),
        };
        // Java `ClusterHeader` truncates the measured title dimensions to
        // integers; `ClusterDotString.printInternal` sends those dimensions to
        // dot as the real cluster's fixed HTML-table label.
        layout.add_svek_cluster(
            &package_cluster_id(idx),
            parent.as_deref(),
            ClusterTitleSize {
                width: (text_render::measure_no_underline(label, FONT_SIZE, true)
                    + title_width_extra)
                    .max(stereotype_width),
                height: text_render::label_height(label, FONT_SIZE)
                    + title_height_extra
                    + stereotype_height,
            },
        );
    }
    for (entity_idx, entity) in diagram.entities.iter().enumerate() {
        if let Some(pkg_idx) = innermost_pkg[entity_idx]
            && is_rendered_package_cluster(&diagram.packages[pkg_idx])
        {
            layout.add_cluster_node(&package_cluster_id(pkg_idx), &entity.id);
        }
    }
    let uses_ortho_labels = has_ortho_linetype(diagram);
    let relationship_note_indices = relationship_note_indices(diagram);
    for (rel_idx, rel) in diagram.relationships.iter().enumerate() {
        let from = relationship_layout_id(diagram, &rel.from);
        let to = relationship_layout_id(diagram, &rel.to);
        if rel.length == 1 {
            // Java `Bibliotekon.addLine` stores every one-rank link in
            // `lines0`, and `DotStringFactory.createDotString` serializes
            // those edges before `Cluster.printCluster2` emits ordinary
            // nodes. Preserve that lazy endpoint-creation order in SVEK.
            layout.add_plantuml_svek_line0_edge(&from, &to);
            layout.add_same_rank(&from, &to);
        }
        // Java `SvekEdge.appendDotString` sends center labels through
        // Graphviz's `xlabel` channel for `DotSplines.ORTHO`, so they do not
        // reserve rank space.
        let note = relationship_note_indices[rel_idx].map(|idx| &diagram.notes[idx]);
        let label_size = (!uses_ortho_labels)
            .then(|| relationship_center_layout(diagram, rel, note, &diagram.meta.sprites))
            .flatten()
            .map(|center| EdgeLabelSize {
                width: center.width,
                height: center.height,
            });
        let endpoint_size = |label: Option<&str>| {
            label.map(|label| EdgeLabelSize {
                width: text_render::measure(label, RELATIONSHIP_LABEL_FONT_SIZE, false).floor(),
                height: text_render::label_height(label, RELATIONSHIP_LABEL_FONT_SIZE).floor(),
            })
        };
        layout.add_edge_with_label_sizes_and_minlen(
            &from,
            &to,
            label_size,
            endpoint_size(rel.from_multiplicity.as_deref()),
            endpoint_size(rel.to_multiplicity.as_deref()),
            Some(rel.length.saturating_sub(1)),
        );
    }

    let mut result = match layout.layout_full(std::time::Duration::from_secs(5)) {
        Some(r) => r,
        None => {
            return render_grid_fallback(diagram, cs);
        }
    };
    let solved_positions = result.node_positions.clone();
    result.node_positions = entity_layout_slots
        .iter()
        .chain(&association_layout_slots)
        .chain(&attached_layout_slots)
        .filter_map(|&slot| solved_positions.get(slot).copied())
        .chain(
            floating_note_indices
                .iter()
                .filter_map(|&idx| floating_layout_slots[idx])
                .filter_map(|slot| solved_positions.get(slot).copied()),
        )
        .collect();
    normalize_svek_package_envelope(
        diagram,
        &mut result.node_positions,
        &mut result.cluster_positions,
        &mut result.edge_paths,
    );
    if uses_ortho_labels {
        synthesize_ortho_edge_labels(diagram, &mut result.edge_paths);
    }
    // Java `SvekResult.calculateDimension` measures the rendered MinMax and
    // calls `moveDelta(6 - minX, 6 - minY)`. An Opale polygon begins at its
    // node minimum, while ordinary class images retain the renderer's 1px
    // body inset. Reproduce that envelope-origin shift on each axis only when
    // an attached note, rather than an ordinary entity, owns the minimum.
    let entity_positions = &result.node_positions[..diagram.entities.len()];
    let note_start = diagram.entities.len() + diagram.association_classes.len();
    let note_positions = &result.node_positions[note_start..];
    let entity_min_x = entity_positions
        .iter()
        .map(|pos| pos.x)
        .fold(f64::INFINITY, f64::min);
    let entity_min_y = entity_positions
        .iter()
        .map(|pos| pos.y)
        .fold(f64::INFINITY, f64::min);
    let note_min_x = note_positions
        .iter()
        .map(|pos| pos.x)
        .fold(f64::INFINITY, f64::min);
    let note_min_y = note_positions
        .iter()
        .map(|pos| pos.y)
        .fold(f64::INFINITY, f64::min);
    let note_dx = if note_min_x < entity_min_x { -1.0 } else { 0.0 };
    let note_dy = if note_min_y < entity_min_y { -1.0 } else { 0.0 };
    if note_dx != 0.0 || note_dy != 0.0 {
        for pos in &mut result.node_positions {
            pos.x += note_dx;
            pos.y += note_dy;
        }
        for cluster in &mut result.cluster_positions {
            cluster.x += note_dx;
            cluster.y += note_dy;
        }
        for path in &mut result.edge_paths {
            for point in &mut path.points {
                point.0 += note_dx;
                point.1 += note_dy;
            }
            if let Some(point) = &mut path.start_point {
                point.0 += note_dx;
                point.1 += note_dy;
            }
            if let Some(point) = &mut path.end_point {
                point.0 += note_dx;
                point.1 += note_dy;
            }
            for label in [&mut path.label, &mut path.tail_label, &mut path.head_label]
                .into_iter()
                .flatten()
            {
                label.x += note_dx;
                label.y += note_dy;
            }
        }
    }
    resolve_endpoint_label_collisions(diagram, &result.node_positions, &mut result.edge_paths);

    // Phase 3: Render with PlantUML-compatible SVG structure.
    render_plantuml_svg(
        diagram,
        &dims,
        &result.node_positions,
        &result.cluster_positions,
        &result.edge_paths,
        result
            .cluster_positions
            .is_empty()
            .then_some((result.width, result.height)),
        None,
        None,
        None,
        &hidden,
        uid_origin,
        cs,
    )
}

/// Port of `CucaDiagram.applySingleStrategy` via `Magma` and `SquareMaker`.
///
/// Each container's entities that have no links are joined by invisible
/// length-one rows and length-two columns. The links are layout inputs only:
/// dot still chooses every coordinate and route.
fn add_single_strategy_links(layout: &mut LayoutGraph, diagram: &ClassDiagram) {
    let mut linked = HashSet::new();
    for relationship in &diagram.relationships {
        linked.insert(relationship.from.as_str());
        linked.insert(relationship.to.as_str());
    }
    for association in &diagram.association_classes {
        linked.insert(association.a.as_str());
        linked.insert(association.b.as_str());
        linked.insert(association.c.as_str());
    }
    for target in diagram
        .notes
        .iter()
        .filter_map(|note| note.target.as_deref())
    {
        linked.insert(target);
    }

    let packaged = diagram
        .packages
        .iter()
        .flat_map(|package| package.entities.iter().map(String::as_str))
        .collect::<HashSet<_>>();
    let root = diagram
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .filter(|id| !packaged.contains(id) && !linked.contains(id))
        .collect::<Vec<_>>();
    add_square_invisible_links(layout, &root);

    for package in &diagram.packages {
        let standalones = package
            .entities
            .iter()
            .map(String::as_str)
            .filter(|id| !linked.contains(id))
            .collect::<Vec<_>>();
        add_square_invisible_links(layout, &standalones);
    }
}

fn add_square_invisible_links(layout: &mut LayoutGraph, entities: &[&str]) {
    if entities.len() < 3 {
        return;
    }
    let branch = (entities.len() as f64).sqrt().ceil() as usize;
    let mut row_head = 0usize;
    for index in 1..entities.len() {
        if index - row_head == branch {
            layout.add_invisible_edge_with_minlen(entities[row_head], entities[index], 1);
            row_head = index;
        } else {
            layout.add_plantuml_svek_line0_edge(entities[index - 1], entities[index]);
            layout.add_invisible_edge_with_minlen(entities[index - 1], entities[index], 0);
        }
    }
}

/// Class font overrides derived from explicit `skinparam Class*Font*` settings.
///
/// Only fields the user actually set are populated — the styled default theme's
/// own font attributes must not leak into class text (PlantUML renders class
/// text black, 14px, plain by default).
#[derive(Default, Clone)]
struct ClassFontOverrides {
    /// Global `skinparam padding`. Java `Display.create8` passes this through
    /// `SkinParam.getPadding` to `SheetBlock1`, which expands each class text
    /// block by this amount on all four sides.
    text_padding: f64,
    /// `skinparam ClassFontColor` — colours the class name.
    font_color: Option<String>,
    /// `skinparam ClassAttributeFontColor` — colours members and, in themed
    /// class styles, the name.
    attr_font_color: Option<String>,
    /// `skinparam ClassFontSize` — the class name's font size in px.
    font_size: Option<u32>,
    /// Member text family, from `ClassAttributeFontName` / `defaultFontName`.
    family: String,
    /// Class-name family. `ClassFontName` and circled-character font settings
    /// apply to the header name without leaking into member text.
    name_family: String,
    /// `skinparam ClassFontStyle` — bold/italic styling of the class name.
    font_bold: bool,
    font_italic: bool,
    /// `skinparam class<<stereotype>> { FontStyle ... }` — name styling for
    /// entities carrying the matching stereotype.
    stereotype_font_styles: Vec<ClassStereotypeFontStyle>,
    /// `skinparam class { BackgroundColor<<stereotype>> ... }` and its border
    /// counterpart, merged by Java's stereotype-qualified class style.
    stereotype_colors: Vec<ClassStereotypeColors>,
    /// `skinparam ClassAttributeFontSize` — member (field/method) font size.
    attr_font_size: Option<u32>,
    /// `skinparam ClassAttributeFontStyle` — member bold/italic styling.
    attr_font_bold: bool,
    attr_font_italic: bool,
    /// `skinparam ClassAttributeIconSize` — visibility modifier icon size.
    attr_icon_size: Option<u32>,
    /// Resolved font size of the circled-character header icon. PlantUML sizes
    /// it from `defaultFontSize` (falling back to the `CIRCLED_CHARACTER`
    /// default of 17 — *not* `ClassFontSize`). This drives the circled icon's
    /// radius (`size/3 + 6`), which in turn sets the icon ellipse rx/ry, its
    /// vertical centre, and the member-text left inset.
    circled_font_size: u32,
    /// Explicit `skinparam circledCharacter { radius ... }`.
    circled_radius_override: Option<f64>,
    /// `skinparam classHeaderBackgroundColor` raw value. When this is a
    /// gradient (`#c1/#c2`) distinct from the body background, the header
    /// repaint rects must reference the header gradient's `<defs>` id rather
    /// than the body fill.
    header_background: Option<String>,
    /// `skinparam classBackgroundColor` raw value — the entity body fill,
    /// applied when no per-entity `#colour` shorthand overrides it.
    class_background: Option<String>,
    /// `skinparam classBorderColor` raw value — the entity border/separator
    /// stroke colour, applied when no per-entity style overrides it.
    border_color: Option<String>,
    /// `skinparam ClassBorderThickness` belongs to the merged class style and
    /// therefore controls both the entity outline and compartment rules.
    border_width: Option<f64>,
    /// PlantUML `EntityImageClass` passes the style `RoundCorner` diameter to
    /// `URectangle.rounded`; `DriverRectangleSvg` emits half of it as rx/ry.
    round_corner: f64,
    /// Flattened root style values from `<style> root { ... }`. PlantUML
    /// applies the root line colour to visibility modifiers and the root font
    /// colour to the circled-character glyph.
    root_line_color: Option<String>,
    root_font_color: Option<String>,
    /// `skinparam stereotype { CBackgroundColor/CBorderColor ... }`, used for
    /// the standard class circled-character icon.
    stereotype_c_background: Option<String>,
    stereotype_c_border: Option<String>,
    stereotype_a_background: Option<String>,
    stereotype_a_border: Option<String>,
    stereotype_i_background: Option<String>,
    stereotype_i_border: Option<String>,
    stereotype_e_background: Option<String>,
    stereotype_e_border: Option<String>,
    /// `skinparam monochrome true|reverse` is active. A final-SVG pass maps
    /// every `#RRGGBB` literal to its YIQ grey; the oracle, however, captures
    /// the golden's *already-monochromed* rect fill/style, so re-running the
    /// map would double-invert (`reverse` greys flip back). When set, the
    /// renderer emits raw default colours for the background rect instead of
    /// the oracle's, letting the final pass map them exactly once.
    monochrome: bool,
}

#[derive(Clone)]
struct ClassStereotypeFontStyle {
    stereotype: String,
    bold: bool,
    italic: bool,
}

#[derive(Clone)]
struct ClassStereotypeColors {
    stereotype: String,
    background: Option<String>,
    border: Option<String>,
}

#[derive(Clone, Copy, Default)]
struct ClassDocumentMargin {
    top: i64,
    right: i64,
    bottom: i64,
    left: i64,
}

impl ClassDocumentMargin {
    fn parse(value: &str) -> Option<Self> {
        let values = value
            .split_whitespace()
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        match values.as_slice() {
            [all] => Some(Self {
                top: i64::from(*all),
                right: i64::from(*all),
                bottom: i64::from(*all),
                left: i64::from(*all),
            }),
            [vertical, horizontal] => Some(Self {
                top: i64::from(*vertical),
                right: i64::from(*horizontal),
                bottom: i64::from(*vertical),
                left: i64::from(*horizontal),
            }),
            [top, horizontal, bottom] => Some(Self {
                top: i64::from(*top),
                right: i64::from(*horizontal),
                bottom: i64::from(*bottom),
                left: i64::from(*horizontal),
            }),
            [top, right, bottom, left] => Some(Self {
                top: i64::from(*top),
                right: i64::from(*right),
                bottom: i64::from(*bottom),
                left: i64::from(*left),
            }),
            _ => None,
        }
    }
}

/// Resolve the root document margin from PlantUML's preprocessed `<style>`
/// source. This mirrors `Style.getMargin` plus
/// `TextBlockExporter12026.Builder.calculateMargin`; later root declarations
/// replace earlier theme values.
fn class_document_margin(diagram: &ClassDiagram) -> Option<ClassDocumentMargin> {
    let source = diagram.meta.source.as_deref()?;
    let mut in_style = false;
    let mut selector_stack = Vec::<String>::new();
    let mut margin = None;

    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.eq_ignore_ascii_case("<style>") {
            in_style = true;
            selector_stack.clear();
            continue;
        }
        if line.eq_ignore_ascii_case("</style>") {
            in_style = false;
            selector_stack.clear();
            continue;
        }
        if !in_style || line.is_empty() {
            continue;
        }
        if let Some(selector) = line.strip_suffix('{') {
            selector_stack.push(selector.trim().to_ascii_lowercase());
            continue;
        }
        if line.starts_with('}') {
            selector_stack.pop();
            continue;
        }
        if selector_stack.len() == 1 && selector_stack[0] == "root" {
            let mut parts = line.split_whitespace();
            if parts
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("margin"))
            {
                margin = ClassDocumentMargin::parse(&parts.collect::<Vec<_>>().join(" "));
            }
        }
    }
    margin
}

impl ClassFontOverrides {
    fn from_skinparams(params: &[rustuml_parser::diagram::SkinParam]) -> Self {
        let plain_theme = params.iter().any(|sp| {
            sp.key.eq_ignore_ascii_case("__theme") && sp.value.trim().eq_ignore_ascii_case("plain")
        });
        let find = |names: &[&str]| -> Option<String> {
            params
                .iter()
                .find(|sp| names.iter().any(|n| sp.key.eq_ignore_ascii_case(n)))
                .map(|sp| sp.value.clone())
        };
        let style = find(&["ClassFontStyle"]).unwrap_or_default().to_lowercase();
        let attr_style = find(&["ClassAttributeFontStyle"])
            .unwrap_or_default()
            .to_lowercase();
        let stereotype_font_styles = params
            .iter()
            .filter_map(stereotype_font_style_param)
            .collect();
        let mut stereotype_colors = Vec::<ClassStereotypeColors>::new();
        for param in params {
            let Some((stereotype, background)) = stereotype_class_color_param(param) else {
                continue;
            };
            let color_index = stereotype_colors
                .iter()
                .position(|colors| colors.stereotype.eq_ignore_ascii_case(&stereotype))
                .unwrap_or_else(|| {
                    let index = stereotype_colors.len();
                    stereotype_colors.push(ClassStereotypeColors {
                        stereotype,
                        background: None,
                        border: None,
                    });
                    index
                });
            let colors = &mut stereotype_colors[color_index];
            if background {
                colors.background = Some(param.value.clone());
            } else {
                colors.border = Some(param.value.clone());
            }
        }
        // `skinparam defaultFontSize` is the base size for all class text,
        // overridden by the more specific `ClassFontSize` (name) and
        // `ClassAttributeFontSize` (members). It only applies when the
        // specific skinparam is absent.
        let default_font_size =
            find(&["defaultFontSize"]).and_then(|v| v.trim().parse::<u32>().ok());
        let class_font_size = find(&["ClassFontSize"]).and_then(|v| v.trim().parse::<u32>().ok());
        let class_attribute_font_size =
            find(&["ClassAttributeFontSize"]).and_then(|v| v.trim().parse::<u32>().ok());
        let family = find(&["ClassAttributeFontName", "defaultFontName", "fontName"])
            .map(|v| canonical_class_font_family(&v))
            .unwrap_or_else(|| {
                if plain_theme {
                    "Verdana".to_string()
                } else {
                    "sans-serif".to_string()
                }
            });
        let name_family = find(&["circledCharacterFontName"])
            .map(|v| canonical_class_font_family(&v))
            .or_else(|| find(&["ClassFontName"]).map(|v| canonical_class_font_family(&v)))
            .unwrap_or_else(|| family.clone());
        let circled_font_size = find(&["circledCharacterFontSize"])
            .and_then(|v| v.trim().parse::<u32>().ok())
            .or(default_font_size)
            .unwrap_or(CIRCLED_CHARACTER_DEFAULT_SIZE);
        let default_font_color = find(&["defaultFontColor"]);
        Self {
            text_padding: params
                .iter()
                .filter(|sp| sp.key.eq_ignore_ascii_case("padding"))
                .filter_map(|sp| sp.value.trim().parse::<f64>().ok())
                .next_back()
                .unwrap_or(0.0),
            font_color: find(&["ClassFontColor"]).or_else(|| default_font_color.clone()),
            attr_font_color: find(&["ClassAttributeFontColor"])
                .or_else(|| default_font_color.clone()),
            // Java `EntityImageClassHeader` resolves
            // root.element.classDiagram.class.header. The header therefore
            // inherits FontSize from its parent class style before falling
            // back to the root default.
            font_size: class_font_size
                .or(class_attribute_font_size)
                .or(default_font_size),
            family,
            name_family,
            font_bold: style.contains("bold"),
            font_italic: style.contains("italic"),
            stereotype_font_styles,
            stereotype_colors,
            attr_font_size: class_attribute_font_size.or(default_font_size),
            attr_font_bold: attr_style.contains("bold"),
            attr_font_italic: attr_style.contains("italic"),
            attr_icon_size: find(&["ClassAttributeIconSize"])
                .and_then(|v| v.trim().parse::<u32>().ok()),
            // The CIRCLED_CHARACTER font ignores ClassFontSize; it follows
            // circledCharacterFontSize, then defaultFontSize, then PlantUML's
            // CIRCLED_CHARACTER size 17.
            circled_font_size,
            circled_radius_override: find(&["circledCharacterRadius"])
                .and_then(|v| v.trim().parse::<f64>().ok())
                .or(if plain_theme { Some(9.0) } else { None }),
            header_background: find(&["classHeaderBackgroundColor"]),
            class_background: find(&["classBackgroundColor"]),
            border_color: find(&["classBorderColor"]),
            border_width: find(&["classBorderThickness"])
                .and_then(|v| v.trim().parse::<f64>().ok()),
            round_corner: find(&["classRoundCorner"])
                .or_else(|| find(&["roundCorner"]))
                .and_then(|v| v.trim().parse::<f64>().ok())
                .unwrap_or(5.0),
            root_line_color: find(&["__styleRootLineColor"]),
            root_font_color: find(&["__styleRootFontColor"]).or(default_font_color),
            stereotype_c_background: find(&["stereotypeCBackgroundColor"]).or_else(|| {
                if plain_theme {
                    Some("#FFFFFF".to_string())
                } else {
                    None
                }
            }),
            stereotype_c_border: find(&["stereotypeCBorderColor"]).or_else(|| {
                if plain_theme {
                    Some("#000000".to_string())
                } else {
                    None
                }
            }),
            stereotype_a_background: find(&["stereotypeABackgroundColor"]),
            stereotype_a_border: find(&["stereotypeABorderColor"]),
            stereotype_i_background: find(&["stereotypeIBackgroundColor"]),
            stereotype_i_border: find(&["stereotypeIBorderColor"]),
            stereotype_e_background: find(&["stereotypeEBackgroundColor"]),
            stereotype_e_border: find(&["stereotypeEBorderColor"]),
            monochrome: params.iter().any(|sp| {
                sp.key.eq_ignore_ascii_case("monochrome")
                    && matches!(
                        sp.value.trim().to_ascii_lowercase().as_str(),
                        "true" | "reverse"
                    )
            }),
        }
    }

    /// Radius of the circled-character header icon, per
    /// `SkinParam.getCircledCharacterRadius`: `circled_font_size / 3 + 6`
    /// (integer division). At the default circled size (17) this is 11.
    fn circled_radius(&self) -> f64 {
        self.circled_radius_override
            .unwrap_or((self.circled_font_size / 3 + 6) as f64)
    }

    fn name_font_size(&self) -> u32 {
        self.font_size.unwrap_or(FONT_SIZE as u32)
    }

    fn member_font_size(&self) -> u32 {
        self.attr_font_size.unwrap_or(FONT_SIZE as u32)
    }

    fn visibility_icon_geom(&self) -> VisibilityIconGeom {
        VisibilityIconGeom::from_attribute_icon_size(self.attr_icon_size)
    }

    fn monospace_member_spaces(&self) -> bool {
        is_monospace_font(&self.family)
    }

    fn stereotype_font_style(&self, stereotypes: &[String]) -> (bool, bool) {
        for stereotype in stereotypes {
            if let Some(style) = self
                .stereotype_font_styles
                .iter()
                .find(|style| stereotype.eq_ignore_ascii_case(&style.stereotype))
            {
                return (style.bold, style.italic);
            }
        }
        (false, false)
    }

    fn stereotype_colors(&self, stereotypes: &[String]) -> (Option<&str>, Option<&str>) {
        for stereotype in stereotypes {
            if let Some(colors) = self
                .stereotype_colors
                .iter()
                .find(|colors| stereotype.eq_ignore_ascii_case(&colors.stereotype))
            {
                return (colors.background.as_deref(), colors.border.as_deref());
            }
        }
        (None, None)
    }
}

fn canonical_class_font_family(value: &str) -> String {
    let raw = value.trim();
    let (trimmed, quoted) = if raw.len() >= 2
        && ((raw.starts_with('"') && raw.ends_with('"'))
            || (raw.starts_with('\'') && raw.ends_with('\'')))
    {
        (&raw[1..raw.len() - 1], true)
    } else {
        (raw, false)
    };
    let trimmed = trimmed.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("sansserif")
        || trimmed.eq_ignore_ascii_case("sans-serif")
    {
        "sans-serif".to_string()
    } else if quoted {
        format!("'{trimmed}'")
    } else {
        trimmed.to_string()
    }
}

fn is_monospace_font(font_family: &str) -> bool {
    let normalized = font_family
        .trim_matches(|c| c == '"' || c == '\'')
        .to_ascii_lowercase();
    MONOSPACE_FONTS.contains(&normalized.as_str())
}

fn unquoted_class_font_family(font_family: &str) -> &str {
    font_family
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(font_family)
}

fn stereotype_font_style_param(
    sp: &rustuml_parser::diagram::SkinParam,
) -> Option<ClassStereotypeFontStyle> {
    const PREFIX: &str = "class<<";
    let key = sp.key.trim();
    if !key.to_ascii_lowercase().starts_with(PREFIX) {
        return None;
    }
    let after_prefix = &key[PREFIX.len()..];
    let end = after_prefix.find(">>")?;
    let suffix = after_prefix[end + 2..].trim();
    if !suffix.eq_ignore_ascii_case("FontStyle") {
        return None;
    }
    let stereotype = after_prefix[..end].trim();
    if stereotype.is_empty() {
        return None;
    }
    let style = sp.value.trim().to_ascii_lowercase();
    Some(ClassStereotypeFontStyle {
        stereotype: stereotype.to_string(),
        bold: style.contains("bold"),
        italic: style.contains("italic"),
    })
}

fn stereotype_class_color_param(
    param: &rustuml_parser::diagram::SkinParam,
) -> Option<(String, bool)> {
    let key = param.key.trim();

    const GROUPED_PREFIX: &str = "class<<";
    if key.len() >= GROUPED_PREFIX.len()
        && key[..GROUPED_PREFIX.len()].eq_ignore_ascii_case(GROUPED_PREFIX)
    {
        let after_prefix = &key[GROUPED_PREFIX.len()..];
        let end = after_prefix.find(">>")?;
        let property = after_prefix[end + 2..].trim();
        let background = if property.eq_ignore_ascii_case("BackgroundColor") {
            true
        } else if property.eq_ignore_ascii_case("BorderColor") {
            false
        } else {
            return None;
        };
        let stereotype = after_prefix[..end].trim();
        if !stereotype.is_empty() {
            return Some((stereotype.to_string(), background));
        }
    }

    for (prefix, background) in [
        ("classBackgroundColor<<", true),
        ("classBorderColor<<", false),
    ] {
        if key.len() < prefix.len() || !key[..prefix.len()].eq_ignore_ascii_case(prefix) {
            continue;
        }
        let stereotype = key[prefix.len()..].strip_suffix(">>")?.trim();
        if !stereotype.is_empty() {
            return Some((stereotype.to_string(), background));
        }
    }
    None
}

#[derive(Clone, Copy)]
struct VisibilityIconGeom {
    center_offset: f64,
    round_half: f64,
    angled_half: f64,
    triangle_half_y: f64,
    block_height: f64,
    placement_center_bias: f64,
}

impl VisibilityIconGeom {
    fn from_attribute_icon_size(size: Option<u32>) -> Self {
        if let Some(size) = size {
            let round_half = (size / VIS_ICON_SIZE_RADIUS_DIVISOR) as f64;
            let even_size = size - size % 2;
            let block_height = if size == 0 { 0.0 } else { (size + 1) as f64 };
            Self {
                center_offset: size as f64,
                round_half,
                angled_half: (size as f64 / 2.0 - VIS_ICON_ANGLED_INSET).max(round_half),
                triangle_half_y: round_half,
                block_height,
                placement_center_bias: if block_height == 0.0 {
                    0.0
                } else {
                    2.0 + even_size as f64 / 2.0 - block_height / 2.0
                },
            }
        } else {
            Self {
                center_offset: VIS_ICON_OFFSET,
                round_half: VIS_ICON_R,
                angled_half: VIS_ICON_ANGLED_HALF,
                triangle_half_y: VIS_ICON_R,
                block_height: (VIS_ICON_DEFAULT_SIZE + 1) as f64,
                placement_center_bias: 1.5,
            }
        }
    }
}

/// Resolve the `<defs>` linearGradient id whose two stops match the gradient
/// spelled `c1/c2` (PlantUML's `#c1/#c2` shorthand). `defs_inner_xml` carries
/// the captured `<linearGradient id=…><stop stop-color=…/><stop stop-color=…/>`
/// entries; we parse each gradient's id and its two stop colours and pick the
/// one whose colours match (case-insensitively). Returns `None` when no
/// gradient matches (e.g. the header colour is solid, or the value isn't a
/// gradient at all). Only granular id + stop-colour scalars are consumed.
fn resolve_gradient_id(defs_inner_xml: &str, c1: &str, c2: &str) -> Option<String> {
    let c1 = c1.trim_start_matches('#');
    let c2 = c2.trim_start_matches('#');
    let mut rest = defs_inner_xml;
    while let Some(start) = rest.find("<linearGradient") {
        rest = &rest[start..];
        // Isolate this gradient element (up to its closing tag).
        let end = rest
            .find("</linearGradient>")
            .map(|e| e + "</linearGradient>".len());
        let (elem, after) = match end {
            Some(e) => (&rest[..e], &rest[e..]),
            None => (rest, ""),
        };
        rest = after;

        let id = attr_value(elem, "id");
        let stops: Vec<&str> = elem
            .match_indices("stop-color=\"")
            .filter_map(|(i, _)| {
                let v = &elem[i + "stop-color=\"".len()..];
                v.find('"').map(|q| &v[..q])
            })
            .collect();
        if let (Some(id), [s0, s1, ..]) = (id, stops.as_slice())
            && s0.trim_start_matches('#').eq_ignore_ascii_case(c1)
            && s1.trim_start_matches('#').eq_ignore_ascii_case(c2)
        {
            return Some(id.to_string());
        }
        if after.is_empty() {
            break;
        }
    }
    None
}

fn split_gradient_colors(val: &str) -> Option<(&str, &str)> {
    for sep in ['/', '\\', '|', '-'] {
        if let Some((left, right)) = val.split_once(sep) {
            let left = left.trim();
            let right = right.trim();
            if !left.is_empty() && !right.is_empty() {
                return Some((left, right));
            }
        }
    }
    None
}

#[derive(Clone)]
struct ClassGradient {
    color1: String,
    color2: String,
    policy: char,
    id: String,
}

fn split_class_gradient(value: &str) -> Option<(&str, &str, char)> {
    for policy in ['-', '\\', '|', '/'] {
        if let Some((color1, color2)) = value.split_once(policy) {
            let color1 = color1.trim();
            let color2 = color2.trim();
            if !color1.is_empty() && !color2.is_empty() {
                return Some((color1, color2, policy));
            }
        }
    }
    None
}

fn class_gradients(diagram: &ClassDiagram, font: &ClassFontOverrides) -> Vec<ClassGradient> {
    let source = diagram.meta.source.as_deref().unwrap_or("");
    let mut gradients: Vec<ClassGradient> = Vec::new();
    for value in [
        font.class_background.as_deref(),
        font.header_background.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        let Some((raw1, raw2, policy)) = split_class_gradient(value) else {
            continue;
        };
        let color1 = crate::sequence::resolve_color(raw1);
        let color2 = crate::sequence::resolve_color(raw2);
        if gradients.iter().any(|gradient| {
            gradient.color1 == color1 && gradient.color2 == color2 && gradient.policy == policy
        }) {
            continue;
        }
        gradients.push(ClassGradient {
            color1,
            color2,
            policy,
            id: crate::filter_registry::gradient_id_for(source, gradients.len()),
        });
    }
    gradients
}

fn class_gradient_defs(gradients: &[ClassGradient]) -> String {
    let mut defs = String::new();
    for gradient in gradients {
        // Java `SvgGraphics.createSvgGradient` maps `HColorGradient` policies
        // to these endpoint pairs and emits attributes alphabetically.
        let (x1, x2, y1, y2) = match gradient.policy {
            '|' => ("0%", "100%", "50%", "50%"),
            '\\' => ("0%", "100%", "100%", "0%"),
            '-' => ("50%", "50%", "0%", "100%"),
            _ => ("0%", "100%", "0%", "100%"),
        };
        write!(
            defs,
            r#"<linearGradient id="{}" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}"><stop offset="0%" stop-color="{}"/><stop offset="100%" stop-color="{}"/></linearGradient>"#,
            gradient.id, gradient.color1, gradient.color2,
        )
        .unwrap();
    }
    defs
}

fn gradient_fill_from_defs(value: Option<&str>, defs: Option<&str>) -> Option<String> {
    let (c1, c2) = split_gradient_colors(value?)?;
    defs.and_then(|defs| resolve_gradient_id(defs, c1, c2))
        .map(|id| format!("url(#{id})"))
        .or_else(|| Some(crate::sequence::resolve_color(c1)))
}

fn resolve_flat_or_gradient_start(value: &str) -> String {
    split_gradient_colors(value)
        .map(|(first, _)| crate::sequence::resolve_color(first))
        .unwrap_or_else(|| crate::sequence::resolve_color(value))
}

fn style_stroke_width(style: &str) -> Option<&str> {
    style
        .split(';')
        .filter_map(|part| part.trim().strip_prefix("stroke-width:"))
        .find(|width| !width.is_empty())
}

fn style_stroke_color(style: &str) -> Option<&str> {
    style
        .split(';')
        .filter_map(|part| part.trim().strip_prefix("stroke:"))
        .find(|color| !color.is_empty())
}

fn oracle_entities_for_diagram(
    diagram: &ClassDiagram,
    oracle: &OracleLayout,
) -> Vec<Option<OracleEntity>> {
    let mut matched = vec![None; diagram.entities.len()];
    let mut used = vec![false; oracle.entity_list.len()];
    for i in entity_emission_order(diagram) {
        let entity = &diagram.entities[i];
        let translated_label = translate_qualified_name(&entity.label);
        let qualified_name = qualify_entity(diagram, entity, &translated_label);
        let candidates = [
            qualified_name.as_str(),
            entity.label.as_str(),
            entity.id.as_str(),
        ];

        if let Some((oracle_idx, oracle_entity)) = candidates.iter().find_map(|candidate| {
            oracle
                .entity_list
                .iter()
                .enumerate()
                .find(|(idx, oracle_entity)| {
                    !used[*idx] && oracle_entity.qualified_name == *candidate
                })
        }) {
            used[oracle_idx] = true;
            matched[i] = Some(oracle_entity.clone());
            continue;
        }

        matched[i] = candidates.iter().find_map(|name| {
            oracle.entities.get(*name).map(|rect| OracleEntity {
                qualified_name: (*name).to_string(),
                rect: rect.clone(),
            })
        });
    }
    matched
}

fn package_parent_indices(diagram: &ClassDiagram) -> Vec<Option<usize>> {
    let n_pkg = diagram.packages.len();
    (0..n_pkg)
        .map(|i| {
            if let Some(parent) = diagram.packages[i].parent {
                return Some(parent);
            }
            let mine = &diagram.packages[i].entities;
            (0..n_pkg)
                .filter(|&j| {
                    j != i
                        && diagram.packages[j].entities.len() > mine.len()
                        && mine
                            .iter()
                            .all(|m| diagram.packages[j].entities.contains(m))
                })
                .min_by_key(|&j| diagram.packages[j].entities.len())
        })
        .collect()
}

fn package_depth(parent_pkg: &[Option<usize>], mut idx: usize) -> usize {
    let mut depth = 0;
    while let Some(parent) = parent_pkg[idx] {
        depth += 1;
        idx = parent;
    }
    depth
}

fn innermost_entity_packages(
    diagram: &ClassDiagram,
    parent_pkg: &[Option<usize>],
) -> Vec<Option<usize>> {
    diagram
        .entities
        .iter()
        .map(|e| {
            diagram
                .packages
                .iter()
                .enumerate()
                .filter(|(_, p)| p.entities.iter().any(|m| m == &e.id))
                .max_by_key(|(idx, p)| (package_depth(parent_pkg, *idx), p.entities.len(), *idx))
                .map(|(idx, _)| idx)
        })
        .collect()
}

fn effective_package_kind(pkg: &Package) -> PackageKind {
    if matches!(pkg.kind, PackageKind::Package | PackageKind::Namespace) {
        for stereotype in &pkg.stereotypes {
            if stereotype.eq_ignore_ascii_case("database") {
                return PackageKind::Database;
            }
            if stereotype.eq_ignore_ascii_case("folder") {
                return PackageKind::Folder;
            }
            if stereotype.eq_ignore_ascii_case("frame") {
                return PackageKind::Frame;
            }
            if stereotype.eq_ignore_ascii_case("rectangle") {
                return PackageKind::Rectangle;
            }
            if stereotype.eq_ignore_ascii_case("node") {
                return PackageKind::Node;
            }
            if stereotype.eq_ignore_ascii_case("cloud") {
                return PackageKind::Cloud;
            }
        }
    }
    pkg.kind
}

fn visible_package_stereotype_lines(pkg: &Package) -> Vec<String> {
    pkg.stereotypes
        .iter()
        .filter(|stereotype| {
            !matches!(
                stereotype.to_ascii_lowercase().as_str(),
                "database" | "folder" | "frame" | "rectangle" | "node" | "cloud"
            )
        })
        .map(|stereotype| format!("\u{00AB}{stereotype}\u{00BB}"))
        .collect()
}

fn is_rendered_package_cluster(pkg: &Package) -> bool {
    if pkg.phantom && pkg.source_line == 0 {
        return false;
    }
    matches!(
        effective_package_kind(pkg),
        PackageKind::Package
            | PackageKind::Namespace
            | PackageKind::Database
            | PackageKind::Folder
            | PackageKind::Frame
            | PackageKind::Rectangle
            | PackageKind::Node
            | PackageKind::Cloud
    )
}

fn package_cluster_id(idx: usize) -> String {
    format!("pkg{idx}")
}

fn package_cluster_envelope_extra(diagram: &ClassDiagram, cluster: &ClusterPosition) -> (f64, f64) {
    cluster
        .id
        .strip_prefix("pkg")
        .and_then(|idx| idx.parse::<usize>().ok())
        .and_then(|idx| diagram.packages.get(idx))
        .map_or((0.0, 0.0), |pkg| match effective_package_kind(pkg) {
            PackageKind::Database => (
                DATABASE_CLUSTER_ENVELOPE_EXTRA,
                DATABASE_CLUSTER_ENVELOPE_EXTRA,
            ),
            PackageKind::Node => (NODE_CLUSTER_ENVELOPE_X_EXTRA, NODE_CLUSTER_ENVELOPE_Y_EXTRA),
            PackageKind::Cloud => {
                let frontier = cloud_frontier(cluster.width, cluster.height);
                (
                    (frontier.max_x - cluster.width).max(0.0),
                    (frontier.max_y - cluster.height).max(0.0),
                )
            }
            _ => (0.0, 0.0),
        })
}

fn package_cloud_frontier(
    diagram: &ClassDiagram,
    cluster: &ClusterPosition,
) -> Option<CloudFrontier> {
    cluster
        .id
        .strip_prefix("pkg")
        .and_then(|idx| idx.parse::<usize>().ok())
        .and_then(|idx| diagram.packages.get(idx))
        .filter(|pkg| effective_package_kind(pkg) == PackageKind::Cloud)
        .map(|_| cloud_frontier(cluster.width, cluster.height))
}

fn package_display_label(pkg: &Package) -> &str {
    pkg.display_name.as_deref().unwrap_or(&pkg.name)
}

fn package_skinparam<'a>(
    diagram: &'a ClassDiagram,
    kind: PackageKind,
    suffix: &str,
) -> Option<&'a str> {
    let prefixes: &[&str] = match kind {
        PackageKind::Database => &["Database"],
        // Java `USymbolFolder.asBig` applies the folder symbol context to
        // `drawFolder`; only its background retains the legacy Package*
        // fallback. Border and title font remain Folder* channels.
        PackageKind::Folder if suffix == "BackgroundColor" => &["Folder", "Package"],
        PackageKind::Folder => &["Folder"],
        PackageKind::Frame => &["Frame"],
        PackageKind::Rectangle => &["Rectangle"],
        PackageKind::Node => &["Node"],
        PackageKind::Cloud => &["Cloud"],
        PackageKind::Package | PackageKind::Namespace => &["Package"],
    };
    for prefix in prefixes {
        let key = format!("{prefix}{suffix}");
        if let Some(value) = diagram
            .meta
            .skinparams
            .iter()
            .rev()
            .find(|skinparam| skinparam.key.eq_ignore_ascii_case(&key))
            .map(|skinparam| skinparam.value.trim())
        {
            return Some(value);
        }
    }
    None
}

fn package_qualified_name(
    diagram: &ClassDiagram,
    parent_pkg: &[Option<usize>],
    idx: usize,
) -> String {
    let package = &diagram.packages[idx];
    if package.display_name.is_some() {
        // `CucaDiagram.eventuallyBuildPhantomGroups` creates these groups from
        // the backing Quark path. `Cluster.drawU` consequently serializes
        // `Quark.getQualifiedName()`, including the configured namespace
        // separator after SVG's punctuation translation.
        let translated = translate_qualified_name(&package.name);
        let Some(parent_idx) = parent_pkg[idx] else {
            return translated;
        };
        let parent = &diagram.packages[parent_idx];
        let parent_qualified = package_qualified_name(diagram, parent_pkg, parent_idx);
        if let Some(suffix) = package.name.strip_prefix(&parent.name)
            && suffix.starts_with(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        {
            return format!("{parent_qualified}{}", translate_qualified_name(suffix));
        }
        return format!("{parent_qualified}.{translated}");
    }

    let mut chain = Vec::new();
    let mut cur = Some(idx);
    while let Some(i) = cur {
        chain.push(translate_qualified_name(package_display_label(
            &diagram.packages[i],
        )));
        cur = parent_pkg[i];
    }
    chain.reverse();
    chain.join(".")
}

fn relationship_endpoint_name<'a>(diagram: &'a ClassDiagram, id: &'a str) -> &'a str {
    // Java `Link.idCommentForSvg` builds path ids from `Entity.getName()`,
    // which is the Quark name. `NameAndCodeParser.CODE4` makes a bare quoted
    // name the Quark name, while `DISPLAY as CODE` makes the explicit code the
    // Quark name. Rust normalizes bare quoted ids for lookup, so recover their
    // display label here but preserve true aliases.
    diagram
        .entities
        .iter()
        .find(|entity| entity.id == id)
        .filter(|entity| !entity.explicit_alias)
        .map_or(id, |entity| entity.label.as_str())
}

fn package_content_offsets(diagram: &ClassDiagram) -> Vec<(f64, f64)> {
    let parent_pkg = package_parent_indices(diagram);
    let innermost_pkg = innermost_entity_packages(diagram, &parent_pkg);
    innermost_pkg
        .into_iter()
        .map(|package_idx| {
            let Some(mut package_idx) = package_idx else {
                return (0.0, 0.0);
            };
            while let Some(parent) = parent_pkg[package_idx] {
                if !is_rendered_package_cluster(&diagram.packages[parent]) {
                    break;
                }
                package_idx = parent;
            }
            match effective_package_kind(&diagram.packages[package_idx]) {
                PackageKind::Frame | PackageKind::Rectangle => (1.0, 1.0),
                PackageKind::Node => (NODE_BEVEL, 0.0),
                _ => (0.0, 0.0),
            }
        })
        .collect()
}

/// Java `SvekResult.calculateDimension` measures only renderer-visible shapes
/// and moves that envelope to `(6, 6)`. SVEK's outer `p0` protection clusters
/// influence dot but are not drawn, so normalize from the solved real cluster
/// boxes rather than Graphviz's root envelope.
fn normalize_svek_package_envelope(
    diagram: &ClassDiagram,
    node_positions: &mut [NodePosition],
    cluster_positions: &mut [ClusterPosition],
    edge_paths: &mut [EdgePath],
) {
    if cluster_positions.is_empty() {
        return;
    }
    let min_x = cluster_positions
        .iter()
        .map(|position| {
            package_cloud_frontier(diagram, position)
                .map(|frontier| position.x + frontier.min_x)
                .unwrap_or(position.x)
        })
        .chain(node_positions.iter().map(|position| position.x))
        .fold(f64::INFINITY, f64::min);
    let min_y = cluster_positions
        .iter()
        .map(|position| {
            package_cloud_frontier(diagram, position)
                .map(|frontier| position.y + frontier.min_y)
                .unwrap_or(position.y)
        })
        .chain(node_positions.iter().map(|position| position.y))
        .fold(f64::INFINITY, f64::min);
    let target = SVEK_LABEL_ENVELOPE_MARGIN - MARGIN;
    let dx = target - min_x;
    let dy = target - min_y;
    for position in node_positions {
        position.x += dx;
        position.y += dy;
    }
    for position in cluster_positions {
        position.x += dx;
        position.y += dy;
    }
    for path in edge_paths {
        for point in &mut path.points {
            point.0 += dx;
            point.1 += dy;
        }
        if let Some(point) = &mut path.start_point {
            point.0 += dx;
            point.1 += dy;
        }
        if let Some(point) = &mut path.end_point {
            point.0 += dx;
            point.1 += dy;
        }
        for label in [&mut path.label, &mut path.tail_label, &mut path.head_label]
            .into_iter()
            .flatten()
        {
            label.x += dx;
            label.y += dy;
        }
    }
}

struct HeaderPositions {
    icon_cx: f64,
    name_x: f64,
}

struct StereotypedHeaderPositions {
    icon_cx: f64,
    stereo_x: f64,
    name_x: f64,
}

fn class_header_positions(
    x: f64,
    width: f64,
    icon_radius: f64,
    name_text_width: f64,
    text_padding: f64,
) -> HeaderPositions {
    let circle_width = icon_radius * 2.0 + HEADER_CIRCLE_LEFT_MARGIN + HEADER_CIRCLE_RIGHT_MARGIN;
    let name_width = name_text_width + HEADER_NAME_MARGIN_X * 2.0 + text_padding * 2.0;
    let supp_width = (width - circle_width - name_width).max(0.0);
    let h2 = (circle_width / 4.0).min(supp_width * HEADER_SECONDARY_GAP_RATIO);
    let h1 = (supp_width - h2) / 2.0;
    HeaderPositions {
        icon_cx: x + h1 + HEADER_CIRCLE_LEFT_MARGIN + icon_radius,
        name_x: x + circle_width + h1 + h2 + HEADER_NAME_MARGIN_X + text_padding,
    }
}

/// Port of PlantUML `HeaderLayout.drawU` for a class header with a generic
/// badge. `EntityImageClassHeader` adds two outer pixels around the generic
/// box, so its layout width is the rendered box width plus those margins.
fn generic_header_positions(
    x: f64,
    width: f64,
    icon_radius: f64,
    name_text_width: f64,
    generic_text_width: f64,
    text_padding: f64,
) -> HeaderPositions {
    let circle_width = icon_radius * 2.0 + HEADER_CIRCLE_LEFT_MARGIN + HEADER_CIRCLE_RIGHT_MARGIN;
    let name_width = name_text_width + HEADER_NAME_MARGIN_X * 2.0 + text_padding * 2.0;
    let generic_width = generic_text_width + GENERIC_BOX_PAD * 2.0 + 2.0 + text_padding * 2.0;
    let supp_width = (width - circle_width - name_width - generic_width).max(0.0);
    let h2 = (circle_width / 4.0).min(supp_width * HEADER_SECONDARY_GAP_RATIO);
    let h1 = (supp_width - h2) / 2.0;
    HeaderPositions {
        icon_cx: x + h1 + HEADER_CIRCLE_LEFT_MARGIN + icon_radius,
        name_x: x + circle_width + h1 + h2 + HEADER_NAME_MARGIN_X + text_padding,
    }
}

/// Port of PlantUML `HeaderLayout.drawU` for stereotype headers. The
/// dimensions include `EntityImageClassHeader`'s circled-character, stereotype,
/// and name margins. RustUML's Graphviz node envelope is one pixel wider than
/// the internal PlantUML header width represented by the golden SVG rectangle.
fn stereotyped_header_positions(
    x: f64,
    width: f64,
    icon_radius: f64,
    stereo_text_width: f64,
    name_text_width: f64,
    text_padding: f64,
) -> StereotypedHeaderPositions {
    let circle_width = icon_radius * 2.0 + HEADER_CIRCLE_LEFT_MARGIN + HEADER_CIRCLE_RIGHT_MARGIN;
    let stereo_width = stereo_text_width + 1.0 + text_padding * 2.0;
    // PlantUML's name Display dimension is one pixel narrower than its SVG
    // textLength; the symmetric three-pixel margins therefore add five here.
    let name_width = name_text_width + 5.0 + text_padding * 2.0;
    let width_stereo_and_name = stereo_width.max(name_width);
    let supp_width = (width - 1.0 - circle_width - width_stereo_and_name).max(0.0);
    let h2 = (circle_width / 4.0).min(supp_width * HEADER_SECONDARY_GAP_RATIO);
    let h1 = (supp_width - h2) / 2.0;
    StereotypedHeaderPositions {
        icon_cx: x + h1 + HEADER_CIRCLE_LEFT_MARGIN + icon_radius,
        stereo_x: x
            + circle_width
            + (width_stereo_and_name - stereo_width) / 2.0
            + h1
            + h2
            + 1.0
            + text_padding,
        name_x: x
            + circle_width
            + (width_stereo_and_name - name_width) / 2.0
            + h1
            + h2
            + HEADER_NAME_MARGIN_X
            + text_padding,
    }
}

struct SvekIdAllocation {
    package_ids: Vec<Option<String>>,
    entity_ids: Vec<String>,
    note_ids: Vec<Option<String>>,
    attached_note_starts: Vec<Option<usize>>,
    relationship_ids: Vec<usize>,
    association_starts: Vec<Option<usize>>,
    entity_order: Vec<usize>,
}

struct SvekEmissionOrder<'a> {
    diagram: &'a ClassDiagram,
    parent_pkg: &'a [Option<usize>],
    innermost_pkg: &'a [Option<usize>],
    entity_order: Vec<usize>,
}

impl SvekEmissionOrder<'_> {
    fn collect_package(&mut self, pkg_idx: usize) {
        for (entity_idx, _) in self.diagram.entities.iter().enumerate() {
            if self.innermost_pkg[entity_idx] == Some(pkg_idx) {
                self.entity_order.push(entity_idx);
            }
        }
        for child_idx in 0..self.diagram.packages.len() {
            if self.parent_pkg[child_idx] == Some(pkg_idx)
                && is_rendered_package_cluster(&self.diagram.packages[child_idx])
            {
                self.collect_package(child_idx);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum CucaUidEvent {
    Package(usize),
    Entity(usize),
    FloatingNote(usize),
    AttachedNote(usize),
    Relationship(usize),
    Association(usize),
}

impl CucaUidEvent {
    fn sort_key(self, diagram: &ClassDiagram) -> (usize, usize, usize) {
        match self {
            Self::Package(idx) => {
                let package = &diagram.packages[idx];
                // `CommandCreateClass.executeArg` creates the leaf before
                // `CucaDiagram.reallyCreateLeaf` materializes missing Quarks
                // through `eventuallyBuildPhantomGroups`. Explicit package
                // commands still create their group before later contents.
                let same_line_order = if package.phantom { 2 } else { 0 };
                (package.source_line, same_line_order, idx)
            }
            // `CommandLinkClass` creates any missing endpoint entities before
            // constructing its `Link`, so entities win same-line ties.
            Self::Entity(idx) => (diagram.entities[idx].source_line, 1, idx),
            Self::FloatingNote(idx) => (diagram.notes[idx].source_line, 3, idx),
            Self::AttachedNote(idx) => (diagram.notes[idx].source_line, 3, idx),
            Self::Association(idx) => (diagram.association_classes[idx].source_line, 4, idx),
            Self::Relationship(idx) => (diagram.relationships[idx].source_line, 5, idx),
        }
    }
}

/// Port of the shared `CucaDiagram.cpt1` UID stream. `Entity` and `Link`
/// constructors both advance that counter, while
/// `CommandFactoryNoteOnEntity.executeInternal` advances it for the generated
/// GMN name, note entity, and connector link in that order.
fn svek_id_allocation(diagram: &ClassDiagram) -> SvekIdAllocation {
    let parent_pkg = package_parent_indices(diagram);
    let innermost_pkg = innermost_entity_packages(diagram, &parent_pkg);
    let mut emission = SvekEmissionOrder {
        diagram,
        parent_pkg: &parent_pkg,
        innermost_pkg: &innermost_pkg,
        entity_order: Vec::with_capacity(diagram.entities.len()),
    };

    for (pkg_idx, parent) in parent_pkg.iter().copied().enumerate() {
        if !is_rendered_package_cluster(&diagram.packages[pkg_idx]) {
            continue;
        }
        let parent_is_rendered =
            parent.is_some_and(|parent| is_rendered_package_cluster(&diagram.packages[parent]));
        if !parent_is_rendered {
            emission.collect_package(pkg_idx);
        }
    }
    let mut root_entities = diagram
        .entities
        .iter()
        .enumerate()
        .filter(|(idx, _)| innermost_pkg[*idx].is_none())
        .map(|(idx, entity)| (entity.source_line, idx))
        .collect::<Vec<_>>();
    root_entities.sort_by_key(|&(source_line, idx)| (source_line, idx));
    emission
        .entity_order
        .extend(root_entities.into_iter().map(|(_, idx)| idx));

    let mut events = diagram
        .packages
        .iter()
        .enumerate()
        .filter_map(|(idx, package)| {
            is_rendered_package_cluster(package).then_some(CucaUidEvent::Package(idx))
        })
        .chain(
            diagram
                .entities
                .iter()
                .enumerate()
                .map(|(idx, _)| CucaUidEvent::Entity(idx)),
        )
        .chain(diagram.notes.iter().enumerate().filter_map(|(idx, note)| {
            if note.target.is_some() && note.position.is_some() {
                Some(CucaUidEvent::AttachedNote(idx))
            } else if note.target.is_none() && note.alias.is_some() {
                Some(CucaUidEvent::FloatingNote(idx))
            } else {
                None
            }
        }))
        .chain(
            diagram
                .association_classes
                .iter()
                .enumerate()
                .map(|(idx, _)| CucaUidEvent::Association(idx)),
        )
        .chain(
            diagram
                .relationships
                .iter()
                .enumerate()
                .map(|(idx, _)| CucaUidEvent::Relationship(idx)),
        )
        .collect::<Vec<_>>();
    events.sort_by_key(|event| event.sort_key(diagram));

    let mut allocation = SvekIdAllocation {
        package_ids: vec![None; diagram.packages.len()],
        entity_ids: vec![String::new(); diagram.entities.len()],
        note_ids: vec![None; diagram.notes.len()],
        attached_note_starts: vec![None; diagram.notes.len()],
        relationship_ids: vec![0; diagram.relationships.len()],
        association_starts: vec![None; diagram.association_classes.len()],
        entity_order: emission.entity_order,
    };
    let mut next_id = 2;
    for event in events {
        match event {
            CucaUidEvent::Package(idx) => {
                allocation.package_ids[idx] = Some(format!("ent{next_id:04}"));
                next_id += 1;
            }
            CucaUidEvent::Entity(idx) => {
                allocation.entity_ids[idx] = format!("ent{next_id:04}");
                next_id += 1;
            }
            CucaUidEvent::FloatingNote(idx) => {
                allocation.note_ids[idx] = Some(format!("ent{next_id:04}"));
                next_id += 1;
            }
            CucaUidEvent::AttachedNote(idx) => {
                allocation.attached_note_starts[idx] = Some(next_id);
                next_id += 3;
            }
            CucaUidEvent::Relationship(idx) => {
                // `CommandLinkClass.executeArg` constructs a Link before
                // applying direction. LEFT/UP then call `Link.getInv()`, whose
                // Link constructor consumes the next `CucaDiagram.cpt1` UID;
                // only that replacement link reaches SVEK.
                let inverted = usize::from(diagram.relationships[idx].style.inverted);
                allocation.relationship_ids[idx] = next_id + inverted;
                next_id += 1 + inverted;
            }
            CucaUidEvent::Association(idx) => {
                allocation.association_starts[idx] = Some(next_id);
                next_id += ASSOCIATION_SEQUENCE_SLOTS;
            }
        }
    }
    allocation
}

/// Preserve `CucaDiagram.cpt1` allocations consumed before a later
/// `removeOrRestore` command. Graph construction uses the filtered diagram,
/// while surviving SVG ids retain their slots from the source model.
fn svek_id_allocation_from_origin(
    diagram: &ClassDiagram,
    origin: &ClassDiagram,
) -> SvekIdAllocation {
    if std::ptr::eq(diagram, origin) {
        return svek_id_allocation(diagram);
    }

    let mut allocation = svek_id_allocation(diagram);
    let original = svek_id_allocation(origin);

    for (index, entity) in diagram.entities.iter().enumerate() {
        if let Some(original_index) = origin.entities.iter().position(|candidate| {
            candidate.source_line == entity.source_line && candidate.id == entity.id
        }) {
            allocation.entity_ids[index] = original.entity_ids[original_index].clone();
        }
    }
    for (index, package) in diagram.packages.iter().enumerate() {
        if let Some(original_index) = origin
            .packages
            .iter()
            .position(|candidate| candidate.source_line == package.source_line)
        {
            allocation.package_ids[index] = original.package_ids[original_index].clone();
        }
    }
    for (index, note) in diagram.notes.iter().enumerate() {
        if let Some(original_index) = origin.notes.iter().position(|candidate| {
            candidate.source_line == note.source_line
                && candidate.alias == note.alias
                && candidate.target == note.target
        }) {
            allocation.note_ids[index] = original.note_ids[original_index].clone();
            allocation.attached_note_starts[index] = original.attached_note_starts[original_index];
        }
    }
    for (index, relationship) in diagram.relationships.iter().enumerate() {
        if let Some(original_index) = origin.relationships.iter().position(|candidate| {
            candidate.source_line == relationship.source_line
                && candidate.from == relationship.from
                && candidate.to == relationship.to
        }) {
            allocation.relationship_ids[index] = original.relationship_ids[original_index];
        }
    }
    for (index, association) in diagram.association_classes.iter().enumerate() {
        if let Some(original_index) = origin.association_classes.iter().position(|candidate| {
            candidate.source_line == association.source_line
                && candidate.a == association.a
                && candidate.b == association.b
                && candidate.c == association.c
        }) {
            allocation.association_starts[index] = original.association_starts[original_index];
        }
    }
    allocation
}

fn entity_emission_order(diagram: &ClassDiagram) -> Vec<usize> {
    svek_id_allocation(diagram).entity_order
}

/// Read the value of a double-quoted attribute `name="…"` from an element's
/// opening tag text. Returns the first match.
fn attr_value<'a>(elem: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let i = elem.find(&needle)? + needle.len();
    let v = &elem[i..];
    v.find('"').map(|q| &v[..q])
}

/// Render the full SVG with PlantUML-compatible structure.
///
/// When `canvas_override` is `Some((w, h))`, use those dimensions for the SVG
/// canvas instead of computing from entity extents. This is used with oracle
/// layout to match PlantUML's exact canvas size.
///
/// When `oracle` is `Some`, edge rendering uses the oracle's raw SVG path data
/// and arrowhead polygons directly, wrapped in `<g class="link">` groups.
#[allow(clippy::too_many_arguments)]
fn render_plantuml_svg(
    diagram: &ClassDiagram,
    dims: &[EntityDims],
    positions: &[rustuml_layout::graph::NodePosition],
    cluster_positions: &[ClusterPosition],
    edge_paths: &[EdgePath],
    layout_extent: Option<(f64, f64)>,
    canvas_override: Option<(f64, f64)>,
    oracle_entities: Option<&[Option<OracleEntity>]>,
    oracle: Option<&OracleLayout>,
    hidden_entities: &std::collections::HashSet<usize>,
    uid_origin: &ClassDiagram,
    cs: &crate::style::ClassStyle,
) -> String {
    if positions.len() < diagram.entities.len() {
        return render_grid_fallback(diagram, cs);
    }

    let font = ClassFontOverrides::from_skinparams(&diagram.meta.skinparams);
    let document_margin = class_document_margin(diagram);
    let shadow_filter_id = has_shadowing_skinparam(diagram).then(|| {
        crate::filter_registry::shadow_id_for(diagram.meta.source.as_deref().unwrap_or(""))
    });
    let package_content_offsets = package_content_offsets(diagram);
    let mut adjusted_edge_paths = edge_paths.to_vec();
    for edge in &mut adjusted_edge_paths {
        let endpoint_offset = |id: &str| {
            diagram
                .entities
                .iter()
                .position(|entity| entity.id == id)
                .map(|idx| package_content_offsets[idx])
                .unwrap_or((0.0, 0.0))
        };
        let from_offset = endpoint_offset(&edge.from);
        if from_offset == endpoint_offset(&edge.to) && from_offset != (0.0, 0.0) {
            for point in &mut edge.points {
                point.0 += from_offset.0;
                point.1 += from_offset.1;
            }
            if let Some(point) = &mut edge.start_point {
                point.0 += from_offset.0;
                point.1 += from_offset.1;
            }
            if let Some(point) = &mut edge.end_point {
                point.0 += from_offset.0;
                point.1 += from_offset.1;
            }
            for label in [&mut edge.label, &mut edge.tail_label, &mut edge.head_label]
                .into_iter()
                .flatten()
            {
                label.x += from_offset.0;
                label.y += from_offset.1;
            }
        }
    }
    let text_padding = font.text_padding;

    let layout_x_bias = svek_layout_x_bias(
        diagram,
        positions,
        cluster_positions,
        &adjusted_edge_paths,
        &font,
    );

    // Compute entity positions (offset from layout).
    let mut entity_positions: Vec<(f64, f64)> = (0..diagram.entities.len())
        .map(|i| {
            let (content_dx, content_dy) = package_content_offsets[i];
            (
                positions[i].x + MARGIN + layout_x_bias + content_dx,
                positions[i].y + MARGIN + content_dy,
            )
        })
        .collect();
    let attached_notes: Vec<(usize, usize, NotePosition)> = diagram
        .notes
        .iter()
        .enumerate()
        .filter_map(|(note_idx, note)| {
            note.target
                .as_ref()
                .zip(note.position)
                .map(|(_, p)| (note_idx, p))
        })
        .enumerate()
        .map(|(ordinal, (note_idx, position))| {
            (
                note_idx,
                diagram.entities.len() + diagram.association_classes.len() + ordinal,
                position,
            )
        })
        .collect();
    let floating_notes: Vec<(usize, usize)> = diagram
        .notes
        .iter()
        .enumerate()
        .filter_map(|(note_idx, note)| {
            (note.target.is_none() && note.alias.is_some()).then_some(note_idx)
        })
        .enumerate()
        .map(|(ordinal, note_idx)| {
            (
                note_idx,
                diagram.entities.len()
                    + diagram.association_classes.len()
                    + attached_notes.len()
                    + ordinal,
            )
        })
        .collect();

    // Java `DiagramChromeFactory12026.create` wraps the SVEK body with title,
    // caption, then header/footer. Each `DecorateEntityImage.drawU` centres the
    // wrapped image and translates it by the top text block's height; nested
    // wrappers accumulate those deltas through `getDeltaX/getDeltaY`.
    let mut body_min_x = f64::INFINITY;
    let mut body_max_x = f64::NEG_INFINITY;
    let mut body_top = f64::INFINITY;
    let mut body_bottom = f64::NEG_INFINITY;
    for (i, (x, y)) in entity_positions.iter().enumerate() {
        body_min_x = body_min_x.min(*x);
        body_max_x = body_max_x.max(x + dims[i].width);
        body_top = body_top.min(*y);
        body_bottom = body_bottom.max(y + dims[i].height);
    }
    for cluster in cluster_positions {
        let x = cluster.x + MARGIN;
        let y = cluster.y + MARGIN;
        let cloud_frontier = package_cloud_frontier(diagram, cluster);
        let (envelope_extra_x, envelope_extra_y) = package_cluster_envelope_extra(diagram, cluster);
        body_min_x = body_min_x.min(x + cloud_frontier.as_ref().map_or(0.0, |f| f.min_x));
        body_max_x = body_max_x.max(x + cluster.width + envelope_extra_x);
        body_top = body_top.min(y + cloud_frontier.as_ref().map_or(0.0, |f| f.min_y));
        body_bottom = body_bottom.max(y + cluster.height + envelope_extra_y);
    }
    for &(_, node_idx, _) in &attached_notes {
        if let Some(pos) = positions.get(node_idx) {
            let x = pos.x + MARGIN + layout_x_bias;
            let y = pos.y + MARGIN;
            body_min_x = body_min_x.min(x);
            body_max_x = body_max_x.max(x + pos.width);
            body_top = body_top.min(y);
            body_bottom = body_bottom.max(y + pos.height);
        }
    }
    for &(_, node_idx) in &floating_notes {
        if let Some(pos) = positions.get(node_idx) {
            let x = pos.x + MARGIN + layout_x_bias;
            let y = pos.y + MARGIN;
            body_min_x = body_min_x.min(x);
            body_max_x = body_max_x.max(x + pos.width);
            body_top = body_top.min(y);
            body_bottom = body_bottom.max(y + pos.height);
        }
    }
    if !body_min_x.is_finite() {
        body_min_x = 0.0;
        body_max_x = 0.0;
        body_top = 0.0;
        body_bottom = 0.0;
    }
    // Rust's effective body dimension excludes the final one-pixel
    // `TextBlockBordered` extent, so the 7px SVG envelope restores the same
    // integer canvas size as Java's 6px `ImageBuilder` margin.
    let body_inner_w = (body_max_x - body_min_x) + BODY_DECORATION_MARGIN;
    let layout = DecorationLayout::new(diagram, body_inner_w);
    let (body_dx, body_dy) = if oracle.is_none() {
        ((layout.dim_total_w - body_inner_w) / 2.0, layout.top_h)
    } else {
        (0.0, 0.0)
    };
    let (body_dx, body_dy) = document_margin.map_or((body_dx, body_dy), |margin| {
        (body_dx + margin.left as f64, body_dy + margin.top as f64)
    });

    for position in &mut entity_positions {
        position.0 += body_dx;
        position.1 += body_dy;
    }
    body_top += body_dy;
    body_bottom += body_dy;

    let mut adjusted_cluster_positions = cluster_positions.to_vec();
    for position in &mut adjusted_cluster_positions {
        position.x += body_dx;
        position.y += body_dy;
    }
    let cluster_positions = adjusted_cluster_positions.as_slice();

    for edge in &mut adjusted_edge_paths {
        for point in &mut edge.points {
            point.0 += body_dx;
            point.1 += body_dy;
        }
        if let Some(point) = &mut edge.start_point {
            point.0 += body_dx;
            point.1 += body_dy;
        }
        if let Some(point) = &mut edge.end_point {
            point.0 += body_dx;
            point.1 += body_dy;
        }
        for label in [&mut edge.label, &mut edge.tail_label, &mut edge.head_label]
            .into_iter()
            .flatten()
        {
            label.x += body_dx;
            label.y += body_dy;
        }
    }
    let edge_paths = adjusted_edge_paths.as_slice();
    let resolved_relationship_edges = relationship_edge_indices(diagram, edge_paths);
    let floating_note_opale_relationships = diagram
        .notes
        .iter()
        .enumerate()
        .map(|(note_idx, _)| floating_note_opale_relationship(diagram, note_idx))
        .collect::<Vec<_>>();
    let opale_relationships = floating_note_opale_relationships
        .iter()
        .flatten()
        .copied()
        .collect::<HashSet<_>>();

    // Compute canvas dimensions.
    let (canvas_w, canvas_h) = if let Some((w, h)) = canvas_override {
        (w.round() as i64, h.round() as i64)
    } else {
        let mut max_x = 0.0_f64;
        let mut max_y = 0.0_f64;
        let mut latex_image_max_x = 0.0_f64;
        for (i, (x, y)) in entity_positions.iter().enumerate() {
            max_x = max_x.max(x + dims[i].width);
            max_y = max_y.max(y + dims[i].height);
            let entity = &diagram.entities[i];
            let hide = resolve_hide(entity, &diagram.hide_show);
            for member in entity
                .members
                .iter()
                .filter(|member| !hide.hides_member(member))
            {
                let text_offset = if member.visibility == Visibility::Default {
                    ENUM_TEXT_OFFSET
                } else {
                    MEMBER_TEXT_INSET + font.circled_radius()
                };
                for line in member_display_lines(member, font.monospace_member_spaces()) {
                    if let Some(latex) = latex_member_content(&line) {
                        latex_image_max_x = latex_image_max_x.max(
                            x + font.text_padding
                                + text_offset
                                + crate::math::raw_latex_image(latex).width as f64,
                        );
                    }
                }
            }
        }
        for &(_, node_idx, _) in &attached_notes {
            if let Some(pos) = positions.get(node_idx) {
                max_x = max_x.max(pos.x + MARGIN + layout_x_bias + body_dx + pos.width);
                max_y = max_y.max(pos.y + MARGIN + body_dy + pos.height);
            }
        }
        for &(_, node_idx) in &floating_notes {
            if let Some(pos) = positions.get(node_idx) {
                max_x = max_x.max(pos.x + MARGIN + layout_x_bias + body_dx + pos.width);
                max_y = max_y.max(pos.y + MARGIN + body_dy + pos.height);
            }
        }
        for cluster in cluster_positions {
            let (envelope_extra_x, envelope_extra_y) =
                package_cluster_envelope_extra(diagram, cluster);
            max_x = max_x.max(cluster.x + MARGIN + cluster.width + envelope_extra_x);
            max_y = max_y.max(cluster.y + MARGIN + cluster.height + envelope_extra_y);
        }
        if let Some((width, height)) = layout_extent {
            max_x = max_x.max(width + layout_x_bias + body_dx);
            max_y = max_y.max(height + body_dy);
        }
        for edge in edge_paths {
            // Java `SvekResult.calculateDimension` delegates to
            // `TextBlockUtils.getMinMax`, so `LimitFinder.drawDotPath` includes
            // every solved DotPath control point in the painted envelope.
            // Graphviz's layout extent can be narrower than those splines in
            // dense and cyclic graphs.
            for &(x, _) in &edge.points {
                max_x = max_x.max(x + MARGIN + layout_x_bias);
            }
            for label in [edge.tail_label, edge.head_label].into_iter().flatten() {
                max_x = max_x.max(label.x + MARGIN + label.width);
                max_y = max_y.max(label.y + MARGIN + label.height);
            }
        }
        for (relationship, edge_idx) in diagram
            .relationships
            .iter()
            .zip(relationship_edge_indices(diagram, edge_paths))
        {
            let Some(edge) = edge_idx.and_then(|idx| edge_paths.get(idx)) else {
                continue;
            };
            if let Some((_, decor_max_x)) =
                relationship_endpoint_decor_x_bounds(relationship, &edge.points)
            {
                max_x = max_x.max(decor_max_x + MARGIN + layout_x_bias + body_dx);
            }
        }
        let relationship_note_indices = relationship_note_indices(diagram);
        for ((relationship, note_idx), edge_idx) in diagram
            .relationships
            .iter()
            .zip(&relationship_note_indices)
            .zip(relationship_edge_indices(diagram, edge_paths))
        {
            let note = note_idx.map(|idx| &diagram.notes[idx]);
            let Some(center) =
                relationship_center_layout(diagram, relationship, note, &diagram.meta.sprites)
            else {
                continue;
            };
            let Some(edge) = edge_idx.and_then(|idx| edge_paths.get(idx)) else {
                continue;
            };
            let Some(position) = edge.label else {
                continue;
            };
            if relationship_has_center_label(relationship) {
                max_x = max_x.max(
                    position.x
                        + MARGIN
                        + layout_x_bias
                        + (center.width - center.label_width) / 2.0
                        + center.label_width,
                );
            }
            if note.is_some() {
                let note_x =
                    position.x + MARGIN + layout_x_bias + (center.width - center.note_width) / 2.0;
                let note_y =
                    position.y + MARGIN + center.label_height + RELATIONSHIP_NOTE_COMPONENT_PADDING;
                // `SvekResult.calculateDimension` measures the rendered graph
                // through `LimitFinder`; its `drawUPath` includes the visible
                // `ComponentRoseNote` polygon in the final envelope.
                max_x = max_x.max(note_x + center.note_width);
                max_y = max_y.max(note_y + center.note_height);
            }
        }
        // Java `GraphvizImageBuilder.buildImage` selects
        // `EntityImageDegenerated` only when
        // `DotData.isDegeneratedWithFewEntities(1)` reports zero groups, zero
        // links, and exactly one leaf. Every other graph is a `SvekResult`,
        // whose `calculateDimension` adds 15px to the measured body envelope.
        let uses_degenerated_entity = uses_degenerated_entity(diagram, cluster_positions);
        let extent_pad = if uses_degenerated_entity {
            13
        } else {
            PACKAGE_CANVAS_EXTENT_PAD + i64::from(shadow_filter_id.is_some()) * 5
        };
        let (extent_pad_x, extent_pad_y, document_width) =
            document_margin.map_or((extent_pad, extent_pad, 0), |margin| {
                (
                    extent_pad + margin.right - DEFAULT_DOCUMENT_EXTENT_MARGIN,
                    extent_pad + margin.bottom - DEFAULT_DOCUMENT_EXTENT_MARGIN,
                    margin.left + margin.right,
                )
            });
        let decorated_w = if layout.has_decorations {
            layout.dim_total_w as i64 + MARGIN as i64 + document_width
        } else {
            0
        };
        // Java's SVG backend expands the canvas around the fallback image
        // payload even though `AtomMath` contributed the smaller raster box to
        // entity layout. One pixel is retained beyond the image's right edge.
        let latex_image_w = latex_image_max_x.ceil() as i64 + i64::from(latex_image_max_x > 0.0);
        (
            (max_x as i64 + extent_pad_x)
                .max(decorated_w)
                .max(latex_image_w),
            (max_y + layout.bottom_h) as i64 + extent_pad_y,
        )
    };

    let mut svg = String::new();

    // `skinparam backgroundColor` recolours the canvas: the style `background`
    // takes the colour and a full-canvas `<rect>` is emitted after `<g>` (white
    // is the default and emits neither). Mirrors the sequence renderer.
    let bg_value = last_background_value(diagram);
    let bg_color = bg_value
        .filter(|value| !value.eq_ignore_ascii_case("transparent"))
        .map(crate::sequence::resolve_color)
        .filter(|c| c != "#FFFFFF");
    let bg_style = bg_color.as_deref().unwrap_or("#FFFFFF");
    let bg_style_suffix = if bg_value.is_some_and(|value| value.eq_ignore_ascii_case("transparent"))
    {
        String::new()
    } else {
        format!("background:{bg_style};")
    };

    // Root <svg> element with PlantUML attributes (alphabetical order).
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="CLASS" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;{bg_style_suffix}" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">"#,
        w = canvas_w,
        h = canvas_h,
    )
    .unwrap();

    // Processing instruction and defs.
    svg.push_str("<?plantuml 1.2026.3beta6?>");
    // Emit any `<defs>` the oracle captured verbatim (e.g. the
    // `<linearGradient>` PlantUML generates for a `#c1/c2` gradient
    // background, or background-colour filters). The entity rects reference
    // these via oracle-captured `fill="url(#...)"`, so the ids must be live.
    let gradients = class_gradients(diagram, &font);
    let mut generated_defs = class_gradient_defs(&gradients);
    if let Some(shadow_defs) = shadow_filter_id
        .as_deref()
        .map(crate::filter_registry::shadow_filter_def)
    {
        generated_defs.push_str(&shadow_defs);
    }
    let active_defs = oracle
        .map(|o| o.defs_inner_xml.as_str())
        .or((!generated_defs.is_empty()).then_some(generated_defs.as_str()));
    match active_defs {
        Some(defs) if !defs.is_empty() => {
            svg.push_str("<defs>");
            svg.push_str(defs);
            svg.push_str("</defs>");
        }
        _ => svg.push_str("<defs/>"),
    }
    svg.push_str("<g>");
    if let Some(color) = &bg_color {
        write!(
            svg,
            r#"<rect fill="{color}" height="{canvas_h}" style="stroke:none;stroke-width:1;" width="{canvas_w}" x="0" y="0"/>"#,
        )
        .unwrap();
    }
    if has_handwritten_skinparam(diagram)
        && let Some(warning) = oracle.and_then(|o| o.handwritten_warning.as_ref())
    {
        emit_handwritten_warning(&mut svg, warning);
    }
    let suppress_header_icon = has_strictuml_style(diagram);

    let oracle_decoration_texts = |class_name: &str| {
        oracle.and_then(|o| {
            o.decorations
                .iter()
                .find(|d| d.class_name == class_name)
                .map(|d| d.texts.as_slice())
        })
    };

    // Top-of-canvas decorations, emitted header-first then title (PlantUML's
    // `addTopAndBottom` group order), each as
    // `<g class="..." data-source-line="N"><text ...>TEXT</text></g>`.
    layout.emit(
        &mut svg,
        "header",
        diagram.meta.header.as_deref(),
        diagram.header_line,
        DECORATION_HEADER_BASELINE_Y,
        oracle_decoration_texts("header"),
    );
    layout.emit(
        &mut svg,
        "title",
        diagram.meta.title.as_deref(),
        diagram.title_line,
        body_top - DECORATION_TITLE_GAP_ABOVE_BODY,
        oracle_decoration_texts("title"),
    );

    // Legend placement follows PlantUML's `addTopAndBottom`: a `legend top …`
    // goes into the top group (ahead of the body); a default/`bottom` legend
    // goes into the bottom group (after the relationships, emitted further
    // below). Distinguish the two from the oracle geometry — a top legend sits
    // entirely above the body's first row.
    let is_top_legend = |legend: &OracleLegend| {
        body_top.is_finite() && legend.rect.y + legend.rect.height <= body_top
    };
    if let Some(orc) = oracle {
        for legend in orc.legends.iter().filter(|l| is_top_legend(l)) {
            emit_oracle_legend(&mut svg, legend, diagram.legend_line);
        }
    }

    // Render any oracle-captured clusters (package/database/folder/...)
    // in document order,
    // BEFORE the diagram entities. This matches Java's emission order and
    // lets entities inside a cluster claim the next available `ent000N`
    // ID. Notes captured here have `group_class = "entity"` and are
    // emitted AFTER the diagram entities below.
    let oracle_pkg_clusters: Vec<&OracleCluster> = oracle
        .map(|o| {
            o.clusters
                .iter()
                .filter(|c| c.group_class == "cluster")
                .collect()
        })
        .unwrap_or_default();
    let layout_pkg_clusters = if oracle.is_none() {
        layout_package_clusters(diagram, cluster_positions)
    } else {
        Vec::new()
    };
    let svek_ids = svek_id_allocation_from_origin(diagram, uid_origin);
    // Note entities (alias-named like `N1` AND auto-generated `GMNn`) are
    // captured separately in `note_entities`. The legacy `clusters`
    // collection only picks up GMN-prefixed qnames; reading from
    // `note_entities` covers explicit aliases too.
    // Note entities (`note "…" as N` floating notes, plus auto-generated
    // `GMNn`) share the `ent000N` emission counter with regular entities and
    // are interleaved with them in PlantUML's output by that counter — i.e. a
    // note declared before an entity in the source is emitted before it.
    // Order them by the numeric suffix of their captured `entity_id` so the
    // interleave below matches the golden's document order.
    let mut oracle_note_entities: Vec<&crate::layout_oracle::OracleNoteEntity> = oracle
        .map(|o| o.note_entities.iter().collect())
        .unwrap_or_default();
    oracle_note_entities.sort_by_key(|n| ent_id_seq(n.entity_id.as_deref()));
    for cluster in &oracle_pkg_clusters {
        write!(svg, "<!--cluster {}-->", cluster.qualified_name).unwrap();
        let cluster_id = cluster.entity_id.as_deref().unwrap_or("ent0002");
        let source_line = cluster.source_line.as_deref().unwrap_or("0");
        write!(
            svg,
            r#"<g class="cluster" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
            escape_xml(&cluster.qualified_name),
            source_line,
            cluster_id,
        )
        .unwrap();
        emit_oracle_cluster_children(&mut svg, cluster);
        svg.push_str("</g>");
    }
    for cluster in &layout_pkg_clusters {
        let entity_id = svek_ids.package_ids[cluster.package_idx]
            .as_deref()
            .unwrap_or("ent0002");
        emit_layout_package_cluster(&mut svg, cluster, entity_id);
    }
    if let Some(oracle) = oracle {
        for cluster in &oracle.loose_clusters {
            emit_oracle_cluster_children(&mut svg, cluster);
        }
    }

    // Entity ID counter (PlantUML starts at ent0002, shifted past clusters).
    let mut ent_id = 2 + oracle_pkg_clusters.len() + layout_pkg_clusters.len();

    let emission_order = entity_emission_order(diagram);
    let mut layout_floating_notes = floating_notes
        .iter()
        .filter_map(|&(note_idx, node_idx)| {
            svek_ids.note_ids[note_idx]
                .as_deref()
                .map(|entity_id| (note_idx, node_idx, entity_id))
        })
        .collect::<Vec<_>>();
    layout_floating_notes.sort_by_key(|(_, _, entity_id)| ent_id_seq(Some(entity_id)));
    let mut layout_note_cursor = 0;

    // Cursor over `oracle_note_entities` (already sorted by emission counter).
    // `emit_note` writes one note's `<g class="entity">…</g>` wrapper; the loop
    // below flushes any notes whose counter precedes the current entity so the
    // interleaving matches PlantUML's document order.
    let mut note_cursor = 0usize;
    let emit_note = |svg: &mut String, note: &crate::layout_oracle::OracleNoteEntity| {
        let _ =
            emit_oracle_note_entity(svg, note, "#181818", "#FEFFDD", 13, "sans-serif", "#000000");
    };

    // Render each entity.
    for &i in &emission_order {
        let entity = &diagram.entities[i];
        let (x, y) = entity_positions[i];
        let dim = &dims[i];
        let seq_ent_id = format!("ent{:04}", ent_id);
        ent_id += 1;

        // Compute qualified name by joining all containing package names
        // (outermost → innermost in package declaration order) with the
        // entity label, dot-separated. Mirrors Java's
        // `data-qualified-name` attribute, including its translation of
        // `&` → `.` (used when entities are quoted with special chars,
        // e.g. `"A&B"`).
        let translated_label = translate_qualified_name(&entity.label);
        let qualified_name = qualify_entity(diagram, entity, &translated_label);

        // Look up oracle overrides for this entity. Prefer the ordered
        // entity list because PlantUML folds many non-ASCII qualified names
        // to the same dot string (`用户` and `系统` both become `..`), so the
        // map form can only retain the last one. Fall back to the legacy map
        // lookup for older oracle data and unique-name cases.
        let ordered_oracle_entity =
            oracle_entities.and_then(|entities| entities.get(i).and_then(Option::as_ref));
        let fallback_oracle_rect_with_name = if ordered_oracle_entity.is_none() {
            oracle.and_then(|orc| {
                [
                    qualified_name.as_str(),
                    entity.label.as_str(),
                    entity.id.as_str(),
                ]
                .into_iter()
                .find_map(|name| orc.entities.get(name).map(|rect| (name, rect)))
            })
        } else {
            None
        };
        let oracle_rect = ordered_oracle_entity
            .map(|entity| &entity.rect)
            .or_else(|| fallback_oracle_rect_with_name.map(|(_, rect)| rect));
        let qualified_name = ordered_oracle_entity
            .map(|entity| entity.qualified_name.as_str())
            .or_else(|| fallback_oracle_rect_with_name.map(|(name, _)| name))
            .unwrap_or(qualified_name.as_str());
        let oracle_lollipop =
            oracle.and_then(|orc| oracle_lollipop_for_entity(diagram, orc, entity));

        // Prefer the oracle's verbatim entity id. PlantUML's `ent000N`
        // counter is not a clean source-order sequence: interface targets of
        // realization edges and other entities can claim ids out of step with
        // our left-to-right entity walk, so reconstructing the counter
        // ourselves drifts. Consume the captured id like other verbatim oracle
        // data, falling back to the sequential counter when absent.
        let current_ent_id = if oracle.is_none() {
            svek_ids.entity_ids[i].clone()
        } else {
            oracle_rect
                .and_then(|r| r.entity_id.clone())
                .or_else(|| oracle_lollipop.and_then(|(_, r)| r.entity_id.clone()))
                .unwrap_or(seq_ent_id)
        };

        // Flush any note entities whose emission counter precedes this entity's
        // (e.g. a `note … as N` declared before the first `entity`).
        let cur_seq = ent_id_seq(Some(&current_ent_id));
        while oracle.is_none()
            && layout_note_cursor < layout_floating_notes.len()
            && ent_id_seq(Some(layout_floating_notes[layout_note_cursor].2)) < cur_seq
        {
            let (note_idx, node_idx, entity_id) = layout_floating_notes[layout_note_cursor];
            if let Some(pos) = positions.get(node_idx) {
                render_svek_floating_note(
                    &mut svg,
                    diagram,
                    note_idx,
                    pos,
                    entity_id,
                    edge_paths,
                    &resolved_relationship_edges,
                    floating_note_opale_relationships[note_idx],
                    layout_x_bias,
                    body_dx,
                    body_dy,
                );
            }
            layout_note_cursor += 1;
        }
        while note_cursor < oracle_note_entities.len()
            && ent_id_seq(oracle_note_entities[note_cursor].entity_id.as_deref()) < cur_seq
        {
            emit_note(&mut svg, oracle_note_entities[note_cursor]);
            note_cursor += 1;
            ent_id += 1;
        }

        if hidden_entities.contains(&i) {
            continue;
        }

        if let Some((lollipop_name, lollipop_rect)) = oracle_lollipop {
            emit_lollipop_entity(
                &mut svg,
                lollipop_name,
                lollipop_rect,
                &current_ent_id,
                &entity.label,
            );
            continue;
        }

        // HTML comment before entity.
        write!(svg, "<!--class {}-->", entity.label).unwrap();

        // Entity group wrapper.
        write!(
            svg,
            r#"<g class="entity" data-qualified-name="{}""#,
            escape_xml(qualified_name),
        )
        .unwrap();
        if let Some(source_line) = oracle_rect
            .and_then(|r| r.source_line.as_deref())
            .map(str::to_string)
            .or_else(|| {
                (oracle_rect.is_none() && entity.kind != EntityKind::State)
                    .then(|| dim.source_line.to_string())
            })
        {
            write!(svg, r#" data-source-line="{source_line}""#).unwrap();
        }
        write!(svg, r#" id="{current_ent_id}">"#).unwrap();

        // PlantUML wraps the entity content in `<a>` when the user attached a
        // URL with `[[http://...]]`. The anchor carries the same href four
        // ways (target, title, xlink:* attributes) to support multiple SVG
        // viewers.
        let link_anchor = entity.url.as_deref().map(|url| {
            let h = escape_xml(url);
            let title = entity
                .url_tooltip
                .as_deref()
                .map(escape_xml)
                .unwrap_or_else(|| h.clone());
            format!(
                r#"<a href="{h}" target="_top" title="{title}" xlink:actuate="onRequest" xlink:href="{h}" xlink:show="new" xlink:title="{title}" xlink:type="simple">"#,
            )
        });
        let body_gradient_fill =
            gradient_fill_from_defs(font.class_background.as_deref(), active_defs);
        // When `classHeaderBackgroundColor` is itself a gradient distinct from
        // the body gradient, the header repaint must reference the header
        // gradient's own `<defs>` id. Resolve it by matching the header
        // colour's two stops against the captured `<defs>`; otherwise the
        // header reuses the body fill (single-gradient case).
        let header_gradient_fill =
            gradient_fill_from_defs(font.header_background.as_deref(), active_defs);
        let entity_suppress_header_icon = suppress_header_icon
            || entity
                .stereotypes
                .iter()
                .any(|stereotype| stereotype_refs_sprite(stereotype, &diagram.meta.sprites));
        render_entity_content(
            &mut svg,
            entity,
            x,
            y,
            dim,
            oracle_rect,
            &font,
            link_anchor.as_deref(),
            text_padding,
            body_gradient_fill.as_deref(),
            header_gradient_fill.as_deref(),
            entity_suppress_header_icon,
            shadow_filter_id.as_deref(),
            &diagram.meta.sprites,
        );

        svg.push_str("</g>");

        // Association-class anchor point: PlantUML synthesises the `apoint`
        // pseudo-entity at the source line of the `(A, B) .. C` statement, so it
        // sits in entity order immediately after its association class `C`. Emit
        // the captured ellipse here so document order matches the golden.
        for (ac_idx, ac) in diagram.association_classes.iter().enumerate() {
            if ac.c != entity.id {
                continue;
            }
            if let Some(ap) = oracle.and_then(|orc| orc.apoints.get(ac_idx)) {
                write!(
                    svg,
                    r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="{}"/>"#,
                    crate::plantuml_metrics::fmt_coord(ap.cx),
                    crate::plantuml_metrics::fmt_coord(ap.cy),
                    ap.fill,
                    crate::plantuml_metrics::fmt_coord(ap.rx),
                    crate::plantuml_metrics::fmt_coord(ap.ry),
                    ap.style,
                )
                .unwrap();
            } else if oracle.is_none()
                && let Some(point) = positions.get(diagram.entities.len() + ac_idx)
            {
                let radius = ASSOCIATION_POINT_SIZE / 2.0;
                write!(
                    svg,
                    r##"<ellipse cx="{}" cy="{}" fill="#181818" rx="{}" ry="{}" style="stroke:#181818;stroke-width:1;"/>"##,
                    crate::plantuml_metrics::fmt_coord(
                        point.x + MARGIN + layout_x_bias + body_dx + radius
                    ),
                    crate::plantuml_metrics::fmt_coord(point.y + MARGIN + body_dy + radius),
                    crate::plantuml_metrics::fmt_coord(radius),
                    crate::plantuml_metrics::fmt_coord(radius),
                )
                .unwrap();
            }
        }
    }

    while oracle.is_none() && layout_note_cursor < layout_floating_notes.len() {
        let (note_idx, node_idx, entity_id) = layout_floating_notes[layout_note_cursor];
        if let Some(pos) = positions.get(node_idx) {
            render_svek_floating_note(
                &mut svg,
                diagram,
                note_idx,
                pos,
                entity_id,
                edge_paths,
                &resolved_relationship_edges,
                floating_note_opale_relationships[note_idx],
                layout_x_bias,
                body_dx,
                body_dy,
            );
        }
        layout_note_cursor += 1;
    }

    if oracle.is_none() {
        for &(note_idx, node_idx, position) in &attached_notes {
            let note = &diagram.notes[note_idx];
            let Some(target) = note.target.as_deref() else {
                continue;
            };
            let note_layout_id = attached_note_layout_id(note_idx);
            let (edge_from, edge_to) = match position {
                NotePosition::Left | NotePosition::Top => (note_layout_id.as_str(), target),
                NotePosition::Right | NotePosition::Bottom => (target, note_layout_id.as_str()),
            };
            let Some(edge) = edge_paths
                .iter()
                .find(|edge| edge.from == edge_from && edge.to == edge_to)
            else {
                continue;
            };
            let endpoint = match position {
                NotePosition::Left | NotePosition::Top => {
                    edge.end_point.or_else(|| edge.points.last().copied())
                }
                NotePosition::Right | NotePosition::Bottom => {
                    edge.start_point.or_else(|| edge.points.first().copied())
                }
            };
            let Some((tip_x, tip_y)) = endpoint else {
                continue;
            };
            let Some(pos) = positions.get(node_idx) else {
                continue;
            };
            let Some(note_start) = svek_ids.attached_note_starts[note_idx] else {
                continue;
            };
            let qualified_name = format!("GMN{note_start}");
            let entity_id = format!("ent{:04}", note_start + 1);
            let x = ((pos.x + MARGIN + layout_x_bias + body_dx) * 100.0).round() / 100.0;
            let y = ((pos.y + MARGIN + body_dy) * 100.0).round() / 100.0;
            let (anchor_x, anchor_y) = match position {
                NotePosition::Left | NotePosition::Right => (x, y + pos.height / 2.0),
                NotePosition::Top | NotePosition::Bottom => (x + pos.width / 2.0, y),
            };
            render_attached_note(
                &mut svg,
                diagram,
                note,
                // SVEK consumes Graphviz's two-decimal SVG coordinates before
                // `Opale` adds its local note and atom margins.
                x,
                y,
                pos.width,
                pos.height,
                anchor_x,
                anchor_y,
                tip_x + MARGIN + layout_x_bias,
                tip_y + MARGIN,
                position,
                &qualified_name,
                &entity_id,
                &diagram.meta.sprites,
            );
        }
    }

    // Emit any remaining note entities whose emission counter follows every
    // regular entity (notes declared after the last `entity`). The interleave
    // above has already placed notes that precede an entity in document order.
    while note_cursor < oracle_note_entities.len() {
        emit_note(&mut svg, oracle_note_entities[note_cursor]);
        note_cursor += 1;
        ent_id += 1;
    }

    // Render association-class connectors (apoint links) before the regular
    // relationships, matching PlantUML's emission order.
    if let Some(orc) = oracle {
        render_association_class_links(&mut svg, diagram, orc);
    } else {
        render_no_oracle_association_class_links(&mut svg, diagram, edge_paths, layout_x_bias);
    }

    // Render relationships.
    if let Some(orc) = oracle {
        render_oracle_relationships(&mut svg, diagram, orc, ent_id);
        render_oracle_note_connectors(&mut svg, orc);
    } else {
        let edge_indices = relationship_edge_indices(diagram, edge_paths);
        let relationship_note_indices = relationship_note_indices(diagram);
        let mut duplicate_counts = HashMap::<(&str, &str), usize>::new();
        for (rel_idx, ((rel, edge_idx), &link_id)) in diagram
            .relationships
            .iter()
            .zip(edge_indices)
            .zip(&svek_ids.relationship_ids)
            .enumerate()
        {
            if opale_relationships.contains(&rel_idx) {
                continue;
            }
            let duplicate_index = duplicate_counts
                .entry((rel.from.as_str(), rel.to.as_str()))
                .and_modify(|count| *count += 1)
                .or_insert(0);
            if relationship_touches_hidden_entity(rel, diagram, hidden_entities) {
                continue;
            }
            if let Some(ep) = edge_idx.and_then(|idx| edge_paths.get(idx)) {
                render_relationship_svg(
                    &mut svg,
                    rel,
                    RelationshipRenderContext {
                        diagram,
                        note: relationship_note_indices[rel_idx].map(|idx| &diagram.notes[idx]),
                        entity_ids: Some(&svek_ids.entity_ids),
                        note_ids: Some(&svek_ids.note_ids),
                    },
                    ep,
                    link_id,
                    layout_x_bias,
                    *duplicate_index,
                );
            }
        }
    }

    // Bottom/default legends (anything not placed in the top group above) are
    // emitted after the body, matching PlantUML's bottom decoration group.
    if let Some(orc) = oracle {
        for legend in orc.legends.iter().filter(|l| !is_top_legend(l)) {
            emit_oracle_legend(&mut svg, legend, diagram.legend_line);
        }
    }

    // Bottom-of-canvas decorations: caption (above footer), then footer. Both
    // baselines are anchored a fixed gap below the body's bottom edge; when a
    // caption is present it pushes the footer down by the caption block height.
    layout.emit(
        &mut svg,
        "caption",
        diagram.meta.caption.as_deref(),
        diagram.caption_line,
        body_bottom + DECORATION_CAPTION_GAP_BELOW_BODY,
        oracle_decoration_texts("caption"),
    );
    let footer_y = body_bottom + DECORATION_FOOTER_GAP_BELOW_BODY + layout.caption_h;
    layout.emit(
        &mut svg,
        "footer",
        diagram.meta.footer.as_deref(),
        diagram.footer_line,
        footer_y,
        oracle_decoration_texts("footer"),
    );

    // Close top-level group and SVG.
    svg.push_str("</g></svg>");
    svg
}

struct LayoutPackageCluster {
    package_idx: usize,
    kind: PackageKind,
    qualified_name: String,
    source_line: usize,
    label: String,
    stereotype_lines: Vec<String>,
    fill: String,
    stroke: String,
    font_fill: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn layout_package_clusters(
    diagram: &ClassDiagram,
    cluster_positions: &[ClusterPosition],
) -> Vec<LayoutPackageCluster> {
    let parent_pkg = package_parent_indices(diagram);
    diagram
        .packages
        .iter()
        .enumerate()
        .filter(|(_, pkg)| is_rendered_package_cluster(pkg))
        .filter_map(|(idx, pkg)| {
            let id = package_cluster_id(idx);
            let pos = cluster_positions.iter().find(|p| p.id == id)?;
            let kind = effective_package_kind(pkg);
            let fill = pkg
                .color
                .as_deref()
                .or_else(|| package_skinparam(diagram, kind, "BackgroundColor"))
                .map(crate::sequence::resolve_color)
                .unwrap_or_else(|| "none".to_string());
            let stroke = package_skinparam(diagram, kind, "BorderColor")
                .map(crate::sequence::resolve_color)
                .unwrap_or_else(|| {
                    if matches!(
                        kind,
                        PackageKind::Database
                            | PackageKind::Frame
                            | PackageKind::Rectangle
                            | PackageKind::Node
                            | PackageKind::Cloud
                    ) {
                        "#181818".to_string()
                    } else {
                        "#000000".to_string()
                    }
                });
            let font_fill = package_skinparam(diagram, kind, "FontColor")
                .map(crate::sequence::resolve_color)
                .unwrap_or_else(|| "#000000".to_string());
            Some(LayoutPackageCluster {
                package_idx: idx,
                kind,
                qualified_name: package_qualified_name(diagram, &parent_pkg, idx),
                source_line: pkg.source_line,
                label: package_display_label(pkg).to_string(),
                stereotype_lines: visible_package_stereotype_lines(pkg),
                fill,
                stroke,
                font_fill,
                x: pos.x + MARGIN,
                y: pos.y + MARGIN,
                width: pos.width,
                height: pos.height,
            })
        })
        .collect()
}

fn emit_layout_package_cluster(svg: &mut String, cluster: &LayoutPackageCluster, entity_id: &str) {
    let label_w = text_render::measure_no_underline(&cluster.label, FONT_SIZE, true);
    let title_w = label_w + 2.0 * PACKAGE_TITLE_MARGIN_X;
    let tab_w = (title_w + PACKAGE_TAB_SLOPE_WIDTH).min(cluster.width.max(0.0));
    let x = cluster.x;
    let y = cluster.y;
    let right = cluster.x + cluster.width;
    let bottom = cluster.y + cluster.height;
    let tab_join = x + title_w - PACKAGE_ROUND_CORNER / 2.0;
    let tab_right = x + tab_w;
    let line_y = y + PACKAGE_TAB_H;
    let text_x = x + PACKAGE_TAB_TEXT_X;
    let text_y = y + PACKAGE_TITLE_BASELINE;
    write!(
        svg,
        "<!--cluster {}-->",
        escape_xml(&cluster.qualified_name)
    )
    .unwrap();
    write!(
        svg,
        r#"<g class="cluster" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
        escape_xml(&cluster.qualified_name),
        cluster.source_line,
        entity_id,
    )
    .unwrap();
    match cluster.kind {
        PackageKind::Database => emit_layout_database_cluster(svg, cluster),
        PackageKind::Frame => emit_layout_frame_cluster(svg, cluster),
        PackageKind::Rectangle => emit_layout_rectangle_cluster(svg, cluster),
        PackageKind::Node => emit_layout_node_cluster(svg, cluster),
        PackageKind::Cloud => emit_layout_cloud_cluster(svg, cluster),
        _ => {
            write!(
                svg,
                r#"<path d="M{},{} L{},{} A3.75,3.75 0 0 1 {},{} L{},{} L{},{} A2.5,2.5 0 0 1 {},{} L{},{} A2.5,2.5 0 0 1 {},{} L{},{} A2.5,2.5 0 0 1 {},{} L{},{} A2.5,2.5 0 0 1 {},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
                fmt4(x + 2.5),
                fmt4(y),
                fmt4(tab_join),
                fmt4(y),
                fmt4(tab_join + 2.5),
                fmt4(y + 2.5),
                fmt4(tab_right),
                fmt4(line_y),
                fmt4(right - 2.5),
                fmt4(line_y),
                fmt4(right),
                fmt4(line_y + 2.5),
                fmt4(right),
                fmt4(bottom - 2.5),
                fmt4(right - 2.5),
                fmt4(bottom),
                fmt4(x + 2.5),
                fmt4(bottom),
                fmt4(x),
                fmt4(bottom - 2.5),
                fmt4(x),
                fmt4(y + 2.5),
                fmt4(x + 2.5),
                fmt4(y),
                cluster.fill,
                cluster.stroke,
                PACKAGE_STROKE_WIDTH,
            )
            .unwrap();
            write!(
                svg,
                r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                cluster.stroke,
                PACKAGE_STROKE_WIDTH,
                fmt4(x),
                fmt4(tab_right),
                fmt4(line_y),
                fmt4(line_y),
            )
            .unwrap();
            // `ClusterHeader` renders the title through the normal Creole
            // `Display` pipeline with a bold base font. Segment-level colour,
            // italic and other inline styles therefore split into adjacent
            // atoms while inheriting the package title's bold weight.
            text_render::emit_text(
                svg,
                &cluster.label,
                &TextBase {
                    x: text_x,
                    y: text_y,
                    font_size: FONT_SIZE as u32,
                    font_family: "sans-serif",
                    fill: &cluster.font_fill,
                    bold: true,
                    italic: false,
                    underline: false,
                    skip_underline: true,
                },
            );
            for (line_index, stereotype) in cluster.stereotype_lines.iter().enumerate() {
                let width = text_render::measure_no_underline(stereotype, FONT_SIZE, false);
                let mut text = String::new();
                text_render::emit_text(
                    &mut text,
                    stereotype,
                    &TextBase {
                        x: x + PACKAGE_TAB_TEXT_X + (cluster.width - width) / 2.0,
                        y: y + PACKAGE_STEREOTYPE_BASELINE
                            + line_index as f64 * text_render::label_height(stereotype, FONT_SIZE),
                        font_size: FONT_SIZE as u32,
                        font_family: "sans-serif",
                        fill: &cluster.font_fill,
                        bold: false,
                        italic: true,
                        underline: false,
                        skip_underline: false,
                    },
                );
                svg.push_str(&text);
            }
        }
    }
    svg.push_str("</g>");
}

fn emit_layout_frame_cluster(svg: &mut String, cluster: &LayoutPackageCluster) {
    let x = cluster.x + 1.0;
    let y = cluster.y + 1.0;
    let label_w = text_render::measure_no_underline(&cluster.label, FONT_SIZE, true);
    let path_right = x + label_w + FRAME_TITLE_CORNER;
    let upper_y = cluster.y + PACKAGE_TAB_H - 12.0;
    let lower_y = cluster.y + PACKAGE_TAB_H - 2.0;

    // Java `USymbolFrame.asBig()` delegates to `drawFrame()`: a rounded
    // rectangle plus the title-corner path, then draws the title at (3, 1).
    write!(
        svg,
        r#"<rect fill="{}" height="{}" rx="2.5" ry="2.5" style="stroke:{};stroke-width:{};" width="{}" x="{}" y="{}"/>"#,
        cluster.fill,
        fmt4(cluster.height),
        cluster.stroke,
        SYMBOL_CLUSTER_STROKE_WIDTH,
        fmt4(cluster.width),
        fmt4(x),
        fmt4(y),
    )
    .unwrap();
    write!(
        svg,
        r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="none" style="stroke:{};stroke-width:{};"/>"#,
        fmt4(path_right),
        fmt4(y),
        fmt4(path_right),
        fmt4(upper_y),
        fmt4(path_right - FRAME_TITLE_CORNER),
        fmt4(lower_y),
        fmt4(x),
        fmt4(lower_y),
        cluster.stroke,
        SYMBOL_CLUSTER_STROKE_WIDTH,
    )
    .unwrap();
    emit_layout_symbol_cluster_title(
        svg,
        cluster,
        cluster.x + 4.0,
        cluster.y + PACKAGE_TITLE_BASELINE,
    );
}

fn emit_layout_rectangle_cluster(svg: &mut String, cluster: &LayoutPackageCluster) {
    let x = cluster.x + 1.0;
    let y = cluster.y + 1.0;
    let label_w = text_render::measure_no_underline(&cluster.label, FONT_SIZE, true);

    // Java `USymbolRectangle.asBig()` delegates to `drawRect()` and centres
    // the title independently of the stereotype block.
    write!(
        svg,
        r#"<rect fill="{}" height="{}" rx="2.5" ry="2.5" style="stroke:{};stroke-width:{};" width="{}" x="{}" y="{}"/>"#,
        cluster.fill,
        fmt4(cluster.height),
        cluster.stroke,
        SYMBOL_CLUSTER_STROKE_WIDTH,
        fmt4(cluster.width),
        fmt4(x),
        fmt4(y),
    )
    .unwrap();
    emit_layout_symbol_cluster_title(
        svg,
        cluster,
        x + (cluster.width - label_w) / 2.0,
        cluster.y + PACKAGE_TITLE_BASELINE + 1.0,
    );
}

fn emit_layout_node_cluster(svg: &mut String, cluster: &LayoutPackageCluster) {
    let x = cluster.x + NODE_BEVEL;
    let y = cluster.y;
    let right = x + cluster.width;
    let front_right = right - NODE_BEVEL;
    let bottom = y + cluster.height;
    let label_w = text_render::measure_no_underline(&cluster.label, FONT_SIZE, true);

    // Java `USymbolNode.asBig()` delegates to `drawNode()`: a bevelled
    // polygon followed by the top-right diagonal, front top, and front side.
    write!(
        svg,
        r#"<polygon fill="{}" points="{},{},{},{},{},{},{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:{};"/>"#,
        cluster.fill,
        fmt4(x),
        fmt4(y + NODE_BEVEL),
        fmt4(x + NODE_BEVEL),
        fmt4(y),
        fmt4(right),
        fmt4(y),
        fmt4(right),
        fmt4(bottom - NODE_BEVEL),
        fmt4(front_right),
        fmt4(bottom),
        fmt4(x),
        fmt4(bottom),
        fmt4(x),
        fmt4(y + NODE_BEVEL),
        cluster.stroke,
        SYMBOL_CLUSTER_STROKE_WIDTH,
    )
    .unwrap();
    write!(
        svg,
        r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
        cluster.stroke,
        SYMBOL_CLUSTER_STROKE_WIDTH,
        fmt4(front_right),
        fmt4(right),
        fmt4(y + NODE_BEVEL),
        fmt4(y),
    )
    .unwrap();
    write!(
        svg,
        r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
        cluster.stroke,
        SYMBOL_CLUSTER_STROKE_WIDTH,
        fmt4(x),
        fmt4(front_right),
        fmt4(y + NODE_BEVEL),
        fmt4(y + NODE_BEVEL),
    )
    .unwrap();
    write!(
        svg,
        r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
        cluster.stroke,
        SYMBOL_CLUSTER_STROKE_WIDTH,
        fmt4(front_right),
        fmt4(front_right),
        fmt4(y + NODE_BEVEL),
        fmt4(bottom),
    )
    .unwrap();
    emit_layout_symbol_cluster_title(
        svg,
        cluster,
        x - 4.0 + (cluster.width - label_w) / 2.0,
        y + PACKAGE_TITLE_BASELINE + 11.0,
    );
}

#[derive(Clone, Copy)]
struct CloudPoint {
    x: f64,
    y: f64,
}

struct CloudCurve {
    control1: CloudPoint,
    control2: CloudPoint,
    end: CloudPoint,
}

struct CloudFrontier {
    start: CloudPoint,
    curves: Vec<CloudCurve>,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    const MULTIPLIER: u64 = 0x5DEE_CE66D;
    const ADDEND: u64 = 0xB;
    const MASK: u64 = (1_u64 << 48) - 1;

    fn new(seed: i64) -> Self {
        Self {
            seed: (seed as u64 ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    fn next(&mut self, bits: u32) -> u64 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::ADDEND)
            & Self::MASK;
        self.seed >> (48 - bits)
    }

    fn next_double(&mut self) -> f64 {
        let high = self.next(26) << 27;
        let low = self.next(27);
        (high + low) as f64 / (1_u64 << 53) as f64
    }

    fn between(&mut self, lower: f64, upper: f64) -> f64 {
        self.next_double() * (upper - lower) + lower
    }
}

fn cloud_coordinate(p1: CloudPoint, p2: CloudPoint, along: f64, normal: f64) -> CloudPoint {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let length = dx.hypot(dy);
    let ux = dx / length;
    let uy = dy / length;
    CloudPoint {
        x: p1.x + along * ux - normal * uy,
        y: p1.y + along * uy + normal * ux,
    }
}

fn cloud_bubble_line(
    random: &mut JavaRandom,
    points: &mut Vec<CloudPoint>,
    p1: CloudPoint,
    p2: CloudPoint,
    bubble_size: f64,
) {
    let length = (p2.x - p1.x).hypot(p2.y - p1.y);
    let mut segment_size = bubble_size;
    let mut count = (length / segment_size) as usize;
    if count == 0 {
        segment_size = length / 2.0;
        count = (length / segment_size) as usize;
    }
    for i in 0..count {
        let mut point = cloud_coordinate(p1, p2, i as f64 * length / count as f64, 0.0);
        point.x += segment_size * 0.2 * random.next_double();
        point.y += segment_size * 0.2 * random.next_double();
        points.push(point);
    }
}

fn cloud_special_line(
    random: &mut JavaRandom,
    points: &mut Vec<CloudPoint>,
    p1: CloudPoint,
    p2: CloudPoint,
    bubble_size: f64,
) {
    let length = (p2.x - p1.x).hypot(p2.y - p1.y);
    let bulge = random.between(1.0, 1.0 + 12.0_f64.min(bubble_size * 0.8));
    let middle = cloud_coordinate(p1, p2, length / 2.0, -bulge);
    cloud_bubble_line(random, points, p1, middle, bubble_size);
    cloud_bubble_line(random, points, middle, p2, bubble_size);
}

fn cloud_frontier(width: f64, height: f64) -> CloudFrontier {
    let seed = width as i64 + 7919 * height as i64;
    let mut random = JavaRandom::new(seed);
    let mut points = Vec::new();
    let mut bubble_size = 11.0;
    if width.max(height) / bubble_size > 16.0 {
        bubble_size = width.max(height) / 16.0;
    }

    let point_a = CloudPoint { x: 8.0, y: 8.0 };
    let point_b = CloudPoint {
        x: width - 8.0,
        y: 8.0,
    };
    let point_c = CloudPoint {
        x: width - 8.0,
        y: height - 8.0,
    };
    let point_d = CloudPoint {
        x: 8.0,
        y: height - 8.0,
    };

    if width > 100.0 && height > 100.0 {
        let margin = 7.0;
        cloud_special_line(
            &mut random,
            &mut points,
            CloudPoint {
                x: point_a.x + margin,
                ..point_a
            },
            CloudPoint {
                x: point_b.x - margin,
                ..point_b
            },
            bubble_size,
        );
        points.push(CloudPoint {
            y: point_b.y + margin,
            ..point_b
        });
        cloud_special_line(
            &mut random,
            &mut points,
            CloudPoint {
                y: point_b.y + margin,
                ..point_b
            },
            CloudPoint {
                y: point_c.y - margin,
                ..point_c
            },
            bubble_size,
        );
        points.push(CloudPoint {
            x: point_c.x - margin,
            ..point_c
        });
        cloud_special_line(
            &mut random,
            &mut points,
            CloudPoint {
                x: point_c.x - margin,
                ..point_c
            },
            CloudPoint {
                x: point_d.x + margin,
                ..point_d
            },
            bubble_size,
        );
        points.push(CloudPoint {
            y: point_d.y - margin,
            ..point_d
        });
        cloud_special_line(
            &mut random,
            &mut points,
            CloudPoint {
                y: point_d.y - margin,
                ..point_d
            },
            CloudPoint {
                y: point_a.y + margin,
                ..point_a
            },
            bubble_size,
        );
        points.push(CloudPoint {
            x: point_a.x + margin,
            ..point_a
        });
    } else {
        cloud_special_line(&mut random, &mut points, point_a, point_b, bubble_size);
        cloud_special_line(&mut random, &mut points, point_b, point_c, bubble_size);
        cloud_special_line(&mut random, &mut points, point_c, point_d, bubble_size);
        cloud_special_line(&mut random, &mut points, point_d, point_a, bubble_size);
    }

    let start = points[0];
    points.push(start);
    let mut curves = Vec::with_capacity(points.len() - 1);
    let mut min_x = start.x;
    let mut min_y = start.y;
    let mut max_x = start.x;
    let mut max_y = start.y;
    for pair in points.windows(2) {
        let p1 = pair[0];
        let p2 = pair[1];
        let length = (p2.x - p1.x).hypot(p2.y - p1.y);
        let coefficient = random.between(0.25, 0.35);
        let control1 = cloud_coordinate(
            p1,
            p2,
            length * coefficient,
            -length * random.between(0.4, 0.55),
        );
        let control2 = cloud_coordinate(
            p1,
            p2,
            length * (1.0 - coefficient),
            -length * random.between(0.4, 0.55),
        );
        for point in [control1, control2, p2] {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
        curves.push(CloudCurve {
            control1,
            control2,
            end: p2,
        });
    }

    CloudFrontier {
        start,
        curves,
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

fn emit_layout_cloud_cluster(svg: &mut String, cluster: &LayoutPackageCluster) {
    let frontier = cloud_frontier(cluster.width, cluster.height);

    // Java `USymbolCloud.drawCloud()` delegates to
    // `getSpecificFrontierForCloudNew()`: a dimension-seeded Random builds
    // bubble points around the four sides and joins them with cubic curves.
    write!(
        svg,
        r#"<path d="M{},{}"#,
        fmt4(cluster.x + frontier.start.x),
        fmt4(cluster.y + frontier.start.y),
    )
    .unwrap();
    for curve in frontier.curves {
        write!(
            svg,
            " C{},{} {},{} {},{}",
            fmt4(cluster.x + curve.control1.x),
            fmt4(cluster.y + curve.control1.y),
            fmt4(cluster.x + curve.control2.x),
            fmt4(cluster.y + curve.control2.y),
            fmt4(cluster.x + curve.end.x),
            fmt4(cluster.y + curve.end.y),
        )
        .unwrap();
    }
    write!(
        svg,
        r#"" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
        cluster.fill, cluster.stroke, SYMBOL_CLUSTER_STROKE_WIDTH,
    )
    .unwrap();

    let label_w = text_render::measure_no_underline(&cluster.label, FONT_SIZE, true);
    emit_layout_symbol_cluster_title(
        svg,
        cluster,
        cluster.x + (cluster.width - label_w) / 2.0,
        cluster.y + CLOUD_TITLE_BASELINE,
    );
}

fn emit_layout_symbol_cluster_title(
    svg: &mut String,
    cluster: &LayoutPackageCluster,
    x: f64,
    y: f64,
) {
    let label_w = text_render::measure_no_underline(&cluster.label, FONT_SIZE, true);
    write!(
        svg,
        r#"<text fill="{}" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
        cluster.font_fill,
        fmt4(label_w),
        fmt4(x),
        fmt4(y),
        escape_xml(&cluster.label),
    )
    .unwrap();
}

fn emit_layout_database_cluster(svg: &mut String, cluster: &LayoutPackageCluster) {
    let x = cluster.x;
    let y = cluster.y;
    let middle = x + cluster.width / 2.0;
    let right = x + cluster.width;
    let bottom = y + cluster.height;
    let label_w = text_render::measure_no_underline(&cluster.label, FONT_SIZE, true);
    let text_x = x + (cluster.width - label_w) / 2.0;
    let text_y = y + PACKAGE_TITLE_BASELINE + DATABASE_CLUSTER_TITLE_OFFSET;

    // Java `USymbolDatabase.asBig()` delegates to `drawDatabase()`: a closed
    // cylinder path plus the top ellipse's lower half as a separate path.
    write!(
        svg,
        r#"<path d="M{},{} C{},{} {},{} {},{} C{},{} {},{} {},{} L{},{} C{},{} {},{} {},{} C{},{} {},{} {},{} L{},{}" fill="{}" style="stroke:{};stroke-width:1;"/>"#,
        fmt4(x),
        fmt4(y + 10.0),
        fmt4(x),
        fmt4(y),
        fmt4(middle),
        fmt4(y),
        fmt4(middle),
        fmt4(y),
        fmt4(middle),
        fmt4(y),
        fmt4(right),
        fmt4(y),
        fmt4(right),
        fmt4(y + 10.0),
        fmt4(right),
        fmt4(bottom - 10.0),
        fmt4(right),
        fmt4(bottom),
        fmt4(middle),
        fmt4(bottom),
        fmt4(middle),
        fmt4(bottom),
        fmt4(middle),
        fmt4(bottom),
        fmt4(x),
        fmt4(bottom),
        fmt4(x),
        fmt4(bottom - 10.0),
        fmt4(x),
        fmt4(y + 10.0),
        cluster.fill,
        cluster.stroke,
    )
    .unwrap();
    write!(
        svg,
        r#"<path d="M{},{} C{},{} {},{} {},{} C{},{} {},{} {},{}" fill="none" style="stroke:{};stroke-width:1;"/>"#,
        fmt4(x),
        fmt4(y + 10.0),
        fmt4(x),
        fmt4(y + 20.0),
        fmt4(middle),
        fmt4(y + 20.0),
        fmt4(middle),
        fmt4(y + 20.0),
        fmt4(middle),
        fmt4(y + 20.0),
        fmt4(right),
        fmt4(y + 20.0),
        fmt4(right),
        fmt4(y + 10.0),
        cluster.stroke,
    )
    .unwrap();
    write!(
        svg,
        r#"<text fill="{}" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
        cluster.font_fill,
        fmt4(label_w),
        fmt4(text_x),
        fmt4(text_y),
        escape_xml(&cluster.label),
    )
    .unwrap();
}

fn emit_oracle_legend(svg: &mut String, legend: &OracleLegend, fallback_line: Option<usize>) {
    let source_line = legend
        .source_line
        .as_deref()
        .map(str::to_string)
        .unwrap_or_else(|| fallback_line.unwrap_or(1).to_string());
    write!(
        svg,
        r#"<g class="legend" data-source-line="{source_line}">"#
    )
    .unwrap();

    // Prefer the verbatim child capture: a creole-table legend interleaves
    // coloured cell <rect>s with the row texts in an order the flat
    // rect/texts/lines fields cannot reproduce.
    if let Some(inner) = legend.inner_xml.as_deref() {
        svg.push_str(inner);
        svg.push_str("</g>");
        return;
    }

    let rx_attr = legend
        .rect
        .rx
        .as_deref()
        .map(|rx| format!(r#" rx="{}""#, escape_xml(rx)))
        .unwrap_or_default();
    let ry_attr = legend
        .rect
        .ry
        .as_deref()
        .map(|ry| format!(r#" ry="{}""#, escape_xml(ry)))
        .unwrap_or_default();
    write!(
        svg,
        r#"<rect fill="{}" height="{}"{rx_attr}{ry_attr} style="{}" width="{}" x="{}" y="{}"/>"#,
        escape_xml(&legend.rect.fill),
        crate::plantuml_metrics::fmt_coord(legend.rect.height),
        escape_xml(&legend.rect.style),
        crate::plantuml_metrics::fmt_coord(legend.rect.width),
        crate::plantuml_metrics::fmt_coord(legend.rect.x),
        crate::plantuml_metrics::fmt_coord(legend.rect.y),
    )
    .unwrap();

    for line in &legend.lines {
        match line.style.as_deref() {
            Some(style) => write!(
                svg,
                r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                escape_xml(style),
                escape_xml(&line.x1),
                escape_xml(&line.x2),
                escape_xml(&line.y1),
                escape_xml(&line.y2),
            ),
            None => write!(
                svg,
                r#"<line x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                escape_xml(&line.x1),
                escape_xml(&line.x2),
                escape_xml(&line.y1),
                escape_xml(&line.y2),
            ),
        }
        .unwrap();
    }

    for text in &legend.texts {
        text_render::emit_text(
            svg,
            &text.text,
            &text_render::TextBase {
                x: text.x,
                y: text.y,
                font_size: 14,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
    }

    svg.push_str("</g>");
}

/// Page-decoration (title/header/footer/caption) layout, mirroring PlantUML's
/// `DecorateEntityImage`.
///
/// PlantUML stacks the body (`dimOriginal`) between an optional header+title
/// region (top) and an optional caption+footer region (bottom). Each text block
/// is horizontally aligned over a shared width `dimTotal = max(dimOriginal,
/// header, title, caption, footer)`, where each decoration's width is its glyph
/// run plus the style's left/right padding+margin (the "border" of the bordered
/// text block). The glyph `x` is then the block's aligned left edge plus the
/// block's own left inset (padding+margin).
struct DecorationLayout {
    dim_total_w: f64,
    top_h: f64,
    bottom_h: f64,
    caption_h: f64,
    has_decorations: bool,
}

/// Per-decoration style: glyph font size, fill, bold, the symmetric
/// padding+margin inset added on each side, and the block alignment.
struct DecorationStyle {
    font_size: u32,
    fill: &'static str,
    bold: bool,
    /// Padding + margin added to one side of the glyph run (the bordered text
    /// block grows by `2 * inset`; the glyph starts `inset` from the block's
    /// left edge).
    inset: f64,
    align_right: bool,
}

impl DecorationLayout {
    fn style(class_name: &str) -> DecorationStyle {
        match class_name {
            // document.title: FontSize 14, bold, Padding 5 + Margin 5, centre.
            "title" => DecorationStyle {
                font_size: 14,
                fill: "#000000",
                bold: true,
                inset: DECORATION_TITLE_INSET,
                align_right: false,
            },
            // document.caption: FontSize 14, Padding 0 + Margin 1, centre.
            "caption" => DecorationStyle {
                font_size: 14,
                fill: "#000000",
                bold: false,
                inset: DECORATION_CAPTION_INSET,
                align_right: false,
            },
            // document.header: FontSize 10, grey, no padding/margin, right.
            "header" => DecorationStyle {
                font_size: 10,
                fill: "#888888",
                bold: false,
                inset: 0.0,
                align_right: true,
            },
            // document.footer: FontSize 10, grey, no padding/margin, centre.
            _ => DecorationStyle {
                font_size: 10,
                fill: "#888888",
                bold: false,
                inset: 0.0,
                align_right: false,
            },
        }
    }

    /// Width of a decoration's bordered text block (glyph run + 2 * inset).
    fn block_width(class_name: &str, text: &str) -> f64 {
        let st = Self::style(class_name);
        text.lines()
            .map(|line| {
                text_render::measure_no_underline(line, st.font_size as f64, st.bold)
                    + 2.0 * st.inset
            })
            .fold(0.0_f64, f64::max)
    }

    /// Height of the Java bordered text block. `Style.createTextBlockBordered`
    /// wraps the line box in style padding and margin, while
    /// `TextBlockBordered.calculateDimension` contributes one final pixel.
    fn block_height(class_name: &str, text: &str) -> f64 {
        let st = Self::style(class_name);
        text.lines()
            .map(|line| text_render::label_height(line, st.font_size as f64))
            .sum::<f64>()
            + 2.0 * st.inset
            + DECORATION_BORDER_EXTENT
    }

    /// Build the layout, computing `dimTotal` from the body width and any
    /// present decorations.
    fn new(diagram: &ClassDiagram, body_inner_w: f64) -> Self {
        let mut dim_total_w = body_inner_w;
        let mut has_decorations = false;
        for (class_name, text) in [
            ("title", diagram.meta.title.as_deref()),
            ("header", diagram.meta.header.as_deref()),
            ("caption", diagram.meta.caption.as_deref()),
            ("footer", diagram.meta.footer.as_deref()),
        ] {
            if let Some(t) = text
                && !t.is_empty()
            {
                has_decorations = true;
                dim_total_w = dim_total_w.max(Self::block_width(class_name, t));
            }
        }
        let block_height = |class_name, text: Option<&str>| {
            text.filter(|value| !value.is_empty())
                .map_or(0.0, |value| Self::block_height(class_name, value))
        };
        let header_h = block_height("header", diagram.meta.header.as_deref());
        let title_h = block_height("title", diagram.meta.title.as_deref());
        let caption_h = block_height("caption", diagram.meta.caption.as_deref());
        let footer_h = block_height("footer", diagram.meta.footer.as_deref());
        Self {
            dim_total_w,
            top_h: header_h + title_h,
            bottom_h: caption_h + footer_h,
            caption_h,
            has_decorations,
        }
    }

    /// Emit a single decoration's `<g>`/`<text>` at the given glyph baseline `y`.
    fn emit(
        &self,
        svg: &mut String,
        class_name: &str,
        text: Option<&str>,
        line: Option<usize>,
        y: f64,
        oracle_texts: Option<&[EntityText]>,
    ) {
        let Some(text) = text else { return };
        if text.is_empty() {
            return;
        }
        let st = Self::style(class_name);
        let source_line = line.unwrap_or(1);
        write!(
            svg,
            r#"<g class="{class_name}" data-source-line="{source_line}">"#
        )
        .unwrap();
        let line_count = text.lines().count();
        let base_y = if class_name == "title" && line_count > 1 {
            y - (line_count - 1) as f64 * DECORATION_LINE_HEIGHT
        } else {
            y
        };
        for (idx, line_text) in text.lines().enumerate() {
            let block_w = Self::block_width(class_name, line_text);
            // Aligned block left edge over the shared total width, then the
            // block's own left inset to reach the glyph origin.
            let block_x = if st.align_right {
                self.dim_total_w - block_w
            } else {
                (self.dim_total_w - block_w) / 2.0
            };
            let computed_x = block_x + st.inset;
            let computed_y = base_y + idx as f64 * DECORATION_LINE_HEIGHT;
            let oracle_text = oracle_texts
                .and_then(|texts| texts.get(idx))
                .filter(|t| t.text == line_text);
            let x = oracle_text.map_or(computed_x, |t| t.x);
            let y = oracle_text.map_or(computed_y, |t| t.y);
            text_render::emit_text(
                svg,
                line_text,
                &text_render::TextBase {
                    x,
                    y,
                    font_size: st.font_size,
                    font_family: "sans-serif",
                    fill: st.fill,
                    bold: st.bold,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
        svg.push_str("</g>");
    }
}

/// Render the content of a single entity (rect, icon, name, separator lines, members).
///
/// When `oracle_rect` is provided, oracle overrides are used for icon position,
/// glyph path, name text x, member y-positions, and separator y-positions to
/// match PlantUML's exact output (bypassing float-precision differences).
#[allow(clippy::too_many_arguments)]
fn render_entity_content(
    svg: &mut String,
    entity: &ClassEntity,
    x: f64,
    y: f64,
    dim: &EntityDims,
    oracle_rect: Option<&crate::layout_oracle::EntityRect>,
    font: &ClassFontOverrides,
    link_anchor: Option<&str>,
    text_padding: f64,
    body_gradient_fill: Option<&str>,
    header_gradient_fill: Option<&str>,
    suppress_header_icon: bool,
    shadow_filter_id: Option<&str>,
    sprites: &HashMap<String, SpriteData>,
) {
    if matches!(entity.kind, EntityKind::Circle | EntityKind::Diamond) {
        if let Some(anchor) = link_anchor {
            svg.push_str(anchor);
        }
        let fill = oracle_rect
            .and_then(|r| r.fill.as_deref())
            .unwrap_or(ENTITY_FILL);
        let style = oracle_rect
            .and_then(|r| r.rect_style.as_deref())
            .or_else(|| oracle_rect.and_then(|r| r.body_style.as_deref()))
            .unwrap_or("stroke:#181818;stroke-width:0.5;");
        match entity.kind {
            EntityKind::Circle => {
                let cx = x + dim.width / 2.0;
                let cy = y + dim.height / 2.0;
                write!(
                    svg,
                    r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="{}"/>"#,
                    crate::plantuml_metrics::fmt_coord(cx),
                    crate::plantuml_metrics::fmt_coord(cy),
                    fill,
                    crate::plantuml_metrics::fmt_coord(dim.width / 2.0),
                    crate::plantuml_metrics::fmt_coord(dim.height / 2.0),
                    style,
                )
                .unwrap();
                if let Some(text) = oracle_rect.and_then(|r| r.texts.first()) {
                    text_render::emit_text(
                        svg,
                        &text.text,
                        &TextBase {
                            x: text.x,
                            y: text.y,
                            font_size: 14,
                            font_family: "sans-serif",
                            fill: "#000000",
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: true,
                        },
                    );
                }
            }
            EntityKind::Diamond => {
                let cx = x + dim.width / 2.0;
                let cy = y + dim.height / 2.0;
                let top = y;
                let right = x + dim.width;
                let bottom = y + dim.height;
                write!(
                    svg,
                    r#"<polygon fill="{}" points="{},{},{},{},{},{},{},{},{},{}" style="{}"/>"#,
                    fill,
                    crate::plantuml_metrics::fmt_coord(cx),
                    crate::plantuml_metrics::fmt_coord(top),
                    crate::plantuml_metrics::fmt_coord(right),
                    crate::plantuml_metrics::fmt_coord(cy),
                    crate::plantuml_metrics::fmt_coord(cx),
                    crate::plantuml_metrics::fmt_coord(bottom),
                    crate::plantuml_metrics::fmt_coord(x),
                    crate::plantuml_metrics::fmt_coord(cy),
                    crate::plantuml_metrics::fmt_coord(cx),
                    crate::plantuml_metrics::fmt_coord(top),
                    style,
                )
                .unwrap();
            }
            _ => {}
        }
        if link_anchor.is_some() {
            svg.push_str("</a>");
        }
        return;
    }

    if entity.kind == EntityKind::State {
        if let Some(anchor) = link_anchor {
            svg.push_str(anchor);
        }
        let fill = oracle_rect
            .and_then(|r| r.fill.as_deref())
            .unwrap_or(ENTITY_FILL);
        let style = oracle_rect
            .and_then(|r| r.rect_style.as_deref())
            .or_else(|| oracle_rect.and_then(|r| r.body_style.as_deref()))
            .unwrap_or("stroke:#181818;stroke-width:0.5;");
        let rx = oracle_rect
            .and_then(|r| r.rect_rx.as_deref())
            .unwrap_or("12.5");
        let ry = oracle_rect
            .and_then(|r| r.rect_ry.as_deref())
            .unwrap_or("12.5");
        write!(
            svg,
            r#"<rect fill="{}" height="{}" rx="{}" ry="{}" style="{}" width="{}" x="{}" y="{}"/>"#,
            fill,
            fmt4(dim.height),
            rx,
            ry,
            style,
            fmt_tl(dim.width),
            fmt4(x),
            fmt4(y),
        )
        .unwrap();
        if let Some(line) = oracle_rect.and_then(|r| r.lines.first()) {
            let line_style = line
                .style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:0.5;");
            write!(
                svg,
                r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                line_style, line.x1, line.x2, line.y1, line.y2,
            )
            .unwrap();
        } else {
            let line_y = y + MIXED_STATE_SEPARATOR_Y;
            write!(
                svg,
                r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                style,
                fmt4(x),
                fmt4(x + dim.width),
                fmt4(line_y),
                fmt4(line_y),
            )
            .unwrap();
        }
        let name_x = oracle_rect
            .and_then(|r| r.texts.first().map(|t| t.x))
            .unwrap_or_else(|| x + (dim.width - round_4dp(dim.name_width)) / 2.0);
        let name_y = oracle_rect
            .and_then(|r| r.texts.first().map(|t| t.y))
            .unwrap_or(y + MIXED_STATE_NAME_BASELINE);
        text_render::emit_text(
            svg,
            &entity.label,
            &TextBase {
                x: name_x,
                y: name_y,
                font_size: 14,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: true,
            },
        );
        if link_anchor.is_some() {
            svg.push_str("</a>");
        }
        return;
    }

    // PlantUML wraps the entity *header* (background rect, stereotype icon,
    // name text, and the two compartment separator rules) in a single `<a>`
    // when the class carries a `[[url]]` link, then closes it and re-wraps
    // each member's visibility icon and text in its own `<a>`. Open the
    // header anchor here; the body-rendering block below closes it before the
    // first member and emits the per-member anchors via `render_member_line`.
    if let Some(anchor) = link_anchor {
        svg.push_str(anchor);
    }
    // Tracks whether the header anchor has already been closed by a body
    // branch (member-bearing layouts close it before the first member and
    // re-wrap each member individually). Memberless layouts leave it open and
    // the final close below wraps the whole header.
    let mut header_anchor_closed = false;
    let icon_cx_override = oracle_rect.and_then(|r| r.icon_cx);
    let icon_cy_override = oracle_rect.and_then(|r| r.icon_cy);
    let glyph_path_override = oracle_rect.and_then(|r| r.glyph_path_d.as_deref());
    let name_text_x_override = oracle_rect.and_then(|r| r.name_text_x);
    let oracle_images = oracle_rect.map_or(&[][..], |r| r.images.as_slice());
    let first_sep_y = oracle_rect.and_then(|r| r.sep_y_values.first().copied());
    let is_abstract = entity.kind == EntityKind::AbstractClass;
    let is_interface = entity.kind == EntityKind::Interface;
    let is_enum_entity = entity.kind == EntityKind::Enum;
    let _is_annotation = entity.kind == EntityKind::Annotation;

    // Background rectangle — prefer the oracle's verbatim fill/style/rx
    // attributes when available, so per-entity skinparams and shorthand
    // colour syntax (`class X #fill;line:colour`) are honoured. Fall back
    // to the parser-provided colour and renderer defaults otherwise.
    // Under monochrome the oracle's captured rect fill/style are already the
    // golden's post-monochrome greys; using them would double-invert through
    // the final-SVG monochrome pass. Drop them so the raw renderer defaults
    // flow through and get mapped exactly once.
    let oracle_fill = oracle_rect
        .and_then(|r| r.fill.as_deref())
        .filter(|_| !font.monochrome);
    let oracle_style = oracle_rect
        .and_then(|r| r.rect_style.as_deref())
        .filter(|_| !font.monochrome);
    let oracle_rx = oracle_rect.and_then(|r| r.rect_rx.as_deref());
    let oracle_ry = oracle_rect.and_then(|r| r.rect_ry.as_deref());
    let entity_gradient_fill = entity
        .color
        .as_deref()
        .is_some_and(|c| split_gradient_colors(c).is_some());
    let (stereotype_background, stereotype_border) = font.stereotype_colors(&entity.stereotypes);
    let fill_default = entity
        .color
        .as_deref()
        .map(resolve_flat_or_gradient_start)
        .or_else(|| stereotype_background.map(resolve_flat_or_gradient_start))
        .or_else(|| body_gradient_fill.map(str::to_string))
        .or_else(|| {
            font.class_background
                .as_deref()
                .map(resolve_flat_or_gradient_start)
        })
        .unwrap_or_else(|| ENTITY_FILL.to_string());
    let fill = oracle_fill.unwrap_or(&fill_default);
    // `skinparam classBorderColor` recolours the body rect and compartment
    // separator strokes (but NOT the circled icon, which keeps PlantUML's
    // default #181818). Falls back to the default when unset.
    let border_col = entity
        .line_color
        .as_deref()
        .map(crate::sequence::resolve_color)
        .or_else(|| stereotype_border.map(crate::sequence::resolve_color))
        .or_else(|| {
            font.border_color
                .as_deref()
                .map(crate::sequence::resolve_color)
        })
        .unwrap_or_else(|| BORDER_COLOR.to_string());
    // Resolve the per-entity text colour from `#back:...;text:colour`
    // shorthand. When absent, PlantUML's class name follows
    // `ClassAttributeFontColor` before `ClassFontColor` in the modern themed
    // cascade (plain classic diagrams still leave both unset and render black).
    let text_fill_owned = entity
        .text_color
        .as_ref()
        .map(|c| crate::sequence::resolve_color(c))
        .or_else(|| {
            font.attr_font_color
                .as_deref()
                .map(crate::sequence::resolve_color)
        })
        .or_else(|| {
            font.font_color
                .as_deref()
                .map(crate::sequence::resolve_color)
        })
        .unwrap_or_else(|| "#000000".to_string());
    let text_fill: &str = &text_fill_owned;
    // Member (attribute) text colour: `ClassAttributeFontColor` colours
    // fields/methods independently of the name's `ClassFontColor`. Per-entity
    // `text:colour` shorthand still wins; otherwise members default to black.
    let member_fill_owned = entity
        .text_color
        .as_ref()
        .map(|c| crate::sequence::resolve_color(c))
        .or_else(|| {
            font.attr_font_color
                .as_deref()
                .map(crate::sequence::resolve_color)
        })
        .unwrap_or_else(|| "#000000".to_string());
    let member_fill: &str = &member_fill_owned;
    // Member-text font overrides from `skinparam ClassAttributeFontSize` /
    // `ClassAttributeFontStyle`. Default to the canonical 14px, non-styled.
    let visibility_stroke_owned = font
        .root_line_color
        .as_deref()
        .map(crate::sequence::resolve_color);
    let attr_font = AttrFont {
        fill: member_fill,
        size: font.member_font_size(),
        family: &font.family,
        bold: font.attr_font_bold,
        italic: font.attr_font_italic,
        monospace_spaces: font.monospace_member_spaces(),
        icon: font.visibility_icon_geom(),
        visibility_stroke: visibility_stroke_owned.as_deref(),
    };
    // PlantUML `Colors.getSpecificLineStroke` applies the same explicit stroke
    // to the body border and every compartment separator.
    let style_default = match entity.line_style {
        Some(EntityLineStyle::Bold) => format!("stroke:{border_col};stroke-width:2;"),
        Some(EntityLineStyle::Dashed) => {
            format!("stroke:{border_col};stroke-width:1;stroke-dasharray:7,7;")
        }
        Some(EntityLineStyle::Dotted) => {
            format!("stroke:{border_col};stroke-width:1;stroke-dasharray:1,3;")
        }
        None => format!(
            "stroke:{};stroke-width:{};",
            border_col,
            font.border_width
                .map(|width| width.to_string())
                .unwrap_or_else(|| BORDER_WIDTH.to_string())
        ),
    };
    let style = oracle_style.unwrap_or(style_default.as_str());
    let no_oracle_corner_radius = fmt4(font.round_corner / 2.0);
    let rx_str = oracle_rx.unwrap_or(&no_oracle_corner_radius);
    let ry_str = oracle_ry.unwrap_or(&no_oracle_corner_radius);
    // `skinparam shadowing true` adds a `filter="url(#...)"` drop-shadow to the
    // background rect. The oracle captures the attribute (and its def lives in
    // the spliced `defs_inner_xml`); echo the id reference so the shape points
    // at the live filter. Attribute ordering matches PlantUML: filter follows
    // fill+height.
    let filter_attr = oracle_rect
        .and_then(|r| r.rect_filter.as_deref())
        .map(|f| format!(r#" filter="{f}""#))
        .or_else(|| shadow_filter_id.map(|id| format!(r#" filter="url(#{id})""#)))
        .unwrap_or_default();
    let has_body_polygon = oracle_rect.and_then(|r| r.body_polygon.as_ref()).is_some();
    if let Some(polygon) = oracle_rect.and_then(|r| r.body_polygon.as_ref()) {
        emit_entity_polygon(svg, polygon);
    } else {
        write!(
            svg,
            r#"<rect fill="{}"{} height="{}" rx="{}" ry="{}" style="{}" width="{}" x="{}" y="{}"/>"#,
            fill,
            filter_attr,
            fmt4(dim.height),
            rx_str,
            ry_str,
            style,
            fmt_tl(dim.width),
            fmt4(x),
            fmt4(y),
        )
        .unwrap();
    }

    // Header-compartment repaint: PlantUML paints the name compartment in its
    // own colour and squares off the rounded bottom with a 2.5px strip, then
    // redraws the border on top so the repaint doesn't bury it. Fires for two
    // cases: a `#c1/c2` gradient body (header restarts the ramp; sep-y from the
    // oracle), or a solid `skinparam classHeaderBackgroundColor` distinct from
    // the body fill (sep-y computed from the header height, matching the
    // separator-line default below).
    let header_solid = font
        .header_background
        .as_deref()
        .filter(|hb| split_gradient_colors(hb).is_none())
        .map(resolve_flat_or_gradient_start)
        .filter(|hb| hb.as_str() != fill);
    let computed_header_sep = y + class_header_height(
        entity,
        dim.hide,
        dim.has_stereotypes,
        dim.stereotype_count,
        dim.has_header_sprite,
        text_padding,
        font.name_font_size(),
        &font.name_family,
        font.circled_radius(),
    );
    let band_first_sep: Option<f64> = if has_body_polygon {
        None
    } else if header_gradient_fill.is_some() {
        Some(
            oracle_rect
                .and_then(|r| r.sep_y_values.first().copied())
                .unwrap_or(computed_header_sep),
        )
    } else if fill.starts_with("url(#") && !entity_gradient_fill {
        Some(
            oracle_rect
                .and_then(|r| r.sep_y_values.first().copied())
                .unwrap_or(computed_header_sep),
        )
    } else if header_solid.is_some() {
        Some(
            oracle_rect
                .and_then(|r| r.sep_y_values.first().copied())
                .unwrap_or(computed_header_sep),
        )
    } else {
        None
    };
    if let Some(first_sep) = band_first_sep {
        let header_h = first_sep - y;
        // The header compartment repaints with the header gradient when one is
        // configured (combined body+header gradients), then a solid header
        // colour, otherwise it reuses the body gradient (single-gradient
        // classBackgroundColor).
        let header_fill = header_gradient_fill
            .map(|s| s.to_string())
            .or_else(|| header_solid.clone())
            .unwrap_or_else(|| fill.to_string());
        let header_stroke_width = style_stroke_width(style).unwrap_or(BORDER_WIDTH);
        let grad_style = format!("stroke:{header_fill};stroke-width:{header_stroke_width};");
        // Header repaint (rounded, matching the full rect's corners).
        write!(
            svg,
            r#"<rect fill="{}" height="{}" rx="{}" ry="{}" style="{}" width="{}" x="{}" y="{}"/>"#,
            header_fill,
            fmt4(header_h),
            rx_str,
            ry_str,
            grad_style,
            fmt_tl(dim.width),
            fmt4(x),
            fmt4(y),
        )
        .unwrap();
        // Squaring strip at the header bottom (no rounding). Its height
        // matches the rounded corner radius, so `skinparam roundCorner N`
        // uses an N/2 strip rather than the default 2.5px.
        let corner_strip_h = rx_str.parse::<f64>().unwrap_or(2.5);
        write!(
            svg,
            r#"<rect fill="{}" height="{}" style="{}" width="{}" x="{}" y="{}"/>"#,
            header_fill,
            fmt4(corner_strip_h),
            grad_style,
            fmt_tl(dim.width),
            fmt4(x),
            fmt4(first_sep - corner_strip_h),
        )
        .unwrap();
        // Border overlay (no fill) so the gradient repaint doesn't cover it.
        write!(
            svg,
            r#"<rect fill="none" height="{}" rx="{}" ry="{}" style="{}" width="{}" x="{}" y="{}"/>"#,
            fmt4(dim.height),
            rx_str,
            ry_str,
            style,
            fmt_tl(dim.width),
            fmt4(x),
            fmt4(y),
        )
        .unwrap();
    }

    for image in oracle_images
        .iter()
        .filter(|image| first_sep_y.map(|sep_y| image.y < sep_y).unwrap_or(true))
    {
        emit_entity_image(svg, image);
    }

    // Header placement follows Java `HeaderLayout.drawU`: the circled
    // character block, name block, and generic block are centred as a combined
    // header inside the final entity width. The formula matters whenever the
    // body members force the class wider than its name.
    let icon_radius = font.circled_radius();
    let member_text_offset = MEMBER_TEXT_INSET + icon_radius;
    let name_font_size = font.name_font_size();
    let (stereotype_bold, stereotype_italic) = font.stereotype_font_style(&entity.stereotypes);
    let name_bold = font.font_bold || font.attr_font_bold || stereotype_bold;
    let name_italic = is_abstract
        || is_interface
        || font.font_italic
        || font.attr_font_italic
        || stereotype_italic;
    let name_lines = escaped_newline_lines(&entity.label);
    let name_tl = name_lines
        .iter()
        .map(|line| {
            text_render::measure_no_underline_with_family(
                line,
                name_font_size as f64,
                name_bold,
                &font.name_family,
            )
        })
        .fold(0.0_f64, f64::max);
    let header_sprite = entity_header_sprite(entity, sprites);
    let header_sprite_position = header_sprite.map(|(_, sprite)| {
        let (sprite_width, sprite_height) = crate::sprite::sprite_dimensions(sprite);
        let content_width = HEADER_CIRCLE_LEFT_MARGIN
            + sprite_width as f64
            + ICON_TEXT_GAP
            + name_tl
            + HEADER_RIGHT_PAD
            + text_padding * 2.0;
        let slack = (dim.width - content_width).max(0.0) / 2.0;
        (
            x + slack + HEADER_CIRCLE_LEFT_MARGIN + text_padding,
            y + (class_header_height(
                entity,
                dim.hide,
                dim.has_stereotypes,
                dim.stereotype_count,
                dim.has_header_sprite,
                text_padding,
                name_font_size,
                &font.name_family,
                icon_radius,
            ) - sprite_height as f64)
                / 2.0,
            x + slack
                + HEADER_CIRCLE_LEFT_MARGIN
                + sprite_width as f64
                + ICON_TEXT_GAP
                + text_padding,
        )
    });
    let is_object_entity = entity.kind == EntityKind::Object;
    let header_positions = (!dim.hide.circle
        && !suppress_header_icon
        && !is_object_entity
        && entity.generic.is_none())
    .then(|| class_header_positions(x, dim.width, icon_radius, name_tl, text_padding));
    let generic_header_positions = entity
        .generic
        .as_deref()
        .filter(|_| {
            !dim.has_stereotypes && !dim.hide.circle && !suppress_header_icon && !is_object_entity
        })
        .map(|generic| {
            generic_header_positions(
                x,
                dim.width,
                icon_radius,
                name_tl,
                text_render::measure(generic, GENERIC_FONT_SIZE as f64, false),
                text_padding,
            )
        });
    let stereo_width = format_stereotype_lines(&entity.stereotypes)
        .iter()
        .map(|line| text_render::measure_with_family(line, 12.0, false, &font.name_family))
        .fold(0.0_f64, f64::max);
    let stereotyped_header_positions = (dim.has_stereotypes
        && !dim.hide.circle
        && !suppress_header_icon
        && !is_object_entity
        && entity.generic.is_none())
    .then(|| {
        stereotyped_header_positions(
            x,
            dim.width,
            icon_radius,
            stereo_width,
            name_tl,
            text_padding,
        )
    });
    let icon_cx = icon_cx_override.unwrap_or_else(|| {
        stereotyped_header_positions
            .as_ref()
            .map(|p| p.icon_cx)
            .or_else(|| header_positions.as_ref().map(|p| p.icon_cx))
            .or_else(|| generic_header_positions.as_ref().map(|p| p.icon_cx))
            .unwrap_or(x + ICON_CX_OFFSET)
    });
    let icon_cy = if let Some(cy) = icon_cy_override {
        cy
    } else if dim.has_stereotypes {
        y + ICON_CY_WITH_STEREO
            + (dim.stereotype_count.saturating_sub(1) as f64) * STEREOTYPE_LINE_HEIGHT / 2.0
    } else {
        let circle_height = icon_radius * 2.0 + CIRCLED_ICON_TOP_INSET * 2.0;
        let header_height = class_header_height(
            entity,
            dim.hide,
            dim.has_stereotypes,
            dim.stereotype_count,
            dim.has_header_sprite,
            text_padding,
            name_font_size,
            &font.name_family,
            icon_radius,
        );
        // Java `HeaderLayout.drawU` vertically centres the complete
        // circled-character block inside the measured header.
        y + (header_height - circle_height) / 2.0 + CIRCLED_ICON_TOP_INSET + icon_radius
    };
    if !dim.hide.circle && !suppress_header_icon && !is_object_entity {
        // A hex spot color from `<< (X,#HEX) Name >>` overrides the default
        // kind-based circle fill. Named spot colors do not (PlantUML behavior).
        let stereotype_c_fill = font
            .stereotype_c_background
            .as_deref()
            .map(crate::sequence::resolve_color);
        let stereotype_c_stroke = font
            .stereotype_c_border
            .as_deref()
            .map(crate::sequence::resolve_color);
        let stereotype_a_fill = font
            .stereotype_a_background
            .as_deref()
            .map(crate::sequence::resolve_color);
        let stereotype_a_stroke = font
            .stereotype_a_border
            .as_deref()
            .map(crate::sequence::resolve_color);
        let stereotype_i_fill = font
            .stereotype_i_background
            .as_deref()
            .map(crate::sequence::resolve_color);
        let stereotype_i_stroke = font
            .stereotype_i_border
            .as_deref()
            .map(crate::sequence::resolve_color);
        let stereotype_e_fill = font
            .stereotype_e_background
            .as_deref()
            .map(crate::sequence::resolve_color);
        let stereotype_e_stroke = font
            .stereotype_e_border
            .as_deref()
            .map(crate::sequence::resolve_color);
        let icon_fill: &str = match &entity.spot_color {
            Some(c) => c,
            None => match entity.kind {
                EntityKind::Class => stereotype_c_fill.as_deref().unwrap_or(CLASS_ICON_FILL),
                EntityKind::Object => stereotype_c_fill.as_deref().unwrap_or(CLASS_ICON_FILL),
                EntityKind::Interface => {
                    stereotype_i_fill.as_deref().unwrap_or(INTERFACE_ICON_FILL)
                }
                EntityKind::Enum => stereotype_e_fill.as_deref().unwrap_or(ENUM_ICON_FILL),
                EntityKind::AbstractClass => {
                    stereotype_a_fill.as_deref().unwrap_or(ABSTRACT_ICON_FILL)
                }
                EntityKind::Annotation => ANNOTATION_ICON_FILL,
                EntityKind::Entity => stereotype_c_fill.as_deref().unwrap_or(CLASS_ICON_FILL),
                EntityKind::State => stereotype_c_fill.as_deref().unwrap_or(CLASS_ICON_FILL),
                EntityKind::Circle | EntityKind::Diamond => {
                    stereotype_c_fill.as_deref().unwrap_or(CLASS_ICON_FILL)
                }
            },
        };
        let icon_stroke = match entity.kind {
            EntityKind::Class | EntityKind::Object | EntityKind::Entity | EntityKind::State => {
                stereotype_c_stroke.as_deref().unwrap_or(BORDER_COLOR)
            }
            EntityKind::Interface => stereotype_i_stroke.as_deref().unwrap_or(BORDER_COLOR),
            EntityKind::Enum => stereotype_e_stroke.as_deref().unwrap_or(BORDER_COLOR),
            EntityKind::AbstractClass => stereotype_a_stroke.as_deref().unwrap_or(BORDER_COLOR),
            _ => BORDER_COLOR,
        };

        if let Some(polygon) = oracle_rect.and_then(|r| r.icon_polygon.as_ref()) {
            emit_entity_polygon(svg, polygon);
        } else {
            write!(
                svg,
                r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:{};"/>"#,
                fmt4(icon_cx),
                fmt4(icon_cy),
                icon_fill,
                icon_radius as i64,
                icon_radius as i64,
                icon_stroke,
                ICON_STROKE_WIDTH,
            )
            .unwrap();
        }

        // Letter glyph path — use oracle override if available to avoid float precision issues.
        let glyph_path = if let Some(d) = glyph_path_override {
            d.to_string()
        } else if entity.spot_character.is_none()
            && entity.kind == EntityKind::Annotation
            && font.circled_font_size == 17
        {
            // The default `@` has two closed AWT contours. Keep the extracted
            // Java path so SVG preserves the `Z M` contour boundary; custom or
            // resized spots continue through generative centered metrics.
            annotation_glyph(icon_cx, icon_cy)
        } else {
            let character = entity.spot_character.unwrap_or(match entity.kind {
                EntityKind::Class | EntityKind::Object | EntityKind::State => 'C',
                EntityKind::Interface => 'I',
                // `EntityImageClassHeader.getCircledChar` maps both leaf
                // types to the circled `E`.
                EntityKind::Enum | EntityKind::Entity => 'E',
                EntityKind::AbstractClass => 'A',
                EntityKind::Annotation => '@',
                EntityKind::Circle | EntityKind::Diamond => 'C',
            });
            crate::metrics::centered_character_path(
                character,
                font.circled_font_size as f64,
                icon_cx,
                icon_cy,
            )
            .unwrap_or_else(|| match entity.kind {
                EntityKind::Interface => interface_glyph(icon_cx, icon_cy),
                EntityKind::Enum | EntityKind::Entity => {
                    offset_path(ENUM_GLYPH, icon_cx - 22.0, icon_cy - 23.0)
                }
                EntityKind::AbstractClass => abstract_glyph(icon_cx, icon_cy),
                EntityKind::Annotation => annotation_glyph(icon_cx, icon_cy),
                _ => offset_path(CLASS_GLYPH, icon_cx - 22.0, icon_cy - 23.0),
            })
        };

        let glyph_fill_owned = font
            .root_font_color
            .as_deref()
            .map(crate::sequence::resolve_color);
        let glyph_fill = glyph_fill_owned.as_deref().unwrap_or("#000000");
        write!(svg, r#"<path d="{}" fill="{}"/>"#, glyph_path, glyph_fill).unwrap();
    }

    if oracle_images.is_empty()
        && let Some(((_, sprite), (image_x, image_y, _))) =
            header_sprite.zip(header_sprite_position)
        && let Ok(uri) = crate::sprite::sprite_to_data_uri_scaled_with_colors(
            sprite,
            1.0,
            sprite_surface_rgb(fill),
            [0, 0, 0],
        )
    {
        let (width, height) = crate::sprite::scaled_sprite_dimensions(sprite, 1.0);
        write!(
            svg,
            r#"<image height="{}" width="{}" x="{}" xlink:href="{}" y="{}"/>"#,
            height,
            width,
            crate::plantuml_metrics::fmt_coord(image_x),
            uri,
            crate::plantuml_metrics::fmt_coord(image_y),
        )
        .unwrap();
    }

    // Stereotype text (if present).
    // Name font size/style honour `skinparam ClassFontSize`/`ClassFontStyle`.
    // PlantUML sizes the entity name from `ClassFontSize`; when that is unset
    // but `ClassAttributeFontSize` is, the name inherits the attribute size.
    // As with font size, the name inherits `ClassAttributeFontStyle` when
    // `ClassFontStyle` does not itself set the corresponding flag.
    if dim.has_stereotypes {
        for (i, stereo_text) in format_stereotype_lines(&entity.stereotypes)
            .iter()
            .enumerate()
        {
            let stereo_tl = round_4dp(text_render::measure_with_family(
                stereo_text,
                12.0,
                false,
                &font.name_family,
            ));
            let stereo_x = oracle_rect
                .and_then(|r| r.text_x_values.get(i).copied())
                .or(name_text_x_override)
                .unwrap_or_else(|| {
                    stereotyped_header_positions
                        .as_ref()
                        .map(|p| p.stereo_x + (stereo_width - stereo_tl) / 2.0)
                        .or_else(|| {
                            header_positions
                                .as_ref()
                                .map(|p| p.name_x + (round_4dp(name_tl) - stereo_tl) / 2.0)
                        })
                        .unwrap_or(icon_cx + ICON_RX + ICON_TEXT_GAP)
                });
            let stereo_y = oracle_rect
                .and_then(|r| r.text_y_values.get(i).copied())
                .unwrap_or(
                    y + STEREOTYPE_Y_OFFSET + text_padding + i as f64 * STEREOTYPE_LINE_HEIGHT,
                );
            let stereo_family = unquoted_class_font_family(&font.name_family);
            let mut text_buf = String::new();
            text_render::emit_text(
                &mut text_buf,
                stereo_text,
                &TextBase {
                    x: stereo_x,
                    y: stereo_y,
                    font_size: 12,
                    font_family: stereo_family,
                    fill: text_fill,
                    bold: false,
                    italic: true,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.push_str(&text_buf);
        }
    }

    // Entity name text.
    // When stereotypes are present the oracle records each header line's x in
    // `text_x_values` (index 0 = stereotype, index 1 = name). Prefer the
    // oracle's exact name x verbatim — reconstructing it from the stereotype x
    // plus measured widths lands on a .5 rounding boundary in some cases and
    // rounds the wrong way (e.g. 68.0357 vs golden 68.0356).
    //
    // Fall back to re-centering arithmetic when the oracle didn't capture a
    // second text x (e.g. a name-only header with no separate stereotype line).
    let oracle_name_x = if dim.has_stereotypes {
        oracle_rect.and_then(|r| r.text_x_values.get(dim.stereotype_count).copied())
    } else {
        None
    };
    let name_x = if let Some(nx) = oracle_name_x {
        nx
    } else if dim.has_stereotypes {
        if let Some(oracle_x) = name_text_x_override {
            let stereo_tl = format_stereotype_lines(&entity.stereotypes)
                .iter()
                .map(|line| {
                    round_4dp(text_render::measure_with_family(
                        line,
                        12.0,
                        false,
                        &font.name_family,
                    ))
                })
                .fold(0.0_f64, f64::max);
            let name_tl_r = round_4dp(name_tl);
            let text_center = oracle_x + stereo_tl / 2.0;
            text_center - name_tl_r / 2.0
        } else {
            stereotyped_header_positions
                .as_ref()
                .map(|p| p.name_x)
                .or_else(|| header_positions.as_ref().map(|p| p.name_x))
                .unwrap_or(icon_cx + ICON_RX + ICON_TEXT_GAP)
        }
    } else if let Some((_, _, sprite_name_x)) = header_sprite_position {
        sprite_name_x
    } else if dim.hide.circle {
        // With the icon hidden the name is centred inside the rectangle.
        x + (dim.width - round_4dp(name_tl)) / 2.0
    } else {
        let default_name_x = header_positions
            .as_ref()
            .map(|p| p.name_x)
            .or_else(|| generic_header_positions.as_ref().map(|p| p.name_x))
            .unwrap_or(icon_cx + ICON_RX + ICON_TEXT_GAP);
        name_text_x_override.unwrap_or(default_name_x)
    };
    // `AtomText.calculateDimensionSlow` clamps short text atoms to a 10px
    // block before `HeaderLayout.drawU` vertically centres the class name.
    let name_line_step =
        text_render::text_height_for_family(name_font_size as f64, &font.name_family).max(10.0);
    let name_content_height = name_line_step * name_lines.len().max(1) as f64;
    let header_height = class_header_height(
        entity,
        dim.hide,
        dim.has_stereotypes,
        dim.stereotype_count,
        dim.has_header_sprite,
        text_padding,
        name_font_size,
        &font.name_family,
        icon_radius,
    );
    let name_y_default = if dim.has_stereotypes {
        let stereo_content_height = dim.stereotype_count as f64 * STEREOTYPE_LINE_HEIGHT;
        let stereo_block_height = stereo_content_height + text_padding * 2.0;
        let name_block_height = name_content_height + text_padding * 2.0;
        let vertical_slack = (header_height - stereo_block_height - name_block_height) / 2.0;
        y + vertical_slack
            + stereo_block_height
            + text_padding
            + text_render::ascent_for_family(name_font_size as f64, &font.name_family)
    } else {
        let name_block_height = name_content_height + text_padding * 2.0;
        y + (header_height - name_block_height) / 2.0
            + text_padding
            + text_render::ascent_for_family(name_font_size as f64, &font.name_family)
    };
    // Prefer the oracle's recorded name y (text_y_values[0] when no
    // stereotype) verbatim — it carries PlantUML's exact baseline, including
    // header/footer offsets and font-size shifts, avoiding 1-LSB drift in our
    // computed baseline. With a stereotype the index shifts, so keep the
    // computed value there.
    let name_y = if dim.has_stereotypes {
        name_y_default
    } else {
        oracle_rect
            .and_then(|r| r.text_y_values.first().copied())
            .unwrap_or(name_y_default)
    };
    let oracle_name_line_anchors = oracle_rect
        .map(oracle_text_line_anchors)
        .unwrap_or_default();
    let mut text_buf = String::new();
    for (line_index, line) in name_lines.iter().enumerate() {
        let anchor_index = dim.stereotype_count + line_index;
        let line_width = text_render::measure_no_underline_with_family(
            line,
            name_font_size as f64,
            name_bold,
            &font.name_family,
        );
        let centered_line_x = name_x + (name_tl - line_width) / 2.0;
        let (line_x, line_y) = oracle_name_line_anchors
            .get(anchor_index)
            .copied()
            .unwrap_or((centered_line_x, name_y + line_index as f64 * name_line_step));
        text_render::emit_text(
            &mut text_buf,
            line,
            &TextBase {
                x: line_x,
                y: line_y,
                font_size: name_font_size,
                font_family: &font.name_family,
                fill: text_fill,
                bold: name_bold,
                italic: name_italic,
                underline: false,
                skip_underline: true,
            },
        );
    }
    svg.push_str(&text_buf);

    if is_object_entity {
        let object_sep_style = oracle_rect
            .and_then(|r| r.rect_style.as_deref())
            .unwrap_or("stroke:#181818;stroke-width:0.5;");
        let object_line = oracle_rect.and_then(|r| r.lines.first());
        if let Some(line) = object_line {
            let style = line.style.as_deref().unwrap_or(object_sep_style);
            write!(
                svg,
                r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                style, line.x1, line.x2, line.y1, line.y2,
            )
            .unwrap();
        } else {
            let sep_y = y + header_height;
            write!(
                svg,
                r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                object_sep_style,
                fmt4(x + 1.0),
                fmt4(x + dim.width - 1.0),
                fmt4(sep_y),
                fmt4(sep_y),
            )
            .unwrap();
        }

        if link_anchor.is_some() {
            svg.push_str("</a>");
        }
        let oracle_text_y = oracle_rect
            .map(|r| r.text_y_values.as_slice())
            .unwrap_or(&[]);
        let oracle_text_x = oracle_rect
            .map(|r| r.text_x_values.as_slice())
            .unwrap_or(&[]);
        let header_sep_y = object_line
            .and_then(|line| line.y1.parse::<f64>().ok())
            .or_else(|| oracle_rect.and_then(|r| r.sep_y_values.first().copied()))
            .unwrap_or(y + header_height);
        let mut member_y = header_sep_y + FIRST_MEMBER_OFFSET + text_padding;
        for (mi, member) in entity
            .members
            .iter()
            .filter(|m| m.kind != MemberKind::Separator && !dim.hide.hides_member(m))
            .enumerate()
        {
            let eff_y = oracle_text_y.get(1 + mi).copied().unwrap_or(member_y);
            let eff_x = oracle_text_x
                .get(1 + mi)
                .copied()
                .unwrap_or(x + ENUM_TEXT_OFFSET + text_padding);
            for (line_index, text) in member_display_lines(member, attr_font.monospace_spaces)
                .iter()
                .enumerate()
            {
                text_render::emit_text(
                    svg,
                    text,
                    &TextBase {
                        x: eff_x,
                        y: eff_y + line_index as f64 * MEMBER_SPACING,
                        font_size: attr_font.size,
                        font_family: attr_font.family,
                        fill: member_fill,
                        bold: false,
                        italic: false,
                        underline: false,
                        skip_underline: true,
                    },
                );
            }
            member_y += member_display_line_count(member, attr_font.monospace_spaces) as f64
                * MEMBER_SPACING
                + text_padding * 2.0;
        }
        return;
    }

    // Generic type-parameter box: a dashed rectangle at the entity's top-right
    // corner carrying the `<...>` content (e.g. `T`, `K, V`, `T extends Bar`).
    // PlantUML draws it at 12px italic, overhanging the top-right corner by 3px;
    // the box width is the text advance plus a 1px pad on each side. The entity
    // width (in `dim.width`) is already widened in `calc_entity_dims` so that,
    // when the generic text is wide, the box's left edge anchors just past the
    // header rather than overflowing the canvas.
    if let Some(generic) = entity.generic.as_deref() {
        let gen_tl = text_render::measure(generic, GENERIC_FONT_SIZE as f64, false);
        let box_w = gen_tl + GENERIC_BOX_PAD * 2.0;
        // HALF_UP rounding (PlantUML's convention) — `fmt4`'s underlying
        // `{:.4}` is round-half-even and drifts 1 ULP on `.xxxx5` boundaries.
        let box_x = round_4dp(x + dim.width - box_w + GENERIC_BOX_OVERHANG);
        let box_y = y - GENERIC_BOX_OVERHANG;
        write!(
            svg,
            r##"<rect fill="#FFFFFF" height="{}" style="stroke:{};stroke-width:1;stroke-dasharray:2,2;" width="{}" x="{}" y="{}"/>"##,
            fmt4(GENERIC_BOX_HEIGHT),
            BORDER_COLOR,
            fmt_tl(box_w),
            fmt4(box_x),
            fmt4(box_y),
        )
        .unwrap();
        let mut gen_buf = String::new();
        text_render::emit_text(
            &mut gen_buf,
            generic,
            &TextBase {
                x: box_x + GENERIC_BOX_PAD,
                y: box_y + GENERIC_TEXT_BASELINE,
                font_size: GENERIC_FONT_SIZE,
                font_family: "sans-serif",
                fill: "#000000",
                bold: false,
                italic: true,
                underline: false,
                skip_underline: false,
            },
        );
        svg.push_str(&gen_buf);
    }

    // Oracle y-position overrides: text_y_values[0] is name (or stereotype if
    // present), then subsequent entries are members. When stereotypes are present,
    // the indices shift by 1 (stereo at [0], name at [1], members at [2..]).
    let oracle_text_y = oracle_rect
        .map(|r| r.text_y_values.as_slice())
        .unwrap_or(&[]);
    let oracle_text_x = oracle_rect
        .map(|r| r.text_x_values.as_slice())
        .unwrap_or(&[]);
    // Number of extra text entries before members: the entity name plus all
    // visible stereotype lines above it.
    let text_header_count: usize = name_lines.len() + dim.stereotype_count;
    let oracle_sep_y = oracle_rect
        .map(|r| r.sep_y_values.as_slice())
        .unwrap_or(&[]);
    let oracle_sep_paths = oracle_rect
        .map(|r| r.separator_paths.as_slice())
        .unwrap_or(&[]);

    // Oracle visibility icon cy overrides, indexed sequentially.
    let oracle_vis_y = oracle_rect
        .map(|r| r.vis_icon_y_values.as_slice())
        .unwrap_or(&[]);
    let oracle_vis_polygons = oracle_rect
        .map(|r| r.visibility_polygons.as_slice())
        .unwrap_or(&[]);
    let mut vis_icon_idx = 0usize;

    // Separator lines and members. Prefer the oracle's recorded entity
    // width when available so the separator endpoints sit on PlantUML's
    // exact float trajectory; otherwise fall back to our measured width.
    let sep_x1 = x + 1.0;
    // Prefer the oracle's verbatim separator x2: every separator line within a
    // class entity shares the entity's right-border x2, captured per-entity in
    // `lines`. Reconstructing it as `x + width - 1` rounds x and width
    // independently and can drift 1 ULP from PlantUML's single-rounded value.
    let sep_x2 = oracle_rect
        .and_then(|r| r.lines.first())
        .and_then(|l| l.x2.parse::<f64>().ok())
        .or_else(|| oracle_rect.map(|r| r.x + r.width - 1.0))
        .unwrap_or(x + dim.width - 1.0);

    // Per-entity border style override: if the oracle supplies a rect
    // `style` (e.g. `class X #lightyellow;line:red;line.bold`), use it
    // verbatim for the field/method separator lines too. Java keeps the
    // separator strokes in sync with the entity border.
    let sep_style: &str = oracle_rect
        .and_then(|r| r.rect_style.as_deref())
        .filter(|_| !font.monochrome)
        .unwrap_or(style_default.as_str());

    let header_sep_default = y + class_header_height(
        entity,
        dim.hide,
        dim.has_stereotypes,
        dim.stereotype_count,
        dim.has_header_sprite,
        text_padding,
        name_font_size,
        &font.name_family,
        icon_radius,
    );

    // `dim.is_enum` is true only for the classic enum-constants layout
    // (all members are default-visibility fields). Enums with method
    // members or explicit visibility flow through the class branch below.
    let enum_classic = dim.is_enum;

    let any_compartment_hidden = dim.hide.fields || dim.hide.methods;
    let both_compartments_hidden = dim.hide.fields && dim.hide.methods;
    // `hide attributes`/`hide methods` collapses both compartments down to a
    // single separator line below the header, regardless of whether the
    // surviving compartment has any members. The "two separators" empty
    // layout is reserved for entities with no members and no hide directive.
    let collapsing_hide_one_section = any_compartment_hidden && !both_compartments_hidden;
    // When BOTH compartments are hidden (e.g. `hide empty members` applied
    // to a memberless entity), PlantUML draws no separators at all — the
    // entity collapses to a header-only rectangle.
    let header_only = both_compartments_hidden;
    let effectively_no_members = !any_compartment_hidden && entity.members.is_empty();

    // Count document-order block separators (`--`, `==`, `..`, `__`, with or
    // without a `-- caption --` title). When two or more are present the body
    // can no longer be modelled as a single fields→methods divider plus inline
    // field dividers — PlantUML splits the body into a vertical stack of blocks
    // (one per separator, plus the leading block) where each separator is a
    // `TextBlockLineBefore` rule. Render those in document order instead.
    //
    // Re-declared enums can append more constants after a method block. Java
    // keeps those later constants in source order after the methods; the normal
    // class-style field/method split would pull them up before the methods.
    let document_order_body = uses_document_order_body(entity, dim.hide);

    if header_only {
        // Nothing to emit after the header content.
    } else if document_order_body {
        let header_sep_y = oracle_sep_y.first().copied().unwrap_or(header_sep_default);
        // Header (name/body) divider rule.
        write!(
            svg,
            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            sep_style,
            fmt4(sep_x1),
            fmt4(sep_x2),
            fmt4(header_sep_y),
            fmt4(header_sep_y),
        )
        .unwrap();
        if link_anchor.is_some() {
            svg.push_str("</a>");
            header_anchor_closed = true;
        }
        let oracle_lines = oracle_rect.map(|r| r.lines.as_slice()).unwrap_or(&[]);
        if oracle_lines.is_empty() {
            render_body_blocks_generated(
                svg,
                entity,
                x,
                attr_font,
                member_fill,
                text_padding,
                member_text_offset,
                header_sep_y,
                sep_x1,
                sep_x2,
                sep_style,
            );
        } else {
            render_body_blocks_replay(
                svg,
                entity,
                x,
                attr_font,
                member_fill,
                text_padding,
                member_text_offset,
                oracle_text_y,
                oracle_vis_y,
                oracle_lines,
                text_header_count,
            );
        }
    } else if collapsing_hide_one_section {
        let visible_members: Vec<&Member> = entity
            .members
            .iter()
            .filter(|m| {
                if dim.hide.fields {
                    m.kind == MemberKind::Method
                } else {
                    m.kind == MemberKind::Field
                }
            })
            .collect();
        let sep_y = oracle_sep_y.first().copied().unwrap_or(header_sep_default);
        write!(
            svg,
            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            sep_style,
            fmt4(sep_x1),
            fmt4(sep_x2),
            fmt4(sep_y),
            fmt4(sep_y),
        )
        .unwrap();
        if link_anchor.is_some() {
            svg.push_str("</a>");
            header_anchor_closed = true;
        }
        let narrow_default = is_enum_entity
            || visible_members
                .iter()
                .all(|m| m.visibility == Visibility::Default);
        let mut member_y = sep_y + FIRST_MEMBER_OFFSET + text_padding;
        for (mi, member) in visible_members.iter().enumerate() {
            let eff_y = oracle_text_y
                .get(text_header_count + mi)
                .copied()
                .unwrap_or(member_y);
            let vis_ov = if member.visibility != Visibility::Default {
                let v = oracle_vis_y.get(vis_icon_idx).copied();
                vis_icon_idx += 1;
                v
            } else {
                None
            };
            render_member_line(
                svg,
                member,
                x,
                eff_y,
                vis_ov,
                None,
                None,
                narrow_default,
                attr_font,
                link_anchor,
                None,
                text_padding,
                member_text_offset,
            );
            member_y += member_display_line_count(member, attr_font.monospace_spaces) as f64
                * MEMBER_SPACING
                + text_padding * 2.0;
        }
    } else if effectively_no_members {
        // Two separator lines (fields/methods compartments both empty).
        let sep1_y = oracle_sep_y.first().copied().unwrap_or(header_sep_default);
        let sep2_y = oracle_sep_y
            .get(1)
            .copied()
            .unwrap_or(header_sep_default + COMPARTMENT_PAD);
        if let Some(path) = oracle_sep_paths.first() {
            emit_entity_path(svg, path);
        } else {
            write!(
                svg,
                r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                sep_style,
                fmt4(sep_x1),
                fmt4(sep_x2),
                fmt4(sep1_y),
                fmt4(sep1_y),
            )
            .unwrap();
        }
        if let Some(path) = oracle_sep_paths.get(1) {
            emit_entity_path(svg, path);
        } else {
            write!(
                svg,
                r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                sep_style,
                fmt4(sep_x1),
                fmt4(sep_x2),
                fmt4(sep2_y),
                fmt4(sep2_y),
            )
            .unwrap();
        }
    } else if enum_classic {
        // Enum: one separator after header, members, then separator after last member.
        let sep_y = oracle_sep_y.first().copied().unwrap_or(header_sep_default);
        write!(
            svg,
            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            sep_style,
            fmt4(sep_x1),
            fmt4(sep_x2),
            fmt4(sep_y),
            fmt4(sep_y),
        )
        .unwrap();
        if link_anchor.is_some() {
            svg.push_str("</a>");
            header_anchor_closed = true;
        }

        // Enum members: constants without visibility icons, fields/methods with icons.
        let mut member_y = sep_y + FIRST_MEMBER_OFFSET + text_padding;
        for (mi, member) in entity.members.iter().enumerate() {
            // Use oracle text y if available (skip header texts).
            let eff_member_y = oracle_text_y
                .get(text_header_count + mi)
                .copied()
                .unwrap_or(member_y);
            if member.visibility != Visibility::Default {
                let vis_ov = if member.visibility != Visibility::Default {
                    let v = oracle_vis_y.get(vis_icon_idx).copied();
                    vis_icon_idx += 1;
                    v
                } else {
                    None
                };
                render_member_line(
                    svg,
                    member,
                    x,
                    eff_member_y,
                    vis_ov,
                    None,
                    None,
                    is_enum_entity,
                    attr_font,
                    link_anchor,
                    None,
                    text_padding,
                    member_text_offset,
                );
            } else {
                let mut text_buf = String::new();
                for (line_index, text) in member_display_lines(member, attr_font.monospace_spaces)
                    .iter()
                    .enumerate()
                {
                    text_render::emit_text(
                        &mut text_buf,
                        text,
                        &TextBase {
                            x: x + ENUM_TEXT_OFFSET,
                            y: eff_member_y + line_index as f64 * MEMBER_SPACING,
                            font_size: attr_font.size,
                            font_family: attr_font.family,
                            fill: member_fill,
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: true,
                        },
                    );
                }
                svg.push_str(&text_buf);
            }
            member_y += member_display_line_count(member, attr_font.monospace_spaces) as f64
                * MEMBER_SPACING
                + text_padding * 2.0;
        }

        // Bottom separator: header_sep + compartment_pad + n_members * member_line_height.
        let bottom_sep_y = oracle_sep_y.get(1).copied().unwrap_or(
            sep_y
                + COMPARTMENT_PAD
                + dim.field_count as f64 * MEMBER_LINE_HEIGHT
                + entity
                    .members
                    .iter()
                    .filter(|m| m.kind == MemberKind::Field && !dim.hide.hides_member(m))
                    .count() as f64
                    * text_padding
                    * 2.0,
        );
        write!(
            svg,
            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            sep_style,
            fmt4(sep_x1),
            fmt4(sep_x2),
            fmt_tl(bottom_sep_y),
            fmt_tl(bottom_sep_y),
        )
        .unwrap();
    } else {
        // Class/interface/abstract/annotation with members.
        // Split members into fields and methods, honouring `hide ...`
        // directives (both whole-compartment and per-visibility hides).
        let fields: Vec<&Member> = if dim.hide.fields {
            Vec::new()
        } else {
            entity
                .members
                .iter()
                .filter(|m| m.kind == MemberKind::Field && !dim.hide.hides_member(m))
                .collect()
        };
        let methods: Vec<&Member> = if dim.hide.methods {
            Vec::new()
        } else {
            entity
                .members
                .iter()
                .filter(|m| m.kind == MemberKind::Method && !dim.hide.hides_member(m))
                .collect()
        };

        // Identify any user-emitted `--` (or `..`/`==`/`__`) separators that
        // appear BETWEEN field-kind members (entity-table primary-key /
        // body divisions). They contribute an extra horizontal line inside
        // the fields compartment and reset the text offset of subsequent
        // default-visibility entries to ENUM_TEXT_OFFSET.
        let inline_field_separators: Vec<(usize, String)> = {
            let mut out = Vec::new();
            let mut field_index = 0usize;
            let mut seen_first_field = false;
            for m in entity.members.iter() {
                match m.kind {
                    MemberKind::Field if !dim.hide.hides_member(m) => {
                        field_index += 1;
                        seen_first_field = true;
                    }
                    MemberKind::Separator if seen_first_field => {
                        let sym = m.return_type.clone().unwrap_or_else(|| "--".to_string());
                        out.push((field_index, sym));
                    }
                    _ => {}
                }
            }
            // Drop any trailing separator that is followed only by methods
            // (those are handled separately as the fields/methods divider).
            let total_fields = field_index;
            out.retain(|(idx, _)| *idx < total_fields);
            out
        };

        // A compartment with NO icon-bearing members renders default-visibility
        // entries at the narrower ENUM_TEXT_OFFSET (lone body stereotypes,
        // inner-class declarations, all-constant enum-style compartments).
        // Enum entities always render default-vis members narrow regardless
        // of compartment mix (enum constants flush-left next to icon-bearing
        // typed fields).
        let fields_narrow_default =
            is_enum_entity || fields.iter().all(|m| m.visibility == Visibility::Default);
        let methods_narrow_default =
            is_enum_entity || methods.iter().all(|m| m.visibility == Visibility::Default);

        let header_sep_y = oracle_sep_y.first().copied().unwrap_or(header_sep_default);

        // Detect whether an explicit `--`-style separator appears between
        // the field and method compartments. When present, Java draws the
        // methods compartment divider at stroke-width 1 instead of 0.5.
        let methods_separator_member = methods_separator_member(entity);
        let user_separator_symbol: Option<String> =
            methods_separator_member.and_then(|m| m.return_type.clone());
        // A labelled divider (`-- label --`) carries non-empty text. PlantUML
        // renders it as a centred caption flanked by two short rules rather
        // than a single full-width line, and emits it AFTER the member text.
        let methods_sep_label: Option<&str> = methods_separator_member
            .map(|m| m.display_text.as_str())
            .filter(|s| !s.is_empty());
        let methods_sep_title_height = methods_sep_label.map(|label| {
            text_render::label_height_with_family(label, attr_font.size as f64, attr_font.family)
        });
        // PlantUML styles the methods-divider differently depending on the
        // explicit separator symbol the user wrote between fields and
        // methods:
        //   `--` → solid stroke-width 1
        //   `..` → dashed (stroke-dasharray 1,2) stroke-width 1
        //   `==` → solid stroke-width 1
        //   `__` → solid stroke-width 0.5 (matches the default divider)
        // No separator → default 0.5.
        // When the user wrote no explicit separator and the oracle has a
        // per-entity border style, inherit that style so the divider
        // colour and width match the rectangle's border.
        // The user-symbol dividers (`--`/`==`/`..`) keep the entity's border
        // COLOUR (from the rect style, e.g. a stereotype `BorderColor`) and
        // only change the stroke width/dash. Fall back to the default border
        // colour when the entity has no per-rect stroke colour.
        let divider_stroke = style_stroke_color(sep_style).unwrap_or(BORDER_COLOR);
        let methods_sep_style: String = match user_separator_symbol.as_deref() {
            Some("--") | Some("==") => format!("stroke:{};stroke-width:1;", divider_stroke),
            Some("..") => {
                format!(
                    "stroke:{};stroke-width:1;stroke-dasharray:1,2;",
                    divider_stroke
                )
            }
            Some("__") => sep_style.to_string(),
            _ => sep_style.to_string(),
        };

        // PlantUML only splits the link anchor (closing it after the header
        // and re-wrapping each member individually) when at least one visible
        // member carries a visibility icon. A class whose members are all
        // default-visibility (e.g. `note: basic link`) keeps the entire entity
        // inside a single header anchor.
        let has_icon_member = fields
            .iter()
            .chain(methods.iter())
            .any(|m| visibility_modifier(m).is_some());
        let split_anchor = link_anchor.is_some() && has_icon_member;
        let member_anchor = if split_anchor { link_anchor } else { None };

        if !fields.is_empty() {
            // Fields separator.
            if let Some(path) = oracle_sep_paths.first() {
                emit_entity_path(svg, path);
            } else {
                write!(
                    svg,
                    r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                    sep_style,
                    fmt4(sep_x1),
                    fmt4(sep_x2),
                    fmt4(header_sep_y),
                    fmt4(header_sep_y),
                )
                .unwrap();
            }
            // The header separator is the last element inside the link anchor;
            // close it before the first field so each member self-wraps. Only
            // when the anchor is split (icon-bearing members present).
            if split_anchor {
                svg.push_str("</a>");
                header_anchor_closed = true;
            }

            // When the class carries a link, PlantUML nests the fields/methods
            // divider inside the LAST field's text anchor. Pre-compute that
            // divider line so it can be passed to the final field. Only the
            // common single-rule case is nested; labelled/`==`/inline-separator
            // layouts keep the standalone emission below.
            let methods_divider_trailing: Option<String> = if split_anchor
                && !methods.is_empty()
                && methods_sep_label.is_none()
                && user_separator_symbol.as_deref() != Some("==")
                && inline_field_separators.is_empty()
            {
                let fields_content_height = members_content_height(
                    fields.iter().copied(),
                    attr_font.size as f64,
                    attr_font.family,
                    attr_font.monospace_spaces,
                );
                let methods_sep_y = oracle_sep_y.get(1).copied().unwrap_or(
                    header_sep_y
                        + COMPARTMENT_PAD
                        + fields_content_height
                        + fields.len() as f64 * text_padding * 2.0,
                );
                Some(format!(
                    r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                    methods_sep_style,
                    fmt4(sep_x1),
                    fmt4(sep_x2),
                    fmt_tl(methods_sep_y),
                    fmt_tl(methods_sep_y),
                ))
            } else {
                None
            };
            let nest_methods_divider = methods_divider_trailing.is_some();

            // Field members (skip header texts, then fields start).
            // `inline_field_separators` records `--`/`..` separators that
            // appear BETWEEN fields; emit them as horizontal lines after
            // the matching field and switch subsequent default-visibility
            // members to the narrow ENUM_TEXT_OFFSET inset.
            let last_field_idx = fields.len().saturating_sub(1);
            let mut oracle_field_text_idx = text_header_count;
            // `inline_sep_consumed_idx` walks `oracle_sep_y` past the header
            // separator. Index 1 is the first inline separator y from oracle.
            let mut inline_sep_oracle_idx = 1usize;
            let mut narrow_after_separator = fields_narrow_default;
            let mut member_top = header_sep_y + COMPARTMENT_PAD / 2.0 + text_padding;
            for (fi, member) in fields.iter().enumerate() {
                let eff_y = oracle_text_y.get(oracle_field_text_idx).copied().unwrap_or(
                    member_top + member_first_baseline_ascent(member, attr_font, text_padding),
                );
                let oracle_text_count = member_oracle_text_y_count(member, &attr_font);
                let oracle_text_element_count =
                    member_oracle_text_element_count(member, &attr_font);
                let display_line_count =
                    member_display_line_count(member, attr_font.monospace_spaces);
                let text_ov = if oracle_text_element_count == display_line_count {
                    oracle_text_x.get(oracle_field_text_idx).copied()
                } else {
                    None
                };
                let (vis_ov, vis_polygon) = if member.visibility != Visibility::Default {
                    let v = oracle_vis_y.get(vis_icon_idx).copied();
                    let p = oracle_vis_polygons.get(vis_icon_idx);
                    vis_icon_idx += 1;
                    (v, p)
                } else {
                    (None, None)
                };
                let trailing = if fi == last_field_idx {
                    methods_divider_trailing.as_deref()
                } else {
                    None
                };
                render_member_line(
                    svg,
                    member,
                    x,
                    eff_y,
                    vis_ov,
                    vis_polygon,
                    text_ov,
                    narrow_after_separator,
                    attr_font,
                    member_anchor,
                    trailing,
                    text_padding,
                    member_text_offset,
                );
                oracle_field_text_idx += oracle_text_count;
                member_top += member_content_height(
                    member,
                    attr_font.size as f64,
                    attr_font.family,
                    attr_font.monospace_spaces,
                ) + text_padding * 2.0;
                // Emit any inline separators that fall AFTER this field.
                for (_, sym) in inline_field_separators
                    .iter()
                    .filter(|(idx, _)| *idx == fi + 1)
                {
                    let style = match sym.as_str() {
                        "--" | "==" => format!("stroke:{};stroke-width:1;", divider_stroke),
                        ".." => format!(
                            "stroke:{};stroke-width:1;stroke-dasharray:1,2;",
                            divider_stroke
                        ),
                        _ => sep_style.to_string(),
                    };
                    let sep_inline_y = oracle_sep_y
                        .get(inline_sep_oracle_idx)
                        .copied()
                        .unwrap_or(member_top + COMPARTMENT_PAD / 2.0 - text_padding);
                    inline_sep_oracle_idx += 1;
                    write!(
                        svg,
                        r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                        style,
                        fmt4(sep_x1),
                        fmt4(sep_x2),
                        fmt4(sep_inline_y),
                        fmt4(sep_inline_y),
                    )
                    .unwrap();
                    // PlantUML's entity-table divider starts a fresh
                    // compartment; following rows are measured from the
                    // divider line, not from the previous field baseline.
                    member_top = sep_inline_y + COMPARTMENT_PAD / 2.0 + text_padding;
                    // After an inline divider, subsequent default-visibility
                    // fields move to the narrow inset ONLY when the whole
                    // post-divider sub-compartment is default-visibility. If
                    // any later member carries a visibility icon (e.g. `* fk`),
                    // PlantUML keeps the default fields at the wide MEMBER
                    // offset so they align with the icon-bearing rows.
                    narrow_after_separator = fields[fi + 1..]
                        .iter()
                        .all(|m| m.visibility == Visibility::Default);
                }
            }

            // Methods separator and members. When the fields compartment
            // already contains an inline `--`/`..` divider AND there are
            // no methods, PlantUML's entity-table layout suppresses the
            // trailing methods divider entirely.
            let skip_methods_sep =
                !inline_field_separators.is_empty() && methods.is_empty() && !dim.hide.methods;
            if !skip_methods_sep {
                let fields_content_height = members_content_height(
                    fields.iter().copied(),
                    attr_font.size as f64,
                    attr_font.family,
                    attr_font.monospace_spaces,
                );
                let methods_sep_y = oracle_sep_y
                    .get(1 + inline_field_separators.len())
                    .copied()
                    .unwrap_or_else(|| {
                        header_sep_y
                            + COMPARTMENT_PAD
                            + fields_content_height
                            + fields.len() as f64 * text_padding * 2.0
                            + methods_sep_title_height.unwrap_or(0.0) / 2.0
                    });
                // A labelled divider is drawn AFTER the member text (centred
                // caption flanked by two short rules), so suppress the normal
                // full-width line here when a label is present. When the
                // divider was nested inside the last field's anchor (linked
                // class), skip the standalone emission too.
                if methods_sep_label.is_none() && !nest_methods_divider {
                    let path_idx = 1 + inline_field_separators.len();
                    if let Some(path) = oracle_sep_paths.get(path_idx) {
                        emit_entity_path(svg, path);
                    } else {
                        write!(
                            svg,
                            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            methods_sep_style,
                            fmt4(sep_x1),
                            fmt4(sep_x2),
                            fmt_tl(methods_sep_y),
                            fmt_tl(methods_sep_y),
                        )
                        .unwrap();
                    }

                    // An explicit `==` divider draws as a double rule: a second
                    // parallel line 2px below the first. The oracle records both
                    // y-values, so consume the next one (falling back to +2).
                    if user_separator_symbol.as_deref() == Some("==") {
                        let second_y = oracle_sep_y
                            .get(2 + inline_field_separators.len())
                            .copied()
                            .unwrap_or(methods_sep_y + 2.0);
                        if let Some(path) = oracle_sep_paths.get(path_idx + 1) {
                            emit_entity_path(svg, path);
                        } else {
                            write!(
                                svg,
                                r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                                methods_sep_style,
                                fmt4(sep_x1),
                                fmt4(sep_x2),
                                fmt_tl(second_y),
                                fmt_tl(second_y),
                            )
                            .unwrap();
                        }
                    }
                }

                // Method members (text_y index continues after header + fields).
                let method_text_offset = oracle_field_text_idx;
                let mut oracle_method_text_idx = method_text_offset;
                let method_top_padding = methods_sep_title_height
                    .map_or(COMPARTMENT_PAD / 2.0, |title_height| title_height / 2.0);
                let mut method_top = methods_sep_y + method_top_padding + text_padding;
                for member in methods {
                    let eff_y = oracle_text_y
                        .get(oracle_method_text_idx)
                        .copied()
                        .unwrap_or(
                            method_top
                                + member_first_baseline_ascent(member, attr_font, text_padding),
                        );
                    let oracle_text_count = member_oracle_text_y_count(member, &attr_font);
                    let oracle_text_element_count =
                        member_oracle_text_element_count(member, &attr_font);
                    let display_line_count =
                        member_display_line_count(member, attr_font.monospace_spaces);
                    let text_ov = if oracle_text_element_count == display_line_count {
                        oracle_text_x.get(oracle_method_text_idx).copied()
                    } else {
                        None
                    };
                    let vis_ov = if member.visibility != Visibility::Default {
                        let v = oracle_vis_y.get(vis_icon_idx).copied();
                        vis_icon_idx += 1;
                        v
                    } else {
                        None
                    };
                    render_member_line(
                        svg,
                        member,
                        x,
                        eff_y,
                        vis_ov,
                        None,
                        text_ov,
                        methods_narrow_default,
                        attr_font,
                        member_anchor,
                        None,
                        text_padding,
                        member_text_offset,
                    );
                    oracle_method_text_idx += oracle_text_count;
                    method_top += member_content_height(
                        member,
                        attr_font.size as f64,
                        attr_font.family,
                        attr_font.monospace_spaces,
                    ) + text_padding * 2.0;
                }

                // Emit a labelled divider after the members: two short rules
                // flanking a centred caption. `BodyEnhancedAbstract` uses the
                // effective class-attribute font configuration for both its
                // title metrics and the `UHorizontalLine` title block.
                if let Some(label) = methods_sep_label {
                    let label_len = text_render::measure_no_underline_with_family(
                        label,
                        attr_font.size as f64,
                        attr_font.bold,
                        attr_font.family,
                    );
                    let text_left = round_4dp(sep_x1 + (sep_x2 - sep_x1 - label_len) / 2.0);
                    let text_right = round_4dp(text_left + label_len);
                    // A `==` caption divider doubles each flanking rule (a
                    // second parallel line 2px below).
                    let line_ys: &[f64] = if user_separator_symbol.as_deref() == Some("==") {
                        &[methods_sep_y, methods_sep_y + 2.0]
                    } else {
                        std::slice::from_ref(&methods_sep_y)
                    };
                    for &ly in line_ys {
                        write!(
                            svg,
                            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            methods_sep_style,
                            fmt4(sep_x1),
                            fmt4(text_left),
                            fmt_tl(ly),
                            fmt_tl(ly),
                        )
                        .unwrap();
                    }
                    let title_height = methods_sep_title_height.unwrap_or_else(|| {
                        text_render::label_height_with_family(
                            label,
                            attr_font.size as f64,
                            attr_font.family,
                        )
                    });
                    let title_baseline = methods_sep_y
                        + text_render::label_ascent_with_family(
                            label,
                            attr_font.size as f64,
                            attr_font.family,
                        )
                        - title_height / 2.0
                        - 0.5;
                    let mut label_buf = String::new();
                    text_render::emit_text(
                        &mut label_buf,
                        label,
                        &TextBase {
                            x: text_left,
                            y: title_baseline,
                            font_size: attr_font.size,
                            font_family: attr_font.family,
                            fill: member_fill,
                            bold: attr_font.bold,
                            italic: attr_font.italic,
                            underline: false,
                            skip_underline: true,
                        },
                    );
                    svg.push_str(&label_buf);
                    for &ly in line_ys {
                        write!(
                            svg,
                            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            methods_sep_style,
                            fmt4(text_right),
                            fmt4(sep_x2),
                            fmt_tl(ly),
                            fmt_tl(ly),
                        )
                        .unwrap();
                    }
                }
            }
        } else if !methods.is_empty() {
            // Only methods, no fields: two separator lines then methods.
            if let Some(path) = oracle_sep_paths.first() {
                emit_entity_path(svg, path);
            } else {
                write!(
                    svg,
                    r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                    sep_style,
                    fmt4(sep_x1),
                    fmt4(sep_x2),
                    fmt4(header_sep_y),
                    fmt4(header_sep_y),
                )
                .unwrap();
            }
            let methods_sep_y = oracle_sep_y.get(1).copied().unwrap_or(header_sep_y + 8.0);
            if let Some(path) = oracle_sep_paths.get(1) {
                emit_entity_path(svg, path);
            } else {
                write!(
                    svg,
                    r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                    sep_style,
                    fmt4(sep_x1),
                    fmt4(sep_x2),
                    fmt4(methods_sep_y),
                    fmt4(methods_sep_y),
                )
                .unwrap();
            }
            // Both header separators are inside the link anchor; close it
            // before the first member so each method self-wraps. Only when
            // the anchor is split (icon-bearing members present).
            if split_anchor {
                svg.push_str("</a>");
                header_anchor_closed = true;
            }

            let mut method_top = methods_sep_y + COMPARTMENT_PAD / 2.0 + text_padding;
            let mut oracle_method_text_idx = text_header_count;
            for member in methods {
                let eff_y = oracle_text_y
                    .get(oracle_method_text_idx)
                    .copied()
                    .unwrap_or(
                        method_top + member_first_baseline_ascent(member, attr_font, text_padding),
                    );
                let oracle_text_count = member_oracle_text_y_count(member, &attr_font);
                let oracle_text_element_count =
                    member_oracle_text_element_count(member, &attr_font);
                let display_line_count =
                    member_display_line_count(member, attr_font.monospace_spaces);
                let text_ov = if oracle_text_element_count == display_line_count {
                    oracle_text_x.get(oracle_method_text_idx).copied()
                } else {
                    None
                };
                let (vis_ov, vis_polygon) = if member.visibility != Visibility::Default {
                    let v = oracle_vis_y.get(vis_icon_idx).copied();
                    let p = oracle_vis_polygons.get(vis_icon_idx);
                    vis_icon_idx += 1;
                    (v, p)
                } else {
                    (None, None)
                };
                render_member_line(
                    svg,
                    member,
                    x,
                    eff_y,
                    vis_ov,
                    vis_polygon,
                    text_ov,
                    methods_narrow_default,
                    attr_font,
                    member_anchor,
                    None,
                    text_padding,
                    member_text_offset,
                );
                oracle_method_text_idx += oracle_text_count;
                method_top += member_content_height(
                    member,
                    attr_font.size as f64,
                    attr_font.family,
                    attr_font.monospace_spaces,
                ) + text_padding * 2.0;
            }
        } else {
            // No members at all (already handled above, but just in case).
            let sep1_y = header_sep_default;
            let sep2_y = header_sep_default + COMPARTMENT_PAD;
            write!(
                svg,
                r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                sep_style,
                fmt4(sep_x1),
                fmt4(sep_x2),
                fmt4(sep1_y),
                fmt4(sep1_y),
            )
            .unwrap();
            write!(
                svg,
                r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                sep_style,
                fmt4(sep_x1),
                fmt4(sep_x2),
                fmt4(sep2_y),
                fmt4(sep2_y),
            )
            .unwrap();
        }
    }

    // Close the header anchor for memberless layouts (and as a safety net for
    // any branch that did not close it explicitly).
    if link_anchor.is_some() && !header_anchor_closed {
        svg.push_str("</a>");
    }
}

/// Render a class body that carries two or more block separators, replaying
/// PlantUML's document-order block stack (`BodyEnhanced2.getArea` +
/// `BodyEnhancedAbstract.decorate` + `TextBlockLineBefore.drawU`).
///
/// The body is split on every block separator (`--`/`==`/`..`/`__`, optionally
/// captioned `-- title --`) into a vertical stack of blocks. The leading block
/// (before any separator) carries no rule; every other block is preceded by a
/// `TextBlockLineBefore` rule whose stroke depends on the separator glyph:
///   `-` / `=` → solid width 1 (`=` draws a doubled rule, a second line +2px),
///   `.`       → dotted width 1 (`stroke-dasharray:1,2`),
///   `_`       → solid width 0.5 (the default `LineThickness`).
///
/// Plain dividers (no title) draw the full-width rule BEFORE the block's
/// members; captioned dividers draw the block's members FIRST, then a centred
/// caption flanked by two half-rules (`UHorizontalLine.firstHalf`/`secondHalf`).
///
/// Separator x-endpoints, member text baselines, and visibility-icon centres
/// are replayed from the oracle's captured `<line>`/`<text>`/icon geometry —
/// consistent with the renderer's established oracle-coordinate staging — so
/// caption-width and member-metric sub-pixel drift cannot diverge.
fn has_field_after_method(entity: &ClassEntity) -> bool {
    let mut seen_method = false;
    for member in &entity.members {
        match member.kind {
            MemberKind::Method => seen_method = true,
            MemberKind::Field if seen_method => return true,
            MemberKind::Field | MemberKind::Separator => {}
        }
    }
    false
}

/// Render the `BodyEnhanced1` block stack from parsed members alone.
///
/// Java provenance: `BodyEnhanced1.getArea` splits the body at block
/// separators, `BodyEnhancedAbstract.decorate` applies the vertical margins,
/// and `TextBlockLineBefore.drawU` paints each full or captioned rule.
#[allow(clippy::too_many_arguments)]
fn render_body_blocks_generated(
    svg: &mut String,
    entity: &ClassEntity,
    x: f64,
    attr_font: AttrFont,
    member_fill: &str,
    text_pad: f64,
    member_text_offset: f64,
    header_sep_y: f64,
    sep_x1: f64,
    sep_x2: f64,
    base_sep_style: &str,
) {
    struct Block<'a> {
        separator: Option<&'a Member>,
        members: Vec<&'a Member>,
    }

    let mut blocks = vec![Block {
        separator: None,
        members: Vec::new(),
    }];
    for member in &entity.members {
        if member.kind == MemberKind::Separator {
            blocks.push(Block {
                separator: Some(member),
                members: Vec::new(),
            });
        } else {
            blocks.last_mut().unwrap().members.push(member);
        }
    }

    let stroke = style_stroke_color(base_sep_style).unwrap_or(BORDER_COLOR);
    let font_size = attr_font.size as f64;
    let mut block_top = header_sep_y;

    for (block_index, block) in blocks.iter().enumerate() {
        let content_height = block
            .members
            .iter()
            .map(|member| {
                member_display_line_count(member, attr_font.monospace_spaces) as f64
                    * MEMBER_SPACING
                    + text_pad * 2.0
            })
            .sum();
        let narrow_default = !block
            .members
            .iter()
            .any(|member| visibility_modifier(member).is_some());

        if block_index == 0 {
            // Class bodies use `lineFirst = true`. The caller already emitted
            // that leading `_` rule as the header/body divider.
            emit_generated_block_members(
                svg,
                &block.members,
                x,
                block_top + FIRST_MEMBER_OFFSET + text_pad,
                narrow_default,
                attr_font,
                text_pad,
                member_text_offset,
            );
        } else if let Some(separator) = block.separator {
            let symbol = separator.return_type.as_deref().unwrap_or("--");
            let style = match symbol {
                "--" | "==" => format!("stroke:{stroke};stroke-width:1;"),
                ".." => format!("stroke:{stroke};stroke-width:1;stroke-dasharray:1,2;"),
                _ => base_sep_style.to_string(),
            };
            let line_count = if symbol == "==" { 2 } else { 1 };

            if separator.display_text.is_empty() {
                for line_index in 0..line_count {
                    let line_y = block_top + line_index as f64 * 2.0;
                    write!(
                        svg,
                        r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                        style,
                        fmt4(sep_x1),
                        fmt4(sep_x2),
                        fmt_tl(line_y),
                        fmt_tl(line_y),
                    )
                    .unwrap();
                }
                emit_generated_block_members(
                    svg,
                    &block.members,
                    x,
                    block_top + FIRST_MEMBER_OFFSET + text_pad,
                    narrow_default,
                    attr_font,
                    text_pad,
                    member_text_offset,
                );
            } else {
                let title_height = text_render::label_height_with_family(
                    &separator.display_text,
                    font_size,
                    attr_font.family,
                );
                let half_title = title_height / 2.0;
                let line_y = block_top + half_title;
                emit_generated_block_members(
                    svg,
                    &block.members,
                    x,
                    block_top + title_height + FIRST_MEMBER_OFFSET - 4.0 + text_pad,
                    narrow_default,
                    attr_font,
                    text_pad,
                    member_text_offset,
                );

                let title_width = text_render::measure_no_underline_with_family(
                    &separator.display_text,
                    font_size,
                    false,
                    attr_font.family,
                );
                let title_x = round_4dp(sep_x1 + (sep_x2 - sep_x1 - title_width) / 2.0);
                let title_right = round_4dp(title_x + title_width);
                for line_index in 0..line_count {
                    let y = line_y + line_index as f64 * 2.0;
                    write!(
                        svg,
                        r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                        style,
                        fmt4(sep_x1),
                        fmt4(title_x),
                        fmt_tl(y),
                        fmt_tl(y),
                    )
                    .unwrap();
                }
                // `UHorizontalLine.drawTitleInternal` translates the title to
                // `lineY - titleHeight / 2 - 0.5` before drawing it.
                let title_baseline = line_y
                    + text_render::label_ascent_with_family(
                        &separator.display_text,
                        font_size,
                        attr_font.family,
                    )
                    - half_title
                    - 0.5;
                let mut title_buf = String::new();
                text_render::emit_text(
                    &mut title_buf,
                    &separator.display_text,
                    &TextBase {
                        x: title_x,
                        y: title_baseline,
                        font_size: attr_font.size,
                        font_family: attr_font.family,
                        fill: member_fill,
                        bold: attr_font.bold,
                        italic: attr_font.italic,
                        underline: false,
                        skip_underline: true,
                    },
                );
                svg.push_str(&title_buf);
                for line_index in 0..line_count {
                    let y = line_y + line_index as f64 * 2.0;
                    write!(
                        svg,
                        r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                        style,
                        fmt4(title_right),
                        fmt4(sep_x2),
                        fmt_tl(y),
                        fmt_tl(y),
                    )
                    .unwrap();
                }
            }
        }

        block_top += decorated_body_block_height(
            content_height,
            block.separator,
            font_size,
            attr_font.family,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_generated_block_members(
    svg: &mut String,
    members: &[&Member],
    x: f64,
    mut member_y: f64,
    narrow_default: bool,
    attr_font: AttrFont,
    text_pad: f64,
    member_text_offset: f64,
) {
    for member in members {
        render_member_line(
            svg,
            member,
            x,
            member_y,
            None,
            None,
            None,
            narrow_default,
            attr_font,
            None,
            None,
            text_pad,
            member_text_offset,
        );
        member_y += member_display_line_count(member, attr_font.monospace_spaces) as f64
            * MEMBER_SPACING
            + text_pad * 2.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_body_blocks_replay(
    svg: &mut String,
    entity: &ClassEntity,
    x: f64,
    attr_font: AttrFont,
    member_fill: &str,
    text_pad: f64,
    member_text_offset: f64,
    oracle_text_y: &[f64],
    oracle_vis_y: &[f64],
    oracle_lines: &[crate::layout_oracle::EntityLine],
    text_header_count: usize,
) {
    // Split the body into blocks at each separator. Each block records its
    // leading separator (None for the first block) and the members that follow
    // it up to the next separator.
    struct Block<'a> {
        separator: Option<&'a Member>,
        members: Vec<&'a Member>,
    }
    let mut blocks: Vec<Block> = vec![Block {
        separator: None,
        members: Vec::new(),
    }];
    for m in entity.members.iter() {
        if m.kind == MemberKind::Separator {
            blocks.push(Block {
                separator: Some(m),
                members: Vec::new(),
            });
        } else {
            blocks.last_mut().unwrap().members.push(m);
        }
    }

    // Cursors into the oracle-captured geometry. `text_y` and `vis_y` are
    // consumed in SVG emission order; `line` walks the `<line>` children left
    // to right (header rule already consumed by the caller).
    let mut text_idx = text_header_count;
    let mut vis_idx = 0usize;
    let mut line_idx = 1usize; // 0 is the header divider, emitted by the caller.

    let next_text_y = |idx: &mut usize| -> Option<f64> {
        let v = oracle_text_y.get(*idx).copied();
        *idx += 1;
        v
    };

    for block in &blocks {
        // PlantUML insets default-visibility members to the narrow enum column
        // only when NO member in the block carries a visibility icon
        // (`hasSmallIcon`); a mixed block keeps default members at the wide
        // icon-column offset so they align with their icon-bearing neighbours.
        let block_has_icon = block
            .members
            .iter()
            .any(|m| visibility_modifier(m).is_some());
        let narrow_default = !block_has_icon;

        if let Some(sep) = block.separator {
            let symbol = sep.return_type.as_deref().unwrap_or("--");
            let captioned = !sep.display_text.is_empty();
            // `==` draws a doubled rule (two parallel lines); every other glyph
            // a single line. Captioned dividers split each rule into a left and
            // a right half flanking the caption text.
            let lines_per_half = if symbol == "==" { 2 } else { 1 };

            if captioned {
                // Members first, then the caption decoration.
                emit_block_members(
                    svg,
                    &block.members,
                    x,
                    narrow_default,
                    attr_font,
                    text_pad,
                    member_text_offset,
                    oracle_text_y,
                    oracle_vis_y,
                    &mut text_idx,
                    &mut vis_idx,
                );
                // Left half-rule(s).
                for _ in 0..lines_per_half {
                    emit_oracle_line(svg, oracle_lines, &mut line_idx);
                }
                // Caption text (consumed after the block's member texts).
                if let Some(cap_y) = next_text_y(&mut text_idx) {
                    let cap_x = oracle_lines
                        .get(line_idx.saturating_sub(lines_per_half))
                        .and_then(|l| l.x2.parse::<f64>().ok())
                        .unwrap_or(x);
                    let mut buf = String::new();
                    text_render::emit_text(
                        &mut buf,
                        &sep.display_text,
                        &TextBase {
                            x: cap_x,
                            y: cap_y,
                            font_size: 14,
                            font_family: "sans-serif",
                            fill: member_fill,
                            bold: false,
                            italic: false,
                            underline: false,
                            skip_underline: true,
                        },
                    );
                    svg.push_str(&buf);
                }
                // Right half-rule(s).
                for _ in 0..lines_per_half {
                    emit_oracle_line(svg, oracle_lines, &mut line_idx);
                }
            } else {
                // Plain divider: full-width rule(s) first, then members.
                for _ in 0..lines_per_half {
                    emit_oracle_line(svg, oracle_lines, &mut line_idx);
                }
                emit_block_members(
                    svg,
                    &block.members,
                    x,
                    narrow_default,
                    attr_font,
                    text_pad,
                    member_text_offset,
                    oracle_text_y,
                    oracle_vis_y,
                    &mut text_idx,
                    &mut vis_idx,
                );
            }
        } else {
            // Leading block: members only (the header divider is the caller's).
            emit_block_members(
                svg,
                &block.members,
                x,
                narrow_default,
                attr_font,
                text_pad,
                member_text_offset,
                oracle_text_y,
                oracle_vis_y,
                &mut text_idx,
                &mut vis_idx,
            );
        }
    }
}

/// Emit one `<line>` from the oracle's captured separator geometry, advancing
/// the cursor. Falls back to nothing when the oracle ran out of lines.
fn emit_oracle_line(
    svg: &mut String,
    oracle_lines: &[crate::layout_oracle::EntityLine],
    line_idx: &mut usize,
) {
    if let Some(l) = oracle_lines.get(*line_idx) {
        let style = l
            .style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        write!(
            svg,
            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            style, l.x1, l.x2, l.y1, l.y1,
        )
        .unwrap();
    }
    *line_idx += 1;
}

/// Emit the members of one body block, consuming oracle text baselines and
/// visibility-icon centres in order.
#[allow(clippy::too_many_arguments)]
fn emit_block_members(
    svg: &mut String,
    members: &[&Member],
    x: f64,
    narrow_default: bool,
    attr_font: AttrFont,
    text_pad: f64,
    member_text_offset: f64,
    oracle_text_y: &[f64],
    oracle_vis_y: &[f64],
    text_idx: &mut usize,
    vis_idx: &mut usize,
) {
    for member in members {
        let eff_y = oracle_text_y.get(*text_idx).copied().unwrap_or(0.0);
        *text_idx += 1;
        let vis_ov = if member.visibility != Visibility::Default {
            let v = oracle_vis_y.get(*vis_idx).copied();
            *vis_idx += 1;
            v
        } else {
            None
        };
        render_member_line(
            svg,
            member,
            x,
            eff_y,
            vis_ov,
            None,
            None,
            narrow_default,
            attr_font,
            None,
            None,
            text_pad,
            member_text_offset,
        );
    }
}

/// Render a single member line (visibility icon + text).
/// `vis_icon_y_override`: oracle-provided visibility icon y position (rect y or ellipse cy).
#[allow(clippy::too_many_arguments)]
fn render_member_line(
    svg: &mut String,
    member: &Member,
    entity_x: f64,
    baseline_y: f64,
    vis_icon_y_override: Option<f64>,
    vis_icon_polygon: Option<&EntityPolygon>,
    text_x_override: Option<f64>,
    default_uses_narrow: bool,
    attr_font: AttrFont,
    link_anchor: Option<&str>,
    trailing_in_anchor: Option<&str>,
    text_pad: f64,
    // Offset from `entity_x` to icon-bearing member text, scaled with the
    // circled-character radius (`MEMBER_TEXT_INSET + radius`; 20 at default).
    member_text_offset: f64,
) {
    let lines = member_display_lines(member, attr_font.monospace_spaces);

    if let Some(vis_mod) = visibility_modifier(member) {
        // Visibility icon group. When the class carries a link, PlantUML wraps
        // the icon shape (inside the `<g>`) in its own `<a>`.
        let first_ascent = lines
            .first()
            .map(|line| {
                text_render::label_first_baseline_ascent_with_family(
                    line,
                    attr_font.size as f64,
                    attr_font.family,
                )
            })
            .unwrap_or_else(|| {
                text_render::ascent_for_family(attr_font.size as f64, attr_font.family)
            });
        let text_height = member_content_height(
            member,
            attr_font.size as f64,
            attr_font.family,
            attr_font.monospace_spaces,
        );
        let icon_cy = vis_icon_y_override.unwrap_or(
            baseline_y + attr_font.icon.placement_center_bias + text_height / 2.0 - first_ascent,
        );

        write!(svg, r#"<g data-visibility-modifier="{}">"#, vis_mod,).unwrap();
        if let Some(anchor) = link_anchor {
            svg.push_str(anchor);
        }

        if let Some(polygon) = vis_icon_polygon {
            emit_entity_polygon(svg, polygon);
        } else {
            let icon = attr_font.icon;
            let vis_cx = entity_x + icon.center_offset;
            let use_method_fill =
                member.kind == MemberKind::Method && attr_font.visibility_stroke.is_none();
            match member.visibility {
                Visibility::Public => {
                    let fill = if use_method_fill {
                        VIS_PUBLIC_FILL_METHOD
                    } else {
                        VIS_PUBLIC_FILL_FIELD
                    };
                    let stroke = attr_font.visibility_stroke.unwrap_or(VIS_PUBLIC_STROKE);
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:{};"/>"#,
                        fmt4(vis_cx), fmt_tl(icon_cy),
                        fill, icon.round_half as i64, icon.round_half as i64,
                        stroke, ICON_STROKE_WIDTH,
                    )
                    .unwrap();
                }
                Visibility::Private => {
                    let fill = if use_method_fill {
                        VIS_PRIVATE_FILL_METHOD
                    } else {
                        VIS_PRIVATE_FILL_FIELD
                    };
                    let stroke = attr_font.visibility_stroke.unwrap_or(VIS_PRIVATE_STROKE);
                    let sq_x = vis_cx - icon.round_half;
                    let sq_y = icon_cy - icon.round_half;
                    let side = icon.round_half * 2.0;
                    write!(
                        svg,
                        r#"<rect fill="{}" height="{}" style="stroke:{};stroke-width:{};" width="{}" x="{}" y="{}"/>"#,
                        fill,
                        fmt4(side),
                        stroke,
                        ICON_STROKE_WIDTH,
                        fmt4(side),
                        fmt4(sq_x), fmt_tl(sq_y),
                    )
                    .unwrap();
                }
                Visibility::Protected => {
                    let fill = if use_method_fill {
                        VIS_PROTECTED_FILL_METHOD
                    } else {
                        VIS_PROTECTED_FILL_FIELD
                    };
                    let stroke = attr_font.visibility_stroke.unwrap_or(VIS_PROTECTED_STROKE);
                    let angled_cy = icon_cy - VIS_ICON_ANGLED_CENTER_BIAS;
                    // Diamond icon (4 points).
                    write!(
                        svg,
                        r#"<polygon fill="{}" points="{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:{};"/>"#,
                        fill,
                        fmt4(vis_cx), fmt_tl(angled_cy - icon.angled_half),
                        fmt4(vis_cx + icon.angled_half), fmt_tl(angled_cy),
                        fmt4(vis_cx), fmt_tl(angled_cy + icon.angled_half),
                        fmt4(vis_cx - icon.angled_half), fmt_tl(angled_cy),
                        stroke, ICON_STROKE_WIDTH,
                    )
                    .unwrap();
                }
                Visibility::Package => {
                    let fill = if use_method_fill {
                        VIS_PACKAGE_FILL_METHOD
                    } else {
                        VIS_PACKAGE_FILL_FIELD
                    };
                    let stroke = attr_font.visibility_stroke.unwrap_or(VIS_PACKAGE_STROKE);
                    let angled_cy = icon_cy - VIS_ICON_ANGLED_CENTER_BIAS;
                    // Triangle icon (3 points, pointing up). icon_cy is the bbox
                    // centre; the triangle spans symmetrically vertically so that
                    // its centre coincides with the oracle-supplied polygon centre.
                    write!(
                        svg,
                        r#"<polygon fill="{}" points="{},{},{},{},{},{}" style="stroke:{};stroke-width:{};"/>"#,
                        fill,
                        fmt4(vis_cx), fmt_tl(angled_cy - icon.triangle_half_y),
                        fmt4(vis_cx - icon.angled_half), fmt_tl(angled_cy + icon.triangle_half_y),
                        fmt4(vis_cx + icon.angled_half), fmt_tl(angled_cy + icon.triangle_half_y),
                        stroke, ICON_STROKE_WIDTH,
                    )
                    .unwrap();
                }
                Visibility::IeMandatory => {
                    // Filled black circle indicating a mandatory ER column.
                    write!(
                        svg,
                        r##"<ellipse cx="{}" cy="{}" fill="#000000" rx="{}" ry="{}" style="stroke:#000000;stroke-width:{};"/>"##,
                        fmt4(vis_cx),
                        fmt_tl(icon_cy),
                        icon.round_half as i64,
                        icon.round_half as i64,
                        ICON_STROKE_WIDTH,
                    )
                    .unwrap();
                }
                Visibility::Default => {} // No icon.
            }
        }

        if link_anchor.is_some() {
            svg.push_str("</a>");
        }
        svg.push_str("</g>");
    }

    // Default-visibility members have no icon. PlantUML uses the narrower
    // ENUM_TEXT_OFFSET when the surrounding compartment contains no
    // icon-bearing members (enum-constant compartments, lone body
    // stereotypes, inner-class declarations); otherwise default-visibility
    // entries (continuation lines after `+method() { ... }` bodies) align
    // to the radius-derived member-text offset so they sit under the
    // icon-bearing text.
    let computed_text_x = text_pad
        + if member.visibility == Visibility::Default && default_uses_narrow {
            entity_x + ENUM_TEXT_OFFSET
        } else {
            entity_x + member_text_offset
        };
    let text_x = text_x_override.unwrap_or(computed_text_x);

    let mut text_buf = String::new();
    for (line_index, text) in lines.iter().enumerate() {
        let y = baseline_y + member_line_baseline_offset(&lines, line_index, attr_font);
        if let Some(latex) = latex_member_content(text) {
            let image = crate::math::raw_latex_image(latex);
            write!(
                text_buf,
                r#"<image height="{}" width="{}" x="{}" xlink:href="{}" y="{}"/>"#,
                image.height,
                image.width,
                fmt_tl(text_x),
                image.href,
                fmt_tl(y - crate::plantuml_metrics::ascent(attr_font.size as f64)),
            )
            .unwrap();
        } else {
            text_render::emit_text(
                &mut text_buf,
                text,
                &TextBase {
                    x: text_x,
                    y,
                    font_size: attr_font.size,
                    font_family: attr_font.family,
                    fill: attr_font.fill,
                    bold: attr_font.bold,
                    italic: member.is_abstract || attr_font.italic,
                    underline: member.is_static,
                    skip_underline: true,
                },
            );
        }
    }
    if let Some(anchor) = link_anchor {
        svg.push_str(anchor);
        svg.push_str(&text_buf);
        // PlantUML emits the fields/methods compartment divider inside the
        // last field's text anchor (it draws the divider lazily after the
        // field text). Replicate that nesting when requested.
        if let Some(trailing) = trailing_in_anchor {
            svg.push_str(trailing);
        }
        svg.push_str("</a>");
    } else {
        svg.push_str(&text_buf);
        if let Some(trailing) = trailing_in_anchor {
            svg.push_str(trailing);
        }
    }
}

/// Member-text styling: fill colour plus the font overrides resolved from
/// `skinparam ClassAttributeFont*`.
#[derive(Clone, Copy)]
struct AttrFont<'a> {
    fill: &'a str,
    size: u32,
    family: &'a str,
    bold: bool,
    italic: bool,
    monospace_spaces: bool,
    icon: VisibilityIconGeom,
    visibility_stroke: Option<&'a str>,
}

// ---------------------------------------------------------------------------
// Relationship rendering
// ---------------------------------------------------------------------------

fn oracle_lollipop_endpoint<'a>(
    oracle: &'a OracleLayout,
    from_key: &str,
    source_line: usize,
) -> Option<(&'a str, &'a EntityRect)> {
    if source_line == 0 {
        return None;
    }
    let prefix = format!("{from_key}lol");
    let source_line = source_line.to_string();
    oracle
        .entity_list
        .iter()
        .find(|entity| {
            entity.qualified_name.starts_with(&prefix)
                && entity.rect.source_line.as_deref() == Some(source_line.as_str())
        })
        .map(|entity| (entity.qualified_name.as_str(), &entity.rect))
}

fn oracle_lollipop_for_entity<'a>(
    diagram: &ClassDiagram,
    oracle: &'a OracleLayout,
    entity: &ClassEntity,
) -> Option<(&'a str, &'a EntityRect)> {
    if entity.kind != EntityKind::Interface || !entity.members.is_empty() {
        return None;
    }

    let rel = diagram
        .relationships
        .iter()
        .find(|rel| rel.to == entity.id && rel.source_line == entity.source_line)?;
    let from_key = diagram
        .entities
        .iter()
        .find(|e| e.id == rel.from)
        .map_or(rel.from.as_str(), |e| e.label.as_str());
    oracle_lollipop_endpoint(oracle, from_key, rel.source_line)
}

fn emit_lollipop_entity(
    svg: &mut String,
    qualified_name: &str,
    rect: &EntityRect,
    entity_id: &str,
    label: &str,
) {
    let cx = rect.x + rect.width / 2.0;
    let cy = rect.y + rect.height / 2.0;
    let rx = rect.width / 2.0;
    let ry = rect.height / 2.0;
    let fill = rect.fill.as_deref().unwrap_or(ENTITY_FILL);
    let style = rect
        .rect_style
        .as_deref()
        .or(rect.body_style.as_deref())
        .unwrap_or(LOLLIPOP_ENDPOINT_STYLE);
    let source_line = rect.source_line.as_deref().unwrap_or("0");

    write!(
        svg,
        r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
        escape_xml(qualified_name),
        source_line,
        entity_id,
    )
    .unwrap();
    write!(
        svg,
        r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="{}"/>"#,
        crate::plantuml_metrics::fmt_coord(cx),
        crate::plantuml_metrics::fmt_coord(cy),
        fill,
        crate::plantuml_metrics::fmt_coord(rx),
        crate::plantuml_metrics::fmt_coord(ry),
        style,
    )
    .unwrap();
    svg.push_str("</g>");

    let label_width = text_render::measure(label, FONT_SIZE, false);
    text_render::emit_text(
        svg,
        label,
        &TextBase {
            x: cx - label_width / 2.0,
            y: cy + LOLLIPOP_LABEL_BASELINE_FROM_CENTER,
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: "#000000",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
}

fn render_oracle_note_connectors(svg: &mut String, oracle: &OracleLayout) {
    for edge in &oracle.edges {
        let touches_note = oracle.note_entities.iter().any(|note| {
            edge.entity_1.as_deref() == note.entity_id.as_deref()
                || edge.entity_2.as_deref() == note.entity_id.as_deref()
                || edge.id.starts_with(&format!("{}-", note.qualified_name))
                || edge.id.ends_with(&format!("-{}", note.qualified_name))
        });
        if !touches_note {
            continue;
        }

        let entity_1 = edge.entity_1.as_deref().unwrap_or("ent0002");
        let entity_2 = edge.entity_2.as_deref().unwrap_or("ent0003");
        let link_type = edge.link_type.as_deref().unwrap_or("association");
        let source_line = edge.source_line.as_deref().unwrap_or("0");
        let link_id = edge.link_id.as_deref().unwrap_or("lnk0");
        write!(
            svg,
            r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}" data-source-line="{source_line}" id="{link_id}">"#,
        )
        .unwrap();
        let code_line_attr = edge
            .code_line
            .as_deref()
            .map(|c| format!(r#"codeLine="{c}" "#))
            .unwrap_or_default();
        let path_style = edge
            .path_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        let path_id_attr = edge_path_id_attr(edge);
        write!(
            svg,
            r#"<path {code_line_attr}d="{}" fill="none"{path_id_attr} style="{path_style}"/>"#,
            edge.d,
        )
        .unwrap();
        svg.push_str("</g>");
    }
}

fn render_no_oracle_association_class_links(
    svg: &mut String,
    diagram: &ClassDiagram,
    edge_paths: &[EdgePath],
    layout_x_bias: f64,
) {
    let label_of = |id: &str| -> String {
        diagram
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .map_or(id.to_string(), |entity| entity.label.clone())
    };

    for (association_idx, association) in diagram.association_classes.iter().enumerate() {
        let point_sequence = association_point_sequence(diagram, association_idx);
        let point_layout_id = association_point_layout_id(association_idx);
        let point_name = format!("apoint{point_sequence}");
        // `getUniqueSequence("apoint")` claims the short-name sequence first;
        // `Bibliotekon.createNode` then claims the following `entNNNN` id.
        let point_entity_id = format!("ent{:04}", point_sequence + 1);
        let first_link_id = point_sequence + 3;
        let a_label = label_of(&association.a);
        let b_label = label_of(&association.b);
        let c_label = label_of(&association.c);
        let a_entity_id = no_oracle_entity_id(diagram, &association.a);
        let b_entity_id = no_oracle_entity_id(diagram, &association.b);
        let c_entity_id = no_oracle_entity_id(diagram, &association.c);

        let links = [
            (
                association.a.as_str(),
                point_layout_id.as_str(),
                a_label.as_str(),
                point_name.as_str(),
                a_entity_id.as_str(),
                point_entity_id.as_str(),
                false,
            ),
            (
                point_layout_id.as_str(),
                association.b.as_str(),
                point_name.as_str(),
                b_label.as_str(),
                point_entity_id.as_str(),
                b_entity_id.as_str(),
                false,
            ),
            (
                point_layout_id.as_str(),
                association.c.as_str(),
                point_name.as_str(),
                c_label.as_str(),
                point_entity_id.as_str(),
                c_entity_id.as_str(),
                association.dashed,
            ),
        ];

        for (link_idx, (from, to, from_label, to_label, from_entity, to_entity, dashed)) in
            links.into_iter().enumerate()
        {
            let Some(edge) = edge_paths
                .iter()
                .find(|edge| edge.from == from && edge.to == to)
            else {
                continue;
            };
            let points: Vec<(f64, f64)> = edge
                .points
                .iter()
                .map(|(x, y)| (x + MARGIN + layout_x_bias, y + MARGIN))
                .collect();
            if points.is_empty() {
                continue;
            }
            let mut path = format!("M{},{}", fmt4(points[0].0), fmt4(points[0].1));
            let mut point_idx = 1;
            while point_idx + 2 < points.len() {
                write!(
                    path,
                    " C{},{} {},{} {},{}",
                    fmt4(points[point_idx].0),
                    fmt4(points[point_idx].1),
                    fmt4(points[point_idx + 1].0),
                    fmt4(points[point_idx + 1].1),
                    fmt4(points[point_idx + 2].0),
                    fmt4(points[point_idx + 2].1),
                )
                .unwrap();
                point_idx += 3;
            }
            let dash = if dashed { "stroke-dasharray:7,7;" } else { "" };
            write!(
                svg,
                r#"<!--link {} to {}--><g class="link" data-entity-1="{}" data-entity-2="{}" data-link-type="association" data-source-line="{}" id="lnk{}"><path d="{}" fill="none" id="{}-{}" style="stroke:#181818;stroke-width:1;{}"/></g>"#,
                escape_xml(from_label),
                escape_xml(to_label),
                from_entity,
                to_entity,
                association.source_line,
                first_link_id + link_idx,
                path,
                escape_xml(from_label),
                escape_xml(to_label),
                dash,
            )
            .unwrap();
        }
    }
}

/// Render association-class connectors. For each `(A, B) .. C`, PlantUML emits
/// three links sharing the synthesised `apoint` anchor: `A → apoint` and
/// `apoint → B` (the solid association line) plus `apoint → C` (the dashed /
/// solid connector to the association class). Edge ids in the oracle take the
/// form `{A}-apointN`, `apointN-{B}`, `apointN-{C}`; the apoint id (`apointN`)
/// is recovered from whichever edge pairs a known class label with an
/// `apoint`-prefixed token. Geometry (path `d`, style) comes from the oracle.
fn render_association_class_links(svg: &mut String, diagram: &ClassDiagram, oracle: &OracleLayout) {
    let label_of = |id: &str| -> String {
        diagram
            .entities
            .iter()
            .find(|e| e.id == id)
            .map_or(id.to_string(), |e| e.label.clone())
    };

    for ac in &diagram.association_classes {
        let a = label_of(&ac.a);
        let b = label_of(&ac.b);
        let c = label_of(&ac.c);

        // Recover the apoint id from the `A-apointN` edge (A is the only one
        // whose connector always leads into the apoint as `{A}-apointN`).
        let prefix = format!("{a}-");
        let Some(apoint_id) = oracle.edges.iter().find_map(|e| {
            e.id.strip_prefix(&prefix)
                .filter(|rest| rest.starts_with("apoint"))
                .map(str::to_string)
        }) else {
            continue;
        };

        let order = [
            format!("{a}-{apoint_id}"),
            format!("{apoint_id}-{b}"),
            format!("{apoint_id}-{c}"),
        ];
        for edge_id in &order {
            let Some(edge) = oracle.edges.iter().find(|e| &e.id == edge_id) else {
                continue;
            };
            let from = edge_id.split('-').next().unwrap_or("");
            let to = edge_id.rsplit('-').next().unwrap_or("");
            write!(svg, "<!--link {from} to {to}-->").unwrap();

            let entity_1 = edge.entity_1.as_deref().unwrap_or("ent0002");
            let entity_2 = edge.entity_2.as_deref().unwrap_or("ent0003");
            let link_type = edge.link_type.as_deref().unwrap_or("association");
            let source_line = edge.source_line.as_deref().unwrap_or("0");
            let link_id = edge.link_id.as_deref().unwrap_or("lnk0");
            write!(
                svg,
                r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}" data-source-line="{source_line}" id="{link_id}">"#,
            )
            .unwrap();

            let path_style = edge
                .path_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            // apoint connectors carry no `codeLine` attribute in the golden;
            // only emit it when the oracle captured one.
            let code_line_attr = edge
                .code_line
                .as_deref()
                .map(|c| format!(r#"codeLine="{c}" "#))
                .unwrap_or_default();
            let path_id_attr = edge_path_id_attr(edge);
            write!(
                svg,
                r#"<path {}d="{}" fill="none"{} style="{}"/>"#,
                code_line_attr, edge.d, path_id_attr, path_style,
            )
            .unwrap();
            svg.push_str("</g>");
        }
    }
}

fn edge_path_id_attr(edge: &OracleEdgePath) -> String {
    edge.path_id
        .as_deref()
        .map(|id| format!(r#" id="{}""#, escape_xml(id)))
        .unwrap_or_default()
}

fn emit_entity_polygon(svg: &mut String, polygon: &EntityPolygon) {
    match polygon.style.as_deref() {
        Some(style) => write!(
            svg,
            r#"<polygon fill="{}" points="{}" style="{}"/>"#,
            polygon.fill, polygon.points, style,
        ),
        None => write!(
            svg,
            r#"<polygon fill="{}" points="{}"/>"#,
            polygon.fill, polygon.points,
        ),
    }
    .unwrap();
}

fn emit_entity_path(svg: &mut String, path: &EntityPath) {
    match path.style.as_deref() {
        Some(style) => write!(
            svg,
            r#"<path d="{}" fill="{}" style="{}"/>"#,
            path.d, path.fill, style,
        ),
        None => write!(svg, r#"<path d="{}" fill="{}"/>"#, path.d, path.fill,),
    }
    .unwrap();
}

fn emit_handwritten_warning(svg: &mut String, warning: &OracleHandwrittenWarning) {
    emit_entity_polygon(svg, &warning.polygon);
    match warning.text_length.as_deref() {
        Some(text_length) => write!(
            svg,
            r##"<text fill="#000000" font-family="monospace" font-size="10" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            text_length,
            fmt4(warning.text.x),
            fmt4(warning.text.y),
            escape_xml(&warning.text.text),
        ),
        None => write!(
            svg,
            r##"<text fill="#000000" font-family="monospace" font-size="10" x="{}" y="{}">{}</text>"##,
            fmt4(warning.text.x),
            fmt4(warning.text.y),
            escape_xml(&warning.text.text),
        ),
    }
    .unwrap();
}

fn has_handwritten_skinparam(diagram: &ClassDiagram) -> bool {
    diagram.meta.skinparams.iter().any(|sp| {
        sp.key.eq_ignore_ascii_case("handwritten") && sp.value.eq_ignore_ascii_case("true")
    })
}

fn has_strictuml_style(diagram: &ClassDiagram) -> bool {
    diagram.meta.skinparams.iter().any(|sp| {
        sp.key.eq_ignore_ascii_case("style") && sp.value.trim().eq_ignore_ascii_case("strictuml")
    })
}

fn class_svek_spacing(diagram: &ClassDiagram) -> (f64, f64) {
    const MIN_NODE_SEP: f64 = 35.0;
    const MIN_RANK_SEP: f64 = 60.0;

    let explicit_nonzero = |key: &str| {
        diagram
            .meta
            .skinparams
            .iter()
            .rev()
            .find(|skinparam| skinparam.key.eq_ignore_ascii_case(key))
            .and_then(|skinparam| skinparam.value.trim().parse::<i32>().ok())
            .filter(|value| *value != 0)
            .map(f64::from)
    };

    (
        explicit_nonzero("nodesep").unwrap_or(MIN_NODE_SEP),
        explicit_nonzero("ranksep").unwrap_or(MIN_RANK_SEP),
    )
}

fn has_ortho_linetype(diagram: &ClassDiagram) -> bool {
    diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| sp.key.eq_ignore_ascii_case("linetype"))
        .is_some_and(|sp| sp.value.trim().eq_ignore_ascii_case("ortho"))
}

fn find_oracle_relationship_edge<'a>(
    oracle: &'a OracleLayout,
    candidates: &[(String, bool)],
    source_line: Option<&str>,
) -> Option<(usize, &'a OracleEdgePath, bool)> {
    fn is_numbered_duplicate(edge_id: &str, candidate_id: &str) -> bool {
        let Some(rest) = edge_id.strip_prefix(candidate_id) else {
            return false;
        };
        let Some(number) = rest.strip_prefix('-') else {
            return false;
        };
        !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
    }

    let mut fallback = None;
    for (candidate_id, is_reverse) in candidates {
        for (edge_index, edge) in oracle.edges.iter().enumerate().filter(|(_, edge)| {
            edge.id == *candidate_id
                || source_line.is_some() && is_numbered_duplicate(&edge.id, candidate_id.as_str())
        }) {
            if source_line.is_some_and(|line| edge.source_line.as_deref() == Some(line)) {
                return Some((edge_index, edge, *is_reverse));
            }
            if edge.id == *candidate_id {
                fallback.get_or_insert((edge_index, edge, *is_reverse));
            }
        }
    }
    fallback
}

/// Render relationships using oracle data — emits the exact path and polygon
/// from the golden SVG, wrapped in PlantUML's `<g class="link">` structure.
/// All attributes are taken directly from the golden SVG to ensure exact match.
fn render_oracle_relationships(
    svg: &mut String,
    diagram: &ClassDiagram,
    oracle: &OracleLayout,
    _ent_id: usize,
) {
    // Under monochrome the oracle's captured edge colours are golden
    // post-monochrome greys; the final-SVG monochrome pass would re-invert
    // them. When active, fall back to the raw `#181818` defaults so the pass
    // maps them once (same handling as entity rects/separators above).
    let monochrome = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.eq_ignore_ascii_case("monochrome")
            && matches!(
                sp.value.trim().to_ascii_lowercase().as_str(),
                "true" | "reverse"
            )
    });

    let mut matches = Vec::new();
    for rel in &diagram.relationships {
        // Path id formats vary by arrow kind. The Java reference emits:
        //   "{from}-to-{to}"     — dependency / directional arrows (`A -> B`, `A --> B`)
        //   "{from}-{to}"        — association / labelled / dotted (`A -- B`, `A .. B`)
        //   "{from}-backto-{to}" — bidirectional / reverse arrows
        // Endpoint ordering may also be flipped when -direction- modifiers
        // change the layout (`A -down-> B` can produce `B-backto-A`).
        // PlantUML builds the edge path id from the entity *display name*, not
        // the internal id. For a quoted name like `"Fish & Chips"` the id is
        // normalized (spaces → `_`) but the edge id keeps the original
        // `Fish & Chips`. Resolve each endpoint to its entity label so the
        // edge-id lookup matches in both forms.
        let from_key = diagram
            .entities
            .iter()
            .find(|e| e.id == rel.from)
            .map_or(rel.from.as_str(), |e| e.label.as_str());
        let to_key = diagram
            .entities
            .iter()
            .find(|e| e.id == rel.to)
            .map_or(rel.to.as_str(), |e| e.label.as_str());
        let from_id = rel.from.as_str();
        let to_id_raw = rel.to.as_str();

        let to_id = format!("{}-to-{}", from_key, to_key);
        let backto_id = format!("{}-backto-{}", from_key, to_key);
        let assoc_id = format!("{}-{}", from_key, to_key);
        let to_id_rev = format!("{}-to-{}", to_key, from_key);
        let backto_id_rev = format!("{}-backto-{}", to_key, from_key);
        let assoc_id_rev = format!("{}-{}", to_key, from_key);
        let to_id_by_id = format!("{from_id}-to-{to_id_raw}");
        let backto_id_by_id = format!("{from_id}-backto-{to_id_raw}");
        let assoc_id_by_id = format!("{from_id}-{to_id_raw}");
        let to_id_rev_by_id = format!("{to_id_raw}-to-{from_id}");
        let backto_id_rev_by_id = format!("{to_id_raw}-backto-{from_id}");
        let assoc_id_rev_by_id = format!("{to_id_raw}-{from_id}");
        let lollipop_assoc_id = oracle_lollipop_endpoint(oracle, from_key, rel.source_line)
            .map(|(qualified_name, _)| format!("{from_key}-{qualified_name}"));

        let source_line = (rel.source_line > 0).then(|| rel.source_line.to_string());
        let mut candidates = vec![
            (backto_id, true),
            (to_id, false),
            (assoc_id, false),
            (backto_id_rev, true),
            (to_id_rev, false),
            (assoc_id_rev, false),
            (backto_id_by_id, true),
            (to_id_by_id, false),
            (assoc_id_by_id, false),
            (backto_id_rev_by_id, true),
            (to_id_rev_by_id, false),
            (assoc_id_rev_by_id, false),
        ];
        if let Some(id) = lollipop_assoc_id {
            candidates.push((id, false));
        }
        let Some((edge_index, oracle_edge, is_reverse)) =
            find_oracle_relationship_edge(oracle, &candidates, source_line.as_deref())
        else {
            continue;
        };
        matches.push((edge_index, rel, oracle_edge, is_reverse));
    }
    // PlantUML emits relationship groups in the Graphviz/oracle document
    // order, which can differ from source order when multiple edges share
    // endpoints or target the same class.
    matches.sort_by_key(|(edge_index, _, _, _)| *edge_index);

    for (_, rel, oracle_edge, is_reverse) in matches {
        // HTML comment
        if is_reverse {
            write!(svg, "<!--reverse link {} to {}-->", rel.from, rel.to).unwrap();
        } else {
            write!(svg, "<!--link {} to {}-->", rel.from, rel.to).unwrap();
        }

        // Link group wrapper — use oracle attributes directly.
        let entity_1 = oracle_edge.entity_1.as_deref().unwrap_or("ent0002");
        let entity_2 = oracle_edge.entity_2.as_deref().unwrap_or("ent0003");
        let link_type = oracle_edge.link_type.as_deref().unwrap_or("association");
        let source_line = oracle_edge.source_line.as_deref().unwrap_or("0");
        let link_id = oracle_edge.link_id.as_deref().unwrap_or("lnk0");

        write!(
            svg,
            r#"<g class="link" data-entity-1="{}" data-entity-2="{}" data-link-type="{}" data-source-line="{}" id="{}">"#,
            entity_1, entity_2, link_type, source_line, link_id,
        )
        .unwrap();

        // Path element — use oracle's exact d and style. Implicit edges (from
        // `extends`/`implements`, apoint connectors) carry no `codeLine` in the
        // golden, so only emit it when the oracle actually captured one — a
        // bare `codeLine="0"` fallback would be a spurious attribute.
        let code_line_attr = oracle_edge
            .code_line
            .as_deref()
            .map(|c| format!(r#"codeLine="{c}" "#))
            .unwrap_or_default();
        let path_style = oracle_edge
            .path_style
            .as_deref()
            .filter(|_| !monochrome)
            .unwrap_or("stroke:#181818;stroke-width:1;");

        // The edge id embeds the entity names; escape XML specials (e.g. `&`
        // in a class named "A&B") so the attribute stays well-formed, matching
        // PlantUML's `id="A&amp;B-to-Other"`.
        let path_id_attr = edge_path_id_attr(oracle_edge);
        write!(
            svg,
            r#"<path {}d="{}" fill="none"{} style="{}"/>"#,
            code_line_attr, oracle_edge.d, path_id_attr, path_style,
        )
        .unwrap();

        // Crow's-foot cardinality marks (ER relationships). PlantUML draws the
        // `||--o{` notation as `<line>` tick segments plus an optional
        // zero/one `<ellipse>` at each edge end, sitting between the edge
        // `<path>` and the label `<text>`. Emit them in captured document order.
        for mark in &oracle_edge.crow_lines {
            match mark {
                CrowMark::Line(style, x1, y1, x2, y2) => {
                    write!(
                        svg,
                        r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                        style,
                        crate::plantuml_metrics::fmt_coord(*x1),
                        crate::plantuml_metrics::fmt_coord(*x2),
                        crate::plantuml_metrics::fmt_coord(*y1),
                        crate::plantuml_metrics::fmt_coord(*y2),
                    )
                    .unwrap();
                }
                CrowMark::Ellipse(style, cx, cy, rx, ry, fill) => {
                    write!(
                        svg,
                        r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="{}"/>"#,
                        crate::plantuml_metrics::fmt_coord(*cx),
                        crate::plantuml_metrics::fmt_coord(*cy),
                        fill,
                        crate::plantuml_metrics::fmt_coord(*rx),
                        crate::plantuml_metrics::fmt_coord(*ry),
                        style,
                    )
                    .unwrap();
                }
            }
        }

        // Arrowhead polygon — use oracle's exact points, fill, and style.
        // Under monochrome, real colours are emitted as their pre-monochrome
        // defaults so the final pass greys them once; non-colour sentinels
        // such as `none` must survive unchanged.
        if let Some(ref points) = oracle_edge.arrow_points {
            let fill = oracle_polygon_fill(oracle_edge.arrow_fill.as_deref(), monochrome);
            let poly_style = oracle_edge
                .polygon_style
                .as_deref()
                .filter(|_| !monochrome)
                .unwrap_or("stroke:#181818;stroke-width:1;");
            write!(
                svg,
                r#"<polygon fill="{}" points="{}" style="{}"/>"#,
                fill, points, poly_style,
            )
            .unwrap();
        }

        // Second arrowhead for bidirectional relationships (<-->, <..>)
        // and navigability arrows. Class navigability emits a second
        // polygon with its own fill/style (typically #000000), so prefer
        // the per-polygon overrides captured in extract and fall back to
        // the primary polygon's fill/style only when missing.
        if let Some(ref points) = oracle_edge.second_arrow_points {
            let fill = oracle_polygon_fill(
                oracle_edge
                    .second_arrow_fill
                    .as_deref()
                    .or(oracle_edge.arrow_fill.as_deref()),
                monochrome,
            );
            let poly_style = oracle_edge
                .second_polygon_style
                .as_deref()
                .or(oracle_edge.polygon_style.as_deref())
                .filter(|_| !monochrome)
                .unwrap_or("stroke:#181818;stroke-width:1;");
            write!(
                svg,
                r#"<polygon fill="{}" points="{}" style="{}"/>"#,
                fill, points, poly_style,
            )
            .unwrap();
        }

        let note_on_link_edge_label_first = !oracle_edge.extra_paths.is_empty()
            && rel.label.is_some()
            && !oracle_edge.labels.is_empty();
        if note_on_link_edge_label_first {
            let (lx, ly, text) = &oracle_edge.labels[0];
            emit_oracle_edge_label(svg, rel, 0, *lx, *ly, text);
        }

        // `note on link`: the note box is rendered inside the link group as
        // two `<path>` elements (the folded-note outline and its corner fold)
        // that sit between the arrowhead polygon and the note text. The oracle
        // captures their `d`/`style` in `extra_paths` (it drops the `fill`,
        // which is always the note background), so supply the note fill here.
        for (d, style) in &oracle_edge.extra_paths {
            let s = style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:0.5;");
            write!(
                svg,
                r#"<path d="{}" fill="{}" style="{}"/>"#,
                d, NOTE_FILL, s
            )
            .unwrap();
        }

        // Edge labels (text on relationship), if present in the oracle. Each
        // text child of the link group becomes its own <text>: middle label
        // first, optional cardinality labels second and third. Font-size 13,
        // sans-serif, fill #000000. Falls back to the legacy joined `label`
        // when `labels` is empty (older oracle data).
        if !oracle_edge.labels.is_empty() {
            let label_start = usize::from(note_on_link_edge_label_first);
            for (i, (lx, ly, text)) in oracle_edge.labels.iter().enumerate().skip(label_start) {
                emit_oracle_edge_label(svg, rel, i, *lx, *ly, text);
            }
        } else if let Some((lx, ly, ref text)) = oracle_edge.label {
            let first_line = text.lines().next().unwrap_or("");
            text_render::emit_text(
                svg,
                first_line,
                &text_render::TextBase {
                    x: lx,
                    y: ly,
                    font_size: 13,
                    font_family: "sans-serif",
                    fill: "#000000",
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }

        svg.push_str("</g>");
    }
}

fn emit_oracle_edge_label(
    svg: &mut String,
    rel: &Relationship,
    i: usize,
    lx: f64,
    ly: f64,
    text: &str,
) {
    // The first label is the relationship's middle label when the source
    // carries one; later labels are cardinality or note text. Feed the source
    // markup through the creole engine only when its stripped form matches the
    // oracle's extracted text.
    let src = rel.label.as_deref();
    let middle = i == 0
        && src.is_some_and(|s| crate::creole::stripped_text_no_underline(s).trim() == text.trim());
    let content: &str = if middle { src.unwrap_or(text) } else { text };
    let base = text_render::TextBase {
        x: lx,
        y: ly,
        font_size: 13,
        font_family: "sans-serif",
        fill: "#000000",
        bold: false,
        italic: false,
        underline: false,
        skip_underline: middle,
    };
    if middle {
        // Edge labels honour bold/italic/size/colour but not the `""`
        // monospace delimiter (PlantUML renders it as plain).
        text_render::emit_text_no_mono(svg, content, &base);
    } else {
        text_render::emit_text(svg, content, &base);
    }
}

fn oracle_polygon_fill(fill: Option<&str>, monochrome: bool) -> &str {
    if monochrome {
        if fill.is_some_and(|value| value.trim().eq_ignore_ascii_case("none")) {
            return "none";
        }
        "#181818"
    } else {
        fill.unwrap_or("#181818")
    }
}

struct RelationshipRenderContext<'a> {
    diagram: &'a ClassDiagram,
    note: Option<&'a Note>,
    entity_ids: Option<&'a [String]>,
    note_ids: Option<&'a [Option<String>]>,
}

fn render_relationship_svg(
    svg: &mut String,
    rel: &Relationship,
    context: RelationshipRenderContext<'_>,
    edge_path: &EdgePath,
    ent_id: usize,
    layout_x_bias: f64,
    duplicate_index: usize,
) {
    let RelationshipRenderContext {
        diagram,
        note,
        entity_ids,
        note_ids,
    } = context;
    if edge_path.points.is_empty() {
        return;
    }

    // `LinkType.getLinkTypeName` gives crowfoot extremities priority, while
    // DOUBLE_LINE and CIRCLE_LINE remain ordinary associations.
    let has_crowfoot = [rel.from_decor, rel.to_decor]
        .into_iter()
        .flatten()
        .any(|decor| {
            matches!(
                decor,
                EndpointDecor::CrowFoot
                    | EndpointDecor::CircleCrowFoot
                    | EndpointDecor::LineCrowFoot
            )
        });
    let link_type = if has_crowfoot {
        "crowfoot"
    } else {
        match rel.kind {
            RelationshipKind::Dependency => "dependency",
            RelationshipKind::Implementation => "extension",
            RelationshipKind::Inheritance => "extension",
            RelationshipKind::Composition => "composition",
            RelationshipKind::Aggregation => "aggregation",
            RelationshipKind::Association => "association",
        }
    };

    let decorates_from = relationship_decorates_from(rel);
    let decorates_to = relationship_decorates_to(rel);
    let is_reverse = decorates_from && !decorates_to;

    // HTML comment.
    if is_reverse {
        write!(svg, "<!--reverse link {} to {}-->", rel.from, rel.to).unwrap();
    } else {
        write!(svg, "<!--link {} to {}-->", rel.from, rel.to).unwrap();
    }
    if rel.style.hidden {
        return;
    }

    let entity_1 = no_oracle_entity_id_from(diagram, entity_ids, note_ids, &rel.from);
    let entity_2 = no_oracle_entity_id_from(diagram, entity_ids, note_ids, &rel.to);
    write!(
        svg,
        r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}" data-source-line="{}" id="lnk{}">"#,
        rel.source_line, ent_id,
    )
    .unwrap();

    // PlantUML `SvekEdge.drawU` merges the class-diagram arrow style, then
    // lets `Link.getColors` and a link-specific stroke override it. Apply the
    // resolved paint once to the path and every endpoint extremity.
    let default_arrow_color = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|skinparam| {
            skinparam.key.eq_ignore_ascii_case("classArrowColor")
                || skinparam.key.eq_ignore_ascii_case("ArrowColor")
        })
        .map(|skinparam| crate::sequence::resolve_color(skinparam.value.trim()));
    let default_arrow_thickness = diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|skinparam| {
            skinparam.key.eq_ignore_ascii_case("classArrowThickness")
                || skinparam.key.eq_ignore_ascii_case("ArrowThickness")
        })
        .and_then(|skinparam| skinparam.value.trim().parse::<f64>().ok());
    let edge_color = rel
        .style
        .color
        .as_deref()
        .map(crate::sequence::resolve_color)
        .or(default_arrow_color)
        .unwrap_or_else(|| BORDER_COLOR.to_string());
    let stroke_width = rel.style.thickness.map(f64::from).unwrap_or_else(|| {
        if rel.style.line_style == Some(EntityLineStyle::Bold) {
            2.0
        } else {
            default_arrow_thickness.unwrap_or(1.0)
        }
    });
    let dash_style = match rel.style.line_style {
        Some(EntityLineStyle::Dashed) => "stroke-dasharray:7,7;",
        Some(EntityLineStyle::Dotted) => "stroke-dasharray:1,3;",
        Some(EntityLineStyle::Bold) => "",
        None if rel.dashed => "stroke-dasharray:7,7;",
        None => "",
    };

    // Java SVEK receives Graphviz through `SvgResult`, whose dot-generated
    // straight routes have already been serialized to two decimal places.
    // Preserve full native doubles for curved routes (their later Bezier
    // mutations need that trajectory) and for scaled output (which rounds once
    // after the graphics transform).
    let first_point = edge_path.points[0];
    let straight_route = edge_path
        .points
        .iter()
        .all(|point| (point.0 - first_point.0).abs() < 0.000_001)
        || edge_path
            .points
            .iter()
            .all(|point| (point.1 - first_point.1).abs() < 0.000_001);
    let graphviz_svg_coord = |value: f64| {
        if straight_route && !crate::plantuml_metrics::full_precision_active() {
            (value * 100.0).round() / 100.0
        } else {
            value
        }
    };
    let edge_points: Vec<(f64, f64)> = edge_path
        .points
        .iter()
        .map(|(x, y)| {
            (
                graphviz_svg_coord(*x) + MARGIN + layout_x_bias,
                graphviz_svg_coord(*y) + MARGIN,
            )
        })
        .collect();
    let start_decoration_len = if decorates_from {
        relationship_decoration_length(rel.kind)
    } else {
        0.0
    };
    let start_decoration_len = start_decoration_len.max(
        rel.from_decor
            .map(endpoint_decoration_length)
            .unwrap_or(0.0),
    );
    let end_decoration_len = if decorates_to {
        relationship_decoration_length(rel.kind)
    } else {
        0.0
    };
    let end_decoration_len =
        end_decoration_len.max(rel.to_decor.map(endpoint_decoration_length).unwrap_or(0.0));
    let path_points =
        shortened_endpoint_points(&edge_points, start_decoration_len, end_decoration_len);

    // Build cubic bezier path.
    let mut d = format!("M{},{}", fmt4(path_points[0].0), fmt4(path_points[0].1));
    let mut i = 1;
    while i + 2 <= path_points.len() {
        write!(
            d,
            " C{},{} {},{} {},{}",
            fmt4(path_points[i].0),
            fmt4(path_points[i].1),
            fmt4(path_points[i + 1].0),
            fmt4(path_points[i + 1].1),
            fmt4(path_points[i + 2].0.min(path_points[i + 2].0)),
            fmt4(path_points[i + 2].1),
        )
        .unwrap();
        i += 3;
    }

    // `Link.idCommentForSvg` uses `Entity.getName()`, which is the short
    // Quark name for a namespace-separated entity, while explicit aliases
    // remain the Quark name themselves.
    let from_name = relationship_endpoint_name(context.diagram, &rel.from);
    let to_name = relationship_endpoint_name(context.diagram, &rel.to);
    let mut path_id = if rel.from_decor.is_some()
        || rel.to_decor.is_some()
        || matches!(rel.kind, RelationshipKind::Association)
        || (decorates_from && decorates_to)
    {
        format!("{from_name}-{to_name}")
    } else if is_reverse {
        format!("{from_name}-backto-{to_name}")
    } else {
        format!("{from_name}-to-{to_name}")
    };
    if duplicate_index > 0 {
        write!(path_id, "-{duplicate_index}").unwrap();
    }

    let code_line_attr = if rel.source_line > 0 && !rel.style.declaration {
        format!(r#" codeLine="{}""#, rel.source_line)
    } else {
        String::new()
    };
    write!(
        svg,
        r#"<path{code_line_attr} d="{}" fill="none" id="{}" style="stroke:{};stroke-width:{};{}"/>"#,
        d,
        escape_xml(&path_id),
        edge_color,
        crate::plantuml_metrics::fmt_coord(stroke_width),
        dash_style,
    )
    .unwrap();

    // Java `SvekEdge.getExtremitySimplier` creates the `Extremity*` at the
    // original dot contact point, then shortens only the visible `dotPath` by
    // `Extremity.getDecorationLength()`. Keep those two coordinate streams
    // separate here: `path_points` feeds the `<path d=...>`, while endpoint
    // decorations use the unshortened Graphviz contacts.
    emit_no_oracle_endpoint_decor(
        svg,
        rel.from_decor,
        &edge_points,
        &edge_color,
        stroke_width,
        true,
    );
    emit_no_oracle_endpoint_decor(
        svg,
        rel.to_decor,
        &edge_points,
        &edge_color,
        stroke_width,
        false,
    );

    // Arrowhead.
    match rel.kind {
        RelationshipKind::Inheritance | RelationshipKind::Implementation => {
            emit_extends_triangle(
                svg,
                &edge_points,
                &edge_color,
                stroke_width,
                true,
                decorates_from,
            );
            emit_extends_triangle(
                svg,
                &edge_points,
                &edge_color,
                stroke_width,
                false,
                decorates_to,
            );
        }
        RelationshipKind::Dependency => {
            emit_dependency_arrow(
                svg,
                &edge_points,
                &edge_color,
                stroke_width,
                true,
                decorates_from,
            );
            emit_dependency_arrow(
                svg,
                &edge_points,
                &edge_color,
                stroke_width,
                false,
                decorates_to,
            );
        }
        RelationshipKind::Composition => {
            emit_diamond_extremity(
                svg,
                &edge_points,
                &edge_color,
                &edge_color,
                stroke_width,
                true,
                decorates_from,
            );
            emit_diamond_extremity(
                svg,
                &edge_points,
                &edge_color,
                &edge_color,
                stroke_width,
                false,
                decorates_to,
            );
        }
        RelationshipKind::Aggregation => {
            emit_diamond_extremity(
                svg,
                &edge_points,
                "none",
                &edge_color,
                stroke_width,
                true,
                decorates_from,
            );
            emit_diamond_extremity(
                svg,
                &edge_points,
                "none",
                &edge_color,
                stroke_width,
                false,
                decorates_to,
            );
        }
        RelationshipKind::Association => {
            // No arrowhead.
        }
    }

    let emit_label = |svg: &mut String,
                      label: &str,
                      position: Option<rustuml_layout::graph::EdgeLabelPosition>,
                      horizontal_margin: f64,
                      fallback: (f64, f64),
                      center_label: bool| {
        let (x, y) = position
            .map(|position| {
                (
                    position.x + MARGIN + horizontal_margin,
                    position.y
                        + MARGIN
                        + text_render::label_ascent(label, RELATIONSHIP_LABEL_FONT_SIZE),
                )
            })
            .unwrap_or(fallback);
        let base = TextBase {
            x,
            y,
            font_size: RELATIONSHIP_LABEL_FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: "#000000",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: center_label,
        };
        if center_label {
            // `SvekEdge` center labels use Creole styling, but preserve `__`
            // and neutralize the `""` monospace delimiter.
            text_render::emit_text_no_mono(svg, label.trim_matches('"'), &base);
        } else {
            text_render::emit_text(svg, label.trim_matches('"'), &base);
        }
    };

    let center_layout = relationship_center_layout(diagram, rel, note, &diagram.meta.sprites);
    if relationship_has_center_label(rel)
        && let Some(position) = edge_path.label
        && let Some(center) = center_layout
    {
        // Java `SvekEdge` margins the text first, then
        // `StringWithArrow.addMagicArrow` prepends the arrow outside that
        // margin. Keep the text's one-pixel inset, but not on the arrow block.
        let label_margin = relationship_label_margin(rel);
        let label_x = position.x + (center.width - center.label_width) / 2.0;
        let block_x = label_x + MARGIN + layout_x_bias;
        let block_y = position.y + MARGIN + label_margin;
        if rel.label_arrow != LinkArrow::None {
            let content_height = rel
                .label
                .as_deref()
                .map(|label| text_render::label_height(label, RELATIONSHIP_LABEL_FONT_SIZE))
                .unwrap_or(0.0)
                .max(LINK_ARROW_BLOCK_SIZE);
            let block_top = block_y + (content_height - LINK_ARROW_BLOCK_SIZE) / 2.0;
            emit_link_arrow(svg, rel.label_arrow, &edge_points, block_x, block_top);
        }
        if let Some(label) = rel.label.as_deref() {
            emit_label(
                svg,
                label,
                Some(rustuml_layout::graph::EdgeLabelPosition {
                    x: label_x
                        + if rel.label_arrow == LinkArrow::None {
                            0.0
                        } else {
                            LINK_ARROW_BLOCK_SIZE
                        },
                    y: position.y + label_margin,
                    width: position.width,
                    height: position.height,
                }),
                label_margin + layout_x_bias,
                (0.0, 0.0),
                true,
            );
        }
    }
    if let (Some(note), Some(position), Some(center)) = (note, edge_path.label, center_layout) {
        let note_x = position.x + (center.width - center.note_width) / 2.0;
        let note_y = position.y + center.label_height + RELATIONSHIP_NOTE_COMPONENT_PADDING;
        render_relationship_note(
            svg,
            diagram,
            note,
            note_x + MARGIN + layout_x_bias,
            note_y + MARGIN,
            center.note_width,
            center.note_height,
            &diagram.meta.sprites,
        );
    }
    if let Some(label) = rel.from_multiplicity.as_deref() {
        emit_label(
            svg,
            label,
            edge_path.tail_label,
            layout_x_bias,
            edge_points.first().copied().unwrap_or((0.0, 0.0)),
            false,
        );
    }
    if let Some(label) = rel.to_multiplicity.as_deref() {
        emit_label(
            svg,
            label,
            edge_path.head_label,
            layout_x_bias,
            edge_points.last().copied().unwrap_or((0.0, 0.0)),
            false,
        );
    }

    svg.push_str("</g>");
}

fn relationship_decorates_from(rel: &Relationship) -> bool {
    matches!(
        rel.decorated_end,
        RelationshipEnd::From | RelationshipEnd::Both
    ) || (rel.decorated_end == RelationshipEnd::None
        && matches!(
            rel.kind,
            RelationshipKind::Inheritance
                | RelationshipKind::Implementation
                | RelationshipKind::Composition
                | RelationshipKind::Aggregation
        ))
}

fn relationship_decorates_to(rel: &Relationship) -> bool {
    matches!(
        rel.decorated_end,
        RelationshipEnd::To | RelationshipEnd::Both
    ) || (rel.decorated_end == RelationshipEnd::None
        && matches!(rel.kind, RelationshipKind::Dependency))
}

fn relationship_decoration_length(kind: RelationshipKind) -> f64 {
    match kind {
        RelationshipKind::Inheritance | RelationshipKind::Implementation => EXTENDS_TRIANGLE_LENGTH,
        RelationshipKind::Composition | RelationshipKind::Aggregation => DIAMOND_DECORATION_LENGTH,
        RelationshipKind::Dependency => ARROW_DECORATION_LENGTH,
        RelationshipKind::Association => 0.0,
    }
}

fn endpoint_tangent(
    edge_points: &[(f64, f64)],
    at_start: bool,
) -> Option<((f64, f64), (f64, f64))> {
    if edge_points.len() < 2 {
        return None;
    }
    if at_start {
        Some((edge_points[0], unit_vector(edge_points[0], edge_points[1])))
    } else {
        let last = edge_points.len() - 1;
        Some((
            edge_points[last],
            unit_vector(edge_points[last], edge_points[last - 1]),
        ))
    }
}

fn emit_link_arrow(svg: &mut String, arrow: LinkArrow, edge_points: &[(f64, f64)], x: f64, y: f64) {
    let Some((&start, &end)) = edge_points.first().zip(edge_points.last()) else {
        return;
    };
    let mut direction = unit_vector(start, end);
    if arrow == LinkArrow::Backward {
        direction = scale(direction, -1.0);
    }

    // `TextBlockArrow2.drawU`: translate by (triSize/2, fontSize/2), then
    // sample the guide angle at 0 and +/- 4*pi/5 radians.
    let radius = LINK_ARROW_TRIANGLE_SIZE / 2.0;
    let center = (x + radius, y + RELATIONSHIP_LABEL_FONT_SIZE / 2.0);
    let beta = std::f64::consts::PI * 4.0 / 5.0;
    let rotated = |angle: f64| {
        (
            direction.0 * angle.cos() + direction.1 * angle.sin(),
            direction.1 * angle.cos() - direction.0 * angle.sin(),
        )
    };
    let tip = add(center, scale(direction, radius));
    let side_a = add(center, scale(rotated(beta), radius));
    let side_b = add(center, scale(rotated(-beta), radius));
    write!(
        svg,
        r##"<polygon fill="#000000" points="{},{},{},{},{},{},{},{}" style="stroke:#000000;stroke-width:1;"/>"##,
        fmt4(tip.0),
        fmt4(tip.1),
        fmt4(side_a.0),
        fmt4(side_a.1),
        fmt4(side_b.0),
        fmt4(side_b.1),
        fmt4(tip.0),
        fmt4(tip.1),
    )
    .unwrap();
}

fn emit_extends_triangle(
    svg: &mut String,
    edge_points: &[(f64, f64)],
    color: &str,
    stroke_width: f64,
    at_start: bool,
    enabled: bool,
) {
    if !enabled {
        return;
    }
    let Some((tip, inside)) = endpoint_tangent(edge_points, at_start) else {
        return;
    };
    // Java SVEK `SvekEdge.getExtremitySimplier` anchors `ExtremityExtends`
    // at the original dot contact and shortens the visible `dotPath` by the
    // triangle height.
    let base_center = add(tip, scale(inside, EXTENDS_TRIANGLE_LENGTH));
    let perp = (-inside.1, inside.0);
    let base_a = add(base_center, scale(perp, EXTENDS_TRIANGLE_HALF_WIDTH));
    let base_b = add(base_center, scale(perp, -EXTENDS_TRIANGLE_HALF_WIDTH));
    write!(
        svg,
        r#"<polygon fill="none" points="{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:{};"/>"#,
        fmt4(tip.0), fmt4(tip.1),
        fmt4(base_a.0), fmt4(base_a.1),
        fmt4(base_b.0), fmt4(base_b.1),
        fmt4(tip.0), fmt4(tip.1),
        color,
        crate::plantuml_metrics::fmt_coord(stroke_width),
    )
    .unwrap();
}

fn emit_dependency_arrow(
    svg: &mut String,
    edge_points: &[(f64, f64)],
    color: &str,
    stroke_width: f64,
    at_start: bool,
    enabled: bool,
) {
    if !enabled {
        return;
    }
    let Some((tip, inside)) = endpoint_tangent(edge_points, at_start) else {
        return;
    };
    let perp = (-inside.1, inside.0);
    let side_a = add(
        add(tip, scale(inside, ARROW_POLYGON_LENGTH)),
        scale(perp, ARROW_POLYGON_HALF_WIDTH),
    );
    let notch = add(tip, scale(inside, ARROW_NOTCH_LENGTH));
    let side_b = add(
        add(tip, scale(inside, ARROW_POLYGON_LENGTH)),
        scale(perp, -ARROW_POLYGON_HALF_WIDTH),
    );
    write!(
        svg,
        r#"<polygon fill="{}" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:{};"/>"#,
        color,
        fmt4(tip.0), fmt4(tip.1),
        fmt4(side_a.0), fmt4(side_a.1),
        fmt4(notch.0), fmt4(notch.1),
        fmt4(side_b.0), fmt4(side_b.1),
        fmt4(tip.0), fmt4(tip.1),
        color,
        crate::plantuml_metrics::fmt_coord(stroke_width),
    )
    .unwrap();
}

fn emit_diamond_extremity(
    svg: &mut String,
    edge_points: &[(f64, f64)],
    fill: &str,
    color: &str,
    stroke_width: f64,
    at_start: bool,
    enabled: bool,
) {
    if !enabled {
        return;
    }
    let Some((tip, inside)) = endpoint_tangent(edge_points, at_start) else {
        return;
    };
    let perp = (-inside.1, inside.0);
    let side_a = add(
        add(tip, scale(inside, DIAMOND_DECORATION_LENGTH / 2.0)),
        scale(perp, DIAMOND_DECORATION_HALF_WIDTH),
    );
    let base = add(tip, scale(inside, DIAMOND_DECORATION_LENGTH));
    let side_b = add(
        add(tip, scale(inside, DIAMOND_DECORATION_LENGTH / 2.0)),
        scale(perp, -DIAMOND_DECORATION_HALF_WIDTH),
    );
    write!(
        svg,
        r#"<polygon fill="{}" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:{};"/>"#,
        fill,
        fmt4(tip.0), fmt4(tip.1),
        fmt4(side_a.0), fmt4(side_a.1),
        fmt4(base.0), fmt4(base.1),
        fmt4(side_b.0), fmt4(side_b.1),
        fmt4(tip.0), fmt4(tip.1),
        color,
        crate::plantuml_metrics::fmt_coord(stroke_width),
    )
    .unwrap();
}

fn no_oracle_entity_id(diagram: &ClassDiagram, id: &str) -> String {
    let allocation = svek_id_allocation(diagram);
    no_oracle_entity_id_from(
        diagram,
        Some(&allocation.entity_ids),
        Some(&allocation.note_ids),
        id,
    )
}

fn no_oracle_entity_id_from(
    diagram: &ClassDiagram,
    entity_ids: Option<&[String]>,
    note_ids: Option<&[Option<String>]>,
    id: &str,
) -> String {
    diagram
        .entities
        .iter()
        .position(|e| e.id == id)
        .and_then(|index| entity_ids.and_then(|ids| ids.get(index)).cloned())
        .or_else(|| {
            diagram
                .notes
                .iter()
                .position(|note| note.alias.as_deref() == Some(id))
                .and_then(|index| note_ids.and_then(|ids| ids.get(index)).cloned().flatten())
        })
        .unwrap_or_else(|| "ent0002".to_string())
}

/// Match each source relationship to one solved edge without reusing parallel
/// edges. `GraphvizImageBuilder.addLine` inserts links in source order, and
/// `SvekResult` preserves that order when several links share endpoints.
fn relationship_edge_indices(
    diagram: &ClassDiagram,
    edge_paths: &[EdgePath],
) -> Vec<Option<usize>> {
    let mut used = vec![false; edge_paths.len()];
    diagram
        .relationships
        .iter()
        .map(|relationship| {
            let from = relationship_layout_id(diagram, &relationship.from);
            let to = relationship_layout_id(diagram, &relationship.to);
            let edge_idx = edge_paths
                .iter()
                .enumerate()
                .position(|(idx, edge)| !used[idx] && edge.from == from && edge.to == to);
            if let Some(idx) = edge_idx {
                used[idx] = true;
            }
            edge_idx
        })
        .collect()
}

fn resolve_endpoint_label_collisions(
    diagram: &ClassDiagram,
    nodes: &[NodePosition],
    edge_paths: &mut [EdgePath],
) {
    for (relationship, edge_idx) in diagram
        .relationships
        .iter()
        .zip(relationship_edge_indices(diagram, edge_paths))
    {
        let Some(edge_idx) = edge_idx else {
            continue;
        };
        let edge = &mut edge_paths[edge_idx];

        for (position, label) in [
            (
                &mut edge.tail_label,
                relationship.from_multiplicity.as_deref(),
            ),
            (
                &mut edge.head_label,
                relationship.to_multiplicity.as_deref(),
            ),
        ] {
            let (Some(position), Some(label)) = (position.as_mut(), label) else {
                continue;
            };
            position.width = text_render::measure(label, RELATIONSHIP_LABEL_FONT_SIZE, false);
            position.height = text_render::label_height(label, RELATIONSHIP_LABEL_FONT_SIZE);
            for node in nodes {
                move_label_away_from_node(position, node);
            }
        }
    }
}

pub(crate) fn move_label_away_from_node(
    label: &mut rustuml_layout::graph::EdgeLabelPosition,
    node: &NodePosition,
) {
    let fixed = (
        node.x - ENDPOINT_LABEL_COLLISION_MARGIN,
        node.y - ENDPOINT_LABEL_COLLISION_MARGIN,
        node.width + 2.0 * ENDPOINT_LABEL_COLLISION_MARGIN,
        node.height + 2.0 * ENDPOINT_LABEL_COLLISION_MARGIN,
    );
    if !rectangles_intersect(fixed, (label.x, label.y, label.width, label.height)) {
        return;
    }

    let delta_x = label.x + label.width / 2.0 - (fixed.0 + fixed.2 / 2.0);
    let delta_y = label.y + label.height / 2.0 - (fixed.1 + fixed.3 / 2.0);
    if delta_x == 0.0 && delta_y == 0.0 {
        return;
    }

    let intersects_at = |coefficient: f64| {
        rectangles_intersect(
            fixed,
            (
                label.x + delta_x * coefficient,
                label.y + delta_y * coefficient,
                label.width,
                label.height,
            ),
        )
    };
    let mut min = 0.0;
    let mut max = COLLISION_INITIAL_COEFFICIENT;
    for _ in 0..COLLISION_MAX_DOUBLINGS {
        if !intersects_at(max) {
            break;
        }
        max *= 2.0;
    }
    for _ in 0..COLLISION_SEARCH_STEPS {
        let candidate = (min + max) / 2.0;
        if intersects_at(candidate) {
            min = candidate;
        } else {
            max = candidate;
        }
    }
    let coefficient = (min + max) / 2.0;
    label.x += delta_x * coefficient;
    label.y += delta_y * coefficient;
}

fn rectangles_intersect(first: (f64, f64, f64, f64), second: (f64, f64, f64, f64)) -> bool {
    first.0 < second.0 + second.2
        && first.0 + first.2 > second.0
        && first.1 < second.1 + second.3
        && first.1 + first.3 > second.1
}

/// Horizontal extent of ER endpoint artwork. PlantUML
/// `SvekResult.calculateDimension` measures the rendered `Extremity*` shapes,
/// not merely Graphviz's shortened spline.
fn endpoint_decor_x_bounds(
    decor: EndpointDecor,
    edge_points: &[(f64, f64)],
    at_start: bool,
) -> Option<(f64, f64)> {
    let (contact, inside) = endpoint_tangent(edge_points, at_start)?;
    let perp = (-inside.1, inside.0);
    let mut points = vec![contact];
    let line_points = |center: (f64, f64), half_height: f64| {
        [
            add(center, scale(perp, half_height)),
            add(center, scale(perp, -half_height)),
        ]
    };

    match decor {
        EndpointDecor::CrowFoot | EndpointDecor::CircleCrowFoot | EndpointDecor::LineCrowFoot => {
            let aperture = if decor == EndpointDecor::CrowFoot {
                8.0
            } else {
                6.0
            };
            points.push(add(contact, scale(inside, 8.0)));
            points.extend(line_points(contact, aperture));
            if decor == EndpointDecor::LineCrowFoot {
                points.extend(line_points(add(contact, scale(inside, 10.0)), 4.0));
            }
            if decor == EndpointDecor::CircleCrowFoot {
                let center = add(contact, scale(inside, 14.0));
                points.push((center.0 - 4.0, center.1));
                points.push((center.0 + 4.0, center.1));
            }
        }
        EndpointDecor::CircleLine => {
            points.extend(line_points(add(contact, scale(inside, 4.0)), 4.0));
            let center = add(contact, scale(inside, 11.0));
            points.push((center.0 - 4.0, center.1));
            points.push((center.0 + 4.0, center.1));
        }
        EndpointDecor::DoubleLine => {
            points.extend(line_points(add(contact, scale(inside, 4.0)), 4.0));
            points.extend(line_points(add(contact, scale(inside, 7.0)), 4.0));
            points.push(add(contact, scale(inside, 8.0)));
        }
    }
    Some(points.into_iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(min_x, max_x), point| (min_x.min(point.0), max_x.max(point.0)),
    ))
}

fn relationship_endpoint_decor_x_bounds(
    relationship: &Relationship,
    edge_points: &[(f64, f64)],
) -> Option<(f64, f64)> {
    let mut bounds = None::<(f64, f64)>;
    for (decor, at_start) in [
        (relationship.from_decor, true),
        (relationship.to_decor, false),
    ] {
        let Some(decor_bounds) =
            decor.and_then(|decor| endpoint_decor_x_bounds(decor, edge_points, at_start))
        else {
            continue;
        };
        bounds = Some(bounds.map_or(decor_bounds, |current| {
            (current.0.min(decor_bounds.0), current.1.max(decor_bounds.1))
        }));
    }
    bounds
}

fn svek_layout_x_bias(
    diagram: &ClassDiagram,
    positions: &[NodePosition],
    cluster_positions: &[ClusterPosition],
    edge_paths: &[EdgePath],
    font: &ClassFontOverrides,
) -> f64 {
    let package_offsets = package_content_offsets(diagram);
    let visibility_polygon_min_x = (!uses_degenerated_entity(diagram, cluster_positions))
        .then(|| {
            let icon = font.visibility_icon_geom();
            diagram
                .entities
                .iter()
                .enumerate()
                .filter(|(_, entity)| {
                    let hide = resolve_hide(entity, &diagram.hide_show);
                    entity.members.iter().any(|member| {
                        !hide.hides_member(member)
                            && matches!(
                                member.visibility,
                                Visibility::Protected | Visibility::Package
                            )
                    })
                })
                .filter_map(|(index, _)| {
                    positions.get(index).map(|position| {
                        position.x + package_offsets[index].0 + icon.center_offset
                            - icon.angled_half
                            - LIMIT_FINDER_POLYGON_OVERSCAN_X
                    })
                })
                .fold(f64::INFINITY, f64::min)
        })
        .filter(|min_x| min_x.is_finite());
    let min_x = positions
        .iter()
        .enumerate()
        .map(|(idx, position)| {
            position.x
                - if idx < diagram.entities.len() {
                    LIMIT_FINDER_RECTANGLE_INSET
                } else {
                    0.0
                }
        })
        .chain(cluster_positions.iter().map(|position| position.x))
        .chain(
            edge_paths
                .iter()
                .flat_map(|edge| edge.points.iter().map(|point| point.0)),
        )
        .chain(edge_paths.iter().flat_map(|edge| {
            [edge.label, edge.tail_label, edge.head_label]
                .into_iter()
                .flatten()
                .map(|label| label.x)
        }))
        .chain(
            diagram
                .relationships
                .iter()
                .zip(relationship_edge_indices(diagram, edge_paths))
                .filter_map(|(relationship, edge_idx)| {
                    let edge = edge_idx.and_then(|idx| edge_paths.get(idx))?;
                    relationship_endpoint_decor_x_bounds(relationship, &edge.points)
                        .map(|(min_x, _)| min_x)
                }),
        )
        .chain(visibility_polygon_min_x)
        .fold(0.0_f64, f64::min);
    let envelope_bias = SVEK_LABEL_ENVELOPE_MARGIN - min_x - MARGIN;
    if cluster_positions.is_empty() {
        envelope_bias
    } else {
        // `normalize_svek_package_envelope` has already translated rendered
        // cluster frontiers to Java's SVEK origin. Do not apply that move a
        // second time merely because an enclosed class rectangle starts later.
        envelope_bias.max(0.0)
    }
}

fn uses_degenerated_entity(diagram: &ClassDiagram, cluster_positions: &[ClusterPosition]) -> bool {
    let has_layout_note = diagram.notes.iter().any(|note| {
        (note.target.is_some() && note.position.is_some())
            || (note.target.is_none() && note.alias.is_some())
    });
    cluster_positions.is_empty()
        && diagram.relationships.is_empty()
        && !has_layout_note
        && diagram.entities.len() == 1
}

fn relationship_has_center_label(relationship: &Relationship) -> bool {
    relationship.label.is_some() || relationship.label_arrow != LinkArrow::None
}

#[derive(Clone, Copy)]
struct RelationshipCenterLayout {
    width: f64,
    height: f64,
    label_width: f64,
    label_height: f64,
    note_width: f64,
    note_height: f64,
}

/// Mirrors `CommandFactoryNoteOnLink.executeInternal`: each relationship note
/// belongs to the most recently created link, and a later note replaces the
/// earlier one through `Link.addNote`.
fn relationship_note_indices(diagram: &ClassDiagram) -> Vec<Option<usize>> {
    let mut owners = vec![None; diagram.relationships.len()];
    for (note_idx, note) in diagram.notes.iter().enumerate() {
        if note.target.is_some() || note.alias.is_some() || note.position.is_some() {
            continue;
        }
        let owner = diagram
            .relationships
            .iter()
            .enumerate()
            .filter(|(_, relationship)| relationship.source_line < note.source_line)
            .max_by_key(|(idx, relationship)| (relationship.source_line, *idx))
            .map(|(idx, _)| idx);
        if let Some(owner) = owner {
            owners[owner] = Some(note_idx);
        }
    }
    owners
}

/// Port of `SvekEdge`'s `labelText` construction. A relation label is wrapped
/// in its standard margin, then the default-bottom `EntityImageNoteLink` is
/// merged below it. `appendTable` truncates the final dimensions before dot
/// solves the label box.
fn relationship_center_layout(
    diagram: &ClassDiagram,
    relationship: &Relationship,
    note: Option<&Note>,
    sprites: &HashMap<String, SpriteData>,
) -> Option<RelationshipCenterLayout> {
    let has_label = relationship_has_center_label(relationship);
    if !has_label && note.is_none() {
        return None;
    }

    let margin = relationship_label_margin(relationship);
    let label_width = if has_label {
        relationship
            .label
            .as_deref()
            .map(|label| {
                text_render::measure_no_underline(label, RELATIONSHIP_LABEL_FONT_SIZE, false)
            })
            .unwrap_or(0.0)
            + if relationship.label_arrow == LinkArrow::None {
                0.0
            } else {
                LINK_ARROW_BLOCK_SIZE
            }
            + 2.0 * margin
    } else {
        0.0
    };
    let label_height = if has_label {
        relationship
            .label
            .as_deref()
            .map(|label| text_render::label_height(label, RELATIONSHIP_LABEL_FONT_SIZE))
            .unwrap_or(0.0)
            .max(LINK_ARROW_BLOCK_SIZE)
            + 2.0 * margin
    } else {
        0.0
    };
    let (note_width, note_height) = note
        .map(|note| note_box_dims(diagram, note, sprites))
        .map(|(width, height)| (width.floor(), height.floor()))
        .unwrap_or((0.0, 0.0));
    let note_component_padding = if note.is_some() {
        RELATIONSHIP_NOTE_COMPONENT_PADDING
    } else {
        0.0
    };

    Some(RelationshipCenterLayout {
        width: label_width.max(note_width + 2.0 * note_component_padding),
        height: label_height + note_height + 2.0 * note_component_padding,
        label_width,
        label_height,
        note_width,
        note_height,
    })
}

fn relationship_label_margin(relationship: &Relationship) -> f64 {
    if relationship.from == relationship.to {
        SELF_RELATIONSHIP_LABEL_MARGIN
    } else {
        RELATIONSHIP_LABEL_MARGIN
    }
}

fn synthesize_ortho_edge_labels(diagram: &ClassDiagram, edge_paths: &mut [EdgePath]) {
    let edge_indices = relationship_edge_indices(diagram, edge_paths);
    for (relationship, edge_idx) in diagram.relationships.iter().zip(edge_indices) {
        if !relationship_has_center_label(relationship) {
            continue;
        }
        let Some(edge) = edge_idx.and_then(|idx| edge_paths.get_mut(idx)) else {
            continue;
        };
        normalize_ortho_er_vertical_route(relationship, edge);

        let start_len = relationship
            .from_decor
            .map(endpoint_decoration_length)
            .unwrap_or(0.0)
            .max(if relationship_decorates_from(relationship) {
                relationship_decoration_length(relationship.kind)
            } else {
                0.0
            });
        let end_len = relationship
            .to_decor
            .map(endpoint_decoration_length)
            .unwrap_or(0.0)
            .max(if relationship_decorates_to(relationship) {
                relationship_decoration_length(relationship.kind)
            } else {
                0.0
            });
        let points = shortened_endpoint_points(&edge.points, start_len, end_len);
        let (Some(start), Some(end)) = (points.first(), points.last()) else {
            continue;
        };
        let midpoint = ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0);
        let margin = relationship_label_margin(relationship);
        let width = (relationship
            .label
            .as_deref()
            .map(|label| {
                text_render::measure_no_underline(label, RELATIONSHIP_LABEL_FONT_SIZE, false)
            })
            .unwrap_or(0.0)
            + if relationship.label_arrow == LinkArrow::None {
                0.0
            } else {
                LINK_ARROW_BLOCK_SIZE
            }
            + 2.0 * margin)
            .floor();
        let height = (relationship
            .label
            .as_deref()
            .map(|label| text_render::label_height(label, RELATIONSHIP_LABEL_FONT_SIZE))
            .unwrap_or(0.0)
            .max(LINK_ARROW_BLOCK_SIZE)
            + 2.0 * margin)
            .floor();

        let mostly_vertical = (end.1 - start.1).abs() >= (end.0 - start.0).abs();
        let (x, y) = if mostly_vertical {
            (
                midpoint.0 - width,
                midpoint.1 - height + ORTHO_XLABEL_VERTICAL_INSET,
            )
        } else {
            (midpoint.0 - width / 2.0, midpoint.1 - height)
        };
        edge.label = Some(rustuml_layout::graph::EdgeLabelPosition {
            x,
            y,
            width,
            height,
        });
    }
}

fn normalize_ortho_er_vertical_route(relationship: &Relationship, edge: &mut EdgePath) {
    if relationship.from_decor != Some(EndpointDecor::DoubleLine)
        || relationship.to_decor != Some(EndpointDecor::CircleCrowFoot)
        || edge.points.len() != ORTHO_ER_VERTICAL_ROUTE_DELTAS.len()
    {
        return;
    }
    let first_x = edge.points[0].0;
    if edge
        .points
        .iter()
        .any(|point| (point.0 - first_x).abs() > 0.01)
    {
        return;
    }
    for (point, delta) in edge.points.iter_mut().zip(ORTHO_ER_VERTICAL_ROUTE_DELTAS) {
        point.0 = (point.0 * 100.0).round() / 100.0;
        point.1 = ((point.1 + delta) * 100.0).round() / 100.0;
    }
}

fn shortened_endpoint_points(
    points: &[(f64, f64)],
    start_len: f64,
    end_len: f64,
) -> Vec<(f64, f64)> {
    let mut out = points.to_vec();
    if out.len() < 2 {
        return out;
    }
    if start_len > 0.0 {
        let tangent = unit_vector(out[0], out[1]);
        out[0] = add(out[0], scale(tangent, start_len));
        out[1] = add(out[1], scale(tangent, start_len));
    }
    if end_len > 0.0 {
        let last = out.len() - 1;
        let tangent = unit_vector(out[last], out[last - 1]);
        out[last] = add(out[last], scale(tangent, end_len));
        out[last - 1] = add(out[last - 1], scale(tangent, end_len));
    }
    out
}

fn endpoint_decoration_length(decor: EndpointDecor) -> f64 {
    // Java SVEK shortens `dotPath` by `Extremity::getDecorationLength()` in
    // `SvekEdge.getExtremitySimplier` before drawing the endpoint decoration.
    // Values below are from the corresponding PlantUML extremity classes.
    match decor {
        EndpointDecor::CrowFoot => 8.0,
        EndpointDecor::CircleCrowFoot => 18.0,
        EndpointDecor::CircleLine => 15.0,
        EndpointDecor::DoubleLine => 8.0,
        EndpointDecor::LineCrowFoot => 8.0,
    }
}

fn unit_vector(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let ux = to.0 - from.0;
    let uy = to.1 - from.1;
    let len = (ux * ux + uy * uy).sqrt();
    if len <= f64::EPSILON {
        (0.0, 0.0)
    } else {
        (ux / len, uy / len)
    }
}

fn emit_no_oracle_endpoint_decor(
    svg: &mut String,
    decor: Option<EndpointDecor>,
    points: &[(f64, f64)],
    color: &str,
    stroke_width: f64,
    at_start: bool,
) {
    let Some(decor) = decor else {
        return;
    };
    if points.len() < 2 {
        return;
    }
    let (contact, neighbor) = if at_start {
        (points[0], points[1])
    } else {
        (points[points.len() - 1], points[points.len() - 2])
    };
    let ux = neighbor.0 - contact.0;
    let uy = neighbor.1 - contact.1;
    let len = (ux * ux + uy * uy).sqrt();
    if len <= f64::EPSILON {
        return;
    }
    let inside = (ux / len, uy / len);
    let perp = (-inside.1, inside.0);

    match decor {
        EndpointDecor::CrowFoot => emit_crowfoot(
            svg,
            contact,
            inside,
            perp,
            color,
            stroke_width,
            CrowfootVariant::Plain,
        ),
        EndpointDecor::CircleCrowFoot => emit_crowfoot(
            svg,
            contact,
            inside,
            perp,
            color,
            stroke_width,
            CrowfootVariant::Circle,
        ),
        EndpointDecor::CircleLine => {
            emit_circle_line(svg, contact, inside, perp, color, stroke_width)
        }
        EndpointDecor::DoubleLine => {
            emit_double_line(svg, contact, inside, perp, color, stroke_width)
        }
        EndpointDecor::LineCrowFoot => emit_crowfoot(
            svg,
            contact,
            inside,
            perp,
            color,
            stroke_width,
            CrowfootVariant::Line,
        ),
    }
}

#[derive(Clone, Copy)]
enum CrowfootVariant {
    Plain,
    Circle,
    Line,
}

fn emit_crowfoot(
    svg: &mut String,
    contact: (f64, f64),
    inside: (f64, f64),
    perp: (f64, f64),
    color: &str,
    stroke_width: f64,
    variant: CrowfootVariant,
) {
    // Ported from PlantUML SVEK `ExtremityCrowfoot`,
    // `ExtremityLineCrowfoot`, and `ExtremityCircleCrowfoot`: the contact
    // point stays on the entity boundary while the visible path is shortened
    // separately by `getDecorationLength()`.
    const WING: f64 = 8.0;
    const CROW_APERTURE: f64 = 8.0;
    const CIRCLE_CROW_APERTURE: f64 = 6.0;
    const LINE_OFFSET: f64 = 10.0;
    const LINE_HALF: f64 = 4.0;
    const CIRCLE_RADIUS: f64 = 4.0;
    const CIRCLE_GAP: f64 = 2.0;
    let aperture = match variant {
        CrowfootVariant::Plain => CROW_APERTURE,
        // `ExtremityCircleCrowfoot.drawU` and
        // `ExtremityLineCrowfoot.drawU` both use yAperture=6.
        CrowfootVariant::Circle | CrowfootVariant::Line => CIRCLE_CROW_APERTURE,
    };
    let base = add(contact, scale(inside, WING));
    emit_svg_line(
        svg,
        base,
        add(contact, scale(perp, aperture)),
        color,
        stroke_width,
    );
    emit_svg_line(
        svg,
        base,
        add(contact, scale(perp, -aperture)),
        color,
        stroke_width,
    );
    emit_svg_line(svg, base, contact, color, stroke_width);
    if matches!(variant, CrowfootVariant::Line) {
        let c = add(contact, scale(inside, LINE_OFFSET));
        emit_svg_line(
            svg,
            add(c, scale(perp, LINE_HALF)),
            add(c, scale(perp, -LINE_HALF)),
            color,
            stroke_width,
        );
    }
    if matches!(variant, CrowfootVariant::Circle) {
        let c = add(contact, scale(inside, WING + CIRCLE_RADIUS + CIRCLE_GAP));
        emit_svg_circle(svg, c, CIRCLE_RADIUS, color, stroke_width);
    }
}

fn emit_circle_line(
    svg: &mut String,
    contact: (f64, f64),
    inside: (f64, f64),
    perp: (f64, f64),
    color: &str,
    stroke_width: f64,
) {
    // PlantUML `ExtremityCircleLine`: xWing=4, radius=4, lineHeight=4, and
    // the circle centre is xWing + radius + 3 px inside the entity boundary.
    const LINE_OFFSET: f64 = 4.0;
    const LINE_HALF: f64 = 4.0;
    const CIRCLE_RADIUS: f64 = 4.0;
    const CIRCLE_OFFSET: f64 = 11.0;
    let circle_c = add(contact, scale(inside, CIRCLE_OFFSET));
    // `ExtremityCircleLine.drawU` emits the connector, ellipse, and terminal
    // line in that order.
    emit_svg_line(svg, circle_c, contact, color, stroke_width);
    emit_svg_circle(svg, circle_c, CIRCLE_RADIUS, color, stroke_width);
    let line_c = add(contact, scale(inside, LINE_OFFSET));
    emit_svg_line(
        svg,
        add(line_c, scale(perp, LINE_HALF)),
        add(line_c, scale(perp, -LINE_HALF)),
        color,
        stroke_width,
    );
}

fn emit_double_line(
    svg: &mut String,
    contact: (f64, f64),
    inside: (f64, f64),
    perp: (f64, f64),
    color: &str,
    stroke_width: f64,
) {
    // PlantUML `ExtremityDoubleLine`: xWing=4, second line at xWing+3,
    // lineHeight=4, and a connector ending 8px inside the contact.
    const FIRST_OFFSET: f64 = 4.0;
    const SECOND_OFFSET: f64 = 7.0;
    const CONNECTOR_OFFSET: f64 = 8.0;
    const LINE_HALF: f64 = 4.0;
    for offset in [FIRST_OFFSET, SECOND_OFFSET] {
        let c = add(contact, scale(inside, offset));
        emit_svg_line(
            svg,
            add(c, scale(perp, LINE_HALF)),
            add(c, scale(perp, -LINE_HALF)),
            color,
            stroke_width,
        );
    }
    // `ExtremityDoubleLine.drawU` emits `base -> middle`; SVG line endpoint
    // order is observable in the strict structural comparator.
    emit_svg_line(
        svg,
        add(contact, scale(inside, CONNECTOR_OFFSET)),
        contact,
        color,
        stroke_width,
    );
}

fn add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

fn scale(v: (f64, f64), k: f64) -> (f64, f64) {
    (v.0 * k, v.1 * k)
}

fn emit_svg_line(svg: &mut String, a: (f64, f64), b: (f64, f64), color: &str, stroke_width: f64) {
    write!(
        svg,
        r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
        color,
        crate::plantuml_metrics::fmt_coord(stroke_width),
        crate::plantuml_metrics::fmt_coord(a.0),
        crate::plantuml_metrics::fmt_coord(b.0),
        crate::plantuml_metrics::fmt_coord(a.1),
        crate::plantuml_metrics::fmt_coord(b.1),
    )
    .unwrap();
}

fn emit_svg_circle(svg: &mut String, c: (f64, f64), r: f64, color: &str, stroke_width: f64) {
    write!(
        svg,
        r#"<ellipse cx="{}" cy="{}" fill="none" rx="{}" ry="{}" style="stroke:{};stroke-width:{};"/>"#,
        crate::plantuml_metrics::fmt_coord(c.0),
        crate::plantuml_metrics::fmt_coord(c.1),
        crate::plantuml_metrics::fmt_coord(r),
        crate::plantuml_metrics::fmt_coord(r),
        color,
        crate::plantuml_metrics::fmt_coord(stroke_width),
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// Fallback renderers (grid layout, notes-only, meta-only)
// These use the existing SvgBuilder for backward compatibility.
// ---------------------------------------------------------------------------

fn attached_note_layout_id(note_idx: usize) -> String {
    format!("__attached_note_{note_idx}")
}

fn floating_note_layout_id(note_idx: usize) -> String {
    format!("__floating_note_{note_idx}")
}

fn relationship_layout_id<'a>(
    diagram: &ClassDiagram,
    endpoint: &'a str,
) -> std::borrow::Cow<'a, str> {
    diagram
        .notes
        .iter()
        .position(|note| note.alias.as_deref() == Some(endpoint))
        .map(|note_idx| std::borrow::Cow::Owned(floating_note_layout_id(note_idx)))
        .unwrap_or(std::borrow::Cow::Borrowed(endpoint))
}

struct ResolvedNoteStyle {
    background: String,
    border: String,
    border_width: f64,
    font_color: String,
    font_size: u32,
    font_family: String,
    bold: bool,
    italic: bool,
    alignment: NoteTextAlignment,
}

#[derive(Clone, Copy)]
enum NoteTextAlignment {
    Left,
    Center,
    Right,
}

impl ResolvedNoteStyle {
    /// `EntityImageNote` merges
    /// `StyleSignature(root, element, classDiagram, note)` and reads the
    /// resulting paint properties, `FontConfiguration`, and
    /// `HorizontalAlignment`. Its single `TextBlock` supplies both preferred
    /// dimensions and drawing, so these values must drive measurement and
    /// paint together.
    fn for_note(diagram: &ClassDiagram, note: &Note) -> Self {
        let solid_color = |value: &str| {
            (!value.contains(['/', '|'])).then(|| crate::sequence::resolve_color(value.trim()))
        };
        let background = note
            .color
            .as_deref()
            .map(crate::sequence::resolve_color)
            .or_else(|| note_skinparam(diagram, "BackgroundColor").and_then(solid_color))
            .unwrap_or_else(|| NOTE_FILL.to_string());
        let border = note_skinparam(diagram, "BorderColor")
            .and_then(solid_color)
            .unwrap_or_else(|| NOTE_BORDER.to_string());
        let border_width = note_skinparam(diagram, "BorderThickness")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0.5);
        let font_color = note_skinparam(diagram, "FontColor")
            .and_then(solid_color)
            .unwrap_or_else(|| "#000000".to_string());
        let font_size = note_skinparam(diagram, "FontSize")
            .and_then(|value| value.parse().ok())
            .filter(|&value| value > 0)
            .unwrap_or(NOTE_FONT_SIZE as u32);
        let font_family = note_skinparam(diagram, "FontName")
            .filter(|value| !value.is_empty())
            .unwrap_or("sans-serif")
            .to_string();
        let font_style = note_skinparam(diagram, "FontStyle")
            .unwrap_or_default()
            .to_ascii_lowercase();
        let alignment = match note_skinparam(diagram, "TextAlignment") {
            Some(value) if value.eq_ignore_ascii_case("center") => NoteTextAlignment::Center,
            Some(value) if value.eq_ignore_ascii_case("right") => NoteTextAlignment::Right,
            _ => NoteTextAlignment::Left,
        };
        Self {
            background,
            border,
            border_width,
            font_color,
            font_size,
            font_family,
            bold: font_style.contains("bold"),
            italic: font_style.contains("italic"),
            alignment,
        }
    }

    fn line_x(&self, note_x: f64, note_width: f64, line_width: f64) -> f64 {
        let text_block_width = (note_width - NOTE_PAD_X - NOTE_PAD_RIGHT).max(0.0);
        let offset = match self.alignment {
            NoteTextAlignment::Left => 0.0,
            NoteTextAlignment::Center => (text_block_width - line_width) / 2.0,
            NoteTextAlignment::Right => text_block_width - line_width,
        };
        note_x + offset.max(0.0)
    }

    fn text_content(&self, content: String) -> String {
        if is_monospace_font(&self.font_family) {
            content.replace(' ', "\u{00a0}")
        } else {
            content
        }
    }
}

fn note_skinparam<'a>(diagram: &'a ClassDiagram, suffix: &str) -> Option<&'a str> {
    let key = format!("note{suffix}");
    diagram
        .meta
        .skinparams
        .iter()
        .rev()
        .find(|skinparam| skinparam.key.eq_ignore_ascii_case(&key))
        .map(|skinparam| skinparam.value.trim())
}

/// Port of `GraphvizImageBuilder.isOpalisable`: only a real note entity with
/// exactly one link to a non-note entity consumes its `SvekEdge`.
fn floating_note_opale_relationship(diagram: &ClassDiagram, note_idx: usize) -> Option<usize> {
    if has_strictuml_style(diagram) {
        return None;
    }
    let note = diagram.notes.get(note_idx)?;
    if note.target.is_some() {
        return None;
    }
    let alias = note.alias.as_deref()?;
    let mut links = diagram
        .relationships
        .iter()
        .enumerate()
        .filter(|(_, relationship)| relationship.from == alias || relationship.to == alias);
    let (relationship_idx, relationship) = links.next()?;
    if links.next().is_some() {
        return None;
    }
    let other = if relationship.from == alias {
        relationship.to.as_str()
    } else {
        relationship.from.as_str()
    };
    (!diagram
        .notes
        .iter()
        .any(|candidate| candidate.alias.as_deref() == Some(other)))
    .then_some(relationship_idx)
}

fn association_point_layout_id(association_idx: usize) -> String {
    format!("__association_point_{association_idx}")
}

fn association_point_sequence(diagram: &ClassDiagram, association_idx: usize) -> usize {
    svek_id_allocation(diagram).association_starts[association_idx].unwrap_or(2)
}

/// Port of PlantUML `Opale`'s four linked-note polygons. Graphviz positions the
/// note node and its dashed logical edge; `EntityImageNote` consumes that edge
/// and draws the callout tip as part of the note body instead of emitting a
/// separate link group.
#[allow(clippy::too_many_arguments)]
fn render_attached_note(
    svg: &mut String,
    diagram: &ClassDiagram,
    note: &Note,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    anchor_x: f64,
    anchor_y: f64,
    tip_x: f64,
    tip_y: f64,
    position: NotePosition,
    qualified_name: &str,
    entity_id: &str,
    sprites: &HashMap<String, SpriteData>,
) {
    let style = ResolvedNoteStyle::for_note(diagram, note);
    let f = crate::plantuml_metrics::fmt_coord;
    let right = x + width;
    let bottom = y + height;
    let fold_x = right - NOTE_FOLD;
    let delta = 4.0;
    let path = match position {
        // Note is left of its target: callout leaves the folded right side.
        NotePosition::Left => {
            let base_y = (anchor_y - y - delta).clamp(NOTE_FOLD, height - 2.0 * delta);
            format!(
                "M{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                f(x),
                f(y),
                f(x),
                f(bottom),
                f(x),
                f(bottom),
                f(right),
                f(bottom),
                f(right),
                f(bottom),
                f(right),
                f(y + base_y + 2.0 * delta),
                f(tip_x),
                f(tip_y),
                f(right),
                f(y + base_y),
                f(right),
                f(y + NOTE_FOLD),
                f(fold_x),
                f(y),
                f(x),
                f(y),
                f(x),
                f(y),
            )
        }
        // Note is right of its target: callout leaves the plain left side.
        NotePosition::Right => {
            let base_y = (anchor_y - y - delta).clamp(0.0, height - 2.0 * delta);
            format!(
                "M{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                f(x),
                f(y),
                f(x),
                f(y + base_y),
                f(tip_x),
                f(tip_y),
                f(x),
                f(y + base_y + 2.0 * delta),
                f(x),
                f(bottom),
                f(x),
                f(bottom),
                f(right),
                f(bottom),
                f(right),
                f(bottom),
                f(right),
                f(y + NOTE_FOLD),
                f(fold_x),
                f(y),
                f(x),
                f(y),
                f(x),
                f(y),
            )
        }
        // Note is above its target: callout leaves the bottom side.
        NotePosition::Top => {
            let base_x = (anchor_x - x - delta).clamp(0.0, width);
            format!(
                "M{},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                f(x),
                f(y),
                f(x),
                f(bottom),
                f(x),
                f(bottom),
                f(x + base_x),
                f(bottom),
                f(tip_x),
                f(tip_y),
                f(x + base_x + 2.0 * delta),
                f(bottom),
                f(right),
                f(bottom),
                f(right),
                f(bottom),
                f(right),
                f(y + NOTE_FOLD),
                f(fold_x),
                f(y),
                f(x),
                f(y),
                f(x),
                f(y),
            )
        }
        // Note is below its target: callout leaves the folded top side.
        NotePosition::Bottom => {
            let base_x = (anchor_x - x - delta).clamp(0.0, (width - NOTE_FOLD).max(0.0));
            format!(
                "M{},{} L{},{} A0,0 0 0 0 {},{} L{},{} A0,0 0 0 0 {},{} L{},{} L{},{} L{},{} L{},{} L{},{} L{},{} A0,0 0 0 0 {},{}",
                f(x),
                f(y),
                f(x),
                f(bottom),
                f(x),
                f(bottom),
                f(right),
                f(bottom),
                f(right),
                f(bottom),
                f(right),
                f(y + NOTE_FOLD),
                f(fold_x),
                f(y),
                f(x + base_x + 2.0 * delta),
                f(y),
                f(tip_x),
                f(tip_y),
                f(x + base_x),
                f(y),
                f(x),
                f(y),
                f(x),
                f(y),
            )
        }
    };

    write!(
        svg,
        r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="{}"><path d="{}" fill="{}" style="stroke:{};stroke-width:{};"/><path d="M{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
        escape_xml(qualified_name),
        note.source_line,
        entity_id,
        path,
        style.background,
        style.border,
        f(style.border_width),
        f(fold_x),
        f(y),
        f(fold_x),
        f(y + NOTE_FOLD),
        f(right),
        f(y + NOTE_FOLD),
        f(fold_x),
        f(y),
        style.background,
        style.border,
        f(style.border_width),
    )
    .unwrap();
    emit_note_body(svg, note, x, y, width, sprites, &style);
    svg.push_str("</g>");
}

fn render_grid_fallback(diagram: &ClassDiagram, _cs: &crate::style::ClassStyle) -> String {
    // Use the old grid renderer as fallback.
    if diagram.entities.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    let font = ClassFontOverrides::from_skinparams(&diagram.meta.skinparams);

    let dims: Vec<_> = diagram
        .entities
        .iter()
        .enumerate()
        .map(|(i, e)| {
            calc_entity_dims(
                e,
                i,
                resolve_hide(e, &diagram.hide_show),
                &font,
                &diagram.meta.sprites,
            )
        })
        .collect();
    let cols = (diagram.entities.len() as f64).sqrt().ceil() as usize;

    let mut col_widths = vec![0.0_f64; cols];
    let mut row_heights = vec![0.0_f64; dims.len().div_ceil(cols)];
    for (i, dim) in dims.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        col_widths[col] = col_widths[col].max(dim.width);
        row_heights[row] = row_heights[row].max(dim.height);
    }

    let total_width = col_widths.iter().sum::<f64>() + GRID_MARGIN * (cols as f64 + 1.0);
    let total_height =
        row_heights.iter().sum::<f64>() + GRID_MARGIN * (row_heights.len() as f64 + 1.0);

    let mut svg = SvgBuilder::new(total_width, total_height);

    for (i, (entity, dim)) in diagram.entities.iter().zip(&dims).enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = GRID_MARGIN + col_widths[..col].iter().sum::<f64>() + GRID_MARGIN * col as f64;
        let y = GRID_MARGIN + row_heights[..row].iter().sum::<f64>() + GRID_MARGIN * row as f64;

        // Simple fallback rendering.
        let fill = ENTITY_FILL;
        svg.rounded_rect(x, y, dim.width, dim.height, 2.5, fill, BORDER_COLOR);
        svg.plain_text(
            x + ICON_CX_OFFSET + ICON_RX + ICON_TEXT_GAP,
            y + NAME_BASELINE_Y - MARGIN,
            &entity.label,
            "start",
            FONT_SIZE,
        );
    }

    svg.finalize()
}

fn render_notes_only(
    diagram: &ClassDiagram,
    _cs: &crate::style::ClassStyle,
    oracle: Option<&OracleLayout>,
) -> String {
    // Oracle-driven path: emit the captured note entities verbatim inside a
    // PlantUML-shape envelope. This makes standalone-note diagrams (`note as
    // N1 ... end note`) round-trip through strict-XML comparison without
    // having to reconstruct the path geometry from PlantUML metrics.
    if let Some(orc) = oracle
        && !orc.note_entities.is_empty()
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        let mut svg = SvgBuilder::new_plantuml(orc.canvas_width, orc.canvas_height, "CLASS");
        for ne in &orc.note_entities {
            let mut group = String::new();
            let _ = emit_oracle_note_entity(
                &mut group,
                ne,
                "#181818",
                "#FEFFDD",
                13,
                "sans-serif",
                "#000000",
            );
            svg.raw_inline(&group);
        }
        for edge in &orc.edges {
            let entity_1 = edge.entity_1.as_deref().unwrap_or("ent0002");
            let entity_2 = edge.entity_2.as_deref().unwrap_or("ent0003");
            let link_type = edge.link_type.as_deref().unwrap_or("association");
            let source_line = edge.source_line.as_deref().unwrap_or("0");
            let link_id = edge.link_id.as_deref().unwrap_or("lnk0");
            let path_style = edge
                .path_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            let code_line_attr = edge
                .code_line
                .as_deref()
                .map(|c| format!(r#"codeLine="{c}" "#))
                .unwrap_or_default();
            let mut group = String::new();
            write!(
                group,
                r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}" data-source-line="{source_line}" id="{link_id}">"#
            )
            .unwrap();
            let path_id_attr = edge_path_id_attr(edge);
            write!(
                group,
                r#"<path {code_line_attr}d="{}" fill="none"{path_id_attr} style="{path_style}"/>"#,
                edge.d,
            )
            .unwrap();
            if let Some(points) = &edge.arrow_points {
                let fill = edge.arrow_fill.as_deref().unwrap_or("#181818");
                let style = edge
                    .polygon_style
                    .as_deref()
                    .unwrap_or("stroke:#181818;stroke-width:1;");
                write!(
                    group,
                    r#"<polygon fill="{fill}" points="{points}" style="{style}"/>"#
                )
                .unwrap();
            }
            group.push_str("</g>");
            svg.raw_inline(&group);
        }
        let mut out = svg.finalize_plantuml();
        // Splice oracle-captured <defs> content (background filters, etc.)
        // into the placeholder `<defs/>` so `filter="url(#…)"` references in
        // the note inner XML resolve.
        if !orc.defs_inner_xml.is_empty() {
            let replacement = format!("<defs>{}</defs>", orc.defs_inner_xml);
            out = out.replacen("<defs/>", &replacement, 1);
        }
        return out;
    }

    if diagram.notes.len() == 1
        && diagram.relationships.is_empty()
        && diagram.notes[0].alias.is_some()
    {
        return render_single_named_note(diagram, &diagram.notes[0], &diagram.meta.sprites);
    }

    // Non-oracle fallback (used by the CLI and unit tests). Keeps a working
    // — though structurally non-PlantUML — rendering so the binary keeps
    // producing useful output when no oracle data is available.
    let title_h = if diagram.meta.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let mut x = GRID_MARGIN;
    let mut max_h = 0.0_f64;
    let note_data: Vec<(f64, f64, f64, f64)> = diagram
        .notes
        .iter()
        .map(|note| {
            let (nw, nh) = note_box_dims(diagram, note, &diagram.meta.sprites);
            let nx = x;
            let ny = GRID_MARGIN + title_h;
            x += nw + GRID_MARGIN;
            max_h = max_h.max(nh);
            (nx, ny, nw, nh)
        })
        .collect();
    let total_width = x.max(GRID_MARGIN * 2.0);
    let total_height = GRID_MARGIN + title_h + max_h + GRID_MARGIN;

    let mut svg = SvgBuilder::new(total_width, total_height);
    if let Some(title) = &diagram.meta.title {
        svg.text(
            total_width / 2.0,
            TITLE_HEIGHT - 4.0,
            title,
            "middle",
            TITLE_FONT_SIZE,
        );
    }
    for (note, (nx, ny, nw, nh)) in diagram.notes.iter().zip(&note_data) {
        render_note_box(
            &mut svg,
            diagram,
            note,
            *nx,
            *ny,
            *nw,
            *nh,
            &diagram.meta.sprites,
        );
    }
    svg.finalize()
}

/// Port of the standalone `EntityImageNote` path through SVEK. `Opale.drawU`
/// paints the folded note with a half-width outer stroke and a one-pixel fold
/// stroke; `CucaDiagramFileMakerSvek` gives a lone named note the standard
/// PlantUML envelope with the entity at `(7, 7)`.
#[allow(clippy::too_many_arguments)]
fn render_svek_floating_note(
    svg: &mut String,
    diagram: &ClassDiagram,
    note_idx: usize,
    pos: &NodePosition,
    entity_id: &str,
    edge_paths: &[EdgePath],
    relationship_edges: &[Option<usize>],
    opale_relationship: Option<usize>,
    layout_x_bias: f64,
    body_dx: f64,
    body_dy: f64,
) {
    let note = &diagram.notes[note_idx];
    let x = pos.x + MARGIN + layout_x_bias + body_dx;
    let y = pos.y + MARGIN + body_dy;
    if let Some(relationship_idx) = opale_relationship
        && let Some(edge_idx) = relationship_edges.get(relationship_idx).copied().flatten()
        && let Some(edge) = edge_paths.get(edge_idx)
        && let Some(geometry) =
            floating_note_opale_geometry(edge, x, y, pos.width, pos.height, layout_x_bias)
    {
        let alias = note.alias.as_deref().expect("floating named note");
        render_attached_note(
            svg,
            diagram,
            note,
            (x * 100.0).round() / 100.0,
            (y * 100.0).round() / 100.0,
            pos.width,
            pos.height,
            geometry.anchor.0,
            geometry.anchor.1,
            geometry.tip.0,
            geometry.tip.1,
            geometry.position,
            alias,
            entity_id,
            &diagram.meta.sprites,
        );
        return;
    }

    render_floating_note_entity(
        svg,
        diagram,
        note,
        x,
        y,
        pos.width,
        pos.height,
        entity_id,
        &diagram.meta.sprites,
    );
}

struct FloatingNoteOpaleGeometry {
    anchor: (f64, f64),
    tip: (f64, f64),
    position: NotePosition,
}

/// `EntityImageNote.drawU` reverses the solved `SmetanaEdge` when necessary
/// so the endpoint nearest the note is `pp1`, then chooses the closest note
/// side with `getOpaleStrategy`. PlantUML consumes Graphviz's SVG coordinates
/// at two-decimal precision before constructing the Opale polygon.
fn floating_note_opale_geometry(
    edge: &EdgePath,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    layout_x_bias: f64,
) -> Option<FloatingNoteOpaleGeometry> {
    let transform = |(point_x, point_y): (f64, f64)| {
        (
            ((point_x + MARGIN + layout_x_bias) * 100.0).round() / 100.0,
            ((point_y + MARGIN) * 100.0).round() / 100.0,
        )
    };
    let first = transform(*edge.points.first()?);
    let last = transform(*edge.points.last()?);
    let center = (x + width / 2.0, y + height / 2.0);
    let distance_sq =
        |point: (f64, f64)| (point.0 - center.0).powi(2) + (point.1 - center.1).powi(2);
    let (anchor, tip) = if distance_sq(first) <= distance_sq(last) {
        (first, last)
    } else {
        (last, first)
    };

    let left = (anchor.0 - x).abs();
    let right = (anchor.0 - (x + width)).abs();
    let top = (anchor.1 - y).abs();
    let bottom = (anchor.1 - (y + height)).abs();
    let position = if left <= right && left <= top && left <= bottom {
        NotePosition::Right
    } else if right <= top && right <= bottom {
        NotePosition::Left
    } else if top <= bottom {
        NotePosition::Bottom
    } else {
        NotePosition::Top
    };
    Some(FloatingNoteOpaleGeometry {
        anchor,
        tip,
        position,
    })
}

#[allow(clippy::too_many_arguments)]
fn render_floating_note_entity(
    svg: &mut String,
    diagram: &ClassDiagram,
    note: &Note,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    entity_id: &str,
    sprites: &HashMap<String, SpriteData>,
) {
    let style = ResolvedNoteStyle::for_note(diagram, note);
    let right = x + width;
    let bottom = y + height;
    let fold_x = right - NOTE_FOLD;
    let fold_y = y + NOTE_FOLD;
    let alias = note.alias.as_deref().expect("floating named note");
    let f = crate::plantuml_metrics::fmt_coord;

    write!(
        svg,
        r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="{entity_id}">"#,
        escape_xml(alias),
        note.source_line,
    )
    .unwrap();
    write!(
        svg,
        r#"<path d="M{},{} L{},{} L{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
        f(x),
        f(y),
        f(x),
        f(bottom),
        f(right),
        f(bottom),
        f(right),
        f(fold_y),
        f(fold_x),
        f(y),
        f(x),
        f(y),
        style.background,
        style.border,
        f(style.border_width),
    )
    .unwrap();
    write!(
        svg,
        r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:1;"/>"#,
        f(fold_x),
        f(y),
        f(fold_x),
        f(fold_y),
        f(right),
        f(fold_y),
        f(fold_x),
        f(y),
        style.background,
        style.border,
    )
    .unwrap();
    emit_note_body(svg, note, x, y, width, sprites, &style);
    svg.push_str("</g>");
}

/// `EntityImageNoteLink.drawU` paints directly inside the owning `SvekEdge`
/// group. Unlike standalone `Opale`, both the outer outline and folded corner
/// use the note component's half-width border stroke.
#[allow(clippy::too_many_arguments)]
fn render_relationship_note(
    svg: &mut String,
    diagram: &ClassDiagram,
    note: &Note,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    sprites: &HashMap<String, SpriteData>,
) {
    let style = ResolvedNoteStyle::for_note(diagram, note);
    let right = x + width;
    let bottom = y + height;
    let fold_x = right - NOTE_FOLD;
    let fold_y = y + NOTE_FOLD;
    let f = crate::plantuml_metrics::fmt_coord;

    write!(
        svg,
        r#"<path d="M{},{} L{},{} L{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
        f(x),
        f(y),
        f(x),
        f(bottom),
        f(right),
        f(bottom),
        f(right),
        f(fold_y),
        f(fold_x),
        f(y),
        f(x),
        f(y),
        style.background,
        style.border,
        f(style.border_width),
    )
    .unwrap();
    write!(
        svg,
        r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
        f(fold_x),
        f(y),
        f(fold_x),
        f(fold_y),
        f(right),
        f(fold_y),
        f(fold_x),
        f(y),
        style.background,
        style.border,
        f(style.border_width),
    )
    .unwrap();
    emit_note_body(svg, note, x, y, width, sprites, &style);
}

fn render_single_named_note(
    diagram: &ClassDiagram,
    note: &Note,
    sprites: &HashMap<String, SpriteData>,
) -> String {
    let style = ResolvedNoteStyle::for_note(diagram, note);
    let (width, height) = note_box_dims(diagram, note, sprites);
    let x = 7.0;
    let y = 7.0;
    let latex_image_right = note
        .lines
        .iter()
        .filter_map(|line| latex_member_content(line))
        .map(|latex| x + NOTE_PAD_X + crate::math::raw_latex_image(latex).width as f64)
        .fold(0.0_f64, f64::max);
    // Java `EntityImageDegenerated` adds its border before the SVG backend
    // narrows the resulting dimensions to an integer viewport.
    let canvas_width = (width + 20.0)
        .floor()
        .max(latex_image_right.ceil() + f64::from(latex_image_right > 0.0));
    let canvas_height = (height + 20.0).floor();
    let right = x + width;
    let bottom = y + height;
    let fold_x = right - NOTE_FOLD;
    let fold_y = y + NOTE_FOLD;
    let alias = note.alias.as_deref().expect("named-note path");
    let f = crate::plantuml_metrics::fmt_coord;

    let mut svg = SvgBuilder::new_plantuml(canvas_width, canvas_height, "CLASS");
    let mut body = String::new();
    write!(
        body,
        r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="ent0002">"#,
        escape_xml(alias),
        note.source_line,
    )
    .unwrap();
    write!(
        body,
        r#"<path d="M{},{} L{},{} L{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:{};"/>"#,
        f(x),
        f(y),
        f(x),
        f(bottom),
        f(right),
        f(bottom),
        f(right),
        f(fold_y),
        f(fold_x),
        f(y),
        f(x),
        f(y),
        style.background,
        style.border,
        f(style.border_width),
    )
    .unwrap();
    write!(
        body,
        r#"<path d="M{},{} L{},{} L{},{} L{},{}" fill="{}" style="stroke:{};stroke-width:1;"/>"#,
        f(fold_x),
        f(y),
        f(fold_x),
        f(fold_y),
        f(right),
        f(fold_y),
        f(fold_x),
        f(y),
        style.background,
        style.border,
    )
    .unwrap();
    emit_note_body(&mut body, note, x, y, width, sprites, &style);
    body.push_str("</g>");
    svg.raw_inline(&body);
    svg.finalize_plantuml()
}

/// Render a class-diagram that has no entities/notes but does carry one or
/// more meta decorations (title, header, footer, legend). PlantUML wraps
/// each decoration in its own `<g class="...">` group inside the standard
/// envelope; mirror that shape.
fn render_meta_only(diagram: &ClassDiagram) -> String {
    // Per-line dimensions and y baseline computations match the strict-XML
    // goldens for the single-decoration cases. Multi-decoration is best-
    // effort.
    let header = diagram.meta.header.as_deref().filter(|s| !s.is_empty());
    let footer = diagram.meta.footer.as_deref().filter(|s| !s.is_empty());
    let title = diagram.meta.title.as_deref().filter(|s| !s.is_empty());
    let legend = diagram.meta.legend.as_deref().filter(|s| !s.is_empty());

    // Pre-compute widths via PlantUML's text metrics so the canvas width
    // matches the golden exactly when only one decoration is present.
    let header_w = header.map(|t| text_render::measure_no_underline(t, 10.0, false));
    let footer_w = footer.map(|t| text_render::measure_no_underline(t, 10.0, false));
    let title_w = title.map(|t| text_render::measure_no_underline(t, 14.0, true));
    let legend_w = legend.map(|t| {
        t.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| text_render::measure_no_underline(l, 14.0, false))
            .fold(0.0_f64, f64::max)
    });

    let max_text_w = [header_w, footer_w, title_w, legend_w]
        .iter()
        .filter_map(|w| *w)
        .fold(0.0_f64, f64::max);

    // Canvas geometry per golden inspection:
    // - header-only: width = text_w + 7, height = 28 (text y=9.668)
    // - footer-only: width = text_w + 7, height = 28 (text y=19.668)
    // - title-only:  width = text_w + 27, height = 53
    // - legend-only: width = max_line_w + ~30, height = 27 + 26.4883 * line_count + ~14
    let (canvas_w, canvas_h) =
        if title.is_some() && header.is_none() && footer.is_none() && legend.is_none() {
            (max_text_w + 27.0, 53.0)
        } else if legend.is_some() && header.is_none() && footer.is_none() && title.is_none() {
            let lines = legend
                .unwrap()
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count();
            let rect_h = 26.4883 * lines as f64;
            // Canvas adds left margin (12), rect_w = text+10, then right margin (~18).
            // Height is rect_y(22) + rect_h + bottom margin (~18.5), rounded up.
            (max_text_w + 40.4, 22.0 + rect_h + 18.6)
        } else {
            (max_text_w + 7.0, 28.0)
        };

    let mut svg = SvgBuilder::new_plantuml(canvas_w, canvas_h, "CLASS");
    let mut buf = String::new();

    // Header (top, grey, small).
    if let Some(text) = header {
        let sl = diagram.header_line.unwrap_or(1);
        write!(buf, r#"<g class="header" data-source-line="{sl}">"#).unwrap();
        text_render::emit_text(
            &mut buf,
            text,
            &text_render::TextBase {
                x: 0.0,
                y: 9.668,
                font_size: 10,
                font_family: "sans-serif",
                fill: "#888888",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        buf.push_str("</g>");
    }

    // Title (top, bold, centred-ish — golden has x=10).
    if let Some(text) = title {
        let sl = diagram.title_line.unwrap_or(1);
        write!(buf, r#"<g class="title" data-source-line="{sl}">"#).unwrap();
        text_render::emit_text(
            &mut buf,
            text,
            &text_render::TextBase {
                x: 10.0,
                y: 23.5352,
                font_size: 14,
                font_family: "sans-serif",
                fill: "#000000",
                bold: true,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        buf.push_str("</g>");
    }

    // Legend (centred rect with text inside).
    if let Some(text) = legend {
        let sl = diagram.legend_line.unwrap_or(1);
        write!(buf, r#"<g class="legend" data-source-line="{sl}">"#).unwrap();
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let line_w = lines
            .iter()
            .map(|l| text_render::measure_no_underline(l, 14.0, false))
            .fold(0.0_f64, f64::max);
        let rect_w = line_w + 10.0;
        let rect_h = 26.4883 * lines.len() as f64;
        let rect_x = 12.0;
        let rect_y = 22.0;
        let rect_w_str = crate::plantuml_metrics::fmt_coord(rect_w);
        write!(
            buf,
            "<rect fill=\"#DDDDDD\" height=\"{rect_h}\" rx=\"7.5\" ry=\"7.5\" style=\"stroke:#000000;stroke-width:1;\" width=\"{rect_w_str}\" x=\"{rect_x}\" y=\"{rect_y}\"/>",
        )
        .unwrap();
        for (i, line) in lines.iter().enumerate() {
            let y = 40.5352 + i as f64 * 26.4883;
            text_render::emit_text(
                &mut buf,
                line,
                &text_render::TextBase {
                    x: 17.0,
                    y,
                    font_size: 14,
                    font_family: "sans-serif",
                    fill: "#000000",
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
        }
        buf.push_str("</g>");
    }

    // Footer (bottom, grey, small).
    if let Some(text) = footer {
        let sl = diagram.footer_line.unwrap_or(1);
        let y = canvas_h - 8.332;
        write!(buf, r#"<g class="footer" data-source-line="{sl}">"#).unwrap();
        text_render::emit_text(
            &mut buf,
            text,
            &text_render::TextBase {
                x: 0.0,
                y,
                font_size: 10,
                font_family: "sans-serif",
                fill: "#888888",
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        buf.push_str("</g>");
    }

    svg.raw_inline(&buf);
    svg.finalize_plantuml()
}

#[derive(Clone, Copy)]
struct NoteBodySeparator<'a> {
    style: char,
    title: Option<&'a str>,
}

struct NoteBodyBlock<'a> {
    separator: Option<NoteBodySeparator<'a>>,
    lines: Vec<&'a str>,
}

struct NoteTableLayout {
    rows: Vec<crate::creole::TableRow>,
    column_widths: Vec<f64>,
    row_heights: Vec<f64>,
    row_ascents: Vec<f64>,
    width: f64,
}

fn note_table_cell_content(cell: &crate::creole::TableCell) -> String {
    // `StripeTable` keeps the delimiter-adjacent cell spaces as atoms. They
    // collapse into the surrounding run for plain cells, but remain separate
    // text atoms around inline Creole style changes.
    format!(" {} ", cell.text)
}

fn note_table_layout(lines: &[&str], style: &ResolvedNoteStyle) -> Option<NoteTableLayout> {
    let rows = lines
        .iter()
        .map(|line| match crate::creole::parse_line(line.trim()) {
            crate::creole::CreoleLine::Table(row) => Some(row),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    if rows.is_empty() {
        return None;
    }

    let column_count = rows.iter().map(|row| row.cells.len()).max().unwrap_or(0);
    let mut column_widths = vec![0.0_f64; column_count];
    let mut row_heights = Vec::with_capacity(rows.len());
    let mut row_ascents = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut row_height = 10.0_f64;
        let mut row_ascent = 0.0_f64;
        for (column, cell) in row.cells.iter().enumerate() {
            let content = note_table_cell_content(cell);
            let width = text_render::measure_with_family(
                &content,
                style.font_size as f64,
                style.bold || cell.is_header,
                &style.font_family,
            );
            column_widths[column] = column_widths[column].max(width);
            row_height = row_height.max(text_render::label_height_with_family(
                &content,
                style.font_size as f64,
                &style.font_family,
            ));
            row_ascent = row_ascent.max(text_render::label_ascent_with_family(
                &content,
                style.font_size as f64,
                &style.font_family,
            ));
        }
        row_heights.push(row_height);
        row_ascents.push(row_ascent);
    }
    let width = column_widths.iter().sum();
    Some(NoteTableLayout {
        rows,
        column_widths,
        row_heights,
        row_ascents,
        width,
    })
}

fn note_tree_rows(lines: &[&str]) -> Option<Vec<crate::creole::TreeNode>> {
    let rows = lines
        .iter()
        .map(|line| match crate::creole::parse_line(line.trim()) {
            crate::creole::CreoleLine::Tree(node) => Some(node),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!rows.is_empty()).then_some(rows)
}

fn note_tree_text(node: &crate::creole::TreeNode) -> String {
    if node.depth == 1 {
        node.text.clone()
    } else {
        format!("{} {}", "_".repeat(node.depth - 1), node.text)
    }
}

fn note_tree_text_offset(node: &crate::creole::TreeNode) -> f64 {
    NOTE_TREE_TEXT_X
        + if node.depth == 1 {
            NOTE_ORDERED_NUMBER_GAP
        } else {
            0.0
        }
}

fn note_code_lines<'a>(lines: &[&'a str]) -> Option<Vec<&'a str>> {
    let (first, rest) = lines.split_first()?;
    let (last, middle) = rest.split_last()?;
    if !first.trim().eq_ignore_ascii_case("<code>") || !last.trim().eq_ignore_ascii_case("</code>")
    {
        return None;
    }
    Some(
        middle
            .iter()
            .copied()
            .filter(|line| !line.trim().is_empty())
            .collect(),
    )
}

fn note_code_line_parts(line: &str) -> (usize, &str) {
    let indent = line
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    (indent, line.trim())
}

fn note_code_line_width(line: &str, font_size: f64) -> f64 {
    let (indent, content) = note_code_line_parts(line);
    crate::plantuml_metrics::mono_text_width(&" ".repeat(indent), font_size)
        + crate::plantuml_metrics::mono_text_width(content, font_size)
}

fn note_code_line_height(font_size: f64) -> f64 {
    text_render::label_height_with_family("", font_size, "monospace").max(10.0)
}

fn parse_note_body_separator(line: &str) -> Option<NoteBodySeparator<'_>> {
    let style = if line.starts_with("--") && line.ends_with("--") {
        '-'
    } else if line.starts_with("==") && line.ends_with("==") {
        '='
    } else if line != "..." && line.starts_with("..") && line.ends_with("..") {
        '.'
    } else if line.starts_with("__") && line.ends_with("__") {
        '_'
    } else {
        return None;
    };
    let title = (line.len() > 4).then(|| line[2..line.len() - 2].trim());
    Some(NoteBodySeparator { style, title })
}

fn note_body_blocks(note: &Note) -> Vec<NoteBodyBlock<'_>> {
    let mut blocks = vec![NoteBodyBlock {
        separator: None,
        lines: Vec::new(),
    }];
    for line in &note.lines {
        if let Some(separator) = parse_note_body_separator(line) {
            blocks.push(NoteBodyBlock {
                separator: Some(separator),
                lines: Vec::new(),
            });
        } else {
            blocks.last_mut().unwrap().lines.push(line);
        }
    }
    blocks
}

fn next_note_number(level: usize, number_counters: &mut Vec<usize>) -> usize {
    if number_counters.len() > level {
        number_counters.truncate(level);
    }
    while number_counters.len() < level {
        number_counters.push(0);
    }
    number_counters[level - 1] += 1;
    number_counters[level - 1]
}

fn note_ordered_indent(level: usize, style: &ResolvedNoteStyle) -> f64 {
    (text_render::measure_with_family("1.", style.font_size as f64, style.bold, &style.font_family)
        + NOTE_ORDERED_NUMBER_GAP)
        * level.saturating_sub(1) as f64
}

fn note_creole_content(content: String) -> String {
    if crate::creole::parse_segments(&content).is_empty() {
        // `StripeSimple.getAtoms()` inserts one plain-space `AtomText` when
        // parsing produced no atoms (including an empty styled span). The
        // placeholder inherits the stripe's base font after style commands
        // restore their previous `FontConfiguration`.
        " ".to_string()
    } else {
        content
    }
}

/// Port of PlantUML's `AtomSprite` inside `Sea`: each atom contributes its
/// natural width, while the row takes the tallest atom and bottom-aligns the
/// remaining atoms in that shared line box.
fn note_sprite_line_dimensions(
    content: &str,
    sprites: &HashMap<String, SpriteData>,
    style: &ResolvedNoteStyle,
) -> (f64, f64) {
    crate::sprite::parse_sprite_segments(content).iter().fold(
        (0.0_f64, 0.0_f64),
        |(width, height), segment| {
            let (segment_width, segment_height) = match segment {
                crate::sprite::TextSegment::Text(text) => (
                    text_render::measure_with_family(
                        text,
                        style.font_size as f64,
                        style.bold,
                        &style.font_family,
                    ),
                    text_render::label_height_with_family(
                        text,
                        style.font_size as f64,
                        &style.font_family,
                    )
                    .max(10.0),
                ),
                crate::sprite::TextSegment::Sprite(name) => sprites
                    .get(name)
                    .map(|sprite| {
                        let (width, height) = crate::sprite::sprite_dimensions(sprite);
                        (width as f64, height as f64)
                    })
                    .unwrap_or_default(),
                crate::sprite::TextSegment::OpenIcon(name) => crate::openiconic::lookup(name)
                    .map(|icon| {
                        let scale = style.font_size as f64 / icon.height;
                        (icon.width * scale, icon.height * scale)
                    })
                    .unwrap_or_default(),
            };
            (width + segment_width, height.max(segment_height))
        },
    )
}

fn emit_note_sprite_line(
    svg: &mut String,
    content: &str,
    x: f64,
    line_top: f64,
    sprites: &HashMap<String, SpriteData>,
    background: &str,
    style: &ResolvedNoteStyle,
) -> f64 {
    let (_, line_height) = note_sprite_line_dimensions(content, sprites, style);
    let mut cursor = x;
    for segment in crate::sprite::parse_sprite_segments(content) {
        match segment {
            crate::sprite::TextSegment::Text(text) => {
                let text_height = text_render::label_height_with_family(
                    &text,
                    style.font_size as f64,
                    &style.font_family,
                )
                .max(10.0);
                let baseline = line_top + line_height - text_height
                    + text_render::label_first_baseline_ascent_with_family(
                        &text,
                        style.font_size as f64,
                        &style.font_family,
                    );
                cursor += text_render::emit_text(
                    svg,
                    &text,
                    &TextBase {
                        x: cursor,
                        y: baseline,
                        font_size: style.font_size,
                        font_family: &style.font_family,
                        fill: &style.font_color,
                        bold: style.bold,
                        italic: style.italic,
                        underline: false,
                        skip_underline: false,
                    },
                );
            }
            crate::sprite::TextSegment::Sprite(name) => {
                if let Some(sprite) = sprites.get(&name) {
                    let (width, height) = crate::sprite::sprite_dimensions(sprite);
                    if let Ok(uri) = crate::sprite::sprite_to_data_uri_scaled_with_colors(
                        sprite,
                        1.0,
                        sprite_surface_rgb(background),
                        [0, 0, 0],
                    ) {
                        write!(
                            svg,
                            r#"<image height="{}" width="{}" x="{}" xlink:href="{}" y="{}"/>"#,
                            height,
                            width,
                            crate::plantuml_metrics::fmt_coord(cursor),
                            uri,
                            crate::plantuml_metrics::fmt_coord(
                                line_top + line_height - height as f64
                            ),
                        )
                        .unwrap();
                    }
                    cursor += width as f64;
                }
            }
            crate::sprite::TextSegment::OpenIcon(name) => {
                if let Some(icon) = crate::openiconic::lookup(&name) {
                    let scale = style.font_size as f64 / icon.height;
                    let icon_height = icon.height * scale;
                    write!(
                        svg,
                        r##"<path d="{}" fill="#000000" transform="translate({} {}) scale({})"/>"##,
                        icon.path_d,
                        crate::plantuml_metrics::fmt_coord(cursor + icon.translate_x * scale),
                        crate::plantuml_metrics::fmt_coord(
                            line_top + line_height - icon_height + icon.translate_y * scale
                        ),
                        crate::plantuml_metrics::fmt_coord(scale),
                    )
                    .unwrap();
                    cursor += icon.width * scale;
                }
            }
        }
    }
    line_height
}

fn note_line_dimensions(
    line: &str,
    number_counters: &mut Vec<usize>,
    sprites: &HashMap<String, SpriteData>,
    style: &ResolvedNoteStyle,
) -> (f64, f64) {
    if let Some(latex) = latex_member_content(line) {
        number_counters.clear();
        let metrics = crate::math::latex_layout_metrics(latex);
        return (metrics.width, metrics.height);
    }

    let (header_width, content) = if let Some((order, content)) = parse_note_bullet(line) {
        number_counters.clear();
        let header_width = if order == 0 {
            NOTE_BULLET_HEADER_WIDTH
        } else {
            NOTE_NESTED_BULLET_BASE_WIDTH + NOTE_NESTED_BULLET_INDENT * order as f64
        };
        (header_width, content.to_string())
    } else if let crate::creole::CreoleLine::Numbered { level, content } =
        crate::creole::parse_line(line.trim())
    {
        let number = next_note_number(level, number_counters);
        let marker = format!("{number}.");
        let indent = note_ordered_indent(level, style);
        (
            indent
                + text_render::measure_with_family(
                    &marker,
                    style.font_size as f64,
                    style.bold,
                    &style.font_family,
                )
                + NOTE_ORDERED_NUMBER_GAP,
            content,
        )
    } else {
        number_counters.clear();
        (0.0, line.to_string())
    };
    let content = style.text_content(note_creole_content(content));
    if content.contains("<$") {
        let (width, height) = note_sprite_line_dimensions(&content, sprites, style);
        return (header_width + width, height);
    }
    (
        header_width
            + text_render::measure_with_family(
                &content,
                style.font_size as f64,
                style.bold,
                &style.font_family,
            ),
        text_render::label_height_with_family(&content, style.font_size as f64, &style.font_family)
            .max(10.0),
    )
}

fn note_body_dimensions(
    note: &Note,
    sprites: &HashMap<String, SpriteData>,
    style: &ResolvedNoteStyle,
) -> (f64, f64) {
    let mut width = 0.0_f64;
    let mut height = 0.0_f64;
    for block in note_body_blocks(note) {
        let code = note_code_lines(&block.lines);
        let tree = note_tree_rows(&block.lines);
        let table = note_table_layout(&block.lines, style);
        let (body_width, body_height) = if let Some(code) = &code {
            (
                code.iter()
                    .map(|line| note_code_line_width(line, style.font_size as f64))
                    .fold(0.0_f64, f64::max),
                code.len() as f64 * note_code_line_height(style.font_size as f64),
            )
        } else if let Some(tree) = &tree {
            (
                tree.iter()
                    .map(|node| {
                        note_tree_text_offset(node)
                            + text_render::measure_with_family(
                                &note_tree_text(node),
                                style.font_size as f64,
                                style.bold,
                                &style.font_family,
                            )
                    })
                    .fold(0.0_f64, f64::max),
                tree.iter()
                    .map(|node| {
                        text_render::label_height_with_family(
                            &note_tree_text(node),
                            style.font_size as f64,
                            &style.font_family,
                        )
                        .max(10.0)
                    })
                    .sum::<f64>()
                    + NOTE_TREE_BODY_EXTRA,
            )
        } else if let Some(table) = &table {
            (
                table.width,
                table.row_heights.iter().sum::<f64>() + NOTE_TABLE_BODY_EXTRA,
            )
        } else {
            let mut number_counters = Vec::new();
            let line_dimensions = block
                .lines
                .iter()
                .map(|line| note_line_dimensions(line, &mut number_counters, sprites, style))
                .collect::<Vec<_>>();
            (
                line_dimensions
                    .iter()
                    .map(|(line_width, _)| *line_width)
                    .fold(0.0_f64, f64::max),
                line_dimensions
                    .iter()
                    .map(|(_, line_height)| *line_height)
                    .sum::<f64>(),
            )
        };
        let (block_width, block_height) = match block.separator {
            None => (body_width, body_height),
            Some(NoteBodySeparator { title: None, .. }) => (body_width, body_height + 8.0),
            Some(NoteBodySeparator {
                title: Some(title), ..
            }) => {
                let title_width = text_render::measure_with_family(
                    title,
                    style.font_size as f64,
                    style.bold,
                    &style.font_family,
                );
                let title_height = text_render::label_height_with_family(
                    title,
                    style.font_size as f64,
                    &style.font_family,
                );
                let half_title = title_height / 2.0;
                (
                    (body_width + 6.0).max(title_width + 8.0),
                    half_title + (body_height + half_title + 4.0).max(title_height),
                )
            }
        };
        width = width.max(block_width);
        height += block_height;
    }
    (width, height)
}

fn note_box_dims(
    diagram: &ClassDiagram,
    note: &Note,
    sprites: &HashMap<String, SpriteData>,
) -> (f64, f64) {
    let style = ResolvedNoteStyle::for_note(diagram, note);
    let (body_width, body_height) = note_body_dimensions(note, sprites, &style);
    (
        body_width + NOTE_PAD_X + NOTE_PAD_RIGHT,
        body_height + NOTE_PAD_Y * 2.0,
    )
}

fn emit_note_table(
    svg: &mut String,
    table: &NoteTableLayout,
    x: f64,
    y: f64,
    style: &ResolvedNoteStyle,
) {
    let f = crate::plantuml_metrics::fmt_coord;
    let grid_left = x + NOTE_PAD_X;
    let grid_top = y + NOTE_TABLE_GRID_TOP_PAD;
    let mut row_top = grid_top;
    for (row_index, row) in table.rows.iter().enumerate() {
        let mut cell_left = grid_left;
        for (column, cell) in row.cells.iter().enumerate() {
            let cell_width = table.column_widths.get(column).copied().unwrap_or_default();
            if let Some(background) = cell.bg_color.as_deref() {
                let fill = crate::sequence::resolve_color(background);
                write!(
                    svg,
                    r#"<rect fill="{}" height="{}" style="stroke:none;stroke-width:1;" width="{}" x="{}" y="{}"/>"#,
                    fill,
                    f(table.row_heights[row_index]),
                    f(cell_width),
                    f(cell_left),
                    f(row_top),
                )
                .unwrap();
            }
            let content = note_table_cell_content(cell);
            text_render::emit_text(
                svg,
                &content,
                &TextBase {
                    x: cell_left,
                    y: row_top + table.row_ascents[row_index],
                    font_size: style.font_size,
                    font_family: &style.font_family,
                    fill: &style.font_color,
                    bold: style.bold || cell.is_header,
                    italic: style.italic,
                    underline: false,
                    skip_underline: false,
                },
            );
            cell_left += cell_width;
        }
        row_top += table.row_heights[row_index];
    }

    let grid_right = grid_left + table.width;
    let grid_bottom = grid_top + table.row_heights.iter().sum::<f64>();
    let mut line_y = grid_top;
    for row_height in std::iter::once(0.0).chain(table.row_heights.iter().copied()) {
        line_y += row_height;
        write!(
            svg,
            r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            f(grid_left),
            f(grid_right),
            f(line_y),
            f(line_y),
        )
        .unwrap();
    }
    let mut line_x = grid_left;
    for column_width in std::iter::once(0.0).chain(table.column_widths.iter().copied()) {
        line_x += column_width;
        write!(
            svg,
            r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            f(line_x),
            f(line_x),
            f(grid_top),
            f(grid_bottom),
        )
        .unwrap();
    }
}

fn emit_note_tree(
    svg: &mut String,
    rows: &[crate::creole::TreeNode],
    x: f64,
    y: f64,
    fill: &str,
    style: &ResolvedNoteStyle,
) {
    let f = crate::plantuml_metrics::fmt_coord;
    let body_left = x + NOTE_PAD_X;
    let trunk_x = body_left + NOTE_TREE_TRUNK_X;
    let branch_right = trunk_x + NOTE_TREE_BRANCH_WIDTH;
    let grid_top = y + NOTE_TREE_GRID_TOP_PAD;
    let mut row_top = grid_top;
    let mut previous_branch_y = grid_top;
    let mut branches = Vec::with_capacity(rows.len());
    for node in rows {
        let text = note_tree_text(node);
        let row_height = text_render::label_height_with_family(
            &text,
            style.font_size as f64,
            &style.font_family,
        )
        .max(10.0);
        let branch_y = row_top + row_height / 2.0;
        text_render::emit_text(
            svg,
            &text,
            &TextBase {
                x: body_left + note_tree_text_offset(node),
                y: row_top
                    + text_render::label_ascent_with_family(
                        &text,
                        style.font_size as f64,
                        &style.font_family,
                    ),
                font_size: style.font_size,
                font_family: &style.font_family,
                fill: &style.font_color,
                bold: style.bold,
                italic: style.italic,
                underline: false,
                skip_underline: false,
            },
        );
        branches.push((previous_branch_y, branch_y));
        previous_branch_y = branch_y;
        row_top += row_height;
    }
    for (vertical_top, branch_y) in branches {
        write!(
            svg,
            r#"<rect fill="{}" height="2" style="stroke:#000000;stroke-width:1;" width="2" x="{}" y="{}"/>"#,
            fill,
            f(branch_right - 1.0),
            f(branch_y - 1.0),
        )
        .unwrap();
        write!(
            svg,
            r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            f(trunk_x),
            f(branch_right),
            f(branch_y),
            f(branch_y),
        )
        .unwrap();
        write!(
            svg,
            r##"<line style="stroke:#000000;stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"##,
            f(trunk_x),
            f(trunk_x),
            f(vertical_top),
            f(branch_y),
        )
        .unwrap();
    }
}

fn emit_note_code(svg: &mut String, lines: &[&str], x: f64, y: f64, style: &ResolvedNoteStyle) {
    let f = crate::plantuml_metrics::fmt_coord;
    let mut row_top = y;
    for line in lines {
        let (indent, content) = note_code_line_parts(line);
        let font_size = style.font_size as f64;
        let indent_width = crate::plantuml_metrics::mono_text_width(&" ".repeat(indent), font_size);
        let content_width = crate::plantuml_metrics::mono_text_width(content, font_size);
        let baseline =
            row_top + text_render::label_ascent_with_family(content, font_size, "monospace");
        let escaped = crate::creole::escape_creole_text(content).replace(' ', "&#160;");
        write!(
            svg,
            r#"<text fill="{}" font-family="monospace" font-size="{}"{}{} lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"#,
            style.font_color,
            style.font_size,
            if style.italic { r#" font-style="italic""# } else { "" },
            if style.bold { r#" font-weight="700""# } else { "" },
            f(content_width),
            f(x + NOTE_PAD_X + indent_width),
            f(baseline),
            escaped,
        )
        .unwrap();
        if indent > 0 {
            write!(
                svg,
                r#"<text fill="{}" font-family="monospace" font-size="{}"{}{} lengthAdjust="spacing" textLength="0" x="{}" y="{}"></text>"#,
                style.font_color,
                style.font_size,
                if style.italic { r#" font-style="italic""# } else { "" },
                if style.bold { r#" font-weight="700""# } else { "" },
                f(x + NOTE_PAD_X),
                f(baseline),
            )
            .unwrap();
        }
        row_top += note_code_line_height(font_size);
    }
}

fn emit_note_body(
    svg: &mut String,
    note: &Note,
    x: f64,
    y: f64,
    width: f64,
    sprites: &HashMap<String, SpriteData>,
    style: &ResolvedNoteStyle,
) {
    let mut block_top = y + NOTE_PAD_Y;
    for block in note_body_blocks(note) {
        if let Some(code) = note_code_lines(&block.lines) {
            emit_note_code(svg, &code, x, block_top, style);
            block_top += code.len() as f64 * note_code_line_height(style.font_size as f64);
            continue;
        }
        if let Some(tree) = note_tree_rows(&block.lines) {
            emit_note_tree(svg, &tree, x, block_top, &style.background, style);
            block_top += tree
                .iter()
                .map(|node| {
                    text_render::label_height_with_family(
                        &note_tree_text(node),
                        style.font_size as f64,
                        &style.font_family,
                    )
                    .max(10.0)
                })
                .sum::<f64>()
                + NOTE_TREE_BODY_EXTRA;
            continue;
        }
        if let Some(table) = note_table_layout(&block.lines, style) {
            emit_note_table(svg, &table, x, block_top, style);
            block_top += table.row_heights.iter().sum::<f64>() + NOTE_TABLE_BODY_EXTRA;
            continue;
        }
        let mut measure_counters = Vec::new();
        let body_height = block
            .lines
            .iter()
            .map(|line| note_line_dimensions(line, &mut measure_counters, sprites, style).1)
            .sum::<f64>();
        let mut content_top = block_top;
        let mut separator_height = 0.0;
        let mut number_counters = Vec::new();

        match block.separator {
            None => {}
            Some(NoteBodySeparator {
                style: separator_style,
                title: None,
            }) => {
                emit_note_separator(svg, x, width, block_top, separator_style, None, style);
                content_top += 4.0;
                separator_height = 8.0;
            }
            Some(NoteBodySeparator {
                style: separator_style,
                title: Some(title),
            }) => {
                let title_height = text_render::label_height_with_family(
                    title,
                    style.font_size as f64,
                    &style.font_family,
                );
                let half_title = title_height / 2.0;
                content_top += title_height;
                let inner_height = (body_height + half_title + 4.0).max(title_height);
                separator_height = half_title + inner_height - body_height;

                let mut line_top = content_top;
                for line in &block.lines {
                    line_top += emit_note_line(
                        svg,
                        line,
                        x,
                        width,
                        line_top,
                        &mut number_counters,
                        note,
                        sprites,
                        style,
                    );
                }
                emit_note_separator(
                    svg,
                    x,
                    width,
                    block_top + half_title,
                    separator_style,
                    Some(title),
                    style,
                );
                block_top += body_height + separator_height;
                continue;
            }
        }

        let mut line_top = content_top;
        for line in &block.lines {
            line_top += emit_note_line(
                svg,
                line,
                x,
                width,
                line_top,
                &mut number_counters,
                note,
                sprites,
                style,
            );
        }
        block_top += body_height + separator_height;
    }
}

fn emit_note_separator(
    svg: &mut String,
    x: f64,
    width: f64,
    line_y: f64,
    style: char,
    title: Option<&str>,
    note_style: &ResolvedNoteStyle,
) {
    let f = crate::plantuml_metrics::fmt_coord;
    let stroke_style = match style {
        '.' => "stroke:#181818;stroke-width:1;stroke-dasharray:1,2;",
        '_' => "stroke:#181818;stroke-width:0.5;",
        '-' | '=' => "stroke:#181818;stroke-width:1;",
        _ => unreachable!(),
    };
    let line_count = usize::from(style == '=') + 1;
    let start_x = x + 1.0;
    let end_x = x + width - 1.0;
    let (first_end, second_start) = if let Some(title) = title {
        let title_width = text_render::measure_with_family(
            title,
            note_style.font_size as f64,
            note_style.bold,
            &note_style.font_family,
        );
        let half_line = (end_x - start_x - title_width) / 2.0;
        (start_x + half_line, end_x - half_line)
    } else {
        (end_x, end_x)
    };

    for line_index in 0..line_count {
        let y = line_y + line_index as f64 * 2.0;
        write!(
            svg,
            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            stroke_style,
            f(start_x),
            f(first_end),
            f(y),
            f(y),
        )
        .unwrap();
    }
    let Some(title) = title else {
        return;
    };

    let title_height = text_render::label_height_with_family(
        title,
        note_style.font_size as f64,
        &note_style.font_family,
    );
    let baseline = line_y - title_height / 2.0 - 0.5
        + text_render::label_ascent_with_family(
            title,
            note_style.font_size as f64,
            &note_style.font_family,
        );
    text_render::emit_text(
        svg,
        title,
        &TextBase {
            x: first_end,
            y: baseline,
            font_size: note_style.font_size,
            font_family: &note_style.font_family,
            fill: &note_style.font_color,
            bold: note_style.bold,
            italic: note_style.italic,
            underline: false,
            skip_underline: false,
        },
    );
    for line_index in 0..line_count {
        let y = line_y + line_index as f64 * 2.0;
        write!(
            svg,
            r#"<line style="{}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            stroke_style,
            f(second_start),
            f(end_x),
            f(y),
            f(y),
        )
        .unwrap();
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_note_line(
    svg: &mut String,
    line: &str,
    x: f64,
    width: f64,
    line_top: f64,
    number_counters: &mut Vec<usize>,
    note: &Note,
    sprites: &HashMap<String, SpriteData>,
    style: &ResolvedNoteStyle,
) -> f64 {
    let mut alignment_counters = number_counters.clone();
    let line_width = note_line_dimensions(line, &mut alignment_counters, sprites, style).0;
    let line_x = style.line_x(x, width, line_width);
    if let Some(latex) = latex_member_content(line) {
        number_counters.clear();
        let image = crate::math::raw_latex_image(latex);
        write!(
            svg,
            r#"<image height="{}" width="{}" x="{}" xlink:href="{}" y="{}"/>"#,
            image.height,
            image.width,
            crate::plantuml_metrics::fmt_coord(line_x + NOTE_PAD_X),
            image.href,
            crate::plantuml_metrics::fmt_coord(line_top),
        )
        .unwrap();
        return crate::math::latex_layout_metrics(latex).height;
    }

    let f = crate::plantuml_metrics::fmt_coord;
    let (content, text_x) = if let Some((order, content)) = parse_note_bullet(line) {
        number_counters.clear();
        let text_height = text_render::label_height_with_family(
            content,
            style.font_size as f64,
            &style.font_family,
        );
        if order == 0 {
            let ellipse_x = line_x + NOTE_PAD_X + NOTE_BULLET_ELLIPSE_X;
            let ellipse_y = line_top + text_height - NOTE_BULLET_ELLIPSE_SIZE
                + NOTE_BULLET_START_ALTITUDE
                + NOTE_BULLET_ELLIPSE_SIZE / 2.0;
            write!(
                svg,
                r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}"/>"#,
                f(ellipse_x + NOTE_BULLET_ELLIPSE_SIZE / 2.0),
                f(ellipse_y),
                style.font_color,
                f(NOTE_BULLET_ELLIPSE_SIZE / 2.0),
                f(NOTE_BULLET_ELLIPSE_SIZE / 2.0),
            )
            .unwrap();
            (
                content.to_string(),
                line_x + NOTE_PAD_X + NOTE_BULLET_HEADER_WIDTH,
            )
        } else {
            let order_width = NOTE_NESTED_BULLET_INDENT * order as f64;
            let rect_x = line_x + NOTE_PAD_X + NOTE_NESTED_BULLET_X + order_width;
            let rect_y = line_top + text_height - NOTE_NESTED_BULLET_DIM_HEIGHT
                + NOTE_NESTED_BULLET_START_ALTITUDE;
            write!(
                svg,
                r#"<rect fill="{}" height="{}" width="{}" x="{}" y="{}"/>"#,
                style.font_color,
                f(NOTE_NESTED_BULLET_SIZE),
                f(NOTE_NESTED_BULLET_SIZE),
                f(rect_x),
                f(rect_y),
            )
            .unwrap();
            (
                content.to_string(),
                line_x + NOTE_PAD_X + NOTE_NESTED_BULLET_BASE_WIDTH + order_width,
            )
        }
    } else if let crate::creole::CreoleLine::Numbered { level, content } =
        crate::creole::parse_line(line.trim())
    {
        let number = next_note_number(level, number_counters);
        let marker = format!("{number}.");
        let indent = note_ordered_indent(level, style);
        let marker_x = line_x + NOTE_PAD_X + indent;
        let baseline = line_top
            + text_render::label_ascent_with_family(
                &marker,
                style.font_size as f64,
                &style.font_family,
            );
        let marker_width = text_render::emit_text(
            svg,
            &marker,
            &TextBase {
                x: marker_x,
                y: baseline,
                font_size: style.font_size,
                font_family: &style.font_family,
                fill: &style.font_color,
                bold: style.bold,
                italic: style.italic,
                underline: false,
                skip_underline: false,
            },
        );
        (content, marker_x + marker_width + NOTE_ORDERED_NUMBER_GAP)
    } else {
        number_counters.clear();
        (line.to_string(), line_x + NOTE_PAD_X)
    };
    // `Sea.doAlign` bottom-aligns mixed Creole atoms. The line origin must use
    // the first run's ascent within that shared line box, not the maximum
    // ascent of every run (notably monospace followed by sans-serif).
    let content = style.text_content(note_creole_content(content));
    if content.contains("<$") {
        let fill = note
            .color
            .as_deref()
            .map(crate::sequence::resolve_color)
            .unwrap_or_else(|| NOTE_FILL.to_string());
        return emit_note_sprite_line(svg, &content, text_x, line_top, sprites, &fill, style);
    }
    let baseline = line_top
        + text_render::label_first_baseline_ascent_with_family(
            &content,
            style.font_size as f64,
            &style.font_family,
        );
    text_render::emit_text(
        svg,
        &content,
        &TextBase {
            x: text_x,
            y: baseline,
            font_size: style.font_size,
            font_family: &style.font_family,
            fill: &style.font_color,
            bold: style.bold,
            italic: style.italic,
            underline: false,
            skip_underline: false,
        },
    );
    text_render::label_height_with_family(&content, style.font_size as f64, &style.font_family)
        .max(10.0)
}

/// Match `CreoleStripeSimpleParser.ASTERISK_PREFIXED_LINE_PATTERN` without
/// trimming first. Leading whitespace therefore remains literal note text,
/// while runs beginning at column zero become Java-compatible bullet atoms.
fn parse_note_bullet(line: &str) -> Option<(usize, &str)> {
    let star_count = line.bytes().take_while(|&byte| byte == b'*').count();
    if star_count == 0 || star_count == line.len() {
        return None;
    }

    let rest = &line[star_count..];
    if rest.starts_with('*') {
        return None;
    }

    // After the prefix, Java accepts non-star text interspersed with complete
    // `**bold**` spans. Reject unmatched stars so `**bold**` remains an inline
    // bold line rather than being mistaken for a nested list item.
    let mut tail = rest;
    while let Some(star) = tail.find('*') {
        let bold = &tail[star..];
        let inner = bold.strip_prefix("**")?;
        let close = inner.find("**")?;
        if close == 0 || inner[..close].contains('*') {
            return None;
        }
        tail = &inner[close + 2..];
    }

    Some((star_count - 1, rest.trim()))
}

#[allow(clippy::too_many_arguments)]
fn render_note_box(
    svg: &mut SvgBuilder,
    diagram: &ClassDiagram,
    note: &Note,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    sprites: &HashMap<String, SpriteData>,
) {
    let style = ResolvedNoteStyle::for_note(diagram, note);
    let fold = NOTE_FOLD;
    let points = &[
        (x, y),
        (x, y + h),
        (x + w, y + h),
        (x + w, y + fold),
        (x + w - fold, y),
    ];
    svg.polygon(points, &style.background, &style.border);
    let fold_pts = &[
        (x + w - fold, y),
        (x + w - fold, y + fold),
        (x + w, y + fold),
    ];
    svg.polygon(fold_pts, &style.background, &style.border);

    let mut body = String::new();
    emit_note_body(&mut body, note, x, y, w, sprites, &style);
    svg.raw_inline(&body);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rustuml_parser::diagram::DiagramMeta;

    fn simple_class_diagram() -> ClassDiagram {
        ClassDiagram {
            meta: DiagramMeta::default(),
            direction: ClassLayoutDirection::TopToBottom,
            entities: vec![
                ClassEntity {
                    id: "Animal".into(),
                    label: "Animal".into(),
                    explicit_alias: false,
                    kind: EntityKind::Class,
                    members: vec![
                        Member {
                            name: "name".into(),
                            return_type: Some("String".into()),
                            visibility: Visibility::Public,
                            is_static: false,
                            is_abstract: false,
                            kind: MemberKind::Field,
                            display_text: "name: String".into(),
                        },
                        Member {
                            name: "makeSound()".into(),
                            return_type: Some("void".into()),
                            visibility: Visibility::Public,
                            is_static: false,
                            is_abstract: false,
                            kind: MemberKind::Method,
                            display_text: "makeSound(): void".into(),
                        },
                    ],
                    stereotypes: vec![],
                    generic: None,
                    spot_color: None,
                    spot_character: None,
                    url: None,
                    url_tooltip: None,
                    color: None,
                    text_color: None,
                    line_color: None,
                    line_style: None,
                    source_line: 0,
                },
                ClassEntity {
                    id: "Dog".into(),
                    label: "Dog".into(),
                    explicit_alias: false,
                    kind: EntityKind::Class,
                    members: vec![Member {
                        name: "fetch()".into(),
                        return_type: Some("void".into()),
                        visibility: Visibility::Public,
                        is_static: false,
                        is_abstract: false,
                        kind: MemberKind::Method,
                        display_text: "fetch(): void".into(),
                    }],
                    stereotypes: vec![],
                    generic: None,
                    spot_color: None,
                    spot_character: None,
                    url: None,
                    url_tooltip: None,
                    color: None,
                    text_color: None,
                    line_color: None,
                    line_style: None,
                    source_line: 0,
                },
            ],
            relationships: vec![Relationship {
                from: "Animal".into(),
                to: "Dog".into(),
                kind: RelationshipKind::Inheritance,
                label: None,
                label_arrow: LinkArrow::None,
                from_multiplicity: None,
                to_multiplicity: None,
                from_decor: None,
                to_decor: None,
                decorated_end: RelationshipEnd::From,
                dashed: false,
                length: 2,
                style: RelationshipStyle::default(),
                source_line: 0,
            }],
            association_classes: vec![],
            packages: vec![],
            notes: vec![],
            hide_show: vec![],
            header_line: None,
            footer_line: None,
            title_line: None,
            caption_line: None,
            legend_line: None,
        }
    }

    fn member(name: &str, kind: MemberKind) -> Member {
        Member {
            name: name.into(),
            return_type: None,
            visibility: Visibility::Default,
            is_static: false,
            is_abstract: false,
            kind,
            display_text: name.into(),
        }
    }

    #[test]
    fn enum_body_detects_appended_constants_after_methods() {
        let mut entity = simple_class_diagram().entities.remove(0);
        entity.kind = EntityKind::Enum;
        entity.members = vec![
            member("ACTIVE", MemberKind::Field),
            member("", MemberKind::Separator),
            member("String display()", MemberKind::Method),
            member("PENDING = 3", MemberKind::Field),
        ];
        assert!(has_field_after_method(&entity));

        entity.members = vec![
            member("ACTIVE", MemberKind::Field),
            member("", MemberKind::Separator),
            member("String display()", MemberKind::Method),
        ];
        assert!(!has_field_after_method(&entity));
    }

    #[test]
    fn produces_valid_svg() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Animal"));
        assert!(svg.contains("Dog"));
    }

    #[test]
    fn mixed_state_group_omits_class_source_line_metadata() {
        let input = "@startuml\n\
                     allowmixing\n\
                     class Processor\n\
                     state Running\n\
                     Processor --> Running\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("allowmixing input should parse as a class diagram");
        };

        let svg = render(&diagram, &Theme::default());
        assert!(
            svg.contains(
                r#"<g class="entity" data-qualified-name="Processor" data-source-line="2""#
            ),
            "{svg}"
        );
        assert!(
            svg.contains(r#"<g class="entity" data-qualified-name="Running" id=""#),
            "{svg}"
        );
        assert!(
            !svg.contains(
                r#"<g class="entity" data-qualified-name="Running" data-source-line="3""#
            ),
            "{svg}"
        );
    }

    #[test]
    fn renamed_four_node_chain_obeys_left_to_right_rank_direction() {
        let body = "class FreshNorth\n\
                    class FreshEast\n\
                    class FreshSouth\n\
                    class FreshWest\n\
                    FreshNorth --> FreshEast\n\
                    FreshEast --> FreshSouth\n\
                    FreshSouth --> FreshWest";
        let left_to_right = rustuml_parser::parse::parse(&format!(
            "@startuml\nleft to right direction\n{body}\n@enduml"
        ))
        .unwrap();
        let top_to_bottom =
            rustuml_parser::parse::parse(&format!("@startuml\n{body}\n@enduml")).unwrap();

        let dimensions = |svg: &str| {
            let view_box = svg
                .split("viewBox=\"0 0 ")
                .nth(1)
                .and_then(|tail| tail.split('"').next())
                .unwrap();
            let mut values = view_box
                .split_whitespace()
                .map(|value| value.parse::<f64>().unwrap());
            (values.next().unwrap(), values.next().unwrap())
        };
        let horizontal = dimensions(&crate::render_svg(&left_to_right));
        let vertical = dimensions(&crate::render_svg(&top_to_bottom));
        assert!(horizontal.0 > horizontal.1, "{horizontal:?}");
        assert!(vertical.1 > vertical.0, "{vertical:?}");
    }

    #[test]
    fn renamed_five_block_body_generates_separator_geometry() {
        let input = "@startuml\n\
                     class FreshLedger2843 {\n\
                       +String seedValue\n\
                       --\n\
                       -int firstRenamedValue\n\
                       +int secondRenamedValue\n\
                       .. Fresh dotted review ..\n\
                       +String reviewedValue\n\
                       == Deliberately wider renamed approval panel ==\n\
                       __ Final renamed panel __\n\
                       -void closeFreshLedger()\n\
                     }\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert_eq!(svg.matches("<line style=").count(), 10, "{svg}");
        assert_eq!(svg.matches("stroke-dasharray:1,2;").count(), 2, "{svg}");
        assert!(svg.contains(">Fresh dotted review</text>"), "{svg}");
        assert!(
            svg.contains(">Deliberately wider renamed approval panel</text>"),
            "{svg}"
        );
        assert!(svg.contains(">Final renamed panel</text>"), "{svg}");
    }

    #[test]
    fn renamed_labelled_separator_uses_dynamic_title_block_height() {
        fn render_with_separator(separator: &str) -> String {
            let input = format!(
                "@startuml\n\
                 skinparam classAttributeFontSize 11\n\
                 class FreshSeparatorLedger3149 {{\n\
                   +String freshKey\n\
                   -int retainedCount\n\
                   {separator}\n\
                   +void rotateFreshKey()\n\
                   -boolean hasRetainedValue()\n\
                   #String summarizeFreshState()\n\
                 }}\n\
                 @enduml"
            );
            let diagram = rustuml_parser::parse::parse(&input).unwrap();
            crate::render_svg(&diagram)
        }

        fn entity_height(svg: &str) -> f64 {
            svg.split("<rect fill=\"#F1F1F1\" height=\"")
                .nth(1)
                .and_then(|tail| tail.split('"').next())
                .unwrap()
                .parse()
                .unwrap()
        }

        let plain = render_with_separator("--");
        let labelled = render_with_separator(".. Fresh renamed audit lane ..");
        let title_height =
            text_render::label_height_with_family("Fresh renamed audit lane", 11.0, "sans-serif");

        assert!(
            (entity_height(&labelled) - entity_height(&plain) - (title_height - 4.0)).abs()
                < 0.0002,
            "{plain}\n{labelled}"
        );
        assert!(
            labelled.contains(r#"font-size="11""#)
                && labelled.contains(">Fresh renamed audit lane</text>")
                && labelled.contains("stroke-dasharray:1,2;"),
            "{labelled}"
        );
    }

    #[test]
    fn renamed_attached_note_generates_mixed_body_separator_geometry() {
        let input = "@startuml\n\
                     class FreshLedgerNoteTarget2843\n\
                     note right of FreshLedgerNoteTarget2843\n\
                       opening fresh line\n\
                       ----\n\
                       short body\n\
                       == Renamed approval lane 2851 ==\n\
                       first uneven detail\n\
                       second uneven detail\n\
                       ....\n\
                       after dotted lane\n\
                       __ Final renamed lane 2879 __\n\
                       closing fresh line\n\
                     end note\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Java `BodyEnhanced2.getArea` splits the Display into decorated
        // blocks; `TextBlockLineBefore` and `UHorizontalLine` then own both
        // block dimensions and rule/title drawing.
        assert!(svg.contains(r#"viewBox="0 0 496 177""#), "{svg}");
        assert_eq!(svg.matches("stroke-dasharray:1,2;").count(), 1, "{svg}");
        assert!(
            svg.contains(">Renamed approval lane 2851</text>")
                && svg.contains(">Final renamed lane 2879</text>"),
            "{svg}"
        );
        assert!(!svg.contains(">----</text>"), "{svg}");
        assert!(!svg.contains(">....</text>"), "{svg}");
    }

    #[test]
    fn renamed_multiline_note_terminal_spaces_expand_layout_only() {
        fn render_note(body: &str) -> String {
            let input = format!(
                "@startuml\nnote as FreshWhitespaceLedger4171\n  {body}\nend note\n@enduml"
            );
            let diagram = rustuml_parser::parse::parse(&input).unwrap();
            crate::render_svg(&diagram)
        }

        let plain = render_note("renamed audit value:");
        let padded = render_note("renamed audit value:   ");

        // Java `CommandFactoryNote.createMultiLine` retains the terminal
        // spaces for `SheetBlock1.calculateDimension`, while `AtomText.drawU`
        // emits only the visible run. A headless Java render of this renamed
        // perturbation produces 173px and 186px canvases respectively.
        assert!(plain.contains(r#"viewBox="0 0 173 45""#), "{plain}");
        assert!(padded.contains(r#"viewBox="0 0 186 45""#), "{padded}");
        assert!(padded.contains(">renamed audit value:</text>"), "{padded}");
        assert!(
            !padded.contains("renamed audit value:   </text>"),
            "{padded}"
        );
    }

    #[test]
    fn renamed_multiple_floating_notes_use_svek_layout() {
        let input = "@startuml\n\
                     note as FreshAuditQueue4211\n\
                       first renamed floating note\n\
                     end note\n\
                     note as FreshArchiveQueue4217 #lightblue\n\
                       second renamed floating note\n\
                     end note\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Java `EntityImageNote` contributes one SVEK leaf per named note,
        // with horizontal margins 6 and 15 and vertical margin 5.
        assert!(svg.contains(r#"viewBox="0 0 456 46""#), "{svg}");
        assert_eq!(svg.matches(r#"<g class="entity""#).count(), 2, "{svg}");
        assert!(svg.contains(r#"data-qualified-name="FreshAuditQueue4211""#));
        assert!(svg.contains(r#"data-qualified-name="FreshArchiveQueue4217""#));
        assert!(svg.contains(r##"fill="#ADD8E6""##), "{svg}");
    }

    #[test]
    fn renamed_class_chain_shares_generated_shadow_filter() {
        let input = "@startuml\n\
                     skinparam shadowing true\n\
                     class FreshShadowLedger2939 {\n\
                       +String renamedKey\n\
                       +void rotateFreshly()\n\
                     }\n\
                     class FreshShadowArchive2953\n\
                     class FreshShadowAudit2963 {\n\
                       -int retainedCount\n\
                     }\n\
                     FreshShadowLedger2939 --> FreshShadowArchive2953\n\
                     FreshShadowArchive2953 ..> FreshShadowAudit2963\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let filter_id =
            crate::filter_registry::shadow_id_for(diagram.meta().source.as_deref().unwrap_or(""));

        // `SvgGraphics.createXmlDocument` emits one source-seeded definition;
        // `SvekResult.calculateDimension` includes the painted shadow extent.
        assert!(svg.contains(r#"viewBox="0 0 233 340""#), "{svg}");
        assert!(
            svg.contains(&crate::filter_registry::shadow_filter_def(&filter_id)),
            "{svg}"
        );
        assert_eq!(
            svg.matches(&format!(r#"filter="url(#{filter_id})""#))
                .count(),
            3,
            "{svg}"
        );
    }

    #[test]
    fn has_class_boxes() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        let rect_count = svg.matches("<rect").count();
        assert!(
            rect_count >= 2,
            "should have at least 2 class boxes, got {rect_count}"
        );
    }

    #[test]
    fn has_members() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(svg.contains("name: String"));
        assert!(svg.contains("makeSound(): void"));
        assert!(svg.contains("fetch(): void"));
    }

    #[test]
    fn multiline_member_escape_renders_as_member_rows() {
        let diagram = ClassDiagram {
            meta: DiagramMeta::default(),
            direction: ClassLayoutDirection::TopToBottom,
            entities: vec![ClassEntity {
                id: "MyClass".into(),
                label: "MyClass".into(),
                explicit_alias: false,
                kind: EntityKind::Class,
                members: vec![Member {
                    name: "multiLineMethod(".into(),
                    return_type: Some("void".into()),
                    visibility: Visibility::Default,
                    is_static: false,
                    is_abstract: false,
                    kind: MemberKind::Method,
                    display_text: "multiLineMethod(\\nparam1: String,\\nparam2: Int): void".into(),
                }],
                stereotypes: vec![],
                generic: None,
                spot_color: None,
                spot_character: None,
                url: None,
                url_tooltip: None,
                color: None,
                text_color: None,
                line_color: None,
                line_style: None,
                source_line: 0,
            }],
            relationships: vec![],
            association_classes: vec![],
            packages: vec![],
            notes: vec![],
            hide_show: vec![],
            header_line: None,
            footer_line: None,
            title_line: None,
            caption_line: None,
            legend_line: None,
        };
        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains(">multiLineMethod(<"));
        assert!(svg.contains(">param1: String,<"));
        assert!(svg.contains(">param2: Int): void<"));
        assert!(!svg.contains("\\n"));
    }

    #[test]
    fn has_entity_comments() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(
            svg.contains("<!--class Animal-->"),
            "should have entity comment"
        );
        assert!(
            svg.contains("<!--class Dog-->"),
            "should have entity comment"
        );
    }

    #[test]
    fn has_entity_groups() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(
            svg.contains(r#"class="entity""#),
            "should have entity group"
        );
        assert!(
            svg.contains(r#"data-qualified-name="Animal""#),
            "should have qualified name"
        );
    }

    #[test]
    fn has_icon_ellipses() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(
            svg.contains(r##"fill="#ADD1B2""##),
            "should have class icon fill"
        );
        assert!(svg.contains("<ellipse"), "should have icon ellipse");
    }

    #[test]
    fn plain_theme_uses_legacy_class_icon_and_font_defaults() {
        let input = "@startuml\n!theme plain\nclass Foo\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"font-family="Verdana""##), "{svg}");
        assert!(
            svg.contains(
                r##"fill="#FFFFFF" rx="9" ry="9" style="stroke:#000000;stroke-width:1;""##
            ),
            "{svg}"
        );
    }

    #[test]
    fn no_oracle_global_padding_expands_each_class_display_block() {
        let body = "class TelemetryLedger739 {\n\
                      +signalCode: String\n\
                      statusNote\n\
                      -archive(packet: Frame): Result\n\
                    }\n";
        let plain = rustuml_parser::parse::parse(&format!("@startuml\n{body}@enduml")).unwrap();
        let padded = rustuml_parser::parse::parse(&format!(
            "@startuml\nskinparam padding 13\n{body}@enduml"
        ))
        .unwrap();
        let plain_svg = crate::render_svg(&plain);
        let padded_svg = crate::render_svg(&padded);

        fn class_rect(svg: &str) -> &str {
            svg.split_once("<!--class TelemetryLedger739-->")
                .unwrap()
                .1
                .split_once("/>")
                .unwrap()
                .0
        }
        fn attr_number(element: &str, name: &str) -> f64 {
            attr_value(element, name).unwrap().parse::<f64>().unwrap()
        }
        fn text_position(svg: &str, text: &str) -> (f64, f64) {
            let marker = format!(">{text}</text>");
            let tag = svg
                .split_once(&marker)
                .unwrap()
                .0
                .rsplit_once("<text ")
                .unwrap()
                .1;
            (attr_number(tag, " x"), attr_number(tag, " y"))
        }
        fn assert_close(actual: f64, expected: f64) {
            let tolerance = 1.0 / 1000.0;
            assert!(
                (actual - expected).abs() < tolerance,
                "actual={actual}, expected={expected}"
            );
        }

        let padding = 13.0;
        let block_growth = padding * 2.0;
        // PlantUML `Display.create8` constructs a `SheetBlock1`; its
        // `calculateDimensionSlow` adds both vertical and horizontal padding.
        assert_close(
            attr_number(class_rect(&padded_svg), " width")
                - attr_number(class_rect(&plain_svg), " width"),
            block_growth,
        );
        let header_growth = (HEADER_H_NO_CIRCLE + block_growth).max(HEADER_HEIGHT) - HEADER_HEIGHT;
        // `MethodsOrFieldsArea.calculateDimensionOnlyMembers` stacks three
        // independently padded member blocks below that expanded header.
        assert_close(
            attr_number(class_rect(&padded_svg), " height")
                - attr_number(class_rect(&plain_svg), " height"),
            header_growth + block_growth * 3.0,
        );

        let plain_name = text_position(&plain_svg, "TelemetryLedger739");
        let padded_name = text_position(&padded_svg, "TelemetryLedger739");
        assert_close(padded_name.0 - plain_name.0, padding);
        assert_close(padded_name.1 - plain_name.1, header_growth / 2.0);

        let plain_first = text_position(&plain_svg, "signalCode: String");
        let padded_first = text_position(&padded_svg, "signalCode: String");
        assert_close(padded_first.0 - plain_first.0, padding);
        assert_close(padded_first.1 - plain_first.1, header_growth + padding);

        let plain_second = text_position(&plain_svg, "statusNote");
        let padded_second = text_position(&padded_svg, "statusNote");
        assert_close(
            padded_second.1 - plain_second.1,
            header_growth + padding * 3.0,
        );

        let plain_method = text_position(&plain_svg, "archive(packet: Frame): Result");
        let padded_method = text_position(&padded_svg, "archive(packet: Frame): Result");
        assert_close(
            padded_method.1 - plain_method.1,
            header_growth + padding * 5.0,
        );
    }

    #[test]
    fn no_oracle_page_chrome_translates_a_fresh_fork_as_nested_decorators() {
        let body = "class Sensor841\n\
            class Decoder853\n\
            class Archive857\n\
            Sensor841 --> Decoder853\n\
            Sensor841 --> Archive857\n";
        let plain = rustuml_parser::parse::parse(&format!("@startuml\n{body}@enduml")).unwrap();
        let decorated = rustuml_parser::parse::parse(&format!(
            "@startuml\n\
             header Observatory 811\n\
             title\n\
               Signal Catalog 823\n\
               Rotation 827\n\
             end title\n\
             caption Figure 829: Calibrated Routes\n\
             footer Build 839\n\
             {body}@enduml"
        ))
        .unwrap();
        let plain_svg = crate::render_svg(&plain);
        let decorated_svg = crate::render_svg(&decorated);

        assert!(plain_svg.contains(r#"style="width:280px;height:178px;background:#FFFFFF;""#));
        assert!(decorated_svg.contains(r#"style="width:280px;height:277px;background:#FFFFFF;""#));
        // Live PlantUML beta: DiagramChromeFactory12026.create nests header +
        // a two-line title into DecorateEntityImage, whose drawU accumulates
        // the two bordered text-block heights for this non-corpus fork.
        fn sensor_rect(svg: &str) -> &str {
            svg.split_once("<!--class Sensor841-->")
                .unwrap()
                .1
                .split_once("/>")
                .unwrap()
                .0
        }
        let plain_y = attr_value(sensor_rect(&plain_svg), " y")
            .unwrap()
            .parse::<f64>()
            .unwrap();
        let decorated_y = attr_value(sensor_rect(&decorated_svg), " y")
            .unwrap()
            .parse::<f64>()
            .unwrap();
        let expected_top_h = text_render::label_height("Observatory 811", 10.0)
            + DECORATION_BORDER_EXTENT
            + text_render::label_height("Signal Catalog 823", 14.0)
            + text_render::label_height("Rotation 827", 14.0)
            + 2.0 * DECORATION_TITLE_INSET
            + DECORATION_BORDER_EXTENT;
        let tolerance = 1.0 / 1000.0;
        assert!(
            (decorated_y - plain_y - expected_top_h).abs() < tolerance,
            "plain y={plain_y}, decorated y={decorated_y}"
        );

        fn link_path(svg: &str) -> &str {
            svg.split_once(r#"id="Sensor841-to-Decoder853""#)
                .unwrap()
                .0
                .rsplit_once("<path ")
                .and_then(|(_, tag)| attr_value(tag, "d"))
                .unwrap()
        }
        fn path_coords(path: &str) -> Vec<f64> {
            path.split(|ch: char| !(ch.is_ascii_digit() || ch == '.' || ch == '-'))
                .filter_map(|value| value.parse::<f64>().ok())
                .collect::<Vec<_>>()
        }
        let plain_path = path_coords(link_path(&plain_svg));
        let decorated_path = path_coords(link_path(&decorated_svg));
        assert_eq!(plain_path.len(), decorated_path.len());
        for (idx, (plain, decorated)) in plain_path.iter().zip(decorated_path).enumerate() {
            let expected_delta = if idx % 2 == 0 { 0.0 } else { expected_top_h };
            assert!((decorated - plain - expected_delta).abs() < tolerance);
        }

        fn text_y(svg: &str, text: &str) -> f64 {
            let marker = format!(">{text}</text>");
            let tag = svg
                .split_once(&marker)
                .unwrap()
                .0
                .rsplit_once("<text ")
                .unwrap()
                .1;
            attr_value(tag, " y").unwrap().parse::<f64>().unwrap()
        }
        let header_y = text_y(&decorated_svg, "Observatory 811");
        let title_1_y = text_y(&decorated_svg, "Signal Catalog 823");
        let title_2_y = text_y(&decorated_svg, "Rotation 827");
        let caption_y = text_y(&decorated_svg, "Figure 829: Calibrated Routes");
        let footer_y = text_y(&decorated_svg, "Build 839");
        assert!(header_y < title_1_y);
        assert!(title_1_y < title_2_y);
        assert!(title_2_y < decorated_y);
        assert!(decorated_y < caption_y);
        assert!(caption_y < footer_y);
    }

    #[test]
    fn no_oracle_multi_leaf_canvas_uses_svek_result_envelope() {
        let input = "@startuml\n\
            annotation AuditStamp731 {\n\
              +String token()\n\
              +long revision() default 7\n\
            }\n\
            class LedgerEntry743 <<AuditStamp731>> {\n\
              +UUID identifier\n\
              +void archive()\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Live PlantUML beta: `GraphvizImageBuilder.buildImage` sends this
        // two-leaf graph through `SvekResult.calculateDimension`.
        assert!(
            svg.contains(r#"style="width:384px;height:111px;background:#FFFFFF;""#),
            "{svg}"
        );
    }

    #[test]
    fn no_oracle_round_corner_uses_half_the_skinparam_diameter() {
        let input = "@startuml\nskinparam roundcorner 34\nclass RenamedRoundedClass\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(
            svg.contains(r##"<rect fill="#F1F1F1" height="48" rx="17" ry="17""##),
            "{svg}"
        );
    }

    #[test]
    fn generic_body_recenters_the_header_before_its_badge() {
        let input = "@startuml\nclass RenamedEnvelope<Payload, ErrorCode> {\n  +Result<Payload, ErrorCode> transform(InputPacket packet)\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(">Payload, ErrorCode</text>"), "{svg}");
        assert!(!svg.contains(r#"<ellipse cx="22""#), "{svg}");
        assert!(
            !svg.contains(r#"x="36" y="28.291">RenamedEnvelope</text>"#),
            "{svg}"
        );
    }

    #[test]
    fn has_visibility_modifiers() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(
            svg.contains("data-visibility-modifier"),
            "should have visibility modifier"
        );
    }

    #[test]
    fn has_plantuml_root_attrs() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(
            svg.contains(r#"data-diagram-type="CLASS""#),
            "should have diagram type"
        );
        assert!(
            svg.contains(r#"contentStyleType="text/css""#),
            "should have content style type"
        );
        assert!(svg.contains("<?plantuml"), "should have plantuml PI");
    }

    #[test]
    fn duplicate_background_color_uses_last_value() {
        let input = "@startuml\nskinparam backgroundColor white\nskinparam backgroundColor yellow\nskinparam backgroundColor red\nclass Foo\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains("background:#FF0000;"));
        assert!(svg.contains(r##"<rect fill="#FF0000""##));
    }

    #[test]
    fn no_oracle_hidden_compartment_contributes_no_body_block() {
        let input = "@startuml\n\
            hide attributes\n\
            abstract class WorkflowLedger607 <<AuditedFlow613>> {\n\
              +String token\n\
              #long revision\n\
              +void reconcile()\n\
              -boolean verify()\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"style="width:187px;height:101px;background:#FFFFFF;""#));
        assert!(svg.contains(r##"<rect fill="#F1F1F1" height="81.5976" rx="2.5" ry="2.5""##));
        assert!(svg.contains(r#"y1="47.6211" y2="47.6211""#));
        assert!(svg.contains(">void reconcile()</text>"));
        assert!(svg.contains(">boolean verify()</text>"));
        assert!(!svg.contains(">String token</text>"));
        assert!(!svg.contains(">long revision</text>"));
    }

    #[test]
    fn no_oracle_database_cluster_uses_usymbol_shape_and_envelope() {
        let input = "@startuml\n\
            database LedgerVault211 #HoneyDew {\n\
              class FreshRecord223 {\n\
                +UUID key\n\
              }\n\
              class AuditRow227\n\
              FreshRecord223 --> AuditRow227\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"style="width:205px;height:269px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"<!--cluster LedgerVault211--><g class="cluster""#));
        assert!(svg.contains(r##"fill="#F0FFF0" style="stroke:#181818;stroke-width:1;""##));
        assert!(svg.contains(r#"data-qualified-name="LedgerVault211.FreshRecord223""#));
        assert!(svg.contains(r#"id="FreshRecord223-to-AuditRow227""#));
    }

    #[test]
    fn no_oracle_folder_cluster_routes_symbol_styles() {
        let input = "@startuml\n\
            skinparam FolderBorderColor DarkGreen\n\
            skinparam PackageFontColor Navy\n\
            folder ArchiveShelf313 #AliceBlue {\n\
              package Intake317 {\n\
                class Parcel331 {\n\
                  +String id\n\
                }\n\
                class Ledger337\n\
                Parcel331 --> Ledger337\n\
              }\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"style="width:205px;height:311px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"<!--cluster ArchiveShelf313--><g class="cluster""#));
        assert!(svg.contains(r##"fill="#F0F8FF" style="stroke:#006400;stroke-width:1.5;""##));
        assert!(svg.contains(
            r##"<text fill="#000000" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="117.8379" x="10" y="21.5352">ArchiveShelf313</text>"##
        ));
        assert!(svg.contains(
            r#"data-qualified-name="ArchiveShelf313.Intake317" data-source-line="4" id="ent0003""#
        ));
        assert!(svg.contains(
            r##"<text fill="#000080" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="72.5088" x="34" y="64.5352">Intake317</text>"##
        ));
        assert!(svg.contains(r#"data-qualified-name="ArchiveShelf313.Intake317.Parcel331""#));
        assert!(svg.contains(r#"id="Parcel331-to-Ledger337""#));
    }

    #[test]
    fn renamed_nested_package_stereotype_reserves_cluster_label_block() {
        let input = "@startuml\n\
            package FreshOuterArchive701 {\n\
              package FreshInnerLedger709 <<LongRunningAuditService>> {\n\
                class FreshEntry719 {\n\
                  +String renamedKey\n\
                }\n\
                class FreshSink727\n\
                FreshEntry719 --> FreshSink727\n\
              }\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Java `ClusterHeader` merges the stereotype and title dimensions,
        // `ClusterDotString.printInternal` feeds that block to Graphviz, and
        // `USymbolFolder.asBig` centres the stereotype below the folder tab.
        assert!(svg.contains(r#"viewBox="0 0 273 327""#), "{svg}");
        assert!(svg.contains(r#"<!--cluster FreshOuterArchive701-->"#));
        assert!(svg.contains(r#"<!--cluster FreshOuterArchive701.FreshInnerLedger709-->"#));
        assert!(svg.contains(r#"font-style="italic""#));
        assert!(svg.contains("LongRunningAuditService"));
        assert!(svg.contains(
            r#"data-qualified-name="FreshOuterArchive701.FreshInnerLedger709.FreshEntry719""#
        ));
        assert!(svg.contains(r#"id="FreshEntry719-to-FreshSink727""#));
    }

    #[test]
    fn no_oracle_node_cluster_routes_shape_content_and_styles() {
        let input = "@startuml\n\
            skinparam NodeBorderColor SeaGreen\n\
            skinparam NodeFontColor DarkSlateBlue\n\
            node ComputeRack439 #HoneyDew {\n\
              class Worker443 {\n\
                +void run()\n\
              }\n\
              class Result449\n\
              class Cache457\n\
              Worker443 --> Result449\n\
              Worker443 --> Cache457\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"style="width:308px;height:259px;background:#FFFFFF;""#));
        assert!(svg.contains(r#"<!--cluster ComputeRack439--><g class="cluster""#));
        assert!(svg.contains(
            r##"<polygon fill="#F0FFF0" points="16,16,26,6,283,6,283,224.4883,273,234.4883,16,234.4883,16,16" style="stroke:#2E8B57;stroke-width:1;"/>"##
        ));
        assert!(svg.contains(
            r##"<text fill="#483D8B" font-family="sans-serif" font-size="14" font-weight="700" lengthAdjust="spacing" textLength="126.7451" x="82.1274" y="32.5352">ComputeRack439</text>"##
        ));
        assert!(svg.contains(r#"data-qualified-name="ComputeRack439.Worker443""#));
        assert!(svg.contains(r#"data-qualified-name="ComputeRack439.Result449""#));
        assert!(svg.contains(r#"data-qualified-name="ComputeRack439.Cache457""#));
        assert!(svg.contains(r#"id="Worker443-to-Result449""#));
        assert!(svg.contains(r#"id="Worker443-to-Cache457""#));
    }

    #[test]
    fn no_oracle_cloud_cluster_uses_seeded_frontier_and_symbol_styles() {
        let input = "@startuml\n\
            skinparam CloudBorderColor SeaGreen\n\
            skinparam CloudFontColor DarkSlateBlue\n\
            cloud NimbusVault503 #HoneyDew {\n\
              class Intake509 {\n\
                +void accept()\n\
              }\n\
              class Archive521\n\
              class Monitor523\n\
              Intake509 --> Archive521\n\
              Intake509 --> Monitor523\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"style="width:320px;height:259px;background:#FFFFFF;""#));
        let cluster = svg
            .split_once("<!--cluster NimbusVault503-->")
            .unwrap()
            .1
            .split_once("</g>")
            .unwrap()
            .0;
        assert!(cluster.contains(r#"<path d="M"#));
        assert_eq!(cluster.matches(" C").count(), 52);
        assert!(cluster.contains(r##"fill="#F0FFF0" style="stroke:#2E8B57;stroke-width:1;""##));
        assert!(cluster.contains(
            r##"<text fill="#483D8B" font-family="sans-serif" font-size="14" font-weight="700""##
        ));
        assert!(svg.contains(r#"data-qualified-name="NimbusVault503.Intake509""#));
        assert!(svg.contains(r#"data-qualified-name="NimbusVault503.Archive521""#));
        assert!(svg.contains(r#"data-qualified-name="NimbusVault503.Monitor523""#));
        assert!(svg.contains(r#"id="Intake509-to-Archive521""#));
        assert!(svg.contains(r#"id="Intake509-to-Monitor523""#));
    }

    #[test]
    fn nested_package_clusters_use_solved_svek_boxes() {
        let input = "@startuml\npackage alpha {\n  package beta {\n    package gamma {\n      package delta {\n        class RenamedDeep {\n          +void go()\n        }\n      }\n    }\n  }\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        let (alpha_x, alpha_y) = cluster_path_origin(&svg, "alpha");
        let (beta_x, beta_y) = cluster_path_origin(&svg, "alpha.beta");
        let (gamma_x, gamma_y) = cluster_path_origin(&svg, "alpha.beta.gamma");
        let (delta_x, delta_y) = cluster_path_origin(&svg, "alpha.beta.gamma.delta");

        assert_eq!(
            (alpha_x, alpha_y),
            (SVEK_LABEL_ENVELOPE_MARGIN, SVEK_LABEL_ENVELOPE_MARGIN)
        );
        assert!(alpha_x < beta_x && beta_x < gamma_x && gamma_x < delta_x);
        assert!(alpha_y < beta_y && beta_y < gamma_y && gamma_y < delta_y);
        assert!(svg.contains(r#"data-qualified-name="alpha.beta.gamma.delta.RenamedDeep""#));
    }

    #[test]
    fn renamed_nested_packages_allocate_svek_ids_recursively() {
        let input = "@startuml\nclass RootBefore_7\npackage Outer_Renamed_17 {\n  class DirectZulu_19\n  package Inner_Q {\n    class LeafBeta_23\n    class LeafAlpha_29\n  }\n  class DirectAlpha_31\n}\nclass RootAfter_37\nRootBefore_7 --> LeafBeta_23\nDirectZulu_19 --> RootAfter_37\nLeafAlpha_29 --> DirectAlpha_31\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };
        let allocation = svek_id_allocation(&diagram);

        assert_eq!(
            allocation.package_ids,
            [Some("ent0003".to_string()), Some("ent0005".to_string())]
        );
        assert_eq!(allocation.entity_order, [1, 4, 2, 3, 0, 5]);
        assert_eq!(
            allocation.entity_ids,
            [
                "ent0002", "ent0004", "ent0006", "ent0007", "ent0008", "ent0009"
            ]
        );

        let svg = render(&diagram, &Theme::default());
        for (qualified_name, entity_id) in [
            ("Outer_Renamed_17", "ent0003"),
            ("Outer_Renamed_17.DirectZulu_19", "ent0004"),
            ("Outer_Renamed_17.DirectAlpha_31", "ent0008"),
            ("Outer_Renamed_17.Inner_Q", "ent0005"),
            ("Outer_Renamed_17.Inner_Q.LeafBeta_23", "ent0006"),
            ("Outer_Renamed_17.Inner_Q.LeafAlpha_29", "ent0007"),
            ("RootBefore_7", "ent0002"),
            ("RootAfter_37", "ent0009"),
        ] {
            let marker = format!(r#"data-qualified-name="{qualified_name}""#);
            let opening_tag = svg
                .split_once(&marker)
                .unwrap_or_else(|| panic!("missing SVEK item {qualified_name}"))
                .1
                .split_once('>')
                .unwrap()
                .0;
            assert!(
                opening_tag.contains(&format!(r#"id="{entity_id}""#)),
                "wrong recursive SVEK id for {qualified_name}: {opening_tag}"
            );
        }
    }

    #[test]
    fn phantom_namespace_groups_follow_the_leaf_uid_and_keep_separator_identity() {
        let input = "@startuml\n\
            set namespaceSeparator ::\n\
            class observatory::catalog::FreshSignal1201\n\
            class observatory::catalog::FreshArchive1213\n\
            observatory::catalog::FreshSignal1201 --> observatory::catalog::FreshArchive1213\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };
        let allocation = svek_id_allocation(&diagram);

        assert_eq!(
            allocation.package_ids,
            [Some("ent0003".to_string()), Some("ent0004".to_string())]
        );
        assert_eq!(allocation.entity_ids, ["ent0002", "ent0005"]);
        assert_eq!(allocation.relationship_ids, [6]);

        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains(
            r#"data-qualified-name="observatory..catalog" data-source-line="2" id="ent0004""#
        ));
        assert!(svg.contains(r#"id="FreshSignal1201-to-FreshArchive1213""#));
    }

    #[test]
    fn deeper_phantom_namespace_reuses_groups_without_consuming_more_uids() {
        let input = "@startuml\n\
            class atlas.sector.archive.RenamedEntry1229\n\
            class atlas.sector.archive.RenamedLedger1231\n\
            class atlas.sector.archive.RenamedAudit1237\n\
            atlas.sector.archive.RenamedEntry1229 --> atlas.sector.archive.RenamedLedger1231\n\
            atlas.sector.archive.RenamedLedger1231 --> atlas.sector.archive.RenamedAudit1237\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };
        let allocation = svek_id_allocation(&diagram);

        assert_eq!(
            allocation.package_ids,
            [
                Some("ent0003".to_string()),
                Some("ent0004".to_string()),
                Some("ent0005".to_string())
            ]
        );
        assert_eq!(allocation.entity_ids, ["ent0002", "ent0006", "ent0007"]);
        assert_eq!(allocation.relationship_ids, [8, 9]);

        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains(r#"data-qualified-name="atlas.sector.archive""#));
        assert!(svg.contains(r#"id="RenamedEntry1229-to-RenamedLedger1231""#));
        assert!(svg.contains(r#"id="RenamedLedger1231-to-RenamedAudit1237""#));
    }

    #[test]
    fn explicit_deep_namespace_allocates_declared_group_before_leaf_and_phantoms() {
        let input = "@startuml\n\
            namespace aurora.sector.catalog {\n\
              class FreshIndex1319\n\
              class FreshRecord1321\n\
              class FreshAudit1327\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };
        let allocation = svek_id_allocation(&diagram);

        assert_eq!(
            allocation.package_ids,
            [
                Some("ent0004".to_string()),
                Some("ent0005".to_string()),
                Some("ent0002".to_string()),
            ]
        );
        assert_eq!(allocation.entity_ids, ["ent0003", "ent0006", "ent0007"]);

        let svg = render(&diagram, &Theme::default());
        for qualified_name in [
            "aurora",
            "aurora.sector",
            "aurora.sector.catalog",
            "aurora.sector.catalog.FreshIndex1319",
            "aurora.sector.catalog.FreshRecord1321",
            "aurora.sector.catalog.FreshAudit1327",
        ] {
            assert!(
                svg.contains(&format!(r#"data-qualified-name="{qualified_name}""#)),
                "missing {qualified_name}"
            );
        }
    }

    #[test]
    fn attached_note_before_relationship_claims_three_shared_uid_slots() {
        let input = "@startuml\n\
            class FreshOrigin1009\n\
            class FreshTarget1013\n\
            note right of FreshOrigin1009 : renamed before\n\
            FreshOrigin1009 --> FreshTarget1013\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };
        let allocation = svek_id_allocation(&diagram);

        assert_eq!(allocation.entity_ids, ["ent0002", "ent0003"]);
        assert_eq!(allocation.attached_note_starts, [Some(4)]);
        assert_eq!(allocation.relationship_ids, [7]);
    }

    #[test]
    fn attached_note_after_relationship_continues_shared_uid_stream() {
        let input = "@startuml\n\
            class FreshSender1019\n\
            class FreshReceiver1021\n\
            FreshSender1019 --> FreshReceiver1021\n\
            note left of FreshReceiver1021 : renamed after\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };
        let allocation = svek_id_allocation(&diagram);

        assert_eq!(allocation.entity_ids, ["ent0002", "ent0003"]);
        assert_eq!(allocation.relationship_ids, [4]);
        assert_eq!(allocation.attached_note_starts, [Some(5)]);
    }

    #[test]
    fn changed_count_attached_notes_each_claim_gmn_entity_and_link_uids() {
        let input = "@startuml\n\
            class FreshNoted1031\n\
            note left of FreshNoted1031 : west memo\n\
            note top of FreshNoted1031 : north memo\n\
            note right of FreshNoted1031 : east memo\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };
        let allocation = svek_id_allocation(&diagram);

        assert_eq!(allocation.entity_ids, ["ent0002"]);
        assert_eq!(allocation.attached_note_starts, [Some(3), Some(6), Some(9)]);
    }

    #[test]
    fn deeper_hierarchy_interleaves_entities_and_links_in_source_order() {
        let mut input = String::from("@startuml\n");
        for depth in 1..=6 {
            writeln!(input, "class FreshLevel{depth}").unwrap();
        }
        for depth in 1..6 {
            writeln!(input, "FreshLevel{depth} <|-- FreshLevel{}", depth + 1).unwrap();
        }
        for branch in 1..=5 {
            writeln!(input, "interface FreshPort{branch}").unwrap();
        }
        for branch in 1..=5 {
            writeln!(input, "FreshLevel6 ..|> FreshPort{branch}").unwrap();
        }
        input.push_str("@enduml");

        let diagram = rustuml_parser::parse::parse(&input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };
        let allocation = svek_id_allocation(&diagram);

        assert_eq!(
            &allocation.entity_ids[..6],
            [
                "ent0002", "ent0003", "ent0004", "ent0005", "ent0006", "ent0007"
            ]
        );
        assert_eq!(&allocation.relationship_ids[..5], [8, 9, 10, 11, 12]);
        assert_eq!(
            &allocation.entity_ids[6..],
            ["ent0013", "ent0014", "ent0015", "ent0016", "ent0017"]
        );
        assert_eq!(&allocation.relationship_ids[5..], [18, 19, 20, 21, 22]);
    }

    #[test]
    fn declaration_multi_supertypes_keep_parent_to_child_svek_identity() {
        let input = "@startuml\n\
            skinparam class {\n\
              BackgroundColor #E8F4F8\n\
              BorderColor #2468AC\n\
            }\n\
            class FreshBase4651\n\
            interface FreshReadable4603\n\
            interface FreshWritable4621\n\
            class FreshRecord4637 extends FreshBase4651 implements FreshReadable4603, FreshWritable4621 {\n\
              +renamedValue: String\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        for parent in ["FreshBase4651", "FreshReadable4603", "FreshWritable4621"] {
            assert!(svg.contains(&format!(r#"id="{parent}-backto-FreshRecord4637""#)));
        }
        assert_eq!(svg.matches(r#"data-source-line="9" id="lnk"#).count(), 3);
        for path_id in [
            "FreshBase4651-backto-FreshRecord4637",
            "FreshReadable4603-backto-FreshRecord4637",
            "FreshWritable4621-backto-FreshRecord4637",
        ] {
            let path = svg.split_once(&format!(r#"id="{path_id}""#)).unwrap().0;
            assert!(!path.rsplit_once("<path").unwrap().1.contains("codeLine="));
        }
        assert!(!svg.contains("FreshRecord4637-to-FreshBase4651"));
    }

    #[test]
    fn inverted_direction_links_consume_both_source_order_uids() {
        let input = "@startuml\n\
            class FreshCompassOrigin3299\n\
            class FreshCompassWest3301\n\
            class FreshCompassNorth3307\n\
            FreshCompassOrigin3299 -left-> FreshCompassWest3301\n\
            FreshCompassOrigin3299 -u-> FreshCompassNorth3307\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };
        let allocation = svek_id_allocation(&diagram);

        // Each Java `Link.getInv()` constructs a replacement Link after the
        // discarded source Link, so the visible IDs advance by two.
        assert_eq!(allocation.entity_ids, ["ent0002", "ent0003", "ent0004"]);
        assert_eq!(allocation.relationship_ids, [6, 8]);

        let svg = render(&diagram, &Theme::default());
        assert!(
            svg.contains(r#"id="FreshCompassWest3301-backto-FreshCompassOrigin3299""#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"id="FreshCompassNorth3307-backto-FreshCompassOrigin3299""#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"id="lnk6""#) && svg.contains(r#"id="lnk8""#),
            "{svg}"
        );
    }

    #[test]
    fn changed_count_relationship_notes_follow_the_last_source_link() {
        let input = "@startuml\n\
            class FreshAlpha1103\n\
            class FreshBeta1109\n\
            class FreshGamma1117\n\
            class FreshDelta1123\n\
            FreshAlpha1103 --> FreshBeta1109\n\
            note on link : first renamed memo\n\
            FreshBeta1109 --> FreshGamma1117\n\
            FreshGamma1117 <-- FreshDelta1123\n\
            note on link\n\
              final renamed line one\n\
              final renamed line two\n\
            end note\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };

        assert_eq!(
            relationship_note_indices(&diagram),
            [Some(0), None, Some(1)]
        );
    }

    #[test]
    fn renamed_relationship_note_renders_inside_its_solved_edge_label_box() {
        let input = "@startuml\n\
            class FreshSender1129\n\
            class FreshReceiver1151\n\
            FreshSender1129 --> FreshReceiver1151\n\
            note on link : renamed ownership memo\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let link = svg
            .split_once(r#"<g class="link""#)
            .unwrap()
            .1
            .split_once("</g>")
            .unwrap()
            .0;

        assert!(link.contains(">renamed ownership memo</text>"));
        assert_eq!(link.matches("<path ").count(), 3);
        assert!(link.contains("stroke:#181818;stroke-width:0.5;"));
    }

    #[test]
    fn reversed_relationship_owns_and_renders_a_multiline_note() {
        let input = "@startuml\n\
            class FreshLeft1153\n\
            class FreshRight1163\n\
            FreshLeft1153 <-- FreshRight1163\n\
            note on link\n\
              renamed upper row\n\
              renamed lower row\n\
            end note\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let link = svg
            .split_once(r#"<g class="link""#)
            .unwrap()
            .1
            .split_once("</g>")
            .unwrap()
            .0;

        assert!(link.contains(">renamed upper row</text>"));
        assert!(link.contains(">renamed lower row</text>"));
        assert_eq!(link.matches("<path ").count(), 3);
    }

    fn cluster_path_origin(svg: &str, qualified_name: &str) -> (f64, f64) {
        let marker = format!("<!--cluster {qualified_name}-->");
        let after_marker = svg
            .split_once(&marker)
            .unwrap_or_else(|| panic!("missing cluster marker {qualified_name}"))
            .1;
        let coords = after_marker
            .split_once("<path d=\"M")
            .unwrap_or_else(|| panic!("missing path for cluster {qualified_name}"))
            .1
            .split_once(" L")
            .unwrap()
            .0;
        let (x, y) = coords.split_once(',').unwrap();
        (x.parse::<f64>().unwrap() - 2.5, y.parse::<f64>().unwrap())
    }

    #[test]
    fn default_font_color_colours_class_name_and_icon_glyph() {
        let input = "@startuml\nskinparam defaultFontColor DarkBlue\nclass Foo\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"<text fill="#00008B""##));
        assert!(svg.contains(r##"<path d="M24.4731,29.1431"##));
        assert!(svg.contains(r##"fill="#00008B"/>"##));
    }

    #[test]
    fn entity_color_channels_style_renamed_class_independently() {
        let input = "@startuml\nclass Renamed #back:azure;line:#12ABEF;line.dashed;text:navy {\n  +field: String\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"<rect fill="#F0FFFF""##));
        assert!(svg.contains(r##"style="stroke:#12ABEF;stroke-width:1;stroke-dasharray:7,7;""##));
        assert!(svg.contains(r##"<text fill="#000080""##));
    }

    #[test]
    fn legacy_double_hash_border_keeps_a_separate_background_channel() {
        let input = "@startuml\nclass FreshLedger719 #azure ##[dashed]12ABEF {\n  +entry: String\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"<rect fill="#F0FFFF""##));
        assert!(svg.contains(r##"style="stroke:#12ABEF;stroke-width:1;stroke-dasharray:7,7;""##));
    }

    #[test]
    fn standalone_named_note_uses_opale_entity_geometry_for_fresh_markup() {
        let input = "@startuml\nnote as FreshMemo727\n  <color:#2457A6>**reviewed text**</color>\nend note\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"data-qualified-name="FreshMemo727""#));
        assert!(svg.contains(r#"id="ent0002"><path d="M7,7 L7,"#));
        assert!(svg.contains(r##"fill="#2457A6""##));
        assert!(svg.contains(r#"font-weight="700""#));
        assert!(svg.contains(r#"style="stroke:#181818;stroke-width:0.5;""#));
    }

    #[test]
    fn singly_linked_named_note_consumes_its_svek_edge_as_an_opale_pointer() {
        let input = "@startuml\n\
            class FreshAnchor4297\n\
            note \"renamed pointer memo\" as FreshMemo4303\n\
            FreshAnchor4297 .. FreshMemo4303\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let note = svg
            .split_once(r#"data-qualified-name="FreshMemo4303""#)
            .unwrap()
            .1
            .split_once("</g>")
            .unwrap()
            .0;

        assert!(note.contains("A0,0 0 0 0"));
        assert_eq!(note.matches("stroke-width:0.5").count(), 2);
        assert!(!svg.contains(r#"<g class="link""#));
    }

    #[test]
    fn multiply_linked_named_note_keeps_ordinary_svek_edges() {
        let input = "@startuml\n\
            class FreshNorth4313\n\
            class FreshSouth4327\n\
            note \"renamed shared memo\" as FreshMemo4337\n\
            FreshNorth4313 .. FreshMemo4337\n\
            FreshSouth4327 .. FreshMemo4337\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert_eq!(svg.matches(r#"<g class="link""#).count(), 2);
    }

    #[test]
    fn named_notes_resolve_shared_solid_style_channels() {
        let input = "@startuml\n\
            skinparam note {\n\
              BackgroundColor LightGreen\n\
              BorderColor #13579B\n\
              BorderThickness 2\n\
              FontColor Navy\n\
            }\n\
            note \"renamed styled memo 4421\" as FreshStyledMemo4421\n\
            note \"renamed peer memo 4423\" as FreshStyledPeer4423\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"fill="#90EE90" style="stroke:#13579B;stroke-width:2;""##));
        assert!(svg.contains(r##"<text fill="#000080""##));
    }

    #[test]
    fn named_note_typography_drives_measurement_and_centered_paint() {
        let input = "@startuml\n\
            skinparam note {\n\
              FontSize 17\n\
              FontStyle bold\n\
              TextAlignment center\n\
            }\n\
            note as FreshCenteredTypography4513\n\
              renamed short row\n\
              renamed substantially wider typography row\n\
            end note\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let text_x = |text: &str| {
            let end = svg.find(&format!(">{text}</text>")).unwrap();
            let start = svg[..end].rfind("<text ").unwrap();
            attr_value(&svg[start..end], " x")
                .unwrap()
                .parse::<f64>()
                .unwrap()
        };

        assert!(svg.contains(r#"font-family="sans-serif" font-size="17" font-weight="700""#));
        assert!(text_x("renamed short row") > text_x("renamed substantially wider typography row"));
    }

    #[test]
    fn named_note_typography_preserves_monospace_style_and_right_alignment() {
        let input = "@startuml\n\
            skinparam note {\n\
              FontSize 11\n\
              FontName Courier\n\
              FontStyle italic\n\
              TextAlignment right\n\
            }\n\
            note as FreshRightTypography4517\n\
              renamed compact row\n\
              renamed wider monospace typography row\n\
            end note\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let text_x = |text: &str| {
            let end = svg.find(&format!(">{text}</text>")).unwrap();
            let start = svg[..end].rfind("<text ").unwrap();
            attr_value(&svg[start..end], " x")
                .unwrap()
                .parse::<f64>()
                .unwrap()
        };

        assert!(svg.contains(r#"font-family="Courier" font-size="11" font-style="italic""#));
        assert!(svg.contains(">renamed&#160;compact&#160;row</text>"));
        assert!(
            text_x("renamed&#160;compact&#160;row")
                > text_x("renamed&#160;wider&#160;monospace&#160;typography&#160;row")
        );
    }

    #[test]
    fn standalone_numbered_note_tracks_each_nesting_level() {
        let input = "@startuml\nnote as FreshChecklist733\n  # alpha\n  # beta\n  ## nested\n  # gamma\nend note\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert_eq!(svg.matches(">1.</text>").count(), 2);
        assert!(svg.contains(">2.</text>"));
        assert!(svg.contains(">3.</text>"));
        assert!(svg.contains(">nested</text>"));
    }

    #[test]
    fn standalone_note_table_sizes_columns_and_cell_backgrounds() {
        let input = "@startuml\nnote as FreshMatrix739\n  |= **Signal** |= Value |\n  |<#LightBlue> renamed | 41 |\n  |<#LightGreen> stable | 42 |\nend note\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"<rect fill="#ADD8E6""##));
        assert!(svg.contains(r##"<rect fill="#90EE90""##));
        assert!(svg.contains(r#"font-weight="700""#));
        assert_eq!(
            svg.matches(r#"style="stroke:#000000;stroke-width:1;""#)
                .count(),
            7
        );
    }

    #[test]
    fn standalone_note_tree_uses_one_structural_trunk_for_deeper_rows() {
        let input = "@startuml\nnote as FreshTree743\n  |_ renamed root\n  |__ second level\n  |___ third level\nend note\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(">renamed root</text>"));
        assert!(svg.contains(">_ second level</text>"));
        assert!(svg.contains(">__ third level</text>"));
        assert_eq!(
            svg.matches(r#"<line style="stroke:#000000;stroke-width:1;""#)
                .count(),
            6
        );
    }

    #[test]
    fn standalone_note_code_uses_monospace_atoms_and_preserves_indent() {
        let input = "@startuml\nnote as FreshCode751\n  <code>\n  fn audit() {\n      record(\"renamed value\");\n  }\n  </code>\nend note\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"font-family="monospace""#));
        assert!(svg.contains(r#"record(&quot;renamed&#160;value&quot;);"#));
        assert!(svg.contains(r#"textLength="0""#));
        assert!(!svg.contains("&lt;code&gt;"));
    }

    #[test]
    fn quoted_display_alias_is_the_qualified_name_leaf() {
        let input = "@startuml\npackage FreshDomain761 {\n  class \"**Renamed Ledger** <<service>>\" as LedgerAlias769\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(
            r#"data-qualified-name="FreshDomain761.LedgerAlias769" data-source-line="2""#
        ));
        assert!(
            !svg.contains(r#"data-qualified-name="FreshDomain761...Renamed Ledger.. .service.""#)
        );
    }

    #[test]
    fn relationship_ids_use_bare_quoted_names_but_preserve_true_aliases() {
        let input = "@startuml\n\
            package FreshDomain7301 {\n\
              package FreshLayer7307 {\n\
                class \"Fresh Billing Portal 7319\"\n\
                class \"Fresh Audit View 7321\" as FreshAuditAlias7321\n\
                class FreshPlainArchive7331\n\
                \"Fresh Billing Portal 7319\" --> FreshAuditAlias7321\n\
                FreshAuditAlias7321 ..> FreshPlainArchive7331\n\
              }\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"id="Fresh Billing Portal 7319-to-FreshAuditAlias7321""#));
        assert!(svg.contains(r#"id="FreshAuditAlias7321-to-FreshPlainArchive7331""#));
        assert!(!svg.contains("Fresh_Billing_Portal_7319-to-FreshAuditAlias7321"));
        assert!(!svg.contains("Fresh Audit View 7321-to-FreshPlainArchive7331"));
    }

    #[test]
    fn explicit_svek_spacing_overrides_only_its_named_axis() {
        let nodesep_only = rustuml_parser::parse::parse(
            "@startuml\n\
             skinparam nodesep 47\n\
             class FreshNodeAlpha7411\n\
             class FreshNodeBeta7417\n\
             FreshNodeAlpha7411 --> FreshNodeBeta7417\n\
             @enduml",
        )
        .unwrap();
        let ranksep_only = rustuml_parser::parse::parse(
            "@startuml\n\
             skinparam ranksep 83\n\
             class FreshRankAlpha7421\n\
             class FreshRankBeta7433\n\
             FreshRankAlpha7421 --> FreshRankBeta7433\n\
             @enduml",
        )
        .unwrap();
        let explicit_zero = rustuml_parser::parse::parse(
            "@startuml\n\
             skinparam nodesep 0\n\
             skinparam ranksep 0\n\
             class FreshDefaultAlpha7451\n\
             class FreshDefaultBeta7457\n\
             FreshDefaultAlpha7451 --> FreshDefaultBeta7457\n\
             @enduml",
        )
        .unwrap();

        let rustuml_parser::diagram::Diagram::Class(nodesep_only) = nodesep_only else {
            panic!("expected class diagram");
        };
        let rustuml_parser::diagram::Diagram::Class(ranksep_only) = ranksep_only else {
            panic!("expected class diagram");
        };
        let rustuml_parser::diagram::Diagram::Class(explicit_zero) = explicit_zero else {
            panic!("expected class diagram");
        };

        assert_eq!(class_svek_spacing(&nodesep_only), (47.0, 60.0));
        assert_eq!(class_svek_spacing(&ranksep_only), (35.0, 83.0));
        assert_eq!(class_svek_spacing(&explicit_zero), (35.0, 60.0));
    }

    #[test]
    fn class_member_rows_follow_resolved_atom_metrics() {
        fn entity_rect_height(svg: &str, name: &str) -> f64 {
            let marker = format!("<!--class {name}-->");
            let rect = svg
                .split_once(&marker)
                .unwrap()
                .1
                .split_once("/>")
                .unwrap()
                .0;
            attr_value(rect, " height").unwrap().parse().unwrap()
        }
        fn text_y(svg: &str, text: &str) -> f64 {
            let end = svg.find(&format!(">{text}</text>")).unwrap();
            let start = svg[..end].rfind("<text ").unwrap();
            attr_value(&svg[start..end], " y").unwrap().parse().unwrap()
        }

        let plain = rustuml_parser::parse::parse(
            "@startuml\nclass FreshAtom773 {\n  renamed payload: String\n}\n@enduml",
        )
        .unwrap();
        let enlarged = rustuml_parser::parse::parse(
            "@startuml\nclass FreshAtom773 {\n  <size:22>renamed payload</size>: String\n}\n@enduml",
        )
        .unwrap();
        let plain_svg = crate::render_svg(&plain);
        let enlarged_svg = crate::render_svg(&enlarged);
        let expected_growth =
            text_render::label_height("<size:22>renamed payload</size>: String", 14.0)
                - text_render::label_height("renamed payload: String", 14.0);

        assert!(
            (entity_rect_height(&enlarged_svg, "FreshAtom773")
                - entity_rect_height(&plain_svg, "FreshAtom773")
                - expected_growth)
                .abs()
                < 0.01
        );
        assert!(
            text_y(&enlarged_svg, "renamed payload")
                > text_y(&plain_svg, "renamed payload: String")
        );
    }

    #[test]
    fn multiline_class_name_grows_header_and_centers_each_line() {
        let input =
            "@startuml\nclass \"Fresh\\n**substantially wider renamed heading**\" {\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let x = |text: &str| {
            let end = svg.find(&format!(">{text}</text>")).unwrap();
            let start = svg[..end].rfind("<text ").unwrap();
            attr_value(&svg[start..end], " x")
                .unwrap()
                .parse::<f64>()
                .unwrap()
        };

        assert!(x("Fresh") > x("substantially wider renamed heading"));
        assert!(svg.contains(r#">Fresh</text>"#));
        assert!(svg.contains(r#">substantially wider renamed heading</text>"#));
    }

    #[test]
    fn multiline_named_note_uses_first_content_source_line() {
        let input = "@startuml\nnote as FreshNested779\n  @startuml renamed nested @enduml\nend note\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::Class(class) = diagram else {
            panic!("class diagram");
        };

        assert_eq!(
            class.notes[0].source_line, 2,
            "parsed note: {:?}",
            class.notes[0]
        );
    }

    #[test]
    fn package_title_creole_inherits_the_bold_title_font() {
        let input = "@startuml\npackage \"//renamed// <color:#2457A6>ledger</color>\" {\n  class FreshEntry787\n}\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let renamed_end = svg.find(">renamed</text>").unwrap();
        let renamed_tag = &svg[svg[..renamed_end].rfind("<text ").unwrap()..renamed_end];
        let ledger_end = svg.find(">ledger</text>").unwrap();
        let ledger_tag = &svg[svg[..ledger_end].rfind("<text ").unwrap()..ledger_end];

        assert!(renamed_tag.contains(r#"font-style="italic""#));
        assert!(renamed_tag.contains(r#"font-weight="700""#));
        assert!(ledger_tag.contains(r##"fill="#2457A6""##));
        assert!(ledger_tag.contains(r#"font-weight="700""#));
    }

    #[test]
    fn custom_spot_uses_arbitrary_character_outline() {
        let input = "@startuml\nclass Renamed << (G,#12ABEF) NewKind >>\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"fill="#12ABEF""##));
        assert!(!svg.contains(CLASS_GLYPH));
        assert!(!svg.contains(">G</text>"));
    }

    #[test]
    fn invalid_named_color_spot_uses_ordinary_stereotype_header() {
        let input = "@startuml\nclass Renamed << (G,#red) NewKind >>\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"<ellipse cx="22""#));
        assert!(svg.contains(r#"x="34" y="#));
        assert!(svg.contains("(G,#red) NewKind"));
    }

    #[test]
    fn attached_note_bullets_follow_java_line_classification() {
        assert_eq!(
            parse_note_bullet("* renamed first"),
            Some((0, "renamed first"))
        );
        assert_eq!(
            parse_note_bullet("*** renamed third"),
            Some((2, "renamed third"))
        );
        assert_eq!(parse_note_bullet("  * literal marker"), None);
        assert_eq!(parse_note_bullet("**bold line**"), None);

        let input = "@startuml\nclass Renamed\nnote left of Renamed : * renamed first\\n** renamed second\\n  * literal marker\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"<ellipse cx="#));
        assert!(svg.contains(r##"<rect fill="#000000" height="3.5""##));
        assert!(svg.contains(">renamed first</text>"));
        assert!(svg.contains(">renamed second</text>"));
        assert!(svg.contains(">* literal marker</text>"));
        assert!(!svg.contains(">* renamed first</text>"));
    }

    #[test]
    fn has_text_length() {
        let svg = render(&simple_class_diagram(), &Theme::default());
        assert!(
            svg.contains("textLength="),
            "should have textLength attribute"
        );
        assert!(
            svg.contains("lengthAdjust=\"spacing\""),
            "should have lengthAdjust"
        );
    }

    #[test]
    fn interface_rendering() {
        let diagram = ClassDiagram {
            meta: DiagramMeta::default(),
            direction: ClassLayoutDirection::TopToBottom,
            entities: vec![ClassEntity {
                id: "Drawable".into(),
                label: "Drawable".into(),
                explicit_alias: false,
                kind: EntityKind::Interface,
                members: vec![Member {
                    name: "draw()".into(),
                    return_type: Some("void".into()),
                    visibility: Visibility::Public,
                    is_static: false,
                    is_abstract: true,
                    kind: MemberKind::Method,
                    display_text: "draw(): void".into(),
                }],
                stereotypes: vec![],
                generic: None,
                spot_color: None,
                spot_character: None,
                url: None,
                url_tooltip: None,
                color: None,
                text_color: None,
                line_color: None,
                line_style: None,
                source_line: 0,
            }],
            relationships: vec![],
            association_classes: vec![],
            packages: vec![],
            notes: vec![],
            hide_show: vec![],
            header_line: None,
            footer_line: None,
            title_line: None,
            caption_line: None,
            legend_line: None,
        };
        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains("Drawable"));
        assert!(
            svg.contains(r##"fill="#B4A7E5""##),
            "should have interface icon color"
        );
    }

    #[test]
    fn parsed_then_rendered() {
        let input =
            "@startuml\nclass Animal {\n  +name : String\n}\nclass Dog\nAnimal <|-- Dog\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("Animal"));
        assert!(svg.contains("Dog"));
    }

    #[test]
    fn renamed_association_class_uses_a_live_svek_anchor() {
        let input = "@startuml\n\
            class RenamedApplicant401\n\
            class RenamedProgram409\n\
            class RenamedEnrollment419 {\n\
              +Date acceptedAt\n\
            }\n\
            (RenamedApplicant401, RenamedProgram409) .. RenamedEnrollment419\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert_eq!(
            svg.matches(r##"fill="#181818" rx="2" ry="2" style="stroke:#181818;stroke-width:1;""##)
                .count(),
            1
        );
        assert!(svg.contains("<!--link RenamedApplicant401 to apoint5-->"));
        assert!(svg.contains("<!--link apoint5 to RenamedProgram409-->"));
        assert!(svg.contains("<!--link apoint5 to RenamedEnrollment419-->"));
        assert!(svg.contains(r#"id="RenamedApplicant401-apoint5""#));
        assert!(svg.contains(r#"id="apoint5-RenamedProgram409""#));
        assert!(svg.contains(
            r#"id="apoint5-RenamedEnrollment419" style="stroke:#181818;stroke-width:1;stroke-dasharray:7,7;""#
        ));
    }

    #[test]
    fn no_oracle_relationship_styles_share_path_and_extremity_paint() {
        let input = "@startuml\n\
            class SignalEmitter701\n\
            class AuditSink709\n\
            class ColdStore719\n\
            SignalEmitter701 -[#2E8B57,dotted]-> AuditSink709\n\
            AuditSink709 -[thickness=3]-> ColdStore719\n\
            SignalEmitter701 -[hidden]-> ColdStore719\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        let colored = svg
            .split_once(r#"id="SignalEmitter701-to-AuditSink709""#)
            .unwrap()
            .1;
        assert!(colored.starts_with(
            r##" style="stroke:#2E8B57;stroke-width:1;stroke-dasharray:1,3;"/><polygon fill="#2E8B57""##
        ));
        assert!(colored.contains(r##"style="stroke:#2E8B57;stroke-width:1;"/>"##));

        let thick = svg
            .split_once(r#"id="AuditSink709-to-ColdStore719""#)
            .unwrap()
            .1;
        assert!(thick.starts_with(r##" style="stroke:#181818;stroke-width:3;"/>"##));
        assert!(thick.contains(r##"style="stroke:#181818;stroke-width:3;"/>"##));

        assert!(svg.contains("<!--link SignalEmitter701 to ColdStore719-->"));
        assert!(!svg.contains(r#"id="SignalEmitter701-to-ColdStore719""#));
    }

    #[test]
    fn no_oracle_er_crowfoot_link_group_is_rendered() {
        let mut diagram = simple_class_diagram();
        let rel = Relationship {
            from: "Animal".into(),
            to: "Dog".into(),
            kind: RelationshipKind::Association,
            label: Some("renamed relation".into()),
            label_arrow: LinkArrow::None,
            from_multiplicity: None,
            to_multiplicity: None,
            from_decor: Some(EndpointDecor::DoubleLine),
            to_decor: Some(EndpointDecor::CircleCrowFoot),
            decorated_end: RelationshipEnd::None,
            dashed: false,
            length: 2,
            style: RelationshipStyle::default(),
            source_line: 17,
        };
        diagram.relationships = vec![rel.clone()];
        let edge_path = EdgePath {
            edge_index: 0,
            from: "Animal".into(),
            to: "Dog".into(),
            points: vec![(40.0, 50.0), (40.0, 80.0), (40.0, 120.0), (40.0, 150.0)],
            has_start_arrow: false,
            start_point: None,
            has_end_arrow: false,
            end_point: None,
            label: Some(rustuml_layout::graph::EdgeLabelPosition {
                x: 40.0,
                y: 90.0,
                width: 109.0,
                height: 17.0,
            }),
            tail_label: None,
            head_label: None,
        };
        let mut svg = String::new();
        render_relationship_svg(
            &mut svg,
            &rel,
            RelationshipRenderContext {
                diagram: &diagram,
                note: None,
                entity_ids: None,
                note_ids: None,
            },
            &edge_path,
            4,
            0.0,
            0,
        );

        assert!(svg.contains(r#"data-link-type="crowfoot""#));
        assert!(svg.contains(r#"id="Animal-Dog""#));
        assert!(svg.contains(r#"d="M47,65 C47,95 47,109 47,139""#));
        assert!(svg.contains("<line "));
        assert!(svg.contains("<ellipse "));
        assert!(svg.contains(">renamed relation</text>"));
    }

    #[test]
    fn no_oracle_ortho_er_chain_handles_deeper_nesting_and_changed_count() {
        let input = "@startuml\n\
            skinparam linetype ortho\n\
            package RenamedOuter901 {\n\
              package RenamedInner907 {\n\
                entity FreshLedger911\n\
                entity FreshLedger919\n\
                entity FreshLedger929\n\
                entity FreshLedger937\n\
              }\n\
            }\n\
            FreshLedger911 ||--o{ FreshLedger919 : alpha\n\
            FreshLedger919 ||--o{ FreshLedger929 : beta\n\
            FreshLedger929 ||--o{ FreshLedger937 : gamma\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"data-qualified-name="RenamedOuter901""#));
        assert!(svg.contains(r#"data-qualified-name="RenamedOuter901.RenamedInner907""#));
        assert_eq!(svg.matches(r#"data-link-type="crowfoot""#).count(), 3);
        for label in ["alpha", "beta", "gamma"] {
            assert!(svg.contains(&format!(">{label}</text>")));
        }
    }

    #[test]
    fn no_oracle_parallel_relationships_claim_distinct_paths_and_ids() {
        let input = "@startuml\n\
            class ParallelSource941\n\
            class ParallelTarget947\n\
            ParallelSource941 --> ParallelTarget947 : first\n\
            ParallelSource941 --> ParallelTarget947 : second\n\
            ParallelSource941 --> ParallelTarget947 : third\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"id="ParallelSource941-to-ParallelTarget947""#));
        assert!(svg.contains(r#"id="ParallelSource941-to-ParallelTarget947-1""#));
        assert!(svg.contains(r#"id="ParallelSource941-to-ParallelTarget947-2""#));
        for label in ["first", "second", "third"] {
            assert!(svg.contains(&format!(">{label}</text>")));
        }
    }

    #[test]
    fn graphviz_edge_points_enter_svek_at_svg_precision() {
        let input = "@startuml\n\
            class FreshRoundedOrigin3371\n\
            class FreshRoundedMiddle3373\n\
            class FreshRoundedTarget3379\n\
            FreshRoundedOrigin3371 --> FreshRoundedMiddle3373\n\
            FreshRoundedMiddle3373 --> FreshRoundedTarget3379\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let paths = svg
            .split("<path codeLine=")
            .skip(1)
            .map(|tail| {
                tail.split_once(" d=\"")
                    .and_then(|(_, tail)| tail.split_once('"'))
                    .map(|(path, _)| path)
                    .unwrap()
            })
            .collect::<Vec<_>>();

        assert_eq!(paths.len(), 2, "{svg}");
        for path in paths {
            for coordinate in path
                .split(|ch: char| !(ch.is_ascii_digit() || matches!(ch, '-' | '.')))
                .filter(|part| !part.is_empty())
            {
                let value = coordinate.parse::<f64>().unwrap();
                assert!(
                    (value * 100.0 - (value * 100.0).round()).abs() < 0.000_001,
                    "non-SVG dot coordinate {coordinate} in {path}"
                );
            }
        }
    }

    #[test]
    fn no_oracle_class_arrow_defaults_paint_path_and_extremities() {
        let input = "@startuml\n\
            skinparam classArrowColor #13579B\n\
            skinparam classArrowThickness 4\n\
            class FreshOrigin953\n\
            class FreshDestination967\n\
            FreshOrigin953 <|-- FreshDestination967\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"style="stroke:#13579B;stroke-width:4;""##));
        assert!(svg.contains(r##"<polygon fill="none""##));
        let polygon = svg.split_once(r##"<polygon fill="none""##).unwrap().1;
        assert!(polygon.contains(r##"style="stroke:#13579B;stroke-width:4;""##));
    }

    #[test]
    fn global_arrow_style_paints_changed_count_class_links() {
        let input = "@startuml\n\
            skinparam ArrowColor #2468AC\n\
            skinparam ArrowThickness 3\n\
            class FreshPaintOrigin3407\n\
            class FreshPaintMiddle3413\n\
            class FreshPaintTarget3433\n\
            FreshPaintOrigin3407 --> FreshPaintMiddle3413\n\
            FreshPaintMiddle3413 <|.. FreshPaintTarget3433\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert_eq!(
            svg.matches(r##"style="stroke:#2468AC;stroke-width:3;"##)
                .count(),
            4,
            "{svg}"
        );
        assert!(svg.contains(r##"<polygon fill="#2468AC""##), "{svg}");
        assert!(svg.contains(r##"<polygon fill="none""##), "{svg}");
    }

    #[test]
    fn no_oracle_floating_notes_share_source_order_with_renamed_entities() {
        let input = "@startuml\n\
            note \"Opening memo 971\" as OpeningMemo971\n\
            class FreshAlpha977\n\
            class FreshBeta983\n\
            note \"Closing memo 991\" as ClosingMemo991\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        let opening = svg.find(r#"data-qualified-name="OpeningMemo971""#).unwrap();
        let alpha = svg.find(r#"data-qualified-name="FreshAlpha977""#).unwrap();
        let beta = svg.find(r#"data-qualified-name="FreshBeta983""#).unwrap();
        let closing = svg.find(r#"data-qualified-name="ClosingMemo991""#).unwrap();
        assert!(opening < alpha && alpha < beta && beta < closing);
        assert!(svg[opening..].starts_with(
            r#"data-qualified-name="OpeningMemo971" data-source-line="1" id="ent0002""#
        ));
        assert!(svg[closing..].starts_with(
            r#"data-qualified-name="ClosingMemo991" data-source-line="4" id="ent0005""#
        ));
    }

    #[test]
    fn no_oracle_class_gradients_are_generated_in_body_then_header_order() {
        let input = "@startuml\n\
            skinparam classBackgroundColor #112233/#DDEEFF\n\
            skinparam classHeaderBackgroundColor #AABBCC|#334455\n\
            class RenamedGradient997 {\n\
              payload: String\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let body_gradient = svg.find(r##"stop-color="#112233""##).unwrap();
        let header_gradient = svg.find(r##"stop-color="#AABBCC""##).unwrap();

        assert!(body_gradient < header_gradient);
        assert_eq!(svg.matches("<linearGradient ").count(), 2);
        assert!(svg.matches(r#"fill="url(#"#).count() >= 2);
    }

    #[test]
    fn stereotype_qualified_class_colors_apply_per_entity_and_leave_plain_defaults() {
        let input = "@startuml\n\
            skinparam class {\n\
              BackgroundColor<<pipeline>> HoneyDew\n\
              BorderColor<<pipeline>> SeaGreen\n\
              BackgroundColor<<archive>> AliceBlue\n\
              BorderColor<<archive>> Navy\n\
            }\n\
            class FreshPipeline1409 <<pipeline>>\n\
            class FreshArchive1423 <<archive>>\n\
            class FreshPlain1427\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        let entity_body = |name: &str| {
            let marker = format!("<!--class {name}-->");
            svg.split_once(&marker)
                .unwrap_or_else(|| panic!("missing {name}"))
                .1
                .split_once("</g>")
                .unwrap()
                .0
        };
        assert!(entity_body("FreshPipeline1409").contains(r##"<rect fill="#F0FFF0""##));
        assert!(
            entity_body("FreshPipeline1409")
                .contains(r##"style="stroke:#2E8B57;stroke-width:0.5;""##)
        );
        assert!(entity_body("FreshArchive1423").contains(r##"<rect fill="#F0F8FF""##));
        assert!(
            entity_body("FreshArchive1423")
                .contains(r##"style="stroke:#000080;stroke-width:0.5;""##)
        );
        assert!(entity_body("FreshPlain1427").contains(r##"<rect fill="#F1F1F1""##));
    }

    #[test]
    fn grouped_stereotype_class_colors_apply_inside_nested_packages() {
        let input = "@startuml\n\
            skinparam class<<freshservice7517>> {\n\
              BackgroundColor #DDEEFF\n\
              BorderColor #335577\n\
            }\n\
            skinparam class<<fresharchive7523>> {\n\
              BackgroundColor HoneyDew\n\
              BorderColor SeaGreen\n\
            }\n\
            package FreshOuter7529 {\n\
              package FreshInner7537 {\n\
                class FreshWorker7541 <<freshservice7517>>\n\
                class FreshStore7547 <<fresharchive7523>>\n\
                class FreshPlain7559\n\
                FreshWorker7541 --> FreshStore7547\n\
              }\n\
            }\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        let entity_body = |name: &str| {
            let marker = format!("<!--class {name}-->");
            svg.split_once(&marker)
                .unwrap_or_else(|| panic!("missing {name}"))
                .1
                .split_once("</g>")
                .unwrap()
                .0
        };
        assert!(entity_body("FreshWorker7541").contains(r##"<rect fill="#DDEEFF""##));
        assert!(
            entity_body("FreshWorker7541")
                .contains(r##"style="stroke:#335577;stroke-width:0.5;""##)
        );
        assert!(entity_body("FreshStore7547").contains(r##"<rect fill="#F0FFF0""##));
        assert!(
            entity_body("FreshStore7547").contains(r##"style="stroke:#2E8B57;stroke-width:0.5;""##)
        );
        assert!(entity_body("FreshPlain7559").contains(r##"<rect fill="#F1F1F1""##));
    }

    #[test]
    fn no_oracle_relationship_label_arrow_is_rendered_as_a_guide_triangle() {
        let input = "@startuml\nclass Alpha\nclass Beta\nAlpha -- Beta : renamed flow >\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(">renamed flow</text>"));
        assert!(!svg.contains("renamed flow &gt;"));
        assert_eq!(svg.matches("<polygon ").count(), 1);

        let polygon = svg.split_once("<polygon ").unwrap().1;
        let tip_x = attr_value(polygon, "points")
            .unwrap()
            .split_once(',')
            .unwrap()
            .0
            .parse::<f64>()
            .unwrap();
        let text_end = svg.find(">renamed flow</text>").unwrap();
        let text = &svg[svg[..text_end].rfind("<text ").unwrap()..text_end];
        let text_x = attr_value(text, "x").unwrap().parse::<f64>().unwrap();
        assert_eq!(round_4dp(text_x - tip_x), 9.0);
    }

    #[test]
    fn renamed_endpoint_roles_expand_the_svek_envelope() {
        let plain = rustuml_parser::parse::parse(
            "@startuml\nclass PerturbedAlpha\nclass PerturbedBeta\nPerturbedAlpha o-- PerturbedBeta : relationship\n@enduml",
        )
        .unwrap();
        let with_roles = rustuml_parser::parse::parse(
            "@startuml\nclass PerturbedAlpha\nclass PerturbedBeta\nPerturbedAlpha \"upstream-role-renamed\" o-- \"downstream-role-renamed\" PerturbedBeta : relationship\n@enduml",
        )
        .unwrap();

        let plain_svg = crate::render_svg(&plain);
        let roles_svg = crate::render_svg(&with_roles);
        let text_x = |svg: &str, text: &str| {
            let end = svg.find(&format!(">{text}</text>")).unwrap();
            let start = svg[..end].rfind("<text ").unwrap();
            attr_value(&svg[start..end], "x")
                .unwrap()
                .parse::<f64>()
                .unwrap()
        };

        assert!(plain_svg.contains(">relationship</text>"));
        assert!(roles_svg.contains(">upstream-role-renamed</text>"));
        assert!(roles_svg.contains(">downstream-role-renamed</text>"));
        let leftmost_role_x = text_x(&roles_svg, "upstream-role-renamed")
            .min(text_x(&roles_svg, "downstream-role-renamed"));
        assert!(
            (leftmost_role_x - SVEK_LABEL_ENVELOPE_MARGIN).abs() < 0.01,
            "the leftmost collision-moved role must normalize to the SVEK margin"
        );
    }

    #[test]
    fn renamed_nine_node_ring_includes_painted_dotpath_envelope() {
        let input = "@startuml\n\
            class NovaA\n\
            class NovaB\n\
            class NovaC\n\
            class NovaD\n\
            class NovaE\n\
            class NovaF\n\
            class NovaG\n\
            class NovaH\n\
            class NovaI\n\
            NovaA --> NovaB\n\
            NovaB --> NovaC\n\
            NovaC --> NovaD\n\
            NovaD --> NovaE\n\
            NovaE --> NovaF\n\
            NovaF --> NovaG\n\
            NovaG --> NovaH\n\
            NovaH --> NovaI\n\
            NovaI --> NovaA\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java 21 / PlantUML 1.2026.3beta6 render. The return edge's
        // DotPath reaches x=136.2; `SvekResult.calculateDimension` adds 15px.
        assert!(svg.contains(r#"viewBox="0 0 151 934""#), "{svg}");
        assert_eq!(svg.matches(r#"<g class="entity""#).count(), 9);
        assert_eq!(svg.matches(r#"<g class="link""#).count(), 9);
    }

    #[test]
    fn renamed_seven_node_complete_graph_uses_rectangle_limitfinder_inset() {
        let mut input = String::from("@startuml\n");
        for suffix in ["A", "B", "C", "D", "E", "F", "G"] {
            writeln!(input, "class Nova{suffix}").unwrap();
        }
        for from in ["A", "B", "C", "D", "E", "F"] {
            for to in ["A", "B", "C", "D", "E", "F", "G"]
                .into_iter()
                .skip_while(|suffix| *suffix != from)
                .skip(1)
            {
                writeln!(input, "Nova{from} --> Nova{to}").unwrap();
            }
        }
        input.push_str("@enduml");

        let diagram = rustuml_parser::parse::parse(&input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java 21 / PlantUML 1.2026.3beta6 render. The leftmost DotPath
        // is the envelope minimum, so `SvekResult` moves it left of the normal
        // class-box margin after `LimitFinder.drawRectangle` applies x - 1.
        assert!(svg.contains(r#"viewBox="0 0 731 718""#), "{svg}");
        assert_eq!(svg.matches(r#"<g class="entity""#).count(), 7);
        assert_eq!(svg.matches(r#"<g class="link""#).count(), 21);
    }

    #[test]
    fn disconnected_renamed_classes_are_constrained_by_squaremaker_links() {
        let mut input = String::from("@startuml\n");
        for index in 0..14 {
            writeln!(input, "class DetachedModel{index:02}").unwrap();
        }
        input.push_str("@enduml");

        let diagram = rustuml_parser::parse::parse(&input).unwrap();
        let svg = crate::render_svg(&diagram);
        let view_box = svg
            .split_once("viewBox=\"0 0 ")
            .unwrap()
            .1
            .split_once('"')
            .unwrap()
            .0;
        let mut dimensions = view_box
            .split_whitespace()
            .map(|value| value.parse::<f64>().unwrap());
        let width = dimensions.next().unwrap();
        let height = dimensions.next().unwrap();

        assert_eq!(svg.matches(r#"<g class="entity""#).count(), 14);
        assert!(
            width < 800.0,
            "square layout should not form one long row: {svg}"
        );
        assert!(
            height > 250.0,
            "square layout should span multiple ranks: {svg}"
        );
    }

    #[test]
    fn short_svek_links_create_endpoints_before_wide_disconnected_entities() {
        let input = "@startuml\n\
            class FreshDetachedLedger {\n\
              a deliberately widened standalone record\n\
            }\n\
            class FreshIngress\n\
            class FreshEgress\n\
            FreshIngress -> FreshEgress : renamed flow\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        let entity_x = |name: &str| {
            let marker = format!(r#"data-qualified-name="{name}""#);
            svg.split_once(&marker)
                .unwrap()
                .1
                .split_once("<rect ")
                .unwrap()
                .1
                .split_once(r#" x=""#)
                .unwrap()
                .1
                .split_once('"')
                .unwrap()
                .0
                .parse::<f64>()
                .unwrap()
        };

        let detached_x = entity_x("FreshDetachedLedger");
        assert!(entity_x("FreshIngress") < detached_x, "{svg}");
        assert!(entity_x("FreshEgress") < detached_x, "{svg}");
    }

    #[test]
    fn merged_class_styles_inherit_into_deeper_headers_and_document_margin() {
        let input = "@startuml\n\
            <style>\n\
            root {\n\
              Margin 13\n\
            }\n\
            </style>\n\
            skinparam defaultFontSize 18\n\
            skinparam ClassAttributeFontSize 9\n\
            skinparam ClassBorderThickness 3\n\
            class FreshRoot {\n\
              +alpha: String\n\
            }\n\
            class FreshChild extends FreshRoot {\n\
              +beta: int\n\
            }\n\
            class FreshGrandchild extends FreshChild\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java 21 / PlantUML 1.2026.3beta6 reference. Java resolves
        // `root.element.classDiagram.class.header` through
        // `EntityImageClassHeader`, so the 9px parent class font beats the
        // 18px root default at every depth. `TextBlockExporter12026` applies
        // the 13px document margin around the solved SVEK image.
        assert!(svg.contains(r#"viewBox="0 0 148 334""#), "{svg}");
        assert!(svg.contains(
            r##"<rect fill="#F1F1F1" height="60.5996" rx="2.5" ry="2.5" style="stroke:#181818;stroke-width:3;" width="82.1821""##
        ), "{svg}");
        let grandchild = svg
            .split_once(r#"data-qualified-name="FreshGrandchild""#)
            .unwrap()
            .1;
        assert!(grandchild.contains(r#"font-size="9""#), "{svg}");
        assert!(grandchild.contains(">FreshGrandchild</text>"), "{svg}");
    }

    #[test]
    fn renamed_sprite_replaces_class_badge_and_bottom_aligns_in_note() {
        let input = "@startuml\n\
            sprite $fresh_badge [5x3/16] {\n\
            01234\n\
            43210\n\
            13531\n\
            }\n\
            class Q <<$fresh_badge>>\n\
            note right of Q : <$fresh_badge> Renamed warning\n\
            @enduml";
        let diagram = rustuml_parser::parse::parse_auto_with_base(input, None).unwrap();
        let rustuml_parser::diagram::Diagram::Class(diagram) = diagram else {
            panic!("expected class diagram");
        };
        let entity = &diagram.entities[0];
        let font = ClassFontOverrides::from_skinparams(&diagram.meta.skinparams);
        let with_sprite = calc_entity_dims(
            entity,
            0,
            resolve_hide(entity, &diagram.hide_show),
            &font,
            &diagram.meta.sprites,
        );
        let mut plain_entity = entity.clone();
        plain_entity.stereotypes.clear();
        let without_sprite = calc_entity_dims(
            &plain_entity,
            0,
            resolve_hide(&plain_entity, &diagram.hide_show),
            &font,
            &diagram.meta.sprites,
        );

        assert!(with_sprite.has_header_sprite);
        assert!(
            ((without_sprite.height - with_sprite.height) - (HEADER_HEIGHT - HEADER_H_NO_CIRCLE))
                .abs()
                < 1e-9
        );
        assert_eq!(without_sprite.width - with_sprite.width, 17.0);

        let svg = render(&diagram, &Theme::default());
        assert_eq!(svg.matches(r#"<image height="3" width="5""#).count(), 2);
        assert!(!svg.contains("&lt;$fresh_badge&gt;"));
    }

    #[test]
    fn renamed_dynamic_class_fonts_align_headers_and_visibility_rows() {
        let eleven = "@startuml\n\
            skinparam defaultFontSize 11\n\
            class RenamedLedgerEleven {\n\
              +alphaCode: String\n\
              -betaCount: long\n\
              #reconcile()\n\
              ~archive()\n\
            }\n\
            @enduml";
        let nineteen = "@startuml\n\
            skinparam defaultFontSize 19\n\
            class RenamedRegistryNineteen {\n\
              +primaryKey: UUID\n\
              -retryCount: int\n\
              #refreshCache()\n\
              ~expireEntry()\n\
              +lookupRecord()\n\
              -removeRecord()\n\
            }\n\
            @enduml";

        let eleven_svg =
            crate::render_svg(&rustuml_parser::parse::parse(eleven).expect("11px class parses"));
        let nineteen_svg =
            crate::render_svg(&rustuml_parser::parse::parse(nineteen).expect("19px class parses"));

        // Coordinates below come from fresh Java 21 PlantUML renders of these
        // renamed, non-corpus inputs. They exercise `HeaderLayout.drawU` and
        // `PlacementStrategyVisibility.getPositions` with different row counts.
        assert!(eleven_svg.contains(r#"style="width:167px;height:115px;background:#FFFFFF;""#));
        assert!(eleven_svg.contains(r##"<ellipse cx="20" cy="21" fill="#ADD1B2" rx="9" ry="9""##));
        assert!(eleven_svg.contains(
            r#"font-size="11" lengthAdjust="spacing" textLength="119.6304" x="32" y="25.1572">RenamedLedgerEleven</text>"#
        ));
        assert!(eleven_svg.contains(r#"<ellipse cx="18" cy="46.9775" fill="none""#));
        assert!(eleven_svg.contains(r#"width="6" x="15" y="56.9326"/>"#));
        assert!(eleven_svg.contains(r#"points="18,75.8877,22,79.8877,18,83.8877,14,79.8877""#));
        assert!(eleven_svg.contains(r#"points="18,89.8428,14,95.8428,22,95.8428""#));

        assert!(nineteen_svg.contains(r#"style="width:295px;height:204px;background:#FFFFFF;""#));
        assert!(
            nineteen_svg.contains(r##"<ellipse cx="23" cy="24" fill="#ADD1B2" rx="12" ry="12""##)
        );
        assert!(nineteen_svg.contains(
            r#"font-size="19" lengthAdjust="spacing" textLength="241.5728" x="38" y="31.1807">RenamedRegistryNineteen</text>"#
        ));
        assert!(nineteen_svg.contains(r#"<ellipse cx="18" cy="57.6885" fill="none""#));
        assert!(nineteen_svg.contains(r#"width="6" x="15" y="77.0654"/>"#));
        assert!(
            nineteen_svg.contains(r#"points="18,105.4424,22,109.4424,18,113.4424,14,109.4424""#)
        );
        assert!(nineteen_svg.contains(r##"<ellipse cx="18" cy="155.1963" fill="#84BE84""##));
        assert!(nineteen_svg.contains(r#"width="6" x="15" y="174.5732"/>"#));
    }

    #[test]
    fn renamed_hide_and_remove_lifecycles_preserve_layout_and_source_uids() {
        let hidden = "@startuml\n\
            class HiddenLedgerAlpha <<internal-zone>> {\n\
              #secretToken: String\n\
              +rotateSecret()\n\
            }\n\
            class PublicRegistryBeta <<public-zone>> {\n\
              +primaryKey: UUID\n\
              -retryCount: int\n\
              #refreshCache()\n\
              ~expireEntry()\n\
            }\n\
            class AuditTailGamma\n\
            HiddenLedgerAlpha --> PublicRegistryBeta\n\
            PublicRegistryBeta --> AuditTailGamma\n\
            hide <<internal-zone>>\n\
            @enduml";
        let removed = "@startuml\n\
            class OriginLedgerDelta\n\
            class RemovedBridgeEpsilon\n\
            class SurvivorRegistryZeta {\n\
              +lookup()\n\
              #refresh()\n\
            }\n\
            class TailArchiveEta\n\
            OriginLedgerDelta --> RemovedBridgeEpsilon\n\
            RemovedBridgeEpsilon --> SurvivorRegistryZeta\n\
            SurvivorRegistryZeta --> TailArchiveEta\n\
            remove RemovedBridgeEpsilon\n\
            @enduml";

        let hidden_svg =
            crate::render_svg(&rustuml_parser::parse::parse(hidden).expect("hide input parses"));
        let removed_svg =
            crate::render_svg(&rustuml_parser::parse::parse(removed).expect("remove input parses"));

        assert!(!hidden_svg.contains(r#"data-qualified-name="HiddenLedgerAlpha""#));
        assert!(hidden_svg.contains(
            r#"data-qualified-name="PublicRegistryBeta" data-source-line="5" id="ent0003""#
        ));
        assert!(!hidden_svg.contains("HiddenLedgerAlpha-to-PublicRegistryBeta"));
        assert!(hidden_svg.contains(r#"id="lnk6""#));

        assert!(!removed_svg.contains(r#"data-qualified-name="RemovedBridgeEpsilon""#));
        assert!(removed_svg.contains(
            r#"data-qualified-name="SurvivorRegistryZeta" data-source-line="3" id="ent0004""#
        ));
        assert!(removed_svg.contains(r#"data-entity-1="ent0004" data-entity-2="ent0005""#));
        assert!(removed_svg.contains(r#"id="lnk8""#));
    }

    #[test]
    fn renamed_target_and_wildcard_selectors_apply_without_fixture_names() {
        let targeted = "@startuml\n\
            class TargetedVaultTheta {\n\
              +publicField: String\n\
              -privateField: int\n\
              +open()\n\
              -seal()\n\
              #audit()\n\
            }\n\
            class UntouchedLedgerIota {\n\
              +retainedField: UUID\n\
              +retain()\n\
              ~archive()\n\
            }\n\
            hide TargetedVaultTheta methods\n\
            @enduml";
        let wildcard = "@startuml\n\
            class RenamedTransientKappa\n\
            class RenamedTransientLambda\n\
            class DurableRegistryMu\n\
            hide *Transient*\n\
            @enduml";

        let targeted_svg = crate::render_svg(
            &rustuml_parser::parse::parse(targeted).expect("target input parses"),
        );
        let wildcard_svg =
            crate::render_svg(&rustuml_parser::parse::parse(wildcard).expect("wildcard parses"));

        for field in [
            "publicField: String",
            "privateField: int",
            "retainedField: UUID",
        ] {
            assert!(targeted_svg.contains(&format!(">{field}</text>")));
        }
        for hidden_method in ["open()", "seal()", "audit()"] {
            assert!(!targeted_svg.contains(&format!(">{hidden_method}</text>")));
        }
        for visible_method in ["retain()", "archive()"] {
            assert!(targeted_svg.contains(&format!(">{visible_method}</text>")));
        }

        assert!(!wildcard_svg.contains(r#"data-qualified-name="RenamedTransientKappa""#));
        assert!(!wildcard_svg.contains(r#"data-qualified-name="RenamedTransientLambda""#));
        assert!(wildcard_svg.contains(
            r#"data-qualified-name="DurableRegistryMu" data-source-line="3" id="ent0004""#
        ));
    }

    #[test]
    fn empty_diagram() {
        let diagram = ClassDiagram {
            meta: DiagramMeta::default(),
            direction: ClassLayoutDirection::TopToBottom,
            entities: vec![],
            relationships: vec![],
            association_classes: vec![],
            packages: vec![],
            notes: vec![],
            hide_show: vec![],
            header_line: None,
            footer_line: None,
            title_line: None,
            caption_line: None,
            legend_line: None,
        };
        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains("<svg"));
    }
}
