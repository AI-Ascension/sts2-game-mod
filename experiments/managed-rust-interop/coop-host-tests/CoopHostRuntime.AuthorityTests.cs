// SPDX-License-Identifier: MIT

using System;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.CoopHostTests;

internal static partial class Program
{
    private static void AuthorityChangePrunesPendingReceipt()
    {
        FakePort port = new()
        {
            ReturnUnknown = true,
            KeepUnknownPending = true,
            ChangeEpochAfterRejoin = true,
            RejoinSetsConverged = true
        };
        CoopHostRuntime runtime = new(port);
        CoopOperationReceipt pending = runtime.DispatchLocalAction(new(
            "op:authority-prune", 1, "peer:host1", "end_turn", null, null));
        Check(pending.Outcome == CoopOutcome.Unknown && runtime.HasPendingMutation,
            "the old authority starts with a pending native mutation");

        CoopOperationReceipt rejoin = runtime.Rejoin("op:authority-prune-rejoin",
            "peer:host1", 2);
        Check(rejoin.Outcome == CoopOutcome.Accepted && runtime.HasPendingMutation,
            "authority transition keeps rejoin pending until it is reconciled");
        Check(runtime.Reconcile("op:authority-prune-rejoin",
                out CoopOperationReceipt? retiredRejoin)
            && retiredRejoin?.Outcome == CoopOutcome.Rejected
            && retiredRejoin.ErrorCode == "native_authority_changed"
            && !runtime.HasPendingMutation,
            "a new authority retires the old rejoin instead of blocking forever");
        Check(runtime.Reconcile("op:authority-prune", out CoopOperationReceipt? stale)
            && stale?.Outcome == CoopOutcome.Rejected
            && stale.ErrorCode == "native_authority_changed",
            "the old operation remains a fenced rejected replay after authority change");
    }

}
