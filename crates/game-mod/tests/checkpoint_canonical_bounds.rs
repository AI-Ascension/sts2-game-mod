// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use sts2_game_mod::{
    CHECKPOINT_CAPTURE_MAX_BYTES, CanonicalError, CanonicalValue, parse_canonical_text,
    to_canonical_bytes,
};

#[test]
fn byte_limit_includes_quotes_and_raw_input_whitespace() -> Result<(), CanonicalError> {
    assert_eq!(CHECKPOINT_CAPTURE_MAX_BYTES, 16 * 1024 * 1024);
    let value = CanonicalValue::Text("x".repeat(CHECKPOINT_CAPTURE_MAX_BYTES - 2));
    let bytes = to_canonical_bytes(&value)?;
    assert_eq!(bytes.len(), CHECKPOINT_CAPTURE_MAX_BYTES);
    let mut text = String::from_utf8(bytes).map_err(|_| CanonicalError::InvalidUtf8)?;
    assert_eq!(parse_canonical_text(&text)?, value);
    text.push(' ');
    assert_eq!(
        parse_canonical_text(&text),
        Err(CanonicalError::PayloadTooLarge)
    );
    assert_eq!(
        to_canonical_bytes(&CanonicalValue::Text(
            "x".repeat(CHECKPOINT_CAPTURE_MAX_BYTES - 1)
        )),
        Err(CanonicalError::PayloadTooLarge)
    );
    Ok(())
}

#[test]
fn escaping_and_utf8_are_bounded_by_encoded_bytes() -> Result<(), CanonicalError> {
    for (character, width) in [('\u{0001}', 6), ('\n', 2), ('"', 2), ('\\', 2), ('é', 2)] {
        let count = (CHECKPOINT_CAPTURE_MAX_BYTES - 2) / width;
        let padding = (CHECKPOINT_CAPTURE_MAX_BYTES - 2) % width;
        let mut text = character.to_string().repeat(count);
        text.push_str(&"x".repeat(padding));
        let bytes = to_canonical_bytes(&CanonicalValue::Text(text.clone()))?;
        assert_eq!(bytes.len(), CHECKPOINT_CAPTURE_MAX_BYTES);
        text.push('x');
        assert_eq!(
            to_canonical_bytes(&CanonicalValue::Text(text)),
            Err(CanonicalError::PayloadTooLarge)
        );
    }
    Ok(())
}

#[test]
fn keys_and_delimiters_share_the_output_budget() -> Result<(), CanonicalError> {
    let key = "k".repeat(CHECKPOINT_CAPTURE_MAX_BYTES - 9);
    let value = CanonicalValue::Object(BTreeMap::from([(key, CanonicalValue::Null)]));
    let bytes = to_canonical_bytes(&value)?;
    assert_eq!(bytes.len(), CHECKPOINT_CAPTURE_MAX_BYTES);
    assert_eq!(
        to_canonical_bytes(&CanonicalValue::Array(vec![value])),
        Err(CanonicalError::PayloadTooLarge)
    );
    Ok(())
}

#[test]
fn object_depth_uses_pinned_64_container_boundary() -> Result<(), CanonicalError> {
    let at_limit = format!("{}0{}", "{\"k\":".repeat(64), "}".repeat(64));
    let value = parse_canonical_text(&at_limit)?;
    assert_eq!(to_canonical_bytes(&value)?, at_limit.as_bytes());
    assert_eq!(
        to_canonical_bytes(&CanonicalValue::Object(BTreeMap::from([(
            "k".to_owned(),
            value
        )]))),
        Err(CanonicalError::DepthExceeded)
    );
    let beyond = format!("{{\"k\":{at_limit}}}");
    assert_eq!(
        parse_canonical_text(&beyond),
        Err(CanonicalError::DepthExceeded)
    );
    let hostile = format!("{}0{}", "{\"k\":".repeat(20_000), "}".repeat(20_000));
    assert_eq!(
        parse_canonical_text(&hostile),
        Err(CanonicalError::DepthExceeded)
    );
    Ok(())
}

#[test]
fn hostile_owned_depth_returns_error_without_traversing_entire_tree() {
    let mut value = (0..20_000).fold(CanonicalValue::Null, |value, _| {
        CanonicalValue::Array(vec![value])
    });
    let result = to_canonical_bytes(&value);
    // The caller also avoids recursively dropping its hostile input.
    while let CanonicalValue::Array(mut items) = value {
        value = items.remove(0);
    }
    assert_eq!(result, Err(CanonicalError::DepthExceeded));
}

#[test]
fn key_grammar_checks_decoded_keys_and_typed_values() -> Result<(), CanonicalError> {
    for key in [
        "", "Value", "my-key", "1key", "_key", "aB", "a b", "a\n", "a.",
    ] {
        let text = serde_json::to_string(&BTreeMap::from([(key, 1)]))
            .map_err(|_| CanonicalError::InvalidUtf8)?;
        assert_eq!(
            parse_canonical_text(&text),
            Err(CanonicalError::InvalidKey),
            "{key}"
        );
        let value = CanonicalValue::Object(BTreeMap::from([(
            key.to_owned(),
            CanonicalValue::Integer(1),
        )]));
        assert_eq!(
            to_canonical_bytes(&value),
            Err(CanonicalError::InvalidKey),
            "{key}"
        );
    }
    let accepted = r#"{"\u0061_0":1}"#;
    assert_eq!(
        to_canonical_bytes(&parse_canonical_text(accepted)?)?,
        br#"{"a_0":1}"#
    );
    assert_eq!(
        parse_canonical_text(r#"{"\u0041":1}"#),
        Err(CanonicalError::InvalidKey)
    );
    assert_eq!(
        parse_canonical_text(r#"{"a":1,"\u0061":2}"#),
        Err(CanonicalError::DuplicateKey)
    );
    Ok(())
}

#[test]
fn typed_integer_extremes_and_unpaired_surrogates_are_rejected() -> Result<(), CanonicalError> {
    for integer in [
        i64::MIN,
        i64::MAX,
        -9_007_199_254_740_992,
        9_007_199_254_740_992,
    ] {
        assert_eq!(
            to_canonical_bytes(&CanonicalValue::Integer(integer)),
            Err(CanonicalError::UnsafeInteger)
        );
    }
    assert_eq!(
        to_canonical_bytes(&CanonicalValue::Integer(-9_007_199_254_740_991))?,
        b"-9007199254740991"
    );
    for text in [r#""\ud800""#, r#""\udc00""#, r#""\ud800\u0061""#] {
        assert_eq!(
            parse_canonical_text(text),
            Err(CanonicalError::InvalidUnicodeEscape { offset: 1 })
        );
    }
    Ok(())
}
