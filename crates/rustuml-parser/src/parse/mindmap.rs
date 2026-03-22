// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Mind map parser — turns preprocessed @startmindmap lines into a
//! `MindMapDiagram`.
//!
//! Syntax:
//! ```text
//! @startmindmap
//! * Root
//! ** Branch A
//! *** Leaf 1
//! ** Branch B
//! @endmindmap
//! ```
//!
//! Each line starts with one or more `*` characters; the count is the depth.
//! Depth 1 is the root.  Indentation with spaces or tabs (as in the
//! "outline" variant) is also supported via `+` or `-` bullet prefixes, but
//! the canonical form uses `*`.

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::mindmap::{MindMapDiagram, MindMapNode};

/// Parse preprocessed lines from a `@startmindmap` block.
///
/// The caller passes the *preprocessed* lines (i.e., after stripping the
/// `@startmindmap`/`@endmindmap` delimiters and expanding any TIM
/// directives).  Lines that are empty after trimming are skipped.
pub fn parse_mindmap(lines: &[String]) -> Result<MindMapDiagram, ParseError> {
    let meta = DiagramMeta::default();
    let mut roots: Vec<MindMapNode> = Vec::new();

    // `stack` always contains a path from the conceptual "root container"
    // (depth 0, never emitted) down to the most recently inserted node.
    // Each entry is `(depth, index_in_parent_children)`.
    // We rebuild the insertion point by walking from `roots`.
    //
    // Because we need mutable access to the tree we keep a simple "parent
    // stack" of depths and re-navigate on each insertion, which is O(depth)
    // per node — acceptable for typical mind-map sizes.

    let mut depth_stack: Vec<usize> = Vec::new(); // depths of ancestors

    for (line_no, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Count leading `*` characters.
        let stars = trimmed.chars().take_while(|&c| c == '*').count();
        if stars == 0 {
            // Not a node line — skip (could be a skinparam, comment, etc.).
            continue;
        }

        let label = trimmed[stars..].trim().to_string();
        if label.is_empty() {
            return Err(ParseError {
                line: line_no + 1,
                message: "mind map node has no label".to_string(),
            });
        }

        let depth = stars;
        let node = MindMapNode {
            label,
            depth,
            children: Vec::new(),
        };

        if depth == 1 {
            // Root-level node.
            roots.push(node);
            depth_stack.clear();
            depth_stack.push(1);
        } else {
            // Find the deepest ancestor whose depth is < current depth.
            // Pop the stack until we find it.
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

            // Navigate to the correct parent in the tree.
            let parent = find_deepest_at(&mut roots, &depth_stack).ok_or_else(|| ParseError {
                line: line_no + 1,
                message: "internal error: could not locate parent node".to_string(),
            })?;
            parent.children.push(node);
            depth_stack.push(depth);
        }
    }

    Ok(MindMapDiagram { meta, roots })
}

/// Navigate to the parent node for the next insertion.
///
/// `depth_stack` holds the depths of all ancestor nodes in the current path,
/// starting from the root (depth 1).  To insert a new child, we need the
/// node at the *tip* of this stack (i.e., the most recently inserted
/// ancestor).
///
/// We start at `roots.last_mut()` (the most recent depth-1 node) and then
/// descend one level for each entry in `depth_stack[1..]`.
fn find_deepest_at<'a>(
    roots: &'a mut [MindMapNode],
    depth_stack: &[usize],
) -> Option<&'a mut MindMapNode> {
    // depth_stack always has at least one entry (depth 1 = root level).
    let steps_below_root = depth_stack.len().saturating_sub(1);
    let mut node = roots.last_mut()?;
    for _ in 0..steps_below_root {
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

    fn parse(s: &str) -> MindMapDiagram {
        parse_mindmap(&lines(s)).expect("parse failed")
    }

    #[test]
    fn single_root() {
        let d = parse("* Root");
        assert_eq!(d.roots.len(), 1);
        assert_eq!(d.roots[0].label, "Root");
        assert_eq!(d.roots[0].depth, 1);
        assert!(d.roots[0].children.is_empty());
    }

    #[test]
    fn root_with_branches() {
        let d = parse("* Root\n** A\n** B");
        assert_eq!(d.roots.len(), 1);
        assert_eq!(d.roots[0].children.len(), 2);
        assert_eq!(d.roots[0].children[0].label, "A");
        assert_eq!(d.roots[0].children[1].label, "B");
    }

    #[test]
    fn three_levels() {
        let d = parse("* Root\n** Branch\n*** Leaf");
        let branch = &d.roots[0].children[0];
        assert_eq!(branch.label, "Branch");
        assert_eq!(branch.children.len(), 1);
        assert_eq!(branch.children[0].label, "Leaf");
    }

    #[test]
    fn full_example() {
        let d = parse("* Root\n** Branch A\n*** Leaf 1\n*** Leaf 2\n** Branch B\n*** Leaf 3");
        assert_eq!(d.roots[0].children.len(), 2);
        assert_eq!(d.roots[0].children[0].children.len(), 2);
        assert_eq!(d.roots[0].children[1].children.len(), 1);
    }

    #[test]
    fn skips_empty_lines() {
        let d = parse("* Root\n\n** A\n\n** B");
        assert_eq!(d.roots[0].children.len(), 2);
    }

    #[test]
    fn multiple_roots() {
        let d = parse("* Root1\n** A\n* Root2\n** B");
        assert_eq!(d.roots.len(), 2);
        assert_eq!(d.roots[0].label, "Root1");
        assert_eq!(d.roots[1].label, "Root2");
    }

    #[test]
    fn depth_jump_error() {
        let err = parse_mindmap(&lines("* Root\n*** TooDeep")).unwrap_err();
        assert!(err.message.contains("depth jump"), "{}", err.message);
    }

    #[test]
    fn missing_label_error() {
        let err = parse_mindmap(&lines("* Root\n**")).unwrap_err();
        assert!(err.message.contains("no label"), "{}", err.message);
    }
}
