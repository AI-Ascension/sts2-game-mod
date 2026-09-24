// SPDX-License-Identifier: MIT

using System.Globalization;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Decides whether the host may offer <c>continue_run</c> beside <c>start_run</c> from a bounded,
/// host-owned summary. It adds no authority: the caller reads the summary from the same
/// SaveManager/RunManager guards <c>LiveCampaignStart.ResumeAsync</c> already relies on, and the
/// dispatch path re-checks them before any mutation.
/// </summary>
internal static class RuntimeV3GameplayContinuation
{
    internal const string Kind = "continue_run";

    /// <summary>Bounded native-owner summary. It carries no save payload, path, or account identity.</summary>
    internal readonly record struct Availability(
        bool ProfileInitialized,
        int ProfileId,
        bool HasRunSave,
        bool RunInProgress);

    /// <summary>
    /// A saved standard run is resumable only when the native owner reports an initialized
    /// profile, an existing run save, and no run already in progress. A missing or failed read is
    /// not resumable, so a broken host can never fabricate a continuation.
    /// </summary>
    internal static bool IsResumable(in Availability availability) =>
        availability.ProfileInitialized
        && availability.ProfileId is >= 1 and <= 3
        && availability.HasRunSave
        && !availability.RunInProgress;

    /// <summary>
    /// The offered continuation. A single detected run is named by omitting the optional
    /// discriminator, which is the accepted wire shape; naming a specific run would require host
    /// authority this boundary does not hold.
    /// </summary>
    internal static LegalActionReference Create(ulong generation) =>
        new(
            $"continue_run:{generation.ToString(CultureInfo.InvariantCulture)}",
            Kind,
            null,
            null,
            generation);
}
