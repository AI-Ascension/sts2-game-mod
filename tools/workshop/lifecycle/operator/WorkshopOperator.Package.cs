// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;

internal static partial class Program
{
    private static PackageInfo VerifyPackage(string value, string platform, uint appId, ulong itemId)
    {
        string package = RegularDirectory(value, "package directory");
        string[] payload = Payload(platform);
        string[] expected = [.. payload, "sts2-workshop-manifest.json", "SHA256SUMS"];
        string[] entries = new DirectoryInfo(package).GetFileSystemInfos()
            .Select(entry => entry.Name)
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToArray();
        if (!entries.SequenceEqual(
                expected.OrderBy(name => name, StringComparer.Ordinal),
                StringComparer.Ordinal))
            throw new OperatorException($"package must contain exactly {expected.Length} allowlisted files");
        foreach (string name in expected)
        {
            string path = RegularFile(Path.Combine(package, name), $"package entry {name}");
            if (new FileInfo(path).Length == 0)
                throw new OperatorException($"package entry must not be empty: {name}");
        }

        string manifestPath = Path.Combine(package, "sts2-workshop-manifest.json");
        using JsonDocument manifest = ParseUniqueJson(manifestPath);
        JsonElement root = manifest.RootElement;
        RequireString(root, "schema_version", PackageSchema);
        RequireString(root, "package_id", PackageId);
        RequireString(root, "loader_contract", LoaderContract);
        RequireString(root, "content_kind", "first_party_executable");
        RequireString(root, "entrypoint", Entrypoint);
        RequireUInt(root, "consumer_app_id", appId);
        RequireUInt(root, "published_file_id", itemId);
        RequireString(root, "platform", platform);

        JsonElement manifestFiles = RequiredProperty(root, "files");
        if (manifestFiles.ValueKind != JsonValueKind.Array
            || manifestFiles.GetArrayLength() != payload.Length)
            throw new OperatorException("manifest files must contain exactly three payload records");
        PackageFile[] files = new PackageFile[payload.Length];
        string[] roles = ["managed_assembly", "loader_manifest", "native_library"];
        for (int index = 0; index < payload.Length; index++)
        {
            JsonElement record = manifestFiles[index];
            string name = payload[index];
            string path = Path.Combine(package, name);
            string actual = Digest(path);
            long size = new FileInfo(path).Length;
            if (record.ValueKind != JsonValueKind.Object
                || StringProperty(record, "path") != name
                || StringProperty(record, "role") != roles[index]
                || IntProperty(record, "size_bytes") != size
                || !string.Equals(StringProperty(record, "sha256"), actual, StringComparison.OrdinalIgnoreCase))
                throw new OperatorException($"manifest file record {index} does not match payload");
            files[index] = new PackageFile
            {
                Path = name,
                Role = roles[index],
                SizeBytes = size,
                Sha256 = actual
            };
        }
        string contentDigest = CanonicalDigest(files);
        if (StringProperty(root, "content_digest") != contentDigest)
            throw new OperatorException("manifest content_digest does not match the payload inventory");

        string[] checksumNames = [.. payload, "sts2-workshop-manifest.json"];
        string[] checksumLines = File.ReadAllLines(Path.Combine(package, "SHA256SUMS"));
        if (checksumLines.Length != checksumNames.Length)
            throw new OperatorException("SHA256SUMS must contain four ordered rows");
        for (int index = 0; index < checksumLines.Length; index++)
        {
            (string hash, string name) = ParseChecksum(checksumLines[index]);
            if (name != checksumNames[index]
                || !string.Equals(hash, Digest(Path.Combine(package, name)), StringComparison.OrdinalIgnoreCase))
                throw new OperatorException($"SHA256SUMS digest or order is invalid for {name}");
        }
        return new PackageInfo
        {
            PackageDirectory = package,
            Platform = platform,
            AppId = appId,
            PublishedFileId = itemId,
            PackageVersion = OptionalString(root, "package_version"),
            GameVersion = OptionalString(root, "game_version"),
            SourceRevision = OptionalString(root, "source_revision"),
            ManifestSha256 = Digest(manifestPath),
            ContentDigest = contentDigest,
            Files = files
        };
    }

    private static void VerifyVdf(string value, PackageInfo package, uint appId, ulong itemId)
    {
        string path = RegularFile(value, "VDF");
        string[] lines = File.ReadAllLines(path);
        string[] nonEmpty = lines
            .Select(line => line.Trim())
            .Where(line => line.Length != 0)
            .ToArray();
        if (nonEmpty.Length < 3 || nonEmpty[0] != "\"workshopitem\""
            || nonEmpty[1] != "{" || nonEmpty[^1] != "}")
            throw new OperatorException("VDF must have a workshopitem root");

        Dictionary<string, string> fields = new(StringComparer.Ordinal);
        foreach (string line in lines)
        {
            string trimmed = line.Trim();
            if (trimmed.Length == 0 || trimmed is "{" or "}" || trimmed == "\"workshopitem\"")
                continue;
            if (!TryParseVdfField(trimmed, out string key, out string fieldValue))
                throw new OperatorException("VDF contains an invalid field");
            if (!fields.TryAdd(key, fieldValue))
                throw new OperatorException($"VDF contains a duplicate field: {key}");
        }
        string[] required = [
            "appid", "publishedfileid", "contentfolder", "previewfile",
            "visibility", "title", "description", "changenote"
        ];
        foreach (string name in required)
        {
            if (!fields.ContainsKey(name))
                throw new OperatorException($"VDF is missing a required field: {name}");
        }
        if (ParseDecimal(fields["appid"], "VDF app ID", uint.MaxValue) != appId
            || ParseDecimal(fields["publishedfileid"], "VDF published file ID", ulong.MaxValue) != itemId)
            throw new OperatorException("VDF IDs do not match the verified package");
        if (AbsoluteLexical(fields["contentfolder"]) != AbsoluteLexical(package.PackageDirectory))
            throw new OperatorException("VDF contentfolder does not match the verified package");
        _ = RegularFile(fields["previewfile"], "VDF preview file");
        if (fields["visibility"] is not ("0" or "1" or "2" or "3"))
            throw new OperatorException("VDF visibility is outside Steam's enum");
    }

    private static (string Hash, string Name) ParseChecksum(string line)
    {
        if (line.Length < 67 || line[64..66] != "  ")
            throw new OperatorException("checksum row must contain a 64-character digest and safe path");
        string hash = line[..64];
        string name = line[66..];
        if (!IsSha256(hash) || name.Length == 0
            || name.Any(character => !(char.IsLetterOrDigit(character)
                || character is '.' or '_' or '-')))
            throw new OperatorException("checksum row contains an unsafe path or digest");
        return (hash, name);
    }

    private static bool TryParseVdfField(string line, out string key, out string value)
    {
        key = "";
        value = "";
        int index = 0;
        if (!TryParseQuoted(line, ref index, out key))
            return false;
        SkipWhitespace(line, ref index);
        if (!TryParseQuoted(line, ref index, out value))
            return false;
        SkipWhitespace(line, ref index);
        return index == line.Length;
    }

    private static bool TryParseQuoted(string line, ref int index, out string value)
    {
        value = "";
        SkipWhitespace(line, ref index);
        if (index >= line.Length || line[index++] != '"')
            return false;
        System.Text.StringBuilder result = new();
        while (index < line.Length)
        {
            char character = line[index++];
            if (character == '"')
            {
                value = result.ToString();
                return true;
            }
            if (character != '\\')
            {
                result.Append(character);
                continue;
            }
            if (index >= line.Length || line[index] is not ('"' or '\\'))
                return false;
            result.Append(line[index++]);
        }
        return false;
    }

    private static void SkipWhitespace(string value, ref int index)
    {
        while (index < value.Length && char.IsWhiteSpace(value[index]))
            index++;
    }

    private static JsonElement RequiredProperty(JsonElement root, string name)
    {
        if (!root.TryGetProperty(name, out JsonElement value))
            throw new OperatorException($"manifest is missing property: {name}");
        return value;
    }

    private static string StringProperty(JsonElement root, string name)
    {
        JsonElement value = RequiredProperty(root, name);
        if (value.ValueKind != JsonValueKind.String || value.GetString() is not string text)
            throw new OperatorException($"manifest property is not a string: {name}");
        return text;
    }

    private static string? OptionalString(JsonElement root, string name)
        => root.TryGetProperty(name, out JsonElement value) && value.ValueKind == JsonValueKind.String
            ? value.GetString()
            : null;

    private static long IntProperty(JsonElement root, string name)
    {
        JsonElement value = RequiredProperty(root, name);
        if (!value.TryGetInt64(out long number))
            throw new OperatorException($"manifest property is not an integer: {name}");
        return number;
    }

    private static void RequireUInt(JsonElement root, string name, ulong expected)
    {
        JsonElement value = RequiredProperty(root, name);
        if (!value.TryGetUInt64(out ulong actual) || actual != expected)
            throw new OperatorException($"manifest {name} does not match the operation");
    }

    private static void RequireString(JsonElement root, string name, string expected)
    {
        if (StringProperty(root, name) != expected)
            throw new OperatorException($"manifest {name} does not match the operation");
    }
}
