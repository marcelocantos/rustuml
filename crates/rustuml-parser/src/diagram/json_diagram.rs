// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! JSON/YAML visualization diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

/// The data format the diagram was parsed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataFormat {
    Json,
    Yaml,
}

/// A node in the JSON/YAML data tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonNode {
    /// The key (field name for objects; `None` for array items and the root).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub value: JsonNodeValue,
    /// Whether this node is highlighted via a `#highlight` directive.
    pub highlighted: bool,
}

/// The value of a JSON/YAML node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum JsonNodeValue {
    Null,
    Bool { val: bool },
    Number { val: String },
    Str { val: String },
    Array { items: Vec<JsonNode> },
    Object { fields: Vec<JsonNode> },
}

/// The complete JSON/YAML visualization diagram.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonDiagram {
    pub meta: DiagramMeta,
    pub format: DataFormat,
    pub root: JsonNode,
}
