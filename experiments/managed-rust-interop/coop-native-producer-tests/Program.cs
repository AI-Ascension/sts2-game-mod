// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.CoopNativeProducerTests;

internal static class Program
{
    private const string ProtocolVersion = "coop-native-v1";
    private const string Artifact = "sts2-protocol/coop-native-v1";
    private const string SchemaSource = "schemas/coop-native-v1.schema.json";
    private const string SchemaDigest =
        "2f3bc99e53080fa11b39592b64fb0ab964a16f568719a2622d0b2caf766ab629";
    private const string InstanceId = "instance:native-test";
    private const string SessionId = "session:native-test";
    private const string LeaseId = "lease:native-test";
    private static readonly JsonSerializerOptions JsonOptions = new() { WriteIndented = true };

    public static int Main()
    {
        string? output = Environment.GetEnvironmentVariable("STS2_COOP_NATIVE_CAPTURE_DIR");
        if (string.IsNullOrWhiteSpace(output))
        {
            Console.Error.WriteLine("STS2_COOP_NATIVE_CAPTURE_DIR is required");
            return 2;
        }
        Directory.CreateDirectory(output);
        CaptureObservation(output);
        CaptureCatalog(output);
        CaptureAction(output, CaptureMode.ActionSettled, "local-action-settled", "op:native:action:settled");
        CaptureAction(output, CaptureMode.ActionRejected, "local-action-rejected", "op:native:action:rejected");
        CaptureUnknownAndRecovery(output);
        CaptureVote(output);
        CaptureRejoin(output);
        Console.WriteLine("managed source-only native co-op producer captures passed");
        return 0;
    }

    private static void CaptureCatalog(string directory)
    {
        var port = new CapturePort(CaptureMode.ActionSettled);
        var runtime = new CoopNativeRuntime(port);
        const string correlation = "corr:native:legal-catalog";
        string request = Envelope(
            "legal_catalog_request", null, "peer:host1", 1, null, null, null, correlation);
        (int status, string response) = runtime.Handle(
            InstanceId, SessionId, LeaseId, correlation, "7",
            "legal_catalog_request", request);
        Check(status == 200 && response.Contains("\"kind\":\"legal_catalog_response\"", StringComparison.Ordinal),
            "legal catalog response");
        Write(directory, "legal-catalog", "legal_catalog_request", request, null, status, response);
    }

    private static void CaptureObservation(string directory)
    {
        var port = new CapturePort(CaptureMode.ActionSettled);
        var runtime = new CoopNativeRuntime(port);
        (int status, string response) = runtime.Handle(
            InstanceId, SessionId, LeaseId, "corr:native:observation", "7",
            "observation", string.Empty);
        Check(status == 200, "observation status");
        Write(directory, "observation", "observation", null, null, status, response);
    }

    private static void CaptureAction(
        string directory, CaptureMode mode, string name, string operationId)
    {
        var port = new CapturePort(mode);
        var runtime = new CoopNativeRuntime(port);
        string correlation = "corr:native:" + name;
        string request = Envelope(
            "local_action_request", operationId, "peer:host1", 1,
            new { kind = "end_turn", action_id = "turn:1", target_peer = (string?)null },
            null, null, correlation);
        (int status, string response) = runtime.Handle(
            InstanceId, SessionId, LeaseId, correlation, "7",
            "local_action_request", request);
        Check(mode == CaptureMode.ActionRejected ? status == 409 : status == 200, name + " status");
        Write(directory, name, "local_action_request", request, null, status, response);
    }

    private static void CaptureUnknownAndRecovery(string directory)
    {
        var port = new CapturePort(CaptureMode.ActionUnknown);
        var runtime = new CoopNativeRuntime(port);
        const string operationId = "op:native:action:unknown";
        const string correlation = "corr:native:action:unknown";
        string request = Envelope(
            "local_action_request", operationId, "peer:host1", 1,
            new { kind = "end_turn", action_id = "turn:1", target_peer = (string?)null },
            null, null, correlation);
        (int acceptedStatus, string accepted) = runtime.Handle(
            InstanceId, SessionId, LeaseId, correlation, "7",
            "local_action_request", request);
        Check(acceptedStatus == 200 && Status(accepted) == "unknown", "unknown action response");
        Write(directory, "local-action-unknown", "local_action_request", request,
            "accepted_unknown", acceptedStatus, accepted);

        port.PublishForRecovery();
        string recovery = Envelope(
            "recovery_response", operationId, null, null, null, null,
            new { kind = "reconcile", rejoin_epoch = 1UL }, "corr:native:action:recovered");
        (int recoveredStatus, string recovered) = runtime.Handle(
            InstanceId, SessionId, LeaseId, "corr:native:action:recovered", "7",
            "recovery_response", recovery);
        Check(recoveredStatus == 200 && Status(recovered) == "settled", "recovered action response");
        Write(directory, "local-action-recovered", "recovery_response", recovery,
            "reconciled", recoveredStatus, recovered);
    }

    private static void CaptureVote(string directory)
    {
        var port = new CapturePort(CaptureMode.VoteSettled);
        var runtime = new CoopNativeRuntime(port);
        const string operationId = "op:native:vote:event";
        const string correlation = "corr:native:vote:event";
        string request = Envelope(
            "shared_vote_request", operationId, "peer:host1", 1, null,
            new { proposal_id = "event:campfire", voter_peer = "peer:host1", choice = "index:1" },
            null, correlation);
        (int status, string response) = runtime.Handle(
            InstanceId, SessionId, LeaseId, correlation, "7",
            "shared_vote_request", request);
        Check(status == 200 && Status(response) == "settled", "shared vote response");
        Write(directory, "shared-vote-settled", "shared_vote_request", request,
            null, status, response);
    }

    private static void CaptureRejoin(string directory)
    {
        var port = new CapturePort(CaptureMode.RejoinPending) { RejoinSettled = false };
        port.DisconnectLocalForRecovery();
        var runtime = new CoopNativeRuntime(port);
        const string operationId = "op:native:rejoin";
        const string requestCorrelation = "corr:native:rejoin";
        string request = Envelope(
            "rejoin_request", operationId, "peer:client1", 1, null, null,
            new { kind = "rejoin", rejoin_epoch = 3UL }, requestCorrelation);
        (int acceptedStatus, string accepted) = runtime.Handle(
            InstanceId, SessionId, LeaseId, requestCorrelation, "7",
            "rejoin_request", request);
        Check(acceptedStatus == 200 && Status(accepted) == "unknown", "rejoin pending response");
        Write(directory, "rejoin-pending", "rejoin_request", request,
            "accepted_pending", acceptedStatus, accepted);

        port.RejoinSettled = true;
        string recovery = Envelope(
            "recovery_response", operationId, null, null, null, null,
            new { kind = "reconcile", rejoin_epoch = 3UL }, "corr:native:rejoin:recovered");
        (int recoveredStatus, string recovered) = runtime.Handle(
            InstanceId, SessionId, LeaseId, "corr:native:rejoin:recovered", "7",
            "recovery_response", recovery);
        Check(recoveredStatus == 200 && Status(recovered) == "settled", "rejoin recovered response");
        Write(directory, "rejoin-recovered", "recovery_response", recovery,
            "reconciled", recoveredStatus, recovered);
    }

    private static string Envelope(
        string kind, string? operationId, string? actorPeer, ulong? expectedGeneration,
        object? action, object? vote, object? recovery, string correlation) =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = ProtocolVersion,
            ["schema_digest"] = SchemaDigest,
            ["provenance"] = new { artifact = Artifact, source = SchemaSource, generator = "hand-authored" },
            ["correlation_id"] = correlation,
            ["instance_id"] = InstanceId,
            ["session_id"] = SessionId,
            ["lease_id"] = LeaseId,
            ["lease_epoch"] = 7UL,
            ["kind"] = kind,
            ["operation_id"] = operationId,
            ["actor_peer"] = actorPeer,
            ["expected_host_generation"] = expectedGeneration,
            ["action"] = action,
            ["vote"] = vote,
            ["status"] = null,
            ["observation"] = null,
            ["effect"] = null,
            ["recovery"] = recovery,
            ["catalog"] = null,
            ["receipt"] = null
        });

    private static void Write(
        string directory, string name, string requestKind, string? request,
        string? transition, int status, string response)
    {
        object fixture = new
        {
            fixture_version = "coop-native-v1-producer-fixture-v1",
            protocol_version = ProtocolVersion,
            artifact = Artifact,
            schema_source = SchemaSource,
            schema_digest = SchemaDigest,
            producer = new
            {
                kind = "managed_source_only",
                implementation = "experiments/managed-rust-interop/game-loader/CoopNativeRuntime",
                host = "CapturePort synthetic host",
                limitation = "does not execute a native STS2 session"
            },
            capture = new
            {
                name,
                request_kind = requestKind,
                transition,
                http_status = status,
                request = request is null ? (JsonElement?)null : Parse(request),
                response = Parse(response)
            }
        };
        string path = Path.Combine(directory, name + ".json");
        if (File.Exists(path) || Directory.Exists(path))
            throw new IOException("refusing to overwrite capture: " + path);
        // Fixtures are checked into the source tree. Keep the byte stream stable on Windows
        // and Linux so the recorded SHA-256 values describe the serialized JSON, not the host
        // newline convention.
        File.WriteAllText(path, JsonSerializer.Serialize(fixture, JsonOptions) + "\n",
            new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
    }

    private static JsonElement Parse(string json)
    {
        using JsonDocument document = JsonDocument.Parse(json, new JsonDocumentOptions { MaxDepth = 24 });
        return document.RootElement.Clone();
    }

    private static string? Status(string json)
    {
        using JsonDocument document = JsonDocument.Parse(json);
        return document.RootElement.GetProperty("status").GetString();
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
