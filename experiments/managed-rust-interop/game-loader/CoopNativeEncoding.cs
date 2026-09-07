// SPDX-License-Identifier: MIT

using System;
using System.Globalization;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Canonical action arguments understood by the first-party action queue adapter. The wire's
/// action_id is intentionally kept as an identity; only the explicit turn encoding is accepted
/// here, so a model cannot make an arbitrary string mean a different native action.
/// </summary>
internal static class NativeActionEncoding
{
    internal static bool TryTurnNumber(string? value, int currentTurn, out int turnNumber,
        out string error)
    {
        turnNumber = 0;
        error = string.Empty;
        if (currentTurn < 0)
        {
            error = "native_player_turn_unavailable";
            return false;
        }

        if (string.IsNullOrEmpty(value) || value is "end_turn" or "turn")
        {
            turnNumber = currentTurn;
            return true;
        }

        const string prefix = "turn:";
        if (!value.StartsWith(prefix, StringComparison.Ordinal)
            || !int.TryParse(value[prefix.Length..], NumberStyles.None,
                CultureInfo.InvariantCulture, out turnNumber)
            || turnNumber < 0
            || turnNumber != currentTurn)
        {
            error = "native_end_turn_argument_invalid";
            return false;
        }
        return true;
    }
}

/// <summary>
/// Explicit native combat-card identity carried by a local-action request. The ID is allocated
/// by the installed first-party <c>NetCombatCardDb</c>; it is not a hand position or a model ID.
/// An optional creature combat ID identifies an enemy target. Both values are checked again
/// against the live hand/combat state before a native action is queued.
/// </summary>
internal readonly record struct NativeCardEncoding(uint CardId, uint? TargetId)
{
    internal static bool TryParse(string? value, out NativeCardEncoding encoding,
        out string error)
    {
        encoding = default;
        error = string.Empty;
        if (string.IsNullOrEmpty(value))
        {
            error = "native_card_action_requires_card_id";
            return false;
        }

        string[] fields = value.Split(':', StringSplitOptions.None);
        if (fields.Length is not (2 or 4)
            || fields[0] != "card"
            || !uint.TryParse(fields[1], NumberStyles.None,
                CultureInfo.InvariantCulture, out uint cardId)
            || cardId == 0)
        {
            error = "native_card_action_card_id_invalid";
            return false;
        }

        uint? targetId = null;
        uint parsedTarget = 0;
        if (fields.Length == 4
            && (fields[2] != "target"
                || !uint.TryParse(fields[3], NumberStyles.None,
                    CultureInfo.InvariantCulture, out parsedTarget)
                || parsedTarget == 0))
        {
            error = "native_card_action_target_id_invalid";
            return false;
        }
        if (fields.Length == 4)
            targetId = parsedTarget;

        encoding = new NativeCardEncoding(cardId, targetId);
        return true;
    }
}

/// <summary>
/// Typed vote arguments carried by the additive native profile. Generic approve/reject strings
/// are rejected at this boundary because they do not identify a native event or relic choice.
/// </summary>
internal sealed record NativeVoteEncoding(
    CoopVoteDomain Domain,
    int Index,
    bool SkipRelic)
{
    internal static bool TryParse(CoopSharedVoteRequest request,
        out NativeVoteEncoding encoding, out string error)
    {
        encoding = new(CoopVoteDomain.PlayerChoice, 0, false);
        error = string.Empty;
        switch (request.Domain)
        {
            case CoopVoteDomain.SharedEvent:
                if (!TryIndex(request.Choice, "index:", out int eventIndex))
                {
                    error = "native_event_choice_requires_index";
                    return false;
                }
                encoding = new(request.Domain, eventIndex, false);
                return true;

            case CoopVoteDomain.TreasureRelic:
                if (request.Choice == "skip")
                {
                    encoding = new(request.Domain, 0, true);
                    return true;
                }
                if (!TryIndex(request.Choice, "index:", out int relicIndex))
                {
                    error = "native_relic_choice_requires_index_or_skip";
                    return false;
                }
                encoding = new(request.Domain, relicIndex, false);
                return true;

            default:
                error = "native_vote_domain_not_supported";
                return false;
        }
    }

    private static bool TryIndex(string value, string prefix, out int index)
    {
        index = -1;
        return value.StartsWith(prefix, StringComparison.Ordinal)
            && int.TryParse(value[prefix.Length..], NumberStyles.None,
                CultureInfo.InvariantCulture, out index)
            && index >= 0
            && index <= 4096;
    }
}
