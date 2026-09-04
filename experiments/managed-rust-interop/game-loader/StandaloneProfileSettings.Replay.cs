// SPDX-License-Identifier: MIT

using System;
using System.Threading;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private static Func<bool>? _replaySeedResetCallback;

    internal static bool IsReplaySeedResetAvailable =>
        Volatile.Read(ref _replaySeedResetCallback) is not null
        && SeedReplayController.CanReplayCurrentRun;

    internal static bool TryRegisterReplaySeedResetCallback(Func<bool>? callback)
    {
        if (callback is null)
        {
            GD.PrintErr($"{LogPrefix} replay seed reset callback registration rejected");
            return false;
        }

        return Interlocked.CompareExchange(ref _replaySeedResetCallback, callback, null) is null;
    }

    internal static bool TryRequestReplaySeedReset(out string status)
    {
        if (!AllowRepeatingSeeds)
        {
            status = "Enable repeating seeds before requesting a reset.";
            return false;
        }

        Func<bool>? callback = Volatile.Read(ref _replaySeedResetCallback);
        if (callback is null)
        {
            status = "Replay seed reset is unavailable.";
            return false;
        }

        try
        {
            if (callback())
            {
                status = "Replay seed reset requested.";
                return true;
            }
        }
        catch (Exception)
        {
            GD.PrintErr($"{LogPrefix} replay seed reset callback failed");
            status = "Replay seed reset failed safely.";
            return false;
        }

        status = "Replay seed reset was not accepted.";
        return false;
    }
}
