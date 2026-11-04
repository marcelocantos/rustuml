from pathlib import Path
p=Path('crates/rustuml-parser/src/preprocess/mod.rs');s=p.read_text();s=s.replace('struct PreprocessContext {','''// CommandSpriteSvgMultiline and CommandFactorySprite have different END
// patterns. A declaration owns its body until that command's terminator.
#[derive(Clone, Copy)]
enum SpriteBlockKind {
    Pixel,
    Svg,
}

struct PreprocessContext {''',1).replace('in_sprite_block: bool,','in_sprite_block: Option<SpriteBlockKind>,',1).replace('in_sprite_block: false,','in_sprite_block: None,',1)
a=s.index('        // Collect sprite pixel-data blocks.');b=s.index('        // Handle block comments.',a)
s=s[:a]+'''        // CommandMultilines2#isValid uses END.matcher(trimmed).matches().
        // SVG END is (.*</svg>)$; pixel END is ^end[%s]?sprite|\\}$.
        // Pattern2 is ASCII case-insensitive; %s is Java whitespace plus NBSP.
        if let Some(kind) = self.in_sprite_block {
            static PIXEL_END: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new(r"\\A(?:(?i-u:end)[\\x09-\\x0d \\u{00a0}]?(?i-u:sprite)|\\})\\z")
                    .unwrap()
            });
            static SVG_END: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new(r"\\A[^\\r\\n\\u{0085}\\u{2028}\\u{2029}]*(?i-u:</svg>)\\z").unwrap()
            });
            // StringUtils#trin, as used by Trim.BOTH; do not broaden the end
            // condition to Unicode trimming or a substring search.
            let body = line.trim_matches([' ', '\\t', '\\r', '\\n', '\\0']);
            let ended = match kind {
                SpriteBlockKind::Pixel => PIXEL_END.is_match(body),
                SpriteBlockKind::Svg => SVG_END.is_match(body),
            };
            if ended {
                self.in_sprite_block = None;
                self.current_sprite_name = None;
            } else if let SpriteBlockKind::Pixel = kind
                && let Some(name) = self.current_sprite_name.as_ref()
                && let Some(sprite) = self.sprites.get_mut(name)
                && !trimmed.is_empty()
                && trimmed.chars().all(|c| c.is_ascii_hexdigit())
            {
                sprite.rows.push(trimmed.to_string());
                sprite.height = sprite.rows.len() as u32;
            }
            // Keep one placeholder per owned source line, including the end.
            output.push(String::new());
            return;
        }

'''+s[b:]
a=s.index('        // Parse sprite definitions:');b=s.index('        // Collecting definelong body.',a)
s=s[:a]+'''        // CommandFactorySprite#multiline and CommandSpriteSvg[Multiline]
        // #getRegexConcat: consume complete declarations before family parsing.
        // Keep unsupported sprite materialization visible, but never expose its
        // body as diagram commands. These are command patterns, not a prefix skip.
        static SPRITE_PIXEL: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"\\A(?i-u:sprite)[\\x09-\\x0d \\u{00a0}]+\\$?([-.\\p{L}\\p{N}_]+)[\\x09-\\x0d \\u{00a0}]*(?:\\[([0-9]+)(?i-u:x)([0-9]+)/(?:([0-9]+)(?i-u:z)?|(?i-u:color))\\])?[\\x09-\\x0d \\u{00a0}]*\\{\\z").unwrap()
        });
        static SPRITE_SVG: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"\\A(?i-u:sprite)[\\x09-\\x0d \\u{00a0}]+\\$?([-\\p{L}\\p{N}_]+)[\\x09-\\x0d \\u{00a0}]+((?i-u:<svg)(?-u:\\b)[^\\r\\n\\u{0085}\\u{2028}\\u{2029}]*)\\z").unwrap()
        });
        // Other single-line sprite commands remain unsupported. Match their
        // registered grammar (CommandFactorySprite#singleLine, CommandSprite
        // Stdlib/StdlibSvg/File#getRegexConcat), independent of case/separators.
        static SPRITE_SINGLE: LazyLock<Vec<Regex>> = LazyLock::new(|| {
            [
                r"sprite%s+\\$?([-.%pLN_]+)%s*(?:\\[([0-9]+)x([0-9]+)/(?:([0-9]+)z|color)\\])?%s+([-_A-Za-z0-9]+)",
                r"sprite%s+\\$([-%pLN_]+)%s*#([^#]+)#([^%S]+)",
                r"sprite%s+\\$?([-%pLN_]+)%s*:([^:]+):([^%S]+)",
                r#"sprite%s+\\$?([-%pLN_]+)%s+([^<>"“”\\u{e121}#]*)"#,
            ]
            .iter()
            .map(|pattern| {
                let pattern = pattern
                    .replace("%s", r"[\\x09-\\x0d \\u{00a0}]")
                    .replace("%S", r"\\x09-\\x0d \\u{00a0}")
                    .replace("%pLN", r"\\p{L}\\p{N}");
                // ASCII case folding of the input preserves Unicode names.
                Regex::new(&format!(r"\\A(?:{pattern})\\z")).unwrap()
            })
            .collect()
        });
        // RegexConcat's required space signature is checked over the whole
        // command; FoxSignature omits VT even though Pattern2 %s includes it.
        let sprite_line = line_no_comment.trim_matches([' ', '\\t', '\\r', '\\n', '\\0']);
        let sprite_space = sprite_line.chars().any(|c| {
            matches!(c, ' ' | '\\t' | '\\r' | '\\n' | '\\u{000c}' | '\\u{00a0}')
        });
        if sprite_space && let Some(caps) = SPRITE_PIXEL.captures(sprite_line) {
            let name = caps[1].to_string();
            let (width, height) = parse_sprite_dimensions(sprite_line);
            self.sprites.insert(name.clone(), SpriteData { width, height, rows: Vec::new() });
            self.current_sprite_name = Some(name);
            self.in_sprite_block = Some(SpriteBlockKind::Pixel);
            output.push(String::new());
            return;
        }
        if sprite_space && let Some(caps) = SPRITE_SVG.captures(sprite_line) {
            // CommandSpriteSvg precedes CommandSpriteSvgMultiline: a complete
            // inline SVG never opens a body that could consume later commands.
            if !caps[2].to_ascii_lowercase().ends_with("</svg>") {
                self.in_sprite_block = Some(SpriteBlockKind::Svg);
            }
            eprintln!("warning: SVG sprite declaration consumed; SVG sprite materialization remains unsupported");
            output.push(String::new());
            return;
        }
        if sprite_space && SPRITE_SINGLE.iter().any(|re| re.is_match(&sprite_line.to_ascii_lowercase())) {
            eprintln!("warning: single-line sprite declaration consumed; sprite execution remains unsupported");
            output.push(String::new());
            return;
        }

'''+s[b:];p.write_text(s)
