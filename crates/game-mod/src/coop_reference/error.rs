// SPDX-License-Identifier: MIT

//! Sanitized failures, the source boundary's own error, and the capability a read withholds.

/// Failure before an owned co-op snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoopSourceError {
    /// No supported co-op registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for CoopSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CoopSourceError {}

/// Sanitized failures while producing or reading source-only co-op party data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoopError {
    /// The source failed before an owned snapshot was available.
    NoActiveSource,
    /// The source denied a read.
    SourceAccessDenied,
    /// The source returned malformed data.
    MalformedSource,
    /// The source snapshot names another content manifest.
    ManifestMismatch,
    /// The source snapshot uses another locale.
    LocaleMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// Source and declared party counts disagree.
    FamilyCountMismatch,
    /// Two party records repeat one opaque party identity.
    DuplicateParty(String),
    /// Two peers repeat one opaque peer identity.
    DuplicatePeer(String),
    /// Two peers of one party repeat one membership generation.
    DuplicatePeerGeneration(String),
    /// Two shared effects repeat one effect identity.
    DuplicateEffect(String),
    /// Two scaling rules repeat one rule identity.
    DuplicateScalingRule(String),
    /// Two piles repeat one pile kind.
    DuplicatePile(String),
    /// An identity contains a control byte, a path separator or a traversal segment.
    NonOpaqueIdentity(&'static str),
    /// A field's status and value disagree.
    InconsistentField(&'static str),
    /// A party identity aliases the run or instance identity it belongs to.
    IdentityNamespaceCollision(&'static str),
    /// The party carries no local member, so no view could own local-only fields.
    MissingLocalPeer,
    /// The party carries more than one local member.
    MultipleLocalPeers(String),
    /// A non-active membership carries a gameplay value even though it has no coherent snapshot.
    GameplayForInactivePeer(String),
    /// A value is published as current while the peer's freshness says it is not.
    ValueWithoutCurrentFreshness(String),
    /// A local-only value is carried by a member that is not the local one.
    LocalOnlyValueOnAlly(String),
    /// A local-only pile is published to another member.
    LocalPilePublishedToAlly(String),
    /// A pile's declared visibility contradicts its kind.
    PileVisibilityMismatch(String),
    /// A targeted effect names no peer in the party.
    UnknownPeerTarget(String),
    /// A targeted effect omits its target, or a party-wide effect names one.
    UnresolvedEffectTarget(String),
    /// An effect's scope and its target contradict each other.
    EffectScopeConflict(String),
    /// A scaling rule's target is not an effect the party carries.
    ScalingTargetMismatch(String),
    /// A health value is negative or above its own maximum.
    ImpossibleHealth(String),
    /// A list repeats one entry identity.
    DuplicateEntry(String),
    /// An entry states a count below one.
    ZeroCountEntry(String),
    /// A present collection is empty, so it states nothing.
    EmptyPresentCollection(&'static str),
    /// An input field is invalid or exceeds a local collection bound.
    InvalidInput(&'static str),
    /// A party exceeds the aggregate byte bound.
    PartyTooLarge {
        /// Configured aggregate bound.
        limit: usize,
        /// Measured aggregate size.
        actual: usize,
    },
    /// A referenced definition is absent from the content manifest.
    UnknownManifestReference {
        /// Manifest entity kind.
        entity_kind: String,
        /// Namespaced manifest identity.
        namespaced_id: String,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// The source declares the party family unavailable, so no record can be read.
    UnavailableFamily,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// A reference was produced for another manifest, locale or producer.
    StaleReference,
    /// A member reference names a membership generation the party no longer has.
    StalePeerGeneration(String),
    /// The record is hidden by the selected read scope.
    ExcludedByScope,
    /// No record has the requested identity.
    NotFound,
    /// A live party read is missing its fence.
    MissingLiveFence,
    /// A party read carried a live fence it does not need.
    UnexpectedLiveFence,
    /// A live fence is incomplete or names another instance or party.
    StaleLiveFence,
    /// A live fence names an observation epoch other than the one the catalog holds.
    EpochMismatch {
        /// Epoch the fence names.
        expected: u64,
        /// Epoch the catalog holds.
        actual: u64,
    },
    /// A party vote is published in a phase that admits no vote.
    VoteOutsideVotingPhase(String),
    /// A vote repeats one option or names a member the party does not carry.
    InvalidVote(String),
    /// A targeting relationship names a member the party does not carry, or names itself.
    InvalidTargeting(String),
}

impl std::fmt::Display for CoopError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CoopError {}

/// Capability a published party read does not grant.
///
/// Observing a party is not authority to act in it, to select the single-player save profile, or
/// to touch a save, so every published read states the capability it withholds.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopReadAuthority {
    /// A party read never authorizes selection, save loading, or an action in the party.
    NotGranted,
}

pub(super) fn map_source_error(error: CoopSourceError) -> CoopError {
    match error {
        CoopSourceError::NoActiveSource => CoopError::NoActiveSource,
        CoopSourceError::AccessDenied => CoopError::SourceAccessDenied,
        CoopSourceError::Malformed => CoopError::MalformedSource,
    }
}
