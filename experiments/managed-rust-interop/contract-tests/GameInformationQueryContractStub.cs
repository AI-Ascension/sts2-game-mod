// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    // This source-only contract probe does not link the game-information adapter itself. The
    // production dispatch branch remains compiled and is exercised as an explicit unavailable
    // callback rather than being omitted from the dispatcher fixture.
    private const uint RuntimeRequestKindGameInformationQuery = 25;

    private static (int Status, string Response) ProcessGameInformationQueryWork(
        RuntimeContext _, string __) => (503, "{\"error_code\":\"query_probe_unavailable\"}");
}
