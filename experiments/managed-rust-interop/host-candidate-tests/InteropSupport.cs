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
    private const uint ExpectedAbiVersion = 1;
    private const int ExpectedCheckedAddResult = 42;
    private const string StatusNodeName = "probe";
    private static nint _nativeLibrary { get; set; }
    private static void AddStatusOverlay(Godot.SceneTree tree, uint abi, int result, bool force)
    {
        throw new InvalidOperationException("Host gameplay probe must not invoke the v1 overlay");
    }
}
