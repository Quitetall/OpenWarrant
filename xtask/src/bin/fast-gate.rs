// SPDX-License-Identifier: Apache-2.0
//! The PR tier. The full, historically pinned `cargo xtask gate` stays unchanged.
//!
//! `cargo xtask-fast [--only <id>[,<id>...]] [--base <merge-base>]`. With no
//! `--only`, every step runs; CI runs one id per job (fast.yml), and every job
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
    Step {
        id: "corpus",
        name: "record and generated-view integrity",
        program: "./target/debug/war",
        args: &["check", "--generated"],
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
    ids: build, format, clippy, tests, doc-tests, licenses, corpus, skills, schemas, acceptance";

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
        let known = |id: &str| STEPS.iter().any(|s| s.id == id) || id == "acceptance";
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
