// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Math/LaTeX diagram parser.
//!
//! The preprocessor already strips `@startmath`/`@endmath` (and the latex
//! variants), so the `lines` slice contains only the body content.

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::math::MathDiagram;

/// Parse pre-processed lines into a [`MathDiagram`].
///
/// `is_latex` should be `true` when the source used `@startlatex`,
/// `false` for `@startmath`.
pub fn parse_math(lines: &[String], is_latex: bool) -> Result<MathDiagram, ParseError> {
    // The preprocessor strips @start/@end tags, so all remaining lines
    // are body content.
    let content = lines
        .iter()
        .map(|l| l.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(MathDiagram {
        meta: DiagramMeta::default(),
        content,
        is_latex,
    })
}
