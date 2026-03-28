// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

/// Attributes that carry coordinate/dimension data and should be ignored
/// in structural comparison.
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
/// be ignored in structural comparison.
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

impl CompareResult {
    pub fn is_match(&self) -> bool {
        self.differences.is_empty()
    }
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

/// Extracts a flat list of structural elements from an SVG string.
pub fn extract_elements(svg: &str) -> Result<Vec<SvgElement>, String> {
    // Strip XML declaration and DOCTYPE if present — roxmltree 0.20+ rejects DTDs.
    // Graphviz-generated SVGs (used by @startdot) include these.
    let cleaned = strip_xml_preamble(svg);
    let doc =
        roxmltree::Document::parse(&cleaned).map_err(|e| format!("failed to parse SVG: {e}"))?;

    let mut elements = Vec::new();
    collect_elements(doc.root(), 0, &mut elements);
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

/// Compares two SVGs structurally, ignoring coordinates and dimensions.
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
}
