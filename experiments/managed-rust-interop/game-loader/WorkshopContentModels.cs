// SPDX-License-Identifier: MIT

using System;
using System.Text.Json.Serialization;

namespace AiAscension.Sts2GameMod.Runtime;

public sealed class WorkshopManifest
{
    [JsonPropertyName("schema_version")]
    public string? SchemaVersion { get; set; }

    [JsonPropertyName("package_id")]
    public string? PackageId { get; set; }

    [JsonPropertyName("package_version")]
    public string? PackageVersion { get; set; }

    [JsonPropertyName("consumer_app_id")]
    public uint ConsumerAppId { get; set; }

    [JsonPropertyName("published_file_id")]
    public ulong PublishedFileId { get; set; }

    [JsonPropertyName("game_version")]
    public string? GameVersion { get; set; }

    [JsonPropertyName("platform")]
    public string? Platform { get; set; }

    [JsonPropertyName("loader_contract")]
    public string? LoaderContract { get; set; }

    [JsonPropertyName("content_kind")]
    public string? ContentKind { get; set; }

    [JsonPropertyName("entrypoint")]
    public string? Entrypoint { get; set; }

    [JsonPropertyName("files")]
    public WorkshopFile[]? Files { get; set; }

    [JsonPropertyName("content_digest")]
    public string? ContentDigest { get; set; }

    [JsonPropertyName("source_revision")]
    public string? SourceRevision { get; set; }
}

public sealed class WorkshopFile
{
    [JsonPropertyName("path")]
    public string? Path { get; set; }

    [JsonPropertyName("role")]
    public string? Role { get; set; }

    [JsonPropertyName("size_bytes")]
    public ulong SizeBytes { get; set; }

    [JsonPropertyName("sha256")]
    public string? Sha256 { get; set; }
}

public sealed class WorkshopPackageValidationResult
{
    public required string InstallDirectory { get; init; }

    public required WorkshopManifest Manifest { get; init; }
}

public sealed class WorkshopPackageValidationException : Exception
{
    public WorkshopPackageValidationException(string code, string message)
        : base(message)
    {
        Code = code;
    }

    public string Code { get; }
}
