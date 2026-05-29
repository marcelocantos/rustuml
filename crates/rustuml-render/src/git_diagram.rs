// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Git log diagram SVG renderer.
//!
//! Java PlantUML's git renderer currently emits an empty 21x21 SVG with
//! a `<?plantuml-src ?>` processing instruction containing the compressed
//! source — no visible nodes or branch lanes. To stay structurally
//! identical to the goldens, we emit the same empty envelope. The
//! richer rendering implemented previously is preserved in git history
//! and can be revived if/when Java PlantUML gains a real visual output.

use rustuml_parser::diagram::git_diagram::GitDiagram;

use crate::style::Theme;

pub fn render(_diagram: &GitDiagram, _theme: &Theme) -> String {
    r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" contentStyleType="text/css" data-diagram-type="GIT" height="21px" preserveAspectRatio="none" style="width:21px;height:21px;background:#FFFFFF;" version="1.1" viewBox="0 0 21 21" width="21px" zoomAndPan="magnify"><defs/><g></g></svg>"#
        .to_string()
}
