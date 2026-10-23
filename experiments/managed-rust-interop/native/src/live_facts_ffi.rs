// SPDX-License-Identifier: MIT

use serde::Deserialize;
use sts2_game_mod::{LiveFixedCardFacts, LiveFixedCardObservation};
use sts2_protocol::game_facts_reference_v1::{GameFactsReferenceV1Codec, Message};

const MAX_INPUT: u32 = 16 * 1024 * 1024;
const MAX_OUTPUT: u32 = 262_144;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    operation: String,
    correlation_id: String,
    #[serde(deserialize_with = "required_option")]
    request: Option<Box<serde_json::value::RawValue>>,
    #[serde(deserialize_with = "required_option")]
    source: Option<Box<serde_json::value::RawValue>>,
    #[serde(deserialize_with = "required_option")]
    observation: Option<LiveFixedCardObservation>,
}

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn respond(input: &[u8]) -> Result<(i32, Vec<u8>), i32> {
    let input: Input = serde_json::from_slice(input).map_err(|_| 400)?;
    let status = if input.operation == "reject_stale" {
        409
    } else {
        200
    };
    let response: Message = match input.operation.as_str() {
        "capabilities"
            if input.request.is_none() && input.source.is_none() && input.observation.is_none() =>
        {
            LiveFixedCardFacts::capabilities(&input.correlation_id).map_err(|_| 400)?
        }
        "query" | "reject_stale" => {
            let request = input.request.ok_or(400)?;
            let request =
                GameFactsReferenceV1Codec::decode(request.get().as_bytes()).map_err(|_| 400)?;
            if request.correlation_id != input.correlation_id {
                return Err(400);
            }
            if input.operation == "reject_stale" {
                if input.source.is_some() || input.observation.is_some() {
                    return Err(400);
                }
                let response = LiveFixedCardFacts::reject_stale(&request).map_err(|_| 400)?;
                return GameFactsReferenceV1Codec::encode(&response)
                    .map(|bytes| (status, bytes))
                    .map_err(|_| 500);
            }
            let manifest = input
                .source
                .map(|source| {
                    super::content_manifest_ffi::produce_manifest(source.get().as_bytes())
                        .map_err(|_| 503)
                })
                .transpose()?;
            LiveFixedCardFacts::respond(&request, manifest.as_ref(), input.observation.as_ref())
                .map_err(|_| 400)?
        }
        _ => return Err(400),
    };
    GameFactsReferenceV1Codec::encode(&response)
        .map(|bytes| (status, bytes))
        .map_err(|_| 500)
}

/// Canonical facts codec/manifest bridge. Accepts only owned managed source snapshots.
/// This internal ABI is not an unauthenticated HTTP inventory submission endpoint.
///
/// # Safety
/// Non-null buffers must be valid for their declared sizes; `output_length` must be writable.
/// Input/output storage must not overlap and must remain live for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sts2_game_mod_live_facts_respond(
    input: *const u8,
    input_length: u32,
    output: *mut u8,
    output_capacity: u32,
    output_length: *mut u32,
) -> i32 {
    if output_length.is_null() {
        return 400;
    }
    // SAFETY: The caller guarantees a valid writable length pointer.
    unsafe {
        *output_length = 0;
    }
    // Check every bound before making any slice, including hostile lengths with null pointers.
    if input.is_null()
        || output.is_null()
        || input_length == 0
        || input_length > MAX_INPUT
        || output_capacity == 0
        || output_capacity > MAX_OUTPUT
    {
        return 400;
    }
    let result = std::panic::catch_unwind(|| {
        // SAFETY: Input pointer validity is required by the caller; length was bounded above.
        respond(unsafe { std::slice::from_raw_parts(input, input_length as usize) })
    });
    match result {
        Ok(Ok((status, bytes))) if bytes.len() <= output_capacity as usize => {
            // SAFETY: The non-overlapping writable caller buffer has sufficient capacity.
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
                *output_length = bytes.len() as u32;
            }
            status
        }
        Ok(Err(status)) => status,
        _ => 500,
    }
}

#[cfg(test)]
#[path = "live_facts_ffi_tests.rs"]
mod tests;
