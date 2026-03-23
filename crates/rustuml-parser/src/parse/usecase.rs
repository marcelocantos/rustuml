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
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<<\s*([^>]+?)\s*>>").unwrap());
    RE.replace_all(s, |caps: &regex::Captures| {
        format!("«{}»", caps[1].trim())
    })
    .into_owned()
}

/// Extract a stereotype string from a `<<foo>>` or `<< foo >>` token.
fn extract_stereotype_text(s: &str) -> Option<String> {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<<\s*([^>]+?)\s*>>").unwrap());
    RE.captures(s).map(|c| c[1].trim().to_string())
}

/// Turn a label into a simple identifier (strip spaces, keep alphanumerics/underscores).
fn label_to_id(label: &str) -> String {
    label.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect()
}

pub fn parse_usecase(lines: &[String]) -> Result<UseCaseDiagram, ParseError> {
    let mut actors: Vec<Actor> = Vec::new();
    let mut use_cases: Vec<UseCase> = Vec::new();
    let mut connections = Vec::new();
    let mut packages: Vec<UseCasePackage> = Vec::new();
    let mut notes: Vec<UseCaseNote> = Vec::new();
    let mut meta = DiagramMeta::default();
    let mut current_package: Option<usize> = None;
    // For multiline string literals in usecase declarations.
    let mut multiline_uc_id: Option<String> = None;
    let mut multiline_label_lines: Vec<String> = Vec::new();
    // For multiline note blocks.
    let mut in_note_block = false;
    let mut note_block_lines: Vec<String> = Vec::new();

    // Regex patterns compiled once.

    // actor "Label" as ID <<stereotype>>  (all optional parts, color modifier ignored)
    static RE_ACTOR_QUOTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^actor\s+"([^"]+)"(?:\s+as\s+(\w+))?(?:\s+(<<\s*[^>]+\s*>>))?(?:\s+#\w+)?"#).unwrap()
    });
    // actor Word <<stereotype>>  (bare single-word name, optional color modifier)
    static RE_ACTOR_BARE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^actor\s+(\w+)(?:\s+(<<\s*[^>]+\s*>>))?(?:\s+#\w+)?"#).unwrap()
    });
    // :Label: shorthand actor syntax
    static RE_ACTOR_COLON: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^:([^:]+):\s*$").unwrap());

    // usecase "Label" as ID <<stereotype>>
    static RE_UC_QUOTED_AS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^usecase\s+"([^"]+)"\s+as\s+(\w+)(?:\s+(<<\s*[^>]+\s*>>))?"#).unwrap());
    // usecase "Label" <<stereotype>>  (no alias)
    static RE_UC_QUOTED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^usecase\s+"([^"]+)"(?:\s+(<<\s*[^>]+\s*>>))?(?:\s+#\w+)?"#).unwrap());
    // usecase ID as "Label" <<stereotype>>  (reversed alias, closing quote required)
    static RE_UC_ID_AS_LABEL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r##"^usecase\s+(\w+)(?:\s+#\w+)?\s+as\s+"([^"]+)""##).unwrap());
    // usecase (Label) as ID  (paren-based use case with alias)
    static RE_UC_PAREN_AS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^usecase\s+\(([^)]+)\)\s+as\s+(\w+)").unwrap());
    // usecase ID [#color] as " (multiline label start — opening quote not closed on same line)
    static RE_UC_ID_AS_MULTI: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^usecase\s+(\w+)(?:\s+#\w+)?\s+as\s+"\s*$"#).unwrap());
    // usecase ID <<stereotype>>  (bare word, with optional stereotype/color)
    static RE_UC_BARE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"^usecase\s+(\w+)(?:\s+(<<\s*[^>]+\s*>>))?(?:\s+#\w+)?"#).unwrap());
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

    // Inline note: note <position> of <target> : <text>
    static RE_NOTE_INLINE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^note\s+\w+\s+of\s+(?:"[^"]+"|[(][^)]+[)]|\w+)\s*:\s*(.+)$"#).unwrap()
    });
    // Note on link: note on link : text
    static RE_NOTE_ON_LINK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^note on link\s*:\s*(.+)$"#).unwrap()
    });
    // Block note start: note <position> of <target> (no colon)
    static RE_NOTE_BLOCK_START: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^note\s+(?:\w+\s+of\s+)?(?:"[^"]+"|[(][^)]+[)]|\w+)?\s*$"#).unwrap()
    });
    // Floating note start: note "text"
    static RE_NOTE_FLOAT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^note\s+"([^"]+)""#).unwrap()
    });

    // Package/rectangle opening (with optional color/style modifiers)
    static RE_PKG: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^(?:rectangle|package)\s+(?:"([^"]+)"|(\w+))(?:\s+[^{]*)?\{"#).unwrap()
    });

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Handle multiline string literal for usecase label.
        if let Some(ref uc_id) = multiline_uc_id.clone() {
            if trimmed.contains('"') {
                // End of multiline literal.
                // First non-separator, non-empty line is the title/label.
                let is_sep = |s: &str| {
                    let t = s.trim();
                    t == "--" || t == "==" || t == ".." || t.chars().all(|c| c == '-') || t.chars().all(|c| c == '=') || t.chars().all(|c| c == '.')
                };
                let label = multiline_label_lines
                    .iter()
                    .find(|l| !l.trim().is_empty() && !is_sep(l))
                    .map(|l| l.trim().to_string())
                    .unwrap_or_else(|| uc_id.clone());
                // All non-separator content lines form the description.
                let description: Vec<String> = multiline_label_lines
                    .iter()
                    .filter(|l| !l.trim().is_empty() && !is_sep(l))
                    .map(|l| l.trim().to_string())
                    .collect();
                let id = uc_id.clone();
                if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                    use_cases.push(UseCase { id, label, stereotype: None, description });
                }
                multiline_uc_id = None;
                multiline_label_lines.clear();
            } else {
                multiline_label_lines.push(trimmed.to_string());
            }
            continue;
        }

        if trimmed == "}" {
            current_package = None;
            continue;
        }

        // Handle multiline note block body.
        if in_note_block {
            if trimmed == "end note" {
                let text = note_block_lines.join("\n");
                if !text.trim().is_empty() {
                    notes.push(UseCaseNote { text: text.trim().to_string(), target: None });
                }
                in_note_block = false;
                note_block_lines.clear();
            } else {
                note_block_lines.push(trimmed.to_string());
            }
            continue;
        }

        // Skip top-level directives.
        if trimmed.starts_with("left to right")
            || trimmed.starts_with("top to bottom")
            || trimmed.starts_with("skinparam")
            || trimmed.starts_with("end note")
            || trimmed.starts_with("hide")
            || trimmed.starts_with("show")
            || trimmed.starts_with("!") {
            continue;
        }

        // title directive.
        if let Some(t) = trimmed.strip_prefix("title") {
            let title = t.trim().to_string();
            if !title.is_empty() {
                meta.title = Some(title);
            }
            continue;
        }

        // header directive.
        if let Some(h) = trimmed.strip_prefix("header") {
            let header = h.trim().to_string();
            if !header.is_empty() {
                meta.header = Some(header);
            }
            continue;
        }

        // footer directive.
        if let Some(f) = trimmed.strip_prefix("footer") {
            let footer = f.trim().to_string();
            if !footer.is_empty() {
                meta.footer = Some(footer);
            }
            continue;
        }

        // Inline note: note X of Y : text
        if let Some(caps) = RE_NOTE_INLINE.captures(trimmed) {
            let note_text = caps[1].trim().to_string();
            notes.push(UseCaseNote { text: note_text, target: None });
            continue;
        }

        // Note on link: note on link : text
        if let Some(caps) = RE_NOTE_ON_LINK.captures(trimmed) {
            let note_text = caps[1].trim().to_string();
            notes.push(UseCaseNote { text: note_text, target: None });
            continue;
        }

        // Floating note: note "text"
        if let Some(caps) = RE_NOTE_FLOAT.captures(trimmed) {
            let note_text = caps[1].trim().to_string();
            notes.push(UseCaseNote { text: note_text, target: None });
            continue;
        }

        // Block note start: note X of Y (no colon).
        if RE_NOTE_BLOCK_START.is_match(trimmed) && trimmed.starts_with("note") {
            in_note_block = true;
            note_block_lines.clear();
            continue;
        }

        if let Some(caps) = RE_ACTOR_QUOTED.captures(trimmed) {
            let label = caps[1].to_string();
            let id = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_else(|| label_to_id(&label));
            let stereotype = caps.get(3).and_then(|m| extract_stereotype_text(m.as_str()));
            if !actors.iter().any(|a: &Actor| a.id == id) {
                actors.push(Actor { id, label, stereotype });
            }
        } else if let Some(caps) = RE_ACTOR_COLON.captures(trimmed) {
            let label = caps[1].trim().to_string();
            let id = label_to_id(&label);
            if !actors.iter().any(|a: &Actor| a.id == id) {
                actors.push(Actor { id, label, stereotype: None });
            }
        } else if let Some(caps) = RE_ACTOR_BARE.captures(trimmed) {
            let label = caps[1].to_string();
            let id = label.clone();
            let stereotype = caps.get(2).and_then(|m| extract_stereotype_text(m.as_str()));
            if !actors.iter().any(|a: &Actor| a.id == id) {
                actors.push(Actor { id, label, stereotype });
            }
        } else if let Some(caps) = RE_UC_ID_AS_LABEL.captures(trimmed) {
            // usecase ID as "Label" (single-line quoted label)
            let id = caps[1].to_string();
            let label = caps[2].to_string();
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase { id, label, stereotype: None, description: Vec::new() });
            }
        } else if RE_UC_ID_AS_MULTI.is_match(trimmed) {
            // Multiline label: usecase ID as "...
            let caps = RE_UC_ID_AS_MULTI.captures(trimmed).unwrap();
            let id = caps[1].to_string();
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            multiline_uc_id = Some(id);
            multiline_label_lines.clear();
        } else if let Some(caps) = RE_UC_PAREN_AS.captures(trimmed) {
            // usecase (Label) as ID
            let label = caps[1].trim().to_string();
            let id = caps[2].to_string();
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase { id, label, stereotype: None, description: Vec::new() });
            }
        } else if let Some(caps) = RE_UC_QUOTED_AS.captures(trimmed) {
            let label = caps[1].to_string();
            let id = caps[2].to_string();
            let stereotype = caps.get(3).and_then(|m| extract_stereotype_text(m.as_str()));
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase { id, label, stereotype, description: Vec::new() });
            }
        } else if let Some(caps) = RE_UC_QUOTED.captures(trimmed) {
            let label = caps[1].to_string();
            let id = label_to_id(&label);
            let stereotype = caps.get(2).and_then(|m| extract_stereotype_text(m.as_str()));
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase { id, label, stereotype, description: Vec::new() });
            }
        } else if let Some(caps) = RE_UC_BARE.captures(trimmed) {
            let id = caps[1].to_string();
            let label = id.clone();
            let stereotype = caps.get(2).and_then(|m| extract_stereotype_text(m.as_str()));
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase { id, label, stereotype, description: Vec::new() });
            }
        } else if let Some(caps) = RE_UC_PAREN.captures(trimmed) {
            let label = caps[1].trim().to_string();
            let id = label_to_id(&label);
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                use_cases.push(UseCase { id, label, stereotype: None, description: Vec::new() });
            }
        } else if let Some(caps) = RE_CONN.captures(trimmed) {
            let from = normalize_endpoint(&caps[1]);
            let to = normalize_endpoint(&caps[3]);
            let raw_label = caps.get(4).map(|m| m.as_str().trim().to_string());
            let stereotype = raw_label
                .as_ref()
                .and_then(|l| extract_stereotype_text(l));
            // Normalize <<foo>> → «foo» in the label.
            let label = raw_label.map(|l| normalize_stereotype(&l));

            // Auto-register any (paren) use cases referenced in connections.
            for ep in [&caps[1], &caps[3]] {
                let ep = ep.trim();
                if ep.starts_with('(') && ep.ends_with(')') {
                    let inner = ep[1..ep.len() - 1].trim().to_string();
                    let id = label_to_id(&inner);
                    if !use_cases.iter().any(|u: &UseCase| u.id == id) {
                        use_cases.push(UseCase { id, label: inner, stereotype: None, description: Vec::new() });
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
                        actors.push(Actor { id, label: inner, stereotype: None });
                    }
                }
            }

            connections.push(UseCaseConnection { from, to, label, stereotype });
        } else if let Some(caps) = RE_PKG.captures(trimmed) {
            let name = caps
                .get(1)
                .or(caps.get(2))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            current_package = Some(packages.len());
            packages.push(UseCasePackage { name, elements: Vec::new() });
        }
    }

    Ok(UseCaseDiagram { meta, actors, use_cases, connections, packages, notes })
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
        // Label should be normalized to guillemets.
        assert_eq!(d.connections[0].label.as_deref(), Some("«include»"));
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
    }

    #[test]
    fn usecase_reversed_alias() {
        let d = parse("actor User\nusecase BaseUC as \"Base Use Case\"\nUser --> BaseUC");
        assert_eq!(d.use_cases.len(), 1);
        assert_eq!(d.use_cases[0].id, "BaseUC");
        assert_eq!(d.use_cases[0].label, "Base Use Case");
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
        assert!(d.use_cases[0].description.contains(&"Description text here".to_string()));
    }
}
