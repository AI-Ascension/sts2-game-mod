// SPDX-License-Identifier: MIT

use serde::de::{Deserialize, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

struct Unique(Value);

impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON without duplicate object keys")
            }
            fn visit_bool<E: Error>(self, value: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(value)))
            }
            fn visit_i64<E: Error>(self, value: i64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(value.into())))
            }
            fn visit_u64<E: Error>(self, value: u64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(value.into())))
            }
            fn visit_f64<E: Error>(self, value: f64) -> Result<Unique, E> {
                Number::from_f64(value)
                    .map(|number| Unique(Value::Number(number)))
                    .ok_or_else(|| E::custom("nonfinite JSON number"))
            }
            fn visit_str<E: Error>(self, value: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(value.into())))
            }
            fn visit_none<E: Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_unit<E: Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(Unique(value)) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
                let mut values = Map::new();
                while let Some((key, Unique(value))) = map.next_entry::<String, Unique>()? {
                    if values.insert(key, value).is_some() {
                        return Err(A::Error::custom("duplicate JSON key"));
                    }
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(JsonVisitor)
    }
}

pub(super) fn parse(bytes: &[u8]) -> Result<Value, String> {
    if bytes.is_empty() || bytes.len() > 1024 * 1024 {
        return Err("settings byte bound exceeded".into());
    }
    serde_json::from_slice::<Unique>(bytes)
        .map(|value| value.0)
        .map_err(|_| "invalid or duplicate-key settings JSON".into())
}
