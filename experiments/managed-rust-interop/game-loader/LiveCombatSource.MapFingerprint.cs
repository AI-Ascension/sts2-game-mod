// SPDX-License-Identifier: MIT

using System;
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
    private static string MapSurfaceFingerprint()
    {
        try
        {
            if (!LiveCombatDemo.Campaign || !RunManager.Instance.IsInProgress)
                return "map:unavailable";

            RunState? run = RunManager.Instance.DebugOnlyGetState();
            NMapScreen? screen = NMapScreen.Instance;
            if (run?.Map == null || screen?.IsOpen != true || !screen.IsVisibleInTree())
                return "map:closed";

            var builder = new StringBuilder("map:open:");
            builder.Append(run.CurrentActIndex).Append('|');
            AppendMapModelFingerprint(builder, run.Map);
            builder.Append("|current=");
            if (run.CurrentMapCoord is { } current)
                builder.Append(current.row).Append(',').Append(current.col);
            else
                builder.Append("none");
            builder.Append("|visited=");
            foreach (MapCoord coordinate in run.VisitedMapCoords ?? Array.Empty<MapCoord>())
                builder.Append(coordinate.row).Append(',').Append(coordinate.col).Append(';');
            builder.Append("|enabled=").Append(screen.IsTravelEnabled)
                .Append('|').Append(screen.IsTraveling)
                .Append('|').Append(MapActionsEnabled(screen));
            builder.Append("|travelable=");
            foreach (NMapPoint point in TravelablePoints()
                .OrderBy(point => point.Point.coord.row)
                .ThenBy(point => point.Point.coord.col))
            {
                builder.Append(point.Point.coord.row).Append(',').Append(point.Point.coord.col)
                    .Append(';');
            }
            return builder.ToString();
        }
        catch
        {
            return "map:error";
        }
    }

    private static void AppendMapModelFingerprint(StringBuilder builder, ActMap map)
    {
        var points = CollectMapPoints(map);
        foreach (MapPoint point in points.OrderBy(point => point.coord.row)
            .ThenBy(point => point.coord.col)
            .ThenBy(point => (int)point.PointType))
        {
            builder.Append(point.coord.row).Append(',').Append(point.coord.col)
                .Append(':').Append((int)point.PointType).Append('>');
            foreach (MapPoint child in point.Children.OrderBy(child => child.coord.row)
                .ThenBy(child => child.coord.col)
                .ThenBy(child => (int)child.PointType))
            {
                builder.Append(child.coord.row).Append(',').Append(child.coord.col).Append(',');
            }
            builder.Append(';');
        }
    }
}
