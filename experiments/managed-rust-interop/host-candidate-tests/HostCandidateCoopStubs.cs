// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

// Host-candidate tests compile shared routing and fencing code without the native co-op host
// implementation. Keep the source-only boundary explicitly unconfigured here.
internal static partial class SeededRunStandardHost
{
    internal static bool HasPendingMutation => false;
}

public static partial class ModEntry
{
    private const uint RuntimeRequestKindCoopObservation = 16;
    private const uint RuntimeRequestKindCoopAction = 17;
    private const uint RuntimeRequestKindCoopVote = 18;
    private const uint RuntimeRequestKindCoopRejoin = 19;
    private const uint RuntimeRequestKindCoopRecover = 20;
    private const uint RuntimeRequestKindCoopLegalCatalog = 21;

    internal static bool HasPendingCoopMutation => false;

    private static (int Status, string Response) ProcessCoopNativeWork(
        uint kind, RuntimeContext context, string body) =>
        (RuntimeUnavailable, "{\"error_code\":\"coop_native_unconfigured\"}");
}
