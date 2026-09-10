// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static object? _runtimeV2Pending;
    private static TestGameplaySupport? _runtimeV3Gameplay;
    private static bool _runtimeV4Pending;
    private static bool _seededPending;

    private static bool HasPendingRuntimeV4ExpertMutation() => _runtimeV4Pending;

    internal static void SetPendingProfiles(
        bool runtimeV2, bool runtimeV3, bool runtimeV4, bool seeded = false)
    {
        _runtimeV2Pending = runtimeV2 ? new object() : null;
        _runtimeV3Gameplay = runtimeV3 ? new TestGameplaySupport() : null;
        _runtimeV4Pending = runtimeV4;
        _seededPending = seeded;
    }

    internal static bool HasPendingNonSeededMutation() =>
        HasPendingNonCoopMutation() || HasPendingCoopMutation;

    internal static bool HasPendingNonCoopMutation() =>
        _runtimeV2Pending is not null || (_runtimeV3Gameplay?.HasPendingMutation ?? false)
            || HasPendingRuntimeV4ExpertMutation();

    internal static bool TestSeededPending => _seededPending;

    internal static bool HasPendingNonExpertMutation() =>
        HasPendingNonSeededMutation() || SeededRunStandardHost.HasPendingMutation;

    internal static CoopOperationReceipt DispatchTest(CoopLocalActionRequest request)
    {
        CoopNativeRuntime runtime = _coopNativeRuntime
            ?? throw new InvalidOperationException("co-op runtime was not configured");
        return runtime.TestDispatch(request);
    }

    internal static bool ReconcileTest(string operationId, out CoopOperationReceipt? receipt)
    {
        CoopNativeRuntime runtime = _coopNativeRuntime
            ?? throw new InvalidOperationException("co-op runtime was not configured");
        return runtime.TestReconcile(operationId, out receipt);
    }
}

internal sealed class TestGameplaySupport
{
    private readonly bool _pending = true;

    internal bool HasPendingMutation => _pending;
}

// This probe links the production profile gate without starting a native session.
internal static class NativeCoopSessionController
{
    internal static void StartIfConfigured() { }
}

internal static class SeededRunStandardHost
{
    internal static bool HasPendingMutation => ModEntry.TestSeededPending;
}
