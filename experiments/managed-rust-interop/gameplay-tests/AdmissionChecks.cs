// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class AdmissionChecks
{
    internal static void Run()
    {
        OperationAwareAdmissionAllowsOwnSettlement();
        PendingReceiptStillFencesOverlappingOperation();
    }

    private static void OperationAwareAdmissionAllowsOwnSettlement()
    {
        var source = new FakeHost { Complete = true };
        RuntimeV3GameplaySupport? support = null;
        support = RuntimeV3GameplaySupport.WithHost(source, new TestQueue(),
            operation => !support!.HasPendingMutationExcept(operation));
        using JsonDocument response = Wire.Call(support, "dispatch_action_request", 1, out int status);
        Check(status == 200 && Wire.Status(response) == "settled" && source.Dispatches == 1,
            "shared operation-aware admission must ignore the receipt currently settling");
    }

    private static void PendingReceiptStillFencesOverlappingOperation()
    {
        var source = new FakeHost { AdvanceGenerationOnDispatch = false };
        var queue = new TestQueue { Deferred = true };
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, queue,
            _ => true);
        using JsonDocument first = Wire.Call(support, "dispatch_action_request", 1, out int firstStatus);
        queue.RunNext();
        using JsonDocument second = Wire.Call(support, "dispatch_action_request", 1, out int secondStatus,
            operationId: "operation-2");
        Check(firstStatus == 200 && secondStatus == 200
            && Wire.Status(first) == "accepted" && Wire.Status(second) == "accepted"
            && queue.PendingCount == 1, "distinct queued operations reserve separate receipts");
        queue.RunNext();
        using JsonDocument rejected = Wire.Call(support, "dispatch_action_request", 1,
            out int rejectedStatus, operationId: "operation-2");
        Check(rejectedStatus == 409 && Wire.Status(rejected) == "rejected"
            && Wire.Error(rejected) == "operation_in_progress" && source.Dispatches == 1,
            "host-local pending receipt fence rejects overlapping operations");
    }

    private static void Check(bool passed, string message)
    {
        if (!passed) { throw new InvalidOperationException(message); }
    }
}
