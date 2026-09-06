// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplayHost
{
    private void Settle(RuntimeV3OperationKey operation, RuntimeV3HostAdmissionTicket? ticket)
    {
        if (!_receipts.TryGetValue(operation, out RuntimeV3DispatchReceipt? receipt))
        {
            return;
        }
        if (receipt.Status != RuntimeV3DispatchStatus.Accepted)
        {
            return;
        }
        IDisposable? executionFence = ticket is not null && _recovery is not null
            ? _recovery.AcquireExecutionFence() : null;
        using (executionFence)
        {
        if (ticket is not null && _recovery is not null
            && !_recovery.TryAuthorizeExecution(ticket, out string fenceError))
        {
            if (fenceError == "persistence_unavailable")
            {
                MarkUnknown(operation, fenceError);
            }
            else
            {
                MarkRejected(operation, fenceError);
            }
            return;
        }
        try
        {
            RuntimeV3GameplayObservation current = Observe();
            IReadOnlyList<LegalActionReference> actions = LegalActions(current);
            string? rejection = !_canDispatch() ? "operation_in_progress"
                : current.Generation != receipt.Before.Generation
                || current.StateId != receipt.Before.StateId ? "stale_generation"
                : !current.IsActionable || current.ModalBlocking || !current.InputEnabled ? "input_disabled"
                : !new LegalActionCatalog(current.Generation, actions).ContainsExact(receipt.Action)
                    ? "action_not_current" : null;
            if (rejection is not null)
            {
                _receipts[operation] = receipt with
                {
                    Status = RuntimeV3DispatchStatus.Rejected,
                    Observation = current,
                    LegalActions = actions,
                    ErrorCode = rejection
                };
                if (ticket is not null)
                {
                    MarkRejected(operation, rejection);
                }
                return;
            }

            // This durable transition is the final admission fence. It is written before the
            // source callback because the callback may mutate and then throw or lose its reply.
            receipt = receipt with
            {
                Status = RuntimeV3DispatchStatus.Unknown,
                WasDispatched = true,
                ErrorCode = "dispatch_outcome_unknown"
            };
            _receipts[operation] = receipt;
            if (ticket is not null && _recovery is not null
                && !_recovery.TryMarkExecuting(ticket, out string executionError))
            {
                _receipts[operation] = receipt with { ErrorCode = executionError };
                return;
            }
            if (!_source.Dispatch(operation, receipt.Action))
            {
                _receipts[operation] = receipt with
                {
                    Status = RuntimeV3DispatchStatus.Rejected,
                    Observation = current,
                    LegalActions = actions,
                    ErrorCode = "action_rejected"
                };
                if (ticket is not null)
                {
                    MarkRejected(operation, "action_rejected");
                }
                return;
            }

            _receipts[operation] = receipt with { ErrorCode = "settlement_unproven" };
            if (ticket is not null)
            {
                MarkUnknown(operation, "settlement_unproven");
            }
            CheckCompletion(operation);
        }
        catch (Exception)
        {
            _receipts[operation] = _receipts[operation] with
            {
                Status = RuntimeV3DispatchStatus.Unknown,
                ErrorCode = "settlement_unproven"
            };
            if (ticket is not null)
            {
                MarkUnknown(operation, "settlement_unproven");
            }
        }
        }
    }

    private void CheckCompletion(RuntimeV3OperationKey operation)
    {
        if (!_receipts.TryGetValue(operation, out RuntimeV3DispatchReceipt? receipt)
            || !receipt.WasDispatched || receipt.Status != RuntimeV3DispatchStatus.Unknown)
        {
            return;
        }
        try
        {
            RuntimeV3HostCompletion? completion = _source.Completion(operation, receipt.Action);
            if (completion is null || !PostconditionVerifier.Verify(
                operation, receipt.Action, receipt.Before, completion.Observation,
                completion.Witness, out _))
            {
                return;
            }
            IReadOnlyList<LegalActionReference> actions = SnapshotActions(
                completion.Observation, completion.LegalActions);
            RuntimeV3DispatchReceipt settled = receipt with
            {
                Status = RuntimeV3DispatchStatus.Settled,
                Observation = SnapshotObservation(completion.Observation),
                Witness = completion.Witness,
                LegalActions = actions,
                ErrorCode = null
            };
            if (_recovery is not null
                && !_recovery.TryMarkSettled(operation, completion.Witness, out _))
            {
                _receipts[operation] = settled with
                {
                    Status = RuntimeV3DispatchStatus.Unknown,
                    Observation = null,
                    Witness = null,
                    ErrorCode = "receipt_persistence_unavailable"
                };
                return;
            }
            _receipts[operation] = settled;
        }
        catch (Exception)
        {
            // Preserve explicit uncertainty on completion-source or catalog failure.
            _receipts[operation] = receipt with { ErrorCode = "settlement_unproven" };
        }
    }

    private RuntimeV3DispatchReceipt CurrentReceipt(
        RuntimeV3OperationKey operation, RuntimeV3DispatchReceipt fallback) =>
        _receipts.TryGetValue(operation, out RuntimeV3DispatchReceipt? receipt) && receipt is not null
            ? receipt
            : fallback;

    private void MarkUnknown(RuntimeV3OperationKey operation, string errorCode)
    {
        if (_receipts.TryGetValue(operation, out RuntimeV3DispatchReceipt? receipt))
        {
            _receipts[operation] = receipt with { Status = RuntimeV3DispatchStatus.Unknown,
                ErrorCode = errorCode };
        }
        _recovery?.TryMarkUnknown(operation, errorCode, out _);
    }

    private void MarkRejected(RuntimeV3OperationKey operation, string errorCode)
    {
        if (_recovery is not null
            && !_recovery.TryMarkRejected(operation, errorCode, out _))
        {
            return;
        }
        if (_receipts.TryGetValue(operation, out RuntimeV3DispatchReceipt? receipt))
        {
            _receipts[operation] = receipt with
            {
                Status = RuntimeV3DispatchStatus.Rejected,
                ErrorCode = errorCode
            };
        }
    }
}
