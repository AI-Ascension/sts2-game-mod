// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.Linq;
using System.Reflection;
using System.Text;
using System.Text.Json;
using System.Threading;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2ModelDbRegistryProbe;

internal sealed record RegistryProbeItem(
    string IdCategory,
    string IdEntry,
    string RuntimeType,
    string CategoryType);

internal sealed class RegistryProbeSnapshot
{
    private readonly ReadOnlyCollection<RegistryProbeItem> _items;

    internal RegistryProbeSnapshot(
        string gameBuild,
        IReadOnlyList<RegistryProbeItem> items,
        int copiedStringBytes)
    {
        GameBuild = gameBuild;
        _items = Array.AsReadOnly(items.ToArray());
        CopiedStringBytes = copiedStringBytes;
    }

    internal string GameBuild { get; }
    internal IReadOnlyList<RegistryProbeItem> Items => _items;
    internal int Count => _items.Count;
    internal int CopiedStringBytes { get; }

    internal bool HasSameOwnedValues(RegistryProbeSnapshot other) =>
        string.Equals(GameBuild, other.GameBuild, StringComparison.Ordinal)
        && _items.SequenceEqual(other._items);

    internal string SerializeBoundedReport(string hostHash, int maxEncodedBytes)
    {
        var report = new Dictionary<string, object?>
        {
            ["kind"] = "modeldb_registry_probe",
            ["game_build"] = GameBuild,
            ["host_sha256"] = hostHash,
            ["registry_entry_count"] = Count,
            ["stability_captures"] = 2,
            ["copied_string_bytes"] = CopiedStringBytes,
            ["items"] = _items
        };
        string encoded = JsonSerializer.Serialize(report);
        if (Encoding.UTF8.GetByteCount(encoded) > maxEncodedBytes)
            throw new ProbeFailure("report_limit_exceeded");
        return encoded;
    }
}

internal sealed record RegistryProbeLimits(
    int MaxEntries,
    int MaxIdentityBytes,
    int MaxTypeNameBytes,
    int MaxOwnedStringBytes,
    TimeSpan MaxCaptureDuration)
{
    internal static RegistryProbeLimits Production { get; } = new(
        MaxEntries: 65_536,
        MaxIdentityBytes: 256,
        MaxTypeNameBytes: 512,
        MaxOwnedStringBytes: 8 * 1024 * 1024,
        MaxCaptureDuration: TimeSpan.FromSeconds(2));
}

internal static class ModelDbRegistryCapture
{
    internal const string ExpectedGameBuild = "v0.107.1";
    internal const string RegistryFieldName = "_contentById";

    internal static Dictionary<ModelId, AbstractModel> ResolvePinnedRegistry(
        Type ownerType,
        FieldInfo? field,
        object? fieldValue)
    {
        _ = RequirePinnedField(ownerType, field);
        if (fieldValue is not Dictionary<ModelId, AbstractModel> registry)
            throw new ProbeFailure("registry_unavailable");
        return registry;
    }

    internal static FieldInfo RequirePinnedField(Type ownerType, FieldInfo? field)
    {
        Type expectedType = typeof(Dictionary<ModelId, AbstractModel>);
        if (ownerType != typeof(ModelDb)
            || field is null
            || field.Name != RegistryFieldName
            || field.DeclaringType != ownerType
            || !field.IsPrivate
            || !field.IsStatic
            || field.FieldType != expectedType)
        {
            throw new ProbeFailure("registry_shape_unsupported");
        }
        return field;
    }

    internal static FieldInfo? FindPinnedField(Type ownerType) =>
        ownerType.GetField(
            RegistryFieldName,
            BindingFlags.NonPublic | BindingFlags.Static | BindingFlags.DeclaredOnly);

    internal static RegistryProbeSnapshot Capture(
        Dictionary<ModelId, AbstractModel> registry,
        Func<Type, Type> categoryTypeResolver,
        string gameBuild,
        int ownerThreadId,
        CancellationToken cancellationToken,
        RegistryProbeLimits? limits = null)
    {
        RegistryProbeLimits bounds = limits ?? RegistryProbeLimits.Production;
        if (!string.Equals(gameBuild, ExpectedGameBuild, StringComparison.Ordinal))
            throw new ProbeFailure("game_build_mismatch");
        if (ownerThreadId <= 0 || Environment.CurrentManagedThreadId != ownerThreadId)
            throw new ProbeFailure("wrong_owner_thread");
        if (bounds.MaxEntries <= 0 || bounds.MaxIdentityBytes <= 0
            || bounds.MaxTypeNameBytes <= 0 || bounds.MaxOwnedStringBytes <= 0
            || bounds.MaxCaptureDuration <= TimeSpan.Zero)
        {
            throw new ProbeFailure("invalid_probe_bounds");
        }

        if (cancellationToken.IsCancellationRequested)
            throw new ProbeFailure("probe_cancelled");
        var stopwatch = Stopwatch.StartNew();
        using var captureCancellation =
            CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        captureCancellation.CancelAfter(bounds.MaxCaptureDuration);
        CancellationToken captureToken = captureCancellation.Token;
        int beforeCount;
        try
        {
            beforeCount = registry.Count;
        }
        catch (Exception)
        {
            throw new ProbeFailure("registry_unavailable");
        }

        if (beforeCount == 0)
            throw new ProbeFailure("registry_not_initialized");
        if (beforeCount > bounds.MaxEntries)
            throw new ProbeFailure("registry_entry_limit_exceeded");

        var items = new List<RegistryProbeItem>(beforeCount);
        var identities = new HashSet<(string Category, string Entry)>();
        int ownedBytes = 0;
        Dictionary<ModelId, AbstractModel>.Enumerator enumerator = registry.GetEnumerator();
        try
        {
            while (true)
            {
                bool hasNext;
                KeyValuePair<ModelId, AbstractModel> pair;
                try
                {
                    hasNext = enumerator.MoveNext();
                    pair = hasNext ? enumerator.Current : default;
                }
                catch (InvalidOperationException)
                {
                    throw new ProbeFailure("registry_changed");
                }
                if (!hasNext)
                    break;

                captureToken.ThrowIfCancellationRequested();
                if (stopwatch.Elapsed >= bounds.MaxCaptureDuration)
                    throw new ProbeFailure("capture_timeout");
                if (items.Count >= bounds.MaxEntries)
                    throw new ProbeFailure("registry_entry_limit_exceeded");
                if (pair.Value is null)
                    throw new ProbeFailure("registry_value_malformed");

                string category = pair.Key.Category;
                string entry = pair.Key.Entry;
                Type runtimeType = pair.Value.GetType();
                Type categoryType = categoryTypeResolver(runtimeType);
                string runtimeTypeName = runtimeType.FullName ?? string.Empty;
                string categoryTypeName = categoryType.FullName ?? string.Empty;
                ValidateIdentity(category, bounds.MaxIdentityBytes);
                ValidateIdentity(entry, bounds.MaxIdentityBytes);
                ValidateTypeName(runtimeTypeName, bounds.MaxTypeNameBytes);
                ValidateTypeName(categoryTypeName, bounds.MaxTypeNameBytes);

                if (!identities.Add((category, entry)))
                    throw new ProbeFailure("registry_duplicate_id");
                ownedBytes = checked(ownedBytes
                    + Encoding.UTF8.GetByteCount(category)
                    + Encoding.UTF8.GetByteCount(entry)
                    + Encoding.UTF8.GetByteCount(runtimeTypeName)
                    + Encoding.UTF8.GetByteCount(categoryTypeName));
                if (ownedBytes > bounds.MaxOwnedStringBytes)
                    throw new ProbeFailure("registry_string_limit_exceeded");

                items.Add(new RegistryProbeItem(
                    category, entry, runtimeTypeName, categoryTypeName));
            }

            captureToken.ThrowIfCancellationRequested();
            if (stopwatch.Elapsed >= bounds.MaxCaptureDuration)
                throw new ProbeFailure("capture_timeout");
            int afterCount = registry.Count;
            if (beforeCount != afterCount || items.Count != beforeCount)
                throw new ProbeFailure("registry_changed");
        }
        catch (ProbeFailure)
        {
            throw;
        }
        catch (OperationCanceledException)
        {
            throw new ProbeFailure(cancellationToken.IsCancellationRequested
                ? "probe_cancelled"
                : "capture_timeout");
        }
        catch (Exception)
        {
            throw new ProbeFailure("registry_read_failed");
        }
        finally
        {
            enumerator.Dispose();
        }

        RegistryProbeItem[] ordered = items
            .OrderBy(item => item.IdCategory, StringComparer.Ordinal)
            .ThenBy(item => item.IdEntry, StringComparer.Ordinal)
            .ThenBy(item => item.CategoryType, StringComparer.Ordinal)
            .ToArray();
        return new RegistryProbeSnapshot(gameBuild, ordered, ownedBytes);
    }

    private static void ValidateIdentity(string value, int maxBytes)
    {
        if (string.IsNullOrEmpty(value)
            || Encoding.UTF8.GetByteCount(value) > maxBytes
            || value.Any(static character => !(char.IsAsciiLetterOrDigit(character)
                || character is '.' or ':' or '/' or '_' or '-')))
        {
            throw new ProbeFailure("registry_id_malformed");
        }
    }

    private static void ValidateTypeName(string value, int maxBytes)
    {
        if (string.IsNullOrEmpty(value)
            || Encoding.UTF8.GetByteCount(value) > maxBytes
            || value.Any(char.IsControl))
        {
            throw new ProbeFailure("registry_type_malformed");
        }
    }
}
