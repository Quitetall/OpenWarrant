// SPDX-License-Identifier: AGPL-3.0-or-later
//! The diagnostic model behind `war check` (SAS §71.7, §76.2).

use std::fmt;

/// Severity of one finding.
///
/// `Unknown` is a first-class severity, not a convenience. SAS Law 15: "Unknown
/// is not failure and not pass." A check that cannot be performed reports
/// exactly that — degrading it to ERROR makes a sound Warrant look defective,
/// and degrading it to PASS makes an unasked question look answered. §96.4
/// states the same rule for migrating legacy gate results ("SHALL not collapse
/// 'could not ask' into 'failed'"), and it applies to our own output for the
/// same reason.
///
/// Retrofitting this later would mean auditing every check that had already
/// collapsed it into something else, which is why it exists from the first
/// version.
///
/// # Ordering
///
/// Declaration order IS severity order, and it must agree with
/// [`Severity::blocks_readiness`].
///
/// `Unknown` outranks `Warn` deliberately. An earlier version had them the other
/// way round, and a test caught the consequence: a report holding both a warning
/// and an unknown reported `worst() == Warn`, which does not block, so
/// `is_ready()` answered **ready while an unresolved unknown was sitting in the
/// report**. A severity lattice that disagrees with the blocking predicate
/// manufactures exactly the false PASS this whole system exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Pass,
    Warn,
    Unknown,
    Error,
}

impl Severity {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Unknown => "UNKNOWN",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    /// Whether a Warrant carrying this severity may be considered well-formed.
    #[must_use]
    pub const fn blocks_readiness(self) -> bool {
        match self {
            Self::Pass | Self::Warn => false,
            // An unknown blocks: §54 requires a required unknown gate result to
            // block resolution, and the same caution applies to a check that
            // could not run. Proceeding on an unanswered question is how a false
            // PASS is manufactured.
            Self::Unknown | Self::Error => true,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// The rule this finding is about, e.g. `manifest.duplicate-ordinal`.
    pub rule: String,
    /// The file the finding is anchored to, repository-relative.
    pub file: Option<String>,
    pub message: String,
}

impl Diagnostic {
    pub fn new(
        severity: Severity,
        rule: impl Into<String>,
        file: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            rule: rule.into(),
            file,
            message: message.into(),
        }
    }

    pub fn pass(rule: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Pass, rule, None, message)
    }

    pub fn error(
        rule: impl Into<String>,
        file: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Error, rule, Some(file.into()), message)
    }

    pub fn warn(
        rule: impl Into<String>,
        file: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Warn, rule, Some(file.into()), message)
    }

    pub fn unknown(
        rule: impl Into<String>,
        file: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Unknown, rule, Some(file.into()), message)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<7} {:<34} {}", self.severity, self.rule, self.message)?;
        if let Some(file) = &self.file {
            write!(f, "\n{:<7} {:<34}   → {file}", "", "")?;
        }
        Ok(())
    }
}

/// The findings for one Warrant, plus its verdict.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    /// Statements of what this run does NOT cover.
    ///
    /// Notes are printed on every run, including a clean one, because a report
    /// that answers "ok" while whole classes of check go unasked reads as full
    /// coverage. They are deliberately NOT diagnostics: an early version made
    /// the Phase 1 scope note an `Unknown`, which meant readiness could never be
    /// reached on any corpus — a verdict that is always "NOT READY" carries no
    /// information and gets ignored, which is the exact failure mode the
    /// severity model exists to prevent.
    pub notes: Vec<String>,
}

impl Report {
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Record something this run did not check.
    pub fn note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    #[must_use]
    pub fn count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count()
    }

    /// The highest severity present, or `Pass` for an empty report.
    #[must_use]
    pub fn worst(&self) -> Severity {
        self.diagnostics
            .iter()
            .map(|d| d.severity)
            .max()
            .unwrap_or(Severity::Pass)
    }

    /// Whether every check that ran was clean enough to proceed.
    ///
    /// Asks every diagnostic directly rather than going through
    /// [`Self::worst`]. Deriving readiness from the maximum severity makes the
    /// answer depend on the enum's ordering agreeing with `blocks_readiness`,
    /// and when those two disagreed, a report containing a blocking unknown
    /// reported itself ready. Two independent facts should not be inferred from
    /// one another.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|d| d.severity.blocks_readiness())
    }

    /// The readiness line printed after the findings.
    ///
    /// Deliberately never the bare word "READY". §32 defines readiness as
    /// including Preflight, which does not exist in Phase 1; printing "READY"
    /// unqualified would claim more than was checked, and a reader would
    /// reasonably take it to mean the work can be executed.
    #[must_use]
    pub fn verdict_line(&self) -> String {
        if self.is_ready() {
            "WELL-FORMED (record only — Preflight is not implemented, and `war check` does not run gates)"
                .to_owned()
        } else {
            "NOT READY".to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    /// OW-WAR-0005 OBL-001 — §71.7's output shape: `SEVERITY rule  message`
    /// per line, severity a fixed vocabulary, rule a dotted lowercase name.
    #[test]
    fn a_rendered_diagnostic_has_the_seventy_one_seven_shape() {
        for d in [
            Diagnostic::pass("manifest.valid", "ok"),
            Diagnostic::warn("independence.insufficient", "x".to_owned(), "why"),
            Diagnostic::error("generated.drift", "y".to_owned(), "why"),
            Diagnostic::unknown("gate-run.unaskable", "z".to_owned(), "why"),
        ] {
            let line = d.to_string();
            let (sev, rest) = line.split_once(' ').expect("severity then rule");
            assert!(
                ["PASS", "WARN", "ERROR", "UNKNOWN"].contains(&sev),
                "{line}"
            );
            let rule = rest.split_whitespace().next().expect("rule");
            assert!(
                rule.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '.' || c == '-'),
                "{line}"
            );
        }
    }

    use super::*;

    const ALL: [Severity; 4] = [
        Severity::Pass,
        Severity::Warn,
        Severity::Unknown,
        Severity::Error,
    ];

    #[test]
    fn severity_orders_pass_below_everything_that_matters() {
        assert!(Severity::Pass < Severity::Warn);
        assert!(Severity::Warn < Severity::Unknown);
        assert!(Severity::Unknown < Severity::Error);
    }

    /// The ordering and the blocking predicate must agree: once a severity
    /// blocks, every severity above it must block too.
    ///
    /// This is the invariant whose violation let a report with a blocking
    /// unknown call itself ready. Asserted over the whole lattice rather than
    /// for the one pair that broke, because the next reordering will pick a
    /// different pair.
    #[test]
    fn blocking_is_upward_closed_in_the_severity_order() {
        for lower in ALL {
            for higher in ALL {
                if lower <= higher && lower.blocks_readiness() {
                    assert!(
                        higher.blocks_readiness(),
                        "{higher:?} ranks at or above blocking {lower:?} but does not block"
                    );
                }
            }
        }
    }

    /// `is_ready` must agree with the diagnostics present, whatever else is in
    /// the report alongside them.
    #[test]
    fn a_blocking_diagnostic_blocks_regardless_of_company() {
        for blocking in ALL.iter().filter(|s| s.blocks_readiness()) {
            let mut report = Report::default();
            report.push(Diagnostic::pass("p", "fine"));
            report.push(Diagnostic::warn("w", "f", "eh"));
            report.push(Diagnostic::new(*blocking, "b", Some("f".into()), "blocks"));
            assert!(
                !report.is_ready(),
                "{blocking:?} must block even beside a warning"
            );
        }
    }

    /// Law 15, encoded: unknown is neither pass nor failure, but it does block.
    #[test]
    fn unknown_blocks_readiness_without_being_an_error() {
        assert!(Severity::Unknown.blocks_readiness());
        assert_ne!(Severity::Unknown, Severity::Error);
        assert!(!Severity::Warn.blocks_readiness());
        assert!(!Severity::Pass.blocks_readiness());
    }

    #[test]
    fn an_empty_report_is_well_formed() {
        let report = Report::default();
        assert_eq!(report.worst(), Severity::Pass);
        assert!(report.is_ready());
    }

    #[test]
    fn a_warning_does_not_block_but_an_unknown_does() {
        let mut report = Report::default();
        report.push(Diagnostic::warn("r", "f", "m"));
        assert!(report.is_ready());

        report.push(Diagnostic::unknown("r2", "f", "could not ask"));
        assert!(
            !report.is_ready(),
            "an unknown blocks even beside a warning"
        );
        // Unknown outranks Warn, so it is also the worst severity present.
        assert_eq!(report.worst(), Severity::Unknown);
    }

    #[test]
    fn errors_dominate() {
        let mut report = Report::default();
        report.push(Diagnostic::pass("a", "fine"));
        report.push(Diagnostic::warn("b", "f", "eh"));
        report.push(Diagnostic::error("c", "f", "bad"));
        assert_eq!(report.worst(), Severity::Error);
        assert_eq!(report.count(Severity::Error), 1);
        assert_eq!(report.verdict_line(), "NOT READY");
    }

    /// The verdict must not claim readiness Phase 1 has not established.
    #[test]
    fn the_ready_verdict_names_what_it_excludes() {
        let verdict = Report::default().verdict_line();
        assert!(!verdict.starts_with("READY"), "got {verdict:?}");
        assert!(verdict.contains("Preflight"), "got {verdict:?}");
    }

    /// A scope note states coverage; it must never decide the verdict.
    ///
    /// The first version of `war check` emitted its Phase 1 scope note as an
    /// `Unknown` diagnostic, so a corpus with zero defects still reported NOT
    /// READY. A gate whose answer never changes is not a gate.
    #[test]
    fn notes_do_not_affect_readiness() {
        let mut report = Report::default();
        report.note("gate execution is not implemented");
        report.note("Preflight is not implemented");
        assert!(report.is_ready(), "notes must not block");
        assert_eq!(report.worst(), Severity::Pass);
        assert_eq!(report.notes.len(), 2);
    }
}
