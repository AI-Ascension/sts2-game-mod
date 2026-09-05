// SPDX-License-Identifier: MIT

use super::*;

impl<G: RuntimeV3GameplayGamePort> RuntimeV3GameplayMod<G> {
    pub(super) fn finish(
        &mut self,
        operation_id: &str,
        response: RuntimeV3GameplayMessage,
    ) -> RuntimeV3GameplayMessage {
        if let Some(receipt) = self.receipts.get_mut(operation_id) {
            receipt.response = response.clone();
        }
        response
    }

    pub(super) fn finish_unknown(
        &mut self,
        operation_id: &str,
        request: &RuntimeV3GameplayMessage,
        error_code: &str,
    ) -> RuntimeV3GameplayMessage {
        let response = self.result_response(
            request,
            RuntimeV3GameplayStatus::Unknown,
            None,
            None,
            None,
            Some(error_code),
            None,
        );
        self.finish(operation_id, response)
    }
}
