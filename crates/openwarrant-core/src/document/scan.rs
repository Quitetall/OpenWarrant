// SPDX-License-Identifier: Apache-2.0
use super::{Diagnostic, Dialect, ParseLimits, Unit, UnitKind};
use std::ops::Range;

fn line(source: &str, at: usize) -> (&str, usize) {
    let end = source[at..].find('\n').map_or(source.len(), |n| at + n + 1);
    let text = &source[at..end];
    let text = if let Some(without_lf) = text.strip_suffix('\n') {
        without_lf.strip_suffix('\r').unwrap_or(without_lf)
    } else {
        text
    };
    (text, end)
}

pub(super) fn unit_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

fn heading(value: &str) -> bool {
    let count = value.bytes().take_while(|c| *c == b'#').count();
    (1..=6).contains(&count) && value[count..].starts_with(' ') && !value[count + 1..].is_empty()
}

fn expect(source: &str, at: &mut usize, wanted: &str) -> Result<(), Diagnostic> {
    let (text, end) = line(source, *at);
    if *at == source.len() || text != wanted {
        return Err(Diagnostic::error(
            "source-invalid",
            format!("Expected {wanted}"),
            *at..end,
        ));
    }
    *at = end;
    Ok(())
}

fn footer(source: &str, mut at: usize, limit: usize) -> Result<Range<usize>, Diagnostic> {
    expect(source, &mut at, "<!-- ow:metadata -->")?;
    let wrapped = line(source, at).0 == "<details>";
    if wrapped {
        expect(source, &mut at, "<details>")?;
        expect(source, &mut at, "<summary>OpenWarrant metadata</summary>")?;
        if line(source, at).0.is_empty() && at < source.len() {
            at = line(source, at).1;
        }
    }
    expect(source, &mut at, "```toml")?;
    let start = at;
    while at < source.len() && line(source, at).0 != "```" {
        at = line(source, at).1;
        if at - start > limit {
            return Err(Diagnostic::error(
                "resource-limit",
                "Metadata byte limit exceeded",
                start..at,
            ));
        }
    }
    let span = start..at;
    expect(source, &mut at, "```")?;
    if wrapped {
        if line(source, at).0.is_empty() && at < source.len() {
            at = line(source, at).1;
        }
        expect(source, &mut at, "</details>")?;
    }
    expect(source, &mut at, "<!-- /ow:metadata -->")?;
    if !source[at..]
        .bytes()
        .all(|c| matches!(c, b' ' | b'\t' | b'\r' | b'\n'))
    {
        return Err(Diagnostic::error(
            "source-invalid",
            "Content after metadata footer",
            at..source.len(),
        ));
    }
    Ok(span)
}

pub(super) fn scan(
    source: &str,
    dialect: Dialect,
    limits: ParseLimits,
) -> Result<(Range<usize>, Vec<Unit>), Diagnostic> {
    let mut at = 0;
    let mut metadata = None;
    if dialect == Dialect::Rc2 {
        expect(source, &mut at, "+++")?;
        let start = at;
        while at < source.len() && line(source, at).0 != "+++" {
            at = line(source, at).1;
            if at - start > limits.metadata_bytes {
                return Err(Diagnostic::error(
                    "resource-limit",
                    "Metadata byte limit exceeded",
                    start..at,
                ));
            }
        }
        metadata = Some(start..at);
        expect(source, &mut at, "+++")?;
    }
    let mut units: Vec<Unit> = Vec::new();
    let mut unit_ids = std::collections::BTreeSet::new();
    let mut fence: Option<(u8, usize)> = None;
    let mut body_end = source.len();
    while at < source.len() {
        let (text, end) = line(source, at);
        if let Some((character, count)) = fence {
            let prefix = text.bytes().take_while(|c| *c == character).count();
            if prefix >= count && text[prefix..].bytes().all(|c| c == b' ' || c == b'\t') {
                fence = None;
            }
            at = end;
            continue;
        }
        if text.starts_with("<!-- ow:metadata") || text.starts_with("<!-- /ow:metadata") {
            if dialect != Dialect::Rc3 || text != "<!-- ow:metadata -->" {
                return Err(Diagnostic::error(
                    "source-invalid",
                    "Malformed or mixed metadata framing",
                    at..end,
                ));
            }
            body_end = at;
            metadata = Some(footer(source, at, limits.metadata_bytes)?);
            break;
        }
        if text.starts_with("<!-- ow:unit") {
            let marker = text
                .strip_prefix("<!-- ow:unit ")
                .and_then(|v| v.strip_suffix(" -->"));
            let (id, kind) = marker.and_then(|v| v.split_once(' ')).ok_or_else(|| {
                Diagnostic::error("source-invalid", "Malformed unit marker", at..end)
            })?;
            let kind = match kind {
                "binding" => UnitKind::Binding,
                "background" => UnitKind::Background,
                _ => {
                    return Err(Diagnostic::error(
                        "source-invalid",
                        "Invalid unit kind",
                        at..end,
                    ));
                }
            };
            if !unit_id(id) {
                return Err(Diagnostic::error(
                    "source-invalid",
                    "Invalid unit ID",
                    at..end,
                ));
            }
            if !unit_ids.insert(id) {
                return Err(Diagnostic::error(
                    "unit-duplicate",
                    "Duplicate unit ID",
                    at..end,
                ));
            }
            if units.len() == limits.units {
                return Err(Diagnostic::error(
                    "resource-limit",
                    "Unit count limit exceeded",
                    at..end,
                ));
            }
            if !heading(line(source, end).0) {
                return Err(Diagnostic::error(
                    "source-invalid",
                    "Unit requires an immediate heading",
                    end..line(source, end).1,
                ));
            }
            if let Some(previous) = units.last_mut() {
                previous.span.end = at;
            }
            units.push(Unit {
                id: id.into(),
                kind,
                span: end..source.len(),
            });
        } else if units.is_empty() && !text.bytes().all(|c| matches!(c, b' ' | b'\t' | b'\r')) {
            return Err(Diagnostic::error(
                "source-invalid",
                "Content before first unit",
                at..end,
            ));
        }
        if let Some(character @ (b'`' | b'~')) = text.as_bytes().first().copied() {
            let count = text.bytes().take_while(|c| *c == character).count();
            if count >= 3 {
                if character == b'`' && text[count..].contains('`') {
                    return Err(Diagnostic::error(
                        "source-invalid",
                        "Backticks in fence info string",
                        at..end,
                    ));
                }
                fence = Some((character, count));
            }
        }
        at = end;
    }
    if fence.is_some() {
        return Err(Diagnostic::error(
            "source-invalid",
            "Unclosed body fence",
            at..at,
        ));
    }
    if let Some(last) = units.last_mut() {
        last.span.end = body_end;
    }
    let metadata = metadata.ok_or_else(|| {
        Diagnostic::error(
            "source-invalid",
            "Missing metadata footer",
            source.len()..source.len(),
        )
    })?;
    Ok((metadata, units))
}
