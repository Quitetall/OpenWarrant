// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war gate list` and `war gate run` — local gate execution (SAS §44).
//!
//! # Askability is decided BEFORE execution
//!
//! §44.1 separates askability from result, and the order matters more than the
//! separation. [`askability_of`] runs to completion before any process is
//! spawned, and it is the only thing that can produce a `not_askable` run. Once
//! a process has been spawned the code cannot reach `missing_tool` — which is
//! precisely how "could not ask" becomes "failed" in systems that decide
//! askability from a non-zero exit code.
//!
//! # This command runs code from the corpus
//!
//! `war gate --run` spawns each gate's declared `argv` with the repository root
//! as its working directory. There is no sandbox and no allowlist. A gate
//! definition is executable content, and `mutating` is self-declared — a gate
//! that lies about it will still run. Running this against a corpus you did not
//! author is running that corpus's code. The gate author is the trust boundary,
//! and sandboxing is beta hardening, not something this alpha claims.
//!
//! OW-WAR-0020's Intent records the cost of getting this wrong: the parent
//! project's corpus contained, when measured once at LamQuant `5369da81` on
//! 2026-08-17, 12 missing-tool, 7 missing-script and 4
//! missing-crate gates. Collapsed into `failed`, those 23 would have read as
//! measured failures of the subject rather than as gates that never ran.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use openwarrant_compiler::sha256_hex;
use openwarrant_core::gate::GateDefinition;
use openwarrant_core::{Askability, ExecutionStatus, GateExitResult, GateRun, ReasonCode, Verdict};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

const BONSAI_EVIDENCE_GATE: &str = "software.repo.bonsai-evidence";

/// The deadline used when a gate declares none.
const DEFAULT_GATE_TIMEOUT: Duration = Duration::from_secs(600);

const MAX_GATE_STDOUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_GATE_STDERR_BYTES: usize = 16 * 1024 * 1024;
const MAX_GATE_BINDING_BYTES: u64 = 1024 * 1024;
static NEXT_GATE_RUN: AtomicU64 = AtomicU64::new(0);
static NEXT_GATE_STAGE: AtomicU64 = AtomicU64::new(0);

/// Why a gate cannot be asked, or `None` if it can.
///
/// Every branch returns a §96.4 class, so a gate that cannot be asked always
/// carries a reason and never has to be summarised as a failure.
#[must_use]
pub fn askability_of(def: &GateDefinition, repo: &Repository) -> Option<ReasonCode> {
    // §44.8 first. A mutating gate is not asked in a routine run at all, so it
    // is unaskable here regardless of how well it is declared.
    if def.mutating {
        return Some(ReasonCode::Mutating);
    }
    // §43.3 — an unqualified or invalidated definition cannot be bound, so there
    // is nothing legitimate to ask.
    if !def.lifecycle.is_bindable() {
        return Some(ReasonCode::Malformed);
    }
    if def.validate().is_err() {
        return Some(ReasonCode::Malformed);
    }
    if def.selection_manifest.is_empty() {
        return Some(ReasonCode::ZeroSelectedTests);
    }
    let Some(program) = def.argv.first() else {
        // No argument vector is not an empty command; it is a definition that
        // never said what to run.
        return Some(ReasonCode::Malformed);
    };
    if program.trim().is_empty() {
        return Some(ReasonCode::Malformed);
    }
    if !repo.root.is_dir() {
        return Some(ReasonCode::ForeignWorkingDirectory);
    }
    if !tool_is_available(program, repo) {
        return Some(classify_missing(program));
    }
    None
}

/// Distinguish the three ways a thing can be absent (§96.4 keeps them apart).
///
/// The classification is by SHAPE, which is a heuristic and is the honest limit
/// of what can be known before the thing runs. It is still worth making: §96.4
/// preserves these three as distinct classes, and "the script is not in the tree"
/// and "the toolchain is not installed" call for different repairs.
///
/// `missing_crate` is only reachable when cargo or rustc itself is absent. A
/// gate that invokes a crate that does not exist gets as far as running cargo,
/// so it comes back `completed` + `fail` — correctly, since cargo was asked and
/// answered. Detecting a missing crate inside a successful cargo invocation
/// means parsing cargo's output, which is a gate's job and not the runner's.
fn classify_missing(program: &str) -> ReasonCode {
    if program.ends_with(".sh") || program.ends_with(".py") {
        ReasonCode::MissingScript
    } else if program == "cargo" || program == "rustc" {
        ReasonCode::MissingCrate
    } else {
        ReasonCode::MissingTool
    }
}

/// Whether the program resolves, on PATH or relative to the repository root.
///
/// Deliberately checks existence, NOT the execute bit. A file that exists but
/// cannot be executed is a different failure from one that is not there, and it
/// surfaces as `infrastructure_error` from the spawn rather than being guessed
/// at here. Do not "fix" this into an `access(X_OK)` check without deciding
/// which §96.4 class a non-executable file belongs to.
fn tool_is_available(program: &str, repo: &Repository) -> bool {
    if program.contains('/') {
        let candidate = repo.root.join(program);
        return candidate.is_file() || camino::Utf8Path::new(program).is_file();
    }
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(program).is_file())
}

/// Immutable path for one Gate Run object.
#[must_use]
pub fn run_record_path(run_id: &str, repo: &Repository) -> camino::Utf8PathBuf {
    repo.root
        .join(&repo.config.paths.receipts)
        .join(format!("{run_id}.run.toml"))
}

pub(crate) fn canonical_run_id(run_id: &str) -> bool {
    !run_id.is_empty()
        && run_id.len() <= 160
        && run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

struct GateArtifact<'a> {
    path: &'a camino::Utf8Path,
    bytes: &'a [u8],
    label: &'a str,
}

struct StagedGateArtifact {
    temporary_name: String,
    final_name: String,
    label: String,
}

/// Create one immutable evidence object through the same staged publication
/// seam used by completed receipt bundles.
fn write_new_gate_artifact(
    repo: &Repository,
    path: &camino::Utf8Path,
    bytes: &[u8],
    label: &str,
) -> Result<(), RepoError> {
    publish_gate_artifact_bundle(repo, &[GateArtifact { path, bytes, label }], None)
}

/// Publish supporting objects first and the receipt as the sole commit marker.
///
/// Every byte sequence is staged and synced under a non-receipt name in the
/// same pinned directory. Final names are then created by atomic, no-clobber
/// hard links. The directory is synced after all support links and again after
/// the receipt link, so a crash exposes either no receipt or one whose complete
/// support set was already durable.
fn publish_gate_artifact_bundle(
    repo: &Repository,
    supporting: &[GateArtifact<'_>],
    receipt: Option<GateArtifact<'_>>,
) -> Result<(), RepoError> {
    publish_gate_artifact_bundle_with_barriers(
        repo,
        supporting,
        receipt,
        || Ok(()),
        || Ok(()),
        || Ok(()),
    )
}

fn publish_gate_artifact_bundle_with_barriers(
    repo: &Repository,
    supporting: &[GateArtifact<'_>],
    receipt: Option<GateArtifact<'_>>,
    before_receipt: impl FnOnce() -> Result<(), RepoError>,
    after_receipt_link: impl FnOnce() -> Result<(), RepoError>,
    after_receipt_commit: impl FnOnce() -> Result<(), RepoError>,
) -> Result<(), RepoError> {
    use std::io::Write;

    let first = supporting.first().or(receipt.as_ref()).ok_or_else(|| {
        RepoError::Message("cannot publish an empty Gate artifact bundle".to_owned())
    })?;
    let parent = first.path.parent().ok_or_else(|| {
        RepoError::Message(format!(
            "{} path {} has no parent directory",
            first.label, first.path
        ))
    })?;
    let relative_parent = parent.strip_prefix(&repo.root).map_err(|_| {
        RepoError::Message(format!(
            "Gate artifact directory {parent} is outside repository root {}",
            repo.root
        ))
    })?;
    let relative_parent = relative_parent.as_str();
    if !safe_evidence_path(relative_parent) || relative_parent.split('/').count() > 256 {
        return Err(RepoError::Message(format!(
            "Gate artifact directory {parent} is not a bounded portable repository path"
        )));
    }
    let all = supporting.iter().chain(receipt.iter());
    let mut final_names = std::collections::BTreeSet::new();
    for artifact in all {
        if artifact.path.parent() != Some(parent) {
            return Err(RepoError::Message(format!(
                "{} path {} is outside Gate artifact directory {parent}",
                artifact.label, artifact.path
            )));
        }
        let Some(name) = artifact.path.file_name() else {
            return Err(RepoError::Message(format!(
                "{} path {} has no file name",
                artifact.label, artifact.path
            )));
        };
        if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains(':') {
            return Err(RepoError::Message(format!(
                "{} path {} has a non-portable file name",
                artifact.label, artifact.path
            )));
        }
        if !final_names.insert(name) {
            return Err(RepoError::Message(format!(
                "Gate artifact bundle repeats final name {name:?}"
            )));
        }
    }
    if let Some(receipt) = receipt.as_ref()
        && !receipt
            .path
            .file_name()
            .is_some_and(|name| name.ends_with(".receipt.json"))
    {
        return Err(RepoError::Message(format!(
            "Gate receipt path {} is not a receipt commit marker",
            receipt.path
        )));
    }

    let canonical_root = std::fs::canonicalize(&repo.root).map_err(|source| RepoError::Io {
        context: format!("could not canonicalize repository root {}", repo.root),
        source,
    })?;
    let canonical_root =
        camino::Utf8PathBuf::from_path_buf(canonical_root).map_err(|_| RepoError::NonUtf8Path)?;
    let canonical_parent = canonical_root.join(relative_parent);
    let (directory, directory_guard) =
        ensure_gate_artifact_directory(&canonical_root, relative_parent)?;
    let publication_lock = directory
        .try_clone()
        .map_err(|source| RepoError::Io {
            context: format!("could not clone Gate artifact directory {canonical_parent}"),
            source,
        })?
        .into_std_file();
    publication_lock.lock().map_err(|source| RepoError::Io {
        context: format!("could not lock Gate artifact directory {canonical_parent}"),
        source,
    })?;
    directory_guard.verify_current_location()?;

    for artifact in supporting.iter().chain(receipt.iter()) {
        let name = artifact
            .path
            .file_name()
            .expect("file name validated above");
        match directory.symlink_metadata(name) {
            Ok(_) => {
                return Err(RepoError::Message(format!(
                    "refusing to replace pre-existing {} {}",
                    artifact.label, artifact.path
                )));
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(RepoError::Io {
                    context: format!("could not inspect {} {}", artifact.label, artifact.path),
                    source,
                });
            }
        }
    }

    let mut staged = Vec::with_capacity(supporting.len() + usize::from(receipt.is_some()));
    for (ordinal, artifact) in supporting.iter().chain(receipt.iter()).enumerate() {
        if let Err(error) = directory_guard.verify_current_location() {
            return Err(gate_publication_failure(error, &directory, &staged));
        }
        let sequence = NEXT_GATE_STAGE.fetch_add(1, Ordering::Relaxed);
        let temporary_name = format!(
            ".openwarrant-gate-stage-{}-{sequence}-{ordinal}.tmp",
            std::process::id()
        );
        let mut options = cap_std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = match directory.open_with(&temporary_name, &options) {
            Ok(file) => file,
            Err(source) => {
                let error = RepoError::Io {
                    context: format!("could not stage {} {}", artifact.label, artifact.path),
                    source,
                };
                return Err(gate_publication_failure(error, &directory, &staged));
            }
        };
        staged.push(StagedGateArtifact {
            temporary_name,
            final_name: artifact
                .path
                .file_name()
                .expect("file name validated above")
                .to_owned(),
            label: artifact.label.to_owned(),
        });
        if let Err(source) = file
            .write_all(artifact.bytes)
            .and_then(|()| file.sync_all())
        {
            drop(file);
            let error = RepoError::Io {
                context: format!(
                    "could not stage complete {} {}",
                    artifact.label, artifact.path
                ),
                source,
            };
            return Err(gate_publication_failure(error, &directory, &staged));
        }
        drop(file);
    }

    let support_count = supporting.len();
    for artifact in staged.iter().take(support_count) {
        if let Err(error) = directory_guard.verify_current_location() {
            return Err(gate_publication_failure(error, &directory, &staged));
        }
        if let Err(source) =
            directory.hard_link(&artifact.temporary_name, &directory, &artifact.final_name)
        {
            let error = RepoError::Io {
                context: format!(
                    "could not atomically publish {} {}",
                    artifact.label, artifact.final_name
                ),
                source,
            };
            return Err(gate_publication_failure(error, &directory, &staged));
        }
        if let Err(source) = directory.remove_file(&artifact.temporary_name) {
            let error = RepoError::Io {
                context: format!(
                    "could not remove staged {} {}",
                    artifact.label, artifact.temporary_name
                ),
                source,
            };
            return Err(gate_publication_failure(error, &directory, &staged));
        }
    }
    if let Err(error) = sync_gate_directory(&directory, &canonical_parent) {
        return Err(gate_publication_failure(error, &directory, &staged));
    }
    if let Err(error) = directory_guard.verify_current_location() {
        return Err(gate_publication_failure(
            error,
            &directory,
            &staged[support_count..],
        ));
    }

    if let Some(artifact) = staged.get(support_count) {
        if let Err(error) = before_receipt() {
            return Err(gate_publication_failure(
                error,
                &directory,
                &staged[support_count..],
            ));
        }
        if let Err(error) = directory_guard.verify_current_location() {
            return Err(gate_publication_failure(
                error,
                &directory,
                &staged[support_count..],
            ));
        }
        if let Err(source) =
            directory.hard_link(&artifact.temporary_name, &directory, &artifact.final_name)
        {
            let error = RepoError::Io {
                context: format!(
                    "could not atomically publish {} {}",
                    artifact.label, artifact.final_name
                ),
                source,
            };
            return Err(gate_publication_failure(
                error,
                &directory,
                &staged[support_count..],
            ));
        }
        if let Err(error) = after_receipt_link() {
            return Err(RepoError::GateReceiptCommitted {
                path: canonical_parent.join(&artifact.final_name),
                state: crate::repo_error::GateReceiptCommitState::DurabilityUnknown,
                detail: error.to_string(),
            });
        }
        if let Err(error) = sync_gate_directory(&directory, &canonical_parent) {
            return Err(RepoError::GateReceiptCommitted {
                path: canonical_parent.join(&artifact.final_name),
                state: crate::repo_error::GateReceiptCommitState::DurabilityUnknown,
                detail: error.to_string(),
            });
        }
        if let Err(error) = directory_guard.verify_current_location() {
            let cleanup = cleanup_gate_staging(&directory, &staged[support_count..]);
            let detail = if cleanup.is_empty() {
                error.to_string()
            } else {
                format!(
                    "{error}; staging cleanup also failed for {}",
                    cleanup.join(", ")
                )
            };
            return Err(RepoError::GateReceiptCommitted {
                path: canonical_parent.join(&artifact.final_name),
                state: crate::repo_error::GateReceiptCommitState::ContainmentChanged,
                detail,
            });
        }
        if let Err(error) = after_receipt_commit() {
            return Err(RepoError::GateReceiptCommitted {
                path: canonical_parent.join(&artifact.final_name),
                state: crate::repo_error::GateReceiptCommitState::CleanupIncomplete,
                detail: error.to_string(),
            });
        }
        if let Err(source) = directory.remove_file(&artifact.temporary_name) {
            return Err(RepoError::GateReceiptCommitted {
                path: canonical_parent.join(&artifact.final_name),
                state: crate::repo_error::GateReceiptCommitState::CleanupIncomplete,
                detail: format!(
                    "staged link {} could not be removed: {source}",
                    artifact.temporary_name
                ),
            });
        }
        if let Err(error) = sync_gate_directory(&directory, &canonical_parent) {
            return Err(RepoError::GateReceiptCommitted {
                path: canonical_parent.join(&artifact.final_name),
                state: crate::repo_error::GateReceiptCommitState::CleanupIncomplete,
                detail: format!("staging cleanup durability is unknown: {error}"),
            });
        }
    }
    Ok(())
}

fn ensure_gate_artifact_directory(
    canonical_root: &camino::Utf8Path,
    relative: &str,
) -> Result<
    (
        cap_std::fs::Dir,
        crate::contained_file::OpenedRepositoryDirectory,
    ),
    RepoError,
> {
    let mut current =
        cap_std::fs::Dir::open_ambient_dir(canonical_root, cap_std::ambient_authority()).map_err(
            |source| RepoError::Io {
                context: format!("could not open repository capability root {canonical_root}"),
                source,
            },
        )?;
    let mut current_path = canonical_root.to_owned();
    let mut current_guard = None;

    for component in relative.split('/') {
        loop {
            match current.symlink_metadata(component) {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() || !metadata.is_dir() {
                        return Err(RepoError::Message(format!(
                            "Gate artifact path component {component:?} below {current_path} is not a plain directory"
                        )));
                    }
                    break;
                }
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                    match current.create_dir(component) {
                        Ok(()) => {
                            sync_gate_directory(&current, &current_path)?;
                            break;
                        }
                        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
                            continue;
                        }
                        Err(source) => {
                            return Err(RepoError::Io {
                                context: format!(
                                    "could not create Gate artifact path component {component:?} below {current_path}"
                                ),
                                source,
                            });
                        }
                    }
                }
                Err(source) => {
                    return Err(RepoError::Io {
                        context: format!(
                            "could not inspect Gate artifact path component {component:?} below {current_path}"
                        ),
                        source,
                    });
                }
            }
        }

        current_path.push(component);
        let opened = crate::contained_file::open_repository_directory(
            &current_path,
            canonical_root,
            "Gate artifact directory",
        )?;
        current = cap_std::fs::Dir::from_std_file(opened.try_clone_file()?);
        current_guard = Some(opened);
    }
    Ok((
        current,
        current_guard.ok_or_else(|| {
            RepoError::Message("Gate artifact directory has no path components".to_owned())
        })?,
    ))
}

fn cleanup_gate_staging(
    directory: &cap_std::fs::Dir,
    staged: &[StagedGateArtifact],
) -> Vec<String> {
    let mut failures = Vec::new();
    for artifact in staged {
        if let Err(error) = directory.remove_file(&artifact.temporary_name)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            failures.push(format!("{}: {error}", artifact.temporary_name));
        }
    }
    failures
}

fn gate_publication_failure(
    primary: RepoError,
    directory: &cap_std::fs::Dir,
    staged: &[StagedGateArtifact],
) -> RepoError {
    let cleanup = cleanup_gate_staging(directory, staged);
    if cleanup.is_empty() {
        primary
    } else {
        RepoError::Message(format!(
            "{primary}; staging cleanup also failed for {}",
            cleanup.join(", ")
        ))
    }
}

fn sync_gate_directory(
    directory: &cap_std::fs::Dir,
    path: &camino::Utf8Path,
) -> Result<(), RepoError> {
    directory
        .try_clone()
        .map_err(|source| RepoError::Io {
            context: format!("could not clone Gate artifact directory {path} for sync"),
            source,
        })?
        .into_std_file()
        .sync_all()
        .map_err(|source| RepoError::Io {
            context: format!("could not sync Gate artifact directory {path}"),
            source,
        })
}

/// Persist a non-receipted non-completed observation. Resolution never admits
/// this path; completed runs publish only through a receipt-bound bundle.
fn persist_unreceipted_run(run: &GateRun, repo: &Repository) -> Result<(), RepoError> {
    if !canonical_run_id(&run.id) {
        return Err(RepoError::Message(format!(
            "Gate Run id {:?} is not a safe immutable artifact name",
            run.id
        )));
    }
    let path = run_record_path(&run.id, repo);
    let rendered = toml::to_string_pretty(run)
        .map_err(|error| RepoError::Message(format!("cannot serialize Gate Run: {error}")))?;
    write_new_gate_artifact(repo, &path, rendered.as_bytes(), "Gate Run")
}

fn next_run_id(def: &GateDefinition) -> String {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let gate_digest = sha256_hex(def.key().as_bytes());
    format!(
        "GR-{}-{:x}-{:08x}-{:x}-{:x}",
        &gate_digest[..16],
        elapsed.as_secs(),
        elapsed.subsec_nanos(),
        std::process::id(),
        NEXT_GATE_RUN.fetch_add(1, Ordering::Relaxed)
    )
}

struct GateExecution {
    run: GateRun,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_result: Option<GateExitResult>,
}

impl GateExecution {
    fn without_streams(run: GateRun) -> Self {
        Self {
            run,
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_result: None,
        }
    }
}

fn exact_exit_result(status: &std::process::ExitStatus) -> Option<GateExitResult> {
    if let Some(code) = status.code() {
        return Some(GateExitResult::ExitCode { code });
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status
            .signal()
            .map(|signal| GateExitResult::Signal { signal })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Run one gate and produce its §44 result.
///
/// The `not_askable` path returns before any process is spawned.
#[must_use]
#[cfg(test)]
pub fn run_gate(def: &GateDefinition, repo: &Repository) -> GateRun {
    execute_gate(def, repo, next_run_id(def), None).run
}

fn execute_gate(
    def: &GateDefinition,
    repo: &Repository,
    id: String,
    exact_input: Option<Vec<u8>>,
) -> GateExecution {
    if let Some(reason) = askability_of(def, repo) {
        // §44.4: not_askable pairs with not_run (or invalid, for a malformed
        // definition) and verdict unknown. Never a verdict.
        let execution_status = if reason == ReasonCode::Malformed {
            ExecutionStatus::Invalid
        } else {
            ExecutionStatus::NotRun
        };
        return GateExecution::without_streams(GateRun {
            id,
            gate: def.key(),
            askability: Askability::NotAskable,
            execution_status,
            verdict: Verdict::Unknown,
            reason_code: Some(reason),
        });
    }

    let deadline = def
        .timeout_secs
        .map_or(DEFAULT_GATE_TIMEOUT, Duration::from_secs);

    let mut command = Command::new(&def.argv[0]);
    command.args(&def.argv[1..]).current_dir(&repo.root);
    let output_result = match exact_input {
        Some(input) => crate::git_cmd::output_command_with_input(
            command,
            input,
            MAX_GATE_STDOUT_BYTES,
            MAX_GATE_STDERR_BYTES,
            deadline,
            "Gate",
        ),
        None => crate::git_cmd::output_command(
            command,
            MAX_GATE_STDOUT_BYTES,
            MAX_GATE_STDERR_BYTES,
            deadline,
            "Gate",
        ),
    };
    let output = match output_result {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return GateExecution::without_streams(GateRun {
                id,
                gate: def.key(),
                askability: Askability::NotAskable,
                execution_status: ExecutionStatus::NotRun,
                verdict: Verdict::Unknown,
                reason_code: Some(classify_missing(&def.argv[0])),
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
            return GateExecution::without_streams(GateRun {
                id,
                gate: def.key(),
                askability: Askability::Askable,
                execution_status: ExecutionStatus::Timeout,
                verdict: Verdict::Unknown,
                reason_code: Some(ReasonCode::Timeout),
            });
        }
        Err(_) => {
            return GateExecution::without_streams(GateRun {
                id,
                gate: def.key(),
                askability: Askability::Askable,
                execution_status: ExecutionStatus::InfrastructureError,
                verdict: Verdict::Unknown,
                reason_code: None,
            });
        }
    };

    let passed = output.status.success();
    GateExecution {
        run: GateRun {
            id,
            gate: def.key(),
            askability: Askability::Askable,
            execution_status: ExecutionStatus::Completed,
            verdict: if passed { Verdict::Pass } else { Verdict::Fail },
            reason_code: Some(if passed {
                ReasonCode::Passed
            } else {
                ReasonCode::Failed
            }),
        },
        stdout: output.stdout,
        stderr: output.stderr,
        exit_result: exact_exit_result(&output.status),
    }
}

/// `war gate list` / `war gate run`.
pub struct RunRequest<'a> {
    pub execute: bool,
    pub only: Option<&'a str>,
    pub record: bool,
    pub subject_digests: &'a [String],
    pub raw_evidence_refs: &'a [String],
    pub binding_path: Option<&'a camino::Utf8Path>,
    pub warrant_alias: Option<&'a str>,
}

pub fn run(repo: &Repository, request: RunRequest<'_>) -> Result<Report, RepoError> {
    let RunRequest {
        execute,
        only,
        record,
        subject_digests,
        raw_evidence_refs,
        binding_path,
        warrant_alias,
    } = request;
    if record && !execute {
        return Err(RepoError::Message(
            "--record requires --run; no receipt can exist without execution".to_owned(),
        ));
    }
    if record && (only.is_none() || binding_path.is_none() || warrant_alias.is_none()) {
        return Err(RepoError::Message(
            "recorded Gate execution requires one selected gate, one Gate Binding, and one Warrant"
                .to_owned(),
        ));
    }
    let mut report = Report::default();
    let registry = crate::check::load_gate_registry(repo, &mut report);

    if registry.is_empty() && only.is_none() {
        report.note("no gate definitions found; nothing to run".to_owned());
        return Ok(report);
    }

    let selected: Vec<&GateDefinition> = registry
        .definitions
        .iter()
        .filter(|def| only.is_none_or(|filter| def.gate_id == filter || def.key() == filter))
        .collect();
    if only.is_some() && selected.is_empty() {
        report.push(Diagnostic::error(
            "gate-run.not-found",
            only.unwrap_or_default(),
            "selected Gate Definition does not exist or failed registry admission".to_owned(),
        ));
        return Ok(report);
    }
    if record && selected.len() != 1 {
        report.push(Diagnostic::error(
            "gate-run.ambiguous-selection",
            only.unwrap_or_default(),
            format!(
                "recorded execution requires exactly one Gate Definition; selection matched {}",
                selected.len()
            ),
        ));
        return Ok(report);
    }
    let mut exact_gate_input = if record {
        validate_bonsai_bindings(repo, selected[0], subject_digests, raw_evidence_refs)?
    } else {
        None
    };

    for def in selected {
        if !execute {
            report.push(Diagnostic::pass(
                "gate.listed",
                format!(
                    "{}: {} · lifecycle {} · {}",
                    def.key(),
                    def.provenance,
                    def.lifecycle,
                    match askability_of(def, repo) {
                        Some(r) => format!("not askable ({r})"),
                        None => "askable".to_owned(),
                    }
                ),
            ));
            continue;
        }

        let started_at = receipt::now_rfc3339_public();
        let mut prepared_receipt = if record {
            let Some(binding_path) = binding_path else {
                report.push(Diagnostic::error(
                    "gate-run.receipt-admission-failed",
                    def.key(),
                    "recorded Gate Run has no Gate Binding".to_owned(),
                ));
                continue;
            };
            let Some(warrant_alias) = warrant_alias else {
                report.push(Diagnostic::error(
                    "gate-run.receipt-admission-failed",
                    def.key(),
                    "recorded Gate Run has no authorizing Warrant".to_owned(),
                ));
                continue;
            };
            match receipt::prepare(
                repo,
                def,
                receipt::AdmissionRequest {
                    binding_path,
                    subject_digests,
                    raw_evidence_refs,
                    started_at: &started_at,
                },
            ) {
                Ok(prepared) => {
                    if let Err(error) = crate::authorize::require_authorized_gate_binding(
                        repo,
                        warrant_alias,
                        &prepared.authorized_binding_identity(),
                        prepared.binding_subjects(),
                    ) {
                        report.push(Diagnostic::error(
                            "gate-run.binding-not-authorized",
                            def.key(),
                            error.to_string(),
                        ));
                        continue;
                    }
                    Some(prepared)
                }
                Err(error) => {
                    report.push(Diagnostic::error(
                        "gate-run.receipt-admission-failed",
                        def.key(),
                        error.to_string(),
                    ));
                    continue;
                }
            }
        } else {
            None
        };

        let execution = execute_gate(def, repo, next_run_id(def), exact_gate_input.take());
        let run = &execution.run;
        // Coherence is checked on our own output. A runner that emits an
        // incoherent run is a runner that can emit a passing unaskable gate.
        if let Err(err) = run.validate() {
            report.push(Diagnostic::error(
                "gate-run.incoherent",
                def.key(),
                format!("the runner produced an invalid run: {err}"),
            ));
            continue;
        }

        // §44.6 — persist the RUN, not only its streams, but ONLY when asked.
        //
        // Recording is opt-in because a gate is run for two different reasons.
        // Producing evidence about a subject is one; PROBING the gate's own
        // behaviour is the other, and `conformance/plant.sh` does the second by
        // deliberately corrupting the gate definition and checking the refusal.
        //
        // When recording happened on every invocation, the last such probe left
        // `not_askable / invalid / malformed` as the gate's persisted last word,
        // and every later `war resolve` read that as the real verdict. A
        // deliberately broken test run had silently become the evidentiary
        // record.
        //
        // The fix is NOT to skip recording bad runs — refusing to write failures
        // is the "only record good news" pattern this system exists to prevent.
        // It is to make recording deliberate, which is what the receipts
        // .gitignore already says: evidence is committed on purpose, never as a
        // side effect of running.
        //
        // Before this, a run existed for the length of the process and left
        // behind stdout/stderr text. §56.1's "every required gate has admissible
        // result" cannot be answered from prose, so the structured verdict is
        // written where a later `war resolve` can read it.
        //
        // Written under the receipts path, which is disposable by policy: a run
        // is evidence produced BY running, and it becomes committed evidence
        // deliberately at resolution rather than as a side effect.
        let reason = run
            .reason_code
            .map_or_else(String::new, |r| format!(" ({r})"));
        let line = format!(
            "{}: askability {} · execution {} · verdict {}{reason}",
            def.key(),
            run.askability,
            run.execution_status,
            run.verdict
        );

        // §44.6 — a recorded, completed run produces a receipt. An ordinary
        // probe remains ephemeral; requiring evidentiary identity for it would
        // turn `--record` from an explicit choice into an accidental default.
        // Only a run that actually executed has anything to receipt: an
        // unaskable gate produced no working directory, no exit result and no
        // streams, and minting a receipt for it would be minting evidence of
        // something that did not happen.
        let mut receipt_complete = run.execution_status != ExecutionStatus::Completed;
        if record && run.execution_status == ExecutionStatus::Completed {
            let Some(exit_result) = execution.exit_result else {
                report.push(Diagnostic::error(
                    "gate-run.receipt-failed",
                    def.key(),
                    "completed Gate process exposed neither an exit code nor a signal".to_owned(),
                ));
                continue;
            };
            match receipt::mint(
                repo,
                def,
                run,
                prepared_receipt
                    .take()
                    .expect("recorded Gate Run was admitted before execution"),
                receipt::MintRequest {
                    started_at: &started_at,
                    exit_result,
                },
                &execution.stdout,
                &execution.stderr,
            ) {
                Ok(path) => {
                    receipt_complete = true;
                    report.push(Diagnostic::pass(
                        "gate-run.receipt",
                        format!(
                            "{}: §44.6 receipt written to {}",
                            def.key(),
                            repo.relative(&path)
                        ),
                    ));
                }
                Err(RepoError::GateReceiptCommitted {
                    path,
                    state: crate::repo_error::GateReceiptCommitState::CleanupIncomplete,
                    detail,
                }) => {
                    receipt_complete = true;
                    report.push(Diagnostic::pass(
                        "gate-run.receipt",
                        format!(
                            "{}: §44.6 receipt committed to {}",
                            def.key(),
                            repo.relative(&path)
                        ),
                    ));
                    report.push(Diagnostic::warn(
                        "gate-run.receipt-cleanup-incomplete",
                        def.key(),
                        format!(
                            "receipt is committed and must not be retried under the same run id; cleanup needs attention: {detail}"
                        ),
                    ));
                }
                Err(RepoError::GateReceiptCommitted {
                    path,
                    state,
                    detail,
                }) => {
                    receipt_complete = true;
                    report.push(Diagnostic::unknown(
                        "gate-run.receipt-commit-uncertain",
                        def.key(),
                        format!(
                            "receipt commit reached {} with state {state}; do not retry under the same run id: {detail}",
                            repo.relative(&path)
                        ),
                    ));
                }
                Err(e) => report.push(Diagnostic::error(
                    "gate-run.receipt-failed",
                    def.key(),
                    format!("{e}"),
                )),
            }
        }

        // Completed runs publish inside `receipt::mint`, with the receipt last
        // as commit marker. Non-completed observations have no admissible
        // receipt and are retained only as immutable diagnostic records.
        if record
            && receipt_complete
            && run.execution_status != ExecutionStatus::Completed
            && let Err(err) = persist_unreceipted_run(run, repo)
        {
            report.push(Diagnostic::error(
                "gate-run.not-persisted",
                def.key(),
                format!(
                    "{err} — a run that is not written cannot answer §56.1's \
                     admissible-result requirement later"
                ),
            ));
        }

        if run.satisfies_required_pass() {
            report.push(Diagnostic::pass("gate-run.pass", line));
        } else if run.askability == Askability::NotAskable {
            // §44.1 and RQ-054: this is NOT a failure. Reporting it as one is the
            // collapse §96.4 forbids, and it is why this branch exists.
            //
            // Routed on ASKABILITY, not on the verdict being unknown. Keyed on
            // `is_blocking_unknown()` this branch also caught askable runs that
            // timed out or hit an infrastructure error, and told the reader
            // "could not ask" about a gate that was asked and did not finish —
            // the same class of collapse, in the opposite direction.
            report.push(Diagnostic::unknown(
                "gate-run.unaskable",
                def.key(),
                format!("{line} — could not ask, so there is no result to report"),
            ));
        } else if run.is_blocking_unknown() {
            // Asked, started, did not produce an answer: timeout, cancellation,
            // infrastructure. Blocking under RQ-054 but NOT a failure of the
            // subject, so it gets neither the unaskable rule nor the fail rule.
            report.push(Diagnostic::unknown(
                "gate-run.no-result",
                def.key(),
                format!("{line} — asked, but produced no result"),
            ));
        } else {
            report.push(Diagnostic::error("gate-run.fail", def.key(), line));
        }
    }
    Ok(report)
}

/// Refuse a receipt that attaches a Bonsai document by name but not by bytes,
/// or that lets a passing local gate appear to endorse failed Bonsai evidence.
///
/// This is intentionally narrow: only the Warrant/Bonsai adapter needs these
/// extra receipt fields today. A future generic evidence registry can widen it
/// with typed artifact kinds instead of accepting arbitrary strings now.
fn validate_bonsai_bindings(
    repo: &Repository,
    definition: &GateDefinition,
    subject_digests: &[String],
    raw_evidence_refs: &[String],
) -> Result<Option<Vec<u8>>, RepoError> {
    if definition.key() != format!("{BONSAI_EVIDENCE_GATE}@1.0.0") {
        return Ok(None);
    }
    if subject_digests.len() != 1 || raw_evidence_refs.len() != 1 {
        return Err(RepoError::Message(
            "Bonsai receipt binding requires exactly one contract subject and one evidence reference"
                .to_owned(),
        ));
    }
    let subject = &subject_digests[0];
    let Some(contract_digest) = subject.strip_prefix("contract:sha256:") else {
        return Err(RepoError::Message(
            "Bonsai receipt subject must be contract:sha256:<digest>".to_owned(),
        ));
    };
    if !is_hex_digest(contract_digest) {
        return Err(RepoError::Message(
            "Bonsai receipt contract digest must be 64 lowercase hex characters".to_owned(),
        ));
    }
    let Some((path, expected_digest)) = raw_evidence_refs[0]
        .strip_prefix("file:")
        .and_then(|reference| reference.rsplit_once("#sha256:"))
    else {
        return Err(RepoError::Message(
            "Bonsai evidence reference must be file:<repo-relative-path>#sha256:<digest>"
                .to_owned(),
        ));
    };
    if !safe_evidence_path(path) || !is_hex_digest(expected_digest) {
        return Err(RepoError::Message(
            "Bonsai evidence reference has an unsafe path or invalid digest".to_owned(),
        ));
    }
    let mut consumed_paths = definition
        .argv
        .windows(2)
        .filter(|pair| pair[0] == "--evidence")
        .map(|pair| pair[1].as_str());
    let consumed_path = consumed_paths.next().ok_or_else(|| {
        RepoError::Message(
            "Bonsai Gate Definition does not declare exactly one --evidence path".to_owned(),
        )
    })?;
    if consumed_paths.next().is_some() || consumed_path != "-" {
        return Err(RepoError::Message(
            "Bonsai Gate Definition must consume exactly one immutable --evidence - input"
                .to_owned(),
        ));
    }
    let bytes = crate::repo::read_repository_regular_bounded(
        camino::Utf8Path::new(path),
        &repo.root,
        "Bonsai evidence",
        crate::bonsai::MAX_BONSAI_EVIDENCE_BYTES,
    )?;
    if sha256_hex(&bytes) != expected_digest {
        return Err(RepoError::Message(
            "Bonsai evidence reference digest does not match file bytes".to_owned(),
        ));
    }
    let evidence =
        crate::bonsai::validate_passing_evidence_bytes(&bytes).map_err(RepoError::Message)?;
    if evidence.warrant.contract_digest != contract_digest {
        return Err(RepoError::Message(
            "Bonsai evidence must be a passing v1 report for the bound contract digest".to_owned(),
        ));
    }
    Ok(Some(bytes))
}

fn safe_evidence_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Mint a §44.6 receipt for a completed run, and write it beside its streams.
///
/// # Why this exists
///
/// Through the whole of alpha, `GateReceipt` was implemented, unit-tested, and
/// referenced by no code in any binary. §44.6 says a receipt SHALL record
/// its full execution and evidence identity; nothing produced one, so the requirement was satisfied by a
/// struct definition and a test that constructed one by hand.
///
/// A receipt is also the artifact every beta obligation cites. An obligation
/// demanding "a gate-run receipt whose verdict is pass" is undischargeable by a
/// `#[test]` precisely because a receipt binds wall-clock times, a working
/// directory, an exit result and a digest that has to recompute.
pub mod receipt {
    use std::collections::BTreeSet;

    use super::{
        GateArtifact, MAX_GATE_BINDING_BYTES, canonical_run_id, publish_gate_artifact_bundle,
        run_record_path,
    };

    use camino::Utf8Path;
    use openwarrant_compiler::canonical::sha256_digest;
    use openwarrant_compiler::digest::DigestDomain;
    use openwarrant_compiler::sha256_hex;
    use openwarrant_core::authority::ActorRole;
    use openwarrant_core::gate::GateDefinition;
    use openwarrant_core::legacy_disposition::is_canonical_utc;
    use openwarrant_core::{GateBinding, GateExitResult, GateReceipt, GateRun};

    use crate::gate_adapter::GateRecordingAdapter;
    use crate::repo::{RepoError, Repository, read_repository_regular_bounded};

    /// RFC 3339, UTC, seconds precision — enough to order runs, and no more
    /// precision than the value actually carries.
    /// Public alias so the run path can stamp `started_at` before spawning.
    #[must_use]
    pub fn now_rfc3339_public() -> String {
        now_rfc3339()
    }

    fn now_rfc3339() -> String {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        // Civil-from-days, so a receipt does not pull in a date crate for one
        // timestamp. Correct for all dates this system will ever record.
        let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        format!(
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
            rem / 3600,
            (rem % 3600) / 60,
            rem % 60
        )
    }

    /// Inputs that must be admitted before a recorded gate may execute.
    pub struct AdmissionRequest<'a> {
        pub binding_path: &'a Utf8Path,
        pub subject_digests: &'a [String],
        pub raw_evidence_refs: &'a [String],
        pub started_at: &'a str,
    }

    /// Validated, owned context for one later receipt.
    ///
    /// Owning these values closes the check/use gap between admission and gate
    /// execution: caller slices and Binding bytes cannot change underneath the
    /// receipt after executable work begins.
    #[derive(Debug)]
    pub struct PreparedReceipt {
        binding: GateBinding,
        gate_binding_digest: String,
        binding_source_ref: String,
        producer_actor: String,
        adapter: GateRecordingAdapter,
        raw_evidence_refs: Vec<String>,
    }

    impl PreparedReceipt {
        pub(super) fn authorized_binding_identity(
            &self,
        ) -> crate::authorize::AuthorizedGateBinding {
            crate::authorize::AuthorizedGateBinding {
                id: self.binding.id.clone(),
                gate: self.binding.gate.key(),
                digest: self.gate_binding_digest.clone(),
                source_ref: self.binding_source_ref.clone(),
            }
        }

        pub(super) fn binding_subjects(&self) -> &[String] {
            &self.binding.subjects
        }
    }

    /// Admit exact evidentiary context before executing repository code.
    pub fn prepare(
        repo: &Repository,
        def: &GateDefinition,
        request: AdmissionRequest<'_>,
    ) -> Result<PreparedReceipt, RepoError> {
        let AdmissionRequest {
            binding_path,
            subject_digests,
            raw_evidence_refs,
            started_at,
        } = request;
        let canonical_raw_evidence: BTreeSet<&str> =
            raw_evidence_refs.iter().map(String::as_str).collect();
        if canonical_raw_evidence.len() != raw_evidence_refs.len()
            || raw_evidence_refs
                .iter()
                .any(|reference| reference.is_empty() || reference.trim() != reference)
        {
            return Err(RepoError::Message(
                "raw evidence references must be a canonical nonblank set".to_owned(),
            ));
        }
        let adapter = GateRecordingAdapter::for_definition(def).map_err(RepoError::Message)?;
        adapter
            .validate_raw_evidence(raw_evidence_refs)
            .map_err(RepoError::Message)?;
        let binding_path = if binding_path.is_absolute() {
            binding_path.to_owned()
        } else {
            repo.root.join(binding_path)
        };
        let binding_ref = binding_path.strip_prefix(&repo.root).map_err(|_| {
            RepoError::Message(format!(
                "Gate Binding {binding_path} is outside repository root {}",
                repo.root
            ))
        })?;
        let binding_ref = binding_ref.as_str().replace('\\', "/");
        if !super::safe_evidence_path(&binding_ref)
            || binding_ref.contains(':')
            || !binding_ref.ends_with(".binding.json")
        {
            return Err(RepoError::Message(format!(
                "Gate Binding {binding_ref:?} must be a portable repository-relative .binding.json object"
            )));
        }
        let binding_bytes = read_repository_regular_bounded(
            &binding_path,
            &repo.root,
            "Gate Binding",
            MAX_GATE_BINDING_BYTES,
        )?;
        let binding: GateBinding = openwarrant_core::legacy_disposition::parse_strict_json(
            &binding_bytes,
        )
        .map_err(|error| {
            RepoError::Message(format!("cannot parse Gate Binding {binding_path}: {error}"))
        })?;
        if binding.id.trim().is_empty()
            || binding.gate.key() != def.key()
            || binding.gate.digest.is_empty()
            || binding.gate.digest != def.digest
            || binding.subjects.is_empty()
        {
            return Err(RepoError::Message(format!(
                "Gate Binding {binding_ref:?} does not exactly bind {} and at least one subject",
                def.key()
            )));
        }
        let bound_subjects: BTreeSet<&str> = binding.subjects.iter().map(String::as_str).collect();
        if bound_subjects.len() != binding.subjects.len()
            || bound_subjects
                .iter()
                .any(|subject| subject.trim().is_empty())
        {
            return Err(RepoError::Message(format!(
                "Gate Binding {binding_ref:?} has duplicate or blank subjects"
            )));
        }
        if !subject_digests.is_empty() {
            let requested: BTreeSet<&str> = subject_digests.iter().map(String::as_str).collect();
            if requested.len() != subject_digests.len() || requested != bound_subjects {
                return Err(RepoError::Message(format!(
                    "--subject-digest inventory does not match Gate Binding {binding_ref:?}"
                )));
            }
        }
        if def.selection_manifest.is_empty() {
            return Err(RepoError::Message(
                "recorded Gate Run needs a verifier-controlled selection manifest in its Gate Definition"
                    .to_owned(),
            ));
        }
        let producer_actor = binding.evidence_policy.producer.clone();
        if producer_actor.is_empty() || producer_actor.trim() != producer_actor {
            return Err(RepoError::Message(format!(
                "Gate Binding {binding_ref:?} names a blank or noncanonical evidence producer {:?}",
                producer_actor
            )));
        }
        if producer_actor == repo.performer()
            && !binding.evidence_policy.performer_authored_report_admissible
        {
            return Err(RepoError::Message(format!(
                "Gate Binding {binding_ref:?} forbids a performer-authored receipt"
            )));
        }
        if !is_canonical_utc(started_at) {
            return Err(RepoError::Message(
                "recorded Gate Run started_at must be canonical UTC".to_owned(),
            ));
        }
        let authority = repo.load_authority_register()?;
        let producer_assignment = authority.actor(&producer_actor).ok_or_else(|| {
            RepoError::Message(format!(
                "Gate Binding {binding_ref:?} producer actor {producer_actor:?} has no authority assignment"
            ))
        })?;
        if !producer_assignment.holds(ActorRole::Verifier)
            || !is_canonical_utc(&producer_assignment.effective_time)
            || producer_assignment.effective_time.as_str() > started_at
        {
            return Err(RepoError::Message(format!(
                "Gate Binding {binding_ref:?} producer actor {producer_actor:?} does not hold an effective Verifier assignment at execution"
            )));
        }
        if binding
            .fixtures
            .iter()
            .any(|fixture| fixture.reference.trim().is_empty() || fixture.digest.trim().is_empty())
        {
            return Err(RepoError::Message(format!(
                "Gate Binding {binding_ref:?} has an incomplete fixture inventory"
            )));
        }
        if !binding.fixtures.is_empty() {
            return Err(RepoError::Message(format!(
                "Gate Binding {binding_ref:?} declares fixtures, but this runner has no fixture-aware execution adapter that can prove the exact admitted bytes were consumed"
            )));
        }
        if !binding.parameters.is_empty() || !binding.pass_predicate.is_empty() {
            return Err(RepoError::Message(format!(
                "Gate Binding {binding_ref:?} declares parameters or a pass predicate, but this runner has no binding-aware execution adapter"
            )));
        }
        let gate_binding_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateBinding, &binding)
                .map_err(|error| RepoError::Message(format!("{error}")))?
        );
        Ok(PreparedReceipt {
            binding,
            gate_binding_digest,
            binding_source_ref: binding_ref,
            producer_actor,
            adapter,
            raw_evidence_refs: raw_evidence_refs.to_vec(),
        })
    }

    /// Completion fields only known after execution.
    pub struct MintRequest<'a> {
        pub started_at: &'a str,
        pub exit_result: GateExitResult,
    }

    /// Build and persist a receipt from pre-admitted context. Returns its path.
    ///
    /// The receipt is VALIDATED before it is written. A malformed receipt on
    /// disk is worse than none: it looks like evidence.
    pub fn mint(
        repo: &Repository,
        def: &GateDefinition,
        run: &GateRun,
        prepared: PreparedReceipt,
        request: MintRequest<'_>,
        stdout: &[u8],
        stderr: &[u8],
    ) -> Result<camino::Utf8PathBuf, RepoError> {
        let MintRequest {
            started_at,
            exit_result,
        } = request;
        let PreparedReceipt {
            binding,
            gate_binding_digest,
            producer_actor,
            adapter,
            raw_evidence_refs,
            binding_source_ref: _,
        } = prepared;
        let rel = |p: &Utf8Path| repo.relative(p);
        let dir = repo.root.join(&repo.config.paths.receipts);
        if !canonical_run_id(&run.id) {
            return Err(RepoError::Message(format!(
                "Gate Run id {:?} is not a safe immutable artifact name",
                run.id
            )));
        }
        let stdout_path = dir.join(format!("{}.stdout.txt", run.id));
        let stderr_path = dir.join(format!("{}.stderr.txt", run.id));
        let binding_path = dir.join(format!("{}.binding.json", run.id));
        let selection_path = dir.join(format!("{}.selection.json", run.id));
        let stdout_digest = format!("sha256:{}", sha256_hex(stdout));
        let stderr_digest = format!("sha256:{}", sha256_hex(stderr));
        let gate_run_path = run_record_path(&run.id, repo);
        let selection = adapter.observe(def, run).map_err(RepoError::Message)?;
        let selection_json = serde_json::to_string_pretty(&selection).map_err(|error| {
            RepoError::Message(format!(
                "cannot serialize test-selection observation: {error}"
            ))
        })?;
        let selection_body = format!("{selection_json}\n");
        let selection_digest = format!("sha256:{}", sha256_hex(selection_body.as_bytes()));
        let selected_tests = selection.selected_test_manifest.clone();

        let run_body = toml::to_string_pretty(run)
            .map_err(|error| RepoError::Message(format!("cannot serialize Gate Run: {error}")))?;
        let mut receipt = GateReceipt {
            schema: openwarrant_core::GATE_RECEIPT_SCHEMA.to_owned(),
            kind: openwarrant_core::GATE_RECEIPT_KIND.to_owned(),
            run_id: run.id.clone(),
            gate_run_digest: format!("sha256:{}", sha256_hex(run_body.as_bytes())),
            // Registry loading binds local candidates to their exact source
            // digest. A direct caller that supplies no identity gets a malformed
            // receipt rather than a digest invented from the gate key.
            gate_definition_digest: def.digest.clone(),
            gate_binding_digest,
            subject_digests: binding.subjects.clone(),
            fixture_digests: binding
                .fixtures
                .iter()
                .map(|fixture| fixture.digest.clone())
                .collect(),
            runner: adapter.id().to_owned(),
            // Tool identity and actor identity are distinct. Omitting this
            // field would let a performer-authored receipt appear independent
            // because `"war gate --run" != "claude"`.
            producer_actor,
            runtime_environment: format!(
                "{} {} / rustc {}",
                std::env::consts::OS,
                std::env::consts::ARCH,
                option_env!("CARGO_PKG_RUST_VERSION").unwrap_or("unknown")
            ),
            arguments: def.argv.clone(),
            working_directory: repo.root.to_string(),
            started_at: started_at.to_owned(),
            completed_at: now_rfc3339(),
            exit_result,
            selected_test_count: u64::try_from(selected_tests.len()).map_err(|_| {
                RepoError::Message("selected-test manifest does not fit receipt count".to_owned())
            })?,
            selected_test_manifest: selected_tests,
            selection_observation_ref: rel(&selection_path),
            selection_observation_digest: selection_digest,
            raw_evidence_refs,
            stdout_ref: rel(&stdout_path),
            stdout_digest,
            stderr_ref: rel(&stderr_path),
            stderr_digest,
            resource_usage: format!("wall-clock only; {} argv item(s)", def.argv.len()),
            verdict: run.verdict,
            receipt_digest: String::new(),
            extensions: openwarrant_core::GateReceiptExtensions::default(),
        };

        // Digest last, over everything else, so it covers the record it seals.
        receipt.receipt_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateReceipt, &receipt)
                .map_err(|e| RepoError::Message(format!("{e}")))?
        );

        receipt.validate().map_err(|e| {
            RepoError::Message(format!("refusing to write a malformed receipt: {e}"))
        })?;

        let path = dir.join(format!("{}.receipt.json", run.id));
        let binding_body = serde_json::to_vec_pretty(&binding).map_err(|error| {
            RepoError::Message(format!("cannot serialize Gate Binding: {error}"))
        })?;
        let body = serde_json::to_string_pretty(&receipt)
            .map_err(|e| RepoError::Message(format!("{e}")))?;
        let receipt_body = format!("{body}\n");
        publish_gate_artifact_bundle(
            repo,
            &[
                GateArtifact {
                    path: &binding_path,
                    bytes: &binding_body,
                    label: "Gate Binding",
                },
                GateArtifact {
                    path: &stdout_path,
                    bytes: stdout,
                    label: "Gate stdout",
                },
                GateArtifact {
                    path: &stderr_path,
                    bytes: stderr,
                    label: "Gate stderr",
                },
                GateArtifact {
                    path: &gate_run_path,
                    bytes: run_body.as_bytes(),
                    label: "Gate Run",
                },
                GateArtifact {
                    path: &selection_path,
                    bytes: selection_body.as_bytes(),
                    label: "test-selection observation",
                },
            ],
            Some(GateArtifact {
                path: &path,
                bytes: receipt_body.as_bytes(),
                label: "Gate receipt",
            }),
        )?;
        Ok(path)
    }

    #[cfg(test)]
    mod tests {
        use std::collections::BTreeMap;
        use std::sync::atomic::{AtomicU64, Ordering};

        use camino::Utf8PathBuf;
        use openwarrant_core::gate::{EvidencePolicy, GateLifecycle, GateProvenance, GateRef};
        use openwarrant_core::{
            Askability, ExecutionStatus, GateBinding, GateReceipt, Namespace, RepositoryConfig,
            Verdict,
        };

        use super::*;

        static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);

        struct Scratch {
            root: Utf8PathBuf,
        }

        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.root);
            }
        }

        fn fixture() -> (Scratch, Repository, GateDefinition, GateRun, Utf8PathBuf) {
            let mut root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
                .expect("temporary directory is UTF-8");
            root.push(format!(
                "openwarrant-gate-receipt-{}-{}",
                std::process::id(),
                NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = std::fs::remove_dir_all(&root);
            let config = RepositoryConfig::new(
                "Gate receipt fixture",
                Namespace::parse("OW").expect("fixture namespace"),
            );
            let repo = Repository {
                root: root.clone(),
                config,
            };
            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            std::fs::create_dir_all(&receipt_dir).expect("receipt directory");
            let authority_dir = repo.root.join("docs/authority");
            std::fs::create_dir_all(&authority_dir).expect("authority directory");
            std::fs::write(
                authority_dir.join("roles.toml"),
                r#"[[assignment]]
actor = "gate_runner"
actor_kind = "agent"
roles = ["verifier"]
assigned_by = "fixture-authorizer"
effective_time = "2026-01-01T00:00:00Z"
"#,
            )
            .expect("write verifier authority fixture");
            let digest = format!("sha256:{}", "a".repeat(64));
            let definition = GateDefinition {
                gate_id: "fixture.pass".to_owned(),
                version: "1.0.0".to_owned(),
                digest: digest.clone(),
                lifecycle: GateLifecycle::Draft,
                implementation_ref: "fixture".to_owned(),
                input_kinds: vec![],
                output_schema_ref: "fixture".to_owned(),
                fault_model: vec![],
                known_blind_spots: vec![],
                qualification: None,
                provenance: GateProvenance::LocalCandidate,
                argv: vec!["fixture".to_owned()],
                selection_manifest: vec!["test://fixture-pass".to_owned()],
                mutating: false,
                timeout_secs: None,
            };
            let binding = GateBinding {
                id: "GB-1".to_owned(),
                gate: GateRef {
                    id: definition.gate_id.clone(),
                    version: definition.version.clone(),
                    digest,
                },
                subjects: vec![format!("sha256:{}", "b".repeat(64))],
                fixtures: vec![],
                parameters: BTreeMap::new(),
                pass_predicate: BTreeMap::new(),
                evidence_policy: EvidencePolicy {
                    producer: "gate_runner".to_owned(),
                    performer_authored_report_admissible: false,
                },
            };
            let binding_path = receipt_dir.join("input.binding.json");
            std::fs::write(
                &binding_path,
                serde_json::to_vec(&binding).expect("serialize Gate Binding"),
            )
            .expect("write Gate Binding");
            std::fs::write(
                receipt_dir.join("fixture_pass_1_0_0.stdout.txt"),
                b"stdout\n",
            )
            .expect("write stdout");
            std::fs::write(
                receipt_dir.join("fixture_pass_1_0_0.stderr.txt"),
                b"stderr\n",
            )
            .expect("write stderr");
            let run = GateRun {
                id: "GR-1".to_owned(),
                gate: definition.key(),
                askability: Askability::Askable,
                execution_status: ExecutionStatus::Completed,
                verdict: Verdict::Pass,
                reason_code: None,
            };
            (Scratch { root }, repo, definition, run, binding_path)
        }

        #[test]
        fn mint_binds_exact_binding_actor_tests_and_raw_streams() {
            let (_scratch, repo, definition, run, binding_path) = fixture();
            let prepared = prepare(
                &repo,
                &definition,
                AdmissionRequest {
                    binding_path: &binding_path,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    started_at: "2026-08-26T00:00:00Z",
                },
            )
            .expect("admit exact receipt context");
            let path = mint(
                &repo,
                &definition,
                &run,
                prepared,
                MintRequest {
                    started_at: "2026-08-26T00:00:00Z",
                    exit_result: GateExitResult::ExitCode { code: 0 },
                },
                b"stdout\n",
                b"stderr\n",
            )
            .expect("mint exact receipt");
            let receipt: GateReceipt = openwarrant_core::legacy_disposition::parse_strict_json(
                &std::fs::read(path).expect("read receipt"),
            )
            .expect("parse receipt");

            assert_eq!(receipt.producer_actor, "gate_runner");
            assert_eq!(receipt.selected_test_count, 1);
            assert_eq!(receipt.selected_test_manifest, ["test://fixture-pass"]);
            assert!(receipt.gate_binding_digest.starts_with("sha256:"));
            assert_eq!(
                receipt.stdout_digest,
                format!("sha256:{}", sha256_hex(b"stdout\n"))
            );
            assert_eq!(
                receipt.stderr_digest,
                format!("sha256:{}", sha256_hex(b"stderr\n"))
            );
            assert!(receipt.validate().is_ok());
        }

        #[test]
        fn mint_refuses_to_label_performer_output_independent() {
            let (_scratch, repo, definition, _run, binding_path) = fixture();
            let mut binding: GateBinding = openwarrant_core::legacy_disposition::parse_strict_json(
                &std::fs::read(&binding_path).expect("Gate Binding bytes"),
            )
            .expect("Gate Binding");
            binding.evidence_policy.producer = repo.performer();
            binding.evidence_policy.performer_authored_report_admissible = false;
            std::fs::write(
                &binding_path,
                serde_json::to_vec(&binding).expect("serialize Gate Binding plant"),
            )
            .expect("write Gate Binding plant");
            let error = prepare(
                &repo,
                &definition,
                AdmissionRequest {
                    binding_path: &binding_path,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    started_at: "2026-08-26T00:00:00Z",
                },
            )
            .expect_err("Gate Binding forbids performer-authored evidence");
            assert!(
                error
                    .to_string()
                    .contains("forbids a performer-authored receipt")
            );
        }

        #[test]
        fn prepare_refuses_unassigned_producer_before_execution() {
            let (_scratch, repo, definition, _run, binding_path) = fixture();
            std::fs::remove_file(repo.root.join("docs/authority/roles.toml"))
                .expect("remove verifier authority fixture");

            let error = prepare(
                &repo,
                &definition,
                AdmissionRequest {
                    binding_path: &binding_path,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    started_at: "2026-08-26T00:00:00Z",
                },
            )
            .expect_err("unassigned evidence producer must fail preflight");

            assert!(error.to_string().contains("no authority assignment"));
        }

        #[test]
        fn mint_refuses_clock_rollback_before_publishing_any_bundle_member() {
            let (_scratch, repo, definition, run, binding_path) = fixture();
            let prepared = prepare(
                &repo,
                &definition,
                AdmissionRequest {
                    binding_path: &binding_path,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    started_at: "2026-08-26T00:00:00Z",
                },
            )
            .expect("admit receipt context");

            let error = mint(
                &repo,
                &definition,
                &run,
                prepared,
                MintRequest {
                    started_at: "9999-12-31T23:59:59Z",
                    exit_result: GateExitResult::ExitCode { code: 0 },
                },
                b"stdout\n",
                b"stderr\n",
            )
            .expect_err("clock rollback must make receipt unpublishable");

            assert!(error.to_string().contains("chronology"));
            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            for suffix in [
                "binding.json",
                "stdout.txt",
                "stderr.txt",
                "run.toml",
                "receipt.json",
            ] {
                assert!(
                    !receipt_dir.join(format!("{}.{}", run.id, suffix)).exists(),
                    "clock rollback published {suffix}"
                );
            }
        }

        #[test]
        fn a_preexisting_receipt_blocks_before_any_supporting_artifact_is_published() {
            let (_scratch, repo, definition, run, binding_path) = fixture();
            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            let receipt_path = receipt_dir.join(format!("{}.receipt.json", run.id));
            let sentinel = b"preexisting receipt sentinel\n";
            std::fs::write(&receipt_path, sentinel).expect("plant preexisting receipt");
            let prepared = prepare(
                &repo,
                &definition,
                AdmissionRequest {
                    binding_path: &binding_path,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    started_at: "2026-08-26T00:00:00Z",
                },
            )
            .expect("admit receipt context");

            mint(
                &repo,
                &definition,
                &run,
                prepared,
                MintRequest {
                    started_at: "2026-08-26T00:00:00Z",
                    exit_result: GateExitResult::ExitCode { code: 0 },
                },
                b"stdout\n",
                b"stderr\n",
            )
            .expect_err("immutable receipt path must block the entire bundle");

            assert_eq!(
                std::fs::read(&receipt_path).expect("read receipt sentinel"),
                sentinel
            );
            for suffix in ["binding.json", "stdout.txt", "stderr.txt", "run.toml"] {
                assert!(
                    !receipt_dir.join(format!("{}.{}", run.id, suffix)).exists(),
                    "preexisting receipt allowed {suffix} publication"
                );
            }
        }

        #[test]
        fn a_preexisting_supporting_artifact_is_unchanged_and_blocks_the_bundle() {
            let (_scratch, repo, definition, run, binding_path) = fixture();
            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            let stderr_path = receipt_dir.join(format!("{}.stderr.txt", run.id));
            let sentinel = b"preexisting stderr sentinel\n";
            std::fs::write(&stderr_path, sentinel).expect("plant preexisting stderr");
            let prepared = prepare(
                &repo,
                &definition,
                AdmissionRequest {
                    binding_path: &binding_path,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    started_at: "2026-08-26T00:00:00Z",
                },
            )
            .expect("admit receipt context");

            mint(
                &repo,
                &definition,
                &run,
                prepared,
                MintRequest {
                    started_at: "2026-08-26T00:00:00Z",
                    exit_result: GateExitResult::ExitCode { code: 0 },
                },
                b"stdout\n",
                b"stderr\n",
            )
            .expect_err("immutable supporting path must block the entire bundle");

            assert_eq!(
                std::fs::read(&stderr_path).expect("read stderr sentinel"),
                sentinel
            );
            for suffix in ["binding.json", "stdout.txt", "run.toml", "receipt.json"] {
                assert!(
                    !receipt_dir.join(format!("{}.{}", run.id, suffix)).exists(),
                    "preexisting support allowed {suffix} publication"
                );
            }
        }

        #[test]
        fn concurrent_writers_publish_exactly_one_complete_bundle_without_stage_debris() {
            use std::sync::{Arc, Barrier};

            let (_scratch, repo, definition, run, binding_path) = fixture();
            let prepared_a = prepare(
                &repo,
                &definition,
                AdmissionRequest {
                    binding_path: &binding_path,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    started_at: "2026-08-26T00:00:00Z",
                },
            )
            .expect("admit first receipt context");
            let prepared_b = prepare(
                &repo,
                &definition,
                AdmissionRequest {
                    binding_path: &binding_path,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    started_at: "2026-08-26T00:00:00Z",
                },
            )
            .expect("admit second receipt context");
            let barrier = Arc::new(Barrier::new(2));
            let outcomes = std::thread::scope(|scope| {
                let barrier_a = Arc::clone(&barrier);
                let repo_a = &repo;
                let definition_a = &definition;
                let run_a = &run;
                let first = scope.spawn(move || {
                    barrier_a.wait();
                    mint(
                        repo_a,
                        definition_a,
                        run_a,
                        prepared_a,
                        MintRequest {
                            started_at: "2026-08-26T00:00:00Z",
                            exit_result: GateExitResult::ExitCode { code: 0 },
                        },
                        b"publisher-a-stdout\n",
                        b"publisher-a-stderr\n",
                    )
                });
                let barrier_b = Arc::clone(&barrier);
                let repo_b = &repo;
                let definition_b = &definition;
                let run_b = &run;
                let second = scope.spawn(move || {
                    barrier_b.wait();
                    mint(
                        repo_b,
                        definition_b,
                        run_b,
                        prepared_b,
                        MintRequest {
                            started_at: "2026-08-26T00:00:00Z",
                            exit_result: GateExitResult::ExitCode { code: 0 },
                        },
                        b"publisher-b-stdout\n",
                        b"publisher-b-stderr\n",
                    )
                });
                [
                    first.join().expect("first publisher thread"),
                    second.join().expect("second publisher thread"),
                ]
            });

            assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
            assert_eq!(outcomes.iter().filter(|result| result.is_err()).count(), 1);
            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            let receipt_path = receipt_dir.join(format!("{}.receipt.json", run.id));
            let receipt: GateReceipt = openwarrant_core::legacy_disposition::parse_strict_json(
                &std::fs::read(receipt_path).expect("read winning receipt"),
            )
            .expect("winning receipt is complete");
            assert!(receipt.validate().is_ok());
            let stdout = std::fs::read(receipt_dir.join(format!("{}.stdout.txt", run.id)))
                .expect("read winning stdout");
            let stderr = std::fs::read(receipt_dir.join(format!("{}.stderr.txt", run.id)))
                .expect("read winning stderr");
            assert!(
                (stdout == b"publisher-a-stdout\n" && stderr == b"publisher-a-stderr\n")
                    || (stdout == b"publisher-b-stdout\n" && stderr == b"publisher-b-stderr\n"),
                "concurrent writers produced a mixed support bundle"
            );
            assert_eq!(
                receipt.stdout_digest,
                format!("sha256:{}", sha256_hex(&stdout))
            );
            assert_eq!(
                receipt.stderr_digest,
                format!("sha256:{}", sha256_hex(&stderr))
            );
            for entry in std::fs::read_dir(&receipt_dir).expect("read receipt directory") {
                let name = entry
                    .expect("receipt entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned();
                assert!(
                    !name.starts_with(".openwarrant-gate-stage-"),
                    "publication left staging object {name}"
                );
            }
        }

        #[test]
        fn failure_after_support_sync_never_publishes_receipt_commit_marker() {
            let (_scratch, repo, _definition, _run, _binding_path) = fixture();
            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            let support_path = receipt_dir.join("GR-INJECTED.stdout.txt");
            let receipt_path = receipt_dir.join("GR-INJECTED.receipt.json");
            let error = super::super::publish_gate_artifact_bundle_with_barriers(
                &repo,
                &[super::super::GateArtifact {
                    path: &support_path,
                    bytes: b"durable support\n",
                    label: "injected Gate support",
                }],
                Some(super::super::GateArtifact {
                    path: &receipt_path,
                    bytes: b"must never publish\n",
                    label: "injected Gate receipt",
                }),
                || {
                    Err(RepoError::Message(
                        "injected failure before receipt publication".to_owned(),
                    ))
                },
                || Ok(()),
                || Ok(()),
            )
            .expect_err("injected pre-receipt failure");

            assert!(error.to_string().contains("injected failure"));
            assert_eq!(
                std::fs::read(&support_path).expect("support was durably published first"),
                b"durable support\n"
            );
            assert!(
                !receipt_path.exists(),
                "partial publication exposed an admissibility marker"
            );
            for entry in std::fs::read_dir(&receipt_dir).expect("read receipt directory") {
                let name = entry
                    .expect("receipt entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned();
                assert!(
                    !name.starts_with(".openwarrant-gate-stage-"),
                    "failed publication left staging object {name}"
                );
            }
        }

        #[test]
        fn post_link_failure_reports_durability_unknown_without_rolling_back_receipt() {
            let (_scratch, repo, _definition, _run, _binding_path) = fixture();
            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            let support_path = receipt_dir.join("GR-POST-LINK.stdout.txt");
            let receipt_path = receipt_dir.join("GR-POST-LINK.receipt.json");
            let error = super::super::publish_gate_artifact_bundle_with_barriers(
                &repo,
                &[super::super::GateArtifact {
                    path: &support_path,
                    bytes: b"post-link support\n",
                    label: "post-link Gate support",
                }],
                Some(super::super::GateArtifact {
                    path: &receipt_path,
                    bytes: b"post-link receipt\n",
                    label: "post-link Gate receipt",
                }),
                || Ok(()),
                || {
                    Err(RepoError::Message(
                        "injected directory-barrier failure".to_owned(),
                    ))
                },
                || Ok(()),
            )
            .expect_err("post-link barrier failure must be typed");

            assert!(matches!(
                error,
                RepoError::GateReceiptCommitted {
                    state: crate::repo_error::GateReceiptCommitState::DurabilityUnknown,
                    ..
                }
            ));
            assert_eq!(
                std::fs::read(&receipt_path).expect("linked receipt remains visible"),
                b"post-link receipt\n"
            );
        }

        #[test]
        fn post_commit_cleanup_failure_reports_committed_receipt_without_rollback() {
            let (_scratch, repo, _definition, _run, _binding_path) = fixture();
            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            let support_path = receipt_dir.join("GR-POST-COMMIT.stdout.txt");
            let receipt_path = receipt_dir.join("GR-POST-COMMIT.receipt.json");
            let error = super::super::publish_gate_artifact_bundle_with_barriers(
                &repo,
                &[super::super::GateArtifact {
                    path: &support_path,
                    bytes: b"post-commit support\n",
                    label: "post-commit Gate support",
                }],
                Some(super::super::GateArtifact {
                    path: &receipt_path,
                    bytes: b"post-commit receipt\n",
                    label: "post-commit Gate receipt",
                }),
                || Ok(()),
                || Ok(()),
                || {
                    Err(RepoError::Message(
                        "injected cleanup failure after commit".to_owned(),
                    ))
                },
            )
            .expect_err("post-commit cleanup failure must be typed");

            assert!(matches!(
                error,
                RepoError::GateReceiptCommitted {
                    state: crate::repo_error::GateReceiptCommitState::CleanupIncomplete,
                    ..
                }
            ));
            assert_eq!(
                std::fs::read(&receipt_path).expect("committed receipt remains visible"),
                b"post-commit receipt\n"
            );
        }

        #[cfg(unix)]
        #[test]
        fn moved_receipt_directory_is_detected_before_receipt_publication() {
            let (_scratch, repo, _definition, _run, _binding_path) = fixture();
            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            let moved = repo.root.with_file_name(format!(
                "openwarrant-moved-receipts-{}-{}",
                std::process::id(),
                NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
            ));
            let support_path = receipt_dir.join("GR-MOVED.stdout.txt");
            let receipt_path = receipt_dir.join("GR-MOVED.receipt.json");
            let error = super::super::publish_gate_artifact_bundle_with_barriers(
                &repo,
                &[super::super::GateArtifact {
                    path: &support_path,
                    bytes: b"support before relocation\n",
                    label: "relocation Gate support",
                }],
                Some(super::super::GateArtifact {
                    path: &receipt_path,
                    bytes: b"must never publish after relocation\n",
                    label: "relocation Gate receipt",
                }),
                || {
                    std::fs::rename(&receipt_dir, &moved).map_err(|source| RepoError::Io {
                        context: "could not plant receipt-directory relocation".to_owned(),
                        source,
                    })
                },
                || Ok(()),
                || Ok(()),
            )
            .expect_err("directory relocation must fail before receipt link");

            assert!(
                error.to_string().contains("Gate artifact directory")
                    || error.to_string().contains("could not resolve"),
                "unexpected relocation refusal: {error}"
            );
            assert!(
                !moved.join("GR-MOVED.receipt.json").exists(),
                "receipt was published through relocated directory handle"
            );
            for entry in std::fs::read_dir(&moved).expect("read relocated receipt directory") {
                let name = entry
                    .expect("relocated entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned();
                assert!(
                    !name.starts_with(".openwarrant-gate-stage-"),
                    "relocation left staging object {name}"
                );
            }
            let _ = std::fs::remove_dir_all(moved);
        }

        #[cfg(unix)]
        #[test]
        fn receipt_directory_symlink_cannot_create_outside_repository() {
            use std::os::unix::fs::symlink;

            let (_scratch, mut repo, definition, run, binding_path) = fixture();
            let outside = repo.root.with_file_name(format!(
                "openwarrant-receipt-outside-{}-{}",
                std::process::id(),
                NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&outside).expect("create outside directory");
            symlink(&outside, repo.root.join("receipt-jump")).expect("plant receipt symlink");
            repo.config.paths.receipts = "receipt-jump/new-receipts".to_owned();
            let prepared = prepare(
                &repo,
                &definition,
                AdmissionRequest {
                    binding_path: &binding_path,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    started_at: "2026-08-26T00:00:00Z",
                },
            )
            .expect("admit receipt context before publication");

            mint(
                &repo,
                &definition,
                &run,
                prepared,
                MintRequest {
                    started_at: "2026-08-26T00:00:00Z",
                    exit_result: GateExitResult::ExitCode { code: 0 },
                },
                b"stdout\n",
                b"stderr\n",
            )
            .expect_err("receipt directory symlink must fail closed");

            assert!(
                !outside.join("new-receipts").exists(),
                "receipt publication created a directory outside repository"
            );
            let _ = std::fs::remove_dir_all(outside);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::atomic::{AtomicU64, Ordering};

    use camino::{Utf8Path, Utf8PathBuf};
    use openwarrant_compiler::{DigestDomain, lower, sha256_digest};
    use openwarrant_core::authority::{ActorRole, AuthorityRegister, RoleAssignment};
    use openwarrant_core::contract::{Authorization, ContractRevision, Independence};
    use openwarrant_core::gate::{
        DetectionResult, EvidencePolicy, Fixture, GateLifecycle, GateProvenance, GateRef,
        Qualification,
    };
    use openwarrant_core::{ActorKind, GateBinding, Namespace, RepositoryConfig};

    use super::*;

    static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);

    struct Scratch {
        root: Utf8PathBuf,
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn unrecorded_gate_fixture() -> (Scratch, Repository) {
        let mut root =
            Utf8PathBuf::from_path_buf(std::env::temp_dir()).expect("temporary directory is UTF-8");
        root.push(format!(
            "openwarrant-unrecorded-gate-{}-{}",
            std::process::id(),
            NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let config = RepositoryConfig::new(
            "Unrecorded Gate fixture",
            Namespace::parse("OW").expect("fixture namespace"),
        );
        let repo = Repository {
            root: root.clone(),
            config,
        };
        let gate_dir = repo.root.join(&repo.config.paths.gates);
        std::fs::create_dir_all(&gate_dir).expect("gate directory");
        let authority_dir = repo.root.join("docs/authority");
        std::fs::create_dir_all(&authority_dir).expect("authority directory");
        std::fs::write(
            authority_dir.join("roles.toml"),
            r#"[[assignment]]
actor = "gate_runner"
actor_kind = "agent"
roles = ["verifier"]
assigned_by = "fixture-authorizer"
effective_time = "2026-01-01T00:00:00Z"
"#,
        )
        .expect("write verifier authority fixture");
        std::fs::write(
            gate_dir.join("fixture.pass@1.0.0.yaml"),
            r#"gate_id: "fixture.pass"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "fixture://rustc-version"
output_schema_ref: "schema://fixture/v1"
provenance: "local_candidate"
input_kinds: ["fixture"]
argv: ["rustc", "--version"]
selection_manifest: ["test://rustc-version"]
mutating: "false"
fault_model: ["unavailable-rustc"]
known_blind_spots: ["fixture only"]
qualification_qualifier: "unit fixture"
qualification_digest: ""
qualification_positive_controls: ["rustc --version exits zero"]
qualification_negative_controls: ["missing rustc is not askable"]
qualification_mutation_classes: ["program removal"]
qualification_environments: ["pinned Rust toolchain"]
qualification_limitations: ["exercises unrecorded execution only"]
detection_results:
  - fault_class: "unavailable-rustc"
    mutation: "remove rustc from PATH"
    detected: "true"
"#,
        )
        .expect("write Gate Definition");
        (Scratch { root }, repo)
    }

    fn qualified_gate(argv: Vec<String>, timeout_secs: u64) -> GateDefinition {
        GateDefinition {
            gate_id: "fixture.process-tree".to_owned(),
            version: "1.0.0".to_owned(),
            digest: format!("sha256:{}", "a".repeat(64)),
            lifecycle: GateLifecycle::Qualified,
            implementation_ref: "fixture://process-tree".to_owned(),
            input_kinds: vec!["fixture".to_owned()],
            output_schema_ref: "schema://fixture/v1".to_owned(),
            fault_model: vec!["retained-output-handle".to_owned()],
            known_blind_spots: vec!["Unix-only process fixture".to_owned()],
            qualification: Some(Qualification {
                positive_controls: vec!["descendant retains output handle".to_owned()],
                negative_controls: vec!["single process exits normally".to_owned()],
                mutation_classes: vec!["descendant process".to_owned()],
                environments: vec!["Unix test host".to_owned()],
                detection_results: vec![DetectionResult {
                    fault_class: "retained-output-handle".to_owned(),
                    mutation: "spawn descendant before parent exits".to_owned(),
                    detected: true,
                }],
                limitations: vec!["test-only command".to_owned()],
                qualifier: "unit fixture".to_owned(),
                qualification_digest: String::new(),
            }),
            provenance: GateProvenance::LocalCandidate,
            argv,
            selection_manifest: vec!["test://process-tree".to_owned()],
            mutating: false,
            timeout_secs: Some(timeout_secs),
        }
    }

    #[test]
    fn empty_definition_selection_is_zero_selected_tests() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let mut definition = qualified_gate(vec!["rustc".to_owned(), "--version".to_owned()], 5);
        definition.selection_manifest.clear();
        assert_eq!(
            askability_of(&definition, &repo),
            Some(ReasonCode::ZeroSelectedTests)
        );
    }

    #[test]
    fn recording_without_execution_is_refused() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let error = run(
            &repo,
            RunRequest {
                execute: false,
                only: Some("fixture.pass"),
                record: true,
                subject_digests: &[],
                raw_evidence_refs: &[],
                binding_path: Some(Utf8Path::new("docs/receipts/input.binding.json")),
                warrant_alias: Some("OW-WAR-0014"),
            },
        )
        .expect_err("recording without execution must fail before registry work");
        assert!(error.to_string().contains("--record requires --run"));
    }

    #[test]
    fn unknown_selected_gate_is_a_blocking_result() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let report = run(
            &repo,
            RunRequest {
                execute: true,
                only: Some("does.not.exist"),
                record: false,
                subject_digests: &[],
                raw_evidence_refs: &[],
                binding_path: None,
                warrant_alias: None,
            },
        )
        .expect("unknown selection is a bounded diagnostic");
        assert!(!report.is_ready());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "gate-run.not-found")
        );
    }

    fn copy_tree(from: &Utf8Path, to: &Utf8Path) {
        std::fs::create_dir_all(to).expect("create fixture directory");
        for entry in std::fs::read_dir(from).expect("read fixture directory") {
            let entry = entry.expect("fixture entry");
            let source = Utf8PathBuf::from_path_buf(entry.path()).expect("fixture path is UTF-8");
            let target = to.join(entry.file_name().to_str().expect("fixture name is UTF-8"));
            if entry.file_type().expect("fixture type").is_dir() {
                copy_tree(&source, &target);
            } else {
                std::fs::copy(source, target).expect("copy fixture file");
            }
        }
    }

    fn ensure_authorization_fixture(repo: &Repository) -> (&'static str, String) {
        const ALIAS: &str = "OW-WAR-0014";
        let warrant_dir = repo.root.join(&repo.config.paths.warrants).join(ALIAS);
        if !warrant_dir.join("manifest.toml").exists() {
            let workspace = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize_utf8()
                .expect("workspace root");
            copy_tree(&workspace.join("docs/warrants").join(ALIAS), &warrant_dir);
        }
        let loaded = repo
            .load_warrant(&warrant_dir)
            .expect("load fixture Warrant");
        assert!(loaded.report.is_ready(), "fixture Warrant must compile");
        let basis = loaded.basis.as_ref().expect("fixture compilation basis");
        let validated = loaded.validated.as_ref().expect("fixture manifest");
        let digest = lower(basis, validated)
            .expect("lower fixture Warrant")
            .contract_digest()
            .expect("digest fixture Warrant");
        (ALIAS, digest)
    }

    fn install_binding_authorization(
        repo: &Repository,
        alias: &str,
        contract_digest: &str,
        binding_path: &Utf8Path,
        binding: &GateBinding,
    ) {
        let warrant_dir = repo.root.join(&repo.config.paths.warrants).join(alias);
        let loaded = repo
            .load_warrant(&warrant_dir)
            .expect("load fixture Warrant");
        let basis = loaded.basis.as_ref().expect("fixture compilation basis");
        let validated = loaded.validated.as_ref().expect("fixture manifest");
        let ir = lower(basis, validated).expect("lower fixture Warrant");
        assert_eq!(
            ir.contract_digest().expect("digest fixture Warrant"),
            contract_digest
        );
        let binding_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateBinding, binding).expect("digest fixture Gate Binding")
        );
        let record = crate::authorize::AuthorizationRecord {
            schema: crate::authorize::AUTHORIZATION_SCHEMA.to_owned(),
            warrant: alias.to_owned(),
            revision: ContractRevision::draft(contract_digest.to_owned(), ir.contract_coverage)
                .propose(repo.performer())
                .expect("propose fixture revision")
                .authorize(Authorization {
                    authorizer: "fixture-human".to_owned(),
                    actor_kind: ActorKind::Human,
                    acting_role: ActorRole::Authorizer.to_string(),
                    meaning: "Authorize exact fixture Gate Binding.".to_owned(),
                    effective_time: "2026-08-26T00:00:00Z".to_owned(),
                    policy_basis: None,
                    independence: Independence::SeparateRole,
                })
                .expect("authorize fixture revision"),
            gate_bindings: Some(vec![crate::authorize::AuthorizedGateBinding {
                id: binding.id.clone(),
                gate: binding.gate.key(),
                digest: binding_digest,
                source_ref: repo.relative(binding_path),
            }]),
        };
        std::fs::write(
            warrant_dir.join("authorization.toml"),
            toml::to_string_pretty(&record).expect("serialize fixture authorization"),
        )
        .expect("write fixture authorization");
    }

    fn matching_binding(repo: &Repository) -> Utf8PathBuf {
        let (alias, contract_digest) = ensure_authorization_fixture(repo);
        let mut report = Report::default();
        let registry = crate::check::load_gate_registry(repo, &mut report);
        let definition = registry
            .definitions
            .iter()
            .find(|definition| definition.gate_id == "fixture.pass")
            .expect("qualified fixture Gate Definition");
        let binding = GateBinding {
            id: "GB-fixture-pass".to_owned(),
            gate: GateRef {
                id: definition.gate_id.clone(),
                version: definition.version.clone(),
                digest: definition.digest.clone(),
            },
            subjects: vec![format!(
                "contract:sha256:{}",
                contract_digest
                    .strip_prefix("sha256:")
                    .unwrap_or(&contract_digest)
            )],
            fixtures: vec![],
            parameters: BTreeMap::new(),
            pass_predicate: BTreeMap::new(),
            evidence_policy: EvidencePolicy {
                producer: "gate_runner".to_owned(),
                performer_authored_report_admissible: false,
            },
        };
        let binding_dir = repo
            .root
            .join(&repo.config.paths.warrants)
            .join(alias)
            .join("gate-bindings");
        std::fs::create_dir_all(&binding_dir).expect("Gate Binding candidate directory");
        let path = binding_dir.join("GB-fixture-pass.binding.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&binding).expect("serialize Gate Binding"),
        )
        .expect("write Gate Binding");
        install_binding_authorization(repo, alias, &contract_digest, &path, &binding);
        path
    }

    fn verifier_register(performer: &str) -> AuthorityRegister {
        verifier_registers(&[performer])
    }

    fn verifier_registers(actors: &[&str]) -> AuthorityRegister {
        AuthorityRegister::new(
            actors
                .iter()
                .map(|actor| RoleAssignment {
                    actor: (*actor).to_owned(),
                    actor_kind: ActorKind::Agent,
                    roles: [ActorRole::Verifier].into_iter().collect(),
                    assigned_by: "fixture-authorizer".to_owned(),
                    effective_time: "2026-01-01T00:00:00Z".to_owned(),
                    note: None,
                })
                .collect(),
        )
    }

    fn passing_bonsai_evidence(contract_digest: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schema": "oh.war/bonsai-evidence/v1",
            "warrant": {
                "alias": "OW-WAR-0050",
                "contract_digest": contract_digest,
                "scope_source": "docs/warrants/OW-WAR-0050/scope.toml",
                "scope_source_digest": format!("sha256:{}", "b".repeat(64))
            },
            "git": {
                "repository": "github:Quitetall/OpenWarrant",
                "base": "c".repeat(40),
                "head": "d".repeat(40),
                "tree": "e".repeat(40)
            },
            "policy": {
                "path": "bonsai.toml",
                "digest": format!("sha256:{}", "f".repeat(64))
            },
            "changed_paths": ["crates/openwarrant-cli/src/bonsai.rs"],
            "scope_findings": [],
            "bonsai": {
                "executable": "target/release/bonsai",
                "binary_digest": format!("sha256:{}", "1".repeat(64)),
                "expected_source": "github:Quitetall/bonsai",
                "expected_revision": "2".repeat(40),
                "version": "bonsai fixture",
                "exit_code": 0,
                "raw_output": {"tool": "bonsai", "version": "fixture", "findings": []},
                "stderr": "",
                "spawn_error": null
            },
            "architecture_findings": [],
            "advisory_findings": [],
            "verdict": "pass"
        }))
        .expect("serialize passing Bonsai evidence")
    }

    #[test]
    fn bonsai_receipt_refuses_evidence_path_not_consumed_by_gate_definition() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let contract_digest = "a".repeat(64);
        let evidence = passing_bonsai_evidence(&contract_digest);
        let path = repo.root.join("substitute-bonsai-evidence.json");
        std::fs::write(&path, &evidence).expect("write substitute Bonsai evidence");
        let evidence_ref = format!(
            "file:substitute-bonsai-evidence.json#sha256:{}",
            sha256_hex(&evidence)
        );
        let mut definition = qualified_gate(
            vec![
                "./target/debug/war".to_owned(),
                "bonsai".to_owned(),
                "verify-evidence".to_owned(),
                "--evidence".to_owned(),
                "bonsai-evidence.json".to_owned(),
            ],
            30,
        );
        definition.gate_id = BONSAI_EVIDENCE_GATE.to_owned();

        let error = validate_bonsai_bindings(
            &repo,
            &definition,
            &[format!("contract:sha256:{contract_digest}")],
            &[evidence_ref],
        )
        .expect_err("receipt must make the verifier consume immutable stdin");

        assert!(
            error
                .to_string()
                .contains("must consume exactly one immutable --evidence - input"),
            "unexpected refusal: {error}"
        );
    }

    #[test]
    fn bonsai_receipt_owns_exact_evidence_bytes_consumed_by_gate_definition() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let contract_digest = "a".repeat(64);
        let evidence = passing_bonsai_evidence(&contract_digest);
        std::fs::write(repo.root.join("bonsai-evidence.json"), &evidence)
            .expect("write consumed Bonsai evidence");
        let evidence_ref = format!("file:bonsai-evidence.json#sha256:{}", sha256_hex(&evidence));
        let mut definition = qualified_gate(
            vec![
                "./target/debug/war".to_owned(),
                "bonsai".to_owned(),
                "verify-evidence".to_owned(),
                "--evidence".to_owned(),
                "-".to_owned(),
            ],
            30,
        );
        definition.gate_id = BONSAI_EVIDENCE_GATE.to_owned();

        let exact_input = validate_bonsai_bindings(
            &repo,
            &definition,
            &[format!("contract:sha256:{contract_digest}")],
            &[evidence_ref],
        )
        .expect("receipt binds exact Bonsai evidence consumed by Gate Definition")
        .expect("Bonsai gate owns input bytes");
        assert_eq!(exact_input, evidence);
    }

    #[cfg(unix)]
    #[test]
    fn bonsai_receipt_refuses_symlinked_evidence_before_execution() {
        use std::os::unix::fs::symlink;

        let (_scratch, repo) = unrecorded_gate_fixture();
        let contract_digest = "a".repeat(64);
        let evidence = passing_bonsai_evidence(&contract_digest);
        let outside = repo.root.with_file_name(format!(
            "openwarrant-bonsai-outside-{}-{}",
            std::process::id(),
            NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&outside, &evidence).expect("write outside evidence");
        symlink(&outside, repo.root.join("bonsai-evidence.json")).expect("symlink evidence");
        let evidence_ref = format!("file:bonsai-evidence.json#sha256:{}", sha256_hex(&evidence));
        let mut definition = qualified_gate(
            vec![
                "./target/debug/war".to_owned(),
                "bonsai".to_owned(),
                "verify-evidence".to_owned(),
                "--evidence".to_owned(),
                "-".to_owned(),
            ],
            30,
        );
        definition.gate_id = BONSAI_EVIDENCE_GATE.to_owned();

        let error = validate_bonsai_bindings(
            &repo,
            &definition,
            &[format!("contract:sha256:{contract_digest}")],
            &[evidence_ref],
        )
        .expect_err("repository evidence cannot resolve through an outside symlink");

        assert!(
            error.to_string().contains("contains symlink component"),
            "unexpected refusal: {error}"
        );
        let _ = std::fs::remove_file(outside);
    }

    #[test]
    fn bonsai_receipt_refuses_oversized_evidence_before_execution() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let contract_digest = "a".repeat(64);
        let path = repo.root.join("bonsai-evidence.json");
        std::fs::File::create(&path)
            .expect("create oversized evidence")
            .set_len(crate::bonsai::MAX_BONSAI_EVIDENCE_BYTES + 1)
            .expect("size oversized evidence");
        let evidence_ref = format!("file:bonsai-evidence.json#sha256:{}", "b".repeat(64));
        let mut definition = qualified_gate(
            vec![
                "./target/debug/war".to_owned(),
                "bonsai".to_owned(),
                "verify-evidence".to_owned(),
                "--evidence".to_owned(),
                "-".to_owned(),
            ],
            30,
        );
        definition.gate_id = BONSAI_EVIDENCE_GATE.to_owned();

        let error = validate_bonsai_bindings(
            &repo,
            &definition,
            &[format!("contract:sha256:{contract_digest}")],
            &[evidence_ref],
        )
        .expect_err("oversized evidence must fail before allocation or execution");

        assert!(
            error.to_string().contains("exceed") && error.to_string().contains("byte limit"),
            "unexpected refusal: {error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn bonsai_execution_consumes_admitted_bytes_after_source_path_is_replaced() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let contract_digest = "a".repeat(64);
        let admitted = passing_bonsai_evidence(&contract_digest);
        let replacement = passing_bonsai_evidence(&"b".repeat(64));
        let source = repo.root.join("bonsai-evidence.json");
        std::fs::write(&source, &admitted).expect("write admitted evidence");
        let evidence_ref = format!("file:bonsai-evidence.json#sha256:{}", sha256_hex(&admitted));
        let captured = repo.root.join("captured-evidence.json");
        let mut definition = qualified_gate(
            vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                "cat > \"$1\"".to_owned(),
                "openwarrant-bonsai-fixture".to_owned(),
                captured.to_string(),
                "--evidence".to_owned(),
                "-".to_owned(),
            ],
            30,
        );
        definition.gate_id = BONSAI_EVIDENCE_GATE.to_owned();
        let exact_input = validate_bonsai_bindings(
            &repo,
            &definition,
            &[format!("contract:sha256:{contract_digest}")],
            &[evidence_ref],
        )
        .expect("admit evidence")
        .expect("Bonsai input");

        std::fs::write(&source, &replacement).expect("replace source after admission");
        let execution = execute_gate(
            &definition,
            &repo,
            "GR-swap-plant".to_owned(),
            Some(exact_input),
        );

        assert_eq!(execution.run.verdict, Verdict::Pass);
        assert_eq!(
            std::fs::read(captured).expect("captured verifier input"),
            admitted
        );
    }

    #[test]
    fn an_unrecorded_completed_gate_needs_no_evidence_binding() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let report = run(
            &repo,
            RunRequest {
                execute: true,
                only: Some("fixture.pass"),
                record: false,
                subject_digests: &[],
                raw_evidence_refs: &[],
                binding_path: None,
                warrant_alias: None,
            },
        )
        .expect("an unrecorded run does not mint evidence");

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "gate-run.pass")
        );
    }

    #[test]
    fn a_failed_receipt_admission_executes_nothing_and_leaves_no_passing_run() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let receipt_dir = repo.root.join(&repo.config.paths.receipts);
        std::fs::create_dir_all(&receipt_dir).expect("receipt directory");
        let invalid_binding = receipt_dir.join("invalid.binding.json");
        std::fs::write(&invalid_binding, b"{}\n").expect("invalid Gate Binding fixture");

        let report = run(
            &repo,
            RunRequest {
                execute: true,
                only: Some("fixture.pass"),
                record: true,
                subject_digests: &[],
                raw_evidence_refs: &[],
                binding_path: Some(&invalid_binding),
                warrant_alias: Some("OW-WAR-0014"),
            },
        )
        .expect("receipt admission failure is reported, not promoted");

        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "gate-run.receipt-admission-failed")
        );
        assert!(
            !receipt_dir.join("fixture_pass_1_0_0.stdout.txt").exists(),
            "receipt admission must happen before executor invocation"
        );
        assert!(
            std::fs::read_dir(&receipt_dir)
                .expect("receipt directory")
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().ends_with(".run.toml")),
            "a run without its mandatory receipt must not remain admissible"
        );
    }

    #[test]
    fn unsupported_binding_semantics_execute_nothing() {
        for field in ["parameters", "pass_predicate", "fixtures"] {
            let (_scratch, repo) = unrecorded_gate_fixture();
            let binding_path = matching_binding(&repo);
            let mut binding: GateBinding = openwarrant_core::legacy_disposition::parse_strict_json(
                &std::fs::read(&binding_path).expect("Gate Binding bytes"),
            )
            .expect("Gate Binding");
            match field {
                "parameters" => {
                    binding
                        .parameters
                        .insert("mode".to_owned(), "strict".to_owned());
                }
                "pass_predicate" => {
                    binding
                        .pass_predicate
                        .insert("byte_equal".to_owned(), "true".to_owned());
                }
                "fixtures" => binding.fixtures.push(Fixture {
                    reference: "tests/fixtures/input.bin".to_owned(),
                    digest: format!("sha256:{}", "c".repeat(64)),
                }),
                _ => unreachable!("fixed binding-semantics plant"),
            }
            std::fs::write(
                &binding_path,
                serde_json::to_vec(&binding).expect("serialize Gate Binding plant"),
            )
            .expect("write Gate Binding plant");

            let report = run(
                &repo,
                RunRequest {
                    execute: true,
                    only: Some("fixture.pass"),
                    record: true,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    binding_path: Some(&binding_path),
                    warrant_alias: Some("OW-WAR-0014"),
                },
            )
            .expect("unsupported binding semantics are reported");

            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.rule == "gate-run.receipt-admission-failed"
                        && diagnostic.message.contains("execution adapter")
                }),
                "{field} must fail before execution: {:?}",
                report.diagnostics
            );
            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            if receipt_dir.exists() {
                assert!(
                    std::fs::read_dir(&receipt_dir)
                        .expect("receipt directory")
                        .filter_map(Result::ok)
                        .all(|entry| !entry.file_name().to_string_lossy().ends_with(".run.toml")),
                    "{field} reached execution despite unsupported semantics"
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn decorative_raw_evidence_executes_nothing_without_an_adapter() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let gate_path = repo
            .root
            .join(&repo.config.paths.gates)
            .join("fixture.pass@1.0.0.yaml");
        std::fs::write(
            &gate_path,
            r#"gate_id: "fixture.pass"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "fixture://decorative-evidence-preflight"
output_schema_ref: "schema://fixture/v1"
provenance: "local_candidate"
input_kinds: ["fixture"]
argv: ["/bin/sh", "-c", "printf ran > execution-marker"]
selection_manifest: ["test://decorative-evidence-preflight"]
mutating: "false"
fault_model: ["decorative-evidence-reference"]
known_blind_spots: ["unit fixture"]
qualification_qualifier: "unit fixture"
qualification_digest: ""
qualification_positive_controls: ["no decorative evidence executes"]
qualification_negative_controls: ["unconsumed evidence refuses before execution"]
qualification_mutation_classes: ["decorative raw evidence reference"]
qualification_environments: ["unix test host"]
qualification_limitations: ["test-only command"]
detection_results:
  - fault_class: "decorative-evidence-reference"
    mutation: "attach a raw reference the executor never consumes"
    detected: "true"
"#,
        )
        .expect("write side-effect Gate Definition");
        let binding_path = matching_binding(&repo);

        let report = run(
            &repo,
            RunRequest {
                execute: true,
                only: Some("fixture.pass"),
                record: true,
                subject_digests: &[],
                raw_evidence_refs: &["artifact://DEL-1".to_owned()],
                binding_path: Some(&binding_path),
                warrant_alias: Some("OW-WAR-0014"),
            },
        )
        .expect("decorative evidence is a bounded admission diagnostic");

        assert!(!repo.root.join("execution-marker").exists());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule == "gate-run.receipt-admission-failed"
                && diagnostic
                    .message
                    .contains("cannot prove raw evidence consumption")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn duplicate_raw_evidence_refs_execute_nothing() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let gate_path = repo
            .root
            .join(&repo.config.paths.gates)
            .join("fixture.pass@1.0.0.yaml");
        std::fs::write(
            &gate_path,
            r#"gate_id: "fixture.pass"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "fixture://duplicate-evidence-preflight"
output_schema_ref: "schema://fixture/v1"
provenance: "local_candidate"
input_kinds: ["fixture"]
argv: ["/bin/sh", "-c", "printf ran > execution-marker"]
selection_manifest: ["test://duplicate-evidence-preflight"]
mutating: "false"
fault_model: ["duplicate-evidence-reference"]
known_blind_spots: ["unit fixture"]
qualification_qualifier: "unit fixture"
qualification_digest: ""
qualification_positive_controls: ["unique evidence refs execute"]
qualification_negative_controls: ["duplicate evidence refs refuse before execution"]
qualification_mutation_classes: ["duplicate raw evidence reference"]
qualification_environments: ["unix test host"]
qualification_limitations: ["test-only command"]
detection_results:
  - fault_class: "duplicate-evidence-reference"
    mutation: "repeat one raw evidence reference"
    detected: "true"
"#,
        )
        .expect("write side-effect Gate Definition");
        let binding_path = matching_binding(&repo);
        let duplicate = "artifact://DEL-1".to_owned();

        let report = run(
            &repo,
            RunRequest {
                execute: true,
                only: Some("fixture.pass"),
                record: true,
                subject_digests: &[],
                raw_evidence_refs: &[duplicate.clone(), duplicate],
                binding_path: Some(&binding_path),
                warrant_alias: Some("OW-WAR-0014"),
            },
        )
        .expect("duplicate inventory is a bounded admission diagnostic");

        assert!(!repo.root.join("execution-marker").exists());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule == "gate-run.receipt-admission-failed"
                && diagnostic.message.contains("raw evidence")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn recording_without_a_known_execution_adapter_executes_nothing() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let gate_path = repo
            .root
            .join(&repo.config.paths.gates)
            .join("fixture.pass@1.0.0.yaml");
        std::fs::write(
            &gate_path,
            r#"gate_id: "fixture.pass"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "fixture://unknown-recording-adapter"
output_schema_ref: "schema://fixture/v1"
provenance: "local_candidate"
input_kinds: ["fixture"]
argv: ["/bin/sh", "-c", "printf ran > execution-marker"]
selection_manifest: ["test://unobserved-selection"]
mutating: "false"
fault_model: ["unobserved-test-selection"]
known_blind_spots: ["fixture only"]
qualification_qualifier: "unit fixture"
qualification_digest: ""
qualification_positive_controls: ["known adapter observes selection"]
qualification_negative_controls: ["unknown adapter refuses before spawn"]
qualification_mutation_classes: ["adapter substitution"]
qualification_environments: ["unix test host"]
qualification_limitations: ["test-only command"]
detection_results:
  - fault_class: "unobserved-test-selection"
    mutation: "replace the known adapter identity"
    detected: "true"
"#,
        )
        .expect("write unknown-adapter Gate Definition");
        let binding_path = matching_binding(&repo);

        let report = run(
            &repo,
            RunRequest {
                execute: true,
                only: Some("fixture.pass"),
                record: true,
                subject_digests: &[],
                raw_evidence_refs: &[],
                binding_path: Some(&binding_path),
                warrant_alias: Some("OW-WAR-0014"),
            },
        )
        .expect("unknown adapter is a bounded admission refusal");

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule == "gate-run.receipt-admission-failed"
                && diagnostic.message.contains("execution adapter")
        }));
        assert!(
            !repo.root.join("execution-marker").exists(),
            "recording without an observation-capable adapter must refuse before spawn"
        );
    }

    #[cfg(unix)]
    #[test]
    fn binding_mutated_after_authorization_is_refused_before_spawn() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let gate_path = repo
            .root
            .join(&repo.config.paths.gates)
            .join("fixture.pass@1.0.0.yaml");
        std::fs::write(
            &gate_path,
            r#"gate_id: "fixture.pass"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "fixture://binding-authority-preflight"
output_schema_ref: "schema://fixture/v1"
provenance: "local_candidate"
input_kinds: ["fixture"]
argv: ["/bin/sh", "-c", "printf ran > execution-marker"]
selection_manifest: ["test://binding-authority-preflight"]
mutating: "false"
fault_model: ["binding-substitution"]
known_blind_spots: ["unit fixture"]
qualification_qualifier: "unit fixture"
qualification_digest: ""
qualification_positive_controls: ["authorized exact Binding executes"]
qualification_negative_controls: ["post-authorization Binding mutation refuses"]
qualification_mutation_classes: ["Binding id and digest substitution"]
qualification_environments: ["unix test host"]
qualification_limitations: ["test-only command"]
detection_results:
  - fault_class: "binding-substitution"
    mutation: "replace authorized Binding before execution"
    detected: "true"
"#,
        )
        .expect("write side-effect Gate Definition");
        let binding_path = matching_binding(&repo);
        let mut substituted: GateBinding = openwarrant_core::legacy_disposition::parse_strict_json(
            &std::fs::read(&binding_path).expect("authorized Binding bytes"),
        )
        .expect("authorized Binding");
        substituted.id = "GB-attacker-substitute".to_owned();
        std::fs::write(
            &binding_path,
            serde_json::to_vec(&substituted).expect("serialize substituted Binding"),
        )
        .expect("replace Binding candidate");

        let report = run(
            &repo,
            RunRequest {
                execute: true,
                only: Some("fixture.pass"),
                record: true,
                subject_digests: &[],
                raw_evidence_refs: &[],
                binding_path: Some(&binding_path),
                warrant_alias: Some("OW-WAR-0014"),
            },
        )
        .expect("unauthorized Binding is a bounded diagnostic");

        assert!(!repo.root.join("execution-marker").exists());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule == "gate-run.binding-not-authorized"
                && diagnostic.message.contains("not selected")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn bonsai_record_is_admissible_to_live_resolution_with_unrelated_artifacts() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let gate_dir = repo.root.join(&repo.config.paths.gates);
        std::fs::write(
            gate_dir.join("software.repo.bonsai-evidence@1.0.0.yaml"),
            r#"gate_id: "software.repo.bonsai-evidence"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "fixture://bonsai-stdin-adapter"
output_schema_ref: "schema://bonsai-evidence/v1"
provenance: "local_candidate"
input_kinds: ["bonsai-evidence"]
argv: ["/bin/sh", "-c", "cat >/dev/null", "bonsai-fixture", "--evidence", "-"]
selection_manifest: ["check://bonsai-evidence-document"]
mutating: "false"
timeout_secs: "30"
fault_model: ["producer-resolver-inventory-mismatch"]
known_blind_spots: ["unit fixture"]
qualification_qualifier: "unit fixture"
qualification_digest: ""
qualification_positive_controls: ["recorded exact stdin is resolver-admissible"]
qualification_negative_controls: ["foreign contract or altered evidence rejects"]
qualification_mutation_classes: ["contract and evidence digest substitution"]
qualification_environments: ["unix-like test host"]
qualification_limitations: ["test-only verifier command"]
detection_results:
  - fault_class: "producer-resolver-inventory-mismatch"
    mutation: "record contract-only gate while unrelated artifacts exist"
    detected: "true"
"#,
        )
        .expect("write Bonsai Gate Definition");
        let mut report = Report::default();
        let registry = crate::check::load_gate_registry(&repo, &mut report);
        let definition = registry
            .definitions
            .iter()
            .find(|definition| definition.gate_id == BONSAI_EVIDENCE_GATE)
            .expect("qualified Bonsai Gate Definition");
        let (warrant_alias, current_contract_digest) = ensure_authorization_fixture(&repo);
        let contract_digest = current_contract_digest
            .strip_prefix("sha256:")
            .unwrap_or(&current_contract_digest)
            .to_owned();
        let contract_subject = format!("contract:sha256:{contract_digest}");
        let binding = GateBinding {
            id: "GB-bonsai-record-resolve".to_owned(),
            gate: GateRef {
                id: definition.gate_id.clone(),
                version: definition.version.clone(),
                digest: definition.digest.clone(),
            },
            subjects: vec![contract_subject.clone()],
            fixtures: vec![],
            parameters: BTreeMap::new(),
            pass_predicate: BTreeMap::new(),
            evidence_policy: EvidencePolicy {
                producer: "gate_runner".to_owned(),
                performer_authored_report_admissible: false,
            },
        };
        let binding_dir = repo
            .root
            .join(&repo.config.paths.warrants)
            .join(warrant_alias)
            .join("gate-bindings");
        std::fs::create_dir_all(&binding_dir).expect("create Gate Binding directory");
        let binding_path = binding_dir.join("GB-bonsai-record-resolve.binding.json");
        std::fs::write(
            &binding_path,
            serde_json::to_vec(&binding).expect("serialize Bonsai binding"),
        )
        .expect("write Bonsai binding");
        install_binding_authorization(
            &repo,
            warrant_alias,
            &current_contract_digest,
            &binding_path,
            &binding,
        );
        let evidence = passing_bonsai_evidence(&contract_digest);
        std::fs::write(repo.root.join("bonsai-evidence.json"), &evidence)
            .expect("write Bonsai evidence");
        let evidence_ref = format!("file:bonsai-evidence.json#sha256:{}", sha256_hex(&evidence));

        let report = run(
            &repo,
            RunRequest {
                execute: true,
                only: Some(BONSAI_EVIDENCE_GATE),
                record: true,
                subject_digests: std::slice::from_ref(&contract_subject),
                raw_evidence_refs: std::slice::from_ref(&evidence_ref),
                binding_path: Some(&binding_path),
                warrant_alias: Some(warrant_alias),
            },
        )
        .expect("record Bonsai receipt bundle");
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule == "gate-run.receipt" || diagnostic.rule == "gate-run.pass"
        }));

        std::fs::write(repo.root.join("artifact-a.bin"), b"artifact-a").expect("write artifact A");
        std::fs::write(repo.root.join("artifact-b.bin"), b"artifact-b").expect("write artifact B");
        let basis = crate::gate_evidence::ResolutionEvidenceBasis::new(
            &contract_digest,
            [
                crate::gate_evidence::ResolutionEvidenceArtifact {
                    deliverable_id: "DEL-A".to_owned(),
                    target_ref: "artifact-a.bin".to_owned(),
                    observed_sha256: format!("sha256:{}", sha256_hex(b"artifact-a")),
                },
                crate::gate_evidence::ResolutionEvidenceArtifact {
                    deliverable_id: "DEL-B".to_owned(),
                    target_ref: "artifact-b.bin".to_owned(),
                    observed_sha256: format!("sha256:{}", sha256_hex(b"artifact-b")),
                },
            ],
            [crate::authorize::AuthorizedGateBinding {
                id: binding.id.clone(),
                gate: binding.gate.key(),
                digest: format!(
                    "sha256:{}",
                    openwarrant_compiler::sha256_digest(
                        openwarrant_compiler::DigestDomain::GateBinding,
                        &binding,
                    )
                    .expect("digest authorized Bonsai binding")
                ),
                source_ref: repo.relative(&binding_path),
            }],
        )
        .expect("Resolution basis");
        let admitted = crate::gate_evidence::load_admissible_for_resolution(
            &repo,
            &basis,
            &verifier_register("gate_runner"),
        );

        assert!(admitted.failures.is_empty(), "{:?}", admitted.failures);
        assert_eq!(admitted.runs.len(), 1);
        assert!(crate::resolve::every_required_gate_has_admissible_result(
            &[format!("gate://{BONSAI_EVIDENCE_GATE}@1.0.0")],
            &admitted.runs,
        ));

        let unauthorized_basis = crate::gate_evidence::ResolutionEvidenceBasis::new(
            &contract_digest,
            [],
            [crate::authorize::AuthorizedGateBinding {
                id: "GB-human-selected-other-binding".to_owned(),
                gate: binding.gate.key(),
                digest: format!("sha256:{}", "f".repeat(64)),
                source_ref: format!(
                    "docs/warrants/{warrant_alias}/gate-bindings/other.binding.json"
                ),
            }],
        )
        .expect("canonical unauthorized comparison basis");
        let rejected = crate::gate_evidence::load_admissible_for_resolution(
            &repo,
            &unauthorized_basis,
            &verifier_register("gate_runner"),
        );
        assert!(rejected.runs.is_empty());
        assert!(rejected.failures.iter().any(|(_, error)| {
            error.contains("not authorized for the current Contract Revision")
        }));
    }

    #[test]
    fn recorded_gate_runs_preserve_two_immutable_receipt_bundles() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let binding_path = matching_binding(&repo);

        for _ in 0..2 {
            let report = run(
                &repo,
                RunRequest {
                    execute: true,
                    only: Some("fixture.pass"),
                    record: true,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    binding_path: Some(&binding_path),
                    warrant_alias: Some("OW-WAR-0014"),
                },
            )
            .expect("record immutable Gate evidence bundle");
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.rule == "gate-run.receipt")
            );
        }

        let receipt_dir = repo.root.join(&repo.config.paths.receipts);
        let mut receipt_ids = std::collections::BTreeSet::new();
        for entry in std::fs::read_dir(&receipt_dir).expect("receipt directory") {
            let path = Utf8PathBuf::from_path_buf(entry.expect("directory entry").path())
                .expect("UTF-8 receipt path");
            if !path.as_str().ends_with(".receipt.json") {
                continue;
            }
            let receipt: openwarrant_core::GateReceipt =
                openwarrant_core::legacy_disposition::parse_strict_json(
                    &std::fs::read(&path).expect("receipt bytes"),
                )
                .expect("strict Gate receipt");
            assert!(run_record_path(&receipt.run_id, &repo).is_file());
            assert!(
                receipt_dir
                    .join(format!("{}.binding.json", receipt.run_id))
                    .is_file()
            );
            assert!(
                receipt_dir
                    .join(format!("{}.stdout.txt", receipt.run_id))
                    .is_file()
            );
            assert!(
                receipt_dir
                    .join(format!("{}.stderr.txt", receipt.run_id))
                    .is_file()
            );
            receipt_ids.insert(receipt.run_id);
        }
        assert_eq!(receipt_ids.len(), 2, "each execution keeps its own receipt");

        let input_binding: GateBinding = openwarrant_core::legacy_disposition::parse_strict_json(
            &std::fs::read(&binding_path).expect("read authorized input Binding"),
        )
        .expect("parse authorized input Binding");
        let expected_subjects = input_binding.subjects.into_iter().collect();
        let evidence = crate::gate_evidence::load_admissible(
            &repo,
            &expected_subjects,
            &BTreeSet::new(),
            &verifier_register("gate_runner"),
        );
        assert!(evidence.failures.is_empty(), "{:?}", evidence.failures);
        assert_eq!(evidence.runs.len(), 2);
        assert!(crate::resolve::every_required_gate_has_admissible_result(
            &["gate://fixture.pass@1.0.0".to_owned()],
            &evidence.runs,
        ));
    }

    #[test]
    fn standalone_passing_run_is_not_admissible_evidence() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let receipt_dir = repo.root.join(&repo.config.paths.receipts);
        std::fs::create_dir_all(&receipt_dir).expect("receipt directory");
        let run = GateRun {
            id: "GR-standalone".to_owned(),
            gate: "fixture.pass@1.0.0".to_owned(),
            askability: Askability::Askable,
            execution_status: ExecutionStatus::Completed,
            verdict: Verdict::Pass,
            reason_code: Some(ReasonCode::Passed),
        };
        std::fs::write(
            run_record_path(&run.id, &repo),
            toml::to_string_pretty(&run).expect("serialize standalone run"),
        )
        .expect("write standalone Gate Run");

        let expected_subjects = [format!("sha256:{}", "b".repeat(64))].into_iter().collect();
        let evidence = crate::gate_evidence::load_admissible(
            &repo,
            &expected_subjects,
            &BTreeSet::new(),
            &verifier_register("gate_runner"),
        );
        assert!(evidence.runs.is_empty());
        assert!(evidence.failures.is_empty());
        assert!(!crate::resolve::every_required_gate_has_admissible_result(
            &["gate://fixture.pass@1.0.0".to_owned()],
            &evidence.runs,
        ));
    }

    #[test]
    fn tampered_receipt_bundle_is_not_admissible_evidence() {
        for plant in [
            "run",
            "run-bytes",
            "binding",
            "stdout",
            "selection",
            "selection-observation",
            "producer-substitution",
            "receipt",
        ] {
            let (_scratch, repo) = unrecorded_gate_fixture();
            let binding_path = matching_binding(&repo);
            let bound_subjects: BTreeSet<String> =
                openwarrant_core::legacy_disposition::parse_strict_json::<GateBinding>(
                    &std::fs::read(&binding_path).expect("authorized Gate Binding bytes"),
                )
                .expect("authorized Gate Binding")
                .subjects
                .into_iter()
                .collect();
            run(
                &repo,
                RunRequest {
                    execute: true,
                    only: Some("fixture.pass"),
                    record: true,
                    subject_digests: &[],
                    raw_evidence_refs: &[],
                    binding_path: Some(&binding_path),
                    warrant_alias: Some("OW-WAR-0014"),
                },
            )
            .expect("record Gate evidence fixture");

            let receipt_dir = repo.root.join(&repo.config.paths.receipts);
            let receipt_path = std::fs::read_dir(&receipt_dir)
                .expect("receipt directory")
                .filter_map(Result::ok)
                .filter_map(|entry| Utf8PathBuf::from_path_buf(entry.path()).ok())
                .find(|path| path.as_str().ends_with(".receipt.json"))
                .expect("recorded Gate receipt");
            let receipt: openwarrant_core::GateReceipt =
                openwarrant_core::legacy_disposition::parse_strict_json(
                    &std::fs::read(&receipt_path).expect("receipt bytes"),
                )
                .expect("Gate receipt");
            match plant {
                "run" => {
                    let path = run_record_path(&receipt.run_id, &repo);
                    let mut run: GateRun =
                        toml::from_str(&std::fs::read_to_string(&path).expect("Gate Run bytes"))
                            .expect("Gate Run");
                    run.verdict = Verdict::Fail;
                    run.reason_code = Some(ReasonCode::Failed);
                    std::fs::write(
                        path,
                        toml::to_string_pretty(&run).expect("serialize Gate Run plant"),
                    )
                    .expect("write Gate Run plant");
                }
                "run-bytes" => {
                    let path = run_record_path(&receipt.run_id, &repo);
                    let mut bytes = std::fs::read(&path).expect("Gate Run bytes");
                    bytes.extend_from_slice(b"# semantically inert byte mutation\n");
                    std::fs::write(path, bytes).expect("write Gate Run byte plant");
                }
                "binding" => {
                    let path = receipt_dir.join(format!("{}.binding.json", receipt.run_id));
                    let mut binding: GateBinding =
                        openwarrant_core::legacy_disposition::parse_strict_json(
                            &std::fs::read(&path).expect("Gate Binding bytes"),
                        )
                        .expect("Gate Binding");
                    binding.subjects.push(format!("sha256:{}", "c".repeat(64)));
                    std::fs::write(
                        path,
                        serde_json::to_vec(&binding).expect("serialize Gate Binding plant"),
                    )
                    .expect("write Gate Binding plant");
                }
                "stdout" => {
                    std::fs::write(
                        receipt_dir.join(format!("{}.stdout.txt", receipt.run_id)),
                        b"tampered stdout\n",
                    )
                    .expect("write stdout plant");
                }
                "selection" => {
                    let mut changed = receipt.clone();
                    changed.selected_test_manifest = vec!["test://caller-choice".to_owned()];
                    changed.selected_test_count = 1;
                    changed.receipt_digest.clear();
                    changed.receipt_digest = format!(
                        "sha256:{}",
                        sha256_digest(DigestDomain::GateReceipt, &changed)
                            .expect("Gate receipt digest")
                    );
                    std::fs::write(
                        &receipt_path,
                        serde_json::to_vec(&changed).expect("serialize selection plant"),
                    )
                    .expect("write selection plant");
                }
                "selection-observation" => {
                    let path = receipt_dir.join(format!("{}.selection.json", receipt.run_id));
                    let mut observation: openwarrant_core::TestSelectionObservation =
                        openwarrant_core::legacy_disposition::parse_strict_json(
                            &std::fs::read(&path).expect("selection observation bytes"),
                        )
                        .expect("selection observation");
                    observation.selected_test_manifest = vec!["test://caller-choice".to_owned()];
                    std::fs::write(
                        path,
                        serde_json::to_vec(&observation)
                            .expect("serialize selection-observation plant"),
                    )
                    .expect("write selection-observation plant");
                }
                "producer-substitution" => {
                    let mut changed = receipt.clone();
                    changed.producer_actor = "fixture-other-verifier".to_owned();
                    changed.receipt_digest.clear();
                    changed.receipt_digest = format!(
                        "sha256:{}",
                        sha256_digest(DigestDomain::GateReceipt, &changed)
                            .expect("Gate receipt digest")
                    );
                    std::fs::write(
                        &receipt_path,
                        serde_json::to_vec(&changed).expect("serialize producer plant"),
                    )
                    .expect("write producer plant");
                }
                "receipt" => {
                    let mut bytes = std::fs::read(&receipt_path).expect("receipt bytes");
                    bytes.pop();
                    bytes.extend_from_slice(b",\"unknown\":true}");
                    std::fs::write(&receipt_path, bytes).expect("write receipt plant");
                }
                _ => unreachable!("fixed receipt-bundle plant"),
            }

            let authority =
                verifier_registers(&[&repo.performer(), "gate_runner", "fixture-other-verifier"]);
            let evidence = crate::gate_evidence::load_admissible(
                &repo,
                &bound_subjects,
                &BTreeSet::new(),
                &authority,
            );
            assert!(
                evidence.runs.is_empty(),
                "{plant} tamper reached Resolution evidence"
            );
            assert_eq!(
                evidence.failures.len(),
                1,
                "{plant} tamper must produce one bounded rejection: {:?}",
                evidence.failures
            );
            if plant == "producer-substitution" {
                assert!(
                    evidence.failures[0]
                        .1
                        .contains("does not match its Gate Binding producer"),
                    "producer substitution must fail on exact Binding identity, not an incidental check: {:?}",
                    evidence.failures
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_gate_cannot_pass_while_a_descendant_keeps_its_output_open() {
        let (_scratch, repo) = unrecorded_gate_fixture();
        let pid_path = repo.root.join("descendant.pid");
        let script = format!(
            "while :; do printf x; sleep 0.01; done & printf '%s\\n' $! > {}; exit 0",
            pid_path
        );
        let definition = qualified_gate(vec!["/bin/sh".to_owned(), "-c".to_owned(), script], 1);

        let run = run_gate(&definition, &repo);
        let descendant = std::fs::read_to_string(&pid_path)
            .expect("descendant pid")
            .trim()
            .to_owned();
        let _ = Command::new("kill").args(["-9", &descendant]).status();

        assert_eq!(run.execution_status, ExecutionStatus::Timeout);
        assert_eq!(run.verdict, Verdict::Unknown);
    }
}
