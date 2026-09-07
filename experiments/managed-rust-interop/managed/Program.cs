// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Runtime.InteropServices;

namespace Sts2.GameMod.ManagedInterop.Spike;

internal static class Program
{
    private const uint ExpectedAbiVersion = 1;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate uint AbiVersion();

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int CheckedAdd(int left, int right, out int output);

    private static int Main(string[] args)
    {
        if (args.Length != 1)
        {
            Console.Error.WriteLine("Usage: Sts2.GameMod.ManagedInterop.Spike <absolute-native-library-path>");
            return 2;
        }

        string libraryPath = Path.GetFullPath(args[0]);
        nint library = NativeLibrary.Load(libraryPath);

        try
        {
            AbiVersion getVersion = LoadExport<AbiVersion>(library, "sts2_game_mod_interop_abi_version");
            CheckedAdd checkedAdd = LoadExport<CheckedAdd>(library, "sts2_game_mod_interop_checked_add");

            uint version = getVersion();
            if (version != ExpectedAbiVersion)
            {
                Console.Error.WriteLine($"ABI mismatch: expected {ExpectedAbiVersion}, found {version}.");
                return 3;
            }

            int status = checkedAdd(19, 23, out int sum);
            if (status != 0 || sum != 42)
            {
                Console.Error.WriteLine($"Call mismatch: status={status}, result={sum}.");
                return 4;
            }

            Console.WriteLine($"managed-net9 -> rust-cdylib: ok (ABI {version}, 19 + 23 = {sum})");
            return 0;
        }
        finally
        {
            NativeLibrary.Free(library);
        }
    }

    private static T LoadExport<T>(nint library, string name) where T : Delegate
    {
        nint address = NativeLibrary.GetExport(library, name);
        return Marshal.GetDelegateForFunctionPointer<T>(address);
    }
}
