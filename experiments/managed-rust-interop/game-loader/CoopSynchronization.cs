// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal enum CoopPeerRole
{
    Local,
    Ally
}

internal enum CoopSyncStatus
{
    Synchronized,
    Disagreement,
    Disconnected
}

internal sealed record CoopPeer(string PeerId, CoopPeerRole Role);

/// <summary>Bounded peer state used to suspend mutation during disagreement or disconnect.</summary>
internal sealed record CoopSynchronization(
    CoopSyncStatus Status,
    ulong Generation,
    byte PeerCount,
    IReadOnlyList<string> MissingPeers)
{
    internal bool MutationAllowed =>
        Status == CoopSyncStatus.Synchronized
        && MissingPeers.Count == 0
        && PeerCount >= 2
        && PeerCount <= 4;

    internal bool Validate(out string error)
    {
        if (Generation > RuntimeV3GameplayContract.MaxGeneration
            || PeerCount < 2
            || PeerCount > 4
            || MissingPeers.Count > 4)
        {
            error = "co-op synchronization is outside its bounds";
            return false;
        }
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (string peerId in MissingPeers)
        {
            if (!RuntimeV3GameplayContract.IsIdentity(peerId) || !seen.Add(peerId))
            {
                error = "co-op missing peer identity is invalid";
                return false;
            }
        }
        error = string.Empty;
        return true;
    }
}
