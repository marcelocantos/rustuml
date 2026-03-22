// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Use case diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

#[derive(Debug, Serialize, Deserialize)]
pub struct UseCaseDiagram {
    pub meta: DiagramMeta,
    pub actors: Vec<Actor>,
    pub use_cases: Vec<UseCase>,
    pub connections: Vec<UseCaseConnection>,
    pub packages: Vec<UseCasePackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCase {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCaseConnection {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub stereotype: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCasePackage {
    pub name: String,
    pub elements: Vec<String>,
}
