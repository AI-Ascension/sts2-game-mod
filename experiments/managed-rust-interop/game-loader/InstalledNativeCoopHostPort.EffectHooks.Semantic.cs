// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Entities.TreasureRelicPicking;
using MegaCrit.Sts2.Core.Events;
using MegaCrit.Sts2.Core.Helpers;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Multiplayer.Game;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    private void RefreshEventModelHooks(EventSynchronizer? synchronizer)
    {
        if (synchronizer is null)
            return;
        HashSet<EventModel> current = synchronizer.Events.ToHashSet();
        foreach (EventModel eventModel in _effectEventModels.ToList())
        {
            if (current.Contains(eventModel))
                continue;
            eventModel.StateChanged -= OnNativeEventStateChanged;
            _effectEventModels.Remove(eventModel);
        }
        foreach (EventModel eventModel in current)
        {
            if (_effectEventModels.Add(eventModel))
                eventModel.StateChanged += OnNativeEventStateChanged;
        }
    }

    private void OnNativeEventVoteChanged(Player _)
    {
        EventSynchronizer? synchronizer = _effectEventSynchronizer;
        if (synchronizer is null)
            return;
        try
        {
            if (!synchronizer.IsShared || synchronizer.Events.Count == 0)
                return;
        }
        catch
        {
            return;
        }

        RefreshEventModelHooks(synchronizer);

        NativeEventEffectBatch batch = _nativeEventBatch is { } current
            && ReferenceEquals(current.Synchronizer, synchronizer)
            && current.Expected.SetEquals(synchronizer.Events)
            ? current
            : new NativeEventEffectBatch(synchronizer);
        batch.VoteObserved = true;
        batch.Observed.Clear();
        batch.CheckpointScheduled = false;
        _nativeEventBatch = batch;
        foreach (NativePendingOperation pending in _pending.Values)
            pending.TryRecordEventVote(synchronizer);
    }

    private void OnNativeEventStateChanged(EventModel eventModel)
    {
        NativeEventEffectBatch? batch = _nativeEventBatch;
        if (batch is null || !batch.VoteObserved || !batch.Expected.Contains(eventModel))
            return;
        batch.Observed.Add(eventModel);
        foreach (NativePendingOperation pending in _pending.Values)
            pending.TryRecordEventState(batch.Synchronizer, eventModel);
        if (batch.IsComplete && !batch.CheckpointScheduled)
        {
            batch.CheckpointScheduled = true;
            TaskHelper.RunSafely(GenerateEventSemanticCheckpoint(batch));
        }
    }

    private async Task GenerateEventSemanticCheckpoint(NativeEventEffectBatch batch)
    {
        // ChooseOptionForSharedEvent adds every option task after its first StateChanged callback.
        // Yield once so AwaitPendingOptionTasks observes the complete first-party task list.
        await Task.Yield();
        try
        {
            await batch.Synchronizer.AwaitPendingOptionTasks();
            await Task.Yield();
            if (ReferenceEquals(_nativeEventBatch, batch) && batch.IsComplete)
                GenerateSemanticChecksum(NativeSemanticContexts.SharedEvent);
        }
        catch
        {
            // A failed option task is retained as unknown. Its next native observation may still
            // recover the operation; this hook never turns an exception into settlement.
        }
    }

    private void OnNativeRelicsAwarded(List<RelicPickingResult> results)
    {
        TreasureRoomRelicSynchronizer? synchronizer = _effectRelicSynchronizer;
        if (synchronizer is null)
            return;
        NativeRelicEffectBatch batch = new(synchronizer, results);
        _nativeRelicBatch = batch;
        foreach (NativePendingOperation pending in _pending.Values)
            pending.TryRecordRelicAward(synchronizer, results);
        if (batch.IsComplete)
            ScheduleRelicSemanticCheckpoint(batch);
    }

    private void OnNativeRelicObtained(RelicModel relic)
    {
        Player? player = relic.Owner;
        if (player is null)
            return;
        NativeRelicEffectBatch? batch = _nativeRelicBatch;
        if (batch is null)
            return;
        int expectedIndex = batch.ExpectedObtains
            .FindIndex(item => item == (player.NetId, relic.Id));
        if (expectedIndex < 0)
            return;
        batch.ExpectedObtains.RemoveAt(expectedIndex);
        batch.Obtained.Add((player.NetId, relic.Id));
        foreach (NativePendingOperation pending in _pending.Values)
            pending.TryRecordRelicObtained(player, relic);
        if (batch.IsComplete)
            ScheduleRelicSemanticCheckpoint(batch);
    }

    private void ScheduleRelicSemanticCheckpoint(NativeRelicEffectBatch batch)
    {
        if (batch.CheckpointScheduled)
            return;
        batch.CheckpointScheduled = true;
        TaskHelper.RunSafely(GenerateRelicSemanticCheckpoint(batch));
    }

    private async Task GenerateRelicSemanticCheckpoint(NativeRelicEffectBatch batch)
    {
        // Relic awards are raised before EndRelicVoting and before the UI starts its inventory
        // tasks. For awarded relics this method is reached only after every RelicObtained event;
        // skips have no inventory mutation and are complete at RelicsAwarded itself.
        await Task.Yield();
        if (ReferenceEquals(_nativeRelicBatch, batch) && batch.IsComplete)
            GenerateSemanticChecksum(NativeSemanticContexts.TreasureRelic);
    }
}
