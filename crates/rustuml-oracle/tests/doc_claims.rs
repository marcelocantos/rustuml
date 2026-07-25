// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Drift guards for T14.5 documentation claims.

use std::collections::BTreeMap;
use std::path::PathBuf;

const README_TABLE_START: &str = "<!-- no-oracle-status:start -->";
const README_TABLE_END: &str = "<!-- no-oracle-status:end -->";

struct Row {
    title: &'static str,
    tag: &'static str,
    family: Option<&'static str>,
    excluded: Option<&'static str>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_baseline() -> BTreeMap<String, (usize, usize)> {
    let text = std::fs::read_to_string(repo_root().join("test-diagrams/no_oracle_baseline.txt"))
        .expect("read no_oracle_baseline.txt");
    let mut counts = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(family), Some(pass), Some(eligible)) = (parts.next(), parts.next(), parts.next())
        else {
            panic!("malformed baseline line: {line}");
        };
        assert!(
            parts.next().is_none(),
            "unexpected trailing fields in baseline line: {line}"
        );
        counts.insert(
            family.to_string(),
            (
                pass.parse().expect("baseline pass count"),
                eligible.parse().expect("baseline eligible count"),
            ),
        );
    }
    counts
}

fn rows() -> &'static [Row] {
    &[
        Row {
            title: "Sequence",
            tag: "`@startuml`",
            family: Some("sequence"),
            excluded: None,
        },
        Row {
            title: "Class",
            tag: "`@startuml`",
            family: Some("class"),
            excluded: None,
        },
        Row {
            title: "Archimate",
            tag: "`@startuml`",
            family: Some("archimate"),
            excluded: None,
        },
        Row {
            title: "Activity (new syntax)",
            tag: "`@startuml`",
            family: Some("activity"),
            excluded: None,
        },
        Row {
            title: "State",
            tag: "`@startuml`",
            family: Some("state"),
            excluded: None,
        },
        Row {
            title: "Component",
            tag: "`@startuml`",
            family: Some("component"),
            excluded: None,
        },
        Row {
            title: "Deployment",
            tag: "`@startuml`",
            family: Some("deployment"),
            excluded: None,
        },
        Row {
            title: "Use Case",
            tag: "`@startuml`",
            family: Some("usecase"),
            excluded: None,
        },
        Row {
            title: "Object",
            tag: "`@startuml`",
            family: Some("object"),
            excluded: None,
        },
        Row {
            title: "Timing",
            tag: "`@startuml`",
            family: Some("timing"),
            excluded: None,
        },
        Row {
            title: "ER (crow's foot)",
            tag: "`@startuml`",
            family: Some("er"),
            excluded: None,
        },
        Row {
            title: "Gantt",
            tag: "`@startgantt`",
            family: Some("gantt"),
            excluded: None,
        },
        Row {
            title: "Mindmap",
            tag: "`@startmindmap`",
            family: Some("mindmap"),
            excluded: None,
        },
        Row {
            title: "WBS",
            tag: "`@startwbs`",
            family: Some("wbs"),
            excluded: None,
        },
        Row {
            title: "JSON/YAML",
            tag: "`@startjson` / `@startyaml`",
            family: Some("json-yaml"),
            excluded: None,
        },
        Row {
            title: "Salt (wireframes)",
            tag: "`@startsalt`",
            family: Some("salt"),
            excluded: None,
        },
        Row {
            title: "Network (nwdiag)",
            tag: "`@startnwdiag`",
            family: Some("nwdiag"),
            excluded: None,
        },
        Row {
            title: "Regex (railroad)",
            tag: "`@startregex`",
            family: Some("regex"),
            excluded: None,
        },
        Row {
            title: "EBNF",
            tag: "`@startebnf`",
            family: Some("ebnf"),
            excluded: None,
        },
        Row {
            title: "DOT",
            tag: "`@startdot`",
            family: Some("dot"),
            excluded: None,
        },
        Row {
            title: "Git",
            tag: "`@startgit`",
            family: Some("git"),
            excluded: None,
        },
        Row {
            title: "Board / Wire",
            tag: "`@startboard`",
            family: Some("wire"),
            excluded: None,
        },
        Row {
            title: "Ditaa (ASCII art)",
            tag: "`@startditaa`",
            family: None,
            excluded: Some("Excluded from SVG parity tiers; raster comparator pending"),
        },
        Row {
            title: "Math/LaTeX",
            tag: "`@startmath` / `@startlatex`",
            family: Some("math"),
            excluded: None,
        },
    ]
}

fn comma(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

fn status(pass: usize, eligible: usize) -> String {
    if eligible == 0 {
        return "0/0 (no eligible SVG goldens)".to_string();
    }
    let pct = pass as f64 * 100.0 / eligible as f64;
    let label = if pass == eligible {
        "exact"
    } else if pass == 0 {
        "none"
    } else {
        "partial"
    };
    format!("{}/{} ({pct:.1}%, {label})", comma(pass), comma(eligible))
}

fn expected_readme_table(baseline: &BTreeMap<String, (usize, usize)>) -> String {
    let mut out = String::new();
    out.push_str(README_TABLE_START);
    out.push('\n');
    out.push_str("| Type | Tag | Baseline family | No-oracle product status |\n");
    out.push_str("|------|-----|-----------------|--------------------------|\n");
    for row in rows() {
        let (family, status) = if let Some(family) = row.family {
            let (pass, eligible) = baseline
                .get(family)
                .copied()
                .unwrap_or_else(|| panic!("missing baseline family {family}"));
            (format!("`{family}`"), status(pass, eligible))
        } else {
            ("excluded".to_string(), row.excluded.unwrap().to_string())
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            row.title, row.tag, family, status
        ));
    }
    out.push_str(README_TABLE_END);
    out
}

fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let start_idx = text.find(start).expect("section start marker missing");
    let end_idx = text.find(end).expect("section end marker missing") + end.len();
    &text[start_idx..end_idx]
}

#[test]
fn readme_no_oracle_table_matches_baseline() {
    let baseline = read_baseline();
    let readme = std::fs::read_to_string(repo_root().join("README.md")).expect("read README");
    assert_eq!(
        section(&readme, README_TABLE_START, README_TABLE_END),
        expected_readme_table(&baseline)
    );
}

#[test]
fn docs_name_the_product_truth_metric_and_error_skips() {
    let baseline = read_baseline();
    let total_pass: usize = baseline.values().map(|(pass, _)| pass).sum();
    let total_eligible: usize = baseline.values().map(|(_, eligible)| eligible).sum();
    assert_eq!((total_pass, total_eligible), (7_374, 11_251));

    let readme = std::fs::read_to_string(repo_root().join("README.md")).expect("read README");
    let stability =
        std::fs::read_to_string(repo_root().join("STABILITY.md")).expect("read STABILITY");
    let help_agent = std::fs::read_to_string(repo_root().join("crates/rustuml/src/main.rs"))
        .expect("read CLI source");
    for (name, text) in [
        ("README.md", readme),
        ("STABILITY.md", stability),
        ("--help-agent", help_agent),
    ] {
        assert!(
            text.contains("no-oracle product tier"),
            "{name} must identify no-oracle as the product metric"
        );
        assert!(
            text.contains("7,374/11,251"),
            "{name} must carry the current no-oracle total from the baseline"
        );
        assert!(
            text.contains("11,130/11,251"),
            "{name} must carry the current strict-tier total"
        );
        assert!(
            text.contains("1,199 Java"),
            "{name} must describe Java error-page goldens as skips"
        );
    }
}
