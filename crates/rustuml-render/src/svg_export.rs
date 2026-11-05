// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! The export step of PlantUML's SVG writer: the UTF-16 surrogate state of
//! the JDK XML serializer.
//!
//! Java: `SvgGraphics.createXmlInternal` runs an identity `Transformer`
//! (`OutputKeys.ENCODING` "us-ascii") over the DOM. `DOM2TO.parse` emits one
//! `characters(String)` per text node and adds each element's attributes in
//! name order; `ToStream.outputCharacters` / `writeAttrString` send every
//! character outside clean ASCII to `accumDefaultEscape` (OpenJDK 21
//! ToStream.java 1693–1781). That method keeps one field, `m_highSurrogate`,
//! across all text nodes and attribute values of the document:
//!
//! * a high unit that is the last character of its node is held there and
//!   written nothing;
//! * the next checked character either completes it (a low unit: the pair is
//!   written as `&#cp;`) or fails with `throwIOE` ("Invalid UTF-16 surrogate
//!   detected: high next ?");
//! * a high unit followed inside its node by anything but a low unit fails the
//!   same way;
//! * a lone low unit fails `XMLChar.isInvalid` ("An invalid XML character
//!   (Unicode: 0x…) was found in the node's character data content.");
//! * nothing flushes the field at the end, so a held unit is dropped.
//!
//! `UGraphicSvg.writeToStream` turns a failure into an IOException and
//! `UgDiagram.exportDiagram` exports Java's crash diagram (🎯T14.20); Rust
//! returns a structured error instead.
//!
//! Rust renderers write the document as text, carrying unpaired units as
//! `rustuml_parser::java_utf16` scalars (raw in text nodes, `&#N;` in
//! attributes). This pass is the serialization boundary: it reads the finished
//! document in Java's node order and applies the rule above. Comments are not
//! character data for it (`ToStream.comment` writes them through the
//! StreamEncoder, where `svg::comment_text` already wrote `?`).

use rustuml_parser::java_utf16;

/// A Java SVG export failure: the SAXException/IOException message that
/// `ToStream` raises while serializing the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgExportError {
    pub message: String,
}

impl std::fmt::Display for SvgExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SvgExportError {}

/// Which `ToStream` writer handles a node's characters.
#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    /// `outputCharacters` (text nodes and CDATA).
    Text,
    /// `writeAttrString` (attribute values).
    Attribute,
}

/// One UTF-16 unit of a node's character data, with the document bytes that
/// wrote it.
struct Unit {
    unit: u16,
    /// Whether `accumDefaultEscape` reaches its surrogate checks for it.
    checked: bool,
    /// Whether the unit is an unpaired unit carried by a java_utf16 scalar.
    carried: bool,
    span: std::ops::Range<usize>,
}

/// Apply `ToStream`'s surrogate rule to a finished SVG document.
pub fn serialize(document: String) -> Result<String, SvgExportError> {
    if !carries_unpaired_unit(&document) {
        return Ok(document);
    }
    let mut serializer = Serializer {
        document: &document,
        high_surrogate: None,
        edits: Vec::new(),
    };
    serializer.walk()?;
    let mut edits = serializer.edits;
    edits.sort_by_key(|(span, _)| span.start);
    let mut output = String::with_capacity(document.len());
    let mut copied = 0;
    for (span, replacement) in edits {
        output.push_str(&document[copied..span.start]);
        output.push_str(&replacement);
        copied = span.end;
    }
    output.push_str(&document[copied..]);
    Ok(output)
}

/// Fast path: documents without a carried unit serialize unchanged.
fn carries_unpaired_unit(document: &str) -> bool {
    document
        .chars()
        .any(|ch| java_utf16::carried_unit(ch).is_some())
        || document.match_indices("&#").any(|(at, _)| {
            character_reference(&document[at..])
                .is_some_and(|(ch, _)| java_utf16::carried_unit(ch).is_some())
        })
}

/// Decode `&#N;` / `&#xN;` at the start of `text`: the scalar and the length.
fn character_reference(text: &str) -> Option<(char, usize)> {
    let body = text.strip_prefix("&#")?;
    let end = body.find(';')?;
    let digits = &body[..end];
    let value = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse().ok()?,
    };
    Some((char::from_u32(value)?, end + 3))
}

struct Serializer<'a> {
    document: &'a str,
    /// `ToStream.m_highSurrogate`, with the bytes of a carried held unit.
    high_surrogate: Option<(u16, Option<std::ops::Range<usize>>)>,
    edits: Vec<(std::ops::Range<usize>, String)>,
}

impl Serializer<'_> {
    /// DOM2TO order: an element's attributes (by name), then its children.
    fn walk(&mut self) -> Result<(), SvgExportError> {
        let document = self.document;
        let mut at = 0;
        while at < document.len() {
            let rest = &document[at..];
            if let Some(body) = rest.strip_prefix("<!--") {
                at += 4 + body.find("-->").map_or(body.len(), |end| end + 3);
            } else if let Some(body) = rest.strip_prefix("<![CDATA[") {
                let end = body.find("]]>").unwrap_or(body.len());
                self.node(NodeKind::Text, at + 9..at + 9 + end, false)?;
                at += 9 + (end + 3).min(body.len());
            } else if rest.starts_with("<?") || rest.starts_with("<!") || rest.starts_with("</") {
                at += rest.find('>').map_or(rest.len(), |end| end + 1);
            } else if rest.starts_with('<') {
                at = self.start_tag(at)?;
            } else {
                let end = rest.find('<').unwrap_or(rest.len());
                self.node(NodeKind::Text, at..at + end, true)?;
                at += end;
            }
        }
        Ok(())
    }

    /// Serialize one start tag's attribute values; returns the byte after it.
    fn start_tag(&mut self, start: usize) -> Result<usize, SvgExportError> {
        let document = self.document;
        let bytes = document.as_bytes();
        let mut at = start + 1;
        while at < bytes.len()
            && !bytes[at].is_ascii_whitespace()
            && !matches!(bytes[at], b'>' | b'/')
        {
            at += 1;
        }
        let mut attributes = Vec::new();
        loop {
            while at < bytes.len() && bytes[at].is_ascii_whitespace() {
                at += 1;
            }
            if at >= bytes.len() {
                break;
            }
            if bytes[at] == b'>' {
                at += 1;
                break;
            }
            if bytes[at] == b'/' {
                at += 1;
                continue;
            }
            let name_start = at;
            while at < bytes.len()
                && !matches!(bytes[at], b'=' | b'>' | b'/')
                && !bytes[at].is_ascii_whitespace()
            {
                at += 1;
            }
            let name = &document[name_start..at];
            while at < bytes.len() && bytes[at].is_ascii_whitespace() {
                at += 1;
            }
            if bytes.get(at) != Some(&b'=') {
                continue;
            }
            at += 1;
            while at < bytes.len() && bytes[at].is_ascii_whitespace() {
                at += 1;
            }
            let Some(&quote) = bytes.get(at).filter(|quote| matches!(quote, b'"' | b'\'')) else {
                continue;
            };
            let value_start = at + 1;
            let value_end = document[value_start..]
                .find(quote as char)
                .map_or(document.len(), |end| value_start + end);
            attributes.push((name, value_start..value_end));
            at = (value_end + 1).min(document.len());
        }
        // Xerces keeps attributes sorted by name (String.compareTo over UTF-16
        // units; attribute names here are ASCII).
        attributes.sort_by(|left, right| left.0.cmp(right.0));
        for (_, value) in attributes {
            self.node(NodeKind::Attribute, value, true)?;
        }
        Ok(at)
    }

    /// One `characters()` or `writeAttrString` call.
    fn node(
        &mut self,
        kind: NodeKind,
        span: std::ops::Range<usize>,
        escaped: bool,
    ) -> Result<(), SvgExportError> {
        let units = decode(self.document, span, kind, escaped);
        let mut index = 0;
        while index < units.len() {
            let current = &units[index];
            if !current.checked {
                index += 1;
                continue;
            }
            if let Some((high, held)) = self.high_surrogate.take() {
                if !java_utf16::is_low_surrogate(current.unit) {
                    return Err(surrogate_error(high, current.unit));
                }
                if current.carried || held.is_some() {
                    self.edits
                        .push((current.span.clone(), pair_reference(high, current.unit)));
                }
                index += 1;
                continue;
            }
            if java_utf16::is_high_surrogate(current.unit) {
                let Some(next) = units.get(index + 1) else {
                    // Held for the next read; nothing is written here.
                    let held = current.carried.then(|| current.span.clone());
                    if let Some(span) = &held {
                        self.edits.push((span.clone(), String::new()));
                    }
                    self.high_surrogate = Some((current.unit, held));
                    index += 1;
                    continue;
                };
                if !java_utf16::is_low_surrogate(next.unit) {
                    return Err(surrogate_error(current.unit, next.unit));
                }
                if current.carried || next.carried {
                    let span = current.span.start..next.span.end;
                    self.edits
                        .push((span, pair_reference(current.unit, next.unit)));
                }
                index += 2;
                continue;
            }
            if java_utf16::is_low_surrogate(current.unit) {
                return Err(SvgExportError {
                    message: format!(
                        "An invalid XML character (Unicode: 0x{:x}) was found in the node's character data content.",
                        current.unit
                    ),
                });
            }
            index += 1;
        }
        Ok(())
    }
}

/// `ToStream.throwIOE`: SerializerMessages ER_INVALID_UTF16_SURROGATE.
fn surrogate_error(high: u16, next: u16) -> SvgExportError {
    SvgExportError {
        message: format!("Invalid UTF-16 surrogate detected: {high:x} {next:x} ?"),
    }
}

/// `ToStream.writeCharRef(writer, high, low)`.
fn pair_reference(high: u16, low: u16) -> String {
    let scalar = char::decode_utf16([high, low])
        .next()
        .and_then(Result::ok)
        .expect("a high unit followed by a low unit is a scalar");
    format!("&#{};", scalar as u32)
}

/// The UTF-16 units of a node as Java holds them, classified by whether
/// `accumDefaultEscape` reaches its surrogate checks (ToStream 1693–1781).
fn decode(
    document: &str,
    span: std::ops::Range<usize>,
    kind: NodeKind,
    escaped: bool,
) -> Vec<Unit> {
    let text = &document[span.clone()];
    let mut units = Vec::with_capacity(text.len());
    let mut offset = 0;
    while offset < text.len() {
        let rest = &text[offset..];
        let (ch, length, entity) = if escaped && rest.starts_with('&') {
            if let Some((ch, length)) = character_reference(rest) {
                (ch, length, false)
            } else if let Some((ch, name)) = [
                ('&', "&amp;"),
                ('<', "&lt;"),
                ('>', "&gt;"),
                ('"', "&quot;"),
                ('\'', "&apos;"),
            ]
            .into_iter()
            .find(|(_, name)| rest.starts_with(name))
            {
                (ch, name.len(), true)
            } else {
                ('&', 1, true)
            }
        } else {
            let ch = rest.chars().next().expect("offset is inside the node");
            (ch, ch.len_utf8(), matches!(ch, '&' | '<' | '>'))
        };
        let char_span = span.start + offset..span.start + offset + length;
        offset += length;
        if let Some(unit) = java_utf16::carried_unit(ch) {
            units.push(Unit {
                unit,
                checked: true,
                carried: true,
                span: char_span,
            });
            continue;
        }
        let checked = !entity && reaches_surrogate_check(ch, kind);
        let mut buffer = [0; 2];
        let encoded = ch.encode_utf16(&mut buffer);
        for (position, unit) in encoded.iter().enumerate() {
            units.push(Unit {
                unit: *unit,
                // The pair's low unit is consumed with its high unit.
                checked: checked && position == 0,
                carried: false,
                span: char_span.clone(),
            });
        }
    }
    units
}

/// Whether a non-entity character reaches `accumDefaultEscape`'s surrogate
/// checks under the us-ascii encoding (ToStream.escapingNotNeeded, CharInfo).
fn reaches_surrogate_check(ch: char, kind: NodeKind) -> bool {
    match kind {
        // outputCharacters: clean ASCII is skipped and processDirty writes
        // LF itself; CR, other C0 controls and everything from DEL up are dirty.
        NodeKind::Text => !matches!(ch, '\n' | '\t' | ' '..='~'),
        // writeAttrString: printable ASCII and DEL are in the encoding; TAB,
        // LF and CR are special attribute characters without an entity.
        NodeKind::Attribute => !matches!(ch, ' '..='\u{7f}'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carried(text: &str, index: usize) -> char {
        let units: Vec<u16> = text.encode_utf16().collect();
        java_utf16::carrier(units[index])
    }

    #[test]
    fn documents_without_carried_units_are_unchanged() {
        let document = "<svg><text x=\"1\">a&amp;b 🚀 &#233;</text><!--c--></svg>".to_string();
        assert_eq!(serialize(document.clone()), Ok(document));
    }

    #[test]
    fn a_high_unit_ending_a_text_node_is_dropped() {
        // AW surrogate_high_class: Java writes 'x' and exits 0.
        let high = carried("🚀", 0);
        let document =
            format!("<svg><text textLength=\"22.5859\">x{high}</text><line x1=\"1\"/></svg>");
        assert_eq!(
            serialize(document).unwrap(),
            "<svg><text textLength=\"22.5859\">x</text><line x1=\"1\"/></svg>"
        );
    }

    #[test]
    fn a_high_unit_followed_inside_its_node_fails() {
        // AW surrogate_activity_label: 'Invalid UTF-16 surrogate detected: d83d 74 ?'.
        let high = carried("😀", 0);
        let error = serialize(format!("<svg><text>{high}tail</text></svg>")).unwrap_err();
        assert_eq!(
            error.message,
            "Invalid UTF-16 surrogate detected: d83d 74 ?"
        );
        let error = serialize(format!("<svg><a title=\"{high}x\"/></svg>")).unwrap_err();
        assert_eq!(
            error.message,
            "Invalid UTF-16 surrogate detected: d83d 78 ?"
        );
    }

    #[test]
    fn a_lone_low_unit_fails_as_an_invalid_character() {
        // AW surrogate_upper_sequence: 'An invalid XML character (Unicode: 0xdc9c) ...'.
        let low = carried("𝒜", 1);
        let error = serialize(format!("<svg><text>{low}</text></svg>")).unwrap_err();
        assert_eq!(
            error.message,
            "An invalid XML character (Unicode: 0xdc9c) was found in the node's character data content."
        );
        let reference = format!("&#{};", low as u32);
        let error = serialize(format!("<svg><a title=\"{reference}\"/></svg>")).unwrap_err();
        assert!(error.message.contains("0xdc9c"), "{error}");
    }

    #[test]
    fn a_held_high_unit_meets_the_next_checked_character() {
        let high = carried("🚀", 0);
        let low = carried("🚀", 1);
        // Clean ASCII, entities and LF are not checked; the next non-ASCII
        // character in a later node fails.
        let error = serialize(format!(
            "<svg><text>a{high}</text><text>b&amp;\n</text><text>é</text></svg>"
        ))
        .unwrap_err();
        assert_eq!(
            error.message,
            "Invalid UTF-16 surrogate detected: d83d e9 ?"
        );
        // An astral character reports its own high unit.
        let error =
            serialize(format!("<svg><text>{high}</text><text>🦊</text></svg>")).unwrap_err();
        assert_eq!(
            error.message,
            "Invalid UTF-16 surrogate detected: d83d d83e ?"
        );
        // A later low unit completes the pair where it is written.
        assert_eq!(
            serialize(format!(
                "<svg><text>a{high}</text><text>{low}b</text></svg>"
            ))
            .unwrap(),
            "<svg><text>a</text><text>&#128640;b</text></svg>"
        );
        // Attributes are serialized in name order before the element's text.
        let error = serialize(format!("<svg><a z=\"{high}\" b=\"é\">t</a></svg>"));
        assert_eq!(error, Ok("<svg><a z=\"\" b=\"é\">t</a></svg>".to_string()));
        let error = serialize(format!("<svg><a b=\"{high}\" z=\"é\">t</a></svg>")).unwrap_err();
        assert_eq!(
            error.message,
            "Invalid UTF-16 surrogate detected: d83d e9 ?"
        );
    }

    #[test]
    fn comments_are_not_character_data() {
        let high = carried("🚀", 0);
        let document = format!("<svg><!--class {high}--><text>é</text></svg>");
        assert_eq!(serialize(document.clone()), Ok(document));
    }
}
