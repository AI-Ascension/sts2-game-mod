// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Player-visible cooperative metadata. It has no host object, hidden state, or input primitive.
/// </summary>
internal sealed record CoopProjection(
    string StateId,
    ulong Generation,
    IReadOnlyList<CoopPeer> Players,
    CoopSynchronization Synchronization)
{
    internal bool MutationAllowed => Synchronization.MutationAllowed;

    internal static bool TryCreate(
        string stateId,
        ulong generation,
        IReadOnlyList<CoopPeer> players,
        CoopSynchronization synchronization,
        out CoopProjection? projection,
        out string error)
    {
        projection = null;
        error = string.Empty;
        if (!RuntimeV3GameplayContract.IsIdentity(stateId)
            || generation > RuntimeV3GameplayContract.MaxGeneration
            || players.Count < 2
            || players.Count > 4
            || synchronization.Generation != generation
            || synchronization.PeerCount != players.Count
            || !synchronization.Validate(out error))
        {
            error = string.IsNullOrEmpty(error) ? "co-op projection identity or bounds are invalid" : error;
            return false;
        }
        var peerIds = new HashSet<string>(StringComparer.Ordinal);
        var allyIds = new HashSet<string>(StringComparer.Ordinal);
        int localCount = 0;
        foreach (CoopPeer peer in players)
        {
            if (!Enum.IsDefined(peer.Role)
                || !RuntimeV3GameplayContract.IsIdentity(peer.PeerId)
                || !peerIds.Add(peer.PeerId))
            {
                error = "co-op peer identity is invalid or duplicated";
                return false;
            }
            if (peer.Role == CoopPeerRole.Local)
            {
                localCount++;
            }
            else
            {
                allyIds.Add(peer.PeerId);
            }
        }
        if (localCount != 1 || allyIds.Count == 0)
        {
            error = "co-op projection must contain one local peer and an ally";
            return false;
        }
        foreach (string missingPeer in synchronization.MissingPeers)
        {
            if (!peerIds.Contains(missingPeer))
            {
                error = "co-op missing peer is not in the player set";
                return false;
            }
        }
        error = string.Empty;
        projection = new CoopProjection(stateId, generation,
            new List<CoopPeer>(players).AsReadOnly(), synchronization with
            {
                MissingPeers = new List<string>(synchronization.MissingPeers).AsReadOnly()
            });
        return true;
    }

    internal bool IsAlly(string peerId)
    {
        foreach (CoopPeer peer in Players)
        {
            if (peer.Role == CoopPeerRole.Ally
                && string.Equals(peer.PeerId, peerId, StringComparison.Ordinal))
            {
                return true;
            }
        }
        return false;
    }
}
