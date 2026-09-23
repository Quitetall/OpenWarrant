// SPDX-License-Identifier: Apache-2.0

//! Is an authority record one a human actually signed?
//!
//! Until this module existed, the whole test for "this Warrant is authorized"
//! was four lines in [`crate::authorize::authorizes_current_contract`]: the
//! record says `authorized`, its digest matches the contract, and an
//! `[revision.authorization]` table is present. Every one of those three is a
//! string an agent can write. Demonstrated on 2026-09-19 against the published
//! alpha: a hand-written `authorization.toml` naming the owner as authorizer,
//! whose `meaning` field read "FORGED BY AN AGENT. No human saw this", produced
//! `war check` 0 errors and `PASS §56.1 exact authorized Contract Revision`.
//!
//! The signatures were already there — `war sign --ssh-sign` writes
//! `docs/authority/responses/<subject>.response.toml` with a `.sig` sidecar and
//! a DSSE attestation — and no rule on the READ path required one. A signature
//! nothing checks is decoration.
//!
//! What this asks, for one record:
//!
//! 1. a response exists for that subject, and a `.sig` beside it;
//! 2. `ssh-keygen -Y verify` accepts the signature over the response's exact
//!    bytes, under the `oh.war/response` namespace, against
//!    `docs/authority/allowed_signers`;
//! 3. the principal it verified as is the one `docs/authority/roles.toml` binds
//!    to the record's actor — so a valid signature by the wrong person is not a
//!    signature by that person;
//! 4. the actor is `human` in the register;
//! 5. the digest the response was signed over is the digest the record carries,
//!    so a signature cannot be moved onto a different contract.
//!
//! Every failure is fail-closed and named. A missing `ssh-keygen` is
//! [`Verdict::Unavailable`] rather than a pass: a tool that cannot check a
//! signature has not checked it, and §32's rule is that an unanswerable
//! question is unmet.
//!
//! The friction this deliberately does NOT add: an agent still drafts the
//! record, the request, the reason and the batch. The human act is a signature
//! over bytes already prepared, which is one confirmation per act and can be
//! none per act beyond loading the key. What an agent cannot do is skip it.

use camino::{Utf8Path, Utf8PathBuf};

use openwarrant_core::authority::ActorKind;

use crate::repo::Repository;

/// The namespace every response signature is made under. A signature made for
/// another namespace (`oh.war/dsse`, say) does not verify here, which is what
/// stops an attestation being replayed as an approval.
const RESPONSE_NAMESPACE: &str = "oh.war/response";

/// What kind of act a record claims, for the message and for the response name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Authorize,
    Resolve,
    Accept,
    Correct,
    /// A roadmap revision (OW-ADR-0023). Its own schema, so a SAS acceptance
    /// can never verify as a roadmap's or the reverse.
    AcceptRoadmap,
}

impl Act {
    /// The response schema this act's signature must have been made over.
    ///
    /// Load-bearing, not decoration. `war sign` names a response by SUBJECT,
    /// so an authorization and a resolution of one Warrant share the path
    /// `<alias>.response.toml`, and both bind `contract_digest`. Without this
    /// the authorization's signature would verify the resolution — the act that
    /// says "authorized to start" would silently also say "done".
    const fn response_schema(self) -> &'static str {
        match self {
            Self::Authorize => "oh.war/authorization-response/v1",
            Self::Resolve => "oh.war/resolution-response/v1",
            Self::Accept => "oh.war/sas-acceptance-response/v1",
            Self::Correct => "oh.war/correction-response/v1",
            Self::AcceptRoadmap => "oh.war/roadmap-acceptance-response/v1",
        }
    }

    const fn word(self) -> &'static str {
        match self {
            Self::Authorize => "authorization",
            Self::Resolve => "resolution",
            Self::Accept => "SAS acceptance",
            Self::Correct => "correction",
            Self::AcceptRoadmap => "roadmap acceptance",
        }
    }
}

/// The answer, with the reason attached to every unhappy case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// A human signature over these exact bytes verifies, by this principal.
    Signed { principal: String, response: String },
    /// No response, or no signature beside it.
    Unsigned { why: String },
    /// A signature exists and does not verify, or verifies as someone else.
    Invalid { why: String },
    /// The register does not bind the actor to a principal, or calls it an agent.
    NotHuman { why: String },
    /// `ssh-keygen` is absent. Not a pass.
    Unavailable { why: String },
}

impl Verdict {
    #[must_use]
    pub const fn is_signed(&self) -> bool {
        matches!(self, Self::Signed { .. })
    }

    /// The `war check` rule name this verdict reports under.
    #[must_use]
    pub const fn rule(&self) -> &'static str {
        match self {
            Self::Signed { .. } => "authority.signed",
            Self::Unsigned { .. } => "authority.unsigned",
            Self::Invalid { .. } => "authority.signature-invalid",
            Self::NotHuman { .. } => "authority.actor-not-human",
            Self::Unavailable { .. } => "authority.verify-unavailable",
        }
    }

    #[must_use]
    pub fn why(&self) -> String {
        match self {
            Self::Signed {
                principal,
                response,
            } => format!("signed by {principal} over {response}"),
            Self::Unsigned { why }
            | Self::Invalid { why }
            | Self::NotHuman { why }
            | Self::Unavailable { why } => why.clone(),
        }
    }
}

/// The name a signed response for one act of one subject carries.
///
/// One file per act, because two acts sharing one name collide: an
/// authorization and a resolution of the same Warrant both bind
/// `contract_digest`, and while both were called `<alias>.response.toml`
/// whichever signed second retired the first — a batch signing 29 recorded
/// resolutions refused all 29 with `sign.response-exists` against the
/// authorization already sitting at that path. Authorizations keep the bare
/// name, because 50 of them are committed under it; every other act says which
/// act it is.
#[must_use]
pub fn response_stem(act: Act, subject: &str) -> String {
    match act {
        Act::Authorize | Act::Accept | Act::AcceptRoadmap => subject.to_owned(),
        Act::Resolve => format!("{subject}.resolution"),
        Act::Correct => format!("{subject}.correction"),
    }
}

/// Where the signed response for one act of one subject lives.
#[must_use]
pub fn response_path(repo: &Repository, act: Act, subject: &str) -> Utf8PathBuf {
    repo.root
        .join("docs/authority/responses")
        .join(format!("{}.response.toml", response_stem(act, subject)))
}

/// Verify that a human signed this act over these bytes.
///
/// `subject` is the alias, `<alias>/<D-id>` or SAS version the response is named
/// for; `actor` is the name the record claims signed it; `bound_digest` is the
/// digest the record says was signed — the contract digest for an authorization
/// or resolution, the new digest for a correction, the document sha256 for an
/// acceptance.
pub fn verify(
    repo: &Repository,
    act: Act,
    subject: &str,
    actor: &str,
    bound_digest: Option<&str>,
) -> Verdict {
    verify_excluding(repo, act, subject, actor, bound_digest, None)
}

/// [`verify`], ignoring one response file.
///
/// `war sign` writes and signs the response BEFORE handing it to ingest, so a
/// caller asking "was this record already signed by some earlier act?" must
/// exclude the response it is about to ingest — otherwise it sees its own
/// fresh signature and concludes the work is done. That mistake cost 57
/// refusals in one run of `war sign --all --ssh-sign`, and the parameter is
/// here so no caller has to thread the answer through by hand.
pub fn verify_excluding(
    repo: &Repository,
    act: Act,
    subject: &str,
    actor: &str,
    bound_digest: Option<&str>,
    exclude: Option<&Utf8Path>,
) -> Verdict {
    // Every response whose name starts with this subject: the current one and
    // any retired sibling. `war sign` retires a superseded response by renaming
    // it to carry its digest (§34.4 — supersede, never erase), so the signature
    // for an earlier act of the same subject lives under a different name.
    let candidates = candidate_responses(repo, act, subject, exclude);
    if candidates.is_empty() {
        return Verdict::Unsigned {
            why: format!(
                "the {} record names {actor} as its actor and no signed {} response exists under \
                 docs/authority/responses/ for {subject}. An authority record is believed because \
                 a human signed it, not because a file says so — draft the act and run \
                 `war sign {subject} --ssh-sign`",
                act.word(),
                act.response_schema()
            ),
        };
    }

    // The register decides who may sign and under which principal. It is
    // human-written and committed; an agent editing it is a commit a reviewer
    // sees, which is the control §27.2 relies on.
    let register = match repo.load_authority_register() {
        Ok(r) => r,
        Err(e) => {
            return Verdict::Unavailable {
                why: format!("docs/authority/roles.toml could not be read: {e}"),
            };
        }
    };
    let Some(assignment) = register.assignments.iter().find(|a| a.actor == actor) else {
        return Verdict::NotHuman {
            why: format!(
                "{actor} has no assignment in docs/authority/roles.toml, so nothing says this \
                 actor may sign anything"
            ),
        };
    };
    if assignment.actor_kind != ActorKind::Human {
        return Verdict::NotHuman {
            why: format!(
                "{actor} is {} in the register; §27.2 reserves this act for a human",
                assignment.actor_kind
            ),
        };
    }
    let Some(principal) = assignment.ssh_principal.clone() else {
        return Verdict::NotHuman {
            why: format!(
                "{actor} has no `ssh_principal` in docs/authority/roles.toml, so a signature \
                 cannot be tied to this actor"
            ),
        };
    };

    // Each candidate must be a response for THIS act, over THIS digest, with a
    // signature that verifies as THIS principal. The first that satisfies all
    // three is the signature; otherwise the strongest reason is reported.
    let mut strongest: Option<Verdict> = None;
    let primary = response_path(repo, act, subject);
    for response in &candidates {
        let text = match std::fs::read_to_string(response) {
            Ok(t) => t,
            Err(e) => {
                strongest = Some(Verdict::Unavailable {
                    why: format!("{} could not be read: {e}", repo.relative(response)),
                });
                continue;
            }
        };
        if !text.contains(act.response_schema()) {
            // A response for a different act of the same subject. Not a reason
            // to complain: an authorization response sits beside a resolution's.
            continue;
        }
        if let Some(want) = bound_digest {
            let bare = want.trim_start_matches("sha256:");
            if !text.contains(bare) {
                // A retired sibling binds the digest it was signed over, which
                // is by definition not the current one — §34.4 keeps it rather
                // than erasing it, and finding it here is expected, not a
                // finding. The CURRENT response failing to mention the current
                // digest is a different thing: that file is what this act
                // offers as its signature, and it is over other bytes.
                if *response == primary {
                    strongest = Some(Verdict::Invalid {
                        why: format!(
                            "{} is a {} response and does not mention the digest this record \
                             binds ({bare}); a signature over other bytes is not a signature \
                             over this act",
                            repo.relative(response),
                            act.word()
                        ),
                    });
                }
                continue;
            }
        }
        let sig = Utf8PathBuf::from(format!("{response}.sig"));
        if !sig.is_file() {
            // OW-WAR-0072: a response signed as part of a batch carries no
            // `.sig` of its own. It is signed when a batch whose signer is
            // this record's actor, and whose signature verifies as their
            // principal, lists this response by name and exact sha256.
            // Nothing in the record claims the batch; the batch claims it.
            match batch_covering(repo, &principal, actor, response) {
                Some(v @ (Verdict::Signed { .. } | Verdict::Unavailable { .. })) => return v,
                Some(v) => {
                    strongest = Some(v);
                    continue;
                }
                None => {}
            }
            strongest = Some(Verdict::Unsigned {
                why: format!(
                    "{} carries no `.sig` sidecar, so nothing signed it. A response written by \
                     hand is a draft, whatever its fields claim",
                    repo.relative(response)
                ),
            });
            continue;
        }
        match ssh_verify(repo, &principal, response, &sig) {
            Ok(()) => {
                return Verdict::Signed {
                    principal,
                    response: repo.relative(response),
                };
            }
            Err(SshFailure::Unavailable(why)) => return Verdict::Unavailable { why },
            Err(SshFailure::Rejected(why)) => strongest = Some(Verdict::Invalid { why }),
        }
    }
    strongest.unwrap_or_else(|| Verdict::Unsigned {
        why: format!(
            "no {} response for {subject} exists under docs/authority/responses/ — {} file(s) \
             there carry this subject's name and none is a {} signed by {actor}. Draft the act \
             and run `war sign {subject} --ssh-sign`",
            act.word(),
            candidates.len(),
            act.response_schema()
        ),
    })
}

/// Every response file whose name belongs to this subject, current or retired.
///
/// Retired siblings matter because `war sign` renames a superseded response to
/// carry its digest rather than deleting it, and because one subject's
/// authorization and resolution share a path — whichever act signs second
/// pushes the first aside, and the first is still a signature.
fn candidate_responses(
    repo: &Repository,
    act: Act,
    subject: &str,
    exclude: Option<&Utf8Path>,
) -> Vec<Utf8PathBuf> {
    let dir = repo.root.join("docs/authority/responses");
    let primary = response_path(repo, act, subject);
    let excluded = |p: &Utf8Path| exclude.is_some_and(|e| e == p);
    let mut out = Vec::new();
    if primary.is_file() && !excluded(&primary) {
        out.push(primary.clone());
    }
    if let Ok(entries) = dir.read_dir_utf8() {
        for e in entries.flatten() {
            let path = e.path().to_owned();
            let Some(name) = path.file_name() else {
                continue;
            };
            if path == primary
                || excluded(&path)
                || !name.starts_with(subject)
                || !name.ends_with(".response.toml")
            {
                continue;
            }
            out.push(path);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The batch that signs `response`, if one does: its signer is `actor`, it
/// lists the response's file name and exact sha256, and its signature
/// verifies as `principal`. `Some(Invalid)` when a listing batch's signature
/// fails; `None` when no batch lists these bytes.
fn batch_covering(
    repo: &Repository,
    principal: &str,
    actor: &str,
    response: &Utf8Path,
) -> Option<Verdict> {
    let name = response.file_name()?;
    let bytes = std::fs::read(response).ok()?;
    let digest = openwarrant_compiler::sha256_hex(&bytes);
    let mut failed = None;
    for (path, batch) in crate::batch_cmd::load_all(repo) {
        if batch.signer != actor || batch.covers(name, &digest).is_none() {
            continue;
        }
        let sig = Utf8PathBuf::from(format!("{path}.sig"));
        if !sig.is_file() {
            continue;
        }
        match ssh_verify(repo, principal, &path, &sig) {
            Ok(()) => {
                return Some(Verdict::Signed {
                    principal: principal.to_owned(),
                    response: format!("{} (in batch {})", repo.relative(response), batch.batch_id),
                });
            }
            Err(SshFailure::Unavailable(why)) => return Some(Verdict::Unavailable { why }),
            Err(SshFailure::Rejected(why)) => failed = Some(Verdict::Invalid { why }),
        }
    }
    failed
}

enum SshFailure {
    /// `ssh-keygen` could not be run at all.
    Unavailable(String),
    /// It ran and refused.
    Rejected(String),
}

/// `ssh-keygen -Y verify` over the response's exact bytes.
///
/// `-I <principal>` is not optional: without it any key in `allowed_signers`
/// satisfies the check, which would make "signed by the owner" mean "signed by
/// somebody this repository has heard of".
fn ssh_verify(
    repo: &Repository,
    principal: &str,
    response: &Utf8Path,
    sig: &Utf8Path,
) -> Result<(), SshFailure> {
    let allowed = repo.root.join("docs/authority/allowed_signers");
    if !allowed.is_file() {
        return Err(SshFailure::Unavailable(format!(
            "{} does not exist, so no key is allowed to sign anything",
            repo.relative(&allowed)
        )));
    }
    let input = std::fs::File::open(response).map_err(|e| {
        SshFailure::Unavailable(format!("could not open {}: {e}", repo.relative(response)))
    })?;
    let out = std::process::Command::new("ssh-keygen")
        .args(["-Y", "verify", "-f"])
        .arg(&allowed)
        .args(["-I", principal, "-n", RESPONSE_NAMESPACE, "-s"])
        .arg(sig)
        .stdin(input)
        .output()
        .map_err(|e| {
            SshFailure::Unavailable(format!(
                "ssh-keygen could not be run ({e}), so this signature is unchecked. An unchecked \
                 signature is not a pass"
            ))
        })?;
    if out.status.success() {
        return Ok(());
    }
    Err(SshFailure::Rejected(format!(
        "the signature on {} does not verify as {principal} under {RESPONSE_NAMESPACE}: {}",
        repo.relative(response),
        String::from_utf8_lossy(&out.stderr).trim()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_response_name_follows_its_act() {
        // Deliberately asserted, because the enforcement finds a signature by
        // guessing this path: a rename here silently becomes "unsigned".
        let repo = crate::repo::Repository::discover(None);
        if let Ok(repo) = repo {
            let a = response_path(&repo, Act::Authorize, "OW-WAR-0063");
            assert!(a.as_str().ends_with("OW-WAR-0063.response.toml"), "{a}");
            let c = response_path(&repo, Act::Correct, "OW-WAR-0055.D-001");
            assert!(
                c.as_str()
                    .ends_with("OW-WAR-0055.D-001.correction.response.toml"),
                "{c}"
            );
        }
    }

    #[test]
    fn every_unhappy_verdict_carries_its_reason_and_rule() {
        for v in [
            Verdict::Unsigned { why: "u".into() },
            Verdict::Invalid { why: "i".into() },
            Verdict::NotHuman { why: "n".into() },
            Verdict::Unavailable { why: "a".into() },
        ] {
            assert!(!v.is_signed(), "{v:?}");
            assert!(!v.why().is_empty(), "{v:?}");
            assert!(v.rule().starts_with("authority."), "{v:?}");
        }
        let ok = Verdict::Signed {
            principal: "brian".into(),
            response: "docs/authority/responses/X.response.toml".into(),
        };
        assert!(ok.is_signed());
        assert_eq!(ok.rule(), "authority.signed");
        assert!(ok.why().contains("brian"));
    }
}
