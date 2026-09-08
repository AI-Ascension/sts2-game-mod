// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private static bool ValidateWitness(
        RuntimeV4ExpertRestEffectWitness witness,
        ulong generation,
        string operationId,
        string optionId,
        out string error)
    {
        error = string.Empty;
        if (witness is null
            || witness.Operation is null
            || witness.Operation.OperationId != operationId
            || witness.RestOptionId != optionId
            || witness.Generation != generation
            || (optionId == "mend") != (witness.TargetPlayerId is not null)
            || witness.TargetPlayerId is not null
                && !RuntimeV4ExpertRestActionContract.IsIdentity(witness.TargetPlayerId))
        {
            error = "rest effect witness identity is invalid";
            return false;
        }
        using JsonDocument document = JsonDocument.Parse(JsonSerializer.Serialize(
            EffectWitness(witness)));
        return ValidateWitness(document.RootElement, generation, operationId, optionId, out error);
    }

    private static bool ValidateWitness(
        JsonElement witness,
        ulong generation,
        string operationId,
        string optionId,
        out string error)
    {
        error = string.Empty;
        if (!HasExactFieldsOrOptional(witness, "target_player_id",
                "version", "kind", "operation_id", "rest_option_id", "generation", "evidence")
            || StringField(witness, "version")
                != RuntimeV4ExpertRestActionContract.EffectWitnessVersion
            || StringField(witness, "operation_id") != operationId
            || StringField(witness, "rest_option_id") != optionId
            || !UInt64Field(witness, "generation", generation)
            || !witness.TryGetProperty("evidence", out JsonElement evidence)
            || evidence.ValueKind != JsonValueKind.Object)
        {
            error = "rest effect witness identity is invalid";
            return false;
        }
        string? kind = StringField(witness, "kind");
        if (kind != optionId + "_applied")
        {
            error = "rest effect witness kind does not match its option";
            return false;
        }
        bool targetRequired = optionId == "mend";
        string? target = null;
        if (witness.TryGetProperty("target_player_id", out JsonElement targetValue))
        {
            target = targetValue.ValueKind == JsonValueKind.Null
                ? null : StringField(witness, "target_player_id");
            if (targetValue.ValueKind is not (JsonValueKind.Null or JsonValueKind.String))
            {
                error = "rest effect witness target field has an invalid type";
                return false;
            }
        }
        if (targetRequired != (target is not null)
            || target is not null && !RuntimeV4ExpertRestActionContract.IsIdentity(target))
        {
            error = "rest effect witness target identity is invalid";
            return false;
        }
        return ValidateEvidence(evidence, kind!, out error);
    }

    private static bool ValidateEvidence(JsonElement evidence, string witnessKind, out string error)
    {
        error = string.Empty;
        string? kind = StringField(evidence, "kind");
        if (kind is null)
        {
            error = "rest effect evidence kind is missing";
            return false;
        }
        if (witnessKind is "heal_applied" or "mend_applied")
            return kind is "hp_change" or "native_completion"
                ? true : Fail(out error, "HP or native evidence is required");
        if (witnessKind is "clone_applied" or "cook_applied" or "smith_applied")
            return kind == "card_change"
                ? HasExactFields(evidence, "kind", "added_card_ids", "removed_card_ids", "upgraded_card_ids")
                    && BoundedIds(evidence, "added_card_ids") && BoundedIds(evidence, "removed_card_ids")
                    && BoundedIds(evidence, "upgraded_card_ids")
                    && (witnessKind != "clone_applied"
                        || evidence.GetProperty("added_card_ids").GetArrayLength() > 0)
                    && (witnessKind != "cook_applied"
                        || evidence.GetProperty("removed_card_ids").GetArrayLength() > 0)
                    && (witnessKind != "smith_applied"
                        || evidence.GetProperty("upgraded_card_ids").GetArrayLength() > 0)
                : Fail(out error, "card evidence is required");
        if (witnessKind is "dig_applied" or "hatch_applied")
            return kind == "relic_change"
                ? HasExactFields(evidence, "kind", "added_relic_ids", "removed_relic_ids")
                    && BoundedIds(evidence, "added_relic_ids") && BoundedIds(evidence, "removed_relic_ids")
                    && evidence.GetProperty("added_relic_ids").GetArrayLength() > 0
                : Fail(out error, "relic evidence is required");
        if (witnessKind == "lift_applied")
            return kind is "stat_change" or "native_completion"
                ? true : Fail(out error, "stat or native evidence is required");
        if (witnessKind == "kindle_applied")
            return kind == "native_completion"
                ? HasExactFields(evidence, "kind", "completion_id", "native_state_id")
                    && RuntimeV4ExpertRestActionContract.IsIdentity(StringField(evidence, "completion_id"))
                    && RuntimeV4ExpertRestActionContract.IsIdentity(StringField(evidence, "native_state_id"))
                : Fail(out error, "native completion evidence is required");
        return Fail(out error, "rest effect witness kind is unsupported");
    }
}
