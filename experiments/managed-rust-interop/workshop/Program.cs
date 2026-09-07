// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.WorkshopValidationProbe;

internal static class Program
{
    private const uint AppId = 480;
    private const ulong PublishedFileId = 123456789;
    private const string GameVersion = "0.107.1";
    private static int Main()
    {
        return ValidatePlatform("windows-x86_64", "AIAscensionSTS2GameModNative.dll") == 0
            && ValidatePlatform("linux-x86_64", "libAIAscensionSTS2GameModNative.so") == 0 ? 0 : 1;
    }

    private static int ValidatePlatform(string platform, string nativeLibrary)
    {
        string root = Path.Combine(Path.GetTempPath(), $"sts2-workshop-probe-{Guid.NewGuid():N}");
        try
        {
            Directory.CreateDirectory(root);
            WritePayload(root, nativeLibrary);
            WorkshopFile[] files = CreateInventory(root, nativeLibrary);
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
            File.WriteAllText(Path.Combine(root, WorkshopPackageValidator.ChecksumFileName), "synthetic fixture\n");

            WorkshopPackageValidationResult result = WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, platform);
            if (result.Manifest.PackageId != "ai-ascension.sts2-game-mod")
            {
                Console.Error.WriteLine("valid Workshop package returned the wrong identity");
                return 1;
            }

            NegativeCases.Run(root, () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, platform));
            ExpectFailure("compatibility", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, "unsupported-platform"));
            string foreignLibrary = platform == "windows-x86_64"
                ? "libAIAscensionSTS2GameModNative.so" : "AIAscensionSTS2GameModNative.dll";
            File.Move(Path.Combine(root, nativeLibrary), Path.Combine(root, foreignLibrary));
            ExpectFailure("unexpected_file", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, platform));
            File.Move(Path.Combine(root, foreignLibrary), Path.Combine(root, nativeLibrary));
            string manifestPath = Path.Combine(root, WorkshopPackageValidator.ManifestFileName);
            string originalManifest = File.ReadAllText(manifestPath);
            File.WriteAllText(manifestPath, originalManifest.Replace(platform,
                platform == "windows-x86_64" ? "linux-x86_64" : "windows-x86_64", StringComparison.Ordinal));
            ExpectFailure("compatibility", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, platform));
            File.WriteAllText(manifestPath, originalManifest);
            File.WriteAllText(Path.Combine(root, "unexpected.txt"), "reject me\n");
            ExpectFailure("unexpected_file", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, platform));
            File.Delete(Path.Combine(root, "unexpected.txt"));

            File.AppendAllText(Path.Combine(root, "AIAscensionSTS2GameMod.json"), "changed\n");
            ExpectFailure("file_digest_mismatch", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, platform));
            Console.WriteLine($"Managed Workshop validation probe passed: {platform}.");
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

    private static void WritePayload(string root, string nativeLibrary)
    {
        File.WriteAllText(Path.Combine(root, "AIAscensionSTS2GameMod.dll"), "synthetic managed\n");
        File.WriteAllText(Path.Combine(root, "AIAscensionSTS2GameMod.json"), "{\"id\":\"synthetic\"}\n");
        File.WriteAllText(Path.Combine(root, nativeLibrary), "synthetic native\n");
    }

    private static WorkshopFile[] CreateInventory(string root, string nativeLibrary)
    {
        return
        [
            CreateFile(root, "AIAscensionSTS2GameMod.dll", "managed_assembly"),
            CreateFile(root, "AIAscensionSTS2GameMod.json", "loader_manifest"),
            CreateFile(root, nativeLibrary, "native_library")
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
