// SPDX-License-Identifier: MIT

using System;
using System.Buffers;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Globalization;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Serializes one owned <see cref="CheckpointPayloadRecord"/> to UTF-8 JSON in the
/// <c>checkpoint-payload-v1</c> shape. Object keys are emitted in lexicographic order and
/// <c>uint64</c> values in the tagged <c>asc-jcs-state-v1</c> form; the Rust owner still applies
/// the authoritative canonical encoding before any identity is derived.
/// </summary>
internal static class CheckpointCapturePayloadWriter
{
    private static readonly JsonWriterOptions Options = new() { Indented = false };

    internal static ImmutableArray<byte> Write(CheckpointPayloadRecord record)
    {
        ArgumentNullException.ThrowIfNull(record);
        var buffer = new ArrayBufferWriter<byte>();
        using (var writer = new Utf8JsonWriter(buffer, Options))
        {
            writer.WriteStartObject();
            writer.WriteString("boundary", record.Boundary.Code());
            writer.WriteString("canonical_profile", CheckpointPayloadRecord.CanonicalProfile);
            writer.WritePropertyName("families");
            WriteFamilies(writer, record.Families);
            writer.WriteString("schema", CheckpointPayloadRecord.Schema);
            writer.WriteEndObject();
            writer.Flush();
        }
        return ImmutableArray.Create(buffer.WrittenSpan);
    }

    private static void WriteFamilies(Utf8JsonWriter writer, CheckpointPayloadFamilies families)
    {
        writer.WriteStartObject();
        Family(writer, "campaign_progress", families.CampaignProgress, WriteCampaignProgress);
        Family(writer, "card_instances", families.CardInstances, WriteCardInstances);
        Family(writer, "combat_turn", families.CombatTurn, WriteCombatTurn);
        Family(writer, "deck_and_piles", families.DeckAndPiles, WriteDeckAndPiles);
        Family(writer, "enemies_and_intents", families.EnemiesAndIntents, WriteEnemies);
        Family(writer, "pending_effects", families.PendingEffects, WritePendingEffects);
        Family(writer, "player_resources", families.PlayerResources, WritePlayerResources);
        Family(writer, "potions", families.Potions, WritePotions);
        Family(writer, "powers", families.Powers, WritePowers);
        Family(writer, "relics", families.Relics, WriteRelics);
        Family(writer, "run_configuration", families.RunConfiguration, WriteRunConfiguration);
        Family(writer, "seed_and_rng", families.SeedAndRng, WriteSeedAndRng);
        writer.WriteEndObject();
    }

    private static void Family<T>(
        Utf8JsonWriter writer,
        string name,
        CheckpointPayloadFamily<T> family,
        Action<Utf8JsonWriter, T> body) where T : class
    {
        writer.WritePropertyName(name);
        writer.WriteStartObject();
        writer.WriteString("coverage", family.Coverage.Code());
        if (family.Coverage == CheckpointPayloadCoverage.Captured && family.Value is { } value)
        {
            writer.WritePropertyName("value");
            writer.WriteStartObject();
            body(writer, value);
            writer.WriteEndObject();
        }
        writer.WriteEndObject();
    }

    private static void WriteSeedAndRng(Utf8JsonWriter writer, CheckpointPayloadSeedAndRng value)
    {
        writer.WriteString("derivation_version", value.DerivationVersion);
        writer.WriteString("master_seed", value.MasterSeed);
        writer.WriteStartArray("streams");
        foreach (CheckpointPayloadRngStream stream in value.Streams)
        {
            writer.WriteStartObject();
            writer.WriteString("algorithm", stream.Algorithm);
            writer.WritePropertyName("cursor");
            UInt64(writer, stream.Cursor);
            writer.WriteStartArray("state_words");
            foreach (ulong word in stream.StateWords)
                UInt64(writer, word);
            writer.WriteEndArray();
            writer.WriteString("stream_id", stream.StreamId);
            writer.WriteEndObject();
        }
        writer.WriteEndArray();
    }

    private static void WriteRunConfiguration(
        Utf8JsonWriter writer, CheckpointPayloadRunConfiguration value)
    {
        Strings(writer, "act_sequence", value.ActSequence);
        writer.WriteNumber("ascension", value.Ascension);
        writer.WriteString("character", value.Character);
        writer.WriteString("mode", value.Mode);
        Strings(writer, "modifiers", value.Modifiers);
    }

    private static void WriteCampaignProgress(
        Utf8JsonWriter writer, CheckpointPayloadCampaignProgress value)
    {
        writer.WriteNumber("act_index", value.ActIndex);
        writer.WriteString("current_node", value.CurrentNode);
        writer.WriteNumber("floor", value.Floor);
        writer.WriteString("room_kind", value.RoomKind);
        Strings(writer, "visited_nodes", value.VisitedNodes);
    }

    private static void WritePlayerResources(
        Utf8JsonWriter writer, CheckpointPayloadPlayerResources value)
    {
        writer.WriteNumber("current_hp", value.CurrentHp);
        writer.WriteNumber("gold", value.Gold);
        Strings(writer, "keys", value.Keys);
        writer.WriteNumber("max_hp", value.MaxHp);
    }

    private static void WriteDeckAndPiles(Utf8JsonWriter writer, CheckpointPayloadDeckAndPiles value)
    {
        Strings(writer, "deck", value.Deck);
        writer.WriteStartArray("piles");
        foreach (CheckpointPayloadCardPile pile in value.Piles)
        {
            writer.WriteStartObject();
            Strings(writer, "cards", pile.Cards);
            writer.WriteString("pile_id", pile.PileId);
            writer.WriteEndObject();
        }
        writer.WriteEndArray();
    }

    private static void WriteCardInstances(
        Utf8JsonWriter writer, CheckpointPayloadCardInstances value)
    {
        writer.WriteStartArray("cards");
        foreach (CheckpointPayloadCardInstance card in value.Cards)
        {
            writer.WriteStartObject();
            writer.WriteString("definition_id", card.DefinitionId);
            writer.WriteString("instance_id", card.InstanceId);
            writer.WriteStartArray("temporary_values");
            foreach (CheckpointPayloadTemporaryValue temporary in card.TemporaryValues)
            {
                writer.WriteStartObject();
                writer.WriteString("key", temporary.Key);
                writer.WriteNumber("value", temporary.Value);
                writer.WriteEndObject();
            }
            writer.WriteEndArray();
            writer.WriteNumber("upgrade_level", card.UpgradeLevel);
            writer.WriteEndObject();
        }
        writer.WriteEndArray();
    }

    private static void WriteRelics(Utf8JsonWriter writer, CheckpointPayloadRelics value)
    {
        writer.WriteStartArray("relics");
        foreach (CheckpointPayloadRelic relic in value.Relics)
        {
            writer.WriteStartObject();
            writer.WriteNumber("counter", relic.Counter);
            writer.WriteString("relic_id", relic.RelicId);
            writer.WriteEndObject();
        }
        writer.WriteEndArray();
    }

    private static void WritePotions(Utf8JsonWriter writer, CheckpointPayloadPotions value)
    {
        writer.WriteNumber("capacity", value.Capacity);
        writer.WriteStartArray("slots");
        foreach (CheckpointPayloadPotionSlot slot in value.Slots)
        {
            writer.WriteStartObject();
            writer.WriteNumber("index", slot.Index);
            writer.WriteString("potion_id", slot.PotionId);
            writer.WriteEndObject();
        }
        writer.WriteEndArray();
    }

    private static void WriteCombatTurn(Utf8JsonWriter writer, CheckpointPayloadCombatTurn value)
    {
        writer.WriteNumber("energy", value.Energy);
        writer.WriteNumber("player_block", value.PlayerBlock);
        writer.WriteNumber("turn", value.Turn);
    }

    private static void WritePowers(Utf8JsonWriter writer, CheckpointPayloadPowers value)
    {
        writer.WriteStartArray("powers");
        foreach (CheckpointPayloadPower power in value.Powers)
        {
            writer.WriteStartObject();
            writer.WriteNumber("amount", power.Amount);
            writer.WriteString("owner", power.Owner);
            writer.WriteString("power_id", power.PowerId);
            writer.WriteEndObject();
        }
        writer.WriteEndArray();
    }

    private static void WriteEnemies(Utf8JsonWriter writer, CheckpointPayloadEnemiesAndIntents value)
    {
        writer.WriteStartArray("enemies");
        foreach (CheckpointPayloadEnemy enemy in value.Enemies)
        {
            writer.WriteStartObject();
            writer.WriteNumber("block", enemy.Block);
            writer.WriteNumber("current_hp", enemy.CurrentHp);
            writer.WriteString("enemy_id", enemy.EnemyId);
            writer.WriteStartArray("intents");
            foreach (CheckpointPayloadIntent intent in enemy.Intents)
            {
                writer.WriteStartObject();
                writer.WriteString("intent_id", intent.IntentId);
                writer.WriteNumber("value", intent.Value);
                writer.WriteEndObject();
            }
            writer.WriteEndArray();
            writer.WriteNumber("max_hp", enemy.MaxHp);
            writer.WriteEndObject();
        }
        writer.WriteEndArray();
    }

    private static void WritePendingEffects(
        Utf8JsonWriter writer, CheckpointPayloadPendingEffects value)
    {
        writer.WriteStartArray("external_inputs");
        foreach (CheckpointPayloadExternalInput input in value.ExternalInputs)
        {
            writer.WriteStartObject();
            writer.WriteString("input_id", input.InputId);
            writer.WriteString("mode", input.Mode);
            writer.WriteEndObject();
        }
        writer.WriteEndArray();
        Strings(writer, "pending_effects", value.PendingEffects);
    }

    private static void Strings(Utf8JsonWriter writer, string name, IReadOnlyList<string> values)
    {
        writer.WriteStartArray(name);
        foreach (string value in values)
            writer.WriteStringValue(value);
        writer.WriteEndArray();
    }

    private static void UInt64(Utf8JsonWriter writer, ulong value)
    {
        writer.WriteStartObject();
        writer.WriteString("kind", "uint64");
        writer.WriteString("value", value.ToString(CultureInfo.InvariantCulture));
        writer.WriteEndObject();
    }
}
