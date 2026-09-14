// SPDX-License-Identifier: MIT

mod catalog;
mod page;
mod reader;

pub use catalog::EnemyCatalog;
pub use page::*;
pub use reader::EnemyCatalogReader;

use crate::ContentUnlockState;

use super::{EnemyDefinition, EnemyMoveDefinition, EnemyVisibility, EnemyVisibilityScope};

fn visible_definition(definition: &EnemyDefinition, scope: EnemyVisibilityScope) -> bool {
    let unlocked = match definition.unlock_state {
        ContentUnlockState::Unlocked => true,
        ContentUnlockState::Locked => {
            matches!(
                scope,
                EnemyVisibilityScope::Reference | EnemyVisibilityScope::Owner
            )
        }
        ContentUnlockState::Unknown => false,
    };
    unlocked && visibility_allowed(definition.visibility, scope)
}

fn visible_move(movement: &EnemyMoveDefinition, scope: EnemyVisibilityScope) -> bool {
    visibility_allowed(movement.visibility, scope)
}

fn visibility_allowed(visibility: EnemyVisibility, scope: EnemyVisibilityScope) -> bool {
    match visibility {
        EnemyVisibility::Visible => true,
        EnemyVisibility::OwnerOnly => matches!(scope, EnemyVisibilityScope::Owner),
        EnemyVisibility::Hidden | EnemyVisibility::Unknown => false,
    }
}
