// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war` — the OpenWarrant command line interface (SAS §70–§76).

#![forbid(unsafe_code)]

use std::process::ExitCode;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use openwarrant_core::Profile;

mod authorize;
mod blut;
mod bonsai;
mod check;
mod compile;
mod correct;
mod diagnostic;
mod dispatch;
mod evidence;
mod export;
mod gate_cmd;
mod init;
mod journal_cmd;
mod kf;
mod migrate;
mod new;
mod next;
mod output;
mod pins;
mod relations;
mod repo;
mod resolution_cmd;
mod resolve;
mod sas;
mod show;
mod sign;
mod status;
mod telemetry;
mod verify;

#[derive(clap::Subcommand, Debug)]
enum KfCommand {
    /// Read KF's health. The only call here that cannot mutate.
    Health {
        #[arg(long, default_value = "http://127.0.0.1:4000")]
        base: String,
    },
    /// POST a §67 typed action. WRITES to an authoritative external record.
    Act {
        #[arg(long, default_value = "http://127.0.0.1:4000")]
        base: String,
        /// e.g. `document.create`.
        action_type: String,
        #[arg(long)]
        actor: String,
        #[arg(long)]
        acting_role: String,
        #[arg(long)]
        organization: String,
        #[arg(long)]
        reason: String,
        /// Supplied by the CALLER; KF refuses fewer than 8 characters.
        #[arg(long)]
        idempotency_key: String,
        /// What the action targets. KF validates these; an empty envelope
        /// would 400 at the server, which tells the caller nothing useful.
        #[arg(long = "target-id")]
        target_ids: Vec<String>,
        /// The action payload, as JSON.
        #[arg(long, default_value = "{}")]
        payload: String,
        /// Required. Without it this refuses rather than writes.
        #[arg(long)]
        confirm_write: bool,
    },
}

#[derive(Subcommand, Debug)]
enum EvidenceCommand {
    /// Run the gates the Warrant's assurance atom cites and mint each §44.6
    /// receipt into `docs/warrants/<alias>/gate-runs/`, bound to the Warrant's
    /// current contract digest. Refuses a Warrant that cites no gate, and a
    /// named gate the Warrant does not cite.
    Record {
        /// The Warrant's local alias.
        alias: String,
        /// One cited gate (`<id>@<version>`). Defaults to every cited gate.
        #[arg(long)]
        gate: Option<String>,
    },
}

#[derive(clap::Subcommand, Debug)]
enum SasCommand {
    /// Record the document as it stands as a proposed revision (§101.2).
    Propose {
        /// e.g. `0.1.0-draft.1`, `1.0.0`.
        version: String,
    },
    /// Emit the acceptance request, or ingest a human's signed response.
    Accept {
        version: String,
        /// A signed response to ingest. Without it, the request is emitted.
        #[arg(long)]
        response: Option<Utf8PathBuf>,
    },
    /// §106 of the document versus a candidate document; refuses a removed id.
    Diff { candidate: Utf8PathBuf },
    /// Every revision on record, and which one the document matches.
    Status,
}

#[derive(clap::Subcommand, Debug)]
enum BonsaiCommand {
    /// Check a Warrant-bound change with a fixed Bonsai binary.
    Check {
        /// Warrant authorizing this change.
        #[arg(long)]
        warrant: String,
        /// Merge-base commit for the candidate diff.
        #[arg(long)]
        base: String,
        /// Candidate commit. It must be checked out at HEAD.
        #[arg(long)]
        head: String,
        /// Explicit Bonsai binary. The Warrant never supplies an executable.
        #[arg(long, value_name = "BONSAI_BINARY")]
        bonsai: Utf8PathBuf,
    },
    /// Validate a completed passing Bonsai evidence document without rerunning Bonsai.
    VerifyEvidence {
        /// Repository-relative evidence document emitted by `war bonsai check`.
        #[arg(long)]
        evidence: Utf8PathBuf,
    },
}

/// Exit codes. §76.4 wants machine-usable output; a caller distinguishing
/// "your input was wrong" from "the Warrant is not sound" needs more than 0/1.
///
/// Codes are added when a command can actually produce them. An exit code the
/// binary never returns is a promise to callers that nothing keeps.
pub(crate) const EXIT_OK: u8 = 0;
pub(crate) const EXIT_DIAGNOSTIC: u8 = 1;
pub(crate) const EXIT_NOT_READY: u8 = 2;

#[derive(Parser)]
#[command(
    name = "war",
    about = "Work Authorization Records — author, check, and compile Warrants.",
    version,
    long_about = None,
)]
struct Cli {
    /// Machine output (SAS §76.4): one `oh.war/report/v1` envelope on stdout,
    /// for every command. Errors are envelopes too.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize repository configuration and directories (§71.1).
    Init {
        /// Namespace prefixing every local alias, e.g. `OW` in `OW-WAR-0001`.
        #[arg(long)]
        namespace: String,
        /// Project name. Defaults to the directory name.
        #[arg(long)]
        name: Option<String>,
        /// Repository root. Defaults to the current directory.
        #[arg(long)]
        root: Option<Utf8PathBuf>,
    },
    /// Write the AGENTS.md this repository ships, for the repository's
    /// namespace. `war init` writes it once; this rewrites (--force) or prints it.
    AgentsMd {
        /// Print to stdout instead of writing.
        #[arg(long)]
        stdout: bool,
        /// Overwrite an existing AGENTS.md.
        #[arg(long)]
        force: bool,
    },
    /// Create a draft Warrant (§71.2).
    New {
        /// The Warrant's title.
        title: String,
        /// Composition profile (§16.3).
        #[arg(long, default_value = "delivery")]
        profile: String,
    },
    /// Validate deterministically, without any agent (§71.7).
    Check {
        /// A single Warrant's local alias. Defaults to the whole corpus.
        alias: Option<String>,
        /// Also compare committed generated views against a fresh compilation.
        #[arg(long)]
        generated: bool,
    },
    /// Inspect or run local gate definitions (§44).
    Gate {
        /// Execute the gates rather than listing them.
        #[arg(long)]
        run: bool,
        /// A single gate id or `<id>@<version>`. Defaults to every gate.
        #[arg(long)]
        gate: Option<String>,
        /// Record the run as evidence under the receipts path (§44.6).
        ///
        /// Off by default. Running a gate to PROBE its behaviour — as the
        /// conformance battery does, by corrupting the definition on purpose —
        /// must not overwrite the evidentiary record for the subject.
        #[arg(long)]
        record: bool,
        /// Subject digests bound into a receipt. Requires --record.
        #[arg(long = "subject-digest")]
        subject_digests: Vec<String>,
        /// Immutable evidence references bound into a receipt. Requires --record.
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
    },
    /// Bind a Warrant's machine scope to Bonsai evidence.
    Bonsai {
        #[command(subcommand)]
        command: BonsaiCommand,
    },
    /// Build a drafting request for an agent (§71.3, §75.2).
    ///
    /// Emits the canonical request and stops: this build ships no agent, and a
    /// seam with nothing on the other side should say so rather than pretend.
    Plan {
        /// What the Warrant should accomplish.
        request: String,
        #[arg(long, default_value = "delivery")]
        profile: String,
        #[arg(long, default_value = "basic")]
        assurance: String,
        /// A Draft Proposal returned by an agent, to validate against §74.4.
        #[arg(long)]
        proposal: Option<Utf8PathBuf>,
        /// Record that §74.4 steps 5 and 6 (semantic diff, review) happened.
        #[arg(long)]
        reviewed: bool,
    },
    /// Lower a computational Warrant's stage graph into a BLUT PlanSpec (§49).
    Blut {
        /// The Warrant's local alias.
        alias: String,
        /// Path to a real BLUT binary. When given, the lowered PlanSpec is
        /// handed to `<binary> plan check --json` and BLUT's own verdict is
        /// reported. Without it the lowering is only structurally faithful to
        /// a schema, which is a weaker claim.
        #[arg(long, value_name = "BLUT_BINARY")]
        verify: Option<camino::Utf8PathBuf>,
        /// Write the lowered PlanSpec here, so it can be handed to
        /// `blut plan run`. Without this the spec exists only inside the
        /// report, and running it means copying JSON out of prose.
        #[arg(long, value_name = "PATH")]
        emit: Option<camino::Utf8PathBuf>,
    },
    /// Compile a Stage Dispatch for one stage of a Warrant (§47).
    ///
    /// The only packet a stateless actor receives. Built from the Warrant's
    /// own atoms; digested under §65's Dispatch domain; never executed here.
    Dispatch {
        /// The Warrant's local alias.
        alias: String,
        /// The stage id, e.g. `STAGE-003`.
        stage: String,
        /// §52: initial, replay, repair, restart.
        #[arg(long, default_value = "initial")]
        attempt_kind: String,
        /// Prior failure evidence refs. Required for a repair (§52.3).
        #[arg(long = "prior-failure")]
        prior_failure: Vec<String>,
        /// Write the packet here instead of stdout.
        #[arg(long, value_name = "PATH")]
        emit: Option<Utf8PathBuf>,
    },

    /// Evaluate §56.1's thirteen resolution requirements without recording one.
    /// Correct a delivered artifact of a RESOLVED Warrant (OW-WAR-0064): emit
    /// the request naming the pinned digest and the file's digest now, or
    /// ingest a human's signed correction. The pin in `deliverables.toml` is
    /// never edited; the correction is appended beside it and the superseded
    /// digest stays visible.
    Correct {
        /// The Warrant's local alias.
        alias: String,
        /// The deliverable id, e.g. `D-002`.
        deliverable_id: String,
        /// A human's signed correction to ingest. Without it, the request is emitted.
        #[arg(long)]
        response: Option<camino::Utf8PathBuf>,
    },
    Resolve {
        /// The Warrant's local alias.
        alias: String,
        /// Report the thirteen §56.1 requirements and stop.
        #[arg(long)]
        dry_run: bool,
        /// A resolver's signed response to ingest (§56.2). Without it and
        /// without --dry-run, the resolution REQUEST is emitted: what a
        /// signature would bind, which outcomes §38.6 permits, and who may sign.
        #[arg(long, conflicts_with = "dry_run")]
        response: Option<camino::Utf8PathBuf>,
    },
    /// Read a Warrant's local draft journal (§66), or reconstruct one from its
    /// records. There is no way to append by hand: events come from the
    /// commands that change records.
    Journal {
        /// The Warrant's local alias.
        alias: String,
        /// Write the events the existing records imply, each marked
        /// `backfilled`. Refused on a journal that already has events.
        #[arg(long)]
        backfill: bool,
    },
    /// Record gate runs as committed evidence for a Warrant (§44.6).
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    /// Render one of §17.5's projections (§17.5).
    Show {
        /// The Warrant's local alias.
        alias: String,
        /// Which projection. Defaults to the full Warrant.
        #[arg(long, default_value = "full_warrant")]
        view: String,
    },
    /// Semantic difference between the committed compilation and a fresh one (§71.10).
    Diff {
        /// The Warrant's local alias.
        alias: String,
        /// A canonical JSON file to compare against. Defaults to the committed one.
        #[arg(long)]
        from: Option<Utf8PathBuf>,
    },
    /// Import a legacy ADR corpus (§96), discharging OW-WAR-0043.
    Migrate {
        /// Directory of ADR files named `NNNN-*.md`.
        #[arg(long)]
        corpus: Utf8PathBuf,
        /// The one named, frozen commit the corpus is read at (OBL-001).
        #[arg(long)]
        commit: String,
        /// Where to write the import artifact.
        #[arg(long, default_value = "artifacts/lamquant-adr-import.json")]
        out: Utf8PathBuf,
        /// Compare against the existing artifact instead of writing it — OBL-001's
        /// "a re-run at that SHA producing byte-identical output".
        #[arg(long)]
        verify: bool,
        /// NEGATIVE CONTROL. Attempt to promote each completion line to a
        /// resolution, so §96.3's refusal is observable from outside the binary.
        /// It must always fail; a build where this succeeds is the defect.
        #[arg(long)]
        attempt_promotion: bool,
    },

    /// §68 portable export and round trip.
    Export {
        /// The Warrant's local alias.
        alias: String,
        /// Write the package here even if §68.2 contents are missing. Never
        /// reports the result as valid.
        #[arg(long)]
        force: bool,
        /// §68.3 — export, re-export, compare. Without --reconnect the
        /// comparison is refused as vacuous.
        #[arg(long)]
        round_trip: bool,
        /// Record that the preserved evidence bytes were reconnected.
        #[arg(long)]
        reconnect: bool,
    },

    /// §67 Knowledge Fabric seam. `health` reads; `act` WRITES and needs
    /// --confirm-write.
    Kf {
        #[command(subcommand)]
        cmd: KfCommand,
    },

    /// §94 telemetry baseline, §95 untracked-work candidates, §100 metrics.
    Telemetry {
        /// The commit this baseline is taken at (§94, OBL-001).
        #[arg(long)]
        commit: String,
        /// Where to write the baseline artifact.
        #[arg(long, default_value = "artifacts/telemetry-baseline.json")]
        out: camino::Utf8PathBuf,
        /// Compare against the existing artifact instead of writing it.
        #[arg(long)]
        verify: bool,
        /// §95 — relate an untracked-work candidate to a Warrant. Requires
        /// --reviewer; an empty one is refused as a fabrication.
        #[arg(long, value_name = "SCOPE")]
        attach: Option<String>,
        /// The Warrant the candidate belongs to.
        #[arg(long, value_name = "ALIAS", default_value = "")]
        warrant: String,
        /// Who reviewed the attribution. §95 will not accept an empty one —
        /// pass `--reviewer <name>`. It is not defaulted to the acting user on
        /// purpose: a review nobody performed is the fabrication §95 forbids.
        #[arg(long, default_value = "")]
        reviewer: String,
    },

    /// §46 independent verification. Emits a request; ingests verdicts.
    ///
    /// This command never verifies anything itself — the actor that produced
    /// the work cannot be the actor that clears it.
    Verify {
        /// The Warrant's local alias.
        alias: String,
        /// The performer whose work is under verification.
        #[arg(long, default_value = "unknown")]
        performer: String,
        /// A verifier's response to ingest. Without it, the request is emitted.
        #[arg(long)]
        response: Option<Utf8PathBuf>,
    },

    /// §28.4 authorization. Emits a request; ingests a human's signature.
    ///
    /// Like `verify`, this command never decides anything itself: §27.2 says an
    /// agent SHALL NOT authorize a proposed WAR, and ingestion refuses every
    /// agent named in a response regardless of what the response claims.
    Authorize {
        /// The Warrant's local alias.
        alias: String,
        /// A signed response to ingest. Without it, the request is emitted.
        #[arg(long)]
        response: Option<Utf8PathBuf>,
    },

    /// Sign what is waiting — authorize, resolve, or accept a SAS revision —
    /// from one screen at a terminal.
    ///
    /// Refuses without a TTY (§27.2: an agent's shell has none). Drafts the
    /// response from the record's own facts, shows what is being signed, asks
    /// once, and on `y` runs the same ingest a hand-written response would.
    /// There is no `--yes`.
    Sign {
        /// A Warrant alias, or a SAS version. Omit with --list or --all.
        target: Option<String>,
        /// Show what awaits a signature and exit. Needs no terminal.
        #[arg(long)]
        list: bool,
        /// Sign every pending act, one prompt each.
        #[arg(long)]
        all: bool,
        /// Sign as this actor (required when more than one is eligible).
        #[arg(long = "as")]
        actor: Option<String>,
        /// Your own words, appended to the drafted meaning.
        #[arg(long)]
        meaning: Option<String>,
        /// Resolution outcome when §38.6 forbids `satisfied`:
        /// not_satisfied, cancelled, blocked.
        #[arg(long)]
        outcome: Option<String>,
        /// §101.3 ADR reference for an architecture-changing SAS revision.
        #[arg(long)]
        adr: Option<String>,
        /// none | separate_role | organizational (§27.4).
        #[arg(long, default_value = "separate_role")]
        independence: String,
        /// Open $VISUAL/$EDITOR on the drafted response before the prompt.
        #[arg(long)]
        edit: bool,
        /// Render what would be signed and stop. No terminal needed; writes
        /// nothing. Read this from anywhere; sign it from a terminal.
        #[arg(long)]
        show: bool,
        /// Sign with your ssh key instead of a terminal prompt. Needs
        /// `ssh_principal` on your roles.toml entry and a matching line in
        /// docs/authority/allowed_signers. Load the key with `ssh-add -c` so
        /// every signature asks YOU through a dialog; `war` cannot check that.
        #[arg(long)]
        ssh_sign: bool,
        /// Verify a recorded response's .sig sidecar and stop. Writes nothing.
        #[arg(long)]
        verify: bool,
        /// For a correction (`<alias>/<D-id>`): behaviour-change | added-refusal.
        #[arg(long)]
        kind: Option<String>,
    },

    /// The SAS as a controlled document (§101): propose, accept, diff, status.
    Sas {
        #[command(subcommand)]
        command: SasCommand,
    },

    /// Every file a Warrant's `deliverables.toml` pins, with the Warrant's
    /// state — ask BEFORE editing; a resolved Warrant's pin moves only through
    /// `war correct` (OW-WAR-0064).
    Pins {
        /// Only pins held by a resolution.
        #[arg(long)]
        resolved_only: bool,
    },
    /// What should happen next, and whose act it is. An agent is never handed
    /// a signing act; it is told that a human must sign, and how.
    Next,
    /// Where the corpus stands, from records (§17.5 `status`; §34.3; §98).
    ///
    /// Bare `war status` is the corpus projection. `war status <alias>` is the
    /// per-Warrant form §72.5 names. Every count is a ladder; nothing is a
    /// percentage.
    Status {
        /// A Warrant's local alias. Omit for the whole corpus. (`--json` is the
        /// global flag; for the corpus it yields the canonical projection.)
        alias: Option<String>,
    },

    /// Compile the configured projections (§71.8).
    Compile {
        /// A single Warrant's local alias. Defaults to the whole corpus.
        alias: Option<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mode = output::Mode::from_flag(cli.json);
    match run(cli) {
        Ok(code) => ExitCode::from(code),
        Err(report) => {
            // §76.2: an explicit diagnostic naming what was wrong and where,
            // never a bare "error". Under --json, an envelope on stdout.
            output::error(mode, &report.to_string());
            ExitCode::from(EXIT_DIAGNOSTIC)
        }
    }
}

fn run(cli: Cli) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = output::Mode::from_flag(cli.json);
    match cli.command {
        Command::Init {
            namespace,
            name,
            root,
        } => {
            init::run(&namespace, name.as_deref(), root)?;
            Ok(EXIT_OK)
        }

        Command::AgentsMd { stdout, force } => {
            let repository = repo::Repository::discover(None)?;
            let ns = repository.config.project.namespace.as_str().to_owned();
            if stdout {
                print!("{}", init::render_agents_md(&ns));
                return Ok(EXIT_OK);
            }
            match init::write_agents_md(&repository.root, &ns, force)? {
                true => {
                    output::emit(
                        mode,
                        "agents_md",
                        "wrote AGENTS.md",
                        serde_json::json!({"path": "AGENTS.md", "namespace": ns, "written": true}),
                    );
                    Ok(EXIT_OK)
                }
                false => {
                    let mut report = diagnostic::Report::default();
                    report.push(diagnostic::Diagnostic::error(
                        "agents-md.exists",
                        "AGENTS.md".to_owned(),
                        "AGENTS.md exists; pass --force to replace it, or --stdout to read the template",
                    ));
                    output::finish(mode, "agents_md", &report, None);
                    Ok(EXIT_DIAGNOSTIC)
                }
            }
        }
        Command::New { title, profile } => {
            let profile: Profile = profile.parse()?;
            let repository = repo::Repository::discover(None)?;
            let dir = new::run(&repository, &title, profile)?;
            let rel = repository.relative(&dir);
            let alias = dir.file_name().unwrap_or_default().to_owned();
            output::emit(
                mode,
                "new",
                &format!("created {rel}\nedit its atoms, then run `war check`"),
                serde_json::json!({"alias": alias, "dir": rel, "profile": profile.to_string()}),
            );
            Ok(EXIT_OK)
        }

        Command::Migrate {
            corpus,
            commit,
            out,
            verify,
            attempt_promotion,
        } => {
            let artifact = migrate::import(&corpus, &commit, attempt_promotion)?;
            migrate::write_or_verify(&artifact, &out, verify)?;
            migrate::print(&artifact);
            // OBL-002 and OBL-003 are countable, so they decide the exit code.
            // §96.3's other half — that a gate cannot arrive qualified — is a type
            // invariant rather than a count, and has no way to be violated here.
            Ok(if migrate::obligations_met(&artifact) {
                EXIT_OK
            } else {
                EXIT_NOT_READY
            })
        }

        Command::Gate {
            run,
            gate,
            record,
            subject_digests,
            evidence_refs,
        } => {
            if !record && (!subject_digests.is_empty() || !evidence_refs.is_empty()) {
                return Err(Box::new(repo::RepoError::Message(
                    "--subject-digest and --evidence-ref require --record".to_owned(),
                )));
            }
            let repository = repo::Repository::discover(None)?;
            let report = gate_cmd::run(
                &repository,
                run,
                gate.as_deref(),
                record,
                &subject_digests,
                &evidence_refs,
                None,
            )?;
            let _ = output::finish(mode, "gate", &report, None);
            // §44.1 and RQ-054: an unaskable gate is NOT a pass, and is not a
            // failure either. `is_ready()` blocks on unknowns, so both land on a
            // non-zero exit without the two being conflated in the report.
            Ok(if report.is_ready() {
                EXIT_OK
            } else {
                EXIT_NOT_READY
            })
        }
        Command::Bonsai { command } => match command {
            BonsaiCommand::Check {
                warrant,
                base,
                head,
                bonsai: binary,
            } => {
                let repository = repo::Repository::discover(None)?;
                let evidence = bonsai::check(&repository, &warrant, &base, &head, &binary)?;
                println!("{}", serde_json::to_string_pretty(&evidence)?);
                Ok(match evidence.verdict {
                    bonsai::EvidenceVerdict::Pass => EXIT_OK,
                    bonsai::EvidenceVerdict::Unknown => EXIT_NOT_READY,
                    bonsai::EvidenceVerdict::Fail => EXIT_DIAGNOSTIC,
                })
            }
            BonsaiCommand::VerifyEvidence { evidence } => {
                let repository = repo::Repository::discover(None)?;
                bonsai::verify_evidence_file(&repository, &evidence)?;
                println!("Bonsai evidence is a valid passing v1 document");
                Ok(EXIT_OK)
            }
        },
        Command::Export {
            alias,
            force,
            round_trip,
            reconnect,
        } => {
            let repository = repo::Repository::discover(None)?;
            if round_trip {
                let rt = export::round_trip(&repository, &alias, reconnect)?;
                match rt.verify() {
                    Ok(()) => {
                        println!(
                            "{alias}: §68.3 round trip verified ({})",
                            rt.original_digest
                        );
                        return Ok(EXIT_OK);
                    }
                    Err(e) => {
                        return Err(Box::new(repo::RepoError::Message(format!("{alias}: {e}"))));
                    }
                }
            }
            let (package, missing) = export::assemble(&repository, &alias)?;
            match package.validate() {
                Ok(()) => {
                    println!(
                        "{alias}: §68.2 export complete ({} record(s))",
                        package.embedded_record_count
                    );
                    Ok(EXIT_OK)
                }
                Err(e) => {
                    println!(
                        "{alias}: §68.2 export INCOMPLETE — {} of {} required contents absent:",
                        missing.len(),
                        openwarrant_core::journal::EXPORT_CONTENTS.len() - 1
                    );
                    for m in &missing {
                        println!("  · {m}");
                    }
                    if force {
                        println!(
                            "\n--force: the package would be written anyway. It is NOT a valid \
                             §68 export and is not reported as one."
                        );
                        return Ok(EXIT_NOT_READY);
                    }
                    Err(Box::new(repo::RepoError::Message(format!("{alias}: {e}"))))
                }
            }
        }

        Command::Kf { cmd } => match cmd {
            KfCommand::Health { base } => {
                let client = kf::Client::new(&base)?;
                println!("{}", client.health()?);
                Ok(EXIT_OK)
            }
            KfCommand::Act {
                base,
                action_type,
                actor,
                acting_role,
                organization,
                reason,
                idempotency_key,
                target_ids,
                payload,
                confirm_write,
            } => {
                if !confirm_write {
                    return Err(Box::new(repo::RepoError::Message(
                        "refusing to POST a §67 action without --confirm-write. This writes \
                         to an authoritative external record, and a seam that is easy to \
                         reach by accident is how a diagnostic becomes a fabrication."
                            .to_owned(),
                    )));
                }
                let client = kf::Client::new(&base)?;
                let body = client.post_action(
                    &action_type,
                    &kf::Actor {
                        actor,
                        acting_role,
                        organization,
                    },
                    &kf::ActionEnvelope {
                        target_ids,
                        payload: serde_json::from_str(&payload).map_err(|e| {
                            repo::RepoError::Message(format!("--payload is not JSON: {e}"))
                        })?,
                        reason,
                        idempotency_key,
                    },
                )?;
                println!("{body}");
                Ok(EXIT_OK)
            }
        },

        Command::Telemetry {
            commit,
            out,
            verify,
            attach,
            warrant,
            reviewer,
        } => {
            let repository = repo::Repository::discover(None)?;
            if let Some(scope) = attach {
                println!("{}", telemetry::attach(&scope, &warrant, &reviewer)?);
                return Ok(EXIT_OK);
            }
            let baseline = telemetry::take(&repository, &commit)?;
            let rendered = telemetry::render(&baseline)?;
            if verify {
                let existing = std::fs::read_to_string(&out)
                    .map_err(|e| repo::RepoError::Message(format!("cannot read {out}: {e}")))?;
                // Compared EXACTLY. Trimming would let whitespace drift through
                // while the doc claims byte-for-byte agreement.
                if existing == rendered {
                    println!("telemetry baseline at {commit} is unchanged");
                    return Ok(EXIT_OK);
                }
                return Err(Box::new(repo::RepoError::Message(format!(
                    "the committed baseline differs from a fresh one at {commit}. A baseline \
                     is a record of one moment; if the corpus moved, take a NEW one rather \
                     than overwriting the old."
                ))));
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    repo::RepoError::Message(format!("cannot create {parent}: {e}"))
                })?;
            }
            std::fs::write(&out, &rendered)
                .map_err(|e| repo::RepoError::Message(format!("cannot write {out}: {e}")))?;
            let untaken = baseline
                .measures
                .values()
                .filter(|m| matches!(m, telemetry::Measure::NotYet { .. }))
                .count();
            println!(
                "telemetry baseline written to {out}\n  {} of {} §94 measures taken; {untaken} \
                 recorded `not_measurable_yet` with a reason\n  {} §95 untracked-work \
                 candidate(s)\n  {} §100 metrics, every one `no baseline` — one measurement \
                 supports no delta",
                baseline.measures.len() - untaken,
                baseline.measures.len(),
                baseline.untracked_work_candidates.len(),
                baseline.success_metrics.len()
            );
            Ok(EXIT_OK)
        }

        Command::Plan {
            request,
            profile,
            assurance,
            proposal,
            reviewed,
        } => {
            let repository = repo::Repository::discover(None)?;

            // The return half of §75.2's seam: validate what an agent sent back.
            if let Some(path) = proposal {
                let json = std::fs::read_to_string(&path)
                    .map_err(|e| repo::RepoError::Message(format!("cannot read {path}: {e}")))?;
                // Every `war://` this corpus can resolve, so an invented one can
                // be told from a real one by more than its shape.
                //
                // Built only when the proposal actually cites something. Loading
                // the whole corpus to answer a question nobody asked is a cost
                // every `war plan` would pay for the benefit of the few that
                // carry relations.
                let cites_relations = serde_json::from_str::<serde_json::Value>(&json)
                    .ok()
                    .and_then(|v| {
                        v.get("proposed_relations")
                            .and_then(|r| r.as_array().map(|a| !a.is_empty()))
                    })
                    .unwrap_or(false);

                let mut known = std::collections::BTreeSet::new();
                if cites_relations {
                    for dir in repository.warrant_dirs()? {
                        // A Warrant that fails to LOAD is not a Warrant that does
                        // not exist. Swallowing the error here would report a real
                        // reference as invented — the wrong diagnosis, and the
                        // more alarming one, for a corrupt file.
                        let loaded = repository.load_warrant(&dir).map_err(|e| {
                            repo::RepoError::Message(format!(
                                "cannot resolve proposal references: {} failed to load \
                                 ({e}). Refusing to report a reference as invented when \
                                 the corpus could not be read.",
                                repository.relative(&dir)
                            ))
                        })?;
                        if let Some(v) = loaded.validated {
                            known.insert(format!("war://{}", v.uuid));
                        }
                    }
                }
                let (parsed, pipeline) = show::plan::validate_proposal(&json, reviewed, &known)?;
                match pipeline.may_apply() {
                    Ok(()) => {
                        println!(
                            "draft proposal is applicable: {} atom operation(s), \
                             {} ADR draft(s)",
                            parsed.atom_operations.len(),
                            parsed.proposed_adr_drafts.len()
                        );
                        Ok(EXIT_OK)
                    }
                    Err(e) => {
                        // NOT an error exit for a well-formed proposal awaiting
                        // review: §74.4 step 6 is a human step, and reporting
                        // "not yet reviewed" as a failure would train people to
                        // pass --reviewed to make the message go away.
                        println!("draft proposal parsed and validated, but not applicable yet");
                        println!("  {e}");
                        Ok(EXIT_NOT_READY)
                    }
                }
            } else {
                let req = show::plan::DraftRequest {
                    api_version: "oh.war/draft-request/v1".to_owned(),
                    user_request: request,
                    namespace: repository.config.project.namespace.as_str().to_owned(),
                    profile,
                    assurance,
                    existing_warrants: repository
                        .warrant_dirs()?
                        .iter()
                        .filter_map(|d| d.file_name().map(ToOwned::to_owned))
                        .collect(),
                    existing_adrs: vec![],
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&req).expect("request serializes")
                );
                eprintln!(
                    "\nThis build ships no drafting agent. Pipe this request to one \
                 speaking `war-agent --protocol oh.war/agent-drafter/v1` (§75.2), \
                 then return its Draft Proposal with `war plan --proposal <file>`; \
                 it must clear §74.4's eight steps before anything is written."
                );
                Ok(EXIT_OK)
            }
        }
        Command::Blut {
            alias,
            verify,
            emit,
        } => {
            let repository = repo::Repository::discover(None)?;
            let report = blut::lower(&repository, &alias, verify.as_deref(), emit.as_deref())?;
            Ok(output::finish(mode, "blut", &report, None))
        }
        Command::Dispatch {
            alias,
            stage,
            attempt_kind,
            prior_failure,
            emit,
        } => {
            let repository = repo::Repository::discover(None)?;
            let kind = attempt_kind
                .parse::<openwarrant_core::execution::AttemptKind>()
                .map_err(|e| repo::RepoError::Message(e.to_string()))?;
            let report = dispatch::run(
                &repository,
                &alias,
                &stage,
                kind,
                &prior_failure,
                emit.as_deref(),
            )?;
            // The packet is the only thing on stdout when it goes there. An
            // actor piping `war dispatch` into a parser must get canonical JSON
            // (oh.war/stage-dispatch/v1, its own schema) and nothing else — so
            // the report goes to STDERR in that case: human lines in human
            // mode, one envelope in --json mode. The packet's printer lives in
            // dispatch.rs, a pinned deliverable of resolved OW-WAR-0056; moving
            // the packet inside the envelope is a correction for another day.
            if emit.is_none() {
                match mode {
                    output::Mode::Human => {
                        for d in &report.diagnostics {
                            eprintln!("{d}");
                        }
                    }
                    output::Mode::Json => {
                        eprintln!("{}", output::envelope("dispatch", &report, None));
                    }
                }
                Ok(output::exit_code(&report))
            } else {
                // With --emit the packet is on disk, so the report may take
                // stdout — as an envelope under --json.
                Ok(output::finish(mode, "dispatch", &report, None))
            }
        }
        Command::Correct {
            alias,
            deliverable_id,
            response,
        } => {
            let repository = repo::Repository::discover(None)?;
            match response {
                Some(path) => {
                    let report = correct::ingest(&repository, &alias, &deliverable_id, &path)?;
                    Ok(output::finish(mode, "correct", &report, None))
                }
                None => {
                    let request = correct::request(&repository, &alias, &deliverable_id)?;
                    output::emit(
                        mode,
                        "correct.request",
                        &toml::to_string_pretty(&request).map_err(|e| {
                            repo::RepoError::Message(format!("could not render the request: {e}"))
                        })?,
                        output::value(&request),
                    );
                    if !request.resolved {
                        eprintln!(
                            "# {alias} is NOT resolved: regenerate deliverables.toml instead. A \
                             correction is for a pin a resolution holds."
                        );
                        return Ok(EXIT_NOT_READY);
                    }
                    if !request.drift {
                        eprintln!(
                            "# {alias}/{deliverable_id}: the file is at the digest on record; nothing to correct."
                        );
                        return Ok(EXIT_NOT_READY);
                    }
                    eprintln!(
                        "# {alias}/{deliverable_id}: {} → {} (correction {}). Sign with `war sign {alias}/{deliverable_id}`.",
                        request.chain_head, request.current_digest, request.next_sequence
                    );
                    Ok(EXIT_OK)
                }
            }
        }
        Command::Resolve {
            alias,
            dry_run,
            response,
        } => {
            let repository = repo::Repository::discover(None)?;
            if dry_run {
                let report = resolve::run(&repository, &alias)?;
                return Ok(output::finish(mode, "resolve.dry_run", &report, None));
            }
            match response {
                Some(path) => {
                    let report = resolution_cmd::ingest(&repository, &alias, &path)?;
                    Ok(output::finish(mode, "resolve", &report, None))
                }
                None => {
                    let request = resolution_cmd::request(&repository, &alias)?;
                    output::emit(
                        mode,
                        "resolve.request",
                        &toml::to_string_pretty(&request).map_err(|e| repo::RepoError::Io {
                            context: "could not render the resolution request".to_owned(),
                            source: std::io::Error::other(e.to_string()),
                        })?,
                        output::value(&request),
                    );
                    if request.requirements_met {
                        eprintln!(
                            "# {alias}: all 13 §56.1 requirements met. Permitted outcomes: {}. \
                             Eligible resolvers: {}.",
                            request.permitted_outcomes.join(", "),
                            if request.eligible_resolvers.is_empty() {
                                "NOBODY — docs/authority/roles.toml grants `resolver` to no human"
                                    .to_owned()
                            } else {
                                request.eligible_resolvers.join(", ")
                            }
                        );
                        Ok(EXIT_OK)
                    } else {
                        eprintln!(
                            "# {alias}: {} of 13 §56.1 requirements unmet; a response would be refused.",
                            request.unmet.len()
                        );
                        Ok(EXIT_NOT_READY)
                    }
                }
            }
        }
        Command::Journal { alias, backfill } => {
            let repository = repo::Repository::discover(None)?;
            if backfill {
                let report = journal_cmd::backfill(&repository, &alias)?;
                Ok(output::finish(mode, "journal", &report, None))
            } else {
                let text = journal_cmd::show(&repository, &alias)?;
                match mode {
                    output::Mode::Human => print!("{text}"),
                    output::Mode::Json => output::emit(
                        mode,
                        "journal",
                        &text,
                        serde_json::json!({"alias": alias, "rendered": text}),
                    ),
                }
                Ok(EXIT_OK)
            }
        }
        Command::Evidence { command } => {
            let repository = repo::Repository::discover(None)?;
            let EvidenceCommand::Record { alias, gate } = command;
            let report = evidence::record(&repository, &alias, gate.as_deref())?;
            Ok(output::finish(mode, "evidence", &report, None))
        }
        Command::Sas { command } => {
            let repository = repo::Repository::discover(None)?;
            let ready = |report: diagnostic::Report| Ok(output::finish(mode, "sas", &report, None));
            match command {
                SasCommand::Propose { version } => ready(sas::propose(&repository, &version)?),
                SasCommand::Accept { version, response } => match response {
                    Some(path) => ready(sas::accept_ingest(&repository, &version, &path)?),
                    None => {
                        let request = sas::accept_request(&repository, &version)?;
                        output::emit(
                            mode,
                            "accept.accept.request",
                            &toml::to_string_pretty(&request).map_err(|e| {
                                repo::RepoError::Message(format!(
                                    "could not render the request: {e}"
                                ))
                            })?,
                            output::value(&request),
                        );
                        eprintln!(
                            "# SAS {} at sha256:{} — architecture-changing: {}{}",
                            request.version,
                            &request.sha256[..12],
                            request.architecture_changing,
                            if request.adr_required {
                                " (an adr_ref is REQUIRED, §101.3)"
                            } else {
                                ""
                            }
                        );
                        if request.eligible_acceptors.is_empty() {
                            eprintln!(
                                "# NOBODY may accept this: docs/authority/roles.toml grants the authorizer\n\
                                 # role to no one. Only a human may write that file."
                            );
                        } else {
                            eprintln!(
                                "# May be accepted by: {}",
                                request.eligible_acceptors.join(", ")
                            );
                        }
                        eprintln!(
                            "# Fill in accepted_by, acting_role, meaning, effective_time, then:\n\
                             #   war sas accept {} --response <file>",
                            request.version
                        );
                        Ok(EXIT_OK)
                    }
                },
                SasCommand::Diff { candidate } => ready(sas::diff(&repository, &candidate)?),
                SasCommand::Status => ready(sas::status(&repository)?),
            }
        }
        Command::Pins { resolved_only } => {
            let repository = repo::Repository::discover(None)?;
            let pins = pins::list(&repository, resolved_only)?;
            output::emit(
                mode,
                "pins",
                pins::render(&pins).trim_end(),
                output::value(&pins),
            );
            Ok(EXIT_OK)
        }
        Command::Next => {
            let repository = repo::Repository::discover(None)?;
            let next = next::run(&repository)?;
            output::emit(
                mode,
                "next",
                next::render(&next).trim_end(),
                output::value(&next),
            );
            Ok(EXIT_OK)
        }
        Command::Status { alias } => {
            let repository = repo::Repository::discover(None)?;
            match alias {
                Some(alias) => {
                    let rendered = show::run(&repository, &alias, "status")?;
                    output::emit(
                        mode,
                        "status",
                        &rendered,
                        serde_json::json!({"alias": alias, "view": "status", "rendered": rendered}),
                    );
                }
                None => match mode {
                    // The corpus projection IS canonical JSON already; under
                    // --json it rides inside the envelope as `result`, so the
                    // committed CORPUS_STATUS.json (written by `compile`) and
                    // this output agree on the payload.
                    output::Mode::Json => {
                        let (_, text) = status::corpus_status_json(&repository)?;
                        let value: serde_json::Value = serde_json::from_str(&text)
                            .map_err(|e| repo::RepoError::Message(format!("corpus status: {e}")))?;
                        output::emit(mode, "status", &text, value);
                    }
                    output::Mode::Human => {
                        let (_, text) = status::corpus_status_md(&repository)?;
                        println!("{text}");
                    }
                },
            }
            Ok(EXIT_OK)
        }
        Command::Show { alias, view } => {
            let repository = repo::Repository::discover(None)?;
            let rendered = show::run(&repository, &alias, &view)?;
            output::emit(
                mode,
                "show",
                &rendered,
                serde_json::json!({"alias": alias, "view": view, "rendered": rendered}),
            );
            Ok(EXIT_OK)
        }
        Command::Diff { alias, from } => {
            let repository = repo::Repository::discover(None)?;
            let report = show::diff(&repository, &alias, from.as_ref())?;
            // A diff is information, not a verdict: exit 0 whatever it found.
            let _ = output::finish(mode, "diff", &report, None);
            Ok(EXIT_OK)
        }
        Command::Check { alias, generated } => {
            let repository = repo::Repository::discover(None)?;
            let report = check::run(&repository, alias.as_deref(), generated)?;
            // A non-zero exit for an unsound Warrant is what lets CI gate on it.
            Ok(output::finish(mode, "check", &report, None))
        }

        Command::Verify {
            alias,
            performer,
            response,
        } => {
            let repository = repo::Repository::discover(None)?;
            match response {
                // Ingest verdicts something else produced.
                Some(path) => {
                    let report = verify::ingest(&repository, &alias, &path)?;
                    Ok(output::finish(mode, "verify", &report, None))
                }
                // Emit the request and stop. §75.2: a seam with nothing on the
                // other side should say so rather than pretend.
                None => {
                    let request = verify::request(&repository, &alias, &performer)?;
                    output::emit(
                        mode,
                        "verify.request",
                        &toml::to_string_pretty(&request).map_err(|e| repo::RepoError::Io {
                            context: "could not render the verification request".to_owned(),
                            source: std::io::Error::other(e.to_string()),
                        })?,
                        output::value(&request),
                    );
                    eprintln!(
                        "# {} obligation(s) for {alias} at {} assurance.",
                        request.obligations.len(),
                        request.assurance_level
                    );
                    eprintln!(
                        "# Hand this to an INDEPENDENT verifier, then ingest with:\n\
                         #   war verify {alias} --response <file>"
                    );
                    Ok(EXIT_OK)
                }
            }
        }

        Command::Sign {
            target,
            list,
            all,
            actor,
            meaning,
            outcome,
            adr,
            independence,
            edit,
            show,
            ssh_sign,
            verify,
            kind,
        } => {
            let repository = repo::Repository::discover(None)?;
            if list {
                let waiting = sign::list(&repository)?;
                if waiting.is_empty() {
                    println!("nothing awaits a signature");
                } else {
                    println!("{} awaiting a signature:", waiting.len());
                    for p in &waiting {
                        println!("  {}", sign::line(p));
                    }
                }
                return Ok(EXIT_OK);
            }
            let parse_err = |what: &str, v: &str, known: &str| {
                repo::RepoError::Message(format!("--{what} {v:?} is not one of {known}"))
            };
            let outcome = outcome
                .as_deref()
                .map(|s| {
                    s.parse::<openwarrant_core::resolution::CommonOutcome>()
                        .map_err(|_| parse_err("outcome", s, "not_satisfied, cancelled, blocked"))
                })
                .transpose()?;
            let independence = match independence.as_str() {
                "none" => openwarrant_core::Independence::None,
                "separate_role" => openwarrant_core::Independence::SeparateRole,
                "organizational" => openwarrant_core::Independence::Organizational,
                other => {
                    return Err(parse_err(
                        "independence",
                        other,
                        "none, separate_role, organizational",
                    )
                    .into());
                }
            };
            let kind = kind
                .as_deref()
                .map(|s| {
                    s.parse::<openwarrant_core::correction::CorrectionKind>()
                        .map_err(|e| repo::RepoError::Message(format!("--kind: {e}")))
                })
                .transpose()?;
            let opts = sign::Options {
                actor,
                meaning,
                outcome,
                adr_ref: adr,
                independence,
                edit,
                all,
                show,
                ssh_sign,
                verify,
                kind,
            };
            let report = sign::run(&repository, target.as_deref(), &opts)?;
            Ok(output::finish(mode, "sign", &report, None))
        }
        Command::Authorize { alias, response } => {
            let repository = repo::Repository::discover(None)?;
            match response {
                Some(path) => {
                    let report = authorize::ingest(&repository, &alias, &path)?;
                    Ok(output::finish(mode, "authorize", &report, None))
                }
                None => {
                    let request = authorize::request(&repository, &alias)?;
                    output::emit(
                        mode,
                        "authorize.request",
                        &toml::to_string_pretty(&request).map_err(|e| repo::RepoError::Io {
                            context: "could not render the authorization request".to_owned(),
                            source: std::io::Error::other(e.to_string()),
                        })?,
                        output::value(&request),
                    );
                    eprintln!(
                        "# {alias} at {} assurance, contract {}.",
                        request.assurance_level, request.contract_digest
                    );
                    if request.eligible_authorizers.is_empty() {
                        // Naming this here rather than at ingestion time saves a
                        // round trip through a signature that could never have
                        // been accepted.
                        eprintln!(
                            "# NOBODY may authorize this: docs/authority/roles.toml grants the\n\
                             # authorizer role to no one. Authority comes from that file, and\n\
                             # only a human may write it."
                        );
                    } else {
                        eprintln!(
                            "# May be authorized by: {}",
                            request.eligible_authorizers.join(", ")
                        );
                    }
                    eprintln!(
                        "# Fill in authorizer, acting_role, meaning and effective_time, then:\n\
                         #   war authorize {alias} --response <file>"
                    );
                    Ok(EXIT_OK)
                }
            }
        }

        Command::Compile { alias } => {
            let repository = repo::Repository::discover(None)?;
            compile::run(&repository, alias.as_deref())?;
            if mode == output::Mode::Json {
                output::emit(mode, "compile", "", serde_json::json!({"alias": alias}));
            }
            Ok(EXIT_OK)
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    /// §76.4 says EVERY command should support `--json`. This is the ratchet:
    /// the subcommands that still print only for humans are listed here, by
    /// name, and a new subcommand cannot ship without either supporting the
    /// envelope or being added to this list on purpose. The list shrinks; it
    /// does not grow silently.
    #[test]
    fn every_subcommand_supports_json_or_is_listed_as_not_yet() {
        const NOT_YET: &[&str] = &["init", "kf", "telemetry", "migrate", "export", "plan"];
        let cmd = super::Cli::command();
        let all: Vec<String> = cmd
            .get_subcommands()
            .map(|c| c.get_name().to_owned())
            .collect();
        assert!(all.len() >= 20, "{all:?}");
        // Every NOT_YET entry must be a real subcommand — a stale name here
        // would make the list look longer than the gap is.
        for n in NOT_YET {
            assert!(
                all.iter().any(|a| a == n),
                "{n} is not a subcommand any more; drop it"
            );
        }
        // The global flag exists and is global.
        let json = cmd
            .get_arguments()
            .find(|a| a.get_id() == "json")
            .expect("--json is a top-level argument");
        assert!(
            json.is_global_set(),
            "--json must be global so it works after the subcommand"
        );
    }
}
