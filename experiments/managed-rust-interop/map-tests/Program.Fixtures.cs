// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class Program
{
    private static RuntimeMapV1Snapshot Snapshot() => new(
        StateId: "map-state-42",
        Generation: 42,
        SchemaVersion: RuntimeMapV1Contract.SnapshotSchemaVersion,
        ProjectionVersion: RuntimeMapV1Contract.ProjectionVersion,
        GameBuild: "0.107.1",
        ModVersion: "map-mod-1",
        MapInstanceId: "map-instance-1",
        ActId: 1,
        ScopeId: "campaign-1",
        Availability: "available",
        Completeness: "complete",
        Freshness: "current",
        Reason: null,
        Nodes: new[]
        {
            new RuntimeMapV1Node("map:1:0:0", 0, 0, "start", true),
            new RuntimeMapV1Node("map:1:1:0", 1, 0, "monster", true),
            new RuntimeMapV1Node("map:1:1:1", 1, 1, "event", false),
            new RuntimeMapV1Node("map:1:2:0", 2, 0, "boss", false)
        },
        Edges: new[]
        {
            new RuntimeMapV1Edge("map:1:0:0", "map:1:1:0"),
            new RuntimeMapV1Edge("map:1:0:0", "map:1:1:1"),
            new RuntimeMapV1Edge("map:1:1:0", "map:1:2:0"),
            new RuntimeMapV1Edge("map:1:1:1", "map:1:2:0")
        },
        Position: new RuntimeMapV1Position("current", "map:1:0:0"),
        History: new List<string> { "map:1:0:0" },
        TerminalNodeIds: new List<string> { "map:1:2:0" },
        Bindings: new[]
        {
            new RuntimeMapV1ActionBinding("map:1:1:0",
                "select_map_node:42:map:1:1:0", "host-action:42:map:1:1:0"),
            new RuntimeMapV1ActionBinding("map:1:1:1",
                "select_map_node:42:map:1:1:1", "host-action:42:map:1:1:1")
        });

    private static RuntimeMapV1Snapshot Unavailable() => Snapshot() with
    {
        StateId = "map-unavailable-42",
        MapInstanceId = null,
        ActId = null,
        ScopeId = null,
        Availability = "unavailable",
        Completeness = "unknown",
        Reason = "map_not_open",
        Nodes = Array.Empty<RuntimeMapV1Node>(),
        Edges = Array.Empty<RuntimeMapV1Edge>(),
        Position = new RuntimeMapV1Position("unavailable", null),
        History = Array.Empty<string>(),
        TerminalNodeIds = Array.Empty<string>(),
        Bindings = Array.Empty<RuntimeMapV1ActionBinding>()
    };
}
