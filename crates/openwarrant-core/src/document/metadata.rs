// SPDX-License-Identifier: Apache-2.0
use super::{Diagnostic, MetadataValue};
use std::ops::Range;

// TOML owns grammar. This pass bounds a conservative depth before TOML can
// allocate a recursively nested value. Dotted keys add depth to their enclosing
// inline table; counting dots and braces independently is unsafe.
#[derive(Clone, Copy)]
enum Container {
    Root,
    Inline,
    Array,
}

struct Frame {
    kind: Container,
    base: usize,
    value_depth: usize,
    dots: usize,
    in_value: bool,
}

impl Frame {
    fn new(kind: Container, base: usize) -> Self {
        Self {
            kind,
            base,
            value_depth: base + 1,
            dots: 0,
            in_value: false,
        }
    }
}

pub(super) fn preflight(text: &str, base: usize, max_depth: usize) -> Result<(), Diagnostic> {
    let bytes = text.as_bytes();
    let mut at = 0;
    let mut frames = vec![Frame::new(Container::Root, 0)];
    let mut header: Option<(bool, usize)> = None;
    let check = |depth: usize, offset: usize| {
        if depth > max_depth {
            Err(Diagnostic::error(
                "resource-limit",
                "Metadata depth bound exceeded",
                base + offset..base + offset + 1,
            ))
        } else {
            Ok(())
        }
    };
    while at < bytes.len() {
        match bytes[at] {
            b'#' => {
                while at < bytes.len() && bytes[at] != b'\n' {
                    at += 1;
                }
            }
            quote @ (b'\'' | b'"') => {
                if bytes.get(at..at + 3) == Some(&[quote, quote, quote]) {
                    return Err(Diagnostic::error(
                        "source-invalid",
                        "TOML multiline strings are unsupported",
                        base + at..base + at + 3,
                    ));
                }
                at += 1;
                while at < bytes.len() {
                    if bytes[at] == quote {
                        at += 1;
                        break;
                    }
                    if quote == b'"' && bytes[at] == b'\\' {
                        at += 1;
                    }
                    at += 1;
                }
            }
            b'.' => {
                if let Some((_, dots)) = &mut header {
                    *dots += 1;
                    check(2 * (*dots + 1), at)?;
                } else {
                    let frame = frames.last_mut().expect("root frame");
                    if !frame.in_value && !matches!(frame.kind, Container::Array) {
                        frame.dots += 1;
                        check(frame.base + frame.dots + 1, at)?;
                    }
                }
                at += 1;
            }
            b'=' => {
                let frame = frames.last_mut().expect("root frame");
                if !frame.in_value && !matches!(frame.kind, Container::Array) {
                    frame.value_depth = frame.base + frame.dots + 1;
                    check(frame.value_depth, at)?;
                    frame.in_value = true;
                }
                at += 1;
            }
            b'[' if frames.len() == 1 && !frames[0].in_value && header.is_none() => {
                let array = bytes.get(at + 1) == Some(&b'[');
                header = Some((array, 0));
                at += if array { 2 } else { 1 };
            }
            b']' if header.is_some() => {
                let (array, dots) = header.take().expect("header");
                // Any path segment can traverse an earlier array-of-tables.
                // Two levels per segment bounds both ordinary and array tables
                // without reproducing TOML key decoding or symbol resolution.
                let depth = 2 * (dots + 1);
                check(depth, at)?;
                frames[0] = Frame::new(Container::Root, depth);
                at += 1;
                if array && bytes.get(at) == Some(&b']') {
                    at += 1;
                }
            }
            b'[' | b'{' => {
                let parent = frames.last().expect("root frame");
                let depth = if matches!(parent.kind, Container::Array) {
                    parent.base + 1
                } else {
                    parent.value_depth
                };
                check(depth, at)?;
                let kind = if bytes[at] == b'[' {
                    Container::Array
                } else {
                    Container::Inline
                };
                frames.push(Frame::new(kind, depth));
                at += 1;
            }
            b']' | b'}' => {
                if frames.len() > 1 {
                    frames.pop();
                }
                at += 1;
            }
            b',' => {
                let frame = frames.last_mut().expect("root frame");
                if matches!(frame.kind, Container::Inline) {
                    frame.in_value = false;
                    frame.dots = 0;
                }
                at += 1;
            }
            b'\n' => {
                if frames.len() == 1 && header.is_none() {
                    frames[0].in_value = false;
                    frames[0].dots = 0;
                }
                at += 1;
            }
            _ => at += 1,
        }
    }
    Ok(())
}

pub(super) fn check_values(
    value: &MetadataValue,
    span: &Range<usize>,
    depth: usize,
    limit: usize,
) -> Result<(), Diagnostic> {
    if depth > limit {
        return Err(Diagnostic::error(
            "resource-limit",
            "Metadata nesting limit exceeded",
            span.clone(),
        ));
    }
    match value {
        MetadataValue::String(_) | MetadataValue::Boolean(_) => Ok(()),
        MetadataValue::Integer(v) if (0..=9_007_199_254_740_991).contains(v) => Ok(()),
        MetadataValue::Array(values) => {
            for item in values {
                check_values(item, span, depth + 1, limit)?;
            }
            Ok(())
        }
        MetadataValue::Table(values) => {
            for item in values.values() {
                check_values(item, span, depth + 1, limit)?;
            }
            Ok(())
        }
        _ => Err(Diagnostic::error(
            "source-invalid",
            "Unsupported metadata value (TOML subset)",
            span.clone(),
        )),
    }
}
