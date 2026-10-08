# ADR 0077: Static game-facts Protocol producer

- Status: Accepted source-only mapping boundary; owner inventory and production source remain pending
- Date: 2026-10-07
- Tracking: sts2-game-mod #207

## Context

GameMod has a locally validated caller-supplied inventory model, but the inspected source contains
no production inventory builder or supported installed-host rule/input source. The existing synthetic fixture is
not evidence of native rule coverage. The Protocol owner has accepted the typed
game-facts-reference-v1 profile at merge 1879e0b00993c9b6d662d1f015ef59d6810d82de, schema
digest 3065a2ff96e6e5af628b3d5cb43b8f2db4232ae4c4dc71324b973419908a2985. This decision adds only
a typed static mapping seam and does not claim that any rule is currently supported by a real source.

## Decision

GameMod advances its existing sts2-protocol dependency to the accepted merge and exposes a
stateless Rust adapter for game-facts-reference-v1. It adds no route, listener, managed callback,
host provider, Core catalog, rule evaluator, formula, value, or live observation. The adapter
accepts caller-supplied inventory and manifest values and validates the request, bounded response,
exact query/correlation echo, and Protocol response join.

The capabilities response reports only the fixed profile limits: 16 rule IDs, 64 inputs per rule,
256 unsupported combinations, and 262,144 message bytes. Its snapshot policy is null. These are
schema limits, not a supported-rule or live-capability declaration.

For static metadata, only caller-labeled SourceDerived records with a shape-valid, non-null per-input
source reference can map to Found. This is synthetic mapping behavior, not source authentication.
Observation is null and contains no value. Unit labels map by exact,
case-sensitive names; multiplier numerator and denominator stay separate named inputs. Unsupported
or unrepresentable inputs never produce partial records. Static Confirmed and all live queries
return MissingCapability. Proposed, Inferred, and Unverified records remain Unsupported. A rule ID
absent from the supplied subset returns query-level MissingCapability because this adapter has no
complete Core membership catalog. Declared unsupported interactions are copied only when all
members are requested; a subset that hides an interaction leaves affected rules unsupported.

ADR 0038 admits ContentManifest.inventory_revision as the content identity. The new manifest-bound
inventory constructor derives the build and complete owner-local cursor from a supplied
ContentManifest and privately records inventory_revision and locale; the caller supplies mode
separately. The adapter compares that recorded identity, build, cursor and request ID against the
supplied manifest value, and requires the privately captured locale to agree with both the supplied
manifest and request locale. ContentManifest has public fields and derives Clone, so this check
establishes caller-value consistency only. It does not authenticate producer origin, verify a source
token, or prove that facts and manifest came from one coherent owner read. The caller/source
integration owns that evidence boundary. Synthetic mapper fixtures do not establish it.

Source references are bounded opaque ASCII tokens. They are never opened as paths or included in
errors. The mapper rejects path-like and exception-shaped tokens and refuses unsupported units
without aliases or guessed conversion.

## Consequences and acceptance limits

The existing FactsInventory::new constructor remains unbound for legacy and synthetic callers.
The additive new_for_manifest constructor records revision and locale for consistency checks; it is
not an authenticity token. No static positive response from caller-provided synthetic values proves
production support. A future owner-source integration must supply an agreed rule set, exact ordered
inputs and units, real per-input provenance, build/mode, and coherent manifest/facts evidence before
any production rule claim is made.

Conformance tests use synthetic public manifest values and synthetic source tokens. They verify
mapping, response byte bounds, refusal semantics, ordering, unsupported interactions, and request echo only.
They do not claim exact-host parity, native execution, a populated production inventory, consumer
adoption by Gateway/MCP/Harness, or completion of #207. Those remain separate acceptance evidence.
Issue #207 stays partial and open until owner-supported inventory/source and coherent exact-build
evidence are available.
