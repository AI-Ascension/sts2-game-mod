// SPDX-License-Identifier: MIT

#![allow(
    unsafe_code,
    reason = "this dedicated spike proves the reviewed native FFI boundary"
)]

const ABI_VERSION: u32 = 1;
const STATUS_OK: i32 = 0;
const STATUS_NULL_OUTPUT: i32 = 1;
const STATUS_OVERFLOW: i32 = 2;

mod runtime;

pub use runtime::{RuntimeCallbacks, RuntimeRequest};

/// Returns the version of the deliberately small native ABI.
#[unsafe(no_mangle)]
pub extern "C" fn sts2_game_mod_interop_abi_version() -> u32 {
    ABI_VERSION
}

/// Adds two values to prove argument, return-code, and output-pointer marshalling.
///
/// # Safety
///
/// When non-null, `output` must be aligned, writable, and valid for one `i32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sts2_game_mod_interop_checked_add(
    left: i32,
    right: i32,
    output: *mut i32,
) -> i32 {
    if output.is_null() {
        return STATUS_NULL_OUTPUT;
    }

    let Some(sum) = left.checked_add(right) else {
        return STATUS_OVERFLOW;
    };

    // SAFETY: The caller contract requires a valid output pointer and null was rejected above.
    unsafe { output.write(sum) };
    STATUS_OK
}
