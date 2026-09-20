// SPDX-License-Identifier: MIT

//! Synthetic source-only progression fixture: one manifest, one port, one snapshot builder.

use std::cell::Cell;

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    PROGRESSION_REFERENCE_PRODUCER_VERSION, ProgressionCatalog, ProgressionCatalogProducer,
    ProgressionCatalogSnapshot, ProgressionContentReference, ProgressionDomain,
    ProgressionDomainCoverage, ProgressionDomainState, ProgressionEntryInput, ProgressionField,
    ProgressionFieldAvailability, ProgressionFieldStatus, ProgressionFieldValue,
    ProgressionProfile, ProgressionProfileInput, ProgressionProfileKind, ProgressionProfilePermit,
    ProgressionProfileQuery, ProgressionQuantity, ProgressionReadAvailability, ProgressionReadPort,
    ProgressionReadState, ProgressionReferenceError, ProgressionRelatedReference,
    ProgressionRequirement, ProgressionRequirementKind, ProgressionSensitivity, ProgressionText,
    ProgressionUnit, ProgressionVisibility, SaveProfileBaseline, SaveSlotId, SelectionAuthority,
    UserDataIdentity,
};

pub const DOMAIN_KINDS: [(&str, ProgressionDomain); 6] = [
    ("unlock", ProgressionDomain::Unlocks),
    ("achievement", ProgressionDomain::Achievements),
    ("compendium_entry", ProgressionDomain::Compendium),
    ("character", ProgressionDomain::CharacterProgression),
    ("best_record", ProgressionDomain::BestRecords),
    ("statistic", ProgressionDomain::AggregateStatistics),
];

pub struct ManifestSource {
    pub snapshot: ContentCatalogSnapshot,
}

impl ContentCatalogSource for ManifestSource {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}

/// Builds one synthetic manifest whose six progression families are all handled.
pub fn manifest(extra_handled: &[&str]) -> ContentManifest {
    let handled = DOMAIN_KINDS
        .iter()
        .map(|(kind, _)| *kind)
        .chain(extra_handled.iter().copied())
        .collect::<Vec<_>>();
    manifest_handling(&handled)
}

/// Builds one synthetic manifest whose handled families are exactly the listed kinds.
pub fn manifest_handling(handled: &[&str]) -> ContentManifest {
    let definitions = DOMAIN_KINDS
        .iter()
        .map(|(kind, _)| ContentDefinitionInput {
            entity_kind: (*kind).to_owned(),
            namespaced_id: format!("base:{kind}:one"),
            semantic_inputs: format!("kind={kind};id=one"),
            localized_text: Some(format!("{kind} one")),
            origin: ContentOriginInput {
                package_id: Some("base:game".to_owned()),
                package_version: None,
            },
            override_chain: Vec::new(),
        })
        .collect::<Vec<_>>();
    let snapshot = ContentCatalogSnapshot {
        generation_before: 4,
        generation_after: 4,
        game_build: "sts2-build:0.108.0".to_owned(),
        locale: "en-US".to_owned(),
        packages: vec![ContentPackageInput {
            package_id: "base:game".to_owned(),
            package_version: None,
            order: 0,
        }],
        available_entity_kinds: DOMAIN_KINDS
            .iter()
            .map(|(kind, _)| (*kind).to_owned())
            .collect(),
        registry_definition_counts: DOMAIN_KINDS
            .iter()
            .map(|(kind, _)| ((*kind).to_owned(), 1))
            .collect(),
        definitions,
    };
    ContentManifestProducer::new("adapter-v1", handled.iter().map(|kind| (*kind).to_owned()))
        .expect("manifest producer")
        .produce(&ManifestSource { snapshot })
        .expect("manifest")
}

/// Synthetic read port that counts how often it was asked for a snapshot.
pub struct FixturePort {
    pub snapshot: Result<ProgressionCatalogSnapshot, sts2_game_mod::ProgressionSourceError>,
    reads: Cell<usize>,
}

impl FixturePort {
    #[must_use]
    pub fn new(snapshot: ProgressionCatalogSnapshot) -> Self {
        Self {
            snapshot: Ok(snapshot),
            reads: Cell::new(0),
        }
    }

    #[must_use]
    pub fn failing(error: sts2_game_mod::ProgressionSourceError) -> Self {
        Self {
            snapshot: Err(error),
            reads: Cell::new(0),
        }
    }

    #[must_use]
    pub fn reads(&self) -> usize {
        self.reads.get()
    }
}

impl ProgressionReadPort for FixturePort {
    fn read_availability(&self) -> ProgressionReadAvailability {
        ProgressionReadAvailability::SyntheticFixtureOnly
    }

    fn read_progression(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<ProgressionCatalogSnapshot, sts2_game_mod::ProgressionSourceError> {
        self.reads.set(self.reads.get() + 1);
        self.snapshot.clone()
    }
}

pub fn produce<P: ProgressionReadPort>(
    manifest: &ContentManifest,
    port: &P,
) -> Result<ProgressionCatalog, ProgressionReferenceError> {
    ProgressionCatalogProducer::new().produce(manifest, port, ProgressionProfileQuery::Active)
}

pub fn profile(
    user_data_id: &str,
    kind: ProgressionProfileKind,
    revision: u64,
) -> ProgressionProfileInput {
    let identity = UserDataIdentity::new(user_data_id).expect("user data identity");
    ProgressionProfileInput {
        profile: ProgressionProfile {
            user_data_id: identity.clone(),
            save_slot: SaveSlotId::new("slot:one").expect("save slot"),
            kind,
        },
        baseline: SaveProfileBaseline::new(identity, "digest:one", revision).expect("baseline"),
        permit: ProgressionProfilePermit::Permitted {
            authority: SelectionAuthority::new("owner:permit").expect("authority"),
        },
    }
}

pub fn unnamed_profile(kind: ProgressionProfileKind, revision: u64) -> ProgressionProfileInput {
    profile("profile:one", kind, revision)
}

/// Domains the canonical snapshot projects; the rest are stated unsupported or unavailable.
pub const PROJECTED_DOMAINS: [ProgressionDomain; 5] = [
    ProgressionDomain::Unlocks,
    ProgressionDomain::Achievements,
    ProgressionDomain::Compendium,
    ProgressionDomain::BestRecords,
    ProgressionDomain::AggregateStatistics,
];

/// One canonical entry per domain and state the slice has to keep apart.
pub fn canonical_entries() -> Vec<ProgressionEntryInput> {
    let mut award = entry(
        "achievement:first_blood",
        ProgressionDomain::Achievements,
        ProgressionReadState::Unlocked,
    );
    award.progress = ProgressionFieldValue::available(percent(100));
    award.description = Some("Win one combat.".to_owned());
    award = finish(award);

    let mut locked = locked_entry("achievement:ascendancy", ProgressionDomain::Achievements);
    locked.progress = ProgressionFieldValue::available(percent(40));
    locked = finish(locked);

    let mut discovered = entry(
        "compendium_entry:ember",
        ProgressionDomain::Compendium,
        ProgressionReadState::Undiscovered,
    );
    discovered.content_references =
        ProgressionFieldValue::available(vec![content("compendium_entry")]);
    discovered = finish(discovered);

    let mut best = entry(
        "best_record:fastest_floor",
        ProgressionDomain::BestRecords,
        ProgressionReadState::Unlocked,
    );
    best.best = ProgressionFieldValue::available(seconds(642));
    best = finish(best);

    let mut stat = entry(
        "statistic:runs_started",
        ProgressionDomain::AggregateStatistics,
        ProgressionReadState::NotTracked,
    );
    stat.description = Some("Not tracked by the supported build.".to_owned());
    stat = finish(stat);

    let mut unlock = unlocked_entry("unlock:act_four", ProgressionDomain::Unlocks);
    unlock.related = ProgressionFieldValue::available(vec![related(
        ProgressionDomain::CharacterProgression,
        "character:silent",
    )]);
    unlock = finish(unlock);

    vec![unlock, award, locked, discovered, best, stat]
}

/// Builds the canonical snapshot at one stated profile revision.
pub fn canonical_snapshot(manifest: &ContentManifest, revision: u64) -> ProgressionCatalogSnapshot {
    snapshot(
        manifest,
        unnamed_profile(ProgressionProfileKind::Active, revision),
        &PROJECTED_DOMAINS,
        canonical_entries(),
    )
}

/// Builds one snapshot with the coverage the entries imply and the listed domains projected.
pub fn snapshot(
    manifest: &ContentManifest,
    profile: ProgressionProfileInput,
    projected: &[ProgressionDomain],
    entries: Vec<ProgressionEntryInput>,
) -> ProgressionCatalogSnapshot {
    ProgressionCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: PROGRESSION_REFERENCE_PRODUCER_VERSION.to_owned(),
        profile,
        domains: coverage(projected, &entries),
        entries,
    }
}

/// States one coverage row per domain, with observed counts and explicit unsupported states.
pub fn coverage(
    projected: &[ProgressionDomain],
    entries: &[ProgressionEntryInput],
) -> Vec<ProgressionDomainCoverage> {
    ProgressionDomain::all()
        .into_iter()
        .map(|domain| {
            let entry_count = entries
                .iter()
                .filter(|entry| entry.domain == domain)
                .count();
            let state = if projected.contains(&domain) {
                ProgressionDomainState::Projected
            } else if domain.is_game_progression() {
                ProgressionDomainState::Unavailable
            } else {
                ProgressionDomainState::Unsupported
            };
            ProgressionDomainCoverage {
                domain,
                state,
                entry_count,
                unsupported_fields: Vec::new(),
            }
        })
        .collect()
}

pub fn finish(mut input: ProgressionEntryInput) -> ProgressionEntryInput {
    input.fields = rows(&input);
    input
}

/// Mirrors the producer's own carrier rule so a fixture can state rows and deliberately break one.
pub fn rows(input: &ProgressionEntryInput) -> Vec<ProgressionFieldAvailability> {
    ProgressionField::all()
        .into_iter()
        .map(|field| ProgressionFieldAvailability {
            field,
            status: status_of(input, field),
        })
        .collect()
}

pub fn status_of(input: &ProgressionEntryInput, field: ProgressionField) -> ProgressionFieldStatus {
    match field {
        ProgressionField::Title => ProgressionFieldStatus::Available,
        ProgressionField::Description => match input.description {
            Some(_) => ProgressionFieldStatus::Available,
            None => ProgressionFieldStatus::NotObserved,
        },
        ProgressionField::ReadState => {
            if input.read_state.is_stated() {
                ProgressionFieldStatus::Available
            } else {
                ProgressionFieldStatus::NotObserved
            }
        }
        ProgressionField::Progress => input.progress.status(),
        ProgressionField::BestValue => input.best.status(),
        ProgressionField::Requirements => input.requirements.status(),
        ProgressionField::ContentReferences => input.content_references.status(),
        ProgressionField::RelatedEntries => input.related.status(),
    }
}

pub fn count(value: i64) -> ProgressionQuantity {
    ProgressionQuantity::new(value, ProgressionUnit::Count)
}

pub fn percent(value: i64) -> ProgressionQuantity {
    ProgressionQuantity::new(value, ProgressionUnit::Percent)
}

pub fn seconds(value: i64) -> ProgressionQuantity {
    ProgressionQuantity::new(value, ProgressionUnit::Seconds)
}

pub fn content(kind: &str) -> ProgressionContentReference {
    ProgressionContentReference::new(kind, &format!("base:{kind}:one")).expect("content reference")
}

pub fn requirement(id: &str, state: ProgressionReadState) -> ProgressionRequirement {
    ProgressionRequirement {
        requirement_id: id.to_owned(),
        kind: ProgressionRequirementKind::Content,
        state,
        progress: ProgressionFieldValue::not_observed(),
        content: Some(content("unlock")),
        description: ProgressionText::available("Reach the stated requirement.")
            .expect("requirement text"),
    }
}

/// One entry with every optional field stated absent, so a test states only what it varies.
pub fn entry(
    entry_id: &str,
    domain: ProgressionDomain,
    state: ProgressionReadState,
) -> ProgressionEntryInput {
    finish(ProgressionEntryInput {
        entry_id: entry_id.to_owned(),
        domain,
        read_state: state,
        title: entry_id.rsplit(':').next().unwrap_or(entry_id).to_owned(),
        description: None,
        sensitivity: ProgressionSensitivity::GameProgression,
        visibility: ProgressionVisibility::Visible,
        account_id: None,
        progress: ProgressionFieldValue::not_observed(),
        best: ProgressionFieldValue::not_observed(),
        requirements: ProgressionFieldValue::not_observed(),
        content_references: ProgressionFieldValue::not_observed(),
        related: ProgressionFieldValue::not_observed(),
        fields: Vec::new(),
    })
}

pub fn locked_entry(entry_id: &str, domain: ProgressionDomain) -> ProgressionEntryInput {
    let mut input = entry(entry_id, domain, ProgressionReadState::Locked);
    input.requirements = ProgressionFieldValue::available(vec![requirement(
        "requirement:one",
        ProgressionReadState::Locked,
    )]);
    finish(input)
}

pub fn unlocked_entry(entry_id: &str, domain: ProgressionDomain) -> ProgressionEntryInput {
    let mut input = entry(entry_id, domain, ProgressionReadState::Unlocked);
    input.content_references = ProgressionFieldValue::available(vec![content("unlock")]);
    finish(input)
}

pub fn related(domain: ProgressionDomain, id: &str) -> ProgressionRelatedReference {
    ProgressionRelatedReference {
        domain,
        id: id.to_owned(),
    }
}

/// A deliberately broken row, so a test can prove the coverage rule is load-bearing.
pub fn break_row(input: &mut ProgressionEntryInput, field: ProgressionField) {
    for row in &mut input.fields {
        if row.field == field {
            row.status = ProgressionFieldStatus::Withheld;
        }
    }
}
