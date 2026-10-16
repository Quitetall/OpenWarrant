// SPDX-License-Identifier: Apache-2.0
//! Experimental model view beside a lossless v2 archive (proposed ADR0030).
//! Previews and opt-in model inputs use distinct wire identities. Emission
//! never calls a verifier, ingests an observation or grants assurance.
use super::*;
use serde_json::{Value, json};

const SCHEMA: &str = "oh.war/verification-view-preview/v1";
const EXECUTABLE_SCHEMA: &str = openwarrant_core::compact_verification_record::VIEW_SCHEMA;

#[derive(Clone, Copy)]
enum Mode {
    Preview,
    Executable,
}

fn encoded(value: &Value) -> Result<String, RepoError> {
    serde_jcs::to_string(value).map_err(|e| RepoError::Message(e.to_string()))
}

/// Estimate the actual final preview, including its accounting fields.
fn account(mut value: Value, budget: u64) -> Result<Value, RepoError> {
    value["estimated_tokens"] = json!(0);
    value["over_budget"] = json!(false);
    for _ in 0..16 {
        let estimate = openwarrant_core::tokens::estimate(encoded(&value)?.len() as u64);
        let eligible = value["schema"] == EXECUTABLE_SCHEMA && estimate <= budget;
        if value["estimated_tokens"] == estimate
            && value["over_budget"] == (estimate > budget)
            && value["dispatch_eligible"] == eligible
        {
            return Ok(value);
        }
        value["estimated_tokens"] = json!(estimate);
        value["over_budget"] = json!(estimate > budget);
        value["dispatch_eligible"] = json!(eligible);
    }
    Err(RepoError::Message(
        "verify.preview-accounting: token estimate did not converge".into(),
    ))
}

struct Preview {
    archive_name: String,
    archive_bytes: Vec<u8>,
    view: Value,
}

fn project(bundle: &Bundle) -> Result<Preview, RepoError> {
    project_for(bundle, Mode::Preview)
}

fn project_for(bundle: &Bundle, mode: Mode) -> Result<Preview, RepoError> {
    let (archive, digest) = canonical(bundle)?;
    let archive_name = format!("bundle-{}.json", short(&digest));
    let archive_bytes = (archive + "\n").into_bytes();
    let archive = serde_json::to_value(bundle).map_err(|e| RepoError::Message(e.to_string()))?;
    let view = project_archive_for(archive, &archive_bytes, &digest, mode)?;
    Ok(Preview {
        archive_name,
        archive_bytes,
        view,
    })
}

fn project_archive(view: Value, archive_bytes: &[u8], digest: &str) -> Result<Value, RepoError> {
    project_archive_for(view, archive_bytes, digest, Mode::Preview)
}

fn project_archive_for(
    mut view: Value,
    archive_bytes: &[u8],
    digest: &str,
    mode: Mode,
) -> Result<Value, RepoError> {
    let budget = view["budget_tokens"]
        .as_u64()
        .ok_or_else(|| RepoError::Message("verify.preview-budget: expected token budget".into()))?;
    let input_count = view["request"]["reviewed_subject"]["gate_inputs"]
        .as_object()
        .map_or(0, |m| m.len());
    view["schema"] = json!(match mode {
        Mode::Preview => SCHEMA,
        Mode::Executable => EXECUTABLE_SCHEMA,
    });
    view["dispatch_eligible"] = json!(false);
    view["limitations"] = json!([
        "Preview only: current verifier adapters and ingestion do not support this view.",
        "Archived bytes are available offline, but have not been shown in this view.",
        "Excerpts cannot establish claims requiring observations outside their coverage."
    ]);
    if matches!(mode, Mode::Executable) {
        view["response_schema"] =
            json!(openwarrant_core::compact_verification_record::RESPONSE_SCHEMA);
        view["limitations"] = json!([
            "Only a v3-aware verifier may consume this model input; budget eligibility is not authority or assurance.",
            "Archived bytes omitted from this view have not been shown to the model.",
            "Excerpts cannot establish claims requiring observations outside their coverage."
        ]);
    }
    view["archive"] = json!({
        "path": format!("bundle-{}.json", short(digest)),
        "digest": digest,
        "sha256": sha256_hex(archive_bytes),
        "bytes": archive_bytes.len()
    });
    let request = view["request"].as_object_mut().ok_or_else(|| {
        RepoError::Message("verify.preview-request: expected request object".into())
    })?;
    request.remove("reviewed_subject");
    if matches!(mode, Mode::Executable)
        && let Some(source_schema) = request.remove("schema")
    {
        // This is a projection, not another instance of the frozen request.
        request.insert("source_schema".into(), source_schema);
    }
    request.insert("reviewed_subject_archive".into(), json!({
        "pointer": "/request/reviewed_subject",
        "gate_input_count": input_count,
        "meaning": "Exact workspace bindings are retained in the accompanying archive; this is not a source coverage claim."
    }));
    // Raw evidence remains lossless in the archive. The existing gate_runs
    // projection carries explicitly numbered excerpts under its own bounds.
    for source in view["required_sources"]
        .as_array_mut()
        .ok_or_else(|| RepoError::Message("verify.preview-sources: expected source array".into()))?
    {
        if source["kind"] == "gate-evidence" {
            let source = source.as_object_mut().ok_or_else(|| {
                RepoError::Message("verify.preview-source: expected source object".into())
            })?;
            source.remove("text");
            source.remove("bytes");
            source.insert("archive_only".into(), json!(true));
        }
    }
    account(view, budget)
}

fn read_value(path: &Utf8Path) -> Result<(Vec<u8>, Value), RepoError> {
    let relative = path.strip_prefix("/").map_err(|_| {
        RepoError::Message("verify.preview-path: an absolute file path is required".into())
    })?;
    let bytes = crate::progress_viewer::source::read(
        std::path::Path::new("/"),
        relative.as_std_path(),
        64 * 1024 * 1024,
    )
    .map_err(|e| RepoError::ObservationUnavailable {
        rule: "verify.preview-unavailable",
        message: e.to_string(),
    })?;
    let value =
        crate::sdk::wire::decode_value_with_limits(&bytes, bytes.len().saturating_add(1), 128)
            .map_err(|e| RepoError::Message(format!("verify.preview-malformed: {e}")))?;
    Ok((bytes, value))
}

/// Read-only prerequisite for a future compact review protocol. Reuses the
/// current packet verifier, but does not import a packet or ingest a verdict.
pub(crate) fn inspect(
    repo: &Repository,
    alias: &str,
    performer: &str,
    path: &Utf8Path,
) -> Result<(Report, Value), RepoError> {
    let (bytes, view) = read_value(path)?;
    let name = format!("preview-{}.json", sha256_hex(&bytes));
    if let Some(found) = view["schema"].as_str()
        && matches!((crate::repo::compat::parse_schema(found), crate::repo::compat::parse_schema(SCHEMA)),
            (Some((kind, major)), Some((known, supported))) if kind == known && major > supported)
    {
        return Err(RepoError::ObservationUnavailable {
            rule: "verify.preview-schema",
            message: format!(
                "unsupported newer preview protocol {found}; nothing was imported or verified"
            ),
        });
    }
    if path.file_name() != Some(name.as_str()) || view["schema"] != SCHEMA {
        return Err(RepoError::Message(
            "verify.preview-binding: schema or exact view filename does not match".into(),
        ));
    }
    let digest = view["archive"]["digest"]
        .as_str()
        .filter(|d| d.len() == 64 && d.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
        .ok_or_else(|| {
            RepoError::Message("verify.preview-binding: invalid archive digest".into())
        })?;
    let archive_name = format!("bundle-{}.json", short(digest));
    if view["archive"]["path"] != archive_name {
        return Err(RepoError::Message(
            "verify.preview-binding: archive must be the exact sibling bundle".into(),
        ));
    }
    let archive_path = path
        .parent()
        .ok_or_else(|| RepoError::Message("verify.preview-path: missing parent".into()))?
        .join(&archive_name);
    let (archive_bytes, archive) = read_value(&archive_path)?;
    let actual = sha256_digest(DigestDomain::VerificationBundle, &archive)
        .map_err(|e| RepoError::Message(e.to_string()))?;
    if actual != digest || project_archive(archive.clone(), &archive_bytes, digest)? != view {
        return Err(RepoError::Message(
            "verify.preview-binding: view or archive differs from the exact projection".into(),
        ));
    }
    let one = repo.load_warrant(&repo.warrant_dir(alias)?)?;
    let reviewed: crate::verify::ReviewedSubject =
        serde_json::from_value(archive["request"]["reviewed_subject"].clone())
            .map_err(|e| RepoError::Message(e.to_string()))?;
    if crate::verify::subject(repo, &one)? != reviewed {
        return Err(RepoError::Message("verify.preview-subject-stale: reviewed contract or sources changed; nothing was written".into()));
    }
    let request: crate::verify::VerificationRequest =
        serde_json::from_value(archive["request"].clone())
            .map_err(|e| RepoError::Message(e.to_string()))?;
    let obligations: Vec<_> = request.obligations.iter().map(|o| o.id.clone()).collect();
    let reference = crate::verify::ReviewedPacket {
        path: repo.relative(&one.dir.join("verifications").join(&archive_name)),
        digest: digest.to_owned(),
    };
    if !crate::verify::packets_cover_with(
        repo,
        &one,
        &reviewed,
        &[reference],
        &obligations,
        performer,
        |_| Ok(archive_bytes.clone()),
    )? {
        return Err(RepoError::Message("verify.preview-binding: archived task, sources or evidence do not match the current subject; nothing was written".into()));
    }
    let mut report = Report::default();
    report.push(Diagnostic::pass(
        "verify.preview.inspect",
        "Exact view, archive and current subject agree; no verifier or verdict was produced",
    ));
    Ok((
        report,
        json!({"schema": "oh.war/review-preview-inspection/v1", "path": path.as_str(), "dispatch_eligible": false, "estimated_tokens": view["estimated_tokens"], "over_budget": view["over_budget"]}),
    ))
}

fn obligation(src: &Sources, requested: &RequestedObligation) -> Result<Preview, RepoError> {
    obligation_for(src, requested, Mode::Preview)
}

fn obligation_for(
    src: &Sources,
    requested: &RequestedObligation,
    mode: Mode,
) -> Result<Preview, RepoError> {
    let fixed = project_for(&obligation_bundle_at(src, requested, 0)?, mode)?;
    let budget = src.budget.saturating_mul(4);
    let fixed_bytes = encoded(&fixed.view)?.len() as u64;
    if fixed_bytes >= budget {
        return Ok(fixed);
    }
    let mut allowance = usize::try_from(budget - fixed_bytes).unwrap_or(usize::MAX);
    for _ in 0..16 {
        let preview = project_for(&obligation_bundle_at(src, requested, allowance)?, mode)?;
        if preview.view["over_budget"] == false || allowance == 0 {
            return Ok(preview);
        }
        let over = (encoded(&preview.view)?.len() as u64).saturating_sub(budget);
        let over = usize::try_from(over).unwrap_or(usize::MAX);
        allowance = allowance.saturating_sub(over.saturating_add(over / 4).saturating_add(1024));
    }
    Ok(fixed)
}

/// Retain v3 model inputs and their exact v2 archives. Canonical archives stay
/// at the paths checked by existing packet validation. Each view also carries
/// a sibling archive copy so the pair remains portable offline. Both view
/// outputs are under the existing verification bookkeeping namespace.
pub(crate) fn emit_compact(
    repo: &Repository,
    alias: &str,
    performer: &str,
) -> Result<(Report, Value), RepoError> {
    let directory = repo.warrant_dir(alias)?.join("verifications");
    let relative = repo.relative(&directory);
    let archives = store::Directory::open(&repo.root, Utf8Path::new(&relative))?;
    let view_directory = directory.join("responses/views");
    let relative_views = repo.relative(&view_directory);
    let retained_views = store::Directory::open(&repo.root, Utf8Path::new(&relative_views))?;
    // Create namespaces before capture, including when an obligation names a
    // directory. No output may silently change the subject it just captured.
    let src = load(repo, alias, performer)?;
    let inputs = if src.request.obligations.is_empty() {
        vec![project_for(&warrant_bundle(&src)?, Mode::Executable)?]
    } else {
        src.request
            .obligations
            .iter()
            .map(|o| obligation_for(&src, o, Mode::Executable))
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut report = Report::default();
    let mut packets = Vec::new();
    let mut views = Vec::new();
    for input in inputs {
        let digest = input.view["archive"]["digest"]
            .as_str()
            .ok_or_else(|| RepoError::Message("verify.compact-archive: missing digest".into()))?;
        let text = encoded(&input.view)?;
        let sha = sha256_hex(text.as_bytes());
        let name = format!("view-{sha}.json");
        archives.retain(&input.archive_name, &input.archive_bytes)?;
        retained_views.retain(&input.archive_name, &input.archive_bytes)?;
        retained_views.retain(&name, text.as_bytes())?;
        packets.push(
            json!({"path": repo.relative(&directory.join(&input.archive_name)), "digest": digest}),
        );
        views.push(json!({"path": repo.relative(&view_directory.join(&name)), "sha256": sha}));
        report.push(Diagnostic::pass("verify.compact.bundle", format!("{alias}: retained model input {name} and exact archive; no verifier or result ingestion was performed")));
        if input.view["over_budget"] == true {
            report.push(Diagnostic::warn("verify.compact-over-budget", repo.relative(&view_directory.join(&name)), "Required context exceeds the reading budget; exact input retained and dispatch eligibility is false"));
        }
    }
    let subject_archive = packets.first().map(|packet| {
        json!({
            "path": packet["path"], "digest": packet["digest"],
            "pointer": "/request/reviewed_subject"
        })
    });
    Ok((
        report,
        json!({"schema": "oh.war/review-context/v3", "subject_archive": subject_archive, "packets": packets, "views": views, "qualification_established": false}),
    ))
}

pub(crate) fn emit(
    repo: &Repository,
    alias: &str,
    performer: &str,
    directory: &Utf8Path,
) -> Result<(Report, Value), RepoError> {
    if !directory.is_absolute()
        || directory.starts_with(&repo.root)
        || directory.as_std_path().components().any(|c| {
            !matches!(
                c,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
    {
        return Err(RepoError::Message("verify.preview-path: an absolute directory outside the reviewed repository, without dot components, is required".into()));
    }
    // Anchor every output component without following symlinks. Keep package
    // outputs out of the workspace: emitting a preview must not change its subject.
    let relative = directory
        .strip_prefix("/")
        .map_err(|e| RepoError::Message(e.to_string()))?;
    let retained = store::Directory::open(Utf8Path::new("/"), relative)?;
    let src = load(repo, alias, performer)?;
    let previews = if src.request.obligations.is_empty() {
        vec![project(&warrant_bundle(&src)?)?]
    } else {
        src.request
            .obligations
            .iter()
            .map(|o| obligation(&src, o))
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut report = Report::default();
    let mut index = Vec::new();
    for preview in previews {
        let text = encoded(&preview.view)?;
        let hash = sha256_hex(text.as_bytes());
        let name = format!("preview-{}.json", hash.trim_start_matches("sha256:"));
        retained.retain(&preview.archive_name, &preview.archive_bytes)?;
        retained.retain(&name, text.as_bytes())?;
        index.push(
            json!({"path": directory.join(&name).as_str(), "sha256": hash,
            "estimated_tokens": preview.view["estimated_tokens"],
            "budget_tokens": src.budget, "over_budget": preview.view["over_budget"],
            "dispatch_eligible": false}),
        );
        report.push(Diagnostic::pass("verify.preview", format!("{alias}: review preview and full archive retained at {directory}/{name}; no verification or dispatch performed")));
        if preview.view["over_budget"] == true {
            report.push(Diagnostic::warn("verify.preview-over-budget", directory.to_string(), "Exact required context exceeds the preview budget; it was retained without removing rules or fixtures"));
        }
    }
    Ok((
        report,
        json!({"schema": "oh.war/review-previews/v1", "dispatch_eligible": false, "previews": index}),
    ))
}
