// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class Program
{
    private static void Main(string[] args)
    {
        if (args.Length == 1 && args[0] == "--emit-contract-frames")
        {
            ContractFrames.Emit();
            return;
        }
        ReadsDiscoverNewGenerations();
        UnavailableOperationsKeepResponseKind();
        SettledReceiptIsReplayedBeforeAdmission();
        UnrelatedTransitionDoesNotSettle();
        MismatchedCompletionDoesNotSettle();
        QueuedActionRechecksGeneration();
        MalformedNumbersAndDuplicateFieldsAreRejected();
        RecoveryReconcilesScopedReceipts();
        TextBoundsUseUtf8Bytes();
        HelpersCompileAndRejectInvalidCoop();
        Console.WriteLine("Runtime-v3 managed request, receipt, and settlement checks passed.");
    }

    private static void ReadsDiscoverNewGenerations()
    {
        var source = new FakeHost { Generation = 8 };
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, new TestQueue());
        foreach (string kind in new[] { "state_request", "reobserve_request", "recover_request" })
        {
            using JsonDocument response = Wire.Call(support, kind, 0, out int status);
            Check(status == 200 && response.RootElement.GetProperty("generation").GetUInt64() == 8,
                "reads must discover the authoritative generation");
        }
        source.Generation = 9;
        using JsonDocument refreshed = Wire.Call(support, "reobserve_request", 8, out int refreshedStatus);
        Check(refreshedStatus == 200 && refreshed.RootElement.GetProperty("generation").GetUInt64() == 9,
            "refresh after unsolicited transition");
        using JsonDocument stale = Wire.Call(support, "dispatch_action_request", 8, out int staleStatus);
        Check(staleStatus == 409 && source.Dispatches == 0
            && stale.RootElement.GetProperty("kind").GetString() == "dispatch_action_response"
            && Wire.Status(stale) == "rejected" && Wire.Error(stale) == "stale_generation"
            && stale.RootElement.GetProperty("generation").GetUInt64() == 9,
            "stale mutations return the canonical rejection and current host observation");
        using JsonDocument catalog = Wire.Call(support, "legal_actions_request", 8, out int catalogStatus);
        Check(catalogStatus == 409 && Wire.Error(catalog) == "stale_generation"
            && !catalog.RootElement.TryGetProperty("kind", out _)
            && catalog.RootElement.GetProperty("recovery").GetString() == "reobserve",
            "stale catalogs use explicit owner-local HTTP failure without masquerading as an observation");
    }

    private static void UnavailableOperationsKeepResponseKind()
    {
        var source = new FakeHost { ThrowReads = true };
        foreach (RuntimeV3GameplaySupport support in new[] { RuntimeV3GameplaySupport.Unconfigured(),
            RuntimeV3GameplaySupport.WithHost(source, new TestQueue()) })
        {
            using JsonDocument dispatch = Wire.Call(support, "dispatch_action_request", 1, out int dispatchStatus);
            Check(dispatchStatus == 503 && Wire.Status(dispatch) == "unknown"
                && dispatch.RootElement.GetProperty("kind").GetString() == "dispatch_action_response"
                && dispatch.RootElement.GetProperty("operation_id").GetString() == "operation-1",
                "unavailable dispatch retains canonical kind and requested operation identity");
            using JsonDocument catalog = Wire.Call(support, "legal_actions_request", 1, out int catalogStatus);
            Check(catalogStatus == 503 && !catalog.RootElement.TryGetProperty("kind", out _)
                && catalog.RootElement.GetProperty("recovery").GetString() == "reobserve",
                "unavailable catalog returns owner-local HTTP failure");
            using JsonDocument recovery = Wire.Call(support, "recover_request", 1, out int recoveryStatus);
            Check(recoveryStatus == 503 && Wire.Status(recovery) == "unknown"
                && recovery.RootElement.GetProperty("kind").GetString() == "recover_response",
                "unavailable recovery retains canonical recovery response kind");
        }
    }

    private static void SettledReceiptIsReplayedBeforeAdmission()
    {
        var source = new FakeHost { Complete = true };
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, new TestQueue());
        using JsonDocument first = Wire.Call(support, "dispatch_action_request", 1, out int firstStatus);
        Check(firstStatus == 200 && Wire.Status(first) == "settled", "independent completion settles");
        source.Hand.Clear();
        source.ThrowReads = true;
        using JsonDocument replay = Wire.Call(support, "dispatch_action_request", 1, out int replayStatus);
        Check(replayStatus == 200 && replay.RootElement.GetRawText() == first.RootElement.GetRawText()
            && source.Dispatches == 1, "replay preserves receipt without reading changed host state");
        using JsonDocument conflict = Wire.Call(support, "dispatch_action_request", 1, out int conflictStatus,
            stateId: "another-state");
        Check(conflictStatus == 409 && Wire.Error(conflict) == "idempotency_conflict", "state is part of payload identity");
        using JsonDocument changedGeneration = Wire.Call(support, "dispatch_action_request", 2, out int changedStatus);
        Check(changedStatus == 409 && Wire.Error(changedGeneration) == "idempotency_conflict", "generation is part of payload identity");
        using JsonDocument changedAction = Wire.Call(support, "dispatch_action_request", 1, out int actionStatus,
            actionKind: "rest");
        Check(actionStatus == 409 && Wire.Error(changedAction) == "idempotency_conflict", "action is part of payload identity");
        foreach ((string session, ulong epoch) in new[] { ("other-session", 1UL), ("session-1", 2UL) })
        {
            using JsonDocument foreign = Wire.Call(support, "wait_request", 1, out int foreignStatus,
                session: session, epoch: epoch);
            Check(foreignStatus == 503 && Wire.Error(foreign) == "operation_not_found", "receipt isolation");
        }
        Check(source.Dispatches == 1, "no conflicting replay dispatches");
    }

    private static void UnrelatedTransitionDoesNotSettle()
    {
        var source = new FakeHost();
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, new TestQueue());
        using JsonDocument response = Wire.Call(support, "dispatch_action_request", 1, out int status);
        Check(status == 503 && Wire.Status(response) == "unknown"
            && source.Generation == 2, "generation advance without completion stays unknown");
        using JsonDocument replay = Wire.Call(support, "dispatch_action_request", 1, out _);
        Check(Wire.Status(replay) == "unknown" && source.Dispatches == 1, "unknown retry does not mutate");
        source.Complete = true;
        using JsonDocument settled = Wire.Call(support, "wait_request", 1, out int settledStatus);
        Check(settledStatus == 200 && Wire.Status(settled) == "settled" && source.Dispatches == 1,
            "read-only wait can establish delayed completion");
        Check(settled.RootElement.GetProperty("wait_outcome").GetString() == "same_state_mutation",
            "same-state completion must not be labeled a successor");
    }

    private static void MismatchedCompletionDoesNotSettle()
    {
        foreach (bool wrongAction in new[] { false, true })
        {
            var source = new FakeHost { Complete = true, WrongOperation = !wrongAction, WrongAction = wrongAction };
            RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, new TestQueue());
            using JsonDocument response = Wire.Call(support, "dispatch_action_request", 1, out int status);
            Check(status == 503 && Wire.Status(response) == "unknown", "foreign completion must not settle");
        }
    }

    private static void QueuedActionRechecksGeneration()
    {
        var source = new FakeHost { Complete = true };
        var queue = new TestQueue { Deferred = true };
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, queue);
        using JsonDocument response = Wire.Call(support, "dispatch_action_request", 1, out int status);
        Check(status == 200 && Wire.Status(response) == "accepted", "queue admission is not settlement");
        using JsonDocument retry = Wire.Call(support, "dispatch_action_request", 1, out _);
        Check(Wire.Status(retry) == "accepted" && queue.PendingCount == 1, "queued replay does not enqueue again");
        source.Generation = 2;
        queue.Run();
        using JsonDocument rejected = Wire.Call(support, "dispatch_action_request", 1, out int rejectedStatus);
        Check(rejectedStatus == 409 && Wire.Error(rejected) == "stale_generation" && source.Dispatches == 0,
            "queued mutation rechecks the host generation");
    }

    private static void Check(bool passed, string message)
    {
        if (!passed) { throw new InvalidOperationException(message); }
    }

    private static void RecoveryReconcilesScopedReceipts()
    {
        var source = new FakeHost();
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, new TestQueue());
        using JsonDocument first = Wire.Call(support, "dispatch_action_request", 1, out _);
        source.Complete = true;
        using JsonDocument recovered = Wire.Call(support, "recover_request", 1, out int status,
            recoveryKind: "reconcile");
        Check(status == 200 && Wire.Status(recovered) == "settled" && source.Dispatches == 1,
            "reconcile uses independent completion without dispatching again");
        using JsonDocument foreign = Wire.Call(support, "recover_request", 1, out int foreignStatus,
            session: "other-session", recoveryKind: "reconcile");
        Check(foreignStatus == 503 && Wire.Error(foreign) == "operation_not_found", "reconcile isolation");

        var queue = new TestQueue { Deferred = true };
        var staleSource = new FakeHost();
        RuntimeV3GameplaySupport stale = RuntimeV3GameplaySupport.WithHost(staleSource, queue);
        using JsonDocument accepted = Wire.Call(stale, "dispatch_action_request", 1, out _);
        staleSource.Generation = 2;
        queue.Run();
        using JsonDocument waited = Wire.Call(stale, "wait_request", 1, out _, recoveryKind: "reconcile");
        Check(waited.RootElement.GetProperty("wait_outcome").GetString() == "recovery_required",
            "terminal rejection directs caller to reconciliation rather than another wait");
        using JsonDocument rejected = Wire.Call(stale, "recover_request", 1, out int rejectedStatus,
            recoveryKind: "reconcile");
        Check(rejectedStatus == 409 && Wire.Status(rejected) == "rejected"
            && Wire.Error(rejected) == "stale_generation" && staleSource.Dispatches == 0,
            "reconcile exposes terminal rejection without mutation");
    }

    private static void TextBoundsUseUtf8Bytes()
    {
        Check(RuntimeV3GameplayContract.IsText(new string('é', 256)), "512 UTF8 bytes accepted");
        Check(!RuntimeV3GameplayContract.IsText(new string('é', 257)), "514 UTF8 bytes rejected");
        Check(!RuntimeV3GameplayContract.IsText("name\u0085"), "Unicode controls rejected");
    }

    private static void HelpersCompileAndRejectInvalidCoop()
    {
        var players = new List<CoopPeer> { new("local", CoopPeerRole.Local), new("ally", CoopPeerRole.Ally) };
        var missing = new List<string>();
        var synchronization = new CoopSynchronization(CoopSyncStatus.Synchronized, 1, 2, missing);
        Check(CoopProjection.TryCreate("combat", 1, players, synchronization, out CoopProjection? projection, out _)
            && projection is not null && projection.MutationAllowed, "valid co-op helper projection");
        players.Clear();
        missing.Add("ally");
        Check(projection!.Players.Count == 2 && projection.Synchronization.MissingPeers.Count == 0,
            "co-op helper snapshot does not retain mutable caller collections");
        Check(!CoopProjection.TryCreate("combat", 1,
            new[] { new CoopPeer("local", CoopPeerRole.Local), new CoopPeer("ally", (CoopPeerRole)99) },
            synchronization with { MissingPeers = Array.Empty<string>() }, out _, out _), "invalid peer role rejected");
        Check(!(synchronization with { Status = (CoopSyncStatus)99 }).Validate(out _), "invalid sync enum rejected");
        var source = new FakeHost { Complete = true };
        var patch = new LlmCombatPatch(new RuntimeV3GameplayHost(source, new TestQueue()));
        var operation = new RuntimeV3OperationKey("instance-1", "session-1", "lease-1", 1, "operation-1");
        Check(!patch.TryDispatchCurrent(operation, source.Observe(), "missing", out _), "missing semantic action rejected");
        Check(patch.TryDispatchCurrent(operation, source.Observe(), "combat.end-turn", out RuntimeV3DispatchReceipt? receipt)
            && receipt?.Status == RuntimeV3DispatchStatus.Settled, "combat helper uses scoped operation identity");
    }

    private static void MalformedNumbersAndDuplicateFieldsAreRejected()
    {
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(new FakeHost(), new TestQueue());
        foreach (string field in new[] { "generation", "lease_epoch", "wait_for_millis" })
        {
            using JsonDocument response = Wire.Call(support, "wait_request", 1, out int status,
                transform: body => body.Replace($"\"{field}\":1", $"\"{field}\":\"bad\"", StringComparison.Ordinal));
            Check(status == 400, "wrong numeric types fail closed without throwing");
        }
        using JsonDocument duplicate = Wire.Call(support, "state_request", 1, out int duplicateStatus,
            transform: body => body.Replace("\"generation\":1", "\"generation\":1,\"generation\":1", StringComparison.Ordinal));
        Check(duplicateStatus == 400, "duplicate envelope fields are rejected");
    }
}
