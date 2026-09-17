// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

internal static class Program
{
    private static int Main(string[] args) => CoverageInventoryCli.Execute(args, Console.Out, Console.Error);
}
