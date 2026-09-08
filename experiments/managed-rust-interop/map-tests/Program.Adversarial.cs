// SPDX-License-Identifier: MIT

using System.Collections.Generic;
using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class Program
{
    private static readonly int[] WorkBoundChild = { 1 };

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

        CheckFingerprintCollectionBounds();
    }

    private static void CheckFingerprintCollectionBounds()
    {
        int work = 0;
        int edgeCount = 0;
        int selected = 0;
        bool collected = RuntimeMapV1GraphFingerprint.TryCollectBoundedChildIds(
            InfiniteChildren(), _ =>
            {
                selected++;
                return "node:d";
            }, ref work, ref edgeCount, out List<string> childIds, out string reason);
        Check(!collected && reason == "map_edge_bound_exceeded"
            && childIds.Count == RuntimeMapV1Contract.MaxEdges
            && edgeCount == RuntimeMapV1Contract.MaxEdges
            && selected == RuntimeMapV1Contract.MaxEdges,
            "fingerprint child collection stops before appending beyond the total edge bound");

        work = RuntimeMapV1Contract.MaxMapTraversalWork;
        edgeCount = 0;
        collected = RuntimeMapV1GraphFingerprint.TryCollectBoundedChildIds(
            WorkBoundChild, _ => "node:d", ref work, ref edgeCount,
            out childIds, out reason);
        Check(!collected && reason == "map_graph_work_bound_exceeded"
            && childIds.Count == 0 && edgeCount == 0,
            "fingerprint child collection fails before appending beyond the work bound");
    }

    private static IEnumerable<int> InfiniteChildren()
    {
        for (int index = 0; ; index++) yield return index;
    }
}
