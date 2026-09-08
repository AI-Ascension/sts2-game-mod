// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    // Runtime-v4 owns callback IDs 7 (expert state) and 8 (expert action). Keep the co-op
    // callbacks in a disjoint range so a combined managed/native addon cannot dispatch an
    // expert request into the co-op adapter or vice versa.
    private const uint RuntimeRequestKindCoopObservation = 9;
    private const uint RuntimeRequestKindCoopAction = 10;
    private const uint RuntimeRequestKindCoopVote = 11;
    private const uint RuntimeRequestKindCoopRejoin = 12;
    private const uint RuntimeRequestKindCoopRecover = 13;
    private static CoopNativeRuntime? _coopNativeRuntime;

    /// <summary>
    /// True while a co-op action or vote has been accepted by the host adapter but its native
    /// effect is not yet settled. The gameplay profiles use this predicate to serialize their
    /// own mutations against the co-op profile.
    /// </summary>
    internal static bool HasPendingCoopMutation => _coopNativeRuntime?.HasPendingMutation ?? false;

    /// <summary>
    /// Installs the managed host port; the caller must provide a game-thread port. The optional
    /// predicate must return true only when the other mutation profiles are idle. It is checked
    /// immediately before each native co-op mutation, after a fresh co-op observation.
    /// </summary>
    internal static void ConfigureCoopNative(ICoopNativeHostPort port, Func<bool>? canDispatch = null)
    {
        _coopNativeRuntime = new CoopNativeRuntime(port,
            canDispatch ?? (() => _runtimeV2Pending is null
                && !(_runtimeV3Gameplay?.HasPendingMutation ?? false)
                && !HasPendingRuntimeV4ExpertMutation()));
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
