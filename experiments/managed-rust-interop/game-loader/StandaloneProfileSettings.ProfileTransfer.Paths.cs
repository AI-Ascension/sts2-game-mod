// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using Godot;
using MegaCrit.Sts2.Core.Saves;
using MegaCrit.Sts2.Core.Saves.Managers;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private static bool TryGetProfileDirectory(
        int profileId,
        bool requireExistingProgress,
        out string profileDirectory,
        out string error)
    {
        profileDirectory = string.Empty;
        string? rawProgressPath;
        try
        {
            rawProgressPath = ProgressSaveManager.GetProgressPathForProfile(profileId);
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} profile path lookup failed: {exception.GetType().Name}");
            error = "The game save location is unavailable.";
            return false;
        }

        if (string.IsNullOrWhiteSpace(rawProgressPath))
        {
            error = "The game save location is unavailable.";
            return false;
        }

        List<string> candidates = BuildProgressPathCandidates(rawProgressPath, profileId);
        foreach (string candidate in candidates)
        {
            string? savesDirectory = Path.GetDirectoryName(candidate);
            if (string.IsNullOrWhiteSpace(savesDirectory)
                || !string.Equals(Path.GetFileName(savesDirectory), "saves", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            string? root = Directory.GetParent(savesDirectory)?.FullName;
            if (string.IsNullOrWhiteSpace(root) || (requireExistingProgress && !File.Exists(candidate))) continue;
            if (Directory.Exists(root) && IsReparsePoint(root)) continue;
            profileDirectory = root;
            error = string.Empty;
            return true;
        }

        error = requireExistingProgress
            ? "The selected profile has no saved progress to export."
            : "The game save location is unavailable.";
        return false;
    }

    private static List<string> BuildProgressPathCandidates(string rawPath, int profileId)
    {
        var candidates = new List<string>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        string profileRoot = GetProfileRoot(rawPath, profileId);

        void AddCandidate(string path)
        {
            try
            {
                string fullPath = Path.GetFullPath(path);
                if (seen.Add(fullPath)) candidates.Add(fullPath);
            }
            catch
            {
                // Ignore malformed host paths and keep the next safe candidate.
            }
        }

        if (Path.IsPathRooted(rawPath))
        {
            AddCandidate(rawPath);
            return candidates;
        }

        string userDataDirectory = OS.GetUserDataDir();
        if (!string.IsNullOrWhiteSpace(userDataDirectory))
        {
            AddCandidate(Path.Combine(userDataDirectory, rawPath));
            AddCandidate(Path.Combine(userDataDirectory, profileRoot, "saves", "progress.save"));
        }

        return candidates;
    }

    private static string GetProfileRoot(string rawPath, int profileId)
    {
        string normalized = rawPath.Replace('\\', '/');
        string moddedRoot = $"modded/profile{profileId}";
        return normalized.Contains(moddedRoot, StringComparison.OrdinalIgnoreCase)
            ? moddedRoot
            : $"profile{profileId}";
    }

    private static int GetCurrentProfileId() => SaveManager.Instance.CurrentProfileId;

    private static bool TryGetCurrentProfileId(out int profileId)
    {
        try
        {
            profileId = GetCurrentProfileId();
            return profileId is >= MinProfileId and <= MaxProfileId;
        }
        catch (InvalidOperationException)
        {
            profileId = 0;
            return false;
        }
    }
}
