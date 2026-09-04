// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
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
