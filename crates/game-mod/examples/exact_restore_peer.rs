// SPDX-License-Identifier: MIT
//
//! Test-only loopback peer for Gateway exact-restore and recovery integration.
//! This executable installs no native game hook and never certifies native restore.

#[path = "exact_restore_peer/config.rs"]
mod config;
#[path = "exact_restore_peer/fixture.rs"]
mod fixture;
#[path = "exact_restore_peer/http.rs"]
mod http;
#[path = "exact_restore_peer/http_wire.rs"]
mod http_wire;
#[path = "exact_restore_peer/recovery.rs"]
mod recovery;
#[path = "exact_restore_peer/runtime_v3.rs"]
mod runtime_v3;
#[path = "exact_restore_peer/runtime_v3_recovery.rs"]
mod runtime_v3_recovery;
#[path = "exact_restore_peer/runtime_v3_wire.rs"]
mod runtime_v3_wire;

fn main() {
    if let Err(error) = http::run() {
        eprintln!("exact-restore-peer: {error}");
        std::process::exit(1);
    }
}
