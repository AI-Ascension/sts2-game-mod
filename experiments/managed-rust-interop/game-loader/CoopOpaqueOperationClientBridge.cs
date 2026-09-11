// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Client-owned causal record for one first-party operation.  It may originate an already
/// gateway-admitted v1 envelope, but never turns a client event into settlement: until the
/// authenticated host sends a reply, the only observable result is unknown/pending.
/// </summary>
internal sealed class CoopOpaqueOperationClientBridge
{
    private readonly Func<OpaqueOperationNativeMessageAdapter?> _adapter;
    private readonly object _gate = new();
    private PendingOperation? _pending;

    internal CoopOpaqueOperationClientBridge(Func<OpaqueOperationNativeMessageAdapter?> adapter) =>
        _adapter = adapter ?? throw new ArgumentNullException(nameof(adapter));

    internal bool TrySubmit(CoopNativeRuntime runtime, string expectedKind, string instanceId,
        string sessionId, string leaseId, string correlationId, string leaseEpochText, string body,
        out string operationId, out string errorCode)
    {
        operationId = string.Empty;
        errorCode = "coop_native_client_operation_pending";
        if (!runtime.TryCreateClientCarrierRequest(expectedKind, instanceId, sessionId, leaseId,
                correlationId, leaseEpochText, body, out operationId, out byte[] payload,
                out errorCode))
        {
            return false;
        }

        lock (_gate)
        {
            if (_pending is not null)
            {
                errorCode = _pending.OperationId == operationId
                    ? "coop_native_duplicate_pending_operation"
                    : "coop_native_client_operation_pending";
                return false;
            }
            OpaqueOperationNativeMessageAdapter? adapter = _adapter();
            if (adapter is null)
            {
                errorCode = "coop_native_client_transport_unavailable";
                return false;
            }
            _pending = new PendingOperation(operationId, payload);
            try
            {
                adapter.SendRequest(operationId, payload);
                return true;
            }
            catch (InvalidOperationException)
            {
                _pending = null;
                errorCode = "coop_native_client_transport_unavailable";
                return false;
            }
        }
    }

    internal bool TryReconcile(out OpaqueOperationNativeMessage? reply, out string originalBody)
    {
        lock (_gate)
        {
            reply = _pending?.Reply;
            originalBody = _pending is null ? string.Empty
                : System.Text.Encoding.UTF8.GetString(_pending.Payload);
            if (reply is not null)
            {
                if (reply.Kind is OpaqueOperationNativeMessageKind.SettledReply
                    or OpaqueOperationNativeMessageKind.RejectedReply)
                {
                    _pending = null;
                }
                return true;
            }
            if (_pending is null)
                return false;
            OpaqueOperationNativeMessageAdapter? adapter = _adapter();
            if (adapter is not null)
            {
                try { adapter.SendRequest(_pending.OperationId, _pending.Payload); }
                catch (InvalidOperationException) { }
            }
            return false;
        }
    }

    internal void ReceiveAuthenticatedHostReply(OpaqueOperationNativeMessage message)
    {
        if (!message.IsReply())
            return;
        lock (_gate)
        {
            if (_pending is null || !string.Equals(_pending.OperationId,
                    message.OriginalOperationId, StringComparison.Ordinal))
            {
                return;
            }
            _pending = _pending with { Reply = message };
        }
    }

    private sealed record PendingOperation(string OperationId, byte[] Payload)
    {
        internal OpaqueOperationNativeMessage? Reply { get; init; }
    }
}
