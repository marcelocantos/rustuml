// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! `RUSTUML_DEBUG` env-var support — knobs for deterministic testing.
//!
//! Syntax: `RUSTUML_DEBUG=key=value,key=value,...`
//!
//! Recognised keys:
//!   * `date=<epoch_ms>` — pins `%date()` to a fixed instant (Java's
//!     `System.currentTimeMillis()` value).
//!   * `tz=<name><±HHMM>` — pins timezone for date formatting.
//!     Examples: `tz=UTC`, `tz=+1100`, `tz=AEDT+1100`, `tz=PST-0800`.
//!
//! Missing keys fall through to the host: system clock for the time,
//! local timezone (via `localtime_r`) for the zone.

/// Resolved render-time snapshot. Captured once per preprocess so all
/// `%date(...)` calls within a render see the same instant and zone.
#[derive(Debug, Clone)]
pub struct RenderClock {
    pub epoch_ms: u64,
    pub tz: TzSpec,
}

#[derive(Debug, Clone)]
pub struct TzSpec {
    /// Offset from UTC in seconds. Positive = east of Greenwich.
    pub offset_secs: i32,
    /// Display name (e.g. "UTC", "AEDT", "+0530").
    pub name: String,
}

impl RenderClock {
    pub fn from_env() -> Self {
        let cfg = parse_debug(std::env::var("RUSTUML_DEBUG").ok().as_deref());
        let epoch_ms = cfg.date.unwrap_or_else(system_epoch_ms);
        let tz = cfg.tz.unwrap_or_else(|| system_tz_at(epoch_ms));
        Self { epoch_ms, tz }
    }
}

struct DebugConfig {
    date: Option<u64>,
    tz: Option<TzSpec>,
}

fn parse_debug(s: Option<&str>) -> DebugConfig {
    let mut out = DebugConfig {
        date: None,
        tz: None,
    };
    let Some(s) = s else { return out };
    for pair in s.split(',') {
        let pair = pair.trim();
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        match k.trim() {
            "date" => out.date = v.trim().parse::<u64>().ok(),
            "tz" => out.tz = parse_tz(v.trim()),
            _ => {} // ignore unknown keys for forward-compat
        }
    }
    out
}

/// Parse a tz spec: `<name><±HHMM>` where either part is optional.
/// Bare offsets become both the offset and the display name.
fn parse_tz(s: &str) -> Option<TzSpec> {
    let split = s
        .char_indices()
        .find(|(i, c)| *i > 0 && (*c == '+' || *c == '-'))
        .map(|(i, _)| i);
    let (name_part, offset_part) = match split {
        Some(i) => (&s[..i], &s[i..]),
        None => {
            if s.starts_with('+') || s.starts_with('-') {
                ("", s)
            } else {
                (s, "")
            }
        }
    };
    let offset_secs = if offset_part.is_empty() {
        0
    } else {
        parse_offset(offset_part)?
    };
    let name = if name_part.is_empty() {
        if offset_part.is_empty() {
            "UTC".to_string()
        } else {
            offset_part.to_string()
        }
    } else {
        name_part.to_string()
    };
    Some(TzSpec { offset_secs, name })
}

/// Parse `±HHMM` or `±HH:MM` into seconds.
fn parse_offset(s: &str) -> Option<i32> {
    let sign = match s.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let digits: String = s[1..].chars().filter(|c| c.is_ascii_digit()).collect();
    let (h, m) = match digits.len() {
        4 => (
            digits[..2].parse::<i32>().ok()?,
            digits[2..].parse::<i32>().ok()?,
        ),
        2 => (digits.parse::<i32>().ok()?, 0),
        _ => return None,
    };
    Some(sign * (h * 3600 + m * 60))
}

fn system_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(unix)]
fn system_tz_at(epoch_ms: u64) -> TzSpec {
    use std::ffi::CStr;
    let t: libc::time_t = (epoch_ms / 1000) as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::localtime_r(&t, &mut tm) };
    if result.is_null() {
        return TzSpec {
            offset_secs: 0,
            name: "UTC".to_string(),
        };
    }
    let name = if tm.tm_zone.is_null() {
        "UTC".to_string()
    } else {
        unsafe { CStr::from_ptr(tm.tm_zone) }
            .to_string_lossy()
            .into_owned()
    };
    TzSpec {
        offset_secs: tm.tm_gmtoff as i32,
        name,
    }
}

#[cfg(not(unix))]
fn system_tz_at(_epoch_ms: u64) -> TzSpec {
    TzSpec {
        offset_secs: 0,
        name: "UTC".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full() {
        let c = parse_debug(Some("date=1774221226000,tz=AEDT+1100"));
        assert_eq!(c.date, Some(1774221226000));
        let tz = c.tz.unwrap();
        assert_eq!(tz.name, "AEDT");
        assert_eq!(tz.offset_secs, 39600);
    }

    #[test]
    fn parse_bare_offset() {
        let tz = parse_tz("+0530").unwrap();
        assert_eq!(tz.name, "+0530");
        assert_eq!(tz.offset_secs, 5 * 3600 + 30 * 60);
    }

    #[test]
    fn parse_negative_named() {
        let tz = parse_tz("PST-0800").unwrap();
        assert_eq!(tz.name, "PST");
        assert_eq!(tz.offset_secs, -8 * 3600);
    }

    #[test]
    fn parse_utc_bare() {
        let tz = parse_tz("UTC").unwrap();
        assert_eq!(tz.name, "UTC");
        assert_eq!(tz.offset_secs, 0);
    }

    #[test]
    fn parse_ignores_unknown_keys() {
        let c = parse_debug(Some("date=42,unknown=foo,tz=UTC"));
        assert_eq!(c.date, Some(42));
        assert_eq!(c.tz.unwrap().name, "UTC");
    }

    #[test]
    fn parse_none() {
        let c = parse_debug(None);
        assert!(c.date.is_none());
        assert!(c.tz.is_none());
    }
}
