# Decision registry

Each normative decision has one four-digit identifier matching its filename and `# ADR NNNN:`
heading. Historical decisions retain their identifier; changing status does not free a number.
Repository policy rejects duplicate identifiers and mismatched numbered filenames/headings.
Old URLs may retain thin `# Moved:` redirects to the normative document; redirects contain no
decision body and do not reserve another identifier. Local link validation checks their targets.

| ID | Decision |
| --- | --- |
| 0001 | [Managed loader and Rust native boundary](0001-managed-loader-rust-native-boundary.md) |
| 0002 | [Initial game compatibility baseline](0002-initial-game-compatibility-baseline.md) |
| 0003 | [Rust-first implementation](0003-rust-first-with-managed-loader-exception.md) |
| 0004 | [Non-destructive scaffold](0004-non-destructive-target-scaffold.md) |
| 0005 | [Ownership and dependency direction](0005-mod-ownership-and-dependency-direction.md) |
| 0006 | [Sixth-target protocol scope](0006-current-sixth-target-protocol-scope.md) |
| 0007 | [Wave2 initialization](0007-wave2-codebase-initialization.md) |
| 0008 | [Minimal POC seam](0008-minimal-poc-game-mod-seam.md) |
| 0009 | [Runtime addon load smoke](0009-runtime-addon-load-smoke.md) |
| 0010 | [Runtime-v1 host probe](0010-runtime-v1-host-probe.md) |
| 0011 | [Optional ModConfig settings, historical](0011-optional-mod-settings.md) |
| 0012 | [Runtime listener settings](0012-runtime-listener-settings.md) |
| 0013 | [Ephemeral runtime-session launcher](0013-ephemeral-runtime-session-launcher.md) |
| 0014 | [Runtime-v2 fake boundary](0014-runtime-v2-fake-boundary.md) |
| 0015 | [First-party Workshop package](0015-steam-workshop-first-party-package.md) |

The last two decisions formerly both used 0011. Their content is unchanged apart from identifier
and references, and the old paths remain redirects. The original 0011 settings record remains
historical rather than being silently rewritten to claim the current built-in implementation.

For pending-branch reconciliation, reserve 0016/0017 for the earlier gameplay catalog/host proposal,
0018 for the newer neutral Runtime-v3 host bridge, and 0019 for repeat-seed practice replay.
These reservations are not accepted decisions or evidence that those branches have merged.
