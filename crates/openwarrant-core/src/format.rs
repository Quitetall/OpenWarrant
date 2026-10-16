// SPDX-License-Identifier: Apache-2.0
//! OpenWarrant Format 1.0 documents (`docs/format/FORMAT-1.0.md`,
//! OW-ADR-0032): the reader.
//!
//! A document is UTF-8 Markdown: a frontmatter in the format's restricted
//! subset (§3.2, not YAML), an optional preamble whose first `# ` heading
//! is the title, and units. Every unindented `## ` line outside a fence
//! opens a unit (§3.4): `## <id> · <type> — <title>` a **record**, any
//! other `## <name>` a **section** with id `S-<slug>`. A record's lines
//! directly after its heading, up to the first blank line, are its
//! attribute block (§3.5): fields, relation lines and two-space
//! continuations. Items are list lines with a box (§3.8).
//!
//! [`parse`] never panics and never fails: what it refuses, could not
//! decide, or warns about is a [`Finding`] naming the rule of §11.2 and a
//! 1-based line. A document with a refusal is not read into the model; one
//! with an `UNKNOWN` finding of the whole document (`format.newer-major`,
//! `format.requires-unknown`) reads nothing (§6, §7.3).
//!
//! Extension names (§7): a dotted frontmatter key or field is carried in
//! `ext` and drives nothing; an undotted name neither the format nor the
//! profile defines is refused (`format.key-unknown`) unless the document
//! declares a newer minor than this reader, when it is ignored and reported
//! `UNKNOWN` (`format.newer-minor`). Ignoring never allows: nothing here
//! turns a refusal into an acceptance because it ignored something.
//!
//! [`content_digest`] is §9's digest over meaning; a unit's revision is
//! `sha256:` of its exact span (§5).
//!
//! Pure: a parse over bytes.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::format_profile::{FieldDecl, Lookup, Profile, Profiles, Shape};
use crate::record::{fence_closes, fence_open};
use crate::relation::{CoreKind, Target, is_record_id};
use crate::ticks::Level;

/// The format this reader implements: `openwarrant/1.0`.
pub const VERSION: &str = "openwarrant/1.0";
/// The major and minor this reader implements.
pub const MAJOR: u32 = 1;
pub const MINOR: u32 = 0;
/// The longest work document `summary` the format allows, in bytes (§3.3).
pub const SUMMARY_MAX: usize = 200;

/// §3.3: the undotted frontmatter keys the core defines.
pub const CORE_KEYS: [&str; 7] = [
    "format", "profile", "alias", "uuid", "summary", "paths", "requires",
];

/// §3.3: the frontmatter keys that are relations of the document's own
/// record, not fields: each value (a scalar or a list) is a target.
pub const RELATION_KEYS: [&str; 4] = ["implements", "depends_on", "supersedes", "part_of"];

/// The extensions this reader understands, for `requires` (§7.3). None:
/// `war` implements no extension's semantics, so a document that lists one
/// as critical reads `UNKNOWN`.
pub const UNDERSTOOD_EXTENSIONS: [&str; 0] = [];

/// The undotted frontmatter keys a built-in profile adds (§3.3).
#[must_use]
pub fn profile_keys(profile: &str) -> &'static [&'static str] {
    match profile {
        "delivery" => &["assurance_level"],
        "ticket" => &[
            "type",
            "priority",
            "labels",
            "created_at",
            "created_by",
            "imported_from",
            "promoted_to",
            "issue",
            "issue_url",
            "due",
        ],
        _ => &[],
    }
}

/// The sections a built-in work profile requires (§4.1), as section ids.
#[must_use]
pub fn required_sections(profile: &str) -> &'static [&'static str] {
    match profile {
        "delivery" => &["S-intent", "S-basis", "S-work-order", "S-assurance"],
        "decision" => &["S-intent", "S-basis"],
        "ticket" => &["S-checklist"],
        _ => &[],
    }
}

/// The undotted fields a core type defines (§3.7, §4.1, §12), or `None` for
/// a type whose fields come from a profile that does not enumerate them.
#[must_use]
pub fn type_fields(record_type: &str) -> Option<&'static [&'static str]> {
    match record_type {
        "obligation" => Some(&[
            "scope",
            "evidence",
            "refusal",
            "gate",
            "scope_kind",
            "criticality",
        ]),
        "stage" => Some(&["executor", "tier", "budget_tokens", "context"]),
        "milestone" => Some(&[]),
        "check" => Some(&["run", "on", "timeout", "gating", "reopen"]),
        "kpi" => Some(&[
            "run",
            "on",
            "timeout",
            "gating",
            "reopen",
            "direction",
            "target",
            "mode",
        ]),
        _ => None,
    }
}

/// The record types the core defines (§3.4, §3.7, §3.8, §4.1, §12).
pub const CORE_TYPES: [&str; 7] = [
    "section",
    "item",
    "obligation",
    "milestone",
    "stage",
    "check",
    "kpi",
];

/// §3.7: the scope kinds an obligation may name.
pub const SCOPE_KINDS: [&str; 9] = [
    "single_instance",
    "enumerated_set",
    "bounded_domain",
    "bounded_corpus",
    "sampled_population",
    "temporal_window",
    "existential",
    "universal",
    "formal_model",
];

/// §12: the triggers a check fires on, besides `paths:<glob>`,
/// `ci:<name>`, `schedule:<cron>` and a dotted extension trigger.
pub const TRIGGERS: [&str; 4] = ["manual", "done", "commit", "pr.merged"];

/// The declaration of a core type's field (§3.7, §12): whether it is
/// REQUIRED and its shape. A check's `on` has a shape no profile shape
/// names; [`core_shape`] checks it.
#[must_use]
pub fn core_fields(record_type: &str) -> Option<BTreeMap<String, FieldDecl>> {
    let names = type_fields(record_type)?;
    let decl = |name: &str| -> FieldDecl {
        let e = |values: &[&str]| FieldDecl {
            required: false,
            shape: Shape::Enum,
            values: values.iter().map(|v| (*v).to_owned()).collect(),
        };
        let plain = |required, shape| FieldDecl {
            required,
            shape,
            values: Vec::new(),
        };
        match (record_type, name) {
            ("obligation", "scope" | "evidence") | ("check" | "kpi", "run") => {
                plain(true, Shape::Text)
            }
            ("obligation", "gate") => plain(false, Shape::Gate),
            ("obligation", "scope_kind") => e(&SCOPE_KINDS),
            ("obligation", "criticality") => e(&["required", "advisory"]),
            ("check" | "kpi", "timeout") => plain(false, Shape::Int),
            ("check" | "kpi", "gating" | "reopen") => plain(false, Shape::Bool),
            ("kpi", "target") => plain(false, Shape::Number),
            ("kpi", "direction") => FieldDecl {
                required: true,
                ..e(&["min", "max"])
            },
            ("kpi", "mode") => e(&["threshold", "best", "optimise"]),
            _ => plain(false, Shape::Text),
        }
    };
    Some(names.iter().map(|n| ((*n).to_owned(), decl(n))).collect())
}

/// §12: why a core field's value with a shape of its own is refused: a
/// check's `on` (triggers, comma-separated).
#[must_use]
pub fn core_shape(record_type: &str, field: &str, value: &str) -> Option<String> {
    match (record_type, field) {
        ("check" | "kpi", "on") => value
            .split(',')
            .map(str::trim)
            .find(|t| !is_trigger(t))
            .map(|t| {
                format!(
                    "{t:?} is no trigger: manual, done, commit, paths:<glob>, pr.merged, \
                     ci:<name>, schedule:<five-field cron>, or a dotted extension trigger"
                )
            }),
        _ => None,
    }
}

fn is_trigger(t: &str) -> bool {
    if TRIGGERS.contains(&t) {
        return true;
    }
    if let Some(glob) = t.strip_prefix("paths:") {
        return !glob.is_empty() && !glob.contains(char::is_whitespace);
    }
    if let Some(name) = t.strip_prefix("ci:") {
        return !name.is_empty();
    }
    if let Some(cron) = t.strip_prefix("schedule:") {
        return cron.split_whitespace().count() == 5;
    }
    is_dotted(t)
}

/// Appendix A `semver`: SemVer 2.0.0's `MAJOR.MINOR.PATCH`, then an
/// optional `-<pre>` and `+<build>`.
#[must_use]
pub fn is_semver(v: &str) -> bool {
    let (rest, build) = match v.split_once('+') {
        Some((r, b)) => (r, Some(b)),
        None => (v, None),
    };
    let (core, pre) = match rest.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (rest, None),
    };
    let num_ok = |n: &str| {
        !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && (n == "0" || !n.starts_with('0'))
    };
    let ident_ok = |s: &str| {
        !s.is_empty()
            && s.split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    };
    let nums: Vec<&str> = core.split('.').collect();
    nums.len() == 3
        && nums.iter().all(|n| num_ok(n))
        && pre.is_none_or(ident_ok)
        && build.is_none_or(ident_ok)
}

/// Appendix D: the registered extension prefixes.
pub const REGISTERED_PREFIXES: [&str; 7] = [
    "kf",
    "katana",
    "liminal",
    "blut",
    "contractor",
    "legacy",
    "x",
];

/// Appendix D, §7.1: whether a dotted name's prefix is registered or a
/// reverse domain name. A name of three or more labels whose first is a
/// run of letters (`com.example.risk`) is read as a reverse domain and its
/// last label as the name; a name of two labels has the first as its
/// prefix, which must be registered.
#[must_use]
pub fn prefix_registered(name: &str) -> bool {
    let labels: Vec<&str> = name.split('.').collect();
    let first = labels.first().copied().unwrap_or_default();
    REGISTERED_PREFIXES.contains(&first)
        || (labels.len() >= 3 && first.bytes().all(|b| b.is_ascii_lowercase()))
}

/// §13.2: every rule id of Format 1.0, with what a finding of it does.
/// A reader reports nothing under another id; the test below holds this
/// list to the text of §13.2.
pub const RULES: [(&str, Outcome); 47] = [
    ("format.encoding", Outcome::Refusal),
    ("format.frontmatter", Outcome::Refusal),
    ("format.key-unknown", Outcome::Refusal),
    ("format.heading", Outcome::Refusal),
    ("format.title-missing", Outcome::Refusal),
    ("format.attribute", Outcome::Refusal),
    ("format.field-duplicate", Outcome::Refusal),
    ("format.field-missing", Outcome::Refusal),
    ("format.authored-state", Outcome::Refusal),
    ("format.relation-kind", Outcome::Refusal),
    ("format.relation-target", Outcome::Refusal),
    ("format.item", Outcome::Refusal),
    ("format.id-duplicate", Outcome::Refusal),
    ("format.fence-unclosed", Outcome::Refusal),
    ("format.section-missing", Outcome::Refusal),
    ("format.document-duplicate", Outcome::Refusal),
    ("format.manifest-restates", Outcome::Refusal),
    ("format.unrecognised", Outcome::Refusal),
    ("format.field-shape", Outcome::Refusal),
    ("profile.invalid", Outcome::Refusal),
    ("context.target-unknown", Outcome::Refusal),
    ("context.over-budget", Outcome::Refusal),
    ("apply.stale", Outcome::Refusal),
    ("apply.identity", Outcome::Refusal),
    ("work.stale-revision", Outcome::Refusal),
    ("work.claimed-by-other", Outcome::Refusal),
    ("work.journal-rewritten", Outcome::Refusal),
    ("work.journal-malformed", Outcome::Refusal),
    ("work.journal-torn", Outcome::Refusal),
    ("work.idempotency-conflict", Outcome::Refusal),
    ("format.newer-major", Outcome::Unknown),
    ("format.newer-minor", Outcome::Unknown),
    ("format.requires-unknown", Outcome::Unknown),
    ("format.profile-unknown", Outcome::Unknown),
    ("work.claim-remote-unreachable", Outcome::Unknown),
    ("work.journal-newer", Outcome::Unknown),
    ("check.unknown", Outcome::Unknown),
    ("format.target-unknown", Outcome::Warning),
    ("format.refusal-missing", Outcome::Warning),
    ("format.item-id-missing", Outcome::Warning),
    ("format.legacy-obligation", Outcome::Warning),
    ("context.summary-shape", Outcome::Warning),
    ("context.statement-shape", Outcome::Warning),
    ("context.section-ref", Outcome::Warning),
    ("context.span-unknown", Outcome::Warning),
    ("work.tick-unbacked", Outcome::Warning),
    ("format.prefix-unregistered", Outcome::Warning),
];

/// Fields an obligation may never author (§3.7): state comes from records
/// of acts, never from the document.
pub const AUTHORED_STATE: [&str; 4] = ["disposition", "verdict", "state", "status"];

/// What a finding does to the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// The document (or the construct) is refused: `FAIL`.
    Refusal,
    /// The question could not be decided: `UNKNOWN`, never `PASS`.
    Unknown,
    /// Reported; nothing is refused.
    Warning,
}

/// One refusal, unknown or warning, by rule (§11.2), at a 1-based line.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub outcome: Outcome,
    pub rule: &'static str,
    /// 1-based line; 1 for the document as a whole.
    pub line: usize,
    pub message: String,
}

/// A field's value: a string, or a frontmatter list.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    One(String),
    Many(Vec<String>),
}

impl FieldValue {
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::One(s) => Some(s),
            Self::Many(_) => None,
        }
    }
}

/// One relation line's relation (§3.6): the kind as written and the target
/// as written (a local id is resolved by the caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub kind: String,
    pub target: Target,
    pub line: usize,
}

/// One item (§3.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// `i-<hex>`; `None` for a box line with no id group (reported).
    pub id: Option<String>,
    pub text: String,
    pub done: bool,
    /// `after` references, as written.
    pub after: Vec<String>,
    /// The done suffix (§3.8): who ticked it and on what date, as written.
    pub done_by: Option<String>,
    pub done_on: Option<String>,
    /// The tick level written in the done suffix (` [observed]`,
    /// ` [independent]`, ` [signed]`, §11.4); `None` is `claimed`. What the
    /// file says, not what a journal backs.
    pub level: Option<Level>,
    /// The note after `: `.
    pub note: Option<String>,
    /// `implements <id>` phrases in the text.
    pub implements: Vec<Target>,
    pub line: usize,
    /// `[start, end)` of the line with its terminator.
    pub start: usize,
    pub end: usize,
}

/// One unit: a section or a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// Local id: a record id, or `S-<slug>` for a section.
    pub id: String,
    /// `section` for a section.
    pub record_type: String,
    pub title: String,
    /// 1-based line of the heading.
    pub line: usize,
    /// `[start, end)` byte offsets of the span in the file.
    pub start: usize,
    pub end: usize,
    pub fields: BTreeMap<String, FieldValue>,
    pub ext: BTreeMap<String, FieldValue>,
    /// The 1-based line each field (or dotted field) was authored on.
    pub field_lines: BTreeMap<String, usize>,
    pub relations: Vec<Relation>,
    pub body: String,
    pub items: Vec<Item>,
    /// The local id of the last section opened before this unit.
    pub section: Option<String>,
    /// The heading is spelled with the ASCII separators ` | ` and ` -- `
    /// (§3.4); a writer keeps the spelling it found.
    pub ascii_heading: bool,
}

impl Unit {
    #[must_use]
    pub fn is_section(&self) -> bool {
        self.record_type == "section"
    }
}

/// One parsed document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    /// `(major, minor)` as declared; `(0, 0)` when none could be read.
    pub version: (u32, u32),
    pub profile: String,
    /// The alias, for a work document.
    pub alias: Option<String>,
    /// The undotted frontmatter keys this reader admits, except `format`
    /// (the encoding's declaration, not content: a legacy encoding of the
    /// same work has none, §12).
    pub fields: BTreeMap<String, FieldValue>,
    /// Dotted frontmatter keys (§7.2).
    pub ext: BTreeMap<String, FieldValue>,
    /// The document record's own relations: its `implements`, `depends_on`,
    /// `supersedes` and `part_of` frontmatter keys (§3.3).
    pub relations: Vec<Relation>,
    pub requires: Vec<String>,
    /// The first `# ` heading of the preamble.
    pub title: Option<String>,
    /// The preamble with its title line removed, exactly.
    pub body: String,
    /// `[start, end)` of frontmatter and preamble: the document record's own
    /// span, so that editing one unit moves no other revision.
    pub head_end: usize,
    pub units: Vec<Unit>,
    pub findings: Vec<Finding>,
}

impl Document {
    /// Whether anything was refused.
    #[must_use]
    pub fn refused(&self) -> bool {
        self.findings.iter().any(|f| f.outcome == Outcome::Refusal)
    }

    /// Whether the whole document reads `UNKNOWN` (§6, §7.3): nothing of it
    /// may be read as PASS or FAIL.
    #[must_use]
    pub fn unknown(&self) -> bool {
        self.findings.iter().any(|f| {
            f.outcome == Outcome::Unknown
                && matches!(f.rule, "format.newer-major" | "format.requires-unknown")
        })
    }

    /// Whether it is a work document (it has an `alias`).
    #[must_use]
    pub fn is_work(&self) -> bool {
        self.alias.is_some()
    }

    /// The document record's type (§4.1).
    #[must_use]
    pub fn record_type(&self) -> &'static str {
        if self.profile == "ticket" {
            "ticket"
        } else {
            "warrant"
        }
    }

    fn push(&mut self, outcome: Outcome, rule: &'static str, line: usize, message: String) {
        self.findings.push(Finding {
            outcome,
            rule,
            line,
            message,
        });
    }
}

/// `sha256:<hex>` of `bytes`.
#[must_use]
pub fn sha256(bytes: &[u8]) -> String {
    format!(
        "sha256:{}",
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

/// The format version `value` declares: `openwarrant/<major>.<minor>`.
#[must_use]
pub fn parse_version(value: &str) -> Option<(u32, u32)> {
    let rest = value.strip_prefix("openwarrant/")?;
    let (major, minor) = rest.split_once('.')?;
    let num = |s: &str| -> Option<u32> {
        (!s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit())
            && (s == "0" || !s.starts_with('0')))
        .then(|| s.parse().ok())
        .flatten()
    };
    Some((num(major)?, num(minor)?))
}

/// Whether `text` begins with a frontmatter holding a `format:` line: the
/// test that routes a file to this reader rather than a legacy one (§6:
/// "no `format`" reads as a legacy encoding).
#[must_use]
pub fn declares_format(text: &str) -> bool {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return false;
    }
    for l in lines {
        if l.trim_end() == "---" {
            return false;
        }
        if l.starts_with("format:") {
            return true;
        }
    }
    false
}

/// §3.4: the slug of a section name.
#[must_use]
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(c.to_ascii_lowercase());
        } else {
            dash = true;
        }
    }
    out
}

fn is_word(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|c| c.is_ascii_lowercase())
        && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn is_label(s: &str) -> bool {
    let mut c = s.chars();
    c.next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Appendix A `dotted`: two or more labels joined by `.`.
#[must_use]
pub fn is_dotted(s: &str) -> bool {
    s.contains('.') && s.split('.').all(is_label)
}

fn is_name(s: &str) -> bool {
    is_word(s) || is_dotted(s)
}

fn is_uuid(s: &str) -> bool {
    let groups: Vec<&str> = s.split('-').collect();
    groups.len() == 5
        && groups
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(g, n)| g.len() == n && g.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// §5 / Appendix A `ref`, optionally pinned: a record id, `<alias>/<id>`,
/// an alias, an item id, `war://<uuid>`, `md:<file>#<slug>` or `doc:<path>`.
#[must_use]
pub fn parse_target(token: &str) -> Option<Target> {
    if let Some(t) = crate::relation::parse_target(token) {
        return Some(t);
    }
    let (id, pin) = match token.split_once('@') {
        None => (token, None),
        Some((id, pin)) => {
            let hex = pin.strip_prefix("sha256:")?;
            if hex.len() != 64 || !hex.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) {
                return None;
            }
            (id, Some(pin.to_owned()))
        }
    };
    let ok = id.strip_prefix("war://").is_some_and(is_uuid)
        || id.strip_prefix("doc:").is_some_and(is_doc_path)
        || crate::ticket::is_item_id(id)
        || id
            .split_once('/')
            .is_some_and(|(a, b)| is_record_id(a) && crate::ticket::is_item_id(b));
    ok.then(|| Target {
        id: id.to_owned(),
        pin,
    })
}

/// Appendix A `path` after `doc:`: repository-relative, `/`-separated, no
/// empty, `.` or `..` segment, and nothing a target list or pin would split.
fn is_doc_path(p: &str) -> bool {
    !p.is_empty()
        && !p
            .bytes()
            .any(|b| b.is_ascii_whitespace() || matches!(b, b',' | b'@' | b'#' | b'\\'))
        && p.split('/')
            .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

// ---- frontmatter (§3.2, Appendix A) ----------------------------------------

/// A frontmatter value with the line it was declared on.
type FmEntry = (String, FieldValue, usize);

fn construct_of(value: &str) -> Option<&'static str> {
    Some(match value.chars().next()? {
        '&' => "an anchor",
        '*' => "an alias",
        '!' => "a tag",
        '{' => "a flow mapping",
        '[' => "a flow sequence",
        '|' => "a block scalar",
        '>' => "a folded scalar",
        _ => return None,
    })
}

/// One scalar (Appendix A `scalar`), or why it is refused.
fn scalar(raw: &str) -> Result<String, String> {
    if let Some(rest) = raw.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = rest.chars();
        while let Some(c) = chars.next() {
            match c {
                '\\' => match chars.next() {
                    Some(e @ ('"' | '\\')) => out.push(e),
                    other => {
                        return Err(format!(
                            "the escape `\\{}`; a double-quoted scalar allows only `\\\"` and `\\\\`",
                            other.map(String::from).unwrap_or_default()
                        ));
                    }
                },
                '"' => {
                    return if chars.as_str().trim_end().is_empty() {
                        Ok(out)
                    } else {
                        Err("text after a closing quote".to_owned())
                    };
                }
                c => out.push(c),
            }
        }
        return Err("an unterminated double-quoted scalar".to_owned());
    }
    if let Some(rest) = raw.strip_prefix('\'') {
        let mut out = String::new();
        let mut chars = rest.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\'' {
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    out.push('\'');
                    continue;
                }
                let tail: String = chars.collect();
                return if tail.trim_end().is_empty() {
                    Ok(out)
                } else {
                    Err("text after a closing quote".to_owned())
                };
            }
            out.push(c);
        }
        return Err("an unterminated single-quoted scalar".to_owned());
    }
    if let Some(c) = construct_of(raw) {
        return Err(format!("{c}; frontmatter is a restricted subset, not YAML"));
    }
    if raw.starts_with(['%', '@', '`']) || raw.starts_with("- ") {
        return Err(format!("a plain scalar may not begin {:?}", &raw[..1]));
    }
    if raw.contains(": ") || raw.contains(" #") {
        return Err("a plain scalar holding `: ` or ` #`; quote it".to_owned());
    }
    Ok(raw.trim_end().to_owned())
}

/// The frontmatter's entries, or the refusal (`format.frontmatter`) and its
/// line. `lines` are the frontmatter's lines between the fences, each with
/// its 1-based line number.
fn frontmatter(lines: &[(usize, &str)]) -> Result<Vec<FmEntry>, (usize, String)> {
    let mut out: Vec<FmEntry> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut list_open = false;
    for &(n, raw) in lines {
        let line = raw.trim_end_matches(['\n', '\r']);
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(item) = line.strip_prefix("  - ") {
            if !list_open {
                return Err((n, "a list item under no `key:` list head".to_owned()));
            }
            let v = scalar(item).map_err(|e| (n, e))?;
            if let Some((_, FieldValue::Many(items), _)) = out.last_mut() {
                items.push(v);
            }
            continue;
        }
        list_open = false;
        if line.starts_with([' ', '\t']) {
            return Err((
                n,
                "an indented line: a nested mapping (or a list item not written `  - `); \
                 frontmatter is flat"
                    .to_owned(),
            ));
        }
        let Some((key, rest)) = line.split_once(':') else {
            return Err((n, format!("{line:?} is not `key: value` or `key:`")));
        };
        if !is_name(key) {
            return Err((
                n,
                format!("{key:?} is not a key: a lowercase word or a dotted name"),
            ));
        }
        if !seen.insert(key.to_owned()) {
            return Err((n, format!("the duplicate key `{key}`")));
        }
        if rest.trim().is_empty() {
            list_open = true;
            out.push((key.to_owned(), FieldValue::Many(Vec::new()), n));
            continue;
        }
        let Some(value) = rest.strip_prefix(' ') else {
            return Err((n, format!("`{key}:` must be followed by one space")));
        };
        let v = scalar(value).map_err(|e| (n, e))?;
        out.push((key.to_owned(), FieldValue::One(v), n));
    }
    Ok(out)
}

// ---- the document ----------------------------------------------------------

/// One line of the file: (1-based number, start offset, text with terminator).
fn lines_of(text: &str) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let mut pos = 0;
    for (i, l) in text.split_inclusive('\n').enumerate() {
        out.push((i + 1, pos, l));
        pos += l.len();
    }
    out
}

fn bare(line: &str) -> &str {
    line.trim_end_matches(['\n', '\r'])
}

fn is_blank(line: &str) -> bool {
    bare(line).trim_matches([' ', '\t']).is_empty()
}

/// A unit heading read: `(id, type, title)`, or the refusal `(rule, why)`.
type Heading = Result<(String, String, String), (&'static str, String)>;

/// The two spellings of a record heading's separators (§3.4): the Unicode
/// ` · ` and ` — `, and the ASCII ` | ` and ` -- ` a person can type on any
/// keyboard. The spelling changes the unit's bytes and nothing else.
const UNICODE_SEPARATORS: (&str, &str) = (" \u{b7} ", " \u{2014} ");
const ASCII_SEPARATORS: (&str, &str) = (" | ", " -- ");

/// Whether a record heading's text uses the ASCII separators: it begins
/// `<id> | ` (§3.4). A `## Q&A | notes` heading does not (no record id).
#[must_use]
pub fn is_ascii_heading(rest: &str) -> bool {
    rest.split_once(ASCII_SEPARATORS.0)
        .is_some_and(|(id, _)| is_record_id(id))
}

/// A unit heading: `Ok((id, type, title))` for a record,
/// `Ok((id, "section", name))` for a section, `Err` for a refused one.
fn heading(rest: &str) -> Heading {
    let rest = rest.trim_end_matches([' ', '\t']);
    // §3.4: `## <id> | ` opens an ASCII record heading whatever follows;
    // otherwise a ` · ` anywhere opens a Unicode one.
    let (spelling, other) = if is_ascii_heading(rest) {
        (ASCII_SEPARATORS, UNICODE_SEPARATORS)
    } else if rest.contains(UNICODE_SEPARATORS.0) {
        (UNICODE_SEPARATORS, ASCII_SEPARATORS)
    } else {
        let s = slug(rest);
        if s.is_empty() {
            return Err((
                "format.heading",
                format!("`## {rest}` has an empty slug; a section name needs a letter or digit"),
            ));
        }
        return Ok((format!("S-{s}"), "section".to_owned(), rest.to_owned()));
    };
    let (dot, dash) = spelling;
    let (id, tail) = rest.split_once(dot).unwrap_or_default();
    let is_type = |t: &str| is_word(t) || is_dotted(t);
    let (ty, title) = match tail.split_once(dash) {
        Some((ty, title)) if is_type(ty) => (ty, Some(title)),
        _ => {
            // A heading uses one spelling for both separators (§3.4).
            if tail.split_once(other.1).is_some_and(|(ty, _)| is_type(ty)) {
                return Err((
                    "format.heading",
                    format!(
                        "`## {rest}` mixes the separators: `{}` with `{}`; a heading is \
                         `## <id> · <type> — <title>` or `## <id> | <type> -- <title>`",
                        dot.trim(),
                        other.1.trim()
                    ),
                ));
            }
            match tail.split_once(dash) {
                Some((ty, title)) => (ty, Some(title)),
                None => (tail, None),
            }
        }
    };
    if !is_record_id(id) || !is_type(ty) {
        return Err((
            "format.heading",
            format!(
                "`## {rest}` has `{}` and is not a record heading `## <id> · <type> — <title>` \
                 (or `## <id> | <type> -- <title>`): the id like `REQ-pr1`, the type a \
                 lowercase word or a dotted extension type",
                dot.trim()
            ),
        ));
    }
    let Some(title) = title.map(str::trim).filter(|t| !t.is_empty()) else {
        return Err((
            "format.title-missing",
            format!(
                "record {id} has no title; a record heading is `## {id}{dot}{ty}{dash}<title>`"
            ),
        ));
    };
    if title.len() > 120 {
        return Err((
            "format.title-missing",
            format!(
                "record {id}'s title is {} bytes; a title is one line of at most 120",
                title.len()
            ),
        ));
    }
    Ok((id.to_owned(), ty.to_owned(), title.to_owned()))
}

/// The built-in profiles of §7.6, read once.
#[must_use]
pub fn builtin_profiles() -> &'static Profiles {
    static BUILTINS: std::sync::OnceLock<Profiles> = std::sync::OnceLock::new();
    BUILTINS.get_or_init(Profiles::builtins)
}

/// Parse one Format 1.0 document under the built-in profiles alone, as if
/// it were at no particular path.
#[must_use]
pub fn parse(bytes: &[u8]) -> Document {
    parse_with(bytes, builtin_profiles(), "")
}

/// Whether `profile` is, or extends, the profile named `name`.
fn is_profile(profile: Option<&Profile>, name: &str) -> bool {
    profile.is_some_and(|p| p.name == name || p.ancestors.iter().any(|a| a == name))
}

/// Parse one Format 1.0 document at the repository-relative `path` (`""`
/// when it has none), under `profiles`.
#[must_use]
pub fn parse_with(bytes: &[u8], profiles: &Profiles, path: &str) -> Document {
    let mut doc = Document::default();
    // §3.1: UTF-8, no NUL.
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            let line = 1 + bytes[..e.valid_up_to()]
                .iter()
                .filter(|b| **b == b'\n')
                .count();
            doc.push(
                Outcome::Refusal,
                "format.encoding",
                line,
                format!("invalid UTF-8 at byte {}", e.valid_up_to()),
            );
            return doc;
        }
    };
    if let Some(at) = text.find('\0') {
        let line = 1 + text[..at].matches('\n').count();
        doc.push(
            Outcome::Refusal,
            "format.encoding",
            line,
            format!("a NUL byte at byte {at}"),
        );
        return doc;
    }
    let bom = if text.starts_with('\u{feff}') { 3 } else { 0 };
    let lines: Vec<(usize, usize, &str)> = lines_of(text)
        .into_iter()
        .map(|(n, s, l)| {
            if n == 1 && bom > 0 {
                (n, s + bom, &l[bom..])
            } else {
                (n, s, l)
            }
        })
        .collect();

    // §3.2: the frontmatter fences.
    // §6: a document that does not open with `---` declares no format.
    if lines.first().map(|l| bare(l.2)) != Some("---") {
        doc.push(
            Outcome::Refusal,
            "format.unrecognised",
            1,
            "no frontmatter, so no `format:`; a Format 1.0 document opens with `---` and \
             declares `format: openwarrant/1.0`"
                .to_owned(),
        );
        return doc;
    }
    let Some(close) = lines.iter().skip(1).position(|l| bare(l.2) == "---") else {
        doc.push(
            Outcome::Refusal,
            "format.frontmatter",
            1,
            "a missing fence: the frontmatter is never closed by `---`".to_owned(),
        );
        return doc;
    };
    let close = close + 1;
    let fm_lines: Vec<(usize, &str)> = lines[1..close].iter().map(|l| (l.0, l.2)).collect();
    let entries = match frontmatter(&fm_lines) {
        Ok(e) => e,
        Err((line, why)) => {
            doc.push(Outcome::Refusal, "format.frontmatter", line, why);
            return doc;
        }
    };
    let get = |k: &str| entries.iter().find(|(key, _, _)| key == k);

    // §6: the version first, so a newer major reads nothing.
    let Some((_, format_value, format_line)) = get("format") else {
        doc.push(
            Outcome::Refusal,
            "format.unrecognised",
            1,
            "no `format:`; a Format 1.0 document declares `format: openwarrant/1.0`".to_owned(),
        );
        return doc;
    };
    let Some(version) = format_value.as_str().and_then(parse_version) else {
        doc.push(
            Outcome::Refusal,
            "format.unrecognised",
            *format_line,
            format!("`format: {format_value:?}` is not `openwarrant/<major>.<minor>`"),
        );
        return doc;
    };
    doc.version = version;
    if version.0 > MAJOR {
        doc.push(
            Outcome::Unknown,
            "format.newer-major",
            *format_line,
            format!(
                "openwarrant/{}.{} is a newer major than this reader's {VERSION}; nothing of \
                 it is read and every check over it is UNKNOWN",
                version.0, version.1
            ),
        );
        return doc;
    }
    if version.0 < MAJOR {
        doc.push(
            Outcome::Refusal,
            "format.unrecognised",
            *format_line,
            format!(
                "openwarrant/{}.{} names no major this reader knows",
                version.0, version.1
            ),
        );
        return doc;
    }
    let newer_minor = version.1 > MINOR;

    // §7.3: a critical extension this reader does not understand.
    let requires: Vec<String> = match get("requires").map(|e| &e.1) {
        Some(FieldValue::Many(v)) => v.clone(),
        Some(FieldValue::One(v)) => v.split(',').map(|s| s.trim().to_owned()).collect(),
        None => Vec::new(),
    };
    doc.requires.clone_from(&requires);
    let not_understood: Vec<&String> = requires
        .iter()
        .filter(|r| !UNDERSTOOD_EXTENSIONS.contains(&r.as_str()))
        .collect();
    if !not_understood.is_empty() {
        let line = get("requires").map_or(1, |e| e.2);
        doc.push(
            Outcome::Unknown,
            "format.requires-unknown",
            line,
            format!(
                "`requires` lists {} this reader does not understand; every check over the \
                 document is UNKNOWN",
                not_understood
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        return doc;
    }

    // §3.3: the keys.
    doc.profile = get("profile")
        .and_then(|e| e.1.as_str())
        .unwrap_or_default()
        .to_owned();
    // §7.4: the profile that governs it. One a reader cannot find leaves
    // every profile rule over the document UNKNOWN, never PASS.
    let profile: Option<&Profile> = if doc.profile.is_empty() {
        doc.push(
            Outcome::Refusal,
            "format.field-missing",
            1,
            "no `profile:`; every document names the profile that governs it".to_owned(),
        );
        None
    } else {
        match profiles.lookup(&doc.profile) {
            Lookup::Found(p) => Some(p),
            found => {
                doc.push(
                    Outcome::Unknown,
                    "format.profile-unknown",
                    1,
                    format!(
                        "profile `{}` {}; the document is read under the core alone and every \
                         rule of its profile is UNKNOWN",
                        doc.profile,
                        if matches!(found, Lookup::Invalid) {
                            "is refused (`profile.invalid`)"
                        } else {
                            "is no built-in profile and no file `profiles/<name>.toml`"
                        }
                    ),
                );
                None
            }
        }
    };
    let profile_known = profile.is_some() || doc.profile.is_empty();
    doc.alias = get("alias").and_then(|e| e.1.as_str()).map(str::to_owned);
    let own_type = if doc.alias.is_some() {
        doc.record_type()
    } else if is_profile(profile, "spec") {
        "spec"
    } else {
        "document"
    };
    let own_fields = profile.and_then(|p| p.fields_of(own_type));
    let allowed: BTreeSet<&str> = CORE_KEYS
        .iter()
        .copied()
        .chain(profile.into_iter().flat_map(Profile::frontmatter_keys))
        .chain(
            own_fields
                .into_iter()
                .flat_map(|f| f.keys().map(String::as_str)),
        )
        .collect();
    for (key, value, line) in &entries {
        if is_dotted(key) {
            doc.ext.insert(key.clone(), value.clone());
            prefix_warning(&mut doc, key, *line);
        } else if RELATION_KEYS.contains(&key.as_str()) {
            let targets: Vec<&str> = match value {
                FieldValue::One(v) => v.split(", ").collect(),
                FieldValue::Many(v) => v.iter().map(String::as_str).collect(),
            };
            for t in targets {
                match parse_target(t.trim()) {
                    Some(target) => doc.relations.push(Relation {
                        kind: key.clone(),
                        target,
                        line: *line,
                    }),
                    None => doc.push(
                        Outcome::Refusal,
                        "format.relation-target",
                        *line,
                        format!("`{key}: {t}`: {t:?} is not an id (section 5)"),
                    ),
                }
            }
        } else if allowed.contains(key.as_str()) {
            if key != "format" {
                doc.fields.insert(key.clone(), value.clone());
            }
            // §7.5: a declared value has its shape.
            if let Some(decl) = own_fields.and_then(|f| f.get(key)) {
                for v in values_of(value) {
                    if let Some(why) = decl.refuse(v) {
                        doc.push(
                            Outcome::Refusal,
                            "format.field-shape",
                            *line,
                            format!("`{key}`: {why}"),
                        );
                    }
                }
            }
        } else if newer_minor {
            doc.push(
                Outcome::Unknown,
                "format.newer-minor",
                *line,
                format!(
                    "`{key}` is no key of {VERSION}; the document declares openwarrant/{}.{}, \
                     so it is ignored and UNKNOWN, not refused",
                    version.0, version.1
                ),
            );
        } else if !profile_known {
            // The profile might declare it: UNKNOWN, reported once as
            // `format.profile-unknown` above.
            doc.fields.insert(key.clone(), value.clone());
        } else {
            doc.push(
                Outcome::Refusal,
                "format.key-unknown",
                *line,
                format!(
                    "`{key}` is a key neither the format nor profile `{}` defines; an \
                     extension key is dotted (`x.{key}`)",
                    doc.profile
                ),
            );
        }
    }
    // §7.5: a key the profile requires (and §4.5: `version` on SPEC.md).
    let root_spec = path == "SPEC.md" && is_profile(profile, "spec");
    // §4.5: a part needs no `version`, and one it carries is ignored.
    if is_profile(profile, "spec") && !path.is_empty() && !root_spec {
        doc.fields.remove("version");
    }
    let mut required_keys: Vec<&str> = profile
        .map(|p| p.frontmatter_required.iter().map(String::as_str).collect())
        .unwrap_or_default();
    required_keys.extend(
        own_fields
            .into_iter()
            .flatten()
            .filter(|(_, d)| d.required)
            .map(|(k, _)| k.as_str()),
    );
    if root_spec {
        required_keys.push("version");
    }
    required_keys.sort_unstable();
    required_keys.dedup();
    for key in required_keys {
        let missing = get(key).is_none_or(|e| match &e.1 {
            FieldValue::One(v) => v.trim().is_empty(),
            FieldValue::Many(v) => v.is_empty(),
        });
        if missing {
            doc.push(
                Outcome::Refusal,
                "format.field-missing",
                1,
                format!(
                    "profile `{}` requires the key `{key}:`{}",
                    doc.profile,
                    if root_spec && key == "version" {
                        " on SPEC.md: a SemVer label for the specification as a whole"
                    } else {
                        ""
                    }
                ),
            );
        }
    }
    if root_spec
        && let Some((_, FieldValue::One(v), line)) = get("version")
        && !v.trim().is_empty()
        && !is_semver(v)
    {
        doc.push(
            Outcome::Refusal,
            "format.field-shape",
            *line,
            format!("`version: {v}` is not a SemVer label (`0.3.0`)"),
        );
    }
    if doc.is_work() {
        for key in ["uuid", "summary"] {
            if get(key)
                .and_then(|e| e.1.as_str())
                .is_none_or(str::is_empty)
            {
                doc.push(
                    Outcome::Refusal,
                    "format.field-missing",
                    1,
                    format!("a work document declares `{key}:`"),
                );
            }
        }
        // §3.3, §10.1: one line of at most SUMMARY_MAX bytes. The format
        // bounds it and names no refusal for it, so a long one is a warning.
        if let Some((_, value, line)) = get("summary")
            && let Some(s) = value.as_str()
            && s.len() > SUMMARY_MAX
        {
            doc.push(
                Outcome::Warning,
                "context.summary-shape",
                *line,
                format!(
                    "`summary` is {} bytes; one line of 1 to {SUMMARY_MAX} bytes is what the \
                     index shows",
                    s.len()
                ),
            );
        }
    }

    // §3.4: units, fences first (§3.9).
    let body_lines = &lines[close + 1..];
    let mut heads: Vec<(usize, usize, Heading)> = Vec::new();
    let mut fence: Option<((char, usize), usize)> = None;
    let mut legacy_obligations: Vec<usize> = Vec::new();
    for &(n, start, raw) in body_lines {
        let line = bare(raw);
        if let Some((open, _)) = fence {
            if fence_closes(line, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_open(line) {
            fence = Some((open, n));
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            heads.push((n, start, heading(rest)));
        } else if line.starts_with("### OBL-") {
            legacy_obligations.push(n);
        }
    }
    if let Some((_, line)) = fence {
        doc.push(
            Outcome::Refusal,
            "format.fence-unclosed",
            line,
            "a fence opened here is never closed".to_owned(),
        );
        return doc;
    }
    for n in legacy_obligations {
        doc.push(
            Outcome::Warning,
            "format.legacy-obligation",
            n,
            "a `### OBL-` heading is prose in Format 1.0; an obligation is a record \
             `## OBL-NNN · obligation — <title>`"
                .to_owned(),
        );
    }

    // The preamble: its first `# ` heading is the title; the rest the body.
    let head_start = body_lines.first().map_or(text.len(), |l| l.1);
    let head_end = heads.first().map_or(text.len(), |h| h.1);
    doc.head_end = head_end;
    let preamble = &text[head_start..head_end];
    let mut body = String::new();
    let mut in_fence: Option<(char, usize)> = None;
    let mut pos = 0;
    for raw in preamble.split_inclusive('\n') {
        let line = bare(raw);
        let skip = if let Some(open) = in_fence {
            if fence_closes(line, open) {
                in_fence = None;
            }
            false
        } else if let Some(open) = fence_open(line) {
            in_fence = Some(open);
            false
        } else if doc.title.is_none()
            && let Some(t) = line.strip_prefix("# ")
        {
            doc.title = Some(t.trim().to_owned());
            true
        } else {
            false
        };
        if !skip {
            body.push_str(raw);
        }
        pos += raw.len();
    }
    let _ = pos;
    // A blank line between the title and the first prose is the title's,
    // as a blank line after a record's attribute block is the block's.
    doc.body = body;
    if doc.is_work() && doc.title.is_none() {
        doc.push(
            Outcome::Refusal,
            "format.title-missing",
            body_lines.first().map_or(1, |l| l.0),
            "a work document's title is its first `# ` heading, and it has none".to_owned(),
        );
    }

    // Each unit.
    let mut ids: BTreeMap<String, usize> = BTreeMap::new();
    let mut authored: Vec<(String, String)> = Vec::new();
    let mut item_ids: BTreeMap<String, usize> = BTreeMap::new();
    let mut section: Option<String> = None;
    for (i, (n, start, head)) in heads.iter().enumerate() {
        let end = heads.get(i + 1).map_or(text.len(), |h| h.1);
        let (id, record_type, title) = match head {
            Ok(h) => h.clone(),
            Err((rule, why)) => {
                unit_fault(&mut doc, newer_minor, rule, *n, why.clone());
                continue;
            }
        };
        if let Some(first) = ids.get(&id) {
            unit_fault(
                &mut doc,
                newer_minor,
                "format.id-duplicate",
                *n,
                format!("{id} is also the id of the unit at line {first}"),
            );
            continue;
        }
        ids.insert(id.clone(), *n);
        let span_lines: Vec<(usize, usize, &str)> = lines
            .iter()
            .filter(|l| l.1 >= *start && l.1 < end)
            .copied()
            .collect();
        let head_line = span_lines.first().map_or("", |l| bare(l.2));
        let mut unit = Unit {
            id: id.clone(),
            record_type: record_type.clone(),
            title,
            line: *n,
            start: *start,
            end,
            fields: BTreeMap::new(),
            ext: BTreeMap::new(),
            field_lines: BTreeMap::new(),
            relations: Vec::new(),
            body: String::new(),
            items: Vec::new(),
            section: section.clone(),
            ascii_heading: record_type != "section"
                && head_line.strip_prefix("## ").is_some_and(is_ascii_heading),
        };
        // §7.5: the fields declared for its type, by the core and the
        // profile. `None`: neither enumerates them, so any undotted field
        // is admitted (unless the profile is unknown, any is UNKNOWN).
        let declared = declared_fields(&record_type, profile);
        let mut faults: Vec<(&'static str, usize, String)> = Vec::new();
        let mut warnings: Vec<(&'static str, usize, String)> = Vec::new();
        // The attribute block (§3.5), for a record; a section has none.
        let mut body_from = 1;
        if record_type == "section" {
            if span_lines.get(1).is_some_and(|l| is_blank(l.2)) {
                body_from = 2;
            }
            section = Some(id.clone());
        } else {
            attributes(
                &span_lines,
                &mut unit,
                declared.as_ref(),
                &mut body_from,
                &mut faults,
            );
        }
        let body_start = span_lines.get(body_from).map_or(end, |l| l.1);
        unit.body = text[body_start..end].to_owned();
        // Items (§3.8), in the body, outside fences.
        let body_lines: Vec<(usize, usize, &str)> =
            span_lines.iter().skip(body_from).copied().collect();
        items(&body_lines, &mut unit, &mut faults, &mut warnings);
        if let Some(declared) = &declared {
            field_rules(&unit, declared, &mut faults);
        }
        if is_profile(profile, "spec") && record_type != "section" {
            spec_warnings(&unit, text, &mut warnings);
        }
        for (name, line) in &unit.field_lines {
            if is_dotted(name) && !prefix_registered(name) {
                warnings.push(prefix_finding(name, *line));
            }
        }
        for rel in &unit.relations {
            if is_dotted(&rel.kind) && !prefix_registered(&rel.kind) {
                warnings.push(prefix_finding(&rel.kind, rel.line));
            }
        }
        if is_dotted(&record_type) && !prefix_registered(&record_type) {
            warnings.push(prefix_finding(&record_type, *n));
        }
        // §7.5: a closed set of record types.
        if let Some(p) = profile
            && p.records_closed
            && record_type != "section"
            && !is_dotted(&record_type)
            && !CORE_TYPES.contains(&record_type.as_str())
            && !p.record_types.contains(&record_type)
        {
            faults.push((
                "format.key-unknown",
                *n,
                format!(
                    "`{record_type}` is a record type profile `{}` does not declare, and its \
                     types are closed; an extension type is dotted (`x.{record_type}`)",
                    p.name
                ),
            ));
        }
        // §7.5: a closed set of sections.
        if record_type == "section"
            && let Some(p) = profile
            && p.sections_closed
            && !p
                .sections_required
                .iter()
                .chain(&p.sections_optional)
                .any(|s| format!("S-{}", slug(s)) == id)
        {
            faults.push((
                "format.key-unknown",
                *n,
                format!(
                    "`## {}` is a section profile `{}` does not list, and its sections are \
                     closed",
                    unit.title, p.name
                ),
            ));
        }
        if record_type == "obligation" && !unit.fields.contains_key("refusal") {
            warnings.push((
                "format.refusal-missing",
                *n,
                format!(
                    "obligation {id} names no `refusal:`: a claim nothing was seen to reject is \
                     satisfiable by a control that accepts everything"
                ),
            ));
        }
        for item in &unit.items {
            if let Some(iid) = &item.id {
                if let Some(first) = item_ids.get(iid) {
                    faults.push((
                        "format.id-duplicate",
                        item.line,
                        format!("item {iid} is also on line {first}"),
                    ));
                } else {
                    item_ids.insert(iid.clone(), item.line);
                }
            }
        }
        for (rule, line, msg) in warnings {
            doc.push(Outcome::Warning, rule, line, msg);
        }
        // What was authored, refused or not: a section the document has is
        // never also reported missing (§4.1).
        authored.push((unit.id.clone(), unit.record_type.clone()));
        if faults.is_empty() {
            doc.units.push(unit);
        } else {
            for (rule, line, msg) in faults {
                unit_fault(&mut doc, newer_minor, rule, line, msg);
            }
        }
    }

    // §4.1, §4.5, §7.5: the sections the profile requires (and SPEC.md's
    // Purpose and Scope).
    let mut required: Vec<String> = profile
        .map(|p| p.sections_required.clone())
        .unwrap_or_default();
    if root_spec {
        for s in ["Purpose", "Scope"] {
            if !required.iter().any(|r| r == s) {
                required.push(s.to_owned());
            }
        }
    }
    for s in &required {
        let sid = format!("S-{}", slug(s));
        if !authored.iter().any(|(id, _)| *id == sid) {
            doc.push(
                Outcome::Refusal,
                "format.section-missing",
                1,
                format!(
                    "{} requires the section `## {s}`",
                    if root_spec && (s == "Purpose" || s == "Scope") {
                        "SPEC.md".to_owned()
                    } else {
                        format!("profile `{}`", doc.profile)
                    }
                ),
            );
        }
    }
    if doc.is_work()
        && doc.profile == "delivery"
        && !authored.iter().any(|(_, t)| t == "obligation")
    {
        doc.push(
            Outcome::Refusal,
            "format.section-missing",
            1,
            "profile `delivery` requires at least one obligation record".to_owned(),
        );
    }
    doc.findings.sort();
    doc
}

/// The fields declared for records of `record_type`: the core's (§3.7,
/// §12) and the profile's `[fields.<type>]` (§7.5). A profile adds fields
/// and narrows (a field either requires is required); it never loosens a
/// core shape. `None` when neither declares any.
fn declared_fields(
    record_type: &str,
    profile: Option<&Profile>,
) -> Option<BTreeMap<String, FieldDecl>> {
    let core = core_fields(record_type);
    let own = profile.and_then(|p| p.fields_of(record_type));
    if core.is_none() && own.is_none() {
        return None;
    }
    let mut out = core.unwrap_or_default();
    for (name, d) in own.into_iter().flatten() {
        match out.get_mut(name) {
            // A profile narrows a core field: it may make it REQUIRED, and
            // give `text` a stricter shape, never loosen one.
            Some(c) => {
                c.required |= d.required;
                if c.shape == Shape::Text {
                    c.shape = d.shape;
                    c.values.clone_from(&d.values);
                }
            }
            None => {
                out.insert(name.clone(), d.clone());
            }
        }
    }
    Some(out)
}

/// A field's values: one, or each of a frontmatter list.
fn values_of(value: &FieldValue) -> Vec<&str> {
    match value {
        FieldValue::One(v) => vec![v.as_str()],
        FieldValue::Many(v) => v.iter().map(String::as_str).collect(),
    }
}

/// §7.5, §3.7, §12: each declared field a record lacks is missing when
/// REQUIRED (an empty value is a missing one), and each value it has has
/// its shape. An optional field authored empty is read as absent.
fn field_rules(
    unit: &Unit,
    declared: &BTreeMap<String, FieldDecl>,
    faults: &mut Vec<(&'static str, usize, String)>,
) {
    for (name, decl) in declared {
        let value = unit
            .fields
            .get(name)
            .and_then(FieldValue::as_str)
            .filter(|v| !v.trim().is_empty());
        let Some(value) = value else {
            if decl.required {
                faults.push((
                    "format.field-missing",
                    unit.line,
                    format!(
                        "{} {} has no `{name}:`, or an empty one",
                        unit.record_type, unit.id
                    ),
                ));
            }
            continue;
        };
        let line = unit.field_lines.get(name).copied().unwrap_or(unit.line);
        let why = decl
            .refuse(value)
            .or_else(|| core_shape(&unit.record_type, name, value));
        if let Some(why) = why {
            faults.push((
                "format.field-shape",
                line,
                format!("{} {}: `{name}`: {why}", unit.record_type, unit.id),
            ));
        }
    }
}

/// The longest `statement` a specification's record SHOULD carry (§4.5).
pub const STATEMENT_MAX: usize = 300;

/// Whether `text` holds more than one sentence (§4.5): a `.`, `!` or `?`
/// followed by a space before its end.
fn sentences_over_one(text: &str) -> bool {
    let t = text.trim();
    let b = t.as_bytes();
    b.iter().enumerate().any(|(i, c)| {
        matches!(c, b'.' | b'!' | b'?')
            && b.get(i + 1) == Some(&b' ')
            && !t[i + 1..].trim().is_empty()
    })
}

/// §4.5's context-engineering warnings over one record of a
/// specification: a `statement` of one sentence and at most
/// [`STATEMENT_MAX`] bytes (`context.statement-shape`), and no citation by
/// `§` (`context.section-ref`).
fn spec_warnings(unit: &Unit, text: &str, warnings: &mut Vec<(&'static str, usize, String)>) {
    if let Some(st) = unit.fields.get("statement").and_then(FieldValue::as_str) {
        // §4.5: at the record's heading line.
        let line = unit.line;
        if st.len() > STATEMENT_MAX || sentences_over_one(st) {
            warnings.push((
                "context.statement-shape",
                line,
                format!(
                    "{}'s statement is {} bytes{}; a statement is one sentence of at most \
                     {STATEMENT_MAX} bytes, and its reasons go in `rationale`",
                    unit.id,
                    st.len(),
                    if sentences_over_one(st) {
                        " and more than one sentence"
                    } else {
                        ""
                    }
                ),
            ));
        }
    }
    // §4.5: at the line the `§` is on.
    let span = text.get(unit.start..unit.end).unwrap_or_default();
    let cites = span
        .find('\u{a7}')
        .map(|at| unit.line + span[..at].matches('\n').count());
    if let Some(line) = cites {
        warnings.push((
            "context.section-ref",
            line,
            format!(
                "{} cites by `\u{a7}`; a record of a specification is read alone, so it cites \
                 another by its id",
                unit.id
            ),
        ));
    }
}

/// Appendix D: a dotted name whose prefix is neither registered nor a
/// reverse domain name (`format.prefix-unregistered`, a warning).
fn prefix_finding(name: &str, line: usize) -> (&'static str, usize, String) {
    (
        "format.prefix-unregistered",
        line,
        format!(
            "`{name}`: its prefix is not registered (kf, katana, liminal, blut, contractor, \
             legacy, x; Appendix D) nor a reverse domain name"
        ),
    )
}

fn prefix_warning(doc: &mut Document, name: &str, line: usize) {
    if !prefix_registered(name) {
        let (rule, line, message) = prefix_finding(name, line);
        doc.push(Outcome::Warning, rule, line, message);
    }
}

/// A unit-level refusal; under a newer minor, `UNKNOWN` instead (§6).
fn unit_fault(doc: &mut Document, newer_minor: bool, rule: &'static str, line: usize, why: String) {
    if newer_minor {
        doc.push(
            Outcome::Unknown,
            "format.newer-minor",
            line,
            format!("{rule}: {why}; the document declares a newer minor, so this unit is UNKNOWN"),
        );
    } else {
        doc.push(Outcome::Refusal, rule, line, why);
    }
}

/// §3.5: read the attribute block into `unit`, leaving `body_from` at the
/// index (into `span`) of the body's first line.
fn attributes(
    span: &[(usize, usize, &str)],
    unit: &mut Unit,
    declared: Option<&BTreeMap<String, FieldDecl>>,
    body_from: &mut usize,
    faults: &mut Vec<(&'static str, usize, String)>,
) {
    // (name, dotted) of the field a continuation would extend.
    let mut last: Option<(String, bool)> = None;
    let mut i = 1;
    while i < span.len() {
        let (n, _, raw) = span[i];
        if is_blank(raw) {
            i += 1;
            break;
        }
        let line = bare(raw);
        if let Some(cont) = line.strip_prefix("  ") {
            let Some((name, dotted)) = &last else {
                faults.push((
                    "format.attribute",
                    n,
                    "a continuation under no field".to_owned(),
                ));
                i += 1;
                continue;
            };
            let map = if *dotted {
                &mut unit.ext
            } else {
                &mut unit.fields
            };
            if let Some(FieldValue::One(v)) = map.get_mut(name) {
                if !v.is_empty() {
                    v.push('\n');
                }
                v.push_str(cont);
            }
            i += 1;
            continue;
        }
        last = None;
        // A field: `<name>:` or `<name>: <value>`.
        if let Some((name, rest)) = line.split_once(':')
            && is_name(name)
            && (rest.is_empty() || rest.starts_with(' '))
        {
            let value = rest.strip_prefix(' ').unwrap_or(rest).to_owned();
            let dotted = is_dotted(name);
            if !dotted && unit.record_type == "obligation" && AUTHORED_STATE.contains(&name) {
                faults.push((
                    "format.authored-state",
                    n,
                    format!(
                        "`{name}` is state, never authored on an obligation: a disposition comes \
                         only from a recorded verdict"
                    ),
                ));
            } else if !dotted && declared.is_some_and(|d| !d.contains_key(name)) {
                faults.push((
                    "format.key-unknown",
                    n,
                    format!(
                        "`{name}` is no field of type `{}`; an extension field is dotted \
                         (`x.{name}`)",
                        unit.record_type
                    ),
                ));
            } else if unit.fields.contains_key(name) || unit.ext.contains_key(name) {
                faults.push((
                    "format.field-duplicate",
                    n,
                    format!("`{name}` appears twice in {}", unit.id),
                ));
            } else {
                let map = if dotted {
                    &mut unit.ext
                } else {
                    &mut unit.fields
                };
                map.insert(name.to_owned(), FieldValue::One(value));
                unit.field_lines.insert(name.to_owned(), n);
                last = Some((name.to_owned(), dotted));
            }
            i += 1;
            continue;
        }
        // §3.5: otherwise a lowercase word or a dotted name, one space and
        // text is a relation line, `<kind> <target>[, <target>]…`, whatever
        // follows: its kind is checked (`format.relation-kind`), then each
        // target (`format.relation-target`), so neither is read as prose.
        if let Some((kind, rest)) = line.split_once(' ')
            && is_name(kind)
            && !rest.starts_with(' ')
            && !rest.trim_end_matches([' ', '\t']).is_empty()
        {
            if !(is_dotted(kind) || CoreKind::parse(kind).is_some()) {
                faults.push((
                    "format.relation-kind",
                    n,
                    format!(
                        "`{kind}` is no core relation kind; an extension kind is dotted \
                         (`x.{kind}`)"
                    ),
                ));
            } else {
                for t in rest.trim_end_matches([' ', '\t']).split(", ") {
                    match parse_target(t) {
                        Some(target) => unit.relations.push(Relation {
                            kind: kind.to_owned(),
                            target,
                            line: n,
                        }),
                        None => faults.push((
                            "format.relation-target",
                            n,
                            format!("`{kind} {t}`: {t:?} is not an id (section 5)"),
                        )),
                    }
                }
            }
            i += 1;
            continue;
        }
        faults.push((
            "format.attribute",
            n,
            format!(
                "{line:?} is not a field, relation or continuation; the attribute block ends \
                 at the first blank line"
            ),
        ));
        i += 1;
    }
    *body_from = i;
}

/// §3.8: the items of a unit's body lines.
fn items(
    body: &[(usize, usize, &str)],
    unit: &mut Unit,
    faults: &mut Vec<(&'static str, usize, String)>,
    warnings: &mut Vec<(&'static str, usize, String)>,
) {
    let mut fence: Option<(char, usize)> = None;
    let mut text = String::new();
    let mut map: Vec<(usize, usize, usize)> = Vec::new();
    for &(n, start, raw) in body {
        let line = bare(raw);
        if let Some(open) = fence {
            if fence_closes(line, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_open(line) {
            fence = Some(open);
            continue;
        }
        let t = line.trim_start_matches(' ');
        if !(t.starts_with("- ") || t.starts_with("* ")) {
            continue;
        }
        map.push((n, start, start + raw.len()));
        text.push_str(line);
        text.push('\n');
    }
    if map.is_empty() {
        return;
    }
    let parsed = crate::ticket::parse(&text);
    for f in &parsed.faults {
        let n = map.get(f.line.saturating_sub(1)).map_or(unit.line, |m| m.0);
        match f.rule {
            "ticket.checklist-malformed" => faults.push(("format.item", n, f.message.clone())),
            "ticket.item-duplicate" => {} // reported per document below
            _ => {}
        }
    }
    for it in parsed.items {
        let Some(&(n, start, end)) = map.get(it.line) else {
            continue;
        };
        if it.id.is_none() {
            warnings.push((
                "format.item-id-missing",
                n,
                format!(
                    "the item {:?} has no `(i-…)` id group; a writer assigns one",
                    it.text
                ),
            ));
        }
        // §3.8, §11.4: the done suffix's date, split from its tick level.
        let (done_on, level) = match it.done_on.as_deref() {
            Some(d) => {
                let (date, level) = crate::ticks::split_level(d);
                (Some(date.to_owned()), level)
            }
            None => (None, None),
        };
        unit.items.push(Item {
            implements: crate::relation::implements_phrases(&it.text),
            id: it.id,
            after: it.after.iter().map(ToString::to_string).collect(),
            done_by: it.done_by,
            done_on,
            level,
            note: it.note,
            text: it.text,
            done: it.done,
            line: n,
            start,
            end,
        });
    }
}

// ---- content digest (§9) ---------------------------------------------------

/// §9's body normalization: CRLF to LF, trailing spaces and tabs removed
/// from each line, trailing empty lines removed.
#[must_use]
pub fn normalize_body(body: &str) -> String {
    let lf = body.replace("\r\n", "\n");
    let mut lines: Vec<&str> = lf
        .split('\n')
        .map(|l| l.trim_end_matches([' ', '\t']))
        .collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

/// One outgoing relation in a content payload: `{kind, to, to_revision?}`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ContentRelation {
    pub kind: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_revision: Option<String>,
}

#[derive(Serialize)]
struct ContentPayload<'a> {
    id: &'a str,
    #[serde(rename = "type")]
    kind: &'a str,
    title: &'a str,
    fields: &'a BTreeMap<String, FieldValue>,
    relations: Vec<ContentRelation>,
    body: String,
}

#[derive(Serialize)]
struct Domained<T> {
    digest_domain: &'static str,
    payload: T,
}

/// The digest domain of §9's `content_digest`.
pub const CONTENT_DOMAIN: &str = "oh.war/record-content/v1";

/// §9: `content_digest`, over meaning. `id` is the global id, `title` `""`
/// when absent; `relations` are the record's outgoing relations (sorted
/// here); the body is normalized here.
#[must_use]
pub fn content_digest(
    id: &str,
    record_type: &str,
    title: &str,
    fields: &BTreeMap<String, FieldValue>,
    relations: &[ContentRelation],
    body: &str,
) -> String {
    let mut relations = relations.to_vec();
    relations.sort();
    relations.dedup();
    let value = Domained {
        digest_domain: CONTENT_DOMAIN,
        payload: ContentPayload {
            id,
            kind: record_type,
            title,
            fields,
            relations,
            body: normalize_body(body),
        },
    };
    // Serializing strings, arrays and string maps cannot fail.
    let bytes = serde_jcs::to_vec(&value).unwrap_or_default();
    sha256(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WARRANT: &str = "---\nformat: openwarrant/1.0\nprofile: delivery\nalias: OW-WAR-9001\n\
uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e\nsummary: A bundle carries its output.\n\
assurance_level: basic\n---\n# A bundle carries its output\n\n## Intent\nWhy.\n\n## Basis\n\
Sources.\n\n## Work order\nDo it.\n\n## Assurance\n\n## OBL-001 · obligation — output is carried\n\
scope: one run.\n  No claim about several.\nevidence: the digest matches.\nrefusal: a removed file is absent.\n\
part_of M-1\nx.note: carried\n\n## M-1 · milestone — the bundle\n\n- [ ] write it (i-4d11)\n\
- [x] test it (i-77be, after i-4d11) — done by claude, 2026-09-25: ok\n";

    fn rules(d: &Document) -> Vec<(&'static str, usize)> {
        d.findings.iter().map(|f| (f.rule, f.line)).collect()
    }

    #[test]
    fn a_warrant_reads() {
        let d = parse(WARRANT.as_bytes());
        assert_eq!(rules(&d), vec![]);
        assert_eq!(d.alias.as_deref(), Some("OW-WAR-9001"));
        assert_eq!(d.title.as_deref(), Some("A bundle carries its output"));
        let ids: Vec<&str> = d.units.iter().map(|u| u.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "S-intent",
                "S-basis",
                "S-work-order",
                "S-assurance",
                "OBL-001",
                "M-1"
            ]
        );
        let obl = &d.units[4];
        assert_eq!(
            obl.fields.get("scope"),
            Some(&FieldValue::One(
                "one run.\nNo claim about several.".to_owned()
            ))
        );
        assert_eq!(
            obl.ext.get("x.note"),
            Some(&FieldValue::One("carried".to_owned()))
        );
        assert_eq!(obl.relations[0].target.id, "M-1");
        assert_eq!(obl.section.as_deref(), Some("S-assurance"));
        assert_eq!(d.units[0].body, "Why.\n\n");
        assert_eq!(d.units[5].items.len(), 2);
        assert_eq!(d.units[5].items[1].after, vec!["i-4d11".to_owned()]);
        assert!(!d.fields.contains_key("format"));
    }

    #[test]
    fn refusals_carry_their_rule_and_line() {
        let cases: &[(&str, &str, usize)] = &[
            // No opening fence: no `format:` is declared (§6).
            ("profile: delivery\n", "format.unrecognised", 1),
            // An unclosed one is a frontmatter fault.
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n",
                "format.frontmatter",
                1,
            ),
            // A frontmatter relation key's target must parse (§3.3).
            (
                "---\nformat: openwarrant/1.0\nprofile: p\nimplements: the rule above\n---\n",
                "format.relation-target",
                4,
            ),
            // An empty required field is a missing one (§3.7).
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## OBL-1 · obligation — t\nscope:\nevidence: e\n",
                "format.field-missing",
                5,
            ),
            // An id group whose id is not 3 to 16 hex digits (§5).
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## R-1 · note — t\n\n- [ ] x (i-7)\n",
                "format.item",
                7,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## R-1 · note — t\n\n- [ ] x (i-zzzz)\n",
                "format.item",
                7,
            ),
            (
                "---\nformat: openwarrant/1.0\nx:\n  y: 1\n---\n",
                "format.frontmatter",
                4,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: prd\nsumary: typo\n---\n",
                "format.key-unknown",
                4,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## R-1 · note\n",
                "format.title-missing",
                5,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## r1 · note — t\n",
                "format.heading",
                5,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## R-1 · note — t\nNot an attribute\n",
                "format.attribute",
                6,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## R-1 · note — t\na: 1\na: 2\n",
                "format.field-duplicate",
                7,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## R-1 · note — t\nmentions R-2\n",
                "format.relation-kind",
                6,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## R-1 · note — t\npart_of r2\n",
                "format.relation-target",
                6,
            ),
            (
                // A core kind opens a relation line whatever follows it.
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## R-1 · note — t\nconstrains the rule above\n",
                "format.relation-target",
                6,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## OBL-1 · obligation — t\nscope: s\nevidence: e\nstatus: done\n",
                "format.authored-state",
                8,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n## R-1 · note — t\n\n- [y] x (i-1a2)\n",
                "format.item",
                7,
            ),
            (
                "---\nformat: openwarrant/1.0\nprofile: p\n---\n```\n## R-1 · note — t\n",
                "format.fence-unclosed",
                5,
            ),
        ];
        for &(text, rule, line) in cases {
            let d = parse(text.as_bytes());
            assert!(
                d.findings
                    .iter()
                    .any(|f| f.rule == rule && f.line == line && f.outcome == Outcome::Refusal),
                "{rule}:{line} not in {:?} for {text:?}",
                rules(&d)
            );
        }
        assert!(
            parse(b"---\nformat: openwarrant/1.0\nprofile: p\n---\n\xff\n").findings[0].rule
                == "format.encoding"
        );
    }

    #[test]
    fn frontmatter_relations_and_dotted_types_are_read() {
        let d = parse(
            "---\nformat: openwarrant/1.0\nprofile: p\nimplements: REQ-1, REQ-2\ndepends_on:\n  - t-0e4f\n---\n\
# T\n\n## X-1 · com.example.risk — spam\nscore: high\nconstrains REQ-1\n"
                .as_bytes(),
        );
        assert!(!d.refused(), "{:?}", rules(&d));
        let rels: Vec<(&str, &str)> = d
            .relations
            .iter()
            .map(|r| (r.kind.as_str(), r.target.id.as_str()))
            .collect();
        assert_eq!(
            rels,
            [
                ("implements", "REQ-1"),
                ("implements", "REQ-2"),
                ("depends_on", "t-0e4f")
            ]
        );
        // Relation keys are not fields.
        assert!(!d.fields.contains_key("implements") && !d.fields.contains_key("depends_on"));
        let u = &d.units[0];
        assert_eq!(u.record_type, "com.example.risk");
        assert_eq!(
            u.fields.get("score").and_then(FieldValue::as_str),
            Some("high")
        );
        // The control: an undotted, capitalised type is no record heading.
        let bad = parse(b"---\nformat: openwarrant/1.0\nprofile: p\n---\n## X-1 \xc2\xb7 Risk \xe2\x80\x94 spam\n");
        assert!(
            bad.findings.iter().any(|f| f.rule == "format.heading"),
            "{:?}",
            rules(&bad)
        );
    }

    #[test]
    fn a_refused_unit_is_not_also_missing() {
        // The obligation is refused for its unknown field; the document still
        // has an obligation and an Assurance section, so neither is "missing".
        let d = parse(
            "---\nformat: openwarrant/1.0\nprofile: delivery\nalias: OW-WAR-0001\n\
uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e\nsummary: s\n---\n# T\n\n## Intent\n\n## Basis\n\n\
## Work order\n\n## Assurance\n## OBL-1 · obligation — t\nscope: s\nevidence: e\nrefusal: r\nowner: me\n"
                .as_bytes(),
        );
        assert_eq!(rules(&d), vec![("format.key-unknown", 21)]);
        // With the section and the obligation absent, both are missing.
        let none = parse(
            "---\nformat: openwarrant/1.0\nprofile: delivery\nalias: OW-WAR-0001\n\
uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e\nsummary: s\n---\n# T\n\n## Intent\n\n## Basis\n\n\
## Work order\n"
                .as_bytes(),
        );
        assert_eq!(
            rules(&none),
            vec![("format.section-missing", 1), ("format.section-missing", 1)]
        );
    }

    #[test]
    fn the_accepting_side_of_each_refusal() {
        // A dotted key, a dotted field, a dotted relation kind, a pinned target.
        let d = parse(
            "---\nformat: openwarrant/1.0\nprofile: p\nkf.enterprise_id: E-1\n---\n\
## R-1 · note — t\nx.weight: 3\nx.mentions R-2\nsupports R-2@sha256:0000000000000000000000000000000000000000000000000000000000000000\n"
                .as_bytes(),
        );
        assert!(!d.refused(), "{:?}", rules(&d));
        assert_eq!(
            d.ext.get("kf.enterprise_id"),
            Some(&FieldValue::One("E-1".to_owned()))
        );
        assert_eq!(d.units[0].relations.len(), 2);
    }

    #[test]
    fn versions_and_requires() {
        let major = parse(b"---\nformat: openwarrant/2.0\nwhatever: x\n---\n");
        assert!(major.unknown() && !major.refused());
        assert_eq!(rules(&major), vec![("format.newer-major", 2)]);
        let minor = parse(b"---\nformat: openwarrant/1.1\nprofile: prd\nnewkey: x\n---\n");
        assert!(!minor.refused() && !minor.unknown());
        assert_eq!(rules(&minor), vec![("format.newer-minor", 4)]);
        let req = parse(
            b"---\nformat: openwarrant/1.0\nprofile: p\nrequires:\n  - com.example.risk\n---\n",
        );
        assert!(req.unknown() && !req.refused());
        assert_eq!(rules(&req), vec![("format.requires-unknown", 4)]);
        let none = parse(b"---\nprofile: p\n---\n");
        assert_eq!(rules(&none), vec![("format.unrecognised", 1)]);
    }

    #[test]
    fn content_digest_reads_meaning_not_bytes() {
        let f: BTreeMap<String, FieldValue> = BTreeMap::new();
        let a = content_digest("R-1", "note", "t", &f, &[], "a\nb\n");
        assert_eq!(
            a,
            content_digest("R-1", "note", "t", &f, &[], "a  \r\nb\t\r\n\n\n")
        );
        assert_ne!(a, content_digest("R-1", "note", "t", &f, &[], "a\nc\n"));
        assert_ne!(a, content_digest("R-1", "note", "T", &f, &[], "a\nb\n"));
        assert_ne!(a, content_digest("R-1", "note", "t", &f, &[], "\na\nb\n"));
    }

    fn has(d: &Document, rule: &str, line: usize, outcome: Outcome) -> bool {
        d.findings
            .iter()
            .any(|f| f.rule == rule && f.line == line && f.outcome == outcome)
    }

    const DOC: &str = "---\nformat: openwarrant/1.0\nprofile: prd\n---\n";

    #[test]
    fn an_ascii_heading_is_the_same_record() {
        let uni = parse(
            format!(
                "{DOC}## REQ-001 \u{b7} requirement \u{2014} a link works once\nverify: reuse it\n"
            )
            .as_bytes(),
        );
        let ascii = parse(
            format!("{DOC}## REQ-001 | requirement -- a link works once\nverify: reuse it\n")
                .as_bytes(),
        );
        assert_eq!(rules(&uni), vec![]);
        assert_eq!(rules(&ascii), vec![]);
        let (u, a) = (&uni.units[0], &ascii.units[0]);
        assert_eq!(
            (&u.id, &u.record_type, &u.title, &u.fields),
            (&a.id, &a.record_type, &a.title, &a.fields)
        );
        assert!(a.ascii_heading && !u.ascii_heading);
        // The meaning is one; the bytes (the revision) are not.
        let digest =
            |x: &Unit| content_digest(&x.id, &x.record_type, &x.title, &x.fields, &[], &x.body);
        assert_eq!(digest(u), digest(a));
        let bytes = |d: &str, x: &Unit| sha256(&d.as_bytes()[x.start..x.end]);
        assert_ne!(
            bytes(
                &format!(
                    "{DOC}## REQ-001 \u{b7} requirement \u{2014} a link works once\nverify: reuse it\n"
                ),
                u
            ),
            bytes(
                &format!("{DOC}## REQ-001 | requirement -- a link works once\nverify: reuse it\n"),
                a
            )
        );
        // A title may hold the other spelling's characters.
        let t = parse(format!("{DOC}## R-1 | note -- a \u{b7} b \u{2014} c\n").as_bytes());
        assert_eq!(rules(&t), vec![]);
        assert_eq!(t.units[0].title, "a \u{b7} b \u{2014} c");
        // `## Q&A | notes` begins with no record id: a section.
        let q = parse(format!("{DOC}## Q&A | notes\n").as_bytes());
        assert_eq!(rules(&q), vec![]);
        assert_eq!(q.units[0].id, "S-q-a-notes");
    }

    #[test]
    fn a_heading_mixing_the_spellings_is_refused() {
        for head in [
            "## R-1 | note \u{2014} t",
            "## R-1 \u{b7} note -- t",
            "## R-1 | Bad Type -- t",
            "## R-1 | -- t",
        ] {
            let d = parse(format!("{DOC}{head}\n").as_bytes());
            assert!(
                has(&d, "format.heading", 5, Outcome::Refusal),
                "{head}: {:?}",
                rules(&d)
            );
            assert!(d.units.is_empty(), "{head}");
        }
        let d = parse(format!("{DOC}## R-1 | note\n").as_bytes());
        assert!(
            has(&d, "format.title-missing", 5, Outcome::Refusal),
            "{:?}",
            rules(&d)
        );
    }

    #[test]
    fn a_tick_level_is_read_from_the_done_suffix() {
        let d = parse(
            "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: t-0d61d\nuuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4f\nsummary: s\n---\n# T\n\n## Checklist\n\
- [x] a (i-0a1) \u{2014} done by claude, 2026-10-12 [observed]: ran\n\
- [x] b (i-0b2) \u{2014} done by ana, 2026-10-12 [independent]\n\
- [x] c (i-0c3) -- done by bo, 2026-10-13 [signed]: ok\n\
- [x] d (i-0d4) \u{2014} done by claude, 2026-10-12: claimed\n\
- [ ] e (i-0e5)\n"
                .as_bytes(),
        );
        assert_eq!(rules(&d), vec![]);
        let items = &d.units[0].items;
        type Row<'a> = (&'a str, Option<Level>, Option<&'a str>, Option<&'a str>);
        let got: Vec<Row<'_>> = items
            .iter()
            .map(|i| {
                (
                    i.text.as_str(),
                    i.level,
                    i.done_on.as_deref(),
                    i.note.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                ("a", Some(Level::Observed), Some("2026-10-12"), Some("ran")),
                ("b", Some(Level::Independent), Some("2026-10-12"), None),
                ("c", Some(Level::Signed), Some("2026-10-13"), Some("ok")),
                ("d", None, Some("2026-10-12"), Some("claimed")),
                ("e", None, None, None),
            ]
        );
        assert_eq!(items[1].done_by.as_deref(), Some("ana"));
        // The legacy checklist reads the same line as it always did: the
        // marker rides in its date.
        let legacy = crate::ticket::parse(
            "- [x] a (i-0a1) \u{2014} done by claude, 2026-10-12 [observed]: ran\n",
        );
        assert_eq!(
            legacy.items[0].done_on.as_deref(),
            Some("2026-10-12 [observed]")
        );
    }

    const CHK: &str = "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: t-0d61d\nuuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4f\nsummary: s\n---\n# T\n\n## Checklist\n- [ ] a (i-0a1)\n\n";

    #[test]
    fn check_and_kpi_records_are_validated() {
        let ok = parse(
            format!(
                "{CHK}## CHK-reset \u{b7} check \u{2014} reset tests pass\nrun: cargo test -p accounts reset::\non: done, pr.merged, paths:src/**, ci:build, schedule:0 3 * * 1, x.deploy\nevaluates i-0a1\ntimeout: 600\ngating: false\n\n\
## KPI-p95 \u{b7} kpi \u{2014} p95 latency\nrun: ./bench.sh\ndirection: min\ntarget: 120.5\nmode: threshold\n"
            )
            .as_bytes(),
        );
        assert_eq!(rules(&ok), vec![]);
        assert_eq!(ok.units[1].record_type, "check");
        let cases: &[(&str, &str, usize)] = &[
            (
                "## C-1 \u{b7} check \u{2014} t\non: done\n",
                "format.field-missing",
                13,
            ),
            (
                "## C-1 \u{b7} check \u{2014} t\nrun: x\ntimeout: ten\n",
                "format.field-shape",
                15,
            ),
            (
                "## C-1 \u{b7} check \u{2014} t\nrun: x\ngating: maybe\n",
                "format.field-shape",
                15,
            ),
            (
                "## C-1 \u{b7} check \u{2014} t\nrun: x\non: nightly\n",
                "format.field-shape",
                15,
            ),
            (
                "## C-1 \u{b7} check \u{2014} t\nrun: x\non: schedule:daily\n",
                "format.field-shape",
                15,
            ),
            (
                "## C-1 \u{b7} check \u{2014} t\nrun: x\nowner: me\n",
                "format.key-unknown",
                15,
            ),
            (
                "## K-1 \u{b7} kpi \u{2014} t\nrun: x\n",
                "format.field-missing",
                13,
            ),
            (
                "## K-1 \u{b7} kpi \u{2014} t\nrun: x\ndirection: up\n",
                "format.field-shape",
                15,
            ),
            (
                "## K-1 \u{b7} kpi \u{2014} t\nrun: x\ndirection: max\ntarget: high\n",
                "format.field-shape",
                16,
            ),
            (
                "## K-1 \u{b7} kpi \u{2014} t\nrun: x\ndirection: max\nmode: maximise\n",
                "format.field-shape",
                16,
            ),
        ];
        for &(unit, rule, line) in cases {
            let d = parse(format!("{CHK}{unit}").as_bytes());
            assert!(
                has(&d, rule, line, Outcome::Refusal),
                "{unit:?}: {:?}",
                rules(&d)
            );
        }
    }

    const SPEC: &str = "---\nformat: openwarrant/1.0\nprofile: spec\nuuid: 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e01\nversion: 0.3.0\nsummary: Password reset.\n---\n# Account service\n\n## Purpose\nUsers regain access.\n\n## Scope\nIn: email. Out: SMS.\n\n";

    #[test]
    fn a_specification_reads_and_its_refusals() {
        let req = "## REQ-reset-1 \u{b7} requirement \u{2014} a reset link works once\nstatement: A reset link MUST be accepted at most once and only within 30\n  minutes of issue.\nrationale: a reused link is a takeover path.\nverify: reuse a consumed link; expect refusal.\npriority: must\n\n## CON-1 \u{b7} constraint \u{2014} no new datastore\nstatement: Reset state MUST live in the existing accounts database.\n\n## T-1 \u{b7} term \u{2014} reset link\n\nA one-time URL.\n";
        let ok = parse_with(
            format!("{SPEC}{req}").as_bytes(),
            builtin_profiles(),
            "SPEC.md",
        );
        assert_eq!(rules(&ok), vec![]);
        assert_eq!(
            ok.fields.get("version"),
            Some(&FieldValue::One("0.3.0".to_owned()))
        );
        // A part needs neither `version` nor Purpose and Scope.
        let part = parse_with(
            format!("---\nformat: openwarrant/1.0\nprofile: spec\npart_of: doc:SPEC.md\n---\n# Reset\n\n{req}").as_bytes(),
            builtin_profiles(),
            "spec/reset.md",
        );
        assert_eq!(rules(&part), vec![]);
        // SPEC.md without `version`, without Scope.
        let bare = parse_with(
            b"---\nformat: openwarrant/1.0\nprofile: spec\n---\n# S\n\n## Purpose\nx\n",
            builtin_profiles(),
            "SPEC.md",
        );
        assert_eq!(
            rules(&bare),
            vec![("format.field-missing", 1), ("format.section-missing", 1)]
        );
        let cases: &[(&str, &str, usize)] = &[
            (
                "## REQ-1 \u{b7} requirement \u{2014} t\nverify: v\n",
                "format.field-missing",
                16,
            ),
            (
                "## REQ-1 \u{b7} requirement \u{2014} t\nstatement: s.\npriority: urgent\n",
                "format.field-shape",
                18,
            ),
            (
                "## CON-1 \u{b7} constraint \u{2014} t\nstatement: s.\nowner: me\n",
                "format.key-unknown",
                18,
            ),
        ];
        for &(unit, rule, line) in cases {
            let d = parse_with(
                format!("{SPEC}{unit}").as_bytes(),
                builtin_profiles(),
                "SPEC.md",
            );
            assert!(
                has(&d, rule, line, Outcome::Refusal),
                "{unit:?}: {:?}",
                rules(&d)
            );
        }
        let bad_version = parse_with(
            SPEC.replace("0.3.0", "three").as_bytes(),
            builtin_profiles(),
            "SPEC.md",
        );
        assert!(has(&bad_version, "format.field-shape", 5, Outcome::Refusal));
        // Warnings: a statement of two sentences, one over 300 bytes, a `§`.
        let long = "x".repeat(301);
        let w = parse_with(
            format!(
                "{SPEC}## REQ-1 \u{b7} requirement \u{2014} t\nstatement: It MUST work. It MUST be fast.\n\n\
## REQ-2 \u{b7} requirement \u{2014} t\nstatement: {long}\n\n\
## REQ-3 \u{b7} requirement \u{2014} t\nstatement: It MUST do what \u{a7}4.2 says.\n"
            )
            .as_bytes(),
            builtin_profiles(),
            "SPEC.md",
        );
        assert!(!w.refused(), "{:?}", rules(&w));
        // statement-shape at the heading line; section-ref at the `§`'s.
        assert!(has(&w, "context.statement-shape", 16, Outcome::Warning));
        assert!(has(&w, "context.statement-shape", 19, Outcome::Warning));
        assert!(has(&w, "context.section-ref", 23, Outcome::Warning));
        assert!(
            !ok.findings
                .iter()
                .any(|f| f.rule == "context.statement-shape")
        );
        // A part's `version` is accepted and ignored.
        let part_v = parse_with(
            b"---\nformat: openwarrant/1.0\nprofile: spec\nversion: 9.9.9\npart_of: doc:SPEC.md\n---\n# P\n",
            builtin_profiles(),
            "docs/records/spec/p.md",
        );
        assert_eq!(rules(&part_v), vec![]);
        assert!(!part_v.fields.contains_key("version"));
    }

    #[test]
    fn profile_data_is_enforced() {
        let files: Vec<(&str, &[u8])> = vec![(
            "profiles/runbook.toml",
            b"schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"runbook\"\nform = \"document\"\n\
[frontmatter]\nrequired = [\"service\"]\noptional = [\"owner\"]\n\
[sections]\nrequired = [\"Steps\"]\noptional = [\"Rollback\"]\nclosed = true\n\
[fields.step]\ncommand = { required = true, shape = \"line\" }\ntimeout = { shape = \"int\" }\n",
        )];
        let ps = Profiles::load(files);
        let head = "---\nformat: openwarrant/1.0\nprofile: runbook\nservice: accounts\n---\n# R\n\n## Steps\n";
        let ok = parse_with(
            format!("{head}## ST-1 \u{b7} step \u{2014} restart\ncommand: systemctl restart a\ntimeout: 30\n").as_bytes(),
            &ps,
            "docs/records/ops/r.md",
        );
        assert_eq!(rules(&ok), vec![]);
        let cases: &[(String, &str, usize)] = &[
            (
                head.replace("service: accounts\n", ""),
                "format.field-missing",
                1,
            ),
            (
                head.replace("service: accounts\n", "service: a\ncolour: red\n"),
                "format.key-unknown",
                5,
            ),
            (
                head.replace("## Steps\n", "## Notes\n"),
                "format.section-missing",
                1,
            ),
            (
                head.replace("## Steps\n", "## Steps\n\n## Notes\n"),
                "format.key-unknown",
                10,
            ),
            (
                format!("{head}## ST-1 \u{b7} step \u{2014} t\ntimeout: 30\n"),
                "format.field-missing",
                9,
            ),
            (
                format!("{head}## ST-1 \u{b7} step \u{2014} t\ncommand: c\ntimeout: soon\n"),
                "format.field-shape",
                11,
            ),
            (
                format!("{head}## ST-1 \u{b7} step \u{2014} t\ncommand: c\nowner: me\n"),
                "format.key-unknown",
                11,
            ),
        ];
        for (text, rule, line) in cases {
            let d = parse_with(text.as_bytes(), &ps, "docs/records/ops/r.md");
            assert!(
                has(&d, rule, *line, Outcome::Refusal),
                "{text:?}: {:?}",
                rules(&d)
            );
        }
        // A profile nobody can find: read under the core alone, its rules
        // UNKNOWN, never refused and never a pass.
        let unknown = parse(b"---\nformat: openwarrant/1.0\nprofile: nowhere\ncolour: red\n---\n## R-1 \xc2\xb7 note \xe2\x80\x94 t\nowner: me\n");
        assert_eq!(rules(&unknown), vec![("format.profile-unknown", 1)]);
        assert!(!unknown.refused() && unknown.findings[0].outcome == Outcome::Unknown);
        assert_eq!(unknown.units.len(), 1);
        // A refused profile file: the same, never its built-in namesake.
        let bad = Profiles::load(vec![(
            "profiles/adr.toml",
            b"schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"adr\"\nform = \"document\"\n[sections]\nrequired = [\"Context\"]\n".as_slice(),
        )]);
        let d = parse_with(
            b"---\nformat: openwarrant/1.0\nprofile: adr\n---\n",
            &bad,
            "docs/records/a/x.md",
        );
        assert_eq!(rules(&d), vec![("format.profile-unknown", 1)]);
        // The built-in adr requires its three sections.
        let adr = parse(b"---\nformat: openwarrant/1.0\nprofile: adr\n---\n## Context\n");
        assert_eq!(
            rules(&adr),
            vec![("format.section-missing", 1), ("format.section-missing", 1)]
        );
    }

    #[test]
    fn an_unregistered_prefix_is_a_warning() {
        let d = parse(
            "---\nformat: openwarrant/1.0\nprofile: prd\nacme.owner: me\nkf.enterprise_id: E-1\ncom.example.risk: high\n---\n\
## R-1 \u{b7} acme.risk \u{2014} t\nx.note: fine\nacme.weight: 3\nacme.mentions R-2\n"
            .as_bytes(),
        );
        assert!(!d.refused());
        let lines: Vec<usize> = d
            .findings
            .iter()
            .filter(|f| f.rule == "format.prefix-unregistered")
            .map(|f| f.line)
            .collect();
        assert_eq!(lines, [4, 8, 10, 11]);
        assert!(prefix_registered("x.note") && prefix_registered("com.example.risk"));
        assert!(!prefix_registered("acme.note"));
    }

    #[test]
    fn the_constant_profile_tables_agree_with_the_builtins() {
        for name in crate::format_profile::builtin_names() {
            let p = crate::format_profile::builtin(name).unwrap();
            let keys: Vec<&str> = p.frontmatter_keys().collect();
            let mut want: Vec<&str> = profile_keys(name).to_vec();
            if name == "spec" {
                want.push("version");
            }
            assert_eq!(keys, want, "{name}");
            if p.form == crate::format_profile::Form::Work {
                let sections: Vec<String> = p
                    .sections_required
                    .iter()
                    .map(|s| format!("S-{}", slug(s)))
                    .collect();
                assert_eq!(sections, required_sections(name), "{name}");
            }
        }
    }

    #[test]
    fn the_rule_registry_is_section_13_2() {
        let text = include_str!("../../../docs/format/FORMAT-1.0.md");
        let start = text.find("### 13.2 Rule ids").expect("13.2");
        let end = start + text[start..].find("\n## ").expect("end of 13");
        let part = &text[start..end];
        let block = |from: &str, to: &str| -> BTreeSet<String> {
            let a = part.find(from).expect(from) + from.len();
            let b = if to.is_empty() {
                part.len()
            } else {
                a + part[a..].find(to).expect(to)
            };
            part[a..b]
                .split('`')
                .skip(1)
                .step_by(2)
                .map(str::to_owned)
                .collect()
        };
        let want = [
            (Outcome::Refusal, block("Refusals:", "Unknown:")),
            (Outcome::Unknown, block("Unknown:", "Warnings:")),
            (Outcome::Warning, block("Warnings:", "\n\n")),
        ];
        for (outcome, names) in want {
            let have: BTreeSet<String> = RULES
                .iter()
                .filter(|(_, o)| *o == outcome)
                .map(|(r, _)| (*r).to_owned())
                .collect();
            assert_eq!(have, names, "{outcome:?}");
        }
        // Every rule this reader reports is registered.
        for rule in [
            "format.field-shape",
            "format.profile-unknown",
            "context.statement-shape",
        ] {
            assert!(RULES.iter().any(|(r, _)| *r == rule), "{rule}");
        }
    }

    #[test]
    fn closed_record_types_and_relation_lines() {
        let ps = Profiles::load(vec![(
            "profiles/runbook.toml",
            b"schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"runbook\"\nform = \"document\"\n\
[records]\ntypes = [\"step\"]\nclosed = true\n"
                .as_slice(),
        )]);
        let head = "---\nformat: openwarrant/1.0\nprofile: runbook\n---\n";
        let ok = parse_with(
            format!("{head}## S-1 \u{b7} step \u{2014} t\n\n## X-1 \u{b7} x.note \u{2014} t\n\n## C-1 \u{b7} check \u{2014} t\nrun: x\n").as_bytes(),
            &ps,
            "docs/records/ops/r.md",
        );
        assert_eq!(rules(&ok), vec![]);
        let bad = parse_with(
            format!("{head}## O-1 \u{b7} other \u{2014} t\n").as_bytes(),
            &ps,
            "docs/records/ops/r.md",
        );
        assert_eq!(rules(&bad), vec![("format.key-unknown", 5)]);
        // Without `closed`, any type is admitted (prd declares none).
        assert_eq!(
            rules(&parse(
                format!("{DOC}## O-1 \u{b7} other \u{2014} t\n").as_bytes()
            )),
            vec![]
        );
        // §3.5: a lowercase word, a space and text is a relation line whose
        // kind is checked; anything else is format.attribute.
        let kind = parse(format!("{DOC}## R-1 \u{b7} note \u{2014} t\nblocks REQ-1\n").as_bytes());
        assert_eq!(rules(&kind), vec![("format.relation-kind", 6)]);
        let prose = parse(
            format!("{DOC}## R-1 \u{b7} note \u{2014} t\nblocks the rule above\n").as_bytes(),
        );
        assert_eq!(rules(&prose), vec![("format.relation-kind", 6)]);
        let attr = parse(format!("{DOC}## R-1 \u{b7} note \u{2014} t\nBlocks REQ-1\n").as_bytes());
        assert_eq!(rules(&attr), vec![("format.attribute", 6)]);
        // `legacy` is a registered prefix.
        let legacy = parse(
            format!("{DOC}## R-1 \u{b7} note \u{2014} t\nlegacy.id: WAR-SAS-101.1\n").as_bytes(),
        );
        assert_eq!(rules(&legacy), vec![]);
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("Work order"), "work-order");
        assert_eq!(slug("  Intent!! "), "intent");
        assert_eq!(slug("Q&A 2"), "q-a-2");
        assert_eq!(slug("··"), "");
    }
}
