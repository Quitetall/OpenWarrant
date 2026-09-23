// SPDX-License-Identifier: Apache-2.0
//! `war` — the OpenWarrant command line interface (SAS §70–§76).

#![forbid(unsafe_code)]

use std::process::ExitCode;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use openwarrant_core::Profile;

pub mod attest;
pub mod authority_check;
pub mod authority_cmd;
pub mod authorize;
pub mod blut;
pub mod board;
pub mod bonsai;
pub mod bundle;
pub mod check;
pub mod commit;
pub mod compile;
pub mod console;
pub mod context_select;
pub mod contract_history;
pub mod correct;
pub mod diagnostic;
pub mod diff_target;
pub mod dispatch;
pub mod dispatch_bundle_cmd;
pub mod doctor;
pub mod document;
pub mod eval;
pub mod evidence;
pub mod export;
pub mod frontier;
pub mod gate_cmd;
pub mod inbox;
pub mod init;
pub mod install;
pub mod journal_cmd;
pub mod kf;
pub mod mcp;
pub mod migrate;
pub mod new;
pub mod next;
pub mod output;
pub mod overview;
pub mod ownership;
pub mod perform;
pub mod pins;
pub mod plan;
pub mod preflight_cmd;
pub mod preservation;
pub mod progress;
pub mod progress_viewer;
pub mod questions;
pub mod relations;
pub mod remedy;
pub mod repo;
pub mod resolution_cmd;
pub mod resolve;
pub mod run_cmd;
pub mod sas;
#[cfg(feature = "schema")]
pub mod schema_typescript;
#[cfg(feature = "schema")]
pub mod schemas;
pub mod sdk;
pub mod show;
pub mod sign;
pub mod status;
pub mod telemetry;
pub mod timeline;
pub mod tui;
pub mod verify;
pub mod watch;

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
pub const EXIT_OK: u8 = 0;
pub const EXIT_DIAGNOSTIC: u8 = 1;
pub const EXIT_NOT_READY: u8 = 2;

#[derive(Parser)]
#[command(
    name = "war",
    about = "Work Authorization Records — author, check, and compile Warrants.",
    version,
    long_about = None,
)]
pub struct Cli {
    /// Machine output (SAS §76.4): one `oh.war/report/v1` envelope on stdout,
    /// for every command. Errors are envelopes too.
    #[arg(long, global = true)]
    json: bool,
    /// The repository to act on. Defaults to the nearest ancestor of the
    /// current directory holding `openwarrant.toml`. A command that reads no
    /// repository (`init`, `sdk`, `install`) ignores it.
    ///
    /// A flag and not an environment variable on purpose: which repository was
    /// checked is a fact a reader should be able to recover from the process
    /// table, the journal and the gate argv, not one the ambient environment
    /// can change out from under them.
    #[arg(long, global = true, value_name = "ROOT")]
    root: Option<Utf8PathBuf>,
    /// None: `war` alone. At a terminal that is the app (OW-WAR-0112); on a
    /// pipe it is a refusal by name, so no script ever meets a full-screen
    /// application by accident.
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum EvalCommand {
    /// Run every task (or one) and write `evals/results/<sha>-<drafter>.json`.
    Run {
        /// One task id under the tasks directory.
        #[arg(long)]
        task: Option<String>,
        /// The drafter argv, one element per flag (repeat --drafter). Falls
        /// back to `[plan] drafter_argv`.
        #[arg(long = "drafter", value_name = "ARGV")]
        drafter: Vec<String>,
        /// The verifier argv, one element per flag. Falls back to
        /// `[verify] verifier_argv`.
        #[arg(long = "verifier", value_name = "ARGV")]
        verifier: Vec<String>,
        /// Where to write the result (canonical JSON; timings beside it).
        #[arg(long, value_name = "FILE")]
        out: Option<Utf8PathBuf>,
        /// Keep each scratch program for inspection.
        #[arg(long)]
        keep: bool,
        /// The tasks directory.
        #[arg(long, default_value = eval::DEFAULT_TASKS_DIR, value_name = "DIR")]
        tasks_dir: Utf8PathBuf,
    },
    /// Compare a result against the committed baseline, one rung per task.
    Verify {
        /// The result file to compare.
        result: Utf8PathBuf,
        #[arg(long, default_value = eval::DEFAULT_BASELINE, value_name = "FILE")]
        baseline: Utf8PathBuf,
    },
}

#[derive(Subcommand)]
enum DocumentCommand {
    /// Author a draft interactively; checkpoints remain on cancel or failure.
    Draft {
        #[arg(long)]
        draft_dir: Utf8PathBuf,
        #[arg(long)]
        output: Utf8PathBuf,
        #[arg(long)]
        resume: bool,
    },
    /// Review Markdown deliverables (the gate `document.review@1.0.0` runs this).
    Review {
        /// One Warrant; omit for every Warrant with a Markdown deliverable.
        alias: Option<String>,
    },
}

#[derive(Subcommand)]
enum Command {
    /// Run an offline SDK operation from an explicit JSON request. Always emits JSON.
    Sdk {
        /// Request JSON file, or - for stdin.
        #[arg(long)]
        request: String,
        /// Also save the result envelope to a new file; existing files are never replaced.
        #[arg(long)]
        output: Option<Utf8PathBuf>,
    },
    /// The app: every pane of the corpus, the queue, help and setup, in the
    /// terminal. `war` with no arguments is the same thing (OW-WAR-0112).
    Tui {
        /// Panic right after the terminal is set up — the fixture that proves
        /// it is restored. Hidden; the battery's.
        #[arg(long, hide = true)]
        panic_after_setup: bool,
    },
    /// Initialize repository configuration and directories (§71.1). With no
    /// `--namespace`, at a terminal, it asks — and walks the whole setup:
    /// who signs, the SAS, the first Warrant (OW-WAR-0112).
    Init {
        /// Namespace prefixing every local alias, e.g. `OW` in `OW-WAR-0001`.
        /// Required unless `war init` runs at a terminal, where it is asked.
        #[arg(long)]
        namespace: Option<String>,
        /// Project name. Defaults to the directory name.
        #[arg(long, conflicts_with = "program")]
        name: Option<String>,
        /// Scaffold a whole program: a SAS the tool reads, the authority
        /// examples, the `war check` gate, and a first Warrant with real
        /// atoms. `war check` on the result exits 0.
        #[arg(long, value_name = "PROGRAM")]
        program: Option<String>,
        /// Never ask, even at a terminal: `--namespace` is then required and
        /// only the examples are written, exactly as a script gets them.
        #[arg(long)]
        non_interactive: bool,
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
    /// Which `war` this is, where it came from, and what else answers to that
    /// name on PATH. Needs no repository.
    Version,
    /// Install a published release into ~/.local/lib/openwarrant and repoint
    /// the links this tool put on PATH. Reads the network; never touches a
    /// `war` it did not install.
    Update {
        /// Report what is published and stop.
        #[arg(long)]
        check: bool,
        /// Offer prereleases (1.0.0-alpha.2 and the like). Without it, only
        /// releases marked stable.
        #[arg(long)]
        preview: bool,
        /// A specific version, e.g. `1.0.0-alpha.2`.
        #[arg(long, value_name = "VERSION")]
        to: Option<String>,
        /// Install even when the version is the one already running.
        #[arg(long)]
        force: bool,
    },
    /// Read-only repository diagnostics; never signs, repairs, or starts work.
    Doctor {
        /// Inspect one Warrant instead of the whole corpus.
        alias: Option<String>,
        /// Include deterministic generated-view drift checks.
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
        /// What the Warrant should accomplish. Ignored with --proposal.
        #[arg(default_value = "")]
        request: String,
        #[arg(long, default_value = "delivery")]
        profile: String,
        #[arg(long, default_value = "basic")]
        assurance: String,
        /// A Draft Proposal returned by an agent (v2), to validate against §74.4.
        #[arg(long)]
        proposal: Option<Utf8PathBuf>,
        /// Record that §74.4 steps 5 and 6 (semantic diff, review) happened.
        #[arg(long)]
        reviewed: bool,
        /// Hand the request to the drafter configured in `[plan] drafter_argv`
        /// and read its proposal (§75.2). The process must not touch the tree.
        #[arg(long)]
        draft: bool,
        /// Where `--draft` writes the proposal. Defaults to a scratch file.
        #[arg(long)]
        out: Option<Utf8PathBuf>,
        /// An interview answer, `<question-id>=<text>` (§74.6). Repeatable.
        #[arg(long = "answer")]
        answers: Vec<String>,
        /// Apply a reviewed proposal: create the Warrant and its atoms through
        /// the seven §74.3 operations. Never a raw file write from the model.
        #[arg(long)]
        apply: bool,
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
    /// Draft, authenticate and activate authority changes under previous trusted state.
    Authority {
        #[command(subcommand)]
        command: authority_cmd::Command,
    },
    /// Capture or check portable context for an existing Dispatch; never execute it.
    DispatchBundle {
        #[command(subcommand)]
        command: dispatch_bundle_cmd::Command,
    },
    /// Compile a Stage Dispatch for one stage of a Warrant (§47).
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
        /// Write the §33 context manifest (what was selected and what was
        /// omitted, with reasons) beside the packet.
        #[arg(long, value_name = "PATH")]
        emit_context: Option<Utf8PathBuf>,
        /// Compile a packet for a contract no human has signed. The packet
        /// records `prototype://unauthorized` as the authority it acted under,
        /// so nothing downstream can mistake the work for authorized.
        #[arg(long)]
        prototype: bool,
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
        /// JSON baseline or contract:N retained revision (requires --to). Defaults to committed JSON.
        #[arg(long)]
        from: Option<Utf8PathBuf>,
        /// Explicit JSON target or contract:N retained revision. Defaults to fresh compilation.
        #[arg(long)]
        to: Option<Utf8PathBuf>,
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

    /// Experimental preservation transport. Imported records grant no authority.
    Archive {
        #[command(subcommand)]
        cmd: preservation::Command,
    },

    /// §68 portable export and round trip.
    Export {
        /// The Warrant's local alias (§68). Not needed with --progress.
        #[arg(required_unless_present_any = ["progress", "verify_progress"])]
        alias: Option<String>,
        /// Write the progress bundle (`oh.war/progress-bundle/v1`) — the corpus
        /// projections and every compiled WAR.json with a sha256 manifest — into
        /// this empty directory instead of exporting one Warrant.
        #[arg(long, value_name = "DIR", conflicts_with_all = ["force", "round_trip", "reconnect"])]
        progress: Option<Utf8PathBuf>,
        /// Verify a progress bundle directory against its MANIFEST.json.
        #[arg(long, value_name = "DIR", conflicts_with_all = ["alias", "progress", "force", "round_trip", "reconnect"])]
        verify_progress: Option<Utf8PathBuf>,
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
        /// Write the verification bundle (`oh.war/verification-bundle/v1`):
        /// the request with the authorized digest, every atom, each
        /// deliverable's bytes, the plants naming the alias, gate runs and
        /// prior verifications, under `verifications/bundle-<digest>.json`.
        #[arg(long, conflicts_with = "response")]
        bundle: bool,
        /// Write the bundle, run `[verify] verifier_argv` on it, and ingest
        /// what it prints through the same seam as `--response`.
        #[arg(long, conflicts_with_all = ["response", "bundle"])]
        run: bool,
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
        /// Draft the response and run the act's ingest with the write
        /// withheld: every refusal the real signature would meet, by its rule
        /// name, and `<act>.would-record` when none. Writes nothing, touches
        /// no key. For an agent to troubleshoot before asking a human.
        #[arg(long)]
        dry_run: bool,
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
    /// The ops / runs work kind (1.0 plan C4b): run a `service` stage's gate
    /// under its wall time, mint the receipt, write the submission.
    Run {
        /// The Warrant's local alias.
        alias: String,
        /// The stage id, e.g. `STAGE-002`.
        stage: String,
        /// Run a stage of a contract no human has signed. The Dispatch the run
        /// is built from records `prototype://unauthorized`, and so does the
        /// receipt's subject.
        #[arg(long)]
        prototype: bool,
    },
    /// Ingest a Stage Submission something else produced (§51): it must name a
    /// dispatch this Warrant compiled and may not request its own resolution.
    Submit {
        /// The Warrant's local alias.
        alias: String,
        /// The submission (`oh.war/stage-submission/v1` JSON).
        file: Utf8PathBuf,
    },
    /// The document work kind (1.0 plan C4a): review every Warrant's Markdown
    /// deliverables — digest, citations, placeholders, independent review.
    Document {
        #[command(subcommand)]
        command: DocumentCommand,
    },
    /// Attestations for ssh-signed acts (OW-ADR-0015): list them, or verify
    /// every signature and subject digest against the tree today.
    Attest {
        /// A Warrant alias or a SAS version; omit with --all.
        target: Option<String>,
        /// Verify signatures and subject digests (default lists only).
        #[arg(long)]
        verify: bool,
        /// Every attestation in the repository (the xtask step).
        #[arg(long)]
        all: bool,
    },
    /// One screen for everything you owe: the acts awaiting your signature as
    /// a checklist, the questions an agent asked, and the stages it can start
    /// (OW-WAR-0069). Check rows, pick a reason from your presets, sign the
    /// batch. It never signs for you: each row is your own ssh confirmation.
    Console,
    /// Read-only project board, including every stage and exact approval commands.
    Board {
        /// Print a self-contained offline HTML document to stdout.
        #[arg(long, conflicts_with = "json")]
        html: bool,
    },
    /// Perform an agent stage with the configured performer (OW-WAR-0069): the
    /// Dispatch goes in on stdin, a Stage Submission comes back on stdout, and
    /// it is ingested through `war submit`'s refusals. It cannot decide the work
    /// is done — §51.2 — and writes nothing if the performer sends nothing.
    Perform {
        /// The Warrant's local alias. Omit with --all.
        alias: Option<String>,
        /// The stage to perform. Omit with --all.
        stage: Option<String>,
        /// Every open agent stage on the frontier, one at a time.
        #[arg(long)]
        all: bool,
        /// Perform a stage of a contract no human has signed. The operator
        /// types this, never the agent: the Dispatch records
        /// `prototype://unauthorized` and the work is not authorized work.
        #[arg(long)]
        prototype: bool,
    },
    /// The commit message, drafted from the records that changed (OW-WAR-0069).
    Commit {
        /// Stage everything and commit with the drafted message.
        #[arg(long)]
        write: bool,
    },
    /// Ask the human a question that blocks a stage (OW-WAR-0069). An agent
    /// asks; only a human answers. Neither act authorizes anything.
    Ask {
        /// The Warrant's local alias.
        alias: String,
        /// The stage the answer unblocks; must exist in its milestones atom.
        stage: String,
        /// The question, in full.
        question: String,
        /// Your recommended answer, so it can be answered in one word.
        #[arg(long, default_value = "")]
        recommend: String,
        /// The stage cannot proceed without the answer.
        #[arg(long)]
        blocking: bool,
    },
    /// Answer a question an agent asked (OW-WAR-0069). Refused for an
    /// agent-kind actor by the register, and nothing is written.
    Answer {
        /// The Warrant's local alias.
        alias: String,
        /// The question id, e.g. `Q-001`.
        id: String,
        /// The answer, in full.
        answer: String,
        /// Who is answering; must hold a human role in roles.toml.
        #[arg(long = "as")]
        actor: String,
    },
    /// Every question across the corpus, blocking and open first, each with
    /// the command that answers it (OW-WAR-0069).
    Questions {
        /// One Warrant; omit for all.
        alias: Option<String>,
        /// Only what awaits an answer.
        #[arg(long)]
        open: bool,
    },
    /// The answers a stage's performer should read before it starts.
    Answers {
        /// The Warrant's local alias.
        alias: String,
        /// One stage; omit for every stage of the Warrant.
        stage: Option<String>,
    },
    /// The stages that can start now (OW-WAR-0068): open, unblocked by
    /// their milestone's `depends_on`, and not yet dispatched. Derived from
    /// the same records a resolution reads; never a status claim.
    Frontier {
        /// One Warrant; omit for every unresolved Warrant.
        alias: Option<String>,
    },
    /// The JSON Schema pack (OW-WAR-0032): write `schemas/` from the record
    /// types, or `--check` the tree against them. Built with `--features schema`.
    #[cfg(feature = "schema")]
    Schemas {
        /// Compare instead of writing; drift is reported by file.
        #[arg(long)]
        check: bool,
    },
    /// The agent loop, measured (1.0 plan F1): scaffold a throwaway program
    /// per task, draft, dispatch, perform, verify blind, and ask what a
    /// resolution would say. Nothing is authorized, resolved or signed.
    Eval {
        #[command(subcommand)]
        command: EvalCommand,
    },
    /// Tell the human when an act awaits them: print the pending set, then
    /// poll the record trees and print what appears or is signed away.
    Watch {
        /// Print the pending set once and exit.
        #[arg(long)]
        once: bool,
        /// Poll interval in milliseconds (minimum 50; lower values are raised to it).
        #[arg(long, default_value_t = 500)]
        interval: u64,
        /// Also raise a desktop notification through `notify-send`.
        #[arg(long)]
        notify_send: bool,
        /// Stop after this many polls (for tests).
        #[arg(long, hide = true)]
        ticks: Option<u64>,
    },
    /// Serve the Model Context Protocol over stdio (OW-ADR-0014): every
    /// read, request half and agent-permitted write as a tool; no signing,
    /// no ingest. `--describe` prints the tool table instead of serving.
    Mcp {
        /// Print the tools, the refusal list and the resources, then exit.
        #[arg(long)]
        describe: bool,
    },
    Pins {
        /// Re-record each DRAFT Warrant's content digests from the bytes on
        /// disk. Refused for anything a human has signed for; `war correct`
        /// is the act that moves those.
        #[arg(long)]
        refresh: bool,
        /// One Warrant instead of every draft.
        #[arg(long, value_name = "ALIAS")]
        alias: Option<String>,
        /// Only pins held by a resolution.
        #[arg(long)]
        resolved_only: bool,
        /// Every Warrant that ever governed one path, oldest first, and
        /// whether each delivery still verifies from history (OW-ADR-0021).
        #[arg(long, value_name = "PATH")]
        history: Option<String>,
    },
    /// What should happen next, and whose act it is. An agent is never handed
    /// a signing act; it is told that a human must sign, and how.
    Next,
    /// Warrants waiting on a human act (read-only; OW-WAR-0070).
    Inbox,
    /// Report six readiness dimensions; unavailable checks block readiness.
    Preflight { alias: String },
    /// List remaining Warrant records and status, read-only, from live sources.
    /// `progress` is an alias. Legacy resolution is not implementation completion.
    #[command(visible_alias = "progress")]
    Overview {
        /// Include records with an existing resolution in text/JSON output.
        #[arg(long)]
        all: bool,
        /// Return the validated viewer snapshot, including attributed work reports.
        #[arg(long, conflicts_with_all = ["html", "serve"])]
        snapshot: bool,
        /// Write a self-contained HTML snapshot (all records), with no implicit server.
        #[arg(long, num_args=0..=1, default_missing_value=".openwarrant/state/progress.html", conflicts_with="serve")]
        html: Option<std::path::PathBuf>,
        /// Serve a read-only, periodically refreshed view on 127.0.0.1.
        #[arg(long)]
        serve: bool,
        /// Local server port; 0 selects an available port.
        #[arg(long, default_value_t = 8765, requires = "serve")]
        port: u16,
        /// Refresh interval, in seconds (1..=3600).
        #[arg(long, default_value_t=5, value_parser=clap::value_parser!(u64).range(1..=3600), requires="serve")]
        refresh_secs: u64,
    },
    /// Where the corpus stands, from records (§17.5 `status`; §34.3; §98).
    ///
    /// Bare `war status` is the corpus projection. `war status <alias>` is the
    /// per-Warrant form §72.5 names. Every count is a ladder; nothing is a
    /// percentage.
    Status {
        /// A Warrant's local alias. Omit for the whole corpus. (`--json` is the
        /// global flag; for the corpus it yields the canonical projection.)
        alias: Option<String>,
        /// The corpus timeline (`oh.war/corpus-timeline/v1`) instead of the status.
        /// (One projection per call: `timeline` excludes `pending`, and both
        /// exclude an alias; clap checks conflicts in both directions.)
        #[arg(long, conflicts_with_all = ["alias", "pending"])]
        timeline: bool,
        /// The pending human acts (`oh.war/corpus-pending/v1`) instead of the status.
        #[arg(long, conflicts_with = "alias")]
        pending: bool,
    },

    /// Compile the configured projections (§71.8).
    Compile {
        /// A single Warrant's local alias. Defaults to the whole corpus.
        alias: Option<String>,
    },
}

pub fn entrypoint() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            // Preserve legacy argument handling. SDK callers always receive a
            // report for invocation errors; --help and --version remain help.
            //
            // The global flags have to be stepped over to find the subcommand.
            // `--root` also takes a VALUE, so the token after a bare `--root`
            // is skipped too; without that, `war --root /x sdk` finds `/x`,
            // decides this is not an SDK call, and hands an SDK caller clap's
            // help instead of the envelope it parses.
            let mut args = std::env::args_os().skip(1);
            let sdk = loop {
                let Some(arg) = args.next() else { break false };
                if arg == "--json" {
                    continue;
                }
                if arg == "--root" {
                    let _ = args.next();
                    continue;
                }
                if arg.to_string_lossy().starts_with("--root=") {
                    continue;
                }
                break arg == "sdk";
            };
            if sdk && error.use_stderr() {
                return ExitCode::from(sdk::argument_error(&error.to_string()));
            }
            error.exit();
        }
    };
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

pub fn run(cli: Cli) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = output::Mode::from_flag(cli.json);
    let root = cli.root;
    // One place decides which repository every command acts on, and it stays
    // LAZY. `init`, `sdk`, `install` and `schemas` must work where no
    // `openwarrant.toml` exists, and `doctor` reports a discovery failure
    // itself, inside a valid envelope, rather than aborting on it. Resolving
    // eagerly here would turn all of those into `repository.not-found` and
    // reorder error reporting for the rest.
    let open_repo = || repo::Repository::discover(root.clone());
    let Some(command) = cli.command else {
        // `war` alone. `--json` has no envelope to give — a terminal
        // application is not a projection — so it names the commands that do.
        if cli.json {
            return Err(Box::new(repo::RepoError::Message(
                "`war --json` has no envelope: the app is a rendering. Use `war status --json`, \
                 `war console --json`, `war next --json` or `war check --json`"
                    .to_owned(),
            )));
        }
        return Ok(tui::run(root, false)?);
    };
    match command {
        Command::Tui { panic_after_setup } => Ok(tui::run(root, panic_after_setup)?),
        Command::Sdk { request, output } => Ok(sdk::run(&request, output.as_deref())),
        Command::Init {
            namespace,
            name,
            program,
            non_interactive,
        } => {
            match (namespace, program) {
                (Some(namespace), Some(program)) => {
                    init::run_program(&program, &namespace, root)?;
                }
                (Some(namespace), None) => init::run(&namespace, name.as_deref(), root)?,
                // No namespace: a conversation, and only at a real terminal.
                // A script, a pipe, `--json` or `--non-interactive` gets the
                // refusal below, byte-for-byte what it always got from a
                // missing required flag, and never a prompt it cannot answer.
                (None, program) => {
                    if non_interactive || cli.json || !sign::at_a_terminal() {
                        return Err(Box::new(repo::RepoError::Message(
                            "`--namespace` is required here. At a terminal `war init` asks for \
                             it and walks the setup; a script passes `--namespace <NS>` (and \
                             `--program <name>` for a whole scaffold)"
                                .to_owned(),
                        )));
                    }
                    init::guided(root, program.as_deref())?;
                }
            }
            Ok(EXIT_OK)
        }

        Command::AgentsMd { stdout, force } => {
            let repository = open_repo()?;
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
            let repository = open_repo()?;
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
            let repository = open_repo()?;
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
                let repository = open_repo()?;
                let evidence = bonsai::check(&repository, &warrant, &base, &head, &binary)?;
                println!("{}", serde_json::to_string_pretty(&evidence)?);
                Ok(match evidence.verdict {
                    bonsai::EvidenceVerdict::Pass => EXIT_OK,
                    bonsai::EvidenceVerdict::Unknown => EXIT_NOT_READY,
                    bonsai::EvidenceVerdict::Fail => EXIT_DIAGNOSTIC,
                })
            }
            BonsaiCommand::VerifyEvidence { evidence } => {
                let repository = open_repo()?;
                bonsai::verify_evidence_file(&repository, &evidence)?;
                println!("Bonsai evidence is a valid passing v1 document");
                Ok(EXIT_OK)
            }
        },
        Command::Archive { cmd } => {
            let (human, result) = preservation::run(root.clone(), cmd)?;
            output::emit(mode, "archive", &human, result);
            Ok(EXIT_OK)
        }
        Command::Export {
            alias,
            force,
            round_trip,
            reconnect,
            progress,
            verify_progress,
        } => {
            if let Some(dir) = verify_progress {
                let report = progress::verify(&dir)?;
                return Ok(output::finish(
                    mode,
                    "export.progress.verify",
                    &report,
                    None,
                ));
            }
            let repository = open_repo()?;
            if let Some(dir) = progress {
                let report = progress::export(&repository, &dir)?;
                return Ok(output::finish(mode, "export.progress", &report, None));
            }
            let Some(alias) = alias else {
                // clap: `alias` is required unless a progress flag was given,
                // and both of those returned above.
                return Err(Box::new(repo::RepoError::Message(
                    "war export: a Warrant alias is required".to_owned(),
                )));
            };
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
            let repository = open_repo()?;
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
            draft,
            out,
            answers,
            apply,
        } => {
            let repository = open_repo()?;
            let answer_map: std::collections::BTreeMap<String, String> = answers
                .iter()
                .filter_map(|a| {
                    a.split_once('=')
                        .map(|(k, v)| (k.trim().to_owned(), v.to_owned()))
                })
                .collect();
            let answered: std::collections::BTreeSet<String> = answer_map.keys().cloned().collect();
            let req = plan::request(&repository, &request, &profile, &assurance, &answer_map)?;

            // Where the proposal comes from: a file, or the configured drafter.
            // A proposal written to the default scratch path is removed once
            // `--apply` has recorded it under plan/; one the user named is theirs.
            let scratch = out.is_none();
            let (proposal_path, drafter_run) = match (proposal, draft) {
                (Some(path), _) => (Some(path), None),
                (None, true) => {
                    let (json, run) = plan::run_drafter(&repository, &req)?;
                    let path = out.unwrap_or_else(|| plan::default_out(&repository));
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            repo::RepoError::Message(format!("cannot create {parent}: {e}"))
                        })?;
                    }
                    std::fs::write(&path, &json).map_err(|e| {
                        repo::RepoError::Message(format!("cannot write {path}: {e}"))
                    })?;
                    eprintln!("{}", plan::scratch_note(&path));
                    (Some(path), Some(run))
                }
                (None, false) => (None, None),
            };

            let Some(path) = proposal_path else {
                // The request half: emit and stop.
                if request.trim().is_empty() {
                    return Err(Box::new(repo::RepoError::Message(
                        "war plan needs a request sentence, or --proposal <file>".to_owned(),
                    )));
                }
                let human = serde_json::to_string_pretty(&req).expect("request serializes");
                output::emit(mode, "plan.request", &human, output::value(&req));
                if repository.config.plan.drafter_argv.is_empty() {
                    eprintln!(
                        "\nNo drafter is configured. Hand this request to an agent speaking \
                         `oh.war/agent-drafter/v1` (§75.2) and return its proposal with \
                         `war plan --proposal <file>`; or set [plan] drafter_argv and use --draft."
                    );
                }
                return Ok(EXIT_OK);
            };

            // The return half: the §74.4 gauntlet, then --apply.
            let json = std::fs::read_to_string(&path)
                .map_err(|e| repo::RepoError::Message(format!("cannot read {path}: {e}")))?;
            let is_v1 = serde_json::from_str::<serde_json::Value>(&json)
                .ok()
                .and_then(|v| {
                    v.get("api_version")
                        .and_then(|a| a.as_str().map(str::to_owned))
                })
                .is_some_and(|a| a == "oh.war/draft-proposal/v1");
            if is_v1 {
                if apply {
                    return Err(Box::new(repo::RepoError::Message(
                        "plan.v1-has-no-payloads: a v1 proposal validates but its operations carry \
                         no payload, so it cannot be applied. Return an oh.war/draft-proposal/v2"
                            .to_owned(),
                    )));
                }
                let known = plan::known_refs(&repository)?;
                let (parsed, pipeline) = show::plan::validate_proposal(&json, reviewed, &known)?;
                let mut report = diagnostic::Report::default();
                match pipeline.may_apply() {
                    Ok(()) => report.push(diagnostic::Diagnostic::pass(
                        "plan.applicable",
                        format!(
                            "v1 proposal is applicable in principle: {} operation(s), {} ADR draft(s) — but v1 carries no payloads; --apply needs v2",
                            parsed.atom_operations.len(),
                            parsed.proposed_adr_drafts.len()
                        ),
                    )),
                    Err(e) => report.push(diagnostic::Diagnostic::warn(
                        "plan.not-applicable",
                        path.to_string(),
                        format!("validated, not applicable yet: {e}"),
                    )),
                }
                let code = output::finish(mode, "plan.validate", &report, None);
                return Ok(if code == EXIT_OK && pipeline.may_apply().is_err() {
                    EXIT_NOT_READY
                } else {
                    code
                });
            }
            let known = plan::known_refs(&repository)?;
            let (parsed, mut pipeline) = plan::validate_v2(&json, reviewed, &answered, &known)?;
            if !apply {
                let mut report = diagnostic::Report::default();
                match pipeline.may_apply() {
                    Ok(()) => report.push(diagnostic::Diagnostic::pass(
                        "plan.applicable",
                        format!(
                            "proposal is applicable: {} operation(s); run again with --apply",
                            parsed.operations.len()
                        ),
                    )),
                    Err(e) => report.push(diagnostic::Diagnostic::warn(
                        "plan.not-applicable",
                        path.to_string(),
                        format!("validated, not applicable yet: {e}"),
                    )),
                }
                let ready = pipeline.may_apply().is_ok();
                let code = output::finish(
                    mode,
                    "plan.validate",
                    &report,
                    Some(output::value(&pipeline)),
                );
                return Ok(if ready { code } else { EXIT_NOT_READY });
            }
            let (applied, report) = plan::apply(
                &repository,
                &parsed,
                &mut pipeline,
                &req,
                drafter_run.as_ref(),
                &json,
            )?;
            if scratch && drafter_run.is_some() {
                // Recorded verbatim under plan/proposal.json; the scratch copy
                // under generated/ would otherwise accumulate.
                let _ = std::fs::remove_file(&path);
            }
            Ok(output::finish(
                mode,
                "plan.apply",
                &report,
                Some(output::value(&applied)),
            ))
        }
        Command::Blut {
            alias,
            verify,
            emit,
        } => {
            let repository = open_repo()?;
            let report = blut::lower(&repository, &alias, verify.as_deref(), emit.as_deref())?;
            Ok(output::finish(mode, "blut", &report, None))
        }
        Command::Authority { command } => {
            let (report, result) = authority_cmd::run(command)?;
            Ok(output::finish(mode, "authority", &report, Some(result)))
        }
        Command::DispatchBundle { command } => {
            let (report, result) = dispatch_bundle_cmd::run(root.clone(), command)?;
            Ok(output::finish(
                mode,
                "dispatch-bundle",
                &report,
                Some(result),
            ))
        }
        Command::Dispatch {
            alias,
            stage,
            attempt_kind,
            prior_failure,
            emit,
            emit_context,
            prototype,
        } => {
            let repository = open_repo()?;
            let kind = attempt_kind
                .parse::<openwarrant_core::execution::AttemptKind>()
                .map_err(|e| repo::RepoError::Message(e.to_string()))?;
            let report = dispatch::run(
                &repository,
                &alias,
                &stage,
                dispatch::Options {
                    attempt_kind: kind,
                    prior_failure_evidence: &prior_failure,
                    emit_to: emit.as_deref(),
                    emit_context_to: emit_context.as_deref(),
                    prototype,
                },
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
            let repository = open_repo()?;
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
            let repository = open_repo()?;
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
            let repository = open_repo()?;
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
            let repository = open_repo()?;
            let EvidenceCommand::Record { alias, gate } = command;
            let report = evidence::record(&repository, &alias, gate.as_deref())?;
            Ok(output::finish(mode, "evidence", &report, None))
        }
        Command::Sas { command } => {
            let repository = open_repo()?;
            let ready = |report: diagnostic::Report| Ok(output::finish(mode, "sas", &report, None));
            match command {
                SasCommand::Propose { version } => ready(sas::propose(&repository, &version)?),
                SasCommand::Accept { version, response } => match response {
                    Some(path) => ready(sas::accept_ingest(&repository, &version, &path)?),
                    None => {
                        let request = sas::accept_request(&repository, &version)?;
                        output::emit(
                            mode,
                            "sas.accept.request",
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
        Command::Pins {
            refresh,
            alias,
            resolved_only,
            ..
        } if refresh => {
            let repository = open_repo()?;
            let report = pins::refresh(&repository, alias.as_deref())?;
            let _ = resolved_only;
            Ok(output::finish(mode, "pins", &report, None))
        }
        Command::Pins {
            history: Some(path),
            ..
        } => {
            let repository = open_repo()?;
            let text = pins::history(&repository, &path)?;
            output::emit(
                mode,
                "pins",
                text.trim_end(),
                serde_json::json!({ "schema": "oh.war/pins-history/v1", "path": path, "rendered": text }),
            );
            Ok(EXIT_OK)
        }
        Command::Pins { resolved_only, .. } => {
            let repository = open_repo()?;
            let pins = pins::list(&repository, resolved_only)?;
            output::emit(
                mode,
                "pins",
                pins::render(&pins).trim_end(),
                output::value(&pins),
            );
            Ok(EXIT_OK)
        }
        Command::Run {
            alias,
            stage,
            prototype,
        } => {
            let repository = open_repo()?;
            let report = run_cmd::run(&repository, &alias, &stage, prototype)?;
            Ok(output::finish(mode, "run", &report, None))
        }
        Command::Submit { alias, file } => {
            let repository = open_repo()?;
            let report = run_cmd::submit(&repository, &alias, &file)?;
            Ok(output::finish(mode, "submit", &report, None))
        }
        Command::Document { command } => match command {
            DocumentCommand::Draft {
                draft_dir,
                output,
                resume,
            } => Ok(document::draft::run(&draft_dir, &output, resume, mode)),
            DocumentCommand::Review { alias } => {
                let repository = open_repo()?;
                let report = document::review(&repository, alias.as_deref())?;
                Ok(output::finish(mode, "document.review", &report, None))
            }
        },
        Command::Attest {
            target,
            verify,
            all,
        } => {
            let repository = open_repo()?;
            let report = match (target, all) {
                (_, true) => attest::verify_all(&repository)?,
                (Some(t), false) => attest::run(&repository, &t, verify)?,
                (None, false) => {
                    return Err(Box::new(repo::RepoError::Message(
                        "war attest: name a Warrant or SAS version, or pass --all".to_owned(),
                    )));
                }
            };
            Ok(output::finish(mode, "attest", &report, None))
        }
        Command::Board { html } => {
            let repository = open_repo()?;
            let (report, view) = board::build(&repository)?;
            if !matches!(mode, output::Mode::Json) {
                if html {
                    print!("{}", board::html(&view, &report));
                    return Ok(output::exit_code(&report));
                } else {
                    print!("{}", board::render(&view));
                }
            }
            Ok(output::finish(
                mode,
                "board",
                &report,
                Some(serde_json::to_value(&view)?),
            ))
        }
        Command::Console => {
            let repository = open_repo()?;
            // `--json` is a reader, not a screen: a harness asking what a human
            // owes gets the board and no prompt. Signing is a TTY act.
            if matches!(mode, output::Mode::Json) {
                let board = console::board(&repository)?;
                let report = diagnostic::Report::default();
                return Ok(output::finish(
                    mode,
                    "console",
                    &report,
                    Some(serde_json::to_value(&board)?),
                ));
            }
            let report = console::run(&repository)?;
            Ok(output::finish(mode, "console", &report, None))
        }
        Command::Perform {
            alias,
            stage,
            all,
            prototype,
        } => {
            let repository = open_repo()?;
            let report = match (alias.as_deref(), stage.as_deref(), all) {
                (Some(a), Some(st), false) => perform::run(&repository, a, st, prototype)?,
                (None, None, true) => perform::all(&repository, prototype)?,
                _ => {
                    return Err(Box::new(repo::RepoError::Message(
                        "war perform: name a Warrant and a stage, or pass --all".to_owned(),
                    )));
                }
            };
            Ok(output::finish(mode, "perform", &report, None))
        }
        Command::Commit { write } => {
            let repository = open_repo()?;
            let (report, result) = commit::run(&repository, write, mode)?;
            Ok(output::finish(mode, "commit", &report, result))
        }
        Command::Ask {
            alias,
            stage,
            question,
            recommend,
            blocking,
        } => {
            let repository = open_repo()?;
            let report =
                questions::ask(&repository, &alias, &stage, &question, &recommend, blocking)?;
            Ok(output::finish(mode, "ask", &report, None))
        }
        Command::Answer {
            alias,
            id,
            answer,
            actor,
        } => {
            let repository = open_repo()?;
            let report = questions::answer(&repository, &alias, &id, &answer, &actor)?;
            Ok(output::finish(mode, "answer", &report, None))
        }
        Command::Questions { alias, open } => {
            let repository = open_repo()?;
            let (report, list) = questions::list(&repository, alias.as_deref(), open)?;
            match mode {
                output::Mode::Human => {
                    print!("{}", questions::render(&list));
                    Ok(output::finish(mode, "questions", &report, None))
                }
                output::Mode::Json => Ok(output::finish(
                    mode,
                    "questions",
                    &report,
                    Some(output::value(&list)),
                )),
            }
        }
        Command::Answers { alias, stage } => {
            let repository = open_repo()?;
            let answered = questions::answers_for(&repository, &alias, stage.as_deref())?;
            let human = if answered.is_empty() {
                "no answered question for this stage\n".to_owned()
            } else {
                answered
                    .iter()
                    .map(|q| {
                        format!(
                            "{} ({}): {}\n  → {}\n",
                            q.id,
                            q.stage,
                            q.question,
                            q.answer.as_ref().map_or("", |a| a.answer.as_str())
                        )
                    })
                    .collect()
            };
            output::emit(mode, "answers", human.trim_end(), output::value(&answered));
            Ok(EXIT_OK)
        }
        Command::Frontier { alias } => {
            let repository = open_repo()?;
            let (report, f) = frontier::run(&repository, alias.as_deref())?;
            match mode {
                output::Mode::Human => {
                    print!("{}", frontier::render(&f));
                    Ok(output::finish(mode, "frontier", &report, None))
                }
                output::Mode::Json => Ok(output::finish(
                    mode,
                    "frontier",
                    &report,
                    Some(output::value(&f)),
                )),
            }
        }
        #[cfg(feature = "schema")]
        Command::Schemas { check } => {
            let repository = open_repo()?;
            let report = schemas::run(&repository, check)?;
            Ok(output::finish(mode, "schemas", &report, None))
        }
        Command::Eval { command } => {
            let repository = open_repo()?;
            match command {
                EvalCommand::Run {
                    task,
                    drafter,
                    verifier,
                    out,
                    keep,
                    tasks_dir,
                } => {
                    let opts = eval::Options {
                        tasks_dir,
                        only: task,
                        drafter,
                        verifier,
                        out,
                        keep,
                    };
                    let (report, result, path) = eval::run(&repository, &opts)?;
                    let mut value = output::value(&result);
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert(
                            "result_path".to_owned(),
                            serde_json::Value::String(repository.relative(&path)),
                        );
                    }
                    Ok(output::finish(mode, "eval.run", &report, Some(value)))
                }
                EvalCommand::Verify { result, baseline } => {
                    let report = eval::verify(&repository, &result, &baseline)?;
                    Ok(output::finish(mode, "eval.verify", &report, None))
                }
            }
        }
        Command::Watch {
            once,
            interval,
            notify_send,
            ticks,
        } => {
            let repository = open_repo()?;
            if once {
                let snap = watch::snapshot(&repository)?;
                output::emit(
                    mode,
                    "watch",
                    watch::render(&snap).trim_end(),
                    output::value(&snap),
                );
                return Ok(EXIT_OK);
            }
            watch::run(&repository, interval, notify_send, ticks)?;
            Ok(EXIT_OK)
        }
        Command::Mcp { describe } => {
            // Discover BEFORE any runtime exists: outside a repository this
            // is an ordinary CLI refusal, never a half-started server.
            let repository = open_repo()?;
            if describe {
                print!("{}", mcp::describe(repository));
                return Ok(EXIT_OK);
            }
            mcp::run(repository)?;
            Ok(EXIT_OK)
        }
        Command::Preflight { alias } => {
            let repository = open_repo()?;
            let (result, report) = preflight_cmd::run(&repository, &alias)?;
            match mode {
                output::Mode::Human => print!("{}", preflight_cmd::render(&result)),
                output::Mode::Json => println!(
                    "{}",
                    output::envelope("preflight", &report, Some(output::value(&result)))
                ),
            }
            Ok(output::exit_code(&report))
        }
        Command::Inbox => {
            let repository = open_repo()?;
            let inbox = inbox::run(&repository)?;
            output::emit(
                mode,
                "inbox",
                inbox::render(&inbox).trim_end(),
                output::value(&inbox),
            );
            Ok(EXIT_OK)
        }
        Command::Next => {
            let repository = open_repo()?;
            let next = next::run(&repository)?;
            output::emit(
                mode,
                "next",
                next::render(&next).trim_end(),
                output::value(&next),
            );
            Ok(EXIT_OK)
        }
        Command::Overview {
            all,
            snapshot,
            html,
            serve,
            port,
            refresh_secs,
        } => {
            let repository = open_repo()?;
            if snapshot {
                let view = progress_viewer::json_snapshot(&repository)?;
                output::emit(
                    mode,
                    "overview",
                    &serde_json::to_string_pretty(&view).unwrap(),
                    view,
                );
                return Ok(EXIT_OK);
            }
            if let Some(path) = html {
                progress_viewer::export(&repository, &path)?;
                output::emit(
                    mode,
                    "overview",
                    &format!("Progress: {}", path.display()),
                    serde_json::json!({"html":path}),
                );
                return Ok(EXIT_OK);
            }
            if serve {
                progress_viewer::serve(repository, port, refresh_secs, mode)?;
                return Ok(EXIT_OK);
            }
            let view = overview::build(status::build(&repository)?, all);
            output::emit(
                mode,
                "overview",
                &overview::render(&view),
                output::value(&view),
            );
            Ok(EXIT_OK)
        }
        Command::Status {
            alias,
            timeline,
            pending,
        } => {
            let repository = open_repo()?;
            if timeline || pending {
                let (what, value) = if timeline {
                    let t = timeline::build_timeline(&repository)?;
                    (
                        format!(
                            "{} event(s) across {} Warrant(s), {} day(s)",
                            t.events.len(),
                            t.warrants,
                            t.days.len()
                        ),
                        output::value(&t),
                    )
                } else {
                    let p = timeline::build_pending(&repository)?;
                    let lines: Vec<String> = p
                        .acts
                        .iter()
                        .map(|a| format!("{}  {}  {}", a.warrant, a.action, a.command))
                        .collect();
                    (
                        if lines.is_empty() {
                            "nothing awaits a signature".to_owned()
                        } else {
                            lines.join("\n")
                        },
                        output::value(&p),
                    )
                };
                output::emit(mode, "status", &what, value);
                return Ok(EXIT_OK);
            }
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
            let repository = open_repo()?;
            let rendered = show::run(&repository, &alias, &view)?;
            output::emit(
                mode,
                "show",
                &rendered,
                serde_json::json!({"alias": alias, "view": view, "rendered": rendered}),
            );
            Ok(EXIT_OK)
        }
        Command::Diff { alias, from, to } => {
            let repository = open_repo()?;
            let report = if let Some(to) = to {
                diff_target::compare(&repository, &alias, from.as_deref(), &to)?
            } else {
                if from
                    .as_ref()
                    .is_some_and(|value| value.as_str().starts_with("contract:"))
                {
                    return Err(repo::RepoError::Message(
                        "contract baseline requires explicit --to target".into(),
                    )
                    .into());
                }
                show::diff(&repository, &alias, from.as_ref())?
            };
            // A diff is information, not a verdict: exit 0 whatever it found.
            let _ = output::finish(mode, "diff", &report, None);
            Ok(EXIT_OK)
        }
        Command::Version => {
            let install = install::observe();
            let report = install.report();
            Ok(output::finish(
                mode,
                "version",
                &report,
                Some(install.json()),
            ))
        }
        Command::Update {
            check,
            preview,
            to,
            force,
        } => {
            let channel = if preview || to.is_some() {
                install::Channel::Preview
            } else {
                install::Channel::Stable
            };
            let report = if check {
                install::check(channel)
            } else {
                install::update(channel, to.as_deref(), force)
            };
            Ok(output::finish(mode, "update", &report, None))
        }
        Command::Doctor { alias, generated } => {
            let (report, result) = doctor::run(root.clone(), alias.as_deref(), generated);
            Ok(output::finish(mode, "doctor", &report, Some(result)))
        }
        Command::Check { alias, generated } => {
            let repository = open_repo()?;
            let report = check::run(&repository, alias.as_deref(), generated)?;
            // A non-zero exit for an unsound Warrant is what lets CI gate on it.
            Ok(output::finish(mode, "check", &report, None))
        }

        Command::Verify {
            alias,
            performer,
            response,
            bundle,
            run,
        } => {
            let repository = open_repo()?;
            if bundle {
                let report = bundle::emit(&repository, &alias, &performer)?;
                return Ok(output::finish(mode, "verify.bundle", &report, None));
            }
            if run {
                let report = bundle::run(&repository, &alias, &performer)?;
                return Ok(output::finish(mode, "verify.run", &report, None));
            }
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
            dry_run,
            ssh_sign,
            verify,
            kind,
        } => {
            let repository = open_repo()?;
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
                dry_run,
                ssh_sign,
                verify,
                kind,
            };
            let report = sign::run(&repository, target.as_deref(), &opts)?;
            Ok(output::finish(mode, "sign", &report, None))
        }
        Command::Authorize { alias, response } => {
            let repository = open_repo()?;
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
            let repository = open_repo()?;
            compile::run(&repository, alias.as_deref())?;
            if mode == output::Mode::Json {
                output::emit(mode, "compile", "", serde_json::json!({"alias": alias}));
            }
            Ok(EXIT_OK)
        }
    }
}
