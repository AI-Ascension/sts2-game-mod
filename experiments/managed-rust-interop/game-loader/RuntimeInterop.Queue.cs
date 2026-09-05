// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static int RuntimeQueueCapacity()
    {
        string? value = System.Environment.GetEnvironmentVariable("STS2_RUNTIME_QUEUE_CAPACITY");
        return int.TryParse(value, out int capacity) && capacity >= 1 && capacity <= 64 ? capacity : 16;
    }
}
