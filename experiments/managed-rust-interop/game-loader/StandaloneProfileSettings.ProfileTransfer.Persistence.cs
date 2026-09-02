// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.Json;
using Godot;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;
using MegaCrit.Sts2.Core.Saves.Managers;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private static bool CanTransferProfile(out string error)
    {
        if (RunManager.Instance?.IsInProgress == true)
        {
            error = "Profile transfer is unavailable while a run is in progress.";
            return false;
        }

        try
        {
            if (SaveManager.Instance.CurrentProfileId is < MinProfileId or > MaxProfileId)
            {
                error = "Profile data is not ready yet. Try again from the main menu.";
                return false;
            }
        }
        catch (InvalidOperationException)
        {
            error = "Profile data is not ready yet. Try again from the main menu.";
            return false;
        }

        error = string.Empty;
        return true;
    }

    private static string ExportProfile(int profileId, string selectedPath)
    {
        string temporaryPath = string.Empty;
        try
        {
            if (!CanTransferProfile(out string error)) return error;
            if (!TryGetProfileDirectory(profileId, requireExistingProgress: true, out string profileDirectory, out error))
            {
                return error;
            }

            if (TryGetCurrentProfileId(out int currentProfileId) && currentProfileId == profileId)
            {
                SaveManager.Instance.SaveProgressFile();
            }

            string outputPath = NormalizeArchivePath(selectedPath);
            if (IsPathInside(profileDirectory, outputPath))
            {
                return "Choose an export location outside the selected profile.";
            }

            List<string> files = EnumerateProfileFiles(profileDirectory).ToList();
            if (files.Count == 0) return "The selected profile has no saved progress to export.";
            if (files.Count > MaxProfileTransferFiles)
            {
                return "The selected profile contains too many files to export.";
            }

            long expectedBytes = GetTotalFileLength(files);
            if (expectedBytes > MaxProfileTransferBytes)
            {
                return "The selected profile is too large to export.";
            }

            string? outputDirectory = Path.GetDirectoryName(outputPath);
            if (string.IsNullOrWhiteSpace(outputDirectory))
            {
                return "Choose a valid export location.";
            }

            Directory.CreateDirectory(outputDirectory);
            temporaryPath = Path.Combine(outputDirectory, $".{Path.GetFileName(outputPath)}.{Guid.NewGuid():N}.tmp");
            long copiedBytes = 0;
            using (var archiveStream = new FileStream(
                temporaryPath,
                FileMode.CreateNew,
                System.IO.FileAccess.Write,
                FileShare.None,
                64 * 1024,
                FileOptions.SequentialScan))
            using (var archive = new ZipArchive(archiveStream, ZipArchiveMode.Create, leaveOpen: false))
            {
                foreach (string file in files)
                {
                    string relativePath = ToArchivePath(Path.GetRelativePath(profileDirectory, file));
                    ZipArchiveEntry entry = archive.CreateEntry(relativePath, CompressionLevel.Fastest);
                    using Stream source = File.OpenRead(file);
                    using Stream destination = entry.Open();
                    CopyBounded(source, destination, ref copiedBytes);
                }

                if (copiedBytes != expectedBytes)
                {
                    throw new IOException("A profile file changed while it was being exported.");
                }

                string manifest = JsonSerializer.Serialize(new Dictionary<string, object?>
                {
                    ["format"] = ProfileManifestFormat,
                    ["version"] = ProfileManifestVersion,
                    ["profile_id"] = profileId,
                    ["file_count"] = files.Count,
                    ["total_bytes"] = copiedBytes
                });
                ZipArchiveEntry manifestEntry = archive.CreateEntry(ProfileManifestName, CompressionLevel.Fastest);
                using Stream manifestStream = manifestEntry.Open();
                using var writer = new StreamWriter(manifestStream);
                writer.Write(manifest);
            }

            File.Move(temporaryPath, outputPath, overwrite: true);
            temporaryPath = string.Empty;
            return $"Profile {profileId} exported successfully.";
        }
        catch (InvalidDataException)
        {
            return "The selected profile could not be exported because its data is invalid.";
        }
        catch (UnauthorizedAccessException)
        {
            return "Profile export failed because the selected location is not writable.";
        }
        catch (IOException exception)
        {
            GD.PrintErr($"{LogPrefix} profile export failed: {exception.GetType().Name}");
            return "Profile export failed. Check the selected location and try again.";
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} profile export failed: {exception.GetType().Name}");
            return "Profile export failed. Try again from the main menu.";
        }
        finally
        {
            DeleteTemporaryFile(temporaryPath);
        }
    }

    private static string ImportProfile(int profileId, string archivePath)
    {
        string stagingDirectory = string.Empty;
        try
        {
            if (!CanTransferProfile(out string error)) return error;
            if (!File.Exists(archivePath)) return "The selected profile export file was not found.";
            if (!TryGetProfileDirectory(profileId, requireExistingProgress: false, out string profileDirectory, out error))
            {
                return error;
            }

            if (IsPathInside(profileDirectory, Path.GetFullPath(archivePath)))
            {
                return "Choose a profile export file outside the selected profile.";
            }

            if (new FileInfo(archivePath).Length > MaxProfileTransferBytes)
            {
                return "The selected profile export file is too large.";
            }

            stagingDirectory = Path.Combine(Path.GetTempPath(), $"ai-ascension-profile-{Guid.NewGuid():N}");
            Directory.CreateDirectory(stagingDirectory);
            ProfileManifest manifest;
            using (var archive = ZipFile.OpenRead(archivePath))
            {
                if (!TryReadManifest(archive, out manifest, out error)) return error;
                ExtractArchive(archive, stagingDirectory, manifest);
            }

            string stagedSavesDirectory = Path.Combine(stagingDirectory, "saves");
            if (!File.Exists(Path.Combine(stagedSavesDirectory, "progress.save")))
            {
                return "The profile export does not contain saved progress.";
            }

            if (TryGetCurrentProfileId(out int currentProfileId) && currentProfileId == profileId)
            {
                SaveManager.Instance.SaveProgressFile();
            }

            ReplaceProfileSaves(profileDirectory, stagedSavesDirectory);
            return $"Profile {profileId} imported successfully. Restart the game to load it.";
        }
        catch (InvalidDataException)
        {
            return "The selected file is not a valid STS2 profile export.";
        }
        catch (UnauthorizedAccessException)
        {
            return "Profile import failed because the save location is not writable.";
        }
        catch (IOException exception)
        {
            GD.PrintErr($"{LogPrefix} profile import failed: {exception.GetType().Name}");
            return "Profile import failed. The selected profile was restored when possible.";
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} profile import failed: {exception.GetType().Name}");
            return "Profile import failed. Try again from the main menu.";
        }
        finally
        {
            DeleteTemporaryDirectory(stagingDirectory);
        }
    }

}
