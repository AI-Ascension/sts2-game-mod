// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{CANONICAL_MAX_SAFE_INTEGER, CanonicalError, CanonicalValue};

/// Strictly parses restricted canonical JSON text into the game-owned value model.
///
/// # Errors
///
/// Returns [`CanonicalError`] for duplicate object keys, floats, exponents,
/// negative zero, unsafe integers, non-ASCII object keys, malformed input, or
/// trailing text.
pub fn parse_canonical_text(text: &str) -> Result<CanonicalValue, CanonicalError> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        offset: 0,
    };
    parser.skip_whitespace();
    if parser.peek().is_none() {
        return Err(CanonicalError::EmptyInput);
    }
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.offset != parser.bytes.len() {
        return Err(CanonicalError::TrailingInput);
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.offset).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.offset += 1;
        Some(byte)
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.peek() {
            if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                self.offset += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), CanonicalError> {
        match self.advance() {
            Some(actual) if actual == byte => Ok(()),
            Some(_) => Err(CanonicalError::UnexpectedToken {
                offset: self.offset - 1,
            }),
            None => Err(CanonicalError::UnexpectedEnd),
        }
    }

    fn parse_value(&mut self) -> Result<CanonicalValue, CanonicalError> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(CanonicalValue::Text(self.parse_string()?)),
            Some(b't') => self.parse_literal("true", CanonicalValue::Bool(true)),
            Some(b'f') => self.parse_literal("false", CanonicalValue::Bool(false)),
            Some(b'n') => self.parse_literal("null", CanonicalValue::Null),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(_) => Err(CanonicalError::UnexpectedToken {
                offset: self.offset,
            }),
            None => Err(CanonicalError::UnexpectedEnd),
        }
    }

    fn parse_literal(
        &mut self,
        literal: &str,
        value: CanonicalValue,
    ) -> Result<CanonicalValue, CanonicalError> {
        for expected in literal.bytes() {
            match self.advance() {
                Some(actual) if actual == expected => {}
                Some(_) => {
                    return Err(CanonicalError::UnexpectedToken {
                        offset: self.offset - 1,
                    });
                }
                None => return Err(CanonicalError::UnexpectedEnd),
            }
        }
        Ok(value)
    }

    fn parse_object(&mut self) -> Result<CanonicalValue, CanonicalError> {
        self.expect(b'{')?;
        let mut entries = BTreeMap::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.advance();
            return Ok(CanonicalValue::Object(entries));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.unexpected());
            }
            let key = self.parse_string()?;
            if !key.is_ascii() {
                return Err(CanonicalError::NonAsciiKey);
            }
            self.skip_whitespace();
            self.expect(b':')?;
            let value = self.parse_value()?;
            if entries.insert(key, value).is_some() {
                return Err(CanonicalError::DuplicateKey);
            }
            self.skip_whitespace();
            match self.advance() {
                Some(b',') => {}
                Some(b'}') => return Ok(CanonicalValue::Object(entries)),
                Some(_) => {
                    return Err(CanonicalError::UnexpectedToken {
                        offset: self.offset - 1,
                    });
                }
                None => return Err(CanonicalError::UnexpectedEnd),
            }
        }
    }

    fn parse_array(&mut self) -> Result<CanonicalValue, CanonicalError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.advance();
            return Ok(CanonicalValue::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.advance() {
                Some(b',') => {}
                Some(b']') => return Ok(CanonicalValue::Array(items)),
                Some(_) => {
                    return Err(CanonicalError::UnexpectedToken {
                        offset: self.offset - 1,
                    });
                }
                None => return Err(CanonicalError::UnexpectedEnd),
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, CanonicalError> {
        self.expect(b'"')?;
        let mut bytes = Vec::new();
        loop {
            match self.advance() {
                Some(b'"') => break,
                Some(b'\\') => self.parse_escape(&mut bytes)?,
                Some(byte) if byte < 0x20 => {
                    return Err(CanonicalError::ControlCharacter {
                        offset: self.offset - 1,
                    });
                }
                Some(byte) => bytes.push(byte),
                None => return Err(CanonicalError::UnexpectedEnd),
            }
        }
        String::from_utf8(bytes).map_err(|_| CanonicalError::InvalidUtf8)
    }

    fn parse_escape(&mut self, output: &mut Vec<u8>) -> Result<(), CanonicalError> {
        let escape_offset = self.offset - 1;
        match self.advance() {
            Some(b'"') => output.push(b'"'),
            Some(b'\\') => output.push(b'\\'),
            Some(b'/') => output.push(b'/'),
            Some(b'b') => output.push(0x08),
            Some(b'f') => output.push(0x0c),
            Some(b'n') => output.push(b'\n'),
            Some(b'r') => output.push(b'\r'),
            Some(b't') => output.push(b'\t'),
            Some(b'u') => self.parse_unicode_escape(output)?,
            Some(_) => {
                return Err(CanonicalError::InvalidEscape {
                    offset: escape_offset,
                });
            }
            None => return Err(CanonicalError::UnexpectedEnd),
        }
        Ok(())
    }

    fn parse_unicode_escape(&mut self, output: &mut Vec<u8>) -> Result<(), CanonicalError> {
        let escape_offset = self.offset.saturating_sub(2);
        let invalid = || CanonicalError::InvalidUnicodeEscape {
            offset: escape_offset,
        };
        let first = self.read_hex_quad().ok_or_else(invalid)?;
        let scalar = if (0xd800..=0xdbff).contains(&first) {
            if self.advance() != Some(b'\\') || self.advance() != Some(b'u') {
                return Err(invalid());
            }
            let second = self.read_hex_quad().ok_or_else(invalid)?;
            if !(0xdc00..=0xdfff).contains(&second) {
                return Err(invalid());
            }
            let high = u32::from(first - 0xd800);
            let low = u32::from(second - 0xdc00);
            0x1_0000 + (high << 10) + low
        } else if (0xdc00..=0xdfff).contains(&first) {
            return Err(invalid());
        } else {
            u32::from(first)
        };
        let character = char::from_u32(scalar).ok_or_else(invalid)?;
        let mut buffer = [0_u8; 4];
        output.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
        Ok(())
    }

    fn read_hex_quad(&mut self) -> Option<u16> {
        let mut value = 0_u16;
        for _ in 0..4 {
            value = (value << 4) | u16::from(hex_digit(self.advance()?)?);
        }
        Some(value)
    }

    fn parse_number(&mut self) -> Result<CanonicalValue, CanonicalError> {
        let negative = self.peek() == Some(b'-');
        if negative {
            self.offset += 1;
        }
        let Some(first) = self.peek() else {
            return Err(CanonicalError::UnexpectedEnd);
        };
        let mut magnitude: u64 = 0;
        if first == b'0' {
            self.offset += 1;
            if self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                return Err(CanonicalError::LeadingZero);
            }
        } else if first.is_ascii_digit() {
            while let Some(byte) = self.peek() {
                if !byte.is_ascii_digit() {
                    break;
                }
                self.offset += 1;
                magnitude = magnitude
                    .checked_mul(10)
                    .and_then(|value| value.checked_add(u64::from(byte - b'0')))
                    .ok_or(CanonicalError::UnsafeInteger)?;
            }
        } else {
            return Err(CanonicalError::UnexpectedToken {
                offset: self.offset,
            });
        }
        match self.peek() {
            Some(b'.') => return Err(CanonicalError::FloatNotAllowed),
            Some(b'e' | b'E') => return Err(CanonicalError::ExponentNotAllowed),
            _ => {}
        }
        if magnitude > CANONICAL_MAX_SAFE_INTEGER as u64 {
            return Err(CanonicalError::UnsafeInteger);
        }
        if negative {
            if magnitude == 0 {
                return Err(CanonicalError::NegativeZero);
            }
            let signed = i64::try_from(magnitude).map_err(|_| CanonicalError::UnsafeInteger)?;
            return Ok(CanonicalValue::Integer(-signed));
        }
        let signed = i64::try_from(magnitude).map_err(|_| CanonicalError::UnsafeInteger)?;
        Ok(CanonicalValue::Integer(signed))
    }

    fn unexpected(&self) -> CanonicalError {
        match self.peek() {
            Some(_) => CanonicalError::UnexpectedToken {
                offset: self.offset,
            },
            None => CanonicalError::UnexpectedEnd,
        }
    }
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
