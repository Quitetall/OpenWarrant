// SPDX-License-Identifier: Apache-2.0
//! Print candidate record wire schemas. Semantics and authenticity require SDK checks.
fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&openwarrant_core::document::records::record_schemas())
            .expect("schemas serialize")
    );
}
