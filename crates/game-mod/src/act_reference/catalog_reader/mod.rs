// SPDX-License-Identifier: MIT

mod catalog;
mod lookup;
mod page;
mod reader;

pub use catalog::ActCatalog;
pub use page::*;
pub use reader::ActCatalogReader;

use crate::ContentUnlockState;

use super::{
    ActDefinition, ActVisibility, ActVisibilityScope, EncounterDefinition, EncounterPool,
    MapGenerationConstraint, RoomCategoryDefinition,
};

fn visible_act(definition: &ActDefinition, scope: ActVisibilityScope) -> bool {
    let unlocked = match definition.unlock_state {
        ContentUnlockState::Unlocked => true,
        ContentUnlockState::Locked => {
            matches!(
                scope,
                ActVisibilityScope::Reference | ActVisibilityScope::Owner
            )
        }
        ContentUnlockState::Unknown => false,
    };
    unlocked && visibility_allowed(definition.visibility, scope)
}

fn visible_encounter(encounter: &EncounterDefinition, scope: ActVisibilityScope) -> bool {
    visibility_allowed(encounter.visibility, scope)
}

fn visible_room_category(category: &RoomCategoryDefinition, scope: ActVisibilityScope) -> bool {
    visibility_allowed(category.visibility, scope)
}

fn visible_pool(pool: &EncounterPool, scope: ActVisibilityScope) -> bool {
    visibility_allowed(pool.visibility, scope)
}

fn visible_constraint(constraint: &MapGenerationConstraint, scope: ActVisibilityScope) -> bool {
    visibility_allowed(constraint.visibility, scope)
}

fn visibility_allowed(visibility: ActVisibility, scope: ActVisibilityScope) -> bool {
    match visibility {
        ActVisibility::Visible => true,
        ActVisibility::OwnerOnly => matches!(scope, ActVisibilityScope::Owner),
        ActVisibility::Hidden | ActVisibility::Unknown => false,
    }
}
