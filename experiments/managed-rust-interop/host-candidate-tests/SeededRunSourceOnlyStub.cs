// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

// Source-only host-candidate double. The candidate exercises Runtime-v2 and v3 boundaries;
// native seeded-run admission and settlement remain outside this probe.
internal static class SeededRunStandardHost
{
    internal static void Pump()
    {
    }
}

public static partial class ModEntry
{
    private static (int, string) ProcessSeededRunWork(RuntimeWork work) =>
        (RuntimeUnavailable, "{\"error_code\":\"seeded_run_host_unavailable\"}");
}
