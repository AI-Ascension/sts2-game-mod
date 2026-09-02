# ADR 0013: Local profile export and import

- Status: Accepted for the standalone settings slice; host/runtime behavior unverified
- Date: 2026-09-02

## Context

The AI-Ascension settings tab already identifies a target profile and applies an explicit full
profile unlock. Users also need a controlled way to move that profile's saved progress between local
installations or profile slots without adding a settings framework, HTTP route, cloud service, or
repository artifact. Profile data is host-owned and may contain proprietary or user-specific state,
so the operation must remain visible, local, bounded, and reversible when installation fails.

## Decision

Add `Export profile` and `Import profile` actions to the standalone profile-settings panel. The
selected slot is the destination for import and the source for export. Export and import are refused
while a run is in progress. When the selected slot is active, the addon asks the host save manager
to write current progress before reading or replacing files.

The local archive format is a ZIP file with the `.sts2profile` extension. It contains exactly one
`ai-ascension-profile.json` manifest and files under the selected profile's `saves/` directory.
Manifest format `ai-ascension-sts2-profile`, version `1`, source profile ID, file count, and total
uncompressed bytes are required. Imports accept only profile IDs 1 through 3, a maximum of 8,192
files, and at most 256 MiB of uncompressed data. Paths must be safe relative paths below `saves/`;
reparse points are not followed during export or installed from an archive.

Export writes to a temporary file in the chosen destination directory and moves it into place only
after all entries and metadata are complete. Import validates and extracts into a system temporary
directory, then backs up the destination `saves/` directory and swaps the staged directory into
place. If the swap fails, the prior directory is restored when possible. Import requires an explicit
confirmation because it replaces saved progress, and successful import reports that the game must be
restarted to reload host-owned in-memory state.

The file dialog is local-only and user-directed. The archive path, save path, contents, and host
assemblies never enter the repository, addon package, HTTP surface, native ABI, MCP, gateway, or
harness. Status text and diagnostics use bounded categories without raw paths or exception details.

## Consequences

The settings panel provides a compact, discoverable profile transfer path while preserving the host
as the authority for profile selection and current-profile flushing. Cross-slot import is supported
because the manifest records the source slot but the UI controls the destination slot. The selected
profile's save directory is intentionally replaced as one unit; unrelated profile slots are not
touched.

The managed source and exact-host reference build can verify API names and compile-time shape, but
they do not prove the file dialog rendering, host path resolution, archive round trip, rollback, or
restart reload. Those behaviors require a separately authorized disposable-profile runtime test.

## Alternatives and scope

Direct arbitrary-file copying was rejected because it could capture unrelated host data and would
make path traversal and partial writes harder to bound. A network or cloud transfer was rejected
because it would expand the addon into a service and create a new credentials/privacy boundary.
Editing save fields in place was rejected because the host owns the save format; this decision moves
the host-owned save directory as an opaque, validated local unit and requires a restart afterward.
