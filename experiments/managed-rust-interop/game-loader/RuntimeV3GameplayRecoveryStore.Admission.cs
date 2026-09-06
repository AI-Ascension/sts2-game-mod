// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplayRecoveryStore
{
    internal bool TryReplaceFence(
        RuntimeV3HostBootstrapRequest request, out string error)
    {
        lock (_executionGate)
        {
            lock (_gate)
            {
            error = ValidateBootstrapLocked(request);
            if (error.Length != 0)
            {
                return false;
            }
            if (_fence is not null && request.Fence == _fence)
            {
                return true;
            }
            if (!AppendEventLocked("fence_replaced", SerializeFence(request.Fence, request.Release)))
            {
                error = "persistence_unavailable";
                return false;
            }

            _fence = request.Fence;
            _release = request.Release;
            List<RuntimeV3HostOperationRecord> unresolved = new();
            foreach (RuntimeV3HostOperationRecord record in _operations.Values)
            {
                bool unresolvedRecord = record.Status == RuntimeV3DispatchStatus.Accepted
                    && record.Ticket is not null
                    || record.Status == RuntimeV3DispatchStatus.Unknown && record.WasDispatched;
                if (unresolvedRecord)
                {
                    unresolved.Add(record with
                    {
                        RecoveryState = RuntimeV3HostOperationState.Unknown,
                        Status = RuntimeV3DispatchStatus.Unknown,
                        ErrorCode = "authority_rotated",
                        Ticket = record.Ticket is null ? null
                            : record.Ticket with { State = RuntimeV3HostTicketState.Unknown },
                        UpdatedAt = _clock().ToUniversalTime()
                    });
                }
            }
            foreach (RuntimeV3HostOperationRecord record in unresolved)
            {
                if (!AppendOperationLocked(record))
                {
                    error = "persistence_unavailable";
                    return false;
                }
                _operations[record.Operation] = record;
            }
            return true;
            }
        }
    }

    internal RuntimeV3RecoveryAdmission TryAdmit(
        RuntimeV3OperationKey operation,
        LegalActionReference action,
        RuntimeV3GameplayObservation before,
        IReadOnlyList<LegalActionReference> legalActions,
        out RuntimeV3HostAdmissionTicket? ticket,
        out RuntimeV3HostOperationRecord? record,
        out string error)
    {
        ticket = null;
        record = null;
        lock (_gate)
        {
            error = string.Empty;
            if (!_healthy || _disposed)
            {
                error = "persistence_unavailable";
                return RuntimeV3RecoveryAdmission.Blocked;
            }
            if (!RuntimeV3GameplayRecoveryContract.IsUuid(operation.InstanceId)
                || !RuntimeV3GameplayRecoveryContract.IsUuid(operation.SessionId)
                || !RuntimeV3GameplayRecoveryContract.IsRandomUuid(operation.LeaseId)
                || operation.LeaseEpoch == 0
                || operation.LeaseEpoch > RuntimeV3GameplayRecoveryContract.MaxWireInteger
                || !RuntimeV3GameplayRecoveryContract.IsRandomUuid(operation.OperationId))
            {
                error = "invalid_operation_identity";
                return RuntimeV3RecoveryAdmission.Stale;
            }
            if (!RuntimeV3GameplayRecoveryCanonical.TryCreate(
                    action, out string canonicalAction, out string payloadDigest))
            {
                error = "non_canonical_action";
                return RuntimeV3RecoveryAdmission.Stale;
            }
            foreach (RuntimeV3HostOperationRecord existing in _operations.Values)
            {
                if (existing.Operation != operation)
                {
                    continue;
                }
                if (existing.PayloadDigest == payloadDigest
                    && existing.CanonicalAction == canonicalAction
                    && existing.StateId == before.StateId
                    && existing.Action == action)
                {
                    record = existing;
                    ticket = existing.Ticket;
                    return RuntimeV3RecoveryAdmission.Duplicate;
                }
                error = "idempotency_conflict";
                return RuntimeV3RecoveryAdmission.Conflict;
            }
            if (_fence is null || !CurrentLeaseMatchesLocked(operation)
                || _clock().ToUniversalTime() >= _fence.ExpiresAt)
            {
                error = "host_fence_required";
                return RuntimeV3RecoveryAdmission.Stale;
            }
            if (_operations.Count >= RuntimeV3GameplayRecoveryContract.MaxOperationRecords)
            {
                error = "recovery_operation_capacity";
                return RuntimeV3RecoveryAdmission.Blocked;
            }
            if (!before.Validate(out _)
                || action.Generation != before.Generation
                || !LegalActionCatalog.TryCreate(before.Generation, legalActions, out _))
            {
                error = "invalid_host_projection";
                return RuntimeV3RecoveryAdmission.Stale;
            }

            DateTimeOffset now = _clock().ToUniversalTime();
            DateTimeOffset requestedExpiry = now.AddSeconds(30);
            DateTimeOffset expiry = requestedExpiry < _fence.ExpiresAt
                ? requestedExpiry
                : _fence.ExpiresAt;
            var issued = new RuntimeV3HostAdmissionTicket(
                Guid.NewGuid().ToString("D"),
                operation,
                payloadDigest,
                _fence.BootId,
                _fence.InstanceIncarnation,
                _fence.LeaseEpoch,
                _fence.HostFenceId,
                _fence.AuthorityGeneration,
                _fence.FenceGeneration,
                expiry,
                RuntimeV3HostTicketState.Issued);
            var created = new RuntimeV3HostOperationRecord(
                operation,
                payloadDigest,
                canonicalAction,
                before.StateId,
                action,
                before,
                new List<LegalActionReference>(legalActions).AsReadOnly(),
                RuntimeV3HostOperationState.Accepted,
                RuntimeV3DispatchStatus.Accepted,
                false,
                issued,
                before,
                null,
                null,
                now,
                now);
            if (!AppendOperationLocked(created))
            {
                error = "persistence_unavailable";
                return RuntimeV3RecoveryAdmission.Blocked;
            }
            _operations.Add(operation, created);
            ticket = issued;
            record = created;
            return RuntimeV3RecoveryAdmission.New;
        }
    }

    internal bool TryAuthorizeExecution(
        RuntimeV3HostAdmissionTicket ticket, out string error)
    {
        lock (_gate)
        {
            error = "stale_fence";
            if (!_healthy || _disposed)
            {
                error = "persistence_unavailable";
                return false;
            }
            if (_fence is null || !TicketMatchesFenceLocked(ticket)
                || _clock().ToUniversalTime() >= ticket.ExpiresAt
                || !_operations.TryGetValue(ticket.Operation, out RuntimeV3HostOperationRecord? record)
                || record.Ticket?.TicketId != ticket.TicketId
                || record.Status != RuntimeV3DispatchStatus.Accepted)
            {
                return false;
            }
            RuntimeV3HostOperationRecord admitted = record with
            {
                Ticket = ticket with { State = RuntimeV3HostTicketState.Admitted },
                UpdatedAt = _clock().ToUniversalTime()
            };
            if (!AppendOperationLocked(admitted))
            {
                error = "persistence_unavailable";
                return false;
            }
            _operations[ticket.Operation] = admitted;
            return true;
        }
    }

    internal bool TryMarkExecuting(
        RuntimeV3HostAdmissionTicket ticket, out string error)
    {
        lock (_gate)
        {
            error = "stale_fence";
            if (!_healthy || _disposed)
            {
                error = "persistence_unavailable";
                return false;
            }
            if (_fence is null || !TicketMatchesFenceLocked(ticket)
                || !_operations.TryGetValue(ticket.Operation, out RuntimeV3HostOperationRecord? record)
                || record.Ticket?.TicketId != ticket.TicketId
                || record.Status != RuntimeV3DispatchStatus.Accepted)
            {
                return false;
            }
            RuntimeV3HostOperationRecord executing = record with
            {
                RecoveryState = RuntimeV3HostOperationState.MayHaveBeenDispatched,
                Status = RuntimeV3DispatchStatus.Unknown,
                WasDispatched = true,
                ErrorCode = "dispatch_outcome_unknown",
                Ticket = ticket with { State = RuntimeV3HostTicketState.Executing },
                UpdatedAt = _clock().ToUniversalTime()
            };
            if (!AppendOperationLocked(executing))
            {
                error = "persistence_unavailable";
                return false;
            }
            _operations[ticket.Operation] = executing;
            return true;
        }
    }
}
