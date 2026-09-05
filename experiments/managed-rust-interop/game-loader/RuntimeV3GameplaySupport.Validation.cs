// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplaySupport
{
    private static bool TryParseRequest(
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        string leaseEpochText,
        string body,
        out JsonDocument? document,
        out JsonElement root,
        out ulong generation,
        out string kind,
        out string? error)
    {
        document = null;
        root = default;
        generation = 0;
        kind = string.Empty;
        error = null;
        try
        {
            document = JsonDocument.Parse(body, new JsonDocumentOptions { MaxDepth = 16 });
        }
        catch (JsonException)
        {
            error = "invalid_json";
            return false;
        }
        root = document!.RootElement;
        if (root.ValueKind != JsonValueKind.Object
            || !HasExactFields(root, EnvelopeFields)
            || !RuntimeV3GameplayContract.IsIdentity(instanceId)
            || !RuntimeV3GameplayContract.IsIdentity(sessionId)
            || !RuntimeV3GameplayContract.IsIdentity(leaseId)
            || !RuntimeV3GameplayContract.IsIdentity(correlationId)
            || !root.TryGetProperty("provenance", out JsonElement provenance)
            || !HasExactFields(provenance, "artifact", "source", "generator")
            || StringField(root, "protocol_version") != RuntimeV3GameplayContract.ProtocolVersion
            || StringField(root, "schema_digest") != RuntimeV3GameplayContract.SchemaDigest
            || StringField(root, "correlation_id") != correlationId
            || StringField(root, "instance_id") != instanceId
            || StringField(root, "session_id") != sessionId
            || StringField(root, "lease_id") != leaseId
            || StringField(root, "provenance.artifact") != RuntimeV3GameplayContract.Artifact
            || StringField(root, "provenance.source") != RuntimeV3GameplayContract.SchemaSource
            || StringField(root, "provenance.generator") != RuntimeV3GameplayContract.Generator
            || !TryEpoch(root, "lease_epoch", leaseEpochText, out _)
            || !root.TryGetProperty("generation", out JsonElement generationElement)
            || generationElement.ValueKind != JsonValueKind.Number
            || !generationElement.TryGetUInt64(out generation)
            || generation > RuntimeV3GameplayContract.MaxGeneration
            || !TryString(root, "kind", out string? parsedKind))
        {
            error = "invalid_runtime_v3_envelope";
            document!.Dispose();
            document = null;
            return false;
        }
        kind = parsedKind!;
        if (!ValidateRequestShape(root, kind, generation))
        {
            error = "invalid_runtime_v3_request_shape";
            document!.Dispose();
            document = null;
            return false;
        }
        return true;
    }

    private static bool ValidateRequestShape(JsonElement root, string kind, ulong generation) =>
        kind switch
        {
            "state_request" or "reobserve_request" => NullFields(
                root, "state_id", "operation_id", "observation", "legal_actions", "action",
                "status", "transition", "error_code", "wait_for_millis", "wait_outcome", "recovery"),
            "legal_actions_request" => TryString(root, "state_id", out _)
                && NullFields(
                    root, "operation_id", "observation", "legal_actions", "action", "status",
                    "transition", "error_code", "wait_for_millis", "wait_outcome", "recovery"),
            "dispatch_action_request" => TryString(root, "state_id", out _)
                && TryString(root, "operation_id", out _)
                && TryAction(root, generation, out _)
                && NullFields(
                    root, "observation", "legal_actions", "status", "transition", "error_code",
                    "wait_for_millis", "wait_outcome", "recovery"),
            "wait_request" => TryString(root, "operation_id", out _)
                && TryWait(root)
                && NullFields(
                    root, "state_id", "observation", "legal_actions", "action", "status",
                    "transition", "error_code", "wait_outcome", "recovery"),
            "recover_request" => NullFields(
                    root, "state_id", "operation_id", "observation", "legal_actions", "action",
                    "status", "transition", "error_code", "wait_for_millis", "wait_outcome")
                && TryRecovery(root),
            _ => false
        };

    private static bool NullFields(JsonElement root, params string[] fields)
    {
        foreach (string field in fields)
        {
            if (!root.TryGetProperty(field, out JsonElement value)
                || value.ValueKind != JsonValueKind.Null)
            {
                return false;
            }
        }
        return true;
    }

    private static bool TryWait(JsonElement root)
    {
        return root.TryGetProperty("wait_for_millis", out JsonElement value)
            && value.ValueKind == JsonValueKind.Number
            && value.TryGetInt32(out int waitForMillis)
            && waitForMillis is >= 1 and <= 120_000;
    }

    private static bool TryRecovery(JsonElement root)
    {
        if (!root.TryGetProperty("recovery", out JsonElement recovery)
            || !HasExactFields(recovery, "kind", "operation_id")
            || !TryString(recovery, "kind", out string? recoveryKind)
            || !MatchesRecoveryKind(recoveryKind)
            || !TryOptionalString(recovery, "operation_id", out string? operationId))
        {
            return false;
        }
        return (recoveryKind == "reconcile") == (operationId is not null);
    }

    private static bool MatchesRecoveryKind(string? kind) =>
        kind is "reobserve" or "reconcile" or "release_lease" or "stop_episode";

    private static bool TryAction(
        JsonElement root,
        ulong generation,
        out LegalActionReference? action)
    {
        action = null;
        if (!root.TryGetProperty("action", out JsonElement value)
            || value.ValueKind != JsonValueKind.Object
            || !HasExactFields(value, "action_id", "action")
            || !TryString(value, "action_id", out string? actionId)
            || !value.TryGetProperty("action", out JsonElement payload)
            || payload.ValueKind != JsonValueKind.Object
            || !TryString(payload, "kind", out string? kind))
        {
            return false;
        }
        string? selectedValue = null;
        string? targetId = null;
        string? field = kind switch
        {
            "start_run" => "character_id",
            "select_map_node" => "node_id",
            "choose_reward" => "reward_id",
            "shop_purchase" => "item_id",
            "shop_remove" or "smith" or "select_card" => "card_id",
            "event_choice" => "choice_id",
            "play_card" => "card_id",
            "end_turn" or "skip_reward" or "rest" or "confirm_victory" or "save_quit" => null,
            _ => "invalid"
        };
        if (field == "invalid")
        {
            return false;
        }
        if (field is null)
        {
            if (!HasExactFields(payload, "kind"))
            {
                return false;
            }
        }
        else if (kind == "play_card")
        {
            if (!HasExactFields(payload, "kind", "card_id", "target_id")
                || !TryString(payload, "card_id", out selectedValue)
                || !TryOptionalString(payload, "target_id", out targetId))
            {
                return false;
            }
        }
        else if (!HasExactFields(payload, "kind", field)
            || !TryString(payload, field, out selectedValue))
        {
            return false;
        }
        if (actionId is null || kind is null)
        {
            return false;
        }
        action = new LegalActionReference(actionId, kind, selectedValue, targetId, generation);
        return action.Validate(out _);
    }

    private static string? StringField(JsonElement root, string path)
    {
        JsonElement value = root;
        foreach (string segment in path.Split('.'))
        {
            if (value.ValueKind != JsonValueKind.Object || !value.TryGetProperty(segment, out value))
            {
                return null;
            }
        }
        return value.ValueKind == JsonValueKind.String ? value.GetString() : null;
    }

    private static bool TryString(JsonElement root, string property, out string? value)
    {
        value = StringField(root, property);
        return value is not null && RuntimeV3GameplayContract.IsIdentity(value);
    }

    private static bool TryOptionalString(JsonElement root, string property, out string? value)
    {
        value = null;
        if (!root.TryGetProperty(property, out JsonElement element))
        {
            return false;
        }
        if (element.ValueKind == JsonValueKind.Null)
        {
            return true;
        }
        return element.ValueKind == JsonValueKind.String
            && RuntimeV3GameplayContract.IsIdentity(value = element.GetString() ?? string.Empty);
    }

    private static bool TryEpoch(JsonElement root, string property, string expected, out ulong value)
    {
        value = 0;
        return ulong.TryParse(expected, out ulong expectedEpoch)
            && expectedEpoch <= RuntimeV3GameplayContract.MaxGeneration
            && root.TryGetProperty(property, out JsonElement element)
            && element.ValueKind == JsonValueKind.Number
            && element.TryGetUInt64(out value)
            && value <= RuntimeV3GameplayContract.MaxGeneration
            && value == expectedEpoch;
    }

    private static ulong ParseEpoch(string value) =>
        ulong.TryParse(value, out ulong epoch)
            && epoch <= RuntimeV3GameplayContract.MaxGeneration
            ? epoch
            : 0;
}
