// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Uniform geometric scaling of rendered SVG.
//!
//! PlantUML implements `skinparam dpi N` and the `scale` directive as a single
//! mechanism: the diagram is laid out and rendered at the base resolution (96
//! dpi) and then every numeric quantity in the output SVG is multiplied by a
//! uniform factor `k`, formatted with PlantUML's 4-decimal-place rounding.
//!
//! * `skinparam dpi N` → `k = N / 96`
//! * `scale N`          → `k = N`
//!
//! This module provides:
//!
//! * [`factor_from_meta`] — derive `k` from a diagram's metadata (returns 1.0
//!   when no dpi/scale directive is present, so the scaling path is inert for
//!   the overwhelming majority of diagrams and output stays byte-identical).
//! * [`scale_svg_numbers`] — multiply every geometric number in a finished SVG
//!   string by `k`.
//! * [`scale_oracle_layout`] — multiply (or divide, for the inverse pass) every
//!   captured coordinate in an [`OracleLayout`] by a factor.
//!
//! The render pipeline (see `lib.rs`) uses these as a sandwich when `k != 1`:
//! the oracle layout (captured at golden, i.e. *scaled*, coordinates) is
//! un-scaled by `1/k`, the renderer runs at base coordinates, and the resulting
//! SVG is scaled back up by `k`. The round-trip `÷k` then `×k` drifts by only a
//! few ULPs, which PlantUML's 4-dp rounding absorbs.

use crate::layout_oracle::{
    ApointMark, CrowMark, EdgeDecoration, EntityRect, JsonBox, OracleEdgePath, OracleLayout,
    OracleNoteChild,
};
use crate::plantuml_metrics::{fmt_coord, with_full_precision};
use rustuml_parser::diagram::DiagramMeta;

/// Format a number for the active scaling pass. Delegates to the shared
/// [`fmt_coord`], which emits full round-trippable precision while the
/// full-precision thread-local is active (oracle un-scale pass) and PlantUML's
/// 4-dp rounding otherwise (final-SVG forward pass). This keeps the inverse
/// pass loss-free: `48.9463 / 2 = 24.47315` is preserved rather than rounded
/// to `24.4732` (which would re-scale to the wrong `48.9464`).
fn fmt_num(v: f64) -> String {
    fmt_coord(v)
}

/// Derive the uniform geometric scale factor `k` from a diagram's metadata.
///
/// Honours `skinparam dpi N` (`k = N/96`) and the `scale` directive (`scale N`
/// → `k = N`). Returns `1.0` when neither is present or when the directive
/// resolves to factor 1, keeping the scaling pipeline inert.
///
/// The fit-to-box `scale` forms (`scale W*H`, `scale WxH`, `scale max N
/// width/height`) are NOT handled here — they require the base natural canvas
/// size, which is unknown at this point. They resolve to `1.0` (no scaling).
pub fn factor_from_meta(meta: &DiagramMeta) -> f64 {
    // `skinparam dpi N` — last one wins, matching PlantUML's override order.
    let dpi = meta
        .skinparams
        .iter()
        .rev()
        .find(|sp| sp.key.eq_ignore_ascii_case("dpi"))
        .and_then(|sp| sp.value.trim().parse::<f64>().ok());
    if let Some(dpi) = dpi
        && dpi > 0.0
    {
        return dpi / 96.0;
    }

    // `scale N` directive — scan the diagram source. The directive is a
    // standalone line `scale <expr>`. We only honour the bare numeric form
    // (`scale 2`, `scale 1.5`); fit-to-box forms are deferred.
    if let Some(src) = meta.source.as_deref() {
        for line in src.lines() {
            let t = line.trim();
            let Some(rest) = t.strip_prefix("scale ") else {
                continue;
            };
            let rest = rest.trim();
            // Reject fit-to-box forms: `scale max ...`, `scale W*H`, `scale WxH`.
            if rest.starts_with("max ")
                || rest.contains('*')
                || rest.contains('x')
                || rest.contains('X')
            {
                continue;
            }
            if let Ok(n) = rest.parse::<f64>()
                && n > 0.0
            {
                return n;
            }
        }
    }

    1.0
}

/// Multiply every geometric number in a finished SVG string by `k`, formatting
/// results with PlantUML's 4-dp rounding ([`fmt_coord`]).
///
/// The root `<svg>` element is handled specially to mirror PlantUML: its
/// `width`/`height` pixel attributes carry the exact scaled value
/// (`221.875px`), while `viewBox` and the `style` width/height are the integer
/// floor of the scaled value (`viewBox="0 0 221 303"`, `style="width:221px"`).
///
/// All other tags have their geometric attributes scaled uniformly:
/// `x y cx cy rx ry width height x1 y1 x2 y2 textLength font-size`, the
/// `stroke-width:`/`stroke-dasharray:` lengths inside `style="…"`, every number
/// in a `<path d="…">`, and every number in `points="…"`. Non-geometric
/// attributes (fill, stroke colour, ids, `data-*`, font-family) are untouched.
pub fn scale_svg_numbers(svg: &str, k: f64) -> String {
    if k == 1.0 {
        return svg.to_string();
    }
    let bytes = svg.as_bytes();
    let mut out = String::with_capacity(svg.len() + svg.len() / 8);
    let mut i = 0;
    let mut first_tag = true;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Find the end of this tag (next unquoted '>').
            let tag_start = i;
            let mut j = i + 1;
            let mut in_quote = 0u8;
            while j < bytes.len() {
                let c = bytes[j];
                if in_quote != 0 {
                    if c == in_quote {
                        in_quote = 0;
                    }
                } else if c == b'"' || c == b'\'' {
                    in_quote = c;
                } else if c == b'>' {
                    break;
                }
                j += 1;
            }
            // `j` now points at '>' (or end of input for malformed SVG).
            let tag = &svg[tag_start..j.min(bytes.len())];
            // Processing instructions / comments / closing tags: copy verbatim.
            let is_element = tag
                .as_bytes()
                .get(1)
                .is_some_and(|&c| c.is_ascii_alphabetic());
            if !is_element {
                out.push_str(tag);
            } else {
                let is_root_svg = first_tag && tag.starts_with("<svg");
                out.push_str(&scale_tag(tag, k, is_root_svg));
                first_tag = false;
            }
            if j < bytes.len() {
                out.push('>');
            }
            i = j + 1;
        } else {
            // Text content between tags — copy verbatim (never scaled).
            let start = i;
            while i < bytes.len() && bytes[i] != b'<' {
                i += 1;
            }
            out.push_str(&svg[start..i]);
        }
    }
    out
}

/// Geometric attributes scaled uniformly on every element.
const SCALED_ATTRS: &[&str] = &[
    "x",
    "y",
    "cx",
    "cy",
    "rx",
    "ry",
    "width",
    "height",
    "x1",
    "y1",
    "x2",
    "y2",
    "textLength",
    "font-size",
];

/// Rewrite a single opening/self-closing tag (without the trailing `>`).
fn scale_tag(tag: &str, k: f64, is_root_svg: bool) -> String {
    // Tag name end.
    let name_end = tag[1..]
        .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .map(|p| p + 1)
        .unwrap_or(tag.len());
    let mut out = String::with_capacity(tag.len() + 16);
    out.push_str(&tag[..name_end]);

    let mut rest = &tag[name_end..];
    while let Some(eq) = rest.find('=') {
        // Emit everything up to and including the attribute name + '='.
        let name_seg = &rest[..eq];
        let name = name_seg.trim();
        out.push_str(name_seg);
        out.push('=');
        let after_eq = &rest[eq + 1..];
        // The value is quoted.
        let quote = after_eq.as_bytes().first().copied();
        let Some(q) = quote.filter(|&c| c == b'"' || c == b'\'') else {
            // No quoted value (shouldn't happen in our SVG); copy and stop.
            out.push_str(after_eq);
            return out;
        };
        let qc = q as char;
        let val_start = 1;
        let Some(val_len) = after_eq[val_start..].find(qc) else {
            out.push_str(after_eq);
            return out;
        };
        let value = &after_eq[val_start..val_start + val_len];

        let new_value = scale_attr_value(name, value, k, is_root_svg);
        out.push(qc);
        out.push_str(&new_value);
        out.push(qc);

        rest = &after_eq[val_start + val_len + 1..];
    }
    out.push_str(rest);
    out
}

/// Compute the scaled replacement for one attribute value.
fn scale_attr_value(name: &str, value: &str, k: f64, is_root_svg: bool) -> String {
    if is_root_svg {
        match name {
            // Pixel size attrs carry the exact scaled value.
            "width" | "height" => {
                if let Some(num) = value.strip_suffix("px")
                    && let Ok(v) = num.parse::<f64>()
                {
                    return format!("{}px", fmt_coord(v * k));
                }
                return value.to_string();
            }
            // viewBox: "minx miny w h" — scale each, floor to integer.
            "viewBox" => {
                let parts: Vec<String> = value
                    .split_whitespace()
                    .map(|p| match p.parse::<f64>() {
                        Ok(v) => format!("{}", (v * k).floor() as i64),
                        Err(_) => p.to_string(),
                    })
                    .collect();
                return parts.join(" ");
            }
            // style="width:NNNpx;height:NNNpx;background:#FFFFFF;" — floor.
            "style" => return scale_root_style(value, k),
            _ => return value.to_string(),
        }
    }

    if SCALED_ATTRS.contains(&name) {
        return scale_number_token(value, k);
    }
    match name {
        "d" => scale_path_d(value, k),
        "points" => scale_number_list(value, k),
        "style" => scale_style(value, k),
        _ => value.to_string(),
    }
}

/// Scale a value that is a single number, optionally with a `px` suffix.
fn scale_number_token(value: &str, k: f64) -> String {
    if let Some(num) = value.strip_suffix("px")
        && let Ok(v) = num.parse::<f64>()
    {
        return format!("{}px", fmt_num(v * k));
    }
    match value.parse::<f64>() {
        Ok(v) => fmt_num(v * k),
        Err(_) => value.to_string(),
    }
}

/// Root `<svg style>`: floor the `width`/`height` lengths, leave colours etc.
fn scale_root_style(value: &str, k: f64) -> String {
    value
        .split(';')
        .map(|decl| {
            let Some((prop, val)) = decl.split_once(':') else {
                return decl.to_string();
            };
            let p = prop.trim();
            if (p == "width" || p == "height")
                && let Some(num) = val.trim().strip_suffix("px")
                && let Ok(v) = num.parse::<f64>()
            {
                return format!("{prop}:{}px", (v * k).floor() as i64);
            }
            decl.to_string()
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Element `style="…"`: scale `stroke-width` and `stroke-dasharray` lengths.
fn scale_style(value: &str, k: f64) -> String {
    value
        .split(';')
        .map(|decl| {
            let Some((prop, val)) = decl.split_once(':') else {
                return decl.to_string();
            };
            let p = prop.trim();
            match p {
                "stroke-width" => format!("{prop}:{}", scale_number_token(val.trim(), k)),
                "stroke-dasharray" => {
                    format!("{prop}:{}", scale_number_list(val.trim(), k))
                }
                _ => decl.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Scale every number in a whitespace/comma-separated list (`points`,
/// `stroke-dasharray`), preserving the original separators.
fn scale_number_list(value: &str, k: f64) -> String {
    scale_numbers_in(value, k, |c| c == ',' || c.is_whitespace())
}

/// Scale every number embedded in an SVG path `d` attribute, preserving
/// command letters and separators.
fn scale_path_d(value: &str, k: f64) -> String {
    scale_numbers_in(value, k, |c| {
        c == ',' || c.is_whitespace() || c.is_ascii_alphabetic()
    })
}

/// Walk `s`, scaling each maximal numeric run (matching a signed decimal /
/// exponent token) by `k` and copying every other byte (separators, command
/// letters) verbatim. `is_sep` identifies bytes that delimit numbers.
fn scale_numbers_in(s: &str, k: f64, is_sep: impl Fn(char) -> bool) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 8);
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        // Start of a number: digit, leading '.', or sign immediately before a
        // digit/'.'.
        let is_num_start = c.is_ascii_digit()
            || (c == '.' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit())
            || ((c == '-' || c == '+')
                && i + 1 < bytes.len()
                && ((bytes[i + 1] as char).is_ascii_digit() || bytes[i + 1] == b'.'));
        if is_num_start {
            let start = i;
            i += 1; // consume sign/first char
            while i < bytes.len() {
                let d = bytes[i] as char;
                if d.is_ascii_digit() || d == '.' {
                    i += 1;
                } else if (d == 'e' || d == 'E')
                    && i + 1 < bytes.len()
                    && ((bytes[i + 1] as char).is_ascii_digit()
                        || bytes[i + 1] == b'-'
                        || bytes[i + 1] == b'+')
                {
                    i += 2;
                } else {
                    break;
                }
            }
            let tok = &s[start..i];
            match tok.parse::<f64>() {
                Ok(v) => out.push_str(&fmt_num(v * k)),
                Err(_) => out.push_str(tok),
            }
        } else {
            // Copy the separator/letter byte run verbatim.
            let start = i;
            while i < bytes.len() {
                let d = bytes[i] as char;
                let num_starts = d.is_ascii_digit()
                    || (d == '.' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit())
                    || ((d == '-' || d == '+')
                        && i + 1 < bytes.len()
                        && ((bytes[i + 1] as char).is_ascii_digit() || bytes[i + 1] == b'.'));
                if num_starts {
                    break;
                }
                // Only advance over separator chars; if we hit something that
                // is neither a separator nor a number start, still copy it.
                let _ = is_sep(d);
                i += 1;
            }
            out.push_str(&s[start..i]);
        }
    }
    out
}

// ─── Oracle layout scaling ──────────────────────────────────────────────────

/// Scale every captured coordinate in an [`OracleLayout`] by `k` (in place).
///
/// Used with `k = 1/factor` to un-scale a golden-derived oracle back to base
/// coordinates before rendering. Numeric fields are multiplied directly;
/// verbatim XML fragments (cluster/note inner XML, captured `<defs>`, etc.) are
/// re-scaled with [`scale_svg_numbers`] so embedded coordinates track.
pub fn scale_oracle_layout(o: &mut OracleLayout, k: f64) {
    if k == 1.0 {
        return;
    }
    with_full_precision(|| scale_oracle_layout_inner(o, k));
}

fn scale_oracle_layout_inner(o: &mut OracleLayout, k: f64) {
    o.canvas_width *= k;
    o.canvas_height *= k;

    for e in o.entities.values_mut() {
        scale_entity(e, k);
    }
    for edge in &mut o.edges {
        scale_edge(edge, k);
    }
    for c in &mut o.clusters {
        c.inner_xml = scale_svg_numbers(&c.inner_xml, k);
    }
    for n in &mut o.note_entities {
        if let Some(g) = n.box_geom.as_mut() {
            g.x *= k;
            g.y *= k;
            g.width *= k;
            g.height *= k;
            if let Some((ax, ay)) = g.apex.as_mut() {
                *ax *= k;
                *ay *= k;
            }
            if let Some(((x1, y1), (x2, y2))) = g.leader_base.as_mut() {
                *x1 *= k;
                *y1 *= k;
                *x2 *= k;
                *y2 *= k;
            }
            if let Some(tx) = g.text_x.as_mut() {
                *tx *= k;
            }
            if let Some(ty) = g.text_y.as_mut() {
                *ty *= k;
            }
            for (x, y, _) in &mut g.text_lines {
                *x *= k;
                *y *= k;
            }
            for child in &mut g.children {
                scale_note_child(child, k);
            }
        }
    }
    for d in &mut o.decorations {
        for t in &mut d.texts {
            t.x *= k;
            t.y *= k;
        }
    }
    if let Some(w) = o.handwritten_warning.as_mut() {
        scale_entity_polygon(&mut w.polygon, k);
        w.text.x *= k;
        w.text.y *= k;
        if let Some(text_length) = w.text_length.as_mut() {
            *text_length = scale_number_token(text_length, k);
        }
    }
    if !o.defs_inner_xml.is_empty() {
        o.defs_inner_xml = scale_svg_numbers(&o.defs_inner_xml, k);
    }
    if let Some(g) = o.root_g_inner_xml.as_mut() {
        *g = scale_svg_numbers(g, k);
    }
    if let Some(open) = o.root_open_tag.as_mut() {
        // Re-scale the captured root `<svg …>` opening tag (no children, no
        // trailing '>'). Use the generic (non-root) attribute scaler so the
        // inverse pass divides every numeric uniformly — the special
        // floor-rounding of `viewBox`/`style` belongs only to the forward
        // final-SVG pass, never to oracle un-scaling.
        *open = scale_tag(open, k, false);
    }
    for rd in &mut o.region_dividers {
        rd.y *= k;
        rd.xml = scale_svg_numbers(&rd.xml, k);
    }
    for ap in &mut o.apoints {
        scale_apoint(ap, k);
    }
    for jb in &mut o.json_boxes {
        scale_json_box(jb, k);
    }
    for jc in &mut o.json_connectors {
        jc.curve = scale_svg_numbers(&jc.curve, k);
        if let Some(a) = jc.arrowhead.as_mut() {
            *a = scale_svg_numbers(a, k);
        }
        if let Some(d) = jc.dot.as_mut() {
            *d = scale_svg_numbers(d, k);
        }
    }
}

fn scale_note_child(child: &mut OracleNoteChild, k: f64) {
    match child {
        OracleNoteChild::Path(path) => {
            path.d = scale_path_d(&path.d, k);
            if let Some(style) = path.style.as_mut() {
                *style = scale_style(style, k);
            }
        }
        OracleNoteChild::Rect(rect) => {
            rect.x *= k;
            rect.y *= k;
            rect.width *= k;
            rect.height *= k;
            if let Some(style) = rect.style.as_mut() {
                *style = scale_style(style, k);
            }
        }
        OracleNoteChild::Text(text) => {
            text.x *= k;
            text.y *= k;
            text.font_size = scale_number_token(&text.font_size, k);
            if let Some(text_length) = text.text_length.as_mut() {
                *text_length = scale_number_token(text_length, k);
            }
        }
        OracleNoteChild::Ellipse(ellipse) => {
            ellipse.cx *= k;
            ellipse.cy *= k;
            ellipse.rx *= k;
            ellipse.ry *= k;
            if let Some(style) = ellipse.style.as_mut() {
                *style = scale_style(style, k);
            }
        }
        OracleNoteChild::Line(line) => {
            line.x1 *= k;
            line.x2 *= k;
            line.y1 *= k;
            line.y2 *= k;
            if let Some(style) = line.style.as_mut() {
                *style = scale_style(style, k);
            }
        }
    }
}

fn scale_entity(e: &mut EntityRect, k: f64) {
    e.x *= k;
    e.y *= k;
    e.width *= k;
    e.height *= k;
    if let Some(v) = e.icon_cx.as_mut() {
        *v *= k;
    }
    if let Some(d) = e.glyph_path_d.as_mut() {
        *d = scale_path_d(d, k);
    }
    if let Some(p) = e.body_polygon.as_mut() {
        scale_entity_polygon(p, k);
    }
    if let Some(p) = e.icon_polygon.as_mut() {
        scale_entity_polygon(p, k);
    }
    for p in &mut e.separator_paths {
        p.d = scale_path_d(&p.d, k);
        if let Some(s) = p.style.as_mut() {
            *s = scale_style(s, k);
        }
    }
    for p in &mut e.visibility_polygons {
        scale_entity_polygon(p, k);
    }
    if let Some(v) = e.name_text_x.as_mut() {
        *v *= k;
    }
    for v in &mut e.text_y_values {
        *v *= k;
    }
    for v in &mut e.text_x_values {
        *v *= k;
    }
    for v in &mut e.sep_y_values {
        *v *= k;
    }
    for (x1, x2, y1) in &mut e.sep_lines {
        *x1 *= k;
        *x2 *= k;
        *y1 *= k;
    }
    for v in &mut e.vis_icon_y_values {
        *v *= k;
    }
    if let Some(s) = e.body_style.as_mut() {
        *s = scale_style(s, k);
    }
    if let Some(s) = e.rect_style.as_mut() {
        *s = scale_style(s, k);
    }
    if let Some(s) = e.rect_rx.as_mut() {
        *s = scale_number_token(s, k);
    }
    if let Some(s) = e.rect_ry.as_mut() {
        *s = scale_number_token(s, k);
    }
    for r in &mut e.aux_rects {
        r.x *= k;
        r.y *= k;
        r.width *= k;
        r.height *= k;
        if let Some(s) = r.style.as_mut() {
            *s = scale_style(s, k);
        }
    }
    for l in &mut e.lines {
        l.x1 = scale_number_token(&l.x1, k);
        l.x2 = scale_number_token(&l.x2, k);
        l.y1 = scale_number_token(&l.y1, k);
        l.y2 = scale_number_token(&l.y2, k);
        if let Some(s) = l.style.as_mut() {
            *s = scale_style(s, k);
        }
    }
    for t in &mut e.texts {
        t.x *= k;
        t.y *= k;
    }
}

fn scale_entity_polygon(p: &mut crate::layout_oracle::EntityPolygon, k: f64) {
    p.points = scale_svg_numbers(&p.points, k);
    if let Some(s) = p.style.as_mut() {
        *s = scale_style(s, k);
    }
}

fn scale_edge(e: &mut OracleEdgePath, k: f64) {
    e.d = scale_path_d(&e.d, k);
    if let Some(p) = e.arrow_points.as_mut() {
        *p = scale_number_list(p, k);
    }
    if let Some(p) = e.second_arrow_points.as_mut() {
        *p = scale_number_list(p, k);
    }
    if let Some(s) = e.second_polygon_style.as_mut() {
        *s = scale_style(s, k);
    }
    if let Some(s) = e.path_style.as_mut() {
        *s = scale_style(s, k);
    }
    if let Some(s) = e.polygon_style.as_mut() {
        *s = scale_style(s, k);
    }
    if let Some((x, y, _)) = e.label.as_mut() {
        *x *= k;
        *y *= k;
    }
    for (x, y, _) in &mut e.labels {
        *x *= k;
        *y *= k;
    }
    for (d, style) in &mut e.extra_paths {
        *d = scale_path_d(d, k);
        if let Some(s) = style.as_mut() {
            *s = scale_style(s, k);
        }
    }
    for cm in &mut e.crow_lines {
        scale_crow(cm, k);
    }
    for dec in &mut e.decorations {
        scale_decoration(dec, k);
    }
}

fn scale_crow(cm: &mut CrowMark, k: f64) {
    match cm {
        CrowMark::Line(style, x1, y1, x2, y2) => {
            *style = scale_style(style, k);
            *x1 *= k;
            *y1 *= k;
            *x2 *= k;
            *y2 *= k;
        }
        CrowMark::Ellipse(style, cx, cy, rx, ry, _fill) => {
            *style = scale_style(style, k);
            *cx *= k;
            *cy *= k;
            *rx *= k;
            *ry *= k;
        }
    }
}

fn scale_decoration(dec: &mut EdgeDecoration, k: f64) {
    match dec {
        EdgeDecoration::Path { d, style, .. } => {
            *d = scale_path_d(d, k);
            if let Some(s) = style.as_mut() {
                *s = scale_style(s, k);
            }
        }
        EdgeDecoration::Ellipse {
            cx,
            cy,
            rx,
            ry,
            style,
            ..
        } => {
            *cx *= k;
            *cy *= k;
            *rx *= k;
            *ry *= k;
            if let Some(s) = style.as_mut() {
                *s = scale_style(s, k);
            }
        }
        EdgeDecoration::Line {
            x1,
            y1,
            x2,
            y2,
            style,
        } => {
            *x1 *= k;
            *y1 *= k;
            *x2 *= k;
            *y2 *= k;
            if let Some(s) = style.as_mut() {
                *s = scale_style(s, k);
            }
        }
        EdgeDecoration::Text { x, y, .. } => {
            *x *= k;
            *y *= k;
        }
    }
}

fn scale_apoint(ap: &mut ApointMark, k: f64) {
    ap.cx *= k;
    ap.cy *= k;
    ap.rx *= k;
    ap.ry *= k;
    ap.style = scale_style(&ap.style, k);
}

fn scale_json_box(jb: &mut JsonBox, k: f64) {
    jb.x *= k;
    jb.y *= k;
    jb.width *= k;
    jb.height *= k;
    for v in &mut jb.text_ys {
        *v *= k;
    }
    for (y1, y2) in &mut jb.line_ys {
        *y1 = scale_number_token(y1, k);
        *y2 = scale_number_token(y2, k);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_svg_special_rounding() {
        let svg = r#"<svg width="142px" height="194px" viewBox="0 0 142 194" style="width:142px;height:194px;background:#FFFFFF;" data-diagram-type="CLASS"></svg>"#;
        let out = scale_svg_numbers(svg, 1.5625);
        assert!(out.contains(r#"width="221.875px""#), "{out}");
        assert!(out.contains(r#"height="303.125px""#), "{out}");
        assert!(out.contains(r#"viewBox="0 0 221 303""#), "{out}");
        assert!(
            out.contains(r#"style="width:221px;height:303px;background:#FFFFFF;""#),
            "{out}"
        );
        assert!(out.contains(r#"data-diagram-type="CLASS""#), "{out}");
    }

    #[test]
    fn element_attrs_and_style() {
        let svg = r##"<rect x="7" y="7" width="120.4453" height="64.4883" rx="2.5" ry="2.5" fill="#F1F1F1" style="stroke:#181818;stroke-width:0.5;"/>"##;
        let out = scale_svg_numbers(svg, 1.5625);
        assert!(out.contains(r#"x="10.9375""#), "{out}");
        assert!(out.contains(r#"rx="3.9063""#), "{out}");
        assert!(out.contains(r#"stroke-width:0.7813"#), "{out}");
        assert!(out.contains(r##"fill="#F1F1F1""##), "{out}");
        assert!(out.contains(r##"stroke:#181818"##), "{out}");
    }

    #[test]
    fn font_size_and_textlength() {
        let svg = r#"<text font-size="14" textLength="9.6592" x="78.6431" font-family="sans-serif">A</text>"#;
        let out = scale_svg_numbers(svg, 1.5625);
        assert!(out.contains(r#"font-size="21.875""#), "{out}");
        assert!(out.contains(r#"textLength="15.0925""#), "{out}");
        assert!(out.contains(r#"font-family="sans-serif""#), "{out}");
    }

    #[test]
    fn path_and_polygon() {
        let svg = r#"<path d="M67.22,71.77 C67.22,90.52 67.22,107.92 67.22,125.05"/><polygon points="67.22,131.05,71.22,122.05,67.22,126.05"/>"#;
        let out = scale_svg_numbers(svg, 1.5625);
        // 67.22*1.5625 = 105.03125 -> 105.0313 (HALF_UP at 4dp)
        assert!(out.contains("M105.0313,112.1406"), "{out}");
        assert!(out.contains("points=\"105.0313,204.7656"), "{out}");
    }

    #[test]
    fn factor_inert_when_absent() {
        let meta = DiagramMeta::default();
        assert_eq!(factor_from_meta(&meta), 1.0);
        assert_eq!(scale_svg_numbers("<svg></svg>", 1.0), "<svg></svg>");
    }

    #[test]
    fn factor_from_dpi_and_scale() {
        let mut meta = DiagramMeta {
            source: Some("@startuml\nscale 3\nclass Foo\n@enduml\n".to_string()),
            ..Default::default()
        };
        assert_eq!(factor_from_meta(&meta), 3.0);
        meta.source = Some("@startuml\nskinparam dpi 150\n@enduml\n".to_string());
        meta.skinparams.push(rustuml_parser::diagram::SkinParam {
            key: "dpi".to_string(),
            value: "150".to_string(),
        });
        assert_eq!(factor_from_meta(&meta), 1.5625);
    }

    #[test]
    fn factor_ignores_fit_to_box_forms() {
        let mk = |s: &str| DiagramMeta {
            source: Some(format!("@startuml\n{s}\nclass Foo\n@enduml\n")),
            ..Default::default()
        };
        assert_eq!(factor_from_meta(&mk("scale max 100 width")), 1.0);
        assert_eq!(factor_from_meta(&mk("scale 100*100")), 1.0);
        assert_eq!(factor_from_meta(&mk("scale 200x100")), 1.0);
    }

    #[test]
    fn round_trip_unscale_rescale_is_stable() {
        // Coordinate captured at golden scale, un-scaled then re-scaled, must
        // land on the same 4-dp string.
        let k = 1.5625;
        let golden = 90.8485_f64;
        let base = golden / k;
        let back = fmt_coord(base * k);
        assert_eq!(back, "90.8485");
    }
}

#[cfg(test)]
mod fp_tests {
    use super::*;
    use crate::layout_oracle::{EntityRect, OracleLayout};
    #[test]
    fn glyph_unscale_keeps_full_precision() {
        let mut o = OracleLayout::default();
        let mut e = EntityRect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            icon_cx: None,
            glyph_path_d: Some("M48.9463,58.2861".to_string()),
            body_polygon: None,
            icon_polygon: None,
            separator_paths: vec![],
            visibility_polygons: vec![],
            name_text_x: None,
            text_y_values: vec![],
            text_x_values: vec![],
            sep_y_values: vec![],
            sep_lines: vec![],
            vis_icon_y_values: vec![],
            fill: None,
            body_style: None,
            rect_style: None,
            rect_rx: None,
            rect_ry: None,
            rect_filter: None,
            entity_id: None,
            source_line: None,
            aux_rects: vec![],
            lines: vec![],
            texts: vec![],
        };
        e.glyph_path_d = Some("M48.9463,58.2861".to_string());
        o.entities.insert("Foo".to_string(), e);
        scale_oracle_layout(&mut o, 0.5);
        let g = o.entities["Foo"].glyph_path_d.as_deref().unwrap();
        assert_eq!(g, "M24.47315,29.14305", "got {g}");
    }
}
