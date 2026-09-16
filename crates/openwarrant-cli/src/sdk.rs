// SPDX-License-Identifier: Apache-2.0
//! Offline SDK shell. Embedded bytes and assumed facts are never acquired or authenticated.
use camino::Utf8Path;
use openwarrant_core::document::{self as d, legacy as l, packet as p, records as r, source as s};
use serde::{Deserialize, Serialize};
pub(crate) mod wire;
use openwarrant_core::document::{adapter as a, schedule as q};
use r::workflow as w;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
};
const INPUT_LIMIT: usize = 4 * 1024 * 1024;
const OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
#[derive(Deserialize, Serialize)]
struct Request {
    schema: String,
    #[serde(flatten)]
    operation: Operation,
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum Operation {
    RawDigest {
        bytes_hex: String,
    },
    SourceCodec {
        payload: String,
    },
    ReferenceCodec {
        payload: String,
    },
    PacketEncode {
        payload: String,
    },
    Schedule {
        scopes: Vec<q::WorkScope>,
        dependencies: Vec<q::Dependency>,
        facts: Vec<q::ResultFact>,
    },
    Difficulty {
        estimate: q::DifficultyEstimate,
    },
    Handoff {
        stop: String,
        overview: String,
        handoff: String,
        assumptions: w::TrackerTrust,
        word: String,
        style: w::ResponseStyle,
    },
    Replay {
        previous: String,
        incoming: String,
    },
    Resume {
        change: String,
        assumptions: w::ResumeFacts,
    },
    Availability {
        record: String,
        assumptions: w::AvailabilityFacts,
    },
    BatchAcceptance {
        manifest: String,
        member: w::ReviewMember,
        assumptions: w::HumanReviewTrust,
    },
    AdapterPrepare {
        basis: Box<p::CompileBasis>,
        files: Vec<Blob>,
        provider: a::ProviderInfo,
        required: std::collections::BTreeSet<a::Capability>,
    },
    AdapterCheck {
        basis: Box<p::CompileBasis>,
        files: Vec<Blob>,
        provider: a::ProviderInfo,
        required: std::collections::BTreeSet<a::Capability>,
        response: Box<AdapterResponse>,
    },

    Parse {
        source: String,
        dialect: String,
    },
    Validate {
        source: String,
        dialect: String,
    },
    Unit {
        source: String,
        dialect: String,
        unit: String,
    },
    Author {
        metadata: Vec<(String, Value)>,
        units: Vec<Unit>,
        #[serde(default)]
        crlf: bool,
        #[serde(default)]
        wrap: bool,
    },
    Edit {
        source: String,
        dialect: String,
        edits: Vec<Edit>,
    },
    Digest {
        domain: String,
        payload: Value,
    },
    Condition {
        condition: Value,
    },
    SourceDescribe {
        path: String,
        bytes_hex: String,
        holder: s::Holder,
        metadata: s::SourceMetadata,
        dialect: Option<String>,
    },
    SourcesCheck {
        descriptors: Vec<s::SourceDescriptor>,
        files: Vec<Blob>,
        reference: Option<s::BoundReference>,
    },
    PacketDecode {
        payload: String,
    },
    PacketCheck {
        payload: String,
        files: Vec<Blob>,
        entry: String,
    },
    PackageCheck {
        files: Vec<Blob>,
    },
    AgentAct {
        payload: String,
    },
    Records {
        records: Vec<String>,
        subject: r::Subject,
        #[serde(default)]
        assumptions: Vec<r::TrustedRecord>,
    },
    Readiness {
        records: Vec<String>,
        subject: r::Subject,
        #[serde(default)]
        assumptions: Vec<r::TrustedRecord>,
        conditions: Vec<r::Condition>,
        action: String,
        stage: String,
        #[serde(default)]
        policies: Vec<r::PolicyAuthority>,
    },
    Assurance {
        records: Vec<String>,
        subject: r::Subject,
        #[serde(default)]
        assumptions: Vec<r::TrustedRecord>,
        contract: Option<String>,
        verification_ref: String,
        acceptance_ref: String,
        conditions: Vec<r::Condition>,
        #[serde(default)]
        policies: Vec<r::PolicyAuthority>,
    },
    WorkflowDecode {
        payload: String,
    },
    LegacyImport {
        payload: String,
    },
    LegacyCapture {
        dialect: String,
        adapter: String,
        entry: String,
        files: Vec<Blob>,
    },
    Successor {
        predecessor: String,
        successor: String,
        paths: Vec<String>,
    },
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Unit {
    id: String,
    kind: String,
    text: String,
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum Edit {
    Metadata { key: String, value: Option<Value> },
    Unit { id: String, text: String },
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Blob {
    path: String,
    hex: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AdapterResponse {
    provider: a::ProviderInfo,
    basis_digest: String,
    files: Vec<Blob>,
    semantic_evidence: Option<a::SemanticEvidence>,
}
#[derive(Debug)]
struct Error {
    code: String,
    message: String,
    span: Option<std::ops::Range<usize>>,
}
impl Error {
    fn new(code: &str, message: impl ToString) -> Self {
        Self {
            code: code.into(),
            message: message.to_string(),
            span: None,
        }
    }
}
impl From<d::Diagnostic> for Error {
    fn from(e: d::Diagnostic) -> Self {
        Self {
            code: e.code.into(),
            message: e.message,
            span: Some(e.span),
        }
    }
}
impl From<r::RecordError> for Error {
    fn from(e: r::RecordError) -> Self {
        Self::new(e.code, e.message)
    }
}
impl From<l::Error> for Error {
    fn from(e: l::Error) -> Self {
        Self::new(e.code, e.message)
    }
}
fn dialect(s: &str) -> Result<d::Dialect, Error> {
    match s {
        "rc2" => Ok(d::Dialect::Rc2),
        "rc3" => Ok(d::Dialect::Rc3),
        _ => Err(Error::new("sdk.dialect", "Use rc2 or rc3")),
    }
}
fn metadata(v: Value) -> Result<d::MetadataValue, Error> {
    serde_json::from_value(v).map_err(|e| Error::new("sdk.metadata", e))
}
fn bytes(hex: &str) -> Result<Vec<u8>, Error> {
    if !hex.len().is_multiple_of(2) || hex.len() > INPUT_LIMIT {
        return Err(Error::new("sdk.hex", "Invalid hex length"));
    }
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |b: u8| match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                _ => None,
            };
            Ok(
                digit(pair[0]).ok_or_else(|| Error::new("sdk.hex", "Use lowercase hex"))? * 16
                    + digit(pair[1]).ok_or_else(|| Error::new("sdk.hex", "Use lowercase hex"))?,
            )
        })
        .collect()
}
fn blobs(input: Vec<Blob>) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    let mut out = BTreeMap::new();
    for b in input {
        let value = bytes(&b.hex)?;
        if out.insert(b.path, value).is_some() {
            return Err(Error::new("sdk.duplicate", "Duplicate blob path"));
        }
    }
    Ok(out)
}
fn source_result(bytes: Vec<u8>) -> Value {
    json!({"source":String::from_utf8(bytes).expect("SDK authored UTF-8"),"saved":false,"qualified":false})
}
fn records(
    input: &[String],
    subject: &r::Subject,
    assumptions: &[r::TrustedRecord],
) -> Result<r::CheckedRecords, Error> {
    Ok(r::check_records(
        &input.iter().map(|s| s.as_bytes()).collect::<Vec<_>>(),
        subject,
        assumptions,
        r::RecordLimits::default(),
    )?)
}
fn legacy_result(value: l::Preserved) -> Result<Value, Error> {
    Ok(
        json!({"payload":String::from_utf8(l::export(&value,l::Limits::default())?).expect("JSON UTF-8"),"original_schema":value.report().original_schema,"original_identity":value.report().original_identity,"original_state":value.report().original_state,"historical_fields":value.report().historical_fields,"inventory_preserved":true,"historical_closure_established":false,"qualification_established":false,"unsupported":value.report().unsupported}),
    )
}
fn inspect_document(source: &str, kind: &str, validate: bool) -> Result<Value, Error> {
    let doc = d::parse_document(source.as_bytes(), dialect(kind)?, d::ParseLimits::default())?;
    let report = d::validate_document(&doc, &d::ValidationOptions::default());
    if validate && report.validity == d::Validity::Invalid {
        return Err(report
            .diagnostics
            .first()
            .cloned()
            .map(Error::from)
            .unwrap_or_else(|| Error::new("source-invalid", "Invalid document")));
    }
    Ok(
        json!({"metadata":doc.metadata(),"units":doc.units().iter().map(|u|json!({"id":u.id,"kind":format!("{:?}",u.kind),"start":u.span.start,"end":u.span.end})).collect::<Vec<_>>(),"validity":format!("{:?}",report.validity),"semantic_support":format!("{:?}",report.semantic_support),"diagnostics":report.diagnostics.iter().map(|e|json!({"code":e.code,"message":e.message,"span":e.span})).collect::<Vec<_>>(),"context_resolution":"not-evaluated","readiness":"not-evaluated","qualification_established":false}),
    )
}
fn workflow(payload: &str) -> Result<w::CheckedWorkflowRecord, Error> {
    Ok(w::decode_workflow_record(
        payload.as_bytes(),
        r::RecordLimits::default(),
    )?)
}
fn adapter_error(e: a::AdapterError) -> Error {
    Error::new("sdk.adapter", format!("{:?}: {}", e.kind, e.message))
}
fn execute(op: Operation) -> Result<Value, Error> {
    let limits = d::ParseLimits::default();
    match op {
        Operation::RawDigest { bytes_hex } => {
            Ok(json!({"digest":r::raw_digest(&bytes(&bytes_hex)?)}))
        }
        Operation::SourceCodec { payload } => {
            let value =
                s::decode_source_descriptor(payload.as_bytes(), s::SourceLimits::default())?;
            Ok(
                json!({"descriptor":value,"canonical":String::from_utf8(s::encode_source_descriptor(&value,s::SourceLimits::default())?).expect("JSON UTF-8")}),
            )
        }
        Operation::ReferenceCodec { payload } => {
            let value = s::decode_bound_reference(payload.as_bytes(), s::SourceLimits::default())?;
            Ok(
                json!({"reference":value,"canonical":String::from_utf8(s::encode_bound_reference(&value,s::SourceLimits::default())?).expect("JSON UTF-8"),"resolved":false}),
            )
        }
        Operation::PacketEncode { payload } => {
            let packet = p::decode_packet(payload.as_bytes(), p::PacketLimits::default())?;
            Ok(
                json!({"payload":String::from_utf8(p::encode_packet(&packet,p::PacketLimits::default())?).expect("JSON UTF-8"),"semantic_coverage_established":false}),
            )
        }
        Operation::Schedule {
            scopes,
            dependencies,
            facts,
        } => Ok(
            json!({"evaluation":q::evaluate_schedule(&scopes,&dependencies,&facts,q::ScheduleLimits::default()).map_err(|e|Error::new("sdk.schedule",format!("{e:?}")))?,"assumptions_authenticated_by_cli":false,"dispatch_permitted":false}),
        ),
        Operation::Difficulty { estimate } => {
            q::check_estimate(&estimate, INPUT_LIMIT)
                .map_err(|e| Error::new("sdk.difficulty", format!("{e:?}")))?;
            Ok(json!({"estimate":estimate,"advisory":true}))
        }
        Operation::Handoff {
            stop,
            overview,
            handoff,
            assumptions,
            word,
            style,
        } => Ok(
            json!({"evaluation":w::render_handoff(&workflow(&stop)?,&workflow(&overview)?,&workflow(&handoff)?,&assumptions,&word,style,OUTPUT_LIMIT)?,"assumptions_authenticated_by_cli":false}),
        ),
        Operation::Replay { previous, incoming } => {
            Ok(json!({"same_event":w::check_replay(&workflow(&previous)?,&workflow(&incoming)?)?}))
        }
        Operation::Resume {
            change,
            assumptions,
        } => Ok(
            json!({"evaluation":w::evaluate_resume(&workflow(&change)?,&assumptions)?,"assumptions_authenticated_by_cli":false,"processes_changed":false}),
        ),
        Operation::Availability {
            record,
            assumptions,
        } => Ok(
            json!({"evaluation":w::evaluate_availability(&workflow(&record)?,&assumptions)?,"assumptions_authenticated_by_cli":false,"storage_changed":false}),
        ),
        Operation::BatchAcceptance {
            manifest,
            member,
            assumptions,
        } => Ok(
            json!({"evaluation":w::batch_acceptance(&workflow(&manifest)?,&member,&assumptions)?,"assumptions_authenticated_by_cli":false,"qualification_established":false}),
        ),
        Operation::AdapterPrepare {
            basis,
            files,
            provider,
            required,
        } => {
            let req = a::prepare(
                &basis,
                blobs(files)?,
                provider,
                &required,
                p::PackageLimits::default(),
            )
            .map_err(adapter_error)?;
            Ok(
                json!({"basis":req.value(),"basis_digest":req.basis_digest(),"provider":req.provider(),"provider_called":false}),
            )
        }
        Operation::AdapterCheck {
            basis,
            files,
            provider,
            required,
            response,
        } => {
            let req = a::prepare(
                &basis,
                blobs(files)?,
                provider,
                &required,
                p::PackageLimits::default(),
            )
            .map_err(adapter_error)?;
            let response = a::CompileResponse {
                provider: response.provider,
                basis_digest: response.basis_digest,
                files: blobs(response.files)?,
                semantic_evidence: response.semantic_evidence,
            };
            let result = a::validate_response(&req, response).map_err(adapter_error)?;
            Ok(
                json!({"packet":result.package.packet,"provider":result.provider,"provider_semantic_claim":result.provider_semantic_evidence,"semantic_coverage_established":false,"provider_called":false}),
            )
        }

        Operation::Parse {
            source,
            dialect: kind,
        } => inspect_document(&source, &kind, false),
        Operation::Validate {
            source,
            dialect: kind,
        } => inspect_document(&source, &kind, true),
        Operation::Unit {
            source,
            dialect: kind,
            unit,
        } => {
            let doc = d::parse_document(source.as_bytes(), dialect(&kind)?, limits)?;
            Ok(
                json!({"unit":doc.unit(&unit).ok_or_else(||Error::new("target-missing","Local unit absent"))?,"external_resolution":"not-performed"}),
            )
        }
        Operation::Author {
            metadata: fields,
            units,
            crlf,
            wrap,
        } => {
            let fields = d::DocumentFields {
                metadata: fields
                    .into_iter()
                    .map(|(k, v)| Ok((k, metadata(v)?)))
                    .collect::<Result<_, Error>>()?,
                units: units
                    .into_iter()
                    .map(|u| {
                        Ok(d::AuthoredUnit {
                            id: u.id,
                            text: u.text,
                            kind: match u.kind.as_str() {
                                "binding" => d::UnitKind::Binding,
                                "background" => d::UnitKind::Background,
                                _ => return Err(Error::new("sdk.kind", "Unknown unit kind")),
                            },
                        })
                    })
                    .collect::<Result<_, Error>>()?,
            };
            Ok(source_result(d::author_document(
                &fields,
                d::AuthorOptions {
                    line_ending: if crlf {
                        d::LineEnding::CrLf
                    } else {
                        d::LineEnding::Lf
                    },
                    wrap_metadata: wrap,
                },
                limits,
            )?))
        }
        Operation::Edit {
            source,
            dialect: kind,
            edits,
        } => {
            let doc = d::parse_document(source.as_bytes(), dialect(&kind)?, limits)?;
            let edits = edits
                .into_iter()
                .map(|e| {
                    Ok(match e {
                        Edit::Metadata { key, value } => d::DocumentEdit::Metadata {
                            key,
                            value: value.map(metadata).transpose()?,
                        },
                        Edit::Unit { id, text } => d::DocumentEdit::Unit { id, text },
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            Ok(source_result(d::edit_document(&doc, &edits, limits)?))
        }
        Operation::Digest { domain, payload } => {
            let domain = match domain.as_str() {
                "basis" => p::DigestDomain::Basis,
                "contract" => p::DigestDomain::Contract,
                "package" => p::DigestDomain::Package,
                _ => return Err(Error::new("sdk.domain", "Unsupported digest domain")),
            };
            Ok(json!({"digest":p::structured_digest(domain,&payload,INPUT_LIMIT)?}))
        }
        Operation::Condition { condition } => {
            let m = metadata(condition)?;
            d::condition::validate_condition(&m, d::condition::ConditionLimits::default())?;
            Ok(json!({"syntax":"valid","applicability":"not-evaluated"}))
        }
        Operation::SourceDescribe {
            path,
            bytes_hex,
            holder,
            metadata,
            dialect: kind,
        } => {
            let b = bytes(&bytes_hex)?;
            Ok(json!(s::describe_source(
                &path,
                holder,
                metadata,
                &b,
                kind.as_deref().map(dialect).transpose()?,
                s::SourceLimits::default()
            )?))
        }
        Operation::SourcesCheck {
            descriptors,
            files,
            reference,
        } => {
            let files = blobs(files)?;
            let checked = s::check_sources(&descriptors, &files, s::SourceLimits::default())?;
            let selected = reference
                .as_ref()
                .map(|r| checked.check_reference(r))
                .transpose()?;
            Ok(
                json!({"source_count":checked.len(),"selected_hex":selected.map(|b|b.iter().map(|x|format!("{x:02x}")).collect::<String>()),"semantic_coverage_established":false}),
            )
        }
        Operation::PacketDecode { payload } => Ok(
            json!({"packet":p::decode_packet(payload.as_bytes(),p::PacketLimits::default())?,"integrity_checked":false,"semantic_coverage_established":false}),
        ),
        Operation::PacketCheck {
            payload,
            files,
            entry,
        } => {
            let packet = p::decode_packet(payload.as_bytes(), p::PacketLimits::default())?;
            p::check_packet_integrity(
                &packet,
                &blobs(files)?,
                entry.as_bytes(),
                p::PacketLimits::default(),
                s::SourceLimits::default(),
            )?;
            Ok(json!({"integrity_checked":true,"semantic_coverage_established":false}))
        }
        Operation::PackageCheck { files } => {
            let package = p::check_package(blobs(files)?, p::PackageLimits::default())?;
            Ok(
                json!({"packet":package.packet,"manifest":package.manifest,"integrity_checked":true,"semantic_coverage_established":false}),
            )
        }
        Operation::AgentAct { payload } => Ok(json!(r::decode_agent_act(
            payload.as_bytes(),
            r::RecordLimits::default()
        )?)),
        Operation::Records {
            records: input,
            subject,
            assumptions,
        } => {
            let checked = records(&input, &subject, &assumptions)?;
            Ok(
                json!({"record_count":input.len(),"missing_refs":checked.missing_refs(),"assumptions_authenticated_by_cli":false,"qualification_established":false}),
            )
        }
        Operation::Readiness {
            records: input,
            subject,
            assumptions,
            conditions,
            action,
            stage,
            policies,
        } => {
            let checked = records(&input, &subject, &assumptions)?;
            Ok(
                json!({"evaluation":r::evaluate_readiness(&checked,&conditions,&action,&stage,&policies,r::RecordLimits::default())?,"assumptions_authenticated_by_cli":false}),
            )
        }
        Operation::Assurance {
            records: input,
            subject,
            assumptions,
            contract,
            verification_ref,
            acceptance_ref,
            conditions,
            policies,
        } => {
            let checked = records(&input, &subject, &assumptions)?;
            Ok(
                json!({"evaluation":r::evaluate_assurance(&checked,contract.as_deref().map(str::as_bytes),&verification_ref,&acceptance_ref,&conditions,&policies,r::RecordLimits::default())?,"assumptions_authenticated_by_cli":false,"qualification_established":false}),
            )
        }
        Operation::WorkflowDecode { payload } => {
            let decoded = r::workflow::decode_workflow_record(
                payload.as_bytes(),
                r::RecordLimits::default(),
            )?;
            Ok(
                json!({"record":decoded.envelope(),"digest":decoded.raw_digest(),"claims_authenticated":false}),
            )
        }
        Operation::LegacyImport { payload } => {
            legacy_result(l::import(payload.as_bytes(), l::Limits::default())?)
        }
        Operation::LegacyCapture {
            dialect,
            adapter,
            entry,
            files,
        } => legacy_result(l::capture(
            &dialect,
            &adapter,
            &entry,
            blobs(files)?,
            l::Limits::default(),
        )?),
        Operation::Successor {
            predecessor,
            successor,
            paths,
        } => Ok(json!(l::check_successor(
            &l::import(predecessor.as_bytes(), l::Limits::default())?,
            &l::import(successor.as_bytes(), l::Limits::default())?,
            &paths.iter().map(String::as_str).collect::<Vec<_>>(),
            l::Limits::default()
        )?)),
    }
}
fn request(path: &str) -> Result<Request, Error> {
    let reader: Box<dyn Read> = if path == "-" {
        Box::new(std::io::stdin())
    } else {
        Box::new(std::fs::File::open(path).map_err(|e| Error::new("sdk.input", e))?)
    };
    let mut bytes = Vec::new();
    reader
        .take(INPUT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::new("sdk.input", e))?;
    if bytes.len() > INPUT_LIMIT {
        return Err(Error::new("resource-limit", "Request exceeds 4 MiB"));
    }
    let value = wire::decode(&bytes).map_err(|e| Error::new("sdk.request", e))?;
    let req: Request =
        serde_json::from_value(value.clone()).map_err(|e| Error::new("sdk.request", e))?;
    wire::shape(
        &value,
        &serde_json::to_value(&req).expect("request serializes"),
    )
    .map_err(|e| Error::new("sdk.request", e))?;
    if req.schema != "oh.war/sdk-request/v1" {
        return Err(Error::new("sdk.schema", "Unsupported request schema"));
    }
    Ok(req)
}
fn envelope(result: Result<Value, Error>) -> (u8, Value) {
    let (code, value, diagnostics) = match result {
        Ok(value) => (0, value, json!([])),
        Err(e) => (
            1,
            Value::Null,
            json!([{"severity":"error","rule":e.code,"code":e.code,"file":null,"message":e.message,"span":e.span}]),
        ),
    };
    (
        code,
        json!({"schema":"oh.war/report/v1","command":"sdk","diagnostics":diagnostics,
        "notes":[],"counts":{"pass":0,"warn":0,"unknown":0,"error":code,"worst":if code==0 {"pass"} else {"error"}},
        "verdict":if code==0 {"well_formed"} else {"not_ready"},
        "verdict_line":if code==0 {"SDK operation evaluated; see result for standing"} else {"SDK operation refused"},
        "exit_code":code,"result":value}),
    )
}
pub fn argument_error(message: &str) -> u8 {
    let (_, value) = envelope(Err(Error::new("sdk.arguments", message)));
    let mut data = serde_json::to_vec(&value).expect("report JSON");
    data.push(b'\n');
    let _ = std::io::stdout().write_all(&data);
    1
}
pub fn run(path: &str, output: Option<&Utf8Path>) -> u8 {
    let (mut code, value) = envelope(request(path).and_then(|r| execute(r.operation)));
    let mut data = match wire::encode(&value, OUTPUT_LIMIT - 1) {
        Ok(data) => data,
        Err(_) => {
            code = 1;
            serde_json::to_vec(
                &envelope(Err(Error::new("resource-limit", "Output exceeds 16 MiB"))).1,
            )
            .unwrap()
        }
    };
    data.push(b'\n');
    if code == 0
        && let Some(path) = output
        && let Err(e) = save_new(path, &data)
    {
        code = 1;
        data = serde_json::to_vec(&envelope(Err(Error::new("sdk.output", e))).1).unwrap();
        data.push(b'\n');
    }
    if std::io::stdout().write_all(&data).is_err() {
        return 1;
    }
    code
}
fn save_new(path: &Utf8Path, data: &[u8]) -> std::io::Result<()> {
    // A same-directory temporary file plus hard-link publish never replaces a destination.
    let parent = path
        .parent()
        .filter(|p| !p.as_str().is_empty())
        .unwrap_or(Utf8Path::new("."));
    let name = format!(
        ".ow-sdk-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let temp = parent.join(name);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        std::fs::hard_link(&temp, path)
    })();
    drop(file);
    let _ = std::fs::remove_file(temp);
    result
}
