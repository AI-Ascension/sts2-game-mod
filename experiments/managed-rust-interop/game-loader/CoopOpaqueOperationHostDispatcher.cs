// SPDX-License-Identifier: MIT

using System;
using System.Collections.Concurrent;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Host-owned bridge from a transport-authenticated native sender to the existing co-op runtime.
/// Native-message handlers may run away from the game thread, so they only enqueue owned bytes.
/// <see cref="Pump"/> is invoked from the Godot frame callback and is the sole path that reads
/// host state or calls a native action/vote producer.
/// </summary>
internal sealed partial class CoopOpaqueOperationHostDispatcher : IHostAdmittedOpaqueOperationDispatcher
{
    private const int MaximumPendingOperations = 4096;
    private readonly CoopNativeRuntime _runtime;
    private readonly ConcurrentQueue<QueuedOperation> _queue = new();
    private readonly object _gate = new();
    private readonly Dictionary<string, OperationState> _operations = new(StringComparer.Ordinal);

    internal CoopOpaqueOperationHostDispatcher(CoopNativeRuntime runtime) =>
        _runtime = runtime ?? throw new ArgumentNullException(nameof(runtime));

    public OpaqueOperationHostDispatchResult Dispatch(ulong authenticatedPeerId,
        string originalOperationId, byte[] payload)
    {
        if (authenticatedPeerId == 0 || payload is null || payload.Length == 0)
            return OpaqueOperationHostDispatchResult.Rejected("invalid_request");
        if (!IsPreAdmissionPayloadValid(payload))
            return OpaqueOperationHostDispatchResult.Rejected("invalid_request_json");

        lock (_gate)
        {
            if (_operations.TryGetValue(originalOperationId, out OperationState? existing))
            {
                return existing.NativePeerId == authenticatedPeerId
                    ? ResultFor(existing.Receipt)
                    : OpaqueOperationHostDispatchResult.Rejected("operation_owned_by_another_peer");
            }
            if (_operations.Count >= MaximumPendingOperations)
                return OpaqueOperationHostDispatchResult.Rejected("operation_capacity_exhausted");

            byte[] ownedPayload = new byte[payload.Length];
            Array.Copy(payload, ownedPayload, ownedPayload.Length);
            _operations.Add(originalOperationId, new OperationState(authenticatedPeerId, null));
            _queue.Enqueue(new QueuedOperation(authenticatedPeerId, originalOperationId, ownedPayload));
        }
        return OpaqueOperationHostDispatchResult.Pending("queued");
    }

    public OpaqueOperationHostDispatchResult Reconcile(ulong authenticatedPeerId,
        string originalOperationId)
    {
        if (authenticatedPeerId == 0)
            return OpaqueOperationHostDispatchResult.Rejected("invalid_request");
        lock (_gate)
        {
            if (!_operations.TryGetValue(originalOperationId, out OperationState? state))
                return OpaqueOperationHostDispatchResult.Rejected("unknown_operation");
            return state.NativePeerId == authenticatedPeerId
                ? ResultFor(state.Receipt)
                : OpaqueOperationHostDispatchResult.Rejected("operation_owned_by_another_peer");
        }
    }

    /// <summary>Called only by the main-thread frame callback installed by ModEntry.</summary>
    internal void Pump()
    {
        while (_queue.TryDequeue(out QueuedOperation? queued))
        {
            CoopOperationReceipt receipt = Execute(queued);
            lock (_gate)
            {
                if (_operations.TryGetValue(queued.OperationId, out OperationState? state)
                    && state.NativePeerId == queued.NativePeerId)
                {
                    _operations[queued.OperationId] = state with { Receipt = receipt };
                }
            }
        }

        KeyValuePair<string, OperationState>[] states;
        lock (_gate)
        {
            var snapshot = new List<KeyValuePair<string, OperationState>>(_operations.Count);
            foreach (KeyValuePair<string, OperationState> entry in _operations)
                snapshot.Add(entry);
            states = snapshot.ToArray();
        }
        foreach (KeyValuePair<string, OperationState> entry in states)
        {
            if (entry.Value.Receipt is not { Outcome: CoopOutcome.Accepted or CoopOutcome.Unknown })
                continue;
            if (_runtime.ReconcileOpaqueOperation(entry.Key, out CoopOperationReceipt? receipt)
                && receipt is not null)
            {
                lock (_gate)
                {
                    if (_operations.TryGetValue(entry.Key, out OperationState? current)
                        && current.NativePeerId == entry.Value.NativePeerId)
                    {
                        _operations[entry.Key] = current with { Receipt = receipt };
                    }
                }
            }
        }
    }

    private CoopOperationReceipt Execute(QueuedOperation queued)
    {
        if (LooksLikeFrozenV1Envelope(queued.Payload))
            return _runtime.DispatchAuthenticatedOpaqueCarrier(queued.NativePeerId,
                queued.OperationId, queued.Payload);
        if (!_runtime.TryResolveAuthenticatedNativePeer(queued.NativePeerId,
                out CoopNativePeerBinding binding))
        {
            return Rejected(queued.OperationId, "unknown_or_stale_peer_identity");
        }
        if (!TryParse(queued.Payload, out CarrierRequest? request, out string error))
            return Rejected(queued.OperationId, error);

        return request!.Kind switch
        {
            CarrierKind.LocalAction => _runtime.DispatchAuthenticatedLocalAction(new(
                queued.OperationId, request.ExpectedHostGeneration, binding.OpaquePeerId,
                request.ActionKind!, request.ActionId, null), binding),
            CarrierKind.SharedVote => _runtime.DispatchAuthenticatedSharedVote(new(
                queued.OperationId, request.ProposalId!, request.ExpectedHostGeneration,
                binding.OpaquePeerId, VoteDomain(request.ProposalId!), request.Choice!), binding),
            _ => Rejected(queued.OperationId, "invalid_request")
        };
    }

    private static bool LooksLikeFrozenV1Envelope(byte[] payload)
    {
        foreach (byte value in payload)
        {
            if (value is (byte)' ' or (byte)'\t' or (byte)'\r' or (byte)'\n')
                continue;
            return value == (byte)'{';
        }
        return false;
    }

    private static OpaqueOperationHostDispatchResult ResultFor(CoopOperationReceipt? receipt)
    {
        if (receipt is null)
            return OpaqueOperationHostDispatchResult.Pending("queued");
        return receipt.Outcome switch
        {
            CoopOutcome.Settled or CoopOutcome.Recovered => OpaqueOperationHostDispatchResult.Settled(
                $"receipt-{receipt.OperationId}"),
            CoopOutcome.Rejected => OpaqueOperationHostDispatchResult.Rejected(
                SafeCode(receipt.ErrorCode, "host_rejected")),
            CoopOutcome.Unknown => OpaqueOperationHostDispatchResult.Pending(
                SafeCode(receipt.ErrorCode, "unknown")),
            _ => OpaqueOperationHostDispatchResult.Pending(
                SafeCode(receipt.ErrorCode, "accepted"))
        };
    }

    private static CoopOperationReceipt Rejected(string operationId, string errorCode)
    {
        var observation = new CoopHostObservation("unavailable", "peer:local", CoopHostRole.Unknown,
            "unknown", null, 0, "digest:unknown",
            new[] { new CoopPeerSnapshot("peer:local", true, false, 0, "digest:unknown") },
            true, null);
        return new CoopOperationReceipt(operationId, "opaque", CoopOutcome.Rejected, 0,
            null, null, observation, errorCode);
    }

    private static bool IsPreAdmissionPayloadValid(byte[] payload)
    {
        if (!LooksLikeFrozenV1Envelope(payload))
            return TryParse(payload, out _, out _);
        try
        {
            using var document = System.Text.Json.JsonDocument.Parse(payload,
                new System.Text.Json.JsonDocumentOptions { MaxDepth = 16 });
            return document.RootElement.ValueKind == System.Text.Json.JsonValueKind.Object;
        }
        catch (System.Text.Json.JsonException) { return false; }
    }

    private sealed record QueuedOperation(ulong NativePeerId, string OperationId, byte[] Payload);
    private sealed record OperationState(ulong NativePeerId, CoopOperationReceipt? Receipt);
}
