// SPDX-License-Identifier: MIT

use super::super::super::super::{
    EXACT_RESTORE_MAX_CHUNK_BASE64_BYTES, EXACT_RESTORE_MAX_CHUNK_BYTES, ExactRestoreError,
};

pub(super) fn decode_base64_canonical(encoded: &str) -> Result<Vec<u8>, ExactRestoreError> {
    if encoded.is_empty()
        || encoded.len() > EXACT_RESTORE_MAX_CHUNK_BASE64_BYTES
        || !encoded.len().is_multiple_of(4)
    {
        return Err(ExactRestoreError::InvalidFrame);
    }
    let bytes = encoded.as_bytes();
    let padding = if bytes.ends_with(b"==") {
        2
    } else if bytes.ends_with(b"=") {
        1
    } else {
        0
    };
    let decoded_len = bytes
        .len()
        .checked_div(4)
        .and_then(|groups| groups.checked_mul(3))
        .and_then(|length| length.checked_sub(padding))
        .ok_or(ExactRestoreError::InvalidFrame)?;
    if decoded_len == 0 || decoded_len > EXACT_RESTORE_MAX_CHUNK_BYTES {
        return Err(ExactRestoreError::InvalidFrame);
    }
    let mut output = Vec::with_capacity(decoded_len);
    for (group_index, group) in bytes.chunks_exact(4).enumerate() {
        let is_last = group_index + 1 == bytes.len() / 4;
        let a = base64_digit(group[0]).ok_or(ExactRestoreError::InvalidFrame)?;
        let b = base64_digit(group[1]).ok_or(ExactRestoreError::InvalidFrame)?;
        let c = if group[2] == b'=' {
            if !is_last || group[3] != b'=' || b & 0x0f != 0 {
                return Err(ExactRestoreError::InvalidFrame);
            }
            None
        } else {
            Some(base64_digit(group[2]).ok_or(ExactRestoreError::InvalidFrame)?)
        };
        let d = if group[3] == b'=' {
            if !is_last || c.is_some_and(|value| value & 0x03 != 0) {
                return Err(ExactRestoreError::InvalidFrame);
            }
            None
        } else {
            Some(base64_digit(group[3]).ok_or(ExactRestoreError::InvalidFrame)?)
        };
        output.push((a << 2) | (b >> 4));
        if let Some(c) = c {
            output.push((b << 4) | (c >> 2));
            if let Some(d) = d {
                output.push((c << 6) | d);
            }
        }
    }
    if output.len() != decoded_len {
        return Err(ExactRestoreError::InvalidFrame);
    }
    Ok(output)
}

fn base64_digit(value: u8) -> Option<u8> {
    match value {
        b'A'..=b'Z' => Some(value - b'A'),
        b'a'..=b'z' => Some(value - b'a' + 26),
        b'0'..=b'9' => Some(value - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
