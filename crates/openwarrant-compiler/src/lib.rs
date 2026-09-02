// SPDX-License-Identifier: AGPL-3.0-or-later
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
pub mod digest;
pub mod dispatch;
pub mod ir;
pub mod lower;
pub mod render;
pub mod warrant_overview;

pub use adr_overview::render as render_adr_overview;
pub use canonical::{
    CanonicalError, preimage_bytes, sha256_digest, to_canonical_bytes, to_canonical_string,
};
pub use corpus_status::{
    canonical_json as corpus_status_json, render_html as render_corpus_status_html,
    render_markdown as render_corpus_status,
};
pub use digest::{DigestDomain, sha256_hex};
pub use dispatch::{
    DispatchError, DispatchInputs, canonical_json as dispatch_json, compile_dispatch,
    required_normative_sources,
};
pub use ir::{API_VERSION, KIND, SCHEMA_PACK_ID, SCHEMA_PACK_VERSION, SourceScope, WarIr};
pub use lower::{AtomSource, CompilationBasis, ScopeSource, lower};
pub use render::{ChildRef, View, canonical_json, full_warrant};
pub use warrant_overview::{WarrantSummary, render as render_warrant_overview};
