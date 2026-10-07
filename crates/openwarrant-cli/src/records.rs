// SPDX-License-Identifier: Apache-2.0
//! A program's record atoms and the relations its documents author
//! (OW-WAR-0148 M3; OW-ADR-0031). `docs/TYPES.md` is the guide.
//!
//! # Where record atoms live
//!
//! `docs/records/<area>/<NN>-<name>.md`: one directory per program area or
//! feature (`docs/records/password-reset/`), each file an
//! `oh.war/records/v1` atom ([`openwarrant_core::record`]) whose frontmatter
//! names the profile that governs its records. Every `*.md` inside an area
//! directory is a record atom; a file directly in `docs/records/` (a
//! README) is not read. A record is bound by its id, never by its file:
//! Warrants, tickets and the roadmap name `REQ-pr1`, and moving it to
//! another file or area changes nothing that names it.
//!
//! # What is read, and refused
//!
//! - **Records.** Each `## <ID> · <type>` of each atom, with its revision
//!   (sha256 of its exact byte span). Refused, each by rule: an atom that
//!   does not parse (`record.malformed`), a profile the registry does not
//!   admit (`record.profile-unknown`), a type the governing profile does not
//!   declare (`record.type-undeclared`; the kernel knows only `obligation`
//!   and `item`, which live in their documents), and an id already declared
//!   (`record.duplicate-id`; the first declaration stands).
//! - **Relations** of three origins: a record atom's relation lines; an
//!   obligation's `- **evaluates:**` bullet in a Warrant's assurance atom,
//!   governed by the Warrant's profile; and `implements <ID>` in a ticket
//!   item's text, governed by the ticket profile, read only where it allows
//!   `implements` (item text is prose otherwise). Refused: a kind that is
//!   neither core nor namespaced (`record.relation-kind-unknown`), a core
//!   kind the governing profile does not allow (`record.relation-undeclared`),
//!   an evaluates target that does not parse (`record.relation-malformed`),
//!   and a record missing a relation its profile requires
//!   (`record.relation-required`). A namespaced kind is carried and inert.
//! - **Targets.** A relation whose target is no record of the corpus is a
//!   warning (`record.relation-target-unknown`), kept, never dropped.
//! - **Instruction sections** (M16, [`crate::instructions`]): each `##`
//!   section of the root `CLAUDE.md` and `AGENTS.md` (and nested files the
//!   config names) is a record `md:<file>#<slug>` of type `instruction`, so a
//!   relation can cite one (`constrains md:CLAUDE.md#testing`). They are read
//!   apart from record atoms: no profile governs them, they count in no
//!   `records.well-formed` total, and a malformed block in one is a warning
//!   (`instruction.block-malformed`), never a refusal of the file.
//!
//! Nothing here is written.

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};

use openwarrant_core::record::{self, RecordAtom};
use openwarrant_core::relation::{CoreKind, RelationKind, Target, Vocabulary};

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::Repository;

/// Where record atoms live, relative to the repository root.
pub const DIR: &str = "docs/records";

/// One record read from a record atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub id: String,
    pub record_type: String,
    /// The profile that governs it.
    pub profile: String,
    /// Repository-relative path of its atom.
    pub source: String,
    /// 1-based line of its heading.
    pub line: usize,
    /// `sha256:<hex>` of its byte span.
    pub revision: String,
}

/// One authored relation, of any origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    /// A record id, `<alias>/<OBL-…>` or `<ticket>/<item>`.
    pub from: String,
    pub kind: RelationKind,
    pub target: Target,
    /// Repository-relative file and 1-based line it was authored at.
    pub source: String,
    pub line: usize,
}

/// One refusal or warning, by rule, anchored to `file:line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    pub rule: &'static str,
    pub file: String,
    pub line: usize,
    pub message: String,
}

/// Every record atom's records and every authored relation of the corpus.
#[derive(Debug, Clone, Default)]
pub struct Records {
    /// In file order, then heading order. Refused records are absent.
    pub records: Vec<Record>,
    /// Admitted relations; refused ones are absent and have a fault.
    pub relations: Vec<Relation>,
    pub faults: Vec<Fault>,
    /// Record atoms read.
    pub files: usize,
    /// Instruction-file sections (M16), in file order, then heading order.
    pub instructions: Vec<Record>,
    /// What could not be read in an instruction file: warnings, never errors.
    pub instruction_faults: Vec<Fault>,
}

impl Records {
    /// The record named `id`, if a record atom declares one.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Record> {
        self.records.iter().find(|r| r.id == id)
    }

    /// The instruction section named `id` (`md:CLAUDE.md#testing`), if one
    /// exists.
    #[must_use]
    pub fn instruction(&self, id: &str) -> Option<&Record> {
        self.instructions.iter().find(|r| r.id == id)
    }

    /// Whether there is anything to report: a record atom, or a relation
    /// authored elsewhere, or a fault. Instruction sections alone are not:
    /// every repository with a `CLAUDE.md` has them, and its check stays
    /// what it was until something cites one.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files == 0 && self.relations.is_empty() && self.faults.is_empty()
    }
}

/// The record atoms of `repo`, sorted: `docs/records/<area>/*.md`.
#[must_use]
pub fn files(repo: &Repository) -> Vec<Utf8PathBuf> {
    let read = |d: &Utf8Path| -> Vec<Utf8PathBuf> {
        let Ok(rd) = crate::vfs::read_dir(d) else {
            return Vec::new();
        };
        rd.filter_map(Result::ok)
            .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
            .collect()
    };
    let mut out: Vec<Utf8PathBuf> = read(&repo.root.join(DIR))
        .into_iter()
        .filter(|p| crate::vfs::is_dir(p))
        .flat_map(|area| read(&area))
        .filter(|p| crate::vfs::is_file(p) && p.extension() == Some("md"))
        .collect();
    out.sort();
    out
}

pub(crate) fn kind_fault(
    vocabulary: &Vocabulary,
    profile: &str,
    kind: &str,
    file: &str,
    line: usize,
) -> Result<RelationKind, Fault> {
    let parsed = RelationKind::parse(kind).map_err(|e| Fault {
        rule: "record.relation-kind-unknown",
        file: file.to_owned(),
        line,
        message: format!("line {line}: {e}"),
    })?;
    if vocabulary.allows(&parsed) {
        Ok(parsed)
    } else {
        Err(Fault {
            rule: "record.relation-undeclared",
            file: file.to_owned(),
            line,
            message: format!(
                "line {line}: `{kind}` is a core relation kind profile {profile} does not \
                 allow ([relations] allow = [{}])",
                vocabulary
                    .allow
                    .iter()
                    .map(|k| format!("\"{k}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
    }
}

/// Read every record atom, every obligation's `evaluates` and every ticket
/// item's `implements`, and refuse what the governing profiles do not
/// declare.
#[must_use]
pub fn load(corpus: &Corpus) -> Records {
    load_with(corpus, &[])
}

/// [`load`], as if each of `proposed` — `(repository-relative path, text)` —
/// were a record atom read after every file on disk: the same rules over the
/// same corpus, for a proposal that is not written yet (OW-WAR-0148 M7,
/// `war plan --records`). Read last, a proposed record that repeats an id
/// already on disk is the one refused as `record.duplicate-id`.
#[must_use]
pub fn load_with(corpus: &Corpus, proposed: &[(String, String)]) -> Records {
    let repo = corpus.repo();
    let mut out = Records::default();
    let mut first: BTreeMap<String, String> = BTreeMap::new();
    let empty = Vocabulary::default();

    // ---- Record atoms.
    let mut atoms: Vec<(String, RecordAtom)> = Vec::new();
    let on_disk = files(repo).into_iter().map(|path| {
        let rel = repo.relative(&path);
        (rel, crate::vfs::read(&path).map(String::from_utf8))
    });
    let virtual_atoms = proposed
        .iter()
        .map(|(rel, text)| (rel.clone(), Ok(Ok(text.clone()))));
    for (rel, read) in on_disk.chain(virtual_atoms) {
        out.files += 1;
        let text = match read {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => {
                out.faults.push(Fault {
                    rule: "record.malformed",
                    file: rel,
                    line: 0,
                    message: format!("not UTF-8: {e}"),
                });
                continue;
            }
            Err(e) => {
                out.faults.push(Fault {
                    rule: "record.malformed",
                    file: rel,
                    line: 0,
                    message: format!("could not read: {e}"),
                });
                continue;
            }
        };
        match record::parse(&text) {
            Ok(atom) => atoms.push((rel, atom)),
            Err(e) => out.faults.push(Fault {
                rule: "record.malformed",
                file: rel,
                line: e.line(),
                message: e.to_string(),
            }),
        }
    }
    for (rel, atom) in &atoms {
        let Some(vocabulary) = repo.profiles.vocabulary(&atom.profile) else {
            out.faults.push(Fault {
                rule: "record.profile-unknown",
                file: rel.clone(),
                line: 1,
                message: format!(
                    "line 1: profile {:?} governs these records and is not a profile of this \
                     program (known: {})",
                    atom.profile,
                    repo.profiles.names().join(", ")
                ),
            });
            continue;
        };
        for r in &atom.records {
            if !vocabulary.has_type(&r.record_type) {
                let kernel = openwarrant_core::relation::KERNEL_RECORD_TYPES
                    .contains(&r.record_type.as_str());
                out.faults.push(Fault {
                    rule: "record.type-undeclared",
                    file: rel.clone(),
                    line: r.line,
                    message: if kernel {
                        format!(
                            "line {}: {} · {}: `{}` is a kernel type, which lives in its \
                             document (an obligation in a Warrant's assurance atom, an item in \
                             a ticket's checklist), never in a record atom",
                            r.line, r.id, r.record_type, r.record_type
                        )
                    } else {
                        format!(
                            "line {}: {} · {}: profile {} declares no record type `{}` \
                             ([records] types = [{}])",
                            r.line,
                            r.id,
                            r.record_type,
                            atom.profile,
                            r.record_type,
                            vocabulary
                                .types
                                .iter()
                                .map(|t| format!("\"{t}\""))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    },
                });
                continue;
            }
            if let Some(at) = first.get(&r.id) {
                out.faults.push(Fault {
                    rule: "record.duplicate-id",
                    file: rel.clone(),
                    line: r.line,
                    message: format!(
                        "line {}: {} is already declared at {at}; a record id is unique across \
                         the program, and the first declaration stands",
                        r.line, r.id
                    ),
                });
                continue;
            }
            first.insert(r.id.clone(), format!("{rel}:{}", r.line));
            out.records.push(Record {
                id: r.id.clone(),
                record_type: r.record_type.clone(),
                profile: atom.profile.clone(),
                source: rel.clone(),
                line: r.line,
                revision: r.revision.clone(),
            });
            for rel_line in &r.relations {
                match kind_fault(
                    vocabulary,
                    &atom.profile,
                    &rel_line.kind,
                    rel,
                    rel_line.line,
                ) {
                    Ok(kind) => out.relations.push(Relation {
                        from: r.id.clone(),
                        kind,
                        target: rel_line.target.clone(),
                        source: rel.clone(),
                        line: rel_line.line,
                    }),
                    Err(f) => out.faults.push(f),
                }
            }
        }
    }

    // ---- Instruction files' sections (M16): records no profile governs.
    let (instructions, faults, _) = crate::instructions::read(repo);
    out.instructions = instructions;
    out.instruction_faults = faults;

    // ---- Obligations' `evaluates`, governed by their Warrant's profile.
    if let Ok(entries) = corpus.entries() {
        for e in entries {
            let Some(one) = e.ok() else { continue };
            let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
                continue;
            };
            let alias = validated.alias.to_string();
            let profile = validated.profile.as_str().to_owned();
            let vocabulary = repo.profiles.vocabulary(&profile).unwrap_or(&empty);
            for atom in basis.atoms.iter().filter(|a| a.role == "assurance") {
                let rel = repo.relative(&one.dir.join(&atom.source));
                match record::obligation_evaluates(&String::from_utf8_lossy(&atom.bytes)) {
                    Ok(found) => {
                        for (obl, target, line) in found {
                            match kind_fault(vocabulary, &profile, "evaluates", &rel, line) {
                                Ok(kind) => out.relations.push(Relation {
                                    from: format!("{alias}/{obl}"),
                                    kind,
                                    target,
                                    source: rel.clone(),
                                    line,
                                }),
                                Err(f) => out.faults.push(f),
                            }
                        }
                    }
                    Err((line, token)) => out.faults.push(Fault {
                        rule: "record.relation-malformed",
                        file: rel,
                        line,
                        message: format!(
                            "line {line}: `evaluates` names {token:?}, which is no record id; \
                             a target is `REQ-pr1`, `OW-WAR-0001/OBL-001` or `t-3f2a/i-9c01`, \
                             optionally `@sha256:<64 hex>`"
                        ),
                    }),
                }
            }
        }
    }

    // ---- Ticket items' `implements`, where the ticket profile allows it.
    if let (Ok((tickets, _)), Ok(store)) =
        (corpus.tickets(), crate::ticket::Store::open(repo, None))
        && store
            .definition
            .vocabulary
            .allows(&RelationKind::Core(CoreKind::Implements))
    {
        for t in tickets {
            let rel = repo.relative(&t.checklist_path);
            for item in &t.checklist.items {
                let Some(iid) = &item.id else { continue };
                for target in openwarrant_core::relation::implements_phrases(&item.text) {
                    out.relations.push(Relation {
                        from: format!("{}/{iid}", t.id()),
                        kind: RelationKind::Core(CoreKind::Implements),
                        target,
                        source: rel.clone(),
                        line: item.line + 1,
                    });
                }
            }
        }
    }

    // ---- Required relations, over admitted records.
    let types: BTreeMap<&str, &str> = out
        .records
        .iter()
        .map(|r| (r.id.as_str(), r.record_type.as_str()))
        .collect();
    let type_of = |id: &str| -> Option<&str> {
        types.get(id).copied().or_else(|| {
            let (_, tail) = id.split_once('/')?;
            if tail.starts_with("OBL-") {
                Some("obligation")
            } else if tail.starts_with("i-") {
                Some("item")
            } else {
                None
            }
        })
    };
    let mut missing = Vec::new();
    for r in &out.records {
        let Some(vocabulary) = repo.profiles.vocabulary(&r.profile) else {
            continue;
        };
        for req in vocabulary
            .require
            .iter()
            .filter(|q| q.from == r.record_type)
        {
            let met = out.relations.iter().any(|x| {
                x.from == r.id
                    && x.kind == RelationKind::Core(req.kind)
                    && type_of(&x.target.id) == Some(req.to.as_str())
            });
            if !met {
                missing.push(Fault {
                    rule: "record.relation-required",
                    file: r.source.clone(),
                    line: r.line,
                    message: format!(
                        "line {}: {} · {}: profile {} requires every {} to `{}` a record of \
                         type `{}`, and this one names none",
                        r.line, r.id, r.record_type, r.profile, req.from, req.kind, req.to
                    ),
                });
            }
        }
    }
    out.faults.extend(missing);
    out
}

/// The relations whose target is no record of the corpus. Record-atom ids
/// first; only when a target is not among them is the whole model built, so
/// a program whose records name only records pays nothing more.
#[must_use]
pub fn unknown_targets<'a>(corpus: &Corpus, records: &'a Records) -> Vec<&'a Relation> {
    let local: BTreeSet<&str> = records
        .records
        .iter()
        .chain(&records.instructions)
        .map(|r| r.id.as_str())
        .collect();
    let rest: Vec<&Relation> = records
        .relations
        .iter()
        .filter(|r| !local.contains(r.target.id.as_str()))
        .collect();
    if rest.is_empty() {
        return rest;
    }
    let Ok(model) = crate::model::build(corpus) else {
        return rest;
    };
    let ids: BTreeSet<&str> = model.records.iter().map(|r| r.id.as_str()).collect();
    rest.into_iter()
        .filter(|r| !ids.contains(r.target.id.as_str()))
        .collect()
}

/// `war check`'s record rules, into `report`: each fault as an error, each
/// unknown target as a warning, and one pass naming what was read when
/// nothing was refused. Silent for a program with no record atom and no
/// authored relation, so its check is what it was.
pub fn check(corpus: &Corpus, report: &mut Report) {
    let records = corpus.records();
    // An instruction file's malformed block is the file owner's to fix and
    // blocks nothing: a warning, said even where nothing else is read.
    for f in &records.instruction_faults {
        report.push(Diagnostic::warn(
            f.rule,
            if f.line > 0 {
                format!("{}:{}", f.file, f.line)
            } else {
                f.file.clone()
            },
            f.message.clone(),
        ));
    }
    if records.is_empty() {
        return;
    }
    for f in &records.faults {
        report.push(Diagnostic::error(
            f.rule,
            if f.line > 0 {
                format!("{}:{}", f.file, f.line)
            } else {
                f.file.clone()
            },
            f.message.clone(),
        ));
    }
    // A namespaced relation drives no check: its target is the model's to
    // report, not this one's.
    for r in unknown_targets(corpus, records)
        .into_iter()
        .filter(|r| !r.kind.is_inert())
    {
        report.push(Diagnostic::warn(
            "record.relation-target-unknown",
            format!("{}:{}", r.source, r.line),
            format!(
                "line {}: {} {} {}: {} is not a record of this corpus; the relation is kept, \
                 not dropped{}",
                r.line,
                r.from,
                r.kind,
                r.target.id,
                r.target.id,
                if r.target
                    .id
                    .starts_with(openwarrant_core::instruction::ID_PREFIX)
                {
                    ". A section's id is `md:<file>#<slug>`, its slug the heading as GitHub \
                     anchors it; `war plan model --json` lists every section"
                } else {
                    ""
                }
            ),
        ));
    }
    if records.faults.is_empty() {
        let core = records
            .relations
            .iter()
            .filter(|r| !r.kind.is_inert())
            .count();
        report.push(Diagnostic::pass(
            "records.well-formed",
            format!(
                "{} record(s) in {} record atom(s), {core} core relation(s): every type and \
                 core kind declared by its profile",
                records.records.len(),
                records.files,
            ),
        ));
    }
}
