# Refuse Warrant source-validation downgrade

Before this fix, import and re-export invoked source reconstruction only when
`__ow_archive__/basis.json` was present. An archive with a `war://` subject could
omit that descriptor and pass as generic inert transport. A regression test
reproduced successful import where a missing-basis refusal was required.

Import and re-export now require source reconstruction for every `war://` subject,
as well as any archive containing a basis descriptor. Synthetic non-Warrant
transport fixtures retain their existing bounded byte-preservation behavior.
The refusal happens before a destination or re-export output is created.

All fifteen preservation integration tests passed on Rust 1.97.1. Red and green
logs are retained beside this note. This closes a source-validation bypass;
it does not authenticate source history or grant authority.

The provider source-complete fixture extension passed hosted CI run 35365827192
and merged through Knowledge Fabric PR #4 as
`2bbcbaf6435bfa29261d9787b9a1afa224b911e6` (tested head `bcb7accc`). Its retained
log is `kf-hosted-bcb7accc.log.gz`. Provider test success does not settle the
remaining real-runtime resolver, format adoption or independent qualification.

All-target CLI Clippy passed with warnings denied.
