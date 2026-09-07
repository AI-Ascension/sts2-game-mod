// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

internal static partial class Program
{
    private const string AppIdDefault = "2868840";
    private const string PackageId = "ai-ascension.sts2-game-mod";
    private const string PackageSchema = "sts2-workshop-manifest-v1";
    private const string LoaderContract = "sts2-managed-loader-v1";
    private const string Entrypoint = "AIAscensionSTS2GameMod.json";
    private const string JournalSchema = "sts2-steam-workshop-operation-journal-v1";
    private static readonly string[] Platforms = ["windows-x86_64", "linux-x86_64"];

    private sealed class OperatorException(string message) : Exception(message);

    private sealed class ParsedArguments
    {
        internal Dictionary<string, string> Values { get; } = new(StringComparer.Ordinal);
        internal HashSet<string> Flags { get; } = new(StringComparer.Ordinal);
    }

    private sealed class PackageInfo
    {
        public string PackageDirectory { get; init; } = "";
        public string Platform { get; init; } = "";
        public uint AppId { get; init; }
        public ulong PublishedFileId { get; init; }
        public string? PackageVersion { get; init; }
        public string? GameVersion { get; init; }
        public string? SourceRevision { get; init; }
        public string ManifestSha256 { get; init; } = "";
        public string ContentDigest { get; init; } = "";
        public PackageFile[] Files { get; init; } = [];
    }

    private sealed class PackageFile
    {
        public string Path { get; init; } = "";
        public string Role { get; init; } = "";
        public long SizeBytes { get; init; }
        public string Sha256 { get; init; } = "";
    }

    private sealed class OperationJournal
    {
        public string Schema { get; set; } = JournalSchema;
        public string Operation { get; set; } = "";
        public uint AppId { get; set; }
        public ulong PublishedFileId { get; set; }
        public PackageInfo? Package { get; set; }
        public string[] Command { get; set; } = [];
        public string Journal { get; set; } = "";
        public string? Platform { get; set; }
        public string? ExpectedInstallDirectory { get; set; }
        public Dictionary<string, string>? Environment { get; set; }
        public string Phase { get; set; } = "prepared";
        public string Outcome { get; set; } = "not_started";
        public int? ProcessExitCode { get; set; }
        public string? PostconditionError { get; set; }
        public string? ExceptionType { get; set; }
        public string UpdatedAtUtc { get; set; } = "";
    }

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };

    private static ParsedArguments ParseArguments(string[] values)
    {
        HashSet<string> flags = [
            "--execute-public-write", "--execute-local-download", "--execute-ugc"
        ];
        HashSet<string> valueOptions = [
            "--app-id", "--item-id", "--package-dir", "--platform", "--steamcmd",
            "--vdf", "--journal", "--timeout", "--steamcmd-dir", "--runner",
            "--runner-sha256", "--library", "--library-sha256", "--result",
            "--runner-journal"
        ];
        ParsedArguments result = new();
        for (int index = 0; index < values.Length; index++)
        {
            string value = values[index];
            if (value is "-h" or "--help")
                throw new OperatorException(UsageText());
            if (flags.Contains(value))
            {
                if (!result.Flags.Add(value))
                    throw new OperatorException($"duplicate flag: {value}");
                continue;
            }
            if (!valueOptions.Contains(value) || index + 1 >= values.Length
                || values[index + 1].StartsWith("--", StringComparison.Ordinal))
                throw new OperatorException($"invalid or incomplete option: {value}");
            if (!result.Values.TryAdd(value, values[++index]))
                throw new OperatorException($"duplicate option: {value}");
        }
        return result;
    }

    private static string Required(ParsedArguments args, string name)
        => args.Values.TryGetValue(name, out string? value) && value.Length != 0
            ? value
            : throw new OperatorException($"{name} is required");

    private static string Optional(ParsedArguments args, string name, string fallback)
        => args.Values.GetValueOrDefault(name, fallback);

    private static bool HasFlag(ParsedArguments args, string name) => args.Flags.Contains(name);

    private static string UsageText()
        => "usage: workshop_operator verify-package|upload|update|rollback|download|subscribe [options]";

    private static string Invariant(uint value)
        => value.ToString(CultureInfo.InvariantCulture);

    private static string Invariant(ulong value)
        => value.ToString(CultureInfo.InvariantCulture);

    private static string Invariant(int value)
        => value.ToString(CultureInfo.InvariantCulture);

    private static (uint AppId, ulong ItemId) ParseIds(ParsedArguments args, bool allowZeroItem)
    {
        ulong app = ParseDecimal(Optional(args, "--app-id", AppIdDefault), "app ID", uint.MaxValue);
        ulong item = ParseDecimal(Required(args, "--item-id"), "published file ID", ulong.MaxValue);
        if (app == 0 || (!allowZeroItem && item == 0))
            throw new OperatorException("the app ID and published file ID must be non-zero");
        return ((uint)app, item);
    }

    private static ulong ParseDecimal(string value, string label, ulong maximum)
    {
        if (value.Length == 0 || (value.Length > 1 && value[0] == '0')
            || value.Any(character => character is < '0' or > '9')
            || !ulong.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out ulong number))
            throw new OperatorException($"{label} must be an unsigned decimal without leading zeroes");
        if (number > maximum)
            throw new OperatorException($"{label} exceeds its unsigned bound");
        return number;
    }

    private static int ParseTimeout(ParsedArguments args)
    {
        string value = Optional(args, "--timeout", "900");
        if (!int.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out int timeout)
            || timeout < 1 || timeout > 3600)
            throw new OperatorException("timeout must be between one and 3600 seconds");
        return timeout;
    }

    private static string ParsePlatform(ParsedArguments args)
    {
        string platform = Required(args, "--platform");
        if (!Platforms.Contains(platform, StringComparer.Ordinal))
            throw new OperatorException($"unsupported platform: {platform}");
        return platform;
    }

    private static string[] Payload(string platform)
        => platform == "windows-x86_64"
            ? ["AIAscensionSTS2GameMod.dll", Entrypoint, "AIAscensionSTS2GameModNative.dll"]
            : ["AIAscensionSTS2GameMod.dll", Entrypoint, "libAIAscensionSTS2GameModNative.so"];

    private static string AbsoluteLexical(string value) => Path.GetFullPath(value);

    private static void RejectSymlinkComponents(string value, string label)
    {
        string full = AbsoluteLexical(value);
        string root = Path.GetPathRoot(full) ?? Path.DirectorySeparatorChar.ToString();
        string current = root;
        string remainder = full[root.Length..];
        foreach (string part in remainder.Split(
                     [Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar],
                     StringSplitOptions.RemoveEmptyEntries))
        {
            current = Path.Combine(current, part);
            if (new FileInfo(current).LinkTarget is not null
                || new DirectoryInfo(current).LinkTarget is not null)
                throw new OperatorException($"{label} path contains a symlink: {current}");
        }
    }

    private static string RegularFile(string value, string label)
    {
        RejectSymlinkComponents(value, label);
        string path = AbsoluteLexical(value);
        FileInfo file = new(path);
        if (!file.Exists || file.LinkTarget is not null)
            throw new OperatorException($"{label} must be an existing regular non-symlink file: {path}");
        return path;
    }

    private static string RegularDirectory(string value, string label)
    {
        RejectSymlinkComponents(value, label);
        string path = AbsoluteLexical(value);
        DirectoryInfo directory = new(path);
        if (!directory.Exists || directory.LinkTarget is not null)
            throw new OperatorException($"{label} must be an existing regular non-symlink directory: {path}");
        return path;
    }

    private static string EnsureAbsent(string value, string label)
    {
        RejectSymlinkComponents(value, label);
        string path = AbsoluteLexical(value);
        if (File.Exists(path) || Directory.Exists(path)
            || new FileInfo(path).LinkTarget is not null
            || new DirectoryInfo(path).LinkTarget is not null)
            throw new OperatorException($"{label} already exists; reconcile it before retrying: {path}");
        return path;
    }

    private static string Digest(string path)
    {
        using FileStream stream = File.OpenRead(path);
        return Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant();
    }

    private static string CanonicalDigest(IEnumerable<PackageFile> files)
    {
        string canonical = string.Concat(files.Select(file =>
            $"{file.Path}\t{file.SizeBytes}\t{file.Sha256}\n"));
        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(canonical))).ToLowerInvariant();
    }

    private static bool IsSha256(string value)
        => value.Length == 64 && value.All(character => character is >= '0' and <= '9'
            or >= 'a' and <= 'f' or >= 'A' and <= 'F');

    private static JsonDocument ParseUniqueJson(string path)
    {
        byte[] bytes = File.ReadAllBytes(path);
        Utf8JsonReader reader = new(bytes, true, new JsonReaderState());
        Stack<HashSet<string>> objects = [];
        while (reader.Read())
        {
            if (reader.TokenType == JsonTokenType.StartObject)
                objects.Push(new HashSet<string>(StringComparer.Ordinal));
            else if (reader.TokenType == JsonTokenType.EndObject)
                objects.Pop();
            else if (reader.TokenType == JsonTokenType.PropertyName
                && (!objects.TryPeek(out HashSet<string>? properties)
                    || !properties.Add(reader.GetString()!)))
                throw new OperatorException($"JSON contains a duplicate property: {path}");
        }
        return JsonDocument.Parse(bytes);
    }

    private static string Serialize(object value)
        => JsonSerializer.Serialize(value, JsonOptions);
}
