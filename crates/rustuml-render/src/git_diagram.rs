// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Git log diagram SVG renderer.
//!
//! Produces a simple left-to-right visualization:
//! - Each branch gets a horizontal lane with a coloured line.
//! - Commits are circles placed along the branch lane.
//! - Tags are shown as labels above commit nodes.
//! - Merge lines connect the source branch to the target.

use std::collections::HashMap;

use rustuml_parser::diagram::git_diagram::{CommitType, GitCommand, GitDiagram};

use crate::style::Theme;
use crate::svg::SvgBuilder;

// ── Layout constants ─────────────────────────────────────────────────────────

const COMMIT_SPACING: f64 = 60.0;
const LANE_HEIGHT: f64 = 40.0;
const MARGIN: f64 = 20.0;
const COMMIT_RADIUS: f64 = 8.0;
const FONT_SIZE: f64 = 11.0;

/// Branch colours (cycled).
const BRANCH_COLORS: &[&str] = &[
    "#4CAF50", // green (master)
    "#2196F3", // blue
    "#FF9800", // orange
    "#9C27B0", // purple
    "#F44336", // red
    "#00BCD4", // cyan
    "#795548", // brown
];

/// Render a git diagram to SVG.
pub fn render(diagram: &GitDiagram, _theme: &Theme) -> String {
    // Simulate the git state to build a commit graph.
    let mut branches: Vec<String> = vec!["master".to_string()];
    let mut current_branch = "master".to_string();
    let mut branch_color_map: HashMap<String, &str> = HashMap::new();
    branch_color_map.insert("master".to_string(), BRANCH_COLORS[0]);

    // First pass: discover all branches and assign colours.
    for cmd in &diagram.commands {
        if let GitCommand::Branch(name) = cmd
            && !branches.contains(name)
        {
            let idx = branches.len() % BRANCH_COLORS.len();
            branch_color_map.insert(name.clone(), BRANCH_COLORS[idx]);
            branches.push(name.clone());
        }
    }

    // Build a list of commit positions and annotations.
    struct CommitNode {
        x: f64,
        y: f64,
        color: String,
        id: Option<String>,
        tag: Option<String>,
        commit_type: CommitType,
    }
    struct MergeLine {
        from_y: f64,
        to_x: f64,
        to_y: f64,
        color: String,
    }

    let mut commits: Vec<CommitNode> = Vec::new();
    let mut merges: Vec<MergeLine> = Vec::new();
    let mut x_cursor = MARGIN + COMMIT_SPACING;

    let branch_y = |name: &str, branches: &[String]| -> f64 {
        let idx = branches.iter().position(|b| b == name).unwrap_or(0);
        MARGIN + 20.0 + idx as f64 * LANE_HEIGHT
    };

    for cmd in &diagram.commands {
        match cmd {
            GitCommand::Commit {
                id,
                tag,
                commit_type,
            } => {
                let y = branch_y(&current_branch, &branches);
                let color = branch_color_map
                    .get(&current_branch)
                    .unwrap_or(&"#333")
                    .to_string();
                commits.push(CommitNode {
                    x: x_cursor,
                    y,
                    color,
                    id: id.clone(),
                    tag: tag.clone(),
                    commit_type: commit_type.clone(),
                });
                x_cursor += COMMIT_SPACING;
            }
            GitCommand::Branch(name) => {
                if !branches.contains(name) {
                    let idx = branches.len() % BRANCH_COLORS.len();
                    branch_color_map.insert(name.clone(), BRANCH_COLORS[idx]);
                    branches.push(name.clone());
                }
            }
            GitCommand::Checkout(name) => {
                current_branch = name.clone();
            }
            GitCommand::Merge { branch, tag } => {
                // The merge creates a commit on the current branch.
                let target_y = branch_y(&current_branch, &branches);
                let source_y = branch_y(branch, &branches);
                let color = branch_color_map
                    .get(&current_branch)
                    .unwrap_or(&"#333")
                    .to_string();
                merges.push(MergeLine {
                    from_y: source_y,
                    to_x: x_cursor,
                    to_y: target_y,
                    color: color.clone(),
                });
                commits.push(CommitNode {
                    x: x_cursor,
                    y: target_y,
                    color,
                    id: None,
                    tag: tag.clone(),
                    commit_type: CommitType::Normal,
                });
                x_cursor += COMMIT_SPACING;
            }
            GitCommand::CherryPick { id } => {
                let y = branch_y(&current_branch, &branches);
                let color = branch_color_map
                    .get(&current_branch)
                    .unwrap_or(&"#333")
                    .to_string();
                commits.push(CommitNode {
                    x: x_cursor,
                    y,
                    color,
                    id: Some(id.clone()),
                    tag: None,
                    commit_type: CommitType::Highlight,
                });
                x_cursor += COMMIT_SPACING;
            }
        }
    }

    // Compute canvas size.
    let total_width = x_cursor + MARGIN;
    let total_height = MARGIN * 2.0 + 20.0 + branches.len() as f64 * LANE_HEIGHT;

    let mut svg = SvgBuilder::new(total_width, total_height);

    // Draw branch lanes (horizontal lines).
    for (i, branch_name) in branches.iter().enumerate() {
        let y = MARGIN + 20.0 + i as f64 * LANE_HEIGHT;
        let color = branch_color_map.get(branch_name).unwrap_or(&"#333");
        svg.line_segment(MARGIN, y, x_cursor, y, color, false);
        // Branch label.
        svg.text(MARGIN - 2.0, y - 12.0, branch_name, "end", FONT_SIZE);
    }

    // Draw merge lines.
    for m in &merges {
        svg.line_segment(m.to_x, m.from_y, m.to_x, m.to_y, &m.color, false);
    }

    // Draw commit nodes.
    for c in &commits {
        let (fill, stroke) = match c.commit_type {
            CommitType::Normal => (c.color.as_str(), "#181818"),
            CommitType::Reverse => ("#181818", c.color.as_str()),
            CommitType::Highlight => ("#FFFF00", c.color.as_str()),
        };
        svg.circle(c.x, c.y, COMMIT_RADIUS, fill, stroke);

        // Commit id label (below).
        if let Some(id) = &c.id {
            svg.text(c.x, c.y + COMMIT_RADIUS + 14.0, id, "middle", FONT_SIZE);
        }

        // Tag label (above).
        if let Some(tag) = &c.tag {
            svg.text(c.x, c.y - COMMIT_RADIUS - 6.0, tag, "middle", FONT_SIZE);
        }
    }

    svg.finalize()
}
