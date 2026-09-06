# ADR 0026: Intended-addon load preparation

- Status: Accepted operator boundary; host verification recorded separately
- Date: 2026-09-06
- Owner: sts2-game-mod launch tooling

After a patch or first launch, the host can require Load Mods consent before loading the addon.
An unloaded addon cannot automate that screen. The owned launch path may instead prepare the
host's public serialized mod settings while the explicitly selected game is stopped.

The Rust `sts2-game-mod-loading` operator tool accepts an explicit settings file and addon
directory. It validates the intended manifest and a narrow directory artifact allowlist. It
enables only `AIAscensionSTS2GameMod` with source `mods_directory`, and records native global
mod-loading consent. When consent was false, another enabled mod-list entry causes rejection
because the same consent could activate that other addon. Existing unrelated settings and
disabled mod entries are preserved. This tool is intended for the owned isolated fixture; it
does not inventory Steam Workshop subscriptions or discover a user's active account/profile.

Read-only plan mode returns hashes and whether a change is needed. Explicit apply requires the
reviewed input digest, exclusive preparation lock, private backup, bounded JSON parsing without
duplicate keys, rejection of linked/reparse paths, atomic file replacement, and byte readback.
Unknown mod-settings shapes fail before replacement. A repeated unchanged apply makes no backup
and preserves the original bytes. Backup settings remain private operator data.

The tool does not start or stop a game. The launch owner must keep the selected game stopped
throughout preparation, just as for addon installation. The Windows/WSL `dev-cycle.sh` integrates
this operation behind `--mod-settings PATH`, after selected-process stop verification and verified
addon installation, before launch. Native Linux and Windows operators can invoke the same Rust
tool under their guest-agent launch owner. No OS input automation or provider call is involved.

This is an operator filesystem concern outside the core mod composition and managed loader.
It adds no runtime route, ABI symbol, game rule, automatic profile discovery, or persistent loader
override. Compatibility is limited to the observed v0.107.1 `mod_settings`, `mods_enabled`,
`mod_list`, `id`, `source`, and `is_enabled` serialization contract.
