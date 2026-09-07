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
    }
}
