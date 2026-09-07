// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Messages.Game.Checksums;
using MegaCrit.Sts2.Core.Multiplayer.Serialization;
using MegaCrit.Sts2.Core.Multiplayer.Quality;
using MegaCrit.Sts2.Core.Multiplayer.Transport;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    private void EnsureChecksumHooks(ChecksumTracker? tracker, INetGameService service)
    {
        if (ReferenceEquals(_checksumTracker, tracker)
            && ReferenceEquals(_checksumService, service))
            return;
        if (_checksumTracker is not null)
        {
            _checksumTracker.ChecksumGenerated -= OnChecksumGenerated;
            _checksumTracker.StateDiverged -= OnStateDiverged;
        }
        if (_checksumService is not null)
        {
            try
            {
                _checksumService.UnregisterMessageHandler(_checksumMessageHandler);
            }
            catch
            {
                // A disconnected service may already have disposed its message bus.
            }
        }
        _checksumTracker = tracker;
        _checksumService = service;
        if (_checksumTracker is not null)
        {
            _checksumTracker.ChecksumGenerated += OnChecksumGenerated;
            _checksumTracker.StateDiverged += OnStateDiverged;
        }
        try
        {
            // ChecksumDataMessage is the first-party client-to-host checkpoint producer. The
            // tracker registers its own consumer; this additive handler observes the same
            // public message after transport authentication and binds it to a pending action.
            _checksumService.RegisterMessageHandler(_checksumMessageHandler);
        }
        catch
        {
            // The adapter remains fail-closed if the service cannot register the witness hook.
        }
        _nativeChecksumDigest = null;
        _nativeChecksumData = null;
        _remoteChecksums.Clear();
        _nativeChecksumOrdinal = 0;
        _nativeStateDiverged = false;
    }

    private void OnChecksumGenerated(NetChecksumData checksum, string context,
        NetFullCombatState fullState)
    {
        RecordNativeChecksum(checksum, fullState);
        foreach (NativePendingOperation pending in _pending.Values)
        {
            // A checksum context has no operation ID. Bind one callback to one matching pending
            // operation; never let duplicate action renderings settle two requests.
            if (pending.TryRecordPassiveChecksum(
                    _nativeChecksumOrdinal, context, checksum, fullState))
                break;
        }
    }

    private void OnRemoteChecksumDataMessage(ChecksumDataMessage message, ulong senderId)
    {
        if (senderId == 0 || _checksumService is not INetHostGameService
            || !ConnectedNativePeerIds(_checksumService, _checksumService.NetId)
                .Contains(senderId))
        {
            return;
        }

        NetChecksumData checksum = message.checksumData;
        _remoteChecksums[senderId] = checksum;
        foreach (NativePendingOperation pending in _pending.Values)
            pending.RecordRemoteChecksum(senderId, checksum);

        if (_nativeChecksumData is not { } local || checksum.id != local.id)
            return;

        // The first-party tracker will also raise StateDiverged for this mismatch. Marking it
        // here closes the race where this additive handler runs before the tracker callback.
        if (checksum.checksum != local.checksum)
            _nativeStateDiverged = true;
    }

    private void RecordNativeChecksum(NetChecksumData checksum, NetFullCombatState fullState)
    {
        _nativeChecksumOrdinal++;
        _nativeChecksumData = checksum;
        // The first-party checksum is only a 32-bit comparison value. The event also supplies
        // the exact NetFullCombatState that produced it; hash its first-party packet encoding
        // so HostStateDigest is an actual state witness rather than a context-derived string.
        var writer = new PacketWriter();
        fullState.Serialize(writer);
        _nativeChecksumDigest = Sha256Hex(writer.Buffer.AsSpan(0, writer.BytePosition));
        _nativeStateDiverged = RemoteChecksumsDiverge(checksum);
    }

    private bool RemoteChecksumMatches(ulong nativePeerId)
    {
        if (_nativeChecksumData is not { } local || _nativeStateDiverged
            || !_remoteChecksums.TryGetValue(nativePeerId, out NetChecksumData remote))
        {
            return false;
        }
        return remote.id == local.id && remote.checksum == local.checksum;
    }

    private bool RemoteChecksumDivergent(ulong nativePeerId)
    {
        if (_nativeChecksumData is not { } local
            || !_remoteChecksums.TryGetValue(nativePeerId, out NetChecksumData remote))
        {
            return false;
        }
        return remote.id == local.id && remote.checksum != local.checksum;
    }

    private bool RemoteChecksumsDiverge(NetChecksumData local)
    {
        foreach (NetChecksumData remote in _remoteChecksums.Values)
        {
            if (remote.id == local.id && remote.checksum != local.checksum)
                return true;
        }
        return false;
    }

    private void OnStateDiverged(NetFullCombatState _)
    {
        _nativeStateDiverged = true;
    }

    private static ulong[] ConnectedNativePeerIds(
        INetGameService service, ulong localNativeId)
    {
        return NativePeerIds(service, localNativeId, filterUnresponsive: true);
    }

    private static ulong[] RawConnectedNativePeerIds(
        INetGameService service, ulong localNativeId)
    {
        return NativePeerIds(service, localNativeId, filterUnresponsive: false);
    }

    private static ulong[] NativePeerIds(
        INetGameService service, ulong localNativeId, bool filterUnresponsive)
    {
        var ids = new HashSet<ulong>();
        if (service is INetHostGameService host)
        {
            NetHost? nativeHost = host.NetHost;
            if (service.IsConnected && nativeHost is not null && nativeHost.IsConnected)
            {
                foreach (ulong peerId in nativeHost.ConnectedPeerIds)
                {
                    if (!filterUnresponsive || IsNativePeerResponsive(host, peerId))
                        ids.Add(peerId);
                }
            }
        }
        else if (service is INetClientGameService client
            && service.IsConnected
            && client.NetClient is { HostNetId: not 0, IsConnected: true } nativeClient)
        {
            ids.Add(nativeClient.HostNetId);
        }
        if (service.IsConnected)
            ids.Add(localNativeId);
        return ids.OrderBy(id => id).ToArray();
    }

    private static bool IsNativePeerResponsive(INetHostGameService host, ulong peerId)
    {
        try
        {
            ConnectionStats? stats = host.GetStatsForPeer(peerId);
            return stats is null
                || !NativeCoopPeerLiveness.IsUnresponsive(stats.PacketLoss);
        }
        catch
        {
            // A quality-tracker read is diagnostic. Keep the native roster when it is not
            // readable; the first-party disconnect event or a later successful read can then
            // provide the authoritative transition without inventing one here.
            return true;
        }
    }

    private static ulong[] RunRosterNativePeerIds(
        RunState? run, IReadOnlyList<ulong> connectedNativeIds, ulong localNativeId)
    {
        var ids = new HashSet<ulong>(connectedNativeIds);
        if (run is not null)
        {
            foreach (Player player in run.Players)
                ids.Add(player.NetId);
        }
        ids.Add(localNativeId);
        return ids.OrderBy(id => id).ToArray();
    }

    private static ulong NativeHostId(INetGameService service, ulong localNativeId)
    {
        if (service is INetClientGameService { NetClient: { HostNetId: not 0 } client })
            return client.HostNetId;
        if (service is INetHostGameService { NetHost: { NetId: not 0 } host })
            return host.NetId;
        return localNativeId;
    }
}
