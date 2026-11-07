// Reads lines "expected_path\tactual_path" on stdin and prints, per line,
// "MATCH" or "DIFF <n>" or "ERR <msg>" using the repo's compare_svg_strict
// (vendored verbatim from da6c18866, unchanged since 9fb2e52a6).
#[allow(dead_code)]
mod compare;
use std::io::BufRead;
fn main() {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let mut it = line.split('\t');
        let (e, a) = (it.next().unwrap(), it.next().unwrap());
        let exp = std::fs::read_to_string(e);
        let act = std::fs::read_to_string(a);
        match (exp, act) {
            (Ok(exp), Ok(act)) => match compare::compare_svg_strict(&exp, &act) {
                Ok(r) if r.differences.is_empty() => println!("MATCH"),
                Ok(r) => { println!("DIFF {}", r.differences.len()); if std::env::var("OPUS_VERBOSE").is_ok() { for d in &r.differences { eprintln!("  {:?}", d); } } }
                Err(m) => println!("ERR {}", m.replace('\n', " ")),
            },
            _ => println!("ERR read"),
        }
    }
}
