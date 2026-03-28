// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! EBNF (Extended Backus-Naur Form) diagram model.

use serde::{Deserialize, Serialize};

use super::DiagramMeta;

/// A parsed EBNF diagram containing a list of production rules.
#[derive(Debug, Serialize, Deserialize)]
pub struct EbnfDiagram {
    pub meta: DiagramMeta,
    pub rules: Vec<EbnfRule>,
}

/// A single EBNF production rule: `name = body ;`.
#[derive(Debug, Serialize, Deserialize)]
pub struct EbnfRule {
    pub name: String,
    pub body: EbnfExpr,
}

/// An EBNF expression tree.
#[derive(Debug, Serialize, Deserialize)]
pub enum EbnfExpr {
    /// A quoted terminal: `"text"`.
    Terminal(String),
    /// A bare identifier referencing another rule.
    Nonterminal(String),
    /// A comma-separated sequence: `a , b , c`.
    Sequence(Vec<EbnfExpr>),
    /// Alternatives: `a | b | c`.
    Alternation(Vec<EbnfExpr>),
    /// Zero-or-more repetition: `{ expr }`.
    Repetition(Box<EbnfExpr>),
    /// Optional: `[ expr ]`.
    Optional(Box<EbnfExpr>),
    /// Parenthesised group: `( expr )`.
    Group(Box<EbnfExpr>),
}
