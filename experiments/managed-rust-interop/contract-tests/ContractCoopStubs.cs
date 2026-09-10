// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    // The v1 contract probe intentionally excludes host-backed co-op implementation files.
    // Keep the route range visible to RuntimeContract while returning the unconfigured boundary.
    private const uint RuntimeRequestKindCoopObservation = 16;
    private const uint RuntimeRequestKindCoopAction = 17;
    private const uint RuntimeRequestKindCoopVote = 18;
    private const uint RuntimeRequestKindCoopRejoin = 19;
    private const uint RuntimeRequestKindCoopRecover = 20;
    private const uint RuntimeRequestKindCoopLegalCatalog = 21;

    private static (int Status, string Response) ProcessCoopNativeWork(
        uint kind, RuntimeContext context, string body) =>
        (RuntimeUnavailable, "{\"error_code\":\"coop_native_unconfigured\"}");
}
