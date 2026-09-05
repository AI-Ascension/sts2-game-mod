// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const uint RuntimeRequestKindGameplay = 6;
    private static RuntimeV3GameplaySupport? _runtimeV3Gameplay;

    private static void InitializeRuntimeV3Gameplay()
    {
        _runtimeV3Gameplay = RuntimeV3GameplaySupport.Unconfigured();
        if (System.Environment.GetEnvironmentVariable("STS2_LIVE_COMBAT") == "1")
        {
            var source = new LiveCombatSource();
            ConfigureRuntimeV3Gameplay(source, source);
        }
    }

    private static void ConfigureRuntimeV3Gameplay(IRuntimeV3HostSource source, IRuntimeV3HostThread thread)
    {
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
}
