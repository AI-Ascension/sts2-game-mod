// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Messages.Game.Checksums;
using MegaCrit.Sts2.Core.Multiplayer.Transport;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// First-party host adapter for the native multiplayer service. It attests the active native
/// lobby and peer roster and forwards mutations through the installed game's synchronizers.
/// Action settlement requires a finished native action, a fresh checkpoint, and peer convergence.
/// </summary>
internal sealed partial class InstalledNativeCoopHostPort : ICoopNativeHostPort
{
    private const string DigestAlgorithm = "sha256";
    private const string UnknownDigest = "0000000000000000000000000000000000000000000000000000000000000000";
    private const int MaxPendingOperations = 4096;
    private readonly Dictionary<string, CoopNativePeerBinding> _bindings = new(StringComparer.Ordinal);
    private readonly Dictionary<ulong, string> _opaqueByNativeId = new();
    private readonly HashSet<ulong> _knownDisconnectedNativeIds = new();
    private readonly Dictionary<string, NativePendingOperation> _pending = new(StringComparer.Ordinal);
    private readonly MessageHandlerDelegate<ChecksumDataMessage> _checksumMessageHandler;
    private string? _authorityKey;
    private string? _authorityEpoch;
    private string? _rejoinAuthorityEpoch;
    private bool _lastAuthorityConnected;
    private string? _lastHostFingerprint;
    private ulong _hostSequence;
    private ChecksumTracker? _checksumTracker;
    private INetGameService? _checksumService;
    private string? _nativeChecksumDigest;
    private NetChecksumData? _nativeChecksumData;
    private readonly Dictionary<ulong, NetChecksumData> _remoteChecksums = new();
    private ulong _nativeChecksumOrdinal;
    private bool _nativeStateDiverged;

    internal InstalledNativeCoopHostPort()
    {
        _checksumMessageHandler = OnRemoteChecksumDataMessage;
    }

    public CoopHostObservation Observe()
    {
        RunManager manager = RunManager.Instance
            ?? throw new InvalidOperationException("run manager is unavailable");
        INetGameService service = NativeCoopSessionController.ActiveService
            ?? manager.NetService
            ?? throw new InvalidOperationException("native network service is unavailable");
        EnsureChecksumHooks(manager.ChecksumTracker, service);
        CoopHostRole role = service.Type switch
        {
            NetGameType.Host => CoopHostRole.Host,
            NetGameType.Client => CoopHostRole.Client,
            NetGameType.Singleplayer => CoopHostRole.Singleplayer,
            _ => CoopHostRole.Unknown
        };
        if (role == CoopHostRole.Unknown)
            throw new InvalidOperationException("native network service role is unavailable");

        ulong localNativeId = service.NetId;
        string? lobby = service.GetRawLobbyIdentifier();
        ulong hostNativeId = NativeHostId(service, localNativeId);
        RunState? run = manager.IsInProgress ? manager.DebugOnlyGetState() : null;
        IReadOnlyList<ulong> connectedNativeIds = ConnectedNativePeerIds(service, localNativeId);
        ulong[] unresponsiveNativeIds = UnresponsiveNativePeerIds(service);
        IReadOnlyList<ulong> rosterNativeIds = RunRosterNativePeerIds(
            run, connectedNativeIds, localNativeId);
        string authorityId = CreateAuthorityId(lobby, hostNativeId);
        bool authorityConnected = service.IsConnected
            && role is CoopHostRole.Host or CoopHostRole.Client;
        string authorityEpoch = UpdateAuthorityEpoch(
            lobby, hostNativeId, authorityConnected);
        bool clearRejoinFenceAfterObservation =
            authorityConnected && NativeCoopSessionController.IsRejoinRecovered;
        UpdatePeerBindings(connectedNativeIds, rosterNativeIds);
        string localPeerId = OpaquePeer(localNativeId);
        string runId = CreateRunId(run, lobby);
        bool hostDigestKnown = _nativeChecksumDigest is not null && !_nativeStateDiverged;
        string hostDigest = _nativeChecksumDigest
            ?? CreateSharedStateDigest(run, rosterNativeIds, service.IsConnected);
        string hostFingerprint = $"{runId}|{hostDigest}|{string.Join(',', rosterNativeIds)}|"
            + $"{string.Join(',', connectedNativeIds)}|{string.Join(',', unresponsiveNativeIds)}|"
            + $"{service.IsGameLoading}";
        if (!string.Equals(_lastHostFingerprint, hostFingerprint, StringComparison.Ordinal))
        {
            if (_lastHostFingerprint is not null)
                _hostSequence++;
            _lastHostFingerprint = hostFingerprint;
        }

        bool checksumEnabled = manager.ChecksumTracker?.IsEnabled == true;
        string checksumStatus = _nativeStateDiverged
            ? "divergent"
            : checksumEnabled
                ? _nativeChecksumDigest is null ? "enabled_unread" : "enabled"
                : "disabled";
        var peers = new List<CoopPeerSnapshot>();
        string checkpointId = _nativeChecksumDigest is null
            ? CheckpointId(_hostSequence)
            : NativeCheckpointId(_nativeChecksumOrdinal);
        var connectedSet = connectedNativeIds.ToHashSet();
        foreach (ulong nativeId in rosterNativeIds)
        {
            string opaque = OpaquePeer(nativeId);
            bool local = nativeId == localNativeId;
            bool remoteMatched = !local && RemoteChecksumMatches(nativeId);
            bool peerDivergent = !local && RemoteChecksumDivergent(nativeId);
            peers.Add(new CoopPeerSnapshot(
                opaque,
                local,
                service.IsConnected && connectedSet.Contains(nativeId),
                local || remoteMatched ? _hostSequence : 0,
                local && hostDigestKnown || remoteMatched ? hostDigest : UnknownDigest)
            {
                AuthorityId = authorityId,
                AuthorityEpoch = authorityEpoch,
                CheckpointId = local && hostDigestKnown || remoteMatched ? checkpointId : null,
                RejoinEpoch = 0,
                DigestKnown = local ? hostDigestKnown : remoteMatched,
                IsLoading = service.IsGameLoading,
                IsDivergent = _nativeStateDiverged || peerDivergent,
                ChecksumStatus = peerDivergent ? "divergent" : remoteMatched ? "matched" : checksumStatus
            });
        }

        // A connected client can attest its host NetId before the run player collection exists.
        // It remains non-converged until a native checkpoint supplies the remote witness.
        if (role == CoopHostRole.Client && service is INetClientGameService client)
        {
            NetClient? nativeClient = client.NetClient;
            if (nativeClient is not null && nativeClient.HostNetId != 0
                && !connectedNativeIds.Contains(nativeClient.HostNetId))
            {
                string hostOpaque = OpaquePeer(hostNativeId);
                _bindings[hostOpaque] = new CoopNativePeerBinding(hostOpaque, hostNativeId, true);
                peers.Add(new CoopPeerSnapshot(hostOpaque, false, service.IsConnected, 0, UnknownDigest)
                {
                    AuthorityId = authorityId,
                    AuthorityEpoch = authorityEpoch,
                    CheckpointId = null,
                    RejoinEpoch = 0,
                    DigestKnown = false,
                    IsLoading = service.IsGameLoading,
                    IsDivergent = _nativeStateDiverged,
                    ChecksumStatus = checksumStatus
                });
            }
        }

        if (peers.Count == 0)
        {
            peers.Add(new CoopPeerSnapshot(localPeerId, true, service.IsConnected,
                _hostSequence, UnknownDigest)
            {
                AuthorityId = authorityId,
                AuthorityEpoch = authorityEpoch,
                CheckpointId = checkpointId,
                RejoinEpoch = 0,
                DigestKnown = false,
                IsLoading = service.IsGameLoading,
                IsDivergent = false,
                ChecksumStatus = checksumStatus
            });
        }

        CoopHostObservation observation = new CoopHostObservation(
            SessionId: $"native:{runId}|{authorityEpoch}",
            LocalPeerId: localPeerId,
            Role: role,
            Platform: service.Platform.ToString().ToLowerInvariant(),
            LobbyIdentifier: lobby,
            HostGeneration: _hostSequence,
            HostStateDigest: hostDigest,
            Peers: peers,
            RecoveryRequired: !service.IsConnected || manager.IsCleaningUp
                || role == CoopHostRole.Host
                && (unresponsiveNativeIds.Length > 0
                    || rosterNativeIds.Any(id => id != localNativeId
                        && !connectedNativeIds.Contains(id))),
            PendingProposal: null)
        {
            AuthorityId = authorityId,
            AuthorityEpoch = authorityEpoch,
            RunId = runId,
            CheckpointId = checkpointId,
            ChecksumAlgorithm = DigestAlgorithm,
            ChecksumStatus = checksumStatus,
            NativeChecksum = _nativeChecksumDigest,
            HostDigestKnown = hostDigestKnown,
            HostLoading = service.IsGameLoading,
            HostDivergent = _nativeStateDiverged
        };
        if (clearRejoinFenceAfterObservation)
            _rejoinAuthorityEpoch = null;
        return observation;
    }

    public bool TryResolvePeer(string opaquePeerId, out CoopNativePeerBinding binding) =>
        _bindings.TryGetValue(opaquePeerId, out binding!);

    public bool MayDispatchWithUnknownDigest(CoopHostObservation observation)
    {
        if (observation.HostDigestKnown || observation.Role == CoopHostRole.Singleplayer
            || observation.RecoveryRequired || observation.HostLoading || observation.HostDivergent)
        {
            return false;
        }

        RunManager? manager = RunManager.Instance;
        INetGameService? service = CurrentService(manager);
        return manager is not null
            && service is not null
            && service.IsConnected
            && manager.IsInProgress
            && manager.ActionQueueSynchronizer is not null;
    }
}
