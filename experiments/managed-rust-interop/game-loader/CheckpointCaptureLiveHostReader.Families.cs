// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Context;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Family copy. A family is <c>captured</c> only when every payload field is a direct copy of a
/// public metadata-observed member; a family whose closure would need a private member or a
/// runtime-unverified interpretation is reported <c>unknown</c> (never guessed):
/// <c>seed_and_rng</c> (stream state words are private; the host serializes counters),
/// <c>card_instances</c> (temporary card values are private), <c>relics</c> (no public counter;
/// the hidden relic grab bag is private), <c>powers</c> unless no power exists (untyped internal
/// data), <c>enemies_and_intents</c> (hidden move state machine), and <c>pending_effects</c>
/// (the wall clock is present but neither <c>controlled</c> nor <c>absent</c>).
/// </summary>
internal sealed partial class CheckpointCaptureLiveHostReader
{
    public CheckpointPayloadFamilies ReadFamilies(CheckpointCaptureBoundary boundary)
    {
        RunManager run = RunManager.Instance;
        RunState? state = run.IsInProgress ? run.DebugOnlyGetState() : null;
        Player? player = state is null ? null : LocalContext.GetMe(state);
        bool combat = boundary != CheckpointCaptureBoundary.SettledMapChoice;
        if (state is null || player is null)
            return Unavailable(combat);

        PlayerCombatState? playerCombat = combat ? player.PlayerCombatState : null;
        var instances = new Dictionary<CardModel, string>(ReferenceEqualityComparer.Instance);
        var cards = new List<CheckpointPayloadCardInstance>();
        return new CheckpointPayloadFamilies(
            SeedAndRng: CheckpointPayloadFamily<CheckpointPayloadSeedAndRng>.Unknown(),
            RunConfiguration: RunConfiguration(state, player),
            CampaignProgress: CampaignProgress(state),
            PlayerResources: CheckpointPayloadFamily<CheckpointPayloadPlayerResources>.Captured(
                new CheckpointPayloadPlayerResources(player.Creature.CurrentHp,
                    player.Creature.MaxHp, player.Gold, Array.Empty<string>())),
            DeckAndPiles: DeckAndPiles(player, playerCombat, instances, cards),
            CardInstances: CheckpointPayloadFamily<CheckpointPayloadCardInstances>.Unknown(),
            Relics: CheckpointPayloadFamily<CheckpointPayloadRelics>.Unknown(),
            Potions: Potions(player),
            CombatTurn: combat && playerCombat is not null
                ? CheckpointPayloadFamily<CheckpointPayloadCombatTurn>.Captured(
                    new CheckpointPayloadCombatTurn(playerCombat.TurnNumber, playerCombat.Energy,
                        player.Creature.Block))
                : combat
                    ? CheckpointPayloadFamily<CheckpointPayloadCombatTurn>.Unknown()
                    : CheckpointPayloadFamily<CheckpointPayloadCombatTurn>.NotApplicable(),
            Powers: combat
                ? Powers(player)
                : CheckpointPayloadFamily<CheckpointPayloadPowers>.NotApplicable(),
            EnemiesAndIntents: combat
                ? CheckpointPayloadFamily<CheckpointPayloadEnemiesAndIntents>.Unknown()
                : CheckpointPayloadFamily<CheckpointPayloadEnemiesAndIntents>.NotApplicable(),
            PendingEffects: CheckpointPayloadFamily<CheckpointPayloadPendingEffects>.Unknown());
    }

    private static CheckpointPayloadFamilies Unavailable(bool combat) => new(
        CheckpointPayloadFamily<CheckpointPayloadSeedAndRng>.Unknown(),
        CheckpointPayloadFamily<CheckpointPayloadRunConfiguration>.Unknown(),
        CheckpointPayloadFamily<CheckpointPayloadCampaignProgress>.Unknown(),
        CheckpointPayloadFamily<CheckpointPayloadPlayerResources>.Unknown(),
        CheckpointPayloadFamily<CheckpointPayloadDeckAndPiles>.Unknown(),
        CheckpointPayloadFamily<CheckpointPayloadCardInstances>.Unknown(),
        CheckpointPayloadFamily<CheckpointPayloadRelics>.Unknown(),
        CheckpointPayloadFamily<CheckpointPayloadPotions>.Unknown(),
        combat ? CheckpointPayloadFamily<CheckpointPayloadCombatTurn>.Unknown()
            : CheckpointPayloadFamily<CheckpointPayloadCombatTurn>.NotApplicable(),
        combat ? CheckpointPayloadFamily<CheckpointPayloadPowers>.Unknown()
            : CheckpointPayloadFamily<CheckpointPayloadPowers>.NotApplicable(),
        combat ? CheckpointPayloadFamily<CheckpointPayloadEnemiesAndIntents>.Unknown()
            : CheckpointPayloadFamily<CheckpointPayloadEnemiesAndIntents>.NotApplicable(),
        CheckpointPayloadFamily<CheckpointPayloadPendingEffects>.Unknown());

    private static CheckpointPayloadFamily<CheckpointPayloadRunConfiguration> RunConfiguration(
        RunState state, Player player)
    {
        string? mode = state.GameMode == GameMode.Standard ? "standard"
            : state.GameMode == GameMode.Daily ? "daily"
            : state.GameMode == GameMode.Custom ? "custom" : null;
        var acts = new List<string>();
        foreach (ActModel act in state.Acts)
            acts.Add(act.Id.Entry);
        var modifiers = new List<string>();
        foreach (ModifierModel modifier in state.Modifiers)
            modifiers.Add(modifier.Id.Entry);
        string character = player.Character.Id.Entry;
        if (mode is null || !Identifiers(acts) || !Identifiers(modifiers)
            || CheckpointCaptureSource.Identifier(character) is not null)
            return CheckpointPayloadFamily<CheckpointPayloadRunConfiguration>.Unknown();
        return CheckpointPayloadFamily<CheckpointPayloadRunConfiguration>.Captured(
            new CheckpointPayloadRunConfiguration(character, state.AscensionLevel, mode, acts,
                modifiers));
    }

    private static CheckpointPayloadFamily<CheckpointPayloadCampaignProgress> CampaignProgress(
        RunState state)
    {
        if (state.CurrentMapCoord is not { } current)
            return CheckpointPayloadFamily<CheckpointPayloadCampaignProgress>.Unknown();
        var visited = new List<string>();
        foreach (MapCoord coordinate in state.VisitedMapCoords ?? Array.Empty<MapCoord>())
            visited.Add(NodeId(coordinate));
        string roomKind = state.CurrentRoom is { } room
            ? room.RoomType.ToString().ToLowerInvariant()
            : "unknown_until_entered";
        if (CheckpointCaptureSource.Identifier(roomKind) is not null)
            return CheckpointPayloadFamily<CheckpointPayloadCampaignProgress>.Unknown();
        return CheckpointPayloadFamily<CheckpointPayloadCampaignProgress>.Captured(
            new CheckpointPayloadCampaignProgress(state.CurrentActIndex, state.ActFloor,
                NodeId(current), roomKind, visited));
    }

    private static CheckpointPayloadFamily<CheckpointPayloadDeckAndPiles> DeckAndPiles(
        Player player,
        PlayerCombatState? playerCombat,
        Dictionary<CardModel, string> instances,
        List<CheckpointPayloadCardInstance> cards)
    {
        List<string> deck = References(player.Deck.Cards, instances, cards);
        var piles = new List<CheckpointPayloadCardPile>();
        if (playerCombat is not null)
        {
            piles.Add(new CheckpointPayloadCardPile("hand",
                References(playerCombat.Hand.Cards, instances, cards)));
            piles.Add(new CheckpointPayloadCardPile("draw",
                References(playerCombat.DrawPile.Cards, instances, cards)));
            piles.Add(new CheckpointPayloadCardPile("discard",
                References(playerCombat.DiscardPile.Cards, instances, cards)));
            piles.Add(new CheckpointPayloadCardPile("exhaust",
                References(playerCombat.ExhaustPile.Cards, instances, cards)));
            piles.Add(new CheckpointPayloadCardPile("play",
                References(playerCombat.PlayPile.Cards, instances, cards)));
        }
        foreach (CheckpointPayloadCardInstance card in cards)
            if (CheckpointCaptureSource.Identifier(card.DefinitionId) is not null)
                return CheckpointPayloadFamily<CheckpointPayloadDeckAndPiles>.Unknown();
        return CheckpointPayloadFamily<CheckpointPayloadDeckAndPiles>.Captured(
            new CheckpointPayloadDeckAndPiles(deck, piles));
    }

    private static List<string> References(
        IEnumerable<CardModel> hostCards,
        Dictionary<CardModel, string> instances,
        List<CheckpointPayloadCardInstance> cards)
    {
        var references = new List<string>();
        foreach (CardModel card in hostCards)
        {
            if (!instances.TryGetValue(card, out string? id))
            {
                id = "card:" + (instances.Count + 1).ToString(CultureInfo.InvariantCulture);
                instances.Add(card, id);
                cards.Add(new CheckpointPayloadCardInstance(id, card.Id.Entry,
                    card.CurrentUpgradeLevel, Array.Empty<CheckpointPayloadTemporaryValue>()));
            }
            references.Add(id);
        }
        return references;
    }

    private static CheckpointPayloadFamily<CheckpointPayloadPotions> Potions(Player player)
    {
        var slots = new List<CheckpointPayloadPotionSlot>();
        IReadOnlyList<PotionModel?> hostSlots = player.PotionSlots;
        for (int index = 0; index < hostSlots.Count; index++)
        {
            if (hostSlots[index] is not { } potion)
                continue;
            if (potion.IsQueued || CheckpointCaptureSource.Identifier(potion.Id.Entry) is not null)
                return CheckpointPayloadFamily<CheckpointPayloadPotions>.Unknown();
            slots.Add(new CheckpointPayloadPotionSlot(index, potion.Id.Entry));
        }
        return CheckpointPayloadFamily<CheckpointPayloadPotions>.Captured(
            new CheckpointPayloadPotions(player.MaxPotionCount, slots));
    }

    private static CheckpointPayloadFamily<CheckpointPayloadPowers> Powers(Player player)
    {
        int count = player.Creature.Powers.Count;
        if (CombatManager.Instance.DebugOnlyGetState() is { } combatState)
            foreach (Creature enemy in combatState.Enemies)
                count += enemy.Powers.Count;
        return count == 0
            ? CheckpointPayloadFamily<CheckpointPayloadPowers>.Captured(
                new CheckpointPayloadPowers(Array.Empty<CheckpointPayloadPower>()))
            : CheckpointPayloadFamily<CheckpointPayloadPowers>.Unknown();
    }

    private static string NodeId(MapCoord coordinate) =>
        "node:" + coordinate.row.ToString(CultureInfo.InvariantCulture) + ":"
        + coordinate.col.ToString(CultureInfo.InvariantCulture);

    private static bool Identifiers(List<string> values)
    {
        foreach (string value in values)
            if (CheckpointCaptureSource.Identifier(value) is not null)
                return false;
        return true;
    }
}
