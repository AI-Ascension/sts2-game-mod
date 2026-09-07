// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private static List<MapPoint> CollectMapPoints(ActMap map)
    {
        var points = new HashSet<MapPoint>(ReferenceEqualityComparer.Instance);
        foreach (MapPoint point in map.GetAllMapPoints()) points.Add(point);
        points.Add(map.StartingMapPoint);
        points.Add(map.BossMapPoint);
        if (map.SecondBossMapPoint != null) points.Add(map.SecondBossMapPoint);

        // GetAllMapPoints is the authoritative map enumeration. Children are included as a
        // defensive endpoint check so a malformed host graph cannot silently lose an edge.
        var pending = new Queue<MapPoint>(points);
        while (pending.Count > 0)
        {
            MapPoint point = pending.Dequeue();
            foreach (MapPoint child in point.Children)
                if (points.Add(child)) pending.Enqueue(child);
        }
        return points.ToList();
    }

    private static Dictionary<MapPoint, string> StableMapNodeIds(int act,
        IReadOnlyList<MapPoint> points)
    {
        var ids = new Dictionary<MapPoint, string>(ReferenceEqualityComparer.Instance);
        foreach (IGrouping<(int Row, int Column), MapPoint> group in points
            .GroupBy(point => (point.coord.row, point.coord.col)))
        {
            List<MapPoint> ordered = group.OrderBy(point => (int)point.PointType)
                .ThenBy(MapPointShape, StringComparer.Ordinal).ToList();
            string baseId = MapNodeId(act, new MapCoord(group.Key.Row, group.Key.Column));
            for (int index = 0; index < ordered.Count; index++)
            {
                string nodeId = ordered.Count == 1 ? baseId : $"{baseId}:overlap:{index}";
                ids.Add(ordered[index], nodeId);
            }
        }
        return ids;
    }

    private static string MapPointShape(MapPoint point) =>
        string.Join(';', point.Children.OrderBy(child => child.coord.row)
            .ThenBy(child => child.coord.col)
            .ThenBy(child => (int)child.PointType)
            .Select(child => $"{child.coord.row},{child.coord.col}:{(int)child.PointType}"));

    private static string? StableNodeIdForBaseId(Dictionary<MapPoint, string> nodeIds,
        string baseId)
    {
        string[] matches = nodeIds.Values.Where(id => id.StartsWith(baseId + ":overlap:",
            StringComparison.Ordinal)).OrderBy(id => id, StringComparer.Ordinal).ToArray();
        return matches.Length == 1 ? matches[0] : null;
    }

    private string EnsureMapInstanceId(RunState run, ActMap map)
    {
        if (!ReferenceEquals(run, _mapRunIdentity) || !ReferenceEquals(map, _mapObjectIdentity))
        {
            _mapRunIdentity = run;
            _mapObjectIdentity = map;
            _mapInstanceId = $"map-instance:{Guid.NewGuid():N}";
        }
        return _mapInstanceId!;
    }

    private static string MapNodeId(int act, MapCoord coordinate) =>
        $"map:{act}:{coordinate.row}:{coordinate.col}";

    private static string MapHostActionId(ulong generation, string nodeId) =>
        $"select_map_node:{generation}:{nodeId}";

    private static bool MapActionsEnabled(NMapScreen screen) =>
        screen.IsTravelEnabled && !screen.IsTraveling
        && NModalContainer.Instance?.OpenModal == null;

    private static RuntimeMapV1Snapshot UnavailableMap(ulong generation, string reason,
        string? mapInstanceId = null, int? act = null) => new(
        StateId: $"map-unavailable:{generation}",
        Generation: generation,
        SchemaVersion: RuntimeMapV1Contract.SnapshotSchemaVersion,
        ProjectionVersion: RuntimeMapV1Contract.ProjectionVersion,
        GameBuild: "unknown",
        ModVersion: RuntimeMapV1Contract.ModVersion,
        MapInstanceId: null,
        ActId: null,
        ScopeId: null,
        Availability: "unavailable",
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
