// SPDX-License-Identifier: Apache-2.0
//! Compilation from source atoms to canonical WAR IR and projections.
//!
//! Governed by WAR SAS v0.1.0-draft.1. Section references cite that document.
//!
//! # Status
//!
//! Canonicalization and domain-separated digesting are implemented and pinned
//! against the official RFC 8785 vectors in `conformance/rfc8785/`. The
//! implementation was selected by OW-ADR-0001 on measured conformance, not on
//! reputation.
//!
//! Still to come: lowering a Compilation Basis to the §63 IR, and the
//! projections of §17.5.

#![forbid(unsafe_code)]

pub mod adr_overview;
pub mod canonical;
pub mod corpus_status;
pub mod current;
pub mod digest;
pub mod dispatch;
pub mod dispatch_bundle;
pub mod history;
pub mod ir;
pub mod lower;
pub mod preservation;
pub mod render;
pub mod warrant_overview;

pub use adr_overview::render as render_adr_overview;
pub use canonical::{
    CanonicalError, preimage_bytes, sha256_digest, to_canonical_bytes, to_canonical_string,
};
pub use corpus_status::{
    canonical_json as corpus_status_json, render_html as render_corpus_status_html,
    render_markdown as render_corpus_status, render_platform as render_corpus_status_platform,
};
pub use current::{Corpus as CurrentCorpus, ROLE_SECTIONS, render as render_current, role_row};
pub use digest::{DigestDomain, sha256_hex};
pub use dispatch::{
    DispatchError, DispatchInputs, canonical_json as dispatch_json, compile_dispatch,
    required_normative_sources,
};
pub use history::render as render_history;
pub use ir::{API_VERSION, KIND, SCHEMA_PACK_ID, SCHEMA_PACK_VERSION, SourceScope, WarIr};
pub use lower::{AtomSource, CompilationBasis, SasPin, ScopeSource, lower};
pub use render::{ChildRef, View, canonical_json, full_warrant};
pub use warrant_overview::{WarrantSummary, render as render_warrant_overview};
