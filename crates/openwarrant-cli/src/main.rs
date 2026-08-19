// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war` — the OpenWarrant command line interface (SAS §70–§76).

#![forbid(unsafe_code)]

use std::process::ExitCode;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use openwarrant_core::Profile;

mod check;
mod compile;
mod diagnostic;
mod gate_cmd;
mod init;
mod new;
mod repo;

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
