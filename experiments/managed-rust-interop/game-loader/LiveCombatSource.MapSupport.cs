// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    // These tokens are intentionally opaque. They are assigned once for a host MapPoint
    // reference and are never derived from PointType, child shape, or coordinate overlap
    // ordering. A map lifetime reset clears the registry in EnsureMapInstanceId.
    private readonly Dictionary<MapPoint, string> _mapPointDisambiguators =
        new(ReferenceEqualityComparer.Instance);
    private uint _nextMapPointDisambiguator;

    private static bool TryCollectMapPoints(ActMap map, out List<MapPoint> points,
        out string reason)
    {
        // Fixed capacities keep the first allocations bounded. Every item is checked before
        // it can grow either collection, including items returned by the initial enumeration.
        points = new List<MapPoint>(RuntimeMapV1Contract.MaxNodes);
        var seen = new HashSet<MapPoint>(RuntimeMapV1Contract.MaxNodes,
            ReferenceEqualityComparer.Instance);
        var pending = new Queue<MapPoint>(RuntimeMapV1Contract.MaxNodes);
        int work = 0;
        int edges = 0;

        try
        {
            foreach (MapPoint point in map.GetAllMapPoints())
                if (!TryAddMapPoint(point, points, seen, pending, ref work, out reason))
                    return false;

            // The declared special points are part of the public map projection even if a
            // host build omits one from GetAllMapPoints().
            if (!TryAddMapPoint(map.StartingMapPoint, points, seen, pending, ref work,
                    out reason)
                || !TryAddMapPoint(map.BossMapPoint, points, seen, pending, ref work,
                    out reason)
                || map.SecondBossMapPoint != null
                    && !TryAddMapPoint(map.SecondBossMapPoint, points, seen, pending, ref work,
                        out reason))
                return false;

            while (pending.Count > 0)
            {
                MapPoint point = pending.Dequeue();
                foreach (MapPoint child in point.Children)
                {
                    if (++work > RuntimeMapV1Contract.MaxMapTraversalWork)
                    {
                        reason = "map_traversal_work_bound_exceeded";
                        return false;
                    }
                    if (++edges > RuntimeMapV1Contract.MaxEdges)
                    {
                        reason = "map_edge_bound_exceeded";
                        return false;
                    }
                    if (!seen.Contains(child))
                    {
                        if (points.Count >= RuntimeMapV1Contract.MaxNodes)
                        {
                            reason = "map_node_bound_exceeded";
                            return false;
                        }
                        seen.Add(child);
                        points.Add(child);
                        pending.Enqueue(child);
                    }
                }
            }
        }
        catch
        {
            reason = "map_traversal_failed";
            points.Clear();
            return false;
        }

        reason = string.Empty;
        return true;
    }

    private static bool TryAddMapPoint(MapPoint point, List<MapPoint> points,
        HashSet<MapPoint> seen, Queue<MapPoint> pending, ref int work, out string reason)
    {
        if (++work > RuntimeMapV1Contract.MaxMapTraversalWork)
        {
            reason = "map_traversal_work_bound_exceeded";
            return false;
        }
        if (seen.Contains(point))
        {
            reason = string.Empty;
            return true;
        }
        if (points.Count >= RuntimeMapV1Contract.MaxNodes)
        {
            reason = "map_node_bound_exceeded";
            return false;
        }
        seen.Add(point);
        points.Add(point);
        pending.Enqueue(point);
        reason = string.Empty;
        return true;
    }

    private string EnsureMapInstanceId(RunState run, ActMap map)
    {
        if (!ReferenceEquals(run, _mapRunIdentity) || !ReferenceEquals(map, _mapObjectIdentity))
        {
            _mapRunIdentity = run;
            _mapObjectIdentity = map;
            _mapInstanceId = $"map-instance:{Guid.NewGuid():N}";
            _mapPointDisambiguators.Clear();
            _nextMapPointDisambiguator = 0;
        }
        return _mapInstanceId!;
    }

    private string StableMapNodeId(string mapInstanceId, int act, MapPoint point)
    {
        if (!_mapPointDisambiguators.TryGetValue(point, out string? token))
        {
            if (_nextMapPointDisambiguator == uint.MaxValue)
                throw new InvalidOperationException("map point disambiguator bound exceeded");
            token = $"p{++_nextMapPointDisambiguator}";
            _mapPointDisambiguators.Add(point, token);
        }
        return $"map-node:{mapInstanceId}:act:{act}:ref:{token}";
    }

    private Dictionary<MapPoint, string> StableMapNodeIds(string mapInstanceId, int act,
        List<MapPoint> points)
    {
        var ids = new Dictionary<MapPoint, string>(points.Count, ReferenceEqualityComparer.Instance);
        foreach (MapPoint point in points)
            ids.Add(point, StableMapNodeId(mapInstanceId, act, point));
        return ids;
    }

    private static string MapNodeId(int act, MapCoord coordinate) =>
        // This is the legacy action payload used by runtime-v3 campaign actions. It must stay
        // coordinate-based; map graph IDs are the map-lifetime opaque IDs above.
        $"map:{act}:{coordinate.row}:{coordinate.col}";

    private static string MapHostActionId(ulong generation, string nodeId) =>
        $"select_map_node:{generation}:{nodeId}";

    private static bool MapActionsEnabled(NMapScreen screen) =>
        screen.IsTravelEnabled && !screen.IsTraveling
        && NModalContainer.Instance?.OpenModal == null;

    private static string CurrentGameBuild()
    {
        try
        {
            string? version = MegaCrit.Sts2.Core.Debug.ReleaseInfoManager.Instance
                ?.ReleaseInfo?.Version;
            if (RuntimeMapV1Contract.IsText(version)) return version!;
        }
        catch
        {
            // The projection remains valid with an explicit unknown build if the host's
            // release-info resource is unavailable during early startup.
        }
        return "unknown";
    }

    private static RuntimeMapV1Snapshot UnavailableMap(ulong generation, string reason,
        string? mapInstanceId = null, int? act = null, string availability = "unavailable")
    {
        uint? actId = act is { } value ? (uint)Math.Max(value, 0) : null;
        return new(
            StateId: $"map-unavailable:{generation}",
            Generation: generation,
            SchemaVersion: RuntimeMapV1Contract.SnapshotSchemaVersion,
            ProjectionVersion: RuntimeMapV1Contract.ProjectionVersion,
            GameBuild: CurrentGameBuild(),
            ModVersion: RuntimeMapV1Contract.ModVersion,
            MapInstanceId: mapInstanceId,
            ActId: actId,
            ScopeId: act is { } knownAct ? $"campaign:act:{knownAct}" : null,
            Availability: availability,
            Completeness: "unknown",
            Freshness: "current",
            Reason: reason,
            Nodes: Array.Empty<RuntimeMapV1Node>(),
            Edges: Array.Empty<RuntimeMapV1Edge>(),
            Position: new RuntimeMapV1Position("unavailable", null),
            History: Array.Empty<string>(),
            TerminalNodeIds: Array.Empty<string>(),
            Bindings: Array.Empty<RuntimeMapV1ActionBinding>());
    }
}
