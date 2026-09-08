// SPDX-License-Identifier: MIT

use serde::de::{self, Deserialize, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

struct Unique(Value);

impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = Unique;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("JSON without duplicate object keys")
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(value)))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(value.into())))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(value.into())))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Unique, E> {
                Number::from_f64(value)
                    .map(|number| Unique(Value::Number(number)))
                    .ok_or_else(|| E::custom("nonfinite JSON number"))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(value.to_owned())))
            }

            fn visit_none<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }

            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
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

pub fn parse_json(data: &[u8], label: &str) -> Result<Value, String> {
    serde_json::from_slice::<Unique>(data)
        .map(|value| value.0)
        .map_err(|error| format!("{label} is not valid UTF-8 JSON: {error}"))
}

pub fn parse_object(data: &[u8], label: &str) -> Result<Map<String, Value>, String> {
    parse_json(data, label)?
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{label} must be a JSON object"))
}
