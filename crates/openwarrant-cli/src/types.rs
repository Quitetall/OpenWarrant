// SPDX-License-Identifier: Apache-2.0
//! Every development document is a type (OW-WAR-0148 M18; decisions 25-27).
//!
//! - **Built-in types.** The roadmap, the specification and ADRs are types
//!   whose records are read from the stores that predate typed records
//!   (`encoding = "roadmap" | "sas" | "adr"`), and their rules are gated on
//!   the capabilities those types select. This build ships their definitions
//!   ([`builtins`]); a program's own `profiles/<name>.toml` replaces one.
//!
//! Nothing here reads a store: the readers are the ones that always read it
//! (`roadmap_cmd`, `sas`, the ADR loader).

use openwarrant_core::projection::Store;
use openwarrant_core::{Capabilities, Capability};

use crate::repo::Repository;

/// The types this build ships, admitted where a program declares none of
/// the same name (or, for a store's type, none reading the same store).
const BUILTIN: [(&str, &str); 3] = [
    (
        "(built in)/roadmap.toml",
        include_str!("../../../profiles/roadmap.toml"),
    ),
    (
        "(built in)/spec.toml",
        include_str!("../../../profiles/spec.toml"),
    ),
    (
        "(built in)/adr.toml",
        include_str!("../../../profiles/adr.toml"),
    ),
];

/// The built-in types, as `(file, bytes)` for
/// [`openwarrant_core::ProfileRegistry::with_builtins`].
pub fn builtins() -> impl Iterator<Item = (&'static str, &'static [u8])> {
    BUILTIN.iter().map(|(f, b)| (*f, b.as_bytes()))
}

/// The capabilities of the type that reads `store`: what its rules are
/// gated on.
#[must_use]
pub fn caps(repo: &Repository, store: Store) -> Capabilities {
    repo.profiles.store_capabilities(store)
}

/// Whether the type that reads `store` selects `cap`.
#[must_use]
pub fn has(repo: &Repository, store: Store, cap: Capability) -> bool {
    caps(repo, store).has(cap)
}
