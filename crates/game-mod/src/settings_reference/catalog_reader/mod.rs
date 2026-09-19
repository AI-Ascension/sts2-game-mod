// SPDX-License-Identifier: MIT

mod catalog;
mod page;
mod reader;

pub use catalog::SettingsCatalog;
pub use page::*;
pub use reader::SettingsCatalogReader;

use super::{SettingDefinition, SettingsSensitivity, SettingsVisibility, SettingsVisibilityScope};

fn visible_setting(definition: &SettingDefinition, scope: SettingsVisibilityScope) -> bool {
    if definition.sensitivity == SettingsSensitivity::Private {
        return matches!(scope, SettingsVisibilityScope::Owner);
    }
    visibility_allowed(definition.visibility, scope)
}

fn visibility_allowed(visibility: SettingsVisibility, scope: SettingsVisibilityScope) -> bool {
    match visibility {
        SettingsVisibility::Visible => true,
        SettingsVisibility::OwnerOnly => matches!(
            scope,
            SettingsVisibilityScope::Reference | SettingsVisibilityScope::Owner
        ),
        SettingsVisibility::Hidden | SettingsVisibility::Unknown => false,
    }
}
