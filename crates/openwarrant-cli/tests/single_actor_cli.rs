// SPDX-License-Identifier: Apache-2.0
//! Issue #132: `[independence] single_actor = true` (item 3), and `war sign`'s
//! structural refusals said once, with the register's resolved path, before
//! any sign screen (item 1).
use std::{fs, path::Path, path::PathBuf, process::Command};

fn war(root: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_war"))
        .arg("--root")
        .arg(root)
        .args(args)
        .arg("--json")
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_AGENT_PID")
        .env_remove("OPENWARRANT_ACTOR")
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    let value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "args={args:?} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().unwrap_or(-1), value)
}

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new(env!("CARGO_BIN_EXE_war"))
            .arg("--root")
            .arg(&root)
            .args(["init", "--namespace", "SA"])
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .output()
            .unwrap()
            .status
            .success()
    );
    // Two acts awaiting authorization, so a sweep has something to stop.
    assert_eq!(war(&root, &["new", "count me"]).0, 0);
    assert_eq!(war(&root, &["new", "count me too"]).0, 0);
    root
}

const NINE_FALSE: &str = "performer_transcript_blind = false
performer_rationale_blind = false
separate_writable_workspace = false
cannot_modify_subject_artifacts = false
cannot_modify_gate_definition = false
cannot_modify_gate_fixtures = false
separate_context_compilation = false
distinct_model_required = false
distinct_human_required = false
";

fn set_independence(root: &Path, base: &str, table: &str) {
    fs::write(
        root.join("openwarrant.toml"),
        format!("{base}\n[independence]\n{table}"),
    )
    .unwrap();
}

/// `(severity:rule)` of every `independence.*` diagnostic, sorted, and the
/// notes that start with "independence".
fn independence_findings(v: &serde_json::Value) -> (Vec<String>, Vec<String>) {
    let mut d: Vec<String> = v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["rule"].as_str().unwrap().starts_with("independence."))
        .map(|d| {
            format!(
                "{}:{}",
                d["severity"].as_str().unwrap(),
                d["rule"].as_str().unwrap()
            )
        })
        .collect();
    d.sort();
    let n = v["notes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n.as_str())
        .filter(|n| n.starts_with("independence"))
        .map(str::to_owned)
        .collect();
    (d, n)
}

#[test]
fn single_actor_states_the_gap_once_and_never_reads_as_verified() {
    let root = scratch("single-actor");
    let base = fs::read_to_string(root.join("openwarrant.toml")).unwrap();

    // Undeclared, nine false: the WARN this declaration answers.
    set_independence(&root, &base, NINE_FALSE);
    let (_, none) = war(&root, &["check"]);
    assert_eq!(
        independence_findings(&none),
        (vec!["warn:independence.insufficient".to_owned()], vec![])
    );
    let (_, score_none) = war(&root, &["admin", "score"]);

    // Declared: no independence.* diagnostic at all (no WARN, no PASS), one
    // note naming the declaration, and the score is unmoved.
    set_independence(&root, &base, "single_actor = true\n");
    let (code, single) = war(&root, &["check"]);
    assert_eq!(code, 0, "{single}");
    let (diags, notes) = independence_findings(&single);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(
        notes[0].contains("single_actor = true") && notes[0].contains("is unmet for"),
        "{}",
        notes[0]
    );
    let (_, score_single) = war(&root, &["admin", "score"]);
    assert_eq!(
        score_none["result"], score_single["result"],
        "the declaration earns nothing in the score"
    );
    let (_, status) = war(&root, &["status"]);
    assert_eq!(
        status["result"]["independence"],
        serde_json::json!({"declared": "single_actor", "independent": false})
    );
    assert_eq!(status["result"]["compliance"], score_single["result"]);

    // Contradicted: refused by name, and the WARN is not suppressed.
    set_independence(
        &root,
        &base,
        &format!(
            "{}single_actor = true\n",
            NINE_FALSE.replace(
                "separate_writable_workspace = false",
                "separate_writable_workspace = true"
            )
        ),
    );
    let (code, contra) = war(&root, &["check"]);
    assert_eq!(code, 2, "{contra}");
    assert_eq!(
        independence_findings(&contra).0,
        [
            "error:independence.single-actor-contradicted",
            "warn:independence.insufficient"
        ]
    );
    let message = contra["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["rule"] == "independence.single-actor-contradicted")
        .unwrap()["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(message.contains("separate_writable_workspace"), "{message}");
    let (_, status) = war(&root, &["status"]);
    assert_eq!(
        status["result"]["independence"]["contradicted_by"],
        serde_json::json!(["separate_writable_workspace"])
    );

    // No declaration: `war status` carries no independence key.
    set_independence(&root, &base, NINE_FALSE);
    let (_, status) = war(&root, &["status"]);
    assert!(status["result"].get("independence").is_none());
    fs::remove_dir_all(root).unwrap();
}

fn rules(v: &serde_json::Value) -> Vec<String> {
    v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["rule"].as_str().unwrap().to_owned())
        .collect()
}

/// Item 1: with no register, `war sign --all` names the resolved path, says
/// absent, points at the example, and stops after the first refusal.
#[test]
fn sign_all_with_no_register_refuses_once_naming_the_path() {
    let root = scratch("sign-absent");
    let (code, v) = war(&root, &["sign", "--all", "--ssh-sign"]);
    assert_ne!(code, 0);
    assert_eq!(rules(&v), ["sign.who", "sign.sweep-stopped"], "{v}");
    let m = v["diagnostics"][0]["message"].as_str().unwrap();
    let canon = fs::canonicalize(&root).unwrap();
    let resolved = canon.join("docs/authority/roles.toml");
    assert!(
        m.contains(&format!("{} does not exist (absent", resolved.display())),
        "{m}"
    );
    assert!(m.contains("roles.toml.example"), "{m}");
    assert!(
        v["diagnostics"][1]["message"]
            .as_str()
            .unwrap()
            .contains("1 more act(s) not attempted"),
        "{v}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// Item 1: a register whose only human has no `ssh_principal` is refused
/// `sign.ssh-principal` before any sign screen (the JSON on stdout would not
/// parse if a screen had been printed), once under `--all`. Nothing reaches
/// an ssh agent: SSH_AUTH_SOCK is removed and no signature is attempted.
#[test]
fn ssh_sign_refuses_a_missing_principal_before_the_screen_and_once() {
    let root = scratch("sign-principal");
    fs::create_dir_all(root.join("docs/authority")).unwrap();
    fs::write(
        root.join("docs/authority/roles.toml"),
        "[[assignment]]\nactor = \"ada\"\nactor_kind = \"human\"\n\
         roles = [\"authorizer\", \"resolver\"]\nassigned_by = \"ada\"\n\
         effective_time = \"2026-01-01T00:00:00Z\"\n",
    )
    .unwrap();
    let (code, v) = war(&root, &["sign", "--all", "--ssh-sign"]);
    assert_ne!(code, 0);
    assert_eq!(
        rules(&v),
        ["sign.ssh-principal", "sign.sweep-stopped"],
        "{v}"
    );
    let responses = root.join("docs/authority/responses");
    let written = fs::read_dir(&responses).map_or(0, |d| d.count());
    assert_eq!(written, 0, "no draft or response is left behind");
    // Refused, not stopped: one act named alone is refused the same way.
    let (_, one) = war(&root, &["sign", "SA-WAR-0001", "--ssh-sign"]);
    assert_eq!(rules(&one), ["sign.ssh-principal"], "{one}");
    fs::remove_dir_all(root).unwrap();
}
