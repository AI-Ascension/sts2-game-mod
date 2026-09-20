// SPDX-License-Identifier: MIT

use super::{
    PROGRESSION_MAX_FIELD_ROWS, ProgressionCatalogBinding, ProgressionContentReference,
    ProgressionDomain, ProgressionField, ProgressionFieldAvailability, ProgressionFieldStatus,
    ProgressionFieldValue, ProgressionProfileQuery, ProgressionQuantity, ProgressionReadState,
    ProgressionReferenceError, ProgressionRelatedReference, ProgressionRequirement,
    ProgressionSensitivity, ProgressionText, ProgressionVisibility,
};

/// Typed source-owned progression entry before it is bound to a catalog witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionEntryInput {
    /// Stable progression identity, scoped by domain.
    pub entry_id: String,
    /// Domain this entry belongs to.
    pub domain: ProgressionDomain,
    /// What the source reports about the entry's state for the selected profile.
    pub read_state: ProgressionReadState,
    /// Localized entry title.
    pub title: String,
    /// Localized entry description, when the source supplies one.
    pub description: Option<String>,
    /// Whether this entry is game progression or account-scoped data.
    pub sensitivity: ProgressionSensitivity,
    /// Owner-defined visibility.
    pub visibility: ProgressionVisibility,
    /// Account or private identifier the source reported beside the entry, if any.
    ///
    /// This is deliberately a separate field: a source that reports one is refused rather than
    /// having the identifier copied into the progression payload.
    pub account_id: Option<String>,
    /// Progress toward the entry, or its stated absence.
    pub progress: ProgressionFieldValue<ProgressionQuantity>,
    /// Best recorded value for the entry, or its stated absence.
    pub best: ProgressionFieldValue<ProgressionQuantity>,
    /// Requirements gating the entry, or their stated absence.
    pub requirements: ProgressionFieldValue<Vec<ProgressionRequirement>>,
    /// Manifest definitions the entry resolves against, or their stated absence.
    pub content_references: ProgressionFieldValue<Vec<ProgressionContentReference>>,
    /// Related owner identities, or their stated absence.
    pub related: ProgressionFieldValue<Vec<ProgressionRelatedReference>>,
    /// Declared availability of every field this entry inventories.
    pub fields: Vec<ProgressionFieldAvailability>,
}

/// Immutable progression entry bound to one catalog witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionEntry {
    /// Catalog witness that owns this progression identity.
    pub binding: ProgressionCatalogBinding,
    /// Stable progression identity, scoped by domain.
    pub entry_id: String,
    /// Domain this entry belongs to.
    pub domain: ProgressionDomain,
    /// What the source reports about the entry's state for the selected profile.
    pub read_state: ProgressionReadState,
    /// Localized entry title.
    pub title: String,
    /// Localized entry description or an explicit non-value.
    pub description: ProgressionText,
    /// Whether this entry is game progression or account-scoped data.
    pub sensitivity: ProgressionSensitivity,
    /// Owner-defined visibility.
    pub visibility: ProgressionVisibility,
    /// Progress toward the entry, or its stated absence.
    pub progress: ProgressionFieldValue<ProgressionQuantity>,
    /// Best recorded value for the entry, or its stated absence.
    pub best: ProgressionFieldValue<ProgressionQuantity>,
    /// Requirements gating the entry, or their stated absence.
    pub requirements: ProgressionFieldValue<Vec<ProgressionRequirement>>,
    /// Manifest definitions the entry resolves against, or their stated absence.
    pub content_references: ProgressionFieldValue<Vec<ProgressionContentReference>>,
    /// Related owner identities, or their stated absence.
    pub related: ProgressionFieldValue<Vec<ProgressionRelatedReference>>,
    /// Declared availability of every field this entry inventories.
    pub fields: Vec<ProgressionFieldAvailability>,
}

impl ProgressionEntry {
    pub(crate) fn from_input(
        binding: &ProgressionCatalogBinding,
        input: ProgressionEntryInput,
    ) -> Self {
        let description = match input.description {
            Some(description) => super::model::text_from_validated(description),
            None => ProgressionText::not_observed(),
        };
        Self {
            binding: binding.clone(),
            entry_id: input.entry_id,
            domain: input.domain,
            read_state: input.read_state,
            title: input.title,
            description,
            sensitivity: input.sensitivity,
            visibility: input.visibility,
            progress: input.progress,
            best: input.best,
            requirements: input.requirements,
            content_references: input.content_references,
            related: input.related,
            fields: input.fields,
        }
    }

    /// Returns the declared status of one field, when this entry states a row for it.
    #[must_use]
    pub fn field_status(&self, field: ProgressionField) -> Option<ProgressionFieldStatus> {
        self.fields
            .iter()
            .find(|row| row.field == field)
            .map(|row| row.status)
    }

    /// Returns whether every field this entry inventories is either projected or a declared failure.
    #[must_use]
    pub fn fields_are_declared(&self) -> bool {
        ProgressionField::all()
            .iter()
            .all(|field| self.field_status(*field).is_some())
    }
}

/// Estimates the aggregate bytes retained for one entry.
pub(super) fn entry_bytes(input: &ProgressionEntryInput) -> usize {
    let mut total = input.entry_id.len()
        + input.title.len()
        + input.description.as_ref().map_or(0, String::len)
        + input.account_id.as_ref().map_or(0, String::len)
        + quantity_bytes(&input.progress)
        + quantity_bytes(&input.best);
    if let Some(requirements) = input.requirements.value() {
        for requirement in requirements {
            total += requirement.requirement_id.len()
                + requirement.description.value().map_or(0, str::len)
                + quantity_bytes(&requirement.progress);
        }
    }
    if let Some(references) = input.content_references.value() {
        total += references
            .iter()
            .map(|reference| reference.entity_kind.len() + reference.namespaced_id.len())
            .sum::<usize>();
    }
    if let Some(related) = input.related.value() {
        total += related.iter().map(|entry| entry.id.len()).sum::<usize>();
    }
    total
}

fn quantity_bytes(field: &ProgressionFieldValue<ProgressionQuantity>) -> usize {
    field.value().map_or(0, |_| 16)
}

/// Validates the declared field-row inventory of one entry.
pub(super) fn validate_field_rows(
    input: &ProgressionEntryInput,
) -> Result<(), ProgressionReferenceError> {
    if input.fields.len() > PROGRESSION_MAX_FIELD_ROWS {
        return Err(ProgressionReferenceError::InvalidInput("fields"));
    }
    for field in ProgressionField::all() {
        let rows = input.fields.iter().filter(|row| row.field == field).count();
        if rows == 0 {
            return Err(ProgressionReferenceError::MissingFieldCoverage(
                field.name(),
            ));
        }
        if rows > 1 {
            return Err(ProgressionReferenceError::DuplicateFieldCoverage(
                field.name(),
            ));
        }
    }
    for row in &input.fields {
        if row.status != carried_status(input, row.field) {
            return Err(ProgressionReferenceError::InconsistentField(
                row.field.name(),
            ));
        }
    }
    Ok(())
}

/// Returns the status the entry's own carrier states for one field.
fn carried_status(
    input: &ProgressionEntryInput,
    field: ProgressionField,
) -> ProgressionFieldStatus {
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

/// Exact request for one retained progression entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionEntryReference {
    /// Catalog witness that owns this entry identity.
    pub catalog: ProgressionCatalogBinding,
    /// Stable progression identity.
    pub entry_id: String,
}

/// Bounded request for one entry's detail.
#[derive(Debug, Eq, PartialEq)]
pub struct ProgressionDetailQuery {
    /// Exact entry whose detail is requested.
    pub entry: ProgressionEntryReference,
    /// Profile expectation for this read.
    pub profile: ProgressionProfileQuery,
    /// Requested visibility scope.
    pub scope: super::ProgressionVisibilityScope,
    /// Profile revision the caller believes it is reading, when it states one.
    pub revision: Option<super::ProgressionRevision>,
}
