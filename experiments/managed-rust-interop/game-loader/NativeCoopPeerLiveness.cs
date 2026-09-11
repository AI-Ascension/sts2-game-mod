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
    // A loss value without a recent heartbeat receipt is not enough to identify a stale peer.
    // This fence spans several first-party heartbeat intervals and uses the same millisecond
    // clock as NetQualityTracker.LastReceivedTime.
    internal const ulong LastReceivedFenceMsec = 2_000;

    internal static bool IsUnresponsive(float packetLoss) =>
        !float.IsNaN(packetLoss) && packetLoss >= PacketLossFence;

    internal static bool IsUnresponsive(
        float packetLoss, ulong? lastReceivedMsec, ulong nowMsec)
    {
        if (!lastReceivedMsec.HasValue || nowMsec < lastReceivedMsec.Value
            || nowMsec - lastReceivedMsec.Value < LastReceivedFenceMsec)
        {
            return false;
        }

        return IsUnresponsive(packetLoss);
    }
}
