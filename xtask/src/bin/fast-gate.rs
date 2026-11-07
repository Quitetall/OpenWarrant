// SPDX-License-Identifier: Apache-2.0
//! The PR tier. The full, historically pinned `cargo xtask gate` stays unchanged.
//!
//! `cargo xtask-fast [--only <id>[,<id>...]] [--base <merge-base>]`. With no
//! `--only`, every step runs (`views` and `acceptance` only with a base); CI runs one id per job (fast.yml), and every job
//! runs this same definition. The steps are ordered so the schema feature
//! build, which recompiles the workspace under different features, runs last.
use std::process::{Command, ExitCode};

struct Step {
    id: &'static str,
    name: &'static str,
    program: &'static str,
    args: &'static [&'static str],
}

const STEPS: &[Step] = &[
    Step {
        id: "build",
        name: "build",
        program: "cargo",
        args: &["build", "--locked", "--workspace"],
    },
    Step {
        id: "format",
        name: "format",
        program: "cargo",
        args: &["fmt", "--all", "--check"],
    },
    Step {
        id: "clippy",
        name: "clippy",
        program: "cargo",
        args: &[
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    },
    Step {
        id: "tests",
        name: "workspace tests (including refusal cases)",
        program: "cargo",
        args: &[
            "nextest",
            "run",
            "--locked",
            "--workspace",
            "--no-fail-fast",
        ],
    },
    Step {
        id: "doc-tests",
        name: "doc tests",
        program: "cargo",
        args: &["test", "--locked", "--workspace", "--doc"],
    },
    Step {
        id: "licenses",
        name: "licenses",
        program: "cargo",
        args: &["deny", "check", "licenses"],
    },
    // Views are written on main only (conformance/views.toml), so a branch's
    // committed copies may be stale: they are compiled fresh from the masters
    // in a scratch worktree and checked there, never read as committed.
    Step {
        id: "corpus",
        name: "record and generated-view integrity (views compiled fresh)",
        program: "python3",
        args: &[
            "conformance/views.py",
            "verify",
            "--war",
            "./target/debug/war",
        ],
    },
    Step {
        id: "skills",
        name: "agent guidance",
        program: "cargo",
        args: &["xtask", "skills"],
    },
    Step {
        id: "schemas",
        name: "schema drift",
        program: "cargo",
        args: &[
            "run",
            "--locked",
            "--quiet",
            "-p",
            "openwarrant-cli",
            "--features",
            "schema",
            "--",
            "schemas",
            "--check",
        ],
    },
];

const USAGE: &str = "usage: cargo xtask-fast [--only <id>[,<id>...]] [--base <merge-base>]\n\
    ids: build, format, clippy, tests, doc-tests, licenses, corpus, skills, schemas, views, acceptance";

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--list"] {
        for step in STEPS {
            println!("{}: {} {}", step.id, step.program, step.args.join(" "));
        }
        return ExitCode::SUCCESS;
    }
    let mut only: Option<Vec<String>> = None;
    let mut base: Option<String> = None;
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        match (flag.as_str(), rest.next()) {
            ("--only", Some(ids)) => only = Some(ids.split(',').map(str::to_owned).collect()),
            ("--base", Some(b)) => base = Some(b.clone()),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
        }
    }
    let wanted = |id: &str| only.as_ref().is_none_or(|o| o.iter().any(|x| x == id));
    if let Some(ids) = &only {
        let known =
            |id: &str| STEPS.iter().any(|s| s.id == id) || id == "views" || id == "acceptance";
        if let Some(bad) = ids.iter().find(|id| !known(id)) {
            eprintln!("unknown step {bad:?}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    }
    let mut failed = Vec::new();
    for step in STEPS.iter().filter(|s| wanted(s.id)) {
        println!("== {} ==", step.name);
        match Command::new(step.program).args(step.args).status() {
            Ok(status) if status.success() => {}
            Ok(status) => {
                eprintln!("{}: {status}", step.name);
                failed.push(step.name);
            }
            Err(error) => {
                eprintln!("{}: COULD NOT RUN: {error}", step.name);
                failed.push(step.name);
            }
        }
    }
    // A pull request carries masters, never views (conformance/views.toml):
    // a change to a view since the base is refused, path by path.
    if let (true, Some(base)) = (wanted("views"), base.as_deref()) {
        println!("== pull requests carry no views ==");
        let result = Command::new("python3")
            .args(["conformance/views.py", "pr", "--base", base])
            .status();
        if !result.is_ok_and(|status| status.success()) {
            failed.push("pull requests carry no views");
        }
    }
    // Acceptance needs a base to compare with, so it runs only when one is given.
    if let (true, Some(base)) = (wanted("acceptance"), base.as_deref()) {
        println!("== candidate acceptance ==");
        let result = Command::new("./target/debug/war")
            .args([
                "admin",
                "pins",
                "--candidate",
                "HEAD",
                "--base",
                base,
                "--resolved-only",
            ])
            .status();
        if !result.is_ok_and(|status| status.success()) {
            failed.push("candidate acceptance");
        }
    }
    if failed.is_empty() {
        println!("fast-gate: PASS (PR tier; full checkpoint gate not run)");
        ExitCode::SUCCESS
    } else {
        eprintln!("fast-gate: FAIL: {}", failed.join(", "));
        ExitCode::FAILURE
    }
}
