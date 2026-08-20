// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war show` — render any §17.5 projection, and `war diff` (§71.10).
//!
//! # Why `show` exists at all
//!
//! §17.5 requires the compiler to SUPPORT nine projections. Only two are
//! committed to disk, because emitting all nine per Warrant would put 360
//! drift-checked files in a 40-Warrant repository to serve views a reader wants
//! occasionally. `war show` is how the other seven are reachable, which is what
//! makes "supported" true rather than aspirational.
//!
//! # What `war diff` compares
//!
//! §71.10 asks for the *semantic* difference between revisions or Bases. So the
//! comparison runs over the canonical IR, not over rendered Markdown: §91.1 test
//! 3 requires that changing how a document renders cannot change its meaning, and
//! a differ that reported reflowed prose as a semantic change would contradict
//! that on its first run.

use camino::Utf8PathBuf;
use openwarrant_compiler::render::{self, View};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// `war show <alias> --view <name>`.
pub fn run(repo: &Repository, alias: &str, view_name: &str) -> Result<String, RepoError> {
    let Some(view) = View::parse(view_name) else {
        let known = View::ALL
            .iter()
            .map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(RepoError::Message(format!(
            "unknown view {view_name:?}; §17.5 defines {known}"
        )));
    };

    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(RepoError::Message(format!(
            "{alias} could not be compiled, so it has no projections to show"
        )));
    };
    let ir = openwarrant_compiler::lower::lower(basis, validated)
        .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?;

    render::render_view(view, &ir, basis)
        .map_err(|e| RepoError::Message(format!("{alias}: {view} did not render: {e}")))?
        .ok_or_else(|| {
            RepoError::Message(format!(
                "`{view}` is a projection of the ADR corpus rather than of one \
                 Warrant; see the generated ADR overview"
            ))
        })
}

/// One semantic difference between two compilations (§71.10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticChange {
    pub path: String,
    pub from: String,
    pub to: String,
}

/// Compare two canonical IRs and report what actually differs.
///
/// Walks the canonical JSON trees rather than the rendered text. Two
/// compilations whose Markdown differs only in wrapping produce no changes here,
/// and that is the property that makes the output worth reading.
#[must_use]
pub fn semantic_diff(from: &serde_json::Value, to: &serde_json::Value) -> Vec<SemanticChange> {
    let mut out = Vec::new();
    walk("", from, to, &mut out);
    out
}

fn walk(path: &str, a: &serde_json::Value, b: &serde_json::Value, out: &mut Vec<SemanticChange>) {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            // Union of keys, so a field added or removed is reported rather than
            // skipped — a differ that only walks the left side cannot see an
            // addition.
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort_unstable();
            keys.dedup();
            for k in keys {
                let child = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                match (x.get(k), y.get(k)) {
                    (Some(av), Some(bv)) => walk(&child, av, bv, out),
                    (Some(av), None) => out.push(SemanticChange {
                        path: child,
                        from: compact(av),
                        to: "*absent*".to_owned(),
                    }),
                    (None, Some(bv)) => out.push(SemanticChange {
                        path: child,
                        from: "*absent*".to_owned(),
                        to: compact(bv),
                    }),
                    (None, None) => {}
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            let len = x.len().max(y.len());
            for i in 0..len {
                let child = format!("{path}[{i}]");
                match (x.get(i), y.get(i)) {
                    (Some(av), Some(bv)) => walk(&child, av, bv, out),
                    (Some(av), None) => out.push(SemanticChange {
                        path: child,
                        from: compact(av),
                        to: "*absent*".to_owned(),
                    }),
                    (None, Some(bv)) => out.push(SemanticChange {
                        path: child,
                        from: "*absent*".to_owned(),
                        to: compact(bv),
                    }),
                    (None, None) => {}
                }
            }
        }
        _ if a == b => {}
        _ => out.push(SemanticChange {
            path: path.to_owned(),
            from: compact(a),
            to: compact(b),
        }),
    }
}

/// Shorten a value for display, on a CHARACTER boundary.
///
/// `&s[..119]` panics when byte 119 lands inside a multi-byte character, and
/// this corpus is full of `§` and `—`. A differ that crashes on the documents it
/// exists to compare is worse than one that prints a long line.
fn compact(v: &serde_json::Value) -> String {
    const MAX_CHARS: usize = 119;
    let s = v.to_string();
    // Single pass: find the byte offset of character MAX_CHARS, if there is one.
    // `chars().count()` would walk the whole string just to decide whether to
    // walk it again, which matters on the pathological long value.
    match s.char_indices().nth(MAX_CHARS) {
        None => s,
        Some((byte_offset, _)) => format!("{}…", &s[..byte_offset]),
    }
}

/// `war diff <alias> --from <file> --to <file>`, over canonical JSON.
pub fn diff(
    repo: &Repository,
    alias: &str,
    from_path: Option<&Utf8PathBuf>,
) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(RepoError::Message(format!("{alias} could not be compiled")));
    };
    let ir = openwarrant_compiler::lower::lower(basis, validated)
        .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?;
    let fresh = openwarrant_compiler::render::canonical_json(&ir)
        .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?;

    // Default comparison: the committed canonical JSON against a fresh
    // compilation — which is the difference an author most often wants, and the
    // same pair `war check --generated` reports as drift.
    let baseline_path = from_path
        .cloned()
        .unwrap_or_else(|| dir.join("generated/WAR.json"));
    let baseline = std::fs::read_to_string(&baseline_path).map_err(|_| {
        RepoError::Message(format!("cannot read {baseline_path} to compare against"))
    })?;

    let mut report = Report::default();
    let from: serde_json::Value = serde_json::from_str(&baseline)
        .map_err(|e| RepoError::Message(format!("{baseline_path}: {e}")))?;
    let to: serde_json::Value = serde_json::from_str(&fresh)
        .map_err(|e| RepoError::Message(format!("fresh compilation: {e}")))?;

    let changes = semantic_diff(&from, &to);
    if changes.is_empty() {
        report.push(Diagnostic::pass(
            "diff.identical",
            format!("{alias}: no semantic difference"),
        ));
    } else {
        for c in &changes {
            report.push(Diagnostic::warn(
                "diff.changed",
                repo.relative(&baseline_path),
                format!("{}: {} -> {}", c.path, c.from, c.to),
            ));
        }
        report.note(format!(
            "{} semantic change(s). Rendering differences do not appear here: the \
             comparison is over the canonical IR, because §91.1 test 3 requires \
             that how a document renders cannot change what it means",
            changes.len()
        ));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A differ that only walks the left side cannot see an addition.
    #[test]
    fn an_added_field_is_reported() {
        let changes = semantic_diff(&json!({"a": 1}), &json!({"a": 1, "b": 2}));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "b");
        assert_eq!(changes[0].from, "*absent*");
        assert_eq!(changes[0].to, "2");
    }

    #[test]
    fn a_removed_field_is_reported() {
        let changes = semantic_diff(&json!({"a": 1, "b": 2}), &json!({"a": 1}));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "b");
        assert_eq!(changes[0].to, "*absent*");
    }

    #[test]
    fn identical_documents_produce_no_changes() {
        let doc = json!({"a": [1, 2, {"b": "c"}], "d": null});
        assert!(semantic_diff(&doc, &doc).is_empty());
    }

    #[test]
    fn nested_changes_are_reported_by_path() {
        let changes = semantic_diff(
            &json!({"identity": {"assurance_level": "basic"}}),
            &json!({"identity": {"assurance_level": "controlled"}}),
        );
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "identity.assurance_level");
        assert_eq!(changes[0].from, "\"basic\"");
        assert_eq!(changes[0].to, "\"controlled\"");
    }

    #[test]
    fn array_changes_are_reported_by_index() {
        let changes = semantic_diff(&json!({"a": [1, 2]}), &json!({"a": [1, 3, 4]}));
        let paths: Vec<&str> = changes.iter().map(|c| c.path.as_str()).collect();
        assert!(paths.contains(&"a[1]"), "{paths:?}");
        assert!(paths.contains(&"a[2]"), "{paths:?}");
    }

    /// Long values are truncated so one huge field cannot bury the rest of the
    /// report — but the path is always shown in full, because the path is what a
    /// reader acts on.
    ///
    /// The truncation is by CHARACTER, not by byte: this corpus is full of `§`
    /// and `—`, and slicing at a byte offset inside one of those panics.
    #[test]
    fn long_values_are_truncated_but_paths_are_not() {
        let long = "x".repeat(500);
        let changes = semantic_diff(
            &json!({"deeply": {"nested": {"field": "short"}}}),
            &json!({"deeply": {"nested": {"field": long}}}),
        );
        assert_eq!(changes[0].path, "deeply.nested.field");
        assert_eq!(
            changes[0].to.chars().count(),
            120,
            "119 chars plus an ellipsis"
        );
        assert!(changes[0].to.ends_with('…'));
    }

    /// §17.5's nine parse, and nothing else does.
    /// Regression: truncating multi-byte text must not panic. Slicing `&s[..119]`
    /// crashes when byte 119 falls inside a `§` or `—`, which every document in
    /// this repository contains.
    #[test]
    fn truncation_does_not_split_a_multibyte_character() {
        for filler in ["§", "—", "é", "🔒"] {
            let long = filler.repeat(400);
            let changes = semantic_diff(&json!({"f": "short"}), &json!({"f": long}));
            assert_eq!(changes.len(), 1);
            assert!(changes[0].to.ends_with('…'));
            // A boundary-crossing slice would have panicked before reaching here.
        }
        // ...and at exactly the boundary length.
        for n in 115..125 {
            let s = "§".repeat(n);
            let _ = semantic_diff(&json!({"f": ""}), &json!({"f": s}));
        }
    }

    #[test]
    fn every_sas_view_name_parses() {
        for v in View::ALL {
            assert_eq!(View::parse(v.as_str()), Some(v));
        }
        assert_eq!(View::parse("pretty_print"), None);
        assert_eq!(View::ALL.len(), 9);
    }

    /// Only the two configured views are written to disk; the rest are
    /// renderable. Both halves matter: "supported" must be true, and the
    /// repository must not carry 360 generated files.
    #[test]
    fn exactly_two_views_are_committed_and_the_rest_are_renderable() {
        let committed: Vec<&str> = View::ALL
            .iter()
            .filter(|v| v.is_committed())
            .map(|v| v.as_str())
            .collect();
        assert_eq!(committed, ["full_warrant", "canonical_json"]);
        for v in View::ALL {
            assert_eq!(
                v.is_committed(),
                v.filename().is_some(),
                "{v}: is_committed and filename disagree"
            );
        }
    }
}

/// `war plan` — §71.3's planning entry point, over §75.2's process seam.
///
/// # What this does and does not do
///
/// §75.2 defines the seam as: one canonical JSON request on stdin to the agent,
/// one canonical Draft Proposal on stdout back. This implements OpenWarrant's
/// half — building the request, and validating and gating the proposal — and
/// does NOT include an agent. Without `--agent`, it emits the request and stops,
/// which is the honest behaviour for a seam whose other side is absent.
///
/// §74.5 is why the gating half matters more than the invoking half: the model
/// never writes files, so everything a planner can do has to survive
/// [`ApplicationPipeline`] first.
pub mod plan {
    use openwarrant_core::drafting::{APPLICATION_STEPS, ApplicationPipeline, DraftProposal};

    use crate::repo::RepoError;

    /// The request handed to a drafting agent (§74.1, §75.2).
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct DraftRequest {
        pub api_version: String,
        pub user_request: String,
        pub namespace: String,
        pub profile: String,
        pub assurance: String,
        #[serde(default)]
        pub existing_warrants: Vec<String>,
        #[serde(default)]
        pub existing_adrs: Vec<String>,
    }

    /// Validate a Draft Proposal and decide whether it may be applied.
    ///
    /// Runs §74.4's first four steps here — parse, schema, semantic references,
    /// risk and authority — and leaves steps 5 and 6 (show a diff, require
    /// review) to the caller, because they need a human. `may_apply` then
    /// refuses while either is outstanding, so `--apply` cannot skip the review
    /// that §74.4 requires.
    pub fn validate_proposal(
        json: &str,
        reviewed: bool,
    ) -> Result<(DraftProposal, ApplicationPipeline), RepoError> {
        let mut pipeline = ApplicationPipeline::default();

        let proposal: DraftProposal = serde_json::from_str(json)
            .map_err(|e| RepoError::Message(format!("draft proposal did not parse: {e}")))?;
        pipeline.complete(APPLICATION_STEPS[0]);

        if proposal.api_version != "oh.war/draft-proposal/v1" {
            return Err(RepoError::Message(format!(
                "draft proposal declares api_version {:?}; this build understands \
                 oh.war/draft-proposal/v1",
                proposal.api_version
            )));
        }
        pipeline.complete(APPLICATION_STEPS[1]);

        proposal
            .validate()
            .map_err(|e| RepoError::Message(format!("{e}")))?;
        pipeline.complete(APPLICATION_STEPS[2]);
        pipeline.complete(APPLICATION_STEPS[3]);

        // §74.4 steps 5 and 6 need a human. Recording them only when review has
        // actually happened is what makes `may_apply` mean something.
        if reviewed {
            pipeline.complete(APPLICATION_STEPS[4]);
            pipeline.complete(APPLICATION_STEPS[5]);
        }
        Ok((proposal, pipeline))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const MINIMAL: &str = r#"{"api_version":"oh.war/draft-proposal/v1"}"#;

        /// §74.4 — an unreviewed proposal cannot be applied, however valid.
        #[test]
        fn a_valid_but_unreviewed_proposal_cannot_be_applied() {
            let (_, pipeline) = validate_proposal(MINIMAL, false).expect("valid");
            assert!(
                pipeline.may_apply().is_err(),
                "a proposal was applicable without the review §74.4 requires"
            );

            let (_, reviewed) = validate_proposal(MINIMAL, true).expect("valid");
            assert_eq!(reviewed.may_apply(), Ok(()));
        }

        #[test]
        fn a_proposal_from_another_protocol_version_is_refused() {
            let other = r#"{"api_version":"oh.war/draft-proposal/v2"}"#;
            assert!(validate_proposal(other, true).is_err());
        }

        #[test]
        fn a_malformed_proposal_is_refused_at_the_parse_step() {
            assert!(validate_proposal("{not json", true).is_err());
        }

        /// §74.7 travels through the seam: a buried durable choice is refused
        /// here too, not only in core.
        #[test]
        fn a_buried_durable_choice_is_refused_at_the_seam() {
            let buried = r#"{
                "api_version":"oh.war/draft-proposal/v1",
                "durable_choices":[{"statement":"format choice","alternatives":["a","b"],
                                    "proposed_adr_draft":""}]
            }"#;
            let err = validate_proposal(buried, true).unwrap_err().to_string();
            assert!(err.contains("durable alternatives"), "{err}");
        }
    }
}
