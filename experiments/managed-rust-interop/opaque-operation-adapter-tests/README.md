# Opaque-operation native-message adapter probe

This source-only component defines a deliberately host-type-free native-message seam. It carries
one bounded opaque original operation ID and opaque bounded payload through a host-side admitted
dispatch boundary. Only a settled result returned by that boundary can produce a settlement
witness; client input has no witness field. Replies use the same `IAuthenticatedPeerLink` object
that submitted the operation. A repeated original ID is replayed only to its original peer and is
never dispatched as new work.

Run the deterministic component probe with the pinned .NET SDK:

```text
dotnet run --project experiments/managed-rust-interop/opaque-operation-adapter-tests/OpaqueOperationAdapterProbe.csproj --configuration Release
```

The probe itself does not implement `INetMessage`, register a host handler, choose a
`MessageTypes` ID, or prove authentication, discovery, serialization, host dispatch, or peer
delivery in STS2. The separately source-derived loader adapter subscribes when the normal co-op
composition path exposes a native service and queues host-carried work for a game-thread
dispatcher. Read-only metadata inspection found a concrete
serializer/type-ID pipeline, but no evidence that a mod-defined message type is safe or admitted.
An exact-host, two-peer trace remains required before relying on this seam at runtime.
