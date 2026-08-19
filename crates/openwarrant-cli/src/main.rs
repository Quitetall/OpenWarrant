// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war` — the OpenWarrant command line interface (SAS §70–§76).

#![forbid(unsafe_code)]

use std::process::ExitCode;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

mod init;

/// Exit codes. §76.4 wants machine-usable output; a caller distinguishing
/// "your input was wrong" from "I could not run" needs more than 0/1.
///
/// Codes are added when a command can actually produce them. An exit code the
/// binary never returns is a promise to callers that nothing keeps.
const EXIT_OK: u8 = 0;
const EXIT_DIAGNOSTIC: u8 = 1;

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
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Init {
            namespace,
            name,
            root,
        } => init::run(&namespace, name.as_deref(), root),
    };

    match result {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err(report) => {
            // §76.2: an explicit diagnostic naming what was wrong and where,
            // never a bare "error".
            eprintln!("error: {report}");
            ExitCode::from(EXIT_DIAGNOSTIC)
        }
    }
}
