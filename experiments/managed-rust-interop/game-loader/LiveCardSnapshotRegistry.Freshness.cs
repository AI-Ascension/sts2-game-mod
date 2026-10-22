// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCardSnapshotRegistry
{
    internal bool MatchesCurrent(
        LiveCardCapturedSnapshot snapshot,
        object runHandle,
        string runKey,
        IReadOnlyList<LiveCardCaptureInput> cards)
    {
        if (!IsCurrent(snapshot, snapshot.InstanceId, snapshot.ContentManifest)
            || !ReferenceEquals(_runHandle, runHandle) || _runKey != runKey
            || cards.Count != snapshot.Cards.Count)
            return false;
        for (int index = 0; index < cards.Count; index++)
        {
            LiveCardCaptureInput current = cards[index];
            LiveCardCapturedCard retained = snapshot.Cards[index];
            if (!_occurrences.TryGetValue(current.HostCard, out string? occurrence)
                || occurrence != retained.InstanceId
                || current.DefinitionId != retained.DefinitionId
                || current.DefinitionManifest != retained.DefinitionManifest
                || current.OwnerId != retained.OwnerId
                || current.Location != retained.Location
                || current.UpgradeCount != retained.UpgradeCount
                || current.UpgradeVariant != retained.UpgradeVariant
                || current.UpgradePath != retained.UpgradePath
                || current.Title != retained.Title
                || current.Upgraded != retained.Upgraded
                || current.ResolvedCost != retained.ResolvedCost
                || current.BaseCost != retained.BaseCost
                || current.EffectiveCost != retained.EffectiveCost
                || current.FixedEnergyCost != retained.FixedEnergyCost
                || !SameList(current.Modifiers, retained.Modifiers)
                || !SameList(current.Flags, retained.Flags)
                || !SameMap(current.EffectParameters, retained.EffectParameters))
                return false;
        }
        return true;
    }

    private static bool SameList(LiveCardField<IReadOnlyList<string>> left,
        LiveCardField<IReadOnlyList<string>> right) =>
        left.Status == right.Status && (left.Status != LiveCardFieldStatus.Available
            || (left.Value is not null && right.Value is not null && left.Value.SequenceEqual(right.Value)));

    private static bool SameMap(LiveCardField<IReadOnlyDictionary<string, string>> left,
        LiveCardField<IReadOnlyDictionary<string, string>> right) =>
        left.Status == right.Status && (left.Status != LiveCardFieldStatus.Available
            || (left.Value is not null && right.Value is not null && left.Value.Count == right.Value.Count
                && left.Value.All(pair => right.Value.TryGetValue(pair.Key, out string? value) && value == pair.Value)));
}
