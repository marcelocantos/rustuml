// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Independent checker; compiled by checker.py against the selected tree.
use rustuml_oracle::{compare::compare_svg_strict, harness::golden_has_syntax_error};
use rustuml_render::filter_registry::{FilterRegistry, id_for_current, with_registry};
use serde_json::{Value, json};
use std::any::Any;
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind, panic_any};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

fn payload(p: &(dyn Any + Send)) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "non-string payload".into())
}

fn render(source: &str) -> Result<String, String> {
    let blocks = rustuml_parser::parse::split_blocks(source);
    let diagram = if blocks.len() > 1 {
        rustuml_parser::parse::parse_block(source, 0)
    } else {
        rustuml_parser::parse::parse_auto_with_base(source, None)
    }
    .map_err(|e| e.to_string())?;
    Ok(rustuml_render::render_svg(&diagram))
}

fn cli_render(cli: &Path, source: &str) -> (Option<String>, Value) {
    let mut child = Command::new(cli)
        .arg("-pipe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let svg = output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).unwrap());
    let report = json!({"exit_code": output.status.code(),
        "stderr": String::from_utf8_lossy(&output.stderr)});
    (svg, report)
}

fn compare(golden: &str, svg: &str) -> Value {
    match compare_svg_strict(golden, svg) {
        Ok(c) => json!({"status": if c.is_match() {"pass"} else {"mismatch"},
            "detail": c.to_string()}),
        Err(e) => json!({"status": "compare_error", "detail": e.to_string()}),
    }
}

fn sources(root: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(sources(&path));
        } else if path.extension().is_some_and(|e| e == "puml") {
            result.push(path);
        }
    }
    result.sort();
    result
}

fn corpus(root: &Path, cli: Option<&Path>) -> Value {
    let rows: Vec<_> = sources(root)
        .iter()
        .map(|path| {
            let source = std::fs::read_to_string(path).unwrap();
            let golden = std::fs::read_to_string(path.with_extension("svg")).unwrap();
            if golden_has_syntax_error(&golden) {
                return json!({"source": path, "in_process": {"status": "invalid_reference"}});
            }
            let result = catch_unwind(AssertUnwindSafe(|| render(&source)));
            let (svg, report) = match result {
                Ok(Ok(svg)) => {
                    let report = compare(&golden, &svg);
                    (Some(svg), report)
                }
                Ok(Err(e)) => (None, json!({"status": "parse_error", "detail": e})),
                Err(p) => (None, json!({"status": "panic", "detail": payload(&*p)})),
            };
            let mut row = json!({"source": path, "in_process": report});
            if let Some(cli) = cli {
                let (isolated, process) = cli_render(cli, &source);
                row["cli"] = match &isolated {
                    Some(s) => compare(&golden, s),
                    None => json!({"status": "process_failure"}),
                };
                row["process"] = process;
                row["byte_equal"] = json!(match (&svg, &isolated) {
                    (Some(a), Some(b)) => Some(a == b),
                    _ => None,
                });
            }
            row
        })
        .collect();
    json!(rows)
}

fn registry_checks() -> Value {
    let checks: [(&str, fn()); 4] = [
        ("normal_transfer", || {
            let seed = "checker ownership seed / red-blue";
            let mut expected = FilterRegistry::for_source(seed);
            let red = expected.id_for("red");
            let blue = expected.id_for("blue");
            let token = Arc::new(String::from("original body result"));
            let (returned, registry) = with_registry(seed, || {
                assert_eq!(id_for_current("#ff0000").as_deref(), Some(red.as_str()));
                assert_eq!(id_for_current("blue").as_deref(), Some(blue.as_str()));
                assert_eq!(id_for_current("RED").as_deref(), Some(red.as_str()));
                Arc::clone(&token)
            });
            assert!(Arc::ptr_eq(&token, &returned));
            assert_eq!(
                registry.render_defs_content(),
                expected.render_defs_content()
            );
            assert!(id_for_current("red").is_none());
            assert!(with_registry("empty", || ()).1.is_empty());
        }),
        ("body_panic_payload_identity", || {
            let token = Arc::new(String::from("checker non-string panic payload"));
            let panic = catch_unwind(|| {
                with_registry("body panic seed", || {
                    id_for_current("#CAFEBA").unwrap();
                    panic_any(Arc::clone(&token));
                })
            })
            .unwrap_err();
            assert!(Arc::ptr_eq(
                &token,
                panic.downcast_ref::<Arc<String>>().unwrap()
            ));
            assert!(id_for_current("#CAFEBA").is_none());
            let (_, empty) = with_registry("new scope", || ());
            assert!(empty.is_empty());
        }),
        ("caught_nested_rejection", || {
            let seed = "outer retained source";
            let mut expected = FilterRegistry::for_source(seed);
            let before = expected.id_for("coral");
            expected.id_for("navy");
            let (_, registry) = with_registry(seed, || {
                assert_eq!(id_for_current("coral"), Some(before.clone()));
                for _ in 0..3 {
                    let p = catch_unwind(|| {
                        with_registry("rejected different seed", || {
                            panic!("nested body must never execute");
                        })
                    })
                    .unwrap_err();
                    assert_eq!(
                        payload(&*p),
                        "filter_registry::with_registry is not re-entrant"
                    );
                    assert_eq!(id_for_current("coral"), Some(before.clone()));
                }
                id_for_current("navy").unwrap();
            });
            assert_eq!(
                registry.render_defs_content(),
                expected.render_defs_content()
            );
            assert!(id_for_current("coral").is_none());
        }),
        ("uncaught_nested_rejection", || {
            let p = catch_unwind(|| {
                with_registry("outer", || {
                    id_for_current("magenta").unwrap();
                    with_registry("inner", || panic!("nested body entered"));
                })
            })
            .unwrap_err();
            assert_eq!(
                payload(&*p),
                "filter_registry::with_registry is not re-entrant"
            );
            assert!(id_for_current("magenta").is_none());
            let mut expected = FilterRegistry::for_source("following");
            expected.id_for("lime");
            let (_, following) = with_registry("following", || id_for_current("lime"));
            assert_eq!(
                following.render_defs_content(),
                expected.render_defs_content()
            );
        }),
    ];
    json!(
        checks
            .into_iter()
            .map(|(name, check)| {
                // A failed baseline check must not poison a later independent check.
                let result = std::thread::spawn(move || catch_unwind(check))
                    .join()
                    .unwrap();
                match result {
                    Ok(()) => json!({"check": name, "pass": true}),
                    Err(p) => json!({"check": name, "pass": false, "failure": payload(&*p)}),
                }
            })
            .collect::<Vec<_>>()
    )
}

fn recovery(root: &Path, cli: &Path) -> Value {
    let controls: Vec<_> =
        sources(&root.join("test-diagrams/perturbations/render-registry-unwind-scope"))
            .into_iter()
            .map(|path| {
                let source = std::fs::read_to_string(&path).unwrap();
                let golden = std::fs::read_to_string(path.with_extension("svg")).unwrap();
                assert!(!golden_has_syntax_error(&golden));
                let (clean, process) = cli_render(cli, &source);
                let clean = clean.expect("fresh CLI control must succeed");
                (path, source, golden, clean, process)
            })
            .collect();
    let mut rows = Vec::new();
    for depth in [2, 3] {
        let failure = root.join(format!("test-diagrams/perturbations/sequence-grouped-note-origin/checker_live_stack_{depth}.puml"));
        let source = std::fs::read_to_string(&failure).unwrap();
        let golden = std::fs::read_to_string(failure.with_extension("svg")).unwrap();
        assert!(!golden_has_syntax_error(&golden));
        let (_, process) = cli_render(cli, &source);
        let controls = controls.clone();
        // One thread per initiating case; all recovery renders use that same thread.
        let row = std::thread::spawn(move || {
            let mut row = json!({"initiating_source": failure, "isolated_cli": process});
            let fresh = controls
                .iter()
                .map(|(_, source, _, clean, _)| {
                    assert_eq!(render(source).unwrap(), *clean);
                })
                .count();
            row["clean_checks"] = json!(fresh);
            let mut rounds = Vec::new();
            for round in 0..2 {
                let result = catch_unwind(AssertUnwindSafe(|| render(&source)));
                let panic = result.err().map(|p| payload(&*p));
                let mut cases = Vec::new();
                for (path, source, golden, clean, process) in &controls {
                    let outcome = catch_unwind(AssertUnwindSafe(|| render(source)));
                    cases.push(match outcome {
                        Ok(Ok(svg)) => {
                            json!({"source": path, "byte_equal_clean_cli": svg == *clean,
                            "java": compare(golden, &svg), "clean_cli": process,
                            "has_background_filters": svg.contains("<feFlood")})
                        }
                        Ok(Err(e)) => json!({"source": path, "error": e}),
                        Err(p) => json!({"source": path, "panic": payload(&*p)}),
                    });
                }
                rounds.push(json!({"round": round, "initiating_panic": panic, "controls": cases}));
            }
            row["rounds"] = json!(rounds);
            row
        })
        .join()
        .unwrap();
        rows.push(row);
    }
    let registry = registry_checks();
    let pass =
        registry
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["pass"] == true)
            && rows.iter().all(|r| {
                r["rounds"].as_array().unwrap().iter().all(|round| {
                    round["initiating_panic"]
                        .as_str()
                        .is_some_and(|p| p.starts_with("index out of bounds:"))
                        && round["controls"].as_array().unwrap().iter().all(|c| {
                            c["byte_equal_clean_cli"] == true && c["java"]["status"] == "pass"
                        })
                })
            });
    json!({"pass": pass, "registry_checks": registry, "recovery": rows})
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(&args[2]);
    let result = match args[1].as_str() {
        "corpus" => corpus(root, args.get(3).map(Path::new)),
        "recovery" => recovery(root, Path::new(&args[3])),
        _ => panic!("usage: checker corpus CORPUS [CLI] | recovery REPO CLI"),
    };
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    let pass = if args[1] == "recovery" {
        result["pass"] == true
    } else {
        result.as_array().unwrap().iter().all(|r| {
            r["in_process"]["status"] == "pass"
                && (args.len() < 4 || r["cli"]["status"] == "pass" && r["byte_equal"] == true)
        })
    };
    if !pass {
        std::process::exit(1);
    }
}
