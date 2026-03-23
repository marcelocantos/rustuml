// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Network diagram (nwdiag) parser.

use std::sync::LazyLock;

use regex::Regex;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::nwdiag::*;

/// Parse a quoted or unquoted attribute value after `=`.
fn parse_attr_value(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub fn parse_nwdiag(lines: &[String]) -> Result<NwdiagDiagram, ParseError> {
    let meta = DiagramMeta::default();
    let mut networks: Vec<Network> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();

    // Parsing states.
    #[derive(PartialEq)]
    enum State {
        Top,
        InNetwork(usize), // index into networks
        InGroup(usize),   // index into groups
    }

    let mut state = State::Top;
    // Brace depth within the top-level `nwdiag { ... }` block.
    let mut depth: usize = 0;

    static RE_NETWORK: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*network\s+(\w+)\s*\{?\s*$").unwrap());
    static RE_GROUP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*group(?:\s+(\w+))?\s*\{?\s*$").unwrap());
    static RE_ATTR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(\w+)\s*=\s*(.+?)\s*;?\s*$").unwrap());
    static RE_HOST: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(\w+)\s*(?:\[([^\]]*)\])?\s*;?\s*$").unwrap());
    static RE_HOST_ATTR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"address\s*=\s*"?([^",\]]+)"?"#).unwrap());
    static RE_HOST_DESC: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"description\s*=\s*"([^"]+)""#).unwrap());

    for (line_no, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip blank lines and @start/@end/@nwdiag markers.
        if trimmed.is_empty()
            || trimmed.starts_with("@start")
            || trimmed.starts_with("@end")
            || trimmed == "nwdiag {"
            || trimmed == "nwdiag{"
        {
            if trimmed == "nwdiag {" || trimmed == "nwdiag{" {
                depth = 1;
            }
            continue;
        }

        // Count brace depth for the outer nwdiag block.
        if depth == 0 {
            if trimmed.contains('{') {
                depth += trimmed.chars().filter(|&c| c == '{').count();
                depth -= trimmed.chars().filter(|&c| c == '}').count();
            }
            continue;
        }

        // Closing brace: pop state.
        if trimmed == "}" {
            match state {
                State::InNetwork(_) | State::InGroup(_) => {
                    state = State::Top;
                }
                State::Top => {
                    depth = depth.saturating_sub(1);
                }
            }
            continue;
        }

        // Network block start.
        if let Some(caps) = RE_NETWORK.captures(trimmed) {
            let name = caps[1].to_string();
            networks.push(Network {
                name,
                address: None,
                color: None,
                hosts: Vec::new(),
            });
            let idx = networks.len() - 1;
            state = State::InNetwork(idx);
            continue;
        }

        // Group block start.
        if let Some(caps) = RE_GROUP.captures(trimmed) {
            let name = caps.get(1).map(|m| m.as_str().to_string());
            groups.push(Group {
                name,
                color: None,
                hosts: Vec::new(),
            });
            let idx = groups.len() - 1;
            state = State::InGroup(idx);
            continue;
        }

        match &state {
            State::InNetwork(idx) => {
                let idx = *idx;
                // Attribute assignment inside a network block.
                if let Some(caps) = RE_ATTR.captures(trimmed) {
                    let key = caps[1].to_string();
                    let val = parse_attr_value(&caps[2]);
                    match key.as_str() {
                        "address" => networks[idx].address = Some(val),
                        "color" => networks[idx].color = Some(val),
                        _ => {}
                    }
                    continue;
                }
                // Host entry.
                if let Some(caps) = RE_HOST.captures(trimmed) {
                    let host_name = caps[1].to_string();
                    // Skip keywords that might be mistakenly matched.
                    if matches!(host_name.as_str(), "network" | "group" | "nwdiag") {
                        continue;
                    }
                    let attrs_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                    let address = RE_HOST_ATTR
                        .captures(attrs_str)
                        .map(|c| c[1].trim().to_string());
                    let description = RE_HOST_DESC
                        .captures(attrs_str)
                        .map(|c| c[1].trim().to_string());
                    networks[idx].hosts.push(NetworkHost {
                        name: host_name,
                        address,
                        description,
                    });
                    continue;
                }
            }
            State::InGroup(idx) => {
                let idx = *idx;
                // Attribute assignment inside a group block.
                if let Some(caps) = RE_ATTR.captures(trimmed) {
                    let key = caps[1].to_string();
                    let val = parse_attr_value(&caps[2]);
                    match key.as_str() {
                        "color" => groups[idx].color = Some(val),
                        _ => {}
                    }
                    continue;
                }
                // Host reference (bare name) in a group block.
                if let Some(caps) = RE_HOST.captures(trimmed) {
                    let host_name = caps[1].to_string();
                    if !matches!(host_name.as_str(), "network" | "group" | "nwdiag") {
                        groups[idx].hosts.push(host_name);
                    }
                    continue;
                }
            }
            State::Top => {
                // Ignore unrecognised top-level lines.
                let _ = line_no;
            }
        }
    }

    Ok(NwdiagDiagram {
        meta,
        networks,
        groups,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> NwdiagDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_nwdiag(&lines).unwrap()
    }

    #[test]
    fn basic_single_network() {
        let d = parse(
            "@startnwdiag\nnwdiag {\n  network internet {\n    web01\n    web02\n  }\n}\n@endnwdiag",
        );
        assert_eq!(d.networks.len(), 1);
        assert_eq!(d.networks[0].name, "internet");
        assert_eq!(d.networks[0].hosts.len(), 2);
    }

    #[test]
    fn network_with_addresses() {
        let d = parse(
            "@startnwdiag\nnwdiag {\n  network lan {\n    address = \"10.0.0.0/24\"\n    host1 [address = \"10.0.0.1\"]\n  }\n}\n@endnwdiag",
        );
        assert_eq!(d.networks[0].address.as_deref(), Some("10.0.0.0/24"));
        assert_eq!(d.networks[0].hosts[0].address.as_deref(), Some("10.0.0.1"));
    }

    #[test]
    fn groups_parsed() {
        let d = parse(
            "@startnwdiag\nnwdiag {\n  group web {\n    color = \"#FFD700\"\n    web01\n    web02\n  }\n  network net { web01\n web02\n }\n}\n@endnwdiag",
        );
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].color.as_deref(), Some("#FFD700"));
        assert_eq!(d.groups[0].hosts.len(), 2);
    }
}
