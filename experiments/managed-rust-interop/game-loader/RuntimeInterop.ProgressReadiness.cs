// SPDX-License-Identifier: MIT

using System;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static readonly object ProgressReadinessObserverGate = new();
    private static Action? _progressReadinessFrameCallback;
    private static bool _progressReadinessObserverAttempted;
    private static bool _progressReadinessObserverAttached;

    internal static bool TryInstallProgressReadinessObserver()
    {
        lock (ProgressReadinessObserverGate)
        {
            if (_progressReadinessObserverAttempted)
            {
                return _progressReadinessObserverAttached;
            }

            _progressReadinessObserverAttempted = true;
            try
            {
                if (Engine.GetMainLoop() is not SceneTree tree || tree.Root == null)
                {
                    ProgressReadinessRuntime.DisableSafely(ProgressReadinessRuntime.State);
                    return false;
                }

                ProgressReadinessRuntime.State.MarkObserverAttached();
                _progressReadinessFrameCallback = ObserveProgressReadinessOwnerThread;
                tree.ProcessFrame += _progressReadinessFrameCallback;
                _progressReadinessObserverAttached = true;
                return true;
            }
            catch (Exception)
            {
                ProgressReadinessRuntime.DisableSafely(ProgressReadinessRuntime.State);
                return false;
            }
        }
    }

    internal static bool TryCaptureProgressReadiness(out ProgressReadinessSnapshot snapshot)
    {
        try
        {
            return ProgressReadinessOptionalAdapter.TryCapture(out snapshot);
        }
        catch (Exception)
        {
            ProgressReadinessRuntime.DisableSafely(ProgressReadinessRuntime.State);
            snapshot = default;
            return false;
        }
    }

    private static void ObserveProgressReadinessOwnerThread()
    {
        try
        {
            ProgressReadinessRuntime.State.ObserveProcessFrame(System.Environment.CurrentManagedThreadId);
        }
        catch (Exception)
        {
            ProgressReadinessRuntime.DisableSafely(ProgressReadinessRuntime.State);
        }
    }
}
