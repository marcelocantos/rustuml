// Copyright 2026 Marcelo Cantos
// SPDX-License-Identifier: Apache-2.0

//! Pattern runs and literals from JDK 21 SimpleDateFormat.compile/subFormat,
//! used by PlantUML DateFunction.executeReturnFunction. English date symbols
//! match the pinned reference locale; numeric widths are minimum digit counts.

use super::debug::TzSpec;

pub(super) fn format(epoch_ms: i64, tz: &TzSpec, pattern: &str) -> Result<String, String> {
    let seconds = epoch_ms.div_euclid(1000) + i64::from(tz.offset_secs);
    let epoch_day = seconds.div_euclid(86400);
    let time = seconds.rem_euclid(86400);
    let hour = time / 3600;
    let minute = time % 3600 / 60;
    let second = time % 60;
    // GregorianCalendar rules; signed i32 seconds span 1901 through 2038.
    let leap = |year: i64| year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let year_length = |year| if leap(year) { 366 } else { 365 };
    let mut year = 1970;
    let mut day_of_year = epoch_day;
    while day_of_year < 0 {
        year -= 1;
        day_of_year += year_length(year);
    }
    while day_of_year >= year_length(year) {
        day_of_year -= year_length(year);
        year += 1;
    }
    let january_first = epoch_day - day_of_year;
    let month_lengths = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut day = day_of_year;
    let mut month = 0;
    while day >= month_lengths[month] {
        day -= month_lengths[month];
        month += 1;
    }
    day += 1;
    // Epoch Thursday, Sunday=0. Pinned English locale uses Sunday, minimum 1
    // day for week fields (GregorianCalendar.getWeekYear/getWeekNumber).
    let weekday = (epoch_day + 4).rem_euclid(7);
    let first_week = january_first - (january_first + 4).rem_euclid(7);
    let next_january = january_first + year_length(year);
    let next_first_week = next_january - (next_january + 4).rem_euclid(7);
    let (week_year, week) = if epoch_day >= next_first_week {
        (year + 1, 1)
    } else {
        (year, (epoch_day - first_week) / 7 + 1)
    };
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const DAYS: [&str; 7] = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    let chars: Vec<char> = pattern.chars().collect();
    let mut index = 0;
    let mut quoted = false;
    let mut result = String::new();
    while index < chars.len() {
        let ch = chars[index];
        if ch == '\'' {
            if chars.get(index + 1) == Some(&'\'') {
                result.push('\'');
                index += 2;
            } else {
                quoted = !quoted;
                index += 1;
            }
            continue;
        }
        if quoted || !ch.is_ascii_alphabetic() {
            result.push(ch);
            index += 1;
            continue;
        }
        let start = index;
        while chars.get(index) == Some(&ch) {
            index += 1;
        }
        let width = index - start;
        let numeric = match ch {
            'y' => Some(if width == 2 { year % 100 } else { year }),
            'Y' => Some(if width == 2 {
                week_year % 100
            } else {
                week_year
            }),
            'M' | 'L' if width < 3 => Some(month as i64 + 1),
            'd' => Some(day),
            'D' => Some(day_of_year + 1),
            'H' => Some(hour),
            'k' => Some(if hour == 0 { 24 } else { hour }),
            'K' => Some(hour % 12),
            'h' => Some(if hour % 12 == 0 { 12 } else { hour % 12 }),
            'm' => Some(minute),
            's' => Some(second),
            'S' => Some(epoch_ms.rem_euclid(1000)),
            'u' => Some((weekday + 6) % 7 + 1),
            'F' => Some((day - 1) / 7 + 1),
            'w' => Some(week),
            'W' => Some((day - 1 + (epoch_day - day + 1 + 4).rem_euclid(7)) / 7 + 1),
            _ => None,
        };
        if let Some(value) = numeric {
            result.push_str(&format!("{value:0width$}"));
            continue;
        }
        match ch {
            'M' | 'L' => result.push_str(if width == 3 {
                &MONTHS[month][..3]
            } else {
                MONTHS[month]
            }),
            'E' => result.push_str(if width < 4 {
                &DAYS[weekday as usize][..3]
            } else {
                DAYS[weekday as usize]
            }),
            'a' => result.push_str(if hour < 12 { "AM" } else { "PM" }),
            'G' => result.push_str("AD"),
            'z' => result.push_str(&tz.name),
            'Z' | 'X' => {
                if ch == 'X' && width > 3 {
                    return Err("Bad date pattern: X must have one to three letters".into());
                }
                if ch == 'X' && tz.offset_secs == 0 {
                    result.push('Z');
                    continue;
                }
                let minutes = tz.offset_secs.unsigned_abs() / 60;
                result.push(if tz.offset_secs < 0 { '-' } else { '+' });
                result.push_str(&format!("{:02}", minutes / 60));
                if ch == 'Z' || width >= 2 {
                    if ch == 'X' && width == 3 {
                        result.push(':');
                    }
                    result.push_str(&format!("{:02}", minutes % 60));
                }
            }
            _ => return Err(format!("Bad date pattern: unsupported letter {ch}")),
        }
    }
    if quoted {
        return Err("Bad date pattern: unterminated quote".into());
    }
    Ok(result)
}
