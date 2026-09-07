# Linux Steam callback ABI proof

`create-item-result-linux.cpp` is a compile-time proof for the callback used
by the first-party `CreateItem` helper. Run `verify-sdk-abi.sh` with a clean
checkout of Valve's public `source-sdk-2013` headers at commit
`b8cfb12c0e083a2ef5b2f9f9b50f3902fa034474`:

```text
tools/workshop/lifecycle/abi-proof/verify-sdk-abi.sh /path/to/source-sdk-2013
```

The headers select `VALVE_CALLBACK_PACK_SMALL` for Linux. Their
`CreateItemResult_t` is callback ID `k_iSteamUGCCallbacks + 3` (`3403`) with
`EResult` at offset 0, `PublishedFileId_t` at offset 4, the legal-agreement
boolean at offset 12, and total size 16 bytes. The managed declaration in the
versioned CreateItem helper therefore uses `StructLayout(LayoutKind.Sequential,
Pack = 4)` and a one-byte boolean field. A 24-byte or Pack=8 declaration is
rejected by both the managed runtime guard and this native proof.

This proof checks the public SDK header contract only. It does not load a
Steam library, create an item, or contact Steam. The binary output is placed
in a temporary directory and is never committed. The SDK headers themselves
remain an operator-supplied external input and are not copied into this
repository.
