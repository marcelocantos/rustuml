// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::usecase::*;

/// Convert `<<foo>>` or `<< foo >>` stereotype syntax to `«foo»` guillemet form.
fn normalize_stereotype(s: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<<\s*([^>]+?)\s*>>").unwrap());
    RE.replace_all(s, |caps: &regex::Captures| format!("«{}»", caps[1].trim()))
        .into_owned()
}

/// Extract a stereotype string from a `<<foo>>` or `<< foo >>` token.
fn extract_stereotype_text(s: &str) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<<\s*([^>]+?)\s*>>").unwrap());
    RE.captures(s).map(|c| c[1].trim().to_string())
}

/// Extract a trailing `#color` modifier from an actor/use-case declaration
/// line, e.g. `actor User #Pink` → `Some("Pink")`, `usecase "X" #AAFFAA` →
/// `Some("AAFFAA")`. Returns the token without the leading `#`.
fn trailing_color(line: &str) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"#([0-9A-Za-z]+)\s*$").unwrap());
    RE.captures(line).map(|c| c[1].to_string())
}

fn usecase_separator_style(line: &str) -> Option<UseCaseSeparatorStyle> {
    let trimmed = line.trim();
    if trimmed.chars().count() < 2 || trimmed == "..." {
        return None;
    }
    let first = trimmed.chars().next()?;
    if !trimmed.chars().all(|ch| ch == first) {
        return None;
    }
    match first {
        '-' | '_' => Some(UseCaseSeparatorStyle::Solid),
        '=' => Some(UseCaseSeparatorStyle::Double),
        '.' => Some(UseCaseSeparatorStyle::Dotted),
        _ => None,
    }
}

/// Turn a label into a simple identifier (strip spaces, keep alphanumerics/underscores).
fn label_to_id(label: &str) -> String {
    label
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn remove_common_note_indent(lines: &[String]) -> Vec<String> {
    let common = lines
        .iter()
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.as_bytes()
                .iter()
                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                .count()
        })
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|line| line[common.min(line.len())..].to_string())
        .collect()
}

pub fn parse_usecase(lines: &[String]) -> Result<UseCaseDiagram, ParseError> {
    let mut actors: Vec<Actor> = Vec::new();
    let mut use_cases: Vec<UseCase> = Vec::new();
    let mut connections = Vec::new();
    let mut packages: Vec<UseCasePackage> = Vec::new();
    let mut notes: Vec<UseCaseNote> = Vec::new();
    let mut meta = DiagramMeta::default();
    let mut direction = UseCaseLayoutDirection::TopToBottom;
    let mut current_package: Option<usize> = None;
    // For multiline string literals in usecase declarations.
    let mut multiline_uc_id: Option<String> = None;
    let mut multiline_uc_color: Option<String> = None;
    let mut multiline_label_lines: Vec<String> = Vec::new();
    // Source line of the `usecase ID as "` opening for a multiline label.
    let mut multiline_start_line: usize = 0;
    // For multiline note blocks.
    let mut note_block: Option<PendingNote> = None;
    let mut note_block_lines: Vec<String> = Vec::new();
    // For `skinparam <prefix> { ... }` blocks: flatten nested `Key Value`
    // entries to `<prefix>Key`.
    let mut skinparam_block_prefix: Option<String> = None;

    // Regex patterns compiled once.

    // actor "Label" as ID <<stereotype>>  (all optional parts, color modifier ignored)
    static RE_ACTOR_QUOTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^actor\s+"((?:[^"]|"")+)"(?:\s+as\s+(\w+))?(?:\s+(<<\s*[^>]+\s*>>))?(?:\s+#\w+)?"#,
        )
        .unwrap()
    });
    // actor Word <<stereotype>>  (bare single-word name, optional color modifier)
    static RE_ACTOR_BARE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^actor\s+(\w+)(?:\s+(<<\s*[^>]+\s*>>))?(?:\s+#\w+)?"#).unwrap()
    });
    // :Label: shorthand actor syntax
    static RE_ACTOR_COLON: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^:([^:]+):\s*$").unwrap());

    // usecase "Label" as ID <<stereotype>>
    static RE_UC_QUOTED_AS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^usecase\s+"((?:[^"]|"")+)"\s+as\s+(\w+)(?:\s+(<<\s*[^>]+\s*>>))?"#).unwrap()
    });
    // usecase "Label" <<stereotype>>  (no alias)
    static RE_UC_QUOTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^usecase\s+"((?:[^"]|"")+)"(?:\s+(<<\s*[^>]+\s*>>))?(?:\s+#\w+)?"#).unwrap()
    });
    // usecase ID as "Label" <<stereotype>>  (reversed alias, closing quote required)
    static RE_UC_ID_AS_LABEL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r##"^usecase\s+(\w+)(?:\s+#\w+)?\s+as\s+"((?:[^"]|"")+)""##).unwrap()
    });
    // usecase (Label) as ID  (paren-based use case with alias)
    static RE_UC_PAREN_AS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^usecase\s+\(([^)]+)\)\s+as\s+(\w+)").unwrap());
    // usecase ID [#color] as " (multiline label start — opening quote not closed on same line)
    static RE_UC_ID_AS_MULTI: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^usecase\s+(\w+)(?:\s+#(\w+))?\s+as\s+"\s*$"#).unwrap());
    // usecase ID <<stereotype>>  (bare word, with optional stereotype/color)
    static RE_UC_BARE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^usecase\s+(\w+)(?:\s+(<<\s*[^>]+\s*>>))?(?:\s+#\w+)?"#).unwrap()
    });
    // (Label) shorthand use case
    static RE_UC_PAREN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\(([^)]+)\)\s*$").unwrap());

    // Connection: endpoint arrow endpoint : label
    // Endpoint can be:
    //   - bare word: \w+
    //   - quoted:    "..."
    //   - paren:     (...)
    static RE_CONN: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(\w+|"[^"]+"|[(][^)]+[)])\s*([-.<|>]+)\s*(\w+|"[^"]+"|[(][^)]+[)])\s*(?::\s*(.+))?$"#,
        )
        .unwrap()
    });

    // Inline entity note. The target is optional; Java attaches an omitted
    // target to the most recently declared entity.
    static RE_NOTE_INLINE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^note\s+(right|left|top|bottom)(?:\s+of\s+("[^"]+"|[(][^)]+[)]|[\w.]+))?\s*:\s*(.*)$"#,
        )
        .unwrap()
    });
    // Link note; position defaults to bottom in PlantUML.
    static RE_NOTE_ON_LINK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^note(?:\s+(right|left|top|bottom))?\s+(?:on|of)\s+link\s*:\s*(.*)$"#)
            .unwrap()
    });
    // Multiline entity note start.
    static RE_NOTE_BLOCK_ENTITY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^note\s+(right|left|top|bottom)(?:\s+of\s+("[^"]+"|[(][^)]+[)]|[\w.]+))?\s*$"#,
        )
        .unwrap()
    });
    // Multiline note-on-link start.
    static RE_NOTE_BLOCK_LINK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^note(?:\s+(right|left|top|bottom))?\s+(?:on|of)\s+link\s*$"#).unwrap()
    });
    // Floating note declarations.
    static RE_NOTE_FLOAT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^note\s+"([^"]+)"\s+as\s+([\w.]+)\s*$"#).unwrap());
    static RE_NOTE_FLOAT_BLOCK: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^note\s+as\s+([\w.]+)\s*$"#).unwrap());

    // Package/rectangle opening (with optional color/style modifiers). The
    // optional `#color` modifier (named or hex) before the brace is captured.
    static RE_PKG: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"^(rectangle|package)\s+(?:"([^"]+)"|(\w+))(?:\s+[^{]*?#([0-9A-Za-z]+))?(?:\s+[^{]*)?\{"#,
        )
        .unwrap()
    });

    for (line_idx, line) in lines.iter().enumerate() {
        let (current_line, trimmed) = super::source_line_and_trimmed(line_idx + 1, line);
        if trimmed.is_empty() {
            continue;
        }

        // Handle multiline string literal for usecase label.
        if let Some(ref uc_id) = multiline_uc_id.clone() {
            if trimmed.contains('"') {
                // End of multiline literal.
                // First non-separator, non-empty line is the title/label.
                let label = multiline_label_lines
                    .iter()
                    .find(|line| !line.trim().is_empty() && usecase_separator_style(line).is_none())
                    .map(|l| l.trim().to_string())
                    .unwrap_or_else(|| uc_id.clone());
                let mut description = Vec::new();
                let mut separators = Vec::new();
                for line in &multiline_label_lines {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Some(style) = usecase_separator_style(line) {
                        separators.push(UseCaseSeparator {
                            before_line: description.len(),
                            style,
                        });
                    } else {
                        description.push(line.trim().to_string());
                    }
                }
                let id = uc_id.clone();
                if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                    use_cases.push(UseCase {
                        id,
                        label,
                        explicit_id: true,
                        stereotype: None,
                        description,
                        separators,
                        color: multiline_uc_color.clone(),
                        source_line: multiline_start_line,
                    });
                }
                multiline_uc_id = None;
                multiline_uc_color = None;
                multiline_label_lines.clear();
            } else {
                multiline_label_lines.push(trimmed.to_string());
            }
            continue;
        }

        // Inside a `skinparam <prefix> { ... }` block: flatten nested
        // `Key Value` entries to `<prefix>Key` until the closing `}`. Must be
        // checked before the package-close handler so the block's `}` resets
        // the prefix rather than being swallowed as a package terminator.
        if let Some(prefix) = &skinparam_block_prefix {
            if trimmed == "}" {
                skinparam_block_prefix = None;
            } else {
                let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
                if parts.len() == 2 {
                    meta.skinparams.push(crate::diagram::SkinParam {
                        key: format!("{prefix}{}", parts[0]),
                        value: parts[1].trim().to_string(),
                    });
                }
            }
            continue;
        }

        if trimmed == "}" {
            current_package = None;
            continue;
        }

        // Handle multiline note block body.
        if let Some(pending) = note_block.as_ref() {
            if trimmed == "end note" {
                // Java `CommandFactoryNoteOnEntity.createMultiLine` delegates
                // to `BlocLines.removeEmptyColumns`: remove only indentation
                // shared by every body line and retain deeper Creole indents.
                let text = remove_common_note_indent(&note_block_lines)
                    .join("\n")
                    .trim_end()
                    .to_string();
                if !text.trim().is_empty() {
                    notes.push(UseCaseNote {
                        text,
                        kind: pending.kind.clone(),
                        position: pending.position,
                        source_line: pending.source_line,
                    });
                }
                note_block = None;
                note_block_lines.clear();
            } else {
                note_block_lines.push(line.trim_end().to_string());
            }
            continue;
        }

        // Collect skinparam directives into metadata.
        if let Some(rest) = trimmed.strip_prefix("skinparam ") {
            let rest = rest.trim();
            // Block form: `skinparam usecase {` opens a nested block.
            if let Some(prefix) = rest.strip_suffix('{') {
                skinparam_block_prefix = Some(prefix.trim().to_string());
                continue;
            }
            if let Some((key, value)) = rest.split_once(' ') {
                meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            continue;
        }
        // Skip top-level directives.
        if trimmed.starts_with("left to right direction") {
            direction = UseCaseLayoutDirection::LeftToRight;
            continue;
        }
        if trimmed.starts_with("top to bottom direction") {
            direction = UseCaseLayoutDirection::TopToBottom;
            continue;
        }
        if trimmed.starts_with("end note")
            || trimmed.starts_with("hide")
            || trimmed.starts_with("show")
            || trimmed.starts_with("!")
        {
            continue;
        }

        // title directive.
        if let Some(t) = trimmed.strip_prefix("title") {
            let title = super::strip_title_quotes(t).to_string();
            if !title.is_empty() {
                meta.title = Some(title);
                meta.title_line.get_or_insert(current_line);
            }
            continue;
        }

        // header directive.
        if let Some(h) = trimmed.strip_prefix("header") {
            let header = h.trim().to_string();
            if !header.is_empty() {
                meta.header = Some(header);
                meta.header_line.get_or_insert(current_line);
            }
            continue;
        }

        // footer directive.
        if let Some(f) = trimmed.strip_prefix("footer") {
            let footer = f.trim().to_string();
            if !footer.is_empty() {
                meta.footer = Some(footer);
                meta.footer_line.get_or_insert(current_line);
            }
            continue;
        }

        // A link note belongs to the most recently declared connection.
        if let Some(caps) = RE_NOTE_ON_LINK.captures(trimmed) {
            let Some(connection) = connections.len().checked_sub(1) else {
                continue;
            };
            notes.push(UseCaseNote {
                text: caps[2].trim().to_string(),
                kind: UseCaseNoteKind::OnLink { connection },
                position: note_position(caps.get(1).map(|m| m.as_str()).unwrap_or("bottom")),
                source_line: current_line,
            });
            continue;
        }

        // Inline note attached to an explicit target or the last entity.
        if let Some(caps) = RE_NOTE_INLINE.captures(trimmed) {
            let target = caps
                .get(2)
                .map(|m| normalize_endpoint(m.as_str()))
                .or_else(|| last_entity_id(&actors, &use_cases));
            let Some(target) = target else {
                continue;
            };
            notes.push(UseCaseNote {
                text: caps[3].trim().to_string(),
                kind: UseCaseNoteKind::Attached { target },
                position: note_position(&caps[1]),
                source_line: current_line,
            });
            continue;
        }

        // Floating notes are ordinary explicitly named note entities.
        if let Some(caps) = RE_NOTE_FLOAT.captures(trimmed) {
            notes.push(UseCaseNote {
                text: caps[1].trim().to_string(),
                kind: UseCaseNoteKind::Floating {
                    id: caps[2].to_string(),
                },
                position: UseCaseNotePosition::Bottom,
                source_line: current_line,
            });
            continue;
        }

        if let Some(caps) = RE_NOTE_BLOCK_LINK.captures(trimmed) {
            let Some(connection) = connections.len().checked_sub(1) else {
                continue;
            };
            note_block = Some(PendingNote {
                kind: UseCaseNoteKind::OnLink { connection },
                position: note_position(caps.get(1).map(|m| m.as_str()).unwrap_or("bottom")),
                // Java provenance: `CommandFactoryNoteOnLink` creates the
                // synthetic note entity on the line following the block
                // command, unlike the inline form which keeps the command line.
                source_line: current_line + 1,
            });
            note_block_lines.clear();
            continue;
        }

        if let Some(caps) = RE_NOTE_BLOCK_ENTITY.captures(trimmed) {
            let target = caps
                .get(2)
                .map(|m| normalize_endpoint(m.as_str()))
                .or_else(|| last_entity_id(&actors, &use_cases));
            let Some(target) = target else {
                continue;
            };
            note_block = Some(PendingNote {
                kind: UseCaseNoteKind::Attached { target },
                position: note_position(&caps[1]),
                // Java provenance: `CommandFactoryNoteOnEntity` assigns its
                // synthetic `GMN*` entity to the line after the block command.
                source_line: current_line + 1,
            });
            note_block_lines.clear();
            continue;
        }

        if let Some(caps) = RE_NOTE_FLOAT_BLOCK.captures(trimmed) {
            note_block = Some(PendingNote {
                kind: UseCaseNoteKind::Floating {
                    id: caps[1].to_string(),
                },
                position: UseCaseNotePosition::Bottom,
                source_line: current_line + 1,
            });
            note_block_lines.clear();
            continue;
        }

        if let Some(caps) = RE_ACTOR_QUOTED.captures(trimmed) {
            let label = caps[1].to_string();
            let id = caps
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| label_to_id(&label));
            let stereotype = caps
                .get(3)
                .and_then(|m| extract_stereotype_text(m.as_str()));
            if !actors.iter().any(|a: &Actor| a.id == id) {
                actors.push(Actor {
                    id,
                    label,
                    stereotype,
                    color: trailing_color(trimmed),
                    source_line: current_line,
                });
            }
        } else if let Some(caps) = RE_ACTOR_COLON.captures(trimmed) {
            let label = caps[1].trim().to_string();
            let id = label_to_id(&label);
            if !actors.iter().any(|a: &Actor| a.id == id) {
                actors.push(Actor {
                    id,
                    label,
                    stereotype: None,
                    color: None,
                    source_line: current_line,
                });
            }
        } else if let Some(caps) = RE_ACTOR_BARE.captures(trimmed) {
            let label = caps[1].to_string();
            let id = label.clone();
            let stereotype = caps
                .get(2)
                .and_then(|m| extract_stereotype_text(m.as_str()));
            if !actors.iter().any(|a: &Actor| a.id == id) {
                actors.push(Actor {
                    id,
                    label,
                    stereotype,
                    color: trailing_color(trimmed),
                    source_line: current_line,
                });
            }
        } else if let Some(caps) = RE_UC_ID_AS_LABEL.captures(trimmed) {
            // usecase ID as "Label" (single-line quoted label)
            let id = caps[1].to_string();
            let label = caps[2].to_string();
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase {
                    id,
                    label,
                    explicit_id: true,
                    stereotype: None,
                    description: Vec::new(),
                    separators: Vec::new(),
                    color: trailing_color(trimmed),
                    source_line: current_line,
                });
            }
        } else if RE_UC_ID_AS_MULTI.is_match(trimmed) {
            // Multiline label: usecase ID as "...
            let caps = RE_UC_ID_AS_MULTI.captures(trimmed).unwrap();
            let id = caps[1].to_string();
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            multiline_uc_id = Some(id);
            multiline_uc_color = caps.get(2).map(|m| m.as_str().to_string());
            multiline_label_lines.clear();
            multiline_start_line = current_line;
        } else if let Some(caps) = RE_UC_PAREN_AS.captures(trimmed) {
            // usecase (Label) as ID
            let label = caps[1].trim().to_string();
            let id = caps[2].to_string();
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase {
                    id,
                    label,
                    explicit_id: true,
                    stereotype: None,
                    description: Vec::new(),
                    separators: Vec::new(),
                    color: trailing_color(trimmed),
                    source_line: current_line,
                });
            }
        } else if let Some(caps) = RE_UC_QUOTED_AS.captures(trimmed) {
            let label = caps[1].to_string();
            let id = caps[2].to_string();
            let stereotype = caps
                .get(3)
                .and_then(|m| extract_stereotype_text(m.as_str()));
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase {
                    id,
                    label,
                    explicit_id: true,
                    stereotype,
                    description: Vec::new(),
                    separators: Vec::new(),
                    color: trailing_color(trimmed),
                    source_line: current_line,
                });
            }
        } else if let Some(caps) = RE_UC_QUOTED.captures(trimmed) {
            let label = caps[1].to_string();
            let id = label_to_id(&label);
            let stereotype = caps
                .get(2)
                .and_then(|m| extract_stereotype_text(m.as_str()));
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase {
                    id,
                    label,
                    explicit_id: false,
                    stereotype,
                    description: Vec::new(),
                    separators: Vec::new(),
                    color: trailing_color(trimmed),
                    source_line: current_line,
                });
            }
        } else if let Some(caps) = RE_UC_BARE.captures(trimmed) {
            let id = caps[1].to_string();
            let label = id.clone();
            let stereotype = caps
                .get(2)
                .and_then(|m| extract_stereotype_text(m.as_str()));
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase {
                    id,
                    label,
                    explicit_id: false,
                    stereotype,
                    description: Vec::new(),
                    separators: Vec::new(),
                    color: trailing_color(trimmed),
                    source_line: current_line,
                });
            }
        } else if let Some(caps) = RE_UC_PAREN.captures(trimmed) {
            let label = caps[1].trim().to_string();
            let id = label_to_id(&label);
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase {
                    id,
                    label,
                    explicit_id: false,
                    stereotype: None,
                    description: Vec::new(),
                    separators: Vec::new(),
                    color: None,
                    source_line: current_line,
                });
            }
        } else if let Some(caps) = RE_CONN.captures(trimmed) {
            let from = normalize_endpoint(&caps[1]);
            let arrow = caps[2].to_string();
            let to = normalize_endpoint(&caps[3]);
            let raw_label = caps.get(4).map(|m| m.as_str().trim().to_string());
            let stereotype = raw_label.as_ref().and_then(|l| extract_stereotype_text(l));
            // Normalize <<foo>> → «foo» in the label.
            let label = raw_label.map(|l| normalize_stereotype(&l));

            // Auto-register any (paren) use cases referenced in connections.
            for ep in [&caps[1], &caps[3]] {
                let ep = ep.trim();
                if ep.starts_with('(') && ep.ends_with(')') {
                    let inner = ep[1..ep.len() - 1].trim().to_string();
                    let id = label_to_id(&inner);
                    if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                        use_cases.push(UseCase {
                            id,
                            label: inner,
                            explicit_id: false,
                            stereotype: None,
                            description: Vec::new(),
                            separators: Vec::new(),
                            color: None,
                            source_line: current_line,
                        });
                    }
                }
            }
            // Auto-register quoted actor endpoints that haven't been seen yet.
            for ep in [&caps[1], &caps[3]] {
                let ep = ep.trim();
                if ep.starts_with('"') && ep.ends_with('"') {
                    let inner = ep[1..ep.len() - 1].to_string();
                    let id = label_to_id(&inner);
                    if !actors.iter().any(|a: &Actor| a.id == id)
                        && !use_cases.iter().any(|u: &UseCase| u.id == id)
                    {
                        actors.push(Actor {
                            id,
                            label: inner,
                            stereotype: None,
                            color: None,
                            source_line: current_line,
                        });
                    }
                }
            }

            connections.push(UseCaseConnection {
                from,
                to,
                label,
                stereotype,
                queue_len: arrow
                    .bytes()
                    .filter(|byte| matches!(byte, b'-' | b'.'))
                    .count(),
                dashed: arrow.contains('.'),
                arrow: arrow.contains('>'),
                extension: arrow.contains("|>") || arrow.contains("<|"),
                arrow_at_start: arrow.contains('<'),
                source_line: current_line,
            });
        } else if let Some(caps) = RE_PKG.captures(trimmed) {
            let kind = match caps.get(1).map(|m| m.as_str()) {
                Some("rectangle") => crate::diagram::usecase::PackageKind::Rectangle,
                _ => crate::diagram::usecase::PackageKind::Package,
            };
            let name = caps
                .get(2)
                .or(caps.get(3))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let color = caps.get(4).map(|m| m.as_str().to_string());
            current_package = Some(packages.len());
            packages.push(UseCasePackage {
                name,
                elements: Vec::new(),
                color,
                kind,
                source_line: current_line,
            });
        }
    }

    Ok(UseCaseDiagram {
        meta,
        direction,
        actors,
        use_cases,
        connections,
        packages,
        notes,
    })
}

/// Normalize a connection endpoint to an ID:
/// - `(Label)` → stripped label as id
/// - `"Label"` → stripped label as id
/// - `\w+` → as-is
fn normalize_endpoint(ep: &str) -> String {
    let ep = ep.trim();
    if ep.starts_with('(') && ep.ends_with(')') {
        label_to_id(ep[1..ep.len() - 1].trim())
    } else if ep.starts_with('"') && ep.ends_with('"') {
        label_to_id(&ep[1..ep.len() - 1])
    } else {
        ep.to_string()
    }
}

#[derive(Clone)]
struct PendingNote {
    kind: UseCaseNoteKind,
    position: UseCaseNotePosition,
    source_line: usize,
}

fn note_position(position: &str) -> UseCaseNotePosition {
    match position {
        "right" => UseCaseNotePosition::Right,
        "left" => UseCaseNotePosition::Left,
        "top" => UseCaseNotePosition::Top,
        _ => UseCaseNotePosition::Bottom,
    }
}

fn last_entity_id(actors: &[Actor], use_cases: &[UseCase]) -> Option<String> {
    actors
        .iter()
        .map(|actor| (actor.source_line, actor.id.as_str()))
        .chain(
            use_cases
                .iter()
                .map(|use_case| (use_case.source_line, use_case.id.as_str())),
        )
        .max_by_key(|(line, _)| *line)
        .map(|(_, id)| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> UseCaseDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_usecase(&lines).unwrap()
    }

    #[test]
    fn basic_usecase() {
        let d = parse(
            "actor User\nusecase \"Login\" as UC1\nusecase \"Browse\" as UC2\nUser --> UC1\nUser --> UC2",
        );
        assert_eq!(d.actors.len(), 1);
        assert_eq!(d.use_cases.len(), 2);
        assert_eq!(d.connections.len(), 2);
    }

    #[test]
    fn with_stereotype() {
        let d = parse(
            "actor User\nusecase \"Login\" as UC1\nusecase \"Auth\" as UC2\nUC1 ..> UC2 : <<include>>",
        );
        assert_eq!(d.connections[0].stereotype.as_deref(), Some("include"));
        assert!(d.connections[0].dashed);
        assert!(d.connections[0].arrow);
        // Label should be normalized to guillemets.
        assert_eq!(d.connections[0].label.as_deref(), Some("«include»"));
    }

    #[test]
    fn inheritance_triangles_preserve_their_endpoint() {
        let d = parse(
            "actor Parent\nactor FirstChild\nactor SecondChild\nFirstChild --|> Parent\nParent <|-- SecondChild",
        );
        assert!(d.connections[0].extension);
        assert!(d.connections[0].arrow);
        assert!(!d.connections[0].arrow_at_start);
        assert!(d.connections[1].extension);
        assert!(!d.connections[1].arrow);
        assert!(d.connections[1].arrow_at_start);
    }

    #[test]
    fn relation_queue_length_preserves_layout_direction() {
        let d = parse(
            "actor RenamedQueueSource\n\
             usecase RenamedHorizontalTarget\n\
             usecase RenamedVerticalTarget\n\
             RenamedQueueSource -> RenamedHorizontalTarget\n\
             RenamedQueueSource ---> RenamedVerticalTarget",
        );
        assert_eq!(d.connections[0].queue_len, 1);
        assert_eq!(d.connections[1].queue_len, 3);
    }

    #[test]
    fn direction_directive_is_recorded() {
        let d = parse("left to right direction\nactor User\nusecase UC1\nUser --> UC1");
        assert_eq!(d.direction, UseCaseLayoutDirection::LeftToRight);
    }

    #[test]
    fn with_package() {
        let d = parse("actor User\nrectangle System {\nusecase \"Login\" as UC1\n}\nUser --> UC1");
        assert_eq!(d.packages.len(), 1);
        assert_eq!(d.packages[0].name, "System");
    }

    #[test]
    fn quoted_actor_with_alias() {
        let d = parse("actor \"Regular User\" as RU\nusecase \"Login\" as L\nRU --> L");
        assert_eq!(d.actors.len(), 1);
        assert_eq!(d.actors[0].label, "Regular User");
        assert_eq!(d.actors[0].id, "RU");
    }

    #[test]
    fn actor_with_stereotype() {
        let d = parse("actor \"System 1\" <<system>>\nusecase UC1\n\"System 1\" --> UC1");
        assert_eq!(d.actors.len(), 1);
        assert_eq!(d.actors[0].stereotype.as_deref(), Some("system"));
    }

    #[test]
    fn actor_with_spaced_stereotype() {
        let d = parse("actor User << Human >>\nusecase UC1\nUser --> UC1");
        assert_eq!(d.actors[0].stereotype.as_deref(), Some("Human"));
    }

    #[test]
    fn colon_actor_syntax() {
        let d = parse(":User:\n(Login)\nUser --> (Login)");
        assert_eq!(d.actors.len(), 1);
        assert_eq!(d.actors[0].label, "User");
        assert!(!d.use_cases.is_empty());
    }

    #[test]
    fn paren_usecase_in_connection() {
        let d = parse("actor User\nusecase \"Action\"\nUser .. (Action)");
        assert_eq!(d.actors.len(), 1);
        assert_eq!(d.use_cases.len(), 1);
        assert_eq!(d.connections.len(), 1);
    }

    #[test]
    fn bare_usecase_without_alias() {
        let d = parse("actor User\nusecase UC1\nUser --> UC1");
        assert_eq!(d.use_cases.len(), 1);
        assert_eq!(d.use_cases[0].label, "UC1");
    }

    #[test]
    fn usecase_quoted_no_alias() {
        let d = parse("actor User\nusecase \"Action\"\nUser --> Action");
        assert_eq!(d.use_cases.len(), 1);
        assert_eq!(d.use_cases[0].label, "Action");
        assert!(!d.use_cases[0].explicit_id);
    }

    #[test]
    fn usecase_reversed_alias() {
        let d = parse("actor User\nusecase BaseUC as \"Base Use Case\"\nUser --> BaseUC");
        assert_eq!(d.use_cases.len(), 1);
        assert_eq!(d.use_cases[0].id, "BaseUC");
        assert_eq!(d.use_cases[0].label, "Base Use Case");
        assert!(d.use_cases[0].explicit_id);
    }

    #[test]
    fn usecase_with_stereotype() {
        let d = parse("actor User\nusecase UC1 <<automated>>\nUser --> UC1");
        assert_eq!(d.use_cases.len(), 1);
        assert_eq!(d.use_cases[0].stereotype.as_deref(), Some("automated"));
    }

    #[test]
    fn actor_with_color_modifier() {
        let d = parse("actor User #LightBlue\nusecase UC1\nUser --> UC1");
        assert_eq!(d.actors.len(), 1);
        assert_eq!(d.actors[0].id, "User");
    }

    #[test]
    fn multiline_usecase_description() {
        let src = "usecase UC1 as \"\n  Title\n  --\n  Description text here\n  Multiple lines allowed\n\"\nactor User\nUser --> UC1";
        let d = parse(src);
        assert_eq!(d.use_cases.len(), 1);
        assert_eq!(d.use_cases[0].id, "UC1");
        assert_eq!(d.use_cases[0].label, "Title");
        assert!(d.use_cases[0].explicit_id);
        assert!(
            d.use_cases[0]
                .description
                .contains(&"Description text here".to_string())
        );
        assert_eq!(
            d.use_cases[0].separators,
            vec![UseCaseSeparator {
                before_line: 1,
                style: UseCaseSeparatorStyle::Solid,
            }]
        );
    }

    #[test]
    fn multiline_usecase_preserves_compartment_order_and_styles() {
        let src = "usecase FreshFlow as \"\n\
                   Summary\n\
                   ....\n\
                   First detail\n\
                   ====\n\
                   Second detail\n\
                   ____\n\
                   Final detail\n\
                   \"";
        let d = parse(src);
        assert_eq!(
            d.use_cases[0].description,
            ["Summary", "First detail", "Second detail", "Final detail"]
        );
        assert_eq!(
            d.use_cases[0].separators,
            [
                UseCaseSeparator {
                    before_line: 1,
                    style: UseCaseSeparatorStyle::Dotted,
                },
                UseCaseSeparator {
                    before_line: 2,
                    style: UseCaseSeparatorStyle::Double,
                },
                UseCaseSeparator {
                    before_line: 3,
                    style: UseCaseSeparatorStyle::Solid,
                },
            ]
        );
    }

    #[test]
    fn multiline_usecase_keeps_three_dot_ellipsis_as_body_text() {
        let src = "usecase FreshFlow as \"\n\
                   Summary\n\
                   ...\n\
                   Details\n\
                   \"";
        let d = parse(src);
        assert_eq!(d.use_cases[0].description, ["Summary", "...", "Details"]);
        assert!(d.use_cases[0].separators.is_empty());
    }

    #[test]
    fn renamed_notes_preserve_svek_ownership_and_source_lines() {
        let d = parse(
            "actor \"Night Auditor\" as Auditor\n\
             usecase \"Reconcile Archived Statements\" as Reconcile\n\
             note left of Auditor : Reviews the overnight queue\n\
             note bottom of Reconcile\n\
               First pass\n\
               Second pass\n\
             end note\n\
             note \"Detached checklist\" as Checklist\n\
             Auditor --> Reconcile\n\
             note top on link : Escalates discrepancies",
        );

        assert_eq!(d.notes.len(), 4);
        assert_eq!(
            d.notes[0].kind,
            UseCaseNoteKind::Attached {
                target: "Auditor".to_string()
            }
        );
        assert_eq!(d.notes[0].position, UseCaseNotePosition::Left);
        assert_eq!(d.notes[0].source_line, 3);
        assert_eq!(d.notes[1].text, "First pass\nSecond pass");
        assert_eq!(
            d.notes[1].kind,
            UseCaseNoteKind::Attached {
                target: "Reconcile".to_string()
            }
        );
        assert_eq!(d.notes[1].position, UseCaseNotePosition::Bottom);
        assert_eq!(d.notes[1].source_line, 5);
        assert_eq!(
            d.notes[2].kind,
            UseCaseNoteKind::Floating {
                id: "Checklist".to_string()
            }
        );
        assert_eq!(d.notes[3].kind, UseCaseNoteKind::OnLink { connection: 0 });
        assert_eq!(d.notes[3].position, UseCaseNotePosition::Top);
        assert_eq!(d.notes[3].source_line, 10);
    }

    #[test]
    fn multiline_note_removes_only_shared_indent() {
        let d = parse(concat!(
            "actor \"Fresh Examiner 5101\" as Examiner5101\n",
            "note bottom of Examiner5101\n",
            "   Fresh heading\n",
            "     * deeper one\n",
            "       ** deeper two\n",
            "end note",
        ));

        // Java `CommandFactoryNoteOnEntity.createMultiLine` passes the body
        // through `BlocLines.removeEmptyColumns`, preserving relative indents.
        assert_eq!(
            d.notes[0].text,
            "Fresh heading\n  * deeper one\n    ** deeper two"
        );
    }

    #[test]
    fn targetless_note_attaches_to_last_renamed_entity() {
        let d = parse(
            "actor \"Queue Steward\" as Steward\n\
             usecase \"Archive Reviewed Batch\" as Archive\n\
             note right : This follows the last entity",
        );

        assert_eq!(
            d.notes[0].kind,
            UseCaseNoteKind::Attached {
                target: "Archive".to_string()
            }
        );
    }

    #[test]
    fn document_chrome_preserves_directive_source_lines() {
        let d = parse(
            "header Renamed Header\n\
             title Renamed Title\n\
             footer Renamed Footer\n\
             actor Reviewer",
        );

        assert_eq!(d.meta.header_line, Some(1));
        assert_eq!(d.meta.title_line, Some(2));
        assert_eq!(d.meta.footer_line, Some(3));
    }
}
