// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.CoopHostTests;

internal static partial class Program
{
    private const string Digest = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    private static void Main()
    {
        PeerGenerationsMayDifferWhenDigestsConverge();
        StaleHostGenerationIsRejectedBeforeNativeDispatch();
        UnknownOutcomeReconcilesWithoutRetryingMutation();
        PendingCoopMutationBlocksNewOperation();
        RejoinReplayDoesNotRepeatNativeMutation();
        AcceptedRejoinReconcilesWithoutRetrying();
        AcceptedRejoinWaitsForNativeSettlement();
        RejoinAttemptWindowResetsAfterLateReconnectWithoutExtension();
        NativeHeartbeatLivenessIsBoundedAndUnknownSafe();
        UnknownRejoinStaysUnknownWithoutNativeWitness();
        RejectedRejoinIsRemembered();
        RejoinMustRetainOriginalAuthorityEpoch();
        RejoinRecoveryGateAllowsDisconnectedLocalPeer();
        UnboundOpaqueIdentityIsRejected();
        DisconnectedPeerBlocksSettlement();
        MismatchedAuthorityIdentityBlocksSettlement();
        UnknownDigestBlocksDispatch();
        ExternalAdmissionSerializesMutations();
        NativeArgumentEncodingFailsClosed();
        NativeCardEncodingFailsClosed();
        Console.WriteLine("Native co-op host fencing and per-peer generation checks passed.");
    }

    private static void PeerGenerationsMayDifferWhenDigestsConverge()
    {
        FakePort port = new();
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt receipt = runtime.DispatchLocalAction(new(
            "op:1", 1, "peer:host1", "end_turn", null, null));
        Check(receipt.Outcome == CoopOutcome.Settled,
            "host action settles with a native effect witness");
        Check(receipt.Observation.Peers[1].Generation != receipt.Observation.HostGeneration,
            "test fixture must retain a distinct client-local generation");
        Check(port.DispatchCount == 1, "settlement dispatches exactly once");
    }

    private static void StaleHostGenerationIsRejectedBeforeNativeDispatch()
    {
        FakePort port = new();
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt receipt = runtime.DispatchLocalAction(new(
            "op:stale", 0, "peer:host1", "end_turn", null, null));
        Check(receipt.Outcome == CoopOutcome.Rejected
            && receipt.ErrorCode == "stale_host_generation"
            && port.DispatchCount == 0, "stale host generation cannot reach native dispatch");
    }

    private static void UnknownOutcomeReconcilesWithoutRetryingMutation()
    {
        FakePort port = new() { ReturnUnknown = true };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt first = runtime.DispatchLocalAction(new(
            "op:unknown", 1, "peer:host1", "end_turn", null, null));
        Check(first.Outcome == CoopOutcome.Unknown && port.DispatchCount == 1,
            "native exception remains unknown after admission");
        Check(runtime.HasPendingMutation, "unknown native outcome remains a pending mutation");
        Check(runtime.Reconcile("op:unknown", out CoopOperationReceipt? recovered)
            && recovered?.Outcome == CoopOutcome.Settled
            && port.DispatchCount == 1, "reconcile settles without a blind retry");
        Check(!runtime.HasPendingMutation, "settled native outcome clears the pending predicate");
    }

    private static void PendingCoopMutationBlocksNewOperation()
    {
        FakePort port = new() { ReturnUnknown = true, KeepUnknownPending = true };
        CoopHostRuntime runtime = new(port);
        CoopLocalActionRequest request = new(
            "op:pending", 1, "peer:host1", "end_turn", null, null);
        CoopOperationReceipt first = runtime.DispatchLocalAction(request);
        Check(first.Outcome == CoopOutcome.Unknown && port.DispatchCount == 1,
            "an unknown co-op action remains the sole pending native mutation");
        CoopOperationReceipt blocked = runtime.DispatchLocalAction(new(
            "op:pending-new", 1, "peer:host1", "end_turn", null, null));
        Check(blocked.Outcome == CoopOutcome.Rejected
            && blocked.ErrorCode == "operation_in_progress"
            && port.DispatchCount == 1,
            "a new co-op operation cannot pass its own pending mutation fence");
        CoopOperationReceipt replay = runtime.DispatchLocalAction(request);
        Check(replay.Outcome == CoopOutcome.Unknown && port.DispatchCount == 1,
            "the pending operation's exact duplicate still replays without dispatch");
    }

    private static void UnboundOpaqueIdentityIsRejected()
    {
        FakePort port = new();
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt receipt = runtime.DispatchLocalAction(new(
            "op:identity", 1, "peer:unknown", "end_turn", null, null));
        Check(receipt.Outcome == CoopOutcome.Rejected
            && receipt.ErrorCode == "unknown_or_stale_peer_identity"
            && port.DispatchCount == 0, "caller cannot invent a peer identity");
    }

    private static void DisconnectedPeerBlocksSettlement()
    {
        FakePort port = new() { DisconnectClientBeforeEffect = true };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt receipt = runtime.DispatchLocalAction(new(
            "op:disconnect", 1, "peer:host1", "end_turn", null, null));
        Check(receipt.Outcome == CoopOutcome.Unknown,
            "a disconnected peer prevents a settled effect witness");
    }

    private static void UnknownDigestBlocksDispatch()
    {
        FakePort port = new() { DigestKnown = false };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt receipt = runtime.DispatchLocalAction(new(
            "op:digest", 1, "peer:host1", "end_turn", null, null));
        Check(receipt.Outcome == CoopOutcome.Rejected
            && receipt.ErrorCode == "native_state_not_settled"
            && port.DispatchCount == 0,
            "a partial native digest cannot admit mutation");
    }

    private static void ExternalAdmissionSerializesMutations()
    {
        FakePort port = new();
        bool otherProfileIdle = true;
        CoopHostRuntime runtime = new(port, () => otherProfileIdle);
        CoopLocalActionRequest request = new(
            "op:gate", 1, "peer:host1", "end_turn", null, null);

        CoopOperationReceipt first = runtime.DispatchLocalAction(request);
        Check(first.Outcome == CoopOutcome.Settled && port.DispatchCount == 1,
            "an idle external profile admits the first co-op mutation");

        // Duplicate replay is intentionally checked before the cross-profile gate. A caller
        // polling a settled co-op receipt must be able to observe it while another profile is
        // active, without dispatching the native mutation a second time.
        otherProfileIdle = false;
        CoopOperationReceipt replay = runtime.DispatchLocalAction(request);
        Check(replay.Outcome == CoopOutcome.Settled && port.DispatchCount == 1,
            "co-op duplicate replay bypasses a gate for a new mutation");

        CoopOperationReceipt blocked = runtime.DispatchLocalAction(new(
            "op:gate-new", 2, "peer:host1", "end_turn", null, null));
        Check(blocked.Outcome == CoopOutcome.Rejected
            && blocked.ErrorCode == "operation_in_progress"
            && port.DispatchCount == 1
            && !runtime.HasPendingMutation,
            "a cross-profile pending mutation blocks native co-op dispatch");

        otherProfileIdle = true;
        CoopOperationReceipt admitted = runtime.DispatchLocalAction(new(
            "op:gate-new", 2, "peer:host1", "end_turn", null, null));
        Check(admitted.Outcome is CoopOutcome.Accepted or CoopOutcome.Unknown
            && port.DispatchCount == 2,
            "co-op dispatch resumes after the external profile settles");
    }

    private static void MismatchedAuthorityIdentityBlocksSettlement()
    {
        FakePort port = new() { AuthorityIdsMatch = false };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt receipt = runtime.DispatchLocalAction(new(
            "op:authority", 1, "peer:host1", "end_turn", null, null));
        Check(receipt.Outcome == CoopOutcome.Unknown,
            "a peer from another canonical native authority cannot settle an operation");
    }

    private static void NativeArgumentEncodingFailsClosed()
    {
        Check(NativeActionEncoding.TryTurnNumber("end_turn", 3, out int implicitTurn, out _)
            && implicitTurn == 3, "end_turn binds to the observed native turn");
        Check(NativeActionEncoding.TryTurnNumber("turn:3", 3, out int explicitTurn, out _)
            && explicitTurn == 3, "explicit turn encoding is accepted");
        Check(!NativeActionEncoding.TryTurnNumber("turn:2", 3, out _, out _),
            "stale turn encoding is rejected");
        Check(!NativeActionEncoding.TryTurnNumber("approve", 3, out _, out _),
            "generic action text cannot select a native action");

        CoopSharedVoteRequest eventRequest = new(
            "op:event", "event", 1, "peer:host1", CoopVoteDomain.SharedEvent, "index:2");
        Check(NativeVoteEncoding.TryParse(eventRequest, out NativeVoteEncoding eventVote, out _)
            && eventVote.Domain == CoopVoteDomain.SharedEvent && eventVote.Index == 2,
            "event vote maps to a typed native option index");

        CoopSharedVoteRequest relicRequest = new(
            "op:relic", "relic", 1, "peer:host1", CoopVoteDomain.TreasureRelic, "skip");
        Check(NativeVoteEncoding.TryParse(relicRequest, out NativeVoteEncoding relicVote, out _)
            && relicVote.SkipRelic, "relic skip maps to the native skip synchronizer");

        CoopSharedVoteRequest ambiguousRequest = new(
            "op:ambiguous", "event", 1, "peer:host1", CoopVoteDomain.SharedEvent, "approve");
        Check(!NativeVoteEncoding.TryParse(ambiguousRequest, out _, out _),
            "generic approve text cannot select a native event option");
    }

    private static void NativeCardEncodingFailsClosed()
    {
        Check(NativeCardEncoding.TryParse("card:7", out NativeCardEncoding card, out _)
            && card.CardId == 7 && card.TargetId is null,
            "native card identity parses without a target");
        Check(NativeCardEncoding.TryParse("card:7:target:19", out NativeCardEncoding targeted, out _)
            && targeted.CardId == 7 && targeted.TargetId == 19,
            "native card identity parses an explicit creature target");
        Check(!NativeCardEncoding.TryParse("card:0", out _, out _),
            "zero native card identity is rejected");
        Check(!NativeCardEncoding.TryParse("card:7:target:0", out _, out _),
            "zero native target identity is rejected");
        Check(!NativeCardEncoding.TryParse("play:7", out _, out _),
            "generic card action text cannot select a native card");
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }

}
