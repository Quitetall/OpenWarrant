// SPDX-License-Identifier: Apache-2.0
//! Opt-in observations from a frozen retained Git tree. No tuning or authority.
use super::{Baseline, Measure, candidate_lines, take_with_candidates};
use crate::{
    diagnostic::{Report, Severity},
    repo::{RepoError, Repository},
};
use openwarrant_core::telemetry_metrics::{
    MeasurementUnit, MetricValue, ObservedTerm, derive_metric,
};

fn unavailable(message: impl Into<String>) -> RepoError {
    RepoError::ObservationUnavailable {
        rule: "telemetry.observation-unavailable",
        message: message.into(),
    }
}

pub fn take(repo: &Repository, candidate: &str) -> Result<Baseline, RepoError> {
    let snapshot =
        crate::acceptance::tree::Snapshot::read(&repo.root, candidate).map_err(unavailable)?;
    let captured = &snapshot.repository;
    let commit = &captured
        .candidate_history
        .as_ref()
        .ok_or_else(|| unavailable("frozen commit identity absent"))?
        .1;
    let range = if let Some(adoption) = &captured.config.adoption {
        crate::acceptance::git(
            &snapshot.history_root,
            &["merge-base", "--is-ancestor", &adoption.baseline, commit],
        )
        .ok_or_else(|| {
            unavailable(
                "adoption baseline is unavailable or not an ancestor of the selected commit",
            )
        })?;
        format!("{}..{commit}", adoption.baseline)
    } else {
        commit.clone()
    };
    let history =
        crate::acceptance::git(&snapshot.history_root, &["log", "--format=%H %s", &range])
            .ok_or_else(|| unavailable("cannot read selected history; no zero inferred"))?;
    let text =
        std::str::from_utf8(&history).map_err(|_| unavailable("selected history is not UTF-8"))?;
    let candidates = candidate_lines(captured, &history);
    let commits = text.lines().count() as u64;
    let mut baseline = take_with_candidates(captured, commit, Some(candidates))?;
    baseline.schema = "oh.war/telemetry-baseline/v2".into();
    let dirs = captured.warrant_dirs()?;
    let mut loaded = Vec::new();
    for dir in &dirs {
        let one = captured.load_warrant(dir)?;
        if one.basis.is_none() || one.validated.is_none() {
            return Err(unavailable(format!(
                "{} cannot compile; no Warrant denominator inferred",
                one.alias()
            )));
        }
        loaded.push(one);
    }
    let term = |value, population: &str, source: &str| ObservedTerm::Measured {
        value,
        unit: MeasurementUnit::Count,
        population: population.into(),
        source: format!("git:{commit}#{source}"),
    };
    let compute = |name, numerator, denominator| -> Result<Measure, RepoError> {
        derive_metric(name, numerator, denominator)
            .map(|value| match value {
                MetricValue::Ratio(ratio) => Measure::Ratio { ratio },
                MetricValue::Unavailable { not_measurable_yet } => {
                    Measure::NotYet { not_measurable_yet }
                }
            })
            .map_err(|error| RepoError::Message(error.to_string()))
    };
    let mut amendments = 0u64;
    let mut invalid_amendment = false;
    for dir in &dirs {
        for file in crate::amendment_id::files(dir) {
            if file.id.is_err() {
                invalid_amendment = true;
            }
            amendments += 1;
        }
    }
    let numerator = if invalid_amendment {
        ObservedTerm::Unavailable {
            reason: "native amendment record has an invalid identifier".into(),
        }
    } else {
        term(
            amendments,
            "native amendment record paths",
            "amendment-record-inventory",
        )
    };
    baseline.derived.insert(
        "amendments per WAR".into(),
        compute(
            "amendments per WAR",
            numerator,
            term(
                loaded.len() as u64,
                "compiled Warrants",
                "warrant-manifests",
            ),
        )?,
    );
    baseline.derived.insert(
        "untracked-work rate".into(),
        compute(
            "untracked-work rate",
            term(
                baseline.untracked_work_candidates.len() as u64,
                "commit candidates without a lexical tracking identifier",
                &range,
            ),
            term(
                commits,
                "all commits examined in the same history range",
                &range,
            ),
        )?,
    );
    let (mut reviews, mut catches) = (0, 0);
    for one in &loaded {
        for atom in one
            .basis
            .as_ref()
            .unwrap()
            .atoms
            .iter()
            .filter(|a| a.role == "assurance")
        {
            let source = std::str::from_utf8(&atom.bytes)
                .map_err(|_| unavailable("assurance source is not UTF-8"))?;
            let review = openwarrant_core::adequacy::parse(source);
            if review.present
                && review.has_outcome()
                && !review
                    .outcomes
                    .contains(&openwarrant_core::adequacy::AdequacyOutcome::ReviewNotRequired)
            {
                reviews += 1;
                if review
                    .outcomes
                    .contains(&openwarrant_core::adequacy::AdequacyOutcome::CounterexampleFound)
                {
                    catches += 1;
                }
            }
        }
    }
    baseline.derived.insert(
        "adequacy-review catch rate".into(),
        compute(
            "adequacy-review catch rate",
            term(
                catches,
                "parsed adequacy reviews reporting a counterexample",
                "assurance-outcomes",
            ),
            term(
                reviews,
                "parsed required adequacy reviews with recorded outcomes",
                "assurance-outcomes",
            ),
        )?,
    );
    let mut report = Report::default();
    let registry = crate::check::load_gate_registry(captured, &mut report);
    let citations: Vec<_> = loaded
        .iter()
        .flat_map(crate::resolve::cited_gate_keys)
        .collect();
    let bad_registry = report.count(Severity::Error) > 0
        || report.count(Severity::Unknown) > 0
        || citations.iter().any(|key| registry.get(key).is_none());
    let numerator = if bad_registry {
        ObservedTerm::Unavailable {
            reason: "gate definitions or cited references could not be established".into(),
        }
    } else {
        term(
            citations.len() as u64,
            "parsed gate URI citations in assurance atoms",
            "assurance-gate-citations",
        )
    };
    baseline.derived.insert(
        "gate-library reuse rate".into(),
        compute(
            "gate-library reuse rate",
            numerator,
            term(
                registry.len() as u64,
                "parsed gate definitions in the frozen registry",
                "gate-registry",
            ),
        )?,
    );
    for (name, reason) in [
        (
            "human control minutes per accepted WAR",
            "no instrumented human control durations correlated with an accepted cohort",
        ),
        (
            "safe auto-amendment fraction",
            "no per-amendment observed safety eligibility under an effective policy",
        ),
        (
            "gate-failure-to-repair success rate",
            "no correlated failure cases with complete repair follow-up",
        ),
        (
            "post-resolution escape rate",
            "no resolved-Warrant cohort with confirmed escape observations and a complete observation window",
        ),
    ] {
        baseline.derived.insert(
            name.into(),
            Measure::NotYet {
                not_measurable_yet: reason.into(),
            },
        );
    }
    Ok(baseline)
}

/// Atomic no-overwrite publication. Identical bytes are reusable; a different,
/// linked or nonregular existing artifact cannot be replaced by this operation.
#[cfg(unix)]
pub fn publish(path: &camino::Utf8Path, bytes: &[u8]) -> Result<(), RepoError> {
    use rustix::fs::{Mode, OFlags, open};
    use std::io::{Read, Write};
    struct Pending(camino::Utf8PathBuf);
    impl Drop for Pending {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| unavailable(e.to_string()))?
        .as_nanos();
    let temp = path.with_file_name(format!(".ow-derived-{}-{nonce}", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = options
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)
        .map_err(|e| unavailable(format!("cannot stage {path}: {e}")))?;
    let pending = Pending(temp);
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| unavailable(format!("cannot stage {path}: {e}")))?;
    match std::fs::hard_link(&pending.0, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let fd = open(
                path.as_std_path(),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|e| unavailable(format!("cannot safely read retained {path}: {e}")))?;
            let file: std::fs::File = fd.into();
            let metadata = file.metadata().map_err(|e| unavailable(e.to_string()))?;
            if !metadata.is_file() || metadata.len() != bytes.len() as u64 {
                return Err(RepoError::Message(format!(
                    "retained artifact {path} differs or is not regular; nothing replaced"
                )));
            }
            let mut old = Vec::new();
            file.take(bytes.len() as u64 + 1)
                .read_to_end(&mut old)
                .map_err(|e| unavailable(e.to_string()))?;
            if old != bytes {
                return Err(RepoError::Message(format!(
                    "retained artifact {path} differs; write a new artifact instead"
                )));
            }
            Ok(())
        }
        Err(error) => Err(unavailable(format!("cannot publish {path}: {error}"))),
    }
}

#[cfg(not(unix))]
pub fn publish(_: &camino::Utf8Path, _: &[u8]) -> Result<(), RepoError> {
    Err(unavailable(
        "safe candidate publication is unavailable on this platform",
    ))
}
