// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private static bool TryCollectVisitedCoordinates(RunState run,
        out HashSet<(int Row, int Column)> visited, out bool hasAny, out string reason)
    {
        visited = new HashSet<(int Row, int Column)>();
        hasAny = false;
        int work = 0;
        try
        {
            foreach (MapCoord coordinate in run.VisitedMapCoords ?? Array.Empty<MapCoord>())
            {
                if (++work > RuntimeMapV1Contract.MaxHistory)
                {
                    reason = "map_history_bound_exceeded";
                    return false;
                }
                hasAny = true;
                visited.Add((coordinate.row, coordinate.col));
            }
        }
        catch
        {
            reason = "map_history_unavailable";
            return false;
        }
        reason = string.Empty;
        return true;
    }

    private static void BuildCoordinateIndex(List<MapPoint> points,
        out Dictionary<(int Row, int Column), MapPoint> byCoordinate,
        out HashSet<(int Row, int Column)> ambiguousCoordinates)
    {
        byCoordinate = new Dictionary<(int Row, int Column), MapPoint>(points.Count);
        ambiguousCoordinates = new HashSet<(int Row, int Column)>();
        foreach (MapPoint point in points)
        {
            (int Row, int Column) key = (point.coord.row, point.coord.col);
            if (ambiguousCoordinates.Contains(key)) continue;
            if (byCoordinate.Remove(key))
            {
                ambiguousCoordinates.Add(key);
                continue;
            }
            byCoordinate.Add(key, point);
        }
    }

    private static bool TryUniqueLegacyActionPoint(string actionNodeId, int act,
        List<MapPoint> points, HashSet<(int Row, int Column)> ambiguousCoordinates,
        out MapPoint? point)
    {
        point = null;
        foreach (MapPoint candidate in points)
        {
            if (MapNodeId(act, candidate.coord) != actionNodeId
                || ambiguousCoordinates.Contains((candidate.coord.row, candidate.coord.col)))
                continue;
            if (point != null) return false;
            point = candidate;
        }
        return point != null;
    }

    private static RuntimeMapV1Position CurrentMapPosition(RunState run,
        Dictionary<(int Row, int Column), MapPoint> points,
        HashSet<(int Row, int Column)> ambiguousCoordinates,
        Dictionary<MapPoint, string> nodeIds, List<string> history,
        bool hasVisitedCoordinates, out bool ambiguous)
    {
        ambiguous = false;
        if (run.CurrentMapCoord is not { } coordinate)
            return hasVisitedCoordinates || history.Count != 0
                ? new RuntimeMapV1Position("unavailable", null)
                : new RuntimeMapV1Position("pre_start", null);

        (int Row, int Column) key = (coordinate.row, coordinate.col);
        if (ambiguousCoordinates.Contains(key))
        {
            ambiguous = true;
            return new RuntimeMapV1Position("unavailable", null);
        }
        return points.TryGetValue(key, out MapPoint? point)
            && history.Contains(nodeIds[point], StringComparer.Ordinal)
            ? new RuntimeMapV1Position("current", nodeIds[point])
            : new RuntimeMapV1Position("unavailable", null);
    }

    private static string MapCategory(ActMap map, MapPoint point)
    {
        bool isStart = ReferenceEquals(point, map.StartingMapPoint);
        bool isTerminal = ReferenceEquals(point, map.BossMapPoint)
            || map.SecondBossMapPoint != null && ReferenceEquals(point, map.SecondBossMapPoint);
        // The host's v0.107.1 enum has Ancient but no Event member. Runtime-map-v1 therefore
        // deliberately normalizes a non-start Ancient point to "other"; only the declared
        // starting point receives the public "start" category.
        return RuntimeMapV1Category.Normalize(point.PointType.ToString(), isStart, isTerminal);
    }
}
