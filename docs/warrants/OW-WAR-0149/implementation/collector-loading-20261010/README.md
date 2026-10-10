# Collector execution loading observations

The read-only loader checks the actual execution account before accepting store
configuration, requires its configured execution UID to match, verifies signed
enrollment through OpenSSH, rereads authority after authentication and requires
fresh authority for every use. No activation or launch is performed.

Rust 1.97.1 observations:

- RED: missing module, E0432.
- First implementation compile: borrowed argument mismatch, E0308; corrected.
- Namespace fixture compile: unsupported process credential functions, E0425;
  corrected by executing a separate credential-dropped process through setpriv.
- Default regressions: 3 loading controls, 4 real-crypto/SDK controls and one
  pre-existing authority-transition CLI regression PASS.
- Two namespace test entries are explicitly ignored in the default run; the
  complete roundtrip was run separately and PASS. Operator/executor use actual
  namespace UIDs 0/1; worker capabilities are zero. Authority write is denied,
  valid enrollment is accepted, configured UID mismatch is refused, and a real
  software-signed authority transition revokes an already loaded enrollment.
- Namespace fixture removes its files. No host account is created. Software
  keys stay in memory; no physical human acceptance or key custody is claimed.

All-target/all-feature CLI Clippy with warnings denied: PASS.

No positive host-account/operator qualification, enrollment activation, native
collector wiring or atomic launch fencing is established. The trusted host must
select the authority path independently of agent input and authenticate the
acting collector; names alone are not caller authentication. Operator rollback
remains a residual of the reference store. Test sources and outcomes are retained
here; earlier compile failures remain visible rather than rewritten as passes.
