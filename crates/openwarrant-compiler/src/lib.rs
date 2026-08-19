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
pub mod digest;
pub mod ir;
pub mod lower;
pub mod render;

pub use adr_overview::render as render_adr_overview;
pub use canonical::{
    CanonicalError, preimage_bytes, sha256_digest, to_canonical_bytes, to_canonical_string,
};
pub use digest::{DigestDomain, sha256_hex};
pub use ir::{API_VERSION, KIND, SCHEMA_PACK_ID, SCHEMA_PACK_VERSION, WarIr};
pub use lower::{AtomSource, CompilationBasis, lower};
pub use render::{View, canonical_json, full_warrant};
