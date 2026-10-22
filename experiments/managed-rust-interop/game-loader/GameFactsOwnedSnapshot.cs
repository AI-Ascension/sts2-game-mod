// SPDX-License-Identifier: MIT

using System.Collections.Generic;
using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    // ABI translation only. Rust selects rules, admits Core signatures and maps availability.
    private static Dictionary<string, object?> GameFactsOwnedSnapshot(RuntimeContext context,
        LiveCardBindingAssociation association, LiveCardCapturedSnapshot snapshot) =>
        new Dictionary<string, object?>
        {
            ["scope"] = new
            {
                instance_id = association.InstanceId,
                run_id = association.RunId,
                authority_epoch = association.AuthorityEpoch,
                content_manifest_id = snapshot.ContentManifest,
                locale = context.Locale
            },
            ["mode_id"] = snapshot.ModeId,
            ["catalog_generation"] = association.CatalogGeneration,
            ["snapshot_id"] = snapshot.SnapshotId,
            ["epoch"] = snapshot.Epoch,
            ["state_generation"] = snapshot.StateGeneration,
            ["player_energy"] = GameFactsField(snapshot.PlayerEnergy),
            ["cards"] = snapshot.Cards.Select(card => new
            {
                instance_id = card.InstanceId,
                definition_id = card.DefinitionId.Value,
                definition_manifest = card.DefinitionManifest.Value,
                resolved_cost = GameFactsField(card.ResolvedCost),
                fixed_energy_cost = GameFactsField(card.FixedEnergyCost)
            }).ToArray()
        };

    private static Dictionary<string, object?> GameFactsField<T>(LiveCardField<T> field) =>
        new Dictionary<string, object?>
        {
            ["status"] = field.Status switch
            {
                LiveCardFieldStatus.Available => "available",
                LiveCardFieldStatus.NotObserved => "not_observed",
                LiveCardFieldStatus.Unsupported => "unsupported",
                LiveCardFieldStatus.Unknown => "unknown",
                LiveCardFieldStatus.Failed => "failed",
                _ => "stale"
            },
            ["value"] = field.Status == LiveCardFieldStatus.Available ? field.Value : null
        };
}
