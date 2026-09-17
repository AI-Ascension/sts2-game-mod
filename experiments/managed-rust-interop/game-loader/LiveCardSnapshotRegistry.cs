// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Globalization;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Security.Cryptography;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Bounded source-owned occurrence registry. CardModel reference identity is used only as an
/// in-thread lifetime handle; no object hash or host pointer is emitted as an ID.
/// </summary>
internal sealed class LiveCardSnapshotRegistry
{
    internal const int MaxCards = 512;
    internal const int MaxIdentityBytes = 256;
    internal const int MaxTitleBytes = 4 * 1024;
    internal const int MaxAuxiliaryEntries = 64;

    private readonly Dictionary<object, string> _occurrences =
        new(ReferenceEqualityComparer.Instance);
    private readonly string _sourceNonce = CreateSourceNonce();
    private object? _runHandle;
    private string? _runKey;
    private ulong _runIncarnation;
    private ulong _epoch;
    private ulong _nextOccurrence;
    private bool _invalidated;

    internal LiveCardCapturedSnapshot Capture(
        string instanceId,
        object runHandle,
        string runKey,
        string contentManifest,
        IReadOnlyList<LiveCardCaptureInput> cards)
    {
        if (!ValidIdentity(instanceId) || !ValidIdentity(contentManifest) || runHandle is null)
            return LiveCardCapturedSnapshot.Unavailable("identity_unavailable");
        if (string.IsNullOrEmpty(runKey))
            return LiveCardCapturedSnapshot.Unavailable("run_identity_unavailable");
        if (cards.Count > MaxCards)
            return LiveCardCapturedSnapshot.Unavailable("card_count_exceeded");

        if (_invalidated || _runHandle is null || !ReferenceEquals(_runHandle, runHandle)
            || _runKey is null || !string.Equals(_runKey, runKey,
                StringComparison.Ordinal))
        {
            RotateRun(runHandle, runKey);
        }

        var seen = new HashSet<object>(ReferenceEqualityComparer.Instance);
        foreach (LiveCardCaptureInput card in cards)
        {
            if (card.HostCard is null || !seen.Add(card.HostCard))
                return LiveCardCapturedSnapshot.Unavailable("duplicate_card_occurrence");
            if (!RequiredFieldAvailable(card.DefinitionId)
                || !RequiredFieldAvailable(card.DefinitionManifest)
                || !RequiredFieldAvailable(card.OwnerId)
                || !RequiredFieldAvailable(card.Location)
                || !RequiredFieldAvailable(card.UpgradeCount))
            {
                return LiveCardCapturedSnapshot.Unavailable("required_card_field_unavailable");
            }
            if (!ValidFieldIdentity(card.DefinitionId)
                || !ValidFieldIdentity(card.DefinitionManifest)
                || !ValidFieldIdentity(card.OwnerId)
                || !ValidFieldIdentity(card.UpgradeVariant)
                || !ValidFieldIdentity(card.UpgradePath)
                || !ValidFieldText(card.Title))
            {
                return LiveCardCapturedSnapshot.Unavailable("card_field_invalid");
            }
            if (!string.Equals(card.DefinitionManifest.Value, contentManifest,
                    StringComparison.Ordinal))
            {
                return LiveCardCapturedSnapshot.Unavailable("content_manifest_mismatch");
            }
            if (card.Location.Value.Position < 0 || card.Location.Value.Position >= MaxCards)
                return LiveCardCapturedSnapshot.Unavailable("card_position_invalid");
            if (!BoundedCollection(card.Modifiers) || !BoundedCollection(card.Flags)
                || !BoundedMap(card.EffectParameters))
            {
                return LiveCardCapturedSnapshot.Unavailable("card_field_bound_exceeded");
            }
        }

        var captured = new List<LiveCardCapturedCard>(cards.Count);
        foreach (LiveCardCaptureInput card in cards)
        {
            if (!_occurrences.TryGetValue(card.HostCard, out string? instance))
            {
                if (_nextOccurrence >= MaxCards)
                    return LiveCardCapturedSnapshot.Unavailable("occurrence_registry_exhausted");
                instance = $"card:{_sourceNonce}:"
                    + $"{_runIncarnation.ToString(CultureInfo.InvariantCulture)}:"
                    + $"{_nextOccurrence.ToString(CultureInfo.InvariantCulture)}";
                _nextOccurrence++;
                _occurrences.Add(card.HostCard, instance);
            }

            captured.Add(new LiveCardCapturedCard(
                instance,
                card.DefinitionId,
                card.DefinitionManifest,
                card.OwnerId,
                card.Location,
                card.UpgradeCount,
                card.UpgradeVariant,
                card.UpgradePath,
                card.Title,
                card.Upgraded,
                card.ResolvedCost,
                card.BaseCost,
                card.EffectiveCost,
                CopyListField(card.Modifiers),
                CopyListField(card.Flags),
                CopyMapField(card.EffectParameters)));
        }

        if (_epoch == ulong.MaxValue)
            return LiveCardCapturedSnapshot.Unavailable("snapshot_epoch_exhausted");
        _invalidated = false;
        _epoch++;
        string runId = $"run:{_sourceNonce}:"
            + $"{_runIncarnation.ToString(CultureInfo.InvariantCulture)}:"
            + Digest(_runKey!);
        string snapshotId = $"snapshot:{_sourceNonce}:"
            + $"{_runIncarnation.ToString(CultureInfo.InvariantCulture)}:"
            + $"{_epoch.ToString(CultureInfo.InvariantCulture)}";
        return new LiveCardCapturedSnapshot(
            instanceId,
            contentManifest,
            $"{_sourceNonce}:{_runIncarnation.ToString(CultureInfo.InvariantCulture)}",
            runId,
            snapshotId,
            _epoch,
            new ReadOnlyCollection<LiveCardCapturedCard>(captured),
            true,
            null);
    }

    /// <summary>
    /// Invalidates all host-object occurrence handles. The next active run gets a distinct
    /// source-owned incarnation even when the host repeats the same seed.
    /// </summary>
    internal void Invalidate()
    {
        _occurrences.Clear();
        _runHandle = null;
        _runKey = null;
        _runIncarnation++;
        _invalidated = true;
    }

    internal bool IsCurrent(
        LiveCardCapturedSnapshot snapshot,
        string instanceId,
        string contentManifest) =>
        snapshot.Available
        && _runKey is not null
        && string.Equals(snapshot.InstanceId, instanceId, StringComparison.Ordinal)
        && string.Equals(snapshot.ContentManifest, contentManifest, StringComparison.Ordinal)
        && string.Equals(snapshot.SourceIncarnation,
            $"{_sourceNonce}:{_runIncarnation.ToString(CultureInfo.InvariantCulture)}",
            StringComparison.Ordinal)
        && snapshot.Epoch == _epoch
        && string.Equals(snapshot.RunId,
            $"run:{_sourceNonce}:{_runIncarnation.ToString(CultureInfo.InvariantCulture)}:"
                + Digest(_runKey),
            StringComparison.Ordinal);

    private void RotateRun(object runHandle, string runKey)
    {
        _occurrences.Clear();
        _runHandle = runHandle;
        _runKey = runKey;
        _runIncarnation++;
        _nextOccurrence = 0;
        _invalidated = false;
    }

    private static bool RequiredFieldAvailable<T>(LiveCardField<T> field) =>
        field.Status == LiveCardFieldStatus.Available;

    private static bool ValidFieldIdentity(LiveCardField<string> field) =>
        field.Status != LiveCardFieldStatus.Available
            || field.Value is not null && ValidIdentity(field.Value);

    private static bool ValidFieldText(LiveCardField<string> field) =>
        field.Status != LiveCardFieldStatus.Available
            || field.Value is not null && Encoding.UTF8.GetByteCount(field.Value) <= MaxTitleBytes
                && !field.Value.Any(char.IsControl);

    private static LiveCardField<IReadOnlyList<string>> CopyListField(
        LiveCardField<IReadOnlyList<string>> field)
    {
        if (field.Status != LiveCardFieldStatus.Available || field.Value is null)
            return new(field.Status, default!);
        string[] values = field.Value.ToArray();
        return LiveCardField<IReadOnlyList<string>>.Available(
            new ReadOnlyCollection<string>(values));
    }

    private static LiveCardField<IReadOnlyDictionary<string, string>> CopyMapField(
        LiveCardField<IReadOnlyDictionary<string, string>> field)
    {
        if (field.Status != LiveCardFieldStatus.Available || field.Value is null)
            return new(field.Status, default!);
        var values = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach ((string key, string value) in field.Value)
        {
            if (!ValidIdentity(key) || value is null || value.Any(char.IsControl))
                return LiveCardField<IReadOnlyDictionary<string, string>>.Failed();
            values.Add(key, value);
        }
        return LiveCardField<IReadOnlyDictionary<string, string>>.Available(
            new ReadOnlyDictionary<string, string>(values));
    }

    private static bool ValidIdentity(string value) =>
        !string.IsNullOrEmpty(value)
        && Encoding.UTF8.GetByteCount(value) <= MaxIdentityBytes
        && value.All(character => char.IsAsciiLetterOrDigit(character)
            || character is '.' or ':' or '/' or '_' or '-');

    private static string Digest(string value) =>
        Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(value)))
            .ToLowerInvariant();

    private static bool BoundedCollection<T>(LiveCardField<IReadOnlyList<T>> field) =>
        field.Status != LiveCardFieldStatus.Available
        || field.Value is not null && field.Value.Count <= MaxAuxiliaryEntries;

    private static bool BoundedMap(
        LiveCardField<IReadOnlyDictionary<string, string>> field) =>
        field.Status != LiveCardFieldStatus.Available
        || field.Value is not null && field.Value.Count <= MaxAuxiliaryEntries;

    private static string CreateSourceNonce()
    {
        Span<byte> bytes = stackalloc byte[12];
        RandomNumberGenerator.Fill(bytes);
        return Convert.ToHexString(bytes).ToLowerInvariant();
    }

    private sealed class ReferenceEqualityComparer : IEqualityComparer<object>
    {
        internal static readonly ReferenceEqualityComparer Instance = new();

        public new bool Equals(object? left, object? right) => ReferenceEquals(left, right);

        public int GetHashCode(object value) => RuntimeHelpers.GetHashCode(value);
    }
}
