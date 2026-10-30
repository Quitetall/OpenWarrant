// SPDX-License-Identifier: Apache-2.0
//! The local draft journal (SAS §66; OW-WAR-0031).
//!
//! # What it is
//!
//! One append-only file per Warrant, `docs/warrants/<alias>/journal.jsonl`,
//! one §66.3 envelope per line. It is written by the commands that change a
//! Warrant's records — `war plan new`, `war sign authorize --response`, `war evidence verify
//! --response`, `war evidence record`, `war sign resolve --response` — and by
//! nothing else. There is no `war admin journal --append`: an event an agent can
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
//! working copy to extend it: an edited or deleted line is
//! `work.journal-rewritten`, an error. The file is provenance, not proof (§66.2) — a commit can still
//! rewrite it — but a rewrite has to be committed as a rewrite, in a diff
//! anyone can read, rather than slipped past a tool that never looked.
//!
//! # Backfill
//!
//! Fifty-six Warrants existed before the journal did. `war admin journal <alias>
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

/// Now, in Unix seconds: the process's `--at` when a command set one
/// ([`crate::ticket::answer_at`]), so an event a replayed write appends
/// occurs at the time it replays.
fn now_secs() -> u64 {
    crate::ticket::now_secs()
}

// ---- the envelope, v1 and v2 (FORMAT-1.0 §11.2) ------------------------------

/// Which envelope a journal is appended in.
///
/// A reader reads both, in one journal, forever, and never rewrites a `v: 1`
/// line. A writer appends `v: 2` to the journal of a Format 1.0 work
/// document and `v: 1` to a legacy directory Warrant's or ticket's, so that
/// journal keeps one shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `{v: 1, warrant_uuid, payload: "<JSON text>"}`: every journal written
    /// before Format 1.0, and every legacy directory document's still.
    V1,
    /// `{v: 2, document_uuid, payload: {...}}`, the line RFC 8785 JSON.
    V2,
}

/// The envelope a journal in `dir` is appended in: [`Shape::V2`] when `dir`
/// is the evidence directory of a Format 1.0 work document — it holds
/// `warrant.md` or `ticket.md`, or `<dir>.md` sits beside it (§4.1, §4.3) —
/// and [`Shape::V1`] otherwise.
#[must_use]
pub fn shape_of(dir: &Utf8Path) -> Shape {
    if crate::format_doc::is_document_dir(dir) {
        return Shape::V2;
    }
    let single = match (dir.parent(), dir.file_name()) {
        (Some(parent), Some(name)) if !name.is_empty() => parent.join(format!("{name}.md")),
        _ => return Shape::V1,
    };
    if crate::vfs::is_file(&single) {
        Shape::V2
    } else {
        Shape::V1
    }
}

/// The `work.*` name of an event type (§11.2): journals written before
/// Format 1.0 name the tracker's events `ticket.*`, and a reader reads each
/// as the `work.*` event of the same suffix. Any other type is itself.
#[must_use]
pub fn work_type(event_type: &str) -> std::borrow::Cow<'_, str> {
    match event_type.strip_prefix("ticket.") {
        Some(suffix) => std::borrow::Cow::Owned(format!("work.{suffix}")),
        None => std::borrow::Cow::Borrowed(event_type),
    }
}

/// The name a `v: 1` journal writes a tracker event under: `ticket.*`, the
/// name every event in it already has. Any other type is itself.
#[must_use]
pub fn legacy_type(event_type: &str) -> std::borrow::Cow<'_, str> {
    match event_type.strip_prefix("work.") {
        Some(suffix) => std::borrow::Cow::Owned(format!("ticket.{suffix}")),
        None => std::borrow::Cow::Borrowed(event_type),
    }
}

/// Whether journal `text` may hold an event of `event_type`, under its
/// `work.*` name or the `ticket.*` name a `v: 1` journal gives it: the cheap
/// scan a reader makes before it parses.
#[must_use]
pub fn may_mention(text: &str, event_type: &str) -> bool {
    text.contains(event_type)
        || text.contains(legacy_type(event_type).as_ref())
        || text.contains(work_type(event_type).as_ref())
}

/// RFC 8785 bytes of a payload given as JSON text; the text itself when it
/// is not JSON (a `v: 1` payload is whatever text was written).
#[must_use]
pub fn canonical_payload(payload: &str) -> String {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .and_then(|v| serde_jcs::to_string(&v).ok())
        .unwrap_or_else(|| payload.to_owned())
}

/// One `v: 2` line as written (§11.2).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct EnvelopeV2 {
    v: u32,
    id: String,
    document_uuid: String,
    #[serde(rename = "type")]
    event_type: String,
    class: EventClass,
    actor_ref: String,
    occurred_at: String,
    payload: serde_json::Value,
    idempotency_key: String,
}

/// One journal line, `v: 1` or `v: 2`, as the event every reader reads: a
/// `v: 2` line's `document_uuid` is the event's document (`warrant_uuid`),
/// its payload object is carried as its RFC 8785 text, and a `ticket.*`
/// type of either version reads as its `work.*` name ([`work_type`]).
pub fn parse_line(line: &str) -> Result<JournalEvent, String> {
    let value: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
    let members: &[&str] = match value.get("v").and_then(serde_json::Value::as_u64) {
        Some(1) => &V1_MEMBERS,
        Some(2) => &V2_MEMBERS,
        Some(v) if v > 2 => {
            return Err(format!(
                "{NEWER}: envelope version {v}; this reader knows v1 and v2 (FORMAT-1.0 §11.2), \
                 so what the line says is UNKNOWN, not malformed"
            ));
        }
        _ => return Err("`v` is neither 1 nor 2 (FORMAT-1.0 §11.2)".to_owned()),
    };
    // §11.2: a line lacking a member is malformed.
    if let Some(missing) = members.iter().find(|m| value.get(**m).is_none()) {
        return Err(format!("the event lacks `{missing}` (FORMAT-1.0 §11.2)"));
    }
    let mut event = if value["v"] == 2 {
        let e: EnvelopeV2 = serde_json::from_value(value).map_err(|e| e.to_string())?;
        if !e.payload.is_object() {
            return Err("a v2 event's payload is a JSON object".to_owned());
        }
        JournalEvent {
            v: 2,
            id: e.id,
            warrant_uuid: e.document_uuid,
            event_type: e.event_type,
            class: e.class,
            actor_ref: e.actor_ref,
            occurred_at: e.occurred_at,
            payload: serde_jcs::to_string(&e.payload).map_err(|e| e.to_string())?,
            idempotency_key: e.idempotency_key,
        }
    } else {
        serde_json::from_str::<JournalEvent>(line).map_err(|e| e.to_string())?
    };
    // §11.2: a key that does not match its content is malformed. A v1 key
    // is over the payload text as written, a v2 key over its RFC 8785 bytes.
    let want = sha256_hex(
        format!(
            "{}\n{}\n{}",
            event.warrant_uuid, event.event_type, event.payload
        )
        .as_bytes(),
    );
    if event.idempotency_key != want {
        return Err(format!(
            "event {}: its idempotency_key does not match its document, type and payload \
             (FORMAT-1.0 §11.2)",
            event.id
        ));
    }
    if let std::borrow::Cow::Owned(t) = work_type(&event.event_type) {
        event.event_type = t;
    }
    Ok(event)
}

/// `work.journal-newer` (FORMAT-1.0 §11.2): a line whose `v` is greater
/// than 2 is `UNKNOWN`, never malformed.
pub const NEWER: &str = "work.journal-newer";

/// Whether a journal error is a newer envelope's (`UNKNOWN`), not a fault.
#[must_use]
pub fn is_newer(error: &str) -> bool {
    error.contains(NEWER)
}

const V1_MEMBERS: [&str; 9] = [
    "v",
    "id",
    "warrant_uuid",
    "type",
    "class",
    "actor_ref",
    "occurred_at",
    "payload",
    "idempotency_key",
];
const V2_MEMBERS: [&str; 9] = [
    "v",
    "id",
    "document_uuid",
    "type",
    "class",
    "actor_ref",
    "occurred_at",
    "payload",
    "idempotency_key",
];

/// Whether `current` keeps every line of `committed` unchanged and in the
/// same order (FORMAT-1.0 §11.2): lines added between them, as a union
/// merge adds them, are no change. `Err` names the first committed event
/// missing, changed or out of order.
pub fn extends(committed: &[JournalEvent], current: &[JournalEvent]) -> Result<(), String> {
    extends_at(committed, current).map_err(|(_, e)| e)
}

/// [`extends`], naming the position (0-based, among the committed events)
/// of the first committed event that is gone.
pub fn extends_at(
    committed: &[JournalEvent],
    current: &[JournalEvent],
) -> Result<(), (usize, String)> {
    let mut at = 0;
    for (position, old) in committed.iter().enumerate() {
        match current[at..].iter().position(|e| e == old) {
            Some(k) => at += k + 1,
            None => {
                return Err((
                    position,
                    format!(
                        "the journal is not append-only: committed event {:?} at position \
                         {position} is missing, changed or out of order. A journal that can be \
                         rewritten records what someone currently believes, not what happened",
                        old.id
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// The 1-based line of a journal's text that holds its `position`-th event
/// (0-based): blank lines hold none.
#[must_use]
pub fn line_of_event(text: &str, position: usize) -> Option<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .nth(position)
        .map(|(n, _)| n + 1)
}

/// The line a journal error names (`line N: ...`, what [`parse`] gives).
#[must_use]
pub fn error_line(error: &str) -> Option<usize> {
    let rest = error.split("line ").nth(1)?;
    rest.split(':').next()?.trim().parse().ok()
}

/// `<path>:<line>`, or `<path>` alone without a line: where a journal
/// finding is reported (FORMAT-1.0 §13.1: a journal's repository path and
/// the faulty line).
fn at_line(rel: &str, line: Option<usize>) -> String {
    line.map_or_else(|| rel.to_owned(), |n| format!("{rel}:{n}"))
}

/// Of two events about one target, the one that wins (FORMAT-1.0 §11.2):
/// the later `occurred_at`, and of two at one time the greater `id`.
#[must_use]
pub fn later<'a>(a: &'a JournalEvent, b: &'a JournalEvent) -> &'a JournalEvent {
    if (b.occurred_at.as_str(), b.id.as_str()) > (a.occurred_at.as_str(), a.id.as_str()) {
        b
    } else {
        a
    }
}

/// The line an event is written as: a `v: 2` event as RFC 8785 JSON with its
/// payload an object, a `v: 1` event exactly as it always was.
pub fn render_line(ev: &JournalEvent) -> Result<String, String> {
    if ev.v != 2 {
        return serde_json::to_string(ev).map_err(|e| e.to_string());
    }
    let payload: serde_json::Value = serde_json::from_str(&ev.payload)
        .map_err(|e| format!("a v2 event's payload is a JSON object ({e})"))?;
    if !payload.is_object() {
        return Err("a v2 event's payload is a JSON object".to_owned());
    }
    serde_jcs::to_string(&EnvelopeV2 {
        v: 2,
        id: ev.id.clone(),
        document_uuid: ev.warrant_uuid.clone(),
        event_type: ev.event_type.clone(),
        class: ev.class,
        actor_ref: ev.actor_ref.clone(),
        occurred_at: ev.occurred_at.clone(),
        payload,
        idempotency_key: ev.idempotency_key.clone(),
    })
    .map_err(|e| e.to_string())
}

/// Whether two events record the same fact (§11.2's exactly-once): the same
/// document, the same type read as its `work.*` name, and the same payload
/// as RFC 8785 bytes. So a `v: 2` event and a `v: 1` event of one fact are
/// one fact, though their keys are over different bytes.
#[must_use]
pub fn same_fact(a: &JournalEvent, b: &JournalEvent) -> bool {
    a.warrant_uuid == b.warrant_uuid
        && work_type(&a.event_type) == work_type(&b.event_type)
        && canonical_payload(&a.payload) == canonical_payload(&b.payload)
}

/// Parse a journal file. A line that is not an envelope is an error — an
/// unreadable journal is not an empty one.
pub fn parse(text: &str) -> Result<Journal, String> {
    let mut journal = Journal::default();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event = parse_line(line).map_err(|e| format!("line {}: {e}", n + 1))?;
        journal
            .append(event)
            .map_err(|e| format!("line {}: {e}", n + 1))?;
    }
    Ok(journal)
}

/// An incomplete final write (OW-WAR-0121): the last line has no newline.
/// FORMAT-1.0 §11.2 refuses "an unterminated last line"
/// (`work.journal-torn`), whether or not its bytes happen to parse: a
/// writer always ends a line, so a line without its end was not finished,
/// and an append after it would join two events on one line. A bad line
/// with a newline after it, anywhere, is `work.journal-malformed` — it was
/// written whole, and something else is wrong with it.
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
            "{}: line {} ({} byte(s) from offset {}) is an incomplete final \
             write — it has no newline. Every line before it is whole. \
             `truncate -s {} {path}` removes only the unfinished bytes; war does not run it \
             for you (OW-WAR-0121 U-001){}",
            crate::work_rules::JOURNAL_TORN,
            self.line,
            self.len,
            self.offset,
            self.offset,
            crate::work_rules::formerly(crate::work_rules::JOURNAL_TORN)
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
    if last.iter().all(u8::is_ascii_whitespace) {
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
/// error naming `work.journal-torn`, never silently dropped: the journal is
/// read as it is, and repairing it is a person's act.
pub fn load(warrant_dir: &Utf8Path) -> Result<Journal, RepoError> {
    let path = warrant_dir.join(FILE);
    if !crate::vfs::is_file(&path) {
        return Ok(Journal::default());
    }
    // One command reads a journal from several places (its status, its
    // assessment, its frontier), and a journal's lines can carry large
    // payloads. A parse of an unchanged file is remembered for the life of
    // the process while its length and modification time stand. A hosted
    // run (`war host`) has no modification times to key on. Recording also
    // bypasses the cache so its source footprint remains complete. An error
    // is never remembered.
    type Seen =
        std::collections::HashMap<camino::Utf8PathBuf, (u64, std::time::SystemTime, Journal)>;
    static CACHE: std::sync::OnceLock<std::sync::Mutex<Seen>> = std::sync::OnceLock::new();
    let stamp = crate::vfs::is_disk()
        .then(|| std::fs::metadata(&path).ok())
        .flatten()
        .and_then(|m| Some((m.len(), m.modified().ok()?)));
    let cache = CACHE.get_or_init(Default::default);
    if let Some(stamp) = stamp
        && let Ok(seen) = cache.lock()
        && let Some((len, modified, journal)) = seen.get(&path)
        && (*len, *modified) == stamp
    {
        return Ok(journal.clone());
    }
    let journal = load_uncached(&path)?;
    if let Some((len, modified)) = stamp
        && let Ok(mut seen) = cache.lock()
    {
        seen.insert(path, (len, modified, journal.clone()));
    }
    Ok(journal)
}

fn load_uncached(path: &Utf8Path) -> Result<Journal, RepoError> {
    let bytes = crate::vfs::read(path).map_err(|source| RepoError::Io {
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

/// Build one `v: 1` envelope. `occurred_at` is RFC 3339; the idempotency key
/// is the digest of (warrant, type, payload), so the same fact recorded twice
/// is refused rather than duplicated. "Same fact" is deliberate: a gate
/// re-run that mints a receipt with the same digest as the last one attaches
/// nothing new, and is refused; a re-run whose receipt digest differs (it ran
/// at a different time, so it always does) is a new event. Time is not in
/// the key so that a backfill and a live write of the same record cannot
/// both land.
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

/// Build one envelope in `shape` (§11.2). A `v: 1` event is [`event`], its
/// type the name a `v: 1` journal already uses ([`legacy_type`]). A `v: 2`
/// event names the document `document_uuid`, its type the `work.*` name
/// ([`work_type`]), its payload the RFC 8785 bytes of the JSON object
/// given, and its key `sha256(<document_uuid> LF <type> LF <RFC 8785
/// payload>)`. A `v: 2` payload that is not a JSON object is refused.
pub fn event_in(
    shape: Shape,
    document_uuid: &str,
    event_type: &str,
    actor_ref: &str,
    occurred_at: &str,
    payload: &str,
) -> Result<JournalEvent, String> {
    match shape {
        Shape::V1 => Ok(event(
            document_uuid,
            &legacy_type(event_type),
            actor_ref,
            occurred_at,
            payload,
        )),
        Shape::V2 => {
            let value: serde_json::Value = serde_json::from_str(payload)
                .map_err(|e| format!("a v2 event's payload is a JSON object ({e})"))?;
            if !value.is_object() {
                return Err("a v2 event's payload is a JSON object".to_owned());
            }
            let canonical = serde_jcs::to_string(&value).map_err(|e| e.to_string())?;
            let event_type = work_type(event_type).into_owned();
            Ok(JournalEvent {
                v: 2,
                id: WarUuid::mint().to_string(),
                idempotency_key: sha256_hex(
                    format!("{document_uuid}\n{event_type}\n{canonical}").as_bytes(),
                ),
                warrant_uuid: document_uuid.to_owned(),
                event_type,
                class: EventClass::DraftHistory,
                actor_ref: actor_ref.to_owned(),
                occurred_at: occurred_at.to_owned(),
                payload: canonical,
            })
        }
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
///
/// The fact is compared as §11.2 keys it, whichever envelope holds it: the
/// type read as its `work.*` name and the payload as RFC 8785 bytes
/// ([`same_fact`]). A `v: 1` line and a `v: 2` append of one fact are one
/// fact: the same actor replays, another is refused.
pub fn already_recorded(
    warrant_dir: &Utf8Path,
    event_type: &str,
    payload: &str,
    actor_ref: &str,
) -> Result<Prior, RepoError> {
    let journal = load(warrant_dir)?;
    let kind = work_type(event_type);
    let canonical = canonical_payload(payload);
    let Some(prior) = journal
        .events
        .iter()
        .find(|e| e.event_type == kind && canonical_payload(&e.payload) == canonical)
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

/// The 1-based line of the journal in `dir` that already records
/// `event_type` with `payload` (the same fact, §11.2), if one does: where a
/// `work.idempotency-conflict` is reported (FORMAT-1.0 §13.1, "the existing
/// event's line").
#[must_use]
pub fn recorded_line(dir: &Utf8Path, event_type: &str, payload: &str) -> Option<usize> {
    let text = crate::vfs::read_to_string(dir.join(FILE)).ok()?;
    let kind = work_type(event_type);
    let canonical = canonical_payload(payload);
    text.lines().enumerate().find_map(|(n, line)| {
        let e = parse_line(line).ok()?;
        (e.event_type == kind && canonical_payload(&e.payload) == canonical).then_some(n + 1)
    })
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
        "{}: {file} already records `{event_type}` with this exact \
         payload by {recorded_by}, and this act is {actor_ref}. The same fact recorded by a \
         second actor is a conflicting reuse of its idempotency key (§67.4), not a retry; \
         nothing was written{}",
        crate::work_rules::IDEMPOTENCY_CONFLICT,
        crate::work_rules::formerly(crate::work_rules::IDEMPOTENCY_CONFLICT)
    )
}

/// `work.idempotency-conflict` as a diagnostic, for a command that
/// reports rather than returns its refusals.
#[must_use]
pub fn conflict(file: String, event_type: &str, recorded_by: &str, actor_ref: &str) -> Diagnostic {
    let message = conflict_message(event_type, recorded_by, actor_ref, Utf8Path::new(&file));
    let prefix = format!("{}: ", crate::work_rules::IDEMPOTENCY_CONFLICT);
    Diagnostic::error(
        crate::work_rules::IDEMPOTENCY_CONFLICT,
        file,
        message.trim_start_matches(prefix.as_str()),
    )
}

/// Append one event, now, and write ONLY the new line.
///
/// OW-WAR-0130 (§67.4): the same key by the same actor is an equivalent
/// retry and returns [`Recorded::Replayed`] having written nothing; the same
/// key by a different actor is refused `work.idempotency-conflict`.
/// Errors are returned, never swallowed: a command whose effect was recorded
/// but whose journal line was not is a command that partially happened, and
/// the caller decides what that means. A command should ask
/// [`already_recorded`] before its first write, so that it never reaches
/// here with a conflict after writing its record.
///
/// The envelope is the directory's ([`shape_of`]): `v: 2` beside a Format
/// 1.0 work document, `v: 1` beside a legacy one.
pub fn record(
    warrant_dir: &Utf8Path,
    warrant_uuid: &str,
    event_type: &str,
    actor_ref: &str,
    payload: &str,
) -> Result<Recorded, RepoError> {
    record_in(
        shape_of(warrant_dir),
        warrant_dir,
        warrant_uuid,
        event_type,
        actor_ref,
        payload,
    )
}

/// [`record`] in the envelope `shape`, whatever the directory holds.
pub fn record_in(
    shape: Shape,
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
    let ev = event_in(
        shape,
        warrant_uuid,
        event_type,
        actor_ref,
        &occurred_at,
        payload,
    )
    .map_err(|e| RepoError::Message(format!("{}: {e}", warrant_dir.join(FILE))))?;
    append(warrant_dir, ev).map(|()| Recorded::Recorded)
}

fn append(warrant_dir: &Utf8Path, ev: JournalEvent) -> Result<(), RepoError> {
    let mut journal = load(warrant_dir)?;
    if journal.events.iter().any(|e| same_fact(e, &ev)) {
        return Err(RepoError::Message(format!(
            "{}: `{}` with this payload is already journalled (idempotency key {})",
            warrant_dir.join(FILE),
            ev.event_type,
            &ev.idempotency_key[..12]
        )));
    }
    let line = render_line(&ev)
        .map_err(|e| RepoError::Message(format!("could not render the event: {e}")))?;
    let mut read_back = ev.clone();
    if let std::borrow::Cow::Owned(t) = work_type(&read_back.event_type) {
        read_back.event_type = t;
    }
    journal
        .append(read_back)
        .map_err(|e| RepoError::Message(format!("{}: {e}", warrant_dir.join(FILE))))?;
    use std::io::Write;
    let path = warrant_dir.join(FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
            context: format!("could not create {parent}"),
            source,
        })?;
    }
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

/// A journal that does not read: `work.journal-malformed`, or `UNKNOWN`
/// `work.journal-newer` when a line is of an envelope newer than v2
/// (FORMAT-1.0 §11.2).
fn unreadable(name: &str, rel: String, e: &str) -> Diagnostic {
    let rel = if is_newer(e) {
        rel
    } else {
        at_line(&rel, error_line(e))
    };
    if is_newer(e) {
        return Diagnostic::unknown(
            NEWER,
            rel,
            format!(
                "{name}: {e}; every state that depends on this journal is UNKNOWN until a reader \
                 that knows the newer envelope reads it"
            ),
        );
    }
    Diagnostic::error(
        crate::work_rules::JOURNAL_MALFORMED,
        rel,
        format!(
            "{name}: {e} — an unreadable journal is not an empty one{}",
            crate::work_rules::formerly(crate::work_rules::JOURNAL_MALFORMED)
        ),
    )
}

/// `war check`'s rules over the journals of work documents a tracker
/// writes (FORMAT-1.0 §11.2): each journal in `dirs` (`(name, dir)`) is
/// refused `work.journal-torn` for an unterminated last line,
/// `work.journal-malformed` for a line that does not parse, and
/// `work.journal-rewritten` when a committed line was changed or removed.
/// Only refusals are reported. The committed baseline is read only for the
/// journals git says changed since `HEAD` (one `git diff` for them all),
/// since a journal git holds unchanged extends its baseline by definition.
/// With no journals there is nothing to diff, and git is not run: a `git
/// diff HEAD --` with no pathspec reads the whole tree (issue #365).
pub fn check_work(root: &Utf8Path, dirs: &[(String, camino::Utf8PathBuf)], report: &mut Report) {
    if dirs.is_empty() {
        return;
    }
    let relative = |p: &Utf8Path| {
        p.strip_prefix(root)
            .map_or_else(|_| p.to_string(), ToString::to_string)
    };
    let changed: std::collections::BTreeSet<String> = {
        let out = std::process::Command::new("git")
            .args(["diff", "--name-only", "--no-renames", "-z", "HEAD", "--"])
            .args(
                dirs.iter()
                    .map(|(_, d)| format!(":(literal){}", relative(&d.join(FILE)))),
            )
            .current_dir(root)
            .output();
        match out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(str::to_owned)
                .collect(),
            _ => std::collections::BTreeSet::new(),
        }
    };
    for (name, dir) in dirs {
        let path = dir.join(FILE);
        let Ok(bytes) = crate::vfs::read(&path) else {
            continue;
        };
        let rel = relative(&path);
        if let Some(torn) = torn_tail(&bytes) {
            let prefix = format!("{}: ", crate::work_rules::JOURNAL_TORN);
            report.push(Diagnostic::error(
                crate::work_rules::JOURNAL_TORN,
                at_line(&rel, Some(torn.line)),
                format!(
                    "{name}: {}",
                    torn.describe(&rel).trim_start_matches(prefix.as_str())
                ),
            ));
            continue;
        }
        let journal = match std::str::from_utf8(&bytes)
            .map_err(|e| format!("not UTF-8 ({e})"))
            .and_then(parse)
        {
            Ok(j) => j,
            Err(e) => {
                report.push(unreadable(name, rel, &e.to_string()));
                continue;
            }
        };
        if !changed.contains(&rel) {
            continue;
        }
        let Some(base) = std::process::Command::new("git")
            .args(["show", &format!("HEAD:{rel}")])
            .current_dir(root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        else {
            continue;
        };
        if let Ok(prior) = parse(&base)
            && let Err((position, e)) = extends_at(&prior.events, &journal.events)
        {
            report.push(Diagnostic::error(
                crate::work_rules::JOURNAL_REWRITTEN,
                at_line(&rel, line_of_event(&base, position)),
                format!(
                    "{name}: {e}. A journal is extended, never edited; if history must change, \
                     the change is a new event{}",
                    crate::work_rules::formerly(crate::work_rules::JOURNAL_REWRITTEN)
                ),
            ));
        }
    }
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
        let prefix = format!("{}: ", crate::work_rules::JOURNAL_TORN);
        report.push(Diagnostic::error(
            crate::work_rules::JOURNAL_TORN,
            at_line(&rel, Some(torn.line)),
            format!(
                "{alias}: {} (the path is relative to the repository root)",
                torn.describe(&rel).trim_start_matches(prefix.as_str())
            ),
        ));
        return;
    }
    let journal = match load(warrant_dir) {
        Ok(j) => j,
        Err(e) => {
            report.push(unreadable(alias, rel, &e.to_string()));
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
                if let Err((position, e)) = extends_at(&prior.events, &journal.events) {
                    report.push(Diagnostic::error(
                        crate::work_rules::JOURNAL_REWRITTEN,
                        at_line(&rel, line_of_event(&base, position)),
                        format!(
                            "{alias}: {e}. A journal is extended, never edited; if history \
                             must change, the change is a new event{}",
                            crate::work_rules::formerly(crate::work_rules::JOURNAL_REWRITTEN)
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

/// `war admin journal <alias> --backfill`: the events the records already imply.
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

/// `war admin journal <alias>`: print the events.
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

    const DOC_UUID: &str = "01a0d342-ee5e-7b71-9623-f2d8cd5acd4e";

    /// A fresh directory, removed when the guard drops.
    struct Scratch(camino::Utf8PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn scratch(tag: &str) -> (Scratch, camino::Utf8PathBuf) {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8 temp dir")
            .join(format!(
                "war-journal-{tag}-{}-{}",
                std::process::id(),
                N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch dir");
        (Scratch(root.clone()), root)
    }

    #[test]
    fn performance_cache_preserves_recording_journal_reads() {
        use std::{cell::RefCell, rc::Rc};
        let (_guard, root) = scratch("recording-cache");
        let entry = event(
            "fixture",
            DRAFT_CREATED,
            "agent://fixture",
            "2026-10-17T00:00:00Z",
            "{}",
        );
        std::fs::write(
            root.join(FILE),
            format!("{}\n", serde_json::to_string(&entry).unwrap()),
        )
        .unwrap();
        assert_eq!(load(&root).unwrap().events.len(), 1);
        let recorder = Rc::new(RefCell::new(crate::vfs::Recorder {
            root: root.clone(),
            ..Default::default()
        }));
        crate::vfs::recording(Rc::clone(&recorder), || {
            assert_eq!(load(&root).unwrap().events.len(), 1);
        });
        assert!(
            recorder.borrow().files.contains(FILE),
            "a warm journal must not omit its source from a recorded context"
        );
    }

    fn v2(event_type: &str, actor: &str, payload: &str) -> JournalEvent {
        event_in(
            Shape::V2,
            DOC_UUID,
            event_type,
            actor,
            "2026-10-15T00:00:00Z",
            payload,
        )
        .expect("a JSON object payload")
    }

    /// §11.2: a v2 line is RFC 8785 JSON, names the document
    /// `document_uuid`, carries its payload as an object, and keys it over
    /// `<uuid> LF <type> LF <RFC 8785 payload>`; it reads back as it was.
    #[test]
    fn a_v2_event_is_canonical_and_keyed_over_its_canonical_payload() {
        let ev = v2(
            "ticket.claimed",
            "alice",
            r#"{"target":"t-1/i-1","claim":"c1"}"#,
        );
        assert_eq!(
            ev.event_type, "work.claimed",
            "a v2 event takes the work.* name"
        );
        let line = render_line(&ev).unwrap();
        assert!(
            line.starts_with(r#"{"actor_ref":"alice","class":"draft_history","document_uuid":""#),
            "{line}"
        );
        assert!(
            line.contains(r#""payload":{"claim":"c1","target":"t-1/i-1"}"#),
            "{line}"
        );
        assert!(!line.contains("warrant_uuid"), "{line}");
        assert_eq!(
            ev.idempotency_key,
            sha256_hex(
                format!("{DOC_UUID}\nwork.claimed\n{{\"claim\":\"c1\",\"target\":\"t-1/i-1\"}}")
                    .as_bytes()
            )
        );
        assert_eq!(parse_line(&line).unwrap(), ev);
        // Refused: a payload that is not an object, and a version nobody knows.
        assert!(event_in(Shape::V2, DOC_UUID, "work.note_added", "a", "t", "\"text\"").is_err());
        assert!(
            parse_line(&line.replace(
                r#""payload":{"claim":"c1","target":"t-1/i-1"}"#,
                r#""payload":"x""#
            ))
            .is_err()
        );
        assert!(parse_line(&line.replace(r#""v":2"#, r#""v":3"#)).is_err());
    }

    /// §11.2: one journal may hold v1 and v2 lines; a reader reads both,
    /// and a `ticket.*` type as its `work.*` name. A v1 line is never
    /// rewritten: its bytes render as they were.
    #[test]
    fn v1_and_v2_lines_read_in_one_journal() {
        let old = event(
            "u",
            "ticket.claimed",
            "alice",
            "t",
            r#"{"claim":"c1","target":"t-1"}"#,
        );
        let old_line = render_line(&old).unwrap();
        assert!(
            old_line.contains(r#""warrant_uuid":"u""#)
                && old_line.contains(r#""type":"ticket.claimed""#)
        );
        assert_eq!(
            old_line,
            serde_json::to_string(&old).unwrap(),
            "v1 bytes are what they were"
        );
        let new = event_in(
            Shape::V2,
            "u",
            "work.item_done",
            "alice",
            "t",
            r#"{"item":"i-1"}"#,
        )
        .unwrap();
        let text = format!("{old_line}\n{}\n", render_line(&new).unwrap());
        let j = parse(&text).expect("both versions read");
        let types: Vec<&str> = j.events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(types, ["work.claimed", "work.item_done"]);
        assert_eq!((j.events[0].v, j.events[1].v), (1, 2));
        assert!(torn_tail(text.as_bytes()).is_none());
        // A v2 line cut short is a torn tail like any other.
        let cut = &text[..text.len() - 10];
        assert!(torn_tail(cut.as_bytes()).is_some());
    }

    /// §11.2 (draft 3 revision 1): a line lacking a member, with a `v`
    /// other than 1 or 2, or whose key does not match its content is
    /// malformed; a `v` greater than 2 is `work.journal-newer`, UNKNOWN.
    #[test]
    fn a_line_is_whole_and_keyed_or_refused() {
        let good = render_line(&event("u", "ticket.claimed", "a", "t", r#"{"x":1}"#)).unwrap();
        assert!(parse_line(&good).is_ok());
        let lacks = good.replace(r#","idempotency_key":"#, r#","idempotency_kez":"#);
        assert!(
            parse_line(&lacks)
                .unwrap_err()
                .contains("lacks `idempotency_key`")
        );
        let rekeyed = good.replace(r#""payload":"{\"x\":1}""#, r#""payload":"{\"x\":2}""#);
        assert_ne!(rekeyed, good);
        assert!(parse_line(&rekeyed).unwrap_err().contains("does not match"));
        let zero = good.replace(r#""v":1"#, r#""v":0"#);
        let e = parse_line(&zero).unwrap_err();
        assert!(!is_newer(&e) && e.contains("neither 1 nor 2"), "{e}");
        let newer = good.replace(r#""v":1"#, r#""v":3"#);
        let e = parse_line(&newer).unwrap_err();
        assert!(is_newer(&e), "{e}");
        assert_eq!(
            unreadable("t", "j".into(), &e).severity,
            crate::diagnostic::Severity::Unknown
        );
        assert_eq!(
            unreadable("t", "j".into(), "x").rule,
            "work.journal-malformed"
        );
    }

    /// §11.2: rewritten means a committed line missing, changed or out of
    /// order; lines inserted between committed ones (a union merge) are not.
    #[test]
    fn inserted_lines_extend_and_moved_ones_do_not() {
        let a = event("u", "work.a", "x", "t", "{}");
        let b = event("u", "work.b", "x", "t", "{}");
        let c = event("u", "work.c", "x", "t", "{}");
        assert!(extends(&[a.clone(), c.clone()], &[a.clone(), b.clone(), c.clone()]).is_ok());
        assert!(extends(&[a.clone(), c.clone()], &[c.clone(), a.clone()]).is_err());
        assert!(extends(&[a.clone(), b.clone()], std::slice::from_ref(&a)).is_err());
        // Of two conflicting events the later wins, the greater id on a tie.
        let mut early = a.clone();
        early.occurred_at = "2026-01-01T00:00:00Z".into();
        let mut late = b.clone();
        late.occurred_at = "2026-01-02T00:00:00Z".into();
        assert_eq!(later(&late, &early).id, late.id);
        let mut tie = late.clone();
        tie.id = "zzzz".into();
        assert_eq!(later(&late, &tie).id, "zzzz");
    }

    /// A writer keeps a legacy journal one shape: v1, under the `ticket.*`
    /// name, the payload as text.
    #[test]
    fn a_v1_append_keeps_the_ticket_name() {
        let ev = event_in(Shape::V1, "u", "work.claimed", "a", "t", r#"{"b":1,"a":2}"#).unwrap();
        assert_eq!(
            (ev.v, ev.event_type.as_str(), ev.payload.as_str()),
            (1, "ticket.claimed", r#"{"b":1,"a":2}"#)
        );
        assert_eq!(
            ev.idempotency_key,
            sha256_hex(b"u\nticket.claimed\n{\"b\":1,\"a\":2}")
        );
    }

    /// §11.2's exactly-once across versions: a v1 line and a v2 append of
    /// one fact (its type read as `work.*`, its payload as RFC 8785 bytes)
    /// are one fact. The same actor replays; another is refused.
    #[test]
    fn one_fact_in_v1_and_v2_is_recorded_once() {
        let (_guard, dir) = scratch("fact");
        let legacy = event(
            DOC_UUID,
            "ticket.note_added",
            "alice",
            "t",
            r#"{"note":"x","at":"t"}"#,
        );
        std::fs::write(
            dir.join(FILE),
            format!("{}\n", render_line(&legacy).unwrap()),
        )
        .unwrap();
        let same = r#"{"at":"t","note":"x"}"#;
        assert!(matches!(
            already_recorded(&dir, "work.note_added", same, "alice").unwrap(),
            Prior::Equivalent { .. }
        ));
        assert_eq!(
            record_in(Shape::V2, &dir, DOC_UUID, "work.note_added", "alice", same).unwrap(),
            Recorded::Replayed
        );
        let refused =
            record_in(Shape::V2, &dir, DOC_UUID, "work.note_added", "bob", same).unwrap_err();
        assert!(
            refused.to_string().contains("work.idempotency-conflict"),
            "{refused}"
        );
        assert!(
            refused
                .to_string()
                .contains("(formerly journal.idempotency-conflict)"),
            "{refused}"
        );
        // A different fact lands, as v2, beside the v1 line.
        assert_eq!(
            record_in(
                Shape::V2,
                &dir,
                DOC_UUID,
                "work.note_added",
                "bob",
                r#"{"note":"y"}"#
            )
            .unwrap(),
            Recorded::Recorded
        );
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(
            text.lines().next().unwrap().contains(r#""v":1"#),
            "the v1 line is untouched"
        );
        assert!(text.lines().nth(1).unwrap().contains(r#""v":2"#));
    }

    /// The envelope follows the document: v2 beside a Format 1.0 work
    /// document (either form), v1 beside a legacy directory one.
    #[test]
    fn the_shape_is_the_documents() {
        let (_guard, root) = scratch("shape");
        let single = root.join("t-1");
        std::fs::write(root.join("t-1.md"), "---\n").unwrap();
        assert_eq!(shape_of(&single), Shape::V2);
        let dir_form = root.join("t-2");
        std::fs::create_dir_all(&dir_form).unwrap();
        std::fs::write(dir_form.join("ticket.md"), "---\n").unwrap();
        assert_eq!(shape_of(&dir_form), Shape::V2);
        let legacy = root.join("t-3");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("manifest.toml"), "").unwrap();
        assert_eq!(shape_of(&legacy), Shape::V1);
    }

    #[test]
    fn an_unterminated_last_line_is_a_torn_tail_parseable_or_not() {
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

        // Whole and terminated is not torn. FORMAT-1.0 §11.2 refuses "an
        // unterminated last line": one that parses is torn all the same.
        assert!(torn_tail(format!("{a}\n{b}\n").as_bytes()).is_none());
        let whole = torn_tail(format!("{a}\n{b}").as_bytes()).expect("unterminated is torn");
        assert_eq!((whole.line, whole.len), (2, b.len()));
        // The same bytes with a newline after them, or between good lines, are
        // a malformed journal, not a torn one.
        assert!(torn_tail(format!("{a}\n{half}\n").as_bytes()).is_none());
        assert!(torn_tail(format!("{a}\n{half}\n{b}\n").as_bytes()).is_none());
        assert!(parse(&format!("{a}\n{half}\n{b}\n")).is_err());
        // A bad middle line and a torn tail: the middle line wins.
        assert!(torn_tail(format!("{half}\n{a}\n{half}").as_bytes()).is_none());
    }
}
