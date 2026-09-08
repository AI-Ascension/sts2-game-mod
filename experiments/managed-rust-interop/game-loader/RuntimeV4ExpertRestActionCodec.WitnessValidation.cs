// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private static bool ValidateWitness(
        RuntimeV4ExpertRestEffectWitness witness,
        ulong generation,
        RuntimeV4ExpertRestOperation operation,
        string optionId,
        string stateId,
        out string error)
    {
        error = string.Empty;
        if (witness is null
            || witness.Operation is null
            || witness.Operation != operation
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
        return ValidateWitness(document.RootElement, generation, operation.OperationId, optionId,
            stateId, out error);
    }

    private static bool ValidateWitness(
        JsonElement witness,
        ulong generation,
        string operationId,
        string optionId,
        string stateId,
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
        return ValidateEvidence(evidence, kind!, stateId, out error);
    }

    private static bool ValidateEvidence(
        JsonElement evidence,
        string witnessKind,
        string stateId,
        out string error)
    {
        error = string.Empty;
        string? kind = StringField(evidence, "kind");
        if (kind is null)
        {
            error = "rest effect evidence kind is missing";
            return false;
        }
        if (witnessKind == "heal_applied")
            return kind == "hp_change"
                && ValidateHpEvidence(evidence, requireIncrease: true, out error)
                || Fail(out error, "HP evidence is required");
        if (witnessKind == "mend_applied")
            return kind is "hp_change" or "native_completion"
                && (kind == "hp_change"
                    ? ValidateHpEvidence(evidence, requireIncrease: true, out error)
                    : ValidateNativeEvidence(evidence, stateId, out error))
                || Fail(out error, "HP or native evidence is required");
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
                && (kind == "stat_change"
                    ? ValidateStatEvidence(evidence, out error)
                    : ValidateNativeEvidence(evidence, stateId, out error))
                || Fail(out error, "stat or native evidence is required");
        if (witnessKind == "kindle_applied")
            return kind == "native_completion"
                && ValidateNativeEvidence(evidence, stateId, out error)
                || Fail(out error, "native completion evidence is required");
        return Fail(out error, "rest effect witness kind is unsupported");
    }

    private static bool ValidateHpEvidence(
        JsonElement evidence,
        bool requireIncrease,
        out string error)
    {
        error = string.Empty;
        if (!HasExactFields(evidence, "kind", "hp_before", "hp_after", "max_hp_before",
                "max_hp_after")
            || !UInt16Field(evidence, "hp_before", out ushort before)
            || !UInt16Field(evidence, "hp_after", out ushort after)
            || !UInt16Field(evidence, "max_hp_before", out ushort maxBefore)
            || !UInt16Field(evidence, "max_hp_after", out ushort maxAfter)
            || before > maxBefore || after > maxAfter
            || requireIncrease && after <= before)
            return Fail(out error, "rest HP evidence is incomplete or unchanged");
        return true;
    }

    private static bool ValidateStatEvidence(JsonElement evidence, out string error)
    {
        error = string.Empty;
        if (!HasExactFields(evidence, "kind", "stat_id", "before", "after")
            || !IdentityField(evidence, "stat_id")
            || !Int32Field(evidence, "before", -65535, 65535, out int before)
            || !Int32Field(evidence, "after", -65535, 65535, out int after)
            || before == after)
            return Fail(out error, "rest stat evidence is incomplete or unchanged");
        return true;
    }

    private static bool ValidateNativeEvidence(
        JsonElement evidence,
        string stateId,
        out string error)
    {
        error = string.Empty;
        if (!HasExactFields(evidence, "kind", "completion_id", "native_state_id")
            || !IdentityField(evidence, "completion_id")
            || !IdentityField(evidence, "native_state_id")
            || StringField(evidence, "native_state_id") != stateId)
            return Fail(out error, "rest native completion evidence is invalid");
        return true;
    }

    private static bool ValidateCompletedWitnessBinding(
        JsonElement witness,
        JsonElement transition,
        RuntimeV4ExpertRestActionReference action,
        string optionId,
        out string error)
    {
        error = string.Empty;
        string? witnessKind = StringField(witness, "kind");
        if (optionId == "smith")
        {
            if (action.Action.Kind != "confirm_selection"
                || witnessKind != "smith_applied"
                || !JsonElementDeepEquals(
                    transition.GetProperty("selected_choice_ids"),
                    witness.GetProperty("evidence").GetProperty("upgraded_card_ids")))
                return Fail(out error, "Smith completion witness does not bind selected cards");
            return true;
        }
        if (optionId == "mend")
        {
            string? target = StringField(witness, "target_player_id");
            JsonElement selected = transition.GetProperty("selected_choice_ids");
            if (witnessKind != "mend_applied" || target is null
                || selected.GetArrayLength() != 1
                || selected[0].GetString() != target
                || action.Action.Kind == "select_player" && action.Action.PlayerId != target)
                return Fail(out error, "Mend completion witness does not bind selected player");
        }
        return true;
    }
}
