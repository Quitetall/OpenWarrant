// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war` — the OpenWarrant command line interface (SAS §70–§76).

#![forbid(unsafe_code)]

use std::process::ExitCode;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use openwarrant_core::Profile;

mod blut;
mod check;
mod compile;
mod diagnostic;
mod gate_cmd;
mod init;
mod migrate;
mod new;
mod repo;
mod resolve;
mod show;

/// Exit codes. §76.4 wants machine-usable output; a caller distinguishing
/// "your input was wrong" from "the Warrant is not sound" needs more than 0/1.
///
/// Codes are added when a command can actually produce them. An exit code the
/// binary never returns is a promise to callers that nothing keeps.
const EXIT_OK: u8 = 0;
const EXIT_DIAGNOSTIC: u8 = 1;
const EXIT_NOT_READY: u8 = 2;

#[derive(Parser)]
#[command(
    name = "war",
    about = "Work Authorization Records — author, check, and compile Warrants.",
    version,
    long_about = None,
)]
struct Cli {
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
    /// Evaluate §56.1's thirteen resolution requirements without recording one.
    Resolve {
        /// The Warrant's local alias.
        alias: String,
        /// Required. Recording a resolution needs an authorizer and a stated
        /// meaning, which this command may not invent.
        #[arg(long)]
        dry_run: bool,
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

    /// Compile the configured projections (§71.8).
    Compile {
        /// A single Warrant's local alias. Defaults to the whole corpus.
        alias: Option<String>,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(report) => {
            // §76.2: an explicit diagnostic naming what was wrong and where,
            // never a bare "error".
            eprintln!("error: {report}");
            ExitCode::from(EXIT_DIAGNOSTIC)
        }
    }
}

fn run(cli: Cli) -> Result<u8, Box<dyn std::error::Error>> {
    match cli.command {
        Command::Init {
            namespace,
            name,
            root,
        } => {
            init::run(&namespace, name.as_deref(), root)?;
            Ok(EXIT_OK)
        }

        Command::New { title, profile } => {
            let profile: Profile = profile.parse()?;
            let repository = repo::Repository::discover(None)?;
            let dir = new::run(&repository, &title, profile)?;
            println!("created {}", repository.relative(&dir));
            println!("edit its atoms, then run `war check`");
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

        Command::Gate { run, gate } => {
            let repository = repo::Repository::discover(None)?;
            let report = gate_cmd::run(&repository, run, gate.as_deref())?;
            check::print(&report);
            // §44.1 and RQ-054: an unaskable gate is NOT a pass, and is not a
            // failure either. `is_ready()` blocks on unknowns, so both land on a
            // non-zero exit without the two being conflated in the report.
            Ok(if report.is_ready() {
                EXIT_OK
            } else {
                EXIT_NOT_READY
            })
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
                let (parsed, pipeline) = show::plan::validate_proposal(&json, reviewed)?;
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
            check::print(&report);
            Ok(if report.is_ready() {
                EXIT_OK
            } else {
                EXIT_NOT_READY
            })
        }
        Command::Resolve { alias, dry_run } => {
            if !dry_run {
                return Err(Box::new(repo::RepoError::Message(
                    "recording a resolution requires --dry-run today. §56.2's record \
                     needs an authorizer, an acting role and a stated meaning, and this \
                     build has no authority model to supply them (OW-WAR-0044)."
                        .to_owned(),
                )));
            }
            let repository = repo::Repository::discover(None)?;
            let report = resolve::run(&repository, &alias)?;
            check::print(&report);
            Ok(if report.is_ready() {
                EXIT_OK
            } else {
                EXIT_NOT_READY
            })
        }
        Command::Show { alias, view } => {
            let repository = repo::Repository::discover(None)?;
            let rendered = show::run(&repository, &alias, &view)?;
            println!("{rendered}");
            Ok(EXIT_OK)
        }
        Command::Diff { alias, from } => {
            let repository = repo::Repository::discover(None)?;
            let report = show::diff(&repository, &alias, from.as_ref())?;
            check::print(&report);
            Ok(EXIT_OK)
        }
        Command::Check { alias, generated } => {
            let repository = repo::Repository::discover(None)?;
            let report = check::run(&repository, alias.as_deref(), generated)?;
            check::print(&report);
            // A non-zero exit for an unsound Warrant is what lets CI gate on it.
            Ok(if report.is_ready() {
                EXIT_OK
            } else {
                EXIT_NOT_READY
            })
        }

        Command::Compile { alias } => {
            let repository = repo::Repository::discover(None)?;
            compile::run(&repository, alias.as_deref())?;
            Ok(EXIT_OK)
        }
    }
}
