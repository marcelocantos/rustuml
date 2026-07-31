// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Closed schema-version-1 normalizer for T14 parity evidence records.

use rustuml_oracle::harness::golden_has_syntax_error;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const ACCOUNT_INVARIANT_FIELDS: &[&str] = &["invariant", "claimed_invariant"];
const ACCOUNT_PLANNED_CHANGE_FIELDS: &[&str] = &["planned_change", "planned_model_change"];
const REVIEWER_FIELDS: &[&str] = &["reviewer", "checker"];
const REVIEWER_IDENTITY_FIELDS: &[&str] = &["identity", "task_id", "agent"];
const REVIEWER_MODEL_FIELDS: &[&str] = &["model", "model_tool"];
const REVIEWER_TOOL_FIELDS: &[&str] = &["tool", "tools", "tooling"];
const EVIDENCE_ARRAY_FIELDS: &[&str] = &[
    "perturbations",
    "generated_inputs",
    "inputs",
    "fresh_perturbations",
    "fresh_inputs",
    "fresh_held_outs",
    "valid_heldouts",
];
const SOURCE_FIELDS: &[&str] = &["source", "puml", "path", "input", "stem"];
const GOLDEN_FIELDS: &[&str] = &["golden", "java_svg", "oracle_svg", "java", "svg"];
const RESULT_FIELDS: &[&str] = &[
    "result",
    "semantic_result",
    "metadata_result",
    "comparison",
    "outcome",
    "observation",
    "stdout",
    "diff_one_status",
    "diff_one_no_oracle",
    "strict",
    "diff_one",
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AcceptedHeldOut {
    pub mechanism: String,
    pub review_path: PathBuf,
    pub source: PathBuf,
    pub golden: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Verdict {
    Accepted,
    Rejected,
}

#[derive(Debug)]
struct AccountRecord {
    mechanism: String,
    path: PathBuf,
    java_revision: String,
    object: serde_json::Map<String, Value>,
}

#[derive(Debug)]
struct ReviewRecord {
    mechanism: String,
    path: PathBuf,
    sequence: usize,
    verdict: Verdict,
    object: serde_json::Map<String, Value>,
}

#[derive(Default)]
struct EvidenceReport {
    accepted_heldouts: BTreeSet<AcceptedHeldOut>,
    violations: Vec<String>,
}

#[allow(dead_code)]
pub fn validate_records(root: &Path) -> Vec<String> {
    build_report(root).violations
}

#[allow(dead_code)]
pub fn accepted_java_success_heldouts(root: &Path) -> Result<Vec<AcceptedHeldOut>, Vec<String>> {
    let report = build_report(root);
    if report.violations.is_empty() {
        Ok(report.accepted_heldouts.into_iter().collect())
    } else {
        Err(report.violations)
    }
}

fn build_report(root: &Path) -> EvidenceReport {
    let mut report = EvidenceReport::default();
    let reviews_root = root.join("docs/parity-reviews");
    let Ok(entries) = std::fs::read_dir(&reviews_root) else {
        report.violations.push(format!(
            "missing parity review directory {}",
            reviews_root.display()
        ));
        return report;
    };

    let mut accounts_by_mechanism: BTreeMap<String, Vec<AccountRecord>> = BTreeMap::new();
    let mut reviews_by_mechanism: BTreeMap<String, Vec<ReviewRecord>> = BTreeMap::new();

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&dir) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if name == "account.json" {
                if let Some(account) = normalize_account(&path, &dir, &mut report.violations) {
                    accounts_by_mechanism
                        .entry(account.mechanism.clone())
                        .or_default()
                        .push(account);
                }
            } else if review_sequence(&path).is_some() {
                if let Some(review) = normalize_review(&path, &dir, &mut report.violations) {
                    reviews_by_mechanism
                        .entry(review.mechanism.clone())
                        .or_default()
                        .push(review);
                }
            } else if name.starts_with("review") && name.ends_with(".json") {
                validate_archival_review(&path, &mut report.violations);
            }
        }
    }

    for (mechanism, reviews) in &mut reviews_by_mechanism {
        reviews.sort_by_key(|review| (review.sequence, review.path.clone()));
        for window in reviews.windows(2) {
            if window[0].sequence == window[1].sequence {
                report.violations.push(format!(
                    "{} and {}: duplicate review sequence number {} for mechanism {mechanism}",
                    window[0].path.display(),
                    window[1].path.display(),
                    window[0].sequence
                ));
            }
        }

        let latest = reviews.last().expect("non-empty review list");
        let latest_path = latest.path.clone();
        let latest_verdict = latest.verdict;
        let account = accounts_by_mechanism
            .get(mechanism)
            .and_then(|accounts| (accounts.len() == 1).then(|| &accounts[0]));
        if account.is_none() {
            report.violations.push(format!(
                "{}: review has no single canonical directory-local account for mechanism {mechanism}",
                latest_path.display()
            ));
        }
        let mut latest_can_select_positive_evidence =
            latest_verdict == Verdict::Accepted && account.is_some();
        if latest_verdict == Verdict::Accepted {
            if let Some(account) = account {
                if !validate_accepting_review_attestation(
                    root,
                    account,
                    &latest.object,
                    &latest_path,
                    &mut report.violations,
                ) {
                    latest_can_select_positive_evidence = false;
                }
            } else {
                latest_can_select_positive_evidence = false;
            }
        }

        for review in reviews {
            validate_review_declared_evidence(
                root,
                review,
                review.path == latest_path,
                review.path == latest_path && latest_can_select_positive_evidence,
                &mut report,
            );
        }
    }

    for accounts in accounts_by_mechanism.values() {
        for account in accounts {
            if !reviews_by_mechanism.contains_key(&account.mechanism) {
                // A pre-implementation account is complete but non-accepting.
                continue;
            }
            if account.path.file_name().and_then(|value| value.to_str()) == Some("account.json") {
                let directory = account
                    .path
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|value| value.to_str());
                if directory != Some(account.mechanism.as_str()) {
                    report.violations.push(format!(
                        "{}: canonical account mechanism must match directory name",
                        account.path.display()
                    ));
                }
            }
        }
    }

    if !report.violations.is_empty() {
        report.accepted_heldouts.clear();
    }
    report
}

fn normalize_account(
    path: &Path,
    dir: &Path,
    violations: &mut Vec<String>,
) -> Option<AccountRecord> {
    let object = read_json_object(path, violations)?;
    require_schema_version(&object, path, violations);
    let mechanism = require_string_field(&object, "mechanism", path, violations)?;
    let java_revision = require_commit_field(&object, "java_revision", path, violations)?;
    require_location_array(&object, "java_locations", path, violations);
    require_location_array(&object, "rust_locations", path, violations);
    require_one_string_field(&object, ACCOUNT_INVARIANT_FIELDS, path, violations);
    require_string_field(&object, "causal_divergence", path, violations);
    require_one_descriptive_field(&object, ACCOUNT_PLANNED_CHANGE_FIELDS, path, violations);
    require_string_array(&object, "perturbation_axes", 2, path, violations);

    let directory = dir.file_name().and_then(|value| value.to_str());
    if directory != Some(mechanism.as_str()) {
        violations.push(format!(
            "{}: canonical account mechanism must match directory name",
            path.display()
        ));
        return None;
    }

    Some(AccountRecord {
        mechanism,
        path: path.to_path_buf(),
        java_revision,
        object,
    })
}

fn normalize_review(path: &Path, dir: &Path, violations: &mut Vec<String>) -> Option<ReviewRecord> {
    let object = read_json_object(path, violations)?;
    require_schema_version(&object, path, violations);
    let mechanism = require_string_field(&object, "mechanism", path, violations).or_else(|| {
        dir.file_name()
            .and_then(|value| value.to_str())
            .map(str::to_owned)
    })?;
    if dir.file_name().and_then(|value| value.to_str()) != Some(mechanism.as_str()) {
        violations.push(format!(
            "{}: canonical review mechanism must match directory name",
            path.display()
        ));
        return None;
    }
    let sequence = review_sequence(path).expect("caller admits only canonical review names");
    let verdict = match normalize_verdict(object.get("verdict")) {
        Ok(verdict) => verdict,
        Err(error) => {
            violations.push(format!("{}: {error}", path.display()));
            return None;
        }
    };
    require_reviewer(&object, path, violations);
    require_commands(&object, path, violations);

    Some(ReviewRecord {
        mechanism,
        path: path.to_path_buf(),
        sequence,
        verdict,
        object,
    })
}

fn validate_archival_review(path: &Path, violations: &mut Vec<String>) {
    let Some(object) = read_json_object(path, violations) else {
        return;
    };
    match normalize_verdict(object.get("verdict")) {
        Ok(Verdict::Rejected) => {}
        Ok(Verdict::Accepted) => violations.push(format!(
            "{}: noncanonical review filenames are archival and cannot accept a mechanism",
            path.display()
        )),
        Err(error) => violations.push(format!("{}: {error}", path.display())),
    }
}

fn validate_review_declared_evidence(
    root: &Path,
    review: &ReviewRecord,
    is_latest: bool,
    may_select_positive_evidence: bool,
    report: &mut EvidenceReport,
) {
    let mut valid_passing_sources = BTreeSet::new();
    let mut used_goldens = BTreeSet::new();
    let mut observed_axes = BTreeSet::new();
    let base_dir = match evidence_base_dir(&review.object) {
        Ok(base_dir) => base_dir,
        Err(error) => {
            report
                .violations
                .push(format!("{}: {error}", review.path.display()));
            None
        }
    };

    for field in EVIDENCE_ARRAY_FIELDS {
        let Some(value) = review.object.get(*field) else {
            continue;
        };
        let Some(values) = value.as_array() else {
            report.violations.push(format!(
                "{}: {field} must be an array",
                review.path.display()
            ));
            continue;
        };
        for value in values {
            if !(is_latest && review.verdict == Verdict::Accepted) {
                // Rejected and superseded evidence stays preserved and auditable,
                // but it cannot participate in acceptance selection.
                continue;
            }
            let Some(item) = value.as_object() else {
                report.violations.push(format!(
                    "{}: every {field} entry must be an object",
                    review.path.display()
                ));
                continue;
            };
            let resolved = match resolve_evidence_item(root, review, item, base_dir.as_deref()) {
                Ok(resolved) => resolved,
                Err(error) => {
                    report
                        .violations
                        .push(format!("{}: {error}", review.path.display()));
                    continue;
                }
            };

            let positive = match item_positive_pass(item) {
                Ok(positive) => positive,
                Err(error) => {
                    report
                        .violations
                        .push(format!("{}: {error}", review.path.display()));
                    continue;
                }
            };
            if !positive {
                continue;
            }
            let Some((source, golden)) = resolved else {
                report.violations.push(format!(
                    "{}: passing evidence must name a source and Java SVG",
                    review.path.display()
                ));
                continue;
            };
            let golden_abs = root.join(&golden);
            let Ok(golden_text) = std::fs::read_to_string(&golden_abs) else {
                report.violations.push(format!(
                    "{}: accepted golden does not exist or is unreadable: {}",
                    review.path.display(),
                    golden.display()
                ));
                continue;
            };
            if golden_has_syntax_error(&golden_text) {
                report.violations.push(format!(
                    "{}: accepted held-out is a Java error/invalid-control SVG: {}",
                    review.path.display(),
                    golden.display()
                ));
                continue;
            }
            valid_passing_sources.insert(source.clone());
            if !used_goldens.insert(golden.clone()) {
                report.violations.push(format!(
                    "{}: a Java SVG may not be reused by multiple held-outs: {}",
                    review.path.display(),
                    golden.display()
                ));
                continue;
            }
            match non_empty_string_set(item.get("axes")) {
                Ok(axes) => observed_axes.extend(axes),
                Err(error) => {
                    report
                        .violations
                        .push(format!("{}: axes {error}", review.path.display()));
                    continue;
                }
            }
            if may_select_positive_evidence {
                report.accepted_heldouts.insert(AcceptedHeldOut {
                    mechanism: review.mechanism.clone(),
                    review_path: review.path.clone(),
                    source,
                    golden,
                });
            }
        }
    }

    if is_latest && review.verdict == Verdict::Accepted {
        if valid_passing_sources.len() < 2 {
            report.violations.push(format!(
                "{}: accepted latest review must declare at least two valid Java-success passing sources",
                review.path.display()
            ));
        }
        if observed_axes.len() < 2 {
            report.violations.push(format!(
                "{}: accepted latest review must exercise at least two distinct axes",
                review.path.display()
            ));
        }
    }
}

fn resolve_evidence_item(
    root: &Path,
    review: &ReviewRecord,
    item: &serde_json::Map<String, Value>,
    base_dir: Option<&str>,
) -> Result<Option<(PathBuf, PathBuf)>, String> {
    let Some(source) = consensus_path(item, SOURCE_FIELDS, base_dir, true)? else {
        return Ok(None);
    };
    validate_mechanism_path(root, &review.mechanism, &source, "source", "puml")?;
    let golden = consensus_path(item, GOLDEN_FIELDS, base_dir, false)?
        .or_else(|| inferred_golden(root, &source));
    let Some(golden) = golden else {
        if item.get("valid").and_then(Value::as_bool) == Some(false) {
            return Ok(None);
        }
        return Err(format!(
            "review-declared source has no readable Java SVG: {}",
            source.display()
        ));
    };
    validate_mechanism_path(root, &review.mechanism, &golden, "golden", "svg")?;
    let same_stem = source.with_extension("svg");
    let java_stem = source.with_extension("java.svg");
    if golden != same_stem && golden != java_stem {
        return Err(format!(
            "Java SVG must share the source stem: {} versus {}",
            source.display(),
            golden.display()
        ));
    }
    Ok(Some((source, golden)))
}

fn item_declares_positive_pass(item: &serde_json::Map<String, Value>) -> bool {
    item_positive_pass(item).unwrap_or(false)
}

fn item_positive_pass(item: &serde_json::Map<String, Value>) -> Result<bool, String> {
    if item.get("valid").and_then(Value::as_bool) == Some(false) {
        return Ok(false);
    }
    let result_texts = result_texts(item)?;
    if result_texts.is_empty() {
        return Ok(false);
    }
    let has_positive = result_texts.iter().any(|text| text_is_positive_pass(text));
    let has_negative = result_texts
        .iter()
        .any(|text| text_disqualifies_positive_pass(text));
    if has_positive && has_negative {
        return Err("contradictory passing and failing result aliases".to_owned());
    }
    Ok(has_positive && !has_negative)
}

fn result_texts(item: &serde_json::Map<String, Value>) -> Result<Vec<String>, String> {
    let mut texts = Vec::new();
    for field in RESULT_FIELDS {
        let Some(value) = item.get(*field) else {
            continue;
        };
        collect_result_texts(value, &mut texts)
            .map_err(|()| format!("{field} has an unsupported result representation"))?;
    }
    if let Some(status) = item.get("diff_one_no_oracle_status") {
        let Some(status) = status.as_i64() else {
            return Err("diff_one_no_oracle_status must be an integer".to_owned());
        };
        texts.push(status.to_string());
    }
    Ok(texts)
}

fn collect_result_texts(value: &Value, texts: &mut Vec<String>) -> Result<(), ()> {
    if let Some(text) = value.as_str() {
        texts.push(text.to_owned());
    } else if let Some(number) = value.as_i64() {
        texts.push(number.to_string());
    } else if let Some(boolean) = value.as_bool() {
        texts.push(boolean.to_string());
    } else if let Some(values) = value.as_array() {
        for value in values {
            collect_result_texts(value, texts)?;
        }
    } else if let Some(object) = value.as_object() {
        let mut recognized = false;
        for field in [
            "result",
            "status",
            "strict",
            "semantic_result",
            "summary",
            "outcome",
            "observation",
            "stdout",
            "comparison",
            "diff_one_status",
            "exit_code",
        ] {
            if let Some(value) = object.get(field) {
                recognized = true;
                collect_result_texts(value, texts)?;
            }
        }
        if !recognized {
            return Err(());
        }
    } else {
        return Err(());
    }
    Ok(())
}

fn text_disqualifies_positive_pass(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    lower == "fail"
        || lower == "failed"
        || lower == "false"
        || lower == "reject"
        || lower == "rejected"
        || lower.parse::<i64>().is_ok_and(|status| status != 0)
        || lower.contains("diverg")
        || lower.contains("svgs differ")
        || lower.contains("svgs are different")
        || lower.contains("mismatch")
        || lower.contains("not equivalent")
        || lower.contains("not structurally equivalent")
        || lower.contains("error page")
        || lower.contains("java rejects")
        || lower.contains("java reports")
        || lower.contains("java svg contains")
        || lower.contains("syntax error")
        || lower.contains("no such color")
        || lower.contains("rust rejects")
        || lower.contains("unrelated_residue")
        || lower.contains("separate invalid")
        || lower.contains("invalid-control")
        || lower.contains("invalid control")
        || lower.contains("invalid mixed-family")
        || lower.contains("invalid combined-feature")
}

fn text_is_positive_pass(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    lower == "pass"
        || lower == "0"
        || lower == "diff_one_no_oracle_status=0"
        || lower == "structurally equivalent"
        || lower == "svgs are structurally equivalent"
        || lower == "svgs structurally equivalent"
        || lower == "strict structural match"
        || lower == "pass_mechanism_structure"
        || lower == "pass_with_declared_metadata_residue"
        || lower.starts_with("pass:")
        || lower.starts_with("pass;")
        || lower.starts_with("pass ")
        || lower.starts_with("agree accept;")
        || lower.starts_with("java and rust both emit ")
        || lower.starts_with("semantic pass:")
        || lower.starts_with("strict structural match;")
}

fn consensus_path(
    item: &serde_json::Map<String, Value>,
    fields: &[&str],
    base_dir: Option<&str>,
    infer_puml_extension: bool,
) -> Result<Option<PathBuf>, String> {
    let mut paths = BTreeSet::new();
    for field in fields {
        let Some(value) = item.get(*field) else {
            continue;
        };
        let Some(text) = value.as_str() else {
            return Err(format!("{field} path alias must be a string"));
        };
        if text.trim().is_empty() {
            return Err(format!("{field} path alias must not be blank"));
        }
        let mut path = PathBuf::from(text);
        if path.components().count() == 1 {
            if let Some(base_dir) = base_dir {
                path = Path::new(base_dir).join(path);
            }
        }
        if infer_puml_extension && path.extension().is_none() && matches!(*field, "input" | "stem")
        {
            path.set_extension("puml");
        }
        paths.insert(path);
    }
    if paths.len() > 1 {
        return Err(format!("conflicting path aliases: {paths:?}"));
    }
    Ok(paths.into_iter().next())
}

fn inferred_golden(root: &Path, source: &Path) -> Option<PathBuf> {
    let same_stem = source.with_extension("svg");
    if root.join(&same_stem).is_file() {
        return Some(same_stem);
    }
    let java_svg = source.with_extension("java.svg");
    if root.join(&java_svg).is_file() {
        return Some(java_svg);
    }

    None
}

fn evidence_base_dir(object: &serde_json::Map<String, Value>) -> Result<Option<String>, String> {
    let mut directories = BTreeSet::new();
    for field in [
        "evidence_directory",
        "perturbation_directory",
        "inputs_root",
        "preserved_evidence_root",
        "preserved_evidence_directory",
    ] {
        if let Some(value) = object.get(field) {
            let Some(text) = value.as_str() else {
                return Err(format!("{field} must be a string"));
            };
            if text.trim().is_empty() {
                return Err(format!("{field} must not be blank"));
            }
            directories.insert(text.to_owned());
        }
    }
    for (outer, inner) in [
        ("case_evidence", "root"),
        ("evidence_manifest", "directory"),
        ("artifacts", "root"),
    ] {
        if let Some(container) = object.get(outer) {
            let Some(container) = container.as_object() else {
                return Err(format!("{outer} must be an object"));
            };
            if let Some(value) = container.get(inner) {
                let Some(text) = value.as_str() else {
                    return Err(format!("{outer}.{inner} must be a string"));
                };
                if text.trim().is_empty() {
                    return Err(format!("{outer}.{inner} must not be blank"));
                }
                directories.insert(text.to_owned());
            }
        }
    }
    if directories.len() > 1 {
        return Err(format!(
            "conflicting evidence-directory aliases: {directories:?}"
        ));
    }
    Ok(directories.into_iter().next())
}

fn validate_mechanism_path(
    root: &Path,
    mechanism: &str,
    path: &Path,
    kind: &str,
    extension: &str,
) -> Result<(), String> {
    let relative_root = Path::new("test-diagrams/perturbations").join(mechanism);
    if !is_normal_relative_path(path) || !path.starts_with(&relative_root) {
        return Err(format!(
            "review-declared {kind} must stay under {}: {}",
            relative_root.display(),
            path.display()
        ));
    }
    if path.extension().and_then(|value| value.to_str()) != Some(extension) {
        return Err(format!(
            "review-declared {kind} must have .{extension} extension: {}",
            path.display()
        ));
    }
    let absolute = root.join(path);
    if std::fs::symlink_metadata(&absolute).is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(format!(
            "review-declared {kind} must not be a symlink: {}",
            path.display()
        ));
    }
    let perturbations_root = std::fs::canonicalize(root.join("test-diagrams/perturbations"))
        .map_err(|error| format!("cannot canonicalize perturbation root: {error}"))?;
    let canonical_root = std::fs::canonicalize(root.join(&relative_root)).map_err(|error| {
        format!(
            "cannot canonicalize mechanism perturbation directory {}: {error}",
            relative_root.display()
        )
    })?;
    if !canonical_root.starts_with(&perturbations_root) {
        return Err(format!(
            "mechanism perturbation directory escapes through a symlink: {}",
            relative_root.display()
        ));
    }
    let canonical = std::fs::canonicalize(&absolute).map_err(|error| {
        format!(
            "review-declared {kind} does not exist: {}: {error}",
            path.display()
        )
    })?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!(
            "review-declared {kind} escapes the canonical mechanism directory: {}",
            path.display()
        ));
    }
    Ok(())
}

fn non_empty_string_set(value: Option<&Value>) -> Result<BTreeSet<String>, String> {
    let Some(values) = value.and_then(Value::as_array) else {
        return Err("must be an array".to_owned());
    };
    let mut result = BTreeSet::new();
    for value in values {
        let Some(text) = value.as_str() else {
            return Err("must contain only strings".to_owned());
        };
        let text = text.trim();
        if text.is_empty() {
            return Err("must not contain blank strings".to_owned());
        }
        result.insert(text.to_owned());
    }
    if result.is_empty() {
        return Err("must contain at least one non-empty string".to_owned());
    }
    Ok(result)
}

fn review_sequence(path: &Path) -> Option<usize> {
    let name = path.file_name()?.to_str()?;
    if name == "review.json" || name == "review-1.json" {
        return Some(1);
    }
    let stem = name.strip_prefix("review-")?.strip_suffix(".json")?;
    if let Ok(number) = stem.parse::<usize>() {
        return (number > 0).then_some(number);
    }
    if stem == "followup" {
        return Some(2);
    }
    if let Some(number) = stem.strip_prefix("followup-") {
        return number
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .map(|value| value + 1);
    }
    if let Some(number) = stem.strip_prefix("followup") {
        return number
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .map(|value| value + 1);
    }
    None
}

fn normalize_verdict(value: Option<&Value>) -> Result<Verdict, String> {
    let Some(value) = value else {
        return Err("verdict must be present".to_owned());
    };
    let text = if let Some(text) = value.as_str() {
        text
    } else if let Some(object) = value.as_object() {
        object
            .get("status")
            .or_else(|| object.get("verdict"))
            .or_else(|| object.get("result"))
            .and_then(Value::as_str)
            .ok_or_else(|| "object verdict must contain status/verdict/result".to_owned())?
    } else {
        return Err("verdict must be a string or known object verdict".to_owned());
    };
    match text.trim().to_ascii_lowercase().as_str() {
        "accepted" | "accept" | "pass" => Ok(Verdict::Accepted),
        "rejected" | "reject" => Ok(Verdict::Rejected),
        other => Err(format!("unsupported schema-version-1 verdict {other:?}")),
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
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(object)) => Some(object),
        Ok(_) => {
            violations.push(format!("{}: root must be an object", path.display()));
            None
        }
        Err(error) => {
            violations.push(format!("{}: invalid JSON: {error}", path.display()));
            None
        }
    }
}

fn require_schema_version(
    object: &serde_json::Map<String, Value>,
    path: &Path,
    violations: &mut Vec<String>,
) {
    if object.get("schema_version").and_then(Value::as_u64) != Some(1) {
        violations.push(format!("{}: schema_version must equal 1", path.display()));
    }
}

fn require_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
    path: &Path,
    violations: &mut Vec<String>,
) -> Option<String> {
    let value = object.get(field).and_then(Value::as_str);
    if value.is_none_or(|value| value.trim().is_empty()) {
        violations.push(format!(
            "{}: {field} must be a non-empty string",
            path.display()
        ));
        return None;
    }
    value.map(str::to_owned)
}

fn require_one_string_field(
    object: &serde_json::Map<String, Value>,
    fields: &[&str],
    path: &Path,
    violations: &mut Vec<String>,
) {
    if fields.iter().any(|field| {
        object
            .get(*field)
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty())
    }) {
        return;
    }
    violations.push(format!(
        "{}: one of {fields:?} must be a non-empty string",
        path.display()
    ));
}

fn require_one_descriptive_field(
    object: &serde_json::Map<String, Value>,
    fields: &[&str],
    path: &Path,
    violations: &mut Vec<String>,
) {
    if fields.iter().any(|field| {
        object
            .get(*field)
            .is_some_and(descriptive_value_is_non_empty)
    }) {
        return;
    }
    violations.push(format!(
        "{}: one of {fields:?} must be a non-empty string/list/object",
        path.display()
    ));
}

fn descriptive_value_is_non_empty(value: &Value) -> bool {
    value.as_str().is_some_and(|text| !text.trim().is_empty())
        || value.as_array().is_some_and(|values| !values.is_empty())
        || value.as_object().is_some_and(|object| !object.is_empty())
}

fn require_commit_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
    path: &Path,
    violations: &mut Vec<String>,
) -> Option<String> {
    let value = object.get(field).and_then(Value::as_str);
    if !value.is_some_and(is_commit) {
        violations.push(format!(
            "{}: {field} must be a 40-character Git commit",
            path.display()
        ));
        return None;
    }
    value.map(str::to_owned)
}

fn is_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn require_location_array(
    object: &serde_json::Map<String, Value>,
    field: &str,
    path: &Path,
    violations: &mut Vec<String>,
) {
    let valid = object
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|values| !values.is_empty() && values.iter().all(valid_location));
    if !valid {
        violations.push(format!(
            "{}: {field} must contain at least one string or structured location",
            path.display()
        ));
    }
}

fn valid_location(value: &Value) -> bool {
    if value.as_str().is_some_and(|text| !text.trim().is_empty()) {
        return true;
    }
    let Some(object) = value.as_object() else {
        return false;
    };
    object
        .get("path")
        .and_then(Value::as_str)
        .is_some_and(|text| !text.trim().is_empty())
        || object
            .get("symbol")
            .and_then(Value::as_str)
            .is_some_and(|text| !text.trim().is_empty())
        || object
            .get("symbols")
            .and_then(Value::as_array)
            .is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
            })
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

fn require_reviewer(
    object: &serde_json::Map<String, Value>,
    path: &Path,
    violations: &mut Vec<String>,
) {
    if has_historical_reviewer_marker(object) {
        return;
    }
    violations.push(format!(
        "{}: reviewer/checker identity must use a known schema-version-1 identity field",
        path.display()
    ));
}

fn has_historical_reviewer_marker(object: &serde_json::Map<String, Value>) -> bool {
    has_checker_identity(object)
        || reviewer_objects(object).iter().any(|reviewer| {
            ["role", "model", "model_tool", "tool", "tools", "tooling"]
                .iter()
                .any(|field| {
                    reviewer
                        .get(*field)
                        .is_some_and(descriptive_value_is_non_empty)
                })
        })
}

fn validate_accepting_review_attestation(
    root: &Path,
    account: &AccountRecord,
    object: &serde_json::Map<String, Value>,
    path: &Path,
    violations: &mut Vec<String>,
) -> bool {
    let start = violations.len();
    validate_explicit_checker(object, path, violations);
    validate_attested_revisions(root, account, object, path, violations);
    require_commands(object, path, violations);
    if !has_review_inputs(object) {
        violations.push(format!(
            "{}: accepted review must preserve generated inputs",
            path.display()
        ));
    }
    if !has_findings(object) {
        violations.push(format!(
            "{}: accepted review must record findings",
            path.display()
        ));
    }
    violations.len() == start
}

fn validate_explicit_checker(
    object: &serde_json::Map<String, Value>,
    path: &Path,
    violations: &mut Vec<String>,
) {
    let reviewers: Vec<_> = REVIEWER_FIELDS
        .iter()
        .filter_map(|field| object.get(*field).and_then(Value::as_object))
        .collect();
    if reviewers.len() != 1 {
        violations.push(format!(
            "{}: accepted review must contain exactly one explicit reviewer/checker object",
            path.display()
        ));
        return;
    }
    let reviewer = reviewers[0];
    for (label, fields) in [
        ("identity", REVIEWER_IDENTITY_FIELDS),
        ("model", REVIEWER_MODEL_FIELDS),
        ("tool", REVIEWER_TOOL_FIELDS),
    ] {
        if !fields.iter().any(|field| {
            reviewer
                .get(*field)
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
        }) {
            violations.push(format!(
                "{}: accepted review must explicitly name checker {label} in the same reviewer/checker object",
                path.display()
            ));
        }
    }
    let independence = ["role", "independence"].iter().any(|field| {
        reviewer
            .get(*field)
            .and_then(Value::as_str)
            .is_some_and(|text| {
                let lower = text.to_ascii_lowercase();
                lower.contains("independent")
                    || lower.contains("did not implement")
                    || lower.contains("not the implementer")
                    || lower.contains("checker")
            })
    }) || ["implemented_change", "implemented_mechanism"]
        .iter()
        .any(|field| reviewer.get(*field).and_then(Value::as_bool) == Some(false));
    if !independence {
        violations.push(format!(
            "{}: accepted review must explicitly attest checker independence",
            path.display()
        ));
    }
}

fn validate_attested_revisions(
    root: &Path,
    account: &AccountRecord,
    object: &serde_json::Map<String, Value>,
    path: &Path,
    violations: &mut Vec<String>,
) {
    let Some(revisions) = object.get("revisions").and_then(Value::as_object) else {
        violations.push(format!(
            "{}: accepted review must contain an explicit revisions object",
            path.display()
        ));
        return;
    };
    let account_commit = consensus_commit(
        revisions,
        &[
            "account_commit",
            "account_revision_checked",
            "rust_account_commit",
        ],
        "account revision",
        path,
        violations,
    );
    let implementation_commit = consensus_commit(
        revisions,
        &[
            "implementation_commit",
            "rust_implementation_commit",
            "implementation_revision",
            "rust_implementation_revision",
            "requested_implementation_revision",
        ],
        "Rust implementation revision",
        path,
        violations,
    );
    let java_revision = consensus_commit(
        revisions,
        &[
            "java_revision",
            "java_revision_checked",
            "java_oracle_commit",
            "java_oracle_revision",
        ],
        "Java revision",
        path,
        violations,
    );

    let (Some(account_commit), Some(implementation_commit), Some(java_revision)) =
        (account_commit, implementation_commit, java_revision)
    else {
        return;
    };
    if java_revision != account.java_revision {
        violations.push(format!(
            "{}: reviewed Java revision {java_revision} does not match account Java revision {}",
            path.display(),
            account.java_revision
        ));
    }
    if account_commit == implementation_commit {
        violations.push(format!(
            "{}: account and implementation revisions must be distinct commits",
            path.display()
        ));
    }
    for (label, revision) in [
        ("account", account_commit.as_str()),
        ("implementation", implementation_commit.as_str()),
    ] {
        if !git_commit_exists(root, revision) {
            violations.push(format!(
                "{}: attested {label} revision does not exist in this repository: {revision}",
                path.display()
            ));
        }
    }
    if git_commit_exists(root, &account_commit) {
        validate_committed_account(root, account, &account_commit, path, violations);
    }
    if git_commit_exists(root, &account_commit)
        && git_commit_exists(root, &implementation_commit)
        && !git_is_ancestor(root, &account_commit, &implementation_commit)
    {
        violations.push(format!(
            "{}: account revision must precede the implementation revision",
            path.display()
        ));
    }
}

fn consensus_commit(
    object: &serde_json::Map<String, Value>,
    fields: &[&str],
    label: &str,
    path: &Path,
    violations: &mut Vec<String>,
) -> Option<String> {
    let mut commits = BTreeSet::new();
    for field in fields {
        let Some(value) = object.get(*field) else {
            continue;
        };
        let Some(commit) = value.as_str() else {
            violations.push(format!(
                "{}: {label} field {field} must be a commit string",
                path.display()
            ));
            continue;
        };
        if !is_commit(commit) {
            violations.push(format!(
                "{}: {label} field {field} must be a 40-character Git commit",
                path.display()
            ));
            continue;
        }
        commits.insert(commit.to_ascii_lowercase());
    }
    if commits.is_empty() {
        violations.push(format!(
            "{}: accepted review must explicitly name {label}",
            path.display()
        ));
        return None;
    }
    if commits.len() > 1 {
        violations.push(format!(
            "{}: conflicting {label} aliases: {commits:?}",
            path.display()
        ));
        return None;
    }
    commits.into_iter().next()
}

fn git_commit_exists(root: &Path, revision: &str) -> bool {
    Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["cat-file", "-e", &format!("{revision}^{{commit}}")])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn git_is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> bool {
    Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn validate_committed_account(
    root: &Path,
    account: &AccountRecord,
    revision: &str,
    review_path: &Path,
    violations: &mut Vec<String>,
) {
    let Ok(relative) = account.path.strip_prefix(root) else {
        violations.push(format!(
            "{}: account path is outside repository root",
            review_path.display()
        ));
        return;
    };
    let spec = format!("{revision}:{}", relative.to_string_lossy());
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["show", &spec])
        .output();
    let Ok(output) = output else {
        violations.push(format!(
            "{}: cannot inspect account at attested revision",
            review_path.display()
        ));
        return;
    };
    let committed = serde_json::from_slice::<Value>(&output.stdout).ok();
    if !output.status.success()
        || committed.as_ref().and_then(Value::as_object) != Some(&account.object)
    {
        violations.push(format!(
            "{}: current account does not match the account committed at {revision}",
            review_path.display()
        ));
    }
}

fn has_checker_identity(object: &serde_json::Map<String, Value>) -> bool {
    reviewer_objects(object).iter().any(|reviewer| {
        REVIEWER_IDENTITY_FIELDS.iter().any(|field| {
            reviewer
                .get(*field)
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
        })
    }) || object
        .get("identity")
        .and_then(Value::as_str)
        .is_some_and(|text| !text.trim().is_empty())
}

fn reviewer_objects<'a>(
    object: &'a serde_json::Map<String, Value>,
) -> Vec<&'a serde_json::Map<String, Value>> {
    let mut reviewers = Vec::new();
    for field in REVIEWER_FIELDS {
        if let Some(reviewer) = object.get(*field).and_then(Value::as_object) {
            reviewers.push(reviewer);
        }
    }
    if let Some(reviewer) = object.get("checker_identity").and_then(Value::as_object) {
        reviewers.push(reviewer);
    }
    if let Some(reviewer) = object.get("identity").and_then(Value::as_object) {
        reviewers.push(reviewer);
    }
    reviewers
}

fn has_review_inputs(object: &serde_json::Map<String, Value>) -> bool {
    EVIDENCE_ARRAY_FIELDS.iter().any(|field| {
        object
            .get(*field)
            .and_then(Value::as_array)
            .is_some_and(|values| !values.is_empty())
    })
}

fn has_findings(object: &serde_json::Map<String, Value>) -> bool {
    [
        "findings",
        "source_inspection",
        "source_audit",
        "source_review",
        "mechanism_assessment",
        "implementation_audit",
        "verification",
        "results",
        "conclusion",
        "residue",
    ]
    .iter()
    .any(|field| {
        object
            .get(*field)
            .is_some_and(descriptive_value_is_non_empty)
    })
}

fn require_commands(
    object: &serde_json::Map<String, Value>,
    path: &Path,
    violations: &mut Vec<String>,
) {
    let valid = match object.get("commands") {
        Some(Value::Array(values)) => values.iter().any(|value| match value {
            Value::String(text) => !text.trim().is_empty(),
            Value::Object(object) => !object.is_empty(),
            _ => false,
        }),
        Some(Value::Object(object)) => !object.is_empty(),
        _ => false,
    };
    if !valid {
        violations.push(format!(
            "{}: commands must be a non-empty known schema-version-1 list/object",
            path.display()
        ));
    }
}

pub fn is_normal_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn verdict_normalizer_is_closed() {
        assert_eq!(
            normalize_verdict(Some(&json!("ACCEPT"))).unwrap(),
            Verdict::Accepted
        );
        assert_eq!(
            normalize_verdict(Some(&json!({"status": "reject"}))).unwrap(),
            Verdict::Rejected
        );
        assert_eq!(
            normalize_verdict(Some(&json!("PASS"))).unwrap(),
            Verdict::Accepted
        );
        assert!(normalize_verdict(Some(&json!("maybe"))).is_err());
        assert!(normalize_verdict(Some(&json!("accepted_later"))).is_err());
        assert!(normalize_verdict(Some(&json!("rejected_with_counterexamples"))).is_err());
        assert!(normalize_verdict(Some(&json!("failed"))).is_err());
    }

    #[test]
    fn account_aliases_accept_structured_locations() {
        let object = json!({
            "schema_version": 1,
            "mechanism": "m",
            "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
            "java_locations": [{"path": "A.java", "symbol": "A#m"}],
            "rust_locations": [{"path": "a.rs", "symbols": ["render"]}],
            "claimed_invariant": "model invariant",
            "causal_divergence": "old model differs",
            "planned_model_change": "new model",
            "perturbation_axes": ["labels", "topology"]
        })
        .as_object()
        .unwrap()
        .clone();
        let mut violations = Vec::new();
        require_schema_version(&object, Path::new("account.json"), &mut violations);
        require_location_array(
            &object,
            "java_locations",
            Path::new("account.json"),
            &mut violations,
        );
        require_location_array(
            &object,
            "rust_locations",
            Path::new("account.json"),
            &mut violations,
        );
        require_one_string_field(
            &object,
            ACCOUNT_INVARIANT_FIELDS,
            Path::new("account.json"),
            &mut violations,
        );
        require_one_descriptive_field(
            &object,
            ACCOUNT_PLANNED_CHANGE_FIELDS,
            Path::new("account.json"),
            &mut violations,
        );
        assert!(violations.is_empty(), "{violations:?}");
    }

    #[test]
    fn positive_heldout_requires_explicit_known_pass_signal() {
        let missing_result = json!({
            "source": "test-diagrams/perturbations/m/case.puml",
            "golden": "test-diagrams/perturbations/m/case.svg"
        })
        .as_object()
        .unwrap()
        .clone();
        assert!(!item_declares_positive_pass(&missing_result));

        let explicit_pass = json!({
            "source": "test-diagrams/perturbations/m/case.puml",
            "golden": "test-diagrams/perturbations/m/case.svg",
            "result": "pass: SVGs are structurally equivalent"
        })
        .as_object()
        .unwrap()
        .clone();
        assert!(item_declares_positive_pass(&explicit_pass));

        let invalid_control = json!({
            "source": "test-diagrams/perturbations/m/control.puml",
            "golden": "test-diagrams/perturbations/m/control.svg",
            "result": "pass invalid-control acceptance"
        })
        .as_object()
        .unwrap()
        .clone();
        assert!(!item_declares_positive_pass(&invalid_control));
    }

    #[test]
    fn java_error_svg_is_not_a_positive_heldout() {
        let root = std::env::temp_dir().join(format!(
            "rustuml-evidence-test-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("anon")
        ));
        let dir = root.join("test-diagrams/perturbations/m");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("case.puml"), "@startuml\nA --> B\n@enduml\n").unwrap();
        std::fs::write(dir.join("case.svg"), "<svg>Syntax Error</svg>").unwrap();

        let item = json!({
            "source": "test-diagrams/perturbations/m/case.puml",
            "golden": "test-diagrams/perturbations/m/case.svg",
            "axes": ["label", "topology"],
            "result": "pass"
        })
        .as_object()
        .unwrap()
        .clone();
        let review = ReviewRecord {
            mechanism: "m".to_owned(),
            path: root.join("docs/parity-reviews/m/review.json"),
            sequence: 1,
            verdict: Verdict::Accepted,
            object: json!({"perturbations": [item]})
                .as_object()
                .unwrap()
                .clone(),
        };
        let mut report = EvidenceReport::default();
        validate_review_declared_evidence(&root, &review, true, true, &mut report);
        assert!(
            report
                .violations
                .iter()
                .any(|violation| violation.contains("Java error/invalid-control")),
            "{:?}",
            report.violations
        );
    }

    #[test]
    fn malformed_accepted_review_does_not_select_positive_evidence() {
        let root = temp_root("malformed-accepted");
        write_account(&root, "m");
        write_source_and_svg(&root, "m", "case_a", "<svg><text>A</text></svg>");
        write_source_and_svg(&root, "m", "case_b", "<svg><text>B</text></svg>");
        write_review(
            &root,
            "m",
            json!({
                "schema_version": 1,
                "mechanism": "m",
                "reviewer": {"model": "GPT-5"},
                "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
                "commands": ["diff_one --no-oracle"],
                "perturbations": [
                    heldout("m", "case_a", ["label", "topology"]),
                    heldout("m", "case_b", ["label", "direction"])
                ],
                "findings": ["model-only checker must not pass"],
                "verdict": "ACCEPT"
            }),
        );

        let report = build_report(&root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(
            report
                .violations
                .iter()
                .any(|violation| violation.contains("reviewer/checker identity"))
                || report
                    .violations
                    .iter()
                    .any(|violation| violation.contains("checker identity")),
            "{:?}",
            report.violations
        );
    }

    #[test]
    fn rejected_latest_review_is_non_accepting_even_with_pass_results() {
        let root = temp_root("rejected-latest");
        write_account(&root, "m");
        write_source_and_svg(&root, "m", "case_a", "<svg><text>A</text></svg>");
        write_source_and_svg(&root, "m", "case_b", "<svg><text>B</text></svg>");
        write_review(
            &root,
            "m",
            json!({
                "schema_version": 1,
                "mechanism": "m",
                "reviewer": {"identity": "checker", "model": "GPT-5", "tool": "Codex desktop"},
                "rust_revision": "1111111111111111111111111111111111111111",
                "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
                "commands": ["diff_one --no-oracle"],
                "perturbations": [
                    heldout("m", "case_a", ["label", "topology"]),
                    heldout("m", "case_b", ["label", "direction"])
                ],
                "findings": ["rejecting counterexample remains"],
                "verdict": "REJECT"
            }),
        );

        let report = build_report(&root);
        assert!(report.violations.is_empty(), "{:?}", report.violations);
        assert!(report.accepted_heldouts.is_empty());
    }

    #[test]
    fn pending_account_without_reviews_is_complete_but_non_accepting() {
        let root = temp_root("pending-account");
        write_account(&root, "m");

        let report = build_report(&root);
        assert!(report.violations.is_empty(), "{:?}", report.violations);
        assert!(report.accepted_heldouts.is_empty());
    }

    #[test]
    fn canonical_accepted_review_binds_checker_revisions_and_evidence() {
        let fixture = accepted_fixture("canonical-accepted");

        let report = build_report(&fixture.root);
        assert!(report.violations.is_empty(), "{:?}", report.violations);
        assert_eq!(report.accepted_heldouts.len(), 2);
    }

    #[test]
    fn inferred_checker_metadata_and_fabricated_revisions_are_rejected() {
        let fixture = accepted_fixture("fabricated-attestation");
        mutate_review(&fixture.root, |review| {
            review.remove("reviewer");
            review.insert("identity".to_owned(), json!("Codex checker"));
            review.insert("commands".to_owned(), json!(["cargo test"]));
            review.insert(
                "revisions".to_owned(),
                json!({
                    "account_commit": "0000000000000000000000000000000000000000",
                    "implementation_commit": "ffffffffffffffffffffffffffffffffffffffff",
                    "java_revision": JAVA_REVISION
                }),
            );
        });

        let report = build_report(&fixture.root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(contains_violation(
            &report,
            "explicit reviewer/checker object"
        ));
        assert!(contains_violation(
            &report,
            "does not exist in this repository"
        ));
    }

    #[test]
    fn revisions_hidden_in_evidence_do_not_attest_the_review() {
        let fixture = accepted_fixture("nested-revisions");
        mutate_review(&fixture.root, |review| {
            let revisions = review.remove("revisions").unwrap();
            perturbations_mut(review)[0]["hidden_revisions"] = revisions;
        });

        let report = build_report(&fixture.root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(contains_violation(&report, "explicit revisions object"));
    }

    #[test]
    fn contradictory_result_aliases_and_malformed_items_fail_closed() {
        let fixture = accepted_fixture("contradictory-results");
        mutate_review(&fixture.root, |review| {
            let perturbations = perturbations_mut(review);
            perturbations[0]["outcome"] = json!("fail: SVGs diverge");
            perturbations.push(json!(7));
        });

        let report = build_report(&fixture.root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(contains_violation(
            &report,
            "contradictory passing and failing"
        ));
        assert!(contains_violation(&report, "entry must be an object"));
    }

    #[test]
    fn blank_axes_do_not_count_as_perturbation_dimensions() {
        let fixture = accepted_fixture("blank-axes");
        mutate_review(&fixture.root, |review| {
            let perturbations = perturbations_mut(review);
            perturbations[0]["axes"] = json!(["", " "]);
            perturbations[1]["axes"] = json!([" "]);
        });

        let report = build_report(&fixture.root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(contains_violation(
            &report,
            "must not contain blank strings"
        ));
        assert!(contains_violation(&report, "at least two distinct axes"));
    }

    #[test]
    fn open_ended_review_filenames_cannot_enter_latest_selection() {
        let fixture = accepted_fixture("open-review-name");
        let directory = fixture.root.join("docs/parity-reviews/m");
        std::fs::rename(
            directory.join("review.json"),
            directory.join("review-attacker-99.json"),
        )
        .unwrap();

        let report = build_report(&fixture.root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(contains_violation(&report, "noncanonical review filenames"));
    }

    #[test]
    fn cross_directory_accounts_cannot_attest_a_review() {
        let fixture = accepted_fixture("cross-directory-account");
        let account = fixture.root.join("docs/parity-reviews/m/account.json");
        let donor = fixture.root.join("docs/parity-reviews/donor");
        std::fs::create_dir_all(&donor).unwrap();
        std::fs::rename(account, donor.join("account-attacker.json")).unwrap();

        let report = build_report(&fixture.root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(contains_violation(
            &report,
            "no single canonical directory-local account"
        ));
    }

    #[test]
    fn cross_mechanism_evidence_and_non_same_stem_goldens_are_rejected() {
        let fixture = accepted_fixture("evidence-ownership");
        write_source_and_svg(&fixture.root, "donor", "donated", "<svg/>");
        mutate_review(&fixture.root, |review| {
            let perturbations = perturbations_mut(review);
            perturbations[0]["source"] = json!("test-diagrams/perturbations/donor/donated.puml");
            perturbations[0]["golden"] = json!("test-diagrams/perturbations/donor/donated.svg");
            perturbations[1]["golden"] = json!("test-diagrams/perturbations/m/case_a.svg");
        });

        let report = build_report(&fixture.root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(contains_violation(
            &report,
            "must stay under test-diagrams/perturbations/m"
        ));
        assert!(contains_violation(&report, "must share the source stem"));
    }

    #[test]
    fn conflicting_path_aliases_fail_even_when_the_first_alias_is_valid() {
        let fixture = accepted_fixture("conflicting-aliases");
        mutate_review(&fixture.root, |review| {
            perturbations_mut(review)[0]["puml"] = json!("../../outside.puml");
        });

        let report = build_report(&fixture.root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(contains_violation(&report, "conflicting path aliases"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escapes_fail_canonical_containment() {
        use std::os::unix::fs::symlink;

        let fixture = accepted_fixture("symlink-escape");
        let source = fixture
            .root
            .join("test-diagrams/perturbations/m/case_a.puml");
        let outside = fixture.root.join("outside.puml");
        std::fs::write(&outside, "@startuml\nX --> Y\n@enduml\n").unwrap();
        std::fs::remove_file(&source).unwrap();
        symlink(&outside, &source).unwrap();

        let report = build_report(&fixture.root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(contains_violation(&report, "must not be a symlink"));
    }

    #[test]
    fn rejected_reviews_still_require_a_preimplementation_account() {
        let fixture = accepted_fixture("rejected-without-account");
        mutate_review(&fixture.root, |review| {
            review.insert("verdict".to_owned(), json!("REJECT"));
        });
        std::fs::remove_file(fixture.root.join("docs/parity-reviews/m/account.json")).unwrap();

        let report = build_report(&fixture.root);
        assert!(report.accepted_heldouts.is_empty());
        assert!(contains_violation(
            &report,
            "no single canonical directory-local account"
        ));
    }

    #[test]
    fn named_rejected_archives_remain_preserved_but_inert() {
        let root = temp_root("rejected-archive");
        let dir = root.join("docs/parity-reviews/m");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("review-counterexample.json"),
            serde_json::to_vec_pretty(&json!({"verdict": "REJECT"})).unwrap(),
        )
        .unwrap();

        let report = build_report(&root);
        assert!(report.violations.is_empty(), "{:?}", report.violations);
        assert!(report.accepted_heldouts.is_empty());
    }

    fn temp_root(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rustuml-evidence-{name}-{}-{}-{nonce}",
            std::process::id(),
            std::thread::current().name().unwrap_or("anon")
        ))
    }

    const JAVA_REVISION: &str = "71806a23780b04a5ccde2f8ceb5121edad5eb711";

    struct AcceptedFixture {
        root: PathBuf,
    }

    fn accepted_fixture(name: &str) -> AcceptedFixture {
        let root = temp_root(name);
        run_git(&root, &["init", "-q"]);
        run_git(&root, &["config", "user.name", "Parity Gate Test"]);
        run_git(
            &root,
            &["config", "user.email", "parity-gate@example.invalid"],
        );
        write_account(&root, "m");
        write_source_and_svg(&root, "m", "case_a", "<svg><text>A</text></svg>");
        write_source_and_svg(&root, "m", "case_b", "<svg><text>B</text></svg>");
        run_git(&root, &["add", "."]);
        run_git(&root, &["commit", "-q", "-m", "account"]);
        let account_commit = git_stdout(&root, &["rev-parse", "HEAD"]);

        std::fs::write(root.join("implementation.txt"), "model implementation\n").unwrap();
        run_git(&root, &["add", "implementation.txt"]);
        run_git(&root, &["commit", "-q", "-m", "implementation"]);
        let implementation_commit = git_stdout(&root, &["rev-parse", "HEAD"]);

        write_review(
            &root,
            "m",
            json!({
                "schema_version": 1,
                "mechanism": "m",
                "reviewer": {
                    "identity": "independent checker task",
                    "model": "GPT-5.6",
                    "tool": "Codex desktop",
                    "role": "independent checker; did not implement the change"
                },
                "revisions": {
                    "account_commit": account_commit,
                    "implementation_commit": implementation_commit,
                    "java_revision": JAVA_REVISION
                },
                "commands": ["diff_one --no-oracle"],
                "perturbations": [
                    heldout("m", "case_a", ["label", "topology"]),
                    heldout("m", "case_b", ["label", "direction"])
                ],
                "findings": ["fresh counterexamples did not disprove the mechanism"],
                "verdict": "ACCEPT"
            }),
        );
        AcceptedFixture { root }
    }

    fn mutate_review(root: &Path, mutate: impl FnOnce(&mut serde_json::Map<String, Value>)) {
        let path = root.join("docs/parity-reviews/m/review.json");
        let mut value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        mutate(value.as_object_mut().unwrap());
        std::fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }

    fn perturbations_mut(review: &mut serde_json::Map<String, Value>) -> &mut Vec<Value> {
        review
            .get_mut("perturbations")
            .and_then(Value::as_array_mut)
            .unwrap()
    }

    fn contains_violation(report: &EvidenceReport, needle: &str) -> bool {
        report
            .violations
            .iter()
            .any(|violation| violation.contains(needle))
    }

    fn run_git(root: &Path, args: &[&str]) {
        std::fs::create_dir_all(root).unwrap();
        let output = Command::new("git")
            .args(["-C"])
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_stdout(root: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(["-C"])
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn write_account(root: &Path, mechanism: &str) {
        let dir = root.join("docs/parity-reviews").join(mechanism);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("account.json"),
            serde_json::to_string_pretty(&json!({
                "schema_version": 1,
                "mechanism": mechanism,
                "java_revision": "71806a23780b04a5ccde2f8ceb5121edad5eb711",
                "java_locations": ["A.java"],
                "rust_locations": ["a.rs"],
                "claimed_invariant": "invariant",
                "causal_divergence": "divergence",
                "planned_model_change": "change",
                "perturbation_axes": ["label", "topology"]
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn write_review(root: &Path, mechanism: &str, value: Value) {
        let dir = root.join("docs/parity-reviews").join(mechanism);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("review.json"),
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();
    }

    fn write_source_and_svg(root: &Path, mechanism: &str, stem: &str, svg: &str) {
        let dir = root.join("test-diagrams/perturbations").join(mechanism);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{stem}.puml")),
            "@startuml\nA --> B\n@enduml\n",
        )
        .unwrap();
        std::fs::write(dir.join(format!("{stem}.svg")), svg).unwrap();
    }

    fn heldout<const N: usize>(mechanism: &str, stem: &str, axes: [&str; N]) -> Value {
        json!({
            "source": format!("test-diagrams/perturbations/{mechanism}/{stem}.puml"),
            "golden": format!("test-diagrams/perturbations/{mechanism}/{stem}.svg"),
            "axes": axes.to_vec(),
            "result": "pass"
        })
    }
}
