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
    format!("f{}", abs_base36(plantuml_seed(source)))
}

/// Derive the id for the `index`th unique SVG gradient in paint order.
///
/// Java provenance: `SvgGraphics` initializes `gradientId` to
/// `"g" + getSeed(seed)`, and `createSvgGradient` appends
/// `gradients.size()` when it first sees a unique color/policy tuple.
pub fn gradient_id_for(source: &str, index: usize) -> String {
    format!("g{}{index}", abs_base36(plantuml_seed(source)))
}

/// Emit PlantUML's SVG drop-shadow filter body for `shadow_id`.
///
/// This is the `defs` child constructed by
/// `SvgGraphics.createXmlDocument`, using the fixed blur, color-matrix,
/// offset, and blend pipeline that backs shapes with `deltaShadow > 0`.
pub fn shadow_filter_def(shadow_id: &str) -> String {
    format!(
        r#"<filter height="300%" id="{shadow_id}" width="300%" x="-1" y="-1"><feGaussianBlur result="blurOut" stdDeviation="2"/><feColorMatrix in="blurOut" result="blurOut2" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 .4 0"/><feOffset dx="4" dy="4" in="blurOut2" result="blurOut3"/><feBlend in="SourceGraphic" in2="blurOut3" mode="normal"/></filter>"#
    )
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
    // PlantUML hashes `UmlSource.getPlainString("\n")`, after its source
    // reader has discarded empty lines immediately following `@start...`.
    // Preserve every other byte because comments, indentation, and the final
    // newline all participate in `StringUtils.seed`.
    let mut normalized;
    let s = if let Some(first_newline) = s.find('\n')
        && s[..first_newline].trim_start().starts_with("@start")
    {
        let mut content_start = first_newline + 1;
        while content_start < s.len() {
            let line_end = s[content_start..]
                .find('\n')
                .map_or(s.len(), |offset| content_start + offset + 1);
            if !s[content_start..line_end].trim().is_empty() {
                break;
            }
            content_start = line_end;
        }
        if content_start > first_newline + 1 {
            normalized = String::with_capacity(s.len());
            normalized.push_str(&s[..first_newline + 1]);
            normalized.push_str(&s[content_start..]);
            normalized.as_str()
        } else {
            s
        }
    } else {
        s
    };
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
    fn seed_uses_plantuml_plain_source_without_initial_blank_lines() {
        let src = "@startuml\n\nskinparam shadowing true\n:Fresh seeded shadow;\n@enduml\n";
        assert_eq!(shadow_id_for(src), "f8uac2ses7vez");
    }

    #[test]
    fn gradient_id_shares_the_source_seed_and_uses_paint_order() {
        let src = "@startuml\nstate Copper #red/blue\n@enduml\n";
        assert_eq!(gradient_id_for(src, 0), "gdxt0vbudmxt10");
        assert_eq!(gradient_id_for(src, 3), "gdxt0vbudmxt13");
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
}
