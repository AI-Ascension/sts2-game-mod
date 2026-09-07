# Steam Workshop lifecycle tooling

This directory contains the versioned operator workflow for the first-party
STS2 managed addon. Package verification is local and read-only. Upload,
update, and rollback require `--execute-public-write`; download requires
`--execute-local-download`; native subscription requires `--execute-ugc`.
Without the matching flag each command prints a plan and makes no Steam call.

The shell wrapper resolves `dotnet` from `PATH`; set `DOTNET_BIN` to the pinned
.NET 9 executable when it is not on `PATH`.

Every executed operation claims a new JSON journal with an exclusive create,
flushes it before starting the external process, and writes the result through
an atomic replacement. A non-zero process exit, timeout, exception, or failed
postcondition is recorded as `outcome: unknown`. Keep that journal and
reconcile the destination before attempting another operation; reusing a
journal is rejected. Logs are created beside the journal with mode 0600 and
are never printed by the tooling.

## Verify a package

`package-platform-item.sh` stages exactly three platform payload files,
`sts2-workshop-manifest.json`, `SHA256SUMS`, and an operator-only VDF beside
the package. The package manifest must contain the assigned nonzero Workshop
item ID before a public operation.

```text
tools/workshop/lifecycle/verify-package.sh \
  --package-dir /path/to/linux-package \
  --platform linux-x86_64 \
  --app-id 2868840 \
  --item-id 123456789
```

The verifier rejects symlinks, extra entries, malformed or duplicate JSON
properties, out-of-order checksum rows, digest/size mismatches, and platform
or item-ID mismatches. It does not contact Steam.

## Create one item

`ugc-create-item/UgcCreateItemHelper.csproj` builds a separate native helper.
The Linux x86-64 helper runs only on a Linux x86-64 process and creates an empty Community item.
It checks the exact package
with `published_file_id: 0`, hashes the selected `libsteam_api.so`,
requires `SteamAppId` and `SteamGameId` to be inherited at process start,
checks the loaded module identity and account/interface preconditions, and
persists a pre-call journal before `CreateItem`.

The helper uses the proven Linux Valve callback declaration: callback
`k_iSteamUGCCallbacks + 3` (`3403`), size 16, `EResult` at offset 0,
`PublishedFileId_t` at offset 4, and the one-byte legal agreement field at
offset 12. The ABI proof also asserts the numeric callback ID `3403`. A callback result is persisted
with mode 0600 before the helper exits. A failed
transport, result retrieval, timeout, or exception is `unknown`, including
when a nonzero API call handle was already issued.

Run the public SDK proof separately:

```text
tools/workshop/lifecycle/abi-proof/verify-sdk-abi.sh /path/to/source-sdk-2013
```

The helper source does not include proprietary game files, Steam credentials,
or generated native binaries. `test-ugc-create-item.sh` compiles a synthetic
library into a temporary directory and exercises success, failed transport,
timeout, and journal collision without contacting Steam.

## Upload, update, rollback

The VDF is verified against the same package and nonzero item ID before any
SteamCMD process starts. Use a dedicated account environment value; this
workflow never accepts or stores a password:

```text
STEAMCMD_USER=<account> tools/workshop/lifecycle/upload-item.sh \
  --steamcmd /path/to/steamcmd \
  --package-dir /path/to/linux-package \
  --platform linux-x86_64 --app-id 2868840 --item-id 123456789 \
  --vdf /path/to/linux-package.vdf \
  --journal /path/to/journals/upload-123456789.json \
  --execute-public-write
```

`update-item.sh` uses the same checked `workshop_build_item` operation for a
new package and an existing item. `rollback-item.sh` uses a preserved
known-good package, manifest, and VDF with the same item ID. Never overwrite
the known-good package while rolling back.

## Subscribe and download

`subscribe-item.sh` requires a separately reviewed native runner and exact
runner/library SHA-256 values. It sets the AppID environment before starting
the runner and accepts success only when the process exits zero and its JSON
result says `outcome: succeeded`; SteamCMD download output alone is not proof
of subscription.

`download-item.sh` requires a clean SteamCMD root and verifies the exact
five-file package in `steamapps/workshop/content/2868840/<item-id>` after the
process exits. A stale or malformed destination is `unknown` and requires
reconciliation. Neither command claims game discovery or loading; those are
separate host evidence gates.
