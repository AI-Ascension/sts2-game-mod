// SPDX-License-Identifier: MIT

using System;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class Program
{
    private static nint _library;
    private static int _assertions;
    private static readonly string[] RequestedRules = ["resource.fixed_card_cost", "damage.nominal_card"];
    private static int Main(string[] args)
    {
        try
        {
            if (args.Length != 1) throw new InvalidOperationException("one native library path is required");
            _library = NativeLibrary.Load(args[0]);
            SourceCaptureAndFreshness();
            FailedReadsCannotRetainOlderValues();
            CaptureRefusalsAreExplicit();
            NativeFactsRoundTrip();
            VariableAndUnknownCosts();
            RefusesUnownedAndChangedInputs();
            Console.WriteLine($"GameFactsProbe: PASS ({_assertions} assertions; controlled host only)");
            return 0;
        }
        catch (Exception exception)
        {
            Console.Error.WriteLine(exception.Message);
            return 1;
        }
        finally
        {
            if (_library != 0) NativeLibrary.Free(_library);
        }
    }

    private static (BoundaryFixture Fixture, string Body) AdmittedFixture(bool? xCost = false)
    {
        var fixture = new BoundaryFixture();
        fixture.Player.PlayerCombatState!.Energy = 300;
        fixture.Card.EnergyCost.Value = 270;
        fixture.Card.HasEnergyCostX = xCost;
        ModEntry.Configure(_library, fixture.Reader);
        string manifest = ModEntry.CurrentManifestId();
        LiveCardCapturedSnapshot snapshot = fixture.Reader.Read("instance", manifest);
        Check(snapshot.Available, "production capture is available for the controlled host");
        ModEntry.Associate(snapshot);
        var instance = new { instance_id = "instance", run_id = "harness-run", epoch = snapshot.Epoch,
            entity_kind = "card", entity_id = snapshot.Cards[0].InstanceId };
        var snapshotRef = new { snapshot_id = snapshot.SnapshotId, instance_ref = instance,
            state_generation = snapshot.StateGeneration };
        string body = JsonSerializer.Serialize(new
        {
            protocol_version = "game-facts-reference-v1",
            schema_digest = "3065a2ff96e6e5af628b3d5cb43b8f2db4232ae4c4dc71324b973419908a2985",
            provenance = new { artifact = "sts2-protocol/game-facts-reference-v1",
                source = "schemas/game-facts-reference-v1.schema.json", generator = "hand-authored" },
            correlation_id = "facts-probe", kind = "query_request",
            query = new
            {
                rules_reference_version = 2,
                rule_ids = RequestedRules,
                binding = new { mode = "live", content_manifest_id = manifest, locale = "en",
                    visibility_scope = "owner", instance_ref = instance, snapshot_ref = snapshotRef },
                scope = new { instance_id = "instance", run_id = "harness-run", authority_epoch = 1,
                    content_manifest_id = manifest, locale = "en" },
                parent_observation = new { instance_ref = instance, snapshot_ref = snapshotRef,
                    state_generation = snapshot.StateGeneration }
            },
            result = (object?)null, capabilities = (object?)null, error = (object?)null
        });
        return (fixture, body);
    }

    private static void NativeFactsRoundTrip()
    {
        (BoundaryFixture fixture, string body) = AdmittedFixture();
        (int status, string response) = ModEntry.Query(body);
        using JsonDocument result = JsonDocument.Parse(response);
        Check(status == 200 && result.RootElement.GetProperty("kind").GetString() == "query_response",
            "managed source and actual native producer return canonical facts-v1");
        JsonElement payload = result.RootElement.GetProperty("result");
        JsonElement found = payload.GetProperty("results")[0];
        JsonElement inputs = found.GetProperty("inputs");
        Check(found.GetProperty("evidence_status").GetString() == "source_derived"
            && inputs[0].GetProperty("name").GetString() == "energy"
            && inputs[1].GetProperty("name").GetString() == "card_cost"
            && inputs[0].GetProperty("observation").GetProperty("value").GetProperty("value").GetInt32() == 300
            && inputs[1].GetProperty("observation").GetProperty("value").GetProperty("value").GetInt32() == 270,
            "actual bridge preserves Core input order and raw values without exactness promotion");
        Check(payload.GetProperty("inventory_binding").GetProperty("build_id").GetString() == "source-probe-build"
            && payload.GetProperty("inventory_binding").GetProperty("mode_id").GetString() == "standard"
            && payload.GetProperty("results")[1].GetProperty("kind").GetString() == "unsupported",
            "build and mode come from owners while unsourced Core rules stay unsupported");
        Check(!response.Contains("controlled-seed", StringComparison.Ordinal)
            && !response.Contains("confirmed", StringComparison.Ordinal),
            "response discloses no seed or native-confirmation claim");
        using JsonDocument capabilities = JsonDocument.Parse(ModEntry.Query(string.Empty).Response);
        Check(capabilities.RootElement.GetProperty("capabilities").GetProperty("snapshot_policy")
            .GetProperty("max_retained_snapshots").GetInt32() == 1, "capabilities declare the real single capture bound");

        int reads = fixture.Card.EnergyCost.Reads;
        Check(ModEntry.Query(body.Replace("facts-probe", "wrong-correlation", StringComparison.Ordinal)).Status == 400
            && fixture.Card.EnergyCost.Reads == reads,
            "wrong request correlation is rejected by canonical admission before host reads");
        Check(ModEntry.Query(body.Replace("\"kind\":\"query_request\"", "\"kind\":\"query_request\",\"kind\":\"query_request\"",
            StringComparison.Ordinal)).Status == 400, "duplicate JSON keys never reach source extraction");
    }

    private static void RefusesUnownedAndChangedInputs()
    {
        (_, string body) = AdmittedFixture();
        Check(ErrorCode(ModEntry.QueryFromWrongLease(body)) == "denied_scope", "different admitted lease cannot reuse the binding");
        ModEntry.Authorized = false;
        Check(ErrorCode(ModEntry.Query(body)) == "denied_scope", "revoked runtime authority is refused");
        foreach (string change in new[] { "generation", "semantic", "locale", "during-catalog", "run-claim", "visibility" })
        {
            (BoundaryFixture fixture, string current) = AdmittedFixture();
            string expected = "invalid_binding";
            switch (change)
            {
                case "generation": NativeContentCatalogManifestSource.Generation++; break;
                case "semantic": NativeContentCatalogManifestSource.Semantic = "changed-semantics"; break;
                case "locale": NativeContentCatalogManifestSource.Locale = "fr"; break;
                case "during-catalog":
                    NativeContentCatalogManifestSource.OnCapture = () => fixture.Player.PlayerCombatState!.Energy++;
                    expected = "stale_snapshot"; break;
                case "run-claim":
                    current = current.Replace("harness-run", "invented-run", StringComparison.Ordinal);
                    expected = "denied_scope"; break;
                default:
                    JsonNode request = JsonNode.Parse(current)!;
                    request["query"]!["binding"]!["visibility_scope"] = "other";
                    current = request.ToJsonString(); expected = "denied_scope"; break;
            }
            Check(ErrorCode(ModEntry.Query(current)) == expected, change + " is refused across the real managed/native boundary");
        }
    }

    private static void VariableAndUnknownCosts()
    {
        foreach (bool? xCost in new bool?[] { true, null })
        {
            (_, string body) = AdmittedFixture(xCost);
            using JsonDocument response = JsonDocument.Parse(ModEntry.Query(body).Response);
            JsonElement observation = response.RootElement.GetProperty("result")
                .GetProperty("results")[0].GetProperty("inputs")[1].GetProperty("observation");
            Check(observation.GetProperty("value").ValueKind == JsonValueKind.Null
                && observation.GetProperty("availability").GetString()
                    == (xCost is true ? "unsupported" : "not_observable"),
                "production fixedness read preserves variable or unavailable costs without fallback");
        }
    }

    private static string? ErrorCode((int Status, string Response) response)
    {
        using JsonDocument document = JsonDocument.Parse(response.Response);
        return document.RootElement.GetProperty("error").GetProperty("code").GetString();
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException("FAIL: " + message);
        _assertions++;
        Console.WriteLine("PASS: " + message);
    }
}
