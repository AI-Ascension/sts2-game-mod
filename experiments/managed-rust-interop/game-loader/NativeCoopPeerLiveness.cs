// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Bounded interpretation of the first-party heartbeat loss metric. A missing quality record is
/// retained as unknown until the native quality tracker has observed the peer; only a near-total
/// loss estimate is evidence of a stale peer.
/// </summary>
internal static class NativeCoopPeerLiveness
{
    // NetQualityTracker sends heartbeats every 200 ms. The installed ConnectionStats applies a
    // weighted loss estimate, so 0.99 represents sustained loss rather than one dropped packet.
    internal const float PacketLossFence = 0.99f;

    internal static bool IsUnresponsive(float packetLoss) =>
        !float.IsNaN(packetLoss) && packetLoss >= PacketLossFence;
}
