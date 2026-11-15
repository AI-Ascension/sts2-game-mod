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
    private static string? _originalRoot;
    private static SortedDictionary<string, string>? _originalFiles;
    private static SortedSet<string>? _originalDirectories;

    internal static string CaptureInitial()
    {
        // A failed capture must not leave a previous profile's digest admissible.
        ClearCaptured();
        string root = Path.GetFullPath(OS.GetUserDataDir());
        var rootDirectory = new DirectoryInfo(root);
        if (HasLinkedOrMissingAncestor(rootDirectory))
        {
            throw new InvalidOperationException("isolated user directory is unavailable");
        }

        var inventory = new SortedDictionary<string, string>(StringComparer.Ordinal);
        var originalDirectories = new SortedSet<string>(StringComparer.Ordinal);
        var pending = new Stack<DirectoryInfo>();
        pending.Push(rootDirectory);
        while (pending.Count > 0)
        {
            DirectoryInfo directory = pending.Pop();
            if (IsLink(directory.Attributes))
            {
                throw new InvalidOperationException("isolated user directory contains a linked profile entry");
            }

            foreach (DirectoryInfo child in directory.EnumerateDirectories())
            {
                string relative = Path.GetRelativePath(root, child.FullName).Replace('\\', '/');
                if (IsLink(child.Attributes))
                {
                    throw new InvalidOperationException("isolated user directory contains a linked profile entry");
                }

                // Cache/telemetry is not hashed, but it must still be traversed for
                // linked descendants before admitting a save-profile baseline.
                if (IsBootDerivedPath(relative))
                {
                    pending.Push(child);
                    continue;
                }
                originalDirectories.Add(relative);
                pending.Push(child);
            }

            foreach (FileInfo file in directory.EnumerateFiles())
            {
                string relative = Path.GetRelativePath(root, file.FullName).Replace('\\', '/');
                if (IsLink(file.Attributes))
                {
                    throw new InvalidOperationException("isolated user directory contains a linked profile entry");
                }

                if (IsBootDerivedPath(relative))
                {
                    continue;
                }
                inventory[relative] = Convert.ToHexString(
                    SHA256.HashData(File.ReadAllBytes(file.FullName))).ToLowerInvariant();
            }
        }

        if (inventory.Count == 0)
        {
            throw new InvalidOperationException("isolated user directory has no profile files");
        }

        string digest = Convert.ToHexString(
            SHA256.HashData(JsonSerializer.SerializeToUtf8Bytes(inventory))).ToLowerInvariant();
        _originalRoot = root;
        _originalFiles = inventory;
        _originalDirectories = originalDirectories;
        _digest = digest;
        return digest;
    }

    private static void ClearCaptured()
    {
        _digest = null;
        _originalRoot = null;
        _originalFiles = null;
        _originalDirectories = null;
    }

    // Lexical GetFullPath does not resolve linked ancestors. An otherwise ordinary
    // root below a symlinked parent must never be accepted as isolated profile evidence.
    // This is a metadata guard, not an atomic no-follow native root attestation.
    private static bool HasLinkedOrMissingAncestor(DirectoryInfo root)
    {
        for (DirectoryInfo? current = root; current is not null; current = current.Parent)
        {
            if (!current.Exists || IsLink(current.Attributes)) return true;
        }
        return false;
    }

    // Only files present before the mock save backend was installed are pinned.
    // New overlay files are not part of the original profile's identity.
    // Source-only drift detection does NOT establish an atomic no-follow root witness.
    private static bool OriginalFilesUnchanged()
    {
        if (_originalRoot is null || _originalFiles is null
            || _originalDirectories is null) return false;
        try
        {
            string root = Path.GetFullPath(OS.GetUserDataDir());
            StringComparison comparison = OperatingSystem.IsWindows()
                ? StringComparison.OrdinalIgnoreCase : StringComparison.Ordinal;
            if (!string.Equals(root, _originalRoot, comparison)
                || HasLinkedOrMissingAncestor(new DirectoryInfo(root))) return false;

            // New overlay files are allowed, but linked escapes are never evidence.
            // This metadata scan is NOT an atomic native no-follow path witness.
            if (!NoLateLinks(root)) return false;

            // Empty original directories are part of the protected baseline, too.
            // New overlay directories remain permitted, but original removals refuse.
            foreach (string relative in _originalDirectories)
            {
                string path = Path.Combine(root, relative.Replace('/', Path.DirectorySeparatorChar));
                var directory = new DirectoryInfo(path);
                if (!directory.Exists || IsLink(directory.Attributes)) return false;
            }

            foreach (KeyValuePair<string, string> entry in _originalFiles)
            {
                string[] components = entry.Key.Split('/');
                string directory = root;
                for (int i = 0; i < components.Length - 1; i++)
                {
                    directory = Path.Combine(directory, components[i]);
                    var parent = new DirectoryInfo(directory);
                    if (!parent.Exists || IsLink(parent.Attributes)) return false;
                }

                string path = Path.Combine(directory, components[^1]);
                var file = new FileInfo(path);
                if (!file.Exists || IsLink(file.Attributes)) return false;
                string current = Convert.ToHexString(
                    SHA256.HashData(File.ReadAllBytes(path))).ToLowerInvariant();
                if (!string.Equals(current, entry.Value, StringComparison.Ordinal))
                    return false;
            }

            return true;
        }
        catch (IOException) { return false; }
        catch (UnauthorizedAccessException) { return false; }
        catch (ArgumentException) { return false; }
    }

    private static bool NoLateLinks(string root)
    {
        var pending = new Stack<DirectoryInfo>();
        pending.Push(new DirectoryInfo(root));
        while (pending.Count > 0)
        {
            DirectoryInfo directory = pending.Pop();
            if (IsLink(directory.Attributes)) return false;
            foreach (DirectoryInfo child in directory.EnumerateDirectories())
            {
                string relative = Path.GetRelativePath(root, child.FullName).Replace('\\', '/');
                if (IsLink(child.Attributes)) return false;
                pending.Push(child);
            }
            foreach (FileInfo file in directory.EnumerateFiles())
            {
                string relative = Path.GetRelativePath(root, file.FullName).Replace('\\', '/');
                if (IsLink(file.Attributes)) return false;
            }
        }
        return true;
    }

    private static bool IsLink(FileAttributes attributes) =>
        (attributes & FileAttributes.ReparsePoint) != 0;

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

        if (!OriginalFilesUnchanged())
        {
            ClearCaptured();
            error = "profile_baseline_original_changed";
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
