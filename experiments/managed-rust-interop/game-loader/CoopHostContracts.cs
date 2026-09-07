// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class CoopNativeValueRules
{
    internal static bool IsDigest(string value) =>
        value.Length == 64
        && AllHex(value);

    private static bool AllHex(string value)
    {
        foreach (char character in value)
        {
            if (!char.IsAsciiDigit(character) && (character < 'a' || character > 'f'))
            {
                return false;
            }
        }
        return true;
    }
}

internal static class CoopPeerIdentity
{
    // Public traces use a session-scoped opaque token. The native authenticated ID is retained
    // only in the host adapter's memory and is never put in a protocol response or artifact.
    internal static bool IsOpaque(string value) =>
        RuntimeV3GameplayContract.IsIdentity(value)
        && value.StartsWith("peer:", StringComparison.Ordinal)
        && value.Length >= 10;
}

/// <summary>
/// The role reported by the first-party native network service. A caller cannot choose this
/// value; it is part of the host snapshot.
/// </summary>
internal enum CoopHostRole
{
    Host,
    Client,
    Singleplayer,
    Unknown
}

internal enum CoopOutcome
{
    Accepted,
    Settled,
    Rejected,
    Unknown,
    Recovered
}

internal enum CoopVoteDomain
{
    Map,
    SharedEvent,
    TreasureRelic,
    PlayerChoice
}

/// <summary>
/// A peer's native state witness. Peer generations are local observations and may differ from
/// the host generation while animation, loading, or a queued action is in flight.
/// </summary>
internal sealed record CoopPeerSnapshot(
    string PeerId,
    bool IsLocal,
    bool Connected,
    ulong Generation,
    string StateDigest)
{
    // Canonical native authority identity derived from the lobby and native host ID. This can
    // be compared across host/client observations when both first-party values agree.
    internal string AuthorityId { get; init; } = "authority:test";
    // Process-local adapter lifecycle fence. It is intentionally not cross-process stable.
    // These fields are adapter attestations. They are deliberately separate from the peer's
    // local generation: the installed host does not expose a global native game generation.
    internal string AuthorityEpoch { get; init; } = "epoch:test";
    internal string? CheckpointId { get; init; } = "checkpoint:test";
    internal ulong RejoinEpoch { get; init; }
    internal bool DigestKnown { get; init; } = true;
    internal bool IsLoading { get; init; }
    internal bool IsDivergent { get; init; }
    internal string ChecksumStatus { get; init; } = "unavailable";
    internal string HostSequenceKind { get; init; } = "adapter_sequence";
}

/// <summary>
/// Snapshot assembled from the native game service and run state on the Godot game thread.
/// HostGeneration is the only authoritative ordering fence. Peer generations are diagnostics;
/// convergence is proved with the native state digest/checksum and an operation witness.
/// </summary>
internal sealed record CoopHostObservation(
    string SessionId,
    string LocalPeerId,
    CoopHostRole Role,
    string Platform,
    string? LobbyIdentifier,
    ulong HostGeneration,
    string HostStateDigest,
    IReadOnlyList<CoopPeerSnapshot> Peers,
    bool RecoveryRequired,
    string? PendingProposal)
{
    // HostGeneration is an adapter-owned sequence, not a native global generation. The native
    // service exposes peer IDs, role, loading state, and checksum checkpoints, but no such
    // global counter. A producer may only advance this sequence after a fresh host snapshot.
    internal string AuthorityId { get; init; } = "authority:test";
    // This epoch is a process-local adapter fence, not the native authority identity.
    internal string AuthorityEpoch { get; init; } = "epoch:test";
    internal string RunId { get; init; } = "run:test";
    internal string HostSequenceKind { get; init; } = "adapter_sequence";
    internal string CheckpointId { get; init; } = "checkpoint:test";
    internal string ChecksumAlgorithm { get; init; } = "sha256";
    internal string ChecksumStatus { get; init; } = "unavailable";
    internal string? NativeChecksum { get; init; }
    internal bool HostDigestKnown { get; init; } = true;
    internal bool HostLoading { get; init; }
    internal bool HostDivergent { get; init; }

    internal bool Validate(out string error)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(SessionId)
            || !CoopPeerIdentity.IsOpaque(LocalPeerId)
            || !Enum.IsDefined(Role)
            || Role == CoopHostRole.Unknown
            || !RuntimeV3GameplayContract.IsIdentity(Platform)
            || LobbyIdentifier is not null && !RuntimeV3GameplayContract.IsText(LobbyIdentifier)
            || HostGeneration > RuntimeV3GameplayContract.MaxGeneration
            || !RuntimeV3GameplayContract.IsIdentity(AuthorityId)
            || !CoopNativeValueRules.IsDigest(HostStateDigest)
            || !RuntimeV3GameplayContract.IsIdentity(AuthorityEpoch)
            || !RuntimeV3GameplayContract.IsIdentity(RunId)
            || !RuntimeV3GameplayContract.IsIdentity(CheckpointId)
            || HostSequenceKind != "adapter_sequence"
            || ChecksumAlgorithm != "sha256"
            || !RuntimeV3GameplayContract.IsIdentity(ChecksumStatus)
            || NativeChecksum is not null && !CoopNativeValueRules.IsDigest(NativeChecksum)
            || Peers.Count < 1
            || Peers.Count > 4
            || PendingProposal is not null && !RuntimeV3GameplayContract.IsIdentity(PendingProposal))
        {
            error = "native co-op observation is outside its bounds";
            return false;
        }

        if ((Role == CoopHostRole.Host || Role == CoopHostRole.Client) && Peers.Count < 2)
        {
            error = "native co-op multiplayer observations require at least two peers";
            return false;
        }

        var peerIds = new HashSet<string>(StringComparer.Ordinal);
        int localCount = 0;
        foreach (CoopPeerSnapshot peer in Peers)
        {
            if (!CoopPeerIdentity.IsOpaque(peer.PeerId)
                || !peerIds.Add(peer.PeerId)
                || peer.Generation > RuntimeV3GameplayContract.MaxGeneration
                || !CoopNativeValueRules.IsDigest(peer.StateDigest)
                || !RuntimeV3GameplayContract.IsIdentity(peer.AuthorityId)
                || !RuntimeV3GameplayContract.IsIdentity(peer.AuthorityEpoch)
                || peer.CheckpointId is not null && !RuntimeV3GameplayContract.IsIdentity(peer.CheckpointId)
                || peer.RejoinEpoch > RuntimeV3GameplayContract.MaxGeneration
                || !RuntimeV3GameplayContract.IsIdentity(peer.ChecksumStatus))
            {
                error = "native co-op peer snapshot is invalid";
                return false;
            }

            if (peer.IsLocal)
            {
                localCount++;
                if (!string.Equals(peer.PeerId, LocalPeerId, StringComparison.Ordinal))
                {
                    error = "native co-op local peer does not match the service identity";
                    return false;
                }
            }
        }

        if (localCount != 1 || !peerIds.Contains(LocalPeerId))
        {
            error = "native co-op observation must contain exactly one local peer";
            return false;
        }

        error = string.Empty;
        return true;
    }

    internal bool AllConnectedPeersConverged()
    {
        if (!HostDigestKnown || HostLoading || HostDivergent)
        {
            return false;
        }
        foreach (CoopPeerSnapshot peer in Peers)
        {
            if (!peer.Connected
                || peer.IsLoading
                || peer.IsDivergent
                || !peer.DigestKnown
                || !string.Equals(peer.AuthorityId, AuthorityId, StringComparison.Ordinal)
                || CheckpointId != peer.CheckpointId
                || !string.Equals(peer.StateDigest, HostStateDigest, StringComparison.Ordinal))
            {
                return false;
            }
        }
        return !RecoveryRequired;
    }
}
