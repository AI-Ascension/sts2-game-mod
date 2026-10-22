# ADR 0083: Core catalog admission for static game facts

- Status: Accepted source-only signature boundary; native extraction and parity remain pending
- Date: 2026-10-22
- Tracking: game-mod #278; partial component of game-mod #207 and game-core #13

## Problem

ADR 0077's generic mapper deliberately accepts caller-supplied opaque rule IDs and input labels.
A valid manifest binding establishes value consistency, but does not establish agreement with
Core's ordered input catalog. Unknown rules or different input signatures can therefore be mapped
without being Core declarations. Changing that existing mapper would break its generic callers.

## Decision

Add the immutable `CoreFactsInventory` constructor and the explicit
`GameFactsReferenceV1Adapter::respond_admitted` entrypoint. The constructor first applies the
ordinary manifest and inventory validation, then reads `sts2_game_core::rule_inventory()` from an
exact source dependency. It requires each rule ID and the entire ordered input signature to agree:
input count, case-sensitive name, exact unit, and the required/conditional flag. Unknown availability
does not agree with either flag. The constructor refuses the entire inventory on disagreement;
it never reorders inputs, infers a unit conversion, or substitutes a second production catalog.

The representation version derives from Core's `RULES_REFERENCE_VERSION` (currently 2) and the
encoding derives from Protocol's accepted `GAME_FACTS_REFERENCE_V1_PROFILE`. The legacy synthetic
modifier signature still has its single `multiplier` input. Rational numerator/denominator inputs
belong only to the records that actually declare them.

The admitted inventory contains a privately held `FactsInventory`, exposed only through a shared
reference. There is no mutable accessor or unchecked public conversion. Caller rule order, evidence
labels, source references and declared unsupported combinations survive unchanged. The admitted
adapter reuses the existing codec and mapper, retaining identical bytes for identical admitted
values. The generic constructor and `respond` entrypoint remain available with their prior behavior.

## Dependency and license review

Core is pinned to `5070070784d6b62dabf535a230bcdb46a054e3ef`, the merged #24 catalog source baseline.
Its entire `crates/core/src` tree is byte-identical to Core default `941939dc75e3305d409340552cb1bd2b0e2f98a5`
when this change was prepared. The later Core changes update dependency pins, including exact
serde_json 1.0.154 and sha2 0.11.1; those would conflict with the already accepted Protocol source's
exact serde_json 1.0.151 and sha2 0.11.0 requirements. The chosen immutable pin shares the existing
compatible graph and adds only the Core package to the lockfile. This decision does not update a
Protocol pin, schema, profile, digest, golden, conformance vector or dependency version.

The dependency direction is GameMod to the pure Core library. Core's reviewed package depends only
on serde_json and sha2; no host, filesystem, process or transport dependency is added to Core, and
this adapter calls only its inert catalog API. Core is MIT-licensed by the sts2-game-core
contributors. The complete license notice is retained in `THIRD_PARTY_NOTICES.md`; existing locked
dependency notices remain applicable. No implementation is copied into GameMod production source.

## Evidence boundary

Catalog admission is not owner-source authentication, applicability validation, coherent host
extraction or calculation parity. It accepts metadata supplied by the caller; it does not supply
observations, values, formulas, authoritative sources, native support or a populated owner inventory.
Core's synthetic fixture `Confirmed` labels are not copied into GameMod claims. The existing mapper
still refuses live and static Confirmed requests, missing per-input provenance, unavailable subset
rules and stale bindings. Unsupported interactions remain explicit. Capabilities continue to report
schema limits only, with no supported-rule or live declaration.

Independent synthetic fixtures enumerate all sixteen pinned signatures and exercise every input's
name, case, unit and availability, missing/extra/reordered inputs, unknown rules and ordinary bounds.
Wire tests use the accepted codec and request/response join, compare admitted and generic bytes,
and preserve evidence, provenance, interaction and binding refusals. These tests establish the
source-level catalog boundary only. #207 and game-core #13 remain open for the broader metadata
contract, owner-controlled capture, downstream adoption/replay and native extraction/parity evidence.
