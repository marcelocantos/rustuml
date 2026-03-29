// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! WBS (Work Breakdown Structure) diagram parser.
//!
//! Syntax:
//! ```text
//! @startwbs
//! * Project
//! ** Phase 1
//! *** Task A
//! *** Task B
//! ** Phase 2
//! *** Task C
//! @endwbs
//! ```
//!
//! Each line starts with one or more `*` characters; the count is the depth
//! (1 = root).  Lines that are empty after trimming or that don't start with
//! `*` are silently ignored (e.g. `@startwbs`, `@endwbs`, comments).

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::wbs::{WbsDiagram, WbsNode, WbsSide};

/// Parse preprocessed lines from a `@startwbs` block into a [`WbsDiagram`].
pub fn parse_wbs(lines: &[String]) -> Result<WbsDiagram, ParseError> {
    let mut meta = DiagramMeta::default();
    let mut roots: Vec<WbsNode> = Vec::new();

    // `depth_stack` tracks the path of depths from the top-level sentinel
    // (never emitted) down to the most recently inserted node.
    let mut depth_stack: Vec<usize> = Vec::new();

    for (line_no, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Detect node prefix: `*` chars for right-side, `-` chars for left-side.
        let stars = trimmed.chars().take_while(|&c| c == '*').count();
        let dashes = if stars == 0 {
            trimmed.chars().take_while(|&c| c == '-').count()
        } else {
            0
        };

        let (prefix_len, side) = if stars > 0 {
            (stars, WbsSide::Right)
        } else if dashes > 0 {
            (dashes, WbsSide::Left)
        } else {
            // Collect skinparam directives into metadata.
            if let Some(rest) = trimmed.strip_prefix("skinparam ")
                && let Some((key, value)) = rest.split_once(' ')
            {
                meta.skinparams.push(crate::diagram::SkinParam {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
            // Not a node line — skip (could be @startwbs, @endwbs, comment, etc.).
            continue;
        };

        let label = trimmed[prefix_len..].trim().to_string();
        if label.is_empty() {
            return Err(ParseError {
                line: line_no + 1,
                message: "WBS node has no label".to_string(),
            });
        }

        let depth = prefix_len;
        let node = WbsNode {
            label,
            depth,
            side,
            children: Vec::new(),
        };

        if depth == 1 {
            // Root-level node.
            roots.push(node);
            depth_stack.clear();
            depth_stack.push(1);
        } else {
            // Pop the stack until the top is < current depth.
            while depth_stack.len() > 1 && *depth_stack.last().unwrap() >= depth {
                depth_stack.pop();
            }

            let parent_depth = *depth_stack.last().ok_or_else(|| ParseError {
                line: line_no + 1,
                message: format!(
                    "depth-{depth} node has no parent (no preceding depth-{} node)",
                    depth - 1
                ),
            })?;

            if parent_depth != depth - 1 {
                return Err(ParseError {
                    line: line_no + 1,
                    message: format!("unexpected depth jump from {} to {}", parent_depth, depth),
                });
            }

            // Navigate to the parent node and append.
            let parent = find_deepest_at(&mut roots, &depth_stack).ok_or_else(|| ParseError {
                line: line_no + 1,
                message: "internal error: could not locate parent node".to_string(),
            })?;
            parent.children.push(node);
            depth_stack.push(depth);
        }
    }

    Ok(WbsDiagram { meta, nodes: roots })
}

/// Navigate to the deepest node described by `path_depths` (ancestor-first,
/// not including the sentinel entry) and return a mutable reference to the
/// last child at each step.
fn find_deepest_at<'a>(roots: &'a mut [WbsNode], path_depths: &[usize]) -> Option<&'a mut WbsNode> {
    if path_depths.is_empty() {
        return None;
    }
    let mut node = roots.last_mut()?;
    for _ in &path_depths[1..] {
        node = node.children.last_mut()?;
    }
    Some(node)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(|l| l.to_string()).collect()
    }

    fn parse(s: &str) -> WbsDiagram {
        parse_wbs(&lines(s)).expect("parse failed")
    }

    #[test]
    fn single_root() {
        let d = parse("* Root");
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].label, "Root");
        assert_eq!(d.nodes[0].depth, 1);
        assert!(d.nodes[0].children.is_empty());
    }

    #[test]
    fn root_with_children() {
        let d = parse("* Project\n** Phase 1\n** Phase 2");
        assert_eq!(d.nodes.len(), 1);
        let root = &d.nodes[0];
        assert_eq!(root.label, "Project");
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].label, "Phase 1");
        assert_eq!(root.children[1].label, "Phase 2");
    }

    #[test]
    fn three_levels() {
        let d = parse("* Project\n** Phase 1\n*** Task A\n*** Task B\n** Phase 2\n*** Task C");
        let root = &d.nodes[0];
        assert_eq!(root.children.len(), 2);
        let phase1 = &root.children[0];
        assert_eq!(phase1.children.len(), 2);
        assert_eq!(phase1.children[0].label, "Task A");
        assert_eq!(phase1.children[1].label, "Task B");
        let phase2 = &root.children[1];
        assert_eq!(phase2.children.len(), 1);
        assert_eq!(phase2.children[0].label, "Task C");
    }

    #[test]
    fn ignores_non_star_lines() {
        let d = parse("@startwbs\n* Root\n** Child\n@endwbs");
        assert_eq!(d.nodes.len(), 1);
        assert_eq!(d.nodes[0].children.len(), 1);
    }

    #[test]
    fn empty_input_gives_empty_nodes() {
        let d = parse("");
        assert!(d.nodes.is_empty());
    }

    #[test]
    fn depth_field_correct() {
        let d = parse("* Root\n** Child\n*** Grandchild");
        assert_eq!(d.nodes[0].depth, 1);
        assert_eq!(d.nodes[0].children[0].depth, 2);
        assert_eq!(d.nodes[0].children[0].children[0].depth, 3);
    }

    #[test]
    fn skips_empty_lines() {
        let d = parse("* Root\n\n** A\n\n** B");
        assert_eq!(d.nodes[0].children.len(), 2);
    }

    #[test]
    fn multiple_roots() {
        let d = parse("* Root1\n** A\n* Root2\n** B");
        assert_eq!(d.nodes.len(), 2);
        assert_eq!(d.nodes[0].label, "Root1");
        assert_eq!(d.nodes[1].label, "Root2");
    }

    #[test]
    fn depth_jump_error() {
        let err = parse_wbs(&lines("* Root\n*** TooDeep")).unwrap_err();
        assert!(err.message.contains("depth jump"), "{}", err.message);
    }

    #[test]
    fn missing_label_error() {
        let err = parse_wbs(&lines("* Root\n**")).unwrap_err();
        assert!(err.message.contains("no label"), "{}", err.message);
    }
}
