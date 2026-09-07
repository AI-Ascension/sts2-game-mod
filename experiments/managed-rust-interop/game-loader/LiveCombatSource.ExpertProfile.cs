// SPDX-License-Identifier: MIT

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Godot;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    /// <summary>Opt-in projection entry point; the existing Runtime-v3 route is unchanged.</summary>
    internal RuntimeV4ExpertGameplayObservation ObserveExpert()
    {
        RequireThread();
        RuntimeV3GameplayObservation observation = Observe();
        Player? player = CurrentPlayer();
        PlayerCombatState? combat = player?.PlayerCombatState;
        IReadOnlyList<RuntimeV4ExpertGameplayAction> actions = ExpertActions(observation, player, combat);
        return new RuntimeV4ExpertGameplayObservation(
            observation.StateId,
            observation.Generation,
            observation.VisibleSeed,
            ProjectRun(observation, player),
            ProjectPlayerExpert(observation.Player, player, combat),
            ProjectStateExpert(observation),
            actions);
    }

    internal bool TrySerializeExpert(out string json, out string error)
    {
        RuntimeV4ExpertGameplayObservation observation = ObserveExpert();
        return RuntimeV4ExpertGameplayCodec.TrySerialize(observation, out json, out error);
    }

    private static RuntimeV4ExpertGameplayRun ProjectRun(
        RuntimeV3GameplayObservation observation,
        Player? player)
    {
        RunState? run = MegaCrit.Sts2.Core.Runs.RunManager.Instance.DebugOnlyGetState();
        byte? act = run is null ? null : (byte)Math.Clamp(run.CurrentActIndex, 0, 255);
        string? character = ReadNestedIdentity(player, "Character", "Id");
        return new RuntimeV4ExpertGameplayRun(character, act, observation.NodeId);
    }

    private RuntimeV4ExpertGameplayPlayer ProjectPlayerExpert(
        RuntimeV3GameplayPlayer baseline,
        Player? player,
        PlayerCombatState? combat)
    {
        RuntimeV4ExpertGameplayPlayer result = new(
            player is null ? baseline.Hp : U16(player.Creature.CurrentHp),
            player is null ? baseline.MaxHp : U16(player.Creature.MaxHp),
            PublicU16(player?.Creature, "Block"),
            player is null ? baseline.Energy : (byte)Math.Clamp(combat?.Energy ?? 0, 0, 255),
            player is null ? baseline.Gold : (uint)Math.Max(player.Gold, 0))
        {
            Hand = combat is null ? V3Cards(baseline.Hand) : Cards(combat.Hand.Cards),
            Deck = player is null ? V3Cards(baseline.Deck) : Cards(player.Deck.Cards),
            Discard = combat is null ? V3Cards(baseline.Discard) : Cards(combat.DiscardPile.Cards),
            Exhaust = combat is null ? V3Cards(baseline.Exhaust) : Cards(combat.ExhaustPile.Cards),
            Powers = PublicStatuses(player?.Creature, "Powers"),
            Statuses = PublicStatuses(player?.Creature, "Statuses"),
            Relics = Relics(player),
            Potions = Potions(player),
            PotionSlots = OccupiedPotionCount(player),
            MaxPotionSlots = MaxPotionCount(player)
        };
        return result;
    }

    private RuntimeV4ExpertGameplayCard[]? Cards(
        IEnumerable<CardModel>? cards) => cards?.Select(ProjectCard).ToArray();

    private RuntimeV4ExpertGameplayCard ProjectCard(CardModel card) => new(
        CardId(card),
        card.Title,
        Cost(card),
        card.IsUpgraded,
        PublicText(card, "Type"),
        PublicText(card, "Rarity"),
        card.TargetType.ToString(),
        PublicText(card, "Description"));

    private static RuntimeV4ExpertGameplayCard[] V3Cards(
        IReadOnlyList<RuntimeV3GameplayCard> cards) => cards.Select(card =>
            new RuntimeV4ExpertGameplayCard(
                card.CardId,
                card.Name,
                card.Cost,
                card.Upgraded,
                null,
                null,
                null,
                null)).ToArray();

    private static byte? Cost(CardModel card)
    {
        int cost = card.EnergyCost.GetResolved();
        return cost is >= 0 and <= byte.MaxValue ? (byte)cost : null;
    }

    private static RuntimeV4ExpertGameplayRelic[]? Relics(Player? player)
    {
        if (player is null) return null;
        try
        {
            return player.Relics.Select(relic => new RuntimeV4ExpertGameplayRelic(
                relic.Id.Entry, relic.Title.GetFormattedText())).ToArray();
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static RuntimeV4ExpertGameplayPotion[]? Potions(Player? player)
    {
        if (player is null) return null;
        try
        {
            var result = new List<RuntimeV4ExpertGameplayPotion>();
            foreach (PotionModel potion in player.Potions)
            {
                int slot = player.GetPotionSlotIndex(potion);
                if (slot < 0 || slot > byte.MaxValue) return null;
                result.Add(new RuntimeV4ExpertGameplayPotion(
                    PotionId(potion, slot),
                    potion.Title.GetFormattedText(),
                    (byte)slot,
                    PotionUsable(player, potion, slot),
                    PotionTargetMode(potion)));
            }
            return result.ToArray();
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static RuntimeV4ExpertGameplayState ProjectStateExpert(
        RuntimeV3GameplayObservation observation)
    {
        RuntimeV4ExpertGameplayState state = new(observation.State switch
        {
            RuntimeV3GameplayState.Setup => "setup",
            RuntimeV3GameplayState.Map => "map",
            RuntimeV3GameplayState.Combat => "combat",
            RuntimeV3GameplayState.Reward => "reward",
            RuntimeV3GameplayState.Shop => "shop",
            RuntimeV3GameplayState.Event => "event",
            RuntimeV3GameplayState.Rest => "rest",
            RuntimeV3GameplayState.Selection => "selection",
            RuntimeV3GameplayState.Victory => "victory",
            RuntimeV3GameplayState.Defeat => "defeat",
            RuntimeV3GameplayState.Recovery => "recovery",
            _ => "recovery"
        });
        return observation.State switch
        {
            RuntimeV3GameplayState.Setup => state with { Characters = observation.StateValues },
            RuntimeV3GameplayState.Map => state with
            {
                CurrentNodeId = CurrentMapNodeId() ?? observation.NodeId,
                Nodes = VisibleMapNodes(),
                Edges = VisibleMapEdges(),
                Options = observation.StateValues
            },
            RuntimeV3GameplayState.Combat => state with
            {
                TurnIndex = observation.TurnIndex,
                Enemies = observation.Enemies.Select(ProjectEnemy).ToArray()
            },
            RuntimeV3GameplayState.Reward or RuntimeV3GameplayState.Event
                or RuntimeV3GameplayState.Rest or RuntimeV3GameplayState.Selection => state with
            {
                Choices = observation.StateValues.Select(value =>
                    new RuntimeV4ExpertGameplayChoice(value, value, state.Kind, null)).ToArray()
            },
            RuntimeV3GameplayState.Shop => state with
            {
                Items = observation.ShopItems.Select(item =>
                    new RuntimeV4ExpertGameplayShopItem(item.ItemId, item.Name, "unknown", item.Price)).ToArray()
            },
            RuntimeV3GameplayState.Defeat => state with
            {
                Reason = observation.StateValues.Count == 0 ? null : observation.StateValues[0]
            },
            RuntimeV3GameplayState.Recovery => state with
            {
                Code = observation.StateValues.Count == 0 ? "recovery" : observation.StateValues[0]
            },
            _ => state
        };
    }

}
