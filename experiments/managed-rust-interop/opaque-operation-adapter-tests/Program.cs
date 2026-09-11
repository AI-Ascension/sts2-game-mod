// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.OpaqueOperationAdapter;

internal static class Program
{
    private static void Main()
    {
        OpaqueOperationAdapterTests.Run();
        Console.WriteLine("Opaque operation adapter component tests passed.");
    }
}
