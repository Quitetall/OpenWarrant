// SPDX-License-Identifier: Apache-2.0
//! Fixed skill artifacts exercise public parsers, not model quality or human authority.
#[test]
fn stage_artifact_preserves_dependency_and_refuses_cycle() {
    let proposal: serde_json::Value =
        serde_json::from_str(include_str!("../../../conformance/sdk/skills/stages.json")).unwrap();
    let body = proposal["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["role"] == "milestones")
        .unwrap()["body"]
        .as_str()
        .unwrap();
    let graph = openwarrant_core::milestones::parse(body).unwrap();
    assert_eq!(graph.milestones[1].depends_on, vec!["M1"]);
    let cycle = body.replace(
        "title: \"Prepare acceptance cases\"",
        "title: \"Prepare acceptance cases\"\n    depends_on: [\"M2\"]",
    );
    let error = openwarrant_core::milestones::parse(&cycle).unwrap_err();
    assert!(error.to_string().contains("cycle"), "{error}");
}
