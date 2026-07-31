// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Closed schema-version-1 normalizer for T14 parity evidence records.

use rustuml_oracle::harness::golden_has_syntax_error;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

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
            if name.starts_with("account") && name.ends_with(".json") {
                if let Some(account) = normalize_account(&path, &dir, &mut report.violations) {
                    accounts_by_mechanism
                        .entry(account.mechanism.clone())
                        .or_default()
                        .push(account);
                }
            } else if name.starts_with("review") && name.ends_with(".json") {
                if let Some(review) = normalize_review(&path, &dir, &mut report.violations) {
                    reviews_by_mechanism
                        .entry(review.mechanism.clone())
                        .or_default()
                        .push(review);
                }
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
        let has_account = accounts_by_mechanism
            .get(mechanism)
            .is_some_and(|accounts| !accounts.is_empty());
        if latest_verdict == Verdict::Accepted && !has_account {
            report.violations.push(format!(
                "{}: accepted review has no normalized account for mechanism {mechanism}",
                latest_path.display()
            ));
        }
        let mut latest_can_select_positive_evidence =
            latest_verdict == Verdict::Accepted && has_account;
        if latest_verdict == Verdict::Accepted
            && !validate_accepting_review_attestation(
                &latest.object,
                &latest_path,
                &mut report.violations,
            )
        {
            latest_can_select_positive_evidence = false;
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
    require_commit_field(&object, "java_revision", path, violations);
    require_location_array(&object, "java_locations", path, violations);
    require_location_array(&object, "rust_locations", path, violations);
    require_one_string_field(&object, ACCOUNT_INVARIANT_FIELDS, path, violations);
    require_string_field(&object, "causal_divergence", path, violations);
    require_one_descriptive_field(&object, ACCOUNT_PLANNED_CHANGE_FIELDS, path, violations);
    require_string_array(&object, "perturbation_axes", 2, path, violations);

    if path.file_name().and_then(|value| value.to_str()) == Some("account.json") {
        let directory = dir.file_name().and_then(|value| value.to_str());
        if directory != Some(mechanism.as_str()) {
            violations.push(format!(
                "{}: canonical account mechanism must match directory name",
                path.display()
            ));
        }
    }

    Some(AccountRecord {
        mechanism,
        path: path.to_path_buf(),
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
    let sequence = review_sequence(path).unwrap_or_else(|| {
        violations.push(format!(
            "{}: unrecognized schema-version-1 review filename",
            path.display()
        ));
        usize::MAX
    });
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

fn validate_review_declared_evidence(
    root: &Path,
    review: &ReviewRecord,
    is_latest: bool,
    may_select_positive_evidence: bool,
    report: &mut EvidenceReport,
) {
    let mut valid_passing_sources = BTreeSet::new();
    let mut observed_axes = BTreeSet::new();
    let base_dir = evidence_base_dir(&review.object);

    for field in EVIDENCE_ARRAY_FIELDS {
        let Some(values) = review.object.get(*field).and_then(Value::as_array) else {
            continue;
        };
        for value in values {
            let Some(item) = value.as_object() else {
                continue;
            };
            validate_evidence_item_paths(
                root,
                review,
                item,
                base_dir.as_deref(),
                &mut report.violations,
            );

            if !(is_latest && review.verdict == Verdict::Accepted) {
                continue;
            }
            if !item_declares_positive_pass(item) {
                continue;
            }
            let Some(source) = source_path_from_item(item, base_dir.as_deref()) else {
                continue;
            };
            let Some(golden) = golden_path_from_item(root, item, &source, base_dir.as_deref())
            else {
                continue;
            };
            if !path_is_under_perturbations(&source) || !path_is_under_perturbations(&golden) {
                report.violations.push(format!(
                    "{}: accepted held-out paths must stay under test-diagrams/perturbations",
                    review.path.display()
                ));
                continue;
            }
            if source.extension().and_then(|value| value.to_str()) != Some("puml") {
                report.violations.push(format!(
                    "{}: accepted source must be a .puml: {}",
                    review.path.display(),
                    source.display()
                ));
                continue;
            }
            if golden.extension().and_then(|value| value.to_str()) != Some("svg") {
                report.violations.push(format!(
                    "{}: accepted golden must be an SVG: {}",
                    review.path.display(),
                    golden.display()
                ));
                continue;
            }
            let source_abs = root.join(&source);
            let golden_abs = root.join(&golden);
            if !source_abs.is_file() {
                report.violations.push(format!(
                    "{}: accepted source does not exist: {}",
                    review.path.display(),
                    source.display()
                ));
                continue;
            }
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
            if let Some(axes) = item.get("axes").and_then(Value::as_array) {
                observed_axes.extend(axes.iter().filter_map(Value::as_str).map(str::to_owned));
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

fn validate_evidence_item_paths(
    root: &Path,
    review: &ReviewRecord,
    item: &serde_json::Map<String, Value>,
    base_dir: Option<&str>,
    violations: &mut Vec<String>,
) {
    let Some(source) = source_path_from_item(item, base_dir) else {
        return;
    };
    if !path_is_under_perturbations(&source) {
        violations.push(format!(
            "{}: review-declared source must stay under test-diagrams/perturbations: {}",
            review.path.display(),
            source.display()
        ));
        return;
    }
    if !root.join(&source).is_file() {
        violations.push(format!(
            "{}: review-declared source does not exist: {}",
            review.path.display(),
            source.display()
        ));
        return;
    }
    if let Some(golden) = golden_path_from_item(root, item, &source, base_dir) {
        if !path_is_under_perturbations(&golden) {
            violations.push(format!(
                "{}: review-declared golden must stay under test-diagrams/perturbations: {}",
                review.path.display(),
                golden.display()
            ));
        } else if !root.join(&golden).is_file() {
            violations.push(format!(
                "{}: review-declared golden does not exist: {}",
                review.path.display(),
                golden.display()
            ));
        }
    } else if item.get("valid").and_then(Value::as_bool) != Some(false) {
        violations.push(format!(
            "{}: review-declared source has no readable Java SVG: {}",
            review.path.display(),
            source.display()
        ));
    }
}

fn item_declares_positive_pass(item: &serde_json::Map<String, Value>) -> bool {
    if item.get("valid").and_then(Value::as_bool) == Some(false) {
        return false;
    }
    let result_texts = result_texts(item);
    if result_texts.is_empty() {
        return false;
    }
    if result_texts
        .iter()
        .any(|text| text_disqualifies_positive_pass(text))
    {
        return false;
    }
    result_texts.iter().any(|text| text_is_positive_pass(text))
}

fn result_texts(item: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut texts = Vec::new();
    for field in RESULT_FIELDS {
        let Some(value) = item.get(*field) else {
            continue;
        };
        collect_result_texts(value, &mut texts);
    }
    if item
        .get("diff_one_no_oracle_status")
        .and_then(Value::as_i64)
        == Some(0)
    {
        texts.push("diff_one_no_oracle_status=0".to_owned());
    }
    texts
}

fn collect_result_texts(value: &Value, texts: &mut Vec<String>) {
    if let Some(text) = value.as_str() {
        texts.push(text.to_owned());
    } else if let Some(number) = value.as_i64() {
        texts.push(number.to_string());
    } else if let Some(boolean) = value.as_bool() {
        texts.push(boolean.to_string());
    } else if let Some(object) = value.as_object() {
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
                collect_result_texts(value, texts);
            }
        }
    }
}

fn text_disqualifies_positive_pass(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    lower.contains("error page")
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

fn source_path_from_item(
    item: &serde_json::Map<String, Value>,
    base_dir: Option<&str>,
) -> Option<PathBuf> {
    for field in SOURCE_FIELDS {
        let Some(text) = item.get(*field).and_then(Value::as_str) else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        let mut path = PathBuf::from(text);
        if path.components().count() == 1 {
            if let Some(base_dir) = base_dir {
                path = Path::new(base_dir).join(path);
            }
        }
        if path.extension().is_none() && matches!(*field, "input" | "stem") {
            path.set_extension("puml");
        }
        if path.extension().and_then(|value| value.to_str()) == Some("puml") {
            return Some(path);
        }
    }
    None
}

fn golden_path_from_item(
    root: &Path,
    item: &serde_json::Map<String, Value>,
    source: &Path,
    base_dir: Option<&str>,
) -> Option<PathBuf> {
    for field in GOLDEN_FIELDS {
        let Some(text) = item.get(*field).and_then(Value::as_str) else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        let mut path = PathBuf::from(text);
        if path.components().count() == 1 {
            if let Some(base_dir) = base_dir {
                path = Path::new(base_dir).join(path);
            }
        }
        if path.extension().and_then(|value| value.to_str()) == Some("svg") {
            return Some(path);
        }
    }

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

fn evidence_base_dir(object: &serde_json::Map<String, Value>) -> Option<String> {
    if let Some(text) = object.get("evidence_directory").and_then(Value::as_str) {
        return Some(text.to_owned());
    }
    if let Some(text) = object.get("perturbation_directory").and_then(Value::as_str) {
        return Some(text.to_owned());
    }
    if let Some(text) = object.get("inputs_root").and_then(Value::as_str) {
        return Some(text.to_owned());
    }
    if let Some(text) = object
        .get("preserved_evidence_root")
        .and_then(Value::as_str)
    {
        return Some(text.to_owned());
    }
    if let Some(text) = object
        .get("preserved_evidence_directory")
        .and_then(Value::as_str)
    {
        return Some(text.to_owned());
    }
    for (outer, inner) in [
        ("case_evidence", "root"),
        ("evidence_manifest", "directory"),
        ("artifacts", "root"),
    ] {
        if let Some(text) = object
            .get(outer)
            .and_then(Value::as_object)
            .and_then(|object| object.get(inner))
            .and_then(Value::as_str)
        {
            return Some(text.to_owned());
        }
    }
    None
}

fn review_sequence(path: &Path) -> Option<usize> {
    let name = path.file_name()?.to_str()?;
    if name == "review.json" || name == "review-1.json" {
        return Some(1);
    }
    let stem = name.strip_prefix("review-")?.strip_suffix(".json")?;
    if let Ok(number) = stem.parse() {
        return Some(number);
    }
    if stem == "followup" {
        return Some(2);
    }
    if let Some(number) = stem.strip_prefix("followup-") {
        return number.parse::<usize>().ok().map(|value| value + 1);
    }
    if let Some(number) = stem.strip_prefix("followup") {
        return number.parse::<usize>().ok().map(|value| value + 1);
    }
    if let Some((_, number)) = stem.rsplit_once('-') {
        if let Ok(number) = number.parse::<usize>() {
            return Some(number);
        }
    }
    Some(1)
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
) {
    let valid = object
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(is_commit);
    if !valid {
        violations.push(format!(
            "{}: {field} must be a 40-character Git commit",
            path.display()
        ));
    }
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
    object: &serde_json::Map<String, Value>,
    path: &Path,
    violations: &mut Vec<String>,
) -> bool {
    let start = violations.len();
    if !has_checker_identity(object) {
        violations.push(format!(
            "{}: accepted review must name the independent checker identity",
            path.display()
        ));
    }
    if !has_checker_model(object) {
        violations.push(format!(
            "{}: accepted review must name the checker model",
            path.display()
        ));
    }
    if !has_checker_tool(object) {
        violations.push(format!(
            "{}: accepted review must name the checker tool",
            path.display()
        ));
    }
    if !has_revision(object, RevisionKind::Rust) {
        violations.push(format!(
            "{}: accepted review must name an exact Rust revision",
            path.display()
        ));
    }
    if !has_revision(object, RevisionKind::Java) {
        violations.push(format!(
            "{}: accepted review must name an exact Java revision",
            path.display()
        ));
    }
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

fn has_checker_model(object: &serde_json::Map<String, Value>) -> bool {
    reviewer_objects(object).iter().any(|reviewer| {
        REVIEWER_MODEL_FIELDS.iter().any(|field| {
            reviewer
                .get(*field)
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
        }) || reviewer
            .get("identity")
            .and_then(Value::as_str)
            .is_some_and(identity_contains_model_alias)
    }) || object
        .get("identity")
        .and_then(Value::as_str)
        .is_some_and(identity_contains_model_alias)
}

fn identity_contains_model_alias(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("gpt-5") || lower.contains("codex")
}

fn has_checker_tool(object: &serde_json::Map<String, Value>) -> bool {
    reviewer_objects(object).iter().any(|reviewer| {
        REVIEWER_TOOL_FIELDS
            .iter()
            .any(|field| non_empty_string_or_array(reviewer.get(*field)))
            || reviewer
                .get("model_tool")
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
            || reviewer
                .get("model")
                .and_then(Value::as_str)
                .is_some_and(|text| {
                    text.contains(" / ") && text.to_ascii_lowercase().contains("codex desktop")
                })
    }) || [
        "helper",
        "rust_helper",
        "comparator",
        "diff_one",
        "java_jar",
    ]
    .iter()
    .any(|field| {
        object
            .get(*field)
            .is_some_and(descriptive_value_is_non_empty)
    }) || object
        .get("revisions")
        .and_then(Value::as_object)
        .is_some_and(|revisions| {
            [
                "helper",
                "rust_helper",
                "comparator",
                "diff_one",
                "java_jar",
                "plantuml_jar",
            ]
            .iter()
            .any(|field| {
                revisions
                    .get(*field)
                    .is_some_and(descriptive_value_is_non_empty)
            })
        })
        || commands_contain_tool_signal(object)
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

fn non_empty_string_or_array(value: Option<&Value>) -> bool {
    value.is_some_and(|value| {
        value.as_str().is_some_and(|text| !text.trim().is_empty())
            || value.as_array().is_some_and(|values| !values.is_empty())
    })
}

#[derive(Clone, Copy)]
enum RevisionKind {
    Rust,
    Java,
}

fn has_revision(object: &serde_json::Map<String, Value>, kind: RevisionKind) -> bool {
    value_contains_revision(&Value::Object(object.clone()), kind)
}

fn value_contains_revision(value: &Value, kind: RevisionKind) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            revision_key_matches(key, kind) && value_is_or_contains_commit(value)
                || matches!(value, Value::Object(_) | Value::Array(_))
                    && value_contains_revision(value, kind)
        }),
        Value::Array(values) => values
            .iter()
            .any(|value| value_contains_revision(value, kind)),
        _ => false,
    }
}

fn revision_key_matches(key: &str, kind: RevisionKind) -> bool {
    match kind {
        RevisionKind::Rust => matches!(
            key,
            "rust"
                | "rust_revision"
                | "combined_rust_revision"
                | "rust_review_revision"
                | "rust_review_head"
                | "rust_revision_checked"
                | "reviewed_rust"
                | "reviewed_working_head"
                | "head_at_review"
                | "implementation_commit"
                | "rust_implementation_commit"
                | "implementation_revision"
                | "rust_implementation_revision"
                | "resolved_implementation_revision"
                | "requested_implementation_revision"
                | "candidate_commit"
                | "review_revision"
        ),
        RevisionKind::Java => matches!(
            key,
            "java"
                | "java_revision"
                | "java_revision_checked"
                | "java_oracle_commit"
                | "java_oracle_revision"
                | "plantuml_revision"
                | "reviewed_java"
        ),
    }
}

fn value_is_or_contains_commit(value: &Value) -> bool {
    if value.as_str().is_some_and(is_commit) {
        return true;
    }
    match value {
        Value::Object(object) => object.values().any(value_is_or_contains_commit),
        Value::Array(values) => values.iter().any(value_is_or_contains_commit),
        _ => false,
    }
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

fn commands_contain_tool_signal(object: &serde_json::Map<String, Value>) -> bool {
    let Some(commands) = object.get("commands") else {
        return false;
    };
    command_value_contains_tool_signal(commands)
}

fn command_value_contains_tool_signal(value: &Value) -> bool {
    match value {
        Value::String(text) => {
            let lower = text.to_ascii_lowercase();
            ["codex", "diff_one", "cargo", "java", "xmllint", "git", "rg"]
                .iter()
                .any(|needle| lower.contains(needle))
        }
        Value::Array(values) => values.iter().any(command_value_contains_tool_signal),
        Value::Object(object) => object.values().any(command_value_contains_tool_signal),
        _ => false,
    }
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

fn path_is_under_perturbations(path: &Path) -> bool {
    is_normal_relative_path(path) && path.starts_with("test-diagrams/perturbations")
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

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rustuml-evidence-{name}-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("anon")
        ))
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
