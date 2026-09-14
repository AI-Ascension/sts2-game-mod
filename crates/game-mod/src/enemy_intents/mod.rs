// SPDX-License-Identifier: MIT

//! Source-only structured enemy-intent and public-target projection.
//!
//! The module retains an ordered, bounded component list and explicit target visibility while
//! keeping live enemy identity separate from static definition, move, and intent identities.  A
//! live read is fenced by content manifest, locale, game instance, run, combat, snapshot, and
//! monotonic epoch.  These owner-local values are not a native extractor, transport schema,
//! gateway capability, or wire contract.

mod error;
mod field;
mod live;
mod measure;
mod model;
mod source;
mod targets;
mod validation;
mod validation_fields;

pub use error::{EnemyIntentError, EnemyIntentSourceError, EnemyIntentUnavailableReason};
pub use field::{EnemyIntentField, EnemyIntentFieldStatus, EnemyIntentVisibilityScope};
pub use live::{EnemyIntentEnemy, EnemyIntentLiveReader, EnemyIntentLiveSnapshot};
pub use model::{
    ENEMY_INTENT_MAX_COMPONENTS, ENEMY_INTENT_MAX_EFFECT_REFERENCES, ENEMY_INTENT_MAX_ENEMIES,
    ENEMY_INTENT_MAX_IDENTITY_BYTES, ENEMY_INTENT_MAX_KIND_BYTES,
    ENEMY_INTENT_MAX_LIVE_DETAIL_BYTES, ENEMY_INTENT_MAX_PARAMETERS,
    ENEMY_INTENT_MAX_SNAPSHOT_BYTES, ENEMY_INTENT_MAX_STATUSES, ENEMY_INTENT_MAX_TARGETS,
    ENEMY_INTENT_MAX_TEXT_BYTES, ENEMY_INTENT_PRODUCER_VERSION, EnemyIntentCatalogBinding,
    EnemyIntentComponent, EnemyIntentComponentKind, EnemyIntentComponentReference,
    EnemyIntentEffectReference, EnemyIntentEffectReferenceKind, EnemyIntentEnemyInput,
    EnemyIntentEnemyReference, EnemyIntentInput, EnemyIntentLinkage, EnemyIntentLiveBinding,
    EnemyIntentParameter, EnemyIntentParameterValue, EnemyIntentSnapshotInput, EnemyIntentStatus,
};
pub use source::{
    EnemyIntentCapability, EnemyIntentSource, FixtureEnemyIntentSource,
    UnavailableEnemyIntentSource,
};
pub use targets::{
    EnemyIntentAmount, EnemyIntentDamage, EnemyIntentTargetDomain, EnemyIntentTargetInfo,
    EnemyIntentTargetKind, EnemyIntentTargetReference, EnemyIntentTargets, EnemyIntentUnit,
};
