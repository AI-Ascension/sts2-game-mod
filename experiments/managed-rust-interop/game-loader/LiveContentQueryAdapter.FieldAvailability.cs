// SPDX-License-Identifier: MIT

using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static Dictionary<string, object?>? LiveField(
        string name,
        LiveCardCapturedCard card,
        string instanceId,
        out (int Status, string Code, string Reason)? refusal)
    {
        refusal = null;
        LiveCardFieldStatus status;
        object? value;
        switch (name)
        {
            case "display_name": status = card.Title.Status; value = card.Title.Value; break;
            case "cost": status = card.ResolvedCost.Status; value = card.ResolvedCost.Value; break;
            case "owner": status = card.OwnerId.Status; value = card.OwnerId.Value; break;
            default: status = LiveCardFieldStatus.NotObserved; value = null; break;
        }
        string kind = name switch { "cost" => "integer", "tags" => "text_list", _ => "text" };
        string? unit = name == "cost" ? "count" : null;
        string availability;
        string? reason;
        switch (status)
        {
            case LiveCardFieldStatus.Available:
                if (value is null || kind == "text" && value is string text && text.Length == 0)
                {
                    refusal = (503, "missing_capability", "live_field_value_invalid");
                    return null;
                }
                availability = "available";
                reason = null;
                break;
            case LiveCardFieldStatus.NotObserved:
                availability = "not_observable";
                reason = "field_not_observed";
                value = null;
                break;
            case LiveCardFieldStatus.Unsupported:
                availability = "unsupported";
                reason = "field_unsupported";
                value = null;
                break;
            case LiveCardFieldStatus.Unknown:
                availability = "unavailable";
                reason = "field_value_unknown";
                value = null;
                break;
            case LiveCardFieldStatus.Failed:
                availability = "unavailable";
                reason = "field_read_failed";
                value = null;
                break;
            case LiveCardFieldStatus.Stale:
                refusal = (409, "stale_snapshot", "live_field_stale");
                return null;
            default:
                availability = "unavailable";
                reason = "field_status_unrecognized";
                value = null;
                break;
        }
        return new Dictionary<string, object?>
        {
            ["name"] = name, ["kind"] = kind, ["value"] = value, ["unit"] = unit,
            ["availability"] = availability, ["reason"] = reason,
            ["source"] = new Dictionary<string, string> { ["kind"] = "game_mod", ["ref"] = instanceId }
        };
    }
}
