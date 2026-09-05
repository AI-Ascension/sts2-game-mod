// SPDX-License-Identifier: MIT

use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const CALLBACK_ACTION: u32 = 2;
const CALLBACK_RUNTIME_V2_STATE: u32 = 3;
const CALLBACK_RUNTIME_V2_ACTION: u32 = 4;
const CALLBACK_RUNTIME_V2_OPERATION: u32 = 5;
const CALLBACK_GAMEPLAY: u32 = 6;
const MAX_RESPONSE_BYTES: usize = 128 * 1024;
const STARTED: i32 = 0;
const INVALID_ARGUMENT: i32 = 1;
const ALREADY_STARTED: i32 = 2;
const BIND_FAILED: i32 = 3;
const THREAD_FAILED: i32 = 4;
const STOP_FAILED: i32 = 5;

#[path = "runtime_auth.rs"]
mod auth;
#[cfg(test)]
#[path = "runtime_endpoint_tests.rs"]
mod endpoint_tests;
#[path = "runtime_gameplay_route.rs"]
mod gameplay_route;
#[cfg(test)]
#[path = "runtime_gameplay_route_tests.rs"]
mod gameplay_route_tests;
#[path = "runtime_http.rs"]
mod http;
#[path = "runtime_io.rs"]
mod io;
#[cfg(test)]
#[path = "runtime_io_tests.rs"]
mod io_tests;
#[path = "runtime_listener.rs"]
mod listener;
#[path = "runtime_routes.rs"]
mod routes;

pub type RuntimeRequestCallback = unsafe extern "C" fn(
    request: *const RuntimeRequest,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> i32;

#[repr(C)]
#[derive(Debug)]
pub struct RuntimeCallbacks {
    pub request: Option<RuntimeRequestCallback>,
}

#[repr(C)]
#[derive(Debug)]
pub struct RuntimeRequest {
    pub kind: u32,
    pub instance_id: *const u8,
    pub instance_id_len: usize,
    pub caller_id: *const u8,
    pub caller_id_len: usize,
    pub session_id: *const u8,
    pub session_id_len: usize,
    pub lease_id: *const u8,
    pub lease_id_len: usize,
    pub lease_epoch: *const u8,
    pub lease_epoch_len: usize,
    pub correlation_id: *const u8,
    pub correlation_id_len: usize,
    pub body: *const u8,
    pub body_len: usize,
}

struct RuntimeHandle {
    stop: Arc<AtomicBool>,
    join: JoinHandle<()>,
}

static SERVER: OnceLock<Mutex<Option<RuntimeHandle>>> = OnceLock::new();

fn server_slot() -> &'static Mutex<Option<RuntimeHandle>> {
    SERVER.get_or_init(|| Mutex::new(None))
}

#[unsafe(no_mangle)]
/// Starts the listener with copied configuration and a borrowed callback.
///
/// # Safety
/// Inputs must be valid for their lengths and unmodified during copying; `callbacks` must be
/// aligned, readable and unmodified. The callback must remain callable on the listener thread until stop joins it,
/// must not unwind across the ABI, and must return within its own bounded time.
/// It may only access request/output pointers during the call and may not write
/// beyond output capacity. Calling stop from the callback itself is unsupported.
pub unsafe extern "C" fn sts2_game_mod_runtime_start(
    port: u16,
    bind_address: *const u8,
    bind_address_len: usize,
    token: *const u8,
    token_len: usize,
    callbacks: *const RuntimeCallbacks,
) -> i32 {
    // SAFETY: The caller supplies readable, unmodified input for the declared length.
    // copy_input checks null/bounds and copies synchronously into a Rust-owned Vec;
    // no caller allocation is freed, retained, or accessed by the listener thread.
    let bind_address = match unsafe {
        copy_input(
            bind_address,
            bind_address_len,
            listener::MAX_BIND_ADDRESS_BYTES,
        )
    }
    .ok()
    .and_then(|value| String::from_utf8(value).ok())
    {
        Some(value) if listener::valid_bind_address(&value) => value,
        _ => return INVALID_ARGUMENT,
    };
    // SAFETY: The same caller input contract applies to the token; copying is local
    // and read-only, with no ownership transfer or caller pointer retained after start.
    let token = match unsafe { copy_input(token, token_len, 256) } {
        Ok(value) if !value.is_empty() && value.iter().all(|byte| !byte.is_ascii_whitespace()) => {
            value
        }
        _ => return INVALID_ARGUMENT,
    };
    // SAFETY: A non-null callbacks pointer is caller-owned, aligned and readable,
    // with no mutation during this borrow. Only the function pointer is copied;
    // the caller keeps its code/delegate alive on the listener thread through stop/join.
    // Neither the table nor callback is freed here; unload must wait for that join.
    let callback = match unsafe { callbacks.as_ref() }.and_then(|value| value.request) {
        Some(value) => value,
        None => return INVALID_ARGUMENT,
    };

    let Ok(mut slot) = server_slot().lock() else {
        return THREAD_FAILED;
    };
    if slot.is_some() {
        return ALREADY_STARTED;
    }

    let (listener, address) = match listener::bind(&bind_address, port) {
        Ok(value) => value,
        Err(_) => return BIND_FAILED,
    };
    if listener.set_nonblocking(true).is_err() {
        return BIND_FAILED;
    }

    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let listener_address = address.to_string();
    let join = match thread::Builder::new()
        .name(String::from("sts2-runtime-http"))
        .spawn(move || serve(listener, listener_address, token, callback, thread_stop))
    {
        Ok(value) => value,
        Err(_) => return THREAD_FAILED,
    };
    *slot = Some(RuntimeHandle { stop, join });
    STARTED
}

#[unsafe(no_mangle)]
pub extern "C" fn sts2_game_mod_runtime_stop() -> i32 {
    let Ok(mut slot) = server_slot().lock() else {
        return STOP_FAILED;
    };
    let Some(handle) = slot.take() else {
        return STARTED;
    };
    handle.stop.store(true, Ordering::Release);
    if handle.join.join().is_err() {
        return STOP_FAILED;
    }
    STARTED
}

fn serve(
    listener: TcpListener,
    listener_address: String,
    token: Vec<u8>,
    callback: RuntimeRequestCallback,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut connection =
                    io::Connection::new(&mut stream, &stop, Duration::from_secs(10));
                let _ = handle_connection(&mut connection, &listener_address, &token, callback);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => thread::sleep(Duration::from_millis(10)),
        }
    }
}

fn handle_connection(
    stream: &mut io::Connection<'_>,
    listener_address: &str,
    token: &[u8],
    callback: RuntimeRequestCallback,
) -> std::io::Result<()> {
    let request = match http::read_request(stream) {
        Ok(value) => value,
        Err(status) => {
            return http::write_response(stream, status, b"{\"error_code\":\"malformed_request\"}");
        }
    };
    if !http::headers_are_allowed(&request.headers) {
        return http::write_response(stream, 400, b"{\"error_code\":\"unsupported_header\"}");
    }
    if !auth::bearer_token_matches(request.headers.get("authorization"), token) {
        return http::write_response(stream, 401, b"{\"error_code\":\"unauthorized\"}");
    }

    routes::dispatch(callback, &request, listener_address, stream)
}

fn dispatch(
    callback: RuntimeRequestCallback,
    kind: u32,
    request: &http::Request,
    stream: &mut io::Connection<'_>,
) -> std::io::Result<()> {
    dispatch_with_body(callback, kind, request, &request.body, stream)
}

fn dispatch_with_body(
    callback: RuntimeRequestCallback,
    kind: u32,
    request: &http::Request,
    body: &[u8],
    stream: &mut io::Connection<'_>,
) -> std::io::Result<()> {
    let Some(instance_id) = request.headers.get("x-sts2-instance-id") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_instance_id\"}");
    };
    let Some(caller_id) = request.headers.get("x-sts2-caller-id") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_caller_id\"}");
    };
    let Some(session_id) = request.headers.get("x-sts2-session-id") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_session_id\"}");
    };
    let Some(lease_id) = request.headers.get("x-sts2-lease-id") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_lease_id\"}");
    };
    let Some(lease_epoch) = request.headers.get("x-sts2-lease-epoch") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_lease_epoch\"}");
    };
    let Some(correlation_id) = request.headers.get("x-sts2-correlation-id") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_correlation_id\"}");
    };
    if [
        instance_id.as_str(),
        caller_id.as_str(),
        session_id.as_str(),
        lease_id.as_str(),
        lease_epoch.as_str(),
        correlation_id.as_str(),
    ]
    .into_iter()
    .any(|value| !http::safe_header_value(value))
    {
        return http::write_response(stream, 400, b"{\"error_code\":\"unsafe_identity\"}");
    }

    let native_request = RuntimeRequest {
        kind,
        instance_id: instance_id.as_bytes().as_ptr(),
        instance_id_len: instance_id.len(),
        caller_id: caller_id.as_bytes().as_ptr(),
        caller_id_len: caller_id.len(),
        session_id: session_id.as_bytes().as_ptr(),
        session_id_len: session_id.len(),
        lease_id: lease_id.as_bytes().as_ptr(),
        lease_id_len: lease_id.len(),
        lease_epoch: lease_epoch.as_bytes().as_ptr(),
        lease_epoch_len: lease_epoch.len(),
        correlation_id: correlation_id.as_bytes().as_ptr(),
        correlation_id_len: correlation_id.len(),
        body: body.as_ptr(),
        body_len: body.len(),
    };
    let mut output = vec![0_u8; MAX_RESPONSE_BYTES];
    let mut output_length = 0_usize;
    // SAFETY: Request fields borrow live, read-only buffers for this synchronous call.
    // Output and length are distinct, exclusively borrowed Rust-owned storage; the
    // callback must obey capacity, retain/free no pointers, and never unwind.
    // The start caller guarantees callback validity on this listener thread until
    // stop joins it, before delegate release or native-library unload.
    let status = unsafe {
        callback(
            &native_request,
            output.as_mut_ptr(),
            output.len(),
            &mut output_length,
        )
    };
    if !(200..600).contains(&status) || output_length > output.len() {
        return http::write_response(stream, 500, b"{\"error_code\":\"callback_failed\"}");
    }
    http::write_response(stream, status as u16, &output[..output_length])
}

unsafe fn copy_input(pointer: *const u8, length: usize, maximum: usize) -> Result<Vec<u8>, i32> {
    if pointer.is_null() || length > maximum {
        return Err(INVALID_ARGUMENT);
    }
    // SAFETY: Callers guarantee one readable allocation, unmodified for this borrow;
    // null/length checks above bound it, including non-null for an empty slice.
    // This thread only reads and copies; the caller retains allocation ownership.
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
    Ok(bytes.to_vec())
}
