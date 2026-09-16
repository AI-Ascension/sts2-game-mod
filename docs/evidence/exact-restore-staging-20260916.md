# Exact-restore source-only staging evidence

## Scope and pinned input

This report covers the `sts2-game-mod` source consumer for the accepted neutral
`exact-restore-v1` contract. The copied protocol artifact is byte-identical to the reviewed
artifact from `sts2-protocol` commit `3233bb727fd48b9e8899a98801af392b9f86ba52`, merged to main
as `5d5a368ef8a89fd1cb356b04dbf9d8a056adbf05`. Its schema SHA-256 is
`2289d888c33eac46873408303c4423eab762e3f7bd6132ae8ae88d0d3b1858e4`.

The owner-local consumer uses an embedded schema and strict duplicate-key and canonical-frame
checks. It enforces 16 KiB request and response frames, 8 KiB nonempty raw chunks, canonical
base64, at most 64 references, 16 MiB per manifest/blob, and 64 MiB aggregate closure. Manifest
bytes are counted separately from deduplicated canonical/restore blobs. Before a synthetic host
adapter can be called, the consumer verifies the complete manifest shape, selected branch and
owner bindings, exact-state identity, every reference, canonical payload and closure digest.

The source-only phase engine persists ordered staging and never retries after durable
`COMMIT_INTENT`; restart converts uncertain intent to `UNKNOWN`. Its Linux storage candidate uses
an owner-private directory, no-follow descriptor-relative opens, a process lock, and checksummed
content-derived filenames. Other platforms refuse this backend. Owner-provider and host-applier
ports exist for deterministic tests only.

## Production boundary

The native listener maps the five fixed exact-restore POST paths directly to a typed unsupported
response after its ordinary bearer-token check. Begin returns `REJECTED/no_restore_adapter`;
chunk, finish, commit, and lookup return `UNAVAILABLE/native_unavailable`. All responses state
`host_effect=not_started`. This route does not construct the staging store or dispatch a managed
callback. Transport identity headers are only request-matching inputs; they do not prove current
authority.

No production authoritative local owner-fence provider, exact-host restore adapter, independent
native recapture, profile mutation, or live restore test exists. The source-only engine is not
wired into production and must not be represented as native restore support.

## Verification

Run the focused owner and fixed-route tests:

~~~text
cargo test --locked --offline --package sts2-game-mod --test checkpoint_restore
cargo test --locked --offline --package sts2-game-mod-interop exact_restore_routes_refuse_before_managed_callback_or_staging
~~~

Run the repository's required Rust format, policy, Clippy, build, and workspace test gates from
`AGENTS.md`. All passed at the candidate source state:

~~~text
cargo fmt --all --check                                           exit 0
cargo run --locked --offline --package repo-policy -- --strict   exit 0
cargo build --locked --offline --workspace --all-targets --all-features   exit 0
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings   exit 0
cargo test --locked --offline --workspace --all-targets --all-features   exit 0
dotnet build experiments/managed-rust-interop/managed/ManagedInteropSpike.csproj --configuration Release   exit 0
dotnet run --project experiments/managed-rust-interop/workshop/WorkshopValidationProbe.csproj --configuration Release   exit 0
~~~

The managed commands used `/home/agent/.dotnet/dotnet` with the available ICU runtime directory
from the native-toolchain bundle. The focused restore suite passed 10 tests; the fixed native route
test passed 1 test. No native game or restore execution was performed.
