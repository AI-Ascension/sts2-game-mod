// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Text;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.WorkshopValidationProbe;

internal static class Program
{
    private const uint AppId = 480;
    private const ulong PublishedFileId = 123456789;
    private const string GameVersion = "0.107.1";
    private const string Platform = "windows-x86_64";

    private static int Main()
    {
        string root = Path.Combine(Path.GetTempPath(), $"sts2-workshop-probe-{Guid.NewGuid():N}");
        try
        {
            ValidateExternalPackageIfRequested();
            Directory.CreateDirectory(root);
            WritePayload(root);
            WorkshopFile[] files = CreateInventory(root);
            WorkshopManifest manifest = new()
            {
                SchemaVersion = "sts2-workshop-manifest-v1",
                PackageId = "ai-ascension.sts2-game-mod",
                PackageVersion = "0.1.0",
                ConsumerAppId = AppId,
                PublishedFileId = PublishedFileId,
                GameVersion = GameVersion,
                Platform = Platform,
                LoaderContract = "sts2-managed-loader-v1",
                ContentKind = "first_party_executable",
                Entrypoint = "AIAscensionSTS2GameMod.json",
                Files = files,
                ContentDigest = WorkshopPackageValidator.ComputeContentDigest(files),
                SourceRevision = "commit-123"
            };
            File.WriteAllText(
                Path.Combine(root, WorkshopPackageValidator.ManifestFileName),
                JsonSerializer.Serialize(manifest, WorkshopPackageValidator.JsonOptions));
            WriteChecksumInventory(root, files);

            WorkshopPackageValidationResult result = WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, Platform);
            if (result.Manifest.PackageId != "ai-ascension.sts2-game-mod")
            {
                Console.Error.WriteLine("valid Workshop package returned the wrong identity");
                return 1;
            }
            ExpectFailure("unsupported_platform", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, "freebsd-x86_64"));

            NegativeCases.Run(root, () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, Platform));
            File.WriteAllText(Path.Combine(root, "unexpected.txt"), "reject me\n");
            ExpectFailure("unexpected_file", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, Platform));
            File.Delete(Path.Combine(root, "unexpected.txt"));

            string checksumPath = Path.Combine(root, WorkshopPackageValidator.ChecksumFileName);
            string checksums = File.ReadAllText(checksumPath);
            File.WriteAllText(checksumPath, new string('0', 64) + checksums[64..]);
            ExpectFailure("checksum_mismatch", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, Platform));
            File.WriteAllText(checksumPath, checksums);

            File.AppendAllText(Path.Combine(root, "AIAscensionSTS2GameMod.json"), "changed\n");
            ExpectFailure("file_digest_mismatch", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, Platform));
            RunPlatformCase("linux-x86_64", "libAIAscensionSTS2GameModNative.so");
            Console.WriteLine("Managed Workshop validation probe passed.");
            return 0;
        }
        catch (Exception exception)
        {
            Console.Error.WriteLine($"managed Workshop validation probe failed: {exception.Message}");
            return 1;
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }

    private static void ValidateExternalPackageIfRequested()
    {
        string? packageDirectory = Environment.GetEnvironmentVariable("STS2_WORKSHOP_PROBE_PACKAGE");
        if (string.IsNullOrWhiteSpace(packageDirectory))
        {
            return;
        }

        string platform = Environment.GetEnvironmentVariable("STS2_WORKSHOP_PROBE_PLATFORM")
            ?? throw new InvalidOperationException("STS2_WORKSHOP_PROBE_PLATFORM is required with a package.");
        if (!uint.TryParse(Environment.GetEnvironmentVariable("STS2_WORKSHOP_PROBE_APP_ID"), out uint appId)
            || !ulong.TryParse(Environment.GetEnvironmentVariable("STS2_WORKSHOP_PROBE_ITEM_ID"), out ulong itemId)
            || appId != 2868840
            || itemId == 0)
        {
            throw new InvalidOperationException("external Workshop probe identity is invalid.");
        }

        string gameVersion = Environment.GetEnvironmentVariable("STS2_WORKSHOP_PROBE_GAME_VERSION") ?? "0.107.1";
        WorkshopPackageValidationResult result = WorkshopPackageValidator.ValidateDirectory(
            packageDirectory, appId, itemId, gameVersion, platform);
        Console.WriteLine($"Managed external Workshop package validated: {result.Manifest.Platform}");
    }

    private static void RunPlatformCase(string platform, string nativeName)
    {
        string root = Path.Combine(Path.GetTempPath(), $"sts2-workshop-probe-{Guid.NewGuid():N}");
        try
        {
            Directory.CreateDirectory(root);
            WritePayload(root, nativeName);
            WorkshopFile[] files = CreateInventory(root, nativeName);
            WorkshopManifest manifest = new()
            {
                SchemaVersion = "sts2-workshop-manifest-v1",
                PackageId = "ai-ascension.sts2-game-mod",
                PackageVersion = "0.1.0",
                ConsumerAppId = AppId,
                PublishedFileId = PublishedFileId,
                GameVersion = GameVersion,
                Platform = platform,
                LoaderContract = "sts2-managed-loader-v1",
                ContentKind = "first_party_executable",
                Entrypoint = "AIAscensionSTS2GameMod.json",
                Files = files,
                ContentDigest = WorkshopPackageValidator.ComputeContentDigest(files),
                SourceRevision = "commit-123"
            };
            File.WriteAllText(
                Path.Combine(root, WorkshopPackageValidator.ManifestFileName),
                JsonSerializer.Serialize(manifest, WorkshopPackageValidator.JsonOptions));
            WriteChecksumInventory(root, files);
            WorkshopPackageValidator.ValidateDirectory(root, AppId, PublishedFileId, GameVersion, platform);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }

    private static void WritePayload(string root)
    {
        WritePayload(root, "AIAscensionSTS2GameModNative.dll");
    }

    private static void WritePayload(string root, string nativeName)
    {
        File.WriteAllText(Path.Combine(root, "AIAscensionSTS2GameMod.dll"), "synthetic managed\n");
        File.WriteAllText(Path.Combine(root, "AIAscensionSTS2GameMod.json"), "{\"id\":\"synthetic\"}\n");
        File.WriteAllText(Path.Combine(root, nativeName), "synthetic native\n");
    }

    private static WorkshopFile[] CreateInventory(string root)
    {
        return CreateInventory(root, "AIAscensionSTS2GameModNative.dll");
    }

    private static WorkshopFile[] CreateInventory(string root, string nativeName)
    {
        return
        [
            CreateFile(root, "AIAscensionSTS2GameMod.dll", "managed_assembly"),
            CreateFile(root, "AIAscensionSTS2GameMod.json", "loader_manifest"),
            CreateFile(root, nativeName, "native_library")
        ];
    }

    private static WorkshopFile CreateFile(string root, string path, string role)
    {
        FileInfo file = new(Path.Combine(root, path));
        return new WorkshopFile
        {
            Path = path,
            Role = role,
            SizeBytes = (ulong)file.Length,
            Sha256 = WorkshopPackageValidator.ComputeFileDigest(file.FullName)
        };
    }

    private static void WriteChecksumInventory(string root, WorkshopFile[] files)
    {
        StringBuilder checksums = new();
        foreach (WorkshopFile file in files)
        {
            checksums.Append(WorkshopPackageValidator.ComputeFileDigest(Path.Combine(root, file.Path!)));
            checksums.Append("  ");
            checksums.Append(file.Path);
            checksums.Append('\n');
        }

        string manifestPath = Path.Combine(root, WorkshopPackageValidator.ManifestFileName);
        checksums.Append(WorkshopPackageValidator.ComputeFileDigest(manifestPath));
        checksums.Append("  ");
        checksums.Append(WorkshopPackageValidator.ManifestFileName);
        checksums.Append('\n');
        File.WriteAllText(Path.Combine(root, WorkshopPackageValidator.ChecksumFileName), checksums.ToString());
    }

    private static void ExpectFailure(string code, Action action)
    {
        try
        {
            action();
            throw new InvalidOperationException($"expected Workshop validation failure: {code}");
        }
        catch (WorkshopPackageValidationException exception) when (exception.Code == code)
        {
        }
    }
}
