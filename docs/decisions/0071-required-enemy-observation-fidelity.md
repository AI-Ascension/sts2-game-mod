# ADR 0071: Required enemy observation fidelity

Status: proposed; source-linked regression checks executed; native acceptance pending.

## Requirement and owner

Refs sts2-game-mod#84 and sts2-game-mod#96. Game-mod owns native reads, host-thread
translation and unavailable-read classification. No ownership or execution route changes.
The runtime-v4-expert producer calls the v3 observation path before adding expert fields.

## Defect

The native enemy collection projection converted collection-read failure into an empty
collection and a failed enemy read into a fabricated zero-HP enemy. Those results cannot
be treated as observed facts. Required health values were also silently clamped.

## Decision

Required enemy projection is all-or-unavailable. Preserve a genuine observed empty
collection, genuine zero values, encounter ordering, and owned record values. Refuse the
whole projection when enumeration, required getters or enumeration disposal fail; when
required identity/name/health violates existing wire bounds; when enemy identities repeat;
or when the existing MaxEntities bound would be exceeded. Bound allocation while reading,
not after constructing an oversized array.

Sanitize the internal refusal without retaining the original exception as an inner
exception. Existing observation boundaries remain responsible for their unavailable wire
response. No new response code, protocol field, profile, digest, or retry is introduced.

Intent visibility and intent calculation are optional information. Retain the existing
Unknown intent when rendering is hidden, absent, frozen, compound, or not calculable.
An internal Unknown record's zero damage/hit placeholders are not evidence of zero damage.
Do not read hidden targets, advance AI/RNG, or mutate the game to complete an observation.

## Compatibility and exclusions

Healthy in-bound observations retain their schema and ordering. Both v3 and v4 callers of
the shared producer now refuse malformed or unavailable required enemy reads. This is a
deliberate failure-path behavior correction, not a wire-schema change.

This slice does not certify native accessors or HTTP composition, add composite serialization,
change generation/fingerprint coverage, fix other nullable expert getters, or provide exact
card effects, modifier evaluation, lethal analysis or end-turn simulation. It neither closes
the referenced feature issues nor establishes gameplay improvement.

## Verification

The EnemyProjectionProbe links production IntentProjection, gameplay DTO and contract
sources. Original external API-shape doubles inject enumeration/getter faults and visible
intent cases. The optional intent logic is production code, not a duplicate implementation.
The two unchanged identity/clamping helpers supplied by another native partial are test-only
substitutes; native identity derivation is not validated by this probe.

The probe compiled against the pre-fix source and failed the overlarge-enumeration assertion.
With the correction, all 32 checks passed on Linux using .NET SDK 9.0.317. This establishes
source-linked regression behavior only; managed/Rust gates and copied-artifact checks remain required.
Exact-host required-read failures, read-only behavior, unavailable-response mapping and
unchanged successful gameplay remain separate authorized evidence gates.

## Next boundary

After this correction is validated, fix authoritative expert-only freshness and native
composite intent projection. Then prove producer-to-gateway-to-MCP-to-harness equality;
matching schema files alone do not establish it.
