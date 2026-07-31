// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Background-filter registry.
//!
//! PlantUML renders `<back:color>text</back>` creole markup as an SVG
//! `<filter>` element in `<defs>` plus a `filter="url(#id)"` attribute on
//! the matching `<text>` element. Filters are shared across same-coloured
//! segments — one filter per unique colour. Ids are derived from the
//! diagram source's seed (so every render is deterministic and byte-for-
//! byte reproducible against Java PlantUML's output).
//!
//! The registry is exposed as a thread-local. Each `render_svg` call
//! installs a fresh registry for the current diagram, the text-emission
//! path queries it to look up an id for a segment's background, and the
//! caller drains it into the `<defs>` block before finalising the SVG.

use std::cell::RefCell;

use crate::text_render;

/// Tracks unique background colours and assigns deterministic ids.
#[derive(Debug, Default)]
pub struct FilterRegistry {
    /// `"b" + base36(abs(seed))` — the prefix shared by every filter id.
    uid_prefix: String,
    /// Insertion order: normalized colour → assigned suffix.
    entries: Vec<(String, String)>,
}

impl FilterRegistry {
    /// Create a registry from a diagram source string. The source is hashed
    /// with PlantUML's `StringUtils.seed` and formatted in base36 to derive
    /// the shared id prefix. An empty source still works (`seed = h0`).
    pub fn for_source(source: &str) -> Self {
        let seed = plantuml_seed(source);
        let prefix = format!("b{}", abs_base36(seed));
        Self {
            uid_prefix: prefix,
            entries: Vec::new(),
        }
    }

    /// Look up (or allocate) the filter id for `color`. The colour is
    /// normalised to upper-case `#RRGGBB` first; subsequent lookups for the
    /// same colour return the same id.
    pub fn id_for(&mut self, color: &str) -> String {
        let normalized = normalize_back_color(color);
        if let Some((_, id)) = self.entries.iter().find(|(c, _)| c == &normalized) {
            return id.clone();
        }
        let id = format!("{}{}", self.uid_prefix, self.entries.len());
        self.entries.push((normalized, id.clone()));
        id
    }

    /// Emit the `<filter>` elements (without the surrounding `<defs>` tag)
    /// for every colour the registry has been asked about. Matches Java
    /// PlantUML's element shape exactly: ordered `height`, `id`, `width`,
    /// `x`, `y` attributes and the two filter children.
    pub fn render_defs_content(&self) -> String {
        let mut out = String::new();
        use std::fmt::Write;
        for (color, id) in &self.entries {
            write!(
                &mut out,
                r#"<filter height="1" id="{id}" width="1" x="0" y="0"><feFlood flood-color="{color}" result="flood"/><feComposite in="SourceGraphic" in2="flood" operator="over"/></filter>"#,
            )
            .unwrap();
        }
        out
    }

    /// True when no segment carrying a background has been emitted.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

thread_local! {
    /// Per-render registry. `with_registry` installs a fresh one for the
    /// duration of a render pass; text emission consults it via
    /// `id_for_current`.
    static CURRENT: RefCell<Option<FilterRegistry>> = const { RefCell::new(None) };
}

/// Install `registry` as the current filter registry for the duration of
/// `body`, then return the (mutated) registry. Nested calls panic — only
/// one render pass at a time per thread.
pub fn with_registry<R>(source: &str, body: impl FnOnce() -> R) -> (R, FilterRegistry) {
    CURRENT.with(|slot| {
        assert!(
            slot.borrow().is_none(),
            "filter_registry::with_registry is not re-entrant",
        );
        *slot.borrow_mut() = Some(FilterRegistry::for_source(source));
    });
    let result = body();
    let registry = CURRENT
        .with(|slot| slot.borrow_mut().take())
        .expect("registry was taken mid-render");
    (result, registry)
}

/// Derive PlantUML's drop-shadow filter id for a diagram source: `"f" +
/// base36(abs(seed))`, mirroring `SvgGraphics.shadowId`. Shares the same
/// seed as the `b`-prefixed back-colour filter ids.
pub fn shadow_id_for(source: &str) -> String {
    shadow_id_for_prefix(&id_seed_prefix_for_source(source))
}

/// Compute the base-36 prefix shared by SVG filters and gradients.
pub fn id_seed_prefix_for_source(source: &str) -> String {
    abs_base36(plantuml_seed(source))
}

/// Derive a shadow id from an already-computed `SvgGraphics.getSeed` prefix.
pub fn shadow_id_for_prefix(seed_prefix: &str) -> String {
    format!("f{seed_prefix}")
}

/// Derive the id for the `index`th unique SVG gradient in paint order.
///
/// Java provenance: `SvgGraphics` initializes `gradientId` to
/// `"g" + getSeed(seed)`, and `createSvgGradient` appends
/// `gradients.size()` when it first sees a unique color/policy tuple.
pub fn gradient_id_for(source: &str, index: usize) -> String {
    gradient_id_for_prefix(&id_seed_prefix_for_source(source), index)
}

/// Derive a gradient id from an already-computed `SvgGraphics.getSeed` prefix.
pub fn gradient_id_for_prefix(seed_prefix: &str, index: usize) -> String {
    format!("g{seed_prefix}{index}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SvgResourceKind {
    Filter,
    Gradient,
}

#[derive(Debug)]
struct SvgResource<'a> {
    id: &'a str,
    kind: SvgResourceKind,
    xml: &'a str,
}

/// Finalize source-seeded SVG resources from the shapes that actually use them.
///
/// Java provenance: `SvgGraphics.manageShadow` and
/// `createSvgGradient(HColorGradient)` register definitions when their drawing
/// paths first request them. Some renderers resolve theme styles before
/// painting and therefore produce an eager superset. This pass restores Java's
/// shared lazy registry contract: retain painted resources in their original
/// registration order, compact gradient suffixes, and prepare shadow geometry
/// for `SvgOption.getScale()`.
pub(crate) fn finalize_painted_resources(svg: &str, source: &str, document_scale: f64) -> String {
    let Some(defs_start) = svg.find("<defs") else {
        return svg.to_string();
    };
    let Some(open_len) = svg[defs_start..].find('>').map(|idx| idx + 1) else {
        return svg.to_string();
    };
    let defs_open_end = defs_start + open_len;
    if svg[defs_start..defs_open_end].trim_end().ends_with("/>") {
        return svg.to_string();
    }
    let Some(close_offset) = svg[defs_open_end..].find("</defs>") else {
        return svg.to_string();
    };
    let defs_close_start = defs_open_end + close_offset;
    let defs_close_end = defs_close_start + "</defs>".len();
    let defs_content = &svg[defs_open_end..defs_close_start];
    let Some(resources) = parse_generated_resources(defs_content, source) else {
        return svg.to_string();
    };
    if resources.is_empty() {
        return svg.to_string();
    }

    let body = &svg[defs_close_end..];
    let mut painted: Vec<&str> = Vec::new();
    let mut cursor = 0;
    while let Some(tag_start_rel) = body[cursor..].find('<') {
        let tag_start = cursor + tag_start_rel;
        let Some(tag_end_rel) = body[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end_rel + 1;
        let tag = &body[tag_start..tag_end];
        for id in resource_refs(tag, &resources) {
            if !painted.contains(&id) {
                painted.push(id);
            }
        }
        cursor = tag_end;
    }

    let seed_prefix = id_seed_prefix_for_source(source);
    let mut renames: Vec<(&str, String)> = Vec::new();
    let mut gradient_index = 0;
    let mut finalized_defs = String::new();
    for painted_id in &painted {
        let resource = resources
            .iter()
            .find(|resource| resource.id == *painted_id)
            .expect("painted resource came from parsed definitions");
        let new_id = match resource.kind {
            SvgResourceKind::Filter => resource.id.to_string(),
            SvgResourceKind::Gradient => {
                let id = gradient_id_for_prefix(&seed_prefix, gradient_index);
                gradient_index += 1;
                id
            }
        };
        let old_id_attr = format!(r#"id="{}""#, resource.id);
        let new_id_attr = format!(r#"id="{new_id}""#);
        if resource.kind == SvgResourceKind::Filter && document_scale != 1.0 {
            finalized_defs.push_str(&shadow_filter_def_before_document_scale(
                &new_id,
                document_scale,
            ));
        } else {
            finalized_defs.push_str(&resource.xml.replacen(&old_id_attr, &new_id_attr, 1));
        }
        renames.push((resource.id, new_id));
    }

    let mut out = String::with_capacity(svg.len());
    out.push_str(&svg[..defs_start]);
    if finalized_defs.is_empty() {
        out.push_str("<defs/>");
    } else {
        out.push_str(&svg[defs_start..defs_open_end]);
        out.push_str(&finalized_defs);
        out.push_str("</defs>");
    }
    out.push_str(&rewrite_resource_refs(body, &renames));
    out
}

fn parse_generated_resources<'a>(defs: &'a str, source: &str) -> Option<Vec<SvgResource<'a>>> {
    let seed = id_seed_prefix_for_source(source);
    let gradient_prefix = format!("g{seed}");
    let shadow_id = shadow_id_for_prefix(&seed);
    let mut resources = Vec::new();
    let mut cursor = 0;

    while cursor < defs.len() {
        let whitespace = defs[cursor..]
            .find(|ch: char| !ch.is_whitespace())
            .unwrap_or(defs.len() - cursor);
        cursor += whitespace;
        if cursor == defs.len() {
            break;
        }

        let (kind, close) = if defs[cursor..].starts_with("<linearGradient") {
            (SvgResourceKind::Gradient, "</linearGradient>")
        } else if defs[cursor..].starts_with("<filter") {
            (SvgResourceKind::Filter, "</filter>")
        } else {
            return None;
        };
        let end = defs[cursor..].find(close)? + cursor + close.len();
        let xml = &defs[cursor..end];
        let id = xml_attr(xml, "id")?;
        let generated = match kind {
            SvgResourceKind::Gradient => id
                .strip_prefix(&gradient_prefix)
                .is_some_and(|suffix| !suffix.is_empty() && suffix.parse::<usize>().is_ok()),
            SvgResourceKind::Filter => id == shadow_id,
        };
        if !generated {
            return None;
        }
        resources.push(SvgResource { id, kind, xml });
        cursor = end;
    }
    Some(resources)
}

fn xml_attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!(r#"{name}=""#);
    let start = tag.find(&needle)? + needle.len();
    let value = &tag[start..];
    value.find('"').map(|end| &value[..end])
}

fn resource_refs<'a>(tag: &str, resources: &[SvgResource<'a>]) -> Vec<&'a str> {
    let mut refs = Vec::new();
    let mut rest = tag;
    while let Some(start) = rest.find("url(#") {
        rest = &rest[start + "url(#".len()..];
        let Some(end) = rest.find(')') else {
            break;
        };
        let id = &rest[..end];
        if let Some(resource) = resources.iter().find(|resource| resource.id == id) {
            refs.push(resource.id);
        }
        rest = &rest[end + 1..];
    }
    refs
}

fn rewrite_resource_refs(body: &str, renames: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(start) = rest.find("url(#") {
        out.push_str(&rest[..start + "url(#".len()]);
        rest = &rest[start + "url(#".len()..];
        let Some(end) = rest.find(')') else {
            out.push_str(rest);
            return out;
        };
        let id = &rest[..end];
        if let Some((_, new_id)) = renames.iter().find(|(old_id, _)| *old_id == id) {
            out.push_str(new_id);
        } else {
            out.push_str(id);
        }
        out.push(')');
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Emit PlantUML's SVG drop-shadow filter body for `shadow_id`.
///
/// This is the `defs` child constructed by
/// `SvgGraphics.createXmlDocument`, using the fixed blur, color-matrix,
/// offset, and blend pipeline that backs shapes with `deltaShadow > 0`.
pub fn shadow_filter_def(shadow_id: &str) -> String {
    shadow_filter_def_scaled(shadow_id, 1.0)
}

/// Emit the drop-shadow filter using the same SVG scale as shape coordinates.
///
/// Java provenance: `SvgGraphics.manageShadow` keeps the filter region at
/// literal `-1` and passes only blur `2` and offset `4` through
/// `SvgGraphics.format`, which multiplies by `SvgOption.getScale()`.
pub fn shadow_filter_def_scaled(shadow_id: &str, scale: f64) -> String {
    shadow_filter_def_with_region(shadow_id, scale, "-1")
}

/// Encode a Java-scaled shadow def before Rust's finished-SVG scale pass.
///
/// The final pass scales geometric `x`/`y` attributes but does not scale SVG
/// filter primitives. Pre-dividing the literal filter region preserves
/// `SvgGraphics.manageShadow`'s `-1`, while blur and offset are emitted at
/// their final `SvgOption` scale.
pub fn shadow_filter_def_before_document_scale(shadow_id: &str, scale: f64) -> String {
    let region = format_svg_number(-1.0 / scale);
    shadow_filter_def_with_region(shadow_id, scale, &region)
}

fn shadow_filter_def_with_region(shadow_id: &str, scale: f64, region: &str) -> String {
    let blur = format_svg_number(2.0 * scale);
    let offset = format_svg_number(4.0 * scale);
    format!(
        r#"<filter height="300%" id="{shadow_id}" width="300%" x="{region}" y="{region}"><feGaussianBlur result="blurOut" stdDeviation="{blur}"/><feColorMatrix in="blurOut" result="blurOut2" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 .4 0"/><feOffset dx="{offset}" dy="{offset}" in="blurOut2" result="blurOut3"/><feBlend in="SourceGraphic" in2="blurOut3" mode="normal"/></filter>"#
    )
}

pub(crate) fn format_svg_number(value: f64) -> String {
    let mut formatted = format!("{value:.4}");
    while formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    if formatted == "-0" {
        "0".to_string()
    } else {
        formatted
    }
}

/// Look up an id from the current registry, if any. Used by the text-
/// emission path: a segment with `style.background = Some(_)` calls this
/// to obtain the `filter="url(#...)"` value.
pub fn id_for_current(color: &str) -> Option<String> {
    CURRENT.with(|slot| slot.borrow_mut().as_mut().map(|reg| reg.id_for(color)))
}

/// Normalise a `<back:color>` colour spec to the form PlantUML uses inside
/// the filter element and the registry key. Named CSS colours are
/// resolved to upper-case `#RRGGBB`; explicit hex is upper-cased. Anything
/// we don't recognise is passed through.
fn normalize_back_color(color: &str) -> String {
    let resolved = text_render::normalize_color(color);
    if let Some(rest) = resolved.strip_prefix('#') {
        format!("#{}", rest.to_ascii_uppercase())
    } else {
        resolved
    }
}

/// PlantUML's `StringUtils.seed`: rolling 31× hash starting from the
/// Mersenne-ish prime `1125899906842597`, taken modulo 2^64 with Java's
/// signed-overflow semantics.
fn plantuml_seed(s: &str) -> i64 {
    // Java provenance: `UmlSource.getPlainString("\n")` appends every
    // post-TIM source line, including blank lines, before `StringUtils.seed`
    // hashes the resulting string.
    let mut h: u64 = 1_125_899_906_842_597;
    for c in s.chars() {
        // Java's `long h = 31 * h + s.charAt(i)` — `charAt` returns a 16-bit
        // UTF-16 unit. For characters outside the BMP this would need
        // surrogate-pair splitting, but the diagram source never hits that
        // path (all creole tokens are ASCII or BMP code points).
        h = h.wrapping_mul(31).wrapping_add(c as u32 as u64);
    }
    h as i64
}

/// Java's `Long.toString(Math.abs(seed), 36)`. `Math.abs(Long.MIN_VALUE)`
/// stays negative — Java emits `-Long.MIN_VALUE` in base36 with a leading
/// `-`. We treat that corner case by falling back to the absolute u64 of
/// `0x8000_0000_0000_0000`, which is what PlantUML would print after the
/// `-` it accidentally keeps. The seed is wildly unlikely to land there.
fn abs_base36(seed: i64) -> String {
    let n = if seed == i64::MIN {
        i64::MAX as u64 + 1
    } else {
        seed.unsigned_abs()
    };
    if n == 0 {
        return "0".to_string();
    }
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buf = Vec::new();
    let mut v = n;
    while v > 0 {
        buf.push(digits[(v % 36) as usize]);
        v /= 36;
    }
    buf.reverse();
    String::from_utf8(buf).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_matches_plantuml_known_source() {
        // The cyan-in-class golden's filter id is `bjdpys1nesotu0`, i.e.
        // prefix `bjdpys1nesotu` for the colour at index 0.
        let src = "@startuml\nclass MyClass {\n  <back:cyan>cyan bg</back>: field\n}\n@enduml\n";
        let reg = FilterRegistry::for_source(src);
        assert_eq!(reg.uid_prefix, "bjdpys1nesotu");
    }

    #[test]
    fn seed_preserves_blank_lines_in_plantuml_plain_source() {
        // This represents a blank emitted by TIM after the source reader, not
        // a raw empty line immediately following `@startuml`.
        let src = "@startuml\n\nskinparam shadowing true\n:Fresh seeded shadow;\n@enduml\n";
        assert_eq!(shadow_id_for(src), "f11pg9m4y9hc3p");
    }

    #[test]
    fn parser_source_identity_matches_java_shadow_id_after_initial_blanks() {
        let input = concat!(
            "@startuml\n",
            "\n",
            "\n",
            "skinparam shadowing true\n",
            "start\n",
            ":Fresh intake;\n",
            "if (route?) then (north)\n",
            "  :North alpha;\n",
            "  :North beta;\n",
            "else (south)\n",
            "  :South gamma;\n",
            "endif\n",
            ":Fresh archive;\n",
            "stop\n",
            "@enduml\n",
        );
        let diagram = rustuml_parser::parse::parse(input).unwrap();
        let source = diagram.meta().source.as_deref().unwrap();

        // `SvgGraphics.shadowId`: StringUtils.seed over the post-reader source.
        assert_eq!(shadow_id_for(source), "fbdz2fx1jlqnt");
    }

    #[test]
    fn gradient_id_shares_the_source_seed_and_uses_paint_order() {
        let src = "@startuml\nstate Copper #red/blue\n@enduml\n";
        assert_eq!(gradient_id_for(src, 0), "gdxt0vbudmxt10");
        assert_eq!(gradient_id_for(src, 3), "gdxt0vbudmxt13");
    }

    #[test]
    fn prefixed_ids_and_dpi_scaled_shadow_match_svg_graphics_contract() {
        assert_eq!(shadow_id_for_prefix("freshseed"), "ffreshseed");
        assert_eq!(gradient_id_for_prefix("freshseed", 2), "gfreshseed2");
        let defs = shadow_filter_def_scaled("ffreshseed", 100.0 / 96.0);
        assert!(defs.contains(r#"x="-1" y="-1""#));
        let blur = format_svg_number(2.0 * 100.0 / 96.0);
        let offset = format_svg_number(4.0 * 100.0 / 96.0);
        assert!(defs.contains(&format!(r#"stdDeviation="{blur}""#)));
        assert!(defs.contains(&format!(r#"dx="{offset}" dy="{offset}""#)));
        let pre_scaled = shadow_filter_def_before_document_scale("ffreshseed", 100.0 / 96.0);
        assert!(pre_scaled.contains(r#"x="-0.96" y="-0.96""#));
        assert!(pre_scaled.contains(&format!(r#"stdDeviation="{blur}""#)));
    }

    #[test]
    fn cyan_resolves_to_upper_hex() {
        let mut reg = FilterRegistry::for_source("seed");
        assert_eq!(&reg.id_for("cyan")[reg.uid_prefix.len()..], "0");
        assert_eq!(&reg.id_for("#00FFFF")[reg.uid_prefix.len()..], "0");
        assert_eq!(&reg.id_for("#FFEECC")[reg.uid_prefix.len()..], "1");
    }

    #[test]
    fn defs_content_emits_two_filters() {
        let mut reg = FilterRegistry::for_source("seed");
        let id_a = reg.id_for("cyan");
        let id_b = reg.id_for("#FFEECC");
        let defs = reg.render_defs_content();
        assert!(defs.contains(&format!("id=\"{id_a}\"")));
        assert!(defs.contains(&format!("id=\"{id_b}\"")));
        assert!(defs.contains("flood-color=\"#00FFFF\""));
        assert!(defs.contains("flood-color=\"#FFEECC\""));
    }

    #[test]
    fn painted_theme_resources_are_lazy_compacted_in_first_paint_order() {
        let source = concat!(
            "@startuml\n",
            "!theme fresh-copper\n",
            "participant RenamedIntake\n",
            "participant DeepArchive\n",
            "participant AuditBranch\n",
            "RenamedIntake -> DeepArchive : route\n",
            "note right of AuditBranch : fresh override\n",
            "@enduml\n",
        );
        let seed = id_seed_prefix_for_source(source);
        let gradient = |index| gradient_id_for_prefix(&seed, index);
        let shadow = shadow_id_for_prefix(&seed);
        let svg = format!(
            concat!(
                r#"<svg><defs>"#,
                r##"<linearGradient id="{g0}"><stop stop-color="#111111"/></linearGradient>"##,
                r#"<filter id="{shadow}"><feGaussianBlur/></filter>"#,
                r##"<linearGradient id="{g1}"><stop stop-color="#22AA44"/></linearGradient>"##,
                r##"<linearGradient id="{g2}"><stop stop-color="#333333"/></linearGradient>"##,
                r##"<linearGradient id="{g3}"><stop stop-color="#4477CC"/></linearGradient>"##,
                r#"</defs><g>"#,
                r#"<rect fill="url(#{g1})" filter="url(#{shadow})"/>"#,
                r#"<path fill="url(#{g3})"/>"#,
                r#"</g></svg>"#,
            ),
            g0 = gradient(0),
            g1 = gradient(1),
            g2 = gradient(2),
            g3 = gradient(3),
            shadow = shadow,
        );

        let scale = 125.0 / 96.0;
        let finalized = finalize_painted_resources(&svg, source, scale);
        assert!(finalized.contains("<defs>"), "{finalized}");
        let defs = finalized
            .split_once("<defs>")
            .and_then(|(_, rest)| rest.split_once("</defs>"))
            .map(|(defs, _)| defs)
            .unwrap();
        let expected_first = format!(r#"id="{}""#, gradient(0));
        let expected_second = format!(r#"id="{shadow}""#);
        let expected_third = format!(r#"id="{}""#, gradient(1));
        assert!(defs.find(&expected_first).unwrap() < defs.find(&expected_second).unwrap());
        assert!(defs.find(&expected_second).unwrap() < defs.find(&expected_third).unwrap());
        assert!(defs.contains(r##"stop-color="#22AA44""##));
        assert!(defs.contains(r##"stop-color="#4477CC""##));
        assert!(!defs.contains(r##"stop-color="#111111""##));
        assert!(!defs.contains(r##"stop-color="#333333""##));
        assert!(finalized.contains(&format!(
            r#"fill="url(#{})" filter="url(#{shadow})""#,
            gradient(0)
        )));
        assert!(finalized.contains(&format!(r#"fill="url(#{})""#, gradient(1))));
        let region = format_svg_number(-1.0 / scale);
        let blur = format_svg_number(2.0 * scale);
        assert!(defs.contains(&format!(r#"x="{region}" y="{region}""#)));
        assert!(defs.contains(&format!(r#"stdDeviation="{blur}""#)));
    }
}
