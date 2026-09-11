// SPDX-License-Identifier: MIT

using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Threading;
using MegaCrit.Sts2.Core.Multiplayer.Game;

namespace AiAscension.Sts2GameMod.Runtime;

internal interface IHostAdmittedOpaqueOperationDispatcher
{
    OpaqueOperationHostDispatchResult Dispatch(ulong authenticatedPeerId,
        string originalOperationId, byte[] payload);

    OpaqueOperationHostDispatchResult Reconcile(ulong authenticatedPeerId,
        string originalOperationId);
}

internal sealed class OpaqueOperationHostDispatchResult
{
    private OpaqueOperationHostDispatchResult(string? settlementWitness, string? rejectionCode,
        bool isPending = false)
    {
        SettlementWitness = settlementWitness;
        RejectionCode = rejectionCode;
        IsPending = isPending;
    }

    internal string? SettlementWitness { get; }
    internal string? RejectionCode { get; }
    internal bool IsPending { get; }

    internal static OpaqueOperationHostDispatchResult Settled(string settlementWitness) =>
        new(settlementWitness, null);

    internal static OpaqueOperationHostDispatchResult Rejected(string rejectionCode) =>
        new(null, rejectionCode);

    internal static OpaqueOperationHostDispatchResult Pending(string causalCode) =>
        new(null, causalCode, true);
}

/// <summary>Registers the concrete message type and keeps host settlement scoped to sender ID.</summary>
internal sealed partial class OpaqueOperationNativeMessageAdapter : IDisposable
{
    private const int MaximumRecordedOperations = 4096;
    internal const int MaximumInboundMessages = 128;
    internal const int MaximumInboundMessagesPerPump = 16;
    private const int MaximumPendingReconciliationsPerPump = 16;
    private const int MaximumOutboundRepliesPerPump = 16;
    private readonly INetGameService _service;
    private readonly IHostAdmittedOpaqueOperationDispatcher? _hostDispatcher;
    private readonly Func<ulong, bool> _isAuthenticatedHost;
    private readonly Action<OpaqueOperationNativeMessage>? _replySink;
    private readonly MessageHandlerDelegate<OpaqueOperationNativeMessage> _handler;
    private readonly Dictionary<string, RecordedReply> _recorded = new(StringComparer.Ordinal);
    private readonly ConcurrentQueue<InboundMessage> _inboundMessages = new();
    private readonly ConcurrentQueue<string> _pendingOperationIds = new();
    private readonly ConcurrentQueue<OutboundReply> _outboundReplies = new();
    private readonly object _recordGate = new();
    private int _inboundCount;
    private int _droppedInboundMessages;
    private bool _registered;

    internal OpaqueOperationNativeMessageAdapter(INetGameService service,
        IHostAdmittedOpaqueOperationDispatcher? hostDispatcher,
        Func<ulong, bool> isAuthenticatedHost,
        Action<OpaqueOperationNativeMessage>? replySink = null)
    {
        _service = service ?? throw new ArgumentNullException(nameof(service));
        _hostDispatcher = hostDispatcher;
        _isAuthenticatedHost = isAuthenticatedHost ?? throw new ArgumentNullException(nameof(isAuthenticatedHost));
        _replySink = replySink;
        _handler = OnMessage;
    }

    internal void Register()
    {
        if (_registered)
            return;
        _service.RegisterMessageHandler(_handler);
        _registered = true;
    }

    internal void SendRequest(string originalOperationId, byte[] payload)
    {
        if (_service is INetHostGameService)
            throw new InvalidOperationException("native request sender must not be the host");
        OpaqueOperationNativeMessage request = OpaqueOperationNativeMessage.Request(
            originalOperationId, payload);
        if (!request.IsRequest())
            throw new ArgumentOutOfRangeException(nameof(originalOperationId));
        _service.SendMessage(request);
    }

    /// <summary>Runs on the game-thread frame pump; network callbacks only copy and enqueue.</summary>
    internal void Pump()
    {
        if (_service is INetHostGameService)
            RefreshHostReplies();
        for (int processed = 0; processed < MaximumInboundMessagesPerPump
            && _inboundMessages.TryDequeue(out InboundMessage? inbound); processed++)
        {
            Interlocked.Decrement(ref _inboundCount);
            if (_service is INetHostGameService)
                HandleHostRequest(inbound.Message, inbound.SenderId);
            else if (inbound.Message.IsReply() && _isAuthenticatedHost(inbound.SenderId))
                _replySink?.Invoke(inbound.Message);
        }
        if (_service is INetHostGameService)
        {
            for (int sent = 0; sent < MaximumOutboundRepliesPerPump
                && _outboundReplies.TryDequeue(out OutboundReply? reply); sent++)
                _service.SendMessage(reply.Message, reply.PeerId);
        }
    }

    public void Dispose()
    {
        if (!_registered)
            return;
        _service.UnregisterMessageHandler(_handler);
        _registered = false;
    }

    private void OnMessage(OpaqueOperationNativeMessage message, ulong senderId)
    {
        if (message is null || (!message.IsRequest() && !message.IsReply()))
            return;
        if (!TryReserveInboundSlot())
        {
            Interlocked.Increment(ref _droppedInboundMessages);
            return;
        }
        try
        {
            _inboundMessages.Enqueue(new InboundMessage(senderId, CopyForPump(message)));
        }
        catch
        {
            Interlocked.Decrement(ref _inboundCount);
            throw;
        }
    }

    internal int PendingInboundMessagesForTest => Volatile.Read(ref _inboundCount);
    internal int DroppedInboundMessagesForTest => Volatile.Read(ref _droppedInboundMessages);

    private bool TryReserveInboundSlot()
    {
        while (true)
        {
            int current = Volatile.Read(ref _inboundCount);
            if (current >= MaximumInboundMessages)
                return false;
            if (Interlocked.CompareExchange(ref _inboundCount, current + 1, current) == current)
                return true;
        }
    }

    private static OpaqueOperationNativeMessage CopyForPump(OpaqueOperationNativeMessage message)
    {
        byte[] payload = new byte[message.Payload.Length];
        Array.Copy(message.Payload, payload, payload.Length);
        return message.Kind switch
        {
            OpaqueOperationNativeMessageKind.Request => OpaqueOperationNativeMessage.Request(
                message.OriginalOperationId, payload),
            OpaqueOperationNativeMessageKind.SettledReply => OpaqueOperationNativeMessage.SettledReply(
                message.OriginalOperationId, message.SettlementWitness),
            OpaqueOperationNativeMessageKind.PendingReply => OpaqueOperationNativeMessage.PendingReply(
                message.OriginalOperationId, message.RejectionCode),
            OpaqueOperationNativeMessageKind.RejectedReply => OpaqueOperationNativeMessage.RejectedReply(
                message.OriginalOperationId, message.RejectionCode),
            _ => new OpaqueOperationNativeMessage()
        };
    }

    private sealed record InboundMessage(ulong SenderId, OpaqueOperationNativeMessage Message);
}
