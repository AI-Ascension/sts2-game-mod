// SPDX-License-Identifier: MIT

using System;
using System.Runtime.InteropServices;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const string RuntimeSessionVariable = "STS2_RUNTIME_SESSION";

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int RuntimeStop();

    internal static string ApplyRuntimeSettings()
    {
        if (_nativeLibrary == 0)
        {
            return "Saved. The listener will use these values when the addon loads.";
        }

        if (!StopRuntimeServer(_nativeLibrary, out string error))
        {
            return $"Saved, but the active listener could not be stopped: {error}.";
        }

        StartRuntimeServer(_nativeLibrary);
        if (_runtimeListenerStatus.StartsWith("Listening on ", StringComparison.Ordinal))
        {
            return $"Applied now: {_runtimeListenerStatus}.";
        }

        if (!StandaloneProfileSettings.RuntimeEnabled && !RuntimeSessionLaunchEnabled())
        {
            return "Applied now: Runtime API is disabled.";
        }

        return $"Saved, but the listener is {_runtimeListenerStatus}.";
    }

    private static bool StopRuntimeServer(nint nativeLibrary, out string error)
    {
        try
        {
            nint export = NativeLibrary.GetExport(nativeLibrary, "sts2_game_mod_runtime_stop");
            RuntimeStop stop = Marshal.GetDelegateForFunctionPointer<RuntimeStop>(export);
            int status = stop();
            if (status != 0)
            {
                error = $"native stop returned status {status}";
                GD.PrintErr($"{LogPrefix} runtime HTTP listener stop failed: status={status}");
                return false;
            }

            _runtimeListenerStatus = "Stopped";
            GD.Print($"{LogPrefix} runtime HTTP listener stopped for live settings update");
            error = string.Empty;
            return true;
        }
        catch (Exception exception)
        {
            error = exception.GetType().Name;
            GD.PrintErr($"{LogPrefix} runtime HTTP listener stop unavailable: {error}: {exception.Message}");
            return false;
        }
    }

    private static bool RuntimeSessionLaunchEnabled() => string.Equals(
        System.Environment.GetEnvironmentVariable(RuntimeSessionVariable),
        "1",
        StringComparison.Ordinal);

    private static bool TryReadPort(out ushort port)
    {
        string? value = System.Environment.GetEnvironmentVariable(RuntimePortVariable);
        if (string.IsNullOrWhiteSpace(value))
        {
            int configuredPort = StandaloneProfileSettings.RuntimePort;
            if (configuredPort <= 0 || configuredPort > ushort.MaxValue)
            {
                port = 0;
                return false;
            }

            port = (ushort)configuredPort;
            return true;
        }
        return ushort.TryParse(value, out port) && port > 0;
    }

    private static bool TryReadBindAddress(out string bindAddress)
    {
        string? value = System.Environment.GetEnvironmentVariable(RuntimeBindAddressVariable);
        if (string.IsNullOrWhiteSpace(value))
        {
            value = StandaloneProfileSettings.RuntimeBindAddress;
        }

        value = value.Trim();
        if (!StandaloneProfileSettings.IsValidRuntimeBindAddress(value))
        {
            bindAddress = string.Empty;
            return false;
        }

        bindAddress = value;
        return true;
    }
}
