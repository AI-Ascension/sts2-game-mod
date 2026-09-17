// SPDX-License-Identifier: MIT

use super::*;

const STATUS_BAD_REQUEST: i32 = 400;

pub(super) fn run(input: &[u8], query: &[u8]) -> (i32, Vec<u8>) {
    if input.len() > MAX_INPUT_BYTES || query.len() > MAX_INPUT_BYTES {
        return (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"query_input_limit"}"#.to_vec(),
        );
    }
    let Ok(query) = serde_json::from_slice::<InputQuery>(query) else {
        return (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"query_input_malformed"}"#.to_vec(),
        );
    };
    if query.limit == 0 || query.limit > 64 || query.entity_kind != "card" {
        return (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"query_bounds"}"#.to_vec(),
        );
    }
    if let Some(cursor) = &query.cursor {
        let Some(state) = cursors()
            .lock()
            .ok()
            .and_then(|mut values| values.remove(cursor))
        else {
            return (409, br#"{"error_code":"stale_cursor"}"#.to_vec());
        };
        if state.manifest_id.is_empty() {
            return (409, br#"{"error_code":"stale_cursor"}"#.to_vec());
        }
        if state.binding_key != query.binding_key {
            return (409, br#"{"error_code":"stale_cursor"}"#.to_vec());
        }
        if !query.manifest_id.is_empty() && query.manifest_id != state.manifest_id {
            return (409, br#"{"error_code":"stale_cursor"}"#.to_vec());
        }
        let CursorState {
            reader,
            continuation,
            manifest_id,
            tags,
            ..
        } = state;
        match projection::project(reader, &query, &manifest_id, Some(continuation), &tags) {
            Ok((output, next)) => {
                if let Some(next) = next
                    && let Some(cursor) = &output.next_cursor
                {
                    retain_cursor(cursor.clone(), next);
                }
                return serde_json::to_vec(&output)
                    .map_or((STATUS_UNAVAILABLE, Vec::new()), |bytes| (STATUS_OK, bytes));
            }
            Err(_) => return (409, br#"{"error_code":"stale_cursor"}"#.to_vec()),
        }
    }
    let Ok(snapshot) = serde_json::from_slice::<InputSnapshot>(input) else {
        return (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"query_snapshot_malformed"}"#.to_vec(),
        );
    };
    let Ok((manifest, index, tags)) = build_index(snapshot) else {
        return (
            STATUS_UNAVAILABLE,
            br#"{"error_code":"query_index_unavailable"}"#.to_vec(),
        );
    };
    let manifest_id = manifest.inventory_revision.clone();
    if !query.manifest_id.is_empty() && query.manifest_id != manifest_id {
        return (409, br#"{"error_code":"stale_cursor"}"#.to_vec());
    }
    match projection::project(index.reader(), &query, &manifest_id, None, &tags) {
        Ok((output, next)) => {
            if let Some(next) = next
                && let Some(cursor) = &output.next_cursor
            {
                retain_cursor(cursor.clone(), next);
            }
            serde_json::to_vec(&output)
                .map_or((STATUS_UNAVAILABLE, Vec::new()), |bytes| (STATUS_OK, bytes))
        }
        Err(error) if error == "not_found" => (404, br#"{"error_code":"unknown_id"}"#.to_vec()),
        Err(error) if error == "denied_scope" => {
            (403, br#"{"error_code":"denied_scope"}"#.to_vec())
        }
        Err(error) if error == "unsupported_filter" => (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"unsupported_filter"}"#.to_vec(),
        ),
        Err(_) => (
            STATUS_UNAVAILABLE,
            br#"{"error_code":"query_failed"}"#.to_vec(),
        ),
    }
}

fn write_output(output: *mut u8, capacity: usize, length: *mut usize, bytes: &[u8]) -> i32 {
    if output.is_null()
        || length.is_null()
        || bytes.len() > capacity
        || bytes.len() > MAX_OUTPUT_BYTES
    {
        return STATUS_UNAVAILABLE;
    }
    // SAFETY: the caller supplies writable storage for the declared capacity and a valid length.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
        *length = bytes.len();
    }
    STATUS_OK
}

/// Runs one bounded static content-index query through the Rust ContentIndexReader.
///
/// The input snapshot and query are copied before this call and the output is written only to
/// caller-owned bounded storage. Cursor state retains one native reader and continuation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sts2_game_mod_content_index_query(
    input: *const u8,
    input_length: usize,
    query: *const u8,
    query_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> i32 {
    let result = std::panic::catch_unwind(|| {
        if query.is_null() || output_length.is_null() {
            return (STATUS_BAD_REQUEST, Vec::new());
        }
        // SAFETY: caller guarantees readable buffers for the declared lengths.
        let query_bytes = unsafe { std::slice::from_raw_parts(query, query_length) };
        let input_bytes = if input.is_null() {
            &[][..]
        } else {
            // SAFETY: caller guarantees readable snapshot buffer for the declared length.
            unsafe { std::slice::from_raw_parts(input, input_length) }
        };
        run(input_bytes, query_bytes)
    });
    match result {
        Ok((status, bytes)) => {
            if bytes.is_empty() {
                return status;
            }
            let write_status = write_output(output, output_capacity, output_length, &bytes);
            if write_status == STATUS_OK {
                status
            } else {
                write_status
            }
        }
        Err(_) => STATUS_UNAVAILABLE,
    }
}
