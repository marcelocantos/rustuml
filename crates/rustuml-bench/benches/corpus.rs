// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Criterion benchmarks over a deterministic sample of the golden corpus.
//!
//! Groups:
//! - `parse/<family>`  — `parse_shipped` on every sampled file of the family
//! - `render/<family>` — `render_shipped` on the pre-parsed diagrams
//! - `e2e/<family>`    — parse + render, what `rustuml file.puml` costs
//! - `layout/<n>`      — Graphviz dot layout of a synthetic n-node graph
//!
//! Throughput is reported per file (or per node for `layout`). Run with
//! `cargo bench -p rustuml-bench`; compare runs with `--save-baseline` /
//! `--baseline` as usual for criterion.

use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rustuml_bench::{CorpusFile, load_corpus, parse_shipped, render_shipped, sample_per_family};
use rustuml_layout::graph::{Direction, LayoutGraph};
use rustuml_parser::diagram::Diagram;

/// Files per family. Small so a full `cargo bench` stays under ten
/// minutes; the whole-corpus runner (`rustuml-bench run`) covers breadth.
const SAMPLES_PER_FAMILY: usize = 3;
const WARM_UP: Duration = Duration::from_millis(500);
const MEASUREMENT: Duration = Duration::from_secs(2);
const SAMPLE_SIZE: usize = 20;
/// Node counts for the synthetic layout graphs.
const LAYOUT_SIZES: &[usize] = &[10, 50, 200];
/// Fan-out of the synthetic layout tree; every node also gets one
/// cross-edge so the ranking pass has real work to do.
const LAYOUT_FANOUT: usize = 3;
const LAYOUT_NODE_W: f64 = 80.0;
const LAYOUT_NODE_H: f64 = 40.0;

fn families(sample: &[CorpusFile]) -> Vec<String> {
    let mut f: Vec<String> = sample.iter().map(|c| c.family.clone()).collect();
    f.sort();
    f.dedup();
    f
}

fn corpus_benches(c: &mut Criterion) {
    let corpus = load_corpus(&rustuml_bench::golden_root());
    if corpus.is_empty() {
        eprintln!("golden submodule not populated — run: git submodule update --init");
        return;
    }
    let sample = sample_per_family(&corpus, SAMPLES_PER_FAMILY);
    // Warm process-wide lazies (font tables) so they are not measured.
    for f in &sample {
        let _ = rustuml_bench::time_file(f);
    }

    for family in families(&sample) {
        let files: Vec<&CorpusFile> = sample.iter().filter(|f| f.family == family).collect();
        let parsed: Vec<Diagram> = files
            .iter()
            .map(|f| parse_shipped(&f.source, f.path.parent()).expect("sampled file parses"))
            .collect();
        let n = files.len() as u64;

        let mut g = c.benchmark_group("parse");
        g.warm_up_time(WARM_UP)
            .measurement_time(MEASUREMENT)
            .sample_size(SAMPLE_SIZE)
            .throughput(Throughput::Elements(n));
        g.bench_with_input(BenchmarkId::from_parameter(&family), &files, |b, files| {
            b.iter(|| {
                for f in files.iter() {
                    std::hint::black_box(parse_shipped(&f.source, f.path.parent()).unwrap());
                }
            })
        });
        g.finish();

        let mut g = c.benchmark_group("render");
        g.warm_up_time(WARM_UP)
            .measurement_time(MEASUREMENT)
            .sample_size(SAMPLE_SIZE)
            .throughput(Throughput::Elements(n));
        g.bench_with_input(
            BenchmarkId::from_parameter(&family),
            &parsed,
            |b, parsed| {
                b.iter(|| {
                    for d in parsed.iter() {
                        std::hint::black_box(render_shipped(d));
                    }
                })
            },
        );
        g.finish();

        let mut g = c.benchmark_group("e2e");
        g.warm_up_time(WARM_UP)
            .measurement_time(MEASUREMENT)
            .sample_size(SAMPLE_SIZE)
            .throughput(Throughput::Elements(n));
        g.bench_with_input(BenchmarkId::from_parameter(&family), &files, |b, files| {
            b.iter(|| {
                for f in files.iter() {
                    let d = parse_shipped(&f.source, f.path.parent()).unwrap();
                    std::hint::black_box(render_shipped(&d));
                }
            })
        });
        g.finish();
    }
}

fn synthetic_graph(n: usize) -> LayoutGraph {
    let mut g = LayoutGraph::new(Direction::TopToBottom);
    for i in 0..n {
        let id = format!("n{i}");
        g.add_node(&id, &id, LAYOUT_NODE_W, LAYOUT_NODE_H);
    }
    for i in 1..n {
        let parent = (i - 1) / LAYOUT_FANOUT;
        g.add_edge(&format!("n{parent}"), &format!("n{i}"), None);
        let cross = (i * 7) % n;
        if cross != i && cross != parent {
            g.add_edge(&format!("n{i}"), &format!("n{cross}"), Some("x"));
        }
    }
    g
}

fn layout_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("layout");
    g.warm_up_time(WARM_UP)
        .measurement_time(MEASUREMENT)
        .sample_size(SAMPLE_SIZE);
    for &n in LAYOUT_SIZES {
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let graph = synthetic_graph(n);
            b.iter(|| std::hint::black_box(graph.layout_full_no_timeout()))
        });
    }
    g.finish();
}

criterion_group!(benches, corpus_benches, layout_benches);
criterion_main!(benches);
