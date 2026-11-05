# Copyright 2026 Marcelo Cantos
# SPDX-License-Identifier: Apache-2.0
from pathlib import Path
p=Path('crates/rustuml-render/src/creole.rs');s=p.read_text();pos=s.index('\nfn walk_parameter_body(')
helper='''
// HColorSet.parseColor/parseSimpleColor separate resolution from command syntax.
// Valid values retain their existing emission path; this predicate only decides
// which command-specific failure behavior applies.
fn is_resolvable_color(color: &str) -> bool {
    fn simple(color: &str) -> bool {
        let color = color.strip_prefix('#').unwrap_or(color);
        if matches!(color.len(), 1 | 3 | 6 | 8)
            && color.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return true;
        }
        let name = color.to_ascii_lowercase();
        // ColorTrieNode's registry shares the existing CSS lookup except these
        // seven Archimate names and CSS4 RebeccaPurple (absent in pinned Java).
        match name.as_str() {
            "business" | "application" | "motivation" | "strategy" | "technology"
            | "physical" | "implementation" => true,
            "rebeccapurple" => false,
            _ => crate::text_render::normalize_color(&name) != name,
        }
    }

    let color = color.strip_prefix('#').unwrap_or(color);
    if matches!(
        color.to_ascii_lowercase().as_str(),
        "transparent" | "background" | "automatic"
    ) || simple(color)
    {
        return true;
    }
    // HColorSet resolves both gradient operands as simple colors. This does
    // not implement gradient painting; an unresolved operand makes it invalid.
    color
        .split_once(['-', '\\\\', '|', '/'])
        .is_some_and(|(first, second)| simple(first) && simple(second))
}
'''
s=s[:pos]+helper+s[pos:]
s=s.replace('''            ParameterCommand::Color(color) => {
                nested.fill = Some(color.to_string());''','''            ParameterCommand::Color(color) => {
                // CommandCreoleColorChange catches NoSuchColorException and
                // keeps fc1 while still consuming/analyzing the command body.
                if is_resolvable_color(color) {
                    nested.fill = Some(color.to_string());
                }''')
s=s.replace('''                    |s| s.background = Some(color.to_string()),''','''                    // CommandCreoleStyle.getExtendedColor uses getColorOrWhite,
                    // unlike ColorChange's inherited-color failure behavior.
                    |s| {
                        s.background = Some(
                            if is_resolvable_color(color) {
                                color
                            } else {
                                "#FFFFFF"
                            }
                            .to_string(),
                        );
                    },''')
s=s.replace('''                nested.monospace = true;
                nested.size = Some(14);
                out.push(Segment {''','''                // AtomImg.create constructs its own blackBlueTrue(monospaced(14))
                // configuration. Reset nested markup, including face, color,
                // decoration, family and position; consumer TextBase inheritance
                // is outside this atom's lexical configuration.
                nested = Style {
                    monospace: true,
                    size: Some(14),
                    fill: Some("#000000".to_string()),
                    ..Style::default()
                };
                out.push(Segment {''')
pos=s.index('    #[test]\n    fn parameter_font_aliases_preserve_shared_consumer_flags')
tests='''    #[test]
    fn color_resolution_is_separate_from_candidate_eligibility() {
        let segments = parse_segments(
            "<color:navy>a<color:#GGHHII>b</color>c<color:#12></color>d</color>e",
        );
        assert_eq!(segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["a", "b", "c", "d", "e"]);
        assert!(segments[..4].iter().all(|s| s.style.fill.as_deref() == Some("navy")));
        assert_eq!(segments[4].style.fill, None);
        assert_eq!(parse_segments("<color:UnknownTone>x</color>")[0].style.fill, None);
        // A hidden marker still fails grammar and leaves literal text, rather
        // than selecting the command and applying its resolution fallback.
        assert_eq!(strip_markup("<color:~#123456>x</color>"), "<color:#123456>x</color>");
        for color in ["red", "#RED", "F", "123", "#112233", "#8044aa77", "BUSINESS", "transparent", "red-blue"] {
            assert!(is_resolvable_color(color), "{color}");
        }
        for color in ["#GGGGGG", "#12", "RebeccaPurple", "UnknownTone", "red-UnknownTone"] {
            assert!(!is_resolvable_color(color), "{color}");
        }
    }

    #[test]
    fn unresolved_background_uses_white_then_restores_outer_style() {
        let segments = parse_segments("<back:blue>a<back:UnknownTone>b</back>c</back>d");
        assert_eq!(segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["a", "b", "c", "d"]);
        assert_eq!(segments.iter().map(|s| s.style.background.as_deref()).collect::<Vec<_>>(), [Some("blue"), Some("#FFFFFF"), Some("blue"), None]);
        assert_eq!(parse_segments("<back:red-UnknownTone>x</back>")[0].style.background.as_deref(), Some("#FFFFFF"));
        assert_eq!(parse_segments("<back:red-blue>x</back>")[0].style.background.as_deref(), Some("red-blue"));
        assert_eq!(parse_segments("<back:#GGGGGG>x</back>")[0].style.background, None);
    }

    #[test]
    fn missing_image_resets_nested_markup_and_restores_its_neighbors() {
        let segments = parse_segments("<b><i><u><s><color:red><back:yellow><font:Serif><size:21><sub>a<img:absent.png>b</sub></size></font></back></color></s></u></i></b>");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].style, segments[2].style);
        assert!(segments[0].style.bold && segments[0].style.italic);
        assert_eq!(segments[1].text, "(Cannot\\u{00a0}decode)");
        assert_eq!(segments[1].style, Style {
            monospace: true,
            size: Some(14),
            fill: Some("#000000".to_string()),
            ..Style::default()
        });
    }

'''
s=s[:pos]+tests+s[pos:];p.write_text(s)
