// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Git diagram parser.
//!
//! Recognised syntax (subset of PlantUML gitlog):
//!
//! ```text
//! commit [id:"..."] [tag:"..."] [type:REVERSE|HIGHLIGHT]
//! branch <name>
//! checkout <name>
//! merge <branch> [tag:"..."]
//! cherry-pick id:"<id>"
//! ```

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::git_diagram::{CommitType, GitCommand, GitDiagram};

/// Parse pre-processed lines into a [`GitDiagram`].
pub fn parse_git(lines: &[String]) -> Result<GitDiagram, ParseError> {
    let mut meta = DiagramMeta::default();
    let mut commands = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('\'') {
            continue;
        }

        // title
        if let Some(rest) = trimmed.strip_prefix("title ") {
            meta.title = Some(super::strip_title_quotes(rest).to_string());
            continue;
        }
        // header / footer
        if let Some(rest) = trimmed.strip_prefix("header ") {
            meta.header = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("footer ") {
            meta.footer = Some(rest.trim().to_string());
            continue;
        }

        if let Some(cmd) = parse_command(trimmed, i + 1)? {
            commands.push(cmd);
        }
    }

    Ok(GitDiagram { meta, commands })
}

/// Try to parse a single line into a GitCommand.
fn parse_command(line: &str, _line_num: usize) -> Result<Option<GitCommand>, ParseError> {
    // commit [id:"..."] [tag:"..."] [type:REVERSE|HIGHLIGHT]
    if line == "commit" || line.starts_with("commit ") {
        let rest = line.strip_prefix("commit").unwrap().trim();
        let id = extract_quoted_param(rest, "id:");
        let tag = extract_quoted_param(rest, "tag:");
        let commit_type = if rest.contains("type:REVERSE") {
            CommitType::Reverse
        } else if rest.contains("type:HIGHLIGHT") {
            CommitType::Highlight
        } else {
            CommitType::Normal
        };
        return Ok(Some(GitCommand::Commit {
            id,
            tag,
            commit_type,
        }));
    }

    // branch <name>
    if let Some(rest) = line.strip_prefix("branch ") {
        let name = rest.trim().to_string();
        if !name.is_empty() {
            return Ok(Some(GitCommand::Branch(name)));
        }
    }

    // checkout <name>
    if let Some(rest) = line.strip_prefix("checkout ") {
        let name = rest.trim().to_string();
        if !name.is_empty() {
            return Ok(Some(GitCommand::Checkout(name)));
        }
    }

    // merge <branch> [tag:"..."]
    if let Some(rest) = line.strip_prefix("merge ") {
        let rest = rest.trim();
        let tag = extract_quoted_param(rest, "tag:");
        // Branch name is the first token.
        let branch = rest.split_whitespace().next().unwrap_or("").to_string();
        if !branch.is_empty() {
            return Ok(Some(GitCommand::Merge { branch, tag }));
        }
    }

    // cherry-pick id:"<id>"
    if let Some(rest) = line.strip_prefix("cherry-pick ") {
        let rest = rest.trim();
        if let Some(id) = extract_quoted_param(rest, "id:") {
            return Ok(Some(GitCommand::CherryPick { id }));
        }
    }

    // Silently ignore unknown directives.
    Ok(None)
}

/// Extract a quoted parameter value, e.g. `id:"foo"` -> Some("foo").
fn extract_quoted_param(text: &str, prefix: &str) -> Option<String> {
    let idx = text.find(prefix)?;
    let after = &text[idx + prefix.len()..];
    if after.starts_with('"') {
        let end = after[1..].find('"')?;
        Some(after[1..1 + end].to_string())
    } else {
        // Unquoted: take until whitespace.
        let end = after.find(char::is_whitespace).unwrap_or(after.len());
        Some(after[..end].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> GitDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_git(&lines).unwrap()
    }

    #[test]
    fn basic_commit() {
        let d = parse("commit");
        assert_eq!(d.commands.len(), 1);
        assert!(matches!(
            &d.commands[0],
            GitCommand::Commit {
                id: None,
                tag: None,
                ..
            }
        ));
    }

    #[test]
    fn commit_with_id_and_tag() {
        let d = parse(r#"commit id:"abc" tag:"v1.0""#);
        assert!(matches!(
            &d.commands[0],
            GitCommand::Commit {
                id: Some(id),
                tag: Some(tag),
                ..
            } if id == "abc" && tag == "v1.0"
        ));
    }

    #[test]
    fn branch_and_checkout() {
        let d = parse("branch develop\ncheckout develop");
        assert_eq!(d.commands.len(), 2);
        assert!(matches!(&d.commands[0], GitCommand::Branch(name) if name == "develop"));
        assert!(matches!(&d.commands[1], GitCommand::Checkout(name) if name == "develop"));
    }

    #[test]
    fn merge_with_tag() {
        let d = parse(r#"merge develop tag:"v1.0""#);
        assert!(matches!(
            &d.commands[0],
            GitCommand::Merge { branch, tag: Some(tag) } if branch == "develop" && tag == "v1.0"
        ));
    }

    #[test]
    fn cherry_pick() {
        let d = parse(r#"cherry-pick id:"abc""#);
        assert!(matches!(
            &d.commands[0],
            GitCommand::CherryPick { id } if id == "abc"
        ));
    }
}
