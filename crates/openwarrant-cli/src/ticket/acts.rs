// SPDX-License-Identifier: Apache-2.0
//! The two upper rungs of the tick ladder on a ticket's item (OW-WAR-0148
//! M13): `independent`, from a verification by someone other than the
//! performer, and `signed`, from a human's signature.
//!
//! # Independent: `war verify <t-x/i-y>`
//!
//! The same seam as a Warrant's verification (`crate::verify`), at the size
//! of one item. Without `--response` it writes the request: the item, its
//! performer, the checks that apply and the runs on record, for something
//! that is not the performer to answer. With `--response <file>` it ingests
//! the answer, an `oh.war/tick-verification-response/v1` whose
//! `[verification]` is the core's [`Verification`], admitted by the same
//! [`Verification::admissible_for`] a Warrant's verdicts are: no verifier,
//! no evidence, or a verifier who is the performer is refused by rule and
//! nothing is written. The performer is read from the record (who ticked the
//! item, or who holds its claim), never from the response, so a response
//! cannot name someone else to pass the self-verification refusal. An
//! `established` verdict ticks the item at `independent` (or raises its
//! tick); any other is journalled and ticks nothing.
//!
//! # Signed: `war sign <t-x/i-y> --ssh-sign`
//!
//! A response naming the ticket, the item and a digest of its text is
//! written under `docs/authority/responses/` and signed with the signer's
//! ssh key through [`crate::sign::ssh_sign_act`], under the act
//! [`crate::authority_check::Act::SignOff`]; it counts only once
//! [`crate::authority_check::verify`] says a human's signature over those
//! bytes verifies. The tool never holds a key: the ssh agent asks the human.
//! Without `--ssh-sign` nothing is signed (a terminal confirmation makes no
//! signature a later reader could check).

use camino::Utf8PathBuf;
use openwarrant_core::ticks::{self, Level};
use openwarrant_core::verification::{Verification, VerificationError};
use serde::{Deserialize, Serialize};

use super::ladder::{self, Reader, event};
use super::{Outcome, Store, Target, Ticket, resolve};
use crate::diagnostic::Diagnostic;
use crate::repo::{RepoError, Repository};

/// The request `war verify <item>` writes.
pub const REQUEST_SCHEMA: &str = "oh.war/tick-verification-request/v1";
/// The response it ingests.
pub const RESPONSE_SCHEMA: &str = "oh.war/tick-verification-response/v1";
/// The response a sign-off signs.
pub const SIGNOFF_SCHEMA: &str = "oh.war/tick-signoff-response/v1";

/// A verifier's answer about one item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TickVerificationResponse {
    pub schema: String,
    pub ticket: String,
    pub item: String,
    pub verification: Verification,
}

fn item_of<'a>(
    tickets: &'a [Ticket],
    query: &str,
    act: &str,
) -> Result<(&'a Ticket, String), Box<Outcome>> {
    match resolve(tickets, query) {
        Ok(Target::Item(n, i)) => Ok((&tickets[n], i)),
        Ok(Target::Ticket(n)) => Err(Box::new(Outcome::refused(
            "tick.item-required",
            String::new(),
            format!(
                "{} names a ticket; {act} is one item's (`{}/<item>`)",
                tickets[n].id(),
                tickets[n].id()
            ),
        ))),
        Err(d) => Err(Box::new(Outcome::from_diagnostic(d))),
    }
}

/// Who performed `item`: who ticked it, or (open) who holds its claim or
/// the whole ticket's. `Err` says why nobody can be named.
fn performer_of(store: &Store, t: &Ticket, item: &str, what: &str) -> Result<String, String> {
    let it = t
        .item(item)
        .ok_or_else(|| format!("{what} is not in the checklist"))?;
    if it.done {
        return it.done_by.clone().ok_or_else(|| {
            format!(
                "{what} was ticked by hand and names nobody, so no verification can be told \
                 apart from its performer's own"
            )
        });
    }
    let claims = store.claims().map_err(|e| e.to_string())?;
    super::claim_on(&claims, t.id(), Some(item))
        .or_else(|| super::claim_on(&claims, t.id(), None))
        .and_then(|(_, c)| c.map(|c| c.actor.clone()))
        .ok_or_else(|| {
            format!(
                "{what} is open and nobody holds it; its performer claims it first (`war claim \
                 {what}`), then `war verify {what}` names them"
            )
        })
}

/// `war verify <t-x/i-y>`: the request an independent verifier answers.
pub fn verify_request(store: &Store, query: &str) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let (t, item) = match item_of(&tickets, query, "a verification") {
        Ok(v) => v,
        Err(refusal) => return Ok(*refusal),
    };
    let what = format!("{}/{item}", t.id());
    let performer = match performer_of(store, t, &item, &what) {
        Ok(p) => p,
        Err(why) => {
            return Ok(Outcome::refused(
                "tick.performer-unknown",
                store.rel(&t.checklist_path),
                why,
            ));
        }
    };
    let it = t.item(&item).expect("resolved");
    let reader = Reader::new(store);
    let checks = ladder::checks_of(t);
    let backing = ladder::backing(t);
    let level = it
        .done
        .then(|| ladder::view_of(&reader, t, it, &checks, &backing).level);
    let (tests, kpis) = checks.for_item(Some(&item));
    let request = serde_json::json!({
        "schema": REQUEST_SCHEMA,
        "ticket": t.id(),
        "ticket_uuid": t.manifest.uuid,
        "title": t.manifest.title,
        "item": item,
        "text": it.text,
        "performer": performer,
        "done": it.done,
        "level": level,
        "tests": tests,
        "kpis": kpis,
        "kpi_runs": backing.kpi_runs.iter().filter(|r| r.item.is_none() || r.item.as_deref() == Some(item.as_str())).collect::<Vec<_>>(),
        "answer": {
            "schema": RESPONSE_SCHEMA,
            "how": format!(
                "Examine the work yourself; you are not {performer}. Answer with TOML: schema, \
                 ticket, item, and a [verification] table (obligation = \"{what}\", performer = \
                 \"{performer}\", disposition = established|refuted|not_established, evidence = \
                 what you examined, and [verification.verifier] with actor, kind and the nine \
                 independence booleans). `war verify {what} --response <file>` ingests it."
            ),
        },
    });
    let human = serde_json::to_string_pretty(&request).unwrap_or_default();
    Ok(Outcome::ok(human, request))
}

/// `war verify <t-x/i-y> --response <file>`: ingest an independent verdict
/// about one item.
pub fn verify_ingest(
    store: &Store,
    query: &str,
    response: &camino::Utf8Path,
) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let (t, item) = match item_of(&tickets, query, "a verification") {
        Ok(v) => v,
        Err(refusal) => return Ok(*refusal),
    };
    let what = format!("{}/{item}", t.id());
    let file = response.to_string();
    let refuse = |rule: &str, why: String| Ok(Outcome::refused(rule, file.clone(), why));
    let bytes = match std::fs::read(response) {
        Ok(b) => b,
        Err(e) => {
            return refuse(
                "tick.response-unreadable",
                format!("could not read it: {e}"),
            );
        }
    };
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let parsed: TickVerificationResponse = match toml::from_str(&text)
        .map_err(|e| e.to_string())
        .or_else(|toml_err| {
            serde_json::from_str(&text).map_err(|json_err| format!("{toml_err} / {json_err}"))
        }) {
        Ok(r) => r,
        Err(e) => {
            return refuse(
                "tick.response-malformed",
                format!("not an {RESPONSE_SCHEMA} (TOML or JSON): {e}; nothing was written"),
            );
        }
    };
    if parsed.schema != RESPONSE_SCHEMA {
        return refuse(
            "tick.response-schema",
            format!(
                "schema {:?}; this build ingests {RESPONSE_SCHEMA}; nothing was written",
                parsed.schema
            ),
        );
    }
    if parsed.ticket != t.id() || parsed.item != item || parsed.verification.obligation != what {
        return refuse(
            "tick.response-target",
            format!(
                "the response is about {}/{} (obligation {:?}), not {what}; nothing was written",
                parsed.ticket, parsed.item, parsed.verification.obligation
            ),
        );
    }
    let performer = match performer_of(store, t, &item, &what) {
        Ok(p) => p,
        Err(why) => return refuse("tick.performer-unknown", why),
    };
    let v = &parsed.verification;
    if v.performer != performer {
        return refuse(
            "tick.performer-mismatch",
            format!(
                "the response names {:?} as the performer, and the record says {performer} did \
                 {what}; a verdict about someone else's work is not this item's. Nothing was \
                 written",
                v.performer
            ),
        );
    }
    if let Err(e) = v.admissible_for("basic") {
        let (rule, why) = match &e {
            VerificationError::NoVerifier { .. } => (
                "tick.no-verifier",
                "the response names no verifier".to_owned(),
            ),
            VerificationError::SelfVerification { actor, .. } => (
                "tick.self-verification",
                format!(
                    "the verifier, {actor}, is the performer: a performer's own check is not \
                     independent (their tests passing is `war done {what} --check`, observed)"
                ),
            ),
            VerificationError::NoEvidence { .. } => (
                "tick.no-evidence",
                "the verdict cites no evidence: say what was examined".to_owned(),
            ),
            VerificationError::InsufficientIndependence { .. } => {
                ("tick.independence", format!("not independent enough: {e}"))
            }
        };
        return refuse(rule, format!("{what}: {why}; nothing was written"));
    }
    let established = v.disposition == openwarrant_core::obligation::Disposition::Established;
    let digest = format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes));
    let it = t.item(&item).expect("resolved");
    let reader = Reader::new(store);
    let checks = ladder::checks_of(t);
    let before = it
        .done
        .then(|| ladder::view_of(&reader, t, it, &checks, &ladder::backing(t)).level);
    // The verdict is journalled first, so the tick it raises is backed the
    // moment the line says so.
    store.journal(
        t,
        event::TICK_VERIFIED,
        &serde_json::json!({
            "item": item,
            "target": what,
            "verifier": v.verifier.actor,
            "verifier_kind": v.verifier.kind,
            "performer": performer,
            "disposition": v.disposition.as_str(),
            "evidence": v.evidence,
            "response": store.rel(&Utf8PathBuf::from(response)),
            "response_sha256": digest,
        }),
    )?;
    if !established {
        return Ok(Outcome::ok(
            format!(
                "{what}: {} by {} recorded; nothing ticked or raised (only `established` raises \
                 a tick to independent)",
                v.disposition.as_str(),
                v.verifier.actor
            ),
            serde_json::json!({"schema": "oh.war/tick-verified/v1", "target": what,
                "disposition": v.disposition.as_str(), "verifier": v.verifier.actor,
                "level": before}),
        ));
    }
    if before.is_some_and(|l| l >= Level::Independent) {
        return Ok(Outcome::ok(
            format!(
                "{what} is already ticked at {}; the verdict by {} is recorded",
                before.map_or("", Level::as_str),
                v.verifier.actor
            ),
            serde_json::json!({"schema": "oh.war/tick-verified/v1", "target": what,
                "disposition": "established", "verifier": v.verifier.actor, "level": before}),
        ));
    }
    let raised = set_level(store, t, &item, &what, Level::Independent, &performer)?;
    if let Err(refusal) = raised {
        return Ok(*refusal);
    }
    if before.is_some() {
        store.journal(
            t,
            event::TICK_RAISED,
            &serde_json::json!({"item": item, "target": what, "from": before,
                "to": "independent", "verifier": v.verifier.actor}),
        )?;
    } else {
        let _ = super::claim::release(&store.lock_of(t.id(), Some(&item)), &performer);
    }
    Ok(Outcome::ok(
        format!(
            "{what} {} at independent: verified by {}, who is not {performer}",
            if before.is_some() { "raised" } else { "ticked" },
            v.verifier.actor
        ),
        serde_json::json!({"schema": "oh.war/tick-verified/v1", "target": what,
            "disposition": "established", "verifier": v.verifier.actor,
            "from": before, "level": "independent"}),
    ))
}

/// Write `item`'s tick at `level`: tick an open item (done by `performer`,
/// today), or raise a done one's marker, keeping who ticked it, when, and
/// its note. Journals `ticket.item_done` for an open item.
fn set_level(
    store: &Store,
    t: &Ticket,
    item: &str,
    what: &str,
    level: Level,
    performer: &str,
) -> Result<Result<(), Box<Outcome>>, RepoError> {
    let today = super::date_of(super::now_secs());
    let mut ticked_now = false;
    let written = super::rewrite(&t.checklist_path, |text| {
        let c = openwarrant_core::ticket::parse(text);
        let Some(it) = c.item(item) else {
            return Err(Box::new(Outcome::refused(
                "ticket.unknown",
                String::new(),
                format!("{what} is no longer in the checklist; nothing was written"),
            )));
        };
        let (date, by) = if it.done {
            (
                it.done_on
                    .as_deref()
                    .map_or_else(|| today.clone(), |d| ticks::split_level(d).0.to_owned()),
                it.done_by.clone().unwrap_or_else(|| performer.to_owned()),
            )
        } else {
            ticked_now = true;
            (today.clone(), performer.to_owned())
        };
        let line = it
            .ticked(&by, &ticks::with_level(&date, level), it.note.as_deref())
            .render();
        Ok((
            openwarrant_core::ticket::replace_line(text, it.line, &line),
            (),
        ))
    })?;
    if let Err(refusal) = written {
        return Ok(Err(refusal));
    }
    if ticked_now {
        store.journal(
            t,
            super::event::ITEM_DONE,
            &serde_json::json!({"item": item, "target": what, "on": today,
                "level": level.as_str(), "by": performer}),
        )?;
    }
    Ok(Ok(()))
}

// ---- signed -------------------------------------------------------------------------

/// What `war sign <t-x/i-y>` is given.
#[derive(Debug, Clone, Default)]
pub struct SignArgs {
    pub actor: Option<String>,
    pub meaning: Option<String>,
    pub ssh_sign: bool,
    pub dry_run: bool,
}

/// `war sign <t-x/i-y> --ssh-sign [--as <human>]`: a human's sign-off of
/// one item, which ticks it (or raises its tick) at `signed` once the
/// signature verifies.
pub fn sign(
    repo: &Repository,
    store: &Store,
    query: &str,
    args: &SignArgs,
) -> Result<Outcome, RepoError> {
    use crate::authority_check::{self, Act};
    let (tickets, _) = store.load_all()?;
    let (t, item) = match item_of(&tickets, query, "a sign-off") {
        Ok(v) => v,
        Err(refusal) => return Ok(*refusal),
    };
    let what = format!("{}/{item}", t.id());
    let it = t.item(&item).expect("resolved");
    let register = repo.load_authority_register()?;
    let eligible: Vec<String> = register
        .assignments
        .iter()
        .filter(|a| {
            a.actor_kind == openwarrant_core::authority::ActorKind::Human
                && a.ssh_principal.is_some()
        })
        .map(|a| a.actor.clone())
        .collect();
    let opts = crate::sign::Options {
        actor: args.actor.clone(),
        meaning: None,
        outcome: None,
        adr_ref: None,
        independence: openwarrant_core::Independence::SeparateRole,
        edit: false,
        all: false,
        show: false,
        dry_run: args.dry_run,
        ssh_sign: args.ssh_sign,
        verify: false,
        kind: None,
        revoke: false,
    };
    let signer = match crate::sign::choose_actor(&eligible, &opts) {
        Ok(s) => s,
        Err(why) => {
            let why = if eligible.is_empty() {
                "no human in docs/authority/roles.toml has an `ssh_principal`, so nobody here \
                 can sign it off; a person adds one (and the key's line in \
                 docs/authority/allowed_signers)"
                    .to_owned()
            } else {
                why
            };
            return Ok(Outcome::refused(
                "sign.no-eligible-signer",
                "docs/authority/roles.toml",
                format!("{what}: {why}. This blocks only the sign-off, not your work"),
            ));
        }
    };
    let digest = ladder::statement_digest(t, &item, &it.text);
    let subject = ladder::signoff_subject(t, &item);
    let reader = Reader::new(store);
    let checks = ladder::checks_of(t);
    let backing = ladder::backing(t);
    let before = it
        .done
        .then(|| ladder::view_of(&reader, t, it, &checks, &backing).level);
    if before == Some(Level::Signed) {
        return Ok(Outcome::ok(
            format!("{what} is already signed off, and the signature verifies"),
            serde_json::json!({"schema": "oh.war/tick-signed/v1", "target": what,
                "level": "signed", "already": true}),
        ));
    }
    let performer = if it.done {
        it.done_by.clone().unwrap_or_else(|| signer.clone())
    } else {
        let claims = store.claims()?;
        super::claim_on(&claims, t.id(), Some(&item))
            .or_else(|| super::claim_on(&claims, t.id(), None))
            .and_then(|(_, c)| c.map(|c| c.actor.clone()))
            .unwrap_or_else(|| signer.clone())
    };
    let now = crate::gate_cmd::receipt::rfc3339_from_secs(super::now_secs());
    let mut meaning = format!(
        "I, {signer}, sign off {what} (\"{}\") of ticket {} (\"{}\"), {}.",
        openwarrant_core::ticket::one_line(&it.text),
        t.id(),
        t.manifest.title,
        match before {
            Some(l) => format!("ticked by {performer} at {}", l.as_str()),
            None => "not yet ticked; this sign-off ticks it".to_owned(),
        }
    );
    if let Some(m) = args
        .meaning
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
    {
        meaning.push(' ');
        meaning.push_str(m);
    }
    let body = format!(
        "# A human's sign-off of one ticket item (OW-WAR-0148 M13). Signed with\n\
         # `war sign {what} --ssh-sign`; the .sig beside it is the signature.\n\
         schema = {schema}\nticket = {ticket}\nticket_uuid = {uuid}\nitem = {item_q}\n\
         text = {text}\nsha256 = {digest_q}\nlevel_before = {level}\nperformer = {performer_q}\n\
         signed_by = {signer_q}\nsigned_at = {now_q}\nmeaning = {meaning_q}\n",
        schema = openwarrant_core::ticket::toml_string(SIGNOFF_SCHEMA),
        ticket = openwarrant_core::ticket::toml_string(t.id()),
        uuid = openwarrant_core::ticket::toml_string(&t.manifest.uuid),
        item_q = openwarrant_core::ticket::toml_string(&item),
        text = openwarrant_core::ticket::toml_string(&it.text),
        digest_q = openwarrant_core::ticket::toml_string(&digest),
        level = openwarrant_core::ticket::toml_string(before.map_or("open", Level::as_str)),
        performer_q = openwarrant_core::ticket::toml_string(&performer),
        signer_q = openwarrant_core::ticket::toml_string(&signer),
        now_q = openwarrant_core::ticket::toml_string(&now),
        meaning_q = openwarrant_core::ticket::toml_string(&meaning),
    );
    let path = authority_check::response_path(repo, Act::SignOff, &subject);
    if args.dry_run {
        let mut out = Outcome::ok(
            format!(
                "would sign off {what} as {signer}, writing {} and its .sig; nothing was written \
                 and no key was asked\n\n{body}",
                repo.relative(&path)
            ),
            serde_json::json!({"schema": "oh.war/tick-signed/v1", "target": what,
                "signer": signer, "response": repo.relative(&path), "digest": digest,
                "dry_run": true}),
        );
        out.report.push(Diagnostic::pass(
            "sign.would-record",
            format!("{what}: a sign-off by {signer} would be recorded"),
        ));
        return Ok(out);
    }
    if !args.ssh_sign {
        return Ok(Outcome::refused(
            "sign.ssh-required",
            String::new(),
            format!(
                "a sign-off of {what} is a signature a later reader can check, so it is made \
                 with your ssh key: `war sign {what} --ssh-sign --as {signer}`. This blocks only \
                 the sign-off, not your work"
            ),
        ));
    }
    if let Err(why) = crate::sign::retire_prior(&path, &digest) {
        return Ok(Outcome::refused(
            "sign.response-exists",
            repo.relative(&path),
            why,
        ));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|source| RepoError::Io {
            context: format!("could not create {dir}"),
            source,
        })?;
    }
    crate::compile::atomic::write(&path, &body)?;
    let signed = match crate::sign::ssh_sign_act(repo, &signer, &[Act::SignOff], &path) {
        Ok(s) => s,
        Err(refusal) => {
            let _ = std::fs::remove_file(&path);
            return Ok(Outcome::refused(
                refusal.rule,
                repo.relative(&path),
                format!(
                    "{}; nothing was recorded. This blocks only the sign-off, not your work",
                    refusal.why
                ),
            ));
        }
    };
    let verdict = authority_check::verify(repo, Act::SignOff, &subject, &signer, Some(&digest));
    if !verdict.is_signed() {
        let _ = std::fs::remove_file(&signed.sig);
        let _ = std::fs::remove_file(&path);
        return Ok(Outcome::refused(
            verdict.rule(),
            repo.relative(&path),
            format!(
                "the signature was made and does not verify as {signer}'s: {}; nothing was \
                 recorded",
                verdict.why()
            ),
        ));
    }
    store.journal(
        t,
        event::TICK_SIGNED,
        &serde_json::json!({
            "item": item,
            "target": what,
            "signer": signer,
            "performer": performer,
            "response": repo.relative(&path),
            "digest": digest,
            "from": before,
        }),
    )?;
    if let Err(refusal) = set_level(store, t, &item, &what, Level::Signed, &performer)? {
        return Ok(*refusal);
    }
    if before.is_some() {
        store.journal(
            t,
            event::TICK_RAISED,
            &serde_json::json!({"item": item, "target": what, "from": before,
                "to": "signed", "signer": signer}),
        )?;
    }
    Ok(Outcome::ok(
        format!(
            "{what} {} at signed: signed off by {signer}; {} verifies",
            if before.is_some() { "raised" } else { "ticked" },
            repo.relative(&signed.sig)
        ),
        serde_json::json!({"schema": "oh.war/tick-signed/v1", "target": what,
            "signer": signer, "response": repo.relative(&path), "digest": digest,
            "from": before, "level": "signed"}),
    ))
}
