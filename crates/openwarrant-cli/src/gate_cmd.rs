// SPDX-License-Identifier: Apache-2.0
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

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use openwarrant_compiler::sha256_hex;
use openwarrant_core::gate::GateDefinition;
use openwarrant_core::{Askability, ExecutionStatus, GateRun, ReasonCode, Verdict};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

const BONSAI_EVIDENCE_GATE: &str = "software.repo.bonsai-evidence";

/// The deadline used when a gate declares none.
const DEFAULT_GATE_TIMEOUT: Duration = Duration::from_secs(600);

/// How often the deadline is checked at first, and the ceiling it backs off to.
///
/// A flat 50ms wakes 2,400 times across a 120-second gate to learn nothing.
/// Backing off keeps a fast gate responsive and a slow one cheap.
const POLL_INTERVAL_MIN: Duration = Duration::from_millis(10);
const POLL_INTERVAL_MAX: Duration = Duration::from_millis(250);

/// How long to wait for a killed child to be reaped before giving up on it.
const REAP_GRACE: Duration = Duration::from_secs(5);

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

/// The file a gate's structured run is written to, under the receipts path.
///
/// One file per gate key, overwritten by the next run: the QUESTION answered is
/// "what did this gate last say?", and keeping a history here would invite
/// reading a stale pass as a current one.
#[must_use]
pub fn run_record_path(gate_key: &str, dir: &camino::Utf8Path) -> camino::Utf8PathBuf {
    let safe = gate_key.replace(['/', ':', '@', '.'], "_");
    dir.join(format!("{safe}.run.toml"))
}

/// Where a run's records go: the disposable receipts path by default, or a
/// caller-chosen directory — `war evidence record` passes a Warrant's tracked
/// `gate-runs/` so the receipt is minted AS committed evidence rather than
/// copied into it later (a copy with rewritten stream refs would not reseal).
#[must_use]
pub fn receipts_dir(repo: &Repository, out_dir: Option<&camino::Utf8Path>) -> camino::Utf8PathBuf {
    out_dir.map_or_else(
        || repo.root.join(&repo.config.paths.receipts),
        camino::Utf8Path::to_path_buf,
    )
}

/// Write a Gate Run where a later resolution can read it (§44.6).
pub fn persist_run(run: &GateRun, dir: &camino::Utf8Path) -> Result<(), String> {
    let path = run_record_path(&run.gate, dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("could not create {parent}: {e}"))?;
    }
    let rendered = toml::to_string_pretty(run).map_err(|e| e.to_string())?;
    // OW-WAR-0121: temp, fsync, rename; a symlink in the run's place refused.
    crate::compile::atomic::write(&path, rendered).map_err(|r| r.to_string())
}

/// Run one gate and produce its §44 result.
///
/// The `not_askable` path returns before any process is spawned.
#[must_use]
pub fn run_gate(def: &GateDefinition, repo: &Repository, dir: &camino::Utf8Path) -> GateRun {
    let id = format!("GR-{}", def.key());

    if let Some(reason) = askability_of(def, repo) {
        // §44.4: not_askable pairs with not_run (or invalid, for a malformed
        // definition) and verdict unknown. Never a verdict.
        let execution_status = if reason == ReasonCode::Malformed {
            ExecutionStatus::Invalid
        } else {
            ExecutionStatus::NotRun
        };
        return GateRun {
            id,
            gate: def.key(),
            askability: Askability::NotAskable,
            execution_status,
            verdict: Verdict::Unknown,
            reason_code: Some(reason),
        };
    }

    let deadline = def
        .timeout_secs
        .map_or(DEFAULT_GATE_TIMEOUT, Duration::from_secs);

    // Where the streams land. Created before the spawn so a failure to open them
    // is an askability problem, not a half-run.
    if std::fs::create_dir_all(dir).is_err() {
        return GateRun {
            id,
            gate: def.key(),
            askability: Askability::NotAskable,
            execution_status: ExecutionStatus::Invalid,
            verdict: Verdict::Unknown,
            reason_code: Some(ReasonCode::ForeignWorkingDirectory),
        };
    }
    let slug = def.key().replace(['/', '@', '.'], "_");
    let (out_path, err_path) = (
        dir.join(format!("{slug}.stdout.txt")),
        dir.join(format!("{slug}.stderr.txt")),
    );
    let (Ok(stdout_file), Ok(stderr_file)) = (
        std::fs::File::create(&out_path),
        std::fs::File::create(&err_path),
    ) else {
        return GateRun {
            id,
            gate: def.key(),
            askability: Askability::NotAskable,
            execution_status: ExecutionStatus::Invalid,
            verdict: Verdict::Unknown,
            reason_code: Some(ReasonCode::ForeignWorkingDirectory),
        };
    };

    let started = Instant::now();
    let spawned = Command::new(&def.argv[0])
        .args(&def.argv[1..])
        .current_dir(&repo.root)
        // §44.6 requires stdout and stderr REFS on the receipt, so the streams
        // are captured to files rather than discarded. Piping without draining
        // can deadlock a chatty child on a full pipe buffer, so they go straight
        // to files opened here.
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn();

    let mut child = match spawned {
        Ok(c) => c,
        // The tool resolved during askability and would not spawn. The commonest
        // cause is a race — it was removed, or it is not executable — so the two
        // are told apart rather than both landing on "infrastructure".
        Err(err) => {
            return match err.kind() {
                // It vanished between the check and the spawn. That is still
                // "could not ask", and saying so keeps it out of the failure
                // column where it does not belong.
                std::io::ErrorKind::NotFound => GateRun {
                    id,
                    gate: def.key(),
                    askability: Askability::NotAskable,
                    execution_status: ExecutionStatus::NotRun,
                    verdict: Verdict::Unknown,
                    reason_code: Some(classify_missing(&def.argv[0])),
                },
                // Present but unusable: permissions, exhausted resources. The
                // gate was askable and the environment failed, which is not a
                // verdict about the subject.
                _ => GateRun {
                    id,
                    gate: def.key(),
                    askability: Askability::Askable,
                    execution_status: ExecutionStatus::InfrastructureError,
                    verdict: Verdict::Unknown,
                    reason_code: None,
                },
            };
        }
    };

    let mut poll = POLL_INTERVAL_MIN;

    // A real deadline. The previous version compared elapsed time AFTER
    // `output()` had already blocked to completion, which meant a gate running
    // 601 seconds and exiting 0 was reported `timeout` + `unknown` — discarding
    // a genuine pass and reporting a status that had not happened. Polling and
    // killing is what `timeout` has to mean for the word to be true.
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let passed = status.success();
                return GateRun {
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
                };
            }
            Ok(None) if started.elapsed() >= deadline => {
                let _ = child.kill();
                // Reap, but never unboundedly. A plain `wait()` here blocks
                // forever if the child is in uninterruptible sleep and cannot
                // take the signal yet — and a runner whose whole purpose is a
                // deadline must not be the thing that hangs. SIGKILL is not
                // blockable, so the child dies once its I/O completes; if it
                // outlasts this window it is left to be reaped at exit rather
                // than held onto.
                let reap_until = Instant::now() + REAP_GRACE;
                while Instant::now() < reap_until {
                    match child.try_wait() {
                        Ok(Some(_)) | Err(_) => break,
                        Ok(None) => std::thread::sleep(POLL_INTERVAL_MIN),
                    }
                }
                return GateRun {
                    id,
                    gate: def.key(),
                    askability: Askability::Askable,
                    execution_status: ExecutionStatus::Timeout,
                    verdict: Verdict::Unknown,
                    reason_code: Some(ReasonCode::Timeout),
                };
            }
            Ok(None) => {
                std::thread::sleep(poll);
                poll = (poll * 2).min(POLL_INTERVAL_MAX);
            }
            Err(_) => {
                return GateRun {
                    id,
                    gate: def.key(),
                    askability: Askability::Askable,
                    execution_status: ExecutionStatus::InfrastructureError,
                    verdict: Verdict::Unknown,
                    reason_code: None,
                };
            }
        }
    }
}

/// What a receipt says it ran over, beyond the contract (OW-WAR-0133).
///
/// # The subjects
///
/// A receipt minted before OW-WAR-0133 named one subject, the contract digest,
/// and so survived every change the contract digest does not see: the source
/// bytes, the deliverable set, the fixtures. These are the subjects added
/// beside it — new entries in `subject_digests`, never new receipt fields, so
/// every receipt already on disk still parses and still reseals:
///
/// - `tree:<sha>` — `git rev-parse HEAD^{tree}` when the gate started;
/// - `worktree:dirty` — present when the working tree differed from that
///   tree, outside the evidence records, projections and authority records
///   ([`source::Exclusions`]);
/// - `inputs:sha256:<hex>` — the bytes of every file the Gate Definition's
///   `inputs` globs match, when it declares any;
/// - `deliverables:sha256:<hex>` — the bytes of the Warrant's declared
///   deliverables in `D-` order, added by `war evidence record`, which is the
///   only caller that knows which Warrant the run is for.
///
/// `fixture_digests` is filled from the definition's `fixtures`, one
/// `<path>#sha256:<hex>` per declared fixture.
///
/// # Which subject decides reuse (Q-001, settled as (c) with the (a) fallback)
///
/// A gate that declares `inputs` is judged by them: the run holds while those
/// files digest the same. A gate that declares none falls back to the tree:
/// the run holds while nothing outside the evidence records, compiled
/// projections and authority records has changed since the tree it ran over.
/// The deliverables digest is recorded and advisory —
/// a gate reads what its inputs say it reads, and a gate that reads more than
/// it declares keeps a stale pass that the tree subject makes visible (R-001).
///
/// # Why a signature does not move the tree (t-22fd)
///
/// The tree rule also skips the records a human act writes — authorizations,
/// judgments, resolutions, responses, batches, attestations, corrections,
/// disputes, invalidations and answered questions
/// ([`source::is_authority_record`]). Without that, every signature recorded
/// after the evidence staled every tree-bound receipt, and a Warrant needed
/// two sittings: authorize, then — after a re-run — resolve. The records are
/// not the source a gate ran over; each is verified by its own ingest when it
/// is written, and the ones a resolution relies on (its authorization,
/// judgments and the resolver's role) are judged live by the resolution
/// itself. `war check` and the battery do read them; their receipts say
/// nothing about authority records written after them, and that is the
/// stated limit. Declared `inputs` are not narrowed by this: a gate that
/// declares it reads an authority record is held to it.
///
/// `inputs` and `fixtures` are read from the definition file here rather than
/// from `GateDefinition`, whose fields are the core crate's; the definition
/// reader ignores keys it does not know, so a definition carrying them still
/// registers everywhere else unchanged.
pub mod source {
    use camino::{Utf8Path, Utf8PathBuf};
    use openwarrant_compiler::sha256_hex;
    use std::process::Command;

    use crate::repo::Repository;

    /// `tree:<git tree sha>`.
    pub const TREE: &str = "tree:";
    /// Present when the tree a run started from was not what the gate saw.
    pub const DIRTY: &str = "worktree:dirty";
    /// `inputs:sha256:<hex>` over the files a definition declares it reads.
    pub const INPUTS: &str = "inputs:sha256:";
    /// `deliverables:sha256:<hex>` over a Warrant's declared deliverables.
    pub const DELIVERABLES: &str = "deliverables:sha256:";

    /// What a Gate Definition declares about the files it reads, beyond the
    /// fields `GateDefinition` carries.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Declared {
        /// Repository-relative globs (`*`, `?`, `**`) the gate reads.
        pub inputs: Vec<String>,
        /// Repository-relative fixture files the gate runs against.
        pub fixtures: Vec<String>,
    }

    /// The definition file for `<gate_id>@<version>` and what it declares, or
    /// `None` when no definition in the registry directory has that key.
    #[must_use]
    pub fn declared(repo: &Repository, key: &str) -> Option<Declared> {
        let key = key.trim_start_matches("gate://");
        let dir = repo.root.join(&repo.config.paths.gates);
        let entries = dir.read_dir_utf8().ok()?;
        let mut paths: Vec<Utf8PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.into_path())
            .filter(|p| p.extension().is_some_and(|e| e == "yaml" || e == "yml"))
            .collect();
        paths.sort();
        for path in paths {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(doc) = openwarrant_core::structured::parse(&text) else {
                continue;
            };
            let id = doc.scalar("gate_id").unwrap_or_default();
            let version = doc.scalar("version").unwrap_or_default();
            if format!("{id}@{version}") != key {
                continue;
            }
            let list = |k: &str| {
                doc.get(k)
                    .and_then(openwarrant_core::StructuredValue::as_list)
                    .map(<[String]>::to_vec)
                    .unwrap_or_default()
            };
            return Some(Declared {
                inputs: list("inputs"),
                fixtures: list("fixtures"),
            });
        }
        None
    }

    /// Whether a repository-relative path is an evidence record: a Warrant's
    /// `gate-runs/` or its `journal.jsonl`.
    ///
    /// These are excluded from every source comparison because recording
    /// evidence writes them. Without the exclusion, committing a receipt
    /// would move the tree the receipt names, and no tree-bound run could
    /// ever be admissible once it was committed — the "refuses every
    /// receipt" rule OBL-002's control exists to catch.
    #[must_use]
    pub fn is_evidence_record(path: &str, warrants: &str) -> bool {
        let Some(rest) = under(path, warrants) else {
            return false;
        };
        let mut parts = rest.splitn(3, '/');
        let (Some(_alias), Some(second)) = (parts.next(), parts.next()) else {
            return false;
        };
        match parts.next() {
            Some(_) => second == "gate-runs",
            None => second == "journal.jsonl",
        }
    }

    /// Whether a repository-relative path is a record a human act writes
    /// (t-22fd), under the configured roots: `warrants`, `sas`, `roadmap`,
    /// `gates`.
    ///
    /// Named file by file, not by directory, where the directory also holds
    /// what a gate reads: `docs/authority/roles.toml` and `allowed_signers`
    /// are trust roots a human edits by hand and stay bound, as does a SAS or
    /// roadmap revision record — accepting one changes the specification the
    /// corpus is judged against, so it is source, not a record about the
    /// work. Their acceptance attestations are records.
    #[must_use]
    pub fn is_authority_record(
        path: &str,
        warrants: &str,
        sas: &str,
        roadmap: &str,
        gates: &str,
    ) -> bool {
        const AUTHORITY_DIRS: [&str; 3] = [
            "docs/authority/responses",
            "docs/authority/batches",
            "docs/authority/standing/attestations",
        ];
        if AUTHORITY_DIRS.iter().any(|d| under(path, d).is_some()) {
            return true;
        }
        if under(
            path,
            &format!("{}/revisions/attestations", sas.trim_end_matches('/')),
        )
        .is_some()
            || under(
                path,
                &format!("{}/revisions/attestations", roadmap.trim_end_matches('/')),
            )
            .is_some()
            || under(
                path,
                &format!(
                    "{}/{}",
                    gates.trim_end_matches('/'),
                    crate::invalidation::RECORDS_DIR
                ),
            )
            .is_some()
        {
            return true;
        }
        // `<warrants>/<alias>/<file>` or `<warrants>/<alias>/<dir>/…`.
        let Some(rest) = under(path, warrants) else {
            return false;
        };
        let mut parts = rest.splitn(3, '/');
        let (Some(_alias), Some(second)) = (parts.next(), parts.next()) else {
            return false;
        };
        match parts.next() {
            Some(_) => matches!(
                second,
                "attestations" | "corrections" | "disputes" | crate::questions::DIR
            ),
            None => {
                matches!(second, "authorization.toml" | "judgments.toml" | "resolution.toml")
                    // The superseded authorization kept beside the new one
                    // (OW-WAR-0144): `authorization.<sha256[..8]>.toml`.
                    || second
                        .strip_prefix("authorization.")
                        .and_then(|r| r.strip_suffix(".toml"))
                        .is_some_and(|h| {
                            h.len() == 8 && h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
                        })
            }
        }
    }

    fn under<'a>(path: &'a str, dir: &str) -> Option<&'a str> {
        let dir = dir.trim_end_matches('/');
        if dir.is_empty() {
            return None;
        }
        path.strip_prefix(dir).and_then(|r| r.strip_prefix('/'))
    }

    /// Paths no source comparison reads: evidence records, and the
    /// projections `war compile` writes.
    ///
    /// Projections are excluded for the same reason as records, one step
    /// removed: the corpus status projects each run's admissibility, so
    /// committing the projection a receipt produced would move the tree the
    /// receipt names, and the next compile would project it stale — a
    /// projection that could never agree with itself. `war check --generated`
    /// is what holds a projection to its sources; the tree rule does not
    /// need to.
    ///
    /// The tree rule skips one class more: the records a human act writes
    /// ([`is_authority_record`], [`Exclusions::excludes_from_tree`]). Declared
    /// inputs do not — a gate that says it reads one is held to it.
    #[derive(Debug, Clone)]
    pub struct Exclusions {
        warrants: String,
        sas: String,
        roadmap: String,
        gates: String,
        projection_dirs: Vec<String>,
    }

    impl Exclusions {
        #[must_use]
        pub fn of(repo: &Repository) -> Self {
            let p = &repo.config.paths;
            let generated_parent =
                std::path::Path::new(openwarrant_compiler::current::CURRENT_PATH)
                    .parent()
                    .map(|d| d.to_string_lossy().into_owned())
                    .unwrap_or_default();
            Self {
                warrants: p.warrants.to_string(),
                sas: p.sas.to_string(),
                roadmap: p.roadmap.to_string(),
                gates: p.gates.to_string(),
                projection_dirs: vec![
                    format!("{}/generated", p.warrants),
                    format!("{}/generated", p.adrs),
                    format!("{}/generated", p.sas),
                    generated_parent,
                ],
            }
        }

        /// Whether `path` is an evidence record or a compiled projection.
        #[must_use]
        pub fn excludes(&self, path: &str) -> bool {
            if is_evidence_record(path, &self.warrants) {
                return true;
            }
            if self
                .projection_dirs
                .iter()
                .any(|d| under(path, d).is_some())
            {
                return true;
            }
            // A Warrant's own projections: `<warrants>/<alias>/generated/…`.
            under(path, &self.warrants).is_some_and(|rest| {
                let mut parts = rest.splitn(3, '/');
                matches!(
                    (parts.next(), parts.next(), parts.next()),
                    (Some(_), Some("generated"), Some(_))
                )
            })
        }

        /// What the tree rule skips: [`Exclusions::excludes`], and the
        /// records a human act writes. A signature recorded after a run does
        /// not move the tree that run names (t-22fd).
        #[must_use]
        pub fn excludes_from_tree(&self, path: &str) -> bool {
            self.excludes(path)
                || is_authority_record(path, &self.warrants, &self.sas, &self.roadmap, &self.gates)
        }
    }

    fn git(root: &Utf8Path, args: &[&str]) -> Result<Vec<u8>, String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(root.as_str())
            .args(args)
            .output()
            .map_err(|e| format!("git could not be run: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "`git {}` failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(out.stdout)
    }

    fn nul_paths(bytes: &[u8]) -> Vec<String> {
        bytes
            .split(|b| *b == 0)
            .filter(|p| !p.is_empty())
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect()
    }

    /// `git rev-parse HEAD^{tree}`.
    pub fn head_tree(root: &Utf8Path) -> Result<String, String> {
        let out = git(root, &["rev-parse", "--verify", "-q", "HEAD^{tree}"])?;
        let sha = String::from_utf8_lossy(&out).trim().to_owned();
        if sha.is_empty() {
            return Err("the repository has no HEAD commit".to_owned());
        }
        Ok(sha)
    }

    /// Paths that differ between `tree` and the working tree, plus untracked
    /// files that are not ignored — what [`Exclusions::excludes_from_tree`]
    /// names left out.
    pub fn moved_since(
        root: &Utf8Path,
        tree: &str,
        ex: &Exclusions,
    ) -> Result<Vec<String>, String> {
        let mut moved = nul_paths(&git(root, &["diff", "--name-only", "-z", tree, "--"])?);
        moved.extend(nul_paths(&git(
            root,
            &["ls-files", "--others", "--exclude-standard", "-z"],
        )?));
        moved.retain(|p| !ex.excludes_from_tree(p));
        moved.sort();
        moved.dedup();
        Ok(moved)
    }

    /// Whether `pattern` matches `path`. `*` and `?` stay within one path
    /// segment; `**` spans any number of segments, including none.
    #[must_use]
    pub fn glob_matches(pattern: &str, path: &str) -> bool {
        fn segs(pat: &[&str], path: &[&str]) -> bool {
            match pat.split_first() {
                None => path.is_empty(),
                Some((&"**", rest)) => (0..=path.len()).any(|i| segs(rest, &path[i..])),
                Some((p, rest)) => path
                    .split_first()
                    .is_some_and(|(s, tail)| seg(p.as_bytes(), s.as_bytes()) && segs(rest, tail)),
            }
        }
        fn seg(p: &[u8], s: &[u8]) -> bool {
            match p.split_first() {
                None => s.is_empty(),
                Some((b'*', rest)) => (0..=s.len()).any(|i| seg(rest, &s[i..])),
                Some((b'?', rest)) => !s.is_empty() && seg(rest, &s[1..]),
                Some((c, rest)) => s.first() == Some(c) && seg(rest, &s[1..]),
            }
        }
        let pat: Vec<&str> = pattern.trim_start_matches("./").split('/').collect();
        let path: Vec<&str> = path.split('/').collect();
        segs(&pat, &path)
    }

    /// `sha256` over `<path>\0<sha256 of bytes>\n` for every tracked or
    /// untracked-but-not-ignored file the globs match, sorted by path, read
    /// from the working tree. An excluded path is never an input.
    pub fn inputs_digest(
        root: &Utf8Path,
        globs: &[String],
        ex: &Exclusions,
    ) -> Result<String, String> {
        let mut files = nul_paths(&git(
            root,
            &[
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
            ],
        )?);
        files.retain(|p| !ex.excludes(p) && globs.iter().any(|g| glob_matches(g, p)));
        files.sort();
        files.dedup();
        let mut preimage = String::new();
        for f in &files {
            // A tracked file deleted from the working tree is part of what the
            // gate would now see: its absence, named.
            let digest = std::fs::read(root.join(f))
                .map_or_else(|_| "absent".to_owned(), |b| sha256_hex(&b));
            preimage.push_str(&format!("{f}\0{digest}\n"));
        }
        Ok(format!("{INPUTS}{}", sha256_hex(preimage.as_bytes())))
    }

    /// `<path>#sha256:<hex>` for each declared fixture. A declared fixture that
    /// cannot be read is an error: the receipt would otherwise say the gate
    /// ran against a fixture nobody could name.
    pub fn fixture_digests(root: &Utf8Path, fixtures: &[String]) -> Result<Vec<String>, String> {
        fixtures
            .iter()
            .map(|f| {
                std::fs::read(root.join(f))
                    .map(|b| format!("{f}#sha256:{}", sha256_hex(&b)))
                    .map_err(|e| format!("declared fixture {f} cannot be read: {e}"))
            })
            .collect()
    }

    /// `deliverables:sha256:<hex>` over `(id, target_ref, sha256 of bytes)` in
    /// `D-` order. A target that is not a readable file digests as `absent`.
    #[must_use]
    pub fn deliverables_digest(root: &Utf8Path, set: &[(String, String)]) -> String {
        let mut set: Vec<&(String, String)> = set.iter().collect();
        set.sort();
        let mut preimage = String::new();
        for (id, target) in set {
            let digest = std::fs::read(root.join(target))
                .map_or_else(|_| "absent".to_owned(), |b| sha256_hex(&b));
            preimage.push_str(&format!("{id}\0{target}\0{digest}\n"));
        }
        format!("{DELIVERABLES}{}", sha256_hex(preimage.as_bytes()))
    }

    /// What a run is about to observe: its source subjects and fixture
    /// digests. Taken BEFORE the gate is spawned, so the receipt names the
    /// source the gate ran over rather than whatever it left behind.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct Observed {
        pub subjects: Vec<String>,
        pub fixture_digests: Vec<String>,
    }

    /// Observe the source for `key`. Not being a git repository is not an
    /// error: the receipt then names no tree and its reuse is `UNKNOWN`
    /// later, which is the truth. An unreadable declared fixture is an error.
    pub fn observe(repo: &Repository, key: &str) -> Result<Observed, String> {
        let ex = Exclusions::of(repo);
        let declared = declared(repo, key).unwrap_or_default();
        let mut subjects = Vec::new();
        if let Ok(tree) = head_tree(&repo.root) {
            subjects.push(format!("{TREE}{tree}"));
            if moved_since(&repo.root, &tree, &ex).map_or(true, |m| !m.is_empty()) {
                subjects.push(DIRTY.to_owned());
            }
        }
        if !declared.inputs.is_empty()
            && let Ok(d) = inputs_digest(&repo.root, &declared.inputs, &ex)
        {
            subjects.push(d);
        }
        let fixture_digests = fixture_digests(&repo.root, &declared.fixtures)?;
        Ok(Observed {
            subjects,
            fixture_digests,
        })
    }
}

/// `war gate list` / `war gate run`.
pub fn run(
    repo: &Repository,
    execute: bool,
    only: Option<&str>,
    record: bool,
    subject_digests: &[String],
    raw_evidence_refs: &[String],
    out_dir: Option<&camino::Utf8Path>,
) -> Result<Report, RepoError> {
    validate_bonsai_bindings(repo, only, subject_digests, raw_evidence_refs)?;
    let dir = receipts_dir(repo, out_dir);
    let mut report = Report::default();
    let registry = crate::check::load_gate_registry(repo, &mut report);

    if registry.is_empty() {
        report.note("no gate definitions found; nothing to run".to_owned());
        return Ok(report);
    }

    for def in &registry.definitions {
        if only.is_some_and(|filter| def.gate_id != filter && def.key() != filter) {
            continue;
        }

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
        // OW-WAR-0133: what the gate is about to run over, named before it is
        // spawned. A recorded run that cannot name a declared fixture is not
        // run; an unrecorded one is receipted without the source.
        let observed = match source::observe(repo, &def.key()) {
            Ok(o) => Some(o),
            Err(e) if record => {
                report.push(Diagnostic::error(
                    "gate-run.receipt-failed",
                    def.key(),
                    format!("{e} — nothing was run"),
                ));
                continue;
            }
            Err(_) => None,
        };
        let run = run_gate(def, repo, &dir);
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
        if record && let Err(err) = persist_run(&run, &dir) {
            report.push(Diagnostic::error(
                "gate-run.not-persisted",
                def.key(),
                format!(
                    "{err} — a run that is not written cannot answer §56.1's \
                     admissible-result requirement later"
                ),
            ));
        }

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

        // §44.6 — a completed run produces a receipt. Only a run that actually
        // executed has anything to receipt: an unaskable gate produced no
        // working directory, no exit result and no streams, and minting a
        // receipt for it would be minting evidence of something that did not
        // happen.
        if run.execution_status == ExecutionStatus::Completed {
            match receipt::mint_observed(
                repo,
                def,
                &run,
                &started_at,
                &format!("{}", run.verdict),
                receipt::Bindings {
                    subject_digests,
                    raw_evidence_refs,
                },
                &dir,
                observed.as_ref(),
            ) {
                Ok(path) => report.push(Diagnostic::pass(
                    "gate-run.receipt",
                    format!(
                        "{}: §44.6 receipt written to {}",
                        def.key(),
                        repo.relative(&path)
                    ),
                )),
                Err(e) => report.push(Diagnostic::error(
                    "gate-run.receipt-failed",
                    def.key(),
                    format!("{e}"),
                )),
            }
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
    only: Option<&str>,
    subject_digests: &[String],
    raw_evidence_refs: &[String],
) -> Result<(), RepoError> {
    if subject_digests.is_empty() && raw_evidence_refs.is_empty() {
        return Ok(());
    }
    // The pairing rules below are Bonsai's (§43.5 binding of a document to its
    // evidence). Any other gate may be bound to a contract subject alone —
    // that is what `war evidence record` does (OW-WAR-0059) — and the subject
    // is checked for shape, not for Bonsai's one-to-one pairing.
    if !matches!(
        only,
        Some(BONSAI_EVIDENCE_GATE) | Some("software.repo.bonsai-evidence@1.0.0")
    ) {
        for subject in subject_digests {
            let ok = subject
                .strip_prefix("contract:sha256:")
                .is_some_and(is_hex_digest)
                || subject
                    .strip_prefix(source::DELIVERABLES)
                    .is_some_and(is_hex_digest)
                || subject.starts_with("warrant-corpus:");
            if !ok {
                return Err(RepoError::Message(format!(
                    "receipt subject {subject:?} must be contract:sha256:<64 hex>, \
                     deliverables:sha256:<64 hex> or warrant-corpus:<path>"
                )));
            }
        }
        if !raw_evidence_refs.is_empty() {
            return Err(RepoError::Message(
                "--evidence-ref is a Bonsai binding; other gates take --subject-digest only"
                    .to_owned(),
            ));
        }
        return Ok(());
    }
    if subject_digests.len() != 1 || raw_evidence_refs.len() != 1 {
        return Err(RepoError::Message(
            "Bonsai receipt binding requires exactly one contract subject and one evidence reference"
                .to_owned(),
        ));
    }
    if !matches!(
        only,
        Some(BONSAI_EVIDENCE_GATE) | Some("software.repo.bonsai-evidence@1.0.0")
    ) {
        return Err(RepoError::Message(format!(
            "Bonsai receipt bindings are valid only for {BONSAI_EVIDENCE_GATE}@1.0.0"
        )));
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
    let bytes = std::fs::read(repo.root.join(path)).map_err(|source| RepoError::Io {
        context: format!("could not read Bonsai evidence {path}"),
        source,
    })?;
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
    Ok(())
}

fn safe_evidence_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
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
/// eighteen things; nothing produced one, so the requirement was satisfied by a
/// struct definition and a test that constructed one by hand.
///
/// A receipt is also the artifact every beta obligation cites. An obligation
/// demanding "a gate-run receipt whose verdict is pass" is undischargeable by a
/// `#[test]` precisely because a receipt binds wall-clock times, a working
/// directory, an exit result and a digest that has to recompute.
pub mod receipt {
    use camino::Utf8Path;
    use openwarrant_compiler::canonical::sha256_digest;
    use openwarrant_compiler::digest::DigestDomain;
    use openwarrant_core::gate::GateDefinition;
    use openwarrant_core::{GateReceipt, GateRun};

    use crate::repo::{RepoError, Repository};

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
        rfc3339_from_secs(secs)
    }

    /// RFC 3339 UTC from Unix seconds. Shared with the journal, whose
    /// backfilled `draft.created` events take their time from the UUIDv7.
    #[must_use]
    pub fn rfc3339_from_secs(secs: u64) -> String {
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

    /// What a receipt is bound to beyond the gate itself (§44.6 subject and
    /// raw-evidence refs).
    #[derive(Debug, Clone, Copy)]
    pub struct Bindings<'a> {
        pub subject_digests: &'a [String],
        pub raw_evidence_refs: &'a [String],
    }

    /// Build and persist the receipt. Returns its path.
    ///
    /// The receipt is VALIDATED before it is written. A malformed receipt on
    /// disk is worse than none: it looks like evidence.
    ///
    /// No source subjects: this is `war run`'s path, whose receipt is bound
    /// to exactly one subject, the dispatch digest, and is checked against
    /// that one subject when a Warrant is preserved. Receipts that count
    /// toward requirement 5 are minted by `war evidence record` through
    /// [`mint_observed`].
    pub fn mint(
        repo: &Repository,
        def: &GateDefinition,
        run: &GateRun,
        started_at: &str,
        exit_result: &str,
        bindings: Bindings<'_>,
        dir: &Utf8Path,
    ) -> Result<camino::Utf8PathBuf, RepoError> {
        mint_observed(repo, def, run, started_at, exit_result, bindings, dir, None)
    }

    /// [`mint`], with the source the run was observed to start from
    /// (OW-WAR-0133). Its subjects are appended after the caller's, and its
    /// fixture digests fill `fixture_digests`. `None` records neither.
    #[allow(clippy::too_many_arguments)]
    pub fn mint_observed(
        repo: &Repository,
        def: &GateDefinition,
        run: &GateRun,
        started_at: &str,
        exit_result: &str,
        bindings: Bindings<'_>,
        dir: &Utf8Path,
        observed: Option<&super::source::Observed>,
    ) -> Result<camino::Utf8PathBuf, RepoError> {
        let Bindings {
            subject_digests,
            raw_evidence_refs,
        } = bindings;
        let slug = def.key().replace(['/', '@', '.'], "_");
        let rel = |p: &Utf8Path| repo.relative(p);

        let mut receipt = GateReceipt {
            run_id: run.id.clone(),
            gate_definition_digest: if def.digest.is_empty() {
                // A definition with no declared digest still has an identity;
                // deriving one here keeps the receipt complete without inventing
                // a value that looks like the author's.
                format!(
                    "sha256:{}",
                    sha256_digest(DigestDomain::GateRun, &def.key())
                        .map_err(|e| RepoError::Message(format!("{e}")))?
                )
            } else {
                def.digest.clone()
            },
            // §43.5 bindings do not exist in this corpus yet. Recording the
            // gate's own key is honest; inventing a binding digest would not be.
            gate_binding_digest: format!("unbound:{}", def.key()),
            subject_digests: {
                let mut s = if subject_digests.is_empty() {
                    vec![format!("warrant-corpus:{}", repo.config.paths.warrants)]
                } else {
                    subject_digests.to_vec()
                };
                s.extend(observed.iter().flat_map(|o| o.subjects.iter().cloned()));
                s
            },
            fixture_digests: observed
                .map(|o| o.fixture_digests.clone())
                .unwrap_or_default(),
            runner: "war gate --run".to_owned(),
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
            exit_result: exit_result.to_owned(),
            selected_test_count: 0,
            selected_test_manifest: vec![],
            raw_evidence_refs: raw_evidence_refs.to_vec(),
            stdout_ref: rel(&dir.join(format!("{slug}.stdout.txt"))),
            stderr_ref: rel(&dir.join(format!("{slug}.stderr.txt"))),
            resource_usage: format!("wall-clock only; {} argv item(s)", def.argv.len()),
            verdict: run.verdict,
            receipt_digest: String::new(),
        };

        // Digest last, over everything else, so it covers the record it seals.
        receipt.receipt_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateRun, &receipt)
                .map_err(|e| RepoError::Message(format!("{e}")))?
        );

        receipt.validate().map_err(|e| {
            RepoError::Message(format!("refusing to write a malformed receipt: {e}"))
        })?;

        let path = dir.join(format!("{slug}.receipt.json"));
        let body = serde_json::to_string_pretty(&receipt)
            .map_err(|e| RepoError::Message(format!("{e}")))?;
        // OW-WAR-0121: a receipt is evidence a resolution cites; a crash while
        // writing it leaves the previous receipt whole, never half of one.
        crate::compile::atomic::write(&path, body + "\n")?;
        Ok(path)
    }
}

#[cfg(test)]
mod source_tests {
    use super::source::{glob_matches, is_authority_record, is_evidence_record};

    #[test]
    fn globs_keep_star_within_a_segment_and_let_double_star_span() {
        assert!(glob_matches("src/**", "src/a.rs"));
        assert!(glob_matches("src/**", "src/deep/er/a.rs"));
        assert!(glob_matches("src/*.rs", "src/a.rs"));
        assert!(!glob_matches("src/*.rs", "src/deep/a.rs"));
        assert!(glob_matches("**/*.md", "README.md"));
        assert!(glob_matches("docs/?.md", "docs/a.md"));
        assert!(!glob_matches("src/**", "srcx/a.rs"));
        assert!(!glob_matches("README.md", "docs/README.md"));
    }

    #[test]
    fn only_gate_runs_and_the_journal_are_evidence_records() {
        let w = "docs/warrants";
        assert!(is_evidence_record(
            "docs/warrants/X-WAR-0001/gate-runs/a.json",
            w
        ));
        assert!(is_evidence_record(
            "docs/warrants/X-WAR-0001/journal.jsonl",
            w
        ));
        assert!(!is_evidence_record(
            "docs/warrants/X-WAR-0001/atoms/10-intent.md",
            w
        ));
        assert!(!is_evidence_record(
            "docs/warrants/X-WAR-0001/deliverables.toml",
            w
        ));
        assert!(!is_evidence_record("src/gate-runs/a", w));
        assert!(!is_evidence_record("docs/warrants/journal.jsonl", w));
    }

    /// t-22fd: what a human act writes is skipped by the tree rule; what a
    /// gate reads as source, and the trust roots, are not.
    #[test]
    fn authority_records_are_named_file_by_file_and_source_is_not_one() {
        let a = |p: &str| {
            is_authority_record(p, "docs/warrants", "docs/sas", "docs/roadmap", "docs/gates")
        };
        for p in [
            "docs/warrants/X-WAR-0001/authorization.toml",
            "docs/warrants/X-WAR-0001/authorization.0a1b2c3d.toml",
            "docs/warrants/X-WAR-0001/judgments.toml",
            "docs/warrants/X-WAR-0001/resolution.toml",
            "docs/warrants/X-WAR-0001/attestations/authorize-1.dsse.json",
            "docs/warrants/X-WAR-0001/corrections/D-001-1.toml",
            "docs/warrants/X-WAR-0001/disputes/DSP-001.toml",
            "docs/warrants/X-WAR-0001/questions/Q-001.toml",
            "docs/authority/responses/X-WAR-0001.response.toml",
            "docs/authority/responses/X-WAR-0001.response.toml.sig",
            "docs/authority/batches/B-1-abcdef01.json",
            "docs/authority/batches/attestations/batch-B-1-1.dsse.json",
            "docs/authority/standing/attestations/standing-accept-c@1-1.dsse.json",
            "docs/sas/revisions/attestations/sas-accept-1.0.0-1.dsse.json",
            "docs/roadmap/revisions/attestations/roadmap-accept-r1-1.dsse.json",
            "docs/gates/invalidations/g@1.0.0.toml",
        ] {
            assert!(a(p), "{p} is written by a human act");
        }
        for p in [
            // Source, and what a Warrant is made of.
            "src/authorization.toml",
            "crates/x/src/lib.rs",
            "docs/warrants/X-WAR-0001/manifest.toml",
            "docs/warrants/X-WAR-0001/deliverables.toml",
            "docs/warrants/X-WAR-0001/atoms/60-assurance.md",
            "docs/warrants/X-WAR-0001/authorization.toml.bak",
            "docs/warrants/X-WAR-0001/authorization.ZZZZZZZZ.toml",
            "docs/warrants/X-WAR-0001/authorization.0a1b2c3.toml",
            "docs/warrants/authorization.toml",
            // Trust roots and governing records: bound.
            "docs/authority/roles.toml",
            "docs/authority/allowed_signers",
            "docs/authority/standing/c@1.toml",
            "docs/sas/revisions/1.0.0.toml",
            "docs/sas/SAS.md",
            "docs/roadmap/revisions/1.toml",
            "docs/gates/g@1.0.0.yaml",
        ] {
            assert!(!a(p), "{p} is not an authority record");
        }
    }
}
