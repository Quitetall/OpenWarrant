// SPDX-License-Identifier: Apache-2.0
//! Synthetic recorder/journal controls. The fixture gate executes `true`;
//! it establishes no implementation behavior, independent review or authority.
#![cfg(unix)]
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        fn copy(from: &Path, to: &Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let dest = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &dest);
                } else {
                    fs::copy(entry.path(), dest).unwrap();
                }
            }
        }
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ow-evidence-recorder-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        copy(&repo.join("conformance/fixtures/inbox/repository"), &root);
        fs::create_dir_all(root.join("docs/gates")).unwrap();
        let mut gate = fs::read_to_string(repo.join("docs/gates/ops.echo@1.0.0.yaml"))
            .unwrap()
            .replace(
                "gate_id: \"ops.echo\"",
                "gate_id: \"software.repo.war-check\"",
            );
        // Explicit input hashing still needs Git's inventory. An empty owned
        // repository suffices; no fake author, commit or signing key is needed.
        gate.push_str("\ninputs: [\"openwarrant.toml\"]\n");
        fs::write(
            root.join("docs/gates/software.repo.war-check@1.0.0.yaml"),
            gate,
        )
        .unwrap();
        assert!(
            Command::new("git")
                .current_dir(&root)
                .args(["init", "-q"])
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .status()
                .unwrap()
                .success()
        );
        let f = Self(root);
        f.run(&["admin", "compile", "--json"]);
        f
    }
    fn run(&self, args: &[&str]) -> serde_json::Value {
        let out = Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&self.0)
            .args(args)
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)))
    }
    fn record(&self, actor: Option<&str>) -> serde_json::Value {
        let mut args = vec!["evidence", "record", "IX-WAR-0003", "--json"];
        if let Some(actor) = actor {
            args.extend(["--as", actor]);
        }
        self.run(&args)
    }
    fn journal(&self) -> Vec<serde_json::Value> {
        let p = self.0.join("docs/warrants/IX-WAR-0003/journal.jsonl");
        fs::read_to_string(p)
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn has(r: &serde_json::Value, rule: &str) -> bool {
    r["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["rule"] == rule)
}

#[test]
fn explicit_agent_attribution_preserves_policy_independence_and_retry_conflicts() {
    let f = Fixture::new();
    let config = fs::read(f.0.join("openwarrant.toml")).unwrap();
    let first = f.record(Some("agent://codex"));
    assert_eq!(first["exit_code"], 0, "{first}");
    assert!(has(&first, "gate-run.receipt"), "{first}");
    let journal = f.journal();
    let event = journal
        .iter()
        .find(|e| e["type"] == "sync.receipt_attached")
        .unwrap();
    assert_eq!(event["actor_ref"], "agent://codex");
    assert_eq!(fs::read(f.0.join("openwarrant.toml")).unwrap(), config);
    let replay = f.record(Some("agent://codex"));
    assert!(
        has(&replay, "evidence.replayed"),
        "{replay}; current evidence diagnostics: {}",
        f.run(&["check", "IX-WAR-0003", "--json"])
    );
    assert_eq!(f.journal(), journal);
    let conflict = f.record(Some("agent://other"));
    assert_ne!(conflict["exit_code"], 0, "{conflict}");
    assert_eq!(f.journal(), journal);
    let closure = f.run(&["resolve", "IX-WAR-0003", "--dry-run", "--json"]);
    assert_ne!(closure["exit_code"], 0, "{closure}");
    assert!(
        closure["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "resolution.requirement-unmet"
                && d["message"]
                    .as_str()
                    .unwrap()
                    .contains("independence requirements")),
        "{closure}"
    );
}

#[test]
fn nonagent_malformed_and_oversized_recorders_refuse_before_writes() {
    let f = Fixture::new();
    let before = f.journal();
    for actor in [
        "human://owner".to_owned(),
        "agent://".to_owned(),
        "agent://../x".to_owned(),
        "agent://codex\nforged".to_owned(),
        format!("agent://{}", "x".repeat(257)),
    ] {
        let r = f.record(Some(&actor));
        assert!(has(&r, "evidence.recorder-invalid"), "{r}");
        assert_ne!(r["exit_code"], 0);
        assert_eq!(f.journal(), before);
        assert!(!f.0.join("docs/warrants/IX-WAR-0003/gate-runs").exists());
    }
}

#[test]
fn absent_recorder_retains_legacy_configured_performer_label() {
    let f = Fixture::new();
    let r = f.record(None);
    assert_eq!(r["exit_code"], 0, "{r}");
    assert!(
        f.journal()
            .iter()
            .any(|e| e["type"] == "sync.receipt_attached" && e["actor_ref"] == "agent://claude")
    );
}
