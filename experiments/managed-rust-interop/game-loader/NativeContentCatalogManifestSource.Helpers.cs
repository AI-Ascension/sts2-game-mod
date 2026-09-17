// SPDX-License-Identifier: MIT

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Text.Json;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class NativeContentCatalogManifestSource
{
    internal static readonly Dictionary<string, string> KnownFamilyCategories =
        new(StringComparer.Ordinal)
        {
            ["AllCards"] = "card",
            ["AllCardPools"] = "card_pool",
            ["AllSharedCardPools"] = "card_pool",
            ["AllCharacterCardPools"] = "card_pool",
            ["AllCharacters"] = "character",
            ["AllSharedEvents"] = "event",
            ["AllAncients"] = "ancient",
            ["AllSharedAncients"] = "ancient",
            ["AllEvents"] = "event",
            ["Monsters"] = "monster",
            ["AllEncounters"] = "encounter",
            ["AllPotions"] = "potion",
            ["AllPotionPools"] = "potion_pool",
            ["AllCharacterPotionPools"] = "potion_pool",
            ["AllCharacterRelicPools"] = "relic_pool",
            ["AllSharedPotionPools"] = "potion_pool",
            ["AllPowers"] = "power",
            ["AllRelics"] = "relic",
            ["AllRelicPools"] = "relic_pool",
            ["CharacterRelicPools"] = "relic_pool",
            ["AllSharedRelicPools"] = "relic_pool",
            ["Orbs"] = "orb",
            ["Acts"] = "act",
            ["BadgeModels"] = "badge",
            ["Achievements"] = "achievement",
            ["GoodModifiers"] = "modifier",
            ["BadModifiers"] = "modifier",
            ["DebugAfflictions"] = "affliction",
            ["DebugEnchantments"] = "enchantment"
        };

    private static Dictionary<(string Category, string Entry), CardModel> ReadCardsByIdentity()
    {
        CardModel[] cards = ModelDb.AllCards.ToArray();
        var result = new Dictionary<(string Category, string Entry), CardModel>();
        foreach (CardModel card in cards)
        {
            if (card is null || !result.TryAdd((card.Id.Category, card.Id.Entry), card))
                throw new InvalidOperationException("card family identity is malformed");
        }
        return result;
    }

    private static bool SameCardReferences(
        Dictionary<(string Category, string Entry), CardModel> left,
        Dictionary<(string Category, string Entry), CardModel> right) =>
        left.Count == right.Count
        && left.All(pair => right.TryGetValue(pair.Key, out CardModel? value)
            && ReferenceEquals(pair.Value, value));

    private static bool SamePackages(
        NativeContentCatalogSnapshot left,
        NativeContentCatalogSnapshot right) =>
        left.GameBuild == right.GameBuild
        && left.Locale == right.Locale
        && left.Packages.SequenceEqual(right.Packages);

    private static void EnsureCountsMatch(
        IReadOnlyDictionary<string, int> registry,
        IReadOnlyDictionary<string, int> independent)
    {
        if (registry.Count != independent.Count
            || registry.Any(pair => !independent.TryGetValue(pair.Key, out int count)
                || count != pair.Value))
        {
            throw new InvalidOperationException("independent family counts do not match");
        }
    }

    private static Dictionary<string, int> ReadIndependentFamilyCounts()
    {
        string[] properties =
        {
            "AllCards", "AllCardPools", "AllSharedCardPools", "AllCharacterCardPools",
            "AllCharacters", "AllSharedEvents", "AllAncients", "AllSharedAncients",
            "AllEvents", "Monsters", "AllEncounters", "AllPotions", "AllPotionPools",
            "AllCharacterPotionPools", "AllCharacterRelicPools", "AllSharedPotionPools",
            "AllPowers", "AllRelics", "AllRelicPools", "CharacterRelicPools",
            "AllSharedRelicPools", "Orbs", "Acts", "BadgeModels", "Achievements", "GoodModifiers",
            "BadModifiers", "DebugAfflictions", "DebugEnchantments"
        };
        var seen = new HashSet<AbstractModel>(ReferenceComparer.Instance);
        var counts = KnownFamilyCategories.Values
            .Distinct(StringComparer.Ordinal)
            .ToDictionary(value => value, _ => 0, StringComparer.Ordinal);
        foreach (string name in properties)
        {
            PropertyInfo? property = typeof(ModelDb).GetProperty(
                name, BindingFlags.Public | BindingFlags.Static);
            if (property is null)
                throw new InvalidOperationException($"host ModelDb family property unavailable: {name}");
            if (property.GetValue(null) is not IEnumerable values)
                throw new InvalidOperationException($"host ModelDb family property is not enumerable: {name}");
            foreach (object value in values)
            {
                if (value is not AbstractModel model || !seen.Add(model))
                    continue;
                string category = model.Id.Category;
                if (!ContentManifestWireContract.ValidIdentity(category))
                    throw new InvalidOperationException("family identity is malformed");
                counts[category] = counts.TryGetValue(category, out int count)
                    ? checked(count + 1)
                    : 1;
            }
        }
        return counts;
    }

    private static Dictionary<string, object?> GenericDefinition(
        NativeContentCatalogOwnerObservation.Definition definition)
    {
        string semantic = NativeContentCatalogManifestSourceHelpers.SemanticInputs(definition);
        if (System.Text.Encoding.UTF8.GetByteCount(semantic) > MaxSemanticBytes)
            throw new InvalidOperationException("model semantic inputs exceed source bounds");
        return new Dictionary<string, object?>
        {
            ["entity_kind"] = definition.EntityKind,
            ["namespaced_id"] = definition.NamespacedId,
            ["semantic_inputs"] = semantic,
            ["localized_text"] = NativeContentCatalogManifestSourceHelpers.LocalizedText(definition),
            ["origin"] = new Dictionary<string, object?>
            {
                ["package_id"] = null,
                ["package_version"] = null
            },
            ["override_chain"] = Array.Empty<string>()
        };
    }

    private sealed class ReferenceComparer : IEqualityComparer<AbstractModel>
    {
        internal static readonly ReferenceComparer Instance = new();
        public bool Equals(AbstractModel? left, AbstractModel? right) =>
            ReferenceEquals(left, right);
        public int GetHashCode(AbstractModel value) =>
            System.Runtime.CompilerServices.RuntimeHelpers.GetHashCode(value);
    }
}
