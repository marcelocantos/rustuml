// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! EBNF parser — turns `@startebnf` … `@endebnf` lines into an [`EbnfDiagram`].
//!
//! EBNF syntax supported:
//! - Rules: `name = body ;`
//! - Terminals: `"text"`
//! - Nonterminals: bare identifiers
//! - Alternation: `a | b`
//! - Sequence: `a , b`
//! - Repetition: `{ expr }`
//! - Optional: `[ expr ]`
//! - Grouping: `( expr )`

use super::ParseError;
use crate::diagram::DiagramMeta;
use crate::diagram::ebnf::*;

/// Parse `@startebnf` … `@endebnf` lines into an [`EbnfDiagram`].
pub fn parse_ebnf(lines: &[String]) -> Result<EbnfDiagram, ParseError> {
    // Extract body lines (skip @start/@end directives).
    let body: String = lines
        .iter()
        .map(|s| s.as_str())
        .filter(|l| {
            let t = l.trim();
            !t.starts_with("@start") && !t.starts_with("@end")
        })
        .collect::<Vec<&str>>()
        .join(" ");

    let mut rules = Vec::new();

    // Split on `;` respecting quoted strings (both `"` and `'`).
    for chunk in split_on_semicolons(&body) {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        // Split on the first `=` to separate name from body.
        let eq_pos = chunk.find('=').ok_or_else(|| ParseError {
            line: 1,
            message: format!("expected '=' in EBNF rule: {chunk}"),
        })?;
        let name = chunk[..eq_pos].trim().to_string();
        let body_str = chunk[eq_pos + 1..].trim();

        if name.is_empty() {
            return Err(ParseError {
                line: 1,
                message: "empty rule name".to_string(),
            });
        }

        let body = parse_expr(body_str)?;
        rules.push(EbnfRule { name, body });
    }

    Ok(EbnfDiagram {
        meta: DiagramMeta::default(),
        rules,
    })
}

/// Split a string on unquoted `;` characters, respecting `"` and `'` delimiters.
fn split_on_semicolons(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let bytes = input.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1; // skip closing quote
                }
            }
            b';' => {
                parts.push(&input[start..i]);
                i += 1;
                start = i;
            }
            _ => {
                i += 1;
            }
        }
    }
    if start < input.len() {
        parts.push(&input[start..]);
    }
    parts
}

// ── Recursive-descent expression parser ──────────────────────────────────────

/// Tokeniser output.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Terminal(String),
    Ident(String),
    Pipe,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '"' | '\'' => {
                let quote = ch;
                chars.next(); // consume opening quote
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some(c) if c == quote => break,
                        Some(c) => s.push(c),
                        None => {
                            return Err(ParseError {
                                line: 1,
                                message: "unterminated string literal".to_string(),
                            });
                        }
                    }
                }
                tokens.push(Token::Terminal(s));
            }
            '?' => {
                // Special sequence: `? description ?`
                chars.next(); // consume opening `?`
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some('?') => break,
                        Some(c) => s.push(c),
                        None => {
                            return Err(ParseError {
                                line: 1,
                                message: "unterminated special sequence (missing closing '?')"
                                    .to_string(),
                            });
                        }
                    }
                }
                tokens.push(Token::Terminal(s.trim().to_string()));
            }
            '|' => {
                chars.next();
                tokens.push(Token::Pipe);
            }
            ',' => {
                chars.next();
                tokens.push(Token::Comma);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '{' => {
                chars.next();
                tokens.push(Token::LBrace);
            }
            '}' => {
                chars.next();
                tokens.push(Token::RBrace);
            }
            '[' => {
                chars.next();
                tokens.push(Token::LBracket);
            }
            ']' => {
                chars.next();
                tokens.push(Token::RBracket);
            }
            _ if ch.is_alphanumeric() || ch == '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Ident(ident));
            }
            other => {
                return Err(ParseError {
                    line: 1,
                    message: format!("unexpected character in EBNF: '{other}'"),
                });
            }
        }
    }

    Ok(tokens)
}

/// Parse a complete EBNF expression from a string.
fn parse_expr(input: &str) -> Result<EbnfExpr, ParseError> {
    let tokens = tokenize(input)?;
    let mut pos = 0;
    let expr = parse_alternation(&tokens, &mut pos)?;
    if pos < tokens.len() {
        return Err(ParseError {
            line: 1,
            message: format!("unexpected token at position {pos}: {:?}", tokens[pos]),
        });
    }
    Ok(expr)
}

/// alternation = sequence { "|" sequence } .
fn parse_alternation(tokens: &[Token], pos: &mut usize) -> Result<EbnfExpr, ParseError> {
    let mut branches = vec![parse_sequence(tokens, pos)?];
    while *pos < tokens.len() && tokens[*pos] == Token::Pipe {
        *pos += 1; // consume '|'
        branches.push(parse_sequence(tokens, pos)?);
    }
    if branches.len() == 1 {
        Ok(branches.pop().unwrap())
    } else {
        Ok(EbnfExpr::Alternation(branches))
    }
}

/// sequence = atom { "," atom } .
///
/// If there is no comma, a bare juxtaposition of atoms also forms a sequence
/// (some EBNF dialects omit commas).  However, the primary golden tests use
/// commas, so we require them here for clarity and only collapse a single-item
/// "sequence" to its inner element.
fn parse_sequence(tokens: &[Token], pos: &mut usize) -> Result<EbnfExpr, ParseError> {
    let mut items = vec![parse_atom(tokens, pos)?];
    while *pos < tokens.len() && tokens[*pos] == Token::Comma {
        *pos += 1; // consume ','
        items.push(parse_atom(tokens, pos)?);
    }
    if items.len() == 1 {
        Ok(items.pop().unwrap())
    } else {
        Ok(EbnfExpr::Sequence(items))
    }
}

/// atom = terminal | nonterminal | "(" alternation ")" | "{" alternation "}" | "[" alternation "]" .
fn parse_atom(tokens: &[Token], pos: &mut usize) -> Result<EbnfExpr, ParseError> {
    if *pos >= tokens.len() {
        return Err(ParseError {
            line: 1,
            message: "unexpected end of EBNF expression".to_string(),
        });
    }
    match &tokens[*pos] {
        Token::Terminal(s) => {
            let s = s.clone();
            *pos += 1;
            Ok(EbnfExpr::Terminal(s))
        }
        Token::Ident(s) => {
            let s = s.clone();
            *pos += 1;
            Ok(EbnfExpr::Nonterminal(s))
        }
        Token::LParen => {
            *pos += 1;
            let inner = parse_alternation(tokens, pos)?;
            expect(tokens, pos, &Token::RParen, ")")?;
            Ok(EbnfExpr::Group(Box::new(inner)))
        }
        Token::LBrace => {
            *pos += 1;
            let inner = parse_alternation(tokens, pos)?;
            expect(tokens, pos, &Token::RBrace, "}")?;
            Ok(EbnfExpr::Repetition(Box::new(inner)))
        }
        Token::LBracket => {
            *pos += 1;
            let inner = parse_alternation(tokens, pos)?;
            expect(tokens, pos, &Token::RBracket, "]")?;
            Ok(EbnfExpr::Optional(Box::new(inner)))
        }
        other => Err(ParseError {
            line: 1,
            message: format!("unexpected token in EBNF expression: {other:?}"),
        }),
    }
}

fn expect(
    tokens: &[Token],
    pos: &mut usize,
    expected: &Token,
    label: &str,
) -> Result<(), ParseError> {
    if *pos >= tokens.len() || tokens[*pos] != *expected {
        return Err(ParseError {
            line: 1,
            message: format!("expected '{label}' in EBNF expression"),
        });
    }
    *pos += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> EbnfDiagram {
        let lines: Vec<String> = input.lines().map(|s| s.to_string()).collect();
        parse_ebnf(&lines).unwrap()
    }

    #[test]
    fn basic_rule() {
        let d = parse("@startebnf\nfoo = \"bar\";\n@endebnf");
        assert_eq!(d.rules.len(), 1);
        assert_eq!(d.rules[0].name, "foo");
        assert!(matches!(&d.rules[0].body, EbnfExpr::Terminal(s) if s == "bar"));
    }

    #[test]
    fn alternation() {
        let d = parse("@startebnf\nop = \"+\" | \"-\";\n@endebnf");
        assert_eq!(d.rules.len(), 1);
        assert!(matches!(&d.rules[0].body, EbnfExpr::Alternation(branches) if branches.len() == 2));
    }

    #[test]
    fn sequence() {
        let d = parse("@startebnf\nexpr = term , \"+\" , term;\n@endebnf");
        assert_eq!(d.rules.len(), 1);
        assert!(matches!(&d.rules[0].body, EbnfExpr::Sequence(items) if items.len() == 3));
    }

    #[test]
    fn repetition() {
        let d = parse("@startebnf\nlist = { item };\n@endebnf");
        assert_eq!(d.rules.len(), 1);
        assert!(matches!(&d.rules[0].body, EbnfExpr::Repetition(_)));
    }

    #[test]
    fn optional() {
        let d = parse("@startebnf\nmaybe = [ thing ];\n@endebnf");
        assert_eq!(d.rules.len(), 1);
        assert!(matches!(&d.rules[0].body, EbnfExpr::Optional(_)));
    }

    #[test]
    fn multiple_rules() {
        let d = parse(
            "@startebnf\nop = add_op | mul_op;\nadd_op = \"+\" | \"-\";\nmul_op = \"*\" | \"/\";\n@endebnf",
        );
        assert_eq!(d.rules.len(), 3);
    }

    #[test]
    fn empty_diagram() {
        let d = parse("@startebnf\n@endebnf");
        assert_eq!(d.rules.len(), 0);
    }

    #[test]
    fn grouping() {
        let d = parse("@startebnf\nexpr = a , ( b | c );\n@endebnf");
        assert_eq!(d.rules.len(), 1);
        if let EbnfExpr::Sequence(items) = &d.rules[0].body {
            assert_eq!(items.len(), 2);
            assert!(matches!(&items[1], EbnfExpr::Group(_)));
        } else {
            panic!("expected Sequence");
        }
    }

    #[test]
    fn single_quoted_terminal() {
        let d = parse("@startebnf\nstring = '\"' , { char } , '\"';\n@endebnf");
        assert_eq!(d.rules.len(), 1);
    }

    #[test]
    fn json_value_ebnf() {
        let input = "@startebnf
value = string | number | object | array | \"true\" | \"false\" | \"null\";
object = \"{\" , [ member , { \",\" , member } ] , \"}\";
member = string , \":\" , value;
array = \"[\" , [ value , { \",\" , value } ] , \"]\";
string = '\"' , { char } , '\"';
number = digit , { digit };
char = letter | digit;
letter = \"a\" | \"b\" | \"c\";
digit = \"0\" | \"1\" | \"2\";
@endebnf";
        let d = parse(input);
        assert_eq!(d.rules.len(), 9);
    }

    #[test]
    fn xml_element_ebnf() {
        let input = "@startebnf
element = start_tag , content , end_tag | empty_element;
start_tag = \"<\" , name , { attribute } , \">\";
end_tag = \"</\" , name , \">\";
empty_element = \"<\" , name , { attribute } , \"/>\";
content = { element | char_data };
attribute = name , \"=\" , quoted_string;
quoted_string = '\"' , { char } , '\"';
name = letter , { letter | digit | \"-\" | \"_\" };
char_data = { char };
char = letter | digit;
letter = \"a\" | \"b\" | \"c\";
digit = \"0\" | \"1\" | \"2\";
@endebnf";
        let d = parse(input);
        assert_eq!(d.rules.len(), 12);
    }
}
