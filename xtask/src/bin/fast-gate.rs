// SPDX-License-Identifier: Apache-2.0
//! The PR tier. The full, historically pinned `cargo xtask gate` stays unchanged.
use std::process::{Command, ExitCode};

struct Step {
    name: &'static str,
    program: &'static str,
    args: &'static [&'static str],
}

const STEPS: &[Step] = &[
    Step {
        name: "build",
        program: "cargo",
        args: &["build", "--locked", "--workspace"],
    },
    Step {
        name: "format",
        program: "cargo",
        args: &["fmt", "--all", "--check"],
    },
    Step {
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
        name: "workspace tests (including refusal cases)",
        program: "cargo",
        args: &["test", "--locked", "--workspace", "--no-fail-fast"],
    },
    Step {
        name: "licenses",
        program: "cargo",
        args: &["deny", "check", "licenses"],
    },
    Step {
        name: "record and generated-view integrity",
        program: "./target/debug/war",
        args: &["check", "--generated"],
    },
    Step {
        name: "agent guidance",
        program: "cargo",
        args: &["xtask", "skills"],
    },
    Step {
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

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--list"] {
        for step in STEPS {
            println!("{}: {} {}", step.name, step.program, step.args.join(" "));
        }
        return ExitCode::SUCCESS;
    }
    let candidate = match args.as_slice() {
        [] => None,
        [flag, base] if flag == "--base" => Some(base),
        _ => {
            eprintln!("usage: cargo xtask-fast [--list | --base <merge-base>]");
            return ExitCode::FAILURE;
        }
    };
    let mut failed = Vec::new();
    for step in STEPS {
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
    if let Some(base) = candidate {
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
