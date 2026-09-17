// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Builds the owned JSON DTO consumed by the native content-manifest producer. Cards are the
/// first explicitly supported family because the exact host exposes their canonical semantic and
/// localized fields; every other registry family remains present as an unhandled inventory entry.
/// </summary>
internal static partial class NativeContentCatalogManifestSource
{
    private const string AdapterCompatibility = "sts2-game-mod-modeldb-card-v1";
    private const int MaxSemanticBytes = 16 * 1024;
    private const int MaxLocalizedTextBytes = 64 * 1024;

    internal static string CaptureJson()
    {
        NativeContentCatalogSnapshot known = NativeContentCatalogSnapshot.CaptureKnownInputs();
        NativeContentCatalogOwnerObservation owner =
            NativeContentCatalogOwnerObservation.Capture();
        IReadOnlyDictionary<string, int> independentCounts = ReadIndependentFamilyCounts();
        EnsureCountsMatch(owner.RegistryDefinitionCounts, independentCounts);
        Dictionary<(string Category, string Entry), CardModel> cards =
            ReadCardsByIdentity();
        var definitions = owner.Definitions.Select(definition =>
            definition.EntityKind == "card"
                ? CardDefinition(cards[(definition.EntityKind, definition.NamespacedId)])
                : GenericDefinition(definition)).ToArray();
        string[] firstSemantic = definitions.Select(JsonSerializer.Serialize).ToArray();
        Dictionary<(string Category, string Entry), CardModel> afterCards =
            ReadCardsByIdentity();
        NativeContentCatalogOwnerObservation after =
            NativeContentCatalogOwnerObservation.Capture();
        NativeContentCatalogSnapshot knownAfter =
            NativeContentCatalogSnapshot.CaptureKnownInputs();
        IReadOnlyDictionary<string, int> independentAfter = ReadIndependentFamilyCounts();
        EnsureCountsMatch(after.RegistryDefinitionCounts, independentAfter);
        if (after.Generation != owner.Generation
            || !SamePackages(known, knownAfter)
            || !SameCardReferences(cards, afterCards))
        {
            throw new InvalidOperationException("catalog changed during extraction");
        }
        EnsureCountsMatch(independentCounts, independentAfter);
        string[] secondSemantic = after.Definitions.Select(definition =>
                definition.EntityKind == "card"
                    ? CardDefinition(afterCards[(definition.EntityKind, definition.NamespacedId)])
                    : GenericDefinition(definition))
            .Select(JsonSerializer.Serialize)
            .ToArray();
        if (!firstSemantic.SequenceEqual(secondSemantic, StringComparer.Ordinal))
            throw new InvalidOperationException("catalog semantics changed during extraction");

        var packages = known.Packages.Select(package => new Dictionary<string, object?>
        {
            ["package_id"] = package.PackageId,
            ["package_version"] = package.PackageVersion,
            ["order"] = package.Order
        }).ToArray();
        var payload = new Dictionary<string, object?>
        {
            ["generation_before"] = owner.Generation,
            ["generation_after"] = after.Generation,
            ["game_build"] = known.GameBuild,
            ["locale"] = known.Locale,
            ["packages"] = packages,
            ["available_entity_kinds"] = owner.RegistryDefinitionCounts.Keys.OrderBy(
                key => key, StringComparer.Ordinal).ToArray(),
            ["registry_definition_counts"] = owner.RegistryDefinitionCounts,
            ["definitions"] = definitions,
            ["adapter_compatibility"] = AdapterCompatibility
        };
        return JsonSerializer.Serialize(payload);
    }

    private static Dictionary<string, object?> CardDefinition(CardModel card)
    {
        if (card is null)
            throw new InvalidOperationException("card registry contains null");
        string category = card.Id.Category;
        string entry = card.Id.Entry;
        if (!ContentManifestWireContract.ValidIdentity(category)
            || !ContentManifestWireContract.ValidIdentity(entry))
        {
            throw new InvalidOperationException("card identity is malformed");
        }

        var semantic = new Dictionary<string, object?>
        {
            ["id_category"] = category,
            ["id_entry"] = entry,
            ["is_canonical"] = card.IsCanonical,
            ["is_mutable"] = card.IsMutable,
            ["category_sorting_id"] = card.CategorySortingId,
            ["entry_sorting_id"] = card.EntrySortingId,
            ["type"] = card.Type.ToString(),
            ["rarity"] = card.Rarity.ToString(),
            ["target_type"] = card.TargetType.ToString(),
            ["canonical_energy_cost"] = card.CanonicalEnergyCost,
            ["has_energy_cost_x"] = card.HasEnergyCostX,
            ["base_replay_count"] = card.BaseReplayCount,
            ["base_star_cost"] = card.BaseStarCost,
            ["canonical_star_cost"] = card.CanonicalStarCost,
            ["has_star_cost_x"] = card.HasStarCostX,
            ["max_upgrade_level"] = card.MaxUpgradeLevel,
            ["is_upgradable"] = card.IsUpgradable,
            ["can_be_generated_in_combat"] = card.CanBeGeneratedInCombat,
            ["can_be_generated_by_modifiers"] = card.CanBeGeneratedByModifiers,
            ["canonical_vars"] = card.CanonicalVars?.ToString(),
            ["has_single_turn_retain"] = card.HasSingleTurnRetain,
            ["has_single_turn_sly"] = card.HasSingleTurnSly,
            ["gains_block"] = card.GainsBlock,
            ["orb_evoke_type"] = card.OrbEvokeType.ToString(),
            ["is_removable"] = card.IsRemovable,
            ["is_transformable"] = card.IsTransformable,
            ["should_show_in_card_library"] = card.ShouldShowInCardLibrary
        };
        string[] keywords = card.CanonicalKeywords.Select(value => value.ToString())
            .OrderBy(value => value, StringComparer.Ordinal).ToArray();
        string[] tags = card.Tags.Select(value => value.ToString())
            .OrderBy(value => value, StringComparer.Ordinal).ToArray();
        semantic["canonical_keywords"] = keywords;
        semantic["tags"] = tags;
        string semanticInputs = JsonSerializer.Serialize(semantic);
        if (System.Text.Encoding.UTF8.GetByteCount(semanticInputs) > MaxSemanticBytes)
            throw new InvalidOperationException("card semantic inputs exceed source bounds");

        string title = card.TitleLocString.GetRawText();
        string description = card.Description.GetRawText();
        if (System.Text.Encoding.UTF8.GetByteCount(title)
                > MaxLocalizedTextBytes
            || System.Text.Encoding.UTF8.GetByteCount(description)
                > MaxLocalizedTextBytes)
        {
            throw new InvalidOperationException("card localized text exceeds source bounds");
        }
        var localized = new Dictionary<string, object?>
        {
            ["title"] = title,
            ["description"] = description
        };
        string localizedText = JsonSerializer.Serialize(localized);
        if (System.Text.Encoding.UTF8.GetByteCount(localizedText) > MaxLocalizedTextBytes)
            throw new InvalidOperationException("card localized text exceeds source bounds");
        return new Dictionary<string, object?>
        {
            ["entity_kind"] = category,
            ["namespaced_id"] = entry,
            ["semantic_inputs"] = semanticInputs,
            ["localized_text"] = localizedText,
            ["origin"] = new Dictionary<string, object?>
            {
                ["package_id"] = null,
                ["package_version"] = null
            },
            ["override_chain"] = Array.Empty<string>()
        };
    }
}
