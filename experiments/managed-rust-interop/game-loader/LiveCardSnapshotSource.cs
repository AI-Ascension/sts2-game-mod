// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Availability carried by one owner-local live-card field. A missing host value is never
/// represented by an empty string, zero, or an empty collection.
/// </summary>
internal enum LiveCardFieldStatus
{
    Available,
    NotObserved,
    Unsupported,
    Unknown,
    Failed,
    Stale
}

internal readonly record struct LiveCardField<T>(LiveCardFieldStatus Status, T Value)
{
    internal static LiveCardField<T> Available(T value) =>
        new(LiveCardFieldStatus.Available, value);

    internal static LiveCardField<T> NotObserved() =>
        new(LiveCardFieldStatus.NotObserved, default!);

    internal static LiveCardField<T> Unsupported() =>
        new(LiveCardFieldStatus.Unsupported, default!);

    internal static LiveCardField<T> Unknown() =>
        new(LiveCardFieldStatus.Unknown, default!);

    internal static LiveCardField<T> Failed() =>
        new(LiveCardFieldStatus.Failed, default!);
}

internal enum LiveCardZone
{
    Hand,
    Deck,
    Discard,
    Exhaust,
    Selector
}

internal readonly record struct LiveCardLocation(LiveCardZone Zone, int Position);

/// <summary>
/// Owned values copied from one host-thread CardModel before the object is released.
/// HostCard is used only as an in-thread reference key in the source fence and registry.
/// </summary>
internal sealed record LiveCardCaptureInput(
    object HostCard,
    LiveCardField<string> DefinitionId,
    LiveCardField<string> DefinitionManifest,
    LiveCardField<string> OwnerId,
    LiveCardField<LiveCardLocation> Location,
    LiveCardField<ushort> UpgradeCount,
    LiveCardField<string> UpgradeVariant,
    LiveCardField<string> UpgradePath,
    LiveCardField<string> Title,
    LiveCardField<bool> Upgraded,
    LiveCardField<int> ResolvedCost,
    LiveCardField<int> BaseCost,
    LiveCardField<int> EffectiveCost,
    LiveCardField<IReadOnlyList<string>> Modifiers,
    LiveCardField<IReadOnlyList<string>> Flags,
    LiveCardField<IReadOnlyDictionary<string, string>> EffectParameters)
{
    internal LiveCardField<int> RawResolvedCost { get; init; } = LiveCardField<int>.NotObserved();
    internal LiveCardField<bool> HasEnergyCostX { get; init; } = LiveCardField<bool>.NotObserved();
}

/// <summary>Immutable source-owned card record returned by the host-thread capture.</summary>
internal sealed record LiveCardCapturedCard(
    string InstanceId,
    LiveCardField<string> DefinitionId,
    LiveCardField<string> DefinitionManifest,
    LiveCardField<string> OwnerId,
    LiveCardField<LiveCardLocation> Location,
    LiveCardField<ushort> UpgradeCount,
    LiveCardField<string> UpgradeVariant,
    LiveCardField<string> UpgradePath,
    LiveCardField<string> Title,
    LiveCardField<bool> Upgraded,
    LiveCardField<int> ResolvedCost,
    LiveCardField<int> BaseCost,
    LiveCardField<int> EffectiveCost,
    LiveCardField<IReadOnlyList<string>> Modifiers,
    LiveCardField<IReadOnlyList<string>> Flags,
    LiveCardField<IReadOnlyDictionary<string, string>> EffectParameters);

/// <summary>
/// Complete owned witness copied from the canonical content-manifest producer.
/// </summary>
internal sealed record NativeContentManifestWitness(
    ulong CatalogGeneration,
    string AdapterCompatibility,
    string ContentSetRevision,
    string LocalizedTextRevision,
    string InventoryRevision)
{
    internal static NativeContentManifestWitness FromManifest(System.Text.Json.JsonElement manifest) =>
        new(manifest.GetProperty("catalog_generation").GetUInt64(),
            manifest.GetProperty("adapter_compatibility").GetString()!,
            manifest.GetProperty("content_set_revision").GetString()!,
            manifest.GetProperty("localized_text_revision").GetString()!,
            manifest.GetProperty("inventory_revision").GetString()!);
}

/// <summary>
/// One immutable owner-local read fence. Run and snapshot IDs are source-owned lifecycle
/// identities, not native global generations.
/// </summary>
internal sealed record LiveCardCapturedSnapshot(
    string InstanceId,
    string ContentManifest,
    string SourceIncarnation,
    string RunId,
    string SnapshotId,
    ulong Epoch,
    ulong StateGeneration,
    IReadOnlyList<LiveCardCapturedCard> Cards,
    bool Available,
    string? UnavailableReason)
{
    internal LiveCardField<int> Energy { get; init; } = LiveCardField<int>.NotObserved();
    internal string ModeId { get; init; } = string.Empty;
    internal string Locale { get; init; } = string.Empty;
    internal NativeContentManifestWitness? ManifestWitness { get; init; }

    internal static LiveCardCapturedSnapshot Unavailable(string reason) =>
        new(string.Empty, string.Empty, string.Empty, string.Empty, string.Empty, 0, 0,
            Array.Empty<LiveCardCapturedCard>(), false, reason);
}
