// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compilation from source atoms to canonical WAR IR and projections.
//!
//! Governed by WAR SAS v0.1.0-draft.1. Section references cite that document.
//!
//! # Status
//!
//! OW-WAR-0001 establishes this crate; OW-WAR-0003 and OW-WAR-0004 fill it. What
//! exists today is the interface from §81 and the digest-domain vocabulary from
//! §65, which are the two things later work must not be free to redefine
//! casually.
//!
//! The canonicalization implementation (RFC 8785, §65.2) is deliberately ABSENT.
//! It decides the exact bytes every cross-system digest is computed over, so the
//! library choice binds the wire format and requires an implementation ADR first
//! (§80). Guessing now and correcting later would silently invalidate every
//! digest minted in between.

#![forbid(unsafe_code)]

pub mod digest;

pub use digest::DigestDomain;
