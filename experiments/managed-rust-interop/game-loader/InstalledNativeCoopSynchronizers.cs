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
        bool canProduceEffect, GameAction? action, string authorityEpoch,
        IReadOnlyCollection<ulong> participantNativeIds)
    {
        OperationId = operationId;
        EffectKind = effectKind;
        BeforeHostGeneration = beforeHostGeneration;
        ChecksumOrdinalBefore = checksumOrdinalBefore;
        IsComplete = isComplete;
        CanProduceEffect = canProduceEffect;
        Action = action;
        AuthorityEpoch = authorityEpoch;
        ParticipantNativeIds = participantNativeIds
            .Distinct()
            .OrderBy(id => id)
            .ToArray();
    }

    internal string OperationId { get; }
    internal string EffectKind { get; }
    internal ulong BeforeHostGeneration { get; }
    internal ulong ChecksumOrdinalBefore { get; }
    internal Func<bool> IsComplete { get; }
    internal bool CanProduceEffect { get; }
    internal GameAction? Action { get; }
    internal string AuthorityEpoch { get; }
    internal IReadOnlyList<ulong> ParticipantNativeIds { get; }
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
        ulong ordinal, string context, NetChecksumData checksum,
        NetFullCombatState fullState)
    {
        if (PassiveChecksumOrdinal.HasValue || Action is null)
            return false;

        // The checksum callback is operation-bound only when its serialized native state names
        // the same action that was admitted. Context text remains a useful diagnostic fence,
        // but it is not the identity by itself and can be reproduced by another action.
        if (!Action.Id.HasValue || fullState.lastExecutedActionId != Action.Id)
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
    /// Requires an exact ID/value match from every remote native peer that participated when the
    /// operation was admitted. The current connection set must still contain every admitted
    /// participant, so a disconnect cannot silently reduce the convergence quorum. A local
    /// checksum, a matching ID with a different value, or a message from an old sender is
    /// insufficient to settle the operation.
    /// </summary>
    internal bool HasMatchingRemoteChecksums(
        IEnumerable<ulong> connectedNativePeerIds, ulong localNativePeerId)
    {
        if (!LocalChecksum.HasValue)
            return false;

        NetChecksumData local = LocalChecksum.Value;
        HashSet<ulong> connected = connectedNativePeerIds.ToHashSet();
        bool foundRemote = false;
        foreach (ulong peerId in ParticipantNativeIds)
        {
            if (!connected.Contains(peerId))
                return false;
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
        ulong checksumOrdinalBefore, GameAction action, Player _, string authorityEpoch,
        IReadOnlyCollection<ulong> participantNativeIds)
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
            action,
            authorityEpoch,
            participantNativeIds);
    }

    internal static NativePendingOperation ForEvent(
        string operationId, CoopVoteDomain domain, ulong beforeHostGeneration,
        ulong checksumOrdinalBefore, Player player, int index, EventSynchronizer synchronizer,
        string authorityEpoch, IReadOnlyCollection<ulong> participantNativeIds)
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
            action: null,
            authorityEpoch,
            participantNativeIds);
    }

    internal static NativePendingOperation ForRelic(
        string operationId, CoopVoteDomain domain, ulong beforeHostGeneration,
        ulong checksumOrdinalBefore, Player _, Func<ulong> checksumOrdinal,
        string authorityEpoch, IReadOnlyCollection<ulong> participantNativeIds)
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
            action: null,
            authorityEpoch,
            participantNativeIds);
    }
}
