// SPDX-License-Identifier: MIT

using System.Runtime.CompilerServices;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class ProgressReadinessOptionalAdapter
{
    [MethodImpl(MethodImplOptions.NoInlining)]
    internal static bool TryInstallHooks() => SaveManagerLifecyclePatches.TryInstall();

    [MethodImpl(MethodImplOptions.NoInlining)]
    internal static bool TryCapture(out ProgressReadinessSnapshot snapshot) =>
        SaveManagerLifecyclePatches.TryCaptureOwnedSnapshot(out snapshot);
}
