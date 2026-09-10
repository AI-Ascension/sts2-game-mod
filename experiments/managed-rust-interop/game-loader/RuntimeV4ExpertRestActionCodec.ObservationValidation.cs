// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private delegate bool ElementValidator(JsonElement value, out string error);

    private static bool ValidateObservation(
        JsonElement observation,
        string expectedStateId,
        ulong expectedGeneration,
        out string error)
    {
        error = string.Empty;
        if (!HasExactFields(observation,
                "protocol_version", "schema_digest", "provenance", "profile", "state_id",
                "generation", "visible_seed", "run", "player", "state", "legal_actions")
            || StringField(observation, "protocol_version")
                != RuntimeV4ExpertGameplayCodec.ProtocolVersion
            || StringField(observation, "schema_digest")
                != RuntimeV4ExpertGameplayCodec.SchemaDigest
            || StringField(observation, "profile") != RuntimeV4ExpertGameplayCodec.Profile
            || !GameplayProvenance(observation)
            || StringField(observation, "state_id") != expectedStateId
            || !UInt64Field(observation, "generation", expectedGeneration)
            || !NullableText(observation, "visible_seed")
            || !ValidateRun(observation.GetProperty("run"), out error)
            || !ValidatePlayer(observation.GetProperty("player"), out error)
            || !ValidateState(observation.GetProperty("state"), out error)
            || !ValidateLegalActions(observation.GetProperty("legal_actions"), out error))
        {
            if (error.Length == 0) error = "rest response observation is invalid";
            return false;
        }

        if (observation.GetProperty("state").GetProperty("state").GetString() == "selection"
            && (observation.GetProperty("state").GetProperty("choices").ValueKind
                    != JsonValueKind.Array
                || observation.GetProperty("state").GetProperty("choices").GetArrayLength() == 0))
        {
            return Fail(out error, "rest selector observation is empty");
        }
        return true;
    }

    private static bool ValidateObservationValue(
        RuntimeV4ExpertGameplayObservation observation,
        string expectedStateId,
        ulong expectedGeneration,
        out string error)
    {
        error = string.Empty;
        if (!RuntimeV4ExpertGameplayCodec.TrySerialize(observation, out string json, out error))
            return false;
        try
        {
            using JsonDocument document = JsonDocument.Parse(json,
                new JsonDocumentOptions { MaxDepth = 24 });
            return ValidateObservation(document.RootElement, expectedStateId,
                expectedGeneration, out error);
        }
        catch (JsonException)
        {
            return Fail(out error, "rest response observation serialization is invalid");
        }
    }

    private static bool GameplayProvenance(JsonElement value) =>
        value.TryGetProperty("provenance", out JsonElement provenance)
        && HasExactFields(provenance, "artifact", "source", "generator")
        && StringField(provenance, "artifact") == RuntimeV4ExpertGameplayCodec.Artifact
        && StringField(provenance, "source") == RuntimeV4ExpertGameplayCodec.SchemaSource
        && StringField(provenance, "generator") == RuntimeV4ExpertGameplayCodec.Generator;

    private static bool ValidateRun(JsonElement run, out string error)
    {
        error = string.Empty;
        if (!HasExactFields(run, "character_id", "act", "location")
            || !NullableIdentity(run, "character_id")
            || !NullableByte(run, "act")
            || !NullableIdentity(run, "location"))
            return Fail(out error, "rest observation run is invalid");
        return true;
    }

    private static bool ValidatePlayer(JsonElement player, out string error)
    {
        error = string.Empty;
        if (!HasExactFields(player, "hp", "max_hp", "block", "energy", "gold", "hand",
                "deck", "discard", "exhaust", "powers", "statuses", "relics", "potions",
                "potion_slots", "max_potion_slots")
            || !UInt16Field(player, "hp", out ushort hp)
            || !UInt16Field(player, "max_hp", out ushort maxHp)
            || hp > maxHp
            || !NullableUInt16(player, "block")
            || !ByteField(player, "energy")
            || !UInt32Field(player, "gold")
            || !ValidateCollection(player, "hand", ValidateCard, out error)
            || !ValidateCollection(player, "deck", ValidateCard, out error)
            || !ValidateCollection(player, "discard", ValidateCard, out error)
            || !ValidateCollection(player, "exhaust", ValidateCard, out error)
            || !ValidateCollection(player, "powers", ValidateStatus, out error)
            || !ValidateCollection(player, "statuses", ValidateStatus, out error)
            || !ValidateCollection(player, "relics", ValidateRelic, out error)
            || !ValidateCollection(player, "potions", ValidatePotion, out error)
            || !NullableByte(player, "potion_slots")
            || !NullableByte(player, "max_potion_slots"))
        {
            if (error.Length == 0) error = "rest observation player is invalid";
            return false;
        }
        return true;
    }

    private static bool ValidateCollection(
        JsonElement parent,
        string field,
        ElementValidator validator,
        out string error)
    {
        error = string.Empty;
        if (!parent.TryGetProperty(field, out JsonElement value))
            return Fail(out error, "rest observation collection is missing");
        if (value.ValueKind == JsonValueKind.Null) return true;
        if (value.ValueKind != JsonValueKind.Array
            || value.GetArrayLength() > RuntimeV3GameplayContract.MaxEntities)
            return Fail(out error, "rest observation collection is invalid");
        foreach (JsonElement item in value.EnumerateArray())
            if (!validator(item, out error)) return false;
        return true;
    }

    private static bool ValidateCard(JsonElement card, out string error)
    {
        error = string.Empty;
        if (!HasExactFields(card, "card_id", "name", "cost", "upgraded", "type", "rarity",
                "target", "description")
            || !IdentityField(card, "card_id")
            || !TextField(card, "name")
            || !NullableByte(card, "cost")
            || !BoolField(card, "upgraded")
            || !NullableIdentity(card, "type")
            || !NullableIdentity(card, "rarity")
            || !NullableIdentity(card, "target")
            || !NullableText(card, "description"))
            return Fail(out error, "rest observation card is invalid");
        return true;
    }

    private static bool ValidateStatus(JsonElement status, out string error)
    {
        error = string.Empty;
        if (!HasExactFields(status, "status_id", "name", "amount")
            || !IdentityField(status, "status_id")
            || !TextField(status, "name")
            || !NullableInt(status, "amount", -65535, 65535))
            return Fail(out error, "rest observation status is invalid");
        return true;
    }

    private static bool ValidateRelic(JsonElement relic, out string error)
    {
        error = string.Empty;
        if (!HasExactFields(relic, "relic_id", "name")
            || !IdentityField(relic, "relic_id")
            || !TextField(relic, "name"))
            return Fail(out error, "rest observation relic is invalid");
        return true;
    }

    private static bool ValidatePotion(JsonElement potion, out string error)
    {
        error = string.Empty;
        if (!HasExactFields(potion, "potion_id", "name", "slot", "usable", "target_mode")
            || !IdentityField(potion, "potion_id")
            || !TextField(potion, "name")
            || !NullableByte(potion, "slot")
            || !NullableBool(potion, "usable")
            || StringField(potion, "target_mode") is not ("self" or "any_enemy"
                or "all_enemies" or "none" or "unknown"))
            return Fail(out error, "rest observation potion is invalid");
        return true;
    }

    private static bool ValidateState(JsonElement state, out string error)
    {
        error = string.Empty;
        string? kind = StringField(state, "state");
        if (kind == "setup")
        {
            return HasExactFields(state, "state", "characters")
                && ValidateIdentityArray(state, "characters", 256, out error);
        }
        if (kind == "map")
        {
            return HasExactFields(state, "state", "current_node_id", "nodes", "edges", "options")
                && NullableIdentity(state, "current_node_id")
                && ValidateCollection(state, "nodes", ValidateMapNode, out error)
                && ValidateCollection(state, "edges", ValidateMapEdge, out error)
                && ValidateIdentityArray(state, "options", 256, out error);
        }
        if (kind == "combat")
        {
            return HasExactFields(state, "state", "turn_index", "enemies")
                && UInt16Field(state, "turn_index")
                && ValidateCollection(state, "enemies", ValidateEnemy, out error);
        }
        if (kind is "reward" or "event" or "rest" or "selection")
        {
            return HasExactFields(state, "state", "choices")
                && ValidateCollection(state, "choices", ValidateChoice, out error)
                && UniqueObjectIds(state, "choices", "choice_id", out error);
        }
        if (kind == "shop")
        {
            return HasExactFields(state, "state", "items")
                && ValidateCollection(state, "items", ValidateShopItem, out error)
                && UniqueObjectIds(state, "items", "item_id", out error);
        }
        if (kind == "victory") return HasExactFields(state, "state");
        if (kind == "defeat")
        {
            return HasExactFields(state, "state", "reason")
                && NullableText(state, "reason");
        }
        if (kind == "recovery")
        {
            return HasExactFields(state, "state", "code") && IdentityField(state, "code");
        }
        return Fail(out error, "rest observation state is unsupported");
    }

}
