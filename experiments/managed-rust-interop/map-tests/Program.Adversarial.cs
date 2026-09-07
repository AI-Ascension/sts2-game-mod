// SPDX-License-Identifier: MIT

using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class Program
{
    private static void CheckAdversarialGraphRejection()
    {
        RuntimeMapV1Snapshot duplicateNode = Snapshot() with
        {
            Nodes = Snapshot().Nodes.Append(Snapshot().Nodes[0]).ToArray()
        };
        Check(!duplicateNode.Validate(out _), "duplicate node IDs are rejected");

        RuntimeMapV1Snapshot duplicateCoordinate = Snapshot() with
        {
            Nodes = Snapshot().Nodes.Append(new RuntimeMapV1Node(
                "map:1:0:9", 0, 0, "monster", false)).ToArray()
        };
        Check(duplicateCoordinate.Validate(out string duplicateError),
            $"duplicate coordinates are preserved: {duplicateError}");

        RuntimeMapV1Snapshot distinctActionPayload = Snapshot() with
        {
            Bindings = new[] { new RuntimeMapV1ActionBinding(
                "map:1:1:0", "select_map_node:42:legacy-map-option", "legacy-map-option") }
        };
        Check(distinctActionPayload.Validate(out _),
            "host action payload may remain distinct from the stable graph node ID");

        RuntimeMapV1Snapshot cycle = Snapshot() with
        {
            Edges = Snapshot().Edges.Append(new RuntimeMapV1Edge(
                "map:1:2:0", "map:1:0:0")).ToArray()
        };
        Check(!cycle.Validate(out _), "cyclic topology is rejected");

        RuntimeMapV1Snapshot staleBinding = Snapshot() with
        {
            Bindings = new[] { new RuntimeMapV1ActionBinding(
                "map:1:1:0", "select_map_node:41:map:1:1:0", "map:1:1:0") }
        };
        Check(staleBinding.Validate(out _),
            "binding generation remains an opaque host action identity at snapshot validation");
        RuntimeMapV1Snapshot duplicateOption = Snapshot() with
        {
            Bindings = new[]
            {
                new RuntimeMapV1ActionBinding(
                    "map:1:1:0", "select_map_node:41:left", "map-option:shared"),
                new RuntimeMapV1ActionBinding(
                    "map:1:1:1", "select_map_node:41:right", "map-option:shared")
            }
        };
        Check(!duplicateOption.Validate(out _),
            "serialized action option IDs are unique within a snapshot");
        RuntimeMapV1Snapshot currentBinding = Snapshot() with
        {
            Position = new RuntimeMapV1Position("current", "map:1:1:0")
        };
        Check(!currentBinding.Validate(out _),
            "the current node cannot also be exposed as a travel binding");
        Check(!RuntimeMapV1Contract.IsHostActionId("bad action"),
            "unsafe host action identity is rejected");
    }
}
