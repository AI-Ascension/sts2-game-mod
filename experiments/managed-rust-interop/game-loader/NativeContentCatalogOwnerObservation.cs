// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Reflection;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Copies the exact host ModelDb owner registry into bounded values. This is an observation seam,
/// not a complete content-manifest source: the inspected host exposes no catalog generation,
/// origin/override provenance, or semantic-input reader.
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
        IReadOnlyDictionary<string, int> registryDefinitionCounts)
    {
        Definitions = definitions;
        RegistryDefinitionCounts = registryDefinitionCounts;
    }

    internal IReadOnlyList<Definition> Definitions { get; }
    internal IReadOnlyDictionary<string, int> RegistryDefinitionCounts { get; }

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
        int beforeCount = registry.Count;
        if (beforeCount <= 0 || beforeCount > MaxEntries)
            throw new InvalidOperationException("host ModelDb registry is not ready");

        var definitions = new List<Definition>(beforeCount);
        var counts = new Dictionary<string, int>(StringComparer.Ordinal);
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
        return new NativeContentCatalogOwnerObservation(definitions, counts);
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
}
