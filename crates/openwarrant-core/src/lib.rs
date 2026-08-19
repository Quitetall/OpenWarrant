// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure domain types and validators for Work Authorization Records.
//!
//! Governed by `WAR_Software_Architecture_Specification.md` v0.1.0-draft.1
//! (sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`).
//! Section references in this crate cite that document.
//!
//! This crate holds no I/O (SAS §79.1). Everything here is a value, a parse, or
//! a validation over values.

#![forbid(unsafe_code)]

pub mod config;
pub mod identity;

pub use config::{ConfigError, GeneratedPolicy, Namespace, Paths, RepositoryConfig};
pub use identity::{IdentityError, LocalAlias, WarUuid};
