// SPDX-License-Identifier: MIT

using System;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Reads the native-owner resumable-run summary for the live campaign host. It reads only the
/// guards <c>LiveCampaignStart.ResumeAsync</c> already uses; it never loads, selects, or mutates a
/// save, and any host read failure is reported as not resumable.
/// </summary>
internal static class LiveCampaignContinuation
{
    /// <summary>
    /// Whether the live host may offer a continuation beside <c>start_run</c>. Practice runs and
    /// the map-bound fixture never continue, and the standard resume path owns the actual
    /// compatibility check at dispatch.
    /// </summary>
    internal static bool Available() =>
        LiveCombatDemo.Campaign
        && !LiveCombatDemo.RunOptions.Practice
        && !LiveCombatDemo.CampaignMapBound
        && RuntimeV3GameplayContinuation.IsResumable(Read());

    private static RuntimeV3GameplayContinuation.Availability Read()
    {
        try
        {
            SaveManager saves = SaveManager.Instance;
            RunManager runs = RunManager.Instance;
            return new RuntimeV3GameplayContinuation.Availability(
                saves.IsProfileInitialized,
                saves.CurrentProfileId,
                saves.HasRunSave,
                runs.IsInProgress || runs.IsCleaningUp || runs.DebugOnlyGetState() is not null);
        }
        catch (Exception)
        {
            return default;
        }
    }
}
