// SPDX-License-Identifier: AGPL-3.0-or-later
//! The restricted reader for structured atoms (OW-ADR-0003, SAS §62.1).
//!
//! A SECOND grammar, deliberately not a widening of the frontmatter one. §62.1
//! permits machine-dense atoms to use YAML; this covers the bounded shape those
//! atoms actually use and refuses everything else by name.
//!
//! Accepted:
//!
//! ```text
//! schema: "oh.war/milestones/v1"
//!
//! milestones:
//!   - id: "M1"
//!     title: "Something"
//!     depends_on: ["M0"]
//!
//! stages:
//!   - id: "STAGE-001"
//!     executor_kind: "human"
//! ```
//!
//! Top-level `key: scalar`, or `key:` followed by a block sequence of flat
//! mappings whose values are scalars or flow sequences of scalars. Two levels,
//! no more.
//!
//! Refused by name: anchors, aliases, tags, nested flow collections, block
//! scalars, mappings nested deeper than a sequence item, and multi-document
//! streams.
//!
//! # Why not a YAML library
//!
//! OW-ADR-0003. The short version: implicit typing would read
//! `responsibility_tier: NO` as boolean `false` and `id: Y` as `true`, and
//! milestone identifiers are exactly the short uppercase tokens that mangles.

use std::collections::BTreeMap;
use std::fmt;

/// A value in a structured atom. Three shapes, deliberately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuredValue {
    Scalar(String),
    /// A flow sequence of plain scalars: `["A", "B"]`.
    List(Vec<String>),
    /// A block sequence of flat mappings.
    Records(Vec<BTreeMap<String, StructuredValue>>),
}

impl StructuredValue {
    #[must_use]
    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(s) => Some(s),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_records(&self) -> Option<&[BTreeMap<String, StructuredValue>]> {
        match self {
            Self::Records(r) => Some(r),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StructuredError {
    #[error(
        "line {line}: expected `key: value`, `- key: value`, a comment, or a blank line, found {found:?}"
    )]
    Malformed { line: usize, found: String },
    #[error(
        "line {line}: {construct} is not supported. Structured atoms use a restricted \
         subset, not full YAML (OW-ADR-0003) — this is deliberate, not a missing feature"
    )]
    Unsupported { line: usize, construct: String },
    #[error("line {line}: duplicate key {key:?} in the same mapping")]
    DuplicateKey { line: usize, key: String },
    #[error("line {line}: a sequence item appears before any key")]
    OrphanItem { line: usize },
    #[error(
        "line {line}: indentation {found} is not a level this grammar defines (expected 0, 2, or 4)"
    )]
    BadIndent { line: usize, found: usize },
    #[error("line {line}: unterminated quoted value {value:?}")]
    UnterminatedQuote { line: usize, value: String },
}

/// A parsed structured atom: ordered top-level key/value pairs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StructuredDoc {
    entries: Vec<(String, StructuredValue)>,
}

impl StructuredDoc {
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&StructuredValue> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    #[must_use]
    pub fn scalar(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(StructuredValue::as_scalar)
    }

    #[must_use]
    pub fn records(&self, key: &str) -> Option<&[BTreeMap<String, StructuredValue>]> {
        self.get(key).and_then(StructuredValue::as_records)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(k, _)| k.as_str())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl fmt::Display for StructuredDoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (k, _) in &self.entries {
            writeln!(f, "{k}:")?;
        }
        Ok(())
    }
}

/// Strip one layer of quoting, or return the trimmed input.
fn unquote(raw: &str, line: usize) -> Result<String, StructuredError> {
    let t = raw.trim();
    for q in ['"', '\''] {
        if let Some(rest) = t.strip_prefix(q) {
            return match rest.strip_suffix(q) {
                Some(inner) if !rest.is_empty() => Ok(inner.to_owned()),
                _ => Err(StructuredError::UnterminatedQuote {
                    line,
                    value: t.to_owned(),
                }),
            };
        }
    }
    Ok(t.to_owned())
}

/// Reject constructs outside the subset, by name.
fn reject(value: &str, line: usize) -> Result<(), StructuredError> {
    let v = value.trim();
    let construct = match v.chars().next() {
        Some('&') => "a YAML anchor",
        Some('*') => "a YAML alias",
        Some('!') => "a YAML tag",
        Some('{') => "a flow mapping",
        Some('|') => "a block scalar",
        Some('>') => "a folded block scalar",
        _ => return Ok(()),
    };
    Err(StructuredError::Unsupported {
        line,
        construct: construct.to_owned(),
    })
}

/// Parse a flow sequence of plain scalars: `["A", "B"]`.
fn parse_flow_list(raw: &str, line: usize) -> Result<Vec<String>, StructuredError> {
    let inner = raw
        .trim()
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| StructuredError::Malformed {
            line,
            found: raw.trim().to_owned(),
        })?;
    if inner.contains('[') || inner.contains('{') {
        return Err(StructuredError::Unsupported {
            line,
            construct: "a nested flow collection".to_owned(),
        });
    }
    if inner.trim().is_empty() {
        return Ok(vec![]);
    }
    inner
        .split(',')
        .map(|item| unquote(item, line))
        .collect::<Result<Vec<_>, _>>()
}

/// Parse one `key: value` pair, choosing the value shape.
fn parse_pair(
    text: &str,
    line: usize,
) -> Result<(String, Option<StructuredValue>), StructuredError> {
    let (key, rest) = text
        .split_once(':')
        .ok_or_else(|| StructuredError::Malformed {
            line,
            found: text.to_owned(),
        })?;
    let key = key.trim();
    if key.is_empty() || key.starts_with(['"', '\'']) {
        return Err(StructuredError::Malformed {
            line,
            found: text.to_owned(),
        });
    }
    let rest = rest.trim();
    if rest.is_empty() {
        // A key with no inline value opens a block sequence.
        return Ok((key.to_owned(), None));
    }
    reject(rest, line)?;
    let value = if rest.starts_with('[') {
        StructuredValue::List(parse_flow_list(rest, line)?)
    } else {
        StructuredValue::Scalar(unquote(rest, line)?)
    };
    Ok((key.to_owned(), Some(value)))
}

/// Parse a structured atom.
pub fn parse(source: &str) -> Result<StructuredDoc, StructuredError> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut entries: Vec<(String, StructuredValue)> = Vec::new();
    let mut open_key: Option<String> = None;
    let mut records: Vec<BTreeMap<String, StructuredValue>> = Vec::new();

    // Flush the block sequence accumulated under `open_key`.
    macro_rules! flush {
        () => {
            if let Some(key) = open_key.take() {
                entries.push((key, StructuredValue::Records(std::mem::take(&mut records))));
            }
        };
    }

    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "---" || trimmed == "..." {
            return Err(StructuredError::Unsupported {
                line,
                construct: "a multi-document stream marker".to_owned(),
            });
        }

        let indent = raw.len() - raw.trim_start().len();
        match indent {
            0 => {
                flush!();
                let (key, value) = parse_pair(trimmed, line)?;
                if entries.iter().any(|(k, _)| *k == key) {
                    return Err(StructuredError::DuplicateKey { line, key });
                }
                match value {
                    Some(v) => entries.push((key, v)),
                    None => open_key = Some(key),
                }
            }
            2 => {
                // A sequence item begins a new record.
                let Some(item) = trimmed.strip_prefix("- ") else {
                    return Err(StructuredError::Malformed {
                        line,
                        found: trimmed.to_owned(),
                    });
                };
                if open_key.is_none() {
                    return Err(StructuredError::OrphanItem { line });
                }
                let (key, value) = parse_pair(item, line)?;
                let Some(value) = value else {
                    return Err(StructuredError::Unsupported {
                        line,
                        construct: "a nested block sequence".to_owned(),
                    });
                };
                let mut record = BTreeMap::new();
                record.insert(key, value);
                records.push(record);
            }
            4 => {
                // A continuation field of the current record.
                if trimmed.starts_with("- ") {
                    return Err(StructuredError::Unsupported {
                        line,
                        construct: "a nested block sequence".to_owned(),
                    });
                }
                let Some(record) = records.last_mut() else {
                    return Err(StructuredError::OrphanItem { line });
                };
                let (key, value) = parse_pair(trimmed, line)?;
                let Some(value) = value else {
                    return Err(StructuredError::Unsupported {
                        line,
                        construct: "a mapping nested below a sequence item".to_owned(),
                    });
                };
                if record.insert(key.clone(), value).is_some() {
                    return Err(StructuredError::DuplicateKey { line, key });
                }
            }
            other => return Err(StructuredError::BadIndent { line, found: other }),
        }
    }
    flush!();

    Ok(StructuredDoc { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"schema: "oh.war/milestones/v1"

milestones:
  - id: "M1"
    title: "First"
    stage_refs: ["STAGE-001"]
  - id: "M2"
    title: "Second"
    depends_on: ["M1"]
    stage_refs: ["STAGE-002"]

stages:
  - id: "STAGE-001"
    title: "Do the thing"
    executor_kind: "human"
    responsibility_tier: "T2"
"#;

    #[test]
    fn parses_a_real_milestones_atom() {
        let doc = parse(SAMPLE).expect("valid");
        assert_eq!(doc.scalar("schema"), Some("oh.war/milestones/v1"));

        let ms = doc.records("milestones").expect("milestones");
        assert_eq!(ms.len(), 2);
        assert_eq!(ms[0]["id"].as_scalar(), Some("M1"));
        assert_eq!(
            ms[1]["depends_on"].as_list(),
            Some(["M1".to_owned()].as_slice())
        );

        let st = doc.records("stages").expect("stages");
        assert_eq!(st.len(), 1);
        assert_eq!(st[0]["responsibility_tier"].as_scalar(), Some("T2"));
    }

    #[test]
    fn every_milestones_atom_in_this_repository_parses() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/warrants");
        let mut checked = 0;
        for entry in std::fs::read_dir(dir).expect("warrants dir") {
            let path = entry
                .expect("entry")
                .path()
                .join("atoms/45-milestones.yaml");
            if !path.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("readable");
            let doc =
                parse(&text).unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()));
            assert!(doc.records("milestones").is_some(), "{path:?}");
            assert!(doc.records("stages").is_some(), "{path:?}");
            checked += 1;
        }
        // The corpus is the acceptance test; a silently empty run would prove nothing.
        assert!(
            checked >= 40,
            "expected the whole corpus, checked {checked}"
        );
    }

    /// The Norway problem, which is why this is not a YAML parser: a tier or an
    /// identifier must survive as text.
    #[test]
    fn short_uppercase_tokens_are_not_implicitly_typed() {
        let doc = parse("a: NO\nb: Y\nc: TRUE\nd: 1.10\n").expect("valid");
        assert_eq!(doc.scalar("a"), Some("NO"));
        assert_eq!(doc.scalar("b"), Some("Y"));
        assert_eq!(doc.scalar("c"), Some("TRUE"));
        assert_eq!(
            doc.scalar("d"),
            Some("1.10"),
            "version strings must not become floats"
        );
    }

    #[test]
    fn empty_flow_lists_parse() {
        let doc = parse("k:\n  - id: \"A\"\n    refs: []\n").expect("valid");
        assert_eq!(
            doc.records("k").expect("k")[0]["refs"].as_list(),
            Some([].as_slice())
        );
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let doc = parse("# note\n\nschema: \"x\"\n\n# another\n").expect("valid");
        assert_eq!(doc.len(), 1);
    }

    // --- planted violations ---

    #[test]
    fn anchors_aliases_and_tags_are_refused() {
        for (src, what) in [
            ("a: &anchor v\n", "a YAML anchor"),
            ("a: *alias\n", "a YAML alias"),
            ("a: !!str 5\n", "a YAML tag"),
            ("a: {b: 1}\n", "a flow mapping"),
            ("a: |\n", "a block scalar"),
            ("a: >\n", "a folded block scalar"),
        ] {
            assert_eq!(
                parse(src),
                Err(StructuredError::Unsupported {
                    line: 1,
                    construct: what.to_owned()
                }),
                "{src:?}"
            );
        }
    }

    #[test]
    fn nested_flow_collections_are_refused() {
        assert_eq!(
            parse("k:\n  - id: \"A\"\n    refs: [[1], [2]]\n"),
            Err(StructuredError::Unsupported {
                line: 3,
                construct: "a nested flow collection".to_owned()
            })
        );
    }

    #[test]
    fn deeper_nesting_is_refused() {
        assert_eq!(
            parse("k:\n  - id: \"A\"\n      deep: 1\n"),
            Err(StructuredError::BadIndent { line: 3, found: 6 })
        );
    }

    #[test]
    fn a_sequence_item_before_any_key_is_refused() {
        assert_eq!(
            parse("  - id: \"A\"\n"),
            Err(StructuredError::OrphanItem { line: 1 })
        );
    }

    #[test]
    fn duplicate_keys_are_refused() {
        assert_eq!(
            parse("a: 1\na: 2\n"),
            Err(StructuredError::DuplicateKey {
                line: 2,
                key: "a".to_owned()
            })
        );
        assert_eq!(
            parse("k:\n  - id: \"A\"\n    id: \"B\"\n"),
            Err(StructuredError::DuplicateKey {
                line: 3,
                key: "id".to_owned()
            })
        );
    }

    #[test]
    fn multi_document_streams_are_refused() {
        assert_eq!(
            parse("a: 1\n---\nb: 2\n"),
            Err(StructuredError::Unsupported {
                line: 2,
                construct: "a multi-document stream marker".to_owned()
            })
        );
    }

    #[test]
    fn unterminated_quotes_are_refused() {
        assert_eq!(
            parse("a: \"oops\n"),
            Err(StructuredError::UnterminatedQuote {
                line: 1,
                value: "\"oops".to_owned()
            })
        );
    }

    #[test]
    fn a_line_without_a_colon_is_refused() {
        assert_eq!(
            parse("just prose\n"),
            Err(StructuredError::Malformed {
                line: 1,
                found: "just prose".to_owned()
            })
        );
    }
}
