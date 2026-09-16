// SPDX-License-Identifier: Apache-2.0
//! Bound JSON before typed decoding; never discard duplicate object members.
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::{fmt, io::Write};

pub(super) fn decode(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    let value = decode_value(bytes)?;
    if !value.is_object() {
        return Err(de::Error::custom("request must be an object"));
    }
    Ok(value)
}
pub(crate) fn decode_value(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let mut remaining = 65_536usize;
    let value = Node {
        remaining: &mut remaining,
        depth: 0,
    }
    .deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(value)
}
struct Node<'a> {
    remaining: &'a mut usize,
    depth: usize,
}
impl<'de> DeserializeSeed<'de> for Node<'_> {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<Value, D::Error> {
        if self.depth > 64 || *self.remaining == 0 {
            return Err(de::Error::custom("resource-limit: JSON nodes or depth"));
        }
        *self.remaining -= 1;
        decoder.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Node<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("bounded JSON value")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom("nonfinite JSON number"))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut out = Vec::new();
        while let Some(value) = seq.next_element_seed(Node {
            remaining: self.remaining,
            depth: self.depth + 1,
        })? {
            out.push(value);
        }
        Ok(Value::Array(out))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut out = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if out.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON field"));
            }
            out.insert(
                key,
                map.next_value_seed(Node {
                    remaining: self.remaining,
                    depth: self.depth + 1,
                })?,
            );
        }
        Ok(Value::Object(out))
    }
}
// Serde also accepts positional arrays for named structs. Compare with the
// typed object's serialized shape so that only documented object forms pass.
pub(crate) fn shape(input: &Value, typed: &Value) -> Result<(), &'static str> {
    match (input, typed) {
        (Value::Array(_), Value::Object(_)) => {
            Err("named fields require an object, not a positional array")
        }
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in a {
                match b.get(key) {
                    Some(other) => shape(value, other)?,
                    // Typed decoding has already rejected unknown names. SDK
                    // optional fields may serialize None by omitting the key.
                    None if value.is_null() => {}
                    None => return Err("unknown JSON field"),
                }
            }
            Ok(())
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Err("array members must not collapse during decoding");
            }
            for (value, other) in a.iter().zip(b) {
                shape(value, other)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
pub(super) fn encode(value: &Value, limit: usize) -> Result<Vec<u8>, serde_json::Error> {
    struct Bounded {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                return Err(std::io::Error::other("output limit"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Bounded {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut writer, value)?;
    Ok(writer.bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_writer_refuses_before_crossing_limit() {
        assert!(encode(&serde_json::json!({"x":"abcdef"}), 5).is_err());
        assert_eq!(encode(&serde_json::json!(null), 4).unwrap(), b"null");
    }
}
