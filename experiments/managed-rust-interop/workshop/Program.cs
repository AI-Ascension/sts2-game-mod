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
    private const string Platform = "windows-x86_64";

    private static int Main()
    {
        string root = Path.Combine(Path.GetTempPath(), $"sts2-workshop-probe-{Guid.NewGuid():N}");
        try
        {
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
            File.WriteAllText(Path.Combine(root, WorkshopPackageValidator.ChecksumFileName), "synthetic fixture\n");

            WorkshopPackageValidationResult result = WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, Platform);
            if (result.Manifest.PackageId != "ai-ascension.sts2-game-mod")
            {
                Console.Error.WriteLine("valid Workshop package returned the wrong identity");
                return 1;
            }

            NegativeCases.Run(root, () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, Platform));
            File.WriteAllText(Path.Combine(root, "unexpected.txt"), "reject me\n");
            ExpectFailure("unexpected_file", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, Platform));
            File.Delete(Path.Combine(root, "unexpected.txt"));

            File.AppendAllText(Path.Combine(root, "AIAscensionSTS2GameMod.json"), "changed\n");
            ExpectFailure("file_digest_mismatch", () => WorkshopPackageValidator.ValidateDirectory(
                root, AppId, PublishedFileId, GameVersion, Platform));
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

    private static void WritePayload(string root)
    {
        File.WriteAllText(Path.Combine(root, "AIAscensionSTS2GameMod.dll"), "synthetic managed\n");
        File.WriteAllText(Path.Combine(root, "AIAscensionSTS2GameMod.json"), "{\"id\":\"synthetic\"}\n");
        File.WriteAllText(Path.Combine(root, "AIAscensionSTS2GameModNative.dll"), "synthetic native\n");
    }

    private static WorkshopFile[] CreateInventory(string root)
    {
        return
        [
            CreateFile(root, "AIAscensionSTS2GameMod.dll", "managed_assembly"),
            CreateFile(root, "AIAscensionSTS2GameMod.json", "loader_manifest"),
            CreateFile(root, "AIAscensionSTS2GameModNative.dll", "native_library")
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
