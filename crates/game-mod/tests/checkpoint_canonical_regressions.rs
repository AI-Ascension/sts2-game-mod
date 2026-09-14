// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::error::Error;

use sts2_game_mod::{
    CANONICAL_MAX_DEPTH, CANONICAL_MAX_SAFE_INTEGER, CanonicalError, CanonicalValue,
    parse_canonical_text, to_canonical_bytes,
};

#[test]
fn arrays_nulls_and_booleans_round_trip() -> Result<(), Box<dyn Error>> {
    let text = r#"[null,true,false,[1,2],{}]"#;
    let expected = CanonicalValue::Array(vec![
        CanonicalValue::Null,
        CanonicalValue::Bool(true),
        CanonicalValue::Bool(false),
        CanonicalValue::Array(vec![CanonicalValue::Integer(1), CanonicalValue::Integer(2)]),
        CanonicalValue::Object(BTreeMap::new()),
    ]);
    assert_eq!(parse_canonical_text(text)?, expected);
    assert_eq!(
        to_canonical_bytes(&expected)?,
        b"[null,true,false,[1,2],{}]".to_vec()
    );
    Ok(())
}

#[test]
fn escaped_strings_decode_and_reencode_exactly() -> Result<(), Box<dyn Error>> {
    let raw = br#""\"\\\n\t\u0001""#;
    let value = parse_canonical_text(std::str::from_utf8(raw)?)?;
    assert_eq!(
        value,
        CanonicalValue::Text("\"\\\n\t\u{1}".to_owned()),
        "escapes did not decode to the expected characters"
    );
    assert_eq!(to_canonical_bytes(&value)?, raw.to_vec());
    Ok(())
}

#[test]
fn non_ascii_values_are_accepted_while_keys_are_rejected() -> Result<(), Box<dyn Error>> {
    let emoji = parse_canonical_text(r#""\ud83d\ude00""#)?;
    assert_eq!(emoji, CanonicalValue::Text("\u{1F600}".to_owned()));
    assert_eq!(
        to_canonical_bytes(&emoji)?,
        "\"\u{1F600}\"".as_bytes().to_vec()
    );

    assert_eq!(
        parse_canonical_text(r#"{"é":1}"#),
        Err(CanonicalError::NonAsciiKey)
    );
    assert_eq!(
        parse_canonical_text(r#"{"\u00e9":1}"#),
        Err(CanonicalError::NonAsciiKey)
    );

    let mut object = BTreeMap::new();
    object.insert("é".to_owned(), CanonicalValue::Integer(1));
    assert_eq!(
        to_canonical_bytes(&CanonicalValue::Object(object)),
        Err(CanonicalError::NonAsciiKey)
    );
    Ok(())
}

#[test]
fn safe_integer_endpoints_are_enforced() -> Result<(), Box<dyn Error>> {
    assert_eq!(CANONICAL_MAX_SAFE_INTEGER, 9_007_199_254_740_991);
    assert_eq!(
        parse_canonical_text("9007199254740991")?,
        CanonicalValue::Integer(9_007_199_254_740_991)
    );
    assert_eq!(
        parse_canonical_text("-9007199254740991")?,
        CanonicalValue::Integer(-9_007_199_254_740_991)
    );
    assert_eq!(
        parse_canonical_text("9007199254740992"),
        Err(CanonicalError::UnsafeInteger)
    );
    assert_eq!(
        to_canonical_bytes(&CanonicalValue::Integer(9_007_199_254_740_991))?,
        b"9007199254740991".to_vec()
    );
    assert_eq!(
        to_canonical_bytes(&CanonicalValue::Integer(9_007_199_254_740_992)),
        Err(CanonicalError::UnsafeInteger)
    );
    Ok(())
}

#[test]
fn escaped_duplicate_keys_are_rejected() {
    assert_eq!(
        parse_canonical_text(r#"{"a":1,"\u0061":2}"#),
        Err(CanonicalError::DuplicateKey)
    );
    assert_eq!(
        parse_canonical_text(r#"{"key":1,"key":2}"#),
        Err(CanonicalError::DuplicateKey)
    );
}

#[test]
fn depth_limit_is_enforced_by_parser_and_encoder() -> Result<(), Box<dyn Error>> {
    assert_eq!(CANONICAL_MAX_DEPTH, 64);

    let at_limit = format!(
        "{}0{}",
        "[".repeat(CANONICAL_MAX_DEPTH),
        "]".repeat(CANONICAL_MAX_DEPTH)
    );
    assert!(
        parse_canonical_text(&at_limit).is_ok(),
        "depth exactly at the limit must be accepted"
    );

    let over_limit = format!(
        "{}0{}",
        "[".repeat(CANONICAL_MAX_DEPTH + 1),
        "]".repeat(CANONICAL_MAX_DEPTH + 1)
    );
    assert_eq!(
        parse_canonical_text(&over_limit),
        Err(CanonicalError::DepthExceeded)
    );

    let hostile = format!("{}0{}", "[".repeat(100_000), "]".repeat(100_000));
    assert_eq!(
        parse_canonical_text(&hostile),
        Err(CanonicalError::DepthExceeded),
        "hostile nesting must be rejected, not abort"
    );

    let mut value = CanonicalValue::Integer(0);
    for _ in 0..CANONICAL_MAX_DEPTH {
        value = CanonicalValue::Array(vec![value]);
    }
    assert!(
        to_canonical_bytes(&value).is_ok(),
        "depth exactly at the limit must encode"
    );
    value = CanonicalValue::Array(vec![value]);
    assert_eq!(
        to_canonical_bytes(&value),
        Err(CanonicalError::DepthExceeded)
    );
    Ok(())
}

fn nest_arrays(value: CanonicalValue, levels: usize) -> CanonicalValue {
    (0..levels).fold(value, |inner, _| CanonicalValue::Array(vec![inner]))
}

#[test]
fn tagged_numeric_objects_count_toward_the_depth_limit() -> Result<(), Box<dyn Error>> {
    assert_eq!(CANONICAL_MAX_DEPTH, 64);

    let cases = [
        ("uint64", CanonicalValue::Uint64(u64::MAX), "uint64"),
        (
            "float64_bits",
            CanonicalValue::Float64Bits(0x3ff0_0000_0000_0000),
            "float64_bits",
        ),
    ];

    for (label, tagged, kind) in cases {
        let accepted = nest_arrays(tagged.clone(), CANONICAL_MAX_DEPTH - 1);
        let bytes = to_canonical_bytes(&accepted)?;
        let text = std::str::from_utf8(&bytes)?;
        assert!(
            text.contains(kind),
            "{label} encoding must carry its tag: {text}"
        );
        assert!(
            parse_canonical_text(text).is_ok(),
            "{label} at 63 enclosing arrays must be accepted by the parser"
        );

        let rejected = nest_arrays(tagged, CANONICAL_MAX_DEPTH);
        assert_eq!(
            to_canonical_bytes(&rejected),
            Err(CanonicalError::DepthExceeded),
            "{label} at 64 enclosing arrays must be rejected by the encoder"
        );

        let tagged_json = format!("{{\"kind\":\"{kind}\",\"value\":\"0\"}}");
        let deep_text = format!(
            "{}{}{}",
            "[".repeat(CANONICAL_MAX_DEPTH),
            tagged_json,
            "]".repeat(CANONICAL_MAX_DEPTH)
        );
        assert_eq!(
            parse_canonical_text(&deep_text),
            Err(CanonicalError::DepthExceeded),
            "{label} at 64 enclosing arrays must be rejected by the parser"
        );
    }
    Ok(())
}

#[test]
fn accepted_encodings_are_parser_readable_at_the_depth_boundary() -> Result<(), Box<dyn Error>> {
    let bases = [
        CanonicalValue::Integer(0),
        CanonicalValue::Null,
        CanonicalValue::Bool(true),
        CanonicalValue::Text("x".to_owned()),
        CanonicalValue::Uint64(u64::MAX),
        CanonicalValue::Float64Bits(0x3ff0_0000_0000_0000),
    ];

    for base in bases {
        for levels in (CANONICAL_MAX_DEPTH - 2)..=(CANONICAL_MAX_DEPTH + 1) {
            let value = nest_arrays(base.clone(), levels);
            let Ok(bytes) = to_canonical_bytes(&value) else {
                continue;
            };
            let text = std::str::from_utf8(&bytes)?;
            assert!(
                parse_canonical_text(text).is_ok(),
                "encoder output must be parser-readable at {levels} enclosing arrays"
            );
        }
    }
    Ok(())
}
