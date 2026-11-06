// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! The sprite commands registered by `CommonCommands#addCommonCommands2`
//! (pinned 71806a23), executed after TIM by each family's command dispatch.
//!
//! Java runs `TimLoader#load` (`!if`, `!procedure`, `!include`) inside
//! `BlockUml#<init>`; `PSystemCommandFactory#getCandidate` then walks the
//! post-TIM lines and offers each command-start line to the registered
//! commands in order. Lines owned by an earlier multiline command (a note,
//! legend, ref or class body) are never offered, and a family command
//! registered earlier wins. Families call [`dispatch`] at their own
//! registration position; nothing here runs inside the preprocessor.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use super::sequence::{registered_regex, sequence_command_trim, sequence_fox_signature};
use std::collections::HashMap;

use crate::diagram::{DiagramMeta, SpriteData};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpriteKind {
    FactoryMultiline,
    FactorySingle,
    Svg,
    Stdlib,
    StdlibSvg,
    SvgMultiline,
    File,
}

struct SpriteGrammar {
    kind: SpriteKind,
    class: &'static str,
    start: &'static str,
    /// RegexConcat#foxRegex over the direct RegexLeaf children.
    signature: u64,
    /// CommandMultilines2 END; `None` for SingleLineCommand2.
    end: Option<&'static str>,
}

// CommonCommands#addCommonCommands2 registration order. The patterns and
// signatures are the ones in REGISTERED_SEQUENCE_COMMANDS 22-28 (sequence
// inventory probe); `grammars_match_sequence_registry` keeps them identical.
const SPRITE_COMMANDS: [SpriteGrammar; 7] = [
    SpriteGrammar {
        kind: SpriteKind::FactoryMultiline,
        class: "CommandFactorySprite$1",
        start: r"^sprite[%s]+\$?([-.%pLN_]+)[%s]*(?:\[(\d+)x(\d+)/(?:(\d+)(z)?|(color))\])?[%s]*\{$",
        signature: 72057594041730113,
        end: Some(r"^end[%s]?sprite|\}$"),
    },
    SpriteGrammar {
        kind: SpriteKind::FactorySingle,
        class: "CommandFactorySprite$2",
        start: r"^sprite[%s]+\$?([-.%pLN_]+)[%s]*(?:\[(\d+)x(\d+)/(?:(\d+)(z)|(color))\])?[%s]+([-_A-Za-z0-9]+)$",
        signature: 3802177,
        end: None,
    },
    SpriteGrammar {
        kind: SpriteKind::Svg,
        class: "CommandSpriteSvg",
        start: r"^sprite[%s]+\$?([-%pLN_]+)[%s]+(\<svg\b.*\</svg\>)$",
        signature: 360639826101569,
        end: None,
    },
    SpriteGrammar {
        kind: SpriteKind::Stdlib,
        class: "CommandSpriteStdlib",
        start: r"^sprite[%s]+\$([-%pLN_]+)[%s]*#([^#]+)#([^%s]+)$",
        signature: 4298769473,
        end: None,
    },
    SpriteGrammar {
        kind: SpriteKind::StdlibSvg,
        class: "CommandSpriteStdlibSvg",
        start: r"^sprite[%s]+\$?([-%pLN_]+)[%s]*:([^:]+):([^%s]+)$",
        signature: 3802177,
        end: None,
    },
    SpriteGrammar {
        kind: SpriteKind::SvgMultiline,
        class: "CommandSpriteSvgMultiline",
        start: r"^sprite[%s]+\$?([-%pLN_]+)[%s]+(\<svg\b.*)$",
        signature: 70368756368705,
        end: Some(r"(.*\</svg\>)$"),
    },
    SpriteGrammar {
        kind: SpriteKind::File,
        class: "CommandSpriteFile",
        start: r"^sprite[%s]+\$?([-%pLN_]+)[%s]+([^<>%g#]*)$",
        signature: 3802177,
        end: None,
    },
];

struct CompiledGrammar {
    grammar: &'static SpriteGrammar,
    start: Regex,
    end: Option<Regex>,
}

static COMPILED: LazyLock<Vec<CompiledGrammar>> = LazyLock::new(|| {
    SPRITE_COMMANDS
        .iter()
        .map(|grammar| CompiledGrammar {
            grammar,
            start: registered_regex(grammar.start),
            end: grammar.end.map(registered_regex),
        })
        .collect()
});

// StringLocated#getTrimmed (StringUtils#trin) plus Pattern2's ASCII case
// folding. ASCII folding keeps byte offsets, so captures project back onto
// the original text.
fn command_text(line: &str) -> &str {
    sequence_command_trim(super::source_text(line))
}

struct Candidate {
    compiled: &'static CompiledGrammar,
    end: usize,
}

/// PSystemCommandFactory#getCandidate restricted to the sprite group.
/// `starts_multiline` reports a start line whose CommandMultilines2 never
/// completed (OK_PARTIAL until EOF, so isMultilineCommandOk returned null).
fn candidate(lines: &[String], index: usize) -> (Option<Candidate>, bool) {
    let text = command_text(&lines[index]);
    let folded = text.to_ascii_lowercase();
    let signature = sequence_fox_signature(&folded);
    let mut starts_multiline = false;
    for compiled in COMPILED.iter() {
        if compiled.grammar.signature & signature != compiled.grammar.signature
            || !compiled.start.is_match(&folded)
        {
            continue;
        }
        let Some(end_pattern) = &compiled.end else {
            return (
                Some(Candidate {
                    compiled,
                    end: index,
                }),
                starts_multiline,
            );
        };
        // CommandMultilines2#isValid: the first line alone is OK_PARTIAL; END
        // is tested (Trim.BOTH, Matcher.matches) on each following line.
        let end = (index + 1..lines.len())
            .find(|&i| end_pattern.is_match(&command_text(&lines[i]).to_ascii_lowercase()));
        match end {
            Some(end) => return (Some(Candidate { compiled, end }), starts_multiline),
            None => starts_multiline = true,
        }
    }
    (None, starts_multiline)
}

/// The last line a complete sprite command starting at `index` would own,
/// without executing it (for a family's structural pre-pass).
pub(super) fn complete_span(lines: &[String], index: usize) -> Option<usize> {
    candidate(lines, index).0.map(|candidate| candidate.end)
}

enum Execution {
    /// Pixel data the renderers can draw (`<$name>`).
    Pixel(String, SpriteData),
    /// Java would build a sprite this crate does not materialize.
    Unmaterialized,
}

fn execute(lines: &[String], index: usize, candidate: &Candidate) -> Result<Execution, String> {
    let text = command_text(&lines[index]);
    let folded = text.to_ascii_lowercase();
    let caps = candidate
        .compiled
        .start
        .captures(&folded)
        .expect("candidate start pattern matched");
    let original = |group: usize| caps.get(group).map(|m| &text[m.range()]);
    match candidate.compiled.grammar.kind {
        SpriteKind::FactoryMultiline | SpriteKind::FactorySingle => {
            let multiline = candidate.compiled.grammar.kind == SpriteKind::FactoryMultiline;
            // CommandFactorySprite#executeNow: trim, removeEmptyLines,
            // subExtract(1, 1); an empty body is an execution error.
            let rows: Vec<&str> = if multiline {
                lines[index + 1..candidate.end]
                    .iter()
                    .map(|line| super::source_text(line).trim())
                    .filter(|row| !row.is_empty())
                    .collect()
            } else {
                Vec::new()
            };
            if multiline && rows.is_empty() {
                return Err("No sprite defined.".to_string());
            }
            // CommandFactorySprite#executeInternal: DIM groups 2..5 are
            // width, height, graylevel, z and color.
            let width = caps.get(2).map(|m| m.as_str());
            let levels = caps.get(4).map(|m| m.as_str());
            let compressed = caps.get(5).is_some();
            if width.is_some() && caps.get(6).is_none() {
                let levels: u32 = levels.and_then(|l| l.parse().ok()).unwrap_or(0);
                if !matches!(levels, 4 | 8 | 16) {
                    return Err("Only 4, 8 or 16 graylevel are allowed.".to_string());
                }
            }
            if !multiline || compressed {
                // SpriteGrayLevel#buildSpriteZ decoding is not ported.
                return Ok(Execution::Unmaterialized);
            }
            let name = original(1).expect("NAME capture").to_string();
            // Existing pixel model: hex rows as written; width from DIM.
            let rows: Vec<String> = rows
                .into_iter()
                .filter(|row| row.chars().all(|c| c.is_ascii_hexdigit()))
                .map(str::to_string)
                .collect();
            let width = width.and_then(|w| w.parse().ok()).unwrap_or(0);
            // The pixel model records the rows written, not the DIM height.
            let height = rows.len() as u32;
            Ok(Execution::Pixel(
                name,
                SpriteData {
                    width,
                    height,
                    rows,
                },
            ))
        }
        SpriteKind::File => {
            // CommandSpriteFile#executeArg resolves FILE through
            // FileSystem#getFile; a missing file is "Cannot read: <src>".
            let src = original(2).unwrap_or_default();
            if src.starts_with("jar:") {
                return Ok(Execution::Unmaterialized);
            }
            let path = src.rfind('~').map_or(src, |tilde| &src[..tilde]);
            if std::path::Path::new(path).exists() {
                Ok(Execution::Unmaterialized)
            } else {
                Err(format!("Cannot read: {src}"))
            }
        }
        SpriteKind::Svg | SpriteKind::Stdlib | SpriteKind::StdlibSvg | SpriteKind::SvgMultiline => {
            Ok(Execution::Unmaterialized)
        }
    }
}

/// Offer a command-start line to the sprite group at the family's dispatch
/// position. `Ok(Some(end))`: the command owns `index..=end` and executed.
/// `Ok(None)`: no sprite command starts here. An execution failure, or a
/// multiline start that never completes with no other sprite command
/// accepting the line, is the error Java reports for that line.
///
/// `pixel_rendered` is false for families whose renderers do not draw
/// `<$name>` from the collected pixel data; executing such a sprite is
/// reported rather than silently dropped.
pub(super) fn dispatch(
    lines: &[String],
    index: usize,
    meta: &mut DiagramMeta,
    family: &str,
    pixel_rendered: bool,
) -> Result<Option<usize>, ParseError> {
    let (source_line, text) = super::source_line_and_trimmed(index + 1, &lines[index]);
    let (candidate, starts_multiline) = candidate(lines, index);
    let Some(candidate) = candidate else {
        if starts_multiline {
            // PSystemCommandFactory#executeFewLines: no candidate.
            return Err(ParseError {
                line: source_line,
                message: format!("Syntax Error? sprite command never completed: {text}"),
            });
        }
        return Ok(None);
    };
    let class = candidate.compiled.grammar.class;
    match execute(lines, index, &candidate) {
        Ok(Execution::Pixel(name, data)) => {
            if !pixel_rendered {
                eprintln!(
                    "warning: {family} sprite command {class} executed; {family} rendering of sprite references remains unsupported at line {source_line}"
                );
            }
            meta.sprites.insert(name, data);
        }
        Ok(Execution::Unmaterialized) => eprintln!(
            "warning: {family} sprite command {class} is recognized; sprite materialization remains unsupported at line {source_line}: {text}"
        ),
        Err(message) => {
            return Err(ParseError {
                line: source_line,
                message,
            });
        }
    }
    Ok(Some(candidate.end))
}

/// Execute a sprite command another registry (the sequence registry) has
/// already selected with the same grammar and END.
pub(super) fn execute_selected(
    lines: &[String],
    index: usize,
    end: usize,
    class: &str,
    meta: &mut DiagramMeta,
) -> Result<(), String> {
    let compiled = COMPILED
        .iter()
        .find(|compiled| compiled.grammar.class == class)
        .expect("sequence registry sprite class is in the CommonCommands group");
    match execute(lines, index, &Candidate { compiled, end })? {
        Execution::Pixel(name, data) => {
            meta.sprites.insert(name, data);
        }
        Execution::Unmaterialized => {}
    }
    Ok(())
}

/// Context-free consumption of complete sprite commands, for the families
/// whose dispatch position is not ported and for subtype detection only.
/// This is the former TIM behaviour moved after TIM: it never rejects, and
/// it leaves incomplete or failing commands to the family parser.
pub(super) fn consume_complete_commands(
    lines: &[String],
) -> (Vec<String>, HashMap<String, SpriteData>) {
    let mut output = lines.to_vec();
    let mut sprites = HashMap::new();
    let mut index = 0;
    while index < lines.len() {
        let (candidate, _) = candidate(lines, index);
        if let Some(candidate) = candidate
            && let Ok(execution) = execute(lines, index, &candidate)
        {
            if let Execution::Pixel(name, data) = execution {
                sprites.insert(name, data);
            }
            for line in &mut output[index..=candidate.end] {
                line.clear();
            }
            index = candidate.end + 1;
        } else {
            index += 1;
        }
    }
    (output, sprites)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn grammars_match_sequence_registry() {
        for grammar in &SPRITE_COMMANDS {
            let (start, signature, end) = super::super::sequence::registered_grammar(grammar.class)
                .expect("sprite class registered in the sequence inventory");
            assert_eq!(
                start,
                registered_regex(grammar.start).as_str(),
                "{}",
                grammar.class
            );
            assert_eq!(signature, grammar.signature, "{}", grammar.class);
            assert_eq!(
                end,
                grammar
                    .end
                    .map(|end| registered_regex(end).as_str().to_string()),
                "{}",
                grammar.class
            );
        }
    }

    #[test]
    fn pixel_block_executes_and_owns_its_body() {
        for (header, close) in [
            ("sprite $disk [8x8/16] {", "}"),
            ("SPRITE\u{00a0}$disk [8x8/16] {", "END SPRITE"),
            ("Sprite disk {", "endsprite"),
        ] {
            let input = lines(&format!("{header}\nFF00FF00\n\n00FF00FF\n{close}\nA -> B"));
            let mut meta = DiagramMeta::default();
            assert_eq!(
                dispatch(&input, 0, &mut meta, "test", true).ok(),
                Some(Some(4)),
                "{header}"
            );
            assert_eq!(meta.sprites["disk"].rows, ["FF00FF00", "00FF00FF"]);
            assert_eq!(meta.sprites["disk"].height, 2);
        }
    }

    #[test]
    fn execution_failures_are_errors() {
        let mut meta = DiagramMeta::default();
        let cases = [
            ("sprite $e {\n}", "No sprite defined."),
            (
                "sprite $g [2x2/7] {\nFF\n}",
                "Only 4, 8 or 16 graylevel are allowed.",
            ),
            // Unclosed pixel header: CommandFactorySprite never completes and
            // CommandSpriteFile accepts FILE='[2x2/16] {'.
            (
                "sprite $u [2x2/16] {\nFF\nA -> B",
                "Cannot read: [2x2/16] {",
            ),
            ("sprite -- [Engine]", "Cannot read: [Engine]"),
            // Top-level text is a command start; only command bodies are not.
            ("Sprite sheets are loaded", "Cannot read: are loaded"),
        ];
        for (source, message) in cases {
            let err = dispatch(&lines(source), 0, &mut meta, "test", true).unwrap_err();
            assert_eq!(err.message, message, "{source}");
        }
        // Unclosed SVG: CommandSpriteFile excludes '<', so there is no candidate.
        let err = dispatch(
            &lines("sprite $s <svg width=\"2\" height=\"2\">\n<rect/>\nA -> B"),
            0,
            &mut meta,
            "test",
            true,
        )
        .unwrap_err();
        assert!(err.message.starts_with("Syntax Error?"), "{}", err.message);
    }

    #[test]
    fn inline_svg_precedes_multiline_and_trailing_text_is_not_inline() {
        let mut meta = DiagramMeta::default();
        let inline = lines("sprite $t <svg><text> 'hi'</text></svg>\nA -> B");
        assert_eq!(
            dispatch(&inline, 0, &mut meta, "test", true).ok(),
            Some(Some(0))
        );
        let trailing = lines("sprite $t <svg><rect/></svg> ' trailing\nA -> B");
        assert!(dispatch(&trailing, 0, &mut meta, "test", true).is_err());
        let multi = lines("sprite $t <svg width=\"2\">\n<rect/>\n</svg>\nA -> B");
        assert_eq!(
            dispatch(&multi, 0, &mut meta, "test", true).ok(),
            Some(Some(2))
        );
    }

    #[test]
    fn non_sprite_lines_are_not_claimed() {
        let mut meta = DiagramMeta::default();
        for source in [
            "sprite -> bob : hi",
            "sprite --> Cedar : reply",
            "sprite foo",
            "A -> B",
        ] {
            assert_eq!(
                dispatch(&lines(source), 0, &mut meta, "test", true).ok(),
                Some(None),
                "{source}"
            );
        }
    }
}
