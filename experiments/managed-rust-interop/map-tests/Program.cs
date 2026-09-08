// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class Program
{
    private static readonly string[] PositionKindFields = { "kind" };
    private static readonly string[] FingerprintStartChildren = { "node:b", "node:c" };
    private static readonly string[] FingerprintTerminalChildren = Array.Empty<string>();
    private static readonly string[] FingerprintSingleChild = { "node:d" };
    private static readonly string[] FingerprintRewiredChild = { "node:c" };
    private static readonly TestContext Context = new(
        "corr-42", "instance-1", "session-1", "lease-1", "7");

    private static int Main()
    {
        CheckSnapshotValidation();
        CheckCodecAndCanonicalShape();
        CheckRequestAndSupportBoundary();
        CheckAdversarialGraphRejection();
        CheckFairPlayPairedFixtures();
        CheckIdentityRegistryLifetimeBound();
        CheckGraphFingerprintIdentityAndOrdering();
        CheckAncientCategoryNormalization();
        CheckObservationGenerationFence();
        CheckArtifactBytesAndContractBoundaries();
        Console.WriteLine("RuntimeMapV1Probe: PASS");
        return 0;
    }

    private static void CheckRequestAndSupportBoundary()
    {
        const string request =
            "{\"correlation_id\":\"corr-42\",\"generation\":42,\"instance_id\":\"instance-1\","
            + "\"kind\":\"snapshot_request\",\"lease_epoch\":7,\"lease_id\":\"lease-1\","
            + "\"protocol_version\":\"runtime-map-v1\",\"provenance\":{\"artifact\":"
            + "\"sts2-protocol/runtime-map-v1\",\"generator\":\"hand-authored\","
            + "\"source\":\"schemas/runtime-map-v1.schema.json\"},\"schema_digest\":\""
            + RuntimeMapV1Contract.SchemaDigest
            + "\",\"session_id\":\"session-1\",\"snapshot\":null,\"timeout\":null}";
        Check(RuntimeMapV1Codec.TryParseRequest(request, Context.CorrelationId,
            Context.InstanceId, Context.SessionId, Context.LeaseId, Context.LeaseEpoch,
            out ulong generation, out string error) && generation == 42,
            $"request parses: {error}");
        string oversized = new('é', RuntimeMapV1Contract.MaxRequestBytes / 2 + 1);
        Check(!RuntimeMapV1Codec.TryParseRequest(oversized, Context.CorrelationId,
            Context.InstanceId, Context.SessionId, Context.LeaseId, Context.LeaseEpoch,
            out _, out string oversizedError)
            && oversizedError == "map request exceeds its bound",
            "request bound is measured in UTF-8 bytes");

        RuntimeMapV1Support support = RuntimeMapV1Support.WithHost(new FixtureSource(Snapshot()));
        string result = support.Handle(Context.InstanceId, Context.SessionId, Context.LeaseId,
            Context.CorrelationId, Context.LeaseEpoch, request, out int status);
        Check(status == 200, "support accepts a valid read-only request");
        using JsonDocument response = JsonDocument.Parse(result);
        Check(response.RootElement.GetProperty("kind").GetString() == "snapshot_response",
            "support returns a snapshot response");

        result = support.Handle(Context.InstanceId, Context.SessionId, Context.LeaseId,
            Context.CorrelationId, Context.LeaseEpoch, string.Empty, out status);
        using JsonDocument emptyResponse = JsonDocument.Parse(result);
        Check(status == 200 && emptyResponse.RootElement.GetProperty("kind").GetString()
            == "snapshot_response",
            "native GET map route may request the current snapshot without a body");
    }

    private static void CheckFairPlayPairedFixtures()
    {
        var permitted = new HiddenFixture("secret-a");
        var paired = new HiddenFixture("secret-b");
        Check(Serialize(permitted.Projection) == Serialize(paired.Projection),
            "paired fixtures with different hidden state have identical permitted bytes");
        Check(!Serialize(permitted.Projection).Contains("secret", StringComparison.Ordinal),
            "hidden fixture state is absent from the serialized projection");
    }

    private static void CheckIdentityRegistryLifetimeBound()
    {
        var registry = new MapIdentityRegistry();
        var run = new object();
        var map = new object();
        string mapId = registry.EnsureMap(run, map);
        var points = new List<RegistryPoint>();
        Dictionary<object, string> ids = new();
        string reason = string.Empty;
        string firstId = string.Empty;
        bool chunksPass = true;
        for (int offset = 0; offset < RuntimeMapV1Contract.MaxMapIdentityRegistryEntries;
             offset += 128)
        {
            RegistryPoint[] batch = Enumerable.Range(offset, Math.Min(128,
                    RuntimeMapV1Contract.MaxMapIdentityRegistryEntries - offset))
                .Select(_ => new RegistryPoint()).ToArray();
            points.AddRange(batch);
            chunksPass &= registry.TryGetNodeIds(mapId, 1, batch, out ids, out reason)
                && reason.Length == 0;
            if (offset == 0) firstId = ids[batch[0]];
        }
        Check(points.Count == RuntimeMapV1Contract.MaxMapIdentityRegistryEntries
            && chunksPass
            && registry.TryGetNodeIds(mapId, 1, new[] { points[0] }, out ids, out reason)
            && ids[points[0]] == firstId,
            "identity registry preserves IDs through bounded churn reads");
        Check(!registry.TryGetNodeIds(mapId, 1, new[] { points[0], new RegistryPoint() },
                out _, out reason) && reason == "map_identity_registry_bound_exceeded",
            "identity registry fails closed when a map lifetime is exhausted");
        Check(!registry.TryGetNodeIds(mapId, 1, new[] { points[0] }, out _, out reason)
            && reason == "map_identity_registry_bound_exceeded",
            "identity exhaustion remains unavailable until a new map is observed");

        string resetMapId = registry.EnsureMap(run, new object());
        Check(resetMapId != mapId && registry.TryGetNodeIds(resetMapId, 1,
                new[] { new RegistryPoint() }, out _, out reason) && reason.Length == 0,
            "new map identity resets the bounded registry");
        string resetRunId = registry.EnsureMap(new object(), new object());
        Check(resetRunId != resetMapId && registry.TryGetNodeIds(resetRunId, 1,
                new[] { new RegistryPoint() }, out _, out reason) && reason.Length == 0,
            "new run identity resets the bounded registry");
        var reordered = points.Take(3).Reverse().ToArray();
        Check(registry.TryGetNodeIds(resetRunId, 1, points.Take(3).ToArray(),
                out Dictionary<object, string> firstIds, out _)
            && registry.TryGetNodeIds(resetRunId, 1, reordered,
                out Dictionary<object, string> reorderedIds, out _)
            && points.Take(3).All(point => firstIds[point] == reorderedIds[point]),
            "reference identities survive reordered map enumeration");
    }

    private static void CheckGraphFingerprintIdentityAndOrdering()
    {
        var nodes = new[]
        {
            new RuntimeMapV1FingerprintNode("node:a", 0, 0, "start", "none",
                FingerprintStartChildren),
            new RuntimeMapV1FingerprintNode("node:b", 1, 0, "other", "none",
                FingerprintSingleChild),
            new RuntimeMapV1FingerprintNode("node:c", 1, 0, "other", "none",
                FingerprintSingleChild),
            new RuntimeMapV1FingerprintNode("node:d", 2, 0, "boss", "boss",
                FingerprintTerminalChildren)
        };
        Check(RuntimeMapV1GraphFingerprint.TryCreate(nodes, out string fingerprint,
                out string reason) && reason.Length == 0,
            "stable graph IDs produce a fingerprint with duplicate coordinates");
        var reordered = new[]
        {
            nodes[2] with { ChildIds = nodes[2].ChildIds.Reverse().ToArray() },
            nodes[0] with { ChildIds = nodes[0].ChildIds.Reverse().ToArray() },
            nodes[3], nodes[1]
        };
        Check(RuntimeMapV1GraphFingerprint.TryCreate(reordered, out string reorderedFingerprint,
                out _)
            && fingerprint == reorderedFingerprint,
            "reordered nodes and child sets retain canonical fingerprint bytes");
        var rewired = nodes.Select(node => node.Id == "node:b"
            ? node with { ChildIds = FingerprintRewiredChild } : node).ToArray();
        Check(RuntimeMapV1GraphFingerprint.TryCreate(rewired, out string rewiredFingerprint,
                out _)
            && fingerprint != rewiredFingerprint,
            "rewiring duplicate-coordinate references changes the fingerprint");
    }

    private static void CheckAncientCategoryNormalization()
    {
        Check(RuntimeMapV1Category.Normalize("Ancient", false, false) == "other",
            "non-start Ancient points deliberately normalize to other");
        Check(RuntimeMapV1Category.Normalize("Ancient", true, false) == "start",
            "a declared Ancient starting point retains the start category");
    }

    private static void CheckObservationGenerationFence()
    {
        Check(RuntimeMapV1ObservationFence.IsStable("live:17", 17, "live:17", 17),
            "unchanged gameplay identity and generation remains eligible");
        RuntimeMapV1Snapshot rejected = RuntimeMapV1ObservationFence.RejectChangedSurface(
            Snapshot(), "live:18", 18);
        Check(!RuntimeMapV1ObservationFence.IsStable("live:17", 17, "live:17", 18)
            && !RuntimeMapV1ObservationFence.IsStable("live:17", 17, "live:18", 17)
            && rejected.Generation == 18
            && rejected.StateId == "live:18"
            && rejected.Availability == "unavailable"
            && rejected.Completeness == "unknown"
            && rejected.Reason == RuntimeMapV1ObservationFence.ChangedSurfaceReason
            && rejected.Validate(out _),
            "a topology, visibility, or legal-catalog mutation requires an unavailable retry");
    }

    private static string Serialize(RuntimeMapV1Snapshot snapshot)
    {
        Check(RuntimeMapV1Codec.TrySerializeResponse(
            Context.CorrelationId, Context.InstanceId, Context.SessionId, Context.LeaseId,
            ulong.Parse(Context.LeaseEpoch, CultureInfo.InvariantCulture), snapshot,
            out string json, out string error), error);
        return json;
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
        Console.WriteLine("PASS: " + message);
    }

    private sealed class FixtureSource(RuntimeMapV1Snapshot snapshot) : IRuntimeMapV1HostSource
    {
        public RuntimeMapV1Snapshot ObserveMap() => snapshot;
    }

    private sealed record TestContext(
        string CorrelationId,
        string InstanceId,
        string SessionId,
        string LeaseId,
        string LeaseEpoch);

    private sealed class RegistryPoint
    {
    }

    private sealed class HiddenFixture(string hiddenValue)
    {
        private readonly string _hiddenValue = hiddenValue;
        internal RuntimeMapV1Snapshot Projection => ProjectWithoutHiddenState(_hiddenValue);

        private static RuntimeMapV1Snapshot ProjectWithoutHiddenState(string hiddenValue)
        {
            _ = hiddenValue;
            return Snapshot();
        }
    }
}
