// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Reflection;
using System.Security.Cryptography;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Copies the exact host ModelDb owner registry into bounded values. The generation is a
/// source-owned before/after identity fingerprint over the actual owner references and canonical
/// registry fields; it is an invalidation witness, not a native counter.
/// </summary>
internal sealed class NativeContentCatalogOwnerObservation
{
    private const string RegistryFieldName = "_contentById";
    private const int MaxEntries = 16384;
    private const int MaxIdentityBytes = 256;
    private const int MaxTypeNameBytes = 512;
    private const int MaxOwnedBytes = 16 * 1024 * 1024;

    private NativeContentCatalogOwnerObservation(
        IReadOnlyList<Definition> definitions,
        IReadOnlyDictionary<string, int> registryDefinitionCounts,
        ulong generation)
    {
        Definitions = definitions;
        RegistryDefinitionCounts = registryDefinitionCounts;
        Generation = generation;
    }

    internal IReadOnlyList<Definition> Definitions { get; }
    internal IReadOnlyDictionary<string, int> RegistryDefinitionCounts { get; }
    internal ulong Generation { get; }

    internal static NativeContentCatalogOwnerObservation Capture()
    {
        FieldInfo field = typeof(ModelDb).GetField(
                RegistryFieldName,
                BindingFlags.NonPublic | BindingFlags.Static | BindingFlags.DeclaredOnly)
            ?? throw new InvalidOperationException("host ModelDb registry field unavailable");
        if (!field.IsPrivate
            || !field.IsStatic
            || field.FieldType != typeof(Dictionary<ModelId, AbstractModel>))
        {
            throw new InvalidOperationException("host ModelDb registry shape changed");
        }

        if (field.GetValue(null) is not Dictionary<ModelId, AbstractModel> registry)
            throw new InvalidOperationException("host ModelDb registry unavailable");
        RegistryRead before = ReadRegistry(registry);
        if (before.Definitions.Count == 0)
            throw new InvalidOperationException("host ModelDb registry is not ready");
        RegistryRead after = ReadRegistry(registry);
        if (!before.HasSameReferences(after))
            throw new InvalidOperationException("host ModelDb registry changed during observation");

        return new NativeContentCatalogOwnerObservation(
            before.Definitions, before.Counts, before.Fingerprint);
    }

    private static RegistryRead ReadRegistry(Dictionary<ModelId, AbstractModel> registry)
    {
        int beforeCount = registry.Count;
        if (beforeCount <= 0 || beforeCount > MaxEntries)
            throw new InvalidOperationException("host ModelDb registry is not ready");
        var definitions = new List<Definition>(beforeCount);
        var counts = new Dictionary<string, int>(StringComparer.Ordinal);
        var references = new Dictionary<(string Category, string Entry), AbstractModel>();
        var identities = new HashSet<(string Category, string Entry)>();
        int ownedBytes = 0;
        Dictionary<ModelId, AbstractModel>.Enumerator enumerator = registry.GetEnumerator();
        try
        {
            while (enumerator.MoveNext())
            {
                KeyValuePair<ModelId, AbstractModel> pair = enumerator.Current;
                if (pair.Value is null)
                    throw new InvalidOperationException("host ModelDb registry contains null");

                string category = RequireIdentity(pair.Key.Category, MaxIdentityBytes);
                string entry = RequireIdentity(pair.Key.Entry, MaxIdentityBytes);
                if (!identities.Add((category, entry)))
                    throw new InvalidOperationException("host ModelDb registry duplicates an ID");
                references.Add((category, entry), pair.Value);

                Type runtimeType = pair.Value.GetType();
                string runtimeTypeName = RequireTypeName(
                    runtimeType.FullName ?? runtimeType.Name);
                string categoryTypeName = RequireTypeName(
                    ModelDb.GetCategoryType(runtimeType).FullName ?? string.Empty);
                ownedBytes = checked(ownedBytes
                    + System.Text.Encoding.UTF8.GetByteCount(category)
                    + System.Text.Encoding.UTF8.GetByteCount(entry)
                    + System.Text.Encoding.UTF8.GetByteCount(runtimeTypeName)
                    + System.Text.Encoding.UTF8.GetByteCount(categoryTypeName));
                if (ownedBytes > MaxOwnedBytes)
                    throw new InvalidOperationException("host ModelDb observation exceeds its bound");

                definitions.Add(new Definition(
                    category,
                    entry,
                    runtimeTypeName,
                    categoryTypeName,
                    pair.Value.IsCanonical,
                    pair.Value.IsMutable,
                    pair.Value.CategorySortingId,
                    pair.Value.EntrySortingId));
                counts[category] = counts.TryGetValue(category, out int count)
                    ? checked(count + 1)
                    : 1;
            }
        }
        finally
        {
            enumerator.Dispose();
        }
        if (registry.Count != beforeCount || definitions.Count != beforeCount)
            throw new InvalidOperationException("host ModelDb registry changed during observation");
        definitions.Sort(static (left, right) =>
        {
            int category = string.CompareOrdinal(left.EntityKind, right.EntityKind);
            return category != 0
                ? category
                : string.CompareOrdinal(left.NamespacedId, right.NamespacedId);
        });
        string fingerprintInput = string.Join(
            "\n",
            definitions.ConvertAll(static item =>
                $"{item.EntityKind}\u001f{item.NamespacedId}\u001f{item.RuntimeType}\u001f"
                + $"{item.CategorySortingId}\u001f{item.EntrySortingId}\u001f"
                + $"{item.IsCanonical}\u001f{item.IsMutable}"));
        byte[] fingerprint = SHA256.HashData(
            System.Text.Encoding.UTF8.GetBytes(fingerprintInput));
        ulong generation = BitConverter.ToUInt64(fingerprint, 0)
            & 9_007_199_254_740_991UL;
        return new RegistryRead(definitions, counts, references, generation);
    }

    private static string RequireIdentity(string value, int maxBytes)
    {
        if (!ContentManifestWireContract.ValidIdentity(value)
            || System.Text.Encoding.UTF8.GetByteCount(value) > maxBytes)
        {
            throw new InvalidOperationException("host ModelDb identity is malformed");
        }
        return value;
    }

    private static string RequireTypeName(string value)
    {
        if (value.Length == 0
            || value.Length > MaxTypeNameBytes
            || value.IndexOfAny(['\0', '\r', '\n']) >= 0)
        {
            throw new InvalidOperationException("host ModelDb type name is malformed");
        }
        return value;
    }

    internal sealed record Definition(
        string EntityKind,
        string NamespacedId,
        string RuntimeType,
        string CategoryType,
        bool IsCanonical,
        bool IsMutable,
        int CategorySortingId,
        int EntrySortingId);

    private sealed record RegistryRead(
        IReadOnlyList<Definition> Definitions,
        IReadOnlyDictionary<string, int> Counts,
        IReadOnlyDictionary<(string Category, string Entry), AbstractModel> References,
        ulong Fingerprint)
    {
        internal bool HasSameReferences(RegistryRead other)
        {
            if (Fingerprint != other.Fingerprint
                || References.Count != other.References.Count)
                return false;
            foreach ((string Category, string Entry) key in References.Keys)
            {
                if (!other.References.TryGetValue(key, out AbstractModel? value)
                    || !ReferenceEquals(References[key], value))
                    return false;
            }
            return true;
        }
    }
}
