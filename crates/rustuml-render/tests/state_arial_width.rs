// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! State titles drawn in Arial use that face's advance table.
//!
//! Java `EntityImageStateCommon` (70-75) builds the title `TextBlock` from the
//! state style's `FontConfiguration`, and `DriverTextSvg` (128-129) writes
//! that face's `StringBounder` width as `textLength`. Widths below are
//! `fmt_coord` of units/2048*14:
//! italic "Kiln wq vista" 11382 units; held-out "Held out kiln" 11270 units
//! (trailing zero stripped); bold "Kiln wq vista" 12518 units; plain "A"/"B"
//! 1366 units. This file is outside `src/` so those measured widths are not
//! production constants.

use rustuml_render::render_svg;

fn svg_of(source: &str) -> String {
    render_svg(&rustuml_parser::parse::parse(source).unwrap())
}

#[test]
fn arial_state_name_uses_the_face_advance_table() {
    let italic = "\
@startuml
skinparam defaultFontName Arial
skinparam stateFontStyle italic
state \"Kiln wq vista\" as E1
@enduml
";
    let svg = svg_of(italic);
    let needle = concat!(
        r#"font-family="Arial" font-size="14" font-style="italic" "#,
        r#"lengthAdjust="spacing" textLength="77.8066""#
    );
    assert!(svg.contains(needle), "{svg}");

    let held = "\
@startuml
skinparam stateFontName Arial
state \"Held out kiln\" as H1
@enduml
";
    let svg = svg_of(held);
    assert!(svg.contains("textLength=\"77.041\""), "{svg}");

    let bold = "\
@startuml
skinparam stateFontName Arial
skinparam stateFontStyle bold
state \"Kiln wq vista\" as E1
@enduml
";
    let svg = svg_of(bold);
    assert!(svg.contains("textLength=\"85.5723\""), "{svg}");
    assert!(svg.contains("font-weight=\"700\""), "{svg}");

    let letters = "\
@startuml
skinparam state {
  FontName Arial
}
[*] --> A
A --> B
@enduml
";
    let svg = svg_of(letters);
    assert!(svg.contains("textLength=\"9.3379\""), "{svg}");
}
