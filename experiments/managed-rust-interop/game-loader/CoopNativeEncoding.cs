// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
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
internal enum NativePlayerChoiceKind
{
    Index,
    Indexes,
    Player
}

internal sealed record NativePlayerChoiceEncoding(
    NativePlayerChoiceKind Kind,
    int? Index,
    IReadOnlyList<int>? Indexes,
    ulong? PlayerId)
{
    internal static NativePlayerChoiceEncoding SingleIndex(int? index) =>
        new(NativePlayerChoiceKind.Index, index, null, null);

    internal static NativePlayerChoiceEncoding MultipleIndexes(IReadOnlyList<int> indexes) =>
        new(NativePlayerChoiceKind.Indexes, null, indexes, null);

    internal static NativePlayerChoiceEncoding Player(ulong playerId) =>
        new(NativePlayerChoiceKind.Player, null, null, playerId);
}

internal sealed partial record NativeVoteEncoding(
    CoopVoteDomain Domain,
    int Index,
    bool SkipRelic,
    int MapRow,
    int MapColumn,
    int MapGenerationCount,
    NativePlayerChoiceEncoding? PlayerChoice,
    int? MapActIndex)
{
    internal static bool TryParse(CoopSharedVoteRequest request,
        out NativeVoteEncoding encoding, out string error)
    {
        encoding = new(CoopVoteDomain.PlayerChoice, 0, false, -1, -1, 0, null, null);
        error = string.Empty;
        switch (request.Domain)
        {
            case CoopVoteDomain.SharedEvent:
                if (!TryIndex(request.Choice, "index:", out int eventIndex))
                {
                    error = "native_event_choice_requires_index";
                    return false;
                }
                encoding = new(request.Domain, eventIndex, false, -1, -1, 0, null, null);
                return true;

            case CoopVoteDomain.TreasureRelic:
                if (request.Choice == "skip")
                {
                    encoding = new(request.Domain, 0, true, -1, -1, 0, null, null);
                    return true;
                }
                if (!TryIndex(request.Choice, "index:", out int relicIndex))
                {
                    error = "native_relic_choice_requires_index_or_skip";
                    return false;
                }
                encoding = new(request.Domain, relicIndex, false, -1, -1, 0, null, null);
                return true;

            case CoopVoteDomain.Map:
                if (!TryMapCoord(request.Choice, out int mapRow, out int mapColumn,
                        out int mapGenerationCount, out int? mapActIndex))
                {
                    error = "native_map_choice_requires_coord_row_col";
                    return false;
                }
                encoding = new(request.Domain, 0, false, mapRow, mapColumn,
                    mapGenerationCount, null, mapActIndex);
                return true;

            case CoopVoteDomain.PlayerChoice:
                if (!TryPlayerChoice(request.Choice,
                        out NativePlayerChoiceEncoding? playerChoice))
                {
                    error = "native_player_choice_encoding_invalid";
                    return false;
                }
                encoding = new(request.Domain, 0, false, -1, -1, 0, playerChoice, null);
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

    /// <summary>
    /// Parses the stable native map vote encoding. A short form uses the current map generation:
    /// <c>coord:&lt;row&gt;:&lt;col&gt;</c>. The explicit form carries the synchronizer generation so a
    /// stale vote cannot be applied after a map regeneration:
    /// <c>coord:&lt;generation&gt;:&lt;row&gt;:&lt;col&gt;</c>. Runtime map node identities are also
    /// accepted as <c>map:&lt;act&gt;:&lt;row&gt;:&lt;col&gt;</c>; that form uses the current synchronizer
    /// generation and is checked against the active run act by the host adapter.
    /// </summary>
    private static bool TryMapCoord(string value, out int row, out int column,
        out int mapGenerationCount, out int? mapActIndex)
    {
        row = -1;
        column = -1;
        mapActIndex = null;
        // -1 denotes the short form; the caller resolves it against the synchronizer's
        // current generation after it has fetched the live run state.
        mapGenerationCount = -1;
        string[] fields = value.Split(':', StringSplitOptions.None);
        bool explicitGeneration = fields.Length == 4 && fields[0] == "coord";
        bool currentGeneration = fields.Length == 3 && fields[0] == "coord";
        bool mapIdentity = fields.Length == 4 && fields[0] == "map";
        if (!explicitGeneration && !currentGeneration && !mapIdentity)
            return false;

        int generationField = explicitGeneration ? 1 : -1;
        int rowField = explicitGeneration || mapIdentity ? 2 : 1;
        int columnField = explicitGeneration || mapIdentity ? 3 : 2;
        if (explicitGeneration
            && !int.TryParse(fields[generationField], NumberStyles.None,
                CultureInfo.InvariantCulture, out mapGenerationCount))
            return false;
        int parsedAct = -1;
        if (mapIdentity && !int.TryParse(fields[1], NumberStyles.None,
                CultureInfo.InvariantCulture, out parsedAct))
            return false;
        if (mapIdentity)
            mapActIndex = parsedAct;
        if (!int.TryParse(fields[rowField], NumberStyles.None,
                CultureInfo.InvariantCulture, out row)
            || !int.TryParse(fields[columnField], NumberStyles.None,
                CultureInfo.InvariantCulture, out column)
            || mapGenerationCount < -1
            || mapActIndex is < 0 or > 4096
            || row < 0
            || column < 0
            || mapGenerationCount > 4096
            || row > 4096
            || column > 4096)
            return false;

        return true;
    }
}
