// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Math/LaTeX diagram model.

use super::DiagramMeta;
use serde::{Deserialize, Serialize};

/// A math diagram produced by `@startmath`/`@endmath` or
/// `@startlatex`/`@endlatex`.
#[derive(Debug, Serialize, Deserialize)]
pub struct MathDiagram {
    pub meta: DiagramMeta,
    /// The raw LaTeX content between the start/end tags.
    pub content: String,
    /// `true` when the source used `@startlatex`; `false` for `@startmath`.
    pub is_latex: bool,
}
