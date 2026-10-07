// SPDX-License-Identifier: Apache-2.0
//! From a sentence to typed records (OW-WAR-0148 M7; OW-ADR-0031):
//! `war plan "<sentence>" --records` and `war create "<sentence>" --draft
//! --records`. `docs/TYPES.md`, "From a sentence to records", is the guide.
//!
//! # The seam
//!
//! The same drafter `war plan` asks (`[plan] drafter_argv`), handed a
//! different request on stdin:
//!
//! - **`oh.war/records-request/v1`** ([`Request`]): the sentence, the area
//!   the records land in when the caller named one, the governing profile,
//!   the record types and relation kinds that profile declares (and the
//!   relations it requires), the kinds a ticket item may author, and every
//!   record id the program already has, so the drafter can relate to them
//!   and never reuse one.
//! - **`oh.war/records-proposal/v1`** ([`Proposal`]): records (id, type,
//!   title, body, relations), and a ticket whose items `implements` them.
//!   `deny_unknown_fields` throughout: a field this reader does not know is
//!   refused at parse, never dropped. Additive beside
//!   `oh.war/draft-proposal/v2`, which is unchanged; the schema pack is not
//!   touched (draft proposals are not pack members).
//!
//! # Validated before any write, by the rules authored records get
//!
//! The proposal is rendered to the exact record atom `--apply` would write,
//! and that text is read by [`crate::records::load_with`], after every atom
//! on disk: the same parser and the same `record.*` rules `war check`
//! applies — `record.type-undeclared`, `record.relation-kind-unknown`,
//! `record.relation-undeclared`, `record.duplicate-id` (against the
//! proposal and the corpus), `record.relation-required`. A target that is
//! no record of the corpus and not proposed is `record.relation-target-unknown`,
//! which `war check` warns about and a proposal is refused for: a new record
//! is not let in already dangling. Each fault is named; a refused proposal
//! writes nothing.
//!
//! Before that, each record is rendered alone and parsed back: an id or type
//! that is no heading, a relation that does not round-trip, or a body that
//! would open a record or a relation of its own is refused by name
//! (`record.malformed`, `record.relation-kind-unknown`,
//! `record.relation-malformed`, `plan.record-body`), so no prose of the
//! drafter's becomes structure.
//!
//! # What is written
//!
//! `--apply` writes one record atom, `docs/records/<area>/<NN>-<slug>.md`
//! (created, never overwritten), then a ticket through `war create` whose
//! items carry `implements <ID>`. A ticket the store refuses removes the
//! atom again. Applying the same proposal twice is refused by
//! `record.duplicate-id`, one per record, naming where each already is.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8PathBuf;
use openwarrant_core::drafting::InterviewQuestion;
use openwarrant_core::record;
use openwarrant_core::relation::{CoreKind, RelationKind, parse_target};
use serde::{Deserialize, Serialize};

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::records::{Fault, Records, Relation};
use crate::repo::{RepoError, Repository};

pub const REQUEST_API: &str = "oh.war/records-request/v1";
pub const PROPOSAL_API: &str = "oh.war/records-proposal/v1";
pub const APPLIED_SCHEMA: &str = "oh.war/records-applied/v1";
pub const REFUSED_SCHEMA: &str = "oh.war/records-refused/v1";

/// The request a drafter answers with records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub api_version: String,
    pub user_request: String,
    pub namespace: String,
    /// The area the records land in, when the caller named one; otherwise
    /// the drafter may propose one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
    /// The profile that governs the records.
    pub profile: String,
    /// `[records] types` of that profile: every record is one of these.
    pub record_types: Vec<String>,
    /// `[relations] allow`: the core kinds a record may author. A namespaced
    /// kind (`x.mentions`) is always allowed and drives nothing.
    pub relation_kinds: Vec<String>,
    /// `[relations] require`, as `[from type, kind, to type]`.
    pub required_relations: Vec<[String; 3]>,
    /// The kinds a proposal's ticket item may author: `implements`, when the
    /// ticket profile allows it; empty when it does not.
    pub item_relation_kinds: Vec<String>,
    /// Every record the program already has: relate to them, never reuse
    /// their ids.
    pub existing_records: Vec<Existing>,
    /// Interview answers by question id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub answers: BTreeMap<String, String>,
}

/// A record already in the program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Existing {
    pub id: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub area: String,
}

/// What a drafter answers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub api_version: String,
    /// The record atom's heading, and the ticket's title when it names none.
    #[serde(default)]
    pub title: String,
    /// The area, when the request named none (`--area` wins).
    #[serde(default)]
    pub area: Option<String>,
    #[serde(default)]
    pub records: Vec<ProposedRecord>,
    #[serde(default)]
    pub ticket: Option<ProposedTicket>,
    #[serde(default)]
    pub unresolved_questions: Vec<InterviewQuestion>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedRecord {
    pub id: String,
    #[serde(rename = "type")]
    pub record_type: String,
    /// One line: what the record says.
    pub title: String,
    /// Markdown under the title.
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub relations: Vec<ProposedRelation>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedRelation {
    pub kind: String,
    pub target: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedTicket {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub priority: Option<u8>,
    #[serde(default)]
    pub items: Vec<ProposedItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedItem {
    pub text: String,
    /// Record ids, proposed or existing, this item implements.
    #[serde(default)]
    pub implements: Vec<String>,
}

/// A validated proposal: exactly what `--apply` writes.
#[derive(Debug, Clone)]
pub struct Plan {
    pub area: String,
    /// Repository-relative path of the record atom.
    pub path: String,
    pub text: String,
    pub profile: String,
    pub records: Vec<RecordSummary>,
    pub ticket_title: String,
    pub ticket_body: Option<String>,
    pub priority: Option<u8>,
    pub items: Vec<ItemPlan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub revision: String,
    pub relations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemPlan {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub text: String,
    pub implements: Vec<String>,
}

/// What `--apply` produced.
#[derive(Debug, Clone, Serialize)]
pub struct Applied {
    pub schema: &'static str,
    pub area: String,
    pub file: String,
    pub profile: String,
    pub records: Vec<RecordSummary>,
    pub ticket: String,
    pub ticket_title: String,
    pub items: Vec<ItemPlan>,
}

fn fault(rule: &'static str, file: &str, line: usize, message: String) -> Fault {
    Fault {
        rule,
        file: file.to_owned(),
        line,
        message,
    }
}

/// A file or directory name made of a title: lowercase words joined by `-`.
#[must_use]
pub fn slug(text: &str, words: usize) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .take(words)
        .collect::<Vec<_>>()
        .join("-")
}

fn valid_area(area: &str) -> bool {
    !area.is_empty()
        && area.len() <= 64
        && area.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && area
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The records already in record atoms, by a parse of each atom alone:
/// what the request lists. Cheap — no Warrant is loaded.
fn existing(repo: &Repository) -> Vec<Existing> {
    let mut out = Vec::new();
    for path in crate::records::files(repo) {
        let area = path
            .parent()
            .and_then(|p| p.file_name())
            .unwrap_or_default()
            .to_owned();
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(atom) = record::parse(&text) else {
            continue;
        };
        for r in atom.records {
            out.push(Existing {
                id: r.id,
                record_type: r.record_type,
                area: area.clone(),
            });
        }
    }
    out
}

/// Whether the ticket profile lets an item `implements` a record.
fn items_may_implement(repo: &Repository) -> Result<bool, RepoError> {
    let store = crate::ticket::Store::open(repo, None)?;
    Ok(store
        .definition
        .vocabulary
        .allows(&RelationKind::Core(CoreKind::Implements)))
}

/// The records request for `sentence`. Refused, `Err(Diagnostic)`, when the
/// profile governs no records.
pub fn request(
    repo: &Repository,
    sentence: &str,
    profile: &str,
    area: Option<&str>,
    answers: &BTreeMap<String, String>,
) -> Result<Result<Request, Diagnostic>, RepoError> {
    if let Some(a) = area
        && !valid_area(a)
    {
        return Ok(Err(Diagnostic::error(
            "plan.records-area",
            crate::records::DIR,
            format!(
                "--area {a:?} is not an area name: lowercase letters, digits and `-`, like \
                 `password-reset`"
            ),
        )));
    }
    let Some(vocabulary) = repo
        .profiles
        .vocabulary(profile)
        .filter(|v| !v.types.is_empty())
    else {
        let with: Vec<&str> = repo
            .profiles
            .names()
            .into_iter()
            .filter(|n| {
                repo.profiles
                    .vocabulary(n)
                    .is_some_and(|v| !v.types.is_empty())
            })
            .collect();
        return Ok(Err(Diagnostic::error(
            "plan.records-profile",
            format!("profiles/{profile}.toml"),
            format!(
                "profile {profile:?} declares no record types ([records] types), so it governs \
                 no records; name one that does with --profile ({})",
                if with.is_empty() {
                    "none here does".to_owned()
                } else {
                    with.join(", ")
                }
            ),
        )));
    };
    let item_relation_kinds = if items_may_implement(repo)? {
        vec![CoreKind::Implements.as_str().to_owned()]
    } else {
        Vec::new()
    };
    Ok(Ok(Request {
        api_version: REQUEST_API.to_owned(),
        user_request: sentence.to_owned(),
        namespace: repo.config.project.namespace.as_str().to_owned(),
        area: area.map(str::to_owned),
        profile: profile.to_owned(),
        record_types: vocabulary.types.iter().cloned().collect(),
        relation_kinds: vocabulary
            .allow
            .iter()
            .map(|k| k.as_str().to_owned())
            .collect(),
        required_relations: vocabulary
            .require
            .iter()
            .map(|r| [r.from.clone(), r.kind.as_str().to_owned(), r.to.clone()])
            .collect(),
        item_relation_kinds,
        existing_records: existing(repo),
        answers: answers.clone(),
    }))
}

/// One record as it is written: heading, relation lines, title, body.
fn render_record(r: &ProposedRecord) -> String {
    let mut s = format!("## {}{}{}\n", r.id, record::SEPARATOR, r.record_type);
    for rel in &r.relations {
        s.push_str(&format!("{} {}\n", rel.kind, rel.target));
    }
    s.push('\n');
    s.push_str(&openwarrant_core::ticket::one_line(&r.title));
    s.push('\n');
    let body = r.body.trim();
    if !body.is_empty() {
        s.push('\n');
        s.push_str(body);
        s.push('\n');
    }
    s
}

fn frontmatter(profile: &str) -> String {
    format!(
        "---\nschema: {}\nprofile: {profile}\n---\n",
        record::RECORDS_SCHEMA
    )
}

/// The next free `<NN>-` of an area: ten past the highest, else 10.
fn next_path(repo: &Repository, area: &str, title: &str) -> String {
    let dir = repo.root.join(crate::records::DIR).join(area);
    let highest = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            name.split_once('-')?.0.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0);
    let nn = (highest / 10 + 1) * 10;
    let name = match slug(title, 6) {
        s if s.is_empty() => "records".to_owned(),
        s => s,
    };
    format!("{}/{area}/{nn:02}-{name}.md", crate::records::DIR)
}

/// Validate `json` for `request` without writing anything: `Ok(Plan)` is
/// exactly what `--apply` would write; `Err(faults)` names every refusal.
pub fn validate(
    repo: &Repository,
    request: &Request,
    json: &str,
    sentence: &str,
) -> Result<Result<Plan, Vec<Fault>>, RepoError> {
    let proposal: Proposal = match serde_json::from_str(json.trim()) {
        Ok(p) => p,
        Err(e) => {
            return Ok(Err(vec![fault(
                "plan.records-parse",
                "",
                0,
                format!("the proposal is not an {PROPOSAL_API}: {e}"),
            )]));
        }
    };
    let mut faults = Vec::new();
    if proposal.api_version != PROPOSAL_API {
        return Ok(Err(vec![fault(
            "plan.records-api",
            "",
            0,
            format!(
                "api_version {:?}; a records proposal is {PROPOSAL_API}",
                proposal.api_version
            ),
        )]));
    }
    let open: Vec<&InterviewQuestion> = proposal
        .unresolved_questions
        .iter()
        .filter(|q| !request.answers.contains_key(&q.id))
        .collect();
    if !open.is_empty() {
        return Ok(Err(vec![fault(
            "plan.interview-required",
            "",
            0,
            format!(
                "the drafter asked instead of proposing: {}. Nothing was written; answer with \
                 --answer <id>=<text>",
                open.iter()
                    .map(|q| format!("{}: {}", q.id, q.question))
                    .collect::<Vec<_>>()
                    .join(" / ")
            ),
        )]));
    }
    if proposal.records.is_empty() {
        faults.push(fault(
            "plan.records-empty",
            "",
            0,
            "the proposal holds no records; nothing was invented".to_owned(),
        ));
    }
    let title = match openwarrant_core::ticket::one_line(&proposal.title) {
        t if t.is_empty() => openwarrant_core::ticket::one_line(sentence),
        t => t,
    };
    let area = request
        .area
        .clone()
        .or_else(|| proposal.area.clone())
        .unwrap_or_else(|| slug(&title, 4));
    if !valid_area(&area) {
        faults.push(fault(
            "plan.records-area",
            crate::records::DIR,
            0,
            format!(
                "area {area:?} is not an area name: lowercase letters, digits and `-`; name one \
                 with --area"
            ),
        ));
    }
    let ticket = proposal.ticket.clone().unwrap_or_default();
    if ticket.items.is_empty() {
        faults.push(fault(
            "plan.records-no-items",
            "",
            0,
            "the proposal names no ticket items; the records are applied with the ticket that \
             implements them, and nothing was invented"
                .to_owned(),
        ));
    }
    if !faults.is_empty() {
        return Ok(Err(faults));
    }
    let path = next_path(repo, &area, &title);

    // ---- Render, and each record alone must read back as itself.
    let mut text = frontmatter(&request.profile);
    text.push_str(&format!("# {title}\n\n"));
    let asked = openwarrant_core::ticket::one_line(sentence);
    text.push_str(&format!(
        "Drafted{} (`{PROPOSAL_API}`). The ticket that implements these records names them \
         by id.\n",
        if asked.is_empty() {
            String::new()
        } else {
            format!(" from \u{201c}{asked}\u{201d}")
        }
    ));
    for r in &proposal.records {
        text.push('\n');
        let line = text.lines().count() + 1;
        let one = render_record(r);
        text.push_str(&one);
        if openwarrant_core::ticket::one_line(&r.title).is_empty() {
            faults.push(fault(
                "plan.record-title",
                &path,
                line,
                format!(
                    "line {line}: {} has no title; a record says something",
                    r.id
                ),
            ));
            continue;
        }
        let mut shaped = true;
        for rel in &r.relations {
            if let Err(e) = RelationKind::parse(&rel.kind) {
                faults.push(fault(
                    "record.relation-kind-unknown",
                    &path,
                    line,
                    format!("line {line}: {}: {e}", r.id),
                ));
                shaped = false;
            }
            if parse_target(&rel.target).is_none() {
                faults.push(fault(
                    "record.relation-malformed",
                    &path,
                    line,
                    format!(
                        "line {line}: {} {} {:?}: the target is no record id; a target is \
                         `REQ-pr1`, `OW-WAR-0001/OBL-001` or `t-3f2a/i-9c01`, optionally \
                         `@sha256:<64 hex>`",
                        r.id, rel.kind, rel.target
                    ),
                ));
                shaped = false;
            }
        }
        if !shaped {
            continue;
        }
        let alone = format!("{}{one}", frontmatter(&request.profile));
        let fm_lines = frontmatter(&request.profile).lines().count();
        match record::parse(&alone) {
            Err(e) => {
                let at = line + e.line().saturating_sub(fm_lines + 1);
                faults.push(fault(
                    "record.malformed",
                    &path,
                    at,
                    format!("{} (proposed {} · {}): {e}", r.id, r.id, r.record_type),
                ));
            }
            Ok(atom) => {
                let read: Vec<(String, String)> = atom
                    .records
                    .iter()
                    .flat_map(|x| {
                        x.relations
                            .iter()
                            .map(|y| (y.kind.clone(), y.target.id.clone()))
                    })
                    .collect();
                let meant: Vec<(String, String)> = r
                    .relations
                    .iter()
                    .map(|y| {
                        (
                            y.kind.clone(),
                            parse_target(&y.target).map(|t| t.id).unwrap_or_default(),
                        )
                    })
                    .collect();
                let heads: Vec<(&str, &str)> = atom
                    .records
                    .iter()
                    .map(|x| (x.id.as_str(), x.record_type.as_str()))
                    .collect();
                if heads != [(r.id.as_str(), r.record_type.as_str())] || read != meant {
                    faults.push(fault(
                        "plan.record-body",
                        &path,
                        line,
                        format!(
                            "line {line}: {}: its title or body reads as {}; relations go in \
                             `relations`, and prose stays off the `## <ID> · <type>` and \
                             `<kind> <ID>` line shapes",
                            r.id,
                            if heads.len() > 1 {
                                format!(
                                    "another record heading ({})",
                                    heads
                                        .iter()
                                        .skip(1)
                                        .map(|(i, t)| format!("{i} · {t}"))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )
                            } else {
                                format!("relation lines {:?} where {:?} were proposed", read, meant)
                            }
                        ),
                    ));
                }
            }
        }
    }
    if !faults.is_empty() {
        return Ok(Err(faults));
    }

    // ---- The same rules authored records get, over the corpus plus this.
    let corpus = Corpus::new(repo);
    let all = crate::records::load_with(&corpus, &[(path.clone(), text.clone())]);
    faults.extend(all.faults.iter().filter(|f| f.file == path).cloned());
    let proposed_ids: BTreeSet<&str> = proposal.records.iter().map(|r| r.id.as_str()).collect();

    // Items: the ticket profile governs the kind; the targets must resolve.
    let items_may_implement = items_may_implement(repo)?;
    let mut items = Vec::new();
    let mut item_relations = Vec::new();
    for (n, item) in ticket.items.iter().enumerate() {
        let text_one = openwarrant_core::ticket::one_line(&item.text);
        let label = format!("ticket.items[{n}]");
        if text_one.is_empty() {
            faults.push(fault(
                "plan.records-item",
                &label,
                0,
                format!("{label}: an item has no text"),
            ));
            continue;
        }
        let mut implements = Vec::new();
        for target in &item.implements {
            if !items_may_implement {
                faults.push(fault(
                    "record.relation-undeclared",
                    &label,
                    0,
                    format!(
                        "{label}: `implements` is a core relation kind profile ticket does not \
                         allow, so an item cannot implement {target}"
                    ),
                ));
                continue;
            }
            match parse_target(target) {
                Some(t) => {
                    item_relations.push(Relation {
                        from: label.clone(),
                        kind: RelationKind::Core(CoreKind::Implements),
                        target: t,
                        source: label.clone(),
                        line: 0,
                    });
                    implements.push(target.clone());
                }
                None => faults.push(fault(
                    "record.relation-malformed",
                    &label,
                    0,
                    format!("{label}: implements {target:?}, which is no record id"),
                )),
            }
        }
        items.push(ItemPlan {
            id: None,
            text: text_one,
            implements,
        });
    }

    // Targets: a proposed relation must land on a record of the corpus or
    // of the proposal. A proposed record refused above is reported once,
    // there, not again as every relation's unknown target.
    let mine: Vec<Relation> = all
        .relations
        .iter()
        .filter(|r| r.source == path && !r.kind.is_inert())
        .cloned()
        .chain(item_relations)
        .collect();
    let probe = Records {
        records: all.records.clone(),
        relations: mine,
        faults: Vec::new(),
        files: all.files,
        instructions: all.instructions.clone(),
        instruction_faults: Vec::new(),
    };
    for r in crate::records::unknown_targets(&corpus, &probe) {
        if proposed_ids.contains(r.target.id.as_str()) {
            continue;
        }
        faults.push(fault(
            "record.relation-target-unknown",
            &r.source,
            r.line,
            format!(
                "{}{} {} {}: {} is no record of this corpus and none the proposal makes",
                if r.line > 0 {
                    format!("line {}: ", r.line)
                } else {
                    String::new()
                },
                r.from,
                r.kind,
                r.target.id,
                r.target.id
            ),
        ));
    }
    if !faults.is_empty() {
        return Ok(Err(faults));
    }

    let atom = record::parse(&text)
        .map_err(|e| RepoError::Message(format!("plan.records-render: the rendered atom: {e}")))?;
    let records = atom
        .records
        .iter()
        .map(|r| RecordSummary {
            id: r.id.clone(),
            record_type: r.record_type.clone(),
            revision: r.revision.clone(),
            relations: r
                .relations
                .iter()
                .map(|x| format!("{} {}", x.kind, x.target.id))
                .collect(),
        })
        .collect();
    let ticket_title = match openwarrant_core::ticket::one_line(&ticket.title) {
        t if t.is_empty() => title.clone(),
        t => t,
    };
    Ok(Ok(Plan {
        area,
        path,
        text,
        profile: request.profile.clone(),
        records,
        ticket_title,
        ticket_body: ticket.body.clone(),
        priority: ticket.priority,
        items,
    }))
}

/// Faults as a report, each an error by its rule.
#[must_use]
pub fn refusal_report(faults: &[Fault]) -> Report {
    let mut report = Report::default();
    for f in faults {
        report.push(Diagnostic::error(
            f.rule,
            match (f.file.is_empty(), f.line) {
                (true, _) => "proposal".to_owned(),
                (false, 0) => f.file.clone(),
                (false, l) => format!("{}:{l}", f.file),
            },
            f.message.clone(),
        ));
    }
    report
}

/// The `result` of a refusal: the rules, and the proposal as it came back,
/// so it can be read without a file having been written.
#[must_use]
pub fn refusal_result(faults: &[Fault], json: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "schema": REFUSED_SCHEMA,
        "rules": faults.iter().map(|f| f.rule).collect::<Vec<_>>(),
        "proposal": json.and_then(|j| serde_json::from_str::<serde_json::Value>(j.trim()).ok()),
    })
}

/// The item line `war create` writes: the text, then each `implements`.
fn item_line(item: &ItemPlan) -> String {
    if item.implements.is_empty() {
        item.text.clone()
    } else {
        format!(
            "{} ({})",
            item.text,
            item.implements
                .iter()
                .map(|t| format!("implements {t}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// Write the record atom, then the ticket. A ticket the store refuses takes
/// the atom (and an area directory made for it) back out: `Err(outcome)`.
pub fn apply(
    repo: &Repository,
    store: &crate::ticket::Store,
    plan: &Plan,
    drafter: Option<&str>,
    proposal_json: &str,
) -> Result<Result<Applied, crate::ticket::Outcome>, RepoError> {
    use std::io::Write as _;
    let file: Utf8PathBuf = repo.root.join(&plan.path);
    let area_dir = file
        .parent()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| repo.root.clone());
    let made_area = !area_dir.exists();
    std::fs::create_dir_all(&area_dir).map_err(|source| RepoError::Io {
        context: format!("could not create {area_dir}"),
        source,
    })?;
    let undo = || {
        let _ = std::fs::remove_file(&file);
        if made_area {
            let _ = std::fs::remove_dir(&area_dir);
        }
    };
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)
        .and_then(|mut f| f.write_all(plan.text.as_bytes()));
    if let Err(source) = written {
        if source.kind() != std::io::ErrorKind::AlreadyExists {
            undo();
        } else if made_area {
            let _ = std::fs::remove_dir(&area_dir);
        }
        return Err(RepoError::Io {
            context: format!("could not create {} (never overwritten)", plan.path),
            source,
        });
    }
    let digest = openwarrant_compiler::sha256_hex(proposal_json.trim().as_bytes());
    let mut body = plan
        .ticket_body
        .as_deref()
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .map(|b| format!("{b}\n\n"))
        .unwrap_or_default();
    body.push_str(&format!(
        "Records: {} ({}). Drafted{} as `{PROPOSAL_API}` sha256:{digest}.",
        plan.records
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        plan.path,
        drafter.map_or_else(String::new, |d| format!(" by {d}")),
    ));
    let args = crate::ticket::CreateArgs {
        title: plan.ticket_title.clone(),
        items: plan.items.iter().map(item_line).collect(),
        body: Some(body),
        priority: plan.priority,
        kind: None,
        labels: Vec::new(),
        part_of: None,
        issue: None,
    };
    let outcome = match crate::ticket::create(store, &args) {
        Ok(o) => o,
        Err(e) => {
            undo();
            return Err(e);
        }
    };
    if outcome.is_refused() {
        undo();
        return Ok(Err(outcome));
    }
    let made = outcome.result["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let items = plan
        .items
        .iter()
        .zip(made.iter())
        .map(|(p, m)| ItemPlan {
            id: m["id"].as_str().map(str::to_owned),
            text: p.text.clone(),
            implements: p.implements.clone(),
        })
        .collect();
    Ok(Ok(Applied {
        schema: APPLIED_SCHEMA,
        area: plan.area.clone(),
        file: plan.path.clone(),
        profile: plan.profile.clone(),
        records: plan.records.clone(),
        ticket: outcome.result["id"].as_str().unwrap_or_default().to_owned(),
        ticket_title: plan.ticket_title.clone(),
        items,
    }))
}

/// The human rendering of an application.
#[must_use]
pub fn human(a: &Applied) -> String {
    let mut s = format!("{}  {}\n", a.ticket, a.ticket_title);
    s.push_str(&format!("records {} ({}):\n", a.file, a.profile));
    for r in &a.records {
        s.push_str(&format!("  {} · {}", r.id, r.record_type));
        if !r.relations.is_empty() {
            s.push_str(&format!("  {}", r.relations.join(", ")));
        }
        s.push('\n');
    }
    s.push_str("items:\n");
    for i in &a.items {
        s.push_str(&format!(
            "  {}  {}{}\n",
            i.id.as_deref().unwrap_or("-"),
            i.text,
            if i.implements.is_empty() {
                String::new()
            } else {
                format!("  (implements {})", i.implements.join(", "))
            }
        ));
    }
    let first = a.records.first().map_or("<ID>", |r| r.id.as_str());
    s.push_str(&format!(
        "`war ready` lists the items; `war impact {first}` lists what a change to a record reaches"
    ));
    s
}

/// The application as a report line, for `war plan`'s envelope.
#[must_use]
pub fn applied_report(a: &Applied) -> Report {
    let mut report = Report::default();
    report.push(Diagnostic::pass(
        "plan.records-applied",
        format!(
            "{} record(s) in {} ({}), and ticket {} with {} item(s) implementing them",
            a.records.len(),
            a.file,
            a.records
                .iter()
                .map(|r| format!("{} · {}", r.id, r.record_type))
                .collect::<Vec<_>>()
                .join(", "),
            a.ticket,
            a.items.len()
        ),
    ));
    report
}

// ---------------------------------------------------------------------------
// The two commands.
// ---------------------------------------------------------------------------

/// `war plan --records`'s arguments.
#[derive(Debug, Clone, Default)]
pub struct PlanArgs {
    pub sentence: String,
    pub profile: String,
    pub area: Option<String>,
    pub proposal: Option<Utf8PathBuf>,
    pub draft: bool,
    pub reviewed: bool,
    pub apply: bool,
    pub out: Option<Utf8PathBuf>,
    pub answers: BTreeMap<String, String>,
}

/// Where `--draft` keeps a proposal for review when `--out` is not given:
/// under `.openwarrant/state/`, which is disposable and never tracked.
fn scratch(repo: &Repository) -> Utf8PathBuf {
    repo.root
        .join(".openwarrant/state/plan")
        .join(format!("records-proposal-{}.json", std::process::id()))
}

fn shell_word(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./@=".contains(c))
    {
        s.to_owned()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Print a report (human: its diagnostics, or `human` when given and
/// nothing was refused; JSON: the envelope) and return the exit code.
fn answer(
    mode: crate::output::Mode,
    command: &str,
    report: &Report,
    human: Option<&str>,
    result: Option<serde_json::Value>,
) -> u8 {
    match (mode, human) {
        (crate::output::Mode::Human, Some(h)) if report.is_ready() => {
            println!("{h}");
            crate::output::exit_code(report)
        }
        _ => crate::output::finish(mode, command, report, result),
    }
}

/// `war plan "<sentence>" --records [--area] [--profile] [--draft |
/// --proposal <file>] [--reviewed --apply]`.
pub fn run_plan(
    mode: crate::output::Mode,
    repo: &Repository,
    args: &PlanArgs,
) -> Result<u8, RepoError> {
    if args.proposal.is_none() && args.sentence.trim().is_empty() {
        return Err(RepoError::Message(
            "war plan --records needs a sentence, or --proposal <file>".to_owned(),
        ));
    }
    let req = match request(
        repo,
        &args.sentence,
        &args.profile,
        args.area.as_deref(),
        &args.answers,
    )? {
        Ok(r) => r,
        Err(d) => {
            let mut report = Report::default();
            report.push(d);
            return Ok(answer(mode, "plan.records", &report, None, None));
        }
    };
    if args.apply && !args.reviewed {
        let mut report = Report::default();
        report.push(Diagnostic::error(
            "plan.review-required",
            args.proposal
                .as_ref()
                .map_or_else(|| "--apply".to_owned(), ToString::to_string),
            "--apply writes records and a ticket, and a person reads the proposal first (§74.4 \
             step 6): validate it without --apply, read it, then run again with --reviewed \
             --apply",
        ));
        return Ok(answer(mode, "plan.records", &report, None, None));
    }
    let (json, drafter) = match (&args.proposal, args.draft) {
        (Some(path), _) => (
            std::fs::read_to_string(path)
                .map_err(|e| RepoError::Message(format!("cannot read {path}: {e}")))?,
            None,
        ),
        (None, true) => {
            if repo.config.plan.drafter_argv.is_empty() {
                let mut report = Report::default();
                report.push(Diagnostic::error(
                    "plan.no-drafter",
                    crate::init::CONFIG_FILE,
                    "`--draft` asks the configured drafter, and `[plan] drafter_argv` is not \
                     set; nothing was invented. Hand the request (`war plan --records` without \
                     --draft) to an agent and return its proposal with --proposal <file>",
                ));
                return Ok(answer(mode, "plan.records", &report, None, None));
            }
            let (json, run) = crate::plan::run_drafter(repo, &req)?;
            (json, Some(run.name))
        }
        (None, false) => {
            let human = serde_json::to_string_pretty(&req).expect("request serializes");
            crate::output::emit(
                mode,
                "plan.records-request",
                &human,
                crate::output::value(&req),
            );
            if repo.config.plan.drafter_argv.is_empty() {
                eprintln!(
                    "\nNo drafter is configured. Hand this request to an agent; it answers with \
                     an {PROPOSAL_API}, returned with `war plan --records --proposal <file>`. Or \
                     set [plan] drafter_argv and use --draft."
                );
            }
            return Ok(crate::EXIT_OK);
        }
    };
    let plan = match validate(repo, &req, &json, &args.sentence)? {
        Ok(p) => p,
        Err(faults) => {
            return Ok(answer(
                mode,
                "plan.records",
                &refusal_report(&faults),
                None,
                Some(refusal_result(&faults, Some(&json))),
            ));
        }
    };
    if !args.apply {
        let mut report = Report::default();
        let mut cmd = String::from("war plan --records");
        if args.profile != "delivery" {
            cmd.push_str(&format!(" --profile {}", shell_word(&args.profile)));
        }
        cmd.push_str(&format!(" --area {}", shell_word(&plan.area)));
        let kept = if let Some(path) = &args.proposal {
            Some(path.clone())
        } else {
            let path = args.out.clone().unwrap_or_else(|| scratch(repo));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| RepoError::Message(format!("cannot create {parent}: {e}")))?;
            }
            std::fs::write(&path, &json)
                .map_err(|e| RepoError::Message(format!("cannot write {path}: {e}")))?;
            Some(path)
        };
        if let Some(path) = &kept {
            cmd.push_str(&format!(
                " --proposal {} --reviewed --apply",
                shell_word(path.as_str())
            ));
        }
        report.push(Diagnostic::pass(
            "plan.records-applicable",
            format!(
                "{} record(s) for {} ({}) and a ticket of {} item(s); nothing written. Read it, \
                 then: {cmd}",
                plan.records.len(),
                plan.path,
                plan.records
                    .iter()
                    .map(|r| format!("{} · {}", r.id, r.record_type))
                    .collect::<Vec<_>>()
                    .join(", "),
                plan.items.len()
            ),
        ));
        let result = serde_json::json!({
            "schema": "oh.war/records-plan/v1",
            "area": plan.area,
            "file": plan.path,
            "profile": plan.profile,
            "records": plan.records,
            "ticket_title": plan.ticket_title,
            "items": plan.items,
            "proposal": kept.as_ref().map(|p| repo.relative(p)),
            "apply": cmd,
        });
        let mut human = format!(
            "{}\n\n{} \u{2014} the ticket, {} item(s):\n",
            plan.text.trim_end(),
            plan.ticket_title,
            plan.items.len()
        );
        for i in &plan.items {
            human.push_str(&format!("- [ ] {}\n", item_line(i)));
        }
        human.push_str(&format!(
            "\nValid: {} record(s) and {} item(s), checked as authored records are; nothing \
             was written. Read it, then:\n  {cmd}",
            plan.records.len(),
            plan.items.len()
        ));
        return Ok(answer(
            mode,
            "plan.records",
            &report,
            Some(human.trim_end()),
            Some(result),
        ));
    }
    let store = crate::ticket::Store::open(repo, None)?;
    match apply(repo, &store, &plan, drafter.as_deref(), &json)? {
        Ok(applied) => Ok(answer(
            mode,
            "plan.records-apply",
            &applied_report(&applied),
            Some(&human(&applied)),
            Some(crate::output::value(&applied)),
        )),
        Err(outcome) => Ok(answer(
            mode,
            "plan.records-apply",
            &outcome.report,
            None,
            Some(outcome.result),
        )),
    }
}

/// `war create "<sentence>" --draft --records [--area]`: the ticket and the
/// records it implements, from one sentence. The caller's `--item`s join
/// the drafted ones, implementing nothing; `--body` leads the ticket's body;
/// `--priority` wins.
pub fn run_create(
    repo: &Repository,
    store: &crate::ticket::Store,
    sentence: &str,
    area: Option<&str>,
    extra_items: &[String],
    body: Option<&str>,
    priority: Option<u8>,
) -> Result<crate::ticket::Outcome, RepoError> {
    let refused = |report: Report, result: serde_json::Value| crate::ticket::Outcome {
        human: report
            .diagnostics
            .first()
            .map(|d| d.message.clone())
            .unwrap_or_default(),
        report,
        result,
    };
    if repo.config.plan.drafter_argv.is_empty() {
        let mut report = Report::default();
        report.push(Diagnostic::error(
            "ticket.no-drafter",
            crate::init::CONFIG_FILE,
            "`--draft` asks the configured drafter, and `[plan] drafter_argv` is not set; \
             nothing was invented. Give the items with `--item`, or add them later with \
             `war add`",
        ));
        return Ok(refused(report, serde_json::Value::Null));
    }
    let req = match request(repo, sentence, "delivery", area, &BTreeMap::new())? {
        Ok(r) => r,
        Err(d) => {
            let mut report = Report::default();
            report.push(d);
            return Ok(refused(report, serde_json::Value::Null));
        }
    };
    let (json, run) = crate::plan::run_drafter(repo, &req)?;
    let mut plan = match validate(repo, &req, &json, sentence)? {
        Ok(p) => p,
        Err(faults) => {
            return Ok(refused(
                refusal_report(&faults),
                refusal_result(&faults, Some(&json)),
            ));
        }
    };
    for text in extra_items {
        let text = openwarrant_core::ticket::one_line(text);
        if !text.is_empty() {
            plan.items.push(ItemPlan {
                id: None,
                text,
                implements: Vec::new(),
            });
        }
    }
    if let Some(lead) = body.map(str::trim).filter(|b| !b.is_empty()) {
        plan.ticket_body = Some(match plan.ticket_body.as_deref().map(str::trim) {
            Some(b) if !b.is_empty() => format!("{lead}\n\n{b}"),
            _ => lead.to_owned(),
        });
    }
    if priority.is_some() {
        plan.priority = priority;
    }
    match apply(repo, store, &plan, Some(&run.name), &json)? {
        Ok(applied) => Ok(crate::ticket::Outcome {
            report: Report::default(),
            human: human(&applied),
            result: crate::output::value(&applied),
        }),
        Err(outcome) => Ok(outcome),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_renders_as_the_atom_format_reads_it() {
        let r = ProposedRecord {
            id: "REQ-x1".to_owned(),
            record_type: "requirement".to_owned(),
            title: "A link   is valid once.".to_owned(),
            body: "\nMore.\n".to_owned(),
            relations: vec![ProposedRelation {
                kind: "implements".to_owned(),
                target: "OUT-x1".to_owned(),
            }],
        };
        let text = format!("{}{}", frontmatter("delivery"), render_record(&r));
        assert_eq!(
            text,
            "---\nschema: oh.war/records/v1\nprofile: delivery\n---\n## REQ-x1 · requirement\nimplements OUT-x1\n\nA link is valid once.\n\nMore.\n"
        );
        let atom = record::parse(&text).unwrap();
        assert_eq!(atom.records.len(), 1);
        assert_eq!(atom.records[0].relations[0].target.id, "OUT-x1");
    }

    #[test]
    fn a_proposal_with_a_field_it_does_not_know_is_refused_at_parse() {
        let bad =
            r#"{"api_version":"oh.war/records-proposal/v1","records":[],"authorized_by":"x"}"#;
        assert!(serde_json::from_str::<Proposal>(bad).is_err());
        let bad = r#"{"api_version":"oh.war/records-proposal/v1","records":[{"id":"A-1","type":"outcome","title":"t","revision":"x"}]}"#;
        assert!(serde_json::from_str::<Proposal>(bad).is_err());
    }

    #[test]
    fn areas_and_slugs() {
        assert_eq!(
            slug("Add password reset, by e-mail!", 4),
            "add-password-reset-by"
        );
        assert!(valid_area("password-reset"));
        assert!(!valid_area("Password"));
        assert!(!valid_area("../x"));
        assert!(!valid_area(""));
    }
}
