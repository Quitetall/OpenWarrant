// SPDX-License-Identifier: Apache-2.0
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn war(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn fixture(name: &str) -> PathBuf {
    fn copy(a: &Path, b: &Path) {
        fs::create_dir_all(b).unwrap();
        for e in fs::read_dir(a).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                copy(&e.path(), &b.join(e.file_name()));
            } else {
                fs::copy(e.path(), b.join(e.file_name())).unwrap();
            }
        }
    }
    let p = std::env::temp_dir().join(format!("ow23-{name}-{}", std::process::id()));
    copy(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/fixtures/inbox/repository"),
        &p.join("repo"),
    );
    let graph = p.join("repo/docs/warrants/IX-WAR-0003/atoms/45-milestones.yaml");
    fs::write(
        &graph,
        fs::read_to_string(&graph).unwrap().replace(
            "executor_kind: \"agent\"",
            "executor_kind: \"agent\"\n    executor_ref: \"agent://fixture\"",
        ),
    )
    .unwrap();
    p
}
#[test]
fn portable_context_survives_removal_of_source_repository_and_denies_unlisted_capability() {
    let root = fixture("offline");
    let repo = root.join("repo");
    let dispatch = root.join("dispatch.json");
    let context = root.join("context.json");
    let out = war(
        &repo,
        &[
            "dispatch",
            "IX-WAR-0003",
            "STAGE-001",
            "--emit",
            dispatch.to_str().unwrap(),
            "--emit-context",
            context.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bundle = root.join("bundle.json");
    let out = war(
        &repo,
        &[
            "dispatch-bundle",
            "create",
            "IX-WAR-0003",
            "--dispatch",
            dispatch.to_str().unwrap(),
            "--context",
            context.to_str().unwrap(),
            "--emit",
            bundle.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let digest = report["result"]["bundle_digest"].as_str().unwrap();
    let expected = fs::read(repo.join("docs/warrants/IX-WAR-0003/atoms/10-intent.md")).unwrap();
    fs::remove_dir_all(&repo).unwrap();
    let out = war(
        &root,
        &[
            "dispatch-bundle",
            "check",
            bundle.to_str().unwrap(),
            "--expected-digest",
            digest,
            "--read",
            "atoms/10-intent.md",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        report["result"]["content"],
        String::from_utf8(expected).unwrap()
    );
    assert_eq!(report["result"]["execution_authorized"], false);
    let out = war(
        &root,
        &[
            "dispatch-bundle",
            "check",
            bundle.to_str().unwrap(),
            "--expected-digest",
            digest,
            "--require-capability",
            "network:write",
            "--json",
        ],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("capability-denied"));
    fs::remove_dir_all(root).unwrap();
}

fn compiled(root: &Path) -> (PathBuf, PathBuf) {
    let d = root.join("dispatch.json");
    let c = root.join("context.json");
    let out = war(
        &root.join("repo"),
        &[
            "dispatch",
            "IX-WAR-0003",
            "STAGE-001",
            "--emit",
            d.to_str().unwrap(),
            "--emit-context",
            c.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (d, c)
}
fn create(root: &Path, d: &Path, c: &Path, b: &Path, extra: &[&str]) -> Output {
    let mut args = vec![
        "dispatch-bundle",
        "create",
        "IX-WAR-0003",
        "--dispatch",
        d.to_str().unwrap(),
        "--context",
        c.to_str().unwrap(),
        "--emit",
        b.to_str().unwrap(),
        "--json",
    ];
    args.extend_from_slice(extra);
    war(&root.join("repo"), &args)
}
#[test]
fn listed_capability_requires_external_policy_trust_and_never_implies_sandbox() {
    use openwarrant_compiler::{
        DigestDomain,
        dispatch_bundle::{self as b, Policy},
        sha256_digest,
    };
    let root = fixture("policy");
    let (d, c) = compiled(&root);
    let policy = serde_jcs::to_vec(&Policy {
        schema: "oh.war/dispatch-capabilities/1".into(),
        policy_ref: "policy://fixture".into(),
        allow: vec!["fs:read".into()],
    })
    .unwrap();
    let policy_digest = b::content_digest(&policy);
    let p = root.join("policy.json");
    fs::write(&p, policy).unwrap();
    // A synthetic producer binds its explicit policy through the existing
    // Dispatch SDK; these fixture records are not human authorization.
    let mut dispatch: openwarrant_core::execution::StageDispatch =
        serde_json::from_slice(&fs::read(&d).unwrap()).unwrap();
    dispatch.capability_authorization.policy_ref = "policy://fixture".into();
    dispatch.capability_authorization.digest = policy_digest.clone();
    dispatch.dispatch_digest.clear();
    dispatch.dispatch_digest = sha256_digest(DigestDomain::Dispatch, &dispatch).unwrap();
    fs::write(&d, serde_jcs::to_vec(&dispatch).unwrap()).unwrap();
    let bundle = root.join("bundle.json");
    let out = create(&root, &d, &c, &bundle, &["--policy", p.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let bytes = fs::read(&bundle).unwrap();
    let digest = b::content_digest(&bytes);
    let base = [
        "dispatch-bundle",
        "check",
        bundle.to_str().unwrap(),
        "--expected-digest",
        &digest,
        "--require-capability",
        "fs:read",
        "--json",
    ];
    let out = war(&root, &base);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("capability-policy-not-trusted"));
    let mut args = base.to_vec();
    args.extend(["--trusted-policy-digest", &policy_digest]);
    let out = war(&root, &args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["result"]["execution_authorized"], false);
    args.extend(["--require-capability", "fs:write"]);
    let out = war(&root, &args);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("capability-denied: fs:write"));
    // Same packet consumed directly by the public SDK, with no filesystem.
    let checked = b::check(&bytes, &digest).unwrap();
    checked
        .require_capability("fs:read", Some(&policy_digest))
        .unwrap();
    assert!(
        checked
            .require_capability("fs:write", Some(&policy_digest))
            .is_err()
    );
    let mut human = checked.dispatch().clone();
    human.objective = "Human review view".into();
    assert!(
        checked
            .dispatch()
            .same_normative_contract_as(&human, "human")
            .is_ok()
    );
    human.contract_digest = "changed".into();
    assert!(
        checked
            .dispatch()
            .same_normative_contract_as(&human, "human")
            .is_err()
    );
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn missing_or_changed_bytes_refuse_even_when_outer_digest_is_recomputed() {
    use openwarrant_compiler::dispatch_bundle as b;
    let root = fixture("tamper");
    let (d, c) = compiled(&root);
    let dest = root.join("bundle.json");
    let out = create(&root, &d, &c, &dest, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let original = fs::read(&dest).unwrap();
    let bundle: b::Bundle = serde_json::from_slice(&original).unwrap();
    for (label, mut changed) in [
        ("missing", bundle.clone()),
        ("tampered", bundle.clone()),
        ("policy", bundle.clone()),
        ("schema", bundle.clone()),
        ("reserved", bundle.clone()),
        ("basis-metadata", bundle.clone()),
        ("composition-metadata", bundle.clone()),
        ("ir-kind", bundle.clone()),
    ] {
        match label {
            "missing"=>{changed.sources.remove("atoms/10-intent.md");},
            "tampered"=>changed.sources.get_mut("atoms/10-intent.md").unwrap().push(b'x'),
            "schema"=>changed.sources.get_mut("schema://oh.war/stage-submission/v1").unwrap().push(b'x'),
            _=>changed.policy=br#"{"schema":"oh.war/dispatch-capabilities/1","policy_ref":"policy://none-declared","allow":["network:write"]}"#.to_vec(),
        }
        let bytes = serde_jcs::to_vec(&changed).unwrap();
        let digest = b::content_digest(&bytes);
        assert!(b::check(&bytes, &digest).is_err(), "{label}");
    }
    let mut raw = original.clone();
    raw.push(b' ');
    assert!(b::check(&raw, &b::content_digest(&original)).is_err());
    assert!(b::check(&raw, &b::content_digest(&raw)).is_err());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn changed_context_and_escaping_source_refuse_before_writing_output() {
    let root = fixture("capture-refusals");
    let (d, c) = compiled(&root);
    let target = root.join("repo/docs/warrants/IX-WAR-0003/atoms/10-intent.md");
    let original = fs::read(&target).unwrap();
    fs::write(&target, b"changed").unwrap();
    let out = create(&root, &d, &c, &root.join("changed.json"), &[]);
    assert!(!out.status.success());
    assert!(!root.join("changed.json").exists());
    fs::write(&target, original).unwrap();
    let mut context: serde_json::Value = serde_json::from_slice(&fs::read(&c).unwrap()).unwrap();
    context["included"][0]["holder"]["path"] = "../secret".into();
    fs::write(&c, serde_json::to_vec(&context).unwrap()).unwrap();
    let out = create(&root, &d, &c, &root.join("escape.json"), &[]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("bundle-source-unreadable"));
    assert!(!root.join("escape.json").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn dispatch_support_references_resolve_inside_the_detached_bundle() {
    use openwarrant_compiler::dispatch_bundle as b;
    let root = fixture("support-refs");
    let (d, c) = compiled(&root);
    let dest = root.join("bundle.json");
    let out = create(&root, &d, &c, &dest, &[]);
    assert!(out.status.success());
    let bytes = fs::read(&dest).unwrap();
    let digest = b::content_digest(&bytes);
    fs::remove_dir_all(root).unwrap();
    let checked = b::check(&bytes, &digest).unwrap();
    let dispatch = checked.dispatch();
    for id in [
        &dispatch.workspace_basis_ref,
        &dispatch.context_manifest_ref,
        &dispatch.warrant_ref,
        &dispatch.capability_authorization.policy_ref,
        &dispatch.submission_schema_ref,
    ] {
        let value = checked.read(id).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert!(!value.is_empty());
    }
}

fn optional_atom(root: &Path, section: bool) {
    let dir = root.join("repo/docs/warrants/IX-WAR-0003");
    let p = dir.join("manifest.toml");
    let old = fs::read_to_string(&p).unwrap();
    fs::write(&p,format!("{old}\n[[atoms]]\nordinal = 70\nrole = \"lab.protocol\"\npath = \"atoms/70-note.md\"\nrequired = false\n")).unwrap();
    fs::write(dir.join("atoms/70-note.md"),"---\nschema: oh.war/atom/v1\nrole: lab.protocol\n---\n\n# Notes\n\n## Selected\n\nExact useful context.\n\n## Other\n\nBackground.\n").unwrap();
    let graph = dir.join("atoms/45-milestones.yaml");
    let text = fs::read_to_string(&graph).unwrap();
    let declaration = if section {
        "context_sections: [\"atoms/70-note.md#Selected\"]"
    } else {
        "context_atoms: [\"atoms/70-note.md\"]"
    };
    fs::write(
        graph,
        text.replace(
            "executor_ref: \"agent://fixture\"",
            &format!("executor_ref: \"agent://fixture\"\n    {declaration}"),
        ),
    )
    .unwrap();
}
#[test]
fn resealing_optional_context_cannot_change_the_captured_contract() {
    use openwarrant_compiler::{DigestDomain, dispatch_bundle as b, sha256_digest};
    let root = fixture("optional-binding");
    optional_atom(&root, false);
    let (d, c) = compiled(&root);
    let dest = root.join("bundle.json");
    let out = create(&root, &d, &c, &dest, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let mut bundle: b::Bundle = serde_json::from_slice(&fs::read(dest).unwrap()).unwrap();
    let bytes = b"Contradictory optional context.".to_vec();
    bundle
        .context
        .included
        .iter_mut()
        .find(|i| i.id == "atoms/70-note.md")
        .unwrap()
        .content_digest = b::content_digest(&bytes);
    bundle.sources.insert("atoms/70-note.md".into(), bytes);
    bundle.dispatch.context_manifest_digest =
        sha256_digest(DigestDomain::ContextManifest, &bundle.context).unwrap();
    bundle.dispatch.context_manifest_ref = format!(
        "artifact://context-manifest/sha256:{}",
        bundle.dispatch.context_manifest_digest
    );
    bundle.dispatch.dispatch_digest.clear();
    bundle.dispatch.dispatch_digest =
        sha256_digest(DigestDomain::Dispatch, &bundle.dispatch).unwrap();
    let bytes = serde_jcs::to_vec(&bundle).unwrap();
    assert!(
        b::check(&bytes, &b::content_digest(&bytes))
            .err()
            .unwrap()
            .to_string()
            .contains("bundle-source-digest")
    );
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn selected_section_keeps_exact_provenance_and_rejects_deep_typed_metadata() {
    use openwarrant_compiler::dispatch_bundle as b;
    let root = fixture("section");
    optional_atom(&root, true);
    let (d, c) = compiled(&root);
    let dest = root.join("bundle.json");
    let out = create(&root, &d, &c, &dest, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let bytes = fs::read(dest).unwrap();
    let checked = b::check(&bytes, &b::content_digest(&bytes)).unwrap();
    let text = String::from_utf8(
        checked
            .read("atoms/70-note.md#Selected")
            .unwrap()
            .into_owned(),
    )
    .unwrap();
    assert!(text.contains("Exact useful context."));
    assert!(!text.contains("Background."));
    let mut bundle: b::Bundle = serde_json::from_slice(&bytes).unwrap();
    bundle
        .sources
        .remove("provenance://docs/warrants/IX-WAR-0003/atoms/70-note.md");
    assert!(bundle.encode().is_err());
    let mut bundle: b::Bundle = serde_json::from_slice(&bytes).unwrap();
    let mut deep = serde_json::Value::Null;
    for _ in 0..20_000 {
        deep = serde_json::Value::Array(vec![deep]);
    }
    bundle.contract.extensions = Some(deep);
    let result = bundle.encode();
    // Do not recursively destroy the adversarial caller-owned tree. The API
    // borrows it and must return without cloning, walking or owning it.
    std::mem::forget(bundle);
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("bundle-unsupported-ir-section")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn input_attachments_are_required_and_packaging_is_deterministic_and_non_overwriting() {
    let root = fixture("attachments");
    let graph = root.join("repo/docs/warrants/IX-WAR-0003/atoms/45-milestones.yaml");
    let text = fs::read_to_string(&graph).unwrap();
    fs::write(
        &graph,
        text.replace(
            "executor_ref: \"agent://fixture\"",
            "executor_ref: \"agent://fixture\"\n    inputs: [\"specimen:bytes\"]",
        ),
    )
    .unwrap();
    let (d, c) = compiled(&root);
    let one = root.join("one.json");
    let out = create(&root, &d, &c, &one, &[]);
    assert!(!out.status.success());
    assert!(!one.exists());
    assert!(String::from_utf8_lossy(&out.stdout).contains("bundle-missing-source: specimen"));
    let specimen = root.join("specimen.bin");
    fs::write(&specimen, [0u8, 255, 42]).unwrap();
    let arg = format!("specimen={}", specimen.display());
    let out = create(&root, &d, &c, &one, &["--attachment", &arg]);
    assert!(out.status.success());
    let two = root.join("two.json");
    let out = create(&root, &d, &c, &two, &["--attachment", &arg]);
    assert!(out.status.success());
    assert_eq!(fs::read(&one).unwrap(), fs::read(&two).unwrap());
    let before = fs::read(&one).unwrap();
    let out = create(&root, &d, &c, &one, &["--attachment", &arg]);
    assert!(!out.status.success());
    assert_eq!(fs::read(one).unwrap(), before);
    fs::remove_dir_all(root).unwrap();
}
