// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Component diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentDiagram {
    pub meta: DiagramMeta,
    pub components: Vec<Component>,
    pub interfaces: Vec<Interface>,
    pub connections: Vec<Connection>,
    pub packages: Vec<ComponentPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub dashed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPackage {
    pub name: String,
    pub label: String,
    pub components: Vec<String>,
    pub packages: Vec<ComponentPackage>,
}
