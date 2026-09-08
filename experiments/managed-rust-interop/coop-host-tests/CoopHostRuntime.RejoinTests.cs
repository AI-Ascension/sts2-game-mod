// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.CoopHostTests;

internal static partial class Program
{
    private static void RejoinMustRetainOriginalAuthorityEpoch()
    {
        FakePort port = new() { RejoinSetsConverged = true, ChangeEpochAfterRejoin = true };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt first = runtime.Rejoin("op:epoch-change", "peer:host1", 6);
        Check(first.Outcome == CoopOutcome.Accepted,
            "a changed authority epoch cannot immediately prove recovery");
        runtime.Reconcile("op:epoch-change", out CoopOperationReceipt? later);
        Check(later?.Outcome != CoopOutcome.Recovered,
            "reconciliation must not adopt a different authority epoch as the original admission");
    }

    private static void RejoinRecoveryGateAllowsDisconnectedLocalPeer()
    {
        FakePort port = new() { LocalPeerDisconnected = true };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt receipt = runtime.Rejoin(
            "op:rejoin-disconnected", "peer:host1", 7);
        Check(receipt.Outcome == CoopOutcome.Accepted
            && receipt.ErrorCode == "rejoin_recovery_required"
            && port.RejoinCount == 1,
            "rejoin may be admitted for the bound local peer while transport recovery is required");
    }

    private static void RejoinReplayDoesNotRepeatNativeMutation()
    {
        FakePort port = new() { RejoinSetsConverged = true };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt first = runtime.Rejoin("op:rejoin", "peer:host1", 2);
        CoopOperationReceipt replay = runtime.Rejoin("op:rejoin", "peer:host1", 2);
        Check(first.Outcome == CoopOutcome.Recovered
            && replay.Outcome == CoopOutcome.Recovered
            && port.RejoinCount == 1,
            "a recovered rejoin receipt replays without a second native mutation");
    }

    private static void AcceptedRejoinReconcilesWithoutRetrying()
    {
        FakePort port = new() { RejoinStartsDivergent = true };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt first = runtime.Rejoin("op:rejoin-pending", "peer:host1", 3);
        Check(first.Outcome == CoopOutcome.Accepted && runtime.HasPendingMutation,
            "known native rejoin acceptance remains pending until convergence");
        port.EffectPublished = true;
        Check(runtime.Reconcile("op:rejoin-pending", out CoopOperationReceipt? recovered)
            && recovered?.Outcome == CoopOutcome.Recovered
            && port.RejoinCount == 1,
            "accepted rejoin reconciles from a fresh convergence observation");
    }

    private static void AcceptedRejoinWaitsForNativeSettlement()
    {
        FakePort port = new() { RejoinSetsConverged = true, NativeRejoinSettled = false };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt first = runtime.Rejoin(
            "op:rejoin-async", "peer:host1", 8);
        Check(first.Outcome == CoopOutcome.Accepted
            && first.ErrorCode == "rejoin_recovery_pending"
            && runtime.HasPendingMutation
            && port.RejoinCount == 1,
            "a converged pre-settlement observation cannot settle an asynchronous rejoin");
        port.NativeRejoinSettled = true;
        Check(runtime.Reconcile("op:rejoin-async", out CoopOperationReceipt? recovered)
            && recovered?.Outcome == CoopOutcome.Recovered
            && port.RejoinCount == 1,
            "native settlement witness allows the pending rejoin to reconcile");
    }

    private static void UnknownRejoinStaysUnknownWithoutNativeWitness()
    {
        FakePort port = new() { ReturnUnknownRejoin = true, RejoinSetsConverged = true };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt first = runtime.Rejoin("op:rejoin-unknown", "peer:host1", 4);
        Check(first.Outcome == CoopOutcome.Unknown && runtime.HasPendingMutation,
            "unknown native rejoin outcome remains pending");
        Check(runtime.Reconcile("op:rejoin-unknown", out CoopOperationReceipt? stillUnknown)
            && stillUnknown?.Outcome == CoopOutcome.Unknown
            && port.RejoinCount == 1,
            "convergence alone cannot settle an unknown rejoin outcome");
        CoopOperationReceipt replay = runtime.Rejoin("op:rejoin-unknown", "peer:host1", 4);
        Check(replay.Outcome == CoopOutcome.Unknown && port.RejoinCount == 1,
            "an unknown rejoin replay never retries the native call");
    }

    private static void RejectedRejoinIsRemembered()
    {
        FakePort port = new() { ReturnRejectedRejoin = true };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt first = runtime.Rejoin("op:rejoin-rejected", "peer:host1", 5);
        CoopOperationReceipt replay = runtime.Rejoin("op:rejoin-rejected", "peer:host1", 5);
        Check(first.Outcome == CoopOutcome.Rejected
            && replay.Outcome == CoopOutcome.Rejected
            && port.RejoinCount == 1,
            "a rejected rejoin receipt prevents a duplicate native call");
    }

    private sealed class FakePort : ICoopNativeHostPort
    {
        private readonly Dictionary<string, CoopNativePeerBinding> _bindings = new(StringComparer.Ordinal)
        {
            ["peer:host1"] = new("peer:host1", 101, true),
            ["peer:client1"] = new("peer:client1", 202, true)
        };

        internal int DispatchCount { get; private set; }
        internal bool ReturnUnknown { get; init; }
        internal bool KeepUnknownPending { get; init; }
        internal bool ReturnUnknownRejoin { get; init; }
        internal bool ReturnRejectedRejoin { get; init; }
        internal bool RejoinSetsConverged { get; init; }
        internal bool RejoinStartsDivergent { get; init; }
        internal bool ChangeEpochAfterRejoin { get; init; }
        internal bool LocalPeerDisconnected { get; init; }
        internal bool DisconnectClientBeforeEffect { get; set; }
        internal bool NativeRejoinSettled { get; set; } = true;
        internal bool DigestKnown { get; init; } = true;
        internal bool AuthorityIdsMatch { get; init; } = true;
        internal bool EffectPublished { get; set; }
        internal int ConfirmedOperationCount { get; private set; }
        internal int RejoinCount { get; private set; }
        private string _operationId = "op:none";

        public CoopHostObservation Observe()
        {
            bool clientConnected = !DisconnectClientBeforeEffect || !EffectPublished;
            return new CoopHostObservation(
                "session:one", "peer:host1", CoopHostRole.Host, "lan", "lobby:one",
                EffectPublished ? 2UL : 1UL, Digest,
                new[]
                {
                    new CoopPeerSnapshot("peer:host1", true, !LocalPeerDisconnected,
                        EffectPublished ? 2UL : 1UL, Digest),
                    new CoopPeerSnapshot("peer:client1", false, clientConnected,
                        EffectPublished ? 1UL : 0UL,
                        RejoinStartsDivergent && !EffectPublished ? DivergentDigest : Digest)
                    {
                        AuthorityId = AuthorityIdsMatch ? "authority:test" : "authority:other"
                    }
                },
                LocalPeerDisconnected, null)
            {
                AuthorityId = "authority:test",
                AuthorityEpoch = ChangeEpochAfterRejoin && RejoinCount > 0 ? "epoch:other" : "epoch:test",
                HostDigestKnown = DigestKnown
            };
        }

        public bool TryResolvePeer(string opaquePeerId, out CoopNativePeerBinding binding) =>
            _bindings.TryGetValue(opaquePeerId, out binding!);

        public CoopNativeDispatchResult DispatchLocalAction(CoopLocalActionRequest request)
        {
            DispatchCount++;
            _operationId = request.OperationId;
            if (ReturnUnknown)
            {
                if (!KeepUnknownPending)
                    EffectPublished = true;
                return CoopNativeDispatchResult.Unknown("native_dispatch_outcome_unknown");
            }
            EffectPublished = true;
            return new(CoopOutcome.Accepted, Effect(), null);
        }

        public CoopNativeDispatchResult SubmitSharedVote(CoopSharedVoteRequest request) =>
            DispatchLocalAction(new(request.OperationId, request.ExpectedHostGeneration,
                request.VoterPeerId, "shared_vote", request.Choice, null));

        public void ConfirmSettlement(string operationId) => ConfirmedOperationCount++;

        public CoopNativeDispatchResult Rejoin(string opaquePeerId, ulong rejoinEpoch)
        {
            RejoinCount++;
            if (!_bindings.ContainsKey(opaquePeerId))
                return CoopNativeDispatchResult.Rejected("unknown_peer");
            if (ReturnRejectedRejoin)
                return CoopNativeDispatchResult.Rejected("native_rejoin_rejected");
            if (RejoinSetsConverged)
                EffectPublished = true;
            return ReturnUnknownRejoin
                ? CoopNativeDispatchResult.Unknown("native_rejoin_outcome_unknown")
                : CoopNativeDispatchResult.Accepted();
        }

        public CoopEffectWitness? Reconcile(string operationId) => EffectPublished ? Effect() : null;

        public bool IsRejoinSettled() => NativeRejoinSettled;

        private CoopEffectWitness Effect() =>
            new(_operationId, "effect:1", "turn_ended", 1, 2, Digest);
    }

    private const string DivergentDigest =
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
}
