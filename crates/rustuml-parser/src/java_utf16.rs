// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Java `String` UTF-16 units carried in Rust text.
//!
//! Java strings are sequences of UTF-16 units and may hold an unpaired
//! surrogate (for example after `%substr` splits an astral character). Such a
//! unit travels through PlantUML's parser, layout and drawing unchanged and is
//! only judged when `ToStream` serializes the SVG. Rust `String` cannot hold
//! it, so each unpaired unit `u` is carried as the scalar
//! `CARRIER_BASE + (u - 0xD800)`: the last 2048 code points of Supplementary
//! Private Use Area-B, which Unicode leaves unassigned. A user literal in that
//! range is read back as an unpaired unit (declared residue).

/// First scalar of the carrier range (U+10F800..=U+10FFFF).
const CARRIER_BASE: u32 = 0x10_F800;
const SURROGATE_FIRST: u16 = 0xD800;
const SURROGATE_LAST: u16 = 0xDFFF;
const HIGH_SURROGATE_LAST: u16 = 0xDBFF;

/// The scalar that carries an unpaired UTF-16 unit.
pub fn carrier(unit: u16) -> char {
    debug_assert!((SURROGATE_FIRST..=SURROGATE_LAST).contains(&unit));
    char::from_u32(CARRIER_BASE + u32::from(unit - SURROGATE_FIRST))
        .expect("carrier range lies inside Unicode")
}

/// The unpaired unit carried by `ch`, if it is a carrier scalar.
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

/// Java's UTF-16 units of `text`: carriers become their unpaired unit.
pub fn units(text: &str) -> Vec<u16> {
    let mut units = Vec::with_capacity(text.len());
    for ch in text.chars() {
        match carried_unit(ch) {
            Some(unit) => units.push(unit),
            None => units.extend(ch.encode_utf16(&mut [0; 2]).iter()),
        }
    }
    units
}

/// Rust text for Java UTF-16 units: pairs decode, unpaired units are carried.
pub fn text(units: &[u16]) -> String {
    char::decode_utf16(units.iter().copied())
        .map(|decoded| decoded.unwrap_or_else(|error| carrier(error.unpaired_surrogate())))
        .collect()
}

/// Re-join carried units that became adjacent, as Java string concatenation
/// would: a carried high unit followed by a carried low unit is one scalar.
pub fn normalize(text_in: &str) -> String {
    if !text_in.chars().any(|ch| carried_unit(ch).is_some()) {
        return text_in.to_string();
    }
    text(&units(text_in))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carriers_round_trip_every_unpaired_unit() {
        for unit in SURROGATE_FIRST..=SURROGATE_LAST {
            assert_eq!(carried_unit(carrier(unit)), Some(unit));
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
}
