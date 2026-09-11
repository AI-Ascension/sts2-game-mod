// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.OpaqueOperationAdapter;

internal static class OpaqueOperationAdapterTests
{
    internal static void Run()
    {
        SettledReplyUsesSubmittingAuthenticatedLink();
        DuplicateDoesNotDispatchNewWork();
        AnotherPeerCannotReceiveTheRecordedReply();
        ClientInputCannotIssueSettlementWitness();
        PayloadIsOwnedBeforeHostDispatch();
        BoundsRejectBeforeHostAdmission();
    }

    private static void SettledReplyUsesSubmittingAuthenticatedLink()
    {
        var host = new StubHost(HostDispatchResult.Settled("host-witness-17"));
        var origin = new StubPeer("peer-a");
        var adapter = new OpaqueOperationAdapter(host);

        adapter.Handle(origin, Request("op_opaque_17"));

        Check(host.DispatchCount == 1, "accepted request reaches host dispatch once");
        Check(host.LastOperationId == "op_opaque_17", "host receives the original opaque ID");
        Check(origin.Replies.Count == 1 && origin.Replies[0].HostSettlementWitness == "host-witness-17",
            "only host dispatch produces the settlement witness on the origin link");
    }

    private static void DuplicateDoesNotDispatchNewWork()
    {
        var host = new StubHost(HostDispatchResult.Settled("host-witness-duplicate"));
        var origin = new StubPeer("peer-a");
        var adapter = new OpaqueOperationAdapter(host);

        adapter.Handle(origin, Request("op_same"));
        adapter.Handle(origin, Request("op_same"));

        Check(host.DispatchCount == 1, "same original ID never retries as new host work");
        Check(origin.Replies.Count == 2
            && origin.Replies[0].HostSettlementWitness == origin.Replies[1].HostSettlementWitness,
            "same authenticated peer receives its recorded reply");
    }

    private static void AnotherPeerCannotReceiveTheRecordedReply()
    {
        var host = new StubHost(HostDispatchResult.Settled("host-witness-private"));
        var owner = new StubPeer("peer-owner");
        var other = new StubPeer("peer-other");
        var adapter = new OpaqueOperationAdapter(host);

        adapter.Handle(owner, Request("op_private"));
        adapter.Handle(other, Request("op_private"));

        Check(host.DispatchCount == 1, "foreign peer cannot cause a second dispatch");
        Check(other.Replies.Count == 1 && other.Replies[0].RejectionCode == "operation_owned_by_another_peer"
            && other.Replies[0].HostSettlementWitness is null,
            "foreign peer receives no owner witness");
    }

    private static void ClientInputCannotIssueSettlementWitness()
    {
        var host = new StubHost(HostDispatchResult.Rejected("host_rejected"));
        var origin = new StubPeer("peer-a");
        var adapter = new OpaqueOperationAdapter(host);

        adapter.Handle(origin, Request("op_client_claim", new byte[] { 1, 2, 3 }));

        Check(origin.Replies.Count == 1 && origin.Replies[0].Disposition == "rejected"
            && origin.Replies[0].HostSettlementWitness is null,
            "request payload has no client settlement-witness field");
    }

    private static void PayloadIsOwnedBeforeHostDispatch()
    {
        var host = new StubHost(HostDispatchResult.Rejected("host_rejected"));
        var origin = new StubPeer("peer-a");
        var adapter = new OpaqueOperationAdapter(host);
        byte[] sourcePayload = { 0x17 };
        OpaqueOperationRequest request = Request("op_owned_payload", sourcePayload);
        sourcePayload[0] = 0x99;

        adapter.Handle(origin, request);

        Check(host.LastPayload is not null && host.LastPayload[0] == 0x17,
            "host dispatch receives an owned payload snapshot");
    }

    private static void BoundsRejectBeforeHostAdmission()
    {
        var host = new StubHost(HostDispatchResult.Settled("unreachable"));
        var origin = new StubPeer("peer-a");
        var adapter = new OpaqueOperationAdapter(host);

        adapter.Handle(origin, Request(new string('a', OpaqueOriginalOperationId.MaximumLength + 1)));
        adapter.Handle(origin, Request("op_large", new byte[OpaqueOperationRequest.MaximumPayloadBytes + 1]));

        Check(host.DispatchCount == 0, "invalid bounded input does not reach host admission");
        Check(origin.Replies.Count == 2 && origin.Replies[0].RejectionCode == "invalid_original_operation_id"
            && origin.Replies[1].RejectionCode == "payload_too_large", "bounds have deterministic replies");
    }

    private static OpaqueOperationRequest Request(string operationId, byte[]? payload = null) =>
        new(operationId, payload ?? new byte[] { 0x17 });

    private static void Check(bool condition, string message)
    {
        if (!condition)
            throw new InvalidOperationException(message);
    }

    private sealed class StubHost : IHostAdmittedActionDispatcher
    {
        private readonly HostDispatchResult _result;

        internal StubHost(HostDispatchResult result) => _result = result;

        internal int DispatchCount { get; private set; }
        internal string? LastOperationId { get; private set; }
        internal byte[]? LastPayload { get; private set; }

        public HostDispatchResult Dispatch(HostAdmittedOperation operation)
        {
            DispatchCount++;
            LastOperationId = operation.OriginalOperationId.Value;
            LastPayload = new byte[operation.Payload.Length];
            Array.Copy(operation.Payload, LastPayload, operation.Payload.Length);
            return _result;
        }
    }

    private sealed class StubPeer : IAuthenticatedPeerLink
    {
        internal StubPeer(string peerId) => PeerId = peerId;

        public string PeerId { get; }
        internal List<OpaqueOperationReply> Replies { get; } = new();

        public void Send(OpaqueOperationReply reply) => Replies.Add(reply);
    }
}
