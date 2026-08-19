// SPDX-License-Identifier: AGPL-3.0-or-later
//! `cargo xtask gate` — the aggregate gate (SAS §92).
//!
//! §92: "The final command SHALL exit zero only when every positive fixture
//! passes and every planted violation is rejected by the intended control."
//!
//! Two halves, and the second is the one that matters. Running the test suite
//! proves the positive fixtures pass. It does NOT prove that any control would
//! have caught a violation — a validator that returns `Ok(())` unconditionally
//! passes every positive fixture ever written. The planted-violation half exists
//! because this fleet has three times shipped a green gate that compared nothing.
//!
//! Planted violations currently live as `#[test]` cases asserting a refusal
//! (`init_refuses_to_overwrite`, `non_v7_uuid_is_refused`, `unknown_schema_fails_closed`,
//! and the rest). That is sufficient while every control is in-process. When
//! OW-WAR-0005 lands `war check`, the plants move to `conformance/` as real
//! source trees mutated on disk, because at that point the control being tested
//! is a binary reading files rather than a function taking a value.

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("gate") => gate(),
        Some(other) => {
            eprintln!("xtask: unknown task {other:?}; known tasks: gate");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("usage: cargo xtask gate");
            ExitCode::FAILURE
        }
    }
}

/// One step of the gate: a label and the command that decides it.
struct Step {
    label: &'static str,
    program: &'static str,
    args: &'static [&'static str],
}

fn gate() -> ExitCode {
    let steps = [
        Step {
            label: "format",
            program: "cargo",
            args: &["fmt", "--all", "--check"],
        },
        Step {
            label: "clippy",
            program: "cargo",
            args: &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        },
        Step {
            label: "tests (positive fixtures + planted violations)",
            program: "cargo",
            args: &["test", "--workspace", "--no-fail-fast"],
        },
        // The license gate is not hygiene here. This repository ships
        // AGPL-3.0-or-later and is intended to relicense to Apache-2.0; a
        // copyleft dependency adopted today would make that impossible later.
        Step {
            label: "licenses (permissive-only; protects the Apache-2.0 path)",
            program: "cargo",
            args: &["deny", "check", "licenses"],
        },
        // §92's second half, and the reason the first half means anything: every
        // planted violation must be rejected BY ITS INTENDED CONTROL. The unit
        // tests above prove the code does what it says; this proves the shipped
        // binary refuses what it should, on real files, for the stated reason.
        Step {
            label: "planted violations (§92 — each rejected by its intended control)",
            program: "bash",
            args: &["conformance/plant.sh"],
        },
    ];

    let mut failed = Vec::new();

    for step in &steps {
        println!("== {} ==", step.label);
        let status = Command::new(step.program).args(step.args).status();
        match status {
            Ok(status) if status.success() => println!("   ok"),
            Ok(status) => {
                println!("   FAILED ({status})");
                failed.push(step.label);
            }
            Err(err) => {
                // "Could not run" is not "failed". Collapsing them is exactly
                // what SAS §96.4 forbids for gate results, and the same honesty
                // applies to the gate that enforces it.
                println!("   COULD NOT RUN: {err}");
                failed.push(step.label);
            }
        }
    }

    // Report every failing step, not the first. A gate that stops at the first
    // failure makes the operator re-run it once per defect.
    if failed.is_empty() {
        println!("\ngate: PASS — {} step(s) green", steps.len());
        ExitCode::SUCCESS
    } else {
        println!(
            "\ngate: FAIL — {}/{} step(s) failed:",
            failed.len(),
            steps.len()
        );
        for label in &failed {
            println!("  - {label}");
        }
        ExitCode::FAILURE
    }
}
