// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
fn main() {
    // Exact label from the held-out legacy_title source, not another fixture.
    let label = "A<$north>B";
    println!("shared={:?}", rustuml_render::creole::parse_segments(label));
    println!("legacy={}", rustuml_render::creole::to_svg_tspans(label));
}
