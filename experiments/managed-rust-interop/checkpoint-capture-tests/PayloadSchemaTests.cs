// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class PayloadSchemaTests
{
    private const string PinnedSchemaSha256 =
        "e332d8ca1a44d903147d3d3a3336f65686ff502522a45293228d6fc9c63f73dc";

    private static string Root => AppContext.BaseDirectory;

    private static string SchemaPath =>
        Path.Combine(Root, "schemas", "checkpoint-payload-v1.schema.json");

    /// <summary>
    /// The evaluator is only trusted after it reproduces the pinned conformance verdicts: every
    /// valid fixture passes and every invalid fixture with <c>schema_valid: false</c> fails, while
    /// the two fixtures rejected only by the typed parser still pass schema validation.
    /// </summary>
    internal static void ValidatorAgreesWithPinnedConformanceFixtures()
    {
        byte[] schemaBytes = File.ReadAllBytes(SchemaPath);
        string digest = Convert.ToHexStringLower(SHA256.HashData(schemaBytes));
        Program.Check(digest == PinnedSchemaSha256, "the linked schema is the pinned digest");
        var validator = new PayloadSchemaValidator(Encoding.UTF8.GetString(schemaBytes));

        using JsonDocument caseDocument = JsonDocument.Parse(File.ReadAllText(
            Path.Combine(Root, "conformance", "cases", "checkpoint-payload-v1.json")));
        JsonElement root = caseDocument.RootElement;
        Program.Check(root.GetProperty("schema_sha256").GetString() == PinnedSchemaSha256,
            "the conformance case pins the same schema digest");

        int valid = 0;
        foreach (JsonElement vector in root.GetProperty("valid_vectors").EnumerateArray())
        {
            IReadOnlyList<string> errors = ValidateFixture(validator, vector);
            Program.Check(errors.Count == 0,
                vector.GetProperty("id").GetString() + " validates: " + string.Join("; ", errors));
            valid++;
        }
        int invalid = 0;
        foreach (JsonElement vector in root.GetProperty("invalid_vectors").EnumerateArray())
        {
            bool schemaValid = vector.GetProperty("schema_valid").GetBoolean();
            IReadOnlyList<string> errors = ValidateFixture(validator, vector);
            Program.Check((errors.Count == 0) == schemaValid,
                vector.GetProperty("id").GetString() + " schema_valid=" + schemaValid
                + " reproduced");
            invalid++;
        }
        Program.Check(valid == 4 && invalid == 9, "all 13 pinned fixtures were consumed");
    }

    internal static void CapturedRecordValidatesAgainstPayloadSchemaV1()
    {
        var validator = new PayloadSchemaValidator(File.ReadAllText(SchemaPath));
        var host = new SyntheticHost();
        var source = new CheckpointCaptureSource(host);
        foreach ((CheckpointCaptureBoundary boundary, CheckpointHostSettlementWitness witness) in new[]
        {
            (CheckpointCaptureBoundary.StablePlayerTurnCombat, SyntheticHost.QuiescentCombat(1)),
            (CheckpointCaptureBoundary.LaterTurnCombat, SyntheticHost.QuiescentCombat(4)),
            (CheckpointCaptureBoundary.SettledMapChoice, SyntheticHost.QuiescentMapChoice())
        })
        {
            host.State.Witness = witness;
            CheckpointCaptureOutcome outcome = source.Capture(boundary);
            Program.Check(outcome.IsCaptured, boundary.Code() + " captures on the synthetic host");
            using JsonDocument payload = JsonDocument.Parse(outcome.PayloadUtf8.AsSpan().ToArray());
            IReadOnlyList<string> errors = validator.Validate(payload.RootElement);
            Program.Check(errors.Count == 0,
                boundary.Code() + " payload validates against checkpoint-payload-v1: "
                + string.Join("; ", errors));
            Program.Check(payload.RootElement.GetProperty("boundary").GetString() == boundary.Code()
                && payload.RootElement.GetProperty("schema").GetString()
                    == CheckpointPayloadRecord.Schema,
                boundary.Code() + " payload carries the boundary and schema tokens");
        }

        host.State.Witness = SyntheticHost.QuiescentCombat(1);
        CheckpointCaptureOutcome combat = source.Capture(CheckpointCaptureBoundary.StablePlayerTurnCombat);
        using JsonDocument fixture = JsonDocument.Parse(File.ReadAllText(Path.Combine(Root,
            "conformance", "fixtures", "checkpoint-payload-v1", "valid",
            "stable-player-turn-combat.json")));
        using JsonDocument captured = JsonDocument.Parse(combat.PayloadUtf8.AsSpan().ToArray());
        Program.Check(Canonical(fixture.RootElement) == Canonical(captured.RootElement),
            "the synthetic capture is structurally identical to the pinned valid fixture");

        // Stronger than structural identity: the writer's own bytes must match the canonical
        // encoding the conformance case pins for this vector. The checked-in fixture file is
        // pretty-printed, so its length is not the pinned canonical one.
        JsonElement pinned = PinnedVector("CPV-VALID-STABLE-PLAYER-TURN-COMBAT");
        byte[] capturedBytes = combat.PayloadUtf8.AsSpan().ToArray();
        Program.Check(capturedBytes.Length == pinned.GetProperty("canonical_bytes").GetInt32(),
            "the synthetic capture has the pinned canonical byte length");
        string blob = "sha256:" + Convert.ToHexStringLower(SHA256.HashData(capturedBytes));
        Program.Check(blob == pinned.GetProperty("blob_digest").GetString(),
            "the synthetic capture hashes to the pinned canonical blob digest");
    }

    /// <summary>The pinned conformance vector with the given id.</summary>
    private static JsonElement PinnedVector(string id)
    {
        using JsonDocument caseDocument = JsonDocument.Parse(File.ReadAllText(
            Path.Combine(Root, "conformance", "cases", "checkpoint-payload-v1.json")));
        foreach (JsonElement vector in caseDocument.RootElement
            .GetProperty("valid_vectors").EnumerateArray())
        {
            if (vector.GetProperty("id").GetString() == id)
                return vector.Clone();
        }
        throw new InvalidOperationException("pinned vector is missing: " + id);
    }

    private static IReadOnlyList<string> ValidateFixture(
        PayloadSchemaValidator validator, JsonElement vector)
    {
        string relative = vector.GetProperty("fixture").GetString()!;
        using JsonDocument document = JsonDocument.Parse(File.ReadAllText(Path.Combine(Root,
            relative.Replace('/', Path.DirectorySeparatorChar))));
        return validator.Validate(document.RootElement);
    }

    /// <summary>Order-independent structural rendering (sorted keys) for comparison only.</summary>
    private static string Canonical(JsonElement element)
    {
        var builder = new StringBuilder();
        Render(element, builder);
        return builder.ToString();
    }

    private static void Render(JsonElement element, StringBuilder builder)
    {
        switch (element.ValueKind)
        {
            case JsonValueKind.Object:
                var names = new List<string>();
                foreach (JsonProperty property in element.EnumerateObject())
                    names.Add(property.Name);
                names.Sort(StringComparer.Ordinal);
                builder.Append('{');
                foreach (string name in names)
                {
                    builder.Append(name).Append(':');
                    Render(element.GetProperty(name), builder);
                    builder.Append(',');
                }
                builder.Append('}');
                break;
            case JsonValueKind.Array:
                builder.Append('[');
                foreach (JsonElement item in element.EnumerateArray())
                {
                    Render(item, builder);
                    builder.Append(',');
                }
                builder.Append(']');
                break;
            default:
                builder.Append(element.GetRawText());
                break;
        }
    }
}
