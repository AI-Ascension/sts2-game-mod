// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Managed composition seam for Runtime-v3. It accepts only the versioned semantic envelope and
/// delegates observations, catalog generation, and mutation to the host-thread owner.
/// </summary>
internal sealed partial class RuntimeV3GameplaySupport
{
    private const int Accepted = 200;
    private const int Rejected = 409;
    private const int Unavailable = 503;
    private const int MaxRequestBytes = 128 * 1024;
    private const int MaxResponseBytes = 128 * 1024;
    private static readonly string[] EnvelopeFields =
    {
        "protocol_version", "schema_digest", "provenance", "correlation_id", "instance_id",
        "session_id", "lease_id", "lease_epoch", "generation", "kind", "state_id",
        "operation_id", "observation", "legal_actions", "action", "status", "transition",
        "error_code", "wait_for_millis", "wait_outcome", "recovery"
    };

    private readonly RuntimeV3GameplayHost? _host;

    private RuntimeV3GameplaySupport(RuntimeV3GameplayHost? host)
    {
        _host = host;
    }

    internal static RuntimeV3GameplaySupport Unconfigured() => new(null);

    internal static RuntimeV3GameplaySupport WithHost(
        IRuntimeV3HostSource source,
        IRuntimeV3HostThread thread,
        Func<bool>? canDispatch = null) =>
        new(new RuntimeV3GameplayHost(source, thread, canDispatch));

    internal bool HasPendingMutation => _host?.HasPendingMutation ?? false;

    internal string Handle(
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        string leaseEpochText,
        string body,
        out int status)
    {
        status = Unavailable;
        if (body.Length > MaxRequestBytes
            || !TryParseRequest(
                instanceId,
                sessionId,
                leaseId,
                correlationId,
                leaseEpochText,
                body,
                out JsonDocument? document,
                out JsonElement root,
                out ulong requestGeneration,
                out string kind,
                out _))
        {
            return ErrorEnvelope(
                correlationId,
                instanceId,
                sessionId,
                leaseId,
                ParseEpoch(leaseEpochText),
                0,
                "state_response",
                "invalid_runtime_v3_envelope",
                out status);
        }

        using (JsonDocument parsedDocument = document!)
        {
            return HandleValidated(root, kind, instanceId, sessionId, leaseId, correlationId,
                ParseEpoch(leaseEpochText), requestGeneration, out status);
        }
    }


    private string HandleValidated(JsonElement root, string kind, string instanceId,
        string sessionId, string leaseId, string correlationId, ulong leaseEpoch,
        ulong requestGeneration, out int status) => kind switch
    {
        "state_request" => ReadState(instanceId, sessionId, leaseId, correlationId,
            leaseEpoch, requestGeneration, "state_response", out status),
        "reobserve_request" => ReadState(instanceId, sessionId, leaseId, correlationId,
            leaseEpoch, requestGeneration, "reobserve_response", out status),
        "legal_actions_request" => ReadLegalActions(root, instanceId, sessionId, leaseId,
            correlationId, leaseEpoch, requestGeneration, out status),
        "dispatch_action_request" => Dispatch(root, instanceId, sessionId, leaseId,
            correlationId, leaseEpoch, requestGeneration, out status),
        "wait_request" => Wait(root, instanceId, sessionId, leaseId, correlationId,
            leaseEpoch, requestGeneration, out status),
        "recover_request" => Recover(root, instanceId, sessionId, leaseId, correlationId,
            leaseEpoch, requestGeneration, out status),
        _ => ErrorEnvelope(correlationId, instanceId, sessionId, leaseId, leaseEpoch,
            requestGeneration, "state_response", "unsupported_runtime_v3_operation", out status)
    };
}
