// SPDX-License-Identifier: Apache-2.0
//! Refusal observations, not a successful separate-account deployment.
use openwarrant_cli::runtime_capture::{
    collector_loading::LoadedEnrollment, collector_signature::OpenSshSignatureCheck,
};
use openwarrant_core::runtime_collector::{Enrollment, Fault, Provider, SCHEMA, Signed};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    time::Duration,
};
fn wire() -> Vec<u8> {
    Signed {
        enrollment: Enrollment {
            schema: SCHEMA.into(),
            repository: "fixture".into(),
            authority_digest: format!("sha256:{}", "a".repeat(64)),
            provider: Provider {
                kind: "katana".into(),
                identity: "fixture".into(),
                version: "v1".into(),
            },
            verifier_digest: format!("sha256:{}", "b".repeat(64)),
            collector: "collector".into(),
            warrants: BTreeSet::from(["01a0f502-4941-70a1-a446-e1eb77dff191".into()]),
        },
        signatures: BTreeMap::from([("owner".into(), "not-an-authenticated-signature".into())]),
    }
    .encode()
    .unwrap()
}
fn root(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("ow-collector-load-{label}-{}", std::process::id()));
    fs::create_dir(&p).unwrap();
    p
}
#[test]
fn an_executor_owned_store_cannot_become_trusted_by_claiming_another_uid() {
    let p = root("owned");
    let store = p.join("store");
    fs::create_dir(&store).unwrap();
    // Inert bytes cannot self-declare a different executor and become a root.
    fs::write(store.join("state.json"), br#"{"agent_uid":4294967294}"#).unwrap();
    let enrollment = p.join("enrollment.json");
    fs::write(&enrollment, wire()).unwrap();
    let verifier = OpenSshSignatureCheck::new(p.clone(), Duration::from_secs(5)).unwrap();
    assert!(matches!(
        LoadedEnrollment::load(&store, "fixture", &enrollment, &verifier),
        Err(Fault::Rejected(
            "authority store is writable by the executor"
        ))
    ));
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn unavailable_sources_stay_unknown_and_bad_wire_is_rejected() {
    let p = root("sources");
    let verifier = OpenSshSignatureCheck::new(p.clone(), Duration::from_secs(5)).unwrap();
    let enrollment = p.join("enrollment.json");
    assert!(matches!(
        LoadedEnrollment::load(&p.join("missing-store"), "fixture", &enrollment, &verifier),
        Err(Fault::Unavailable(_))
    ));
    fs::write(&enrollment, b"{}").unwrap();
    assert!(matches!(
        LoadedEnrollment::load(&p.join("missing-store"), "fixture", &enrollment, &verifier),
        Err(Fault::Rejected(_))
    ));
    fs::write(&enrollment, vec![b' '; 65_537]).unwrap();
    assert!(matches!(
        LoadedEnrollment::load(&p.join("missing-store"), "fixture", &enrollment, &verifier),
        Err(Fault::Rejected("enrollment wire budget"))
    ));
    fs::remove_dir_all(p).unwrap();
}
#[cfg(unix)]
#[test]
fn symlinks_and_non_regular_enrollment_sources_are_refused() {
    let p = root("links");
    let verifier = OpenSshSignatureCheck::new(p.clone(), Duration::from_secs(5)).unwrap();
    let real = p.join("real.json");
    fs::write(&real, wire()).unwrap();
    let link = p.join("link.json");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert!(matches!(
        LoadedEnrollment::load(&p, "fixture", &link, &verifier),
        Err(Fault::Rejected(_))
    ));
    assert!(matches!(
        LoadedEnrollment::load(&p, "fixture", &p, &verifier),
        Err(Fault::Rejected(_))
    ));
    let fifo = p.join("fifo");
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    assert!(matches!(
        LoadedEnrollment::load(&p, "fixture", &fifo, &verifier),
        Err(Fault::Rejected("regular enrollment file required"))
    ));
    fs::remove_dir_all(p).unwrap();
}
