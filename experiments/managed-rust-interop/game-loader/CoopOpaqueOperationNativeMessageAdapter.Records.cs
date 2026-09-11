// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class OpaqueOperationNativeMessageAdapter
{
    private void HandleHostRequest(OpaqueOperationNativeMessage message, ulong senderId)
    {
        lock (_recordGate)
            HandleHostRequestLocked(message, senderId);
    }

    private void HandleHostRequestLocked(OpaqueOperationNativeMessage message, ulong senderId)
    {
        if (_hostDispatcher is null || !message.IsRequest())
        {
            SendReply(senderId, OpaqueOperationNativeMessage.RejectedReply("invalid", "invalid_request"));
            return;
        }
        if (_recorded.TryGetValue(message.OriginalOperationId, out RecordedReply? recorded))
        {
            if (recorded.PeerId != senderId)
                SendReply(senderId, OpaqueOperationNativeMessage.RejectedReply(
                    message.OriginalOperationId, "operation_owned_by_another_peer"));
            else
                ReplayOrReconcile(message.OriginalOperationId, recorded, senderId);
            return;
        }
        OpaqueOperationHostDispatchResult dispatch = _hostDispatcher.Dispatch(senderId,
            message.OriginalOperationId, message.Payload);
        OpaqueOperationNativeMessage reply = ToReply(message.OriginalOperationId, dispatch);
        if (reply.Kind != OpaqueOperationNativeMessageKind.RejectedReply)
        {
            if (_recorded.Count >= MaximumRecordedOperations)
            {
                SendReply(senderId, OpaqueOperationNativeMessage.RejectedReply(message.OriginalOperationId,
                    "operation_capacity_exhausted"));
                return;
            }
            _recorded.Add(message.OriginalOperationId, new RecordedReply(senderId, reply));
        }
        if (reply.Kind == OpaqueOperationNativeMessageKind.PendingReply)
            _pendingOperationIds.Enqueue(message.OriginalOperationId);
        SendReply(senderId, reply);
    }

    private void ReplayOrReconcile(string operationId, RecordedReply recorded, ulong senderId)
    {
        if (recorded.Reply.Kind == OpaqueOperationNativeMessageKind.PendingReply)
        {
            recorded = recorded with { Reply = ToReply(operationId,
                _hostDispatcher!.Reconcile(senderId, operationId)) };
            _recorded[operationId] = recorded;
        }
        SendReply(senderId, recorded.Reply);
    }

    private void RefreshHostReplies()
    {
        lock (_recordGate)
        {
            if (_hostDispatcher is null) return;
            for (int refreshed = 0; refreshed < MaximumPendingReconciliationsPerPump
                && _pendingOperationIds.TryDequeue(out string? operationId); refreshed++)
            {
                if (!_recorded.TryGetValue(operationId, out RecordedReply? recorded)
                    || recorded.Reply.Kind != OpaqueOperationNativeMessageKind.PendingReply) continue;
                OpaqueOperationNativeMessage reply = ToReply(operationId,
                    _hostDispatcher.Reconcile(recorded.PeerId, operationId));
                if (SameReply(recorded.Reply, reply)) _pendingOperationIds.Enqueue(operationId);
                else
                {
                    _recorded[operationId] = recorded with { Reply = reply };
                    SendReply(recorded.PeerId, reply);
                }
            }
        }
    }

    private void SendReply(ulong peerId, OpaqueOperationNativeMessage reply) =>
        _outboundReplies.Enqueue(new OutboundReply(peerId, reply));

    private static bool SameReply(OpaqueOperationNativeMessage left, OpaqueOperationNativeMessage right) =>
        left.Kind == right.Kind && left.SettlementWitness == right.SettlementWitness
        && left.RejectionCode == right.RejectionCode;

    private static OpaqueOperationNativeMessage ToReply(string operationId,
        OpaqueOperationHostDispatchResult dispatch) => dispatch.SettlementWitness is not null
        ? OpaqueOperationNativeMessage.SettledReply(operationId, dispatch.SettlementWitness)
        : dispatch.IsPending ? OpaqueOperationNativeMessage.PendingReply(operationId,
            dispatch.RejectionCode ?? "pending") : OpaqueOperationNativeMessage.RejectedReply(operationId,
            dispatch.RejectionCode ?? "host_rejected");

    private sealed record RecordedReply(ulong PeerId, OpaqueOperationNativeMessage Reply);
    private sealed record OutboundReply(ulong PeerId, OpaqueOperationNativeMessage Message);
}
