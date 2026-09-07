// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;

internal static partial class Program
{
    private static int Main(string[] values)
    {
        if (values.Length == 0)
        {
            Console.Error.WriteLine(UsageText());
            return 64;
        }
        try
        {
            string operation = values[0];
            if (operation is "-h" or "--help")
                throw new OperatorException(UsageText());
            ParsedArguments args = ParseArguments(values[1..]);
            return operation switch
            {
                "verify-package" => VerifyCommand(args),
                "upload" or "update" or "rollback" => UploadCommand(args, operation),
                "download" => DownloadCommand(args),
                "subscribe" => SubscribeCommand(args),
                _ => throw new OperatorException($"unsupported operation: {operation}")
            };
        }
        catch (OperatorException exception)
        {
            Console.Error.WriteLine($"workshop operator: {exception.Message}");
            return 64;
        }
        catch (Exception exception) when (exception is IOException
            or UnauthorizedAccessException or JsonException or FormatException
            or ArgumentException)
        {
            Console.Error.WriteLine($"workshop operator: {exception.GetType().Name}: {exception.Message}");
            return 64;
        }
    }

    private static int VerifyCommand(ParsedArguments args)
    {
        string platform = ParsePlatform(args);
        (uint appId, ulong itemId) = ParseIds(args, allowZeroItem: true);
        PackageInfo package = VerifyPackage(
            Required(args, "--package-dir"),
            platform,
            appId,
            itemId);
        Console.WriteLine(Serialize(package));
        return 0;
    }

    private static int UploadCommand(ParsedArguments args, string operation)
    {
        string platform = ParsePlatform(args);
        (uint appId, ulong itemId) = ParseIds(args, allowZeroItem: false);
        PackageInfo package = VerifyPackage(
            Required(args, "--package-dir"),
            platform,
            appId,
            itemId);
        string vdf = RegularFile(Required(args, "--vdf"), "VDF");
        VerifyVdf(vdf, package, appId, itemId);
        string steamCmd = RegularFile(Required(args, "--steamcmd"), "SteamCMD executable");
        string journal = AbsoluteLexical(Required(args, "--journal"));
        RequireOutside(package.PackageDirectory, vdf, "VDF");
        RequireOutside(package.PackageDirectory, journal, "operation journal");
        string[] command = [
            steamCmd, "+login", "<STEAMCMD_USER>", "+workshop_build_item", vdf, "+quit"
        ];
        return RunJournaled(
            operation,
            appId,
            itemId,
            journal,
            command,
            package,
            platform,
            null,
            null,
            HasFlag(args, "--execute-public-write"),
            ParseTimeout(args),
            null);
    }

    private static int DownloadCommand(ParsedArguments args)
    {
        string platform = ParsePlatform(args);
        (uint appId, ulong itemId) = ParseIds(args, allowZeroItem: false);
        string steamCmd = RegularFile(Required(args, "--steamcmd"), "SteamCMD executable");
        string root = RegularDirectory(Required(args, "--steamcmd-dir"), "SteamCMD directory");
        string target = Path.Combine(
            root, "steamapps", "workshop", "content", Invariant(appId), Invariant(itemId));
        RejectSymlinkComponents(target, "download target");
        if (File.Exists(target) || Directory.Exists(target)
            || new FileInfo(target).LinkTarget is not null
            || new DirectoryInfo(target).LinkTarget is not null)
            throw new OperatorException(
                "download target already exists; use an isolated clean SteamCMD directory");
        string journal = AbsoluteLexical(Required(args, "--journal"));
        string[] command = [
            steamCmd, "+login", "<STEAMCMD_USER>", "+workshop_download_item",
            Invariant(appId), Invariant(itemId), "+quit"
        ];
        return RunJournaled(
            "download",
            appId,
            itemId,
            journal,
            command,
            null,
            platform,
            target,
            null,
            HasFlag(args, "--execute-local-download"),
            ParseTimeout(args),
            () => _ = VerifyPackage(target, platform, appId, itemId));
    }

    private static int SubscribeCommand(ParsedArguments args)
    {
        string platform = ParsePlatformIfPresent(args);
        (uint appId, ulong itemId) = ParseIds(args, allowZeroItem: false);
        string runner = RegularFile(Required(args, "--runner"), "reviewed native UGC runner");
        string runnerSha = Required(args, "--runner-sha256").ToLowerInvariant();
        if (!IsSha256(runnerSha) || Digest(runner) != runnerSha)
            throw new OperatorException("native UGC runner SHA-256 does not match");
        string library = RegularFile(Required(args, "--library"), "Steam API library");
        string librarySha = Digest(library);
        if (!string.Equals(
                librarySha,
                Required(args, "--library-sha256"),
                StringComparison.OrdinalIgnoreCase))
            throw new OperatorException("Steam API library SHA-256 does not match");
        string result = EnsureAbsent(Required(args, "--result"), "native UGC result");
        string runnerJournal = EnsureAbsent(
            Required(args, "--runner-journal"), "native UGC journal");
        string journal = AbsoluteLexical(Required(args, "--journal"));
        string[] command = [
            runner, "--operation", "subscribe", "--library", library,
            "--expected-sha256", librarySha, "--app-id", Invariant(appId),
            "--item-id", Invariant(itemId), "--output", result,
            "--journal", runnerJournal, "--timeout-seconds", Invariant(ParseTimeout(args))
        ];
        Dictionary<string, string> environment = new(StringComparer.Ordinal)
        {
            ["SteamAppId"] = Invariant(appId),
            ["SteamGameId"] = Invariant(appId)
        };
        Action postcondition = () =>
        {
            using JsonDocument value = ParseUniqueJson(result);
            if (!value.RootElement.TryGetProperty("outcome", out JsonElement outcome)
                || outcome.ValueKind != JsonValueKind.String
                || outcome.GetString() != "succeeded")
                throw new OperatorException("native UGC runner did not report outcome=succeeded");
        };
        return RunJournaled(
            "subscribe",
            appId,
            itemId,
            journal,
            command,
            null,
            platform,
            null,
            environment,
            HasFlag(args, "--execute-ugc"),
            ParseTimeout(args),
            postcondition);
    }

    private static string ParsePlatformIfPresent(ParsedArguments args)
    {
        string platform = Optional(args, "--platform", "linux-x86_64");
        if (!Platforms.Contains(platform, StringComparer.Ordinal))
            throw new OperatorException($"unsupported platform: {platform}");
        return platform;
    }

    private static void RequireOutside(string parent, string candidate, string label)
    {
        string parentPath = AbsoluteLexical(parent).TrimEnd(Path.DirectorySeparatorChar)
            + Path.DirectorySeparatorChar;
        string candidatePath = AbsoluteLexical(candidate);
        if (candidatePath == parentPath[..^1]
            || candidatePath.StartsWith(parentPath, StringComparison.Ordinal))
            throw new OperatorException($"{label} must be outside the package directory");
    }
}
