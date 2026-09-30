# Source-detached provider runtime observation

The real PostgreSQL/MinIO preservation test produced a signed disposable export,
then stopped its database fixtures. The built KF CLI read that export from an
empty working directory with an unusable database URL. It returned one retained
runtime receipt and one explicitly unmapped dispatch. An empty external trust
store refused with nonzero exit and no stdout.

`provider-offline-runtime-proof.json` records exact manifest/snapshot/output
identity, source revisions and test hash. The retained tarball contains the signed
fixture package, public fixture key, Warrant ID and observed projection. No private
key is retained. The public fixture key supports reproducibility only; it is not a
project trust root and must never be installed as one.

The native runtime receipt is synthetic and labeled `fixture://no-model-called`.
This is real database/storage/CLI preservation evidence, not agent execution or
Warrant qualification. CLI integration and native runtime semantics remain open.
