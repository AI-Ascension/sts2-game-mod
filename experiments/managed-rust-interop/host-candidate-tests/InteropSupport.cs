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
    private const uint RuntimeRequestKindGameplay = 6;
    private const uint RuntimeRequestKindExpertState = 7;
    private static RuntimeV3GameplaySupport? _runtimeV3Gameplay;

    // This probe supplies only the shared v3 route. Production host wiring, including the
    // expert bridge, is compiled by GameLoaderProbe.csproj against the actual game host.
    private static void InitializeRuntimeV3Gameplay() =>
        _runtimeV3Gameplay = RuntimeV3GameplaySupport.Unconfigured();

    private static void ConfigureRuntimeV3Gameplay(
        IRuntimeV3HostSource source, IRuntimeV3HostThread thread) =>
        _runtimeV3Gameplay = RuntimeV3GameplaySupport.WithHost(source, thread,
            () => _runtimeV2Pending is null);

    private static (int Status, string Response) ProcessRuntimeV3GameplayWork(
        RuntimeContext context, string body)
    {
        if (!TryAuthorizeRuntimeV2Context(context, out string error))
            return (RuntimeRejected, RuntimeV2PlainError(error));
        RuntimeV3GameplaySupport support = _runtimeV3Gameplay
            ?? RuntimeV3GameplaySupport.Unconfigured();
        string response = support.Handle(context.InstanceId, context.SessionId,
            context.LeaseId, context.CorrelationId, context.LeaseEpoch, body, out int status);
        return (status, response);
    }

    private static (int Status, string Response) ProcessRuntimeV4ExpertWork(
        RuntimeContext context) =>
        (RuntimeUnavailable, "{\"error_code\":\"runtime_v4_expert_host_unavailable\"}");

    private static (int Status, string Response) ProcessRuntimeV4ExpertActionWork(
        RuntimeContext context, string body) =>
        (RuntimeUnavailable, "{\"error_code\":\"runtime_v4_expert_host_unavailable\"}");

    private const uint ExpectedAbiVersion = 1;
    private const int ExpectedCheckedAddResult = 42;
    private const string StatusNodeName = "probe";
    private static nint _nativeLibrary { get; set; }
    private static void AddStatusOverlay(Godot.SceneTree tree, uint abi, int result, bool force)
    {
        throw new InvalidOperationException("Host gameplay probe must not invoke the v1 overlay");
    }
}
