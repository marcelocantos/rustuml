// Private checker tool: compare expected/actual SVG with the unchanged strict comparator.
// Usage: checker-cmp <listfile>  (each line: expected<TAB>actual) -> prints name\tMATCH|DIFF|ERR\tndiffs
use rustuml_oracle::compare;
fn main() {
    let list = std::env::args().nth(1).unwrap();
    for line in std::fs::read_to_string(list).unwrap().lines() {
        let mut it = line.split('\t');
        let (e, a) = (it.next().unwrap(), it.next().unwrap());
        let exp = std::fs::read_to_string(e).unwrap_or_default();
        let act = std::fs::read_to_string(a).unwrap_or_default();
        match compare::compare_svg_strict(&exp, &act) {
            Ok(r) => {
                let d = format!("{:?}", r.differences.iter().take(3).collect::<Vec<_>>());
                println!("{a}\t{}\t{}\t{}", if r.is_match() {"MATCH"} else {"DIFF"}, r.differences.len(), d.chars().take(600).collect::<String>().replace('\n'," "));
            }
            Err(err) => println!("{a}\tERR\t0\t{}", err.chars().take(200).collect::<String>()),
        }
    }
}
