// SPDX-License-Identifier: Apache-2.0
//! `cargo scale-budget`: the daily verbs' speed budget at 10,000 Warrants
//! (`tools/scale/budget-10k.toml`).
//!
//! Too slow for the battery: the corpus takes minutes to build, through the
//! real verbs. So it is its own command, run before a release and whenever a
//! change touches what `war next`, `war view ready` or `war claim` read
//! (CONTRIBUTING.md, "The speed budget"). It is a binary of the xtask
//! package beside `cargo xtask`, whose `src/main.rs` a resolved Warrant pins
//! (OW-WAR-0060/D-002), so it is not a task of that dispatcher. The battery exercises the same
//! tooling at a small size in `conformance/plants.d/174-scale-budget.sh`,
//! including the refusals.
//!
//! The steps:
//!
//! 1. `cargo build --release -p openwarrant-cli`: the budget is for a release
//!    build, and `budget.sh` reads the profile from the binary's path.
//! 2. The corpus: built by `tools/scale/synth-corpus.sh` in the shape the
//!    budget's `applies_to` names (`--unsigned`: no key, no agent, nothing
//!    signed), or, with `--corpus <dir>`, a copy of one built before. The
//!    budget's `claim` row takes items, so it always runs on a copy.
//! 3. `tools/scale/budget.sh` over it, whose exit code is this command's: 0
//!    only when every row is within its limit.
//!
//! Every child runs with no ssh-agent reachable.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const BUDGET: &str = "tools/scale/budget-10k.toml";

struct Args {
    corpus: Option<PathBuf>,
    runs: Option<String>,
    out: Option<PathBuf>,
    keep: bool,
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: cargo scale-budget [--corpus <dir>] [--runs <n>] [--out <file>] [--keep]\n\
         \n\
         Builds war in release, builds the 10,000-Warrant corpus {BUDGET} names (or copies\n\
         --corpus, one built before), and runs tools/scale/budget.sh over it. --out keeps the\n\
         record; --keep keeps the corpus and says where."
    );
    ExitCode::from(2)
}

fn parse(args: impl Iterator<Item = String>) -> Option<Args> {
    let mut out = Args {
        corpus: None,
        runs: None,
        out: None,
        keep: false,
    };
    let mut args = args;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--corpus" => out.corpus = Some(PathBuf::from(args.next()?)),
            "--runs" => out.runs = Some(args.next()?),
            "--out" => out.out = Some(PathBuf::from(args.next()?)),
            "--keep" => out.keep = true,
            _ => return None,
        }
    }
    Some(out)
}

/// `key = <integer>` under `[applies_to]` in the budget file.
fn applies_to(text: &str, key: &str) -> Option<u64> {
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[applies_to]";
            continue;
        }
        if inside
            && let Some((k, v)) = line.split_once('=')
            && k.trim() == key
        {
            return v.trim().parse().ok();
        }
    }
    None
}

fn run(cmd: &mut Command, what: &str) -> Result<(), String> {
    cmd.env_remove("SSH_AUTH_SOCK").env_remove("SSH_AGENT_PID");
    match cmd.status() {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("{what} exited {}", s.code().unwrap_or(-1))),
        Err(e) => Err(format!("{what} could not start: {e}")),
    }
}

fn main() -> ExitCode {
    let Some(args) = parse(std::env::args().skip(1)) else {
        return usage();
    };
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let budget = repo.join(BUDGET);
    let Ok(text) = std::fs::read_to_string(&budget) else {
        eprintln!("scale-budget: cannot read {}", budget.display());
        return ExitCode::FAILURE;
    };
    let (Some(warrants), Some(tickets)) =
        (applies_to(&text, "warrants"), applies_to(&text, "tickets"))
    else {
        eprintln!("scale-budget: {BUDGET} names no [applies_to] warrants and tickets");
        return ExitCode::FAILURE;
    };

    println!("== 1. cargo build --release -p openwarrant-cli");
    if let Err(e) = run(
        Command::new("cargo").current_dir(&repo).args([
            "build",
            "--release",
            "-p",
            "openwarrant-cli",
        ]),
        "cargo build",
    ) {
        eprintln!("scale-budget: {e}");
        return ExitCode::FAILURE;
    }
    let target =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| repo.join("target"), PathBuf::from);
    let war = target.join("release").join("war");
    if !war.is_file() {
        eprintln!("scale-budget: no binary at {}", war.display());
        return ExitCode::FAILURE;
    }

    let scratch =
        std::env::temp_dir().join(format!("openwarrant-scale-budget-{}", std::process::id()));
    if let Err(e) = std::fs::create_dir_all(&scratch) {
        eprintln!("scale-budget: cannot create {}: {e}", scratch.display());
        return ExitCode::FAILURE;
    }
    let corpus = scratch.join("corpus");
    let made = match &args.corpus {
        Some(from) => {
            println!("== 2. the corpus: a copy of {}", from.display());
            run(Command::new("cp").arg("-a").arg(from).arg(&corpus), "cp -a")
        }
        None => {
            println!(
                "== 2. the corpus: {warrants} directory Warrants and {tickets} tickets, unsigned (takes minutes)"
            );
            run(
                Command::new(repo.join("tools/scale/synth-corpus.sh"))
                    .current_dir(&repo)
                    .arg("--war")
                    .arg(&war)
                    .args(["--n", &warrants.to_string(), "--resolved", "0"])
                    .args(["--tickets", &tickets.to_string(), "--unsigned"])
                    .arg("--out")
                    .arg(&corpus),
                "synth-corpus.sh",
            )
        }
    };
    let code = match made {
        Err(e) => {
            eprintln!("scale-budget: {e}");
            ExitCode::FAILURE
        }
        Ok(()) => {
            println!("== 3. tools/scale/budget.sh --budget {BUDGET}");
            let mut cmd = Command::new(repo.join("tools/scale/budget.sh"));
            cmd.current_dir(&repo)
                .arg("--war")
                .arg(&war)
                .arg("--root")
                .arg(&corpus)
                .arg("--budget")
                .arg(&budget);
            if let Some(runs) = &args.runs {
                cmd.args(["--runs", runs]);
            }
            if let Some(out) = &args.out {
                cmd.arg("--out").arg(out);
            }
            match run(&mut cmd, "budget.sh") {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("scale-budget: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    };
    if args.keep {
        println!("the corpus is kept at {}", corpus.display());
    } else if let Err(e) = std::fs::remove_dir_all(&scratch) {
        eprintln!("scale-budget: could not remove {}: {e}", scratch.display());
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shape_is_read_from_the_budget_file() {
        let text = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join(BUDGET),
        )
        .expect("the budget file");
        assert_eq!(applies_to(&text, "warrants"), Some(200));
        assert_eq!(applies_to(&text, "tickets"), Some(9800));
        // A key outside [applies_to] is not read as one.
        assert_eq!(applies_to("[other]\nwarrants = 5\n", "warrants"), None);
    }

    #[test]
    fn an_unknown_argument_is_refused() {
        assert!(parse(["--nope".to_owned()].into_iter()).is_none());
        assert!(parse(["--runs".to_owned()].into_iter()).is_none());
        let a = parse(["--keep".to_owned(), "--runs".to_owned(), "3".to_owned()].into_iter())
            .expect("parses");
        assert!(a.keep && a.runs.as_deref() == Some("3"));
    }
}
