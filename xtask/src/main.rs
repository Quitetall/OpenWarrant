// SPDX-License-Identifier: Apache-2.0
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
//! Planted violations live in two places, and both are needed:
//!
//! - `#[test]` cases asserting a refusal from a function given a value
//!   (`non_v7_uuid_is_refused`, `duplicate_ordinal_is_refused`, and the rest);
//! - `conformance/plant.sh`, which mutates real files and runs the SHIPPED
//!   BINARY, then asserts the right rule fired for the right reason.
//!
//! The second exists because the first cannot catch a control that is correct in
//! isolation and never reached in the real code path.

use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("gate") => gate(),
        // OW-WAR-0068: the skill checks alone, for a plant and for a fast
        // local loop while writing a skill.
        Some("skills") => {
            let a = check_skill();
            let b = check_agent_docs();
            match (a, b) {
                (Ok(_), Ok(0)) => ExitCode::SUCCESS,
                (Ok(_), Ok(n)) => {
                    eprintln!("xtask skills: {n} agent-facing document(s) carry an em-dash");
                    ExitCode::FAILURE
                }
                (Err(e), _) | (_, Err(e)) => {
                    eprintln!("xtask skills: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some(other) => {
            eprintln!("xtask: unknown task {other:?}; known tasks: gate, skills");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("usage: cargo xtask gate | skills");
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

/// The SPDX identifier a Rust source file must declare after the relicense.
///
/// Checked mechanically because the relicense rewrites exactly this line in
/// every file (RELICENSING.md, OW-ADR-0017). A file that never carried a
/// header would be silently skipped by that rewrite and would keep asserting
/// the old licence, or none at all, afterwards.
/// Both identifiers are assembled from parts on purpose. A relicense sweeps
/// the repository for the whole header string, and the first run of it
/// rewrote the constant that NAMES the old licence here, which silently
/// turned the ratchet below into a check that nothing could satisfy. Split,
/// no sed can reach them.
const SPDX_PREFIX: &str = "// SPDX-License-Identifier: ";
const EXPECTED_ID: &str = "Apache-2.0";
const PRIOR_ID: &str = "AGPL-3.0-or-later";

/// Files allowed to carry the prior identifier, one repository-relative path per line.
/// Absent before the relicense, when either identifier passes; present after,
/// when it is the whole allowance. Shrinks by one line per correction signed.
const PENDING_SPDX_LIST: &str = "RELICENSING-PENDING.txt";

/// Walk `.rs` files and report every one missing its SPDX header.
fn check_spdx() -> Result<usize, std::io::Error> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                walk(&path, out)?;
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    for root in ["crates", "xtask"] {
        let dir = Path::new(root);
        if dir.is_dir() {
            walk(dir, &mut files)?;
        }
    }
    files.sort();

    // The ratchet: before the relicense there is no pending list and either
    // identifier passes; after it, the old identifier passes only for a file
    // the list names. A relicense half-applied is therefore visible, and a
    // correction that frees a file is a line removed from the list.
    let pending: Vec<String> = std::fs::read_to_string(PENDING_SPDX_LIST)
        .map(|t| {
            t.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let ratcheted = Path::new(PENDING_SPDX_LIST).exists();

    let mut missing = 0usize;
    for file in &files {
        let text = std::fs::read_to_string(file)?;
        let expected = format!("{SPDX_PREFIX}{EXPECTED_ID}");
        let prior = format!("{SPDX_PREFIX}{PRIOR_ID}");
        let as_listed = file.to_string_lossy().replace('\\', "/");
        let allowed_prior = !ratcheted || pending.iter().any(|p| p == &as_listed);
        if text.starts_with(&expected) {
            continue;
        }
        if text.starts_with(&prior) && allowed_prior {
            continue;
        }
        // Report every one, not the first: otherwise the fix is one re-run
        // per file.
        if text.starts_with(&prior) {
            println!(
                "   still AGPL and not in {PENDING_SPDX_LIST}: {}",
                file.display()
            );
        } else {
            println!("   missing SPDX header: {}", file.display());
        }
        missing += 1;
    }
    if missing == 0 {
        println!(
            "   {} file(s) carry an SPDX header{}",
            files.len(),
            if ratcheted {
                format!(" ({} awaiting a correction)", pending.len())
            } else {
                String::new()
            }
        );
    }
    Ok(missing)
}

/// Every workflow action pinned to a full commit SHA, and the Pages workflow
/// deploying the committed projection and nothing else (OW-WAR-0060).
///
/// A `uses:` pinned to a tag is a supply-chain hole: the tag can move. This
/// repository has always pinned by SHA by convention; a convention nobody checks
/// is the "string, not a gate" failure, so it is checked here.
fn check_workflows() -> Result<usize, std::io::Error> {
    let dir = Path::new(".github/workflows");
    if !dir.is_dir() {
        println!("   no workflows directory");
        return Ok(0);
    }
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .collect();
    files.sort();
    let mut problems = 0usize;
    for file in &files {
        let text = std::fs::read_to_string(file)?;
        for why in workflow_problems(&file.display().to_string(), &text) {
            println!("   {why}");
            problems += 1;
        }
    }
    if problems == 0 {
        println!(
            "   {} workflow(s): every `uses:` pinned by SHA; pages deploys the committed projection",
            files.len()
        );
    }
    Ok(problems)
}

/// The rules, on text, so they can be tested against a workflow that breaks them.
///
/// Text, not a YAML parse: OW-ADR-0002 keeps YAML libraries out of this
/// repository. So the matcher must not depend on where the key sits. It finds
/// EVERY `uses` key on a line — at the start, after `- `, inside a flow mapping
/// `{uses: …}`, quoted `"uses":`, or with space before the colon `uses :` —
/// because each of those is a form GitHub's parser accepts, and a matcher that
/// only recognised the common one passed the others vacuously (found by review).
/// A `uses` inside a comment or a block scalar is checked too: a false refusal
/// there is a nuisance, a silent pass is a hole.
fn workflow_problems(name: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        for spec in uses_specs(line) {
            if let Err(why) = pin_verdict(&spec) {
                out.push(format!("{name}:{}: `uses: {spec}` {why}", n + 1));
            }
        }
    }
    if name.ends_with("pages.yml") {
        const SOURCE: &str = "docs/warrants/generated/CORPUS_STATUS.html";
        if !text.contains(SOURCE) {
            out.push(format!(
                "{name}: the Pages workflow does not name {SOURCE}; it must deploy the committed projection"
            ));
        }
        if !text.contains("sha256sum") {
            out.push(format!(
                "{name}: the Pages workflow does not compare digests; the deploy must prove it uploaded the committed bytes"
            ));
        }
    }
    out
}

/// Every value of a `uses` key on one line, quotes removed.
fn uses_specs(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut from = 0;
    while let Some(i) = line[from..].find("uses") {
        let start = from + i;
        let end = start + 4;
        from = end;
        // A key, not part of a longer word: preceded by nothing or a
        // non-identifier byte, and followed by optional spaces and a colon
        // (an optional closing quote first, for a quoted key).
        let boundary_before =
            start == 0 || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
        if !boundary_before {
            continue;
        }
        let mut j = end;
        if j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'\'') {
            j += 1;
        }
        while j < bytes.len() && bytes[j] == b' ' {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b':' {
            continue;
        }
        j += 1;
        let value = line[j..].trim_start();
        // The value ends at whitespace, a flow-mapping delimiter, or a comment.
        let raw: String = value
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != ',' && *c != '}' && *c != '#')
            .collect();
        let spec = raw.trim_matches(|c| c == '"' || c == '\'').to_owned();
        if !spec.is_empty() {
            out.push(spec);
        }
    }
    out
}

/// Whether one `uses` ref is pinned well enough to run.
fn pin_verdict(spec: &str) -> Result<(), &'static str> {
    let hex = |r: &str, len: usize| r.len() == len && r.bytes().all(|b| b.is_ascii_hexdigit());
    if spec.starts_with("./") {
        // A local action is this repository's own code, already reviewed.
        return Ok(());
    }
    if let Some(image) = spec.strip_prefix("docker://") {
        // A container ref pins by content digest, not by commit.
        return if image
            .rsplit_once("@sha256:")
            .is_some_and(|(_, d)| hex(d, 64))
        {
            Ok(())
        } else {
            Err("is not pinned to a `@sha256:<64 hex>` image digest")
        };
    }
    if spec.rsplit_once('@').is_some_and(|(_, r)| hex(r, 40)) {
        Ok(())
    } else {
        Err("is not pinned to a 40-hex commit SHA")
    }
}

/// The Claude Code skill is what an agent reads first; a reference file the
/// SKILL.md never links is invisible, and a link to a missing file is a lie.
/// writing-for-agents (mattpocock/skills, MIT), applied: no em-dash in any
/// document an agent reads under `.claude/skills/`. The dash is where a
/// sentence hides a second sentence; an agent reads it as noise. Reports the
/// count of offending files and names each.
fn check_agent_docs() -> Result<usize, std::io::Error> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else if path.extension().is_some_and(|e| e == "md") {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    let root = Path::new(".claude/skills");
    if root.is_dir() {
        walk(root, &mut files)?;
    }
    files.sort();
    let mut bad = 0;
    for f in files {
        let text = std::fs::read_to_string(&f)?;
        if let Some(line) = text.lines().position(|l| l.contains('\u{2014}')) {
            println!("   em-dash: {}:{}", f.display(), line + 1);
            bad += 1;
        }
    }
    if bad == 0 {
        println!("   no em-dash in any agent-facing document");
    }
    Ok(bad)
}

/// Text rules, no YAML parse (OW-ADR-0002), so they are testable on strings.
fn check_skill() -> Result<usize, std::io::Error> {
    let dir = Path::new(".claude/skills/openwarrant");
    let skill = std::fs::read_to_string(dir.join("SKILL.md"))?;
    let mut refs: Vec<String> = match std::fs::read_dir(dir.join("references")) {
        Ok(rd) => rd
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".md"))
            .collect(),
        Err(_) => Vec::new(),
    };
    refs.sort();
    let mut problems = 0usize;
    for why in skill_problems(&skill, &refs) {
        println!("   {why}");
        problems += 1;
    }
    if problems == 0 {
        println!(
            "   SKILL.md: frontmatter present, {} reference(s) all linked and present",
            refs.len()
        );
    }
    Ok(problems)
}

/// The rules on text: frontmatter with `name:` and `description:`; every
/// `references/<file>.md` that exists is linked; every linked reference exists;
/// the file stays short enough to be read before the work starts.
fn skill_problems(skill: &str, references: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut parts = skill.splitn(3, "---\n");
    let (Some(""), Some(front), Some(_body)) = (parts.next(), parts.next(), parts.next()) else {
        out.push("SKILL.md: no `---` frontmatter block at the top".to_owned());
        return out;
    };
    for key in ["name:", "description:"] {
        if !front.lines().any(|l| l.starts_with(key)) {
            out.push(format!("SKILL.md: frontmatter lacks `{key}`"));
        }
    }
    for r in references {
        if !skill.contains(&format!("references/{r}")) {
            out.push(format!(
                "SKILL.md: references/{r} exists but is never linked"
            ));
        }
    }
    // Links are matched in the bare `](references/<file>)` form the shipped
    // file uses; a `./` or `../` prefix would be invisible to both checks.
    let mut rest = skill;
    while let Some(i) = rest.find("](references/") {
        let after = &rest[i + 2..];
        let end = after.find(')').unwrap_or(after.len());
        let target = &after["references/".len()..end];
        if !references.iter().any(|r| r == target) {
            out.push(format!(
                "SKILL.md: links references/{target}, which does not exist"
            ));
        }
        rest = &after[end..];
    }
    let lines = skill.lines().count();
    if lines > 120 {
        out.push(format!(
            "SKILL.md: {lines} lines; keep it under 120 — details go in references/"
        ));
    }
    out
}

/// Steps run in-process before the commands: spdx headers, workflows, skill.
const IN_PROCESS_STEPS: usize = 4;

fn gate() -> ExitCode {
    let steps = [
        // FIRST, and not merely for speed. `conformance/plant.sh` runs the
        // SHIPPED BINARY at target/debug/war, and `cargo test` does not produce
        // it — test harnesses land at target/debug/deps/war-<hash> instead.
        //
        // Without this step the gate passed on any machine where someone had
        // previously run `cargo build` and failed on a clean checkout, which is
        // exactly what happened: green locally, red in CI, with the plant step
        // reporting "build first". A gate whose result depends on leftover build
        // state is not a gate, and CONTRIBUTING.md promises that green-locally
        // and green-in-CI cannot come to mean different things.
        Step {
            label: "build (plants run the shipped binary, so it must exist)",
            program: "cargo",
            args: &["build", "--workspace"],
        },
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
        // OW-WAR-0032: the JSON Schema pack is generated from the record types
        // and drift-checked like every projection. The generator lives behind
        // the `schema` cargo feature, so this step builds it on; the shipped
        // binary the plants run is rebuilt without it below.
        Step {
            label: "schemas (generated, drift-checked)",
            program: "cargo",
            args: &[
                "run",
                "-q",
                "-p",
                "openwarrant-cli",
                "--features",
                "schema",
                "--",
                "schemas",
                "--check",
            ],
        },
        Step {
            label: "rebuild the shipped binary without the schema feature",
            program: "cargo",
            args: &["build", "--workspace"],
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
        // The REAL corpus, unmutated.
        //
        // Until this step existed, nothing in CI ever ran `war check` against
        // the corpus as committed. `conformance/plant.sh` runs it, but always on
        // a tree it has just mutated and is about to restore — so the tool that
        // validates this repository's Warrants was exercised only against
        // deliberately broken inputs, and a regression in the corpus itself was
        // invisible to the gate.
        //
        // That is not hypothetical. Twice in two days a pull request touching
        // `.github/workflows/ci.yml` invalidated OW-WAR-0001's declared artifact
        // digest, and both times the Warrant silently lost a §56.1 requirement
        // with every check green.
        //
        // `--generated` is deliberate: it also compares the committed
        // projections against a fresh compilation, so this one step covers both
        // §37.2 digest drift and §17 view drift.
        //
        // Runs BEFORE the plants because the plants require a clean
        // `docs/warrants/`, and a corpus failure here is the more specific
        // diagnosis when both would fail.
        Step {
            label: "corpus (§37.2 digests and §17 view drift, on the committed tree)",
            program: "./target/debug/war",
            args: &["check", "--generated"],
        },
        // §92's second half, and the reason the first half means anything: every
        // planted violation must be rejected BY ITS INTENDED CONTROL. The unit
        // tests above prove the code does what it says; this proves the shipped
        // binary refuses what it should, on real files, for the stated reason.
        // OW-ADR-0015: signature verification is not part of `war check`
        // (which stays deterministic and structural); it is this step.
        Step {
            label: "attestations (every DSSE envelope verifies; every subject digest holds)",
            program: "./target/debug/war",
            args: &["attest", "--all"],
        },
        Step {
            label: "planted violations (§92 — each rejected by its intended control)",
            program: "bash",
            args: &["conformance/plant.sh"],
        },
    ];

    let mut failed = Vec::new();

    // An in-process step, run first because it is instant and its failure is
    // always actionable.
    println!("== spdx headers ==");
    match check_spdx() {
        Ok(0) => println!("   ok"),
        Ok(n) => {
            println!("   FAILED ({n} file(s) missing a header)");
            failed.push("spdx headers");
        }
        Err(err) => {
            println!("   COULD NOT RUN: {err}");
            failed.push("spdx headers");
        }
    }

    println!("== workflows ==");
    match check_workflows() {
        Ok(0) => println!("   ok"),
        Ok(n) => {
            println!("   FAILED ({n} problem(s))");
            failed.push("workflows");
        }
        Err(err) => {
            println!("   COULD NOT RUN: {err}");
            failed.push("workflows");
        }
    }

    println!("== skill (SKILL.md frontmatter; every reference linked and present) ==");
    match check_skill() {
        Ok(0) => println!("   ok"),
        Ok(n) => {
            println!("   FAILED ({n} problem(s))");
            failed.push("skill");
        }
        Err(err) => {
            println!("   COULD NOT RUN: {err}");
            failed.push("skill");
        }
    }

    println!("== agent-facing documents (no em-dash under .claude/skills/) ==");
    match check_agent_docs() {
        Ok(0) => println!("   ok"),
        Ok(n) => {
            println!("   FAILED ({n} file(s))");
            failed.push("agent-facing documents");
        }
        Err(err) => {
            println!("   COULD NOT RUN: {err}");
            failed.push("agent-facing documents");
        }
    }

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
        // Four in-process steps (spdx, workflows, skill, agent docs) plus the commands.
        println!(
            "\ngate: PASS — {} step(s) green",
            steps.len() + IN_PROCESS_STEPS
        );
        ExitCode::SUCCESS
    } else {
        println!(
            "\ngate: FAIL — {}/{} step(s) failed:",
            failed.len(),
            steps.len() + IN_PROCESS_STEPS
        );
        for label in &failed {
            println!("  - {label}");
        }
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::{skill_problems, workflow_problems};

    #[test]
    fn skill_rules_catch_each_defect_and_pass_the_shipped_file() {
        let refs = vec!["loop.md".to_owned(), "mcp.md".to_owned()];
        let good =
            "---\nname: x\ndescription: y\n---\n\n[a](references/loop.md) [b](references/mcp.md)\n";
        assert!(skill_problems(good, &refs).is_empty());
        assert_eq!(skill_problems("# no frontmatter\n", &refs).len(), 1);
        let unlinked = "---\nname: x\ndescription: y\n---\n[a](references/loop.md)\n";
        assert!(skill_problems(unlinked, &refs)[0].contains("mcp.md exists but is never linked"));
        let dangling = "---\nname: x\ndescription: y\n---\n[a](references/loop.md) [b](references/mcp.md) [c](references/gone.md)\n";
        assert!(skill_problems(dangling, &refs)[0].contains("gone.md, which does not exist"));
        let no_desc = "---\nname: x\n---\n[a](references/loop.md) [b](references/mcp.md)\n";
        assert!(skill_problems(no_desc, &refs)[0].contains("`description:`"));
        // The shipped skill passes against the shipped references.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let skill =
            std::fs::read_to_string(root.join(".claude/skills/openwarrant/SKILL.md")).unwrap();
        let mut shipped: Vec<String> =
            std::fs::read_dir(root.join(".claude/skills/openwarrant/references"))
                .unwrap()
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
        shipped.sort();
        // The contract is that every shipped reference is linked and present,
        // not that the skill can never gain another task-specific reference.
        assert!(!shipped.is_empty());
        assert!(
            skill_problems(&skill, &shipped).is_empty(),
            "{:?}",
            skill_problems(&skill, &shipped)
        );
    }

    #[test]
    fn a_sha_pinned_action_passes() {
        let text =
            "steps:\n  - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7\n";
        assert!(workflow_problems(".github/workflows/ci.yml", text).is_empty());
    }

    #[test]
    fn a_tag_pinned_action_is_refused() {
        let text = "  - uses: actions/checkout@v7\n";
        let p = workflow_problems(".github/workflows/ci.yml", text);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("not pinned"), "{}", p[0]);
    }

    #[test]
    fn a_quoted_ref_is_read_without_its_quotes() {
        let ok = "  - uses: \"actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1\"\n";
        assert!(workflow_problems("x.yml", ok).is_empty());
        let bad = "  - uses: 'actions/checkout@v7'\n";
        assert_eq!(workflow_problems("x.yml", bad).len(), 1);
    }

    #[test]
    fn a_local_action_needs_no_pin() {
        assert!(workflow_problems("x.yml", "  - uses: ./.github/actions/local\n").is_empty());
    }

    #[test]
    fn a_docker_ref_pins_by_image_digest_or_is_refused() {
        let ok = format!("  - uses: docker://ghcr.io/x/y@sha256:{}\n", "a".repeat(64));
        assert!(workflow_problems("x.yml", &ok).is_empty());
        let p = workflow_problems("x.yml", "  - uses: docker://alpine:3.20\n");
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("image digest"), "{}", p[0]);
    }

    /// The three forms a start-of-line matcher passed vacuously (found by review).
    #[test]
    fn a_flow_mapping_a_quoted_key_and_a_spaced_colon_are_all_checked() {
        for line in [
            "  - {uses: actions/checkout@v1, with: {ref: main}}\n",
            "  \"uses\": actions/checkout@v1\n",
            "  'uses': actions/checkout@v1\n",
            "    uses : actions/checkout@v1\n",
            "  - {name: x, uses: actions/checkout@v1}\n",
        ] {
            let p = workflow_problems("x.yml", line);
            assert_eq!(p.len(), 1, "{line:?} must be refused: {p:?}");
        }
        let ok = "  - {uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1, with: {ref: main}}\n";
        assert!(workflow_problems("x.yml", ok).is_empty());
    }

    #[test]
    fn a_word_containing_uses_is_not_a_key_and_a_comment_is_skipped() {
        assert!(workflow_problems("x.yml", "  reuses: nothing@v1\n  causes: x@v1\n").is_empty());
        assert!(workflow_problems("x.yml", "  # uses: actions/checkout@v1\n").is_empty());
        assert!(workflow_problems("x.yml", "  run: echo uses\n").is_empty());
    }

    #[test]
    fn a_short_sha_is_refused() {
        let text = "  - uses: actions/checkout@3d3c42e5\n";
        assert_eq!(workflow_problems("x.yml", text).len(), 1);
    }

    #[test]
    fn the_pages_workflow_must_deploy_the_committed_projection_and_prove_it() {
        let text = "  - uses: a/b@3d3c42e5aac5ba805825da76410c181273ba90b1\n  run: cp somewhere/else.html _site/index.html\n";
        let p = workflow_problems(".github/workflows/pages.yml", text);
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(p.iter().any(|w| w.contains("CORPUS_STATUS.html")));
        assert!(p.iter().any(|w| w.contains("compare digests")));
    }

    #[test]
    fn the_real_pages_workflow_passes() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/pages.yml");
        let text = std::fs::read_to_string(path).expect("pages.yml");
        assert!(workflow_problems(".github/workflows/pages.yml", &text).is_empty());
    }
}
