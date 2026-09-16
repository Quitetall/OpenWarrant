// SPDX-License-Identifier: Apache-2.0
use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};
fn invoke(request: &Value) -> (bool, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["sdk", "--request", "-"])
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.success(),
        serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr))),
    )
}
#[test]
fn parse_matches_library_outside_repository_and_refuses_bad_source() {
    use openwarrant_core::document::{Dialect, ParseLimits, parse_document};
    let source = include_str!("../../../conformance/sdk/document/minimal-rc3.md");
    let doc = parse_document(source.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    let (ok, out) = invoke(
        &json!({"schema":"oh.war/sdk-request/v1","operation":"parse","source":source,"dialect":"rc3"}),
    );
    assert!(ok);
    assert_eq!(
        out["result"]["metadata"]["id"],
        doc.metadata()["id"].as_str().unwrap()
    );
    let (ok, out) = invoke(
        &json!({"schema":"oh.war/sdk-request/v1","operation":"parse","source":"bad","dialect":"rc3"}),
    );
    assert!(!ok);
    assert_eq!(out["diagnostics"][0]["code"], "source-invalid");
}
#[test]
fn local_unit_and_condition_syntax_never_claim_external_resolution() {
    let source = include_str!("../../../conformance/sdk/document/minimal-rc3.md");
    let (ok, v) = invoke(
        &json!({"schema":"oh.war/sdk-request/v1","operation":"unit","source":source,"dialect":"rc3","unit":"outcome"}),
    );
    assert!(ok);
    assert!(v["result"]["unit"].as_str().unwrap().contains("signup"));
    let (ok, _) = invoke(
        &json!({"schema":"oh.war/sdk-request/v1","operation":"unit","source":source,"dialect":"rc3","unit":"missing"}),
    );
    assert!(!ok);
    let (ok, v) = invoke(
        &json!({"schema":"oh.war/sdk-request/v1","operation":"condition","condition":{"stage":["implement"]}}),
    );
    assert!(ok);
    assert_eq!(v["result"]["applicability"], "not-evaluated");
    let (ok, _) = invoke(
        &json!({"schema":"oh.war/sdk-request/v1","operation":"condition","condition":{"invented":["x"]}}),
    );
    assert!(!ok);
}
#[test]
fn preservation_and_agent_records_match_public_sdk_without_authentication() {
    let payload = include_str!("../../../conformance/sdk/legacy/current.json");
    let (ok, v) = invoke(
        &json!({"schema":"oh.war/sdk-request/v1","operation":"legacy-import","payload":payload}),
    );
    assert!(ok);
    let expected =
        openwarrant_core::document::legacy::import(payload.as_bytes(), Default::default()).unwrap();
    assert_eq!(
        v["result"]["original_identity"],
        expected.report().original_identity
    );
    assert_eq!(v["result"]["qualification_established"], false);
    let (ok, _) = invoke(
        &json!({"schema":"oh.war/sdk-request/v1","operation":"legacy-import","payload":"{}"}),
    );
    assert!(!ok);
    let payload = include_str!("../../../conformance/sdk/records/unsigned-completion.json");
    let (ok, v) = invoke(
        &json!({"schema":"oh.war/sdk-request/v1","operation":"agent-act","payload":payload}),
    );
    assert!(ok);
    assert_eq!(v["result"]["authenticated"], false);
    assert_eq!(v["result"]["human_acceptance"], false);
}
#[test]
fn unknown_fields_and_wrong_schema_refuse() {
    let (ok, _) =
        invoke(&json!({"schema":"future","operation":"condition","condition":{"stage":["x"]}}));
    assert!(!ok);
    let (ok, _) = invoke(
        &json!({"schema":"oh.war/sdk-request/v1","operation":"condition","condition":{"stage":["x"]},"sign":true}),
    );
    assert!(!ok);
}

#[test]
fn file_backed_operations_and_named_refusals() {
    let manifest: Value =
        serde_json::from_str(include_str!("../../../conformance/sdk/cli/cases.json")).unwrap();
    let mut operations = std::collections::BTreeSet::new();
    let mut refusals = std::collections::BTreeSet::new();
    for case in manifest["cases"].as_array().unwrap() {
        operations.insert(case["request"]["operation"].as_str().unwrap());
        let (ok, out) = invoke(&case["request"]);
        assert_eq!(
            ok,
            case["success"].as_bool().unwrap(),
            "{}: {out}",
            case["id"]
        );
        assert_eq!(out["schema"], "oh.war/report/v1");
        assert_eq!(out["exit_code"], if ok { 0 } else { 1 });
        for (pointer, expected) in case["expect"].as_object().unwrap() {
            assert_eq!(
                out.pointer(pointer),
                Some(expected),
                "{} {pointer}",
                case["id"]
            );
        }
        if !ok {
            refusals.insert(case["request"]["operation"].as_str().unwrap());
            assert_eq!(
                out["diagnostics"][0]["code"], case["error"],
                "{}",
                case["id"]
            );
        }
    }
    assert_eq!(operations, refusals, "Every operation has a named refusal");
    assert_eq!(
        operations.len(),
        33,
        "Every operation has a real successful case"
    );
}
fn raw(input: &[u8], extra: &[&str]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["sdk", "--request", "-"])
        .args(extra)
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Over-limit reader may close its end before all input is sent.
    let _ = child.stdin.take().unwrap().write_all(input);
    child.wait_with_output().unwrap()
}
#[test]
fn malformed_wire_and_resource_limits_refuse_before_work() {
    for request in [
        r#"{"schema":"oh.war/sdk-request/v1","operation":"digest","domain":"basis","payload":{"x":1,"x":2}}"#.to_string(),
        r#"{"schema":"oh.war/sdk-request/v1","operation":"condition","condition":{"stage":["a"],"stage":["b"]}}"#.to_string(),
        r#"{"schema":"oh.war/sdk-request/v1","operation":"schedule","scopes":[["test:work","sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",null]],"dependencies":[],"facts":[]}"#.to_string(),
        format!(r#"{{"schema":"oh.war/sdk-request/v1","operation":"digest","domain":"basis","payload":[{}]}}"#,vec!["0";65_536].join(",")),
        format!(r#"{{"schema":"oh.war/sdk-request/v1","operation":"digest","domain":"basis","payload":{}0{}}}"#,"[".repeat(65),"]".repeat(65)),
        " ".repeat(4*1024*1024+1),
        "{} {}".into(),
    ] {
        let out=raw(request.as_bytes(),&[]);
        assert!(!out.status.success());
        let v:Value=serde_json::from_slice(&out.stdout).unwrap();
        assert!(v["result"].is_null());
    }
}
#[test]
fn file_io_is_explicit_and_never_replaces_existing_output() {
    let root = std::env::temp_dir().join(format!("ow87-io-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("result.json");
    let input =
        br#"{"schema":"oh.war/sdk-request/v1","operation":"raw-digest","bytes_hex":"616263"}"#;
    let out = raw(input, &["--output", path.to_str().unwrap()]);
    assert!(out.status.success());
    assert_eq!(std::fs::read(&path).unwrap(), out.stdout);
    std::fs::write(&path, b"existing bytes").unwrap();
    let out = raw(input, &["--output", path.to_str().unwrap()]);
    assert!(!out.status.success());
    assert_eq!(std::fs::read(&path).unwrap(), b"existing bytes");
    let out = raw(b"bad request", &["--output", path.to_str().unwrap()]);
    assert!(!out.status.success());
    assert_eq!(std::fs::read(&path).unwrap(), b"existing bytes");
    let request = root.join("request.json");
    std::fs::write(&request, input).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["sdk", "--request", request.to_str().unwrap()])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(out.status.success());
    #[cfg(unix)]
    {
        let link = root.join("link.json");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(
            !raw(input, &["--output", link.to_str().unwrap()])
                .status
                .success()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"existing bytes");
    }
    // Cancellation while input is pending cannot publish or replace anything.
    let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["sdk", "--request", "-", "--output", path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"existing bytes");
    let missing = root.join("missing");
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["sdk", "--request", missing.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["diagnostics"][0]["code"],
        "sdk.input"
    );
    assert!(
        !raw(
            input,
            &["--output", missing.join("result").to_str().unwrap()]
        )
        .status
        .success()
    );
    assert!(!missing.exists());
    assert!(!std::fs::read_dir(&root).unwrap().any(|p| {
        p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".ow-sdk-")
    }));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_cli_arguments_return_one_envelope_without_changing_legacy_help() {
    for args in [
        vec!["sdk"],
        vec!["--json", "sdk", "--request"],
        vec!["sdk", "--wrong"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_war"))
            .args(args)
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert_eq!(
            serde_json::from_slice::<Value>(&out.stdout).unwrap()["diagnostics"][0]["code"],
            "sdk.arguments"
        );
    }
    for args in [vec!["sdk", "--help"], vec!["document", "review", "--help"]] {
        let out = Command::new(env!("CARGO_BIN_EXE_war"))
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("Usage:"));
    }
}

#[test]
fn author_edit_digest_and_readiness_results_equal_public_sdk() {
    use openwarrant_core::document::{self as d, records as r};
    let cases: Value =
        serde_json::from_str(include_str!("../../../conformance/sdk/cli/cases.json")).unwrap();
    let req = |id: &str| {
        cases["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap()["request"]
            .clone()
    };
    let author = req("author");
    let fields = d::DocumentFields {
        metadata: author["metadata"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p[0].as_str().unwrap().into(),
                    serde_json::from_value(p[1].clone()).unwrap(),
                )
            })
            .collect(),
        units: vec![d::AuthoredUnit {
            id: "context".into(),
            kind: d::UnitKind::Binding,
            text: author["units"][0]["text"].as_str().unwrap().into(),
        }],
    };
    let expected = d::author_document(
        &fields,
        d::AuthorOptions::default(),
        d::ParseLimits::default(),
    )
    .unwrap();
    let (ok, out) = invoke(&author);
    assert!(ok);
    assert_eq!(
        out["result"]["source"].as_str().unwrap().as_bytes(),
        expected
    );
    let edit = req("edit");
    let source = edit["source"].as_str().unwrap();
    let doc = d::parse_document(source.as_bytes(), d::Dialect::Rc3, Default::default()).unwrap();
    let expected = d::edit_document(
        &doc,
        &[d::DocumentEdit::Metadata {
            key: "revision".into(),
            value: Some(d::MetadataValue::Integer(2)),
        }],
        Default::default(),
    )
    .unwrap();
    let (ok, out) = invoke(&edit);
    assert!(ok);
    assert_eq!(
        out["result"]["source"].as_str().unwrap().as_bytes(),
        expected
    );
    let digest = req("digest");
    let expected = d::packet::structured_digest(
        d::packet::DigestDomain::Basis,
        &digest["payload"],
        4 * 1024 * 1024,
    )
    .unwrap();
    assert_eq!(invoke(&digest).1["result"]["digest"], expected);
    let request = req("readiness-unknown");
    let payload = request["records"][0].as_str().unwrap();
    let subject: r::Subject = serde_json::from_value(request["subject"].clone()).unwrap();
    let records =
        r::check_records(&[payload.as_bytes()], &subject, &[], Default::default()).unwrap();
    let conditions =
        serde_json::from_value::<Vec<r::Condition>>(request["conditions"].clone()).unwrap();
    let expected = r::evaluate_readiness(
        &records,
        &conditions,
        "execute",
        "implement",
        &[],
        Default::default(),
    )
    .unwrap();
    let (ok, out) = invoke(&request);
    assert!(ok);
    assert_eq!(
        out["result"]["evaluation"],
        serde_json::to_value(expected).unwrap()
    );
}

#[test]
fn nested_unknown_requirement_fields_refuse() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../conformance/sdk/cli/cases.json")).unwrap();
    let mut request = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "readiness-unknown")
        .unwrap()["request"]
        .clone();
    request["conditions"][0]["requirement"]["passing-check"]["invented"] = json!("must-refuse");
    let (ok, out) = invoke(&request);
    assert!(!ok);
    assert_eq!(out["diagnostics"][0]["code"], "sdk.request");
}

#[test]
fn explicit_null_sdk_optionals_remain_valid_but_unknown_null_refuses() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../conformance/sdk/cli/cases.json")).unwrap();
    let request = |id: &str| {
        cases["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap()["request"]
            .clone()
    };
    let mut describe = request("describe");
    describe["holder"]["commit"] = Value::Null;
    assert!(invoke(&describe).0);
    describe["holder"]["invented"] = Value::Null;
    assert!(!invoke(&describe).0);
    let mut sources = request("sources");
    sources["descriptors"][0]["document"] = Value::Null;
    assert!(invoke(&sources).0);
    sources["descriptors"][0]["invented"] = Value::Null;
    assert!(!invoke(&sources).0);
    let mut readiness = request("readiness-unknown");
    readiness["conditions"][0]["requirement"]["passing-check"]["invented"] = Value::Null;
    assert!(!invoke(&readiness).0);
}
