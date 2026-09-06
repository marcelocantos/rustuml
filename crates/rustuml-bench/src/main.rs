// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! `rustuml-bench` — corpus throughput runner and perf ratchet.
//!
//! ```text
//! rustuml-bench run   [--serial] [--family F] [--top N]   whole-corpus throughput + slowest files
//! rustuml-bench check [--write]                            ratchet against docs/perf/baseline.md
//! rustuml-bench profile <name-substring> [--iters N]        loop one file/family for a profiler
//! ```

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use rayon::prelude::*;
use rustuml_bench::counting_alloc::{CountingAlloc, Snapshot};
use rustuml_bench::{CorpusFile, Failure, FileTiming, load_corpus, sample_per_family};

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

/// Files per family in the ratchet sample. 34 families × 6 ≈ 200 files,
/// a few seconds single-threaded, so the gate stays cheap enough to run
/// on every `make bullseye`.
const CHECK_SAMPLES_PER_FAMILY: usize = 6;
/// Repeats per family in `check`; wall time takes the minimum.
const CHECK_REPEATS: usize = 3;
/// Allowed drift, both directions, in allocation count and bytes. Counts
/// are deterministic except for hash-seed-dependent container growth,
/// which stays well under this. Anything past it, faster or slower, must
/// be locked in with `check --write` on the same change.
const ALLOC_TOLERANCE_PCT: f64 = 1.0;
/// Wall-time drift allowed when `RUSTUML_PERF_WALL_GATE=1`. Wide because
/// wall time is machine- and load-dependent; the default gate ignores it.
const WALL_TOLERANCE_PCT: f64 = 25.0;
const BASELINE_BEGIN: &str = "<!-- perf-baseline:begin -->";
const BASELINE_END: &str = "<!-- perf-baseline:end -->";
const TOTAL_ROW: &str = "TOTAL";
const DEFAULT_TOP: usize = 25;
/// Families whose shipped render path shells out to an external binary
/// (`dot_diagram::try_render_with_dot` pipes `@startdot` bodies through
/// Graphviz's `dot` when it is on PATH). Their counts depend on the host,
/// so they are reported by `run` but excluded from the ratchet sample.
const EXTERNAL_PROCESS_FAMILIES: &[&str] = &["dot"];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("run") => run(&args[1..]),
        Some("check") => check(&args[1..]),
        Some("profile") => profile(&args[1..]),
        _ => {
            eprintln!("usage: rustuml-bench run [--serial] [--family F] [--top N]");
            eprintln!("       rustuml-bench check [--write]");
            eprintln!("       rustuml-bench profile <name-substring> [--iters N]");
            2
        }
    };
    std::process::exit(code);
}

fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn load() -> Vec<CorpusFile> {
    let root = rustuml_bench::golden_root();
    let corpus = load_corpus(&root);
    if corpus.is_empty() {
        eprintln!(
            "no golden corpus at {} — run: git submodule update --init",
            root.display()
        );
        std::process::exit(2);
    }
    corpus
}

#[derive(Default)]
struct FamilyStats {
    files: usize,
    ok: usize,
    parse_fail: usize,
    panic: usize,
    parse: Duration,
    render: Duration,
    max_render: Duration,
    svg_bytes: usize,
}

fn run(args: &[String]) -> i32 {
    let serial = args.iter().any(|a| a == "--serial");
    let family = flag_value(args, "--family");
    let top: usize = flag_value(args, "--top")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_TOP);

    let mut corpus = load();
    if let Some(f) = family {
        corpus.retain(|c| c.family == f);
    }
    // Warm process-wide lazies (font tables, regex caches) so the first
    // file does not carry them.
    if let Some(first) = corpus.first() {
        let _ = rustuml_bench::time_file(first);
    }

    let t0 = Instant::now();
    let results: Vec<(usize, Result<FileTiming, Failure>)> = if serial {
        corpus
            .iter()
            .enumerate()
            .map(|(i, f)| (i, rustuml_bench::time_file(f)))
            .collect()
    } else {
        corpus
            .par_iter()
            .enumerate()
            .map(|(i, f)| (i, rustuml_bench::time_file(f)))
            .collect()
    };
    let wall = t0.elapsed();

    let mut families: BTreeMap<&str, FamilyStats> = BTreeMap::new();
    let mut total = FamilyStats::default();
    let mut slowest: Vec<(Duration, &CorpusFile)> = Vec::new();
    let mut panics: Vec<(&CorpusFile, &str)> = Vec::new();
    for (i, r) in &results {
        let file = &corpus[*i];
        let fam = families.entry(file.family.as_str()).or_default();
        for s in [&mut *fam, &mut total] {
            s.files += 1;
            match r {
                Ok(t) => {
                    s.ok += 1;
                    s.parse += t.parse;
                    s.render += t.render;
                    s.max_render = s.max_render.max(t.render);
                    s.svg_bytes += t.svg_bytes;
                }
                Err(Failure::Parse(_)) => s.parse_fail += 1,
                Err(Failure::Panic(_)) => s.panic += 1,
            }
        }
        match r {
            Ok(t) => slowest.push((t.render, file)),
            Err(Failure::Panic(msg)) => panics.push((file, msg)),
            Err(Failure::Parse(_)) => {}
        }
    }

    let mode = if serial { "serial" } else { "parallel" };
    println!(
        "corpus: {} files, {} ok, {} parse-fail, {} panic; {mode} wall {:.2}s ({:.0} files/s); \
         cpu-sum parse {:.2}s render {:.2}s; svg {:.1} MB",
        total.files,
        total.ok,
        total.parse_fail,
        total.panic,
        wall.as_secs_f64(),
        total.files as f64 / wall.as_secs_f64(),
        total.parse.as_secs_f64(),
        total.render.as_secs_f64(),
        total.svg_bytes as f64 / 1e6,
    );
    println!();
    println!(
        "{:<16} {:>6} {:>5} {:>5} {:>9} {:>9} {:>9} {:>9}",
        "family", "files", "pfail", "panic", "parse_ms", "render_ms", "mean_ms", "max_ms"
    );
    let mut rows: Vec<(&str, &FamilyStats)> = families.iter().map(|(k, v)| (*k, v)).collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1.render));
    for (name, s) in rows {
        println!(
            "{:<16} {:>6} {:>5} {:>5} {:>9.1} {:>9.1} {:>9.2} {:>9.1}",
            name,
            s.files,
            s.parse_fail,
            s.panic,
            s.parse.as_secs_f64() * 1e3,
            s.render.as_secs_f64() * 1e3,
            s.render.as_secs_f64() * 1e3 / s.ok.max(1) as f64,
            s.max_render.as_secs_f64() * 1e3,
        );
    }

    slowest.sort_by_key(|s| std::cmp::Reverse(s.0));
    println!();
    println!("slowest {} renders:", top.min(slowest.len()));
    for (d, f) in slowest.iter().take(top) {
        println!("{:>9.1} ms  {}", d.as_secs_f64() * 1e3, f.name);
    }
    if !panics.is_empty() {
        println!();
        println!("panics ({}):", panics.len());
        for (f, msg) in panics.iter().take(top) {
            let first_line = msg.lines().next().unwrap_or("");
            println!("  {}: {first_line}", f.name);
        }
    }
    0
}

/// One measured row of the ratchet table.
#[derive(Debug, Clone, PartialEq)]
struct Row {
    family: String,
    files: usize,
    allocs: usize,
    bytes: usize,
    parse_us: u64,
    render_us: u64,
}

fn measure_family(files: &[&CorpusFile]) -> Row {
    let mut allocs = usize::MAX;
    let mut bytes = usize::MAX;
    let mut parse_min = Duration::MAX;
    let mut render_min = Duration::MAX;
    for _ in 0..CHECK_REPEATS {
        let before = Snapshot::now();
        let mut parse = Duration::ZERO;
        let mut render = Duration::ZERO;
        for f in files {
            let t = rustuml_bench::time_file(f).expect("sampled file must render");
            parse += t.parse;
            render += t.render;
        }
        let delta = Snapshot::now().since(before);
        // Allocation counts should be identical across repeats; take the
        // minimum so any first-touch lazy initialisation is excluded.
        allocs = allocs.min(delta.allocs);
        bytes = bytes.min(delta.bytes);
        parse_min = parse_min.min(parse);
        render_min = render_min.min(render);
    }
    Row {
        family: files[0].family.clone(),
        files: files.len(),
        allocs,
        bytes,
        parse_us: parse_min.as_micros() as u64,
        render_us: render_min.as_micros() as u64,
    }
}

fn measure_all(sample: &[CorpusFile]) -> Vec<Row> {
    let mut families: Vec<&str> = sample.iter().map(|f| f.family.as_str()).collect();
    families.sort_unstable();
    families.dedup();
    let mut rows: Vec<Row> = families
        .iter()
        .map(|fam| {
            let files: Vec<&CorpusFile> = sample.iter().filter(|f| f.family == *fam).collect();
            measure_family(&files)
        })
        .collect();
    let total = Row {
        family: TOTAL_ROW.to_string(),
        files: rows.iter().map(|r| r.files).sum(),
        allocs: rows.iter().map(|r| r.allocs).sum(),
        bytes: rows.iter().map(|r| r.bytes).sum(),
        parse_us: rows.iter().map(|r| r.parse_us).sum(),
        render_us: rows.iter().map(|r| r.render_us).sum(),
    };
    rows.push(total);
    rows
}

fn render_table(rows: &[Row]) -> String {
    let mut s = String::new();
    s.push_str("| family | files | allocs | bytes | parse_us | render_us |\n");
    s.push_str("|---|---:|---:|---:|---:|---:|\n");
    for r in rows {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            r.family, r.files, r.allocs, r.bytes, r.parse_us, r.render_us
        ));
    }
    s
}

fn parse_table(md: &str) -> Result<Vec<Row>, String> {
    let begin = md
        .find(BASELINE_BEGIN)
        .ok_or_else(|| format!("baseline.md: missing {BASELINE_BEGIN}"))?;
    let end = md
        .find(BASELINE_END)
        .ok_or_else(|| format!("baseline.md: missing {BASELINE_END}"))?;
    let mut rows = Vec::new();
    for line in md[begin..end].lines() {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() != 6 || cells[0] == "family" || cells[0].starts_with("---") {
            continue;
        }
        let num = |i: usize| -> Result<u64, String> {
            cells[i].parse::<u64>().map_err(|e| {
                format!(
                    "baseline.md: bad number {:?} in row {:?}: {e}",
                    cells[i], cells[0]
                )
            })
        };
        rows.push(Row {
            family: cells[0].to_string(),
            files: num(1)? as usize,
            allocs: num(2)? as usize,
            bytes: num(3)? as usize,
            parse_us: num(4)?,
            render_us: num(5)?,
        });
    }
    Ok(rows)
}

fn pct_delta(now: f64, base: f64) -> f64 {
    if base == 0.0 {
        return if now == 0.0 { 0.0 } else { f64::INFINITY };
    }
    (now - base) / base * 100.0
}

fn check(args: &[String]) -> i32 {
    let write = args.iter().any(|a| a == "--write");
    let wall_gate = std::env::var("RUSTUML_PERF_WALL_GATE").is_ok_and(|v| v == "1");
    let mut corpus = load();
    corpus.retain(|f| !EXTERNAL_PROCESS_FAMILIES.contains(&f.family.as_str()));
    let sample = sample_per_family(&corpus, CHECK_SAMPLES_PER_FAMILY);
    eprintln!(
        "ratchet sample: {} files across {} families",
        sample.len(),
        sample
            .iter()
            .map(|f| f.family.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    // Warm process-wide lazies before counting.
    for f in &sample {
        let _ = rustuml_bench::time_file(f);
    }
    let rows = measure_all(&sample);
    let table = render_table(&rows);
    print!("{table}");

    let path = rustuml_bench::baseline_path();
    let md = std::fs::read_to_string(&path).unwrap_or_default();
    if write {
        let begin = md.find(BASELINE_BEGIN);
        let end = md.find(BASELINE_END);
        let new_md = match (begin, end) {
            (Some(b), Some(e)) => format!("{}{}\n{}{}", &md[..b], BASELINE_BEGIN, table, &md[e..]),
            _ => format!("{md}\n{BASELINE_BEGIN}\n{table}{BASELINE_END}\n"),
        };
        std::fs::write(&path, new_md).expect("write baseline.md");
        eprintln!("baseline written to {}", path.display());
        return 0;
    }

    let baseline = match parse_table(&md) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e} — run `rustuml-bench check --write` to lock a baseline");
            return 1;
        }
    };

    let mut failures = Vec::new();
    for now in &rows {
        let Some(base) = baseline.iter().find(|b| b.family == now.family) else {
            failures.push(format!(
                "{}: not in baseline (lock it with --write)",
                now.family
            ));
            continue;
        };
        if base.files != now.files {
            failures.push(format!(
                "{}: sample size {} → {} (corpus changed; re-lock with --write)",
                now.family, base.files, now.files
            ));
        }
        let checks: [(&str, f64, f64, f64, bool); 4] = [
            (
                "allocs",
                now.allocs as f64,
                base.allocs as f64,
                ALLOC_TOLERANCE_PCT,
                true,
            ),
            (
                "bytes",
                now.bytes as f64,
                base.bytes as f64,
                ALLOC_TOLERANCE_PCT,
                true,
            ),
            (
                "parse_us",
                now.parse_us as f64,
                base.parse_us as f64,
                WALL_TOLERANCE_PCT,
                wall_gate,
            ),
            (
                "render_us",
                now.render_us as f64,
                base.render_us as f64,
                WALL_TOLERANCE_PCT,
                wall_gate,
            ),
        ];
        for (metric, now_v, base_v, tol, gated) in checks {
            let d = pct_delta(now_v, base_v);
            if d.abs() > tol {
                let verdict = if d < 0.0 {
                    "faster/leaner — lock it in"
                } else {
                    "regression"
                };
                let line = format!(
                    "{}: {metric} {base_v} → {now_v} ({d:+.1}%) {verdict}",
                    now.family
                );
                if gated {
                    failures.push(line);
                } else {
                    eprintln!("note (not gated): {line}");
                }
            }
        }
    }
    for base in &baseline {
        if !rows.iter().any(|r| r.family == base.family) {
            failures.push(format!(
                "{}: in baseline but no longer sampled",
                base.family
            ));
        }
    }

    if failures.is_empty() {
        eprintln!(
            "perf ratchet OK (allocs/bytes within ±{ALLOC_TOLERANCE_PCT}%{})",
            if wall_gate {
                format!(", wall within ±{WALL_TOLERANCE_PCT}%")
            } else {
                String::new()
            }
        );
        0
    } else {
        eprintln!("perf ratchet FAILED:");
        for f in &failures {
            eprintln!("  {f}");
        }
        eprintln!(
            "If the change is intentional, re-lock with: cargo run -p rustuml-bench --release -- check --write"
        );
        1
    }
}

fn profile(args: &[String]) -> i32 {
    let Some(needle) = args.first() else {
        eprintln!("usage: rustuml-bench profile <name-substring> [--iters N]");
        return 2;
    };
    let iters: usize = flag_value(args, "--iters")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let corpus = load();
    let files: Vec<&CorpusFile> = corpus
        .iter()
        .filter(|f| f.family == *needle || f.name.contains(needle.as_str()))
        .collect();
    if files.is_empty() {
        eprintln!("no corpus file matches {needle:?}");
        return 1;
    }
    eprintln!(
        "profiling {} files × {iters} iterations (pid {})",
        files.len(),
        std::process::id()
    );
    let t0 = Instant::now();
    let mut parse = Duration::ZERO;
    let mut render = Duration::ZERO;
    for _ in 0..iters {
        for f in &files {
            if let Ok(t) = rustuml_bench::time_file(f) {
                parse += t.parse;
                render += t.render;
            }
        }
    }
    println!(
        "wall {:.2}s parse {:.2}s render {:.2}s",
        t0.elapsed().as_secs_f64(),
        parse.as_secs_f64(),
        render.as_secs_f64()
    );
    0
}
