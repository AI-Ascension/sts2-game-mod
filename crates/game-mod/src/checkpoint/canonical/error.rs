// SPDX-License-Identifier: MIT

/// Rejection while encoding or strictly parsing a canonical payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalError {
    /// The input contained no value.
    EmptyInput,
    /// A value, key, or delimiter ended unexpectedly.
    UnexpectedEnd,
    /// A byte that cannot begin a value or delimiter was found.
    UnexpectedToken {
        /// Byte offset of the offending input.
        offset: usize,
    },
    /// Non-whitespace bytes followed a complete value.
    TrailingInput,
    /// A raw control byte appeared inside a string.
    ControlCharacter {
        /// Byte offset of the offending input.
        offset: usize,
    },
    /// A string escape sequence is not permitted.
    InvalidEscape {
        /// Byte offset of the offending input.
        offset: usize,
    },
    /// A `\u` escape is malformed or an unpaired surrogate.
    InvalidUnicodeEscape {
        /// Byte offset of the offending input.
        offset: usize,
    },
    /// Raw or escaped bytes did not form valid UTF-8.
    InvalidUtf8,
    /// An object key contains a non-ASCII byte.
    NonAsciiKey,
    /// An object repeated a key.
    DuplicateKey,
    /// A numeric value exceeds the JCS safe integer range.
    UnsafeInteger,
    /// A fractional number is not representable in the restricted profile.
    FloatNotAllowed,
    /// An exponent form is not representable in the restricted profile.
    ExponentNotAllowed,
    /// `-0` is distinct from `0` and is not permitted.
    NegativeZero,
    /// A leading zero was found on a multi-digit integer.
    LeadingZero,
    /// Object or array nesting exceeds `CANONICAL_MAX_DEPTH`.
    DepthExceeded,
}

impl std::fmt::Display for CanonicalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInput => formatter.write_str("input is empty"),
            Self::UnexpectedEnd => formatter.write_str("input ended unexpectedly"),
            Self::UnexpectedToken { offset } => {
                write!(formatter, "unexpected token at byte {offset}")
            }
            Self::TrailingInput => formatter.write_str("trailing input after value"),
            Self::ControlCharacter { offset } => {
                write!(formatter, "unescaped control byte at byte {offset}")
            }
            Self::InvalidEscape { offset } => {
                write!(formatter, "invalid escape at byte {offset}")
            }
            Self::InvalidUnicodeEscape { offset } => {
                write!(formatter, "invalid unicode escape at byte {offset}")
            }
            Self::InvalidUtf8 => formatter.write_str("string is not valid UTF-8"),
            Self::NonAsciiKey => formatter.write_str("object key is not ASCII"),
            Self::DuplicateKey => formatter.write_str("object key is duplicated"),
            Self::UnsafeInteger => formatter.write_str("integer exceeds the safe range"),
            Self::FloatNotAllowed => formatter.write_str("fractional numbers are not allowed"),
            Self::ExponentNotAllowed => formatter.write_str("exponent numbers are not allowed"),
            Self::NegativeZero => formatter.write_str("negative zero is not allowed"),
            Self::LeadingZero => formatter.write_str("integer has a leading zero"),
            Self::DepthExceeded => formatter.write_str("nesting exceeds the canonical depth limit"),
        }
    }
}

impl std::error::Error for CanonicalError {}
