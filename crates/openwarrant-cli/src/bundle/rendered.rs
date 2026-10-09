// SPDX-License-Identifier: Apache-2.0
//! Bind displayed bytes and derived evidence to captured source facts. This
//! does not authenticate a verifier or make a short excerpt sufficient evidence.
use super::*;
use crate::verify::VerificationRequest;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn unavailable(message: &str) -> RepoError {
    RepoError::ObservationUnavailable {
        rule: "verify.rendered-unavailable",
        message: message.into(),
    }
}

fn value<T: Serialize>(v: &T) -> Result<Value, RepoError> {
    serde_json::to_value(v).map_err(|e| RepoError::Message(e.to_string()))
}

fn source_bytes(s: &RequiredSource) -> Option<Vec<u8>> {
    s.text
        .as_ref()
        .map(|s| s.as_bytes().to_vec())
        .or_else(|| s.bytes.clone())
}

/// Use the already captured run/receipt/stream bytes, not a second filesystem
/// read which could pair a different output with the reviewed source digest.
pub(super) fn runs_from_sources(sources: &[RequiredSource]) -> Result<Vec<Run>, RepoError> {
    let read = |path: &str| -> Result<Option<Vec<u8>>, RepoError> {
        sources
            .iter()
            .find(|s| s.path == path && s.kind == "gate-evidence")
            .map(source_bytes)
            .ok_or_else(|| unavailable("gate evidence is absent from the captured source set"))
    };
    let mut runs = Vec::new();
    for source in sources
        .iter()
        .filter(|s| s.kind == "gate-evidence" && s.path.ends_with(".run.toml"))
    {
        let bytes = source_bytes(source).ok_or_else(|| unavailable("a captured run is missing"))?;
        let text = std::str::from_utf8(&bytes).map_err(|e| RepoError::Message(e.to_string()))?;
        let run: openwarrant_core::GateRun =
            toml::from_str(text).map_err(|e| RepoError::Message(e.to_string()))?;
        let stem = source.path.strip_suffix(".run.toml").unwrap();
        let receipt = read(&format!("{stem}.receipt.json"))?
            .map(|b| serde_json::from_slice::<openwarrant_core::GateReceipt>(&b))
            .transpose()
            .map_err(|e| RepoError::Message(e.to_string()))?;
        let receipt = receipt.as_ref().map(carried_receipt);
        let stream = |name: &str| -> Result<Stream, RepoError> {
            let beside = format!("{stem}.{name}.txt");
            let path = receipt
                .as_ref()
                .and_then(|r| r[format!("{name}_ref")].as_str())
                .unwrap_or(&beside);
            Ok(Stream {
                bytes: read(path)?,
                recorded: receipt
                    .as_ref()
                    .and_then(|r| r[format!("{name}_digest")].as_str())
                    .map(str::to_owned),
            })
        };
        runs.push(Run {
            gate: run.gate.clone(),
            run_id: run.id.clone(),
            run: value(&run)?,
            receipt: receipt.clone(),
            stdout: stream("stdout")?,
            stderr: stream("stderr")?,
        });
    }
    Ok(runs)
}

/// Map a boundary in lossily decoded text to possible original byte cuts.
/// Invalid UTF-8 can map several cuts to the same replacement character.
fn raw_boundaries(raw: &[u8], offset: usize) -> Vec<usize> {
    let (mut bytes, mut text) = (0, 0);
    let mut found = Vec::new();
    for chunk in raw.utf8_chunks() {
        let n = chunk.valid().len();
        if offset >= text && offset <= text + n {
            found.push(bytes + offset - text);
        }
        bytes += n;
        text += n;
        let n = chunk.invalid().len();
        if n > 0 {
            if offset == text || offset == text + 3 {
                found.extend(bytes..=bytes + n);
            }
            bytes += n;
            text += 3;
        }
    }
    if offset == text {
        found.push(bytes);
    }
    found.sort_unstable();
    found.dedup();
    found
}

/// Accept real head/tail cuts, including cuts inside UTF-8 sequences. Check
/// candidate cuts against the original bytes; substring similarity is not proof.
fn edge(raw: &[u8], shown: &str, tail: bool) -> bool {
    let whole = String::from_utf8_lossy(raw);
    let mut cuts = Vec::new();
    if tail && whole.ends_with(shown) {
        cuts.extend(raw_boundaries(raw, whole.len() - shown.len()));
    } else if !tail && whole.starts_with(shown) {
        cuts.extend(raw_boundaries(raw, shown.len()));
    }
    let mut inner = shown;
    // A tail starting inside a four-byte character can decode its three
    // continuation bytes as three separate replacement characters.
    for _ in 0..3 {
        let Some(rest) = (if tail {
            inner.strip_prefix('\u{fffd}')
        } else {
            inner.strip_suffix('\u{fffd}')
        }) else {
            break;
        };
        inner = rest;
        let offset = if tail && whole.ends_with(inner) {
            Some(whole.len() - inner.len())
        } else if !tail && whole.starts_with(inner) {
            Some(inner.len())
        } else {
            None
        };
        if let Some(offset) = offset {
            for boundary in raw_boundaries(raw, offset) {
                for n in 1..=3 {
                    cuts.push(if tail {
                        boundary.saturating_sub(n)
                    } else {
                        boundary.saturating_add(n)
                    });
                }
            }
        }
    }
    cuts.into_iter().any(|cut| {
        if tail {
            cut > 0 && cut <= raw.len() && String::from_utf8_lossy(&raw[cut..]) == shown
        } else {
            cut < raw.len() && String::from_utf8_lossy(&raw[..cut]) == shown
        }
    })
}

fn excerpts(raw: &[u8], shown: &[Excerpt]) -> bool {
    let whole = String::from_utf8_lossy(raw);
    let mut starts = vec![0];
    for (at, byte) in whole.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(at + 1);
        }
    }
    let mut cursor = 0;
    for e in shown {
        if e.start_line == 0
            || e.end_line < e.start_line
            || e.end_line > starts.len()
            || e.text.is_empty()
        {
            return false;
        }
        let from = starts[e.start_line - 1].max(cursor);
        let to = starts.get(e.end_line).copied().unwrap_or(whole.len());
        if from > to {
            return false;
        }
        let Some(at) = whole[from..to].find(&e.text) else {
            return false;
        };
        let at = from + at;
        let last = at + e.text.len() - e.text.chars().last().unwrap().len_utf8();
        if whole[..at].bytes().filter(|b| *b == b'\n').count() + 1 != e.start_line
            || whole[..last].bytes().filter(|b| *b == b'\n').count() + 1 != e.end_line
        {
            return false;
        }
        cursor = at + e.text.len();
    }
    true
}

fn deliverable_matches(file: &File, shown: &BundledDeliverable) -> Result<bool, RepoError> {
    let expected = value(&carry_file(file, Carry::Head(0), &[], vec![]))?;
    let actual = value(shown)?;
    for field in [
        "id",
        "title",
        "target_ref",
        "present",
        "error",
        "sha256",
        "bytes",
        "lines",
        "test_names",
    ] {
        if expected[field] != actual[field] {
            return Ok(false);
        }
    }
    let Ok(raw) = &file.read else {
        return Ok(!shown.truncated && shown.text.is_none() && shown.excerpts.is_empty());
    };
    Ok(if !shown.truncated {
        shown.text.as_deref() == Some(String::from_utf8_lossy(raw).as_ref())
            && shown.excerpts.is_empty()
    } else if let Some(text) = &shown.text {
        shown.excerpts.is_empty() && edge(raw, text, false)
    } else {
        !raw.is_empty() && excerpts(raw, &shown.excerpts)
    })
}

fn stream_matches(stream: &Stream, shown: &Value) -> Result<bool, RepoError> {
    let Some(raw) = &stream.bytes else {
        return Ok(*shown == serde_json::json!({"captured":false}));
    };
    let Some(fields) = shown.as_object() else {
        return Ok(false);
    };
    if fields.keys().any(|k| {
        ![
            "captured",
            "sha256",
            "bytes",
            "truncated",
            "text",
            "excerpts",
            "mismatch",
        ]
        .contains(&k.as_str())
    }) {
        return Ok(false);
    }
    let sha = sha256_hex(raw);
    let mismatch = stream
        .recorded
        .as_ref()
        .is_some_and(|d| d.trim_start_matches("sha256:") != sha);
    if shown["captured"] != true
        || shown["sha256"] != sha
        || shown["bytes"] != raw.len()
        || shown.get("mismatch") != mismatch.then_some(&Value::Bool(true))
    {
        return Ok(false);
    }
    let Some(text) = shown["text"].as_str() else {
        return Ok(false);
    };
    let extra: Vec<Excerpt> = match shown.get("excerpts") {
        Some(v) => match serde_json::from_value(v.clone()) {
            Ok(v) => v,
            Err(_) => return Ok(false),
        },
        None => vec![],
    };
    if shown
        .get("excerpts")
        .is_some_and(|v| value(&extra).ok().as_ref() != Some(v))
    {
        return Ok(false);
    }
    Ok(match shown["truncated"].as_bool() {
        Some(false) => text == String::from_utf8_lossy(raw) && extra.is_empty(),
        Some(true) => edge(raw, text, true) && excerpts(raw, &extra),
        None => false,
    })
}

fn choice(src: &Sources, o: &RequestedObligation) -> (String, Vec<Vec<String>>) {
    let (paths, _) = named(o);
    let mut why = vec![vec![]; src.files.len()];
    for (i, f) in src.files.iter().enumerate() {
        if f.record.obligation_refs.contains(&o.id) {
            why[i].push("obligation_refs".into());
        }
        for p in &paths {
            if names_deliverable(p, &f.record) {
                why[i].push(format!("named `{p}`"));
            }
        }
    }
    if why.iter().all(Vec::is_empty) {
        for w in &mut why {
            w.push("all".into());
        }
        (
            "all: the obligation names no deliverable and none lists it".into(),
            why,
        )
    } else {
        ("named: the deliverables whose obligation_refs list it, and those its statement, scope or evidence names by path".into(), why)
    }
}

/// One captured evidence set is shared by every packet in a review. Never
/// resnapshot between obligation packets.
pub(crate) struct Captured(Sources);

impl Captured {
    pub(crate) fn new(
        repo: &Repository,
        one: &crate::repo::Loaded,
        request: VerificationRequest,
    ) -> Result<Self, RepoError> {
        load_from_loaded(repo, one, request).map(Self)
    }

    pub(crate) fn matches(&self, packet: &Value) -> Result<bool, RepoError> {
        let src = &self.0;
        // Authorization is independently checked from signed records. This
        // optional display describes packet creation, not the current trust
        // state: a review captured before authorization must survive signing
        // that same contract (55-evidence-signing.sh).
        if packet["plants"] != value(&src.plants)? {
            return Ok(false);
        }
        let Some(raw_files) = packet["deliverables"].as_array() else {
            return Ok(false);
        };
        let shown: Vec<BundledDeliverable> =
            match serde_json::from_value(packet["deliverables"].clone()) {
                Ok(v) => v,
                Err(_) => return Ok(false),
            };
        if value(&shown)? != packet["deliverables"] {
            return Ok(false);
        }
        let scope = packet["scope"].as_str().unwrap_or_default();
        let obligations: Vec<RequestedObligation> =
            match serde_json::from_value(packet["request"]["obligations"].clone()) {
                Ok(v) => v,
                Err(_) => return Ok(false),
            };
        let (selection, why) = if scope == "warrant" {
            (
                "warrant: every deliverable of the Warrant".into(),
                vec![vec![]; src.files.len()],
            )
        } else if let [o] = obligations.as_slice() {
            choice(src, o)
        } else {
            return Ok(false);
        };
        let expected: Vec<_> = (0..src.files.len())
            .filter(|&i| scope == "warrant" || !why[i].is_empty())
            .collect();
        let mut carried = Vec::new();
        let mut ids = BTreeSet::new();
        for (raw, d) in raw_files.iter().zip(&shown) {
            if !ids.insert(&d.id) {
                return Ok(false);
            }
            let Some(i) = src.files.iter().position(|f| f.record.id == d.id) else {
                return Ok(false);
            };
            if !expected.contains(&i)
                || !deliverable_matches(&src.files[i], d)?
                || d.carried_because != why[i]
                || raw != &value(d)?
            {
                return Ok(false);
            }
            carried.push((i, d));
        }
        if carried.len() != expected.len() {
            return Ok(false);
        }
        let omitted: Vec<_> = src
            .files
            .iter()
            .enumerate()
            .filter(|(i, _)| !expected.contains(i))
            .map(|(_, f)| NotCarried {
                id: f.record.id.clone(),
                target_ref: f.record.target_ref.clone(),
                sha256: f.read.as_ref().map(|b| sha256_hex(b)).unwrap_or_default(),
                bytes: f.read.as_ref().map_or(0, |b| b.len() as u64),
            })
            .collect();
        let not_carried = packet
            .get("deliverables_not_carried")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        if not_carried != value(&omitted)? {
            return Ok(false);
        }
        let Some(runs) = packet["gate_runs"].as_array() else {
            return Ok(false);
        };
        if runs.len() != src.runs.len() {
            return Ok(false);
        }
        let mut run_keys = BTreeSet::new();
        for r in runs {
            let Some(actual) = src
                .runs
                .iter()
                .find(|s| r["gate"] == s.gate && r["run_id"] == s.run_id)
            else {
                return Ok(false);
            };
            if !run_keys.insert((&actual.gate, &actual.run_id))
                || r.as_object().is_none_or(|m| m.len() != 6)
                || r["run"] != actual.run
                || r["receipt"] != value(&actual.receipt)?
                || !stream_matches(&actual.stdout, &r["stdout"])?
                || !stream_matches(&actual.stderr, &r["stderr"])?
            {
                return Ok(false);
            }
        }
        let ids: Vec<_> = shown.iter().map(|d| d.id.clone()).collect();
        let evidence: Vec<_> = obligations
            .iter()
            .map(|o| {
                let (paths, terms) = named(o);
                ObligationEvidence {
                    obligation: o.id.clone(),
                    selection: selection.clone(),
                    named_paths: named_paths(src, &paths, &ids),
                    terms: term_places(src, &terms, &carried, runs),
                }
            })
            .collect();
        Ok(packet["obligation_evidence"] == value(&evidence)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_utf8_and_binary_edge_cut_is_real_and_substitution_is_refused() {
        for raw in [
            "é🦀 text\n".as_bytes(),
            &[0xff, 0xe2, 0x82, b'x', 0xc3, 0xa9][..],
            b"plain\n".as_slice(),
        ] {
            for cut in 0..raw.len() {
                assert!(
                    edge(raw, &String::from_utf8_lossy(&raw[..cut]), false),
                    "head cut {cut}: {raw:?}"
                );
            }
            for cut in 1..=raw.len() {
                assert!(
                    edge(raw, &String::from_utf8_lossy(&raw[cut..]), true),
                    "tail cut {cut}: {raw:?}"
                );
            }
            assert!(!edge(raw, "invented bytes", false));
            assert!(!edge(raw, "invented bytes", true));
        }
    }
    #[test]
    fn excerpts_bind_line_positions_and_refuse_wrong_text_and_repetition() {
        let bytes = b"first\nsecond long line\nthird\n";
        let good = Excerpt {
            start_line: 2,
            end_line: 2,
            text: "long".into(),
        };
        assert!(excerpts(bytes, std::slice::from_ref(&good)));
        assert!(!excerpts(
            bytes,
            &[Excerpt {
                start_line: 1,
                ..good.clone()
            }]
        ));
        assert!(!excerpts(
            bytes,
            &[Excerpt {
                text: "invented".into(),
                ..good.clone()
            }]
        ));
        assert!(!excerpts(bytes, &[good.clone(), good]));
    }
}
