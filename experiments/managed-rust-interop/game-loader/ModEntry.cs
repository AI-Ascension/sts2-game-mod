// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using Godot;
using MegaCrit.Sts2.Core.Modding;

namespace AiAscension.Sts2GameMod.Runtime;

[ModInitializer(nameof(Initialize))]
public static class ModEntry
{
    private const uint ExpectedAbiVersion = 1;
    private const int ExpectedCheckedAddStatus = 0;
    private const int ExpectedCheckedAddResult = 42;
    private const string LogPrefix = "[AI-ASCENSION STS2 POC]";
    private static readonly object Gate = new();
    private static nint _nativeLibrary;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate uint AbiVersion();

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int CheckedAdd(int left, int right, out int output);

    public static void Initialize()
    {
        lock (Gate)
        {
            if (_nativeLibrary != 0)
            {
                GD.Print($"{LogPrefix} already initialized");
                return;
            }

            nint candidate = 0;

            try
            {
                string assemblyPath = Assembly.GetExecutingAssembly().Location;
                string directory = Path.GetDirectoryName(assemblyPath)
                    ?? throw new InvalidOperationException("The addon has no assembly directory.");
                string nativePath = Path.Combine(directory, NativeLibraryFileName());
                candidate = NativeLibrary.Load(nativePath);

                nint export = NativeLibrary.GetExport(candidate, "sts2_game_mod_interop_abi_version");
                AbiVersion getVersion = Marshal.GetDelegateForFunctionPointer<AbiVersion>(export);
                uint version = getVersion();

                if (version != ExpectedAbiVersion)
                {
                    throw new InvalidOperationException($"ABI mismatch: expected {ExpectedAbiVersion}, found {version}.");
                }

                export = NativeLibrary.GetExport(candidate, "sts2_game_mod_interop_checked_add");
                CheckedAdd checkedAdd = Marshal.GetDelegateForFunctionPointer<CheckedAdd>(export);
                int status = checkedAdd(19, 23, out int sum);
                if (status != ExpectedCheckedAddStatus || sum != ExpectedCheckedAddResult)
                {
                    throw new InvalidOperationException(
                        $"native smoke call failed: status={status}, result={sum}");
                }

                _nativeLibrary = candidate;
                candidate = 0;
                GD.Print($"{LogPrefix} loaded managed entry point and Rust ABI; ABI={version}; 19+23={sum}");
            }
            catch (Exception exception)
            {
                if (candidate != 0)
                {
                    NativeLibrary.Free(candidate);
                }

                GD.PrintErr($"{LogPrefix} initialization failed: {exception.GetType().Name}: {exception.Message}");
            }
        }
    }

    private static string NativeLibraryFileName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return "ai_ascension_sts2_poc.dll";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return "libai_ascension_sts2_poc.dylib";
        }

        return "libai_ascension_sts2_poc.so";
    }
}
