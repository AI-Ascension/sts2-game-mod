// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.Multiplayer.Game;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>One native operation retained until its game-thread postcondition is observable.</summary>
internal sealed class NativePendingOperation
{
    private NativePendingOperation(string operationId, string effectKind,
        ulong beforeHostGeneration, ulong checksumOrdinalBefore, Func<bool> isComplete,
        bool canProduceEffect, GameAction? action)
    {
        OperationId = operationId;
        EffectKind = effectKind;
        BeforeHostGeneration = beforeHostGeneration;
        ChecksumOrdinalBefore = checksumOrdinalBefore;
        IsComplete = isComplete;
        CanProduceEffect = canProduceEffect;
        Action = action;
    }

    internal string OperationId { get; }
    internal string EffectKind { get; }
    internal ulong BeforeHostGeneration { get; }
    internal ulong ChecksumOrdinalBefore { get; }
    internal Func<bool> IsComplete { get; }
    internal bool CanProduceEffect { get; }
    internal GameAction? Action { get; }
    internal bool Completed { get; set; }

    /// <summary>
    /// Native checksum ordinal observed from the installed RunManager post-action callback for
    /// this exact queued action. The callback has no operation ID, so the adapter binds it by the
    /// immutable action rendering that RunManager places in its checksum context.
    /// </summary>
    internal ulong? PassiveChecksumOrdinal { get; private set; }

    /// <summary>
    /// The local checksum generated for the exact queued action. The native tracker assigns an
    /// ID to the checkpoint and sends that ID/value from clients to the host; retaining the
    /// value here lets the host bind an incoming message to this operation instead of treating
    /// any later checksum as proof.
    /// </summary>
    private NetChecksumData? LocalChecksum { get; set; }

    private readonly Dictionary<ulong, NetChecksumData> _remoteChecksums = new();

    internal bool TryRecordPassiveChecksum(
        ulong ordinal, string context, NetChecksumData checksum)
    {
        if (PassiveChecksumOrdinal.HasValue || Action is null)
            return false;

        const string prefix = "finished action execution ";
        string expectedContext = prefix + (Action.ToString() ?? string.Empty);
        if (!string.Equals(context, expectedContext, StringComparison.Ordinal))
            return false;

        PassiveChecksumOrdinal = ordinal;
        LocalChecksum = checksum;
        return true;
    }

    /// <summary>Retains a checksum received from one authenticated native sender.</summary>
    internal void RecordRemoteChecksum(ulong senderId, NetChecksumData checksum)
    {
        if (senderId != 0)
            _remoteChecksums[senderId] = checksum;
    }

    /// <summary>
    /// Requires an exact ID/value match from every currently connected remote native peer. A
    /// local checksum, a matching ID with a different value, or a message from an old sender is
    /// insufficient to settle the operation.
    /// </summary>
    internal bool HasMatchingRemoteChecksums(
        IEnumerable<ulong> connectedNativePeerIds, ulong localNativePeerId)
    {
        if (!LocalChecksum.HasValue)
            return false;

        NetChecksumData local = LocalChecksum.Value;
        bool foundRemote = false;
        foreach (ulong peerId in connectedNativePeerIds.Distinct())
        {
            if (peerId == localNativePeerId)
                continue;
            foundRemote = true;
            if (!_remoteChecksums.TryGetValue(peerId, out NetChecksumData remote)
                || remote.id != local.id
                || remote.checksum != local.checksum)
            {
                return false;
            }
        }

        return foundRemote;
    }

    internal static NativePendingOperation ForAction(
        string operationId, string actionKind, ulong beforeHostGeneration,
        ulong checksumOrdinalBefore, GameAction action, Player _)
    {
        return new NativePendingOperation(
            operationId,
            $"native_{actionKind}_settled",
            beforeHostGeneration,
            checksumOrdinalBefore,
            () => action.State == MegaCrit.Sts2.Core.Entities.Actions.GameActionState.Finished
                && action.CompletionTask.IsCompletedSuccessfully
                && action.Exception is null,
            canProduceEffect: true,
            action);
    }

    internal static NativePendingOperation ForEvent(
        string operationId, CoopVoteDomain domain, ulong beforeHostGeneration,
        ulong checksumOrdinalBefore, Player player, int index, EventSynchronizer synchronizer)
    {
        return new NativePendingOperation(
            operationId,
            "native_shared_event_vote_settled",
            beforeHostGeneration,
            checksumOrdinalBefore,
            () => synchronizer.GetPlayerVote(player) == (uint)index,
            // This only proves that the local vote was recorded. The host's shared result is
            // selected later by ChooseSharedEventOption and has no public completion witness.
            canProduceEffect: false,
            action: null);
    }

    internal static NativePendingOperation ForRelic(
        string operationId, CoopVoteDomain domain, ulong beforeHostGeneration,
        ulong checksumOrdinalBefore, Player _, Func<ulong> checksumOrdinal)
    {
        // The installed relic synchronizer exposes no public vote identity. A fresh native
        // checksum is therefore the only honest completion hook available to this adapter.
        return new NativePendingOperation(
            operationId,
            "native_treasure_relic_vote_settled",
            beforeHostGeneration,
            checksumOrdinalBefore,
            () => checksumOrdinal() > checksumOrdinalBefore,
            // A checksum advance alone cannot identify this relic vote or its awarded result.
            canProduceEffect: false,
            action: null);
    }
}
