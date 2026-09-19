// SPDX-License-Identifier: MIT

//! Read-only rendering, resolution and metadata for the locale reference catalog.
//!
//! Every method here reads one immutable catalog.  Nothing switches the active game language,
//! mutates the catalog, or invents text for a locale that carries none.

use std::collections::{BTreeMap, BTreeSet};

use super::LocaleCatalogError;
use super::catalog::{
    LocaleCatalog, LocaleCatalogBinding, LocaleRenderRequest, LocaleRenderedText,
    LocaleTextReference,
};
use super::model::{
    LocaleCompleteness, LocaleDirection, LocaleEntityReference, LocaleInput,
    LocalePlaceholderValue, LocaleRenderedSegment, LocaleTextSegment, validate_identity,
};
use super::normalize::validate_presentation;
use super::validation::validate_reference_shape;

impl LocaleCatalog {
    /// Returns the binding that fences this catalog and its references.
    #[must_use]
    pub fn binding(&self) -> &LocaleCatalogBinding {
        &self.binding
    }

    /// Returns the default locale of this catalog.
    #[must_use]
    pub fn default_locale(&self) -> &str {
        &self.default_locale
    }

    /// Returns the supported locales in the owner-declared order.
    #[must_use]
    pub fn supported_locales(&self) -> Vec<&LocaleInput> {
        self.locale_order
            .iter()
            .filter_map(|tag| self.locales.get(tag))
            .collect()
    }

    /// Returns the explicit fallback chain for one locale.
    #[must_use]
    pub fn fallback_chain(&self, locale: &str) -> Option<&[String]> {
        self.locales
            .get(locale)
            .map(|input| input.fallback.as_slice())
    }

    /// Returns the reported direction for one locale.
    #[must_use]
    pub fn direction(&self, locale: &str) -> Option<LocaleDirection> {
        self.locales.get(locale).map(|input| input.direction)
    }

    /// Returns the number of stored locale/plural entry variants.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the number of stored variants for one reference.
    #[must_use]
    pub fn variant_count(&self, entity_kind: &str, namespaced_id: &str) -> usize {
        self.variants
            .get(&(entity_kind.to_owned(), namespaced_id.to_owned()))
            .copied()
            .unwrap_or(0)
    }

    /// Produces a fenced reference to one definition's text in one locale.
    pub fn reference(
        &self,
        locale: &str,
        reference: &LocaleEntityReference,
    ) -> Result<LocaleTextReference, LocaleCatalogError> {
        validate_identity(locale, "locale")?;
        if !self.locales.contains_key(locale) {
            return Err(LocaleCatalogError::UnsupportedLocale(locale.to_owned()));
        }
        validate_reference_shape(reference)?;
        if self.variant_count(&reference.entity_kind, &reference.namespaced_id) == 0 {
            return Err(LocaleCatalogError::NotFound);
        }
        Ok(LocaleTextReference {
            binding: self.binding.clone(),
            locale: locale.to_owned(),
            reference: reference.clone(),
        })
    }

    /// Resolves a fenced reference, rejecting one produced by another catalog.
    pub fn resolve(
        &self,
        reference: &LocaleTextReference,
    ) -> Result<LocaleRenderedText, LocaleCatalogError> {
        if reference.binding != self.binding {
            return Err(LocaleCatalogError::StaleReference);
        }
        self.render(&LocaleRenderRequest {
            locale: reference.locale.clone(),
            entity_kind: reference.reference.entity_kind.clone(),
            namespaced_id: reference.reference.namespaced_id.clone(),
            plural: None,
            placeholders: Vec::new(),
        })
    }

    /// Renders one reference in one locale through its explicit fallback chain.
    pub fn render(
        &self,
        request: &LocaleRenderRequest,
    ) -> Result<LocaleRenderedText, LocaleCatalogError> {
        validate_identity(&request.locale, "locale")?;
        validate_identity(&request.entity_kind, "entity_kind")?;
        validate_identity(&request.namespaced_id, "namespaced_id")?;
        let entry = match self.locales.get(&request.locale) {
            Some(input) => input,
            None => {
                return Err(LocaleCatalogError::UnsupportedLocale(
                    request.locale.clone(),
                ));
            }
        };
        let mut supplied: BTreeMap<&str, &LocalePlaceholderValue> = BTreeMap::new();
        for placeholder in &request.placeholders {
            validate_identity(&placeholder.name, "placeholder")?;
            match &placeholder.value {
                LocalePlaceholderValue::Text(value) | LocalePlaceholderValue::Number(value) => {
                    validate_presentation(value, "placeholder_value")?;
                }
                LocalePlaceholderValue::Reference(reference) => {
                    validate_reference_shape(reference)?;
                }
                LocalePlaceholderValue::Unavailable(_) => {}
            }
            if supplied
                .insert(placeholder.name.as_str(), &placeholder.value)
                .is_some()
            {
                return Err(LocaleCatalogError::InvalidInput(
                    "duplicate_placeholder_value",
                ));
            }
        }
        let chain = entry.fallback.clone();
        for (index, tag) in chain.iter().enumerate() {
            if let Some(stored) = self.find_entry(
                &request.entity_kind,
                &request.namespaced_id,
                tag,
                request.plural,
            ) {
                for name in supplied.keys() {
                    if !stored.requires.iter().any(|required| required == name) {
                        return Err(LocaleCatalogError::UnknownPlaceholder((*name).to_owned()));
                    }
                }
                let mut segments = Vec::with_capacity(stored.segments.len());
                let mut unresolved = BTreeSet::new();
                for segment in &stored.segments {
                    match segment {
                        LocaleTextSegment::Text(value) => {
                            segments.push(LocaleRenderedSegment::Text(value.clone()));
                        }
                        LocaleTextSegment::Placeholder(name) => match supplied.get(name.as_str()) {
                            Some(LocalePlaceholderValue::Text(value))
                            | Some(LocalePlaceholderValue::Number(value)) => {
                                segments.push(LocaleRenderedSegment::Text(value.clone()));
                            }
                            Some(LocalePlaceholderValue::Reference(reference)) => {
                                segments.push(LocaleRenderedSegment::Reference(reference.clone()))
                            }
                            Some(LocalePlaceholderValue::Unavailable(_)) | None => {
                                unresolved.insert(name.clone());
                                segments.push(LocaleRenderedSegment::UnresolvedPlaceholder(
                                    name.clone(),
                                ));
                            }
                        },
                        LocaleTextSegment::Reference(reference) => {
                            segments.push(LocaleRenderedSegment::Reference(reference.clone()));
                        }
                        LocaleTextSegment::Effect { kind, amount } => {
                            segments.push(LocaleRenderedSegment::Effect {
                                kind: kind.clone(),
                                amount: amount.clone(),
                            });
                        }
                    }
                }
                let resolved = self.locales.get(tag);
                let direction = resolved.map_or(LocaleDirection::Unknown, |input| input.direction);
                let unresolved_placeholders: Vec<String> = unresolved.into_iter().collect();
                let exact = tag == &request.locale;
                let completeness = if exact && unresolved_placeholders.is_empty() {
                    LocaleCompleteness::Complete
                } else {
                    LocaleCompleteness::Partial
                };
                return Ok(LocaleRenderedText {
                    entity_kind: request.entity_kind.clone(),
                    namespaced_id: request.namespaced_id.clone(),
                    requested_locale: request.locale.clone(),
                    effective_locale: tag.clone(),
                    fallback_chain: chain[..=index].to_vec(),
                    direction,
                    text_revision: stored.revision.clone(),
                    completeness,
                    segments,
                    unresolved_placeholders,
                });
            }
        }
        Err(LocaleCatalogError::NotFound)
    }
}
