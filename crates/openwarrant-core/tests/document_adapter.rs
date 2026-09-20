// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::{
    adapter::*,
    packet::{CompileBasis, PackageLimits},
};
use std::collections::{BTreeMap, BTreeSet};
fn package_files() -> std::collections::BTreeMap<String, Vec<u8>> {
    use std::{fs, path::Path};
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/sas/drafts/1.0.0-rc.2/examples/expected/signup-packet");
    let mut files = std::collections::BTreeMap::new();
    for name in ["ENTRY.md", "packet.json", "manifest.json"] {
        files.insert(name.into(), fs::read(root.join(name)).unwrap());
    }
    for file in fs::read_dir(root.join("blobs")).unwrap() {
        let file = file.unwrap();
        files.insert(
            format!("blobs/{}", file.file_name().to_str().unwrap()),
            fs::read(file.path()).unwrap(),
        );
    }
    files
}

fn setup() -> (CompileBasis, BTreeMap<String, Vec<u8>>, ProviderInfo) {
    let files = package_files();
    let manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    let basis: CompileBasis = serde_json::from_value(manifest["basis"].clone()).unwrap();
    let blobs = files
        .into_iter()
        .filter_map(|(p, b)| {
            p.strip_prefix("blobs/")
                .and_then(|p| p.strip_suffix(".bin"))
                .map(|h| (format!("sha256:{h}"), b))
        })
        .collect();
    let info = ProviderInfo {
        profile: PROFILE.into(),
        compiler: basis.compiler.clone(),
        capabilities: BTreeSet::from([Capability::TaskPackage]),
    };
    (basis, blobs, info)
}
#[test]
fn explicit_negotiation_and_response_identity_refuse_wrong_boundaries() {
    let (basis, blobs, info) = setup();
    let required = BTreeSet::from([Capability::TaskPackage]);
    let mut wrong = info.clone();
    wrong.profile = "unknown".into();
    assert_eq!(
        prepare(
            &basis,
            blobs.clone(),
            wrong,
            &required,
            PackageLimits::default()
        )
        .err()
        .unwrap()
        .kind,
        ErrorKind::UnsupportedProfile
    );
    let mut wrong = info.clone();
    wrong.capabilities.clear();
    assert_eq!(
        prepare(
            &basis,
            blobs.clone(),
            wrong,
            &required,
            PackageLimits::default()
        )
        .err()
        .unwrap()
        .kind,
        ErrorKind::UnsupportedCapability
    );
    let request = prepare(
        &basis,
        blobs,
        info.clone(),
        &required,
        PackageLimits::default(),
    )
    .unwrap();
    let response = || CompileResponse {
        provider: info.clone(),
        basis_digest: request.basis_digest().into(),
        files: package_files(),
        semantic_evidence: None,
    };
    let checked = validate_response(&request, response()).unwrap();
    assert!(!checked.local_semantic_coverage_established());
    assert!(checked.provider_semantic_evidence.is_none());
    let mut wrong = response();
    wrong.basis_digest = "wrong".into();
    assert_eq!(
        validate_response(&request, wrong).err().unwrap().kind,
        ErrorKind::BasisMismatch
    );
    let mut wrong = response();
    wrong.files.get_mut("ENTRY.md").unwrap().push(b'x');
    assert_eq!(
        validate_response(&request, wrong).err().unwrap().kind,
        ErrorKind::MalformedResponse
    );
    let mut wrong = response();
    wrong.provider.compiler = "other".into();
    assert_eq!(
        validate_response(&request, wrong).err().unwrap().kind,
        ErrorKind::MalformedResponse
    );
}
#[test]
fn provider_unavailable_never_falls_back_and_semantic_claims_remain_separate() {
    struct Offline;
    impl CompilerProvider for Offline {
        fn info(&self) -> Result<ProviderInfo, AdapterError> {
            Err(AdapterError::unavailable("offline"))
        }
        fn compile(&self, _: &CompileRequest) -> Result<CompileResponse, AdapterError> {
            panic!("unavailable provider must not execute")
        }
    }
    let (basis, blobs, mut info) = setup();
    assert_eq!(
        compile_with(
            &Offline,
            &basis,
            blobs.clone(),
            &BTreeSet::new(),
            PackageLimits::default()
        )
        .err()
        .unwrap()
        .kind,
        ErrorKind::Unavailable
    );
    info.capabilities.insert(Capability::OfflineSemanticCheck);
    let required = info.capabilities.clone();
    let request = prepare(
        &basis,
        blobs,
        info.clone(),
        &required,
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        validate_response(
            &request,
            CompileResponse {
                provider: info,
                basis_digest: request.basis_digest().into(),
                files: package_files(),
                semantic_evidence: None
            }
        )
        .err()
        .unwrap()
        .kind,
        ErrorKind::MalformedResponse
    );
}

#[test]
fn malformed_tasks_and_deep_records_refuse_before_provider_call() {
    let (basis, blobs, info) = setup();
    for (field, value) in [
        ("role", "invented-role"),
        ("source_digest", "not-a-digest"),
        (
            "source_digest",
            "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        ),
    ] {
        let mut bad = basis.clone();
        let task = bad.task.as_mut().unwrap();
        if field == "role" {
            task.role = value.into();
        } else {
            task.source_digest = value.into();
        }
        assert_eq!(
            prepare(
                &bad,
                blobs.clone(),
                info.clone(),
                &BTreeSet::new(),
                PackageLimits::default()
            )
            .err()
            .unwrap()
            .kind,
            ErrorKind::InvalidRequest
        );
    }
    let mut bad = basis;
    let mut deep = serde_json::Value::Null;
    for _ in 0..20_000 {
        deep = serde_json::Value::Array(vec![deep]);
    }
    bad.records = vec![BTreeMap::from([("deep".into(), deep)])];
    let result = prepare(
        &bad,
        blobs,
        info,
        &BTreeSet::new(),
        PackageLimits::default(),
    );
    // Caller owns deliberately hostile input; dismantle it without recursive Drop.
    let mut value = bad.records[0].remove("deep").unwrap();
    while let serde_json::Value::Array(mut a) = value {
        value = a.pop().unwrap();
    }
    assert_eq!(result.err().unwrap().kind, ErrorKind::InvalidRequest);
}
