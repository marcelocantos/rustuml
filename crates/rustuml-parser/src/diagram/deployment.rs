// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Deployment diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentDiagram {
    pub meta: DiagramMeta,
    pub nodes: Vec<DeploymentNode>,
    pub connections: Vec<DeploymentConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentNode {
    pub id: String,
    pub label: String,
    pub kind: DeploymentNodeKind,
    pub stereotype: Option<String>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DeploymentNodeKind {
    #[default]
    Node,
    Artifact,
    Cloud,
    Database,
    Storage,
    Frame,
    Folder,
    Actor,
    Queue,
    Component,
    Rectangle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConnection {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}
