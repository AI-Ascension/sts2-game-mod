// SPDX-License-Identifier: MIT

using System;
using System.Collections;
using System.Collections.Generic;
using System.Reflection;
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
        AdmittedRosterSnapshotIsImmutable();
        MismatchedRemoteCheckpointCannotSettle();
        MissingRemoteCheckpointCannotSettle();
        AuthorityEpochResetDropsNativeCheckpointLineage();
        Console.WriteLine("Native co-op checkpoint correlation tests passed.");
    }

    private static void ExactRemoteCheckpointMatchIsRequired()
    {
        TestAction action = new();
        NativePendingOperation pending = NativePendingOperation.ForAction(
            "op:checkpoint", "end_turn", 3, 7, action, null!, "epoch:test",
            new ulong[] { 101, 202, 303 });
        NetChecksumData local = new() { id = 19, checksum = 0x1234u };
        Check(pending.TryRecordPassiveChecksum(
            8, "finished action execution test-action", local, StateFor(action)),
            "the exact native action callback binds the local checkpoint");
        pending.RecordRemoteChecksum(202, local);
        Check(!pending.HasMatchingRemoteChecksums(new ulong[] { 101, 202 }, 101),
            "the original participant set is retained when a peer is missing");
        Check(!pending.HasMatchingRemoteChecksums(new ulong[] { 101, 202, 303 }, 101),
            "a current peer outside the admitted roster cannot complete the quorum");
        pending.RecordRemoteChecksum(303, local);
        Check(pending.HasMatchingRemoteChecksums(new ulong[] { 101, 202, 303 }, 101),
            "all admitted remote peers must match the local ID and checksum");
    }

    private static void AdmittedRosterSnapshotIsImmutable()
    {
        TestAction action = new();
        ulong[] admitted = { 101, 202, 303 };
        NativePendingOperation pending = NativePendingOperation.ForAction(
            "op:roster-copy", "end_turn", 3, 7, action, null!, "epoch:test", admitted);
        admitted[1] = 404;
        Check(pending.ParticipantNativeIds.Count == 3
            && pending.ParticipantNativeIds[0] == 101
            && pending.ParticipantNativeIds[1] == 202
            && pending.ParticipantNativeIds[2] == 303,
            "the admitted roster is copied at operation admission");
    }

    private static void MismatchedRemoteCheckpointCannotSettle()
    {
        TestAction action = new();
        NativePendingOperation pending = NativePendingOperation.ForAction(
            "op:mismatch", "end_turn", 3, 7, action, null!, "epoch:test",
            new ulong[] { 101, 202 });
        NetChecksumData local = new() { id = 19, checksum = 0x1234u };
        pending.TryRecordPassiveChecksum(8, "finished action execution test-action", local,
            StateFor(action));
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
            new NetChecksumData { id = 19, checksum = 0x1234u }, StateFor(action));
        Check(!pending.HasMatchingRemoteChecksums(new ulong[] { 202 }, 101),
            "a local checkpoint alone cannot settle a multiplayer operation");
    }

    private static void AuthorityEpochResetDropsNativeCheckpointLineage()
    {
        InstalledNativeCoopHostPort port = new();
        MethodInfo update = typeof(InstalledNativeCoopHostPort).GetMethod(
            "UpdateAuthorityEpoch", BindingFlags.Instance | BindingFlags.NonPublic)!;
        string firstEpoch = (string)update.Invoke(port, new object?[] { "lobby:a", 101UL, true })!;

        SetField(port, "_lastHostFingerprint", "stale-fingerprint");
        SetField(port, "_hostSequence", 41UL);
        SetField(port, "_nativeChecksumDigest", "stale-digest");
        SetField(port, "_nativeChecksumData", new NetChecksumData { id = 19, checksum = 0x1234u });
        SetField(port, "_nativeChecksumOrdinal", 9UL);
        SetField(port, "_nativeStateDiverged", true);
        SetField(port, "_remoteChecksums", new Dictionary<ulong, NetChecksumData>
        {
            [202] = new NetChecksumData { id = 19, checksum = 0x1234u }
        });
        var pending = NativePendingOperation.ForAction(
            "op:stale", "end_turn", 41, 9, new TestAction(), null!, firstEpoch,
            new ulong[] { 101, 202 });
        SetField(port, "_pending", new Dictionary<string, NativePendingOperation>
        {
            [pending.OperationId] = pending
        });

        string secondEpoch = (string)update.Invoke(port, new object?[] { "lobby:b", 101UL, true })!;
        Check(secondEpoch != firstEpoch, "a new native lobby receives a new authority epoch");
        Check(Field<string?>(port, "_lastHostFingerprint") is null
            && Field<ulong>(port, "_hostSequence") == 0,
            "a new authority starts a fresh host sequence");
        Check(Field<string?>(port, "_nativeChecksumDigest") is null
            && Field<ulong>(port, "_nativeChecksumOrdinal") == 0
            && !Field<bool>(port, "_nativeStateDiverged")
            && Field<NetChecksumData?>(port, "_nativeChecksumData") is null,
            "a new authority cannot reuse the prior native checkpoint lineage");
        Check(((IDictionary)Field<object>(port, "_remoteChecksums")).Count == 0
            && ((IDictionary)Field<object>(port, "_pending")).Count == 0,
            "a new authority cannot reconcile prior remote checksums or pending actions");
    }

    private static T Field<T>(InstalledNativeCoopHostPort port, string name)
    {
        FieldInfo field = typeof(InstalledNativeCoopHostPort).GetField(
            name, BindingFlags.Instance | BindingFlags.NonPublic)!;
        return (T)field.GetValue(port)!;
    }

    private static void SetField(InstalledNativeCoopHostPort port, string name, object? value)
    {
        FieldInfo field = typeof(InstalledNativeCoopHostPort).GetField(
            name, BindingFlags.Instance | BindingFlags.NonPublic)!;
        field.SetValue(port, value);
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

    private static NetFullCombatState StateFor(TestAction action)
    {
        // OnEnqueued wires native queue callbacks and enters Godot's native runtime. The
        // checkpoint probe is a managed correlation test, so set the same generated action ID
        // directly without invoking the host queue.
        MethodInfo setter = typeof(GameAction).GetMethod(
            "set_Id", BindingFlags.Instance | BindingFlags.NonPublic)!;
        setter.Invoke(action, new object?[] { (uint?)19 });
        return new NetFullCombatState { lastExecutedActionId = action.Id };
    }
}
