// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    internal static LiveCardCapturedSnapshot ControlledLiveSnapshot { get; set; } =
        LiveCardCapturedSnapshot.Unavailable("synthetic_unconfigured");

    internal static LiveCardCapturedSnapshot ReadLiveCardSnapshot(
        string instanceId,
        string contentManifest) =>
        ControlledLiveSnapshot.Available
        && ControlledLiveSnapshot.InstanceId == instanceId
        && ControlledLiveSnapshot.ContentManifest == contentManifest
            ? ControlledLiveSnapshot
            : LiveCardCapturedSnapshot.Unavailable("synthetic_identity_mismatch");

    internal static LiveCardCapturedSnapshot ReadRetainedLiveCardSnapshot(
        string instanceId,
        string contentManifest,
        ulong epoch)
    {
        LiveCardCapturedSnapshot snapshot = ReadLiveCardSnapshot(instanceId, contentManifest);
        return snapshot.Available && snapshot.Epoch == epoch
            ? snapshot
            : LiveCardCapturedSnapshot.Unavailable("synthetic_stale_snapshot");
    }
}
