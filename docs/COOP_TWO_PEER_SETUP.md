# Native co-op two-peer package and trace procedure

Status: proposed operator procedure. It is not evidence that either package loads, discovers the
message subtype, forms a session, or settles an action.

## Build receipts

Use an empty directory outside the repository and outside either game installation for each
platform. The package command stages exactly three files and prints their SHA-256 digests.

```text
bash experiments/managed-rust-interop/package-runtime-addon.sh \
  --platform windows-x86_64 \
  /path/to/Slay\ the\ Spire\ 2/data_sts2_windows_x86_64 \
  /external/artifacts/coop-native/windows-x86_64

bash experiments/managed-rust-interop/package-runtime-addon.sh \
  --platform linux-x86_64 \
  /path/to/Slay\ the\ Spire\ 2/data_sts2_linux_x86_64 \
  /external/artifacts/coop-native/linux-x86_64
```

The Windows receipt must contain `AIAscensionSTS2GameMod.dll`,
`AIAscensionSTS2GameMod.json`, and `AIAscensionSTS2GameModNative.dll`. The Linux receipt changes
only the final filename to `libAIAscensionSTS2GameModNative.so`. Record the source commit, exact
host-assembly hashes, command exit status, and all three output hashes. Do not place generated
artifacts, host assemblies, saves, or profiles in this repository.

Before any authorized install or launch, produce the bounded review plan with the source-owned
preparation script. It verifies distinct disposable Windows game copies and byte-identical package
receipts, then writes only external hashes, `host.env`, `client.env`, and a no-launch plan:

```text
bash experiments/managed-rust-interop/prepare-native-coop-two-peer.sh \
  --host-game-dir /external/disposable-host \
  --client-game-dir /external/disposable-client \
  --host-package-dir /external/artifacts/windows-host \
  --client-package-dir /external/artifacts/windows-client \
  --host-profile-id disposable-host-profile \
  --client-profile-id disposable-client-profile \
  --port 33771 --client-id 2 \
  --output-dir /external/two-peer-plan
```

The script does not copy addon files, read profiles/saves, open a socket, or start a process.
The host's native ID is assigned by `StartENetHost`, so the client plan intentionally does not
guess it; the client observes the connected `HostNetId` through first-party `JoinFlow` and checks
it before client-lobby admission. An operator may separately pin a previously observed ID only
when restarting a client against that same host.

## Disposable two-peer trace

An authorized operator must perform these steps only with two fresh copies of the game and two
fresh profiles, keeping the original installations and profiles stopped and unchanged.

1. Hash each source installation and profile, copy each to a distinct disposable location, then
   hash the copied baselines. Record the paths only in private operator evidence.
2. Require matching package manifests and matching managed/native package receipts for the two
   chosen platform copies. Install only the three named files into each disposable copy's `mods`
   directory, preserving a reversible backup of any replaced named file.
3. Start the designated host copy, then start the client copy and establish one native session.
   Record the authenticated native host ID and client ID from host-owned diagnostics; do not use a
   gateway credential, route context, or a client-supplied peer string as identity evidence.
4. With the peers converged at the same host generation and digest, submit one map vote or local
   action from the client. Preserve the client original operation ID, exact carrier bytes, host
   dispatch/queue ID, host-issued receipt, and the reply's authenticated target ID.
5. Prove that the host action is invoked at most once for that original ID, that a retry returns
   the same host result only to its original peer, and that a foreign peer cannot obtain that
   receipt. Capture a fresh post-action observation showing the expected settled generation and
   convergence.
6. Exercise one controlled disconnect/rejoin only after the action trace is complete. Record the
   rejoin request and outcome separately; do not treat a reconnect as proof of an earlier action.
7. Stop only the disposable copies, compare the original installation/profile hashes to their
   baselines, and retain package and trace evidence outside the repository.

The current source deliberately accepts remote opaque carrier votes only for canonical map
proposals. A client `rejoin_request` takes the existing local-client recovery route; remote opaque
carrier rejoin requests are invalid because a host cannot create another process's connection.
A source build, package receipt, or HTTP success is not a substitute for the host-issued trace
above.
