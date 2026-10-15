# Current LAMU provider replay

The existing OpenWarrant adapter integration driver passed on an isolated, clean
LAMU checkout. Exact provider, consumer, SDK pin, source hashes and process outputs
are retained in `receipt.json` (also compressed without changing its bytes).

This exercises the local Rust provider profile, not LAMU service or database
operation. No model calls, shared service changes, human acceptance, independent
verdict or Phase 2 exit is claimed. Readiness remains blocked as expected.

The raw driver receipt contains a static older limitation stating that native
macOS Phase 1 evidence is pending. That text was not edited. New Linux and macOS
observations exist separately in `../../../OW-WAR-0085/implementation/native-20261015/`,
including their exact tested source identities. Neither observation grants
independent phase acceptance.

The foreign LAMU working tree was not modified. Temporary replay checkout and
scratch are removed after terminal success; source remains at the recorded
commit in the original repository.
