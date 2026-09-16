# OW-WAR-0085 implementation

Current-host Phase 1 acceptance is implemented and passes on Linux. The full phase exit remains blocked by missing native macOS evidence. No independent assurance disposition, human acceptance, or Verified status is claimed.

The combined SDK probe covers parser/footer, author/edit, sources, conditions, packet integrity, records and legacy reading. The acceptance runner additionally executes public core/CLI tests, fixed skill artifacts, context-entry preservation, stage graph semantics and skill structure checks. It retains exact case inventories and build inputs, detects changes during execution, and binds the actual Cargo executable.

Independent specification and standards review identified missing build inputs and possible stale CLI selection. Both are fixed with regression tests. Standards recheck passed. The false-maturity plant now changes the existing state to `verified` and observes `source-invalid`; duplicate-state refusal is a separate plant. A legacy plan proposal check alone does not reject milestone cycles; the public milestone parser independently exercises that refusal.

The Linux receipt records exact pre-run and post-run inputs. Later documentation and generated progress updates are not retroactively included in that receipt. The receipt retains all command logs and reports phase_exit_established=false. Clippy passed with warnings denied.

Remaining: repository gate, native macOS run with matching inputs, and final phase review. Independent implementation and integration preparation may continue while platform evidence is missing.

The initial repository gate failed on an existing OW68 delivery digest for
`war-spec/SKILL.md`, causing four additional baseline-dependent plants to fail.
The historical skill file is restored unchanged. New SDK guidance remains in the
OpenWarrant router and its separate reference; no historical pin or signature was
changed. The failed gate log is retained.

LAMU reviewed implementation commit `96b9e9b` through the local/free model and
returned PASS WITH NITS. The executable-selection concern was retracted by the
reviewer itself: missing identity fails closed. Every fixed artifact expectation
is positively checked; one removed constraint per artifact supplies the bounded
negative control. The repository-relative script location is intentional. Large
receipt/generated data was explicitly omitted from this bounded source review.

The corrected full repository gate passed all 14 steps and 308/308 planted controls
on `038f32d`. The newer Linux acceptance receipt passed after the historical skill
was restored. Native macOS Phase 1 evidence remains missing, so full phase exit
stays blocked. LAMU reviewed the correction commit PASS WITH NITS: receipt names
are explicit caller output paths, the referenced file exists, and Linux success
does not justify marking the entire phase complete or Verified.
