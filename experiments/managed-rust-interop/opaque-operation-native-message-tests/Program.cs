// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using AiAscension.Sts2GameMod.Runtime;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Serialization;

namespace AiAscension.Sts2GameMod.OpaqueOperationNativeMessageTests;

internal static class Program
{
    private static void Main()
    {
        ConcreteMessageRoundTripsWithBoundedPacketMethods();
        FrozenEnvelopeSizedRequestRoundTrips();
        HostRegistersDispatchesAndRepliesToSamePeer();
        HostRejectsClientSettlementClaimWithoutDispatch();
        DuplicateDoesNotCreateNewHostWork();
        ForeignPeerCannotReceiveOwnerWitness();
        ClientAcceptsReplyOnlyFromAuthenticatedHost();
        BoundedIngressDropsSaturationAndRecoversAcrossFrames();
        Console.WriteLine("Concrete opaque native-message component tests passed.");
    }

    private static void ConcreteMessageRoundTripsWithBoundedPacketMethods()
    {
        OpaqueOperationNativeMessage request = OpaqueOperationNativeMessage.Request(
            "op_wire_17", new byte[] { 0x17, 0x42 });
        var writer = new PacketWriter();
        request.Serialize(writer);
        var decoded = new OpaqueOperationNativeMessage();
        decoded.Deserialize(new PacketReader(writer.Values));

        Check(decoded.IsRequest() && decoded.OriginalOperationId == "op_wire_17"
            && decoded.Payload.Length == 2 && decoded.Payload[0] == 0x17,
            "concrete INetMessage uses bounded packet serialization");
    }

    private static void FrozenEnvelopeSizedRequestRoundTrips()
    {
        byte[] payload = new byte[OpaqueOperationNativeMessage.MaximumPayloadBytes];
        payload[0] = (byte)'{';
        payload[^1] = (byte)'}';
        OpaqueOperationNativeMessage request = OpaqueOperationNativeMessage.Request(
            "op_frozen_v1", payload);
        var writer = new PacketWriter();
        request.Serialize(writer);
        var decoded = new OpaqueOperationNativeMessage();
        decoded.Deserialize(new PacketReader(writer.Values));

        Check(decoded.IsRequest() && decoded.Payload.Length == payload.Length
            && decoded.Payload[0] == (byte)'{' && decoded.Payload[^1] == (byte)'}',
            "carrier preserves a bounded frozen-envelope payload without a wire-version change");
    }

    private static void HostRegistersDispatchesAndRepliesToSamePeer()
    {
        var host = new StubHostService();
        var dispatcher = new StubDispatcher(OpaqueOperationHostDispatchResult.Settled("host_witness_17"));
        using var adapter = new OpaqueOperationNativeMessageAdapter(host, dispatcher, _ => false);
        adapter.Register();

        host.Deliver(OpaqueOperationNativeMessage.Request("op_host_17", new byte[] { 1 }), 41);
        Check(host.Targeted.Count == 0,
            "network callback queues host reply until the authoritative frame pump");
        adapter.Pump();

        Check(host.RegisterCount == 1 && dispatcher.DispatchCount == 1 && dispatcher.LastPeerId == 41,
            "registered host handler receives the transport-authenticated sender ID");
        Check(host.Targeted.Count == 1 && host.Targeted[0].PeerId == 41
            && host.Targeted[0].Message.SettlementWitness == "host_witness_17",
            "host settlement reply is sent to exactly the submitting peer");
    }

    private static void DuplicateDoesNotCreateNewHostWork()
    {
        var host = new StubHostService();
        var dispatcher = new StubDispatcher(OpaqueOperationHostDispatchResult.Settled("host_witness_once"));
        using var adapter = new OpaqueOperationNativeMessageAdapter(host, dispatcher, _ => false);
        adapter.Register();

        host.Deliver(OpaqueOperationNativeMessage.Request("op_once", new byte[] { 1 }), 41);
        host.Deliver(OpaqueOperationNativeMessage.Request("op_once", new byte[] { 9 }), 41);
        adapter.Pump();

        Check(dispatcher.DispatchCount == 1 && host.Targeted.Count == 2
            && host.Targeted[0].Message.SettlementWitness == host.Targeted[1].Message.SettlementWitness,
            "retry with original ID replays its reply without dispatching new work");
    }

    private static void HostRejectsClientSettlementClaimWithoutDispatch()
    {
        var host = new StubHostService();
        var dispatcher = new StubDispatcher(OpaqueOperationHostDispatchResult.Settled("unreachable"));
        using var adapter = new OpaqueOperationNativeMessageAdapter(host, dispatcher, _ => false);
        adapter.Register();

        host.Deliver(OpaqueOperationNativeMessage.SettledReply("op_claim", "client_claim"), 41);
        adapter.Pump();

        Check(dispatcher.DispatchCount == 0 && host.Targeted.Count == 1
            && host.Targeted[0].Message.RejectionCode == "invalid_request",
            "client settlement-kind messages never reach host admission or witness issuance");
    }

    private static void ForeignPeerCannotReceiveOwnerWitness()
    {
        var host = new StubHostService();
        var dispatcher = new StubDispatcher(OpaqueOperationHostDispatchResult.Settled("private_witness"));
        using var adapter = new OpaqueOperationNativeMessageAdapter(host, dispatcher, _ => false);
        adapter.Register();

        host.Deliver(OpaqueOperationNativeMessage.Request("op_private", new byte[] { 1 }), 41);
        host.Deliver(OpaqueOperationNativeMessage.Request("op_private", new byte[] { 1 }), 99);
        adapter.Pump();

        Check(dispatcher.DispatchCount == 1 && host.Targeted[1].PeerId == 99
            && host.Targeted[1].Message.SettlementWitness.Length == 0
            && host.Targeted[1].Message.RejectionCode == "operation_owned_by_another_peer",
            "different peer receives rejection rather than owner witness");
    }

    private static void ClientAcceptsReplyOnlyFromAuthenticatedHost()
    {
        var client = new StubClientService();
        var replies = new List<OpaqueOperationNativeMessage>();
        using var adapter = new OpaqueOperationNativeMessageAdapter(client, null, peerId => peerId == 7,
            replies.Add);
        adapter.Register();
        adapter.SendRequest("op_client", new byte[] { 1 });

        client.Deliver(OpaqueOperationNativeMessage.SettledReply("op_client", "host_witness"), 9);
        client.Deliver(OpaqueOperationNativeMessage.SettledReply("op_client", "host_witness"), 7);
        adapter.Pump();

        Check(client.Outbound.Count == 1 && client.Outbound[0].IsRequest()
            && replies.Count == 1 && replies[0].SettlementWitness == "host_witness",
            "client sends requests and accepts replies only from the authenticated host peer");
    }

    private static void BoundedIngressDropsSaturationAndRecoversAcrossFrames()
    {
        var host = new StubHostService();
        var dispatcher = new StubDispatcher(OpaqueOperationHostDispatchResult.Settled("host_witness"));
        using var adapter = new OpaqueOperationNativeMessageAdapter(host, dispatcher, _ => false);
        adapter.Register();
        int offered = OpaqueOperationNativeMessageAdapter.MaximumInboundMessages + 64;

        Parallel.For(0, offered, index => host.Deliver(OpaqueOperationNativeMessage.Request(
            $"op_burst_{index:D4}", new byte[] { 1 }), 41));

        Check(adapter.PendingInboundMessagesForTest
                == OpaqueOperationNativeMessageAdapter.MaximumInboundMessages
            && adapter.DroppedInboundMessagesForTest == 64 && host.Targeted.Count == 0,
            "concurrent callbacks reserve a bounded owned ingress backlog without replies");
        for (int frame = 0; frame < 8; frame++)
            adapter.Pump();
        Check(dispatcher.DispatchCount == OpaqueOperationNativeMessageAdapter.MaximumInboundMessages
            && host.Targeted.Count == OpaqueOperationNativeMessageAdapter.MaximumInboundMessages
            && adapter.PendingInboundMessagesForTest == 0,
            "frame pumps drain bounded ingress without manufacturing settlement for dropped messages");

        host.Deliver(OpaqueOperationNativeMessage.Request("op_burst_recovery", new byte[] { 1 }), 41);
        adapter.Pump();
        Check(dispatcher.DispatchCount == OpaqueOperationNativeMessageAdapter.MaximumInboundMessages + 1
            && host.Targeted[^1].Message.SettlementWitness == "host_witness",
            "ingress recovers after bounded backlog drains and preserves host-issued settlement");
    }

    private static void Check(bool condition, string message)
    {
        if (!condition)
            throw new InvalidOperationException(message);
    }

    private sealed class StubDispatcher : IHostAdmittedOpaqueOperationDispatcher
    {
        private readonly OpaqueOperationHostDispatchResult _result;

        internal StubDispatcher(OpaqueOperationHostDispatchResult result) => _result = result;
        internal int DispatchCount { get; private set; }
        internal ulong LastPeerId { get; private set; }

        public OpaqueOperationHostDispatchResult Dispatch(ulong authenticatedPeerId,
            string originalOperationId, byte[] payload)
        {
            DispatchCount++;
            LastPeerId = authenticatedPeerId;
            return _result;
        }

        public OpaqueOperationHostDispatchResult Reconcile(ulong authenticatedPeerId,
            string originalOperationId) => _result;
    }

    private abstract class StubService : INetGameService
    {
        private MessageHandlerDelegate<OpaqueOperationNativeMessage>? _handler;

        internal int RegisterCount { get; private set; }
        internal List<(OpaqueOperationNativeMessage Message, ulong PeerId)> Targeted { get; } = new();
        internal List<OpaqueOperationNativeMessage> Outbound { get; } = new();

        public void RegisterMessageHandler<T>(MessageHandlerDelegate<T> handler)
            where T : INetMessage
        {
            RegisterCount++;
            _handler = (MessageHandlerDelegate<OpaqueOperationNativeMessage>)(object)handler;
        }

        public void UnregisterMessageHandler<T>(MessageHandlerDelegate<T> handler)
            where T : INetMessage => _handler = null;

        public void SendMessage<T>(T message) where T : INetMessage
        {
            Outbound.Add((OpaqueOperationNativeMessage)(object)message);
        }

        public void SendMessage<T>(T message, ulong peerId) where T : INetMessage =>
            Targeted.Add(((OpaqueOperationNativeMessage)(object)message, peerId));

        internal void Deliver(OpaqueOperationNativeMessage message, ulong senderId) =>
            _handler?.Invoke(message, senderId);
    }

    private sealed class StubHostService : StubService, INetHostGameService
    {
    }

    private sealed class StubClientService : StubService
    {
    }
}
