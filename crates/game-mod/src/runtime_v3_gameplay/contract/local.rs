// SPDX-License-Identifier: MIT

use super::*;

/// Local authorization scope; a value alone does not grant host authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeV3GameplayIdentity {
    pub instance_id: String,
    pub session_id: String,
    pub lease_id: String,
    pub lease_epoch: u64,
}

impl RuntimeV3GameplayIdentity {
    #[must_use]
    pub fn new(
        instance_id: impl Into<String>,
        session_id: impl Into<String>,
        lease_id: impl Into<String>,
        lease_epoch: u64,
    ) -> Self {
        Self {
            instance_id: instance_id.into(),
            session_id: session_id.into(),
            lease_id: lease_id.into(),
            lease_epoch,
        }
    }
}

impl RuntimeV3GameplayMessage {
    /// Constructs a wait request; `validate` enforces the permitted time bound.
    #[must_use]
    pub fn wait_request(
        context: RuntimeV3GameplayContext,
        generation: u64,
        operation_id: impl Into<String>,
        wait_for_millis: u32,
    ) -> Self {
        Self {
            operation_id: Some(operation_id.into()),
            wait_for_millis: Some(wait_for_millis),
            ..Self::base(
                context,
                generation,
                RuntimeV3GameplayMessageKind::WaitRequest,
            )
        }
    }
}
