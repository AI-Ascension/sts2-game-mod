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
internal sealed partial class RuntimeV3GameplayHost
{
    private const int MaxReceipts = 16_384;
    private readonly IRuntimeV3HostSource _source;
    private readonly IRuntimeV3HostThread _thread;
    private readonly Func<bool> _canDispatch;
    private readonly RuntimeV3GameplayRecoveryStore? _recovery;
    private readonly bool _recoveryRequired;
    private readonly ConcurrentDictionary<RuntimeV3OperationKey, RuntimeV3DispatchReceipt> _receipts = new();
    private readonly object _receiptGate = new();

    internal RuntimeV3GameplayHost(IRuntimeV3HostSource source, IRuntimeV3HostThread thread,
        Func<bool>? canDispatch = null, RuntimeV3GameplayRecoveryStore? recovery = null,
        bool recoveryRequired = false)
    {
        _source = source;
        _thread = thread;
        _canDispatch = canDispatch ?? (() => true);
        _recovery = recovery;
        _recoveryRequired = recoveryRequired;
    }

    internal bool HasDurableRecovery => _recovery is not null;

    internal bool HasPendingMutation
    {
        get
        {
            if (_recovery?.HasPendingMutation == true)
            {
                return true;
            }
            foreach (RuntimeV3DispatchReceipt receipt in _receipts.Values)
            {
                if (receipt.Status is RuntimeV3DispatchStatus.Accepted or RuntimeV3DispatchStatus.Unknown)
                {
                    return true;
                }
            }
            return false;
        }
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
        if (!_receipts.TryGetValue(operation, out receipt) || receipt is null)
        {
            if (_recovery is null
                || !_recovery.TryGet(operation, out RuntimeV3HostOperationRecord? durable)
                || durable is null
                || !TryRestoreReceipt(durable, out RuntimeV3DispatchReceipt? restored)
                || restored is null)
            {
                receipt = null;
                return false;
            }
            receipt = restored;
            _receipts.TryAdd(operation, receipt);
        }
        if (receipt is null)
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
        if (_recoveryRequired && _recovery is null)
        {
            return accepted with
            {
                Status = RuntimeV3DispatchStatus.Unknown,
                ErrorCode = "persistence_unavailable"
            };
        }
        RuntimeV3HostAdmissionTicket? ticket = null;
        bool enqueue = false;
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
            if (_recovery is not null)
            {
                RuntimeV3RecoveryAdmission admission = _recovery.TryAdmit(
                    operation,
                    action,
                    observation,
                    Array.Empty<LegalActionReference>(),
                    out RuntimeV3HostAdmissionTicket? issuedTicket,
                    out RuntimeV3HostOperationRecord? durable,
                    out string durableError);
                if (admission == RuntimeV3RecoveryAdmission.Duplicate
                    && durable is not null
                    && TryRestoreReceipt(durable, out RuntimeV3DispatchReceipt? restored)
                    && restored is not null)
                {
                    _receipts[operation] = restored;
                    return restored;
                }
                if (admission != RuntimeV3RecoveryAdmission.New || issuedTicket is null)
                {
                    return accepted with
                    {
                        Status = admission == RuntimeV3RecoveryAdmission.Conflict
                            ? RuntimeV3DispatchStatus.Rejected
                            : RuntimeV3DispatchStatus.Unknown,
                        ErrorCode = durableError
                    };
                }
                ticket = issuedTicket;
                _receipts[operation] = accepted;
                enqueue = true;
            }
            else
            {
                _receipts[operation] = accepted;
                enqueue = true;
            }
        }
        // Durable ticket publication and receipt insertion are complete before queueing. No
        // callback runs under the receipt lock, including synchronous test queues.
        if (enqueue)
        {
            try { _thread.Enqueue(() => Settle(operation, ticket)); }
            catch (Exception) { MarkUnknown(operation, "dispatch_queue_unavailable"); }
        }
        return CurrentReceipt(operation, accepted);
    }

    internal bool TryGetReceipt(RuntimeV3OperationKey operation, out RuntimeV3DispatchReceipt? receipt)
    {
        CheckCompletion(operation);
        if (_receipts.TryGetValue(operation, out receipt))
        {
            return true;
        }
        return _recovery is not null
            && _recovery.TryGet(operation, out RuntimeV3HostOperationRecord? durable)
            && durable is not null
            && TryRestoreReceipt(durable, out receipt);
    }
}
