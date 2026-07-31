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

use std::fmt::Write;

#[derive(Clone, Copy)]
struct CompatibilityThemeProfile {
    background: &'static str,
    foreground: &'static str,
    hyperlink: &'static str,
    font: &'static str,
    monospace_font: &'static str,
    line_thickness: u8,
    margin: u8,
    circled_character_radius: u8,
}

impl CompatibilityThemeProfile {
    fn source(self) -> String {
        let mut source = String::new();
        writeln!(
            source,
            "<style>\nroot {{\n\
             BackgroundColor {}\n\
             FontColor {}\n\
             FontName {}\n\
             HyperLinkColor {}\n\
             LineColor {}\n\
             LineThickness {}\n\
             Margin {}\n\
             }}\n</style>",
            self.background,
            self.foreground,
            self.font,
            self.hyperlink,
            self.foreground,
            self.line_thickness,
            self.margin
        )
        .unwrap();
        writeln!(
            source,
            "skinparam CircledCharacterRadius {}",
            self.circled_character_radius
        )
        .unwrap();

        for (key, value) in [
            ("BackgroundColor", self.background),
            ("DefaultFontName", self.font),
            ("Shadowing", "false"),
            ("CircledCharacterFontColor", self.foreground),
            ("CircledCharacterFontName", self.monospace_font),
            ("ClassBackgroundColor", self.background),
            ("ClassBorderColor", self.foreground),
            ("ClassFontColor", self.foreground),
            ("ClassFontName", self.font),
            ("ClassAttributeFontColor", self.foreground),
            ("ClassAttributeFontName", self.font),
            ("ClassStereotypeFontColor", self.foreground),
            ("ClassStereotypeFontName", self.font),
            ("StereotypeABackgroundColor", self.background),
            ("StereotypeABorderColor", self.foreground),
            ("StereotypeCBackgroundColor", self.background),
            ("StereotypeCBorderColor", self.foreground),
            ("StereotypeEBackgroundColor", self.background),
            ("StereotypeEBorderColor", self.foreground),
            ("StereotypeIBackgroundColor", self.background),
            ("StereotypeIBorderColor", self.foreground),
            ("StereotypeNBackgroundColor", self.background),
            ("StereotypeNBorderColor", self.foreground),
        ] {
            writeln!(source, "skinparam {key} {value}").unwrap();
        }
        source
    }
}

/// Build an independently encoded behavioral profile for a known theme whose
/// upstream source cannot be bundled under RustUML's Apache-2.0 distribution.
///
/// These are semantic palette and metric facts, not copies of the upstream
/// theme file. The generated declarations still flow through the ordinary
/// theme preprocessor and style cascade.
pub(super) fn get_compatibility_theme_source(name: &str) -> Option<String> {
    // Java provenance: `puml-theme-plain.puml` lines 15-43, 65-72,
    // 88-105, and 150-169 at PlantUML 71806a23780b04a5ccde2f8ceb5121edad5eb711.
    let profile = match name {
        "plain" => CompatibilityThemeProfile {
            background: "white",
            foreground: "black",
            hyperlink: "blue",
            font: "Verdana",
            monospace_font: "Courier",
            line_thickness: 1,
            margin: 5,
            circled_character_radius: 9,
        },
        _ => return None,
    };
    Some(profile.source())
}

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
        // excluded to keep this distribution Apache-2.0 clean. Except for the
        // independently encoded `plain` compatibility profile above, such
        // names fall through to `None` and render unstyled, matching
        // PlantUML's behaviour for an unknown theme. `_none_` is an
        // intentionally empty file (no copyrightable content), kept as a
        // baseline. Users who want another excluded theme can supply it from
        // the PlantUML source tree.
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
/// 1. `<style>...</style>` blocks — preserved for the shared parser-owned
///    style pass. That pass records their sparse selector/property structure
///    and masks them before diagram-family parsing.
/// 2. Grouped `skinparam Prefix { Key Value ... }` blocks — only some
///    parsers (state, activity) flatten these; flattening here means
///    class, sequence and the rest also pick up the entries.
pub(super) fn flatten_theme_output(lines: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut in_style = false;
    let mut style_scopes: Vec<String> = Vec::new();
    let mut style_compatibility = Vec::new();
    let mut skin_prefix: Option<String> = None;

    for raw in lines {
        let source = super::split_source_line_marker(raw).map_or(raw.as_str(), |(_, text)| text);
        let line = source.trim();

        // Preserve the CSS source for the shared style parser. The three
        // root-level compatibility values remain ordinary skinparams after
        // the block until existing renderers migrate to StyleProgram.
        if in_style {
            if line.contains("</style>") {
                out.push(source.to_string());
                in_style = false;
                style_scopes.clear();
                out.append(&mut style_compatibility);
                continue;
            }
            if line == "}" {
                style_scopes.pop();
            } else if let Some(scope) = line.strip_suffix('{') {
                style_scopes.push(scope.trim().to_ascii_lowercase());
            } else if style_scopes.len() == 1
                && style_scopes.last().is_some_and(|scope| scope == "root")
                && let Some((key, value)) = line.split_once(char::is_whitespace)
            {
                if key.eq_ignore_ascii_case("LineThickness") {
                    style_compatibility.push(format!(
                        "skinparam __styleRootLineThickness {}",
                        value.trim()
                    ));
                } else if key.eq_ignore_ascii_case("LineColor") {
                    style_compatibility
                        .push(format!("skinparam __styleRootLineColor {}", value.trim()));
                } else if key.eq_ignore_ascii_case("FontColor") {
                    style_compatibility
                        .push(format!("skinparam __styleRootFontColor {}", value.trim()));
                }
            }
            out.push(source.to_string());
            continue;
        }
        if line.starts_with("<style") {
            out.push(source.to_string());
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

        out.push(source.to_string());
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
        assert!(get_compatibility_theme_source("not-a-real-theme").is_none());
    }

    #[test]
    fn plain_compatibility_profile_is_generated_without_bundling_upstream_source() {
        assert!(get_theme_source("plain").is_none());
        let source = get_compatibility_theme_source("plain").unwrap();

        assert!(source.contains("BackgroundColor white"));
        assert!(source.contains("LineColor black"));
        assert!(source.contains("LineThickness 1"));
        assert!(source.contains("Margin 5"));
        assert!(source.contains("skinparam ClassFontName Verdana"));
        assert!(source.contains("skinparam CircledCharacterRadius 9"));
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
    fn flatten_preserves_style_block() {
        let input = vec![
            "<style>".to_string(),
            "  root { BackgroundColor white }".to_string(),
            "</style>".to_string(),
            "skinparam shadowing false".to_string(),
        ];
        let out = flatten_theme_output(&input);
        assert_eq!(out, input);
    }

    #[test]
    fn flatten_preserves_root_line_thickness_from_style() {
        let input = vec![
            "<style>".to_string(),
            "root {".to_string(),
            "  FontColor #FFFFFF".to_string(),
            "  LineColor #2683B9".to_string(),
            "  LineThickness 1".to_string(),
            "  Padding 6".to_string(),
            "}".to_string(),
            "activity {".to_string(),
            "  LineThickness 2".to_string(),
            "}".to_string(),
            "</style>".to_string(),
        ];
        let out = flatten_theme_output(&input);
        assert_eq!(
            out,
            vec![
                "<style>".to_string(),
                "root {".to_string(),
                "  FontColor #FFFFFF".to_string(),
                "  LineColor #2683B9".to_string(),
                "  LineThickness 1".to_string(),
                "  Padding 6".to_string(),
                "}".to_string(),
                "activity {".to_string(),
                "  LineThickness 2".to_string(),
                "}".to_string(),
                "</style>".to_string(),
                "skinparam __styleRootFontColor #FFFFFF".to_string(),
                "skinparam __styleRootLineColor #2683B9".to_string(),
                "skinparam __styleRootLineThickness 1".to_string(),
            ]
        );
    }

    #[test]
    fn flatten_preserves_nested_style_without_root_compatibility() {
        let input = vec![
            "<style>".to_string(),
            "wbsDiagram, mindmapDiagram {".to_string(),
            "  root {".to_string(),
            "    FontColor #FFFFFF".to_string(),
            "    LineColor #2683B9".to_string(),
            "  }".to_string(),
            "}".to_string(),
            "</style>".to_string(),
        ];
        let out = flatten_theme_output(&input);
        assert_eq!(out, input);
        assert!(
            !out.iter()
                .any(|line| line.starts_with("skinparam __styleRoot"))
        );
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
    fn flatten_expands_marker_prefixed_skinparam_block() {
        let input = vec![
            "skinparam participant {".to_string(),
            super::super::source_line_marker(153, "  FontColor #FFF"),
            super::super::source_line_marker(154, "  BorderColor #EC7211"),
            "}".to_string(),
        ];
        let out = flatten_theme_output(&input);
        assert_eq!(
            out,
            vec![
                "skinparam participantFontColor #FFF".to_string(),
                "skinparam participantBorderColor #EC7211".to_string(),
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
