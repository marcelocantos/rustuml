// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Honesty guards for the parity harness (🎯T14.1).
//!
//! These tests pin the mechanisms by which "parity" could be gamed rather
//! than earned. Each guard encodes a lesson from a real incident in this
//! repo's history (verbatim golden replay, May 2026; fixture-echo
//! `include_str!` guards, June 2026). Loosening any of them is a deliberate,
//! user-signed-off act — never a side effect of chasing a failing golden.

use rustuml_oracle::harness::{collect_golden_pairs, golden_dir, golden_has_syntax_error};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Shipping crates must not embed golden test data. `include_str!` /
/// `include_bytes!` of anything under test-diagrams/ compiles the expected
/// answer into the product — the June 2026 fixture-echo incident.
///
/// The allow-list below is a RATCHET inherited from that incident. 🎯T14.2
/// drives it to zero, after which this list must stay empty forever.
/// Provenance COMMENTS citing goldens are fine and encouraged; this guard
/// only matches the compile-time embedding vector.
#[test]
fn no_golden_data_embedded_in_shipping_crates() {
    // (crate-relative file, max allowed embedding lines)
    const ALLOWED: &[(&str, usize)] = &[];
    const SHIPPING_CRATES: &[&str] = &[
        "crates/rustuml",
        "crates/rustuml-parser",
        "crates/rustuml-render",
        "crates/rustuml-layout",
        "crates/rustuml-math",
    ];

    let root = repo_root();
    let mut violations = Vec::new();
    for krate in SHIPPING_CRATES {
        let mut files = Vec::new();
        rust_sources(&root.join(krate).join("src"), &mut files);
        for file in files {
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            // Non-comment lines referencing the corpus. Covers single-line
            // and multi-line include_str!/include_bytes! forms (the path
            // string always names test-diagrams); provenance comments are
            // exempt by the comment check.
            let count = text
                .lines()
                .filter(|l| l.contains("test-diagrams") && !l.trim_start().starts_with("//"))
                .count();
            if count == 0 {
                continue;
            }
            let rel = file
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            let allowed = ALLOWED
                .iter()
                .find(|(f, _)| *f == rel)
                .map(|(_, n)| *n)
                .unwrap_or(0);
            if count > allowed {
                violations.push(format!(
                    "{rel}: {count} golden references (allowed {allowed})"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "golden test data referenced from shipping crate source — this embeds \
         the expected answer in the product (see 🎯T14.2):\n  {}",
        violations.join("\n  ")
    );
}

/// The number of goldens classified as Java error pages (and therefore
/// skipped by both parity tiers) is pinned. If this fails, either the corpus
/// changed (update deliberately, citing the submodule bump) or an error
/// marker was added to `golden_has_syntax_error` — which reclassifies
/// failures as skips and requires explicit user sign-off.
#[test]
fn error_golden_census_is_frozen() {
    const EXPECTED_ERROR_GOLDENS: usize = 1199;

    let root = golden_dir();
    if !root.exists() || !root.join("sequence").exists() {
        eprintln!("golden submodule not populated — census skipped");
        return;
    }
    let census = collect_golden_pairs(&root)
        .iter()
        .filter(|p| {
            std::fs::read_to_string(p.with_extension("svg"))
                .is_ok_and(|svg| golden_has_syntax_error(&svg))
        })
        .count();
    assert_eq!(
        census, EXPECTED_ERROR_GOLDENS,
        "error-golden census changed ({EXPECTED_ERROR_GOLDENS} -> {census}). If the corpus \
         grew, update the constant citing the submodule bump. If a marker string was added \
         to golden_has_syntax_error, stop: that converts failures into skips."
    );
}

/// Count of high-precision decimal literals (3+ decimal places) per
/// rustuml-render source file, ratcheted against a checked-in baseline.
///
/// Goodhart guard (oracle-first rule 6): a new measured constant is only
/// legitimate when it traces to a structural counterpart in the Java
/// reference or an extracted metrics table, cited in an adjacent comment.
/// This guard can't read comments, so it makes growth VISIBLE: adding a
/// high-precision literal fails until the baseline is updated in the same
/// change, where the diff reviewer checks the provenance comment.
/// Decreases are progress; lock them in with HARNESS_WRITE_BASELINES=1.
#[test]
fn decimal_literal_ratchet() {
    let root = repo_root();
    let baseline_path = root.join("crates/rustuml-oracle/tests/decimal_literal_baseline.txt");

    let mut files = Vec::new();
    rust_sources(&root.join("crates/rustuml-render/src"), &mut files);
    files.sort();

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let n = count_high_precision_literals(&text);
        if n > 0 {
            let rel = file
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            counts.insert(rel, n);
        }
    }

    if std::env::var("HARNESS_WRITE_BASELINES").is_ok() {
        let mut out = String::from(
            "# High-precision (3+ dp) decimal-literal counts per rustuml-render source file.\n\
             # Ratchet: counts may only grow alongside a provenance comment citing the Java\n\
             # source or an extracted metrics table, reviewed in the same change.\n\
             # Regenerate: HARNESS_WRITE_BASELINES=1 cargo test --test harness_guards\n",
        );
        for (file, n) in &counts {
            out.push_str(&format!("{file} {n}\n"));
        }
        std::fs::write(&baseline_path, out).expect("write baseline");
        eprintln!("baseline written to {}", baseline_path.display());
        return;
    }

    let baseline_text = std::fs::read_to_string(&baseline_path).unwrap_or_else(|e| {
        panic!(
            "missing baseline {} ({e}); bootstrap with HARNESS_WRITE_BASELINES=1",
            baseline_path.display()
        )
    });
    let mut baseline: BTreeMap<String, usize> = BTreeMap::new();
    for line in baseline_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (file, n) = line.rsplit_once(' ').expect("malformed baseline line");
        baseline.insert(file.to_string(), n.parse().expect("malformed count"));
    }

    let mut violations = Vec::new();
    for (file, n) in &counts {
        let allowed = baseline.get(file).copied().unwrap_or(0);
        if *n > allowed {
            violations.push(format!("{file}: {allowed} -> {n}"));
        }
    }
    assert!(
        violations.is_empty(),
        "new high-precision decimal literals in rustuml-render — each needs a provenance \
         comment (Java source or metrics-table derivation) and a reviewed baseline update \
         (HARNESS_WRITE_BASELINES=1):\n  {}",
        violations.join("\n  ")
    );
}

/// Matches `<digit>.<digit>{3,}` — the signature of a measured/curve-fit
/// constant as opposed to a designed one (2.5, 10.0).
fn count_high_precision_literals(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'.'
            && i > 0
            && bytes[i - 1].is_ascii_digit()
            && bytes.len() - i > 3
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
        {
            count += 1;
            // Skip the rest of this number so one literal counts once.
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    count
}

/// The comparator's slack and the park list are frozen. Changing either
/// redefines what "parity" means — user sign-off territory.
#[test]
fn comparator_constants_frozen() {
    assert_eq!(
        rustuml_oracle::compare::GEOM_EPS,
        0.02,
        "GEOM_EPS changed — this redefines strict parity and needs user sign-off"
    );

    let parked = std::fs::read_to_string(repo_root().join("test-diagrams/ulp_parked.txt"))
        .unwrap_or_default();
    let entries: Vec<&str> = parked
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    assert!(
        entries.is_empty(),
        "ulp_parked.txt gained entries — parking excludes failures from the count and \
         needs user sign-off: {entries:?}"
    );
}

/// T14 parity reviews are durable, machine-auditable maker/checker records.
///
/// Git history establishes authorship and ordering. This guard establishes
/// that every checked-in record contains the evidence needed to audit those
/// claims and that all held-out perturbations are preserved as paired files.
#[test]
fn parity_review_records_are_complete() {
    let root = repo_root();
    let reviews_root = root.join("docs/parity-reviews");
    let mut violations = Vec::new();

    let Ok(entries) = std::fs::read_dir(&reviews_root) else {
        panic!("missing parity review directory {}", reviews_root.display());
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let mechanism = entry.file_name().to_string_lossy().to_string();
        let account_path = dir.join("account.json");
        let Some(account) = read_json_object(&account_path, &mut violations) else {
            continue;
        };
        require_u64(
            &account,
            "schema_version",
            1,
            &account_path,
            &mut violations,
        );
        require_string(&account, "mechanism", &account_path, &mut violations);
        require_commit(&account, "java_revision", &account_path, &mut violations);
        require_string_array(
            &account,
            "java_locations",
            1,
            &account_path,
            &mut violations,
        );
        require_string_array(
            &account,
            "rust_locations",
            1,
            &account_path,
            &mut violations,
        );
        require_string(&account, "invariant", &account_path, &mut violations);
        require_string(
            &account,
            "causal_divergence",
            &account_path,
            &mut violations,
        );
        require_string(&account, "planned_change", &account_path, &mut violations);
        require_string_array(
            &account,
            "perturbation_axes",
            2,
            &account_path,
            &mut violations,
        );
        if account.get("mechanism").and_then(Value::as_str) != Some(mechanism.as_str()) {
            violations.push(format!(
                "{}: mechanism must match directory name {mechanism:?}",
                account_path.display()
            ));
        }

        let mut review_paths: Vec<(usize, PathBuf)> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                review_number(&path).map(|number| (number, path))
            })
            .collect();
        review_paths.sort_by_key(|(number, _)| *number);
        if review_paths.is_empty() {
            violations.push(format!(
                "{}: account has no independent checker review",
                dir.display()
            ));
            continue;
        }
        if review_paths
            .windows(2)
            .any(|window| window[0].0 == window[1].0)
        {
            violations.push(format!(
                "{}: duplicate review sequence number",
                dir.display()
            ));
        }

        let latest_number = review_paths.last().unwrap().0;
        let mut referenced_sources = std::collections::BTreeSet::new();
        for (number, review_path) in review_paths {
            validate_review(
                &root,
                &mechanism,
                &review_path,
                number == latest_number,
                &mut referenced_sources,
                &mut violations,
            );
        }

        let perturbation_dir = root.join("test-diagrams/perturbations").join(&mechanism);
        let mut preserved_sources = Vec::new();
        files_with_extension(&perturbation_dir, "puml", &mut preserved_sources);
        let preserved_sources: std::collections::BTreeSet<PathBuf> =
            preserved_sources.into_iter().collect();
        for source in &preserved_sources {
            if !referenced_sources.contains(source) {
                violations.push(format!(
                    "{}: perturbation is not named in any checker review",
                    source.display()
                ));
            }
        }
        for source in &referenced_sources {
            if !preserved_sources.contains(source) {
                violations.push(format!(
                    "{}: checker source is outside the executable perturbation walk",
                    source.display()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "invalid T14 parity review evidence:\n  {}",
        violations.join("\n  ")
    );
}

fn review_number(path: &Path) -> Option<usize> {
    let name = path.file_name()?.to_str()?;
    if name == "review.json" {
        return Some(1);
    }
    name.strip_prefix("review-")?
        .strip_suffix(".json")?
        .parse()
        .ok()
}

fn files_with_extension(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files_with_extension(&path, extension, out);
        } else if path.extension().is_some_and(|value| value == extension) {
            out.push(path);
        }
    }
}

fn read_json_object(
    path: &Path,
    violations: &mut Vec<String>,
) -> Option<serde_json::Map<String, Value>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            violations.push(format!("{}: {error}", path.display()));
            return None;
        }
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            violations.push(format!("{}: invalid JSON: {error}", path.display()));
            return None;
        }
    };
    match value {
        Value::Object(object) => Some(object),
        _ => {
            violations.push(format!("{}: root must be an object", path.display()));
            None
        }
    }
}

fn require_u64(
    object: &serde_json::Map<String, Value>,
    field: &str,
    expected: u64,
    path: &Path,
    violations: &mut Vec<String>,
) {
    if object.get(field).and_then(Value::as_u64) != Some(expected) {
        violations.push(format!("{}: {field} must equal {expected}", path.display()));
    }
}

fn require_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
    path: &Path,
    violations: &mut Vec<String>,
) {
    if object
        .get(field)
        .and_then(Value::as_str)
        .is_none_or(|value| value.trim().is_empty())
    {
        violations.push(format!(
            "{}: {field} must be a non-empty string",
            path.display()
        ));
    }
}

fn require_commit(
    object: &serde_json::Map<String, Value>,
    field: &str,
    path: &Path,
    violations: &mut Vec<String>,
) {
    let valid = object
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()));
    if !valid {
        violations.push(format!(
            "{}: {field} must be a 40-character Git commit",
            path.display()
        ));
    }
}

fn require_string_array(
    object: &serde_json::Map<String, Value>,
    field: &str,
    minimum: usize,
    path: &Path,
    violations: &mut Vec<String>,
) {
    let valid = object
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|values| {
            values.len() >= minimum
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
        });
    if !valid {
        violations.push(format!(
            "{}: {field} must contain at least {minimum} non-empty strings",
            path.display()
        ));
    }
}

fn validate_review(
    root: &Path,
    mechanism: &str,
    path: &Path,
    is_latest: bool,
    referenced_sources: &mut std::collections::BTreeSet<PathBuf>,
    violations: &mut Vec<String>,
) {
    let Some(review) = read_json_object(path, violations) else {
        return;
    };
    require_u64(&review, "schema_version", 1, path, violations);
    require_string(&review, "mechanism", path, violations);
    require_commit(&review, "account_commit", path, violations);
    require_commit(&review, "implementation_commit", path, violations);
    require_commit(&review, "java_revision", path, violations);
    require_string_array(&review, "commands", 2, path, violations);
    require_string_array(&review, "findings", 1, path, violations);
    require_string(&review, "verdict", path, violations);
    let verdict = review.get("verdict").and_then(Value::as_str);
    if !matches!(verdict, Some("accepted" | "rejected")) {
        violations.push(format!(
            "{}: verdict must be \"accepted\" or \"rejected\"",
            path.display()
        ));
    }
    if is_latest && verdict != Some("accepted") {
        violations.push(format!(
            "{}: latest checker review must be accepted",
            path.display()
        ));
    }
    if review.get("mechanism").and_then(Value::as_str) != Some(mechanism) {
        violations.push(format!(
            "{}: mechanism must match directory name {mechanism:?}",
            path.display()
        ));
    }

    match review.get("reviewer").and_then(Value::as_object) {
        Some(reviewer) => {
            require_string(reviewer, "task_id", path, violations);
            require_string(reviewer, "model", path, violations);
        }
        None => violations.push(format!("{}: reviewer must be an object", path.display())),
    }

    let perturbations = review
        .get("perturbations")
        .and_then(Value::as_array)
        .filter(|values| values.len() >= 2);
    let Some(perturbations) = perturbations else {
        violations.push(format!(
            "{}: perturbations must contain at least two entries",
            path.display()
        ));
        return;
    };
    let mut observed_axes = std::collections::BTreeSet::new();
    let mut review_sources = std::collections::BTreeSet::new();
    for perturbation in perturbations {
        let Some(perturbation) = perturbation.as_object() else {
            violations.push(format!(
                "{}: each perturbation must be an object",
                path.display()
            ));
            continue;
        };
        for field in ["source", "golden", "result"] {
            require_string(perturbation, field, path, violations);
        }
        require_string_array(perturbation, "axes", 1, path, violations);
        if let Some(axes) = perturbation.get("axes").and_then(Value::as_array) {
            observed_axes.extend(axes.iter().filter_map(Value::as_str).map(str::to_owned));
        }
        let result = perturbation.get("result").and_then(Value::as_str);
        if !matches!(result, Some("pass" | "fail")) {
            violations.push(format!(
                "{}: perturbation result must be \"pass\" or \"fail\"",
                path.display()
            ));
        }
        if is_latest && result != Some("pass") {
            violations.push(format!(
                "{}: latest accepted review cannot contain failing perturbations",
                path.display()
            ));
        }

        let source = perturbation.get("source").and_then(Value::as_str);
        let golden = perturbation.get("golden").and_then(Value::as_str);
        let expected_prefix = format!("test-diagrams/perturbations/{mechanism}/");
        if let (Some(source), Some(golden)) = (source, golden) {
            let source_path = Path::new(source);
            let golden_path = Path::new(golden);
            let source_is_normal = is_normal_relative_path(source_path);
            let golden_is_normal = is_normal_relative_path(golden_path);
            if !source_is_normal
                || !source.starts_with(&expected_prefix)
                || source_path.extension().and_then(|value| value.to_str()) != Some("puml")
            {
                violations.push(format!(
                    "{}: source must be a .puml under {expected_prefix}",
                    path.display()
                ));
            }
            if !golden_is_normal
                || !golden.starts_with(&expected_prefix)
                || golden_path.extension().and_then(|value| value.to_str()) != Some("svg")
                || source_path.with_extension("svg") != golden_path
            {
                violations.push(format!(
                    "{}: golden must be the source's same-stem .svg under {expected_prefix}",
                    path.display()
                ));
            }
            for (field, relative) in [("source", source), ("golden", golden)] {
                let absolute = root.join(relative);
                if !absolute.is_file() {
                    violations.push(format!(
                        "{}: referenced {field} does not exist: {relative}",
                        path.display()
                    ));
                } else if is_normal_relative_path(Path::new(relative)) {
                    let mechanism_dir = root
                        .join("test-diagrams/perturbations")
                        .join(mechanism)
                        .canonicalize();
                    let canonical = absolute.canonicalize();
                    if !matches!(
                        (mechanism_dir, canonical),
                        (Ok(directory), Ok(file)) if file.starts_with(&directory)
                    ) {
                        violations.push(format!(
                            "{}: referenced {field} escapes the mechanism directory: {relative}",
                            path.display()
                        ));
                    }
                }
            }
            if source_is_normal {
                let absolute = root.join(source);
                referenced_sources.insert(absolute.clone());
                review_sources.insert(absolute);
            }
        }
    }
    if review_sources.len() < 2 {
        violations.push(format!(
            "{}: review must exercise at least two distinct source files",
            path.display()
        ));
    }
    if observed_axes.len() < 2 {
        violations.push(format!(
            "{}: perturbations must exercise at least two distinct axes",
            path.display()
        ));
    }
}

fn is_normal_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

#[test]
fn parity_review_paths_reject_traversal() {
    assert!(is_normal_relative_path(Path::new(
        "test-diagrams/perturbations/state-routing/case.puml"
    )));
    assert!(!is_normal_relative_path(Path::new(
        "test-diagrams/perturbations/state-routing/../../golden/case.puml"
    )));
    assert!(!is_normal_relative_path(Path::new(
        "/tmp/state-routing/case.puml"
    )));
}
