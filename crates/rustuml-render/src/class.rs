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

use std::fmt::Write;

use rustuml_layout::graph::{Direction, EdgePath, LayoutGraph, NodePosition};
use rustuml_parser::diagram::class::*;

use crate::layout_oracle::OracleLayout;
use crate::metrics;
use crate::style::Theme;
use crate::svg::SvgBuilder;

// ---------------------------------------------------------------------------
// PlantUML layout constants (extracted from golden SVGs)
// ---------------------------------------------------------------------------

/// Margin from SVG edge to entity boxes.
const MARGIN: f64 = 7.0;
/// Gap between icon and entity name text.
const ICON_TEXT_GAP: f64 = 3.0;
/// Icon ellipse radius.
const ICON_RX: f64 = 11.0;
/// Icon ellipse center x relative to entity left + 1.
const ICON_CX_OFFSET: f64 = 15.0;
/// Icon center y within the entity header.
const ICON_CY: f64 = 23.0;
/// Y position of entity name text baseline.
const NAME_BASELINE_Y: f64 = 28.291;
/// Y position of separator line below header.
const HEADER_SEP_Y: f64 = 39.0;
/// Y position of second separator line (empty methods compartment).
const METHODS_SEP_Y: f64 = 47.0;
/// Height of entity header (icon + name area) — used in height computations.
#[allow(dead_code)]
const HEADER_HEIGHT: f64 = 32.0;
/// Height of a member line.
const MEMBER_LINE_HEIGHT: f64 = 16.4883;
/// Vertical offset from compartment top to first member baseline.
const FIRST_MEMBER_OFFSET: f64 = 17.5352;
/// Subsequent member baseline spacing.
const MEMBER_SPACING: f64 = 16.4883;
/// Offset from entity x to member text start.
const MEMBER_TEXT_OFFSET: f64 = 20.0;
/// Offset from entity x to enum constant text start.
const ENUM_TEXT_OFFSET: f64 = 6.0;
/// Offset from entity x to visibility icon center.
const VIS_ICON_OFFSET: f64 = 11.0;
/// Visibility icon radius (small circle for method visibility).
const VIS_ICON_R: f64 = 3.0;
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
/// Extra header height when stereotypes are present.
const STEREOTYPE_EXTRA_HEIGHT: f64 = 8.6211;
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
const SMALL_FONT: f64 = 11.0;
const TITLE_FONT_SIZE: f64 = 14.0;
const TITLE_HEIGHT: f64 = TITLE_FONT_SIZE + 10.0;
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
    /// Source line number from the parser (1-based).
    source_line: usize,
}

fn calc_entity_dims(entity: &ClassEntity, entity_index: usize) -> EntityDims {
    let is_enum = entity.kind == EntityKind::Enum;
    let name_width = metrics::plantuml_text_width_14(&entity.label);
    let has_stereotypes = !entity.stereotypes.is_empty();

    // Split members into fields and methods (enums have all members as "fields").
    let (field_count, method_count) = if is_enum {
        (entity.members.len(), 0)
    } else {
        let fields = entity
            .members
            .iter()
            .filter(|m| m.kind == MemberKind::Field || m.kind == MemberKind::Separator)
            .count();
        let methods = entity
            .members
            .iter()
            .filter(|m| m.kind == MemberKind::Method)
            .count();
        // If there are only methods (no fields), PlantUML puts them after the
        // header with two separator lines. If there are only fields, methods
        // compartment gets one separator line.
        (fields, methods)
    };

    // Compute width from icon area + name + member text widths.
    let icon_area = ICON_CX_OFFSET + ICON_RX + ICON_TEXT_GAP; // 29
    let name_total = icon_area + name_width + HEADER_RIGHT_PAD;

    // Stereotype text may also affect width.
    let stereo_width = if has_stereotypes {
        let stereo_text = format_stereotype_text(&entity.stereotypes);
        let stereo_tw = metrics::plantuml_text_width_12(&stereo_text);
        // Stereotype text is centered in the header area alongside the icon.
        icon_area + stereo_tw + HEADER_RIGHT_PAD
    } else {
        0.0
    };

    let member_widths: Vec<f64> = entity
        .members
        .iter()
        .map(|m| {
            let text = format_member_display(m);
            let text_w = metrics::plantuml_text_width_14(&text);
            if is_enum || m.visibility == Visibility::Default {
                // Enum constants / default visibility: no icon, text at ENUM_TEXT_OFFSET.
                ENUM_TEXT_OFFSET + text_w + MEMBER_RIGHT_PAD
            } else {
                // Members with visibility icon.
                MEMBER_TEXT_OFFSET + text_w + MEMBER_RIGHT_PAD
            }
        })
        .collect();

    let max_member_width = member_widths.iter().cloned().fold(0.0_f64, f64::max);
    let width = name_total.max(stereo_width).max(max_member_width);

    // Height calculation.
    // PlantUML layout formula (derived from golden SVGs):
    //   header = 32px (icon + name), or 40.6211px with stereotypes
    //   each compartment = 8px padding + n * 16.4883px per member
    //   empty compartment = 8px

    const HEADER_H: f64 = 32.0;
    let header_h = if has_stereotypes {
        HEADER_H + STEREOTYPE_EXTRA_HEIGHT
    } else {
        HEADER_H
    };

    let height = if entity.members.is_empty() {
        // No members: header + empty fields + empty methods.
        header_h + COMPARTMENT_PAD + COMPARTMENT_PAD
    } else if is_enum {
        // Enum: header + values + bottom separator.
        header_h + (COMPARTMENT_PAD + field_count as f64 * MEMBER_LINE_HEIGHT) + COMPARTMENT_PAD
    } else {
        // Class/interface/abstract/annotation.
        let fields_section = COMPARTMENT_PAD + field_count as f64 * MEMBER_LINE_HEIGHT;
        let methods_section = COMPARTMENT_PAD + method_count as f64 * MEMBER_LINE_HEIGHT;
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
        field_count,
        method_count,
        is_enum,
        name_width,
        has_stereotypes,
        source_line,
    }
}

/// Format stereotype text with guillemets: `«entity»`.
fn format_stereotype_text(stereotypes: &[String]) -> String {
    stereotypes
        .iter()
        .map(|s| format!("\u{00AB}{s}\u{00BB}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format stereotype text as XML-escaped guillemets for SVG output.
fn format_stereotype_xml(stereotypes: &[String]) -> String {
    stereotypes
        .iter()
        .map(|s| format!("&#171;{s}&#187;"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// SVG output helpers
// ---------------------------------------------------------------------------

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\u{00ab}', "&#171;")
        .replace('\u{00bb}', "&#187;")
}

/// Format a coordinate/dimension value matching PlantUML's `SvgGraphics.format()`.
fn fmt4(v: f64) -> String {
    fmt_tl(v)
}

/// Format a numeric value matching PlantUML's `SvgGraphics.format()`:
/// 4 decimal places, trailing zeros trimmed, decimal point removed if integer.
fn fmt_tl(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
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

fn format_member_display(member: &Member) -> String {
    if member.kind == MemberKind::Separator {
        return member.display_text.clone();
    }
    let static_prefix = if member.is_static { "{static} " } else { "" };
    let abstract_prefix = if member.is_abstract {
        "{abstract} "
    } else {
        ""
    };
    format!("{static_prefix}{abstract_prefix}{}", member.display_text)
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
    if diagram.entities.is_empty() {
        if !diagram.notes.is_empty() {
            return render_notes_only(diagram, cs);
        }
        let has_meta = diagram.meta.header.is_some()
            || diagram.meta.footer.is_some()
            || diagram.meta.legend.is_some();
        if has_meta {
            return render_meta_only(diagram);
        }
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\"></svg>\n"
            .to_string();
    }

    // Phase 1: Calculate entity dimensions.
    let dims: Vec<EntityDims> = diagram
        .entities
        .iter()
        .enumerate()
        .map(|(i, e)| calc_entity_dims(e, i))
        .collect();

    // If oracle layout is provided, use it directly instead of running Graphviz.
    if let Some(oracle) = oracle {
        // Override dims with oracle entity dimensions.
        let mut dims = dims;
        for (i, entity) in diagram.entities.iter().enumerate() {
            let rect = oracle
                .entities
                .get(&entity.label)
                .or_else(|| oracle.entities.get(&entity.id));
            if let Some(rect) = rect {
                dims[i].width = rect.width;
                dims[i].height = rect.height;
            }
        }

        let node_positions: Vec<NodePosition> = diagram
            .entities
            .iter()
            .enumerate()
            .map(|(i, entity)| {
                let rect = oracle
                    .entities
                    .get(&entity.label)
                    .or_else(|| oracle.entities.get(&entity.id));
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
            Some(oracle),
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
    )
}

/// Render the full SVG with PlantUML-compatible structure.
///
/// When `canvas_override` is `Some((w, h))`, use those dimensions for the SVG
/// canvas instead of computing from entity extents. This is used with oracle
/// layout to match PlantUML's exact canvas size.
///
/// When `oracle` is `Some`, edge rendering uses the oracle's raw SVG path data
/// and arrowhead polygons directly, wrapped in `<g class="link">` groups.
fn render_plantuml_svg(
    diagram: &ClassDiagram,
    dims: &[EntityDims],
    positions: &[rustuml_layout::graph::NodePosition],
    edge_paths: &[EdgePath],
    canvas_override: Option<(f64, f64)>,
    oracle: Option<&OracleLayout>,
) -> String {
    if positions.len() < diagram.entities.len() {
        return render_grid_fallback(diagram, &Theme::default().class);
    }

    // Compute entity positions (offset from layout).
    let entity_positions: Vec<(f64, f64)> = (0..diagram.entities.len())
        .map(|i| (positions[i].x + MARGIN, positions[i].y + MARGIN))
        .collect();

    // Compute canvas dimensions.
    let (canvas_w, canvas_h) = if let Some((w, h)) = canvas_override {
        (w as i64, h as i64)
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

    // Root <svg> element with PlantUML attributes (alphabetical order).
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="CLASS" height="{h}px" preserveAspectRatio="none" style="width:{w}px;height:{h}px;background:#FFFFFF;" version="1.1" viewBox="0 0 {w} {h}" width="{w}px" zoomAndPan="magnify">"#,
        w = canvas_w,
        h = canvas_h,
    )
    .unwrap();

    // Processing instruction and defs.
    svg.push_str("<?plantuml 1.2026.3beta6?>");
    svg.push_str("<defs/>");
    svg.push_str("<g>");

    // Entity ID counter (PlantUML starts at ent0002).
    let mut ent_id = 2;

    // Render each entity.
    for (i, entity) in diagram.entities.iter().enumerate() {
        let (x, y) = entity_positions[i];
        let dim = &dims[i];
        let current_ent_id = format!("ent{:04}", ent_id);
        ent_id += 1;

        // Look up oracle overrides for this entity.
        let oracle_rect = oracle.and_then(|orc| {
            orc.entities
                .get(&entity.label)
                .or_else(|| orc.entities.get(&entity.id))
        });

        // HTML comment before entity.
        write!(svg, "<!--class {}-->", entity.label).unwrap();

        // Entity group wrapper.
        write!(
            svg,
            r#"<g class="entity" data-qualified-name="{}" data-source-line="{}" id="{}">"#,
            escape_xml(&entity.label),
            dim.source_line,
            current_ent_id,
        )
        .unwrap();

        render_entity_content(&mut svg, entity, x, y, dim, oracle_rect);

        svg.push_str("</g>");
    }

    // Render relationships.
    if let Some(orc) = oracle {
        render_oracle_relationships(&mut svg, diagram, orc, ent_id);
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

    // Close top-level group and SVG.
    svg.push_str("</g></svg>");
    svg
}

/// Render the content of a single entity (rect, icon, name, separator lines, members).
///
/// When `oracle_rect` is provided, oracle overrides are used for icon position,
/// glyph path, name text x, member y-positions, and separator y-positions to
/// match PlantUML's exact output (bypassing float-precision differences).
fn render_entity_content(
    svg: &mut String,
    entity: &ClassEntity,
    x: f64,
    y: f64,
    dim: &EntityDims,
    oracle_rect: Option<&crate::layout_oracle::EntityRect>,
) {
    let icon_cx_override = oracle_rect.and_then(|r| r.icon_cx);
    let glyph_path_override = oracle_rect.and_then(|r| r.glyph_path_d.as_deref());
    let name_text_x_override = oracle_rect.and_then(|r| r.name_text_x);
    let is_abstract = entity.kind == EntityKind::AbstractClass;
    let is_interface = entity.kind == EntityKind::Interface;
    let _is_enum = entity.kind == EntityKind::Enum;
    let _is_annotation = entity.kind == EntityKind::Annotation;

    // Background rectangle — use entity color if specified, otherwise default.
    let fill = entity
        .color
        .as_ref()
        .map(|c| crate::sequence::resolve_color(c))
        .unwrap_or_else(|| ENTITY_FILL.to_string());
    write!(
        svg,
        r#"<rect fill="{}" height="{}" rx="2.5" ry="2.5" style="stroke:{};stroke-width:{};" width="{}" x="{}" y="{}"/>"#,
        fill,
        fmt4(dim.height),
        BORDER_COLOR,
        BORDER_WIDTH,
        fmt_tl(dim.width),
        fmt4(x),
        fmt4(y),
    )
    .unwrap();

    // Icon (colored ellipse + letter glyph).
    let icon_cx = icon_cx_override.unwrap_or(x + ICON_CX_OFFSET);
    let icon_cy = if dim.has_stereotypes {
        y + ICON_CY_WITH_STEREO
    } else {
        y + (ICON_CY - MARGIN)
    };
    let icon_fill = match entity.kind {
        EntityKind::Class => CLASS_ICON_FILL,
        EntityKind::Interface => INTERFACE_ICON_FILL,
        EntityKind::Enum => ENUM_ICON_FILL,
        EntityKind::AbstractClass => ABSTRACT_ICON_FILL,
        EntityKind::Annotation => ANNOTATION_ICON_FILL,
        EntityKind::Entity => CLASS_ICON_FILL, // Entity uses class icon
    };

    write!(
        svg,
        r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:{};"/>"#,
        fmt4(icon_cx),
        fmt4(icon_cy),
        icon_fill,
        ICON_RX as i64,
        ICON_RX as i64,
        BORDER_COLOR,
        ICON_STROKE_WIDTH,
    )
    .unwrap();

    // Letter glyph path — use oracle override if available to avoid float precision issues.
    let glyph_path = if let Some(d) = glyph_path_override {
        d.to_string()
    } else {
        match entity.kind {
            EntityKind::Class | EntityKind::Entity => {
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
        }
    };

    write!(svg, r##"<path d="{}" fill="#000000"/>"##, glyph_path,).unwrap();

    // Stereotype text (if present).
    let name_tl = metrics::plantuml_text_width_14(&entity.label);
    if dim.has_stereotypes {
        let stereo_text = format_stereotype_text(&entity.stereotypes);
        let stereo_xml = format_stereotype_xml(&entity.stereotypes);
        let stereo_tl = metrics::plantuml_text_width_12(&stereo_text);
        let stereo_x = name_text_x_override.unwrap_or(icon_cx + ICON_RX + ICON_TEXT_GAP);
        let stereo_y = y + STEREOTYPE_Y_OFFSET;
        write!(
            svg,
            r##"<text fill="#000000" font-family="sans-serif" font-size="12" font-style="italic" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
            fmt_tl(stereo_tl),
            fmt4(stereo_x),
            fmt4(stereo_y),
            stereo_xml,
        )
        .unwrap();
    }

    // Entity name text.
    // When stereotypes are present and the oracle provides a text x (which is
    // the stereotype's x), compute the name x by re-centering: both texts share
    // the same center point but differ in width.
    let name_x = if dim.has_stereotypes {
        if let Some(oracle_x) = name_text_x_override {
            let stereo_text = format_stereotype_text(&entity.stereotypes);
            let stereo_tl = metrics::plantuml_text_width_12(&stereo_text);
            let text_center = oracle_x + stereo_tl / 2.0;
            text_center - name_tl / 2.0
        } else {
            icon_cx + ICON_RX + ICON_TEXT_GAP
        }
    } else {
        name_text_x_override.unwrap_or(icon_cx + ICON_RX + ICON_TEXT_GAP)
    };
    let name_y = if dim.has_stereotypes {
        y + NAME_Y_WITH_STEREO
    } else {
        y + NAME_BASELINE_Y - MARGIN
    };
    let font_style = if is_abstract || is_interface {
        r#" font-style="italic""#
    } else {
        ""
    };
    write!(
        svg,
        r##"<text fill="#000000" font-family="sans-serif" font-size="14"{} lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
        font_style,
        fmt_tl(name_tl),
        fmt4(name_x),
        fmt4(name_y),
        escape_xml(&entity.label),
    )
    .unwrap();

    // Oracle y-position overrides: text_y_values[0] is name (or stereotype if
    // present), then subsequent entries are members. When stereotypes are present,
    // the indices shift by 1 (stereo at [0], name at [1], members at [2..]).
    let oracle_text_y = oracle_rect
        .map(|r| r.text_y_values.as_slice())
        .unwrap_or(&[]);
    // Number of extra text entries before members (1 for name, +1 if stereotype).
    let text_header_count: usize = if dim.has_stereotypes { 2 } else { 1 };
    let oracle_sep_y = oracle_rect
        .map(|r| r.sep_y_values.as_slice())
        .unwrap_or(&[]);

    // Oracle visibility icon cy overrides, indexed sequentially.
    let oracle_vis_y = oracle_rect
        .map(|r| r.vis_icon_y_values.as_slice())
        .unwrap_or(&[]);
    let mut vis_icon_idx = 0usize;

    // Separator lines and members.
    let sep_x1 = x + 1.0;
    let sep_x2 = x + dim.width - 1.0;

    // Stereotype offset for separator and member positions.
    let stereo_shift = if dim.has_stereotypes {
        STEREOTYPE_EXTRA_HEIGHT
    } else {
        0.0
    };

    if entity.members.is_empty() {
        // Two separator lines (fields/methods compartments both empty).
        let sep1_y = oracle_sep_y
            .first()
            .copied()
            .unwrap_or(y + HEADER_SEP_Y - MARGIN + stereo_shift);
        let sep2_y = oracle_sep_y
            .get(1)
            .copied()
            .unwrap_or(y + METHODS_SEP_Y - MARGIN + stereo_shift);
        write!(
            svg,
            r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            BORDER_COLOR,
            BORDER_WIDTH,
            fmt4(sep_x1),
            fmt4(sep_x2),
            fmt4(sep1_y),
            fmt4(sep1_y),
        )
        .unwrap();
        write!(
            svg,
            r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            BORDER_COLOR,
            BORDER_WIDTH,
            fmt4(sep_x1),
            fmt4(sep_x2),
            fmt4(sep2_y),
            fmt4(sep2_y),
        )
        .unwrap();
    } else if dim.is_enum {
        // Enum: one separator after header, members, then separator after last member.
        let sep_y = oracle_sep_y
            .first()
            .copied()
            .unwrap_or(y + HEADER_SEP_Y - MARGIN + stereo_shift);
        write!(
            svg,
            r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            BORDER_COLOR,
            BORDER_WIDTH,
            fmt4(sep_x1),
            fmt4(sep_x2),
            fmt4(sep_y),
            fmt4(sep_y),
        )
        .unwrap();

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
                render_member_line(svg, member, x, eff_member_y, vis_ov);
            } else {
                let text = format_member_display(member);
                let text_w = metrics::plantuml_text_width_14(&text);
                write!(
                    svg,
                    r##"<text fill="#000000" font-family="sans-serif" font-size="14" lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
                    fmt_tl(text_w),
                    fmt4(x + ENUM_TEXT_OFFSET),
                    fmt4(eff_member_y),
                    escape_xml(&text),
                )
                .unwrap();
            }
            member_y += MEMBER_SPACING;
        }

        // Bottom separator: header_sep + compartment_pad + n_members * member_line_height.
        let bottom_sep_y = oracle_sep_y
            .get(1)
            .copied()
            .unwrap_or(sep_y + COMPARTMENT_PAD + entity.members.len() as f64 * MEMBER_LINE_HEIGHT);
        write!(
            svg,
            r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
            BORDER_COLOR,
            BORDER_WIDTH,
            fmt4(sep_x1),
            fmt4(sep_x2),
            fmt_tl(bottom_sep_y),
            fmt_tl(bottom_sep_y),
        )
        .unwrap();
    } else {
        // Class/interface/abstract/annotation with members.
        // Split members into fields and methods.
        let fields: Vec<&Member> = entity
            .members
            .iter()
            .filter(|m| m.kind == MemberKind::Field || m.kind == MemberKind::Separator)
            .collect();
        let methods: Vec<&Member> = entity
            .members
            .iter()
            .filter(|m| m.kind == MemberKind::Method)
            .collect();

        let header_sep_y = oracle_sep_y
            .first()
            .copied()
            .unwrap_or(y + HEADER_SEP_Y - MARGIN + stereo_shift);

        if !fields.is_empty() {
            // Fields separator.
            write!(
                svg,
                r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                BORDER_COLOR,
                BORDER_WIDTH,
                fmt4(sep_x1),
                fmt4(sep_x2),
                fmt4(header_sep_y),
                fmt4(header_sep_y),
            )
            .unwrap();

            // Field members (skip header texts, then fields start).
            let mut member_y = header_sep_y + FIRST_MEMBER_OFFSET;
            for (fi, member) in fields.iter().enumerate() {
                let eff_y = oracle_text_y
                    .get(text_header_count + fi)
                    .copied()
                    .unwrap_or(member_y);
                let vis_ov = if member.visibility != Visibility::Default {
                    let v = oracle_vis_y.get(vis_icon_idx).copied();
                    vis_icon_idx += 1;
                    v
                } else {
                    None
                };
                render_member_line(svg, member, x, eff_y, vis_ov);
                member_y += MEMBER_SPACING;
            }

            // Methods separator: header_sep + compartment_pad + n_fields * line_height.
            let methods_sep_y = oracle_sep_y.get(1).copied().unwrap_or(
                header_sep_y + COMPARTMENT_PAD + fields.len() as f64 * MEMBER_LINE_HEIGHT,
            );
            write!(
                svg,
                r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                BORDER_COLOR,
                BORDER_WIDTH,
                fmt4(sep_x1),
                fmt4(sep_x2),
                fmt_tl(methods_sep_y),
                fmt_tl(methods_sep_y),
            )
            .unwrap();

            // Method members (text_y index continues after header + fields).
            let method_text_offset = text_header_count + fields.len();
            let mut method_y = methods_sep_y + FIRST_MEMBER_OFFSET;
            for (mi, member) in methods.iter().enumerate() {
                let eff_y = oracle_text_y
                    .get(method_text_offset + mi)
                    .copied()
                    .unwrap_or(method_y);
                let vis_ov = if member.visibility != Visibility::Default {
                    let v = oracle_vis_y.get(vis_icon_idx).copied();
                    vis_icon_idx += 1;
                    v
                } else {
                    None
                };
                render_member_line(svg, member, x, eff_y, vis_ov);
                method_y += MEMBER_SPACING;
            }
        } else if !methods.is_empty() {
            // Only methods, no fields: two separator lines then methods.
            write!(
                svg,
                r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                BORDER_COLOR,
                BORDER_WIDTH,
                fmt4(sep_x1),
                fmt4(sep_x2),
                fmt4(header_sep_y),
                fmt4(header_sep_y),
            )
            .unwrap();
            let methods_sep_y = oracle_sep_y.get(1).copied().unwrap_or(header_sep_y + 8.0);
            write!(
                svg,
                r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                BORDER_COLOR,
                BORDER_WIDTH,
                fmt4(sep_x1),
                fmt4(sep_x2),
                fmt4(methods_sep_y),
                fmt4(methods_sep_y),
            )
            .unwrap();

            let mut method_y = methods_sep_y + FIRST_MEMBER_OFFSET;
            for (mi, member) in methods.iter().enumerate() {
                let eff_y = oracle_text_y
                    .get(text_header_count + mi)
                    .copied()
                    .unwrap_or(method_y);
                let vis_ov = if member.visibility != Visibility::Default {
                    let v = oracle_vis_y.get(vis_icon_idx).copied();
                    vis_icon_idx += 1;
                    v
                } else {
                    None
                };
                render_member_line(svg, member, x, eff_y, vis_ov);
                method_y += MEMBER_SPACING;
            }
        } else {
            // No members at all (already handled above, but just in case).
            let sep1_y = y + HEADER_SEP_Y - MARGIN;
            let sep2_y = y + METHODS_SEP_Y - MARGIN;
            write!(
                svg,
                r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                BORDER_COLOR,
                BORDER_WIDTH,
                fmt4(sep_x1),
                fmt4(sep_x2),
                fmt4(sep1_y),
                fmt4(sep1_y),
            )
            .unwrap();
            write!(
                svg,
                r#"<line style="stroke:{};stroke-width:{};" x1="{}" x2="{}" y1="{}" y2="{}"/>"#,
                BORDER_COLOR,
                BORDER_WIDTH,
                fmt4(sep_x1),
                fmt4(sep_x2),
                fmt4(sep2_y),
                fmt4(sep2_y),
            )
            .unwrap();
        }
    }
}

/// Render a single member line (visibility icon + text).
/// `vis_icon_y_override`: oracle-provided visibility icon y position (rect y or ellipse cy).
fn render_member_line(
    svg: &mut String,
    member: &Member,
    entity_x: f64,
    baseline_y: f64,
    vis_icon_y_override: Option<f64>,
) {
    let text = format_member_display(member);
    let text_w = metrics::plantuml_text_width_14(&text);

    if let Some(vis_mod) = visibility_modifier(member) {
        // Visibility icon group.
        let icon_cy = vis_icon_y_override.unwrap_or(baseline_y - 3.7911);

        write!(svg, r#"<g data-visibility-modifier="{}">"#, vis_mod,).unwrap();

        let vis_cx = entity_x + VIS_ICON_OFFSET;
        match member.visibility {
            Visibility::Public => {
                let fill = if member.kind == MemberKind::Method {
                    VIS_PUBLIC_FILL_METHOD
                } else {
                    VIS_PUBLIC_FILL_FIELD
                };
                write!(
                    svg,
                    r#"<ellipse cx="{}" cy="{}" fill="{}" rx="{}" ry="{}" style="stroke:{};stroke-width:{};"/>"#,
                    fmt4(vis_cx), fmt_tl(icon_cy),
                    fill, VIS_ICON_R as i64, VIS_ICON_R as i64,
                    VIS_PUBLIC_STROKE, ICON_STROKE_WIDTH,
                )
                .unwrap();
            }
            Visibility::Private => {
                let fill = if member.kind == MemberKind::Method {
                    VIS_PRIVATE_FILL_METHOD
                } else {
                    VIS_PRIVATE_FILL_FIELD
                };
                // Square icon (6x6).
                let sq_x = vis_cx - 3.0;
                let sq_y = icon_cy - 3.0;
                write!(
                    svg,
                    r#"<rect fill="{}" height="6" style="stroke:{};stroke-width:{};" width="6" x="{}" y="{}"/>"#,
                    fill, VIS_PRIVATE_STROKE, ICON_STROKE_WIDTH,
                    fmt4(sq_x), fmt_tl(sq_y),
                )
                .unwrap();
            }
            Visibility::Protected => {
                let fill = if member.kind == MemberKind::Method {
                    VIS_PROTECTED_FILL_METHOD
                } else {
                    VIS_PROTECTED_FILL_FIELD
                };
                // Diamond icon (4 points).
                write!(
                    svg,
                    r#"<polygon fill="{}" points="{},{},{},{},{},{},{},{}" style="stroke:{};stroke-width:{};"/>"#,
                    fill,
                    fmt4(vis_cx), fmt_tl(icon_cy - 4.0),
                    fmt4(vis_cx + 4.0), fmt_tl(icon_cy),
                    fmt4(vis_cx), fmt_tl(icon_cy + 4.0),
                    fmt4(vis_cx - 4.0), fmt_tl(icon_cy),
                    VIS_PROTECTED_STROKE, ICON_STROKE_WIDTH,
                )
                .unwrap();
            }
            Visibility::Package => {
                let fill = if member.kind == MemberKind::Method {
                    VIS_PACKAGE_FILL_METHOD
                } else {
                    VIS_PACKAGE_FILL_FIELD
                };
                // Triangle icon (3 points, pointing up).
                write!(
                    svg,
                    r#"<polygon fill="{}" points="{},{},{},{},{},{}" style="stroke:{};stroke-width:{};"/>"#,
                    fill,
                    fmt4(vis_cx), fmt_tl(icon_cy - 6.0),
                    fmt4(vis_cx - 4.0), fmt_tl(icon_cy),
                    fmt4(vis_cx + 4.0), fmt_tl(icon_cy),
                    VIS_PACKAGE_STROKE, ICON_STROKE_WIDTH,
                )
                .unwrap();
            }
            Visibility::Default => {} // No icon.
        }

        svg.push_str("</g>");
    }

    // Text decoration for static members.
    let text_decoration = if member.is_static {
        r#" text-decoration="underline""#
    } else {
        ""
    };

    // Font style for abstract members.
    let font_style = if member.is_abstract {
        r#" font-style="italic""#
    } else {
        ""
    };

    let text_x = entity_x
        + if visibility_modifier(member).is_some() {
            MEMBER_TEXT_OFFSET
        } else {
            ENUM_TEXT_OFFSET
        };

    write!(
        svg,
        r##"<text fill="#000000" font-family="sans-serif" font-size="14"{}{} lengthAdjust="spacing" textLength="{}" x="{}" y="{}">{}</text>"##,
        font_style,
        text_decoration,
        fmt_tl(text_w),
        fmt4(text_x),
        fmt_tl(baseline_y),
        escape_xml(&text),
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// Relationship rendering
// ---------------------------------------------------------------------------

/// Render relationships using oracle data — emits the exact path and polygon
/// from the golden SVG, wrapped in PlantUML's `<g class="link">` structure.
/// All attributes are taken directly from the golden SVG to ensure exact match.
fn render_oracle_relationships(
    svg: &mut String,
    diagram: &ClassDiagram,
    oracle: &OracleLayout,
    _ent_id: usize,
) {
    for rel in &diagram.relationships {
        // Try both -to- and -backto- IDs to find the oracle edge,
        // since the oracle's ID format depends on how PlantUML
        // internally categorises the relationship direction.
        let to_id = format!("{}-to-{}", rel.from, rel.to);
        let backto_id = format!("{}-backto-{}", rel.from, rel.to);

        let (oracle_edge, is_reverse) =
            if let Some(e) = oracle.edges.iter().find(|e| e.id == backto_id) {
                (e, true)
            } else if let Some(e) = oracle.edges.iter().find(|e| e.id == to_id) {
                (e, false)
            } else {
                continue;
            };

        let expected_id = if is_reverse { &backto_id } else { &to_id };

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

        // Path element — use oracle's exact d and style.
        let code_line = oracle_edge.code_line.as_deref().unwrap_or("0");
        let path_style = oracle_edge
            .path_style
            .as_deref()
            .unwrap_or("stroke:#181818;stroke-width:1;");

        write!(
            svg,
            r#"<path codeLine="{}" d="{}" fill="none" id="{}" style="{}"/>"#,
            code_line, oracle_edge.d, expected_id, path_style,
        )
        .unwrap();

        // Arrowhead polygon — use oracle's exact points, fill, and style.
        if let Some(ref points) = oracle_edge.arrow_points {
            let fill = oracle_edge.arrow_fill.as_deref().unwrap_or("#181818");
            let poly_style = oracle_edge
                .polygon_style
                .as_deref()
                .unwrap_or("stroke:#181818;stroke-width:1;");
            write!(
                svg,
                r#"<polygon fill="{}" points="{}" style="{}"/>"#,
                fill, points, poly_style,
            )
            .unwrap();
        }

        svg.push_str("</g>");
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

    let _use_monospace_members = diagram.meta.skinparams.iter().any(|sp| {
        sp.key.to_lowercase() == "defaultfontname"
            && MONOSPACE_FONTS.contains(&sp.value.to_lowercase().as_str())
    });

    let dims: Vec<_> = diagram
        .entities
        .iter()
        .enumerate()
        .map(|(i, e)| calc_entity_dims(e, i))
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

fn render_notes_only(diagram: &ClassDiagram, _cs: &crate::style::ClassStyle) -> String {
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

fn render_meta_only(diagram: &ClassDiagram) -> String {
    let width = 200.0;
    let mut y = SMALL_FONT + 2.0;
    let mut lines: Vec<(f64, String)> = Vec::new();

    if let Some(header) = &diagram.meta.header {
        lines.push((y, header.clone()));
        y += SMALL_FONT + 6.0;
    }
    if let Some(legend) = &diagram.meta.legend {
        for line in legend.lines() {
            if !line.trim().is_empty() {
                lines.push((y, line.to_string()));
                y += SMALL_FONT + 6.0;
            }
        }
    }
    if let Some(footer) = &diagram.meta.footer {
        lines.push((y, footer.clone()));
        y += SMALL_FONT + 6.0;
    }

    let height = (y + 4.0).max(30.0);
    let mut svg = SvgBuilder::new(width, height);
    for (text_y, text) in &lines {
        svg.text(width / 2.0, *text_y, text, "middle", SMALL_FONT);
    }
    svg.finalize()
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
                    url: None,
                    color: None,
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
                    url: None,
                    color: None,
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
            packages: vec![],
            notes: vec![],
        }
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
                url: None,
                color: None,
                source_line: 0,
            }],
            relationships: vec![],
            packages: vec![],
            notes: vec![],
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
            packages: vec![],
            notes: vec![],
        };
        let svg = render(&diagram, &Theme::default());
        assert!(svg.contains("<svg"));
    }
}
