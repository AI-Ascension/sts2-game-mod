// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace AiAscension.Sts2GameMod.Runtime;

public static class WorkshopPackageValidator
{
    public const string ManifestFileName = "sts2-workshop-manifest.json";
    public const string ChecksumFileName = "SHA256SUMS";
    private const string SchemaVersion = "sts2-workshop-manifest-v1";
    private const string PackageId = "ai-ascension.sts2-game-mod";
    private const string LoaderContract = "sts2-managed-loader-v1";
    private const string ContentKind = "first_party_executable";
    private const string Entrypoint = "AIAscensionSTS2GameMod.json";
    private const int MaximumManifestBytes = 64 * 1024;
    private const long MaximumPayloadBytes = 256 * 1024 * 1024;

    private static readonly (string Path, string Role)[] ExpectedPayload =
    [
        ("AIAscensionSTS2GameMod.dll", "managed_assembly"),
        ("AIAscensionSTS2GameMod.json", "loader_manifest"),
        ("AIAscensionSTS2GameModNative.dll", "native_library")
    ];

    private static readonly string[] ExpectedEntries =
    [
        ExpectedPayload[0].Path,
        ExpectedPayload[1].Path,
        ExpectedPayload[2].Path,
        ManifestFileName,
        ChecksumFileName
    ];

    internal static JsonSerializerOptions JsonOptions { get; } = new()
    {
        PropertyNameCaseInsensitive = false,
        UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow,
        MaxDepth = 16
    };

    public static WorkshopPackageValidationResult ValidateDirectory(
        string installDirectory,
        uint expectedConsumerAppId,
        ulong expectedPublishedFileId,
        string expectedGameVersion,
        string expectedPlatform)
    {
        if (expectedConsumerAppId == 0 || expectedPublishedFileId == 0)
        {
            Reject("invalid_trust_policy", "Workshop App ID and published file ID must be positive.");
        }

        string root = GetFullPath(installDirectory);
        if (!Directory.Exists(root))
        {
            Reject("missing_install_directory", "Workshop install directory does not exist.");
        }

        RejectUnexpectedEntries(root);
        string manifestPath = Path.Combine(root, ManifestFileName);
        byte[] manifestBytes = File.ReadAllBytes(manifestPath);
        if (manifestBytes.Length > MaximumManifestBytes)
        {
            Reject("manifest_too_large", "Workshop manifest exceeds its byte bound.");
        }

        WorkshopManifest manifest;
        try
        {
            manifest = JsonSerializer.Deserialize<WorkshopManifest>(manifestBytes, JsonOptions)
                ?? throw new JsonException("manifest is null");
        }
        catch (JsonException exception)
        {
            Reject("malformed_manifest", $"Workshop manifest is invalid: {exception.Message}");
            throw new InvalidOperationException("unreachable");
        }

        ValidateManifest(root, manifest, expectedConsumerAppId, expectedPublishedFileId, expectedGameVersion, expectedPlatform);
        return new WorkshopPackageValidationResult
        {
            InstallDirectory = root,
            Manifest = manifest
        };
    }

    internal static string ComputeFileDigest(string path)
    {
        FileInfo file = new(path);
        if (file.Length <= 0 || file.Length > MaximumPayloadBytes)
        {
            Reject("payload_size", $"Workshop payload has an invalid size: {file.Name}");
        }

        using FileStream stream = new(path, FileMode.Open, FileAccess.Read, FileShare.Read);
        return Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant();
    }

    internal static string ComputeContentDigest(IReadOnlyList<WorkshopFile> files)
    {
        StringBuilder canonical = new();
        foreach (WorkshopFile file in files)
        {
            string path = file.Path ?? string.Empty;
            canonical.Append(path);
            canonical.Append('\t');
            canonical.Append(file.SizeBytes);
            canonical.Append('\t');
            canonical.Append(file.Sha256);
            canonical.Append('\n');
        }

        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(canonical.ToString()))).ToLowerInvariant();
    }

    private static void RejectUnexpectedEntries(string root)
    {
        foreach (string entry in Directory.EnumerateFileSystemEntries(root, "*", SearchOption.TopDirectoryOnly))
        {
            FileAttributes attributes = File.GetAttributes(entry);
            if (attributes.HasFlag(FileAttributes.ReparsePoint))
            {
                Reject("reparse_point", $"Workshop package contains a reparse point: {Path.GetFileName(entry)}");
            }

            string name = Path.GetFileName(entry);
            if (!ExpectedEntries.Contains(name, StringComparer.Ordinal) || !File.Exists(entry))
            {
                Reject("unexpected_file", $"Workshop package contains an unsupported entry: {name}");
            }
        }

        foreach (string expected in ExpectedEntries)
        {
            string path = Path.Combine(root, expected);
            if (!File.Exists(path))
            {
                Reject("missing_file", $"Workshop package is missing: {expected}");
            }
        }
    }

    private static void ValidateManifest(
        string root,
        WorkshopManifest manifest,
        uint expectedConsumerAppId,
        ulong expectedPublishedFileId,
        string expectedGameVersion,
        string expectedPlatform)
    {
        if (manifest.SchemaVersion != SchemaVersion
            || manifest.PackageId != PackageId
            || manifest.LoaderContract != LoaderContract
            || manifest.ContentKind != ContentKind
            || manifest.Entrypoint != Entrypoint)
        {
            Reject("manifest_identity", "Workshop manifest identity does not match the loader contract.");
        }
        if (manifest.PackageVersion is null || !IsSafeToken(manifest.PackageVersion, false)
            || manifest.GameVersion != expectedGameVersion
            || manifest.Platform != expectedPlatform
            || !IsSafeToken(expectedGameVersion, false)
            || !IsSafeToken(expectedPlatform, false))
        {
            Reject("compatibility", "Workshop manifest compatibility metadata is not supported.");
        }
        if (manifest.ConsumerAppId != expectedConsumerAppId
            || manifest.PublishedFileId != expectedPublishedFileId)
        {
            Reject("identity_mismatch", "Workshop manifest App ID or published file ID does not match policy.");
        }
        if (manifest.ContentDigest is null || !IsSha256(manifest.ContentDigest)
            || manifest.SourceRevision is null || !IsSafeToken(manifest.SourceRevision, true))
        {
            Reject("provenance", "Workshop manifest provenance or digest is invalid.");
        }
        if (manifest.Files is null || manifest.Files.Length != ExpectedPayload.Length)
        {
            Reject("file_allowlist", "Workshop manifest file inventory does not match the first-party allowlist.");
        }

        for (int index = 0; index < ExpectedPayload.Length; index++)
        {
            WorkshopFile? file = manifest.Files[index];
            (string expectedPath, string expectedRole) = ExpectedPayload[index];
            if (file is null || file.Path != expectedPath || file.Role != expectedRole
                || file.Sha256 is null || !IsSha256(file.Sha256))
            {
                Reject("file_allowlist", "Workshop manifest file inventory is not the exact first-party allowlist.");
            }

            string payloadPath = Path.Combine(root, expectedPath);
            string digest = ComputeFileDigest(payloadPath);
            long byteLength = new FileInfo(payloadPath).Length;
            if (file.SizeBytes != (ulong)byteLength || file.Sha256 != digest)
            {
                Reject("file_digest_mismatch", $"Workshop payload digest does not match: {expectedPath}");
            }
        }

        string contentDigest = ComputeContentDigest(manifest.Files);
        if (!string.Equals(manifest.ContentDigest, contentDigest, StringComparison.Ordinal))
        {
            Reject("content_digest_mismatch", "Workshop content digest does not match the file inventory.");
        }
    }

    private static string GetFullPath(string path)
    {
        try
        {
            return Path.GetFullPath(path);
        }
        catch (Exception exception) when (exception is ArgumentException or IOException or UnauthorizedAccessException)
        {
            Reject("unsafe_install_path", "Workshop install path cannot be resolved safely.");
            throw new InvalidOperationException("unreachable");
        }
    }

    private static bool IsSafeToken(string value, bool allowSlash)
    {
        return value.Length > 0
            && value.Length <= 256
            && !value.Contains("..", StringComparison.Ordinal)
            && value.All(character => char.IsAsciiLetterOrDigit(character)
                || character is '.' or '_' or ':' or '-'
                || (allowSlash && character == '/'));
    }

    private static bool IsSha256(string value)
    {
        return value.Length == 64 && value.All(character =>
            (character is >= '0' and <= '9') || (character is >= 'a' and <= 'f'));
    }

    [DoesNotReturn]
    private static void Reject(string code, string message)
    {
        throw new WorkshopPackageValidationException(code, message);
    }
}
