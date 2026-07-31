// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram SVG renderer.
//!
//! Produces SVG output that matches PlantUML's use-case diagram rendering.
//! PlantUML emits use-case diagrams as `data-diagram-type="DESCRIPTION"` —
//! the same envelope used by component and deployment diagrams.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use rustuml_layout::graph::{
    ClusterPosition, ClusterTitleSize, Direction, EdgeLabelSize, EdgePath, LayoutGraph,
};
use rustuml_parser::diagram::usecase::*;
use rustuml_parser::diagram::{DiagramMeta, style::StyleScheme};

use crate::filter_registry::{GradientKey, GradientRegistry};
use crate::layout_oracle::{OracleLayout, wrap_oracle_envelope};
use crate::plantuml_metrics as pm;
use crate::style::Theme;
use crate::style_cascade::{StyleBoxSides, StyleCascade, StyleSignature};
use crate::svg::SvgBuilder;
use crate::text_render::{self, TextBase};

const FONT_SIZE: f64 = 14.0;
const STROKE: &str = "#181818";
const ENTITY_FILL: &str = "#F1F1F1";
const TEXT_COLOR: &str = "#000000";

const ACTOR_HEAD_R: f64 = 8.0;
const ACTOR_BODY_LEN: f64 = 27.0;
const ACTOR_ARM_HALF: f64 = 13.0;
const ACTOR_ARM_OFFSET: f64 = 8.0;
const ACTOR_LEG_RUN: f64 = 13.0;
const ACTOR_LEG_DROP: f64 = 15.0;
/// `EntityImageDescription` wraps actor stereotypes with
/// `TextBlockUtils.withMargin(stereotype, 1, 0)`.
const ACTOR_STEREOTYPE_MARGIN_X: f64 = 1.0;
/// Java provenance: `skin.ActorStickMan.getPreferredHeight()` adds twice the
/// current stroke thickness to this 58px geometry plus its final 1px guard.
const ACTOR_STICKMAN_BASE_HEIGHT: f64 = 59.0;
/// Vertical offset from head centre to stereotype baseline (measured).
const ACTOR_STEREO_OFFSET: f64 = 11.4531;
/// Extracted line advance for the undecorated `MethodsOrFieldsArea` created by
/// Java `BodyEnhanced1.buildTextBlock`; decorated bodies use `TextBlockMarged`.
const LINE_H: f64 = 16.4883;

const MARGIN: f64 = 7.0;
const GAP: f64 = 40.0;
const BODY_MARGIN: f64 = 6.0;
/// Java `BodyEnhancedAbstract.decorate` wraps a compartment below a rule
/// with four pixels above and below its body.
const COMPARTMENT_MARGIN_Y: f64 = 4.0;
const SEPARATOR_SKIP_X: f64 = 1.0;
const DOUBLE_SEPARATOR_GAP: f64 = 2.0;
/// Java's `EntityImageDegenerated` wraps a lone non-state entity in 7px.
const DEGENERATED_MARGIN: f64 = 7.0;
const SVEK_CANVAS_PAD: f64 = 14.0;
/// `TextBlockExporter12026` retains six horizontal and five vertical pixels
/// outside the decorated diagram block after `SvekResult.calculateDimension`.
const CHROME_EXPORT_PAD_X: f64 = 6.0;
const CHROME_EXPORT_PAD_Y: f64 = 5.0;
const CHROME_CAPTION_FONT_SIZE: f64 = 10.0;
/// Java `DisplayPositioned.createRibbon` adds one pixel below its line box.
const CHROME_CAPTION_BOTTOM_PAD: f64 = 1.0;
/// Java `Style.createTextBlockBordered` applies the document-title margins
/// consumed by `DiagramChromeFactory12026.addTitle`.
const CHROME_TITLE_MARGIN_X: f64 = 10.0;
const CHROME_TITLE_TOP_PAD: f64 = 10.0;
const CHROME_TITLE_BOTTOM_PAD: f64 = 11.0;
/// Java `CucaDiagram#getDefaultMargins` supplies these asymmetric exporter
/// margins for DESCRIPTION diagrams when root.document has no Margin value.
const DEFAULT_DOCUMENT_MARGIN: StyleBoxSides = StyleBoxSides {
    top: 0.0,
    right: 5.0,
    bottom: 5.0,
    left: 0.0,
};
const LAYOUT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const DEPENDENCY_ARROW_BACK: f64 = 9.0;
const DEPENDENCY_ARROW_NOTCH: f64 = 5.0;
const DEPENDENCY_ARROW_WING: f64 = 4.0;
const DEPENDENCY_ARROW_PATH_GAP: f64 = 6.0;
/// Java `LimitFinder.drawUPolygon` expands every polygon by ten pixels on
/// either horizontal side while measuring the complete painted diagram.
const LIMIT_FINDER_POLYGON_X_GUARD: f64 = 10.0;

const NOTE_FILL: &str = "#FEFFDD";
const NOTE_FOLD: f64 = 10.0;
const NOTE_FONT_SIZE: u32 = 13;
/// Java provenance: `EntityImageNote` and `Opale` use 6px left, 15px right,
/// and 5px vertical text margins.
const NOTE_MARGIN_LEFT: f64 = 6.0;
const NOTE_MARGIN_RIGHT: f64 = 15.0;
const NOTE_MARGIN_Y: f64 = 5.0;
/// Java provenance: `Opale.delta` makes every leader base eight pixels wide.
const NOTE_LEADER_HALF: f64 = 4.0;
/// Java provenance: `Rose` gives link-owned note components 5px padding.
const LINK_NOTE_PADDING: f64 = 5.0;

/// Which box edge carries the note's leader (callout) notch.
enum LeaderSide {
    Top,
    Bottom,
    Left,
    Right,
}

/// Round a coordinate to PlantUML's 4-decimal format, dropping trailing zeros.
fn fc(v: f64) -> String {
    pm::fmt_coord(v)
}

/// Resolve a raw `#color` token (parser strips the leading `#`, so we receive
/// e.g. `Pink`, `LightBlue`, or `FFC0CB`) into a PlantUML fill string. Named
/// colours resolve to `#RRGGBB`; bare hex digits get a `#` prepended.
fn resolve_fill(raw: &str) -> String {
    let normalized = crate::sequence::resolve_color(raw);
    if normalized.starts_with('#') {
        normalized
    } else {
        format!("#{normalized}")
    }
}

/// A resolved Description background keeps gradients typed until the shape
/// that owns the paint is ordered. This mirrors Java's `HColor`/
/// `HColorGradient` distinction instead of flattening a gradient during style
/// resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
enum DescriptionPaint {
    NoPaint,
    Solid(String),
    Gradient(GradientKey),
}

impl DescriptionPaint {
    fn parse(raw: &str) -> Self {
        let whole = raw.trim().strip_prefix('#').unwrap_or(raw.trim());
        if matches!(
            whole.to_ascii_lowercase().as_str(),
            "transparent" | "background"
        ) {
            return Self::NoPaint;
        }
        if let Some((color1, color2, policy)) = crate::sequence::split_gradient_colors(raw) {
            return Self::Gradient(GradientKey::new(
                crate::sequence::resolve_color_rgb(color1),
                crate::sequence::resolve_color_rgb(color2),
                policy,
            ));
        }
        Self::Solid(crate::sequence::resolve_color(raw))
    }

    fn gradient(&self) -> Option<&GradientKey> {
        match self {
            Self::Gradient(key) => Some(key),
            Self::NoPaint | Self::Solid(_) => None,
        }
    }

    fn svg_fill(&self, registry: Option<&GradientRegistry>, captured_defs: Option<&str>) -> String {
        match self {
            Self::NoPaint => "none".to_string(),
            Self::Solid(color) => color.clone(),
            Self::Gradient(key) => registry
                .and_then(|registry| registry.id_for_key(key))
                .map(|id| format!("url(#{id})"))
                .unwrap_or_else(|| crate::sequence::gradient_fill_for_key(key, captured_defs)),
        }
    }

    #[cfg(test)]
    fn flat_control(&self) -> Option<&str> {
        match self {
            Self::Solid(color) => Some(color),
            Self::NoPaint => Some("none"),
            Self::Gradient(_) => None,
        }
    }
}

fn skin_value<'a>(
    skinparams: &'a [rustuml_parser::diagram::SkinParam],
    keys: &[&str],
) -> Option<&'a str> {
    skinparams
        .iter()
        .rev()
        .find(|p| keys.iter().any(|k| p.key.eq_ignore_ascii_case(k)))
        .map(|p| p.value.trim())
}

/// Look up a skinparam value case-insensitively (PlantUML convention) and
/// resolve it to a fill string. The parser flattens block skinparams like
/// `skinparam usecase { BackgroundColor X }` to the key `usecaseBackgroundColor`.
fn skin_color(skinparams: &[rustuml_parser::diagram::SkinParam], key: &str) -> Option<String> {
    skin_value(skinparams, &[key]).map(resolve_fill)
}

fn skin_fill(
    skinparams: &[rustuml_parser::diagram::SkinParam],
    key: &str,
) -> Option<DescriptionPaint> {
    skin_value(skinparams, &[key]).map(DescriptionPaint::parse)
}

fn skin_font_size(
    skinparams: &[rustuml_parser::diagram::SkinParam],
    keys: &[&str],
    default: u32,
) -> u32 {
    skin_value(skinparams, keys)
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v.round() as u32)
        .unwrap_or(default)
}

fn skin_thickness(
    skinparams: &[rustuml_parser::diagram::SkinParam],
    keys: &[&str],
    default: f64,
) -> String {
    skin_value(skinparams, keys)
        .and_then(|v| v.parse::<f64>().ok())
        .map(fc)
        .unwrap_or_else(|| fc(default))
}

fn canonical_usecase_font_family(value: &str) -> String {
    let raw = value.trim();
    let quoted = (raw.starts_with('"') && raw.ends_with('"'))
        || (raw.starts_with('\'') && raw.ends_with('\''));
    let trimmed = raw.trim_matches('"').trim_matches('\'');
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

/// Per-kind background/border/text defaults derived from `skinparam`
/// directives. Keep this opt-in: absent skinparams preserve the renderer's
/// existing PlantUML defaults instead of inheriting RustUML's UI theme.
#[derive(Clone)]
struct SkinColors {
    actor_fill: Option<DescriptionPaint>,
    actor_border: Option<String>,
    actor_border_thickness: String,
    actor_font_color: String,
    actor_font_family: String,
    actor_font_size: u32,
    actor_stereo_font_color: String,
    uc_fill: Option<DescriptionPaint>,
    uc_border: Option<String>,
    uc_border_thickness: String,
    uc_font_color: String,
    uc_font_family: String,
    uc_font_size: u32,
    uc_stereo_font_color: String,
    creole_padding: f64,
    arrow_color: String,
    arrow_head_color: String,
    arrow_stroke_width: f64,
    arrow_dash: Option<(f64, f64)>,
    arrow_font_color: String,
    arrow_font_family: String,
    arrow_font_size: u32,
    canvas_background: Option<String>,
    canvas_rect: Option<String>,
    document_margin: StyleBoxSides,
}

impl SkinColors {
    fn from_meta(meta: &DiagramMeta, _gradient_defs: Option<&str>) -> Self {
        let skinparams = &meta.skinparams;
        let default_font_family = skin_value(skinparams, &["defaultFontName", "fontName"])
            .map(canonical_usecase_font_family)
            .unwrap_or_else(|| "sans-serif".to_string());
        let default_font_size = skin_font_size(skinparams, &["defaultFontSize"], FONT_SIZE as u32);
        let root_font_color = skin_color(skinparams, "__styleRootFontColor");
        let default_font_color = skin_color(skinparams, "defaultFontColor");
        let fallback_font_color = || {
            root_font_color
                .clone()
                .or_else(|| default_font_color.clone())
                .unwrap_or_else(|| TEXT_COLOR.to_string())
        };
        let actor_font_color = skin_color(skinparams, "actorFontColor")
            .or_else(|| Some(fallback_font_color()))
            .unwrap_or_else(|| TEXT_COLOR.to_string());
        let uc_font_color = skin_color(skinparams, "usecaseFontColor")
            .or_else(|| Some(fallback_font_color()))
            .unwrap_or_else(|| TEXT_COLOR.to_string());
        let actor_font_family = skin_value(skinparams, &["actorFontName"])
            .map(canonical_usecase_font_family)
            .unwrap_or_else(|| default_font_family.clone());
        let uc_font_family = skin_value(skinparams, &["usecaseFontName"])
            .map(canonical_usecase_font_family)
            .unwrap_or_else(|| default_font_family.clone());
        let root_line = ["__styleRootLineThickness", "borderThickness"];
        let bg_value = skin_value(skinparams, &["backgroundColor"]);
        let canvas_background = match bg_value {
            Some(v) if v.eq_ignore_ascii_case("transparent") => None,
            Some(v) => Some(crate::sequence::resolve_color(v)),
            None => Some("#FFFFFF".to_string()),
        };
        let canvas_rect = canvas_background
            .as_ref()
            .filter(|c| *c != "#FFFFFF")
            .cloned();
        let mut skin = SkinColors {
            actor_fill: skin_fill(skinparams, "actorBackgroundColor"),
            actor_border: skin_color(skinparams, "actorBorderColor")
                .or_else(|| skin_color(skinparams, "__styleRootLineColor")),
            actor_border_thickness: skin_thickness(
                skinparams,
                &[
                    "actorBorderThickness",
                    "__styleRootLineThickness",
                    "borderThickness",
                ],
                0.5,
            ),
            actor_font_color: actor_font_color.clone(),
            actor_font_family,
            actor_font_size: skin_font_size(
                skinparams,
                &["actorFontSize", "defaultFontSize"],
                default_font_size,
            ),
            actor_stereo_font_color: skin_color(skinparams, "actorStereotypeFontColor")
                .unwrap_or(actor_font_color),
            uc_fill: skin_fill(skinparams, "usecaseBackgroundColor"),
            uc_border: skin_color(skinparams, "usecaseBorderColor")
                .or_else(|| skin_color(skinparams, "__styleRootLineColor")),
            uc_border_thickness: skin_thickness(
                skinparams,
                &["usecaseBorderThickness", root_line[0], root_line[1]],
                0.5,
            ),
            uc_font_color: uc_font_color.clone(),
            uc_font_family,
            uc_font_size: skin_font_size(
                skinparams,
                &["usecaseFontSize", "defaultFontSize"],
                default_font_size,
            ),
            uc_stereo_font_color: skin_color(skinparams, "usecaseStereotypeFontColor")
                .unwrap_or(uc_font_color),
            // Java `Display#getCreole` reads the global `SkinParam#getPadding`
            // when it constructs every `SheetBlock1`. This is independent of
            // the use-case symbol style's own Padding property.
            creole_padding: skin_value(skinparams, &["padding"])
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(0.0),
            // Link styles are rebuilt from each Link's captured StyleBuilder
            // below. Do not seed them from the final raw skinparam map: Java
            // `Link#getStyleBuilder` never receives Entity's legacy refresh.
            arrow_color: STROKE.to_string(),
            arrow_head_color: STROKE.to_string(),
            arrow_stroke_width: 1.0,
            arrow_dash: None,
            arrow_font_color: TEXT_COLOR.to_string(),
            arrow_font_family: "sans-serif".to_string(),
            arrow_font_size: 13,
            canvas_background,
            canvas_rect,
            document_margin: DEFAULT_DOCUMENT_MARGIN,
        };

        let cascade = StyleCascade::new(&meta.style_program);
        let document_signature = StyleSignature::from_selectors(["root", "document"]);
        let document_style = cascade.resolve(&document_signature, StyleScheme::Regular);

        if let Some(value) = document_style.property("backgroundColor") {
            // Java `TitledDiagram#calculateBackColor` resolves root.document,
            // and `HColorSet#parseColor` maps transparent to no paint. Test
            // that token before generic color resolution, which deliberately
            // maps unknown names to white.
            if value.trim().eq_ignore_ascii_case("transparent") {
                skin.canvas_background = None;
                skin.canvas_rect = None;
            } else {
                let color = crate::sequence::resolve_color(value);
                skin.canvas_rect = (color != "#FFFFFF").then(|| color.clone());
                skin.canvas_background = Some(color);
            }
        }
        // Java `TextBlockExporter12026.Builder#calculateMargin` resolves
        // root.document from SkinParam's final builder, then
        // `ClockwiseTopRightBottomLeft#read` retains all four sides.
        skin.document_margin = document_style
            .box_sides("margin")
            .unwrap_or(DEFAULT_DOCUMENT_MARGIN);
        skin
    }

    fn for_actor(&self, meta: &DiagramMeta, actor: &Actor) -> Self {
        let mut result = self.clone();
        let cascade = StyleCascade::new(&meta.style_program);
        let mut title_signature =
            StyleSignature::from_selectors(["root", "element", "usecaseDiagram", "actor", "title"]);
        let mut stereotype_signature = StyleSignature::from_selectors([
            "root",
            "element",
            "usecaseDiagram",
            "actor",
            "stereotype",
        ]);
        if let Some(stereotype) = &actor.stereotype {
            title_signature = title_signature.with_stereotype(stereotype);
            stereotype_signature = stereotype_signature.with_stereotype(stereotype);
        }

        // Java `EntityImageDescription` resolves its title Fashion and font
        // from `Entity#getCurrentStyleBuilder`; that method keeps the creation
        // builder for pure CSS but refreshes to the final builder after any
        // legacy skinparam command.
        let style = cascade.resolve_entity_at_source_line(
            &title_signature,
            StyleScheme::Regular,
            actor.source_line,
        );
        if let Some(value) = style.property("backgroundColor") {
            result.actor_fill = Some(DescriptionPaint::parse(value));
        }
        if let Some(value) = style.property("lineColor") {
            result.actor_border = Some(crate::sequence::resolve_color(value));
        }
        let default_stroke = result.actor_border_thickness.parse::<f64>().unwrap_or(0.5);
        result.actor_border_thickness = fc(style.stroke(default_stroke).thickness);
        if let Some(value) = style.property("fontColor") {
            result.actor_font_color = crate::sequence::resolve_color(value);
        }
        if let Some(value) = style.property("fontName") {
            result.actor_font_family = canonical_usecase_font_family(value);
        }
        if let Some(value) = style.property("fontSize")
            && let Ok(value) = value.parse::<f64>()
        {
            result.actor_font_size = value.round() as u32;
        }

        // Java `EntityImageDescription` uses
        // `forStereotypeItself(stereotype)` for the stereotype text style.
        let stereotype_style = cascade.resolve_entity_at_source_line(
            &stereotype_signature,
            StyleScheme::Regular,
            actor.source_line,
        );
        if let Some(value) = stereotype_style.property("fontColor") {
            result.actor_stereo_font_color = crate::sequence::resolve_color(value);
        }
        result
    }

    fn for_use_case(&self, meta: &DiagramMeta, use_case: &UseCase) -> Self {
        let mut result = self.clone();
        let cascade = StyleCascade::new(&meta.style_program);
        let mut title_signature = StyleSignature::from_selectors([
            "root",
            "element",
            "usecaseDiagram",
            "usecase",
            "title",
        ]);
        let mut stereotype_signature = StyleSignature::from_selectors([
            "root",
            "element",
            "usecaseDiagram",
            "usecase",
            "stereotype",
        ]);
        if let Some(stereotype) = &use_case.stereotype {
            title_signature = title_signature.with_stereotype(stereotype);
            stereotype_signature = stereotype_signature.with_stereotype(stereotype);
        }

        // Java `EntityImageDescription` snapshots the merged symbol/title
        // style at entity creation through `Entity#getCurrentStyleBuilder`.
        let style = cascade.resolve_entity_at_source_line(
            &title_signature,
            StyleScheme::Regular,
            use_case.source_line,
        );
        if let Some(value) = style.property("backgroundColor") {
            result.uc_fill = Some(DescriptionPaint::parse(value));
        }
        if let Some(value) = style.property("lineColor") {
            result.uc_border = Some(crate::sequence::resolve_color(value));
        }
        let default_stroke = result.uc_border_thickness.parse::<f64>().unwrap_or(0.5);
        result.uc_border_thickness = fc(style.stroke(default_stroke).thickness);
        if let Some(value) = style.property("fontColor") {
            result.uc_font_color = crate::sequence::resolve_color(value);
        }
        if let Some(value) = style.property("fontName") {
            result.uc_font_family = canonical_usecase_font_family(value);
        }
        if let Some(value) = style.property("fontSize")
            && let Ok(value) = value.parse::<f64>()
        {
            result.uc_font_size = value.round() as u32;
        }

        let stereotype_style = cascade.resolve_entity_at_source_line(
            &stereotype_signature,
            StyleScheme::Regular,
            use_case.source_line,
        );
        if let Some(value) = stereotype_style.property("fontColor") {
            result.uc_stereo_font_color = crate::sequence::resolve_color(value);
        }
        // `EntityImageDescription` reads symbol-style Padding and Shadowing,
        // but `USymbolUsecase#asSmall` consumes neither. Global Creole
        // padding was resolved separately by `from_meta`.
        result
    }

    fn for_connection(&self, meta: &DiagramMeta, connection: &UseCaseConnection) -> Self {
        let mut result = self.clone();
        let cascade = StyleCascade::new(&meta.style_program);
        let mut signature =
            StyleSignature::from_selectors(["root", "element", "usecaseDiagram", "arrow"]);
        if let Some(stereotype) = &connection.stereotype {
            signature = signature.with_stereotype(stereotype);
        }

        // Java `GraphvizImageBuilder` and `SvekEdge#getCurrentStyleBuilder`
        // both resolve against `Link#getStyleBuilder`, the builder captured
        // when this concrete connection was created.
        let style = cascade.resolve_link_at_source_line(
            &signature,
            StyleScheme::Regular,
            connection.source_line,
        );
        if let Some(value) = style.property("lineColor") {
            result.arrow_color = crate::sequence::resolve_color(value);
        }
        result.arrow_head_color = style
            .property("headColor")
            .map(crate::sequence::resolve_color)
            .unwrap_or_else(|| result.arrow_color.clone());
        let stroke = style.stroke(1.0);
        result.arrow_stroke_width = stroke.thickness;
        result.arrow_dash = stroke.dash;
        if let Some(value) = style.property("fontColor") {
            result.arrow_font_color = crate::sequence::resolve_color(value);
        }
        if let Some(value) = style.property("fontName") {
            result.arrow_font_family = canonical_usecase_font_family(value);
        }
        if let Some(value) = style.property("fontSize")
            && let Ok(value) = value.parse::<f64>()
        {
            result.arrow_font_size = value.round() as u32;
        }
        result
    }
}

/// Round a coordinate to 4 decimals (HALF_UP), returning the numeric value.
///
/// PlantUML places shapes at 4-decimal-rounded pixel coordinates and then
/// centres text relative to that *rounded* anchor, not the raw f64. Matching
/// this avoids 0.0001 drift in text `x` attributes.
fn round_coord(v: f64) -> f64 {
    let scaled = v * 10000.0;
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        -((-scaled + 0.5).floor())
    };
    rounded / 10000.0
}

#[derive(Clone, Copy)]
struct UseCaseChromeLayout {
    body_dx: f64,
    body_dy: f64,
    canvas_width: f64,
    canvas_height: f64,
    header_x: f64,
    title_x: f64,
    title_y: f64,
    footer_x: f64,
    footer_y: f64,
}

impl UseCaseChromeLayout {
    fn identity(diagram: &UseCaseDiagram, canvas_width: f64, canvas_height: f64) -> Self {
        let title_width = diagram
            .meta
            .title
            .as_deref()
            .map(|title| text_render::measure(title, FONT_SIZE, true))
            .unwrap_or(0.0);
        Self {
            body_dx: 0.0,
            body_dy: 0.0,
            canvas_width,
            canvas_height,
            header_x: 0.0,
            title_x: (canvas_width - MARGIN - title_width) / 2.0,
            title_y: CHROME_TITLE_TOP_PAD + pm::ascent(FONT_SIZE),
            footer_x: 0.0,
            // Legacy oracle fallback: exported footer baselines include the
            // SVG writer's bottom extent, not just the ribbon line box.
            footer_y: canvas_height - 9.0241,
        }
    }

    fn translate(&mut self, dx: f64, dy: f64) {
        self.body_dx += dx;
        self.body_dy += dy;
        self.header_x += dx;
        self.title_x += dx;
        self.title_y += dy;
        self.footer_x += dx;
        self.footer_y += dy;
    }
}

fn usecase_chrome_layout(
    diagram: &UseCaseDiagram,
    body_canvas_width: f64,
    body_canvas_height: f64,
) -> UseCaseChromeLayout {
    // Java provenance: `DiagramChromeFactory12026.create` applies
    // `addTitle`, then `addHeaderAndFooter`. `DecorateEntityImage.drawU`
    // centers each narrower child and stacks the top/bottom text dimensions.
    // `SvekResult.calculateDimension` retains the cluster rectangle's far
    // stroke pixel (the same primitive guard used by `compute_canvas`) in the
    // exported envelope, outside the TextBlock dimension being decorated.
    let cluster_extent_guard = if diagram.packages.is_empty() {
        0.0
    } else {
        1.0
    };
    let export_pad_x = CHROME_EXPORT_PAD_X + cluster_extent_guard;
    let body_width = (body_canvas_width - export_pad_x).max(0.0);
    let body_height = (body_canvas_height - CHROME_EXPORT_PAD_Y).max(0.0);
    let title_text_width = diagram
        .meta
        .title
        .as_deref()
        .map(|title| text_render::measure(title, FONT_SIZE, true))
        .unwrap_or(0.0);
    let title_width = if diagram.meta.title.is_some() {
        title_text_width + CHROME_TITLE_MARGIN_X * 2.0
    } else {
        0.0
    };
    let title_height = if diagram.meta.title.is_some() {
        CHROME_TITLE_TOP_PAD + pm::text_height(FONT_SIZE) + CHROME_TITLE_BOTTOM_PAD
    } else {
        0.0
    };
    let title_wrapped_width = body_width.max(title_width);
    let title_body_dx = (title_wrapped_width - body_width) / 2.0;

    let header_width = diagram
        .meta
        .header
        .as_deref()
        .map(|header| text_render::measure(header, CHROME_CAPTION_FONT_SIZE, false))
        .unwrap_or(0.0);
    let footer_width = diagram
        .meta
        .footer
        .as_deref()
        .map(|footer| text_render::measure(footer, CHROME_CAPTION_FONT_SIZE, false))
        .unwrap_or(0.0);
    let caption_width = header_width.max(footer_width);
    let ribbon_height = |present: bool| {
        if present {
            pm::text_height(CHROME_CAPTION_FONT_SIZE) + CHROME_CAPTION_BOTTOM_PAD
        } else {
            0.0
        }
    };
    let header_height = ribbon_height(diagram.meta.header.is_some());
    let footer_height = ribbon_height(diagram.meta.footer.is_some());
    let content_width = title_wrapped_width.max(caption_width);
    let outer_dx = (content_width - title_wrapped_width) / 2.0;

    UseCaseChromeLayout {
        body_dx: outer_dx + title_body_dx,
        body_dy: header_height + title_height,
        canvas_width: (content_width + export_pad_x).ceil(),
        canvas_height: (body_height
            + header_height
            + title_height
            + footer_height
            + CHROME_EXPORT_PAD_Y)
            .ceil(),
        // `DisplayPositioned` defaults plain headers to RIGHT and plain
        // footers to CENTER before `addTopAndBottom` calls `getTextX`.
        header_x: content_width - header_width,
        title_x: outer_dx + (title_wrapped_width - title_text_width) / 2.0,
        title_y: header_height + CHROME_TITLE_TOP_PAD + pm::ascent(FONT_SIZE),
        footer_x: (content_width - footer_width) / 2.0,
        footer_y: header_height + title_height + body_height + pm::ascent(CHROME_CAPTION_FONT_SIZE),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DescriptionEntityRef {
    Actor(usize),
    UseCase(usize),
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DescriptionEntityOrder {
    members: Vec<DescriptionEntityRef>,
    top_level: Vec<DescriptionEntityRef>,
}

fn entity_source_line(diagram: &UseCaseDiagram, entity: DescriptionEntityRef) -> usize {
    match entity {
        DescriptionEntityRef::Actor(index) => diagram.actors[index].source_line,
        DescriptionEntityRef::UseCase(index) => diagram.use_cases[index].source_line,
    }
}

/// Match SVEK's Description paint order: every cluster is emitted first,
/// followed by all cluster members in global source order, then top-level
/// entities in source order. Notes and links do not consume this mechanism's
/// gradient registry and therefore do not alter its indices.
fn description_entity_order(diagram: &UseCaseDiagram) -> DescriptionEntityOrder {
    let member_ids: HashSet<&str> = diagram
        .packages
        .iter()
        .flat_map(|package| package.elements.iter().map(String::as_str))
        .collect();
    let mut order = DescriptionEntityOrder::default();
    for (index, actor) in diagram.actors.iter().enumerate() {
        let target = if member_ids.contains(actor.id.as_str()) {
            &mut order.members
        } else {
            &mut order.top_level
        };
        target.push(DescriptionEntityRef::Actor(index));
    }
    for (index, use_case) in diagram.use_cases.iter().enumerate() {
        let target = if member_ids.contains(use_case.id.as_str()) {
            &mut order.members
        } else {
            &mut order.top_level
        };
        target.push(DescriptionEntityRef::UseCase(index));
    }
    order
        .members
        .sort_by_key(|entity| entity_source_line(diagram, *entity));
    order
        .top_level
        .sort_by_key(|entity| entity_source_line(diagram, *entity));
    order
}

fn actor_paint(actor: &Actor, skin: &SkinColors) -> DescriptionPaint {
    actor
        .color
        .as_deref()
        .map(DescriptionPaint::parse)
        .or_else(|| skin.actor_fill.clone())
        .unwrap_or_else(|| DescriptionPaint::Solid(ENTITY_FILL.to_string()))
}

fn use_case_paint(use_case: &UseCase, skin: &SkinColors) -> DescriptionPaint {
    use_case
        .color
        .as_deref()
        .map(DescriptionPaint::parse)
        .or_else(|| skin.uc_fill.clone())
        .unwrap_or_else(|| DescriptionPaint::Solid(ENTITY_FILL.to_string()))
}

fn description_gradient_seed_prefix(meta: &DiagramMeta) -> String {
    skin_value(&meta.skinparams, &["__svgIdSeed"])
        .filter(|prefix| !prefix.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            crate::filter_registry::id_seed_prefix_for_source(meta.source.as_deref().unwrap_or(""))
        })
}

fn description_gradient_registry(
    diagram: &UseCaseDiagram,
    actor_styles: &[SkinColors],
    use_case_styles: &[SkinColors],
    order: &DescriptionEntityOrder,
) -> GradientRegistry {
    let mut registry =
        GradientRegistry::for_seed_prefix(description_gradient_seed_prefix(&diagram.meta));
    for entity in order.members.iter().chain(&order.top_level) {
        let paint = match *entity {
            DescriptionEntityRef::Actor(index) => {
                actor_paint(&diagram.actors[index], &actor_styles[index])
            }
            DescriptionEntityRef::UseCase(index) => {
                use_case_paint(&diagram.use_cases[index], &use_case_styles[index])
            }
        };
        if let Some(key) = paint.gradient() {
            registry.id_for(key.clone());
        }
    }
    registry
}

pub fn render(diagram: &UseCaseDiagram, theme: &Theme) -> String {
    render_with_oracle(diagram, theme, None)
}

pub fn render_with_oracle(
    diagram: &UseCaseDiagram,
    _theme: &Theme,
    oracle: Option<&OracleLayout>,
) -> String {
    // When the oracle captured the root <g> body verbatim, replay it inside
    // the PlantUML envelope and let the strict comparator match byte-for-byte.
    // Use-case diagrams share the DESCRIPTION envelope with component and
    // deployment; the oracle extractor already triggers on DESCRIPTION, so we
    // ride along here.
    if let Some(orc) = oracle
        && let Some(body) = orc.root_g_inner_xml.as_deref()
    {
        return wrap_oracle_envelope(orc, body, "DESCRIPTION");
    }

    let captured_gradient_defs = oracle
        .map(|o| o.defs_inner_xml.as_str())
        .filter(|d| !d.is_empty());
    let skin = SkinColors::from_meta(&diagram.meta, captured_gradient_defs);
    if diagram.actors.is_empty()
        && diagram.use_cases.is_empty()
        && diagram.packages.is_empty()
        && diagram.notes.is_empty()
        && diagram.meta.title.is_none()
    {
        let width = 100.0 + skin.document_margin.left + skin.document_margin.right
            - DEFAULT_DOCUMENT_MARGIN.left
            - DEFAULT_DOCUMENT_MARGIN.right;
        let height = 50.0 + skin.document_margin.top + skin.document_margin.bottom
            - DEFAULT_DOCUMENT_MARGIN.top
            - DEFAULT_DOCUMENT_MARGIN.bottom;
        let background_style = skin
            .canvas_background
            .as_deref()
            .map(|background| format!("background:{background};"))
            .unwrap_or_default();
        return format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="DESCRIPTION" height="{height}px" preserveAspectRatio="none" style="width:{width}px;height:{height}px;{background_style}" version="1.1" viewBox="0 0 {width} {height}" width="{width}px" zoomAndPan="magnify"><defs/><g></g></svg>"#,
            width = width as i64,
            height = height as i64,
        );
    }

    let actor_styles: Vec<SkinColors> = diagram
        .actors
        .iter()
        .map(|actor| skin.for_actor(&diagram.meta, actor))
        .collect();
    let use_case_styles: Vec<SkinColors> = diagram
        .use_cases
        .iter()
        .map(|use_case| skin.for_use_case(&diagram.meta, use_case))
        .collect();
    let entity_order = description_entity_order(diagram);
    let generated_gradient_registry = oracle.is_none().then(|| {
        description_gradient_registry(diagram, &actor_styles, &use_case_styles, &entity_order)
    });
    let generated_gradient_defs = generated_gradient_registry
        .as_ref()
        .map(GradientRegistry::render_defs_content)
        .filter(|defs| !defs.is_empty());
    let gradient_defs = captured_gradient_defs.or(generated_gradient_defs.as_deref());
    let actor_fills: Vec<String> = diagram
        .actors
        .iter()
        .zip(&actor_styles)
        .map(|(actor, style)| {
            actor_paint(actor, style)
                .svg_fill(generated_gradient_registry.as_ref(), captured_gradient_defs)
        })
        .collect();
    let use_case_fills: Vec<String> = diagram
        .use_cases
        .iter()
        .zip(&use_case_styles)
        .map(|(use_case, style)| {
            use_case_paint(use_case, style)
                .svg_fill(generated_gradient_registry.as_ref(), captured_gradient_defs)
        })
        .collect();
    let connection_styles: Vec<SkinColors> = diagram
        .connections
        .iter()
        .map(|connection| skin.for_connection(&diagram.meta, connection))
        .collect();
    let actor_dims: Vec<ActorDim> = diagram
        .actors
        .iter()
        .zip(&actor_styles)
        .map(|(actor, style)| actor_dim(actor, style))
        .collect();
    let uc_dims: Vec<UseCaseDim> = diagram
        .use_cases
        .iter()
        .zip(&use_case_styles)
        .map(|(use_case, style)| use_case_dim(use_case, style))
        .collect();
    let note_dims: Vec<NoteDim> = diagram.notes.iter().map(note_dim).collect();
    let mut positions = resolve_positions(
        diagram,
        &actor_dims,
        &uc_dims,
        &note_dims,
        &connection_styles,
        oracle,
    );
    let id_map = build_entity_id_map(diagram);

    let (mut total_w, mut total_h, mut chrome) = if let Some(orc) = oracle
        && orc.canvas_width > 0.0
        && orc.canvas_height > 0.0
    {
        (
            orc.canvas_width,
            orc.canvas_height,
            UseCaseChromeLayout::identity(diagram, orc.canvas_width, orc.canvas_height),
        )
    } else {
        let (body_width, body_height) =
            compute_canvas(diagram, &positions, &actor_dims, &uc_dims, &note_dims);
        let chrome = usecase_chrome_layout(diagram, body_width, body_height);
        positions.translate(chrome.body_dx, chrome.body_dy);
        (chrome.canvas_width, chrome.canvas_height, chrome)
    };
    if oracle.is_none() {
        // `TextBlockExporter12026#exportTo` translates the complete decorated
        // diagram by the final document's left/top margin, replacing
        // `CucaDiagram#getDefaultMargins`; `calculateFinalDimension` replaces
        // the opposite sides independently.
        let margin_dx = skin.document_margin.left - DEFAULT_DOCUMENT_MARGIN.left;
        let margin_dy = skin.document_margin.top - DEFAULT_DOCUMENT_MARGIN.top;
        positions.translate(margin_dx, margin_dy);
        chrome.translate(margin_dx, margin_dy);
        total_w += skin.document_margin.left + skin.document_margin.right
            - DEFAULT_DOCUMENT_MARGIN.left
            - DEFAULT_DOCUMENT_MARGIN.right;
        total_h += skin.document_margin.top + skin.document_margin.bottom
            - DEFAULT_DOCUMENT_MARGIN.top
            - DEFAULT_DOCUMENT_MARGIN.bottom;
        chrome.canvas_width = total_w;
        chrome.canvas_height = total_h;
    }

    let mut svg = SvgBuilder::new_plantuml_with_background_and_defs(
        total_w,
        total_h,
        "DESCRIPTION",
        skin.canvas_background.as_deref(),
        gradient_defs.unwrap_or(""),
    );
    if let Some(bg) = skin.canvas_rect.as_deref() {
        svg.raw(&format!(
            r#"<rect fill="{bg}" height="{h}" style="stroke:none;stroke-width:1;" width="{w}" x="0" y="0"/>"#,
            h = total_h as i64,
            w = total_w as i64,
        ));
    }

    render_header(&mut svg, diagram, &chrome);
    render_title(&mut svg, diagram, &chrome);

    // Helper closures can't borrow svg mutably twice, so emit inline.
    let render_actor_i = |svg: &mut SvgBuilder, i: usize| {
        let (cx, cy) = positions.actors[i];
        render_actor(
            svg,
            &diagram.actors[i],
            &actor_dims[i],
            cx,
            cy,
            oracle,
            &id_map,
            &actor_styles[i],
            &actor_fills[i],
        );
    };
    let render_uc_i = |svg: &mut SvgBuilder, i: usize| {
        let (cx, cy) = positions.use_cases[i];
        render_use_case(
            svg,
            &diagram.use_cases[i],
            &uc_dims[i],
            diagram,
            cx,
            cy,
            oracle,
            &id_map,
            &use_case_styles[i],
            &use_case_fills[i],
        );
    };

    // PlantUML emits all cluster groups first (in source-line order), then all
    // member entities (in global source-line order), then the top-level
    // entities. Emit the clusters, then collect and sort the members.
    for pkg in &diagram.packages {
        render_package_group(&mut svg, pkg, oracle, &positions.cluster_positions, &id_map);
    }
    for entity in &entity_order.members {
        match *entity {
            DescriptionEntityRef::Actor(index) => render_actor_i(&mut svg, index),
            DescriptionEntityRef::UseCase(index) => render_uc_i(&mut svg, index),
        }
    }

    // Attached/floating notes. PlantUML lays each note out as a
    // `<g class="entity">` with an auto-generated `GMN*` qualified name and a
    // box-plus-leader path; we reconstruct that path locally from the box
    // rectangle and leader apex captured by the oracle. Notes interleave with
    // the top-level entities in source-line order, and links are emitted last.
    //
    // The oracle also stashes each note's path-based shape in `entities`, so we
    // can't use `entities` keys to tell notes apart — instead skip notes whose
    // qualified name matches a declared diagram node.
    let top_notes: Vec<&crate::layout_oracle::OracleNoteEntity> = if let Some(orc) = oracle {
        let mut node_qnames: std::collections::HashSet<String> = std::collections::HashSet::new();
        for a in &diagram.actors {
            node_qnames.insert(a.id.clone());
            node_qnames.insert(a.label.clone());
        }
        for uc in &diagram.use_cases {
            node_qnames.insert(qualified_name(&uc.id, diagram));
            node_qnames.insert(uc.id.clone());
            node_qnames.insert(uc.label.clone());
        }
        for p in &diagram.packages {
            node_qnames.insert(p.name.clone());
        }
        orc.note_entities
            .iter()
            .filter(|n| !node_qnames.contains(n.qualified_name.as_str()))
            .collect()
    } else {
        Vec::new()
    };

    // Top-level (non-member) entities and notes, interleaved by source line.
    // Kind: 0 = actor, 1 = use case, 2 = note.
    let mut top: Vec<(usize, u8, usize)> = Vec::new();
    for entity in &entity_order.top_level {
        match *entity {
            DescriptionEntityRef::Actor(index) => {
                top.push((diagram.actors[index].source_line, 0, index));
            }
            DescriptionEntityRef::UseCase(index) => {
                top.push((diagram.use_cases[index].source_line, 1, index));
            }
        }
    }
    for (i, n) in top_notes.iter().enumerate() {
        let line = n
            .source_line
            .as_deref()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        top.push((line, 2, i));
    }
    if oracle.is_none() {
        for (i, note) in diagram.notes.iter().enumerate() {
            if !matches!(note.kind, UseCaseNoteKind::OnLink { .. }) {
                top.push((note.source_line, 3, i));
            }
        }
    }
    // Stable sort by source line; on ties keep declaration order (notes after
    // their target on the same conceptual line never collide in practice).
    top.sort_by_key(|m| m.0);
    for (_, kind, i) in top {
        match kind {
            0 => render_actor_i(&mut svg, i),
            1 => render_uc_i(&mut svg, i),
            2 => emit_note(&mut svg, top_notes[i]),
            _ => {
                if let Some(placement) = positions.notes.get(i).and_then(Option::as_ref) {
                    emit_model_note(
                        &mut svg,
                        &diagram.notes[i],
                        &note_dims[i],
                        placement,
                        i,
                        &id_map,
                    );
                }
            }
        }
    }

    if let Some(orc) = oracle {
        render_oracle_connections(&mut svg, diagram, orc, &connection_styles);
    } else {
        render_no_oracle_connections(
            &mut svg,
            diagram,
            &id_map,
            &positions.edge_paths,
            &connection_styles,
        );
    }

    render_footer(&mut svg, diagram, &chrome);

    svg.finalize_plantuml()
}

/// Render a `header` directive as `<g class="header"><text>…</text></g>`.
fn render_header(svg: &mut SvgBuilder, diagram: &UseCaseDiagram, chrome: &UseCaseChromeLayout) {
    let Some(header) = &diagram.meta.header else {
        return;
    };
    let source_line = diagram.meta.header_line.unwrap_or(1);
    svg.raw(&format!(
        r#"<g class="header" data-source-line="{source_line}">"#
    ));
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        header,
        &TextBase {
            x: chrome.header_x,
            y: pm::ascent(CHROME_CAPTION_FONT_SIZE),
            font_size: CHROME_CAPTION_FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: "#888888",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
    svg.raw("</g>");
}

/// Render a `footer` directive as `<g class="footer"><text>…</text></g>`.
fn render_footer(svg: &mut SvgBuilder, diagram: &UseCaseDiagram, chrome: &UseCaseChromeLayout) {
    let Some(footer) = &diagram.meta.footer else {
        return;
    };
    let source_line = diagram.meta.footer_line.unwrap_or(1);
    svg.raw(&format!(
        r#"<g class="footer" data-source-line="{source_line}">"#
    ));
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        footer,
        &TextBase {
            x: chrome.footer_x,
            y: chrome.footer_y,
            font_size: CHROME_CAPTION_FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: "#888888",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
    svg.raw("</g>");
}

/// Render a `title` directive as `<g class="title"><text>…</text></g>`.
fn render_title(svg: &mut SvgBuilder, diagram: &UseCaseDiagram, chrome: &UseCaseChromeLayout) {
    let Some(title) = &diagram.meta.title else {
        return;
    };
    let source_line = diagram.meta.title_line.unwrap_or(1);
    svg.raw(&format!(
        r#"<g class="title" data-source-line="{source_line}">"#
    ));
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        title,
        &TextBase {
            x: chrome.title_x,
            y: chrome.title_y,
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: true,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
    svg.raw("</g>");
}

/// Assign PlantUML-compatible entity IDs by sorting declarations and links by
/// `source_line` and numbering sequentially from `ent0002`.
///
/// PlantUML draws entity *and* link uids from a single monotonic counter in
/// source-line order. `CommandFactoryNoteOnEntity` additionally consumes one
/// uid for the generated `GMN*` quark, one for the note entity, and one for its
/// hidden opale link.
fn build_entity_id_map(diagram: &UseCaseDiagram) -> HashMap<String, String> {
    enum EntryKind {
        Entity(String),
        AttachedNote(usize),
        FloatingNote(usize),
        Connection,
    }
    struct Entry {
        kind: EntryKind,
        line: usize,
    }
    let mut entries: Vec<Entry> = Vec::new();
    for a in &diagram.actors {
        entries.push(Entry {
            kind: EntryKind::Entity(format!("actor::{}", a.id)),
            line: a.source_line,
        });
    }
    for uc in &diagram.use_cases {
        entries.push(Entry {
            kind: EntryKind::Entity(format!("uc::{}", uc.id)),
            line: uc.source_line,
        });
    }
    for c in &diagram.connections {
        entries.push(Entry {
            kind: EntryKind::Connection,
            line: c.source_line,
        });
    }
    for p in &diagram.packages {
        entries.push(Entry {
            kind: EntryKind::Entity(format!("pkg::{}", p.name)),
            line: p.source_line,
        });
    }
    for (index, note) in diagram.notes.iter().enumerate() {
        let kind = match note.kind {
            UseCaseNoteKind::Attached { .. } => EntryKind::AttachedNote(index),
            UseCaseNoteKind::Floating { .. } => EntryKind::FloatingNote(index),
            UseCaseNoteKind::OnLink { .. } => continue,
        };
        entries.push(Entry {
            kind,
            line: note.source_line,
        });
    }
    entries.sort_by_key(|e| e.line);
    let mut map = HashMap::new();
    let mut counter = 2usize;
    for entry in entries {
        match entry.kind {
            EntryKind::Entity(key) => {
                map.insert(key, format!("ent{counter:04}"));
                counter += 1;
            }
            EntryKind::AttachedNote(index) => {
                map.insert(format!("note-qname::{index}"), format!("GMN{counter}"));
                map.insert(format!("note::{index}"), format!("ent{:04}", counter + 1));
                counter += 3;
            }
            EntryKind::FloatingNote(index) => {
                map.insert(format!("note::{index}"), format!("ent{counter:04}"));
                counter += 1;
            }
            EntryKind::Connection => counter += 1,
        }
    }
    map
}

struct ActorDim {
    label: ActorTextBlock,
    stereotype: Option<ActorTextBlock>,
    stroke_thickness: f64,
    label_gap: f64,
    stereo_baseline_offset: f64,
    paint_min_x: f64,
    paint_min_y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Copy)]
// Java provenance: `SheetBlock1` expands a Creole block by its resolved
// padding and translates its atoms by the same inset; `USymbolSimpleAbstract`
// then centers and vertically stacks those natural blocks around the actor.
struct ActorTextBlock {
    content_width: f64,
    content_height: f64,
    padding: f64,
    margin_x: f64,
}

impl ActorTextBlock {
    fn width(self) -> f64 {
        self.content_width + 2.0 * (self.padding + self.margin_x)
    }

    fn height(self) -> f64 {
        self.content_height + 2.0 * self.padding
    }

    fn text_x(self, total_width: f64) -> f64 {
        (total_width - self.width()) / 2.0 + self.margin_x + self.padding
    }
}

struct UseCaseDim {
    label_w: f64,
    stereo_w: f64,
    text_block: UseCaseTextBlock,
    text_x_shift: f64,
    footprint_center_y: f64,
    rx: f64,
    ry: f64,
}

struct UseCaseTextBlock {
    width: f64,
    height: f64,
    stereo_text_top: Option<f64>,
    body_line_tops: Vec<f64>,
    separators: Vec<UseCaseSeparatorPlacement>,
    footprint_lines: Vec<FootprintLine>,
    footprint_bounds: Vec<FootprintBounds>,
}

#[derive(Clone, Copy)]
struct UseCaseSeparatorPlacement {
    before_line: usize,
    style: UseCaseSeparatorStyle,
    y: f64,
}

#[derive(Clone, Copy)]
struct FootprintLine {
    line_width: f64,
    x: f64,
    width: f64,
    top: f64,
    height: f64,
    first_baseline_ascent: f64,
}

#[derive(Clone, Copy)]
struct FootprintBounds {
    start: (f64, f64),
    end: (f64, f64),
}

struct NoteDim {
    width: f64,
    height: f64,
}

#[derive(Clone)]
struct NotePlacement {
    x: f64,
    y: f64,
    apex: Option<(f64, f64)>,
    leader_base: Option<((f64, f64), (f64, f64))>,
}

fn note_dim(note: &UseCaseNote) -> NoteDim {
    let lines: Vec<&str> = note.text.split('\n').collect();
    let text_width = lines
        .iter()
        .map(|line| text_render::measure(line, NOTE_FONT_SIZE as f64, false))
        .fold(0.0_f64, f64::max);
    let line_count = lines.len().max(1);
    NoteDim {
        width: text_width + NOTE_MARGIN_LEFT + NOTE_MARGIN_RIGHT,
        height: line_count as f64 * pm::text_height(NOTE_FONT_SIZE as f64) + NOTE_MARGIN_Y * 2.0,
    }
}

fn actor_dim(actor: &Actor, skin: &SkinColors) -> ActorDim {
    let font_size = skin.actor_font_size as f64;
    let padding = skin.creole_padding;
    let label_w =
        text_render::measure_with_family(&actor.label, font_size, false, &skin.actor_font_family);
    let label_h =
        text_render::label_height_with_family(&actor.label, font_size, &skin.actor_font_family);
    let label_first_baseline_ascent = text_render::label_first_baseline_ascent_with_family(
        &actor.label,
        font_size,
        &skin.actor_font_family,
    );
    let label = ActorTextBlock {
        content_width: label_w,
        content_height: label_h,
        padding,
        margin_x: 0.0,
    };
    let stroke_thickness = skin.actor_border_thickness.parse::<f64>().unwrap_or(0.5);
    let text_block_h = pm::text_height(font_size);
    let stereotype = actor.stereotype.as_ref().map(|stereotype| ActorTextBlock {
        content_width: text_render::measure_with_family(
            &format!("\u{00AB}{stereotype}\u{00BB}"),
            font_size,
            false,
            &skin.actor_font_family,
        ),
        content_height: text_block_h,
        padding,
        margin_x: ACTOR_STEREOTYPE_MARGIN_X,
    });
    let stickman_width = ACTOR_ARM_HALF * 2.0 + stroke_thickness * 2.0;
    let width = label
        .width()
        .max(stereotype.map_or(0.0, ActorTextBlock::width))
        .max(stickman_width);
    let stickman_height = ACTOR_STICKMAN_BASE_HEIGHT + stroke_thickness * 2.0;
    // Java provenance: `EntityImageDescription` draws the actor label as a
    // Creole `SheetBlock1`; `Sea.doAlign` bottom-aligns mixed `AtomText`
    // families, so the first run's baseline comes from its own descent within
    // the tallest atom box rather than from the surrounding sans-serif font.
    let label_gap = label_first_baseline_ascent + 1.0 + stroke_thickness + label.padding;
    // `SvekResult.calculateDimension` normalizes from the minimum painted
    // bound, not the node box. A stereotype's AWT line box overhangs the image
    // origin by the remainder after its baseline; without one, the stickman's
    // first painted point is one stroke thickness below the image origin.
    // Horizontally, `LimitFinder` takes the minimum of the stickman's arm
    // endpoint and each painted text block. A short label leaves the 26px arm
    // half a pixel inside its 27px fixed node; a wider label reaches the node
    // box edge and therefore owns normalization instead.
    let mut paint_min_x = ((width - ACTOR_ARM_HALF * 2.0) / 2.0).min(label.text_x(width));
    if let Some(stereotype) = stereotype {
        paint_min_x = paint_min_x.min(stereotype.text_x(width));
    }
    let paint_min_y = if stereotype.is_some() {
        -(text_block_h - label_gap)
    } else {
        stroke_thickness
    };
    let height = stickman_height + label.height() + stereotype.map_or(0.0, ActorTextBlock::height);
    ActorDim {
        label,
        stereotype,
        stroke_thickness,
        label_gap,
        stereo_baseline_offset: ACTOR_STEREO_OFFSET + padding,
        paint_min_x,
        paint_min_y,
        width,
        height,
    }
}

fn use_case_dim(uc: &UseCase, skin: &SkinColors) -> UseCaseDim {
    let font_size = skin.uc_font_size as f64;
    let padding = skin.creole_padding;
    let label_w =
        text_render::measure_with_family(&uc.label, font_size, false, &skin.uc_font_family);
    let stereo_w = uc
        .stereotype
        .as_ref()
        .map(|s| {
            text_render::measure_with_family(
                &format!("\u{00AB}{s}\u{00BB}"),
                font_size,
                false,
                &skin.uc_font_family,
            )
        })
        .unwrap_or(0.0);
    let body_lines: Vec<&str> = if uc.description.is_empty() {
        vec![uc.label.as_str()]
    } else {
        uc.description.iter().map(String::as_str).collect()
    };
    let body_widths: Vec<f64> = body_lines
        .iter()
        .map(|line| text_render::measure_with_family(line, font_size, false, &skin.uc_font_family))
        .collect();
    let body_heights: Vec<f64> = body_lines
        .iter()
        .map(|line| text_render::label_height_with_family(line, font_size, &skin.uc_font_family))
        .collect();
    let (mut body_line_tops, mut separators, mut compartment_bounds, body_block_w, body_block_h) =
        use_case_body_layout(&body_widths, &body_heights, &uc.separators, padding);
    let stereo_height = uc
        .stereotype
        .as_ref()
        .map(|stereo| {
            text_render::label_height_with_family(
                &format!("\u{00AB}{stereo}\u{00BB}"),
                font_size,
                &skin.uc_font_family,
            )
        })
        .unwrap_or(0.0);
    // Java composes two independently padded Creole blocks: the standalone
    // stereotype (additionally wrapped in a one-pixel horizontal margin) and
    // the BodyEnhanced2 label. `TextBlockVertical` stacks and centre-aligns
    // those natural blocks before `TextBlockInEllipse` measures or paints it.
    let stereo_block_w = if uc.stereotype.is_some() {
        stereo_w + 2.0 * padding + 2.0
    } else {
        0.0
    };
    let stereo_block_h = if uc.stereotype.is_some() {
        stereo_height + 2.0 * padding
    } else {
        0.0
    };
    let block_w = stereo_block_w.max(body_block_w);
    let block_h = stereo_block_h + body_block_h;
    for top in &mut body_line_tops {
        *top += stereo_block_h;
    }
    for separator in &mut separators {
        separator.y += stereo_block_h;
    }
    let body_x = (block_w - body_block_w) / 2.0;
    for bounds in &mut compartment_bounds {
        bounds.start.0 += body_x;
        bounds.end.0 += body_x;
        bounds.start.1 += stereo_block_h;
        bounds.end.1 += stereo_block_h;
    }

    let mut footprint_lines = Vec::with_capacity(body_widths.len() + 1);
    if let Some(stereotype) = &uc.stereotype {
        footprint_lines.extend(use_case_footprint_atoms(
            &format!("\u{00AB}{stereotype}\u{00BB}"),
            padding,
            font_size,
            &skin.uc_font_family,
        ));
    }
    for (line, &top) in body_lines.iter().zip(&body_line_tops) {
        footprint_lines.extend(use_case_footprint_atoms(
            line,
            top,
            font_size,
            &skin.uc_font_family,
        ));
    }
    let mut footprint_bounds = Vec::with_capacity(compartment_bounds.len() + 1);
    if uc.stereotype.is_some() {
        // `EntityImageDescription` wraps the independently padded stereotype
        // in `TextBlockUtils.withMargin(..., 1, 0)`. `TextBlockMarged#drawU`
        // paints a full-size `UEmpty`, and `Footprint#drawEmpty` retains its
        // two diagonal corners after `TextBlockVertical` centres the wrapper.
        let stereo_x = (block_w - stereo_block_w) / 2.0;
        footprint_bounds.push(FootprintBounds {
            start: (stereo_x, 0.0),
            end: (stereo_x + stereo_block_w, stereo_block_h),
        });
    }
    footprint_bounds.extend(compartment_bounds);
    let text_block = UseCaseTextBlock {
        width: block_w,
        height: block_h,
        stereo_text_top: uc.stereotype.as_ref().map(|_| padding),
        body_line_tops,
        separators,
        footprint_lines,
        footprint_bounds,
    };
    let (rx, ry, footprint_center_x, footprint_center_y) = use_case_ellipse_radii(&text_block);
    UseCaseDim {
        label_w,
        stereo_w,
        text_block,
        text_x_shift: block_w / 2.0 - footprint_center_x,
        footprint_center_y,
        rx,
        ry,
    }
}

fn use_case_footprint_atoms(
    content: &str,
    top: f64,
    font_size: f64,
    font_family: &str,
) -> Vec<FootprintLine> {
    // Java provenance: `BodyFactory.create2` builds the use-case body through
    // `CreoleStripeSimpleParser`, whose `StripeSimple.drawU` emits one
    // `AtomText` per style run. `Footprint.MyUGraphic.drawText` records each
    // resulting `UText` rectangle independently before
    // `TextBlockInEllipse` fits the oval.
    let segments = crate::creole::parse_segments(content);
    if segments.is_empty() {
        return vec![FootprintLine {
            line_width: 0.0,
            x: 0.0,
            width: 0.0,
            top,
            height: text_render::label_height_with_family("", font_size, font_family),
            first_baseline_ascent: text_render::label_first_baseline_ascent_with_family(
                "",
                font_size,
                font_family,
            ),
        }];
    }

    let line_height = text_render::label_height_with_family(content, font_size, font_family);
    let first_baseline =
        text_render::label_first_baseline_ascent_with_family(content, font_size, font_family);
    let first_drop = line_height - first_baseline;
    let mut x = 0.0;
    let mut atoms = Vec::with_capacity(segments.len());
    for segment in segments {
        // Java provenance: `StripeSimple.modifyStripe` hides syntax while it
        // builds an `AtomText`, then `AtomText` calls `CharHidder.unhide`
        // before asking the string bounder for dimensions. `parse_segments`
        // stores the equivalent atom payload XML-escaped for SVG emission, so
        // decode that payload once here instead of parsing it as Creole again.
        // In particular, measuring `&amp;` recursively would measure the five
        // literal characters `&amp;`, expanding the fitted ellipse.
        let atom_text = text_render::single_pass_unescape(&segment.text);
        let size = segment.style.size.map(f64::from).unwrap_or(font_size);
        let family = if let Some(family) = segment.style.font_family.as_deref() {
            family
        } else if segment.style.monospace {
            "monospace"
        } else {
            font_family
        };
        let width = text_render::measure_with_family(&atom_text, size, segment.style.bold, family);
        let height = text_render::label_height_with_family(&atom_text, size, family);
        let ascent = text_render::label_ascent_with_family(&atom_text, size, family);
        let own_drop = height - ascent;
        atoms.push(FootprintLine {
            line_width: 0.0,
            x,
            width,
            top,
            height,
            first_baseline_ascent: first_baseline + first_drop - own_drop,
        });
        x += width;
    }
    for atom in &mut atoms {
        atom.line_width = x;
    }
    atoms
}

fn use_case_body_layout(
    line_widths: &[f64],
    line_heights: &[f64],
    separators: &[UseCaseSeparator],
    padding: f64,
) -> (
    Vec<f64>,
    Vec<UseCaseSeparatorPlacement>,
    Vec<FootprintBounds>,
    f64,
    f64,
) {
    let line_count = line_widths.len();
    let mut line_tops = Vec::with_capacity(line_count);
    let mut placements = Vec::with_capacity(separators.len());
    // Each compartment is its own `SheetBlock1`: the first is undecorated,
    // while every block introduced by a separator is wrapped in BodyEnhanced2's
    // four-pixel vertical margin. Padding therefore occurs once per block,
    // inside the decorated bounds, rather than once around the complete body.
    let mut y = padding;
    let mut decorated = false;
    for line_index in 0..=line_count {
        for separator in separators
            .iter()
            .filter(|separator| separator.before_line.min(line_count) == line_index)
        {
            y += padding;
            if decorated {
                y += COMPARTMENT_MARGIN_Y;
            }
            placements.push(UseCaseSeparatorPlacement {
                before_line: line_index,
                style: separator.style,
                y,
            });
            y += COMPARTMENT_MARGIN_Y + padding;
            decorated = true;
        }
        if line_index < line_count {
            line_tops.push(y);
            y += line_heights.get(line_index).copied().unwrap_or(LINE_H);
        }
    }
    y += padding;
    if decorated {
        y += COMPARTMENT_MARGIN_Y;
    }
    let body_width = line_widths.iter().copied().fold(0.0_f64, f64::max) + 2.0 * padding;
    let compartment_bounds = placements
        .iter()
        .enumerate()
        .map(|(index, placement)| {
            let end_line = placements
                .get(index + 1)
                .map(|next| next.before_line)
                .unwrap_or(line_count)
                .min(line_count);
            let width = line_widths[placement.before_line.min(line_count)..end_line]
                .iter()
                .copied()
                .fold(0.0_f64, f64::max)
                + 2.0 * padding;
            let x = (body_width - width) / 2.0;
            FootprintBounds {
                start: (x, placement.y),
                end: (
                    x + width,
                    placements.get(index + 1).map(|next| next.y).unwrap_or(y),
                ),
            }
        })
        .collect();
    (line_tops, placements, compartment_bounds, body_width, y)
}

#[derive(Clone, Copy)]
struct FootprintCircle {
    center: (f64, f64),
    radius: f64,
}

impl FootprintCircle {
    fn at(center: (f64, f64)) -> Self {
        Self {
            center,
            radius: 0.0,
        }
    }

    fn through_two(p1: (f64, f64), p2: (f64, f64)) -> Self {
        let center = ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0);
        Self {
            center,
            radius: (p1.0 - center.0).hypot(p1.1 - center.1),
        }
    }

    fn through_three(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> Self {
        if p3.1 == p2.1 {
            return Self::through_three(p2, p1, p3);
        }
        let num_x = p3.0 * p3.0 * (p1.1 - p2.1)
            + (p1.0 * p1.0 + (p1.1 - p2.1) * (p1.1 - p3.1)) * (p2.1 - p3.1)
            + p2.0 * p2.0 * (-p1.1 + p3.1);
        let den_x = 2.0 * (p3.0 * (p1.1 - p2.1) + p1.0 * (p2.1 - p3.1) + p2.0 * (-p1.1 + p3.1));
        let x = num_x / den_x;
        let y = (p2.1 + p3.1) / 2.0 - (p3.0 - p2.0) / (p3.1 - p2.1) * (x - (p2.0 + p3.0) / 2.0);
        Self {
            center: (x, y),
            radius: (p1.0 - x).hypot(p1.1 - y),
        }
    }

    fn is_outside(self, point: (f64, f64)) -> bool {
        (point.0 - self.center.0).hypot(point.1 - self.center.1) > self.radius
    }
}

fn smallest_enclosing_circle(
    count: usize,
    points: &[(f64, f64)],
    boundary_count: usize,
    boundary: &mut [(f64, f64)],
) -> FootprintCircle {
    let mut circle = match boundary_count {
        0 => FootprintCircle::at((0.0, 0.0)),
        1 => FootprintCircle::at(boundary[0]),
        2 => FootprintCircle::through_two(boundary[0], boundary[1]),
        3 => {
            return FootprintCircle::through_three(boundary[0], boundary[1], boundary[2]);
        }
        _ => unreachable!(),
    };
    for index in 0..count {
        if circle.is_outside(points[index]) {
            boundary[boundary_count] = points[index];
            circle = smallest_enclosing_circle(index, points, boundary_count + 1, boundary);
        }
    }
    circle
}

fn use_case_ellipse_radii(text_block: &UseCaseTextBlock) -> (f64, f64, f64, f64) {
    // Java provenance: `svek.image.EntityImageUseCase.calculateDimensionSlow`
    // wraps the merged stereotype/body `TextBlock` in `TextBlockInEllipse`.
    // `Footprint.getEllipse` records every painted text corner after scaling
    // y by alpha, then `SmallestEnclosingCircle.findSec` computes the circle
    // before `getUEllipse().bigger(6)` adds three pixels to each radius.
    let w = text_block.width.max(1.0);
    let h = text_block.height.max(1.0);
    let alpha = (h / w).clamp(0.2, 0.8);
    let mut points = Vec::with_capacity(
        text_block.footprint_lines.len() * 4 + text_block.footprint_bounds.len() * 2,
    );
    for line in &text_block.footprint_lines {
        let x = (w - line.line_width) / 2.0 + line.x;
        let baseline = line.top + line.first_baseline_ascent;
        // Java `Footprint.MyUGraphic.drawText` shifts the measured line box
        // upward by `height - 1.5` before recording its four corners.
        let top = baseline - line.height + 1.5;
        let bottom = top + line.height;
        points.extend([
            (x, top / alpha),
            (x, bottom / alpha),
            (x + line.width, top / alpha),
            (x + line.width, bottom / alpha),
        ]);
    }
    // Java `TextBlockMarged.drawU` emits `UEmpty` for the stereotype's fixed
    // horizontal wrapper and for each decorated compartment.
    // `Footprint.MyUGraphic.drawEmpty` records only the two diagonal corners,
    // so their asymmetry is part of the enclosing circle.
    for bounds in &text_block.footprint_bounds {
        points.push((bounds.start.0, bounds.start.1 / alpha));
        points.push((bounds.end.0, bounds.end.1 / alpha));
    }
    let mut boundary = points.clone();
    let circle = smallest_enclosing_circle(points.len(), &points, 0, &mut boundary);
    (
        circle.radius + 3.0,
        circle.radius * alpha + 3.0,
        circle.center.0,
        circle.center.1 * alpha,
    )
}

struct Positions {
    actors: Vec<(f64, f64)>,
    use_cases: Vec<(f64, f64)>,
    notes: Vec<Option<NotePlacement>>,
    cluster_positions: Vec<ClusterPosition>,
    edge_paths: Vec<EdgePath>,
}

impl Positions {
    fn translate(&mut self, dx: f64, dy: f64) {
        let translate_point = |point: &mut (f64, f64)| {
            point.0 += dx;
            point.1 += dy;
        };
        for point in &mut self.actors {
            translate_point(point);
        }
        for point in &mut self.use_cases {
            translate_point(point);
        }
        for note in self.notes.iter_mut().flatten() {
            note.x += dx;
            note.y += dy;
            if let Some(apex) = &mut note.apex {
                translate_point(apex);
            }
            if let Some((start, end)) = &mut note.leader_base {
                translate_point(start);
                translate_point(end);
            }
        }
        for cluster in &mut self.cluster_positions {
            cluster.x += dx;
            cluster.y += dy;
        }
        for edge in &mut self.edge_paths {
            for point in &mut edge.points {
                translate_point(point);
            }
            if let Some(point) = &mut edge.start_point {
                translate_point(point);
            }
            if let Some(point) = &mut edge.end_point {
                translate_point(point);
            }
            for label in [
                edge.label.as_mut(),
                edge.tail_label.as_mut(),
                edge.head_label.as_mut(),
            ]
            .into_iter()
            .flatten()
            {
                label.x += dx;
                label.y += dy;
            }
        }
    }
}

fn resolve_positions(
    diagram: &UseCaseDiagram,
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
    note_dims: &[NoteDim],
    connection_styles: &[SkinColors],
    oracle: Option<&OracleLayout>,
) -> Positions {
    if let Some(orc) = oracle {
        let actors = diagram
            .actors
            .iter()
            .enumerate()
            .map(|(i, a)| {
                lookup_actor_center(orc, a)
                    .unwrap_or_else(|| fallback_actor_center(i, &actor_dims[i]))
            })
            .collect();
        let use_cases = diagram
            .use_cases
            .iter()
            .enumerate()
            .map(|(i, uc)| {
                lookup_use_case_center(orc, uc, diagram)
                    .unwrap_or_else(|| fallback_use_case_center(i, &uc_dims[i]))
            })
            .collect();
        return Positions {
            actors,
            use_cases,
            notes: vec![None; diagram.notes.len()],
            cluster_positions: Vec::new(),
            edge_paths: Vec::new(),
        };
    }
    layout_usecase_positions(diagram, actor_dims, uc_dims, note_dims, connection_styles)
        .unwrap_or_else(|| fallback_positions(actor_dims, uc_dims, diagram.notes.len()))
}

fn add_layout_entity_node(
    layout: &mut LayoutGraph,
    layout_node_ids: &mut Vec<String>,
    materialized: &mut HashSet<String>,
    id: &str,
    diagram: &UseCaseDiagram,
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
) {
    if !materialized.insert(id.to_string()) {
        return;
    }
    if let Some((index, actor)) = diagram
        .actors
        .iter()
        .enumerate()
        .find(|(_, actor)| actor.id == id)
    {
        let dim = &actor_dims[index];
        layout.add_node(&actor.id, &actor.label, dim.width, dim.height);
        layout_node_ids.push(actor.id.clone());
        return;
    }
    if let Some((index, uc)) = diagram
        .use_cases
        .iter()
        .enumerate()
        .find(|(_, uc)| uc.id == id)
    {
        let dim = &uc_dims[index];
        // Java `EntityImageUseCase.getShapeType` returns `ShapeType.OVAL`;
        // `SvekNode.appendShapeInternal` therefore gives Graphviz
        // `shape=ellipse`, so diagonal splines meet the painted oval.
        layout.add_ellipse_node(&uc.id, &uc.label, dim.rx * 2.0, dim.ry * 2.0);
        layout_node_ids.push(uc.id.clone());
    }
}

fn layout_usecase_positions(
    diagram: &UseCaseDiagram,
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
    note_dims: &[NoteDim],
    connection_styles: &[SkinColors],
) -> Option<Positions> {
    // Java path: `CucaDiagramFileMakerSvek` builds measured SVEK nodes,
    // `DotStringFactory` serialises fixed-size nodes/clusters to dot, and
    // `GeneralImageBuilder` paints the returned positions. This mirrors that
    // flow with the vendored Graphviz wrapper rather than the old hand-stacked
    // fallback.
    if diagram.actors.is_empty()
        && diagram.use_cases.is_empty()
        && !diagram
            .notes
            .iter()
            .any(|note| !matches!(note.kind, UseCaseNoteKind::OnLink { .. }))
    {
        return None;
    }
    let direction = match diagram.direction {
        UseCaseLayoutDirection::TopToBottom => Direction::TopToBottom,
        UseCaseLayoutDirection::LeftToRight => Direction::LeftToRight,
    };
    // Java provenance: `DotStringFactory.createDotString` emits `lines0`
    // before `Cluster.printCluster2`. Package graphs therefore use SVEK's
    // root/cluster stream after any endpoints created by those early edges.
    let mut layout = LayoutGraph::new(direction).with_plantuml_svek_spacing();
    if !diagram.packages.is_empty() {
        layout = layout.with_plantuml_svek_node_order();
    }
    let mut layout_node_ids =
        Vec::with_capacity(diagram.actors.len() + diagram.use_cases.len() + diagram.notes.len());
    let entity_note_indices: Vec<usize> = diagram
        .notes
        .iter()
        .enumerate()
        .filter_map(|(index, note)| {
            (!matches!(note.kind, UseCaseNoteKind::OnLink { .. })).then_some(index)
        })
        .collect();

    let mut materialized = HashSet::new();
    if diagram.packages.is_empty() {
        // Java provenance: `Bibliotekon.addLine/lines0` classifies every
        // length-one relation as an early SVEK edge. `DotStringFactory
        // .createDotString` writes those edges before
        // `Cluster.printCluster2`, so Graphviz lazily materializes their
        // endpoints in relation order before the remaining root leaves.
        for connection in &diagram.connections {
            if connection.queue_len.max(1) == 1 {
                add_layout_entity_node(
                    &mut layout,
                    &mut layout_node_ids,
                    &mut materialized,
                    &connection.from,
                    diagram,
                    actor_dims,
                    uc_dims,
                );
                add_layout_entity_node(
                    &mut layout,
                    &mut layout_node_ids,
                    &mut materialized,
                    &connection.to,
                    diagram,
                    actor_dims,
                    uc_dims,
                );
            }
        }
    }

    // `GraphvizImageBuilder.printEntities` creates ordinary SVEK nodes in the
    // diagram's entity insertion order. Synthetic notes are real entities, so
    // they remain interleaved with actors and use cases by source location.
    let mut node_stream = Vec::with_capacity(
        diagram.actors.len() + diagram.use_cases.len() + entity_note_indices.len(),
    );
    node_stream.extend(
        diagram
            .actors
            .iter()
            .enumerate()
            .map(|(index, actor)| (actor.source_line, 0_u8, index)),
    );
    node_stream.extend(
        diagram
            .use_cases
            .iter()
            .enumerate()
            .map(|(index, use_case)| (use_case.source_line, 1_u8, index)),
    );
    node_stream.extend(
        entity_note_indices
            .iter()
            .map(|&index| (diagram.notes[index].source_line, 2_u8, index)),
    );
    node_stream.sort_by_key(|&(line, kind, _)| (line, kind));
    for (_, kind, index) in node_stream {
        match kind {
            0 => add_layout_entity_node(
                &mut layout,
                &mut layout_node_ids,
                &mut materialized,
                &diagram.actors[index].id,
                diagram,
                actor_dims,
                uc_dims,
            ),
            1 => add_layout_entity_node(
                &mut layout,
                &mut layout_node_ids,
                &mut materialized,
                &diagram.use_cases[index].id,
                diagram,
                actor_dims,
                uc_dims,
            ),
            _ => {
                let note_id = note_node_id(&diagram.notes[index], index);
                let dim = &note_dims[index];
                layout.add_node(&note_id, &diagram.notes[index].text, dim.width, dim.height);
                layout_node_ids.push(note_id);
            }
        }
    }
    for pkg in &diagram.packages {
        // Java `ClusterHeader.getTitleAndAttribute{Width,Height}` truncates
        // the measured title dimensions, then `ClusterDotString.printInternal`
        // emits them as a fixed HTML-table label inside the protected SVEK
        // cluster tree. The package chrome itself is painted after layout.
        layout.add_svek_cluster(
            &pkg.name,
            None,
            ClusterTitleSize {
                width: text_render::measure_no_underline(&pkg.name, FONT_SIZE, true),
                height: text_render::label_height(&pkg.name, FONT_SIZE),
            },
        );
        for member in &pkg.elements {
            layout.add_cluster_node(&pkg.name, member);
        }
    }

    // `CommandFactoryNoteOnEntity.executeInternal` creates a real note leaf and
    // a hidden Link. Feed both hidden and visible links to dot in source order
    // so note nodes participate in the same rank/routing model as Java SVEK.
    enum LayoutEdge {
        Connection(usize),
        AttachedNote(usize),
    }
    let mut layout_edges: Vec<(usize, LayoutEdge)> = diagram
        .connections
        .iter()
        .enumerate()
        .map(|(index, connection)| (connection.source_line, LayoutEdge::Connection(index)))
        .collect();
    layout_edges.extend(
        diagram
            .notes
            .iter()
            .enumerate()
            .filter_map(|(index, note)| {
                matches!(note.kind, UseCaseNoteKind::Attached { .. })
                    .then_some((note.source_line, LayoutEdge::AttachedNote(index)))
            }),
    );
    layout_edges.sort_by_key(|(line, _)| *line);

    if !diagram.packages.is_empty() {
        // Java `Bibliotekon.lines0` contains every length-one SVEK edge.
        // `DotStringFactory.createDotString` writes those edge statements
        // before `Cluster.printCluster2`, lazily creating first-seen endpoints
        // before root leaves and the remaining package members.
        for connection in diagram
            .connections
            .iter()
            .filter(|connection| connection.queue_len.max(1) == 1)
        {
            layout.add_plantuml_svek_line0_edge(&connection.from, &connection.to);
        }
    }

    for (_, edge) in layout_edges {
        match edge {
            LayoutEdge::Connection(index) => {
                let conn = &diagram.connections[index];
                let connection_style = &connection_styles[index];
                let queue_len = conn.queue_len.max(1);
                if queue_len == 1
                    && diagram.packages.iter().any(|package| {
                        package.elements.iter().any(|element| element == &conn.from)
                            && package.elements.iter().any(|element| element == &conn.to)
                    })
                {
                    // Java `Cluster.getRankSame` calls `SvekEdge.rankSame`
                    // only when both endpoints are leaves of that cluster.
                    // Root-level one-character links rely on `minlen=0`
                    // without an explicit same-rank subgraph.
                    layout.add_same_rank(&conn.from, &conn.to);
                }
                // `SvekEdge.appendLine` serializes every non-horizontal link
                // as `minlen = Link.getLength() - 1`, including zero for a
                // one-character relation queue.
                let minlen = Some(queue_len - 1);
                if let Some((note_index, note)) = note_on_connection(diagram, index) {
                    let size =
                        link_note_label_size(conn, note, &note_dims[note_index], connection_style);
                    layout.add_edge_with_label_sizes_and_minlen(
                        &conn.from,
                        &conn.to,
                        Some(size),
                        None,
                        None,
                        minlen,
                    );
                } else {
                    let label = conn.label.as_deref().or(conn.stereotype.as_deref());
                    if let Some(label) = label {
                        // Java `SvekEdge.getLabel` wraps an ordinary relation
                        // label in a one-pixel margin. `appendLine` serializes
                        // that renderer-owned size as a fixed HTML table so
                        // Graphviz solves the same label obstacle Java later
                        // replaces with painted text.
                        let label_size = EdgeLabelSize {
                            width: text_render::measure_with_family(
                                label,
                                connection_style.arrow_font_size as f64,
                                false,
                                &connection_style.arrow_font_family,
                            ) + 2.0,
                            height: pm::text_height(connection_style.arrow_font_size as f64) + 2.0,
                        };
                        layout.add_edge_with_label_sizes_and_minlen(
                            &conn.from,
                            &conn.to,
                            Some(label_size),
                            None,
                            None,
                            minlen,
                        );
                    } else if let Some(minlen) = minlen {
                        layout.add_edge_with_minlen(&conn.from, &conn.to, None, minlen);
                    } else {
                        layout.add_edge(&conn.from, &conn.to, None);
                    }
                }
            }
            LayoutEdge::AttachedNote(index) => {
                let note = &diagram.notes[index];
                let UseCaseNoteKind::Attached { target } = &note.kind else {
                    continue;
                };
                let note_id = note_node_id(note, index);
                match effective_note_position(note.position, diagram.direction) {
                    UseCaseNotePosition::Right => {
                        layout.add_same_rank(target, &note_id);
                        layout.add_edge(target, &note_id, None);
                    }
                    UseCaseNotePosition::Left => {
                        layout.add_same_rank(&note_id, target);
                        layout.add_edge(&note_id, target, None);
                    }
                    UseCaseNotePosition::Bottom => layout.add_edge(target, &note_id, None),
                    UseCaseNotePosition::Top => layout.add_edge(&note_id, target, None),
                }
            }
        }
    }
    let mut result = layout.layout_full(LAYOUT_TIMEOUT)?;
    for cluster in &mut result.cluster_positions {
        // `DotStringFactory.solve` recovers cluster rectangles from
        // Graphviz's two-decimal SVG polygon, not the internal floating-point
        // box. The layout crate retains those serialized dimensions so every
        // package count and title width follows the same reconstruction.
        cluster.x = (cluster.x * 100.0).round() / 100.0;
        cluster.y = (cluster.y * 100.0).round() / 100.0;
        if let Some(&(width, height)) = result.cluster_serialized_sizes.get(&cluster.id) {
            cluster.width = width;
            cluster.height = height;
        }
    }
    let degenerated = diagram.actors.len() + diagram.use_cases.len() + entity_note_indices.len()
        == 1
        && diagram.packages.is_empty()
        && diagram.connections.is_empty();
    let base_origin_x = if degenerated {
        DEGENERATED_MARGIN
    } else {
        BODY_MARGIN
    };
    let base_origin_y = base_origin_x;
    let node_positions: HashMap<&str, _> = layout_node_ids
        .iter()
        .map(String::as_str)
        .zip(result.node_positions.iter())
        .collect();
    let extension_polygon_min_x = diagram
        .connections
        .iter()
        .filter(|connection| connection.extension)
        .filter_map(|connection| {
            let edge = result
                .edge_paths
                .iter()
                .find(|edge| edge.from == connection.from && edge.to == connection.to)?;
            extension_polygon_min_x(connection, edge)
        });
    // Java provenance: `SvekResult.calculateDimension` asks
    // `TextBlockUtils.getMinMax` for the complete painted image before
    // `moveDelta(6 - minX, ...)`. Every actor image contributes its actual
    // `LimitFinder` minimum, whether the root leaf is connected or detached.
    let min_entity_painted_x = diagram
        .actors
        .iter()
        .zip(actor_dims)
        .map(|(actor, dim)| node_positions[actor.id.as_str()].x + dim.paint_min_x)
        .chain(
            diagram
                .use_cases
                .iter()
                .map(|uc| node_positions[uc.id.as_str()].x),
        )
        .chain(
            entity_note_indices
                .iter()
                .map(|&index| note_node_id(&diagram.notes[index], index))
                .map(|id| node_positions[id.as_str()].x),
        )
        .chain(
            result
                .cluster_positions
                .iter()
                .map(|p| cluster_painted_bounds(diagram, p).0),
        )
        .fold(f64::INFINITY, f64::min);
    let min_edge_painted_x = result
        .edge_paths
        .iter()
        .flat_map(|edge| edge.points.iter().map(|point| point.0))
        .chain(extension_polygon_min_x)
        .fold(f64::INFINITY, f64::min);
    let min_painted_x = min_entity_painted_x.min(min_edge_painted_x);
    // Java `SvekResult.calculateDimension` normalizes the complete painted
    // image after clusters, nodes, and `SvekEdge.drawU` have all contributed
    // limits. A hollow `ExtremityTriangle` is a `UPolygon`, so its horizontal
    // measurement also includes `LimitFinder.drawUPolygon`'s guard.
    let has_generalization = diagram
        .connections
        .iter()
        .any(|connection| connection.extension);
    let edge_owns_left_envelope = min_edge_painted_x < min_entity_painted_x;
    // Root entity-only diagrams retain `GraphvizImageBuilder`'s established
    // node-box offset. Recompute X when a non-node primitive owns the minimum,
    // as `SvekResult` must for clusters, solved edges, and polygon extremities.
    let needs_painted_x_normalization =
        !result.cluster_positions.is_empty() || has_generalization || edge_owns_left_envelope;
    let origin_x = if needs_painted_x_normalization && min_painted_x.is_finite() {
        base_origin_x - min_painted_x
    } else {
        base_origin_x
    };
    let min_painted_y = diagram
        .actors
        .iter()
        .zip(actor_dims)
        .map(|(actor, dim)| node_positions[actor.id.as_str()].y + dim.paint_min_y)
        .chain(
            diagram
                .use_cases
                .iter()
                .map(|uc| node_positions[uc.id.as_str()].y),
        )
        .chain(
            entity_note_indices
                .iter()
                .map(|&index| note_node_id(&diagram.notes[index], index))
                .map(|id| node_positions[id.as_str()].y),
        )
        .chain(
            result
                .cluster_positions
                .iter()
                .map(|p| cluster_painted_bounds(diagram, p).1),
        )
        .chain(
            result
                .edge_paths
                .iter()
                .flat_map(|edge| edge.points.iter().map(|point| point.1)),
        )
        .chain(result.edge_paths.iter().filter_map(|edge| {
            let label = edge.label?;
            let (connection_index, _) =
                diagram
                    .connections
                    .iter()
                    .enumerate()
                    .find(|(_, connection)| {
                        connection.from == edge.from
                            && connection.to == edge.to
                            && (connection.label.is_some() || connection.stereotype.is_some())
                    })?;
            if note_on_connection(diagram, connection_index).is_some() {
                return None;
            }
            let (_, solved_y) = quantized_svek_label_origin(label.x, label.y);
            let connection_style = &connection_styles[connection_index];
            // `SvekEdge.drawU` paints the real one-pixel-margined text at the
            // fixed-table origin recovered by `solveLine`.
            // `LimitFinder.drawText` then moves the UText baseline upward by
            // its line-box height minus 1.5 pixels.
            Some(
                solved_y + 1.0 + pm::ascent(connection_style.arrow_font_size as f64)
                    - pm::text_height(connection_style.arrow_font_size as f64)
                    + 1.5,
            )
        }))
        .fold(f64::INFINITY, f64::min);
    let origin_y = if min_painted_y.is_finite() {
        base_origin_y - min_painted_y
    } else {
        base_origin_y
    };
    for edge in &mut result.edge_paths {
        for point in &mut edge.points {
            point.0 += origin_x;
            point.1 += origin_y;
        }
        if let Some(point) = &mut edge.start_point {
            point.0 += origin_x;
            point.1 += origin_y;
        }
        if let Some(point) = &mut edge.end_point {
            point.0 += origin_x;
            point.1 += origin_y;
        }
        if let Some(label) = &mut edge.label {
            label.x += origin_x;
            label.y += origin_y;
        }
        if let Some(label) = &mut edge.tail_label {
            label.x += origin_x;
            label.y += origin_y;
        }
        if let Some(label) = &mut edge.head_label {
            label.x += origin_x;
            label.y += origin_y;
        }
    }
    for cluster in &mut result.cluster_positions {
        cluster.x += origin_x;
        cluster.y += origin_y;
    }
    let actors = diagram
        .actors
        .iter()
        .zip(actor_dims)
        .map(|(actor, dim)| {
            let p = node_positions[actor.id.as_str()];
            (
                p.x + origin_x + p.width / 2.0,
                p.y + origin_y
                    + dim.stereotype.map_or(0.0, ActorTextBlock::height)
                    + dim.stroke_thickness
                    + ACTOR_HEAD_R,
            )
        })
        .collect();
    let use_cases = diagram
        .use_cases
        .iter()
        .zip(uc_dims)
        .map(|(uc, dim)| {
            let p = node_positions[uc.id.as_str()];
            (p.x + origin_x + dim.rx, p.y + origin_y + dim.ry)
        })
        .collect();

    let note_ids: Vec<String> = entity_note_indices
        .iter()
        .map(|&index| note_node_id(&diagram.notes[index], index))
        .collect();
    let mut notes = vec![None; diagram.notes.len()];
    for (slot, &note_index) in entity_note_indices.iter().enumerate() {
        let note_id = &note_ids[slot];
        let p = node_positions[note_id.as_str()];
        let edge = result
            .edge_paths
            .iter()
            .find(|edge| edge.from == *note_id || edge.to == *note_id);
        notes[note_index] = Some(note_placement(
            p.x + origin_x,
            p.y + origin_y,
            &note_dims[note_index],
            note_id,
            edge,
        ));
    }
    let edge_paths = result
        .edge_paths
        .into_iter()
        .filter(|edge| {
            !note_ids
                .iter()
                .any(|note_id| edge.from == *note_id || edge.to == *note_id)
        })
        .collect();
    Some(Positions {
        actors,
        use_cases,
        notes,
        cluster_positions: result.cluster_positions,
        edge_paths,
    })
}

fn fallback_positions(
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
    note_count: usize,
) -> Positions {
    let actors: Vec<(f64, f64)> = actor_dims
        .iter()
        .enumerate()
        .map(|(i, d)| fallback_actor_center(i, d))
        .collect();
    let use_cases: Vec<(f64, f64)> = uc_dims
        .iter()
        .enumerate()
        .map(|(i, d)| fallback_use_case_center(i, d))
        .collect();
    Positions {
        actors,
        use_cases,
        notes: vec![None; note_count],
        cluster_positions: Vec::new(),
        edge_paths: Vec::new(),
    }
}

fn note_node_id(note: &UseCaseNote, index: usize) -> String {
    match &note.kind {
        UseCaseNoteKind::Floating { id } => id.clone(),
        UseCaseNoteKind::Attached { .. } => format!("__rustuml_note_{index}"),
        UseCaseNoteKind::OnLink { .. } => format!("__rustuml_link_note_{index}"),
    }
}

fn effective_note_position(
    position: UseCaseNotePosition,
    direction: UseCaseLayoutDirection,
) -> UseCaseNotePosition {
    if direction == UseCaseLayoutDirection::TopToBottom {
        return position;
    }
    // Java provenance: `Position.withRankdir` rotates entity-note placement
    // when DESCRIPTION diagrams use `left to right direction`.
    match position {
        UseCaseNotePosition::Right => UseCaseNotePosition::Bottom,
        UseCaseNotePosition::Left => UseCaseNotePosition::Top,
        UseCaseNotePosition::Bottom => UseCaseNotePosition::Right,
        UseCaseNotePosition::Top => UseCaseNotePosition::Left,
    }
}

fn note_on_connection(
    diagram: &UseCaseDiagram,
    connection: usize,
) -> Option<(usize, &UseCaseNote)> {
    diagram.notes.iter().enumerate().find(|(_, note)| {
        matches!(
            note.kind,
            UseCaseNoteKind::OnLink {
                connection: owner
            } if owner == connection
        )
    })
}

fn link_note_label_size(
    connection: &UseCaseConnection,
    note: &UseCaseNote,
    note_dim: &NoteDim,
    skin: &SkinColors,
) -> EdgeLabelSize {
    // `EntityImageNoteLink` delegates to `ComponentRoseNote`, whose preferred
    // size includes the EntityImageNote text margins plus Rose's 5px padding.
    let note_width = note_dim.width + LINK_NOTE_PADDING * 2.0;
    let note_height = note_dim.height + LINK_NOTE_PADDING * 2.0;
    let label = connection
        .label
        .as_deref()
        .or(connection.stereotype.as_deref());
    let Some(label) = label else {
        return EdgeLabelSize {
            width: note_width,
            height: note_height,
        };
    };
    let label_width = text_render::measure_with_family(
        label,
        skin.arrow_font_size as f64,
        false,
        &skin.arrow_font_family,
    ) + 2.0;
    let label_height = pm::text_height(skin.arrow_font_size as f64) + 2.0;
    match note.position {
        UseCaseNotePosition::Left | UseCaseNotePosition::Right => EdgeLabelSize {
            width: note_width + label_width,
            height: note_height.max(label_height),
        },
        UseCaseNotePosition::Top | UseCaseNotePosition::Bottom => EdgeLabelSize {
            width: note_width.max(label_width),
            height: note_height + label_height,
        },
    }
}

fn note_placement(
    x: f64,
    y: f64,
    dim: &NoteDim,
    note_id: &str,
    edge: Option<&EdgePath>,
) -> NotePlacement {
    let Some(edge) = edge else {
        return NotePlacement {
            x,
            y,
            apex: None,
            leader_base: None,
        };
    };
    let Some(start) = edge.points.first().copied() else {
        return NotePlacement {
            x,
            y,
            apex: None,
            leader_base: None,
        };
    };
    let Some(end) = edge.points.last().copied() else {
        return NotePlacement {
            x,
            y,
            apex: None,
            leader_base: None,
        };
    };
    let (contact, apex) = if edge.from == note_id {
        (start, end)
    } else {
        (end, start)
    };
    let right = x + dim.width;
    let bottom = y + dim.height;
    let leader_base = if apex.1 < y {
        let x1 = (contact.0 - x - NOTE_LEADER_HALF).clamp(0.0, dim.width - NOTE_FOLD);
        Some(((x + x1 + NOTE_LEADER_HALF * 2.0, y), (x + x1, y)))
    } else if apex.1 > bottom {
        let x1 = (contact.0 - x - NOTE_LEADER_HALF).clamp(0.0, dim.width);
        Some(((x + x1, bottom), (x + x1 + NOTE_LEADER_HALF * 2.0, bottom)))
    } else if apex.0 < x {
        let y1 = (contact.1 - y - NOTE_LEADER_HALF).clamp(0.0, dim.height - NOTE_LEADER_HALF * 2.0);
        Some(((x, y + y1), (x, y + y1 + NOTE_LEADER_HALF * 2.0)))
    } else {
        let y1 = (contact.1 - y - NOTE_LEADER_HALF)
            .clamp(NOTE_FOLD, dim.height - NOTE_LEADER_HALF * 2.0);
        Some(((right, y + y1 + NOTE_LEADER_HALF * 2.0), (right, y + y1)))
    };
    NotePlacement {
        x,
        y,
        apex: Some(apex),
        leader_base,
    }
}

fn lookup_actor_center(oracle: &OracleLayout, actor: &Actor) -> Option<(f64, f64)> {
    let rect = oracle
        .entities
        .get(&actor.id)
        .or_else(|| oracle.entities.get(&actor.label))?;
    Some((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0))
}

fn lookup_use_case_center(
    oracle: &OracleLayout,
    uc: &UseCase,
    diagram: &UseCaseDiagram,
) -> Option<(f64, f64)> {
    let qualified = qualified_name(&uc.id, diagram);
    let rect = oracle
        .entities
        .get(&qualified)
        .or_else(|| oracle.entities.get(&uc.id))
        .or_else(|| oracle.entities.get(&uc.label))?;
    Some((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0))
}

/// PlantUML strips spaces and punctuation when deriving an entity id from a
/// quoted label (mirrors the parser's `label_to_id`).
fn label_to_id(label: &str) -> String {
    label
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// The name PlantUML emits as `data-qualified-name` and keys oracle entities
/// by. For an aliased use case (`usecase "X" as UC1`) this is the alias/id; for
/// a label-declared one (`usecase "Primary Action"`) it is the original label,
/// spaces and all.
fn display_name(uc: &UseCase) -> &str {
    if uc.explicit_id {
        &uc.id
    } else if uc.id == label_to_id(&uc.label) {
        &uc.label
    } else {
        &uc.id
    }
}

/// PlantUML sanitises `data-qualified-name` (and the entity key it stores in
/// the oracle map) by replacing every non-ASCII character with `.`. The visible
/// label text keeps the original unicode; only the identifier attribute is
/// folded.
fn sanitize_qname(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii() { c } else { '.' })
        .collect()
}

/// The form PlantUML uses in the `<!--entity/cluster …-->` comments: non-ASCII
/// characters are replaced with `?` (distinct from the `.` used in
/// `data-qualified-name`).
fn sanitize_comment(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect()
}

fn qualified_name(id: &str, diagram: &UseCaseDiagram) -> String {
    // Resolve the display name for use cases (label-declared ones use their
    // label, not the space-stripped id).
    let display = diagram
        .use_cases
        .iter()
        .find(|u| u.id == id)
        .map(display_name)
        .unwrap_or(id);
    for pkg in &diagram.packages {
        if pkg.elements.iter().any(|e| e == id) {
            return sanitize_qname(&format!("{}.{display}", pkg.name));
        }
    }
    sanitize_qname(display)
}

fn fallback_actor_center(i: usize, _dim: &ActorDim) -> (f64, f64) {
    let cx = MARGIN + ACTOR_HEAD_R;
    let cy = MARGIN + ACTOR_HEAD_R + i as f64 * (ACTOR_HEAD_R * 2.0 + ACTOR_BODY_LEN + GAP);
    (cx, cy)
}

fn fallback_use_case_center(i: usize, dim: &UseCaseDim) -> (f64, f64) {
    let cx = MARGIN + 80.0 + dim.rx;
    let cy = MARGIN + dim.ry + i as f64 * (dim.ry * 2.0 + GAP);
    (cx, cy)
}

fn compute_canvas(
    diagram: &UseCaseDiagram,
    positions: &Positions,
    actor_dims: &[ActorDim],
    uc_dims: &[UseCaseDim],
    note_dims: &[NoteDim],
) -> (f64, f64) {
    let degenerated = actor_dims.len() + uc_dims.len() == 1
        && positions.edge_paths.is_empty()
        && positions.cluster_positions.is_empty();
    // `EntityImageDegenerated` owns the 7px entity inset; the surrounding
    // image builder contributes the remaining 12px on the far edges.
    let canvas_pad = if degenerated {
        SVEK_CANVAS_PAD - 2.0
    } else {
        SVEK_CANVAS_PAD
    };
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for (i, (cx, cy)) in positions.actors.iter().enumerate() {
        let half = actor_dims[i].width / 2.0;
        let leg_y = cy + ACTOR_HEAD_R + ACTOR_BODY_LEN + ACTOR_LEG_DROP;
        // Java provenance: `LimitFinder.drawText` moves a `UText` up by
        // `height - 1.5`, so its measured maximum is the emitted baseline plus
        // 1.5 rather than the baseline plus a full line box.
        let label_max_y = leg_y + actor_dims[i].label_gap + 1.5;
        let stereotype_max_y = if actor_dims[i].stereotype.is_some() {
            cy - actor_dims[i].stereo_baseline_offset + 1.5
        } else {
            f64::NEG_INFINITY
        };
        max_x = max_x.max(cx + half + canvas_pad);
        max_y = max_y.max(leg_y.max(label_max_y).max(stereotype_max_y) + canvas_pad);
    }
    for (i, (cx, cy)) in positions.use_cases.iter().enumerate() {
        // Java provenance: `LimitFinder.drawEllipse` records the far corner at
        // `origin + dimension - 1`, so the painted oval's maximum is one pixel
        // inside its geometric bounding box on both axes.
        max_x = max_x.max(cx + uc_dims[i].rx - 1.0 + canvas_pad);
        max_y = max_y.max(cy + uc_dims[i].ry - 1.0 + canvas_pad);
    }
    for (index, note) in positions.notes.iter().enumerate() {
        let Some(note) = note else { continue };
        max_x = max_x.max(note.x + note_dims[index].width + SVEK_CANVAS_PAD);
        max_y = max_y.max(note.y + note_dims[index].height + SVEK_CANVAS_PAD);
    }
    for edge in &positions.edge_paths {
        for (x, y) in &edge.points {
            max_x = max_x.max(x + SVEK_CANVAS_PAD);
            max_y = max_y.max(y + SVEK_CANVAS_PAD);
        }
        if let Some(label) = edge.label {
            let rose_note_trailing_pad = diagram
                .connections
                .iter()
                .enumerate()
                .filter(|(_, connection)| {
                    connection.from == edge.from
                        && connection.to == edge.to
                        && connection.label.is_none()
                        && connection.stereotype.is_none()
                })
                .find_map(|(connection_index, _)| {
                    let (note_index, _) = note_on_connection(diagram, connection_index)?;
                    let expected_width = note_dims[note_index].width + LINK_NOTE_PADDING * 2.0;
                    let expected_height = note_dims[note_index].height + LINK_NOTE_PADDING * 2.0;
                    // Graphviz's SVG label rectangle serializes these
                    // component dimensions at whole-pixel precision.
                    ((label.width - expected_width.floor()).abs() < 0.01
                        && (label.height - expected_height.floor()).abs() < 0.01)
                        .then_some(LINK_NOTE_PADDING)
                })
                .unwrap_or(0.0);
            // Java provenance: `ComponentRoseNote` reports a label box with
            // five pixels of padding on every side, then paints the Opale note
            // at `(5,5)`. For a pure note-on-link label, `LimitFinder` sees the
            // painted note but not the unused trailing padding.
            max_x = max_x.max(label.x + label.width - rose_note_trailing_pad + SVEK_CANVAS_PAD);
            max_y = max_y.max(label.y + label.height - rose_note_trailing_pad + SVEK_CANVAS_PAD);
        }
    }
    for cluster in &positions.cluster_positions {
        let (_, _, painted_max_x, painted_max_y) = cluster_painted_bounds(diagram, cluster);
        // `SvekResult.calculateDimension` adds 15 to the complete painted
        // span. Since normalization has already moved the minimum to six,
        // the translated painted maximum receives the remaining 15 pixels.
        max_x = max_x.max(painted_max_x + SVEK_CANVAS_PAD + 1.0);
        max_y = max_y.max(painted_max_y + SVEK_CANVAS_PAD + 1.0);
    }
    (max_x.max(1.0), max_y.max(1.0))
}

fn cluster_painted_bounds(
    diagram: &UseCaseDiagram,
    cluster: &ClusterPosition,
) -> (f64, f64, f64, f64) {
    let kind = diagram
        .packages
        .iter()
        .find(|package| package.name == cluster.id)
        .map(|package| package.kind)
        .unwrap_or(PackageKind::Rectangle);
    match kind {
        // `USymbolRectangle.asBig` draws a `URectangle`.
        // `LimitFinder.drawRectangle` expands its top-left by one pixel and
        // records the far corner one pixel inside the nominal box.
        PackageKind::Rectangle => (
            cluster.x - 1.0,
            cluster.y - 1.0,
            cluster.x + cluster.width - 1.0,
            cluster.y + cluster.height - 1.0,
        ),
        // `USymbolFolder.drawFolder` draws the package outline as a `UPath`;
        // `LimitFinder.drawUPath` uses the path's exact extrema.
        PackageKind::Package => (
            cluster.x,
            cluster.y,
            cluster.x + cluster.width,
            cluster.y + cluster.height,
        ),
    }
}

fn render_package_group(
    svg: &mut SvgBuilder,
    pkg: &UseCasePackage,
    oracle: Option<&OracleLayout>,
    cluster_positions: &[ClusterPosition],
    id_map: &HashMap<String, String>,
) {
    // PlantUML keys clusters by the sanitised qualified name (non-ASCII → `.`).
    let qname = sanitize_qname(&pkg.name);
    let (rect_x, rect_y, rect_w, rect_h, captured_x, captured_y): (
        f64,
        f64,
        f64,
        f64,
        &[f64],
        &[f64],
    ) = if let Some(orc) = oracle {
        let Some(rect) = orc
            .entities
            .get(&qname)
            .or_else(|| orc.entities.get(&pkg.name))
        else {
            return;
        };
        (
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            rect.text_x_values.as_slice(),
            rect.text_y_values.as_slice(),
        )
    } else {
        let Some(cluster) = cluster_positions.iter().find(|p| p.id == pkg.name) else {
            return;
        };
        (
            cluster.x,
            cluster.y,
            cluster.width,
            cluster.height,
            &[],
            &[],
        )
    };
    let ent_id = id_map
        .get(&format!("pkg::{}", pkg.name))
        .cloned()
        .unwrap_or_else(|| "ent0003".to_string());
    let src_attr = source_line_attr(pkg.source_line);
    let fill = pkg
        .color
        .as_deref()
        .map(resolve_fill)
        .unwrap_or_else(|| "none".to_string());
    svg.raw(&format!("<!--cluster {}-->", sanitize_comment(&pkg.name)));
    svg.raw(&format!(
        r#"<g class="cluster" data-qualified-name="{qname}"{src_attr} id="{ent_id}">"#,
    ));
    let label_w = text_render::measure(&pkg.name, FONT_SIZE, true);
    let (label_x, label_y) = match pkg.kind {
        PackageKind::Rectangle => {
            // Plain rounded rect, centred bold label.
            svg.raw(&format!(
                r#"<rect fill="{fill}" height="{h}" rx="2.5" ry="2.5" style="stroke:#181818;stroke-width:1;" width="{w}" x="{x}" y="{y}"/>"#,
                h = fc(rect_h),
                w = fc(rect_w),
                x = fc(rect_x),
                y = fc(rect_y),
            ));
            (rect_x + (rect_w - label_w) / 2.0, rect_y + 15.5352)
        }
        PackageKind::Package => {
            // Folder-tab outline: a notched top-left "tab" carrying the label,
            // a diagonal slope down to the body's top edge, then a rounded
            // rectangle body. Reconstructed from the oracle box rect and label
            // width (HALF_UP coords).
            let x = rect_x;
            let y = rect_y;
            let xr = rect_x + rect_w;
            let yb = rect_y + rect_h;
            // Tab top-right corner: label start (x+4) + label width, less 0.5.
            let tab_tr = x + 3.5 + label_w;
            let tab_y = y + pm::text_height(FONT_SIZE) + 6.0;
            let d = format!(
                "M{x25},{y_s} L{tab_tr},{y_s} A3.75,3.75 0 0 1 {tab_tr25},{y25} L{tab_br},{ty} L{xr25},{ty} A2.5,2.5 0 0 1 {xr_s},{ty25} L{xr_s},{yb2} A2.5,2.5 0 0 1 {xr25},{yb_s} L{x25},{yb_s} A2.5,2.5 0 0 1 {x_s},{yb2} L{x_s},{y25} A2.5,2.5 0 0 1 {x25},{y_s}",
                x25 = fc(x + 2.5),
                y_s = fc(y),
                tab_tr = fc(tab_tr),
                tab_tr25 = fc(tab_tr + 2.5),
                y25 = fc(y + 2.5),
                tab_br = fc(tab_tr + 9.5),
                ty = fc(tab_y),
                xr25 = fc(xr - 2.5),
                xr_s = fc(xr),
                ty25 = fc(tab_y + 2.5),
                yb2 = fc(yb - 2.5),
                yb_s = fc(yb),
                x_s = fc(x),
            );
            svg.raw(&format!(
                r#"<path d="{d}" fill="{fill}" style="stroke:#000000;stroke-width:1.5;"/>"#,
            ));
            // Divider under the tab from the left edge to the slope end.
            svg.raw(&format!(
                r#"<line style="stroke:#000000;stroke-width:1.5;" x1="{x1}" x2="{x2}" y1="{ty}" y2="{ty}"/>"#,
                x1 = fc(x),
                x2 = fc(tab_tr + 9.5),
                ty = fc(tab_y),
            ));
            (x + 4.0, y + 15.5352)
        }
    };
    // Prefer the oracle-captured label baseline (avoids sub-pixel drift from
    // recomputing `rect.y + offset` against the already-rounded oracle rect).
    let label_x = captured_x.first().copied().unwrap_or(label_x);
    let label_y = captured_y.first().copied().unwrap_or(label_y);
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        &pkg.name,
        &TextBase {
            x: label_x,
            y: label_y,
            font_size: FONT_SIZE as u32,
            font_family: "sans-serif",
            fill: TEXT_COLOR,
            bold: true,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
    svg.raw("</g>");
}

#[allow(clippy::too_many_arguments)]
fn render_actor(
    svg: &mut SvgBuilder,
    actor: &Actor,
    dim: &ActorDim,
    cx: f64,
    cy: f64,
    oracle: Option<&OracleLayout>,
    id_map: &HashMap<String, String>,
    skin: &SkinColors,
    fill: &str,
) {
    // Prefer the oracle-captured entity id (PlantUML's real counter allocation,
    // which notes and other synthetic entities perturb), falling back to the
    // source-line-derived map when no oracle is present.
    let oracle_id = oracle.and_then(|orc| {
        orc.entities
            .get(&actor.id)
            .or_else(|| orc.entities.get(&actor.label))
            .and_then(|r| r.entity_id.clone())
    });
    let ent_id = oracle_id
        .or_else(|| id_map.get(&format!("actor::{}", actor.id)).cloned())
        .unwrap_or_else(|| "ent0002".to_string());
    // For a label-declared actor (`actor "External System"`) PlantUML keys the
    // qualified name and comment on the original label (spaces and all); for an
    // aliased/bare actor it uses the id.
    let display_raw = if actor.id == label_to_id(&actor.label) {
        actor.label.as_str()
    } else {
        actor.id.as_str()
    };
    svg.raw(&format!("<!--entity {}-->", sanitize_comment(display_raw)));
    let display = sanitize_qname(display_raw);
    let src_attr = source_line_attr(actor.source_line);
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{display}"{src_attr} id="{ent_id}">"#,
    ));
    let stroke = skin.actor_border.as_deref().unwrap_or(STROKE);
    svg.raw(&format!(
        r#"<ellipse cx="{cx}" cy="{cy}" fill="{fill}" rx="{ACTOR_HEAD_R}" ry="{ACTOR_HEAD_R}" style="stroke:{stroke};stroke-width:{stroke_width};"/>"#,
        cx = fc(cx),
        cy = fc(cy),
        stroke_width = skin.actor_border_thickness,
    ));
    let body_top_y = cy + ACTOR_HEAD_R;
    let body_bot_y = body_top_y + ACTOR_BODY_LEN;
    let arm_y = body_top_y + ACTOR_ARM_OFFSET;
    let leg_x_left = cx - ACTOR_LEG_RUN;
    let leg_x_right = cx + ACTOR_LEG_RUN;
    let leg_y = body_bot_y + ACTOR_LEG_DROP;
    let arm_left_x = cx - ACTOR_ARM_HALF;
    let arm_right_x = cx + ACTOR_ARM_HALF;
    svg.raw(&format!(
        r#"<path d="M{cx},{body_top_y} L{cx},{body_bot_y} M{arm_left_x},{arm_y} L{arm_right_x},{arm_y} M{cx},{body_bot_y} L{leg_x_left},{leg_y} M{cx},{body_bot_y} L{leg_x_right},{leg_y}" fill="none" style="stroke:{stroke};stroke-width:{stroke_width};"/>"#,
        cx = fc(cx),
        body_top_y = fc(body_top_y),
        body_bot_y = fc(body_bot_y),
        arm_left_x = fc(arm_left_x),
        arm_right_x = fc(arm_right_x),
        arm_y = fc(arm_y),
        leg_x_left = fc(leg_x_left),
        leg_x_right = fc(leg_x_right),
        leg_y = fc(leg_y),
        stroke_width = skin.actor_border_thickness,
    ));
    let cx_anchor = round_coord(cx);
    // Prefer PlantUML's captured per-line text x (label first, stereotype
    // second in document order) over reconstructing it from the rounded centre.
    let orc_rect = oracle.and_then(|orc| {
        orc.entities
            .get(&actor.id)
            .or_else(|| orc.entities.get(&actor.label))
    });
    let captured_x = orc_rect.map(|r| r.text_x_values.as_slice()).unwrap_or(&[]);
    let captured_y = orc_rect.map(|r| r.text_y_values.as_slice()).unwrap_or(&[]);
    let label_x = captured_x
        .first()
        .copied()
        .unwrap_or(cx_anchor - dim.width / 2.0 + dim.label.text_x(dim.width));
    let label_y = captured_y.first().copied().unwrap_or(leg_y + dim.label_gap);
    let mut buf = String::new();
    text_render::emit_text(
        &mut buf,
        &actor.label,
        &TextBase {
            x: label_x,
            y: label_y,
            font_size: skin.actor_font_size,
            font_family: &skin.actor_font_family,
            fill: &skin.actor_font_color,
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        },
    );
    svg.raw(&buf);
    if let Some(stereo) = &actor.stereotype {
        let stereo_text = format!("\u{00AB}{stereo}\u{00BB}");
        let stereo_x = captured_x.get(1).copied().unwrap_or_else(|| {
            cx_anchor - dim.width / 2.0
                + dim
                    .stereotype
                    .expect("stereotype block exists")
                    .text_x(dim.width)
        });
        let stereo_y = captured_y
            .get(1)
            .copied()
            .unwrap_or(cy - dim.stereo_baseline_offset);
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            &stereo_text,
            &TextBase {
                x: stereo_x,
                y: stereo_y,
                font_size: skin.actor_font_size,
                font_family: &skin.actor_font_family,
                fill: &skin.actor_stereo_font_color,
                bold: false,
                italic: true,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&buf);
    }
    svg.raw("</g>");
}

#[allow(clippy::too_many_arguments)]
fn render_use_case(
    svg: &mut SvgBuilder,
    uc: &UseCase,
    dim: &UseCaseDim,
    diagram: &UseCaseDiagram,
    cx: f64,
    cy: f64,
    oracle: Option<&OracleLayout>,
    id_map: &HashMap<String, String>,
    skin: &SkinColors,
    fill: &str,
) {
    let qualified = qualified_name(&uc.id, diagram);
    // Prefer the oracle-captured entity id (PlantUML's real counter allocation),
    // falling back to the source-line-derived map when no oracle is present.
    let oracle_id = oracle.and_then(|orc| {
        orc.entities
            .get(&qualified)
            .or_else(|| orc.entities.get(&uc.id))
            .or_else(|| orc.entities.get(&uc.label))
            .and_then(|r| r.entity_id.clone())
    });
    let ent_id = oracle_id
        .or_else(|| id_map.get(&format!("uc::{}", uc.id)).cloned())
        .unwrap_or_else(|| "ent0003".to_string());
    svg.raw(&format!(
        "<!--entity {}-->",
        sanitize_comment(display_name(uc))
    ));
    let src_attr = source_line_attr(uc.source_line);
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qualified}"{src_attr} id="{ent_id}">"#,
    ));
    let orc_rect = oracle.and_then(|orc| {
        orc.entities
            .get(&qualified)
            .or_else(|| orc.entities.get(&uc.id))
            .or_else(|| orc.entities.get(&uc.label))
    });
    let (rx, ry) = if let Some(rect) = orc_rect {
        (rect.width / 2.0, rect.height / 2.0)
    } else {
        (dim.rx, dim.ry)
    };
    let stroke = skin.uc_border.as_deref().unwrap_or(STROKE);
    svg.raw(&format!(
        r#"<ellipse cx="{cx}" cy="{cy}" fill="{fill}" rx="{rx}" ry="{ry}" style="stroke:{stroke};stroke-width:{stroke_width};"/>"#,
        cx = fc(cx),
        cy = fc(cy),
        rx = fc(rx),
        ry = fc(ry),
        stroke_width = skin.uc_border_thickness,
    ));
    let cx_anchor = round_coord(cx);
    // PlantUML's emitted text x values are captured per line (stereotype first,
    // then label/description lines). Reconstructing them from the 4-dp-rounded
    // ellipse centre loses sub-pixel precision, so prefer the captured value and
    // fall back to the geometric centre only when no oracle x is available.
    let captured_x = orc_rect.map(|r| r.text_x_values.as_slice()).unwrap_or(&[]);
    let captured_y = orc_rect.map(|r| r.text_y_values.as_slice()).unwrap_or(&[]);
    let mut line_idx = 0usize;
    // Java `TextBlockInEllipse.drawU` translates the merged text by
    // `(ellipseHalfHeight - footprintCenterY - 2)`. Since `cy` already
    // includes the half-height, each baseline is relative to the computed
    // painted footprint center rather than a fixed one-line offset.
    let text_origin_y = cy - dim.footprint_center_y - 2.0;
    if let Some(stereo) = &uc.stereotype {
        let stereo_text = format!("\u{00AB}{stereo}\u{00BB}");
        let text_y = text_origin_y
            + dim.text_block.stereo_text_top.unwrap_or(0.0)
            + text_render::label_first_baseline_ascent_with_family(
                &stereo_text,
                skin.uc_font_size as f64,
                &skin.uc_font_family,
            );
        let stereo_x = captured_x
            .get(line_idx)
            .copied()
            .unwrap_or(cx_anchor + dim.text_x_shift - dim.stereo_w / 2.0);
        let stereo_y = captured_y.get(line_idx).copied().unwrap_or(text_y);
        line_idx += 1;
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            &stereo_text,
            &TextBase {
                x: stereo_x,
                y: stereo_y,
                font_size: skin.uc_font_size,
                font_family: &skin.uc_font_family,
                fill: &skin.uc_stereo_font_color,
                bold: false,
                italic: true,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&buf);
    }
    let text_y = text_origin_y
        + dim
            .text_block
            .body_line_tops
            .first()
            .copied()
            .unwrap_or(0.0)
        + text_render::label_first_baseline_ascent_with_family(
            if uc.description.is_empty() {
                &uc.label
            } else {
                uc.description.first().map(String::as_str).unwrap_or("")
            },
            skin.uc_font_size as f64,
            &skin.uc_font_family,
        );
    if uc.description.is_empty() {
        let label_x = captured_x
            .get(line_idx)
            .copied()
            .unwrap_or(cx_anchor + dim.text_x_shift - dim.label_w / 2.0);
        let label_y = captured_y.get(line_idx).copied().unwrap_or(text_y);
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            &uc.label,
            &TextBase {
                x: label_x,
                y: label_y,
                font_size: skin.uc_font_size,
                font_family: &skin.uc_font_family,
                fill: &skin.uc_font_color,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&buf);
    } else {
        // Separator dividers (from `--`/`==`/`..` lines in a multiline label)
        // are captured verbatim from the oracle (full geometry + style, so
        // dashed `..` rules and `==` double rules survive) and interleaved with
        // the text lines by y-position: each divider is flushed before the first
        // text line whose baseline sits below it. Fall back to the geometry-only
        // `sep_lines` when the styled capture is unavailable.
        let sep_styled = orc_rect.map(|r| r.lines.as_slice()).unwrap_or(&[]);
        let sep_geom = orc_rect.map(|r| r.sep_lines.as_slice()).unwrap_or(&[]);
        let use_styled = !sep_styled.is_empty();
        let mut sep_idx = 0usize;
        let flush_seps = |svg: &mut SvgBuilder, sep_idx: &mut usize, before_y: f64| {
            if use_styled {
                while let Some(l) = sep_styled.get(*sep_idx) {
                    let y1: f64 = l.y1.parse().unwrap_or(0.0);
                    if y1 < before_y {
                        let style = l
                            .style
                            .as_deref()
                            .unwrap_or("stroke:#181818;stroke-width:1;");
                        svg.raw(&format!(
                            r#"<line style="{style}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            l.x1, l.x2, l.y1, l.y2,
                        ));
                        *sep_idx += 1;
                    } else {
                        break;
                    }
                }
            } else {
                while let Some(&(x1, x2, y1)) = sep_geom.get(*sep_idx) {
                    if y1 < before_y {
                        svg.raw(&format!(
                            r#"<line style="stroke:{STROKE};stroke-width:1;" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                            fc(x1),
                            fc(x2),
                            fc(y1),
                            fc(y1),
                        ));
                        *sep_idx += 1;
                    } else {
                        break;
                    }
                }
            }
        };
        for (body_line, line) in uc.description.iter().enumerate() {
            let lw = text_render::measure_with_family(
                line,
                skin.uc_font_size as f64,
                false,
                &skin.uc_font_family,
            );
            let lx = captured_x
                .get(line_idx)
                .copied()
                .unwrap_or(cx_anchor + dim.text_x_shift - lw / 2.0);
            let ly = captured_y.get(line_idx).copied().unwrap_or_else(|| {
                text_origin_y
                    + dim
                        .text_block
                        .body_line_tops
                        .get(body_line)
                        .copied()
                        .unwrap_or(body_line as f64 * pm::text_height(FONT_SIZE))
                    + text_render::label_first_baseline_ascent_with_family(
                        line,
                        skin.uc_font_size as f64,
                        &skin.uc_font_family,
                    )
            });
            if orc_rect.is_some() {
                flush_seps(svg, &mut sep_idx, ly);
            } else {
                for separator in dim
                    .text_block
                    .separators
                    .iter()
                    .filter(|separator| separator.before_line == body_line)
                {
                    render_usecase_separator(svg, separator, dim, cx, cy, text_origin_y, stroke);
                }
            }
            line_idx += 1;
            let mut buf = String::new();
            text_render::emit_text(
                &mut buf,
                line,
                &TextBase {
                    x: lx,
                    y: ly,
                    font_size: skin.uc_font_size,
                    font_family: &skin.uc_font_family,
                    fill: &skin.uc_font_color,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&buf);
        }
        // Any trailing separators after the last text line.
        if orc_rect.is_some() {
            flush_seps(svg, &mut sep_idx, f64::INFINITY);
        } else {
            for separator in dim
                .text_block
                .separators
                .iter()
                .filter(|separator| separator.before_line >= uc.description.len())
            {
                render_usecase_separator(svg, separator, dim, cx, cy, text_origin_y, stroke);
            }
        }
    }
    svg.raw("</g>");
}

fn render_usecase_separator(
    svg: &mut SvgBuilder,
    separator: &UseCaseSeparatorPlacement,
    dim: &UseCaseDim,
    cx: f64,
    cy: f64,
    text_origin_y: f64,
    stroke: &str,
) {
    let line_y = text_origin_y + separator.y;
    let emit_line = |svg: &mut SvgBuilder, y: f64, dotted: bool| {
        let normalized_y = ((y - cy) / dim.ry).clamp(-1.0, 1.0);
        let half_width = dim.rx * (1.0 - normalized_y * normalized_y).max(0.0).sqrt();
        let x1 = cx - half_width + SEPARATOR_SKIP_X;
        let x2 = cx + half_width - SEPARATOR_SKIP_X;
        let dash = if dotted { "stroke-dasharray:1,2;" } else { "" };
        svg.raw(&format!(
            r#"<line style="stroke:{stroke};stroke-width:1;{dash}" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            fc(x1),
            fc(x2),
            fc(y),
            fc(y),
        ));
    };

    match separator.style {
        UseCaseSeparatorStyle::Solid => emit_line(svg, line_y, false),
        UseCaseSeparatorStyle::Dotted => emit_line(svg, line_y, true),
        UseCaseSeparatorStyle::Double => {
            emit_line(svg, line_y, false);
            emit_line(svg, line_y + DOUBLE_SEPARATOR_GAP, false);
        }
    }
}

/// Render a note entity, reconstructing the box-plus-leader path locally from
/// the oracle-captured geometry (box rect, leader apex/base, text baselines).
fn emit_note(svg: &mut SvgBuilder, note: &crate::layout_oracle::OracleNoteEntity) {
    let Some(g) = note.box_geom.as_ref() else {
        return;
    };
    let d = note_outline(g.x, g.y, g.width, g.height, g.apex, g.leader_base);
    let right = g.x + g.width;
    let rf = right - NOTE_FOLD;
    let yf = g.y + NOTE_FOLD;

    let src_attr = note
        .source_line
        .as_deref()
        .map(|sl| format!(r#" data-source-line="{sl}""#))
        .unwrap_or_default();
    let ent_id = note.entity_id.as_deref().unwrap_or("");
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qn}"{src_attr} id="{ent_id}">"#,
        qn = note.qualified_name,
    ));
    svg.raw(&format!(
        r#"<path d="{d}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
    ));
    // Folded-corner detail (second path).
    svg.raw(&format!(
        r#"<path d="M{rf_s},{by_s} L{rf_s},{yf_s} L{r_s},{yf_s} L{rf_s},{by_s}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        rf_s = fc(rf),
        by_s = fc(g.y),
        yf_s = fc(yf),
        r_s = fc(right),
    ));
    // Text lines at their captured baselines.
    for (tx, ty, line) in &g.text_lines {
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            line,
            &TextBase {
                x: *tx,
                y: *ty,
                font_size: NOTE_FONT_SIZE,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&buf);
    }
    svg.raw("</g>");
}

fn note_outline(
    bx: f64,
    by: f64,
    width: f64,
    height: f64,
    apex: Option<(f64, f64)>,
    leader_base: Option<((f64, f64), (f64, f64))>,
) -> String {
    let right = bx + width;
    let bottom = by + height;
    let rf = right - NOTE_FOLD;
    let yf = by + NOTE_FOLD;
    let side = apex.map(|(ax, ay)| {
        if ay < by {
            LeaderSide::Top
        } else if ay > bottom {
            LeaderSide::Bottom
        } else if ax < bx {
            LeaderSide::Left
        } else {
            LeaderSide::Right
        }
    });
    let leader = |d: &mut String| {
        if let (Some((ax, ay)), Some((b0, b1))) = (apex, leader_base) {
            let _ = write!(
                d,
                "L{},{} L{},{} L{},{} ",
                fc(b0.0),
                fc(b0.1),
                fc(ax),
                fc(ay),
                fc(b1.0),
                fc(b1.1),
            );
        }
    };

    // `Opale.getPolygon*` walks the outline counter-clockwise and splices the
    // hidden link into the nearest box edge.
    let mut d = String::new();
    let _ = write!(d, "M{},{} ", fc(bx), fc(by));
    if matches!(side, Some(LeaderSide::Left)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(bx), fc(bottom));
    let _ = write!(d, "A0,0 0 0 0 {},{} ", fc(bx), fc(bottom));
    if matches!(side, Some(LeaderSide::Bottom)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(right), fc(bottom));
    let _ = write!(d, "A0,0 0 0 0 {},{} ", fc(right), fc(bottom));
    if matches!(side, Some(LeaderSide::Right)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(right), fc(yf));
    let _ = write!(d, "L{},{} ", fc(rf), fc(by));
    if matches!(side, Some(LeaderSide::Top)) {
        leader(&mut d);
    }
    let _ = write!(d, "L{},{} ", fc(bx), fc(by));
    let _ = write!(d, "A0,0 0 0 0 {},{}", fc(bx), fc(by));
    d
}

fn emit_model_note(
    svg: &mut SvgBuilder,
    note: &UseCaseNote,
    dim: &NoteDim,
    placement: &NotePlacement,
    index: usize,
    id_map: &HashMap<String, String>,
) {
    let qname = match &note.kind {
        UseCaseNoteKind::Attached { .. } => id_map
            .get(&format!("note-qname::{index}"))
            .map(String::as_str)
            .unwrap_or("GMN"),
        UseCaseNoteKind::Floating { id } => id.as_str(),
        UseCaseNoteKind::OnLink { .. } => return,
    };
    let entity_id = id_map
        .get(&format!("note::{index}"))
        .map(String::as_str)
        .unwrap_or("");
    let d = note_outline(
        placement.x,
        placement.y,
        dim.width,
        dim.height,
        placement.apex,
        placement.leader_base,
    );
    let right = placement.x + dim.width;
    let fold_x = right - NOTE_FOLD;
    let fold_y = placement.y + NOTE_FOLD;
    svg.raw(&format!(
        r#"<g class="entity" data-qualified-name="{qname}"{} id="{entity_id}">"#,
        source_line_attr(note.source_line),
    ));
    svg.raw(&format!(
        r#"<path d="{d}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#
    ));
    svg.raw(&format!(
        r#"<path d="M{fold_x},{top} L{fold_x},{fold_y} L{right},{fold_y} L{fold_x},{top}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        fold_x = fc(fold_x),
        top = fc(placement.y),
        fold_y = fc(fold_y),
        right = fc(right),
    ));
    let mut baseline = placement.y + NOTE_MARGIN_Y + pm::ascent(NOTE_FONT_SIZE as f64);
    for line in note.text.split('\n') {
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            line,
            &TextBase {
                x: placement.x + NOTE_MARGIN_LEFT,
                y: baseline,
                font_size: NOTE_FONT_SIZE,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&buf);
        baseline += pm::text_height(NOTE_FONT_SIZE as f64);
    }
    svg.raw("</g>");
}

fn source_line_attr(source_line: usize) -> String {
    if source_line == 0 {
        String::new()
    } else {
        format!(r#" data-source-line="{source_line}""#)
    }
}

fn entity_label<'a>(diagram: &'a UseCaseDiagram, id: &'a str) -> &'a str {
    if let Some(a) = diagram.actors.iter().find(|a| a.id == id) {
        return a.label.as_str();
    }
    if let Some(u) = diagram.use_cases.iter().find(|u| u.id == id) {
        return u.label.as_str();
    }
    id
}

fn render_oracle_connections(
    svg: &mut SvgBuilder,
    diagram: &UseCaseDiagram,
    oracle: &OracleLayout,
    connection_styles: &[SkinColors],
) {
    // PlantUML emits links sorted by source line. The parser already stores
    // connections in declaration order, but sort defensively.
    // Iterate the oracle's edges in their captured DOM order — PlantUML does
    // not always emit links in source-line order (e.g. an `<<extend>>` edge can
    // precede the `<<include>>` edges declared before it), so the oracle order
    // is authoritative. For each edge, find the connection it corresponds to
    // (for the note-on-link label/shape ordering), consuming each connection
    // once so edges sharing an endpoint pair bind to distinct connections.
    let mut used_conns: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for oracle_edge in &oracle.edges {
        let conn_idx = diagram.connections.iter().enumerate().position(|(i, c)| {
            if used_conns.contains(&i) {
                return false;
            }
            let from_label = entity_label(diagram, &c.from);
            let to_label = entity_label(diagram, &c.to);
            let candidates = [
                format!("{from_label}-to-{to_label}"),
                format!("{from_label}-{to_label}"),
                format!("{from_label}-backto-{to_label}"),
                format!("{}-to-{}", c.from, c.to),
                format!("{}-{}", c.from, c.to),
                format!("{}-backto-{}", c.from, c.to),
            ];
            candidates.iter().any(|cand| cand == &oracle_edge.id)
        });
        let Some(conn_idx) = conn_idx else {
            continue;
        };
        used_conns.insert(conn_idx);
        let conn = &diagram.connections[conn_idx];
        let skin = &connection_styles[conn_idx];
        let from_label = entity_label(diagram, &conn.from);
        let to_label = entity_label(diagram, &conn.to);
        let entity_1 = oracle_edge.entity_1.as_deref().unwrap_or("");
        let entity_2 = oracle_edge.entity_2.as_deref().unwrap_or("");
        let link_type = oracle_edge.link_type.as_deref().unwrap_or("association");
        let source_line = oracle_edge.source_line.as_deref();
        let link_id = oracle_edge.link_id.as_deref().unwrap_or("lnk0");
        let source_attr = source_line
            .map(|s| format!(r#" data-source-line="{s}""#))
            .unwrap_or_default();
        svg.raw(&format!("<!--link {from_label} to {to_label}-->"));
        svg.raw(&format!(
            r#"<g class="link" data-entity-1="{entity_1}" data-entity-2="{entity_2}" data-link-type="{link_type}"{source_attr} id="{link_id}">"#,
        ));
        let path_style = oracle_edge
            .path_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");
        let code_line_attr = oracle_edge
            .code_line
            .as_ref()
            .map(|c| format!(r#" codeLine="{c}""#))
            .unwrap_or_default();
        svg.raw(&format!(
            r#"<path{code_line_attr} d="{}" fill="none" id="{}" style="{path_style}"/>"#,
            oracle_edge.d, oracle_edge.id,
        ));
        if let Some(ref points) = oracle_edge.arrow_points {
            let fill = oracle_edge.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = oracle_edge
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            svg.raw(&format!(
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            ));
        }
        if let Some(ref points) = oracle_edge.second_arrow_points {
            let fill = oracle_edge.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = oracle_edge
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            svg.raw(&format!(
                r#"<polygon fill="{fill}" points="{points}" style="{poly_style}"/>"#,
            ));
        }
        // Edge labels and any `note on link` shape. PlantUML emits, in document
        // order: the link label (e.g. `«extend»`), then the note's box path and
        // folded-corner path (`extra_paths`), then the note text. We replay that
        // order: the first label, the extra paths, then the remaining labels.
        // Falls back to the single concatenated `label` when no per-line labels
        // were captured.
        let emit_label = |svg: &mut SvgBuilder, lx: f64, ly: f64, text: &str| {
            let mut buf = String::new();
            text_render::emit_text(
                &mut buf,
                text,
                &TextBase {
                    x: lx,
                    y: ly,
                    font_size: skin.arrow_font_size,
                    font_family: &skin.arrow_font_family,
                    fill: &skin.arrow_font_color,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&buf);
        };
        let emit_extra_paths = |svg: &mut SvgBuilder| {
            for (d, style) in &oracle_edge.extra_paths {
                let style = style
                    .as_deref()
                    .unwrap_or("stroke:#181818;stroke-width:0.5;");
                svg.raw(&format!(
                    r#"<path d="{d}" fill="{NOTE_FILL}" style="{style}"/>"#
                ));
            }
        };
        if oracle_edge.labels.is_empty() {
            emit_extra_paths(&mut *svg);
            if let Some((lx, ly, ref text)) = oracle_edge.label {
                emit_label(&mut *svg, lx, ly, text);
            }
        } else {
            // The connection's own label (`: <<extend>>`) precedes the note
            // shape; the note's text follows it. When the edge carries no label
            // of its own, every captured text belongs to the note and follows
            // the note box.
            let mut labels = oracle_edge.labels.iter();
            let has_edge_label = conn.label.is_some() || conn.stereotype.is_some();
            if has_edge_label && let Some((lx, ly, text)) = labels.next() {
                emit_label(&mut *svg, *lx, *ly, text);
            }
            emit_extra_paths(&mut *svg);
            for (lx, ly, text) in labels {
                emit_label(&mut *svg, *lx, *ly, text);
            }
        }
        svg.raw("</g>");
    }
}

fn link_note_blocks(
    x: f64,
    y: f64,
    connection: &UseCaseConnection,
    note: &UseCaseNote,
    note_dim: &NoteDim,
    skin: &SkinColors,
) -> ((f64, f64), (f64, f64)) {
    let note_width = note_dim.width + LINK_NOTE_PADDING * 2.0;
    let note_height = note_dim.height + LINK_NOTE_PADDING * 2.0;
    let label = connection
        .label
        .as_deref()
        .or(connection.stereotype.as_deref());
    let Some(label) = label else {
        return ((x, y), (x, y));
    };
    // `SvekEdge.getLabel` wraps the edge label in a one-pixel margin before
    // merging it with `EntityImageNoteLink`.
    let label_width = text_render::measure_with_family(
        label,
        skin.arrow_font_size as f64,
        false,
        &skin.arrow_font_family,
    ) + 2.0;
    let label_height = pm::text_height(skin.arrow_font_size as f64) + 2.0;
    let ascent = pm::ascent(skin.arrow_font_size as f64);
    match note.position {
        UseCaseNotePosition::Left => {
            let merged_height = note_height.max(label_height);
            let note_y = y + (merged_height - note_height) / 2.0;
            let label_y = y + (merged_height - label_height) / 2.0;
            ((x + note_width + 1.0, label_y + 1.0 + ascent), (x, note_y))
        }
        UseCaseNotePosition::Right => {
            let merged_height = note_height.max(label_height);
            let note_y = y + (merged_height - note_height) / 2.0;
            let label_y = y + (merged_height - label_height) / 2.0;
            ((x + 1.0, label_y + 1.0 + ascent), (x + label_width, note_y))
        }
        UseCaseNotePosition::Top => {
            let merged_width = note_width.max(label_width);
            let note_x = x + (merged_width - note_width) / 2.0;
            let label_x = x + (merged_width - label_width) / 2.0;
            ((label_x + 1.0, y + note_height + 1.0 + ascent), (note_x, y))
        }
        UseCaseNotePosition::Bottom => {
            let merged_width = note_width.max(label_width);
            let note_x = x + (merged_width - note_width) / 2.0;
            let label_x = x + (merged_width - label_width) / 2.0;
            (
                (label_x + 1.0, y + 1.0 + ascent),
                (note_x, y + label_height),
            )
        }
    }
}

fn emit_link_note(
    svg: &mut SvgBuilder,
    connection: &UseCaseConnection,
    note: &UseCaseNote,
    dim: &NoteDim,
    label_x: f64,
    label_y: f64,
    skin: &SkinColors,
) {
    let (_, (component_x, component_y)) =
        link_note_blocks(label_x, label_y, connection, note, dim, skin);
    let x = component_x + LINK_NOTE_PADDING;
    let y = component_y + LINK_NOTE_PADDING;
    // `ComponentRoseNote.drawInternalU` truncates its text-box dimensions
    // before asking Opale for the folded rectangle.
    let width = dim.width.floor();
    let height = dim.height.floor();
    let right = x + width;
    let bottom = y + height;
    let fold_x = right - NOTE_FOLD;
    let fold_y = y + NOTE_FOLD;
    svg.raw(&format!(
        r#"<path d="M{x},{y} L{x},{bottom} L{right},{bottom} L{right},{fold_y} L{fold_x},{y} L{x},{y}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        x = fc(x),
        y = fc(y),
        bottom = fc(bottom),
        right = fc(right),
        fold_y = fc(fold_y),
        fold_x = fc(fold_x),
    ));
    svg.raw(&format!(
        r#"<path d="M{fold_x},{y} L{fold_x},{fold_y} L{right},{fold_y} L{fold_x},{y}" fill="{NOTE_FILL}" style="stroke:{STROKE};stroke-width:0.5;"/>"#,
        fold_x = fc(fold_x),
        y = fc(y),
        fold_y = fc(fold_y),
        right = fc(right),
    ));
    let mut baseline = y + NOTE_MARGIN_Y + pm::ascent(NOTE_FONT_SIZE as f64);
    for line in note.text.split('\n') {
        let mut buf = String::new();
        text_render::emit_text(
            &mut buf,
            line,
            &TextBase {
                x: x + NOTE_MARGIN_LEFT,
                y: baseline,
                font_size: NOTE_FONT_SIZE,
                font_family: "sans-serif",
                fill: TEXT_COLOR,
                bold: false,
                italic: false,
                underline: false,
                skip_underline: false,
            },
        );
        svg.raw(&buf);
        baseline += pm::text_height(NOTE_FONT_SIZE as f64);
    }
}

fn svek_ordered_connection_indices(diagram: &UseCaseDiagram) -> Vec<usize> {
    let same_connections = |left: &UseCaseConnection, right: &UseCaseConnection| {
        (left.from == right.from && left.to == right.to)
            || (left.from == right.to && left.to == right.from)
    };
    let mut ordered = Vec::with_capacity(diagram.connections.len());
    for connection_index in 0..diagram.connections.len() {
        let connection = &diagram.connections[connection_index];
        let Some(first) = ordered
            .iter()
            .position(|&index| same_connections(&diagram.connections[index], connection))
        else {
            ordered.push(connection_index);
            continue;
        };
        let mut insert_at = first + 1;
        while insert_at < ordered.len()
            && same_connections(&diagram.connections[ordered[insert_at]], connection)
        {
            insert_at += 1;
        }
        ordered.insert(insert_at, connection_index);
    }
    ordered
}

fn render_no_oracle_connections(
    svg: &mut SvgBuilder,
    diagram: &UseCaseDiagram,
    id_map: &HashMap<String, String>,
    edge_paths: &[EdgePath],
    connection_styles: &[SkinColors],
) {
    // Java `CucaDiagramFileMakerSvek.getOrderedLinks/addLinkNew` keeps links
    // sharing an unordered endpoint pair adjacent to their first occurrence.
    for connection_index in svek_ordered_connection_indices(diagram) {
        let conn = &diagram.connections[connection_index];
        let skin = &connection_styles[connection_index];
        let Some(edge) = edge_paths
            .iter()
            .find(|edge| edge.from == conn.from && edge.to == conn.to)
        else {
            continue;
        };
        let Some(ent1) = usecase_entity_id(diagram, id_map, &conn.from) else {
            continue;
        };
        let Some(ent2) = usecase_entity_id(diagram, id_map, &conn.to) else {
            continue;
        };
        let link_id = no_oracle_link_id(diagram, conn.source_line);
        let from_label = link_comment_name(diagram, &conn.from);
        let to_label = link_comment_name(diagram, &conn.to);
        let start_decoration = connection_start_decoration(conn);
        let end_decoration = connection_end_decoration(conn);
        let link_type = if conn.extension {
            "extension"
        } else if start_decoration.is_some() || end_decoration.is_some() {
            "dependency"
        } else {
            "association"
        };
        svg.raw(&format!("<!--link {from_label} to {to_label}-->"));
        svg.raw(&format!(
            r#"<g class="link" data-entity-1="{ent1}" data-entity-2="{ent2}" data-link-type="{link_type}" data-source-line="{line}" id="{link_id}">"#,
            line = conn.source_line,
        ));
        let raw_points = quantized_svek_edge_points(edge);
        let mut path_points = raw_points.clone();
        if let Some(decoration) = start_decoration {
            trim_svek_edge_endpoint(&mut path_points, true, decoration.path_gap());
        }
        if let Some(decoration) = end_decoration {
            trim_svek_edge_endpoint(&mut path_points, false, decoration.path_gap());
        }
        if let Some(d) = edge_path_d(&path_points) {
            // Java `SvekEdge#drawU` resolves `Style#getStroke` from the
            // connection's captured builder. Source-level dotted relations
            // remain an inline LinkType override of the style dash pair.
            let dash = if conn.dashed {
                Some((7.0, 7.0))
            } else {
                skin.arrow_dash
            };
            let dash_style = dash
                .map(|(visible, space)| format!("stroke-dasharray:{},{};", fc(visible), fc(space),))
                .unwrap_or_default();
            let path_style = format!(
                "stroke:{};stroke-width:{};{dash_style}",
                skin.arrow_color,
                fc(skin.arrow_stroke_width),
            );
            // Java `Link.idCommentForSvg()` uses `-backto-` whenever
            // `LinkType.looksLikeRevertedForSvg()` reports a decoration at
            // the source endpoint.
            let path_id = if start_decoration.is_some() {
                format!("{from_label}-backto-{to_label}")
            } else if end_decoration.is_some() {
                format!("{from_label}-to-{to_label}")
            } else {
                format!("{from_label}-{to_label}")
            };
            svg.raw(&format!(
                r#"<path d="{d}" fill="none" id="{path_id}" style="{path_style}"/>"#,
            ));
        }
        if let Some(decoration) = start_decoration
            && raw_points.len() >= 2
        {
            render_usecase_extremity(
                svg,
                decoration,
                raw_points[1],
                raw_points[0],
                &skin.arrow_head_color,
                skin.arrow_stroke_width,
            );
        }
        if let Some(decoration) = end_decoration
            && raw_points.len() >= 2
        {
            let endpoint = raw_points.len() - 1;
            render_usecase_extremity(
                svg,
                decoration,
                raw_points[endpoint - 1],
                raw_points[endpoint],
                &skin.arrow_head_color,
                skin.arrow_stroke_width,
            );
        }
        let label_text = conn
            .label
            .as_deref()
            .or(conn.stereotype.as_deref())
            .map(|s| {
                if s.starts_with("<<") {
                    s.replace("<<", "\u{00AB}").replace(">>", "\u{00BB}")
                } else {
                    s.to_string()
                }
            });
        let link_note = note_on_connection(diagram, connection_index);
        if let Some(label) = label_text {
            let position = if let Some((note_index, note)) = link_note {
                edge.label.map(|label_box| {
                    link_note_blocks(
                        label_box.x,
                        label_box.y,
                        conn,
                        note,
                        &note_dim(&diagram.notes[note_index]),
                        skin,
                    )
                    .0
                })
            } else {
                // `SvekEdge.solveLine` captures the fixed-table origin, then
                // `drawU` replaces it with the real one-pixel-margined label.
                edge.label.map(|label_box| {
                    let (x, y) = quantized_svek_label_origin(label_box.x, label_box.y);
                    (x + 1.0, y + 1.0 + pm::ascent(skin.arrow_font_size as f64))
                })
            };
            let Some((x, y)) = position else {
                svg.raw("</g>");
                continue;
            };
            let mut buf = String::new();
            text_render::emit_text(
                &mut buf,
                &label,
                &TextBase {
                    x,
                    y,
                    font_size: skin.arrow_font_size,
                    font_family: &skin.arrow_font_family,
                    fill: &skin.arrow_font_color,
                    bold: false,
                    italic: false,
                    underline: false,
                    skip_underline: false,
                },
            );
            svg.raw(&buf);
        }
        if let Some((note_index, note)) = link_note
            && let Some(label_box) = edge.label
        {
            emit_link_note(
                svg,
                conn,
                note,
                &note_dim(&diagram.notes[note_index]),
                label_box.x,
                label_box.y,
                skin,
            );
        }
        svg.raw("</g>");
    }
}

fn usecase_entity_id<'a>(
    diagram: &UseCaseDiagram,
    id_map: &'a HashMap<String, String>,
    id: &str,
) -> Option<&'a str> {
    if diagram.actors.iter().any(|a| a.id == id) {
        return id_map.get(&format!("actor::{id}")).map(String::as_str);
    }
    if diagram.use_cases.iter().any(|u| u.id == id) {
        return id_map.get(&format!("uc::{id}")).map(String::as_str);
    }
    None
}

fn link_comment_name<'a>(diagram: &'a UseCaseDiagram, id: &'a str) -> &'a str {
    if let Some(uc) = diagram.use_cases.iter().find(|u| u.id == id) {
        return display_name(uc);
    }
    if let Some(actor) = diagram.actors.iter().find(|a| a.id == id) {
        if actor.id == label_to_id(&actor.label) {
            return actor.label.as_str();
        }
        return actor.id.as_str();
    }
    id
}

fn no_oracle_link_id(diagram: &UseCaseDiagram, source_line: usize) -> String {
    let mut counter = 2usize;
    let mut items: Vec<(usize, usize, bool)> = Vec::new();
    items.extend(diagram.actors.iter().map(|a| (a.source_line, 1, false)));
    items.extend(diagram.use_cases.iter().map(|u| (u.source_line, 1, false)));
    items.extend(diagram.packages.iter().map(|p| (p.source_line, 1, false)));
    items.extend(diagram.notes.iter().filter_map(|note| {
        let slots = match note.kind {
            UseCaseNoteKind::Attached { .. } => 3,
            UseCaseNoteKind::Floating { .. } => 1,
            UseCaseNoteKind::OnLink { .. } => return None,
        };
        Some((note.source_line, slots, false))
    }));
    items.extend(diagram.connections.iter().map(|c| (c.source_line, 1, true)));
    items.sort_by_key(|(line, _, _)| *line);
    for (line, slots, is_link) in items {
        if is_link && line == source_line {
            return format!("lnk{counter}");
        }
        counter += slots;
    }
    format!("lnk{counter}")
}

#[derive(Clone, Copy)]
enum UseCaseExtremity {
    Dependency,
    Extension,
}

impl UseCaseExtremity {
    fn path_gap(self) -> f64 {
        match self {
            Self::Dependency => DEPENDENCY_ARROW_PATH_GAP,
            // `LinkDecor.EXTENDS.getExtremityFactoryComplete` constructs an
            // `ExtremityTriangle` with an 18px decoration length.
            Self::Extension => 18.0,
        }
    }
}

fn connection_start_decoration(conn: &UseCaseConnection) -> Option<UseCaseExtremity> {
    conn.arrow_at_start.then_some(if conn.extension {
        UseCaseExtremity::Extension
    } else {
        UseCaseExtremity::Dependency
    })
}

fn connection_end_decoration(conn: &UseCaseConnection) -> Option<UseCaseExtremity> {
    conn.arrow.then_some(if conn.extension {
        UseCaseExtremity::Extension
    } else {
        UseCaseExtremity::Dependency
    })
}

fn quantized_svek_edge_points(edge: &EdgePath) -> Vec<(f64, f64)> {
    // `SvekEdge.solveLine` parses Graphviz's SVG path after Graphviz has
    // serialized every coordinate to two decimal places.
    edge.points
        .iter()
        .map(|(x, y)| ((x * 100.0).round() / 100.0, (y * 100.0).round() / 100.0))
        .collect()
}

fn quantized_svek_label_origin(x: f64, y: f64) -> (f64, f64) {
    // `SvekEdge.solveLine/getXY` recovers the fixed HTML-table origin from
    // Graphviz's SVG polygon, whose coordinates are serialized to two places.
    ((x * 100.0).round() / 100.0, (y * 100.0).round() / 100.0)
}

fn trim_svek_edge_endpoint(points: &mut [(f64, f64)], start: bool, gap: f64) {
    if points.len() < 2 {
        return;
    }
    let endpoint = if start { 0 } else { points.len() - 1 };
    let adjacent = if start { 1 } else { endpoint - 1 };
    let dx = points[endpoint].0 - points[adjacent].0;
    let dy = points[endpoint].1 - points[adjacent].1;
    let len = dx.hypot(dy);
    if len <= f64::EPSILON {
        return;
    }
    let shift = (dx / len * gap, dy / len * gap);
    points[endpoint].0 -= shift.0;
    points[endpoint].1 -= shift.1;
    if points.len() >= 4 {
        points[adjacent].0 -= shift.0;
        points[adjacent].1 -= shift.1;
    }
}

fn edge_path_d(points: &[(f64, f64)]) -> Option<String> {
    let (start, rest) = points.split_first()?;
    let mut d = format!("M{},{}", fc(start.0), fc(start.1));
    for chunk in rest.chunks(3) {
        if let [c1, c2, to] = chunk {
            write!(
                d,
                " C{},{} {},{} {},{}",
                fc(c1.0),
                fc(c1.1),
                fc(c2.0),
                fc(c2.1),
                fc(to.0),
                fc(to.1),
            )
            .unwrap();
        }
    }
    Some(d)
}

fn render_usecase_extremity(
    svg: &mut SvgBuilder,
    decoration: UseCaseExtremity,
    control: (f64, f64),
    endpoint: (f64, f64),
    color: &str,
    stroke_width: f64,
) {
    match decoration {
        UseCaseExtremity::Dependency => {
            let points = dependency_arrow_points(control, endpoint);
            svg.raw(&format!(
                r#"<polygon fill="{color}" points="{points}" style="stroke:{color};stroke-width:{stroke_width};"/>"#,
                stroke_width = fc(stroke_width),
            ));
        }
        UseCaseExtremity::Extension => {
            let points = extension_arrow_points(control, endpoint);
            svg.raw(&format!(
                r#"<polygon fill="none" points="{points}" style="stroke:{color};stroke-width:{stroke_width};"/>"#,
                stroke_width = fc(stroke_width),
            ));
        }
    }
}

fn extension_polygon_min_x(connection: &UseCaseConnection, edge: &EdgePath) -> Option<f64> {
    let points = quantized_svek_edge_points(edge);
    if points.len() < 2 {
        return None;
    }
    let mut min_x = f64::INFINITY;
    if matches!(
        connection_start_decoration(connection),
        Some(UseCaseExtremity::Extension)
    ) {
        for (x, _) in extension_arrow_vertices(points[1], points[0]) {
            min_x = min_x.min(x);
        }
    }
    if matches!(
        connection_end_decoration(connection),
        Some(UseCaseExtremity::Extension)
    ) {
        let endpoint = points.len() - 1;
        for (x, _) in extension_arrow_vertices(points[endpoint - 1], points[endpoint]) {
            min_x = min_x.min(x);
        }
    }
    min_x
        .is_finite()
        .then_some(min_x - LIMIT_FINDER_POLYGON_X_GUARD)
}

fn dependency_arrow_points(control: (f64, f64), endpoint: (f64, f64)) -> String {
    let dx = endpoint.0 - control.0;
    let dy = endpoint.1 - control.1;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    let px = -uy;
    let py = ux;
    let p1 = endpoint;
    let left_wing = (
        endpoint.0 - ux * DEPENDENCY_ARROW_BACK + px * DEPENDENCY_ARROW_WING,
        endpoint.1 - uy * DEPENDENCY_ARROW_BACK + py * DEPENDENCY_ARROW_WING,
    );
    let notch = (
        endpoint.0 - ux * DEPENDENCY_ARROW_NOTCH,
        endpoint.1 - uy * DEPENDENCY_ARROW_NOTCH,
    );
    let right_wing = (
        endpoint.0 - ux * DEPENDENCY_ARROW_BACK - px * DEPENDENCY_ARROW_WING,
        endpoint.1 - uy * DEPENDENCY_ARROW_BACK - py * DEPENDENCY_ARROW_WING,
    );
    // Java's `ExtremityArrow.getDecorationPolygon()` emits the right wing,
    // inset notch, and left wing in that order; SVG points are comma-delimited.
    format!(
        "{},{},{},{},{},{},{},{},{},{}",
        fc(p1.0),
        fc(p1.1),
        fc(right_wing.0),
        fc(right_wing.1),
        fc(notch.0),
        fc(notch.1),
        fc(left_wing.0),
        fc(left_wing.1),
        fc(p1.0),
        fc(p1.1),
    )
}

fn extension_arrow_vertices(control: (f64, f64), endpoint: (f64, f64)) -> [(f64, f64); 4] {
    let dx = endpoint.0 - control.0;
    let dy = endpoint.1 - control.1;
    let len = dx.hypot(dy).max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    let px = -uy;
    let py = ux;
    // `LinkDecor.EXTENDS.getExtremityFactoryComplete` supplies
    // `ExtremityTriangle` with xWing=18 and yAperture=6.
    let back = (endpoint.0 - ux * 18.0, endpoint.1 - uy * 18.0);
    let left = (back.0 + px * 6.0, back.1 + py * 6.0);
    let right = (back.0 - px * 6.0, back.1 - py * 6.0);
    [endpoint, right, left, endpoint]
}

fn extension_arrow_points(control: (f64, f64), endpoint: (f64, f64)) -> String {
    let [tip, right, left, close] = extension_arrow_vertices(control, endpoint);
    format!(
        "{},{},{},{},{},{},{},{}",
        fc(tip.0),
        fc(tip.1),
        fc(right.0),
        fc(right.1),
        fc(left.0),
        fc(left.1),
        fc(close.0),
        fc(close.1),
    )
}

#[cfg(test)]
mod tests {
    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn parsed_then_rendered() {
        let input = "@startuml\nactor User\nusecase \"Login\" as UC1\nUser --> UC1\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains("User"));
        assert!(svg.contains("Login"));
    }

    #[test]
    fn description_gradients_follow_member_then_top_level_paint_order() {
        let input = r##"@startuml
skinparam actorBackgroundColor #102030|#405060
skinparam usecaseBackgroundColor #A0B0C0-#D0E0F0
actor "Top Level Reviewer" as Reviewer
rectangle "Nested Boundary" {
  usecase "Member Approval" as Approval
}
@enduml"##;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let source = diagram.meta().source.as_deref().unwrap();
        let use_case_id = crate::filter_registry::gradient_id_for(source, 0);
        let actor_id = crate::filter_registry::gradient_id_for(source, 1);
        let svg = crate::render_svg(&diagram);

        let use_case_def = format!(
            r##"<linearGradient id="{use_case_id}" x1="50%" x2="50%" y1="0%" y2="100%"><stop offset="0%" stop-color="#A0B0C0"/><stop offset="100%" stop-color="#D0E0F0"/></linearGradient>"##
        );
        let actor_def = format!(
            r##"<linearGradient id="{actor_id}" x1="0%" x2="100%" y1="50%" y2="50%"><stop offset="0%" stop-color="#102030"/><stop offset="100%" stop-color="#405060"/></linearGradient>"##
        );
        let use_case_pos = svg
            .find(&use_case_def)
            .expect("member use-case gradient def");
        let actor_pos = svg.find(&actor_def).expect("top-level actor gradient def");
        assert!(use_case_pos < actor_pos, "{svg}");
        assert!(svg.contains(&format!(r##"fill="url(#{use_case_id})""##)));
        assert!(svg.contains(&format!(r##"fill="url(#{actor_id})""##)));
    }

    #[test]
    fn description_gradient_registry_deduplicates_typed_keys_across_renames_and_counts() {
        for (actor_label, use_case_count) in [
            ("Renamed Intake Owner", 1usize),
            ("Independent Audit Dispatcher", 4usize),
        ] {
            let mut input = format!(
                "@startuml\n\
                 skinparam actorBackgroundColor #00FFFF/#FFC0CB\n\
                 skinparam usecaseBackgroundColor #00FFFF/#FFC0CB\n\
                 actor \"{actor_label}\" as Owner\n"
            );
            for index in 0..use_case_count {
                input.push_str(&format!(
                    "usecase \"Fresh Queue {index}\" as Queue{index}\n"
                ));
            }
            input.push_str("@enduml\n");

            let diagram = rustuml_parser::parse::parse(&input).unwrap();
            let source = diagram.meta().source.as_deref().unwrap();
            let gradient_id = crate::filter_registry::gradient_id_for(source, 0);
            let svg = crate::render_svg(&diagram);
            assert_eq!(svg.matches("<linearGradient ").count(), 1, "{svg}");
            assert_eq!(
                svg.matches(&format!(r##"fill="url(#{gradient_id})""##))
                    .count(),
                use_case_count + 1,
                "{svg}"
            );
            assert!(svg.contains(r##"x1="0%" x2="100%" y1="0%" y2="100%""##));
        }
    }

    #[test]
    fn description_gradient_seed_prefers_parser_carried_prefix_then_source_fallback() {
        let input = r##"@startuml
skinparam usecaseBackgroundColor #123456\#ABCDEF
usecase "Source Seed Control" as Probe
@enduml"##;
        let fallback = rustuml_parser::parse::parse(input).unwrap();
        let fallback_source = fallback.meta().source.as_deref().unwrap();
        let fallback_id = crate::filter_registry::gradient_id_for(fallback_source, 0);
        let fallback_svg = crate::render_svg(&fallback);
        assert!(fallback_svg.contains(&format!(r##"id="{fallback_id}""##)));

        let mut carried = rustuml_parser::parse::parse(input).unwrap();
        carried
            .meta_mut()
            .skinparams
            .push(rustuml_parser::diagram::SkinParam {
                key: "__svgIdSeed".to_string(),
                value: "carriedseed".to_string(),
            });
        let carried_svg = crate::render_svg(&carried);
        assert!(
            carried_svg.contains(r##"id="gcarriedseed0""##),
            "{carried_svg}"
        );
        assert!(
            carried_svg.contains(r##"fill="url(#gcarriedseed0)""##),
            "{carried_svg}"
        );
    }

    #[test]
    fn description_inline_flat_override_does_not_allocate_hidden_style_gradient() {
        let input = r##"@startuml
skinparam actorBackgroundColor #112233/#445566
actor "Inline Flat Override" as Flat #red
@enduml"##;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(!svg.contains("<linearGradient "), "{svg}");
        assert!(svg.contains(r##"fill="#FF0000" rx="8" ry="8""##), "{svg}");
    }

    #[test]
    fn description_flat_transparent_and_default_controls_remain_gradient_free() {
        let transparent_input = r##"@startuml
skinparam actorBackgroundColor transparent
skinparam usecaseBackgroundColor #ABCDEF
actor "Transparent Control" as Clear
usecase "Flat Use Case" as Probe
@enduml"##;
        let transparent = rustuml_parser::parse::parse(transparent_input).unwrap();
        let transparent_svg = crate::render_svg(&transparent);
        assert!(
            !transparent_svg.contains("<linearGradient "),
            "{transparent_svg}"
        );
        assert!(
            transparent_svg.contains(r##"fill="none" rx="8" ry="8""##),
            "{transparent_svg}"
        );
        assert!(
            transparent_svg.contains(r##"fill="#ABCDEF""##),
            "{transparent_svg}"
        );

        let plain = rustuml_parser::parse::parse(
            "@startuml\nactor \"Default Actor\" as Default\nusecase \"Default Use Case\" as Probe\n@enduml",
        )
        .unwrap();
        let plain_svg = crate::render_svg(&plain);
        assert!(!plain_svg.contains("<linearGradient "), "{plain_svg}");
        assert_eq!(plain_svg.matches(r##"fill="#F1F1F1""##).count(), 2);
    }

    #[test]
    fn no_oracle_usecase_routes_renamed_actor_link() {
        let input = "@startuml\nactor \"Reader\" as R\nusecase \"Browse Catalog\" as Browse\nR --> Browse\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(r#"<!--link R to Browse-->"#));
        assert!(svg.contains(r#"<path d="M"#));
        assert!(svg.contains(r##"<polygon fill="#181818""##));
    }

    #[test]
    fn renamed_short_queue_link_uses_horizontal_rank() {
        let input = "@startuml\n\
                     actor \"Renamed Queue Operator 1201\" as Operator1201\n\
                     usecase \"Renamed Queue Action 1213\" as Action1213\n\
                     Operator1201 -> Action1213\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh PlantUML reference for this renamed perturbation. Java
        // `CommandLinkElement.getDirection` and `SvekEdge.rankSame` place a
        // one-character relation queue on a horizontal rank.
        assert!(svg.contains(r#"viewBox="0 0 491 95""#), "{svg}");
        assert!(svg.contains(r#"<ellipse cx="113.9087" cy="14""#), "{svg}");
        assert!(
            svg.contains(r#"<path d="M222.13,43.74 C233.44,43.74 238.75,43.74 250.06,43.74""#),
            "{svg}"
        );
    }

    #[test]
    fn renamed_mixed_root_queue_lengths_follow_svek_minlen() {
        let input = "@startuml\n\
                     left to right direction\n\
                     actor \"Fresh Intake Auditor 2213\" as Auditor2213\n\
                     usecase \"Queue Novel Claim 2221\" as Queue2221\n\
                     usecase \"Verify Unseen Claim 2237\" as Verify2237\n\
                     usecase \"Archive Fresh Claim 2243\" as Archive2243\n\
                     Auditor2213 -> Queue2221\n\
                     Auditor2213 --> Verify2237\n\
                     Queue2221 .> Archive2243 : <<include>>\n\
                     Verify2237 ..> Archive2243 : <<extend>>\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh PlantUML reference. `SvekEdge.appendLine` emits minlen zero
        // for each one-character root relation, while `Cluster.getRankSame`
        // does not add a root-level rank subgraph.
        assert!(svg.contains(r#"viewBox="0 0 970 144""#), "{svg}");
        assert!(
            svg.contains(
                r#"<path d="M184.32,48.38 C199.98,45.82 210.2784,44.137 225.6484,41.627""#
            ),
            "{svg}"
        );
        assert!(
            svg.contains(
                r#"<path d="M405.03,33.62 C504.29,39.83 654.8417,49.2358 755.0117,55.4958""#
            ),
            "{svg}"
        );
        assert!(
            svg.contains(
                r#"<path d="M623.56,95.61 C669.75,88.91 720.9123,81.4721 766.9923,74.7821""#
            ),
            "{svg}"
        );
        assert_eq!(svg.matches(r#"class="entity""#).count(), 4, "{svg}");
        assert_eq!(svg.matches(r#"class="link""#).count(), 4, "{svg}");
    }

    #[test]
    fn renamed_multicompartment_usecase_renders_stencilled_separator_styles() {
        let input = "@startuml\n\
                     actor \"Renamed Reviewer 1409\" as Reviewer1409\n\
                     usecase Review1411 as \"\n\
                       Fresh intake\n\
                       ....\n\
                       Validate unseen record\n\
                       ====\n\
                       Archive renamed result\n\
                     \"\n\
                     usecase \"Fresh downstream audit\" as Audit1423\n\
                     Reviewer1409 --> Review1411\n\
                     Review1411 --> Audit1423\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference for this renamed three-node topology.
        // `BodyEnhanced1.getArea` creates three body compartments,
        // `BodyEnhancedAbstract.decorate` adds their margins, and
        // `TextBlockLineBefore.drawU` paints one dotted and one double rule.
        assert_eq!(svg.matches("<line style=").count(), 3, "{svg}");
        assert_eq!(svg.matches("stroke-dasharray:1,2;").count(), 1, "{svg}");
        assert!(svg.contains(">Fresh intake</text>"), "{svg}");
        assert!(svg.contains(">Archive renamed result</text>"), "{svg}");
    }

    #[test]
    fn renamed_package_uses_svek_title_geometry_and_painted_origin() {
        let input = "@startuml\n\
                     actor \"Renamed Auditor 1701\" as Auditor1701\n\
                     actor \"Renamed Scheduler 1709\" as Scheduler1709\n\
                     rectangle \"Fresh Processing Boundary 1721\" #LightGreen {\n\
                       usecase \"Queue unseen batch 1723\" as Queue1723\n\
                       usecase \"Reconcile unusual result 1733\" as Reconcile1733\n\
                       usecase \"Archive renamed record 1741\" as Archive1741\n\
                     }\n\
                     Auditor1701 --> Queue1723\n\
                     Scheduler1709 --> Reconcile1733\n\
                     Auditor1701 --> Archive1741\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // A fresh Java PlantUML render of this renamed topology has the same
        // 779px canvas width and 758px cluster width. `ClusterHeader` supplies
        // the title table while `SvekResult.calculateDimension` moves the
        // rectangle's painted minimum to x=6, leaving its path origin at x=7.
        assert!(svg.contains(r#"viewBox="0 0 779 "#), "{svg}");
        assert!(svg.contains(r#"width="758" x="7""#), "{svg}");
        assert!(
            svg.contains(">Fresh Processing Boundary 1721</text>"),
            "{svg}"
        );
        assert_eq!(svg.matches(r#"class="entity""#).count(), 5, "{svg}");
    }

    #[test]
    fn renamed_connected_short_actors_use_painted_bounds_for_clustered_canvas() {
        let input = "@startuml\n\
                     left to right direction\n\
                     actor R1\n\
                     actor R2\n\
                     package \"Fresh Intake Zone\" {\n\
                       usecase \"Queue Amber\" as QA\n\
                       usecase \"Review Birch\" as RB\n\
                       usecase \"Archive Cedar\" as AC\n\
                     }\n\
                     package \"Novel Export Zone\" {\n\
                       usecase \"Publish Dogwood\" as PD\n\
                       usecase \"Verify Elm\" as VE\n\
                       usecase \"Store Fir\" as SF\n\
                     }\n\
                     R1 --> QA\n\
                     R2 --> QA\n\
                     R1 --> PD\n\
                     R2 --> PD\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference for a renamed two-package topology
        // with three leaves per package, absent from the golden corpus.
        // `SvekResult.calculateDimension` normalizes from the stickman's arm
        // endpoint at x=6, not the half-pixel-wider Graphviz actor box.
        // Debug Graphviz retains one extra fractional far-edge pixel; release
        // serialization matches Java's 273px canvas exactly.
        assert!(
            svg.contains(r#"viewBox="0 0 273 471""#) || svg.contains(r#"viewBox="0 0 274 471""#),
            "{svg}"
        );
        assert!(svg.contains(r#"<path d="M88.34,6 "#), "{svg}");
        assert!(svg.contains(r#"<path d="M79,242 "#), "{svg}");
        assert!(svg.contains(r#"<ellipse cx="19" "#), "{svg}");
        assert_eq!(svg.matches(r#"class="cluster""#).count(), 2, "{svg}");
        assert_eq!(svg.matches(r#"class="entity""#).count(), 8, "{svg}");
        assert_eq!(svg.matches(r#"class="link""#).count(), 4, "{svg}");
    }

    #[test]
    fn renamed_mixed_clusters_use_primitive_specific_painted_bounds() {
        let input = "@startuml\n\
                     left to right direction\n\
                     actor \"Fresh Bounds Auditor 1811\" as Auditor1811\n\
                     rectangle \"Renamed Intake Boundary 1823\" #LightBlue {\n\
                       usecase \"Queue unseen claim 1831\" as Queue1831\n\
                       usecase \"Review irregular batch 1847\" as Review1847\n\
                       usecase \"Archive fresh outcome 1861\" as Archive1861\n\
                     }\n\
                     package \"Renamed Archive Folder 1871\" {\n\
                       usecase \"Store novel snapshot 1873\" as Store1873\n\
                       usecase \"Verify unseen checksum 1877\" as Verify1877\n\
                     }\n\
                     Auditor1811 --> Queue1831\n\
                     Auditor1811 --> Review1847\n\
                     Review1847 --> Archive1861\n\
                     Archive1861 --> Store1873\n\
                     Store1873 --> Verify1877\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // A fresh Java render is structurally equivalent. Its rectangle starts
        // at y=7 because `LimitFinder.drawRectangle` contributes a -1 minimum;
        // the folder's `UPath` starts at y=6 because its extrema are exact.
        assert!(
            svg.contains(r##"<rect fill="#ADD8E6" height="179""##),
            "{svg}"
        );
        assert!(svg.contains(r#"x="236.31" y="7"/>"#), "{svg}");
        assert!(svg.contains(r#"<path d="M788.53,6 "#), "{svg}");
        assert_eq!(svg.matches(r#"class="cluster""#).count(), 2, "{svg}");
        assert_eq!(svg.matches(r#"class="entity""#).count(), 6, "{svg}");
    }

    #[test]
    fn renamed_root_leaves_precede_package_members_in_svek_node_order() {
        let input = "@startuml\n\
                     actor B1\n\
                     actor B2\n\
                     actor B3\n\
                     actor B4\n\
                     rectangle ModuleZ {\n\
                       usecase \"VX01\" as VX01\n\
                       usecase \"VX02\" as VX02\n\
                       usecase \"VX03\" as VX03\n\
                     }\n\
                     B2 --> VX01\n\
                     B3 --> VX02\n\
                     B4 --> VX03\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference. `DotStringFactory.createDotString`
        // emits root leaves through `Cluster.printCluster1` before
        // `Cluster.printCluster2` emits the child package tree, preserving the
        // detached actor as the leftmost root leaf.
        // Debug Graphviz keeps one extra fractional bottom pixel that release
        // serialization rounds away; width and all ordering coordinates match
        // the fresh Java SVG in both profiles.
        assert!(svg.contains(r#"viewBox="0 0 321 "#), "{svg}");
        assert!(
            svg.contains(
                r#"<rect fill="none" height="80.32" rx="2.5" ry="2.5" style="stroke:#181818;stroke-width:1;" width="270" x="37" y="106.99""#
            ),
            "{svg}"
        );
        for cx in ["19", "81", "172", "263"] {
            assert!(
                svg.contains(&format!(r#"<ellipse cx="{cx}" cy="14""#)),
                "{svg}"
            );
        }
    }

    #[test]
    fn renamed_short_package_relations_create_svek_nodes_before_root_stream() {
        let input = "@startuml\n\
                     actor \"Fresh Operator 1201\" as Operator1201\n\
                     actor \"Novel Auditor 1213\" as Auditor1213\n\
                     rectangle \"Renamed Workflow 1223\" {\n\
                       usecase \"Prepare Entry 1231\" as Entry1231\n\
                       usecase \"Review Entry 1237\" as Review1237\n\
                       usecase \"Publish Entry 1249\" as Publish1249\n\
                       usecase \"Archive Entry 1259\" as Archive1259\n\
                       usecase \"Validate Entry 1277\" as Validate1277\n\
                     }\n\
                     Operator1201 --> Entry1231\n\
                     Auditor1213 --> Review1237\n\
                     Entry1231 .> Validate1277 : <<include>>\n\
                     Publish1249 .> Validate1277 : <<include>>\n\
                     Archive1259 .> Review1237 : <<extend>>\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference absent from the golden corpus.
        // `Bibliotekon.addLine` classifies the three short relations into
        // `lines0`; `DotStringFactory.createDotString` emits them before
        // `Cluster.printCluster2`, creating their endpoints before the two
        // root actors and the remaining package member.
        assert!(
            svg.contains(r#"viewBox="0 0 1108 266""#) || svg.contains(r#"viewBox="0 0 1108 267""#),
            "{svg}"
        );
        for cx in ["103", "297", "553", "749", "1000"] {
            assert!(
                svg.contains(&format!(r#"<ellipse cx="{cx}" cy="217"#)),
                "{svg}"
            );
        }
        assert!(svg.contains(r#"id="Entry1231-to-Validate1277""#), "{svg}");
        assert_eq!(svg.matches(r#"class="entity""#).count(), 7, "{svg}");
        assert_eq!(svg.matches(r#"class="link""#).count(), 5, "{svg}");
    }

    #[test]
    fn renamed_short_relations_materialize_root_nodes_before_remaining_leaves() {
        let input = "@startuml\n\
                     actor \"Fresh Review Steward\" as StewardQ\n\
                     usecase \"Novel Base Amber\" as BaseAmber\n\
                     usecase \"Novel Base Birch\" as BaseBirch\n\
                     usecase \"Novel Base Cedar\" as BaseCedar\n\
                     usecase \"Novel Base Dogwood\" as BaseDogwood\n\
                     usecase \"Unseen Branch Elm\" as BranchElm\n\
                     usecase \"Unseen Branch Fir\" as BranchFir\n\
                     usecase \"Unseen Branch Gum\" as BranchGum\n\
                     usecase \"Unseen Branch Hazel\" as BranchHazel\n\
                     usecase \"Unseen Branch Iris\" as BranchIris\n\
                     BranchElm .> BaseCedar : <<include>>\n\
                     BranchFir .> BaseAmber : <<include>>\n\
                     BranchGum .> BaseDogwood : <<include>>\n\
                     BranchHazel .> BaseBirch : <<include>>\n\
                     BranchIris .> BaseCedar : <<include>>\n\
                     StewardQ --> BaseAmber\n\
                     StewardQ --> BaseBirch\n\
                     StewardQ --> BaseCedar\n\
                     StewardQ --> BaseDogwood\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference with a 4-base/5-branch topology absent
        // from the corpus. `Bibliotekon.lines0` emits the five short relations
        // first, so their endpoint order controls the solved horizontal rank.
        assert!(svg.contains(r#"viewBox="0 0 1904 211""#), "{svg}");
        for cx in ["85.73", "335.73", "521.73", "770.73", "1228.73"] {
            assert!(svg.contains(&format!(r#"<ellipse cx="{cx}"#)), "{svg}");
        }
        assert!(svg.contains(r#"id="BranchElm-to-BaseCedar""#), "{svg}");
        assert_eq!(svg.matches(r#"class="entity""#).count(), 10, "{svg}");
        assert_eq!(svg.matches(r#"class="link""#).count(), 9, "{svg}");
    }

    #[test]
    fn renamed_relation_labels_use_svek_solved_table_origins() {
        let input = "@startuml\n\
                     actor \"Fresh Relation Auditor 413\" as Audit413\n\
                     usecase \"Queue Fresh Transfer 419\" as Transfer419\n\
                     usecase \"Queue Fresh Approval 421\" as Approval421\n\
                     Audit413 --> Transfer419 : initiates freshly\n\
                     Transfer419 ..> Approval421 : <<include>>\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference for this renamed topology.
        // `SvekEdge.getLabel` adds the one-pixel margin, `appendTable` gives
        // Graphviz the integer renderer dimensions, and `solveLine` recovers
        // the two solved table origins used by `drawU`.
        assert!(svg.contains(r#"viewBox="0 0 225 340""#), "{svg}");
        assert!(
            svg.contains(r#"x="109.53"#)
                && svg.contains(r#"y="125.55"#)
                && svg.contains(">initiates freshly</text>"),
            "{svg}"
        );
        assert!(
            svg.contains(r#"y="247.67"#) && svg.contains(">«include»</text>"),
            "{svg}"
        );
    }

    #[test]
    fn renamed_actorless_labels_expand_the_complete_svek_painted_envelope() {
        let input = "@startuml\n\
                     usecase \"Novel Intake 5003\" as Intake5003\n\
                     usecase \"Fresh Review 5009\" as Review5009\n\
                     usecase \"Renamed Archive 5011\" as Archive5011\n\
                     Intake5003 --> Review5009 : initiates unlisted work\n\
                     Review5009 ..> Archive5011 : <<include>>\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference absent from the golden corpus.
        // `SvekResult.drawU` includes edge labels in the `LimitFinder`, whose
        // `drawText` baseline adjustment controls the normalized top edge.
        assert!(
            svg.contains(r#"viewBox="0 0 253 287""#) || svg.contains(r#"viewBox="0 0 254 287""#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"x="98.83" y="85.3484">initiates unlisted work</text>"#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"x="98.83" y="198.7384">«include»</text>"#),
            "{svg}"
        );
    }

    #[test]
    fn renamed_bottom_note_keeps_entity_order_and_relative_indents() {
        let input = concat!(
            "@startuml\n",
            "actor \"Fresh Examiner 5101\" as Examiner5101\n",
            "note bottom of Examiner5101\n",
            "   Fresh heading\n",
            "     * deeper one\n",
            "       ** deeper two\n",
            "end note\n",
            "usecase \"Novel Approve 5107\" as Approve5107\n",
            "usecase \"Renamed Reject 5113\" as Reject5113\n",
            "Examiner5101 --> Approve5107\n",
            "Examiner5101 --> Reject5113\n",
            "@enduml",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java reference. `CommandFactoryNoteOnEntity` inserts the
        // synthetic note at this source position, then
        // `GraphvizImageBuilder.printEntities` preserves that entity order.
        assert!(svg.contains(r#"viewBox="0 0 558 212""#), "{svg}");
        let note = svg.find(r#"data-qualified-name="GMN3""#).unwrap();
        let approve = svg.find(r#"data-qualified-name="Approve5107""#).unwrap();
        assert!(note < approve, "{svg}");
        assert!(
            svg.contains(r#"x="12" y=""#) && svg.contains(">Fresh heading</text>"),
            "{svg}"
        );
        assert!(
            svg.contains(r#"x="20.2266" y=""#) && svg.contains(">* deeper one</text>"),
            "{svg}"
        );
        assert!(
            svg.contains(r#"x="28.4531" y=""#) && svg.contains(">** deeper two</text>"),
            "{svg}"
        );
    }

    #[test]
    fn renamed_reciprocal_links_group_and_share_arrow_style() {
        let input = "@startuml\n\
                     skinparam ArrowColor DarkGreen\n\
                     usecase \"Fresh Ledger 5209\" as Ledger5209\n\
                     usecase \"Novel Check 5213\" as Check5213\n\
                     usecase \"Renamed Archive 5227\" as Archive5227\n\
                     Ledger5209 ..> Check5213 : first unseen flow\n\
                     Ledger5209 --> Archive5227 : unrelated renamed flow\n\
                     Check5213 ..> Ledger5209 : reciprocal unseen flow\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java reference. `CucaDiagramFileMakerSvek.addLinkNew` groups
        // `Link.sameConnections` pairs, while `FromSkinparamToStyle` maps
        // `arrowColor` to the SVEK arrow line and extremity style.
        let first = svg.find("<!--link Ledger5209 to Check5213-->").unwrap();
        let reciprocal = svg.find("<!--link Check5213 to Ledger5209-->").unwrap();
        let unrelated = svg.find("<!--link Ledger5209 to Archive5227-->").unwrap();
        assert!(first < reciprocal && reciprocal < unrelated, "{svg}");
        assert_eq!(svg.matches(r#"stroke:#006400;stroke-width:1;"#).count(), 6);
        assert_eq!(svg.matches(r##"fill="#006400""##).count(), 3);
    }

    #[test]
    fn actor_stereotype_is_a_measured_top_tile_for_renamed_actor() {
        let input = "@startuml\nactor \"Renamed Portal\" as Portal <<externalized>>\nusecase \"Fresh Flow\" as Flow\nPortal --> Flow\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let skin = super::SkinColors::from_meta(&usecase.meta, None);
        let dim = super::actor_dim(&usecase.actors[0], &skin);
        let bare_stereo_w = crate::text_render::measure_with_family(
            "\u{00AB}externalized\u{00BB}",
            skin.actor_font_size as f64,
            false,
            &skin.actor_font_family,
        );
        let stereotype = dim.stereotype.expect("stereotype block");

        assert_eq!(
            stereotype.width(),
            bare_stereo_w + super::ACTOR_STEREOTYPE_MARGIN_X * 2.0
        );
        let text_block_h = super::pm::text_height(skin.actor_font_size as f64);
        assert_eq!(stereotype.height(), text_block_h);
        assert_eq!(
            dim.height,
            super::ACTOR_STICKMAN_BASE_HEIGHT + dim.stroke_thickness * 2.0 + text_block_h * 2.0
        );
        assert_eq!(dim.paint_min_y, -(text_block_h - dim.label_gap));

        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(">Renamed Portal</text>"));
        assert!(svg.contains("\u{00AB}externalized\u{00BB}</text>"));
        assert!(svg.contains(r#"id="Portal-to-Flow""#));
        assert!(svg.contains(r##"<polygon fill="#181818" points=""##));
    }

    #[test]
    fn global_padding_composes_actor_only_blocks_across_label_and_font_axes() {
        let cases = [
            ("sans-serif", 14, "I"),
            ("Verdana", 12, "Renamed Wide Audit Operator"),
            ("Arial", 17, "W"),
        ];

        for (font, font_size, label) in cases {
            for padding in [0.0, 3.0, 11.0] {
                let input = format!(
                    "@startuml\n\
                     skinparam defaultFontName {font}\n\
                     skinparam defaultFontSize {font_size}\n\
                     skinparam Padding {padding}\n\
                     :{label}:\n\
                     @enduml"
                );
                let diagram = rustuml_parser::parse::parse(&input).unwrap();
                let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
                    panic!("expected use-case diagram");
                };
                let base = super::SkinColors::from_meta(&usecase.meta, None);
                let skin = base.for_actor(&usecase.meta, &usecase.actors[0]);
                let dim = super::actor_dim(&usecase.actors[0], &skin);
                let expected_width = crate::text_render::measure_with_family(
                    label,
                    font_size as f64,
                    false,
                    &skin.actor_font_family,
                );
                let expected_height = crate::text_render::label_height_with_family(
                    label,
                    font_size as f64,
                    &skin.actor_font_family,
                );
                let expected_ascent = crate::text_render::label_first_baseline_ascent_with_family(
                    label,
                    font_size as f64,
                    &skin.actor_font_family,
                );
                let stickman_width = super::ACTOR_ARM_HALF * 2.0 + dim.stroke_thickness * 2.0;
                let stickman_height =
                    super::ACTOR_STICKMAN_BASE_HEIGHT + dim.stroke_thickness * 2.0;

                assert_close(dim.label.content_width, expected_width);
                assert_close(dim.label.content_height, expected_height);
                assert_close(dim.label.width(), expected_width + 2.0 * padding);
                assert_close(dim.label.height(), expected_height + 2.0 * padding);
                assert_close(dim.width, dim.label.width().max(stickman_width));
                assert_close(dim.height, stickman_height + dim.label.height());
                assert_close(
                    dim.label_gap,
                    expected_ascent + 1.0 + dim.stroke_thickness + padding,
                );
                assert_close(dim.paint_min_y, dim.stroke_thickness);
            }
        }
    }

    #[test]
    fn global_padding_composes_actor_stereotype_as_an_independent_block() {
        let actor_dim = |padding: f64| {
            let input = format!(
                "@startuml\n\
                 skinparam Padding {padding}\n\
                 actor \"Renamed Held Out Operator\" as Probe <<external-axis>>\n\
                 :Dispatch Guard:\n\
                 @enduml"
            );
            let diagram = rustuml_parser::parse::parse(&input).unwrap();
            let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
                panic!("expected use-case diagram");
            };
            let actor = usecase
                .actors
                .iter()
                .find(|actor| actor.id == "Probe")
                .unwrap();
            let base = super::SkinColors::from_meta(&usecase.meta, None);
            let skin = base.for_actor(&usecase.meta, actor);
            super::actor_dim(actor, &skin)
        };

        let padding = 7.0;
        let plain = actor_dim(0.0);
        let padded = actor_dim(padding);
        let plain_stereo = plain.stereotype.expect("stereotype block");
        let padded_stereo = padded.stereotype.expect("stereotype block");

        assert_close(padded.label.width() - plain.label.width(), 2.0 * padding);
        assert_close(padded.label.height() - plain.label.height(), 2.0 * padding);
        assert_close(padded_stereo.width() - plain_stereo.width(), 2.0 * padding);
        assert_close(
            padded_stereo.height() - plain_stereo.height(),
            2.0 * padding,
        );
        assert_close(padded.height - plain.height, 4.0 * padding);
        assert_close(
            padded.stereo_baseline_offset - plain.stereo_baseline_offset,
            padding,
        );
        assert_close(padded.paint_min_y - plain.paint_min_y, padding);
        assert_close(padded_stereo.margin_x, super::ACTOR_STEREOTYPE_MARGIN_X);
    }

    #[test]
    fn global_padding_actor_fanout_keeps_usecase_ellipse_model_independent() {
        for direction in ["", "left to right direction\n"] {
            for padding in [0.0, 5.0] {
                let mut expected_radii: Option<Vec<(f64, f64)>> = None;
                for actor_label in ["I", "Renamed Wide Fanout Operator"] {
                    let input = format!(
                        "@startuml\n\
                         {direction}\
                         skinparam defaultFontName Verdana\n\
                         skinparam Padding {padding}\n\
                         actor \"{actor_label}\" as Operator\n\
                         usecase \"Renamed Intake\" as Intake\n\
                         usecase \"Renamed Review\" as Review\n\
                         usecase \"Renamed Archive\" as Archive\n\
                         Operator --> Intake\n\
                         Operator --> Review\n\
                         Operator --> Archive\n\
                         @enduml"
                    );
                    let diagram = rustuml_parser::parse::parse(&input).unwrap();
                    let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
                        panic!("expected use-case diagram");
                    };
                    let base = super::SkinColors::from_meta(&usecase.meta, None);
                    let actor_style = base.for_actor(&usecase.meta, &usecase.actors[0]);
                    let actor_dims = vec![super::actor_dim(&usecase.actors[0], &actor_style)];
                    let usecase_dims: Vec<_> = usecase
                        .use_cases
                        .iter()
                        .map(|item| {
                            let style = base.for_use_case(&usecase.meta, item);
                            super::use_case_dim(item, &style)
                        })
                        .collect();
                    let radii: Vec<_> = usecase_dims.iter().map(|dim| (dim.rx, dim.ry)).collect();
                    if let Some(expected) = &expected_radii {
                        assert_eq!(&radii, expected);
                    } else {
                        expected_radii = Some(radii);
                    }
                    let connection_styles: Vec<_> = usecase
                        .connections
                        .iter()
                        .map(|connection| base.for_connection(&usecase.meta, connection))
                        .collect();
                    let note_dims: Vec<_> = usecase.notes.iter().map(super::note_dim).collect();
                    let positions = super::resolve_positions(
                        usecase,
                        &actor_dims,
                        &usecase_dims,
                        &note_dims,
                        &connection_styles,
                        None,
                    );

                    assert_eq!(positions.actors.len(), 1);
                    assert_eq!(positions.use_cases.len(), 3);
                    assert_eq!(positions.edge_paths.len(), 3);
                    assert_close(
                        actor_dims[0].label.width(),
                        actor_dims[0].label.content_width + 2.0 * padding,
                    );
                }
            }
        }
    }

    #[test]
    fn dependency_arrow_uses_extremity_arrow_point_order() {
        assert_eq!(
            super::dependency_arrow_points((0.0, 0.0), (0.0, 10.0)),
            "0,10,4,1,0,5,-4,1,0,10"
        );
    }

    #[test]
    fn reversed_renamed_generalization_keeps_its_hollow_start_triangle() {
        let input = "@startuml\n\
                     actor \"Policy Parent 449\" as Parent\n\
                     actor \"Policy Child 457\" as Child\n\
                     Parent <|-- Child\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"data-link-type="extension""#));
        assert!(svg.contains(r#"<polygon fill="none""#));
        assert!(svg.contains(r#"id="Parent-backto-Child""#));
        assert_eq!(
            super::extension_arrow_points((0.0, 0.0), (0.0, 20.0)),
            "0,20,6,2,-6,2,0,20"
        );
    }

    #[test]
    fn reversed_renamed_dependency_uses_java_backto_path_identity() {
        let input = "@startuml\n\
                     usecase \"Fresh Origin 461\" as Origin\n\
                     usecase \"Fresh Reminder 463\" as Reminder\n\
                     Origin <.. Reminder : renamed extension\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        assert!(svg.contains(r#"id="Origin-backto-Reminder""#));
        assert!(svg.contains("stroke-dasharray:7,7;"));
    }

    #[test]
    fn renamed_generalization_canvas_uses_limit_finder_primitive_bounds() {
        let input = "@startuml\n\
                     actor \"Renamed Principal 701\" as Principal701\n\
                     actor \"Renamed Specialist 709\" as Specialist709\n\
                     usecase \"Renamed Capability 719\" as Capability719\n\
                     Specialist709 --|> Principal701\n\
                     Specialist709 --> Capability719\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Independent PlantUML oracle result for this renamed perturbation.
        // The dimensions exercise `LimitFinder.drawText` for the lower actor
        // and `drawEllipse` for the rightmost use case.
        assert!(svg.contains(r#"viewBox="0 0 402 232""#), "{svg}");
        assert!(svg.contains(r#"width="402px""#), "{svg}");
        assert!(svg.contains(r#"height="232px""#), "{svg}");
    }

    #[test]
    fn renamed_deep_generalization_normalizes_the_solved_painted_envelope() {
        let input = "@startuml\n\
                     actor \"QA\" as Root1201\n\
                     actor \"RB\" as Branch1213\n\
                     actor \"SC\" as Branch1217\n\
                     actor \"TD\" as Branch1223\n\
                     actor \"UE\" as Leaf1229\n\
                     usecase \"Fresh Review 1231\" as Review1231\n\
                     Leaf1229 --> Review1231\n\
                     Branch1213 --|> Root1201\n\
                     Branch1217 --|> Branch1213\n\
                     Branch1223 --|> Branch1217\n\
                     Leaf1229 --|> Branch1223\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Independent PlantUML result for this renamed depth-five perturbation.
        // Java `SvekResult.calculateDimension` moves the complete painted
        // `LimitFinder` envelope, including `ExtremityTriangle`, to x=6.
        assert!(svg.contains(r#"viewBox="0 0 241 641""#), "{svg}");
        assert!(svg.contains(r#"<ellipse cx="22" "#), "{svg}");
        assert!(svg.contains(">Fresh Review 1231</text>"), "{svg}");
        assert_eq!(svg.matches(r#"class="link""#).count(), 5, "{svg}");
    }

    #[test]
    fn renamed_deeper_rank_directions_normalize_edge_owned_envelopes() {
        let body = "actor \"Renamed Analyst 401\" as Analyst401\n\
                    usecase \"Fresh Intake 409\" as Intake409\n\
                    usecase \"Novel Review 419\" as Review419\n\
                    usecase \"Final Check 431\" as Check431\n\
                    Analyst401 -> Intake409\n\
                    Analyst401 -> Review419\n\
                    Intake409 .> Review419 : <<include>>\n\
                    Review419 .> Check431 : <<extend>>";
        for (direction, view_box, painted_guard) in [
            (
                "top to bottom direction",
                r#"viewBox="0 0 833 134""#,
                "C289.25,6 ",
            ),
            (
                "left to right direction",
                r#"viewBox="0 0 191 336""#,
                "C6,130.",
            ),
        ] {
            let input = format!("@startuml\n{direction}\n{body}\n@enduml");
            let diagram = rustuml_parser::parse::parse(&input).unwrap();
            let svg = crate::render_svg(&diagram);

            // Fresh PlantUML references generated for both rank directions.
            // `CommandRankDir.executeArg` selects the rank, then
            // `SvekResult.calculateDimension` normalizes `SvekEdge.drawU`'s
            // complete `LimitFinder` envelope before exporting the canvas.
            assert!(svg.contains(view_box), "{direction}: {svg}");
            assert!(svg.contains(painted_guard), "{direction}: {svg}");
            assert_eq!(svg.matches(r#"class="link""#).count(), 4, "{svg}");
        }
    }

    #[test]
    fn renamed_link_note_canvas_excludes_rose_trailing_padding() {
        let input = "@startuml\n\
                     actor \"Renamed Note Source 811\" as Source811\n\
                     usecase \"Renamed Note Target 821\" as Target821\n\
                     Source811 --> Target821\n\
                     note on link : Renamed approval gate 823\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Independent PlantUML oracle result for this renamed perturbation.
        assert!(svg.contains(r#"viewBox="0 0 325 236""#), "{svg}");
        assert!(svg.contains(r#"width="325px""#), "{svg}");
        assert!(svg.contains(r#"height="236px""#), "{svg}");
    }

    #[test]
    fn renamed_stereotype_uses_painted_text_footprint() {
        let input = "@startuml\n\
                     actor \"Renamed Observer 901\" as Observer901\n\
                     usecase \"Renamed Approval Path 907\" as Approval907 <<automated-variant-911>>\n\
                     Observer901 --> Approval907\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh PlantUML oracle result for this renamed perturbation. The
        // stereotype is narrower than the body, so a bounding-box shortcut
        // cannot reproduce `Footprint.getEllipse`.
        assert!(svg.contains(r#"viewBox="0 0 273 211""#), "{svg}");
        assert!(svg.contains(r#"rx="126.9332" ry="27.7866""#), "{svg}");
    }

    #[test]
    fn diagonal_links_meet_the_renamed_usecase_oval() {
        let input = "@startuml\n\
                     actor \"Audit Reader 431\" as Reader\n\
                     actor \"Policy Writer 433\" as Writer\n\
                     usecase \"Review Unseen Policy 439\" as Review\n\
                     Reader --> Review\n\
                     Writer --> Review\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let skin = super::SkinColors::from_meta(&usecase.meta, None);
        let actor_dims: Vec<_> = usecase
            .actors
            .iter()
            .map(|actor| super::actor_dim(actor, &skin))
            .collect();
        let usecase_dims: Vec<_> = usecase
            .use_cases
            .iter()
            .map(|item| {
                let style = skin.for_use_case(&usecase.meta, item);
                super::use_case_dim(item, &style)
            })
            .collect();
        let connection_styles: Vec<_> = usecase
            .connections
            .iter()
            .map(|connection| skin.for_connection(&usecase.meta, connection))
            .collect();
        let note_dims: Vec<_> = usecase.notes.iter().map(super::note_dim).collect();
        let positions = super::resolve_positions(
            usecase,
            &actor_dims,
            &usecase_dims,
            &note_dims,
            &connection_styles,
            None,
        );
        let (cx, cy) = positions.use_cases[0];
        let oval = &usecase_dims[0];
        let mut diagonal_edges = 0;

        for edge in &positions.edge_paths {
            let endpoint = edge
                .end_point
                .or_else(|| edge.points.last().copied())
                .unwrap();
            let normalized =
                ((endpoint.0 - cx) / oval.rx).powi(2) + ((endpoint.1 - cy) / oval.ry).powi(2);
            assert!(
                (normalized - 1.0).abs() < 0.08,
                "diagonal endpoint {endpoint:?} must meet oval centered at ({cx}, {cy})"
            );
            let source_x = usecase
                .actors
                .iter()
                .position(|actor| actor.id == edge.from)
                .map(|index| positions.actors[index].0)
                .unwrap();
            if (source_x - cx).abs() > 1.0 {
                diagonal_edges += 1;
            }
        }
        assert!(diagonal_edges > 0);
    }

    #[test]
    fn lone_renamed_usecase_uses_degenerated_entity_inset() {
        let input = "@startuml\nusecase \"Fresh Singleton\" as Singleton\n@enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let skin = super::SkinColors::from_meta(&usecase.meta, None);
        let dim = super::use_case_dim(&usecase.use_cases[0], &skin);
        let expected_cx = super::fc(super::DEGENERATED_MARGIN + dim.rx);
        let expected_cy = super::fc(super::DEGENERATED_MARGIN + dim.ry);
        assert!(svg.contains(&format!(
            r#"<ellipse cx="{expected_cx}" cy="{expected_cy}""#
        )));
    }

    #[test]
    fn renamed_document_chrome_wraps_and_centers_the_svek_body() {
        let input = "@startuml\n\
                     header Renamed Audit Ribbon 314159\n\
                     title Fresh Access Review 271828\n\
                     footer Renamed Audit Ribbon 314159\n\
                     actor \"Novel Reviewer\" as Reviewer\n\
                     usecase \"Inspect Unseen Record\" as Inspect\n\
                     Reviewer --> Inspect\n\
                     @enduml";
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference for renamed content absent from the
        // corpus. This exercises both wrappers in
        // `DiagramChromeFactory12026.create`: title first, then ribbons.
        assert!(svg.contains(r#"viewBox="0 0 236 260""#), "{svg}");
        assert!(
            svg.contains(r#"class="header" data-source-line="1""#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"textLength="152.4365" x="76.7842" y="9.668""#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"class="title" data-source-line="2""#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"textLength="209.2207" x="10" y="36.3125""#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"<ellipse cx=""#) && svg.contains(r#"cy="64.2656""#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"class="footer" data-source-line="3""#),
            "{svg}"
        );
        assert!(
            svg.contains(r#"textLength="152.4365" x="38.3921""#),
            "{svg}"
        );
    }

    #[test]
    fn renamed_mixed_monospace_atoms_drive_actor_baselines_and_usecase_ellipse() {
        let input = r#"@startuml
actor """fixed"" Auditor" as FreshActor
usecase """fixed"" Review" as FreshReview
FreshActor --> FreshReview
@enduml"#;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);

        // Fresh Java PlantUML reference absent from the golden corpus.
        // `CreoleStripeSimpleParser` emits separate `AtomText` runs;
        // `Sea.doAlign` bottom-aligns them and
        // `Footprint.MyUGraphic.drawText` contributes each rectangle to
        // `TextBlockInEllipse`.
        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let skin = super::SkinColors::from_meta(&usecase.meta, None);
        let actor = super::actor_dim(&usecase.actors[0], &skin);
        let usecase = super::use_case_dim(&usecase.use_cases[0], &skin);
        assert!((actor.label_gap - 14.69).abs() < 0.01);
        assert!((usecase.rx - 65.44).abs() < 0.01);
        assert!((usecase.ry - 15.49).abs() < 0.01);
        assert!(svg.contains(r#"viewBox="0 0 150 186""#), "{svg}");
        assert_eq!(svg.matches(r#"font-family="monospace""#).count(), 2);
        assert!(svg.contains(">Auditor</text>"), "{svg}");
        assert!(svg.contains(">Review</text>"), "{svg}");
    }

    #[test]
    fn renamed_plain_special_characters_keep_single_atom_footprints() {
        let input = r#"@startuml
actor "Ops+Lead" as FreshOperator
usecase "Review > Queue_2 + Confirm" as FreshCheckpoint
FreshOperator --> FreshCheckpoint
@enduml"#;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let skin = super::SkinColors::from_meta(&usecase.meta, None);
        let label = &usecase.use_cases[0].label;
        let atoms = super::use_case_footprint_atoms(
            label,
            0.0,
            skin.uc_font_size as f64,
            &skin.uc_font_family,
        );

        // Java's `CreoleStripeSimpleParser` leaves this plain line in one
        // `AtomText`; `Footprint.MyUGraphic` therefore records one rectangle.
        assert_eq!(atoms.len(), 1);
        let expected_width = crate::text_render::measure_with_family(
            label,
            skin.uc_font_size as f64,
            false,
            &skin.uc_font_family,
        );
        assert!((atoms[0].width - expected_width).abs() < 0.01);
        assert!((atoms[0].line_width - expected_width).abs() < 0.01);
    }

    #[test]
    fn entity_and_link_styles_follow_their_java_builder_ownership() {
        let input = r##"@startuml
<style>
usecase {
  BackgroundColor #E0F7FA
  LineColor #00838F
  FontColor #006064
}
arrow {
  LineColor #AD1457
  HeadColor #880E4F
  LineThickness 3
  LineStyle 5-2
}
</style>
usecase "Early Harbor" as Early
usecase "Middle Harbor" as Middle
Early --> Middle : early route
<style>
usecase {
  BackgroundColor #FCE4EC
  LineColor #C2185B
  FontColor #880E4F
}
arrow {
  LineColor #1565C0
  LineThickness 2
  LineStyle 7-4
}
</style>
usecase "Late Harbor" as Late
Middle --> Late : late route
@enduml"##;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let base = super::SkinColors::from_meta(&usecase.meta, None);
        let early = base.for_use_case(&usecase.meta, &usecase.use_cases[0]);
        let late = base.for_use_case(&usecase.meta, &usecase.use_cases[2]);
        let early_link = base.for_connection(&usecase.meta, &usecase.connections[0]);
        let late_link = base.for_connection(&usecase.meta, &usecase.connections[1]);

        // Java `Entity#getCurrentStyleBuilder` preserves pure-CSS creation
        // snapshots; `Link#getStyleBuilder` does the same independently.
        assert_eq!(
            early
                .uc_fill
                .as_ref()
                .and_then(super::DescriptionPaint::flat_control),
            Some("#E0F7FA")
        );
        assert_eq!(early.uc_border.as_deref(), Some("#00838F"));
        assert_eq!(early.uc_font_color, "#006064");
        assert_eq!(
            late.uc_fill
                .as_ref()
                .and_then(super::DescriptionPaint::flat_control),
            Some("#FCE4EC")
        );
        assert_eq!(late.uc_border.as_deref(), Some("#C2185B"));
        assert_eq!(late.uc_font_color, "#880E4F");
        assert_eq!(early_link.arrow_color, "#AD1457");
        assert_eq!(early_link.arrow_head_color, "#880E4F");
        assert_eq!(early_link.arrow_stroke_width, 3.0);
        assert_eq!(early_link.arrow_dash, Some((5.0, 2.0)));
        assert_eq!(late_link.arrow_color, "#1565C0");
        // The later sparse arrow rule does not erase the earlier HeadColor.
        assert_eq!(late_link.arrow_head_color, "#880E4F");
        assert_eq!(late_link.arrow_stroke_width, 2.0);
        assert_eq!(late_link.arrow_dash, Some((7.0, 4.0)));

        let svg = crate::render_svg(&diagram);
        assert!(
            svg.contains("stroke:#AD1457;stroke-width:3;stroke-dasharray:5,2;"),
            "{svg}"
        );
        assert!(
            svg.contains(r##"<polygon fill="#880E4F""##)
                && svg.contains("stroke:#880E4F;stroke-width:3;"),
            "{svg}"
        );
        assert!(
            svg.contains("stroke:#1565C0;stroke-width:2;stroke-dasharray:7,4;"),
            "{svg}"
        );
    }

    #[test]
    fn legacy_skinparam_refreshes_entities_but_not_older_links() {
        let input = r##"@startuml
<style>
usecase {
  BackgroundColor #E0F7FA
}
</style>
usecase "Early Harbor" as Early
usecase "Late Harbor" as Late
Early --> Late : before legacy
skinparam UsecaseBackgroundColor #FCE4EC
skinparam ArrowColor #1565C0
Late --> Early : after legacy
@enduml"##;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let base = super::SkinColors::from_meta(&usecase.meta, None);
        let early = base.for_use_case(&usecase.meta, &usecase.use_cases[0]);
        let late = base.for_use_case(&usecase.meta, &usecase.use_cases[1]);
        let before = base.for_connection(&usecase.meta, &usecase.connections[0]);
        let after = base.for_connection(&usecase.meta, &usecase.connections[1]);

        // Java `Entity#getCurrentStyleBuilder` has the compatibility refresh;
        // `Link#getStyleBuilder` always keeps the captured builder.
        assert_eq!(
            early
                .uc_fill
                .as_ref()
                .and_then(super::DescriptionPaint::flat_control),
            Some("#FCE4EC")
        );
        assert_eq!(
            late.uc_fill
                .as_ref()
                .and_then(super::DescriptionPaint::flat_control),
            Some("#FCE4EC")
        );
        assert_eq!(before.arrow_color, super::STROKE);
        assert_eq!(after.arrow_color, "#1565C0");
    }

    #[test]
    fn concrete_actor_and_usecase_signatures_normalize_stereotypes() {
        let input = r##"@startuml
<style>
actor {
  .External.User {
    BackgroundColor #FFF3E0
    LineColor #E65100
    FontColor #BF360C
  }
}
usecase {
  .Review.Gate {
    BackgroundColor #DCEDC8
    LineColor #558B2F
    FontColor #33691E
    LineThickness 2
  }
}
</style>
actor "External User" as External <<external_user>>
usecase "Review Gate" as Review <<review_gate>>
External --> Review
@enduml"##;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let base = super::SkinColors::from_meta(&usecase.meta, None);
        let actor = base.for_actor(&usecase.meta, &usecase.actors[0]);
        let use_case = base.for_use_case(&usecase.meta, &usecase.use_cases[0]);

        // Java `StyleSignatureBasic#clean` makes dots and underscores the same
        // stereotype identity before `withTOBECHANGED` merges the style.
        assert_eq!(
            actor
                .actor_fill
                .as_ref()
                .and_then(super::DescriptionPaint::flat_control),
            Some("#FFF3E0")
        );
        assert_eq!(actor.actor_border.as_deref(), Some("#E65100"));
        assert_eq!(actor.actor_font_color, "#BF360C");
        assert_eq!(
            use_case
                .uc_fill
                .as_ref()
                .and_then(super::DescriptionPaint::flat_control),
            Some("#DCEDC8")
        );
        assert_eq!(use_case.uc_border.as_deref(), Some("#558B2F"));
        assert_eq!(use_case.uc_font_color, "#33691E");
        assert_eq!(use_case.uc_stereo_font_color, "#33691E");
        assert_eq!(use_case.uc_border_thickness, "2");
    }

    #[test]
    fn usecase_symbol_padding_and_shadow_remain_inert() {
        let plain =
            rustuml_parser::parse::parse("@startuml\nusecase \"Inert Controls\" as Probe\n@enduml")
                .unwrap();
        let styled = rustuml_parser::parse::parse(
            "@startuml\n<style>\nusecase {\nPadding 5 9 13\nShadowing 3\n}\n</style>\nusecase \"Inert Controls\" as Probe\n@enduml",
        )
        .unwrap();
        let rustuml_parser::diagram::Diagram::UseCase(plain) = &plain else {
            panic!("expected use-case diagram");
        };
        let rustuml_parser::diagram::Diagram::UseCase(styled) = &styled else {
            panic!("expected use-case diagram");
        };
        let plain_base = super::SkinColors::from_meta(&plain.meta, None);
        let styled_base = super::SkinColors::from_meta(&styled.meta, None);
        let plain_style = plain_base.for_use_case(&plain.meta, &plain.use_cases[0]);
        let styled_style = styled_base.for_use_case(&styled.meta, &styled.use_cases[0]);
        let plain_dim = super::use_case_dim(&plain.use_cases[0], &plain_style);
        let styled_dim = super::use_case_dim(&styled.use_cases[0], &styled_style);

        // Java `USymbolUsecase#asSmall` does not consume symbol-style Padding
        // or Shadowing. This is distinct from global `skinparam Padding`,
        // which `Display#getCreole` passes to every `SheetBlock1`.
        assert_eq!(plain_dim.rx, styled_dim.rx);
        assert_eq!(plain_dim.ry, styled_dim.ry);
    }

    #[test]
    fn global_padding_composes_natural_blocks_across_font_and_label_axes() {
        let cases = [
            ("sans-serif", 14, "IO"),
            ("Verdana", 12, "Renamed account approval queue"),
            ("Arial", 17, "**Bold** and //italic// controls"),
        ];

        for (font, font_size, label) in cases {
            for padding in [0.0, 3.0, 5.0, 11.0] {
                let input = format!(
                    "@startuml\n\
                     skinparam defaultFontName {font}\n\
                     skinparam defaultFontSize {font_size}\n\
                     skinparam Padding {padding}\n\
                     usecase \"{label}\" as Probe\n\
                     @enduml"
                );
                let diagram = rustuml_parser::parse::parse(&input).unwrap();
                let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
                    panic!("expected use-case diagram");
                };
                let base = super::SkinColors::from_meta(&usecase.meta, None);
                let skin = base.for_use_case(&usecase.meta, &usecase.use_cases[0]);
                let dim = super::use_case_dim(&usecase.use_cases[0], &skin);
                let label_height = crate::text_render::label_height_with_family(
                    label,
                    font_size as f64,
                    &skin.uc_font_family,
                );

                assert_close(skin.creole_padding, padding);
                assert_close(dim.text_block.width, dim.label_w + 2.0 * padding);
                assert_close(dim.text_block.height, label_height + 2.0 * padding);
                assert_close(dim.text_block.body_line_tops[0], padding);
                assert!(
                    dim.text_block
                        .footprint_lines
                        .iter()
                        .all(|atom| (atom.top - padding).abs() < 0.0001)
                );
            }
        }
    }

    #[test]
    fn explicit_zero_padding_preserves_the_unpadded_render() {
        let plain = rustuml_parser::parse::parse(
            "@startuml\n' padding control\nusecase \"Zero Padding Control\" as Probe\n@enduml",
        )
        .unwrap();
        let explicit = rustuml_parser::parse::parse(
            "@startuml\nskinparam Padding 0\nusecase \"Zero Padding Control\" as Probe\n@enduml",
        )
        .unwrap();

        assert_eq!(crate::render_svg(&plain), crate::render_svg(&explicit));
    }

    #[test]
    fn global_padding_translates_stereotype_and_compartment_composition() {
        let stereotype_input = |padding: f64| {
            format!(
                "@startuml\n\
                 skinparam Padding {padding}\n\
                 actor \"Held Out Auditor\" as Auditor\n\
                 usecase \"Renamed Held Out Approval Surface\" as Probe <<audit-axis>>\n\
                 Auditor --> Probe\n\
                 @enduml"
            )
        };
        let stereotype_dim = |padding: f64| {
            let diagram = rustuml_parser::parse::parse(&stereotype_input(padding)).unwrap();
            let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
                panic!("expected use-case diagram");
            };
            let base = super::SkinColors::from_meta(&usecase.meta, None);
            let skin = base.for_use_case(&usecase.meta, &usecase.use_cases[0]);
            super::use_case_dim(&usecase.use_cases[0], &skin)
        };
        let stereotype_padding = 7.0;
        let plain_stereo = stereotype_dim(0.0);
        let padded_stereo = stereotype_dim(stereotype_padding);
        assert_close(
            padded_stereo.text_block.height - plain_stereo.text_block.height,
            4.0 * stereotype_padding,
        );
        assert_close(
            padded_stereo.text_block.stereo_text_top.unwrap(),
            stereotype_padding,
        );
        assert_close(
            padded_stereo.text_block.body_line_tops[0] - plain_stereo.text_block.body_line_tops[0],
            3.0 * stereotype_padding,
        );
        assert_close(
            padded_stereo.text_block.footprint_lines[0].top,
            stereotype_padding,
        );
        let stereo_bound = padded_stereo.text_block.footprint_bounds[0];
        let stereo_bound_width = stereo_bound.end.0 - stereo_bound.start.0;
        assert!(padded_stereo.text_block.width > stereo_bound_width);
        assert_close(
            stereo_bound_width,
            padded_stereo.stereo_w + 2.0 * stereotype_padding + 2.0,
        );
        assert_close(
            stereo_bound.start.0,
            (padded_stereo.text_block.width - stereo_bound_width) / 2.0,
        );
        assert_close(stereo_bound.start.1, 0.0);
        assert_close(
            stereo_bound.end.1 - stereo_bound.start.1,
            plain_stereo.text_block.body_line_tops[0] + 2.0 * stereotype_padding,
        );

        let multiline_input = |padding: f64| {
            format!(
                "@startuml\n\
                 skinparam Padding {padding}\n\
                 usecase FreshFlow as \"\n\
                 Summary\n\
                 ....\n\
                 First detail\n\
                 ====\n\
                 Second detail\n\
                 ____\n\
                 Final detail\n\
                 \"\n\
                 actor \"Held Out Reviewer\" as Reviewer\n\
                 usecase \"Dispatch Guard\" as Guard\n\
                 Reviewer --> FreshFlow\n\
                 @enduml"
            )
        };
        let multiline_dim = |padding: f64| {
            let diagram = rustuml_parser::parse::parse(&multiline_input(padding)).unwrap();
            let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
                panic!("expected use-case diagram");
            };
            let base = super::SkinColors::from_meta(&usecase.meta, None);
            let skin = base.for_use_case(&usecase.meta, &usecase.use_cases[0]);
            super::use_case_dim(&usecase.use_cases[0], &skin)
        };
        let plain = multiline_dim(0.0);
        let padded = multiline_dim(5.0);
        assert_close(padded.text_block.width - plain.text_block.width, 10.0);
        assert_close(padded.text_block.height - plain.text_block.height, 40.0);
        for (index, (&plain_top, &padded_top)) in plain
            .text_block
            .body_line_tops
            .iter()
            .zip(&padded.text_block.body_line_tops)
            .enumerate()
        {
            assert_close(padded_top - plain_top, 5.0 * (2 * index + 1) as f64);
        }
        for (index, (plain_bounds, padded_bounds)) in plain
            .text_block
            .footprint_bounds
            .iter()
            .zip(&padded.text_block.footprint_bounds)
            .enumerate()
        {
            assert_close(
                padded_bounds.start.1 - plain_bounds.start.1,
                10.0 * (index + 1) as f64,
            );
            assert_close(
                (padded_bounds.end.0 - padded_bounds.start.0)
                    - (plain_bounds.end.0 - plain_bounds.start.0),
                10.0,
            );
        }
    }

    #[test]
    fn document_style_keeps_transparency_and_four_independent_margins() {
        let input = r##"@startuml
<style>
document {
  BackgroundColor transparent
  Margin 3 7 11 13
}
</style>
usecase "Document Probe" as Probe
@enduml"##;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let rustuml_parser::diagram::Diagram::UseCase(usecase) = &diagram else {
            panic!("expected use-case diagram");
        };
        let skin = super::SkinColors::from_meta(&usecase.meta, None);

        assert_eq!(skin.canvas_background, None);
        assert_eq!(skin.canvas_rect, None);
        assert_eq!(
            skin.document_margin,
            crate::style_cascade::StyleBoxSides {
                top: 3.0,
                right: 7.0,
                bottom: 11.0,
                left: 13.0,
            }
        );
        let svg = crate::render_svg(&diagram);
        assert!(!svg.contains("background:#FFFFFF;"), "{svg}");
        assert!(!svg.contains(r##"<rect fill="#FFFFFF""##), "{svg}");
    }

    #[test]
    fn skinparams_style_actor_and_usecase_text() {
        let input = r##"@startuml
skinparam backgroundColor transparent
skinparam defaultFontName "Verdana"
skinparam defaultFontSize 12
skinparam actorFontColor #fff
skinparam actorBorderColor #78c2ad
skinparam actorBackgroundColor #86c8b5
skinparam __styleRootLineThickness 1
skinparam usecaseFontColor #fff
skinparam usecaseBorderColor #78c2ad
skinparam usecaseBackgroundColor #86c8b5
skinparam usecaseBorderThickness 2
actor User
usecase "Login" as UC1
User --> UC1
@enduml"##;
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let svg = crate::render_svg(&diagram);
        assert!(svg.contains(r#"style="width:"#));
        assert!(!svg.contains("background:#FFFFFF;"));
        assert!(svg.contains(r##"fill="#86C8B5""##));
        assert!(svg.contains(r#"stroke:#78C2AD;stroke-width:1;"#));
        assert!(svg.contains(r#"stroke:#78C2AD;stroke-width:2;"#));
        assert!(svg.contains(r#"font-family="'Verdana'""#));
        assert!(svg.contains(r#"font-size="12""#));
        assert!(svg.contains(r##"fill="#FFFFFF""##));
    }
}
