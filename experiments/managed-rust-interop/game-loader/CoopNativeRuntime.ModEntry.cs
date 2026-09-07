// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const uint RuntimeRequestKindCoopObservation = 7;
    private const uint RuntimeRequestKindCoopAction = 8;
    private const uint RuntimeRequestKindCoopVote = 9;
    private const uint RuntimeRequestKindCoopRejoin = 10;
    private const uint RuntimeRequestKindCoopRecover = 11;
    private static CoopNativeRuntime? _coopNativeRuntime;

    /// <summary>Installs the managed host port; the caller must provide a game-thread port.</summary>
    internal static void ConfigureCoopNative(ICoopNativeHostPort port)
    {
        _coopNativeRuntime = new CoopNativeRuntime(port);
#if STS2_NATIVE_COOP_PROBE
        CoopNativeLobbyController.StartIfConfigured();
        CoopNativeLobbyProbe.StartIfEnabled(port);
#endif
    }

    /// <summary>Explicit route hook for RuntimeInterop's native callback dispatcher.</summary>
    private static (int Status, string Response) ProcessCoopNativeWork(
        uint kind, RuntimeContext context, string body)
    {
        CoopNativeRuntime? runtime = _coopNativeRuntime;
        if (runtime is null)
        {
            return (503, "{\"error_code\":\"coop_native_unconfigured\"}");
        }
        string expected = kind switch
        {
            RuntimeRequestKindCoopObservation => "observation",
            RuntimeRequestKindCoopAction => "local_action_request",
            RuntimeRequestKindCoopVote => "shared_vote_request",
            RuntimeRequestKindCoopRejoin => "rejoin_request",
            RuntimeRequestKindCoopRecover => "recovery_response",
            _ => string.Empty
        };
        return expected.Length == 0
            ? (400, "{\"error_code\":\"coop_native_unknown_route\"}")
            : runtime.Handle(context.InstanceId, context.SessionId, context.LeaseId,
                context.CorrelationId, context.LeaseEpoch, expected, body);
    }
}
