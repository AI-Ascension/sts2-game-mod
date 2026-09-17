# Exact-restore loopback peer (test only)

`exact_restore_peer` is a bounded, loopback-only process for integration tests. It
uses the public `ExactRestoreEngine<SecureExactRestoreStore, ...>` and a synthetic
applier. It does not install a game hook and its receipt is never native
certification.

The process requires:

* `STS2_MOD_ADDR` (for example `127.0.0.1:0`; the process prints `READY host:port`)
* `STS2_MOD_TOKEN`
* `STS2_CALLER_ID`
* `STS2_INSTANCE_ID`, `STS2_SESSION_ID`, `STS2_LEASE_ID`, `STS2_LEASE_EPOCH`
* `STS2_EXACT_OWNER_FILE` (a JSON snapshot with `fence` and `observed_at_millis`)
* `STS2_EXACT_STORE` (a configured private scratch directory)

The owner snapshot is authoritative test fixture input. The HTTP request's
`expected_owner` is checked by the public engine against this provider; it never
creates or replaces the owner. After an authenticated host-lease install, the
synthetic peer updates the in-memory owner from the signed gateway grant so the
subsequent exact-restore frame uses the gateway-issued lease.

Exact routes are fixed `POST` paths:

* `/v1/exact-restore/begin`
* `/v1/exact-restore/chunk`
* `/v1/exact-restore/finish`
* `/v1/exact-restore/commit`
* `/v1/exact-restore/lookup`

They require `Authorization: Bearer ...`, `X-STS2-Caller-Id`,
`X-STS2-Instance-Id`, `X-STS2-Session-Id`, `X-STS2-Lease-Id`, and
`X-STS2-Lease-Epoch`. Every frame and response stays within the 16 KiB exact
wire bound. Unknown paths and methods are rejected.

Gateway recovery uses the one fixed `POST /api/v1/runtime/recovery` mux. The
peer handles host-fence and signed host-lease install, renew, and revoke
frames there. Gateway bootstrap, health, state, and session allocation remain
Gateway-owned routes.

Optional test switches:

* `STS2_EXACT_NATIVE_UNSUPPORTED=1` makes begin refuse before staging.
* `STS2_EXACT_COMMIT_UNKNOWN=1` makes the synthetic applier return `UNKNOWN`.
* `STS2_EXACT_LOOKUP_UNKNOWN_ONCE=1` returns one deterministic lookup `UNKNOWN`.
* `STS2_EXACT_MAX_REQUESTS=N` exits after `N` accepted connections.
* `STS2_RUNTIME_HOST_LEASE_KEY` is the 32-byte hex key used to authenticate
  host-lease acknowledgements.

Run with:

```text
cargo run -p sts2-game-mod --example exact_restore_peer
```
