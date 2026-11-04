# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
p=Path('crates/rustuml-render/src/creole.rs');s=p.read_text();old='''                // CommandCreoleFontFamilyChange changes only the selected family,
                // not the font face and not an unconditional monospace mode.
                nested.font_family = Some(family.to_string());
                nested.monospace = false;''';new='''                // CommandCreoleFontFamilyChange -> FontConfiguration.changeFamily
                // preserves the family; FontStack decodes its named font. SVG's
                // SvgGraphics.drawString maps Monospaced to monospace and uses
                // NBSP for monospace/Courier. Match text_render's existing alias
                // normalization because its monospace flag also selects metrics;
                // do not activate that shared compatibility path for all fonts.
                nested.font_family = Some(family.to_string());
                nested.monospace = matches!(
                    family
                        .trim_matches(|c| c == '\\"' || c == '\\'')
                        .to_ascii_lowercase()
                        .as_str(),
                    "courier" | "monospace" | "monospaced"
                );'''
assert old in s;s=s.replace(old,new)
pos=s.index('    #[test]\n    fn parameter_candidates_reject_hidden_and_malformed_values')
test='''    #[test]
    fn parameter_font_aliases_preserve_shared_consumer_flags() {
        // Font-family grammar is broad; only the existing shared resolver's
        // named alias set activates its monospace spacing/metrics path.
        for family in ["Courier", "cOuRiEr", "monospace", "Monospaced", "\\\"Courier\\\"", "'MONOSPACE'"] {
            let segments = parse_segments(&format!("<font:{family}>code here</font>"));
            assert!(segments[0].style.monospace, "{family}");
            assert_eq!(segments[0].style.font_family.as_deref(), Some(family));
        }
        for family in ["SansSerif", "Dialog", "Arial", "color=blue", "Courier New", "Courier "] {
            let segments = parse_segments(&format!("<font:{family}>code here</font>"));
            assert!(!segments[0].style.monospace, "{family}");
        }
    }

'''
s=s[:pos]+test+s[pos:];p.write_text(s)
