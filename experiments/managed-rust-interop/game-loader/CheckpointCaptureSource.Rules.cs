// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Owner-side coverage and bound rules from <c>checkpoint-payload-v1</c> (ADR 0057), applied
/// before any bytes exist: a required family reported unknown, a combat family present at a map
/// choice, an outstanding pending effect, an unbounded collection, or a malformed identifier is a
/// typed rejection, never a substituted value.
/// </summary>
internal sealed partial class CheckpointCaptureSource
{
    private const int MaxIdentifierLength = 128;

    private static bool TryValidate(
        CheckpointCaptureBoundary boundary,
        CheckpointPayloadFamilies families,
        [NotNullWhen(false)] out CheckpointCaptureRejection? rejection)
    {
        if (families is null)
        {
            rejection = Coverage(boundary, "families_missing");
            return false;
        }

        bool combat = boundary != CheckpointCaptureBoundary.SettledMapChoice;
        string? failure =
            Required(families.SeedAndRng, "seed_and_rng", ValidateSeedAndRng)
            ?? Required(families.RunConfiguration, "run_configuration", ValidateRunConfiguration)
            ?? Required(families.CampaignProgress, "campaign_progress", ValidateCampaignProgress)
            ?? Required(families.PlayerResources, "player_resources", ValidatePlayerResources)
            ?? Required(families.DeckAndPiles, "deck_and_piles", ValidateDeckAndPiles)
            ?? Required(families.CardInstances, "card_instances", ValidateCardInstances)
            ?? Required(families.Relics, "relics", ValidateRelics)
            ?? Required(families.Potions, "potions", ValidatePotions)
            ?? (combat
                ? Required(families.CombatTurn, "combat_turn", ValidateCombatTurn)
                : NotApplicable(families.CombatTurn, "combat_turn"))
            ?? (combat
                ? Required(families.Powers, "powers", ValidatePowers)
                : NotApplicable(families.Powers, "powers"))
            ?? (combat
                ? Required(families.EnemiesAndIntents, "enemies_and_intents", ValidateEnemies)
                : NotApplicable(families.EnemiesAndIntents, "enemies_and_intents"))
            ?? Required(families.PendingEffects, "pending_effects", ValidatePendingEffects)
            ?? ValidateCrossFamily(boundary, families);
        if (failure is null)
        {
            rejection = null;
            return true;
        }

        rejection = failure.StartsWith("pending_effects_outstanding", StringComparison.Ordinal)
            ? new CheckpointCaptureRejection(CheckpointCaptureRejectionKind.UnsafeBoundary,
                boundary, CheckpointSettlement.MidEffect, failure)
            : Coverage(boundary, failure);
        return false;
    }

    private static CheckpointCaptureRejection Coverage(
        CheckpointCaptureBoundary boundary, string detail) =>
        new(CheckpointCaptureRejectionKind.UnsupportedCoverage, boundary,
            CheckpointSettlement.Quiescent, detail);

    private static string? Required<T>(
        CheckpointPayloadFamily<T> family, string name, Func<T, string?> validate) where T : class
    {
        if (family is null || family.Coverage == CheckpointPayloadCoverage.Unknown)
            return "required_family_unknown:" + name;
        if (family.Coverage == CheckpointPayloadCoverage.NotApplicable || family.Value is null)
            return "family_not_applicable:" + name;
        string? detail = validate(family.Value);
        return detail is null ? null : detail + ":" + name;
    }

    private static string? NotApplicable<T>(CheckpointPayloadFamily<T> family, string name)
        where T : class =>
        family is null || family.Coverage != CheckpointPayloadCoverage.NotApplicable
            ? "family_not_applicable:" + name
            : null;

    private static string? ValidateSeedAndRng(CheckpointPayloadSeedAndRng value) =>
        Identifier(value.MasterSeed) ?? Identifier(value.DerivationVersion)
        ?? Bounded(value.Streams, 1, 32)
        ?? Each(value.Streams, stream => Identifier(stream.StreamId)
            ?? Identifier(stream.Algorithm) ?? Bounded(stream.StateWords, 1, 8));

    private static string? ValidateRunConfiguration(CheckpointPayloadRunConfiguration value) =>
        Identifier(value.Character) ?? Count(value.Ascension) ?? Identifier(value.Mode)
        ?? Bounded(value.ActSequence, 1, 8) ?? Each(value.ActSequence, Identifier)
        ?? Bounded(value.Modifiers, 0, 64) ?? Each(value.Modifiers, Identifier);

    private static string? ValidateCampaignProgress(CheckpointPayloadCampaignProgress value) =>
        Count(value.ActIndex) ?? Count(value.Floor) ?? Identifier(value.CurrentNode)
        ?? Identifier(value.RoomKind) ?? Bounded(value.VisitedNodes, 0, 256)
        ?? Each(value.VisitedNodes, Identifier);

    private static string? ValidatePlayerResources(CheckpointPayloadPlayerResources value) =>
        Count(value.CurrentHp) ?? Count(value.MaxHp) ?? Count(value.Gold)
        ?? Bounded(value.Keys, 0, 8) ?? Each(value.Keys, Identifier);

    private static string? ValidateDeckAndPiles(CheckpointPayloadDeckAndPiles value) =>
        Bounded(value.Deck, 0, 1024) ?? Each(value.Deck, Identifier)
        ?? Bounded(value.Piles, 0, 8)
        ?? Each(value.Piles, pile => Identifier(pile.PileId)
            ?? Bounded(pile.Cards, 0, 1024) ?? Each(pile.Cards, Identifier));

    private static string? ValidateCardInstances(CheckpointPayloadCardInstances value) =>
        Bounded(value.Cards, 0, 1024)
        ?? Each(value.Cards, card => Identifier(card.InstanceId) ?? Identifier(card.DefinitionId)
            ?? Count(card.UpgradeLevel) ?? Bounded(card.TemporaryValues, 0, 32)
            ?? Each(card.TemporaryValues, temporary =>
                Identifier(temporary.Key) ?? SafeInteger(temporary.Value)));

    private static string? ValidateRelics(CheckpointPayloadRelics value) =>
        Bounded(value.Relics, 0, 128)
        ?? Each(value.Relics, relic => Identifier(relic.RelicId) ?? SafeInteger(relic.Counter));

    private static string? ValidatePotions(CheckpointPayloadPotions value) =>
        Count(value.Capacity) ?? Bounded(value.Slots, 0, 16)
        ?? Each(value.Slots, slot => Count(slot.Index) ?? Identifier(slot.PotionId));

    private static string? ValidateCombatTurn(CheckpointPayloadCombatTurn value) =>
        Count(value.Turn) ?? Count(value.Energy) ?? Count(value.PlayerBlock);

    private static string? ValidatePowers(CheckpointPayloadPowers value) =>
        Bounded(value.Powers, 0, 256)
        ?? Each(value.Powers, power => Identifier(power.Owner) ?? Identifier(power.PowerId)
            ?? SafeInteger(power.Amount));

    private static string? ValidateEnemies(CheckpointPayloadEnemiesAndIntents value) =>
        Bounded(value.Enemies, 0, 16)
        ?? Each(value.Enemies, enemy => Identifier(enemy.EnemyId) ?? Count(enemy.CurrentHp)
            ?? Count(enemy.MaxHp) ?? Count(enemy.Block) ?? Bounded(enemy.Intents, 0, 8)
            ?? Each(enemy.Intents, intent =>
                Identifier(intent.IntentId) ?? SafeInteger(intent.Value)));

    private static string? ValidatePendingEffects(CheckpointPayloadPendingEffects value)
    {
        if (value.PendingEffects is null)
            return "bound_violated";
        if (value.PendingEffects.Count > 0)
            return "pending_effects_outstanding";
        return Bounded(value.ExternalInputs, 0, 32)
            ?? Each(value.ExternalInputs, input => Identifier(input.InputId)
                ?? (input.Mode is "controlled" or "absent" ? null : "external_input_mode_invalid"));
    }

    private static string? ValidateCrossFamily(
        CheckpointCaptureBoundary boundary, CheckpointPayloadFamilies families)
    {
        CheckpointPayloadSeedAndRng? rng = families.SeedAndRng.Value;
        if (rng is not null)
        {
            var streams = new HashSet<string>(StringComparer.Ordinal);
            foreach (CheckpointPayloadRngStream stream in rng.Streams)
                if (!streams.Add(stream.StreamId))
                    return "duplicate_stream_id:seed_and_rng";
        }

        var instances = new HashSet<string>(StringComparer.Ordinal);
        foreach (CheckpointPayloadCardInstance card in families.CardInstances.Value!.Cards)
            if (!instances.Add(card.InstanceId))
                return "duplicate_card_instance:card_instances";
        CheckpointPayloadDeckAndPiles piles = families.DeckAndPiles.Value!;
        foreach (string reference in piles.Deck)
            if (!instances.Contains(reference))
                return "unresolved_card_reference:deck_and_piles";
        foreach (CheckpointPayloadCardPile pile in piles.Piles)
            foreach (string reference in pile.Cards)
                if (!instances.Contains(reference))
                    return "unresolved_card_reference:deck_and_piles";

        if (boundary == CheckpointCaptureBoundary.SettledMapChoice)
            return null;
        long turn = families.CombatTurn.Value!.Turn;
        long minimum = boundary == CheckpointCaptureBoundary.LaterTurnCombat ? 2 : 1;
        return turn < minimum ? "turn_witness_mismatch:combat_turn" : null;
    }

    /// <summary>The schema's <c>identifier</c>: 1..128 characters of <c>[A-Za-z0-9._:/-]</c>.</summary>
    internal static string? Identifier(string? value)
    {
        if (value is null || value.Length == 0 || value.Length > MaxIdentifierLength)
            return "identifier_invalid";
        foreach (char character in value)
        {
            bool allowed = character is (>= 'A' and <= 'Z') or (>= 'a' and <= 'z')
                or (>= '0' and <= '9') or '.' or '_' or ':' or '/' or '-';
            if (!allowed)
                return "identifier_invalid";
        }
        return null;
    }

    private static string? Count(long value) =>
        value is < 0 or > 9007199254740991 ? "count_out_of_range" : null;

    private static string? SafeInteger(long value) =>
        value is < -9007199254740991 or > 9007199254740991 ? "integer_out_of_range" : null;

    private static string? Bounded<T>(IReadOnlyList<T>? list, int minimum, int maximum) =>
        list is null || list.Count < minimum || list.Count > maximum ? "bound_violated" : null;

    private static string? Each<T>(IReadOnlyList<T> list, Func<T, string?> validate)
    {
        foreach (T item in list)
        {
            string? detail = item is null ? "bound_violated" : validate(item);
            if (detail is not null)
                return detail;
        }
        return null;
    }
}
