// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    /// <summary>
    /// Supplies the map portion of the shared gameplay fingerprint. It only includes state that
    /// is already public in the open map surface and never causes UI mutation or scrolling.
    /// </summary>
    private string MapSurfaceFingerprint()
    {
        try
        {
            if (!LiveCombatDemo.Campaign || !RunManager.Instance.IsInProgress)
                return "map:unavailable";

            RunState? run = RunManager.Instance.DebugOnlyGetState();
            NMapScreen? screen = NMapScreen.Instance;
            if (run?.Map == null || screen?.IsOpen != true || !screen.IsVisibleInTree())
                return "map:closed";

            string mapInstanceId = EnsureMapInstanceId(run, run.Map);
            if (!TryCollectMapPoints(run.Map, out List<MapPoint> points, out string reason))
                return $"map:open:{run.CurrentActIndex}:unavailable:{reason}";
            if (!TryCollectVisitedCoordinates(run, out HashSet<(int Row, int Column)> visited,
                    out _, out reason))
            {
                return $"map:open:{run.CurrentActIndex}:unavailable:{reason}";
            }

            var builder = new StringBuilder("map:open:");
            builder.Append(run.CurrentActIndex).Append("|instance=").Append(mapInstanceId);
            if (!TryAppendMapModelFingerprint(builder, run.Map, points, out reason))
                return $"map:open:{run.CurrentActIndex}:unavailable:{reason}";
            builder.Append("|current=");
            if (run.CurrentMapCoord is { } current)
                builder.Append(current.row).Append(',').Append(current.col);
            else
                builder.Append("none");
            builder.Append("|visited=");
            foreach ((int row, int column) in visited.OrderBy(value => value.Row)
                .ThenBy(value => value.Column))
            {
                builder.Append(row).Append(',').Append(column).Append(';');
            }
            builder.Append("|enabled=").Append(screen.IsTravelEnabled)
                .Append('|').Append(screen.IsTraveling)
                .Append('|').Append(MapActionsEnabled(screen));
            return builder.ToString();
        }
        catch
        {
            return "map:unavailable:map_fingerprint_failed";
        }
    }

    private static bool TryAppendMapModelFingerprint(StringBuilder builder, ActMap map,
        List<MapPoint> points, out string reason)
    {
        // This is the normalized public projection. Raw PointType/child-type enum integers are
        // deliberately absent so host states that collapse to the same visible category hash
        // identically.
        int work = 0;
        int edges = 0;
        try
        {
            foreach (MapPoint point in points.OrderBy(point => point.coord.row)
                .ThenBy(point => point.coord.col)
                .ThenBy(point => MapCategory(map, point), StringComparer.Ordinal)
                .ThenBy(point => MapTerminalKind(map, point), StringComparer.Ordinal))
            {
                if (++work > RuntimeMapV1Contract.MaxMapTraversalWork)
                {
                    reason = "map_traversal_work_bound_exceeded";
                    return false;
                }
                var children = new List<MapPoint>(Math.Min(RuntimeMapV1Contract.MaxEdges,
                    point.Children.Count));
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
                    children.Add(child);
                }

                builder.Append(point.coord.row).Append(',').Append(point.coord.col)
                    .Append(':').Append(MapCategory(map, point))
                    .Append(':').Append(MapTerminalKind(map, point)).Append('>');
                foreach (MapPoint child in children.OrderBy(child => child.coord.row)
                    .ThenBy(child => child.coord.col)
                    .ThenBy(child => MapCategory(map, child), StringComparer.Ordinal)
                    .ThenBy(child => MapTerminalKind(map, child), StringComparer.Ordinal))
                {
                    builder.Append(child.coord.row).Append(',').Append(child.coord.col)
                        .Append(':').Append(MapCategory(map, child))
                        .Append(':').Append(MapTerminalKind(map, child)).Append(',');
                }
                builder.Append(';');
            }
        }
        catch
        {
            reason = "map_model_projection_failed";
            return false;
        }
        reason = string.Empty;
        return true;
    }

    private static string MapTerminalKind(ActMap map, MapPoint point) =>
        ReferenceEquals(point, map.BossMapPoint) ? "boss"
        : map.SecondBossMapPoint != null && ReferenceEquals(point, map.SecondBossMapPoint)
            ? "second_boss" : "none";
}
