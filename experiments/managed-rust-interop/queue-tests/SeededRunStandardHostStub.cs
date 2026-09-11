// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

// Source-only queue boundary double. It owns only the host-frame pump signal; seeded-run
// admission, native mutation, and readback remain covered by the exact-host source and probes.
internal static class SeededRunStandardHost
{
    private static bool _pendingMutation;

    internal static int PumpCount { get; private set; }

    internal static bool HasPendingMutation => _pendingMutation;

    internal static void Reset()
    {
        PumpCount = 0;
        _pendingMutation = false;
    }

    internal static void BeginPendingMutation()
    {
        if (_pendingMutation)
        {
            throw new InvalidOperationException("pending mutation already owned");
        }

        _pendingMutation = true;
    }

    internal static void Pump()
    {
        PumpCount++;
        _pendingMutation = false;
    }
}
