// SPDX-License-Identifier: Apache-2.0
//! The local draft journal (SAS §66; OW-WAR-0031).
//!
//! # What it is
//!
//! One append-only file per Warrant, `docs/warrants/<alias>/journal.jsonl`,
//! one §66.3 envelope per line. It is written by the commands that change a
//! Warrant's records — `war new`, `war authorize --response`, `war verify
//! --response`, `war evidence record`, `war resolve --response` — and by
//! nothing else. There is no `war journal --append`: an event an agent can
//! type is an event an agent can invent.
//!
//! # What it is for
//!
//! OW-WAR-0008's state model had to DERIVE every state, because nothing
//! recorded a transition; every projection said so in a banner. With a journal
//! a Warrant's phase is READ: `draft.created` → draft, `authorization.recorded`
//! → authorized, `resolution.recorded` → resolved, each with the actor and the
//! time it happened. `Provenance::Recorded` starts meaning something.
//!
//! # Append-only, enforced not encouraged
//!
//! `war check` reads the journal as committed at `HEAD` and requires the
//! working copy to extend it: an edited or deleted line is `journal.rewritten`,
//! an error. The file is provenance, not proof (§66.2) — a commit can still
//! rewrite it — but a rewrite has to be committed as a rewrite, in a diff
//! anyone can read, rather than slipped past a tool that never looked.
//!
//! # Backfill
//!
//! Fifty-six Warrants existed before the journal did. `war journal <alias>
//! --backfill` writes the events their records already imply — `draft.created`
//! at the UUIDv7's own timestamp, `authorization.recorded` at the
//! authorization's effective time, and so on — each carrying
//! `"backfilled": true` in its payload so no reader mistakes a reconstruction
//! for a contemporaneous entry. It runs once per Warrant and refuses to run on
//! a journal that already has events.

use camino::Utf8Path;
use openwarrant_compiler::digest::sha256_hex;
use openwarrant_core::journal::{EventClass, Journal, JournalEvent};
use openwarrant_core::{CommonOutcome, Provenance, WarUuid, WarrantState};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const FILE: &str = "journal.jsonl";

/// §66.4 material events this repository records.
pub const DRAFT_CREATED: &str = "draft.created";
pub const AUTHORIZATION_RECORDED: &str = "authorization.recorded";
pub const VERIFICATION_RECORDED: &str = "verification.recorded";
pub const RECEIPT_ATTACHED: &str = "sync.receipt_attached";
pub const RESOLUTION_RECORDED: &str = "resolution.recorded";
/// A record that already existed, now carrying the human signature it was
/// written without. A separate event type rather than a second
/// `*.recorded`: the payload of a re-recorded authorization is identical to
/// the first, so the idempotency key collides and the append is refused —
/// which aborted a signing sweep mid-batch. It is also the truer history:
/// the record was recorded then, and signed now.
pub const AUTHORIZATION_SIGNATURE_RECORDED: &str = "authorization.signature_recorded";
pub const RESOLUTION_SIGNATURE_RECORDED: &str = "resolution.signature_recorded";
pub const CORRECTION_SIGNATURE_RECORDED: &str = "correction.signature_recorded";
/// OW-WAR-0064 — a delivered artifact superseded for a reason. The payload
/// carries the digest of the correction file as written, which is how
/// `war check` tells an authorized record from one edited afterwards.
pub const CORRECTION_RECORDED: &str = "correction.recorded";

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Parse a journal file. A line that is not an envelope is an error — an
/// unreadable journal is not an empty one.
pub fn parse(text: &str) -> Result<Journal, String> {
    let mut journal = Journal::default();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event: JournalEvent =
            serde_json::from_str(line).map_err(|e| format!("line {}: {e}", n + 1))?;
        journal
            .append(event)
            .map_err(|e| format!("line {}: {e}", n + 1))?;
    }
    Ok(journal)
}

/// An incomplete final write (OW-WAR-0121): the last line has no newline and
/// does not parse. Only that shape is a torn tail. A bad line with a newline
/// after it, anywhere, is `journal.malformed` — it was written whole, and
/// something else is wrong with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TornTail {
    /// The byte offset where the unfinished line starts: the file's length
    /// after `truncate -s <offset>`, which keeps every complete line.
    pub offset: usize,
    /// How many bytes the unfinished line holds.
    pub len: usize,
    /// The 1-based line number of the unfinished line.
    pub line: usize,
}

impl TornTail {
    /// The message `war check` and a refused append both give, naming the
    /// exact truncation that removes only the unfinished bytes. `path` is
    /// spelled as the caller wants the operator to type it.
    #[must_use]
    pub fn describe(&self, path: &str) -> String {
        format!(
            "journal.torn-tail: line {} ({} byte(s) from offset {}) is an incomplete final \
             write — no newline, and it does not parse. Every line before it is whole. \
             `truncate -s {} {path}` removes only the unfinished bytes; war does not run it \
             for you (OW-WAR-0121 U-001)",
            self.line, self.len, self.offset, self.offset
        )
    }
}

/// The torn tail of a journal's bytes, if it has one and every line before it
/// parses. Bytes, not text: a write cut inside a multi-byte character leaves
/// a tail that is not UTF-8, and that is still a torn tail.
#[must_use]
pub fn torn_tail(bytes: &[u8]) -> Option<TornTail> {
    if bytes.is_empty() || bytes.ends_with(b"\n") {
        return None;
    }
    let offset = bytes.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    let last = &bytes[offset..];
    if last.iter().all(u8::is_ascii_whitespace)
        || serde_json::from_slice::<JournalEvent>(last).is_ok()
    {
        return None;
    }
    // A bad line earlier is malformed whatever the tail looks like, so the
    // tail is only named when the rest stands.
    let head = std::str::from_utf8(&bytes[..offset]).ok()?;
    parse(head).ok()?;
    Some(TornTail {
        offset,
        len: last.len(),
        line: head.matches('\n').count() + 1,
    })
}

/// The Warrant's journal, empty when the file is absent. A torn tail is an
/// error naming `journal.torn-tail`, never silently dropped: the journal is
/// read as it is, and repairing it is a person's act.
pub fn load(warrant_dir: &Utf8Path) -> Result<Journal, RepoError> {
    let path = warrant_dir.join(FILE);
    if !path.is_file() {
        return Ok(Journal::default());
    }
    let bytes = std::fs::read(&path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    if let Some(torn) = torn_tail(&bytes) {
        return Err(RepoError::Message(format!(
            "{path}: {}",
            torn.describe(path.as_str())
        )));
    }
    let text = String::from_utf8(bytes)
        .map_err(|e| RepoError::Message(format!("{path}: not UTF-8 ({e})")))?;
    parse(&text).map_err(|e| RepoError::Message(format!("{path}: {e}")))
}

/// Build one envelope. `occurred_at` is RFC 3339; the idempotency key is the
/// digest of (warrant, type, payload), so the same fact recorded twice is
/// refused rather than duplicated. "Same fact" is deliberate: a gate re-run
/// that mints a receipt with the same digest as the last one attaches nothing
/// new, and is refused; a re-run whose receipt digest differs (it ran at a
/// different time, so it always does) is a new event. Time is not in the key
/// so that a backfill and a live write of the same record cannot both land.
#[must_use]
pub fn event(
    warrant_uuid: &str,
    event_type: &str,
    actor_ref: &str,
    occurred_at: &str,
    payload: &str,
) -> JournalEvent {
    JournalEvent {
        v: 1,
        id: WarUuid::mint().to_string(),
        warrant_uuid: warrant_uuid.to_owned(),
        event_type: event_type.to_owned(),
        class: EventClass::DraftHistory,
        actor_ref: actor_ref.to_owned(),
        occurred_at: occurred_at.to_owned(),
        payload: payload.to_owned(),
        idempotency_key: sha256_hex(format!("{warrant_uuid}\n{event_type}\n{payload}").as_bytes()),
    }
}

/// What [`record`] did (OW-WAR-0130).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recorded {
    /// A new line was appended.
    Recorded,
    /// The journal already held this key, by this actor: nothing was written.
    Replayed,
}

/// What the journal already holds for one event, before a command writes
/// anything (§67.4). The key is the idempotency key's own preimage — the
/// event type and payload of this Warrant's journal — so "the same fact" is
/// decided exactly as [`event`] decides it, never by a second rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prior {
    /// Not recorded: the act is new.
    Fresh,
    /// Recorded by this same actor: an equivalent retry, which replays.
    Equivalent { occurred_at: String },
    /// Recorded by someone else: a conflicting reuse of the key, refused.
    Conflict { recorded_by: String },
}

/// Whether `event_type` with `payload` is already in the Warrant's journal,
/// and by whom. A command asks this before its FIRST write, so a retry that
/// replays writes nothing at all and a conflicting one is refused with
/// nothing written. A journal that cannot be read is an error, not `Fresh`.
pub fn already_recorded(
    warrant_dir: &Utf8Path,
    event_type: &str,
    payload: &str,
    actor_ref: &str,
) -> Result<Prior, RepoError> {
    let journal = load(warrant_dir)?;
    let Some(prior) = journal
        .events
        .iter()
        .find(|e| e.event_type == event_type && e.payload == payload)
    else {
        return Ok(Prior::Fresh);
    };
    if prior.actor_ref == actor_ref {
        Ok(Prior::Equivalent {
            occurred_at: prior.occurred_at.clone(),
        })
    } else {
        Ok(Prior::Conflict {
            recorded_by: prior.actor_ref.clone(),
        })
    }
}

/// The refusal for a conflicting reuse: the rule, then who holds the key.
#[must_use]
pub fn conflict_message(
    event_type: &str,
    recorded_by: &str,
    actor_ref: &str,
    file: &Utf8Path,
) -> String {
    format!(
        "journal.idempotency-conflict: {file} already records `{event_type}` with this exact \
         payload by {recorded_by}, and this act is {actor_ref}. The same fact recorded by a \
         second actor is a conflicting reuse of its idempotency key (§67.4), not a retry; \
         nothing was written"
    )
}

/// `journal.idempotency-conflict` as a diagnostic, for a command that
/// reports rather than returns its refusals.
#[must_use]
pub fn conflict(file: String, event_type: &str, recorded_by: &str, actor_ref: &str) -> Diagnostic {
    let message = conflict_message(event_type, recorded_by, actor_ref, Utf8Path::new(&file));
    Diagnostic::error(
        "journal.idempotency-conflict",
        file,
        message.trim_start_matches("journal.idempotency-conflict: "),
    )
}

/// Append one event, now, and write ONLY the new line.
///
/// OW-WAR-0130 (§67.4): the same key by the same actor is an equivalent
/// retry and returns [`Recorded::Replayed`] having written nothing; the same
/// key by a different actor is refused `journal.idempotency-conflict`.
/// Errors are returned, never swallowed: a command whose effect was recorded
/// but whose journal line was not is a command that partially happened, and
/// the caller decides what that means. A command should ask
/// [`already_recorded`] before its first write, so that it never reaches
/// here with a conflict after writing its record.
pub fn record(
    warrant_dir: &Utf8Path,
    warrant_uuid: &str,
    event_type: &str,
    actor_ref: &str,
    payload: &str,
) -> Result<Recorded, RepoError> {
    match already_recorded(warrant_dir, event_type, payload, actor_ref)? {
        Prior::Equivalent { .. } => return Ok(Recorded::Replayed),
        Prior::Conflict { recorded_by } => {
            return Err(RepoError::Message(conflict_message(
                event_type,
                &recorded_by,
                actor_ref,
                &warrant_dir.join(FILE),
            )));
        }
        Prior::Fresh => {}
    }
    let occurred_at = crate::gate_cmd::receipt::rfc3339_from_secs(now_secs());
    let ev = event(warrant_uuid, event_type, actor_ref, &occurred_at, payload);
    append(warrant_dir, ev).map(|()| Recorded::Recorded)
}

fn append(warrant_dir: &Utf8Path, ev: JournalEvent) -> Result<(), RepoError> {
    let mut journal = load(warrant_dir)?;
    if journal
        .events
        .iter()
        .any(|e| e.idempotency_key == ev.idempotency_key)
    {
        return Err(RepoError::Message(format!(
            "{}: `{}` with this payload is already journalled (idempotency key {})",
            warrant_dir.join(FILE),
            ev.event_type,
            &ev.idempotency_key[..12]
        )));
    }
    journal
        .append(ev.clone())
        .map_err(|e| RepoError::Message(format!("{}: {e}", warrant_dir.join(FILE))))?;
    let line = serde_json::to_string(&ev)
        .map_err(|e| RepoError::Message(format!("could not render the event: {e}")))?;
    use std::io::Write;
    let path = warrant_dir.join(FILE);
    // A last line written whole but without its newline (by hand, or by an
    // older tool) parsed above; appending straight after it would fuse two
    // events into one bad line, so the newline is supplied first.
    let needs_newline = std::fs::read(&path)
        .map(|b| b.last().is_some_and(|&c| c != b'\n'))
        .unwrap_or(false);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| RepoError::Io {
            context: format!("could not open {path}"),
            source,
        })?;
    let lead = if needs_newline { "\n" } else { "" };
    // One write of the whole line, then fsync (OW-WAR-0121): an append that
    // returned is on disk, and one cut short is a torn tail `war check`
    // names, not a silent loss.
    f.write_all(format!("{lead}{line}\n").as_bytes())
        .and_then(|()| f.sync_all())
        .map_err(|source| RepoError::Io {
            context: format!("could not append to {path}"),
            source,
        })
}

/// The state a journal RECORDS, or `None` when it records nothing.
///
/// Phase is the furthest material event; `outcome` comes from the caller,
/// who has read the resolution record and knows whether it still binds. A
/// `resolution.recorded` event with no binding record is a resolution that
/// happened and no longer stands: phase `resolved` is still true of history,
/// and the outcome is what the record says, so the caller passes it.
#[must_use]
pub fn recorded_state(journal: &Journal, outcome: Option<CommonOutcome>) -> Option<WarrantState> {
    if journal.events.is_empty() {
        return None;
    }
    let has = |t: &str| journal.events.iter().any(|e| e.event_type == t);
    let mut state = WarrantState::draft(Provenance::Recorded);
    if has(RESOLUTION_RECORDED) {
        state = WarrantState::resolved_recorded(outcome.unwrap_or(CommonOutcome::None));
    } else if has(AUTHORIZATION_RECORDED) {
        state.phase = openwarrant_core::Phase::Authorized;
    }
    Some(state)
}

/// What `HEAD` holds for a path, if this is a git checkout and the file is
/// tracked. `None` means "no committed baseline" — a new journal — and is
/// not an error.
fn committed(repo: &Repository, rel: &str) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["show", &format!("HEAD:{rel}")])
        .current_dir(&repo.root)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `war check` rules: envelope validity, identity, and append-only against
/// the committed baseline.
pub fn check(
    repo: &Repository,
    warrant_dir: &Utf8Path,
    alias: &str,
    warrant_uuid: Option<&str>,
    report: &mut Report,
) {
    // OW-WAR-0130: a batch killed between its first record and its last left
    // a marker. Reported under each Warrant whose directory it lists, as an
    // ERROR: this Warrant's records may hold part of a list nobody signed as
    // a part. Here because `war check` runs this for every Warrant it checks.
    for m in crate::batch_cmd::interrupted(repo) {
        if m.1.roots.is_empty() || crate::batch_cmd::touches(repo, &m.1, warrant_dir) {
            let mut d = crate::batch_cmd::interrupted_diagnostic(repo, &m);
            d.message = format!("{alias}: {}", d.message);
            report.push(d);
        }
    }
    let path = warrant_dir.join(FILE);
    if !path.is_file() {
        return;
    }
    let rel = repo.relative(&path);
    // An incomplete final write is named as exactly that, with the truncation
    // that removes only it (OW-WAR-0121). Nothing here truncates.
    if let Ok(bytes) = std::fs::read(&path)
        && let Some(torn) = torn_tail(&bytes)
    {
        report.push(Diagnostic::error(
            "journal.torn-tail",
            rel.clone(),
            format!(
                "{alias}: {} (the path is relative to the repository root)",
                torn.describe(&rel)
                    .trim_start_matches("journal.torn-tail: ")
            ),
        ));
        return;
    }
    let journal = match load(warrant_dir) {
        Ok(j) => j,
        Err(e) => {
            report.push(Diagnostic::error(
                "journal.malformed",
                rel,
                format!("{alias}: {e} — an unreadable journal is not an empty one"),
            ));
            return;
        }
    };
    if let Some(uuid) = warrant_uuid
        && let Some(stray) = journal.events.iter().find(|e| e.warrant_uuid != uuid)
    {
        report.push(Diagnostic::error(
            "journal.wrong-warrant",
            rel.clone(),
            format!(
                "{alias}: event {} names warrant {} and this Warrant is {uuid}",
                stray.id, stray.warrant_uuid
            ),
        ));
    }
    if let Some(base) = committed(repo, &rel) {
        match parse(&base) {
            Ok(prior) => {
                if let Err(e) = journal.verify_append_only(&prior.events) {
                    report.push(Diagnostic::error(
                        "journal.rewritten",
                        rel.clone(),
                        format!(
                            "{alias}: {e}. A journal is extended, never edited; if history \
                             must change, the change is a new event"
                        ),
                    ));
                    return;
                }
            }
            Err(e) => report.push(Diagnostic::warn(
                "journal.baseline-unreadable",
                rel.clone(),
                format!(
                    "{alias}: the committed journal does not parse ({e}); append-only not checked"
                ),
            )),
        }
    }
    let backfilled = journal
        .events
        .iter()
        .filter(|e| e.payload.contains("\"backfilled\":true"))
        .count();
    report.push(Diagnostic::pass(
        "journal.append-only",
        format!(
            "{alias}: {} event(s), {} backfilled, extends the committed journal",
            journal.events.len(),
            backfilled
        ),
    ));
}

/// The UUIDv7's own timestamp, as RFC 3339.
fn uuid_time(uuid: &WarUuid) -> Option<String> {
    let ts = uuid.as_uuid().get_timestamp()?;
    let (secs, _) = ts.to_unix();
    Some(crate::gate_cmd::receipt::rfc3339_from_secs(secs))
}

/// `war journal <alias> --backfill`: the events the records already imply.
pub fn backfill(repo: &Repository, alias: &str) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let Some(validated) = &one.validated else {
        return Err(RepoError::Message(format!(
            "{alias}: the manifest did not validate; nothing to backfill from"
        )));
    };
    let mut report = Report::default();
    let existing = load(&dir)?;
    if !existing.events.is_empty() {
        report.push(Diagnostic::error(
            "journal.not-empty",
            repo.relative(&dir.join(FILE)),
            format!(
                "{alias}: the journal already has {} event(s); backfill reconstructs history \
                 only where there is none",
                existing.events.len()
            ),
        ));
        return Ok(report);
    }
    let uuid = validated.uuid.to_string();
    let today = crate::gate_cmd::receipt::rfc3339_from_secs(now_secs());
    let mut planned: Vec<(String, String, String, String)> = Vec::new(); // (type, actor, occurred_at, payload)

    let created_at = uuid_time(&validated.uuid).unwrap_or_else(|| today.clone());
    planned.push((
        DRAFT_CREATED.to_owned(),
        format!("agent://{}", repo.performer()),
        created_at,
        format!("{{\"backfilled\":true,\"from\":\"uuidv7 timestamp\",\"on\":\"{today}\"}}"),
    ));
    if let Some(a) = repo.load_authorization(&dir)?
        && let Some(auth) = &a.revision.authorization
    {
        planned.push((
            AUTHORIZATION_RECORDED.to_owned(),
            format!("person://{}", auth.authorizer),
            auth.effective_time.clone(),
            format!(
                "{{\"backfilled\":true,\"from\":\"authorization.toml\",\"contract_digest\":\"{}\",\"on\":\"{today}\"}}",
                a.revision.contract_digest
            ),
        ));
    }
    for e in crate::evidence::load(repo, &dir)? {
        if let Some(r) = &e.receipt {
            planned.push((
                RECEIPT_ATTACHED.to_owned(),
                format!("agent://{}", repo.performer()),
                r.completed_at.clone(),
                format!(
                    "{{\"backfilled\":true,\"from\":\"{}\",\"gate\":\"{}\",\"receipt_digest\":\"{}\",\"on\":\"{today}\"}}",
                    repo.relative(&e.receipt_path),
                    e.run.gate,
                    r.receipt_digest
                ),
            ));
        }
    }
    if let Some(r) = repo.load_resolution(&dir)? {
        planned.push((
            RESOLUTION_RECORDED.to_owned(),
            r.resolution.resolved_by_ref.clone(),
            r.resolution.effective_at.clone(),
            format!(
                "{{\"backfilled\":true,\"from\":\"resolution.toml\",\"common_outcome\":\"{}\",\"contract_digest\":\"{}\",\"on\":\"{today}\"}}",
                r.resolution.common_outcome, r.resolution.contract_digest
            ),
        ));
    }
    for (t, actor, at, payload) in &planned {
        append(&dir, event(&uuid, t, actor, at, payload))?;
    }
    report.push(Diagnostic::pass(
        "journal.backfilled",
        format!(
            "{alias}: {} event(s) reconstructed from records, each marked backfilled → {}",
            planned.len(),
            repo.relative(&dir.join(FILE))
        ),
    ));
    Ok(report)
}

/// `war journal <alias>`: print the events.
pub fn show(repo: &Repository, alias: &str) -> Result<String, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let journal = load(&dir)?;
    let mut out = String::new();
    if journal.events.is_empty() {
        out.push_str(&format!(
            "{alias}: no journal. State is derived (§66; OW-WAR-0031).\n"
        ));
        return Ok(out);
    }
    for e in &journal.events {
        out.push_str(&format!(
            "{}  {:<26} {:<28} {}\n",
            e.occurred_at, e.event_type, e.actor_ref, e.payload
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_journal_round_trips_through_jsonl_and_refuses_a_duplicate_key() {
        let a = event(
            "u",
            DRAFT_CREATED,
            "agent://x",
            "2026-09-02T00:00:00Z",
            "{}",
        );
        let b = event(
            "u",
            AUTHORIZATION_RECORDED,
            "person://y",
            "2026-09-02T01:00:00Z",
            "{}",
        );
        let text = format!(
            "{}\n{}\n",
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        let j = parse(&text).expect("parses");
        assert_eq!(j.events.len(), 2);
        assert_eq!(
            a.idempotency_key,
            event("u", DRAFT_CREATED, "agent://z", "later", "{}").idempotency_key
        );
    }

    #[test]
    fn recorded_state_reads_the_furthest_event() {
        let mut j = Journal::default();
        assert!(recorded_state(&j, None).is_none());
        j.append(event("u", DRAFT_CREATED, "a", "t", "{}")).unwrap();
        let s = recorded_state(&j, None).unwrap();
        assert_eq!(s.provenance, Provenance::Recorded);
        assert_eq!(s.phase, openwarrant_core::Phase::Draft);
        j.append(event("u", AUTHORIZATION_RECORDED, "a", "t", "{}"))
            .unwrap();
        assert_eq!(
            recorded_state(&j, None).unwrap().phase,
            openwarrant_core::Phase::Authorized
        );
        j.append(event("u", RESOLUTION_RECORDED, "a", "t", "{}"))
            .unwrap();
        let s = recorded_state(&j, Some(CommonOutcome::Satisfied)).unwrap();
        assert_eq!(s.phase, openwarrant_core::Phase::Resolved);
        assert_eq!(s.outcome, CommonOutcome::Satisfied);
    }

    #[test]
    fn an_edited_line_is_not_append_only() {
        let a = event("u", DRAFT_CREATED, "a", "t", "{}");
        let mut edited = a.clone();
        edited.actor_ref = "someone-else".into();
        let prior = Journal {
            events: vec![a],
            registered: false,
        };
        let now = Journal {
            events: vec![edited],
            registered: false,
        };
        assert!(now.verify_append_only(&prior.events).is_err());
        let prior2 = Journal::default();
        assert!(
            now.verify_append_only(&prior2.events).is_ok(),
            "a new journal extends nothing"
        );
    }

    #[test]
    fn a_malformed_line_is_an_error_not_an_empty_journal() {
        assert!(parse("{\"v\":1}\n").is_err());
        assert!(parse("not json\n").is_err());
        assert!(
            parse("\n\n")
                .expect("blank lines are nothing")
                .events
                .is_empty()
        );
    }

    #[test]
    fn only_an_unterminated_unparseable_last_line_is_a_torn_tail() {
        let a = serde_json::to_string(&event("u", DRAFT_CREATED, "a", "t", "{}")).unwrap();
        let b = serde_json::to_string(&event("u", AUTHORIZATION_RECORDED, "a", "t", "{}")).unwrap();
        let half = &b[..b.len() / 2];

        let torn = torn_tail(format!("{a}\n{half}").as_bytes()).expect("torn tail");
        assert_eq!(torn.offset, a.len() + 1);
        assert_eq!(torn.len, half.len());
        assert_eq!(torn.line, 2);
        assert!(
            torn.describe("j")
                .contains(&format!("truncate -s {} j", a.len() + 1))
        );

        // Whole, terminated, unterminated-but-parseable: none is torn.
        assert!(torn_tail(format!("{a}\n{b}\n").as_bytes()).is_none());
        assert!(torn_tail(format!("{a}\n{b}").as_bytes()).is_none());
        // The same bytes with a newline after them, or between good lines, are
        // a malformed journal, not a torn one.
        assert!(torn_tail(format!("{a}\n{half}\n").as_bytes()).is_none());
        assert!(torn_tail(format!("{a}\n{half}\n{b}").as_bytes()).is_none());
        assert!(parse(&format!("{a}\n{half}\n{b}\n")).is_err());
        // A bad middle line and a torn tail: the middle line wins.
        assert!(torn_tail(format!("{half}\n{a}\n{half}").as_bytes()).is_none());
    }
}
