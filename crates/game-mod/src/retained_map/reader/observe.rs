// SPDX-License-Identifier: MIT

use super::super::binding::{
    RetainedMapFreshness, RetainedMapLiveBinding, RetainedMapObservationState,
};
use super::super::error::{RetainedMapError, RetainedMapSourceError};
use super::super::model::RetainedMapSnapshot;
use super::super::source::{RetainedMapCapability, RetainedMapSource, map_error};
use super::{RetainedMapReader, validate_replacement};

impl RetainedMapReader {
    /// Reobserves the map while permitted, recording only already-public topology.
    ///
    /// A closed, forbidden, unsupported, or unknown surface fails without opening the UI and
    /// revokes current authority. An unavailable capability also revokes current authority so a
    /// capability loss can never leave old travel authorized. A rejected read that proves a changed
    /// live identity or generation marks the retained knowledge stale so an old reference can never
    /// authorize travel. A replacement binding is checked against the retained identity before
    /// topology validation, so malformed new-run topology cannot discard identity-change evidence;
    /// a later failure still revokes the replacement. Transient failures that establish nothing
    /// leave state intact.
    ///
    /// Exit-path authority audit (from an unwithheld, current reader unless noted):
    ///
    /// | Exit/path | Observation | Freshness | `screen_open` | Old travel |
    /// |---|---|---|---|---|
    /// | Capability unavailable | reported if non-observable, else unchanged | `Retained` | `false` | Refused |
    /// | Preflight closed/forbidden/unsupported/unknown | reported state | `Retained` | `false` | Refused |
    /// | Read `Stale` | unchanged | `Stale` | `false` | Refused |
    /// | Read `NotObservable` | `Closed` | `Retained` | `false` | Refused |
    /// | Read `Busy`/`NoActiveSource`/`AccessDenied`/`Malformed` | unchanged | unchanged | unchanged | Preserved (transient) |
    /// | Binding mismatch or replacement rejection | unchanged | `Stale` | `false` | Refused |
    /// | Later binding/topology/limit failure | unchanged | `Stale` | `false` | Refused |
    /// | Malformed first observation | initial `Closed` | `NeverObserved` | `false` | Refused |
    pub fn observe<S: RetainedMapSource>(
        &mut self,
        source: &S,
        expected: &RetainedMapLiveBinding,
    ) -> Result<(), RetainedMapError> {
        let capability = source.capability();
        let observation = source.observation_state();
        if observation != RetainedMapObservationState::Observable {
            self.apply_closure_evidence(observation);
        }
        if let RetainedMapCapability::Unavailable(reason) = capability {
            self.revoke_current_authority();
            return Err(RetainedMapError::Unavailable(reason));
        }
        if observation != RetainedMapObservationState::Observable {
            return Err(RetainedMapError::MapNotObservable(observation));
        }
        let input = match source.read_snapshot(expected, self.scope) {
            Ok(input) => input,
            Err(error) => {
                self.apply_read_error(error);
                return Err(map_error(error));
            }
        };
        if input.binding != *expected {
            self.stale_identity();
            return Err(RetainedMapError::StaleSource);
        }
        if let Some(current) = &self.retained
            && let Err(error) = validate_replacement(current.binding(), &input.binding)
        {
            self.stale_identity();
            return Err(error);
        }
        let candidate = RetainedMapSnapshot::from_input(input).inspect_err(|_| {
            if self.retained.is_some() {
                self.stale_identity();
            }
        })?;
        self.retained = Some(candidate);
        self.observation = RetainedMapObservationState::Observable;
        self.freshness = RetainedMapFreshness::Current;
        self.screen_open = true;
        Ok(())
    }

    fn stale_identity(&mut self) {
        self.freshness = RetainedMapFreshness::Stale;
        self.screen_open = false;
    }

    /// Records a reported non-observable surface state and revokes current authority.
    ///
    /// Closure evidence never discards retained knowledge: a generation-current observation is
    /// only demoted to `Retained`.
    pub(super) fn apply_closure_evidence(&mut self, observation: RetainedMapObservationState) {
        self.observation = observation;
        self.revoke_current_authority();
    }

    /// Closes the surface and demotes a generation-current observation to `Retained`.
    fn revoke_current_authority(&mut self) {
        self.screen_open = false;
        if self.freshness == RetainedMapFreshness::Current {
            self.freshness = RetainedMapFreshness::Retained;
        }
    }

    /// Applies the evidence carried by a rejected source read before its error is returned.
    ///
    /// A closed source surface establishes closure, and a stale source establishes a changed
    /// generation; either revokes current authority. Transient failures that establish nothing
    /// about the live surface (`NoActiveSource`, `AccessDenied`, `Busy`, `Malformed`) leave the
    /// retained observation and freshness untouched.
    fn apply_read_error(&mut self, error: RetainedMapSourceError) {
        match error {
            RetainedMapSourceError::NotObservable => {
                self.apply_closure_evidence(RetainedMapObservationState::Closed);
            }
            RetainedMapSourceError::Stale => self.stale_identity(),
            RetainedMapSourceError::NoActiveSource
            | RetainedMapSourceError::AccessDenied
            | RetainedMapSourceError::Busy
            | RetainedMapSourceError::Malformed => {}
        }
    }
}
