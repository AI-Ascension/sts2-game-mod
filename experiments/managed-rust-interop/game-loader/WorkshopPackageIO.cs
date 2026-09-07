// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class WorkshopPackageValidator
{
    private static void RejectReparsePath(string root)
    {
        for (DirectoryInfo? directory = new(root); directory is not null; directory = directory.Parent)
        {
            if (directory.Attributes.HasFlag(FileAttributes.ReparsePoint))
            {
                Reject("reparse_point", "Workshop install path contains a reparse point.");
            }
        }
    }

    private static byte[] ReadBoundedManifest(string path)
    {
        using FileStream stream = new(path, FileMode.Open, FileAccess.Read, FileShare.Read);
        byte[] buffer = new byte[MaximumManifestBytes + 1];
        int count = 0;
        while (count < buffer.Length)
        {
            int read = stream.Read(buffer, count, buffer.Length - count);
            if (read == 0)
            {
                return buffer.AsSpan(0, count).ToArray();
            }
            count += read;
        }
        Reject("manifest_too_large", "Workshop manifest exceeds its byte bound.");
        throw new InvalidOperationException("unreachable");
    }

    private static void ValidateChecksumInventory(
        string root,
        (string Path, string Role)[] expectedPayload)
    {
        byte[] bytes = ReadBoundedChecksums(Path.Combine(root, ChecksumFileName));
        string text;
        try
        {
            text = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true)
                .GetString(bytes);
        }
        catch (DecoderFallbackException)
        {
            Reject("checksum_inventory", "Workshop checksum inventory is not valid UTF-8.");
            throw new InvalidOperationException("unreachable");
        }

        if (!text.EndsWith('\n'))
        {
            Reject("checksum_inventory", "Workshop checksum inventory must end with a newline.");
        }

        string[] lines = text[..^1].Split('\n', StringSplitOptions.None);
        string[] expected = [.. expectedPayload.Select(file => file.Path), ManifestFileName];
        if (lines.Length != expected.Length)
        {
            Reject("checksum_inventory", "Workshop checksum inventory does not cover the exact package.");
        }

        for (int index = 0; index < expected.Length; index++)
        {
            string line = lines[index];
            if (line.EndsWith('\r'))
            {
                line = line[..^1];
            }

            if (line.Length != 66 + expected[index].Length
                || !IsSha256(line[..64])
                || line[64..66] != "  "
                || line[66..] != expected[index])
            {
                Reject("checksum_inventory", "Workshop checksum inventory is malformed or out of order.");
            }

            string path = Path.Combine(root, expected[index]);
            string digest = ComputeFileDigest(path);
            if (!string.Equals(digest, line[..64], StringComparison.Ordinal))
            {
                Reject("checksum_mismatch", $"Workshop checksum does not match: {expected[index]}");
            }
        }
    }

    private static byte[] ReadBoundedChecksums(string path)
    {
        using FileStream stream = new(path, FileMode.Open, FileAccess.Read, FileShare.Read);
        byte[] buffer = new byte[MaximumChecksumBytes + 1];
        int count = 0;
        while (count < buffer.Length)
        {
            int read = stream.Read(buffer, count, buffer.Length - count);
            if (read == 0)
            {
                return buffer.AsSpan(0, count).ToArray();
            }
            count += read;
        }
        Reject("checksum_inventory", "Workshop checksum inventory exceeds its byte bound.");
        throw new InvalidOperationException("unreachable");
    }

    private static void RejectDuplicateProperties(byte[] bytes)
    {
        using JsonDocument document = JsonDocument.Parse(bytes, new JsonDocumentOptions { MaxDepth = 16 });
        CheckProperties(document.RootElement);
    }

    private static void CheckProperties(JsonElement element)
    {
        if (element.ValueKind == JsonValueKind.Object)
        {
            HashSet<string> names = new(StringComparer.Ordinal);
            foreach (JsonProperty property in element.EnumerateObject())
            {
                if (!names.Add(property.Name))
                {
                    throw new JsonException("Duplicate manifest property.");
                }
                CheckProperties(property.Value);
            }
        }
        else if (element.ValueKind == JsonValueKind.Array)
        {
            foreach (JsonElement item in element.EnumerateArray())
            {
                CheckProperties(item);
            }
        }
    }
}
