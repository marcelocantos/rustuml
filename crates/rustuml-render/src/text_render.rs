// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Creole-aware `<text>` emission.
//!
//! PlantUML emits styled text by **splitting on style transitions and
//! producing one `<text>` element per uniform run** (no `<tspan>` wrappers
//! anywhere in the golden corpus). Each text element advertises its
//! `textLength` and gets a calculated `x` offset, so the runs line up
//! seamlessly. This module owns that emission shape so every renderer
//! routes through one path.
//!
//! Width calculation currently uses PlantUML's sans-serif metrics for all
//! segments — monospace runs will mismatch on `textLength` until those
//! metrics land. The structural shape (text content, font-family/style
//! attributes, NBSP conversion, per-segment positioning) is correct
//! regardless.

use std::fmt::Write;

use crate::creole::{self, Segment, Style};
use crate::filter_registry;
use crate::plantuml_metrics as pm;

/// Effective styling for a base font that the caller controls. Each call to
/// [`emit_text`] starts from this base; segment-level styles add on top.
#[derive(Debug, Clone)]
pub struct TextBase<'a> {
    pub x: f64,
    pub y: f64,
    pub font_size: u32,
    pub font_family: &'a str,
    pub fill: &'a str,
    /// Whether the base text is bold (e.g. class entity names emit bold by
    /// default; creole inside still toggles bold normally).
    pub bold: bool,
    /// Whether the base text is italic.
    pub italic: bool,
    /// Whether the base text is underlined (e.g. static class members).
    /// OR-merged with any creole-driven underline on the segment style.
    pub underline: bool,
    /// When true, treat `__` as literal (class-entity labels).
    pub skip_underline: bool,
}

/// Emit one or more `<text>` elements covering `content` with creole markup
/// resolved. Writes to `buf`. Returns the total advance width.
pub fn emit_text(buf: &mut String, content: &str, base: &TextBase<'_>) -> f64 {
    let segments = if base.skip_underline {
        creole::parse_segments_no_underline(content)
    } else {
        creole::parse_segments(content)
    };
    emit_segments(buf, &segments, base)
}

/// Like [`emit_text`] but neutralises monospace styling: `""..."" ` and
/// backtick runs render as plain (sans-serif) text. PlantUML's edge/link
/// labels parse creole markup (bold, italic, size, colour) but, unlike
/// entity bodies, do *not* honour the monospace delimiter — the `""` glue
/// is consumed and the inner text falls back to the surrounding font.
pub fn emit_text_no_mono(buf: &mut String, content: &str, base: &TextBase<'_>) -> f64 {
    let mut segments = if base.skip_underline {
        creole::parse_segments_no_underline(content)
    } else {
        creole::parse_segments(content)
    };
    for seg in &mut segments {
        seg.style.monospace = false;
    }
    emit_segments(buf, &segments, base)
}

/// Number of distinct baseline y-values [`emit_text`] will produce for one
/// logical line. Uniform-style Creole emits several `<text>` elements but only
/// one baseline; mixed font sizes/families can consume multiple oracle y slots.
pub fn emitted_baseline_count(content: &str, base: &TextBase<'_>) -> usize {
    let segments = if base.skip_underline {
        creole::parse_segments_no_underline(content)
    } else {
        creole::parse_segments(content)
    };
    if segments.is_empty() {
        return 1;
    }
    let first = &segments[0];
    let first_size = first.style.size.unwrap_or(base.font_size) as f64;
    let line_bottom_drop = clamp_drop(first_size, segment_metric_family(first, base));
    let mut offsets: Vec<f64> = Vec::new();
    for seg in &segments {
        let offset = line_bottom_drop
            - clamp_drop(
                effective_font_size(seg, base),
                segment_metric_family(seg, base),
            );
        if offsets
            .last()
            .is_none_or(|&last: &f64| (last - offset).abs() > 0.001)
        {
            offsets.push(offset);
        }
    }
    offsets.len().max(1)
}

/// Width of `content` after creole resolution — the value a renderer needs
/// to size boxes around a label. Per-segment styling (monospace vs sans-
/// serif, bold, custom size) is honoured by routing through `total_width`.
pub fn measure(content: &str, font_size: f64, bold: bool) -> f64 {
    measure_inner(content, font_size, bold, false)
}

/// Width of `content` using a caller-selected base font family. This preserves
/// Creole segment overrides, while allowing diagram-wide `defaultFontName`
/// skinparams to drive both emitted `font-family` and box/layout metrics.
pub fn measure_with_family(content: &str, font_size: f64, bold: bool, font_family: &str) -> f64 {
    measure_inner_with_family(content, font_size, bold, false, font_family)
}

/// Measure variant for class-entity labels where `__` is a literal pair of
/// underscores (not underline markup). The literal `__` therefore counts
/// toward textLength.
pub fn measure_no_underline(content: &str, font_size: f64, bold: bool) -> f64 {
    measure_inner(content, font_size, bold, true)
}

/// Class-label measurement with literal `__` and caller-selected base family.
pub fn measure_no_underline_with_family(
    content: &str,
    font_size: f64,
    bold: bool,
    font_family: &str,
) -> f64 {
    measure_inner_with_family(content, font_size, bold, true, font_family)
}

fn measure_inner(content: &str, font_size: f64, bold: bool, skip_underline: bool) -> f64 {
    measure_inner_with_family(content, font_size, bold, skip_underline, "sans-serif")
}

fn measure_inner_with_family(
    content: &str,
    font_size: f64,
    bold: bool,
    skip_underline: bool,
    font_family: &str,
) -> f64 {
    let base = TextBase {
        x: 0.0,
        y: 0.0,
        font_size: font_size as u32,
        font_family,
        fill: "#000000",
        bold,
        italic: false,
        underline: false,
        skip_underline,
    };
    total_width(content, &base)
}

/// Height of `content` after creole resolution — the value a renderer
/// needs to size boxes around a label vertically. Returns the max of
/// per-segment heights (mono vs sans-serif) at the resolved font size.
///
/// Lines containing `<sub>` or `<sup>` segments grow by `|getSpace()|` to
/// accommodate the descender / ascender beyond the line — matching Java
/// PlantUML's `TileText.spaceBottom`.
pub fn label_height(content: &str, font_size: f64) -> f64 {
    label_height_with_family(content, font_size, "sans-serif")
}

/// Height of `content` using the caller's base font family.
pub fn label_height_with_family(content: &str, font_size: f64, font_family: &str) -> f64 {
    let segments = creole::parse_segments(content);
    if segments.is_empty() {
        return family_text_height(font_size, metric_family(font_family));
    }
    let base_height = segments
        .iter()
        .map(|seg| {
            let size = seg.style.size.map(|s| s as f64).unwrap_or(font_size);
            family_text_height(size, segment_metric_family_for_family(seg, font_family))
        })
        .fold(0.0f64, f64::max);
    base_height + line_extra_space(&segments)
}

/// Ascent for vertical positioning of the text baseline within a label box.
/// Picks the max of per-segment ascents (matches PlantUML's behaviour for
/// mixed-font lines).
///
/// When `<sub>` is present, the baseline shifts down by the sub `getSpace()`
/// so the descender stays within the line — the ascent therefore grows.
/// `<sup>` does not affect ascent (the sup glyph extends above the existing
/// ascent but doesn't move the baseline).
pub fn label_ascent(content: &str, font_size: f64) -> f64 {
    label_ascent_with_family(content, font_size, "sans-serif")
}

/// Ascent of `content` using the caller's base font family.
pub fn label_ascent_with_family(content: &str, font_size: f64, font_family: &str) -> f64 {
    let segments = creole::parse_segments(content);
    if segments.is_empty() {
        return family_ascent(font_size, metric_family(font_family));
    }
    let base_ascent = segments
        .iter()
        .map(|seg| {
            let size = seg.style.size.map(|s| s as f64).unwrap_or(font_size);
            family_ascent(size, segment_metric_family_for_family(seg, font_family))
        })
        .fold(0.0f64, f64::max);
    base_ascent + sub_extra_space(&segments)
}

pub(crate) fn text_height_for_family(font_size: f64, font_family: &str) -> f64 {
    family_text_height(font_size, metric_family(font_family))
}

pub(crate) fn ascent_for_family(font_size: f64, font_family: &str) -> f64 {
    family_ascent(font_size, metric_family(font_family))
}

/// Extra vertical space the line needs beyond the maximum atom height to
/// fit `<sub>` descenders and `<sup>` ascenders — matches Java's
/// `TileText.spaceBottom = abs(getSpace())` per atom kind. We treat sub
/// and sup as additive (line can host both), capped at one each.
fn line_extra_space(segments: &[Segment]) -> f64 {
    let mut has_sub = false;
    let mut has_sup = false;
    for s in segments {
        match s.style.baseline_shift {
            Some("sub") => has_sub = true,
            Some("super") => has_sup = true,
            _ => {}
        }
    }
    (if has_sub { 3.0 } else { 0.0 }) + (if has_sup { 6.0 } else { 0.0 })
}

/// Portion of the extra space that pushes the baseline down (sub glyphs
/// only — sup extends upward from the same baseline).
fn sub_extra_space(segments: &[Segment]) -> f64 {
    if segments
        .iter()
        .any(|s| matches!(s.style.baseline_shift, Some("sub")))
    {
        3.0
    } else {
        0.0
    }
}

/// Pre-computed widths for the segments. Useful when the caller needs the
/// total advance for layout before deciding `x`.
pub fn total_width(content: &str, base: &TextBase<'_>) -> f64 {
    let segments = if base.skip_underline {
        creole::parse_segments_no_underline(content)
    } else {
        creole::parse_segments(content)
    };
    segments
        .iter()
        .map(|seg| segment_width(seg, base))
        .sum::<f64>()
}

/// Shared emission body: walks segments, writes one `<text>` per segment,
/// advancing `x` by each segment's width. Returns the sum of widths.
///
/// PlantUML strips leading and trailing ASCII whitespace from each segment's
/// emitted text but counts those spaces in the cumulative x-advance. So
/// `**bold** State` emits a "bold" element followed by a "State" element
/// whose x is shifted right by one space width (the leading space of
/// segment 2 becomes a gap between the two text elements). Monospace
/// segments use NBSP (U+00A0) instead of ASCII space; NBSP is not stripped.
fn emit_segments(buf: &mut String, segments: &[Segment], base: &TextBase<'_>) -> f64 {
    if segments.is_empty() {
        // Nothing to emit. Still produce an empty <text> with zero width so
        // surrounding layout stays consistent — matches PlantUML behaviour
        // for empty labels.
        write_text_element(
            buf,
            "",
            base,
            &Style::default(),
            base.x,
            0.0,
            pm::descent(base.font_size as f64),
        );
        return 0.0;
    }

    // When a line mixes font sizes OR font families, PlantUML bottom-aligns
    // the runs to a shared line bottom (`Sea.doAlign` translates each atom by
    // `-height` so every box bottom sits at the same y; `translateMinYto`
    // then pins the line top). Each run's baseline within its box is
    // `boxTop + ascent(ownRun)`, so its distance up from the line bottom is
    // `boxHeight(ownRun) - ascent(ownRun)`, where `boxHeight = max(textHeight,
    // 10)` — PlantUML clamps every atom's height to a 10px floor
    // (`AtomText.calculateDimensionSlow`: `if (h < 10) h = 10`).
    //
    // The oracle hands us the *first* run's baseline (the leftmost `<text>` on
    // the line), so the line bottom is `base.y + clampDrop(firstRun)` and
    // every run's baseline offset from `base.y` is
    // `clampDrop(firstRun) - clampDrop(ownRun)`. For sizes whose textHeight is
    // already ≥ 10 (size ≳ 9) the clamp is inert and `clampDrop == descent`,
    // so this reduces to the historical descent-difference and uniform lines
    // are unaffected. The clamp only changes mixed lines that include a
    // size ≤ 8 run, where the 10px floor lifts the small run's baseline.
    // Monospace and sans-serif have different metrics even at the same size,
    // so the run's font family matters too.
    let first = &segments[0];
    let first_size = first.style.size.unwrap_or(base.font_size) as f64;
    let line_bottom_drop = clamp_drop(first_size, segment_metric_family(first, base));

    let mut x = base.x;
    let mut total = 0.0;
    for seg in segments {
        let full_w = segment_width(seg, base);
        let (lead_w, trimmed_text, trimmed_w) = trim_segment_for_emit(seg, base);
        write_text_element(
            buf,
            &trimmed_text,
            base,
            &seg.style,
            x + lead_w,
            trimmed_w,
            line_bottom_drop,
        );
        x += full_w;
        total += full_w;
    }
    total
}

/// Distance from a run's text baseline up to the shared line bottom when
/// PlantUML bottom-aligns mixed-metric runs (`Sea` alignment). Equals
/// `boxHeight - ascent`, where `boxHeight = max(textHeight, 10)` applies
/// PlantUML's per-atom 10px height floor (`AtomText.calculateDimensionSlow`).
/// For sizes with `textHeight ≥ 10` (size ≳ 9) the floor is inert and this
/// equals the plain font descent.
fn clamp_drop(font_size: f64, family: MetricFamily) -> f64 {
    let (text_height, ascent) = (
        family_text_height(font_size, family),
        family_ascent(font_size, family),
    );
    text_height.max(10.0) - ascent
}

/// Trim leading/trailing ASCII whitespace from a segment's emitted text.
/// Returns the leading-whitespace advance width, the trimmed text body,
/// and the width of the trimmed text body. Spaces are stripped from the
/// rendered text but still contribute to the segment's cumulative advance
/// (handled by the caller).
///
/// A pure-whitespace segment (e.g. the single space between `**bold**`
/// and `//italic//`) is left untrimmed — Java PlantUML emits these as
/// literal-space `<text>` elements with their own `textLength`.
fn trim_segment_for_emit(seg: &Segment, base: &TextBase<'_>) -> (f64, String, f64) {
    // Monospace segments use NBSP instead of ASCII space; PlantUML does
    // not trim NBSP, so pass through unchanged.
    if seg.style.monospace {
        let w = segment_width(seg, base);
        return (0.0, seg.text.clone(), w);
    }
    // Count leading and trailing ASCII spaces in the *escaped* form. The
    // escape form only differs for non-space characters, so a leading
    // space stays a leading space.
    let bytes = seg.text.as_bytes();
    let mut lead = 0;
    while lead < bytes.len() && bytes[lead] == b' ' {
        lead += 1;
    }
    // Whole-segment whitespace: PlantUML emits these as literal-space
    // `<text>` elements but converts each ASCII space to NBSP (U+00A0)
    // in the rendered text. Width still counts as a normal ASCII space.
    if lead == bytes.len() {
        let w = segment_width(seg, base);
        let nbsp_text = "\u{00a0}".repeat(lead);
        return (0.0, nbsp_text, w);
    }
    let mut trail = 0;
    while trail < bytes.len() - lead && bytes[bytes.len() - 1 - trail] == b' ' {
        trail += 1;
    }
    if lead == 0 && trail == 0 {
        let w = segment_width(seg, base);
        return (0.0, seg.text.clone(), w);
    }
    let trimmed = &seg.text[lead..seg.text.len() - trail];
    let bold = base.bold || seg.style.bold;
    let font_size = effective_font_size(seg, base);
    let family = segment_metric_family(seg, base);
    let lead_w = family_text_width(&" ".repeat(lead), font_size, bold, family);
    let trimmed_w = family_text_width(&unescape_for_metrics(trimmed), font_size, bold, family);
    (lead_w, trimmed.to_string(), trimmed_w)
}

/// Width of one segment under the effective styling. Routes to the
/// monospace metric path when the segment is monospaced; otherwise uses
/// sans-serif (with linear scaling for non-tabulated font sizes).
fn segment_width(seg: &Segment, base: &TextBase<'_>) -> f64 {
    let raw = unescape_for_metrics(&seg.text);
    let bold = base.bold || seg.style.bold;
    let font_size = effective_font_size(seg, base);
    family_text_width(&raw, font_size, bold, segment_metric_family(seg, base))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MetricFamily {
    Sans,
    Mono,
    Arial,
    Verdana,
}

fn segment_metric_family(seg: &Segment, base: &TextBase<'_>) -> MetricFamily {
    if seg.style.monospace {
        if explicit_monospace_font_uses_surrounding_metrics(&seg.style) {
            metric_family(base.font_family)
        } else {
            MetricFamily::Mono
        }
    } else {
        seg.style
            .font_family
            .as_deref()
            .map(metric_family)
            .unwrap_or_else(|| metric_family(base.font_family))
    }
}

fn segment_metric_family_for_family(seg: &Segment, font_family: &str) -> MetricFamily {
    if seg.style.monospace {
        if explicit_monospace_font_uses_surrounding_metrics(&seg.style) {
            metric_family(font_family)
        } else {
            MetricFamily::Mono
        }
    } else {
        seg.style
            .font_family
            .as_deref()
            .map(metric_family)
            .unwrap_or_else(|| metric_family(font_family))
    }
}

fn style_metric_family(style: &Style, base: &TextBase<'_>) -> MetricFamily {
    if style.monospace {
        if explicit_monospace_font_uses_surrounding_metrics(style) {
            metric_family(base.font_family)
        } else {
            MetricFamily::Mono
        }
    } else {
        style
            .font_family
            .as_deref()
            .map(metric_family)
            .unwrap_or_else(|| metric_family(base.font_family))
    }
}

fn explicit_monospace_font_uses_surrounding_metrics(style: &Style) -> bool {
    style.monospace
        && style.font_family.as_deref().is_some_and(|family| {
            matches!(
                family
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_ascii_lowercase()
                    .as_str(),
                "monospace" | "monospaced"
            )
        })
}

fn metric_family(font_family: &str) -> MetricFamily {
    let normalized = font_family
        .trim_matches(|c| c == '"' || c == '\'')
        .to_ascii_lowercase();
    match normalized.as_str() {
        "courier" | "courier new" | "monospace" | "monospaced" => MetricFamily::Mono,
        "arial" => MetricFamily::Arial,
        "verdana" => MetricFamily::Verdana,
        _ => MetricFamily::Sans,
    }
}

/// Compute the effective rendered font size for a segment. PlantUML's
/// `FontPosition.mute(font)` decreases the size by 3 (minimum 2) for sub /
/// sup positioning, then the smaller font is used for both width
/// measurement and the emitted `font-size` attribute. The reduction is
/// applied AFTER any explicit `<size:N>` override.
fn effective_font_size(seg: &Segment, base: &TextBase<'_>) -> f64 {
    let nominal = seg
        .style
        .size
        .map(|s| s as f64)
        .unwrap_or(base.font_size as f64);
    if seg.style.baseline_shift.is_some() {
        (nominal - 3.0).max(2.0)
    } else {
        nominal
    }
}

/// Sans-serif text width with linear scaling for non-tabulated sizes.
///
/// `plantuml_metrics::text_width` tabulates sizes 10–14 exactly; for other
/// sizes it currently falls back to size 12 without scaling. Java AWT's
/// Lucida Grande (the underlying font for PlantUML's `SansSerif` on macOS)
/// has fractional metrics that scale linearly with font size, so we lift
/// the size-12 width by `size / 12` for sizes outside the table.
fn sans_text_width(text: &str, font_size: f64, bold: bool) -> f64 {
    let sz = font_size as u32;
    if (10..=14).contains(&sz) {
        pm::text_width(text, font_size, bold)
    } else {
        let base = pm::text_width(text, 12.0, bold);
        base * font_size / 12.0
    }
}

const ARIAL_WIDTH: [f64; 95] = [
    0.277832031250,
    0.277832031250,
    0.354980468750,
    0.556152343750,
    0.556152343750,
    0.889160156250,
    0.666992187500,
    0.190917968750,
    0.333007812500,
    0.333007812500,
    0.389160156250,
    0.583984375000,
    0.277832031250,
    0.333007812500,
    0.277832031250,
    0.277832031250,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.277832031250,
    0.277832031250,
    0.583984375000,
    0.583984375000,
    0.583984375000,
    0.556152343750,
    1.015136718750,
    0.666992187500,
    0.666992187500,
    0.722167968750,
    0.722167968750,
    0.666992187500,
    0.610839843750,
    0.777832031250,
    0.722167968750,
    0.277832031250,
    0.500000000000,
    0.666992187500,
    0.556152343750,
    0.833007812500,
    0.722167968750,
    0.777832031250,
    0.666992187500,
    0.777832031250,
    0.722167968750,
    0.666992187500,
    0.610839843750,
    0.722167968750,
    0.666992187500,
    0.943847656250,
    0.666992187500,
    0.666992187500,
    0.610839843750,
    0.277832031250,
    0.277832031250,
    0.277832031250,
    0.469238281250,
    0.556152343750,
    0.333007812500,
    0.556152343750,
    0.556152343750,
    0.500000000000,
    0.556152343750,
    0.556152343750,
    0.277832031250,
    0.556152343750,
    0.556152343750,
    0.222167968750,
    0.222167968750,
    0.500000000000,
    0.222167968750,
    0.833007812500,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.333007812500,
    0.500000000000,
    0.277832031250,
    0.556152343750,
    0.500000000000,
    0.722167968750,
    0.500000000000,
    0.500000000000,
    0.500000000000,
    0.333984375000,
    0.259765625000,
    0.333984375000,
    0.583984375000,
];

const ARIAL_BOLD_WIDTH: [f64; 95] = [
    0.277832031250,
    0.333007812500,
    0.474121093750,
    0.556152343750,
    0.556152343750,
    0.889160156250,
    0.722167968750,
    0.237792968750,
    0.333007812500,
    0.333007812500,
    0.389160156250,
    0.583984375000,
    0.277832031250,
    0.333007812500,
    0.277832031250,
    0.277832031250,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.556152343750,
    0.333007812500,
    0.333007812500,
    0.583984375000,
    0.583984375000,
    0.583984375000,
    0.610839843750,
    0.975097656250,
    0.722167968750,
    0.722167968750,
    0.722167968750,
    0.722167968750,
    0.666992187500,
    0.610839843750,
    0.777832031250,
    0.722167968750,
    0.277832031250,
    0.556152343750,
    0.722167968750,
    0.610839843750,
    0.833007812500,
    0.722167968750,
    0.777832031250,
    0.666992187500,
    0.777832031250,
    0.722167968750,
    0.666992187500,
    0.610839843750,
    0.722167968750,
    0.666992187500,
    0.943847656250,
    0.666992187500,
    0.666992187500,
    0.610839843750,
    0.333007812500,
    0.277832031250,
    0.333007812500,
    0.583984375000,
    0.556152343750,
    0.333007812500,
    0.556152343750,
    0.610839843750,
    0.556152343750,
    0.610839843750,
    0.556152343750,
    0.333007812500,
    0.610839843750,
    0.610839843750,
    0.277832031250,
    0.277832031250,
    0.556152343750,
    0.277832031250,
    0.889160156250,
    0.610839843750,
    0.610839843750,
    0.610839843750,
    0.610839843750,
    0.389160156250,
    0.556152343750,
    0.333007812500,
    0.610839843750,
    0.556152343750,
    0.777832031250,
    0.556152343750,
    0.556152343750,
    0.500000000000,
    0.389160156250,
    0.279785156250,
    0.389160156250,
    0.583984375000,
];

const VERDANA_WIDTH: [f64; 95] = [
    0.351562500000,
    0.393554687500,
    0.458984375000,
    0.818359375000,
    0.635742187500,
    1.076171875000,
    0.726562500000,
    0.268554687500,
    0.454101562500,
    0.454101562500,
    0.635742187500,
    0.818359375000,
    0.363769531250,
    0.454101562500,
    0.363769531250,
    0.454101562500,
    0.635742187500,
    0.635742187500,
    0.635742187500,
    0.635742187500,
    0.635742187500,
    0.635742187500,
    0.635742187500,
    0.635742187500,
    0.635742187500,
    0.635742187500,
    0.454101562500,
    0.454101562500,
    0.818359375000,
    0.818359375000,
    0.818359375000,
    0.545410156250,
    1.000000000000,
    0.683593750000,
    0.685546875000,
    0.698242187500,
    0.770507812500,
    0.632324218750,
    0.574707031250,
    0.775390625000,
    0.751464843750,
    0.420898437500,
    0.454589843750,
    0.692871093750,
    0.556640625000,
    0.842773437500,
    0.748046875000,
    0.787109375000,
    0.603027343750,
    0.787109375000,
    0.695312500000,
    0.683593750000,
    0.616210937500,
    0.731933593750,
    0.683593750000,
    0.988769531250,
    0.685058593750,
    0.615234375000,
    0.685058593750,
    0.454101562500,
    0.454101562500,
    0.454101562500,
    0.818359375000,
    0.635742187500,
    0.635742187500,
    0.600585937500,
    0.623046875000,
    0.520996093750,
    0.623046875000,
    0.595703125000,
    0.351562500000,
    0.623046875000,
    0.632812500000,
    0.274414062500,
    0.344238281250,
    0.591796875000,
    0.274414062500,
    0.972656250000,
    0.632812500000,
    0.606933593750,
    0.623046875000,
    0.623046875000,
    0.426757812500,
    0.520996093750,
    0.394042968750,
    0.632812500000,
    0.591796875000,
    0.818359375000,
    0.591796875000,
    0.591796875000,
    0.525390625000,
    0.634765625000,
    0.454101562500,
    0.634765625000,
    0.818359375000,
];

const VERDANA_BOLD_WIDTH: [f64; 95] = [
    0.341796875000,
    0.402343750000,
    0.587402343750,
    0.867187500000,
    0.710937500000,
    1.271972656250,
    0.862304687500,
    0.332031250000,
    0.543457031250,
    0.543457031250,
    0.710937500000,
    0.867187500000,
    0.361328125000,
    0.479980468750,
    0.361328125000,
    0.689453125000,
    0.710937500000,
    0.710937500000,
    0.710937500000,
    0.710937500000,
    0.710937500000,
    0.710937500000,
    0.710937500000,
    0.710937500000,
    0.710937500000,
    0.710937500000,
    0.402343750000,
    0.402343750000,
    0.867187500000,
    0.867187500000,
    0.867187500000,
    0.616699218750,
    0.963867187500,
    0.776367187500,
    0.761718750000,
    0.723632812500,
    0.830078125000,
    0.683105468750,
    0.650390625000,
    0.811035156250,
    0.837402343750,
    0.545898437500,
    0.555175781250,
    0.770996093750,
    0.637207031250,
    0.947753906250,
    0.846679687500,
    0.850097656250,
    0.732910156250,
    0.850097656250,
    0.782226562500,
    0.710449218750,
    0.681640625000,
    0.812011718750,
    0.763671875000,
    1.128417968750,
    0.763671875000,
    0.736816406250,
    0.691894531250,
    0.543457031250,
    0.689453125000,
    0.543457031250,
    0.867187500000,
    0.710937500000,
    0.710937500000,
    0.667968750000,
    0.699218750000,
    0.588378906250,
    0.699218750000,
    0.664062500000,
    0.422363281250,
    0.699218750000,
    0.712402343750,
    0.341796875000,
    0.402832031250,
    0.670898437500,
    0.341796875000,
    1.058105468750,
    0.712402343750,
    0.686523437500,
    0.699218750000,
    0.699218750000,
    0.497070312500,
    0.593261718750,
    0.455566406250,
    0.712402343750,
    0.649902343750,
    0.979492187500,
    0.668945312500,
    0.650878906250,
    0.596679687500,
    0.710937500000,
    0.543457031250,
    0.710937500000,
    0.867187500000,
];

fn family_table_text_width(
    text: &str,
    font_size: f64,
    bold: bool,
    plain_table: &[f64; 95],
    bold_table: &[f64; 95],
) -> f64 {
    let table = if bold { bold_table } else { plain_table };
    text.chars()
        .map(|c| {
            let code = c as usize;
            if (32..=126).contains(&code) {
                table[code - 32] * font_size
            } else {
                sans_text_width(&c.to_string(), font_size, bold)
            }
        })
        .sum()
}

fn family_text_width(text: &str, font_size: f64, bold: bool, family: MetricFamily) -> f64 {
    match family {
        MetricFamily::Mono => pm::mono_text_width(text, font_size),
        MetricFamily::Arial => {
            family_table_text_width(text, font_size, bold, &ARIAL_WIDTH, &ARIAL_BOLD_WIDTH)
        }
        MetricFamily::Verdana => {
            family_table_text_width(text, font_size, bold, &VERDANA_WIDTH, &VERDANA_BOLD_WIDTH)
        }
        MetricFamily::Sans => sans_text_width(text, font_size, bold),
    }
}

fn family_text_height(font_size: f64, family: MetricFamily) -> f64 {
    match family {
        MetricFamily::Mono => pm::mono_text_height(font_size),
        MetricFamily::Arial => font_size * 1.14990234375,
        MetricFamily::Verdana => font_size * 1.21533203125,
        MetricFamily::Sans => pm::text_height(font_size),
    }
}

fn family_ascent(font_size: f64, family: MetricFamily) -> f64 {
    match family {
        MetricFamily::Mono => pm::mono_ascent(font_size),
        // Java AWT's Arial line metrics carry non-zero leading. PlantUML's
        // text baseline behaves as ascent + leading, while the text height
        // above already includes the same leading.
        MetricFamily::Arial => font_size * 0.93798828125,
        MetricFamily::Verdana => font_size * 1.00537109375,
        MetricFamily::Sans => pm::ascent(font_size),
    }
}

/// Reverse XML escaping for metric calculation — PlantUML measures text
/// against the source string, not its escaped form.
fn unescape_for_metrics(s: &str) -> String {
    single_pass_unescape(s)
}

/// Decode XML entities in a single left-to-right pass so each `&…;` is
/// resolved exactly once. A naive chain of `.replace()` calls double-decodes:
/// `&amp;lt;` first becomes `&lt;` (correct — the source held a literal
/// `&lt;`) and is then wrongly collapsed to `<`. PlantUML keeps the literal
/// `&lt;` and measures all four glyphs, so we must not re-scan produced text.
pub(crate) fn single_pass_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        if let Some(t) = tail.strip_prefix("&amp;") {
            out.push('&');
            rest = t;
        } else if let Some(t) = tail.strip_prefix("&lt;") {
            out.push('<');
            rest = t;
        } else if let Some(t) = tail.strip_prefix("&gt;") {
            out.push('>');
            rest = t;
        } else if let Some(t) = tail.strip_prefix("&quot;") {
            out.push('"');
            rest = t;
        } else {
            out.push('&');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

/// Normalise a colour spec to PlantUML's preferred form: lower-case hex
/// (`#RRGGBB`) for known named colours, pass-through otherwise.
///
/// PlantUML emits resolved hex colours in SVG output even when the source
/// uses HTML colour names — `<color:blue>` becomes `fill="#0000FF"`.
pub(crate) fn normalize_color(s: &str) -> String {
    if s.starts_with('#') {
        return s.to_string();
    }
    let lower = s.to_ascii_lowercase();
    match css_color_hex(&lower) {
        Some(hex) => hex.to_string(),
        None => s.to_string(),
    }
}

/// CSS3 named colours, lowercased keys. PlantUML accepts the same names
/// (Java AWT's `Color.decode` plus extended palette).
fn css_color_hex(name: &str) -> Option<&'static str> {
    // Listed in case-insensitive lookup order; values are uppercase
    // `#RRGGBB` to match PlantUML's golden SVG output.
    match name {
        "aliceblue" => Some("#F0F8FF"),
        "antiquewhite" => Some("#FAEBD7"),
        "aqua" | "cyan" => Some("#00FFFF"),
        "aquamarine" => Some("#7FFFD4"),
        "azure" => Some("#F0FFFF"),
        "beige" => Some("#F5F5DC"),
        "bisque" => Some("#FFE4C4"),
        "black" => Some("#000000"),
        "blanchedalmond" => Some("#FFEBCD"),
        "blue" => Some("#0000FF"),
        "blueviolet" => Some("#8A2BE2"),
        "brown" => Some("#A52A2A"),
        "burlywood" => Some("#DEB887"),
        "cadetblue" => Some("#5F9EA0"),
        "chartreuse" => Some("#7FFF00"),
        "chocolate" => Some("#D2691E"),
        "coral" => Some("#FF7F50"),
        "cornflowerblue" => Some("#6495ED"),
        "cornsilk" => Some("#FFF8DC"),
        "crimson" => Some("#DC143C"),
        "darkblue" => Some("#00008B"),
        "darkcyan" => Some("#008B8B"),
        "darkgoldenrod" => Some("#B8860B"),
        "darkgray" | "darkgrey" => Some("#A9A9A9"),
        "darkgreen" => Some("#006400"),
        "darkkhaki" => Some("#BDB76B"),
        "darkmagenta" => Some("#8B008B"),
        "darkolivegreen" => Some("#556B2F"),
        "darkorange" => Some("#FF8C00"),
        "darkorchid" => Some("#9932CC"),
        "darkred" => Some("#8B0000"),
        "darksalmon" => Some("#E9967A"),
        "darkseagreen" => Some("#8FBC8F"),
        "darkslateblue" => Some("#483D8B"),
        "darkslategray" | "darkslategrey" => Some("#2F4F4F"),
        "darkturquoise" => Some("#00CED1"),
        "darkviolet" => Some("#9400D3"),
        "deeppink" => Some("#FF1493"),
        "deepskyblue" => Some("#00BFFF"),
        "dimgray" | "dimgrey" => Some("#696969"),
        "dodgerblue" => Some("#1E90FF"),
        "firebrick" => Some("#B22222"),
        "floralwhite" => Some("#FFFAF0"),
        "forestgreen" => Some("#228B22"),
        "fuchsia" | "magenta" => Some("#FF00FF"),
        "gainsboro" => Some("#DCDCDC"),
        "ghostwhite" => Some("#F8F8FF"),
        "gold" => Some("#FFD700"),
        "goldenrod" => Some("#DAA520"),
        "gray" | "grey" => Some("#808080"),
        "green" => Some("#008000"),
        "greenyellow" => Some("#ADFF2F"),
        "honeydew" => Some("#F0FFF0"),
        "hotpink" => Some("#FF69B4"),
        "indianred" => Some("#CD5C5C"),
        "indigo" => Some("#4B0082"),
        "ivory" => Some("#FFFFF0"),
        "khaki" => Some("#F0E68C"),
        "lavender" => Some("#E6E6FA"),
        "lavenderblush" => Some("#FFF0F5"),
        "lawngreen" => Some("#7CFC00"),
        "lemonchiffon" => Some("#FFFACD"),
        "lightblue" => Some("#ADD8E6"),
        "lightcoral" => Some("#F08080"),
        "lightcyan" => Some("#E0FFFF"),
        "lightgoldenrodyellow" => Some("#FAFAD2"),
        "lightgray" | "lightgrey" => Some("#D3D3D3"),
        "lightgreen" => Some("#90EE90"),
        "lightpink" => Some("#FFB6C1"),
        "lightsalmon" => Some("#FFA07A"),
        "lightseagreen" => Some("#20B2AA"),
        "lightskyblue" => Some("#87CEFA"),
        "lightslategray" | "lightslategrey" => Some("#778899"),
        "lightsteelblue" => Some("#B0C4DE"),
        "lightyellow" => Some("#FFFFE0"),
        "lime" => Some("#00FF00"),
        "limegreen" => Some("#32CD32"),
        "linen" => Some("#FAF0E6"),
        "maroon" => Some("#800000"),
        "mediumaquamarine" => Some("#66CDAA"),
        "mediumblue" => Some("#0000CD"),
        "mediumorchid" => Some("#BA55D3"),
        "mediumpurple" => Some("#9370DB"),
        "mediumseagreen" => Some("#3CB371"),
        "mediumslateblue" => Some("#7B68EE"),
        "mediumspringgreen" => Some("#00FA9A"),
        "mediumturquoise" => Some("#48D1CC"),
        "mediumvioletred" => Some("#C71585"),
        "midnightblue" => Some("#191970"),
        "mintcream" => Some("#F5FFFA"),
        "mistyrose" => Some("#FFE4E1"),
        "moccasin" => Some("#FFE4B5"),
        "navajowhite" => Some("#FFDEAD"),
        "navy" => Some("#000080"),
        "oldlace" => Some("#FDF5E6"),
        "olive" => Some("#808000"),
        "olivedrab" => Some("#6B8E23"),
        "orange" => Some("#FFA500"),
        "orangered" => Some("#FF4500"),
        "orchid" => Some("#DA70D6"),
        "palegoldenrod" => Some("#EEE8AA"),
        "palegreen" => Some("#98FB98"),
        "paleturquoise" => Some("#AFEEEE"),
        "palevioletred" => Some("#DB7093"),
        "papayawhip" => Some("#FFEFD5"),
        "peachpuff" => Some("#FFDAB9"),
        "peru" => Some("#CD853F"),
        "pink" => Some("#FFC0CB"),
        "plum" => Some("#DDA0DD"),
        "powderblue" => Some("#B0E0E6"),
        "purple" => Some("#800080"),
        "rebeccapurple" => Some("#663399"),
        "red" => Some("#FF0000"),
        "rosybrown" => Some("#BC8F8F"),
        "royalblue" => Some("#4169E1"),
        "saddlebrown" => Some("#8B4513"),
        "salmon" => Some("#FA8072"),
        "sandybrown" => Some("#F4A460"),
        "seagreen" => Some("#2E8B57"),
        "seashell" => Some("#FFF5EE"),
        "sienna" => Some("#A0522D"),
        "silver" => Some("#C0C0C0"),
        "skyblue" => Some("#87CEEB"),
        "slateblue" => Some("#6A5ACD"),
        "slategray" | "slategrey" => Some("#708090"),
        "snow" => Some("#FFFAFA"),
        "springgreen" => Some("#00FF7F"),
        "steelblue" => Some("#4682B4"),
        "tan" => Some("#D2B48C"),
        "teal" => Some("#008080"),
        "thistle" => Some("#D8BFD8"),
        "tomato" => Some("#FF6347"),
        "turquoise" => Some("#40E0D0"),
        "violet" => Some("#EE82EE"),
        "wheat" => Some("#F5DEB3"),
        "white" => Some("#FFFFFF"),
        "whitesmoke" => Some("#F5F5F5"),
        "yellow" => Some("#FFFF00"),
        "yellowgreen" => Some("#9ACD32"),
        _ => None,
    }
}

fn write_text_element(
    buf: &mut String,
    content: &str,
    base: &TextBase<'_>,
    style: &Style,
    x: f64,
    width: f64,
    line_bottom_drop: f64,
) {
    let bold = base.bold || style.bold;
    let italic = base.italic || style.italic;
    // `<font:Courier>` sets both `style.font_family` AND `style.monospace`
    // (so widths use monospace metrics and spaces become NBSP), but the
    // emitted `font-family` attribute must carry the user-supplied name —
    // not the literal string "monospace".
    let font_family = if let Some(f) = style.font_family.as_deref() {
        f
    } else if style.monospace {
        "monospace"
    } else {
        base.font_family
    };
    let nominal_size = style.size.unwrap_or(base.font_size);
    // Sub/sup: render with a smaller font and a y offset, matching Java
    // PlantUML's `FontPosition.mute(font)` (size -= 3, min 2) plus a
    // baseline shift of `getSpace()` adjusted by the descent difference
    // between the original and reduced sizes (the smaller glyph's baseline
    // sits descent_diff above the line bottom, so the y attribute must
    // compensate). PlantUML does NOT emit `baseline-shift`; it emits a
    // plain <text> with a smaller font-size at a shifted y.
    // Mixed-size lines bottom-align their runs to a shared line bottom. The
    // first run's baseline is `base.y`; every run's baseline is offset by
    // `clampDrop(firstSize) - clampDrop(ownSize)` so all runs share one line
    // bottom (see `emit_segments`). `clampDrop = max(textHeight,10) - ascent`
    // reduces to the plain descent for size ≳ 9, so the offset is zero when
    // this run matches the first run's metrics (always true on a single-size
    // line).
    let own_drop = clamp_drop(nominal_size as f64, style_metric_family(style, base));
    let line_descent_diff = line_bottom_drop - own_drop;
    let (font_size, y_offset) = match style.baseline_shift {
        Some("sub") => {
            let small = (nominal_size as i32 - 3).max(2) as u32;
            let descent_diff = pm::descent(nominal_size as f64) - pm::descent(small as f64);
            (small, 3.0 + descent_diff + line_descent_diff)
        }
        Some("super") => {
            let small = (nominal_size as i32 - 3).max(2) as u32;
            let descent_diff = pm::descent(nominal_size as f64) - pm::descent(small as f64);
            (small, -6.0 + descent_diff + line_descent_diff)
        }
        _ => (nominal_size, line_descent_diff),
    };
    let raw_fill = style.fill.as_deref().unwrap_or(base.fill);
    let fill = normalize_color(raw_fill);

    // `<back:color>` segments carry an SVG `<filter>` reference. The id is
    // allocated lazily in the active `FilterRegistry`; the filter element
    // itself is emitted into `<defs>` at the end of the render pass.
    let filter_attr = style
        .background
        .as_deref()
        .and_then(filter_registry::id_for_current)
        .map(|id| format!(r#" filter="url(#{id})""#))
        .unwrap_or_default();

    let mut decorations: Vec<&str> = Vec::new();
    if style.underline || base.underline {
        decorations.push("underline");
    }
    if style.line_through {
        decorations.push("line-through");
    }
    if style.wavy_underline {
        decorations.push("wavy underline");
    }
    let text_decoration = if decorations.is_empty() {
        String::new()
    } else {
        format!(r#" text-decoration="{}""#, decorations.join(" "))
    };

    let weight_attr = if bold { r#" font-weight="700""# } else { "" };
    let style_attr = if italic {
        r#" font-style="italic""#
    } else {
        ""
    };

    // Build a deterministic attribute order matching PlantUML's golden
    // output: fill, font-family, font-size, font-style, font-weight,
    // lengthAdjust, text-decoration, textLength, x, y.
    //
    // When the segment carries a link URL, PlantUML wraps the <text> in an
    // <a> element with both modern (href) and legacy (xlink:*) attributes.
    if let Some(url) = style.link_url.as_deref() {
        let escaped = escape_xml_attr(url);
        // `title` / `xlink:title` is the `{tooltip}` when supplied, else the URL.
        let title = style
            .link_title
            .as_deref()
            .map(escape_xml_attr)
            .unwrap_or_else(|| escaped.clone());
        write!(
            buf,
            r#"<a href="{escaped}" target="_top" title="{title}" xlink:actuate="onRequest" xlink:href="{escaped}" xlink:show="new" xlink:title="{title}" xlink:type="simple">"#,
        )
        .unwrap();
    }
    // PlantUML rounds each run's baseline from the unrounded line baseline,
    // but the oracle hands us `base.y` already rounded to 4 decimals. Adding
    // the exact descent offset to that rounded value double-rounds. The true
    // baseline is a sum of dyadic ascent/descent terms — a multiple of 1/256
    // for the font sizes in the corpus — so snap back to that grid before
    // applying a non-zero line offset to reproduce PlantUML's single
    // rounding. Single-size lines have a zero offset and are left untouched.
    let effective_y = if y_offset != 0.0 {
        (base.y * 256.0).round() / 256.0 + y_offset
    } else {
        base.y + y_offset
    };
    // PlantUML emits the no-break space (U+00A0, used for inter-run gaps and
    // monospace padding) as the XML entity `&#160;`, never as the raw byte.
    let content = content.replace('\u{00a0}', "&#160;");
    write!(
        buf,
        r#"<text fill="{fill}"{filter_attr} font-family="{font_family}" font-size="{font_size}"{style_attr}{weight_attr} lengthAdjust="spacing"{text_decoration} textLength="{tl}" x="{x_s}" y="{y_s}">{content}</text>"#,
        tl = pm::fmt_coord(width),
        x_s = pm::fmt_coord(x),
        y_s = pm::fmt_coord(effective_y),
    )
    .unwrap();
    if style.link_url.is_some() {
        buf.push_str("</a>");
    }
}

/// Escape characters that have special meaning in an XML attribute value.
fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(x: f64, y: f64) -> TextBase<'static> {
        TextBase {
            x,
            y,
            font_size: 12,
            font_family: "sans-serif",
            fill: "#000000",
            bold: false,
            italic: false,
            underline: false,
            skip_underline: false,
        }
    }

    #[test]
    fn plain_text_emits_one_element() {
        let mut buf = String::new();
        emit_text(&mut buf, "hello", &base(10.0, 20.0));
        assert!(
            buf.starts_with(r##"<text fill="#000000" font-family="sans-serif" font-size="12""##)
        );
        assert!(buf.contains(">hello</text>"));
        assert_eq!(buf.matches("<text").count(), 1);
    }

    #[test]
    fn uniform_bold_lifts_to_text_element() {
        let mut buf = String::new();
        emit_text(&mut buf, "**bold**", &base(0.0, 0.0));
        assert_eq!(buf.matches("<text").count(), 1);
        assert!(buf.contains(r#"font-weight="700""#));
        assert!(buf.contains(">bold</text>"));
        assert!(!buf.contains("**"));
    }

    #[test]
    fn emitted_baseline_count_tracks_mixed_font_sizes() {
        assert_eq!(emitted_baseline_count("**bold**()", &base(0.0, 0.0)), 1);
        assert_eq!(
            emitted_baseline_count("field: <size:20>large</size>", &base(0.0, 0.0)),
            2
        );
    }

    #[test]
    fn courier_base_uses_monospace_metrics() {
        let mut b = base(10.0, 20.0);
        b.font_family = "Courier";
        let mut buf = String::new();
        let width = emit_text(&mut buf, "Alice", &b);
        assert_eq!(
            pm::fmt_coord(width),
            pm::fmt_coord(pm::mono_text_width("Alice", 12.0))
        );
        assert!(buf.contains(r#"font-family="Courier""#));
        assert!(buf.contains(&format!(
            r#"textLength="{}""#,
            pm::fmt_coord(pm::mono_text_width("Alice", 12.0))
        )));
    }

    #[test]
    fn explicit_font_monospace_inherits_surrounding_metrics() {
        let content = "<font:monospace>code here</font>";
        assert_eq!(
            pm::fmt_coord(measure(content, 12.0, false)),
            pm::fmt_coord(pm::text_width("code\u{00a0}here", 12.0, false))
        );
        assert_eq!(
            pm::fmt_coord(label_height(content, 12.0)),
            pm::fmt_coord(pm::text_height(12.0))
        );
        assert_eq!(
            pm::fmt_coord(label_ascent(content, 12.0)),
            pm::fmt_coord(pm::ascent(12.0))
        );

        let mut buf = String::new();
        let width = emit_text(&mut buf, content, &base(26.0, 76.6016));
        assert_eq!(pm::fmt_coord(width), "57.2813");
        assert!(buf.contains(r#"font-family="monospace""#));
        assert!(buf.contains(r#"textLength="57.2813""#));
        assert!(buf.contains(">code&#160;here</text>"));
    }

    #[test]
    fn quoted_monospace_still_uses_monospace_metrics() {
        assert_eq!(
            pm::fmt_coord(measure(r#"""code here"""#, 12.0, false)),
            pm::fmt_coord(pm::mono_text_width("code\u{00a0}here", 12.0))
        );
    }

    #[test]
    fn awt_named_font_metrics_match_goldens() {
        assert_eq!(
            pm::fmt_coord(measure_with_family("Alice", 16.0, false, "Arial")),
            "34.6797"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family("Bob", 16.0, false, "Arial")),
            "28.4688"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family("Alice", 14.0, false, "Verdana")),
            "32.8877"
        );
        assert_eq!(
            pm::fmt_coord(text_height_for_family(16.0, "Arial")),
            "18.3984"
        );
        assert_eq!(pm::fmt_coord(ascent_for_family(16.0, "Arial")), "15.0078");
    }

    #[test]
    fn mixed_styles_split_into_multiple_elements() {
        let mut buf = String::new();
        emit_text(
            &mut buf,
            "<color:blue>**field**</color>: String",
            &base(0.0, 0.0),
        );
        assert_eq!(buf.matches("<text").count(), 2);
        // First element: blue + bold + "field"
        assert!(buf.contains(r##"fill="#0000FF""##));
        assert!(buf.contains(r#"font-weight="700""#));
        assert!(buf.contains(">field</text>"));
        // Second element: plain + ": String"
        assert!(buf.contains(">: String</text>"));
    }

    #[test]
    fn no_tspans_anywhere() {
        // PlantUML's golden corpus never contains <tspan>. Our output must
        // not either, regardless of how complex the input is.
        let inputs = [
            "**bold**",
            "//italic//",
            "__under__",
            "--strike--",
            r#"""mono"""#,
            "<color:red>**bold red**</color>",
            "**//bold italic//**",
            "before **bold** after",
        ];
        for input in inputs {
            let mut buf = String::new();
            emit_text(&mut buf, input, &base(0.0, 0.0));
            assert!(
                !buf.contains("<tspan"),
                "tspan in output for {input:?}: {buf}"
            );
        }
    }
}
