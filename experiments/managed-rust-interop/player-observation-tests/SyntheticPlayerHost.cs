// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace MegaCrit.Sts2.Core.Combat
{
    internal sealed class PlayerCombatState
    {
        internal int Energy { get; init; } = 3;
        internal CardPile Hand { get; init; } = new();
        internal CardPile DiscardPile { get; init; } = new();
        internal CardPile ExhaustPile { get; init; } = new();
    }

    internal sealed class CardPile
    {
        internal IEnumerable<Entities.Cards.CardModel> Cards { get; init; } = Array.Empty<Entities.Cards.CardModel>();
    }
}

namespace MegaCrit.Sts2.Core.Entities.Creatures
{
    internal sealed class Creature
    {
        internal int CurrentHp { get; init; } = 50;
        internal int MaxHp { get; init; } = 50;
    }
}

namespace MegaCrit.Sts2.Core.Entities.Cards
{
    internal sealed class EnergyCost(int resolved)
    {
        internal int GetResolved() => resolved;
    }

    internal sealed class CardModel(string id, string title, string? description, int cost = 1)
    {
        internal string Id { get; } = id;
        public string Title { get; } = title;
        public string? Description { get; } = description;
        internal EnergyCost EnergyCost { get; } = new(cost);
        internal bool IsUpgraded { get; init; }
    }
}

namespace MegaCrit.Sts2.Core.Models
{
    internal sealed record ModelId(string Entry);

    internal enum TargetType { Self, AnyEnemy, AllEnemies, None, Mystery }

    internal sealed class RelicModel(string id, string? title, string? description)
    {
        internal ModelId Id { get; } = new(id);
        public string? Title { get; } = title;
        public string? DynamicDescription { get; } = description;
    }

    internal sealed class PotionModel(
        string id,
        int slot,
        string? title,
        string? description,
        TargetType? targetType,
        bool? usable)
    {
        internal ModelId Id { get; } = new(id);
        internal int SlotIndex { get; } = slot;
        public string? Title { get; } = title;
        public string? DynamicDescription { get; } = description;
        public TargetType? TargetType { get; } = targetType;
        internal bool? Usable { get; } = usable;
    }
}

namespace MegaCrit.Sts2.Core.Entities.Players
{
    using MegaCrit.Sts2.Core.Combat;
    using MegaCrit.Sts2.Core.Entities.Cards;
    using MegaCrit.Sts2.Core.Entities.Creatures;
    using MegaCrit.Sts2.Core.Models;

    internal sealed class Player
    {
        private readonly Dictionary<PotionModel, int> _slotIndices = new();
        internal Creature Creature { get; init; } = new();
        internal int Gold { get; init; } = 99;
        internal PlayerDeck Deck { get; init; } = new();
        internal CardPile DiscardPile { get; init; } = new();
        internal CardPile ExhaustPile { get; init; } = new();
        internal Func<IEnumerable<RelicModel>> ReadRelics { get; init; } = () => Array.Empty<RelicModel>();
        internal Func<IEnumerable<PotionModel>> ReadPotions { get; init; } = () => Array.Empty<PotionModel>();
        internal Func<IReadOnlyList<PotionModel?>> ReadPotionSlots { get; init; } = () => Array.Empty<PotionModel?>();
        internal Func<int> ReadMaxPotionCount { get; init; } = () => 0;
        internal IEnumerable<RelicModel> Relics => ReadRelics();
        internal IEnumerable<PotionModel> Potions => ReadPotions();
        internal IReadOnlyList<PotionModel?> PotionSlots => ReadPotionSlots();
        internal int MaxPotionCount => ReadMaxPotionCount();
        internal int GetPotionSlotIndex(PotionModel potion)
        {
            if (!_slotIndices.ContainsKey(potion)) _slotIndices.Add(potion, potion.SlotIndex);
            return _slotIndices[potion];
        }
    }

    internal sealed class PlayerDeck
    {
        internal IEnumerable<CardModel> Cards { get; init; } = Array.Empty<CardModel>();
    }
}
