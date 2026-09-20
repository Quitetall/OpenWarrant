// SPDX-License-Identifier: Apache-2.0
//! Explicit authority proposals and signed activation. Never trusts working-tree roles.
mod signing;
mod store;
use crate::{
    diagnostic::{Diagnostic, Report},
    repo::RepoError,
};
use openwarrant_core::authority_transition::{self as sdk, Operation, Proposal, Revision};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
const LIMIT: usize = 8 * 1024 * 1024;
type Result<T> = std::result::Result<T, RepoError>;
fn err(e: impl std::fmt::Display) -> RepoError {
    RepoError::Message(e.to_string())
}
#[derive(clap::Subcommand)]
pub enum Command {
    /// Draft canonical authority bytes. With --current, replace or remove one principal.
    Draft {
        #[arg(long)]
        current: Option<PathBuf>,
        #[arg(long)]
        repository: Option<String>,
        #[arg(long)]
        principal: String,
        #[arg(long)]
        public_key: Option<PathBuf>,
        #[arg(long)]
        role: Vec<String>,
        #[arg(long)]
        remove: bool,
        #[arg(long)]
        emit: PathBuf,
    },
    /// Draft an inert proposal and exact permission diff. No authority is changed.
    Propose {
        #[arg(long)]
        current: PathBuf,
        #[arg(long)]
        next: PathBuf,
        #[arg(long, default_value = "update")]
        operation: String,
        #[arg(long)]
        emit: PathBuf,
    },
    /// Show and sign an exact proposal; optionally activate it in one operator command.
    Approve {
        #[arg(long, conflicts_with = "store", required_unless_present = "store")]
        current: Option<PathBuf>,
        #[arg(long)]
        store: Option<PathBuf>,
        #[arg(long)]
        proposal: PathBuf,
        #[arg(long)]
        principal: String,
        #[arg(long)]
        key: PathBuf,
        #[arg(long, required_unless_present = "activate")]
        emit: Option<PathBuf>,
        #[arg(long, requires = "store")]
        activate: bool,
        #[arg(long)]
        unprotected_test_store: bool,
    },
    /// Check a proposal against explicit current state and cryptographic signatures.
    Check {
        #[arg(long)]
        current: PathBuf,
        #[arg(long)]
        proposal: PathBuf,
        #[arg(long)]
        signature: Vec<String>,
    },
    /// Establish trust explicitly in an empty, operator-owned authority directory.
    Bootstrap {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        revision: PathBuf,
        #[arg(long)]
        expected_digest: String,
        #[arg(long)]
        agent_uid: Option<u32>,
        #[arg(long)]
        legacy_dir: Option<PathBuf>,
        #[arg(long)]
        unprotected_test_store: bool,
    },
    /// Atomically activate an authenticated transition under previous trusted authority.
    Activate {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        proposal: PathBuf,
        #[arg(long)]
        signature: Vec<String>,
        #[arg(long)]
        unprotected_test_store: bool,
    },
    /// Read and validate retained authority history. Optional export is the current revision.
    Status {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        emit: Option<PathBuf>,
        #[arg(long)]
        unprotected_test_store: bool,
    },
    /// Export retained bootstrap, migration bytes and signed history for audit.
    History {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        emit: PathBuf,
        #[arg(long)]
        unprotected_test_store: bool,
    },
    /// Query an exact current grant. Caller must separately authenticate the acting principal.
    Allows {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        principal: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        expected_head: String,
        #[arg(long)]
        unprotected_test_store: bool,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Signed {
    proposal: Proposal,
    signatures: BTreeMap<String, String>,
}
fn read(path: &Path) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path).map_err(err)?;
    if !meta.is_file() || meta.len() > LIMIT as u64 {
        return Err(err("authority-input: bounded regular file required"));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(
            (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
        );
    }
    let file = options.open(path).map_err(err)?;
    if !file.metadata().map_err(err)?.is_file() {
        return Err(err("authority-input: regular file required"));
    }
    let mut bytes = Vec::new();
    file.take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() > LIMIT {
        return Err(err("authority-input-too-large"));
    }
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(err)?;
    file.write_all(bytes).map_err(err)?;
    file.sync_all().map_err(err)
}
fn revision(path: &Path) -> Result<Revision> {
    Revision::decode(&read(path)?).map_err(err)
}
fn proposal(path: &Path) -> Result<Proposal> {
    Proposal::decode(&read(path)?).map_err(err)
}
fn signed(proposal: Proposal, entries: Vec<String>) -> Result<Signed> {
    let mut signatures = BTreeMap::new();
    if entries.len() > 128 {
        return Err(err("authority-too-many-signatures"));
    }
    for entry in entries {
        let (principal, path) = entry
            .split_once('=')
            .ok_or_else(|| err("authority-signature: PRINCIPAL=PATH required"))?;
        let bytes = read(Path::new(path))?;
        if bytes.len() > 16384 {
            return Err(err("authority-signature-too-large"));
        }
        if signatures
            .insert(principal.into(), String::from_utf8(bytes).map_err(err)?)
            .is_some()
        {
            return Err(err("authority-duplicate-signature"));
        }
    }
    Ok(Signed {
        proposal,
        signatures,
    })
}
pub fn run(command: Command) -> Result<(Report, serde_json::Value)> {
    let value = match command {
        Command::Draft {
            current,
            repository,
            principal,
            public_key,
            role,
            remove,
            emit,
        } => {
            let mut revision = if let Some(path) = current {
                if repository.is_some() {
                    return Err(err("authority-repository-already-bound"));
                }
                let mut r = revision(&path)?;
                r.sequence = r
                    .sequence
                    .checked_add(1)
                    .ok_or_else(|| err("authority-sequence"))?;
                r
            } else {
                Revision {
                    schema: sdk::REVISION_SCHEMA.into(),
                    repository: repository.ok_or_else(|| err("authority-repository-required"))?,
                    sequence: 0,
                    principals: BTreeMap::new(),
                }
            };
            if remove {
                if public_key.is_some() || !role.is_empty() {
                    return Err(err("authority-remove-conflicting-input"));
                }
                if revision.principals.remove(&principal).is_none() {
                    return Err(err("authority-principal-absent"));
                }
            } else {
                let key = if let Some(path) = public_key {
                    String::from_utf8(read(&path)?)
                        .map_err(err)?
                        .split_whitespace()
                        .take(2)
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    revision
                        .principals
                        .get(&principal)
                        .ok_or_else(|| err("authority-key-required"))?
                        .public_key
                        .clone()
                };
                revision.principals.insert(
                    principal,
                    sdk::Principal {
                        public_key: key,
                        roles: role.into_iter().collect(),
                    },
                );
            }
            revision.validate().map_err(err)?;
            write_new(&emit, &revision.encode().map_err(err)?)?;
            serde_json::json!({"revision_digest":revision.digest().map_err(err)?,"revision":revision,"effective":false})
        }
        Command::Propose {
            current,
            next,
            operation,
            emit,
        } => {
            let current = revision(&current)?;
            let next = revision(&next)?;
            let operation = match operation.as_str() {
                "update" => Operation::Update,
                "recover" => Operation::Recover,
                _ => return Err(err("authority-operation")),
            };
            let p = Proposal {
                schema: sdk::PROPOSAL_SCHEMA.into(),
                operation,
                previous_digest: current.digest().map_err(err)?,
                next,
            };
            p.validate_against(&current).map_err(err)?;
            write_new(&emit, &p.encode().map_err(err)?)?;
            serde_json::json!({"previous_digest":p.previous_digest,"proposal_digest":p.digest().map_err(err)?,"before":current.principals,"after":p.next.principals,"effective":false})
        }
        Command::Approve {
            current,
            proposal: input,
            principal,
            key,
            emit,
            store: root,
            activate,
            unprotected_test_store,
        } => {
            let current = if let Some(root) = &root {
                serde_json::from_value(
                    store::status(root, unprotected_test_store)?["current"].clone(),
                )
                .map_err(err)?
            } else {
                revision(
                    current
                        .as_deref()
                        .ok_or_else(|| err("authority-current-required"))?,
                )?
            };
            let p = proposal(&input)?;
            p.validate_against(&current).map_err(err)?;
            eprintln!(
                "Authority proposal {}\nRepository: {}\nPrevious: {}\nBefore: {}\nAfter: {}",
                p.digest().map_err(err)?,
                p.next.repository,
                p.previous_digest,
                serde_json::to_string(&current.principals).map_err(err)?,
                serde_json::to_string(&p.next.principals).map_err(err)?
            );
            let sig = signing::sign(&current, &p, &principal, &key)?;
            if let Some(path) = &emit {
                write_new(path, sig.as_bytes())?;
            }
            if activate {
                store::activate(
                    root.as_deref()
                        .ok_or_else(|| err("authority-store-required"))?,
                    Signed {
                        proposal: p,
                        signatures: BTreeMap::from([(principal, sig)]),
                    },
                    unprotected_test_store,
                )?
            } else {
                serde_json::json!({"proposal_digest":p.digest().map_err(err)?,"principal":principal,"signature":emit,"effective":false,"human_review_established":false})
            }
        }
        Command::Check {
            current,
            proposal: input,
            signature,
        } => {
            let current = revision(&current)?;
            let record = signed(proposal(&input)?, signature)?;
            signing::verify(&current, &record)?;
            serde_json::json!({"eligible":true,"effective":false,"proposal_digest":record.proposal.digest().map_err(err)?,"trusted_state_source":"caller-supplied"})
        }
        Command::Bootstrap {
            store: root,
            revision: input,
            expected_digest,
            agent_uid,
            legacy_dir,
            unprotected_test_store,
        } => store::bootstrap(
            &root,
            revision(&input)?,
            &expected_digest,
            agent_uid,
            legacy_dir.as_deref(),
            unprotected_test_store,
        )?,
        Command::Activate {
            store: root,
            proposal: input,
            signature,
            unprotected_test_store,
        } => store::activate(
            &root,
            signed(proposal(&input)?, signature)?,
            unprotected_test_store,
        )?,
        Command::Status {
            store: root,
            emit,
            unprotected_test_store,
        } => {
            let state = store::status(&root, unprotected_test_store)?;
            if let Some(path) = emit {
                write_new(&path, &serde_jcs::to_vec(&state["current"]).map_err(err)?)?;
            }
            state
        }
        Command::History {
            store: root,
            emit,
            unprotected_test_store,
        } => store::history(&root, &emit, unprotected_test_store)?,
        Command::Allows {
            store: root,
            principal,
            role,
            expected_head,
            unprotected_test_store,
        } => {
            let state = store::status(&root, unprotected_test_store)?;
            if state["head"].as_str() != Some(&expected_head) {
                return Err(err("authority-stale-head"));
            }
            let allowed = state["current"]["principals"][&principal]["roles"]
                .as_array()
                .is_some_and(|roles| roles.iter().any(|r| r.as_str() == Some(&role)));
            if !allowed {
                return Err(err("authority-role-denied"));
            }
            serde_json::json!({"allowed":true,"principal":principal,"role":role,"head":expected_head,"isolation_enforced":state["isolation_enforced"],"actor_authenticated":false})
        }
    };
    let mut report = Report::default();
    report.push(Diagnostic::pass(
        "authority.checked",
        "Authority operation completed within its reported trust boundary",
    ));
    Ok((report, value))
}
