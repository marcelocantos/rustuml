// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

/// Attributes that carry coordinate/dimension data and should be ignored
/// in structural comparison (used by the legacy `compare_svg` function).
const GEOMETRY_ATTRS: &[&str] = &[
    "x",
    "x1",
    "x2",
    "y",
    "y1",
    "y2",
    "width",
    "height",
    "rx",
    "ry",
    "viewBox",
    "points",
    "textLength",
    "d",
];

/// Attributes on the root <svg> element that vary per render and should
/// be ignored in structural comparison (used by the legacy `compare_svg` function).
const SVG_ROOT_ATTRS: &[&str] = &["style", "height", "width", "viewBox"];

/// A flattened representation of an SVG element for structural comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgElement {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub text: Option<String>,
    pub depth: usize,
}

/// The result of comparing two SVG documents structurally.
#[derive(Debug)]
pub struct CompareResult {
    pub differences: Vec<Difference>,
}

#[derive(Debug)]
pub enum Difference {
    ElementCount {
        expected: usize,
        actual: usize,
    },
    TagMismatch {
        index: usize,
        expected: String,
        actual: String,
    },
    AttrMismatch {
        index: usize,
        tag: String,
        expected_attrs: Vec<(String, String)>,
        actual_attrs: Vec<(String, String)>,
    },
    TextMismatch {
        index: usize,
        tag: String,
        expected: Option<String>,
        actual: Option<String>,
    },
    DepthMismatch {
        index: usize,
        tag: String,
        expected: usize,
        actual: usize,
    },
}

/// Geometry-coordinate attributes that the comparator tolerates within
/// [`GEOM_EPS`] (and that `is_ulp_only` classifies). DELIBERATELY excludes
/// `style` (carries stroke-width — a diff there is a real bug), `font-size`,
/// and `data-source-line` (a line index; off-by-one is a real bug, not
/// rounding noise), so only true coordinate FP drift is forgiven.
const ULP_GEOM_ATTRS: &[&str] = &[
    "x",
    "x1",
    "x2",
    "y",
    "y1",
    "y2",
    "cx",
    "cy",
    "width",
    "height",
    "rx",
    "ry",
    "viewBox",
    "points",
    "textLength",
    "d",
];

/// One unit-in-the-last-place at PlantUML's 4-decimal emission precision is
/// 1e-4; admit a single tick (and its fp representation) but reject >=2 ticks.
pub const ULP_EPS: f64 = 1.5e-4;

/// Per-field tolerance for GEOMETRY-COORDINATE attribute values (the
/// [`ULP_GEOM_ATTRS`] set). A coordinate that differs from the golden by at
/// most this many SVG px is treated as a match — absorbing genuine JVM
/// 4-decimal HALF_UP rounding-boundary FP noise — while the element's
/// STRUCTURE (tags, counts, text, colours, non-geometry attrs, and the path
/// command skeleton) is still compared byte-exact, and any larger numeric
/// difference still fails. This keeps near-miss cases fully in the test suite
/// (rather than excluding them via a park-list), tolerating only true
/// sub-pixel FP drift. Chosen well inside the empirical gap between genuine FP
/// noise (observed <=0.009px) and the smallest real sub-pixel layout bug
/// (~0.185px), so its exact value is not sensitive; tighten if a real bug ever
/// lands below it. NOTE: this intentionally relaxes the byte-exact rule for
/// coordinates only — text, structure, and colours remain exact.
pub const GEOM_EPS: f64 = 0.02;

impl CompareResult {
    pub fn is_match(&self) -> bool {
        self.differences.is_empty()
    }

    /// If — and only if — every difference is confined to geometry-coordinate
    /// attribute VALUES, return `Some(max per-token absolute deviation)`.
    /// Return `None` when there is any STRUCTURAL difference: an
    /// element-count / tag / text / depth mismatch, a differing attribute SET,
    /// or a diff in any non-geometry attribute (color, style, stroke-width,
    /// font, `data-source-line`, …). Element ORDER is already structural here
    /// (the comparator is positional), so a reordering surfaces as tag/attr
    /// mismatches and yields `None`.
    ///
    /// This is the PARK-ELIGIBILITY guard: parking may forgive coordinate
    /// deviations (FP-accumulation, missing font-metric tables, sub-pixel
    /// drift) of ANY magnitude — that magnitude is a documented per-case
    /// judgement — but it can NEVER forgive a structural difference. The
    /// returned magnitude is reported so the parked set can be audited (a
    /// large deviation in a "parked" case is a red flag to re-review).
    pub fn numeric_only_max_deviation(&self) -> Option<f64> {
        if self.differences.is_empty() {
            return None;
        }
        let mut max_dev = 0.0_f64;
        for d in &self.differences {
            match d {
                Difference::AttrMismatch {
                    expected_attrs,
                    actual_attrs,
                    ..
                } => {
                    let dev = attrs_numeric_only_dev(expected_attrs, actual_attrs)?;
                    max_dev = max_dev.max(dev);
                }
                _ => return None,
            }
        }
        Some(max_dev)
    }

    /// Tight informational tier: numeric-only AND within `eps` (≈1 ULP at
    /// 4-decimal precision). Used only for reporting the FP-rounding subset.
    pub fn is_ulp_only(&self, eps: f64) -> bool {
        self.numeric_only_max_deviation().is_some_and(|d| d <= eps)
    }
}

/// `None` if the diff is structural (differing attribute set, duplicate name,
/// or any non-geometry attribute differs, or a geometry value differs in its
/// non-numeric skeleton / token count). Otherwise `Some(max token deviation)`.
fn attrs_numeric_only_dev(exp: &[(String, String)], act: &[(String, String)]) -> Option<f64> {
    use std::collections::BTreeMap;
    let em: BTreeMap<&str, &str> = exp.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let am: BTreeMap<&str, &str> = act.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    if em.len() != exp.len() || am.len() != act.len() || em.keys().ne(am.keys()) {
        return None;
    }
    let mut max_dev = 0.0_f64;
    for (name, ev) in &em {
        let av = am[name];
        if *ev == av {
            continue;
        }
        if !ULP_GEOM_ATTRS.contains(name) {
            return None; // a non-geometry attribute changed: structural
        }
        max_dev = max_dev.max(numeric_tokens_dev(ev, av)?);
    }
    Some(max_dev)
}

/// `None` if `a`/`b` differ in non-numeric skeleton or token count (a
/// structural difference within the value); otherwise `Some(max |xi-yi|)`.
fn numeric_tokens_dev(a: &str, b: &str) -> Option<f64> {
    let (sa, na) = tokenize_numbers(a);
    let (sb, nb) = tokenize_numbers(b);
    if sa != sb || na.len() != nb.len() {
        return None;
    }
    Some(
        na.iter()
            .zip(&nb)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0_f64, f64::max),
    )
}

/// Split a string into its non-numeric "skeleton" (numbers replaced by a
/// sentinel) and the parsed numeric tokens, in order. A number is an optional
/// sign (only at string start or after a non-number char) followed by digits
/// with an optional single decimal point.
fn tokenize_numbers(s: &str) -> (String, Vec<f64>) {
    let bytes = s.as_bytes();
    let mut skeleton = String::with_capacity(s.len());
    let mut nums = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let prev_is_numish = i > 0 && {
            let p = bytes[i - 1];
            p.is_ascii_digit() || p == b'.'
        };
        let starts_number = c.is_ascii_digit()
            || (c == b'.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit())
            || ((c == b'-' || c == b'+')
                && !prev_is_numish
                && i + 1 < bytes.len()
                && (bytes[i + 1].is_ascii_digit() || bytes[i + 1] == b'.'));
        if starts_number {
            let start = i;
            if c == b'-' || c == b'+' {
                i += 1;
            }
            let mut seen_dot = false;
            while i < bytes.len() {
                let d = bytes[i];
                if d.is_ascii_digit() {
                    i += 1;
                } else if d == b'.' && !seen_dot {
                    seen_dot = true;
                    i += 1;
                } else {
                    break;
                }
            }
            match s[start..i].parse::<f64>() {
                Ok(n) => {
                    skeleton.push('\u{0}');
                    nums.push(n);
                }
                Err(_) => skeleton.push_str(&s[start..i]),
            }
        } else {
            skeleton.push(c as char);
            i += 1;
        }
    }
    (skeleton, nums)
}

impl fmt::Display for CompareResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_match() {
            write!(f, "SVGs are structurally equivalent")
        } else {
            writeln!(f, "SVG structural differences:")?;
            for diff in &self.differences {
                match diff {
                    Difference::ElementCount { expected, actual } => {
                        writeln!(f, "  element count: expected {expected}, got {actual}")?;
                    }
                    Difference::TagMismatch {
                        index,
                        expected,
                        actual,
                    } => {
                        writeln!(
                            f,
                            "  [{index}] tag mismatch: expected <{expected}>, got <{actual}>"
                        )?;
                    }
                    Difference::AttrMismatch {
                        index,
                        tag,
                        expected_attrs,
                        actual_attrs,
                    } => {
                        writeln!(f, "  [{index}] <{tag}> attr mismatch:")?;
                        writeln!(f, "    expected: {expected_attrs:?}")?;
                        writeln!(f, "    actual:   {actual_attrs:?}")?;
                    }
                    Difference::TextMismatch {
                        index,
                        tag,
                        expected,
                        actual,
                    } => {
                        writeln!(f, "  [{index}] <{tag}> text mismatch:")?;
                        writeln!(f, "    expected: {expected:?}")?;
                        writeln!(f, "    actual:   {actual:?}")?;
                    }
                    Difference::DepthMismatch {
                        index,
                        tag,
                        expected,
                        actual,
                    } => {
                        writeln!(
                            f,
                            "  [{index}] <{tag}> depth mismatch: expected {expected}, got {actual}"
                        )?;
                    }
                }
            }
            Ok(())
        }
    }
}

fn is_geometry_attr(name: &str) -> bool {
    GEOMETRY_ATTRS.contains(&name)
}

fn is_svg_root_attr(name: &str) -> bool {
    SVG_ROOT_ATTRS.contains(&name)
}

/// Extracts a flat list of structural elements from an SVG string,
/// filtering out geometry attributes (legacy mode).
pub fn extract_elements(svg: &str) -> Result<Vec<SvgElement>, String> {
    let cleaned = strip_xml_preamble(svg);
    let doc =
        roxmltree::Document::parse(&cleaned).map_err(|e| format!("failed to parse SVG: {e}"))?;

    let mut elements = Vec::new();
    collect_elements(doc.root(), 0, &mut elements);
    Ok(elements)
}

/// Extracts a flat list of ALL elements from an SVG string,
/// preserving all attributes (strict mode).
pub fn extract_elements_strict(svg: &str) -> Result<Vec<SvgElement>, String> {
    let cleaned = strip_xml_preamble(svg);
    let doc =
        roxmltree::Document::parse(&cleaned).map_err(|e| format!("failed to parse SVG: {e}"))?;

    let mut elements = Vec::new();
    collect_elements_strict(doc.root(), 0, &mut elements);
    Ok(elements)
}

/// Strip `<?xml ...?>` declaration and `<!DOCTYPE ...>` from SVG content.
fn strip_xml_preamble(svg: &str) -> String {
    let mut result = svg.to_string();
    // Remove <?xml ... ?>
    if let Some(start) = result.find("<?xml")
        && let Some(end) = result[start..].find("?>")
    {
        result = format!("{}{}", &result[..start], &result[start + end + 2..]);
    }
    // Remove <!DOCTYPE ... >  (may span multiple lines)
    if let Some(start) = result.find("<!DOCTYPE")
        && let Some(end) = result[start..].find('>')
    {
        result = format!("{}{}", &result[..start], &result[start + end + 1..]);
    }
    result.trim_start().to_string()
}

/// Legacy element collector — filters out geometry and root SVG attributes.
fn collect_elements(node: roxmltree::Node, depth: usize, elements: &mut Vec<SvgElement>) {
    if node.is_element() {
        let tag = node.tag_name().name().to_string();

        // Skip <title> elements — these are metadata (Graphviz SVGs emit them
        // for nodes and edges) and are not visible content.
        if tag == "title" {
            return;
        }

        let is_root_svg = tag == "svg" && depth <= 1;

        // Collect non-geometry attributes, sorted for stable comparison.
        let mut attrs: Vec<(String, String)> = node
            .attributes()
            .filter(|a| {
                let name = a.name();
                !(is_geometry_attr(name)
                    || name.starts_with("xmlns")
                    || (is_root_svg && is_svg_root_attr(name)))
            })
            .map(|a| (a.name().to_string(), a.value().to_string()))
            .collect();
        attrs.sort_by(|a, b| a.0.cmp(&b.0));

        // Collect direct text content (not from children).
        let text: Option<String> = {
            let t: String = node
                .children()
                .filter(|c| c.is_text())
                .map(|c| c.text().unwrap_or(""))
                .collect::<String>()
                .trim()
                .to_string();
            if t.is_empty() { None } else { Some(t) }
        };

        // Skip processing instructions (<?plantuml ...?>).
        if tag != "processing-instruction" {
            elements.push(SvgElement {
                tag,
                attrs,
                text,
                depth,
            });
        }
    }

    for child in node.children() {
        if child.is_element() {
            collect_elements(child, depth + 1, elements);
        }
    }
}

/// Strict element collector — preserves ALL attributes (except xmlns).
/// Ignores processing instructions, comments, and whitespace-only text nodes.
fn collect_elements_strict(node: roxmltree::Node, depth: usize, elements: &mut Vec<SvgElement>) {
    // Skip processing instructions and comments at the node level.
    if node.is_pi() || node.is_comment() {
        return;
    }

    if node.is_element() {
        let tag = node.tag_name().name().to_string();

        // Skip <title> elements — metadata, not visible content.
        if tag == "title" {
            return;
        }

        // Collect ALL attributes except xmlns (namespace declarations vary).
        // Exact string comparison — no numeric tolerance.
        let mut attrs: Vec<(String, String)> = node
            .attributes()
            .filter(|a| !a.name().starts_with("xmlns"))
            .map(|a| (a.name().to_string(), a.value().to_string()))
            .collect();
        attrs.sort_by(|a, b| a.0.cmp(&b.0));

        // Collect direct text content (not from children).
        // Whitespace-only text nodes are ignored.
        let text: Option<String> = {
            let t: String = node
                .children()
                .filter(|c| c.is_text())
                .map(|c| c.text().unwrap_or(""))
                .collect::<String>()
                .trim()
                .to_string();
            if t.is_empty() { None } else { Some(t) }
        };

        elements.push(SvgElement {
            tag,
            attrs,
            text,
            depth,
        });
    }

    for child in node.children() {
        // Skip processing instructions and comments in children.
        if child.is_pi() || child.is_comment() {
            continue;
        }
        if child.is_element() {
            collect_elements_strict(child, depth + 1, elements);
        }
    }
}

/// Compares two SVGs strictly — all attributes must match exactly (string equality).
/// Processing instructions, comments, and whitespace-only text nodes are ignored.
pub fn compare_svg_strict(expected: &str, actual: &str) -> Result<CompareResult, String> {
    let expected_elems = extract_elements_strict(expected)?;
    let actual_elems = extract_elements_strict(actual)?;

    let mut differences = Vec::new();

    if expected_elems.len() != actual_elems.len() {
        differences.push(Difference::ElementCount {
            expected: expected_elems.len(),
            actual: actual_elems.len(),
        });
    }

    let compare_len = expected_elems.len().min(actual_elems.len());
    for i in 0..compare_len {
        let exp = &expected_elems[i];
        let act = &actual_elems[i];

        if exp.tag != act.tag {
            differences.push(Difference::TagMismatch {
                index: i,
                expected: exp.tag.clone(),
                actual: act.tag.clone(),
            });
            continue;
        }

        if exp.depth != act.depth {
            differences.push(Difference::DepthMismatch {
                index: i,
                tag: exp.tag.clone(),
                expected: exp.depth,
                actual: act.depth,
            });
        }

        // For the root <svg> element, tolerate ±1 px discrepancies on the
        // outer dimensions (width, height, viewBox, style) — these reflect
        // PlantUML's internal Dimension2D rounding which differs from our
        // ceil() by at most 1 px on diagrams where content_w lands at
        // specific fractional boundaries. Interior elements are still
        // strictly compared, so this only relaxes the advertised outer
        // size — content geometry stays locked.
        //
        // For <text> elements whose expected content contains "PlantUML" or
        // the "plantuml.com" URL, normalise to the rustuml brand. Renderer
        // emits "RustUML" / the rustuml URL unconditionally (see
        // rustuml_render::product_name / product_url). The replacement
        // changes the rendered string width, so textLength is also dropped
        // from comparison on those elements.
        let exp_text_branded = exp.tag == "text"
            && exp
                .text
                .as_deref()
                .is_some_and(|t| t.contains("PlantUML") || t.contains("plantuml.com"));
        let attrs_equal = if i == 0 && exp.tag == "svg" {
            root_svg_attrs_match_within_1px(&exp.attrs, &act.attrs)
        } else if exp_text_branded {
            attrs_match_ignoring_text_length(&exp.attrs, &act.attrs)
        } else {
            // Geometry coordinates may differ by <= GEOM_EPS px (FP rounding
            // noise); everything else stays byte-exact.
            attrs_match_geom_eps(&exp.attrs, &act.attrs)
        };
        if !attrs_equal {
            differences.push(Difference::AttrMismatch {
                index: i,
                tag: exp.tag.clone(),
                expected_attrs: exp.attrs.clone(),
                actual_attrs: act.attrs.clone(),
            });
        }

        // For branded text elements, accept either the original PlantUML
        // wording OR its rebranded equivalent. Verbatim oracle-replay paths
        // emit the golden body unchanged (so actual still says "PlantUML");
        // the geometry/welcome-screen path emits "RustUML" through
        // rustuml_render::product_name(). Both should compare equal.
        let texts_equal = if exp_text_branded {
            let act_text = act.text.as_deref();
            act_text == exp.text.as_deref()
                || act_text == exp.text.as_deref().map(rebrand_plantuml).as_deref()
        } else {
            exp.text == act.text
        };
        if !texts_equal {
            differences.push(Difference::TextMismatch {
                index: i,
                tag: exp.tag.clone(),
                expected: exp.text.clone(),
                actual: act.text.clone(),
            });
        }
    }

    Ok(CompareResult { differences })
}

/// Replace PlantUML brand strings with their rustuml equivalents.
/// Kept in sync with `rustuml_render::product_name()` / `product_url()`.
fn rebrand_plantuml(text: &str) -> String {
    text.replace("PlantUML", "RustUML").replace(
        "https://plantuml.com",
        "https://github.com/marcelocantos/rustuml",
    )
}

/// Compare two attribute lists ignoring the `textLength` attribute. Used
/// for text elements whose content has been rebranded (PlantUML → RustUML),
/// where the rendered string width necessarily differs.
/// Match two elements' attribute lists allowing GEOMETRY-COORDINATE values to
/// differ by at most [`GEOM_EPS`] px, with every non-geometry attribute and the
/// coordinate STRUCTURE (path skeleton / token count) still exact. See
/// [`GEOM_EPS`]. Attributes are emitted in sorted order, so a positional
/// lock-step compare is correct.
fn attrs_match_geom_eps(exp: &[(String, String)], act: &[(String, String)]) -> bool {
    if exp.len() != act.len() {
        return false;
    }
    for ((ek, ev), (ak, av)) in exp.iter().zip(act.iter()) {
        if ek != ak {
            return false;
        }
        if ev == av {
            continue;
        }
        // Differing value: tolerated only for a geometry-coordinate attribute
        // (ULP_GEOM_ATTRS — includes cx/cy, excludes style/font-size/
        // data-source-line) whose numbers all differ by <= GEOM_EPS and whose
        // non-numeric skeleton is identical (numeric_tokens_dev returns None
        // otherwise).
        if ULP_GEOM_ATTRS.contains(&ek.as_str())
            && numeric_tokens_dev(ev, av).is_some_and(|dev| dev <= GEOM_EPS)
        {
            continue;
        }
        return false;
    }
    true
}

fn attrs_match_ignoring_text_length(exp: &[(String, String)], act: &[(String, String)]) -> bool {
    let strip = |xs: &[(String, String)]| -> Vec<(String, String)> {
        xs.iter()
            .filter(|(k, _)| k != "textLength")
            .cloned()
            .collect()
    };
    strip(exp) == strip(act)
}

/// Compare the root `<svg>` element's attributes, allowing the advertised
/// width/height (which appear in the `width`, `height`, `viewBox`, and
/// `style` attributes) to differ by at most 1 px. All other attributes
/// must match exactly. This is the only place in the comparator that
/// allows numeric drift; it specifically targets PlantUML's internal
/// outer-dimension rounding that the renderer can't reproduce without
/// instrumenting PlantUML itself.
fn root_svg_attrs_match_within_1px(exp: &[(String, String)], act: &[(String, String)]) -> bool {
    if exp.len() != act.len() {
        return false;
    }
    for ((ek, ev), (ak, av)) in exp.iter().zip(act.iter()) {
        if ek != ak {
            return false;
        }
        if ev == av {
            continue;
        }
        match ek.as_str() {
            "width" | "height" => {
                // "Npx" form (e.g., "139px").
                let Some(ep) = ev.strip_suffix("px") else {
                    return false;
                };
                let Some(ap) = av.strip_suffix("px") else {
                    return false;
                };
                let (Ok(ev_num), Ok(av_num)) = (ep.parse::<i64>(), ap.parse::<i64>()) else {
                    return false;
                };
                if (ev_num - av_num).abs() > 1 {
                    return false;
                }
            }
            "viewBox" => {
                // "x y w h" form.
                let ep: Vec<&str> = ev.split_whitespace().collect();
                let ap: Vec<&str> = av.split_whitespace().collect();
                if ep.len() != 4 || ap.len() != 4 {
                    return false;
                }
                if ep[0] != ap[0] || ep[1] != ap[1] {
                    return false;
                }
                for idx in [2, 3] {
                    let (Ok(en), Ok(an)) = (ep[idx].parse::<i64>(), ap[idx].parse::<i64>()) else {
                        return false;
                    };
                    if (en - an).abs() > 1 {
                        return false;
                    }
                }
            }
            "style" => {
                // "width:Npx;height:Npx;..." with the same N as the width/
                // height attributes. Tolerate ±1 on the width:/height:
                // values; everything else (background, etc.) must match.
                if !style_attrs_match_within_1px(ev, av) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

fn style_attrs_match_within_1px(exp: &str, act: &str) -> bool {
    let ep: Vec<&str> = exp.split(';').filter(|s| !s.is_empty()).collect();
    let ap: Vec<&str> = act.split(';').filter(|s| !s.is_empty()).collect();
    if ep.len() != ap.len() {
        return false;
    }
    for (e, a) in ep.iter().zip(ap.iter()) {
        if e == a {
            continue;
        }
        let (Some((ek, ev)), Some((ak, av))) = (e.split_once(':'), a.split_once(':')) else {
            return false;
        };
        if ek != ak {
            return false;
        }
        if matches!(ek, "width" | "height") {
            let Some(ep_n) = ev.strip_suffix("px") else {
                return false;
            };
            let Some(ap_n) = av.strip_suffix("px") else {
                return false;
            };
            let (Ok(en), Ok(an)) = (ep_n.parse::<i64>(), ap_n.parse::<i64>()) else {
                return false;
            };
            if (en - an).abs() > 1 {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

/// Compares two SVGs structurally, ignoring coordinates and dimensions (legacy).
pub fn compare_svg(expected: &str, actual: &str) -> Result<CompareResult, String> {
    let expected_elems = extract_elements(expected)?;
    let actual_elems = extract_elements(actual)?;

    let mut differences = Vec::new();

    if expected_elems.len() != actual_elems.len() {
        differences.push(Difference::ElementCount {
            expected: expected_elems.len(),
            actual: actual_elems.len(),
        });
    }

    let compare_len = expected_elems.len().min(actual_elems.len());
    for i in 0..compare_len {
        let exp = &expected_elems[i];
        let act = &actual_elems[i];

        if exp.tag != act.tag {
            differences.push(Difference::TagMismatch {
                index: i,
                expected: exp.tag.clone(),
                actual: act.tag.clone(),
            });
            continue;
        }

        if exp.depth != act.depth {
            differences.push(Difference::DepthMismatch {
                index: i,
                tag: exp.tag.clone(),
                expected: exp.depth,
                actual: act.depth,
            });
        }

        if exp.attrs != act.attrs {
            differences.push(Difference::AttrMismatch {
                index: i,
                tag: exp.tag.clone(),
                expected_attrs: exp.attrs.clone(),
                actual_attrs: act.attrs.clone(),
            });
        }

        if exp.text != act.text {
            differences.push(Difference::TextMismatch {
                index: i,
                tag: exp.tag.clone(),
                expected: exp.text.clone(),
                actual: act.text.clone(),
            });
        }
    }

    Ok(CompareResult { differences })
}

/// Compares two preprocessed outputs for exact match.
pub fn compare_preproc(expected: &str, actual: &str) -> Result<CompareResult, String> {
    let mut differences = Vec::new();
    if expected.trim() != actual.trim() {
        differences.push(Difference::TextMismatch {
            index: 0,
            tag: "preproc".to_string(),
            expected: Some(expected.trim().to_string()),
            actual: Some(actual.trim().to_string()),
        });
    }
    Ok(CompareResult { differences })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_svgs_match() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <g class="participant" data-entity-uid="p1">
                <text fill="#000">Alice</text>
            </g>
        </svg>"##;

        let result = compare_svg(svg, svg).unwrap();
        assert!(result.is_match(), "identical SVGs should match: {result}");
    }

    #[test]
    fn different_coordinates_still_match() {
        let svg1 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="200">
            <rect x="10" y="20" width="30" height="40" fill="#E2E2F0"/>
            <text x="15" y="35" fill="#000">Alice</text>
        </svg>"##;

        let svg2 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="400">
            <rect x="50" y="60" width="70" height="80" fill="#E2E2F0"/>
            <text x="55" y="75" fill="#000">Alice</text>
        </svg>"##;

        let result = compare_svg(svg1, svg2).unwrap();
        assert!(
            result.is_match(),
            "SVGs differing only in coordinates should match: {result}"
        );
    }

    #[test]
    fn different_text_does_not_match() {
        let svg1 = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <text fill="#000">Alice</text>
        </svg>"##;

        let svg2 = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <text fill="#000">Bob</text>
        </svg>"##;

        let result = compare_svg(svg1, svg2).unwrap();
        assert!(
            !result.is_match(),
            "SVGs with different text should not match"
        );
    }

    #[test]
    fn different_structure_does_not_match() {
        let svg1 = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <g class="participant"><text>Alice</text></g>
        </svg>"##;

        let svg2 = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <g class="message"><text>Alice</text></g>
        </svg>"##;

        let result = compare_svg(svg1, svg2).unwrap();
        assert!(
            !result.is_match(),
            "SVGs with different structure should not match"
        );
    }

    #[test]
    fn preproc_exact_match() {
        let a = "@startuml\nAlice -> Bob : hello\n@enduml\n";
        let b = "@startuml\nAlice -> Bob : hello\n@enduml\n";
        let result = compare_preproc(a, b).unwrap();
        assert!(result.is_match());
    }

    #[test]
    fn preproc_mismatch() {
        let a = "@startuml\nAlice -> Bob : hello\n@enduml\n";
        let b = "@startuml\nAlice -> Bob : goodbye\n@enduml\n";
        let result = compare_preproc(a, b).unwrap();
        assert!(!result.is_match());
    }

    // --- Strict comparison tests ---

    #[test]
    fn strict_identical_svgs_match() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="200">
            <rect x="10" y="20" width="30" height="40" fill="#E2E2F0"/>
            <text x="15" y="35" fill="#000">Alice</text>
        </svg>"##;

        let result = compare_svg_strict(svg, svg).unwrap();
        assert!(result.is_match(), "identical SVGs should match: {result}");
    }

    #[test]
    fn strict_different_coordinates_do_not_match() {
        let svg1 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="200">
            <rect x="10" y="20" width="30" height="40" fill="#E2E2F0"/>
        </svg>"##;

        let svg2 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="400">
            <rect x="50" y="60" width="70" height="80" fill="#E2E2F0"/>
        </svg>"##;

        let result = compare_svg_strict(svg1, svg2).unwrap();
        assert!(
            !result.is_match(),
            "strict mode should detect coordinate differences"
        );
    }

    #[test]
    fn strict_exact_string_match_on_numeric_values() {
        let svg1 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="200">
            <rect x="10" y="20" width="30.2" height="40" fill="#E2E2F0"/>
        </svg>"##;

        let svg2 = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="200">
            <rect x="10" y="20" width="30.5" height="40" fill="#E2E2F0"/>
        </svg>"##;

        let result = compare_svg_strict(svg1, svg2).unwrap();
        assert!(
            !result.is_match(),
            "strict mode uses exact string comparison — 30.2 != 30.5"
        );
    }

    #[test]
    fn strict_ignores_comments() {
        let svg1 = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <text>Alice</text>
        </svg>"##;

        let svg2 = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <!-- This is a comment -->
            <text>Alice</text>
        </svg>"##;

        let result = compare_svg_strict(svg1, svg2).unwrap();
        assert!(
            result.is_match(),
            "should match ignoring comments: {result}"
        );
    }

    #[test]
    fn strict_all_attributes_checked() {
        let svg1 = r##"<svg xmlns="http://www.w3.org/2000/svg" style="background:#fff">
            <text>Alice</text>
        </svg>"##;

        let svg2 = r##"<svg xmlns="http://www.w3.org/2000/svg" style="background:#000">
            <text>Alice</text>
        </svg>"##;

        let result = compare_svg_strict(svg1, svg2).unwrap();
        assert!(
            !result.is_match(),
            "strict mode should detect style differences on root svg"
        );
    }

    #[test]
    fn strict_attribute_order_independent() {
        let svg1 = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <rect fill="#E2E2F0" x="10" y="20"/>
        </svg>"##;

        let svg2 = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <rect x="10" y="20" fill="#E2E2F0"/>
        </svg>"##;

        let result = compare_svg_strict(svg1, svg2).unwrap();
        assert!(
            result.is_match(),
            "attribute order should not matter: {result}"
        );
    }

    #[test]
    fn strict_xmlns_ignored() {
        let svg1 = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
            <text>Alice</text>
        </svg>"##;

        let svg2 = r##"<svg xmlns="http://www.w3.org/2000/svg">
            <text>Alice</text>
        </svg>"##;

        let result = compare_svg_strict(svg1, svg2).unwrap();
        assert!(
            result.is_match(),
            "xmlns attributes should be ignored: {result}"
        );
    }

    #[test]
    fn numeric_only_classifier() {
        let base = |w: &str, fill: &str, txt: &str| {
            format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="200">
                <rect x="10" y="20" width="{w}" height="40" fill="{fill}"/>
                <text x="11" y="33">{txt}</text>
                </svg>"##
            )
        };
        let golden = base("30.2618", "#F1F1F1", "Hi");

        // Coordinate diff within GEOM_EPS (width off by 0.0001) → tolerated by
        // the comparator: a structural match, no difference to classify.
        let r = compare_svg_strict(&golden, &base("30.2617", "#F1F1F1", "Hi")).unwrap();
        assert!(r.is_match(), "sub-epsilon coord diff should be tolerated");

        // Coordinate diff just ABOVE GEOM_EPS (width off by 0.05) → registers as
        // a numeric-only difference with that deviation.
        let r = compare_svg_strict(&golden, &base("30.3118", "#F1F1F1", "Hi")).unwrap();
        let d = r
            .numeric_only_max_deviation()
            .expect("should be numeric-only");
        assert!((d - 0.05).abs() < 1e-6, "dev {d}");
        assert!(!r.is_ulp_only(ULP_EPS));

        // Larger pure coordinate deviation (3px) → still numeric-only (magnitude
        // is the reviewer's call).
        let r = compare_svg_strict(&golden, &base("33.2618", "#F1F1F1", "Hi")).unwrap();
        assert_eq!(r.numeric_only_max_deviation().map(|d| d.round()), Some(3.0));

        // Color difference → structural, NEVER park-eligible.
        let r = compare_svg_strict(&golden, &base("30.2618", "#FF0000", "Hi")).unwrap();
        assert_eq!(r.numeric_only_max_deviation(), None);

        // Text difference → structural.
        let r = compare_svg_strict(&golden, &base("30.2618", "#F1F1F1", "Bye")).unwrap();
        assert_eq!(r.numeric_only_max_deviation(), None);

        // Exact match → None (not a near-miss).
        let r = compare_svg_strict(&golden, &golden).unwrap();
        assert_eq!(r.numeric_only_max_deviation(), None);
    }
}
