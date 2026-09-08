// SPDX-License-Identifier: MIT

using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.CoopHostTests;

internal static partial class Program
{
    private static void NativeHeartbeatLivenessIsBoundedAndUnknownSafe()
    {
        Check(!NativeCoopPeerLiveness.IsUnresponsive(float.NaN),
            "an unavailable packet-loss estimate remains unknown");
        Check(!NativeCoopPeerLiveness.IsUnresponsive(0.989f),
            "the packet-loss fence boundary remains connected");
        Check(NativeCoopPeerLiveness.IsUnresponsive(0.99f),
            "sustained heartbeat loss fences a peer");
        Check(!NativeCoopPeerLiveness.IsUnresponsive(0.5f),
            "ordinary packet loss does not fence a peer");
        Check(!NativeCoopPeerLiveness.IsUnresponsive(
                1f, null, NativeCoopPeerLiveness.LastReceivedFenceMsec + 1),
            "a peer without a heartbeat receipt remains unknown");
        Check(!NativeCoopPeerLiveness.IsUnresponsive(
                1f, 1_000, 1_000 + NativeCoopPeerLiveness.LastReceivedFenceMsec - 1),
            "a fresh heartbeat keeps even a high loss sample out of the stale fence");
        Check(NativeCoopPeerLiveness.IsUnresponsive(
                1f, 1_000, 1_000 + NativeCoopPeerLiveness.LastReceivedFenceMsec),
            "sustained loss after a stale heartbeat enters recovery");
        Check(!NativeCoopPeerLiveness.IsUnresponsive(
                0.5f, 1_000, 1_000 + NativeCoopPeerLiveness.LastReceivedFenceMsec),
            "freshness and loss remain separate quality dimensions");
    }
}
