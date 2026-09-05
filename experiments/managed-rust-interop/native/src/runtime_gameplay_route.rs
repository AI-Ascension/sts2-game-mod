// SPDX-License-Identifier: MIT

use serde::de::{self, Deserialize, Deserializer, IgnoredAny, MapAccess, Visitor};
use std::collections::BTreeSet;
use std::fmt;

pub(super) fn expected_kind(method: &str, path: &str) -> Option<&'static str> {
    match (method, path) {
        ("GET", "/api/v3/runtime/state") => Some("state_request"),
        ("GET", "/api/v3/runtime/legal-actions") => Some("legal_actions_request"),
        ("POST", "/api/v3/runtime/action") => Some("dispatch_action_request"),
        ("POST", "/api/v3/runtime/wait") => Some("wait_request"),
        ("GET", "/api/v3/runtime/reobserve") => Some("reobserve_request"),
        ("POST", "/api/v3/runtime/recover") => Some("recover_request"),
        _ => None,
    }
}

pub(super) fn body_matches(body: &[u8], expected: &str) -> bool {
    serde_json::from_slice::<EnvelopeKind>(body).is_ok_and(|value| value.0 == expected)
}

struct EnvelopeKind(String);

impl<'de> Deserialize<'de> for EnvelopeKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(KindVisitor)
    }
}

struct KindVisitor;

impl<'de> Visitor<'de> for KindVisitor {
    type Value = EnvelopeKind;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a request object with a unique string kind")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let mut keys = BTreeSet::new();
        let mut kind = None;
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom("duplicate request field"));
            }
            if key == "kind" {
                kind = Some(map.next_value::<String>()?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        kind.map(EnvelopeKind)
            .ok_or_else(|| de::Error::missing_field("kind"))
    }
}
