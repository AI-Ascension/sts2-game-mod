// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplaySupport
{
    private string ReadState(
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        ulong leaseEpoch,
        ulong requestGeneration,
        string responseKind,
        out int status)
    {
        if (!TryCurrent(null, out RuntimeV3GameplayObservation? observation, out IReadOnlyList<LegalActionReference>? actions, out string error))
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                responseKind, error, out status);
        }
        status = Accepted;
        if (observation is null || actions is null)
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                responseKind, "host_observation_unavailable", out status);
        }
        return ObservationEnvelope(
            responseKind,
            correlationId,
            instanceId,
            sessionId,
            leaseId,
            leaseEpoch,
            observation,
            actions,
            null,
            null,
            null,
            null);
    }

    private string ReadLegalActions(
        JsonElement root,
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        ulong leaseEpoch,
        ulong requestGeneration,
        out int status)
    {
        if (!TryString(root, "state_id", out string? requestedStateId))
        {
            return CatalogError(correlationId, "invalid_legal_actions_request", 400, out status);
        }
        if (!TryCurrent(
                null,
                out RuntimeV3GameplayObservation? observation,
                out IReadOnlyList<LegalActionReference>? actions,
                out string error))
        {
            return CatalogError(correlationId, error, Unavailable, out status);
        }
        if (observation is null || actions is null)
        {
            return CatalogError(correlationId, "host_observation_unavailable", Unavailable, out status);
        }
        if (!string.Equals(requestedStateId, observation.StateId, StringComparison.Ordinal)
            || observation.Generation != requestGeneration)
        {
            return CatalogError(correlationId, "stale_generation", Rejected, out status);
        }
        status = Accepted;
        return LegalActionsEnvelope(
            "legal_actions_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
            observation, actions);
    }

    private bool TryCurrent(
        ulong? requestGeneration,
        out RuntimeV3GameplayObservation? observation,
        out IReadOnlyList<LegalActionReference>? actions,
        out string error)
    {
        observation = null;
        actions = Array.Empty<LegalActionReference>();
        error = "host_not_configured";
        if (_host is null)
        {
            return false;
        }
        try
        {
            observation = _host.Observe();
            if (requestGeneration is not null && observation.Generation != requestGeneration)
            {
                observation = null;
                actions = Array.Empty<LegalActionReference>();
                error = "stale_generation";
                return false;
            }
            actions = _host.LegalActions(observation);
            if (!observation.Validate(out _))
            {
                observation = null;
                actions = Array.Empty<LegalActionReference>();
                error = "host_observation_unavailable";
                return false;
            }
            return true;
        }
        catch (Exception)
        {
            observation = null;
            actions = Array.Empty<LegalActionReference>();
            error = "host_observation_unavailable";
            return false;
        }
    }

}
