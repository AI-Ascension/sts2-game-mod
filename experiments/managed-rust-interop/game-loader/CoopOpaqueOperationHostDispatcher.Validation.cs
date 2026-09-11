// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class CoopOpaqueOperationHostDispatcher
{
    private static bool TryParse(byte[] payload, out CarrierRequest? request, out string error)
    {
        request = null;
        error = "invalid_request";
        try
        {
            using JsonDocument document = JsonDocument.Parse(payload,
                new JsonDocumentOptions { MaxDepth = 8, AllowTrailingCommas = false });
            JsonElement root = document.RootElement;
            if (!HasOnly(root, "kind", "expected_host_generation", "action", "vote", "rejoin_epoch")
                || !TryString(root, "kind", out string kind)
                || !TryUInt64(root, "expected_host_generation", out ulong generation)
                || generation > RuntimeV3GameplayContract.MaxGeneration) return false;
            if (kind == "local_action" && TryAction(root, out string actionKind, out string actionId))
            {
                request = new(CarrierKind.LocalAction, generation, actionKind, actionId, null, null);
                return true;
            }
            if (kind == "shared_vote" && TryVote(root, out string proposalId, out string choice))
            {
                request = new(CarrierKind.SharedVote, generation, null, null, proposalId, choice);
                return true;
            }
        }
        catch (JsonException) { error = "invalid_request_json"; }
        return false;
    }

    private static bool TryAction(JsonElement root, out string kind, out string actionId)
    {
        kind = string.Empty;
        actionId = string.Empty;
        return IsNull(root, "vote") && IsNull(root, "rejoin_epoch")
            && root.TryGetProperty("action", out JsonElement action)
            && HasOnly(action, "kind", "action_id") && TryString(action, "kind", out kind)
            && TryString(action, "action_id", out actionId) && (kind is "play_card" or "end_turn")
            && RuntimeV3GameplayContract.IsIdentity(actionId);
    }

    private static bool TryVote(JsonElement root, out string proposalId, out string choice)
    {
        proposalId = string.Empty;
        choice = string.Empty;
        return IsNull(root, "action") && IsNull(root, "rejoin_epoch")
            && root.TryGetProperty("vote", out JsonElement vote)
            && HasOnly(vote, "proposal_id", "choice") && TryString(vote, "proposal_id", out proposalId)
            && TryString(vote, "choice", out choice)
            && (proposalId == "map" || proposalId.StartsWith("map:", StringComparison.Ordinal))
            && RuntimeV3GameplayContract.IsIdentity(proposalId)
            && RuntimeV3GameplayContract.IsIdentity(choice);
    }

    private static bool HasOnly(JsonElement value, params string[] fields)
    {
        if (value.ValueKind != JsonValueKind.Object) return false;
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
            if (!seen.Add(property.Name) || Array.IndexOf(fields, property.Name) < 0) return false;
        return seen.Count == fields.Length;
    }

    private static bool TryString(JsonElement value, string name, out string result)
    {
        result = string.Empty;
        return value.TryGetProperty(name, out JsonElement property)
            && property.ValueKind == JsonValueKind.String
            && (result = property.GetString() ?? string.Empty).Length > 0;
    }

    private static bool TryUInt64(JsonElement value, string name, out ulong result)
    {
        result = 0;
        return value.TryGetProperty(name, out JsonElement property)
            && property.ValueKind == JsonValueKind.Number && property.TryGetUInt64(out result);
    }

    private static bool IsNull(JsonElement value, string name) => value.TryGetProperty(name,
        out JsonElement property) && property.ValueKind == JsonValueKind.Null;

    private static CoopVoteDomain VoteDomain(string proposalId) => proposalId == "map"
        || proposalId.StartsWith("map:", StringComparison.Ordinal)
            ? CoopVoteDomain.Map : throw new ArgumentOutOfRangeException(nameof(proposalId));

    private static string SafeCode(string? value, string fallback) => value is not null
        && value.Length <= OpaqueOperationNativeMessage.MaximumRejectionCodeBytes
        && RuntimeV3GameplayContract.IsIdentity(value) ? value : fallback;

    private sealed record CarrierRequest(CarrierKind Kind, ulong ExpectedHostGeneration,
        string? ActionKind, string? ActionId, string? ProposalId, string? Choice);
    private enum CarrierKind { LocalAction, SharedVote }
}
