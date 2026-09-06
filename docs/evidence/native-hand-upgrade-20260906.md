# Confirmed focused native hand upgrade, 2026-09-06

A normal Linux campaign reached Armaments and stopped with an unresolved parent card
play. The native selector was visible, but the adapter admitted no choices. That original
operation and its artifacts were preserved without retrying the mutation.

A separate, explicitly compiled disposable fixture reproduced the selector quickly.
It entered a weak native encounter and generated Armaments into the hand using a host
command. This altered test setup is not a normal campaign, seed replay, or training run.
The first fixture attempt faulted before any provider started. The revised fixture waits
for native hand readiness and uses the host's generic generated-card command.

Diagnostics reported UpgradeSelect, one retained parent, five visible clickable holders,
and zero holders with `InSelectMode=true`. Removing that incompatible holder check allowed
the unchanged native selection and confirmation path to run.

OpenAI Astra (`gpt-6-astra`) then played Armaments and selected a Strike. Both operations
settled through the normal harness/MCP/gateway/mod runtime. The second observation exposed
`Strike+` with `upgraded=true`; its parent action completed successfully. Astra subsequently
played the upgraded card. The immutable excerpt ends at the verified upgrade observation.

| Artifact | SHA-256 |
| --- | --- |
| Test addon with fixture enabled | `b6290970bfa5db05483976c799c6092f9ec4c6d9a095abcb622e5cc474dff169` |
| Two-operation verified excerpt | `4700345fbc9487d6e0fed4d32258ec5bb0409f8eee40a5d93491b3adc08aea0b` |

The test seed was `AIASCENSIONHANDTEST1`; its invocation manifest explicitly labels the
generated-card fixture. The game was visible in the read-only Linux viewer. Native control
signals drove selection, with no OS keyboard/mouse automation. No source, game assets,
credentials, or private profile contents are included in this record.

This verifies the focused Linux upgrade path. Normal campaign continuation, Windows native
hand selection, arbitrary multiple-card selectors, and other selection modes remain separate
checks. The fixture is excluded from normal addon builds.
