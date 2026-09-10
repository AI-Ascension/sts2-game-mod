// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.CoopSharedGateTests;

using AiAscension.Sts2GameMod.Runtime;

internal static class Program
{
    private const string Digest =
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    private static void Main()
    {
        ParserRejectsUnrecognizedFields();
        PendingRuntimeV2BlocksCoop();
        PendingRuntimeV3BlocksCoop();
        PendingRuntimeV4BlocksCoop();
        PendingSeededRunBlocksCoop();
        ReleasedProfilesAdmitCoop();
        ReconciliationRemainsAvailableWhileAnotherProfileIsPending();
        Console.WriteLine("Shared co-op mutation gate checks passed.");
    }

    private static void ParserRejectsUnrecognizedFields()
    {
        var port = new FakePort();
        var runtime = new CoopNativeRuntime(port);
        (int status, _) = runtime.TestHandle(
            "instance:one", "session:one", "lease:one", "corr:one", "1",
            "local_action_request", ValidActionRequest());
        Check(status == 200, "the managed co-op parser accepts a closed action envelope");

        (status, string catalogResponse) = runtime.TestHandle(
            "instance:one", "session:one", "lease:one", "corr:catalog", "1",
            "legal_catalog_request", ValidCatalogRequest(2));
        Check(status == 200
            && catalogResponse.Contains("\"kind\":\"legal_catalog_response\"", StringComparison.Ordinal)
            && catalogResponse.Contains("\"catalog\":", StringComparison.Ordinal)
            && catalogResponse.Contains("\"receipt\":null", StringComparison.Ordinal),
            "the managed co-op catalog route returns a generation-bound catalog response");

        string unknownRoot = ValidActionRequest().Replace(
            "\"recovery\":null,\"catalog\":null,\"receipt\":null}", "\"recovery\":null,\"catalog\":null,\"receipt\":null,\"unexpected\":null}",
            StringComparison.Ordinal);
        (status, _) = runtime.TestHandle(
            "instance:one", "session:one", "lease:one", "corr:one", "1",
            "local_action_request", unknownRoot);
        Check(status == 400, "the managed co-op parser rejects unknown root fields");

        string unknownAction = ValidActionRequest().Replace(
            "\"target_peer\":null}", "\"target_peer\":null,\"unexpected\":null}",
            StringComparison.Ordinal);
        (status, _) = runtime.TestHandle(
            "instance:one", "session:one", "lease:one", "corr:one", "1",
            "local_action_request", unknownAction);
        Check(status == 400, "the managed co-op parser rejects unknown action fields");
    }

    private static string ValidActionRequest() =>
        "{\"protocol_version\":\"coop-native-v1\",\"schema_digest\":\""
        + "9c24c6d0dbcc3b52c2c504b2a60c9faf9d02f902c00cf16712a2a810c2358391"
        + "\",\"provenance\":{\"artifact\":\"sts2-protocol/coop-native-v1\","
        + "\"source\":\"schemas/coop-native-v1.schema.json\",\"generator\":\"hand-authored\"},"
        + "\"correlation_id\":\"corr:one\",\"instance_id\":\"instance:one\","
        + "\"session_id\":\"session:one\",\"lease_id\":\"lease:one\",\"lease_epoch\":1,"
        + "\"kind\":\"local_action_request\",\"operation_id\":\"op:parser\","
        + "\"actor_peer\":\"peer:host1\",\"expected_host_generation\":1,"
        + "\"action\":{\"kind\":\"end_turn\",\"action_id\":\"end_turn\","
        + "\"target_peer\":null},\"vote\":null,\"status\":null,"
        + "\"observation\":null,\"effect\":null,\"recovery\":null,"
        + "\"catalog\":null,\"receipt\":null}";

    private static string ValidCatalogRequest(ulong generation) =>
        "{\"protocol_version\":\"coop-native-v1\",\"schema_digest\":\""
        + "9c24c6d0dbcc3b52c2c504b2a60c9faf9d02f902c00cf16712a2a810c2358391"
        + "\",\"provenance\":{\"artifact\":\"sts2-protocol/coop-native-v1\","
        + "\"source\":\"schemas/coop-native-v1.schema.json\",\"generator\":\"hand-authored\"},"
        + "\"correlation_id\":\"corr:catalog\",\"instance_id\":\"instance:one\","
        + "\"session_id\":\"session:one\",\"lease_id\":\"lease:one\",\"lease_epoch\":1,"
        + "\"kind\":\"legal_catalog_request\",\"operation_id\":null,"
        + "\"actor_peer\":\"peer:host1\",\"expected_host_generation\":"
        + generation.ToString(System.Globalization.CultureInfo.InvariantCulture) + ","
        + "\"action\":null,\"vote\":null,\"status\":null,\"observation\":null,"
        + "\"effect\":null,\"recovery\":null,\"catalog\":null,\"receipt\":null}";

    private static void PendingRuntimeV2BlocksCoop() =>
        PendingProfileBlocksCoop("runtime-v2", runtimeV2: true, runtimeV3: false, runtimeV4: false);

    private static void PendingRuntimeV3BlocksCoop() =>
        PendingProfileBlocksCoop("runtime-v3", runtimeV2: false, runtimeV3: true, runtimeV4: false);

    private static void PendingRuntimeV4BlocksCoop() =>
        PendingProfileBlocksCoop("runtime-v4", runtimeV2: false, runtimeV3: false, runtimeV4: true);

    private static void PendingSeededRunBlocksCoop()
    {
        var port = new FakePort();
        ModEntry.SetPendingProfiles(runtimeV2: false, runtimeV3: false, runtimeV4: false, seeded: true);
        ModEntry.ConfigureCoopNative(port);
        CoopOperationReceipt receipt = ModEntry.DispatchTest(Request("op:seeded"));
        Check(receipt.Outcome == CoopOutcome.Rejected
            && receipt.ErrorCode == "operation_in_progress"
            && port.DispatchCount == 0,
            "pending seeded run blocks co-op dispatch before native mutation");
    }

    private static void PendingProfileBlocksCoop(
        string profile, bool runtimeV2, bool runtimeV3, bool runtimeV4)
    {
        var port = new FakePort();
        ModEntry.SetPendingProfiles(runtimeV2, runtimeV3, runtimeV4);
        ModEntry.ConfigureCoopNative(port);
        CoopOperationReceipt receipt = ModEntry.DispatchTest(Request("op:" + profile));
        Check(receipt.Outcome == CoopOutcome.Rejected
            && receipt.ErrorCode == "operation_in_progress"
            && port.DispatchCount == 0,
            profile + " pending mutation blocks co-op dispatch before native mutation");
    }

    private static void ReleasedProfilesAdmitCoop()
    {
        var port = new FakePort();
        ModEntry.SetPendingProfiles(runtimeV2: false, runtimeV3: false, runtimeV4: false);
        ModEntry.ConfigureCoopNative(port);
        CoopOperationReceipt receipt = ModEntry.DispatchTest(Request("op:released"));
        Check(receipt.Outcome == CoopOutcome.Settled && port.DispatchCount == 1,
            "released shared profiles admit and settle co-op dispatch");
    }

    private static void ReconciliationRemainsAvailableWhileAnotherProfileIsPending()
    {
        var port = new FakePort { ReturnUnknown = true };
        ModEntry.SetPendingProfiles(runtimeV2: false, runtimeV3: false, runtimeV4: false);
        ModEntry.ConfigureCoopNative(port);
        CoopOperationReceipt pending = ModEntry.DispatchTest(Request("op:reconcile"));
        Check(pending.Outcome == CoopOutcome.Unknown && ModEntry.HasPendingCoopMutation
            && port.DispatchCount == 1,
            "unknown co-op mutation remains pending without a duplicate native call");

        port.EffectPublished = true;
        ModEntry.SetPendingProfiles(runtimeV2: false, runtimeV3: false, runtimeV4: true);
        Check(ModEntry.ReconcileTest("op:reconcile", out CoopOperationReceipt? settled)
            && settled?.Outcome == CoopOutcome.Settled
            && !ModEntry.HasPendingCoopMutation
            && port.DispatchCount == 1,
            "co-op reconciliation remains available while runtime-v4 is pending");
    }

    private static CoopLocalActionRequest Request(string operationId) =>
        new(operationId, 1, "peer:host1", "end_turn", null, null);

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
        Console.WriteLine("PASS: " + message);
    }

    private sealed class FakePort : ICoopNativeHostPort
    {
        private readonly Dictionary<string, CoopNativePeerBinding> _bindings = new(StringComparer.Ordinal)
        {
            ["peer:host1"] = new("peer:host1", 101, true),
            ["peer:client1"] = new("peer:client1", 202, true)
        };

        internal bool ReturnUnknown { get; init; }
        internal bool EffectPublished { get; set; }
        internal int DispatchCount { get; private set; }

        public CoopHostObservation Observe()
        {
            ulong generation = EffectPublished ? 2UL : 1UL;
            return new CoopHostObservation(
                "session:one", "peer:host1", CoopHostRole.Host, "lan", "lobby:one",
                generation, Digest,
                new[]
                {
                    new CoopPeerSnapshot("peer:host1", true, true, generation, Digest),
                    new CoopPeerSnapshot("peer:client1", false, true, generation, Digest)
                },
                false, null)
            {
                AuthorityId = "authority:test",
                AuthorityEpoch = "epoch:test",
                RunId = "run:test",
                CheckpointId = "checkpoint:test",
                ChecksumStatus = "available",
                HostDigestKnown = true
            };
        }

        public bool TryResolvePeer(string opaquePeerId, out CoopNativePeerBinding binding) =>
            _bindings.TryGetValue(opaquePeerId, out binding!);

        public CoopNativeDispatchResult DispatchLocalAction(CoopLocalActionRequest request)
        {
            DispatchCount++;
            if (ReturnUnknown) return CoopNativeDispatchResult.Unknown("native_outcome_unknown");
            EffectPublished = true;
            return new(CoopOutcome.Accepted,
                new CoopEffectWitness(request.OperationId, "effect:1", "turn_ended", 1, 2, Digest),
                null);
        }

        public CoopNativeDispatchResult SubmitSharedVote(CoopSharedVoteRequest request) =>
            CoopNativeDispatchResult.Rejected("unused");

        public CoopNativeDispatchResult Rejoin(string opaquePeerId, ulong rejoinEpoch) =>
            CoopNativeDispatchResult.Rejected("unused");

        public CoopEffectWitness? Reconcile(string operationId) =>
            EffectPublished
                ? new CoopEffectWitness(operationId, "effect:1", "turn_ended", 1, 2, Digest)
                : null;
    }
}
