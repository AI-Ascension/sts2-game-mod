// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using AiAscension.Sts2GameMod.Runtime;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.GameActions.Multiplayer;

namespace AiAscension.Sts2GameMod.CoopCheckpointTests;

internal static class Program
{
    private static void Main()
    {
        ExactRemoteCheckpointMatchIsRequired();
        MismatchedRemoteCheckpointCannotSettle();
        MissingRemoteCheckpointCannotSettle();
        Console.WriteLine("Native co-op checkpoint correlation tests passed.");
    }

    private static void ExactRemoteCheckpointMatchIsRequired()
    {
        TestAction action = new();
        NativePendingOperation pending = NativePendingOperation.ForAction(
            "op:checkpoint", "end_turn", 3, 7, action, null!, "epoch:test",
            new ulong[] { 101, 202 });
        NetChecksumData local = new() { id = 19, checksum = 0x1234u };
        Check(pending.TryRecordPassiveChecksum(
            8, "finished action execution test-action", local),
            "the exact native action callback binds the local checkpoint");
        pending.RecordRemoteChecksum(202, local);
        Check(pending.HasMatchingRemoteChecksums(new ulong[] { 101 }, 101) == false,
            "the original participant set is retained when a peer is missing");
        Check(pending.HasMatchingRemoteChecksums(new ulong[] { 101, 202 }, 101),
            "all admitted remote peers must match the local ID and checksum");
    }

    private static void MismatchedRemoteCheckpointCannotSettle()
    {
        TestAction action = new();
        NativePendingOperation pending = NativePendingOperation.ForAction(
            "op:mismatch", "end_turn", 3, 7, action, null!, "epoch:test",
            new ulong[] { 101, 202 });
        NetChecksumData local = new() { id = 19, checksum = 0x1234u };
        pending.TryRecordPassiveChecksum(8, "finished action execution test-action", local);
        pending.RecordRemoteChecksum(202, new NetChecksumData { id = 19, checksum = 0x9999u });
        Check(!pending.HasMatchingRemoteChecksums(new ulong[] { 202 }, 101),
            "a same-ID checksum with a different value is divergence, not settlement");
    }

    private static void MissingRemoteCheckpointCannotSettle()
    {
        TestAction action = new();
        NativePendingOperation pending = NativePendingOperation.ForAction(
            "op:missing", "end_turn", 3, 7, action, null!, "epoch:test",
            new ulong[] { 101, 202 });
        pending.TryRecordPassiveChecksum(8, "finished action execution test-action",
            new NetChecksumData { id = 19, checksum = 0x1234u });
        Check(!pending.HasMatchingRemoteChecksums(new ulong[] { 202 }, 101),
            "a local checkpoint alone cannot settle a multiplayer operation");
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }

    private sealed class TestAction : GameAction
    {
        public override ulong OwnerId => 101;
        public override GameActionType ActionType => default;
        protected override Task ExecuteAction() => Task.CompletedTask;
        public override INetAction ToNetAction() => null!;
        public override string ToString() => "test-action";
    }
}
