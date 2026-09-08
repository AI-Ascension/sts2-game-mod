// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class RuntimeMapV1ObservationFence
{
    internal const string ChangedSurfaceReason = "map_surface_changed";

    internal static bool IsStable(ulong capturedGeneration, ulong finalGeneration) =>
        capturedGeneration == finalGeneration;

    internal static RuntimeMapV1Snapshot RejectChangedSurface(
        RuntimeMapV1Snapshot snapshot, ulong finalGeneration) =>
        new(
            StateId: $"map-unavailable:{finalGeneration}",
            Generation: finalGeneration,
            SchemaVersion: snapshot.SchemaVersion,
            ProjectionVersion: snapshot.ProjectionVersion,
            GameBuild: snapshot.GameBuild,
            ModVersion: snapshot.ModVersion,
            MapInstanceId: snapshot.MapInstanceId,
            ActId: snapshot.ActId,
            ScopeId: snapshot.ScopeId,
            Availability: "unavailable",
            Completeness: "unknown",
            Freshness: "current",
            Reason: ChangedSurfaceReason,
            Nodes: Array.Empty<RuntimeMapV1Node>(),
            Edges: Array.Empty<RuntimeMapV1Edge>(),
            Position: new RuntimeMapV1Position("unavailable", null),
            History: Array.Empty<string>(),
            TerminalNodeIds: Array.Empty<string>(),
            Bindings: Array.Empty<RuntimeMapV1ActionBinding>());
}
