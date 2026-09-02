# ADR 0011: Optional ModConfig settings bridge

- Status: Accepted for the managed addon settings slice; host/runtime behavior unverified
- Date: 2026-09-02

## Context

The native STS2 Installed Mods screen currently exposes mod enablement only. Its checkbox owns
whether `AIAscensionSTS2Poc` is enabled; the folder action and mod details panel do not provide a
project-owned surface for configuration. The addon nevertheless has two useful diagnostic and
profile actions that need explicit user controls:

- the existing Rust ABI debug overlay; and
- the existing full-profile unlock operation, which changes persistent profile progress.

The addon is currently a DLL-only package with `has_pck: false`. Adding a framework DLL, a PCK, or a
hard manifest dependency solely to expose settings would change package identity and deployment
responsibility. The settings integration must therefore remain optional and preserve the existing
loader and command-line behavior when no framework is installed.

The public ModConfig API was inspected at commit
[`639eb97fa7824e94a43339913c51433117207d05`](https://github.com/xhyrzldf/ModConfig-STS2/tree/639eb97fa7824e94a43339913c51433117207d05),
including `ModConfig.ModConfigApi`, `ModConfig.ConfigEntry`, and `ModConfig.ConfigType`. The
source-derived API includes three- and localized four-argument registration overloads and the
`Toggle`, `Button`, `Header`, and `Separator` entry types. The inspected upstream project is
attributed to PiPiFanDev and is MIT licensed. No upstream source or binary is bundled in this
repository or the addon package.

## Decision

Add an original, narrow reflection bridge to the optional ModConfig API. After a safe Godot
`ProcessFrame`, the bridge discovers only the verified public ModConfig types and compatible public
registration/value methods. When available, it registers the addon under the stable mod ID
`AIAscensionSTS2Poc` on the framework's `Settings -> Mods` page. The bridge has no compile-time
assembly reference, NuGet dependency, PCK, or manifest dependency.

The bridge must register these entries and no other user controls:

| Key | Label | Type | Default | Behavior |
| --- | --- | --- | --- | --- |
| `show_debug_overlay` | `Show debug overlay on launch` | Toggle | `false` | After the managed loader and Rust ABI smoke call succeed, show the existing diagnostic overlay on the next launch. It does not change gameplay. |
| `unlock_all_on_next_launch` | `Unlock all profile content on next launch` | Toggle | `false` | After profile initialization, schedule the existing full unlock once, then clear the setting only after the profile save succeeds and the reset is accepted. |
| `apply_full_profile_unlock_now` | `Apply full profile unlock now` | Button, when safely supported | n/a | Schedule the same readiness-checked, main-thread unlock path without enabling the persistent launch toggle. |

The full unlock is an explicit profile mutation. It covers all cards, relics, potions, events,
acts, monsters, epochs, every character's maximum ascension of `10`, and multiplayer maximum
ascension of `10`. It does not change achievements, preferred ascension values, runtime tokens,
ports, bind addresses, HTTP routes, MCP actions, AI policy, seeds, or arbitrary save data. The
toggle and button descriptions must warn that profile progress changes. The button is omitted if
the verified API cannot safely construct a writable callback entry.

Settings are hydrated only after deferred registration. Setting-dependent startup work therefore
runs from the bridge-ready callback. The standalone `--debug` argument remains an exact,
case-sensitive override. The standalone `--ai-ascension-unlock-all` argument remains an exact,
standalone, case-insensitive override. If ModConfig is absent, incompatible, or fails during
registration, the bridge invokes its ready continuation, preserves native loading and ABI smoke
behavior, and leaves both command-line fallbacks available.

## Trust, safety, and secret boundaries

The game host remains authoritative for profile readiness, legal profile mutation, main-thread
affinity, and persistence. The bridge only registers known entries and forwards typed values or a
known manual-action callback; it does not edit save files directly or introduce general-purpose
host reflection. The profile action uses one queued attempt, waits on the initialized profile, has
a bounded readiness window, and shares the same path for launch and manual requests. A failed
operation retains the one-shot request and emits a bounded diagnostic so a later launch can retry.

The optional framework is treated as an external integration boundary discovered by exact public
type and method shape. This is source-derived compatibility evidence, not proof that an arbitrary
installed assembly is trustworthy. No credentials, bearer tokens, environment secrets, private
paths, host assemblies, game binaries, saves, profiles, or raw exception/profile contents are
settings values or package inputs. Runtime listener controls and authentication belong to a
separately scoped runtime change.

## Alternatives and scope

`ModManagerSettings` was rejected for this PR because its primary UX adds a settings button directly
to each native Installed Mods row and its registration/deployment model assumes a framework
dependency and PCK-oriented mod identity. That is a valid candidate for a future direct-row UX
evaluation, but it conflicts with this addon’s DLL-only optional-integration boundary.

BaseLib/SimpleModConfig was rejected for this PR because it also introduces a framework dependency
and its own configuration contract, rather than the narrow optional reflection seam selected here.
Neither framework is bundled, referenced, or required by this change. The native enablement
checkbox is not duplicated as a mod setting.

## Evidence and consequences

- **Confirmed:** the target remains a DLL-only package contract with `has_pck: false`; the bridge
  uses no hard framework reference or manifest dependency.
- **Source-derived:** the ModConfig public API shape and MIT provenance at the pinned upstream
  commit above; the implementation is original bridge code.
- **Proposed/accepted:** the stable keys, labels, defaults, deferred registration, fail-open
  behavior, CLI fallbacks, shared profile-action path, and success-gated one-shot reset described
  here.
- **Unverified:** managed compilation against the operator-supplied proprietary host assemblies;
  ModConfig settings rendering; callback invocation; persisted-value hydration; profile mutation;
  and compatibility across other STS2 versions, platforms, or architectures. Rust, policy, and
  source-level checks do not establish those host/runtime claims.

The consequence is an additive settings path that does not prevent the addon from loading when the
optional framework is unavailable. It also makes profile mutation visible and opt-in, while
keeping the native Installed Mods checkbox as the sole enablement control.

## Follow-up

A future decision and evidence set may evaluate `ModManagerSettings` or another maintained approach
for a settings button directly on the native Installed Mods row. The runtime-controls follow-up is
specified separately in [`0012-runtime-listener-settings.md`](0012-runtime-listener-settings.md);
token handling and MCP actions remain outside that follow-up. It is not part of this historical
settings-bridge decision.
