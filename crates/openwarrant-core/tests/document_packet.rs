// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::packet::{PacketLimits, decode_packet};
const PACKET: &[u8] = include_bytes!(
    "../../../docs/sas/drafts/1.0.0-rc.2/examples/expected/signup-packet/packet.json"
);
#[test]
fn bounded_packet_decode_accepts_example_and_refuses_schema_and_duplicate_fields() {
    let packet = decode_packet(PACKET, PacketLimits::default()).unwrap();
    assert_eq!(packet.coverage, "declared-inputs-and-dependencies");
    let mut value: serde_json::Value = serde_json::from_slice(PACKET).unwrap();
    value["schema"] = "invented".into();
    assert_eq!(
        decode_packet(&serde_jcs::to_vec(&value).unwrap(), PacketLimits::default())
            .unwrap_err()
            .code,
        "schema-unsupported"
    );
    assert_eq!(
        decode_packet(
            PACKET,
            PacketLimits {
                json_bytes: 1,
                ..PacketLimits::default()
            }
        )
        .unwrap_err()
        .code,
        "resource-limit"
    );
    let duplicate = String::from_utf8(PACKET.to_vec()).unwrap().replacen(
        "{",
        "{\"compiler\":\"duplicate\",",
        1,
    );
    assert!(decode_packet(duplicate.as_bytes(), PacketLimits::default()).is_err());
}

#[test]
fn packet_integrity_checks_exact_source_ranges_and_entry_accounting() {
    use openwarrant_core::document::{packet::check_packet_integrity, source::SourceLimits};
    use std::{collections::BTreeMap, fs, path::Path};
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/sas/drafts/1.0.0-rc.2/examples/expected/signup-packet");
    let mut blobs = BTreeMap::new();
    for file in fs::read_dir(root.join("blobs")).unwrap() {
        let file = file.unwrap();
        blobs.insert(
            format!(
                "sha256:{}",
                file.path().file_stem().unwrap().to_str().unwrap()
            ),
            fs::read(file.path()).unwrap(),
        );
    }
    let entry = fs::read(root.join("ENTRY.md")).unwrap();
    let mut packet = decode_packet(PACKET, PacketLimits::default()).unwrap();
    check_packet_integrity(
        &packet,
        &blobs,
        &entry,
        PacketLimits::default(),
        SourceLimits::default(),
    )
    .unwrap();
    packet.binding_context[0].reference.end -= 1;
    assert_eq!(
        check_packet_integrity(
            &packet,
            &blobs,
            &entry,
            PacketLimits::default(),
            SourceLimits::default()
        )
        .unwrap_err()
        .code,
        "range-mismatch"
    );
}

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
#[test]
fn package_integrity_refuses_tampering_extra_files_and_noncanonical_manifest() {
    use openwarrant_core::document::packet::{PackageLimits, check_package};
    let files = package_files();
    let checked = check_package(files.clone(), PackageLimits::default()).unwrap();
    assert!(!checked.semantic_coverage_established());
    let mut tampered = files.clone();
    tampered.get_mut("ENTRY.md").unwrap().push(b'x');
    assert_eq!(
        check_package(tampered, PackageLimits::default())
            .unwrap_err()
            .code,
        "digest-mismatch"
    );
    let mut extra = files.clone();
    extra.insert("../escape".into(), vec![]);
    assert_eq!(
        check_package(extra, PackageLimits::default())
            .unwrap_err()
            .code,
        "package-invalid"
    );
    let mut manifest = files.clone();
    manifest.get_mut("manifest.json").unwrap().push(b'\n');
    assert_eq!(
        check_package(manifest, PackageLimits::default())
            .unwrap_err()
            .code,
        "package-invalid"
    );
    assert_eq!(
        check_package(
            files,
            PackageLimits {
                total_bytes: 1,
                ..PackageLimits::default()
            }
        )
        .unwrap_err()
        .code,
        "resource-limit"
    );
}

#[test]
fn self_consistent_incomplete_package_is_integrity_only_and_missing_routing_blob_refuses() {
    use openwarrant_core::document::packet::{
        DigestDomain, PackageLimits, check_package, structured_digest,
    };
    use sha2::{Digest, Sha256};
    fn reseal(files: &mut std::collections::BTreeMap<String, Vec<u8>>) {
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&files["manifest.json"]).unwrap();
        manifest["files"] = serde_json::Value::Array(
            files
                .iter()
                .filter(|(p, _)| p.as_str() != "manifest.json")
                .map(|(p, b)| {
                    let hash: String = Sha256::digest(b)
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect();
                    serde_json::json!({"path":p,"bytes":b.len(),"sha256":format!("sha256:{hash}")})
                })
                .collect(),
        );
        manifest.as_object_mut().unwrap().remove("root_digest");
        let root = structured_digest(DigestDomain::Package, &manifest, 1024 * 1024).unwrap();
        manifest["root_digest"] = root.into();
        files.insert(
            "manifest.json".into(),
            serde_jcs::to_vec(&manifest).unwrap(),
        );
    }
    let mut incomplete = package_files();
    let mut packet: serde_json::Value = serde_json::from_slice(&incomplete["packet.json"]).unwrap();
    packet["binding_context"] = serde_json::json!([]);
    for name in [
        "outcome",
        "scope",
        "context",
        "constraints",
        "outputs",
        "stop",
    ] {
        packet["brief"][name] = serde_json::json!([]);
    }
    incomplete.insert("packet.json".into(), serde_jcs::to_vec(&packet).unwrap());
    reseal(&mut incomplete);
    assert!(
        !check_package(incomplete, PackageLimits::default())
            .unwrap()
            .semantic_coverage_established()
    );
    let mut missing = package_files();
    let path = missing
        .keys()
        .find(|p| p.starts_with("blobs/"))
        .unwrap()
        .clone();
    missing.remove(&path);
    reseal(&mut missing);
    assert_eq!(
        check_package(missing, PackageLimits::default())
            .unwrap_err()
            .code,
        "source-missing"
    );
}

#[test]
fn structured_digest_bounds_actual_canonical_bytes_and_depth() {
    use openwarrant_core::document::packet::{DigestDomain, structured_digest};
    assert_eq!(
        structured_digest(DigestDomain::Basis, &serde_json::json!({"n":1e20}), 65)
            .unwrap_err()
            .code,
        "resource-limit"
    );
    let mut nested = serde_json::Value::Null;
    for _ in 0..129 {
        nested = serde_json::Value::Array(vec![nested]);
    }
    assert_eq!(
        structured_digest(DigestDomain::Basis, &nested, 1000000)
            .unwrap_err()
            .code,
        "resource-limit"
    );
}

#[test]
fn typed_packet_integrity_enforces_json_budget() {
    use openwarrant_core::document::{
        packet::{PackageLimits, check_package, check_packet_integrity},
        source::SourceLimits,
    };
    let checked = check_package(package_files(), PackageLimits::default()).unwrap();
    assert_eq!(
        check_packet_integrity(
            &checked.packet,
            &checked.blobs,
            &checked.entry,
            PacketLimits {
                json_bytes: 1,
                ..PacketLimits::default()
            },
            SourceLimits::default()
        )
        .unwrap_err()
        .code,
        "resource-limit"
    );
}

#[test]
fn incomplete_basis_refuses_even_with_valid_hashes() {
    use openwarrant_core::document::packet::{
        DigestDomain, PackageLimits, check_package, structured_digest,
    };
    use sha2::{Digest, Sha256};
    let mut files = package_files();
    let mut m: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    let mut p: serde_json::Value = serde_json::from_slice(&files["packet.json"]).unwrap();
    for field in ["selection_inputs", "policy_facts", "records", "options"] {
        m["basis"].as_object_mut().unwrap().remove(field);
    }
    p["basis_digest"] = structured_digest(DigestDomain::Basis, &m["basis"], 1_000_000)
        .unwrap()
        .into();
    files.insert("packet.json".into(), serde_jcs::to_vec(&p).unwrap());
    m["files"]=serde_json::Value::Array(files.iter().filter(|(p,_)|p.as_str()!="manifest.json").map(|(p,b)|serde_json::json!({"path":p,"bytes":b.len(),"sha256":format!("sha256:{}",Sha256::digest(b).iter().map(|b|format!("{b:02x}")).collect::<String>())})).collect());
    m.as_object_mut().unwrap().remove("root_digest");
    m["root_digest"] = structured_digest(DigestDomain::Package, &m, 1_000_000)
        .unwrap()
        .into();
    files.insert("manifest.json".into(), serde_jcs::to_vec(&m).unwrap());
    let result = check_package(files, PackageLimits::default());
    assert_eq!(result.unwrap_err().code, "package-invalid");
}
