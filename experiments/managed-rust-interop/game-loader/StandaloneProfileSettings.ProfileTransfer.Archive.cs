// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private const string ProfileArchiveExtension = ".sts2profile";
    private const string ProfileManifestName = "ai-ascension-profile.json";
    private const string ProfileManifestFormat = "ai-ascension-sts2-profile";
    private const int ProfileManifestVersion = 1;
    private const int MaxProfileManifestBytes = 16 * 1024;
    private const int MaxProfileTransferFiles = 8192;
    private const long MaxProfileTransferBytes = 256L * 1024 * 1024;

    private readonly record struct ProfileManifest(int ProfileId, int FileCount, long TotalBytes);

    private static bool TryReadManifest(ZipArchive archive, out ProfileManifest manifest, out string error)
    {
        manifest = default;
        ZipArchiveEntry? manifestEntry = null;
        int nonManifestEntries = 0;
        long totalBytes = 0;
        bool hasProgress = false;
        var names = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        foreach (ZipArchiveEntry entry in archive.Entries)
        {
            string name = ToArchivePath(entry.FullName);
            if (name == ProfileManifestName)
            {
                if (manifestEntry != null)
                {
                    error = "The profile export contains duplicate metadata.";
                    return false;
                }

                manifestEntry = entry;
                continue;
            }

            if (!IsSafeArchivePath(name) || !names.Add(name))
            {
                error = "The profile export contains an unsafe or duplicate file path.";
                return false;
            }

            if (entry.Length < 0 || entry.Length > MaxProfileTransferBytes - totalBytes)
            {
                error = "The profile export is too large.";
                return false;
            }

            totalBytes += entry.Length;
            nonManifestEntries++;
            hasProgress |= string.Equals(name, "saves/progress.save", StringComparison.OrdinalIgnoreCase);
        }

        if (manifestEntry == null
            || manifestEntry.Length > MaxProfileManifestBytes
            || !hasProgress
            || nonManifestEntries > MaxProfileTransferFiles)
        {
            error = "The profile export is missing required metadata or saved progress.";
            return false;
        }

        try
        {
            using Stream stream = manifestEntry.Open();
            using JsonDocument document = JsonDocument.Parse(stream);
            JsonElement root = document.RootElement;
            if (!TryGetString(root, "format", out string? format)
                || !string.Equals(format, ProfileManifestFormat, StringComparison.Ordinal)
                || !TryGetInt32(root, "version", out int version)
                || version != ProfileManifestVersion
                || !TryGetInt32(root, "profile_id", out int profileId)
                || profileId is < MinProfileId or > MaxProfileId
                || !TryGetInt32(root, "file_count", out int fileCount)
                || fileCount != nonManifestEntries
                || !TryGetInt64(root, "total_bytes", out long declaredBytes)
                || declaredBytes != totalBytes)
            {
                error = "The profile export metadata is invalid.";
                return false;
            }

            manifest = new ProfileManifest(profileId, fileCount, totalBytes);
            error = string.Empty;
            return true;
        }
        catch (JsonException)
        {
            error = "The profile export metadata is invalid.";
            return false;
        }
    }

    private static bool IsSafeArchivePath(string path)
    {
        if (path.Length is 0 or > 240 || !path.StartsWith("saves/", StringComparison.Ordinal)) return false;
        if (path.Contains('\\') || path.Contains(':') || path.StartsWith('/')) return false;
        return path.Split('/').All(part => part.Length > 0 && part is not "." and not "..");
    }

    private static bool TryGetString(JsonElement root, string name, out string? value)
    {
        value = null;
        return root.ValueKind == JsonValueKind.Object
            && root.TryGetProperty(name, out JsonElement element)
            && element.ValueKind == JsonValueKind.String
            && (value = element.GetString()) != null;
    }

    private static bool TryGetInt32(JsonElement root, string name, out int value)
    {
        value = 0;
        return root.ValueKind == JsonValueKind.Object
            && root.TryGetProperty(name, out JsonElement element)
            && element.TryGetInt32(out value);
    }

    private static bool TryGetInt64(JsonElement root, string name, out long value)
    {
        value = 0;
        return root.ValueKind == JsonValueKind.Object
            && root.TryGetProperty(name, out JsonElement element)
            && element.TryGetInt64(out value);
    }

    private static void ExtractArchive(ZipArchive archive, string stagingDirectory, ProfileManifest manifest)
    {
        long extractedBytes = 0;
        int extractedFiles = 0;
        foreach (ZipArchiveEntry entry in archive.Entries)
        {
            string name = ToArchivePath(entry.FullName);
            if (name == ProfileManifestName) continue;

            string outputPath = Path.GetFullPath(Path.Combine(stagingDirectory, name.Replace('/', Path.DirectorySeparatorChar)));
            if (!IsPathInside(stagingDirectory, outputPath))
            {
                throw new InvalidDataException("The profile export path escapes its staging directory.");
            }

            string? directory = Path.GetDirectoryName(outputPath);
            if (string.IsNullOrWhiteSpace(directory)) throw new InvalidDataException("Invalid profile path.");
            Directory.CreateDirectory(directory);
            using Stream source = entry.Open();
            using var destination = new FileStream(outputPath, FileMode.CreateNew, FileAccess.Write, FileShare.None);
            CopyBounded(source, destination, ref extractedBytes);
            extractedFiles++;
        }

        if (extractedFiles != manifest.FileCount || extractedBytes != manifest.TotalBytes)
        {
            throw new InvalidDataException("The profile export changed while it was being read.");
        }
    }

    private static void ReplaceProfileSaves(string profileDirectory, string stagedSavesDirectory)
    {
        if (Directory.Exists(profileDirectory) && IsReparsePoint(profileDirectory))
        {
            throw new IOException("The selected profile directory is not a regular directory.");
        }

        Directory.CreateDirectory(profileDirectory);
        string targetSavesDirectory = Path.Combine(profileDirectory, "saves");
        if (Directory.Exists(targetSavesDirectory) && IsReparsePoint(targetSavesDirectory))
        {
            throw new IOException("The selected save directory is not a regular directory.");
        }

        string backupDirectory = Path.Combine(profileDirectory, $".ai-ascension-profile-backup-{Guid.NewGuid():N}");
        bool movedExisting = false;
        bool installedNew = false;
        try
        {
            if (Directory.Exists(targetSavesDirectory))
            {
                Directory.Move(targetSavesDirectory, backupDirectory);
                movedExisting = true;
            }

            Directory.Move(stagedSavesDirectory, targetSavesDirectory);
            installedNew = true;
            if (movedExisting) Directory.Delete(backupDirectory, recursive: true);
        }
        catch
        {
            if (installedNew && Directory.Exists(targetSavesDirectory))
            {
                Directory.Delete(targetSavesDirectory, recursive: true);
            }

            if (movedExisting && Directory.Exists(backupDirectory))
            {
                Directory.Move(backupDirectory, targetSavesDirectory);
            }

            throw;
        }
    }
}
