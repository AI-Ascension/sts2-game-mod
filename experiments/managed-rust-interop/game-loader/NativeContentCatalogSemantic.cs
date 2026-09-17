// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Copies stable, owner-local model inputs for families that are not yet query-adapter handled.
/// Runtime ownership, visual resources, and mutable combat state are excluded explicitly.
/// </summary>
internal static partial class NativeContentCatalogManifestSourceHelpers
{
    private const int MaxSemanticBytes = 16 * 1024;
    private const int MaxCollectionItems = 512;
    private const int MaxValueBytes = 1024;
    private static readonly string[] RelicSemanticProperties =
    {
        "Rarity", "IsTradable", "IsAllowedInShops", "HasUponPickupEffect",
        "SpawnsPets", "IsStackable", "AddsPet", "ShowCounter", "CanonicalVars",
        "DynamicVars"
    };
    private static readonly string[] PotionSemanticProperties =
    {
        "Rarity", "Usage", "TargetType", "CanBeGeneratedInCombat",
        "CanonicalVars", "DynamicVars"
    };
    private static readonly string[] PowerSemanticProperties =
    {
        "Type", "IsInstanced", "IsVisible", "ShouldPlayVfx", "StackType",
        "AllowNegative", "CanonicalVars", "DynamicVars"
    };
    private static readonly Dictionary<string, string[]> AdditionalSemanticProperties =
        new Dictionary<string, string[]>(StringComparer.Ordinal)
        {
            ["CardPoolModel"] = new[]
                { "Title", "EnergyColorName", "IsColorless", "AllCardIds" },
            ["PotionPoolModel"] = new[]
                { "EnergyColorName", "AllPotionIds" },
            ["RelicPoolModel"] = new[] { "EnergyColorName" },
            ["CharacterModel"] = new[]
            {
                "StartingHp", "StartingGold", "MaxEnergy", "BaseOrbSlotCount",
                "ShouldAlwaysShowStarCounter", "Title", "CharacterSelectTitle"
            },
            ["EventModel"] = new[]
                { "IsDeterministic", "IsShared", "LayoutType", "Title", "InitialDescription" },
            ["AncientEventModel"] = new[]
                { "LayoutType", "Epithet", "InitialDescription", "HealedAmount" },
            ["MonsterModel"] = new[]
            {
                "MinInitialHp", "MaxInitialHp", "IsHealthBarVisible",
                "ShouldFadeAfterDeath", "ShouldDisappearFromDoom", "CanChangeScale",
                "TakeDamageSfxType", "Title"
            },
            ["EncounterModel"] = new[]
            {
                "IsWeak", "ShouldGiveRewards", "MinGoldReward", "MaxGoldReward",
                "IsDebugEncounter", "Tags", "FullyCenterPlayers", "HasScene",
                "HasBgm", "HasAmbientSfx", "Title"
            },
            ["OrbModel"] = new[]
                { "PassiveVal", "EvokeVal", "Title", "Description", "HasSmartDescription" },
            ["ActModel"] = new[] { "Title", "AmbientSfx", "BgMusicOptions", "MusicBankPaths" },
            ["BadgeModel"] = new[] { "ShouldReceiveCombatHooks" },
            ["AchievementModel"] = Array.Empty<string>(),
            ["ModifierModel"] = new[] { "ClearsPlayerDeck", "Title", "Description" },
            ["AfflictionModel"] = new[]
            {
                "HasExtraCardText", "HasCard", "CanAfflictUnplayableCards",
                "IsStackable", "Amount", "Title"
            },
            ["EnchantmentModel"] = new[]
            {
                "HasExtraCardText", "ShowAmount", "ShouldStartAtBottomOfDrawPile",
                "HasCard", "Amount", "IsStackable", "Status", "Title"
            }
        };

    internal static string SemanticInputs(NativeContentCatalogOwnerObservation.Definition definition)
    {
        var semantic = new Dictionary<string, object?>
        {
            ["id_category"] = definition.EntityKind,
            ["id_entry"] = definition.NamespacedId,
            ["runtime_type"] = definition.RuntimeType,
            ["category_type"] = definition.CategoryType,
            ["is_canonical"] = definition.IsCanonical,
            ["is_mutable"] = definition.IsMutable,
            ["category_sorting_id"] = definition.CategorySortingId,
            ["entry_sorting_id"] = definition.EntrySortingId
        };
        if (definition.Model is RelicModel)
        {
            semantic["semantic_scope"] = "typed-relic-v1";
            AddProperties(semantic, definition.Model, RelicSemanticProperties);
        }
        else if (definition.Model is PotionModel)
        {
            semantic["semantic_scope"] = "typed-potion-v1";
            AddProperties(semantic, definition.Model, PotionSemanticProperties);
        }
        else if (definition.Model is PowerModel)
        {
            semantic["semantic_scope"] = "typed-power-v1";
            AddProperties(semantic, definition.Model, PowerSemanticProperties);
        }
        else if (definition.Model is BadgeModel)
        {
            semantic["semantic_scope"] = "typed-badge-v1";
            AddProperties(semantic, definition.Model, AdditionalSemanticProperties["BadgeModel"]);
        }
        else if (TryAdditionalSemanticProperties(
                     definition, out string[] properties))
        {
            semantic["semantic_scope"] =
                $"typed-{FamilyTypeName(definition).Replace("Model", string.Empty,
                    StringComparison.Ordinal).ToLowerInvariant()}-v1";
            AddProperties(semantic, definition.Model, properties);
        }
        else
        {
            throw new InvalidOperationException(
                $"semantic mapping unavailable for {definition.RuntimeType}");
        }
        string result = JsonSerializer.Serialize(semantic);
        if (System.Text.Encoding.UTF8.GetByteCount(result) > MaxSemanticBytes)
            throw new InvalidOperationException("model semantic inputs exceed source bounds");
        return result;
    }

    private static bool TryAdditionalSemanticProperties(
        NativeContentCatalogOwnerObservation.Definition definition,
        out string[] properties)
    {
        string familyType = FamilyTypeName(definition);
        if (AdditionalSemanticProperties.TryGetValue(
                familyType, out string[]? familyProperties))
        {
            properties = familyProperties;
            return true;
        }
        if (AdditionalSemanticProperties.TryGetValue(
                definition.Model.GetType().Name, out string[]? concreteProperties))
        {
            properties = concreteProperties;
            return true;
        }
        properties = Array.Empty<string>();
        return false;
    }

    private static string FamilyTypeName(
        NativeContentCatalogOwnerObservation.Definition definition)
    {
        for (Type? type = definition.Model.GetType();
             type is not null;
             type = type.BaseType)
        {
            if (AdditionalSemanticProperties.ContainsKey(type.Name))
                return type.Name;
        }
        int separator = definition.CategoryType.LastIndexOf('.');
        return separator >= 0
            ? definition.CategoryType[(separator + 1)..]
            : definition.CategoryType;
    }

    internal static string? LocalizedText(NativeContentCatalogOwnerObservation.Definition definition)
    {
        string[] names = definition.Model switch
        {
            RelicModel => new[] { "Title", "Description", "Flavor", "DynamicDescription" },
            PotionModel => new[] { "Title", "Description", "DynamicDescription" },
            PowerModel => new[] { "Title", "Description", "SmartDescription" },
            _ => Array.Empty<string>()
        };
        if (names.Length == 0)
            return null;
        var values = new Dictionary<string, object?>();
        foreach (string name in names)
            values[name] = LocalizedValue(ReadProperty(definition.Model, name));
        string result = JsonSerializer.Serialize(values);
        if (System.Text.Encoding.UTF8.GetByteCount(result) > 64 * 1024)
            throw new InvalidOperationException("model localized text exceeds source bounds");
        return result;
    }

}
