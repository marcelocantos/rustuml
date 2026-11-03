// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Shared primitives for the golden-pair test harnesses.
//!
//! Both parity tiers use these: the strict tier (`tests/golden_pairs.rs`,
//! which injects oracle layout extracted from the golden) and the no-oracle
//! product tier (`tests/golden_no_oracle.rs`, which renders through the same
//! path the CLI uses). Keeping the corpus walk, skip classification, and the
//! determinism pin in one place guarantees the two tiers agree on what a
//! testable golden is.

use std::path::{Path, PathBuf};

/// Root of the required golden corpus (git submodule).
pub fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-diagrams/golden")
}

pub const SUPPORTED_START_KEYWORDS: &[&str] = &[
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

pub fn has_supported_start_keyword(source: &str) -> bool {
    let first_line = source
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();
    // Accept files that start with a supported @start keyword, OR headerless
    // files (no @start prefix at all — the parser auto-detects the type).
    if !first_line.starts_with("@start") {
        return true;
    }
    SUPPORTED_START_KEYWORDS
        .iter()
        .any(|kw| first_line.starts_with(kw))
}

/// True when the golden SVG is a Java PlantUML error page rather than a
/// rendered diagram. Such pairs are skipped by both tiers: reproducing
/// Java's error screen byte-for-byte is not a parity goal.
///
/// HONESTY GUARD: `tests/harness_guards.rs` pins the number of goldens this
/// matches. Adding a marker string here reclassifies failures as skips —
/// that census must be updated deliberately, with user sign-off, never as a
/// side effect of chasing a failing golden.
pub fn golden_has_syntax_error(svg: &str) -> bool {
    svg.contains("Syntax Error")
        || svg.contains("NoSuchElementException")
        || svg.contains("Welcome to PlantUML")
        || svg.contains("An error has occured")
        || svg.contains("kill cannot be used here")
        || svg.contains("swimlane must be defined at the start")
        || svg.contains("Note already created:")
        || svg.contains("Parsing syntax error about %")
        || svg.contains("[From string")
        || svg.contains("Your data does not sound like YAML data")
        || svg.contains("does&#160;not&#160;sound&#160;like&#160;YAML")
        || svg.contains("Your data does not sound like JSON data")
        || svg.contains("does&#160;not&#160;sound&#160;like&#160;JSON")
        || svg.contains("No class ")
        || svg.contains("(Assumed diagram type:")
        || svg.contains("DITAA has crashed")
        || svg.contains("This feature has been suppressed")
}

/// Ditaa produces raster images, not SVG elements — skipped by both tiers.
pub fn is_ditaa(source: &str) -> bool {
    source.lines().any(|l| l.trim().starts_with("@startditaa"))
}

/// `%date(...)` goldens in the corpus were all generated on 2026-03-23
/// AEDT (UTC+11). The `edge_seq_title_with_variables` golden captures a
/// specific second within that day (`Mon Mar 23 07:13:46 AEDT 2026`,
/// epoch 1774210426s); the others (`yyyy-MM-dd` form) only depend on the
/// day, so this pin works for all three.
pub const GOLDEN_DEBUG: &str = "date=1774210426000,tz=AEDT+1100";

/// All .puml files under `root` that have a sibling .svg golden, sorted.
/// Missing, unreadable, or empty corpora are setup failures, never passing suites.
pub fn collect_golden_pairs(root: &Path) -> Vec<PathBuf> {
    let mut pairs = Vec::new();
    collect_recursive(root, &mut pairs);
    pairs.sort();
    assert!(
        !pairs.is_empty(),
        "required golden corpus {} contains no pairs; run: git submodule update --init test-diagrams/golden",
        root.display()
    );
    pairs
}

fn collect_recursive(dir: &Path, pairs: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|error| {
        panic!(
            "cannot read required golden corpus {}: {error}; run: git submodule update --init test-diagrams/golden",
            dir.display()
        )
    });
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("cannot read golden entry in {}: {error}", dir.display())
        });
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, pairs);
        } else if path.extension().is_some_and(|e| e == "puml")
            && path.with_extension("svg").exists()
        {
            pairs.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn make_gate_preserves_failure_and_runs_remaining_checks() {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        let root = std::env::temp_dir().join(format!(
            "rustuml-gate-negative-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let cargo = root.join("cargo");
        // Reproduce the original masking trigger: print a matching result line
        // but exit unsuccessfully. The other checks succeed independently.
        std::fs::write(
            &cargo,
            "#!/bin/sh\nprintf 'mock cargo %s\\n' \"$*\"\ncase \" $* \" in\n  *' --lib '*) echo 'test result: FAILED'; exit 101;;\nesac\nexit 0\n",
        )
        .unwrap();
        std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).unwrap();
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        let paths = std::iter::once(root.clone()).chain(std::env::split_paths(&inherited));
        let path = std::env::join_paths(paths).unwrap();
        let repo = golden_dir()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let run = |target| {
            Command::new("make")
                .arg(target)
                .current_dir(&repo)
                .env("PATH", &path)
                .env_remove("MAKEFLAGS")
                .env_remove("MFLAGS")
                .env_remove("GOLDEN_FILTER")
                .env_remove("HARNESS_WRITE_BASELINES")
                .output()
                .expect("run make to exercise exit propagation")
        };
        let unit = run("test");
        let gate = run("gate");
        // The ratchet enables writes on variable presence, even an empty value.
        // Reject both empty and nonempty overrides before invoking Cargo.
        for variable in ["GOLDEN_FILTER", "HARNESS_WRITE_BASELINES"] {
            for value in ["", "1"] {
                let guarded = Command::new("make")
                    .arg("test-parity")
                    .current_dir(&repo)
                    .env("PATH", &path)
                    .env_remove("MAKEFLAGS")
                    .env_remove("MFLAGS")
                    .env_remove("GOLDEN_FILTER")
                    .env_remove("HARNESS_WRITE_BASELINES")
                    .env(variable, value)
                    .output()
                    .expect("run parity gate with forbidden override");
                assert!(
                    !guarded.status.success(),
                    "gate accepted {variable}={value:?}"
                );
                assert!(
                    !String::from_utf8_lossy(&guarded.stdout).contains("mock cargo"),
                    "gate ran Cargo with {variable}={value:?}"
                );
            }
        }
        std::fs::remove_dir_all(&root).unwrap();
        let unit_stdout = String::from_utf8_lossy(&unit.stdout);
        assert!(!unit.status.success(), "make test masked a failing command");
        assert!(unit_stdout.contains("test result: FAILED"));
        assert!(!unit_stdout.contains("✓ tests"));
        assert!(!gate.status.success(), "make gate masked a failing check");
        let gate_stdout = String::from_utf8_lossy(&gate.stdout);
        for expected in [
            "mock cargo fmt",
            "mock cargo clippy",
            "mock cargo test --workspace --lib",
            "mock cargo test --workspace --release --no-fail-fast",
        ] {
            assert!(
                gate_stdout.contains(expected),
                "gate omitted {expected}: {gate_stdout}"
            );
        }
    }

    #[test]
    fn required_golden_corpus_fails_when_missing_or_empty() {
        let root = std::env::temp_dir().join(format!(
            "rustuml-corpus-negative-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!root.exists());
        let missing = std::panic::catch_unwind(|| collect_golden_pairs(&root));
        std::fs::create_dir(&root).unwrap();
        let empty = std::panic::catch_unwind(|| collect_golden_pairs(&root));
        // Control: the same collector accepts a populated corpus.
        std::fs::write(root.join("control.puml"), "@startuml\nA -> B\n@enduml\n").unwrap();
        std::fs::write(root.join("control.svg"), "<svg/>").unwrap();
        let pairs = collect_golden_pairs(&root);
        std::fs::remove_dir_all(&root).unwrap();
        assert!(missing.is_err(), "missing corpus must fail");
        assert!(empty.is_err(), "empty corpus must fail");
        assert_eq!(pairs.len(), 1);
    }
}
