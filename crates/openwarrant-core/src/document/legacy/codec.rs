// SPDX-License-Identifier: Apache-2.0
//! Check wire shape and resource counts before allocating retained byte arrays.
use super::Limits;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::{collections::BTreeSet, fmt};
pub(super) fn preflight(bytes: &[u8], limits: Limits) -> Result<(), serde_json::Error> {
    // JSON escapes can encode one byte with six wire bytes. Bound every raw
    // string token before serde allocates its decoded string or scratch buffer.
    let max_token = limits.total_bytes.min(4096).saturating_mul(6);
    let mut quoted = false;
    let mut escaped = false;
    let mut length = 0usize;
    for &byte in bytes {
        if !quoted {
            if byte == b'"' {
                quoted = true;
                length = 0;
            }
        } else if !escaped && byte == b'"' {
            quoted = false;
        } else {
            length += 1;
            if length > max_token {
                return Err(de::Error::custom("resource-limit: JSON string token"));
            }
            if escaped {
                escaped = false;
            } else {
                escaped = byte == b'\\';
            }
        }
    }
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    Root(limits).deserialize(&mut decoder)?;
    decoder.end()
}
struct Root(Limits);
impl<'de> DeserializeSeed<'de> for Root {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for Root {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("preservation object")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom("duplicate preservation field"));
            }
            match key.as_str() {
                "files" => map.next_value_seed(Files(self.0))?,
                "inventory" => map.next_value_seed(Inventory(self.0))?,
                "schema" | "source_dialect" | "adapter_version" | "entry" => {
                    let value = map.next_value::<String>()?;
                    if value.len() > 4096 {
                        return Err(de::Error::custom("resource-limit: metadata"));
                    }
                }
                _ => return Err(de::Error::custom("unknown preservation field")),
            }
        }
        Ok(())
    }
}
struct Files(Limits);
impl<'de> DeserializeSeed<'de> for Files {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for Files {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("file byte map")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut seen = BTreeSet::new();
        let mut remaining = self.0.total_bytes;
        while let Some(path) = map.next_key::<String>()? {
            if path.len() > 4096 || seen.len() >= self.0.files {
                return Err(de::Error::custom("resource-limit: file map"));
            }
            remaining = remaining
                .checked_sub(path.len())
                .ok_or_else(|| de::Error::custom("resource-limit: total bytes"))?;
            if !seen.insert(path) {
                return Err(de::Error::custom("duplicate file path"));
            }
            let size = map.next_value_seed(Bytes(self.0.file_bytes.min(remaining)))?;
            remaining -= size;
        }
        Ok(())
    }
}
struct Bytes(usize);
impl<'de> DeserializeSeed<'de> for Bytes {
    type Value = usize;
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<usize, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for Bytes {
    type Value = usize;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("bounded byte array")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<usize, A::Error> {
        let mut count = 0;
        while seq.next_element::<u8>()?.is_some() {
            if count >= self.0 {
                return Err(de::Error::custom("resource-limit: file bytes"));
            }
            count += 1;
        }
        Ok(count)
    }
}
struct Inventory(Limits);
impl<'de> DeserializeSeed<'de> for Inventory {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for Inventory {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("bounded inventory")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let mut count = 0;
        while seq.next_element_seed(Entry)?.is_some() {
            if count >= self.0.files {
                return Err(de::Error::custom("resource-limit: inventory count"));
            }
            count += 1;
        }
        Ok(())
    }
}
struct Entry;
impl<'de> DeserializeSeed<'de> for Entry {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for Entry {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("inventory entry object")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom("duplicate inventory field"));
            }
            match key.as_str() {
                "path" | "digest" => {
                    let value = map.next_value::<String>()?;
                    if value.len() > 4096 {
                        return Err(de::Error::custom("resource-limit: inventory field"));
                    }
                }
                "bytes" => {
                    map.next_value::<usize>()?;
                }
                _ => return Err(de::Error::custom("unknown inventory field")),
            }
        }
        Ok(())
    }
}
