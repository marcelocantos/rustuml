// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Network diagram (nwdiag) model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

#[derive(Debug, Serialize, Deserialize)]
pub struct NwdiagDiagram {
    pub meta: DiagramMeta,
    pub networks: Vec<Network>,
    pub groups: Vec<Group>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Network {
    pub name: String,
    pub address: Option<String>,
    pub color: Option<String>,
    pub hosts: Vec<NetworkHost>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkHost {
    pub name: String,
    pub address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub name: Option<String>,
    pub color: Option<String>,
    pub hosts: Vec<String>,
}
