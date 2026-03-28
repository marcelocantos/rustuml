// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Graphviz DOT parser.
//!
//! Hand-written recursive-descent parser for the DOT subset used in golden
//! tests: `digraph`/`graph` declarations, node and edge statements, attribute
//! lists, and `subgraph cluster_*` blocks.

use std::collections::HashMap;

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::dot::*;

/// Parse `@startdot` … `@enddot` lines into a [`DotDiagram`].
pub fn parse_dot(lines: &[String]) -> Result<DotDiagram, ParseError> {
    // Collect body lines (strip @start/@end markers and blanks).
    let body: Vec<&str> = lines
        .iter()
        .map(|s| s.as_str())
        .filter(|l| {
            let t = l.trim();
            !t.starts_with("@start") && !t.starts_with("@end")
        })
        .collect();

    let joined = body.join("\n");
    let mut parser = DotParser::new(&joined);
    parser.parse()
}

// ---------------------------------------------------------------------------
// Tokeniser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    StringLit(String),
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Comma,
    Eq,
    Arrow,    // ->
    DashDash, // --
    Eof,
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            // Skip whitespace.
            while self.pos < self.src.len() && self.src.as_bytes()[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            // Skip C-style line comments.
            if self.rest().starts_with("//") {
                if let Some(nl) = self.rest().find('\n') {
                    self.pos += nl + 1;
                } else {
                    self.pos = self.src.len();
                }
                continue;
            }
            // Skip C-style block comments.
            if self.rest().starts_with("/*") {
                if let Some(end) = self.rest().find("*/") {
                    self.pos += end + 2;
                } else {
                    self.pos = self.src.len();
                }
                continue;
            }
            // Skip # line comments (used in some DOT files).
            if self.pos < self.src.len() && self.src.as_bytes()[self.pos] == b'#' {
                if let Some(nl) = self.rest().find('\n') {
                    self.pos += nl + 1;
                } else {
                    self.pos = self.src.len();
                }
                continue;
            }
            break;
        }
    }

    fn next_token(&mut self) -> Token {
        self.skip_ws_and_comments();
        if self.pos >= self.src.len() {
            return Token::Eof;
        }
        let b = self.src.as_bytes()[self.pos];
        match b {
            b'{' => {
                self.pos += 1;
                Token::LBrace
            }
            b'}' => {
                self.pos += 1;
                Token::RBrace
            }
            b'[' => {
                self.pos += 1;
                Token::LBracket
            }
            b']' => {
                self.pos += 1;
                Token::RBracket
            }
            b';' => {
                self.pos += 1;
                Token::Semi
            }
            b',' => {
                self.pos += 1;
                Token::Comma
            }
            b'=' => {
                self.pos += 1;
                Token::Eq
            }
            b'-' => {
                if self.rest().starts_with("->") {
                    self.pos += 2;
                    Token::Arrow
                } else if self.rest().starts_with("--") {
                    self.pos += 2;
                    Token::DashDash
                } else {
                    // Treat lone dash as part of an identifier (e.g. node names with dashes).
                    self.read_ident()
                }
            }
            b'"' => self.read_string(),
            _ if is_id_start(b) => self.read_ident(),
            // Numeric literals (e.g. port numbers used as node IDs).
            _ if b.is_ascii_digit() || b == b'.' => self.read_ident(),
            _ => {
                // Skip unknown character.
                self.pos += 1;
                self.next_token()
            }
        }
    }

    fn peek_token(&mut self) -> Token {
        let saved = self.pos;
        let tok = self.next_token();
        self.pos = saved;
        tok
    }

    fn read_string(&mut self) -> Token {
        self.pos += 1; // skip opening quote
        let start = self.pos;
        let mut s = String::new();
        while self.pos < self.src.len() {
            let ch = self.src.as_bytes()[self.pos];
            if ch == b'\\' && self.pos + 1 < self.src.len() {
                let next = self.src.as_bytes()[self.pos + 1];
                match next {
                    b'"' => s.push('"'),
                    b'\\' => s.push('\\'),
                    b'n' => s.push('\n'),
                    _ => {
                        s.push('\\');
                        s.push(next as char);
                    }
                }
                self.pos += 2;
                continue;
            }
            if ch == b'"' {
                self.pos += 1;
                return Token::StringLit(s);
            }
            s.push(ch as char);
            self.pos += 1;
        }
        // Unterminated string — return what we have.
        Token::StringLit(self.src[start..].to_string())
    }

    fn read_ident(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.src.len() && is_id_char(self.src.as_bytes()[self.pos]) {
            self.pos += 1;
        }
        Token::Ident(self.src[start..self.pos].to_string())
    }
}

fn is_id_start(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_id_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'-'
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct DotParser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> DotParser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            lexer: Lexer::new(src),
        }
    }

    fn parse(&mut self) -> Result<DotDiagram, ParseError> {
        let mut directed = true;
        let mut strict = false;

        // Optional `strict` keyword.
        let tok = self.lexer.next_token();
        let tok = if tok == Token::Ident("strict".to_string()) {
            strict = true;
            let _ = strict; // suppress unused warning
            self.lexer.next_token()
        } else {
            tok
        };

        // `digraph` or `graph`.
        match &tok {
            Token::Ident(kw) if kw == "digraph" => directed = true,
            Token::Ident(kw) if kw == "graph" => directed = false,
            _ => {
                // Default to digraph if no keyword.
                directed = true;
            }
        }

        // Optional graph name.
        let name = match self.lexer.peek_token() {
            Token::Ident(n) => {
                self.lexer.next_token();
                n
            }
            Token::StringLit(n) => {
                self.lexer.next_token();
                n
            }
            _ => String::new(),
        };

        // Expect opening brace.
        let tok = self.lexer.next_token();
        if tok != Token::LBrace {
            return Ok(empty_diagram(directed, name));
        }

        let mut diagram = DotDiagram {
            meta: DiagramMeta::default(),
            directed,
            name,
            attrs: HashMap::new(),
            node_defaults: HashMap::new(),
            edge_defaults: HashMap::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            clusters: Vec::new(),
        };

        self.parse_body(&mut diagram)?;

        Ok(diagram)
    }

    fn parse_body(&mut self, diagram: &mut DotDiagram) -> Result<(), ParseError> {
        loop {
            let tok = self.lexer.peek_token();
            match tok {
                Token::RBrace | Token::Eof => {
                    self.lexer.next_token();
                    break;
                }
                Token::Semi => {
                    self.lexer.next_token();
                    continue;
                }
                _ => self.parse_statement(diagram)?,
            }
        }
        Ok(())
    }

    fn parse_statement(&mut self, diagram: &mut DotDiagram) -> Result<(), ParseError> {
        let tok = self.lexer.next_token();
        match &tok {
            // Default attributes: `node [...]` or `edge [...]`.
            Token::Ident(kw) if kw == "node" => {
                if self.lexer.peek_token() == Token::LBracket {
                    let attrs = self.parse_attr_list()?;
                    for (k, v) in attrs {
                        diagram.node_defaults.insert(k, v);
                    }
                }
                self.skip_semi();
                Ok(())
            }
            Token::Ident(kw) if kw == "edge" => {
                if self.lexer.peek_token() == Token::LBracket {
                    let attrs = self.parse_attr_list()?;
                    for (k, v) in attrs {
                        diagram.edge_defaults.insert(k, v);
                    }
                }
                self.skip_semi();
                Ok(())
            }
            // Graph-level attributes: `graph [...]`.
            Token::Ident(kw) if kw == "graph" => {
                if self.lexer.peek_token() == Token::LBracket {
                    let attrs = self.parse_attr_list()?;
                    for (k, v) in attrs {
                        diagram.attrs.insert(k, v);
                    }
                }
                self.skip_semi();
                Ok(())
            }
            // Subgraph / cluster.
            Token::Ident(kw) if kw == "subgraph" => {
                let cluster = self.parse_cluster()?;
                diagram.clusters.push(cluster);
                self.skip_semi();
                Ok(())
            }
            // Bare attribute: `rankdir=LR`, `label="title"`, etc.
            Token::Ident(id) | Token::StringLit(id) => {
                let id = id.clone();
                self.parse_node_or_edge(id, diagram)
            }
            _ => {
                // Skip unknown tokens.
                self.skip_semi();
                Ok(())
            }
        }
    }

    /// After reading an identifier, decide whether it's a node declaration,
    /// an edge statement, or a bare graph attribute.
    fn parse_node_or_edge(
        &mut self,
        id: String,
        diagram: &mut DotDiagram,
    ) -> Result<(), ParseError> {
        let next = self.lexer.peek_token();
        match next {
            // Edge: `A -> B` or `A -- B`.
            Token::Arrow | Token::DashDash => {
                self.lexer.next_token(); // consume arrow/dash
                let to = self.read_id()?;
                let attrs = if self.lexer.peek_token() == Token::LBracket {
                    self.parse_attr_list()?
                } else {
                    HashMap::new()
                };
                // Ensure both endpoints exist as nodes.
                ensure_node(diagram, &id);
                ensure_node(diagram, &to);
                diagram.edges.push(DotEdge {
                    from: id,
                    to,
                    attrs,
                });
                self.skip_semi();
                Ok(())
            }
            // Bare attribute assignment: `rankdir=LR`.
            Token::Eq => {
                self.lexer.next_token(); // consume `=`
                let val = self.read_id()?;
                diagram.attrs.insert(id, val);
                self.skip_semi();
                Ok(())
            }
            // Node with attribute list: `A [shape=box]`.
            Token::LBracket => {
                let attrs = self.parse_attr_list()?;
                add_or_update_node(diagram, id, attrs);
                self.skip_semi();
                Ok(())
            }
            // Bare node declaration (just an identifier on its own).
            _ => {
                ensure_node(diagram, &id);
                self.skip_semi();
                Ok(())
            }
        }
    }

    fn parse_cluster(&mut self) -> Result<DotCluster, ParseError> {
        // Read cluster name.
        let name = match self.lexer.peek_token() {
            Token::Ident(n) => {
                self.lexer.next_token();
                n
            }
            Token::StringLit(n) => {
                self.lexer.next_token();
                n
            }
            _ => String::new(),
        };

        // Expect opening brace.
        let tok = self.lexer.next_token();
        if tok != Token::LBrace {
            return Ok(DotCluster {
                name,
                label: None,
                nodes: Vec::new(),
                edges: Vec::new(),
            });
        }

        let mut cluster = DotCluster {
            name: name.clone(),
            label: None,
            nodes: Vec::new(),
            edges: Vec::new(),
        };

        // Parse cluster body — similar to graph body but stores into cluster.
        loop {
            let tok = self.lexer.peek_token();
            match tok {
                Token::RBrace | Token::Eof => {
                    self.lexer.next_token();
                    break;
                }
                Token::Semi => {
                    self.lexer.next_token();
                    continue;
                }
                _ => self.parse_cluster_statement(&mut cluster)?,
            }
        }

        Ok(cluster)
    }

    fn parse_cluster_statement(&mut self, cluster: &mut DotCluster) -> Result<(), ParseError> {
        let tok = self.lexer.next_token();
        match &tok {
            Token::Ident(kw) if kw == "node" || kw == "edge" || kw == "graph" => {
                // Skip default attribute blocks inside clusters.
                if self.lexer.peek_token() == Token::LBracket {
                    self.parse_attr_list()?;
                }
                self.skip_semi();
                Ok(())
            }
            Token::Ident(id) | Token::StringLit(id) => {
                let id = id.clone();
                let next = self.lexer.peek_token();
                match next {
                    Token::Arrow | Token::DashDash => {
                        self.lexer.next_token();
                        let to = self.read_id()?;
                        let attrs = if self.lexer.peek_token() == Token::LBracket {
                            self.parse_attr_list()?
                        } else {
                            HashMap::new()
                        };
                        ensure_cluster_node(cluster, &id);
                        ensure_cluster_node(cluster, &to);
                        cluster.edges.push(DotEdge {
                            from: id,
                            to,
                            attrs,
                        });
                        self.skip_semi();
                        Ok(())
                    }
                    Token::Eq => {
                        self.lexer.next_token();
                        let val = self.read_id()?;
                        if id == "label" {
                            cluster.label = Some(val);
                        }
                        self.skip_semi();
                        Ok(())
                    }
                    Token::LBracket => {
                        let attrs = self.parse_attr_list()?;
                        add_or_update_cluster_node(cluster, id, attrs);
                        self.skip_semi();
                        Ok(())
                    }
                    _ => {
                        ensure_cluster_node(cluster, &id);
                        self.skip_semi();
                        Ok(())
                    }
                }
            }
            _ => {
                self.skip_semi();
                Ok(())
            }
        }
    }

    fn parse_attr_list(&mut self) -> Result<HashMap<String, String>, ParseError> {
        let mut attrs = HashMap::new();
        // Consume `[`.
        let tok = self.lexer.next_token();
        if tok != Token::LBracket {
            return Ok(attrs);
        }
        loop {
            let tok = self.lexer.peek_token();
            match tok {
                Token::RBracket | Token::Eof => {
                    self.lexer.next_token();
                    break;
                }
                Token::Comma | Token::Semi => {
                    self.lexer.next_token();
                    continue;
                }
                _ => {
                    let key = self.read_id()?;
                    if self.lexer.peek_token() == Token::Eq {
                        self.lexer.next_token(); // consume `=`
                        let val = self.read_id()?;
                        attrs.insert(key, val);
                    }
                    // Skip comma/semi between entries.
                    let next = self.lexer.peek_token();
                    if next == Token::Comma || next == Token::Semi {
                        self.lexer.next_token();
                    }
                }
            }
        }
        Ok(attrs)
    }

    fn read_id(&mut self) -> Result<String, ParseError> {
        match self.lexer.next_token() {
            Token::Ident(s) | Token::StringLit(s) => Ok(s),
            other => Err(ParseError {
                line: 0,
                message: format!("expected identifier or string, got {other:?}"),
            }),
        }
    }

    fn skip_semi(&mut self) {
        if self.lexer.peek_token() == Token::Semi {
            self.lexer.next_token();
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn empty_diagram(directed: bool, name: String) -> DotDiagram {
    DotDiagram {
        meta: DiagramMeta::default(),
        directed,
        name,
        attrs: HashMap::new(),
        node_defaults: HashMap::new(),
        edge_defaults: HashMap::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        clusters: Vec::new(),
    }
}

fn ensure_node(diagram: &mut DotDiagram, id: &str) {
    if !diagram.nodes.iter().any(|n| n.id == id) {
        diagram.nodes.push(DotNode {
            id: id.to_string(),
            attrs: HashMap::new(),
        });
    }
}

fn add_or_update_node(diagram: &mut DotDiagram, id: String, attrs: HashMap<String, String>) {
    if let Some(node) = diagram.nodes.iter_mut().find(|n| n.id == id) {
        node.attrs.extend(attrs);
    } else {
        diagram.nodes.push(DotNode { id, attrs });
    }
}

fn ensure_cluster_node(cluster: &mut DotCluster, id: &str) {
    if !cluster.nodes.iter().any(|n| n.id == id) {
        cluster.nodes.push(DotNode {
            id: id.to_string(),
            attrs: HashMap::new(),
        });
    }
}

fn add_or_update_cluster_node(
    cluster: &mut DotCluster,
    id: String,
    attrs: HashMap<String, String>,
) {
    if let Some(node) = cluster.nodes.iter_mut().find(|n| n.id == id) {
        node.attrs.extend(attrs);
    } else {
        cluster.nodes.push(DotNode { id, attrs });
    }
}
