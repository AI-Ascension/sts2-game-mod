// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Pure admission postcondition used before scene-only lobby lookups.</summary>
internal static class CoopNativeLobbyAdmission
{
    internal static bool IsOwnedAndStarted(
        bool runInProgress, bool serviceConnected, bool serviceMatchesOwner) =>
        runInProgress && serviceConnected && serviceMatchesOwner;

    internal static bool TryAdmitOwnedRun(
        bool autoAdmitRun,
        bool runInProgress,
        bool serviceConnected,
        bool serviceMatchesOwner,
        Action admit)
    {
        if (!autoAdmitRun || !IsOwnedAndStarted(
                runInProgress, serviceConnected, serviceMatchesOwner))
        {
            return false;
        }

        admit();
        return true;
    }
}
