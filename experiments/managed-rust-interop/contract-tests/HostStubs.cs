// SPDX-License-Identifier: MIT

using System;

// Original minimal test doubles: no host implementation, DLLs, saves or profiles.
namespace Godot
{
    internal static class Engine
    {
        internal static SceneTree Tree { get; } = new();
        internal static object GetMainLoop() => Tree;
    }
    internal sealed class SceneTree
    {
        internal Node Root { get; } = new();
    }
    internal sealed class Node
    {
        internal CanvasLayer? Overlay { get; set; }
        internal T? GetNodeOrNull<T>(string name) where T : class => Overlay as T;
    }
    internal sealed class CanvasLayer { }
}

namespace AiAscension.Sts2GameMod.Runtime
{
    public static partial class ModEntry
    {
        private const int RuntimeRequestKindState = 1;
        private const int RuntimeRequestKindAction = 2;
        private const int RuntimeAccepted = 200;
        private const int RuntimeRejected = 409;
        private const int RuntimeUnavailable = 503;
        private const uint ExpectedAbiVersion = 1;
        private const int ExpectedCheckedAddResult = 42;
        private const string StatusNodeName = "probe";
        private static ulong _runtimeGeneration;
        private static ulong _runtimeActionCount;
        private static int _hostCalls;
        private static bool _retainOverlay = true;
        private static void AddStatusOverlay(Godot.SceneTree tree, uint abi, int result, bool force)
        {
            _hostCalls++;
            tree.Root.Overlay = _retainOverlay ? new Godot.CanvasLayer() : null;
        }
    }
}
