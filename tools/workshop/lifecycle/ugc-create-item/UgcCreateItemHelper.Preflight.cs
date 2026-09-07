// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;

// This is the deliberately separate mutating gate for one new Workshop item.
// It creates an empty item only. Upload and update remain SteamCMD operations
// guarded by tools/workshop/lifecycle/workshop_operator.

internal static partial class Program
{
    private static PackageSnapshot? VerifyPackage(Options options)
    {
        string native = options.Platform == "windows-x86_64"
            ? "AIAscensionSTS2GameModNative.dll"
            : "libAIAscensionSTS2GameModNative.so";
        string[] payload = ["AIAscensionSTS2GameMod.dll", Entrypoint, native];
        string[] expected = [.. payload, "sts2-workshop-manifest.json", "SHA256SUMS"];
        try
        {
            string[] entries = Directory.GetFileSystemEntries(options.PackageDirectory)
                .Select(Path.GetFileName)
                .OrderBy(value => value, StringComparer.Ordinal)
                .ToArray()!;
            if (!entries.SequenceEqual(expected.OrderBy(value => value, StringComparer.Ordinal), StringComparer.Ordinal))
            {
                return null;
            }
            foreach (string name in expected)
            {
                FileInfo file = new(Path.Combine(options.PackageDirectory, name));
                if (!file.Exists || file.LinkTarget is not null || file.Length == 0)
                {
                    return null;
                }
            }

            using JsonDocument manifest = JsonDocument.Parse(File.ReadAllText(
                Path.Combine(options.PackageDirectory, "sts2-workshop-manifest.json")));
            JsonElement root = manifest.RootElement;
            if (root.GetProperty("schema_version").GetString() != PackageSchema
                || root.GetProperty("package_id").GetString() != PackageId
                || root.GetProperty("loader_contract").GetString() != LoaderContract
                || root.GetProperty("content_kind").GetString() != "first_party_executable"
                || root.GetProperty("entrypoint").GetString() != Entrypoint
                || root.GetProperty("consumer_app_id").GetUInt32() != options.ExpectedAppId
                || root.GetProperty("published_file_id").GetUInt64() != 0
                || root.GetProperty("platform").GetString() != options.Platform)
            {
                return null;
            }

            JsonElement manifestFiles = root.GetProperty("files");
            if (manifestFiles.ValueKind != JsonValueKind.Array || manifestFiles.GetArrayLength() != payload.Length)
            {
                return null;
            }
            List<PackageFile> files = [];
            for (int index = 0; index < payload.Length; index++)
            {
                JsonElement record = manifestFiles[index];
                string name = payload[index];
                string role = index switch
                {
                    0 => "managed_assembly",
                    1 => "loader_manifest",
                    _ => "native_library"
                };
                string path = Path.Combine(options.PackageDirectory, name);
                string actual = FileDigest(path);
                long size = new FileInfo(path).Length;
                if (record.GetProperty("path").GetString() != name
                    || record.GetProperty("role").GetString() != role
                    || record.GetProperty("size_bytes").GetInt64() != size
                    || !string.Equals(record.GetProperty("sha256").GetString(), actual, StringComparison.OrdinalIgnoreCase))
                {
                    return null;
                }
                files.Add(new PackageFile { Path = name, Role = role, SizeBytes = size, Sha256 = actual });
            }
            string contentDigest = CanonicalDigest(files);
            if (root.GetProperty("content_digest").GetString() != contentDigest)
            {
                return null;
            }

            string[] sums = File.ReadAllLines(Path.Combine(options.PackageDirectory, "SHA256SUMS"));
            string[] checksumNames = [.. payload, "sts2-workshop-manifest.json"];
            if (sums.Length != checksumNames.Length)
            {
                return null;
            }
            for (int index = 0; index < sums.Length; index++)
            {
                string[] parts = sums[index].Split("  ", 2, StringSplitOptions.None);
                if (parts.Length != 2 || parts[1] != checksumNames[index] || !IsSha256(parts[0])
                    || !string.Equals(parts[0], FileDigest(Path.Combine(options.PackageDirectory, parts[1])), StringComparison.OrdinalIgnoreCase))
                {
                    return null;
                }
            }
            return new PackageSnapshot
            {
                ManifestSha256 = FileDigest(Path.Combine(options.PackageDirectory, "sts2-workshop-manifest.json")),
                ContentDigest = contentDigest,
                Files = [.. files]
            };
        }
        catch
        {
            return null;
        }
    }

    private static string CanonicalDigest(IEnumerable<PackageFile> files)
    {
        string canonical = string.Concat(files.Select(file =>
            $"{file.Path}\t{file.SizeBytes}\t{file.Sha256}\n"));
        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(canonical))).ToLowerInvariant();
    }

    private static string FileDigest(string path)
    {
        using FileStream stream = File.OpenRead(path);
        return Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant();
    }

}
