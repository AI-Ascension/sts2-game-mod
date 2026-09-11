// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Serialization;

namespace AiAscension.Sts2GameMod.OpaqueOperationCompositionTests;

internal static class Program
{
    private const string Digest = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    private static void Main()
    {
        ClientRetainsClosedEnvelopeAndAcceptsOnlyHostReply();
        ClientRejectsStaleAndDuplicatePendingRequests();
        HostMapsSameNativePeerAndRepliesOnlyAfterPumps();
        HostRejectsForeignActorBeforeNativeAction();
        TerminalRejectionsExpireUnderBoundedPressure();
        Console.WriteLine("Opaque operation client-host composition tests passed.");
    }

    private static void ClientRetainsClosedEnvelopeAndAcceptsOnlyHostReply()
    {
        var port = new FakePort(CoopHostRole.Client);
        var service = new ClientService();
        OpaqueOperationNativeMessageAdapter? adapter = null;
        var bridge = new CoopOpaqueOperationClientBridge(() => adapter);
        adapter = new OpaqueOperationNativeMessageAdapter(service, null, id => id == 101,
            bridge.ReceiveAuthenticatedHostReply);
        adapter.Register();
        var runtime = new CoopNativeRuntime(port);
        string body = Envelope("op:client:one", "peer:client1", 1);

        Check(bridge.TrySubmit(runtime, "local_action_request", "instance:test", "session:test",
                "lease:test", "corr:test", "7", body, out string operationId, out _)
            && operationId == "op:client:one" && service.Outbound.Count == 1
            && Encoding.UTF8.GetString(service.Outbound[0].Payload) == body,
            "client retains exact closed v1 bytes and original operation identity");

        service.Deliver(OpaqueOperationNativeMessage.SettledReply(operationId, "host_witness"), 77);
        adapter.Pump();
        Check(!bridge.TryReconcile(out _, out _), "foreign sender cannot complete client operation");
        service.Deliver(OpaqueOperationNativeMessage.SettledReply(operationId, "host_witness"), 101);
        adapter.Pump();
        Check(bridge.TryReconcile(out OpaqueOperationNativeMessage? reply, out string original)
            && reply?.Kind == OpaqueOperationNativeMessageKind.SettledReply && original == body,
            "only authenticated host reply returns to original client operation");
    }

    private static void ClientRejectsStaleAndDuplicatePendingRequests()
    {
        var port = new FakePort(CoopHostRole.Client);
        var service = new ClientService();
        OpaqueOperationNativeMessageAdapter? adapter = null;
        var bridge = new CoopOpaqueOperationClientBridge(() => adapter);
        adapter = new OpaqueOperationNativeMessageAdapter(service, null, id => id == 101,
            bridge.ReceiveAuthenticatedHostReply);
        adapter.Register();
        var runtime = new CoopNativeRuntime(port);
        string first = Envelope("op:client:pending", "peer:client1", 1);
        Check(bridge.TrySubmit(runtime, "local_action_request", "instance:test", "session:test",
                "lease:test", "corr:test", "7", first, out _, out _), "first request admitted");
        Check(!bridge.TrySubmit(runtime, "local_action_request", "instance:test", "session:test",
                "lease:test", "corr:test", "7", first, out _, out string duplicate)
            && duplicate == "coop_native_duplicate_pending_operation",
            "duplicate pending operation is not sent again");
        var staleRuntime = new CoopNativeRuntime(new FakePort(CoopHostRole.Client));
        string stale = Envelope("op:client:stale", "peer:client1", 0);
        Check(!bridge.TrySubmit(staleRuntime, "local_action_request", "instance:test", "session:test",
                "lease:test", "corr:test", "7", stale, out _, out string staleError)
            && staleError == "coop_native_client_not_admitted",
            "stale generation is rejected before carrier send");
    }

    private static void HostMapsSameNativePeerAndRepliesOnlyAfterPumps()
    {
        var port = new FakePort(CoopHostRole.Host);
        var runtime = new CoopNativeRuntime(port);
        var dispatcher = new CoopOpaqueOperationHostDispatcher(runtime);
        var service = new HostService();
        using var adapter = new OpaqueOperationNativeMessageAdapter(service, dispatcher, _ => false);
        adapter.Register();
        string body = Envelope("op:host:same", "peer:client1", 1);
        service.Deliver(OpaqueOperationNativeMessage.Request("op:host:same", Encoding.UTF8.GetBytes(body)), 202);
        Check(service.Targeted.Count == 0, "network callback cannot emit a host reply directly");
        adapter.Pump();
        Check(service.Targeted.Count == 1 && service.Targeted[0].Peer == 202
            && service.Targeted[0].Message.Kind == OpaqueOperationNativeMessageKind.PendingReply,
            "first response is queued and targeted to authenticated submitting peer");
        dispatcher.Pump();
        adapter.Pump();
        Check(service.Targeted.Count == 2 && service.Targeted[1].Peer == 202
            && service.Targeted[1].Message.Kind == OpaqueOperationNativeMessageKind.SettledReply
            && port.ActionCount == 1,
            "same peer receives host-issued terminal reply after the dispatcher and adapter pumps");
        service.Deliver(OpaqueOperationNativeMessage.Request("op:host:same", Encoding.UTF8.GetBytes(body)), 202);
        adapter.Pump();
        Check(service.Targeted.Count == 3 && service.Targeted[2].Message.Kind
                == OpaqueOperationNativeMessageKind.SettledReply && port.ActionCount == 1,
            "same-peer retry replays its terminal reply without a duplicate native action");
    }

    private static void HostRejectsForeignActorBeforeNativeAction()
    {
        var port = new FakePort(CoopHostRole.Host);
        var runtime = new CoopNativeRuntime(port);
        var dispatcher = new CoopOpaqueOperationHostDispatcher(runtime);
        var service = new HostService();
        using var adapter = new OpaqueOperationNativeMessageAdapter(service, dispatcher, _ => false);
        adapter.Register();
        string body = Envelope("op:host:foreign", "peer:host1", 1);
        service.Deliver(OpaqueOperationNativeMessage.Request("op:host:foreign", Encoding.UTF8.GetBytes(body)), 202);
        adapter.Pump();
        dispatcher.Pump();
        service.Deliver(OpaqueOperationNativeMessage.Request("op:host:foreign", Encoding.UTF8.GetBytes(body)), 202);
        adapter.Pump();
        Check(service.Targeted[^1].Message.Kind == OpaqueOperationNativeMessageKind.RejectedReply
            && port.ActionCount == 0,
            "foreign v1 actor cannot reach host legality or native action");
    }

    private static void TerminalRejectionsExpireUnderBoundedPressure()
    {
        var port = new FakePort(CoopHostRole.Host);
        var runtime = new CoopNativeRuntime(port);
        var dispatcher = new CoopOpaqueOperationHostDispatcher(runtime);
        var service = new HostService();
        using var adapter = new OpaqueOperationNativeMessageAdapter(service, dispatcher, _ => false);
        adapter.Register();

        for (int index = 0; index < 4096; index++)
        {
            string operationId = $"op:capacity:{index:D4}";
            service.Deliver(OpaqueOperationNativeMessage.Request(operationId,
                Encoding.UTF8.GetBytes("[")), 202);
            adapter.Pump();
            dispatcher.Pump();
            adapter.Pump();
        }
        Check(service.Targeted[^1].Message.Kind == OpaqueOperationNativeMessageKind.RejectedReply,
            "invalid carrier records become terminal before bounded retention pressure");

        string body = Envelope("op:capacity:recovered", "peer:client1", 1);
        service.Deliver(OpaqueOperationNativeMessage.Request("op:capacity:recovered",
            Encoding.UTF8.GetBytes(body)), 202);
        adapter.Pump();
        dispatcher.Pump();
        adapter.Pump();

        Check(port.ActionCount == 1 && service.Targeted[^1].Message.Kind
                == OpaqueOperationNativeMessageKind.SettledReply
            && !service.Targeted.Exists(reply => reply.Message.RejectionCode
                == "operation_capacity_exhausted"),
            "terminal rejection retention expires under pressure while a new valid operation settles");
    }

    private static string Envelope(string operationId, string actor, ulong generation) =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = "coop-native-v1",
            ["schema_digest"] = "2f3bc99e53080fa11b39592b64fb0ab964a16f568719a2622d0b2caf766ab629",
            ["provenance"] = new { artifact = "sts2-protocol/coop-native-v1", source = "schemas/coop-native-v1.schema.json", generator = "hand-authored" },
            ["correlation_id"] = "corr:test", ["instance_id"] = "instance:test",
            ["session_id"] = "session:test", ["lease_id"] = "lease:test", ["lease_epoch"] = 7UL,
            ["kind"] = "local_action_request", ["operation_id"] = operationId,
            ["actor_peer"] = actor, ["expected_host_generation"] = generation,
            ["action"] = new { kind = "end_turn", action_id = "turn:1", target_peer = (string?)null },
            ["vote"] = null, ["status"] = null, ["observation"] = null, ["effect"] = null,
            ["recovery"] = null, ["catalog"] = null, ["receipt"] = null
        });

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }

    private sealed class FakePort : ICoopNativeHostPort
    {
        private readonly CoopHostRole _role;
        private ulong _generation = 1;
        internal int ActionCount { get; private set; }
        internal FakePort(CoopHostRole role) => _role = role;
        public CoopHostObservation Observe()
        {
            string local = _role == CoopHostRole.Host ? "peer:host1" : "peer:client1";
            return new CoopHostObservation("session:test", local, _role, "lan", "lobby:test", _generation,
                Digest, new[]
                {
                    Peer(local, true, _generation), Peer(local == "peer:host1" ? "peer:client1" : "peer:host1", false, _generation)
                }, false, null)
            { AuthorityId = "authority:test", AuthorityEpoch = "epoch:test", RunId = "run:test",
                CheckpointId = "checkpoint:1", ChecksumStatus = "available", HostDigestKnown = true };
        }
        private static CoopPeerSnapshot Peer(string id, bool local, ulong generation) => new(id, local, true, generation, Digest)
        { AuthorityId = "authority:test", AuthorityEpoch = "epoch:test", CheckpointId = "checkpoint:1", DigestKnown = true };
        public bool TryResolvePeer(string peer, out CoopNativePeerBinding binding)
        {
            binding = peer switch { "peer:host1" => new(peer, 101, true), "peer:client1" => new(peer, 202, true), _ => null! };
            return binding is not null;
        }
        public bool TryResolveNativePeer(ulong native, out CoopNativePeerBinding binding)
        {
            binding = native == 202 ? new("peer:client1", 202, true) : null!;
            return binding is not null;
        }
        public CoopNativeDispatchResult DispatchLocalAction(CoopLocalActionRequest request)
        {
            ActionCount++;
            _generation = 2;
            return new(CoopOutcome.Settled, new CoopEffectWitness(request.OperationId, "effect:test", "turn_ended", 1, 2, Digest)
            { AuthorityId = "authority:test", AuthorityEpoch = "epoch:test", CheckpointId = "checkpoint:1" }, null);
        }
        public CoopNativeDispatchResult SubmitSharedVote(CoopSharedVoteRequest request) => CoopNativeDispatchResult.Rejected("unexpected_vote");
        public CoopNativeDispatchResult Rejoin(string peer, ulong epoch) => CoopNativeDispatchResult.Rejected("unexpected_rejoin");
        public CoopEffectWitness? Reconcile(string operationId) => null;
    }

    private abstract class Service : INetGameService
    {
        private MessageHandlerDelegate<OpaqueOperationNativeMessage>? _handler;
        internal readonly List<OpaqueOperationNativeMessage> Outbound = new();
        internal readonly List<(OpaqueOperationNativeMessage Message, ulong Peer)> Targeted = new();
        public void RegisterMessageHandler<T>(MessageHandlerDelegate<T> handler) where T : INetMessage => _handler = (MessageHandlerDelegate<OpaqueOperationNativeMessage>)(object)handler;
        public void UnregisterMessageHandler<T>(MessageHandlerDelegate<T> handler) where T : INetMessage => _handler = null;
        public void SendMessage<T>(T message) where T : INetMessage => Outbound.Add((OpaqueOperationNativeMessage)(object)message);
        public void SendMessage<T>(T message, ulong peer) where T : INetMessage => Targeted.Add(((OpaqueOperationNativeMessage)(object)message, peer));
        internal void Deliver(OpaqueOperationNativeMessage message, ulong sender) => _handler?.Invoke(message, sender);
    }
    private sealed class ClientService : Service { }
    private sealed class HostService : Service, INetHostGameService { }
}
