// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Embedded PlantUML theme resolution.
//!
//! The `!theme NAME` directive loads a `.puml` theme file from PlantUML's
//! themes directory and applies its skinparam declarations plus `<style>`
//! block. We embed a vendored copy of the themes shipped with PlantUML
//! (sourced from `plantuml/src/main/resources/themes/puml-theme-*.puml`)
//! so the binary stays self-contained. Each bundled theme carries its own
//! permissive licence (MIT or Apache-2.0) in its YAML front matter. Themes
//! that are GPL-3+ or have no licence declared (and therefore inherit
//! PlantUML's GPL-3+ by default) are deliberately excluded to keep the
//! distribution Apache-2.0 clean. Users who want an excluded theme can supply
//! it themselves from the PlantUML source tree. See `get_theme_source` for the
//! exclusion list.
//!
//! Themes are normalised by stripping the optional YAML front matter
//! (`---\n…\n---`) before expansion, then handed back to the preprocessor
//! as a regular include body so variable assignments, `!if` guards,
//! `!procedure`s and the embedded `<style>` block work the same way they
//! would in PlantUML's own pipeline.

/// Look up the embedded source for a theme by name.
///
/// Returns `None` if the theme name is not bundled.
pub(super) fn get_theme_source(name: &str) -> Option<&'static str> {
    Some(match name {
        // Only themes carrying a permissive licence (MIT or Apache-2.0) in
        // their upstream YAML front matter are bundled. Themes with a GPL-3+
        // licence (`sunlust`) or a blank/unspecified licence (`amiga`,
        // `blueprint`, `carbon-gray`, `crt-amber`, `crt-green`, `mimeograph`,
        // `mono`, `plain`) inherit PlantUML's GPL-3+ by default and are
        // excluded to keep this distribution Apache-2.0 clean. Such names fall
        // through to `None` and render unstyled, matching PlantUML's behaviour
        // for an unknown theme. `_none_` is an intentionally empty file (no
        // copyrightable content), kept as a baseline. Users who want an
        // excluded theme can supply it from the PlantUML source tree.
        "_none_" => include_str!("../../themes/puml-theme-_none_.puml"),
        "aws-orange" => include_str!("../../themes/puml-theme-aws-orange.puml"),
        "black-knight" => include_str!("../../themes/puml-theme-black-knight.puml"),
        "bluegray" => include_str!("../../themes/puml-theme-bluegray.puml"),
        "cerulean" => include_str!("../../themes/puml-theme-cerulean.puml"),
        "cerulean-outline" => include_str!("../../themes/puml-theme-cerulean-outline.puml"),
        "cloudscape-design" => include_str!("../../themes/puml-theme-cloudscape-design.puml"),
        "cyborg" => include_str!("../../themes/puml-theme-cyborg.puml"),
        "cyborg-outline" => include_str!("../../themes/puml-theme-cyborg-outline.puml"),
        "hacker" => include_str!("../../themes/puml-theme-hacker.puml"),
        "lightgray" => include_str!("../../themes/puml-theme-lightgray.puml"),
        "mars" => include_str!("../../themes/puml-theme-mars.puml"),
        "materia" => include_str!("../../themes/puml-theme-materia.puml"),
        "materia-outline" => include_str!("../../themes/puml-theme-materia-outline.puml"),
        "metal" => include_str!("../../themes/puml-theme-metal.puml"),
        "minty" => include_str!("../../themes/puml-theme-minty.puml"),
        "reddress-darkblue" => include_str!("../../themes/puml-theme-reddress-darkblue.puml"),
        "reddress-darkgreen" => include_str!("../../themes/puml-theme-reddress-darkgreen.puml"),
        "reddress-darkorange" => include_str!("../../themes/puml-theme-reddress-darkorange.puml"),
        "reddress-darkred" => include_str!("../../themes/puml-theme-reddress-darkred.puml"),
        "reddress-lightblue" => include_str!("../../themes/puml-theme-reddress-lightblue.puml"),
        "reddress-lightgreen" => include_str!("../../themes/puml-theme-reddress-lightgreen.puml"),
        "reddress-lightorange" => include_str!("../../themes/puml-theme-reddress-lightorange.puml"),
        "reddress-lightred" => include_str!("../../themes/puml-theme-reddress-lightred.puml"),
        "sandstone" => include_str!("../../themes/puml-theme-sandstone.puml"),
        "silver" => include_str!("../../themes/puml-theme-silver.puml"),
        "sketchy" => include_str!("../../themes/puml-theme-sketchy.puml"),
        "sketchy-outline" => include_str!("../../themes/puml-theme-sketchy-outline.puml"),
        "spacelab" => include_str!("../../themes/puml-theme-spacelab.puml"),
        "spacelab-white" => include_str!("../../themes/puml-theme-spacelab-white.puml"),
        "superhero" => include_str!("../../themes/puml-theme-superhero.puml"),
        "superhero-outline" => include_str!("../../themes/puml-theme-superhero-outline.puml"),
        "toy" => include_str!("../../themes/puml-theme-toy.puml"),
        "united" => include_str!("../../themes/puml-theme-united.puml"),
        "vibrant" => include_str!("../../themes/puml-theme-vibrant.puml"),
        _ => return None,
    })
}

/// Strip an optional YAML front-matter block from the head of a theme file.
///
/// PlantUML themes prefix metadata between `---` delimiters; that block is
/// not preprocessor syntax and would otherwise produce a long string of
/// unrecognised lines.
pub(super) fn strip_front_matter(source: &str) -> &str {
    let trimmed = source.trim_start();
    let Some(after_open) = trimmed.strip_prefix("---") else {
        return source;
    };
    // Only treat as front matter when the opener occupies its own line.
    let body = match after_open.strip_prefix('\n') {
        Some(rest) => rest,
        None => after_open.strip_prefix("\r\n").unwrap_or(after_open),
    };
    // Find a closing `---` that occupies its own line.
    for (idx, _) in body.match_indices("---") {
        let before_ok = idx == 0 || matches!(body.as_bytes().get(idx - 1), Some(b'\n'));
        let after = &body[idx + 3..];
        let after_ok = after.is_empty()
            || after.starts_with('\n')
            || after.starts_with("\r\n")
            || after.starts_with(' ')
            || after.starts_with('\t');
        if before_ok && after_ok {
            // Skip past the closing delimiter and (optionally) its newline.
            let mut rest = after;
            if let Some(stripped) = rest.strip_prefix("\r\n") {
                rest = stripped;
            } else if let Some(stripped) = rest.strip_prefix('\n') {
                rest = stripped;
            }
            return rest;
        }
    }
    source
}

/// Post-process expanded theme output so every line is either a single
/// `skinparam Key Value` directive or something the diagram parser can
/// recognise (notes, sprites, etc.).
///
/// Themes use two PlantUML constructs the per-diagram parsers do not all
/// understand:
///
/// 1. `<style>...</style>` blocks — used to drive the modern CSS-style
///    skin system. None of our renderers consume them yet, so dropping
///    them entirely is harmless and keeps stray `{`/`}` tokens from
///    leaking into the diagram.
/// 2. Grouped `skinparam Prefix { Key Value ... }` blocks — only some
///    parsers (state, activity) flatten these; flattening here means
///    class, sequence and the rest also pick up the entries.
pub(super) fn flatten_theme_output(lines: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut in_style = false;
    let mut skin_prefix: Option<String> = None;

    for raw in lines {
        let line = raw.trim();

        // `<style>` block: drop everything up to `</style>` (case-sensitive,
        // matching PlantUML's own convention).
        if in_style {
            if line.contains("</style>") {
                in_style = false;
            }
            continue;
        }
        if line.starts_with("<style") {
            // A single-line `<style>foo</style>` is unusual but also dropped.
            if !line.contains("</style>") {
                in_style = true;
            }
            continue;
        }

        // Grouped skinparam block.
        if let Some(prefix) = &skin_prefix {
            if line == "}" {
                skin_prefix = None;
                continue;
            }
            if line.is_empty() {
                continue;
            }
            // Each nested entry looks like `Key Value [...]`. Combine the
            // prefix and key to form a flat skinparam declaration.
            if let Some((key, value)) = line.split_once(char::is_whitespace) {
                let key = key.trim();
                let value = value.trim();
                if key.is_empty() || value.is_empty() {
                    continue;
                }
                out.push(format!("skinparam {prefix}{key} {value}"));
            }
            continue;
        }

        // Skinparam block opener.
        if let Some(rest) = line.strip_prefix("skinparam ") {
            let rest = rest.trim();
            if let Some(prefix) = rest.strip_suffix('{') {
                skin_prefix = Some(prefix.trim().to_string());
                continue;
            }
            if let Some((key, value)) = rest.split_once(char::is_whitespace) {
                let value = value.trim();
                if value == "{" {
                    skin_prefix = Some(key.trim().to_string());
                    continue;
                }
            }
        }

        out.push(raw.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_stripped() {
        let input = "---\nname: plain\nauthor: someone\n---\n!$X = 1\nbody";
        assert_eq!(strip_front_matter(input), "!$X = 1\nbody");
    }

    #[test]
    fn no_front_matter_passes_through() {
        let input = "!$X = 1\nbody";
        assert_eq!(strip_front_matter(input), "!$X = 1\nbody");
    }

    #[test]
    fn unterminated_front_matter_passes_through() {
        let input = "---\nname: plain\nno_closer\n";
        assert_eq!(strip_front_matter(input), input);
    }

    #[test]
    fn known_themes_resolve() {
        assert!(get_theme_source("cerulean").is_some());
        assert!(get_theme_source("superhero").is_some());
        assert!(get_theme_source("_none_").is_some());
    }

    #[test]
    fn unknown_theme_returns_none() {
        assert!(get_theme_source("not-a-real-theme").is_none());
    }

    #[test]
    fn non_permissive_themes_excluded() {
        // GPL-3+ and blank-licence themes are deliberately not bundled, to keep
        // the distribution Apache-2.0 clean (see `get_theme_source`).
        for name in [
            "sunlust",
            "plain",
            "amiga",
            "blueprint",
            "carbon-gray",
            "crt-amber",
            "crt-green",
            "mimeograph",
            "mono",
        ] {
            assert!(
                get_theme_source(name).is_none(),
                "{name} should be excluded for licence reasons"
            );
        }
    }

    #[test]
    fn flatten_drops_style_block() {
        let input = vec![
            "<style>".to_string(),
            "  root { BackgroundColor white }".to_string(),
            "</style>".to_string(),
            "skinparam shadowing false".to_string(),
        ];
        let out = flatten_theme_output(&input);
        assert_eq!(out, vec!["skinparam shadowing false".to_string()]);
    }

    #[test]
    fn flatten_expands_skinparam_block() {
        let input = vec![
            "skinparam class {".to_string(),
            "  BackgroundColor white".to_string(),
            "  BorderColor black".to_string(),
            "}".to_string(),
        ];
        let out = flatten_theme_output(&input);
        assert_eq!(
            out,
            vec![
                "skinparam classBackgroundColor white".to_string(),
                "skinparam classBorderColor black".to_string(),
            ]
        );
    }

    #[test]
    fn flatten_preserves_plain_skinparams() {
        let input = vec![
            "skinparam backgroundColor white".to_string(),
            "skinparam shadowing false".to_string(),
        ];
        let out = flatten_theme_output(&input);
        assert_eq!(out, input);
    }

    #[test]
    fn flatten_skinparam_block_no_space() {
        let input = vec![
            "skinparam class{".to_string(),
            "  BackgroundColor white".to_string(),
            "}".to_string(),
        ];
        let out = flatten_theme_output(&input);
        assert_eq!(
            out,
            vec!["skinparam classBackgroundColor white".to_string(),]
        );
    }
}
