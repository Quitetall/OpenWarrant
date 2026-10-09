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
//! # With a protected store (OW-WAR-0138 D-002)
//!
//! CONTINGENT on the owner answering OW-WAR-0138 U-001 option A. A
//! repository opts in with `[authority] store` in `openwarrant.toml`. Then
//! steps 2-4 read the store's current head, not the working tree: the actor's
//! principal, its kind and its role for the act come from the v2 revision
//! there, and the signature must verify against the key the store binds.
//! `roles.toml` and `allowed_signers` grant nothing. A record that verifies
//! only against them is [`Verdict::LegacyFallback`], an error naming the
//! attack it is: a key and a "human" added to two working-tree files. A store
//! that cannot be read is [`Verdict::Unavailable`], never a fall back. With no
//! `[authority]` table nothing here changes, and `war check` warns
//! `authority.unprotected` once ([`protection_report`]).
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
    /// A Gate Definition version invalidated (OW-WAR-0136, §45). Subject
    /// `<gate_id>@<version>`, bound to the definition file's digest.
    Invalidate,
    /// OW-ADR-0029 — a human signs a standing authorization's class, over
    /// the class file's exact sha256.
    AcceptStanding,
    /// OW-ADR-0029 — a human revokes a signed class.
    RevokeStanding,
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
            Self::Invalidate => "oh.war/invalidation-response/v1",
            Self::AcceptStanding => "oh.war/standing-acceptance-response/v1",
            Self::RevokeStanding => "oh.war/standing-revocation-response/v1",
        }
    }

    const fn word(self) -> &'static str {
        match self {
            Self::Authorize => "authorization",
            Self::Resolve => "resolution",
            Self::Accept => "SAS acceptance",
            Self::Correct => "correction",
            Self::AcceptRoadmap => "roadmap acceptance",
            Self::Invalidate => "gate invalidation",
            Self::AcceptStanding => "standing authorization",
            Self::RevokeStanding => "standing revocation",
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
    /// `ssh-keygen` is absent, or a configured store cannot be read. Not a pass.
    Unavailable { why: String },
    /// OW-WAR-0138: a store governs, and the actor holds no grant there for
    /// this act (the role was removed, or never given).
    NotGranted { why: String },
    /// OW-WAR-0138: a store governs, and the signature verifies only against
    /// the working-tree register. The register grants nothing when a store is
    /// configured; this is the verdict that names an agent's own key made to
    /// verify as a human by editing `roles.toml` and `allowed_signers`.
    LegacyFallback { why: String },
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
            Self::NotGranted { .. } => "authority.not-granted",
            Self::LegacyFallback { .. } => "authority.legacy-fallback",
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
            | Self::Unavailable { why }
            | Self::NotGranted { why }
            | Self::LegacyFallback { why } => why.clone(),
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
        Act::Authorize | Act::Accept | Act::AcceptRoadmap | Act::AcceptStanding => {
            subject.to_owned()
        }
        Act::Resolve => format!("{subject}.resolution"),
        Act::Correct => format!("{subject}.correction"),
        Act::Invalidate => format!("{subject}.invalidation"),
        Act::RevokeStanding => format!("{subject}.revocation"),
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
    let verdict = verify_responses(repo, act, subject, actor, bound_digest, exclude);
    // OW-ADR-0029: an authorization no response signs may still be one a
    // human signed — through a class. Asked only when the ordinary answer is
    // not `Signed`, and only of a record whose `policy_basis` names a class;
    // every other record gets the ordinary verdict unchanged. The covered
    // verdict is re-derived on every call and never cached: it depends on
    // the class file, its signed responses, the Warrant's atoms and records
    // and every other covered record (the count), and a key that left one of
    // those out would answer from a class that has since moved. The ssh
    // verdicts underneath it go through `ssh_verify`'s memo as before.
    if act == Act::Authorize
        && !verdict.is_signed()
        && let Some(covered) =
            crate::standing_cmd::covered_verdict(repo, subject, actor, bound_digest)
    {
        return covered;
    }
    verdict
}

/// [`verify_excluding`] over signed responses and batches only.
fn verify_responses(
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

    match binding(repo) {
        Binding::Register => verify_register(repo, act, subject, actor, bound_digest, &candidates),
        Binding::Failed { why, .. } => Verdict::Unavailable { why },
        Binding::Store(store) => {
            let verdict =
                verify_store(repo, &store, act, subject, actor, bound_digest, &candidates);
            // A grant the store withdrew, or a store it cannot read, is its
            // own answer. Otherwise: would the working-tree register have
            // believed it? That is exactly the fallback a store exists to
            // refuse, and it is named rather than reported as an ordinary bad
            // signature or an unknown actor.
            if verdict.is_signed()
                || matches!(
                    verdict,
                    Verdict::Unavailable { .. } | Verdict::NotGranted { .. }
                )
            {
                return verdict;
            }
            if verify_register(repo, act, subject, actor, bound_digest, &candidates).is_signed() {
                return Verdict::LegacyFallback {
                    why: format!(
                        "the {} for {subject} verifies against docs/authority/roles.toml and \
                         allowed_signers only. {} governs this repository, and there: {} The \
                         working-tree register grants nothing while a store is configured \
                         (OW-WAR-0138); a key and a human entry added to those two files are not \
                         a grant",
                        act.word(),
                        store.describe(),
                        verdict.why()
                    ),
                };
            }
            verdict
        }
    }
}

/// The legacy path: `roles.toml` binds the actor, `allowed_signers` holds the
/// key. Used when no store is configured, and to name a fallback when one is.
fn verify_register(
    repo: &Repository,
    act: Act,
    subject: &str,
    actor: &str,
    bound_digest: Option<&str>,
    candidates: &[Utf8PathBuf],
) -> Verdict {
    // The register decides who may sign and under which principal. It is
    // human-written and committed; an agent editing it is a commit a reviewer
    // sees, which is the control §27.2 relies on.
    let register = match repo.load_legacy_authority_register() {
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
    let trust = Trust {
        principal,
        allowed: repo.root.join("docs/authority/allowed_signers"),
        label: "register".to_owned(),
        via: String::new(),
    };
    verify_candidates(repo, &trust, act, subject, actor, bound_digest, candidates)
}

/// The store path: the store's current head binds the actor to a principal
/// and a key, says the actor is human, and grants the act's role.
fn verify_store(
    repo: &Repository,
    store: &StoreBinding,
    act: Act,
    subject: &str,
    actor: &str,
    bound_digest: Option<&str>,
    candidates: &[Utf8PathBuf],
) -> Verdict {
    let (principal, allowed) = match store.signer(actor, &[act]) {
        Ok(s) => s,
        Err(v) => return v,
    };
    let trust = Trust {
        principal,
        allowed: allowed.0.clone(),
        label: format!(
            "store\0{}\0{}\0{}",
            store.store, store.head, store.test_mode
        ),
        via: format!(" — bound by {}", store.describe()),
    };
    verify_candidates(repo, &trust, act, subject, actor, bound_digest, candidates)
}

/// Who a signature must verify as, and against which key list. `label`
/// names the source in the verdict memo's key; `via` in the verdict.
struct Trust {
    principal: String,
    allowed: Utf8PathBuf,
    label: String,
    via: String,
}

/// Each candidate must be a response for THIS act, over THIS digest, with a
/// signature that verifies as THIS principal. The first that satisfies all
/// three is the signature; otherwise the strongest reason is reported.
fn verify_candidates(
    repo: &Repository,
    trust: &Trust,
    act: Act,
    subject: &str,
    actor: &str,
    bound_digest: Option<&str>,
    candidates: &[Utf8PathBuf],
) -> Verdict {
    let principal = &trust.principal;
    let mut strongest: Option<Verdict> = None;
    let primary = response_path(repo, act, subject);
    for response in candidates {
        let text = match crate::vfs::read_to_string(response) {
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
        if !crate::vfs::is_file(&sig) {
            // OW-WAR-0072: a response signed as part of a batch carries no
            // `.sig` of its own. It is signed when a batch whose signer is
            // this record's actor, and whose signature verifies as their
            // principal, lists this response by name and exact sha256.
            // Nothing in the record claims the batch; the batch claims it.
            match batch_covering(repo, trust, actor, response) {
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
        match ssh_verify(repo, trust, response, &sig) {
            Ok(()) => {
                return Verdict::Signed {
                    principal: principal.clone(),
                    response: format!("{}{}", repo.relative(response), trust.via),
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
    if crate::vfs::is_file(&primary) && !excluded(&primary) {
        out.push(primary.clone());
    }
    for path in response_files(&dir) {
        {
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

/// The files under the responses directory. Listed once per read-only
/// command (t-eca6): every signature check lists it, and at 1,000 Warrants
/// that was a directory read per Warrant per act (t-f815).
fn response_files(dir: &Utf8Path) -> Vec<Utf8PathBuf> {
    fn list(dir: &Utf8Path) -> Vec<Utf8PathBuf> {
        crate::vfs::read_dir_utf8(dir).unwrap_or_default()
    }
    static ONCE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<Utf8PathBuf, Vec<Utf8PathBuf>>>,
    > = std::sync::OnceLock::new();
    if !crate::gate_cmd::source::tree_reads_remembered() {
        return list(dir);
    }
    let memo = ONCE.get_or_init(Default::default);
    if let Some(hit) = memo.lock().ok().and_then(|m| m.get(dir).cloned()) {
        return hit;
    }
    let fresh = list(dir);
    if let Ok(mut m) = memo.lock() {
        m.insert(dir.to_owned(), fresh.clone());
    }
    fresh
}

/// The batch that signs `response`, if one does: its signer is `actor`, it
/// lists the response's file name and exact sha256, and its signature
/// verifies as `principal`. `Some(Invalid)` when a listing batch's signature
/// fails; `None` when no batch lists these bytes.
fn batch_covering(
    repo: &Repository,
    trust: &Trust,
    actor: &str,
    response: &Utf8Path,
) -> Option<Verdict> {
    let principal = trust.principal.as_str();
    let name = response.file_name()?;
    let bytes = crate::vfs::read(response).ok()?;
    let digest = openwarrant_compiler::sha256_hex(&bytes);
    let mut failed = None;
    for (path, batch) in crate::batch_cmd::load_all(repo).iter() {
        if batch.signer != actor || batch.covers(name, &digest).is_none() {
            continue;
        }
        let sig = Utf8PathBuf::from(format!("{path}.sig"));
        if !crate::vfs::is_file(&sig) {
            continue;
        }
        match ssh_verify(repo, trust, path, &sig) {
            Ok(()) => {
                return Some(Verdict::Signed {
                    principal: principal.to_owned(),
                    response: format!(
                        "{} (in batch {}){}",
                        repo.relative(response),
                        batch.batch_id,
                        trust.via
                    ),
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
    trust: &Trust,
    response: &Utf8Path,
    sig: &Utf8Path,
) -> Result<(), SshFailure> {
    let principal = trust.principal.as_str();
    let allowed = trust.allowed.as_path();
    if !crate::vfs::is_file(allowed) {
        return Err(SshFailure::Unavailable(format!(
            "{} does not exist, so no key is allowed to sign anything",
            repo.relative(allowed)
        )));
    }
    // The verdict is a function of exactly these bytes, so one process asks
    // `ssh-keygen` once per distinct input. `war compile` asked 234,556
    // times for a few hundred distinct signatures: every ownership, sign and
    // dispatch query re-verified the same responses (432 s → seconds). A
    // changed byte in any input is a different key, never a stale verdict;
    // `Unavailable` is not cached, so a missing tool is asked again. The
    // source of the key list is part of the key (OW-WAR-0138): the same
    // bytes read from the register and from a store at a given head are two
    // questions, and a store that moves to another head is another question.
    // In a read-only command (t-eca6) the files named here do not change
    // under it, so the paths alone name the question: building the digest key
    // re-read and hashed three files per ask, a fifth of `war check
    // --generated` at 1,000 Warrants (t-f815).
    let by_path = crate::gate_cmd::source::tree_reads_remembered()
        .then(|| format!("{allowed}\0{principal}\0{response}\0{sig}\0{}", trust.label));
    if let Some(k) = &by_path
        && let Some(known) = BY_PATH
            .get_or_init(Default::default)
            .lock()
            .ok()
            .and_then(|c| c.get(k).cloned())
    {
        return known.map_err(SshFailure::Rejected);
    }
    let verdict = ssh_verify_keyed(repo, trust, response, sig, allowed, principal);
    if let Some(k) = by_path {
        let cached = match &verdict {
            Ok(()) => Some(Ok(())),
            Err(SshFailure::Rejected(why)) => Some(Err(why.clone())),
            Err(SshFailure::Unavailable(_)) => None,
        };
        if let (Some(v), Ok(mut c)) = (cached, BY_PATH.get_or_init(Default::default).lock()) {
            c.insert(k, v);
        }
    }
    verdict
}

/// Verdicts by path, for a read-only command only (see [`ssh_verify`]).
static BY_PATH: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, Result<(), String>>>,
> = std::sync::OnceLock::new();

/// [`ssh_verify`] keyed by the digest of every byte the verdict depends on.
fn ssh_verify_keyed(
    repo: &Repository,
    trust: &Trust,
    response: &Utf8Path,
    sig: &Utf8Path,
    allowed: &Utf8Path,
    principal: &str,
) -> Result<(), SshFailure> {
    let read = |p: &Utf8Path| {
        crate::vfs::read(p).map_err(|e| {
            SshFailure::Unavailable(format!("could not open {}: {e}", repo.relative(p)))
        })
    };
    let key = {
        let mut material = Vec::new();
        for part in [
            read(allowed)?,
            principal.as_bytes().to_vec(),
            RESPONSE_NAMESPACE.as_bytes().to_vec(),
            read(response)?,
            read(sig)?,
            trust.label.as_bytes().to_vec(),
        ] {
            material.extend_from_slice(&(part.len() as u64).to_le_bytes());
            material.extend_from_slice(&part);
        }
        openwarrant_compiler::sha256_hex(&material)
    };
    let cache = VERDICTS.get_or_init(Default::default);
    if let Some(known) = cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
        return known.map_err(SshFailure::Rejected);
    }
    // OW-WAR-0148 M8: a hosted run (`war host`) spawns nothing. It answers
    // from the observation its request supplied for exactly these bytes, or
    // fails closed as a missing `ssh-keygen` does. A recording run (`war
    // host --export`) remembers what `ssh-keygen` answered.
    let verdict = match crate::vfs::hosted_ssh(&key) {
        Some(Some(observed)) => observed.map_err(SshFailure::Rejected),
        Some(None) => Err(SshFailure::Unavailable(
            "a hosted run runs no ssh-keygen, and its request supplied no observation of this \
             signature, so it is unchecked. An unchecked signature is not a pass"
                .to_owned(),
        )),
        None => {
            let verdict = ssh_verify_uncached(repo, principal, response, sig, allowed);
            match &verdict {
                Ok(()) => crate::vfs::record_ssh(&key, &Ok(())),
                Err(SshFailure::Rejected(why)) => crate::vfs::record_ssh(&key, &Err(why.clone())),
                Err(SshFailure::Unavailable(_)) => {}
            }
            verdict
        }
    };
    let cached = match &verdict {
        Ok(()) => Some(Ok(())),
        Err(SshFailure::Rejected(why)) => Some(Err(why.clone())),
        Err(SshFailure::Unavailable(_)) => None,
    };
    if let (Some(v), Ok(mut c)) = (cached, cache.lock()) {
        c.insert(key, v);
    }
    verdict
}

/// Verdicts of [`ssh_verify`] this process has already asked for, keyed by
/// the digest of every byte the verdict depends on.
static VERDICTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, Result<(), String>>>,
> = std::sync::OnceLock::new();

fn ssh_verify_uncached(
    repo: &Repository,
    principal: &str,
    response: &Utf8Path,
    sig: &Utf8Path,
    allowed: &Utf8Path,
) -> Result<(), SshFailure> {
    let input = std::fs::File::open(response).map_err(|e| {
        SshFailure::Unavailable(format!("could not open {}: {e}", repo.relative(response)))
    })?;
    let out = std::process::Command::new("ssh-keygen")
        .args(["-Y", "verify", "-f"])
        .arg(allowed)
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

// ---------------------------------------------------------------------------
// The protected store (OW-WAR-0138 D-002). CONTINGENT on the owner answering
// OW-WAR-0138 U-001 option A.
// ---------------------------------------------------------------------------

/// Where the actor binding comes from for this repository.
pub(crate) enum Binding {
    /// No `[authority]` table: `roles.toml` and `allowed_signers`.
    Register,
    /// A store is configured and read.
    Store(StoreBinding),
    /// A store is configured and cannot be used. Every verdict fails closed.
    Failed { rule: &'static str, why: String },
}

/// A store's current head as the read path uses it.
pub(crate) struct StoreBinding {
    pub store: String,
    pub head: String,
    pub test_mode: bool,
    pub revision: openwarrant_core::authority_transition::Revision,
}

/// The role a store must grant for each act. The names are `roles.toml`'s
/// (`ActorRole`), so one vocabulary serves both.
const fn role_for(act: Act) -> &'static str {
    match act {
        // OW-WAR-0136 Q-001 (a): a gate is invalidated by a holder of
        // `resolver` — it disputes resolutions, and the resolver owns standing.
        Act::Resolve | Act::Invalidate => "resolver",
        // OW-ADR-0029: a class pre-authorizes work, so it is an authorizer's.
        Act::Authorize
        | Act::Accept
        | Act::Correct
        | Act::AcceptRoadmap
        | Act::AcceptStanding
        | Act::RevokeStanding => "authorizer",
    }
}

impl StoreBinding {
    /// "the store /path at sha256:… (unprotected test store)".
    pub(crate) fn describe(&self) -> String {
        format!(
            "the store {} at {}{}",
            self.store,
            self.head,
            if self.test_mode {
                " (UNPROTECTED TEST STORE: same account, no separation)"
            } else {
                ""
            }
        )
    }

    /// The principal the store binds `actor` to, and its key list as an
    /// allowed_signers file, after the kind and every act's role are checked.
    pub(crate) fn signer(
        &self,
        actor: &str,
        acts: &[Act],
    ) -> Result<(String, AllowedFile), Verdict> {
        if self.revision.version() < 2 {
            return Err(Verdict::NotHuman {
                why: format!(
                    "{}: its head is a v1 revision, which binds keys to roles and names no actor \
                     or kind, so it cannot say that {actor} is a human (§27.2). Activate a \
                     v1-to-v2 transition (docs/cli/authority.md)",
                    self.describe()
                ),
            });
        }
        let Some((id, entry)) = self.revision.principal_for_actor(actor) else {
            return Err(Verdict::NotHuman {
                why: format!(
                    "{} binds no principal to {actor}, so nothing says this actor may sign \
                     anything",
                    self.describe()
                ),
            });
        };
        if entry.kind != Some(ActorKind::Human) {
            return Err(Verdict::NotHuman {
                why: format!(
                    "{actor} is {} in {}; §27.2 reserves this act for a human",
                    entry
                        .kind
                        .map_or_else(|| "unkinded".to_owned(), |k| k.to_string()),
                    self.describe()
                ),
            });
        }
        for act in acts {
            let role = role_for(*act);
            if !entry.roles.contains(role) {
                return Err(Verdict::NotGranted {
                    why: format!(
                        "{actor} ({id}) holds no `{role}` role in {}, which a {} needs. A grant \
                         removed from the store is removed for every act after it",
                        self.describe(),
                        act.word()
                    ),
                });
            }
        }
        let text: String = self
            .revision
            .principals
            .iter()
            .map(|(p, e)| {
                format!(
                    "{p} namespaces=\"{RESPONSE_NAMESPACE},oh.war/dsse\" {}\n",
                    e.public_key
                )
            })
            .collect();
        let file = AllowedFile::write(&text).map_err(|why| Verdict::Unavailable { why })?;
        Ok((id.to_owned(), file))
    }
}

/// The store's key list written as an OpenSSH allowed_signers file for one
/// `ssh-keygen` call, in a private file removed when dropped.
pub(crate) struct AllowedFile(pub Utf8PathBuf);

impl AllowedFile {
    fn write(text: &str) -> Result<Self, String> {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .map_err(|p| format!("the temporary directory {} is not UTF-8", p.display()))?;
        let path = dir.join(format!(
            "war-store-signers-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut f = options
            .open(&path)
            .map_err(|e| format!("could not write the store's key list to {path}: {e}"))?;
        std::io::Write::write_all(&mut f, text.as_bytes())
            .map_err(|e| format!("could not write the store's key list to {path}: {e}"))?;
        Ok(Self(path))
    }
}

impl Drop for AllowedFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Read the configured store now. `Err` is the rule and reason every act and
/// every verdict fails closed with.
fn read_store(store: &str, test: bool) -> Result<StoreBinding, (&'static str, String)> {
    // OW-WAR-0148 M8: a store lives outside the repository, so outside any
    // Workspace Basis. A hosted run reads nothing else, and fails closed.
    if crate::vfs::is_hosted() {
        return Err((
            "authority.verify-unavailable",
            format!(
                "the protected store {store} lies outside the hosted basis, and a hosted run \
                 reads nothing else: it authorizes nobody"
            ),
        ));
    }
    let current = crate::authority_cmd::store::read_current(std::path::Path::new(store), test)
        .map_err(|e| {
            (
                "authority.verify-unavailable",
                format!(
                    "the protected store {store} configured in openwarrant.toml could not be \
                     read ({e}). Nothing falls back to docs/authority/roles.toml (OW-WAR-0138): \
                     an unreadable store authorizes nobody"
                ),
            )
        })?;
    Ok(StoreBinding {
        store: store.to_owned(),
        head: current.head,
        test_mode: current.test_mode,
        revision: current.revision,
    })
}

/// This repository's binding, read now.
pub(crate) fn binding(repo: &Repository) -> Binding {
    let Some(a) = &repo.config.authority else {
        return Binding::Register;
    };
    match read_store(&a.store, a.unprotected_test_store) {
        Ok(s) => Binding::Store(s),
        Err((rule, why)) => Binding::Failed { rule, why },
    }
}

/// Who `war sign` signs as, and against which key list (OW-WAR-0138).
pub(crate) struct Signer {
    pub principal: String,
    /// The allowed_signers file the signature is made and checked against.
    pub allowed: Utf8PathBuf,
    /// Held so the store's temporary key list lives as long as the signing.
    _store_file: Option<AllowedFile>,
    /// The store head the grant was read at; `None` on the register path.
    pub head: Option<String>,
    pub via: String,
}

/// Resolve `actor` for signing `acts`, or the refusal with its rule.
///
/// Without a store: `roles.toml`'s `ssh_principal` and `allowed_signers`, as
/// before. With one: the store's binding, kind and roles, and a refusal
/// `authority.legacy-fallback` when only the working-tree register knows the
/// actor. Nothing is signed before this answers.
pub(crate) fn signer_for(
    repo: &Repository,
    actor: &str,
    acts: &[Act],
) -> Result<Signer, (&'static str, String)> {
    match binding(repo) {
        Binding::Register => {
            let principal = crate::sign::principal_of(repo, actor)
                .map_err(|why| ("sign.ssh-principal", why))?;
            Ok(Signer {
                principal,
                allowed: repo.root.join("docs/authority/allowed_signers"),
                _store_file: None,
                head: None,
                via: String::new(),
            })
        }
        Binding::Failed { rule, why } => Err((rule, format!("{why}; nothing was signed"))),
        Binding::Store(store) => match store.signer(actor, acts) {
            Ok((principal, file)) => Ok(Signer {
                principal,
                allowed: file.0.clone(),
                _store_file: Some(file),
                head: Some(store.head.clone()),
                via: store.describe(),
            }),
            Err(v) => {
                let legacy = repo
                    .load_legacy_authority_register()
                    .ok()
                    .and_then(|register| {
                        register.actor(actor).and_then(|a| a.ssh_principal.clone())
                    });
                match (v, legacy) {
                    (Verdict::NotHuman { why }, Some(principal)) => Err((
                        "authority.legacy-fallback",
                        format!(
                            "docs/authority/roles.toml binds {actor} to principal {principal:?}, \
                             and {why}. With a store configured the working-tree register grants \
                             nothing (OW-WAR-0138); nothing was signed"
                        ),
                    )),
                    (v, _) => Err((v.rule(), format!("{}; nothing was signed", v.why()))),
                }
            }
        },
    }
}

/// The store's head now, for the check after the key was asked: a grant read
/// at one head is not a grant at another (`allows --expected-head`).
pub(crate) fn store_head(repo: &Repository) -> Result<Option<String>, String> {
    match binding(repo) {
        Binding::Register => Ok(None),
        Binding::Store(s) => Ok(Some(s.head)),
        Binding::Failed { why, .. } => Err(why),
    }
}

/// Apply the configured store's policy to a configuration just read
/// (OW-WAR-0138 D-006), when the repository is opened. No `[authority]`
/// table: nothing changes.
pub(crate) fn govern(config: &mut openwarrant_core::config::RepositoryConfig) {
    let Some(a) = config.authority.clone() else {
        return;
    };
    match read_store(&a.store, a.unprotected_test_store) {
        Ok(s) => match (&s.revision.policy, s.revision.version()) {
            (Some(policy), 2) => config.govern_from_store(&a.store, &s.head, s.test_mode, policy),
            _ => config.govern_fail_closed(
                &a.store,
                "authority.store-v1",
                format!(
                    "{}: its head is a v1 revision, which carries no actor kind and no policy. \
                     Until a v1-to-v2 transition is activated the store authorizes nobody, and the \
                     protected policy keys take their most restrictive values, not \
                     openwarrant.toml's",
                    s.describe()
                ),
            ),
        },
        Err((rule, why)) => config.govern_fail_closed(&a.store, rule, why),
    }
}

/// What `war check` says once about where authority and policy come from.
pub(crate) fn protection_report(repo: &Repository) -> Vec<crate::diagnostic::Diagnostic> {
    use crate::diagnostic::Diagnostic;
    use openwarrant_core::config::Governance;
    let config = repo.relative(&repo.root.join("openwarrant.toml"));
    match &repo.config.governance {
        Governance::Unprotected => vec![Diagnostic::warn(
            "authority.unprotected",
            config,
            "no protected authority store is configured (`[authority] store`). \
             docs/authority/roles.toml, docs/authority/allowed_signers and the policy keys that \
             decide whether work may close ([independence], allow_automated_resolution, \
             require_user_presence, verifier_argv) are working-tree files the performer can \
             write; review of them is the control (THREAT_MODEL row 6, docs/AUTHENTICATION.md)",
        )],
        Governance::FailedClosed { rule, why, .. } => {
            vec![Diagnostic::error(*rule, config, why.clone())]
        }
        Governance::Store {
            store,
            head,
            test_mode,
            divergences,
        } => {
            let mut out = vec![Diagnostic::pass(
                "authority.store",
                format!(
                    "the store {store} at {head} governs the actor binding and the protected \
                     policy keys; roles.toml and allowed_signers grant nothing"
                ),
            )];
            if *test_mode {
                out.push(Diagnostic::warn(
                    "authority.test-store",
                    config.clone(),
                    format!(
                        "{store} is an UNPROTECTED TEST STORE (`unprotected_test_store = true`): \
                         the account running war owns it, so it separates nothing from the \
                         performer. Every signed verdict read through it says so"
                    ),
                ));
            }
            for d in divergences {
                out.push(Diagnostic::error(
                    "policy.unprotected-divergence",
                    config.clone(),
                    format!(
                        "{} is {} in openwarrant.toml and {} in the store {store} at {head}. \
                         The store's value governs; edit the store through a signed transition, \
                         or make the file agree",
                        d.key, d.file, d.store
                    ),
                ));
            }
            out
        }
    }
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
            let i = response_path(&repo, Act::Invalidate, "ops.exit-demo@1.0.0");
            assert!(
                i.as_str()
                    .ends_with("ops.exit-demo@1.0.0.invalidation.response.toml"),
                "{i}"
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
