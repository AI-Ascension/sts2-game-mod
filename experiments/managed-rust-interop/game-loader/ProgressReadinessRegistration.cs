// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Reflection;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class ProgressReadinessRuntime
{
    internal static readonly ProgressReadinessState State = new();

    internal static void DisableSafely(ProgressReadinessState state)
    {
        try
        {
            state.Disable();
        }
        catch (Exception)
        {
        }
    }
}

internal static class OptionalProgressReadinessInstaller
{
    internal static bool TryInstall(ProgressReadinessState state, Func<bool> install)
    {
        try
        {
            bool installed = install();
            if (!installed)
            {
                ProgressReadinessRuntime.DisableSafely(state);
            }

            return installed;
        }
        catch (Exception)
        {
            ProgressReadinessRuntime.DisableSafely(state);
            return false;
        }
    }
}

internal interface IProgressReadinessPatchRegistry
{
    bool InstallAndVerifyOwner(string target, string ownerId);
}

internal readonly record struct ProgressReadinessPatchBinding(string OwnerId, MethodInfo PatchMethod);

internal static class ProgressReadinessPatchVerifier
{
    internal static bool HasExactlyOneOwnedMethod(
        IEnumerable<ProgressReadinessPatchBinding> patches,
        string ownerId,
        MethodInfo expectedMethod)
    {
        int ownedCount = 0;
        int expectedCount = 0;
        foreach (ProgressReadinessPatchBinding patch in patches)
        {
            if (!string.Equals(patch.OwnerId, ownerId, StringComparison.Ordinal))
            {
                continue;
            }

            ownedCount++;
            if (patch.PatchMethod.Equals(expectedMethod))
            {
                expectedCount++;
            }
        }

        return ownedCount == 1 && expectedCount == 1;
    }
}

internal sealed class ProgressReadinessPatchRegistration
{
    internal const string OwnerId = "ai-ascension.game-mod.progress-readiness.v1";

    private static readonly string[] RequiredTargets =
    {
        "SaveManager.InitProfileId",
        "SaveManager.SwitchProfileId",
        "SaveManager.DeleteProfile",
        "SaveManager.InitProgressData",
        "SaveManager.Progress.set"
    };

    private bool _attempted;
    private bool _installed;

    internal static IReadOnlyList<string> Targets => Array.AsReadOnly(RequiredTargets);

    internal bool TryInstall(ProgressReadinessState state, IProgressReadinessPatchRegistry registry)
    {
        if (_attempted)
        {
            return _installed;
        }

        _attempted = true;
        if (!state.IsOperational)
        {
            return false;
        }

        foreach (string target in RequiredTargets)
        {
            try
            {
                if (!registry.InstallAndVerifyOwner(target, OwnerId))
                {
                    ProgressReadinessRuntime.DisableSafely(state);
                    return false;
                }
            }
            catch (Exception)
            {
                ProgressReadinessRuntime.DisableSafely(state);
                return false;
            }
        }

        state.MarkPatchesVerified();
        _installed = state.IsOperational;
        return _installed;
    }
}
