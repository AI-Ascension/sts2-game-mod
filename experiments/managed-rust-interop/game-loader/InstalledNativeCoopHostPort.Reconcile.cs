// SPDX-License-Identifier: MIT

using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    public CoopNativeDispatchResult Rejoin(string opaquePeerId, ulong rejoinEpoch) =>
        CoopNativeDispatchResult.Rejected("native_rejoin_requires_run_lobby");

    public CoopEffectWitness? Reconcile(string operationId)
    {
        if (!_pending.TryGetValue(operationId, out NativePendingOperation? pending))
            return null;

        bool complete;
        try
        {
            complete = pending.IsComplete();
        }
        catch
        {
            return null;
        }

        if (!complete || pending.Completed || !pending.CanProduceEffect)
            return null;

        // RunManager's ordinary post-action callback is the supported source for this
        // checkpoint. It emits action checksums on both host and client; calling GenerateChecksum
        // here would create an extra unsynchronised native checkpoint.
        if (!pending.PassiveChecksumOrdinal.HasValue
            || pending.PassiveChecksumOrdinal.Value <= pending.ChecksumOrdinalBefore
            || pending.PassiveChecksumOrdinal.Value != _nativeChecksumOrdinal)
        {
            return null;
        }

        CoopHostObservation after;
        try
        {
            after = Observe();
        }
        catch
        {
            return null;
        }

        if (!string.Equals(pending.AuthorityEpoch, after.AuthorityEpoch,
                System.StringComparison.Ordinal))
        {
            return null;
        }

        // ChecksumDataMessage is client-to-host in the installed service. Require the exact
        // native ID/value emitted for this action from every participant admitted with it before
        // exporting a settled effect. The participant set is retained across a disconnect, so a
        // missing original peer cannot be hidden by a smaller current connection set. The client
        // route cannot satisfy this predicate because the first-party message is not broadcast
        // back to clients.
        RunManager? currentManager = RunManager.Instance;
        INetGameService? currentService = CurrentService(currentManager);
        if (currentService is null
            || !pending.HasMatchingRemoteChecksums(
                ConnectedNativePeerIds(currentService, currentService.NetId),
                currentService.NetId))
        {
            return null;
        }

        if (after.HostGeneration <= pending.BeforeHostGeneration)
            return null;

        CoopEffectWitness witness = new(
            operationId,
            $"effect:{operationId}",
            pending.EffectKind,
            pending.BeforeHostGeneration,
            after.HostGeneration,
            after.HostStateDigest)
        {
            AuthorityEpoch = after.AuthorityEpoch,
            AuthorityId = after.AuthorityId,
            CheckpointId = after.CheckpointId
        };
        if (!witness.Validate(out _))
            return null;
        pending.Completed = true;
        return witness;
    }
}
