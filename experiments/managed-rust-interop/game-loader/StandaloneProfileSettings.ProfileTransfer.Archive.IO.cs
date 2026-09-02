// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private static IEnumerable<string> EnumerateProfileFiles(string profileDirectory)
    {
        string savesDirectory = Path.Combine(profileDirectory, "saves");
        if (!Directory.Exists(savesDirectory) || IsReparsePoint(savesDirectory))
        {
            yield break;
        }

        var pending = new Stack<string>();
        pending.Push(savesDirectory);
        while (pending.Count > 0)
        {
            string directory = pending.Pop();
            foreach (string file in Directory.EnumerateFiles(directory).OrderBy(path => path, StringComparer.Ordinal))
            {
                if (!IsReparsePoint(file)) yield return file;
            }

            foreach (string child in Directory.EnumerateDirectories(directory)
                         .OrderBy(path => path, StringComparer.Ordinal).Reverse())
            {
                if (!IsReparsePoint(child)) pending.Push(child);
            }
        }
    }

    private static long GetTotalFileLength(IEnumerable<string> files)
    {
        long total = 0;
        foreach (string file in files)
        {
            long length = new FileInfo(file).Length;
            if (length < 0 || length > MaxProfileTransferBytes - total)
            {
                throw new InvalidDataException("Profile data exceeds the transfer limit.");
            }

            total += length;
        }

        return total;
    }

    private static void CopyBounded(Stream source, Stream destination, ref long total)
    {
        byte[] buffer = new byte[64 * 1024];
        int read;
        while ((read = source.Read(buffer, 0, buffer.Length)) > 0)
        {
            if (read > MaxProfileTransferBytes - total)
            {
                throw new InvalidDataException("Profile data exceeds the transfer limit.");
            }

            destination.Write(buffer, 0, read);
            total += read;
        }
    }

    private static string NormalizeArchivePath(string selectedPath)
    {
        string path = Path.GetFullPath(selectedPath);
        return path.EndsWith(ProfileArchiveExtension, StringComparison.OrdinalIgnoreCase)
            ? path
            : path + ProfileArchiveExtension;
    }

    private static string ToArchivePath(string path) => path.Replace('\\', '/');

    private static bool IsPathInside(string parent, string candidate)
    {
        string parentPath = Path.GetFullPath(parent).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)
            + Path.DirectorySeparatorChar;
        string candidatePath = Path.GetFullPath(candidate);
        return candidatePath.StartsWith(parentPath, StringComparison.OrdinalIgnoreCase);
    }

    private static bool IsReparsePoint(string path)
        => (File.GetAttributes(path) & FileAttributes.ReparsePoint) != 0;

    private static void DeleteTemporaryFile(string path)
    {
        if (!string.IsNullOrWhiteSpace(path) && File.Exists(path))
        {
            try { File.Delete(path); }
            catch { /* The next export can use another temporary name. */ }
        }
    }

    private static void DeleteTemporaryDirectory(string path)
    {
        if (!string.IsNullOrWhiteSpace(path) && Directory.Exists(path))
        {
            try { Directory.Delete(path, recursive: true); }
            catch { /* The directory is confined to the system temporary path. */ }
        }
    }
}
