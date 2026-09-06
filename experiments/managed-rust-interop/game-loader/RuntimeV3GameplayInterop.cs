// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const uint RuntimeRequestKindGameplay = 6;
    private const uint RuntimeRequestKindExpertState = 7;
    private static RuntimeV3GameplaySupport? _runtimeV3Gameplay;
    private static LiveCombatSource? _liveCombatSource;

    private static void InitializeRuntimeV3Gameplay()
    {
        _runtimeV3Gameplay = RuntimeV3GameplaySupport.Unconfigured();
    }

    private static void ConfigureRuntimeV3Gameplay(IRuntimeV3HostSource source, IRuntimeV3HostThread thread)
    {
        _liveCombatSource = source as LiveCombatSource;
        _runtimeV3Gameplay = RuntimeV3GameplaySupport.WithHost(source, thread,
            () => _runtimeV2Pending is null);
    }

    private static (int Status, string Response) ProcessRuntimeV3GameplayWork(
        RuntimeContext context,
        string body)
    {
        if (!TryAuthorizeRuntimeV2Context(context, out string error))
        {
            return (RuntimeRejected, RuntimeV2PlainError(error));
        }
        RuntimeV3GameplaySupport support = _runtimeV3Gameplay ?? RuntimeV3GameplaySupport.Unconfigured();
        string response = support.Handle(
            context.InstanceId,
            context.SessionId,
            context.LeaseId,
            context.CorrelationId,
            context.LeaseEpoch,
            body,
            out int status);
        return (status, response);
    }

    private static (int Status, string Response) ProcessRuntimeV4ExpertWork(
        RuntimeContext context)
    {
        if (!TryAuthorizeRuntimeV2Context(context, out string error))
        {
            return (RuntimeRejected, RuntimeV2PlainError(error));
        }
        LiveCombatSource? source = _liveCombatSource;
        if (source is null)
        {
            return (RuntimeUnavailable, "{\"error_code\":\"runtime_v4_expert_host_unavailable\"}");
        }
        try
        {
            return source.TrySerializeExpert(out string json, out string serializationError)
                ? (RuntimeAccepted, json)
                : (RuntimeUnavailable, JsonSerializer.Serialize(new { error_code = serializationError }));
        }
        catch (Exception)
        {
            return (RuntimeUnavailable, "{\"error_code\":\"runtime_v4_expert_observation_unavailable\"}");
        }
    }
}
