// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Multiplayer.Messages.Game.Checksums;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Transport;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    private bool CanRetainPending(string operationId) =>
        _pending.ContainsKey(operationId) || _pending.Count < MaxPendingOperations;

    private bool TryGetBoundPlayer(string actorPeerId, out RunManager manager,
        out INetGameService service, out Player player, out string error)
    {
        manager = RunManager.Instance!;
        service = null!;
        player = null!;
        error = string.Empty;
        if (manager is null)
        {
            error = "native_run_manager_unavailable";
            return false;
        }

        service = CurrentService(manager)!;
        if (service is null || !service.IsConnected)
        {
            error = "native_network_service_unavailable";
            return false;
        }
        if (!TryResolvePeer(actorPeerId, out CoopNativePeerBinding binding)
            || !binding.Validate(out _)
            || !string.Equals(binding.OpaquePeerId, actorPeerId, StringComparison.Ordinal)
            || service.Type != NetGameType.Host && binding.NativePeerId != service.NetId)
        {
            error = "unknown_or_stale_peer_identity";
            return false;
        }
        if (!manager.IsInProgress || manager.IsCleaningUp)
        {
            error = "native_run_unavailable";
            return false;
        }

        RunState? run = manager.DebugOnlyGetState();
        if (run is null)
        {
            error = "native_run_state_unavailable";
            return false;
        }
        player = run.Players.FirstOrDefault(candidate => candidate.NetId == binding.NativePeerId)!;
        if (player is null)
        {
            error = "native_local_player_unavailable";
            return false;
        }
        return true;
    }

    private static INetGameService? CurrentService(RunManager? manager)
    {
        return NativeCoopSessionController.ActiveService ?? manager?.NetService;
    }

    private string OpaquePeer(ulong nativeId)
    {
        if (_opaqueByNativeId.TryGetValue(nativeId, out string? opaque))
            return opaque;

        opaque = CreateOpaquePeer();
        _opaqueByNativeId[nativeId] = opaque;
        _bindings[opaque] = new CoopNativePeerBinding(opaque, nativeId, true);
        return opaque;
    }

    private string UpdateAuthorityEpoch(string? lobby, ulong hostNativeId, bool connected)
    {
        string key = $"{lobby ?? "none"}|{hostNativeId:x16}";
        if (NativeCoopSessionController.RejoinStatus.StartsWith(
                "failed:", StringComparison.Ordinal))
        {
            _rejoinAuthorityEpoch = null;
        }
        bool controlledRejoin = connected && !_lastAuthorityConnected
            && _rejoinAuthorityEpoch is not null
            && string.Equals(_rejoinAuthorityEpoch, _authorityEpoch,
                StringComparison.Ordinal);
        if (_authorityEpoch is null
            || !string.Equals(_authorityKey, key, StringComparison.Ordinal)
            || (!_lastAuthorityConnected && connected && !controlledRejoin))
        {
            _authorityKey = key;
            _authorityEpoch = CreateAuthorityEpoch();
            _lastHostFingerprint = null;
            _hostSequence = 0;
            _opaqueByNativeId.Clear();
            _bindings.Clear();
            _knownDisconnectedNativeIds.Clear();
            // Operations admitted under the previous native authority cannot be reconciled
            // against a new lobby/checkpoint lineage. Keep the gateway receipt unknown and
            // discard only this adapter's native pending handles so a reconnect cannot settle
            // an old operation from a fresh checksum.
            _pending.Clear();
            _remoteChecksums.Clear();
            _nativeChecksumData = null;
            _nativeChecksumDigest = null;
            _nativeChecksumOrdinal = 0;
            _nativeStateDiverged = false;
        }
        _lastAuthorityConnected = connected;
        return _authorityEpoch;
    }

    private void UpdatePeerBindings(
        IReadOnlyList<ulong> connectedNativeIds, IReadOnlyList<ulong> rosterNativeIds)
    {
        var connected = connectedNativeIds.ToHashSet();
        foreach (ulong nativeId in rosterNativeIds)
        {
            bool preserveRejoinBinding = _rejoinAuthorityEpoch is not null
                && (NativeCoopSessionController.IsRejoinInProgress
                    || NativeCoopSessionController.IsRejoinRecovered);
            if (connected.Contains(nativeId) && _knownDisconnectedNativeIds.Contains(nativeId)
                && !preserveRejoinBinding)
            {
                if (_opaqueByNativeId.TryGetValue(nativeId, out string? previous))
                    _bindings.Remove(previous);
                _opaqueByNativeId.Remove(nativeId);
                _knownDisconnectedNativeIds.Remove(nativeId);
            }
            else if (connected.Contains(nativeId)
                && NativeCoopSessionController.IsRejoinRecovered)
            {
                // The replacement client has now been observed on the original authority.
                // Retain its opaque identity after the recovery fence is cleared so a later
                // observation does not mistake the settled rejoin for a new peer.
                _knownDisconnectedNativeIds.Remove(nativeId);
            }
            _ = OpaquePeer(nativeId);
            if (!connected.Contains(nativeId))
                _knownDisconnectedNativeIds.Add(nativeId);
        }
    }

    private static string CheckpointId(ulong hostSequence) =>
        $"checkpoint:adapter-{hostSequence.ToString(CultureInfo.InvariantCulture)}";


    private static string CreateOpaquePeer()
    {
        Span<byte> bytes = stackalloc byte[16];
        RandomNumberGenerator.Fill(bytes);
        return $"peer:{Convert.ToHexString(bytes).ToLowerInvariant()}";
    }

    private static string CreateAuthorityEpoch()
    {
        Span<byte> bytes = stackalloc byte[16];
        RandomNumberGenerator.Fill(bytes);
        return $"epoch:{Convert.ToHexString(bytes).ToLowerInvariant()}";
    }

    private static string Sha256Hex(string value)
    {
        byte[] bytes = SHA256.HashData(Encoding.UTF8.GetBytes(value));
        return Convert.ToHexString(bytes).ToLowerInvariant();
    }

    private static string Sha256Hex(ReadOnlySpan<byte> bytes) =>
        Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
}
