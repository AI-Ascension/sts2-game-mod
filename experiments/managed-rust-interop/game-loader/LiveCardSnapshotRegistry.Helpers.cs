// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Security.Cryptography;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCardSnapshotRegistry
{
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
        || field.Value is not null
            && field.Value.Count <= MaxAuxiliaryEntries
            && AllAuxiliaryValuesBounded(field.Value);

    private static bool BoundedMap(
        LiveCardField<IReadOnlyDictionary<string, string>> field) =>
        field.Status != LiveCardFieldStatus.Available
        || field.Value is not null
            && field.Value.Count <= MaxAuxiliaryEntries
            && field.Value.All(pair => ValidIdentity(pair.Key)
                && pair.Value is not null
                && Encoding.UTF8.GetByteCount(pair.Value) <= MaxIdentityBytes
                && !pair.Value.Any(char.IsControl));

    private static bool AllAuxiliaryValuesBounded<T>(IReadOnlyList<T> values)
    {
        foreach (T value in values)
        {
            if (value is string text
                && (Encoding.UTF8.GetByteCount(text) > MaxIdentityBytes
                    || text.Any(char.IsControl)))
                return false;
        }
        return true;
    }

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
