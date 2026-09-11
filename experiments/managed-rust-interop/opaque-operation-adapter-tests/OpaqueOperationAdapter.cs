// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.OpaqueOperationAdapter;

/// <summary>
/// Source-only boundary for a future admitted native-message binding. It deliberately does not
/// implement a host message interface or choose a host type ID: metadata alone cannot prove
/// that a mod-defined message is authenticated or admitted by the exact host.
/// </summary>
internal sealed class OpaqueOperationAdapter
{
    private readonly IHostAdmittedActionDispatcher _hostDispatcher;
    private readonly Dictionary<string, RecordedReply> _replies = new(StringComparer.Ordinal);

    internal OpaqueOperationAdapter(IHostAdmittedActionDispatcher hostDispatcher)
    {
        _hostDispatcher = hostDispatcher ?? throw new ArgumentNullException(nameof(hostDispatcher));
    }

    internal void Handle(IAuthenticatedPeerLink origin, OpaqueOperationRequest request)
    {
        ArgumentNullException.ThrowIfNull(origin);
        ArgumentNullException.ThrowIfNull(request);

        if (!OpaqueOriginalOperationId.TryCreate(request.OriginalOperationId, out OpaqueOriginalOperationId operationId))
        {
            origin.Send(OpaqueOperationReply.Rejected("invalid_original_operation_id"));
            return;
        }

        if (request.Payload.Length > OpaqueOperationRequest.MaximumPayloadBytes)
        {
            origin.Send(OpaqueOperationReply.Rejected("payload_too_large"));
            return;
        }

        if (_replies.TryGetValue(operationId.Value, out RecordedReply? recorded))
        {
            RecordedReply existing = recorded!;
            origin.Send(existing.PeerId == origin.PeerId
                ? existing.Reply
                : OpaqueOperationReply.Rejected("operation_owned_by_another_peer"));
            return;
        }

        // The host receives the opaque operation identity directly. This adapter never derives
        // settlement from actor/action/generation tuples, a client claim, or a retry.
        HostDispatchResult result = _hostDispatcher.Dispatch(new HostAdmittedOperation(
            operationId, request.Payload));
        OpaqueOperationReply reply = result.IsSettled
            ? OpaqueOperationReply.Settled(result.HostSettlementWitness!)
            : OpaqueOperationReply.Rejected(result.RejectionCode!);
        _replies.Add(operationId.Value, new RecordedReply(origin.PeerId, reply));

        // Reply on the very authenticated link that submitted the operation; no peer lookup or
        // broadcast is available at this boundary.
        origin.Send(reply);
    }

    private sealed record RecordedReply(string PeerId, OpaqueOperationReply Reply);
}

internal sealed class OpaqueOperationRequest
{
    internal const int MaximumPayloadBytes = 4 * 1024;

    internal OpaqueOperationRequest(string originalOperationId, byte[] payload)
    {
        OriginalOperationId = originalOperationId ?? throw new ArgumentNullException(nameof(originalOperationId));
        ArgumentNullException.ThrowIfNull(payload);
        Payload = new byte[payload.Length];
        Array.Copy(payload, Payload, payload.Length);
    }

    internal string OriginalOperationId { get; }
    internal byte[] Payload { get; }
}

internal readonly record struct OpaqueOriginalOperationId
{
    internal const int MaximumLength = 96;

    private OpaqueOriginalOperationId(string value) => Value = value;

    internal string Value { get; }

    internal static bool TryCreate(string? value, out OpaqueOriginalOperationId operationId)
    {
        operationId = default;
        if (string.IsNullOrEmpty(value) || value.Length > MaximumLength)
            return false;
        foreach (char character in value)
        {
            if (!(character is >= 'a' and <= 'z' or >= 'A' and <= 'Z' or >= '0' and <= '9'
                or '-' or '_'))
            {
                return false;
            }
        }
        operationId = new OpaqueOriginalOperationId(value);
        return true;
    }
}

internal interface IAuthenticatedPeerLink
{
    string PeerId { get; }
    void Send(OpaqueOperationReply reply);
}

/// <summary>
/// Implemented only by the host-side admission/dispatch boundary. The request type does not carry
/// a witness, and the adapter exposes no API for a client to supply one.
/// </summary>
internal interface IHostAdmittedActionDispatcher
{
    HostDispatchResult Dispatch(HostAdmittedOperation operation);
}

internal sealed class HostAdmittedOperation
{
    internal HostAdmittedOperation(OpaqueOriginalOperationId originalOperationId, byte[] payload)
    {
        OriginalOperationId = originalOperationId;
        Payload = new byte[payload.Length];
        Array.Copy(payload, Payload, payload.Length);
    }

    internal OpaqueOriginalOperationId OriginalOperationId { get; }
    internal byte[] Payload { get; }
}

internal sealed class HostDispatchResult
{
    private HostDispatchResult(string? hostSettlementWitness, string? rejectionCode)
    {
        HostSettlementWitness = hostSettlementWitness;
        RejectionCode = rejectionCode;
    }

    internal bool IsSettled => HostSettlementWitness is not null;
    internal string? HostSettlementWitness { get; }
    internal string? RejectionCode { get; }

    internal static HostDispatchResult Settled(string hostSettlementWitness)
    {
        if (string.IsNullOrEmpty(hostSettlementWitness) || hostSettlementWitness.Length > 256)
            throw new ArgumentOutOfRangeException(nameof(hostSettlementWitness));
        return new HostDispatchResult(hostSettlementWitness, null);
    }

    internal static HostDispatchResult Rejected(string rejectionCode)
    {
        if (string.IsNullOrEmpty(rejectionCode) || rejectionCode.Length > 64)
            throw new ArgumentOutOfRangeException(nameof(rejectionCode));
        return new HostDispatchResult(null, rejectionCode);
    }
}

internal sealed class OpaqueOperationReply
{
    private OpaqueOperationReply(string disposition, string? hostSettlementWitness, string? rejectionCode)
    {
        Disposition = disposition;
        HostSettlementWitness = hostSettlementWitness;
        RejectionCode = rejectionCode;
    }

    internal string Disposition { get; }
    internal string? HostSettlementWitness { get; }
    internal string? RejectionCode { get; }

    internal static OpaqueOperationReply Settled(string hostSettlementWitness) =>
        new("settled", hostSettlementWitness, null);

    internal static OpaqueOperationReply Rejected(string rejectionCode) =>
        new("rejected", null, rejectionCode);
}
