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

use std::borrow::Cow;
use std::fmt::Write;

use crate::creole::{self, Segment, Style};
use crate::filter_registry;
use crate::font_chain::{ChainFace, ChainFamily, ChainGlyph};
use crate::plantuml_metrics as pm;
use crate::svg::normalize_svg_link_title;
use rustuml_parser::java_utf16;

const TAB_STOP_SPACES: usize = 8;

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

/// Project an actual text line, including a present empty line. Absent
/// displays are handled by callers before asking to measure or paint a line;
/// lexical `creole::parse_segments` deliberately keeps its zero-atom result.
fn line_segments(content: &str, skip_underline: bool) -> Vec<Segment> {
    let mut segments = if skip_underline {
        creole::parse_segments_no_underline(content)
    } else {
        creole::parse_segments(content)
    };
    if segments.is_empty() {
        // StripeSimple.getAtoms inserts a space after recursive commands have
        // restored the surrounding configuration, also for an ordinary blank.
        segments.push(Segment {
            text: " ".to_string(),
            style: Style::default(),
        });
    }
    segments
}

/// Emit one or more `<text>` elements covering `content` with creole markup
/// resolved. Writes to `buf`. Returns the total advance width.
pub fn emit_text(buf: &mut String, content: &str, base: &TextBase<'_>) -> f64 {
    let content = normalize_tab_escapes(content);
    let segments = line_segments(&content, base.skip_underline);
    emit_segments(buf, &segments, base)
}

/// Like [`emit_text`] but neutralises monospace styling: `""..."" ` and
/// backtick runs render as plain (sans-serif) text. PlantUML's edge/link
/// labels parse creole markup (bold, italic, size, colour) but, unlike
/// entity bodies, do *not* honour the monospace delimiter — the `""` glue
/// is consumed and the inner text falls back to the surrounding font.
pub fn emit_text_no_mono(buf: &mut String, content: &str, base: &TextBase<'_>) -> f64 {
    let content = normalize_tab_escapes(content);
    let mut segments = line_segments(&content, base.skip_underline);
    for seg in &mut segments {
        seg.style.monospace = false;
    }
    emit_segments(buf, &segments, base)
}

/// Number of distinct baseline y-values [`emit_text`] will produce for one
/// logical line. Uniform-style Creole emits several `<text>` elements but only
/// one baseline; mixed font sizes/families can consume multiple oracle y slots.
pub fn emitted_baseline_count(content: &str, base: &TextBase<'_>) -> usize {
    let content = normalize_tab_escapes(content);
    let segments = line_segments(&content, base.skip_underline);
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

/// Number of `<text>` elements [`emit_text`] will write for one logical line.
///
/// This differs from [`emitted_baseline_count`]: a link followed by plain text
/// emits two adjacent `<text>` elements on the same baseline. Callers consuming
/// oracle x-anchors need this element count to know whether a single anchor can
/// safely override the line start.
pub fn emitted_text_element_count(content: &str, base: &TextBase<'_>) -> usize {
    let content = normalize_tab_escapes(content);
    let segments = line_segments(&content, base.skip_underline);
    if segments.is_empty() {
        return 1;
    }
    let mut count = 0usize;
    for seg in &segments {
        if seg.text.contains('\t') {
            count += seg.text.split('\t').filter(|part| !part.is_empty()).count();
        } else {
            count += 1;
        }
    }
    count.max(1)
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
    measure_with_family_and_face(content, font_size, bold, false, font_family)
}

/// Width of `content` using a caller-selected base font family and complete
/// face. Java PlantUML measures the same resolved AWT face that it emits, so
/// callers with an italic skinparam must not silently measure a plain face.
pub fn measure_with_family_and_face(
    content: &str,
    font_size: f64,
    bold: bool,
    italic: bool,
    font_family: &str,
) -> f64 {
    measure_inner_with_family(content, font_size, bold, italic, false, font_family)
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
    measure_no_underline_with_family_and_face(content, font_size, bold, false, font_family)
}

/// [`measure_no_underline_with_family`] in the complete face the label is
/// drawn in (Java measures the TextBlock it draws).
pub fn measure_no_underline_with_family_and_face(
    content: &str,
    font_size: f64,
    bold: bool,
    italic: bool,
    font_family: &str,
) -> f64 {
    measure_inner_with_family(content, font_size, bold, italic, true, font_family)
}

fn measure_inner(content: &str, font_size: f64, bold: bool, skip_underline: bool) -> f64 {
    measure_inner_with_family(
        content,
        font_size,
        bold,
        false,
        skip_underline,
        "sans-serif",
    )
}

fn measure_inner_with_family(
    content: &str,
    font_size: f64,
    bold: bool,
    italic: bool,
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
        italic,
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
    let segments = line_segments(content, false);
    if segments.is_empty() {
        return family_text_height(font_size, metric_family(font_family));
    }
    let (min_y, max_y) = segments
        .iter()
        .map(|segment| {
            let size = positioned_segment_size(segment, font_size);
            let height =
                family_text_height(size, segment_metric_family_for_family(segment, font_family))
                    .max(10.0);
            let altitude = segment_starting_altitude(segment);
            (-height + altitude, altitude)
        })
        .fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(min_y, max_y), (top, bottom)| (min_y.min(top), max_y.max(bottom)),
        );
    max_y - min_y
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
    let segments = line_segments(content, false);
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

/// Distance from the line top to the `TextBase.y` anchor needed to emit the
/// first `<text>` element at PlantUML's aligned baseline.
///
/// [`emit_text`] adds `FontPosition.getSpace()` and the reduced-font descent
/// correction to positioned runs. Java's `Sea.doAlign` and
/// `Sea.translateMinYto`, however, align the atom before `AtomText.drawU`
/// emits its baseline. Subtracting the first run's emission offset here keeps
/// that offset relative to the aligned atom instead of applying it twice.
/// When the first run is shorter than a later run (for example `""mono"" text`
/// in a sans-serif box), callers that place a whole label inside a known line
/// box need this value instead of [`label_ascent_with_family`].
pub fn label_first_baseline_ascent_with_family(
    content: &str,
    font_size: f64,
    font_family: &str,
) -> f64 {
    let segments = line_segments(content, false);
    if segments.is_empty() {
        return family_ascent(font_size, metric_family(font_family));
    }

    // Java `Sea.doAlign` places each atom at
    // `-atomHeight + AtomText.getStartingAltitude()`, where sub/sup altitudes
    // are +3/-6 (`FontPosition.getSpace`). `translateMinYto(0)` then pins the
    // shared line top. Derive the first atom's baseline from those positions
    // rather than treating sub/sup space as a blanket line offset.
    let min_y = segments
        .iter()
        .map(|segment| {
            let size = positioned_segment_size(segment, font_size);
            let height =
                family_text_height(size, segment_metric_family_for_family(segment, font_family))
                    .max(10.0);
            -height + segment_starting_altitude(segment)
        })
        .fold(f64::INFINITY, f64::min);
    let first = &segments[0];
    let first_size = positioned_segment_size(first, font_size);
    let first_family = segment_metric_family_for_family(first, font_family);
    let first_height = family_text_height(first_size, first_family).max(10.0);
    let first_top = -first_height + segment_starting_altitude(first);
    let emitted_baseline = first_top - min_y + family_ascent(first_size, first_family);
    emitted_baseline - positioned_segment_emission_offset(first, font_size)
}

/// Distance from a Creole line's top to the lowest point reported by Java's
/// `LimitFinder.drawText`.
///
/// `LimitFinder` records every `UText` run through `baseline + 1.5`, so this
/// follows the same mixed-family, mixed-size, and sub/sup baseline offsets as
/// [`emit_text`] and returns the largest painted bound.
pub(crate) fn label_limit_finder_height_with_family(
    content: &str,
    font_size: f64,
    font_family: &str,
) -> f64 {
    let segments = line_segments(content, false);
    let first_baseline = label_first_baseline_ascent_with_family(content, font_size, font_family);
    let Some(first) = segments.first() else {
        return first_baseline + 1.5;
    };
    let first_baseline = first_baseline + positioned_segment_emission_offset(first, font_size);
    let first_size = first.style.size.map(f64::from).unwrap_or(font_size);
    let line_bottom_drop = clamp_drop(
        first_size,
        segment_metric_family_for_family(first, font_family),
    );
    let max_baseline_offset = segments
        .iter()
        .map(|segment| {
            let nominal_size = segment.style.size.map(f64::from).unwrap_or(font_size);
            let own_drop = clamp_drop(
                nominal_size,
                segment_metric_family_for_family(segment, font_family),
            );
            let line_descent_diff = line_bottom_drop - own_drop;
            match segment.style.baseline_shift {
                Some("sub") => {
                    let small = (nominal_size as i32 - 3).max(2) as f64;
                    3.0 + pm::descent(nominal_size) - pm::descent(small) + line_descent_diff
                }
                Some("super") => {
                    let small = (nominal_size as i32 - 3).max(2) as f64;
                    -6.0 + pm::descent(nominal_size) - pm::descent(small) + line_descent_diff
                }
                _ => line_descent_diff,
            }
        })
        .fold(0.0_f64, f64::max);
    first_baseline + max_baseline_offset + 1.5
}

pub(crate) fn text_height_for_family(font_size: f64, font_family: &str) -> f64 {
    family_text_height(font_size, metric_family(font_family))
}

pub(crate) fn ascent_for_family(font_size: f64, font_family: &str) -> f64 {
    family_ascent(font_size, metric_family(font_family))
}

fn positioned_segment_size(segment: &Segment, base_size: f64) -> f64 {
    let nominal = segment.style.size.map(f64::from).unwrap_or(base_size);
    if segment.style.baseline_shift.is_some() {
        (nominal - 3.0).max(2.0)
    } else {
        nominal
    }
}

fn segment_starting_altitude(segment: &Segment) -> f64 {
    match segment.style.baseline_shift {
        Some("sub") => 3.0,
        Some("super") => -6.0,
        _ => 0.0,
    }
}

fn positioned_segment_emission_offset(segment: &Segment, base_size: f64) -> f64 {
    let nominal = segment.style.size.map(f64::from).unwrap_or(base_size);
    let reduced = (nominal - 3.0).max(2.0);
    let descent_correction = pm::descent(nominal) - pm::descent(reduced);
    match segment.style.baseline_shift {
        Some("sub") => 3.0 + descent_correction,
        Some("super") => -6.0 + descent_correction,
        _ => 0.0,
    }
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
    let content = normalize_tab_escapes(content);
    let segments = line_segments(&content, base.skip_underline);
    let mut advance = 0.0;
    for seg in &segments {
        advance = segment_advance(seg, base, advance);
    }
    advance
}

/// Width of a run of atoms that is only part of one stripe, such as the
/// spacer atoms before a label's first ink atom.
///
/// Java provenance: `StripeSimple.getAtoms` adds its single `" "` atom only
/// when a whole stripe has no atoms. A prefix of a stripe is not a stripe, so
/// an empty run is zero atoms and has zero width. Whole lines must keep using
/// [`measure_with_family`].
pub fn measure_atom_run_with_family(
    content: &str,
    font_size: f64,
    bold: bool,
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
        skip_underline: false,
    };
    let content = normalize_tab_escapes(content);
    let mut advance = 0.0;
    for seg in &creole::parse_segments(&content) {
        advance = segment_advance(seg, &base, advance);
    }
    advance
}

fn normalize_tab_escapes(s: &str) -> Cow<'_, str> {
    if !s.contains("\\t") {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'t') {
            chars.next();
            out.push('\t');
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
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

    let mut advance = 0.0;
    for seg in segments {
        if seg.text.contains('\t') {
            advance = emit_tabbed_segment(buf, seg, base, advance, line_bottom_drop);
        } else {
            let full_w = segment_width(seg, base);
            let (lead_w, trimmed_text, trimmed_w) = trim_segment_for_emit(seg, base);
            write_text_element(
                buf,
                &trimmed_text,
                base,
                &seg.style,
                base.x + advance + lead_w,
                trimmed_w,
                line_bottom_drop,
            );
            advance += full_w;
        }
    }
    advance
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
    trim_text_for_emit(&seg.text, &seg.style, base)
}

fn trim_text_for_emit(text: &str, style: &Style, base: &TextBase<'_>) -> (f64, String, f64) {
    // Monospace segments use NBSP instead of ASCII space; PlantUML does
    // not trim NBSP, so pass through unchanged.
    if style.monospace {
        let w = text_width_for_style(text, style, base);
        return (0.0, text.to_string(), w);
    }
    // Count leading and trailing ASCII spaces in the *escaped* form. The
    // escape form only differs for non-space characters, so a leading
    // space stays a leading space.
    let bytes = text.as_bytes();
    let mut lead = 0;
    while lead < bytes.len() && bytes[lead] == b' ' {
        lead += 1;
    }
    // Whole-segment whitespace: PlantUML emits these as literal-space
    // `<text>` elements but converts each ASCII space to NBSP (U+00A0)
    // in the rendered text. Width still counts as a normal ASCII space.
    if lead == bytes.len() {
        let w = text_width_for_style(text, style, base);
        let nbsp_text = "\u{00a0}".repeat(lead);
        return (0.0, nbsp_text, w);
    }
    let mut trail = 0;
    while trail < bytes.len() - lead && bytes[bytes.len() - 1 - trail] == b' ' {
        trail += 1;
    }
    if lead == 0 && trail == 0 {
        let w = text_width_for_style(text, style, base);
        return (0.0, text.to_string(), w);
    }
    let trimmed = &text[lead..text.len() - trail];
    let font_size = effective_font_size_for_style(style, base);
    let (family, bold, italic) = style_face(style, base);
    let lead_w = family_text_width(&" ".repeat(lead), font_size, bold, italic, family);
    let trimmed_w = family_text_width(
        &unescape_for_metrics(trimmed),
        font_size,
        bold,
        italic,
        family,
    );
    (lead_w, trimmed.to_string(), trimmed_w)
}

/// Width of one segment under the effective styling. Routes to the
/// monospace metric path when the segment is monospaced; otherwise uses
/// sans-serif (with linear scaling for non-tabulated font sizes).
fn segment_width(seg: &Segment, base: &TextBase<'_>) -> f64 {
    text_width_for_style(&seg.text, &seg.style, base)
}

fn text_width_for_style(text: &str, style: &Style, base: &TextBase<'_>) -> f64 {
    let raw = unescape_for_metrics(text);
    let font_size = effective_font_size_for_style(style, base);
    let (family, bold, italic) = style_face(style, base);
    family_text_width(&raw, font_size, bold, italic, family)
}

fn segment_advance(seg: &Segment, base: &TextBase<'_>, start: f64) -> f64 {
    if seg.text.contains('\t') {
        text_advance_with_tabs(&seg.text, &seg.style, base, start)
    } else {
        start + segment_width(seg, base)
    }
}

fn text_advance_with_tabs(text: &str, style: &Style, base: &TextBase<'_>, start: f64) -> f64 {
    let mut advance = start;
    let mut rest = text;
    let tab_width = tab_stop_width(style, base);
    while let Some(tab_idx) = rest.find('\t') {
        let part = &rest[..tab_idx];
        advance += text_width_for_style(part, style, base);
        advance = next_tab_advance(advance, tab_width);
        rest = &rest[tab_idx + '\t'.len_utf8()..];
    }
    advance + text_width_for_style(rest, style, base)
}

fn emit_tabbed_segment(
    buf: &mut String,
    seg: &Segment,
    base: &TextBase<'_>,
    start: f64,
    line_bottom_drop: f64,
) -> f64 {
    let mut advance = start;
    let mut rest = seg.text.as_str();
    let tab_width = tab_stop_width(&seg.style, base);
    loop {
        let (part, has_tab, next_rest) = match rest.find('\t') {
            Some(tab_idx) => (&rest[..tab_idx], true, &rest[tab_idx + '\t'.len_utf8()..]),
            None => (rest, false, ""),
        };

        let part_w = text_width_for_style(part, &seg.style, base);
        if !part.is_empty() {
            let (lead_w, trimmed_text, trimmed_w) = trim_text_for_emit(part, &seg.style, base);
            if !trimmed_text.is_empty() || trimmed_w > 0.0 {
                write_text_element(
                    buf,
                    &trimmed_text,
                    base,
                    &seg.style,
                    base.x + advance + lead_w,
                    trimmed_w,
                    line_bottom_drop,
                );
            }
        }
        advance += part_w;
        if !has_tab {
            return advance;
        }
        advance = next_tab_advance(advance, tab_width);
        rest = next_rest;
    }
}

fn tab_stop_width(style: &Style, base: &TextBase<'_>) -> f64 {
    let font_size = effective_font_size_for_style(style, base);
    let (family, bold, italic) = style_face(style, base);
    family_text_width(
        &" ".repeat(TAB_STOP_SPACES),
        font_size,
        bold,
        italic,
        family,
    )
}

fn next_tab_advance(advance: f64, tab_width: f64) -> f64 {
    if tab_width <= 0.0 {
        return advance;
    }
    ((advance / tab_width) + 1e-9)
        .floor()
        .mul_add(tab_width, tab_width)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MetricFamily {
    Sans,
    Mono,
    CourierNew,
    Arial,
    Helvetica,
    Verdana,
    TimesNewRoman,
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
    style_face(style, base).0
}

/// Family and face bits for a width measurement. Creole monospace (except the
/// literal family names `monospace` / `monospaced`, which keep the surrounding
/// face) is the Menlo cell. Every other name goes through
/// [`resolve_modelled_face`].
fn style_face(style: &Style, base: &TextBase<'_>) -> (MetricFamily, bool, bool) {
    let bold = base.bold || style.bold;
    let italic = base.italic || style.italic;
    if style.monospace && !explicit_monospace_font_uses_surrounding_metrics(style) {
        return (MetricFamily::Mono, bold, italic);
    }
    // `<font:monospace>` and `<font:monospaced>` keep the surrounding face.
    // The literal name is the SVG spelling, and resolving it would select
    // Menlo. A caller that passes "monospace" as the base family (the logical
    // font) still goes through [`resolve_modelled_face`].
    let name = if explicit_monospace_font_uses_surrounding_metrics(style) {
        base.font_family
    } else {
        style.font_family.as_deref().unwrap_or(base.font_family)
    };
    resolve_modelled_face(name, bold, italic)
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
    resolve_modelled_face(font_family, false, false).0
}

/// The name `Font.decode` leaves after stripping a trailing style token or
/// point size. `FontStack.getFonts` decodes the skinparam string, then
/// `getFont` calls `deriveFont(PLAIN)` and `UFontFace.deriveFont`, so the
/// decoded style is not the face that is measured. Indices follow Java
/// `String` (UTF-16). Every name the JDK registers for a modelled face is
/// BMP, so a scalar index is a code unit.
fn decoded_font_name(name: &str) -> &str {
    let chars: Vec<(usize, char)> = name.char_indices().collect();
    let len = chars.len();
    if len == 0 {
        return name;
    }
    let last = |sep: char| -> isize {
        chars
            .iter()
            .rposition(|(_, c)| *c == sep)
            .map(|i| i as isize)
            .unwrap_or(-1)
    };
    let last_from = |sep: char, from: isize| -> isize {
        if from < 0 {
            return -1;
        }
        chars[..=from as usize]
            .iter()
            .rposition(|(_, c)| *c == sep)
            .map(|i| i as isize)
            .unwrap_or(-1)
    };
    let last_hyphen = last('-');
    let last_space = last(' ');
    let sep = if last_hyphen > last_space { '-' } else { ' ' };
    let mut size_index = last(sep);
    let mut style_index = last_from(sep, size_index - 1);
    if size_index > 0 && (size_index as usize) + 1 < len {
        let tail = &name[chars[(size_index as usize) + 1].0..];
        if tail.parse::<i32>().is_ok() {
            // A parsed size is consumed even when it is not positive
            // (`Font.decode` then substitutes 12).
        } else {
            style_index = size_index;
            size_index = len as isize;
            if chars[(size_index as usize) - 1].1 == sep {
                size_index -= 1;
            }
        }
    }
    let byte_at = |index: isize| -> usize {
        if index <= 0 {
            0
        } else if index as usize >= len {
            name.len()
        } else {
            chars[index as usize].0
        }
    };
    if style_index >= 0 && (style_index as usize) + 1 < len {
        let style_name = name[byte_at(style_index + 1)..byte_at(size_index)].to_ascii_lowercase();
        let style_end = if matches!(
            style_name.as_str(),
            "bolditalic" | "italic" | "bold" | "plain"
        ) {
            style_index
        } else {
            let mut end = size_index;
            if end > 0 && chars[(end as usize) - 1].1 == sep {
                end -= 1;
            }
            end
        };
        return &name[..byte_at(style_end)];
    }
    let mut font_end = len as isize;
    if style_index > 0 {
        font_end = style_index;
    } else if size_index > 0 {
        font_end = size_index;
    }
    if font_end > 0 && chars[(font_end as usize) - 1].1 == sep {
        font_end -= 1;
    }
    &name[..byte_at(font_end)]
}

/// Family and face JDK 21 measures for `name` on macOS, for the families
/// Rust models.
///
/// `requested_bold` / `requested_italic` are the skinparam and creole face
/// (`UFontFace`), not the style `Font.decode` parsed out of the name.
/// Family lookup keeps that requested face (`FontFamily.getFont`). A
/// PostScript name keeps its own face when the request is plain or already
/// equal, and otherwise uses the OR of the two styles (`findFont2D`).
/// Anything else is Dialog, whose macOS face is Lucida Grande.
fn resolve_modelled_face(
    name: &str,
    requested_bold: bool,
    requested_italic: bool,
) -> (MetricFamily, bool, bool) {
    let trimmed = name.trim().trim_matches(|c| c == '"' || c == '\'');
    let decoded = decoded_font_name(trimmed);
    let lower = decoded.to_lowercase();
    if let Some(family) = modelled_family(&lower) {
        return (family, requested_bold, requested_italic);
    }
    if let Some((family, face_bold, face_italic)) = modelled_postscript_name(&lower) {
        let face = u8::from(face_bold) | (u8::from(face_italic) << 1);
        let requested = u8::from(requested_bold) | (u8::from(requested_italic) << 1);
        // Font.PLAIN is 0, BOLD is 1, ITALIC is 2. A plain request does not
        // unbold a face found by its full name.
        let merged = if face == requested || requested == 0 {
            face
        } else {
            face | requested
        };
        return (family, merged & 1 != 0, merged & 2 != 0);
    }
    (MetricFamily::Sans, requested_bold, requested_italic)
}

/// Families registered by `loadNativeFonts`, plus the logical fonts
/// `CFontManager.setupLogicalFonts` clones onto a modelled face. `courier`
/// and `timesroman` are the JDK 1.0 compatibility names, applied only once
/// no real family of that name exists (the reference census has neither).
/// `sans-serif` and `monospace` are the SVG spellings `UFontImpl` /
/// `SvgGraphics` emit for SansSerif and Monospaced; measurement of those
/// fonts uses the logical face, and Rust's own callers pass the SVG spelling.
fn modelled_family(lower: &str) -> Option<MetricFamily> {
    Some(match lower {
        "sansserif" | "sans-serif" | "dialog" | "dialoginput" | "default" | "lucida grande" => {
            MetricFamily::Sans
        }
        "menlo" | "monospaced" | "monospace" | "courier" => MetricFamily::Mono,
        "courier new" => MetricFamily::CourierNew,
        "arial" => MetricFamily::Arial,
        "helvetica" => MetricFamily::Helvetica,
        "verdana" => MetricFamily::Verdana,
        "times new roman" | "serif" | "timesroman" => MetricFamily::TimesNewRoman,
        _ => return None,
    })
}

/// PostScript names `GetFilteredFonts` registers for the modelled faces
/// (census `tools/font-chain-reference/monkeyman-round6/census.tsv`). The
/// style bits are `Font2D.setStyle` on that full name. Names `Font.decode`
/// reduces to the family (`Menlo-Bold`, `Verdana-Italic`, `Helvetica-Bold`)
/// are not listed: the family arm already measures them.
fn modelled_postscript_name(lower: &str) -> Option<(MetricFamily, bool, bool)> {
    Some(match lower {
        "lucidagrande" => (MetricFamily::Sans, false, false),
        "arialmt" => (MetricFamily::Arial, false, false),
        "arial-boldmt" => (MetricFamily::Arial, true, false),
        "arial-italicmt" => (MetricFamily::Arial, false, true),
        "arial-bolditalicmt" => (MetricFamily::Arial, true, true),
        "helvetica-oblique" => (MetricFamily::Helvetica, false, true),
        "helvetica-boldoblique" => (MetricFamily::Helvetica, true, true),
        "menlo-regular" => (MetricFamily::Mono, false, false),
        "couriernewpsmt" => (MetricFamily::CourierNew, false, false),
        "couriernewps-boldmt" => (MetricFamily::CourierNew, true, false),
        "couriernewps-italicmt" => (MetricFamily::CourierNew, false, true),
        "couriernewps-bolditalicmt" => (MetricFamily::CourierNew, true, true),
        "timesnewromanpsmt" => (MetricFamily::TimesNewRoman, false, false),
        "timesnewromanps-boldmt" => (MetricFamily::TimesNewRoman, true, false),
        "timesnewromanps-italicmt" => (MetricFamily::TimesNewRoman, false, true),
        "timesnewromanps-bolditalicmt" => (MetricFamily::TimesNewRoman, true, true),
        _ => return None,
    })
}

/// Compute the effective rendered font size for a segment. PlantUML's
/// `FontPosition.mute(font)` decreases the size by 3 (minimum 2) for sub /
/// sup positioning, then the smaller font is used for both width
/// measurement and the emitted `font-size` attribute. The reduction is
/// applied AFTER any explicit `<size:N>` override.
fn effective_font_size(seg: &Segment, base: &TextBase<'_>) -> f64 {
    effective_font_size_for_style(&seg.style, base)
}

fn effective_font_size_for_style(style: &Style, base: &TextBase<'_>) -> f64 {
    let nominal = style
        .size
        .map(|s| s as f64)
        .unwrap_or(base.font_size as f64);
    if style.baseline_shift.is_some() {
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

/// Printable-ASCII (U+0020..U+007E) advances, in font units, of the face
/// JDK 21 measures for each style slot of a table-measured family: the
/// face's own glyph advance (CTFontGetAdvancesForGlyphs at 2048 units per em)
/// from `tools/font-chain-reference/ascii-advances-round11`
/// (`tools/AsciiAdvanceExtract.swift`; gate `ascii_tables_are_the_extracted_faces`).
/// Verdana-Italic and Verdana-BoldItalic have advances of their own
/// (CFontManager registers them as the family's italic faces); Arial's
/// italics happen to share the upright advances.
const ASCII_UNITS_PER_EM: f64 = 2048.0;

/// `ArialMT.plain.ascii.tsv`.
const ARIAL_ASCII: [u16; 95] = [
    569, 569, 727, 1139, 1139, 1821, 1366, 391, 682, 682, 797, 1196, 569, 682, 569, 569, 1139,
    1139, 1139, 1139, 1139, 1139, 1139, 1139, 1139, 1139, 569, 569, 1196, 1196, 1196, 1139, 2079,
    1366, 1366, 1479, 1479, 1366, 1251, 1593, 1479, 569, 1024, 1366, 1139, 1706, 1479, 1593, 1366,
    1593, 1479, 1366, 1251, 1479, 1366, 1933, 1366, 1366, 1251, 569, 569, 569, 961, 1139, 682,
    1139, 1139, 1024, 1139, 1139, 569, 1139, 1139, 455, 455, 1024, 455, 1706, 1139, 1139, 1139,
    1139, 682, 1024, 569, 1139, 1024, 1479, 1024, 1024, 1024, 684, 532, 684, 1196,
];

/// `Arial-BoldMT.bold.ascii.tsv`.
const ARIAL_BOLD_ASCII: [u16; 95] = [
    569, 682, 971, 1139, 1139, 1821, 1479, 487, 682, 682, 797, 1196, 569, 682, 569, 569, 1139,
    1139, 1139, 1139, 1139, 1139, 1139, 1139, 1139, 1139, 682, 682, 1196, 1196, 1196, 1251, 1997,
    1479, 1479, 1479, 1479, 1366, 1251, 1593, 1479, 569, 1139, 1479, 1251, 1706, 1479, 1593, 1366,
    1593, 1479, 1366, 1251, 1479, 1366, 1933, 1366, 1366, 1251, 682, 569, 682, 1196, 1139, 682,
    1139, 1251, 1139, 1251, 1139, 682, 1251, 1251, 569, 569, 1139, 569, 1821, 1251, 1251, 1251,
    1251, 797, 1139, 682, 1251, 1139, 1593, 1139, 1139, 1024, 797, 573, 797, 1196,
];

/// `Arial-ItalicMT.italic.ascii.tsv`.
const ARIAL_ITALIC_ASCII: [u16; 95] = [
    569, 569, 727, 1139, 1139, 1821, 1366, 391, 682, 682, 797, 1196, 569, 682, 569, 569, 1139,
    1139, 1139, 1139, 1139, 1139, 1139, 1139, 1139, 1139, 569, 569, 1196, 1196, 1196, 1139, 2079,
    1366, 1366, 1479, 1479, 1366, 1251, 1593, 1479, 569, 1024, 1366, 1139, 1706, 1479, 1593, 1366,
    1593, 1479, 1366, 1251, 1479, 1366, 1933, 1366, 1366, 1251, 569, 569, 569, 961, 1139, 682,
    1139, 1139, 1024, 1139, 1139, 569, 1139, 1139, 455, 455, 1024, 455, 1706, 1139, 1139, 1139,
    1139, 682, 1024, 569, 1139, 1024, 1479, 1024, 1024, 1024, 684, 532, 684, 1196,
];

/// `Arial-BoldItalicMT.bolditalic.ascii.tsv`.
const ARIAL_BOLD_ITALIC_ASCII: [u16; 95] = [
    569, 682, 971, 1139, 1139, 1821, 1479, 487, 682, 682, 797, 1196, 569, 682, 569, 569, 1139,
    1139, 1139, 1139, 1139, 1139, 1139, 1139, 1139, 1139, 682, 682, 1196, 1196, 1196, 1251, 1997,
    1479, 1479, 1479, 1479, 1366, 1251, 1593, 1479, 569, 1139, 1479, 1251, 1706, 1479, 1593, 1366,
    1593, 1479, 1366, 1251, 1479, 1366, 1933, 1366, 1366, 1251, 682, 569, 682, 1196, 1139, 682,
    1139, 1251, 1139, 1251, 1139, 682, 1251, 1251, 569, 569, 1139, 569, 1821, 1251, 1251, 1251,
    1251, 797, 1139, 682, 1251, 1139, 1593, 1139, 1139, 1024, 797, 573, 797, 1196,
];

/// `Verdana.plain.ascii.tsv`.
const VERDANA_ASCII: [u16; 95] = [
    720, 806, 940, 1676, 1302, 2204, 1488, 550, 930, 930, 1302, 1676, 745, 930, 745, 930, 1302,
    1302, 1302, 1302, 1302, 1302, 1302, 1302, 1302, 1302, 930, 930, 1676, 1676, 1676, 1117, 2048,
    1400, 1404, 1430, 1578, 1295, 1177, 1588, 1539, 862, 931, 1419, 1140, 1726, 1532, 1612, 1235,
    1612, 1424, 1400, 1262, 1499, 1400, 2025, 1403, 1260, 1403, 930, 930, 930, 1676, 1302, 1302,
    1230, 1276, 1067, 1276, 1220, 720, 1276, 1296, 562, 705, 1212, 562, 1992, 1296, 1243, 1276,
    1276, 874, 1067, 807, 1296, 1212, 1676, 1212, 1212, 1076, 1300, 930, 1300, 1676,
];

/// `Verdana-Bold.bold.ascii.tsv`.
const VERDANA_BOLD_ASCII: [u16; 95] = [
    700, 824, 1203, 1776, 1456, 2605, 1766, 680, 1113, 1113, 1456, 1776, 740, 983, 740, 1412, 1456,
    1456, 1456, 1456, 1456, 1456, 1456, 1456, 1456, 1456, 824, 824, 1776, 1776, 1776, 1263, 1974,
    1590, 1560, 1482, 1700, 1399, 1332, 1661, 1715, 1118, 1137, 1579, 1305, 1941, 1734, 1741, 1501,
    1741, 1602, 1455, 1396, 1663, 1564, 2311, 1564, 1509, 1417, 1113, 1412, 1113, 1776, 1456, 1456,
    1368, 1432, 1205, 1432, 1360, 865, 1432, 1459, 700, 825, 1374, 700, 2167, 1459, 1406, 1432,
    1432, 1018, 1215, 933, 1459, 1331, 2006, 1370, 1333, 1222, 1456, 1113, 1456, 1776,
];

/// `Verdana-Italic.italic.ascii.tsv`.
const VERDANA_ITALIC_ASCII: [u16; 95] = [
    720, 806, 940, 1676, 1302, 2204, 1488, 550, 930, 930, 1302, 1676, 745, 930, 745, 930, 1302,
    1302, 1302, 1302, 1302, 1302, 1302, 1302, 1302, 1302, 930, 930, 1676, 1676, 1676, 1117, 2048,
    1398, 1404, 1430, 1568, 1295, 1177, 1588, 1539, 862, 931, 1419, 1140, 1726, 1532, 1612, 1235,
    1612, 1424, 1400, 1262, 1499, 1398, 2028, 1403, 1260, 1403, 930, 930, 930, 1676, 1302, 1302,
    1230, 1276, 1067, 1276, 1220, 720, 1273, 1296, 562, 705, 1202, 562, 1993, 1296, 1243, 1276,
    1276, 874, 1067, 807, 1296, 1210, 1676, 1212, 1210, 1076, 1300, 930, 1300, 1676,
];

/// `Verdana-BoldItalic.bolditalic.ascii.tsv`.
const VERDANA_BOLD_ITALIC_ASCII: [u16; 95] = [
    700, 824, 1203, 1776, 1456, 2605, 1766, 680, 1113, 1113, 1456, 1776, 740, 983, 740, 1412, 1456,
    1456, 1456, 1456, 1456, 1456, 1456, 1456, 1456, 1456, 824, 824, 1776, 1776, 1776, 1263, 1974,
    1590, 1560, 1482, 1700, 1399, 1332, 1661, 1715, 1118, 1137, 1579, 1305, 1941, 1734, 1741, 1501,
    1741, 1602, 1455, 1396, 1663, 1564, 2311, 1564, 1509, 1417, 1113, 1412, 1113, 1776, 1456, 1456,
    1368, 1432, 1205, 1432, 1360, 865, 1432, 1459, 700, 825, 1374, 700, 2167, 1459, 1404, 1432,
    1432, 1018, 1215, 933, 1459, 1329, 2006, 1370, 1333, 1222, 1456, 1113, 1456, 1776,
];

/// The four style slots of a table-measured family.
struct AsciiFaces {
    plain: &'static [u16; 95],
    bold: &'static [u16; 95],
    italic: &'static [u16; 95],
    bold_italic: &'static [u16; 95],
}

impl AsciiFaces {
    /// The table of the face measured for (bold, italic), the same slot
    /// `ChainFace::for_style` resolves.
    fn slot(&self, bold: bool, italic: bool) -> &'static [u16; 95] {
        match (bold, italic) {
            (false, false) => self.plain,
            (true, false) => self.bold,
            (false, true) => self.italic,
            (true, true) => self.bold_italic,
        }
    }
}

const ARIAL_FACES: AsciiFaces = AsciiFaces {
    plain: &ARIAL_ASCII,
    bold: &ARIAL_BOLD_ASCII,
    italic: &ARIAL_ITALIC_ASCII,
    bold_italic: &ARIAL_BOLD_ITALIC_ASCII,
};

const VERDANA_FACES: AsciiFaces = AsciiFaces {
    plain: &VERDANA_ASCII,
    bold: &VERDANA_BOLD_ASCII,
    italic: &VERDANA_ITALIC_ASCII,
    bold_italic: &VERDANA_BOLD_ITALIC_ASCII,
};

fn family_table_text_width(
    text: &str,
    font_size: f64,
    bold: bool,
    italic: bool,
    family: ChainFamily,
    faces: &AsciiFaces,
) -> f64 {
    let table = faces.slot(bold, italic);
    chain_text_width(
        text,
        font_size,
        bold,
        ChainFace::for_style(family, bold, italic),
        |c| {
            let code = c as usize;
            (32..=126)
                .contains(&code)
                .then(|| f64::from(table[code - 32]) / ASCII_UNITS_PER_EM * font_size)
        },
    )
}

const HELVETICA_UPM: f64 = 2048.0;

fn helvetica_char_units(c: char) -> Option<f64> {
    let units = match c {
        ' ' => 569.0,
        '!' => 569.0,
        '"' => 727.0,
        '#' => 1139.0,
        '$' => 1139.0,
        '%' => 1820.0,
        '&' => 1366.0,
        '\'' => 455.0,
        '(' => 682.0,
        ')' => 682.0,
        '*' => 797.0,
        '+' => 1196.0,
        ',' => 569.0,
        '-' => 682.0,
        '.' => 569.0,
        '/' => 569.0,
        '0'..='9' => 1139.0,
        ':' => 569.0,
        ';' => 569.0,
        '<' => 1196.0,
        '=' => 1196.0,
        '>' => 1196.0,
        '?' => 1139.0,
        '@' => 2079.0,
        'A' | 'B' => 1366.0,
        'C' | 'D' | 'H' | 'N' | 'R' | 'U' => 1479.0,
        'E' => 1366.0,
        'F' | 'T' | 'Z' => 1251.0,
        'G' | 'O' | 'Q' => 1593.0,
        'I' => 569.0,
        'J' => 1024.0,
        'S' => 1366.0,
        'K' | 'X' => 1366.0,
        'L' => 1139.0,
        'M' => 1706.0,
        'P' | 'V' => 1366.0,
        'W' => 1934.0,
        'Y' => 1366.0,
        '[' => 569.0,
        '\\' => 569.0,
        ']' => 569.0,
        '^' => 961.0,
        '_' => 1139.0,
        '`' => 682.0,
        'a' | 'b' | 'd' | 'e' | 'g' | 'h' | 'n' | 'o' | 'p' | 'q' | 'u' => 1139.0,
        'c' | 'k' | 's' | 'v' | 'x' | 'y' | 'z' => 1024.0,
        'f' | 't' => 569.0,
        'i' | 'j' | 'l' => 455.0,
        'm' => 1706.0,
        'r' => 682.0,
        'w' => 1479.0,
        '{' => 684.0,
        '|' => 532.0,
        '}' => 684.0,
        '~' => 1196.0,
        '«' | '»' => 1139.0,
        _ => return None,
    };
    Some(units)
}

fn helvetica_bold_char_units(c: char) -> Option<f64> {
    let units = match c {
        ' ' | ',' | '.' => 569.0,
        '!' | '(' | ')' | '-' | '[' | ']' | '`' => 682.0,
        '"' => 971.0,
        '#' | '$' | '0'..='9' | 'J' | 'a' | 'c' | 'e' | 's' | 'v' | 'x' | 'y' | '«' | '»' => {
            1139.0
        }
        '%' => 1821.0,
        '&' | 'A' | 'B' | 'C' | 'D' | 'H' | 'K' | 'N' | 'R' | 'U' => 1479.0,
        '\'' => 487.0,
        '*' | 'r' | '{' | '}' => 797.0,
        '+' | '<' | '=' | '>' | '^' | '~' => 1196.0,
        '/' | 'I' | '\\' | 'i' | 'j' | 'l' => 569.0,
        ':' | ';' => 682.0,
        '?' | 'F' | 'L' | 'T' | 'Z' | 'z' => 1251.0,
        '@' => 1997.0,
        'E' | 'P' | 'S' | 'V' | 'X' | 'Y' => 1366.0,
        'G' | 'O' | 'Q' | 'w' => 1593.0,
        'M' => 1706.0,
        'W' => 1933.0,
        '_' => 1139.0,
        'b' | 'd' | 'g' | 'h' | 'k' | 'n' | 'o' | 'p' | 'q' | 'u' => 1251.0,
        'f' | 't' => 682.0,
        'm' => 1821.0,
        '|' => 573.0,
        _ => return None,
    };
    Some(units)
}

fn helvetica_text_width(text: &str, font_size: f64, bold: bool, italic: bool) -> f64 {
    let face = ChainFace::for_style(ChainFamily::Helvetica, bold, italic);
    chain_text_width(text, font_size, bold, face, |c| {
        if bold {
            helvetica_bold_char_units(c)
        } else {
            helvetica_char_units(c)
        }
        .map(|units| units / HELVETICA_UPM * font_size)
    })
}

/// Width of carried text in a physical face whose own table covers some
/// characters (`own`): an unpaired unit and a table miss keep the SansSerif
/// measurement, except that a character the face's chain cannot map takes
/// the face's `.notdef` and an unmapped default-ignorable of complex text is
/// hidden (font_chain). The escape prefix is never measured.
fn chain_text_width(
    text: &str,
    font_size: f64,
    bold: bool,
    face: ChainFace,
    own: impl Fn(char) -> Option<f64>,
) -> f64 {
    let complexity = face.complexity_per_token(text);
    let mut buffer = [0_u8; 4];
    java_utf16::tokens(text)
        .zip(complexity)
        .map(|(token, complexity)| match token {
            java_utf16::Token::Unit(unit) => sans_text_width(
                java_utf16::carrier(unit).encode_utf8(&mut buffer),
                font_size,
                bold,
            ),
            java_utf16::Token::Char(c) => {
                own(c).unwrap_or_else(|| match face.resolve(c, complexity) {
                    ChainGlyph::Missing => face.notdef_per_em() * font_size,
                    ChainGlyph::Hidden => 0.0,
                    ChainGlyph::Mapped => sans_text_width(
                        &java_utf16::escape(c.encode_utf8(&mut buffer)),
                        font_size,
                        bold,
                    ),
                })
            }
        })
        .sum()
}

fn family_text_width(
    text: &str,
    font_size: f64,
    bold: bool,
    italic: bool,
    family: MetricFamily,
) -> f64 {
    match family {
        MetricFamily::Mono => pm::mono_text_width_styled(text, font_size, bold, italic),
        MetricFamily::CourierNew => pm::courier_new_text_width(text, font_size, bold, italic),
        MetricFamily::Arial => family_table_text_width(
            text,
            font_size,
            bold,
            italic,
            ChainFamily::Arial,
            &ARIAL_FACES,
        ),
        MetricFamily::Helvetica => helvetica_text_width(text, font_size, bold, italic),
        MetricFamily::Verdana => family_table_text_width(
            text,
            font_size,
            bold,
            italic,
            ChainFamily::Verdana,
            &VERDANA_FACES,
        ),
        MetricFamily::TimesNewRoman => {
            pm::times_new_roman_text_width(text, font_size, bold, italic)
        }
        MetricFamily::Sans => sans_text_width(text, font_size, bold),
    }
}

fn family_text_height(font_size: f64, family: MetricFamily) -> f64 {
    match family {
        MetricFamily::Mono => pm::mono_text_height(font_size),
        MetricFamily::CourierNew => pm::courier_new_text_height(font_size),
        MetricFamily::Arial => font_size * 1.14990234375,
        MetricFamily::Helvetica => font_size,
        MetricFamily::Verdana => font_size * 1.21533203125,
        MetricFamily::TimesNewRoman => pm::times_new_roman_text_height(font_size),
        MetricFamily::Sans => pm::text_height(font_size),
    }
}

fn family_ascent(font_size: f64, family: MetricFamily) -> f64 {
    match family {
        MetricFamily::Mono => pm::mono_ascent(font_size),
        MetricFamily::CourierNew => pm::courier_new_ascent(font_size),
        // Java AWT's Arial line metrics carry non-zero leading. PlantUML's
        // text baseline behaves as ascent + leading, while the text height
        // above already includes the same leading.
        MetricFamily::Arial => font_size * 0.93798828125,
        MetricFamily::Helvetica => font_size * 0.77001953125,
        MetricFamily::Verdana => font_size * 1.00537109375,
        MetricFamily::TimesNewRoman => pm::times_new_roman_ascent(font_size),
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

/// HColorSet.parseSimpleColor / XColor.toSvg: expand short hex and use
/// uppercase RGB, retaining the alpha byte until the paint role is known.
pub(crate) fn normalize_color(s: &str) -> String {
    let value = s.strip_prefix('#').unwrap_or(s);
    if value.bytes().all(|ch| ch.is_ascii_hexdigit()) {
        match value.len() {
            1 => return format!("#{}", value.repeat(6).to_ascii_uppercase()),
            3 => {
                let mut rgb = String::from("#");
                for ch in value.chars() {
                    rgb.push(ch.to_ascii_uppercase());
                    rgb.push(ch.to_ascii_uppercase());
                }
                return rgb;
            }
            6 | 8 => return format!("#{}", value.to_ascii_uppercase()),
            _ => {}
        }
    }
    let lower = value.to_ascii_lowercase();
    // HColors.none() is transparent black. Automatic needs the consumer's
    // active/default background, and must not be guessed here (T14.15).
    if matches!(lower.as_str(), "transparent" | "background") {
        return "#00000000".to_string();
    }
    match css_color_hex(&lower) {
        Some(hex) => hex.to_string(),
        None => s.to_string(),
    }
}

/// Split Java RRGGBBAA into the RGB value and its non-default opacity.
/// The renderer-only Style carries the byte without a floating-point model.
pub(crate) fn color_with_opacity(value: &str) -> (String, Option<u8>) {
    let color = normalize_color(value);
    if color.len() == 9
        && color.starts_with('#')
        && color[1..].bytes().all(|ch| ch.is_ascii_hexdigit())
    {
        let alpha = u8::from_str_radix(&color[7..], 16).unwrap();
        return (color[..7].to_string(), (alpha != u8::MAX).then_some(alpha));
    }
    (color, None)
}

/// CSS3 named colours, lowercased keys. PlantUML accepts the same names
/// (Java AWT's `Color.decode` plus extended palette).
fn css_color_hex(name: &str) -> Option<&'static str> {
    // Listed in case-insensitive lookup order; values are uppercase
    // `#RRGGBB` to match PlantUML's golden SVG output.
    match name {
        // HColorSet delegates to ColorTrieNode's Archimate palette.
        "business" => Some("#FFFFCC"),
        "application" => Some("#C2F0FF"),
        "motivation" => Some("#CCCCFF"),
        "strategy" => Some("#F8E7C0"),
        "technology" => Some("#C9FFC9"),
        "physical" => Some("#97FF97"),
        "implementation" => Some("#FFE0E0"),
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
        // PlantUML's legacy `HColorSet` palette predates the CSS spelling.
        "mediumpurple" => Some("#9370D8"),
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
        "palevioletred" => Some("#D87093"),
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
    let (fill, parsed_opacity) = color_with_opacity(raw_fill);
    let opacity = style.fill_opacity.or(parsed_opacity);
    // DriverTextSvg.draw:94-97 suppresses transparent text before painting.
    // The caller still measures and advances this atom. Shape fill-none
    // serialization is a separate SvgGraphics path.
    if opacity == Some(0) {
        return;
    }
    let opacity_attr = crate::svg::fill_opacity_attribute(opacity);

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
        let escaped = crate::svg::escape_attribute(url);
        // `title` / `xlink:title` is the `{tooltip}` when supplied, else the URL.
        let normalized_title = normalize_svg_link_title(style.link_title.as_deref().unwrap_or(url));
        let title = crate::svg::escape_attribute(&normalized_title);
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
        r#"<text fill="{fill}"{opacity_attr}{filter_attr} font-family="{font_family}" font-size="{font_size}"{style_attr}{weight_attr} lengthAdjust="spacing"{text_decoration} textLength="{tl}" x="{x_s}" y="{y_s}">{content}</text>"#,
        tl = pm::fmt_coord(width),
        x_s = pm::fmt_coord(x),
        y_s = pm::fmt_coord(effective_y), fill = crate::svg::escape_attribute(&fill), font_family = crate::svg::font_family_attribute(font_family),
)
    .unwrap();
    if style.link_url.is_some() {
        buf.push_str("</a>");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn atom_run_measurement_does_not_apply_the_empty_stripe_space() {
        assert_eq!(
            measure_atom_run_with_family("", 11.0, false, "sans-serif"),
            0.0
        );
        assert!(measure_with_family("", 11.0, false, "sans-serif") > 0.0);
        for run in [" ", "   ", "\t"] {
            assert_eq!(
                measure_atom_run_with_family(run, 11.0, false, "sans-serif"),
                measure_with_family(run, 11.0, false, "sans-serif"),
            );
        }
    }

    use super::*;

    #[test]
    fn every_family_measures_java_characters_not_the_escape_prefix() {
        // A legitimate U+10FFFD enters as ESCAPE + itself (java_utf16::escape).
        let escaped = java_utf16::escape("Vault \u{10FFFD}").into_owned();
        assert_ne!(escaped, "Vault \u{10FFFD}");
        for family in [
            "sans-serif",
            "Arial",
            "Helvetica",
            "Verdana",
            "Courier New",
            "Courier",
            "Monospaced",
            "Times New Roman",
        ] {
            let control = measure_with_family("Vault ", 14.0, false, family);
            let width = measure_with_family(&escaped, 14.0, false, family);
            // One missing glyph: the family's .notdef (Times keeps its Java table).
            let notdef = width - control;
            let expected = match family {
                "sans-serif" | "Arial" => 10.5,
                "Helvetica" => 1298.0 / 2048.0 * 14.0,
                "Verdana" => 14.0,
                "Courier New" => pm::courier_new_text_width(" ", 14.0, false, false),
                "Courier" | "Monospaced" => pm::mono_text_width(" ", 14.0),
                _ => 1593.0 / 2048.0 * 14.0,
            };
            assert!(
                (notdef - expected).abs() < 1e-3,
                "{family}: {notdef} vs {expected}"
            );
        }
    }

    #[test]
    fn unmapped_supplementary_format_characters_are_hidden_in_every_family() {
        for family in ["sans-serif", "Arial", "Courier", "Monospaced"] {
            let control = measure_with_family("Vault ", 14.0, false, family);
            let tagged = measure_with_family("Vault \u{E0041}", 14.0, false, family);
            assert!((tagged - control).abs() < 1e-9, "{family}");
        }
    }

    #[test]
    fn inline_colors_use_the_legacy_plantuml_palette() {
        assert_eq!(normalize_color("MediumPurple"), "#9370D8");
        assert_eq!(normalize_color("PaleVioletRed"), "#D87093");
    }

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
    fn transparent_text_advances_without_painting() {
        let style = base(0.0, 20.0);
        for color in ["transparent", "background", "#8044aa00"] {
            let source = format!("q<color:{color}>fir</color>r");
            let mut output = String::new();
            let advance = emit_text(&mut output, &source, &style);
            assert_eq!(advance, total_width("qfirr", &style));
            assert_eq!(output.matches("<text ").count(), 2);
            assert!(!output.contains(">fir</text>"));
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
    fn unresolved_sprite_commands_share_emission_and_measurement() {
        let style = base(0.0, 20.0);
        let mut output = String::new();
        let advance = emit_text(&mut output, "Oak<$missing>Grove", &style);
        assert_eq!(output.matches("<text ").count(), 2);
        assert!(!output.contains("missing"));
        assert_eq!(advance, total_width("OakGrove", &style));
        assert_eq!(advance, total_width("Oak<$missing>Grove", &style));
        output.clear();
        let blank = emit_text(&mut output, "<$missing>", &style);
        assert_eq!(blank, total_width(" ", &style));
        assert!(output.contains("&#160;"));
    }

    #[test]
    fn present_empty_lines_share_the_restored_space_atom() {
        for size in [8, 12, 24] {
            let mut style = base(0.0, 20.0);
            style.font_size = size;
            style.bold = true;
            let mut expected = String::new();
            let width = emit_text(&mut expected, " ", &style);
            for content in [
                "",
                "<$leaf>",
                "<size:31></size>",
                "<size:31><color:red><$leaf>",
            ] {
                let mut output = String::new();
                assert_eq!(emit_text(&mut output, content, &style), width, "{content}");
                assert_eq!(output, expected, "{content}");
                assert_eq!(total_width(content, &style), width);
                assert_eq!(
                    label_height(content, size as f64),
                    label_height(" ", size as f64)
                );
                assert_eq!(
                    label_ascent(content, size as f64),
                    label_ascent(" ", size as f64)
                );
                assert_eq!(emitted_text_element_count(content, &style), 1);
            }
        }
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
    fn first_baseline_ascent_follows_first_mixed_family_run() {
        let mono_first = r#"""mono"" start"#;
        let max_ascent = label_ascent_with_family(mono_first, 12.0, "sans-serif");
        let first_ascent = label_first_baseline_ascent_with_family(mono_first, 12.0, "sans-serif");
        assert_eq!(pm::fmt_coord(max_ascent - first_ascent), "0.2988");

        let sans_first = r#"start ""mono"""#;
        let max_ascent = label_ascent_with_family(sans_first, 12.0, "sans-serif");
        let first_ascent = label_first_baseline_ascent_with_family(sans_first, 12.0, "sans-serif");
        assert_eq!(pm::fmt_coord(max_ascent - first_ascent), "0");

        let small_plain = "plain";
        let max_ascent = label_ascent_with_family(small_plain, 8.0, "sans-serif");
        let first_ascent = label_first_baseline_ascent_with_family(small_plain, 8.0, "sans-serif");
        assert_eq!(pm::fmt_coord(max_ascent - first_ascent), "0");
    }

    #[test]
    fn positioned_first_run_uses_sea_aligned_baseline_for_renamed_notes() {
        let line_top = 17.0;
        for content in ["<sub>renamed-index</sub>", "<sup>renamed-power</sup>"] {
            let anchor =
                line_top + label_first_baseline_ascent_with_family(content, 13.0, "sans-serif");
            let mut b = base(11.0, anchor);
            b.font_size = 13;
            let mut buf = String::new();
            emit_text(&mut buf, content, &b);

            let expected_baseline =
                pm::fmt_coord(line_top + family_ascent(10.0, MetricFamily::Sans));
            assert!(
                buf.contains(&format!(r#" y="{expected_baseline}">"#)),
                "{content} emitted at the wrong baseline: {buf}"
            );
        }
    }

    #[test]
    fn quoted_monospace_still_uses_monospace_metrics() {
        assert_eq!(
            pm::fmt_coord(measure(r#"""code here"""#, 12.0, false)),
            pm::fmt_coord(pm::mono_text_width("code\u{00a0}here", 12.0))
        );
    }

    #[test]
    fn tabs_split_text_at_eight_space_stops() {
        let mut b = base(35.3618, 63.0566);
        b.font_size = 13;

        let mut buf = String::new();
        let width = emit_text(&mut buf, r"col1\tcol2", &b);
        let col1_w = pm::text_width("col1", 13.0, false);
        let col2_w = pm::text_width("col2", 13.0, false);
        let tab_w = pm::text_width("        ", 13.0, false);

        assert_eq!(buf.matches("<text").count(), 2);
        assert!(buf.contains(&format!(
            r#"textLength="{}" x="35.3618""#,
            pm::fmt_coord(col1_w)
        )));
        assert!(buf.contains(&format!(
            r#"textLength="{}" x="{}""#,
            pm::fmt_coord(col2_w),
            pm::fmt_coord(35.3618 + tab_w)
        )));
        assert_eq!(pm::fmt_coord(width), pm::fmt_coord(tab_w + col2_w));
        assert_eq!(
            pm::fmt_coord(measure(r"col1\tcol2", 13.0, false)),
            pm::fmt_coord(width)
        );
    }

    /// Java's widths for the four alphabet runs at 14 pt in each Verdana
    /// face (BT relay, `c10a_ft_vd_{italic,bolditalic,bold}.svg` textLength,
    /// pinned Java 71806a23): the italic faces have advances of their own.
    #[test]
    fn verdana_faces_measure_java_widths() {
        let runs = [
            "ABCDEFGHIJKLM",
            "NOPQRSTUVWXYZ",
            "abcdefghijklm",
            "nopqrstuvwxyz",
        ];
        for (bold, italic, java) in [
            (false, true, ["119.4717", "130.3477", "98.3145", "106.0869"]),
            (true, true, ["133.4307", "144.9082", "111.5557", "120.3945"]),
            (
                true,
                false,
                ["133.4307", "144.9082", "111.5557", "120.4219"],
            ),
        ] {
            for (run, java) in runs.iter().zip(java) {
                assert_eq!(
                    pm::fmt_coord(measure_with_family_and_face(
                        run, 14.0, bold, italic, "Verdana"
                    )),
                    java,
                    "{run} bold={bold} italic={italic}"
                );
            }
        }
    }

    /// Every ASCII table is its face's stored extraction
    /// (tools/font-chain-reference/ascii-advances-round11, whose font files
    /// hash as the reference host's), and each (bold, italic) slot reads its
    /// own face's table.
    #[test]
    fn ascii_tables_are_the_extracted_faces() {
        use sha2::{Digest, Sha256};
        use std::collections::HashMap;
        const DIR: &str = "tools/font-chain-reference/ascii-advances-round11";
        const SHA256SUMS_SHA256: &str =
            "1fa07b8c424f6f6e1d385cbdcd439d6f9d3dab5e0cd9096b0a66ac497ce845d4";
        const REFERENCE_HOST_FONT_HASHES: &str =
            "tools/font-chain-reference/monkeyman-round9/font-hashes.txt";
        let hex = |bytes: &[u8]| {
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = root.join(DIR);
        let sums = std::fs::read(dir.join("SHA256SUMS")).expect("stored SHA256SUMS");
        assert_eq!(hex(&sums), SHA256SUMS_SHA256);
        for line in String::from_utf8(sums).unwrap().lines() {
            let (hash, name) = line.split_once("  ").expect("shasum line");
            let data = std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(hex(&data), hash, "{name}");
        }
        let font_hashes = |path: std::path::PathBuf| -> HashMap<String, String> {
            std::fs::read_to_string(path)
                .unwrap()
                .lines()
                .filter_map(|line| {
                    let (hash, path) = line.split_once("  ")?;
                    (hash.len() == 64).then(|| (path.to_string(), hash.to_string()))
                })
                .collect()
        };
        let here = font_hashes(dir.join("font-hashes.txt"));
        let reference_host = font_hashes(root.join(REFERENCE_HOST_FONT_HASHES));
        let mut slots = Vec::new();
        for (faces, family, files) in [
            (
                &ARIAL_FACES,
                "Arial",
                [
                    "ArialMT.plain",
                    "Arial-BoldMT.bold",
                    "Arial-ItalicMT.italic",
                    "Arial-BoldItalicMT.bolditalic",
                ],
            ),
            (
                &VERDANA_FACES,
                "Verdana",
                [
                    "Verdana.plain",
                    "Verdana-Bold.bold",
                    "Verdana-Italic.italic",
                    "Verdana-BoldItalic.bolditalic",
                ],
            ),
        ] {
            let styles = [(false, false), (true, false), (false, true), (true, true)];
            for ((bold, italic), file) in styles.into_iter().zip(files) {
                let text = std::fs::read_to_string(dir.join(format!("{file}.ascii.tsv")))
                    .unwrap_or_else(|e| panic!("{file}: {e}"));
                let header = |key: &str| {
                    text.lines()
                        .find_map(|line| line.strip_prefix(key))
                        .unwrap_or_else(|| panic!("{file}: no {key}"))
                        .to_string()
                };
                assert!(header("#units_per_em\t").starts_with("2048\t"), "{file}");
                let font_file = header("#file\t");
                assert!(
                    font_file.contains(family),
                    "{file}: measured face file {font_file}"
                );
                assert_eq!(
                    here.get(&font_file),
                    reference_host.get(&font_file),
                    "{font_file}: extraction host and reference host disagree"
                );
                assert!(reference_host.contains_key(&font_file), "{font_file}");
                let units: Vec<u16> = text
                    .lines()
                    .filter_map(|line| line.strip_prefix("A\t"))
                    .enumerate()
                    .map(|(index, fields)| {
                        let fields: Vec<&str> = fields.split('\t').collect();
                        assert_eq!(
                            u32::from_str_radix(fields[0], 16).unwrap(),
                            0x20 + index as u32,
                            "{file}"
                        );
                        let advance: f64 = fields[2].parse().unwrap();
                        assert_eq!(advance.fract(), 0.0, "{file} {}", fields[0]);
                        advance as u16
                    })
                    .collect();
                assert_eq!(
                    faces.slot(bold, italic).as_slice(),
                    units.as_slice(),
                    "{file}: bold={bold} italic={italic}"
                );
                slots.push(file);
            }
        }
        assert_eq!(slots.len(), 8);
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
            pm::fmt_coord(measure_with_family("Alice", 12.0, false, "Helvetica")),
            "26.0098"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family("Bob", 12.0, false, "Helvetica")),
            "21.3516"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family("hello", 12.0, false, "Helvetica")),
            "25.3535"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family("hi", 12.0, false, "Helvetica")),
            "9.3398"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family("Foo", 14.0, false, "Helvetica")),
            "24.124"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family("Bar", 14.0, false, "Helvetica")),
            "21.7861"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family("«interface»", 12.0, false, "Helvetica")),
            "59.373"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family("Measurable", 12.0, true, "Helvetica")),
            "66.0293"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family("LANDSCAPE", 12.0, false, "Helvetica")),
            "72.6914"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family_and_face(
                "M",
                16.0,
                false,
                false,
                "Times New Roman"
            )),
            "14.2266"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family_and_face(
                "M",
                16.0,
                true,
                false,
                "Times New Roman"
            )),
            "15.1016"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family_and_face(
                "M",
                16.0,
                false,
                true,
                "Times New Roman"
            )),
            "13.3281"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family_and_face(
                "M",
                16.0,
                true,
                true,
                "Times New Roman"
            )),
            "14.2266"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family_and_face(
                "italic self label",
                18.0,
                false,
                true,
                "Times New Roman"
            )),
            "106.9805"
        );
        // Java AWT values preserved by the independent Unicode checker and
        // regenerated across the complete assigned-scalar domain by
        // `PhysicalFontMetricsExtract --rust-unicode-ranges`.
        assert_eq!(
            pm::fmt_coord(measure_with_family_and_face(
                "Café Ωmega 27",
                15.0,
                false,
                true,
                "Times New Roman"
            )),
            "94.1675"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family_and_face(
                "Журнал Z8",
                15.0,
                false,
                true,
                "Times New Roman"
            )),
            "68.9722"
        );
        assert_eq!(
            pm::fmt_coord(measure_with_family_and_face(
                "Crème brûlée — déjà vu",
                19.0,
                false,
                true,
                "Times New Roman"
            )),
            "185.7324"
        );
        assert_eq!(
            pm::fmt_coord(text_height_for_family(16.0, "Arial")),
            "18.3984"
        );
        assert_eq!(pm::fmt_coord(ascent_for_family(16.0, "Arial")), "15.0078");
        assert_eq!(
            pm::fmt_coord(text_height_for_family(12.0, "Helvetica")),
            "12"
        );
        assert_eq!(
            pm::fmt_coord(ascent_for_family(12.0, "Helvetica")),
            "9.2402"
        );
        assert_eq!(
            pm::fmt_coord(text_height_for_family(16.0, "Times New Roman")),
            "18.3984"
        );
        assert_eq!(
            pm::fmt_coord(ascent_for_family(16.0, "Times New Roman")),
            "14.9375"
        );
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
    fn creole_link_normalizes_title_without_decoding_href() {
        let mut fallback = String::new();
        emit_text(
            &mut fallback,
            r"[[https://example.com/a\nb linked fallback]]",
            &base(0.0, 0.0),
        );
        assert!(fallback.contains(r#"href="https://example.com/a\nb""#));
        assert!(fallback.contains(r#"title="https://example.com/a&#10;b""#));

        let mut explicit = String::new();
        emit_text(
            &mut explicit,
            r"[[https://example.com/raw\npath{Docs <U+0026> Help\nLine} linked explicit]]",
            &base(0.0, 0.0),
        );
        assert!(explicit.contains(r#"href="https://example.com/raw\npath""#));
        assert!(explicit.contains(r#"title="Docs &amp; Help&#10;Line""#));
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

    /// The face routing of every width consumer for each italic and
    /// bold-italic slot of the families whose faces differ by style (Arial,
    /// Helvetica, Verdana, Menlo, Courier New). Through the public consumer
    /// path (`measure_with_family_and_face`, as layout and emission call it)
    /// a string measures as it does in `ChainFace::for_style`'s face (JDK
    /// `FontFamily.getFont`), by the per-glyph rule of `chain_text_width`
    /// (own table, else the chain: mapped, `.notdef` or hidden) or the
    /// monospaced cell rule. A character in the family's own table is
    /// measured with the table of that very face (selected here, not through
    /// the router under test), so a dropped italic table is caught. The
    /// strings that tell the italic face from the face a dropped italic would
    /// pick are searched in the stored tables and coverage (ASCII letters and
    /// digits alone; each scalar from U+00A0 alone, after `a` and after a
    /// Hebrew base, so both JDK paths are reached), not listed by hand. A slot with no such
    /// string fails unless it is in `EQUIVALENT`: a pair whose faces give
    /// every candidate the same width, so no output can tell the routes
    /// apart; the list fails when stale in either direction.
    #[test]
    fn every_italic_slot_measures_in_its_own_face() {
        use crate::font_chain::{ChainFace, ChainFamily, ChainGlyph};
        const SIZE: f64 = 14.0;
        const MAX_PROBES: usize = 8;
        const HEBREW_ALEF: char = '\u{05D0}';
        // Menlo Bold Italic and Menlo Bold hide the same scalars in every
        // context (monospaced width counts cells only).
        const EQUIVALENT: [(&str, bool); 1] = [("Monospaced", true)];
        // The face's own ASCII table, chosen by the face's own (bold, italic).
        fn ascii(table: &[u16; 95], c: char) -> Option<f64> {
            let code = c as usize;
            (32..=126)
                .contains(&code)
                .then(|| f64::from(table[code - 32]) / ASCII_UNITS_PER_EM * SIZE)
        }
        type OwnWidth = fn(char, bool, bool) -> Option<f64>;
        // (family name as consumers pass it, chain family, own advance table)
        let families: [(&str, ChainFamily, Option<OwnWidth>); 5] = [
            (
                "Arial",
                ChainFamily::Arial,
                Some(|c, bold, italic| match (bold, italic) {
                    (false, false) => ascii(&ARIAL_ASCII, c),
                    (true, false) => ascii(&ARIAL_BOLD_ASCII, c),
                    (false, true) => ascii(&ARIAL_ITALIC_ASCII, c),
                    (true, true) => ascii(&ARIAL_BOLD_ITALIC_ASCII, c),
                }),
            ),
            (
                // Helvetica-Oblique and -BoldOblique share the upright
                // advances (ascii-advances-round11); the upright tables
                // themselves are T14.49.
                "Helvetica",
                ChainFamily::Helvetica,
                Some(|c, bold, _| {
                    if bold {
                        helvetica_bold_char_units(c)
                    } else {
                        helvetica_char_units(c)
                    }
                    .map(|units| units / HELVETICA_UPM * SIZE)
                }),
            ),
            (
                "Verdana",
                ChainFamily::Verdana,
                Some(|c, bold, italic| match (bold, italic) {
                    (false, false) => ascii(&VERDANA_ASCII, c),
                    (true, false) => ascii(&VERDANA_BOLD_ASCII, c),
                    (false, true) => ascii(&VERDANA_ITALIC_ASCII, c),
                    (true, true) => ascii(&VERDANA_BOLD_ITALIC_ASCII, c),
                }),
            ),
            ("Monospaced", ChainFamily::Menlo, None),
            ("Courier New", ChainFamily::CourierNew, None),
        ];
        let candidates: Vec<String> = ('0'..='9')
            .chain('A'..='Z')
            .chain('a'..='z')
            .map(String::from)
            .chain(
                (0xA0_u32..=0x2_FFFF)
                    .filter_map(char::from_u32)
                    .flat_map(|c| [format!("{c}"), format!("a{c}"), format!("{HEBREW_ALEF}{c}")]),
            )
            .collect();
        for (name, family, own) in &families {
            for bold in [false, true] {
                let public =
                    |text: &str| measure_with_family_and_face(text, SIZE, bold, true, name);
                let cell = public("a");
                // Width of `text` in `face` (style `italic`), per Java token
                // of the string.
                let in_face = |face: ChainFace, italic: bool, text: &str| -> f64 {
                    let complexity = face.complexity_per_token(text);
                    text.chars()
                        .zip(complexity)
                        .map(|(c, complexity)| {
                            let glyph = face.resolve(c, complexity);
                            match own.map(|own| own(c, bold, italic)) {
                                None => f64::from(u8::from(glyph != ChainGlyph::Hidden)) * cell,
                                Some(Some(width)) => width,
                                Some(None) => match glyph {
                                    ChainGlyph::Missing => face.notdef_per_em() * SIZE,
                                    ChainGlyph::Hidden => 0.0,
                                    ChainGlyph::Mapped => sans_text_width(
                                        &java_utf16::escape(c.encode_utf8(&mut [0_u8; 4])),
                                        SIZE,
                                        bold,
                                    ),
                                },
                            }
                        })
                        .sum()
                };
                let face = ChainFace::for_style(*family, bold, true);
                let dropped = ChainFace::for_style(*family, bold, false);
                let probes: Vec<&String> = candidates
                    .iter()
                    .filter(|text| {
                        (in_face(face, true, text) - in_face(dropped, false, text)).abs() > 1e-9
                    })
                    .take(MAX_PROBES)
                    .collect();
                assert_eq!(
                    probes.is_empty(),
                    EQUIVALENT.contains(&(*name, bold)),
                    "{name} bold={bold}: {face:?} vs {dropped:?} probes {probes:?}; update EQUIVALENT only if the faces' widths agree on every candidate"
                );
                if probes.is_empty() {
                    assert!(
                        face != dropped,
                        "{name} bold={bold}: for_style ignores italic"
                    );
                }
                for text in probes {
                    assert!(
                        (public(text) - in_face(face, true, text)).abs() < 1e-9,
                        "{name} bold={bold} italic {text:?}: measured {}, {face:?} gives {}, {dropped:?} gives {}",
                        public(text),
                        in_face(face, true, text),
                        in_face(dropped, false, text),
                    );
                }
            }
        }
    }

    /// JDK 21 on macOS resolves a skinparam font name before measuring it
    /// (`Font.decode`, then `SunFontManager.findFont2D`). "Wharf qr" at 14 pt
    /// is eight Menlo cells, `8 * 1233/2048 * 14`.
    #[test]
    fn jdk_font_name_resolves_every_modelled_family() {
        let wharf = "Wharf qr";
        let menlo = pm::mono_text_width(wharf, 14.0);
        let cell = 8.0 * 1233.0 / 2048.0 * 14.0;
        assert!((menlo - cell).abs() < 1e-9, "{menlo} vs {cell}");
        for name in [
            "Menlo",
            "MENLO",
            " menlo ",
            "\"Menlo\"",
            "'Menlo'",
            "Menlo-14",
            "Menlo-Bold",
            "Menlo Bold",
            "Menlo-Bold-14",
            "menlo plain",
            "Menlo-Regular",
            "menlo-regular",
        ] {
            let width = measure_with_family(wharf, 14.0, false, name);
            assert!(
                (width - menlo).abs() < 1e-9,
                "{name}: {width} vs Menlo {menlo}"
            );
        }
        let lucida = measure_with_family(wharf, 14.0, false, "sans-serif");
        assert!(
            (lucida - menlo).abs() > 1.0,
            "Menlo collapsed onto Lucida Grande ({lucida})"
        );
        assert_eq!(
            measure_with_family(wharf, 14.0, false, "Times"),
            measure_with_family(wharf, 14.0, false, "Dialog")
        );
        assert_eq!(measure_with_family(wharf, 14.0, false, "NotAFont"), lucida);

        let bang = "!";
        assert_eq!(
            measure_with_family(bang, 14.0, false, "Arial-BoldMT"),
            measure_with_family_and_face(bang, 14.0, true, false, "Arial")
        );
        assert_ne!(
            measure_with_family(bang, 14.0, false, "Arial-BoldMT"),
            measure_with_family(bang, 14.0, false, "Arial")
        );
        assert_eq!(
            measure_with_family(bang, 14.0, false, "Arial-Bold"),
            measure_with_family(bang, 14.0, false, "Arial")
        );
        assert_eq!(
            measure_with_family(bang, 14.0, false, "courier"),
            measure_with_family(bang, 14.0, false, "Monospaced")
        );
        assert_eq!(
            measure_with_family(bang, 14.0, false, "serif"),
            measure_with_family(bang, 14.0, false, "Times New Roman")
        );
        assert_eq!(
            measure_with_family(bang, 14.0, false, "timesroman"),
            measure_with_family(bang, 14.0, false, "Times New Roman")
        );
        assert_eq!(
            measure_with_family("<font:Menlo>Wharf qr", 14.0, false, "sans-serif"),
            menlo
        );
        // Creole `<font:monospace>` keeps the surrounding face. The same
        // spelling as a base family is the logical Monospaced font.
        let creole_mono =
            measure_with_family("<font:monospace>code here", 14.0, false, "sans-serif");
        let surrounding = measure_with_family("code here", 14.0, false, "sans-serif");
        assert!(
            (creole_mono - surrounding).abs() < 1e-9,
            "creole monospace {creole_mono} vs surrounding {surrounding}"
        );
        assert!(
            (measure_with_family("code here", 14.0, false, "monospace")
                - pm::mono_text_width("code here", 14.0))
            .abs()
                < 1e-9
        );

        assert_eq!(
            resolve_modelled_face("Helvetica-Oblique", false, false),
            (MetricFamily::Helvetica, false, true)
        );
        assert_eq!(
            resolve_modelled_face("Helvetica-BoldOblique", false, false),
            (MetricFamily::Helvetica, true, true)
        );
        assert_eq!(
            resolve_modelled_face("Menlo-Regular", true, true),
            (MetricFamily::Mono, true, true)
        );
        assert_eq!(
            resolve_modelled_face("LucidaGrande", false, false),
            (MetricFamily::Sans, false, false)
        );
        assert_eq!(
            resolve_modelled_face("CourierNewPS-ItalicMT", false, false),
            (MetricFamily::CourierNew, false, true)
        );
        assert_eq!(
            resolve_modelled_face("TimesNewRomanPS-BoldMT", false, true),
            (MetricFamily::TimesNewRoman, true, true)
        );
        assert_eq!(decoded_font_name("Arial-BoldMT"), "Arial-BoldMT");
        assert_eq!(decoded_font_name("Menlo-Bold"), "Menlo");
        assert_eq!(
            decoded_font_name("Helvetica-BoldOblique"),
            "Helvetica-BoldOblique"
        );
        assert_eq!(decoded_font_name("Courier New"), "Courier New");
        assert_eq!(decoded_font_name("Lucida Grande"), "Lucida Grande");

        let mut buf = String::new();
        let mut drawn = base(0.0, 20.0);
        drawn.font_family = "Menlo";
        emit_text(&mut buf, wharf, &drawn);
        assert!(buf.contains(r#"font-family="Menlo""#), "{buf}");
    }
}
