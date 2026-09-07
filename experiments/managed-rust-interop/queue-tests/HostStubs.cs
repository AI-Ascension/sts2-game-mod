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
        internal static bool FailMainLoop { get; set; }
        internal static object GetMainLoop() => FailMainLoop
            ? throw new InvalidOperationException("private sentinel")
            : new SceneTree();
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
        internal static bool RuntimeEnabled { get; set; }
        internal static int RuntimePort => 15526;
        internal static string RuntimeBindAddress => "127.0.0.1";
        internal static bool IsValidRuntimeBindAddress(string value) => value == "127.0.0.1";
    }
    public static partial class ModEntry
    {
        private const string LogPrefix = "test";
        private static void TryFinalizePendingRuntimeV2() { }
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

        internal static void CheckListenerStartFailure()
        {
            string? previous = Environment.GetEnvironmentVariable(RuntimeTokenVariable);
            string? previousPort = Environment.GetEnvironmentVariable(RuntimePortVariable);
            string? previousBind = Environment.GetEnvironmentVariable(RuntimeBindAddressVariable);
            try
            {
                Environment.SetEnvironmentVariable(RuntimeTokenVariable, "synthetic-only");
                Environment.SetEnvironmentVariable(RuntimePortVariable, "15526");
                Environment.SetEnvironmentVariable(RuntimeBindAddressVariable, "127.0.0.1");
                StandaloneProfileSettings.RuntimeEnabled = true;
                Godot.Engine.FailMainLoop = true;
                StartRuntimeServer(0); // The synthetic pump throws before any native call.
                if (_runtimeListenerStatus != "Unavailable: InvalidOperationException")
                    throw new InvalidOperationException("listener failure status changed");
            }
            finally
            {
                Godot.Engine.FailMainLoop = false;
                StandaloneProfileSettings.RuntimeEnabled = false;
                Environment.SetEnvironmentVariable(RuntimeTokenVariable, previous);
                Environment.SetEnvironmentVariable(RuntimePortVariable, previousPort);
                Environment.SetEnvironmentVariable(RuntimeBindAddressVariable, previousBind);
            }
        }
    }
}
