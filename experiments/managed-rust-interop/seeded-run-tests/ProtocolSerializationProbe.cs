// SPDX-License-Identifier: MIT

using System.Security.Cryptography;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const int RuntimeAccepted = 200;
    private const int RuntimeRejected = 409;
    private const int RuntimeUnavailable = 503;
    private const string SeededRunProtocolVersion = "seeded-run-v1";
    private const string SeededRunArtifact = "sts2-protocol/seeded-run-v1";
    private const string SeededRunSchemaSource = "schemas/seeded-run-v1.schema.json";
    private const string SeededRunGenerator = "hand-authored";
    private const string SeededRunSchemaDigest =
        "5c659f344be78f84e8d783986925d462714f933cac95d18943358992f7d3e2b8";
    private static readonly JsonSerializerOptions SeededRunJsonOptions = new()
    {
        PropertyNameCaseInsensitive = false,
        MaxDepth = 16
    };
    private static readonly string[] ContextActs = { "act_1", "act_2", "act_3", "act_4" };

    private static ulong ParseEpoch(string value) =>
        ulong.TryParse(value, out ulong epoch) ? epoch : 0;

    public static void Main()
    {
        string artifactRoot = Path.Combine(
            AppContext.BaseDirectory, "protocol-artifact", "seeded-run-v1");
        string schemaPath = Path.Combine(artifactRoot, "schema.json");
        Check(File.Exists(schemaPath), "frozen seeded-run schema is present");
        Check(string.Equals(
                Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(schemaPath))),
                SeededRunSchemaDigest, StringComparison.OrdinalIgnoreCase),
            "frozen seeded-run schema digest is pinned");

        using JsonDocument schema = JsonDocument.Parse(File.ReadAllText(schemaPath));
        HashSet<string> schemaFields = new(StringComparer.Ordinal);
        foreach (JsonProperty property in schema.RootElement.GetProperty("$defs")
                     .GetProperty("base").GetProperty("properties").EnumerateObject())
        {
            schemaFields.Add(property.Name);
        }

        foreach (JsonProperty property in schema.RootElement.GetProperty("$defs")
                     .GetProperty("message_fields").GetProperty("properties").EnumerateObject())
        {
            schemaFields.Add(property.Name);
        }

        SeededRunSelectionContext context = Context();
        RuntimeContext runtimeContext = new(
            "instance-1", "caller-1", "session-1", "lease-1", "1", "corr-seed-0001");
        SeededRunStandardHostReceipt accepted = new(
            "op-seed-1", "ironclad-42", "seeded_training", context,
            SeededRunStandardHostStatus.Accepted, null, null, null, null, 0);
        AssertResponse(
            SeededRunResponse(runtimeContext, "start_response", accepted),
            "start_response", 200, 0, null, null, schemaFields);

        SeededRunStandardObservation observation = new(
            true, true, 1, "ironclad-42", context.ContextDigest,
            "custom_run_setup", "run_started", "fake-host-build-1");
        SeededRunStandardEffectWitness witness = new("run_started", 1, "ironclad-42");
        SeededRunStandardHostReceipt settled = accepted with
        {
            Status = SeededRunStandardHostStatus.Settled,
            CanonicalSeed = "ironclad-42",
            Observation = observation,
            EffectWitness = witness
        };
        AssertResponse(
            SeededRunResponse(runtimeContext, "start_response", settled),
            "start_response", 200, 0, 1, 1, schemaFields);
        AssertResponse(
            SeededRunResponse(runtimeContext, "reconcile_response", settled),
            "reconcile_response", 200, 0, 1, 1, schemaFields);

        SeededRunStandardHostReceipt nonzeroAccepted = accepted with { RequestGeneration = 7 };
        AssertResponse(
            SeededRunResponse(runtimeContext, "start_response", nonzeroAccepted),
            "start_response", 200, 7, null, null, schemaFields);
        SeededRunStandardHostReceipt nonzeroSettled = nonzeroAccepted with
        {
            Status = SeededRunStandardHostStatus.Settled,
            CanonicalSeed = "ironclad-42",
            Observation = observation with { Generation = 8 },
            EffectWitness = witness with { Generation = 8 }
        };
        AssertResponse(
            SeededRunResponse(runtimeContext, "start_response", nonzeroSettled),
            "start_response", 200, 7, 8, 8, schemaFields);
        AssertResponse(
            SeededRunResponse(runtimeContext, "start_response", nonzeroSettled),
            "start_response", 200, 7, 8, 8, schemaFields);
        AssertResponse(
            SeededRunResponse(runtimeContext, "reconcile_response", nonzeroSettled),
            "reconcile_response", 200, 7, 8, 8, schemaFields);

        AssertFrozenGeneration(Path.Combine(artifactRoot, "golden", "start-accepted.json"), 0, null);
        AssertFrozenGeneration(Path.Combine(artifactRoot, "golden", "start-settled.json"), 0, 1);
        AssertFrozenGeneration(Path.Combine(artifactRoot, "golden", "reconcile-settled.json"), 0, 1);
        Console.WriteLine("seeded-run serializer generation checks passed");
    }

    private static SeededRunSelectionContext Context() => new(
        "standard/ironclad/asc0/fresh", "standard", "ironclad", 0,
        Array.Empty<string>(), ContextActs,
        "standard_default",
        new SeededRunContextProfileBaseline(
            "fresh", "fresh-standard-comparison",
            "4581aaf95348126550cdf3b73ec46b39d447523cf7cb35aec71c2842d1945031"),
        "disabled",
        new SeededRunContextCompatibility(
            new SeededRunContextIdentityDigest(
                "sts2-game/v0.107.1",
                "2db9d9f665c776c2324c7f98134b900a8b3332f32148ea1cb84063d52db94ff4"),
            new SeededRunContextIdentityDigest(
                "ai-ascension/sts2-game-mod",
                "0c6e7bbb54996222a4894de999fc860decc360f9936c9bd5a7382d97b361bb7e")),
        "d57563180f198b73970510427981504a9df10c62931577e601f9dcce6275fbe9");

    private static void AssertResponse(
        (int Status, string Response) response,
        string expectedKind,
        int expectedStatus,
        ulong expectedEnvelopeGeneration,
        ulong? expectedObservationGeneration,
        ulong? expectedWitnessGeneration,
        IReadOnlySet<string> schemaFields)
    {
        Check(response.Status == expectedStatus, $"{expectedKind} status is {expectedStatus}");
        using JsonDocument document = JsonDocument.Parse(response.Response);
        JsonElement root = document.RootElement;
        Check(root.GetProperty("kind").GetString() == expectedKind,
            $"{expectedKind} kind is preserved");
        Check(root.GetProperty("generation").GetUInt64() == expectedEnvelopeGeneration,
            $"{expectedKind} keeps original request generation");
        HashSet<string> actualFields = root.EnumerateObject()
            .Select(property => property.Name)
            .ToHashSet(StringComparer.Ordinal);
        Check(actualFields.SetEquals(schemaFields), $"{expectedKind} fields match frozen schema");

        JsonElement observation = root.GetProperty("observation");
        JsonElement witness = root.GetProperty("effect_witness");
        if (expectedObservationGeneration is ulong observationGeneration)
        {
            Check(observation.GetProperty("generation").GetUInt64() == observationGeneration,
                $"{expectedKind} observation keeps post-start generation");
            Check(observationGeneration > expectedEnvelopeGeneration,
                $"{expectedKind} observation generation advances beyond envelope generation");
        }
        else
        {
            Check(observation.ValueKind == JsonValueKind.Null,
                $"{expectedKind} has no observation before settlement");
        }

        if (expectedWitnessGeneration is ulong witnessGeneration)
        {
            Check(witness.GetProperty("generation").GetUInt64() == witnessGeneration,
                $"{expectedKind} witness keeps post-start generation");
        }
        else
        {
            Check(witness.ValueKind == JsonValueKind.Null,
                $"{expectedKind} has no witness before settlement");
        }
    }

    private static void AssertFrozenGeneration(string path, ulong envelopeGeneration, ulong? observationGeneration)
    {
        using JsonDocument fixture = JsonDocument.Parse(File.ReadAllText(path));
        JsonElement root = fixture.RootElement;
        Check(root.GetProperty("generation").GetUInt64() == envelopeGeneration,
            $"{Path.GetFileName(path)} keeps frozen envelope generation");
        JsonElement observation = root.GetProperty("observation");
        if (observationGeneration is ulong expected)
        {
            Check(observation.GetProperty("generation").GetUInt64() == expected,
                $"{Path.GetFileName(path)} keeps frozen observation generation");
            Check(expected > envelopeGeneration,
                $"{Path.GetFileName(path)} freezes post-start generation ordering");
        }
    }

    private static void Check(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }

        Console.WriteLine("PASS: " + message);
    }
}
