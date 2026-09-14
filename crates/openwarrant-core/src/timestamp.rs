// SPDX-License-Identifier: Apache-2.0
//! RFC 3339 validation without a date crate.
//!
//! Every authority record carries an `effective_time`, and until this existed
//! nothing checked it: `war sas accept` ingested a literal placeholder into an
//! immutable record (found by the Knowledge Fabric team the first time they used
//! it), and `war authorize` had the same gap. The 57 authorizations on file are
//! well-formed because one tool generated them — not because anything refused a
//! bad one.
//!
//! Hand-parsed, in the same spirit as the civil-from-days formatter in the gate
//! runner: correct for every date this system will record, and no dependency
//! for one field.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TimestampError {
    #[error("expected RFC 3339 like 2026-09-11T14:05:00Z, got {0:?}")]
    Shape(String),
    #[error("{field} out of range in {value:?}")]
    Range { field: &'static str, value: String },
}

fn digits(s: &str, n: usize) -> Option<u32> {
    if s.len() != n || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn is_leap(y: u32) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

/// Accepts `YYYY-MM-DDTHH:MM:SS`, an optional `.fraction`, then `Z` or `±HH:MM`.
/// A lowercase `t`/`z` is accepted (RFC 3339 §5.6 permits it). Nothing else is.
pub fn validate_rfc3339_utc(s: &str) -> Result<(), TimestampError> {
    let shape = || TimestampError::Shape(s.to_owned());
    let range = |field: &'static str| TimestampError::Range {
        field,
        value: s.to_owned(),
    };
    let b = s.as_bytes();
    if b.len() < 20 {
        return Err(shape());
    }
    let year = digits(&s[0..4], 4).ok_or_else(shape)?;
    if b[4] != b'-' {
        return Err(shape());
    }
    let month = digits(&s[5..7], 2).ok_or_else(shape)?;
    if b[7] != b'-' {
        return Err(shape());
    }
    let day = digits(&s[8..10], 2).ok_or_else(shape)?;
    if !matches!(b[10], b'T' | b't') {
        return Err(shape());
    }
    let hour = digits(&s[11..13], 2).ok_or_else(shape)?;
    if b[13] != b':' {
        return Err(shape());
    }
    let minute = digits(&s[14..16], 2).ok_or_else(shape)?;
    if b[16] != b':' {
        return Err(shape());
    }
    let second = digits(&s[17..19], 2).ok_or_else(shape)?;
    let mut i = 19;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return Err(shape());
        }
    }
    let rest = &s[i..];
    match rest.as_bytes() {
        [b'Z'] | [b'z'] => {}
        [sign, ..] if matches!(sign, b'+' | b'-') && rest.len() == 6 => {
            let oh = digits(&rest[1..3], 2).ok_or_else(shape)?;
            if rest.as_bytes()[3] != b':' {
                return Err(shape());
            }
            let om = digits(&rest[4..6], 2).ok_or_else(shape)?;
            if oh > 23 {
                return Err(range("offset hour"));
            }
            if om > 59 {
                return Err(range("offset minute"));
            }
        }
        _ => return Err(shape()),
    }
    if year == 0 {
        return Err(range("year"));
    }
    if !(1..=12).contains(&month) {
        return Err(range("month"));
    }
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
    };
    if !(1..=max_day).contains(&day) {
        return Err(range("day"));
    }
    if hour > 23 {
        return Err(range("hour"));
    }
    if minute > 59 {
        return Err(range("minute"));
    }
    // 60 admits a leap second.
    if second > 60 {
        return Err(range("second"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shapes_this_repository_writes_all_validate() {
        for s in [
            "2026-09-02T06:00:00Z",
            "2026-09-11T14:05:00.123Z",
            "2026-09-11t14:05:00z",
            "2028-02-29T23:59:60+05:30",
            "2026-09-11T00:00:00-08:00",
        ] {
            validate_rfc3339_utc(s).unwrap_or_else(|e| panic!("{s}: {e}"));
        }
    }

    #[test]
    fn placeholders_and_near_misses_are_refused_with_the_right_reason() {
        assert!(matches!(
            validate_rfc3339_utc("soon"),
            Err(TimestampError::Shape(_))
        ));
        assert!(matches!(
            validate_rfc3339_utc("2026-09-02"),
            Err(TimestampError::Shape(_))
        ));
        assert!(matches!(
            validate_rfc3339_utc("2026-09-02T06:00:00"),
            Err(TimestampError::Shape(_))
        ));
        assert!(matches!(
            validate_rfc3339_utc("2026-09-02 06:00:00Z"),
            Err(TimestampError::Shape(_))
        ));
        assert!(matches!(
            validate_rfc3339_utc("2026-13-02T06:00:00Z"),
            Err(TimestampError::Range { field: "month", .. })
        ));
        assert!(matches!(
            validate_rfc3339_utc("2025-02-29T06:00:00Z"),
            Err(TimestampError::Range { field: "day", .. })
        ));
        assert!(matches!(
            validate_rfc3339_utc("2026-09-02T24:00:00Z"),
            Err(TimestampError::Range { field: "hour", .. })
        ));
        assert!(matches!(
            validate_rfc3339_utc("2026-09-02T06:00:00+24:00"),
            Err(TimestampError::Range {
                field: "offset hour",
                ..
            })
        ));
        assert!(matches!(
            validate_rfc3339_utc("2026-09-02T06:00:00.Z"),
            Err(TimestampError::Shape(_))
        ));
    }
}
