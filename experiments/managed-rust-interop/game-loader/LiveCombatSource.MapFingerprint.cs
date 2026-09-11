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
            if (!TryStableMapNodeIds(mapInstanceId, run.CurrentActIndex, points,
                    out Dictionary<MapPoint, string> nodeIds, out reason))
            {
                return $"map:open:{run.CurrentActIndex}:unavailable:{reason}";
            }
            if (!TryCollectVisitedCoordinates(run, out HashSet<(int Row, int Column)> visited,
                    out _, out reason))
            {
                return $"map:open:{run.CurrentActIndex}:unavailable:{reason}";
            }

            var builder = new StringBuilder("map:open:");
            builder.Append(run.CurrentActIndex).Append("|instance=").Append(mapInstanceId);
            if (!TryAppendMapModelFingerprint(builder, run.Map, points, nodeIds, out reason))
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
        List<MapPoint> points, Dictionary<MapPoint, string> nodeIds, out string reason)
    {
        // Stable graph IDs are the identity-bearing part of this fingerprint. Coordinates and
        // normalized labels remain public attributes, but they are never used as a tie-breaker:
        // two references may share a coordinate/category and rewiring their edges must still
        // produce a different generation. Child IDs are sorted by the canonical helper so a
        // HashSet enumeration order cannot perturb an unchanged graph.
        try
        {
            var graphNodes = new List<RuntimeMapV1FingerprintNode>(points.Count);
            int work = 0;
            int edgeCount = 0;
            foreach (MapPoint point in points)
            {
                if (++work > RuntimeMapV1Contract.MaxMapTraversalWork)
                {
                    reason = "map_graph_work_bound_exceeded";
                    return false;
                }
                if (!nodeIds.TryGetValue(point, out string? nodeId))
                {
                    reason = "map_graph_identity_invalid";
                    return false;
                }

                if (!RuntimeMapV1GraphFingerprint.TryCollectBoundedChildIds(
                        point.Children,
                        child => nodeIds.TryGetValue(child, out string? childId)
                            ? childId : null,
                        ref work, ref edgeCount, out List<string> childIds, out reason))
                    return false;

                graphNodes.Add(new RuntimeMapV1FingerprintNode(
                    nodeId, point.coord.row, point.coord.col, MapCategory(map, point),
                    MapTerminalKind(map, point), childIds));
            }

            if (!RuntimeMapV1GraphFingerprint.TryCreate(graphNodes, out string graphFingerprint,
                    out reason)) return false;
            builder.Append("|graph=").Append(graphFingerprint);
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
