// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class Program
{
    private static readonly string[] PositionKindFields = { "kind" };
    private static readonly TestContext Context = new(
        "corr-42", "instance-1", "session-1", "lease-1", "7");

    private static int Main()
    {
        CheckSnapshotValidation();
        CheckCodecAndCanonicalShape();
        CheckRequestAndSupportBoundary();
        CheckAdversarialGraphRejection();
        CheckFairPlayPairedFixtures();
        Console.WriteLine("RuntimeMapV1Probe: PASS");
        return 0;
    }

    private static void CheckSnapshotValidation()
    {
        Check(Snapshot().Validate(out _), "complete synthetic map validates");
        Check(Unavailable().Validate(out _), "unavailable projection validates");
    }

    private static void CheckCodecAndCanonicalShape()
    {
        Check(RuntimeMapV1Codec.TrySerializeResponse(
            Context.CorrelationId, Context.InstanceId, Context.SessionId, Context.LeaseId,
            ulong.Parse(Context.LeaseEpoch, CultureInfo.InvariantCulture), Snapshot(),
            out string response, out string error),
            $"response serializes: {error}");
        using JsonDocument document = JsonDocument.Parse(response);
        JsonElement root = document.RootElement;
        Check(root.GetProperty("schema_digest").GetString() == RuntimeMapV1Contract.SchemaDigest,
            "response carries the current schema digest");
        Check(root.GetProperty("snapshot").GetProperty("schema_version").GetString()
            == RuntimeMapV1Contract.SnapshotSchemaVersion,
            "response carries the independent visible-map schema version");
        string[] fields = root.EnumerateObject().Select(property => property.Name).ToArray();
        Check(fields.SequenceEqual(fields.OrderBy(field => field, StringComparer.Ordinal)),
            "response object keys are canonical sorted JSON");
        RuntimeMapV1Snapshot reordered = Snapshot() with
        {
            Nodes = Snapshot().Nodes.Reverse().ToArray(),
            Edges = Snapshot().Edges.Reverse().ToArray(),
            TerminalNodeIds = Snapshot().TerminalNodeIds.Reverse().ToArray(),
            Bindings = Snapshot().Bindings.Reverse().ToArray()
        };
        Check(Serialize(Snapshot()) == Serialize(reordered),
            "response collection ordering is canonical");
        using JsonDocument unavailable = JsonDocument.Parse(Serialize(Unavailable()));
        Check(unavailable.RootElement.GetProperty("snapshot").GetProperty("position")
            .EnumerateObject().Select(property => property.Name).SequenceEqual(PositionKindFields),
            "position variants omit node_id when no current node exists");
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

    private static void CheckAdversarialGraphRejection()
    {
        RuntimeMapV1Snapshot duplicateNode = Snapshot() with
        {
            Nodes = Snapshot().Nodes.Append(Snapshot().Nodes[0]).ToArray()
        };
        Check(!duplicateNode.Validate(out _), "duplicate node IDs are rejected");

        RuntimeMapV1Snapshot duplicateCoordinate = Snapshot() with
        {
            Nodes = Snapshot().Nodes.Append(new RuntimeMapV1Node(
                "map:1:0:9", 0, 0, "monster", false)).ToArray()
        };
        Check(duplicateCoordinate.Validate(out string duplicateError),
            $"duplicate coordinates are preserved: {duplicateError}");

        RuntimeMapV1Snapshot distinctActionPayload = Snapshot() with
        {
            Bindings = new[] { new RuntimeMapV1ActionBinding(
                "map:1:1:0", "select_map_node:42:legacy-map-option", "legacy-map-option") }
        };
        Check(distinctActionPayload.Validate(out _),
            "host action payload may remain distinct from the stable graph node ID");

        RuntimeMapV1Snapshot cycle = Snapshot() with
        {
            Edges = Snapshot().Edges.Append(new RuntimeMapV1Edge(
                "map:1:2:0", "map:1:0:0")).ToArray()
        };
        Check(!cycle.Validate(out _), "cyclic topology is rejected");

        RuntimeMapV1Snapshot staleBinding = Snapshot() with
        {
            Bindings = new[] { new RuntimeMapV1ActionBinding(
                "map:1:1:0", "select_map_node:41:map:1:1:0", "map:1:1:0") }
        };
        Check(staleBinding.Validate(out _),
            "binding generation remains an opaque host action identity at snapshot validation");
        RuntimeMapV1Snapshot duplicateOption = Snapshot() with
        {
            Bindings = new[]
            {
                new RuntimeMapV1ActionBinding(
                    "map:1:1:0", "select_map_node:41:left", "map-option:shared"),
                new RuntimeMapV1ActionBinding(
                    "map:1:1:1", "select_map_node:41:right", "map-option:shared")
            }
        };
        Check(!duplicateOption.Validate(out _),
            "serialized action option IDs are unique within a snapshot");
        RuntimeMapV1Snapshot currentBinding = Snapshot() with
        {
            Position = new RuntimeMapV1Position("current", "map:1:1:0")
        };
        Check(!currentBinding.Validate(out _),
            "the current node cannot also be exposed as a travel binding");
        Check(!RuntimeMapV1Contract.IsHostActionId("bad action"),
            "unsafe host action identity is rejected");
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

    private static string Serialize(RuntimeMapV1Snapshot snapshot)
    {
        Check(RuntimeMapV1Codec.TrySerializeResponse(
            Context.CorrelationId, Context.InstanceId, Context.SessionId, Context.LeaseId,
            ulong.Parse(Context.LeaseEpoch, CultureInfo.InvariantCulture), snapshot,
            out string json, out string error), error);
        return json;
    }

    private static RuntimeMapV1Snapshot Snapshot() => new(
        StateId: "map-state-42",
        Generation: 42,
        SchemaVersion: RuntimeMapV1Contract.SnapshotSchemaVersion,
        ProjectionVersion: RuntimeMapV1Contract.ProjectionVersion,
        GameBuild: "0.107.1",
        ModVersion: "map-mod-1",
        MapInstanceId: "map-instance-1",
        ActId: 1,
        ScopeId: "campaign-1",
        Availability: "available",
        Completeness: "complete",
        Freshness: "current",
        Reason: null,
        Nodes: new[]
        {
            new RuntimeMapV1Node("map:1:0:0", 0, 0, "start", true),
            new RuntimeMapV1Node("map:1:1:0", 1, 0, "monster", true),
            new RuntimeMapV1Node("map:1:1:1", 1, 1, "event", false),
            new RuntimeMapV1Node("map:1:2:0", 2, 0, "boss", false)
        },
        Edges: new[]
        {
            new RuntimeMapV1Edge("map:1:0:0", "map:1:1:0"),
            new RuntimeMapV1Edge("map:1:0:0", "map:1:1:1"),
            new RuntimeMapV1Edge("map:1:1:0", "map:1:2:0"),
            new RuntimeMapV1Edge("map:1:1:1", "map:1:2:0")
        },
        Position: new RuntimeMapV1Position("current", "map:1:0:0"),
        History: new List<string> { "map:1:0:0" },
        TerminalNodeIds: new List<string> { "map:1:2:0" },
        Bindings: new[]
        {
            new RuntimeMapV1ActionBinding("map:1:1:0",
                "select_map_node:42:map:1:1:0", "map:1:1:0"),
            new RuntimeMapV1ActionBinding("map:1:1:1",
                "select_map_node:42:map:1:1:1", "map:1:1:1")
        });

    private static RuntimeMapV1Snapshot Unavailable() => Snapshot() with
    {
        StateId = "map-unavailable-42",
        MapInstanceId = null,
        ActId = null,
        ScopeId = null,
        Availability = "unavailable",
        Completeness = "unknown",
        Reason = "map_not_open",
        Nodes = Array.Empty<RuntimeMapV1Node>(),
        Edges = Array.Empty<RuntimeMapV1Edge>(),
        Position = new RuntimeMapV1Position("unavailable", null),
        History = Array.Empty<string>(),
        TerminalNodeIds = Array.Empty<string>(),
        Bindings = Array.Empty<RuntimeMapV1ActionBinding>()
    };

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
