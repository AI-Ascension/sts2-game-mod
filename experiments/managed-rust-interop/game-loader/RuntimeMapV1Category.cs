// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Normalizes host map-point labels into the permitted public categories.</summary>
internal static class RuntimeMapV1Category
{
    internal static string Normalize(string? hostPointType, bool isStart, bool isTerminal)
    {
        if (isStart) return "start";
        if (isTerminal) return "boss";
        return hostPointType switch
        {
            "Monster" => "monster",
            "Elite" => "elite",
            "Shop" => "shop",
            "RestSite" => "rest",
            "Treasure" => "treasure",
            "Unknown" or "Unassigned" => "unknown",
            // STS2 v0.107.1 exposes Ancient but no Event enum. Keep a non-start Ancient
            // point deliberately unknown at this boundary instead of inventing an event label.
            "Ancient" => "other",
            _ => "other"
        };
    }
}
