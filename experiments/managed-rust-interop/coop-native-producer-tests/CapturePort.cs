// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.CoopNativeProducerTests;

using AiAscension.Sts2GameMod.Runtime;

internal sealed class CapturePort : ICoopNativeHostPort
{
    private const string InitialDigest =
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    private const string SettledDigest =
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    private readonly Dictionary<string, CoopNativePeerBinding> _bindings = new(StringComparer.Ordinal)
    {
        ["peer:host1"] = new("peer:host1", 101, true),
        ["peer:client1"] = new("peer:client1", 202, true)
    };

    internal CapturePort(CaptureMode mode) => Mode = mode;

    internal CaptureMode Mode { get; }
    internal int Generation { get; private set; } = 1;
    internal bool LocalConnected { get; private set; } = true;
    internal bool EffectPublished { get; private set; }
    internal bool RejoinSettled { get; set; } = true;

    public CoopHostObservation Observe()
    {
        bool rejoinMode = Mode is CaptureMode.RejoinPending or CaptureMode.RejoinRecovered;
        string localPeer = rejoinMode ? "peer:client1" : "peer:host1";
        bool hostRole = !rejoinMode;
        bool recovered = rejoinMode && LocalConnected;
        string digest = EffectPublished ? SettledDigest : InitialDigest;
        string checkpoint = EffectPublished ? "checkpoint:2" : "checkpoint:1";
        return new CoopHostObservation(
            rejoinMode ? "session:client" : "session:host",
            localPeer,
            hostRole ? CoopHostRole.Host : CoopHostRole.Client,
            "lan",
            "lobby:one",
            (ulong)Generation,
            digest,
            new[]
            {
                new CoopPeerSnapshot(localPeer, true, LocalConnected, (ulong)Generation, digest)
                {
                    AuthorityId = "authority:native-test",
                    AuthorityEpoch = "epoch:native-test",
                    CheckpointId = checkpoint,
                    DigestKnown = true
                },
                new CoopPeerSnapshot(hostRole ? "peer:client1" : "peer:host1", false, true,
                    (ulong)Generation, digest)
                {
                    AuthorityId = "authority:native-test",
                    AuthorityEpoch = "epoch:native-test",
                    CheckpointId = checkpoint,
                    DigestKnown = true
                }
            },
            !recovered && rejoinMode,
            null)
        {
            AuthorityId = "authority:native-test",
            AuthorityEpoch = "epoch:native-test",
            RunId = "run:native-test",
            CheckpointId = checkpoint,
            ChecksumStatus = "available",
            HostDigestKnown = true
        };
    }

    public bool TryResolvePeer(string opaquePeerId, out CoopNativePeerBinding binding) =>
        _bindings.TryGetValue(opaquePeerId, out binding!);

    public CoopNativeLegalCatalog LegalCatalog(CoopHostObservation observation) =>
        new(observation.HostGeneration, observation.LocalPeerId,
            new[]
            {
                new CoopNativeLegalAction("turn:1", "end_turn", null),
                new CoopNativeLegalAction("card:strike:0", "play_card", null)
            },
            new[]
            {
                new CoopNativeLegalVote("event:campfire", observation.LocalPeerId, "index:1")
            });

    public CoopNativeDispatchResult DispatchLocalAction(CoopLocalActionRequest request)
    {
        if (Mode == CaptureMode.ActionRejected)
            return CoopNativeDispatchResult.Rejected("native_action_kind_not_supported");
        if (Mode == CaptureMode.ActionUnknown)
            return CoopNativeDispatchResult.Unknown("native_action_dispatch_outcome_unknown");
        PublishEffect();
        return Effect(request.OperationId, "turn_ended");
    }

    public CoopNativeDispatchResult SubmitSharedVote(CoopSharedVoteRequest request)
    {
        PublishEffect();
        return Effect(request.OperationId, "shared_event_vote");
    }

    public CoopNativeDispatchResult Rejoin(string opaquePeerId, ulong rejoinEpoch)
    {
        if (Mode is not (CaptureMode.RejoinPending or CaptureMode.RejoinRecovered))
            return CoopNativeDispatchResult.Rejected("native_rejoin_requires_local_client");
        LocalConnected = true;
        return CoopNativeDispatchResult.Accepted();
    }

    public bool IsRejoinSettled() => RejoinSettled;

    public CoopEffectWitness? Reconcile(string operationId) =>
        EffectPublished ? new CoopEffectWitness(
            operationId, $"effect:{operationId}",
            Mode == CaptureMode.VoteSettled ? "shared_event_vote" : "turn_ended",
            1, (ulong)Generation, SettledDigest)
        {
            AuthorityId = "authority:native-test",
            AuthorityEpoch = "epoch:native-test",
            CheckpointId = "checkpoint:2",
            NativeChecksum = SettledDigest
        } : null;

    internal void DisconnectLocalForRecovery() => LocalConnected = false;

    internal void PublishForRecovery()
    {
        EffectPublished = true;
        Generation = 2;
    }

    private void PublishEffect()
    {
        EffectPublished = true;
        Generation = 2;
    }

    private static CoopNativeDispatchResult Effect(string operationId, string effectKind) =>
        new(CoopOutcome.Accepted, new CoopEffectWitness(
            operationId, $"effect:{operationId}", effectKind, 1, 2, SettledDigest)
        {
            AuthorityId = "authority:native-test",
            AuthorityEpoch = "epoch:native-test",
            CheckpointId = "checkpoint:2",
            NativeChecksum = SettledDigest
        }, null);
}

internal enum CaptureMode
{
    ActionSettled,
    ActionRejected,
    ActionUnknown,
    VoteSettled,
    RejoinPending,
    RejoinRecovered
}
