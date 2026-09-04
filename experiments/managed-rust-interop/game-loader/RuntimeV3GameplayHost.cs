// SPDX-License-Identifier: MIT

using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Collections.ObjectModel;

namespace AiAscension.Sts2GameMod.Runtime;

internal enum RuntimeV3DispatchStatus { Accepted, Settled, Rejected, Unknown }

internal sealed record RuntimeV3OperationKey(
    string InstanceId, string SessionId, string LeaseId, ulong LeaseEpoch, string OperationId);

internal sealed record RuntimeV3DispatchReceipt(
    string OperationId,
    RuntimeV3DispatchStatus Status,
    RuntimeV3GameplayObservation? Observation,
    RuntimeV3TransitionWitness? Witness,
    string? ErrorCode,
    LegalActionReference Action,
    RuntimeV3GameplayObservation Before,
    IReadOnlyList<LegalActionReference> LegalActions,
    bool WasDispatched = false);

internal sealed record RuntimeV3HostCompletion(
    RuntimeV3GameplayObservation Observation,
    RuntimeV3TransitionWitness Witness,
    IReadOnlyList<LegalActionReference> LegalActions);

internal interface IRuntimeV3HostSource
{
    RuntimeV3GameplayObservation Observe();
    IReadOnlyList<LegalActionReference> LegalActions(RuntimeV3GameplayObservation observation);
    bool Dispatch(RuntimeV3OperationKey operation, LegalActionReference action);
    // Only return evidence captured from completion of this exact operation, never from
    // a generation change alone. Null means the outcome is still unproven.
    RuntimeV3HostCompletion? Completion(RuntimeV3OperationKey operation, LegalActionReference action);
}

internal interface IRuntimeV3HostThread { void Enqueue(Action work); }

/// <summary>Host-thread owner; source reads, completion checks and mutations run on that thread.</summary>
internal sealed class RuntimeV3GameplayHost
{
    private const int MaxReceipts = 4096;
    private readonly IRuntimeV3HostSource _source;
    private readonly IRuntimeV3HostThread _thread;
    private readonly ConcurrentDictionary<RuntimeV3OperationKey, RuntimeV3DispatchReceipt> _receipts = new();
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
        return SnapshotObservation(observation);
    }

    private static RuntimeV3GameplayObservation SnapshotObservation(RuntimeV3GameplayObservation observation) =>
        observation with
        {
            Player = observation.Player with
            {
                Hand = new List<RuntimeV3GameplayCard>(observation.Player.Hand).AsReadOnly(),
                Deck = new List<RuntimeV3GameplayCard>(observation.Player.Deck).AsReadOnly(),
                Discard = new List<RuntimeV3GameplayCard>(observation.Player.Discard).AsReadOnly(),
                Exhaust = new List<RuntimeV3GameplayCard>(observation.Player.Exhaust).AsReadOnly()
            },
            StateValues = new List<string>(observation.StateValues).AsReadOnly(),
            Enemies = new List<RuntimeV3GameplayEnemy>(observation.Enemies).AsReadOnly(),
            ShopItems = new List<RuntimeV3GameplayShopItem>(observation.ShopItems).AsReadOnly()
        };

    internal IReadOnlyList<LegalActionReference> LegalActions(RuntimeV3GameplayObservation observation)
    {
        if (!observation.Validate(out _) || !observation.IsActionable
            || observation.ModalBlocking || !observation.InputEnabled)
        {
            return Array.Empty<LegalActionReference>();
        }
        return SnapshotActions(observation, _source.LegalActions(observation));
    }

    private static ReadOnlyCollection<LegalActionReference> SnapshotActions(
        RuntimeV3GameplayObservation observation, IReadOnlyList<LegalActionReference> actions)
    {
        if (!LegalActionCatalog.TryCreate(observation.Generation, actions, out _))
        {
            throw new InvalidOperationException("invalid host legal-action catalog");
        }
        return new List<LegalActionReference>(actions).AsReadOnly();
    }

    internal bool TryReplay(RuntimeV3OperationKey operation, string stateId,
        LegalActionReference action, out RuntimeV3DispatchReceipt? receipt)
    {
        if (!_receipts.TryGetValue(operation, out receipt))
        {
            return false;
        }
        if (receipt.Action != action || receipt.Before.StateId != stateId)
        {
            receipt = receipt with { Status = RuntimeV3DispatchStatus.Rejected,
                Witness = null, ErrorCode = "idempotency_conflict" };
        }
        return true;
    }

    internal RuntimeV3DispatchReceipt Dispatch(RuntimeV3OperationKey operation,
        RuntimeV3GameplayObservation observation, LegalActionReference action)
    {
        RuntimeV3DispatchReceipt accepted = new(operation.OperationId, RuntimeV3DispatchStatus.Accepted,
            observation, null, null, action, observation, Array.Empty<LegalActionReference>());
        if (!RuntimeV3GameplayContract.IsIdentity(operation.OperationId)
            || !action.Validate(out _) || action.Generation != observation.Generation)
        {
            return accepted with { Status = RuntimeV3DispatchStatus.Rejected, ErrorCode = "invalid_action" };
        }
        lock (_receiptGate)
        {
            if (TryReplay(operation, observation.StateId, action, out RuntimeV3DispatchReceipt? existing)
                && existing is not null)
            {
                return existing;
            }
            if (_receipts.Count >= MaxReceipts)
            {
                return accepted with { Status = RuntimeV3DispatchStatus.Unknown,
                    ErrorCode = "receipt_capacity_exhausted" };
            }
            _receipts[operation] = accepted;
        }
        // A synchronous queue may call the host immediately. Never hold the receipt lock
        // across host callbacks or queue implementations.
        try { _thread.Enqueue(() => Settle(operation)); }
        catch (Exception)
        {
            _receipts[operation] = _receipts[operation] with {
                Status = RuntimeV3DispatchStatus.Unknown, ErrorCode = "dispatch_queue_unavailable" };
        }
        return _receipts[operation];
    }

    internal bool TryGetReceipt(RuntimeV3OperationKey operation, out RuntimeV3DispatchReceipt? receipt)
    {
        CheckCompletion(operation);
        return _receipts.TryGetValue(operation, out receipt);
    }

    private void Settle(RuntimeV3OperationKey operation)
    {
        RuntimeV3DispatchReceipt receipt = _receipts[operation];
        try
        {
            RuntimeV3GameplayObservation current = Observe();
            IReadOnlyList<LegalActionReference> actions = LegalActions(current);
            string? rejection = current.Generation != receipt.Before.Generation
                || current.StateId != receipt.Before.StateId ? "stale_generation"
                : !current.IsActionable || current.ModalBlocking || !current.InputEnabled ? "input_disabled"
                : !new LegalActionCatalog(current.Generation, actions).ContainsExact(receipt.Action)
                    ? "action_not_current" : null;
            if (rejection is not null)
            {
                _receipts[operation] = receipt with { Status = RuntimeV3DispatchStatus.Rejected,
                    Observation = current, LegalActions = actions, ErrorCode = rejection };
                return;
            }
            // Mark uncertainty before invoking a callback that may mutate then throw.
            receipt = receipt with { Status = RuntimeV3DispatchStatus.Unknown,
                WasDispatched = true, ErrorCode = "dispatch_outcome_unknown" };
            _receipts[operation] = receipt;
            if (!_source.Dispatch(operation, receipt.Action))
            {
                _receipts[operation] = receipt with { Status = RuntimeV3DispatchStatus.Rejected,
                    Observation = current, LegalActions = actions, ErrorCode = "action_rejected" };
                return;
            }
            _receipts[operation] = receipt with { ErrorCode = "settlement_unproven" };
            CheckCompletion(operation);
        }
        catch (Exception)
        {
            _receipts[operation] = _receipts[operation] with {
                Status = RuntimeV3DispatchStatus.Unknown, ErrorCode = "settlement_unproven" };
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
                operation, receipt.Action, receipt.Before, completion.Observation, completion.Witness, out _))
            {
                return;
            }
            IReadOnlyList<LegalActionReference> actions = SnapshotActions(
                completion.Observation, completion.LegalActions);
            _receipts[operation] = receipt with { Status = RuntimeV3DispatchStatus.Settled,
                Observation = SnapshotObservation(completion.Observation), Witness = completion.Witness,
                LegalActions = actions, ErrorCode = null };
        }
        catch (Exception)
        {
            // Preserve explicit uncertainty on completion-source or catalog failure.
            _receipts[operation] = receipt with { ErrorCode = "settlement_unproven" };
        }
    }
}
