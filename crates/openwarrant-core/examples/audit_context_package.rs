// SPDX-License-Identifier: Apache-2.0
//! Minimal independent SDK consumer. Reads a JSON map of package path -> byte array.
use openwarrant_core::document::packet::{PackageLimits, check_package};
use std::io::{self, Read};
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    const INPUT_LIMIT: u64 = 64 * 1024 * 1024;
    let mut bytes = Vec::new();
    io::stdin()
        .take(INPUT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > INPUT_LIMIT {
        return Err("resource-limit: input exceeds 64 MiB".into());
    }
    let Files(files) =
        serde_json::from_slice::<Files>(&bytes).map_err(|e| format!("package-invalid: {e}"))?;
    let checked = check_package(
        files,
        PackageLimits {
            total_bytes: 64 * 1024 * 1024,
            ..PackageLimits::default()
        },
    )
    .map_err(|e| format!("{}: {}", e.code, e.message))?;
    println!(
        "{}",
        serde_json::json!({"schema":"oh.war/package-audit-example/1","local_integrity":true,"local_semantic_coverage":checked.semantic_coverage_established(),"root_digest":checked.manifest.root_digest,"basis_digest":checked.packet.basis_digest,"compiler":checked.packet.compiler,"readiness":checked.packet.readiness.value})
    );
    Ok(())
}

struct Files(std::collections::BTreeMap<String, Vec<u8>>);
impl<'de> serde::Deserialize<'de> for Files {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Files;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("unique package file map")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Files, M::Error> {
                let mut files = std::collections::BTreeMap::new();
                while let Some((key, value)) = map.next_entry::<String, Vec<u8>>()? {
                    if files.len() >= 4099 || files.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom(
                            "duplicate or excessive package files",
                        ));
                    }
                }
                Ok(Files(files))
            }
        }
        d.deserialize_map(Visitor)
    }
}
