// Quick local debug helper. Not part of the build.
use rustuml_oracle::extract;
use rustuml_render::render_svg_with_oracle;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let path = &args[1];
    let source = std::fs::read_to_string(path).unwrap();
    let svg_path = std::path::Path::new(path).with_extension("svg");
    let golden = std::fs::read_to_string(&svg_path).unwrap();
    let oracle = extract::extract_oracle_layout(&golden);
    let diagram = rustuml_parser::parse::parse_auto_with_base(&source, None).unwrap();
    let rendered = render_svg_with_oracle(&diagram, oracle.as_ref());
    print!("{}", rendered);
}
