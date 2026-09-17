// SPDX-License-Identifier: Apache-2.0
use std::{path::PathBuf, process::Command};
#[test]
fn unknown_runtime_blocks_without_omitting_any_dimension() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/fixtures/inbox/repository");
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["preflight", "IX-WAR-0001", "--json"])
        .current_dir(root)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["verdict"], "not_ready");
    let result = &v["result"];
    assert_eq!(result["readiness"], "not_ready");
    assert_eq!(result["execution_condition"], "blocked");
    let groups = result["dimensions"].as_array().unwrap();
    assert_eq!(
        groups
            .iter()
            .map(|g| g["group"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "contract",
            "context",
            "graph",
            "runtime",
            "gates",
            "authority"
        ]
    );
    assert_eq!(
        groups
            .iter()
            .map(|g| g["checks"].as_array().unwrap().len())
            .collect::<Vec<_>>(),
        [6, 6, 5, 10, 9, 5]
    );
    assert_eq!(groups[3]["status"], "unknown");
    assert!(
        groups[3]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |c| c["name"] == "actual network path usable from the actor environment"
                    && c["status"] == "unknown"
            )
    );
}

fn fixture(name: &str) -> PathBuf {
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let dest = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &dest);
            } else {
                std::fs::copy(entry.path(), dest).unwrap();
            }
        }
    }
    let dir = std::env::temp_dir().join(format!("ow11-{name}-{}", std::process::id()));
    copy(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/fixtures/inbox/repository"),
        &dir,
    );
    dir
}
fn run(root: &std::path::Path, alias: &str) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["preflight", alias, "--json"])
        .current_dir(root)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    serde_json::from_slice(&out.stdout).unwrap()
}
fn check<'a>(v: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    v["result"]["dimensions"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|d| d["checks"].as_array().unwrap())
        .find(|c| c["name"] == name)
        .unwrap()
}
#[test]
fn reproduced_commitment_does_not_claim_authority_or_runtime() {
    let root = fixture("digest");
    let v = run(&root, "IX-WAR-0003");
    assert_eq!(check(&v, "profile valid")["status"], "pass");
    assert_eq!(check(&v, "contract digest reproducible")["status"], "pass");
    assert_eq!(check(&v, "authorization valid")["status"], "unknown");
    assert_eq!(v["result"]["state_persisted"], false);
    assert!(
        v["result"]["meaning"]
            .as_str()
            .unwrap()
            .contains("does not prove the deliverable correct")
    );
    let path = root.join("docs/warrants/IX-WAR-0003/atoms/10-intent.md");
    let before = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{before}\nChanged required outcome.\n")).unwrap();
    let changed = run(&root, "IX-WAR-0003");
    assert_eq!(check(&changed, "authorization valid")["status"], "fail");
    assert_ne!(
        v["result"]["contract_digest"],
        changed["result"]["contract_digest"]
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn missing_atom_and_cyclic_graph_refuse_with_named_checks() {
    let root = fixture("bad-inputs");
    let atom = root.join("docs/warrants/IX-WAR-0001/atoms/10-intent.md");
    std::fs::remove_file(atom).unwrap();
    let v = run(&root, "IX-WAR-0001");
    assert_eq!(check(&v, "required atoms present")["status"], "fail");
    assert_eq!(v["result"]["dimensions"].as_array().unwrap().len(), 6);
    let graph = root.join("docs/warrants/IX-WAR-0003/atoms/45-milestones.yaml");
    std::fs::write(&graph,"schema: oh.war/milestones/v1\nmilestones:\n  - id: M1\n    depends_on: [M2]\n  - id: M2\n    depends_on: [M1]\nstages: []\n").unwrap();
    let v = run(&root, "IX-WAR-0003");
    assert_eq!(check(&v, "no stage or milestone cycle")["status"], "fail");
    assert_eq!(check(&v, "required stages reachable")["status"], "unknown");
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn read_only_and_human_output_names_all_dimensions() {
    fn files(root: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
        let mut map = std::collections::BTreeMap::new();
        for e in std::fs::read_dir(root).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                map.extend(files(&e.path()));
            } else {
                map.insert(e.path(), std::fs::read(e.path()).unwrap());
            }
        }
        map
    }
    let root = fixture("readonly");
    let before = files(&root);
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["preflight", "IX-WAR-0003"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let text = String::from_utf8(out.stdout).unwrap();
    for group in [
        "contract:",
        "context:",
        "graph:",
        "runtime:",
        "gates:",
        "authority:",
    ] {
        assert!(text.contains(group), "{group}");
    }
    assert!(text.contains("UNKNOWN actual network path"));
    assert!(!text.contains("WELL-FORMED"));
    assert_eq!(before, files(&root));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn later_graph_cannot_hide_an_earlier_failure() {
    let root = fixture("multiple-graphs");
    let dir = root.join("docs/warrants/IX-WAR-0003");
    let manifest = dir.join("manifest.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(manifest, format!("{text}\n[[atoms]]\nordinal = 46\nrole = \"milestones\"\npath = \"atoms/46-extra.yaml\"\nrequired = true\n")).unwrap();
    std::fs::write(
        dir.join("atoms/45-milestones.yaml"),
        "schema: oh.war/milestones/v1\nmilestones:\n  - id: M1\n    depends_on: [M1]\nstages: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("atoms/46-extra.yaml"),
        "schema: oh.war/milestones/v1\nmilestones: []\nstages: []\n",
    )
    .unwrap();
    let v = run(&root, "IX-WAR-0003");
    assert_eq!(check(&v, "no stage or milestone cycle")["status"], "fail");
    assert_eq!(check(&v, "required stages reachable")["status"], "unknown");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unavailable_authorization_is_not_reported_absent() {
    let root = fixture("authorization-container");
    let path = root.join("docs/warrants/IX-WAR-0003/authorization.toml");
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let v = run(&root, "IX-WAR-0003");
    let auth = check(&v, "authorization valid");
    assert_eq!(auth["status"], "unknown");
    assert!(
        auth["observation"]
            .as_str()
            .unwrap()
            .contains("wrong container type")
    );
    assert_eq!(v["result"]["dimensions"].as_array().unwrap().len(), 6);
    std::fs::remove_dir(&path).unwrap();
    std::fs::write(&path, "broken toml = [").unwrap();
    let v = run(&root, "IX-WAR-0003");
    assert_eq!(check(&v, "authorization valid")["status"], "unknown");
    assert!(
        check(&v, "authorization valid")["observation"]
            .as_str()
            .unwrap()
            .contains("could not parse")
    );
    std::fs::remove_dir_all(root).unwrap();
}
