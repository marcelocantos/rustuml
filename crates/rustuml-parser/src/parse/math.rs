// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Math/LaTeX diagram parser.
//!
//! Extracts the raw LaTeX content between `@startmath`/`@endmath` or
//! `@startlatex`/`@endlatex` tags.

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::math::MathDiagram;

/// Parse pre-processed lines into a [`MathDiagram`].
///
/// `is_latex` should be `true` when the source used `@startlatex`,
/// `false` for `@startmath`.
pub fn parse_math(lines: &[String], is_latex: bool) -> Result<MathDiagram, ParseError> {
    let end_tag = if is_latex { "@endlatex" } else { "@endmath" };
    let start_tag = if is_latex { "@startlatex" } else { "@startmath" };

    let mut content_lines: Vec<&str> = Vec::new();
    let mut inside = false;

    for line in lines {
        let trimmed = line.trim();
        if !inside {
            if trimmed.eq_ignore_ascii_case(start_tag) {
                inside = true;
            }
            continue;
        }
        if trimmed.eq_ignore_ascii_case(end_tag) {
            break;
        }
        content_lines.push(line.as_str());
    }

    let content = content_lines.join("\n");

    Ok(MathDiagram {
        meta: DiagramMeta::default(),
        content,
        is_latex,
    })
}
