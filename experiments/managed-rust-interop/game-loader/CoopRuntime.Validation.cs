// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class CoopNativeRuntime
{
    private static bool HasClosedEnvelope(JsonElement root)
    {
        if (root.ValueKind != JsonValueKind.Object) return false;
        string[] fields = { "protocol_version", "schema_digest", "provenance", "correlation_id", "instance_id", "session_id", "lease_id", "lease_epoch", "kind", "operation_id", "actor_peer", "expected_host_generation", "action", "vote", "status", "observation", "effect", "recovery", "catalog", "receipt" };
        if (!HasOnlyFields(root, fields) || !HasRequiredFields(root, fields)) return false;
        if (StringField(root, "protocol_version") != ProtocolVersion
            || StringField(root, "schema_digest") != SchemaDigest
            || !HasValidProvenance(root)
            || !HasKnownNestedFields(root))
        {
            return false;
        }
        return true;
    }

    private static bool HasValidProvenance(JsonElement root)
    {
        if (!root.TryGetProperty("provenance", out JsonElement provenance)
            || provenance.ValueKind != JsonValueKind.Object
            || !HasOnlyFields(provenance, "artifact", "source", "generator")
            || !HasRequiredFields(provenance, "artifact", "source", "generator"))
        {
            return false;
        }
        return StringField(provenance, "artifact") == Artifact
            && StringField(provenance, "source") == SchemaSource
            && StringField(provenance, "generator") == "hand-authored";
    }

    private static bool HasKnownNestedFields(JsonElement root)
    {
        if (!HasNullableObjectFields(root, "action", "kind", "action_id", "target_peer")
            || !HasNullableObjectFields(root, "vote", "proposal_id", "voter_peer", "choice")
            || !HasNullableObjectFields(root, "recovery", "kind", "rejoin_epoch")
            || !HasNullableObjectFields(root, "catalog", "host_generation", "actor_peer", "actions", "votes")
            || !HasNullableObjectFields(root, "receipt", "operation_id", "status", "before_host_generation", "after_host_generation", "authority_id", "authority_epoch", "checkpoint_id", "state_digest", "native_checksum", "error_code"))
        {
            return false;
        }

        // Requests never carry a producer observation or effect. Requiring null here prevents
        // the hand-written parser from silently accepting an unvalidated nested projection.
        return root.GetProperty("observation").ValueKind == JsonValueKind.Null
            && root.GetProperty("effect").ValueKind == JsonValueKind.Null
            && root.GetProperty("status").ValueKind == JsonValueKind.Null;
    }

    private static bool HasNullableObjectFields(JsonElement root, string name, params string[] fields)
    {
        JsonElement value = root.GetProperty(name);
        return value.ValueKind == JsonValueKind.Null
            || value.ValueKind == JsonValueKind.Object
                && HasOnlyFields(value, fields);
    }

    private static bool HasRequiredFields(JsonElement root, params string[] fields)
    {
        foreach (string field in fields)
        {
            if (!root.TryGetProperty(field, out _)) return false;
        }
        return true;
    }

    private static bool HasOnlyFields(JsonElement root, params string[] fields)
    {
        if (root.ValueKind != JsonValueKind.Object) return false;
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in root.EnumerateObject())
        {
            if (!seen.Add(property.Name)) return false;
            bool known = false;
            foreach (string field in fields)
            {
                if (string.Equals(property.Name, field, StringComparison.Ordinal))
                {
                    known = true;
                    break;
                }
            }
            if (!known) return false;
        }
        return true;
    }

    private static bool TryRequestFields(JsonElement root, out string operationId, out string actorPeer, out ulong expectedGeneration)
    {
        operationId = NullableStringField(root, "operation_id") ?? string.Empty;
        actorPeer = NullableStringField(root, "actor_peer") ?? string.Empty;
        expectedGeneration = 0;
        return RuntimeV3GameplayContract.IsIdentity(operationId)
            && CoopPeerIdentity.IsOpaque(actorPeer)
            && TryNumber(root, "expected_host_generation", out expectedGeneration);
    }

    private static bool TryCatalogFields(JsonElement root, out string actorPeer, out ulong expectedGeneration)
    {
        actorPeer = NullableStringField(root, "actor_peer") ?? string.Empty;
        expectedGeneration = 0;
        return CoopPeerIdentity.IsOpaque(actorPeer)
            && TryNumber(root, "expected_host_generation", out expectedGeneration)
            && root.GetProperty("operation_id").ValueKind == JsonValueKind.Null
            && root.GetProperty("status").ValueKind == JsonValueKind.Null
            && root.GetProperty("observation").ValueKind == JsonValueKind.Null
            && root.GetProperty("effect").ValueKind == JsonValueKind.Null
            && root.GetProperty("action").ValueKind == JsonValueKind.Null
            && root.GetProperty("vote").ValueKind == JsonValueKind.Null
            && root.GetProperty("recovery").ValueKind == JsonValueKind.Null
            && root.GetProperty("catalog").ValueKind == JsonValueKind.Null
            && root.GetProperty("receipt").ValueKind == JsonValueKind.Null;
    }

    private static bool TryAction(JsonElement action, out string kind, out string actionId, out string? targetPeer)
    {
        kind = StringField(action, "kind") ?? string.Empty;
        actionId = StringField(action, "action_id") ?? string.Empty;
        targetPeer = NullableStringField(action, "target_peer");
        return HasRequiredFields(action, "kind", "action_id", "target_peer")
            && RuntimeV3GameplayContract.IsIdentity(actionId)
            && kind is "play_card" or "end_turn" or "select_card" or "choose_reward" or "confirm_selection"
            && (targetPeer is null || CoopPeerIdentity.IsOpaque(targetPeer));
    }

    private static bool TryVote(JsonElement vote, string actorPeer, out string proposalId,
        out string choice, out string voterPeer, out CoopVoteDomain domain)
    {
        proposalId = StringField(vote, "proposal_id") ?? string.Empty;
        choice = StringField(vote, "choice") ?? string.Empty;
        voterPeer = StringField(vote, "voter_peer") ?? string.Empty;
        domain = VoteDomain(proposalId);
        return RuntimeV3GameplayContract.IsIdentity(proposalId)
            && CoopPeerIdentity.IsOpaque(voterPeer)
            && voterPeer == actorPeer
            && RuntimeV3GameplayContract.IsIdentity(choice);
    }

    private static CoopVoteDomain VoteDomain(string proposalId) =>
        proposalId is "event" || proposalId.StartsWith("event:", StringComparison.Ordinal)
            ? CoopVoteDomain.SharedEvent
            : proposalId is "relic" || proposalId.StartsWith("relic:", StringComparison.Ordinal)
                ? CoopVoteDomain.TreasureRelic
                : proposalId is "map" || proposalId.StartsWith("map:", StringComparison.Ordinal)
                    ? CoopVoteDomain.Map
                    : CoopVoteDomain.PlayerChoice;

    private static string? NullableStringField(JsonElement root, string name) =>
        root.TryGetProperty(name, out JsonElement value) && value.ValueKind == JsonValueKind.String
            ? value.GetString() : root.TryGetProperty(name, out value) && value.ValueKind == JsonValueKind.Null ? null : null;

    private static string? StringField(JsonElement root, string name) =>
        root.TryGetProperty(name, out JsonElement value) && value.ValueKind == JsonValueKind.String
            ? value.GetString() : null;

    private static bool NumberField(JsonElement root, string name, ulong expected) =>
        TryNumber(root, name, out ulong value) && value == expected;

    private static bool TryNumber(JsonElement root, string name, out ulong value)
    {
        value = 0;
        return root.TryGetProperty(name, out JsonElement number)
            && number.ValueKind == JsonValueKind.Number
            && number.TryGetUInt64(out value)
            && value <= RuntimeV3GameplayContract.MaxGeneration;
    }

    private static string Serialize(Dictionary<string, object?> value)
    {
        string output = JsonSerializer.Serialize(value, JsonOptions);
        if (output.Length > MaxResponseBytes) throw new InvalidOperationException("co-op response is oversized");
        return output;
    }

    private static (int Status, string Response) Error(int status, string code) =>
        (status, JsonSerializer.Serialize(new Dictionary<string, object?> { ["error_code"] = code }, JsonOptions));
}
