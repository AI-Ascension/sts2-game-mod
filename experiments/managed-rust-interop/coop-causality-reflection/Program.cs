// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.CoopCausalityReflection;

internal static class Program
{
    private static int Main(string[] args) => MetadataCausalityCli.Execute(args, Console.Out, Console.Error);
}
