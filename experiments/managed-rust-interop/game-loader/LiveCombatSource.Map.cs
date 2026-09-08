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
    public RuntimeMapV1Snapshot ObserveMap()
    {
        RequireThread();

        // The gameplay observation owns the shared generation fence. It also makes a map
        // topology or navigation-catalog change visible to the existing runtime-v3 fence.
        RuntimeV3GameplayObservation observation = Observe();
        string? knownMapInstanceId = null;
        int? knownAct = null;
        bool mapNotObservable = false;
        try
        {
            if (!LiveCombatDemo.Campaign || !RunManager.Instance.IsInProgress)
                return UnavailableMap(observation.Generation, "campaign_not_active");

            RunState? run = RunManager.Instance.DebugOnlyGetState();
            if (run?.Map == null)
                return UnavailableMap(observation.Generation, "map_state_unavailable");

            string mapInstanceId = EnsureMapInstanceId(run, run.Map);
            knownMapInstanceId = mapInstanceId;
            knownAct = run.CurrentActIndex;
            NMapScreen? mapScreen = NMapScreen.Instance;
            if (mapScreen?.IsOpen != true || !mapScreen.IsVisibleInTree())
            {
                mapNotObservable = true;
                return UnavailableMap(observation.Generation, "map_not_open", mapInstanceId,
                    run.CurrentActIndex, "not_observable");
            }

            RuntimeMapV1Snapshot snapshot = BuildMapSnapshot(run, mapInstanceId, observation);

            // BuildMapSnapshot rereads the host map and legal catalog after Observe() captured
            // the gameplay generation. Reobserve after that copy so a host animation, callback,
            // visibility change, or catalog mutation cannot pair two different surfaces under
            // one generation. The latest generation is returned with an explicit unavailable
            // result, allowing the caller to retry through the normal stale-read boundary.
            RuntimeV3GameplayObservation finalObservation = Observe();
            if (!RuntimeMapV1ObservationFence.IsStable(observation.Generation,
                    finalObservation.Generation))
            {
                return RuntimeMapV1ObservationFence.RejectChangedSurface(snapshot,
                    finalObservation.Generation);
            }
            return snapshot;
        }
        catch
        {
            return UnavailableMap(observation.Generation, "map_projection_failed",
                knownMapInstanceId, knownAct, mapNotObservable ? "not_observable" : "unavailable");
        }
    }

    private RuntimeMapV1Snapshot BuildMapSnapshot(RunState run, string mapInstanceId,
        RuntimeV3GameplayObservation observation)
    {
        ulong generation = observation.Generation;
        ActMap map = run.Map;
        if (!TryCollectMapPoints(map, out List<MapPoint> points, out string traversalReason))
            return UnavailableMap(generation, traversalReason, mapInstanceId,
                run.CurrentActIndex);

        if (!TryStableMapNodeIds(mapInstanceId, run.CurrentActIndex, points,
                out Dictionary<MapPoint, string> nodeIds, out string identityReason))
        {
            return UnavailableMap(generation, identityReason, mapInstanceId,
                run.CurrentActIndex);
        }
        points = points.OrderBy(point => nodeIds[point], StringComparer.Ordinal).ToList();

        if (!TryCollectVisitedCoordinates(run, out HashSet<(int Row, int Column)> visited,
                out bool hasVisitedCoordinates, out string visitedReason))
        {
            return UnavailableMap(generation, visitedReason, mapInstanceId,
                run.CurrentActIndex);
        }

        BuildCoordinateIndex(points, out Dictionary<(int Row, int Column), MapPoint> byCoordinate,
            out HashSet<(int Row, int Column)> ambiguousCoordinates);

        var nodes = new List<RuntimeMapV1Node>(points.Count);
        foreach (MapPoint point in points)
        {
            string nodeId = nodeIds[point];
            nodes.Add(new RuntimeMapV1Node(nodeId, point.coord.row, point.coord.col,
                MapCategory(map, point), visited.Contains((point.coord.row, point.coord.col))));
        }

        var edges = new List<RuntimeMapV1Edge>(Math.Min(RuntimeMapV1Contract.MaxEdges,
            points.Count));
        var edgeKeys = new HashSet<(string From, string To)>();
        int edgeWork = 0;
        foreach (MapPoint point in points)
        {
            string from = nodeIds[point];
            foreach (MapPoint child in point.Children)
            {
                if (++edgeWork > RuntimeMapV1Contract.MaxEdges)
                    return UnavailableMap(generation, "map_edge_bound_exceeded", mapInstanceId,
                        run.CurrentActIndex);
                if (!nodeIds.TryGetValue(child, out string? to))
                    return UnavailableMap(generation, "map_edge_endpoint_missing", mapInstanceId,
                        run.CurrentActIndex);
                if (edgeKeys.Add((from, to)))
                {
                    edges.Add(new RuntimeMapV1Edge(from, to));
                }
            }
        }
        edges = edges.OrderBy(edge => edge.From, StringComparer.Ordinal)
            .ThenBy(edge => edge.To, StringComparer.Ordinal).ToList();

        var history = new List<string>();
        var historyIds = new HashSet<string>(StringComparer.Ordinal);
        bool ambiguousHistory = false;
        foreach ((int row, int column) in visited.OrderBy(value => value.Row)
            .ThenBy(value => value.Column))
        {
            (int Row, int Column) key = (row, column);
            if (ambiguousCoordinates.Contains(key))
            {
                // A coordinate alone cannot identify one of several host MapPoint references.
                // Keep the history unknown instead of selecting the first overlap.
                ambiguousHistory = true;
                continue;
            }
            if (!byCoordinate.TryGetValue(key, out MapPoint? point)) continue;
            string id = nodeIds[point];
            if (historyIds.Add(id)) history.Add(id);
        }

        RuntimeMapV1Position position = CurrentMapPosition(run, byCoordinate,
            ambiguousCoordinates, nodeIds, history, hasVisitedCoordinates,
            out bool ambiguousCurrent);
        bool ambiguousIdentity = ambiguousCoordinates.Count != 0
            || ambiguousHistory || ambiguousCurrent;

        var terminalIds = new List<string>(2);
        AddDeclaredTerminal(map.BossMapPoint);
        if (map.SecondBossMapPoint != null) AddDeclaredTerminal(map.SecondBossMapPoint);
        terminalIds.Sort(StringComparer.Ordinal);

        NMapScreen? screen = NMapScreen.Instance;
        bool actionsEnabled = screen != null && MapActionsEnabled(screen);
        var bindings = new List<RuntimeMapV1ActionBinding>();
        var boundGraphNodes = new HashSet<string>(StringComparer.Ordinal);
        var boundHostActions = new HashSet<string>(StringComparer.Ordinal);
        var boundActionOptions = new HashSet<string>(StringComparer.Ordinal);
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
                MapPoint? actionPoint = null;
                bool validHostAction = RuntimeMapV1Contract.IsHostActionId(action.ActionId)
                    && TryUniqueLegacyActionPoint(actionNodeId, run.CurrentActIndex, points,
                        ambiguousCoordinates, out actionPoint);
                if (!validHostAction || actionPoint == null)
                {
                    unmappedAction = true;
                    continue;
                }

                string graphNodeId = nodeIds[actionPoint];
                if (!boundGraphNodes.Add(graphNodeId)
                    || !boundHostActions.Add(action.ActionId)
                    || !boundActionOptions.Add(actionNodeId))
                {
                    // Duplicate coordinate actions have no authoritative way to select one
                    // reference. Treat the option as unknown rather than choosing an order.
                    unmappedAction = true;
                    continue;
                }
                bindings.Add(new RuntimeMapV1ActionBinding(graphNodeId, action.ActionId,
                    actionNodeId));
            }
        }

        string? reason = ambiguousIdentity ? "map_coordinate_identity_ambiguous"
            : unmappedAction ? "map_legal_action_unmapped" : null;
        var snapshot = new RuntimeMapV1Snapshot(
            StateId: $"map:{run.CurrentActIndex}:{generation}",
            Generation: generation,
            SchemaVersion: RuntimeMapV1Contract.SnapshotSchemaVersion,
            ProjectionVersion: RuntimeMapV1Contract.ProjectionVersion,
            GameBuild: CurrentGameBuild(),
            ModVersion: RuntimeMapV1Contract.ModVersion,
            MapInstanceId: mapInstanceId,
            ActId: (uint)Math.Max(run.CurrentActIndex, 0),
            ScopeId: $"campaign:act:{run.CurrentActIndex}",
            Availability: "available",
            Completeness: reason == null ? "complete" : "incomplete",
            Freshness: "current",
            Reason: reason,
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

        void AddDeclaredTerminal(MapPoint point)
        {
            if (nodeIds.TryGetValue(point, out string? terminalId))
                terminalIds.Add(terminalId);
        }
    }

}
