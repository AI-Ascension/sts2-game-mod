// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using MegaCrit.Sts2.Core.Modding;

namespace Sts2.GameMod.ManagedInterop.GameLoaderProbe;

[ModInitializer(nameof(Initialize))]
public static class ModEntry
{
    private const uint ExpectedAbiVersion = 1;
    private static readonly object Gate = new();
    private static nint _nativeLibrary;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate uint AbiVersion();

    public static void Initialize()
    {
        lock (Gate)
        {
            if (_nativeLibrary != 0)
            {
                return;
            }

            string assemblyPath = Assembly.GetExecutingAssembly().Location;
            string directory = Path.GetDirectoryName(assemblyPath)
                ?? throw new InvalidOperationException("The managed probe has no assembly directory.");
            string nativePath = Path.Combine(directory, NativeLibraryFileName());
            nint candidate = NativeLibrary.Load(nativePath);

            try
            {
                nint export = NativeLibrary.GetExport(candidate, "sts2_game_mod_interop_abi_version");
                AbiVersion getVersion = Marshal.GetDelegateForFunctionPointer<AbiVersion>(export);
                uint version = getVersion();

                if (version != ExpectedAbiVersion)
                {
                    throw new InvalidOperationException(
                        $"Rust ABI mismatch: expected {ExpectedAbiVersion}, found {version}.");
                }

                _nativeLibrary = candidate;
            }
            catch
            {
                NativeLibrary.Free(candidate);
                throw;
            }
        }
    }

    private static string NativeLibraryFileName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return "sts2_game_mod_interop.dll";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return "libsts2_game_mod_interop.dylib";
        }

        return "libsts2_game_mod_interop.so";
    }
}
