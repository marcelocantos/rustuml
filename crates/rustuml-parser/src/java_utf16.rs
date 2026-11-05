// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Java `String` UTF-16 units carried in Rust text.
//!
//! Java strings are sequences of UTF-16 units and may hold an unpaired
//! surrogate (for example after `%substr` splits an astral character). Such a
//! unit travels through PlantUML's parser, layout and drawing unchanged and is
//! only judged when `ToStream` serializes the SVG. Rust `String` cannot hold
//! it, so each unpaired unit `u` is carried as the scalar
//! `CARRIER_BASE + (u - 0xD800)` (U+10F800..=U+10FFFF).
//!
//! Legitimate text may contain those scalars too. To keep the two apart, every
//! legitimate scalar of the reserved set [`ESCAPE`]`..=U+10FFFF` is written as
//! [`ESCAPE`] followed by itself. [`escape`] applies that once where text
//! enters (source, included files, environment values); [`text`] applies it to
//! every scalar it decodes from units. Text without reserved scalars is
//! unchanged, and an unpaired unit stays one scalar.

/// First scalar of the carrier range (U+10F800..=U+10FFFF).
const CARRIER_BASE: u32 = 0x10_F800;
/// Prefix of an escaped legitimate scalar of the reserved range.
pub const ESCAPE: char = '\u{10F7FF}';
const SURROGATE_FIRST: u16 = 0xD800;
const SURROGATE_LAST: u16 = 0xDFFF;
const HIGH_SURROGATE_LAST: u16 = 0xDBFF;

/// One Java character of carried text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    /// A legitimate Unicode scalar.
    Char(char),
    /// An unpaired UTF-16 unit.
    Unit(u16),
}

/// Whether `ch` must be escaped to be read as itself (an escape or a carrier).
pub fn reserved(ch: char) -> bool {
    ch >= ESCAPE
}

/// The scalar that carries an unpaired UTF-16 unit.
pub fn carrier(unit: u16) -> char {
    debug_assert!((SURROGATE_FIRST..=SURROGATE_LAST).contains(&unit));
    char::from_u32(CARRIER_BASE + u32::from(unit - SURROGATE_FIRST))
        .expect("carrier range lies inside Unicode")
}

/// The unpaired unit a carrier scalar stands for, when it is not escaped.
pub fn carried_unit(ch: char) -> Option<u16> {
    let offset = (ch as u32).checked_sub(CARRIER_BASE)?;
    // The carrier range ends at U+10FFFF, so every offset fits a surrogate.
    Some(SURROGATE_FIRST + offset as u16)
}

/// Whether a UTF-16 unit is a high (leading) surrogate.
pub fn is_high_surrogate(unit: u16) -> bool {
    (SURROGATE_FIRST..=HIGH_SURROGATE_LAST).contains(&unit)
}

/// Whether a UTF-16 unit is a low (trailing) surrogate.
pub fn is_low_surrogate(unit: u16) -> bool {
    (HIGH_SURROGATE_LAST + 1..=SURROGATE_LAST).contains(&unit)
}

/// Whether `text` holds any reserved scalar (an escape or a carrier).
pub fn is_carried(text: &str) -> bool {
    text.chars().any(reserved)
}

/// Escape text read from outside: each reserved scalar becomes
/// [`ESCAPE`] + itself. Apply exactly once per entering text.
pub fn escape(text: &str) -> std::borrow::Cow<'_, str> {
    if !is_carried(text) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut escaped = String::with_capacity(text.len() + 4);
    for ch in text.chars() {
        if reserved(ch) {
            escaped.push(ESCAPE);
        }
        escaped.push(ch);
    }
    std::borrow::Cow::Owned(escaped)
}

/// The Java characters of carried text, with the byte span of each.
pub fn token_spans(text: &str) -> impl Iterator<Item = (Token, std::ops::Range<usize>)> + '_ {
    let mut chars = text.char_indices().peekable();
    std::iter::from_fn(move || {
        let (start, ch) = chars.next()?;
        if ch == ESCAPE
            && let Some((at, escaped)) = chars.next()
        {
            return Some((Token::Char(escaped), start..at + escaped.len_utf8()));
        }
        let span = start..start + ch.len_utf8();
        Some(match carried_unit(ch) {
            Some(unit) if ch != ESCAPE => (Token::Unit(unit), span),
            _ => (Token::Char(ch), span),
        })
    })
}

/// The Java characters of carried text.
pub fn tokens(text: &str) -> impl Iterator<Item = Token> + '_ {
    token_spans(text).map(|(token, _)| token)
}

/// Java's UTF-16 units of carried text.
pub fn units(text: &str) -> Vec<u16> {
    let mut units = Vec::with_capacity(text.len());
    for token in tokens(text) {
        match token {
            Token::Unit(unit) => units.push(unit),
            Token::Char(ch) => units.extend(ch.encode_utf16(&mut [0; 2]).iter()),
        }
    }
    units
}

/// Carried text for Java UTF-16 units: pairs decode (reserved scalars
/// escaped), unpaired units are carried.
pub fn text(units: &[u16]) -> String {
    let mut text = String::with_capacity(units.len());
    for decoded in char::decode_utf16(units.iter().copied()) {
        match decoded {
            Ok(ch) => {
                if reserved(ch) {
                    text.push(ESCAPE);
                }
                text.push(ch);
            }
            Err(error) => text.push(carrier(error.unpaired_surrogate())),
        }
    }
    text
}

/// Re-join carried units that became adjacent, as Java string concatenation
/// would: a carried high unit followed by a carried low unit is one scalar.
pub fn normalize(text_in: &str) -> String {
    if !text_in.chars().any(|ch| carried_unit(ch).is_some()) {
        return text_in.to_string();
    }
    text(&units(text_in))
}

/// Plain Rust text for carried text: escapes removed, each unpaired unit
/// replaced by `unpaired` (for outputs that cannot hold one).
pub fn display(text: &str, unpaired: char) -> std::borrow::Cow<'_, str> {
    if !is_carried(text) {
        return std::borrow::Cow::Borrowed(text);
    }
    std::borrow::Cow::Owned(
        tokens(text)
            .map(|token| match token {
                Token::Char(ch) => ch,
                Token::Unit(_) => unpaired,
            })
            .collect(),
    )
}

/// Serialized form of a model whose strings are shown as plain text
/// ([`display`]), or `None` when no string is carried.
pub fn display_value<T: serde::Serialize>(model: &T, unpaired: char) -> Option<serde_yml::Value> {
    fn walk(value: &mut serde_yml::Value, unpaired: char) -> bool {
        match value {
            serde_yml::Value::String(text) => {
                let shown = display(text, unpaired);
                if let std::borrow::Cow::Owned(shown) = shown {
                    *text = shown;
                    return true;
                }
                false
            }
            serde_yml::Value::Sequence(items) => {
                let mut changed = false;
                for item in items {
                    changed |= walk(item, unpaired);
                }
                changed
            }
            serde_yml::Value::Mapping(mapping) => {
                let mut changed = false;
                let entries = std::mem::take(mapping);
                for (mut key, mut item) in entries {
                    changed |= walk(&mut key, unpaired);
                    changed |= walk(&mut item, unpaired);
                    mapping.insert(key, item);
                }
                changed
            }
            serde_yml::Value::Tagged(tagged) => walk(&mut tagged.value, unpaired),
            serde_yml::Value::Null | serde_yml::Value::Bool(_) | serde_yml::Value::Number(_) => {
                false
            }
        }
    }
    let mut value = serde_yml::to_value(model).ok()?;
    walk(&mut value, unpaired).then_some(value)
}

/// A copy of a model with every string shown as plain text ([`display`]),
/// or `None` when no string is carried (or the copy cannot be rebuilt).
pub fn display_model<T: serde::Serialize + serde::de::DeserializeOwned>(
    model: &T,
    unpaired: char,
) -> Option<T> {
    serde_yml::from_value(display_value(model, unpaired)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carriers_round_trip_every_unpaired_unit() {
        for unit in SURROGATE_FIRST..=SURROGATE_LAST {
            assert_eq!(carried_unit(carrier(unit)), Some(unit));
            assert_eq!(units(&text(&[unit])), vec![unit]);
        }
        assert_eq!(carried_unit('x'), None);
        assert_eq!(carried_unit('\u{1F680}'), None);
    }

    #[test]
    fn units_and_text_match_java_strings() {
        let rocket: Vec<u16> = "🚀".encode_utf16().collect();
        let high = text(&rocket[..1]);
        let low = text(&rocket[1..]);
        assert_eq!(units(&format!("x{high}")), vec![u16::from(b'x'), rocket[0]]);
        assert_eq!(normalize(&format!("{high}{low}")), "🚀");
        assert_eq!(normalize(&format!("{low}{high}")), format!("{low}{high}"));
        assert_eq!(units("a🚀"), "a🚀".encode_utf16().collect::<Vec<_>>());
    }

    #[test]
    fn legitimate_reserved_scalars_never_read_as_units() {
        // BC pua_b_*: U+10F800..U+10FFFF is ordinary Unicode text in Java.
        for literal in [
            "Vault \u{10FFFD}",
            "Gate \u{10F800} end",
            "Rune \u{10F83D}\u{10FE80}",
            "\u{10F7FF}\u{10F83D}",
            "\u{10F7FF}",
        ] {
            let escaped = escape(literal);
            assert_eq!(units(&escaped), literal.encode_utf16().collect::<Vec<_>>());
            assert!(tokens(&escaped).all(|token| matches!(token, Token::Char(_))));
            assert_eq!(display(&escaped, '?'), literal);
            assert_eq!(text(&units(&escaped)), escaped);
            assert_eq!(normalize(&escaped), escaped);
        }
        // %strlen: Java's String.length() is two units for U+10FFFD.
        assert_eq!(units(&escape("\u{10FFFD}")).len(), 2);
        assert_eq!(escape("plain"), "plain");
    }

    #[test]
    fn units_and_literals_coexist() {
        let rocket: Vec<u16> = "🚀".encode_utf16().collect();
        let mut java = vec![u16::from(b'a'), rocket[0]];
        java.extend("\u{10F83D}".encode_utf16());
        java.push(rocket[1]);
        let carried = text(&java);
        assert_eq!(units(&carried), java);
        assert_eq!(
            tokens(&carried).collect::<Vec<_>>(),
            vec![
                Token::Char('a'),
                Token::Unit(rocket[0]),
                Token::Char('\u{10F83D}'),
                Token::Unit(rocket[1]),
            ]
        );
        assert_eq!(display(&carried, '\u{FFFD}'), "a\u{FFFD}\u{10F83D}\u{FFFD}");
    }
}
