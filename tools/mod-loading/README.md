# Prepare intended-addon loading

This offline operator tool prepares the native Load Mods settings before the addon loads. Use the
owned isolated fixture, with the selected game stopped. Choose the exact `settings.save` file;
the tool deliberately does not search accounts or infer an active profile. The addon directory
must contain only the supported AI-Ascension addon artifacts. Workshop discovery is outside this
tool's scope; do not use it to approve a mixed Workshop installation.

The [native fixture evidence](../../docs/evidence/mod-loading-preparation-20260906.md) records
Windows and Linux cleared-consent checks and their compatibility limits.

Build the tool with `cargo build --locked --release --package sts2-game-mod-loading`. On Linux,
run the binary under `target/release`; on native Windows, use its `.exe` counterpart.

```bash
sts2-game-mod-loading --settings-file /absolute/fixture/settings.save \
  --mods-dir /absolute/fixture/host/mods
```

This read-only plan reports `before_sha256`, `after_sha256`, and `changed`. After reviewing the
intended change, apply using the reported input digest and an existing private backup directory:

```bash
sts2-game-mod-loading --settings-file /absolute/fixture/settings.save \
  --mods-dir /absolute/fixture/host/mods --apply \
  --backup-dir /absolute/fixture/backups --expected-sha256 INPUT_DIGEST
```

The launch owner must keep the game stopped throughout both commands. Apply backs up the exact
old bytes and both digests, replaces the settings atomically, and checks the resulting bytes.
It changes native mod consent and the intended addon's enabled entry. Other settings remain
semantically unchanged; whitespace can change on the first update. Repeated unchanged preparation
preserves all bytes and creates no redundant backup.

The owned Windows/WSL installation cycle integrates preparation directly:

```bash
bash experiments/managed-rust-interop/dev-cycle.sh \
  --game-dir /absolute/fixture/host --mod-settings /absolute/fixture/settings.save
```

The cycle's existing live authorization and selected-process guards apply. No extra confirmation
prompt is added. `--dry-run` describes the operation without changing settings. `--no-backup`
does not disable the mod-settings backup. If the host changes the serialization shape or another
addon would be newly activated, preparation fails before changing the settings.

To restore, stop the same fixture, verify the backup's `before_sha256`, and restore
`settings.before.json` to the explicitly selected settings file. Do not overwrite newer unrelated
settings without reviewing them. Backups and settings must never be committed or published.
