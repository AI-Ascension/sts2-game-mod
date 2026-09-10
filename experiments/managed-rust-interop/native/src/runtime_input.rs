// SPDX-License-Identifier: MIT

pub(super) unsafe fn copy_input(
    pointer: *const u8,
    length: usize,
    maximum: usize,
) -> Result<Vec<u8>, i32> {
    if pointer.is_null() || length > maximum {
        return Err(super::INVALID_ARGUMENT);
    }
    // SAFETY: Callers guarantee one readable allocation, unmodified for this borrow;
    // null/length checks above bound it, including non-null for an empty slice.
    // This thread only reads and copies; the caller retains allocation ownership.
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
    Ok(bytes.to_vec())
}
