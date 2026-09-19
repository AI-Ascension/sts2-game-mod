// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    /// <summary>
    /// Runs the launch contract. A refusal is logged exactly as an initialization failure always was
    /// and recorded before it is thrown, so the loader can keep the listener alive and let a runtime
    /// consumer read the same reason the log names.
    /// </summary>
    private static bool LaunchContractDeclined()
    {
        try
        {
            LiveCombatDemo.Initialize();
            return false;
        }
        catch (Exception exception) when (LaunchContractRefusal.Recorded.Length != 0)
        {
            GD.PrintErr(
                $"{LogPrefix} initialization failed: {exception.GetType().Name}: {exception.Message}");
            return true;
        }
    }

    /// <summary>
    /// Keeps the listener alive for a refused launch contract. The recovery code is the only channel
    /// a runtime consumer reads, and starting nothing is what made a refusal indistinguishable from
    /// a port that never opened (sts2-game-mod#185). The profile-touching bootstrap stays skipped,
    /// because the refusal is exactly the statement that this profile is not the isolated one the
    /// contract requires.
    /// </summary>
    private static void StartRefusedRuntime()
    {
        InitializeRuntimeV3Gameplay();
        ConfigureCoopNative(new InstalledNativeCoopHostPort());
        InitializeRuntimeMapV1();
        StartRuntimeServer(_nativeLibrary);
    }
}
