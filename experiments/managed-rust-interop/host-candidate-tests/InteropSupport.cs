// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

internal static class StandaloneProfileSettings
{
    internal static bool RuntimeEnabled => false;
    internal static int RuntimePort => 15526;
    internal static string RuntimeBindAddress => "127.0.0.1";
    internal static bool IsValidRuntimeBindAddress(string value) => value == "127.0.0.1";
}

public static partial class ModEntry
{
    private const uint RuntimeRequestKindCoopObservation = 9;
    private const uint RuntimeRequestKindCoopRecover = 13;
    private static bool _runtimeCoopPending;

    // The actual RuntimeV3GameplayInterop.cs supplies the v3 callback and helper under test.
    // This is only the co-op pending source needed by that production partial class; native
    // co-op itself is intentionally outside this source-only host-candidate probe.
    private static bool HasPendingCoopMutation => _runtimeCoopPending;

    // This source-only probe does not implement native co-op; accidental routing must fail.
    private static (int Status, string Response) ProcessCoopNativeWork(
        uint kind, RuntimeContext context, string body) =>
        throw new InvalidOperationException("Host gameplay probe crossed into native co-op");

    private const uint ExpectedAbiVersion = 1;
    private const int ExpectedCheckedAddResult = 42;
    private const string StatusNodeName = "probe";
    private static nint _nativeLibrary { get; set; }
    private static void AddStatusOverlay(Godot.SceneTree tree, uint abi, int result, bool force)
    {
        throw new InvalidOperationException("Host gameplay probe must not invoke the v1 overlay");
    }
}
