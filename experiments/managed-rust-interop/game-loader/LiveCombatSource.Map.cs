// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Host-thread map projection backed by the installed run's public map model.</summary>
internal sealed partial class LiveCombatSource
{
    private object? _mapRunIdentity;
    private object? _mapObjectIdentity;
    private string? _mapInstanceId;

    public RuntimeMapV1Snapshot ObserveMap()
    {
        RequireThread();

        // The gameplay observation owns the shared generation fence. It also makes a map
        // topology or navigation-catalog change visible to the existing runtime-v3 fence.
        RuntimeV3GameplayObservation observation = Observe();
        try
        {
            if (!LiveCombatDemo.Campaign || !RunManager.Instance.IsInProgress)
                return UnavailableMap(observation.Generation, "campaign_not_active");

            RunState? run = RunManager.Instance.DebugOnlyGetState();
            if (run?.Map == null)
                return UnavailableMap(observation.Generation, "map_state_unavailable");

            string mapInstanceId = EnsureMapInstanceId(run, run.Map);
            NMapScreen? mapScreen = NMapScreen.Instance;
            if (mapScreen?.IsOpen != true || !mapScreen.IsVisibleInTree())
                return UnavailableMap(observation.Generation, "map_not_open", mapInstanceId,
                    run.CurrentActIndex);

            return BuildMapSnapshot(run, mapInstanceId, observation);
        }
        catch
        {
            return UnavailableMap(observation.Generation, "map_projection_failed");
        }
    }

    private RuntimeMapV1Snapshot BuildMapSnapshot(RunState run, string mapInstanceId,
        RuntimeV3GameplayObservation observation)
    {
        ulong generation = observation.Generation;
        ActMap map = run.Map;
        List<MapPoint> points = CollectMapPoints(map)
            .OrderBy(point => point.coord.row)
            .ThenBy(point => point.coord.col)
            .ThenBy(point => (int)point.PointType)
            .ToList();
        if (points.Count > RuntimeMapV1Contract.MaxNodes)
            return UnavailableMap(generation, "map_node_bound_exceeded", mapInstanceId,
                run.CurrentActIndex);

        var byCoordinate = new Dictionary<(int Row, int Column), MapPoint>();
        foreach (MapPoint point in points)
        {
            // A coordinate is a location hint, not a node identity. Keep the first stable
            // representative for current/history lookup while preserving every node below.
            byCoordinate.TryAdd((point.coord.row, point.coord.col), point);
        }

        Dictionary<MapPoint, string> nodeIds = StableMapNodeIds(run.CurrentActIndex, points);

        var nodes = new List<RuntimeMapV1Node>(points.Count);
        foreach (MapPoint point in points)
        {
            string nodeId = nodeIds[point];
            nodes.Add(new RuntimeMapV1Node(nodeId, point.coord.row, point.coord.col,
                MapCategory(map, point), IsVisited(run, point.coord)));
        }

        var edges = new List<RuntimeMapV1Edge>();
        var edgeKeys = new HashSet<(string From, string To)>();
        foreach (MapPoint point in points)
        {
            string from = nodeIds[point];
            foreach (MapPoint child in point.Children
                .OrderBy(candidate => candidate.coord.row)
                .ThenBy(candidate => candidate.coord.col)
                .ThenBy(candidate => (int)candidate.PointType))
            {
                if (!byCoordinate.ContainsKey((child.coord.row, child.coord.col)))
                    return UnavailableMap(generation, "map_edge_endpoint_missing", mapInstanceId,
                        run.CurrentActIndex);
                if (!nodeIds.TryGetValue(child, out string? to))
                {
                    return UnavailableMap(generation, "map_edge_endpoint_missing", mapInstanceId,
                        run.CurrentActIndex);
                }
                if (edgeKeys.Add((from, to)))
                    edges.Add(new RuntimeMapV1Edge(from, to));
            }
        }
        if (edges.Count > RuntimeMapV1Contract.MaxEdges)
            return UnavailableMap(generation, "map_edge_bound_exceeded", mapInstanceId,
                run.CurrentActIndex);

        var history = new List<string>();
        var historyIds = new HashSet<string>(StringComparer.Ordinal);
        foreach (MapCoord coordinate in run.VisitedMapCoords ?? Array.Empty<MapCoord>())
        {
            if (!byCoordinate.TryGetValue((coordinate.row, coordinate.col), out MapPoint? point))
                continue;
            string id = nodeIds[point];
            if (historyIds.Add(id)) history.Add(id);
        }

        RuntimeMapV1Position position = CurrentMapPosition(run, byCoordinate, nodeIds, history);
        var terminalIds = points.Where(point => point.Children.Count == 0)
            .Select(point => nodeIds[point])
            .Distinct(StringComparer.Ordinal)
            .OrderBy(id => id, StringComparer.Ordinal)
            .ToArray();

        NMapScreen? screen = NMapScreen.Instance;
        bool actionsEnabled = screen != null && MapActionsEnabled(screen);
        var bindings = new List<RuntimeMapV1ActionBinding>();
        bool unmappedAction = false;
        if (actionsEnabled)
        {
            // Bindings come from the same generation-bound legal catalog used by runtime-v3.
            // The map read never invents an action ID or rewrites the host's action payload.
            foreach (LegalActionReference action in LegalActions(observation)
                .Where(candidate => candidate.Kind == "select_map_node"
                    && candidate.Value is not null)
                .OrderBy(candidate => candidate.ActionId, StringComparer.Ordinal))
            {
                string actionNodeId = action.Value!;
                string? graphNodeId = nodeIds.ContainsValue(actionNodeId)
                    ? actionNodeId
                    : StableNodeIdForBaseId(nodeIds, actionNodeId);
                if (graphNodeId is null || !RuntimeMapV1Contract.IsHostActionId(action.ActionId))
                {
                    unmappedAction = true;
                    continue;
                }
                bindings.Add(new RuntimeMapV1ActionBinding(graphNodeId, action.ActionId,
                    actionNodeId));
            }
        }

        var snapshot = new RuntimeMapV1Snapshot(
            StateId: $"map:{run.CurrentActIndex}:{generation}",
            Generation: generation,
            SchemaVersion: RuntimeMapV1Contract.SnapshotSchemaVersion,
            ProjectionVersion: RuntimeMapV1Contract.ProjectionVersion,
            GameBuild: "unknown",
            ModVersion: RuntimeMapV1Contract.ModVersion,
            MapInstanceId: mapInstanceId,
            ActId: (uint)Math.Max(run.CurrentActIndex, 0),
            ScopeId: $"campaign:act:{run.CurrentActIndex}",
            Availability: "available",
            Completeness: unmappedAction ? "incomplete" : "complete",
            Freshness: "current",
            Reason: unmappedAction ? "map_legal_action_unmapped" : null,
            Nodes: nodes,
            Edges: edges,
            Position: position,
            History: history,
            TerminalNodeIds: terminalIds,
            Bindings: bindings);
        if (snapshot.Validate(out _)) return snapshot;

        // A malformed native graph must not be presented as a complete projection. The source
        // remains read-only and reports the boundary honestly.
        var incomplete = snapshot with
        {
            Completeness = "incomplete",
            Reason = "map_topology_not_complete",
            Bindings = Array.Empty<RuntimeMapV1ActionBinding>()
        };
        return incomplete.Validate(out _) ? incomplete
            : UnavailableMap(generation, "map_topology_invalid", mapInstanceId,
                run.CurrentActIndex);
    }

    private static RuntimeMapV1Position CurrentMapPosition(RunState run,
        Dictionary<(int Row, int Column), MapPoint> points,
        Dictionary<MapPoint, string> nodeIds, List<string> history)
    {
        if (run.CurrentMapCoord is not { } coordinate)
            return history.Count == 0
                ? new RuntimeMapV1Position("pre_start", null)
                : new RuntimeMapV1Position("unavailable", null);
        return points.TryGetValue((coordinate.row, coordinate.col), out MapPoint? point)
            && IsVisited(run, point.coord)
            ? new RuntimeMapV1Position("current", nodeIds[point])
            : new RuntimeMapV1Position("unavailable", null);
    }

    private static bool IsVisited(RunState run, MapCoord coordinate) =>
        run.VisitedMapCoords?.Any(candidate => candidate.Equals(coordinate)) == true;

    private static string MapCategory(ActMap map, MapPoint point)
    {
        if (ReferenceEquals(point, map.StartingMapPoint)) return "start";
        if (ReferenceEquals(point, map.BossMapPoint) || map.SecondBossMapPoint != null
            && ReferenceEquals(point, map.SecondBossMapPoint)) return "boss";
        return point.PointType switch
        {
            MapPointType.Monster => "monster",
            MapPointType.Elite => "elite",
            MapPointType.Shop => "shop",
            MapPointType.RestSite => "rest",
            MapPointType.Treasure => "treasure",
            MapPointType.Unknown => "unknown",
            MapPointType.Unassigned => "unknown",
            _ => "other"
        };
    }

}
