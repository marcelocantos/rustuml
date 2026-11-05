// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram parser.
//!
//! Parses preprocessed lines into a `SequenceDiagram` model.

use std::sync::LazyLock;

use regex::{Match, Regex};

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::sequence::*;

// Pattern2#QUOTED_REPLACEMENTS expands %s to Java \s plus NBSP;
// compileInternal uses CASE_INSENSITIVE without UNICODE_CHARACTER_CLASS.
// RegexLeaf.spaceOneOrMore/spaceZeroOrMore and sequence command factories all
// use this class. Keep negation consistent for command token boundaries.
fn sequence_regex(pattern: &str) -> Result<Regex, regex::Error> {
    // Pattern2 transforms the token, not a complete bracket expression: this
    // must also work in composite classes such as [^%s:] used for color tokens.
    const JAVA_SPACE_CONTENTS: &str = r"\x09-\x0d \u{00a0}";
    Regex::new(&pattern.replace("%s", JAVA_SPACE_CONTENTS))
}

// RegexConcat#foxRegex includes only direct RegexLeaf children. A mandatory
// [%s]+ leaf contributes FoxSignature.MASK_SPACES; optional/nested leaves do not.
// FoxSignature#getMask recognizes these characters anywhere in the FULL line,
// deliberately excluding VT even though Pattern2's regex whitespace includes it.
fn has_fox_space(line: &str) -> bool {
    line.chars()
        .any(|c| matches!(c, ' ' | '\t' | '\r' | '\n' | '\u{000c}' | '\u{00a0}'))
}

static INLINE_ARROW_STYLE: LazyLock<Regex> = LazyLock::new(|| {
    sequence_regex(
        r"\[((?:#\w+|(?i:dashed|dotted|hidden|bold))(?:,(?:#\w+|(?i:dashed|dotted|hidden|bold)))*)\]",
    )
    .unwrap()
});

static MESSAGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    sequence_regex(
        r#"^("(?:[^"]+)"|\w+)[%s]*([-<>.\\/ox]*(?:\[(?:(?:#\w+|(?i:dashed|dotted|hidden|bold))(?:,(?:#\w+|(?i:dashed|dotted|hidden|bold)))*)\])?[-<>.\\/ox]+)[%s]*("(?:[^"]+)"|\w+)[%s]*(?:((?:\+\+|--|!!))[%s]*(#[^%s]+)?[%s]*)?(?::[%s]*(.*))?$"#,
    )
    .unwrap()
});

static EXTERNAL_MESSAGE_IN_RE: LazyLock<Regex> = LazyLock::new(|| {
    sequence_regex(
        r"^([\[\]])([-<>.\\/ox]*(?:\[(?:(?:#\w+|(?i:dashed|dotted|hidden|bold))(?:,(?:#\w+|(?i:dashed|dotted|hidden|bold)))*)\])?[-<>.\\/ox]+)[%s]*(\w+)[%s]*(?:((?:\+\+|--|!!))[%s]*(#[^%s]+)?[%s]*)?(?::[%s]*(.*))?$",
    )
    .unwrap()
});

static EXTERNAL_MESSAGE_OUT_RE: LazyLock<Regex> = LazyLock::new(|| {
    sequence_regex(
        r"^(\w+)[%s]*([-<>.\\/ox]*(?:\[(?:(?:#\w+|(?i:dashed|dotted|hidden|bold))(?:,(?:#\w+|(?i:dashed|dotted|hidden|bold)))*)\])?[-<>.\\/ox]+)([\[\]])[%s]*(?:((?:\+\+|--|!!))[%s]*(#[^%s]+)?[%s]*)?(?::[%s]*(.*))?$",
    )
    .unwrap()
});

// Pattern2#transform/compileInternal: ASCII case folding and Java's default
// ASCII predefined classes; Unicode identity properties remain Unicode.
pub(super) fn registered_regex(pattern: &str) -> Regex {
    let expanded = pattern
        .replace("%pLN", r"\p{L}\p{N}")
        .replace("%s", r"\x09-\x0d \u{00a0}")
        .replace("%g", "\"\u{201c}\u{201d}\u{e121}")
        .replace("%q", "'\u{2018}\u{2019}")
        .replace(r"\w", "[A-Za-z_0-9]")
        .replace(r"\d", "[0-9]")
        .replace(r"\s", r"[\x09-\x0d ]")
        .replace(r"\S", r"[^\x09-\x0d ]")
        .replace(r"\b", r"(?-u:\b)")
        .replace(r"\<", "<")
        .replace(r"\>", ">")
        .to_ascii_lowercase()
        .replace(r"\p{l}", r"\p{L}")
        .replace(r"\p{n}", r"\p{N}")
        .replace("(?p<", "(?P<");
    // Pattern2 does not enable DOTALL. Java dot excludes all five line
    // terminators, whereas Rust's default dot excludes only LF.
    let mut pattern = String::new();
    let mut escaped = false;
    let mut bracket_depth = 0;
    for ch in expanded.chars() {
        if escaped {
            pattern.push(ch);
            escaped = false;
        } else if ch == '\\' {
            pattern.push(ch);
            escaped = true;
        } else if ch == '[' {
            bracket_depth += 1;
            pattern.push(ch);
        } else if ch == ']' {
            bracket_depth -= 1;
            pattern.push(ch);
        } else if ch == '.' && bracket_depth == 0 {
            pattern.push_str(r"[^\r\n\u{0085}\u{2028}\u{2029}]");
        } else {
            pattern.push(ch);
        }
    }
    Regex::new(&format!(r"\A(?:{pattern})\z")).expect("pinned registered sequence grammar")
}

// StringLocated#getTrimmed -> StringUtils#trin, not Unicode str::trim.
pub(super) fn sequence_command_trim(line: &str) -> &str {
    line.trim_matches([' ', '\t', '\r', '\n', '\0'])
}

// FoxSignature's masks table and getFoxSignatureFromRealString. The required
// masks in the inventory are derived from RegexConcat#foxRegex, not fitted.
pub(super) fn sequence_fox_signature(line: &str) -> u64 {
    const SYMBOLS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0!\"#$%&'()*+,-./:;<=>?@[\\]^_{|}~";
    let mut result = 0;
    for ch in line.chars() {
        if matches!(ch, ' ' | '\t' | '\r' | '\n' | '\u{000c}' | '\u{00a0}') {
            result |= 1;
        } else if let Some(index) = SYMBOLS.find(ch.to_ascii_uppercase()) {
            result |= 1u64 << (index + 2);
            if matches!(ch, '.' | '=' | '-' | '~') {
                result |= 2;
            }
        }
    }
    result
}

enum CommandEnd {
    Single,
    End(Regex),
    Brace,
}

enum RegisteredPattern {
    Plain(Regex),
    Color {
        prefix: Regex,
        suffix: Regex,
        absent: Regex,
    },
}

struct RegisteredSequenceCommand {
    class: &'static str,
    pattern: RegisteredPattern,
    signature: u64,
    end: CommandEnd,
    implemented: bool,
}

impl RegisteredSequenceCommand {
    fn new(
        class: &'static str,
        pattern: &str,
        signature: u64,
        end: CommandEnd,
        implemented: bool,
    ) -> Self {
        let pattern = if let Some((prefix, suffix)) = pattern.split_once("{COLOR}") {
            RegisteredPattern::Color {
                prefix: registered_regex(prefix),
                suffix: registered_regex(suffix),
                absent: registered_regex(&format!("{prefix}{suffix}")),
            }
        } else {
            RegisteredPattern::Plain(registered_regex(pattern))
        };
        Self {
            class,
            pattern,
            signature,
            end,
            implemented,
        }
    }

    // ColorParser.PART2 has one lookahead unavailable in regex::Regex. Split
    // the command at its optional color leaf and enumerate its token boundaries.
    // This preserves the language, including fallback to COLOR_REGEXP, without
    // weakening the lookahead or requiring regex backtracking in Rust.
    fn color_match<'a>(&self, line: &'a str) -> Option<Option<&'a str>> {
        if self.signature & sequence_fox_signature(line) != self.signature {
            return None;
        }
        match &self.pattern {
            RegisteredPattern::Plain(regex) => regex.is_match(line).then_some(None),
            RegisteredPattern::Color {
                prefix,
                suffix,
                absent,
            } => {
                static SIMPLE: LazyLock<Regex> =
                    LazyLock::new(|| registered_regex(r"#\w+[-\\|/]?\w+"));
                // PART2: a nonfinal style must consume ';', because the next
                // style begins with a word character. The final style may end
                // without ';' only when the original negative lookahead holds.
                static COMPOSITE: LazyLock<Regex> = LazyLock::new(|| {
                    registered_regex(
                        r"#(?:\w+[-\\|/]?\w+;)?(?:(?:text|back|header|line|line\.dashed|line\.dotted|line\.bold|shadowing)(?::\w+[-\\|/]?\w+)?;)*(?:text|back|header|line|line\.dashed|line\.dotted|line\.bold|shadowing)(?::\w+[-\\|/]?\w+)?;?",
                    )
                });
                let boundaries = line
                    .char_indices()
                    .map(|(i, _)| i)
                    .chain(std::iter::once(line.len()))
                    .collect::<Vec<_>>();
                for &start in boundaries.iter().rev() {
                    if !line[start..].starts_with('#') || !prefix.is_match(&line[..start]) {
                        continue;
                    }
                    // ColorParser tries PART2 before the simple alternative.
                    for composite in [true, false] {
                        for &end in boundaries.iter().rev().take_while(|&&end| end > start) {
                            let token = &line[start..end];
                            let matches = if composite {
                                COMPOSITE.is_match(token)
                                    && (token.ends_with(';')
                                        || !line[end..].chars().next().is_some_and(|c| {
                                            c.is_ascii_alphanumeric()
                                                || matches!(c, '_' | ';' | ':' | '.')
                                        }))
                            } else {
                                SIMPLE.is_match(token)
                            };
                            if matches && suffix.is_match(&line[end..]) {
                                return Some(Some(token));
                            }
                        }
                    }
                }
                absent.is_match(line).then_some(None)
            }
        }
    }

    fn command_end(&self, lines: &[String], start: usize) -> Option<usize> {
        match &self.end {
            CommandEnd::Single => Some(start),
            CommandEnd::End(regex) => (start + 1..lines.len()).find(|&i| {
                regex.is_match(
                    &sequence_command_trim(super::source_text(&lines[i])).to_ascii_lowercase(),
                )
            }),
            CommandEnd::Brace => {
                // CommandMultilinesBracket#isValid / SkinLoader.p1.
                static CONSISTENT: LazyLock<Regex> = LazyLock::new(|| {
                    registered_regex(r"^([\w.]*(?:\<\<.*\>\>)?[\w.]*)[%s]+(?:(\{)|(.*))$|^\}?$")
                });
                // CommandSkinParamMultilines#hasStartingQuote delegates to
                // CommandMultilinesComment.COMMENT_SINGLE_LINE, including %q.
                static COMMENT: LazyLock<Regex> =
                    LazyLock::new(|| registered_regex(r"^[%s]*([%q].*||/[%q].*[%q]/[%s]*)$"));
                let mut level = 1;
                for (index, raw) in lines.iter().enumerate().skip(start + 1) {
                    let text = sequence_command_trim(super::source_text(raw)).to_ascii_lowercase();
                    if !COMMENT.is_match(&text) && !CONSISTENT.is_match(&text) {
                        return None;
                    }
                    if text.ends_with('{') {
                        level += 1;
                    }
                    if text.ends_with('}') {
                        level -= 1;
                    }
                    if level == 0 {
                        return Some(index);
                    }
                }
                None
            }
        }
    }

    // CODE capture position in CommandParticipantA/A2/A3/A4#getRegexConcat.
    // Match the already selected command's prefix, then project its byte range
    // onto the original (case-preserving) input. ASCII folding preserves offsets.
    fn participant_code<'a>(&self, text: &'a str, color: Option<&str>) -> Option<&'a str> {
        let code_index = match self.class {
            "CommandParticipantA" | "CommandParticipantA3" => 4,
            "CommandParticipantA2" | "CommandParticipantA4" => 3,
            _ => return None,
        };
        let RegisteredPattern::Color { prefix, .. } = &self.pattern else {
            return None;
        };
        let prefix_text = &text[..text.len() - color.map_or(0, str::len)];
        let folded = prefix_text.to_ascii_lowercase();
        let caps = prefix.captures(&folded)?;
        let code = caps.get(code_index)?;
        Some(&text[code.range()])
    }

    fn validate_colors(
        &self,
        line: &str,
        token: Option<&str>,
        source_line: usize,
    ) -> Result<(), ParseError> {
        let bad = |_color: &str| ParseError {
            line: source_line,
            message: "No such color".to_string(),
        };
        // CommandParticipantMultilines#executeNow never reads its COLOR
        // capture. Single-line CommandParticipant#executeArg instead passes the
        // entire token to HColorSet#getColor, not to Colors.
        let token = if self.class == "CommandParticipantMultilines" {
            None
        } else if matches!(
            self.class,
            "CommandParticipantA"
                | "CommandParticipantA2"
                | "CommandParticipantA3"
                | "CommandParticipantA4"
        ) {
            if let Some(token) = token
                && !valid_sequence_color(token)
            {
                return Err(bad(token));
            }
            None
        } else {
            token
        };
        if let Some(token) = token {
            // Colors#Colors removes '#' and splits ';'. Dotted line styles
            // without ':' are styles; shadowing's value is a boolean, not color.
            let data = token.replace('#', "");
            for item in data.split(';').filter(|item| !item.is_empty()) {
                if let Some((name, value)) = item.split_once(':') {
                    if name != "shadowing" && !valid_sequence_color(value) {
                        return Err(bad(value));
                    }
                } else if !item.contains('.') && !valid_sequence_color(item) {
                    return Err(bad(item));
                }
            }
        }
        if let RegisteredPattern::Plain(regex) = &self.pattern
            && let Some(caps) = regex.captures(line)
        {
            for name in regex
                .capture_names()
                .flatten()
                .filter(|name| name.starts_with("seq_color"))
            {
                // CommandReferenceMultilinesOverSeveral#execute reads only
                // REF[0]; its trailing BACKCOLOR capture is not applied.
                if self.class == "CommandReferenceMultilinesOverSeveral" && name == "seq_color1" {
                    continue;
                }
                if let Some(color) = caps.name(name)
                    && !valid_sequence_color(color.as_str())
                {
                    return Err(bad(color.as_str()));
                }
            }
        }
        // CommandArrow#applyStyle is also used by both exo commands. Apart
        // from these four style keywords, each token is passed to HColorSet.
        if let RegisteredPattern::Plain(regex) = &self.pattern
            && let Some(caps) = regex.captures(line)
        {
            for name in regex
                .capture_names()
                .flatten()
                .filter(|name| name.starts_with("seq_style"))
            {
                if let Some(style) = caps.name(name) {
                    for value in style.as_str().split(',') {
                        if !matches!(value, "dashed" | "bold" | "dotted" | "hidden")
                            && !valid_sequence_color(value)
                        {
                            return Err(bad(value));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

// HColorSet#parseColor accepts simple colors and two-color gradients. Command
// color grammars exclude the '?' scheme syntax; return's existing validator is
// retained for its previously supported model input contract.
fn valid_sequence_color(color: &str) -> bool {
    if valid_return_color(color) {
        return true;
    }
    // HColorSet#parseColor resolves gradient endpoints with parseSimpleColor,
    // which excludes the transparent/background/automatic HColor symbols.
    let simple = |part: &str| {
        valid_return_color(part)
            && !matches!(
                part.trim_start_matches('#').to_ascii_lowercase().as_str(),
                "transparent" | "background" | "automatic"
            )
    };
    color.char_indices().any(|(index, ch)| {
        matches!(ch, '-' | '\\' | '|' | '/')
            && simple(&color[..index])
            && simple(&color[index + ch.len_utf8()..])
    })
}

// SequenceDiagramFactory#initCommandsList / CommonCommands#addCommonCommands1,
// pinned 71806a23. Registration order and exact original patterns are preserved
// in sequence-registered-command-rejection/inventory.json (81-command source
// probe). These are source grammars, not fixture data. Account 3b38384b3.
static REGISTERED_SEQUENCE_COMMANDS: LazyLock<Vec<RegisteredSequenceCommand>> = LazyLock::new(
    || {
        vec![
            // 0: net.sourceforge.plantuml.command.CommandTitle
            RegisteredSequenceCommand::new(
                "CommandTitle",
                r###"^title(?:[%s]*:[%s]*|[%s]+)(?:[%g](.*)[%g]|(.*[%pLN_.].*))$"###,
                2106432u64,
                CommandEnd::Single,
                true,
            ),
            // 1: net.sourceforge.plantuml.command.CommandMainframe
            RegisteredSequenceCommand::new(
                "CommandMainframe",
                r###"^mainframe(?:[%s]*:[%s]*|[%s]+)(.*[%pLN_.].*)$"###,
                574660u64,
                CommandEnd::Single,
                false,
            ),
            // 2: net.sourceforge.plantuml.command.CommandCaption
            RegisteredSequenceCommand::new(
                "CommandCaption",
                r###"^caption(?:[%s]*:[%s]*|[%s]+)(?:[%g](.*)[%g]|(.*[%pLN_.].*))$"###,
                2327572u64,
                CommandEnd::Single,
                true,
            ),
            // 3: net.sourceforge.plantuml.command.CommandMultilinesCaption
            RegisteredSequenceCommand::new(
                "CommandMultilinesCaption",
                r###"^caption$"###,
                0u64,
                CommandEnd::End(registered_regex(r###"^end[%s]?caption$"###)),
                false,
            ),
            // 4: net.sourceforge.plantuml.command.CommandMultilinesTitle
            RegisteredSequenceCommand::new(
                "CommandMultilinesTitle",
                r###"^title$"###,
                0u64,
                CommandEnd::End(registered_regex(r###"^end[%s]?title$"###)),
                false,
            ),
            // 5: net.sourceforge.plantuml.command.CommandMultilinesLegend
            RegisteredSequenceCommand::new(
                "CommandMultilinesLegend",
                r###"^legend(?:[%s]+(top|bottom))?(?:[%s]+(left|right|center))?$"###,
                41312u64,
                CommandEnd::End(registered_regex(r###"^end[%s]?legend$"###)),
                true,
            ),
            // 6: net.sourceforge.plantuml.command.CommandLegend
            RegisteredSequenceCommand::new(
                "CommandLegend",
                r###"^legend(?:[%s]*:[%s]*|[%s]+)(?:[%g](.*)[%g]|(.*[%pLN_.].*))$"###,
                41312u64,
                CommandEnd::Single,
                false,
            ),
            // 7: net.sourceforge.plantuml.command.CommandFooter
            RegisteredSequenceCommand::new(
                "CommandFooter",
                r###"^(?:(left|right|center))?[%s]*footer(?:[%s]*:[%s]*|[%s]+)(?:[%g](.*)[%g]|(.*[%pLN_.].*))$"###,
                2687168u64,
                CommandEnd::Single,
                true,
            ),
            // 8: net.sourceforge.plantuml.command.CommandMultilinesFooter
            RegisteredSequenceCommand::new(
                "CommandMultilinesFooter",
                r###"^(?:(left|right|center)?[%s]*)footer$"###,
                0u64,
                CommandEnd::End(registered_regex(r###"^end[%s]?footer$"###)),
                false,
            ),
            // 9: net.sourceforge.plantuml.command.CommandHeader
            RegisteredSequenceCommand::new(
                "CommandHeader",
                r###"^(?:(left|right|center))?[%s]*header(?:[%s]*:[%s]*|[%s]+)(?:[%g](.*)[%g]|(.*[%pLN_.].*))$"###,
                524900u64,
                CommandEnd::Single,
                true,
            ),
            // 10: net.sourceforge.plantuml.command.CommandMultilinesHeader
            RegisteredSequenceCommand::new(
                "CommandMultilinesHeader",
                r###"^(?:(left|right|center)?[%s]*)header$"###,
                0u64,
                CommandEnd::End(registered_regex(r###"^end[%s]?header$"###)),
                false,
            ),
            // 11: net.sourceforge.plantuml.classdiagram.command.CommandNamespaceSeparator
            RegisteredSequenceCommand::new(
                "CommandNamespaceSeparator",
                r###"^set[%s]+(?:separator|namespaceseparator)[%s]+(\S+)$"###,
                3145793u64,
                CommandEnd::Single,
                false,
            ),
            // 12: net.sourceforge.plantuml.command.CommandNope
            RegisteredSequenceCommand::new(
                "CommandNope",
                r###"^[%s]*$"###,
                0u64,
                CommandEnd::Single,
                true,
            ),
            // 13: net.sourceforge.plantuml.command.CommandPragma
            RegisteredSequenceCommand::new(
                "CommandPragma",
                r###"^!pragma[%s]+([A-Za-z_][A-Za-z_0-9]*)(?:[%s]+(.*))?$"###,
                537542917u64,
                CommandEnd::Single,
                true,
            ),
            // 14: net.sourceforge.plantuml.command.CommandAssumeTransparent
            RegisteredSequenceCommand::new(
                "CommandAssumeTransparent",
                r###"^!assume[%s]+transparent[%s]+(dark|light)$"###,
                544915525u64,
                CommandEnd::Single,
                false,
            ),
            // 15: net.sourceforge.plantuml.command.CommandSkinParam
            RegisteredSequenceCommand::new(
                "CommandSkinParam",
                r###"^(skinparam|skinparamlocked)[%s]+([\w.]*(?:\<\<[^<>]*\>\>)?[\w.]*)[%s]+([^{}]*)$"###,
                1758213u64,
                CommandEnd::Single,
                true,
            ),
            // 16: net.sourceforge.plantuml.command.CommandSkinParamJaws
            RegisteredSequenceCommand::new(
                "CommandSkinParamJaws",
                r###"^((skinparam|skinparamlocked)[%s]+.*.*)+$"###,
                0u64,
                CommandEnd::Single,
                false,
            ),
            // 17: net.sourceforge.plantuml.command.CommandSkinParamMultilines
            RegisteredSequenceCommand::new(
                "CommandSkinParamMultilines",
                r###"^skinparam[%s]*(?:[%s]+([\w.]*(?:\<\<.*\>\>)?[\w.]*))?[%s]*\{$"###,
                0u64,
                CommandEnd::Brace,
                true,
            ),
            // 18: net.sourceforge.plantuml.sequencediagram.command.CommandSkin
            RegisteredSequenceCommand::new(
                "CommandSkin",
                r###"^skin[%s]+([\w.]+)$"###,
                1086465u64,
                CommandEnd::Single,
                false,
            ),
            // 19: net.sourceforge.plantuml.command.CommandMinwidth
            RegisteredSequenceCommand::new(
                "CommandMinwidth",
                r###"^minwidth[%s]+(\d+)$"###,
                18925089u64,
                CommandEnd::Single,
                false,
            ),
            // 20: net.sourceforge.plantuml.command.CommandPage
            RegisteredSequenceCommand::new(
                "CommandPage",
                r###"^page[%s]+(\d+)[%s]*x*[%s]*(\d+)$"###,
                131397u64,
                CommandEnd::Single,
                false,
            ),
            // 21: net.sourceforge.plantuml.command.CommandRotate
            RegisteredSequenceCommand::new(
                "CommandRotate",
                r###"^rotate$"###,
                2687044u64,
                CommandEnd::Single,
                false,
            ),
            // 22: net.sourceforge.plantuml.command.CommandFactorySprite$1
            RegisteredSequenceCommand::new(
                "CommandFactorySprite$1",
                r###"^sprite[%s]+\$?([-.%pLN_]+)[%s]*(?:\[(\d+)x(\d+)/(?:(\d+)(z)?|(color))\])?[%s]*\{$"###,
                72057594041730113u64,
                CommandEnd::End(registered_regex(r###"^end[%s]?sprite|\}$"###)),
                false,
            ),
            // 23: net.sourceforge.plantuml.command.CommandFactorySprite$2
            RegisteredSequenceCommand::new(
                "CommandFactorySprite$2",
                r###"^sprite[%s]+\$?([-.%pLN_]+)[%s]*(?:\[(\d+)x(\d+)/(?:(\d+)(z)|(color))\])?[%s]+([-_A-Za-z0-9]+)$"###,
                3802177u64,
                CommandEnd::Single,
                false,
            ),
            // 24: net.sourceforge.plantuml.command.CommandSpriteSvg
            RegisteredSequenceCommand::new(
                "CommandSpriteSvg",
                r###"^sprite[%s]+\$?([-%pLN_]+)[%s]+(\<svg\b.*\</svg\>)$"###,
                360639826101569u64,
                CommandEnd::Single,
                false,
            ),
            // 25: net.sourceforge.plantuml.command.CommandSpriteStdlib
            RegisteredSequenceCommand::new(
                "CommandSpriteStdlib",
                r###"^sprite[%s]+\$([-%pLN_]+)[%s]*#([^#]+)#([^%s]+)$"###,
                4298769473u64,
                CommandEnd::Single,
                false,
            ),
            // 26: net.sourceforge.plantuml.command.CommandSpriteStdlibSvg
            RegisteredSequenceCommand::new(
                "CommandSpriteStdlibSvg",
                r###"^sprite[%s]+\$?([-%pLN_]+)[%s]*:([^:]+):([^%s]+)$"###,
                3802177u64,
                CommandEnd::Single,
                false,
            ),
            // 27: net.sourceforge.plantuml.command.CommandSpriteSvgMultiline
            RegisteredSequenceCommand::new(
                "CommandSpriteSvgMultiline",
                r###"^sprite[%s]+\$?([-%pLN_]+)[%s]+(\<svg\b.*)$"###,
                70368756368705u64,
                CommandEnd::End(registered_regex(r###"(.*\</svg\>)$"###)),
                false,
            ),
            // 28: net.sourceforge.plantuml.command.CommandSpriteFile
            RegisteredSequenceCommand::new(
                "CommandSpriteFile",
                r###"^sprite[%s]+\$?([-%pLN_]+)[%s]+([^<>%g#]*)$"###,
                3802177u64,
                CommandEnd::Single,
                false,
            ),
            // 29: net.sourceforge.plantuml.style.CommandStyleMultilinesCSS
            RegisteredSequenceCommand::new(
                "CommandStyleMultilinesCSS",
                r###"^\<style\>$"###,
                351843791151168u64,
                CommandEnd::End(registered_regex(r###"^[%s]*\</?style\>[%s]*$"###)),
                false,
            ),
            // 30: net.sourceforge.plantuml.style.CommandStyleImport
            RegisteredSequenceCommand::new(
                "CommandStyleImport",
                r###"^\<style[%s]*\w+[%s]*=[%s]*[%q%g]?([^%q%g]*)[%q%g]?\>$"###,
                492581279506498u64,
                CommandEnd::Single,
                false,
            ),
            // 31: net.sourceforge.plantuml.command.CommandScale
            RegisteredSequenceCommand::new(
                "CommandScale",
                r###"^scale[%s]+([0-9.]+)(?:[%s]*/[%s]*([0-9.]+))?$"###,
                1056853u64,
                CommandEnd::Single,
                false,
            ),
            // 32: net.sourceforge.plantuml.command.CommandScaleWidthAndHeight
            RegisteredSequenceCommand::new(
                "CommandScaleWidthAndHeight",
                r###"^scale[%s]+([0-9.]+)[%s]*[*x][%s]*([0-9.]+)$"###,
                1056853u64,
                CommandEnd::Single,
                false,
            ),
            // 33: net.sourceforge.plantuml.command.CommandScaleWidthOrHeight
            RegisteredSequenceCommand::new(
                "CommandScaleWidthOrHeight",
                r###"^scale[%s]+([0-9.]+)[%s]+(width|height)$"###,
                3155541u64,
                CommandEnd::Single,
                false,
            ),
            // 34: net.sourceforge.plantuml.command.CommandScaleMaxWidth
            RegisteredSequenceCommand::new(
                "CommandScaleMaxWidth",
                r###"^scale[%s]+max[%s]+([0-9.]+)[%s]+width$"###,
                53503605u64,
                CommandEnd::Single,
                false,
            ),
            // 35: net.sourceforge.plantuml.command.CommandScaleMaxHeight
            RegisteredSequenceCommand::new(
                "CommandScaleMaxHeight",
                r###"^scale[%s]+max[%s]+([0-9.]+)[%s]+height$"###,
                36726613u64,
                CommandEnd::Single,
                false,
            ),
            // 36: net.sourceforge.plantuml.command.CommandScaleMaxWidthAndHeight
            RegisteredSequenceCommand::new(
                "CommandScaleMaxWidthAndHeight",
                r###"^scale[%s]+max[%s]+([0-9.]+)[%s]*[*x][%s]*([0-9.]+)$"###,
                34627669u64,
                CommandEnd::Single,
                false,
            ),
            // 37: net.sourceforge.plantuml.statediagram.command.CommandHideEmptyDescription
            RegisteredSequenceCommand::new(
                "CommandHideEmptyDescription",
                r###"^(hide|show)[%s]+empty[%s]+description$"###,
                71026289u64,
                CommandEnd::Single,
                false,
            ),
            // 38: net.sourceforge.plantuml.classdiagram.command.CommandHideShowByVisibility
            RegisteredSequenceCommand::new(
                "CommandHideShowByVisibility",
                r###"^(hide|show)[%s]+((?:public|private|protected|package)?(?:[,%s]+(?:public|private|protected|package))*)[%s]+(members?|attributes?|fields?|methods?)$"###,
                577u64,
                CommandEnd::Single,
                false,
            ),
            // 39: net.sourceforge.plantuml.classdiagram.command.CommandHideShowByGender
            RegisteredSequenceCommand::new(
                "CommandHideShowByGender",
                r###"^(hide|show)[%s]+(?:(class|object|interface|enum|annotation|dataclass|record|abstract|[%pLN_.]+|[%g][^%g]+[%g]|\<\<.*\>\>)[%s]+)*?(?:(empty)[%s]+)?(members?|attributes?|fields?|methods?|circles?|circled?|stereotypes?)$"###,
                577u64,
                CommandEnd::Single,
                true,
            ),
            // 40: net.sourceforge.plantuml.sequencediagram.command.CommandHideUnlinked
            RegisteredSequenceCommand::new(
                "CommandHideUnlinked",
                r###"^(hide|show)[%s]+@?unlinked$"###,
                4240993u64,
                CommandEnd::Single,
                false,
            ),
            // 41: net.sourceforge.plantuml.sequencediagram.command.CommandActivate
            RegisteredSequenceCommand::new(
                "CommandActivate",
                r###"^(activate|deactivate|destroy|create)[%s]+([%pLN_.@]+|[%g][^%g]+[%g])[%s]*(?P<seq_color0>#\w+)?(?:[%s]+(?P<seq_color1>#\w+))?$"###,
                2097217u64,
                CommandEnd::Single,
                true,
            ),
            // 42: net.sourceforge.plantuml.sequencediagram.command.CommandDeactivateShort
            RegisteredSequenceCommand::new(
                "CommandDeactivateShort",
                r###"^deactivate[%s]*$"###,
                10486900u64,
                CommandEnd::Single,
                false,
            ),
            // 43: net.sourceforge.plantuml.sequencediagram.command.CommandParticipantA
            RegisteredSequenceCommand::new(
                "CommandParticipantA",
                r###"^(?:(participant|actor|create|boundary|control|entity|queue|database|collections)|create[%s](participant|actor|boundary|control|entity|queue|database|collections))[%s]+(?:[%g]([^%g]+)[%g][%s]+as[%s]+)?([%pLN_.@]+)[%s]*(?:(\<\<.+?\>\>))?[%s]*(?:order[%s]+(-?\d{1,7}))?[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*{COLOR}$"###,
                1u64,
                CommandEnd::Single,
                true,
            ),
            // 44: net.sourceforge.plantuml.sequencediagram.command.CommandParticipantA2
            RegisteredSequenceCommand::new(
                "CommandParticipantA2",
                r###"^(?:(participant|actor|create|boundary|control|entity|queue|database|collections)|create[%s](participant|actor|boundary|control|entity|queue|database|collections))[%s]+([%pLN_.@]+)[%s]+as[%s]+[%g]([^%g]+)[%g][%s]*(?:(\<\<.+?\>\>))?[%s]*(?:order[%s]+(-?\d{1,7}))?[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*{COLOR}$"###,
                1048581u64,
                CommandEnd::Single,
                true,
            ),
            // 45: net.sourceforge.plantuml.sequencediagram.command.CommandParticipantA3
            RegisteredSequenceCommand::new(
                "CommandParticipantA3",
                r###"^(?:(participant|actor|create|boundary|control|entity|queue|database|collections)|create[%s](participant|actor|boundary|control|entity|queue|database|collections))[%s]+([%pLN_.@]+)[%s]+as[%s]+([%pLN_.@]+)[%s]*(?:(\<\<.+?\>\>))?[%s]*(?:order[%s]+(-?\d{1,7}))?[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*{COLOR}$"###,
                1048581u64,
                CommandEnd::Single,
                true,
            ),
            // 46: net.sourceforge.plantuml.sequencediagram.command.CommandParticipantA4
            RegisteredSequenceCommand::new(
                "CommandParticipantA4",
                r###"^(?:(participant|actor|create|boundary|control|entity|queue|database|collections)|create[%s](participant|actor|boundary|control|entity|queue|database|collections))[%s]+[%g]([^%g]+)[%g][%s]*(?:(\<\<.+?\>\>))?[%s]*(?:order[%s]+(-?\d{1,7}))?[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*{COLOR}$"###,
                1u64,
                CommandEnd::Single,
                true,
            ),
            // 47: net.sourceforge.plantuml.sequencediagram.command.CommandParticipantMultilines
            RegisteredSequenceCommand::new(
                "CommandParticipantMultilines",
                r###"^(participant)[%s]+([%pLN_.@]+)[%s]*(?:(\<\<.+?\>\>))?[%s]*(?:order[%s]+(-?\d{1,7}))?[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*{COLOR}[%s]*\[$"###,
                2251799816471573u64,
                CommandEnd::End(registered_regex(r###"^([^\[\]]*)\]$"###)),
                false,
            ),
            // 48: net.sourceforge.plantuml.sequencediagram.command.CommandArrow
            RegisteredSequenceCommand::new(
                "CommandArrow",
                r###"^(&[%s]*)?(\{([%pLN_]+)\}[%s]+)?(([%pLN_.@]+)|[%g]([^%g]+)[%g]|[%g]([^%g]+)[%g][%s]*as[%s]+([%pLN_.@]+)|([%pLN_.@]+)[%s]+as[%s]*[%g]([^%g]+)[%g])(\{([%pLN_]+)\}[%s]+)?[%s]*(?:([%s][ox]|(?:[%s][ox]|\(\d+\))?<<?_?|(?:[%s][ox])?//?|(?:[%s][ox])?\\\\?))?(?:(-+)(?:\[(?P<seq_style0>(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?(-*)|(-*)(?:\[(?P<seq_style1>(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?(-+))(?:(_?>>?(?:[ox][%s]|\(\d+\))?|//?(?:[ox][%s])?|\\\\?(?:[ox][%s])?|[ox][%s]))?[%s]*(([%pLN_.@]+)|[%g]([^%g]+)[%g]|[%g]([^%g]+)[%g][%s]*as[%s]+([%pLN_.@]+)|([%pLN_.@]+)[%s]+as[%s]*[%g]([^%g]+)[%g])((?:\s&\s[%pLN_.@]+)*)(\{([%pLN_]+)\}[%s]+)?[%s]*(?:(\+\+|\*\*|!!|--|--\+\+|\+\+--)?)[%s]*(?:(?P<seq_color0>#\w+)?)[%s]*(?:(\<\<.+?\>\>))?[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*(?::[%s]*(.*))?$"###,
                0u64,
                CommandEnd::Single,
                true,
            ),
            // 49: net.sourceforge.plantuml.sequencediagram.command.CommandExoArrowLeft
            RegisteredSequenceCommand::new(
                "CommandExoArrowLeft",
                r###"^(&[%s]*)?(\{([%pLN_]+)\}[%s]+)?([?\[\]][ox]?)?(?:(<<?|//?|\\\\?)?(-+)(?:\[(?P<seq_style0>(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?(-*)(>>?|//?|\\\\?)|(<<?|//?|\\\\?)(-*)(?:\[(?P<seq_style1>(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?(-+))([ox][%s]+)?[%s]*([%pLN_.@]+|[%g][^%g]+[%g])[%s]*(?:([+*!-]+)?)[%s]*(?:(?P<seq_color0>#\w+)?)[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*(?::[%s]*(.*))?$"###,
                0u64,
                CommandEnd::Single,
                true,
            ),
            // 50: net.sourceforge.plantuml.sequencediagram.command.CommandExoArrowRight
            RegisteredSequenceCommand::new(
                "CommandExoArrowRight",
                r###"^(&[%s]*)?(\{([%pLN_]+)\}[%s]+)?([%pLN_.@]+|[%g][^%g]+[%g])[%s]*([%s]+[ox])?(?:(<<?|//?|\\\\?)?(-+)(?:\[(?P<seq_style0>(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?(-*)(>>?|//?|\\\\?)|(<<?|//?|\\\\?)(-*)(?:\[(?P<seq_style1>(?:#\w+|dotted|dashed|plain|bold|hidden|norank|single|node|thickness=\d+)(?:,#\w+|,dotted|,dashed|,plain|,bold|,hidden|,norank|,single|,node|,thickness=\d+)*)\])?(-+))([ox]?[?\]\[])?[%s]*(?:([+*!-]+)?)[%s]*(?:(?P<seq_color0>#\w+)?)[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*(?::[%s]*(.*))?$"###,
                0u64,
                CommandEnd::Single,
                true,
            ),
            // 51: net.sourceforge.plantuml.command.note.sequence.FactorySequenceNoteCommand$2
            RegisteredSequenceCommand::new(
                "FactorySequenceNoteCommand$2",
                r###"^(&[%s]*)?(/)?[%s]*(note|hnote|rnote)[%s]*(?:(\<\<.+?\>\>))?[%s]*(right|left|over)[%s]+(?:of[%s])?([%pLN_.@]+|[%g][^%g]+[%g])[%s]*(?:(\<\<.+?\>\>))?[%s]*{COLOR}[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*:[%s]*(.*)$"###,
                17592188239937u64,
                CommandEnd::Single,
                true,
            ),
            // 52: net.sourceforge.plantuml.command.note.sequence.FactorySequenceNoteOverSeveralCommand$1
            RegisteredSequenceCommand::new(
                "FactorySequenceNoteOverSeveralCommand$1",
                r###"^(&[%s]*)?(/)?[%s]*(note|hnote|rnote)[%s]*(?:(\<\<.+?\>\>))?[%s]*over[%s]+([%pLN_.@]+|[%g][^%g]+[%g])[%s]*,[%s]*([%pLN_.@]+|[%g][^%g]+[%g])[%s]*(?:(\<\<.+?\>\>))?[%s]*{COLOR}[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*:[%s]*(.*)$"###,
                18691708780609u64,
                CommandEnd::Single,
                true,
            ),
            // 53: net.sourceforge.plantuml.command.note.sequence.FactorySequenceNoteAcrossCommand$1
            RegisteredSequenceCommand::new(
                "FactorySequenceNoteAcrossCommand$1",
                r###"^(/)?[%s]*(note|hnote|rnote)[%s]*(?:(\<\<.+?\>\>))?[%s]*(accross|across)[%s]*(?:(\<\<.+?\>\>))?[%s]*{COLOR}[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*:[%s]*(.*)$"###,
                17592189812820u64,
                CommandEnd::Single,
                true,
            ),
            // 54: net.sourceforge.plantuml.sequencediagram.command.CommandBoxStart
            RegisteredSequenceCommand::new(
                "CommandBoxStart",
                r###"^box(?:(?:[%s]+[%g]([^%g]+)[%g]|[%s]+([^#]+)))?[%s]*(?:(\<\<.+?\>\>))?[%s]*{COLOR}$"###,
                33619976u64,
                CommandEnd::Single,
                true,
            ),
            // 55: net.sourceforge.plantuml.sequencediagram.command.CommandBoxEnd
            RegisteredSequenceCommand::new(
                "CommandBoxEnd",
                r###"^end[%s]*box$"###,
                33652840u64,
                CommandEnd::Single,
                true,
            ),
            // 56: net.sourceforge.plantuml.sequencediagram.command.CommandGrouping
            RegisteredSequenceCommand::new(
                "CommandGrouping",
                r###"^(&[%s]*)?(?:(opt|alt|loop|par|par2|break|critical|group|partition)(?P<seq_color0>#\w+)?|(else|end|also))(?:[%s]+(?P<seq_color1>#\w+))?(?:[%s]+(.*?))?$"###,
                0u64,
                CommandEnd::Single,
                true,
            ),
            // 57: net.sourceforge.plantuml.sequencediagram.command.CommandActivate2
            RegisteredSequenceCommand::new(
                "CommandActivate2",
                r###"^([%pLN_.@]+)[%s]*(\+\+|--)[%s]*(?P<seq_color0>#\w+)?$"###,
                0u64,
                CommandEnd::Single,
                false,
            ),
            // 58: net.sourceforge.plantuml.sequencediagram.command.CommandReturn
            RegisteredSequenceCommand::new(
                "CommandReturn",
                r###"^(&[%s]*)?[%s]*return[%s]*(?:(?P<seq_color0>#\w+)[%s]+)?(.*)$"###,
                6848576u64,
                CommandEnd::Single,
                true,
            ),
            // 59: net.sourceforge.plantuml.command.note.sequence.FactorySequenceNoteOnArrowCommand$1
            RegisteredSequenceCommand::new(
                "FactorySequenceNoteOnArrowCommand$1",
                r###"^[%s]*(note|hnote|rnote)[%s]*(?:(\<\<.+?\>\>))?[%s]*(right|left|bottom|top)[%s]*(?:(\<\<.+?\>\>))?[%s]*{COLOR}[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?[%s]*:[%s]*(.*)$"###,
                17592188239936u64,
                CommandEnd::Single,
                true,
            ),
            // 60: net.sourceforge.plantuml.command.note.sequence.FactorySequenceNoteCommand$1
            RegisteredSequenceCommand::new(
                "FactorySequenceNoteCommand$1",
                r###"^(&[%s]*)?(/)?[%s]*(note|hnote|rnote)[%s]*(?:(\<\<.+?\>\>))?[%s]*(right|left|over)[%s]+(?:of[%s]+)?([%pLN_.@]+|[%g][^%g]+[%g])[%s]*(?:(\<\<.+?\>\>))?[%s]*{COLOR}[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?$"###,
                2195521u64,
                CommandEnd::End(registered_regex(r###"^end[%s]?(note|hnote|rnote)$"###)),
                true,
            ),
            // 61: net.sourceforge.plantuml.command.note.sequence.FactorySequenceNoteOverSeveralCommand$2
            RegisteredSequenceCommand::new(
                "FactorySequenceNoteOverSeveralCommand$2",
                r###"^(&[%s]*)?(/)?[%s]*(note|hnote|rnote)[%s]*(?:(\<\<.+?\>\>))?[%s]*over[%s]+([%pLN_.@]+|[%g][^%g]+[%g])[%s]*,[%s]*([%pLN_.@]+|[%g][^%g]+[%g])[%s]*(?:(\<\<.+?\>\>))?[%s]*{COLOR}[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?$"###,
                1099522736193u64,
                CommandEnd::End(registered_regex(r###"^end[%s]?(note|hnote|rnote)$"###)),
                true,
            ),
            // 62: net.sourceforge.plantuml.command.note.sequence.FactorySequenceNoteOnArrowCommand$2
            RegisteredSequenceCommand::new(
                "FactorySequenceNoteOnArrowCommand$2",
                r###"^[%s]*(note|hnote|rnote)[%s]*(?:(\<\<.+?\>\>))?[%s]*(right|left|bottom|top)[%s]*(?:(\<\<.+?\>\>))?[%s]*{COLOR}[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?$"###,
                2195520u64,
                CommandEnd::End(registered_regex(r###"^[%s]*end[%s]?note$"###)),
                true,
            ),
            // 63: net.sourceforge.plantuml.command.note.sequence.FactorySequenceNoteAcrossCommand$2
            RegisteredSequenceCommand::new(
                "FactorySequenceNoteAcrossCommand$2",
                r###"^(/)?[%s]*(note|hnote|rnote)[%s]*(?:(\<\<.+?\>\>))?[%s]*(accross|across)[%s]*(?:(\<\<.+?\>\>))?[%s]*{COLOR}[%s]*(?:(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]))?$"###,
                3768404u64,
                CommandEnd::End(registered_regex(r###"^end[%s]?(note|hnote|rnote)$"###)),
                true,
            ),
            // 64: net.sourceforge.plantuml.sequencediagram.command.CommandNewpage
            RegisteredSequenceCommand::new(
                "CommandNewpage",
                r###"^@?newpage(?:(?:[%s]*:[%s]*|[%s]+)(.*[%pLN_.].*))?$"###,
                16941380u64,
                CommandEnd::Single,
                true,
            ),
            // 65: net.sourceforge.plantuml.sequencediagram.command.CommandIgnoreNewpage
            RegisteredSequenceCommand::new(
                "CommandIgnoreNewpage",
                r###"^ignore[%s]*newpage$"###,
                17532228u64,
                CommandEnd::Single,
                false,
            ),
            // 66: net.sourceforge.plantuml.sequencediagram.command.CommandAutoNewpage
            RegisteredSequenceCommand::new(
                "CommandAutoNewpage",
                r###"^autonewpage[%s]+(\d+)$"###,
                23298373u64,
                CommandEnd::Single,
                false,
            ),
            // 67: net.sourceforge.plantuml.sequencediagram.command.CommandDivider
            RegisteredSequenceCommand::new(
                "CommandDivider",
                r###"^==[%s]*(.*)[%s]*==$"###,
                140737488355330u64,
                CommandEnd::Single,
                true,
            ),
            // 68: net.sourceforge.plantuml.sequencediagram.command.CommandHSpace
            RegisteredSequenceCommand::new(
                "CommandHSpace",
                r###"^\|\|(\d+)?\|+$"###,
                144115188075855872u64,
                CommandEnd::Single,
                true,
            ),
            // 69: net.sourceforge.plantuml.sequencediagram.command.CommandReferenceOverSeveral
            RegisteredSequenceCommand::new(
                "CommandReferenceOverSeveral",
                r###"^ref(?P<seq_color0>#\w+)?[%s]+over[%s]+(([%pLN_.@]+|[%g][^%g]+[%g])([%s]*,[%s]*([%pLN_.@]+|[%g][^%g]+[%g]))*)[%s]*(?:(\[\[.*?\]\]))?[%s]*:[%s]*(.*)$"###,
                17592195023041u64,
                CommandEnd::Single,
                true,
            ),
            // 70: net.sourceforge.plantuml.sequencediagram.command.CommandReferenceMultilinesOverSeveral
            RegisteredSequenceCommand::new(
                "CommandReferenceMultilinesOverSeveral",
                r###"^ref(?P<seq_color0>#\w+)?[%s]+over[%s]+((?:[%pLN_.@]+|[%g][^%g]+[%g])(?:[%s]*,[%s]*(?:[%pLN_.@]+|[%g][^%g]+[%g]))*)[%s]*(?:(\[\[.*?\]\]))?[%s]*(?P<seq_color1>#\w+)?$"###,
                0u64,
                CommandEnd::End(registered_regex(r###"^end[%s]?(ref)?$"###)),
                true,
            ),
            // 71: net.sourceforge.plantuml.sequencediagram.command.CommandAutonumber
            RegisteredSequenceCommand::new(
                "CommandAutonumber",
                r###"^autonumber[%s]*(\d(?:(?:[^%pLN%s]+|\d+)*\d)?)?(?:[%s]+(\d+))?(?:[%s]+[%g]([^%g]+)[%g])?[%s]*$"###,
                6930508u64,
                CommandEnd::Single,
                true,
            ),
            // 72: net.sourceforge.plantuml.sequencediagram.command.CommandAutonumberStop
            RegisteredSequenceCommand::new(
                "CommandAutonumberStop",
                r###"^autonumber[%s]+stop[%s]*$"###,
                8110157u64,
                CommandEnd::Single,
                true,
            ),
            // 73: net.sourceforge.plantuml.sequencediagram.command.CommandAutonumberResume
            RegisteredSequenceCommand::new(
                "CommandAutonumberResume",
                r###"^autonumber[%s]+resume[%s]*(?:[%s]+(\d+))?(?:[%s]+[%g]([^%g]+)[%g])?$"###,
                7979085u64,
                CommandEnd::Single,
                true,
            ),
            // 74: net.sourceforge.plantuml.sequencediagram.command.CommandAutonumberIncrement
            RegisteredSequenceCommand::new(
                "CommandAutonumberIncrement",
                r###"^autonumber[%s]+inc(?:[%s]+([A-Za-z]))?[%s]*$"###,
                6931549u64,
                CommandEnd::Single,
                false,
            ),
            // 75: net.sourceforge.plantuml.sequencediagram.command.CommandAutoactivate
            RegisteredSequenceCommand::new(
                "CommandAutoactivate",
                r###"^autoactivate[%s]+(off|on)$"###,
                14746709u64,
                CommandEnd::Single,
                true,
            ),
            // 76: net.sourceforge.plantuml.sequencediagram.command.CommandFootbox
            RegisteredSequenceCommand::new(
                "CommandFootbox",
                r###"^(hide|show)?[%s]*footbox$"###,
                35717256u64,
                CommandEnd::Single,
                true,
            ),
            // 77: net.sourceforge.plantuml.sequencediagram.command.CommandDelay
            RegisteredSequenceCommand::new(
                "CommandDelay",
                r###"^(?:\.{3}|…)(?:(.*)(?:\.{3}|…))?$"###,
                0u64,
                CommandEnd::Single,
                true,
            ),
            // 78: net.sourceforge.plantuml.sequencediagram.command.CommandFootboxOld
            RegisteredSequenceCommand::new(
                "CommandFootboxOld",
                r###"^footbox[%s]*(on|off)?[%s]*$"###,
                35717256u64,
                CommandEnd::Single,
                true,
            ),
            // 79: net.sourceforge.plantuml.sequencediagram.command.CommandUrl
            RegisteredSequenceCommand::new(
                "CommandUrl",
                r###"^url[%s]*(?:of|for)?[%s]+([%pLN_.@]+|[%g][^%g]+[%g])[%s]+(?:is)?[%s]*(\[\[[%s]*[%g]([^%g]+)[%g](?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\]|\[\[[%s]*\{(.*)\}[%s]*\]\]|\[\[[%s]*\{([^{}]*)\}[%s]*([^\[%s\{\}\[\]][^\[\]]*)[%s]*\]\]|\[\[[%s]*([^\s%g{}\[\]]+?)[%s]*\{(.+)\}[%s]*\]\]|\[\[[%s]*([^%s%g\[\]]+?)(?:[%s]*\{([^{}]*)\})?(?:[%s]([^%s\{\}\[\]][^\[\]]*))?[%s]*\]\])$"###,
                4726785u64,
                CommandEnd::Single,
                false,
            ),
            // 80: net.sourceforge.plantuml.sequencediagram.command.CommandLinkAnchor
            RegisteredSequenceCommand::new(
                "CommandLinkAnchor",
                r###"^\{([%pLN_]+)\}[%s]*\<-\>[%s]*\{([%pLN_]+)\}[%s]*(?::[%s]*(.*))?$"###,
                354042744143874u64,
                CommandEnd::Single,
                false,
            ),
        ]
    },
);

/// Parse preprocessed lines into a sequence diagram.
/// The compiled start pattern, signature and END of a registered command.
#[cfg(test)]
pub(super) fn registered_grammar(class: &str) -> Option<(String, u64, Option<String>)> {
    let command = REGISTERED_SEQUENCE_COMMANDS
        .iter()
        .find(|command| command.class == class)?;
    let RegisteredPattern::Plain(start) = &command.pattern else {
        return None;
    };
    let end = match &command.end {
        CommandEnd::End(end) => Some(end.as_str().to_string()),
        CommandEnd::Single | CommandEnd::Brace => None,
    };
    Some((start.as_str().to_string(), command.signature, end))
}

pub fn parse_sequence(lines: &[String]) -> Result<SequenceDiagram, ParseError> {
    let mut parser = SeqParser::new();

    let mut index = 0;
    while index < lines.len() {
        let line = &lines[index];
        let (source_line, _) = super::source_line_and_trimmed(index + 1, line);
        let text = sequence_command_trim(super::source_text(line));
        let folded = text.to_ascii_lowercase();
        // PSystemCommandFactory#getCandidate tries the registered commands in
        // order, including completion of multiline candidates before selection.
        let candidate = REGISTERED_SEQUENCE_COMMANDS.iter().find_map(|command| {
            let color = command.color_match(&folded)?;
            let end = command.command_end(lines, index)?;
            Some((command, color, end))
        });
        let Some((command, color, end)) = candidate else {
            return Err(ParseError {
                line: source_line,
                message: format!("Syntax error: no registered sequence command accepts {text:?}"),
            });
        };
        // CommandParticipant#executeArg returns before color/stereotype/create
        // execution when CODE already exists. CommandActivate's separate create
        // path does not take this return and retains repeated creation events.
        let existing_participant = command
            .participant_code(text, color)
            .is_some_and(|code| parser.participant_ids.iter().any(|id| id == code));
        if !existing_participant {
            command.validate_colors(&folded, color, source_line)?;
        } else if let Some(code) = command.participant_code(text, color) {
            parser.report_put_participant_in_last(code, source_line);
        }
        // CommandGrouping#executeArg: COLORS[1] is backColorGeneral, the
        // colour of the whole group area; only COLORS[0] reaches the model.
        if command.class == "CommandGrouping"
            && let RegisteredPattern::Plain(regex) = &command.pattern
            && regex
                .captures(&folded)
                .is_some_and(|caps| caps.name("seq_color1").is_some())
        {
            eprintln!(
                "warning: sequence group second colour (CommandGrouping backColorGeneral) is not painted as the group background at line {source_line}"
            );
        }
        if !existing_participant
            && command.class != "CommandParticipantMultilines"
            && color.is_some_and(|token| token.contains([':', ';', '/', '\\', '|', '-']))
        {
            eprintln!(
                "warning: sequence command {} has an execution-valid composite/gradient color; complete style materialization remains unsupported at line {source_line}",
                command.class
            );
        }
        // CommonCommands sprite commands: the registry above selected the
        // command after TIM; its executeNow/executeArg failures are errors.
        // Sequence rendering does not draw sprite references, so execution
        // still reports the command as unimplemented below.
        if command.class.starts_with("CommandFactorySprite")
            || command.class.starts_with("CommandSprite")
        {
            super::sprite::execute_selected(lines, index, end, command.class, &mut parser.meta)
                .map_err(|message| ParseError {
                    line: source_line,
                    message,
                })?;
        }
        if !command.implemented {
            eprintln!(
                "warning: sequence command {} is recognized but unimplemented at line {source_line}: {text}",
                command.class
            );
        } else if command.class != "CommandNope" && !existing_participant {
            let supported = parser.apply_registered(command.class, source_line, text)?;
            if !supported {
                eprintln!(
                    "warning: sequence command {} variant is recognized but unimplemented at line {source_line}: {text}",
                    command.class
                );
            } else {
                for (offset, line) in lines[index + 1..end + 1].iter().enumerate() {
                    let (number, trimmed) =
                        super::source_line_and_trimmed(index + offset + 2, line);
                    let content = if parser.note_buffer.is_some() {
                        super::source_text(line)
                    } else {
                        trimmed
                    };
                    parser.unconsumed = false;
                    // CommandReferenceMultilinesOverSeveral's terminator is
                    // end[%s]?(ref)?; the admitted terminator owns the close.
                    if index + offset + 1 == end && parser.ref_buffer.is_some() {
                        parser.parse_line(number, "end ref")?;
                    } else if parser.in_legend {
                        // The selected multiline command owns every body line.
                        parser.try_meta(if index + offset + 1 == end {
                            "endlegend"
                        } else {
                            content
                        });
                    } else {
                        parser.parse_line(number, content)?;
                    }
                    if parser.unconsumed {
                        eprintln!(
                            "warning: sequence command {} variant is recognized but unimplemented at line {number}: {content}",
                            command.class
                        );
                    }
                }
            }
        }
        index = end + 1;
    }

    // PSystemCommandFactory#getCandidate requires a completed multiline
    // note command; an open buffer is not a successfully consumed command.
    if let Some(note) = &parser.note_buffer {
        return Err(ParseError {
            line: note.source_line,
            message: "Unclosed sequence note".to_string(),
        });
    }
    validate_return_destinations(&parser.events)?;
    Ok(parser.finish())
}

/// Bounded factory viability for recognized decorated messages. Java's
/// PSystemBuilder tries Sequence before Class; PSystemCommandFactory rejects a
/// candidate on its first unconsumed command. Reuse the actual sequence parser
/// and retain the existing fallback for unsupported commands (account 5b45a172).
pub(super) fn consumes_decorated_sequence(lines: &[String]) -> bool {
    let mut parser = SeqParser::new();
    let mut decorated = false;
    for (index, line) in lines.iter().enumerate() {
        let (number, text) = super::source_line_and_trimmed(index + 1, line);
        if text.is_empty()
            || (text.starts_with("@startuml") || text == "@enduml")
            || text.starts_with('\'')
        {
            continue;
        }
        let before = parser.events.len();
        if parser.parse_line(number, text).is_err() || parser.unconsumed {
            return false;
        }
        if let Some(Event::Message(message)) = parser.events.get(before) {
            // CommandArrow requires a head/cross dressing; bare class/object
            // aggregation has no viable sequence command, even though our
            // historical permissive message regex recognizes its body.
            let text = text.strip_prefix('&').unwrap_or(text).trim_start();
            let Some(caps) = MESSAGE_RE
                .captures(text)
                .or_else(|| EXTERNAL_MESSAGE_IN_RE.captures(text))
                .or_else(|| EXTERNAL_MESSAGE_OUT_RE.captures(text))
            else {
                return false;
            };
            let (_, arrow) = strip_arrow_style_annotation(&caps[2]);
            if !arrow.contains(['<', '>', '/', '\\', 'x']) {
                return false;
            }
            decorated |= message.arrow.source_circle || message.arrow.target_circle;
        }
    }
    decorated
        && parser.note_buffer.is_none()
        && parser.ref_buffer.is_none()
        && !parser.in_legend
        && parser.skinparam_block_prefix.is_none()
        && validate_return_destinations(&parser.events).is_ok()
}

// CommandReturn#executeArg calls HColorSet#getColor; an unknown name is a
// command error, not label text or an SVG color. Account ea313c62.
fn validate_return_destinations(events: &[Event]) -> Result<(), ParseError> {
    for event in events {
        if let Event::Return(ret) = event
            && let Some(color) = &ret.color
            && !valid_return_color(color)
        {
            return Err(ParseError {
                line: ret.source_line,
                message: "No such color".to_owned(),
            });
        }
    }
    let lowered =
        resolve_sequence_returns(events).map_err(|(line, message)| ParseError { line, message })?;
    sequence_create_attachments(&lowered)
        .map(|_| ())
        .map_err(|(line, message)| ParseError { line, message })
}

// Display#getWithNewlines expands inline note/participant escapes before the component
// measures or draws them. Raw math/link bodies retain their backslashes.
// Block commands use Display.create and do not run this inline conversion.
fn inline_sequence_display(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    let mut raw = false;
    while let Some((index, c)) = chars.next() {
        let rest = &text[index..];
        if rest.starts_with("<math>") || rest.starts_with("<latex>") || rest.starts_with("[[") {
            raw = true;
        } else if rest.starts_with("</math>")
            || rest.starts_with("</latex>")
            || rest.starts_with("]]")
        {
            raw = false;
        }
        if !raw && c == '\\' {
            match chars.peek().map(|(_, c)| *c) {
                Some('n') => {
                    chars.next();
                    result.push('\n');
                }
                Some('t') => {
                    chars.next();
                    result.push('\t');
                }
                Some('\\') => {
                    chars.next();
                    result.push('\\');
                }
                _ => result.push(c),
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn valid_return_color(color: &str) -> bool {
    let color = color.strip_prefix('#').unwrap_or(color);
    // HColorSet#parseSimpleColor accepts 1/3/6/8 hexadecimal digits.
    if matches!(color.len(), 1 | 3 | 6 | 8) && color.bytes().all(|b| b.is_ascii_hexdigit()) {
        return true;
    }
    // ColorTrieNode's registered names, case-insensitive, plus HColorSet's
    // transparent/background/automatic symbolic colors. Pinned Java 71806a23.
    matches!(
        color.to_ascii_lowercase().as_str(),
        "aliceblue"
            | "antiquewhite"
            | "application"
            | "aqua"
            | "aquamarine"
            | "automatic"
            | "azure"
            | "background"
            | "beige"
            | "bisque"
            | "black"
            | "blanchedalmond"
            | "blue"
            | "blueviolet"
            | "brown"
            | "burlywood"
            | "business"
            | "cadetblue"
            | "chartreuse"
            | "chocolate"
            | "coral"
            | "cornflowerblue"
            | "cornsilk"
            | "crimson"
            | "cyan"
            | "darkblue"
            | "darkcyan"
            | "darkgoldenrod"
            | "darkgray"
            | "darkgreen"
            | "darkgrey"
            | "darkkhaki"
            | "darkmagenta"
            | "darkolivegreen"
            | "darkorange"
            | "darkorchid"
            | "darkred"
            | "darksalmon"
            | "darkseagreen"
            | "darkslateblue"
            | "darkslategray"
            | "darkslategrey"
            | "darkturquoise"
            | "darkviolet"
            | "deeppink"
            | "deepskyblue"
            | "dimgray"
            | "dimgrey"
            | "dodgerblue"
            | "firebrick"
            | "floralwhite"
            | "forestgreen"
            | "fuchsia"
            | "gainsboro"
            | "ghostwhite"
            | "gold"
            | "goldenrod"
            | "gray"
            | "green"
            | "greenyellow"
            | "grey"
            | "honeydew"
            | "hotpink"
            | "implementation"
            | "indianred"
            | "indigo"
            | "ivory"
            | "khaki"
            | "lavender"
            | "lavenderblush"
            | "lawngreen"
            | "lemonchiffon"
            | "lightblue"
            | "lightcoral"
            | "lightcyan"
            | "lightgoldenrodyellow"
            | "lightgray"
            | "lightgreen"
            | "lightgrey"
            | "lightpink"
            | "lightsalmon"
            | "lightseagreen"
            | "lightskyblue"
            | "lightslategray"
            | "lightslategrey"
            | "lightsteelblue"
            | "lightyellow"
            | "lime"
            | "limegreen"
            | "linen"
            | "magenta"
            | "maroon"
            | "mediumaquamarine"
            | "mediumblue"
            | "mediumorchid"
            | "mediumpurple"
            | "mediumseagreen"
            | "mediumslateblue"
            | "mediumspringgreen"
            | "mediumturquoise"
            | "mediumvioletred"
            | "midnightblue"
            | "mintcream"
            | "mistyrose"
            | "moccasin"
            | "motivation"
            | "navajowhite"
            | "navy"
            | "oldlace"
            | "olive"
            | "olivedrab"
            | "orange"
            | "orangered"
            | "orchid"
            | "palegoldenrod"
            | "palegreen"
            | "paleturquoise"
            | "palevioletred"
            | "papayawhip"
            | "peachpuff"
            | "peru"
            | "physical"
            | "pink"
            | "plum"
            | "powderblue"
            | "purple"
            | "red"
            | "rosybrown"
            | "royalblue"
            | "saddlebrown"
            | "salmon"
            | "sandybrown"
            | "seagreen"
            | "seashell"
            | "sienna"
            | "silver"
            | "skyblue"
            | "slateblue"
            | "slategray"
            | "slategrey"
            | "snow"
            | "springgreen"
            | "steelblue"
            | "strategy"
            | "tan"
            | "teal"
            | "technology"
            | "thistle"
            | "tomato"
            | "transparent"
            | "turquoise"
            | "violet"
            | "wheat"
            | "white"
            | "whitesmoke"
            | "yellow"
            | "yellowgreen"
    )
}

struct SeqParser {
    meta: DiagramMeta,
    participants: Vec<Participant>,
    events: Vec<Event>,
    autonumber: Option<AutoNumber>,
    participant_ids: Vec<String>,
    /// Multiline note accumulator.
    note_buffer: Option<NoteBuffer>,
    /// Multiline ref accumulator.
    ref_buffer: Option<RefBuffer>,
    /// Last message source and target (for bare "note left"/"note right").
    last_message: Option<(String, String)>,
    /// Whether we are inside a `legend ... endlegend` block.
    in_legend: bool,
    /// Whether `hide footbox` was specified.
    hide_footbox: bool,
    hide_stereotypes: bool,
    /// Whether `!pragma teoz true` was specified.
    teoz: bool,
    /// Whether `autoactivate on` is active for subsequent messages.
    autoactivate: bool,
    /// Current 1-based source line number (set before each parse_line call).
    current_line: usize,
    /// PSystemCommandFactory viability: an unknown command rejects a candidate.
    unconsumed: bool,
    /// Named participant boxes (`box ... end box`).
    boxes: Vec<ParticipantBox>,
    /// Index into `boxes` of the box currently being collected, if any.
    current_box: Option<usize>,
    /// Prefix of an open `skinparam X { ... }` block.
    skinparam_block_prefix: Option<String>,
}

struct NoteBuffer {
    position: NotePosition,
    participants: Vec<String>,
    lines: Vec<String>,
    shape: NoteShape,
    color: Option<String>,
    on_message: bool,
    source_line: usize,
}

struct RefBuffer {
    participants: Vec<String>,
    lines: Vec<String>,
    source_line: usize,
}

impl SeqParser {
    fn new() -> Self {
        Self {
            meta: DiagramMeta::default(),
            participants: Vec::new(),
            events: Vec::new(),
            autonumber: None,
            participant_ids: Vec::new(),
            note_buffer: None,
            ref_buffer: None,
            last_message: None,
            in_legend: false,
            hide_footbox: false,
            hide_stereotypes: false,
            teoz: false,
            autoactivate: false,
            current_line: 0,
            unconsumed: false,
            boxes: Vec::new(),
            current_box: None,
            skinparam_block_prefix: None,
        }
    }

    /// CommandParticipant#executeArg returns early for an existing CODE but
    /// first calls SequenceDiagram#putParticipantInLast: the participant is
    /// re-added with addWithOrder and takes the current box (englober).
    fn report_put_participant_in_last(&self, code: &str, source_line: usize) {
        let Some(index) = self.participants.iter().position(|p| p.id == code) else {
            return;
        };
        let key = |i: usize| (self.participants[i].order.unwrap_or(i), i);
        let reorders = (0..self.participants.len()).any(|i| i != index && key(i) > key(index));
        let recorded_box = self.boxes.iter().position(|b| b.members.contains(&index));
        if reorders || recorded_box != self.current_box {
            eprintln!(
                "warning: sequence participant re-declaration moves {code:?} to last and into the current box in Java (SequenceDiagram#putParticipantInLast); this reorder is not modelled at line {source_line}"
            );
        }
    }

    fn finish(self) -> SequenceDiagram {
        if self.teoz && self.participants.iter().any(|p| p.url.is_some()) {
            eprintln!(
                "warning: teoz participant link anchors ([[url]]) are not emitted; the links are dropped from the SVG"
            );
        }
        SequenceDiagram {
            meta: self.meta,
            participants: self.participants,
            events: self.events,
            autonumber: self.autonumber,
            hide_footbox: self.hide_footbox,
            hide_stereotypes: self.hide_stereotypes,
            teoz: self.teoz,
            boxes: self.boxes,
        }
    }

    fn ensure_participant(&mut self, id: &str) -> String {
        self.ensure_participant_at(id, self.current_line)
    }

    fn ensure_participant_at(&mut self, id: &str, source_line: usize) -> String {
        let id = id.trim().to_string();
        if !self.participant_ids.contains(&id) {
            self.participant_ids.push(id.clone());
            self.participants.push(Participant {
                id: id.clone(),
                label: inline_sequence_display(&id),
                kind: ParticipantKind::default(),
                order: Some(self.participants.len()),
                stereotype: None,
                stereotype_position_bottom: false,
                url: None,
                url_tooltip: None,
                color: None,
                source_line,
            });
        }
        id
    }

    // Execute only the handler belonging to the selected Java command. A
    // permissive, unrelated Rust handler cannot masquerade as its support.
    fn apply_registered(
        &mut self,
        class: &str,
        line_num: usize,
        line: &str,
    ) -> Result<bool, ParseError> {
        self.current_line = line_num;
        Ok(match class {
            "CommandTitle"
            | "CommandCaption"
            | "CommandFooter"
            | "CommandHeader"
            | "CommandMultilinesLegend" => self.try_meta(line),
            "CommandPragma" => self.try_pragma(line),
            "CommandSkinParam" | "CommandSkinParamMultilines" => self.try_skinparam(line),
            "CommandHideShowByGender" | "CommandFootbox" | "CommandFootboxOld" => {
                self.try_hide(line)
            }
            "CommandActivate" => {
                self.try_activate_deactivate(line) || self.try_create_destroy(line)
            }
            "CommandParticipantA"
            | "CommandParticipantA2"
            | "CommandParticipantA3"
            | "CommandParticipantA4" => {
                // CommandParticipant#getRegexType includes typed CREATE and
                // bare CREATE declarations with options; only a complete
                // declaration can create a new participant and creation event.
                static CREATE: LazyLock<Regex> =
                    LazyLock::new(|| sequence_regex(r"^(?i-u:create)[%s]+(.*)$").unwrap());
                if let Some(caps) = CREATE.captures(line) {
                    let tail = &caps[1];
                    let declaration = if [
                        "participant",
                        "actor",
                        "boundary",
                        "control",
                        "entity",
                        "database",
                        "collections",
                        "queue",
                    ]
                    .iter()
                    .any(|kind| {
                        tail.get(..kind.len())
                            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(kind))
                            && tail[kind.len()..].starts_with(|c: char| {
                                matches!(
                                    c,
                                    ' ' | '\t' | '\r' | '\n' | '\u{000b}' | '\u{000c}' | '\u{00a0}'
                                )
                            })
                    }) {
                        tail.to_owned()
                    } else {
                        format!("participant {tail}")
                    };
                    let before = self.participants.len();
                    if self.try_participant_decl_in_command(&declaration, line) {
                        if let Some(participant) = self.participants.get(before) {
                            self.events.push(Event::Create(participant.id.clone()));
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    self.try_participant_decl(line)
                }
            }
            "CommandArrow" => self.try_message(line),
            "CommandExoArrowLeft" | "CommandExoArrowRight" => self.try_external_message(line),
            "FactorySequenceNoteCommand$1"
            | "FactorySequenceNoteCommand$2"
            | "FactorySequenceNoteOverSeveralCommand$1"
            | "FactorySequenceNoteOverSeveralCommand$2"
            | "FactorySequenceNoteAcrossCommand$1"
            | "FactorySequenceNoteAcrossCommand$2"
            | "FactorySequenceNoteOnArrowCommand$1"
            | "FactorySequenceNoteOnArrowCommand$2" => self.try_note(line)?,
            "CommandBoxStart" | "CommandBoxEnd" => self.try_box(line),
            "CommandGrouping" => self.try_group(line),
            "CommandReturn" => self.try_return(line),
            "CommandNewpage" => self.try_newpage(line),
            "CommandDivider" => self.try_divider(line),
            "CommandHSpace" => self.try_space(line),
            "CommandReferenceOverSeveral" | "CommandReferenceMultilinesOverSeveral" => {
                self.try_ref(line)
            }
            "CommandAutonumber" | "CommandAutonumberStop" | "CommandAutonumberResume" => {
                self.try_autonumber(line)
            }
            "CommandAutoactivate" => self.try_autoactivate(line),
            "CommandDelay" => self.try_delay(line),
            _ => false,
        })
    }

    fn parse_line(&mut self, line_num: usize, line: &str) -> Result<(), ParseError> {
        self.current_line = line_num;

        if let Some(prefix) = self.skinparam_block_prefix.clone() {
            if line == "}" {
                self.skinparam_block_prefix = None;
            } else if let Some((key, value)) = line.split_once(char::is_whitespace) {
                let key = key.trim();
                let value = value.trim();
                if !key.is_empty() && !value.is_empty() {
                    self.meta.skinparams.push(crate::diagram::SkinParam {
                        key: format!("{prefix}{key}"),
                        value: value.to_string(),
                    });
                }
            }
            return Ok(());
        }

        // Handle multiline ref buffering.
        if self.ref_buffer.is_some() {
            if line == "end ref" {
                let buf = self.ref_buffer.take().unwrap();
                let text = buf.lines.join("\n");
                self.events.push(Event::Ref(Ref {
                    participants: buf.participants,
                    text,
                    source_line: buf.source_line,
                }));
            } else if let Some(buf) = &mut self.ref_buffer {
                buf.lines.push(line.trim().to_string());
            }
            return Ok(());
        }

        // Handle multiline note buffering.
        if let Some(note) = &self.note_buffer {
            let trimmed = line.trim();
            // FactorySequenceNote{,Across,OverSeveral}Command#END accepts all
            // three shape names; FactorySequenceNoteOnArrowCommand#END only note.
            static NOTE_END: LazyLock<Regex> =
                LazyLock::new(|| sequence_regex(r"(?i)^end[%s]?(note|hnote|rnote)$").unwrap());
            let closes = NOTE_END
                .captures(trimmed)
                .is_some_and(|caps| !note.on_message || caps[1].eq_ignore_ascii_case("note"));
            if closes {
                let buf = self.note_buffer.take().unwrap();
                let text = note_text_from_lines(&buf.lines);
                self.events.push(Event::Note(Note {
                    position: buf.position,
                    participants: buf.participants,
                    text,
                    shape: buf.shape,
                    color: buf.color,
                    on_message: buf.on_message,
                    source_line: buf.source_line,
                }));
            } else if let Some(buf) = &mut self.note_buffer {
                buf.lines.push(line.to_string());
            }
            return Ok(());
        }

        // Keywords that could be confused with participant names must be
        // checked before the message regex.
        if self.try_autonumber(line) {
            return Ok(());
        }
        if self.try_return(line) {
            return Ok(());
        }
        if self.try_activate_deactivate(line) {
            return Ok(());
        }
        if self.try_autoactivate(line) {
            return Ok(());
        }
        if self.try_create_destroy(line) {
            return Ok(());
        }
        if self.try_participant_decl(line) {
            return Ok(());
        }
        if self.try_group(line) {
            return Ok(());
        }
        if self.try_note(line)? {
            return Ok(());
        }
        if self.try_divider(line) {
            return Ok(());
        }
        if self.try_delay(line) {
            return Ok(());
        }
        if self.try_space(line) {
            return Ok(());
        }
        // try_box must come before try_message: "box" would otherwise be parsed
        // as a message b -[o]-> x because 'o' and 'x' are valid arrow chars.
        if self.try_box(line) {
            return Ok(());
        }
        // try_meta before try_message: `title <back:cyan>...</back>` would
        // otherwise be parsed as a "title <- back" message because the
        // arrow regex accepts `<` and `>` as arrow characters. Title /
        // header / footer / caption / legend all start with explicit
        // keywords, so checking them first cannot conflict with anything
        // a message line is allowed to look like.
        if self.try_meta(line) {
            return Ok(());
        }
        if self.try_pragma(line) {
            return Ok(());
        }
        // CommandFootbox accepts a zero-length separator. CommandArrow requires
        // its arrow body, so identifiers such as hidefootbox cannot be arrows.
        if self.try_hide(line) {
            return Ok(());
        }
        let message_line = line.strip_prefix('&').unwrap_or(line).trim_start();
        if let Some(caps) = MESSAGE_RE
            .captures(message_line)
            .or_else(|| EXTERNAL_MESSAGE_IN_RE.captures(message_line))
            .or_else(|| EXTERNAL_MESSAGE_OUT_RE.captures(message_line))
        {
            let (_, arrow) = strip_arrow_style_annotation(&caps[2]);
            // CommandArrow ARROW_DRESSING1/2 and CommandExoArrowLeft/Right:
            // circles surround the head; `->o` is valid, `-o>` is not.
            if let (Some(first), Some(last)) = (arrow.find('-'), arrow.rfind('-')) {
                let left = arrow[..first].trim_start_matches(['o', 'x']);
                let right = arrow[last + 1..].trim_end_matches(['o', 'x']);
                if left.contains('o') || right.contains('o') {
                    return Err(ParseError {
                        line: line_num,
                        message: "Illegal sequence arrow decoration".into(),
                    });
                }
            }
        }
        if self.try_message(line) {
            return Ok(());
        }
        if self.try_ref(line) {
            return Ok(());
        }
        if self.try_newpage(line) {
            return Ok(());
        }
        if self.try_skinparam(line) {
            return Ok(());
        }
        if self.try_external_message(line) {
            return Ok(());
        }

        // Preserve permissive sequence parsing, but never count ignored text as
        // successful command consumption in factory competition.
        self.unconsumed = true;
        Ok(())
    }

    fn try_participant_decl(&mut self, line: &str) -> bool {
        self.try_participant_decl_in_command(line, line)
    }

    fn try_participant_decl_in_command(&mut self, line: &str, full_command: &str) -> bool {
        // CommandParticipantA/A2/A3/A4#getRegexConcat: direct spaceOneOrMore leaf.
        // CommandParticipant#getRegexType includes `create` in the SAME command;
        // keep its full-line signature when parsing only the declaration tail.
        if !has_fox_space(full_command) {
            return false;
        }
        let (url, clean_line) = super::extract_link_url(line);
        // Url#getTooltip: the {tooltip} part, else the URL itself.
        let url_tooltip = url.as_ref().and_then(|_| super::extract_link_tooltip(line));
        // A [[link]] inside the quoted FULL display is creole display text in
        // Java (no participant URL); this parser lifts it to the participant.
        let link_in_display = line.find("[[").is_some_and(|link| {
            line.find('"').is_some_and(|open| {
                open < link
                    && line[open + 1..]
                        .find('"')
                        .is_some_and(|close| link < open + 1 + close)
            })
        });
        let line = clean_line.as_str();
        // Matches these forms (label = display text, id = entity key):
        //   1. keyword "Long Label" as alias  <<stereotype>>
        //   2. keyword alias as "Long Label"  <<stereotype>>
        //   3. keyword Label as alias         <<stereotype>>  (both unquoted)
        //   4. keyword SimpleName             <<stereotype>>
        //   5. keyword "Long Label"           <<stereotype>>  (no alias; id = label)
        // CommandParticipantA/A2/A3/A4: complete identity, optional touching
        // stereotype, then ORDER, URL (extracted above), COLOR and end-of-line.
        // Negative order is admitted but cannot be represented by usize. Keep
        // the supported declaration fields and report the ordering residue.
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(
                r#"^(?i-u:(participant|actor|boundary|control|entity|database|collections|queue))[%s]+(?:"([^"]+)"[%s]+(?i-u:as)[%s]+([\p{L}\p{N}_.@]+)|([\p{L}\p{N}_.@]+)[%s]+(?i-u:as)[%s]+"([^"]+)"|([\p{L}\p{N}_.@]+)[%s]+(?i-u:as)[%s]+([\p{L}\p{N}_.@]+)|"([^"]+)"|([\p{L}\p{N}_.@]+))[%s]*(?:<<(.+?)>>)?[%s]*(?:(?i-u:order)[%s]+(?P<order>-?[0-9]{1,7}))?[%s]*(?P<color>#[^%s]+)?$"#,
            )
            .unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            if caps
                .name("order")
                .is_some_and(|m| m.as_str().starts_with('-'))
            {
                eprintln!(
                    "warning: sequence participant signed order is recognized but unsupported; retaining declaration fields with fallback order at line {}",
                    self.current_line
                );
            }
            let kind = parse_participant_kind(&caps[1].to_ascii_lowercase());
            // `quoted` tracks whether the display text came from a `"..."` form.
            // PlantUML's grammar matches the stereotype (`STEREO`) only OUTSIDE the
            // quoted display string (`FULL` = `[%g]([^%g]+)[%g]`), so a `<<...>>`
            // that appears WITHIN a quoted label is literal display text, not a
            // stereotype (see CommandParticipantA). We therefore only mine an
            // inline stereotype from UNQUOTED labels.
            let (raw_label, id, quoted) = if let Some(quoted) = caps.get(2) {
                // Form 1: "Long Label" as alias
                (quoted.as_str().to_string(), caps[3].to_string(), true)
            } else if let Some(alias) = caps.get(4) {
                // Form 2: alias as "Long Label"
                let lbl = caps.get(5).map_or("", |m| m.as_str()).to_string();
                (lbl, alias.as_str().to_string(), true)
            } else if let Some(label) = caps.get(6) {
                // Form 3: Label as alias (both unquoted) — id is the alias.
                (label.as_str().to_string(), caps[7].to_string(), false)
            } else if let Some(quoted) = caps.get(8) {
                // Form 5: "Long Label" (no alias; id = label)
                let lbl = quoted.as_str().to_string();
                (lbl.clone(), lbl, true)
            } else {
                // Form 4: SimpleName
                let name = caps[9].to_string();
                (name.clone(), name, false)
            };
            // Extract `<<stereotype>>` from within the label text (e.g.
            // `Service 1 <<internal>>`) only for unquoted labels — a quoted
            // display string keeps `<<...>>` as literal text.
            let (label, label_stereotype) = if quoted {
                (raw_label, None)
            } else {
                extract_stereotype_from_label(&raw_label)
            };
            // CommandParticipant passes its label through Display#getWithNewlines.
            let label = inline_sequence_display(&label);
            let stereotype = caps
                .get(10)
                .map(|m| m.as_str().to_string())
                .or(label_stereotype);

            // CommandParticipant#executeArg calls setStereotype only when the
            // declaration has one; SkinParam#stereotypePositionTop is captured.
            let stereotype_position_bottom = stereotype.is_some()
                && self
                    .meta
                    .skinparams
                    .iter()
                    .rev()
                    .find(|param| param.key.eq_ignore_ascii_case("stereotypePosition"))
                    .is_some_and(|param| param.value.eq_ignore_ascii_case("bottom"));

            let color = caps.name("color").map(|m| m.as_str().to_string());

            // Explicit `order N` sets the layout sort key; otherwise it defaults
            // to the declaration index. Implicit participants (created by a
            // message) also default to their declaration index, so a participant
            // declared with a large `order N` can be positioned to the right of a
            // later, implicitly-created one (matching Java PlantUML).
            let explicit_order = caps
                .name("order")
                .and_then(|m| m.as_str().parse::<usize>().ok());

            if !self.participant_ids.contains(&id) {
                // Stereotype#build splits '<<a>><<b>>' into one label each.
                if stereotype
                    .as_deref()
                    .is_some_and(|s| s.contains(">>") && s.contains("<<"))
                {
                    eprintln!(
                        "warning: sequence participant has multiple stereotypes (Stereotype#build label list); rendering them as separate labels remains unsupported at line {}",
                        self.current_line
                    );
                }
                if link_in_display {
                    eprintln!(
                        "warning: sequence participant display contains a creole [[link]]; its label and tooltip are not rendered (treated as a participant link) at line {}",
                        self.current_line
                    );
                }
                self.participant_ids.push(id.clone());
                let idx = self.participants.len();
                self.participants.push(Participant {
                    id: id.clone(),
                    label,
                    kind,
                    order: Some(explicit_order.unwrap_or(idx)),
                    stereotype,
                    stereotype_position_bottom,
                    url,
                    url_tooltip,
                    color,
                    source_line: self.current_line,
                });
                if let Some(bi) = self.current_box {
                    self.boxes[bi].members.push(idx);
                }
            }
            true
        } else {
            false
        }
    }

    fn try_message(&mut self, line: &str) -> bool {
        // CommandArrow#getRegexConcat PARALLEL precedes the first participant.
        let parallel = line.starts_with('&');
        let line = line.strip_prefix('&').unwrap_or(line).trim_start();
        if let Some(caps) = MESSAGE_RE.captures(line) {
            // Strip surrounding quotes from quoted participant names.
            let unquote = |s: &str| -> String {
                if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                    s[1..s.len() - 1].to_string()
                } else {
                    s.to_string()
                }
            };
            let from_raw = unquote(&caps[1]);
            let (arrow_style, arrow_str) = strip_arrow_style_annotation(&caps[2]);
            let to_raw = unquote(&caps[3]);
            let activation_str = caps.get(4).map(|m| m.as_str());
            let activation_color = caps.get(5).map(|m| m.as_str().to_string());
            let label = message_label(line, caps.get(6));

            let mut arrow = parse_arrow(&arrow_str);
            apply_inline_arrow_style(&mut arrow, arrow_style);
            let activation = activation_str
                .map(parse_activation)
                .or_else(|| self.autoactivation_for(&arrow));

            // Ensure participants in textual order (left-to-right as written)
            // so the participant list preserves declaration order.
            let left_id = self.ensure_participant(&from_raw);
            let right_id = self.ensure_participant(&to_raw);

            // For left-pointing arrows (Alice <- Bob), swap from/to so that
            // from=sender(Bob), to=receiver(Alice), matching PlantUML semantics.
            let (from_id, to_id) = if arrow.direction == ArrowDirection::RightToLeft {
                (right_id, left_id)
            } else {
                (left_id, right_id)
            };

            self.last_message = Some((from_id.clone(), to_id.clone()));
            self.events.push(Event::Message(Message {
                parallel,
                from: from_id,
                to: to_id,
                label,
                arrow,
                activation,
                activation_color,
                source_line: self.current_line,
            }));
            true
        } else {
            false
        }
    }

    fn try_external_message(&mut self, line: &str) -> bool {
        let (caps, boundary_first) = if let Some(caps) = EXTERNAL_MESSAGE_IN_RE.captures(line) {
            (caps, true)
        } else if let Some(caps) = EXTERNAL_MESSAGE_OUT_RE.captures(line) {
            (caps, false)
        } else {
            return false;
        };
        let (arrow_style, arrow_str) = strip_arrow_style_annotation(&caps[2]);
        let mut arrow = parse_arrow(&arrow_str);
        apply_inline_arrow_style(&mut arrow, arrow_style);
        let (left, right) = if boundary_first {
            (caps[1].to_string(), self.ensure_participant(&caps[3]))
        } else {
            (self.ensure_participant(&caps[1]), caps[3].to_string())
        };
        // CommandExoArrowLeft/Right#getMessageExoType select FROM/TO from
        // the dressing, independently of which side contains the border token.
        let (from, to) = if arrow.direction == ArrowDirection::RightToLeft {
            (right, left)
        } else {
            (left, right)
        };
        let activation = caps
            .get(4)
            .map(|m| parse_activation(m.as_str()))
            .or_else(|| self.autoactivation_for(&arrow));
        // SequenceDiagram#getLastEventWithNote sees AbstractMessage, including
        // MessageExo. FactorySequenceNoteOnArrowCommand attaches a bare note to
        // that event, using its normalized endpoints just like an ordinary arrow.
        self.last_message = Some((from.clone(), to.clone()));
        self.events.push(Event::Message(Message {
            parallel: false,
            from,
            to,
            label: message_label(line, caps.get(6)),
            arrow,
            activation,
            activation_color: caps.get(5).map(|m| m.as_str().to_string()),
            source_line: self.current_line,
        }));
        true
    }

    fn try_note(&mut self, line: &str) -> Result<bool, ParseError> {
        // `note : text` — Java PlantUML creates a participant named "note" with
        // a note box above/beside it showing the text.  We declare the "note"
        // participant and emit a Note event.
        if let Some(text) = line.strip_prefix("note :").map(|s| s.trim()) {
            if !text.is_empty() {
                let participant_id = self.ensure_participant("note");
                self.events.push(Event::Note(Note {
                    position: NotePosition::Over,
                    participants: vec![participant_id],
                    text: inline_sequence_display(text),
                    shape: NoteShape::Note,
                    color: None,
                    on_message: false,
                    source_line: self.current_line,
                }));
            }
            return Ok(true);
        }

        // Note on link (attaches to previous message).
        if let Some(rest) = line.strip_prefix("note on link") {
            let text = rest.trim().trim_start_matches(':').trim().to_string();
            self.events.push(Event::NoteOnLink(text));
            return Ok(true);
        }

        // FactorySequenceNoteAcrossCommand composes STYLE, optional whitespace,
        // ACROSS and color, rather than a literal-space command prefix. Its
        // nested optional stereotype whitespace contributes no required Fox bit.
        static ACROSS: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(r"^(h|r)?note[%s]*(accross|across)[%s]*(#[^%s:]+)?[%s]*(?::[%s]*(.*))?$")
                .unwrap()
        });
        if let Some(caps) = ACROSS.captures(line) {
            // FactorySequenceNoteAcrossCommand#executeInternal rejects this
            // recognized grammar alternative, rather than silently dropping it.
            if &caps[2] == "accross" {
                return Err(ParseError {
                    line: self.current_line,
                    message: "Use 'across' instead of 'accross'".into(),
                });
            }
            let shape = match caps.get(1).map(|m| m.as_str()) {
                Some("h") => NoteShape::Hexagonal,
                Some("r") => NoteShape::Rectangular,
                _ => NoteShape::Note,
            };
            let color = caps.get(3).map(|m| m.as_str().to_string());
            if let Some(text) = caps.get(4) {
                self.events.push(Event::Note(Note {
                    position: NotePosition::Over,
                    participants: Vec::new(),
                    text: inline_sequence_display(text.as_str()),
                    shape,
                    color,
                    on_message: false,
                    source_line: self.current_line,
                }));
            } else {
                self.note_buffer = Some(NoteBuffer {
                    position: NotePosition::Over,
                    participants: Vec::new(),
                    lines: Vec::new(),
                    shape,
                    color,
                    on_message: false,
                    source_line: self.current_line,
                });
            }
            return Ok(true);
        }

        // FactorySequenceNoteCommand requires its PARTICIPANT after the optional
        // prefix. Keep it required, so `of` can itself be the identity. Single-line
        // prefix uses [%s] once; multiline uses [%s]+. OverSeveral has exactly two
        // required identities and no prefix. OnArrow is a separate, unnamed form.
        static NAMED: LazyLock<Vec<Regex>> = LazyLock::new(|| {
            let identity = r#"[\p{L}\p{N}_.@]+|"[^"]+""#;
            let mut patterns = Vec::new();
            for inline in [true, false] {
                let suffix = if inline {
                    r"[%s]*:[%s]*(?P<text>.*)$"
                } else {
                    "$"
                };
                let color = r"[%s]*(?P<color>#[^%s:]+)?[%s]*";
                let prefix = if inline {
                    r"(?:of[%s])?"
                } else {
                    r"(?:of[%s]+)?"
                };
                patterns.push(sequence_regex(&format!(
                    r"^(?P<shape>h|r)?note[%s]+(?P<position>left|right|over)[%s]+{prefix}(?P<p1>{identity}){color}{suffix}"
                )).unwrap());
                patterns.push(sequence_regex(&format!(
                    r"^(?P<shape>h|r)?note[%s]+(?P<position>over)[%s]+(?P<p1>{identity})[%s]*,[%s]*(?P<p2>{identity}){color}{suffix}"
                )).unwrap());
            }
            patterns
        });
        static ATTACHED: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(r"^(?P<shape>h|r)?note[%s]+(?P<position>left|right)[%s]*(?P<color>#[^%s:]+)?[%s]*(?::[%s]*(?P<text>.*))?$").unwrap()
        });
        static BARE_OVER: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(r"^(?:h|r)?note[%s]+over[%s]*(?:#[^%s:]+)?[%s]*(?::.*)?$").unwrap()
        });
        // Both standalone note factories contain a direct required-space leaf;
        // OnArrow has only optional whitespace and must bypass this prefilter.
        let captures = NAMED
            .iter()
            .filter(|_| has_fox_space(line))
            .find_map(|pattern| pattern.captures(line))
            .or_else(|| ATTACHED.captures(line));
        if let Some(caps) = captures {
            let shape = match caps.name("shape").map(|m| m.as_str()) {
                Some("h") => NoteShape::Hexagonal,
                Some("r") => NoteShape::Rectangular,
                _ => NoteShape::Note,
            };
            let position = match caps.name("position").unwrap().as_str() {
                "left" => NotePosition::Left,
                "right" => NotePosition::Right,
                "over" => NotePosition::Over,
                _ => unreachable!(),
            };
            let inline_text = caps.name("text");
            let participant_source_line = if inline_text.is_some() {
                self.current_line
            } else {
                self.current_line + 1
            };
            let mut participants: Vec<String> = ["p1", "p2"]
                .iter()
                .filter_map(|name| caps.name(name))
                .map(|m| {
                    self.ensure_participant_at(
                        m.as_str().trim_matches('"'),
                        participant_source_line,
                    )
                })
                .collect();
            // Bare "note left" / "note right" (no participant) attaches to the
            // last message and straddles its arrow band (on_message). PlantUML
            // anchors such notes to the message's leftmost/rightmost endpoint by
            // screen position, so store BOTH endpoints and let the renderer pick.
            // "note left"  → left of the leftmost endpoint
            // "note right" → right of the rightmost endpoint
            let mut on_message = false;
            if participants.is_empty()
                && position != NotePosition::Over
                && let Some((from, to)) = &self.last_message
            {
                participants = vec![from.clone(), to.clone()];
                on_message = true;
            }
            let color = caps.name("color").map(|m| m.as_str().to_string());

            if let Some(text_match) = inline_text {
                if participants.is_empty() && position != NotePosition::Over && !on_message {
                    return Ok(true);
                }
                // Inline note: note right : text
                let text = inline_sequence_display(text_match.as_str().trim());
                self.events.push(Event::Note(Note {
                    position,
                    participants,
                    text,
                    shape,
                    color,
                    on_message,
                    source_line: self.current_line,
                }));
            } else {
                // Multiline note: note right\n...\nendnote
                self.note_buffer = Some(NoteBuffer {
                    position,
                    participants,
                    lines: Vec::new(),
                    shape,
                    color,
                    on_message,
                    source_line: self.current_line,
                });
            }
            Ok(true)
        } else if BARE_OVER.is_match(line) {
            // Neither standalone factory accepts a missing participant; the
            // OnArrow factory does not include `over` in POSITION.
            Err(ParseError {
                line: self.current_line,
                message: "A note over command requires a participant".into(),
            })
        } else {
            Ok(false)
        }
    }

    fn try_group(&mut self, line: &str) -> bool {
        static RE_START: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(r"^(alt|opt|loop|par|break|critical|group)[%s]*(.*)$").unwrap()
        });
        static RE_ELSE: LazyLock<Regex> =
            LazyLock::new(|| sequence_regex(r"^else[%s]*(.*)$").unwrap());

        if line == "end" {
            self.events.push(Event::GroupEnd);
            return true;
        }

        if let Some(caps) = RE_START.captures(line) {
            let kind = match &caps[1] {
                "alt" => GroupKind::Alt,
                "opt" => GroupKind::Opt,
                "loop" => GroupKind::Loop,
                "par" => GroupKind::Par,
                "break" => GroupKind::Break,
                "critical" => GroupKind::Critical,
                "group" => GroupKind::Group,
                _ => GroupKind::Group,
            };
            let label = {
                let l = caps[2].trim();
                if l.is_empty() {
                    None
                } else {
                    Some(l.to_string())
                }
            };
            self.events.push(Event::GroupStart(GroupStart {
                kind,
                label,
                source_line: self.current_line,
            }));
            return true;
        }

        if let Some(caps) = RE_ELSE.captures(line) {
            let label = {
                let l = caps[1].trim();
                if l.is_empty() {
                    None
                } else {
                    Some(l.to_string())
                }
            };
            self.events.push(Event::GroupElse(GroupElse {
                label,
                source_line: self.current_line,
            }));
            return true;
        }

        false
    }

    fn try_divider(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| sequence_regex(r"^==[%s]*(.*?)[%s]*==$").unwrap());

        if let Some(caps) = RE.captures(line) {
            self.events.push(Event::Divider(caps[1].trim().to_string()));
            true
        } else {
            false
        }
    }

    fn try_delay(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| sequence_regex(r"^\.\.\.(.*?)\.\.\.$").unwrap());

        if line == "..." {
            self.events.push(Event::Delay(None));
            return true;
        }
        if let Some(caps) = RE.captures(line) {
            // CommandDelay#executeArg passes LABEL verbatim to Display.
            // Spacer atoms still contribute to the component preferred width
            // (account a21a0275); only the undelimited `...` has no label.
            self.events.push(Event::Delay(Some(caps[1].to_string())));
            true
        } else {
            false
        }
    }

    fn try_space(&mut self, line: &str) -> bool {
        static RE: LazyLock<Regex> = LazyLock::new(|| sequence_regex(r"^\|\|(\d+)\|\|$").unwrap());

        if line == "|||" {
            self.events.push(Event::Space(None));
            return true;
        }
        if let Some(caps) = RE.captures(line) {
            let px: u32 = caps[1].parse().unwrap_or(20);
            self.events.push(Event::Space(Some(px)));
            true
        } else {
            false
        }
    }

    fn try_autonumber(&mut self, line: &str) -> bool {
        static RE_START: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(r#"^autonumber(?:[%s]+(\d+))?(?:[%s]+(\d+))?(?:[%s]+"([^"]*)")?$"#)
                .unwrap()
        });
        static RE_RESUME: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(r#"^autonumber[%s]+resume(?:[%s]+(\d+))?(?:[%s]+"([^"]*)")?$"#).unwrap()
        });

        if line == "autonumber stop" {
            self.push_autonumber(AutonumberCmd::Stop);
            return true;
        }
        // CommandAutonumberResume#getConcat has a direct required space;
        // CommandAutonumber's optional argument groups do not contribute it.
        if has_fox_space(line)
            && let Some(caps) = RE_RESUME.captures(line)
        {
            let step = caps.get(1).and_then(|m| m.as_str().parse().ok());
            let format = caps.get(2).map(|m| m.as_str().to_string());
            self.push_autonumber(AutonumberCmd::Resume { step, format });
            return true;
        }
        if let Some(caps) = RE_START.captures(line) {
            let start = caps.get(1).map_or(1, |m| m.as_str().parse().unwrap_or(1));
            let step = caps.get(2).map_or(1, |m| m.as_str().parse().unwrap_or(1));
            let format = caps.get(3).map(|m| m.as_str().to_string());
            self.push_autonumber(AutonumberCmd::Start {
                start,
                step,
                format,
            });
            return true;
        }
        false
    }

    /// Record an autonumber directive: keep the first `Start` in the legacy
    /// `self.autonumber` field (so initial layout still sees it) and always emit
    /// an event so mid-stream changes (stop/resume/restart) take effect.
    fn push_autonumber(&mut self, cmd: AutonumberCmd) {
        if self.autonumber.is_none()
            && let AutonumberCmd::Start {
                start,
                step,
                format,
            } = &cmd
        {
            self.autonumber = Some(AutoNumber {
                start: *start,
                step: *step,
                format: format.clone(),
            });
        }
        self.events.push(Event::Autonumber(cmd));
    }

    fn try_activate_deactivate(&mut self, line: &str) -> bool {
        // CommandActivate#getRegexConcat: direct spaceOneOrMore leaf.
        if !has_fox_space(line) {
            return false;
        }
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(r"^(activate|deactivate)[%s]+(\w+)(?:[%s]+(#[^%s]+))?").unwrap()
        });

        if let Some(caps) = RE.captures(line) {
            let id = self.ensure_participant(&caps[2]);
            let color = caps.get(3).map(|m| m.as_str().to_string());
            match &caps[1] {
                "activate" => self.events.push(Event::Activate(id, color)),
                "deactivate" => self.events.push(Event::Deactivate(id)),
                _ => {}
            }
            true
        } else {
            false
        }
    }

    fn try_autoactivate(&mut self, line: &str) -> bool {
        let Some(rest) = line.strip_prefix("autoactivate") else {
            return false;
        };
        match rest.trim() {
            state if state.eq_ignore_ascii_case("on") => {
                self.autoactivate = true;
                true
            }
            state if state.eq_ignore_ascii_case("off") => {
                self.autoactivate = false;
                true
            }
            _ => false,
        }
    }

    fn autoactivation_for(&self, arrow: &Arrow) -> Option<ActivationChange> {
        if !self.autoactivate || arrow.head_half.is_some() || arrow.source_cross {
            return None;
        }
        match (arrow.line, arrow.head) {
            (LineStyle::Dotted, ArrowHead::Filled | ArrowHead::Open) => {
                Some(ActivationChange::Deactivate)
            }
            (LineStyle::Solid | LineStyle::Hidden, ArrowHead::Filled | ArrowHead::Open) => {
                Some(ActivationChange::Activate)
            }
            _ => None,
        }
    }

    fn try_create_destroy(&mut self, line: &str) -> bool {
        // CommandActivate and CommandParticipantA#getRegexConcat: direct spaceOneOrMore leaf.
        if !has_fox_space(line) {
            return false;
        }
        static KW: LazyLock<Regex> =
            LazyLock::new(|| sequence_regex(r"^(create|destroy)[%s]+(.*)$").unwrap());
        // Identifier extraction from a participant declaration tail. Mirrors the
        // forms in `try_participant_decl`; the id is the alias when `as` is
        // present, otherwise the simple name (or quoted label).
        static REST: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(
                r#"^(?:participant|actor|boundary|control|entity|database|collections|queue)?[%s]*(?:"[^"]+"[%s]+as[%s]+(\w+)|(\w+)[%s]+as[%s]+(?:"[^"]+"|\w+)|"([^"]+)"|(\w+))"#,
            )
            .unwrap()
        });

        let Some(kw) = KW.captures(line) else {
            return false;
        };
        let rest = kw[2].trim();
        let Some(caps) = REST.captures(rest) else {
            return false;
        };
        let id = caps
            .get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .or_else(|| caps.get(4))
            .map(|m| m.as_str().to_string());
        let Some(id) = id else {
            return false;
        };

        match &kw[1] {
            "create" => {
                // Register with the declared kind/alias/label if a full declaration
                // was given; otherwise ensure a plain participant exists.
                if !self.try_participant_decl_in_command(rest, line) {
                    self.ensure_participant(&id);
                }
                self.events.push(Event::Create(id));
            }
            "destroy" => self.events.push(Event::Destroy(id)),
            _ => {}
        }
        true
    }

    fn try_return(&mut self, line: &str) -> bool {
        let parallel = line.starts_with('&');
        let line = line.strip_prefix('&').unwrap_or(line).trim_start();
        if let Some(rest) = line.strip_prefix("return") {
            // CommandReturn#getRegexConcat captures COLOR only when #\w+
            // is followed by whitespace; a terminal #word remains label text.
            static COLOR: LazyLock<Regex> =
                LazyLock::new(|| sequence_regex(r"^(#\w+)[%s]+(.*)$").unwrap());
            let rest = rest.trim_start();
            let (color, label) = if let Some(caps) = COLOR.captures(rest) {
                (Some(caps[1].to_owned()), caps[2].to_owned())
            } else {
                (None, rest.to_owned())
            };
            self.events.push(Event::Return(ReturnMessage {
                parallel,
                label,
                color,
                source_line: self.current_line,
            }));
            true
        } else {
            false
        }
    }

    fn try_ref(&mut self, line: &str) -> bool {
        // Inline: ref over A, B : text
        static RE_INLINE: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(r"^ref[%s]+over[%s]+(\w+(?:[%s]*,[%s]*\w+)*)[%s]*:[%s]*(.+)$").unwrap()
        });
        // Multiline start: ref over A, B  (no colon)
        static RE_START: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(r"^ref[%s]+over[%s]+(\w+(?:[%s]*,[%s]*\w+)*)[%s]*$").unwrap()
        });

        // CommandReferenceOverSeveral uses RegexConcat#match. The multiline
        // command instead passes REGEX_CONCAT.getPattern() to Pattern2 directly,
        // so its first-line matcher must NOT run the FoxSignature prefilter.
        if has_fox_space(line)
            && let Some(caps) = RE_INLINE.captures(line)
        {
            let participants: Vec<String> = caps[1]
                .split(',')
                .map(|s| self.ensure_participant(s.trim()))
                .collect();
            let text = caps[2].trim().to_string();
            self.events.push(Event::Ref(Ref {
                participants,
                text,
                source_line: self.current_line,
            }));
            return true;
        }
        if let Some(caps) = RE_START.captures(line) {
            let participants: Vec<String> = caps[1]
                .split(',')
                .map(|s| self.ensure_participant(s.trim()))
                .collect();
            self.ref_buffer = Some(RefBuffer {
                participants,
                lines: Vec::new(),
                source_line: self.current_line,
            });
            return true;
        }
        false
    }

    fn try_meta(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("title ") {
            self.meta.title = Some(super::strip_title_quotes(rest).to_string());
            self.meta.title_line = Some(self.current_line);
            return true;
        }
        if let Some(rest) = line.strip_prefix("header ") {
            self.meta.header = Some(rest.trim().to_string());
            self.meta.header_line = Some(self.current_line);
            return true;
        }
        if let Some(rest) = line.strip_prefix("footer ") {
            self.meta.footer = Some(rest.trim().to_string());
            self.meta.footer_line = Some(self.current_line);
            return true;
        }
        if let Some(rest) = line.strip_prefix("caption ") {
            self.meta.caption = Some(rest.trim().to_string());
            self.meta.caption_line = Some(self.current_line);
            return true;
        }
        // Legend block: `legend` / `legend right` / `legend left` ... `endlegend`
        if line == "legend" || line.starts_with("legend ") {
            self.in_legend = true;
            self.meta.legend_line = Some(self.current_line);
            return true;
        }
        if self.in_legend {
            if line == "endlegend" || line == "end legend" {
                self.in_legend = false;
            } else {
                let l = self.meta.legend.get_or_insert_with(String::new);
                if !l.is_empty() {
                    l.push('\n');
                }
                l.push_str(line);
            }
            return true;
        }
        false
    }

    fn try_box(&mut self, line: &str) -> bool {
        // `box ["Title"] [#color] ... end box` groups consecutive participant
        // declarations into a titled, optionally coloured rectangle.
        if line == "end box" {
            self.current_box = None;
            return true;
        }
        // CommandBoxStart#getRegexConcat: title whitespace is inside an
        // optional RegexOr, so it contributes no required FoxSignature bit.
        // Preserve NAME1/NAME2 order and complete title capture before color.
        static START: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(r#"^box(?:[%s]+(?:"(?P<quoted>[^"]+)"|(?P<unquoted>[^#]+)))?[%s]*(?P<color>#[^%s]+)?$"#).unwrap()
        });
        if let Some(caps) = START.captures(line) {
            let title = caps
                .name("quoted")
                .or_else(|| caps.name("unquoted"))
                .map_or_else(String::new, |m| m.as_str().to_string());
            let color = caps.name("color").map(|m| m.as_str().to_string());
            self.boxes.push(ParticipantBox {
                title,
                color,
                members: Vec::new(),
            });
            self.current_box = Some(self.boxes.len() - 1);
            return true;
        }
        false
    }

    fn try_newpage(&mut self, line: &str) -> bool {
        if line == "newpage" {
            self.events.push(Event::NewPage(None));
            return true;
        }
        if let Some(rest) = line.strip_prefix("newpage ") {
            self.events
                .push(Event::NewPage(Some(rest.trim().to_string())));
            return true;
        }
        false
    }

    fn try_skinparam(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("skinparam ") {
            let rest = rest.trim();
            if let Some(prefix) = rest.strip_suffix('{') {
                let prefix = prefix.trim();
                if !prefix.is_empty() {
                    self.skinparam_block_prefix = Some(prefix.to_string());
                }
                return true;
            }
            if let Some((key, value)) = rest.split_once(char::is_whitespace) {
                self.meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            true
        } else {
            false
        }
    }

    fn try_pragma(&mut self, line: &str) -> bool {
        let mut parts = line.split_whitespace();
        if matches!(parts.next(), Some("!pragma"))
            && matches!(parts.next(), Some("teoz"))
            && matches!(parts.next(), Some("true"))
            && parts.next().is_none()
        {
            self.teoz = true;
            return true;
        }
        false
    }

    fn try_hide(&mut self, line: &str) -> bool {
        // CommandFootbox#HIDE_SHOW_FOOTBOX and CommandFootboxOld#getRegexConcat:
        // zero-or-more Pattern2 whitespace, with no required FoxSignature bit.
        static FOOTBOX: LazyLock<Regex> =
            LazyLock::new(|| sequence_regex(r"(?i)^(hide|show)?[%s]*footbox$").unwrap());
        static FOOTBOX_OLD: LazyLock<Regex> =
            LazyLock::new(|| sequence_regex(r"(?i)^footbox[%s]*(on|off)?[%s]*$").unwrap());
        if let Some(caps) = FOOTBOX.captures(line) {
            self.hide_footbox = !caps
                .get(1)
                .is_some_and(|m| m.as_str().eq_ignore_ascii_case("show"));
            return true;
        }
        if let Some(caps) = FOOTBOX_OLD.captures(line) {
            self.hide_footbox = !caps
                .get(1)
                .is_some_and(|m| m.as_str().eq_ignore_ascii_case("on"));
            return true;
        }
        // CommandHideShowByGender#executeSequenceDiagram ignores the gender
        // selector and updates SequenceDiagram#hiddenPortions, shared by every
        // Participant#getDisplay. Its PORTION accepts singular and plural.
        static STEREOTYPE_VISIBILITY: LazyLock<Regex> = LazyLock::new(|| {
            sequence_regex(
                r#"(?i)^(hide|show)[%s]+(?:(?:[\p{L}\p{N}_.]+|"[^"]+"|<<.*>>)[%s]+)*?stereotypes?$"#,
            )
            .unwrap()
        });
        // CommandHideShowByGender#getRegexConcat contributes spaceOneOrMore.
        if has_fox_space(line)
            && let Some(caps) = STEREOTYPE_VISIBILITY.captures(line)
        {
            self.hide_stereotypes = caps[1].eq_ignore_ascii_case("hide");
            return true;
        }
        line.starts_with("hide ")
    }
}

/// Extract a `<<stereotype>>` marker from a label string.
///
/// Returns `(cleaned_label, Some(stereotype))` if found, or `(label, None)`.
fn extract_stereotype_from_label(label: &str) -> (String, Option<String>) {
    if let Some(start) = label.find("<<")
        && let Some(rel_end) = label[start..].find(">>")
    {
        let end = start + rel_end;
        let stereotype = label[start + 2..end].trim().to_string();
        let cleaned = format!("{} {}", label[..start].trim(), label[end + 2..].trim())
            .trim()
            .to_string();
        return (cleaned, Some(stereotype));
    }
    (label.to_string(), None)
}

fn parse_participant_kind(s: &str) -> ParticipantKind {
    match s {
        "actor" => ParticipantKind::Actor,
        "boundary" => ParticipantKind::Boundary,
        "control" => ParticipantKind::Control,
        "entity" => ParticipantKind::Entity,
        "database" => ParticipantKind::Database,
        "collections" => ParticipantKind::Collections,
        "queue" => ParticipantKind::Queue,
        _ => ParticipantKind::Participant,
    }
}

fn parse_arrow(s: &str) -> Arrow {
    let s = s.trim();
    let line = if s.contains("--") {
        LineStyle::Dotted
    } else {
        LineStyle::Solid
    };

    // CommandArrow#executeArg separates dressings from the body before deciding
    // reverseDefine. Decorations do not participate in that decision.
    let left = s.split('-').next().unwrap_or("");
    let right = s.rsplit('-').next().unwrap_or("");
    let left_head = left.contains(['<', '/', '\\']);
    let right_head = right.contains(['>', '/', '\\']);
    let direction = if left_head && right_head {
        ArrowDirection::Bidirectional
    } else if left_head {
        ArrowDirection::RightToLeft
    } else {
        ArrowDirection::LeftToRight
    };
    // CommandArrow derives decoration ownership before swapping semantic
    // participants for reverseDefine arrows. The raw token's right dressing is
    // therefore the source side for `<-o`, while its left dressing is the
    // source side for `o->`.
    let reverse = direction == ArrowDirection::RightToLeft;
    let source_cross = if reverse {
        s.ends_with('x')
    } else {
        s.starts_with('x')
    };
    let source_circle = if reverse {
        s.ends_with('o')
    } else {
        s.starts_with('o')
    };
    let target_cross = if reverse {
        s.starts_with('x')
    } else {
        s.ends_with('x')
    };
    let target_circle = if reverse {
        s.starts_with('o')
    } else {
        s.ends_with('o')
    };
    // CommandArrow computes sync1/sync2 independently. ArrowConfiguration
    // #withPart modifies destination dressing2, which is initially present;
    // source dressing1 stays FULL. Physical reversal swaps the whole dressing.
    let (source, target) = if reverse {
        (right, left)
    } else {
        (left, right)
    };
    let is_async = |part: &str| {
        part.contains("<<") || part.contains(">>") || part.contains("//") || part.contains("\\\\")
    };
    let source_dressing =
        (direction == ArrowDirection::Bidirectional).then_some(if is_async(source) {
            ArrowHead::Open
        } else {
            ArrowHead::Filled
        });
    let head = if target_cross {
        ArrowHead::Cross
    } else if is_async(target) && !target.contains(['/', '\\']) {
        ArrowHead::Open
    } else {
        ArrowHead::Filled
    };
    let head_half = if right.contains('/') || left.contains('\\') {
        Some(ArrowHalf::Bottom)
    } else if right.contains('\\') || left.contains('/') {
        Some(ArrowHalf::Top)
    } else {
        None
    };
    let thin_head = head_half.is_some() && is_async(target);

    Arrow {
        line,
        head,
        direction,
        color: None,
        head_half,
        thin_head,
        source_cross,
        source_circle,
        target_circle,
        source_dressing,
    }
}

#[derive(Default)]
struct InlineArrowStyle {
    color: Option<String>,
    line: Option<LineStyle>,
}

fn strip_arrow_style_annotation(arrow: &str) -> (InlineArrowStyle, String) {
    let mut style = InlineArrowStyle::default();
    if let Some(tokens) = INLINE_ARROW_STYLE
        .captures(arrow)
        .and_then(|captures| captures.get(1))
        .map(|matched| matched.as_str())
    {
        for token in tokens.split(',') {
            if token.eq_ignore_ascii_case("dashed") || token.eq_ignore_ascii_case("dotted") {
                style.line = Some(LineStyle::Dotted);
            } else if token.eq_ignore_ascii_case("hidden") {
                style.line = Some(LineStyle::Hidden);
            } else if !token.eq_ignore_ascii_case("bold") {
                style.color = Some(token.to_string());
            }
        }
    }

    (style, INLINE_ARROW_STYLE.replace(arrow, "").into_owned())
}

pub(super) fn looks_like_message(line: &str) -> bool {
    MESSAGE_RE
        .captures(line)
        .and_then(|captures| captures.get(2))
        .or_else(|| {
            EXTERNAL_MESSAGE_IN_RE
                .captures(line)
                .and_then(|captures| captures.get(2))
        })
        .or_else(|| {
            EXTERNAL_MESSAGE_OUT_RE
                .captures(line)
                .and_then(|captures| captures.get(2))
        })
        .is_some_and(|arrow| arrow.as_str().contains('-'))
}

fn apply_inline_arrow_style(arrow: &mut Arrow, style: InlineArrowStyle) {
    if let Some(line) = style.line {
        arrow.line = line;
    }
    if let Some(color) = style.color {
        arrow.color = Some(color);
    }
}

fn message_label(line: &str, matched: Option<Match<'_>>) -> String {
    let Some(matched) = matched else {
        return String::new();
    };
    let trimmed = matched.as_str().trim();
    if trimmed.starts_with("\\n")
        && let Some(colon) = line[..matched.start()].rfind(':')
    {
        let consumed = &line[colon + 1..matched.start()];
        if !consumed.is_empty() && consumed.chars().all(char::is_whitespace) {
            return format!("{consumed}{trimmed}");
        }
    }
    trimmed.to_string()
}

fn parse_activation(s: &str) -> ActivationChange {
    match s {
        "++" => ActivationChange::Activate,
        "--" => ActivationChange::Deactivate,
        "!!" => ActivationChange::Destroy,
        _ => ActivationChange::Activate,
    }
}

fn note_text_from_lines(lines: &[String]) -> String {
    let common_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.bytes()
                .take_while(|&b| b == b' ' || b == b'\t')
                .count()
        })
        .min()
        .unwrap_or(0);

    lines
        .iter()
        .map(|line| line.get(common_indent..).unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    #[test]
    fn sprite_blocks_own_only_their_command_body() {
        for keyword in ["sprite ", "SPRITE\t", "SpRiTe\u{00a0}"] {
            for (body, pixel) in [
                (
                    "$mark <svg width=\"2\" height=\"2\">\n}\n<rect width=\"2\" height=\"2\"/>\n</svg>",
                    false,
                ),
                ("$mark <svg width=\"2\" height=\"2\"></svg>", false),
                ("$mark [2x2/16] {\nFF\n00\n}", true),
                ("$mark [2x2/16] {\nFF\n00\nEND SPRITE", true),
                ("$mark [2x2/16] {\nFF\n00\nendsprite", true),
            ] {
                let source = format!("@startuml\n{keyword}{body}\nA -> B: retained\n@enduml");
                let pre = crate::preprocess::preprocess_full(&source, None);
                let diagram = parse_sequence(&pre.lines).unwrap();
                assert_eq!(diagram.events.len(), 1, "{source}");
                assert_eq!(diagram.meta.sprites.contains_key("mark"), pixel);
                if pixel {
                    assert_eq!(diagram.meta.sprites["mark"].rows, ["FF", "00"]);
                }
            }
        }
    }

    #[test]
    fn sprite_completion_keeps_commands_before_an_included_opening() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "rustuml-sprite-span-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        for (header, body, end) in [
            (
                "sprite $s <svg width=\"2\" height=\"2\">",
                "<rect width=\"2\" height=\"2\"/>",
                "</svg>",
            ),
            ("sprite $s [2x2/16] {", "FF", "}"),
        ] {
            std::fs::write(
                directory.join("opening.puml"),
                format!("{header}\n{body}\n"),
            )
            .unwrap();
            let input = format!(
                "@startuml\nparticipant North\nparticipant South\n!include opening.puml\n{end}\nNorth -> South: retained\n@enduml"
            );
            let pre = crate::preprocess::preprocess_full(&input, Some(directory.clone()));
            let diagram = parse_sequence(&pre.lines).unwrap();
            assert_eq!(diagram.participants.len(), 2);
            assert!(pre.lines.iter().any(|line| line == "participant North"));
            assert!(pre.lines.iter().any(|line| line == "participant South"));
            assert_eq!(diagram.events.len(), 1);
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn incomplete_sprite_blocks_remain_syntax_errors() {
        for header in [
            "sprite $mark <svg width=\"2\" height=\"2\">",
            "sprite $mark [2x2/16] {",
        ] {
            let source =
                format!("@startuml\nparticipant A\nparticipant B\n{header}\nFF\nA -> B\n@enduml");
            let pre = crate::preprocess::preprocess_full(&source, None);
            assert!(parse_sequence(&pre.lines).is_err());
        }
    }

    // TimLoader#load runs before PSystemCommandFactory: a sprite header in an
    // inactive branch or an uninvoked procedure never reaches the registry,
    // and an included body completes the command after expansion.
    #[test]
    fn sprite_commands_see_only_tim_output() {
        for header in [
            "sprite $mark <svg width=\"2\" height=\"2\">",
            "sprite $mark [2x2/16] {",
        ] {
            for wrapped in [
                format!("!if %false()\n{header}\nFF\n!endif"),
                format!("!procedure $unused()\n{header}\nFF\n!endprocedure"),
            ] {
                let source =
                    format!("@startuml\nparticipant A\nparticipant B\n{wrapped}\nA -> B\n@enduml");
                for teoz in ["", "!pragma teoz true\n"] {
                    let source = source.replacen("@startuml\n", &format!("@startuml\n{teoz}"), 1);
                    let diagram = crate::parse::parse(&source).expect(&source);
                    assert!(
                        matches!(diagram, crate::diagram::Diagram::Sequence(_)),
                        "{source}"
                    );
                }
            }
            // The same header active and unclosed is a Java error.
            for wrapped in [
                format!("!if %true()\n{header}\nFF\n!endif"),
                format!("!procedure $used()\n{header}\nFF\n!endprocedure\n$used()"),
            ] {
                let source =
                    format!("@startuml\nparticipant A\nparticipant B\n{wrapped}\nA -> B\n@enduml");
                assert!(crate::parse::parse(&source).is_err(), "{source}");
            }
        }
    }

    // Multiline note/legend/ref commands own their bodies through END, so body
    // text is never offered to the sprite commands.
    #[test]
    fn command_bodies_are_not_sprite_commands() {
        for body in [
            "note over A\nSprite sheets are loaded\nend note",
            "note over A\nSprite frame {\nend note",
            "note right of B\nsprite atlas {\nend note",
            "legend\nSPRITE atlas version two\nendlegend",
            "ref over A, B\nSprite cache warm\nend ref",
        ] {
            let source =
                format!("@startuml\nparticipant A\nparticipant B\n{body}\nA -> B\n@enduml");
            let pre = crate::preprocess::preprocess_full(&source, None);
            parse_sequence(&pre.lines).expect(&source);
        }
        // At a command start the same text is CommandSpriteFile: Cannot read.
        let pre = crate::preprocess::preprocess_full(
            "@startuml\nparticipant A\nSprite sheets are loaded\n@enduml",
            None,
        );
        assert!(parse_sequence(&pre.lines).is_err());
    }

    // Url#getTooltip: the {tooltip} of [[url{tooltip} label]] is the anchor
    // title; without one the URL is.
    #[test]
    fn participant_link_tooltip_is_recorded() {
        let d = parse("participant Cedar [[https://example.org/ring{rings} ring]]\nA -> Cedar");
        assert_eq!(
            d.participants[0].url.as_deref(),
            Some("https://example.org/ring")
        );
        assert_eq!(d.participants[0].url_tooltip.as_deref(), Some("rings"));
        let d = parse("participant Cedar [[https://example.org/ring]]\nA -> Cedar");
        assert_eq!(d.participants[0].url_tooltip, None);
    }

    #[test]
    fn complete_participant_declarations_preserve_identity_and_options() {
        let d = parse(
            "participant \"Tall tower\" as Top.Node<<edge>> order 7#Wheat\nqueue @Port as \"Mail room\"<<service>>#Olive\nA -> B",
        );
        assert_eq!(d.participants[0].id, "Top.Node");
        assert_eq!(d.participants[0].label, "Tall tower");
        assert_eq!(d.participants[0].stereotype.as_deref(), Some("edge"));
        assert_eq!(d.participants[0].order, Some(7));
        assert_eq!(d.participants[0].color.as_deref(), Some("#Wheat"));
        assert_eq!(d.participants[1].id, "@Port");
        assert_eq!(d.participants[1].label, "Mail room");
        // Unsupported signed order must not erase the complete declaration.
        let d = parse("participant West order -7\nA -> B");
        assert_eq!(d.participants[0].id, "West");
        let mut parser = SeqParser::new();
        assert!(!parser.try_participant_decl("participant West unsupported"));
        assert!(parser.participants.is_empty());
    }

    #[test]
    fn command_specific_color_execution_respects_early_and_unused_paths() {
        for color in [
            "#back:Wheat;line:Olive",
            "#line.dotted",
            "#shadowing:true",
            "#text:application",
        ] {
            assert!(
                parse_sequence(&[format!("participant West {color}")]).is_err(),
                "{color}"
            );
        }
        for separator in ['-', '/', '\\', '|'] {
            assert!(parse_sequence(&[format!("participant West #Wheat{separator}Olive")]).is_ok());
        }
        for declaration in [
            "participant West #NotAColour",
            "participant West as \"Another label\" #NotAColour",
            "participant \"Another label\" as West #NotAColour",
            "participant Else as West #NotAColour",
            "participant \"West\" #NotAColour",
        ] {
            let d = parse_sequence(&["participant West".into(), declaration.into()]).unwrap();
            assert_eq!(d.participants.len(), 1);
            assert_eq!(d.participants[0].label, "West");
        }
        assert!(
            parse_sequence(&[
                "participant West #NotAColour [".into(),
                "Label".into(),
                "]".into()
            ])
            .is_ok()
        );
        assert!(
            parse_sequence(&[
                "box #back:Wheat;line:Olive".into(),
                "participant West".into(),
                "end box".into()
            ])
            .is_ok()
        );
    }

    #[test]
    fn registered_sequence_commands_separate_syntax_and_execution_residue() {
        assert_eq!(super::REGISTERED_SEQUENCE_COMMANDS.len(), 81);
        for text in [
            "fling N sideways",
            "hide footbox garbage",
            "N\u{2003}->\u{2003}S: text",
            "activate\u{000b}N",
        ] {
            assert!(
                parse_sequence(&["N -> S: seed".into(), text.into()]).is_err(),
                "{text:?}"
            );
        }
        for text in [
            "?-> N: short",
            "activate \"North Pier\"",
            "autonumber inc A",
            "participant N [\nNorthern\nPier\n]",
        ] {
            let lines = format!("N -> S: seed\n{text}")
                .lines()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert!(parse_sequence(&lines).is_ok(), "{text:?}");
        }
        assert!(
            parse_sequence(&[
                "N -> S: seed".into(),
                "note over N".into(),
                "fling N sideways".into(),
                "end note".into()
            ])
            .is_ok()
        );
    }

    #[test]
    fn registered_color_execution_uses_complete_leaf_boundaries() {
        assert!(super::valid_sequence_color("LightBlue/Yellow"));
        assert!(!super::valid_sequence_color("transparent/Yellow"));
        assert!(super::valid_sequence_color("transparent"));
        for color in [
            "#back:LightBlue: cargo",
            "#CeruleanTypo: cargo",
            "#line:MissingShade : cargo",
        ] {
            assert!(
                parse_sequence(&["N -> S: seed".into(), format!("note over N {color}")]).is_err(),
                "{color}"
            );
        }
        for color in [
            "#back:LightBlue : cargo",
            "#LightBlue/Yellow: cargo",
            "#transparent: cargo",
            "#11223344: cargo",
        ] {
            assert!(
                parse_sequence(&["N -> S: seed".into(), format!("note over N {color}")]).is_ok(),
                "{color}"
            );
        }
        assert!(parse_sequence(&["N -[#NoSuchShade]> S: seed".into()]).is_err());
        assert!(parse_sequence(&["N -[#blue,dashed]> S: seed".into()]).is_ok());
    }

    #[test]
    fn footbox_optional_pattern2_separator_and_both_command_orders() {
        for gap in ["", " ", "\t", "\u{000b}", "\u{00a0}"] {
            let hidden = parse(&format!("A -> B\nhide{gap}footbox"));
            assert!(hidden.hide_footbox);
            let shown = parse(&format!("A -> B\nhide footbox\nshow{gap}footbox"));
            assert!(!shown.hide_footbox);
            let old = parse(&format!("A -> B\nfootbox{gap}off"));
            assert!(old.hide_footbox);
        }
    }

    use super::*;

    #[test]
    fn unfinished_note_is_not_a_consumed_command() {
        for open in ["note right", "note over Birch, Elm"] {
            let lines = ["Birch -> Elm: go", open, "body"].map(str::to_string);
            let error = parse_sequence(&lines).unwrap_err();
            assert_eq!(error.line, 2);
            let mut closed = lines.to_vec();
            closed.push("end note".to_string());
            assert!(parse_sequence(&closed).is_ok());
        }
        // These have different Java grammar/finalization contracts.
        assert!(parse_sequence(&["group open".to_string(), "Birch -> Elm".to_string()]).is_ok());
    }

    #[test]
    fn endpoint_heads_are_independent_and_old_models_default() {
        for (token, source, target) in [
            ("o<<->o", ArrowHead::Open, ArrowHead::Filled),
            ("o<-->>o", ArrowHead::Filled, ArrowHead::Open),
        ] {
            let arrow = parse_arrow(token);
            assert_eq!(arrow.direction, ArrowDirection::Bidirectional);
            assert_eq!(arrow.source_dressing, Some(source));
            assert_eq!(arrow.head, target);
            let mut value = serde_json::to_value(&arrow).unwrap();
            let roundtrip: Arrow = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(roundtrip.source_dressing, Some(source));
            value.as_object_mut().unwrap().remove("source_dressing");
            let old: Arrow = serde_json::from_value(value).unwrap();
            assert_eq!(old.source_dressing, None);
            assert!(
                serde_json::to_value(old)
                    .unwrap()
                    .get("source_dressing")
                    .is_none()
            );
        }
        for token in ["o\\--", "o\\\\--", "\\--"] {
            assert_eq!(
                parse_arrow(token).head_half,
                Some(ArrowHalf::Bottom),
                "{token}"
            );
        }
    }

    #[test]
    fn circle_decoration_retains_head_direction_and_serialization_default() {
        for (token, direction, head, source, target) in [
            (
                "->o",
                ArrowDirection::LeftToRight,
                ArrowHead::Filled,
                false,
                true,
            ),
            (
                "o<--",
                ArrowDirection::RightToLeft,
                ArrowHead::Filled,
                false,
                true,
            ),
            (
                "o-->>o",
                ArrowDirection::LeftToRight,
                ArrowHead::Open,
                true,
                true,
            ),
            (
                "o<->o",
                ArrowDirection::Bidirectional,
                ArrowHead::Filled,
                true,
                true,
            ),
            (
                "-/o",
                ArrowDirection::LeftToRight,
                ArrowHead::Filled,
                false,
                true,
            ),
        ] {
            let arrow = parse_arrow(token);
            assert_eq!(arrow.direction, direction, "{token}");
            assert_eq!(arrow.head, head, "{token}");
            assert_eq!((arrow.source_circle, arrow.target_circle), (source, target));
            let mut value = serde_json::to_value(&arrow).unwrap();
            value.as_object_mut().unwrap().remove("target_circle");
            let legacy: Arrow = serde_json::from_value(value).unwrap();
            assert!(!legacy.target_circle);
        }
    }

    #[test]
    fn circle_grammar_retains_valid_heads_and_rejects_interior_decoration() {
        for source in ["A -[#a]>o B", "A o<-- B", "[->o A", "A ->o]", "A o-->>o A"] {
            assert!(parse_sequence(&[source.into()]).is_ok(), "{source}");
        }
        for source in ["A -[#a]o> B", "[--o> A", "A --o>]", "A <o-- B"] {
            assert!(parse_sequence(&[source.into()]).is_err(), "{source}");
        }
    }

    #[test]
    fn pending_create_overwrites_only_unattached_events() {
        for earlier in ["Birch", "Willow"] {
            let input = format!(
                "participant Ash\nparticipant Birch\nparticipant Willow\ncreate {earlier}\ncreate Birch\nAsh -> Birch: one\nAsh -> {earlier}: ordinary\ncreate Birch\nAsh -> Birch: two"
            );
            let diagram = parse(&input);
            let owners = sequence_create_attachments(&diagram.events).unwrap();
            let creates: Vec<_> = diagram
                .events
                .iter()
                .enumerate()
                .filter_map(|(i, event)| matches!(event, Event::Create(_)).then_some(i))
                .collect();
            assert_eq!(creates.len(), 3);
            assert_eq!(owners[creates[0]], None);
            assert_eq!(owners[creates[1]], Some(creates[1] + 1));
            assert_eq!(owners[creates[2]], Some(creates[2] + 1));
        }
        for message in ["Birch -> Ash", "Birch -> Birch"] {
            let lines =
                format!("participant Ash\nparticipant Birch\ncreate Birch\n{message}: invalid")
                    .lines()
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
            assert!(parse_sequence(&lines).is_err(), "{message}");
        }
        for message in ["[-> Birch", "Birch ->]"] {
            let diagram = parse(&format!(
                "participant Birch\ncreate Birch\n{message}: external"
            ));
            assert_eq!(
                sequence_create_attachments(&diagram.events)
                    .unwrap()
                    .iter()
                    .filter(|x| x.is_some())
                    .count(),
                1
            );
        }
    }

    #[test]
    fn participant_labels_keep_display_lines_separate_from_identity() {
        for kind in ["participant", "actor", "queue"] {
            let diagram = parse(&format!(
                r#"{kind} "Long\nShort\nLast" as Mail
Gate -> Mail: enqueue"#
            ));
            let participant = diagram
                .participants
                .iter()
                .find(|p| p.id == "Mail")
                .unwrap();
            assert_eq!(participant.label, "Long\nShort\nLast");
            assert!(matches!(&diagram.events[0], Event::Message(message) if message.to == "Mail"));
        }
    }

    #[test]
    fn inline_sequence_display_normalizes_before_model_storage() {
        for command in ["note right", "note across", "note over Cedar"] {
            let diagram = parse(&format!(
                "Cedar -> Pine : enter\n{command} : first\\nsecond"
            ));
            let note = diagram
                .events
                .iter()
                .find_map(|event| match event {
                    Event::Note(note) => Some(note),
                    _ => None,
                })
                .unwrap();
            assert_eq!(note.text, "first\nsecond", "{command}");
        }
        // Display.create for a block retains literal escapes; getWithNewlines
        // for an inline Display also leaves math and link bodies untouched.
        assert_eq!(
            inline_sequence_display(r"a\\n b\t c\nend"),
            "a\\n b\t c\nend"
        );
        assert_eq!(
            inline_sequence_display(r"<math>x\ny</math>\nz"),
            "<math>x\\ny</math>\nz"
        );
        assert_eq!(
            inline_sequence_display(r"[[https://example.test/x\ny link]]\nz"),
            "[[https://example.test/x\\ny link]]\nz"
        );
        let block = parse("Cedar -> Pine : enter\nnote right\nfirst\\nsecond\nend note");
        let Event::Note(note) = &block.events[1] else {
            panic!("expected note")
        };
        assert_eq!(note.text, r"first\nsecond");
    }

    fn parse(input: &str) -> SequenceDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_sequence(&lines).unwrap()
    }

    #[test]
    fn external_dressing_selects_direction_independently_of_border_syntax() {
        for border in ["[", "]"] {
            for (head, arrow_head) in [(">", ArrowHead::Filled), (">>", ArrowHead::Open)] {
                let left_head = head.replace('>', "<");
                for (source, incoming) in [
                    (format!("{border}-{head} Iris ++ : enter"), true),
                    (format!("Iris {left_head}-{border} ++ : enter"), true),
                    (format!("{border}{left_head}- Iris ++ : enter"), false),
                    (format!("Iris -{head}{border} ++ : enter"), false),
                ] {
                    let diagram = parse(&source);
                    let Event::Message(message) = &diagram.events[0] else {
                        panic!("expected message for {source}");
                    };
                    let expected = if incoming {
                        (border, "Iris")
                    } else {
                        ("Iris", border)
                    };
                    assert_eq!(
                        (message.from.as_str(), message.to.as_str()),
                        expected,
                        "{source}"
                    );
                    assert_eq!(message.arrow.head, arrow_head, "{source}");
                    assert!(matches!(
                        message.activation,
                        Some(ActivationChange::Activate)
                    ));
                    assert_eq!(diagram.participants.len(), 1);
                }
            }
        }
    }

    #[test]
    fn simple_message() {
        let d = parse("Alice -> Bob : hello");
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].id, "Alice");
        assert_eq!(d.participants[1].id, "Bob");
        assert_eq!(d.events.len(), 1);
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "Alice");
            assert_eq!(m.to, "Bob");
            assert_eq!(m.label, "hello");
            assert_eq!(m.arrow.line, LineStyle::Solid);
            assert_eq!(m.arrow.head, ArrowHead::Filled);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn message_label_preserves_space_before_escaped_newline() {
        let d = parse(r"Alice -> Bob: \n \t \\");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.label, r" \n \t \\");
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn local_link_message_label_is_not_arrow_color() {
        let d = parse("Alice -> Bob : [[#anchor local link]]");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.label, "[[#anchor local link]]");
            assert_eq!(m.arrow.color, None);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn quoted_participant_colon_does_not_expose_label_brackets() {
        let d = parse(r#""Cache: primary" -[#AD1457,dashed]> B : [[#anchor label]]"#);
        let Event::Message(message) = &d.events[0] else {
            panic!("expected message");
        };
        assert_eq!(message.from, "Cache: primary");
        assert_eq!(message.label, "[[#anchor label]]");
        assert_eq!(message.arrow.color.as_deref(), Some("#AD1457"));
        assert_eq!(message.arrow.line, LineStyle::Dotted);
    }

    #[test]
    fn style_looking_brackets_in_quoted_participants_are_not_arrow_styles() {
        let d =
            parse(r#""Source [hidden]: 9401" -[#AD1457,dashed]> "Target: 9403" : visible message"#);
        let Event::Message(message) = &d.events[0] else {
            panic!("expected message");
        };
        assert_eq!(message.from, "Source [hidden]: 9401");
        assert_eq!(message.to, "Target: 9403");
        assert_eq!(message.arrow.color.as_deref(), Some("#AD1457"));
        assert_eq!(message.arrow.line, LineStyle::Dotted);
        assert_eq!(d.participants[0].label, "Source [hidden]: 9401");
    }

    #[test]
    fn inline_arrow_style_keeps_color_and_body_orthogonal() {
        let d = parse(
            "A -[#C2185B,dashed]> B : color first\n\
             B -[dotted,#1565C0]> A : style first\n\
             A -[#red,#blue]> B : final color\n\
             A -[bold]> B : accepted no-op\n\
             A -[hidden]> B : hidden",
        );
        for (index, color) in [(0, "#C2185B"), (1, "#1565C0"), (2, "#blue")] {
            let Event::Message(message) = &d.events[index] else {
                panic!("expected message");
            };
            assert_eq!(message.arrow.color.as_deref(), Some(color));
        }
        let Event::Message(first) = &d.events[0] else {
            panic!("expected message");
        };
        let Event::Message(second) = &d.events[1] else {
            panic!("expected message");
        };
        let Event::Message(bold) = &d.events[3] else {
            panic!("expected message");
        };
        let Event::Message(hidden) = &d.events[4] else {
            panic!("expected message");
        };
        assert_eq!(first.arrow.line, LineStyle::Dotted);
        assert_eq!(second.arrow.line, LineStyle::Dotted);
        assert_eq!(bold.arrow.line, LineStyle::Solid);
        assert_eq!(hidden.arrow.line, LineStyle::Hidden);
    }

    #[test]
    fn hidden_normal_head_still_autoactivates_its_target() {
        let d = parse("autoactivate on\nA -[hidden]> B : hidden");
        let Event::Message(message) = &d.events[0] else {
            panic!("expected message");
        };
        assert_eq!(message.arrow.line, LineStyle::Hidden);
        assert_eq!(message.activation, Some(ActivationChange::Activate));
    }

    #[test]
    fn external_messages_share_inline_arrow_style_parser() {
        let d = parse(
            "[-[#AD1457,dashed]> Alice : found\n\
             Alice -[dotted,#2E7D32]>] : lost",
        );
        for (index, color) in [(0, "#AD1457"), (1, "#2E7D32")] {
            let Event::Message(message) = &d.events[index] else {
                panic!("expected message");
            };
            assert_eq!(message.arrow.color.as_deref(), Some(color));
            assert_eq!(message.arrow.line, LineStyle::Dotted);
        }
    }

    #[test]
    fn teoz_pragma_sets_sequence_flag() {
        let d = parse("!pragma teoz true\nAlice -> Bob : hello");
        assert!(d.teoz);
        assert_eq!(d.events.len(), 1);
    }

    #[test]
    fn dotted_arrow() {
        let d = parse("A --> B : reply");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.arrow.line, LineStyle::Dotted);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn bidirectional_arrows() {
        let d = parse("A <-> B : solid\nA <--> B : dotted\nA <<->> B : open");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "A");
            assert_eq!(m.to, "B");
            assert_eq!(m.arrow.direction, ArrowDirection::Bidirectional);
            assert_eq!(m.arrow.line, LineStyle::Solid);
            assert_eq!(m.arrow.head, ArrowHead::Filled);
        } else {
            panic!("expected message");
        }
        if let Event::Message(m) = &d.events[1] {
            assert_eq!(m.from, "A");
            assert_eq!(m.to, "B");
            assert_eq!(m.arrow.direction, ArrowDirection::Bidirectional);
            assert_eq!(m.arrow.line, LineStyle::Dotted);
            assert_eq!(m.arrow.head, ArrowHead::Filled);
        } else {
            panic!("expected message");
        }
        if let Event::Message(m) = &d.events[2] {
            assert_eq!(m.from, "A");
            assert_eq!(m.to, "B");
            assert_eq!(m.arrow.direction, ArrowDirection::Bidirectional);
            assert_eq!(m.arrow.line, LineStyle::Solid);
            assert_eq!(m.arrow.head, ArrowHead::Open);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn source_cross_arrow_keeps_target_head() {
        let d = parse("A x-> B : found\nA ->x B : lost");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "A");
            assert_eq!(m.to, "B");
            assert!(m.arrow.source_cross);
            assert_eq!(m.arrow.head, ArrowHead::Filled);
        } else {
            panic!("expected message");
        }
        if let Event::Message(m) = &d.events[1] {
            assert_eq!(m.from, "A");
            assert_eq!(m.to, "B");
            assert!(!m.arrow.source_cross);
            assert_eq!(m.arrow.head, ArrowHead::Cross);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn source_circle_arrow_keeps_target_head() {
        let d = parse("A o-> B : decorated");
        if let Event::Message(m) = &d.events[0] {
            assert!(m.arrow.source_circle);
            assert_eq!(m.arrow.head, ArrowHead::Filled);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn reverse_source_circle_keeps_semantic_source_and_target_head() {
        let d = parse("A <<-o B : open reverse\nA <-o B : filled reverse");
        let Event::Message(open) = &d.events[0] else {
            panic!("expected open reverse message");
        };
        assert_eq!(open.from, "B");
        assert_eq!(open.to, "A");
        assert!(open.arrow.source_circle);
        assert_eq!(open.arrow.head, ArrowHead::Open);

        let Event::Message(filled) = &d.events[1] else {
            panic!("expected filled reverse message");
        };
        assert_eq!(filled.from, "B");
        assert_eq!(filled.to, "A");
        assert!(filled.arrow.source_circle);
        assert_eq!(filled.arrow.head, ArrowHead::Filled);
    }

    #[test]
    fn left_half_head_dressing_reverses_semantic_participants() {
        let d = parse(
            "West /-o East : lower\n\
             East \\-o West : upper\n\
             Middle //-o East : thin",
        );
        let expected = [("East", "West"), ("West", "East"), ("East", "Middle")];
        for (event, (from, to)) in d.events.iter().zip(expected) {
            let Event::Message(message) = event else {
                panic!("expected message");
            };
            assert_eq!((message.from.as_str(), message.to.as_str()), (from, to));
            assert_eq!(message.arrow.direction, ArrowDirection::RightToLeft);
            assert!(message.arrow.source_circle);
        }
    }

    #[test]
    fn dotted_external_incoming_arrow() {
        let d = parse("[--> Alice : found dotted");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "[");
            assert_eq!(m.to, "Alice");
            assert_eq!(m.arrow.line, LineStyle::Dotted);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn dotted_external_incoming_arrow_preserves_right_boundary() {
        let d = parse("]--[dotted,#6D4C41]> Alice : found from right");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "]");
            assert_eq!(m.to, "Alice");
            assert_eq!(m.label, "found from right");
            assert_eq!(m.arrow.line, LineStyle::Dotted);
            assert_eq!(m.arrow.color.as_deref(), Some("#6D4C41"));
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn dotted_external_outgoing_arrow() {
        let d = parse("Alice -->] : lost dotted");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "Alice");
            assert_eq!(m.to, "]");
            assert_eq!(m.arrow.line, LineStyle::Dotted);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn dotted_external_outgoing_left_arrow() {
        let d = parse("Alice -->[ : lost left dotted");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "Alice");
            assert_eq!(m.to, "[");
            assert_eq!(m.label, "lost left dotted");
            assert_eq!(m.arrow.line, LineStyle::Dotted);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn external_incoming_activation_and_color() {
        let d = parse("[-> Alice ++ #red : found");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "[");
            assert_eq!(m.to, "Alice");
            assert_eq!(m.label, "found");
            assert_eq!(m.activation, Some(ActivationChange::Activate));
            assert_eq!(m.activation_color.as_deref(), Some("#red"));
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn external_outgoing_deactivation() {
        let d = parse("Alice -->] -- : lost return");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.from, "Alice");
            assert_eq!(m.to, "]");
            assert_eq!(m.label, "lost return");
            assert_eq!(m.activation, Some(ActivationChange::Deactivate));
            assert_eq!(m.activation_color, None);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn participant_declaration() {
        let d = parse("participant Alice\nactor Bob\nAlice -> Bob : hi");
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].kind, ParticipantKind::Participant);
        assert_eq!(d.participants[1].kind, ParticipantKind::Actor);
    }

    #[test]
    fn participant_alias() {
        let d = parse("participant \"Alice Johnson\" as A\nA -> A : self");
        assert_eq!(d.participants[0].id, "A");
        assert_eq!(d.participants[0].label, "Alice Johnson");
    }

    #[test]
    fn group_alt_else() {
        let d =
            parse("A -> B : check\nalt success\nB --> A : ok\nelse failure\nB --> A : err\nend");
        assert!(matches!(d.events[1], Event::GroupStart(_)));
        assert!(matches!(d.events[3], Event::GroupElse(_)));
        assert!(matches!(d.events[5], Event::GroupEnd));
    }

    #[test]
    fn note() {
        let d = parse("A -> B : msg\nnote right : hello");
        assert_eq!(d.events.len(), 2);
        if let Event::Note(n) = &d.events[1] {
            assert_eq!(n.position, NotePosition::Right);
            assert_eq!(n.text, "hello");
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn skinparam_blocks_are_flattened() {
        let d = parse(
            "skinparam note {\n  BackgroundColor LightYellow\n  BorderColor Orange\n  FontColor DarkBrown\n}\nA -> B : msg\nnote right : hello",
        );
        assert_eq!(d.meta.skinparams.len(), 3);
        assert_eq!(d.meta.skinparams[0].key, "noteBackgroundColor");
        assert_eq!(d.meta.skinparams[0].value, "LightYellow");
        assert_eq!(d.meta.skinparams[1].key, "noteBorderColor");
        assert_eq!(d.meta.skinparams[1].value, "Orange");
        assert_eq!(d.meta.skinparams[2].key, "noteFontColor");
        assert_eq!(d.meta.skinparams[2].value, "DarkBrown");
    }

    #[test]
    fn bare_side_note_before_message_is_ignored() {
        let d = parse("note left : orphan\nA -> B : msg");
        assert_eq!(d.events.len(), 1);
        assert!(matches!(d.events[0], Event::Message(_)));
    }

    #[test]
    fn divider() {
        let d = parse("A -> B : before\n== Phase 2 ==\nA -> B : after");
        assert!(matches!(d.events[1], Event::Divider(_)));
        if let Event::Divider(text) = &d.events[1] {
            assert_eq!(text, "Phase 2");
        }
    }

    #[test]
    fn delay() {
        let d = parse("A -> B : before\n...5 minutes later...\nA -> B : after");
        assert!(matches!(d.events[1], Event::Delay(Some(_))));
        if let Event::Delay(Some(text)) = &d.events[1] {
            assert_eq!(text, "5 minutes later");
        }
    }

    #[test]
    fn delay_preserves_component_label_whitespace() {
        for label in ["  wait ", "wait   ", ""] {
            let diagram = parse(&format!("A -> B : before\n...{label}...\nA -> B : after"));
            assert!(matches!(&diagram.events[1], Event::Delay(Some(text)) if text == label));
        }
        assert!(matches!(parse("A -> B\n...").events[1], Event::Delay(None)));
    }

    #[test]
    fn box_title_quote_forms_retain_label_color_and_membership() {
        for title in ["Outer domain", "\"Outer domain\""] {
            let diagram = parse(&format!(
                "box {title} #lightblue\nparticipant Oak\nend box\nparticipant Elm\nOak -> Elm : request"
            ));
            assert_eq!(diagram.boxes.len(), 1);
            assert_eq!(
                diagram.boxes[0].title,
                if title.starts_with('"') {
                    "Outer domain"
                } else {
                    "Outer domain "
                }
            );
            assert_eq!(diagram.boxes[0].color.as_deref(), Some("#lightblue"));
            assert_eq!(diagram.boxes[0].members, vec![0]);
        }
    }

    #[test]
    fn spacing() {
        let d = parse("A -> B : m1\n|||\nA -> B : m2\n||45||\nA -> B : m3");
        assert!(matches!(d.events[1], Event::Space(None)));
        assert!(matches!(d.events[3], Event::Space(Some(45))));
    }

    #[test]
    fn autonumber() {
        let d = parse("autonumber\nA -> B : first\nB -> A : second");
        assert!(d.autonumber.is_some());
        let an = d.autonumber.unwrap();
        assert_eq!(an.start, 1);
        assert_eq!(an.step, 1);
    }

    #[test]
    fn autonumber_with_params() {
        let d = parse("autonumber 10 5 \"[000]\"");
        let an = d.autonumber.unwrap();
        assert_eq!(an.start, 10);
        assert_eq!(an.step, 5);
        assert_eq!(an.format.as_deref(), Some("[000]"));
    }

    #[test]
    fn activation() {
        let d = parse("A -> B ++ : activate\nB --> A -- : return");
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.activation, Some(ActivationChange::Activate));
        }
        if let Event::Message(m) = &d.events[1] {
            assert_eq!(m.activation, Some(ActivationChange::Deactivate));
        }
    }

    #[test]
    fn autoactivate_directive_applies_to_subsequent_messages() {
        let d = parse(
            "autoactivate on\nA -> B : call\nB --> A : done\nautoactivate off\nA -> B : later",
        );
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].id, "A");
        assert_eq!(d.participants[1].id, "B");
        assert_eq!(d.events.len(), 3);
        if let Event::Message(m) = &d.events[0] {
            assert_eq!(m.activation, Some(ActivationChange::Activate));
        } else {
            panic!("expected message");
        }
        if let Event::Message(m) = &d.events[1] {
            assert_eq!(m.activation, Some(ActivationChange::Deactivate));
        } else {
            panic!("expected message");
        }
        if let Event::Message(m) = &d.events[2] {
            assert_eq!(m.activation, None);
        } else {
            panic!("expected message");
        }
    }

    #[test]
    fn autoactivate_applies_to_external_messages() {
        let d = parse(
            "autoactivate on\n[-> A : incoming call\nA -->] : outgoing return\n\
             ]-> B : incoming from right\nB -->[ : outgoing to left",
        );
        let changes = d
            .events
            .iter()
            .map(|event| match event {
                Event::Message(message) => message.activation,
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            changes,
            vec![
                Some(ActivationChange::Activate),
                Some(ActivationChange::Deactivate),
                Some(ActivationChange::Activate),
                Some(ActivationChange::Deactivate),
            ]
        );
    }

    #[test]
    fn create_destroy() {
        let d = parse("A -> B : normal\ncreate C\nA -> C : create\ndestroy C");
        assert!(matches!(d.events[1], Event::Create(_)));
        assert!(matches!(d.events[3], Event::Destroy(_)));
    }

    #[test]
    fn return_message() {
        let d = parse("A -> B : request\nreturn response");
        assert!(matches!(d.events[1], Event::Return(_)));
        if let Event::Return(r) = &d.events[1] {
            assert_eq!(r.label, "response");
        }
    }

    #[test]
    fn command_history_preserves_parallel_return_and_destroy_stack() {
        let d = parse(
            "A -> B ++ : enter\nB -> C ++ : nested\ndestroy C\n& return #blue first\nreturn #red next",
        );
        let resolved = resolve_sequence_returns(&d.events).unwrap();
        let Event::Message(first) = &resolved[3] else {
            panic!("return lowered")
        };
        assert_eq!((&*first.from, &*first.to, first.parallel), ("C", "B", true));
        let Event::Message(next) = &resolved[4] else {
            panic!("return lowered")
        };
        assert_eq!((&*next.from, &*next.to, next.parallel), ("B", "A", false));
        let d = parse("A -> B : enter\n& A -> C : sibling");
        assert!(matches!(&d.events[1], Event::Message(m) if m.parallel));
    }

    #[test]
    fn command_life_transitions_and_return_colors_are_validated() {
        for call in ["A -> B", "B -> B"] {
            for close in ["deactivate B", "destroy B"] {
                let lines: Vec<String> =
                    format!("A -> B ++ : begin\n{call} : row\n{close}\nactivate B")
                        .lines()
                        .map(str::to_owned)
                        .collect();
                assert!(
                    parse_sequence(&lines)
                        .unwrap_err()
                        .message
                        .contains("already done")
                );
            }
        }
        // A self message ignores another participant's flags, per addLifeEvent.
        parse("A -> B : begin\nB -> B : row\ndeactivate A\nactivate A");
        for color in [
            "#f",
            "#123",
            "#123abc",
            "#123abc80",
            "#BlueViolet",
            "#transparent",
        ] {
            assert!(valid_return_color(color));
        }
        for color in ["#12", "#12345", "#noSuchColor819", "#red_foo"] {
            assert!(!valid_return_color(color));
            let lines: Vec<String> = format!("A -> B : enter\nreturn {color} done")
                .lines()
                .map(str::to_owned)
                .collect();
            assert_eq!(parse_sequence(&lines).unwrap_err().message, "No such color");
        }
    }

    #[test]
    fn return_color_and_source_history() {
        for prefix in ["", "!pragma teoz true\n"] {
            for call in ["A -> A ++", "A -> B ++", "[-> B ++"] {
                let d = parse(&format!("{prefix}{call} : enter\nreturn #purple settled"));
                let ret = d
                    .events
                    .iter()
                    .find_map(|event| match event {
                        Event::Return(ret) => Some(ret),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(ret.color.as_deref(), Some("#purple"));
                assert_eq!(ret.label, "settled");
            }
        }
        let d = parse("A -> B : enter\nreturn #purple");
        let Event::Return(ret) = &d.events[1] else {
            panic!("return");
        };
        assert_eq!(ret.label, "#purple");
        assert_eq!(ret.color, None);
        // Initial life is not an activating message; END replaces fallback.
        for body in [
            "participant A\nreturn lost",
            "activate A\nreturn lost",
            "[-> A : incoming\nreturn lost",
            "group G\nA -> B : enter\nend\nreturn lost",
        ] {
            let lines = body.lines().map(str::to_owned).collect::<Vec<_>>();
            let error = parse_sequence(&lines).unwrap_err();
            assert_eq!(error.message, "Nowhere to return to.");
            assert_eq!(error.line, lines.len());
        }
        for body in [
            "A -> B : enter\nnote over A : note\nreturn okay",
            "group G\nA -> B ++ : enter\nend\nreturn okay",
            "A -> B ++ : enter\nreturn one\nreturn two",
        ] {
            parse(body);
        }
    }

    #[test]
    fn title_and_meta() {
        let d = parse("title My Diagram\nheader Top\nfooter Bottom\ncaption Fig 1\nA -> B : msg");
        assert_eq!(d.meta.title.as_deref(), Some("My Diagram"));
        assert_eq!(d.meta.header.as_deref(), Some("Top"));
        assert_eq!(d.meta.footer.as_deref(), Some("Bottom"));
        assert_eq!(d.meta.caption.as_deref(), Some("Fig 1"));
    }

    #[test]
    fn ref_over() {
        let d = parse("ref over A, B : See other diagram");
        if let Event::Ref(r) = &d.events[0] {
            assert_eq!(r.participants, vec!["A", "B"]);
            assert_eq!(r.text, "See other diagram");
        } else {
            panic!("expected ref");
        }
    }

    #[test]
    fn newpage() {
        let d = parse("A -> B : page1\nnewpage\nA -> B : page2");
        assert!(matches!(d.events[1], Event::NewPage(None)));
    }

    #[test]
    fn all_participant_types() {
        let d = parse(
            "participant P\nactor A\nboundary B\ncontrol C\n\
             entity E\ndatabase D\ncollections Co\nqueue Q",
        );
        assert_eq!(d.participants.len(), 8);
        assert_eq!(d.participants[1].kind, ParticipantKind::Actor);
        assert_eq!(d.participants[2].kind, ParticipantKind::Boundary);
        assert_eq!(d.participants[5].kind, ParticipantKind::Database);
        assert_eq!(d.participants[7].kind, ParticipantKind::Queue);
    }

    #[test]
    fn note_on_link_is_not_registered_for_sequence() {
        // SequenceDiagramFactory registers FactorySequenceNoteOnArrowCommand,
        // whose POSITION is left/right/top/bottom. `note on link` belongs to
        // other diagram factories and must not be silently accepted here.
        let lines = [
            "A -> B : msg".to_string(),
            "note on link : link note".to_string(),
        ];
        assert!(parse_sequence(&lines).is_err());
    }

    #[test]
    fn multiline_note() {
        let d = parse("A -> B : msg\nnote left\n  Line 1\n  Line 2\nendnote");
        assert_eq!(d.events.len(), 2);
        if let Event::Note(n) = &d.events[1] {
            assert_eq!(n.position, NotePosition::Left);
            assert_eq!(n.text, "Line 1\nLine 2");
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn multiline_note_preserves_relative_code_indent() {
        let d = parse(
            "A -> B : msg\nnote over A\n  <code>\n  function foo() {\n    return 42;\n  }\n  </code>\nend note",
        );
        if let Event::Note(n) = &d.events[1] {
            assert_eq!(n.text, "<code>\nfunction foo() {\n  return 42;\n}\n</code>");
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn multiline_note_end_note() {
        let d = parse("A -> B : msg\nnote right of B\n  First\n  Second\nend note");
        if let Event::Note(n) = &d.events[1] {
            assert_eq!(n.position, NotePosition::Right);
            assert!(n.text.contains("First"));
        } else {
            panic!("expected note");
        }
    }

    #[test]
    fn multiline_note_implicit_participant_uses_first_body_line_source() {
        let d = parse("\n\n\nnote over Alice\n  First\nend note\nAlice -> Bob");
        let alice = d
            .participants
            .iter()
            .find(|p| p.id == "Alice")
            .expect("Alice participant");
        assert_eq!(alice.source_line, 5);
    }

    #[test]
    fn participant_with_url() {
        let d = parse(
            "actor User [[https://example.com/user]]\nparticipant API [[https://example.com/api]]\nUser -> API : hello",
        );
        assert_eq!(d.participants.len(), 2);
        assert_eq!(
            d.participants[0].url.as_deref(),
            Some("https://example.com/user")
        );
        assert_eq!(
            d.participants[1].url.as_deref(),
            Some("https://example.com/api")
        );
    }

    #[test]
    fn participant_no_url() {
        let d = parse("participant Alice\nparticipant Bob\nAlice -> Bob : hi");
        assert_eq!(d.participants[0].url, None);
        assert_eq!(d.participants[1].url, None);
    }
    #[test]
    fn participant_display_visibility_and_declaration_order() {
        let d = parse(
            "skinparam stereotypePosition bottom\nqueue Mail <<work>>\nskinparam stereotypePosition top\nqueue Other <<role>>\nMail -> Other\nhide stereotypes",
        );
        assert!(d.hide_stereotypes);
        assert!(d.participants[0].stereotype_position_bottom);
        assert!(!d.participants[1].stereotype_position_bottom);
        assert_eq!(d.participants[0].stereotype.as_deref(), Some("work"));
        let d = parse(
            "hide stereotype\nactor Gate <<role>>\nparticipant Hub <<service>>\nHub -> Gate\nshow stereotypes",
        );
        assert!(!d.hide_stereotypes);
    }

    #[test]
    fn participant_display_default_serialization_is_backward_compatible() {
        let d = parse("queue Mail <<work>>\nHub -> Mail");
        let yaml = serde_yml::to_string(&d).unwrap();
        assert!(!yaml.contains("hide_stereotypes"));
        assert!(!yaml.contains("stereotype_position_bottom"));
        // An old model has neither key. Defaults must not change its full YAML.
        let legacy: SequenceDiagram = serde_yml::from_str(&yaml).unwrap();
        assert!(!legacy.hide_stereotypes);
        assert!(
            legacy
                .participants
                .iter()
                .all(|p| !p.stereotype_position_bottom)
        );
        assert_eq!(serde_yml::to_string(&legacy).unwrap(), yaml);
        let d = parse(
            "skinparam stereotypePosition bottom\nqueue Mail <<work>>\nHub -> Mail\nhide stereotype",
        );
        let yaml = serde_yml::to_string(&d).unwrap();
        let restored: SequenceDiagram = serde_yml::from_str(&yaml).unwrap();
        assert!(restored.hide_stereotypes);
        assert!(restored.participants[0].stereotype_position_bottom);
        assert_eq!(serde_yml::to_string(&restored).unwrap(), yaml);
    }
    #[test]
    fn external_message_owns_bare_note_after_direction_normalization() {
        for (arrow, from, to) in [
            ("[-> Hub", "[", "Hub"),
            ("[<- Hub", "Hub", "["),
            ("Hub ->]", "Hub", "]"),
            ("Hub <-]", "]", "Hub"),
        ] {
            for body in ["note right: current", "note left\ncurrent\nend note"] {
                let d = parse(&format!("Birch -> Hub: prior\n{arrow}: external\n{body}"));
                let Some(Event::Note(note)) = d.events.last() else {
                    panic!("missing note: {arrow}")
                };
                assert!(note.on_message);
                assert_eq!(note.participants, [from, to]);
                assert_eq!(note.text, "current");
                assert_eq!(d.participants.len(), 2, "border is not a participant");
            }
        }
    }
    #[test]
    fn standalone_note_terminators_consume_the_complete_command() {
        for shape in ["note", "rnote", "hnote"] {
            for ending in [
                "endnote",
                "end note",
                "endrnote",
                "end rnote",
                "endhnote",
                "end hnote",
            ] {
                let d = parse(&format!(
                    "participant Hub\n{shape} over Hub\none\ntwo\n{ending}\nHub -> Hub: after"
                ));
                assert_eq!(d.events.len(), 2);
                let Event::Note(note) = &d.events[0] else {
                    panic!("missing note")
                };
                assert_eq!(note.text, "one\ntwo");
                assert!(
                    matches!(&d.events[1], Event::Message(message) if message.label == "after")
                );
            }
        }
    }
    #[test]
    fn java_pattern2_whitespace_is_shared_across_sequence_commands() {
        for (space, valid) in [
            (' ', true),
            ('\t', true),
            ('\u{00a0}', true),
            ('\u{2003}', false),
            ('\u{202f}', false),
        ] {
            let mut parser = SeqParser::new();
            assert_eq!(
                parser.try_participant_decl(&format!("participant{space}Birch")),
                valid
            );
            assert_eq!(
                parser.try_message(&format!("Birch{space}->{space}Elm")),
                valid
            );
            let body = format!("participant Birch\nrnote over Birch\nbody\nend{space}rnote");
            let lines = body.lines().map(str::to_owned).collect::<Vec<_>>();
            assert_eq!(parse_sequence(&lines).is_ok(), valid, "{body:?}");
        }
        let whitespace = sequence_regex(r"^[%s]$").unwrap();
        let non_whitespace = sequence_regex(r"^[^%s]$").unwrap();
        for character in ['\t', '\n', '\u{000b}', '\u{000c}', '\r', ' ', '\u{00a0}'] {
            assert!(whitespace.is_match(&character.to_string()));
            assert!(!non_whitespace.is_match(&character.to_string()));
        }
    }
    #[test]
    fn pattern2_whitespace_composes_with_other_class_members() {
        let positive = sequence_regex(r"^[%s:]$").unwrap();
        let negative = sequence_regex(r"^[^%s:]$").unwrap();
        for c in [' ', '\t', '\u{000b}', '\u{000c}', '\u{00a0}', ':'] {
            assert!(positive.is_match(&c.to_string()));
            assert!(!negative.is_match(&c.to_string()));
        }
        assert!(negative.is_match("x"));
        for shape in ["note", "rnote", "hnote"] {
            for target in ["over Birch", "right", "across"] {
                for separator in ["", " ", "\t", "\u{00a0}"] {
                    let diagram = parse(&format!(
                        "Birch -> Elm: seed\n{shape} {target} #pink{separator}: body"
                    ));
                    let Some(Event::Note(note)) = diagram.events.last() else {
                        panic!("missing note")
                    };
                    assert_eq!(note.color.as_deref(), Some("#pink"));
                }
            }
        }
    }

    #[test]
    fn command_entry_whitespace_follows_java_composition() {
        for gap in [' ', '\t', '\u{000c}', '\u{00a0}', '\u{000b}'] {
            let mut parser = SeqParser::new();
            assert!(parser.try_box(&format!("box{gap}\"Northern Shore\" #pink")));
            assert_eq!(parser.boxes[0].title, "Northern Shore");
            assert_eq!(parser.boxes[0].color.as_deref(), Some("#pink"));
            assert!(parser.try_note(&format!("rnote{gap}across: body")).unwrap());
            assert!(
                matches!(parser.events.last(), Some(Event::Note(note)) if note.shape == NoteShape::Rectangular)
            );
            assert_eq!(
                parser.try_autonumber(&format!("autonumber{gap}resume")),
                gap != '\u{000b}'
            );
            assert!(parser.try_autonumber(&format!("autonumber{gap}resume \"Item 00\"")));
        }
        for shape in ["note", "rnote", "hnote"] {
            let mut parser = SeqParser::new();
            assert!(parser.try_note(&format!("{shape} accross: body")).is_err());
        }
    }

    #[test]
    fn created_participant_signature_uses_the_complete_command() {
        for declaration in ["create actor\u{000b}Willow", "create\u{000b}actor Willow"] {
            let diagram = parse(&format!(
                "participant Birch\n{declaration}\nBirch -> Willow: born"
            ));
            assert_eq!(diagram.participants[1].kind, ParticipantKind::Actor);
            assert!(matches!(&diagram.events[0], Event::Create(id) if id == "Willow"));
        }
    }

    #[test]
    fn fox_space_prefilter_checks_the_complete_command() {
        for gap in [' ', '\t', '\u{000c}', '\u{00a0}', '\u{000b}'] {
            let required = gap != '\u{000b}';
            let mut parser = SeqParser::new();
            assert_eq!(
                parser.try_participant_decl(&format!("participant{gap}Birch")),
                required
            );
            assert!(parser.try_participant_decl(&format!("participant{gap}\"Birch Grove\"")));
            assert_eq!(
                parser.try_activate_deactivate(&format!("activate{gap}Birch")),
                required
            );
            assert_eq!(
                parser.try_create_destroy(&format!("destroy{gap}Birch")),
                required
            );
            assert_eq!(parser.try_hide(&format!("hide{gap}stereotype")), required);
            // CommandArrow and CommandGrouping have no direct mandatory space leaf.
            assert!(parser.try_message(&format!("Birch{gap}->{gap}Elm")));
            assert!(parser.try_group(&format!("group{gap}title")));
        }
        let mut parser = SeqParser::new();
        parser.parse_line(1, "activate\u{000b}Birch").unwrap();
        assert!(parser.unconsumed); // T14.17 owns the outer failure propagation.
        assert!(parser.events.is_empty());
    }

    #[test]
    fn named_note_identity_is_required_after_optional_prefix() {
        for shape in ["note", "rnote", "hnote"] {
            for (command, identities) in [
                (format!("{shape} over of : body"), vec!["of"]),
                (format!("{shape} over of #pink: body"), vec!["of"]),
                (format!("{shape} over of of : body"), vec!["of"]),
                (format!("{shape} over of, South: body"), vec!["of", "South"]),
                (
                    format!("{shape} left of \"North Pier\": body"),
                    vec!["North Pier"],
                ),
                (format!("{shape} over of\nbody\nend note"), vec!["of"]),
                (
                    format!("{shape} over of  North\nbody\nend note"),
                    vec!["North"],
                ),
            ] {
                let diagram = parse(&command);
                let Event::Note(note) = &diagram.events[0] else {
                    panic!("missing note: {command}")
                };
                assert_eq!(note.participants, identities, "{command}");
                assert!(!note.on_message);
                assert_eq!(note.text, "body");
            }
        }
    }

    #[test]
    fn note_over_requires_a_named_participant() {
        for text in [
            "note over: body",
            "note over\nbody\nend note",
            "rnote over: body",
            "hnote over: body",
        ] {
            let lines = format!("[-> Birch: message\n{text}")
                .lines()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert!(parse_sequence(&lines).is_err(), "{text}");
        }
        for text in [
            "note over Birch: body",
            "note over Birch,Elm: body",
            "note across: body",
            "note right: body",
        ] {
            let lines = format!("Birch -> Elm: message\n{text}")
                .lines()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert!(parse_sequence(&lines).is_ok(), "{text}");
        }
    }
}
