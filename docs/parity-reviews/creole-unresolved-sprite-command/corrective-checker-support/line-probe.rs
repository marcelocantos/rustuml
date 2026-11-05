// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0
fn main() {
    use rustuml_render::{creole, text_render as t};
    for size in [7, 13, 26] {
        for content in ["", " ", "<$seed>", "<size:29></size>", "<size:29><color:red><$seed></color></size>", "<b></b>", "<outer<b></b>>"] {
            let base = t::TextBase { x: 0.0, y: 40.0, font_size: size, font_family: "sans-serif", fill: "#000000", bold: false, italic: false, underline: false, skip_underline: false };
            let mut svg = String::new();
            let painted_width = t::emit_text(&mut svg, content, &base);
            println!("size={size} input={content:?} lexical={:?} width={} painted_width={painted_width} height={} ascent={} first_baseline={} baselines={} elements={} svg={svg:?}", creole::parse_segments(content), t::total_width(content, &base), t::label_height(content, size as f64), t::label_ascent(content,size as f64), t::label_first_baseline_ascent_with_family(content,size as f64,"sans-serif"),t::emitted_baseline_count(content,&base),t::emitted_text_element_count(content,&base));
        }
    }
}
