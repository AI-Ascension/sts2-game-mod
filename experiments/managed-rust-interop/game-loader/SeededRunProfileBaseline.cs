// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Captures the isolated user directory before the live campaign backend is overlaid.
/// The seeded host uses this read-only snapshot to bind a request to the profile that the
/// launcher actually supplied; zero runs alone is not sufficient profile evidence.
/// </summary>
internal static class SeededRunProfileBaseline
{
    private static string? _digest;

    internal static string CaptureInitial()
    {
        string root = Path.GetFullPath(OS.GetUserDataDir());
        if (!Directory.Exists(root))
        {
            throw new InvalidOperationException("isolated user directory is unavailable");
        }

        var inventory = new SortedDictionary<string, string>(StringComparer.Ordinal);
        foreach (string path in Directory.EnumerateFiles(root, "*", SearchOption.AllDirectories))
        {
            string relative = Path.GetRelativePath(root, path).Replace('\\', '/');
            if (IsBootDerivedPath(relative))
            {
                continue;
            }

            inventory[relative] = Convert.ToHexString(
                SHA256.HashData(File.ReadAllBytes(path))).ToLowerInvariant();
        }

        if (inventory.Count == 0)
        {
            throw new InvalidOperationException("isolated user directory has no profile files");
        }

        _digest = Convert.ToHexString(
            SHA256.HashData(JsonSerializer.SerializeToUtf8Bytes(inventory))).ToLowerInvariant();
        return _digest;
    }

    private static bool IsBootDerivedPath(string relative)
    {
        if (string.Equals(relative, "sentry.dat", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        int separator = relative.IndexOf('/');
        string firstComponent = separator < 0 ? relative : relative[..separator];
        return string.Equals(firstComponent, "shader_cache", StringComparison.OrdinalIgnoreCase)
            || string.Equals(firstComponent, "sentry", StringComparison.OrdinalIgnoreCase);
    }

    internal static bool Matches(
        SeededRunContextProfileBaseline baseline,
        out string error)
    {
        if (baseline.Kind != SeededRunSelectionContext.FreshProfileKind)
        {
            error = "profile_baseline_not_fresh";
            return false;
        }

        if (_digest is null)
        {
            error = "profile_baseline_unavailable";
            return false;
        }

        if (!string.Equals(baseline.Digest, _digest, StringComparison.Ordinal))
        {
            error = "profile_baseline_digest_mismatch";
            return false;
        }

        string? expectedDigest = System.Environment.GetEnvironmentVariable(
            "STS2_SEED_PROFILE_BASELINE_DIGEST");
        if (!string.IsNullOrWhiteSpace(expectedDigest)
            && !string.Equals(baseline.Digest, expectedDigest, StringComparison.Ordinal))
        {
            error = "profile_baseline_expected_digest_mismatch";
            return false;
        }

        string? expectedIdentity = System.Environment.GetEnvironmentVariable(
            "STS2_SEED_PROFILE_BASELINE_IDENTITY");
        if (!string.IsNullOrWhiteSpace(expectedIdentity)
            && !string.Equals(baseline.Identity, expectedIdentity, StringComparison.Ordinal))
        {
            error = "profile_baseline_identity_mismatch";
            return false;
        }

        error = string.Empty;
        return true;
    }
}
