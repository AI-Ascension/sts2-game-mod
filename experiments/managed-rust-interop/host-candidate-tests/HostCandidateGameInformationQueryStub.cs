// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    // Host-candidate source probes do not load the game-information adapter. Keep callback 25
    // fail-closed while compiling the production dispatcher wiring.
    private static (int Status, string Response) ProcessGameInformationQueryWork(
        RuntimeContext _, string __) => (503, "{\"error_code\":\"query_probe_unavailable\"}");
}
