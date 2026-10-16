// SPDX-License-Identifier: Apache-2.0
//! Print the opt-in compact record candidate; this does not activate a pack.
fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&openwarrant_core::compact_verification_record::schema())
            .expect("candidate schema serializes")
    );
}
