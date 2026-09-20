// SPDX-License-Identifier: MIT

/// Exact-build legal-action family an action belongs to.
///
/// The named variants are the families the audited source exposes; anything else is carried
/// explicitly as a custom, unsupported, or unknown family rather than folded into one of them.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionKind {
    /// Playing a card from hand.
    PlayCard,
    /// Drinking a potion.
    UsePotion,
    /// Ending the current turn.
    EndTurn,
    /// Performing a rest-site option.
    Rest,
    /// Buying an offered shop item or service.
    ShopPurchase,
    /// Answering an event, shrine, or dialog choice.
    EventChoice,
    /// Using an in-run relic.
    UseRelic,
    /// Owner-defined legal-action family.
    Custom(String),
    /// A family is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the legal-action family.
    Unknown,
}

/// Operation whose legal-action frame owns the action.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionParentOperation {
    /// The combat turn loop.
    CombatTurn,
    /// A reward claim.
    RewardClaim,
    /// A shop visit.
    ShopVisit,
    /// A rest-site visit.
    RestSite,
    /// An event, shrine, or dialog.
    EventChoice,
    /// A map or node selection.
    MapNavigation,
    /// Owner-defined parent operation.
    Custom(String),
    /// Source could not classify the parent operation.
    Unknown,
}

/// Resolved availability of one action or one of its targets.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionEligibilityState {
    /// The action may be dispatched now.
    Available,
    /// The action exists but is currently refused for a stated reason.
    Unavailable,
    /// The host could not resolve the action's availability.
    Unknown,
}

/// Host reason one action or target is refused.
///
/// The first five variants are the acceptance cases this slice must explain: an unmet resource,
/// an invalid or dead target, a full capacity, a disabled option, and a selection the action still
/// requires. A refusal the audited source cannot classify stays `Unknown` rather than being folded
/// into one of them.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionRefusalReason {
    /// A required resource, cost, or charge cannot currently be paid.
    InsufficientResource,
    /// The named target is not a legal target for this action.
    InvalidTarget,
    /// The named target exists but is no longer alive.
    DeadTarget,
    /// A bounded destination has no free capacity.
    FullCapacity,
    /// The action is present but disabled by the current mode or state.
    DisabledOption,
    /// The action still requires the caller to complete a selection.
    SelectionRequired,
    /// The action does not apply to the current mode.
    WrongMode,
    /// A stated requirement is unsatisfied.
    RequirementUnsatisfied,
    /// No supported extractor describes the refusal.
    Unsupported,
    /// The reason exists but must not be revealed.
    Withheld,
    /// Owner-defined refusal reason.
    Custom(String),
    /// Source could not classify the refusal.
    Unknown,
}

/// Kind of one bounded resource a cost contributor draws on.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionCostKind {
    /// Combat energy.
    Energy,
    /// Health paid as a cost.
    Health,
    /// Block consumed as a cost.
    Block,
    /// Gold paid as a cost.
    Gold,
    /// A card consumed, exhausted, or moved by the cost.
    Card,
    /// A potion charge or slot.
    PotionCharge,
    /// A relic charge.
    RelicCharge,
    /// Owner-defined cost kind.
    Custom(String),
    /// Source could not classify the cost kind.
    Unknown,
}

/// Coarse class of a target restriction an action imposes.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionRestrictionKind {
    /// The action needs a living target.
    RequiresLivingTarget,
    /// The action needs a legal target of its own family.
    RequiresValidTarget,
    /// The action needs free capacity at a bounded destination.
    RequiresFreeCapacity,
    /// The action needs an affordable resource.
    RequiresAffordableCost,
    /// The action needs a completed selection first.
    RequiresSelection,
    /// The action refuses one named target family.
    ForbidsTarget,
    /// Owner-defined restriction kind.
    Custom(String),
    /// Source could not classify the restriction kind.
    Unknown,
}

/// Family of a typed action reference.
///
/// A reference names the family it resolves in, so a manifest lookup never guesses which family an
/// identity belongs to; a family the source cannot name stays `Unknown` rather than being folded
/// into a neighbour.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionReferenceKind {
    /// A legal-action definition.
    Action,
    /// A preview effect or rule definition.
    Effect,
    /// A selection the action still requires.
    Selection,
    /// A card definition or card instance.
    Card,
    /// An enemy definition or enemy instance.
    Enemy,
    /// A co-op player identity.
    Player,
    /// A relic definition or relic instance.
    Relic,
    /// A potion definition or potion instance.
    Potion,
    /// A bounded resource such as energy, gold, or block.
    Resource,
    /// A status or power.
    Status,
    /// A pile, slot, or other bounded destination.
    Destination,
    /// Any other manifest family named by the source.
    Content {
        /// Manifest entity family.
        entity_kind: String,
    },
    /// Source could not classify the reference.
    Unknown,
}

/// How confidently a preview describes its consequences.
///
/// The vocabulary is closed: a consequence the source cannot express here is `Partial` with a
/// named omission, never a confident class carrying unexplained gaps.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionPreviewClass {
    /// Every supported consequence is stated exactly and is not conditioned on anything unseen.
    DeterministicExact,
    /// Consequences depend on a stated condition or an unseen choice.
    Conditional,
    /// Consequences fall in a stated or distributional range.
    RangeDistribution,
    /// Some supported consequences are known and some interactions are omitted.
    Partial,
    /// No supported consequence can be stated for this target.
    Unavailable,
}

/// Kind of one consequence a preview states.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionEffectKind {
    /// Health would change.
    HealthChange,
    /// Block would change.
    BlockChange,
    /// A bounded non-health resource (energy, gold) would change.
    ResourceChange,
    /// A card would enter or lead a pile.
    CardMovement,
    /// A card would be drawn.
    CardDraw,
    /// A card would be exhausted.
    CardExhaust,
    /// Damage would be dealt.
    DamageDealt,
    /// A status or power would change.
    StatusChange,
    /// A target would be affected without a numeric change.
    TargetAffected,
    /// Owner-defined effect kind.
    Custom(String),
    /// Source could not classify the effect.
    Unknown,
}

/// Direction of one status transition.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionStatusTransition {
    /// A status or power would be applied.
    Applied,
    /// A status or power would be removed.
    Removed,
    /// An existing status magnitude would increase.
    Increased,
    /// An existing status magnitude would decrease.
    Decreased,
    /// Source could not classify the transition.
    Unknown,
}

/// Card pile a movement starts from or ends in.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionPile {
    /// The draw pile.
    Draw,
    /// The hand.
    Hand,
    /// The discard pile.
    Discard,
    /// The exhaust pile.
    Exhaust,
    /// The play or limbo area a card occupies while resolving.
    Play,
    /// Owner-defined pile.
    Other(String),
    /// Source could not classify the pile.
    Unknown,
}

/// Explicit reason one interaction is not described by a preview.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionOmissionKind {
    /// The interaction is not modelled by this producer.
    UnknownInteraction,
    /// A status, power, or relic chain is not covered.
    UnsupportedChain,
    /// The outcome exists but must not be revealed.
    WithheldOutcome,
    /// The outcome depends on an unconsumed random draw.
    RandomOutcome,
    /// Only part of the affected target set is covered.
    PartialCoverage,
    /// Owner-defined omission kind.
    Custom(String),
}

/// Where a preview's consequences came from.
///
/// A preview may only be built from an authoritative nonmutating host preview port or from
/// versioned rules with established coverage. `SimulatedFromPendingState` exists so that a caller
/// who approximated a read by applying and undoing a real action is refused by name rather than
/// accepted as a preview.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionPreviewProvenance {
    /// An authoritative nonmutating host preview port.
    HostPreviewPort,
    /// Versioned rules with established coverage.
    VersionedRules {
        /// Versioned rules identity.
        rules_id: String,
    },
    /// The caller approximated the read by applying and undoing a real action.
    SimulatedFromPendingState,
}

/// What a caller must still do before dispatching the action.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionDispatchPrerequisite {
    /// Re-read the fresh legal-action catalog.
    FreshLegalCatalog,
    /// Re-read the fresh run epoch.
    FreshEpoch,
    /// Re-validate the concrete target.
    FreshTargetValidation,
}

/// The only dispatch authority a preview can carry.
///
/// A preview is evidence about consequences, not permission to act, so this enum has exactly one
/// variant and no constructor takes a caller-supplied value.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionDispatchAuthority {
    /// A preview never authorizes dispatch; fresh validation is still required.
    NotGranted,
}
