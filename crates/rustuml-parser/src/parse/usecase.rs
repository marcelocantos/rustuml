// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::usecase::*;

pub fn parse_usecase(lines: &[String]) -> Result<UseCaseDiagram, ParseError> {
    let mut actors = Vec::new();
    let mut use_cases = Vec::new();
    let mut connections = Vec::new();
    let mut packages: Vec<UseCasePackage> = Vec::new();
    let meta = DiagramMeta::default();
    let mut current_package: Option<usize> = None;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        static RE_ACTOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^actor\s+(\w+)").unwrap());
        static RE_UC: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#"^usecase\s+"([^"]+)"\s+as\s+(\w+)"#).unwrap());
        static RE_CONN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^(\w+)\s*([-.\|>]+)\s*(\w+)(?:\s*:\s*(.+))?$").unwrap());
        static RE_STEREO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<<(\w+)>>").unwrap());
        static RE_PKG: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^(?:rectangle|package)\s+(?:"([^"]+)"|(\w+))\s*\{"#).unwrap()
        });

        if trimmed == "}" {
            current_package = None;
            continue;
        }

        if let Some(caps) = RE_ACTOR.captures(trimmed) {
            let id = caps[1].to_string();
            if !actors.iter().any(|a: &Actor| a.id == id) {
                actors.push(Actor {
                    id: id.clone(),
                    label: id,
                });
            }
        } else if let Some(caps) = RE_UC.captures(trimmed) {
            let id = caps[2].to_string();
            let label = caps[1].to_string();
            if let Some(idx) = current_package {
                packages[idx].elements.push(id.clone());
            }
            use_cases.push(UseCase { id, label });
        } else if let Some(caps) = RE_CONN.captures(trimmed) {
            let from = caps[1].to_string();
            let to = caps[3].to_string();
            let label = caps.get(4).map(|m| m.as_str().trim().to_string());
            let stereotype = label
                .as_ref()
                .and_then(|l| RE_STEREO.captures(l).map(|c| c[1].to_string()));

            connections.push(UseCaseConnection {
                from,
                to,
                label,
                stereotype,
            });
        } else if let Some(caps) = RE_PKG.captures(trimmed) {
            let name = caps
                .get(1)
                .or(caps.get(2))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            current_package = Some(packages.len());
            packages.push(UseCasePackage {
                name,
                elements: Vec::new(),
            });
        }
    }

    Ok(UseCaseDiagram {
        meta,
        actors,
        use_cases,
        connections,
        packages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> UseCaseDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_usecase(&lines).unwrap()
    }

    #[test]
    fn basic_usecase() {
        let d = parse(
            "actor User\nusecase \"Login\" as UC1\nusecase \"Browse\" as UC2\nUser --> UC1\nUser --> UC2",
        );
        assert_eq!(d.actors.len(), 1);
        assert_eq!(d.use_cases.len(), 2);
        assert_eq!(d.connections.len(), 2);
    }

    #[test]
    fn with_stereotype() {
        let d = parse(
            "actor User\nusecase \"Login\" as UC1\nusecase \"Auth\" as UC2\nUC1 ..> UC2 : <<include>>",
        );
        assert_eq!(d.connections[0].stereotype.as_deref(), Some("include"));
    }

    #[test]
    fn with_package() {
        let d = parse("actor User\nrectangle System {\nusecase \"Login\" as UC1\n}\nUser --> UC1");
        assert_eq!(d.packages.len(), 1);
        assert_eq!(d.packages[0].name, "System");
    }
}
