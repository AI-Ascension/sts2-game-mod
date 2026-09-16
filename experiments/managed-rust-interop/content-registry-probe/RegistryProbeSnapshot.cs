// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2ModelDbRegistryProbe;

internal sealed record RegistryProbeItem(
    string IdCategory,
    string IdEntry,
    string RuntimeType,
    string CategoryType);

internal sealed class RegistryProbeSnapshot
{
    private const string ReadinessKind = "observed_partial_registry_stability";
    private const string ReadinessBasis =
        "mod_manager_initialized_nonempty_registry_two_matching_owner_thread_process_frame_captures";
    private const string ReadinessLimit =
        "does_not_establish_modeldb_init_completion_or_catalog_completeness";
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
            ["readiness_kind"] = ReadinessKind,
            ["readiness_basis"] = ReadinessBasis,
            ["readiness_limit"] = ReadinessLimit,
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
