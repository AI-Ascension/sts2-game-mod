// SPDX-License-Identifier: MIT

using System;
using System.Collections.Concurrent;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal enum RuntimeV3DispatchStatus
{
    Accepted,
    Settled,
    Rejected,
    Unknown
}

internal sealed record RuntimeV3DispatchReceipt(
    string OperationId,
    RuntimeV3DispatchStatus Status,
    RuntimeV3GameplayObservation? Observation,
    RuntimeV3TransitionWitness? Witness,
    string? ErrorCode,
    LegalActionReference? Action);

internal interface IRuntimeV3HostSource
{
    RuntimeV3GameplayObservation Observe();
    IReadOnlyList<LegalActionReference> LegalActions(RuntimeV3GameplayObservation observation);
    bool Dispatch(LegalActionReference action);
}

internal interface IRuntimeV3HostThread
{
    void Enqueue(Action work);
}

/// <summary>
/// Host-thread owner for semantic Runtime-v3 operations. It accepts no coordinates or raw input.
/// </summary>
internal sealed class RuntimeV3GameplayHost
{
    private const int MaxReceipts = 4096;
    private readonly IRuntimeV3HostSource _source;
    private readonly IRuntimeV3HostThread _thread;
    private readonly ConcurrentDictionary<string, RuntimeV3DispatchReceipt> _receipts = new(StringComparer.Ordinal);
    private readonly object _receiptGate = new();

    internal RuntimeV3GameplayHost(IRuntimeV3HostSource source, IRuntimeV3HostThread thread)
    {
        _source = source;
        _thread = thread;
    }

    internal RuntimeV3GameplayObservation Observe()
    {
        RuntimeV3GameplayObservation observation = _source.Observe();
        if (!observation.Validate(out string error))
        {
            throw new InvalidOperationException($"invalid host projection: {error}");
        }
        return observation;
    }

    internal IReadOnlyList<LegalActionReference> LegalActions(RuntimeV3GameplayObservation observation)
    {
        if (!observation.Validate(out _))
        {
            return Array.Empty<LegalActionReference>();
        }
        IReadOnlyList<LegalActionReference> actions = _source.LegalActions(observation);
        if (!LegalActionCatalog.TryCreate(
                observation.Generation,
                actions,
                out LegalActionCatalog? catalog)
            || catalog is null)
        {
            throw new InvalidOperationException("invalid host legal-action catalog");
        }
        return catalog.Actions;
    }

    internal RuntimeV3DispatchReceipt Dispatch(
        string operationId,
        RuntimeV3GameplayObservation observation,
        LegalActionReference action)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(operationId)
            || !action.Validate(out _)
            || action.Generation != observation.Generation)
        {
            return new RuntimeV3DispatchReceipt(operationId, RuntimeV3DispatchStatus.Rejected, observation, null, "invalid_action", action);
        }
        lock (_receiptGate)
        {
            if (_receipts.TryGetValue(operationId, out RuntimeV3DispatchReceipt? existing))
            {
                if (existing.Action == action)
                {
                    return existing;
                }
                return new RuntimeV3DispatchReceipt(
                    operationId,
                    RuntimeV3DispatchStatus.Rejected,
                    observation,
                    null,
                    "idempotency_conflict",
                    action);
            }
            if (_receipts.Count >= MaxReceipts)
            {
                return new RuntimeV3DispatchReceipt(
                    operationId,
                    RuntimeV3DispatchStatus.Unknown,
                    null,
                    null,
                    "receipt_capacity_exhausted",
                    action);
            }
            if (!observation.IsActionable || observation.ModalBlocking || !observation.InputEnabled)
            {
                return new RuntimeV3DispatchReceipt(operationId, RuntimeV3DispatchStatus.Rejected, observation, null, "input_disabled", action);
            }

            RuntimeV3DispatchReceipt accepted = new(operationId, RuntimeV3DispatchStatus.Accepted, observation, null, null, action);
            if (!_receipts.TryAdd(operationId, accepted))
            {
                RuntimeV3DispatchReceipt raced = _receipts[operationId];
                return raced.Action == action
                    ? raced
                    : new RuntimeV3DispatchReceipt(
                        operationId,
                        RuntimeV3DispatchStatus.Rejected,
                        observation,
                        null,
                        "idempotency_conflict",
                        action);
            }
            try
            {
                _thread.Enqueue(() => Settle(operationId, observation, action));
            }
            catch (Exception)
            {
                _receipts[operationId] = new RuntimeV3DispatchReceipt(
                    operationId,
                    RuntimeV3DispatchStatus.Unknown,
                    null,
                    null,
                    "dispatch_queue_unavailable",
                    action);
                return _receipts[operationId];
            }
            return _receipts.TryGetValue(operationId, out RuntimeV3DispatchReceipt? current)
                ? current
                : accepted;
        }
    }

    internal bool TryGetReceipt(string operationId, out RuntimeV3DispatchReceipt? receipt) =>
        _receipts.TryGetValue(operationId, out receipt);

    private void Settle(
        string operationId,
        RuntimeV3GameplayObservation before,
        LegalActionReference action)
    {
        RuntimeV3GameplayObservation current;
        IReadOnlyList<LegalActionReference> actions;
        try
        {
            current = Observe();
            actions = LegalActions(current);
        }
        catch (Exception)
        {
            _receipts[operationId] = new RuntimeV3DispatchReceipt(
                operationId,
                RuntimeV3DispatchStatus.Unknown,
                null,
                null,
                "settlement_unproven",
                action);
            return;
        }
        string? rejection = null;
        if (current.Generation != before.Generation
            || !string.Equals(current.StateId, before.StateId, StringComparison.Ordinal))
        {
            rejection = "stale_generation";
        }
        else if (!current.IsActionable || current.ModalBlocking || !current.InputEnabled)
        {
            rejection = "input_disabled";
        }
        else if (!new LegalActionCatalog(current.Generation, actions).ContainsExact(action))
        {
            rejection = "action_not_current";
        }
        if (rejection is not null)
        {
            _receipts[operationId] = new RuntimeV3DispatchReceipt(
                operationId,
                RuntimeV3DispatchStatus.Rejected,
                current,
                null,
                rejection,
                action);
            return;
        }

        bool dispatched;
        try
        {
            dispatched = _source.Dispatch(action);
        }
        catch (Exception)
        {
            _receipts[operationId] = new RuntimeV3DispatchReceipt(
                operationId,
                RuntimeV3DispatchStatus.Unknown,
                null,
                null,
                "dispatch_outcome_unknown",
                action);
            return;
        }
        if (!dispatched)
        {
            _receipts[operationId] = new RuntimeV3DispatchReceipt(
                operationId,
                RuntimeV3DispatchStatus.Rejected,
                current,
                null,
                "action_rejected",
                action);
            return;
        }

        RuntimeV3GameplayObservation after;
        try
        {
            after = Observe();
        }
        catch (Exception)
        {
            _receipts[operationId] = new RuntimeV3DispatchReceipt(
                operationId,
                RuntimeV3DispatchStatus.Unknown,
                null,
                null,
                "settlement_unproven",
                action);
            return;
        }
        RuntimeV3TransitionWitness witness = new(
            before.Generation,
            after.Generation,
            after.StateId,
            $"{action.ActionId}.settled");
        if (PostconditionVerifier.Verify(before, after, witness, out _))
        {
            _receipts[operationId] = new RuntimeV3DispatchReceipt(
                operationId,
                RuntimeV3DispatchStatus.Settled,
                after,
                witness,
                null,
                action);
        }
        else
        {
            _receipts[operationId] = new RuntimeV3DispatchReceipt(
                operationId,
                RuntimeV3DispatchStatus.Unknown,
                null,
                null,
                "settlement_unproven",
                action);
        }
    }
}
