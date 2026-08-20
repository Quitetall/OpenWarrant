// SPDX-License-Identifier: AGPL-3.0-or-later
//! One macro for the SAS's closed vocabularies.
//!
//! The specification is largely a set of fixed word lists — §33.2's nine context
//! roles, §33.3's six trust classes, §35.1's eight rationale node classes, and so
//! on. Each needs the same five things: the variants, an `ALL` in the
//! specification's order, a string form, a parse that names the alternatives when
//! it fails, and `Display`.
//!
//! Writing that by hand per vocabulary is how one list quietly ends up in a
//! different order from the SAS, or missing a term. `state.rs` and `gate_run.rs`
//! each grew a private copy of this macro before it was worth extracting; this is
//! the extraction, and new vocabularies use it.

/// Define a closed SAS vocabulary.
///
/// `ALL` is in declaration order, which must be the specification's order — the
/// per-module "matches the SAS" tests compare against it.
///
/// `$err` is an `ident` rather than a `path` deliberately: `<$err>::Variant` in
/// expression position is an unstable qualified path, so the error type has to be
/// nameable as a plain identifier in the calling module's scope.
macro_rules! vocabulary {
    (
        $(#[$meta:meta])*
        $name:ident, $label:literal, $err:ident, { $($variant:ident => $text:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            /// Every term, in the specification's order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }

            /// The alternatives, for an error message that helps.
            #[must_use]
            pub fn known() -> String {
                Self::ALL.iter().map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
            }
        }

        impl std::str::FromStr for $name {
            type Err = $err;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::ALL.iter().copied().find(|v| v.as_str() == s).ok_or_else(|| {
                    $err::UnknownTerm {
                        vocabulary: $label,
                        found: s.to_owned(),
                        known: Self::known(),
                    }
                })
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

pub(crate) use vocabulary;
