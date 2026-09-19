# ADR 0061: Owner-local read-only shop inventory, service, and restock reference

- Status: Proposed; source-only owner boundary
- Date: 2026-09-19
- Tracking: game-mod #101
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md),
  [ADR 0039](0039-field-availability-and-completeness-boundary.md),
  [ADR 0058](0058-settings-reference.md),
  [ADR 0060](0060-run-configuration-reference.md)

## Context

Issue #101 requires what a shop actually offers and why it costs what it costs: the inventory
entries with their definition references, displayed price and currency, stock state, sale and
stacked-discount state, and purchase action and capacity or eligibility restrictions; the services
with their exact-build identity, cost, selection domain, limits, eligibility, and prospective
change; the static pricing contributors and reference formulas; and the restock rules that produce
a new inventory generation. A reported kind must not contradict the definition it resolves to, a
documented formula must stay distinct from the observed amount, a blocked purchase reason must stay
separate from the price it would cost, and a stock reference from an earlier restock must be refused
rather than answered with current stock. The boundary must also be read-only: no operation may buy,
sell, restock, or spend gold.

This decision defines an owned source boundary. It does not claim a live shop read, a native
extractor, a transport route, a gateway/MCP adapter, exact-host comparison, or a new purchase API.

## Decision

`crates/game-mod/src/shop_reference` owns an immutable `ShopCatalog` produced by
`ShopCatalogProducer` from a bounded `ShopCatalogSource`. Static results bind to the existing
content-manifest cursor, the exact locale, and the owner-local producer version
`game-shop-reference-producer-v1` through `ShopCatalogBinding`. Family coverage is explicit
(`ShopFamilyState`), and a family the source cannot project is refused rather than reported as an
empty catalog. A non-clonable `ShopCatalogReader` fences listing and exact lookup by locale,
visibility scope, and single-use continuations, and one reader never accepts another reader's
continuation.

### Reported kind versus resolved definition

`ShopItemKind` and `ShopServiceKind` close the supported inventory and service families at card,
relic, potion, service, and owner-defined or unsupported tokens. An entry whose definition resolves
to a card, relic, potion, or service family must report exactly that kind: a mismatch is rejected as
`KindDisagreesWithDefinition` with the reported kind and the resolved family, so no supported item
can stay hard-coded `Unknown`. A reference that names no manifest family is `None` rather than
assumed, and a reference to an absent identity is a `DanglingReference` while a record more visible
than its target is a `HiddenReferenceLeak` whose restricted identity is deliberately omitted.

### Price, discount, and blocked reason

`ShopPrice` separates the currently observed `displayed` amount, the `documented` reference price,
the named `contributors` in source order, the documented `rounding`, the retained
`unresolved_inputs`, and an explicit `blocked_reason`. An amount the source reports as a formula
stays a `ShopNumber::Formula` instead of being folded into an invented total, so
`ShopPriceComparison::of` reports an unavailable comparison rather than substituting a number. A
sale carries its own percentage and a stack carries its tiers plus the combined percentage; a
percentage outside 0–100, a stack with fewer than two tiers, and a tier larger than its combined
percentage are each `InvalidDiscount`. `ShopBlockedReason` names why a purchase would be refused
now — unaffordable, full slot, not eligible, sold out, capacity exhausted, restock pending — without
replacing the displayed amount.

### Stock and restock generations

`ShopStockState` keeps in-stock quantity, sold-out, restricted, not-offered, and unknown distinct, and
an `InStock` quantity of zero is rejected. A `ShopRestockRule` carries the generation it produces and
must strictly increase across the shop's rule chain, so `ShopStockReference` can be fenced to the
exact generation that produced it: `ShopCatalogReader::stock_for_reference` refuses an earlier or
newer generation with `StaleStockReference` instead of answering with current stock.

### Read-only by construction

`ShopCatalogSource::read_catalog` takes `&self`, and the trait declares no purchase, sale, restock,
mutator, selector, or admission method: nothing on this boundary can buy an item, remove it, restock
a shop, spend gold, or admit a run, and the slice retains no currency balance it could change. A
transient `ShopPurchaseActionReference` is rejected as `InvalidInput("purchase_action")`, so a live
action can never enter the static slice. Regressions assert an exact source-read count, an unchanged
retained stock state and entry count across list, get, and stock reads, refusal of a reused or
foreign continuation, scope withholding without placeholders, and generation-fenced stock; the
kind-agreement, duplicate-service, generation-fence, and source-read assertions were each verified
to fail under an isolated production mutation.

## Consequences

An owner-side consumer can inventory a supported build's shop inventory, service declarations,
pricing contributors, and restock generations with explicit non-values and refused stale references,
without reading or changing game state. Because the boundary is source-only, native extraction, live
shop observation, purchase or restock execution, and exact-host comparison remain unverified.
