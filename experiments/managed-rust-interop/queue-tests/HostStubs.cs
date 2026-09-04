// SPDX-License-Identifier: MIT

using System;
using System.Runtime.InteropServices;

// Original minimal test doubles, not host implementation or compatibility evidence.
namespace Godot
{
    internal static class GD
    {
        internal static void Print(string value) { }
        internal static void PrintErr(string value)
        {
            if (value.Contains("private sentinel", StringComparison.Ordinal))
                throw new InvalidOperationException("private exception leaked");
        }
    }
    internal static class Engine
    {
        internal static object GetMainLoop() => new SceneTree();
    }
    internal sealed class SceneTree
    {
        internal object Root { get; } = new();
        internal event Action? ProcessFrame;
        internal void Pump() => ProcessFrame?.Invoke();
    }
}

namespace AiAscension.Sts2GameMod.Runtime
{
    internal static class StandaloneProfileSettings
    {
        internal static bool RuntimeEnabled => false;
        internal static int RuntimePort => 15526;
        internal static string RuntimeBindAddress => "127.0.0.1";
        internal static bool IsValidRuntimeBindAddress(string value) => value == "127.0.0.1";
    }
    public static partial class ModEntry
    {
        private const string LogPrefix = "test";
        private static bool RuntimeSessionLaunchEnabled() => false;
        private static (int, string) ProcessRuntimeWork(RuntimeWork work)
        {
            _runtimeGeneration++;
            _runtimeActionCount++;
            throw new InvalidOperationException("private sentinel");
        }
        internal static void CheckCallback()
        {
            nint request = Marshal.AllocHGlobal(Marshal.SizeOf<NativeRuntimeRequest>());
            nint output = Marshal.AllocHGlobal(1024);
            try
            {
                Marshal.StructureToPtr(new NativeRuntimeRequest(), request, false);
                if (HandleRuntimeRequest(request, output, 1024, out nuint length) != 503 || length == 0)
                    throw new InvalidOperationException("unavailable callback");
                var work = new RuntimeWork(2, default, "{}");
                if (ExecuteRuntimeWork(work) != (503, "{\"error_code\":\"main_thread_outcome_unknown\"}"))
                    throw new InvalidOperationException("exception incorrectly claims rejection");
            }
            finally
            {
                Marshal.FreeHGlobal(request);
                Marshal.FreeHGlobal(output);
            }
        }
    }
}
