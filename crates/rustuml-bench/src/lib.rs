// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Runtime performance harness for RustUML.
//!
//! Everything here measures the **shipped path**: the exact parse and
//! render calls `crates/rustuml/src/main.rs` makes for `rustuml file.puml`
//! (no oracle layout, default theme). The workload is the golden corpus
//! under `test-diagrams/golden/`, the same files `golden_pairs` walks.
//!
//! Three consumers share this module:
//!
//! - `benches/corpus.rs` — criterion micro-benchmarks on a deterministic
//!   per-family sample.
//! - `src/main.rs run` — whole-corpus throughput and slowest-file report.
//! - `src/main.rs check` — the perf ratchet: allocation counts on the
//!   sample compared both ways against `docs/perf/baseline.md`.

pub mod counting_alloc;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rustuml_parser::diagram::Diagram;
use rustuml_parser::parse::ParseError;

/// One `.puml` from the golden corpus.
#[derive(Debug, Clone)]
pub struct CorpusFile {
    /// Top-level golden directory (`class`, `sequence`, ...).
    pub family: String,
    /// Path relative to the golden root, extension stripped.
    pub name: String,
    pub path: PathBuf,
    pub source: String,
}

/// Same list as `golden_pairs`: files starting with any other `@start`
/// keyword are skipped because the parser rejects them.
const SUPPORTED_START_KEYWORDS: &[&str] = &[
    "@startuml",
    "@startgantt",
    "@startmindmap",
    "@startwbs",
    "@startmath",
    "@startlatex",
    "@startregex",
    "@startjson",
    "@startyaml",
    "@startsalt",
    "@startnwdiag",
    "@startditaa",
    "@startdot",
    "@startboard",
    "@startgit",
    "@startebnf",
];

/// `test-diagrams/golden` relative to this crate (a git submodule).
pub fn golden_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-diagrams/golden")
}

/// Path of the ratchet lock file, `docs/perf/baseline.md`.
pub fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs/perf/baseline.md")
}

fn has_supported_start_keyword(source: &str) -> bool {
    let first_line = source
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();
    if !first_line.starts_with("@start") {
        return true;
    }
    SUPPORTED_START_KEYWORDS
        .iter()
        .any(|kw| first_line.starts_with(kw))
}

/// Walk the golden corpus. Returns files sorted by path, skipping those
/// without a sibling `.svg`, with an unsupported `@start` keyword, or
/// that are ditaa (raster output, not on the SVG path).
pub fn load_corpus(root: &Path) -> Vec<CorpusFile> {
    let mut paths = Vec::new();
    collect_recursive(root, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(&path).ok()?;
            if !has_supported_start_keyword(&source) {
                return None;
            }
            if source.lines().any(|l| l.trim().starts_with("@startditaa")) {
                return None;
            }
            let rel = path.strip_prefix(root).ok()?.with_extension("");
            let family = rel
                .components()
                .next()?
                .as_os_str()
                .to_string_lossy()
                .to_string();
            Some(CorpusFile {
                family,
                name: rel.to_string_lossy().to_string(),
                path,
                source,
            })
        })
        .collect()
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, out);
        } else if path.extension().is_some_and(|e| e == "puml")
            && path.with_extension("svg").exists()
        {
            out.push(path);
        }
    }
}

/// Parse exactly as the CLI does for a file argument with no `--block`
/// options (see `crates/rustuml/src/main.rs`).
pub fn parse_shipped(source: &str, base_dir: Option<&Path>) -> Result<Diagram, ParseError> {
    let blocks = rustuml_parser::parse::split_blocks(source);
    if blocks.is_empty() {
        rustuml_parser::parse::parse_auto_with_base(source, base_dir)
    } else {
        rustuml_parser::parse::parse_block(source, 0)
    }
}

/// Render exactly as the CLI does for `-tsvg` with no theme flags.
pub fn render_shipped(diagram: &Diagram) -> String {
    rustuml_render::render_svg_with_theme(diagram, &rustuml_render::style::Theme::default())
}

/// Wall time of one parse and one render of `file`, or the failure reason.
#[derive(Debug, Clone)]
pub struct FileTiming {
    pub parse: Duration,
    pub render: Duration,
    pub svg_bytes: usize,
}

/// Why a file is not on the measured path.
#[derive(Debug, Clone)]
pub enum Failure {
    Parse(String),
    Panic(String),
}

/// Run the shipped path once on `file`, catching panics.
pub fn time_file(file: &CorpusFile) -> Result<FileTiming, Failure> {
    let base_dir = file.path.parent();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let t0 = Instant::now();
        let diagram =
            parse_shipped(&file.source, base_dir).map_err(|e| Failure::Parse(e.to_string()))?;
        let parse = t0.elapsed();
        let t1 = Instant::now();
        let svg = render_shipped(&diagram);
        let render = t1.elapsed();
        Ok(FileTiming {
            parse,
            render,
            svg_bytes: svg.len(),
        })
    }));
    match outcome {
        Ok(r) => r,
        Err(panic) => Err(Failure::Panic(panic_message(&panic))),
    }
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "unknown panic".to_string()
    }
}

/// Upper bound on a single render for a file to be sampled. Files above
/// this are degenerate cases (the Graphviz timeout guard is 5 s) and would
/// dominate every iteration; the whole-corpus `run` still reports them.
pub const SAMPLE_RENDER_CAP: Duration = Duration::from_millis(1000);

/// Deterministic per-family sample: walk each family's sorted file list at
/// a fixed stride and, at each stride point, take the first file that
/// parses and renders cleanly within [`SAMPLE_RENDER_CAP`]. The rule is a
/// function of the corpus alone, so the sample cannot be hand-picked.
pub fn sample_per_family(corpus: &[CorpusFile], per_family: usize) -> Vec<CorpusFile> {
    let mut families: Vec<&str> = corpus.iter().map(|f| f.family.as_str()).collect();
    families.sort_unstable();
    families.dedup();

    let mut sample = Vec::new();
    for family in families {
        let files: Vec<&CorpusFile> = corpus.iter().filter(|f| f.family == family).collect();
        let stride = (files.len() / per_family).max(1);
        let mut taken = 0;
        let mut cursor = 0;
        while taken < per_family && cursor < files.len() {
            let mut probe = cursor;
            let mut picked = None;
            while probe < files.len() && probe < cursor + stride {
                let ok = matches!(time_file(files[probe]), Ok(t) if t.render <= SAMPLE_RENDER_CAP);
                if ok {
                    picked = Some(files[probe]);
                    break;
                }
                probe += 1;
            }
            if let Some(f) = picked {
                sample.push(f.clone());
                taken += 1;
            }
            cursor += stride;
        }
    }
    sample
}
