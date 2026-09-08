// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private static Dictionary<string, object?> Provenance() => new()
    {
        ["artifact"] = RuntimeV4ExpertRestActionContract.Artifact,
        ["source"] = RuntimeV4ExpertRestActionContract.SchemaSource,
        ["generator"] = RuntimeV4ExpertRestActionContract.Generator
    };

    private static Dictionary<string, object?> ActionReference(
        RuntimeV4ExpertRestActionReference reference) => new()
    {
        ["action_id"] = reference.ActionId,
        ["action"] = ActionPayload(reference.Action)
    };

    private static Dictionary<string, object?> ActionPayload(RuntimeV4ExpertRestAction action)
    {
        var result = new Dictionary<string, object?> { ["kind"] = action.Kind };
        switch (action.Kind)
        {
            case "rest_option":
                result["rest_option_id"] = action.RestOptionId;
                break;
            case "select_card":
                result["selection_id"] = action.SelectionId;
                result["rest_option_id"] = action.RestOptionId;
                result["card_id"] = action.CardId;
                break;
            case "select_player":
                result["selection_id"] = action.SelectionId;
                result["rest_option_id"] = action.RestOptionId;
                result["player_id"] = action.PlayerId;
                break;
            case "confirm_selection":
            case "cancel_selection":
                result["selection_id"] = action.SelectionId;
                result["rest_option_id"] = action.RestOptionId;
                break;
            default:
                throw new InvalidOperationException("unsupported rest action arm");
        }
        return result;
    }

    private static Dictionary<string, object?> Selector(RuntimeV4ExpertRestSelector selector) => new()
    {
        ["selection_id"] = selector.SelectionId,
        ["selection_kind"] = selector.SelectionKind,
        ["required_count"] = selector.RequiredCount,
        ["selected_choice_ids"] = selector.SelectedChoiceIds,
        ["remaining_count"] = selector.RemainingCount,
        ["legal_actions"] = selector.LegalActions.Select(ActionReference).ToArray()
    };

    private static Dictionary<string, object?> Transition(RuntimeV4ExpertRestTransition transition)
    {
        var result = new Dictionary<string, object?>
        {
            ["before_generation"] = transition.BeforeGeneration,
            ["after_generation"] = transition.AfterGeneration,
            ["rest_option_id"] = transition.RestOptionId
        };
        switch (transition)
        {
            case RuntimeV4ExpertRestCompletedTransition completed:
                result["kind"] = "rest_option_completed";
                result["completed"] = true;
                result["effect_witness"] = EffectWitness(completed.EffectWitness);
                break;
            case RuntimeV4ExpertRestSelectionRequestedTransition requested:
                result["kind"] = "rest_option_selection_requested";
                result["selector"] = Selector(requested.Selector);
                result["effect_witness"] = null;
                break;
            case RuntimeV4ExpertRestSelectionProgressedTransition progressed:
                result["kind"] = "rest_option_selection_progressed";
                result["selection_id"] = progressed.Selector.SelectionId;
                result["selection_kind"] = progressed.Selector.SelectionKind;
                result["required_count"] = progressed.Selector.RequiredCount;
                result["selected_choice_ids"] = progressed.Selector.SelectedChoiceIds;
                result["remaining_count"] = progressed.Selector.RemainingCount;
                result["selector"] = Selector(progressed.Selector);
                result["effect_witness"] = null;
                break;
            case RuntimeV4ExpertRestSelectionCompletedTransition selected:
                result["kind"] = "rest_option_selection_completed";
                result["selection_id"] = selected.SelectionId;
                result["selection_kind"] = selected.SelectionKind;
                result["required_count"] = selected.RequiredCount;
                result["selected_choice_ids"] = selected.SelectedChoiceIds;
                result["remaining_count"] = 0;
                result["completed"] = true;
                result["effect_witness"] = EffectWitness(selected.EffectWitness);
                break;
            default:
                throw new InvalidOperationException("unsupported rest transition arm");
        }
        return result;
    }

    private static Dictionary<string, object?> EffectWitness(
        RuntimeV4ExpertRestEffectWitness witness)
    {
        var result = new Dictionary<string, object?>
        {
            ["version"] = RuntimeV4ExpertRestActionContract.EffectWitnessVersion,
            ["kind"] = witness.Kind,
            ["operation_id"] = witness.Operation.OperationId,
            ["rest_option_id"] = witness.RestOptionId,
            ["generation"] = witness.Generation,
            ["evidence"] = Evidence(witness.Evidence)
        };
        if (witness.TargetPlayerId is not null)
            result["target_player_id"] = witness.TargetPlayerId;
        return result;
    }

    private static Dictionary<string, object?> Evidence(RuntimeV4ExpertRestEvidence evidence) =>
        evidence switch
        {
            RuntimeV4ExpertRestHpEvidence hp => new()
            {
                ["kind"] = hp.Kind, ["hp_before"] = hp.HpBefore, ["hp_after"] = hp.HpAfter,
                ["max_hp_before"] = hp.MaxHpBefore, ["max_hp_after"] = hp.MaxHpAfter
            },
            RuntimeV4ExpertRestCardEvidence cards => new()
            {
                ["kind"] = cards.Kind, ["added_card_ids"] = cards.AddedCardIds,
                ["removed_card_ids"] = cards.RemovedCardIds,
                ["upgraded_card_ids"] = cards.UpgradedCardIds
            },
            RuntimeV4ExpertRestRelicEvidence relics => new()
            {
                ["kind"] = relics.Kind, ["added_relic_ids"] = relics.AddedRelicIds,
                ["removed_relic_ids"] = relics.RemovedRelicIds
            },
            RuntimeV4ExpertRestStatEvidence stat => new()
            {
                ["kind"] = stat.Kind, ["stat_id"] = stat.StatId,
                ["before"] = stat.Before, ["after"] = stat.After
            },
            RuntimeV4ExpertRestNativeEvidence native => new()
            {
                ["kind"] = native.Kind, ["completion_id"] = native.CompletionId,
                ["native_state_id"] = native.NativeStateId
            },
            _ => throw new InvalidOperationException("unsupported rest evidence arm")
        };

    private static bool BoundedIds(JsonElement value, string field) =>
        BoundedIds(value, field, out _);

    private static bool BoundedIds(
        JsonElement value,
        string field,
        out string[] ids)
    {
        ids = Array.Empty<string>();
        if (!value.TryGetProperty(field, out JsonElement array)
            || array.ValueKind != JsonValueKind.Array
            || array.GetArrayLength() > RuntimeV4ExpertRestActionContract.MaxChoices)
            return false;
        var parsed = new List<string>(array.GetArrayLength());
        var unique = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonElement item in array.EnumerateArray())
        {
            string? id = item.ValueKind == JsonValueKind.String ? item.GetString() : null;
            if (!RuntimeV4ExpertRestActionContract.IsIdentity(id) || !unique.Add(id!))
                return false;
            parsed.Add(id!);
        }
        ids = parsed.ToArray();
        return true;
    }

    private static bool Provenance(JsonElement root) =>
        root.TryGetProperty("provenance", out JsonElement value)
        && HasExactFields(value, "artifact", "source", "generator")
        && StringField(value, "artifact") == RuntimeV4ExpertRestActionContract.Artifact
        && StringField(value, "source") == RuntimeV4ExpertRestActionContract.SchemaSource
        && StringField(value, "generator") == RuntimeV4ExpertRestActionContract.Generator;

    private static bool NullField(JsonElement root, string field) =>
        root.TryGetProperty(field, out JsonElement value) && value.ValueKind == JsonValueKind.Null;

    private static bool UInt64Field(JsonElement root, string field, ulong expected) =>
        UInt64Field(root, field, out ulong value) && value == expected;

    private static bool UInt64Field(JsonElement root, string field, out ulong value)
    {
        value = 0;
        return root.TryGetProperty(field, out JsonElement element)
            && element.ValueKind == JsonValueKind.Number
            && element.TryGetUInt64(out value)
            && value <= RuntimeV3GameplayContract.MaxGeneration;
    }

    private static string? StringField(JsonElement root, string field) =>
        root.TryGetProperty(field, out JsonElement value)
            && value.ValueKind == JsonValueKind.String ? value.GetString() : null;

    private static bool HasExactFields(JsonElement value, params string[] fields)
    {
        if (value.ValueKind != JsonValueKind.Object) return false;
        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
            if (!names.Add(property.Name)) return false;
        return names.Count == fields.Length && fields.All(names.Contains);
    }

    private static bool HasExactFieldsOrOptional(
        JsonElement value, string optional, params string[] fields)
    {
        if (value.ValueKind != JsonValueKind.Object) return false;
        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
            if (!names.Add(property.Name)) return false;
        return (names.Count == fields.Length || names.Count == fields.Length + 1)
            && fields.All(names.Contains)
            && (!names.Contains(optional) || names.Count == fields.Length + 1);
    }

    private static bool JsonElementDeepEquals(JsonElement left, JsonElement right) =>
        JsonSerializer.Serialize(left) == JsonSerializer.Serialize(right);

    private static bool Fail(out string error, string value)
    {
        error = value;
        return false;
    }
}
