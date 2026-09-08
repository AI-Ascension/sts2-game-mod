// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class Program
{
    private static readonly string[] ExpectedMapArtifactChecksums =
    {
        "0d6bc5f9268b42c852b2d4fea120593d69c748cd6fccd73d5833c7658b662091  ../../conformance/cases/runtime-map-v1.json",
        "ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b  ../../schemas/runtime-map-v1.schema.json",
        "e4a1bb88587fc67e2279651baa06334a4f1bc76edad5aec34f662f5eea1f21cf  manifest.json",
        "ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b  schema.json",
        "31b0d8c2cea8611f3b5d67e471f0ddeacca37d0e48d2b27dfef8d905c3dd0ca4  golden/snapshot-request.json",
        "95152c6aee09c40eaba7fcb2ab704f881f013cf75865b18a56fdb96a392925c6  golden/snapshot-response.json",
        "bbb959f20a5293032072ee9ab30c9489807ad185cec7d740f15dd929a9bfa622  golden/visible-map.json"
    };

    private static void CheckArtifactBytesAndContractBoundaries()
    {
        string root = RepositoryRoot();
        string artifactRoot = Path.Combine(root, "protocol-artifact", "runtime-map-v1");
        string sourceSchemaPath = Path.Combine(root, "schemas", "runtime-map-v1.schema.json");
        string packageSchemaPath = Path.Combine(artifactRoot, "schema.json");
        byte[] sourceSchema = File.ReadAllBytes(sourceSchemaPath);
        byte[] packageSchema = File.ReadAllBytes(packageSchemaPath);
        Check(sourceSchema.SequenceEqual(packageSchema),
            "protocol source and package schema bytes are identical");
        Check(Sha256(sourceSchema) == RuntimeMapV1Contract.SchemaDigest,
            "managed consumer digest matches the exact protocol schema bytes");

        string checksums = File.ReadAllText(Path.Combine(artifactRoot, "SHA256SUMS"));
        string expectedChecksums = string.Join('\n', ExpectedMapArtifactChecksums) + "\n";
        Check(checksums == expectedChecksums,
            "managed consumer sees the exact checked-in artifact checksum inventory");
        foreach (string line in ExpectedMapArtifactChecksums)
        {
            string[] fields = line.Split("  ", 2, StringSplitOptions.None);
            string path = Path.GetFullPath(Path.Combine(artifactRoot, fields[1]));
            Check(File.Exists(path) && Sha256(File.ReadAllBytes(path)) == fields[0],
                $"artifact checksum matches {fields[1]}");
        }

        using JsonDocument manifest = JsonDocument.Parse(
            File.ReadAllBytes(Path.Combine(artifactRoot, "manifest.json")));
        Check(manifest.RootElement.GetProperty("schema_digest").GetString()
            == RuntimeMapV1Contract.SchemaDigest,
            "artifact manifest carries the managed consumer digest");
        using JsonDocument schema = JsonDocument.Parse(packageSchema);
        Check(schema.RootElement.GetProperty("$id").GetString() == "sts2-runtime-map-v1"
            && schema.RootElement.GetProperty("$defs").GetProperty("text")
                .GetProperty("maxLength").GetInt32() == RuntimeMapV1Contract.MaxTextBytes,
            "artifact schema retains the bounded map text contract");

        CheckTextProducerBoundaries();

        RuntimeMapV1Snapshot equalGraphAndOptionIds = Snapshot() with
        {
            Bindings = new[]
            {
                new RuntimeMapV1ActionBinding(
                    "map:1:1:0", "host-action:42:option-equals-graph", "map:1:1:0")
            }
        };
        Check(equalGraphAndOptionIds.Validate(out string equalGraphOptionError),
            $"graph and option IDs may share a serialized value with a distinct host action ID: {equalGraphOptionError}");

        RuntimeMapV1Snapshot equalGraphAndHostIds = Snapshot() with
        {
            Bindings = new[]
            {
                new RuntimeMapV1ActionBinding(
                    "map:1:1:0", "map:1:1:0", "host-action:42:graph-equals-option")
            }
        };
        Check(!equalGraphAndHostIds.Validate(out _),
            "host action ID equal to graph node ID is rejected");
    }

    private static void CheckTextProducerBoundaries()
    {
        string textBoundary = new string('é', RuntimeMapV1Contract.MaxTextBytes / 2);
        CheckTextProducerValue("game_build", textBoundary,
            value => Snapshot() with { GameBuild = value }, RuntimeMapV1Contract.MaxTextBytes);
        CheckTextProducerValue("mod_version", textBoundary,
            value => Snapshot() with { ModVersion = value }, RuntimeMapV1Contract.MaxTextBytes);
        CheckTextProducerValue("reason",
            new string('é', RuntimeMapV1Contract.MaxReasonBytes / 2),
            value => Unavailable() with { Reason = value }, RuntimeMapV1Contract.MaxReasonBytes);

        string supplementaryTextBoundary = string.Concat(
            Enumerable.Repeat("🙂", RuntimeMapV1Contract.MaxTextBytes / 4));
        CheckTextProducerValue("game_build", supplementaryTextBoundary,
            value => Snapshot() with { GameBuild = value }, RuntimeMapV1Contract.MaxTextBytes);
        CheckTextProducerValue("mod_version", supplementaryTextBoundary,
            value => Snapshot() with { ModVersion = value }, RuntimeMapV1Contract.MaxTextBytes);

        CheckTextProducerRejects("game_build",
            new string('a', RuntimeMapV1Contract.MaxTextBytes + 1),
            value => Snapshot() with { GameBuild = value });
        CheckTextProducerRejects("mod_version",
            new string('a', RuntimeMapV1Contract.MaxTextBytes + 1),
            value => Snapshot() with { ModVersion = value });
        CheckTextProducerRejects("reason",
            new string('a', RuntimeMapV1Contract.MaxReasonBytes + 1),
            value => Unavailable() with { Reason = value });

        foreach (string control in new[] { "\u0000", "\u007f", "\u0085" })
        {
            CheckTextProducerRejects("game_build", control,
                value => Snapshot() with { GameBuild = value });
            CheckTextProducerRejects("mod_version", control,
                value => Snapshot() with { ModVersion = value });
            CheckTextProducerRejects("reason", control,
                value => Unavailable() with { Reason = value });
        }
    }

    private static void CheckTextProducerValue(string field, string value,
        Func<string, RuntimeMapV1Snapshot> create, int maximumBytes)
    {
        RuntimeMapV1Snapshot snapshot = create(value);
        Check(snapshot.Validate(out string validationError),
            $"{field} exact {maximumBytes} UTF-8 bytes pass Snapshot.Validate: {validationError}");
        Check(RuntimeMapV1Codec.TrySerializeResponse(
            Context.CorrelationId, Context.InstanceId, Context.SessionId, Context.LeaseId,
            7, snapshot, out string json, out string codecError),
            $"{field} exact {maximumBytes} UTF-8 bytes pass codec serialization: {codecError}");
        using JsonDocument document = JsonDocument.Parse(json);
        Check(document.RootElement.GetProperty("snapshot").GetProperty(field).GetString() == value,
            $"{field} exact boundary is preserved by serialized producer");
        CheckTextProducerRejects(field, value + "a", create,
            $"one UTF-8 byte over the {maximumBytes}-byte multibyte boundary");
    }

    private static void CheckTextProducerRejects(string field, string value,
        Func<string, RuntimeMapV1Snapshot> create, string description = "invalid text")
    {
        RuntimeMapV1Snapshot snapshot = create(value);
        Check(!snapshot.Validate(out _),
            $"{field} {description} is rejected by Snapshot.Validate");
        Check(!RuntimeMapV1Codec.TrySerializeResponse(
            Context.CorrelationId, Context.InstanceId, Context.SessionId, Context.LeaseId,
            7, snapshot, out _, out _),
            $"{field} {description} is rejected by codec serialization");
    }

    private static string RepositoryRoot()
    {
        foreach (string start in new[] { Directory.GetCurrentDirectory(), AppContext.BaseDirectory })
        {
            DirectoryInfo? current = new(start);
            for (int depth = 0; current is not null && depth < 12; depth++, current = current.Parent)
            {
                if (File.Exists(Path.Combine(current.FullName,
                    "protocol-artifact", "runtime-map-v1", "SHA256SUMS")))
                {
                    return current.FullName;
                }
            }
        }
        throw new InvalidOperationException("repository root was not found");
    }

    private static string Sha256(byte[] bytes) =>
        Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
}
