// SPDX-License-Identifier: MIT

use super::{
    definition::SelectionDefinition,
    error::SelectionError,
    field::SelectionField,
    model::{
        SEL_MAX_PICKS, SelectionCandidateReference, SelectionDuplicateRule, SelectionReference,
        SelectionVisibilityScope, SelectorGenerationReference,
    },
    progress::{
        SelectionConfirmationState, SelectionProgress, SelectionProgressInput, confirmation_state,
        remaining_picks, required_picks,
    },
    projection::{map_status, selectable_candidate, visible_candidate},
    reader::SelectionReader,
};

impl<'a> SelectionReader<'a> {
    /// Reconciles the picks a caller reports against one selector generation.
    ///
    /// The reported picks are checked against the catalog before anything is described: a selector
    /// generation the catalog does not currently present, a candidate this selector does not
    /// present, a repeated pick under a no-repeat rule, and more picks than the selector accepts are
    /// each refused rather than described as a legal prompt state.
    pub fn describe_progress(
        &self,
        progress: &SelectionProgressInput,
        scope: SelectionVisibilityScope,
    ) -> Result<SelectionProgress, SelectionError> {
        let (definition, selected) = self.resolve(progress, scope)?;
        let picked = selected.len();
        let available = definition
            .candidates
            .values()
            .filter(|candidate| {
                selectable_candidate(candidate, scope, &progress.selected, definition.duplicate)
            })
            .count();
        Ok(SelectionProgress {
            selector: progress.selector.clone(),
            parent: definition.parent.clone(),
            kind: definition.kind.clone(),
            prompt: definition.prompt.clone(),
            picks: definition.picks.clone(),
            selected,
            picked,
            remaining: remaining_picks(&definition.picks, picked),
            available,
            candidates_status: map_status(definition.candidates.values(), scope, visible_candidate),
            confirmation: definition.confirmation,
            cancellation: definition.cancellation.clone(),
            confirmation_state: confirmation_state(
                &definition.picks,
                &definition.cancellation,
                picked,
            ),
            next: definition.next.clone(),
            complete: picked >= required_picks(&definition.picks),
        })
    }

    /// Returns the confirmation state for one selector generation, or refuses an early confirmation.
    ///
    /// A caller that asks to confirm before the required picks are made is refused with the exact
    /// shortfall, so an early confirmation can never be reported as a legal one.
    pub fn confirmation(
        &self,
        progress: &SelectionProgressInput,
        scope: SelectionVisibilityScope,
    ) -> Result<SelectionConfirmationState, SelectionError> {
        let (definition, selected) = self.resolve(progress, scope)?;
        let state = confirmation_state(&definition.picks, &definition.cancellation, selected.len());
        if !state.may_confirm {
            return Err(SelectionError::PrematureConfirmation {
                selection_id: definition.reference.selection_id.clone(),
                required: required_picks(&definition.picks) as u32,
                observed: selected.len() as u32,
            });
        }
        Ok(state)
    }

    /// Returns the selector presented once the current picks are complete.
    ///
    /// A multi-step prompt moves to the next candidate domain, so the next selector is resolved by
    /// identity and generation instead of being inferred. `Ok(None)` means the sequence ends here.
    pub fn next_selector(
        &self,
        progress: &SelectionProgressInput,
        scope: SelectionVisibilityScope,
    ) -> Result<Option<SelectorGenerationReference>, SelectionError> {
        let (definition, selected) = self.resolve(progress, scope)?;
        if selected.len() < required_picks(&definition.picks) {
            return Err(SelectionError::IncompleteSelection {
                selection_id: definition.reference.selection_id.clone(),
                required: required_picks(&definition.picks) as u32,
                observed: selected.len() as u32,
            });
        }
        let SelectionField::Available(next) = &definition.next else {
            return Ok(None);
        };
        self.definition_of(
            &next.selection_id,
            &self.catalog().binding,
            SelectionVisibilityScope::Owner,
        )?;
        Ok(Some(SelectorGenerationReference {
            selection: SelectionReference {
                catalog: self.catalog().binding.clone(),
                selection_id: next.selection_id.clone(),
            },
            selector_generation: next.selector_generation,
        }))
    }

    pub(super) fn selected_ids(
        &self,
        progress: Option<&SelectionProgressInput>,
        scope: SelectionVisibilityScope,
    ) -> Result<Vec<String>, SelectionError> {
        match progress {
            Some(progress) => Ok(self
                .resolve(progress, scope)?
                .1
                .into_iter()
                .map(|reference| reference.candidate_id)
                .collect()),
            None => Ok(Vec::new()),
        }
    }

    pub(super) fn resolve(
        &self,
        progress: &SelectionProgressInput,
        scope: SelectionVisibilityScope,
    ) -> Result<(&'a SelectionDefinition, Vec<SelectionCandidateReference>), SelectionError> {
        self.validate_family()?;
        let definition = self.generation_of(&progress.selector, scope)?;
        if progress.selected.len() > SEL_MAX_PICKS as usize {
            return Err(SelectionError::InvalidInput("selected"));
        }
        ensure_no_repeated_pick(definition, &progress.selected)?;
        if let Some(maximum) = definition.picks.maximum_picks()
            && progress.selected.len() > maximum as usize
        {
            return Err(SelectionError::ExcessPicks {
                selection_id: definition.reference.selection_id.clone(),
                maximum,
                observed: progress.selected.len() as u32,
            });
        }
        let mut resolved = Vec::with_capacity(progress.selected.len());
        for candidate_id in &progress.selected {
            let candidate = definition.candidates.get(candidate_id).ok_or_else(|| {
                SelectionError::UnknownCandidate {
                    selection_id: definition.reference.selection_id.clone(),
                    candidate_id: candidate_id.clone(),
                }
            })?;
            if !visible_candidate(candidate, scope) {
                return Err(SelectionError::ExcludedByScope);
            }
            resolved.push(candidate.reference.clone());
        }
        Ok((definition, resolved))
    }

    /// Resolves one selector generation against the catalog.
    ///
    /// A reference bound to an earlier or later generation no longer describes the presented prompt,
    /// so it is refused rather than answered with the current candidate domain.
    pub(super) fn generation_of(
        &self,
        selector: &SelectorGenerationReference,
        scope: SelectionVisibilityScope,
    ) -> Result<&'a SelectionDefinition, SelectionError> {
        let definition = self.definition_of(
            &selector.selection.selection_id,
            &selector.selection.catalog,
            scope,
        )?;
        if selector.selector_generation != definition.selector_generation {
            return Err(SelectionError::StaleSelectorReference {
                selection_id: definition.reference.selection_id.clone(),
                referenced: selector.selector_generation,
                current: definition.selector_generation,
            });
        }
        Ok(definition)
    }
}

/// Refuses a repeated pick where the selector does not allow repeats.
fn ensure_no_repeated_pick(
    definition: &SelectionDefinition,
    selected: &[String],
) -> Result<(), SelectionError> {
    if definition.duplicate != SelectionDuplicateRule::Distinct {
        return Ok(());
    }
    let mut seen = std::collections::BTreeSet::new();
    for candidate_id in selected {
        if !seen.insert(candidate_id.as_str()) {
            return Err(SelectionError::DuplicateChoice {
                selection_id: definition.reference.selection_id.clone(),
                candidate_id: candidate_id.clone(),
            });
        }
    }
    Ok(())
}
