// SPDX-License-Identifier: Apache-2.0
//! Filesystem adapter for portable legacy dispatch transport. No model calls,
//! execution or signing; packaged references never become filesystem paths on read.
use crate::progress_viewer::source;
use crate::{
    diagnostic::{Diagnostic, Report, Severity},
    repo::{RepoError, Repository},
};
use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::{
    dispatch_bundle::{self as bundle, Bundle, Policy},
    lower,
};
use openwarrant_core::{context::ContextManifest, execution::StageDispatch};

use std::{collections::BTreeMap, io::Write};

#[derive(clap::Subcommand)]
pub enum Command {
    /// Capture exact local context for an already compiled Dispatch. Does not execute it.
    Create {
        alias: String,
        #[arg(long)]
        dispatch: Utf8PathBuf,
        #[arg(long)]
        context: Utf8PathBuf,
        #[arg(long)]
        emit: Utf8PathBuf,
        /// Explicit files for input ports, failure evidence or retained SAS: REF=PATH.
        #[arg(long)]
        attachment: Vec<String>,
        /// Explicit capability policy already bound by the input Dispatch.
        #[arg(long)]
        policy: Option<Utf8PathBuf>,
    },
    /// Check a portable bundle offline against its independently supplied digest.
    Check {
        file: Utf8PathBuf,
        #[arg(long)]
        expected_digest: String,
        /// Read one captured reference as UTF-8; no repository or network lookup.
        #[arg(long)]
        read: Option<String>,
        /// Refuse unless every requested capability is listed by the trusted policy.
        #[arg(long)]
        require_capability: Vec<String>,
        /// Caller-established policy identity, not authority inferred from the package.
        #[arg(long)]
        trusted_policy_digest: Option<String>,
    },
}
fn err(e: impl std::fmt::Display) -> RepoError {
    RepoError::Message(e.to_string())
}
fn read(path: &Utf8Path) -> Result<Vec<u8>, RepoError> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        Utf8PathBuf::from_path_buf(std::env::current_dir().map_err(err)?)
            .map_err(|_| err("non-UTF8 cwd"))?
            .join(path)
    };
    let parent = absolute
        .parent()
        .ok_or_else(|| err("input needs a parent"))?;
    let name = absolute
        .file_name()
        .ok_or_else(|| err("input needs a filename"))?;
    source::read(
        parent.as_std_path(),
        std::path::Path::new(name),
        bundle::MAX_BYTES,
    )
    .map_err(err)
}
fn capture(repo: &Repository, path: &str) -> Result<Vec<u8>, RepoError> {
    source::read(
        repo.root.as_std_path(),
        std::path::Path::new(path),
        bundle::MAX_BYTES,
    )
    .map_err(|e| err(format!("bundle-source-unreadable: {path}: {e}")))
}
fn insert(
    sources: &mut BTreeMap<String, Vec<u8>>,
    id: String,
    bytes: Vec<u8>,
) -> Result<(), RepoError> {
    if let Some(old) = sources.get(&id) {
        if old != &bytes {
            return Err(err(format!("bundle-source-collision: {id}")));
        }
    } else {
        let total = sources.values().map(Vec::len).sum::<usize>();
        if sources.len() >= bundle::MAX_SOURCES
            || bytes.len() > bundle::MAX_BYTES.saturating_sub(total)
        {
            return Err(err("bundle-resource-limit"));
        }
        sources.insert(id, bytes);
    }
    Ok(())
}
/// RQ-046 / §47.2: a Dispatch declares its estimate and budget, and one over
/// its budget is not handed on. `bundle-tokens-unrecorded` for a packet with
/// no `tokens` (hand-edited, or compiled before C2); `bundle-over-budget` for
/// one whose `estimated_tokens` exceeds its `budget_tokens`.
fn judge_tokens(dispatch: &StageDispatch) -> Result<(), RepoError> {
    let Some(tokens) = &dispatch.tokens else {
        return Err(err(format!(
            "bundle-tokens-unrecorded: dispatch {} (stage {}) carries no token account; a \
             Dispatch declares its estimate and budget (RQ-046). Recompile it with `war dispatch`",
            dispatch.dispatch_id, dispatch.stage_id
        )));
    };
    if tokens.estimated_tokens > tokens.budget_tokens {
        return Err(err(format!(
            "bundle-over-budget: dispatch {} (stage {}) estimates ~{} tokens against a budget of \
             {} ({}); an over-budget Dispatch is refused, not bundled (RQ-046, §47.2)",
            dispatch.dispatch_id,
            dispatch.stage_id,
            tokens.estimated_tokens,
            tokens.budget_tokens,
            tokens.method
        )));
    }
    Ok(())
}
pub fn run(
    root: Option<camino::Utf8PathBuf>,
    command: Command,
) -> Result<(Report, serde_json::Value), RepoError> {
    let mut report = Report::default();
    let result = match command {
        Command::Create {
            alias,
            dispatch,
            context,
            emit,
            attachment,
            policy,
        } => {
            let repo = Repository::discover(root)?;
            // RQ-046: a Dispatch from elsewhere is judged before anything is
            // captured. Its account is read from the packet, not recomputed;
            // one with none, or over its own budget, is refused by name, and
            // no context is read and nothing is written.
            let dispatch: StageDispatch = serde_json::from_slice(&read(&dispatch)?).map_err(err)?;
            judge_tokens(&dispatch)?;
            let dir = repo.warrant_dir(&alias)?;
            let loaded = repo.load_warrant(&dir)?;
            if loaded
                .report
                .diagnostics
                .iter()
                .any(|d| matches!(d.severity, Severity::Error | Severity::Unknown))
            {
                return Err(err(
                    "bundle-incomplete-basis: loader errors or unresolved atoms",
                ));
            }
            let (Some(basis), Some(validated)) = (&loaded.basis, &loaded.validated) else {
                return Err(err("bundle-incomplete-basis"));
            };
            let ir = lower(basis, validated).map_err(err)?;
            let context: ContextManifest = serde_json::from_slice(&read(&context)?).map_err(err)?;
            let mut sources = BTreeMap::new();
            insert(
                &mut sources,
                basis.manifest_source.clone(),
                basis.manifest_bytes.clone(),
            )?;
            if let Some(scope) = &basis.scope {
                insert(&mut sources, scope.source.clone(), scope.bytes.clone())?;
            }
            for item in &context.included {
                if item.holder.kind != "git"
                    || item.holder.repository != repo.config.project.name
                    || item.content_digest.is_empty()
                {
                    return Err(err(format!("bundle-unresolved-context: {}", item.id)));
                }
                let raw = capture(&repo, &item.holder.path)?;
                let section = basis
                    .atoms
                    .iter()
                    .find_map(|atom| item.id.strip_prefix(&format!("{}#", atom.source)));
                let bytes = if let Some(heading) = section {
                    openwarrant_core::sections::find(
                        std::str::from_utf8(&raw).map_err(err)?,
                        heading,
                    )
                    .ok_or_else(|| err(format!("bundle-section-missing: {}", item.id)))?
                    .into_bytes()
                } else {
                    raw.clone()
                };
                if section.is_some() {
                    insert(
                        &mut sources,
                        format!("provenance://{}", item.holder.path),
                        raw,
                    )?;
                }
                insert(&mut sources, item.id.clone(), bytes)?;
            }
            for pair in attachment {
                let (id, path) = pair
                    .split_once('=')
                    .ok_or_else(|| err("attachment requires REF=PATH"))?;
                if id.is_empty() {
                    return Err(err("attachment reference is empty"));
                }
                insert(&mut sources, id.into(), read(Utf8Path::new(path))?)?;
            }
            if basis.sas.is_some() && !sources.contains_key("sas://captured") {
                insert(
                    &mut sources,
                    "sas://captured".into(),
                    repo.sas_document()?.1,
                )?;
            }
            insert(
                &mut sources,
                dispatch.submission_schema_ref.clone(),
                bundle::submission_schema().to_vec(),
            )?;
            let policy = match policy {
                Some(path) => read(&path)?,
                None => serde_jcs::to_vec(&Policy::deny_all()).map_err(err)?,
            };
            let bundle = Bundle {
                schema: bundle::SCHEMA.into(),
                dispatch,
                context,
                contract: ir,
                sources,
                policy,
            };
            let bytes = bundle.encode().map_err(err)?;
            let digest = bundle::content_digest(&bytes);
            // Refuse overwrites. All validation precedes the only write.
            let mut out = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&emit)
                .map_err(err)?;
            out.write_all(&bytes).map_err(err)?;
            report.push(Diagnostic::pass(
                "dispatch-bundle.created",
                format!("Portable bundle written to {emit}; digest {digest}"),
            ));
            serde_json::json!({"schema":bundle::SCHEMA,"bundle_digest":digest,"path":emit.as_str(),"execution_authorized":false,"semantic_closure_established":false})
        }
        Command::Check {
            file,
            expected_digest,
            read: reference,
            require_capability,
            trusted_policy_digest,
        } => {
            let checked = bundle::check(&read(&file)?, &expected_digest).map_err(err)?;
            for capability in &require_capability {
                checked
                    .require_capability(capability, trusted_policy_digest.as_deref())
                    .map_err(err)?;
            }
            let content = reference
                .as_deref()
                .map(|id| {
                    checked
                        .read(id)
                        .map_err(err)
                        .and_then(|b| std::str::from_utf8(&b).map(str::to_owned).map_err(err))
                })
                .transpose()?;
            report.push(Diagnostic::pass(
                "dispatch-bundle.integrity",
                "Portable bytes and bindings checked offline; no execution performed.",
            ));
            serde_json::json!({"schema":bundle::SCHEMA,"bundle_digest":expected_digest,"dispatch_digest":checked.dispatch().dispatch_digest,"content":content,"capabilities_checked":require_capability,"execution_authorized":false,"semantic_closure_established":false})
        }
    };
    report.note("Bundle integrity and capability membership are not human assurance, semantic closure, or sandbox enforcement. Caller establishes authority; harness enforces actions.");
    Ok((report, result))
}
