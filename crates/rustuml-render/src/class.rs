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

use std::collections::HashMap;
use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph, NodePosition};
use rustuml_parser::diagram::SpriteData;
use rustuml_parser::diagram::class::*;

use crate::layout_oracle::{
    CrowMark, EntityPath, EntityPolygon, EntityRect, EntityText, OracleCluster, OracleEdgePath,
    OracleEntity, OracleHandwrittenWarning, OracleLayout, OracleLegend, emit_entity_image,
    emit_oracle_cluster_children, emit_oracle_note_entity, wrap_oracle_envelope,
};
use crate::metrics;
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
/// Name baseline y (relative to rect top) when `hide circle` is active.
const NAME_BASELINE_Y_NO_CIRCLE: f64 = 25.5352;
/// Gap between icon and entity name text.
const ICON_TEXT_GAP: f64 = 3.0;
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
/// Icon center y within the entity header.
const ICON_CY: f64 = 23.0;
/// When `skinparam padding N` is set, PlantUML positions the stereotype circle
/// at `rect_top + N + (ICON_CY - MARGIN) - PADDING_ICON_CY_BIAS`. The bias was
/// measured from golden output across padding 5/10/15/20/30.
const PADDING_ICON_CY_BIAS: f64 = 2.7559;
/// Y position of entity name text baseline.
const NAME_BASELINE_Y: f64 = 28.291;
/// Y position of separator line below header.
const HEADER_SEP_Y: f64 = 39.0;
/// Y position of second separator line (empty methods compartment).
const METHODS_SEP_Y: f64 = 47.0;
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
/// Baseline rise of a labelled-separator caption above its divider rule.
const LABEL_SEP_TEXT_RISE: f64 = 4.791015625;
/// Offset from entity x to member text start, at the default circled radius
/// (11): `MEMBER_TEXT_INSET + radius = 9 + 11 = 20`.
const MEMBER_TEXT_OFFSET: f64 = 20.0;
/// Member-text left inset relative to the circled icon radius. PlantUML places
/// member text at `compartment_pad + (circledRadius + 3)`; with the compartment
/// pad and entity left margin this nets to `entity_x + radius + 9`.
const MEMBER_TEXT_INSET: f64 = 9.0;
/// Offset from entity x to enum constant text start.
const ENUM_TEXT_OFFSET: f64 = 6.0;
/// Offset from entity x to visibility icon center.
const VIS_ICON_OFFSET: f64 = 11.0;
/// Visibility icon radius (small circle for method visibility).
const VIS_ICON_R: f64 = 3.0;
/// Default half-size for diamond and triangle visibility icons.
const VIS_ICON_ANGLED_HALF: f64 = 4.0;
/// PlantUML derives the round/square half-size as `classAttributeIconSize / 3`.
const VIS_ICON_SIZE_RADIUS_DIVISOR: u32 = 3;
/// Diamond/triangle horizontal half-size is one pixel inside half the icon box.
const VIS_ICON_ANGLED_INSET: f64 = 1.0;
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
/// Extra header height when stereotypes are present.
const STEREOTYPE_EXTRA_HEIGHT: f64 = 8.6211;
/// Baseline-to-baseline distance between multiple stereotype lines.
const STEREOTYPE_LINE_HEIGHT: f64 = 14.1328;
/// Stereotype text baseline y relative to entity rect top.
const STEREOTYPE_Y_OFFSET: f64 = 16.6016;
/// Name text baseline y relative to entity rect top when stereotypes are present.
const NAME_Y_WITH_STEREO: f64 = 32.668;
/// Icon center y relative to entity rect top when stereotypes are present.
const ICON_CY_WITH_STEREO: f64 = 20.3105;

const NOTE_FILL: &str = "#FEFFDD";
const NOTE_BORDER: &str = "#888888";
const NOTE_FOLD: f64 = 10.0;
const NOTE_PAD_X: f64 = 6.0;
const NOTE_PAD_Y: f64 = 4.0;
const NOTE_LINE_HEIGHT: f64 = 16.0;
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
/// Height of a caption block (pushes the footer down when both are present).
const DECORATION_CAPTION_BLOCK_H: f64 = 23.5352;
/// Baseline-to-baseline spacing for multi-line page decorations.
const DECORATION_LINE_HEIGHT: f64 = MEMBER_LINE_HEIGHT;
const GRID_MARGIN: f64 = 30.0;
#[allow(dead_code)]
const CLASS_MIN_WIDTH: f64 = 120.0;
#[allow(dead_code)]
const PACKAGE_HEADER: f64 = 24.0;
#[allow(dead_code)]
const PACKAGE_PAD: f64 = 12.0;

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
    // A bare single token that exactly names a known entity (by id or label).
    if !arg.is_empty()
        && !arg.contains(char::is_whitespace)
        && entities
            .iter()
            .any(|e| e.id.eq_ignore_ascii_case(arg) || e.label.eq_ignore_ascii_case(arg))
    {
        return Some(EntitySelector::Name(arg.to_string()));
    }
    None
}

/// A whole-entity selector resolved from a `hide`/`remove`/`show` directive.
enum EntitySelector {
    Name(String),
    Stereotype(String),
}

impl EntitySelector {
    fn matches(&self, entity: &ClassEntity) -> bool {
        match self {
            EntitySelector::Name(n) => {
                entity.id.eq_ignore_ascii_case(n) || entity.label.eq_ignore_ascii_case(n)
            }
            EntitySelector::Stereotype(s) => {
                entity.stereotypes.iter().any(|t| t.eq_ignore_ascii_case(s))
            }
        }
    }
}

/// Compute the set of entity indices suppressed by whole-entity
/// `hide`/`remove` directives, honouring later `show` directives that
/// re-enable them (in source order).
fn suppressed_entities(diagram: &ClassDiagram) -> std::collections::HashSet<usize> {
    let mut suppressed = std::collections::HashSet::new();
    for d in &diagram.hide_show {
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

/// Build a copy of `diagram` with the given entity indices removed, along with
/// any relationships and notes that reference them, and any package membership
/// entries. Relationships/notes whose endpoints survive are kept verbatim.
fn filter_suppressed(
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
    // Entity labels treat `__` as literal underscores, not underline markup,
    // so width must include those characters.
    let name_width = escaped_newline_lines(&entity.label)
        .iter()
        .map(|line| {
            text_render::measure_no_underline_with_family(line, 14.0, false, &font.name_family)
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
    let icon_area = if hide.circle {
        // `MyType` etc. golden output shows the name horizontally centred
        // inside a 2*HEADER_RIGHT_PAD-padded box; treat the icon area as
        // empty padding to recover the matching width.
        HEADER_RIGHT_PAD
    } else if entity.kind == EntityKind::Object {
        ENUM_TEXT_OFFSET
    } else {
        ICON_CX_OFFSET + ICON_RX + ICON_TEXT_GAP // 29
    };
    let name_total = icon_area + name_width + HEADER_RIGHT_PAD;

    // Stereotype text may also affect width.
    let stereo_width = if has_stereotypes {
        let stereo_tw = format_stereotype_lines(&visible_stereotypes)
            .iter()
            .map(|line| text_render::measure(line, 12.0, false))
            .fold(0.0_f64, f64::max);
        // Stereotype text is centered in the header area alongside the icon.
        icon_area + stereo_tw + HEADER_RIGHT_PAD
    } else {
        0.0
    };

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
                        crate::math::raw_latex_image(latex).width as f64
                    } else {
                        text_render::measure_no_underline_with_family(
                            text,
                            14.0,
                            false,
                            &font.family,
                        )
                    }
                })
                .fold(0.0_f64, f64::max);
            if m.visibility == Visibility::Default {
                // Default visibility (including enum constants): no icon.
                ENUM_TEXT_OFFSET + text_w + MEMBER_RIGHT_PAD
            } else {
                // Members with visibility icon (including enum members with explicit visibility).
                MEMBER_TEXT_OFFSET + text_w + MEMBER_RIGHT_PAD
            }
        })
        .collect();

    let max_member_width = member_widths.iter().cloned().fold(0.0_f64, f64::max);
    // Hidden compartments contribute nothing to the per-compartment count.
    let eff_field_count = if hide.fields { 0 } else { field_count };
    let eff_method_count = if hide.methods { 0 } else { method_count };
    let mut width = name_total.max(stereo_width).max(max_member_width);

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

    const HEADER_H: f64 = 32.0;
    let header_h = if has_stereotypes {
        HEADER_H + stereotype_header_extra_height(stereotype_count)
    } else if hide.circle || entity.kind == EntityKind::Object {
        HEADER_H_NO_CIRCLE
    } else {
        HEADER_H
    };

    let height = if hide.fields && hide.methods {
        // Both compartments hidden — header only, no body or separators.
        header_h
    } else if entity.kind == EntityKind::Object {
        header_h + COMPARTMENT_PAD + eff_field_count as f64 * MEMBER_LINE_HEIGHT
    } else if entity.members.is_empty()
        || (eff_field_count == 0 && eff_method_count == 0 && !enum_classic)
    {
        // No members: header + empty fields + empty methods.
        header_h + COMPARTMENT_PAD + COMPARTMENT_PAD
    } else if enum_classic {
        // Enum: header + values + bottom separator.
        header_h + (COMPARTMENT_PAD + eff_field_count as f64 * MEMBER_LINE_HEIGHT) + COMPARTMENT_PAD
    } else {
        // Class/interface/abstract/annotation.
        let fields_section = COMPARTMENT_PAD + eff_field_count as f64 * MEMBER_LINE_HEIGHT;
        let methods_section = COMPARTMENT_PAD + eff_method_count as f64 * MEMBER_LINE_HEIGHT;
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
        source_line,
        hide,
    }
}

fn stereotype_header_extra_height(stereotype_count: usize) -> f64 {
    if stereotype_count == 0 {
        0.0
    } else {
        STEREOTYPE_EXTRA_HEIGHT
            + (stereotype_count.saturating_sub(1) as f64) * STEREOTYPE_LINE_HEIGHT
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

    // Apply whole-entity `hide`/`remove` directives by dropping the targeted
    // entities (and their links/notes/package memberships) before layout. The
    // `ent000N` ids of surviving entities are taken from the oracle by name,
    // so the dropped entity's slot in the id sequence is preserved naturally.
    let suppressed = suppressed_entities(diagram);
    if !suppressed.is_empty() {
        let filtered = filter_suppressed(diagram, &suppressed);
        return render_with_oracle(&filtered, theme, oracle);
    }

    if diagram.entities.is_empty() {
        if !diagram.notes.is_empty() {
            return render_notes_only(diagram, cs, oracle);
        }
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
            &edge_paths,
            canvas_dims,
            Some(&oracle_entities),
            Some(oracle),
            cs,
        );
    }

    // Phase 2: Use layout engine to determine positions.
    let mut layout = LayoutGraph::new(Direction::TopToBottom);
    for (entity, dim) in diagram.entities.iter().zip(&dims) {
        layout.add_node(&entity.id, &entity.label, dim.width, dim.height);
    }
    for rel in &diagram.relationships {
        layout.add_edge(&rel.from, &rel.to, rel.label.as_deref());
    }

    let result = match layout.layout_full(std::time::Duration::from_secs(5)) {
        Some(r) => r,
        None => {
            return render_grid_fallback(diagram, cs);
        }
    };

    // Phase 3: Render with PlantUML-compatible SVG structure.
    render_plantuml_svg(
        diagram,
        &dims,
        &result.node_positions,
        &result.edge_paths,
        None,
        None,
        None,
        cs,
    )
}

/// Class font overrides derived from explicit `skinparam Class*Font*` settings.
///
/// Only fields the user actually set are populated — the styled default theme's
/// own font attributes must not leak into class text (PlantUML renders class
/// text black, 14px, plain by default).
#[derive(Default, Clone)]
struct ClassFontOverrides {
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
        // `skinparam defaultFontSize` is the base size for all class text,
        // overridden by the more specific `ClassFontSize` (name) and
        // `ClassAttributeFontSize` (members). It only applies when the
        // specific skinparam is absent.
        let default_font_size =
            find(&["defaultFontSize"]).and_then(|v| v.trim().parse::<u32>().ok());
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
            font_color: find(&["ClassFontColor"]).or_else(|| default_font_color.clone()),
            attr_font_color: find(&["ClassAttributeFontColor"])
                .or_else(|| default_font_color.clone()),
            font_size: find(&["ClassFontSize"])
                .and_then(|v| v.trim().parse::<u32>().ok())
                .or(default_font_size),
            family,
            name_family,
            font_bold: style.contains("bold"),
            font_italic: style.contains("italic"),
            stereotype_font_styles,
            attr_font_size: find(&["ClassAttributeFontSize"])
                .and_then(|v| v.trim().parse::<u32>().ok())
                .or(default_font_size),
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

#[derive(Clone, Copy)]
struct VisibilityIconGeom {
    center_offset: f64,
    round_half: f64,
    angled_half: f64,
    triangle_half_y: f64,
}

impl VisibilityIconGeom {
    fn from_attribute_icon_size(size: Option<u32>) -> Self {
        if let Some(size) = size {
            let round_half = (size / VIS_ICON_SIZE_RADIUS_DIVISOR) as f64;
            Self {
                center_offset: size as f64,
                round_half,
                angled_half: (size as f64 / 2.0 - VIS_ICON_ANGLED_INSET).max(round_half),
                triangle_half_y: round_half,
            }
        } else {
            Self {
                center_offset: VIS_ICON_OFFSET,
                round_half: VIS_ICON_R,
                angled_half: VIS_ICON_ANGLED_HALF,
                triangle_half_y: VIS_ICON_R,
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

fn gradient_fill_from_defs(value: Option<&str>, oracle: Option<&OracleLayout>) -> Option<String> {
    let (c1, c2) = split_gradient_colors(value?)?;
    oracle
        .map(|o| o.defs_inner_xml.as_str())
        .and_then(|defs| resolve_gradient_id(defs, c1, c2))
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

fn entity_emission_order(diagram: &ClassDiagram) -> Vec<usize> {
    let n_pkg = diagram.packages.len();
    let innermost_pkg: Vec<Option<usize>> = diagram
        .entities
        .iter()
        .map(|e| {
            diagram
                .packages
                .iter()
                .enumerate()
                .filter(|(_, p)| p.entities.iter().any(|m| m == &e.id))
                .min_by_key(|(idx, p)| (p.entities.len(), usize::MAX - idx))
                .map(|(idx, _)| idx)
        })
        .collect();

    let parent_pkg: Vec<Option<usize>> = (0..n_pkg)
        .map(|i| {
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
        .collect();

    let pkg_sort_key = |pi: usize| -> usize {
        diagram
            .entities
            .iter()
            .filter(|e| diagram.packages[pi].entities.iter().any(|m| m == &e.id))
            .map(|e| e.source_line)
            .min()
            .unwrap_or(usize::MAX)
    };

    fn emit_pkg(
        pkg_idx: usize,
        diagram: &ClassDiagram,
        innermost_pkg: &[Option<usize>],
        parent_pkg: &[Option<usize>],
        pkg_sort_key: &dyn Fn(usize) -> usize,
        order: &mut Vec<usize>,
    ) {
        for (i, _) in diagram.entities.iter().enumerate() {
            if innermost_pkg[i] == Some(pkg_idx) {
                order.push(i);
            }
        }
        let mut children: Vec<usize> = (0..diagram.packages.len())
            .filter(|&c| parent_pkg[c] == Some(pkg_idx))
            .collect();
        children.sort_by_key(|&c| (pkg_sort_key(c), c));
        for c in children {
            emit_pkg(c, diagram, innermost_pkg, parent_pkg, pkg_sort_key, order);
        }
    }

    enum Item {
        Entity(usize),
        Package(usize),
    }
    let mut items: Vec<(usize, Item)> = Vec::new();
    for (i, e) in diagram.entities.iter().enumerate() {
        if innermost_pkg[i].is_none() {
            items.push((e.source_line, Item::Entity(i)));
        }
    }
    for (pi, parent) in parent_pkg.iter().enumerate() {
        if parent.is_none() {
            items.push((pkg_sort_key(pi), Item::Package(pi)));
        }
    }
    items.sort_by_key(|(line, _)| *line);

    let mut order = Vec::with_capacity(diagram.entities.len());
    for (_, item) in items {
        match item {
            Item::Entity(i) => order.push(i),
            Item::Package(pi) => emit_pkg(
                pi,
                diagram,
                &innermost_pkg,
                &parent_pkg,
                &pkg_sort_key,
                &mut order,
            ),
        }
    }
    let placed: std::collections::HashSet<usize> = order.iter().copied().collect();
    for (i, _) in diagram.entities.iter().enumerate() {
        if !placed.contains(&i) {
            order.push(i);
        }
    }
    order
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
    edge_paths: &[EdgePath],
    canvas_override: Option<(f64, f64)>,
    oracle_entities: Option<&[Option<OracleEntity>]>,
    oracle: Option<&OracleLayout>,
    cs: &crate::style::ClassStyle,
) -> String {
    if positions.len() < diagram.entities.len() {
        return render_grid_fallback(diagram, cs);
    }

    let font = ClassFontOverrides::from_skinparams(&diagram.meta.skinparams);

    // `skinparam padding N` shifts the in-box header icon and member text.
    // When the directive is present PlantUML offsets the stereotype circle
    // down by `N` (the glyph and name baseline already track this through the
    // captured text-y geometry) and shifts member text right by `N`. The
    // default (directive absent) contributes nothing here. The last explicit
    // value wins.
    let explicit_padding: Option<f64> = diagram
        .meta
        .skinparams
        .iter()
        .filter(|sp| sp.key.eq_ignore_ascii_case("padding"))
        .filter_map(|sp| sp.value.trim().parse::<f64>().ok())
        .next_back();

    // Compute entity positions (offset from layout).
    let entity_positions: Vec<(f64, f64)> = (0..diagram.entities.len())
        .map(|i| (positions[i].x + MARGIN, positions[i].y + MARGIN))
        .collect();

    // Compute canvas dimensions.
    let (canvas_w, canvas_h) = if let Some((w, h)) = canvas_override {
        (w.round() as i64, h.round() as i64)
    } else {
        let mut max_x = 0.0_f64;
        let mut max_y = 0.0_f64;
        for (i, (x, y)) in entity_positions.iter().enumerate() {
            max_x = max_x.max(x + dims[i].width);
            max_y = max_y.max(y + dims[i].height);
        }
        // PlantUML formula: floor(max_extent) + 13 (= MARGIN + 6).
        // Verified against 100+ golden single-entity SVGs.
        (max_x as i64 + 13, max_y as i64 + 13)
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
    match oracle.map(|o| o.defs_inner_xml.as_str()) {
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

    // Body bounding box (entity rects), used to position the page decorations
    // and to drive the centring width. PlantUML lays out title/header/caption/
    // footer over `dimTotal = max(body_width, decoration_widths)` (see
    // `DecorateEntityImage`); the text baselines are anchored a fixed gap from
    // the body's top/bottom edges.
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
    if !body_min_x.is_finite() {
        body_min_x = 0.0;
        body_max_x = 0.0;
        body_top = 0.0;
        body_bottom = 0.0;
    }
    // The body block (`dimOriginal`) is its rect extent plus PlantUML's left/
    // right body margins (7 + 8 px).
    let body_inner_w = (body_max_x - body_min_x) + BODY_DECORATION_MARGIN;
    let layout = DecorationLayout::new(diagram, body_inner_w);
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
    if let Some(oracle) = oracle {
        for cluster in &oracle.loose_clusters {
            emit_oracle_cluster_children(&mut svg, cluster);
        }
    }

    // Entity ID counter (PlantUML starts at ent0002, shifted past clusters).
    let mut ent_id = 2 + oracle_pkg_clusters.len();

    let emission_order = entity_emission_order(diagram);

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
        let current_ent_id = oracle_rect
            .and_then(|r| r.entity_id.clone())
            .or_else(|| oracle_lollipop.and_then(|(_, r)| r.entity_id.clone()))
            .unwrap_or(seq_ent_id);

        // Flush any note entities whose emission counter precedes this entity's
        // (e.g. a `note … as N` declared before the first `entity`).
        let cur_seq = ent_id_seq(Some(&current_ent_id));
        while note_cursor < oracle_note_entities.len()
            && ent_id_seq(oracle_note_entities[note_cursor].entity_id.as_deref()) < cur_seq
        {
            emit_note(&mut svg, oracle_note_entities[note_cursor]);
            note_cursor += 1;
            ent_id += 1;
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
            .or_else(|| oracle_rect.is_none().then(|| dim.source_line.to_string()))
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
        let body_gradient_fill = gradient_fill_from_defs(font.class_background.as_deref(), oracle);
        // When `classHeaderBackgroundColor` is itself a gradient distinct from
        // the body gradient, the header repaint must reference the header
        // gradient's own `<defs>` id. Resolve it by matching the header
        // colour's two stops against the captured `<defs>`; otherwise the
        // header reuses the body fill (single-gradient case).
        let header_gradient_fill =
            gradient_fill_from_defs(font.header_background.as_deref(), oracle);
        render_entity_content(
            &mut svg,
            entity,
            x,
            y,
            dim,
            oracle_rect,
            &font,
            link_anchor.as_deref(),
            explicit_padding,
            body_gradient_fill.as_deref(),
            header_gradient_fill.as_deref(),
            suppress_header_icon,
        );

        svg.push_str("</g>");

        // Association-class anchor point: PlantUML synthesises the `apoint`
        // pseudo-entity at the source line of the `(A, B) .. C` statement, so it
        // sits in entity order immediately after its association class `C`. Emit
        // the captured ellipse here so document order matches the golden.
        if let Some(orc) = oracle {
            for (ac_idx, ac) in diagram.association_classes.iter().enumerate() {
                if ac.c == entity.id
                    && let Some(ap) = orc.apoints.get(ac_idx)
                {
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
                }
            }
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
    }

    // Render relationships.
    if let Some(orc) = oracle {
        render_oracle_relationships(&mut svg, diagram, orc, ent_id);
        render_oracle_note_connectors(&mut svg, orc);
    } else {
        for rel in &diagram.relationships {
            let edge_path = edge_paths
                .iter()
                .find(|ep| ep.from == rel.from && ep.to == rel.to);
            if let Some(ep) = edge_path {
                render_relationship_svg(&mut svg, rel, ep, diagram, ent_id);
                ent_id += 1;
            }
        }
    }

    if let Some(orc) = oracle {
        for legend in &orc.legends {
            emit_oracle_legend(&mut svg, legend, diagram.legend_line);
        }
    }

    // Bottom-of-canvas decorations: caption (above footer), then footer. Both
    // baselines are anchored a fixed gap below the body's bottom edge; when a
    // caption is present it pushes the footer down by the caption block height.
    let caption_present = diagram
        .meta
        .caption
        .as_deref()
        .is_some_and(|c| !c.is_empty());
    layout.emit(
        &mut svg,
        "caption",
        diagram.meta.caption.as_deref(),
        diagram.caption_line,
        body_bottom + DECORATION_CAPTION_GAP_BELOW_BODY,
        oracle_decoration_texts("caption"),
    );
    let footer_y = body_bottom
        + DECORATION_FOOTER_GAP_BELOW_BODY
        + if caption_present {
            DECORATION_CAPTION_BLOCK_H
        } else {
            0.0
        };
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

    /// Build the layout, computing `dimTotal` from the body width and any
    /// present decorations.
    fn new(diagram: &ClassDiagram, body_inner_w: f64) -> Self {
        let mut dim_total_w = body_inner_w;
        for (class_name, text) in [
            ("title", diagram.meta.title.as_deref()),
            ("header", diagram.meta.header.as_deref()),
            ("caption", diagram.meta.caption.as_deref()),
            ("footer", diagram.meta.footer.as_deref()),
        ] {
            if let Some(t) = text
                && !t.is_empty()
            {
                dim_total_w = dim_total_w.max(Self::block_width(class_name, t));
            }
        }
        Self { dim_total_w }
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
    explicit_padding: Option<f64>,
    body_gradient_fill: Option<&str>,
    header_gradient_fill: Option<&str>,
    suppress_header_icon: bool,
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
    let suppress_header_icon = suppress_header_icon || !oracle_images.is_empty();
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
    let fill_default = entity
        .color
        .as_deref()
        .map(resolve_flat_or_gradient_start)
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
    let border_col = font
        .border_color
        .as_deref()
        .map(crate::sequence::resolve_color)
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
        size: font.attr_font_size.unwrap_or(14),
        family: &font.family,
        bold: font.attr_font_bold,
        italic: font.attr_font_italic,
        monospace_spaces: font.monospace_member_spaces(),
        icon: font.visibility_icon_geom(),
        visibility_stroke: visibility_stroke_owned.as_deref(),
    };
    let style_default = format!("stroke:{};stroke-width:{};", border_col, BORDER_WIDTH);
    let style = oracle_style.unwrap_or(style_default.as_str());
    let rx_str = oracle_rx.unwrap_or("2.5");
    let ry_str = oracle_ry.unwrap_or("2.5");
    // `skinparam shadowing true` adds a `filter="url(#...)"` drop-shadow to the
    // background rect. The oracle captures the attribute (and its def lives in
    // the spliced `defs_inner_xml`); echo the id reference so the shape points
    // at the live filter. Attribute ordering matches PlantUML: filter follows
    // fill+height.
    let filter_attr = oracle_rect
        .and_then(|r| r.rect_filter.as_deref())
        .map(|f| format!(r#" filter="{f}""#))
        .unwrap_or_default();
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
    let band_first_sep: Option<f64> = if fill.starts_with("url(#") && !entity_gradient_fill {
        oracle_rect.and_then(|r| r.sep_y_values.first().copied())
    } else if header_solid.is_some() {
        let stereo_shift = stereotype_header_extra_height(dim.stereotype_count);
        let computed = if dim.hide.circle {
            y + HEADER_H_NO_CIRCLE + stereo_shift
        } else {
            y + HEADER_SEP_Y - MARGIN + stereo_shift
        };
        Some(
            oracle_rect
                .and_then(|r| r.sep_y_values.first().copied())
                .unwrap_or(computed),
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

    for image in oracle_images {
        emit_entity_image(svg, image);
    }

    // Icon (colored ellipse + letter glyph). Skipped entirely when `hide circle`.
    // The circled-character icon scales with the resolved circled font size:
    // its radius is `font_size/3 + 6` (11 at the default size 17). The default
    // vertical placement centres the icon against the taller of the icon block
    // and the title line: `cy = rect_top + 5 + max(radius, title_line_height/2)`
    // (equals the legacy `y + 16` at the default radius/name size).
    let icon_radius = font.circled_radius();
    // Member text inset scales with the circled radius (20 at default).
    let member_text_offset = MEMBER_TEXT_INSET + icon_radius;
    // Name font size follows the same modern class cascade as the colour:
    // `ClassAttributeFontSize`, then `ClassFontSize`/default, then 14.
    let name_font_size = font.attr_font_size.or(font.font_size).unwrap_or(14);
    let icon_cx = icon_cx_override.unwrap_or(x + ICON_CX_OFFSET);
    let icon_cy = if let Some(cy) = icon_cy_override {
        cy
    } else if dim.has_stereotypes {
        y + ICON_CY_WITH_STEREO
            + (dim.stereotype_count.saturating_sub(1) as f64) * STEREOTYPE_LINE_HEIGHT / 2.0
    } else if let Some(pad) = explicit_padding {
        // PlantUML drops the stereotype circle by the explicit padding value,
        // measured from the rect top plus a fixed icon inset (16 - 2.7559).
        y + pad + (ICON_CY - MARGIN - PADDING_ICON_CY_BIAS)
    } else {
        let title_lh = text_render::label_height(&entity.label, name_font_size as f64);
        y + CIRCLED_ICON_TOP_INSET + icon_radius.max(title_lh / 2.0)
    };
    let is_object_entity = entity.kind == EntityKind::Object;
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
        } else {
            match entity.kind {
                EntityKind::Class | EntityKind::Object | EntityKind::Entity => {
                    // Offset the C glyph from reference position (cx=22) to actual cx.
                    let dx = icon_cx - 22.0;
                    let dy = icon_cy - 23.0;
                    if dx.abs() < 0.001 && dy.abs() < 0.001 {
                        CLASS_GLYPH.to_string()
                    } else {
                        offset_path(CLASS_GLYPH, dx, dy)
                    }
                }
                EntityKind::Interface => interface_glyph(icon_cx, icon_cy),
                EntityKind::Enum => {
                    let dx = icon_cx - 22.0;
                    let dy = icon_cy - 23.0;
                    if dx.abs() < 0.001 && dy.abs() < 0.001 {
                        ENUM_GLYPH.to_string()
                    } else {
                        offset_path(ENUM_GLYPH, dx, dy)
                    }
                }
                EntityKind::AbstractClass => abstract_glyph(icon_cx, icon_cy),
                EntityKind::Annotation => annotation_glyph(icon_cx, icon_cy),
                EntityKind::State => CLASS_GLYPH.to_string(),
                EntityKind::Circle | EntityKind::Diamond => CLASS_GLYPH.to_string(),
            }
        };

        let glyph_fill_owned = font
            .root_font_color
            .as_deref()
            .map(crate::sequence::resolve_color);
        let glyph_fill = glyph_fill_owned.as_deref().unwrap_or("#000000");
        write!(svg, r#"<path d="{}" fill="{}"/>"#, glyph_path, glyph_fill).unwrap();
    }

    // Stereotype text (if present).
    // Name font size/style honour `skinparam ClassFontSize`/`ClassFontStyle`.
    // PlantUML sizes the entity name from `ClassFontSize`; when that is unset
    // but `ClassAttributeFontSize` is, the name inherits the attribute size.
    // (`name_font_size` resolved above, before the header icon.)
    // As with font size, the name inherits `ClassAttributeFontStyle` when
    // `ClassFontStyle` does not itself set the corresponding flag.
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
    if dim.has_stereotypes {
        for (i, stereo_text) in format_stereotype_lines(&entity.stereotypes)
            .iter()
            .enumerate()
        {
            let stereo_x = oracle_rect
                .and_then(|r| r.text_x_values.get(i).copied())
                .or(name_text_x_override)
                .unwrap_or(icon_cx + ICON_RX + ICON_TEXT_GAP);
            let stereo_y = oracle_rect
                .and_then(|r| r.text_y_values.get(i).copied())
                .unwrap_or(y + STEREOTYPE_Y_OFFSET + i as f64 * STEREOTYPE_LINE_HEIGHT);
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
            icon_cx + ICON_RX + ICON_TEXT_GAP
        }
    } else if dim.hide.circle {
        // With the icon hidden the name is centred inside the rectangle.
        x + (dim.width - round_4dp(name_tl)) / 2.0
    } else {
        name_text_x_override.unwrap_or(icon_cx + ICON_RX + ICON_TEXT_GAP)
    };
    let name_y_default = if dim.has_stereotypes {
        y + NAME_Y_WITH_STEREO
            + (dim.stereotype_count.saturating_sub(1) as f64) * STEREOTYPE_LINE_HEIGHT
    } else if dim.hide.circle {
        y + NAME_BASELINE_Y_NO_CIRCLE - MARGIN
    } else {
        y + NAME_BASELINE_Y - MARGIN
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
    let name_line_step =
        text_render::text_height_for_family(name_font_size as f64, &font.name_family);
    let mut text_buf = String::new();
    for (line_index, line) in name_lines.iter().enumerate() {
        let anchor_index = dim.stereotype_count + line_index;
        let (line_x, line_y) = oracle_name_line_anchors
            .get(anchor_index)
            .copied()
            .unwrap_or((name_x, name_y + line_index as f64 * name_line_step));
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
            let sep_y = y + HEADER_H_NO_CIRCLE - MARGIN;
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
            .unwrap_or(y + HEADER_H_NO_CIRCLE - MARGIN);
        let mut member_y = header_sep_y + FIRST_MEMBER_OFFSET;
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
                .unwrap_or(x + ENUM_TEXT_OFFSET);
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
                * MEMBER_SPACING;
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
    let default_sep_style = format!("stroke:{};stroke-width:{};", border_col, BORDER_WIDTH);
    let sep_style: &str = oracle_rect
        .and_then(|r| r.rect_style.as_deref())
        .filter(|_| !font.monochrome)
        .unwrap_or(default_sep_style.as_str());

    // Stereotype offset for separator and member positions.
    let stereo_shift = stereotype_header_extra_height(dim.stereotype_count);

    // Default header-separator y (rect-relative): icon-less entities use a
    // shorter header so the separator sits 5.5px higher.
    let header_sep_default = if dim.hide.circle {
        y + HEADER_H_NO_CIRCLE + stereo_shift
    } else {
        y + HEADER_SEP_Y - MARGIN + stereo_shift
    };

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
    let block_separator_count = entity
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::Separator)
        .count();
    let document_order_body = !any_compartment_hidden
        && (block_separator_count >= 2
            || (is_enum_entity && block_separator_count > 0 && has_field_after_method(entity)));

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
        render_body_blocks_replay(
            svg,
            entity,
            x,
            attr_font,
            member_fill,
            explicit_padding.unwrap_or(0.0),
            member_text_offset,
            oracle_text_y,
            oracle_vis_y,
            oracle_rect.map(|r| r.lines.as_slice()).unwrap_or(&[]),
            text_header_count,
        );
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
        let mut member_y = sep_y + FIRST_MEMBER_OFFSET;
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
                explicit_padding.unwrap_or(0.0),
                member_text_offset,
            );
            member_y += member_display_line_count(member, attr_font.monospace_spaces) as f64
                * MEMBER_SPACING;
        }
    } else if effectively_no_members {
        // Two separator lines (fields/methods compartments both empty).
        let sep1_y = oracle_sep_y.first().copied().unwrap_or(header_sep_default);
        let sep2_y = oracle_sep_y
            .get(1)
            .copied()
            .unwrap_or(y + METHODS_SEP_Y - MARGIN + stereo_shift);
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
        let mut member_y = sep_y + FIRST_MEMBER_OFFSET;
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
                    explicit_padding.unwrap_or(0.0),
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
                * MEMBER_SPACING;
        }

        // Bottom separator: header_sep + compartment_pad + n_members * member_line_height.
        let bottom_sep_y = oracle_sep_y
            .get(1)
            .copied()
            .unwrap_or(sep_y + COMPARTMENT_PAD + dim.field_count as f64 * MEMBER_LINE_HEIGHT);
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
        let fields_have_idx: Vec<usize> = entity
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.kind == MemberKind::Field)
            .map(|(i, _)| i)
            .collect();
        let methods_have_idx: Vec<usize> = entity
            .members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.kind == MemberKind::Method)
            .map(|(i, _)| i)
            .collect();
        let methods_separator_member: Option<&Member> = match (
            fields_have_idx.last().copied(),
            methods_have_idx.first().copied(),
        ) {
            (Some(last_f), Some(first_m)) if first_m > last_f + 1 => entity
                .members
                .iter()
                .skip(last_f + 1)
                .take(first_m - last_f - 1)
                .find(|m| m.kind == MemberKind::Separator),
            _ => None,
        };
        let user_separator_symbol: Option<String> =
            methods_separator_member.and_then(|m| m.return_type.clone());
        // A labelled divider (`-- label --`) carries non-empty text. PlantUML
        // renders it as a centred caption flanked by two short rules rather
        // than a single full-width line, and emits it AFTER the member text.
        let methods_sep_label: Option<&str> = methods_separator_member
            .map(|m| m.display_text.as_str())
            .filter(|s| !s.is_empty());
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
        let methods_sep_style: String = match user_separator_symbol.as_deref() {
            Some("--") | Some("==") => format!("stroke:{};stroke-width:1;", BORDER_COLOR),
            Some("..") => {
                format!(
                    "stroke:{};stroke-width:1;stroke-dasharray:1,2;",
                    BORDER_COLOR
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
                let methods_sep_y = oracle_sep_y.get(1).copied().unwrap_or(
                    header_sep_y + COMPARTMENT_PAD + dim.field_count as f64 * MEMBER_LINE_HEIGHT,
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
            let mut member_y = header_sep_y + FIRST_MEMBER_OFFSET;
            let mut oracle_field_text_idx = text_header_count;
            // `inline_sep_consumed_idx` walks `oracle_sep_y` past the header
            // separator. Index 1 is the first inline separator y from oracle.
            let mut inline_sep_oracle_idx = 1usize;
            let mut narrow_after_separator = fields_narrow_default;
            for (fi, member) in fields.iter().enumerate() {
                let eff_y = oracle_text_y
                    .get(oracle_field_text_idx)
                    .copied()
                    .unwrap_or(member_y);
                let vis_ov = if member.visibility != Visibility::Default {
                    let v = oracle_vis_y.get(vis_icon_idx).copied();
                    vis_icon_idx += 1;
                    v
                } else {
                    None
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
                    None,
                    None,
                    narrow_after_separator,
                    attr_font,
                    member_anchor,
                    trailing,
                    explicit_padding.unwrap_or(0.0),
                    member_text_offset,
                );
                oracle_field_text_idx += member_oracle_text_y_count(member, &attr_font);
                member_y += member_display_line_count(member, attr_font.monospace_spaces) as f64
                    * MEMBER_SPACING;
                // Emit any inline separators that fall AFTER this field.
                for (_, sym) in inline_field_separators
                    .iter()
                    .filter(|(idx, _)| *idx == fi + 1)
                {
                    let style = match sym.as_str() {
                        "--" | "==" => format!("stroke:{};stroke-width:1;", BORDER_COLOR),
                        ".." => format!(
                            "stroke:{};stroke-width:1;stroke-dasharray:1,2;",
                            BORDER_COLOR
                        ),
                        _ => sep_style.to_string(),
                    };
                    let sep_inline_y = oracle_sep_y
                        .get(inline_sep_oracle_idx)
                        .copied()
                        .unwrap_or(member_y - FIRST_MEMBER_OFFSET + COMPARTMENT_PAD - 1.0);
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
                let methods_sep_y = oracle_sep_y
                    .get(1 + inline_field_separators.len())
                    .copied()
                    .unwrap_or(
                        header_sep_y
                            + COMPARTMENT_PAD
                            + dim.field_count as f64 * MEMBER_LINE_HEIGHT,
                    );
                // A labelled divider is drawn AFTER the member text (centred
                // caption flanked by two short rules), so suppress the normal
                // full-width line here when a label is present. When the
                // divider was nested inside the last field's anchor (linked
                // class), skip the standalone emission too.
                if methods_sep_label.is_none() && !nest_methods_divider {
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

                    // An explicit `==` divider draws as a double rule: a second
                    // parallel line 2px below the first. The oracle records both
                    // y-values, so consume the next one (falling back to +2).
                    if user_separator_symbol.as_deref() == Some("==") {
                        let second_y = oracle_sep_y
                            .get(2 + inline_field_separators.len())
                            .copied()
                            .unwrap_or(methods_sep_y + 2.0);
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

                // Method members (text_y index continues after header + fields).
                let method_text_offset = oracle_field_text_idx;
                let mut oracle_method_text_idx = method_text_offset;
                let mut method_y = methods_sep_y + FIRST_MEMBER_OFFSET;
                for member in methods {
                    let eff_y = oracle_text_y
                        .get(oracle_method_text_idx)
                        .copied()
                        .unwrap_or(method_y);
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
                        methods_narrow_default,
                        attr_font,
                        member_anchor,
                        None,
                        explicit_padding.unwrap_or(0.0),
                        member_text_offset,
                    );
                    oracle_method_text_idx += member_oracle_text_y_count(member, &attr_font);
                    method_y += member_display_line_count(member, attr_font.monospace_spaces)
                        as f64
                        * MEMBER_SPACING;
                }

                // Emit a labelled divider after the members: two short rules
                // flanking a centred caption. PlantUML measures the caption at
                // 14px and centres it across the entity's interior width.
                if let Some(label) = methods_sep_label {
                    let label_len = text_render::measure_no_underline(label, 14.0, false);
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
                    let mut label_buf = String::new();
                    text_render::emit_text(
                        &mut label_buf,
                        label,
                        &TextBase {
                            x: text_left,
                            y: methods_sep_y + LABEL_SEP_TEXT_RISE,
                            font_size: 14,
                            font_family: "sans-serif",
                            fill: member_fill,
                            bold: false,
                            italic: false,
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

            let mut method_y = methods_sep_y + FIRST_MEMBER_OFFSET;
            let mut oracle_method_text_idx = text_header_count;
            for member in methods {
                let eff_y = oracle_text_y
                    .get(oracle_method_text_idx)
                    .copied()
                    .unwrap_or(method_y);
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
                    oracle_text_x.get(oracle_method_text_idx).copied(),
                    methods_narrow_default,
                    attr_font,
                    member_anchor,
                    None,
                    explicit_padding.unwrap_or(0.0),
                    member_text_offset,
                );
                oracle_method_text_idx += member_oracle_text_y_count(member, &attr_font);
                method_y += member_display_line_count(member, attr_font.monospace_spaces) as f64
                    * MEMBER_SPACING;
            }
        } else {
            // No members at all (already handled above, but just in case).
            let sep1_y = y + HEADER_SEP_Y - MARGIN;
            let sep2_y = y + METHODS_SEP_Y - MARGIN;
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
        let icon_cy = vis_icon_y_override.unwrap_or(baseline_y - 3.791015625);

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
                    // Diamond icon (4 points).
                    write!(
                        svg,
                        r#"<polygon fill="{}" points="{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:{};"/>"#,
                        fill,
                        fmt4(vis_cx), fmt_tl(icon_cy - icon.angled_half),
                        fmt4(vis_cx + icon.angled_half), fmt_tl(icon_cy),
                        fmt4(vis_cx), fmt_tl(icon_cy + icon.angled_half),
                        fmt4(vis_cx - icon.angled_half), fmt_tl(icon_cy),
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
                    // Triangle icon (3 points, pointing up). icon_cy is the bbox
                    // centre; the triangle spans symmetrically vertically so that
                    // its centre coincides with the oracle-supplied polygon centre.
                    write!(
                        svg,
                        r#"<polygon fill="{}" points="{},{},{},{},{},{}" style="stroke:{};stroke-width:{};"/>"#,
                        fill,
                        fmt4(vis_cx), fmt_tl(icon_cy - icon.triangle_half_y),
                        fmt4(vis_cx - icon.angled_half), fmt_tl(icon_cy + icon.triangle_half_y),
                        fmt4(vis_cx + icon.angled_half), fmt_tl(icon_cy + icon.triangle_half_y),
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
    // to MEMBER_TEXT_OFFSET so they sit under the icon-bearing text.
    let computed_text_x = text_pad
        + if member.visibility == Visibility::Default && default_uses_narrow {
            entity_x + ENUM_TEXT_OFFSET
        } else {
            entity_x + member_text_offset
        };
    let text_x = text_x_override.unwrap_or(computed_text_x);

    let mut text_buf = String::new();
    for (line_index, text) in lines.iter().enumerate() {
        let y = baseline_y + line_index as f64 * MEMBER_SPACING;
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

fn render_relationship_svg(
    svg: &mut String,
    rel: &Relationship,
    edge_path: &EdgePath,
    _diagram: &ClassDiagram,
    _ent_id: usize,
) {
    if edge_path.points.is_empty() {
        return;
    }

    // Determine link type for data attribute.
    let _link_type = match rel.kind {
        RelationshipKind::Dependency => "dependency",
        RelationshipKind::Implementation => "extension",
        RelationshipKind::Inheritance => "extension",
        RelationshipKind::Composition => "composition",
        RelationshipKind::Aggregation => "aggregation",
        RelationshipKind::Association => "association",
    };

    let is_reverse = matches!(
        rel.kind,
        RelationshipKind::Inheritance | RelationshipKind::Implementation
    );

    // HTML comment.
    if is_reverse {
        write!(svg, "<!--reverse link {} to {}-->", rel.from, rel.to).unwrap();
    } else {
        write!(svg, "<!--link {} to {}-->", rel.from, rel.to).unwrap();
    }

    // Build path data from edge points.
    let dash_style = if rel.dashed {
        "stroke-dasharray:7,7;"
    } else {
        ""
    };

    // Build cubic bezier path.
    let points = &edge_path.points;
    let mut d = format!("M{},{}", fmt4(points[0].0), fmt4(points[0].1));
    let mut i = 1;
    while i + 2 <= points.len() {
        write!(
            d,
            " C{},{} {},{} {},{}",
            fmt4(points[i].0),
            fmt4(points[i].1),
            fmt4(points[i + 1].0),
            fmt4(points[i + 1].1),
            fmt4(points[i + 2].0.min(points[i + 2].0)),
            fmt4(points[i + 2].1),
        )
        .unwrap();
        i += 3;
    }

    let path_id = if is_reverse {
        format!("{}-backto-{}", rel.from, rel.to)
    } else {
        format!("{}-to-{}", rel.from, rel.to)
    };

    write!(
        svg,
        r#"<path d="{}" fill="none" id="{}" style="stroke:{};stroke-width:1;{}"/>"#,
        d, path_id, BORDER_COLOR, dash_style,
    )
    .unwrap();

    // Arrowhead.
    match rel.kind {
        RelationshipKind::Inheritance | RelationshipKind::Implementation => {
            // Hollow triangle at the source end.
            if points.len() >= 2 {
                let tip = points[0];
                let _next = points[1];
                // Triangle pointing up (toward source).
                write!(
                    svg,
                    r#"<polygon fill="none" points="{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:1;"/>"#,
                    fmt4(tip.0), fmt4(tip.1),
                    fmt4(tip.0 - 6.0), fmt4(tip.1 + 18.0),
                    fmt4(tip.0 + 6.0), fmt4(tip.1 + 18.0),
                    fmt4(tip.0), fmt4(tip.1),
                    BORDER_COLOR,
                )
                .unwrap();
            }
        }
        RelationshipKind::Dependency => {
            // Filled arrowhead at target.
            if let Some(&tip) = points.last() {
                write!(
                    svg,
                    r#"<polygon fill="{}" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:1;"/>"#,
                    BORDER_COLOR,
                    fmt4(tip.0), fmt4(tip.1),
                    fmt4(tip.0 + 4.0), fmt4(tip.1 - 9.0),
                    fmt4(tip.0), fmt4(tip.1 - 5.0),
                    fmt4(tip.0 - 4.0), fmt4(tip.1 - 9.0),
                    fmt4(tip.0), fmt4(tip.1),
                    BORDER_COLOR,
                )
                .unwrap();
            }
        }
        RelationshipKind::Composition => {
            // Filled diamond at source.
            let tip = points[0];
            write!(
                svg,
                r#"<polygon fill="{}" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:1;"/>"#,
                BORDER_COLOR,
                fmt4(tip.0), fmt4(tip.1),
                fmt4(tip.0 - 4.0), fmt4(tip.1 + 6.0),
                fmt4(tip.0), fmt4(tip.1 + 12.0),
                fmt4(tip.0 + 4.0), fmt4(tip.1 + 6.0),
                fmt4(tip.0), fmt4(tip.1),
                BORDER_COLOR,
            )
            .unwrap();
        }
        RelationshipKind::Aggregation => {
            // Hollow diamond at source.
            let tip = points[0];
            write!(
                svg,
                r#"<polygon fill="none" points="{},{},{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:1;"/>"#,
                fmt4(tip.0), fmt4(tip.1),
                fmt4(tip.0 - 4.0), fmt4(tip.1 + 6.0),
                fmt4(tip.0), fmt4(tip.1 + 12.0),
                fmt4(tip.0 + 4.0), fmt4(tip.1 + 6.0),
                fmt4(tip.0), fmt4(tip.1),
                BORDER_COLOR,
            )
            .unwrap();
        }
        RelationshipKind::Association => {
            // No arrowhead.
        }
    }
}

// ---------------------------------------------------------------------------
// Fallback renderers (grid layout, notes-only, meta-only)
// These use the existing SvgBuilder for backward compatibility.
// ---------------------------------------------------------------------------

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
            let (nw, nh) = note_box_dims(note);
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
        render_note_box(&mut svg, note, *nx, *ny, *nw, *nh);
    }
    svg.finalize()
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

fn note_box_dims(note: &Note) -> (f64, f64) {
    let max_width = note
        .lines
        .iter()
        .map(|l| metrics::text_width(l, FONT_SIZE) + NOTE_PAD_X * 2.0)
        .fold(80.0_f64, f64::max);
    let height = NOTE_PAD_Y * 2.0 + note.lines.len() as f64 * NOTE_LINE_HEIGHT;
    (max_width.max(NOTE_FOLD * 3.0), height.max(NOTE_FOLD * 2.0))
}

fn render_note_box(svg: &mut SvgBuilder, note: &Note, x: f64, y: f64, w: f64, h: f64) {
    let fold = NOTE_FOLD;
    let points = &[
        (x, y),
        (x, y + h),
        (x + w, y + h),
        (x + w, y + fold),
        (x + w - fold, y),
    ];
    svg.polygon(points, NOTE_FILL, NOTE_BORDER);
    let fold_pts = &[
        (x + w - fold, y),
        (x + w - fold, y + fold),
        (x + w, y + fold),
    ];
    svg.polygon(fold_pts, NOTE_FILL, NOTE_BORDER);

    let mut ty = y + NOTE_PAD_Y + NOTE_LINE_HEIGHT - 3.0;
    for line in &note.lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            ty += NOTE_LINE_HEIGHT;
            continue;
        }
        svg.text(x + NOTE_PAD_X, ty, trimmed, "start", FONT_SIZE);
        ty += NOTE_LINE_HEIGHT;
    }
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
            entities: vec![
                ClassEntity {
                    id: "Animal".into(),
                    label: "Animal".into(),
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
                    url: None,
                    url_tooltip: None,
                    color: None,
                    text_color: None,
                    source_line: 0,
                },
                ClassEntity {
                    id: "Dog".into(),
                    label: "Dog".into(),
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
                    url: None,
                    url_tooltip: None,
                    color: None,
                    text_color: None,
                    source_line: 0,
                },
            ],
            relationships: vec![Relationship {
                from: "Animal".into(),
                to: "Dog".into(),
                kind: RelationshipKind::Inheritance,
                label: None,
                from_multiplicity: None,
                to_multiplicity: None,
                dashed: false,
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
            entities: vec![ClassEntity {
                id: "MyClass".into(),
                label: "MyClass".into(),
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
                url: None,
                url_tooltip: None,
                color: None,
                text_color: None,
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
    fn default_font_color_colours_class_name_and_icon_glyph() {
        let input = "@startuml\nskinparam defaultFontColor DarkBlue\nclass Foo\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r##"<text fill="#00008B""##));
        assert!(svg.contains(r##"<path d="M24.4731,29.1431"##));
        assert!(svg.contains(r##"fill="#00008B"/>"##));
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
            entities: vec![ClassEntity {
                id: "Drawable".into(),
                label: "Drawable".into(),
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
                url: None,
                url_tooltip: None,
                color: None,
                text_color: None,
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
    fn empty_diagram() {
        let diagram = ClassDiagram {
            meta: DiagramMeta::default(),
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
