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
    private static RuntimeV4ExpertSupport _runtimeV4Expert = RuntimeV4ExpertSupport.Unconfigured();

    private static void InitializeRuntimeV3Gameplay()
    {
        _runtimeV3Gameplay = RuntimeV3GameplaySupport.Unconfigured();
        _runtimeV4Expert = RuntimeV4ExpertSupport.Unconfigured();
    }

    private static void ConfigureRuntimeV3Gameplay(IRuntimeV3HostSource source, IRuntimeV3HostThread thread)
    {
        _liveCombatSource = source as LiveCombatSource;
        _runtimeV3Gameplay = RuntimeV3GameplaySupport.WithHost(source, thread,
            () => _runtimeV2Pending is null);
        _runtimeV4Expert = _liveCombatSource is null
            ? RuntimeV4ExpertSupport.Unconfigured()
            : RuntimeV4ExpertSupport.WithHost(_liveCombatSource, thread);
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
        return _runtimeV4Expert.HandleState(
            new RuntimeV4ExpertContext(context.InstanceId, context.SessionId, context.LeaseId,
                ParseEpoch(context.LeaseEpoch), context.CorrelationId), out int status);
    }

    private static (int Status, string Response) ProcessRuntimeV4ExpertActionWork(
        RuntimeContext context, string body)
    {
        if (!TryAuthorizeRuntimeV2Context(context, out string error))
        {
            return (RuntimeRejected, RuntimeV2PlainError(error));
        }
        return _runtimeV4Expert.Handle(
            new RuntimeV4ExpertContext(context.InstanceId, context.SessionId, context.LeaseId,
                ParseEpoch(context.LeaseEpoch), context.CorrelationId), body, out int status);
    }
}
