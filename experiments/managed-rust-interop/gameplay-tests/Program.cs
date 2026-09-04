// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class Program
{
    private static void Main()
    {
        ReadsDiscoverNewGenerations();
        SettledReceiptIsReplayedBeforeAdmission();
        UnrelatedTransitionDoesNotSettle();
        MismatchedCompletionDoesNotSettle();
        QueuedActionRechecksGeneration();
        MalformedNumbersAndDuplicateFieldsAreRejected();
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
        Check(staleStatus == 503 && source.Dispatches == 0, "new mutations retain exact generation checks");
        using JsonDocument catalog = Wire.Call(support, "legal_actions_request", 8, out int catalogStatus);
        Check(catalogStatus == 503, "catalog requests retain exact generation checks");
    }

    private static void SettledReceiptIsReplayedBeforeAdmission()
    {
        var source = new FakeHost { Complete = true };
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, new TestQueue());
        using JsonDocument first = Wire.Call(support, "dispatch_action_request", 1, out int firstStatus);
        Check(firstStatus == 200 && Wire.Status(first) == "settled", "independent completion settles");
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
