// SPDX-License-Identifier: AGPL-3.0-or-later
//! The restricted frontmatter reader (OW-ADR-0002, SAS §62).
//!
//! This is **not** a YAML parser and must not grow into one. It accepts a
//! documented subset and refuses everything else with a diagnostic naming the
//! line and the construct.
//!
//! Accepted:
//!
//! ```text
//! ---
//! key: plain scalar
//! quoted: "value with: a colon"
//! list:
//!   - item one
//!   - "item two"
//! # comments and blank lines are fine
//! ---
//! ```
//!
//! Refused, each as a named error rather than a silent reinterpretation:
//! anchors (`&a`), aliases (`*a`), tags (`!!str`), flow collections (`{}`/`[]`),
//! nested mappings, block scalars (`|`/`>`), and duplicate keys.
//!
//! Refusing is the point. A permissive parser hands the validator a value it
//! then has to second-guess; this one makes "I did not understand that" a
//! result the author sees immediately.

use std::collections::BTreeSet;
use std::fmt;

/// A frontmatter value. Deliberately only two shapes (OW-ADR-0002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Scalar(String),
    List(Vec<String>),
}

impl Value {
    /// The scalar text, or `None` if this is a list.
    #[must_use]
    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(s) => Some(s),
            Self::List(_) => None,
        }
    }

    /// The list items, or `None` if this is a scalar.
    #[must_use]
    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            Self::List(items) => Some(items),
            Self::Scalar(_) => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FrontmatterError {
    #[error("the document does not begin with a `---` frontmatter fence")]
    MissingOpenFence,
    #[error("the frontmatter block is never closed by a `---` fence")]
    UnclosedFence,
    #[error("line {line}: expected `key: value`, `- item`, a comment, or `---`, found {found:?}")]
    Malformed { line: usize, found: String },
    #[error(
        "line {line}: {construct} is not supported. Frontmatter uses a restricted subset, \
         not full YAML (OW-ADR-0002) — this is deliberate, not a missing feature"
    )]
    UnsupportedConstruct { line: usize, construct: String },
    #[error("line {line}: duplicate key {key:?}; a key may appear once")]
    DuplicateKey { line: usize, key: String },
    #[error("line {line}: list item `- {item}` appears before any key")]
    OrphanListItem { line: usize, item: String },
    #[error("line {line}: unterminated quoted value {value:?}")]
    UnterminatedQuote { line: usize, value: String },
}

/// A parsed frontmatter block: ordered key/value pairs plus the body offset.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Frontmatter {
    entries: Vec<(String, Value)>,
    /// Byte offset in the original source where the body begins.
    pub body_offset: usize,
}

impl Frontmatter {
    /// Look up a key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Look up a scalar key.
    #[must_use]
    pub fn scalar(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_scalar)
    }

    /// Look up a list key.
    #[must_use]
    pub fn list(&self, key: &str) -> Option<&[String]> {
        self.get(key).and_then(Value::as_list)
    }

    /// Every key, in source order. Unknown keys are preserved, not dropped —
    /// §62.3 requires namespaced optional fields to survive.
    ///
    /// No `#[must_use]`: `Iterator` already carries one, and a redundant
    /// attribute is itself a clippy error.
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

impl fmt::Display for Frontmatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in &self.entries {
            match value {
                Value::Scalar(s) => writeln!(f, "{key}: {s}")?,
                Value::List(items) => {
                    writeln!(f, "{key}:")?;
                    for item in items {
                        writeln!(f, "  - {item}")?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Strip a quoted scalar, or return the input trimmed.
fn unquote(raw: &str, line: usize) -> Result<String, FrontmatterError> {
    let trimmed = raw.trim();
    for quote in ['"', '\''] {
        if let Some(rest) = trimmed.strip_prefix(quote) {
            return match rest.strip_suffix(quote) {
                // `rest` must be non-empty after stripping, else the input was
                // a single lone quote character being read as both delimiters.
                Some(inner) if !rest.is_empty() => Ok(inner.to_owned()),
                _ => Err(FrontmatterError::UnterminatedQuote {
                    line,
                    value: trimmed.to_owned(),
                }),
            };
        }
    }
    Ok(trimmed.to_owned())
}

/// Reject constructs the subset does not cover, by name.
fn reject_unsupported(value: &str, line: usize) -> Result<(), FrontmatterError> {
    let v = value.trim();
    let construct = match v.chars().next() {
        Some('&') => "a YAML anchor",
        Some('*') => "a YAML alias",
        Some('!') => "a YAML tag",
        Some('{') => "a flow mapping",
        Some('[') => "a flow sequence",
        Some('|') => "a block scalar",
        Some('>') => "a folded block scalar",
        _ => return Ok(()),
    };
    Err(FrontmatterError::UnsupportedConstruct {
        line,
        construct: construct.to_owned(),
    })
}

/// Parse a document beginning with a `---` frontmatter fence.
pub fn parse(source: &str) -> Result<Frontmatter, FrontmatterError> {
    // Tolerate a UTF-8 BOM; an editor adding one should not make a valid atom
    // unreadable.
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);

    let mut cursor = 0usize;
    let mut line_no = 0usize;

    let mut lines = source.split_inclusive('\n');
    let first = lines.next().ok_or(FrontmatterError::MissingOpenFence)?;
    line_no += 1;
    if first.trim_end() != "---" {
        return Err(FrontmatterError::MissingOpenFence);
    }
    cursor += first.len();

    let mut entries: Vec<(String, Value)> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut closed = false;

    for raw in lines {
        line_no += 1;
        cursor += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let trimmed = line.trim();

        if trimmed == "---" {
            closed = true;
            break;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Block-sequence item, attaching to the most recent key.
        if let Some(item) = trimmed.strip_prefix("- ").or_else(|| {
            // A bare `-` with nothing after it is an empty item, not a key.
            (trimmed == "-").then_some("")
        }) {
            reject_unsupported(item, line_no)?;
            let value = unquote(item, line_no)?;
            match entries.last_mut() {
                Some((_, Value::List(items))) => items.push(value),
                Some((key, slot @ Value::Scalar(_))) => {
                    // Only an EMPTY scalar may become a list; a key with a real
                    // value followed by list items is ambiguous and refused.
                    if slot.as_scalar().is_some_and(str::is_empty) {
                        *slot = Value::List(vec![value]);
                    } else {
                        return Err(FrontmatterError::Malformed {
                            line: line_no,
                            found: format!(
                                "list item under key {key:?}, which already has a scalar value"
                            ),
                        });
                    }
                }
                None => {
                    return Err(FrontmatterError::OrphanListItem {
                        line: line_no,
                        item: value,
                    });
                }
            }
            continue;
        }

        // A `key: value` mapping. Indentation would mean nesting, which the
        // subset does not cover.
        let indent = line.len() - line.trim_start().len();
        if indent > 0 {
            return Err(FrontmatterError::UnsupportedConstruct {
                line: line_no,
                construct: "an indented (nested) mapping".to_owned(),
            });
        }

        let (key, rest) = trimmed
            .split_once(':')
            .ok_or_else(|| FrontmatterError::Malformed {
                line: line_no,
                found: trimmed.to_owned(),
            })?;
        let key = key.trim();
        if key.is_empty() || key.starts_with(['"', '\'']) {
            return Err(FrontmatterError::Malformed {
                line: line_no,
                found: trimmed.to_owned(),
            });
        }
        if !seen.insert(key.to_owned()) {
            return Err(FrontmatterError::DuplicateKey {
                line: line_no,
                key: key.to_owned(),
            });
        }
        reject_unsupported(rest, line_no)?;
        entries.push((key.to_owned(), Value::Scalar(unquote(rest, line_no)?)));
    }

    if !closed {
        return Err(FrontmatterError::UnclosedFence);
    }

    Ok(Frontmatter {
        entries,
        body_offset: cursor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str =
        "---\nschema: oh.war/atom/v1\nrole: intent\norder: 10\n---\n\n# Intent\n\nbody\n";

    #[test]
    fn parses_a_real_atom_header() {
        let fm = parse(SAMPLE).expect("valid");
        assert_eq!(fm.scalar("schema"), Some("oh.war/atom/v1"));
        assert_eq!(fm.scalar("role"), Some("intent"));
        assert_eq!(fm.scalar("order"), Some("10"));
        assert_eq!(&SAMPLE[fm.body_offset..], "\n# Intent\n\nbody\n");
    }

    #[test]
    fn preserves_key_order_and_unknown_keys() {
        let src = "---\na: 1\nx.custom: keep me\nb: 2\n---\n";
        let fm = parse(src).expect("valid");
        assert_eq!(fm.keys().collect::<Vec<_>>(), vec!["a", "x.custom", "b"]);
        assert_eq!(fm.scalar("x.custom"), Some("keep me"));
    }

    #[test]
    fn parses_block_sequences() {
        let src = "---\ngoverns:\n  - \"war://one\"\n  - war://two\n---\n";
        let fm = parse(src).expect("valid");
        assert_eq!(
            fm.list("governs"),
            Some(["war://one".to_owned(), "war://two".to_owned()].as_slice())
        );
    }

    #[test]
    fn values_may_contain_colons() {
        let src = "---\nref: war://01a0-19fc\nurl: https://example.test/a:b\n---\n";
        let fm = parse(src).expect("valid");
        assert_eq!(fm.scalar("url"), Some("https://example.test/a:b"));
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let src = "---\n# a comment\n\nrole: intent\n\n---\n";
        let fm = parse(src).expect("valid");
        assert_eq!(fm.len(), 1);
    }

    #[test]
    fn accepts_a_utf8_bom() {
        let src = "\u{feff}---\nrole: intent\n---\n";
        assert_eq!(parse(src).expect("valid").scalar("role"), Some("intent"));
    }

    // --- Planted violations: each construct must be REFUSED BY NAME. ---

    #[test]
    fn missing_open_fence_is_refused() {
        assert_eq!(
            parse("role: intent\n"),
            Err(FrontmatterError::MissingOpenFence)
        );
        assert_eq!(parse(""), Err(FrontmatterError::MissingOpenFence));
    }

    #[test]
    fn unclosed_fence_is_refused() {
        assert_eq!(
            parse("---\nrole: intent\n"),
            Err(FrontmatterError::UnclosedFence)
        );
    }

    #[test]
    fn duplicate_key_is_refused() {
        assert_eq!(
            parse("---\nrole: intent\nrole: basis\n---\n"),
            Err(FrontmatterError::DuplicateKey {
                line: 3,
                key: "role".to_owned()
            })
        );
    }

    /// The expansion-DoS vector OW-ADR-0002 exists to exclude.
    #[test]
    fn anchors_and_aliases_are_refused() {
        for (src, what) in [
            ("---\na: &anchor value\n---\n", "a YAML anchor"),
            ("---\na: *alias\n---\n", "a YAML alias"),
        ] {
            assert_eq!(
                parse(src),
                Err(FrontmatterError::UnsupportedConstruct {
                    line: 2,
                    construct: what.to_owned()
                }),
                "{src:?}"
            );
        }
    }

    #[test]
    fn tags_flow_collections_and_block_scalars_are_refused() {
        for (src, what) in [
            ("---\na: !!str 5\n---\n", "a YAML tag"),
            ("---\na: {b: 1}\n---\n", "a flow mapping"),
            ("---\na: [1, 2]\n---\n", "a flow sequence"),
            ("---\na: |\n---\n", "a block scalar"),
            ("---\na: >\n---\n", "a folded block scalar"),
        ] {
            assert_eq!(
                parse(src),
                Err(FrontmatterError::UnsupportedConstruct {
                    line: 2,
                    construct: what.to_owned()
                }),
                "{src:?}"
            );
        }
    }

    #[test]
    fn nested_mappings_are_refused() {
        assert_eq!(
            parse("---\nholder:\n  kind: git\n---\n"),
            Err(FrontmatterError::UnsupportedConstruct {
                line: 3,
                construct: "an indented (nested) mapping".to_owned()
            })
        );
    }

    #[test]
    fn a_list_item_before_any_key_is_refused() {
        assert_eq!(
            parse("---\n- orphan\n---\n"),
            Err(FrontmatterError::OrphanListItem {
                line: 2,
                item: "orphan".to_owned()
            })
        );
    }

    /// A list item under a key that already holds a scalar is ambiguous.
    ///
    /// The list branch is reached before the indentation check, so the
    /// diagnostic names the actual ambiguity rather than blaming the
    /// indentation. That ordering is deliberate: `role: intent` followed by
    /// `- item` is a different mistake from a nested mapping, and telling the
    /// author "indented mapping" here would send them to fix the wrong thing.
    #[test]
    fn a_list_under_a_non_empty_scalar_is_refused() {
        assert_eq!(
            parse("---\nrole: intent\n  - item\n---\n"),
            Err(FrontmatterError::Malformed {
                line: 3,
                found: "list item under key \"role\", which already has a scalar value".to_owned(),
            })
        );
    }

    /// The sibling case, proving the two really do get different diagnostics:
    /// an indented `key: value` is nesting, and says so.
    #[test]
    fn an_indented_mapping_is_reported_as_nesting_not_as_a_list_problem() {
        assert_eq!(
            parse("---\nholder:\n  kind: git\n---\n"),
            Err(FrontmatterError::UnsupportedConstruct {
                line: 3,
                construct: "an indented (nested) mapping".to_owned()
            })
        );
    }

    #[test]
    fn unterminated_quote_is_refused() {
        assert_eq!(
            parse("---\na: \"unterminated\n---\n"),
            Err(FrontmatterError::UnterminatedQuote {
                line: 2,
                value: "\"unterminated".to_owned()
            })
        );
    }

    #[test]
    fn a_line_without_a_colon_is_refused() {
        assert_eq!(
            parse("---\njust some prose\n---\n"),
            Err(FrontmatterError::Malformed {
                line: 2,
                found: "just some prose".to_owned()
            })
        );
    }

    /// A lone quote character must not be read as an empty quoted string.
    #[test]
    fn a_lone_quote_is_refused() {
        assert_eq!(
            parse("---\na: \"\n---\n"),
            Err(FrontmatterError::UnterminatedQuote {
                line: 2,
                value: "\"".to_owned()
            })
        );
    }
}
