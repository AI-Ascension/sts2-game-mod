// SPDX-License-Identifier: MIT

using System;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static void InitializeProgressReadiness()
    {
        if (!TryInstallProgressReadinessObserver())
        {
            GD.PrintErr($"{LogPrefix} progress readiness owner observation unavailable");
        }

        bool lifecycleHooksInstalled;
        try
        {
            lifecycleHooksInstalled = OptionalProgressReadinessInstaller.TryInstall(
                ProgressReadinessRuntime.State,
                ProgressReadinessOptionalAdapter.TryInstallHooks);
        }
        catch (Exception)
        {
            ProgressReadinessRuntime.DisableSafely(ProgressReadinessRuntime.State);
            lifecycleHooksInstalled = false;
        }

        if (!lifecycleHooksInstalled)
        {
            GD.PrintErr($"{LogPrefix} progress readiness lifecycle hooks unavailable");
        }
    }
}
