// SPDX-License-Identifier: MIT

use super::super::CHECKPOINT_CAPTURE_MAX_BYTES;
use super::CanonicalError;

/// Every append is admitted before growing the output allocation.
#[derive(Default)]
pub(super) struct Output(String);

impl Output {
    pub(super) fn push_str(&mut self, text: &str) -> Result<(), CanonicalError> {
        if text.len() > CHECKPOINT_CAPTURE_MAX_BYTES - self.0.len() {
            return Err(CanonicalError::PayloadTooLarge);
        }
        let needed = self.0.len() + text.len();
        if needed > self.0.capacity() {
            let capacity = needed
                .max(self.0.capacity().saturating_mul(2))
                .min(CHECKPOINT_CAPTURE_MAX_BYTES);
            self.0.reserve_exact(capacity - self.0.len());
        }
        self.0.push_str(text);
        Ok(())
    }

    pub(super) fn push(&mut self, character: char) -> Result<(), CanonicalError> {
        self.push_str(character.encode_utf8(&mut [0; 4]))
    }

    pub(super) fn into_bytes(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}
