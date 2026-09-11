// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using Godot;
using MegaCrit.Sts2.Core.Nodes;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class CoopNativeLobbyProbe
{
    private sealed partial class ProbeState
    {
        private long _processFrameTicks;

        internal long ProcessFrameTicks => _processFrameTicks;

        internal void RecordProcessFrame() => _processFrameTicks++;
    }

    private static void AddRuntimeSnapshot(
        Dictionary<string, object?> common, SceneRuntimeSnapshot runtime)
    {
        common["runtime"] = new Dictionary<string, object?>
        {
            ["scene_tree_paused"] = runtime.SceneTreePaused,
            ["nrun_valid"] = runtime.NRunValid,
            ["nrun_can_process"] = runtime.NRunCanProcess,
            ["nrun_process_mode"] = runtime.NRunProcessMode,
            ["window_has_focus"] = runtime.WindowHasFocus,
            ["window_mode"] = runtime.WindowMode,
            ["process_frame_ticks"] = runtime.ProcessFrameTicks
        };
    }

    private static SceneRuntimeSnapshot ReadRuntimeSnapshot(SceneTree tree, ProbeState state)
    {
        NRun? run = NRun.Instance;
        bool runValid = run is not null && GodotObject.IsInstanceValid(run);
        Window? window = tree.Root is null ? null : tree.Root.GetWindow();
        return new SceneRuntimeSnapshot(
            tree.Paused,
            runValid,
            runValid && run!.CanProcess(),
            runValid ? run!.ProcessMode.ToString().ToLowerInvariant() : null,
            window is not null && window.HasFocus(),
            DisplayServer.WindowGetMode().ToString().ToLowerInvariant(),
            state.ProcessFrameTicks);
    }

    private sealed record SceneRuntimeSnapshot(
        bool SceneTreePaused,
        bool NRunValid,
        bool NRunCanProcess,
        string? NRunProcessMode,
        bool WindowHasFocus,
        string WindowMode,
        long ProcessFrameTicks);
}
