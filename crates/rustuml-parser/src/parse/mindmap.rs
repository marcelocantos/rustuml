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
//! Each line starts with one or more `*` or `-` characters; the count is the
//! depth.  Depth 1 is the root (always `*`).  Right-side branches use `**`,
//! `***`, etc.; left-side branches use `--`, `---`, etc.

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::mindmap::{MindMapDiagram, MindMapNode, Side};

/// Parse preprocessed lines from a `@startmindmap` block.
///
/// The caller passes the *preprocessed* lines (i.e., after stripping the
/// `@startmindmap`/`@endmindmap` delimiters and expanding any TIM
/// directives).  Lines that are empty after trimming are skipped.
pub fn parse_mindmap(lines: &[String]) -> Result<MindMapDiagram, ParseError> {
    let mut meta = DiagramMeta::default();
    let mut roots: Vec<MindMapNode> = Vec::new();

    // `stack` always contains a path from the conceptual "root container"
    // (depth 0, never emitted) down to the most recently inserted node.
    // Each entry is `(depth, index_in_parent_children)`.
    // We rebuild the insertion point by walking from `roots`.
    //
    // Because we need mutable access to the tree we keep a simple "parent
    // stack" of depths and re-navigate on each insertion, which is O(depth)
    // per node — acceptable for typical mind-map sizes.

    // `depth_stack` tracks depths of ancestors for the *current side* being
    // built.  We maintain separate stacks for left and right sides so that
    // a block of left-side lines (`--`, `---`, …) after a block of right-side
    // lines can reference the correct root.
    let mut right_stack: Vec<usize> = Vec::new(); // depths of right-side ancestors
    let mut left_stack: Vec<usize> = Vec::new(); // depths of left-side ancestors

    // Multiline node accumulation: `**:first line\nsecond line;`
    // When we see `**:text` without a closing `;` on the same line, we
    // accumulate subsequent lines until a line ending with `;` is found.
    let mut multiline_buf: Option<(usize, usize, Side, String)> = None; // (line_no, depth, side, text)

    for (line_no, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // If we are accumulating a multiline node, keep collecting until `;`.
        if let Some((start_no, depth, side, ref mut buf)) = multiline_buf {
            if trimmed.ends_with(';') {
                // Last line of multiline node (strip trailing `;`).
                let last = trimmed.trim_end_matches(';').trim_end();
                if !last.is_empty() {
                    if !buf.is_empty() {
                        buf.push('\n');
                    }
                    buf.push_str(last);
                }
                let label = buf.replace('\n', " ");
                let node = MindMapNode {
                    label,
                    depth,
                    side,
                    children: Vec::new(),
                };
                let (depth, side) = (depth, side);
                multiline_buf = None;
                insert_node(
                    node,
                    depth,
                    side,
                    &mut roots,
                    &mut right_stack,
                    &mut left_stack,
                    start_no,
                )?;
            } else {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(trimmed);
            }
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        // Handle title directive.
        if let Some(rest) = trimmed.strip_prefix("title ") {
            meta.title = Some(rest.trim().to_string());
            continue;
        }
        if trimmed == "title" {
            // bare `title` with no text — skip
            continue;
        }

        // Determine prefix character and side.
        // `*` → right side; `#` → right side (markdown heading style); `-` → left side.
        let first = trimmed.chars().next().unwrap();
        let (bullet, side) = match first {
            '*' => ('*', Side::Right),
            '#' => ('#', Side::Right),
            '-' => ('-', Side::Left),
            _ => continue, // Not a node line — skip (skinparam, comment, etc.)
        };

        let count = trimmed.chars().take_while(|&c| c == bullet).count();
        let rest = trimmed[count..].trim();

        // Detect multiline block syntax: `**:first line` (no closing `;` yet).
        if let Some(after_colon) = rest.strip_prefix(':') {
            let depth = count;
            if after_colon.ends_with(';') {
                // Single-line block: `**:label;`
                let label = after_colon.trim_end_matches(';').trim().to_string();
                let label = if label.is_empty() {
                    return Err(ParseError {
                        line: line_no + 1,
                        message: "mind map node has no label".to_string(),
                    });
                } else {
                    label
                };
                let node = MindMapNode {
                    label,
                    depth,
                    side,
                    children: Vec::new(),
                };
                insert_node(
                    node,
                    depth,
                    side,
                    &mut roots,
                    &mut right_stack,
                    &mut left_stack,
                    line_no,
                )?;
            } else {
                // Start of multiline block — accumulate until `;`.
                multiline_buf = Some((line_no, depth, side, after_colon.trim().to_string()));
            }
            continue;
        }

        let label = rest.to_string();
        if label.is_empty() {
            return Err(ParseError {
                line: line_no + 1,
                message: "mind map node has no label".to_string(),
            });
        }

        let depth = count;
        let node = MindMapNode {
            label,
            depth,
            side,
            children: Vec::new(),
        };
        insert_node(
            node,
            depth,
            side,
            &mut roots,
            &mut right_stack,
            &mut left_stack,
            line_no,
        )?;
    }

    Ok(MindMapDiagram { meta, roots })
}

/// Insert a parsed `MindMapNode` into the tree, updating the appropriate
/// depth stack.
fn insert_node(
    node: MindMapNode,
    depth: usize,
    side: Side,
    roots: &mut Vec<MindMapNode>,
    right_stack: &mut Vec<usize>,
    left_stack: &mut Vec<usize>,
    line_no: usize,
) -> Result<(), ParseError> {
    let depth_stack = if side == Side::Right {
        right_stack
    } else {
        left_stack
    };

    if depth == 1 {
        // Root-level node (only `*` can be depth-1; `-` starts at depth 2).
        roots.push(node);
        depth_stack.clear();
        depth_stack.push(1);
    } else {
        // For left-side nodes, depth-2 nodes (`--`) attach to the most
        // recent depth-1 root just like right-side depth-2 nodes.
        // Ensure the stack references a valid root.
        if depth_stack.is_empty() {
            if roots.is_empty() {
                return Err(ParseError {
                    line: line_no + 1,
                    message: format!("depth-{depth} node has no parent (no preceding root node)"),
                });
            }
            // Implicitly attach to the most recent root.
            depth_stack.push(1);
        }

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
        let parent = find_deepest_at(roots, depth_stack, side).ok_or_else(|| ParseError {
            line: line_no + 1,
            message: "internal error: could not locate parent node".to_string(),
        })?;
        parent.children.push(node);
        depth_stack.push(depth);
    }
    Ok(())
}

/// Navigate to the parent node for the next insertion.
///
/// `depth_stack` holds the depths of all ancestor nodes in the current path,
/// starting from the root (depth 1).  To insert a new child, we need the
/// node at the *tip* of this stack (i.e., the most recently inserted
/// ancestor).
///
/// We start at `roots.last_mut()` (the most recent depth-1 node) and then
/// descend one level for each entry in `depth_stack[1..]`, following only
/// children whose `side` matches `side` when descending past the root.
fn find_deepest_at<'a>(
    roots: &'a mut [MindMapNode],
    depth_stack: &[usize],
    side: Side,
) -> Option<&'a mut MindMapNode> {
    // depth_stack always has at least one entry (depth 1 = root level).
    let steps_below_root = depth_stack.len().saturating_sub(1);
    let mut node = roots.last_mut()?;
    for _ in 0..steps_below_root {
        // Descend into the last child matching the requested side.
        node = node.children.iter_mut().rev().find(|c| c.side == side)?;
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
    fn left_side_branches() {
        let d = parse("* Root\n-- L1\n--- L1a\n-- L2");
        assert_eq!(d.roots.len(), 1);
        let root = &d.roots[0];
        assert_eq!(root.side, Side::Right);
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].label, "L1");
        assert_eq!(root.children[0].side, Side::Left);
        assert_eq!(root.children[0].children.len(), 1);
        assert_eq!(root.children[0].children[0].label, "L1a");
        assert_eq!(root.children[1].label, "L2");
        assert_eq!(root.children[1].side, Side::Left);
    }

    #[test]
    fn mixed_left_right() {
        let d = parse("* Root\n** R1\n-- L1\n-- L2");
        let root = &d.roots[0];
        assert_eq!(root.children.len(), 3);
        assert_eq!(root.children[0].label, "R1");
        assert_eq!(root.children[0].side, Side::Right);
        assert_eq!(root.children[1].label, "L1");
        assert_eq!(root.children[1].side, Side::Left);
        assert_eq!(root.children[2].label, "L2");
        assert_eq!(root.children[2].side, Side::Left);
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
